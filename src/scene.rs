//! Scene parameters – the subset of MB3D's `TMandHeader10` (+ the formula
//! addon `THeaderCustomAddon` and the lighting record) that the phase-1 renderer
//! understands.  Default values follow MB3D's start-up state.
//!
//! Scenes can be loaded from a small INI-style text format:
//!
//! ```text
//! width = 800
//! height = 600
//! iterations = 12
//! zoom = 1.0
//! [formula]
//! name = Integer Power
//! iterations = 1
//! power = 8
//! ```

use crate::formulas::Formula;
use crate::lighting::Lighting;
use crate::math::{vgrads_from_angles, Mat3};

#[derive(Clone, Debug)]
pub struct FormulaEntry {
    pub formula: Formula,
    /// iteration count of this hybrid slot (`iItCount`); 0 disables the slot
    pub iterations: i32,
}

/// Camera optic (`bPlanarOptic`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraOptic {
    /// 0: common perspective
    Common = 0,
    /// 1: rectilinear (planar)
    Planar = 1,
    /// 2: spherical panorama
    Panorama = 2,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    /// Max iterations (`Iterations`)
    pub iterations: i32,
    /// `MinimumIterations`
    pub min_iterations: i32,
    /// `bColorOnIt`: 0 off, 1 colour on the start vector, n > 1 colour
    /// after n - 1 iterations
    pub color_on_it: u8,
    /// 2D calculation of a plane (`bCalc3D` = 0, `bSliceCalc`):
    /// 1 at Z start, 2 at the middle, 3 at Z end; 0 = 3D
    pub slice_2d: u8,
    /// Stereo image (`bStereoMode`): 0 mono, 1 "very left from midpos",
    /// 3 right eye, 4 left eye
    pub stereo_mode: u8,
    /// Normals calculated from the z-buffer after the calculation
    /// (`byCalcNsOnZBufAuto`)
    pub normals_on_zbuf: bool,
    /// `StereoScreenWidth`, `StereoScreenDistance`, `StereoMinDistance`
    /// (real-world metres)
    pub stereo_screen: [f32; 3],
    /// Settings of the Monte Carlo renderer
    pub mc: McSettings,
    /// Bailout radius (`RStop`, not squared). `None` = use the formulas' default.
    pub rstop: Option<f64>,
    pub zoom: f64,
    /// `dXmid, dYmid, dZmid`
    pub mid: [f64; 3],
    /// `dZstart, dZend`: start plane and far end along the view axis.  Like in
    /// MB3D the camera plane is at `mid + (z_start - mid.z) * view_dir`.
    pub z_start: f64,
    pub z_end: f64,
    /// Camera matrix rows (`hVGrads`), length does not matter.
    pub vgrads: Mat3,
    /// Field of view in degrees (`dFOVy`)
    pub fov_y: f64,
    pub optic: CameraOptic,
    /// `sDEstop`
    pub de_stop: f64,
    /// Raystep multiplier (`mZstepDiv`, 0.001..1)
    pub z_step_div: f64,
    /// `sRaystepLimiter`
    pub raystep_limiter: f64,
    /// Smooth normals 0..8 (`iOptions shr 6`)
    pub smooth_normals: i32,
    /// Binary search steps after DE stop (`bStepsafterDEStop`)
    pub bin_search_steps: i32,
    /// `iOptions` bit 0
    pub first_step_random: bool,
    /// `iOptions` bit 2
    pub step_sub_de_stop: bool,
    /// `bVaryDEstopOnFOV`
    pub vary_de_stop_on_fov: bool,
    /// `bNormalsOnDE`
    pub normals_on_de: bool,
    /// Hybrid option "disable analytic DE" (`bOptions2` bit 0)
    pub disable_analytic_de: bool,
    pub julia: bool,
    pub julia_c: [f64; 4],
    /// 4D rotation angles in radians (`dXWrot, dYWrot, dZWrot`)
    pub rot_4d: [f64; 3],
    /// Mode for the 2nd color choice (`byColor2Option`, 0..5)
    pub color_option: u8,
    /// `sColorMul`
    pub color_mul: f64,
    /// Dynamic fog on iteration (`bDFogIt`), 0 = all
    pub dfog_on_it: u16,
    /// Hybrid formulas (up to 6 slots, `HAddon.Formulas`)
    pub formulas: Vec<FormulaEntry>,
    /// Alternating hybrid: repeat from slot (`bHybOpt1 shr 4`)
    pub repeat_from: usize,
    pub lighting: Lighting,
    /// Ambient occlusion (24 bit SSAO), None = not calculated
    pub ao: Option<crate::ssao::SsaoParams>,
    /// Hard / soft shadows, None = not calculated
    pub shadows: Option<ShadowParams>,
    /// Cutting planes: bits 0..2 = x, y, z plane on (`bCutOption`)
    pub cut_options: u8,
    /// Positions of the cutting planes (`dCutX`, `dCutY`, `dCutZ`)
    pub cut_pos: [f64; 3],
    /// DE ambient occlusion instead of SSAO (used when `ao` is on)
    pub deao: Option<crate::deao::DeaoParams>,
    /// Inside rendering (`bOptions2` bits 1..2)
    pub inside: InsideMode,
    /// DE combination of two hybrid parts, None = alternating hybrid
    pub decomb: Option<DeCombParams>,
    /// Interpolation hybrid (`bOptions1` = 1): weights of the first two
    /// formulas, which are both applied in each iteration and blended
    pub interpolation: Option<[f32; 2]>,
    /// Depth of field, None = off
    pub dof: Option<crate::dof::DofParams>,
    /// Volumetric light (`bVolLightNr`), None = off
    pub vol_light: Option<crate::vollight::VolLightParams>,
    /// Thread count, 0 = all cores
    pub threads: usize,
    /// Tiled rendering ("big renders", `TilingOptions`), None = one piece
    pub tiling: Option<Tiling>,
    /// Calculate only this part of the image (left, top, width, height);
    /// set by the tiled renderer, not stored in scene files.
    pub calc_rect: Option<[i32; 4]>,
    /// Light values of an animation frame: blended from the keyframes by
    /// the renderer (set by [`crate::anim`], not stored in scene files)
    pub light_blend: Option<std::sync::Arc<crate::anim::LightBlend>>,
    /// The statistics of the last calculation (MB3D's header fields
    /// `iAvrgDEsteps`, `iAvrgIts`, `iMaxIts`, shown in the Infos panel):
    /// kept in .m3p/.m3i files, not in scene files, and no calculation
    /// setting
    pub stats: CalcStats,
}

/// Statistics of a calculation as MB3D keeps them in its parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CalcStats {
    /// average ray steps of a pixel, times 10
    pub avg_de_steps10: i32,
    /// average iterations at the surface of the pixels that hit it, times 10
    pub avg_its10: i32,
    /// most iterations at a surface point (0: unknown)
    pub max_its: i32,
}

/// Names of the diffuse map mappings (`Lights[1].FreeByte`).
pub const DIFF_MAP_MODES: [&str; 4] = ["iterations_otrap", "normals", "wrap_sine", "wrap"];

/// Tiled rendering: the image is split into `cols` x `rows` tiles that are
/// calculated one after another (bounded memory, resumable, or spread over
/// several machines with `pos`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tiling {
    pub cols: u32,
    pub rows: u32,
    /// Only this tile (0-based column, row), None = all tiles
    pub pos: Option<(u32, u32)>,
    /// Downscale factor of the tiles (`brDownScale`, 1..3): the image is
    /// calculated that many times larger and reduced (anti-aliasing)
    pub downscale: u32,
}

impl Tiling {
    /// Interior rectangle (left, top, width, height) of a tile.
    pub fn tile_rect(&self, width: i32, height: i32, col: u32, row: u32) -> [i32; 4] {
        let (c, r) = (self.cols.max(1) as i64, self.rows.max(1) as i64);
        let x0 = (col as i64 * width as i64 / c) as i32;
        let x1 = ((col as i64 + 1) * width as i64 / c) as i32;
        let y0 = (row as i64 * height as i64 / r) as i32;
        let y1 = ((row as i64 + 1) * height as i64 / r) as i32;
        [x0, y0, x1 - x0, y1 - y0]
    }
}

/// Header values of MB3D's Monte Carlo renderer (`MonteCarloForm`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct McSettings {
    /// `MCDepth`: ambient (diffuse) bounces
    pub depth: u8,
    /// `SRreflectioncount`: reflection / transmission depth
    pub reflection_depth: u8,
    /// `SRamount`: amount of reflected light (0..1 realistic)
    pub reflection_amount: f32,
    /// `bCalcSRautomatic` bit 1: calculate reflections
    pub reflections: bool,
    /// bit 2: transparency (with reflections)
    pub transparency: bool,
    /// bit 3: transparency only for dIFS formulas
    pub only_difs: bool,
    /// `MCdiffReflects`: diffuse reflections 0..250 (shown as 0.00..2.50)
    pub diffuse_reflects: u8,
    /// `MCSoftShadowRadius`: size of the light sources
    pub soft_shadow_radius: f32,
    /// `MCcontrast`: exposure 0..255 (128 = 1)
    pub contrast: u8,
    /// `bMCSaturation`: 0..127 (32 = 1)
    pub saturation: u8,
    /// `MCoptions`: 1 soft clipping (HDR), 2 secant search, 4 clip the
    /// diffuse plus specular colours, 8 gaussian anti-aliasing,
    /// bits 4..6 bokeh shape
    pub options: u8,
    /// `sTRIndex`: refraction index of transparent surfaces
    pub refraction_index: f32,
    /// `sTransmissionAbsorption`
    pub absorption: f32,
    /// `sTRscattering`: light scattering inside transparent material
    pub scattering: f32,
}

impl Default for McSettings {
    fn default() -> Self {
        McSettings {
            depth: 3,
            reflection_depth: 1,
            reflection_amount: 0.5,
            reflections: false,
            transparency: false,
            only_difs: false,
            diffuse_reflects: 0,
            soft_shadow_radius: 1.0,
            contrast: 128,
            saturation: 32,
            options: 2,
            refraction_index: 1.5,
            absorption: 0.2,
            scattering: 1.0,
        }
    }
}

impl Default for Scene {
    fn default() -> Self {
        Scene {
            width: 800,
            height: 600,
            iterations: 60,
            min_iterations: 1,
            color_on_it: 0,
            slice_2d: 0,
            stereo_mode: 0,
            normals_on_zbuf: false,
            stereo_screen: [1.0, 2.0, 0.5],
            mc: McSettings::default(),
            rstop: None,
            zoom: 1.0,
            mid: [0.0; 3],
            z_start: -2.0,
            z_end: 30.0,
            vgrads: crate::math::IDENTITY3,
            fov_y: 30.0,
            optic: CameraOptic::Common,
            de_stop: 1.0,
            z_step_div: 0.5,
            raystep_limiter: 1.0,
            smooth_normals: 0,
            bin_search_steps: 8,
            first_step_random: true,
            step_sub_de_stop: false,
            vary_de_stop_on_fov: true,
            normals_on_de: true,
            disable_analytic_de: false,
            julia: false,
            julia_c: [0.0; 4],
            rot_4d: [0.0; 3],
            color_option: 0,
            color_mul: 1.0,
            dfog_on_it: 0,
            formulas: vec![FormulaEntry {
                formula: Formula::default_for("Integer Power").unwrap(),
                iterations: 1,
            }],
            repeat_from: 0,
            lighting: Lighting::default(),
            ao: Some(crate::ssao::SsaoParams::default()),
            shadows: None,
            vol_light: None,
            dof: None,
            decomb: None,
            interpolation: None,
            inside: InsideMode::Outside,
            cut_options: 0,
            cut_pos: [0.0; 3],
            deao: None,
            threads: 0,
            tiling: None,
            calc_rect: None,
            light_blend: None,
            stats: CalcStats::default(),
        }
    }
}

impl Scene {
    /// Changes the image size by `f` and keeps the look of the image: the DE
    /// stop threshold is given in pixels, so it is scaled too (like rendering
    /// at full size and downsampling).
    pub fn scale_image(&mut self, f: f64) {
        let f = f.max(1e-4);
        let w = ((self.width as f64 * f).round() as i32).max(16);
        let h = ((self.height as f64 * f).round() as i32).max(16);
        let real = w as f64 / self.width as f64;
        self.width = w;
        self.height = h;
        self.de_stop = (self.de_stop * real).max(0.001);
        if let Some(d) = self.dof.as_mut() {
            d.clip_r = (d.clip_r * real as f32).max(0.1);
        }
    }

    /// `CalcStepWidth`
    pub fn step_width(&self) -> f64 {
        2.1345 / (self.zoom * self.width as f64)
    }

    /// `StereoChange`: the middle moved sideways for the eye of
    /// `stereo_mode` (used for the calculation and the lights).
    pub fn stereo_mid(&self) -> [f64; 3] {
        let [sw, sd, md] = self.stereo_screen.map(|v| v as f64);
        let eyedist = 0.065 * self.width as f64 * sd / f64::max(1e-100, md * sw);
        let k = (sd - md) / sd;
        let xadd = match self.stereo_mode {
            1 => -eyedist * k,
            3 => 0.5 * eyedist * k,
            4 => -0.5 * eyedist * k,
            _ => return self.mid,
        };
        let r = self.vgrads[0];
        let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-300);
        let s = self.step_width() / l * xadd;
        [self.mid[0] + r[0] * s, self.mid[1] + r[1] * s, self.mid[2] + r[2] * s]
    }

    /// `CalcXoff`: horizontal position of the view centre (0.5 = middle),
    /// the off-axis projection of the stereo images.
    pub fn stereo_xoff(&self) -> f32 {
        let xoff = -0.065 / f64::max(1e-100, self.stereo_screen[0] as f64);
        let r = match self.stereo_mode {
            1 => 0.5 + xoff,
            3 => 0.5 * (1.0 - xoff),
            4 => 0.5 * (1.0 + xoff),
            _ => 0.5,
        };
        r as f32
    }

    /// The effective bailout radius.
    pub fn effective_rstop(&self) -> f64 {
        self.rstop.unwrap_or_else(|| {
            self.formulas
                .iter()
                .filter(|f| f.iterations > 0)
                .map(|f| f.formula.default_rstop())
                .fold(0.0, f64::max)
                .max(1.0001)
        })
    }

    /// Convenience: a scene for one built-in formula with a fitting camera.
    pub fn preset(name: &str) -> Result<Scene, String> {
        let formula = crate::formulas::lookup(name)?;
        let mut s = Scene::default();
        // camera framing and colour range (color_start/color_end are MB3D's
        // TBpos[9]/[10] and select the smoothed-iteration window of the palette)
        let (zs, zoom, its, rot, cs, ce) = match formula {
            Formula::AmazingBox { .. } => (-14.0, 0.18, 20, (0.35, -0.6), 44.0, 52.0),
            Formula::Bulbox { .. } => (-14.0, 0.18, 20, (0.35, -0.6), 38.0, 45.0),
            Formula::Quaternion { .. } => {
                s.julia = true;
                s.julia_c = [-0.2, 0.6, 0.2, 0.0];
                (-3.0, 0.8, 12, (0.4, 0.3), 33.0, 59.0)
            }
            Formula::FoldingIntPow { .. } => (-20.0, 0.15, 20, (0.5, 0.4), 61.0, 69.0),
            Formula::Tricorn { .. } => (-3.0, 0.6, 12, (0.6, 0.2), 35.0, 55.0),
            Formula::AexionC { .. } => (-3.0, 0.65, 10, (0.45, 0.3), 40.0, 60.0),
            _ => (-2.5, 0.8, 12, (-0.55, 0.45), 68.0, 72.0),
        };
        s.z_start = zs;
        s.zoom = zoom;
        s.iterations = its;
        s.vgrads = vgrads_from_angles(rot.0, rot.1, 0.0);
        s.lighting.color_start = cs;
        s.lighting.color_end = ce;
        s.formulas = vec![FormulaEntry { formula, iterations: 1 }];
        Ok(s)
    }

    /// Serialise to the text format (round-trips through [`Scene::parse`]).
    pub fn to_text(&self) -> String {
        use std::fmt::Write;
        let hex = |c: [u8; 3]| format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]);
        let m = self.vgrads;
        let l = &self.lighting;
        let mut t = String::new();
        let _ = writeln!(t, "# mb3d scene");
        let _ = writeln!(t, "width = {}\nheight = {}", self.width, self.height);
        if let Some(tl) = &self.tiling {
            let _ = writeln!(t, "tiles = {}x{}", tl.cols, tl.rows);
            if let Some((c, r)) = tl.pos {
                let _ = writeln!(t, "tile = {}, {}", c + 1, r + 1);
            }
            if tl.downscale > 1 {
                let _ = writeln!(t, "tile_downscale = {}", tl.downscale);
            }
        }
        let _ = writeln!(t, "iterations = {}\nmin_iterations = {}", self.iterations, self.min_iterations);
        if self.color_on_it != 0 {
            let _ = writeln!(t, "color_on_iteration = {}", self.color_on_it as i32 - 1);
        }
        if self.normals_on_zbuf {
            let _ = writeln!(t, "normals_on_zbuf = true");
        }
        if self.stereo_mode != 0 {
            let _ = writeln!(t, "stereo = {}", match self.stereo_mode {
                1 => "very_left",
                3 => "right",
                _ => "left",
            });
        }
        if self.stereo_mode != 0 || self.stereo_screen != [1.0, 2.0, 0.5] {
            let [a, b, c] = self.stereo_screen;
            let _ = writeln!(t, "stereo_screen = {a}, {b}, {c}");
        }
        if self.mc != McSettings::default() {
            let m = &self.mc;
            let _ = writeln!(
                t,
                "mc_depth = {}\nmc_reflection_depth = {}\nmc_reflection_amount = {}\nmc_reflections = {}\nmc_transparency = {}\n\
                 mc_only_difs = {}\nmc_diffuse_reflects = {}\nmc_soft_shadow_radius = {}\nmc_exposure = {}\nmc_saturation = {}\n\
                 mc_options = {}\nmc_refraction_index = {}\nmc_absorption = {}\nmc_scattering = {}",
                m.depth,
                m.reflection_depth,
                m.reflection_amount,
                m.reflections,
                m.transparency,
                m.only_difs,
                m.diffuse_reflects as f32 / 100.0,
                m.soft_shadow_radius,
                m.contrast,
                m.saturation,
                m.options,
                m.refraction_index,
                m.absorption,
                m.scattering
            );
        }
        if self.slice_2d != 0 {
            let _ = writeln!(t, "slice_2d = {}", ["start", "mid", "end"][(self.slice_2d.clamp(1, 3) - 1) as usize]);
        }
        if let Some(r) = self.rstop {
            let _ = writeln!(t, "rstop = {r}");
        }
        let _ = writeln!(t, "zoom = {}", self.zoom);
        let _ = writeln!(t, "mid = {}, {}, {}", self.mid[0], self.mid[1], self.mid[2]);
        let _ = writeln!(t, "z_start = {}\nz_end = {}", self.z_start, self.z_end);
        let _ = writeln!(
            t,
            "vgrads = {}, {}, {}, {}, {}, {}, {}, {}, {}",
            m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2]
        );
        let _ = writeln!(t, "fov = {}\noptic = {}", self.fov_y, self.optic as i32);
        let _ = writeln!(t, "de_stop = {}\nraystep = {}\nraystep_limiter = {}", self.de_stop, self.z_step_div, self.raystep_limiter);
        let _ = writeln!(t, "smooth_normals = {}\nbin_search = {}", self.smooth_normals, self.bin_search_steps);
        let _ = writeln!(t, "first_step_random = {}\nstep_sub_de_stop = {}", self.first_step_random, self.step_sub_de_stop);
        let _ = writeln!(t, "vary_de_stop = {}\nnormals_on_de = {}", self.vary_de_stop_on_fov, self.normals_on_de);
        let _ = writeln!(t, "disable_analytic_de = {}", self.disable_analytic_de);
        let _ = writeln!(t, "julia = {}", self.julia);
        let _ = writeln!(t, "julia_c = {}, {}, {}, {}", self.julia_c[0], self.julia_c[1], self.julia_c[2], self.julia_c[3]);
        let _ = writeln!(
            t,
            "rotation_4d = {}, {}, {}",
            self.rot_4d[0].to_degrees(),
            self.rot_4d[1].to_degrees(),
            self.rot_4d[2].to_degrees()
        );
        let _ = writeln!(t, "color_option = {}\ncolor_mul = {}\ndfog_on_it = {}", self.color_option, self.color_mul, self.dfog_on_it);
        let _ = writeln!(t, "repeat_from = {}\nthreads = {}", self.repeat_from, self.threads);
        match &self.ao {
            None => {
                let _ = writeln!(t, "ao = off");
            }
            Some(a) => {
                let mode = if self.deao.is_some() {
                    "deao"
                } else {
                    match (a.bits15, a.t0) {
                        (true, true) => "ssao15t0",
                        (true, false) => "ssao15",
                        (false, true) => "ssao24t0",
                        (false, false) => "ssao24",
                    }
                };
                let _ = writeln!(t, "ao = {mode}\nao_threshold = {}\nao_border = {}", a.threshold, a.border_mirror);
                if a.random > 0 {
                    let _ = writeln!(t, "ao_random = {}", a.random);
                }
            }
        }
        for (k, n) in ["cut_x", "cut_y", "cut_z"].iter().enumerate() {
            if (self.cut_options >> k) & 1 != 0 {
                let _ = writeln!(t, "{n} = {}", self.cut_pos[k]);
            }
        }
        if let Some(d) = &self.deao {
            let _ = writeln!(
                t,
                "deao_quality = {}\ndeao_dither = {}\ndeao_max_len = {}\ndeao_random = {}",
                d.quality, d.dither, d.max_len, d.first_step_random
            );
        }
        if self.inside != InsideMode::Outside {
            let _ = writeln!(t, "inside = {}", if self.inside == InsideMode::Inside { "inside" } else { "both" });
        }
        if let Some(w) = self.interpolation {
            let _ = writeln!(t, "interpolation = {}, {}", w[0], w[1]);
        }
        if let Some(d) = &self.decomb {
            let _ = writeln!(
                t,
                "de_combination = {}\ndecomb_end1 = {}\ndecomb_start2 = {}\ndecomb_end2 = {}\ndecomb_repeat2 = {}\n\
                 decomb_iterations2 = {}\ndecomb_smooth = {}\ndecomb_mix_pow = {}\ndecomb_mix_color = {}",
                DECOMB_NAMES[(d.kind.clamp(1, 6) - 1) as usize],
                d.end1 + 1,
                d.start2 + 1,
                d.end2 + 1,
                d.repeat2 + 1,
                d.iterations2,
                d.smooth,
                d.mix_pow,
                d.mix_color
            );
        }
        match &self.dof {
            None => {
                let _ = writeln!(t, "dof = off");
            }
            Some(d) => {
                let _ = writeln!(
                    t,
                    "dof = {}\ndof_focus = {}\ndof_focus2 = {}\ndof_aperture = {}\ndof_max_radius = {}\ndof_passes = {}",
                    if d.forward { "forward" } else { "sorted" },
                    d.z_sharp,
                    d.z_sharp2,
                    d.aperture,
                    d.clip_r,
                    d.passes
                );
            }
        }
        match &self.vol_light {
            None => {
                let _ = writeln!(t, "vol_light = off");
            }
            Some(v) => {
                let _ = writeln!(t, "vol_light = {}\nvol_light_map_size = {}", v.light + 1, v.map_size);
            }
        }
        match &self.shadows {
            None => {
                let _ = writeln!(t, "shadows = off");
            }
            Some(h) => {
                let list: Vec<String> = (0..6).filter(|i| (h.lights >> i) & 1 != 0).map(|i| (i + 1).to_string()).collect();
                let _ = writeln!(
                    t,
                    "shadows = {}\nshadow_soft = {}\nshadow_soft_radius = {}\nshadow_max_len = {}\nshadow_set_cos = {}",
                    if list.is_empty() { "none".to_string() } else { list.join(",") },
                    h.soft,
                    h.soft_radius,
                    h.max_len_mul,
                    h.set_cos
                );
            }
        }
        let _ = writeln!(t, "ambient_top = {}\nambient_bottom = {}", hex(l.amb_top), hex(l.amb_bottom));
        let _ = writeln!(t, "depth_color = {}\ndepth_color2 = {}", hex(l.depth_col), hex(l.depth_col2));
        let _ = writeln!(t, "dynfog_color = {}\ndynfog_color2 = {}", hex(l.dyn_fog_col), hex(l.dyn_fog_col2));
        let _ = writeln!(t, "fog_offset = {}\ndepth_fog = {}\ndyn_fog = {}", l.fog_offset, l.depth_fog, l.dyn_fog);
        let _ = writeln!(t, "diffuse = {}\nspecular = {}\nambient = {}", l.diffuse, l.specular, l.ambient);
        let _ = writeln!(t, "ambient_shadow = {}\nindirect_light = {}", l.amb_shadow, l.ind_light);
        let _ = writeln!(t, "gamma = {}\nroughness = {}", l.gamma, l.roughness);
        let _ = writeln!(t, "color_start = {}\ncolor_end = {}\ncolor_cycling = {}", l.color_start, l.color_end, l.color_cycling);
        let _ = writeln!(t, "color_var_z = {}\ncolor_on_otrap = {}\nfar_fog = {}\ndepth_func = {}", l.var_col_z, l.color_on_otrap, l.far_fog, l.depth_func);
        if let Some((a, b)) = l.fine_col_adj {
            let _ = writeln!(t, "fine_color_adjust = {a}, {b}");
        }
        let _ = writeln!(t, "interior_start = {}\ninterior_end = {}\ndiffuse_shadowing = {}", l.interior_start, l.interior_end, l.diffuse_shadowing);
        let pal: Vec<String> = l.palette.iter().map(|p| format!("{}:{}:{}", p.position, hex(p.diffuse), hex(p.specular))).collect();
        let _ = writeln!(t, "palette_full = {}", pal.join(", "));
        if let Some(a) = l.palette_alpha {
            let a: Vec<String> = a.iter().map(|v| v.to_string()).collect();
            let _ = writeln!(t, "palette_alpha = {}", a.join(", "));
        }
        let int: Vec<String> =
            l.interior.iter().zip(l.interior_spec).map(|((p, c), a)| format!("{}:{}:{}", p, hex(*c), a)).collect();
        let _ = writeln!(t, "interior_colors = {}", int.join(", "));
        let dl = Lighting::default();
        if l.internal_gamma2 {
            let _ = writeln!(t, "internal_gamma2 = true");
        }
        if l.no_col_ipol {
            let _ = writeln!(t, "color_interpolation = false");
        }
        if l.amb_rel_obj {
            let _ = writeln!(t, "ambient_relative_to_object = true");
        }
        if l.dfog_options != 0 {
            let _ = writeln!(t, "dynfog_options = {}", l.dfog_options);
        }
        if l.ex_mode != 0 {
            let _ = writeln!(t, "light_mode = {}", l.ex_mode);
        }
        if l.diff_map != 0 {
            let _ = writeln!(t, "diffuse_map = {}\ndiffuse_map_mode = {}", l.diff_map, DIFF_MAP_MODES[(l.diff_map_mode & 3) as usize]);
            let _ = writeln!(t, "diffuse_map_offset = {}, {}", l.diff_map_offset[0], l.diff_map_offset[1]);
            let _ = writeln!(t, "diffuse_map_rotation = {}\ndiffuse_map_scale = {}", l.diff_map_rot, l.diff_map_scale);
        } else if (l.diff_map_mode, l.diff_map_offset, l.diff_map_rot, l.diff_map_scale)
            != (dl.diff_map_mode, dl.diff_map_offset, dl.diff_map_rot, dl.diff_map_scale)
        {
            let _ = writeln!(t, "diffuse_map_mode = {}", DIFF_MAP_MODES[(l.diff_map_mode & 3) as usize]);
            let _ = writeln!(t, "diffuse_map_offset = {}, {}", l.diff_map_offset[0], l.diff_map_offset[1]);
            let _ = writeln!(t, "diffuse_map_rotation = {}\ndiffuse_map_scale = {}", l.diff_map_rot, l.diff_map_scale);
        }
        if l.yc_comb {
            let _ = writeln!(t, "diffuse_map_brightness_only = true");
        }
        if l.bg_ambient {
            let _ = writeln!(t, "background_ambient = true");
        }
        if !l.bg_image.is_empty() {
            let _ = writeln!(t, "background_image = {}", l.bg_image);
            let _ = writeln!(t, "background_rotation = {}, {}, {}", l.bg_rot[0], l.bg_rot[1], l.bg_rot[2]);
            let _ = writeln!(t, "background_direct = {}\nbackground_brightness = {}", l.bg_direct, l.bg_brightness);
            let _ = writeln!(t, "background_add_light = {}", l.bg_add_light);
        }
        for f in &self.formulas {
            let _ = writeln!(t, "\n[formula]\nname = {}\niterations = {}", f.formula.name(), f.iterations);
            for (i, (k, v)) in f.formula.options().into_iter().enumerate() {
                let k = k.to_ascii_lowercase();
                let plain = !k.trim().is_empty()
                    && !k.contains(['#', '=', ';', '[', ']'])
                    && !k.trim_start().starts_with("//")
                    && k.trim() != "name"
                    && k.trim() != "iterations"
                    && k.trim() != "its";
                if plain {
                    let _ = writeln!(t, "{} = {}", k.trim(), v);
                } else {
                    let _ = writeln!(t, "option{i} = {v} ; {}", k.replace(['\n', '\r'], " "));
                }
            }
        }
        if !l.lights.iter().any(|li| li.on || li.map > 0) {
            let _ = writeln!(t, "lights = none");
        }
        for (slot, li) in l.lights.iter().enumerate().filter(|(_, li)| li.on || li.map > 0) {
            let _ = writeln!(t, "\n[light]\nslot = {}\ncolor = {}", slot + 1, hex(li.color));
            if !li.on {
                let _ = writeln!(t, "on = false");
            }
            if li.map > 0 {
                let r = li.map_rot;
                let _ = writeln!(t, "map = {}\nmap_rotation = {}, {}, {}", li.map, r[0], r[1], r[2]);
                let _ = writeln!(t, "amplitude = {}", li.amplitude);
                continue;
            }
            if li.positional {
                let _ = writeln!(t, "position = {}, {}, {}", li.position[0], li.position[1], li.position[2]);
            } else {
                let r = |v: f64| (v.to_degrees() * 1e9).round() / 1e9;
                let _ = writeln!(t, "x_angle = {}\ny_angle = {}", r(li.x_angle), r(li.y_angle));
            }
            if li.visible != 0 {
                let _ = writeln!(t, "visible = {}", li.visible / 2);
            }
            if !li.hs_enabled {
                let _ = writeln!(t, "shadow = false");
            }
            let _ = writeln!(t, "amplitude = {}\nspec_power = {}", li.amplitude, 2 << li.spec_func);
            let _ = writeln!(t, "diffuse_func = {}\nrelative_to_object = {}", li.diff_func, li.relative_to_object);
        }
        t
    }

    /// Parse the INI-like scene description.
    pub fn parse(text: &str) -> Result<Scene, String> {
        Self::parse_onto(Scene::default(), text, true)
    }

    /// Applies `key = value` lines (and optional sections) on top of an
    /// existing scene, e.g. command line overrides for a loaded .m3p file.
    pub fn apply(self, text: &str) -> Result<Scene, String> {
        Self::parse_onto(self, text, false)
    }

    fn parse_onto(base: Scene, text: &str, fresh: bool) -> Result<Scene, String> {
        let mut s = base;
        if fresh {
            s.formulas.clear();
        }
        let mut section = String::new();
        let mut light_idx: Option<usize> = None;
        let mut rot_deg: Option<[f64; 3]> = None;
        for (lnr, raw) in text.lines().enumerate() {
            let line = raw.split(';').next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let err = |m: String| format!("line {}: {m}", lnr + 1);
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_ascii_lowercase();
                match section.as_str() {
                    "formula" => {
                        if s.formulas.len() >= 6 {
                            return Err(err("at most 6 formulas".into()));
                        }
                        s.formulas.push(FormulaEntry {
                            formula: Formula::default_for("Integer Power").unwrap(),
                            iterations: 1,
                        });
                    }
                    "light" => {
                        let i = light_idx.map_or(0, |i| i + 1);
                        if i >= 6 {
                            return Err(err("at most 6 lights".into()));
                        }
                        if i == 0 {
                            for l in s.lighting.lights.iter_mut() {
                                l.on = false;
                            }
                        }
                        s.lighting.lights[i].on = true;
                        light_idx = Some(i);
                    }
                    "scene" | "camera" | "lighting" | "render" => {}
                    _ => return Err(err(format!("unknown section [{section}]"))),
                }
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| err("expected key = value".into()))?;
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            let num = || val.parse::<f64>().map_err(|_| err(format!("bad number '{val}'")));
            let int = || num().map(|x| x.round() as i32);
            let boolean = || match val.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => Ok(true),
                "0" | "false" | "no" | "off" => Ok(false),
                _ => Err(err(format!("bad boolean '{val}'"))),
            };
            let vec_n = |n: usize| -> Result<Vec<f64>, String> {
                let parts: Result<Vec<f64>, _> =
                    val.split(',').map(|p| p.trim().parse::<f64>()).collect();
                let parts = parts.map_err(|_| err(format!("bad vector '{val}'")))?;
                if parts.len() != n {
                    return Err(err(format!("expected {n} values")));
                }
                Ok(parts)
            };
            let color = || parse_color(val).ok_or_else(|| err(format!("bad color '{val}'")));

            match section.as_str() {
                "formula" => {
                    let e = s.formulas.last_mut().unwrap();
                    match key.as_str() {
                        "name" => {
                            e.formula = crate::formulas::lookup(val).map_err(err)?
                        }
                        "iterations" | "its" => e.iterations = int()?,
                        _ => e.formula.set_option(&key, num()?).map_err(err)?,
                    }
                }
                "light" => {
                    if key == "slot" {
                        let n = int()?;
                        if !(1..=6).contains(&n) {
                            return Err(err("slot must be 1..6".into()));
                        }
                        let i = light_idx.unwrap();
                        let j = (n - 1) as usize;
                        if j != i {
                            s.lighting.lights[i].on = false;
                            s.lighting.lights[j].on = true;
                            light_idx = Some(j);
                        }
                        continue;
                    }
                    let l = &mut s.lighting.lights[light_idx.unwrap()];
                    match key.as_str() {
                        "map" => {
                            l.map = int()?.clamp(0, 32000) as u16;
                            l.positional = false;
                        }
                        "map_rotation" => {
                            let p = vec_n(3)?;
                            l.map_rot = [p[0] as u8, p[1] as u8, p[2] as u8];
                        }
                        "on" => l.on = boolean()?,
                        "color" => l.color = color()?,
                        "x_angle" | "xangle" => l.x_angle = num()?.to_radians(),
                        "y_angle" | "yangle" => l.y_angle = num()?.to_radians(),
                        "amplitude" | "amp" => l.amplitude = num()? as f32,
                        "spec_power" | "specular_power" => {
                            // MB3D: 2 shl (0..7) = 2..256
                            let p = num()?.max(2.0).log2().round() as i32 - 1;
                            l.spec_func = p.clamp(0, 7);
                        }
                        "diffuse_func" => l.diff_func = int()?.clamp(0, 3),
                        "relative_to_object" => l.relative_to_object = boolean()?,
                        "position" => {
                            let p = vec_n(3)?;
                            l.positional = true;
                            l.position = [p[0], p[1], p[2]];
                        }
                        "positional" => l.positional = boolean()?,
                        // MB3D's visible light functions: 1 = "vislight 3", 2 = old, 3 = "vislight 2", 4 = glow
                        "visible" => l.visible = (int()?.clamp(0, 7) as u8) * 2,
                        "shadow" | "hard_shadow" => l.hs_enabled = boolean()?,
                        _ => return Err(err(format!("unknown light key '{key}'"))),
                    }
                }
                _ => match key.as_str() {
                    "width" => s.width = int()?.max(16),
                    "height" => s.height = int()?.max(16),
                    "tiles" => {
                        let v = val.to_ascii_lowercase();
                        if matches!(v.as_str(), "off" | "none") {
                            s.tiling = None;
                        } else {
                            let (a, b) = v.split_once(['x', ',']).ok_or_else(|| err(format!("bad tiles '{val}' (e.g. 4x3)")))?;
                            let (a, b): (u32, u32) = (
                                a.trim().parse().map_err(|_| err(format!("bad tiles '{val}'")))?,
                                b.trim().parse().map_err(|_| err(format!("bad tiles '{val}'")))?,
                            );
                            if !(1..=127).contains(&a) || !(1..=127).contains(&b) {
                                return Err(err("tile counts must be 1..127".into()));
                            }
                            let old = s.tiling.unwrap_or(Tiling { cols: 1, rows: 1, pos: None, downscale: 1 });
                            // a single tile outside of the new grid is dropped
                            let pos = old.pos.filter(|&(c, r)| c < a && r < b);
                            s.tiling = Some(Tiling { cols: a, rows: b, pos, ..old });
                        }
                    }
                    "tile" => {
                        let tl = s.tiling.as_mut().ok_or_else(|| err("'tile' needs 'tiles' first".into()))?;
                        if matches!(val.to_ascii_lowercase().as_str(), "all" | "none") {
                            tl.pos = None;
                        } else {
                            let p = vec_n(2)?;
                            let (c, r) = (p[0].round() as i64, p[1].round() as i64);
                            if c < 1 || r < 1 || c > tl.cols as i64 || r > tl.rows as i64 {
                                return Err(err(format!("tile {c}, {r} outside of {}x{}", tl.cols, tl.rows)));
                            }
                            tl.pos = Some((c as u32 - 1, r as u32 - 1));
                        }
                    }
                    "tile_downscale" => {
                        let d = int()?.clamp(1, 3) as u32;
                        let tl = s.tiling.get_or_insert(Tiling { cols: 1, rows: 1, pos: None, downscale: 1 });
                        tl.downscale = d;
                    }
                    "lights" => match val.to_ascii_lowercase().as_str() {
                        "none" | "off" | "0" => {
                            for l in s.lighting.lights.iter_mut() {
                                l.on = false;
                                l.map = 0;
                            }
                        }
                        _ => return Err(err(format!("bad lights value '{val}' (none)"))),
                    },
                    "background_image" | "background" => s.lighting.bg_image = val.to_string(),
                    "background_rotation" => {
                        let p = vec_n(3)?;
                        s.lighting.bg_rot = [p[0] as u8, p[1] as u8, p[2] as u8];
                    }
                    "background_direct" => s.lighting.bg_direct = boolean()?,
                    "background_brightness" => s.lighting.bg_brightness = int()?.clamp(0, 255) as u8,
                    "background_add_light" => s.lighting.bg_add_light = boolean()?,
                    "iterations" | "max_iterations" => s.iterations = int()?.max(1),
                    "min_iterations" => s.min_iterations = int()?.max(0),
                    // MB3D's "color on it" field: -1 = off, 0 = the start vector
                    "color_on_iteration" | "color_on_it" => {
                        s.color_on_it = if matches!(val.to_ascii_lowercase().as_str(), "off" | "none" | "no") {
                            0
                        } else {
                            (int()? + 1).clamp(0, 255) as u8
                        }
                    }
                    "mc_depth" => s.mc.depth = int()?.clamp(1, 255) as u8,
                    "mc_reflection_depth" => s.mc.reflection_depth = int()?.clamp(0, 255) as u8,
                    "mc_reflection_amount" => s.mc.reflection_amount = (num()? as f32).clamp(0.0, 100.0),
                    "mc_reflections" => s.mc.reflections = boolean()?,
                    "mc_transparency" => s.mc.transparency = boolean()?,
                    "mc_only_difs" => s.mc.only_difs = boolean()?,
                    "mc_diffuse_reflects" => s.mc.diffuse_reflects = (num()? * 100.0).round().clamp(0.0, 255.0) as u8,
                    "mc_soft_shadow_radius" => s.mc.soft_shadow_radius = (num()? as f32).max(0.0),
                    "mc_exposure" | "mc_contrast" => s.mc.contrast = int()?.clamp(0, 255) as u8,
                    "mc_saturation" => s.mc.saturation = int()?.clamp(0, 127) as u8,
                    "mc_options" => s.mc.options = int()?.clamp(0, 127) as u8,
                    "mc_soft_clip" | "mc_secant_search" | "mc_autoclip" | "mc_gauss_aa" => {
                        let bit = match key.as_str() {
                            "mc_soft_clip" => 1,
                            "mc_secant_search" => 2,
                            "mc_autoclip" => 4,
                            _ => 8,
                        };
                        if boolean()? {
                            s.mc.options |= bit;
                        } else {
                            s.mc.options &= !bit;
                        }
                    }
                    "mc_bokeh" => s.mc.options = (s.mc.options & 0x0F) | (((int()? - 1).clamp(0, 5) as u8) << 4),
                    "mc_refraction_index" => s.mc.refraction_index = (num()? as f32).clamp(0.1, 10.0),
                    "mc_absorption" => s.mc.absorption = (num()? as f32).max(1e-30),
                    "mc_scattering" => s.mc.scattering = (num()? as f32).max(0.0),
                    "normals_on_zbuf" | "normals_on_z_buffer" => s.normals_on_zbuf = boolean()?,
                    "stereo" | "stereo_mode" => {
                        s.stereo_mode = match val.to_ascii_lowercase().as_str() {
                            "off" | "no" | "mono" | "0" => 0,
                            "very_left" | "very left" | "1" => 1,
                            "right" | "3" => 3,
                            "left" | "4" => 4,
                            _ => return Err(err(format!("bad stereo '{val}' (off, left, right, very_left)"))),
                        }
                    }
                    "stereo_screen" => {
                        let p = vec_n(3)?;
                        if p.iter().any(|v| *v <= 0.0) {
                            return Err(err("stereo_screen: screen width, screen distance and minimum distance (metres) must be > 0".to_string()));
                        }
                        s.stereo_screen = [p[0] as f32, p[1] as f32, p[2] as f32];
                    }
                    "slice_2d" | "2d" => {
                        s.slice_2d = match val.to_ascii_lowercase().as_str() {
                            "off" | "no" | "false" | "3d" | "0" => 0,
                            "start" | "z_start" | "1" => 1,
                            "mid" | "middle" | "z_mid" | "2" | "on" | "yes" | "true" => 2,
                            "end" | "z_end" | "3" => 3,
                            _ => return Err(err(format!("bad slice_2d '{val}' (off, start, mid, end)"))),
                        }
                    }
                    "rstop" | "bailout" => s.rstop = Some(num()?),
                    "zoom" => s.zoom = num()?,
                    "mid" | "position" => {
                        let p = vec_n(3)?;
                        s.mid = [p[0], p[1], p[2]];
                    }
                    "z_start" | "zstart" => s.z_start = num()?,
                    "z_end" | "zend" => s.z_end = num()?,
                    "rotation" => {
                        let p = vec_n(3)?;
                        rot_deg = Some([p[0], p[1], p[2]]);
                    }
                    "vgrads" | "matrix" => {
                        let p = vec_n(9)?;
                        s.vgrads = [[p[0], p[1], p[2]], [p[3], p[4], p[5]], [p[6], p[7], p[8]]];
                    }
                    "fov" | "fov_y" => s.fov_y = num()?,
                    "optic" => {
                        s.optic = match int()? {
                            1 => CameraOptic::Planar,
                            2 => CameraOptic::Panorama,
                            _ => CameraOptic::Common,
                        }
                    }
                    "de_stop" | "destop" => s.de_stop = num()?,
                    "raystep" | "z_step_div" | "raystep_multiplier" => {
                        s.z_step_div = num()?.clamp(0.001, 1.0)
                    }
                    "raystep_limiter" => s.raystep_limiter = num()?,
                    "smooth_normals" => s.smooth_normals = int()?.clamp(0, 8),
                    "bin_search" | "bin_search_steps" => s.bin_search_steps = int()?.clamp(0, 255),
                    "first_step_random" => s.first_step_random = boolean()?,
                    "step_sub_de_stop" => s.step_sub_de_stop = boolean()?,
                    "vary_de_stop" | "vary_de_stop_on_fov" => s.vary_de_stop_on_fov = boolean()?,
                    "normals_on_de" => s.normals_on_de = boolean()?,
                    "disable_analytic_de" => s.disable_analytic_de = boolean()?,
                    "julia" => s.julia = boolean()?,
                    "julia_c" => {
                        let p = val.split(',').count();
                        let p = vec_n(p.clamp(3, 4))?;
                        s.julia_c = [p[0], p[1], p[2], *p.get(3).unwrap_or(&0.0)];
                    }
                    "rotation_4d" => {
                        let p = vec_n(3)?;
                        s.rot_4d = [p[0].to_radians(), p[1].to_radians(), p[2].to_radians()];
                    }
                    "ao" | "ambient_occlusion" => {
                        let v = val.to_ascii_lowercase();
                        if v == "deao" {
                            s.deao.get_or_insert_with(Default::default);
                        } else {
                            s.deao = None;
                        }
                        s.ao = match v.as_str() {
                            "deao" => Some(s.ao.unwrap_or(crate::ssao::SsaoParams::default())),
                            "off" | "no" | "false" | "0" | "none" => None,
                            "ssao15" => Some(crate::ssao::SsaoParams {
                                t0: false,
                                bits15: true,
                                random: 0,
                                ..s.ao.unwrap_or_default()
                            }),
                            "ssao15t0" => Some(crate::ssao::SsaoParams {
                                t0: true,
                                bits15: true,
                                random: 0,
                                ..s.ao.unwrap_or_default()
                            }),
                            "ssao24" | "on" | "yes" | "true" | "1" => Some(crate::ssao::SsaoParams {
                                bits15: false,
                                t0: false,
                                ..s.ao.unwrap_or(crate::ssao::SsaoParams::default())
                            }),
                            "ssao24t0" | "t0" => Some(crate::ssao::SsaoParams {
                                bits15: false,
                                t0: true,
                                ..s.ao.unwrap_or(crate::ssao::SsaoParams { t0: true, ..Default::default() })
                            }),
                            _ => return Err(err(format!("bad ao mode '{val}' (off, ssao24, ssao24t0, ssao15, ssao15t0, deao)"))),
                        }
                    }
                    "shadows" | "hard_shadows" => {
                        let v = val.to_ascii_lowercase();
                        let lights = match v.as_str() {
                            "off" | "no" | "false" | "0" => None,
                            "all" | "on" | "yes" | "true" => Some(0x3F),
                            "none" => Some(0),
                            _ => {
                                let mut m = 0u8;
                                for part in v.split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()) {
                                    match part.parse::<u8>() {
                                        Ok(n @ 1..=6) => m |= 1 << (n - 1),
                                        _ => return Err(err(format!("bad shadow light '{part}' (off, all or light numbers 1-6)"))),
                                    }
                                }
                                Some(m)
                            }
                        };
                        s.shadows = lights.map(|l| ShadowParams { lights: l, ..s.shadows.unwrap_or_default() });
                    }
                    "vol_light" | "volumetric_light" => {
                        s.vol_light = match val.to_ascii_lowercase().as_str() {
                            "off" | "no" | "false" | "0" => None,
                            _ => {
                                let n = int()?;
                                if !(1..=6).contains(&n) {
                                    return Err(err(format!("bad vol_light '{val}' (off or light number 1-6)")));
                                }
                                Some(crate::vollight::VolLightParams {
                                    light: (n - 1) as usize,
                                    map_size: s.vol_light.map(|v| v.map_size).unwrap_or(0),
                                })
                            }
                        }
                    }
                    "cut_x" | "cut_y" | "cut_z" => {
                        let k = (key.as_bytes()[4] - b'x') as usize;
                        if matches!(val.to_ascii_lowercase().as_str(), "off" | "none" | "no") {
                            s.cut_options &= !(1 << k);
                        } else {
                            s.cut_pos[k] = num()?;
                            s.cut_options |= 1 << k;
                        }
                    }
                    "deao_quality" | "deao_dither" | "deao_max_len" | "deao_random" => {
                        let d = s.deao.get_or_insert_with(Default::default);
                        match key.as_str() {
                            "deao_quality" => d.quality = int()?.clamp(0, 3) as u8,
                            "deao_dither" => d.dither = int()?.clamp(0, 2) as u8,
                            "deao_max_len" => d.max_len = (num()? as f32).max(0.01),
                            _ => d.first_step_random = boolean()?,
                        }
                    }
                    "inside" | "inside_rendering" => {
                        s.inside = match val.to_ascii_lowercase().as_str() {
                            "off" | "no" | "false" | "outside" => InsideMode::Outside,
                            "on" | "yes" | "true" | "inside" => InsideMode::Inside,
                            "both" | "in_and_outside" => InsideMode::Both,
                            _ => return Err(err(format!("bad inside mode '{val}' (outside, inside, both)"))),
                        }
                    }
                    "interpolation" | "interpolation_hybrid" => {
                        if matches!(val.to_ascii_lowercase().as_str(), "off" | "none" | "no") {
                            s.interpolation = None;
                        } else {
                            let p = vec_n(2)?;
                            s.interpolation = Some([p[0] as f32, p[1] as f32]);
                            s.decomb = None;
                        }
                    }
                    "de_combination" | "decomb" => {
                        let v = val.to_ascii_lowercase();
                        if matches!(v.as_str(), "off" | "none" | "no") {
                            s.decomb = None;
                        } else {
                            let k = DECOMB_NAMES
                                .iter()
                                .position(|n| *n == v)
                                .ok_or_else(|| err(format!("bad de_combination '{val}' ({})", DECOMB_NAMES.join(", "))))?;
                            let d = s.decomb.get_or_insert_with(|| DeCombParams::new(s.iterations));
                            d.kind = k as u8 + 1;
                        }
                    }
                    "decomb_end1" | "decomb_start2" | "decomb_end2" | "decomb_repeat2" | "decomb_iterations2"
                    | "decomb_smooth" | "decomb_mix_pow" | "decomb_mix_color" => {
                        let v = num()?;
                        let its = s.iterations;
                        let d = s.decomb.get_or_insert_with(|| DeCombParams::new(its));
                        let slot = (v as i64 - 1).clamp(0, 5) as usize;
                        match key.as_str() {
                            "decomb_end1" => d.end1 = slot,
                            "decomb_start2" => d.start2 = slot,
                            "decomb_end2" => d.end2 = slot,
                            "decomb_repeat2" => d.repeat2 = slot,
                            "decomb_iterations2" => d.iterations2 = (v as i32).max(1),
                            "decomb_smooth" => d.smooth = v as f32,
                            "decomb_mix_pow" => d.mix_pow = v as f32,
                            _ => d.mix_color = (v as i64).clamp(0, 2) as u8,
                        }
                    }
                    "dof" | "depth_of_field" => {
                        s.dof = match val.to_ascii_lowercase().as_str() {
                            "off" | "no" | "false" | "0" => None,
                            "sorted" | "on" | "yes" | "true" | "1" => Some(crate::dof::DofParams { forward: false, ..s.dof.unwrap_or_default() }),
                            "forward" => Some(crate::dof::DofParams { forward: true, ..s.dof.unwrap_or_default() }),
                            _ => return Err(err(format!("bad dof mode '{val}' (off, sorted, forward)"))),
                        }
                    }
                    "dof_focus" | "dof_focus2" | "dof_aperture" | "dof_max_radius" | "dof_passes" => {
                        let v = num()?;
                        if let Some(d) = s.dof.as_mut() {
                            match key.as_str() {
                                "dof_focus" => {
                                    // a single focus sets both points
                                    d.z_sharp = v as f32;
                                    d.z_sharp2 = v as f32;
                                }
                                "dof_focus2" => d.z_sharp2 = v as f32,
                                "dof_aperture" => d.aperture = (v as f32).clamp(0.0001, 2.0),
                                "dof_max_radius" => d.clip_r = (v as f32).clamp(0.1, 1000.0),
                                _ => d.passes = (v as i64).clamp(1, 4) as u8,
                            }
                        }
                    }
                    "vol_light_map_size" => {
                        let v = int()?.clamp(-7, 7);
                        if let Some(vl) = s.vol_light.as_mut() {
                            vl.map_size = v;
                        }
                    }
                    "shadow_soft" => {
                        let b = boolean()?;
                        if let Some(h) = s.shadows.as_mut() {
                            h.soft = b;
                        }
                    }
                    "shadow_soft_radius" => {
                        let v = num()? as f32;
                        if let Some(h) = s.shadows.as_mut() {
                            h.soft_radius = v.clamp(0.01, 20.0);
                        }
                    }
                    "shadow_max_len" => {
                        let v = num()? as f32;
                        if let Some(h) = s.shadows.as_mut() {
                            h.max_len_mul = v.max(0.01);
                        }
                    }
                    "shadow_set_cos" => {
                        let b = boolean()?;
                        if let Some(h) = s.shadows.as_mut() {
                            h.set_cos = b;
                        }
                    }
                    "ao_threshold" => {
                        let v = num()? as f32;
                        if let Some(a) = s.ao.as_mut() {
                            a.threshold = v;
                        }
                    }
                    "ao_random" => {
                        let v = int()?.clamp(0, 255) as u8;
                        if let Some(a) = s.ao.as_mut() {
                            a.random = v;
                        }
                    }
                    "ao_border" => {
                        let v = num()? as f32;
                        if let Some(a) = s.ao.as_mut() {
                            a.border_mirror = v.clamp(0.0, 0.9);
                        }
                    }
                    "color_option" => s.color_option = int()?.clamp(0, 5) as u8,
                    "color_mul" => s.color_mul = num()?,
                    "dfog_on_it" => s.dfog_on_it = int()?.clamp(0, 65534) as u16,
                    "repeat_from" => s.repeat_from = int()?.clamp(0, 5) as usize,
                    "threads" => s.threads = int()?.max(0) as usize,
                    // lighting (global)
                    "ambient_top" => s.lighting.amb_top = color()?,
                    "ambient_bottom" => s.lighting.amb_bottom = color()?,
                    "depth_color" => s.lighting.depth_col = color()?,
                    "depth_color2" => s.lighting.depth_col2 = color()?,
                    "dynfog_color" => s.lighting.dyn_fog_col = color()?,
                    "dynfog_color2" => s.lighting.dyn_fog_col2 = color()?,
                    "depth_fog" => s.lighting.depth_fog = num()? as f32,
                    "dynfog" | "dyn_fog" => s.lighting.dyn_fog = num()? as f32,
                    "diffuse" => s.lighting.diffuse = num()? as f32,
                    "specular" => s.lighting.specular = num()? as f32,
                    "ambient" => s.lighting.ambient = num()? as f32,
                    "ambient_shadow" => s.lighting.amb_shadow = num()? as f32,
                    "indirect_light" => s.lighting.ind_light = num()? as f32,
                    "fog_offset" => s.lighting.fog_offset = num()? as f32,
                    "gamma" => s.lighting.gamma = num()? as f32,
                    "color_start" => s.lighting.color_start = num()? as f32,
                    "color_end" => s.lighting.color_end = num()? as f32,
                    "color_cycling" => s.lighting.color_cycling = boolean()?,
                    "roughness" => s.lighting.roughness = num()? as f32,
                    "color_var_z" => s.lighting.var_col_z = num()? as f32,
                    "color_on_otrap" => s.lighting.color_on_otrap = boolean()?,
                    "far_fog" => s.lighting.far_fog = boolean()?,
                    "depth_func" => s.lighting.depth_func = int()?.clamp(0, 3) as u8,
                    "fine_color_adjust" => {
                        if matches!(val.to_ascii_lowercase().as_str(), "off" | "none") {
                            s.lighting.fine_col_adj = None;
                        } else {
                            let p = vec_n(2)?;
                            s.lighting.fine_col_adj = Some((p[0].clamp(0.0, 255.0) as u8, p[1].clamp(0.0, 255.0) as u8));
                        }
                    }
                    "interior_start" => s.lighting.interior_start = num()? as f32,
                    "interior_end" => s.lighting.interior_end = num()? as f32,
                    "diffuse_shadowing" => s.lighting.diffuse_shadowing = num()? as f32,
                    "internal_gamma2" => s.lighting.internal_gamma2 = boolean()?,
                    "color_interpolation" => s.lighting.no_col_ipol = !boolean()?,
                    "ambient_relative_to_object" => s.lighting.amb_rel_obj = boolean()?,
                    "dynfog_options" => s.lighting.dfog_options = int()?.clamp(0, 3) as u8,
                    "light_mode" => s.lighting.ex_mode = int()?.clamp(0, 255) as u8,
                    "diffuse_map" => s.lighting.diff_map = int()?.clamp(0, 32000) as u16,
                    "diffuse_map_mode" => {
                        let v = val.to_ascii_lowercase();
                        s.lighting.diff_map_mode = match DIFF_MAP_MODES.iter().position(|m| *m == v) {
                            Some(i) => i as u8,
                            None => int()?.clamp(0, 3) as u8,
                        };
                    }
                    "diffuse_map_offset" => {
                        let p = vec_n(2)?;
                        s.lighting.diff_map_offset = [p[0].clamp(0.0, 255.0) as u8, p[1].clamp(0.0, 255.0) as u8];
                    }
                    "diffuse_map_rotation" => s.lighting.diff_map_rot = int()?.clamp(0, 255) as u8,
                    "diffuse_map_scale" => s.lighting.diff_map_scale = int()?.clamp(0, 255) as u8,
                    "diffuse_map_brightness_only" => s.lighting.yc_comb = boolean()?,
                    "background_ambient" => s.lighting.bg_ambient = boolean()?,
                    "palette_full" => {
                        let items: Vec<&str> = val.split(',').map(|x| x.trim()).collect();
                        if items.len() != 10 {
                            return Err(err("palette_full needs 10 entries position:#diffuse:#specular".into()));
                        }
                        for (i, it) in items.iter().enumerate() {
                            let f: Vec<&str> = it.split(':').collect();
                            let bad = || err(format!("bad palette entry '{it}'"));
                            if f.len() != 3 {
                                return Err(bad());
                            }
                            let pc = &mut s.lighting.palette[i];
                            pc.position = f[0].trim().parse().map_err(|_| bad())?;
                            pc.diffuse = parse_color(f[1].trim()).ok_or_else(bad)?;
                            pc.specular = parse_color(f[2].trim()).ok_or_else(bad)?;
                        }
                    }
                    "interior_colors" => {
                        let items: Vec<&str> = val.split(',').map(|x| x.trim()).collect();
                        if items.len() != 4 {
                            return Err(err("interior_colors needs 4 entries position:#color".into()));
                        }
                        for (i, it) in items.iter().enumerate() {
                            let bad = || err(format!("bad interior colour '{it}'"));
                            let f: Vec<&str> = it.split(':').map(|x| x.trim()).collect();
                            if f.len() < 2 || f.len() > 3 {
                                return Err(bad());
                            }
                            s.lighting.interior[i] = (f[0].parse().map_err(|_| bad())?, parse_color(f[1]).ok_or_else(bad)?);
                            if f.len() == 3 {
                                s.lighting.interior_spec[i] = f[2].parse().map_err(|_| bad())?;
                            }
                        }
                    }
                    "palette_alpha" | "palette_transparency" => {
                        let v: Result<Vec<u8>, _> = val.split(',').map(|x| x.trim().parse::<u8>()).collect();
                        match v {
                            Ok(v) if v.len() == 10 => s.lighting.palette_alpha = Some(v.try_into().unwrap()),
                            _ => return Err(err(format!("palette_alpha: 10 values 0..255 expected, got '{val}'"))),
                        }
                    }
                    "palette" => {
                        let cols: Result<Vec<[u8; 3]>, String> =
                            val.split(',').map(|c| parse_color(c.trim()).ok_or(c)).map(|r| r.map_err(|c| err(format!("bad color '{c}'")))).collect();
                        s.lighting.set_palette(&cols?);
                    }
                    _ => return Err(err(format!("unknown key '{key}'"))),
                },
            }
        }
        if let Some(r) = rot_deg {
            s.vgrads = vgrads_from_angles(r[0].to_radians(), r[1].to_radians(), r[2].to_radians());
        }
        if s.formulas.is_empty() {
            s.formulas = Scene::default().formulas;
        }
        Ok(s)
    }
}

/// Parses `#RRGGBB`, `RRGGBB` or `r,g,b`-less hex colors.
pub fn parse_color(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#').trim_start_matches("0x");
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_scene() {
        let s = Scene::parse(
            "width = 320\nheight=200\nrotation = 0, 90, 0\n[formula]\nname = amazing box\nscale = -1.5\n[formula]\nname=Integer Power\npower = 4\niterations=2\n[light]\ncolor=#ff8000\n",
        )
        .unwrap();
        assert_eq!(s.width, 320);
        assert_eq!(s.formulas.len(), 2);
        assert_eq!(s.formulas[1].iterations, 2);
        assert!(matches!(s.formulas[0].formula, Formula::AmazingBox { scale, .. } if scale == -1.5));
        assert_eq!(s.lighting.lights[0].color, [255, 128, 0]);
        assert!(!s.lighting.lights[1].on);
    }

    #[test]
    fn fewer_tiles_drop_an_outside_tile() {
        let s = Scene::parse("tiles = 4x4
tile = 4, 2
").unwrap();
        assert_eq!(s.tiling.unwrap().pos, Some((3, 1)));
        assert_eq!(s.clone().apply("tiles = 2x2
").unwrap().tiling.unwrap().pos, None);
        assert_eq!(s.apply("tiles = 4x2
").unwrap().tiling.unwrap().pos, Some((3, 1)));
    }

    #[test]
    fn text_round_trip() {
        for name in crate::formulas::Formula::all_names() {
            let a = Scene::preset(name).unwrap();
            let b = Scene::parse(&a.to_text()).unwrap();
            assert_eq!(a.to_text(), b.to_text(), "{name}");
            assert_eq!(a.formulas[0].formula, b.formulas[0].formula);
        }
    }
}

/// Hard shadow settings (`bCalculateHardShadow`, `bCalc1HSsoft`,
/// `MCSoftShadowRadius`, `HSmaxLengthMultiplier`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowParams {
    /// bit i: light i+1 casts shadows
    pub lights: u8,
    /// one soft shadow (for the last selected light) instead of hard shadows
    pub soft: bool,
    pub soft_radius: f32,
    /// maximum shadow ray length multiplier
    pub max_len_mul: f32,
    /// set the diffuse function of the shadowed lights to cos
    pub set_cos: bool,
}

impl Default for ShadowParams {
    fn default() -> Self {
        ShadowParams { lights: 1, soft: false, soft_radius: 1.0, max_len_mul: 1.0, set_cos: false }
    }
}

/// Names of the DE combination types (`FormulaType` 1..6).
pub const DECOMB_NAMES: [&str; 6] = ["min", "max", "max_inverted", "smooth_linear", "smooth", "mix"];

/// DE combination settings (`bOptions1 = 2` in the formula block).
/// Slots are 0-based here and 1-based in the text format.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeCombParams {
    /// 1..6, see [`DECOMB_NAMES`]
    pub kind: u8,
    /// last slot of the first part
    pub end1: usize,
    pub start2: usize,
    pub end2: usize,
    pub repeat2: usize,
    /// maximum iterations of the second part (`iMaxItsF2`)
    pub iterations2: i32,
    /// smoothing distance of the smooth combinations (`sDEcombS`)
    pub smooth: f32,
    /// `sFmixPow` (mix)
    pub mix_pow: f32,
    /// colour of the mix: 0 both, 1 first part, 2 second part
    pub mix_color: u8,
}

impl DeCombParams {
    pub fn new(iterations: i32) -> Self {
        DeCombParams { kind: 1, end1: 0, start2: 1, end2: 5, repeat2: 1, iterations2: iterations, smooth: 0.5, mix_pow: 2.0, mix_color: 0 }
    }
}

/// Which side of the fractal surface is rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsideMode {
    Outside,
    Inside,
    /// inside where the start point is in the set, outside elsewhere
    Both,
}
