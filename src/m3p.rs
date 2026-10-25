//! Loader for Mandelbulb3D's binary parameter files (`.m3p`, MandId >= 20):
//! `TMandHeader10` (840 bytes, including the light record
//! `TLightingParas9`) followed by `THeaderCustomAddon` (the hybrid formulas),
//! see `LoadParameter` in FileHandling.pas.  Older files are converted to
//! this layout by [`crate::m3p_old`].

use crate::formulas::Formula;
use crate::lighting::{Light, Lighting, PaletteColor};
use crate::scene::{CameraOptic, FormulaEntry, Scene};

pub(crate) const HEADER_SIZE: usize = 840;
pub(crate) const ADDON_SIZE: usize = 8 + 6 * 188;

struct Rd<'a>(&'a [u8]);
impl Rd<'_> {
    fn u8(&self, o: usize) -> u8 {
        self.0[o]
    }
    fn u16(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.0[o], self.0[o + 1]])
    }
    fn i16(&self, o: usize) -> i16 {
        self.u16(o) as i16
    }
    fn i32(&self, o: usize) -> i32 {
        i32::from_le_bytes(self.0[o..o + 4].try_into().unwrap())
    }
    fn u32(&self, o: usize) -> u32 {
        self.i32(o) as u32
    }
    fn f32(&self, o: usize) -> f64 {
        f32::from_le_bytes(self.0[o..o + 4].try_into().unwrap()) as f64
    }
    fn f64(&self, o: usize) -> f64 {
        f64::from_le_bytes(self.0[o..o + 8].try_into().unwrap())
    }
    fn rgb(&self, o: usize) -> [u8; 3] {
        [self.0[o], self.0[o + 1], self.0[o + 2]]
    }
    /// `Double7B`: the 7 high bytes of a double
    fn d7b(&self, o: usize) -> f64 {
        let mut b = [0u8; 8];
        b[1..8].copy_from_slice(&self.0[o..o + 7]);
        f64::from_le_bytes(b)
    }
    fn str(&self, o: usize, n: usize) -> String {
        let s = &self.0[o..o + n];
        let end = s.iter().position(|&c| c == 0).unwrap_or(n);
        s[..end].iter().map(|&c| c as char).collect::<String>().trim().to_string()
    }
}

/// A loaded parameter file: the scene plus notes about unsupported features.
pub struct M3pFile {
    pub scene: Scene,
    pub warnings: Vec<String>,
    pub mand_id: i32,
}

/// `ShortFloatToSingle`: mantissa (shortint) * 10^(exponent (shortint) - 1)
pub(crate) fn short_float(w: u16) -> f32 {
    let m = (w & 0xFF) as u8 as i8 as f32;
    let e = ((w >> 8) as u8 as i8).clamp(-25, 25) as i32;
    m * 10f32.powi(e - 1)
}

/// `CreateMatrixFromQuat`
fn matrix_from_quat(q: [f64; 4]) -> [[f64; 3]; 3] {
    [
        [1.0 - 2.0 * (q[1] * q[1] + q[2] * q[2]), 2.0 * (q[0] * q[1] + q[2] * q[3]), 2.0 * (q[0] * q[2] - q[1] * q[3])],
        [2.0 * (q[0] * q[1] - q[2] * q[3]), 1.0 - 2.0 * (q[0] * q[0] + q[2] * q[2]), 2.0 * (q[2] * q[1] + q[0] * q[3])],
        [2.0 * (q[0] * q[2] + q[1] * q[3]), 2.0 * (q[1] * q[2] - q[0] * q[3]), 1.0 - 2.0 * (q[0] * q[0] + q[1] * q[1])],
    ]
}

/// Loads a `.m3p` or `.m3i` file, or a text file with MB3D text parameters.
pub fn load(path: &std::path::Path) -> Result<M3pFile, String> {
    crate::formulas::note_scene_file(path);
    let (raw, title) = read_raw(path)?;
    let mut m = parse(&raw)?;
    if let Some(t) = title {
        m.warnings.insert(0, format!("title: {t}"));
    }
    Ok(m)
}

/// The raw parameters of a file: `TMandHeader10` followed by the full
/// `THeaderCustomAddon` (as in a `.m3p` file), plus the title of text
/// parameters.
pub fn read_raw(path: &std::path::Path) -> Result<(Vec<u8>, Option<String>), String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let is_m3i = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3i"));
    raw_from_bytes(data, is_m3i)
}

/// Like [`read_raw`] for file contents already in memory.
pub fn raw_from_bytes(data: Vec<u8>, is_m3i: bool) -> Result<(Vec<u8>, Option<String>), String> {
    if let Some(text) = find_text_params(&data) {
        let (raw, title) = raw_from_text(&text)?;
        Ok((raw, Some(title)))
    } else if is_m3i {
        Ok((raw_from_m3i(&data)?, None))
    } else if data.len() >= 4 && i32::from_le_bytes(data[0..4].try_into().unwrap()) < 20 {
        Ok((crate::m3p_old::raw_from_old_file(&data)?, None))
    } else {
        let mut raw = data;
        raw.resize(raw.len().max(HEADER_SIZE + ADDON_SIZE), 0);
        raw.truncate(HEADER_SIZE + ADDON_SIZE);
        Ok((raw, None))
    }
}

/// Text parameters inside arbitrary text (a forum post, a clipboard dump).
fn find_text_params(data: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(data);
    let i = text.find("Mandelbulb3Dv")?;
    Some(text[i..].to_string())
}

/// `ThreeBytesTo4Chars`
pub(crate) fn three_bytes_to_4_chars(b: &[u8], out: &mut String) {
    let mut i = b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16;
    for _ in 0..4 {
        let mut c = (i & 0x3F) as u8;
        c += if c < 12 {
            46
        } else if c < 38 {
            53
        } else {
            59
        };
        out.push(c as char);
        i >>= 6;
    }
}

/// `FourCharsTo3Bytes`
fn four_chars_to_3_bytes(s: &[u8]) -> Option<[u8; 3]> {
    if s.len() < 4 {
        return None;
    }
    let mut i = 0u32;
    for (j, &c) in s[..4].iter().enumerate() {
        let v = match c {
            97..=122 => c - 59,
            65..=90 => c - 53,
            46..=57 => c - 46,
            _ => return None,
        };
        i |= (v as u32) << (j * 6);
    }
    Some([i as u8, (i >> 8) as u8, (i >> 16) as u8])
}

/// `GetHeaderFromText`: decodes `Mandelbulb3Dv18{...}{Titel: ...}` text
/// parameters (versions before 16 hold a `TMandHeader8`, see
/// [`crate::m3p_old::raw_from_old_text`]).
pub fn raw_from_text(text: &str) -> Result<(Vec<u8>, String), String> {
    let t = text.trim_start();
    let open = t.find('{').ok_or("text parameters: missing '{'")?;
    // the last run of digits and dots before '{' is the version
    let mut num = String::new();
    for c in t[..open].chars() {
        if c == '.' || c.is_ascii_digit() {
            num.push(c);
        } else {
            num.clear();
        }
    }
    let ver: f32 = if num.is_empty() { 14.0 } else { num.parse().unwrap_or(13.0) };
    let close = t[open..].find('}').ok_or("text parameters: missing '}'")? + open;
    // the "..." character (cp1252 133) stands for three dots
    let body: Vec<u8> = t[open + 1..close]
        .replace('\u{2026}', "...")
        .bytes()
        .filter(|&c| c > 32 && c != 0x85)
        .collect();
    let mut raw = Vec::with_capacity(HEADER_SIZE + ADDON_SIZE);
    for ch in body.chunks(4) {
        if ch.len() < 4 {
            break;
        }
        let b = four_chars_to_3_bytes(ch).ok_or_else(|| format!("text parameters: bad characters '{}'", String::from_utf8_lossy(ch)))?;
        raw.extend_from_slice(&b);
    }
    if ver < 16.0 {
        raw = crate::m3p_old::raw_from_old_text(&raw, ver)?;
    }
    if raw.len() < HEADER_SIZE {
        return Err(format!("text parameters too short ({} of {HEADER_SIZE} header bytes)", raw.len()));
    }
    let w = i32::from_le_bytes(raw[4..8].try_into().unwrap());
    let h = i32::from_le_bytes(raw[8..12].try_into().unwrap());
    if !(1..32768).contains(&w) || !(1..32768).contains(&h) {
        return Err("text parameters: invalid image size".into());
    }
    // formulas after the header (only the used ones are stored)
    raw.resize(HEADER_SIZE + ADDON_SIZE, 0);
    let rest = &t[close + 1..];
    let title = rest
        .find("{Titel: ")
        .map(|i| {
            let r = &rest[i + 8..];
            r[..r.find(['}', '\n', '\r']).unwrap_or(r.len())].chars().take(48).collect::<String>()
        })
        .unwrap_or_else(|| "Mandelbulb3D".to_string());
    Ok((raw, title))
}

/// `MakeTextparas`: the parameters as MB3D text (`Mandelbulb3Dv18{...}`).
/// Formulas with negative iterations count as used (MB3D looks for > 0
/// here only, dropping a last one that its own files and renders keep).
pub fn raw_to_text(raw: &[u8], title: &str) -> String {
    let mut raw = raw.to_vec();
    raw.resize(HEADER_SIZE + ADDON_SIZE, 0);
    let a = HEADER_SIZE;
    raw[a] = 16;
    let n = if raw[a + 1] & 3 == 1 {
        1
    } else {
        (0..6).rev().find(|&i| i32::from_le_bytes(raw[a + 8 + i * 188..a + 12 + i * 188].try_into().unwrap()) != 0).map(|i| i as i32).unwrap_or(-1)
    };
    raw[a + 4] = (n + 1) as u8;
    let mut out = String::from("Mandelbulb3Dv18{\r\n");
    let mut k = 0;
    let push = |b: &[u8], out: &mut String, k: &mut usize| {
        three_bytes_to_4_chars(b, out);
        *k += 1;
        if *k % 20 == 0 {
            out.push_str("\r\n");
        }
    };
    for g in raw[..HEADER_SIZE].chunks(3) {
        push(g, &mut out, &mut k);
    }
    k = 0;
    let groups = ((n + 1) as usize * 188 + 10) / 3;
    for i in 0..groups {
        let mut g = [0u8; 3];
        for (j, v) in g.iter_mut().enumerate() {
            *v = *raw.get(a + i * 3 + j).unwrap_or(&0);
        }
        push(&g, &mut out, &mut k);
    }
    out.push_str(&format!("}}\r\n{{Titel: {title}}}\r\n"));
    out
}

/// `.m3i` image files: header, G-buffer (`TsiLight5`, 18 bytes per pixel of
/// a tile) and the formula addon.
pub fn raw_from_m3i(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < HEADER_SIZE {
        return Err("file too small for a MB3D image".into());
    }
    let r = Rd(data);
    let mand_id = r.i32(0);
    if mand_id < 20 {
        return crate::m3p_old::raw_from_old_file(data);
    }
    let (w, h) = (r.i32(4), r.i32(8));
    let tiling = if mand_id < 35 { 0 } else { r.i32(428) };
    let (tw, th) = if tiling == 0 {
        (w, h)
    } else {
        let crop = ((tiling >> 28) & 3).max(1);
        let (tx, ty) = ((tiling & 0x7F).max(1), ((tiling >> 7) & 0x7F).max(1));
        (w / tx + 2 * crop, h / ty + 2 * crop)
    };
    let px = (tw.clamp(1, 30000) as usize) * (th.clamp(1, 30000) as usize);
    let off = HEADER_SIZE + px * 18;
    let mut raw = data[..HEADER_SIZE].to_vec();
    if data.len() >= off + 8 {
        raw.extend_from_slice(&data[off..data.len().min(off + ADDON_SIZE)]);
    }
    raw.resize(HEADER_SIZE + ADDON_SIZE, 0);
    Ok(raw)
}

/// The G-buffer of an `.m3i` file (for images without tiling); None when
/// the file holds no complete image.
pub fn gbuffer_from_m3i(data: &[u8]) -> Option<(usize, usize, Vec<crate::gbuffer::SiLight>)> {
    if data.len() < HEADER_SIZE {
        return None;
    }
    let r = Rd(data);
    if (0..20).contains(&r.i32(0)) {
        return crate::m3p_old::gbuffer_from_old_m3i(data);
    }
    let (w, h) = (r.i32(4), r.i32(8));
    if r.i32(0) < 20 || w < 1 || h < 1 || (r.i32(0) >= 35 && r.i32(428) != 0) {
        return None;
    }
    let (w, h) = (w as usize, h as usize);
    let end = w.checked_mul(h).and_then(|n| n.checked_mul(18)).and_then(|n| n.checked_add(HEADER_SIZE));
    if end.is_none_or(|end| data.len() < end) {
        return None;
    }
    let g = (0..w * h)
        .map(|i| {
            let o = HEADER_SIZE + i * 18;
            crate::gbuffer::SiLight {
                normal: [r.i16(o), r.i16(o + 2), r.i16(o + 4)],
                zpos_fine: r.u32(o + 6),
                shadow: r.u16(o + 10),
                amb_shadow: r.u16(o + 12),
                si_gradient: r.u16(o + 14),
                otrap: r.u16(o + 16),
            }
        })
        .collect();
    Some((w, h, g))
}

/// The hybrid options of files before MandId 42 (`LoadParameter`): the
/// repeat slot was stored in `bVolLightNr`, and DE combinations of type 2 and
/// 6 kept the formulas in another order.
fn upgrade_hybrid_options(d: &mut [u8], mand_id: i32) {
    let a = HEADER_SIZE;
    if mand_id < 33 {
        // iMaxItsF2 := Iterations
        let it = d[12..16].to_vec();
        d[418..422].copy_from_slice(&it);
    }
    if d.len() < a + ADDON_SIZE || !(16..=99).contains(&d[a]) {
        return;
    }
    let its = |d: &[u8], i: usize| i32::from_le_bytes(d[a + 8 + i * 188..a + 12 + i * 188].try_into().unwrap());
    let vl = d[343].min(5);
    d[343] = vl;
    let mut j = 5;
    while j > 0 && its(d, j) == 0 {
        j -= 1;
    }
    let opt1 = d[a + 1];
    let mut opt3 = d[a + 3];
    let mut hyb1: u8 = if opt1 != 0 { 0 } else { j as u8 | vl << 4 };
    let mut hyb2: u16 = 0x151;
    if opt1 == 2 {
        hyb2 = 1 | (j as u16) << 4 | (vl.max(1) as u16) << 8;
        d[a + 8..a + 12].copy_from_slice(&1i32.to_le_bytes());
        if opt3 != 2 && opt3 != 6 {
            // FlipInt(iMaxItsF2, Iterations)
            let (x, y) = (d[12..16].to_vec(), d[418..422].to_vec());
            d[12..16].copy_from_slice(&y);
            d[418..422].copy_from_slice(&x);
        }
    } else {
        opt3 = 0;
    }
    if opt3 == 2 || opt3 == 6 {
        // first formula moves behind the others
        let first = d[a + 8..a + 8 + 188].to_vec();
        for i in 0..j {
            let src = d[a + 8 + (i + 1) * 188..a + 8 + (i + 2) * 188].to_vec();
            d[a + 8 + i * 188..a + 8 + (i + 1) * 188].copy_from_slice(&src);
        }
        d[a + 8 + j * 188..a + 8 + (j + 1) * 188].copy_from_slice(&first);
        if opt3 == 6 {
            opt3 = 5;
        }
        hyb1 = (j.max(1) - 1) as u8 | (vl.max(1) - 1) << 4;
        hyb2 = j as u16 | (j as u16) << 4 | (j as u16) << 8;
    }
    d[a + 3] = opt3;
    d[a + 5] = hyb1;
    d[a + 6..a + 8].copy_from_slice(&hyb2.to_le_bytes());
}

/// Reads parameters in the `TMandHeader10` layout (files before MandId 20
/// are converted to it by [`raw_from_bytes`] and friends, keeping their
/// MandId).
pub fn parse(data: &[u8]) -> Result<M3pFile, String> {
    if data.len() < HEADER_SIZE {
        return Err("file too small for a MB3D parameter header".into());
    }
    let mand_id = i32::from_le_bytes(data[0..4].try_into().unwrap());
    if mand_id < 0 {
        return Err(format!("parameter version {mand_id} is not a MB3D version"));
    }
    let mut upgraded = data.to_vec();
    if mand_id < 42 {
        upgrade_hybrid_options(&mut upgraded, mand_id);
    }
    if mand_id < 22 {
        // UpdateFormulaOptionAbove20: the gamma (TBoptions bits 23..28) moved
        let o = 432 + 44;
        let t = u32::from_le_bytes(upgraded[o..o + 4].try_into().unwrap());
        let t = (t & 0xE07F_FFFF) | ((t & 0x1F00_0000) >> 1) | 0x1000_0000;
        upgraded[o..o + 4].copy_from_slice(&t.to_le_bytes());
    }
    upgrade_light(&mut upgraded);
    let data: &[u8] = &upgraded;
    let r = Rd(data);
    let mand_id = r.i32(0);
    let mut w: Vec<String> = Vec::new();
    let mut s = Scene::default();
    s.width = r.i32(4);
    s.height = r.i32(8);
    s.iterations = r.i32(12);
    let ioptions = r.u16(16);
    let new_options = r.u8(18);
    s.z_start = r.f64(20);
    s.z_end = r.f64(28);
    s.mid = [r.f64(36), r.f64(44), r.f64(52)];
    s.rot_4d = [r.f64(60), r.f64(68), r.f64(76)];
    s.zoom = r.f64(84);
    if !(1..32768).contains(&s.width) || !(1..32768).contains(&s.height) {
        return Err(format!("invalid image size {}x{}", s.width, s.height));
    }
    if !(s.zoom.is_finite() && s.zoom > 0.0) {
        return Err(format!("invalid zoom {}", s.zoom));
    }
    s.rstop = Some(r.f64(92));
    s.fov_y = r.f64(108);
    s.normals_on_de = r.u8(132) != 0;
    s.bin_search_steps = r.u8(134) as i32;
    s.min_iterations = r.u16(135) as i32;
    // the statistics of the calculation (iAvrgDEsteps, iAvrgIts, iMaxIts)
    s.stats = crate::scene::CalcStats { avg_de_steps10: r.i32(140), avg_its10: r.i32(144), max_its: r.i32(414) };
    s.optic = match r.u8(148) & 3 {
        1 => CameraOptic::Planar,
        2 | 3 => CameraOptic::Panorama,
        _ => CameraOptic::Common,
    };
    s.vary_de_stop_on_fov = r.u8(162) != 0;
    s.de_stop = r.f32(177);
    s.z_step_div = r.f32(182).clamp(0.001, 1.0);
    s.julia = r.u8(190) != 0;
    s.julia_c = [r.f64(191), r.f64(199), r.f64(207), r.f64(215)];
    s.dfog_on_it = r.u8(223) as u16;
    s.raystep_limiter = r.f32(242);
    let mut m = [[0.0; 3]; 3];
    for (i, row) in m.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = r.f64(246 + (i * 3 + j) * 8);
        }
    }
    if mand_id > 43 && (new_options & 1) != 0 {
        m = matrix_from_quat([m[0][0], m[0][1], m[0][2], m[1][0]]);
    }
    s.vgrads = m;
    s.color_mul = r.f32(338);
    s.color_option = r.u8(342).min(5);
    // bVolLightNr: light number (bits 0..2), map size (bits 4..7, 2 = 0)
    let vl = r.u8(343);
    s.vol_light = if mand_id >= 44 && vl & 7 != 0 {
        Some(crate::vollight::VolLightParams { light: ((vl & 7) as usize - 1).min(5), map_size: (vl >> 4) as i32 - 2 })
    } else {
        None
    };
    s.color_on_it = r.u8(19);
    if mand_id >= 27 {
        let sf = |o: usize| short_float(r.u16(o));
        let m = &mut s.mc;
        let fin = |v: f64, d: f32| if v.is_finite() { v as f32 } else { d };
        m.reflection_amount = fin(r.f32(332), 0.5);
        let sr = r.u8(336);
        m.reflections = sr & 1 != 0;
        m.transparency = sr & 2 != 0;
        m.only_difs = sr & 4 != 0;
        m.reflection_depth = r.u8(337);
        if mand_id >= 37 {
            m.refraction_index = fin(r.f32(116), 1.5);
            m.absorption = fin(r.f32(370), 0.2);
        }
        if mand_id >= 38 {
            m.scattering = fin(r.f32(120), 1.0);
        }
        m.options = r.u8(124) & 0x7F;
        m.diffuse_reflects = r.u8(125);
        m.depth = r.u8(186).max(1);
        m.soft_shadow_radius = sf(224);
        m.saturation = r.u8(318) & 0x7F;
        m.contrast = r.u8(423);
        // the upgrades of LoadParameter
        if mand_id < 40 {
            m.soft_shadow_radius = 1.0;
            m.depth = 3;
            m.contrast = 128;
            m.options = 2;
            m.diffuse_reflects = 0;
        }
        if mand_id < 42 {
            m.options |= 16;
        }
        if mand_id < 43 {
            m.saturation = 32;
        }
    } else {
        s.mc.options = 2 | 16;
    }
    if r.u8(344) == 0 {
        // bSliceCalc: 1 start, 3 end, everything else the middle
        s.slice_2d = match r.u8(345) {
            1 => 1,
            3 => 3,
            _ => 2,
        };
    }
    s.normals_on_zbuf = mand_id >= 27 && r.u8(331) & 1 != 0;
    s.stereo_mode = match r.u8(126) {
        m @ (1 | 3 | 4) => m,
        _ => 0,
    };
    let st = [r.f32(230) as f32, r.f32(234) as f32, r.f32(238) as f32];
    if st.iter().all(|v| v.is_finite() && *v > 0.0) {
        s.stereo_screen = st;
    }
    let tiling = if mand_id >= 35 { r.i32(428) as u32 } else { 0 };
    if tiling != 0 {
        // TilingOptions: bits 0..6 tile count H, 7..13 count V, 14..20 pos X,
        // 21..27 pos Y, 28..29 downscale
        let (cols, rows) = ((tiling & 0x7F).max(1), ((tiling >> 7) & 0x7F).max(1));
        let pos = (((tiling >> 14) & 0x7F).min(cols - 1), ((tiling >> 21) & 0x7F).min(rows - 1));
        s.tiling = Some(crate::scene::Tiling { cols, rows, pos: Some(pos), downscale: ((tiling >> 28) & 3).max(1) });
        w.push(format!("tile {}, {} of a {cols}x{rows} tiled render (use -s tile=all for the whole image)", pos.0 + 1, pos.1 + 1));
    }
    s.smooth_normals = ((ioptions >> 6) & 15).min(8) as i32;
    s.first_step_random = ioptions & 1 != 0;
    s.step_sub_de_stop = ioptions & 4 != 0;
    // ambient shadow (bCalcAmbShadowAutomatic): bit0 auto, bit1 "threshold
    // to 0", bits 2..3 type (0: 15 bit, 1: 24 bit, 2: 24 bit random, 3: DEAO)
    let ao_bits = r.u8(149);
    s.ao = if ao_bits & 1 != 0 {
        let kind = (ao_bits >> 2) & 3;
        if kind == 3 {
            s.deao = Some(crate::deao::DeaoParams {
                quality: (ao_bits >> 4) & 3,
                dither: r.u8(188).min(2),
                max_len: if mand_id >= 30 { (r.f32(374) as f32).max(0.01) } else { 1.0 },
                first_step_random: ao_bits & 128 != 0,
            });
        }
        Some(crate::ssao::SsaoParams {
            threshold: r.f32(319) as f32,
            border_mirror: (r.u8(127) as f32 * 0.01).min(0.9),
            t0: ao_bits & 2 != 0,
            random: if kind == 2 { r.u8(187).max(1) } else { 0 },
            bits15: kind == 0,
            wrap_x: false,
        })
    } else {
        None
    };
    // hard shadows: bCalculateHardShadow (bit0 auto, bit1 set cos, bits 2..7
    // lights), bCalc1HSsoft, MCSoftShadowRadius, HSmaxLengthMultiplier
    let hs = r.u8(133);
    s.shadows = if hs & 1 != 0 && hs >> 2 != 0 {
        let mid = mand_id;
        Some(crate::scene::ShadowParams {
            lights: hs >> 2,
            soft: mid >= 40 && r.u8(139) & 1 != 0,
            soft_radius: if mid >= 40 { short_float(r.u16(224)).clamp(0.01, 20.0) } else { 1.0 },
            max_len_mul: if mid >= 25 { (r.f32(226) as f32).max(0.01) } else { 1.0 },
            set_cos: hs & 2 != 0,
        })
    } else {
        None
    };
    // cutting planes: bCutOption bits 0..2, dCutX/Y/Z
    s.cut_options = r.u8(176) & 7;
    s.cut_pos = [r.f64(346), r.f64(354), r.f64(362)];
    // depth of field: bCalcDOFtype bit0 auto, bits 1..2 passes - 1, bit3 forward
    let dt = r.u8(181);
    s.dof = if dt & 1 != 0 {
        let zs = r.f32(164) as f32;
        Some(crate::dof::DofParams {
            z_sharp: zs,
            z_sharp2: if mand_id >= 32 { r.f32(410) as f32 } else { zs },
            clip_r: (r.f32(168) as f32).clamp(0.1, 1000.0),
            aperture: (r.f32(172) as f32).clamp(0.0001, 2.0),
            passes: ((dt >> 1) & 3) + 1,
            forward: (dt >> 3) & 1 != 0,
        })
    } else {
        None
    };

    s.lighting = parse_light(&r, &mut w);
    if let Some(v) = s.vol_light {
        if !s.lighting.lights[v.light].on {
            w.push(format!("volumetric light: light {} is off, no volumetric light", v.light + 1));
            s.vol_light = None;
        }
    }

    // ---- formulas (THeaderCustomAddon) ----
    s.formulas.clear();
    if data.len() >= HEADER_SIZE + ADDON_SIZE && (16..=99).contains(&data[HEADER_SIZE]) {
        let a = HEADER_SIZE;
        let opt1 = r.u8(a + 1);
        let opt2 = r.u8(a + 2);
        let hyb1 = r.u8(a + 5);
        let interp = opt1 & 3 == 1;
        if interp {
            s.interpolation = Some([r.f32(a + 8) as f32, r.f32(a + 8 + 188) as f32]);
        }
        match opt1 & 3 {
            2 => {
                // CheckHybridOptions
                let hyb2 = r.u16(a + 6) as usize;
                let its = |i: usize| r.i32(a + 8 + i * 188);
                let mut x = 5;
                while x > 0 && its(x) == 0 {
                    x -= 1;
                }
                let start2 = (hyb2 & 7).min(x).max(1);
                let end2 = start2.max(x);
                let mut repeat2 = ((hyb2 >> 8) & 7).min(end2).max(start2);
                let mut x2 = 5;
                while x2 > start2 && its(x2) <= 0 {
                    x2 -= 1;
                }
                repeat2 = repeat2.min(x2);
                s.decomb = Some(crate::scene::DeCombParams {
                    kind: (r.u8(a + 3) as i32 + 1).clamp(1, 6) as u8,
                    end1: start2 - 1,
                    start2,
                    end2,
                    repeat2,
                    iterations2: r.i32(418),
                    smooth: if mand_id >= 31 { r.f32(378) as f32 } else { 0.5 },
                    mix_pow: if mand_id >= 39 { r.f32(104) as f32 } else { 2.0 },
                    mix_color: if mand_id >= 34 { r.u8(422) } else { 0 },
                });
            }
            _ => {}
        }
        if opt2 & 1 != 0 {
            s.disable_analytic_de = true;
        }
        s.inside = if opt2 & 4 != 0 {
            crate::scene::InsideMode::Both
        } else if opt2 & 2 != 0 {
            crate::scene::InsideMode::Inside
        } else {
            crate::scene::InsideMode::Outside
        };
        s.repeat_from = (hyb1 >> 4) as usize;
        for i in 0..6 {
            let o = a + 8 + i * 188;
            let its = r.i32(o);
            let fnr = r.i32(o + 4);
            let nopt = r.i32(o + 8).clamp(0, 16) as usize;
            let name = r.str(o + 12, 32);
            let vals: Vec<f64> = (0..16).map(|k| r.f64(o + 60 + 8 * k)).collect();
            // interpolation hybrid: slots 0 and 1 hold weights (singles)
            let its = if interp {
                if i < 2 && fnr >= 0 {
                    1
                } else {
                    0
                }
            } else {
                its
            };
            if its == 0 {
                s.formulas.push(FormulaEntry { formula: Formula::default_for("Integer Power").unwrap(), iterations: 0 });
                continue;
            }
            let f = match fnr {
                0 => Formula::IntPow { power: (vals[0].round() as i32).clamp(2, 8), z_mul: vals[1] },
                1 => Formula::RealPower { power: vals[0], z_mul: vals[1] },
                2 => Formula::Quaternion { yw_mul: vals[0], w_add: vals[1] },
                3 => Formula::Tricorn { z_mul: vals[0], cz_mul: vals[1] },
                4 => Formula::AmazingBox { scale: vals[0], min_r: vals[1], fold: vals[2] },
                5 => Formula::Bulbox {
                    scale: vals[0],
                    min_r: vals[1],
                    fold: vals[2],
                    bulb_scale: vals[3],
                    r_threshold: vals[4],
                    r_threshold2: if nopt < 6 { vals[4] } else { vals[5] },
                },
                6 => Formula::FoldingIntPow { power: (vals[0].round() as i32).clamp(2, 8), z_mul: vals[1], fold: vals[2] },
                9 => Formula::AexionC {
                    power: vals[0],
                    z_mul: vals[1],
                    rot_c: vals[2].round() != 0.0,
                    cond_phi: vals[3].round() != 0.0,
                    power_c: vals[4],
                    cz_mul: vals[5],
                    // files from before the 8 option version have 6 options
                    powc_dist: nopt > 6 && vals[6].round() != 0.0,
                    mode: if nopt > 7 { (vals[7].round() as i32).clamp(0, 31) } else { 0 },
                },
                0..=19 => return Err(format!("internal formula #{fnr} ('{name}') is not supported yet")),
                _ => {
                    let mut f = crate::formulas::lookup(&name)?;
                    if let Formula::Custom(c) = &mut f {
                        for k in 0..nopt.min(c.values.len()) {
                            c.values[k] = vals[k];
                        }
                    } else {
                        for k in 0..nopt {
                            f.set_option(&format!("option{k}"), vals[k])?;
                        }
                    }
                    f
                }
            };
            s.formulas.push(FormulaEntry { formula: f, iterations: its });
        }
        while s.formulas.len() > 1 && s.formulas.last().map(|f| f.iterations == 0).unwrap_or(false) {
            s.formulas.pop();
        }
    } else {
        w.push("no formula block found, using the default formula".into());
        s.formulas = Scene::default().formulas;
    }
    // the upgrades of LoadParameter (FileHandling.pas): older files do not
    // have these fields, their bytes are other data (a raystep limiter read
    // from a version 20 file made the steps a hundred times too short)
    if mand_id < 21 {
        if r.f32(182) < 0.01 {
            s.z_step_div = 0.3;
        }
        s.optic = CameraOptic::Common;
    }
    if mand_id < 22 {
        s.color_option = 0;
        s.rot_4d = [0.0; 3];
        s.color_mul = 1.0;
        s.julia_c[3] = 0.0;
    }
    if mand_id < 23 {
        s.raystep_limiter = 1.0;
    }
    if mand_id < 24 {
        s.stereo_screen = [1.0, 2.0, 0.5];
    }
    if mand_id < 26 {
        if let Some(d) = s.deao.as_mut() {
            d.dither = 0;
        }
    }
    if mand_id < 28 {
        if let Some(a) = s.ao.as_mut() {
            a.random = a.random.min(1);
        }
    }
    if mand_id < 29 {
        if let Some(f) = s.dof.as_mut() {
            f.clip_r = 25.0;
            f.aperture = 0.02;
        }
    }
    if mand_id < 35 {
        s.dfog_on_it = 0;
    }
    if mand_id < 36 {
        if let Some(a) = s.ao.as_mut() {
            a.border_mirror = 0.0;
        }
        s.step_sub_de_stop = false;
    }
    Ok(M3pFile { scene: s, warnings: w, mand_id })
}

/// `UpdateLightParasAbove3`: upgrades the light record of older MB3D
/// versions in place (light version in `TBoptions` bits 20..22, extended
/// version in `Lights[0].AdditionalByteEx`).  Returns the original version.
fn upgrade_light(d: &mut [u8]) -> u32 {
    let b = 432;
    let rd_u32 = |d: &[u8], o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
    let rd_i32 = |d: &[u8], o: usize| i32::from_le_bytes(d[o..o + 4].try_into().unwrap());
    let light = |i: usize| b + 68 + i * 32;
    let tbo = rd_u32(d, b + 44);
    let ver = (tbo >> 20) & 7;
    let mut verex = 0u8;
    if ver < 4 {
        // DynFogCol2 := DynFogR/G/B
        d[b + 4] = d[b + 55];
        d[b + 5] = d[b + 59];
        d[b + 6] = d[b + 63];
    }
    if ver < 5 {
        let tb6 = rd_i32(d, b + 20);
        d[b + 20..b + 24].copy_from_slice(&((tb6 - 53) * 2 + 53).clamp(0, 159).to_le_bytes());
        let vz = i16::from_le_bytes([d[b], d[b + 1]]) as i32;
        d[b..b + 2].copy_from_slice(&((vz * 2).clamp(-120, 360) as i16).to_le_bytes());
    }
    if ver < 6 {
        d[b + 3] = 0; // bColorMap
        d[b + 2] = 255; // RoughnessFactor
        d[b + 67] = d[b + 67].wrapping_add(128); // PicOffsetZ
        for i in 0..6 {
            d[light(i)] &= 0xFD;
        }
    }
    if ver < 7 {
        for i in 0..6 {
            let o = light(i);
            d[o + 8] = 0; // LightMapNr and $FF
            d[o + 16] = 0;
            d[o + 24] = 0;
        }
    } else {
        verex = d[light(0) + 16];
    }
    let tbo = (tbo & 0xFF8F_FFFF) | (7 << 20);
    d[b + 44..b + 48].copy_from_slice(&tbo.to_le_bytes());
    if verex < 1 {
        d[light(0) + 24] = 0;
    }
    if verex < 2 {
        d[light(2) + 16] = 30; // diffuse map scale
        d[light(1) + 24] = (d[b + 7] >> 1) & 1; // diffuse map on normals
        d[b + 7] &= 0xFB;
    }
    if verex < 3 {
        d[light(3) + 16] = 128; // diffuse shadowing
        d[light(2) + 24] = 0; // iExModes
    }
    if verex < 5 {
        for i in 0..4 {
            d[b + 360 + i * 6 + 5] = 0;
        }
        for i in 0..10 {
            let o = b + 260 + i * 10 + 6;
            d[o + 3] = d[o].max(d[o + 1]).max(d[o + 2]);
        }
    }
    if verex < 6 {
        let i = (d[b + 7] >> 4) & 7;
        d[b + 7] = (d[b + 7] & 0x8F) | if i != 0 { 16 } else { 0 };
        for i in 0..6 {
            let o = light(i);
            if d[o] & 6 != 4 {
                d[o + 2..o + 4].copy_from_slice(&to_short_float(1.0).to_le_bytes());
            }
        }
        d[light(4) + 16] = 40; // background brightness
        for i in 0..4 {
            let o = b + 360 + i * 6 + 2;
            let y = (d[o] as f32 * 0.3 + d[o + 1] as f32 * 0.59 + d[o + 2] as f32 * 0.11) / 255.0;
            d[o + 3] = (y * 255.0).round() as u8;
        }
    }
    if verex < 7 {
        for i in 0..6 {
            d[light(i) + 1] &= 0x3F;
        }
    }
    if verex < 8 {
        d[light(0) + 24] &= 1;
        d[light(3) + 24] = 0;
    }
    d[light(0) + 16] = 8;
    ver
}

fn parse_light(r: &Rd, w: &mut Vec<String>) -> Lighting {
    let b = 432; // TLightingParas9
    let mut l = Lighting::default();
    let tb = |k: usize| r.i32(b + 8 + (k - 3) * 4);
    let tb_options = r.u32(b + 44);
    l.var_col_z = r.i16(b) as f32;
    l.roughness = r.u8(b + 2) as f32;
    l.dyn_fog_col2 = r.rgb(b + 4);
    let add_opt = r.u8(b + 7);
    let light = |i: usize| b + 68 + i * 32;
    l.internal_gamma2 = add_opt & 1 != 0;
    l.yc_comb = add_opt & 4 != 0;
    l.bg_ambient = add_opt & 32 != 0;
    l.amb_rel_obj = tb_options & 0x2000_0000 != 0;
    l.dfog_options = r.u8(light(0) + 24) & 3;
    l.ex_mode = r.u8(light(2) + 24);
    l.no_col_ipol = r.u8(light(3) + 24) & 1 != 0;
    l.diff_map = r.u8(b + 3) as u16 | (r.u8(light(1) + 16) as u16) << 8;
    l.diff_map_mode = r.u8(light(1) + 24) & 3;
    l.diff_map_offset = [((tb(7) >> 12) & 0xFF) as u8, ((tb(7) >> 20) & 0xFF) as u8];
    l.diff_map_rot = ((tb(8) >> 20) & 0xFF) as u8;
    l.diff_map_scale = r.u8(light(2) + 16);
    if l.diff_map > 0 && crate::maps::by_number(l.diff_map as i32).is_none() {
        w.push(format!("diffuse colour map {} not found (use --maps DIR)", l.diff_map));
    }
    l.fog_offset = (tb(3) & 0xFFFF) as f32;
    l.depth_fog = tb(4) as f32;
    l.diffuse = tb(5) as f32;
    l.dyn_fog = tb(6) as f32;
    l.specular = (tb(7) & 0xFFF) as f32;
    l.ambient = (tb(8) & 0xFFF) as f32;
    l.color_start = tb(9) as f32;
    l.color_end = tb(10) as f32;
    l.amb_shadow = (tb(11) & 0xFF) as f32;
    l.ind_light = ((tb(11) >> 8) & 0xFF) as u8 as i8 as f32 + 53.0;
    l.color_cycling = tb_options & 0x4000 != 0;
    l.color_on_otrap = (tb_options >> 17) & 1 != 0;
    l.far_fog = tb_options & 0x40000 != 0;
    l.depth_func = (tb_options >> 30) as u8;
    l.gamma = ((tb_options >> 23) & 0x3F) as f32;
    l.interior_start = (tb_options & 0x7F) as f32;
    l.interior_end = ((tb_options >> 7) & 0x7F) as f32;
    if tb_options & 0x10000 != 0 {
        l.fine_col_adj = Some((r.u8(b + 48), r.u8(b + 49)));
    }
    l.amb_top = r.rgb(b + 52);
    l.amb_bottom = r.rgb(b + 56);
    l.depth_col = r.rgb(b + 60);
    l.depth_col2 = r.rgb(b + 64);
    l.dyn_fog_col = [r.u8(b + 55), r.u8(b + 59), r.u8(b + 63)];
    for i in 0..6 {
        let o = b + 68 + i * 32;
        let lopt = r.u8(o);
        let lfunc = r.u8(o + 1);
        let on = (lopt & 3) == 0;
        let map = if (lopt & 3) == 2 { r.u16(o + 7) } else { 0 };
        let positional = (lopt & 4) != 0;
        l.lights[i] = Light {
            on,
            positional,
            position: [r.d7b(o + 9), r.d7b(o + 17), r.d7b(o + 25)],
            visible: if on { (((lopt >> 2) & 7) | ((lfunc & 128) >> 4)) & 14 } else { 0 },
            color: r.rgb(o + 4),
            x_angle: r.d7b(o + 9),
            y_angle: r.d7b(o + 17),
            amplitude: short_float(r.u16(o + 2)),
            spec_func: (lfunc & 7) as i32,
            diff_func: ((lfunc >> 4) & 3) as i32,
            relative_to_object: (lopt >> 5) & 1 != 0,
            hs_enabled: (lopt >> 6) & 1 == 0,
            map,
            map_rot: [r.u8(o + 9), r.u8(o + 17), r.u8(o + 25)],
        };
    }
    l.diffuse_shadowing = r.u8(b + 68 + 3 * 32 + 16) as f32 / 256.0;
    for i in 0..10 {
        let o = b + 260 + i * 10;
        let dif = r.u32(o + 2);
        let spe = r.u32(o + 6);
        l.palette[i] = PaletteColor {
            position: r.u16(o),
            diffuse: [dif as u8, (dif >> 8) as u8, (dif >> 16) as u8],
            specular: [spe as u8, (spe >> 8) as u8, (spe >> 16) as u8],
        };
    }
    let alpha: [u8; 10] = std::array::from_fn(|i| (r.u32(b + 260 + i * 10 + 6) >> 24) as u8);
    if (0..10).any(|i| alpha[i] != l.palette_alpha(i)) {
        l.palette_alpha = Some(alpha);
    }
    for i in 0..4 {
        let o = b + 360 + i * 6;
        let c = r.u32(o + 2);
        l.interior[i] = (r.u16(o), [c as u8, (c >> 8) as u8, (c >> 16) as u8]);
        l.interior_spec[i] = (c >> 24) as u8;
    }
    let bg_name = r.str(b + 384, 24);
    if !bg_name.is_empty() && bg_name.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        l.bg_image = bg_name;
        l.bg_direct = tb_options & 0x8000 != 0;
        l.bg_rot = [r.u8(b + 50), r.u8(b + 51), r.u8(b + 67)];
        l.bg_brightness = r.u8(b + 68 + 4 * 32 + 16);
        l.bg_add_light = add_opt & 8 != 0;
        if crate::maps::by_name(&l.bg_image).is_none() {
            w.push(format!("background picture '{}' not found (put it into M3Maps or use --maps DIR)", l.bg_image));
        }
    }
    for (i, li) in l.lights.iter().enumerate() {
        if li.map > 0 && crate::maps::by_number(li.map as i32).is_none() {
            w.push(format!("light {}: light map {} not found (use --maps DIR)", i + 1, li.map));
        }
    }
    l
}

/// The lighting of a light record (`TLightingParas9`, .m3l files and the
/// header from byte 432).
pub fn light_from_record(rec: &[u8]) -> Option<Lighting> {
    if rec.len() < 408 {
        return None;
    }
    let mut h = vec![0u8; HEADER_SIZE];
    h[432..840].copy_from_slice(&rec[..408]);
    let mut w = Vec::new();
    Some(parse_light(&Rd(&h), &mut w))
}

pub fn to_short_float_pub(v: f32) -> u16 {
    to_short_float(v)
}

/// A value rounded to MB3D's `ShortFloat` (as the light intensity is stored).
pub fn short_float_round(v: f32) -> f32 {
    short_float(to_short_float(v))
}

/// `SingleToShortFloat`: the closest m * 10^(e - 1) with byte m and e.
pub(crate) fn to_short_float(v: f32) -> u16 {
    // MB3D's SingleToShortFloat (Math3D.pas)
    if v.abs() < 1e-45 || !v.is_finite() {
        return 0;
    }
    if v.abs() > 1e38 {
        return 99 | (38u16 << 8);
    }
    let (mut s, mut e) = (v, 0i32);
    while s.abs() >= 9.95 {
        s *= 0.1;
        e += 1;
    }
    while s.abs() <= 0.995 {
        s *= 10.0;
        e -= 1;
    }
    let m = (s * 10.0).round() as i32;
    (m as i8 as u8 as u16) | ((e as i8 as u8 as u16) << 8)
}

/// Writes a scene as a MB3D parameter file (`.m3p`, MandId 44): the
/// `TMandHeader10` with the light record and the formula block.  Settings
/// that this port does not model keep MB3D's neutral values.
pub fn write(sc: &Scene) -> Vec<u8> {
    let mut d = vec![0u8; HEADER_SIZE + ADDON_SIZE];
    fn put(d: &mut [u8], o: usize, b: &[u8]) {
        d[o..o + b.len()].copy_from_slice(b);
    }
    let pi32 = |d: &mut Vec<u8>, o: usize, v: i32| put(d, o, &v.to_le_bytes());
    let pf64 = |d: &mut Vec<u8>, o: usize, v: f64| put(d, o, &v.to_le_bytes());
    let pf32 = |d: &mut Vec<u8>, o: usize, v: f32| put(d, o, &v.to_le_bytes());
    let pu16 = |d: &mut Vec<u8>, o: usize, v: u16| put(d, o, &v.to_le_bytes());
    let pd7b = |d: &mut Vec<u8>, o: usize, v: f64| put(d, o, &v.to_le_bytes()[1..8]);
    let prgb = |d: &mut Vec<u8>, o: usize, c: [u8; 3]| put(d, o, &c);
    pi32(&mut d, 0, 44);
    pi32(&mut d, 4, sc.width);
    pi32(&mut d, 8, sc.height);
    pi32(&mut d, 12, sc.iterations);
    if let Some(tl) = sc.tiling {
        if let Some((x, y)) = tl.pos {
            let v = (tl.cols & 0x7F) | (tl.rows & 0x7F) << 7 | (x & 0x7F) << 14 | (y & 0x7F) << 21 | (tl.downscale.clamp(1, 3)) << 28;
            pi32(&mut d, 428, v as i32);
        }
    }
    let ioptions = (sc.first_step_random as u16) | (sc.step_sub_de_stop as u16) << 2 | ((sc.smooth_normals.clamp(0, 8) as u16) << 6);
    pu16(&mut d, 16, ioptions);
    pf64(&mut d, 20, sc.z_start);
    pf64(&mut d, 28, sc.z_end);
    for k in 0..3 {
        pf64(&mut d, 36 + 8 * k, sc.mid[k]);
        pf64(&mut d, 60 + 8 * k, sc.rot_4d[k]);
    }
    pf64(&mut d, 84, sc.zoom);
    pf64(&mut d, 92, sc.effective_rstop());
    pf64(&mut d, 108, sc.fov_y);
    d[132] = sc.normals_on_de as u8;
    d[134] = sc.bin_search_steps.clamp(0, 255) as u8;
    pu16(&mut d, 135, sc.min_iterations.clamp(0, 65535) as u16);
    d[148] = match sc.optic {
        CameraOptic::Common => 0,
        CameraOptic::Planar => 1,
        CameraOptic::Panorama => 2,
    };
    if let Some(a) = &sc.ao {
        let kind: u8 = if sc.deao.is_some() {
            3
        } else if a.bits15 {
            0
        } else if a.random > 0 {
            2
        } else {
            1
        };
        let mut bits = 1 | (a.t0 as u8) << 1 | kind << 2;
        if let Some(de) = &sc.deao {
            bits |= (de.quality & 3) << 4 | (de.first_step_random as u8) << 7;
            d[188] = de.dither;
            pf32(&mut d, 374, de.max_len);
        }
        d[149] = bits;
        pf32(&mut d, 319, a.threshold);
        d[127] = (a.border_mirror * 100.0).round() as u8;
        d[187] = a.random;
    }
    if let Some(h) = &sc.shadows {
        d[133] = 1 | (h.set_cos as u8) << 1 | (h.lights & 0x3F) << 2;
        d[139] = h.soft as u8;
        pu16(&mut d, 224, to_short_float(h.soft_radius));
        pf32(&mut d, 226, h.max_len_mul);
    } else {
        pf32(&mut d, 226, 1.0);
        // MCSoftShadowRadius is shared with the Monte Carlo renderer
        pu16(&mut d, 224, to_short_float(sc.mc.soft_shadow_radius));
    }
    d[162] = sc.vary_de_stop_on_fov as u8;
    if let Some(f) = &sc.dof {
        pf32(&mut d, 164, f.z_sharp);
        pf32(&mut d, 168, f.clip_r);
        pf32(&mut d, 172, f.aperture);
        d[181] = 1 | ((f.passes.clamp(1, 4) - 1) << 1) | (f.forward as u8) << 3;
        pf32(&mut d, 410, f.z_sharp2);
    }
    d[176] = sc.cut_options & 7;
    for k in 0..3 {
        pf64(&mut d, 346 + 8 * k, sc.cut_pos[k]);
    }
    pf32(&mut d, 177, sc.de_stop as f32);
    pf32(&mut d, 182, sc.z_step_div as f32);
    d[190] = sc.julia as u8;
    for k in 0..4 {
        pf64(&mut d, 191 + 8 * k, sc.julia_c[k]);
    }
    d[223] = sc.dfog_on_it.min(255) as u8;
    pf32(&mut d, 242, sc.raystep_limiter as f32);
    let m = crate::math::normalise_matrix_to(sc.step_width(), &sc.vgrads);
    for i in 0..3 {
        for j in 0..3 {
            pf64(&mut d, 246 + (i * 3 + j) * 8, m[i][j]);
        }
    }
    pf32(&mut d, 338, sc.color_mul as f32);
    d[342] = sc.color_option;
    d[343] = match &sc.vol_light {
        Some(v) => (v.light as u8 + 1) | (((v.map_size + 2).clamp(0, 15) as u8) << 4),
        None => 2 << 4,
    };
    d[344] = (sc.slice_2d == 0) as u8;
    d[345] = if sc.slice_2d == 0 { 2 } else { sc.slice_2d };
    d[19] = sc.color_on_it;
    d[126] = sc.stereo_mode;
    d[331] = sc.normals_on_zbuf as u8;
    {
        let m = &sc.mc;
        pf32(&mut d, 332, m.reflection_amount);
        d[336] = m.reflections as u8 | (m.transparency as u8) << 1 | (m.only_difs as u8) << 2;
        d[337] = m.reflection_depth;
        pf32(&mut d, 116, m.refraction_index);
        pf32(&mut d, 370, m.absorption);
        pf32(&mut d, 120, m.scattering);
        d[124] = m.options;
        d[125] = m.diffuse_reflects;
        d[186] = m.depth;
        d[318] = m.saturation;
        d[423] = m.contrast;
    }
    pf32(&mut d, 230, sc.stereo_screen[0]);
    pf32(&mut d, 234, sc.stereo_screen[1]);
    pf32(&mut d, 238, sc.stereo_screen[2]);
    pf32(&mut d, 104, sc.decomb.map(|c| c.mix_pow).unwrap_or(2.0));
    pf32(&mut d, 378, sc.decomb.map(|c| c.smooth).unwrap_or(0.5));
    pi32(&mut d, 418, sc.decomb.map(|c| c.iterations2).unwrap_or(sc.iterations));
    pi32(&mut d, 140, sc.stats.avg_de_steps10);
    pi32(&mut d, 144, sc.stats.avg_its10);
    pi32(&mut d, 414, sc.stats.max_its);
    d[422] = sc.decomb.map(|c| c.mix_color).unwrap_or(0);

    // ---- TLightingParas9 ----
    let l = &sc.lighting;
    let b = 432;
    put(&mut d, b, &(l.var_col_z.round() as i16).to_le_bytes());
    d[b + 2] = l.roughness.round().clamp(0.0, 255.0) as u8;
    prgb(&mut d, b + 4, l.dyn_fog_col2);
    d[b + 7] = (l.bg_add_light as u8) << 3 | l.internal_gamma2 as u8 | (l.yc_comb as u8) << 2 | (l.bg_ambient as u8) << 5;
    d[b + 3] = l.diff_map as u8;
    let tbv = [
        (l.fog_offset.round() as i32 & 0xFFFF) | (128 << 16),
        l.depth_fog.round() as i32,
        l.diffuse.round() as i32,
        l.dyn_fog.round() as i32,
        l.specular.round() as i32 & 0xFFF | (l.diff_map_offset[0] as i32) << 12 | (l.diff_map_offset[1] as i32) << 20,
        l.ambient.round() as i32 & 0xFFF | (l.diff_map_rot as i32) << 20,
        l.color_start.round() as i32,
        l.color_end.round() as i32,
        (l.amb_shadow.round() as i32 & 0xFF) | (((l.ind_light - 53.0).round() as i32 as i8 as u8 as i32) << 8),
    ];
    for (k, v) in tbv.iter().enumerate() {
        pi32(&mut d, b + 8 + k * 4, *v);
    }
    let mut tbo: u32 = (l.interior_start.round() as u32 & 0x7F) | (l.interior_end.round() as u32 & 0x7F) << 7 | 7 << 20;
    tbo |= (l.amb_rel_obj as u32) << 29;
    tbo |= (l.color_cycling as u32) << 14 | (l.bg_direct as u32) << 15 | (l.color_on_otrap as u32) << 17 | (l.far_fog as u32) << 18;
    tbo |= (l.gamma.round() as u32 & 0x3F) << 23 | ((l.depth_func as u32) & 3) << 30;
    if let Some((a1, a2)) = l.fine_col_adj {
        tbo |= 0x10000;
        d[b + 48] = a1;
        d[b + 49] = a2;
    }
    put(&mut d, b + 44, &tbo.to_le_bytes());
    d[b + 50] = l.bg_rot[0];
    d[b + 51] = l.bg_rot[1];
    d[b + 67] = l.bg_rot[2];
    prgb(&mut d, b + 52, l.amb_top);
    prgb(&mut d, b + 56, l.amb_bottom);
    prgb(&mut d, b + 60, l.depth_col);
    prgb(&mut d, b + 64, l.depth_col2);
    d[b + 55] = l.dyn_fog_col[0];
    d[b + 59] = l.dyn_fog_col[1];
    d[b + 63] = l.dyn_fog_col[2];
    for (i, li) in l.lights.iter().enumerate() {
        let o = b + 68 + i * 32;
        let mut lopt: u8 = if li.map > 0 {
            2
        } else if li.on {
            0
        } else {
            1
        };
        lopt |= (li.positional as u8) << 2 | (li.visible & 6) << 2 | (li.relative_to_object as u8) << 5 | (!li.hs_enabled as u8) << 6;
        d[o] = lopt;
        d[o + 1] = (li.spec_func & 7) as u8 | ((li.diff_func & 3) as u8) << 4 | (li.visible & 8) << 4;
        pu16(&mut d, o + 2, to_short_float(li.amplitude));
        prgb(&mut d, o + 4, li.color);
        if li.positional {
            pd7b(&mut d, o + 9, li.position[0]);
            pd7b(&mut d, o + 17, li.position[1]);
            pd7b(&mut d, o + 25, li.position[2]);
        } else {
            pd7b(&mut d, o + 9, li.x_angle);
            pd7b(&mut d, o + 17, li.y_angle);
        }
        if li.map > 0 {
            pu16(&mut d, o + 7, li.map);
            d[o + 9] = li.map_rot[0];
            d[o + 17] = li.map_rot[1];
            d[o + 25] = li.map_rot[2];
        }
    }
    let light = |i: usize| b + 68 + i * 32;
    d[light(0) + 16] = 8; // extended light version
    d[light(0) + 24] = l.dfog_options & 3;
    d[light(1) + 16] = (l.diff_map >> 8) as u8;
    d[light(1) + 24] = l.diff_map_mode & 3;
    d[light(2) + 16] = l.diff_map_scale;
    d[light(2) + 24] = l.ex_mode;
    d[light(3) + 24] = l.no_col_ipol as u8;
    d[b + 68 + 3 * 32 + 16] = (l.diffuse_shadowing * 256.0).round().clamp(0.0, 255.0) as u8;
    d[b + 68 + 4 * 32 + 16] = l.bg_brightness;
    for (i, c) in l.palette.iter().enumerate() {
        let o = b + 260 + i * 10;
        pu16(&mut d, o, c.position);
        prgb(&mut d, o + 2, c.diffuse);
        prgb(&mut d, o + 6, c.specular);
        d[o + 9] = l.palette_alpha(i);
    }
    for (i, (pos, c)) in l.interior.iter().enumerate() {
        let o = b + 360 + i * 6;
        pu16(&mut d, o, *pos);
        prgb(&mut d, o + 2, *c);
        d[o + 5] = l.interior_spec[i];
    }
    let bg = l.bg_image.as_bytes();
    put(&mut d, b + 384, &bg[..bg.len().min(23)]);

    // ---- THeaderCustomAddon ----
    let a = HEADER_SIZE;
    d[a] = 16;
    if sc.interpolation.is_some() {
        d[a + 1] = 1;
        d[a + 5] = 1;
        pu16(&mut d, a + 6, 0x151);
    } else if let Some(c) = &sc.decomb {
        d[a + 1] = 2;
        d[a + 3] = c.kind.clamp(1, 6) - 1;
        d[a + 5] = (c.end1 as u8 & 7) | ((sc.repeat_from.min(c.end1) as u8) << 4);
        pu16(&mut d, a + 6, (c.start2 as u16 & 7) | (c.end2 as u16 & 7) << 4 | (c.repeat2 as u16 & 7) << 8);
    } else {
        let last = sc.formulas.iter().rposition(|f| f.iterations != 0).unwrap_or(0);
        d[a + 5] = (last as u8 & 7) | ((sc.repeat_from.min(last) as u8) << 4);
        pu16(&mut d, a + 6, 0x151);
    }
    d[a + 2] = (sc.disable_analytic_de as u8)
        | match sc.inside {
            crate::scene::InsideMode::Outside => 0,
            crate::scene::InsideMode::Inside => 2,
            crate::scene::InsideMode::Both => 4,
        };
    d[a + 4] = if sc.interpolation.is_some() {
        2
    } else {
        sc.formulas.iter().rposition(|f| f.iterations != 0).map(|x| x as u8 + 1).unwrap_or(0)
    };
    let nf = if sc.interpolation.is_some() { 2 } else { 6 };
    for (i, f) in sc.formulas.iter().take(nf).enumerate() {
        let o = a + 8 + i * 188;
        pi32(&mut d, o, f.iterations);
        if let Some(w) = sc.interpolation {
            d[o..o + 4].copy_from_slice(&w[i].to_le_bytes());
        } else if f.iterations == 0 {
            continue;
        }
        let opts = f.formula.options();
        let (fnr, types): (i32, Vec<u8>) = match &f.formula {
            Formula::Custom(c) => (20, c.def.options.iter().map(|o| o.ty).collect()),
            Formula::IntPow { .. } => (0, vec![2, 0]),
            Formula::RealPower { .. } => (1, vec![0, 0]),
            Formula::Quaternion { .. } => (2, vec![0, 0]),
            Formula::Tricorn { .. } => (3, vec![0, 0]),
            Formula::AmazingBox { .. } => (4, vec![0, 0, 0]),
            Formula::Bulbox { .. } => (5, vec![0; 6]),
            Formula::FoldingIntPow { .. } => (6, vec![2, 0, 0]),
            Formula::AexionC { .. } => (9, vec![0, 0, 2, 2, 0, 0, 2, 2]),
            Formula::SinCosBulb { .. } | Formula::TailsBulb { .. } | Formula::MandelbarBulb { .. }
            | Formula::Lambdabulb { .. } | Formula::Hopfbrot { .. } | Formula::TetrahedralBox { .. } => {
                // New formulas use the existing custom-formula record so
                // their name and options round-trip without claiming an old
                // Mandelbulb3D internal formula number.
                (20, vec![0; opts.len()])
            }
        };
        pi32(&mut d, o + 4, fnr);
        pi32(&mut d, o + 8, opts.len().min(16) as i32);
        let name = f.formula.name();
        let nb = name.as_bytes();
        put(&mut d, o + 12, &nb[..nb.len().min(31)]);
        for (k, t) in types.iter().take(16).enumerate() {
            d[o + 44 + k] = *t;
        }
        for (k, (_, v)) in opts.iter().take(16).enumerate() {
            pf64(&mut d, o + 60 + 8 * k, *v);
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_formulas_round_trip_as_named_formulas() {
        for name in ["SinCos Bulb", "SinCos Bulb V2", "SinCos Bulb V3", "Tails Bulb", "Mandelbar Bulb", "Lambdabulb", "Hopfbrot", "Tetrahedral Box"] {
            let scene = Scene::preset(name).unwrap();
            let back = parse(&write(&scene)).unwrap().scene;
            assert_eq!(back.formulas[0].formula.name(), name);
        }
    }

    #[test]
    fn modern_bulb_transform_options_round_trip() {
        let mut scene = Scene::preset("Tails Bulb").unwrap();
        let f = &mut scene.formulas[0].formula;
        f.set_option("Rotate X", 27.0).unwrap();
        f.set_option("Rotate Y", -13.0).unwrap();
        f.set_option("Fold YZ", 0.75).unwrap();
        let back = parse(&write(&scene)).unwrap().scene;
        let options = back.formulas[0].formula.options();
        for (name, expected) in [("Rotate X (degrees)", 27.0), ("Rotate Y (degrees)", -13.0), ("Fold YZ", 0.75)] {
            let value = options.iter().find(|(k, _)| k == name).unwrap().1;
            assert!((value - expected).abs() < 1e-12, "{name}: {value}");
        }
    }

    #[test]
    fn m3i_round_trip() {
        let mut sc = Scene::preset("Integer Power").unwrap();
        sc.width = 24;
        sc.height = 16;
        let mut g = vec![crate::gbuffer::SiLight::default(); 24 * 16];
        g[5].si_gradient = 1234;
        g[5].normal = [1, -2, 3];
        let d = write_m3i(&sc, &g);
        assert_eq!(d.len(), HEADER_SIZE + 24 * 16 * 18 + ADDON_SIZE);
        assert_eq!(&d[HEADER_SIZE + 5 * 18..HEADER_SIZE + 5 * 18 + 6], &[1, 0, 254, 255, 3, 0]);
        let raw = raw_from_m3i(&d).unwrap();
        let back = parse(&raw).unwrap().scene;
        assert_eq!((back.width, back.height), (24, 16));
        assert_eq!(back.formulas[0].formula.name(), "Integer Power");
        assert_eq!(gbuffer_from_m3i(&d).unwrap().2[5].si_gradient, 1234);
        // a huge image size in the header is no image, not an overflow
        let mut big = d.clone();
        big[4..8].copy_from_slice(&i32::MAX.to_le_bytes());
        big[8..12].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(gbuffer_from_m3i(&big).is_none());
    }

    #[test]
    fn text_parameters_round_trip() {
        let mut raw = vec![0u8; HEADER_SIZE + ADDON_SIZE];
        for (i, b) in raw.iter_mut().enumerate() {
            *b = (i * 7 + i / 3) as u8;
        }
        raw[4..8].copy_from_slice(&640i32.to_le_bytes());
        raw[8..12].copy_from_slice(&480i32.to_le_bytes());
        let a = HEADER_SIZE;
        raw[a + 1] = 0;
        for f in 0..6 {
            let n: i32 = if f < 3 { 5 } else { 0 };
            raw[a + 8 + f * 188..a + 12 + f * 188].copy_from_slice(&n.to_le_bytes());
        }
        let text = raw_to_text(&raw, "Test scene");
        assert!(text.starts_with("Mandelbulb3Dv18{"));
        let (back, title) = raw_from_text(&format!("some forum text\n{text}")).unwrap();
        assert_eq!(title, "Test scene");
        assert_eq!(back[..HEADER_SIZE], raw[..HEADER_SIZE]);
        // the used formula slots survive, iFCount is set
        assert_eq!(back[a + 4], 3);
        assert_eq!(back[a + 8..a + 8 + 3 * 188], raw[a + 8..a + 8 + 3 * 188]);
        // a last formula with negative (uncounted) iterations is used too
        raw[a + 8 + 3 * 188..a + 12 + 3 * 188].copy_from_slice(&(-3i32).to_le_bytes());
        let (back, _) = raw_from_text(&raw_to_text(&raw, "Test scene")).unwrap();
        assert_eq!(back[a + 4], 4);
        assert_eq!(back[a + 8..a + 8 + 4 * 188], raw[a + 8..a + 8 + 4 * 188]);
    }

    #[test]
    fn parse_synthetic_header() {
        let mut d = vec![0u8; HEADER_SIZE + ADDON_SIZE];
        let put_i32 = |d: &mut Vec<u8>, o: usize, v: i32| d[o..o + 4].copy_from_slice(&v.to_le_bytes());
        let put_f64 = |d: &mut Vec<u8>, o: usize, v: f64| d[o..o + 8].copy_from_slice(&v.to_le_bytes());
        put_i32(&mut d, 0, 44);
        put_i32(&mut d, 4, 640);
        put_i32(&mut d, 8, 480);
        put_i32(&mut d, 12, 30);
        d[344] = 1; // bCalc3D
        put_f64(&mut d, 20, -2.0);
        put_f64(&mut d, 28, 20.0);
        put_f64(&mut d, 84, 1.5);
        put_f64(&mut d, 92, 16.0);
        put_f64(&mut d, 108, 30.0);
        d[177..181].copy_from_slice(&1.0f32.to_le_bytes());
        d[182..186].copy_from_slice(&0.5f32.to_le_bytes());
        for i in 0..3 {
            put_f64(&mut d, 246 + (i * 4) * 8, 1.0); // identity matrix
        }
        // light: version 7 in TBoptions bits 20..22, gamma 32
        let tbo: u32 = (7 << 20) | (32 << 23);
        d[432 + 44..432 + 48].copy_from_slice(&tbo.to_le_bytes());
        // addon: version 16, one internal formula (Amazing Box)
        let a = HEADER_SIZE;
        d[a] = 16;
        put_i32(&mut d, a + 8, 3); // iterations
        put_i32(&mut d, a + 12, 4); // fnr = Amazing Box
        put_i32(&mut d, a + 16, 3);
        put_f64(&mut d, a + 8 + 60, -1.5);
        put_f64(&mut d, a + 8 + 68, 0.5);
        put_f64(&mut d, a + 8 + 76, 1.0);
        let m = parse(&d).unwrap();
        let s = &m.scene;
        assert_eq!((s.width, s.height, s.iterations), (640, 480, 30));
        assert_eq!(s.zoom, 1.5);
        assert_eq!(s.formulas.len(), 1);
        assert_eq!(s.formulas[0].iterations, 3);
        assert_eq!(s.formulas[0].formula, Formula::AmazingBox { scale: -1.5, min_r: 0.5, fold: 1.0 });
        assert!(m.warnings.is_empty(), "{:?}", m.warnings);
        // a bad image size or zoom is an error
        for (o, v) in [(4, -1i32), (8, 0), (4, 40000)] {
            let mut b = d.clone();
            put_i32(&mut b, o, v);
            assert!(parse(&b).is_err());
        }
        for z in [0.0, f64::NAN, -1.0] {
            let mut b = d.clone();
            put_f64(&mut b, 84, z);
            assert!(parse(&b).is_err());
        }
    }
}

/// Writes a `.m3i` image file: the parameters of `sc` with the calculated
/// G-buffer (`TsiLight5`, 18 bytes per pixel), which MB3D opens without
/// recalculating ("Save m3i" without the "Img" option).
pub fn write_m3i(sc: &Scene, gbuf: &[crate::gbuffer::SiLight]) -> Vec<u8> {
    let p = write(sc);
    let mut d = Vec::with_capacity(p.len() + gbuf.len() * 18);
    d.extend_from_slice(&p[..HEADER_SIZE]);
    d.extend_from_slice(&crate::gbuffer::pack(gbuf));
    d.extend_from_slice(&p[HEADER_SIZE..]);
    d
}
