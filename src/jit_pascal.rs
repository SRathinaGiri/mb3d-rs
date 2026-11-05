//! The Pascal formulas (`jit.rs`) compiled to machine code with Cranelift.
//!
//! The tree of a formula is compiled for one set of option values: the
//! locals the formula never assigns (its options, constants, untouched
//! variables) become constants, and what depends on them only is computed
//! at compile time with the closures themselves (`fold`), so a branch on
//! an option disappears.  The code keeps the closures' representation:
//! every value an f64, ordinals whole numbers converted to i64 for their
//! operations exactly as the closures do (saturating, wrapping), so the
//! results are the closures' bit for bit (`examples/pascalcheck.rs --jit`).
//! The math functions are the closures' Rust functions, called; square
//! root, rounding, Min / Max and integer powers with a constant exponent
//! are inline.
//!
//! [`compile`] makes a function of its own (`fn(&mut Iteration)`) that the
//! iteration calls instead of the closures; [`emit`] puts a formula inline
//! into a function being built (the compiled hybrid loop, `clif_jit`), its
//! fields read and written through [`Fields`].

use crate::iteration::Iteration;
use crate::jit::{assigned_locals, fold, lit, Cmp, Field, IOp, Place, Program, Rnd, Ty, E, F1, F1_ALL, F2, F2_ALL, ROp, S};
use crate::m3f::M3f;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::{F32, F64, I32, I64, I8};
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlags, SigRef, Signature, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::Module;
use std::mem::offset_of;

/// The fields `s` writes (into `out`).
pub(crate) fn written_fields(s: &S, out: &mut Vec<Field>) {
    let mut put = |p: &Place| match p {
        Place::Field(f) => out.push(*f),
        Place::V(k) => out.push(Field::V(*k)),
        Place::Local(_) => {}
    };
    match s {
        S::Seq(l) => l.iter().for_each(|s| written_fields(s, out)),
        S::If(_, a, b) => {
            written_fields(a, out);
            written_fields(b, out);
        }
        S::While(_, b) | S::Repeat(b, _) | S::For { body: b, .. } => written_fields(b, out),
        S::Case(_, arms, other) => {
            arms.iter().for_each(|(_, s)| written_fields(s, out));
            if let Some(o) = other {
                written_fields(o, out);
            }
        }
        S::Store(p, _, _) => put(p),
        S::SinCos(_, (a, _), (b, _)) => {
            put(a);
            put(b);
        }
        S::Nop | S::Exit | S::Break | S::Continue => {}
    }
}

/// The tree of `prog` for the option values `values` of `def`, and the
/// start value of every local.
pub(crate) fn specialise(prog: &Program, def: &M3f, values: &[f64]) -> (S, Vec<f64>) {
    let mut start = vec![0.0; prog.nloc];
    for (i, v) in prog.init_values(def, values) {
        start[i] = v;
    }
    let asg = assigned_locals(&prog.tree, prog.nloc);
    let known = (0..prog.nloc).map(|i| (!asg[i]).then_some(start[i])).collect();
    (fold(&prog.tree, known), start)
}

// ---------------------------------------------------------------------------
// the math functions the code calls (the closures' functions)

macro_rules! f1_table {
    ($($k:literal)*) => {
        [$({
            extern "C" fn f(x: f64) -> f64 {
                F1_ALL[$k].eval(x)
            }
            f as *const u8
        }),*]
    };
}

fn f1_addr(f: F1) -> *const u8 {
    const N: usize = F1_ALL.len();
    let t: [*const u8; N] = f1_table!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34);
    t[f as usize]
}

fn f2_addr(f: F2) -> *const u8 {
    extern "C" fn atan2(a: f64, b: f64) -> f64 {
        F2_ALL[0].eval(a, b)
    }
    extern "C" fn hypot(a: f64, b: f64) -> f64 {
        F2_ALL[1].eval(a, b)
    }
    extern "C" fn logn(a: f64, b: f64) -> f64 {
        F2_ALL[2].eval(a, b)
    }
    extern "C" fn power(a: f64, b: f64) -> f64 {
        F2_ALL[3].eval(a, b)
    }
    match f {
        F2::ArcTan2 => atan2 as *const u8,
        F2::Hypot => hypot as *const u8,
        F2::LogN => logn as *const u8,
        F2::Power => power as *const u8,
        F2::Min | F2::Max => unreachable!("inline"),
    }
}

/// SinCos as the closures run it (`jit::sin_cos`: on macOS one
/// `__sincos_stret`, which can differ from `sin` alone in the last bit),
/// one half each
extern "C" fn sincos_sin(x: f64) -> f64 {
    crate::jit::sin_cos(x).0
}

extern "C" fn sincos_cos(x: f64) -> f64 {
    crate::jit::sin_cos(x).1
}

extern "C" fn int_power_fn(a: f64, n: i32) -> f64 {
    crate::jit::int_power(a, n)
}

extern "C" fn omax(a: f64, b: f64) -> f64 {
    a.max(b)
}

extern "C" fn omin(a: f64, b: f64) -> f64 {
    a.min(b)
}

// ---------------------------------------------------------------------------
// code generation

/// Where the fields of the iteration are (the function's iteration
/// pointer, or the variables of the compiled hybrid loop).
pub(crate) trait Fields {
    /// field `f` as the closures read it (`get_field`)
    fn get(&mut self, b: &mut FunctionBuilder, f: Field) -> Value;
    /// `v` (already the field's type, `store_value`) into field `f` as
    /// `set_field` stores it
    fn set(&mut self, b: &mut FunctionBuilder, f: Field, v: Value);
}

/// The fields through a pointer to an [`Iteration`].
pub(crate) struct ItFields {
    pub it: Value,
}

/// Offset and kind (0 f64, 1 i32, 2 f32, 3 bool) of field `f`.
fn field_slot(f: Field) -> (i32, u8) {
    use Field::*;
    let o = |x: usize| x as i32;
    match f {
        C(k) => (o(offset_of!(Iteration, c) + 8 * k), 0),
        J(k) => (o(offset_of!(Iteration, j) + 8 * k), 0),
        Ju(k) => (o(offset_of!(Iteration, ju) + 8 * k), 0),
        V(k) => (o(offset_of!(Iteration, v) + 8 * k), 0),
        Rout => (o(offset_of!(Iteration, rout)), 0),
        Rold => (o(offset_of!(Iteration, rold)), 0),
        OTrap => (o(offset_of!(Iteration, otrap)), 0),
        VaryScale => (o(offset_of!(Iteration, vary_scale)), 0),
        Deriv(0) => (o(offset_of!(Iteration, deriv1)), 0),
        Deriv(1) => (o(offset_of!(Iteration, deriv2)), 0),
        Deriv(_) => (o(offset_of!(Iteration, deriv3)), 0),
        Dfree(k) => (o(offset_of!(Iteration, dfree) + 8 * k), 0),
        RStop => (o(offset_of!(Iteration, rstop)), 0),
        MaxIt => (o(offset_of!(Iteration, max_it)), 1),
        ItResult => (o(offset_of!(Iteration, it_result)), 1),
        FirstIt => (o(offset_of!(Iteration, first_it)), 1),
        DEoption => (o(offset_of!(Iteration, de_option)), 1),
        SmoothIt => (o(offset_of!(Iteration, smooth_it)), 2),
        DoJulia => (o(offset_of!(Iteration, do_julia)), 3),
        CalcSit => (o(offset_of!(Iteration, calc_sit)), 3),
    }
}

/// Field `f` loaded from the iteration at `it`.
pub(crate) fn load_field(b: &mut FunctionBuilder, it: Value, f: Field) -> Value {
    let fl = MemFlags::trusted();
    let (off, kind) = field_slot(f);
    match kind {
        0 => b.ins().load(F64, fl, it, off),
        1 => {
            let x = b.ins().load(I32, fl, it, off);
            b.ins().fcvt_from_sint(F64, x)
        }
        2 => {
            let x = b.ins().load(F32, fl, it, off);
            b.ins().fpromote(F64, x)
        }
        _ => {
            let x = b.ins().load(I8, fl, it, off);
            let x = b.ins().uextend(I32, x);
            b.ins().fcvt_from_sint(F64, x)
        }
    }
}

/// `v` stored into field `f` of the iteration at `it` (`set_field`).
pub(crate) fn store_field(b: &mut FunctionBuilder, it: Value, f: Field, v: Value) {
    let fl = MemFlags::trusted();
    let (off, kind) = field_slot(f);
    let x = match kind {
        0 => v,
        1 => b.ins().fcvt_to_sint_sat(I32, v),
        2 => b.ins().fdemote(F32, v),
        _ => {
            let z = b.ins().f64const(0.0);
            b.ins().fcmp(FloatCC::NotEqual, v, z)
        }
    };
    b.ins().store(fl, x, it, off);
}

impl Fields for ItFields {
    fn get(&mut self, b: &mut FunctionBuilder, f: Field) -> Value {
        load_field(b, self.it, f)
    }
    fn set(&mut self, b: &mut FunctionBuilder, f: Field, v: Value) {
        store_field(b, self.it, f, v)
    }
}

struct Gen<'a, 'b, F: Fields> {
    b: &'a mut FunctionBuilder<'b>,
    fields: &'a mut F,
    locals: Vec<Variable>,
    /// (break, continue) targets of the loops around
    loops: Vec<(Block, Block)>,
    exit: Block,
    ptr: cranelift_codegen::ir::Type,
    sig1: SigRef,
    sig2: SigRef,
    sig_ip: SigRef,
}

impl<F: Fields> Gen<'_, '_, F> {
    fn fconst(&mut self, v: f64) -> Value {
        self.b.ins().f64const(v)
    }

    fn call1(&mut self, addr: *const u8, sig: SigRef, args: &[Value]) -> Value {
        let p = self.b.ins().iconst(self.ptr, addr as i64);
        let c = self.b.ins().call_indirect(sig, p, args);
        self.b.inst_results(c)[0]
    }

    /// `x as i64` (saturating, NaN 0)
    fn int_of(&mut self, x: Value) -> Value {
        self.b.ins().fcvt_to_sint_sat(I64, x)
    }

    fn real_of(&mut self, x: Value) -> Value {
        self.b.ins().fcvt_from_sint(F64, x)
    }

    /// `wrap32(x)` of an i64
    fn wrap32(&mut self, x: Value) -> Value {
        let y = self.b.ins().ireduce(I32, x);
        self.b.ins().fcvt_from_sint(F64, y)
    }

    /// a condition (I8) as 1.0 or 0.0
    fn bool_val(&mut self, c: Value) -> Value {
        let (one, zero) = (self.fconst(1.0), self.fconst(0.0));
        self.b.ins().select(c, one, zero)
    }

    /// `e != 0.0` as I8
    fn cond(&mut self, e: &E) -> Value {
        match e {
            E::Cmp(c, x, y) => {
                let (x, y) = (self.e(x), self.e(y));
                let cc = match c {
                    Cmp::Eq => FloatCC::Equal,
                    Cmp::Ne => FloatCC::NotEqual,
                    Cmp::Lt => FloatCC::LessThan,
                    Cmp::Gt => FloatCC::GreaterThan,
                    Cmp::Le => FloatCC::LessThanOrEqual,
                    Cmp::Ge => FloatCC::GreaterThanOrEqual,
                };
                self.b.ins().fcmp(cc, x, y)
            }
            // no side effects, no traps: both operands can be evaluated
            E::And(x, y) => {
                let (x, y) = (self.cond(x), self.cond(y));
                self.b.ins().band(x, y)
            }
            E::Or(x, y) => {
                let (x, y) = (self.cond(x), self.cond(y));
                self.b.ins().bor(x, y)
            }
            E::Xor(x, y) => {
                let (x, y) = (self.cond(x), self.cond(y));
                self.b.ins().bxor(x, y)
            }
            E::NotB(x) => {
                let x = self.e(x);
                let z = self.fconst(0.0);
                self.b.ins().fcmp(FloatCC::Equal, x, z)
            }
            E::IsNan(x) => {
                let x = self.e(x);
                self.b.ins().fcmp(FloatCC::Unordered, x, x)
            }
            _ => {
                let v = self.e(e);
                let z = self.fconst(0.0);
                self.b.ins().fcmp(FloatCC::NotEqual, v, z)
            }
        }
    }

    /// `int_power(a, n)` for a known `n`: the same multiplications
    fn int_power_const(&mut self, a: Value, n: i32) -> Value {
        let mut y = n.unsigned_abs();
        let mut base = a;
        let mut r = self.fconst(1.0);
        while y > 0 {
            while y & 1 == 0 {
                y >>= 1;
                base = self.b.ins().fmul(base, base);
            }
            y -= 1;
            r = self.b.ins().fmul(r, base);
        }
        if n < 0 {
            let one = self.fconst(1.0);
            self.b.ins().fdiv(one, r)
        } else {
            r
        }
    }

    fn e(&mut self, e: &E) -> Value {
        match e {
            E::Lit(v) => self.fconst(*v),
            E::Load(Place::Local(i)) => self.b.use_var(self.locals[*i]),
            E::Load(Place::V(k)) => self.fields.get(self.b, Field::V(*k)),
            E::Load(Place::Field(f)) => self.fields.get(self.b, *f),
            E::Neg(a, w) => {
                let a = self.e(a);
                if *w {
                    let i = self.int_of(a);
                    let n = self.b.ins().ineg(i);
                    self.wrap32(n)
                } else {
                    self.b.ins().fneg(a)
                }
            }
            E::NotI(a, wide) => {
                let a = self.e(a);
                let i = self.int_of(a);
                if *wide {
                    let n = self.b.ins().bnot(i);
                    self.real_of(n)
                } else {
                    let s = self.b.ins().ireduce(I32, i);
                    let n = self.b.ins().bnot(s);
                    self.b.ins().fcvt_from_sint(F64, n)
                }
            }
            E::NotB(_) | E::Cmp(..) | E::And(..) | E::Or(..) | E::Xor(..) | E::IsNan(_) => {
                let c = self.cond(e);
                self.bool_val(c)
            }
            E::R(o, x, y) => {
                let (x, y) = (self.e(x), self.e(y));
                match o {
                    ROp::Add => self.b.ins().fadd(x, y),
                    ROp::Sub => self.b.ins().fsub(x, y),
                    ROp::Mul => self.b.ins().fmul(x, y),
                    ROp::Div => self.b.ins().fdiv(x, y),
                }
            }
            E::I(o, w, x, y) => {
                let (fx, fy) = (self.e(x), self.e(y));
                let (a, bb) = (self.int_of(fx), self.int_of(fy));
                let r = match o {
                    IOp::Add => self.b.ins().iadd(a, bb),
                    IOp::Sub => self.b.ins().isub(a, bb),
                    IOp::Mul => self.b.ins().imul(a, bb),
                    IOp::And => self.b.ins().band(a, bb),
                    IOp::Or => self.b.ins().bor(a, bb),
                    IOp::Xor => self.b.ins().bxor(a, bb),
                    IOp::Div | IOp::Mod => {
                        // the closures: 0 for a zero divisor, wrapping for MIN / -1
                        let zero = self.b.ins().icmp_imm(IntCC::Equal, bb, 0);
                        let m1 = self.b.ins().icmp_imm(IntCC::Equal, bb, -1);
                        let min = self.b.ins().icmp_imm(IntCC::Equal, a, i64::MIN);
                        let ovf = self.b.ins().band(m1, min);
                        let bad = self.b.ins().bor(zero, ovf);
                        let one = self.b.ins().iconst(I64, 1);
                        let d = self.b.ins().select(bad, one, bb);
                        let q = if *o == IOp::Div { self.b.ins().sdiv(a, d) } else { self.b.ins().srem(a, d) };
                        // MIN / -1 = MIN (= MIN / 1), MIN mod -1 = 0 (= MIN mod 1)
                        let r = if *w { self.wrap32(q) } else { self.real_of(q) };
                        let z = self.fconst(0.0);
                        return self.b.ins().select(zero, z, r);
                    }
                    IOp::Shl | IOp::Shr => {
                        let n = self.b.ins().fcvt_to_uint_sat(I32, fy);
                        return match (o, w) {
                            (IOp::Shl, true) => {
                                let s = self.b.ins().ireduce(I32, a);
                                let r = self.b.ins().ishl(s, n);
                                self.b.ins().fcvt_from_sint(F64, r)
                            }
                            (IOp::Shl, false) => {
                                let r = self.b.ins().ishl(a, n);
                                self.real_of(r)
                            }
                            (_, true) => {
                                let s = self.b.ins().ireduce(I32, a);
                                let r = self.b.ins().ushr(s, n);
                                self.b.ins().fcvt_from_sint(F64, r)
                            }
                            _ => {
                                let r = self.b.ins().ushr(a, n);
                                self.real_of(r)
                            }
                        };
                    }
                };
                if *w {
                    self.wrap32(r)
                } else {
                    self.real_of(r)
                }
            }
            E::F1(f, a) => {
                let x = self.e(a);
                match f {
                    F1::Sqrt => self.b.ins().sqrt(x),
                    F1::Int => self.b.ins().trunc(x),
                    // f64::fract: self - self.trunc()
                    F1::Frac => {
                        let t = self.b.ins().trunc(x);
                        self.b.ins().fsub(x, t)
                    }
                    _ => {
                        let s = self.sig1;
                        self.call1(f1_addr(*f), s, &[x])
                    }
                }
            }
            E::F2(F2::Power, a, p) if lit(p).is_some_and(|v| v.fract() == 0.0 && v.abs() <= i32::MAX as f64) => {
                let n = lit(p).expect("checked");
                let x = self.e(a);
                if n == 0.0 {
                    return self.fconst(1.0);
                }
                let r = self.int_power_const(x, n as i32);
                if n > 0.0 {
                    // Math.Power: 0 for a zero base (also -0)
                    let z = self.fconst(0.0);
                    let is0 = self.b.ins().fcmp(FloatCC::Equal, x, z);
                    self.b.ins().select(is0, z, r)
                } else {
                    r
                }
            }
            E::F2(f, x, y) => {
                let (x, y) = (self.e(x), self.e(y));
                match f {
                    F2::Min => {
                        let lt = self.b.ins().fcmp(FloatCC::LessThan, x, y);
                        self.b.ins().select(lt, x, y)
                    }
                    F2::Max => {
                        let gt = self.b.ins().fcmp(FloatCC::GreaterThan, x, y);
                        self.b.ins().select(gt, x, y)
                    }
                    _ => {
                        let s = self.sig2;
                        self.call1(f2_addr(*f), s, &[x, y])
                    }
                }
            }
            E::OMinMax(mx, x, y) => {
                let (x, y) = (self.e(x), self.e(y));
                let s = self.sig2;
                self.call1(if *mx { omax as *const u8 } else { omin as *const u8 }, s, &[x, y])
            }
            E::IntPower(a, n) => {
                let x = self.e(a);
                if let Some(k) = lit(n) {
                    // `b(e) as i32`
                    return self.int_power_const(x, k as i32);
                }
                let fnv = self.e(n);
                let k = self.b.ins().fcvt_to_sint_sat(I32, fnv);
                let s = self.sig_ip;
                self.call1(int_power_fn as *const u8, s, &[x, k])
            }
            E::IAbs(a) => {
                let x = self.e(a);
                let i = self.int_of(x);
                let r = self.b.ins().iabs(i);
                self.wrap32(r)
            }
            E::ISqr(a) => {
                let x = self.e(a);
                let i = self.int_of(x);
                let r = self.b.ins().imul(i, i);
                self.wrap32(r)
            }
            E::RAbs(a) => {
                let x = self.e(a);
                self.b.ins().fabs(x)
            }
            E::RSqr(a) => {
                let x = self.e(a);
                self.b.ins().fmul(x, x)
            }
            E::Rnd(k, w, a) => {
                let x = self.e(a);
                let r = match k {
                    Rnd::Round => self.b.ins().nearest(x),
                    Rnd::Trunc => self.b.ins().trunc(x),
                    Rnd::Ceil => self.b.ins().ceil(x),
                    Rnd::Floor => self.b.ins().floor(x),
                };
                if *w {
                    let i = self.int_of(r);
                    self.wrap32(i)
                } else {
                    r
                }
            }
            E::Odd(a) => {
                let x = self.e(a);
                let i = self.int_of(x);
                let r = self.b.ins().band_imm(i, 1);
                self.real_of(r)
            }
            E::IsInf(a) => {
                let x = self.e(a);
                let ax = self.b.ins().fabs(x);
                let inf = self.fconst(f64::INFINITY);
                let c = self.b.ins().fcmp(FloatCC::Equal, ax, inf);
                self.bool_val(c)
            }
            E::Sign(a) => {
                let x = self.e(a);
                let z = self.fconst(0.0);
                let gt = self.b.ins().fcmp(FloatCC::GreaterThan, x, z);
                let lt = self.b.ins().fcmp(FloatCC::LessThan, x, z);
                let (one, m1) = (self.fconst(1.0), self.fconst(-1.0));
                let r = self.b.ins().select(lt, m1, z);
                self.b.ins().select(gt, one, r)
            }
        }
    }

    /// `store_value(ty, v)`
    fn store_value(&mut self, ty: Ty, v: Value) -> Value {
        match ty {
            Ty::Single => {
                let s = self.b.ins().fdemote(F32, v);
                self.b.ins().fpromote(F64, s)
            }
            Ty::Int => {
                let i = self.int_of(v);
                self.wrap32(i)
            }
            Ty::Bool => {
                let z = self.fconst(0.0);
                let c = self.b.ins().fcmp(FloatCC::NotEqual, v, z);
                self.bool_val(c)
            }
            Ty::Real | Ty::Int64 => v,
        }
    }

    fn store(&mut self, p: Place, ty: Ty, v: Value) {
        match p {
            Place::Local(i) => {
                let x = self.store_value(ty, v);
                self.b.def_var(self.locals[i], x);
            }
            Place::V(k) => self.fields.set(self.b, Field::V(k), v),
            Place::Field(f) => {
                let x = self.store_value(ty, v);
                self.fields.set(self.b, f, x);
            }
        }
    }

    /// a jump that ends the block; the code after it goes into a new
    /// block nothing jumps to
    fn jump_away(&mut self, to: Block) {
        self.b.ins().jump(to, &[]);
        let dead = self.b.create_block();
        self.b.switch_to_block(dead);
    }

    fn s(&mut self, s: &S) {
        match s {
            S::Nop => {}
            S::Seq(l) => l.iter().for_each(|s| self.s(s)),
            S::If(c, a, b) => {
                let c = self.cond(c);
                let (ta, tb, after) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(c, ta, &[], tb, &[]);
                self.b.switch_to_block(ta);
                self.s(a);
                self.b.ins().jump(after, &[]);
                self.b.switch_to_block(tb);
                self.s(b);
                self.b.ins().jump(after, &[]);
                self.b.switch_to_block(after);
            }
            S::While(c, body) => {
                let (head, bl, after) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let c = self.cond(c);
                self.b.ins().brif(c, bl, &[], after, &[]);
                self.b.switch_to_block(bl);
                self.loops.push((after, head));
                self.s(body);
                self.loops.pop();
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(after);
            }
            S::Repeat(body, c) => {
                let (bl, test, after) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().jump(bl, &[]);
                self.b.switch_to_block(bl);
                self.loops.push((after, test));
                self.s(body);
                self.loops.pop();
                self.b.ins().jump(test, &[]);
                self.b.switch_to_block(test);
                let c = self.cond(c);
                self.b.ins().brif(c, after, &[], bl, &[]);
                self.b.switch_to_block(after);
            }
            S::For { slot, from, to, down, body } => {
                let (fa, fb) = (self.e(from), self.e(to));
                let (a, lim) = (self.int_of(fa), self.int_of(fb));
                let iv = self.b.declare_var(I64);
                self.b.def_var(iv, a);
                let (head, bl, inc, after) = (self.b.create_block(), self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let i = self.b.use_var(iv);
                let done = self.b.ins().icmp(if *down { IntCC::SignedLessThan } else { IntCC::SignedGreaterThan }, i, lim);
                self.b.ins().brif(done, after, &[], bl, &[]);
                self.b.switch_to_block(bl);
                let fi = self.real_of(i);
                self.b.def_var(self.locals[*slot], fi);
                self.loops.push((after, inc));
                self.s(body);
                self.loops.pop();
                self.b.ins().jump(inc, &[]);
                self.b.switch_to_block(inc);
                let i = self.b.use_var(iv);
                let n = self.b.ins().iadd_imm(i, if *down { -1 } else { 1 });
                self.b.def_var(iv, n);
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(after);
            }
            S::Case(sel, arms, other) => {
                let fv = self.e(sel);
                let v = self.int_of(fv);
                let after = self.b.create_block();
                for (labels, st) in arms {
                    let (arm, next) = (self.b.create_block(), self.b.create_block());
                    let mut hit: Option<Value> = None;
                    for &(lo, hi) in labels {
                        let c = if lo == hi {
                            self.b.ins().icmp_imm(IntCC::Equal, v, lo)
                        } else {
                            let a = self.b.ins().icmp_imm(IntCC::SignedGreaterThanOrEqual, v, lo);
                            let z = self.b.ins().icmp_imm(IntCC::SignedLessThanOrEqual, v, hi);
                            self.b.ins().band(a, z)
                        };
                        hit = Some(match hit {
                            Some(h) => self.b.ins().bor(h, c),
                            None => c,
                        });
                    }
                    let hit = hit.unwrap_or_else(|| self.b.ins().iconst(I8, 0));
                    self.b.ins().brif(hit, arm, &[], next, &[]);
                    self.b.switch_to_block(arm);
                    self.s(st);
                    self.b.ins().jump(after, &[]);
                    self.b.switch_to_block(next);
                }
                if let Some(o) = other {
                    self.s(o);
                }
                self.b.ins().jump(after, &[]);
                self.b.switch_to_block(after);
            }
            S::Exit => {
                let x = self.exit;
                self.jump_away(x);
            }
            S::Break | S::Continue => {
                // (the parser allows them only inside loops)
                let Some(&(br, co)) = self.loops.last() else { return };
                self.jump_away(if matches!(s, S::Break) { br } else { co });
            }
            S::Store(p, ty, v) => {
                let x = self.e(v);
                self.store(*p, *ty, x);
            }
            S::SinCos(a, (ps, ts), (pc, tc)) => {
                let x = self.e(a);
                let s1 = self.sig1;
                let sn = self.call1(sincos_sin as *const u8, s1, &[x]);
                let cs = self.call1(sincos_cos as *const u8, s1, &[x]);
                self.store(*ps, *ts, sn);
                self.store(*pc, *tc, cs);
            }
        }
    }
}

/// The signature of a function of `n` f64 (and with `int` an i32 as the
/// second) returning an f64.
fn math_sig(call_conv: CallConv, n: usize, int: bool) -> Signature {
    let mut s = Signature::new(call_conv);
    for k in 0..n {
        s.params.push(AbiParam::new(if int && k == 1 { I32 } else { F64 }));
    }
    s.returns.push(AbiParam::new(F64));
    s
}

/// Formula `tree` inline at the current block of `b`: its locals start
/// with `start`, its fields go through `fields`; the code continues in the
/// block this returns (where the formula ended or exited).
pub(crate) fn emit(b: &mut FunctionBuilder, fields: &mut impl Fields, tree: &S, start: &[f64], ptr: cranelift_codegen::ir::Type, call_conv: CallConv) -> Block {
    let sig1 = b.import_signature(math_sig(call_conv, 1, false));
    let sig2 = b.import_signature(math_sig(call_conv, 2, false));
    let sig_ip = b.import_signature(math_sig(call_conv, 2, true));
    let exit = b.create_block();
    let mut locals = Vec::with_capacity(start.len());
    for v in start {
        let var = b.declare_var(F64);
        let x = b.ins().f64const(*v);
        b.def_var(var, x);
        locals.push(var);
    }
    let mut g = Gen { b, fields, locals, loops: Vec::new(), exit, ptr, sig1, sig2, sig_ip };
    g.s(tree);
    g.b.ins().jump(exit, &[]);
    g.b.switch_to_block(exit);
    exit
}

// ---------------------------------------------------------------------------
// a formula as a function of its own

type PascalFn = unsafe extern "C" fn(*mut Iteration);

/// A formula compiled for one set of option values.
pub struct Compiled {
    f: PascalFn,
    /// the option values it was compiled for
    pub values: Vec<f64>,
    /// owns the code
    _module: JITModule,
}

// SAFETY: the module only owns the finished, immutable code
unsafe impl Send for Compiled {}
unsafe impl Sync for Compiled {}

impl std::fmt::Debug for Compiled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compiled({:?})", self.values)
    }
}

impl Compiled {
    /// One iteration step on `it` (as `Program::run` with the values it
    /// was compiled for).
    #[inline]
    pub fn run(&self, it: &mut Iteration) {
        // SAFETY: the function reads and writes the fields of the
        // iteration it is given, nothing else
        unsafe { (self.f)(it) }
    }
}

/// A new JIT module for the host.
pub(crate) fn module() -> Option<JITModule> {
    let mut fb = settings::builder();
    fb.set("opt_level", "speed").ok()?;
    let isa = cranelift_native::builder().ok()?.finish(settings::Flags::new(fb)).ok()?;
    Some(JITModule::new(JITBuilder::with_isa(isa, cranelift_module::default_libcall_names())))
}

/// `prog` of formula `def` compiled with the option values `values`; None
/// if it cannot be compiled.
pub fn compile(prog: &Program, def: &M3f, values: &[f64]) -> Option<Compiled> {
    let (tree, start) = specialise(prog, def, values);
    let mut module = module()?;
    let ptr = module.target_config().pointer_type();
    let call_conv = module.target_config().default_call_conv;
    let mut ctx = module.make_context();
    ctx.func.signature.params.push(AbiParam::new(ptr));
    let mut fctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let it = b.block_params(entry)[0];
        let mut fields = ItFields { it };
        emit(&mut b, &mut fields, &tree, &start, ptr, call_conv);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    if std::env::var_os("MB3D_JIT_DUMP").is_some() {
        crate::elog!("{}: {}", prog.name, ctx.func.display());
    }
    let id = module.declare_anonymous_function(&ctx.func.signature).ok()?;
    if let Err(e) = module.define_function(id, &mut ctx) {
        crate::elog!("cranelift ({}): {e:?}", prog.name);
        return None;
    }
    module.clear_context(&mut ctx);
    module.finalize_definitions().ok()?;
    let f = module.get_finalized_function(id);
    Some(Compiled {
        // SAFETY: the function was built with this signature
        f: unsafe { std::mem::transmute::<*const u8, PascalFn>(f) },
        values: values.to_vec(),
        _module: module,
    })
}


/// The compiled formulas of the program run (by formula, source and option
/// values): a scene calculated again (a preview, the next frame, a tile)
/// does not compile them again.
type Cache = std::collections::HashMap<(usize, Vec<u64>), (std::sync::Arc<Program>, Option<std::sync::Arc<Compiled>>)>;
static CACHE: std::sync::LazyLock<std::sync::Mutex<Cache>> = std::sync::LazyLock::new(Default::default);

/// [`compile`] through the cache.
pub fn cached(prog: &std::sync::Arc<Program>, def: &M3f, values: &[f64]) -> Option<std::sync::Arc<Compiled>> {
    // the program's address (the cache keeps the program, so the address
    // is not used again for another one)
    let key = (std::sync::Arc::as_ptr(prog) as usize, values.iter().map(|v| v.to_bits()).collect::<Vec<u64>>());
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, r)) = c.get(&key) {
        return r.clone();
    }
    if c.len() > 256 {
        c.clear();
    }
    let r = compile(prog, def, values).map(std::sync::Arc::new);
    if r.is_none() {
        crate::elog!("{}: not compiled with Cranelift (the closures run it)", prog.name);
    }
    c.insert(key, (prog.clone(), r.clone()));
    r
}
