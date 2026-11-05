//! The Pascal formulas (`jit.rs`) translated to WGSL for the graphics card.
//!
//! The formula becomes a shader function with the interface of the lifted
//! x86 formulas (`x86::lift_wgsl`): `fn <name>(cb: u32)` on the record
//! cells (`r<cell>f`, the private globals the shader's loop fills from the
//! iteration), its option values from the constants buffer at `cb` (so the
//! shader text does not depend on them), the constants of the file and the
//! untouched locals folded in.  Like the CPU code it keeps the closures'
//! representation (every value a real, ordinals converted to i32 for their
//! operations) in the shader's precision `R` (f32, or f64 for deep zooms).

use crate::jit::{assigned_locals, fold, Cmp, Field, IOp, Init, Place, Program, ROp, Rnd, Ty, E, F1, F2, S};
use crate::m3f::M3f;
use crate::x86::lift_wgsl::{CellTy, WgslFormula};
use std::collections::BTreeMap;
use std::fmt::Write;

/// What a field is in the record: a double cell, or a part of an integer
/// cell (the low / high 32 bits as i32, the low as f32).
#[derive(Clone, Copy, PartialEq)]
enum Part {
    Double,
    LoI32,
    HiI32,
    LoF32,
}

/// The record cell of field `f` (cell = byte offset / 8 from J4); None if
/// the GPU's record does not have it.
fn field_cell(f: Field) -> Option<(u32, Part)> {
    use Field::*;
    Some(match f {
        V(k) => (3 + k as u32, Part::Double),
        C(k) => (7 + k as u32, Part::Double),
        J(3) => (0, Part::Double),
        J(k) => (10 + k as u32, Part::Double),
        Rout => (14, Part::Double),
        Rold => (1, Part::Double),
        OTrap => (31, Part::Double),
        VaryScale => (32, Part::Double),
        Dfree(k) => (34 + k as u32, Part::Double),
        Deriv(k) => (36 + k as u32, Part::Double),
        ItResult => (15, Part::LoI32),
        MaxIt => (15, Part::HiI32),
        FirstIt => (33, Part::LoI32),
        RStop => (16, Part::LoF32),
        Ju(_) | DEoption | SmoothIt | DoJulia | CalcSit => return None,
    })
}

/// The double cells the shader's loop copies back into the iteration
/// (`marshal_out` in gpu.rs): the fields a formula may write.
fn writable(c: u32) -> bool {
    matches!(c, 0 | 3..=6 | 10..=12 | 14 | 31 | 32 | 34..=38)
}

/// The helpers of the translated formulas (once per shader, after the
/// prelude of `lift_wgsl`).
pub fn prelude(dbl: bool) -> String {
    let isinf = if dbl {
        "fn pas_isinf(x: R) -> bool { return (bitcast<u64>(x) & 0x7FFFFFFFFFFFFFFFlu) == 0x7FF0000000000000lu; }"
    } else {
        "fn pas_isinf(x: R) -> bool { return (bitcast<u32>(x) & 0x7FFFFFFFu) == 0x7F800000u; }"
    };
    format!(
        r#"
// ---- the Pascal formulas (jit_wgsl.rs)
fn pas_b(c: bool) -> R {{ return select(R(0), R(1), c); }}
{isinf}
fn pas_trunc(x: R) -> R {{ if (abs(x) < R(2147483647)) {{ return R(i32(x)); }} return x; }}
fn pas_floor(x: R) -> R {{ let t = pas_trunc(x); return t - select(R(0), R(1), x < t); }}
fn pas_ceil(x: R) -> R {{ let t = pas_trunc(x); return t + select(R(0), R(1), x > t); }}
fn pas_sign(x: R) -> R {{ return select(select(R(0), R(-1), x < R(0)), R(1), x > R(0)); }}
fn pas_sqr(x: R) -> R {{ return x * x; }}
fn pas_isqr(x: R) -> R {{ let i = i32(x); return R(i * i); }}
fn pas_min(a: R, b: R) -> R {{ return select(b, a, a < b); }}
fn pas_max(a: R, b: R) -> R {{ return select(b, a, a > b); }}
fn pas_idiv(a: i32, b: i32) -> R {{ if (b == 0) {{ return R(0); }} return R(a / b); }}
fn pas_imod(a: i32, b: i32) -> R {{ if (b == 0) {{ return R(0); }} return R(a % b); }}
fn pas_ipow(a: R, n: i32) -> R {{
    var y = u32(abs(n));
    var b = a;
    var r = R(1);
    loop {{
        if (y == 0u) {{ break; }}
        if ((y & 1u) == 0u) {{ y = y >> 1u; b = b * b; continue; }}
        y = y - 1u;
        r = r * b;
    }}
    if (n < 0) {{ return R(1) / r; }}
    return r;
}}
fn pas_power(a: R, b: R) -> R {{
    if (b == R(0)) {{ return R(1); }}
    if (a == R(0) && b > R(0)) {{ return R(0); }}
    if (b - pas_trunc(b) == R(0) && abs(b) <= R(2147483647)) {{ return pas_ipow(a, i32(b)); }}
    return r_exp(b * r_log(a));
}}
fn pas_sinh(x: R) -> R {{ let e = r_exp(x); return (e - R(1) / e) * R(0.5); }}
fn pas_cosh(x: R) -> R {{ let e = r_exp(x); return (e + R(1) / e) * R(0.5); }}
fn pas_tanh(x: R) -> R {{ if (abs(x) > R(20)) {{ return pas_sign(x); }} let e = r_exp(R(2) * x); return (e - R(1)) / (e + R(1)); }}
fn pas_asinh(x: R) -> R {{ let a = abs(x); let r = r_log(a + sqrt(a * a + R(1))); return select(r, -r, x < R(0)); }}
fn pas_acosh(x: R) -> R {{ return r_log(x + sqrt(x * x - R(1))); }}
fn pas_atanh(x: R) -> R {{ return R(0.5) * r_log((R(1) + x) / (R(1) - x)); }}
"#
    )
}

/// A literal of the shader's real type.
fn rl(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 16_777_216.0 {
        if v == 0.0 && v.is_sign_negative() {
            return "(-R(0))".into();
        }
        return format!("R({})", v as i64);
    }
    let b = v.to_bits();
    format!("rlit({:#x}u, {:#x}u, {:#x}u)", (v as f32).to_bits(), b as u32, (b >> 32) as u32)
}

struct Gen<'a> {
    o: String,
    depth: usize,
    /// the representation of each record cell the formula uses
    cells: BTreeMap<u32, CellTy>,
    force: &'a BTreeMap<u32, CellTy>,
    /// a temporary name counter
    tmp: usize,
}

/// Why a formula cannot go to the GPU.
type Res<T> = Result<T, String>;

impl Gen<'_> {
    fn line(&mut self, s: &str) {
        for _ in 0..self.depth {
            self.o.push_str("    ");
        }
        self.o.push_str(s);
        self.o.push('\n');
    }

    fn cell(&mut self, f: Field) -> Res<(u32, Part)> {
        let (c, p) = field_cell(f).ok_or_else(|| format!("the field {f:?} is not on the GPU"))?;
        let t = if p == Part::Double { self.force.get(&c).copied().unwrap_or(CellTy::Whole) } else { CellTy::Halves };
        self.cells.insert(c, t);
        Ok((c, p))
    }

    fn get(&mut self, f: Field) -> Res<String> {
        let (c, p) = self.cell(f)?;
        Ok(match (p, self.cells[&c]) {
            (Part::Double, CellTy::Whole) => format!("r{c}f"),
            (Part::Double, CellTy::Halves) => format!("from_f64_bits(r{c}h, r{c}l)"),
            (Part::LoI32, _) => format!("R(bitcast<i32>(r{c}l))"),
            (Part::HiI32, _) => format!("R(bitcast<i32>(r{c}h))"),
            (Part::LoF32, _) => format!("R(bitcast<f32>(r{c}l))"),
        })
    }

    fn set(&mut self, f: Field, v: &str) -> Res<()> {
        let (c, p) = self.cell(f)?;
        if p != Part::Double || !writable(c) {
            return Err(format!("the formula writes {f:?}, which the GPU does not keep"));
        }
        match self.cells[&c] {
            CellTy::Whole => self.line(&format!("r{c}f = {v};")),
            CellTy::Halves => {
                self.tmp += 1;
                let t = self.tmp;
                self.line(&format!("let t{t} = {v}; r{c}l = f64_lo(t{t}); r{c}h = f64_hi(t{t});"));
            }
        }
        Ok(())
    }

    fn i(&mut self, e: &E) -> Res<String> {
        // a constant as an integer literal (naga does not convert constant doubles)
        if let E::Lit(v) = e {
            let k = *v as i32;
            return Ok(if k == i32::MIN { "i32(-2147483647 - 1)".into() } else { format!("i32({k})") });
        }
        Ok(format!("i32({})", self.e(e)?))
    }

    /// `e != 0` as a WGSL bool
    fn cond(&mut self, e: &E) -> Res<String> {
        Ok(match e {
            E::Cmp(c, a, b) => {
                let op = match c {
                    Cmp::Eq => "==",
                    Cmp::Ne => "!=",
                    Cmp::Lt => "<",
                    Cmp::Gt => ">",
                    Cmp::Le => "<=",
                    Cmp::Ge => ">=",
                };
                format!("({} {op} {})", self.e(a)?, self.e(b)?)
            }
            E::And(a, b) => format!("({} && {})", self.cond(a)?, self.cond(b)?),
            E::Or(a, b) => format!("({} || {})", self.cond(a)?, self.cond(b)?),
            E::Xor(a, b) => format!("({} != {})", self.cond(a)?, self.cond(b)?),
            E::NotB(a) => format!("({} == R(0))", self.e(a)?),
            E::IsNan(a) => format!("fnan({})", self.e(a)?),
            E::IsInf(a) => format!("pas_isinf({})", self.e(a)?),
            E::Lit(v) => (if *v != 0.0 { "true" } else { "false" }).to_string(),
            _ => format!("({} != R(0))", self.e(e)?),
        })
    }

    fn e(&mut self, e: &E) -> Res<String> {
        Ok(match e {
            E::Lit(v) => rl(*v),
            E::Load(Place::Local(i)) => format!("l{i}"),
            E::Load(Place::V(k)) => self.get(Field::V(*k))?,
            E::Load(Place::Field(f)) => self.get(*f)?,
            E::Neg(a, true) => format!("R(-{})", self.i(a)?),
            E::Neg(a, false) => format!("(-{})", self.e(a)?),
            E::NotI(a, _) => format!("R(~{})", self.i(a)?),
            E::NotB(_) | E::Cmp(..) | E::And(..) | E::Or(..) | E::Xor(..) | E::IsNan(_) | E::IsInf(_) => format!("pas_b({})", self.cond(e)?),
            E::R(o, a, b) => {
                let op = match o {
                    ROp::Add => "+",
                    ROp::Sub => "-",
                    ROp::Mul => "*",
                    ROp::Div => "/",
                };
                format!("({} {op} {})", self.e(a)?, self.e(b)?)
            }
            E::I(o, _, a, b) => {
                let (x, y) = (self.i(a)?, self.i(b)?);
                match o {
                    IOp::Add => format!("R({x} + {y})"),
                    IOp::Sub => format!("R({x} - {y})"),
                    IOp::Mul => format!("R({x} * {y})"),
                    IOp::And => format!("R({x} & {y})"),
                    IOp::Or => format!("R({x} | {y})"),
                    IOp::Xor => format!("R({x} ^ {y})"),
                    IOp::Div => format!("pas_idiv({x}, {y})"),
                    IOp::Mod => format!("pas_imod({x}, {y})"),
                    IOp::Shl => format!("R({x} << (u32(max({y}, 0)) & 31u))"),
                    IOp::Shr => format!("R(bitcast<i32>(bitcast<u32>({x}) >> (u32(max({y}, 0)) & 31u)))"),
                }
            }
            E::F1(f, a) => {
                let x = self.e(a)?;
                use F1::*;
                match f {
                    Sin => format!("r_sin({x})"),
                    Cos => format!("r_cos({x})"),
                    Tan => format!("r_tan({x})"),
                    Cot => format!("(R(1) / r_tan({x}))"),
                    Sec => format!("(R(1) / r_cos({x}))"),
                    Csc => format!("(R(1) / r_sin({x}))"),
                    ArcTan => format!("r_atan({x})"),
                    ArcSin => format!("r_asin({x})"),
                    ArcCos => format!("r_acos({x})"),
                    ArcCot => format!("r_atan(R(1) / {x})"),
                    ArcSec => format!("r_acos(R(1) / {x})"),
                    ArcCsc => format!("r_asin(R(1) / {x})"),
                    Sinh => format!("pas_sinh({x})"),
                    Cosh => format!("pas_cosh({x})"),
                    Tanh => format!("pas_tanh({x})"),
                    Coth => format!("(R(1) / pas_tanh({x}))"),
                    Sech => format!("(R(1) / pas_cosh({x}))"),
                    Csch => format!("(R(1) / pas_sinh({x}))"),
                    ArcSinh => format!("pas_asinh({x})"),
                    ArcCosh => format!("pas_acosh({x})"),
                    ArcTanh => format!("pas_atanh({x})"),
                    ArcCoth => format!("pas_atanh(R(1) / {x})"),
                    ArcSech => format!("pas_acosh(R(1) / {x})"),
                    ArcCsch => format!("pas_asinh(R(1) / {x})"),
                    Sqrt => format!("sqrt({x})"),
                    Exp => format!("r_exp({x})"),
                    Ln => format!("r_log({x})"),
                    LnXp1 => format!("r_log(R(1) + {x})"),
                    Log10 => format!("(r_log2({x}) * {})", rl(std::f64::consts::LOG10_2)),
                    Log2 => format!("r_log2({x})"),
                    Int => format!("pas_trunc({x})"),
                    Frac => {
                        self.tmp += 1;
                        let t = self.tmp;
                        self.line(&format!("let t{t} = {x};"));
                        format!("(t{t} - pas_trunc(t{t}))")
                    }
                    RadToDeg => format!("({x} * {})", rl(180.0 / std::f64::consts::PI)),
                    DegToRad => format!("({x} * {})", rl(std::f64::consts::PI / 180.0)),
                    DegNormalize => {
                        self.tmp += 1;
                        let t = self.tmp;
                        self.line(&format!("let t{t} = {x};"));
                        format!("(t{t} - pas_floor(t{t} / R(360)) * R(360))")
                    }
                }
            }
            E::F2(f, a, b) => {
                let (x, y) = (self.e(a)?, self.e(b)?);
                match f {
                    F2::ArcTan2 => format!("r_atan2({x}, {y})"),
                    F2::Hypot => format!("sqrt(pas_sqr({x}) + pas_sqr({y}))"),
                    F2::LogN => format!("(r_log({y}) / r_log({x}))"),
                    F2::Power => format!("pas_power({x}, {y})"),
                    F2::Min => format!("pas_min({x}, {y})"),
                    F2::Max => format!("pas_max({x}, {y})"),
                }
            }
            E::OMinMax(mx, a, b) => format!("{}({}, {})", if *mx { "max" } else { "min" }, self.e(a)?, self.e(b)?),
            E::IntPower(a, n) => format!("pas_ipow({}, {})", self.e(a)?, self.i(n)?),
            E::IAbs(a) => format!("R(abs({}))", self.i(a)?),
            E::ISqr(a) => format!("pas_isqr({})", self.e(a)?),
            E::RAbs(a) => format!("abs({})", self.e(a)?),
            E::RSqr(a) => format!("pas_sqr({})", self.e(a)?),
            E::Rnd(k, w, a) => {
                let x = self.e(a)?;
                let r = match k {
                    Rnd::Round => format!("round({x})"),
                    Rnd::Trunc => format!("pas_trunc({x})"),
                    Rnd::Ceil => format!("pas_ceil({x})"),
                    Rnd::Floor => format!("pas_floor({x})"),
                };
                if *w {
                    format!("R(i32({r}))")
                } else {
                    r
                }
            }
            E::Odd(a) => format!("R({} & 1)", self.i(a)?),
            E::Sign(a) => format!("pas_sign({})", self.e(a)?),
        })
    }

    fn store(&mut self, p: Place, ty: Ty, v: String) -> Res<()> {
        let v = match (p, ty) {
            (Place::V(_), _) => v,
            (_, Ty::Single) => format!("R(f32({v}))"),
            (_, Ty::Int) => format!("R(i32({v}))"),
            (_, Ty::Bool) => format!("pas_b({v} != R(0))"),
            _ => v,
        };
        match p {
            Place::Local(i) => self.line(&format!("l{i} = {v};")),
            Place::V(k) => self.set(Field::V(k), &v)?,
            Place::Field(f) => self.set(f, &v)?,
        }
        Ok(())
    }

    fn block(&mut self, s: &S) -> Res<()> {
        self.depth += 1;
        let r = self.s(s);
        self.depth -= 1;
        r
    }

    fn s(&mut self, s: &S) -> Res<()> {
        match s {
            S::Nop => {}
            S::Seq(l) => {
                for s in l {
                    self.s(s)?;
                }
            }
            S::If(c, a, b) => {
                let c = self.cond(c)?;
                self.line(&format!("if ({c}) {{"));
                self.block(a)?;
                if !matches!(**b, S::Nop) {
                    self.line("} else {");
                    self.block(b)?;
                }
                self.line("}");
            }
            S::While(c, body) => {
                self.line("loop {");
                self.depth += 1;
                let c = self.cond(c)?;
                self.line(&format!("if (!{c}) {{ break; }}"));
                self.s(body)?;
                self.depth -= 1;
                self.line("}");
            }
            S::Repeat(body, c) => {
                self.line("loop {");
                self.block(body)?;
                // the condition's temporaries belong into the continuing block
                self.depth += 1;
                self.line("continuing {");
                self.depth += 1;
                let c = self.cond(c)?;
                self.line(&format!("break if {c};"));
                self.depth -= 1;
                self.line("}");
                self.depth -= 1;
                self.line("}");
            }
            S::For { slot, from, to, down, body } => {
                self.tmp += 1;
                let t = self.tmp;
                let (a, b) = (self.i(from)?, self.i(to)?);
                self.line("{");
                self.depth += 1;
                self.line(&format!("var i{t}: i32 = {a};"));
                self.line(&format!("let n{t}: i32 = {b};"));
                self.line("loop {");
                self.depth += 1;
                self.line(&format!("if (i{t} {} n{t}) {{ break; }}", if *down { "<" } else { ">" }));
                self.line(&format!("l{slot} = R(i{t});"));
                self.s(body)?;
                self.line(&format!("continuing {{ i{t} = i{t} {} 1; }}", if *down { "-" } else { "+" }));
                self.depth -= 1;
                self.line("}");
                self.depth -= 1;
                self.line("}");
            }
            S::Case(sel, arms, other) => {
                self.tmp += 1;
                let t = self.tmp;
                let v = self.i(sel)?;
                self.line(&format!("let s{t}: i32 = {v};"));
                let mut first = true;
                for (labels, st) in arms {
                    let test: Vec<String> = labels
                        .iter()
                        .map(|&(lo, hi)| {
                            let (lo, hi) = (lo.clamp(i32::MIN as i64, i32::MAX as i64), hi.clamp(i32::MIN as i64, i32::MAX as i64));
                            if lo == hi {
                                format!("s{t} == {lo}")
                            } else {
                                format!("(s{t} >= {lo} && s{t} <= {hi})")
                            }
                        })
                        .collect();
                    let test = if test.is_empty() { "false".to_string() } else { test.join(" || ") };
                    self.line(&format!("{}if ({test}) {{", if first { "" } else { "} else " }));
                    first = false;
                    self.block(st)?;
                }
                if let Some(o) = other {
                    if first {
                        self.line("{");
                    } else {
                        self.line("} else {");
                    }
                    self.block(o)?;
                    self.line("}");
                } else if !first {
                    self.line("}");
                }
            }
            S::Exit => self.line("return;"),
            S::Break => self.line("break;"),
            S::Continue => self.line("continue;"),
            S::Store(p, ty, v) => {
                let v = self.e(v)?;
                self.store(*p, *ty, v)?;
            }
            S::SinCos(a, (ps, ts), (pc, tc)) => {
                self.tmp += 1;
                let t = self.tmp;
                let x = self.e(a)?;
                self.line(&format!("let a{t} = {x};"));
                self.store(*ps, *ts, format!("r_sin(a{t})"))?;
                self.store(*pc, *tc, format!("r_cos(a{t})"))?;
            }
        }
        Ok(())
    }
}

/// Formula `prog` of `def` as the shader function `name`, the record cells
/// typed as `force` says where it has them (else a double is `Whole`), and
/// the words of its constants for the option values `values` (f32 bits,
/// or with `dbl` the two words of the double); or why it cannot be
/// translated.
pub fn emit(prog: &Program, def: &M3f, values: &[f64], name: &str, dbl: bool, force: &BTreeMap<u32, CellTy>) -> Result<(WgslFormula, Vec<u32>), String> {
    let mut start = vec![0.0; prog.nloc];
    for (i, v) in prog.init_values(def, values) {
        start[i] = v;
    }
    // the options come from the constants; the other untouched locals are constants
    let option: Vec<bool> = (0..prog.nloc).map(|i| prog.inits.iter().any(|(l, init)| *l == i && matches!(init, Init::Value(_) | Init::Below(_)))).collect();
    let asg = assigned_locals(&prog.tree, prog.nloc);
    let known = (0..prog.nloc).map(|i| (!asg[i] && !option[i]).then_some(start[i])).collect();
    let tree = fold(&prog.tree, known);
    let mut g = Gen { o: String::new(), depth: 1, cells: BTreeMap::new(), force, tmp: 0 };
    let mut words = Vec::new();
    for i in 0..prog.nloc {
        let init = if option[i] {
            let k = words.len();
            let v = start[i];
            if dbl {
                words.push(v.to_bits() as u32);
                words.push((v.to_bits() >> 32) as u32);
            } else {
                words.push((v as f32).to_bits());
            }
            format!("cst_r(cb + {k}u)")
        } else {
            rl(start[i])
        };
        if asg[i] || option[i] {
            g.line(&format!("var l{i}: R = {init};"));
        }
    }
    g.s(&tree)?;
    let body = std::mem::take(&mut g.o);
    let mut code = String::new();
    let _ = writeln!(code, "fn {name}(cb: u32) {{");
    // (the locals that are options but never used are not an error)
    code.push_str(&body);
    code.push_str("}\n");
    let record = g.cells.into_iter().collect();
    Ok((WgslFormula { code, record, constants: Vec::new(), const_len: words.len() as u32, writes_vars: false, dyn_var: None }, words))
}
