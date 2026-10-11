//! Number conversions as MB3D does them (DivUtils.pas), so the edits show
//! the same texts as in the original.

/// Delphi `FloatToStrF(v, ffGeneral, precision, 0)`: the shortest form with
/// up to `precision` significant digits; scientific notation ("1E-5",
/// "1.5E20") for very small or large values.
pub fn float_general(v: f64, precision: usize) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v.is_nan() { "NAN".into() } else if v.is_infinite() { if v > 0.0 { "INF".into() } else { "-INF".into() } } else { "0".into() };
    }
    let s = format!("{:.*e}", precision.saturating_sub(1), v);
    let (mant, exp) = s.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let sign = if neg { "-" } else { "" };
    if exp < -4 || exp >= precision as i32 {
        let (a, b) = digits.split_at(1);
        let m = if b.is_empty() { a.to_string() } else { format!("{a}.{b}") };
        let e = if exp < 0 { format!("-{:02}", -exp) } else { format!("{exp}") };
        let e = e.trim_start_matches('0').to_string();
        let e = if exp < 0 { format!("-{}", (-exp)) } else if e.is_empty() { "0".into() } else { e };
        return format!("{sign}{m}E{e}");
    }
    if exp < 0 {
        format!("{sign}0.{}{}", "0".repeat((-exp - 1) as usize), digits)
    } else {
        let int_len = exp as usize + 1;
        if digits.len() <= int_len {
            format!("{sign}{}{}", digits, "0".repeat(int_len - digits.len()))
        } else {
            format!("{sign}{}.{}", &digits[..int_len], &digits[int_len..])
        }
    }
}

/// `FloatToStr` (15 significant digits).
pub fn fts(v: f64) -> String {
    float_general(v, 15)
}

/// `FloatToStrSingle` (6 significant digits).
pub fn fts_single(v: f64) -> String {
    float_general(v as f32 as f64, 6)
}

/// `StrToFloatKtry`: accepts ',' as decimal separator and the factors
/// "pi" / "phi" ("2pi", "pi/2").
pub fn parse_float(s: &str) -> Option<f64> {
    let div = s.contains('/');
    let mut st = s.replace(['/', '*'], "");
    let mut d = 1.0;
    let up = st.to_ascii_uppercase();
    if up.contains("PHI") {
        #[allow(clippy::approx_constant)] // MB3D's digits of the golden ratio
        {
            d = 1.6180339887;
        }
        st = replace_ci(&st, "phi");
    }
    if st.to_ascii_uppercase().contains("PI") {
        d = std::f64::consts::PI;
        st = replace_ci(&st, "pi");
    }
    let st = st.trim();
    if st.is_empty() {
        return if d != 1.0 { Some(d) } else { None };
    }
    let v: f64 = st.replace(',', ".").parse().ok()?;
    Some(if div { v / d } else { v * d })
}

fn replace_ci(s: &str, pat: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(p) = lower[i..].find(pat) {
        out.push_str(&s[i..i + p]);
        i += p + pat.len();
    }
    out.push_str(&s[i..]);
    out
}

/// `StrToFloatK`: 0 when the text is no number.
pub fn ptf(s: &str) -> f64 {
    parse_float(s).unwrap_or(0.0)
}

/// `StrToIntTrim`.
pub fn pti(s: &str) -> i32 {
    let t: String = s.trim().chars().filter(|c| *c != ',' && *c != '.').collect();
    t.parse::<i64>().map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32).unwrap_or(0)
}

/// `IntToTimeStr` (tenths of seconds).
pub fn time_str(tenths: i64) -> String {
    if tenths >= 36000 {
        let s = tenths / 10;
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else if tenths > 0 {
        let s = tenths / 10;
        format!("{}:{:02}.{}", s / 60, s % 60, tenths % 10)
    } else {
        "-".into()
    }
}

/// `D2ByteToStr`: 0..250 as "0.00".."2.50".
pub fn d2byte_str(b: u8) -> String {
    let s = format!("{b:03}");
    format!("{}.{}", &s[..1], &s[1..])
}

pub fn d2byte(s: &str) -> u8 {
    (parse_float(s.trim()).unwrap_or(0.0).clamp(0.0, 2.5) * 100.0).round() as u8
}

/// A colour as Delphi stores it ($00BBGGRR) from our 0xAARRGGBB.
pub fn rgb_of(c: u32) -> [u8; 3] {
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

pub fn argb(c: [u8; 3]) -> u32 {
    0xFF00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delphi_float_texts() {
        assert_eq!(fts(0.5), "0.5");
        assert_eq!(fts(-2.0), "-2");
        assert_eq!(fts(30.0), "30");
        assert_eq!(fts(1e-5), "1E-5");
        assert_eq!(fts(1.5e-7), "1.5E-7");
        assert_eq!(fts(0.0001), "0.0001");
        assert_eq!(fts(123456.789), "123456.789");
        assert_eq!(fts_single(0.1), "0.1");
        assert_eq!(fts_single(1.0 / 3.0), "0.333333");
        assert_eq!(parse_float("2pi"), Some(2.0 * std::f64::consts::PI));
        assert_eq!(parse_float("0,5"), Some(0.5));
        assert_eq!(parse_float("x"), None);
        assert_eq!(time_str(125), "0:12.5");
        assert_eq!(d2byte_str(5), "0.05");
    }
}
