//! Formulas written in Pascal (`[SOURCE]` sections of `.m3f` files).
//!
//! MB3D compiles them at load time with paxCompiler (`formula/FormulaCompiler.pas`,
//! `script/CompilerUtil.pas`).  This module parses the same Delphi subset
//! into a typed tree and makes closures of it that work directly on the
//! iteration state (with the JIT, `jit_pascal` compiles the tree to machine
//! code for each scene's option values):
//!
//! * one `procedure Name(var x, y, z, w: Double; PIteration3D: TPIteration3D);`
//!   with `var` and `const` sections, locals of the ordinal and real types;
//! * statements: assignment, `begin`/`end`, `if`, `while`, `repeat`, `for`,
//!   `case` with ordinal labels, `exit`, `break`, `continue`, `Inc`/`Dec`,
//!   `SinCos`;
//! * expressions with Delphi's precedence and types (integer arithmetic
//!   wraps at 32 bits, `/` is always real, `and`/`or` short-circuit on
//!   booleans and are bitwise on integers);
//! * the math functions MB3D registers (`Register_MathFunctions`) plus the
//!   System ones (`Abs`, `Sqr`, `Round` with banker's rounding, `Trunc`, ...);
//! * `PIteration3D^.J1` etc.: fields of `TIteration3D(ext)`.
//!
//! As in MB3D's preprocessor (`PreprocessCode`), the formula's options and
//! constants become local variables: the n-th name in case-insensitive sorted
//! order is read from `PVar - 16 - 8n` (options) or `PVar + offset`
//! (constants, offsets summed over the sorted constants' sizes).

use crate::iteration::Iteration;
use crate::m3f::M3f;
use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// lexer

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Id(String), // lower case
    Int(i64),
    Real(f64),
    Str,
    Sym(&'static str),
    Eof,
}

struct Lexer<'a> {
    s: &'a [u8],
    p: usize,
    line: usize,
}

const SYMS: [&str; 20] = [":=", "<=", ">=", "<>", "..", "+", "-", "*", "/", "=", "<", ">", "(", ")", "[", "]", ",", ";", ":", "."];

impl<'a> Lexer<'a> {
    fn skip(&mut self) -> Result<(), String> {
        loop {
            while self.p < self.s.len() && (self.s[self.p] as char).is_ascii_whitespace() {
                if self.s[self.p] == b'\n' {
                    self.line += 1;
                }
                self.p += 1;
            }
            let rest = &self.s[self.p..];
            if rest.starts_with(b"//") {
                while self.p < self.s.len() && self.s[self.p] != b'\n' {
                    self.p += 1;
                }
            } else if rest.starts_with(b"{") {
                let start = self.line;
                while self.p < self.s.len() && self.s[self.p] != b'}' {
                    if self.s[self.p] == b'\n' {
                        self.line += 1;
                    }
                    self.p += 1;
                }
                if self.p >= self.s.len() {
                    return Err(format!("line {start}: unterminated comment"));
                }
                self.p += 1;
            } else if rest.starts_with(b"(*") {
                let start = self.line;
                self.p += 2;
                while self.p + 1 < self.s.len() && !(self.s[self.p] == b'*' && self.s[self.p + 1] == b')') {
                    if self.s[self.p] == b'\n' {
                        self.line += 1;
                    }
                    self.p += 1;
                }
                if self.p + 1 >= self.s.len() {
                    return Err(format!("line {start}: unterminated comment"));
                }
                self.p += 2;
            } else {
                return Ok(());
            }
        }
    }

    fn next(&mut self) -> Result<(Tok, usize), String> {
        self.skip()?;
        let line = self.line;
        if self.p >= self.s.len() {
            return Ok((Tok::Eof, line));
        }
        let c = self.s[self.p] as char;
        if c.is_ascii_alphabetic() || c == '_' {
            let st = self.p;
            while self.p < self.s.len() && ((self.s[self.p] as char).is_ascii_alphanumeric() || self.s[self.p] == b'_') {
                self.p += 1;
            }
            let id = String::from_utf8_lossy(&self.s[st..self.p]).to_ascii_lowercase();
            return Ok((Tok::Id(id), line));
        }
        if c.is_ascii_digit() {
            let st = self.p;
            while self.p < self.s.len() && self.s[self.p].is_ascii_digit() {
                self.p += 1;
            }
            let mut real = false;
            // a '.' followed by a digit (not '..' of a range)
            if self.p + 1 < self.s.len() && self.s[self.p] == b'.' && self.s[self.p + 1].is_ascii_digit() {
                real = true;
                self.p += 1;
                while self.p < self.s.len() && self.s[self.p].is_ascii_digit() {
                    self.p += 1;
                }
            } else if self.p + 1 < self.s.len() && self.s[self.p] == b'.' && !matches!(self.s[self.p + 1], b'.' | b')') {
                // "2." is a real in Delphi
                real = true;
                self.p += 1;
            }
            if self.p < self.s.len() && (self.s[self.p] == b'e' || self.s[self.p] == b'E') {
                let save = self.p;
                self.p += 1;
                if self.p < self.s.len() && (self.s[self.p] == b'+' || self.s[self.p] == b'-') {
                    self.p += 1;
                }
                if self.p < self.s.len() && self.s[self.p].is_ascii_digit() {
                    real = true;
                    while self.p < self.s.len() && self.s[self.p].is_ascii_digit() {
                        self.p += 1;
                    }
                } else {
                    self.p = save;
                }
            }
            let t = std::str::from_utf8(&self.s[st..self.p]).unwrap();
            return if real {
                t.parse::<f64>().map(|v| (Tok::Real(v), line)).map_err(|_| format!("line {line}: bad number {t}"))
            } else {
                match t.parse::<i64>() {
                    Ok(v) => Ok((Tok::Int(v), line)),
                    Err(_) => t.parse::<f64>().map(|v| (Tok::Real(v), line)).map_err(|_| format!("line {line}: bad number {t}")),
                }
            };
        }
        if c == '$' {
            self.p += 1;
            let st = self.p;
            while self.p < self.s.len() && self.s[self.p].is_ascii_hexdigit() {
                self.p += 1;
            }
            let t = std::str::from_utf8(&self.s[st..self.p]).unwrap();
            return i64::from_str_radix(t, 16).map(|v| (Tok::Int(v), line)).map_err(|_| format!("line {line}: bad hex number"));
        }
        if c == '\'' {
            self.p += 1;
            loop {
                if self.p >= self.s.len() {
                    return Err(format!("line {line}: unterminated string"));
                }
                if self.s[self.p] == b'\'' {
                    if self.p + 1 < self.s.len() && self.s[self.p + 1] == b'\'' {
                        self.p += 2;
                        continue;
                    }
                    self.p += 1;
                    break;
                }
                self.p += 1;
            }
            return Ok((Tok::Str, line));
        }
        if c == '^' || c == '@' {
            self.p += 1;
            return Ok((Tok::Sym(if c == '^' { "^" } else { "@" }), line));
        }
        for s in SYMS {
            if self.s[self.p..].starts_with(s.as_bytes()) {
                self.p += s.len();
                return Ok((Tok::Sym(s), line));
            }
        }
        Err(format!("line {line}: unexpected character '{c}'"))
    }
}

fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>, String> {
    let mut lx = Lexer { s: src.as_bytes(), p: 0, line: 1 };
    let mut out = Vec::new();
    loop {
        let t = lx.next()?;
        let end = t.0 == Tok::Eof;
        out.push(t);
        if end {
            return Ok(out);
        }
    }
}

// ---------------------------------------------------------------------------
// types and the tree of a formula
//
// The parser builds a typed tree (`E`, `S`); the closures run it (`lower_e`,
// `lower_s`), `jit_pascal` compiles it with Cranelift and `jit_wgsl`
// translates it for the graphics card.  Every value is an f64 as in the
// closures: ordinals are whole numbers (Integer results wrapped to 32 bits),
// booleans 0 or 1; the operations say exactly how they treat them, so that
// the code generators give the closures' results bit for bit.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ty {
    Int,   // 32 bit (Integer, Cardinal, Word, Byte, ...)
    Int64, // Int64 (results of Round and Trunc)
    Real,
    Single,
    Bool,
}

impl Ty {
    fn ordinal(self) -> bool {
        matches!(self, Ty::Int | Ty::Int64)
    }
    fn real(self) -> bool {
        matches!(self, Ty::Real | Ty::Single)
    }
    fn numeric(self) -> bool {
        self.ordinal() || self.real()
    }
    fn name(self) -> &'static str {
        match self {
            Ty::Int => "Integer",
            Ty::Int64 => "Int64",
            Ty::Real => "Double",
            Ty::Single => "Single",
            Ty::Bool => "Boolean",
        }
    }
}

/// Run-time state of one call: the locals and the iteration record.
pub struct Env<'a> {
    l: &'a mut [f64],
    it: &'a mut Iteration,
}

const NORMAL: u8 = 0;
const BREAK: u8 = 1;
const CONTINUE: u8 = 2;
const EXIT: u8 = 3;

type Ex = Box<dyn Fn(&mut Env) -> f64 + Send + Sync>;
type St = Box<dyn Fn(&mut Env) -> u8 + Send + Sync>;

#[inline]
pub(crate) fn wrap32(v: i64) -> f64 {
    v as i32 as f64
}

/// Fields of `TIteration3D` / `TIteration3Dext` the formulas can use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    C(usize),
    J(usize),
    Ju(usize),
    V(usize),
    Rout,
    Rold,
    OTrap,
    VaryScale,
    Deriv(usize),
    Dfree(usize),
    RStop,
    MaxIt,
    ItResult,
    FirstIt,
    DEoption,
    SmoothIt,
    DoJulia,
    CalcSit,
}

fn field_by_name(n: &str) -> Option<(Field, Ty)> {
    use Field::*;
    Some(match n {
        "c1" => (C(0), Ty::Real),
        "c2" => (C(1), Ty::Real),
        "c3" => (C(2), Ty::Real),
        "j1" => (J(0), Ty::Real),
        "j2" => (J(1), Ty::Real),
        "j3" => (J(2), Ty::Real),
        "j4" => (J(3), Ty::Real),
        "ju1" => (Ju(0), Ty::Real),
        "ju2" => (Ju(1), Ty::Real),
        "ju3" => (Ju(2), Ty::Real),
        "ju4" => (Ju(3), Ty::Real),
        "x" => (V(0), Ty::Real),
        "y" => (V(1), Ty::Real),
        "z" => (V(2), Ty::Real),
        "w" => (V(3), Ty::Real),
        "rout" => (Rout, Ty::Real),
        "rold" => (Rold, Ty::Real),
        "otrap" => (OTrap, Ty::Real),
        "varyscale" => (VaryScale, Ty::Real),
        "deriv1" => (Deriv(0), Ty::Real),
        "deriv2" => (Deriv(1), Ty::Real),
        "deriv3" => (Deriv(2), Ty::Real),
        "dfree1" => (Dfree(0), Ty::Real),
        "dfree2" => (Dfree(1), Ty::Real),
        "rstop" | "rstopd" => (RStop, Ty::Real),
        "maxit" => (MaxIt, Ty::Int),
        "itresulti" => (ItResult, Ty::Int),
        "bfirstit" => (FirstIt, Ty::Int),
        "deoption" => (DEoption, Ty::Int),
        "smoothitd" => (SmoothIt, Ty::Single),
        "dojulia" => (DoJulia, Ty::Bool),
        "calcsit" => (CalcSit, Ty::Bool),
        _ => return None,
    })
}

#[inline]
pub(crate) fn get_field(it: &Iteration, f: Field) -> f64 {
    use Field::*;
    match f {
        C(k) => it.c[k],
        J(k) => it.j[k],
        Ju(k) => it.ju[k],
        V(k) => it.v[k],
        Rout => it.rout,
        Rold => it.rold,
        OTrap => it.otrap,
        VaryScale => it.vary_scale,
        Deriv(0) => it.deriv1,
        Deriv(1) => it.deriv2,
        Deriv(_) => it.deriv3,
        Dfree(k) => it.dfree[k],
        RStop => it.rstop,
        MaxIt => it.max_it as f64,
        ItResult => it.it_result as f64,
        FirstIt => it.first_it as f64,
        DEoption => it.de_option as f64,
        SmoothIt => it.smooth_it as f64,
        DoJulia => it.do_julia as i32 as f64,
        CalcSit => it.calc_sit as i32 as f64,
    }
}

#[inline]
pub(crate) fn set_field(it: &mut Iteration, f: Field, v: f64) {
    use Field::*;
    match f {
        C(k) => it.c[k] = v,
        J(k) => it.j[k] = v,
        Ju(k) => it.ju[k] = v,
        V(k) => it.v[k] = v,
        Rout => it.rout = v,
        Rold => it.rold = v,
        OTrap => it.otrap = v,
        VaryScale => it.vary_scale = v,
        Deriv(0) => it.deriv1 = v,
        Deriv(1) => it.deriv2 = v,
        Deriv(_) => it.deriv3 = v,
        Dfree(k) => it.dfree[k] = v,
        RStop => it.rstop = v,
        MaxIt => it.max_it = v as i32,
        ItResult => it.it_result = v as i32,
        FirstIt => it.first_it = v as i32,
        DEoption => it.de_option = v as i32,
        SmoothIt => it.smooth_it = v as f32,
        DoJulia => it.do_julia = v != 0.0,
        CalcSit => it.calc_sit = v != 0.0,
    }
}

/// Storage place of a variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    Local(usize),
    /// one of the `var x, y, z, w` parameters
    V(usize),
    Field(Field),
}

#[derive(Clone, Copy, Debug)]
struct Var {
    place: Place,
    ty: Ty,
    constant: bool,
}

/// Where the initial value of a local comes from.
#[derive(Clone, Debug)]
pub(crate) enum Init {
    /// option value number n (a Double option at the preprocessor's offset)
    Value(usize),
    /// 8 bytes of the variable buffer below `PVar` (offset in bytes)
    Below(usize),
    /// a value from the `[CONSTANTS]` section or a `var` initializer
    Const(f64),
}

/// The math functions of one real argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum F1 {
    Sin,
    Cos,
    Tan,
    Cot,
    Sec,
    Csc,
    ArcTan,
    ArcSin,
    ArcCos,
    ArcCot,
    ArcSec,
    ArcCsc,
    Sinh,
    Cosh,
    Tanh,
    Coth,
    Sech,
    Csch,
    ArcSinh,
    ArcCosh,
    ArcTanh,
    ArcCoth,
    ArcSech,
    ArcCsch,
    Sqrt,
    Exp,
    Ln,
    LnXp1,
    Log10,
    Log2,
    Int,
    Frac,
    RadToDeg,
    DegToRad,
    DegNormalize,
}

/// Every [`F1`], in the order of its number (`F1::ALL[f as usize] == f`).
#[cfg_attr(not(jit), allow(dead_code))]
pub(crate) const F1_ALL: [F1; 35] = {
    use F1::*;
    [
        Sin, Cos, Tan, Cot, Sec, Csc, ArcTan, ArcSin, ArcCos, ArcCot, ArcSec, ArcCsc, Sinh, Cosh, Tanh, Coth, Sech, Csch, ArcSinh, ArcCosh, ArcTanh,
        ArcCoth, ArcSech, ArcCsch, Sqrt, Exp, Ln, LnXp1, Log10, Log2, Int, Frac, RadToDeg, DegToRad, DegNormalize,
    ]
};

impl F1 {
    fn by_name(n: &str) -> Option<F1> {
        use F1::*;
        Some(match n {
            "sin" => Sin,
            "cos" => Cos,
            "tan" => Tan,
            "cotan" | "cot" => Cot,
            "secant" | "sec" => Sec,
            "cosecant" | "csc" => Csc,
            "arctan" => ArcTan,
            "arcsin" => ArcSin,
            "arccos" => ArcCos,
            "arccot" => ArcCot,
            "arcsec" => ArcSec,
            "arccsc" => ArcCsc,
            "sinh" => Sinh,
            "cosh" => Cosh,
            "tanh" => Tanh,
            "coth" => Coth,
            "sech" => Sech,
            "csch" => Csch,
            "arcsinh" => ArcSinh,
            "arccosh" => ArcCosh,
            "arctanh" => ArcTanh,
            "arccoth" => ArcCoth,
            "arcsech" => ArcSech,
            "arccsch" => ArcCsch,
            "sqrt" => Sqrt,
            "exp" => Exp,
            "ln" => Ln,
            "lnxp1" => LnXp1,
            "log10" => Log10,
            "log2" => Log2,
            "int" => Int,
            "frac" => Frac,
            "radtodeg" => RadToDeg,
            "degtorad" => DegToRad,
            "degnormalize" => DegNormalize,
            _ => return None,
        })
    }

    #[inline]
    pub(crate) fn eval(self, x: f64) -> f64 {
        use F1::*;
        match self {
            Sin => x.sin(),
            Cos => x.cos(),
            Tan => x.tan(),
            Cot => 1.0 / x.tan(),
            Sec => 1.0 / x.cos(),
            Csc => 1.0 / x.sin(),
            ArcTan => x.atan(),
            ArcSin => x.asin(),
            ArcCos => x.acos(),
            ArcCot => (1.0 / x).atan(),
            ArcSec => (1.0 / x).acos(),
            ArcCsc => (1.0 / x).asin(),
            Sinh => x.sinh(),
            Cosh => x.cosh(),
            Tanh => x.tanh(),
            Coth => 1.0 / x.tanh(),
            Sech => 1.0 / x.cosh(),
            Csch => 1.0 / x.sinh(),
            ArcSinh => x.asinh(),
            ArcCosh => x.acosh(),
            ArcTanh => x.atanh(),
            ArcCoth => (1.0 / x).atanh(),
            ArcSech => (1.0 / x).acosh(),
            ArcCsch => (1.0 / x).asinh(),
            Sqrt => x.sqrt(),
            Exp => x.exp(),
            Ln => x.ln(),
            LnXp1 => x.ln_1p(),
            Log10 => x.log10(),
            Log2 => x.log2(),
            Int => x.trunc(),
            Frac => x.fract(),
            RadToDeg => x.to_degrees(),
            DegToRad => x.to_radians(),
            DegNormalize => x - (x / 360.0).floor() * 360.0,
        }
    }
}

/// The math functions of two real arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum F2 {
    ArcTan2,
    Hypot,
    LogN,
    Power,
    /// Math.Min: "if A < B then Result := A else Result := B"
    Min,
    /// Math.Max: "if A > B then Result := A else Result := B"
    Max,
}

#[cfg_attr(not(jit), allow(dead_code))]
pub(crate) const F2_ALL: [F2; 6] = [F2::ArcTan2, F2::Hypot, F2::LogN, F2::Power, F2::Min, F2::Max];

impl F2 {
    #[inline]
    pub(crate) fn eval(self, a: f64, b: f64) -> f64 {
        match self {
            F2::ArcTan2 => a.atan2(b),
            F2::Hypot => a.hypot(b),
            F2::LogN => b.ln() / a.ln(),
            F2::Power => power(a, b),
            F2::Min => {
                if a < b {
                    a
                } else {
                    b
                }
            }
            F2::Max => {
                if a > b {
                    a
                } else {
                    b
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cmp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// Real arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ROp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Ordinal arithmetic on `x as i64`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IOp {
    Add,
    Sub,
    Mul,
    /// `div` (0 for a zero divisor)
    Div,
    /// `mod` (0 for a zero divisor)
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rnd {
    /// to nearest, ties to even (x87 FISTP)
    Round,
    Trunc,
    Ceil,
    Floor,
}

/// An expression.
#[derive(Clone, Debug)]
pub(crate) enum E {
    Lit(f64),
    Load(Place),
    /// `-a`; `wrap`: an Integer (`wrap32(-(a as i64))`)
    Neg(Box<E>, bool),
    /// `not` of a Boolean
    NotB(Box<E>),
    /// `not` of an Integer (`!(a as i64 as i32)`) or, `wide`, an Int64
    NotI(Box<E>, bool),
    /// a comparison of the two f64 values (0 or 1)
    Cmp(Cmp, Box<E>, Box<E>),
    R(ROp, Box<E>, Box<E>),
    /// ordinal operation; `wrap`: the result is an Integer (wrap32)
    I(IOp, bool, Box<E>, Box<E>),
    /// Boolean and / or (short-circuit), xor
    And(Box<E>, Box<E>),
    Or(Box<E>, Box<E>),
    Xor(Box<E>, Box<E>),
    F1(F1, Box<E>),
    F2(F2, Box<E>, Box<E>),
    /// Min / Max of ordinals (`f64::min` / `f64::max`)
    OMinMax(bool, Box<E>, Box<E>),
    /// IntPower(a, b as i32)
    IntPower(Box<E>, Box<E>),
    /// Abs / Sqr of an Integer (wrapped)
    IAbs(Box<E>),
    ISqr(Box<E>),
    /// Abs / Sqr of a real or an Int64
    RAbs(Box<E>),
    RSqr(Box<E>),
    /// Round, Trunc, Ceil, Floor of a real; `wrap`: an Integer (Ceil, Floor)
    Rnd(Rnd, bool, Box<E>),
    Odd(Box<E>),
    IsNan(Box<E>),
    IsInf(Box<E>),
    Sign(Box<E>),
}

/// A statement.
#[derive(Clone, Debug)]
pub(crate) enum S {
    Nop,
    Seq(Vec<S>),
    If(E, Box<S>, Box<S>),
    While(E, Box<S>),
    Repeat(Box<S>, E),
    /// `for` over `from as i64 ..= to as i64` (the counter stored as is)
    For { slot: usize, from: E, to: E, down: bool, body: Box<S> },
    /// the selector as i64 against the label ranges
    Case(E, Vec<(Vec<(i64, i64)>, S)>, Option<Box<S>>),
    Exit,
    Break,
    Continue,
    /// the value stored as the type of the place (`store_value`), into
    /// x, y, z, w as it is
    Store(Place, Ty, E),
    /// SinCos(a, s, c)
    SinCos(E, (Place, Ty), (Place, Ty)),
}

fn bx(e: E) -> Box<E> {
    Box::new(e)
}

// ---------------------------------------------------------------------------
// parser / compiler

struct Compiler {
    toks: Vec<(Tok, usize)>,
    p: usize,
    vars: HashMap<String, Var>,
    consts: HashMap<String, (f64, Ty)>,
    nloc: usize,
    inits: Vec<(usize, Init)>,
    loc_ty: Vec<Ty>,
    ptr_name: String,
    loop_depth: usize,
    /// parsing a constant expression: variables are not allowed
    in_const: bool,
}

fn err<T>(line: usize, m: impl fmt::Display) -> Result<T, String> {
    Err(format!("line {line}: {m}"))
}

impl Compiler {
    fn peek(&self) -> &Tok {
        &self.toks[self.p].0
    }
    fn line(&self) -> usize {
        self.toks[self.p].1
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.p].0.clone();
        if self.p + 1 < self.toks.len() {
            self.p += 1;
        }
        t
    }
    fn is_sym(&self, s: &str) -> bool {
        matches!(self.peek(), Tok::Sym(x) if *x == s)
    }
    fn is_kw(&self, s: &str) -> bool {
        matches!(self.peek(), Tok::Id(x) if x == s)
    }
    fn eat_sym(&mut self, s: &str) -> bool {
        if self.is_sym(s) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, s: &str) -> bool {
        if self.is_kw(s) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn expect_sym(&mut self, s: &str) -> Result<(), String> {
        if self.eat_sym(s) {
            Ok(())
        } else {
            err(self.line(), format!("'{s}' expected but {} found", self.desc()))
        }
    }
    fn expect_kw(&mut self, s: &str) -> Result<(), String> {
        if self.eat_kw(s) {
            Ok(())
        } else {
            err(self.line(), format!("'{s}' expected but {} found", self.desc()))
        }
    }
    fn desc(&self) -> String {
        match self.peek() {
            Tok::Id(s) => format!("'{s}'"),
            Tok::Int(v) => format!("'{v}'"),
            Tok::Real(v) => format!("'{v}'"),
            Tok::Str => "a string".into(),
            Tok::Sym(s) => format!("'{s}'"),
            Tok::Eof => "end of file".into(),
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.bump() {
            Tok::Id(s) if !is_reserved(&s) => Ok(s),
            _ => {
                self.p -= 1;
                err(self.line(), format!("identifier expected but {} found", self.desc()))
            }
        }
    }

    fn type_name(&mut self) -> Result<Ty, String> {
        let line = self.line();
        let t = self.ident()?;
        Ok(match t.as_str() {
            "double" | "real" | "extended" | "real48" | "comp" | "currency" => Ty::Real,
            "single" => Ty::Single,
            "integer" | "longint" | "cardinal" | "longword" | "word" | "byte" | "shortint" | "smallint" | "dword" | "nativeint"
            | "nativeuint" => Ty::Int,
            "int64" | "uint64" => Ty::Int64,
            "boolean" | "longbool" | "bytebool" | "wordbool" => Ty::Bool,
            _ => return err(line, format!("type '{t}' is not supported")),
        })
    }

    fn new_local(&mut self, ty: Ty) -> usize {
        self.nloc += 1;
        self.loc_ty.push(ty);
        self.nloc - 1
    }

    fn declare(&mut self, name: &str, v: Var, line: usize) -> Result<(), String> {
        if self.vars.contains_key(name) || self.consts.contains_key(name) {
            return err(line, format!("identifier redeclared: '{name}'"));
        }
        self.vars.insert(name.to_string(), v);
        Ok(())
    }

    // ---- declarations

    fn header(&mut self) -> Result<String, String> {
        // optional "unit"/"uses" are not part of MB3D's formulas
        let line = self.line();
        if !self.eat_kw("procedure") {
            return err(line, "no formula found; should be something like <procedure MyFormula(var x, y, z, w: Double; PIteration3D: TPIteration3D);>");
        }
        let name = self.ident()?;
        self.expect_sym("(")?;
        let mut vparams = Vec::new();
        loop {
            let line = self.line();
            let is_var = self.eat_kw("var");
            if !is_var {
                self.eat_kw("const");
            }
            let mut names = vec![self.ident()?];
            while self.eat_sym(",") {
                names.push(self.ident()?);
            }
            self.expect_sym(":")?;
            let tl = self.line();
            let t = self.ident()?;
            match t.as_str() {
                "tpiteration3d" | "tpiteration3dext" => {
                    if names.len() != 1 || is_var {
                        return err(line, "one PIteration3D parameter expected");
                    }
                    self.ptr_name = names[0].clone();
                }
                "double" => {
                    if !is_var {
                        return err(line, "the coordinates must be var parameters");
                    }
                    vparams.extend(names);
                }
                _ => return err(tl, format!("unexpected parameter type '{t}'")),
            }
            if !self.eat_sym(";") {
                break;
            }
        }
        self.expect_sym(")")?;
        self.expect_sym(";")?;
        if vparams.len() != 4 {
            return err(line, "the formula needs the parameters (var x, y, z, w: Double; PIteration3D: TPIteration3D)");
        }
        if self.ptr_name.is_empty() {
            return err(line, "PIteration3D parameter missing");
        }
        for (k, n) in vparams.iter().enumerate() {
            self.declare(n, Var { place: Place::V(k), ty: Ty::Real, constant: false }, line)?;
        }
        Ok(name)
    }

    fn decls(&mut self) -> Result<(), String> {
        loop {
            if self.eat_kw("var") {
                while let Tok::Id(s) = self.peek() {
                    if is_reserved(s) {
                        break;
                    }
                    let line = self.line();
                    let mut names = vec![self.ident()?];
                    while self.eat_sym(",") {
                        names.push(self.ident()?);
                    }
                    self.expect_sym(":")?;
                    let ty = self.type_name()?;
                    let init = if self.eat_sym("=") { Some(self.const_expr()?) } else { None };
                    self.expect_sym(";")?;
                    for n in names {
                        let l = self.new_local(ty);
                        if let Some((v, _)) = init {
                            self.inits.push((l, Init::Const(v)));
                        }
                        self.declare(&n, Var { place: Place::Local(l), ty, constant: false }, line)?;
                    }
                }
            } else if self.eat_kw("const") {
                while let Tok::Id(s) = self.peek() {
                    if is_reserved(s) {
                        break;
                    }
                    let line = self.line();
                    let n = self.ident()?;
                    let ty = if self.eat_sym(":") { Some(self.type_name()?) } else { None };
                    self.expect_sym("=")?;
                    let (v, t) = self.const_expr()?;
                    self.expect_sym(";")?;
                    let ty = ty.unwrap_or(t);
                    if self.vars.contains_key(&n) || self.consts.contains_key(&n) {
                        return err(line, format!("identifier redeclared: '{n}'"));
                    }
                    self.consts.insert(n, (v, ty));
                }
            } else {
                return Ok(());
            }
        }
    }

    /// A constant expression (evaluated at compile time).
    fn const_expr(&mut self) -> Result<(f64, Ty), String> {
        let line = self.line();
        self.in_const = true;
        let r = self.expr();
        self.in_const = false;
        let (e, ty) = r?;
        // constant expressions only use literals and constants: evaluate on a dummy state
        let v = eval_const(&e);
        if !v.is_finite() && !ty.real() {
            return err(line, "constant expression expected");
        }
        Ok((v, ty))
    }

    // ---- statements

    fn block(&mut self) -> Result<S, String> {
        self.expect_kw("begin")?;
        let mut list: Vec<S> = Vec::new();
        loop {
            if self.eat_kw("end") {
                break;
            }
            if self.eat_sym(";") {
                continue;
            }
            list.push(self.statement()?);
            if self.eat_kw("end") {
                break;
            }
            if !self.is_sym(";") {
                return err(self.line(), format!("';' expected but {} found", self.desc()));
            }
        }
        Ok(S::Seq(list))
    }

    fn statement(&mut self) -> Result<S, String> {
        let line = self.line();
        match self.peek().clone() {
            Tok::Id(k) => match k.as_str() {
                "begin" => self.block(),
                "if" => {
                    self.bump();
                    let c = self.condition()?;
                    self.expect_kw("then")?;
                    let a = if self.is_kw("else") || self.is_sym(";") { S::Nop } else { self.statement()? };
                    let b = if self.eat_kw("else") { if self.is_kw("end") || self.is_sym(";") { S::Nop } else { self.statement()? } } else { S::Nop };
                    Ok(S::If(c, Box::new(a), Box::new(b)))
                }
                "while" => {
                    self.bump();
                    let c = self.condition()?;
                    self.expect_kw("do")?;
                    self.loop_depth += 1;
                    let body = self.statement()?;
                    self.loop_depth -= 1;
                    Ok(S::While(c, Box::new(body)))
                }
                "repeat" => {
                    self.bump();
                    self.loop_depth += 1;
                    let mut list = Vec::new();
                    while !self.is_kw("until") {
                        if self.eat_sym(";") {
                            continue;
                        }
                        list.push(self.statement()?);
                        if !self.is_kw("until") {
                            self.expect_sym(";")?;
                        }
                    }
                    self.loop_depth -= 1;
                    self.expect_kw("until")?;
                    let c = self.condition()?;
                    Ok(S::Repeat(Box::new(S::Seq(list)), c))
                }
                "for" => {
                    self.bump();
                    let vl = self.line();
                    let n = self.ident()?;
                    let v = match self.vars.get(&n) {
                        Some(v) if v.ty.ordinal() => *v,
                        Some(_) => return err(vl, "for loop control variable must be of an ordinal type"),
                        None => return err(vl, format!("undeclared identifier: '{n}'")),
                    };
                    let Place::Local(slot) = v.place else {
                        return err(vl, "for loop control variable must be a local variable");
                    };
                    self.expect_sym(":=")?;
                    let (a, ta) = self.expr()?;
                    let down = if self.eat_kw("to") {
                        false
                    } else if self.eat_kw("downto") {
                        true
                    } else {
                        return err(self.line(), "'to' or 'downto' expected");
                    };
                    let (b, tb) = self.expr()?;
                    if !ta.ordinal() || !tb.ordinal() {
                        return err(vl, "ordinal bounds expected");
                    }
                    self.expect_kw("do")?;
                    self.loop_depth += 1;
                    let body = self.statement()?;
                    self.loop_depth -= 1;
                    Ok(S::For { slot, from: a, to: b, down, body: Box::new(body) })
                }
                "case" => {
                    self.bump();
                    let (sel, ts) = self.expr()?;
                    if !ts.ordinal() {
                        return err(line, "ordinal type required");
                    }
                    self.expect_kw("of")?;
                    let mut arms: Vec<(Vec<(i64, i64)>, S)> = Vec::new();
                    let mut other: Option<Box<S>> = None;
                    loop {
                        if self.eat_kw("end") {
                            break;
                        }
                        if self.eat_kw("else") || self.eat_kw("otherwise") {
                            let mut list = Vec::new();
                            while !self.is_kw("end") {
                                if self.eat_sym(";") {
                                    continue;
                                }
                                list.push(self.statement()?);
                            }
                            self.expect_kw("end")?;
                            other = Some(Box::new(S::Seq(list)));
                            break;
                        }
                        let mut labels = Vec::new();
                        loop {
                            let (lo, _) = self.const_expr()?;
                            let hi = if self.eat_sym("..") { self.const_expr()?.0 } else { lo };
                            labels.push((lo as i64, hi as i64));
                            if !self.eat_sym(",") {
                                break;
                            }
                        }
                        self.expect_sym(":")?;
                        let st = if self.is_sym(";") { S::Nop } else { self.statement()? };
                        arms.push((labels, st));
                        if !self.eat_sym(";") && !self.is_kw("end") && !self.is_kw("else") {
                            return err(self.line(), format!("';' expected but {} found", self.desc()));
                        }
                    }
                    Ok(S::Case(sel, arms, other))
                }
                "exit" => {
                    self.bump();
                    if self.eat_sym("(") {
                        self.expect_sym(")")?;
                    }
                    Ok(S::Exit)
                }
                "break" | "continue" => {
                    self.bump();
                    if self.loop_depth == 0 {
                        return err(line, format!("{k} outside of a loop"));
                    }
                    Ok(if k == "break" { S::Break } else { S::Continue })
                }
                "inc" | "dec" => {
                    self.bump();
                    self.expect_sym("(")?;
                    let (place, ty) = self.lvalue()?;
                    if !ty.ordinal() {
                        return err(line, "ordinal type required");
                    }
                    let d = if self.eat_sym(",") {
                        let (d, td) = self.expr()?;
                        if !td.ordinal() {
                            return err(line, "ordinal type required");
                        }
                        d
                    } else {
                        E::Lit(1.0)
                    };
                    self.expect_sym(")")?;
                    let op = if k == "inc" { IOp::Add } else { IOp::Sub };
                    Ok(S::Store(place, ty, E::I(op, false, bx(E::Load(place)), bx(d))))
                }
                "sincos" => {
                    self.bump();
                    self.expect_sym("(")?;
                    let a = self.real_arg()?;
                    self.expect_sym(",")?;
                    let (ps, ts) = self.lvalue()?;
                    self.expect_sym(",")?;
                    let (pc, tc) = self.lvalue()?;
                    self.expect_sym(")")?;
                    if !ts.real() || !tc.real() {
                        return err(line, "SinCos needs real variables");
                    }
                    Ok(S::SinCos(a, (ps, ts), (pc, tc)))
                }
                _ => {
                    // assignment (or a call of a function whose result is dropped)
                    let (place, ty) = self.lvalue()?;
                    let al = self.line();
                    if !self.eat_sym(":=") {
                        return err(al, format!("':=' expected but {} found", self.desc()));
                    }
                    let (v, tv) = self.expr()?;
                    let v = coerce(v, tv, ty, al)?;
                    Ok(S::Store(place, ty, v))
                }
            },
            _ => err(line, format!("statement expected but {} found", self.desc())),
        }
    }

    fn real_arg(&mut self) -> Result<E, String> {
        let l = self.line();
        let (e, t) = self.expr()?;
        coerce(e, t, Ty::Real, l)
    }

    fn condition(&mut self) -> Result<E, String> {
        let l = self.line();
        let (c, t) = self.expr()?;
        if t != Ty::Bool {
            return err(l, format!("type of expression must be BOOLEAN, not {}", t.name()));
        }
        Ok(c)
    }

    /// An assignable variable.
    fn lvalue(&mut self) -> Result<(Place, Ty), String> {
        let line = self.line();
        let n = self.ident()?;
        if n == self.ptr_name {
            let (f, t) = self.field()?;
            return Ok((Place::Field(f), t));
        }
        match self.vars.get(&n) {
            Some(v) if !v.constant => Ok((v.place, v.ty)),
            Some(_) => err(line, format!("'{n}' cannot be assigned to")),
            None if self.consts.contains_key(&n) => err(line, format!("left side cannot be assigned to: '{n}'")),
            None => err(line, format!("undeclared identifier: '{n}'")),
        }
    }

    /// `^.Field` or `.Field` after the PIteration3D name.
    fn field(&mut self) -> Result<(Field, Ty), String> {
        self.eat_sym("^");
        self.expect_sym(".")?;
        let line = self.line();
        let f = self.ident()?;
        match field_by_name(&f) {
            Some(x) => Ok(x),
            None => err(line, format!("field '{f}' of TIteration3D is not supported")),
        }
    }

    // ---- expressions

    fn expr(&mut self) -> Result<(E, Ty), String> {
        let (a, ta) = self.simple()?;
        let op = match self.peek() {
            Tok::Sym(s @ ("=" | "<>" | "<" | ">" | "<=" | ">=")) => *s,
            _ => return Ok((a, ta)),
        };
        let line = self.line();
        self.bump();
        let (b, tb) = self.simple()?;
        if !((ta.numeric() && tb.numeric()) || (ta == Ty::Bool && tb == Ty::Bool)) {
            return err(line, format!("incompatible types: '{}' and '{}'", ta.name(), tb.name()));
        }
        let c = match op {
            "=" => Cmp::Eq,
            "<>" => Cmp::Ne,
            "<" => Cmp::Lt,
            ">" => Cmp::Gt,
            "<=" => Cmp::Le,
            _ => Cmp::Ge,
        };
        Ok((E::Cmp(c, bx(a), bx(b)), Ty::Bool))
    }

    fn simple(&mut self) -> Result<(E, Ty), String> {
        let (mut a, mut ta) = self.term()?;
        loop {
            let op = match self.peek() {
                Tok::Sym(s @ ("+" | "-")) => s.to_string(),
                Tok::Id(s) if s == "or" || s == "xor" => s.clone(),
                _ => return Ok((a, ta)),
            };
            let line = self.line();
            self.bump();
            let (b, tb) = self.term()?;
            (a, ta) = binary(&op, a, ta, b, tb, line)?;
        }
    }

    fn term(&mut self) -> Result<(E, Ty), String> {
        let (mut a, mut ta) = self.factor()?;
        loop {
            let op = match self.peek() {
                Tok::Sym(s @ ("*" | "/")) => s.to_string(),
                Tok::Id(s) if matches!(s.as_str(), "div" | "mod" | "and" | "shl" | "shr") => s.clone(),
                _ => return Ok((a, ta)),
            };
            let line = self.line();
            self.bump();
            let (b, tb) = self.factor()?;
            (a, ta) = binary(&op, a, ta, b, tb, line)?;
        }
    }

    fn factor(&mut self) -> Result<(E, Ty), String> {
        let line = self.line();
        match self.bump() {
            Tok::Int(v) => {
                let ty = if v > i32::MAX as i64 || v < i32::MIN as i64 { Ty::Int64 } else { Ty::Int };
                Ok((E::Lit(v as f64), ty))
            }
            Tok::Real(v) => Ok((E::Lit(v), Ty::Real)),
            Tok::Sym("(") => {
                let r = self.expr()?;
                self.expect_sym(")")?;
                Ok(r)
            }
            Tok::Sym("-") => {
                let (a, t) = self.factor()?;
                if !t.numeric() {
                    return err(line, "numeric operand expected");
                }
                Ok((E::Neg(bx(a), t == Ty::Int), t))
            }
            Tok::Sym("+") => self.factor(),
            Tok::Id(k) if k == "not" => {
                let (a, t) = self.factor()?;
                match t {
                    Ty::Bool => Ok((E::NotB(bx(a)), t)),
                    Ty::Int => Ok((E::NotI(bx(a), false), t)),
                    Ty::Int64 => Ok((E::NotI(bx(a), true), t)),
                    _ => err(line, "operator not applicable to this operand type"),
                }
            }
            Tok::Id(k) if k == "true" || k == "false" => Ok((E::Lit((k == "true") as i32 as f64), Ty::Bool)),
            Tok::Id(k) if k == "pi" && !self.vars.contains_key("pi") => {
                if self.eat_sym("(") {
                    self.expect_sym(")")?;
                }
                Ok((E::Lit(std::f64::consts::PI), Ty::Real))
            }
            Tok::Id(n) => {
                if self.in_const && (n == self.ptr_name || self.vars.contains_key(&n)) {
                    return err(line, "constant expression expected");
                }
                if n == self.ptr_name {
                    let (f, t) = self.field()?;
                    return Ok((E::Load(Place::Field(f)), t));
                }
                if let Some(v) = self.vars.get(&n).copied() {
                    return Ok((E::Load(v.place), v.ty));
                }
                if let Some(&(c, t)) = self.consts.get(&n) {
                    return Ok((E::Lit(c), t));
                }
                if self.is_sym("(") || is_function(&n) {
                    return self.call(&n, line);
                }
                err(line, format!("undeclared identifier: '{n}'"))
            }
            _ => {
                self.p -= 1;
                err(line, format!("expression expected but {} found", self.desc()))
            }
        }
    }

    fn args(&mut self) -> Result<Vec<(E, Ty, usize)>, String> {
        let mut out = Vec::new();
        if !self.eat_sym("(") {
            return Ok(out);
        }
        if self.eat_sym(")") {
            return Ok(out);
        }
        loop {
            let l = self.line();
            let (e, t) = self.expr()?;
            out.push((e, t, l));
            if self.eat_sym(")") {
                return Ok(out);
            }
            self.expect_sym(",")?;
        }
    }

    fn call(&mut self, name: &str, line: usize) -> Result<(E, Ty), String> {
        let args = self.args()?;
        let n = args.len();
        let want = |k: usize| -> Result<(), String> {
            if n == k {
                Ok(())
            } else {
                err(line, format!("{name}: {k} argument(s) expected, {n} given"))
            }
        };
        let mut args = args.into_iter();
        let real = |args: &mut std::vec::IntoIter<(E, Ty, usize)>| -> Result<E, String> {
            let (e, t, l) = args.next().expect("counted");
            coerce(e, t, Ty::Real, l)
        };
        if let Some(f) = F1::by_name(name) {
            want(1)?;
            return Ok((E::F1(f, bx(real(&mut args)?)), Ty::Real));
        }
        let f2 = match name {
            "arctan2" => Some(F2::ArcTan2),
            "hypot" => Some(F2::Hypot),
            "logn" => Some(F2::LogN),
            "power" => Some(F2::Power),
            _ => None,
        };
        if let Some(f) = f2 {
            want(2)?;
            let a = real(&mut args)?;
            let b = real(&mut args)?;
            return Ok((E::F2(f, bx(a), bx(b)), Ty::Real));
        }
        let mut args: Vec<(E, Ty, usize)> = args.collect();
        match name {
            "min" | "max" => {
                want(2)?;
                let (ta, tb) = (args[0].1, args[1].1);
                let (b, _, lb) = args.pop().expect("counted");
                let (a, _, la) = args.pop().expect("counted");
                if ta.ordinal() && tb.ordinal() {
                    let ty = if ta == Ty::Int64 || tb == Ty::Int64 { Ty::Int64 } else { Ty::Int };
                    return Ok((E::OMinMax(name == "max", bx(a), bx(b)), ty));
                }
                let a = coerce(a, ta, Ty::Real, la)?;
                let b = coerce(b, tb, Ty::Real, lb)?;
                let f = if name == "min" { F2::Min } else { F2::Max };
                Ok((E::F2(f, bx(a), bx(b)), Ty::Real))
            }
            "intpower" => {
                want(2)?;
                let (b, tb, l) = args.pop().expect("counted");
                let (a, ta, la) = args.pop().expect("counted");
                let a = coerce(a, ta, Ty::Real, la)?;
                if !tb.ordinal() {
                    return err(l, "IntPower: integer exponent expected");
                }
                Ok((E::IntPower(bx(a), bx(b)), Ty::Real))
            }
            "abs" | "sqr" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                let sq = name == "sqr";
                match t {
                    Ty::Int => Ok((if sq { E::ISqr(bx(a)) } else { E::IAbs(bx(a)) }, t)),
                    Ty::Int64 => Ok((if sq { E::RSqr(bx(a)) } else { E::RAbs(bx(a)) }, t)),
                    Ty::Real | Ty::Single => Ok((if sq { E::RSqr(bx(a)) } else { E::RAbs(bx(a)) }, Ty::Real)),
                    Ty::Bool => err(l, "numeric argument expected"),
                }
            }
            "round" | "trunc" | "ceil" | "floor" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                if t.ordinal() {
                    return Ok((a, t));
                }
                let a = coerce(a, t, Ty::Real, l)?;
                let ty = if name == "ceil" || name == "floor" { Ty::Int } else { Ty::Int64 };
                let k = match name {
                    "round" => Rnd::Round,
                    "trunc" => Rnd::Trunc,
                    "ceil" => Rnd::Ceil,
                    _ => Rnd::Floor,
                };
                Ok((E::Rnd(k, ty == Ty::Int, bx(a)), ty))
            }
            "odd" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                if !t.ordinal() {
                    return err(l, "ordinal argument expected");
                }
                Ok((E::Odd(bx(a)), Ty::Bool))
            }
            "isnan" | "isinfinite" | "sign" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                let a = bx(coerce(a, t, Ty::Real, l)?);
                Ok(match name {
                    "isnan" => (E::IsNan(a), Ty::Bool),
                    "isinfinite" => (E::IsInf(a), Ty::Bool),
                    _ => (E::Sign(a), Ty::Int),
                })
            }
            _ => err(line, format!("undeclared identifier: '{name}'")),
        }
    }
}

fn is_function(n: &str) -> bool {
    matches!(n, "pi")
}

fn is_reserved(s: &str) -> bool {
    matches!(
        s,
        "begin" | "end" | "if" | "then" | "else" | "while" | "do" | "repeat" | "until" | "for" | "to" | "downto" | "var" | "const"
            | "procedure" | "function" | "and" | "or" | "xor" | "not" | "div" | "mod" | "shl" | "shr" | "case" | "of" | "type"
    )
}

/// Assignment compatibility (Delphi rules).
fn coerce(e: E, from: Ty, to: Ty, line: usize) -> Result<E, String> {
    let ok = match to {
        Ty::Real | Ty::Single => from.numeric(),
        Ty::Int | Ty::Int64 => from.ordinal(),
        Ty::Bool => from == Ty::Bool,
    };
    if ok {
        Ok(e)
    } else {
        err(line, format!("incompatible types: '{}' and '{}'", to.name(), from.name()))
    }
}

fn binary(op: &str, a: E, ta: Ty, b: E, tb: Ty, line: usize) -> Result<(E, Ty), String> {
    let both_ord = ta.ordinal() && tb.ordinal();
    let ity = if ta == Ty::Int64 || tb == Ty::Int64 { Ty::Int64 } else { Ty::Int };
    let w = ity == Ty::Int;
    let (a, b) = (bx(a), bx(b));
    match op {
        "+" | "-" | "*" => {
            if !(ta.numeric() && tb.numeric()) {
                return err(line, format!("operator not applicable to '{}' and '{}'", ta.name(), tb.name()));
            }
            if both_ord {
                let o = match op {
                    "+" => IOp::Add,
                    "-" => IOp::Sub,
                    _ => IOp::Mul,
                };
                return Ok((E::I(o, w, a, b), ity));
            }
            let o = match op {
                "+" => ROp::Add,
                "-" => ROp::Sub,
                _ => ROp::Mul,
            };
            Ok((E::R(o, a, b), Ty::Real))
        }
        "/" => {
            if !(ta.numeric() && tb.numeric()) {
                return err(line, format!("operator not applicable to '{}' and '{}'", ta.name(), tb.name()));
            }
            Ok((E::R(ROp::Div, a, b), Ty::Real))
        }
        "div" | "mod" | "shl" | "shr" => {
            if !both_ord {
                return err(line, format!("operator '{op}' needs integer operands"));
            }
            let o = match op {
                "div" => IOp::Div,
                "mod" => IOp::Mod,
                "shl" => IOp::Shl,
                _ => IOp::Shr,
            };
            Ok((E::I(o, w, a, b), ity))
        }
        "and" | "or" | "xor" => {
            if ta == Ty::Bool && tb == Ty::Bool {
                // complete boolean evaluation is off ({$B-}): short-circuit
                return Ok((
                    match op {
                        "and" => E::And(a, b),
                        "or" => E::Or(a, b),
                        _ => E::Xor(a, b),
                    },
                    Ty::Bool,
                ));
            }
            if !both_ord {
                return err(line, format!("operator '{op}' not applicable to '{}' and '{}'", ta.name(), tb.name()));
            }
            let o = match op {
                "and" => IOp::And,
                "or" => IOp::Or,
                _ => IOp::Xor,
            };
            Ok((E::I(o, w, a, b), ity))
        }
        _ => err(line, format!("unknown operator {op}")),
    }
}

/// Math.IntPower (repeated squaring; negative exponents give the reciprocal).
pub fn int_power(base: f64, exponent: i32) -> f64 {
    let mut y = exponent.unsigned_abs();
    let mut b = base;
    let mut r = 1.0;
    while y > 0 {
        while y & 1 == 0 {
            y >>= 1;
            b *= b;
        }
        y -= 1;
        r *= b;
    }
    if exponent < 0 {
        1.0 / r
    } else {
        r
    }
}

/// Math.Power
pub fn power(base: f64, exponent: f64) -> f64 {
    if exponent == 0.0 {
        1.0
    } else if base == 0.0 && exponent > 0.0 {
        0.0
    } else if exponent.fract() == 0.0 && exponent.abs() <= i32::MAX as f64 {
        int_power(base, exponent as i32)
    } else {
        (exponent * base.ln()).exp()
    }
}

/// The value stored into a local or field of type `ty`.
#[inline]
pub(crate) fn store_value(ty: Ty, v: f64) -> f64 {
    match ty {
        Ty::Single => v as f32 as f64,
        Ty::Int => wrap32(v as i64),
        Ty::Bool => (v != 0.0) as i32 as f64,
        _ => v,
    }
}

/// The ordinal operation `op` on the values `a`, `b` (an Integer result
/// with `w`).
#[inline]
pub(crate) fn int_op(op: IOp, w: bool, a: f64, b: f64) -> f64 {
    let fix = |v: i64| if w { wrap32(v) } else { v as f64 };
    let (x, y) = (a as i64, b as i64);
    match op {
        IOp::Add => fix(x.wrapping_add(y)),
        IOp::Sub => fix(x.wrapping_sub(y)),
        IOp::Mul => fix(x.wrapping_mul(y)),
        IOp::Div => {
            if y == 0 {
                0.0
            } else {
                fix(x.wrapping_div(y))
            }
        }
        IOp::Mod => {
            if y == 0 {
                0.0
            } else {
                fix(x.wrapping_rem(y))
            }
        }
        IOp::And => fix(x & y),
        IOp::Or => fix(x | y),
        IOp::Xor => fix(x ^ y),
        IOp::Shl => {
            if w {
                ((x as i32).wrapping_shl(b as u32)) as f64
            } else {
                (x.wrapping_shl(b as u32)) as f64
            }
        }
        IOp::Shr => {
            if w {
                ((x as u32).wrapping_shr(b as u32)) as i32 as f64
            } else {
                ((x as u64).wrapping_shr(b as u32)) as i64 as f64
            }
        }
    }
}

#[inline]
pub(crate) fn round_value(k: Rnd, w: bool, x: f64) -> f64 {
    let r = match k {
        // x87 FISTP in the default rounding mode: to nearest, ties to even
        Rnd::Round => x.round_ties_even(),
        Rnd::Trunc => x.trunc(),
        Rnd::Ceil => x.ceil(),
        Rnd::Floor => x.floor(),
    };
    if w {
        wrap32(r as i64)
    } else {
        r
    }
}

#[inline]
pub(crate) fn sign_value(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// the closures

/// The value of a constant expression (literals and constants only).
pub(crate) fn eval_const(e: &E) -> f64 {
    let mut it = Iteration::default();
    let mut l = [0.0f64; 0];
    let mut env = Env { l: &mut l, it: &mut it };
    lower_e(e)(&mut env)
}

fn lower_e(e: &E) -> Ex {
    let b = |x: &E| lower_e(x);
    match e {
        E::Lit(v) => {
            let v = *v;
            Box::new(move |_| v)
        }
        E::Load(Place::Local(i)) => {
            let i = *i;
            Box::new(move |e| e.l[i])
        }
        E::Load(Place::V(k)) => {
            let k = *k;
            Box::new(move |e| e.it.v[k])
        }
        E::Load(Place::Field(f)) => {
            let f = *f;
            Box::new(move |e| get_field(e.it, f))
        }
        E::Neg(a, wrap) => {
            let a = b(a);
            if *wrap {
                Box::new(move |e| wrap32(-(a(e) as i64)))
            } else {
                Box::new(move |e| -a(e))
            }
        }
        E::NotB(a) => {
            let a = b(a);
            Box::new(move |e| (a(e) == 0.0) as i32 as f64)
        }
        E::NotI(a, wide) => {
            let a = b(a);
            if *wide {
                Box::new(move |e| !(a(e) as i64) as f64)
            } else {
                Box::new(move |e| !(a(e) as i64 as i32) as f64)
            }
        }
        E::Cmp(c, x, y) => {
            let (x, y) = (b(x), b(y));
            match c {
                Cmp::Eq => Box::new(move |e| (x(e) == y(e)) as i32 as f64),
                Cmp::Ne => Box::new(move |e| (x(e) != y(e)) as i32 as f64),
                Cmp::Lt => Box::new(move |e| (x(e) < y(e)) as i32 as f64),
                Cmp::Gt => Box::new(move |e| (x(e) > y(e)) as i32 as f64),
                Cmp::Le => Box::new(move |e| (x(e) <= y(e)) as i32 as f64),
                Cmp::Ge => Box::new(move |e| (x(e) >= y(e)) as i32 as f64),
            }
        }
        E::R(o, x, y) => {
            let (x, y) = (b(x), b(y));
            match o {
                ROp::Add => Box::new(move |e| x(e) + y(e)),
                ROp::Sub => Box::new(move |e| x(e) - y(e)),
                ROp::Mul => Box::new(move |e| x(e) * y(e)),
                ROp::Div => Box::new(move |e| x(e) / y(e)),
            }
        }
        E::I(o, w, x, y) => {
            let (x, y, o, w) = (b(x), b(y), *o, *w);
            Box::new(move |e| {
                let a = x(e);
                int_op(o, w, a, y(e))
            })
        }
        E::And(x, y) => {
            let (x, y) = (b(x), b(y));
            Box::new(move |e| if x(e) != 0.0 { (y(e) != 0.0) as i32 as f64 } else { 0.0 })
        }
        E::Or(x, y) => {
            let (x, y) = (b(x), b(y));
            Box::new(move |e| if x(e) != 0.0 { 1.0 } else { (y(e) != 0.0) as i32 as f64 })
        }
        E::Xor(x, y) => {
            let (x, y) = (b(x), b(y));
            Box::new(move |e| {
                let a = x(e) != 0.0;
                (a != (y(e) != 0.0)) as i32 as f64
            })
        }
        E::F1(f, a) => {
            let (a, f) = (b(a), *f);
            Box::new(move |e| f.eval(a(e)))
        }
        E::F2(f, x, y) => {
            let (x, y, f) = (b(x), b(y), *f);
            Box::new(move |e| {
                let a = x(e);
                f.eval(a, y(e))
            })
        }
        E::OMinMax(mx, x, y) => {
            let (x, y) = (b(x), b(y));
            if *mx {
                Box::new(move |e| {
                    let a = x(e);
                    a.max(y(e))
                })
            } else {
                Box::new(move |e| {
                    let a = x(e);
                    a.min(y(e))
                })
            }
        }
        E::IntPower(x, y) => {
            let (x, y) = (b(x), b(y));
            Box::new(move |e| {
                let a = x(e);
                int_power(a, y(e) as i32)
            })
        }
        E::IAbs(a) => {
            let a = b(a);
            Box::new(move |e| wrap32((a(e) as i64).abs()))
        }
        E::ISqr(a) => {
            let a = b(a);
            Box::new(move |e| wrap32((a(e) as i64).wrapping_mul(a(e) as i64)))
        }
        E::RAbs(a) => {
            let a = b(a);
            Box::new(move |e| a(e).abs())
        }
        E::RSqr(a) => {
            let a = b(a);
            Box::new(move |e| {
                let v = a(e);
                v * v
            })
        }
        E::Rnd(k, w, a) => {
            let (a, k, w) = (b(a), *k, *w);
            Box::new(move |e| round_value(k, w, a(e)))
        }
        E::Odd(a) => {
            let a = b(a);
            Box::new(move |e| ((a(e) as i64) & 1) as f64)
        }
        E::IsNan(a) => {
            let a = b(a);
            Box::new(move |e| a(e).is_nan() as i32 as f64)
        }
        E::IsInf(a) => {
            let a = b(a);
            Box::new(move |e| a(e).is_infinite() as i32 as f64)
        }
        E::Sign(a) => {
            let a = b(a);
            Box::new(move |e| sign_value(a(e)))
        }
    }
}

fn nop() -> St {
    Box::new(|_| NORMAL)
}

fn setter(p: Place, ty: Ty) -> Box<dyn Fn(&mut Env, f64) + Send + Sync> {
    match p {
        Place::Local(i) => Box::new(move |e, v| e.l[i] = store_value(ty, v)),
        Place::V(k) => Box::new(move |e, v| e.it.v[k] = v),
        Place::Field(f) => Box::new(move |e, v| set_field(e.it, f, store_value(ty, v))),
    }
}

fn lower_s(s: &S) -> St {
    match s {
        S::Nop => nop(),
        S::Seq(list) => {
            let mut list: Vec<St> = list.iter().map(lower_s).collect();
            match list.len() {
                0 => nop(),
                1 => list.pop().unwrap(),
                _ => Box::new(move |e| {
                    for s in &list {
                        let f = s(e);
                        if f != NORMAL {
                            return f;
                        }
                    }
                    NORMAL
                }),
            }
        }
        S::If(c, a, b) => {
            let c = lower_e(c);
            let a = lower_s(a);
            if matches!(**b, S::Nop) {
                Box::new(move |e| if c(e) != 0.0 { a(e) } else { NORMAL })
            } else {
                let b = lower_s(b);
                Box::new(move |e| if c(e) != 0.0 { a(e) } else { b(e) })
            }
        }
        S::While(c, body) => {
            let (c, body) = (lower_e(c), lower_s(body));
            Box::new(move |e| {
                while c(e) != 0.0 {
                    match body(e) {
                        BREAK => break,
                        EXIT => return EXIT,
                        _ => {}
                    }
                }
                NORMAL
            })
        }
        S::Repeat(body, c) => {
            let (c, body) = (lower_e(c), lower_s(body));
            Box::new(move |e| {
                loop {
                    match body(e) {
                        BREAK => break,
                        EXIT => return EXIT,
                        _ => {}
                    }
                    if c(e) != 0.0 {
                        break;
                    }
                }
                NORMAL
            })
        }
        S::For { slot, from, to, down, body } => {
            let (a, b, body, slot, down) = (lower_e(from), lower_e(to), lower_s(body), *slot, *down);
            Box::new(move |e| {
                let (from, to) = (a(e) as i64, b(e) as i64);
                let mut i = from;
                loop {
                    if (!down && i > to) || (down && i < to) {
                        break;
                    }
                    e.l[slot] = i as f64;
                    match body(e) {
                        BREAK => break,
                        EXIT => return EXIT,
                        _ => {}
                    }
                    i = if down { i - 1 } else { i + 1 };
                }
                NORMAL
            })
        }
        S::Case(sel, arms, other) => {
            let sel = lower_e(sel);
            let arms: Vec<(Vec<(i64, i64)>, St)> = arms.iter().map(|(l, s)| (l.clone(), lower_s(s))).collect();
            let other = other.as_ref().map(|s| lower_s(s));
            Box::new(move |e| {
                let v = sel(e) as i64;
                for (labels, st) in &arms {
                    if labels.iter().any(|&(lo, hi)| v >= lo && v <= hi) {
                        return st(e);
                    }
                }
                match &other {
                    Some(st) => st(e),
                    None => NORMAL,
                }
            })
        }
        S::Exit => Box::new(|_| EXIT),
        S::Break => Box::new(|_| BREAK),
        S::Continue => Box::new(|_| CONTINUE),
        S::Store(p, ty, v) => {
            let (v, ty) = (lower_e(v), *ty);
            match (*p, ty) {
                (Place::Local(i), Ty::Real | Ty::Int64) => Box::new(move |e| {
                    e.l[i] = v(e);
                    NORMAL
                }),
                (Place::Local(i), _) => Box::new(move |e| {
                    e.l[i] = store_value(ty, v(e));
                    NORMAL
                }),
                (Place::V(k), _) => Box::new(move |e| {
                    e.it.v[k] = v(e);
                    NORMAL
                }),
                (Place::Field(f), _) => Box::new(move |e| {
                    let x = store_value(ty, v(e));
                    set_field(e.it, f, x);
                    NORMAL
                }),
            }
        }
        S::SinCos(a, (ps, ts), (pc, tc)) => {
            let a = lower_e(a);
            let (ss, sc) = (setter(*ps, *ts), setter(*pc, *tc));
            Box::new(move |e| {
                let (s, c) = a(e).sin_cos();
                ss(e, s);
                sc(e, c);
                NORMAL
            })
        }
    }
}

// ---------------------------------------------------------------------------
// specialising a tree: the locals with known values as constants

/// The locals `s` assigns.
fn assigned(s: &S, out: &mut Vec<bool>) {
    let mut set = |p: &Place| {
        if let Place::Local(i) = p {
            out[*i] = true;
        }
    };
    match s {
        S::Seq(l) => l.iter().for_each(|s| assigned(s, out)),
        S::If(_, a, b) => {
            assigned(a, out);
            assigned(b, out);
        }
        S::While(_, b) | S::Repeat(b, _) => assigned(b, out),
        S::For { slot, body, .. } => {
            out[*slot] = true;
            assigned(body, out);
        }
        S::Case(_, arms, other) => {
            arms.iter().for_each(|(_, s)| assigned(s, out));
            if let Some(o) = other {
                assigned(o, out);
            }
        }
        S::Store(p, _, _) => set(p),
        S::SinCos(_, (a, _), (b, _)) => {
            set(a);
            set(b);
        }
        S::Nop | S::Exit | S::Break | S::Continue => {}
    }
}

struct Folder {
    /// the value of each local that is never assigned
    known: Vec<Option<f64>>,
}

pub(crate) fn lit(e: &E) -> Option<f64> {
    match e {
        E::Lit(v) => Some(*v),
        _ => None,
    }
}

impl Folder {
    fn e(&self, e: &E) -> E {
        let f = |x: &E| Box::new(self.e(x));
        let out = match e {
            E::Lit(v) => return E::Lit(*v),
            E::Load(Place::Local(i)) => {
                return match self.known[*i] {
                    Some(v) => E::Lit(v),
                    None => e.clone(),
                }
            }
            E::Load(_) => return e.clone(),
            E::Neg(a, w) => E::Neg(f(a), *w),
            E::NotB(a) => E::NotB(f(a)),
            E::NotI(a, w) => E::NotI(f(a), *w),
            E::Cmp(c, a, b) => E::Cmp(*c, f(a), f(b)),
            E::R(o, a, b) => E::R(*o, f(a), f(b)),
            E::I(o, w, a, b) => E::I(*o, *w, f(a), f(b)),
            E::And(a, b) | E::Or(a, b) => {
                let (a, b) = (f(a), f(b));
                let and = matches!(e, E::And(..));
                // a known first operand decides or leaves the second
                if let Some(x) = lit(&a) {
                    return match (and, x != 0.0) {
                        (true, false) => E::Lit(0.0),
                        (false, true) => E::Lit(1.0),
                        _ => self.e(&E::Cmp(Cmp::Ne, b, Box::new(E::Lit(0.0)))),
                    };
                }
                if and {
                    E::And(a, b)
                } else {
                    E::Or(a, b)
                }
            }
            E::Xor(a, b) => E::Xor(f(a), f(b)),
            E::F1(k, a) => E::F1(*k, f(a)),
            E::F2(k, a, b) => E::F2(*k, f(a), f(b)),
            E::OMinMax(m, a, b) => E::OMinMax(*m, f(a), f(b)),
            E::IntPower(a, b) => E::IntPower(f(a), f(b)),
            E::IAbs(a) => E::IAbs(f(a)),
            E::ISqr(a) => E::ISqr(f(a)),
            E::RAbs(a) => E::RAbs(f(a)),
            E::RSqr(a) => E::RSqr(f(a)),
            E::Rnd(k, w, a) => E::Rnd(*k, *w, f(a)),
            E::Odd(a) => E::Odd(f(a)),
            E::IsNan(a) => E::IsNan(f(a)),
            E::IsInf(a) => E::IsInf(f(a)),
            E::Sign(a) => E::Sign(f(a)),
        };
        // every operand known: the closures' value
        if children(&out).iter().all(|c| lit(c).is_some()) {
            E::Lit(eval_const(&out))
        } else {
            out
        }
    }

    fn s(&self, s: &S) -> S {
        match s {
            S::Seq(l) => {
                let mut v = Vec::new();
                for s in l {
                    match self.s(s) {
                        S::Nop => {}
                        S::Seq(inner) => v.extend(inner),
                        x => {
                            let ends = matches!(x, S::Exit | S::Break | S::Continue);
                            v.push(x);
                            // the rest of the list never runs
                            if ends {
                                break;
                            }
                        }
                    }
                }
                S::Seq(v)
            }
            S::If(c, a, b) => {
                let c = self.e(c);
                match lit(&c) {
                    Some(x) if x != 0.0 => self.s(a),
                    Some(_) => self.s(b),
                    None => S::If(c, Box::new(self.s(a)), Box::new(self.s(b))),
                }
            }
            S::While(c, b) => {
                let c = self.e(c);
                if lit(&c) == Some(0.0) {
                    return S::Nop;
                }
                S::While(c, Box::new(self.s(b)))
            }
            S::Repeat(b, c) => S::Repeat(Box::new(self.s(b)), self.e(c)),
            S::For { slot, from, to, down, body } => S::For { slot: *slot, from: self.e(from), to: self.e(to), down: *down, body: Box::new(self.s(body)) },
            S::Case(sel, arms, other) => {
                let sel = self.e(sel);
                if let Some(v) = lit(&sel) {
                    let v = v as i64;
                    for (labels, st) in arms {
                        if labels.iter().any(|&(lo, hi)| v >= lo && v <= hi) {
                            return self.s(st);
                        }
                    }
                    return other.as_ref().map_or(S::Nop, |o| self.s(o));
                }
                S::Case(sel, arms.iter().map(|(l, s)| (l.clone(), self.s(s))).collect(), other.as_ref().map(|o| Box::new(self.s(o))))
            }
            S::Store(p, t, v) => S::Store(*p, *t, self.e(v)),
            S::SinCos(a, x, y) => S::SinCos(self.e(a), *x, *y),
            S::Nop | S::Exit | S::Break | S::Continue => s.clone(),
        }
    }
}

fn children(e: &E) -> Vec<&E> {
    match e {
        E::Lit(_) | E::Load(_) => vec![],
        E::Neg(a, _)
        | E::NotB(a)
        | E::NotI(a, _)
        | E::F1(_, a)
        | E::IAbs(a)
        | E::ISqr(a)
        | E::RAbs(a)
        | E::RSqr(a)
        | E::Rnd(_, _, a)
        | E::Odd(a)
        | E::IsNan(a)
        | E::IsInf(a)
        | E::Sign(a) => vec![a],
        E::Cmp(_, a, b) | E::R(_, a, b) | E::I(_, _, a, b) | E::And(a, b) | E::Or(a, b) | E::Xor(a, b) | E::F2(_, a, b) | E::OMinMax(_, a, b) | E::IntPower(a, b) => {
            vec![a, b]
        }
    }
}

/// The locals (of `n`) the tree assigns.
pub(crate) fn assigned_locals(tree: &S, n: usize) -> Vec<bool> {
    let mut v = vec![false; n];
    assigned(tree, &mut v);
    v
}

/// `tree` with the locals of known value (`known[i]`) as constants and
/// what depends only on constants computed (with the closures, so exactly),
/// the branches decided by them taken.
pub(crate) fn fold(tree: &S, known: Vec<Option<f64>>) -> S {
    Folder { known }.s(tree)
}

// ---------------------------------------------------------------------------
// the compiled formula

/// A compiled `[SOURCE]` formula.
pub struct Program {
    pub name: String,
    body: St,
    /// the tree the closures were made from (for the code generators)
    #[cfg_attr(not(jit), allow(dead_code))]
    pub(crate) tree: S,
    pub(crate) nloc: usize,
    /// the type of each local (for the GPU translation)
    #[allow(dead_code)]
    pub(crate) loc_ty: Vec<Ty>,
    pub(crate) inits: Vec<(usize, Init)>,
    /// some option is read from a non-Double slot: the variable buffer is needed
    needs_buffer: bool,
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Program({}, {} locals)", self.name, self.nloc)
    }
}

/// `TStringList.Sort` with `AnsiCompareText` (Windows "word sort",
/// case-insensitive): punctuation before digits before letters; hyphens and
/// apostrophes are ignored.
fn ansi_key(s: &str) -> Vec<(u8, u32)> {
    s.chars()
        .filter(|&c| c != '-' && c != '\'')
        .map(|c| {
            let c = c.to_ascii_lowercase();
            let class = if c.is_ascii_digit() {
                1
            } else if c.is_alphabetic() {
                2
            } else {
                0
            };
            (class, c as u32)
        })
        .collect()
}

/// `NameToPascalName`
fn pascal_name(s: &str) -> String {
    s.chars().map(|c| if matches!(c, ' ' | '-' | '+' | '/' | '*') { '_' } else { c }).collect::<String>().to_ascii_lowercase()
}

/// A named constant of the `[CONSTANTS]` section.
#[derive(Clone, Debug)]
pub struct ConstDef {
    pub name: String,
    /// 0 Double, 1 Integer, 2 Int64, 3 Single
    pub ty: u8,
}

impl Program {
    /// Compiles the source of formula `def` (its options and constants
    /// become variables as in MB3D's preprocessor).
    pub fn compile(def: &M3f, source: &str) -> Result<Program, String> {
        let consts = &def.const_defs;
        let toks = tokenize(source)?;
        let mut c = Compiler {
            toks,
            p: 0,
            vars: HashMap::new(),
            consts: HashMap::new(),
            nloc: 0,
            inits: Vec::new(),
            loc_ty: Vec::new(),
            ptr_name: String::new(),
            loop_depth: 0,
            in_const: false,
        };
        let name = c.header()?;
        let mut needs_buffer = false;

        // the preprocessor's variables (only if there are options or constants)
        let mut params: Vec<(String, usize)> = Vec::new(); // name, option line
        for (i, n) in def.param_names.iter().enumerate() {
            // TStringList with dupError ignores repeated names (SetValue updates)
            if !params.iter().any(|(m, _)| m.eq_ignore_ascii_case(n)) {
                params.push((n.clone(), i));
            }
        }
        params.sort_by(|a, b| ansi_key(&a.0).cmp(&ansi_key(&b.0)));
        // byte offset below PVar of each option in the variable buffer
        let mut opt_off = Vec::new();
        let mut p = 8usize;
        for o in &def.options {
            let need = crate::m3f::MEM_NEEDED.get(o.ty as usize).copied().unwrap_or(0);
            opt_off.push((p + 8, o.ty));
            p += need;
        }
        let mut sorted_consts: Vec<&ConstDef> = consts.iter().collect();
        sorted_consts.sort_by(|a, b| ansi_key(&a.name).cmp(&ansi_key(&b.name)));
        let mut voff = 16usize;
        for (pname, _) in &params {
            let l = c.new_local(Ty::Real);
            // which option sits at PVar - voff?
            let init = match opt_off.iter().position(|&(o, ty)| o == voff && ty == 0) {
                Some(i) => Init::Value(i),
                None => {
                    needs_buffer = true;
                    Init::Below(voff)
                }
            };
            c.inits.push((l, init));
            let n = pascal_name(pname);
            if !n.is_empty() && !c.vars.contains_key(&n) {
                c.vars.insert(n, Var { place: Place::Local(l), ty: Ty::Real, constant: false });
            }
            voff += 8;
        }
        let mut coff = 0usize;
        for cd in sorted_consts {
            let (ty, size) = match cd.ty {
                1 => (Ty::Int, 4),
                2 => (Ty::Int64, 8),
                3 => (Ty::Single, 4),
                _ => (Ty::Real, 8),
            };
            let bytes = |n: usize| -> Vec<u8> { (0..n).map(|k| def.consts.get(coff + k).copied().unwrap_or(0)).collect() };
            let v = match cd.ty {
                1 => i32::from_le_bytes(bytes(4).try_into().unwrap()) as f64,
                2 => i64::from_le_bytes(bytes(8).try_into().unwrap()) as f64,
                3 => f32::from_le_bytes(bytes(4).try_into().unwrap()) as f64,
                _ => f64::from_le_bytes(bytes(8).try_into().unwrap()),
            };
            let l = c.new_local(ty);
            c.inits.push((l, Init::Const(v)));
            let n = pascal_name(&cd.name);
            if !c.vars.contains_key(&n) {
                c.vars.insert(n, Var { place: Place::Local(l), ty, constant: false });
            }
            coff += size;
        }

        c.decls()?;
        let tree = c.block()?;
        c.eat_sym(";");
        if *c.peek() != Tok::Eof {
            if c.is_kw("procedure") || c.is_kw("function") {
                return err(c.line(), "only one procedure per formula is supported");
            }
            return err(c.line(), format!("unexpected {} after the end of the procedure", c.desc()));
        }
        let body = lower_s(&tree);
        Ok(Program { name, body, tree, nloc: c.nloc, loc_ty: c.loc_ty, inits: c.inits, needs_buffer })
    }

    /// The start values of the locals that have one (option values,
    /// constants, initializers), for the option values `values` of `def`.
    #[cfg_attr(not(jit), allow(dead_code))]
    pub(crate) fn init_values(&self, def: &M3f, values: &[f64]) -> Vec<(usize, f64)> {
        let buf = if self.needs_buffer { Some(def.var_buffer(values, &|_| 0)) } else { None };
        self.inits.iter().map(|(i, init)| (*i, init_value(init, values, buf.as_deref()))).collect()
    }

    /// One iteration step on `it`, with the option values `values` of `def`.
    #[inline]
    pub fn run(&self, it: &mut Iteration, def: &M3f, values: &[f64]) {
        let mut small = [0.0f64; 48];
        let mut big;
        let l: &mut [f64] = if self.nloc <= small.len() {
            &mut small[..self.nloc]
        } else {
            big = vec![0.0; self.nloc];
            &mut big[..]
        };
        let buf = if self.needs_buffer { Some(def.var_buffer(values, &|_| 0)) } else { None };
        for (i, init) in &self.inits {
            l[*i] = init_value(init, values, buf.as_deref());
        }
        let mut env = Env { l, it };
        (self.body)(&mut env);
    }
}

#[inline]
fn init_value(init: &Init, values: &[f64], buf: Option<&[u8]>) -> f64 {
    match init {
        Init::Value(k) => values.get(*k).copied().unwrap_or(0.0),
        Init::Const(v) => *v,
        Init::Below(off) => {
            let b = buf.expect("the variable buffer");
            let o = crate::m3f::CONST_OFFSET - off;
            f64::from_le_bytes(b[o..o + 8].try_into().unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(opts: &str, src: &str) -> (M3f, std::sync::Arc<Program>) {
        let text = format!("[OPTIONS]\n.Version = 9\n{opts}\n[SOURCE]\n{src}\n[END]\n");
        let def = M3f::parse("t", &text).unwrap();
        let p = def.jit.clone().unwrap();
        (def, p)
    }

    #[test]
    fn delphi_math() {
        assert_eq!(int_power(2.0, 10), 1024.0);
        assert_eq!(int_power(2.0, -2), 0.25);
        assert_eq!(power(-2.0, 3.0), -8.0);
        assert!(power(-2.0, 2.5).is_nan());
        assert_eq!(power(0.0, 2.5), 0.0);
        assert_eq!(2.5f64.round_ties_even(), 2.0);
    }

    #[test]
    fn const_expr_rejects_variables() {
        for (opts, decl) in [
            ("", "var a: Double = 1;\nconst k = a * 2;"),
            ("", "const k = x + 1;"),
            ("", "const k = PIteration3D^.J1;"),
            (".Double b = 2", "const k = b;"),
        ] {
            let src = format!("procedure F(var x, y, z, w: Double; PIteration3D: TPIteration3D);\n{decl}\nbegin\n x := k;\nend;");
            let text = format!("[OPTIONS]\n.Version = 9\n{opts}\n[SOURCE]\n{src}\n[END]\n");
            let e = M3f::parse("t", &text).err().expect(decl);
            assert!(e.contains("constant expression expected"), "{decl}: {e}");
        }
        // a case label using an option
        let src = "procedure F(var x, y, z, w: Double; PIteration3D: TPIteration3D);\nvar i: integer;\nbegin\n i := 1;\n case i of b: x := 1; end;\nend;";
        let text = format!("[OPTIONS]\n.Version = 9\n.Integer b = 1\n[SOURCE]\n{src}\n[END]\n");
        assert!(M3f::parse("t", &text).is_err());
    }

    #[test]
    fn options_sorted_like_the_preprocessor() {
        // options given in file order b, A: the sorted names are A, b, so A
        // reads the first slot (b's value) like in MB3D
        let (def, p) = prog(
            ".Double b = 2\n.Double A = 5",
            "procedure F(var x, y, z, w: Double; PIteration3D: TPIteration3D);\nbegin\n x := a; y := B;\nend;",
        );
        let mut it = Iteration::default();
        p.run(&mut it, &def, &def.defaults());
        assert_eq!((it.v[0], it.v[1]), (2.0, 5.0));
    }

    #[test]
    fn statements_and_types() {
        let src = r#"
procedure T(var x, y, z, w: Double; PIteration3D: TPIteration3D);
const two = 2;
var i, n: integer; s, c: double; b: boolean;
begin
  n := 0;
  for i := 1 to 10 do begin
    if odd(i) then continue;
    n := n + i;            { 2+4+6+8+10 }
  end;
  x := n div 4 + n mod 4;  // 7 + 2
  y := 7 / two;
  SinCos(0.5, s, c);
  z := s*s + c*c;
  b := (x > 3) and not (y < 0);
  if b then w := round(2.5) + trunc(-1.7) + Ceil(0.2) else w := -1;
  i := 0;
  while true do begin Inc(i, 3); if i > 10 then break; end;
  repeat Dec(i) until i < 5;
  case i of 0..3: x := x; 4: y := 100; else z := 0; end;
  PIteration3D^.J1 := PIteration3D^.J2 * 2;
  exit;
  w := 99;
end;"#;
        let (def, p) = prog("", src);
        let mut it = Iteration::default();
        it.j[1] = 1.5;
        p.run(&mut it, &def, &[]);
        assert_eq!(it.v[0], 9.0);
        assert_eq!(it.v[1], 100.0);
        assert!((it.v[2] - 1.0).abs() < 1e-15);
        assert_eq!(it.v[3], 2.0 - 1.0 + 1.0);
        assert_eq!(it.j[0], 3.0);
    }

    #[test]
    fn compile_errors() {
        let bad = [
            ("begin end", "no formula"),
            ("procedure F(var x, y, z, w: Double; P: TPIteration3D);\nvar i: integer;\nbegin\n i := 1.5;\nend;", "incompatible"),
            ("procedure F(var x, y, z, w: Double; P: TPIteration3D);\nbegin\n x := foo(1);\nend;", "undeclared"),
            ("procedure F(var x, y, z, w: Double; P: TPIteration3D);\nbegin\n if x then y := 1;\nend;", "BOOLEAN"),
            ("procedure F(var x, y, z, w: Double; P: TPIteration3D);\nbegin\n x := 1\n y := 2;\nend;", "';' expected"),
        ];
        for (src, want) in bad {
            let text = format!("[OPTIONS]\n.Version = 9\n[SOURCE]\n{src}\n[END]\n");
            let e = M3f::parse("t", &text).unwrap_err();
            assert!(e.contains(want), "{src}: {e}");
        }
    }
}
