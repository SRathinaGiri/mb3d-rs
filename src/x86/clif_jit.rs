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
    static ON: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| std::env::var_os("MB3D_NO_JIT").is_none_or(|v| v.is_empty() || v == "0"));
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
            let env = Env { mem: p[1], mach: p[2], pvar, var_bytes: &m.mem[..], helpers: &helpers, steps, max_steps: p[3], err };
            let mut rec = DirectRecord { it: p[0], mem: p[1], pvar };
            let cont = prog.clif_emit(&mut b, &env, &mut rec)?;
            b.switch_to_block(cont);
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
                    let env = Env { mem: memp, mach: machp, pvar: st.pvar, var_bytes: &m.mem[..], helpers: &helpers, steps, max_steps, err };
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
