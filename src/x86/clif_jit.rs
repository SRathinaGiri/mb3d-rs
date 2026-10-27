//! Formulas compiled at run time with Cranelift (see `lift_clif`): the JIT
//! module and the functions built with it.
//!
//! A scene's alternating hybrid (`compile_hybrid`) becomes one function:
//! the formulas inlined, the iteration fields they write in registers.
//! `CalcParams` keeps it only if it is faster than the formulas called one
//! by one (timed once per hybrid, the result cached).  `MB3D_NO_JIT=1`
//! switches it off, `MB3D_JIT=force` uses it without the timing;
//! `MB3D_JIT_DUMP=1` / `MB3D_JIT_ASM=1` print the Cranelift code / the
//! machine code of each compiled hybrid.

use super::lift_clif::{helper_addresses, Env, Helpers, Record, HELPERS};
use super::*;
use crate::iteration::Iteration;
use cranelift_codegen::ir::types::{F64, I32, I64};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Type, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use std::mem::offset_of;

/// Whether hybrids are compiled (`MB3D_NO_JIT=1` switches it off).
pub fn enabled() -> bool {
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var_os("MB3D_NO_JIT").is_none_or(|v| v.is_empty() || v == "0")
    });
    *ON
}

/// `MB3D_JIT=force`: compiled hybrids are used without timing them
/// against the formulas called one by one.
pub fn forced() -> bool {
    static F: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var("MB3D_JIT").is_ok_and(|v| v == "force"));
    *F
}

/// The compiled hybrids (or None: not compilable or not faster), by
/// [`hybrid_key`].
static CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<u64, Option<std::sync::Arc<HybridJit>>>>> = std::sync::LazyLock::new(Default::default);

/// A hybrid compiled before (`Some(None)`: tried, not used).
pub fn cached(key: u64) -> Option<Option<std::sync::Arc<HybridJit>>> {
    CACHE.lock().ok()?.get(&key).cloned()
}

/// The compiled dIFS hybrids, as [`CACHE`].
static DIFS_CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<u64, Option<std::sync::Arc<DifsJit>>>>> = std::sync::LazyLock::new(Default::default);

pub fn cached_difs(key: u64) -> Option<Option<std::sync::Arc<DifsJit>>> {
    DIFS_CACHE.lock().ok()?.get(&key).cloned()
}

pub fn cache_difs(key: u64, h: Option<std::sync::Arc<DifsJit>>) {
    if let Ok(mut c) = DIFS_CACHE.lock() {
        if c.len() >= 64 {
            c.clear();
        }
        c.insert(key, h);
    }
}

pub fn cache(key: u64, h: Option<std::sync::Arc<HybridJit>>) {
    if let Ok(mut c) = CACHE.lock() {
        if c.len() >= 64 {
            c.clear();
        }
        c.insert(key, h);
    }
}

/// What a compiled hybrid depends on: the slots (the formulas' code and
/// option values as the machine holds them, iterations, counted, analytic
/// DE, built-in formulas and their options) and the loop.
pub fn hybrid_key(m: &Machine, slots: &[HybridSlot], start: usize, end: usize, repeat: usize, four_d: bool) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (start, end, repeat, four_d, slots.len()).hash(&mut h);
    for (n, s) in slots.iter().enumerate() {
        (s.iterations, s.uncounted, s.ade).hash(&mut h);
        match &s.formula {
            Formula::Custom(_) => {
                m.prog(crate::custom::code_addr(n)).map(|p| p.hash).hash(&mut h);
                let a = mem_off(crate::custom::var_buf_addr(n)) as usize;
                m.mem.get(a..a + 0x400).hash(&mut h);
            }
            f => format!("{f:?}").hash(&mut h),
        }
    }
    h.finish()
}

/// A formula compiled on its own with the interface of the translated
/// direct formulas (for the comparison with them): (iteration, memory image,
/// machine, step limit) -> 0, or 1 on an error.
pub type CompiledDirect = unsafe extern "C" fn(*mut Iteration, *mut u8, *const Machine, u64) -> i32;

/// A JIT module; its code lives as long as it does.
pub struct Jit {
    module: JITModule,
    helpers: Vec<cranelift_module::FuncId>,
}

impl Jit {
    pub fn new() -> Option<Jit> {
        let mut fb = settings::builder();
        fb.set("opt_level", "speed").ok()?;
        let isa = cranelift_native::builder().ok()?.finish(settings::Flags::new(fb)).ok()?;
        let mut jb = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        for (name, addr) in helper_addresses() {
            jb.symbol(name, addr);
        }
        jb.symbol("mb3d_builtin", mb3d_builtin as *const u8);
        let mut module = JITModule::new(jb);
        let mut helpers = Vec::new();
        for (name, nargs) in HELPERS {
            let mut sig = module.make_signature();
            if name == "mb3d_round_int" {
                sig.params.push(AbiParam::new(module.target_config().pointer_type()));
            }
            for _ in 0..nargs {
                sig.params.push(AbiParam::new(F64));
            }
            sig.returns.push(AbiParam::new(F64));
            helpers.push(module.declare_function(name, Linkage::Import, &sig).ok()?);
        }
        Some(Jit { module, helpers })
    }

    /// The formula at `entry` of machine `m` (in the slot whose `PVar` is
    /// `pvar`) compiled on its own; None if it cannot be compiled.
    pub fn compile_direct(&mut self, m: &Machine, entry: u32, pvar: u32) -> Option<CompiledDirect> {
        let prog = m.prog(entry)?;
        if prog.clif_unsupported().is_some() {
            return None;
        }
        let ptr = self.module.target_config().pointer_type();
        let mut ctx = self.module.make_context();
        for ty in [ptr, ptr, ptr, I64] {
            ctx.func.signature.params.push(AbiParam::new(ty));
        }
        ctx.func.signature.returns.push(AbiParam::new(I32));
        let mut fctx = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
            let refs: Vec<_> = self.helpers.iter().map(|id| self.module.declare_func_in_func(*id, b.func)).collect();
            let helpers = Helpers(refs.try_into().ok()?);
            let start = b.create_block();
            let err = b.create_block();
            b.append_block_params_for_function_params(start);
            b.switch_to_block(start);
            let p = b.block_params(start).to_vec();
            let steps = b.declare_var(I64);
            let zero = b.ins().iconst(I64, 0);
            b.def_var(steps, zero);
            let sse = prog.clif_uses_sse();
            let xmm = xmm_vars(&mut b, p[2], sse);
            let env = Env { mem: p[1], mach: p[2], pvar, var_bytes: &m.mem[..], helpers: &helpers, steps, max_steps: p[3], err, xmm };
            let mut rec = DirectRecord { it: p[0], mem: p[1], pvar };
            let cont = prog.clif_emit(&mut b, &env, &mut rec)?;
            b.switch_to_block(cont);
            if sse {
                xmm_store(&mut b, p[2], &xmm);
            }
            let ok = b.ins().iconst(I32, 0);
            b.ins().return_(&[ok]);
            b.switch_to_block(err);
            let e = b.ins().iconst(I32, 1);
            b.ins().return_(&[e]);
            b.seal_all_blocks();
            b.finalize();
        }
        let id = self.module.declare_anonymous_function(&ctx.func.signature).ok()?;
        if let Err(e) = self.module.define_function(id, &mut ctx) {
            eprintln!("cranelift: {e:?}");
            return None;
        }
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().ok()?;
        let f = self.module.get_finalized_function(id);
        // SAFETY: the function was built with this signature
        Some(unsafe { std::mem::transmute::<*const u8, CompiledDirect>(f) })
    }
}

/// The record cells of a direct formula: the fields `rec_get` / `rec_set`
/// take from and put into the `Iteration`, the others in the memory image.
struct DirectRecord {
    it: Value,
    mem: Value,
    pvar: u32,
}

/// Offset of an `Iteration` field of cell `c` that `rec_get` reads as a
/// whole double.
fn it_double(c: u32) -> Option<usize> {
    Some(match c {
        0 => offset_of!(Iteration, j) + 24,
        1 => offset_of!(Iteration, rold),
        3..=6 => offset_of!(Iteration, v) + 8 * (c as usize - 3),
        7..=9 => offset_of!(Iteration, c) + 8 * (c as usize - 7),
        10..=12 => offset_of!(Iteration, j) + 8 * (c as usize - 10),
        14 => offset_of!(Iteration, rout),
        31 => offset_of!(Iteration, otrap),
        32 => offset_of!(Iteration, vary_scale),
        34 => offset_of!(Iteration, dfree),
        35 => offset_of!(Iteration, dfree) + 8,
        36 => offset_of!(Iteration, deriv1),
        37 => offset_of!(Iteration, deriv2),
        38 => offset_of!(Iteration, deriv3),
        _ => return None,
    })
}

/// The SSE registers of the compiled code: loaded from the machine when
/// `used` (else unused zeros).
fn xmm_vars(b: &mut FunctionBuilder, mach: Value, used: bool) -> [[Variable; 2]; 8] {
    let z = b.ins().f64const(0.0);
    std::array::from_fn(|r| {
        std::array::from_fn(|k| {
            let v = b.declare_var(F64);
            let x = if used { b.ins().load(F64, MemFlags::trusted(), mach, (offset_of!(Machine, xmm) + 16 * r + 8 * k) as i32) } else { z };
            b.def_var(v, x);
            v
        })
    })
}

/// The SSE registers back into the machine.
fn xmm_store(b: &mut FunctionBuilder, mach: Value, xmm: &[[Variable; 2]; 8]) {
    for (r, x) in xmm.iter().enumerate() {
        for (k, v) in x.iter().enumerate() {
            let val = b.use_var(*v);
            b.ins().store(MemFlags::trusted(), val, mach, (offset_of!(Machine, xmm) + 16 * r + 8 * k) as i32);
        }
    }
}

fn mem_off(a: u32) -> i32 {
    ((a.wrapping_sub(BASE) as usize) & (MEM_SIZE - 1)) as i32
}

impl Record for DirectRecord {
    fn get(&mut self, b: &mut FunctionBuilder, c: u32, ty: Type) -> Value {
        let fl = MemFlags::trusted();
        if let Some(o) = it_double(c) {
            return b.ins().load(ty, fl, self.it, o as i32);
        }
        let memv = b.ins().load(I64, fl, self.mem, mem_off(crate::custom::IT_BASE + 8 * c));
        // the low half from the iteration, the high half from the memory
        let low = |b: &mut FunctionBuilder, v: Value| -> Value {
            let v = b.ins().uextend(I64, v);
            let h = b.ins().band_imm(memv, !0xFFFF_FFFFi64);
            b.ins().bor(h, v)
        };
        let v = match c {
            13 => {
                let p = b.ins().iconst(I32, self.pvar as i64);
                low(b, p)
            }
            15 => {
                let r = b.ins().load(I32, fl, self.it, offset_of!(Iteration, it_result) as i32);
                let mx = b.ins().load(I32, fl, self.it, offset_of!(Iteration, max_it) as i32);
                let r = b.ins().uextend(I64, r);
                let mx = b.ins().uextend(I64, mx);
                let mx = b.ins().ishl_imm(mx, 32);
                b.ins().bor(r, mx)
            }
            16 => {
                let rs = b.ins().load(F64, fl, self.it, offset_of!(Iteration, rstop) as i32);
                let f = b.ins().fdemote(cranelift_codegen::ir::types::F32, rs);
                let bits = b.ins().bitcast(I32, MemFlags::new(), f);
                low(b, bits)
            }
            33 => {
                let fi = b.ins().load(I32, fl, self.it, offset_of!(Iteration, first_it) as i32);
                low(b, fi)
            }
            _ => memv,
        };
        if ty == F64 {
            b.ins().bitcast(F64, MemFlags::new(), v)
        } else {
            v
        }
    }

    fn set(&mut self, b: &mut FunctionBuilder, c: u32, v: Value) {
        let fl = MemFlags::trusted();
        let settable = matches!(c, 0 | 3..=6 | 10..=12 | 14 | 31 | 32 | 34..=38);
        if settable {
            b.ins().store(fl, v, self.it, it_double(c).unwrap() as i32);
            return;
        }
        let bits = if b.func.dfg.value_type(v) == F64 { b.ins().bitcast(I64, MemFlags::new(), v) } else { v };
        if c == 33 {
            let lo = b.ins().ireduce(I32, bits);
            b.ins().store(fl, lo, self.it, offset_of!(Iteration, first_it) as i32);
        }
        b.ins().store(fl, bits, self.mem, mem_off(crate::custom::IT_BASE + 8 * c));
    }
}

// ---------------------------------------------------------------------------
// The alternating hybrid loop as one compiled function
// ---------------------------------------------------------------------------

use crate::formulas::Formula;
use crate::iteration::HybridSlot;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{StackSlotData, StackSlotKind};
use cranelift_frontend::Variable;

/// The compiled loop: (iteration, memory image, machine, step limit, slots,
/// formula errors) -> the index of the last formula used.
pub type HybridFn = unsafe extern "C" fn(*mut Iteration, *mut u8, *const Machine, u64, *const HybridSlot, *mut u64) -> i32;

/// A hybrid compiled for one slot list (`Iteration::run` with these
/// parameters calls it instead of the formulas one by one).
pub struct HybridJit {
    pub f: HybridFn,
    pub start: usize,
    pub end: usize,
    pub repeat: usize,
    pub four_d: bool,
    pub slots: usize,
    /// owns the code
    _module: JITModule,
}

// SAFETY: the module is only kept to own the finished code; it is not used
// after `compile_hybrid`, and the code is immutable
unsafe impl Send for HybridJit {}
unsafe impl Sync for HybridJit {}

impl std::fmt::Debug for HybridJit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HybridJit({} slots)", self.slots)
    }
}

/// A built-in formula of a compiled hybrid (`Formula::iterate`).
///
/// # Safety
/// `slots` has more than `n` elements; `v` and `j` point to arrays (the
/// compiled code passes its slots and two arrays of its stack).
pub unsafe extern "C" fn mb3d_builtin(slots: *const HybridSlot, n: usize, v: *mut [f64; 4], j: *mut [f64; 4], rout: f64) {
    // SAFETY: see above
    let (s, v, j) = unsafe { (&*slots.add(n), &mut *v, &mut *j) };
    s.formula.iterate(v, j, rout, s.ade);
}

/// The double fields of the iteration a hybrid writes (cells, see
/// `it_double`) are variables of the compiled loop; the ones it only reads
/// are loaded where they are used (kept out of the registers: a variable
/// lives through the whole loop).
struct Fused {
    vars: std::collections::HashMap<u32, Variable>,
    it: Value,
    it_result: Variable,
    first_it: Variable,
    mem: Value,
    pvar: u32,
}

/// The cells `rec_set` writes into the iteration (the others go to the
/// memory image).
fn it_settable(c: u32) -> bool {
    matches!(c, 0 | 3..=6 | 10..=12 | 14 | 31 | 32 | 34..=38)
}

impl Fused {
    /// double field of cell `c`
    fn read(&self, b: &mut FunctionBuilder, c: u32) -> Value {
        match self.vars.get(&c) {
            Some(v) => b.use_var(*v),
            None => b.ins().load(F64, MemFlags::trusted(), self.it, it_double(c).expect("a double field") as i32),
        }
    }

    fn write(&self, b: &mut FunctionBuilder, c: u32, x: Value) {
        b.def_var(*self.vars.get(&c).expect("a written field"), x);
    }

    fn load_i32(&self, b: &mut FunctionBuilder, off: usize) -> Value {
        b.ins().load(I32, MemFlags::trusted(), self.it, off as i32)
    }
}

impl Record for Fused {
    fn get(&mut self, b: &mut FunctionBuilder, c: u32, ty: Type) -> Value {
        if it_double(c).is_some() {
            let x = self.read(b, c);
            return if ty == F64 { x } else { b.ins().bitcast(I64, MemFlags::new(), x) };
        }
        let fl = MemFlags::trusted();
        let memv = b.ins().load(I64, fl, self.mem, mem_off(crate::custom::IT_BASE + 8 * c));
        let low = |b: &mut FunctionBuilder, v: Value| -> Value {
            let v = b.ins().uextend(I64, v);
            let h = b.ins().band_imm(memv, !0xFFFF_FFFFi64);
            b.ins().bor(h, v)
        };
        let v = match c {
            13 => {
                let p = b.ins().iconst(I32, self.pvar as i64);
                low(b, p)
            }
            15 => {
                let r = b.use_var(self.it_result);
                let mx = self.load_i32(b, offset_of!(Iteration, max_it));
                let r = b.ins().uextend(I64, r);
                let mx = b.ins().uextend(I64, mx);
                let mx = b.ins().ishl_imm(mx, 32);
                b.ins().bor(r, mx)
            }
            16 => {
                let rs = b.ins().load(F64, fl, self.it, offset_of!(Iteration, rstop) as i32);
                let f = b.ins().fdemote(cranelift_codegen::ir::types::F32, rs);
                let bits = b.ins().bitcast(I32, MemFlags::new(), f);
                low(b, bits)
            }
            33 => {
                let fi = b.use_var(self.first_it);
                low(b, fi)
            }
            _ => memv,
        };
        if ty == F64 {
            b.ins().bitcast(F64, MemFlags::new(), v)
        } else {
            v
        }
    }

    fn set(&mut self, b: &mut FunctionBuilder, c: u32, v: Value) {
        let is_f = b.func.dfg.value_type(v) == F64;
        if it_settable(c) {
            let x = if is_f { v } else { b.ins().bitcast(F64, MemFlags::new(), v) };
            self.write(b, c, x);
            return;
        }
        let bits = if is_f { b.ins().bitcast(I64, MemFlags::new(), v) } else { v };
        if c == 33 {
            let lo = b.ins().ireduce(I32, bits);
            b.def_var(self.first_it, lo);
        }
        b.ins().store(MemFlags::trusted(), bits, self.mem, mem_off(crate::custom::IT_BASE + 8 * c));
    }
}

/// The alternating hybrid loop (`Iteration::run_with`) of `slots` from
/// `start` (end `end`, repeat from `repeat`) compiled with the formulas of
/// machine `m` (the prototype of the scene) inlined; None if a formula
/// cannot be compiled (or the loop is one of the rare kinds left to the
/// interpreter: no counted formula, MB3D's JIT formulas).
pub fn compile_hybrid(m: &Machine, slots: &[HybridSlot], start: usize, end: usize, repeat: usize, four_d: bool) -> Option<HybridJit> {
    if slots.is_empty() || end >= slots.len() || repeat > end || start > end {
        return None;
    }
    // every slot of the cycle must be compilable; the cycle must count
    for s in slots {
        if let Formula::Custom(c) = &s.formula {
            if c.def.jit.is_some() {
                return None;
            }
        }
    }
    // the double fields written in the loop: v, rout, rold, otrap, those
    // the formulas write, j when a built-in formula runs through the helper
    let mut written: std::collections::BTreeSet<u32> = [1, 3, 4, 5, 6, 14, 31].into_iter().collect();
    for (n, s) in slots.iter().enumerate().take(end + 1) {
        if s.iterations <= 0 {
            continue;
        }
        match &s.formula {
            Formula::Custom(_) => {
                let prog = m.prog(crate::custom::code_addr(n))?;
                if prog.clif_unsupported().is_some() {
                    return None;
                }
                written.extend(prog.clif_record_written()?.into_iter().filter(|c| it_settable(*c)));
            }
            f if super::clif_builtin::supported(f) => {
                if super::clif_builtin::writes_j(f) {
                    written.extend([0, 10, 11, 12]);
                }
            }
            _ => written.extend([0, 10, 11, 12]),
        }
    }
    if !slots[repeat..=end].iter().any(|s| s.iterations > 0 && !s.uncounted) {
        return None;
    }
    let mut jit = Jit::new()?;
    let ptr = jit.module.target_config().pointer_type();
    let mut sig = jit.module.make_signature();
    // the built-in formula helper
    let mut bsig = jit.module.make_signature();
    for ty in [ptr, ptr, ptr, ptr] {
        bsig.params.push(AbiParam::new(ty));
    }
    bsig.params.push(AbiParam::new(F64));
    let builtin = jit.module.declare_function("mb3d_builtin", Linkage::Import, &bsig).ok()?;
    for ty in [ptr, ptr, ptr, I64, ptr, ptr] {
        sig.params.push(AbiParam::new(ty));
    }
    sig.returns.push(AbiParam::new(I32));
    let mut ctx = jit.module.make_context();
    ctx.func.signature = sig;
    let mut fctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let refs: Vec<_> = jit.helpers.iter().map(|id| jit.module.declare_func_in_func(*id, b.func)).collect();
        let helpers = Helpers(refs.try_into().ok()?);
        let builtin_ref = jit.module.declare_func_in_func(builtin, b.func);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let p = b.block_params(entry).to_vec();
        let (itp, memp, machp, max_steps, slotsp, errsp) = (p[0], p[1], p[2], p[3], p[4], p[5]);
        let fl = MemFlags::trusted();
        let mut vars = std::collections::HashMap::new();
        for c in &written {
            let v = b.declare_var(F64);
            let x = b.ins().load(F64, fl, itp, it_double(*c)? as i32);
            b.def_var(v, x);
            vars.insert(*c, v);
        }
        let mut st = Fused { vars, it: itp, it_result: b.declare_var(I32), first_it: b.declare_var(I32), mem: memp, pvar: 0 };
        // run_with: it_result = 0, first_it = 0
        let z32 = b.ins().iconst(I32, 0);
        b.def_var(st.it_result, z32);
        b.def_var(st.first_it, z32);
        let uguard = b.declare_var(I32);
        b.def_var(uguard, z32);
        let btmp = b.declare_var(I32);
        let steps = b.declare_var(I64);
        let zero64 = b.ins().iconst(I64, 0);
        b.def_var(steps, zero64);
        let errs = b.declare_var(I64);
        b.def_var(errs, zero64);
        let sse = slots.iter().enumerate().take(end + 1).any(|(n, s)| s.iterations > 0 && matches!(s.formula, Formula::Custom(_)) && m.prog(crate::custom::code_addr(n)).is_some_and(|p| p.clif_uses_sse()));
        let xmm = xmm_vars(&mut b, machp, sse);
        // the v and j arrays for the built-in formulas through the helper
        let vj = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 64, 3));
        let exit = b.create_block();
        b.append_block_param(exit, I32);
        // per slot: enter (btmp = iterations) and the body
        let enter: Vec<_> = (0..=end).map(|_| b.create_block()).collect();
        let body: Vec<_> = (0..=end).map(|_| b.create_block()).collect();
        let next = |n: usize| if n + 1 > end { repeat } else { n + 1 };
        const V: [u32; 4] = [3, 4, 5, 6];
        const J: [u32; 4] = [10, 11, 12, 0];
        b.ins().jump(enter[start], &[]);
        for n in 0..=end {
            let s = &slots[n];
            // enter: a slot without iterations is skipped (the loop top's
            // `rold = rout` comes first, it changes nothing here)
            b.switch_to_block(enter[n]);
            if s.iterations <= 0 {
                b.ins().jump(enter[next(n)], &[]);
                continue;
            }
            let it = b.ins().iconst(I32, s.iterations as i64);
            b.def_var(btmp, it);
            b.ins().jump(body[n], &[]);
            b.switch_to_block(body[n]);
            let ro = st.read(&mut b, 14);
            st.write(&mut b, 1, ro);
            // the formula
            let after = b.create_block();
            match &s.formula {
                Formula::Custom(_) => {
                    let prog = m.prog(crate::custom::code_addr(n))?;
                    let err = b.create_block();
                    b.def_var(steps, zero64);
                    st.pvar = crate::custom::pconst_addr(n);
                    let env = Env { mem: memp, mach: machp, pvar: st.pvar, var_bytes: &m.mem[..], helpers: &helpers, steps, max_steps, err, xmm };
                    let cont = prog.clif_emit(&mut b, &env, &mut st)?;
                    b.switch_to_block(cont);
                    b.ins().jump(after, &[]);
                    // an error: counted; v and rout infinite (run_custom)
                    b.switch_to_block(err);
                    let e = b.use_var(errs);
                    let e = b.ins().iadd_imm(e, 1);
                    b.def_var(errs, e);
                    let inf = b.ins().f64const(f64::INFINITY);
                    for c in V {
                        st.write(&mut b, c, inf);
                    }
                    st.write(&mut b, 14, inf);
                    b.ins().jump(after, &[]);
                }
                f if super::clif_builtin::supported(f) => {
                    let mut v: [Value; 4] = V.map(|c| st.read(&mut b, c));
                    let mut jv: [Value; 4] = J.map(|c| st.read(&mut b, c));
                    let ro = st.read(&mut b, 14);
                    super::clif_builtin::emit(&mut b, &helpers, f, &mut v, &mut jv, ro, s.ade);
                    for (c, x) in V.iter().zip(v) {
                        st.write(&mut b, *c, x);
                    }
                    if super::clif_builtin::writes_j(f) {
                        for (c, x) in J.iter().zip(jv) {
                            st.write(&mut b, *c, x);
                        }
                    }
                    b.ins().jump(after, &[]);
                }
                _ => {
                    for k in 0..4 {
                        let x = st.read(&mut b, V[k]);
                        b.ins().stack_store(x, vj, 8 * k as i32);
                        let y = st.read(&mut b, J[k]);
                        b.ins().stack_store(y, vj, 32 + 8 * k as i32);
                    }
                    let vp = b.ins().stack_addr(ptr, vj, 0);
                    let jp = b.ins().stack_addr(ptr, vj, 32);
                    let nn = b.ins().iconst(ptr, n as i64);
                    let ro = st.read(&mut b, 14);
                    b.ins().call(builtin_ref, &[slotsp, nn, vp, jp, ro]);
                    for k in 0..4 {
                        let x = b.ins().stack_load(F64, vj, 8 * k as i32);
                        st.write(&mut b, V[k], x);
                        let y = b.ins().stack_load(F64, vj, 32 + 8 * k as i32);
                        st.write(&mut b, J[k], y);
                    }
                    b.ins().jump(after, &[]);
                }
            }
            b.switch_to_block(after);
            let bt = b.use_var(btmp);
            let bt = b.ins().iadd_imm(bt, -1);
            b.def_var(btmp, bt);
            let more = b.ins().icmp_imm(IntCC::SignedGreaterThan, bt, 0);
            let nn = b.ins().iconst(I32, n as i64);
            if s.uncounted {
                let g = b.use_var(uguard);
                let g = b.ins().iadd_imm(g, 1);
                b.def_var(uguard, g);
                let over = b.ins().icmp_imm(IntCC::SignedGreaterThan, g, 100_000);
                let go = b.create_block();
                b.ins().brif(over, exit, &[nn.into()], go, &[]);
                b.switch_to_block(go);
                b.ins().brif(more, body[n], &[], enter[next(n)], &[]);
            } else {
                let r = b.use_var(st.it_result);
                let r = b.ins().iadd_imm(r, 1);
                b.def_var(st.it_result, r);
                let x = st.read(&mut b, 3);
                let y = st.read(&mut b, 4);
                let z = st.read(&mut b, 5);
                let xx = b.ins().fmul(x, x);
                let yy = b.ins().fmul(y, y);
                let zz = b.ins().fmul(z, z);
                let s2 = b.ins().fadd(xx, yy);
                let mut ro = b.ins().fadd(s2, zz);
                if four_d {
                    let w = st.read(&mut b, 6);
                    let ww = b.ins().fmul(w, w);
                    ro = b.ins().fadd(ro, ww);
                }
                st.write(&mut b, 14, ro);
                let ot = st.read(&mut b, 31);
                let lt = b.ins().fcmp(FloatCC::LessThan, ro, ot);
                let not = b.ins().select(lt, ro, ot);
                st.write(&mut b, 31, not);
                let mx = st.load_i32(&mut b, offset_of!(Iteration, max_it));
                let c1 = b.ins().icmp(IntCC::SignedGreaterThanOrEqual, r, mx);
                let rs = b.ins().load(F64, fl, itp, offset_of!(Iteration, rstop) as i32);
                let c2 = b.ins().fcmp(FloatCC::GreaterThan, ro, rs);
                let done = b.ins().bor(c1, c2);
                let go = b.create_block();
                b.ins().brif(done, exit, &[nn.into()], go, &[]);
                b.switch_to_block(go);
                b.ins().brif(more, body[n], &[], enter[next(n)], &[]);
            }
        }
        // the end: the written fields back into the iteration
        b.switch_to_block(exit);
        let res = b.block_params(exit)[0];
        for c in &written {
            let x = st.read(&mut b, *c);
            b.ins().store(fl, x, itp, it_double(*c)? as i32);
        }
        let r = b.use_var(st.it_result);
        b.ins().store(fl, r, itp, offset_of!(Iteration, it_result) as i32);
        let fi = b.use_var(st.first_it);
        b.ins().store(fl, fi, itp, offset_of!(Iteration, first_it) as i32);
        let e = b.use_var(errs);
        b.ins().store(fl, e, errsp, 0);
        if sse {
            xmm_store(&mut b, machp, &xmm);
        }
        b.ins().return_(&[res]);
        b.seal_all_blocks();
        b.finalize();
    }
    if std::env::var_os("MB3D_JIT_DUMP").is_some() {
        eprintln!("{}", ctx.func.display());
    }
    let asm = std::env::var_os("MB3D_JIT_ASM").is_some();
    ctx.set_disasm(asm);
    let id = jit.module.declare_anonymous_function(&ctx.func.signature).ok()?;
    if let Err(e) = jit.module.define_function(id, &mut ctx) {
        eprintln!("cranelift: {e:?}");
        return None;
    }
    if asm {
        if let Some(cc) = ctx.compiled_code() {
            eprintln!("{}", cc.vcode.as_deref().unwrap_or(""));
        }
    }
    jit.module.clear_context(&mut ctx);
    jit.module.finalize_definitions().ok()?;
    let f = jit.module.get_finalized_function(id);
    Some(HybridJit {
        // SAFETY: the function was built with this signature
        f: unsafe { std::mem::transmute::<*const u8, HybridFn>(f) },
        start,
        end,
        repeat,
        four_d,
        slots: slots.len(),
        _module: jit.module,
    })
}

/// A built-in formula compiled on its own (for the comparison with
/// `Formula::iterate`): (v, j, rout).
pub type CompiledBuiltin = unsafe extern "C" fn(*mut [f64; 4], *mut [f64; 4], f64);

impl Jit {
    /// `f` compiled on its own (`clif_builtin`); None if it is not one of the
    /// compiled built-ins.
    pub fn compile_builtin(&mut self, f: &Formula, ade: bool) -> Option<CompiledBuiltin> {
        if !super::clif_builtin::supported(f) {
            return None;
        }
        let ptr = self.module.target_config().pointer_type();
        let mut ctx = self.module.make_context();
        ctx.func.signature.params.extend([AbiParam::new(ptr), AbiParam::new(ptr), AbiParam::new(F64)]);
        let mut fctx = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
            let refs: Vec<_> = self.helpers.iter().map(|id| self.module.declare_func_in_func(*id, b.func)).collect();
            let helpers = Helpers(refs.try_into().ok()?);
            let e = b.create_block();
            b.append_block_params_for_function_params(e);
            b.switch_to_block(e);
            let p = b.block_params(e).to_vec();
            let fl = MemFlags::trusted();
            let mut v: [Value; 4] = std::array::from_fn(|k| b.ins().load(F64, fl, p[0], 8 * k as i32));
            let mut j: [Value; 4] = std::array::from_fn(|k| b.ins().load(F64, fl, p[1], 8 * k as i32));
            super::clif_builtin::emit(&mut b, &helpers, f, &mut v, &mut j, p[2], ade);
            for (k, x) in v.iter().enumerate() {
                b.ins().store(fl, *x, p[0], 8 * k as i32);
            }
            for (k, x) in j.iter().enumerate() {
                b.ins().store(fl, *x, p[1], 8 * k as i32);
            }
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
        }
        let id = self.module.declare_anonymous_function(&ctx.func.signature).ok()?;
        self.module.define_function(id, &mut ctx).ok()?;
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().ok()?;
        // SAFETY: the function was built with this signature
        Some(unsafe { std::mem::transmute::<*const u8, CompiledBuiltin>(self.module.get_finalized_function(id)) })
    }
}

/// A dIFS formula compiled on its own with the interface of the translated
/// ones (for the comparison with them): (registers in and out, memory
/// image, machine, step limit) -> 0, or 1 on an error.
pub type CompiledDifs = unsafe extern "C" fn(*mut [u32; 8], *mut u8, *const Machine, u64) -> i32;

/// The record of a dIFS formula: the cells in the memory image.
struct MemRecord {
    mem: Value,
}

impl Record for MemRecord {
    fn get(&mut self, b: &mut FunctionBuilder, c: u32, ty: Type) -> Value {
        b.ins().load(ty, MemFlags::trusted(), self.mem, mem_off(crate::custom::IT_BASE + 8 * c))
    }

    fn set(&mut self, b: &mut FunctionBuilder, c: u32, v: Value) {
        b.ins().store(MemFlags::trusted(), v, self.mem, mem_off(crate::custom::IT_BASE + 8 * c));
    }
}

impl Jit {
    /// The dIFS formula at `entry` of machine `m` (slot `PVar` `pvar`)
    /// compiled on its own; None if it cannot be compiled.
    pub fn compile_difs_one(&mut self, m: &Machine, entry: u32, pvar: u32) -> Option<CompiledDifs> {
        let prog = m.prog(entry)?;
        if prog.clif_unsupported_mode(true).is_some() || handwritten::lookup_difs(prog.hash, 0).is_some() {
            return None;
        }
        let ptr = self.module.target_config().pointer_type();
        let mut ctx = self.module.make_context();
        for ty in [ptr, ptr, ptr, I64] {
            ctx.func.signature.params.push(AbiParam::new(ty));
        }
        ctx.func.signature.returns.push(AbiParam::new(I32));
        let mut fctx = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
            let refs: Vec<_> = self.helpers.iter().map(|id| self.module.declare_func_in_func(*id, b.func)).collect();
            let helpers = Helpers(refs.try_into().ok()?);
            let start = b.create_block();
            let err = b.create_block();
            b.append_block_params_for_function_params(start);
            b.switch_to_block(start);
            let p = b.block_params(start).to_vec();
            let fl = MemFlags::trusted();
            let regs: [Value; 8] = std::array::from_fn(|r| b.ins().load(I32, fl, p[0], 4 * r as i32));
            let steps = b.declare_var(I64);
            let zero = b.ins().iconst(I64, 0);
            b.def_var(steps, zero);
            let sse = prog.clif_uses_sse();
            let xmm = xmm_vars(&mut b, p[2], sse);
            let env = Env { mem: p[1], mach: p[2], pvar, var_bytes: &m.mem[..], helpers: &helpers, steps, max_steps: p[3], err, xmm };
            let mut rec = MemRecord { mem: p[1] };
            let (cont, out) = prog.clif_emit_mode(&mut b, &env, &mut rec, Some(regs))?;
            b.switch_to_block(cont);
            for (r, v) in out.iter().enumerate() {
                b.ins().store(fl, *v, p[0], 4 * r as i32);
            }
            if sse {
                xmm_store(&mut b, p[2], &xmm);
            }
            let ok = b.ins().iconst(I32, 0);
            b.ins().return_(&[ok]);
            b.switch_to_block(err);
            let e = b.ins().iconst(I32, 1);
            b.ins().return_(&[e]);
            b.seal_all_blocks();
            b.finalize();
        }
        let id = self.module.declare_anonymous_function(&ctx.func.signature).ok()?;
        if let Err(e) = self.module.define_function(id, &mut ctx) {
            eprintln!("cranelift: {e:?}");
            return None;
        }
        self.module.clear_context(&mut ctx);
        self.module.finalize_definitions().ok()?;
        // SAFETY: the function was built with this signature
        Some(unsafe { std::mem::transmute::<*const u8, CompiledDifs>(self.module.get_finalized_function(id)) })
    }
}

// ---------------------------------------------------------------------------
// The dIFS hybrid loop as one compiled function
// ---------------------------------------------------------------------------

/// What the compiled dIFS loop hands back (`Iteration::hybrid_difs`).
#[repr(C)]
#[derive(Default)]
pub struct DifsOut {
    pub it_min: i32,
    pub min_de: f64,
    pub dfree2: f64,
}

/// The compiled dIFS loop: (memory image, machine, step limit, inside,
/// RStopD, max iterations, out) -> 0 ended, 1 a formula failed, 2 left to
/// the formulas called one by one (nothing changed: a formula sent the loop
/// where the compiled code does not go).
pub type DifsLoopFn = unsafe extern "C" fn(*mut u8, *const Machine, u64, i32, f64, i32, *mut DifsOut) -> i32;

/// A dIFS hybrid compiled for one slot list.
pub struct DifsJit {
    pub f: DifsLoopFn,
    pub start: usize,
    pub end: usize,
    pub repeat: usize,
    pub slots: usize,
    _module: JITModule,
}

// SAFETY: as for `HybridJit`
unsafe impl Send for DifsJit {}
unsafe impl Sync for DifsJit {}

impl std::fmt::Debug for DifsJit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DifsJit({} slots)", self.slots)
    }
}

/// The record cells of the compiled dIFS loop: variables (loaded from the
/// memory image at the start).
struct DifsVars {
    vars: std::collections::HashMap<u32, (Variable, Type)>,
}

impl Record for DifsVars {
    fn get(&mut self, b: &mut FunctionBuilder, c: u32, ty: Type) -> Value {
        let (v, vt) = self.vars[&c];
        let x = b.use_var(v);
        if vt == ty {
            x
        } else {
            b.ins().bitcast(ty, MemFlags::new(), x)
        }
    }

    fn set(&mut self, b: &mut FunctionBuilder, c: u32, x: Value) {
        let (v, vt) = self.vars[&c];
        let x = if b.func.dfg.value_type(x) == vt { x } else { b.ins().bitcast(vt, MemFlags::new(), x) };
        b.def_var(v, x);
    }
}

/// The dIFS loop (`Iteration::hybrid_difs` after its set up) of `slots`
/// compiled with the formulas of machine `m` inlined; None if a formula
/// cannot be compiled (or writes its variable buffer).
pub fn compile_difs(m: &Machine, slots: &[HybridSlot], start: usize, end: usize, repeat: usize) -> Option<DifsJit> {
    use crate::custom::{code_addr, pconst_addr, IT_BASE, STACK_TOP};
    if slots.is_empty() || slots.len() > 32 || end >= slots.len() || repeat > end {
        return None;
    }
    // every slot with iterations: a compilable dIFS formula
    let mut cells: std::collections::BTreeMap<u32, bool> = [(14, true), (32, true), (34, true), (15, false)].into_iter().collect();
    let mut written: std::collections::BTreeSet<u32> = [15].into_iter().collect();
    let mut compiled = vec![false; slots.len()];
    for (n, s) in slots.iter().enumerate() {
        let Formula::Custom(c) = &s.formula else {
            if s.iterations > 0 {
                return None;
            }
            continue;
        };
        if c.def.jit.is_some() {
            return None;
        }
        let Some(prog) = m.prog(code_addr(n)) else { continue };
        // the formulas written in Rust (handwritten.rs): their code is only
        // a placeholder
        if prog.clif_unsupported_mode(true).is_some() || handwritten::lookup_difs(prog.hash, 0).is_some() {
            if s.iterations > 0 {
                return None;
            }
            continue;
        }
        let (used, w, vars) = prog.clif_difs_cells()?;
        if vars {
            return None;
        }
        for (c, whole) in used {
            let e = cells.entry(c).or_insert(true);
            *e &= whole;
        }
        written.extend(w);
        compiled[n] = true;
    }
    let mut jit = Jit::new()?;
    let ptr = jit.module.target_config().pointer_type();
    let mut sig = jit.module.make_signature();
    for ty in [ptr, ptr, I64, I32, F64, I32, ptr] {
        sig.params.push(AbiParam::new(ty));
    }
    sig.returns.push(AbiParam::new(I32));
    let mut ctx = jit.module.make_context();
    ctx.func.signature = sig;
    let mut fctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let refs: Vec<_> = jit.helpers.iter().map(|id| jit.module.declare_func_in_func(*id, b.func)).collect();
        let helpers = Helpers(refs.try_into().ok()?);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let p = b.block_params(entry).to_vec();
        let (memp, machp, max_steps, insidep, rstopd, max_it, outp) = (p[0], p[1], p[2], p[3], p[4], p[5], p[6]);
        let fl = MemFlags::trusted();
        // the record cells
        let mut vars = std::collections::HashMap::new();
        for (c, whole) in &cells {
            let ty = if *whole { F64 } else { I64 };
            let v = b.declare_var(ty);
            let x = b.ins().load(ty, fl, memp, mem_off(IT_BASE + 8 * c));
            b.def_var(v, x);
            vars.insert(*c, (v, ty));
        }
        let mut rec = DifsVars { vars };
        let sse = slots.iter().enumerate().any(|(n, _)| compiled[n] && m.prog(code_addr(n)).is_some_and(|p| p.clif_uses_sse()));
        let xmm = xmm_vars(&mut b, machp, sse);
        // the loop state
        let ivar = |b: &mut FunctionBuilder, v: i64| {
            let x = b.declare_var(I32);
            let c = b.ins().iconst(I32, v);
            b.def_var(x, c);
            x
        };
        let n0 = start.min(slots.len() - 1);
        let n = ivar(&mut b, n0 as i64);
        let cnt = ivar(&mut b, slots[n0].iterations.max(0) as i64);
        let edi = ivar(&mut b, pconst_addr(n0) as i64);
        let guard = ivar(&mut b, 0);
        let it = ivar(&mut b, 0);
        let it_min = ivar(&mut b, 0);
        let g = ivar(&mut b, 0);
        let min_de = b.declare_var(F64);
        let c65535 = b.ins().f64const(65535.0);
        b.def_var(min_de, c65535);
        let dfree2 = b.declare_var(F64);
        let z = b.ins().f64const(0.0);
        b.def_var(dfree2, z);
        let steps = b.declare_var(I64);
        let z64 = b.ins().iconst(I64, 0);
        b.def_var(steps, z64);
        let top = b.create_block();
        let adv = b.create_block();
        let step = b.create_block();
        let dispatch = b.create_block();
        let post = b.create_block();
        let exit_ok = b.create_block();
        let exit_fail = b.create_block();
        let deopt = b.create_block();
        b.ins().jump(top, &[]);
        // the loop top: the next slot with iterations (`while cnt <= 0`)
        b.switch_to_block(top);
        let z32 = b.ins().iconst(I32, 0);
        b.def_var(g, z32);
        b.ins().jump(adv, &[]);
        b.switch_to_block(adv);
        let c = b.use_var(cnt);
        let more = b.ins().icmp_imm(IntCC::SignedGreaterThan, c, 0);
        b.ins().brif(more, dispatch, &[], step, &[]);
        b.switch_to_block(step);
        let nv = b.use_var(n);
        let nv = b.ins().iadd_imm(nv, 1);
        let over = b.ins().icmp_imm(IntCC::SignedGreaterThan, nv, end as i64);
        let rep = b.ins().iconst(I32, repeat as i64);
        let nv = b.ins().select(over, rep, nv);
        b.def_var(n, nv);
        // cnt = iterations of n (a select chain over the slots)
        let mut cv = b.ins().iconst(I32, 0);
        for (k, s) in slots.iter().enumerate() {
            let is = b.ins().icmp_imm(IntCC::Equal, nv, k as i64);
            let it_k = b.ins().iconst(I32, s.iterations.max(0) as i64);
            cv = b.ins().select(is, it_k, cv);
        }
        b.def_var(cnt, cv);
        // edi = pconst_addr(n): linear in n
        let step_addr = (pconst_addr(1) - pconst_addr(0)) as i64;
        let e = b.ins().imul_imm(nv, step_addr);
        let e = b.ins().iadd_imm(e, pconst_addr(0) as i64);
        b.def_var(edi, e);
        let gv = b.use_var(g);
        let gv = b.ins().iadd_imm(gv, 1);
        b.def_var(g, gv);
        let gover = b.ins().icmp_imm(IntCC::SignedGreaterThan, gv, 64);
        b.ins().brif(gover, exit_ok, &[], adv, &[]);
        // dispatch: a slot whose formula is compiled, with its own PVar in
        // edi (the constants of the compiled code are that slot's)
        b.switch_to_block(dispatch);
        let nv = b.use_var(n);
        let ev = b.use_var(edi);
        let want = b.ins().imul_imm(nv, step_addr);
        let want = b.ins().iadd_imm(want, pconst_addr(0) as i64);
        let ok = b.ins().icmp(IntCC::Equal, ev, want);
        let bodies: Vec<_> = (0..slots.len()).map(|_| b.create_block()).collect();
        let table_block = b.create_block();
        b.ins().brif(ok, table_block, &[], deopt, &[]);
        b.switch_to_block(table_block);
        let targets: Vec<_> = (0..slots.len()).map(|k| if compiled[k] { b.func.dfg.block_call(bodies[k], &[]) } else { b.func.dfg.block_call(deopt, &[]) }).collect();
        let def = b.func.dfg.block_call(deopt, &[]);
        let jt = b.create_jump_table(cranelift_codegen::ir::JumpTableData::new(def, &targets));
        b.ins().br_table(nv, jt);
        let esi = IT_BASE + 144;
        for k in 0..slots.len() {
            b.switch_to_block(bodies[k]);
            if !compiled[k] {
                b.ins().jump(deopt, &[]);
                continue;
            }
            let prog = m.prog(code_addr(k))?;
            let err = b.create_block();
            b.def_var(steps, z64);
            let c = b.use_var(cnt);
            let e = b.use_var(edi);
            let zero = b.ins().iconst(I32, 0);
            let regs = [
                zero,
                c,
                b.ins().iconst(I32, (esi + 128) as i64),
                b.ins().iconst(I32, k as i64),
                b.ins().iconst(I32, (STACK_TOP - 4) as i64),
                zero,
                b.ins().iconst(I32, esi as i64),
                e,
            ];
            let env = Env { mem: memp, mach: machp, pvar: pconst_addr(k), var_bytes: &m.mem[..], helpers: &helpers, steps, max_steps, err, xmm };
            let (cont, out) = prog.clif_emit_mode(&mut b, &env, &mut rec, Some(regs))?;
            b.switch_to_block(cont);
            // cnt = ecx - 1; n = ebx (when a slot); edi
            let c = b.ins().iadd_imm(out[1], -1);
            b.def_var(cnt, c);
            let inr = b.ins().icmp_imm(IntCC::UnsignedLessThan, out[3], slots.len() as i64);
            let kv = b.ins().iconst(I32, k as i64);
            let nn = b.ins().select(inr, out[3], kv);
            b.def_var(n, nn);
            b.def_var(edi, out[7]);
            let gd = b.use_var(guard);
            let gd = b.ins().iadd_imm(gd, 1);
            b.def_var(guard, gd);
            let gover = b.ins().icmp_imm(IntCC::SignedGreaterThan, gd, 1_000_000);
            b.ins().brif(gover, exit_ok, &[], post, &[]);
            b.switch_to_block(err);
            b.ins().jump(exit_fail, &[]);
        }
        // after a formula: uncounted slots go on, counted ones count
        b.switch_to_block(post);
        let mask: u32 = slots.iter().enumerate().filter(|(_, s)| s.uncounted).fold(0, |m, (k, _)| m | 1 << k);
        let nv = b.use_var(n);
        let mk = b.ins().iconst(I32, mask as i64);
        let u = b.ins().ushr(mk, nv);
        let u = b.ins().band_imm(u, 1);
        let counted = b.create_block();
        b.ins().brif(u, top, &[], counted, &[]);
        b.switch_to_block(counted);
        let itv = b.use_var(it);
        let itv = b.ins().iadd_imm(itv, 1);
        b.def_var(it, itv);
        // IT_RESULT (the low half of cell 15)
        let c15 = rec.get(&mut b, 15, I64);
        let hi = b.ins().band_imm(c15, !0xFFFF_FFFFi64);
        let lo = b.ins().uextend(I64, itv);
        let c15 = b.ins().bor(hi, lo);
        rec.set(&mut b, 15, c15);
        let ro = rec.get(&mut b, 14, F64);
        let vs = rec.get(&mut b, 32, F64);
        let de = b.ins().fdiv(ro, vs);
        let md = b.use_var(min_de);
        let better = b.ins().fcmp(FloatCC::LessThan, de, md);
        let upd = b.create_block();
        let check_max = b.create_block();
        b.ins().brif(better, upd, &[], check_max, &[]);
        b.switch_to_block(upd);
        b.def_var(it_min, itv);
        let d1 = rec.get(&mut b, 34, F64);
        b.def_var(dfree2, d1);
        b.def_var(min_de, de);
        let outside = b.ins().icmp_imm(IntCC::Equal, insidep, 0);
        let close = b.ins().fcmp(FloatCC::LessThan, de, rstopd);
        let stop = b.ins().band(outside, close);
        b.ins().brif(stop, exit_ok, &[], check_max, &[]);
        b.switch_to_block(check_max);
        let itv = b.use_var(it);
        let done = b.ins().icmp(IntCC::SignedGreaterThanOrEqual, itv, max_it);
        b.ins().brif(done, exit_ok, &[], top, &[]);
        // the exits: the written cells back into the memory image
        let finish = |b: &mut FunctionBuilder, rec: &mut DifsVars, code: i64| {
            for c in &written {
                let (v, _) = rec.vars[c];
                let x = b.use_var(v);
                b.ins().store(fl, x, memp, mem_off(IT_BASE + 8 * c));
            }
            if sse {
                xmm_store(b, machp, &xmm);
            }
            let im = b.use_var(it_min);
            b.ins().store(fl, im, outp, std::mem::offset_of!(DifsOut, it_min) as i32);
            let md = b.use_var(min_de);
            b.ins().store(fl, md, outp, std::mem::offset_of!(DifsOut, min_de) as i32);
            let d2 = b.use_var(dfree2);
            b.ins().store(fl, d2, outp, std::mem::offset_of!(DifsOut, dfree2) as i32);
            let r = b.ins().iconst(I32, code);
            b.ins().return_(&[r]);
        };
        b.switch_to_block(exit_ok);
        finish(&mut b, &mut rec, 0);
        b.switch_to_block(exit_fail);
        finish(&mut b, &mut rec, 1);
        b.switch_to_block(deopt);
        let r = b.ins().iconst(I32, 2);
        b.ins().return_(&[r]);
        b.seal_all_blocks();
        b.finalize();
    }
    let id = jit.module.declare_anonymous_function(&ctx.func.signature).ok()?;
    if let Err(e) = jit.module.define_function(id, &mut ctx) {
        eprintln!("cranelift: {e:?}");
        return None;
    }
    jit.module.clear_context(&mut ctx);
    jit.module.finalize_definitions().ok()?;
    let f = jit.module.get_finalized_function(id);
    Some(DifsJit {
        // SAFETY: the function was built with this signature
        f: unsafe { std::mem::transmute::<*const u8, DifsLoopFn>(f) },
        start,
        end,
        repeat,
        slots: slots.len(),
        _module: jit.module,
    })
}
