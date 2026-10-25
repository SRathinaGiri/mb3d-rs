//! Cranelift code from the lifting analysis (stage 3: formulas compiled at
//! run time, so that the formulas of a hybrid can become one function).
//!
//! The ops are those of `lift_emit`'s Rust code, with the same semantics
//! (checked bit for bit against the translated formulas by the example
//! `clifcheck`; NaN results may differ in their payload, as the two
//! compilers order the operands of an operation differently): every 8-byte cell of the record, the variable buffer and the
//! stack the formula touches is a variable (`F64` when it is only used as a
//! whole double, else `I64` bits), and so are the registers, the flags, the
//! x87 registers and the FPU condition bits.  The variable buffer cells the
//! formula does not write are constants of the compiled code; the stack
//! starts with what the call pushes and 0 below (as for the direct
//! translated formulas).  The maths functions are Rust's (the helpers in
//! [`HELPERS`]), so the results are those of the translated code.
//!
//! The x87 compares tested through `fnstsw ax` (`shr ah, 1`, `and` /
//! `test` / `xor` / `cmp ah`, `sahf`) take their flags straight from the
//! compare's results (`St::ah`): the emulated bits would cost dozens of
//! integer ops per compare.
//!
//! Not compiled (the formula keeps the translated code): SSE, record
//! fields chosen by an index, computed addresses, the map functions and
//! integer division.

use super::lift::Loc;
use super::lift_emit::region;
use super::*;
use crate::custom::{IT_C1, STACK_TOP};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::{F32, F64, I16, I32, I64, I8};
use cranelift_codegen::ir::{Block, FuncRef, InstBuilder, MemFlags, Type, Value};
use cranelift_frontend::{FunctionBuilder, Variable};
use std::collections::{BTreeSet, HashMap};

const STACK_LOW: u32 = BASE + 0x4000;

/// The Rust functions the compiled code calls: (name, address, number of
/// f64 arguments; `round_int` takes the machine first).
pub const HELPERS: [(&str, usize); 8] = [("mb3d_sin", 1), ("mb3d_cos", 1), ("mb3d_tan", 1), ("mb3d_atan2", 2), ("mb3d_exp2", 1), ("mb3d_log2", 1), ("mb3d_fmod", 2), ("mb3d_round_int", 1)];

pub extern "C" fn mb3d_sin(x: f64) -> f64 {
    x.sin()
}
pub extern "C" fn mb3d_cos(x: f64) -> f64 {
    x.cos()
}
pub extern "C" fn mb3d_tan(x: f64) -> f64 {
    x.tan()
}
pub extern "C" fn mb3d_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}
pub extern "C" fn mb3d_exp2(x: f64) -> f64 {
    x.exp2()
}
pub extern "C" fn mb3d_log2(x: f64) -> f64 {
    x.log2()
}
pub extern "C" fn mb3d_fmod(x: f64, y: f64) -> f64 {
    x % y
}
/// `Machine::round_int` (the rounding of the machine's control word)
///
/// # Safety
/// `m` points to a machine (the compiled code passes the one it runs on).
pub unsafe extern "C" fn mb3d_round_int(m: *const Machine, x: f64) -> f64 {
    // SAFETY: see above
    unsafe { &*m }.round_int(x)
}

/// The addresses of [`HELPERS`], for the JIT's symbol table.
pub fn helper_addresses() -> [(&'static str, *const u8); 8] {
    [
        ("mb3d_sin", mb3d_sin as *const u8),
        ("mb3d_cos", mb3d_cos as *const u8),
        ("mb3d_tan", mb3d_tan as *const u8),
        ("mb3d_atan2", mb3d_atan2 as *const u8),
        ("mb3d_exp2", mb3d_exp2 as *const u8),
        ("mb3d_log2", mb3d_log2 as *const u8),
        ("mb3d_fmod", mb3d_fmod as *const u8),
        ("mb3d_round_int", mb3d_round_int as *const u8),
    ]
}

/// The helpers declared in the function being built (order of [`HELPERS`]).
pub struct Helpers(pub [FuncRef; 8]);

/// Where the record cells of a formula come from.
pub trait Record {
    /// cell `c` (the 8 bytes at `J4 + 8c`) at the start of the formula, as
    /// `ty` (`F64` or `I64` bits)
    fn get(&mut self, b: &mut FunctionBuilder, c: u32, ty: Type) -> Value;
    /// cell `c` as the formula leaves it (`v` of the type `get` gave)
    fn set(&mut self, b: &mut FunctionBuilder, c: u32, v: Value);
}

/// The values the formula code needs from the function around it.
pub struct Env<'a> {
    /// pointer to the machine's memory image (the byte at `BASE`)
    pub mem: Value,
    /// pointer to the machine (`round_int`)
    pub mach: Value,
    /// `PVar` of the formula's slot
    pub pvar: u32,
    /// the variable buffer as the machine holds it (the constants)
    pub var_bytes: &'a [u8],
    pub helpers: &'a Helpers,
    /// the step counter (I64) and its limit
    pub steps: Variable,
    pub max_steps: Value,
    /// the block that ends the function with an error
    pub err: Block,
}

/// A cell variable and its type.
#[derive(Clone, Copy)]
struct CellVar {
    var: Variable,
    ty: Type,
}

/// The state of one formula being compiled.
struct St<'e, 'a> {
    env: &'e Env<'a>,
    cells: HashMap<(char, u32), CellVar>,
    /// the registers; EAX in two parts: `rg[EAX]` its low 16 bits (AX, AL,
    /// AH: `fnstsw ax` and the tests of the condition bits), `eax_hi` the
    /// high 16 bits in place, so that writing AX does not depend on the
    /// value before (the chain of all condition bits would stay alive)
    rg: [Variable; 8],
    eax_hi: Variable,
    f: [Variable; 8],
    /// the flags cf, zf, sf, of, pf, af (I8: 0 or 1)
    fl: [Variable; 6],
    fsw: Variable,
    /// C0, C2, C3 of the last x87 compare in this block (I8 values), while
    /// they are the condition bits of `fsw`
    fsw_src: Option<[Value; 3]>,
    /// the cells known while compiling (the variable buffer cells the
    /// formula does not write): their bits
    consts: HashMap<(char, u32), u64>,
    /// the bits of AH while it holds the condition bits stored by `fnstsw
    /// ax` (or a mask of them): the flags of the ops that test them come
    /// from these values (the emulated register is computed as well; the
    /// compiler drops it when nothing reads it)
    ah: Option<[Bit; 8]>,
}

/// A bit of a value known while compiling: a constant or an I8 (0 or 1).
#[derive(Clone, Copy)]
enum Bit {
    C(bool),
    V(Value),
}

const CF: usize = 0;
const ZF: usize = 1;
const SF: usize = 2;
const OF: usize = 3;
const PF: usize = 4;
const AF: usize = 5;

impl Prog {
    /// Why the formula cannot be compiled with Cranelift; None if it can.
    pub fn clif_unsupported(&self) -> Option<String> {
        let an = self.analyse(false);
        if !an.report.problems.is_empty() {
            return Some(format!("not lifted: {:?}", an.report.problems));
        }
        if self.direct.is_none() {
            return Some("no direct translation".into());
        }
        for ((k, cs), _) in an.states.iter() {
            let op = &self.ops[*k];
            if let Some(mm) = super::lift_emit::op_mem(op) {
                match an.loc(*k, cs, &mm) {
                    Loc::Field(_) | Loc::Var(_) | Loc::Stack(_) => {}
                    l => return Some(format!("op {k}: memory {l:?}")),
                }
            }
            let ok = match op {
                Op::Gen(ins, _, _) => matches!(
                    ins.as_ref(),
                    Ins::Nop | Ins::Sahf | Ins::Test { .. } | Ins::Mov { .. } | Ins::Not(..) | Ins::Neg(..) | Ins::Inc(..) | Ins::Dec(..) | Ins::Mul(_, 4) | Ins::IMul1(_, 4)
                ) || matches!(ins.as_ref(), Ins::Alu { op, .. } if !matches!(op, Alu::Adc | Alu::Sbb))
                    || matches!(ins.as_ref(), Ins::Shift { op: Shift::Rcl | Shift::Rcr, dst: Rm::Reg(_), cnt: Some(1), size: 4 })
                    || matches!(ins.as_ref(), Ins::Shift { op: Shift::Shl | Shift::Shr | Shift::Sar, dst: Rm::Reg(_), cnt: Some(_), .. }),
                Op::CallHost(_) => false,
                Op::FLd { kind, .. } | Op::FArithM { kind, .. } | Op::FComM { kind, .. } => *kind != FKind::F80,
                Op::FSt { kind, .. } => matches!(kind, FKind::F32 | FKind::F64 | FKind::I16 | FKind::I32),
                Op::MovRM { .. } | Op::MovMR { .. } | Op::MovRR { .. } | Op::Lea { .. } | Op::PushR(_) | Op::PopR(_) | Op::FLd64 { .. } | Op::FSt64 { .. } | Op::FArithM64 { .. } => true,
                Op::Jcc { .. } | Op::JccRot { .. } | Op::Jmp(_) | Op::JmpRot { .. } | Op::Call { .. } | Op::CallRot { .. } | Op::Ret(_) => true,
                Op::FMov { .. } | Op::FSwap { .. } | Op::FArith { .. } | Op::FConst { .. } | Op::FChs(_) | Op::FAbs(_) | Op::FSqrt(_) | Op::FSin(_) | Op::FCos(_) | Op::FRndint(_) => true,
                Op::FPrem { .. } | Op::F2xm1(_) | Op::FPtan { .. } | Op::FSincos { .. } | Op::FPatan { .. } | Op::FYl2x { .. } | Op::FScale { .. } => true,
                Op::FCom { .. } | Op::FTst(_) | Op::Fnstsw { .. } => true,
                _ => false,
            };
            if !ok {
                return Some(format!("op {k}: {op:?}"));
            }
        }
        // the Rust translation must exist (the cells and their typing)
        self.emit_lifted_cells("f", false).map(|_| ()).is_none().then(|| "no lifted Rust code".to_string())
    }

    /// The record cells the formula writes.
    pub fn clif_record_written(&self) -> Option<Vec<u32>> {
        let (_, cells) = self.emit_lifted_cells("f", false)?;
        Some(cells.written_cells().filter(|c| c.0 == 'r').map(|c| c.1).collect())
    }

    /// Emits the formula into `b` (positioned in an open block): the code of
    /// one call, which continues in the block it returns (open).  None if
    /// the formula cannot be compiled (see [`Prog::clif_unsupported`]); the
    /// function being built is then unusable.
    pub fn clif_emit(&self, b: &mut FunctionBuilder, env: &Env, rec: &mut dyn Record) -> Option<Block> {
        let an = self.analyse(false);
        let (_, cells) = self.emit_lifted_cells("f", false)?;
        // typing: a cell only read and written as a whole double is an F64
        let mut whole: HashMap<(char, u32), bool> = HashMap::new();
        for ((cell, sh, n, kind), _) in cells.log.iter().zip(&cells.log_ops) {
            let w = whole.entry(*cell).or_insert(true);
            *w &= *sh == 0 && *n == 8 && *kind == 'd';
        }
        let written: BTreeSet<(char, u32)> = cells.written_cells().collect();
        let used: Vec<(char, u32)> = cells.used_cells().collect();
        let mut st = St {
            env,
            cells: HashMap::new(),
            rg: std::array::from_fn(|_| b.declare_var(I32)),
            eax_hi: b.declare_var(I32),
            f: std::array::from_fn(|_| b.declare_var(F64)),
            fl: std::array::from_fn(|_| b.declare_var(I8)),
            fsw: b.declare_var(I32),
            fsw_src: None,
            ah: None,
            consts: HashMap::new(),
        };
        // the start: registers of a direct call, cells from their sources
        let x = IT_C1 - 32;
        let regs0 = [x, x + 16, x + 8, 0, STACK_TOP - 12, 0, 0, 0];
        for (r, v) in regs0.iter().enumerate() {
            let c = b.ins().iconst(I32, *v as i64);
            st.set_rg(b, r, c);
        }
        let z = b.ins().f64const(0.0);
        for r in st.f {
            b.def_var(r, z);
        }
        let z8 = b.ins().iconst(I8, 0);
        for r in st.fl {
            b.def_var(r, z8);
        }
        let z32 = b.ins().iconst(I32, 0);
        b.def_var(st.fsw, z32);
        let vbase = env.pvar.wrapping_sub(crate::m3f::CONST_OFFSET as u32);
        for cell in &used {
            let ty = if whole.get(cell).copied().unwrap_or(false) { F64 } else { I64 };
            let var = b.declare_var(ty);
            let (p, c) = *cell;
            let v = match p {
                'r' => rec.get(b, c, ty),
                'v' => {
                    let a = vbase.wrapping_add(8 * c);
                    if written.contains(cell) {
                        b.ins().load(ty, MemFlags::trusted(), env.mem, mem_off(a))
                    } else {
                        let i = mem_off(a) as usize;
                        let bits = u64::from_le_bytes(env.var_bytes.get(i..i + 8)?.try_into().ok()?);
                        st.consts.insert(*cell, bits);
                        if ty == F64 {
                            b.ins().f64const(f64::from_bits(bits))
                        } else {
                            b.ins().iconst(I64, bits as i64)
                        }
                    }
                }
                _ => {
                    let bits = stack_init(c);
                    if ty == F64 {
                        b.ins().f64const(f64::from_bits(bits))
                    } else {
                        b.ins().iconst(I64, bits as i64)
                    }
                }
            };
            b.def_var(var, v);
            st.cells.insert(*cell, CellVar { var, ty });
        }
        // the blocks, as in `emit_lifted_mode`
        let n = self.ops.len();
        let mut start = vec![false; n + 1];
        start[0] = true;
        for (k, op) in self.ops.iter().enumerate() {
            match op {
                Op::Jcc { target, .. } | Op::Jmp(target) | Op::Call { target, .. } | Op::JccRot { target, .. } | Op::JmpRot { target, .. } | Op::CallRot { target, .. } => {
                    start[*target as usize] = true;
                    start[k + 1] = true;
                }
                Op::Ret(_) => start[k + 1] = true,
                _ => {}
            }
        }
        let mut keys: Vec<(usize, Vec<usize>)> = an.states.keys().filter(|(k, _)| start[*k]).cloned().collect();
        keys.sort();
        let blocks: HashMap<(usize, Vec<usize>), (usize, Block)> = keys.iter().enumerate().map(|(i, k)| (k.clone(), (i, b.create_block()))).collect();
        let ret = b.create_block();
        let entry = blocks.get(&(0, Vec::new()))?.1;
        b.ins().jump(entry, &[]);
        for key in &keys {
            let (bi, bk) = blocks[key];
            let cs = &key.1;
            b.switch_to_block(bk);
            st.fsw_src = None;
            st.ah = None;
            let mut k = key.0;
            loop {
                let ended = st.op(self, b, &an, k, cs, bi, &blocks, ret)?;
                if ended {
                    break;
                }
                k += 1;
                if k >= n {
                    b.ins().jump(env.err, &[]);
                    break;
                }
                if start[k] {
                    let (_, t) = *blocks.get(&(k, cs.clone()))?;
                    b.ins().jump(t, &[]);
                    break;
                }
            }
        }
        // the return: the written cells go back
        b.switch_to_block(ret);
        for cell in &written {
            let cv = *st.cells.get(cell)?;
            let v = b.use_var(cv.var);
            match cell.0 {
                'r' => rec.set(b, cell.1, v),
                'v' => {
                    b.ins().store(MemFlags::trusted(), v, env.mem, mem_off(vbase.wrapping_add(8 * cell.1)));
                }
                _ => {}
            }
        }
        Some(ret)
    }
}

/// Offset of address `a` in the memory image (as `Machine::ui`).
fn mem_off(a: u32) -> i32 {
    ((a.wrapping_sub(BASE) as usize) & (MEM_SIZE - 1)) as i32
}

/// The start value of stack cell `c` of a direct call (what the call
/// pushes, 0 elsewhere).
fn stack_init(c: u32) -> u64 {
    let x = IT_C1 - 32;
    let call = [(STACK_TOP - 12, RETURN_SENTINEL), (STACK_TOP - 8, IT_C1), (STACK_TOP - 4, x + 24)];
    let half = |a: u32| call.iter().find(|(ca, _)| *ca == a).map_or(0, |(_, v)| *v) as u64;
    let a = STACK_LOW + 8 * c;
    half(a) | half(a + 4) << 32
}

fn mask32(size: u8) -> u32 {
    if size == 4 {
        u32::MAX
    } else {
        (1u32 << (size as u32 * 8)) - 1
    }
}

impl St<'_, '_> {
    // ---- cells ----

    /// The value of an x87 load of `kind` from constant cells (the same
    /// conversions as `fload`, done while compiling).
    fn const_load(&self, (p, off): (char, u32), kind: FKind) -> Option<f64> {
        let n = match kind {
            FKind::F64 | FKind::I64 => 8,
            FKind::F32 | FKind::I32 => 4,
            FKind::I16 => 2,
            FKind::F80 => return None,
        };
        let (c, sh) = (off / 8, off % 8);
        let lo = *self.consts.get(&(p, c))?;
        let v = if sh + n <= 8 {
            if sh == 0 { lo } else { lo >> (8 * sh) }
        } else {
            let hi = *self.consts.get(&(p, c + 1))?;
            (lo >> (8 * sh)) | (hi << (64 - 8 * sh))
        };
        let v = if n >= 8 { v } else { v & ((1u64 << (8 * n)) - 1) };
        Some(match kind {
            FKind::F64 => f64::from_bits(v),
            FKind::F32 => f32::from_bits(v as u32) as f64,
            FKind::I16 => v as u16 as i16 as f64,
            FKind::I32 => v as u32 as i32 as f64,
            FKind::I64 => v as i64 as f64,
            FKind::F80 => return None,
        })
    }

    /// `n` bytes at byte `off` of region `p` as I64 (zero extended)
    fn rd(&mut self, b: &mut FunctionBuilder, (p, off): (char, u32), n: u32) -> Option<Value> {
        let (c, sh) = (off / 8, off % 8);
        let lo = self.bits(b, (p, c))?;
        let v = if sh + n <= 8 {
            if sh == 0 && n == 8 {
                return Some(lo);
            }
            b.ins().ushr_imm(lo, 8 * sh as i64)
        } else {
            let hi = self.bits(b, (p, c + 1))?;
            let a = b.ins().ushr_imm(lo, 8 * sh as i64);
            let h = b.ins().ishl_imm(hi, 64 - 8 * sh as i64);
            b.ins().bor(a, h)
        };
        Some(if n >= 8 { v } else { b.ins().band_imm(v, ((1u64 << (8 * n)) - 1) as i64) })
    }

    /// the cell as I64 bits
    fn bits(&mut self, b: &mut FunctionBuilder, cell: (char, u32)) -> Option<Value> {
        let cv = *self.cells.get(&cell)?;
        let v = b.use_var(cv.var);
        Some(if cv.ty == F64 { b.ins().bitcast(I64, MemFlags::new(), v) } else { v })
    }

    /// a whole double at `off` (8 bytes)
    fn rd_f64(&mut self, b: &mut FunctionBuilder, r: (char, u32)) -> Option<Value> {
        if let Some(x) = self.const_load(r, FKind::F64) {
            return Some(b.ins().f64const(x));
        }
        if r.1.is_multiple_of(8) {
            let cv = *self.cells.get(&(r.0, r.1 / 8))?;
            if cv.ty == F64 {
                return Some(b.use_var(cv.var));
            }
        }
        let v = self.rd(b, r, 8)?;
        Some(b.ins().bitcast(F64, MemFlags::new(), v))
    }

    /// writes the low `n` bytes of the I64 `v` at byte `off` of region `p`
    fn wr(&mut self, b: &mut FunctionBuilder, (p, off): (char, u32), n: u32, v: Value) -> Option<()> {
        let (c, sh) = (off / 8, off % 8);
        if sh + n <= 8 {
            if sh == 0 && n == 8 {
                return self.set_bits(b, (p, c), v);
            }
            let mk = (1u64 << (8 * n)) - 1;
            let old = self.bits(b, (p, c))?;
            let keep = b.ins().band_imm(old, !(mk << (8 * sh)) as i64);
            let nv = b.ins().band_imm(v, mk as i64);
            let nv = b.ins().ishl_imm(nv, 8 * sh as i64);
            let r = b.ins().bor(keep, nv);
            self.set_bits(b, (p, c), r)
        } else {
            let n0 = 8 - sh;
            let n1 = n - n0;
            let m0 = (1u64 << (8 * n0)) - 1;
            let m1 = (1u64 << (8 * n1)) - 1;
            let old = self.bits(b, (p, c))?;
            let keep = b.ins().band_imm(old, !(m0 << (8 * sh)) as i64);
            let a = b.ins().band_imm(v, m0 as i64);
            let a = b.ins().ishl_imm(a, 8 * sh as i64);
            let r0 = b.ins().bor(keep, a);
            self.set_bits(b, (p, c), r0)?;
            let old1 = self.bits(b, (p, c + 1))?;
            let keep1 = b.ins().band_imm(old1, !m1 as i64);
            let h = b.ins().ushr_imm(v, 8 * n0 as i64);
            let h = b.ins().band_imm(h, m1 as i64);
            let r1 = b.ins().bor(keep1, h);
            self.set_bits(b, (p, c + 1), r1)
        }
    }

    fn set_bits(&mut self, b: &mut FunctionBuilder, cell: (char, u32), v: Value) -> Option<()> {
        let cv = *self.cells.get(&cell)?;
        let v = if cv.ty == F64 { b.ins().bitcast(F64, MemFlags::new(), v) } else { v };
        b.def_var(cv.var, v);
        Some(())
    }

    fn wr_f64(&mut self, b: &mut FunctionBuilder, r: (char, u32), v: Value) -> Option<()> {
        if r.1.is_multiple_of(8) {
            let cv = *self.cells.get(&(r.0, r.1 / 8))?;
            if cv.ty == F64 {
                b.def_var(cv.var, v);
                return Some(());
            }
        }
        let bits = b.ins().bitcast(I64, MemFlags::new(), v);
        self.wr(b, r, 8, bits)
    }

    /// x87 memory load of `kind` as an f64
    fn fload(&mut self, b: &mut FunctionBuilder, r: (char, u32), kind: FKind) -> Option<Value> {
        if let Some(x) = self.const_load(r, kind) {
            return Some(b.ins().f64const(x));
        }
        Some(match kind {
            FKind::F64 => self.rd_f64(b, r)?,
            FKind::F32 => {
                let v = self.rd(b, r, 4)?;
                let v = b.ins().ireduce(I32, v);
                let f = b.ins().bitcast(F32, MemFlags::new(), v);
                b.ins().fpromote(F64, f)
            }
            FKind::I16 => {
                let v = self.rd(b, r, 2)?;
                let v = b.ins().ireduce(I16, v);
                let v = b.ins().sextend(I64, v);
                b.ins().fcvt_from_sint(F64, v)
            }
            FKind::I32 => {
                let v = self.rd(b, r, 4)?;
                let v = b.ins().ireduce(I32, v);
                let v = b.ins().sextend(I64, v);
                b.ins().fcvt_from_sint(F64, v)
            }
            FKind::I64 => {
                let v = self.rd(b, r, 8)?;
                b.ins().fcvt_from_sint(F64, v)
            }
            FKind::F80 => return None,
        })
    }

    /// x87 store of `v` as `kind`
    fn fstore(&mut self, b: &mut FunctionBuilder, r: (char, u32), kind: FKind, v: Value) -> Option<()> {
        match kind {
            FKind::F64 => self.wr_f64(b, r, v),
            FKind::F32 => {
                let s = b.ins().fdemote(F32, v);
                let bits = b.ins().bitcast(I32, MemFlags::new(), s);
                let bits = b.ins().uextend(I64, bits);
                self.wr(b, r, 4, bits)
            }
            FKind::I16 | FKind::I32 => {
                // round as the machine does; NaN and out of range: the
                // "integer indefinite"
                let x = self.round_int(b, v);
                let (lo, hi, ind, n) = if kind == FKind::I16 { (-32768.0, 32767.0, 0x8000i64, 2) } else { (-2147483648.0, 2147483647.0, 0x8000_0000i64, 4) };
                let flo = b.ins().f64const(lo);
                let fhi = b.ins().f64const(hi);
                // !(lo..=hi).contains(x) is true for NaN as well
                let c1 = b.ins().fcmp(FloatCC::GreaterThanOrEqual, x, flo);
                let c2 = b.ins().fcmp(FloatCC::LessThanOrEqual, x, fhi);
                let inr = b.ins().band(c1, c2);
                let i = b.ins().fcvt_to_sint_sat(I32, x);
                let i = b.ins().uextend(I64, i);
                let i = b.ins().band_imm(i, if n == 2 { 0xFFFF } else { 0xFFFF_FFFF });
                let indv = b.ins().iconst(I64, ind);
                let r2 = b.ins().select(inr, i, indv);
                self.wr(b, r, n, r2)
            }
            _ => None,
        }
    }

    // ---- helpers ----

    fn call1(&self, b: &mut FunctionBuilder, h: usize, x: Value) -> Value {
        let c = b.ins().call(self.env.helpers.0[h], &[x]);
        b.inst_results(c)[0]
    }

    fn call2(&self, b: &mut FunctionBuilder, h: usize, x: Value, y: Value) -> Value {
        let c = b.ins().call(self.env.helpers.0[h], &[x, y]);
        b.inst_results(c)[0]
    }

    /// `Machine::round_int`: the rounding of the machine's control word
    /// (nearest even, down, up, toward zero)
    fn round_int(&self, b: &mut FunctionBuilder, x: Value) -> Value {
        let cw = b.ins().load(I16, MemFlags::trusted(), self.env.mach, std::mem::offset_of!(Machine, fcw) as i32);
        let cw = b.ins().uextend(I32, cw);
        let mode = b.ins().ushr_imm(cw, 10);
        let mode = b.ins().band_imm(mode, 3);
        let n = b.ins().nearest(x);
        let f = b.ins().floor(x);
        let c = b.ins().ceil(x);
        let t = b.ins().trunc(x);
        let is1 = b.ins().icmp_imm(IntCC::Equal, mode, 1);
        let is2 = b.ins().icmp_imm(IntCC::Equal, mode, 2);
        let is0 = b.ins().icmp_imm(IntCC::Equal, mode, 0);
        let r = b.ins().select(is2, c, t);
        let r = b.ins().select(is1, f, r);
        b.ins().select(is0, n, r)
    }

    fn fv(&self, b: &mut FunctionBuilder, i: u8) -> Value {
        b.use_var(self.f[i as usize & 7])
    }

    fn set_f(&self, b: &mut FunctionBuilder, i: u8, v: Value) {
        b.def_var(self.f[i as usize & 7], v);
    }

    fn fop(&self, b: &mut FunctionBuilder, op: &FOp, a: Value, x: Value) -> Value {
        match op {
            FOp::Add => b.ins().fadd(a, x),
            FOp::Mul => b.ins().fmul(a, x),
            FOp::Sub => b.ins().fsub(a, x),
            FOp::SubR => b.ins().fsub(x, a),
            FOp::Div => b.ins().fdiv(a, x),
            FOp::DivR => b.ins().fdiv(x, a),
        }
    }

    /// `f.rotate_right(n)`
    fn rot(&self, b: &mut FunctionBuilder, n: u8) {
        let old: Vec<Value> = (0..8).map(|k| b.use_var(self.f[k])).collect();
        for k in 0..8 {
            b.def_var(self.f[k], old[(k + 8 - n as usize % 8) % 8]);
        }
    }

    /// `fcompare_cc(a, b)` into fsw
    fn fcompare(&mut self, b: &mut FunctionBuilder, x: Value, y: Value) {
        let lt = b.ins().fcmp(FloatCC::LessThan, x, y);
        let eq = b.ins().fcmp(FloatCC::Equal, x, y);
        let un = b.ins().fcmp(FloatCC::Unordered, x, y);
        let lt = b.ins().bor(lt, un);
        let eq = b.ins().bor(eq, un);
        self.fsw_src = Some([lt, un, eq]);
        let lt = b.ins().uextend(I32, lt);
        let eq = b.ins().uextend(I32, eq);
        let un = b.ins().uextend(I32, un);
        let lt = b.ins().ishl_imm(lt, 8);
        let eq = b.ins().ishl_imm(eq, 14);
        let un = b.ins().ishl_imm(un, 10);
        let r = b.ins().bor(lt, eq);
        let r = b.ins().bor(r, un);
        b.def_var(self.fsw, r);
    }

    fn fsw_and(&mut self, b: &mut FunctionBuilder, m: i64) {
        self.fsw_src = None;
        let v = b.use_var(self.fsw);
        let v = b.ins().band_imm(v, m);
        b.def_var(self.fsw, v);
    }

    // ---- integer registers and flags ----

    /// register `r` (32 bits)
    fn get_rg(&self, b: &mut FunctionBuilder, r: usize) -> Value {
        if r == EAX {
            let lo = b.use_var(self.rg[EAX]);
            let hi = b.use_var(self.eax_hi);
            b.ins().bor(hi, lo)
        } else {
            b.use_var(self.rg[r])
        }
    }

    fn set_rg(&self, b: &mut FunctionBuilder, r: usize, v: Value) {
        if r == EAX {
            let lo = b.ins().band_imm(v, 0xFFFF);
            let hi = b.ins().band_imm(v, 0xFFFF_0000);
            b.def_var(self.rg[EAX], lo);
            b.def_var(self.eax_hi, hi);
        } else {
            b.def_var(self.rg[r], v);
        }
    }

    fn reg(&self, b: &mut FunctionBuilder, r: u8, size: u8) -> Value {
        match size {
            1 => {
                if r < 4 {
                    let v = b.use_var(self.rg[r as usize]);
                    b.ins().band_imm(v, 0xFF)
                } else {
                    let v = b.use_var(self.rg[(r - 4) as usize]);
                    let v = b.ins().ushr_imm(v, 8);
                    b.ins().band_imm(v, 0xFF)
                }
            }
            2 => {
                let v = b.use_var(self.rg[r as usize]);
                b.ins().band_imm(v, 0xFFFF)
            }
            _ => self.get_rg(b, r as usize),
        }
    }

    fn set_reg(&self, b: &mut FunctionBuilder, r: u8, size: u8, v: Value) {
        match size {
            1 => {
                let (i, sh) = if r < 4 { (r as usize, 0) } else { ((r - 4) as usize, 8) };
                let old = b.use_var(self.rg[i]);
                let keep = b.ins().band_imm(old, !(0xFFi64 << sh) & 0xFFFF_FFFF);
                let nv = b.ins().band_imm(v, 0xFF);
                let nv = b.ins().ishl_imm(nv, sh);
                let r = b.ins().bor(keep, nv);
                b.def_var(self.rg[i], r);
            }
            2 => {
                let old = b.use_var(self.rg[r as usize]);
                let keep = b.ins().band_imm(old, 0xFFFF_0000);
                let nv = b.ins().band_imm(v, 0xFFFF);
                let r2 = b.ins().bor(keep, nv);
                b.def_var(self.rg[r as usize], r2);
            }
            _ => self.set_rg(b, r as usize, v),
        }
    }

    fn flag(&self, b: &mut FunctionBuilder, f: usize) -> Value {
        b.use_var(self.fl[f])
    }

    fn set_flag(&self, b: &mut FunctionBuilder, f: usize, v: Value) {
        b.def_var(self.fl[f], v);
    }

    fn set_flag_c(&self, b: &mut FunctionBuilder, f: usize, v: bool) {
        let c = b.ins().iconst(I8, v as i64);
        b.def_var(self.fl[f], c);
    }

    /// `Flags::set_szp`
    fn set_szp(&self, b: &mut FunctionBuilder, r: Value, size: u8) {
        let bits = size as i64 * 8;
        let m = b.ins().band_imm(r, mask32(size) as i64);
        let z = b.ins().icmp_imm(IntCC::Equal, m, 0);
        self.set_flag(b, ZF, z);
        let s = b.ins().ushr_imm(r, bits - 1);
        let s = b.ins().band_imm(s, 1);
        let s = b.ins().ireduce(I8, s);
        self.set_flag(b, SF, s);
        let lo = b.ins().band_imm(r, 0xFF);
        let pc = b.ins().popcnt(lo);
        let p = b.ins().band_imm(pc, 1);
        let p = b.ins().icmp_imm(IntCC::Equal, p, 0);
        self.set_flag(b, PF, p);
    }

    /// `Flags::alu` (a, b: I32) -> result (I32)
    fn alu(&self, b: &mut FunctionBuilder, op: Alu, a: Value, x: Value, size: u8) -> Value {
        let m = mask32(size) as i64;
        let sign = 1i64 << (size as i64 * 8 - 1);
        let a = b.ins().band_imm(a, m);
        let x = b.ins().band_imm(x, m);
        let r = match op {
            Alu::Add | Alu::Adc => {
                let a64 = b.ins().uextend(I64, a);
                let x64 = b.ins().uextend(I64, x);
                let full = b.ins().iadd(a64, x64);
                let r = b.ins().ireduce(I32, full);
                let r = b.ins().band_imm(r, m);
                let cf = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, full, m & 0xFFFF_FFFF);
                self.set_flag(b, CF, cf);
                let ar = b.ins().bxor(a, r);
                let xr = b.ins().bxor(x, r);
                let o = b.ins().band(ar, xr);
                let o = b.ins().band_imm(o, sign);
                let o = b.ins().icmp_imm(IntCC::NotEqual, o, 0);
                self.set_flag(b, OF, o);
                let af = b.ins().bxor(a, x);
                let af = b.ins().bxor(af, r);
                let af = b.ins().band_imm(af, 0x10);
                let af = b.ins().icmp_imm(IntCC::NotEqual, af, 0);
                self.set_flag(b, AF, af);
                r
            }
            Alu::Sub | Alu::Sbb | Alu::Cmp => {
                let r = b.ins().isub(a, x);
                let r = b.ins().band_imm(r, m);
                let cf = b.ins().icmp(IntCC::UnsignedGreaterThan, x, a);
                self.set_flag(b, CF, cf);
                let ax = b.ins().bxor(a, x);
                let ar = b.ins().bxor(a, r);
                let o = b.ins().band(ax, ar);
                let o = b.ins().band_imm(o, sign);
                let o = b.ins().icmp_imm(IntCC::NotEqual, o, 0);
                self.set_flag(b, OF, o);
                let af = b.ins().bxor(ax, r);
                let af = b.ins().band_imm(af, 0x10);
                let af = b.ins().icmp_imm(IntCC::NotEqual, af, 0);
                self.set_flag(b, AF, af);
                r
            }
            Alu::And | Alu::Or | Alu::Xor => {
                let r = match op {
                    Alu::And => b.ins().band(a, x),
                    Alu::Or => b.ins().bor(a, x),
                    _ => b.ins().bxor(a, x),
                };
                self.set_flag_c(b, CF, false);
                self.set_flag_c(b, OF, false);
                r
            }
        };
        self.set_szp(b, r, size);
        r
    }

    /// `Flags::cond` (I8)
    fn cond(&self, b: &mut FunctionBuilder, cc: u8) -> Value {
        let r = match cc >> 1 {
            0 => self.flag(b, OF),
            1 => self.flag(b, CF),
            2 => self.flag(b, ZF),
            3 => {
                let c = self.flag(b, CF);
                let z = self.flag(b, ZF);
                b.ins().bor(c, z)
            }
            4 => self.flag(b, SF),
            5 => self.flag(b, PF),
            6 => {
                let s = self.flag(b, SF);
                let o = self.flag(b, OF);
                b.ins().bxor(s, o)
            }
            _ => {
                let s = self.flag(b, SF);
                let o = self.flag(b, OF);
                let so = b.ins().bxor(s, o);
                let z = self.flag(b, ZF);
                b.ins().bor(z, so)
            }
        };
        if cc & 1 == 1 {
            b.ins().bxor_imm(r, 1)
        } else {
            r
        }
    }

    /// effective address of `m` (I32)
    fn ea(&self, b: &mut FunctionBuilder, m: &Mem) -> Value {
        let mut e = b.ins().iconst(I32, m.disp as u32 as i64);
        if let Some(r) = m.base {
            let v = self.get_rg(b, r as usize);
            e = b.ins().iadd(v, e);
        }
        if let Some((i, sc)) = m.index {
            let v = self.get_rg(b, i as usize);
            let v = b.ins().imul_imm(v, sc as i64);
            e = b.ins().iadd(e, v);
        }
        e
    }

    /// I32 value of a register or memory operand of `size` bytes
    fn rm_rd(&mut self, b: &mut FunctionBuilder, rm: &Rm, size: u8, loc: &dyn Fn(&Mem) -> Option<(char, u32)>) -> Option<Value> {
        Some(match rm {
            Rm::Reg(r) => self.reg(b, *r, size),
            Rm::Mem(mm) => {
                let v = self.rd(b, loc(mm)?, size as u32)?;
                b.ins().ireduce(I32, v)
            }
        })
    }

    fn rm_wr(&mut self, b: &mut FunctionBuilder, rm: &Rm, size: u8, v: Value, loc: &dyn Fn(&Mem) -> Option<(char, u32)>) -> Option<()> {
        match rm {
            Rm::Reg(r) => {
                self.set_reg(b, *r, size, v);
                Some(())
            }
            Rm::Mem(mm) => {
                let v64 = b.ins().uextend(I64, v);
                self.wr(b, loc(mm)?, size as u32, v64)
            }
        }
    }

    fn src(&mut self, b: &mut FunctionBuilder, s: &Src, size: u8, loc: &dyn Fn(&Mem) -> Option<(char, u32)>) -> Option<Value> {
        Some(match s {
            Src::Imm(v) => b.ins().iconst(I32, super::mask(*v, size) as i64),
            Src::Rm(rm) => self.rm_rd(b, rm, size, loc)?,
        })
    }

    /// a counted jump: the step limit, then `t`
    fn counted_jump(&self, b: &mut FunctionBuilder, t: Block) {
        let s = b.use_var(self.env.steps);
        let s = b.ins().iadd_imm(s, 1);
        b.def_var(self.env.steps, s);
        let over = b.ins().icmp(IntCC::UnsignedGreaterThan, s, self.env.max_steps);
        b.ins().brif(over, self.env.err, &[], t, &[]);
    }

    // ---- one op ----

    /// Emits op `k` (call string `cs`) in the current block; true if it
    /// ended the block (a jump or a return).
    #[allow(clippy::too_many_arguments)]
    fn op(
        &mut self,
        p: &Prog,
        b: &mut FunctionBuilder,
        an: &super::lift::Analysis,
        k: usize,
        cs: &Vec<usize>,
        bi: usize,
        blocks: &HashMap<(usize, Vec<usize>), (usize, Block)>,
        ret: Block,
    ) -> Option<bool> {
        let loc = |mm: &Mem| super::lift_emit::region_at(an.loc(k, cs, mm));
        let blk = |t: usize, c: &Vec<usize>| blocks.get(&(t, c.clone())).copied();
        // a counted jump goes through the step limit when it goes back
        let jump_to = |st: &Self, b: &mut FunctionBuilder, t: (usize, Block)| {
            if t.0 <= bi {
                st.counted_jump(b, t.1);
            } else {
                b.ins().jump(t.1, &[]);
            }
        };
        let keeps_ah = match &p.ops[k] {
            Op::Gen(ins, _, _) => Self::ah_idiom(ins),
            Op::MovRM { r, .. } | Op::Lea { r, .. } => *r as usize != EAX,
            Op::MovRR { d, .. } => *d as usize != EAX,
            Op::PopR(r) => *r as usize != EAX,
            _ => true,
        };
        if !keeps_ah {
            self.ah = None;
        }
        match &p.ops[k] {
            Op::MovRM { r, mem } => {
                let v = self.rd(b, loc(mem)?, 4)?;
                let v = b.ins().ireduce(I32, v);
                self.set_rg(b, *r as usize, v);
            }
            Op::MovMR { mem, r } => {
                let v = self.get_rg(b, *r as usize);
                let v = b.ins().uextend(I64, v);
                self.wr(b, loc(mem)?, 4, v)?;
            }
            Op::MovRR { d, s } => {
                let v = self.get_rg(b, *s as usize);
                self.set_rg(b, *d as usize, v);
            }
            Op::Lea { r, mem } => {
                let e = self.ea(b, mem);
                self.set_rg(b, *r as usize, e);
            }
            Op::PushR(r) => {
                let v = self.get_rg(b, *r as usize);
                let esp = b.use_var(self.rg[ESP]);
                let esp = b.ins().iadd_imm(esp, -4);
                b.def_var(self.rg[ESP], esp);
                let v64 = b.ins().uextend(I64, v);
                self.wr(b, region(an.stack_loc(k, cs, true))?, 4, v64)?;
            }
            Op::PopR(r) if *r as usize == ESP => {
                let v = self.rd(b, region(an.stack_loc(k, cs, false))?, 4)?;
                let v = b.ins().ireduce(I32, v);
                self.set_rg(b, ESP, v);
            }
            Op::PopR(r) => {
                let v = self.rd(b, region(an.stack_loc(k, cs, false))?, 4)?;
                let v = b.ins().ireduce(I32, v);
                self.set_rg(b, *r as usize, v);
                let esp = b.use_var(self.rg[ESP]);
                let esp = b.ins().iadd_imm(esp, 4);
                b.def_var(self.rg[ESP], esp);
            }
            Op::FLd64 { dst, mem } => {
                let v = self.rd_f64(b, loc(mem)?)?;
                self.set_f(b, *dst, v);
            }
            Op::FSt64 { src, mem } => {
                let v = self.fv(b, *src);
                self.wr_f64(b, loc(mem)?, v)?;
            }
            Op::FArithM64 { op, dst, mem } => {
                let x = self.rd_f64(b, loc(mem)?)?;
                let a = self.fv(b, *dst);
                let r = self.fop(b, op, a, x);
                self.set_f(b, *dst, r);
            }
            Op::FLd { dst, kind, mem } => {
                let v = self.fload(b, loc(mem)?, *kind)?;
                self.set_f(b, *dst, v);
            }
            Op::FSt { src, kind, mem } => {
                let v = self.fv(b, *src);
                self.fstore(b, loc(mem)?, *kind, v)?;
            }
            Op::FArithM { op, dst, kind, mem } => {
                let x = self.fload(b, loc(mem)?, *kind)?;
                let a = self.fv(b, *dst);
                let r = self.fop(b, op, a, x);
                self.set_f(b, *dst, r);
            }
            Op::FMov { dst, src } => {
                let v = self.fv(b, *src);
                self.set_f(b, *dst, v);
            }
            Op::FSwap { a, b: c } => {
                let x = self.fv(b, *a);
                let y = self.fv(b, *c);
                self.set_f(b, *a, y);
                self.set_f(b, *c, x);
            }
            Op::FArith { op, dst, a, b: c } => {
                let x = self.fv(b, *a);
                let y = self.fv(b, *c);
                let r = self.fop(b, op, x, y);
                self.set_f(b, *dst, r);
            }
            Op::FConst { dst, v } => {
                let c = b.ins().f64const(*v);
                self.set_f(b, *dst, c);
            }
            Op::FChs(i) => {
                let v = self.fv(b, *i);
                let r = b.ins().fneg(v);
                self.set_f(b, *i, r);
            }
            Op::FAbs(i) => {
                let v = self.fv(b, *i);
                let r = b.ins().fabs(v);
                self.set_f(b, *i, r);
            }
            Op::FSqrt(i) => {
                let v = self.fv(b, *i);
                let r = b.ins().sqrt(v);
                self.set_f(b, *i, r);
            }
            Op::FSin(i) | Op::FCos(i) => {
                let v = self.fv(b, *i);
                let r = self.call1(b, if matches!(p.ops[k], Op::FSin(_)) { 0 } else { 1 }, v);
                self.set_f(b, *i, r);
                self.fsw_and(b, !0x0400);
            }
            Op::FRndint(i) => {
                let v = self.fv(b, *i);
                let r = self.round_int(b, v);
                self.set_f(b, *i, r);
            }
            Op::FPrem { a, b: c, ieee } => {
                let x = self.fv(b, *a);
                let y = self.fv(b, *c);
                let d = b.ins().fdiv(x, y);
                let q = if *ieee { b.ins().nearest(d) } else { b.ins().trunc(d) };
                let r = if *ieee {
                    let qy = b.ins().fmul(q, y);
                    b.ins().fsub(x, qy)
                } else {
                    self.call2(b, 6, x, y)
                };
                self.set_f(b, *a, r);
                // the quotient bits: C0 (bit 2), C3 (bit 1), C1 (bit 0)
                let qa = b.ins().fabs(q);
                let qi = b.ins().fcvt_to_uint_sat(I64, qa);
                let bit = |b: &mut FunctionBuilder, s: i64, to: i64| {
                    let t = b.ins().ushr_imm(qi, s);
                    let t = b.ins().band_imm(t, 1);
                    let t = b.ins().ireduce(I32, t);
                    b.ins().ishl_imm(t, to)
                };
                let c0 = bit(b, 2, 8);
                let c3 = bit(b, 1, 14);
                let c1 = bit(b, 0, 9);
                let r = b.ins().bor(c0, c3);
                let r = b.ins().bor(r, c1);
                b.def_var(self.fsw, r);
                self.fsw_src = None;
            }
            Op::F2xm1(i) => {
                let v = self.fv(b, *i);
                let e = self.call1(b, 4, v);
                let one = b.ins().f64const(1.0);
                let r = b.ins().fsub(e, one);
                self.set_f(b, *i, r);
            }
            Op::FPtan { src, dst } => {
                let v = self.fv(b, *src);
                let r = self.call1(b, 2, v);
                self.set_f(b, *src, r);
                let one = b.ins().f64const(1.0);
                self.set_f(b, *dst, one);
                self.fsw_and(b, !0x0400);
            }
            Op::FSincos { src, dst } => {
                let v = self.fv(b, *src);
                let s = self.call1(b, 0, v);
                let c = self.call1(b, 1, v);
                self.set_f(b, *src, s);
                self.set_f(b, *dst, c);
                self.fsw_and(b, !0x0400);
            }
            Op::FPatan { x, y } => {
                let yv = self.fv(b, *y);
                let xv = self.fv(b, *x);
                let r = self.call2(b, 3, yv, xv);
                self.set_f(b, *y, r);
            }
            Op::FYl2x { x, y } => {
                let xv = self.fv(b, *x);
                let l = self.call1(b, 5, xv);
                let yv = self.fv(b, *y);
                let r = b.ins().fmul(yv, l);
                self.set_f(b, *y, r);
            }
            Op::FScale { a, s } => {
                let sv = self.fv(b, *s);
                let t = b.ins().trunc(sv);
                let e = self.call1(b, 4, t);
                let av = self.fv(b, *a);
                let r = b.ins().fmul(av, e);
                self.set_f(b, *a, r);
            }
            Op::FCom { a, b: c } => {
                let x = self.fv(b, *a);
                let y = self.fv(b, *c);
                self.fcompare(b, x, y);
            }
            Op::FComM { a, kind, mem } => {
                let y = self.fload(b, loc(mem)?, *kind)?;
                let x = self.fv(b, *a);
                self.fcompare(b, x, y);
            }
            Op::FTst(a) => {
                let x = self.fv(b, *a);
                let z = b.ins().f64const(0.0);
                self.fcompare(b, x, z);
            }
            Op::Fnstsw { mem, depth } => {
                let top = (((8 - *depth as u16) & 7) << 11) as i64;
                let v = b.use_var(self.fsw);
                let v = b.ins().bor_imm(v, top);
                let v = b.ins().band_imm(v, 0xFFFF);
                match mem {
                    None => {
                        // the low half of EAX only (see `St::rg`)
                        b.def_var(self.rg[EAX], v);
                        // AH = fsw bits 8..15: C0, C1 (0), C2, TOP, C3, B (0)
                        self.ah = self.fsw_src.map(|[c0, c2, c3]| {
                            let t = |i: i64| Bit::C((top >> i) & 1 == 1);
                            [Bit::V(c0), Bit::C(false), Bit::V(c2), t(11), t(12), t(13), Bit::V(c3), Bit::C(false)]
                        });
                        return Some(false);
                    }
                    Some(mm) => {
                        let v64 = b.ins().uextend(I64, v);
                        self.wr(b, loc(mm)?, 2, v64)?;
                    }
                }
            }
            Op::Jcc { cc, target } | Op::JccRot { cc, target, .. } => {
                let t = blk(*target as usize, cs)?;
                let fall = blk(k + 1, cs)?;
                let c = self.cond(b, *cc);
                let taken = b.create_block();
                b.ins().brif(c, taken, &[], fall.1, &[]);
                b.switch_to_block(taken);
                if let Op::JccRot { rot, .. } = &p.ops[k] {
                    self.rot(b, *rot);
                }
                jump_to(self, b, t);
                return Some(true);
            }
            Op::Jmp(t) => {
                let t = blk(*t as usize, cs)?;
                jump_to(self, b, t);
                return Some(true);
            }
            Op::JmpRot { target, rot } => {
                let t = blk(*target as usize, cs)?;
                self.rot(b, *rot);
                jump_to(self, b, t);
                return Some(true);
            }
            Op::Call { target, ret: r } | Op::CallRot { target, ret: r, .. } => {
                let ri = p.rets.iter().find(|e| e.0 == *r)?.1;
                let mut c2 = cs.clone();
                c2.push(ri);
                if let Op::CallRot { rot, .. } = &p.ops[k] {
                    self.rot(b, *rot);
                }
                let esp = b.use_var(self.rg[ESP]);
                let esp = b.ins().iadd_imm(esp, -4);
                b.def_var(self.rg[ESP], esp);
                let rv = b.ins().iconst(I64, *r as i64);
                self.wr(b, region(an.stack_loc(k, cs, true))?, 4, rv)?;
                let t = blk(*target as usize, &c2)?;
                b.ins().jump(t.1, &[]);
                return Some(true);
            }
            Op::Ret(nb) => {
                let esp = b.use_var(self.rg[ESP]);
                let esp = b.ins().iadd_imm(esp, 4 + *nb as i64);
                b.def_var(self.rg[ESP], esp);
                match cs.split_last() {
                    None => {
                        b.ins().jump(ret, &[]);
                    }
                    Some((&ri, rest)) => {
                        let t = blk(ri, &rest.to_vec())?;
                        b.ins().jump(t.1, &[]);
                    }
                }
                return Some(true);
            }
            Op::Gen(ins, _, _) => self.gen(b, ins, &loc)?,
            _ => return None,
        }
        Some(false)
    }

    /// The ops that test the condition bits in AH (see `St::ah`).
    fn ah_idiom(ins: &Ins) -> bool {
        matches!(
            ins,
            Ins::Sahf
                | Ins::Shift { op: Shift::Shr, dst: Rm::Reg(4), cnt: Some(1), size: 1 }
                | Ins::Alu { op: Alu::And | Alu::Xor | Alu::Cmp, dst: Rm::Reg(4), src: Src::Imm(_), size: 1 }
                | Ins::Test { a: Rm::Reg(4), b: Src::Imm(_), size: 1 }
        )
    }

    fn bit(b: &mut FunctionBuilder, x: Bit) -> Value {
        match x {
            Bit::C(c) => b.ins().iconst(I8, c as i64),
            Bit::V(v) => v,
        }
    }

    /// ZF, SF and PF of a byte known as bits
    fn szp_bits(&self, b: &mut FunctionBuilder, r: &[Bit; 8]) {
        // ZF: no bit set
        let mut any: Option<Value> = None;
        let mut one = false;
        // PF: an even number of bits set
        let mut par: Option<Value> = None;
        let mut odd = false;
        for x in r {
            match *x {
                Bit::C(c) => {
                    one |= c;
                    odd ^= c;
                }
                Bit::V(v) => {
                    any = Some(match any {
                        Some(a) => b.ins().bor(a, v),
                        None => v,
                    });
                    par = Some(match par {
                        Some(a) => b.ins().bxor(a, v),
                        None => v,
                    });
                }
            }
        }
        let zf = match (one, any) {
            (true, _) => b.ins().iconst(I8, 0),
            (false, None) => b.ins().iconst(I8, 1),
            (false, Some(a)) => b.ins().bxor_imm(a, 1),
        };
        self.set_flag(b, ZF, zf);
        let sf = Self::bit(b, r[7]);
        self.set_flag(b, SF, sf);
        let pf = match par {
            None => b.ins().iconst(I8, !odd as i64),
            Some(a) => b.ins().bxor_imm(a, !odd as i64),
        };
        self.set_flag(b, PF, pf);
    }

    /// The flags (and the new AH bits) of an AH idiom, after its generic code.
    fn ah_flags(&mut self, b: &mut FunctionBuilder, ins: &Ins) {
        let Some(h) = self.ah else { return };
        match ins {
            Ins::Sahf => {
                for (f, i) in [(SF, 7), (ZF, 6), (AF, 4), (PF, 2), (CF, 0)] {
                    let v = Self::bit(b, h[i]);
                    self.set_flag(b, f, v);
                }
            }
            Ins::Shift { .. } => {
                let cf = Self::bit(b, h[0]);
                self.set_flag(b, CF, cf);
                let of = Self::bit(b, h[7]);
                self.set_flag(b, OF, of);
                let r: [Bit; 8] = std::array::from_fn(|i| if i == 7 { Bit::C(false) } else { h[i + 1] });
                self.szp_bits(b, &r);
                self.ah = Some(r);
            }
            Ins::Alu { op: Alu::And | Alu::Xor, src: Src::Imm(m), .. } | Ins::Test { b: Src::Imm(m), .. } => {
                let and = !matches!(ins, Ins::Alu { op: Alu::Xor, .. });
                let r: [Bit; 8] = std::array::from_fn(|i| {
                    let mb = (m >> i) & 1 == 1;
                    match (and, h[i]) {
                        (true, x) => if mb { x } else { Bit::C(false) },
                        (false, Bit::C(c)) => Bit::C(c ^ mb),
                        (false, Bit::V(v)) => if mb { Bit::V(b.ins().bxor_imm(v, 1)) } else { Bit::V(v) },
                    }
                });
                self.szp_bits(b, &r);
                if !matches!(ins, Ins::Test { .. }) {
                    self.ah = Some(r);
                }
            }
            Ins::Alu { op: Alu::Cmp, src: Src::Imm(m), .. } => {
                // ZF: every bit equal to the immediate's
                let mut all: Option<Value> = None;
                let mut never = false;
                for (i, x) in h.iter().enumerate() {
                    let mb = (m >> i) & 1 == 1;
                    match *x {
                        Bit::C(c) => never |= c != mb,
                        Bit::V(v) => {
                            let e = if mb { v } else { b.ins().bxor_imm(v, 1) };
                            all = Some(match all {
                                Some(a) => b.ins().band(a, e),
                                None => e,
                            });
                        }
                    }
                }
                let zf = match (never, all) {
                    (true, _) => b.ins().iconst(I8, 0),
                    (false, None) => b.ins().iconst(I8, 1),
                    (false, Some(a)) => a,
                };
                self.set_flag(b, ZF, zf);
            }
            _ => {}
        }
    }

    /// An integer op (`lifted_gen`).
    fn gen(&mut self, b: &mut FunctionBuilder, ins: &Ins, loc: &dyn Fn(&Mem) -> Option<(char, u32)>) -> Option<()> {
        self.gen_generic(b, ins, loc)?;
        if Self::ah_idiom(ins) {
            self.ah_flags(b, ins);
        }
        Some(())
    }

    fn gen_generic(&mut self, b: &mut FunctionBuilder, ins: &Ins, loc: &dyn Fn(&Mem) -> Option<(char, u32)>) -> Option<()> {
        match ins {
            Ins::Nop => {}
            Ins::Sahf => {
                let a = b.use_var(self.rg[EAX]);
                let ah = b.ins().ushr_imm(a, 8);
                for (f, bit) in [(SF, 0x80), (ZF, 0x40), (AF, 0x10), (PF, 0x04), (CF, 0x01)] {
                    let t = b.ins().band_imm(ah, bit);
                    let t = b.ins().icmp_imm(IntCC::NotEqual, t, 0);
                    self.set_flag(b, f, t);
                }
            }
            Ins::Alu { op, dst, src, size } if !matches!(op, Alu::Adc | Alu::Sbb) => {
                let x = self.src(b, src, *size, loc)?;
                let a = self.rm_rd(b, dst, *size, loc)?;
                let r = self.alu(b, *op, a, x, *size);
                if *op != Alu::Cmp {
                    self.rm_wr(b, dst, *size, r, loc)?;
                }
            }
            Ins::Test { a, b: s, size } => {
                let x = self.src(b, s, *size, loc)?;
                let av = self.rm_rd(b, a, *size, loc)?;
                self.alu(b, Alu::And, av, x, *size);
            }
            Ins::Mov { dst, src, size } => {
                let v = self.src(b, src, *size, loc)?;
                self.rm_wr(b, dst, *size, v, loc)?;
            }
            Ins::Shift { op: op @ (Shift::Rcl | Shift::Rcr), dst: Rm::Reg(r), cnt: Some(1), size: 4 } => {
                let x = self.get_rg(b, *r as usize);
                let cf = self.flag(b, CF);
                let cf32 = b.ins().uextend(I32, cf);
                let (out, v) = if *op == Shift::Rcl {
                    let o = b.ins().ushr_imm(x, 31);
                    let s = b.ins().ishl_imm(x, 1);
                    (o, b.ins().bor(s, cf32))
                } else {
                    let o = b.ins().band_imm(x, 1);
                    let s = b.ins().ushr_imm(x, 1);
                    let c = b.ins().ishl_imm(cf32, 31);
                    (o, b.ins().bor(s, c))
                };
                self.set_rg(b, *r as usize, v);
                let out = b.ins().ireduce(I8, out);
                self.set_flag(b, CF, out);
            }
            Ins::Shift { op: op @ (Shift::Shl | Shift::Shr | Shift::Sar), dst: Rm::Reg(r), cnt: Some(n), size } => {
                // shift_reg_l with a constant count
                let c = *n as u32 & 31;
                if c == 0 {
                    return Some(());
                }
                let bits = *size as u32 * 8;
                let v = self.reg(b, *r, *size);
                let m = mask32(*size) as i64;
                let r2 = match op {
                    Shift::Shl => {
                        let cf = if c <= bits {
                            let t = b.ins().ushr_imm(v, (bits - c) as i64);
                            let t = b.ins().band_imm(t, 1);
                            b.ins().ireduce(I8, t)
                        } else {
                            b.ins().iconst(I8, 0)
                        };
                        self.set_flag(b, CF, cf);
                        let res = if c >= 32 {
                            b.ins().iconst(I32, 0)
                        } else {
                            let t = b.ins().ishl_imm(v, c as i64);
                            b.ins().band_imm(t, m)
                        };
                        let top = b.ins().ushr_imm(res, (bits - 1) as i64);
                        let top = b.ins().band_imm(top, 1);
                        let top = b.ins().ireduce(I8, top);
                        let of = b.ins().bxor(top, cf);
                        self.set_flag(b, OF, of);
                        res
                    }
                    Shift::Shr => {
                        let t = b.ins().ushr_imm(v, (c - 1) as i64);
                        let t = b.ins().band_imm(t, 1);
                        let cf = b.ins().ireduce(I8, t);
                        self.set_flag(b, CF, cf);
                        let t = b.ins().ushr_imm(v, (bits - 1) as i64);
                        let t = b.ins().band_imm(t, 1);
                        let of = b.ins().ireduce(I8, t);
                        self.set_flag(b, OF, of);
                        if c >= 32 {
                            b.ins().iconst(I32, 0)
                        } else {
                            b.ins().ushr_imm(v, c as i64)
                        }
                    }
                    _ => {
                        // sign extended to 32 bits
                        let sv = match size {
                            1 => {
                                let t = b.ins().ireduce(I8, v);
                                b.ins().sextend(I32, t)
                            }
                            2 => {
                                let t = b.ins().ireduce(I16, v);
                                b.ins().sextend(I32, t)
                            }
                            _ => v,
                        };
                        let t = b.ins().sshr_imm(sv, (c - 1).min(31) as i64);
                        let t = b.ins().band_imm(t, 1);
                        let cf = b.ins().ireduce(I8, t);
                        self.set_flag(b, CF, cf);
                        self.set_flag_c(b, OF, false);
                        let t = b.ins().sshr_imm(sv, c.min(31) as i64);
                        b.ins().band_imm(t, m)
                    }
                };
                self.set_szp(b, r2, *size);
                self.set_reg(b, *r, *size, r2);
            }
            Ins::Not(rm, size) => {
                let v = self.rm_rd(b, rm, *size, loc)?;
                let r = b.ins().bnot(v);
                self.rm_wr(b, rm, *size, r, loc)?;
            }
            Ins::Neg(rm, size) => {
                let v = self.rm_rd(b, rm, *size, loc)?;
                let z = b.ins().iconst(I32, 0);
                let r = self.alu(b, Alu::Sub, z, v, *size);
                let t = b.ins().band_imm(v, mask32(*size) as i64);
                let cf = b.ins().icmp_imm(IntCC::NotEqual, t, 0);
                self.set_flag(b, CF, cf);
                self.rm_wr(b, rm, *size, r, loc)?;
            }
            Ins::Inc(rm, size) | Ins::Dec(rm, size) => {
                let v = self.rm_rd(b, rm, *size, loc)?;
                let cf = self.flag(b, CF);
                let one = b.ins().iconst(I32, 1);
                let r = self.alu(b, if matches!(ins, Ins::Inc(..)) { Alu::Add } else { Alu::Sub }, v, one, *size);
                self.set_flag(b, CF, cf);
                self.rm_wr(b, rm, *size, r, loc)?;
            }
            Ins::Mul(rm, 4) | Ins::IMul1(rm, 4) => {
                let sg = matches!(ins, Ins::IMul1(..));
                let x = self.rm_rd(b, rm, 4, loc)?;
                let a = self.get_rg(b, EAX);
                let (a64, x64) = if sg { (b.ins().sextend(I64, a), b.ins().sextend(I64, x)) } else { (b.ins().uextend(I64, a), b.ins().uextend(I64, x)) };
                let r = b.ins().imul(a64, x64);
                let lo = b.ins().ireduce(I32, r);
                let hi = b.ins().ushr_imm(r, 32);
                let hi = b.ins().ireduce(I32, hi);
                self.set_rg(b, EAX, lo);
                self.set_rg(b, EDX, hi);
                let cf = if sg {
                    let back = b.ins().sextend(I64, lo);
                    b.ins().icmp(IntCC::NotEqual, r, back)
                } else {
                    b.ins().icmp_imm(IntCC::NotEqual, hi, 0)
                };
                self.set_flag(b, CF, cf);
                self.set_flag(b, OF, cf);
            }
            _ => return None,
        }
        Some(())
    }
}
