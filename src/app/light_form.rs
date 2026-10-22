//! The lighting window (LightAdjust.pas, `TLightAdjustForm`): six lights
//! (global, positional, light map), the colour presets, the object /
//! ambient / fog / background pages.  Changes repaint the kept image
//! without a new calculation, as in MB3D.

use super::util::{argb, fts_single, parse_float, pti, rgb_of};
use super::{Mb3d, MAIN};
use crate::lighting::{Light, Lighting, PaletteColor};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{DialogResult, Ev, Event, MouseButton, Ui};
use std::time::Instant;

const F: &str = "LightAdjustForm";
const PID180: f64 = std::f64::consts::PI / 180.0;

pub struct State {
    pub user_change: bool,
    /// colour start / stop while the fine adjustment is down
    pub col_start: i64,
    pub col_stop: i64,
    /// positional light: the last positions of the relative sliders
    pub old_tb: [i64; 3],
    pub custom: Vec<Option<Lighting>>,
    pub custom_tb: Vec<[i64; 3]>,
    /// a lighting change was painted without what takes long (reflections,
    /// depth of field, hard shadows of moved lights): they follow when the
    /// lighting has not changed for `SETTLE` (a full repaint running, and
    /// whether the main window's line says so)
    pub repaint_pending: bool,
    pub last_change: Instant,
    pub full_running: bool,
    pub status_shown: bool,
    pub undo: Vec<Lighting>,
    pub undo_pos: usize,
    /// the running relight shows an undo/redo step: it stores no undo entry
    pub undo_relight: bool,
    pub hist: Vec<u32>,
    pub hist_ot: Vec<u32>,
    pub hist_int: Vec<u32>,
    pub first_show: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            user_change: true,
            col_start: 0,
            col_stop: 0,
            old_tb: [0; 3],
            custom: vec![None; 10],
            custom_tb: vec![[50, 50, 90]; 10],
            repaint_pending: false,
            last_change: Instant::now(),
            full_running: false,
            status_shown: false,
            undo: Vec::new(),
            undo_pos: 0,
            undo_relight: false,
            hist: Vec::new(),
            hist_ot: Vec::new(),
            hist_int: Vec::new(),
            first_show: true,
        }
    }
}

fn tab(ui: &Ui) -> usize {
    ui.c(F, "TabControl1").tab_index.clamp(0, 5) as usize
}

fn pos(ui: &Ui, tb: &str) -> i64 {
    ui.position(F, tb)
}

fn setp(ui: &mut Ui, tb: &str, v: i64) {
    ui.set_position(F, tb, v);
}

// ---------------------------------------------------------------------------
// presets
// ---------------------------------------------------------------------------

/// A `TLight7` of the built-in presets: Loption, LFunction, colour
/// ($BBGGRR), angles (pi / 16384 units).
struct P7(u8, u8, u32, i32, i32);

struct Preset {
    amb_top: u32,
    amb_bot: u32,
    depth: u32,
    depth2: u32,
    dif: [u32; 4],
    spec: [u32; 4],
    lights: [P7; 2],
    tb578: [i64; 3],
}

const PRESETS: [Preset; 5] = [
    Preset {
        amb_top: 0xB1AA9F,
        amb_bot: 0x3464AA,
        depth: 0xB1AA9F,
        depth2: 0x3464AA,
        dif: [0x2537A0, 0x4E9FD1, 0x62AEB3, 0x71808E],
        spec: [0x0B1026, 0x132734, 0x273639, 0x1C2023],
        lights: [P7(0, 51, 0xFFFFFF, 3363, 4000), P7(0, 3, 0x34628F, -4915, -6500)],
        tb578: [50, 50, 90],
    },
    Preset {
        amb_top: 0xFF6000,
        amb_bot: 0x40B0,
        depth: 0x800000,
        depth2: 0x50,
        dif: [0xE0, 0xFF0000, 0xB800, 0xABDD],
        spec: [0xF0F0F0; 4],
        lights: [P7(0, 3, 0xFFFFFF, 0, 4005), P7(1, 3, 0xFFFFFF, 0, 0)],
        tb578: [110, 22, 220],
    },
    Preset {
        amb_top: 0xE89660,
        amb_bot: 0x304860,
        depth: 0xA04B18,
        depth2: 0xD0A488,
        dif: [0x1D85F8, 0xC0C0C0, 0x47CCF8, 0xB9B69B],
        spec: [0x1D85F8, 0xC0C0C0, 0x47CCF8, 0xB9B69B],
        lights: [P7(0, 2, 0xAADDFF, -637, 4915), P7(0, 2, 2894962, 3368, -6463)],
        tb578: [160, 50, 30],
    },
    Preset {
        amb_top: 0xEC974A,
        amb_bot: 0x5976A6,
        depth: 0xEC974A,
        depth2: 0x5976A6,
        dif: [0x204080, 0x349120, 0x1E36C1, 0xC5AAE6],
        spec: [0, 0x40491C, 0x3B3D60, 0x60487F],
        lights: [P7(0, 51, 0xB9EFFF, 0, 3000), P7(1, 3, 0x34628F, -4915, -6500)],
        tb578: [50, 70, 90],
    },
    Preset {
        amb_top: 0x905838,
        amb_bot: 0x8BA8C7,
        depth: 0x905838,
        depth2: 0x8BA8C7,
        dif: [0xA0A0A0; 4],
        spec: [0x404040; 4],
        lights: [P7(0, 52, 0xB9EFFF, 4063, 6405), P7(1, 52, 0x6797C7, -4915, -7100)],
        tb578: [50, 70, 90],
    },
];

/// Delphi colour ($00BBGGRR) as RGB.
fn bgr(c: u32) -> [u8; 3] {
    [c as u8, (c >> 8) as u8, (c >> 16) as u8]
}

/// A `TLight8` from Loption / LFunction (`defaultLight8` for the rest).
fn light_from(lopt: u8, lfunc: u8, color: [u8; 3], xa: f64, ya: f64) -> Light {
    let on = lopt & 3 == 0;
    Light {
        on,
        color,
        x_angle: xa,
        y_angle: ya,
        amplitude: 1.0,
        spec_func: (lfunc & 7) as i32,
        diff_func: ((lfunc >> 4) & 3) as i32,
        relative_to_object: lopt & 0x20 != 0,
        hs_enabled: lopt & 0x40 == 0,
        positional: lopt & 4 != 0,
        visible: if on { (((lopt >> 2) & 7) | ((lfunc & 128) >> 4)) & 14 } else { 0 },
        ..Light::default()
    }
}

/// The lighting of a built-in preset (1..5, `ConvertColPreset164To20`)
/// and its specular / diffuse / ambient slider positions.
fn builtin_preset(nr: usize) -> (Lighting, [i64; 3]) {
    let p = &PRESETS[nr - 1];
    let mut l = Lighting::default();
    l.amb_top = bgr(p.amb_top);
    l.amb_bottom = bgr(p.amb_bot);
    l.depth_col = bgr(p.depth);
    l.depth_col2 = bgr(p.depth2);
    l.dyn_fog_col = [255, 255, 255];
    for i in 0..4 {
        l.palette[i] = PaletteColor { position: (i * 8191) as u16, diffuse: bgr(p.dif[i]), specular: bgr(p.spec[i]) };
        l.interior[i] = ((i * 8191) as u16, bgr(p.dif[i]));
    }
    for i in 4..10 {
        l.palette[i] = PaletteColor { position: (32200 + (i - 3) * 84) as u16, ..l.palette[0] };
    }
    l.palette_alpha = None;
    for i in 0..6 {
        l.lights[i] = Light::default();
    }
    for (i, q) in p.lights.iter().enumerate() {
        let k = std::f64::consts::PI / 16384.0;
        l.lights[i] = light_from(q.0, q.1, bgr(q.2), q.3 as f64 * k, q.4 as f64 * k);
    }
    (l, p.tb578)
}

/// MB3D's start lighting (`StartPreset`, a TLpreset16).
pub fn start_lighting() -> (Lighting, [i64; 3]) {
    let cols: [u32; 9] = [5873889, 8837614, 0x8B491D, 2988346, 12248958, 0xFFC49F, 11287584, 14579248, 7481121];
    let mut l = Lighting::default();
    l.amb_top = bgr(cols[2]);
    l.amb_bottom = bgr(cols[5]);
    l.depth_col = bgr(0x8B491D);
    l.depth_col2 = bgr(0xFFC49F);
    l.dyn_fog_col = [255, 255, 255];
    for i in 0..3 {
        l.palette[i] = PaletteColor { position: (i * 10922) as u16, diffuse: bgr(cols[i * 3]), specular: bgr(cols[i * 3 + 1]) };
        l.interior[i] = ((i * 10922) as u16, bgr(cols[i * 3]));
    }
    for i in 3..10 {
        l.palette[i] = PaletteColor { position: (32200 + (i - 3) * 84) as u16, ..l.palette[0] };
    }
    l.interior[3] = (32700, l.interior[0].1);
    l.palette_alpha = None;
    let k = std::f64::consts::PI / 16384.0;
    l.lights = Default::default();
    l.lights[0] = light_from(0, 53, bgr(0xA0E8FF), 1500.0 * k, 5200.0 * k);
    l.lights[1] = light_from(1, 16, bgr(2307911), -2822.0 * k, -7737.0 * k);
    (l, [50, 120, 90])
}

/// The custom presets 6..15: `color_preset_N.bin` (a TLpreset20).
fn load_custom_preset(app: &mut Mb3d, i: usize) -> bool {
    let p = crate::appdirs::config_file(&format!("color_preset_{}.bin", i + 1));
    let Ok(d) = std::fs::read(&p) else { return false };
    if d.len() < 348 {
        return false;
    }
    let rd32 = |o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    // a header with the light record at 432 so that the m3p reader decodes it
    let mut h = vec![0u8; 840];
    let b = 432;
    let mut l = Lighting::default();
    l.amb_top = bgr(rd32(0));
    l.amb_bottom = bgr(rd32(4));
    l.depth_col = bgr(rd32(8));
    l.depth_col2 = bgr(rd32(12));
    let tb = [rd32(16) as i64, rd32(20) as i64, rd32(24) as i64];
    h[b + 68..b + 68 + 192].copy_from_slice(&d[32..224]);
    h[b + 260..b + 260 + 124].copy_from_slice(&d[224..348]);
    if let Some(parsed) = crate::m3p::light_from_record(&h[b..]) {
        l.lights = parsed.lights;
        l.palette = parsed.palette;
        l.palette_alpha = parsed.palette_alpha;
        l.interior = parsed.interior;
        l.interior_spec = parsed.interior_spec;
        l.no_col_ipol = parsed.no_col_ipol;
    }
    app.light.custom[i] = Some(l);
    app.light.custom_tb[i] = tb;
    true
}

fn save_custom_preset(app: &Mb3d, i: usize) {
    let Some(l) = &app.light.custom[i] else { return };
    let mut sc = app.scene.clone();
    sc.lighting = l.clone();
    let rec = crate::m3p::write(&sc);
    let b = 432;
    let mut d = vec![0u8; 348];
    let c = |v: [u8; 3]| (v[0] as u32 | (v[1] as u32) << 8 | (v[2] as u32) << 16 | 0xFF00_0000).to_le_bytes();
    d[0..4].copy_from_slice(&c(l.amb_top));
    d[4..8].copy_from_slice(&c(l.amb_bottom));
    d[8..12].copy_from_slice(&c(l.depth_col));
    d[12..16].copy_from_slice(&c(l.depth_col2));
    for (k, v) in app.light.custom_tb[i].iter().enumerate() {
        d[16 + k * 4..20 + k * 4].copy_from_slice(&(*v as i32).to_le_bytes());
    }
    d[28..32].copy_from_slice(&5i32.to_le_bytes());
    d[32..224].copy_from_slice(&rec[b + 68..b + 68 + 192]);
    d[224..348].copy_from_slice(&rec[b + 260..b + 260 + 124]);
    let _ = std::fs::write(crate::appdirs::config_folder().join(format!("color_preset_{}.bin", i + 1)), d);
}

/// `RenderPresetQuat`: the little quaternion of the preset buttons, 34 x 34
/// samples of (depth, light X angle, light Y angle); depth 32768 is the
/// background.  Angles in pi / 16384 units.
#[allow(clippy::approx_constant)] // MB3D's rounded constants
fn preset_quat() -> &'static [(i32, i32, i32)] {
    static Q: std::sync::OnceLock<Vec<(i32, i32, i32)>> = std::sync::OnceLock::new();
    Q.get_or_init(|| {
        // isMemberQuat: smooth iteration count of the quaternion Mandelbrot
        let smooth = |c: [f64; 3]| {
            let (mut x, mut y, mut z, mut w) = (c[0], c[1], c[2], 0.0);
            let mut it = 0;
            let mut r;
            loop {
                let (xt, yt, zt) = (x, y, z);
                x = x * x - y * y - z * z - w * w + c[0];
                y = 2.0 * (y * xt + z * w) + c[1];
                z = 2.0 * (z * xt - yt * w) + c[2];
                w = 2.0 * (w * xt + yt * zt);
                r = x * x + y * y + z * z + w * w;
                it += 1;
                if it >= 10 || r > 8.0 {
                    break;
                }
            }
            if r <= 1.0 {
                it as f64
            } else {
                it as f64 - (0.5 * r.ln()).ln() * 1.4427
            }
        };
        let mut v = Vec::with_capacity(34 * 34);
        for y in 0..34 {
            for x in 1..=34 {
                let mut c = [x as f64 * 0.07 - 1.75, y as f64 * 0.07 - 1.156, -1.5];
                let mut de = 0.055;
                let mut s;
                loop {
                    c[2] += de * 0.3;
                    s = smooth(c);
                    de = (1.0 / (1.0 + (s - 3.0).exp())).abs() + 0.02;
                    if c[2] > 0.5 || de < 0.04 {
                        break;
                    }
                }
                if c[2] > 0.499 {
                    v.push((32768, 0, 0));
                    continue;
                }
                let z = 32767 - ((1.0 - c[2]) * 9000.0).round() as i32;
                let d = s;
                let n3 = d - smooth([c[0], c[1], c[2] - 0.001]);
                let n2 = d - smooth([c[0], c[1] + 0.001, c[2]]);
                let n1 = smooth([c[0] - 0.001, c[1], c[2]]) - d;
                let k = 1.0 / (n1 * n1 + n2 * n2 + n3 * n3 + 0.01).sqrt();
                let a = |t: f64| ((t * k).clamp(-1.0, 1.0).asin() * 5215.1891752352).round() as i32 & 0x7FFF;
                v.push((z, a(n1), a(n2)));
            }
        }
        v
    })
}

/// Preset glyph (`MakeColPresetGlyph`): the little quaternion lit with the
/// preset (`CalcPixel`), 34 x 34 averaged down to 17 x 17 in a 19 x 17
/// glyph whose side columns are the transparent colour.
#[allow(clippy::approx_constant)] // MB3D's rounded constants
fn preset_glyph(l: &Lighting, tb: [i64; 3]) -> Bitmap {
    const SPEC_F: [f64; 8] = [1.0, 1.4142, 2.0, 2.8284, 4.0, 5.6568, 8.0, 11.313];
    let f = |c: [u8; 3]| c.map(|v| v as f64);
    let k16 = 16384.0 / std::f64::consts::PI;
    let q = preset_quat();
    let mut img = vec![[0f64; 3]; 34 * 34];
    for y in 0..34 {
        let dy = y as f64 * 0.03;
        for x in 0..34 {
            let (zpos, lxa, lya) = q[y * 34 + x];
            let (d1, d2) = (f(l.depth_col), f(l.depth_col2));
            let depth = [0, 1, 2].map(|i| d1[i] * (1.0 - dy) + d2[i] * dy);
            let out = &mut img[y * 34 + x];
            *out = depth;
            if zpos >= 32768 {
                continue;
            }
            let mut amb_sh = 0.6 - (3.3 - (32767 - zpos) as f64 * 3.6666e-4);
            let col_index = (x as f64 + 1.0) * 1058.0 - 7800.0;
            // the palette colours at colIndex
            let pal = &l.palette;
            let mut i2 = 5;
            while i2 < 10 && (pal[i2].position as f64) < col_index {
                i2 += 1;
            }
            while i2 > 1 && pal[i2 - 1].position as f64 >= col_index {
                i2 -= 1;
            }
            let i1 = i2 - 1;
            let mut ir = pal[i2.min(9)].position as f64;
            if i2 > 9 {
                i2 = 0;
                ir = 32767.0;
            }
            let (dif, spe) = if l.no_col_ipol {
                (f(pal[i1].diffuse).map(|v| v / 255.0), f(pal[i1].specular).map(|v| v / 255.0))
            } else {
                let t = (col_index - pal[i1].position as f64) / 255.0 / (ir - pal[i1].position as f64).max(1.0);
                let s = 1.0 / 255.0 - t;
                let mix = |a: [u8; 3], b: [u8; 3]| [0, 1, 2].map(|i| a[i] as f64 * s + b[i] as f64 * t);
                (mix(pal[i1].diffuse, pal[i2].diffuse), mix(pal[i1].specular, pal[i2].specular))
            };
            let t3 = ((zpos - 20000) as f64 * 0.0001 + 1.0).max(0.0);
            let dep_c = depth.map(|v| v * (1.0 - t3).max(0.0));
            let t3 = t3 * amb_sh;
            let (amb2, spec_k, diff_k) = (tb[2] as f64 * 0.005, tb[0] as f64 * 0.005, tb[1] as f64 * 0.005);
            let (mut li_dif, mut li_spe) = ([0f64; 3], [0f64; 3]);
            for lt in &l.lights[..4] {
                // Loption = 0: a global light that is on
                if !lt.on || lt.positional || lt.relative_to_object || !lt.hs_enabled {
                    continue;
                }
                let sf = SPEC_F[lt.spec_func as usize & 7];
                let fold = |a: i32| {
                    let a = a & 0x7FFF;
                    if a > 16383 { 32767 - a } else { a }
                };
                let a1 = (sf * fold((lt.x_angle * k16 * 0.5).round() as i32 + lxa) as f64).round() as i32;
                let a3 = (sf * fold((lt.y_angle * k16 * 0.5).round() as i32 + lya) as f64).round() as i32;
                let lc = f(lt.color);
                if (a1 | a3) < 16384 {
                    let g = (-((a1 as f64 / 16384.0).powi(2) + (a3 as f64 / 16384.0).powi(2)) * 9.0).exp() * spec_k;
                    li_spe = [0, 1, 2].map(|i| li_spe[i] + lc[i] * g);
                }
                let ax = ((lt.x_angle * k16).round() as i32 + lxa) as f64 / k16;
                let ay = ((lt.y_angle * k16).round() as i32 + lya) as f64 / k16;
                let d = crate::lighting::get_cos_tab_val(lt.diff_func as usize & 3, (ax.cos() * ay.cos()) as f32, 0.0) as f64 * diff_k;
                li_dif = [0, 1, 2].map(|i| li_dif[i] + lc[i] * d);
            }
            let (a1, a2) = (f(l.amb_top), f(l.amb_bottom));
            amb_sh = amb_sh.min(1.0);
            for i in 0..3 {
                let amb = dif[i] * (li_dif[i] + (a1[i] * (1.0 - dy) + a2[i] * dy) * amb2) + spe[i] * li_spe[i];
                out[i] = amb * ((dif[i] + spe[i] * 0.5) * (1.0 - amb_sh) + amb_sh) * t3 + dep_c[i];
            }
        }
    }
    let key = 0xFFFF_FDFE;
    let mut b = Bitmap::new(19, 17, key);
    for y in 0..17 {
        for x in 0..17 {
            let p = |dx: usize, dy: usize| img[(y * 2 + dy) * 34 + x * 2 + dx];
            let v = |dx: usize, dy: usize, i: usize| p(dx, dy)[i].clamp(0.0, 255.0) as u32;
            let c = [0, 1, 2].map(|i| (v(0, 0, i) + v(1, 0, i) + v(0, 1, i) + v(1, 1, i)) >> 2);
            b.px[y * 19 + x + 1] = 0xFF00_0000 | c[0] << 16 | c[1] << 8 | c[2];
        }
    }
    b
}

/// `SetPreset`: colours (and lights) of a preset.
fn set_preset(app: &mut Mb3d, ui: &mut Ui, nr: usize, keep_lights: bool) {
    let (p, tb) = if nr > 5 {
        match app.light.custom[nr - 6].clone() {
            Some(l) => (l, app.light.custom_tb[nr - 6]),
            None => builtin_preset(5),
        }
    } else {
        builtin_preset(nr)
    };
    let l = &mut app.scene.lighting;
    l.amb_top = p.amb_top;
    l.amb_bottom = p.amb_bottom;
    l.depth_col = p.depth_col;
    l.depth_col2 = p.depth_col2;
    l.dyn_fog_col = p.dyn_fog_col;
    l.dyn_fog_col2 = p.dyn_fog_col;
    if !keep_lights {
        for i in 0..6 {
            if p.lights[i].on {
                let keep_map_rot = l.lights[i].map_rot;
                l.lights[i] = p.lights[i].clone();
                l.lights[i].map_rot = keep_map_rot;
            } else {
                l.lights[i].on = false;
            }
        }
    }
    l.palette = p.palette;
    l.palette_alpha = p.palette_alpha;
    l.interior = p.interior;
    l.interior_spec = p.interior_spec;
    l.no_col_ipol = p.no_col_ipol;
    app.light.user_change = false;
    ui.set_checked(F, "CheckBox22", l.no_col_ipol);
    setp(ui, "TrackBar7", tb[0]);
    setp(ui, "TrackBar5", tb[1]);
    setp(ui, "TrackBar8", tb[2]);
    set_button_colors(app, ui);
    if !keep_lights {
        tab_control1_change(app, ui);
        update_tab_header(app, ui);
    }
    ui.set_item_index(F, "ComboBox3", -1);
    app.light.user_change = true;
}

// ---------------------------------------------------------------------------
// header <-> controls
// ---------------------------------------------------------------------------

fn swatch(c: [u8; 3]) -> Bitmap {
    let (w, h) = (15, 13);
    let mut b = Bitmap::new(w, h, 0xFFFF_00FF);
    for y in 0..13 {
        for x in 1..14 {
            let edge = y == 0 || y == 12 || x == 1 || x == 13;
            b.px[y * w + x] = if edge { 0xFF00_0000 } else { argb(c) };
        }
    }
    b
}

/// `SetSButtonColor`.
fn set_button_color(ui: &mut Ui, nr: i32, c: [u8; 3]) {
    let name = format!("SpeedButton{nr}");
    if ui.f(F).has(&name) {
        let b = ui.cm(F, &name);
        b.glyph = Some(swatch(c));
        b.num_glyphs = 1;
    }
}

/// Palette preview glyphs of the Diff / Spec / Cuts buttons
/// (`PaintSDPreviewColors`, `SetSDButtonColors`).
fn set_sd_button_colors(app: &Mb3d, ui: &mut Ui) {
    let l = &app.scene.lighting;
    let wid = 34usize;
    let (w, h) = (wid + 4, 13);
    let mut dif = Bitmap::new(w, h, 0xFFFF_00FF);
    let mut spe = Bitmap::new(w, h, 0xFFFF_00FF);
    let mut cuts = Bitmap::new(w, h, 0xFFFF_00FF);
    let pos = |i: usize| l.palette[i].position as f32 * wid as f32 / 32767.0;
    let mix3 = |a: [u8; 3], b: [u8; 3], t: f32| argb([0, 1, 2].map(|k| (a[k] as f32 * t + b[k] as f32 * (1.0 - t)).round() as u8));
    let (mut act, mut from) = (1usize, 0f32);
    let mut to = pos(1);
    for x in 0..wid {
        if x as f32 > to && act < 10 {
            act += 1;
            from = to;
            to = if act > 9 { wid as f32 - 1.0 } else { pos(act) };
            if to <= from {
                to = from + 1.0;
            }
        }
        let w1 = 1.0 - (x as f32 - from) / (to - from).max(1.0);
        let (a, b) = (&l.palette[act - 1], &l.palette[act % 10]);
        let (cd, cs) = if l.no_col_ipol { (argb(a.diffuse), argb(a.specular)) } else { (mix3(a.diffuse, b.diffuse, w1), mix3(a.specular, b.specular, w1)) };
        for y in 1..12 {
            dif.px[y * w + x + 2] = cd;
        }
        for y in 1..12 {
            spe.px[y * w + x + 2] = if y < 4 { 0xFF80_8080 } else { cs };
        }
    }
    let ipos = |i: usize| l.interior[i].0 as f32 * wid as f32 / 32767.0;
    let (mut act, mut from) = (1usize, 0f32);
    let mut to = ipos(1);
    for x in 0..wid {
        if x as f32 > to && act < 4 {
            act += 1;
            from = to;
            to = if act > 3 { wid as f32 - 1.0 } else { ipos(act) };
            if to <= from {
                to = from + 1.0;
            }
        }
        let w1 = 1.0 - (x as f32 - from) / (to - from).max(1.0);
        let c = if l.no_col_ipol { argb(l.interior[act - 1].1) } else { mix3(l.interior[act - 1].1, l.interior[act & 3].1, w1) };
        for y in 1..12 {
            cuts.px[y * w + x + 2] = c;
        }
    }
    for (name, b) in [("SpeedButton1", dif), ("SpeedButton2", spe), ("SpeedButton33", cuts)] {
        let c = ui.cm(F, name);
        c.glyph = Some(b);
        c.num_glyphs = 1;
    }
}

fn set_button_colors(app: &Mb3d, ui: &mut Ui) {
    let l = &app.scene.lighting;
    set_button_color(ui, 3, l.amb_top);
    set_button_color(ui, 4, l.dyn_fog_col);
    set_button_color(ui, 6, l.amb_bottom);
    set_button_color(ui, 10, l.depth_col);
    set_button_color(ui, 11, l.depth_col2);
    set_button_color(ui, 30, l.dyn_fog_col2);
    set_sd_button_colors(app, ui);
}

/// `SetLightFromHeader`.
pub fn set_from_header(app: &mut Mb3d, ui: &mut Ui) {
    app.light.user_change = false;
    let l = app.scene.lighting.clone();
    let fine = l.fine_col_adj.is_some();
    ui.cm(F, "SBFineAdj").down = fine;
    app.light.col_start = l.color_start as i64;
    app.light.col_stop = l.color_end as i64;
    setp(ui, "TrackBar14", l.var_col_z as i64);
    setp(ui, "TrackBar3", l.fog_offset as i64);
    setp(ui, "TrackBar19", 128);
    setp(ui, "TrackBar20", l.bg_rot[0] as i64);
    setp(ui, "TrackBar21", l.bg_rot[1] as i64);
    setp(ui, "TrackBar22", l.bg_rot[2] as i64);
    setp(ui, "TrackBar23", l.ind_light as i64);
    setp(ui, "TrackBar24", l.roughness as i64);
    setp(ui, "TrackBar11", l.amb_shadow as i64);
    setp(ui, "TrackBar32", (l.diffuse_shadowing * 256.0).round() as i64);
    let dm = l.diff_map as i64;
    ui.set_position(F, "UpDownDiffMap", dm.max(1));
    ui.set_text(F, "Edit21", &dm.max(1).to_string());
    ui.set_checked(F, "CheckBox15", dm != 0);
    if dm != 0 {
        setp(ui, "TrackBar30", l.diff_map_rot as i64);
        setp(ui, "TrackBar29", l.diff_map_offset[0] as i64);
        setp(ui, "TrackBar28", l.diff_map_offset[1] as i64);
        setp(ui, "TrackBar31", l.diff_map_scale as i64);
    } else {
        for t in ["TrackBar30", "TrackBar29", "TrackBar28"] {
            setp(ui, t, 128);
        }
        setp(ui, "TrackBar31", 30);
    }
    setp(ui, "TrackBar4", l.depth_fog as i64);
    setp(ui, "TrackBar5", l.diffuse as i64);
    setp(ui, "TrackBar6", l.dyn_fog as i64);
    setp(ui, "TrackBar9", l.color_start as i64);
    setp(ui, "TrackBar10", l.color_end as i64);
    setp(ui, "TrackBar7", l.specular as i64);
    setp(ui, "TrackBar8", l.ambient as i64);
    if let Some((a, b)) = l.fine_col_adj {
        setp(ui, "TrackBar9", a as i64 - 30);
        setp(ui, "TrackBar10", b as i64 - 30);
    }
    setp(ui, "TrackBar12", l.interior_start as i64);
    setp(ui, "TrackBar13", l.interior_end as i64);
    ui.set_checked(F, "CheckBox1", l.color_cycling);
    ui.set_checked(F, "CheckBox2", l.color_on_otrap);
    ui.set_checked(F, "CheckBox3", l.far_fog);
    ui.set_checked(F, "CheckBox12", !l.bg_direct);
    setp(ui, "TrackBar18", l.gamma as i64);
    ui.set_checked(F, "CheckBox9", l.amb_rel_obj);
    ui.set_checked(F, "CheckBox10", l.internal_gamma2);
    ui.set_item_index(F, "RadioGroup1", (l.diff_map_mode & 3) as i32);
    ui.set_checked(F, "CheckBox16", l.ex_mode & 1 != 0);
    ui.set_checked(F, "CheckBox17", l.bg_add_light);
    ui.set_checked(F, "CheckBox18", l.yc_comb);
    ui.set_checked(F, "CheckBox19", l.dfog_options & 1 != 0);
    ui.set_checked(F, "CheckBox23", l.dfog_options & 2 != 0);
    ui.set_checked(F, "CheckBox21", l.bg_ambient);
    ui.set_checked(F, "CheckBox22", l.no_col_ipol);
    setp(ui, "TrackBar33", l.bg_brightness as i64);
    ui.set_caption(F, "Label45", &fts_single(1.04f64.powi(l.bg_brightness as i32 - 40)));
    ui.set_checked(F, "CheckBox8", !l.bg_image.is_empty());
    let d = match l.depth_func {
        1 => "SpeedButton5",
        2 => "SpeedButton8",
        _ => "SpeedButton7",
    };
    ui.set_checked(F, d, true);
    ui.cm(F, "TabControl1").tab_index = 0;
    tab_control1_change(app, ui);
    update_tab_header(app, ui);
    set_button_colors(app, ui);
    check_box15(app, ui);
    set_bg_preview(app, ui);
    app.light.user_change = true;
}

/// `PutLightFInHeader`: the sliders into the scene's lighting.
pub fn put_light_in_header(app: &mut Mb3d, ui: &mut Ui) {
    let st = (app.light.col_start, app.light.col_stop);
    let l = &mut app.scene.lighting;
    l.fog_offset = pos(ui, "TrackBar3") as f32;
    l.depth_fog = pos(ui, "TrackBar4") as f32;
    l.diffuse = pos(ui, "TrackBar5") as f32;
    l.dyn_fog = pos(ui, "TrackBar6") as f32;
    l.specular = pos(ui, "TrackBar7") as f32;
    l.ambient = pos(ui, "TrackBar8") as f32;
    l.color_start = pos(ui, "TrackBar9") as f32;
    l.color_end = pos(ui, "TrackBar10") as f32;
    l.amb_shadow = pos(ui, "TrackBar11") as f32;
    l.ind_light = pos(ui, "TrackBar23") as f32;
    if ui.checked(F, "SBFineAdj") {
        l.fine_col_adj = Some(((l.color_start + 30.0) as u8, (l.color_end + 30.0) as u8));
        l.color_start = st.0 as f32;
        l.color_end = st.1 as f32;
    } else {
        l.fine_col_adj = None;
    }
    l.bg_rot = [pos(ui, "TrackBar20") as u8, pos(ui, "TrackBar21") as u8, pos(ui, "TrackBar22") as u8];
    l.var_col_z = pos(ui, "TrackBar14") as f32;
    l.roughness = pos(ui, "TrackBar24") as f32;
    if ui.visible(F, "Panel2") {
        l.diff_map = ui.position(F, "UpDownDiffMap").clamp(0, 65535) as u16;
        l.diff_map_offset = [pos(ui, "TrackBar29") as u8, pos(ui, "TrackBar28") as u8];
        l.diff_map_rot = pos(ui, "TrackBar30") as u8;
    } else {
        l.diff_map = 0;
    }
    l.internal_gamma2 = ui.checked(F, "CheckBox10");
    l.bg_add_light = ui.checked(F, "CheckBox17");
    l.yc_comb = ui.checked(F, "CheckBox18");
    l.bg_ambient = ui.checked(F, "CheckBox21");
    l.diff_map_mode = ui.item_index(F, "RadioGroup1").clamp(0, 3) as u8;
    l.depth_func = if ui.checked(F, "SpeedButton5") { 1 } else if ui.checked(F, "SpeedButton8") { 2 } else { 0 };
    l.dfog_options = ui.checked(F, "CheckBox19") as u8 | (ui.checked(F, "CheckBox23") as u8) << 1;
    l.ex_mode = ui.checked(F, "CheckBox16") as u8;
    l.no_col_ipol = ui.checked(F, "CheckBox22");
    l.diff_map_scale = pos(ui, "TrackBar31") as u8;
    l.diffuse_shadowing = pos(ui, "TrackBar32") as f32 / 256.0;
    l.bg_brightness = pos(ui, "TrackBar33") as u8;
    l.interior_start = pos(ui, "TrackBar12") as f32;
    l.interior_end = pos(ui, "TrackBar13") as f32;
    l.color_cycling = ui.checked(F, "CheckBox1");
    l.bg_direct = !ui.checked(F, "CheckBox12");
    l.color_on_otrap = ui.checked(F, "CheckBox2");
    l.far_fog = ui.checked(F, "CheckBox3");
    l.amb_rel_obj = ui.checked(F, "CheckBox9");
    l.gamma = pos(ui, "TrackBar18") as f32;
}

/// How long the lighting stays unchanged before what takes long is added.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(1500);

/// `TriggerRepaint`: repaints the kept image with the new lighting at once,
/// as MB3D does: the hard shadows as calculated, without reflections and
/// depth of field. Those (and the shadows of moved lights) follow when the
/// lighting has settled ([`idle`]); a change meanwhile stops them.
pub fn trigger_repaint(app: &mut Mb3d, ui: &mut Ui) {
    if app.main.calculating {
        return;
    }
    app.light.undo_relight = false;
    app.make_scene(ui);
    if let Some(b) = app.eng.base() {
        app.eng.start(super::engine::Job::Relight { scene: app.scene.clone() });
        let st = &mut app.light;
        st.last_change = Instant::now();
        st.full_running = false;
        st.repaint_pending = super::engine::relight_incomplete(&app.scene, &b);
        if st.repaint_pending {
            // (short: the line has little room beside the Frame field)
            let shadows = super::engine::post_sig(&app.scene) != b.post_sig;
            ui.set_caption(MAIN, "Label6", if shadows { "shadows next" } else { "reflections next" });
            st.status_shown = true;
        }
    }
    ui.cm(F, "ComboBox3").font = Some(crate::vcl::font::Font { color: ui.theme.text_disabled(), custom_color: true, ..Default::default() });
}

/// The lighting settled after a change painted without reflections, depth
/// of field or moved shadows: the full repaint adds them.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    // (a calculation or a post processing started meanwhile makes its own
    // complete picture)
    if app.main.calculating {
        app.light.repaint_pending = false;
        app.light.status_shown = false;
        return;
    }
    let st = &app.light;
    if !st.repaint_pending || st.last_change.elapsed() < SETTLE || app.eng.running() {
        return;
    }
    app.light.repaint_pending = false;
    app.light.full_running = true;
    let shadows = app.eng.base().is_some_and(|b| super::engine::post_sig(&app.scene) != b.post_sig);
    ui.set_caption(MAIN, "Label6", if shadows { "redoing shadows" } else { "adding reflections" });
    app.light.status_shown = true;
    app.eng.start(super::engine::Job::Repaint { scene: app.scene.clone() });
}

fn store_undo_light(app: &mut Mb3d) {
    let l = app.scene.lighting.clone();
    let st = &mut app.light;
    // (as CompareMem in MB3D: an unchanged lighting is no new step)
    if st.undo_pos > 0 && st.undo.get(st.undo_pos - 1).is_some_and(|u| format!("{u:?}") == format!("{l:?}")) {
        return;
    }
    st.undo.truncate(st.undo_pos);
    st.undo.push(l);
    if st.undo.len() > 40 {
        st.undo.remove(0);
    }
    st.undo_pos = st.undo.len();
}

/// The engine finished: the colour histogram of the new image.
pub fn calc_finished(app: &mut Mb3d, ui: &mut Ui) {
    // (the line about the following passes, once they are done)
    if app.light.status_shown && !app.light.repaint_pending && !app.main.calculating {
        app.light.full_running = false;
        app.light.status_shown = false;
        ui.set_caption(MAIN, "Label6", "");
    }
    if let Some(b) = app.eng.base() {
        let mut h = vec![0u32; 32768];
        let mut ho = vec![0u32; 32768];
        let iw = ui.c(F, "Image2").width.max(1) as usize;
        let mut hi = vec![0u32; iw];
        for s in b.post.iter() {
            if s.is_background() {
                continue;
            }
            if s.si_gradient < 32768 {
                h[s.si_gradient as usize] += 1;
            } else {
                let v = ((((s.si_gradient - 32768) as f64) * 0.0000305175).sqrt().sqrt() * (iw - 1) as f64).round() as usize;
                hi[v.min(iw - 1)] += 1;
            }
            ho[(s.otrap & 0x7FFF) as usize] += 1;
        }
        app.light.hist = h;
        app.light.hist_ot = ho;
        app.light.hist_int = hi;
        repaint_col_histo(app, ui);
    }
    if !std::mem::take(&mut app.light.undo_relight) {
        store_undo_light(app);
    }
    let n = app.light.undo.len();
    ui.set_enabled(F, "SpeedButton9", n > 1);
}

/// `RepaintColHisto`: the histogram of the colour values over the colour
/// sliders (Image1) and of the interior (Image2).
fn repaint_col_histo(app: &Mb3d, ui: &mut Ui) {
    let st = &app.light;
    if st.hist.is_empty() {
        return;
    }
    let (w, h) = (ui.c(F, "Image1").width.max(2) as usize, ui.c(F, "Image1").height.max(1) as usize);
    let fine = ui.checked(F, "SBFineAdj");
    let (dmin, dmul) = if fine {
        let a = ((st.col_start as f64 + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0;
        (a, (((st.col_stop as f64 + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0 - a) / 32767.0)
    } else {
        (0.0, 1.0)
    };
    let hist = if ui.checked(F, "CheckBox2") { &st.hist_ot } else { &st.hist };
    let maxn = hist.iter().copied().max().unwrap_or(1).max(1) as f64;
    let mut b = Bitmap::new(w, h, 0xFFFF_FFFF);
    let mut x2: i64 = if fine { (-16384.0 * dmul + dmin).round() as i64 } else { 0 };
    let col = |d: f64| {
        let c = 255 - (d.clamp(0.0, 1.0).sqrt().sqrt() * 255.0).round() as u32;
        0xFF00_0000 | c << 16 | c << 8 | (((c & 0xFE) + 200) >> 1).min(255)
    };
    for x in 0..w {
        let x3 = if fine {
            ((x as f64 * 65535.0 / (w - 1) as f64 - 16384.0) * dmul + dmin).round() as i64
        } else {
            ((x as f64 * 4.0 / (3.0 * (w - 1) as f64)).powi(2) * 32767.0 - 10900.0).round() as i64
        };
        let inc = (x3 - x2).signum();
        let (mut a, mut d) = (0, 0.0);
        loop {
            if (0..32768).contains(&x2) {
                d += hist[x2 as usize] as f64;
                a += 1;
            }
            if x2 == x3 || inc == 0 {
                break;
            }
            x2 += inc;
        }
        if a > 0 {
            d /= a as f64 * maxn;
        }
        let c = col(d);
        for y in 0..h {
            b.px[y * w + x] = c;
        }
    }
    ui.set_picture(F, "Image1", Some(b));
    let (w2, h2) = (ui.c(F, "Image2").width.max(1) as usize, ui.c(F, "Image2").height.max(1) as usize);
    if st.hist_int.len() >= w2 {
        let m = st.hist_int.iter().skip(1).copied().max().unwrap_or(1).max(1) as f64;
        let mut b2 = Bitmap::new(w2, h2, 0xFFFF_FFFF);
        for x in 0..w2 {
            let c = col(st.hist_int[x] as f64 / m);
            for y in 0..h2 {
                b2.px[y * w2 + x] = c;
            }
        }
        ui.set_picture(F, "Image2", Some(b2));
    }
}

// ---------------------------------------------------------------------------
// the lights
// ---------------------------------------------------------------------------

fn vis_to_index(v: u8) -> i32 {
    [0, 3, 1, 2][((v >> 1) & 3) as usize] | if v & 8 != 0 { 4 } else { 0 }
}

fn index_to_vis(i: i32) -> u8 {
    ([0u8, 2, 3, 1][(i & 3) as usize] << 1) | if i & 4 != 0 { 8 } else { 0 }
}

fn light_page(l: &Light) -> usize {
    if l.map > 0 {
        2
    } else if l.positional {
        1
    } else {
        0
    }
}

fn update_tab_header(app: &Mb3d, ui: &mut Ui) {
    let tabs: Vec<String> = (0..6).map(|i| if app.scene.lighting.lights[i].on { format!("Li.{} \u{00BB}", i + 1) } else { format!("Li.{}", i + 1) }).collect();
    ui.cm(F, "TabControl1").tabs = tabs;
}

/// `FindVecAngles`: the light angles (-LX, LY) whose light vector (rotated
/// back with `m` when `abs`, i.e. relative to the object) is `sv`; None when
/// the search ends farther than 1e-3 off.
fn find_vec_angles(sv: &crate::math::Vec3, m: &crate::math::Mat3, abs: bool) -> Option<(f64, f64)> {
    use std::f64::consts::PI;
    let vec = |d1: f64, d2: f64| {
        let v = crate::math::scale(crate::math::build_view_vector_dfov(d2, d1), -1.0);
        crate::math::normalize(if abs { crate::math::rotate_vector_reverse(&v, m) } else { v })
    };
    let err = |v: crate::math::Vec3| (sv[0] - v[0]).abs() + (sv[1] - v[1]).abs() + (sv[2] - v[2]).abs();
    let (mut dmin, mut b1, mut b2) = (1000.0, 0.0, 0.0);
    for y in -5..=5 {
        for x in -5..=5 {
            let (d1, d2) = (x as f64 * PI * 0.2, y as f64 * PI * 0.2);
            let dt = err(vec(d1, d2));
            if dt < dmin {
                (dmin, b1, b2) = (dt, d1, d2);
            }
        }
    }
    let mut i = 4000;
    while i >= 0 && dmin >= 1e-7 {
        // (a probe step gives the gradient, then the step from it)
        let (mut d1, mut d2) = (b1, b2);
        if i & 1 == 1 { d1 = b1 + dmin * 0.001 } else { d2 = b2 + dmin * 0.001 }
        let dt = err(vec(d1, d2));
        // (NonZero)
        let q = dmin - dt;
        let q = if q.abs() < 1e-40 { 1e-3 } else if q.abs() < 1e-3 { q.signum() * 1e-3 } else { q };
        let dt = dmin * dmin * 0.001 / q;
        if i & 1 == 1 { d1 = b1 + dt } else { d2 = b2 + dt }
        let dt = err(vec(d1, d2));
        if dt < dmin {
            (dmin, b1, b2) = (dt, d1, d2);
        }
        i -= 1;
    }
    let wrap = |mut d: f64| {
        while d < -PI {
            d += PI * 2.0;
        }
        while d > PI {
            d -= PI * 2.0;
        }
        d
    };
    (dmin < 1e-3).then(|| (wrap(b1), wrap(b2)))
}

/// `TabControl1Change`: shows light `TabIndex`.
fn tab_control1_change(app: &mut Mb3d, ui: &mut Ui) {
    let b = app.light.user_change;
    app.light.user_change = false;
    let i = tab(ui);
    let li = app.scene.lighting.lights[i].clone();
    ui.set_checked(F, "CheckBox4", li.on);
    ui.set_checked(F, "CheckBox7", li.hs_enabled);
    let page = ["TabSheet1", "TabSheet2", "TabSheet6"][light_page(&li)];
    ui.set_active_page(F, "PageControl1", page);
    set_button_color(ui, 12, li.color);
    ui.set_item_index(F, "ComboBox2", li.spec_func.clamp(0, 7));
    ui.set_item_index(F, "ComboBox1", li.diff_func.clamp(0, 3));
    ui.set_item_index(F, "ComboBox4", vis_to_index(li.visible));
    ui.set_item_index(F, "ComboBox5", vis_to_index(li.visible));
    page_control1_show(app, ui);
    app.light.user_change = b;
}

/// The controls of the light type page (`PageControl1Change` without a
/// user change).
fn page_control1_show(app: &mut Mb3d, ui: &mut Ui) {
    let i = tab(ui);
    let li = app.scene.lighting.lights[i].clone();
    let page = light_page(&li);
    ui.set_visible(F, "SpeedButton12", page < 2);
    ui.set_enabled(F, "ComboBox1", page < 2);
    ui.set_enabled(F, "ComboBox2", page < 2);
    match page {
        0 => {
            setp(ui, "TrackBar2", (li.y_angle / PID180).round() as i64);
            setp(ui, "TrackBar1", (-li.x_angle / PID180).round() as i64);
            ui.set_checked(F, "CheckBox6", li.relative_to_object);
        }
        1 => {
            for t in ["TrackBar15", "TrackBar16", "TrackBar17"] {
                setp(ui, t, 0);
            }
            app.light.old_tb = [0; 3];
        }
        _ => {
            setp(ui, "TrackBar26", li.map_rot[0] as i64);
            setp(ui, "TrackBar25", li.map_rot[1] as i64);
            setp(ui, "TrackBar27", li.map_rot[2] as i64);
            ui.set_position(F, "UpDownLight", li.map.max(1) as i64);
            ui.set_text(F, "Edit2", &li.map.max(1).to_string());
            ui.set_checked(F, "CheckBox14", li.relative_to_object);
            light_map_preview(ui, li.map as i32);
        }
    }
    ui.set_text(F, "Edit1", &short_float_str(li.amplitude));
}

/// `ShortFloatToStr`: "m.m" + "e" + exponent as MB3D shows the intensity.
pub fn short_float_str(v: f32) -> String {
    let w = crate::m3p::to_short_float_pub(v);
    let m = (w & 0xFF) as u8 as i8 as f64;
    let e = (w >> 8) as u8 as i8;
    if m == 0.0 {
        return "0.0".into();
    }
    // FormatFloat('#.#', m * 0.1)
    let mi = m as i32;
    let (a, b) = (mi.abs() / 10, mi.abs() % 10);
    let mut s = if mi < 0 { "-".to_string() } else { String::new() };
    if a > 0 {
        s += &a.to_string();
    }
    if b != 0 {
        s += &format!(".{b}");
    }
    format!("{s}e{e}")
}

fn light_map_preview(ui: &mut Ui, nr: i32) {
    let map = crate::maps::by_number(nr);
    ui.set_visible(F, "Label39", map.is_none());
    let img = map.map(|m| map_preview(&m, ui.c(F, "Image3").width.max(8) as usize, ui.c(F, "Image3").height.max(8) as usize));
    ui.set_picture(F, "Image3", img);
}

fn map_preview(m: &crate::maps::LightMap, w: usize, h: usize) -> Bitmap {
    let mut b = Bitmap::new(w, h, 0xFF00_0000);
    for y in 0..h {
        for x in 0..w {
            let c = m.pixel(x as f32 / w as f32, y as f32 / h as f32, 1);
            let g = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u32;
            b.px[y * w + x] = 0xFF00_0000 | g(c[0]) << 16 | g(c[1]) << 8 | g(c[2]);
        }
    }
    b
}

fn set_bg_preview(app: &Mb3d, ui: &mut Ui) {
    let name = &app.scene.lighting.bg_image;
    let img = if name.is_empty() { None } else { crate::maps::by_name(name) };
    ui.set_visible(F, "Image5", img.is_some());
    let p = img.map(|m| map_preview(&m, ui.c(F, "Image5").width.max(8) as usize, ui.c(F, "Image5").height.max(8) as usize));
    ui.set_picture(F, "Image5", p);
}

fn check_box15(app: &Mb3d, ui: &mut Ui) {
    let on = ui.checked(F, "CheckBox15");
    ui.set_visible(F, "Panel1", !on);
    ui.set_visible(F, "Panel2", on);
    if on {
        let nr = ui.position(F, "UpDownDiffMap") as i32;
        let map = crate::maps::by_number(nr);
        ui.set_visible(F, "Label40", map.is_none());
        let p = map.map(|m| map_preview(&m, ui.c(F, "Image4").width.max(8) as usize, ui.c(F, "Image4").height.max(8) as usize));
        ui.set_picture(F, "Image4", p);
    }
    let _ = app;
}

/// The camera position (`CalcCamPos`).
fn cam_pos(sc: &crate::scene::Scene) -> [f64; 3] {
    let vz = crate::math::normalize(sc.vgrads[2]);
    let d = sc.z_start - sc.mid[2];
    [sc.mid[0] + vz[0] * d, sc.mid[1] + vz[1] * d, sc.mid[2] + vz[2] * d]
}

/// `TrackBar16Change`: moves a positional light relative to the view.
fn move_pos_light(app: &mut Mb3d, ui: &mut Ui) {
    let i = tab(ui);
    let sc = &app.scene;
    let m = crate::math::normalise_matrix_to(1.0, &sc.vgrads);
    let cp = cam_pos(sc);
    let lp = sc.lighting.lights[i].position;
    let lv = crate::math::sub(lp, cp);
    let d = if sc.stereo_mode == 2 { 180.0 } else { sc.fov_y };
    let mut dm = crate::math::len(&lv);
    let size = ((sc.width * sc.width + sc.height * sc.height) as f64).sqrt();
    dm = (sc.step_width() + dm * (d * PID180 / sc.height as f64).sin()) * size * 0.0025;
    let (p15, p16, p17) = (pos(ui, "TrackBar15"), pos(ui, "TrackBar16"), pos(ui, "TrackBar17"));
    let [o15, o16, o17] = app.light.old_tb;
    let dv = [(p16 - o16) as f64 * dm, (p15 - o15) as f64 * -dm];
    let mut np = lp;
    for k in 0..3 {
        np[k] += dv[0] * m[0][k] + dv[1] * m[1][k];
    }
    if p17 != o17 {
        let n = crate::math::normalize(lv);
        for k in 0..3 {
            np[k] += n[k] * (p17 - o17) as f64 * dm;
        }
    }
    app.light.old_tb = [p15, p16, p17];
    app.scene.lighting.lights[i].position = np;
}

/// Light position from a point picked in the main image (`iGetPosFromImage = 2`).
pub fn set_pos_light_from_image(app: &mut Mb3d, ui: &mut Ui, p: [f64; 3]) {
    let i = tab(ui);
    ui.set_caption(F, "ButtonGetPos", "mid");
    app.scene.lighting.lights[i].position = p;
    trigger_repaint(app, ui);
}

// ---------------------------------------------------------------------------
// events
// ---------------------------------------------------------------------------

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    let uc = app.light.user_change;
    let i = tab(ui);
    match h {
        "FormShow" => {
            super::main_form::place_sticky(app, ui, Some(F), true);
            if app.light.first_show {
                app.light.first_show = false;
                for k in 1..=5 {
                    let (l, tb) = builtin_preset(k);
                    let c = ui.cm(F, &format!("SpeedButton{}", k + 14));
                    c.glyph = Some(preset_glyph(&l, tb));
                    c.num_glyphs = 1;
                }
                for k in 0..10 {
                    if load_custom_preset(app, k) {
                        let l = app.light.custom[k].clone().unwrap();
                        let tb = app.light.custom_tb[k];
                        let c = ui.cm(F, &format!("SpeedButton{}", k + 20));
                        c.glyph = Some(preset_glyph(&l, tb));
                        c.num_glyphs = 1;
                    }
                }
                update_quick_load(app, ui);
                make_depth_col_list(ui);
                set_button_colors(app, ui);
            }
        }
        "FormCreate" => {
            // MB3D starts with its start preset (`SetStartPreset`): the
            // preset's colours and lights, everything else as the window's
            // controls start (`PutLightFInHeader`)
            put_light_in_header(app, ui);
            let (p, tb) = start_lighting();
            let l = &mut app.scene.lighting;
            (l.amb_top, l.amb_bottom, l.depth_col, l.depth_col2) = (p.amb_top, p.amb_bottom, p.depth_col, p.depth_col2);
            (l.lights, l.palette, l.interior, l.palette_alpha) = (p.lights, p.palette, p.interior, p.palette_alpha);
            // `UpdateLightParasAbove3`: the interior colours' specular
            // amount is their brightness
            for (s, (_, c)) in l.interior_spec.iter_mut().zip(l.interior) {
                *s = (c[0] as f32 * 0.3 + c[1] as f32 * 0.59 + c[2] as f32 * 0.11).round() as u8;
            }
            set_from_header(app, ui);
            setp(ui, "TrackBar7", tb[0]);
            setp(ui, "TrackBar5", tb[1]);
            setp(ui, "TrackBar8", tb[2]);
        }
        "FormHide" => {
            let f = ui.f(F);
            let s = format!("{} {}", f.left, f.top);
            app.ini.set("LightPos", &s);
        }
        "TrackBar2Change" | "TrackBar33Change" | "TrackBar22Change" => {
            if h == "TrackBar33Change" {
                let p = pos(ui, "TrackBar33");
                ui.set_caption(F, "Label45", &fts_single(1.04f64.powi(p as i32 - 40)));
            }
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "TrackBar1Change" => {
            if uc {
                app.scene.lighting.lights[i].x_angle = pos(ui, "TrackBar1") as f64 * -PID180;
                trigger_repaint(app, ui);
            }
        }
        "TrackBarYangleChange" => {
            if uc {
                app.scene.lighting.lights[i].y_angle = pos(ui, "TrackBar2") as f64 * PID180;
                trigger_repaint(app, ui);
            }
        }
        "TrackBar16Change" => {
            if uc {
                move_pos_light(app, ui);
                trigger_repaint(app, ui);
            }
        }
        "TrackBar16MouseUp" => {
            app.light.user_change = false;
            setp(ui, &e.sender, 0);
            let k = match e.sender.as_str() {
                "TrackBar15" => 0,
                "TrackBar16" => 1,
                _ => 2,
            };
            app.light.old_tb[k] = 0;
            app.light.user_change = true;
        }
        "TrackBar26Change" => {
            if uc {
                let l = &mut app.scene.lighting.lights[i];
                l.map_rot = [pos(ui, "TrackBar26") as u8, pos(ui, "TrackBar25") as u8, pos(ui, "TrackBar27") as u8];
                trigger_repaint(app, ui);
            }
        }
        "TrackBar21KeyPress" | "TrackBar11KeyPress" => {
            let key = if h == "TrackBar21KeyPress" { '0' } else { '1' };
            if e.ev == Ev::KeyPress(key) {
                let s = ui.c(F, &e.sender).sel_start;
                setp(ui, &e.sender, s);
                trigger_repaint(app, ui);
            }
        }
        "UpDown1Click" => {
            if let Ev::UpDown { up } = e.ev {
                let t = ui.tag(F, &e.sender) as usize;
                let sc = &app.scene;
                let m = crate::math::normalise_matrix_to(1.0, &sc.vgrads);
                let cp = cam_pos(sc);
                let lv = sc.lighting.lights[i].position;
                let dv = crate::math::sub(lv, cp);
                let d = crate::math::dot(&dv, &m[2]).abs();
                let mut d = (sc.step_width() + d * (sc.fov_y * PID180 / sc.height as f64).sin()) * 0.5;
                if !up {
                    d = -d;
                }
                if t == 1 {
                    d = -d;
                }
                let mut np = lv;
                for k in 0..3 {
                    np[k] += m[t.min(2)][k] * d;
                }
                app.scene.lighting.lights[i].position = np;
                trigger_repaint(app, ui);
            }
        }
        "UpDown4Click" => {
            if let Ev::UpDown { up } = e.ev {
                let d = parse_float(&ui.text(F, "Edit1")).unwrap_or(1.0) * if up { 1.414 } else { 0.707 };
                app.scene.lighting.lights[i].amplitude = crate::m3p::short_float_round(d as f32);
                ui.set_text(F, "Edit1", &short_float_str(app.scene.lighting.lights[i].amplitude));
                trigger_repaint(app, ui);
            }
        }
        "Edit1Change" => {
            if uc {
                if let Some(d) = parse_float(&ui.text(F, "Edit1")) {
                    app.scene.lighting.lights[i].amplitude = crate::m3p::short_float_round(d as f32);
                    trigger_repaint(app, ui);
                }
            }
        }
        "TabControl1Change" => tab_control1_change(app, ui),
        "TabControl1MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, x, y, .. } = e.ev {
                for k in 0..6 {
                    let n = ["CopythislighttoLight11", "CopythislighttoLight21", "CopythislighttoLight31", "CopythislighttoLight41", "CopythislighttoLight51", "CopythislighttoLight61"][k];
                    ui.set_enabled(F, n, k != i);
                }
                ui.popup_menu(F, "PopupMenu1", "TabControl1", x, y);
            }
        }
        "CopythislighttoLight11Click" => {
            let d = ui.tag(F, &e.sender).clamp(0, 5) as usize;
            if d != i {
                let src = app.scene.lighting.lights[i].clone();
                let repaint = src.on || app.scene.lighting.lights[d].on;
                app.scene.lighting.lights[d] = src;
                update_tab_header(app, ui);
                if repaint {
                    trigger_repaint(app, ui);
                }
            }
        }
        "CheckBox4Click" => {
            if uc {
                app.scene.lighting.lights[i].on = ui.checked(F, "CheckBox4");
                if app.scene.lighting.lights[i].on && app.scene.lighting.lights[i].amplitude == 0.0 {
                    app.scene.lighting.lights[i].amplitude = 1.0;
                }
                trigger_repaint(app, ui);
            }
            update_tab_header(app, ui);
        }
        "CheckBox7Click" => {
            if uc {
                app.scene.lighting.lights[i].hs_enabled = ui.checked(F, "CheckBox7");
                if ui.checked(F, "CheckBox4") {
                    trigger_repaint(app, ui);
                }
            }
        }
        "CheckBox6Click" => {
            if uc {
                let on = ui.checked(F, &e.sender);
                let l = &mut app.scene.lighting.lights[i];
                if e.sender == "CheckBox6" && !l.positional {
                    // keep the light direction when switching between
                    // viewer and object relative angles
                    let mm = crate::math::normalise_matrix_to(1.0, &app.scene.vgrads);
                    let mut v = crate::math::scale(crate::math::build_view_vector_dfov(l.y_angle, -l.x_angle), -1.0);
                    if !on {
                        v = crate::math::normalize(crate::math::rotate_vector_reverse(&v, &mm));
                    }
                    if let Some((d1, d2)) = find_vec_angles(&v, &mm, on) {
                        l.x_angle = -d1;
                        l.y_angle = d2;
                        let b = app.light.user_change;
                        app.light.user_change = false;
                        setp(ui, "TrackBar1", (d1 / PID180).round() as i64);
                        setp(ui, "TrackBar2", (d2 / PID180).round() as i64);
                        app.light.user_change = b;
                    }
                }
                l.relative_to_object = on;
                trigger_repaint(app, ui);
            }
        }
        "ComboBox1Change" => {
            if uc {
                let l = &mut app.scene.lighting.lights[i];
                l.spec_func = ui.item_index(F, "ComboBox2").max(0);
                l.diff_func = ui.item_index(F, "ComboBox1").max(0);
                trigger_repaint(app, ui);
            }
        }
        "ComboBox4Change" => {
            if uc {
                app.light.user_change = false;
                let page0 = light_page(&app.scene.lighting.lights[i]) == 0;
                let idx = if page0 { ui.item_index(F, "ComboBox4") } else { ui.item_index(F, "ComboBox5") };
                ui.set_item_index(F, "ComboBox4", idx);
                ui.set_item_index(F, "ComboBox5", idx);
                app.scene.lighting.lights[i].visible = index_to_vis(idx.max(0));
                app.light.user_change = true;
                trigger_repaint(app, ui);
            }
        }
        "PageControl1Change" => {
            let page = match ui.active_page(F, "PageControl1").as_str() {
                "TabSheet2" => 1,
                "TabSheet6" => 2,
                _ => 0,
            };
            {
                let zoom = app.scene.zoom;
                let l = &mut app.scene.lighting.lights[i];
                match page {
                    0 => {
                        l.map = 0;
                        l.positional = false;
                        l.x_angle = 0.0;
                        l.y_angle = 0.0;
                        l.amplitude = 1.0;
                    }
                    1 => {
                        l.map = 0;
                        l.positional = true;
                        l.amplitude = crate::m3p::short_float_round((1.0 / zoom) as f32);
                        if l.position == [0.0; 3] {
                            l.position = app.scene.mid;
                        }
                    }
                    _ => {
                        l.positional = false;
                        l.map_rot = [128; 3];
                        l.map = 1;
                        l.amplitude = 1.0;
                    }
                }
            }
            app.light.user_change = false;
            page_control1_show(app, ui);
            app.light.user_change = true;
            trigger_repaint(app, ui);
        }
        "PageControl1Changing" | "TabControl1Changing" => {}
        "SpinEdit1Change" => {
            let nr = pti(&ui.text(F, "Edit2")).max(1);
            ui.set_position(F, "UpDownLight", nr as i64);
            if uc {
                app.scene.lighting.lights[i].map = nr as u16;
            }
            light_map_preview(ui, nr);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "SpinEdit2Change" => {
            let nr = pti(&ui.text(F, "Edit21")).max(1);
            ui.set_position(F, "UpDownDiffMap", nr as i64);
            if uc {
                app.scene.lighting.diff_map = nr as u16;
            }
            check_box15(app, ui);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "Edit2MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, .. } = e.ev {
                let o = crate::vcl::dialogs::FileOptions {
                    title: "Choose a map".into(),
                    filter: "Images|*.png;*.jpg;*.jpeg;*.bmp|All files (*.*)|*.*".into(),
                    initial_dir: Some(app.ini.dir(super::ini::DIR_MAPS)),
                    ..Default::default()
                };
                ui.open_dialog(&format!("light:map:{}", e.sender), &o);
            }
        }
        "ButtonGetPosClick" => {
            if ui.caption(F, "ButtonGetPos") == "image" {
                ui.set_caption(F, "ButtonGetPos", "mid");
                app.main.get_pos = 0;
                ui.cm(super::MAIN, "Image1").cursor = "crDefault".into();
            } else {
                ui.set_caption(F, "ButtonGetPos", "image");
                app.main.get_pos = 2;
                ui.cm(super::MAIN, "Image1").cursor = "crCross".into();
            }
        }
        "SpeedButton1Click" => {
            let t = ui.tag(F, &e.sender) as i32;
            let l = &app.scene.lighting;
            let c = match t {
                3 => l.amb_top,
                4 => l.dyn_fog_col,
                6 => l.amb_bottom,
                10 => l.depth_col,
                11 => l.depth_col2,
                12 => l.lights[i].color,
                30 => l.dyn_fog_col2,
                _ => return,
            };
            ui.pick_color(&format!("light:color:{t}"), argb(c));
        }
        "SpeedButton3MouseUp" => {
            if let Ev::MouseUp { button: MouseButton::Right, .. } = e.ev {
                let l = &mut app.scene.lighting;
                l.amb_top = l.depth_col;
                l.amb_bottom = l.depth_col2;
                set_button_colors(app, ui);
                trigger_repaint(app, ui);
            }
        }
        "SpeedButton4MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, x, y, .. } = e.ev {
                if ui.visible(super::MAIN, "UpDown5") {
                    ui.popup_menu(F, "PopupMenu3", &e.sender, x, y);
                }
            }
        }
        "Insertvolumetriclightcolor1Click" => {
            let n = (pti(&ui.text(super::MAIN, "Edit16")) - 1).clamp(0, 5) as usize;
            let c = app.scene.lighting.lights[n].color;
            app.scene.lighting.dyn_fog_col2 = c;
            app.scene.lighting.dyn_fog_col = c;
            set_button_colors(app, ui);
            trigger_repaint(app, ui);
        }
        "N01Click" => {
            let t = ui.tag(F, &e.sender) as usize;
            if (1..=15).contains(&t) {
                let l = if t > 5 { app.light.custom[t - 6].clone().unwrap_or_else(|| builtin_preset(5).0) } else { builtin_preset(t).0 };
                app.scene.lighting.depth_col = l.depth_col;
                app.scene.lighting.depth_col2 = l.depth_col2;
                set_button_colors(app, ui);
                trigger_repaint(app, ui);
            }
        }
        "SpeedButton2Click" => {
            // the palette editor (ColorPick.pas)
            let cuts = ui.tag(F, &e.sender) == 33;
            if ui.has_form("ColorForm") {
                ui.set_item_index("ColorForm", "RadioGroup1", cuts as i32);
                ui.show("ColorForm");
            }
        }
        "SpeedButton15Click" => {
            let t = ui.tag(F, &e.sender) as usize;
            if ui.c(F, &e.sender).cursor == "crUpArrow" && t > 5 {
                // store the current colours as custom preset
                ui.cm(F, "SpeedButtonMem").down = false;
                for k in 20..30 {
                    ui.cm(F, &format!("SpeedButton{k}")).cursor = "crDefault".into();
                }
                put_light_in_header(app, ui);
                app.light.custom[t - 6] = Some(app.scene.lighting.clone());
                app.light.custom_tb[t - 6] = [pos(ui, "TrackBar7"), pos(ui, "TrackBar5"), pos(ui, "TrackBar8")];
                save_custom_preset(app, t - 6);
                let l = app.scene.lighting.clone();
                let tb = app.light.custom_tb[t - 6];
                let c = ui.cm(F, &e.sender);
                c.glyph = Some(preset_glyph(&l, tb));
                c.num_glyphs = 1;
            } else {
                let keep = ui.checked(F, "CheckBox11");
                set_preset(app, ui, t, keep);
                trigger_repaint(app, ui);
            }
        }
        "SpeedButtonMemClick" => {
            let arrow = ui.c(F, "SpeedButton20").cursor == "crUpArrow";
            for k in 20..30 {
                ui.cm(F, &format!("SpeedButton{k}")).cursor = if arrow { "crDefault".into() } else { "crUpArrow".into() };
            }
            if arrow {
                ui.cm(F, "SpeedButtonMem").down = false;
            }
        }
        "SpeedButton31Click" => {
            let tg = 1 - ui.tag(F, "SpeedButton31");
            ui.cm(F, "SpeedButton31").tag = tg;
            let b = tg == 0;
            for k in 20..30 {
                ui.set_visible(F, &format!("SpeedButton{k}"), b);
            }
            ui.set_visible(F, "SpeedButton31", b);
            ui.set_visible(F, "SpeedButton32", !b);
            ui.set_visible(F, "CheckBox11", b);
            let h18 = ui.c(F, "SpeedButton18").height;
            let h13 = ui.c(F, "SpeedButton13").height;
            let (cw, ch) = ui.f(F).client_size();
            if b {
                ui.cm(F, "Panel3").height = 3 * h18 + h13 + 11;
                ui.set_client_size(F, cw, ch + 2 * h18);
            } else {
                ui.cm(F, "Panel3").height = h18 + h13 + 11;
                ui.set_client_size(F, cw, ch - 2 * h18);
            }
            let p3 = ui.c(F, "Panel3").height;
            ui.cm(F, "SpeedButton13").top = p3 - h13 - 5;
            ui.cm(F, "SpeedButton14").top = p3 - h13 - 5;
            ui.cm(F, "ComboBox3").top = p3 - h13 - 1;
        }
        "SBFineAdjClick" => {
            if ui.checked(F, "SBFineAdj") {
                app.light.col_start = pos(ui, "TrackBar9");
                app.light.col_stop = pos(ui, "TrackBar10");
                app.light.user_change = false;
                setp(ui, "TrackBar9", 0);
                setp(ui, "TrackBar10", 60);
            } else {
                app.light.user_change = false;
                setp(ui, "TrackBar9", app.light.col_start);
                setp(ui, "TrackBar10", app.light.col_stop);
            }
            app.light.user_change = true;
            repaint_col_histo(app, ui);
            trigger_repaint(app, ui);
        }
        "SpeedButton34Click" => fit_colors_to_histogram(app, ui),
        "CheckBox1Click" => {
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "CheckBox2Click" => {
            repaint_col_histo(app, ui);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "CheckBox22Click" => {
            app.scene.lighting.no_col_ipol = ui.checked(F, "CheckBox22");
            set_sd_button_colors(app, ui);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "CheckBox21Click" => {
            if ui.checked(F, "CheckBox21") && !ui.checked(F, "CheckBox8") {
                ui.set_checked(F, "CheckBox21", false);
            }
            let on = ui.checked(F, "CheckBox21");
            ui.set_enabled(F, "SpeedButton3", !on);
            ui.set_enabled(F, "SpeedButton6", !on);
            ui.set_enabled(F, "TrackBar8", !on);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "CheckBox15Click" => {
            check_box15(app, ui);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "CheckBox16Click" => {
            let m = ui.item_index(F, "RadioGroup1");
            let (a, b, c) = match m {
                1 => ("Rotation X:", "Rotation Y:", "Rotation Z:"),
                0 => ("Offset X:", "Offset Y:", "Rotation:"),
                _ => ("Offset X:", "Offset Y:", ""),
            };
            ui.set_caption(F, "Label35", a);
            ui.set_caption(F, "Label36", b);
            ui.set_caption(F, "Label41", c);
            ui.set_visible(F, "TrackBar30", m < 2);
            ui.set_visible(F, "Label38", m != 1);
            ui.set_visible(F, "TrackBar31", m != 1);
            if uc {
                trigger_repaint(app, ui);
            }
        }
        "CheckBox8Click" => {
            if uc {
                if ui.checked(F, "CheckBox8") {
                    let o = crate::vcl::dialogs::FileOptions {
                        title: "Background image".into(),
                        filter: "Images|*.png;*.jpg;*.jpeg;*.bmp|All files (*.*)|*.*".into(),
                        initial_dir: Some(app.ini.dir(super::ini::DIR_BGPIC)),
                        ..Default::default()
                    };
                    ui.open_dialog("light:bgpic", &o);
                } else {
                    app.scene.lighting.bg_image.clear();
                    ui.set_checked(F, "CheckBox21", false);
                    set_bg_preview(app, ui);
                    trigger_repaint(app, ui);
                }
            }
        }
        "FogResetButtonClick" => {
            setp(ui, "TrackBar3", 128);
            setp(ui, "TrackBar6", 53);
            setp(ui, "TrackBar19", 128);
            trigger_repaint(app, ui);
        }
        "SpeedButton9MouseUp" => {
            if let Ev::MouseUp { button, .. } = e.ev {
                let n = app.light.undo.len();
                match button {
                    MouseButton::Left if app.light.undo_pos > 1 => app.light.undo_pos -= 1,
                    MouseButton::Right if app.light.undo_pos < n => app.light.undo_pos += 1,
                    _ => return,
                }
                if let Some(l) = app.light.undo.get(app.light.undo_pos - 1).cloned() {
                    app.scene.lighting = l;
                    set_from_header(app, ui);
                    ui.set_item_index(F, "ComboBox3", -1);
                    trigger_repaint(app, ui);
                    app.light.undo_relight = app.eng.running();
                }
            }
        }
        "ComboBox3DropDown" => {
            ui.cm(F, "ComboBox3").font = None;
        }
        "ComboBox3Select" => {
            let name = ui.c(F, "ComboBox3").item_text();
            let p = app.ini.dir(super::ini::DIR_LIGHTS).join(format!("{name}.m3l"));
            load_light_file(app, ui, &p);
        }
        "Button1Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Light parameter (*.m3l)|*.m3l".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_LIGHTS)),
                ..Default::default()
            };
            ui.open_dialog("light:load", &o);
        }
        "Button2Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Light parameter (*.m3l)|*.m3l".into(),
                default_ext: "m3l".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_LIGHTS)),
                ..Default::default()
            };
            ui.save_dialog("light:save", &o);
        }
        "FormMouseWheel" => {
            if let Ev::Wheel { delta, .. } = e.ev {
                let focus = ui.focused(F).unwrap_or_default();
                let up = delta > 0;
                match focus.as_str() {
                    "Edit2" | "Edit21" => {
                        let v = (pti(&ui.text(F, &focus)) + if up { 1 } else { -1 }).max(1);
                        ui.set_text(F, &focus, &v.to_string());
                        let h = if focus == "Edit2" { "SpinEdit1Change" } else { "SpinEdit2Change" };
                        ui.post(F, &focus, h, Ev::Change);
                    }
                    "Edit1" => ui.post(F, "UpDown4", "UpDown4Click", Ev::UpDown { up }),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// `SpeedButton34Click`: colour start / end to the histogram.
fn fit_colors_to_histogram(app: &mut Mb3d, ui: &mut Ui) {
    let st = &app.light;
    let hist = if ui.checked(F, "CheckBox2") { &st.hist_ot } else { &st.hist };
    if hist.is_empty() {
        return;
    }
    let total: u64 = hist.iter().map(|&v| v as u64).sum();
    let cnt = (total / 2000).max(1);
    let mut n = 0u64;
    let imin = hist.iter().position(|&v| {
        n += v as u64;
        v > 0 && n >= cnt
    });
    let Some(imin) = imin else { return };
    n = 0;
    let imax = 32767 - hist.iter().rev().position(|&v| {
        n += v as u64;
        v > 0 && n >= cnt
    }).unwrap_or(0);
    let (a, b) = if ui.checked(F, "SBFineAdj") {
        let dmin = ((st.col_start as f64 + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0;
        let dmul = ((((st.col_stop as f64 + 30.0) / 90.0).powi(2) * 32767.0 - 10900.0 - dmin) / 32767.0).max(1e-4);
        let f = |i: usize| (((i as f64 - dmin) / dmul + 16384.0) * 120.0 / 65535.0).clamp(0.0, 120.0).round() as i64 - 30;
        (f(imin), f(imax))
    } else {
        let f = |i: usize| ((((i as f64 + 10900.0) / 32767.0).sqrt() * 90.0).clamp(0.0, 120.0)).round() as i64 - 30;
        (f(imin), f(imax))
    };
    let (mut a, mut b) = (a, b);
    if a == b {
        if a > 0 {
            a -= 1;
        } else {
            b += 1;
        }
    }
    setp(ui, "TrackBar9", a);
    setp(ui, "TrackBar10", b);
    trigger_repaint(app, ui);
}

fn update_quick_load(app: &Mb3d, ui: &mut Ui) {
    let dir = app.ini.dir(super::ini::DIR_LIGHTS);
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let p = e.path();
                    (p.extension().map(|x| x.eq_ignore_ascii_case("m3l")) == Some(true)).then(|| p.file_stem().unwrap().to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|s| s.to_lowercase());
    ui.set_items(F, "ComboBox3", v);
}

/// The depth colour menu (`MakeDepthColList`): one item per preset.
fn make_depth_col_list(ui: &mut Ui) {
    let f = ui.fm(F);
    let Some(menu) = f.id("PopupMenu2") else { return };
    if f.ctl[menu].children.len() > 1 {
        return;
    }
    if let Some(&first) = f.ctl[menu].children.first() {
        f.ctl[first].visible = false;
    }
    for i in 1..=15 {
        let mut c = crate::vcl::control::Control::new(&format!("DepthColItem{i}"), "TMenuItem");
        c.caption = i.to_string();
        c.tag = i;
        c.events.insert("OnClick".into(), "N01Click".into());
        f.add_control(menu, c);
    }
}

/// `GetLightParaFile`: a .m3l file (TLightingParas9).
fn load_light_file(app: &mut Mb3d, ui: &mut Ui, p: &std::path::Path) {
    let Ok(d) = std::fs::read(p) else {
        app.message(ui, &format!("{}: cannot read", p.display()));
        return;
    };
    match crate::m3p::light_from_record(&d) {
        Some(mut l) => {
            if ui.checked(F, "CheckBox11") {
                l.lights = app.scene.lighting.lights.clone();
            }
            app.scene.lighting = l;
            set_from_header(app, ui);
            trigger_repaint(app, ui);
            ui.cm(F, "ComboBox3").font = None;
        }
        None => app.message(ui, &format!("{}: no light parameters", p.display())),
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        (w, DialogResult::Color(Some(c))) if w.starts_with("color:") => {
            let t: i32 = w[6..].parse().unwrap_or(0);
            let c = rgb_of(*c);
            let i = tab(ui);
            let l = &mut app.scene.lighting;
            match t {
                3 => l.amb_top = c,
                4 => l.dyn_fog_col = c,
                6 => l.amb_bottom = c,
                10 => l.depth_col = c,
                11 => l.depth_col2 = c,
                12 => l.lights[i].color = c,
                30 => l.dyn_fog_col2 = c,
                _ => {}
            }
            set_button_colors(app, ui);
            set_button_color(ui, 12, app.scene.lighting.lights[i].color);
            trigger_repaint(app, ui);
        }
        ("load", DialogResult::File(Some(p))) => {
            if let Some(d) = p.parent() {
                app.ini.dirs[super::ini::DIR_LIGHTS] = d.to_path_buf();
            }
            load_light_file(app, ui, p);
            update_quick_load(app, ui);
        }
        ("save", DialogResult::File(Some(p))) => {
            put_light_in_header(app, ui);
            let rec = crate::m3p::write(&app.scene);
            let p = p.with_extension("m3l");
            match std::fs::write(&p, &rec[432..840]) {
                Ok(()) => app.message(ui, &format!("Saved {}", p.display())),
                Err(e) => app.message(ui, &format!("{}: {e}", p.display())),
            }
            update_quick_load(app, ui);
        }
        ("bgpic", DialogResult::File(r)) => {
            match r {
                Some(p) => {
                    if let Some(d) = p.parent() {
                        app.ini.dirs[super::ini::DIR_BGPIC] = d.to_path_buf();
                        crate::maps::add_map_dir(d.to_path_buf());
                    }
                    app.scene.lighting.bg_image = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    if crate::maps::by_name(&app.scene.lighting.bg_image).is_none() {
                        app.message(ui, "The background image could not be loaded.");
                        app.scene.lighting.bg_image.clear();
                        app.light.user_change = false;
                        ui.set_checked(F, "CheckBox8", false);
                        app.light.user_change = true;
                    }
                }
                None => {
                    app.light.user_change = false;
                    ui.set_checked(F, "CheckBox8", !app.scene.lighting.bg_image.is_empty());
                    app.light.user_change = true;
                }
            }
            set_bg_preview(app, ui);
            trigger_repaint(app, ui);
        }
        (w, DialogResult::File(Some(p))) if w.starts_with("map:") => {
            let edit = &w[4..];
            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let num: String = stem.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
            if !num.is_empty() {
                if let Some(d) = p.parent() {
                    crate::maps::add_map_dir(d.to_path_buf());
                }
                ui.set_text(F, edit, &num.trim_start_matches('0').to_string());
                let h = if edit == "Edit2" { "SpinEdit1Change" } else { "SpinEdit2Change" };
                ui.post(F, edit, h, Ev::Change);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_vec_angles_keeps_the_light_vector() {
        // a rotated view; light at -LX = 0.7, LY = -0.4, viewer relative
        let m = crate::math::normalise_matrix_to(1.0, &[[0.8, 0.0, -0.6], [0.0, 1.0, 0.0], [0.6, 0.0, 0.8]]);
        let v = crate::math::scale(crate::math::build_view_vector_dfov(-0.4, 0.7), -1.0);
        let (d1, d2) = find_vec_angles(&v, &m, true).unwrap();
        let w = crate::math::scale(crate::math::build_view_vector_dfov(d2, d1), -1.0);
        let w = crate::math::normalize(crate::math::rotate_vector_reverse(&w, &m));
        assert!((0..3).all(|k| (v[k] - w[k]).abs() < 1e-3), "{v:?} {w:?}");
    }
}
