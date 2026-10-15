//! Parameters from before MB3D 1.6 (MandId < 20): the header `TMandHeader8`
//! (704 bytes, with the light record `TLightingParas8`) and its
//! predecessors `TMandHeader4` and `TMandHeaderOld`, converted to
//! `TMandHeader10` as `LoadParameter`, `GetHeaderFromText` and
//! `TAnimationForm.LoadAni` do it (`ConvertHeaderFromOldParas`,
//! `ConvertLight8to9` and friends in FileHandling.pas).
//!
//! The result keeps the old MandId, so [`crate::m3p::parse`] applies the
//! upgrades of the later versions (`UpdateFormulaOptionAbove20`,
//! `UpdateLightParasAbove3`) as for any other file.  Where MB3D works on
//! memory it did not initialise (the formula block of the scene loaded
//! before, a fresh stack record), this starts from zeros.

use crate::m3p::{ADDON_SIZE, HEADER_SIZE};

/// `SizeOf(TMandHeader9)` = `SizeOf(TMandHeader8)`
pub(crate) const OLD_HEADER_SIZE: usize = 704;
/// `SizeOf(THeaderCustomAddonOld)`
const OLD_ADDON_SIZE: usize = 1016;
/// `TLightingParas8` in `TMandHeader8`
const L8: usize = 432;

fn gi32(d: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn gu32(d: &[u8], o: usize) -> u32 {
    gi32(d, o) as u32
}
fn gf64(d: &[u8], o: usize) -> f64 {
    f64::from_le_bytes(d[o..o + 8].try_into().unwrap())
}
fn gf32(d: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}
fn put(d: &mut [u8], o: usize, b: &[u8]) {
    d[o..o + b.len()].copy_from_slice(b);
}
fn pi32(d: &mut [u8], o: usize, v: i32) {
    put(d, o, &v.to_le_bytes());
}
fn pu32(d: &mut [u8], o: usize, v: u32) {
    put(d, o, &v.to_le_bytes());
}
fn pf64(d: &mut [u8], o: usize, v: f64) {
    put(d, o, &v.to_le_bytes());
}
fn pf32(d: &mut [u8], o: usize, v: f32) {
    put(d, o, &v.to_le_bytes());
}
/// Delphi's `Round` (banker's rounding) stored in an `Integer`
fn round(v: f64) -> i32 {
    v.round_ties_even() as i64 as i32
}

/// The size of the header record a file of this MandId starts with
/// (`LoadParameter`, `LoadFullM3I`).
fn stored_header_size(mand_id: i32) -> usize {
    match mand_id {
        ..=3 => 200,  // TMandHeaderOld
        4..=6 => 272, // TMandHeader4
        7 => 580,     // TMandHeader9 without LCols and ICols
        _ => OLD_HEADER_SIZE,
    }
}

/// `LoadParameter` for MandId < 20 (`.m3p` and `.m3i`): the old header,
/// converted, followed by the formula block (stored behind the image from
/// MandId 16 on, built from the header before).
pub(crate) fn raw_from_old_file(data: &[u8]) -> Result<Vec<u8>, String> {
    let mid = gi32(data, 0);
    if !(0..20).contains(&mid) || data.len() < 100 {
        return Err("not a MB3D parameter file".into());
    }
    let mut p8 = read_old_header(data)?;
    let mut addon = vec![0u8; ADDON_SIZE];
    convert_header_from_old_paras(&mut p8, &mut addon)?;
    let mut raw = header10_from_old(&p8, true);
    if mid > 15 {
        // the G-buffer of an .m3i image (TsiLight3/4/5) lies between
        let (w, h) = (gi32(&p8, 4).max(0) as usize, gi32(&p8, 8).max(0) as usize);
        let px = match mid {
            16 | 17 => 12,
            18 => 16,
            _ => 18,
        };
        let mut pos = OLD_HEADER_SIZE;
        if data.len() >= pos + w * h * px + OLD_ADDON_SIZE {
            pos += w * h * px;
        }
        load_haddon(data.get(pos..).unwrap_or(&[]), &mut addon);
    }
    finish_addon(&mut raw, &addon, mid);
    Ok(raw)
}

/// The header part of `LoadParameter` for MandId < 20: reads `TMandHeaderOld`
/// (MandId < 4), `TMandHeader4` (< 7) or `TMandHeader9` and brings the
/// first two to the `TMandHeader8` layout.
fn read_old_header(data: &[u8]) -> Result<Vec<u8>, String> {
    let mid = gi32(data, 0);
    let n = stored_header_size(mid);
    if data.len() < n {
        return Err(format!("parameter file of version {mid} is truncated"));
    }
    let mut p8 = vec![0u8; OLD_HEADER_SIZE];
    if mid >= 7 {
        p8[..n].copy_from_slice(&data[..n]);
        if mid < 8 {
            convert_from_old_light_paras(&mut p8[L8..], 148);
        }
        return Ok(p8);
    }
    // TMandHeader4 (the fields after the light record are 0 for MandId < 4)
    let mut h4 = vec![0u8; 272];
    h4[..n].copy_from_slice(&data[..n]);
    let cols = 108 + 56; // Light.Cols[1]
    if mid < 5 && gf64(&h4, 20) > gf64(&h4, 28) {
        let (zs, ze) = (gf64(&h4, 20), gf64(&h4, 28));
        pf64(&mut h4, 20, -zs);
        pf64(&mut h4, 28, -ze);
        let (xr, yr) = (gf64(&h4, 60), gf64(&h4, 68));
        let pid180 = std::f64::consts::PI / 180.0;
        pf64(&mut h4, 60, yr * pid180);
        pf64(&mut h4, 68, xr * pid180);
        // Cols[7..9] := Cols[4..6]
        let c = h4[cols + 12..cols + 24].to_vec();
        put(&mut h4, cols + 24, &c);
    }
    if mid < 5 {
        let zmid = (gf64(&h4, 20) + gf64(&h4, 28)) * 0.5;
        pf64(&mut h4, 52, zmid);
    } else if mid == 5 && gi32(&h4, 16) & 15 == 10 {
        let o = gi32(&h4, 16);
        pf64(&mut h4, 100, ((o >> 18) & 3) as f64);
        pi32(&mut h4, 16, (o & 0x7FFC_3FFF) | ((o & 0x38000) >> 1));
    }
    if mid < 6 {
        h4[255] = 1; // bHScalculated
    }
    let tbo = gu32(&h4, 108 + 44) | 43 << 11;
    pu32(&mut h4, 108 + 44, tbo);
    convert_4_to_7_paras(&h4, &mut p8);
    // bCalc3D is not in these records (MB3D reads what its stack holds,
    // in practice not 0): a 3D calculation, not a 2D slice
    p8[344] = 1;
    Ok(p8)
}

/// `Convert4to7paras`
fn convert_4_to_7_paras(h4: &[u8], p8: &mut [u8]) {
    p8[..108].copy_from_slice(&h4[..108]);
    p8[L8..L8 + 92].copy_from_slice(&h4[108..200]); // TLightingParas
    p8[108..180].copy_from_slice(&h4[200..272]); // dFOVy..bCutOption
    convert_from_old_light_paras(&mut p8[L8..], 92);
    pf32(p8, 177, ((((gi32(h4, 16) >> 10) & 15) + 1) as f64 * 0.2) as f32); // sDEstop
}

/// `ConvertFromOldLightParas`: lights for records without them
/// (`TLightingParas`, 92 bytes), then `UpdateLightVersion8`.
fn convert_from_old_light_paras(l8: &mut [u8], size: usize) {
    if size < 148 {
        for i in 0..4 {
            // defaultLight7
            let o = 92 + i * 14;
            l8[o] = 1;
            l8[o + 1] = 3;
            pu32(l8, o + 2, 0xFF_FFFF);
            pi32(l8, o + 6, 0);
            pi32(l8, o + 10, 0);
        }
        l8[92] = 0;
        l8[93] = gu32(l8, 44) as u8;
        // TBpos[1] * -32768 is an integer product (it overflows)
        let ax = round(gi32(l8, 0).wrapping_mul(-32768) as f64 / 360.0);
        let ay = round(gi32(l8, 4).wrapping_mul(32768) as f64 / 360.0);
        pi32(l8, 98, ax);
        pi32(l8, 102, ay);
    }
    update_light_version8(l8);
}

/// `InterpolateColor`
fn interpolate_color(c1: u32, c2: u32, w1: f64, w2: f64) -> u32 {
    let ch = |m: u32| round((c1 & m) as f64 * w1 + (c2 & m) as f64 * w2) as u32;
    ch(0xFF) | (ch(0xFF00) & 0xFF00) | (ch(0xFF_0000) & 0xFF_0000)
}

/// `InterpolateColorB`
fn interpolate_color_b(c1: u32, c2: u32, c3: u32, w: f64) -> u32 {
    let ch = |m: u32| round((c1 & m) as f64 * w + (c2 & m) as f64 * w + (c3 & m) as f64 * w) as u32;
    ch(0xFF) | (ch(0xFF00) & 0xFF00) | (ch(0xFF_0000) & 0xFF_0000)
}

/// `UpdateLightVersion8`: the surface and interior colours of light
/// version 2 from the 9 object colours of the versions before.
fn update_light_version8(l8: &mut [u8]) {
    let tb = |k: usize| (k - 1) * 4; // TBpos[k]
    let col = |k: usize| 56 + (k - 1) * 4; // Cols[k]
    let lcol = |i: usize| 148 + i * 10;
    let icol = |i: usize| 248 + i * 6;
    pi32(l8, tb(3), 0x80_0080);
    pi32(l8, tb(6), 53);
    let v = (gu32(l8, 44) >> 20) & 7;
    if v >= 2 {
        return;
    }
    if v < 1 {
        let w1 = gi32(l8, tb(5)) as f64;
        let w2 = (gi32(l8, tb(5)) + gi32(l8, tb(8))) as f64;
        let w1 = w1 / w2.max(1.0);
        let w2 = 1.0 - w1;
        for i in 0..3 {
            // average the diffuse colours with the ambient ones
            let c = interpolate_color(gu32(l8, col(i * 3 + 1)), gu32(l8, col(i * 3 + 3)), w1, w2);
            pu32(l8, col(i * 3 + 1), c);
        }
    }
    for i in 0..3 {
        let p = (i * 10922) as u16;
        put(l8, lcol(i), &p.to_le_bytes());
        let (dif, spe) = (gu32(l8, col(i * 3 + 1)), gu32(l8, col(i * 3 + 2)));
        pu32(l8, lcol(i) + 2, dif);
        pu32(l8, lcol(i) + 6, spe);
        put(l8, icol(i), &p.to_le_bytes());
        pu32(l8, icol(i) + 2, dif);
    }
    let n = if gu32(l8, 44) & 0x4000 != 0 { 0 } else { 2 };
    for i in 3..10 {
        let c = l8[lcol(n)..lcol(n) + 10].to_vec();
        put(l8, lcol(i), &c);
        put(l8, lcol(i), &((32200 + (i - 3) * 84) as u16).to_le_bytes());
    }
    let c = l8[icol(n)..icol(n) + 6].to_vec();
    put(l8, icol(3), &c);
    put(l8, icol(3), &32760u16.to_le_bytes());
    if v < 1 {
        // 2 ambient front colours from the average of the ambient colours
        let c = interpolate_color_b(gu32(l8, col(3)), gu32(l8, col(6)), gu32(l8, col(9)), 0.33333);
        for k in [3, 6, 9] {
            pu32(l8, col(k), c);
        }
    }
    let mask = if v < 1 { 0x3F09_FFFF } else { 0x3F0F_FFFF };
    let tbo = (gu32(l8, 44) & mask) | 2 << 20;
    pu32(l8, 44, tbo);
}

/// One formula of `THeaderCustomAddon`.
fn formula(k: usize) -> usize {
    8 + k * 188
}

/// `PutStringInCustomF`
fn put_name(d: &mut [u8], o: usize, s: &str) {
    for (i, c) in s.bytes().take(31).enumerate() {
        d[o + i] = c;
        d[o + i + 1] = 0;
    }
}

/// `SetCFoptionsFromOldF`: the option count of the internal formulas
/// (as `IniCFsFromHAddon` sets it later).
fn intern_option_count(f: i32) -> i32 {
    match f {
        4 | 6 => 3,
        5 => 6,
        9 => 8,
        _ => 2,
    }
}

/// `GetHAddOnFromInternFormula` for formula `f` in slot `t`.
fn haddon_from_intern_formula(addon: &mut [u8], f: i32, t: usize) {
    const OA: [[f64; 2]; 10] = [[8.0, -1.0], [8.0, -1.0], [-1.0, 0.0], [-2.0, 1.0], [2.0, 0.5], [2.0, 0.5], [2.0, -1.0], [-1.0, 0.0], [0.0, 0.0], [8.0, 1.0]];
    const NAMES: [&str; 10] = ["Integer Power", "Real Power", "Quaternion", "Tricorn", "Amazing Box", "Bulbox", "Folding Int Pow", "test", "testIFS", "Aexion C"];
    let o = formula(t);
    pi32(addon, o + 4, f);
    pi32(addon, o + 8, intern_option_count(f));
    let vals: [f64; 16] = std::array::from_fn(|i| match (f, i) {
        (_, 0 | 1) => OA[f as usize][i],
        (4, 2) | (5, 2) | (5, 3) => 1.0,
        (5, 4) | (5, 5) | (6, 2) => 2.0,
        (9, 2) | (9, 3) | (9, 5) => 1.0,
        (9, 4) => 8.0,
        _ => 0.0,
    });
    for (i, v) in vals.iter().enumerate() {
        pf64(addon, o + 60 + i * 8, *v);
    }
    put_name(addon, o + 12, NAMES[f as usize]);
}

/// `ConvertHeaderFromOldParas`: upgrades a `TMandHeader8` of MandId < 20
/// in place and builds the formula block for MandId < 16 (the formula
/// number in `iOptions`, the "alt. hybrid" in the header).  The custom
/// formulas it names are looked up later by [`crate::m3p::parse`], which
/// fails as MB3D does when one is missing.
pub(crate) fn convert_header_from_old_paras(p: &mut [u8], addon: &mut [u8]) -> Result<(), String> {
    const CF: [i32; 16] = [2, 2, 0, 0, 1, 1, 3, 3, 0, 4, 5, 6, 7, 8, 14, 15];
    const AF: [i32; 9] = [0, 0, 2, 2, 3, 3, 4, 4, 20];
    const NF: [i32; 12] = [0, 1, 2, 3, -1, 4, -1, -1, 5, -1, 6, 20];
    let (w, h) = (gi32(p, 4), gi32(p, 8));
    if !(1..32768).contains(&w) || !(1..32768).contains(&h) {
        return Err("parameters: invalid image size".into());
    }
    let mid = gi32(p, 0);
    let l = L8;
    let tb = |k: usize| l + (k - 1) * 4; // Light.TBpos[k]
    if mid < 4 {
        p[149] = 1; // bCalcAmbShadowAutomatic
    }
    if mid < 7 {
        p[190] = 0; // bIsJulia
        for o in [191, 199, 207] {
            pf64(p, o, 0.0);
        }
    }
    if mid < 8 {
        convert_from_old_light_paras(&mut p[l..], 148);
        pf32(p, 181, 1.0); // sBBmulBulb
        pf32(p, 185, 2.0); // sBBRbulb
        p[189] = 1; // bImageScale := ImageScale (the program's setting)
        if p[163] == 0 {
            let v = (gi32(p, tb(6)) - 53) / 3 + 53;
            pi32(p, tb(6), v);
        }
        let opt = gi32(p, 16);
        let i = (opt & 15) as usize;
        let mut opt = (opt & 0x7FFF_FFE0) | CF[i];
        pf64(p, 215, if i == 3 || i == 8 { -1.0 } else { 1.0 }); // dZmul
        if matches!(i, 0 | 1 | 6 | 7) {
            opt = (opt & 0x3FFFF) | ((i as i32 & 1) << 18);
        }
        if matches!(i, 2 | 3 | 8) {
            opt = (opt & 0x3FFFF) | (((i > 3) as i32 * 6) << 18);
        }
        pi32(p, 16, opt);
    }
    let fnr = gi32(p, 16) & 31;
    if mid < 9 {
        pf64(p, 223, 0.5); // dMinR
        let o = gi32(p, 16) >> 18;
        if (fnr == 5 && o & 4 != 0) || (fnr == 8 && o & 3 == 1) {
            pf64(p, 223, 0.25);
        }
        // ResetHybrid gets the header by value and changes nothing
    }
    if mid < 10 {
        p[328] = 0; // CustomFname
    }
    if mid < 11 {
        p[162] = 1; // bVaryDEstopOnFOV
    }
    if mid < 12 {
        for k in 0..3 {
            let v = gf32(p, 164 + k * 4) as f64;
            pf64(p, 346 + k * 8, v); // dCut := sCut
        }
        if fnr == 9 {
            pf64(p, 92, 1024.0); // RStop
        }
        if gf32(p, 319) < 0.001 {
            pf32(p, 319, 3.0); // sAmbShadowThreshold
        }
    }
    if mid < 13 {
        let v = gf64(p, 100);
        pf64(p, 370, v); // dCustomOption1 := dPow
        pi32(p, tb(2), p[148] as i8 as i32); // CalcBackgroundShadow
    }
    if mid < 15 {
        let (tbo, fine, depth) = (gu32(p, l + 44), gu32(p, l + 48), gu32(p, l + 52));
        pu32(p, l + 44, (tbo >> 4) | (fine & 0x10000));
        pu32(p, l + 48, (fine & 0xFFFF) | (depth & 0xFFFF) << 16);
        pu32(p, l + 52, (depth << 8) | ((depth >> 16) & 0xFF));
    }
    if mid < 16 {
        for i in 0..4 {
            let o = l + 92 + i * 14 + 1;
            p[o] = (p[o] & 3) | ((p[o] << 2) & 0xF0);
        }
        convert_formulas(p, addon, fnr, &AF, &NF);
    }
    if mid < 17 {
        calc_vgrads_from_header8_rots(p);
    }
    if mid < 18 {
        pi32(p, tb(1), 0);
        p[343] = 0; // byRepeatFrom
    }
    if mid < 20 {
        update_light_version8(&mut p[l..]);
        p[181] = 0; // bCalcDOF
        pi32(p, 164, 0); // sDOFZsharp
        pf32(p, 168, 5.0); // sDOFradius
        pf32(p, 172, 0.5); // sDOFosize
        pi32(p, 327, 0); // iCalcHStime
        if p[133] > 0 {
            p[133] = 5; // bCalculateHardShadow
        }
        p[163] |= p[133]; // bHScalculated
        p[132] = 0; // bNormalsOnDE
        let i = (((gi32(p, 16) >> 14) & 15) + 1).max(2) as f64; // DE accuracy
        pf32(p, 182, (2.2 / (i + 0.25 * i * i)) as f32); // mZstepDiv
        pf32(p, 150, 0.0); // sDEcolVar
    }
    Ok(())
}

/// The formula part of `ConvertHeaderFromOldParas` (MandId < 16).
fn convert_formulas(p: &mut [u8], a: &mut [u8], fnr: i32, af: &[i32; 9], nf: &[i32; 12]) {
    let val = |a: &mut [u8], k: usize, i: usize, v: f64| pf64(a, formula(k) + 60 + i * 8, v);
    let custom_name = p[328..344].to_vec();
    let (dpow, dzmul, dminr, dcopt1) = (gf64(p, 100), gf64(p, 215), gf64(p, 223), gf64(p, 370));
    a[0] = 1; // bHCAversion
    a[1] = 0; // bOptions1
    a[2] = 0; // bOptions2
    a[4] = 1; // iFCount
    for k in 1..6 {
        pi32(a, formula(k), 0);
        pi32(a, formula(k) + 4, -1);
    }
    a[formula(0) + 44..formula(0) + 49].fill(0); // byOptionType[0..4]
    pi32(a, formula(0) + 4, nf[fnr.min(11) as usize]);
    pi32(a, formula(0), 1);
    if fnr == 9 {
        // the "alt. hybrid" of the header
        for k in 0..3 {
            let o = formula(k);
            let hf = p[231 + k];
            let hcount = gi32(p, 234 + k * 4);
            let (hpow, hzmul, hminr) = (gf64(p, 246 + k * 8), gf64(p, 270 + k * 8), gf64(p, 294 + k * 8));
            a[o + 44..o + 49].fill(0);
            pi32(a, o, hcount);
            let f = af[hf.min(8) as usize];
            pi32(a, o + 4, f);
            pi32(a, o + 8, intern_option_count(f));
            if hcount <= 0 {
                continue;
            }
            val(a, k, 0, hpow);
            val(a, k, 1, hzmul);
            match f {
                2 => val(a, k, 0, if hf == 2 { 1.0 } else { -1.0 }), // Quaternion
                3 => {
                    // Tricorn
                    let (v0, v1) = if hf == 4 { (-2.0, 1.0) } else { (2.0, 0.0) };
                    val(a, k, 0, v0);
                    val(a, k, 1, v1);
                }
                4 => {
                    // Amazing Box
                    val(a, k, 0, hpow);
                    val(a, k, 1, hminr);
                    a[o + 44 + 1] = 7;
                }
                _ => {}
            }
            if hf > 7 {
                // external formula
                put(a, o + 12, &custom_name);
                val(a, k, 0, dcopt1);
                val(a, k, 1, dzmul);
                val(a, k, 2, dminr);
                pi32(a, o + 4, 20);
                pi32(a, o + 8, 3);
            }
        }
    } else if fnr == 11 {
        // external formula
        let o = formula(0);
        put(a, o + 12, &custom_name);
        val(a, 0, 0, dcopt1);
        val(a, 0, 1, dzmul);
        val(a, 0, 2, dminr);
        pi32(a, o + 4, 20);
        pi32(a, o + 8, 3);
    } else {
        let f = gi32(a, formula(0) + 4);
        if (0..=9).contains(&f) {
            haddon_from_intern_formula(a, f, 0);
        }
        val(a, 0, 0, dpow);
        val(a, 0, 1, dzmul);
        let i = (gi32(p, 16) >> 18) & 7;
        match fnr {
            0 | 10 => {
                // Integer Power, Folding Int Pow
                val(a, 0, 0, (i + 2) as f64);
                if fnr == 10 {
                    val(a, 0, 2, gf32(p, 185) as f64); // sBBRbulb
                }
            }
            2 => val(a, 0, 0, (1 - (i & 1) * 2) as f64),
            3 => {
                val(a, 0, 0, ((i & 1) * 4 - 2) as f64);
                val(a, 0, 1, (1 - (i & 1)) as f64);
            }
            4 => put_name(p, 328, "_SinePow2"),
            5 | 8 => {
                val(a, 0, 1, dminr);
                if fnr == 8 {
                    val(a, 0, 2, (gf32(p, 181) as f64).max(0.0)); // sBBmulBulb
                    val(a, 0, 3, (gf32(p, 185) as f64).max(0.0)); // sBBRbulb
                }
            }
            6 => put_name(p, 328, "Riemann"),
            7 => put_name(p, 328, "Fuzzy"),
            _ => {}
        }
        if matches!(fnr, 4 | 6 | 7) {
            // external formulas
            let o = formula(0);
            put(a, o + 12, &p[328..344]);
            pi32(a, o + 4, 20);
            val(a, 0, 0, dpow);
            val(a, 0, 1, (1 - 2 * (i & 1)) as f64);
            pi32(a, o + 8, 2);
            if fnr == 4 {
                // _SinePow2 twice, then Integer Power
                val(a, 0, 0, 1.0);
                val(a, 0, 1, 1.0);
                let o1 = formula(1);
                put(a, o1 + 12, &p[328..344]);
                pi32(a, o1 + 4, 20);
                pi32(a, o1, 1);
                val(a, 1, 0, (1 - ((i >> 1) & 1) * 2) as f64);
                val(a, 1, 1, 1.0);
                pi32(a, o1 + 8, 2);
                let o2 = formula(2);
                pi32(a, o2 + 4, 0);
                pi32(a, o2, 1);
                val(a, 2, 0, 2.0);
                val(a, 2, 1, (1 - (i & 1) * 2) as f64);
                pi32(a, o2 + 8, 2);
            }
        }
    }
}

/// `CalcVGradsFromHeader8rots`: the view matrix from the three rotation
/// angles of the old headers.
fn calc_vgrads_from_header8_rots(p: &mut [u8]) {
    let (s1, c1) = gf64(p, 60).sin_cos();
    let (s2, c2) = gf64(p, 68).sin_cos();
    let (s3, c3) = gf64(p, 76).sin_cos();
    let mut step = 2.1345 / (gf64(p, 84) * gi32(p, 4) as f64);
    if p[344] > 0 {
        step *= 0.01f32 as f64 + (gf64(p, 108).clamp(-1.25, 1.25) * 0.5).cos().abs();
    }
    for i in 0..3 {
        let mut v = [0.0; 3];
        v[i] = step;
        let (x2, y2, z2) = (v[0], c1 * v[1] + s1 * v[2], c1 * v[2] - s1 * v[1]);
        let (x1, y1, z1) = (c2 * x2 + s2 * z2, y2, c2 * z2 - s2 * x2);
        pf64(p, 246 + i * 24, c3 * x1 + s3 * y1);
        pf64(p, 254 + i * 24, c3 * y1 - s3 * x1);
        pf64(p, 262 + i * 24, z1);
    }
}

/// `ConvertLight8to9`: `TLightingParas8` to `TLightingParas9`.  `l9` holds
/// what was in the destination before: MB3D leaves `FineColAdj2` (and the
/// background file name after its first byte) untouched.
fn convert_light8_to9(l8: &[u8], l9: &mut [u8]) {
    let rgb = |c: u32| [c as u8, (c >> 8) as u8, (c >> 16) as u8];
    let col = |k: usize| gu32(l8, 56 + (k - 1) * 4);
    l9[..48].copy_from_slice(&l8[..48]);
    let tb = |k: usize| 8 + (k - 3) * 4; // TBpos[k]
    let fine = gu32(l8, 48);
    let mut tbo = gu32(l8, 44) | 3 << 20;
    // FineColAdj1 is set twice, the second time from the second byte
    l9[48] = (fine >> 8) as u8;
    l9[50] = 128; // PicOffsetX
    l9[51] = 128; // PicOffsetY
    l9[67] = 128; // PicOffsetZ
    let depth = gu32(l8, 52);
    put(l9, 52, &rgb(col(3))); // AmbCol
    put(l9, 56, &rgb(col(6))); // AmbCol2
    put(l9, 60, &rgb(depth >> 8)); // DepthCol
    put(l9, 64, &rgb(((depth & 0xFF) << 16) | (fine >> 16))); // DepthCol2
    l9[55] = 255; // DynFogR
    l9[59] = 255;
    l9[63] = 255;
    put(l9, 4, &[255, 255, 255]); // DynFogCol2
    for i in 0..6 {
        // defaultLight8
        let o = 68 + i * 32;
        l9[o..o + 32].fill(0);
        l9[o] = 1;
        l9[o + 1] = 4;
        put(l9, o + 2, &10u16.to_le_bytes());
        put(l9, o + 4, &[255, 255, 255]);
    }
    let pid16384 = std::f64::consts::PI / 16384.0;
    for i in 0..4 {
        let (s, o) = (92 + i * 14, 68 + i * 32);
        l9[o] = l8[s];
        l9[o + 1] = l8[s + 1];
        put(l9, o + 4, &rgb(gu32(l8, s + 2)));
        put(l9, o + 7, &[0, 0]); // LightMapNr
        put(l9, o + 9, &(gi32(l8, s + 6) as f64 * pid16384).to_le_bytes()[1..8]);
        put(l9, o + 17, &(gi32(l8, s + 10) as f64 * pid16384).to_le_bytes()[1..8]);
    }
    l9[260..384].copy_from_slice(&l8[148..272]); // LCols, ICols
    l9[384] = 0; // BGbmp
    let sqr = |v: i32| v.wrapping_mul(v);
    let i = sqr(gi32(l9, tb(10)) + 30).max(0);
    let j = sqr(gi32(l9, tb(9)) + 30).max(0);
    if tbo & 0x10000 != 0 {
        // fine adjustment down
        pi32(l9, tb(10), round(((i + i - j).max(0) as f64).sqrt() - 30.0));
        let (i, j) = (l9[48] as i32, l9[49] as i32);
        l9[48] = (i / 2 + 15) as u8;
        l9[49] = (j - (j - i) / 4).clamp(0, 255) as u8;
    } else {
        pi32(l9, tb(10), round(((i + ((i - j) * 7) / 4).max(0) as f64).sqrt() - 30.0));
    }
    let i = sqr(sqr((tbo & 127) as i32));
    let j = sqr(sqr(((tbo >> 7) & 127) as i32));
    let j = round((((j + ((j - i) * 7) / 4).max(0)) as f64).sqrt().sqrt()).clamp(0, 120);
    tbo = (tbo & 0x3F_C07F) | (j as u32) << 7;
    pu32(l9, 44, tbo);
    let vz = i16::from_le_bytes([l9[0], l9[1]]).wrapping_mul(2);
    put(l9, 0, &vz.to_le_bytes()); // VarColZpos
    pi32(l9, tb(4), gi32(l9, tb(4)).wrapping_mul(2));
}

/// The converted `TMandHeader8` as `TMandHeader10` with an empty formula
/// block: `FastMove` of the header, then `ConvertLight8to9`.  With
/// `whole_record` (`LoadParameter`, `GetHeaderFromText`) the 704 bytes are
/// moved, so the light record starts out as the old one; `LoadAni` moves
/// only the 432 bytes before it.
pub(crate) fn header10_from_old(p8: &[u8], whole_record: bool) -> Vec<u8> {
    let mut raw = vec![0u8; HEADER_SIZE + ADDON_SIZE];
    let n = if whole_record { OLD_HEADER_SIZE } else { L8 };
    raw[..n].copy_from_slice(&p8[..n]);
    convert_light8_to9(&p8[L8..], &mut raw[L8..HEADER_SIZE]);
    // TMandHeader10 has a word for iOptions; UpdateFormulaOptionAbove20
    // clears bNewOptions, bColorOnIt (MandId < 43, byColor2Option is 0
    // before 22) and bStereoMode
    raw[18] = 0;
    raw[19] = 0;
    raw[126] = 0;
    raw
}

/// `LoadHAddon`: a formula block in the current (`THeaderCustomAddon`) or
/// the old layout (`THeaderCustomAddonOld`, no hybrid options).  The old
/// one only replaces the formulas.  Returns the bytes read.
pub(crate) fn load_haddon(data: &[u8], addon: &mut [u8]) -> usize {
    let mut d = data[..data.len().min(ADDON_SIZE)].to_vec();
    d.resize(ADDON_SIZE, 0); // MB3D reads beyond the end of the file
    if (16..=99).contains(&d[0]) {
        addon.copy_from_slice(&d);
        return ADDON_SIZE;
    }
    for k in 0..6 {
        let o = formula(k);
        put(addon, o, &d[8 + k * 4..12 + k * 4]); // iItCount
        put(addon, o + 4, &d[32 + k * 4..36 + k * 4]); // iFnr
        put(addon, o + 12, &d[56 + k * 32..88 + k * 32]); // CustomFname
        put(addon, o + 60, &d[248 + k * 128..376 + k * 128]); // dOptionValue
        // iOptionCount comes from the formula (IniCFsFromHAddon)
        let f = gi32(addon, o + 4);
        pi32(addon, o + 8, if (0..20).contains(&f) { intern_option_count(f) } else { 16 });
    }
    OLD_ADDON_SIZE
}

/// `UpdateFormulaOptionTo20`: the options of the quaternion, Amazing Box
/// and Bulbox before MandId 20.
fn update_formula_option_to20(addon: &mut [u8]) {
    for k in 0..6 {
        let o = formula(k);
        let v = |i: usize| o + 60 + i * 8;
        match gi32(addon, o + 4) {
            2 => {
                pf64(addon, v(1), 0.0); // quaternion w add
                addon[o + 44 + 1] = 0;
            }
            j @ (4 | 5) => {
                pi32(addon, o + 8, 3);
                if j == 5 {
                    let (v2, v3) = (gf64(addon, v(2)), gf64(addon, v(3)));
                    pf64(addon, v(4), v3);
                    pf64(addon, v(3), v2);
                    addon[o + 44 + 3] = 0;
                    addon[o + 44 + 4] = 9;
                    pi32(addon, o + 8, 5);
                }
                pf64(addon, v(2), 1.0); // box fold
                addon[o + 44 + 2] = 11;
            }
            _ => {}
        }
    }
}

/// The formula block behind the converted header: `UpdateFormulaOptionTo20`
/// and the layout mark that [`crate::m3p::parse`] checks (MB3D writes 16
/// there when it saves).
pub(crate) fn finish_addon(raw: &mut [u8], addon: &[u8], mand_id: i32) {
    let a = &mut raw[HEADER_SIZE..HEADER_SIZE + ADDON_SIZE];
    a.copy_from_slice(addon);
    if mand_id < 20 {
        update_formula_option_to20(a);
    }
    a[0] = 16;
}

/// `GetHeaderFromText` for text parameters before version 16: `stream` is
/// the decoded text.  Its 235th group of 3 bytes overwrites the last two
/// bytes of the 234th, so the 704 byte header comes from 705 decoded bytes.
/// From version 15 on the formula block follows.
pub(crate) fn raw_from_old_text(stream: &[u8], ver: f32) -> Result<Vec<u8>, String> {
    if stream.len() < 702 {
        return Err(format!("text parameters too short ({} of 705 header bytes)", stream.len()));
    }
    let mut p8 = vec![0u8; OLD_HEADER_SIZE];
    p8[..700].copy_from_slice(&stream[..700]);
    let tail = &stream[702..stream.len().min(705)];
    p8[700..700 + tail.len()].copy_from_slice(tail);
    let mut addon = vec![0u8; ADDON_SIZE];
    convert_header_from_old_paras(&mut p8, &mut addon)?;
    let mut raw = header10_from_old(&p8, true);
    if ver > 14.0 {
        // ResetFormulas, then the block as far as the text has it
        for k in 0..6 {
            let o = formula(k);
            pi32(&mut addon, o, 0);
            pi32(&mut addon, o + 4, -1);
            pi32(&mut addon, o + 8, 0);
            addon[o + 12] = 0;
        }
        if stream.len() > 705 {
            let b = &stream[705..stream.len().min(705 + ADDON_SIZE)];
            addon[..b.len()].copy_from_slice(b);
        }
    }
    let mid = gi32(&raw, 0);
    finish_addon(&mut raw, &addon, mid);
    Ok(raw)
}

/// The G-buffer of an `.m3i` image before MandId 20 (`LoadFullM3I`):
/// `TsiLight3` (MandId < 18) or `TsiLight4` (< 19) records with the normal
/// as two angles are converted to `TsiLight5` (`ConvertSiLight4To5`), then
/// the ambient shadow, the z normal and the shadow word are adapted.
pub(crate) fn gbuffer_from_old_m3i(data: &[u8]) -> Option<(usize, usize, Vec<crate::gbuffer::SiLight>)> {
    let mid = gi32(data, 0);
    let p8 = read_old_header(data).ok()?;
    let (w, h) = (gi32(&p8, 4), gi32(&p8, 8));
    if w < 1 || h < 1 {
        return None;
    }
    let (w, h) = (w as usize, h as usize);
    let n = w * h;
    let start = stored_header_size(mid);
    let size = match mid {
        ..=17 => 12,
        18 => 16,
        _ => 18,
    };
    let px = data.get(start..start + n * size)?;
    let u16at = |b: &[u8], o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    // SinTab (singles)
    let sin_tab: Vec<f32> = (0..32768).map(|i| (i as f64 * std::f64::consts::PI / 16384.0).sin() as f32).collect();
    let hard_shadow = p8[133] > 0;
    let g = (0..n)
        .map(|i| {
            // TsiLight4 words: angle x, angle y, RoughZposFine, Zpos,
            // Shadow, AmbShadow, SIgradient, OTrap
            let w4: [u16; 8] = if size == 12 {
                let b = &px[i * 12..i * 12 + 12];
                [u16at(b, 0), u16at(b, 2), 0, u16at(b, 4), u16at(b, 6), u16at(b, 8), u16at(b, 10), 0]
            } else if size == 16 {
                let b = &px[i * 16..i * 16 + 16];
                std::array::from_fn(|k| u16at(b, k * 2))
            } else {
                [0; 8]
            };
            let mut s = if size == 18 {
                let b = &px[i * 18..i * 18 + 18];
                crate::gbuffer::SiLight {
                    normal: [u16at(b, 0) as i16, u16at(b, 2) as i16, u16at(b, 4) as i16],
                    zpos_fine: u16at(b, 6) as u32 | (u16at(b, 8) as u32) << 16,
                    shadow: u16at(b, 10),
                    amb_shadow: u16at(b, 12),
                    si_gradient: u16at(b, 14),
                    otrap: u16at(b, 16),
                }
            } else {
                let (ax, ay) = (w4[0] as usize, w4[1] as usize);
                let nz = sin_tab[(ax + 8192) & 0x7FFF] as f64 * sin_tab[(ay + 8192) & 0x7FFF] as f64 * 32767.0;
                crate::gbuffer::SiLight {
                    normal: [
                        round(sin_tab[ax & 0x7FFF] as f64 * 32767.0) as i16,
                        round(sin_tab[ay & 0x7FFF] as f64 * 32767.0) as i16,
                        round(nz) as i16,
                    ],
                    zpos_fine: w4[2] as u32 | (w4[3] as u32) << 16,
                    shadow: w4[4],
                    amb_shadow: w4[5],
                    si_gradient: w4[6],
                    otrap: w4[7],
                }
            };
            s.amb_shadow >>= 1;
            s.normal[2] = s.normal[2].wrapping_neg();
            s.shadow = if hard_shadow {
                if s.shadow < 32767 {
                    0x400
                } else {
                    0
                }
            } else if (s.zpos_fine >> 16) > 32767 {
                100
            } else {
                (32767 - (s.shadow & 0x7FFF)) >> 5
            };
            s
        })
        .collect();
    Some((w, h, g))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::Formula;
    use crate::m3p::{gbuffer_from_m3i, parse, raw_from_bytes, raw_from_text};

    /// A `TMandHeader8` as MB3D 1.5 wrote it (the fields of `TypeDefinitions.pas`).
    fn header8(mid: i32, fnr_options: i32) -> Vec<u8> {
        let mut p = vec![0u8; OLD_HEADER_SIZE];
        pi32(&mut p, 0, mid);
        pi32(&mut p, 4, 320); // Width
        pi32(&mut p, 8, 240); // Height
        pi32(&mut p, 12, 12); // Iterations
        pi32(&mut p, 16, fnr_options); // iOptions
        pf64(&mut p, 20, -1.5); // dZstart
        pf64(&mut p, 28, 4.0); // dZend
        pf64(&mut p, 36, 0.25); // dXmid
        pf64(&mut p, 84, 1.0); // dZoom
        pf64(&mut p, 92, 1024.0); // RStop
        pf64(&mut p, 100, 8.0); // dPow
        pf64(&mut p, 108, 30.0f64.to_radians()); // dFOVy
        p[133] = 1; // bCalculateHardShadow
        p[134] = 6; // bStepsafterDEStop
        pf32(&mut p, 177, 1.5); // sDEstop
        pf64(&mut p, 215, 1.0); // dZmul
        pf32(&mut p, 319, 0.4); // sAmbShadowThreshold
        // TLightingParas8: TBpos, TBoptions 0, Cols, light 1
        let tb = |k: usize| L8 + (k - 1) * 4;
        for (k, v) in [(4, 7), (5, 50), (7, 30), (8, 50), (9, 10), (10, 20)] {
            pi32(&mut p, tb(k), v);
        }
        let col = |k: usize| L8 + 56 + (k - 1) * 4;
        pu32(&mut p, col(1), 0x0000FF);
        pu32(&mut p, col(2), 0xFFFFFF);
        for k in [3, 6, 9] {
            pu32(&mut p, col(k), 0x102030);
        }
        let l = L8 + 92;
        p[l + 1] = 0x12; // LFunction
        pu32(&mut p, l + 2, 0xFF8040); // Lcolor
        pi32(&mut p, l + 6, 8192); // LXangle = 90 degrees
        pi32(&mut p, l + 10, -4096);
        for i in 1..4 {
            p[l + i * 14] = 1; // off
        }
        p
    }

    fn load(file: &[u8]) -> crate::m3p::M3pFile {
        let (raw, _) = raw_from_bytes(file.to_vec(), false).unwrap();
        parse(&raw).unwrap()
    }

    #[test]
    fn integer_power_of_mand_id_10() {
        // formula 0 (Integer Power), option 6 = power 8, DE accuracy 3
        let m = load(&header8(10, 6 << 18 | 3 << 14));
        let s = &m.scene;
        assert_eq!(m.mand_id, 10);
        assert_eq!((s.width, s.height, s.iterations), (320, 240, 12));
        assert_eq!((s.z_start, s.z_end, s.mid[0]), (-1.5, 4.0, 0.25));
        assert_eq!(s.formulas.len(), 1);
        assert_eq!(s.formulas[0].iterations, 1);
        match s.formulas[0].formula {
            Formula::IntPow { power, z_mul } => assert_eq!((power, z_mul), (8, 1.0)),
            ref f => panic!("{f:?}"),
        }
        // mZstepDiv from the DE accuracy: 2.2 / (4 + 0.25 * 16)
        assert!((s.z_step_div - 0.275).abs() < 1e-6);
        assert_eq!(s.de_stop, 1.5);
        assert_eq!(s.bin_search_steps, 6);
        // CalcVGradsFromHeader8rots without rotation: the step on the diagonal
        let step = 2.1345 / 320.0;
        assert!((s.vgrads[0][0] - step).abs() < 1e-15 && s.vgrads[0][1] == 0.0 && (s.vgrads[2][2] - step).abs() < 1e-15);
        // hard shadows: bCalculateHardShadow 1 becomes 5 (automatic, light 1)
        assert_eq!(s.shadows.as_ref().map(|h| h.lights), Some(1));
        assert!(s.dof.is_none());
        let l = &s.lighting;
        // UpdateLightVersion8: diffuse 1 averaged with ambient 1 (50 : 50)
        assert_eq!(l.palette[0].diffuse, [152, 16, 8]);
        assert_eq!(l.palette[0].specular, [255, 255, 255]);
        assert_eq!((l.palette[1].position, l.palette[3].position), (10922, 32200));
        // the average of the ambient colours with weights 0.33333 (the blue
        // byte, masked after rounding, loses one)
        assert_eq!(l.amb_top, [0x30, 0x20, 0x0F]);
        // ConvertLight8to9: colour end from the squares, depth fog doubled
        assert_eq!((l.color_start, l.color_end, l.depth_fog), (10.0, 34.0, 14.0));
        assert_eq!(l.fog_offset, 128.0);
        assert_eq!(l.gamma, 32.0);
        let l0 = &l.lights[0];
        assert!(l0.on && !l.lights[1].on);
        assert_eq!(l0.color, [0x40, 0x80, 0xFF]);
        assert_eq!((l0.spec_func, l0.diff_func), (2, 0));
        assert!((l0.x_angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((l0.y_angle + std::f64::consts::FRAC_PI_4).abs() < 1e-12);
    }

    #[test]
    fn alt_hybrid_of_mand_id_12() {
        // formula 9: the alternating hybrid of the header
        let mut p = header8(12, 9);
        p[231] = 6; // Amazing Box
        p[232] = 0; // Integer Power
        pi32(&mut p, 234, 2);
        pi32(&mut p, 238, 1);
        pf64(&mut p, 246, 2.5); // dHPow
        pf64(&mut p, 254, 8.0);
        pf64(&mut p, 278, 1.0); // dHZmul[1]
        pf64(&mut p, 294, 0.25); // dHMinR
        pf64(&mut p, 60, 0.3); // dXrot
        let s = load(&p).scene;
        assert_eq!(s.formulas.len(), 2);
        assert_eq!(s.formulas[0].iterations, 2);
        match s.formulas[0].formula {
            Formula::AmazingBox { scale, min_r, fold } => assert_eq!((scale, min_r, fold), (2.5, 0.25, 1.0)),
            ref f => panic!("{f:?}"),
        }
        match s.formulas[1].formula {
            Formula::IntPow { power, z_mul } => assert_eq!((power, z_mul), (8, 1.0)),
            ref f => panic!("{f:?}"),
        }
        // the rotation about the x axis in the view matrix
        let step = 2.1345 / 320.0;
        let (sn, cs) = 0.3f64.sin_cos();
        assert!((s.vgrads[1][1] - cs * step).abs() < 1e-15);
        assert!((s.vgrads[1][2] + sn * step).abs() < 1e-15);
        assert!((s.vgrads[2][1] - sn * step).abs() < 1e-15);
    }

    #[test]
    fn oldest_header_mand_id_3() {
        // TMandHeaderOld: 200 bytes, z range stored the other way round,
        // formula 4 (now Real Power)
        let mut p = header8(3, 4 | 2 << 10);
        p.truncate(200);
        pf64(&mut p, 20, 5.0);
        pf64(&mut p, 28, -1.0);
        pf64(&mut p, 60, 0.0);
        pf64(&mut p, 68, 90.0); // dYrot in degrees
        pi32(&mut p, 108 + 44, 0x13); // Light.TBoptions: LFunction of light 1
        let m = load(&p);
        let s = &m.scene;
        assert_eq!((s.z_start, s.z_end, s.mid[2]), (-5.0, 1.0, -2.0));
        match s.formulas[0].formula {
            Formula::RealPower { power, z_mul } => assert_eq!((power, z_mul), (8.0, 1.0)),
            ref f => panic!("{f:?}"),
        }
        // sDEstop from the iOptions of TMandHeader4
        assert!((s.de_stop - 0.6).abs() < 1e-6);
        assert!(s.ao.is_some());
        // dYrot (90 degrees) became the x rotation in radians; bCalc3D = 1
        // adds the field of view (0 in these records) to the step
        let step = 2.1345 / 320.0 * (0.01f32 as f64 + 1.0);
        let (sn, cs) = std::f64::consts::FRAC_PI_2.sin_cos();
        assert!((s.vgrads[1][1] - cs * step).abs() < 1e-15 && (s.vgrads[1][2] + sn * step).abs() < 1e-15);
        assert!(s.lighting.lights[0].on);
        assert!(!s.lighting.lights[1].on);
    }

    #[test]
    fn sine_pow2_hybrid_of_mand_id_10() {
        // old formula 4 (option 3): _SinePow2 twice and Integer Power, as
        // MB3D 1.99 converts it
        let raw = raw_from_old_file(&header8(10, 4 | 3 << 18)).unwrap();
        let a = &raw[HEADER_SIZE..];
        let name = |k: usize| String::from_utf8_lossy(&a[formula(k) + 12..formula(k) + 21]).into_owned();
        let val = |k: usize, i: usize| gf64(a, formula(k) + 60 + i * 8);
        assert_eq!((name(0), name(1)), ("_SinePow2".to_string(), "_SinePow2".to_string()));
        assert_eq!([gi32(a, formula(0) + 4), gi32(a, formula(1) + 4), gi32(a, formula(2) + 4)], [20, 20, 0]);
        assert_eq!([gi32(a, formula(0)), gi32(a, formula(1)), gi32(a, formula(2)), gi32(a, formula(3))], [1, 1, 1, 0]);
        assert_eq!([val(0, 0), val(0, 1), val(1, 0), val(1, 1), val(2, 0), val(2, 1)], [1.0, 1.0, -1.0, 1.0, 2.0, -1.0]);
        // hybrid options of UpdateFormulaOptionAbove20 come later (parse)
        assert_eq!((a[0], a[1], a[4]), (16, 0, 1));
    }

    #[test]
    fn text_parameters_of_version_14() {
        let mut p = header8(14, 1); // Real Power
        p[703] = 0; // not in the text
        // GetHeaderFromText: the 235th group overwrites 2 bytes of the 234th
        let mut stream = p[..700].to_vec();
        stream.extend_from_slice(&[0xAA, 0xBB]);
        stream.extend_from_slice(&p[700..703]);
        let mut text = String::from("Mandelbulb3Dv14{\r\n");
        for (i, g) in stream.chunks(3).enumerate() {
            crate::m3p::three_bytes_to_4_chars(g, &mut text);
            if i % 20 == 19 {
                text.push_str("\r\n");
            }
        }
        text.push_str("}\r\n{Titel: old one}\r\n");
        let (raw, title) = raw_from_text(&text).unwrap();
        assert_eq!(title, "old one");
        let (bin, _) = raw_from_bytes(p, false).unwrap();
        assert_eq!(raw, bin);
        let s = parse(&raw).unwrap().scene;
        assert!(matches!(s.formulas[0].formula, Formula::RealPower { power, .. } if power == 8.0));
    }

    #[test]
    fn image_of_mand_id_17_with_old_formula_block() {
        let (w, h) = (4usize, 2usize);
        let mut d = header8(17, 0);
        pi32(&mut d, 4, w as i32);
        pi32(&mut d, 8, h as i32);
        d[133] = 0;
        for i in 0..3 {
            pf64(&mut d, 246 + i * 32, 0.01); // hVGrads diagonal
        }
        // TsiLight3: angle x, angle y, Zpos, Shadow, AmbShadow, SIgradient
        for i in 0..w * h {
            let zpos: u16 = if i == 7 { 40000 } else { 1000 };
            for v in [8192u16, 0, zpos, 0x0400, 2000, 77] {
                d.extend_from_slice(&v.to_le_bytes());
            }
        }
        // THeaderCustomAddonOld: Integer Power, 3 iterations
        let mut a = vec![0u8; OLD_ADDON_SIZE];
        pi32(&mut a, 8, 3);
        for k in 1..6 {
            pi32(&mut a, 32 + k * 4, -1);
        }
        pf64(&mut a, 248, 4.0);
        pf64(&mut a, 256, 1.0);
        d.extend_from_slice(&a);
        let (raw, _) = raw_from_bytes(d.clone(), true).unwrap();
        let s = parse(&raw).unwrap().scene;
        assert_eq!(s.formulas.len(), 1);
        assert_eq!(s.formulas[0].iterations, 3);
        assert!(matches!(s.formulas[0].formula, Formula::IntPow { power: 4, .. }));
        let (gw, gh, g) = gbuffer_from_m3i(&d).unwrap();
        assert_eq!((gw, gh), (w, h));
        // ConvertSiLight4To5 and the fixes of LoadFullM3I
        assert_eq!(g[0].normal, [32767, 0, 0]);
        assert_eq!(g[0].zpos_fine, 1000 << 16);
        assert_eq!(g[0].amb_shadow, 1000);
        assert_eq!(g[0].si_gradient, 77);
        assert_eq!(g[0].shadow, (32767 - 0x0400) >> 5);
        assert_eq!(g[7].shadow, 100);
    }

    #[test]
    fn light_conversion_fine_down() {
        // ConvertLight8to9 with "fine adjustment down" (TBoptions bit 16)
        let mut l8 = vec![0u8; 272];
        pu32(&mut l8, 44, 0x10000 | 20 | 40 << 7);
        pu32(&mut l8, 48, 0x0000_6432);
        pi32(&mut l8, 32, 20); // TBpos[9]
        pi32(&mut l8, 36, 10); // TBpos[10]
        let mut l9 = vec![0u8; 408];
        l9[..272].copy_from_slice(&l8);
        convert_light8_to9(&l8, &mut l9);
        // TBpos[10] := Round(Sqrt(i + i - j)) - 30 with i = 40^2, j = 50^2
        assert_eq!(gi32(&l9, 36), round((700f64).sqrt() - 30.0));
        // FineColAdj1 gets the second byte (assigned twice in MB3D)
        assert_eq!(l9[48], 0x64 / 2 + 15);
        assert_eq!(l9[49], 0x64);
        // interior end: Round(Sqrt(Sqrt(j + (j - i) * 7 div 4)))
        let (i, j) = (20i32.pow(4), 40i32.pow(4));
        let e = round(((j + (j - i) * 7 / 4) as f64).sqrt().sqrt()).min(120) as u32;
        assert_eq!(gu32(&l9, 44), 0x30_0000 | 0x10000 | 20 | e << 7);
    }
}
