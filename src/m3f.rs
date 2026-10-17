//! Loader for Mandelbulb3D custom formula files (`.m3f`), ported from
//! `LoadCustomFormula` and `FillCustomVBufWithVars` (CustomFormulas.pas).
//!
//! A formula consists of an `[OPTIONS]` block (meta data and user
//! variables), an optional `[CONSTANTS]` block and the `[CODE]` block with
//! 32-bit x86 machine code, which is run by the interpreter in `x86.rs`.

use std::path::{Path, PathBuf};

/// Option types (`byOptionTypes`), index = MB3D type number.
const OPTION_TYPES: &str = ".DOUBLE.SINGLE.INTEGER.DOUBLEANGLE.SINGLEANGLE.3DOUBLEANGLES.3SINGLEANGLES.BOXSCALE.FOLDING.DSQUARE.NOVARIABLE.FOLDING16.6SINGLEANGLES.DRECIPRO.2DOUBLES..DSQRRECI..2SINGLES..4SINGLES..3SCALESANGLES.SCALESROT.2INTEGER..SRECI2....DRECI2.";

/// Bytes of the option buffer used per option type (`MemNeeded`).
pub const MEM_NEEDED: [usize; 23] = [8, 4, 4, 16, 8, 72, 36, 16, 40, 8, 0, 32, 64, 8, 16, 8, 8, 16, 36, 8, 8, 8, 16];

/// Offset of `pConstPointer16` inside the 1024 byte variable buffer.
pub const CONST_OFFSET: usize = 256;

/// Delphi `StrToInt64`: decimal, or hexadecimal with a `$` (or `0x`) prefix.
fn parse_int(s: &str) -> i64 {
    let s = s.trim();
    let (neg, t) = match s.strip_prefix('-') {
        Some(t) => (true, t),
        None => (false, s),
    };
    let v = if let Some(h) = t.strip_prefix('$').or_else(|| t.strip_prefix("0x")).or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).map(|v| v as i64).unwrap_or(0)
    } else {
        t.parse::<i64>().unwrap_or(0)
    };
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}
pub const VAR_BUFFER_SIZE: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct OptionDef {
    pub ty: u8,
    pub name: String,
    pub default: f64,
}

#[derive(Clone, Debug)]
pub struct M3f {
    pub name: String,
    pub version: i32,
    pub de_option: i32,
    pub de_scale: f64,
    pub ade_scale: f64,
    pub si_pow: f64,
    pub rstop: f64,
    pub simd_level: u32,
    pub options: Vec<OptionDef>,
    /// The constant area (`[CONSTANTS]`), placed at pConstPointer16.
    pub consts: Vec<u8>,
    pub code: Vec<u8>,
    pub description: String,
    /// `[SOURCE]` formulas: the Pascal source, compiled
    pub source: String,
    pub jit: Option<std::sync::Arc<crate::jit::Program>>,
    /// names of the options as MB3D's JIT preprocessor sees them
    pub param_names: Vec<String>,
    /// named constants (`.Double name = value`)
    pub const_defs: Vec<crate::jit::ConstDef>,
}

/// `StrFirstWord`: text up to '=' and then up to the first space.
fn first_word(s: &str) -> &str {
    let s = s.trim();
    let s = s.split('=').next().unwrap_or("");
    s.split(' ').next().unwrap_or("").trim()
}
/// `StrSecondWord`
fn second_word(s: &str) -> String {
    let s = s.trim();
    let s = match s.find(' ') {
        Some(i) => &s[i + 1..],
        None => s,
    };
    s.split('=').next().unwrap_or("").trim().to_string()
}
/// `StrLastWord`: after the last '=', then after the last space.
fn last_word(s: &str) -> &str {
    let s = s.rsplit('=').next().unwrap_or("").trim();
    s.rsplit(' ').next().unwrap_or("")
}
/// `StrFirstWordAfterEqual`
fn first_word_after_equal(s: &str) -> &str {
    let s = s.rsplit('=').next().unwrap_or("");
    first_word(s)
}
fn up5(s: &str) -> String {
    s.chars().take(5).collect::<String>().to_uppercase()
}
fn parse_f(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok()
}

impl M3f {
    pub fn load(path: &Path) -> Result<M3f, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        // the files are ANSI (latin-1)
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Self::parse(&name, &text)
    }

    pub fn parse(name: &str, text: &str) -> Result<M3f, String> {
        let mut f = M3f {
            name: name.to_string(),
            version: 0,
            de_option: 0,
            de_scale: 1.0,
            ade_scale: 1.0,
            si_pow: 0.0,
            rstop: 16.0,
            simd_level: 0,
            options: Vec::new(),
            consts: Vec::new(),
            code: Vec::new(),
            description: String::new(),
            source: String::new(),
            jit: None,
            param_names: Vec::new(),
            const_defs: Vec::new(),
        };
        let mut lines = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).peekable();
        let mut s = lines.next().unwrap_or("").to_string();
        if s.eq_ignore_ascii_case("[OPTIONS]") {
            loop {
                s = match lines.next() {
                    Some(l) => l.to_string(),
                    None => break,
                };
                let key = up5(&s);
                let known = ".SSE2.DESC.SIPO.RSTO.SSE3.SSSE.SSE4.VERS.DEOP.DIFS.ADES.";
                let n = known.find(&key).map(|i| i + 1).unwrap_or(0);
                let num = || parse_f(last_word(&s));
                match n {
                    1 => f.simd_level |= 1,
                    6 => f.de_scale = num().unwrap_or(1.0),
                    11 => f.si_pow = num().unwrap_or(0.0),
                    16 => f.rstop = num().unwrap_or(16.0),
                    21 => f.simd_level |= 2,
                    26 => f.simd_level |= 4,
                    31 => f.simd_level |= 8,
                    36 => f.version = num().unwrap_or(0.0) as i32,
                    41 => f.de_option = num().unwrap_or(0.0) as i32,
                    46 => {}
                    51 => f.ade_scale = num().unwrap_or(1.0),
                    _ => {
                        let w = first_word(&s).to_uppercase();
                        let mut n = if w.is_empty() { 0 } else { OPTION_TYPES.find(&w).map(|i| i + 1).unwrap_or(0) };
                        if n == 0 {
                            break; // end of the options block
                        }
                        if n > 50 {
                            n -= 6;
                        }
                        if n > 130 {
                            n -= 6;
                        }
                        if n > 180 {
                            n -= 4;
                        }
                        if f.options.len() >= 16 {
                            continue;
                        }
                        let ty = ((n + 8) / 10) as u8;
                        let val = num().unwrap_or(0.0);
                        let oname = second_word(&s);
                        if ty != 12 {
                            f.param_names.push(oname.clone());
                        }
                        match ty {
                            12 => {
                                for sfx in [" YZ", " XZ", " XY", " XW", " YW", " ZW"] {
                                    f.options.push(OptionDef { ty, name: format!("{oname}{sfx}"), default: val });
                                }
                            }
                            5 | 6 => {
                                for sfx in [" X", " Y", " Z"] {
                                    f.options.push(OptionDef { ty, name: format!("{oname}{sfx}"), default: val });
                                }
                            }
                            18 => {
                                f.options.push(OptionDef { ty, name: oname, default: val });
                                for c in ['A', 'B', 'C'] {
                                    f.options.push(OptionDef { ty: 6, name: format!("Rotation {c}"), default: 0.0 });
                                }
                            }
                            19 => {
                                let scale = parse_f(first_word_after_equal(&s)).unwrap_or(1.0);
                                f.options.push(OptionDef { ty, name: format!("Scale {oname}"), default: scale });
                                f.options.push(OptionDef { ty: 6, name: format!("Rotation {oname}"), default: val });
                            }
                            _ => f.options.push(OptionDef { ty, name: oname, default: val }),
                        }
                        f.options.truncate(16);
                    }
                }
            }
        }
        if !(2..=9).contains(&f.version) {
            return Err(format!("{name}: unsupported formula version {}", f.version));
        }
        if s.eq_ignore_ascii_case("[CONSTANTS]") {
            let mut consts: Vec<u8> = Vec::new();
            loop {
                s = match lines.next() {
                    Some(l) => l.to_string(),
                    None => break,
                };
                if s.starts_with('[') {
                    break;
                }
                let body = if s.len() > 1 && s.starts_with('.') { &s[1..] } else { &s[..] };
                let n = "DOUBLINTINT64SINGLE".find(&up5(body)).map(|i| i + 1).unwrap_or(0);
                let mut slot = [0u8; 8];
                let lw = last_word(&s);
                let cname = if s.len() > 1 && s.starts_with('.') {
                    second_word(&s)
                } else {
                    // TJITFormula.NextValueName
                    let mut k = 0;
                    while f.const_defs.iter().any(|c| c.name == format!("Const{k}")) {
                        k += 1;
                    }
                    format!("Const{k}")
                };
                match n {
                    1 => slot = parse_f(lw).unwrap_or(0.0).to_le_bytes(),
                    6 => slot[..4].copy_from_slice(&(parse_int(lw) as i32).to_le_bytes()),
                    9 => slot = parse_int(lw).to_le_bytes(),
                    14 => slot[..4].copy_from_slice(&(parse_f(lw).unwrap_or(0.0) as f32).to_le_bytes()),
                    _ => continue,
                }
                let ty = match n {
                    6 => 1,
                    9 => 2,
                    14 => 3,
                    _ => 0,
                };
                f.const_defs.push(crate::jit::ConstDef { name: cname, ty });
                consts.extend_from_slice(&slot);
            }
            f.consts = consts;
        }
        if s.eq_ignore_ascii_case("[SOURCE]") {
            // the raw lines up to the next section (comments and line numbers intact)
            let raw: Vec<&str> = text.lines().collect();
            let start = raw.iter().position(|l| l.trim().eq_ignore_ascii_case("[SOURCE]")).unwrap_or(0) + 1;
            let mut src = Vec::new();
            let mut end = raw.len();
            for (k, l) in raw.iter().enumerate().skip(start) {
                let t = l.trim();
                if t.len() > 1 && t.starts_with('[') && t.ends_with(']') {
                    end = k;
                    break;
                }
                src.push(*l);
            }
            f.source = src.join("\n");
            f.description = raw.get(end + 1..).map(|r| r.join("\n")).unwrap_or_default();
            if f.consts.len() > VAR_BUFFER_SIZE - CONST_OFFSET {
                f.consts.truncate(VAR_BUFFER_SIZE - CONST_OFFSET);
            }
            let prog = crate::jit::Program::compile(&f, &f.source).map_err(|e| format!("{name}: {e}"))?;
            f.jit = Some(std::sync::Arc::new(prog));
            return Ok(f);
        }
        if s.eq_ignore_ascii_case("[CODE]") {
            let mut code = Vec::new();
            for l in lines.by_ref() {
                if l.eq_ignore_ascii_case("[END]") {
                    break;
                }
                if l.starts_with('[') {
                    continue;
                }
                let b = l.as_bytes();
                let mut i = 0;
                while i + 1 < b.len() && code.len() < 4095 {
                    let h = std::str::from_utf8(&b[i..i + 2]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                    match h {
                        Some(v) => code.push(v),
                        None => return Err(format!("{name}: bad hex in [CODE]")),
                    }
                    i += 2;
                }
            }
            f.code = code;
            f.description = lines.collect::<Vec<_>>().join("\n");
        }
        if f.code.len() <= 10 {
            return Err(format!("{name}: no machine code"));
        }
        if f.consts.len() > VAR_BUFFER_SIZE - CONST_OFFSET {
            f.consts.truncate(VAR_BUFFER_SIZE - CONST_OFFSET);
        }
        Ok(f)
    }

    /// The option defaults.
    pub fn defaults(&self) -> Vec<f64> {
        self.options.iter().map(|o| o.default).collect()
    }

    /// `FillCustomVBufWithVars`: builds the 1024 byte variable buffer.
    /// `int_pow_fn(n)` returns the (emulated) address of the internal integer
    /// power function n (used by the FOLDING option type).
    pub fn var_buffer(&self, values: &[f64], int_pow_fn: &dyn Fn(i32) -> u32) -> Vec<u8> {
        let mut buf = vec![0u8; VAR_BUFFER_SIZE];
        buf[CONST_OFFSET..CONST_OFFSET + self.consts.len()].copy_from_slice(&self.consts);
        let mut p = CONST_OFFSET; // byte offset, decreasing
        let mut wr_d = |p: &mut usize, v: f64, buf: &mut Vec<u8>| {
            *p -= 8;
            buf[*p..*p + 8].copy_from_slice(&v.to_le_bytes());
        };
        let wr_s = |p: &mut usize, v: f32, buf: &mut Vec<u8>| {
            *p -= 4;
            buf[*p..*p + 4].copy_from_slice(&v.to_le_bytes());
        };
        let wr_i = |p: &mut usize, v: i32, buf: &mut Vec<u8>| {
            *p -= 4;
            buf[*p..*p + 4].copy_from_slice(&v.to_le_bytes());
        };
        let val = |i: usize| -> f64 { values.get(i).copied().unwrap_or(0.0) };
        wr_d(&mut p, 0.5, &mut buf);
        let n = self.options.len().min(16);
        let d2r = std::f64::consts::PI / 180.0;
        let mut i = 0;
        while i < n {
            let ty = self.options[i].ty as usize;
            let need = MEM_NEEDED.get(ty).copied().unwrap_or(0);
            if CONST_OFFSET - p + need < 257 {
                let v = val(i);
                match ty {
                    0 => wr_d(&mut p, v, &mut buf),
                    1 => wr_s(&mut p, v as f32, &mut buf),
                    2 => wr_i(&mut p, v.round_ties_even() as i32, &mut buf),
                    3 => {
                        wr_d(&mut p, (v * d2r).sin(), &mut buf);
                        wr_d(&mut p, (v * d2r).cos(), &mut buf);
                    }
                    4 => {
                        wr_s(&mut p, (v * d2r).sin() as f32, &mut buf);
                        wr_s(&mut p, (v * d2r).cos() as f32, &mut buf);
                    }
                    5 | 6 => {
                        let m = build_rot_matrix(v * d2r, val(i + 1) * d2r, val(i + 2) * d2r);
                        for r in m.iter() {
                            for &e in r.iter() {
                                if ty == 5 {
                                    wr_d(&mut p, e, &mut buf);
                                } else {
                                    wr_s(&mut p, e as f32, &mut buf);
                                }
                            }
                        }
                        i += 2;
                    }
                    7 => {
                        let mr = f64::max(1e-40, v);
                        wr_d(&mut p, val(i.wrapping_sub(1)) / (mr * mr), &mut buf);
                        wr_d(&mut p, mr * mr, &mut buf);
                    }
                    8 => {
                        wr_d(&mut p, v, &mut buf);
                        wr_d(&mut p, 2.0 * v, &mut buf);
                        wr_d(&mut p, -v, &mut buf);
                        wr_d(&mut p, -2.0 * v, &mut buf);
                        p -= 4;
                        if i > 1 {
                            let n = (val(i - 2).round_ties_even() as i32).clamp(2, 8);
                            buf[p..p + 4].copy_from_slice(&int_pow_fn(n).to_le_bytes());
                        }
                    }
                    9 => wr_d(&mut p, v * v, &mut buf),
                    11 => {
                        wr_d(&mut p, v, &mut buf);
                        wr_d(&mut p, v, &mut buf);
                        wr_d(&mut p, -v, &mut buf);
                        wr_d(&mut p, -v, &mut buf);
                    }
                    12 => {
                        let a: Vec<f64> = (0..6).map(|j| val(i + j) * d2r).collect();
                        let m = build_rot_matrix_4d(&a);
                        for r in m.iter() {
                            for &e in r.iter() {
                                wr_s(&mut p, e, &mut buf);
                            }
                        }
                        i += 5;
                    }
                    13 => {
                        let d = if v.abs() < 1e-40 { if v < 0.0 { -1e-40 } else { 1e-40 } } else { v };
                        wr_d(&mut p, 1.0 / d, &mut buf);
                    }
                    14 => {
                        wr_d(&mut p, v, &mut buf);
                        wr_d(&mut p, v, &mut buf);
                    }
                    15 => wr_d(&mut p, 1.0 / f64::max(1e-40, v * v), &mut buf),
                    16 | 17 => {
                        let l = if ty == 17 { 4 } else { 2 };
                        for _ in 0..l {
                            wr_s(&mut p, v as f32, &mut buf);
                        }
                    }
                    18 => {
                        let mut m = build_rot_matrix(val(i + 1) * d2r, val(i + 2) * d2r, val(i + 3) * d2r);
                        for r in m.iter_mut() {
                            for e in r.iter_mut() {
                                *e *= v;
                            }
                        }
                        for r in m.iter() {
                            for &e in r.iter() {
                                wr_s(&mut p, e as f32, &mut buf);
                            }
                        }
                        i += 3;
                    }
                    19 => {
                        wr_s(&mut p, ((val(i + 1) * d2r).sin() * v) as f32, &mut buf);
                        wr_s(&mut p, ((val(i + 1) * d2r).cos() * v) as f32, &mut buf);
                        i += 1;
                    }
                    20 => {
                        wr_i(&mut p, v.round_ties_even() as i32, &mut buf);
                        wr_i(&mut p, v.round_ties_even() as i32, &mut buf);
                    }
                    21 => {
                        wr_s(&mut p, v as f32, &mut buf);
                        wr_s(&mut p, 1.0 / f32::max(1e-30, v as f32), &mut buf);
                    }
                    22 => {
                        wr_d(&mut p, v, &mut buf);
                        wr_d(&mut p, 1.0 / f64::max(1e-40, v), &mut buf);
                    }
                    _ => {} // 10: no variable
                }
            }
            i += 1;
        }
        let _ = &mut wr_d;
        buf
    }
}

/// `BuildRotMatrix`
pub fn build_rot_matrix(xa: f64, ya: f64, za: f64) -> [[f64; 3]; 3] {
    let (sx, cx) = xa.sin_cos();
    let (sy, cy) = ya.sin_cos();
    let (sz, cz) = za.sin_cos();
    [
        [cy * cz, -cy * sz, sy],
        [sx * sy * cz + cx * sz, cx * cz - sx * sy * sz, -sx * cy],
        [sx * sz - cx * sy * cz, cx * sy * sz + sx * cz, cx * cy],
    ]
}

/// `BuildRotMatrix4d` (single precision matrix, angles yz xz xy xw yw zw)
pub fn build_rot_matrix_4d(angles: &[f64]) -> [[f32; 4]; 4] {
    let i1 = [1, 0, 0, 0, 1, 2];
    let i2 = [2, 2, 1, 3, 3, 3];
    let id = || {
        let mut m = [[0f32; 4]; 4];
        for (k, r) in m.iter_mut().enumerate() {
            r[k] = 1.0;
        }
        m
    };
    let mut ms4 = id();
    for i in 0..6 {
        let (s1, c1) = angles.get(i).copied().unwrap_or(0.0).sin_cos();
        let mut sm4 = id();
        sm4[i1[i]][i1[i]] = c1 as f32;
        sm4[i2[i]][i2[i]] = c1 as f32;
        sm4[i1[i]][i2[i]] = -s1 as f32;
        sm4[i2[i]][i1[i]] = s1 as f32;
        // Multiply2SMatrix4(@SM4, @ms4); ms4 := SM4  ->  ms4 = SM4 * ms4
        let mut r = [[0f32; 4]; 4];
        for a in 0..4 {
            for b in 0..4 {
                r[a][b] = (0..4).map(|k| sm4[a][k] * ms4[k][b]).sum();
            }
        }
        ms4 = r;
    }
    ms4
}

/// Finds `<name>.m3f` (case insensitive) in the given directories.
pub fn find_formula(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    // a formula is a name, never a path (names come from parameter files
    // and swarm jobs)
    if name.contains(['/', '\\', ':']) || name.trim().starts_with('.') {
        return None;
    }
    let want = format!("{}.m3f", name.trim()).to_lowercase();
    for d in dirs {
        let p = d.join(format!("{}.m3f", name.trim()));
        if p.is_file() {
            return Some(p);
        }
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                if e.file_name().to_string_lossy().to_lowercase() == want {
                    return Some(e.path());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mandy_cousin() {
        let t = "[OPTIONS]\n.Version = 5\n.DEoption = 4\n.DEscale = 1\n.SIpower = 2\n.Double X mul = 1\n.Double Y mul = 2\n[CODE]\n558BEC56578B75088B7E30DD00\n5F5E5DC20800\n[END]\n\nDescription:\nhello";
        let f = M3f::parse("MandyCousin", t).unwrap();
        assert_eq!(f.version, 5);
        assert_eq!(f.de_option, 4);
        assert_eq!(f.options.len(), 2);
        assert_eq!(f.options[1].name, "Y mul");
        assert_eq!(f.options[1].default, 2.0);
        let b = f.var_buffer(&f.defaults(), &|_| 0);
        let rd = |o: usize| f64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        assert_eq!(rd(CONST_OFFSET - 8), 0.5);
        assert_eq!(rd(CONST_OFFSET - 16), 1.0);
        assert_eq!(rd(CONST_OFFSET - 24), 2.0);
    }
}
