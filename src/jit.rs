//! Formulas written in Pascal (`[SOURCE]` sections of `.m3f` files).
//!
//! MB3D compiles them at load time with paxCompiler (`formula/FormulaCompiler.pas`,
//! `script/CompilerUtil.pas`).  This module compiles the same Delphi subset
//! into a tree of closures that work directly on the iteration state:
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
// types and run time

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ty {
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
fn wrap32(v: i64) -> f64 {
    v as i32 as f64
}

/// Fields of `TIteration3D` / `TIteration3Dext` the formulas can use.
#[derive(Clone, Copy, Debug)]
enum Field {
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
fn get_field(it: &Iteration, f: Field) -> f64 {
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
fn set_field(it: &mut Iteration, f: Field, v: f64) {
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
#[derive(Clone, Copy, Debug)]
enum Place {
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
enum Init {
    /// option value number n (a Double option at the preprocessor's offset)
    Value(usize),
    /// 8 bytes of the variable buffer below `PVar` (offset in bytes)
    Below(usize),
    /// a value from the `[CONSTANTS]` section
    Const(f64),
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
        let mut it = Iteration::default();
        let mut l = [0.0f64; 0];
        let mut env = Env { l: &mut l, it: &mut it };
        let v = e(&mut env);
        if !v.is_finite() && !ty.real() {
            return err(line, "constant expression expected");
        }
        Ok((v, ty))
    }

    // ---- statements

    fn block(&mut self) -> Result<St, String> {
        self.expect_kw("begin")?;
        let mut list: Vec<St> = Vec::new();
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
        Ok(seq(list))
    }

    fn statement(&mut self) -> Result<St, String> {
        let line = self.line();
        match self.peek().clone() {
            Tok::Id(k) => match k.as_str() {
                "begin" => self.block(),
                "if" => {
                    self.bump();
                    let c = self.condition()?;
                    self.expect_kw("then")?;
                    let a = if self.is_kw("else") || self.is_sym(";") { nop() } else { self.statement()? };
                    if self.eat_kw("else") {
                        let b = if self.is_kw("end") || self.is_sym(";") { nop() } else { self.statement()? };
                        Ok(Box::new(move |e| if c(e) != 0.0 { a(e) } else { b(e) }))
                    } else {
                        Ok(Box::new(move |e| if c(e) != 0.0 { a(e) } else { NORMAL }))
                    }
                }
                "while" => {
                    self.bump();
                    let c = self.condition()?;
                    self.expect_kw("do")?;
                    self.loop_depth += 1;
                    let body = self.statement()?;
                    self.loop_depth -= 1;
                    Ok(Box::new(move |e| {
                        while c(e) != 0.0 {
                            match body(e) {
                                BREAK => break,
                                EXIT => return EXIT,
                                _ => {}
                            }
                        }
                        NORMAL
                    }))
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
                    let body = seq(list);
                    Ok(Box::new(move |e| {
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
                    }))
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
                    Ok(Box::new(move |e| {
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
                    }))
                }
                "case" => {
                    self.bump();
                    let (sel, ts) = self.expr()?;
                    if !ts.ordinal() {
                        return err(line, "ordinal type required");
                    }
                    self.expect_kw("of")?;
                    let mut arms: Vec<(Vec<(i64, i64)>, St)> = Vec::new();
                    let mut other: Option<St> = None;
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
                            other = Some(seq(list));
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
                        let st = if self.is_sym(";") { nop() } else { self.statement()? };
                        arms.push((labels, st));
                        if !self.eat_sym(";") && !self.is_kw("end") && !self.is_kw("else") {
                            return err(self.line(), format!("';' expected but {} found", self.desc()));
                        }
                    }
                    Ok(Box::new(move |e| {
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
                    }))
                }
                "exit" => {
                    self.bump();
                    if self.eat_sym("(") {
                        self.expect_sym(")")?;
                    }
                    Ok(Box::new(|_| EXIT))
                }
                "break" | "continue" => {
                    self.bump();
                    if self.loop_depth == 0 {
                        return err(line, format!("{k} outside of a loop"));
                    }
                    let f = if k == "break" { BREAK } else { CONTINUE };
                    Ok(Box::new(move |_| f))
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
                        lit(1.0)
                    };
                    self.expect_sym(")")?;
                    let sign = if k == "inc" { 1i64 } else { -1 };
                    let get = load(place);
                    Ok(store(place, ty, Box::new(move |e| (get(e) as i64 + sign * d(e) as i64) as f64)))
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
                    let (ss, sc) = (setter(ps, ts), setter(pc, tc));
                    Ok(Box::new(move |e| {
                        let (s, c) = a(e).sin_cos();
                        ss(e, s);
                        sc(e, c);
                        NORMAL
                    }))
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
                    Ok(store(place, ty, v))
                }
            },
            _ => err(line, format!("statement expected but {} found", self.desc())),
        }
    }

    fn real_arg(&mut self) -> Result<Ex, String> {
        let l = self.line();
        let (e, t) = self.expr()?;
        coerce(e, t, Ty::Real, l)
    }

    fn condition(&mut self) -> Result<Ex, String> {
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

    fn expr(&mut self) -> Result<(Ex, Ty), String> {
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
        let f: Ex = match op {
            "=" => Box::new(move |e| (a(e) == b(e)) as i32 as f64),
            "<>" => Box::new(move |e| (a(e) != b(e)) as i32 as f64),
            "<" => Box::new(move |e| (a(e) < b(e)) as i32 as f64),
            ">" => Box::new(move |e| (a(e) > b(e)) as i32 as f64),
            "<=" => Box::new(move |e| (a(e) <= b(e)) as i32 as f64),
            _ => Box::new(move |e| (a(e) >= b(e)) as i32 as f64),
        };
        Ok((f, Ty::Bool))
    }

    fn simple(&mut self) -> Result<(Ex, Ty), String> {
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

    fn term(&mut self) -> Result<(Ex, Ty), String> {
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

    fn factor(&mut self) -> Result<(Ex, Ty), String> {
        let line = self.line();
        match self.bump() {
            Tok::Int(v) => {
                let ty = if v > i32::MAX as i64 || v < i32::MIN as i64 { Ty::Int64 } else { Ty::Int };
                let f = v as f64;
                Ok((Box::new(move |_| f), ty))
            }
            Tok::Real(v) => Ok((Box::new(move |_| v), Ty::Real)),
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
                if t == Ty::Int {
                    Ok((Box::new(move |e| wrap32(-(a(e) as i64))), t))
                } else {
                    Ok((Box::new(move |e| -a(e)), t))
                }
            }
            Tok::Sym("+") => self.factor(),
            Tok::Id(k) if k == "not" => {
                let (a, t) = self.factor()?;
                match t {
                    Ty::Bool => Ok((Box::new(move |e| (a(e) == 0.0) as i32 as f64), t)),
                    Ty::Int => Ok((Box::new(move |e| !(a(e) as i64 as i32) as f64), t)),
                    Ty::Int64 => Ok((Box::new(move |e| !(a(e) as i64) as f64), t)),
                    _ => err(line, "operator not applicable to this operand type"),
                }
            }
            Tok::Id(k) if k == "true" || k == "false" => {
                let v = (k == "true") as i32 as f64;
                Ok((Box::new(move |_| v), Ty::Bool))
            }
            Tok::Id(k) if k == "pi" && !self.vars.contains_key("pi") => {
                if self.eat_sym("(") {
                    self.expect_sym(")")?;
                }
                Ok((Box::new(|_| std::f64::consts::PI), Ty::Real))
            }
            Tok::Id(n) => {
                if self.in_const && (n == self.ptr_name || self.vars.contains_key(&n)) {
                    return err(line, "constant expression expected");
                }
                if n == self.ptr_name {
                    let (f, t) = self.field()?;
                    return Ok((Box::new(move |e| get_field(e.it, f)), t));
                }
                if let Some(v) = self.vars.get(&n).copied() {
                    return Ok((load(v.place), v.ty));
                }
                if let Some(&(c, t)) = self.consts.get(&n) {
                    return Ok((Box::new(move |_| c), t));
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

    fn args(&mut self) -> Result<Vec<(Ex, Ty, usize)>, String> {
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

    fn call(&mut self, name: &str, line: usize) -> Result<(Ex, Ty), String> {
        let mut args = self.args()?;
        let n = args.len();
        let want = |k: usize| -> Result<(), String> {
            if n == k {
                Ok(())
            } else {
                err(line, format!("{name}: {k} argument(s) expected, {n} given"))
            }
        };
        let real = |i: usize, args: &mut Vec<(Ex, Ty, usize)>| -> Result<Ex, String> {
            let (e, t, l) = std::mem::replace(&mut args[i], (Box::new(|_| 0.0), Ty::Real, 0));
            coerce(e, t, Ty::Real, l)
        };
        macro_rules! r1 {
            ($f:expr) => {{
                want(1)?;
                let a = real(0, &mut args)?;
                let f: fn(f64) -> f64 = $f;
                return Ok((Box::new(move |e| f(a(e))), Ty::Real));
            }};
        }
        macro_rules! r2 {
            ($f:expr) => {{
                want(2)?;
                let a = real(0, &mut args)?;
                let b = real(1, &mut args)?;
                let f: fn(f64, f64) -> f64 = $f;
                return Ok((Box::new(move |e| f(a(e), b(e))), Ty::Real));
            }};
        }
        match name {
            "sin" => r1!(f64::sin),
            "cos" => r1!(f64::cos),
            "tan" => r1!(f64::tan),
            "cotan" | "cot" => r1!(|x| 1.0 / x.tan()),
            "secant" | "sec" => r1!(|x| 1.0 / x.cos()),
            "cosecant" | "csc" => r1!(|x| 1.0 / x.sin()),
            "arctan" => r1!(f64::atan),
            "arcsin" => r1!(f64::asin),
            "arccos" => r1!(f64::acos),
            "arccot" => r1!(|x| (1.0 / x).atan()),
            "arcsec" => r1!(|x| (1.0 / x).acos()),
            "arccsc" => r1!(|x| (1.0 / x).asin()),
            "sinh" => r1!(f64::sinh),
            "cosh" => r1!(f64::cosh),
            "tanh" => r1!(f64::tanh),
            "coth" => r1!(|x| 1.0 / x.tanh()),
            "sech" => r1!(|x| 1.0 / x.cosh()),
            "csch" => r1!(|x| 1.0 / x.sinh()),
            "arcsinh" => r1!(f64::asinh),
            "arccosh" => r1!(f64::acosh),
            "arctanh" => r1!(f64::atanh),
            "arccoth" => r1!(|x| (1.0 / x).atanh()),
            "arcsech" => r1!(|x| (1.0 / x).acosh()),
            "arccsch" => r1!(|x| (1.0 / x).asinh()),
            "sqrt" => r1!(f64::sqrt),
            "exp" => r1!(f64::exp),
            "ln" => r1!(f64::ln),
            "lnxp1" => r1!(f64::ln_1p),
            "log10" => r1!(f64::log10),
            "log2" => r1!(f64::log2),
            "int" => r1!(f64::trunc),
            "frac" => r1!(f64::fract),
            "radtodeg" => r1!(f64::to_degrees),
            "degtorad" => r1!(f64::to_radians),
            "degnormalize" => r1!(|d| d - (d / 360.0).floor() * 360.0),
            "arctan2" => r2!(f64::atan2),
            "hypot" => r2!(f64::hypot),
            "logn" => r2!(|b, x| x.ln() / b.ln()),
            "power" => r2!(power),
            "min" | "max" => {
                want(2)?;
                let (ta, tb) = (args[0].1, args[1].1);
                if ta.ordinal() && tb.ordinal() {
                    let (a, _, _) = args.remove(0);
                    let (b, _, _) = args.remove(0);
                    let ty = if ta == Ty::Int64 || tb == Ty::Int64 { Ty::Int64 } else { Ty::Int };
                    let mx = name == "max";
                    return Ok((Box::new(move |e| if mx { a(e).max(b(e)) } else { a(e).min(b(e)) }), ty));
                }
                // Math.Min/Max: "if A < B then Result := A else Result := B"
                if name == "min" {
                    r2!(|a, b| if a < b { a } else { b })
                } else {
                    r2!(|a, b| if a > b { a } else { b })
                }
            }
            "intpower" => {
                want(2)?;
                let a = real(0, &mut args)?;
                let (b, tb, l) = args.remove(1);
                if !tb.ordinal() {
                    return err(l, "IntPower: integer exponent expected");
                }
                return Ok((Box::new(move |e| int_power(a(e), b(e) as i32)), Ty::Real));
            }
            "abs" | "sqr" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                let sq = name == "sqr";
                return match t {
                    Ty::Int => Ok((if sq { Box::new(move |e| wrap32((a(e) as i64).wrapping_mul(a(e) as i64))) } else { Box::new(move |e| wrap32((a(e) as i64).abs())) }, t)),
                    Ty::Int64 => Ok((if sq { Box::new(move |e| { let v = a(e); v * v }) } else { Box::new(move |e| a(e).abs()) }, t)),
                    Ty::Real | Ty::Single => Ok((if sq { Box::new(move |e| { let v = a(e); v * v }) } else { Box::new(move |e| a(e).abs()) }, Ty::Real)),
                    Ty::Bool => err(l, "numeric argument expected"),
                };
            }
            "round" | "trunc" | "ceil" | "floor" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                if t.ordinal() {
                    return Ok((a, t));
                }
                let a = coerce(a, t, Ty::Real, l)?;
                let ty = if name == "ceil" || name == "floor" { Ty::Int } else { Ty::Int64 };
                let f: fn(f64) -> f64 = match name {
                    // x87 FISTP in the default rounding mode: to nearest, ties to even
                    "round" => f64::round_ties_even,
                    "trunc" => f64::trunc,
                    "ceil" => f64::ceil,
                    _ => f64::floor,
                };
                if ty == Ty::Int {
                    return Ok((Box::new(move |e| wrap32(f(a(e)) as i64)), ty));
                }
                return Ok((Box::new(move |e| f(a(e))), ty));
            }
            "odd" => {
                want(1)?;
                let (a, t, l) = args.remove(0);
                if !t.ordinal() {
                    return err(l, "ordinal argument expected");
                }
                return Ok((Box::new(move |e| ((a(e) as i64) & 1) as f64), Ty::Bool));
            }
            "isnan" => {
                want(1)?;
                let a = real(0, &mut args)?;
                return Ok((Box::new(move |e| a(e).is_nan() as i32 as f64), Ty::Bool));
            }
            "isinfinite" => {
                want(1)?;
                let a = real(0, &mut args)?;
                return Ok((Box::new(move |e| a(e).is_infinite() as i32 as f64), Ty::Bool));
            }
            "sign" => {
                want(1)?;
                let a = real(0, &mut args)?;
                return Ok((Box::new(move |e| { let v = a(e); if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 } }), Ty::Int));
            }
            _ => {}
        }
        err(line, format!("undeclared identifier: '{name}'"))
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

fn lit(v: f64) -> Ex {
    Box::new(move |_| v)
}

fn nop() -> St {
    Box::new(|_| NORMAL)
}

fn seq(mut list: Vec<St>) -> St {
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

fn load(p: Place) -> Ex {
    match p {
        Place::Local(i) => Box::new(move |e| e.l[i]),
        Place::V(k) => Box::new(move |e| e.it.v[k]),
        Place::Field(f) => Box::new(move |e| get_field(e.it, f)),
    }
}

fn store_value(ty: Ty, v: f64) -> f64 {
    match ty {
        Ty::Single => v as f32 as f64,
        Ty::Int => wrap32(v as i64),
        Ty::Bool => (v != 0.0) as i32 as f64,
        _ => v,
    }
}

fn setter(p: Place, ty: Ty) -> Box<dyn Fn(&mut Env, f64) + Send + Sync> {
    match p {
        Place::Local(i) => Box::new(move |e, v| e.l[i] = store_value(ty, v)),
        Place::V(k) => Box::new(move |e, v| e.it.v[k] = v),
        Place::Field(f) => Box::new(move |e, v| set_field(e.it, f, store_value(ty, v))),
    }
}

fn store(p: Place, ty: Ty, v: Ex) -> St {
    match (p, ty) {
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

/// Assignment compatibility (Delphi rules).
fn coerce(e: Ex, from: Ty, to: Ty, line: usize) -> Result<Ex, String> {
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

fn binary(op: &str, a: Ex, ta: Ty, b: Ex, tb: Ty, line: usize) -> Result<(Ex, Ty), String> {
    let both_ord = ta.ordinal() && tb.ordinal();
    let ity = if ta == Ty::Int64 || tb == Ty::Int64 { Ty::Int64 } else { Ty::Int };
    let w = ity == Ty::Int;
    let fix = move |v: i64| if w { wrap32(v) } else { v as f64 };
    match op {
        "+" | "-" | "*" => {
            if !(ta.numeric() && tb.numeric()) {
                return err(line, format!("operator not applicable to '{}' and '{}'", ta.name(), tb.name()));
            }
            if both_ord {
                let f: Ex = match op {
                    "+" => Box::new(move |e| fix((a(e) as i64).wrapping_add(b(e) as i64))),
                    "-" => Box::new(move |e| fix((a(e) as i64).wrapping_sub(b(e) as i64))),
                    _ => Box::new(move |e| fix((a(e) as i64).wrapping_mul(b(e) as i64))),
                };
                return Ok((f, ity));
            }
            let f: Ex = match op {
                "+" => Box::new(move |e| a(e) + b(e)),
                "-" => Box::new(move |e| a(e) - b(e)),
                _ => Box::new(move |e| a(e) * b(e)),
            };
            Ok((f, Ty::Real))
        }
        "/" => {
            if !(ta.numeric() && tb.numeric()) {
                return err(line, format!("operator not applicable to '{}' and '{}'", ta.name(), tb.name()));
            }
            Ok((Box::new(move |e| a(e) / b(e)), Ty::Real))
        }
        "div" | "mod" | "shl" | "shr" => {
            if !both_ord {
                return err(line, format!("operator '{op}' needs integer operands"));
            }
            let f: Ex = match op {
                "div" => Box::new(move |e| {
                    let d = b(e) as i64;
                    if d == 0 { 0.0 } else { fix((a(e) as i64).wrapping_div(d)) }
                }),
                "mod" => Box::new(move |e| {
                    let d = b(e) as i64;
                    if d == 0 { 0.0 } else { fix((a(e) as i64).wrapping_rem(d)) }
                }),
                "shl" => Box::new(move |e| {
                    if w { ((a(e) as i64 as i32).wrapping_shl(b(e) as u32)) as f64 } else { ((a(e) as i64).wrapping_shl(b(e) as u32)) as f64 }
                }),
                _ => Box::new(move |e| {
                    if w { ((a(e) as i64 as u32).wrapping_shr(b(e) as u32)) as i32 as f64 } else { ((a(e) as i64 as u64).wrapping_shr(b(e) as u32)) as i64 as f64 }
                }),
            };
            Ok((f, ity))
        }
        "and" | "or" | "xor" => {
            if ta == Ty::Bool && tb == Ty::Bool {
                // complete boolean evaluation is off ({$B-}): short-circuit
                let f: Ex = match op {
                    "and" => Box::new(move |e| if a(e) != 0.0 { (b(e) != 0.0) as i32 as f64 } else { 0.0 }),
                    "or" => Box::new(move |e| if a(e) != 0.0 { 1.0 } else { (b(e) != 0.0) as i32 as f64 }),
                    _ => Box::new(move |e| ((a(e) != 0.0) != (b(e) != 0.0)) as i32 as f64),
                };
                return Ok((f, Ty::Bool));
            }
            if !both_ord {
                return err(line, format!("operator '{op}' not applicable to '{}' and '{}'", ta.name(), tb.name()));
            }
            let f: Ex = match op {
                "and" => Box::new(move |e| fix((a(e) as i64) & (b(e) as i64))),
                "or" => Box::new(move |e| fix((a(e) as i64) | (b(e) as i64))),
                _ => Box::new(move |e| fix((a(e) as i64) ^ (b(e) as i64))),
            };
            Ok((f, ity))
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

// ---------------------------------------------------------------------------
// the compiled formula

/// A compiled `[SOURCE]` formula.
pub struct Program {
    pub name: String,
    body: St,
    nloc: usize,
    inits: Vec<(usize, Init)>,
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
        let line = c.line();
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
        let _ = line;

        c.decls()?;
        let body = c.block()?;
        c.eat_sym(";");
        if *c.peek() != Tok::Eof {
            if c.is_kw("procedure") || c.is_kw("function") {
                return err(c.line(), "only one procedure per formula is supported");
            }
            return err(c.line(), format!("unexpected {} after the end of the procedure", c.desc()));
        }
        Ok(Program { name, body, nloc: c.nloc, inits: c.inits, needs_buffer })
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
            l[*i] = match init {
                Init::Value(k) => values.get(*k).copied().unwrap_or(0.0),
                Init::Const(v) => *v,
                Init::Below(off) => {
                    let b = buf.as_ref().unwrap();
                    let o = crate::m3f::CONST_OFFSET - off;
                    f64::from_le_bytes(b[o..o + 8].try_into().unwrap())
                }
            };
        }
        let mut env = Env { l, it };
        (self.body)(&mut env);
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
