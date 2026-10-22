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

/// `StrToIntTrim`: 0 when the text is no whole number (MB3D's StrToInt
/// rejects "2.5" too).
pub fn pti(s: &str) -> i32 {
    s.trim().parse::<i64>().map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32).unwrap_or(0)
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

/// A time left, rounded: "40 s", "3 min", "2 h 5 min".
pub fn about_str(secs: f64) -> String {
    let s = secs.round() as i64;
    match s {
        ..=59 => format!("{} s", s.max(1)),
        60..=3599 => format!("{} min", (s + 30) / 60),
        _ => format!("{} h {} min", s / 3600, s % 3600 / 60),
    }
}

/// A time left as shown, smoothed: `shown` the last one and when, `est`
/// the new estimate; it counts down meanwhile and moves toward the
/// estimate over about `tau` seconds (at once when it is twice as far or
/// half as far).
pub fn smooth_left(shown: Option<(f64, std::time::Instant)>, est: f64, tau: f64) -> (f64, std::time::Instant) {
    let now = std::time::Instant::now();
    let v = match shown {
        Some((v, t)) => {
            let dt = (now - t).as_secs_f64();
            let was = (v - dt).max(0.0);
            // (far off, e.g. once the graphics card joined: at once)
            if est > 2.0 * was || est < 0.5 * was {
                est
            } else {
                was + (est - was) * (1.0 - (-dt / tau).exp())
            }
        }
        None => est,
    };
    (v, now)
}

/// A time left as the window shows it, and when.
pub type Shown = (f64, std::time::Instant);

/// A time left as shown, following the estimates the way a download's
/// does (Firefox's `getTimeLeft`): `prev` the last one shown and when,
/// `est` the new estimate (see [`shown_step`]).
pub fn smooth_shown(prev: Option<Shown>, est: f64, now: std::time::Instant) -> Shown {
    let v = match prev {
        Some((s, t)) => shown_step(s, est, now.saturating_duration_since(t).as_secs_f64()),
        None => est,
    };
    (v, now)
}

/// The time left shown `dt` seconds after `shown` was, with the new
/// estimate `est`: a drop to half or less is taken at once; a change under
/// 5 seconds and 5 % is not, it counts down with the clock instead (holds,
/// when the estimate is the larger: it must not fall behind); else it
/// moves part of the way (30 % per second downward, 10 % upward: a long
/// render is more often slower than predicted than faster; the estimate
/// itself counts down, so the shown value follows it without adding the
/// clock, which would outrun a rising estimate).
pub fn shown_step(shown: f64, est: f64, dt: f64) -> f64 {
    let d = est - shown;
    if est <= shown / 2.0 {
        est
    } else if d.abs() < 5.0 || d.abs() < 0.05 * shown {
        if d > 0.0 { shown } else { (shown - dt.max(0.0)).max(0.0) }
    } else if d < 0.0 {
        shown + d * (1.0 - 0.7f64.powf(dt))
    } else {
        shown + d * (1.0 - 0.9f64.powf(dt))
    }
}

/// A time left in words: "a few seconds left", "12 s left", "2 min 30 s
/// left", "about 7 min left" (from 4 minutes), "about 1 h 20 min left".
pub fn left_str(secs: f64) -> String {
    let s = secs.round().max(0.0) as i64;
    match s {
        ..=3 => "a few seconds left".into(),
        4..=59 => format!("{s} s left"),
        60..=239 if s % 60 == 0 => format!("{} min left", s / 60),
        60..=239 => format!("{} min {} s left", s / 60, s % 60),
        240..=3599 => format!("about {} min left", (s + 30) / 60),
        _ => {
            let m = (s + 30) / 60;
            match m % 60 {
                0 => format!("about {} h left", m / 60),
                r => format!("about {} h {r} min left", m / 60),
            }
        }
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
        assert_eq!(pti(" 640 "), 640);
        assert_eq!(pti("-3"), -3);
        assert_eq!(pti("2.5"), 0);
        assert_eq!(pti("640,5"), 0);
    }

    #[test]
    fn shown_time_left_follows_a_download_display() {
        // a drop to half or less: at once
        assert_eq!(shown_step(100.0, 50.0, 0.0), 50.0);
        assert_eq!(shown_step(100.0, 20.0, 1.0), 20.0);
        // an increase: damped, a tenth of the way per second
        assert!((shown_step(100.0, 200.0, 1.0) - 110.0).abs() < 1e-9);
        assert!(shown_step(100.0, 200.0, 3.0) < shown_step(100.0, 200.0, 5.0));
        // a decrease short of half: faster, 30 % per second
        assert!((shown_step(100.0, 60.0, 1.0) - 88.0).abs() < 1e-9);
        // a small change (under 5 s or 5 %): only counting down, or holding
        assert_eq!(shown_step(100.0, 103.0, 1.0), 100.0);
        assert_eq!(shown_step(100.0, 96.0, 1.0), 99.0);
        assert_eq!(shown_step(1000.0, 1040.0, 2.0), 1000.0);
        assert_eq!(shown_step(1000.0, 960.0, 2.0), 998.0);
        // never up by less than 5 %
        for est in [101.0, 103.0, 104.0] {
            assert!(shown_step(100.0, est, 0.5) <= 100.0, "{est}");
        }
        // an estimate that stays above what is shown is reached in time
        let mut s = 0.0;
        for _ in 0..40 {
            s = shown_step(s, 10.0, 0.5);
        }
        assert!(s > 5.0, "{s}");
        // and never below zero
        assert_eq!(shown_step(1.0, 0.0, 5.0), 0.0);
        assert_eq!(shown_step(1.0, 2.0, 5.0), 1.0);
        let now = std::time::Instant::now();
        assert_eq!(smooth_shown(None, 42.0, now), (42.0, now));
    }

    #[test]
    fn time_left_words() {
        assert_eq!(left_str(0.0), "a few seconds left");
        assert_eq!(left_str(3.4), "a few seconds left");
        assert_eq!(left_str(3.6), "4 s left");
        assert_eq!(left_str(59.0), "59 s left");
        assert_eq!(left_str(60.0), "1 min left");
        assert_eq!(left_str(150.0), "2 min 30 s left");
        assert_eq!(left_str(239.0), "3 min 59 s left");
        assert_eq!(left_str(240.0), "about 4 min left");
        assert_eq!(left_str(449.0), "about 7 min left");
        assert_eq!(left_str(3599.0), "about 60 min left");
        assert_eq!(left_str(3600.0), "about 1 h left");
        assert_eq!(left_str(4800.0), "about 1 h 20 min left");
        assert_eq!(left_str(7199.0), "about 2 h left");
    }
}
