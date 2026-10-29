//! Render set-up and the distance-estimation ray marcher.
//!
//! * `CalcParams::new` – `GetMCTparasFromHeader` (HeaderTrafos.pas)
//! * `Marcher::calc_de` – `CalcDEanalytic` / `CalcDEnoADE` (Calc.pas)
//! * `Marcher::calculate_normals*` – `RMCalculateNormals[OnSmoothIt]`
//! * `Marcher::bin_search*` – `RMdoBinSearch`, `RMdoBinSearchIt`
//! * `Marcher::march_pixel` – the per pixel loop of `TMandCalcThread.Execute`
//!   (CalcThread.pas)
//!
//! Not yet ported (later phases): DE combinations, interpolation hybrids,
//! dIFS, inside rendering, cutting planes, volumetric light, stereo.

use crate::formulas::FirstOptionType;
use crate::gbuffer::SiLight;
use crate::iteration::{HybridMode, HybridSlot, Iteration};
use crate::math::*;
use crate::scene::{InsideMode, CameraOptic, Scene};

/// Immutable per-render values (`TMCTparameter`).
#[derive(Clone, Debug)]
pub struct CalcParams {
    pub slots: Vec<HybridSlot>,
    pub mode: HybridMode,
    /// normalised weights of an interpolation hybrid
    pub ipol: [f32; 2],
    /// `ColorOnIt`
    pub color_on_it: u8,
    /// 2D calculation (`iSliceCalc` 1..3), 0 = 3D
    pub slice_2d: u8,
    pub is_custom_de: bool,
    /// dIFS hybrid (`doHybridIFS3D`)
    pub difs: bool,
    pub de_option: i32,
    pub d_de_scale: f32,
    pub sm_normals: i32,
    pub de_stop: f32,
    pub z_step_div: f32,
    pub de_add_steps: i32,
    pub width: i32,
    pub height: i32,
    /// The calculated part of the image (`CalcRect`): left, top, width,
    /// height; the G-buffer covers only this rectangle.
    pub rect: [i32; 4],
    pub fov_y: f64,
    pub max_it: i32,
    pub min_it: i32,
    pub color_option: u8,
    pub mct_color_mul: f32,
    pub d_rstop: f64,
    pub rstop3d: f64,
    pub do_julia: bool,
    pub ju: [f64; 4],
    pub vgrads: Mat3,
    pub pl_optic_z: f32,
    pub optic: CameraOptic,
    pub dfog_on_it: u16,
    pub fovx_off: f32,
    pub fovx_mul: f32,
    pub d_col_plus: f32,
    pub zcorr: f64,
    pub zc_mul: f64,
    pub zend: f64,
    pub step_width: f64,
    pub col_var_de_stop_mul: f32,
    pub first_step_random: bool,
    pub mh04zsd: f32,
    pub de_stop_factor: f32,
    pub ln_rstop: f32,
    pub normals_on_de: bool,
    pub ystart: Vec3,
    pub de_offset: f32,
    pub de_offset006: f32,
    pub mcts_m: f32,
    pub ms_de_sub: f32,
    pub fhln: [f32; 6],
    pub smatrix4: Mat4,
    pub end_to: usize,
    pub repeat_from: usize,
    pub zz_stmit_dif: f64,
    /// Prototype interpreter with the custom formulas loaded (if any).
    pub machine: Option<crate::x86::Machine>,
    /// Cutting planes (`iCutOptions`): bits 0..2 = x, y, z plane on,
    /// bits 4..6 = the side that is cut away (from the view direction)
    pub cut_options: u8,
    /// inside rendering (`bInsideRendering`) / inside and outside (`bInAndOutside`)
    pub inside_rendering: bool,
    pub in_and_outside: bool,
    /// `dCOX`, `dCOY`, `dCOZ`
    pub cut_pos: Vec3,
    /// DE combination (second hybrid part), if any
    pub decomb: Option<DeComb>,
    /// Volumetric light map (`DFogOnIt = 65535`), built before the main pass.
    pub vol: Option<std::sync::Arc<crate::vollight::VolLightMap>>,
    /// The post calculations the graphics card ran with the main
    /// calculation (`render::post_process` skips them).
    pub gpu_post: Option<PostJob>,
    /// How long those took when the card ran them after the march (None:
    /// within the march, no separate time).
    pub gpu_post_secs: Option<f64>,
    /// The graphics card calculates in double precision although the zoom
    /// would allow single: a probe found single precision too coarse for
    /// the scene's fine detail (see `render::double_probe`).
    pub gpu_double: bool,
    /// The iteration statistics of the calculation (shared by the clones
    /// of these parameters).
    pub its: std::sync::Arc<ItStats>,
}

/// MB3D's iteration statistics of a calculation (`i64Its`, `iItAvrCount`,
/// `MaxIts` of the calculation threads): the iterations at the surface
/// point of every pixel that hits it, after the binary search.
#[derive(Debug, Default)]
pub struct ItStats {
    sum: std::sync::atomic::AtomicU64,
    count: std::sync::atomic::AtomicU64,
    max: std::sync::atomic::AtomicI32,
}

impl ItStats {
    pub fn add(&self, sum: u64, count: u64, max: i32) {
        use std::sync::atomic::Ordering::Relaxed;
        if count > 0 {
            self.sum.fetch_add(sum, Relaxed);
            self.count.fetch_add(count, Relaxed);
            self.max.fetch_max(max, Relaxed);
        }
    }

    pub fn reset(&self) {
        use std::sync::atomic::Ordering::Relaxed;
        self.sum.store(0, Relaxed);
        self.count.store(0, Relaxed);
        self.max.store(0, Relaxed);
    }

    /// (average iterations times 10, rounded; the most iterations), as
    /// MB3D's `CalcStatistic`: (0, 0) when no pixel hit the surface.
    pub fn get(&self) -> (i32, i32) {
        use std::sync::atomic::Ordering::Relaxed;
        let (s, n) = (self.sum.load(Relaxed), self.count.load(Relaxed));
        if n == 0 {
            return (0, 0);
        }
        (((s as f64 * 10.0 / n as f64).round()) as i32, self.max.load(Relaxed))
    }
}

/// The post calculations on the G-buffer that follow the main calculation:
/// hard (or one soft) shadow and DE ambient occlusion
/// (`render::post_job`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PostJob {
    pub shadows: Option<ShadowJob>,
    pub deao: Option<crate::deao::DeaoParams>,
}

impl PostJob {
    pub fn is_empty(&self) -> bool {
        self.shadows.is_none() && self.deao.is_none()
    }
}

/// The hard shadow pass: the selected lights, soft (one light) with its
/// radius, and the ray length multiplier.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowJob {
    pub lights: Vec<HsLight>,
    /// > 0: one soft shadow of this radius (`lights` has one entry)
    pub soft_radius: f32,
    pub max_len_mul: f32,
}

/// One part of a hybrid (the whole hybrid, or one of the two parts of a DE
/// combination): slot range and how its distance is estimated.
#[derive(Clone, Debug)]
pub struct HybridPart {
    pub start: usize,
    pub end: usize,
    pub repeat: usize,
    pub de_option: i32,
    pub is_custom_de: bool,
    pub difs: bool,
    pub mode: HybridMode,
    pub d_de_scale: f32,
    pub max_it: i32,
}

/// DE combination of two hybrid parts (`FormulaType` 1..6 in MB3D).
#[derive(Clone, Debug)]
pub struct DeComb {
    /// 1 min, 2 max, 3 max with the 2nd part inverted, 4 smooth min
    /// (linear), 5 smooth min, 6 mix (dIFS on the result of part 1)
    pub kind: u8,
    pub part2: HybridPart,
    /// `sDEcombSmooth` in steps
    pub smooth: f64,
    pub fmix_pow: f64,
    pub mix_col: u8,
}

/// `CheckDEoption(0, 1)` of an interpolation hybrid: the formulas with a
/// positive weight (`iItCount > 0` on the bits of the single).
fn ipol_de_option(slots: &[HybridSlot], raw: [f32; 2]) -> i32 {
    let mut de_option = -1;
    for (x, s) in slots.iter().enumerate().take(2) {
        if raw[x].to_bits() as i32 <= 0 {
            continue;
        }
        let de = s.formula.de_option();
        if !(0..=20).contains(&de) || de == de_option {
            continue;
        }
        if ((0..20).contains(&de_option) && de > 19) || (de_option > 19 && (0..20).contains(&de)) {
            return -1;
        }
        de_option = match de_option {
            -1 => de,
            2 if ![2, 5, 6, 11].contains(&de) => 0,
            4 if ![5, 6].contains(&de) => 0,
            5 => {
                if de == 4 {
                    4
                } else if de == 2 || de == 11 {
                    2
                } else if de != 6 {
                    0
                } else {
                    5
                }
            }
            6 => {
                if [2, 4, 5, 11].contains(&de) {
                    de
                } else {
                    0
                }
            }
            11 => {
                if de == 2 || de == 5 {
                    2
                } else if de != 6 {
                    0
                } else {
                    11
                }
            }
            d => d,
        };
    }
    if de_option > 19 {
        de_option = 20;
    }
    de_option
}

/// `CheckFormulaOptions` – combine the DE options of all used formulas.
fn combine_de_options(slots: &[HybridSlot], end_to: usize) -> i32 {
    combine_de_options_range(slots, 0, end_to)
}

fn combine_de_options_range(slots: &[HybridSlot], start: usize, end_to: usize) -> i32 {
    let mut de_option = -1;
    for s in slots.iter().take(end_to + 1).skip(start) {
        if s.iterations == 0 {
            continue;
        }
        let de = s.formula.de_option();
        if de < 0 || de == de_option {
            continue;
        }
        de_option = match de_option {
            -1 | 21 | 22 => de,
            2 => {
                if [2, 5, 6, 11].contains(&de) {
                    2
                } else {
                    0
                }
            }
            4 => {
                if [5, 6].contains(&de) {
                    4
                } else {
                    0
                }
            }
            5 => {
                if de == 4 {
                    4
                } else if de == 2 || de == 11 {
                    2
                } else if de != 6 {
                    0
                } else {
                    5
                }
            }
            6 => {
                if [2, 4, 5, 11].contains(&de) {
                    de
                } else {
                    0
                }
            }
            11 => {
                if de == 2 || de == 5 {
                    2
                } else if de != 6 {
                    0
                } else {
                    11
                }
            }
            other => other,
        };
    }
    de_option
}

impl CalcParams {
    pub fn new(sc: &Scene) -> Result<CalcParams, String> {
        let width = sc.width;
        let height = sc.height;
        // interpolation hybrid: the two first formulas with their weights
        // (normalised to a sum of 1 like in GetMCTparasFromHeader)
        let ipol: Option<[f32; 2]> = match sc.interpolation {
            Some(w) => {
                if sc.formulas.len() < 2 {
                    return Err("an interpolation hybrid needs two formulas".into());
                }
                let (mut x1, mut x2) = (w[0], w[1]);
                let mut y1 = x1 + x2;
                if y1 < 1e-10 {
                    y1 = 1.0;
                    x1 = 1.0;
                    x2 = 0.0;
                } else {
                    y1 = 1.0 / y1;
                }
                Some([x1 * y1, x2 * y1])
            }
            None => None,
        };
        if ipol.is_none() && sc.formulas.iter().all(|f| f.iterations == 0) {
            return Err("no formula with iterations > 0".into());
        }
        // hybrid slots (empty slots get 0 iterations)
        let mut slots: Vec<HybridSlot> = sc
            .formulas
            .iter()
            .take(if ipol.is_some() { 2 } else { 6 })
            .map(|f| HybridSlot {
                formula: f.formula.clone(),
                iterations: if ipol.is_some() { 1 } else { f.iterations.abs() },
                uncounted: ipol.is_none() && f.iterations < 0,
                ade: false,
            })
            .collect();
        let dc_scene = if ipol.is_some() { None } else { sc.decomb };
        // CheckHybridOptions: end = last slot with iterations
        let mut last = slots.len() - 1;
        while last > 0 && slots[last].iterations == 0 {
            last -= 1;
        }
        slots.truncate(last + 1);
        let dc = dc_scene.filter(|d| d.end1 < last && d.start2 <= last);
        let end_to = match &dc {
            Some(d) => d.end1.min(last),
            None => last,
        };
        let mut repeat_from = sc.repeat_from.min(end_to);
        while repeat_from > 0 && slots[repeat_from].iterations <= 0 {
            repeat_from -= 1;
        }

        // interpreter for .m3f formulas
        let is_x86 = |f: &crate::formulas::Formula| matches!(f, crate::formulas::Formula::Custom(c) if c.def.jit.is_none());
        let machine = if slots.iter().any(|s| is_x86(&s.formula)) {
            let cfs: Vec<Option<&crate::custom::CustomFormula>> = slots
                .iter()
                .map(|s| match &s.formula {
                    crate::formulas::Formula::Custom(c) if c.def.jit.is_none() => Some(&**c),
                    _ => None,
                })
                .collect();
            Some(crate::custom::build_machine(&cfs))
        } else {
            None
        };
        let mut de_option = combine_de_options(&slots, end_to);
        if sc.disable_analytic_de {
            de_option = match de_option {
                2 | 11 => 0,
                5 | 6 => 4,
                d => d,
            };
        }
        // dIFS (DE option 20 shapes, 21 transforms) cannot be mixed with other formulas
        let used = || slots.iter().take(end_to + 1).filter(|s| s.iterations != 0).map(|s| s.formula.de_option());
        let has_difs = used().any(|d| d >= 20) && dc.is_none_or(|d| d.kind != 6 || used().any(|x| x == 20));
        if has_difs {
            if used().any(|d| (0..20).contains(&d)) {
                return Err("mixing of dIFS with other formulas will not work".into());
            }
            if !used().any(|d| d == 20) {
                return Err("no dIFS shape formula selected".into());
            }
            de_option = 20;
        }
        if de_option < 0 && ipol.is_none() {
            return Err("formula option is not valid".into());
        }
        let mut mode = if (4..=6).contains(&de_option) { HybridMode::Alt4D } else { HybridMode::Alt3D };
        if let Some(w) = ipol {
            // CheckDEoption(0, 1)
            let _ = w;
            de_option = ipol_de_option(&slots, sc.interpolation.unwrap_or_default());
            if de_option >= 20 {
                return Err("dIFS formulas do not work in an interpolation hybrid".into());
            }
            if de_option < 0 {
                return Err("formula option is not valid".into());
            }
            mode = if (4..=6).contains(&de_option) { HybridMode::Ipol4D } else { HybridMode::Ipol3D };
        }
        let mode = mode;
        let is_custom_de = [2, 5, 6, 11, 20].contains(&de_option);
        let difs = de_option == 20;
        // second part of a DE combination
        let part2_cfg = match &dc {
            Some(d) => {
                let (start2, end2) = (d.start2.min(last), d.end2.clamp(d.start2, last));
                let mut de2 = combine_de_options_range(&slots, start2, end2);
                let used2 = || slots.iter().take(end2 + 1).skip(start2).filter(|s| s.iterations != 0).map(|s| s.formula.de_option());
                if used2().any(|x| x >= 20) {
                    if used2().any(|x| (0..20).contains(&x)) {
                        return Err("DE combination: mixing of dIFS with other formulas in the second part will not work".into());
                    }
                    de2 = if used2().any(|x| x == 20) { 20 } else { -1 };
                }
                if sc.disable_analytic_de {
                    de2 = match de2 {
                        2 | 11 => 0,
                        5 | 6 => 4,
                        d => d,
                    };
                }
                if de2 < 0 {
                    return Err("DE combination: the second formula part has no valid DE option".into());
                }
                if d.kind == 6 && de2 != 20 {
                    return Err("DE combination 'mix': the second hybrid part must be dIFS".into());
                }
                let mut rep2 = d.repeat2.clamp(start2, end2);
                while rep2 > start2 && slots[rep2].iterations <= 0 {
                    rep2 -= 1;
                }
                Some((start2, end2, rep2, de2))
            }
            None => None,
        };
        let is_custom_de2 = part2_cfg.is_some_and(|c| [2, 5, 6, 11, 20].contains(&c.3));
        for (x, s) in slots.iter_mut().enumerate() {
            s.ade = if part2_cfg.is_some() && x > end_to { is_custom_de2 } else { is_custom_de };
            // CheckFormulaOptions: formulas without a valid DE option are
            // iterated but not counted (nHybrid < 0)
            let de = s.formula.de_option();
            s.uncounted = s.uncounted || de < 0 || de > 20;
        }

        // DE scale, color helpers, smooth-it logs
        let max_it = sc.iterations;
        let mut fhln = [(1.0 / 2f64.ln()) as f32; 6];
        let (mut n, mut x1, mut y1, mut i) = (0f64, 0f64, 0f64, 0f64);
        let mut part1_scale: Option<(f64, f64)> = None; // (d_de_scale, i) of part 1 in a DE combination
        for (x, s) in slots.iter().enumerate() {
            if s.iterations == 0 {
                continue;
            }
            let second = part2_cfg.is_some() && x > end_to;
            if second && part1_scale.is_none() {
                part1_scale = Some((if n > 0.0 && de_option != 20 { x1 / n } else { 1.0 }, i));
                (n, x1, y1, i) = (0.0, 0.0, 0.0, 0.0);
            }
            let is_custom_de = if second { is_custom_de2 } else { is_custom_de };
            // fHln: only for the counted formulas (nHybrid > 0), 1/ln(2) else
            if ipol.is_some() || !s.uncounted {
                let sip = s.formula.si_pow();
                let z1 = if sip > 1.0 { sip } else { s.formula.first_option() };
                fhln[x] = (1.0 / f64::max(2.0, z1.abs()).ln()) as f32;
            }
            // interpolation hybrid: weighted by the weights, counted once
            let (z1, cnt) = match ipol {
                Some(w) => (w[x] as f64, 1.0),
                None => (s.iterations as f64, s.iterations as f64),
            };
            let j = s.formula.de_option();
            if is_custom_de && (ipol.is_some() || !s.uncounted) && (j == 5 || j == 11) {
                n += cnt;
                i += cnt;
                let mut x2 = s.formula.first_option();
                if x2 == 0.0 {
                    x2 = 1e-40;
                }
                if x2 < 0.0 {
                    x1 += 0.65 * z1;
                } else {
                    x1 += (x2 * 1.2 + 1.0) / x2 * z1;
                }
                y1 += x2.abs() * z1;
            } else if j >= 0 || ipol.is_some() {
                n += cnt;
                x1 += if is_custom_de { s.formula.ade_scale() } else { s.formula.de_scale() } * z1;
            }
        }
        let mut d_col_plus = 0f32;
        let mut d_de_scale = 1f64;
        let mut d_de_scale2 = 1f64;
        if let Some((s1, _)) = part1_scale {
            d_de_scale = s1;
            if n > 0.0 && part2_cfg.is_some_and(|c| c.3 != 20) {
                d_de_scale2 = x1 / n;
            }
        } else if n > 0.0 && de_option != 20 {
            d_de_scale = if ipol.is_some() { x1 } else { x1 / n };
        }
        if n > 0.0 && i > 0.0 {
            d_col_plus = ((y1 / i).abs().powf(0.25) * 40.0 - 49.0) as f32;
        }
        // mctColVarDEstopMul: from the counted formulas of the first part
        // (of a DE combination: of the part with more iterations)
        let mut col_var = 0.6f32;
        {
            let (mut y1, mut z1, mut z2) = (0f64, 0f64, 0f64);
            let (mut ia, mut itmp) = (0, 0);
            let mut second = false;
            for (x, s) in slots.iter().enumerate() {
                let counted = ipol.is_some() || (s.iterations > 0 && !s.uncounted);
                if !counted {
                    if ipol.is_none() && s.iterations > 0 {
                        if x > end_to {
                            ia += 1;
                        } else {
                            itmp += 1;
                        }
                    }
                    continue;
                }
                if x > end_to && !second {
                    second = true;
                    col_var = if z1 > 0.0 { (y1 / z1) as f32 } else { 0.6 };
                    z2 = z1;
                    z1 = 0.0;
                    y1 = 0.0;
                    if dc.is_none() || part2_cfg.is_some_and(|c| c.3 == 20) {
                        break;
                    }
                }
                let x2 = match ipol {
                    Some(w) => w[x] as f64,
                    None => s.iterations as f64,
                };
                let de = s.formula.de_option();
                if de == 0 || de == 4 {
                    y1 += x2 * 0.6;
                    z1 += x2;
                } else if (1..=3).contains(&de) || (5..=40).contains(&de) {
                    if s.formula.first_option_type() == FirstOptionType::Double {
                        let mut v = s.formula.first_option();
                        if v > 1.0 {
                            if v < 1.2 {
                                v = 1.1 + (v - 1.0) * (v - 1.0) * 2.5;
                            }
                            y1 += x2 * 0.5 / (v.ln() + 0.03);
                            z1 += x2;
                        } else if v < -1.0 {
                            if v > -1.2 {
                                v = -1.1 - (v + 1.0) * (v + 1.0) * 2.5;
                            }
                            y1 += x2 * 0.5 / ((-v).ln() + 0.03);
                            z1 += x2;
                        }
                    }
                }
            }
            if (z1 >= z2 && z1 > 1e-3) || (ia > itmp && (z1 - z2).abs() < 1e-3 && z1 > 1e-3) {
                col_var = (y1 / z1) as f32;
            } else if z2 < 1e-3 {
                col_var = 0.6;
            }
        }

        let rstop = sc.effective_rstop();
        let step_width = sc.step_width();
        let fov_y = if sc.optic == CameraOptic::Panorama {
            std::f64::consts::PI
        } else {
            sc.fov_y.to_radians()
        };
        let z_step_div0 = (sc.z_step_div.max(0.0001)) as f32;
        let de_stop = (sc.de_stop.max(0.001)) as f32;
        let x1c = (fov_y * 0.5).clamp(0.01, 1.5);
        let pl_optic_z = (x1c.cos() * x1c / x1c.sin()) as f32;
        let fovx_off = sc.stereo_xoff() * width as f32;
        let fovx_mul = if sc.optic == CameraOptic::Panorama {
            (2.0 * std::f64::consts::PI / width as f64) as f32
        } else {
            (fov_y / height as f64) as f32
        };

        // CalcPPZvals
        let zcorr = (f64::max(sc.fov_y.to_radians(), 1.0) / height as f64).sin();
        let zc_mul = 32767.0 * 256.0
            / (((sc.z_end - sc.z_start) * zcorr / step_width + 1.0).sqrt() - 0.999999999);
        let zend = f64::max(1e-10, (sc.z_end - sc.z_start) / step_width);

        let mh04zsd = (width.max(height) as f64
            * 0.5
            * (z_step_div0 as f64 + 0.001).sqrt()
            * f64::max(0.01, sc.raystep_limiter)) as f32;
        // GetDEstopFactor
        let de_stop_factor = if sc.vary_de_stop_on_fov {
            let ze = f64::max(1e-16, (sc.z_end - sc.z_start) / step_width);
            let x1 = if sc.optic == CameraOptic::Panorama {
                0.01 * height as f64 / (0.01f64.sin() * std::f64::consts::PI)
            } else {
                0.01 * height as f64 / (0.01f64.sin() * f64::max(1.0 / 65535.0, sc.fov_y.to_radians()))
            };
            let x2 = step_width * (x1 + ze) / x1;
            ((x2 - step_width) / (step_width * f64::max(1e-6, ze))) as f32
        } else {
            0.0
        };

        // camera start position
        let (xh, yh) = if sc.optic == CameraOptic::Panorama {
            (0.0, 0.0)
        } else {
            (width as f64 * 0.5, height as f64 * 0.5)
        };
        // z1 := (dZstart - dZmid) / StepWidth
        // 2D: the plane at Z start, the middle or Z end, with the pixel
        // spacing of the perspective there
        let (z1, x2) = match sc.slice_2d {
            0 | 1 => ((sc.z_start - sc.mid[2]) / step_width, step_width),
            3 => {
                let x2 = step_width * (1.0 + fov_y.sin() * zend / height as f64);
                ((sc.z_end - sc.mid[2]) / x2, x2)
            }
            _ => (0.0, step_width * (1.0 + fov_y.sin() * (sc.mid[2] - sc.z_start) / (step_width * height as f64))),
        };
        let vgrads = normalise_matrix_to(x2, &sc.vgrads);
        let mut ystart = [0.0; 3];
        let mid = sc.stereo_mid();
        for k in 0..3 {
            ystart[k] = mid[k] + z1 * vgrads[2][k] - yh * vgrads[1][k] - xh * vgrads[0][k];
        }
        let mut de_offset = (de_stop * 0.1).min(0.004);
        let d_de_scale = if is_custom_de {
            (d_de_scale as f32 as f64 / step_width) as f32
        } else {
            d_de_scale as f32 * de_offset
        };
        let d_de_scale2 = if is_custom_de2 {
            (d_de_scale2 as f32 as f64 / step_width) as f32
        } else {
            d_de_scale2 as f32 * de_offset
        };
        let decomb = match (&dc, part2_cfg) {
            (Some(d), Some((start2, end2, rep2, de2))) => Some(DeComb {
                kind: d.kind,
                part2: HybridPart {
                    start: start2,
                    end: end2,
                    repeat: rep2,
                    de_option: de2,
                    is_custom_de: is_custom_de2,
                    difs: de2 == 20,
                    mode: if (4..=6).contains(&de2) { HybridMode::Alt4D } else { HybridMode::Alt3D },
                    d_de_scale: d_de_scale2,
                    max_it: d.iterations2,
                },
                smooth: (d.smooth.max(1e-30) as f64) / step_width,
                fmix_pow: d.mix_pow as f64,
                mix_col: d.mix_color,
            }),
            _ => None,
        };
        let de_offset006 = de_offset * 0.06;
        de_offset = (de_offset as f64 * step_width) as f32;

        let (z_step_div, ms_de_sub) = if sc.step_sub_de_stop {
            let s = z_step_div0;
            let s = s * s + (1.2 * s) * (1.0 - s);
            (s, s.sqrt().min(0.9))
        } else {
            (z_step_div0, 0.0)
        };
        // dIFS (HeaderTrafos.pas, after the DE offset): more binary search
        // steps, any iteration count is valid, and with only dIFS formulas
        // the step limit is the image size (the raystep limiter does not
        // apply) and the colour does not vary with the DE stop
        let de2 = part2_cfg.map(|c| c.3);
        let (mut de_add_steps, mut min_it) = (sc.bin_search_steps, sc.min_iterations.min(max_it));
        let mut mh04zsd = mh04zsd;
        if de_option == 20 || de2 == Some(20) {
            de_add_steps = de_add_steps.max((z_step_div0.min(1.0) * 2.0).round() as i32 + 2);
            if dc.as_ref().is_none_or(|d| d.kind < 5) {
                min_it = 1;
            }
        }
        if de_option == 20 && (dc.is_none() || de2 == Some(20)) {
            mh04zsd = width.max(height) as f32;
            col_var = 0.0;
        }
        let ln_rstop = rstop.ln().ln() as f32;
        let d_rstop = (rstop * rstop) as f32 as f64;
        let rstop3d = ((d_rstop * d_rstop) * 64.0) as f32 as f64;
        // the iterations of both parts of a DE combination (iMaxitF2)
        let its = match &dc {
            Some(d) if d.kind > 5 => max_it + d.iterations2,
            Some(d) => max_it.max(d.iterations2),
            None => max_it,
        };
        let mcts_m = 32767.0 / ((its + 1).max(1) as f32);
        let d_col_plus = d_col_plus + its as f32 * 0.1;

        #[allow(unused_mut)]
        let mut p = CalcParams {
            slots,
            mode,
            ipol: ipol.unwrap_or([1.0, 0.0]),
            color_on_it: sc.color_on_it,
            slice_2d: sc.slice_2d.min(3),
            is_custom_de,
            de_option,
            difs,
            d_de_scale,
            sm_normals: sc.smooth_normals.clamp(0, 8),
            de_stop,
            z_step_div,
            de_add_steps,
            width,
            height,
            fov_y,
            max_it,
            min_it,
            color_option: sc.color_option.min(5),
            mct_color_mul: (sc.color_mul * 512.0) as f32,
            rect: sc.calc_rect.unwrap_or([0, 0, width, height]),
            d_rstop,
            rstop3d,
            do_julia: sc.julia,
            ju: sc.julia_c,
            vgrads,
            pl_optic_z,
            optic: sc.optic,
            dfog_on_it: sc.dfog_on_it,
            fovx_off,
            fovx_mul,
            d_col_plus,
            zcorr,
            zc_mul,
            zend,
            step_width,
            col_var_de_stop_mul: col_var,
            first_step_random: sc.first_step_random,
            mh04zsd,
            de_stop_factor,
            ln_rstop,
            normals_on_de: sc.normals_on_de || is_custom_de || dc.is_some(),
            ystart,
            de_offset,
            de_offset006,
            mcts_m,
            ms_de_sub,
            fhln,
            smatrix4: build_smatrix4(sc.rot_4d[0], sc.rot_4d[1], sc.rot_4d[2]),
            end_to,
            repeat_from,
            zz_stmit_dif: sc.z_start - sc.mid[2],
            cut_options: if sc.cut_options & 7 != 0 {
                (sc.cut_options & 7)
                    | ((vgrads[2][0] > 0.0) as u8) << 4
                    | ((vgrads[2][1] > 0.0) as u8) << 5
                    | ((vgrads[2][2] > 0.0) as u8) << 6
            } else {
                0
            },
            cut_pos: sc.cut_pos,
            decomb,
            inside_rendering: sc.inside != InsideMode::Outside,
            in_and_outside: sc.inside == InsideMode::Both,
            vol: None,
            gpu_post: None,
            gpu_post_secs: None,
            gpu_double: false,
            its: Default::default(),
            machine,
        };
        #[cfg(jit)]
        p.attach_jit();
        Ok(p)
    }

    /// The alternating hybrid compiled at run time (`clif_jit`), if it is
    /// faster than the formulas called one by one (timed once per hybrid on
    /// points of three rays of the view; the result is kept with the
    /// compiled code, see `clif_jit::cached`).  `MB3D_JIT=force`: without
    /// the timing.
    #[cfg(jit)]
    fn attach_jit(&mut self) {
        use crate::x86::clif_jit;
        if !clif_jit::enabled() {
            return;
        }
        if self.difs {
            // the dIFS loop
            let Some(m) = self.machine.as_ref() else { return };
            let key = clif_jit::hybrid_key(m, &self.slots, 0, self.end_to, self.repeat_from, false) ^ 0xD1F5;
            let jit = match clif_jit::cached_difs(key) {
                Some(j) => j,
                None => {
                    let h = clif_jit::compile_difs(m, &self.slots, 0, self.end_to, self.repeat_from).map(std::sync::Arc::new);
                    let keep = h.filter(|h| {
                        clif_jit::forced()
                            || self.jit_is_faster(
                                &|m: &mut crate::x86::Machine, on: bool| m.difs_jit = if on { Some(h.clone()) } else { None },
                                false,
                            )
                    });
                    clif_jit::cache_difs(key, keep.clone());
                    keep
                }
            };
            if let Some(m) = self.machine.as_mut() {
                m.difs_jit = jit;
            }
            return;
        }
        if !matches!(self.mode, HybridMode::Alt3D | HybridMode::Alt4D) {
            return;
        }
        // a hybrid of built-in formulas only has no machine: an empty one
        // carries the compiled loop (kept only when it is used)
        let builtin_only = self.machine.is_none();
        if builtin_only {
            if self.slots.iter().any(|s| matches!(s.formula, crate::formulas::Formula::Custom(_))) {
                return;
            }
            self.machine = Some(crate::custom::build_machine(&[]));
        }
        let Some(m) = self.machine.as_ref() else { return };
        let four_d = self.mode == HybridMode::Alt4D;
        let key = clif_jit::hybrid_key(m, &self.slots, 0, self.end_to, self.repeat_from, four_d);
        let jit = match clif_jit::cached(key) {
            Some(j) => j,
            None => {
                let h = clif_jit::compile_hybrid(m, &self.slots, 0, self.end_to, self.repeat_from, four_d).map(std::sync::Arc::new);
                let keep = h.filter(|h| clif_jit::forced() || self.jit_is_faster(&|m: &mut crate::x86::Machine, on: bool| m.jit = if on { Some(h.clone()) } else { None }, builtin_only));
                clif_jit::cache(key, keep.clone());
                keep
            }
        };
        if builtin_only && jit.is_none() {
            self.machine = None;
        } else if let Some(m) = self.machine.as_mut() {
            m.jit = jit;
        }
    }

    /// Whether the compiled hybrid is faster than the formulas called one
    /// by one (at least 3 %, the best of three rounds each); `set` puts it
    /// into the machine (true) or takes it out.
    #[cfg(jit)]
    fn jit_is_faster(&mut self, set: &dyn Fn(&mut crate::x86::Machine, bool), builtin_only: bool) -> bool {
        let rays = [(0.5, 0.5), (0.25, 0.3), (0.7, 0.65)];
        let pts: Vec<Vec3> = rays
            .iter()
            .flat_map(|&(fx, fy)| {
                (0..200).map(move |k| (fx, fy, k as f64 / 200.0))
            })
            .map(|(fx, fy, s)| {
                let mut c = [0.0; 3];
                for (i, ci) in c.iter_mut().enumerate() {
                    *ci = self.ystart[i] + self.vgrads[0][i] * (self.width as f64 * fx) + self.vgrads[1][i] * (self.height as f64 * fy) + self.vgrads[2][i] * s * 400.0;
                }
                c
            })
            .collect();
        // the loop without the compiled hybrid runs as it would: without
        // the machine for a hybrid of built-in formulas
        let time = |p: &mut CalcParams, on: bool| -> f64 {
            let without = !on;
            if let Some(m) = p.machine.as_mut() {
                set(m, on);
            }
            let saved = if without && builtin_only { p.machine.take() } else { None };
            let mut mr = Marcher::new(p, 1);
            let t = std::time::Instant::now();
            for c in &pts {
                mr.iterations_at(*c);
            }
            let dt = t.elapsed().as_secs_f64();
            drop(mr);
            if saved.is_some() {
                p.machine = saved;
            }
            dt
        };
        let (mut tj, mut tr) = (f64::MAX, f64::MAX);
        for _ in 0..3 {
            tj = tj.min(time(self, true));
            tr = tr.min(time(self, false));
        }
        tj < tr * 0.97
    }

    /// `IniIt3D`
    pub fn new_iteration(&self) -> Iteration {
        Iteration {
            v: [0.0; 4],
            c: [0.0; 3],
            j: [self.ju[0], self.ju[1], self.ju[2], self.ju[3]],
            ju: self.ju,
            do_julia: self.do_julia,
            rout: 0.0,
            rold: 0.0,
            otrap: 0.0,
            it_result: 0,
            max_it: self.max_it,
            rstop: self.d_rstop,
            calc_sit: false,
            smooth_it: 0.0,
            ln_rstop: self.ln_rstop,
            de_option: self.de_option,
            deriv1: 1.0,
            deriv2: 0.0,
            deriv3: 0.0,
            smatrix4: self.smatrix4,
            start_from: 0,
            end_to: self.end_to,
            repeat_from: self.repeat_from,
            fhln: self.fhln,
            vary_scale: 1.0,
            first_it: 0,
            dfree: [0.0; 2],
            emu: self.machine.as_ref().map(|m| Box::new(self.init_machine(m))),
            ipol: self.ipol,
            direct: Default::default(),
            jit_off: false,
        }
    }

    /// Writes the static parts of the iteration record into a fresh copy
    /// of the prototype machine.
    fn init_machine(&self, proto: &crate::x86::Machine) -> crate::x86::Machine {
        use crate::custom::{off, pconst_addr, IT_BASE};
        let mut m = proto.clone();
        let b = IT_BASE;
        let _ = m.wrf64(b + off::RSTOPD, self.d_rstop);
        for (i, s) in self.slots.iter().enumerate() {
            let _ = m.wr32(b + off::N_HYBRID + 4 * i as u32, s.iterations as u32);
            let _ = m.wr32(b + off::FHPVAR + 4 * i as u32, pconst_addr(i));
            let _ = m.wrf32(b + off::FHLN + 4 * i as u32, self.fhln[i]);
        }
        let _ = m.wr16(b + off::END_TO, self.end_to as u16);
        let _ = m.wr16(b + off::REPEAT_FROM, self.repeat_from as u16);
        let _ = m.wr32(b + off::DO_JULIA, if self.do_julia { u32::MAX } else { 0 });
        let _ = m.wrf32(b + off::LN_RSTOP, self.ln_rstop);
        let _ = m.wr32(b + off::DE_OPTION, self.de_option as u32);
        for r in 0..4 {
            for c in 0..4 {
                let _ = m.wrf32(b + off::SMATRIX4 + (r * 4 + c) as u32 * 4, self.smatrix4[r][c] as f32);
            }
        }
        for k in 0..4 {
            let _ = m.wrf64(b + off::JU1 + 8 * k as u32, self.ju[k]);
        }
        m
    }
}

/// The random state of pixel (x, y) of the picture at its first step
/// ("first step random"): MB3D's seed of row y, advanced by one step of
/// its random generator per column x (MB3D draws once per pixel along the
/// row).  By the column in the whole picture, so that a tile or a band
/// that starts inside a row gets the same numbers as the whole picture
/// (the rows used to start their sequence at the tile's first column, and
/// tiles came out different).
pub fn pixel_seed(y: i64, x: i64) -> i32 {
    let row = (0x24563487u32 as i64 + (y + 1) * 0x324594A1i64) as i32 as u32;
    // the generator s -> 214013 s + 2531011 applied x times, by squaring
    let (mut a, mut c, mut ra, mut rc) = (214013u32, 2531011u32, 1u32, 0u32);
    let mut n = x.max(0) as u64;
    while n > 0 {
        if n & 1 != 0 {
            rc = rc.wrapping_mul(a).wrapping_add(c);
            ra = ra.wrapping_mul(a);
        }
        c = c.wrapping_mul(a).wrapping_add(c);
        a = a.wrapping_mul(a);
        n >>= 1;
    }
    ra.wrapping_mul(row).wrapping_add(rc) as i32
}

/// Per-thread ray marching state (the mutable parts of `TMCTparameter`).
pub struct Marcher<'a> {
    pub p: &'a CalcParams,
    pub it: Iteration,
    pub ms_de_stop: f32,
    pub mzz: f64,
    pub vfov: Vec3,
    pub cafy: f64,
    pub max_its_result: i32,
    pub s_roughness: f32,
    pub seed: i32,
    /// `bInsideRendering` / `bCalcInside` of the current pixel
    pub inside: bool,
    pub calc_inside: bool,
    /// the last DE came from a dIFS part
    pub(crate) last_difs: bool,
}

thread_local! {
    /// The iteration statistics of the pixels this thread's marcher has
    /// marched so far (sum, count, most): added to its parameters'
    /// [`ItStats`] when the marcher is dropped.  (Kept out of the marcher:
    /// a larger `Marcher` made some scenes 2-5 % slower.)
    static ITS: std::cell::Cell<(u64, u64, i32)> = const { std::cell::Cell::new((0, 0, 0)) };
    /// the marches of this thread do not count (probes)
    static ITS_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The marches of the calling thread do not count in the statistics of
/// their calculation while `off` (probes, samples).
pub fn statistics_off(off: bool) {
    ITS_OFF.with(|c| c.set(off));
}

impl Drop for Marcher<'_> {
    fn drop(&mut self) {
        let (s, n, m) = ITS.with(|c| c.replace((0, 0, 0)));
        if n > 0 && !ITS_OFF.with(|c| c.get()) {
            self.p.its.add(s, n, m);
        }
    }
}

impl<'a> Marcher<'a> {
    pub fn new(p: &'a CalcParams, seed: i32) -> Self {
        Marcher {
            p,
            it: p.new_iteration(),
            ms_de_stop: p.de_stop,
            mzz: 0.0,
            vfov: [0.0, 0.0, 1.0],
            cafy: 0.0,
            max_its_result: p.max_it,
            s_roughness: 0.0,
            seed,
            inside: p.inside_rendering,
            calc_inside: p.inside_rendering,
            last_difs: false,
        }
    }

    #[inline]
    pub(crate) fn mand_function(&mut self) {
        if self.p.difs {
            // dIFS formulas only work in their own loop
            let rstopd = self.ms_de_stop as f64 * self.p.step_width * 1.03;
            self.it.hybrid_difs(&self.p.slots, rstopd, false, true);
            return;
        }
        self.it.mand_function(self.p.mode, &self.p.slots);
    }

    /// `mMandFunction` / `mMandFunctionDE` at a point (voxel export "on
    /// maximum iterations"): returns the iteration count reached.
    pub fn iterations_at(&mut self, pos: Vec3) -> i32 {
        self.it.c = pos;
        self.it.calc_sit = false;
        if self.p.is_custom_de && !self.p.difs {
            self.it.mand_function_de(self.p.mode, &self.p.slots);
        } else {
            self.mand_function();
        }
        self.it.it_result
    }

    /// `CalcDE` at a point, in units of the step width (voxel and mesh
    /// export).  `smooth`: also calculate the smoothed iterations
    /// (`CalcSIT`) for colouring.
    pub fn de_at_point(&mut self, pos: Vec3, smooth: bool) -> f64 {
        self.it.c = pos;
        self.it.calc_sit = smooth;
        self.calc_de()
    }

    /// Colouring values (`SIgradient`, `OTrap`) of the point last passed to
    /// [`Marcher::de_at_point`] with `smooth` (BulbTracer2's
    /// `CalcSIgradient1`); `de` is the distance estimate it returned.
    pub fn color_values(&self, de: f64) -> (u16, u16) {
        let mut si = SiLight::default();
        self.do_color(&mut si);
        if self.p.decomb.is_some() {
            // colour on the DE
            let s = (de * 40.0).abs();
            if self.it.it_result < self.max_its_result {
                si.si_gradient = min_max_clip_15bit(s as f32);
            } else {
                si.si_gradient = (s.clamp(0.0, 32767.0).round() as u16) | 32768;
            }
        } else if self.it.it_result < self.p.max_it {
            si.si_gradient = min_max_clip_15bit(self.it.smooth_it * 32767.0 / self.p.max_it.max(1) as f32);
        } else {
            si.si_gradient = si.otrap.wrapping_add(32768);
        }
        (si.si_gradient, si.otrap)
    }

    /// `CalcDEanalytic` / `CalcDEnoADE`
    pub fn calc_de(&mut self) -> f64 {
        let p = self.p;
        let part1 = HybridPart {
            start: 0,
            end: p.end_to,
            repeat: p.repeat_from,
            de_option: p.de_option,
            is_custom_de: p.is_custom_de,
            difs: p.difs,
            mode: p.mode,
            d_de_scale: p.d_de_scale,
            max_it: p.max_it,
        };
        let Some(dc) = &p.decomb else {
            let r = self.calc_de_part(&part1, false, false);
            return self.invert_inside(r);
        };
        // CalcDEfull with FormulaType > 0
        let mut result = self.calc_de_part(&part1, false, false);
        let (wt, dt) = (self.it.v[3].abs(), self.it.deriv1.abs());
        // PrepareF1DEcomb
        let buf = (self.it.it_result, self.it.otrap, self.it.smooth_it, self.max_its_result);
        self.set_part(&dc.part2);
        let rst = self.calc_de_part(&dc.part2, dc.kind == 3, dc.kind == 6);
        let ms = self.ms_de_stop as f64;
        let restore1 = |me: &mut Self| {
            me.it.it_result = buf.0;
            me.it.smooth_it = buf.2;
            me.it.otrap = buf.1;
            me.max_its_result = buf.3;
        };
        match dc.kind {
            1 => {
                if rst >= result {
                    restore1(self);
                } else {
                    result = rst;
                }
            }
            2 | 3 => {
                if rst > result {
                    result = rst;
                } else {
                    restore1(self);
                }
            }
            4 | 5 => {
                if rst < result {
                    // part 2 rules
                } else {
                    self.it.it_result = buf.0;
                    self.max_its_result = buf.3;
                }
                let sm = dc.smooth;
                if dc.kind == 4 {
                    result = f64::min(rst - (sm - result).max(0.0), result - (sm - rst).max(0.0));
                } else {
                    let rd1 = (sm - result - ms).max(0.0);
                    let d2 = (sm - rst - ms).max(0.0);
                    result = f64::min(rst - rd1 * (sm - d2) / sm, result - d2 * (sm - rd1) / sm);
                }
                let (r2, w) = (rst.abs(), result.abs());
                let f = 1.0 / (w + r2 + 1e-10);
                self.it.smooth_it = ((self.it.smooth_it as f64 * w + buf.2 as f64 * r2) * f) as f32;
                self.it.otrap = (self.it.otrap * w + buf.1 * r2) * f;
            }
            _ => {
                // mix: part 1 first, then the dIFS part for the distance
                match dc.mix_col {
                    0 => {
                        self.it.smooth_it += buf.2;
                        self.it.otrap = (self.it.otrap + buf.1) * 0.5;
                    }
                    1 => {
                        self.it.smooth_it = buf.2;
                        self.it.otrap = buf.1;
                    }
                    _ => {}
                }
                result = match p.de_option {
                    5 | 6 => rst / dt,
                    2 | 11 => rst / wt,
                    _ => rst * dc.fmix_pow.abs().powi(-1 - buf.0),
                };
            }
        }
        // RestoreF1DEcomb
        self.set_part(&part1);
        self.invert_inside(result)
    }

    /// Inside rendering: the distance to the surface from inside the set.
    fn invert_inside(&mut self, r: f64) -> f64 {
        if !self.calc_inside {
            return r;
        }
        let ms = self.ms_de_stop as f64;
        if self.last_difs {
            ms * 2.0 - r
        } else {
            let r = ms * 4.0 - r * 3.0;
            if r >= ms {
                self.it.it_result -= 1;
            }
            r
        }
    }

    /// Switches the iteration to a hybrid part (`PrepareF1DEcomb` /
    /// `RestoreF1DEcomb`).
    fn set_part(&mut self, part: &HybridPart) {
        self.it.start_from = part.start;
        self.it.end_to = part.end;
        self.it.repeat_from = part.repeat;
        self.it.max_it = part.max_it;
        self.it.de_option = part.de_option;
        if let Some(m) = self.it.emu.as_deref_mut() {
            use crate::custom::{off, IT_BASE};
            m.put_u32(IT_BASE + off::REPEAT_FROM, part.repeat as u32 | (part.start as u32) << 16);
            let e = m.get_u32(IT_BASE + off::CALC_SIT) & 0xFFFF;
            m.put_u32(IT_BASE + off::CALC_SIT, e | (part.end as u32) << 16);
            m.put_u32(IT_BASE + off::DE_OPTION, part.de_option as u32);
        }
    }

    /// `CalcDEanalytic` / `CalcDEnoADE` for one hybrid part.  `invert`: the
    /// part is calculated as "inside" (`bCalcInside`, used by the inverted
    /// max combination); `no_vec_ini`: dIFS continues from the current vector
    /// (`doHybridIFS3DnoVecIni`).
    fn calc_de_part(&mut self, part: &HybridPart, invert: bool, no_vec_ini: bool) -> f64 {
        let p = self.p;
        let mut result;
        self.last_difs = part.difs;
        if part.difs {
            // CalcDEanalytic, DEoption 20: absolute DE stop as RStopD
            let rstopd = self.ms_de_stop as f64 * p.step_width * 1.03;
            let mut r = self.it.hybrid_difs(&p.slots, rstopd, invert || self.inside, !no_vec_ini) * part.d_de_scale as f64;
            if r.is_nan() {
                r = self.ms_de_stop as f64 * 0.25;
            }
            // dIFS never counts as "inside by iterations"
            self.max_its_result = self.it.max_it + 1;
            if invert {
                r = self.ms_de_stop as f64 * 2.0 - r;
            }
            return r;
        } else if part.is_custom_de {
            result = self.it.mand_function_de(part.mode, &p.slots) * part.d_de_scale as f64;
        } else {
            self.it.mand_function(part.mode, &p.slots);
            if self.inside && self.it.it_result == self.it.max_it {
                result = 0.0;
            } else if self.it.rout < 1e-200 {
                result = 0.0;
            } else {
                let buf_sit = self.it.calc_sit;
                let buf_max_it = self.it.max_it;
                let buf_rout = self.it.rout;
                self.it.calc_sit = false;
                self.it.max_it = self.it.it_result;
                self.it.rstop = p.rstop3d;
                let off = p.de_offset as f64;
                let mut g = [0.0; 3];
                for k in 0..3 {
                    let bufd = self.it.c[k];
                    self.it.c[k] = bufd + off;
                    self.it.mand_function(part.mode, &p.slots);
                    g[k] = (buf_rout - self.it.rout) * (buf_rout - self.it.rout);
                    self.it.c[k] = bufd;
                }
                result = buf_rout * buf_rout.ln() * part.d_de_scale as f64
                    / ((g[0] + g[1] + g[2]).sqrt() + p.de_offset006 as f64);
                self.it.rout = buf_rout;
                self.it.it_result = self.it.max_it;
                self.it.max_it = buf_max_it;
                self.it.calc_sit = buf_sit;
                self.it.rstop = p.d_rstop;
            }
        }
        self.max_its_result = self.it.max_it;
        let min = self.ms_de_stop as f64 * 0.25;
        if result < min || result.is_nan() {
            result = min;
        }
        if invert {
            result = self.ms_de_stop as f64 * 4.0 - result * 3.0;
            if result >= self.ms_de_stop as f64 {
                self.it.it_result -= 1;
            }
        }
        result
    }

    /// `RMCalculateVgradsFOV`
    #[inline]
    pub(crate) fn calc_vgrads_fov(&mut self, ix: i32) {
        let p = self.p;
        let cafx = ((p.fovx_off - ix as f32) * p.fovx_mul) as f64;
        let v = match p.optic {
            CameraOptic::Planar => {
                normalize([-cafx, self.cafy, p.pl_optic_z as f64])
            }
            CameraOptic::Panorama => build_view_vector_dsphere_fov(self.cafy, cafx),
            CameraOptic::Common => build_view_vector_dfov(self.cafy, cafx),
        };
        self.vfov = rotate_vector_reverse(&v, &p.vgrads);
    }

    /// `RMCalculateStartPos`
    #[inline]
    pub(crate) fn calc_start_pos(&mut self, ix: i32, iy: i32) {
        let p = self.p;
        if p.optic == CameraOptic::Panorama {
            self.it.c = p.ystart;
        } else {
            let (x, y) = (ix as f64, iy as f64);
            for k in 0..3 {
                self.it.c[k] = p.ystart[k] + p.vgrads[0][k] * x + p.vgrads[1][k] * y;
            }
        }
    }

    #[inline]
    fn step(&mut self, d: f64) {
        let v = self.vfov;
        add_weight(&mut self.it.c, &v, d);
    }

    #[inline]
    fn update_de_stop(&mut self) {
        self.ms_de_stop =
            (self.p.de_stop as f64 * (1.0 + self.mzz * self.p.de_stop_factor as f64)) as f32;
    }

    /// `RMdoBinSearch`
    pub(crate) fn bin_search(&mut self, de: &mut f64, last_step_width: f64) {
        let mut itmp = self.p.de_add_steps;
        let mut dt1 = last_step_width * -0.5;
        while (*de - self.ms_de_stop as f64).abs() > 0.001 {
            self.mzz += dt1;
            self.step(dt1);
            self.update_de_stop();
            itmp -= 1;
            if itmp <= 0 {
                break;
            }
            *de = self.calc_de();
            if self.it.it_result >= self.max_its_result {
                dt1 = -dt1.abs();
            } else if *de < self.ms_de_stop as f64 {
                dt1 = dt1.abs() * -0.55;
            } else {
                dt1 = dt1.abs() * 0.55;
            }
        }
    }

    /// `RMdoBinSearchIt` – refine the surface on the iteration count.
    pub(crate) fn bin_search_it(&mut self) {
        let yp = self.it.max_it as f32 - 0.99;
        let saved_max = self.it.max_it;
        self.it.max_it += 1;
        let mut itmp = self.p.de_add_steps;
        self.it.calc_sit = true;
        let mut dt1 = 0f64;
        let mut dmul = 1f64;
        let mut first = true;
        let (mut last_si, mut last_dif) = (0f32, 0f32);
        loop {
            self.mzz += dt1;
            self.step(dt1);
            self.calc_de();
            if !first && last_dif < (yp - self.it.smooth_it).abs() {
                self.mzz -= dt1;
                self.step(-dt1);
                self.it.smooth_it = last_si;
                if dt1 > 0.0 {
                    dmul *= 0.5;
                } else {
                    dmul *= 0.7;
                }
            }
            last_dif = (yp - self.it.smooth_it).abs();
            last_si = self.it.smooth_it;
            if self.it.smooth_it > self.it.max_it as f32 - 0.1 {
                dt1 = -3.0;
            } else {
                self.mzz -= 0.001;
                self.step(-0.001);
                self.calc_de();
                let r = last_si - self.it.smooth_it;
                dt1 = if r.abs() < 1e-30 {
                    (if last_si < self.it.smooth_it { 1.0 } else { 0.0 }) - 0.5
                } else if r < 0.0 {
                    ((yp - self.it.smooth_it) / (r * 500.0)) as f64
                } else {
                    ((yp - self.it.smooth_it) / (r * 1000.0)) as f64
                };
                if dt1 > 4.0 {
                    dt1 = dt1.sqrt() * 2.0;
                } else if dt1 < -9.0 {
                    dt1 = (-dt1).sqrt() * -3.0;
                }
                dt1 = dt1 * dmul + 0.0005;
            }
            first = false;
            itmp -= 1;
            if itmp < 0 {
                break;
            }
        }
        self.it.max_it = saved_max;
    }

    /// `MakeWNormalsFromDVec`
    fn store_normal(si: &mut SiLight, n: &Vec3) {
        let d = 32767.0 / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] + 1e-100).sqrt();
        for k in 0..3 {
            let v = (n[k] * d).round_ties_even();
            si.normal[k] = if v.is_nan() { 0 } else { v.clamp(-32767.0, 32767.0) as i16 };
        }
        if si.normal[0] == 0 && si.normal[1] == 0 {
            si.normal[2] = if si.normal[2] > 0 { 32767 } else { -32767 };
        }
    }

    /// `RMCalcRoughness`
    fn calc_roughness(n: &Vec3, dt2: f64, dsg: f64) -> f32 {
        let a = (dsg * 7.0 * dt2 * dt2 + 1e-40) / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] + 1e-40);
        (a.max(0.0).sqrt() - 0.05).clamp(0.0, 1.0) as f32
    }

    /// `RMCalculateNormals` (normals from the DE gradient). Returns NN (the
    /// smoothed iteration count used for colouring).
    pub(crate) fn calculate_normals(&mut self, si: &mut SiLight) -> f32 {
        let p = self.p;
        let vg = p.vgrads;
        self.it.calc_sit = true;
        let mut noffset =
            (p.de_stop.min(1.0) as f64) * (1.0 + self.mzz * p.de_stop_factor as f64) * 0.15;
        let ct1 = self.it.c;
        let mut dnn = self.calc_de();
        let nn = self.it.smooth_it;
        self.it.calc_sit = false;
        let mut n = [0.0; 3];
        if p.sm_normals == 8 {
            let ssn = noffset * 1.3333;
            for a in -2i32..=2 {
                for b in -2i32..=2 {
                    for c in -2i32..=2 {
                        if (a | b | c) == 0 {
                            continue;
                        }
                        self.it.c = ct1;
                        add_weight(&mut self.it.c, &vg[2], a as f64 * ssn);
                        add_weight(&mut self.it.c, &vg[1], b as f64 * ssn);
                        add_weight(&mut self.it.c, &vg[0], c as f64 * ssn);
                        let d = self.calc_de();
                        if a != 0 {
                            n[2] += d / a as f64;
                        }
                        if b != 0 {
                            n[1] += d / b as f64;
                        }
                        if c != 0 {
                            n[0] += d / c as f64;
                        }
                    }
                }
            }
            n = scale(n, 0.0075);
        } else {
            for (k, axis) in [(2usize, 2usize), (0, 0), (1, 1)] {
                self.it.c = ct1;
                add_weight(&mut self.it.c, &vg[axis], noffset);
                let a = self.calc_de();
                add_weight(&mut self.it.c, &vg[axis], -2.0 * noffset);
                n[k] = (a - self.calc_de()) * 0.5;
            }
        }
        if p.sm_normals > 0 {
            noffset *= 2.0;
            if p.sm_normals < 8 {
                self.it.c = ct1;
                add_weight(&mut self.it.c, &vg[0], -noffset);
                dnn += self.calc_de();
                add_weight(&mut self.it.c, &vg[0], 2.0 * noffset);
                dnn += self.calc_de();
                self.it.c = ct1;
                add_weight(&mut self.it.c, &vg[1], -noffset);
                dnn += self.calc_de();
                add_weight(&mut self.it.c, &vg[1], 2.0 * noffset);
                dnn = (dnn + self.calc_de()) * 0.2;
            }
            let sm = p.sm_normals;
            let ssn = noffset * 3.0 / (sm as f64 + 0.5);
            let dm = sm as f64 * 2.0;
            let (vx, vy) = create_xy_vecs_from_normals(&n);
            let vx = rotate_vector_reverse(&vx, &vg);
            let vy = rotate_vector_reverse(&vy, &vg);
            let sweep = |dir: &Vec3, me: &mut Self| -> (f64, f64) {
                me.it.c = ct1;
                add_weight(&mut me.it.c, dir, -(sm as f64) * ssn);
                let (mut nn1, mut ds) = (0.0, 0.0);
                for k in -sm..=sm {
                    if k != 0 {
                        let d = (me.calc_de() - dnn) / k as f64;
                        nn1 += d;
                        ds += d * d;
                    }
                    add_weight(&mut me.it.c, dir, ssn);
                }
                (nn1, ds)
            };
            let (nn1, ds1) = sweep(&vx, self);
            let (nn2, ds2) = sweep(&vy, self);
            let dsg = ds1 * dm - nn1 * nn1 + ds2 * dm - nn2 * nn2;
            let dt2 = noffset * 0.5 / (dm * ssn);
            self.s_roughness = Self::calc_roughness(&n, dt2, dsg);
            if p.sm_normals < 8 {
                n[0] += nn1 * dt2;
                n[1] += nn2 * dt2;
            }
        }
        self.it.c = ct1;
        Self::store_normal(si, &n);
        nn
    }

    /// `RMCalculateNormalsOnSmoothIt` (normals from the smooth iteration
    /// gradient, used when "Normals on DE" is off and no analytic DE).
    pub(crate) fn calculate_normals_on_smooth_it(&mut self, si: &mut SiLight) -> f32 {
        let p = self.p;
        let vg = p.vgrads;
        let mut noffset =
            (p.de_stop.min(1.0) as f64) * (1.0 + self.mzz * p.de_stop_factor as f64) * 0.15;
        self.it.calc_sit = true;
        let ct1 = self.it.c;
        self.mand_function();
        let mut nn = self.it.smooth_it;
        let mut n = [0.0; 3];
        if p.sm_normals == 8 {
            let ssn = noffset * 1.3333;
            for a in -2i32..=2 {
                for b in -2i32..=2 {
                    for c in -2i32..=2 {
                        if (a | b | c) == 0 {
                            continue;
                        }
                        self.it.c = ct1;
                        add_weight(&mut self.it.c, &vg[2], a as f64 * ssn);
                        add_weight(&mut self.it.c, &vg[1], b as f64 * ssn);
                        add_weight(&mut self.it.c, &vg[0], c as f64 * ssn);
                        self.mand_function();
                        let d = self.it.smooth_it as f64;
                        if a != 0 {
                            n[2] -= d / a as f64;
                        }
                        if b != 0 {
                            n[1] -= d / b as f64;
                        }
                        if c != 0 {
                            n[0] -= d / c as f64;
                        }
                    }
                }
            }
            n = scale(n, 0.0075);
        } else {
            for (k, axis) in [(2usize, 2usize), (0, 0), (1, 1)] {
                self.it.c = ct1;
                add_weight(&mut self.it.c, &vg[axis], noffset);
                self.mand_function();
                let a = self.it.smooth_it as f64;
                add_weight(&mut self.it.c, &vg[axis], -2.0 * noffset);
                self.mand_function();
                n[k] = (self.it.smooth_it as f64 - a) * 0.5;
            }
        }
        if p.sm_normals > 0 {
            noffset *= 2.0;
            let mut acc = nn as f64;
            for (axis, sgn) in [(0usize, -1.0), (0, 1.0), (1, -1.0), (1, 1.0)] {
                self.it.c = ct1;
                add_weight(&mut self.it.c, &vg[axis], sgn * noffset);
                self.mand_function();
                acc += self.it.smooth_it as f64;
            }
            nn = (acc * 0.2) as f32;
            let sm = p.sm_normals;
            let ssn = noffset * 3.0 / (sm as f64 + 0.5);
            let dm = sm as f64 * 2.0;
            let (vx, vy) = create_xy_vecs_from_normals(&n);
            let vx = rotate_vector_reverse(&vx, &vg);
            let vy = rotate_vector_reverse(&vy, &vg);
            let nnv = nn as f64;
            let sweep = |dir: &Vec3, me: &mut Self| -> (f64, f64) {
                me.it.c = ct1;
                add_weight(&mut me.it.c, dir, -(sm as f64) * ssn);
                let (mut s1, mut ds) = (0.0, 0.0);
                for k in -sm..=sm {
                    if k != 0 {
                        me.mand_function();
                        let d = (nnv - me.it.smooth_it as f64) / k as f64;
                        s1 += d;
                        ds += d * d;
                    }
                    add_weight(&mut me.it.c, dir, ssn);
                }
                (s1, ds)
            };
            let (nn1, ds1) = sweep(&vx, self);
            let (nn2, ds2) = sweep(&vy, self);
            let dsg = ds1 * dm - nn1 * nn1 + ds2 * dm - nn2 * nn2;
            let dt2 = noffset * 0.5 / (dm * ssn);
            self.s_roughness = Self::calc_roughness(&n, dt2, dsg);
            if sm < 8 {
                n[0] += nn1 * dt2;
                n[1] += nn2 * dt2;
            }
        }
        self.it.c = ct1;
        if self.inside {
            n = scale(n, -1.0);
        }
        Self::store_normal(si, &n);
        nn
    }

    /// `doColorOnIt`: colour on the start vector (1) or on the vector after
    /// `ColorOnIt - 1` iterations.
    pub(crate) fn do_color_on_it(&mut self) {
        let p = self.p;
        if p.color_on_it == 1 {
            self.it.v[0] = self.it.c[0];
            self.it.v[1] = self.it.c[1];
            self.it.v[2] = self.it.c[2];
        } else {
            let i = self.it.max_it;
            self.it.max_it = p.color_on_it as i32 - 1;
            self.it.rstop = p.rstop3d;
            self.calc_de();
            self.it.max_it = i;
            self.it.rstop = p.d_rstop;
        }
    }

    /// `T2DcalcThread`: one pixel of a 2D calculation, the iteration
    /// colouring of the plane with a flat normal.
    pub fn slice_pixel(&mut self, x: i32, y: i32) -> SiLight {
        let p = self.p;
        let mut si = SiLight {
            normal: [0, 0, -32768],
            zpos_fine: 0x4E20_0000,
            shadow: 0x0010,
            amb_shadow: 0x1388,
            si_gradient: 0,
            otrap: 0,
        };
        self.inside = false;
        self.calc_inside = false;
        self.it.calc_sit = true;
        for k in 0..3 {
            self.it.c[k] = p.ystart[k] + y as f64 * p.vgrads[1][k] + x as f64 * p.vgrads[0][k];
        }
        self.mand_function();
        if p.color_on_it != 0 {
            self.do_color_on_it();
        }
        self.do_color(&mut si);
        if p.decomb.is_some() {
            // colour on the DE
            let s = (self.calc_de() * 40.0).abs() as f32;
            if self.it.it_result < self.max_its_result {
                si.si_gradient = min_max_clip_15bit(s);
                si.normal[1] = 5000;
            } else {
                si.si_gradient = (s.clamp(0.0, 32767.0).round_ties_even() as u16) | 32768;
            }
        } else if self.it.it_result < p.max_it {
            si.si_gradient = min_max_clip_15bit(self.it.smooth_it * 32767.0 / p.max_it as f32);
            si.normal[1] = 5000;
        } else {
            si.si_gradient = si.otrap.wrapping_add(32768);
        }
        si
    }

    /// `RMdoColor` – the orbit trap / 2nd colour choice.
    pub(crate) fn do_color(&self, si: &mut SiLight) {
        let it = &self.it;
        let [x, y, z, _] = it.v;
        let c = it.c;
        let pi = std::f64::consts::PI;
        let s: f64 = match self.p.color_option {
            1 => (it.rout / (it.rold + 1.0)).ln() * self.p.mct_color_mul as f64,
            2 => ((y - c[1]).atan2(x - c[0]) + pi) * 5200.0,
            3 => ((z - c[2]).atan2(x - c[0]) + pi) * 5200.0,
            4 => ((z - c[2]).atan2(y - c[1]) + pi) * 5200.0,
            5 => {
                let s = (x.atan2(y) + pi) * 5215.0;
                let r = (x * x + y * y + 1e-100 + z * z).sqrt();
                let s2 = (pi + arcsin_safe(z / r) * 2.0) * 5215.0;
                si.si_gradient = min_max_clip_15bit(s2 as f32);
                s
            }
            _ => it.otrap * 4096.0,
        };
        si.otrap = min_max_clip_15bit(s as f32);
    }

    /// `RMmaxLengthToCutPlane`: distance (in steps) from the start position
    /// to the cutting planes along the view ray, and the plane (1..3) hit.
    pub(crate) fn max_length_to_cut_plane(&self) -> (f64, usize) {
        let p = self.p;
        let mut len = 0f64;
        let mut plane = 0;
        for k in 0..3 {
            if (p.cut_options >> k) & 1 == 0 || self.vfov[k].abs() <= 1e-20 {
                continue;
            }
            let side = (p.cut_options >> (4 + k)) & 1 != 0;
            let neg = self.vfov[k].is_sign_negative();
            if side == neg {
                len = 1e20;
            } else {
                let t = (p.cut_pos[k] - self.it.c[k]) / self.vfov[k];
                if t > len {
                    len = t;
                    plane = k + 1;
                }
            }
        }
        (len, plane)
    }

    /// `RMcalcNanglesForCut`: z position and normal of a pixel that starts
    /// inside the set, on the cutting plane `cut_plane` (1..3) or the start
    /// plane (0).
    fn calc_nangles_for_cut(&self, si: &mut SiLight, cut_plane: usize) {
        let p = self.p;
        if cut_plane > 0 {
            let nn = 8388352.0 - p.zc_mul * ((self.mzz * p.zcorr + 1.0).sqrt() - 1.0);
            si.zpos_fine = ((nn.round_ties_even().max(0.0) as i64) << 8) as u32;
            let k = cut_plane - 1;
            let vg = &p.vgrads;
            let nn = if vg[2][k].abs() < 1e-40 { -1e40 } else { -1.0 / vg[2][k] };
            let n = [vg[0][k] * nn, vg[1][k] * nn, -1.0];
            let l = 32767.0 / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2] + 1e-100).sqrt();
            si.normal = [
                (n[0] * l).round_ties_even() as i16,
                (n[1] * l).round_ties_even() as i16,
                (n[2] * l).round_ties_even() as i16,
            ];
        } else {
            si.zpos_fine = 0x7FFF_0000;
            si.normal = [0, 0, -32767];
        }
    }

    /// `AtCutPlane2`: is the position closer than `plus_d` to a cutting plane?
    pub fn at_cut_plane(&self, plus_d: f64) -> bool {
        let p = self.p;
        (0..3).any(|k| (p.cut_options >> k) & 1 != 0 && (self.it.c[k] - p.cut_pos[k]).abs() - plus_d < 0.0)
    }

    /// Surface position of a calculated pixel as used by the post passes
    /// (DEAO): `steps` binary search steps down to `|DE - DEstop| <= thr`.
    /// Returns false for pixels without a surface.
    pub fn surface_point(&mut self, x: i32, y: i32, si: &SiLight, steps: i32, thr: f64) -> bool {
        let p = self.p;
        if si.zpos() >= 32768 || si.si_gradient >= 32768 {
            return false;
        }
        self.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
        self.calc_vgrads_fov(x + 1);
        self.calc_start_pos(x, y);
        self.it.calc_sit = false;
        let zf = (si.zpos_fine >> 8) as f64;
        self.mzz = (((8388351.5 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr;
        let v = self.vfov;
        add_weight(&mut self.it.c, &v, self.mzz);
        self.update_de_stop();
        let mut d = self.calc_de().min(self.ms_de_stop as f64 * 2.0);
        if self.it.it_result < self.max_its_result || d < self.ms_de_stop as f64 {
            let mut dt1 = (((8388351.9 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr - self.mzz;
            let mut itmp = steps;
            while itmp > 0 && (d - self.ms_de_stop as f64).abs() > thr {
                self.mzz += dt1;
                add_weight(&mut self.it.c, &v, dt1);
                self.update_de_stop();
                itmp -= 1;
                if itmp > 0 {
                    d = self.calc_de();
                    dt1 = if d < self.ms_de_stop as f64 { dt1.abs() * -0.55 } else { dt1.abs() * 0.55 };
                }
            }
        } else {
            self.bin_search_it();
            self.ms_de_stop = (p.de_stop as f64 * (1.0 + self.mzz.abs() * p.de_stop_factor as f64)) as f32;
        }
        true
    }

    /// `CalcZposAndRough`
    fn calc_zpos_and_rough(&self, si: &mut SiLight) {
        let p = self.p;
        let zz = self.mzz.max(0.0);
        let v = (p.zc_mul * ((zz * p.zcorr + 1.0).sqrt() - 1.0)).round_ties_even() as i64;
        let itmp = (8388352 - v).clamp(0, 8388352) as u32;
        let mut r = itmp << 8;
        if p.sm_normals > 0 {
            r |= ((self.s_roughness * 255.0).round_ties_even() as u32) & 0xFF;
        }
        si.zpos_fine = r;
    }

    /// One pixel of `TMandCalcThread.Execute`.
    pub fn march_pixel(&mut self, x: i32, y: i32) -> SiLight {
        let p = self.p;
        let mut si = SiLight::default();
        si.amb_shadow = 5000; // MB3D's value when no ambient shadow is calculated
        self.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
        self.calc_vgrads_fov(x + 1);
        self.calc_start_pos(x, y);
        self.it.calc_sit = false;
        let step_count = 0f32;
        self.mzz = 0.0;
        self.ms_de_stop = p.de_stop;
        let first_step = p.first_step_random;

        // move to the begin of the cutting planes
        let mut cut_plane = 0usize;
        let mut dtmp;
        if p.cut_options != 0 {
            let (dt1, cp) = self.max_length_to_cut_plane();
            cut_plane = cp;
            if dt1 > p.zend - self.mzz {
                return si; // background
            }
            self.mzz = dt1;
            let v = self.vfov;
            add_weight(&mut self.it.c, &v, dt1);
            self.update_de_stop();
        }
        self.inside = p.inside_rendering;
        self.calc_inside = p.inside_rendering;
        dtmp = self.calc_de();
        if (self.it.it_result >= self.max_its_result || dtmp < self.ms_de_stop as f64)
            && p.in_and_outside
            && self.calc_inside == p.inside_rendering
        {
            // in and outside: the start is in the set, so render from inside
            self.calc_inside = !p.inside_rendering;
            self.inside = !p.inside_rendering;
            dtmp = self.calc_de();
        }
        if self.it.it_result >= self.max_its_result || dtmp < self.ms_de_stop as f64 {
            // already inside the set at the start plane (or the cutting plane)
            if p.color_on_it != 0 {
                self.do_color_on_it();
            }
            self.calc_nangles_for_cut(&mut si, cut_plane);
            self.do_color(&mut si);
            if p.color_option > 4 {
                si.si_gradient |= 32768;
            } else if self.inside {
                self.it.calc_sit = true;
                self.calc_de();
                self.it.calc_sit = false;
                si.si_gradient = 32768 | min_max_clip_15bit(self.it.smooth_it * p.mcts_m);
            } else {
                let t = (self.it.rout / p.d_rstop).clamp(0.0, 1.0);
                si.si_gradient = 32768 + (32767.0 * t).round_ties_even() as u16;
            }
            return si;
        }
        self.march_from(si, dtmp, first_step, step_count)
    }

    /// The pixel from the card's single precision march (`card`, its
    /// G-buffer entry) finished in double precision: at the card's surface
    /// (its depth), as [`Marcher::march_pixel`] has it when it gets there:
    /// the binary search (within the depth's last digit), the normal,
    /// roughness, gradient and colour, which single precision gets wrong
    /// where it finds the same surfaces.  The step count (the dynamic fog)
    /// stays the card's.  A background pixel stays as it is, one that
    /// started in the set (or on a cutting plane) is calculated here.
    pub fn finish_pixel(&mut self, x: i32, y: i32, card: &SiLight) -> SiLight {
        let p = self.p;
        if card.zpos() >= 32768 {
            return *card;
        }
        if card.si_gradient >= 32768 {
            return self.march_pixel(x, y);
        }
        self.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
        self.calc_vgrads_fov(x + 1);
        self.calc_start_pos(x, y);
        self.it.calc_sit = false;
        // (as `hs_surface_start`: the middle of the depth's last digit, the
        // binary search over that digit)
        let zf = (card.zpos_fine >> 8) as f64;
        self.mzz = (((8388351.5 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr;
        let v = self.vfov;
        add_weight(&mut self.it.c, &v, self.mzz);
        self.update_de_stop();
        self.inside = if p.in_and_outside { card.otrap & 0x8000 == 0 } else { p.inside_rendering };
        self.calc_inside = self.inside;
        let dtmp = self.calc_de();
        let digit = (((8388351.9 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr - self.mzz;
        let mut si = SiLight { amb_shadow: 5000, shadow: card.shadow, ..SiLight::default() };
        self.surface_found(&mut si, dtmp, digit, false);
        si
    }

    /// The surface found at the current position (the DE there `dtmp`, the
    /// step that reached it `last_step`): its binary search, normal,
    /// gradient, colour, depth and roughness into `si`.  `count`: the
    /// iterations go into the statistics (`ITS`).
    fn surface_found(&mut self, si: &mut SiLight, mut dtmp: f64, last_step: f64, count: bool) {
        let p = self.p;
        let de_limited = self.it.it_result < self.max_its_result || dtmp < self.ms_de_stop as f64;
        if p.de_add_steps != 0 {
            if de_limited {
                self.bin_search(&mut dtmp, last_step);
            } else {
                self.bin_search_it();
            }
        }
        if count {
            let n = self.it.it_result.max(0);
            ITS.with(|c| {
                let (s, k, m) = c.get();
                c.set((s + n as u64, k + 1, m.max(n)));
            });
        }
        let nn = if p.normals_on_de { self.calculate_normals(si) } else { self.calculate_normals_on_smooth_it(si) };
        let v = if de_limited {
            32767.0 - (nn + p.d_col_plus + p.col_var_de_stop_mul * ((f32::max(p.de_stop, self.ms_de_stop) as f64 * p.step_width).ln() as f32)) * p.mcts_m
        } else {
            32767.0 - nn * p.mcts_m
        };
        si.si_gradient = min_max_clip_15bit(v);
        if p.color_on_it != 0 {
            self.do_color_on_it();
        }
        self.do_color(si);
        self.calc_zpos_and_rough(si);
        if p.in_and_outside && !self.inside {
            si.otrap |= 0x8000;
        }
    }

    /// The march of [`Marcher::march_pixel`] from the current position (the
    /// DE there `dtmp`) to the surface or the background, and the surface's
    /// values.
    fn march_from(&mut self, mut si: SiLight, mut dtmp: f64, mut first_step: bool, mut step_count: f32) -> SiLight {
        let p = self.p;
        let mut rsf_mul = 1f32;
        let mut last_step = dtmp * p.z_step_div as f64;
        let mut last_de;
        let mut guard = 0u32;
        loop {
            guard += 1;
            if guard > 2_000_000 {
                break;
            }
            if self.it.it_result >= self.max_its_result {
                let dt1 = -0.5 * last_step;
                self.mzz += dt1;
                self.step(dt1);
                self.update_de_stop();
                dtmp = self.calc_de();
                last_step = -dt1;
            }
            if self.it.it_result < p.min_it
                || (self.it.it_result < self.max_its_result && dtmp >= self.ms_de_stop as f64)
            {
                // ##### next step #####
                last_de = dtmp;
                // DoDynFog
                let mut d = f64::max(
                    0.11,
                    (dtmp - p.ms_de_sub as f64 * self.ms_de_stop as f64)
                        * p.z_step_div as f64
                        * rsf_mul as f64,
                );
                let s1 = (f32::max(self.ms_de_stop, 0.4) * p.mh04zsd) as f64;
                let count = p.dfog_on_it == 0 || self.it.it_result == p.dfog_on_it as i32;
                if let Some(vl) = &p.vol {
                    if s1 < d {
                        d = s1;
                    }
                    vl.integrate(p, &self.it.c, &self.vfov, last_step, self.mzz, &mut step_count);
                } else if s1 < d {
                    if count {
                        step_count += (s1 / d) as f32;
                    }
                    d = s1;
                } else if count {
                    step_count += 1.0;
                }
                if first_step {
                    first_step = false;
                    self.seed = self.seed.wrapping_mul(214013).wrapping_add(2531011);
                    d *= (self.seed & 0x7FFF_FFFF) as f64 * (1.0 / 0x7FFF_FFFF as f64);
                }
                self.mzz += d;
                if self.mzz > p.zend {
                    // background
                    if let Some(vl) = &p.vol {
                        let last = p.zend - self.mzz + d;
                        self.step(last);
                        vl.integrate(p, &self.it.c, &self.vfov, last, self.mzz, &mut step_count);
                    }
                    break;
                }
                last_step = d;
                self.step(d);
                self.update_de_stop();
                dtmp = self.calc_de();
                if dtmp > last_de + last_step {
                    dtmp = last_de + last_step;
                }
                rsf_mul = if last_de > dtmp + 1e-30 {
                    let t = last_step / (last_de - dtmp);
                    if t < 1.0 {
                        f64::max(0.5, t) as f32
                    } else {
                        1.0
                    }
                } else {
                    1.0
                };
            } else {
                // ##### surface found #####
                self.surface_found(&mut si, dtmp, last_step, true);
                break;
            }
        }
        if self.inside {
            step_count *= 200.0 * p.de_stop / p.width as f32;
        }
        si.shadow = if p.vol.is_some() {
            crate::lighting::encode_vlight(step_count)
        } else {
            step_count.clamp(0.0, 1023.0).round_ties_even() as u16
        };
        si
    }
}

/// The ray of a pixel: its start position and its direction per step
/// (`RMCalculateVgradsFOV` / `RMCalculateStartPos`), for tools.
pub fn pixel_ray(p: &CalcParams, x: i32, y: i32) -> (Vec3, Vec3) {
    let mut m = Marcher::new(p, 1);
    m.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
    m.calc_vgrads_fov(x + 1);
    m.calc_start_pos(x, y);
    (m.it.c, m.vfov)
}

/// Evaluate the DE at an arbitrary world position (useful for tests/tools).
pub fn de_at(p: &CalcParams, pos: Vec3) -> (f64, i32) {
    let mut m = Marcher::new(p, 1);
    m.it.c = pos;
    let d = m.calc_de();
    (d * p.step_width, m.it.it_result)
}

/// One light for the hard shadow pass: header light index and `HSvecs[i]`
/// (world space direction towards the light, scaled to -StepWidth).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HsLight {
    pub idx: usize,
    pub vec: Vec3,
    /// a positional light: its world position and the squared distance
    /// beyond which it casts no shadow (`sLmaxL * sHSmaxLengthMultiplier`);
    /// `vec` is then taken from the position at each pixel
    pub pos: Option<(Vec3, f64)>,
    /// `iLightPos and 14` (the soft shadow's scale of a positional light)
    pub visible: u8,
}

impl HsLight {
    /// `HSvecs[i]` at the surface point `c` and the march length `max_l`
    /// (shortened to a positional light's distance); `None`: a positional
    /// light too far away
    fn at(&self, c: &Vec3, max_l: f64, step_width: f64) -> Option<(Vec3, f64)> {
        let Some((pos, lmax)) = self.pos else { return Some((self.vec, max_l)) };
        let v = [pos[0] - c[0], pos[1] - c[1], pos[2] - c[2]];
        let d = sqr_len(&v);
        if d > lmax {
            return None;
        }
        let max_l = if d < (max_l * step_width).powi(2) { d.sqrt() / step_width } else { max_l };
        let s = -step_width / d.sqrt().max(1e-300);
        Some(([v[0] * s, v[1] * s, v[2] * s], max_l))
    }
}

#[inline]
fn dot_normalize(a: &Vec3, b: &Vec3) -> f64 {
    dot(a, b) / (sqr_len(a) * sqr_len(b)).sqrt().max(1e-300)
}

impl<'a> Marcher<'a> {
    /// Position on the surface for a calculated pixel (common start of the
    /// hard shadow passes): returns false for background / inside pixels.
    fn hs_surface_start(&mut self, x: i32, y: i32, si: &SiLight) -> bool {
        let p = self.p;
        if si.zpos() >= 32768 || si.si_gradient >= 32768 {
            return false;
        }
        self.cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
        self.calc_vgrads_fov(x + 1);
        self.calc_start_pos(x, y);
        let zf = (si.zpos_fine >> 8) as f64;
        self.mzz = (((8388351.5 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr;
        let v = self.vfov;
        add_weight(&mut self.it.c, &v, self.mzz);
        self.update_de_stop();
        let mut d = self.calc_de();
        let de_limited = self.it.it_result < self.max_its_result || d < self.ms_de_stop as f64;
        if de_limited {
            let dt1 = (((8388351.9 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr - self.mzz;
            self.bin_search(&mut d, dt1);
        } else {
            self.bin_search_it();
        }
        true
    }

    /// `MaxLHS`
    fn hs_max_len(&self, y: i32, mul: f32) -> f64 {
        let p = self.p;
        ((p.width + y) as f32
            * 0.6
            * (1.0 + 0.5 * self.mzz.min(p.zend * 0.4) as f32 * p.fov_y.max(0.0) as f32 / p.height as f32)
            * mul) as f64
    }

    /// `THardShadowCalcThread.Execute` for one row: sets the shadow bits
    /// ($400 shl light index) of `si.shadow` for the given lights.
    pub fn hard_shadow_row(&mut self, y: i32, x0: i32, row: &mut [SiLight], lights: &[HsLight], max_len_mul: f32) {
        let p = self.p;
        let mut b_zz_oathr = [0i32; 6];
        let mut d_last_found = [0f64; 6];
        let mut d_last_z = 0f64;
        let mut clear_mask: u16 = 0;
        for l in lights {
            clear_mask |= 0x400 << l.idx;
        }
        for (x, si) in row.iter_mut().enumerate() {
            let x = x as i32 + x0;
            for v in b_zz_oathr.iter_mut() {
                if *v > 0 {
                    *v -= 1;
                }
            }
            si.shadow &= !clear_mask;
            let mut nvec = [si.normal[0] as f64 / 32767.0, si.normal[1] as f64 / 32767.0, si.normal[2] as f64 / 32767.0];
            nvec = rotate_vector_reverse(&nvec, &p.vgrads);
            if !self.hs_surface_start(x, y, si) {
                continue;
            }
            self.mzz = (self.mzz - 0.1).max(0.0);
            let v = self.vfov;
            add_weight(&mut self.it.c, &v, -0.1);
            self.update_de_stop();
            let ic = self.it.c;
            let max_lhs = self.hs_max_len(y, max_len_mul);
            let oa_thr = (1.0 + (self.mzz + max_lhs) * p.de_stop_factor as f64) * 3.3 / p.z_step_div as f64;
            for l in lights {
                let it = l.idx;
                let mut b_thr = b_zz_oathr[it] > 0;
                if b_thr {
                    b_zz_oathr[it] -= ((d_last_z - self.mzz).abs() * 3.3 / oa_thr).round_ties_even() as i32;
                    if b_zz_oathr[it] <= 0 {
                        b_thr = false;
                        b_zz_oathr[it] = 0;
                    } else {
                        d_last_found[it] -= (d_last_z - self.mzz).abs() + (1.0 + self.mzz * p.de_stop_factor as f64);
                        if d_last_found[it] <= 0.0 {
                            b_thr = false;
                            b_zz_oathr[it] = 0;
                        }
                    }
                }
                self.it.c = ic;
                let mut zz2 = self.mzz;
                // (a positional light too far away: in shadow)
                let Some((lvec, max_l)) = l.at(&ic, max_lhs, p.step_width) else {
                    si.shadow |= 0x400 << it;
                    continue;
                };
                let zz2mul = -dot_normalize(&lvec, &self.vfov);
                if dot(&nvec, &lvec) > 0.0 {
                    si.shadow |= 0x400 << it;
                    b_zz_oathr[it] = 0;
                    continue;
                }
                let mut dt1 = max_l;
                if dt1 > 0.0 {
                    self.ms_de_stop = (p.de_stop as f64 * (1.0 + zz2.abs() * p.de_stop_factor as f64)) as f32;
                    let mut rsf = 2f32;
                    let mut d = self.calc_de();
                    let mut guard = 0;
                    loop {
                        guard += 1;
                        if guard > 1_000_000 {
                            break;
                        }
                        if b_thr && d_last_found[it] > dt1 {
                            dt1 = -1.0; // open air
                            break;
                        }
                        let last_de = d;
                        let st = f32::min(
                            f32::max(0.11, ((d - p.ms_de_sub as f64 * self.ms_de_stop as f64) * p.z_step_div as f64 * rsf as f64) as f32),
                            f32::max(self.ms_de_stop, 0.4) * p.mh04zsd,
                        ) as f64;
                        dt1 -= st;
                        add_weight(&mut self.it.c, &lvec, -st);
                        zz2 += st * zz2mul;
                        self.ms_de_stop = (p.de_stop as f64 * (1.0 + zz2.abs() * p.de_stop_factor as f64)) as f32;
                        d = self.calc_de();
                        if self.it.it_result >= self.max_its_result || d <= self.ms_de_stop as f64 {
                            break;
                        }
                        if d > last_de + st {
                            d = last_de + st;
                        }
                        if !b_thr {
                            if d > oa_thr {
                                if b_zz_oathr[it] < 3 {
                                    d_last_found[it] = dt1;
                                    b_zz_oathr[it] = 3;
                                }
                            } else {
                                b_zz_oathr[it] = 0;
                            }
                        }
                        rsf = if last_de > d + 1e-30 {
                            let t = (st / (last_de - d)) as f32;
                            if t < 1.0 {
                                t.max(0.5)
                            } else {
                                1.0
                            }
                        } else {
                            1.0
                        };
                        if dt1 < 0.0 {
                            break;
                        }
                    }
                }
                if dt1 > 0.0 {
                    si.shadow |= 0x400 << it; // in shadow
                    b_zz_oathr[it] = 0;
                }
            }
            d_last_z = self.mzz;
        }
    }

    /// `calcHSsoft`: one light with a soft shadow value (0..63) in bits 10..15.
    pub fn soft_shadow_row(&mut self, y: i32, x0: i32, row: &mut [SiLight], light: &HsLight, soft_radius: f32, max_len_mul: f32) {
        let p = self.p;
        let zrs_mul = match light.pos {
            // (a positional light scales with its distance)
            Some(_) if light.visible & 6 == 2 => 70.0,
            Some(_) => 40.0,
            None => 80.0,
        } / soft_radius.max(0.001);
        for (x, si) in row.iter_mut().enumerate() {
            si.shadow |= 0xFC00;
            let mut nvec = [si.normal[0] as f64 / 32767.0, si.normal[1] as f64 / 32767.0, si.normal[2] as f64 / 32767.0];
            nvec = rotate_vector_reverse(&nvec, &p.vgrads);
            if !self.hs_surface_start(x as i32 + x0, y, si) {
                continue;
            }
            self.mzz -= 0.1;
            let v = self.vfov;
            add_weight(&mut self.it.c, &v, -0.1);
            let max_lhs = self.hs_max_len(y, max_len_mul);
            let mut zz2 = self.mzz;
            self.update_de_stop();
            // (a positional light too far away: no shadow)
            let Some((lvec, d_max_l)) = light.at(&self.it.c, max_lhs, p.step_width) else { continue };
            let zz2mul = -dot_normalize(&lvec, &self.vfov);
            if dot(&nvec, &lvec) > 0.0 {
                si.shadow &= 0x3FF;
                continue;
            }
            let mut zr_soft = 1f32;
            let mut dt1 = d_max_l;
            if dt1 > 0.0 {
                self.ms_de_stop = (p.de_stop as f64 * (1.0 + zz2.abs() * p.de_stop_factor as f64)) as f32;
                let mut rsf = 2f32;
                let mut d = self.calc_de();
                let mut guard = 0;
                loop {
                    guard += 1;
                    if guard > 1_000_000 {
                        break;
                    }
                    let last_de = d;
                    let st = f32::min(
                        f32::max(0.11, ((d - p.ms_de_sub as f64 * self.ms_de_stop as f64) * p.z_step_div as f64 * rsf as f64) as f32),
                        f32::max(self.ms_de_stop, 0.4) * p.mh04zsd,
                    ) as f64;
                    dt1 -= st;
                    add_weight(&mut self.it.c, &lvec, -st);
                    zz2 += st * zz2mul;
                    self.ms_de_stop = (p.de_stop as f64 * (1.0 + zz2.abs() * p.de_stop_factor as f64)) as f32;
                    d = self.calc_de();
                    let r = (d_max_l - dt1) / max_lhs;
                    let r8 = (r * r) * (r * r);
                    zr_soft = zr_soft.min(
                        ((d - self.ms_de_stop as f64) * zrs_mul as f64 / (d_max_l - dt1 + 0.11) + r8 * r8) as f32,
                    );
                    if self.it.it_result >= self.max_its_result || d <= self.ms_de_stop as f64 {
                        break;
                    }
                    if d > last_de + st {
                        d = last_de + st;
                    }
                    rsf = if last_de > d + 1e-30 {
                        let t = (st / (last_de - d)) as f32;
                        if t < 1.0 {
                            t.max(0.5)
                        } else {
                            1.0
                        }
                    } else {
                        1.0
                    };
                    if dt1 < 0.0 {
                        break;
                    }
                }
            }
            si.shadow = (si.shadow & 0x3FF) | (((zr_soft.clamp(0.0, 1.0) * 63.4).round_ties_even() as u16) << 10);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The jump-ahead equals stepping the generator x times.
    #[test]
    fn pixel_seed_steps_along_the_row() {
        for y in [0i64, 7, 1499] {
            let mut s = (0x24563487u32 as i64 + (y + 1) * 0x324594A1i64) as i32;
            for x in 0..3000i64 {
                assert_eq!(pixel_seed(y, x), s, "y {y} x {x}");
                s = s.wrapping_mul(214013).wrapping_add(2531011);
            }
        }
    }
}
