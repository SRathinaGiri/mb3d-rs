//! Animation: keyframes, the sub-frame schedule and the interpolation of
//! the parameters between keyframes (Animation.pas, Interpolation.pas).
//!
//! Like MB3D, a keyframe holds a complete parameter set and the number of
//! sub-frames (`KFcount`) from it to the next keyframe.  A frame is
//! calculated from the parameters of the keyframe it belongs to (everything
//! that is not interpolated comes from there), with the interpolated values
//! of `Interpolate2frames` (linear) or `Interpolate3framesBezier`
//! ("quadratic bezier") put on top:
//!
//! * camera: start/end plane, middle point and field of view linearly,
//!   zoom logarithmically, the view matrix as quaternion (slerp),
//! * iterations, DE stop, raystep, julia values, cutting planes, 4D
//!   rotation angles, DOF, shadow and AO lengths, bailout (logarithmic),
//! * formula options of hybrid slots that hold the same formula in the
//!   keyframes (angle options turn the short way round),
//! * the light values (`TLightVals`): colours, amounts, light directions,
//!   positional lights, palette, fog, gamma (see [`LightBlend`] and
//!   `LightVals::blend`).
//!
//! The quadratic Bezier is MB3D's: the curve runs through the middle points
//! between the keyframes and is pulled towards the keyframes, so the frame of
//! a keyframe is not exactly that keyframe (except at the ends of a linear
//! animation); the sub-frame position is corrected for different frame
//! counts of neighbouring keyframes so the speed stays continuous.

use crate::math::{
    bezier_quat, delphi_round, matrix_to_quat, rotation_matrix, slerp_quat, Mat3, Quat,
};
use crate::scene::Scene;
use std::f64::consts::PI;
use std::sync::Arc;

/// Sub-frame interpolation (`RadioGroup2`: 0 linear, 1 quadratic bezier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    Linear = 0,
    Bezier = 1,
}

/// Output of the frames (`RadioGroup3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Bmp = 0,
    Png = 1,
    Jpg = 2,
    /// Parameter files instead of images (to render the frames elsewhere)
    M3p = 3,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Bmp => "bmp",
            OutputFormat::Png => "png",
            OutputFormat::Jpg => "jpg",
            OutputFormat::M3p => "m3p",
        }
    }

    pub fn parse(s: &str) -> Result<OutputFormat, String> {
        Ok(match s.trim().to_ascii_lowercase().trim_start_matches('.') {
            "bmp" => OutputFormat::Bmp,
            "png" => OutputFormat::Png,
            "jpg" | "jpeg" => OutputFormat::Jpg,
            "m3p" | "params" | "parameters" => OutputFormat::M3p,
            o => return Err(format!("unknown output format '{o}' (png, bmp, m3p)")),
        })
    }
}

/// One keyframe (`TKeyFrame`).
#[derive(Clone, Debug)]
pub struct Keyframe {
    pub scene: Scene,
    /// Sub-frames from this keyframe to the next one (`KFcount`)
    pub frames: u32,
    /// Calculation time of the preview image in ms (`KFtime`, informative)
    pub time_ms: i32,
    /// `KFsmooth` (unused by MB3D 1.9, kept for `.m3a` files)
    pub smooth: i32,
    /// File the keyframe was loaded from (animation text files may refer to
    /// parameter files instead of holding the parameters)
    pub source: Option<String>,
}

impl Keyframe {
    pub fn new(scene: Scene, frames: u32) -> Keyframe {
        Keyframe { scene, frames, time_ms: 0, smooth: 1, source: None }
    }
}

/// An animation project (MB3D's animation window and its `.m3a` file).
#[derive(Clone, Debug)]
pub struct Animation {
    pub keyframes: Vec<Keyframe>,
    /// Size of the output frames (`AniWidth`, `AniHeight`)
    pub width: i32,
    pub height: i32,
    /// Frames are calculated this many times larger and reduced (`AniScale`,
    /// MB3D's "image scale" anti-aliasing)
    pub scale: u32,
    pub interpolation: Interpolation,
    /// Loop animation: after the last keyframe comes the first again
    pub looped: bool,
    /// Output folder and project name; frame files are
    /// `<folder>/<name><6 digit index>.<ext>` like in MB3D
    pub output_folder: String,
    pub name: String,
    pub format: OutputFormat,
    /// File index of the first frame (`Edit3`) and the index step (`Edit4`)
    pub start_index: i64,
    pub index_step: i64,
    /// Overwrite existing images (`CheckBox6`); otherwise frames whose file
    /// exists are skipped
    pub overwrite: bool,
    /// Save a depth (z-buffer) image too (`CheckBox7`)
    pub save_depth: bool,
    /// PNG frames with a transparent background (not in MB3D's files)
    pub transparent: bool,
    /// Stereo options of `.m3a` files: 0x40 stereo animation (a right and
    /// a left eye image per frame), 0x80 only the "very left" image
    pub stereo_bits: u32,
}

impl Default for Animation {
    fn default() -> Self {
        Animation {
            keyframes: Vec::new(),
            width: 640,
            height: 480,
            scale: 1,
            interpolation: Interpolation::Bezier,
            looped: false,
            output_folder: String::new(),
            name: "new".into(),
            format: OutputFormat::Png,
            start_index: 1,
            index_step: 1,
            overwrite: true,
            save_depth: false,
            transparent: false,
            stereo_bits: 0,
        }
    }
}

/// Position of a frame: keyframe index and sub-frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramePos {
    pub key: usize,
    pub sub: u32,
}

/// The light part of an interpolated frame: the keyframes whose light
/// values are blended and the weights (MB3D interpolates the derived
/// `TLightVals`, not the light sliders).  Set on the frame's scene and used
/// by the renderer when it prepares the light values.
#[derive(Debug)]
pub struct LightBlend {
    /// The keyframe scenes (2 for linear, 3 for bezier interpolation)
    pub keys: Vec<Scene>,
    /// Weights of the keyframes
    pub weights: Vec<f64>,
    /// Parameter of the spherical interpolations (directions, rotations)
    pub t: f64,
    pub bezier: bool,
}

impl Animation {
    /// Number of frames (`TotalBMPsToRender`): the sub-frames of all
    /// keyframes that have a successor, plus the last keyframe itself in a
    /// linear (not looped) animation.
    pub fn frame_count(&self) -> usize {
        let n = self.keyframes.len();
        if n == 0 {
            return 0;
        }
        let segs = if self.looped { n } else { n - 1 };
        let mut c: usize = self.keyframes[..segs].iter().map(|k| k.frames as usize).sum();
        if !self.looped && self.keyframes[n - 1].frames > 0 {
            c += 1;
        }
        c
    }

    /// Keyframe and sub-frame of frame `frame` (0-based).
    pub fn frame_pos(&self, frame: usize) -> Option<FramePos> {
        let n = self.keyframes.len();
        let mut f = frame;
        for (k, kf) in self.keyframes.iter().enumerate() {
            if k == n - 1 && !self.looped {
                return (f == 0 && kf.frames > 0).then_some(FramePos { key: k, sub: 0 });
            }
            if f < kf.frames as usize {
                return Some(FramePos { key: k, sub: f as u32 });
            }
            f -= kf.frames as usize;
        }
        None
    }

    /// First frame of keyframe `key` (`SubFramesToKF`).
    pub fn first_frame_of(&self, key: usize) -> usize {
        self.keyframes[..key.min(self.keyframes.len())].iter().map(|k| k.frames as usize).sum()
    }

    /// File index of a frame.
    pub fn file_index(&self, frame: usize) -> i64 {
        self.start_index + frame as i64 * self.index_step
    }

    /// Frame of a file index (None if the index is not one of the frames).
    pub fn frame_of_index(&self, index: i64) -> Option<usize> {
        let step = self.index_step.max(1);
        let d = index - self.start_index;
        (d >= 0 && d % step == 0).then(|| (d / step) as usize).filter(|&f| f < self.frame_count())
    }

    /// Output file of a frame: `<folder>/<name><index, 6 digits>.<ext>`.
    pub fn frame_file(&self, frame: usize) -> std::path::PathBuf {
        self.frame_file_eye(frame, self.stereo_eyes()[0].1)
    }

    /// The images of one frame: (`bStereoMode`, name suffix).  A stereo
    /// animation renders the right eye, then the left one
    /// (`AniRightImage`); "very left" renders only that one.
    pub fn stereo_eyes(&self) -> Vec<(u8, &'static str)> {
        if self.stereo_bits & 0x40 == 0 {
            vec![(0, "")]
        } else if self.stereo_bits & 0x80 != 0 {
            vec![(1, "Left")]
        } else {
            vec![(3, "Right"), (4, "Left")]
        }
    }

    /// The file of one eye's image (`<name>Right<index>` etc.).
    pub fn frame_file_eye(&self, frame: usize, eye: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(&self.output_folder);
        p.push(format!("{}{eye}{:06}.{}", self.name, self.file_index(frame), self.format.extension()));
        p
    }

    /// Depth image of a frame (`ZBuf <name><index>.png`).
    pub fn depth_file(&self, frame: usize) -> std::path::PathBuf {
        self.depth_file_eye(frame, self.stereo_eyes()[0].1)
    }

    pub fn depth_file_eye(&self, frame: usize, eye: &str) -> std::path::PathBuf {
        let mut p = std::path::PathBuf::from(&self.output_folder);
        p.push(format!("ZBuf {}{eye}{:06}.png", self.name, self.file_index(frame)));
        p
    }

    /// Indices of the keyframes around keyframe `a` (`Timer2Timer`):
    /// previous, this, next and the one after.
    fn neighbours(&self, a: usize) -> [usize; 4] {
        let n = self.keyframes.len();
        if self.looped {
            [(a + n - 1) % n, a, (a + 1) % n, (a + 2) % n]
        } else {
            let i0 = if a > 0 { a - 1 } else { a };
            let i2 = if a + 1 < n { a + 1 } else { a };
            let i3 = if a + 2 < n { a + 2 } else { i2 };
            [i0, a, i2, i3]
        }
    }

    /// The parameters of a frame, at the size of the animation (multiplied by
    /// `scale`; reduce the image by `scale` afterwards).
    pub fn frame_scene(&self, frame: usize) -> Result<Scene, String> {
        let pos = self.frame_pos(frame).ok_or_else(|| format!("frame {} is not part of the animation", frame + 1))?;
        Ok(self.scene_at(pos))
    }

    /// The parameters at keyframe `pos.key`, sub-frame `pos.sub`.
    pub fn scene_at(&self, pos: FramePos) -> Scene {
        let [i0, i1, i2, i3] = self.neighbours(pos.key);
        let k = &self.keyframes;
        let count = |i: usize| k[i].frames.max(1) as f64;
        let t = pos.sub as f64 / count(i1);
        let mut s = match self.interpolation {
            Interpolation::Linear => {
                if i1 != i2 {
                    interpolate_linear(&k[i1].scene, &k[i2].scene, t)
                } else {
                    k[i1].scene.clone()
                }
            }
            Interpolation::Bezier => {
                let scenes = [&k[i0].scene, &k[i1].scene, &k[i2].scene, &k[i3].scene];
                interpolate_bezier(scenes, [count(i0), count(i1), count(i2), count(i3)], t)
            }
        };
        s.width = self.width.max(1);
        s.height = self.height.max(1);
        s.tiling = None;
        s.calc_rect = None;
        if self.scale > 1 {
            s.scale_image(self.scale as f64);
        }
        s
    }

    /// Takes the image size from a scene (the first keyframe in MB3D).
    pub fn set_size_from(&mut self, s: &Scene) {
        self.width = s.width.max(1);
        self.height = s.height.max(1);
    }
}

// ---------------------------------------------------------------------------
// interpolation of the header values

/// `while Abs(v - r) - eps > half do v := v +- period`: brings `v` to within
/// half a period of `r`.
fn wrap_to(r: f64, mut v: f64, period: f64, eps: f64) -> f64 {
    let mut n = 0;
    while (v - r).abs() - eps > period * 0.5 && n < 100_000 {
        v += if v < r { period } else { -period };
        n += 1;
    }
    v
}

/// Values of the parameters of the keyframes `h` that MB3D interpolates,
/// combined with the weights `w`, written into `base`.  `rot` is the
/// interpolated view rotation; `bezier` selects the angle wrapping of
/// `Interpolate3framesBezier` (around the middle keyframe) instead of
/// `Interpolate2frames` (around the first).
fn interpolate_header(base: &Scene, h: &[&Scene], w: &[f64], rot: Quat, bezier: bool) -> Scene {
    let mut s = base.clone();
    let lin = |f: &dyn Fn(&Scene) -> f64| -> f64 { h.iter().zip(w).map(|(x, w)| f(x) * w).sum() };
    // Single: the result is stored in single precision
    let lin_s = |f: &dyn Fn(&Scene) -> f64| -> f64 { lin(f) as f32 as f64 };
    let round = |f: &dyn Fn(&Scene) -> f64| -> i64 { delphi_round(lin(f)) };

    // bDFogIt (byte) and the integers: Iterations, MinimumIterations
    s.dfog_on_it = round(&|x| x.dfog_on_it as f64).clamp(0, 255) as u16;
    s.iterations = round(&|x| x.iterations as f64).max(1) as i32;
    s.min_iterations = round(&|x| x.min_iterations as f64).clamp(0, 65535) as i32;

    // doubles: dZstart, dZend, dXmid.., dFOVy, dJx..dJw, dCutX..
    s.z_start = lin(&|x| x.z_start);
    s.z_end = lin(&|x| x.z_end);
    for i in 0..3 {
        s.mid[i] = lin(&|x| x.mid[i]);
        s.cut_pos[i] = lin(&|x| x.cut_pos[i]);
    }
    s.fov_y = lin(&|x| x.fov_y);
    for i in 0..4 {
        s.julia_c[i] = lin(&|x| x.julia_c[i]);
    }

    // singles
    s.de_stop = lin_s(&|x| x.de_stop);
    s.z_step_div = lin_s(&|x| x.z_step_div).clamp(0.001, 1.0);
    s.raystep_limiter = lin_s(&|x| x.raystep_limiter);
    s.color_mul = lin_s(&|x| x.color_mul);
    if let Some(d) = s.dof.as_mut() {
        // DOF values of keyframes without DOF: aperture 0 (MB3D keeps the
        // other values in the header; here the frame's own are used)
        let b = *d;
        let dv = |x: &Scene| x.dof.unwrap_or(b);
        d.z_sharp = lin(&|x| dv(x).z_sharp as f64) as f32;
        d.z_sharp2 = lin(&|x| dv(x).z_sharp2 as f64) as f32;
        d.clip_r = (lin(&|x| dv(x).clip_r as f64) as f32).max(0.1);
        d.aperture = (lin(&|x| x.dof.map_or(0.0, |d| d.aperture as f64)) as f32).max(0.0001);
    }
    if let Some(hs) = s.shadows.as_mut() {
        let b = *hs;
        let hv = |x: &Scene| x.shadows.unwrap_or(b);
        hs.max_len_mul = (lin(&|x| hv(x).max_len_mul as f64) as f32).max(0.01);
        // MCSoftShadowRadius is a ShortFloat
        let r = lin(&|x| hv(x).soft_radius as f64) as f32;
        hs.soft_radius = crate::m3p::short_float(crate::m3p::to_short_float(r)).clamp(0.01, 20.0);
    }
    if let Some(ao) = s.ao.as_mut() {
        let b = *ao;
        ao.threshold = lin(&|x| x.ao.unwrap_or(b).threshold as f64) as f32;
    }
    if let Some(d) = s.deao.as_mut() {
        let b = *d;
        d.max_len = (lin(&|x| x.deao.unwrap_or(b).max_len as f64) as f32).max(0.01);
    }
    if let Some(d) = s.decomb.as_mut() {
        let b = *d;
        d.smooth = lin(&|x| x.decomb.unwrap_or(b).smooth as f64) as f32;
    }

    // logarithmic: dZoom, RStop
    let log = |f: &dyn Fn(&Scene) -> f64| -> f64 { 10f64.powf(lin(&|x| f(x).max(1e-10).log10())) };
    s.zoom = log(&|x| x.zoom);
    if h.iter().any(|x| x.rstop.is_some()) {
        s.rstop = Some(log(&|x| x.effective_rstop()));
    }

    // 4D rotation angles (radians)
    for i in 0..3 {
        let mut d: Vec<f64> = h.iter().map(|x| if x.rot_4d[i].abs() > 1000.0 { 0.0 } else { x.rot_4d[i] }).collect();
        if bezier {
            d[0] = wrap_to(d[1], d[0], 2.0 * PI, 1e-8);
            d[2] = wrap_to(d[1], d[2], 2.0 * PI, 1e-8);
        } else {
            d[1] = wrap_to(d[0], d[1], 2.0 * PI, 1e-8);
        }
        s.rot_4d[i] = d.iter().zip(w).map(|(d, w)| d * w).sum();
    }

    // view matrix
    s.vgrads = rotation_matrix(&rot);

    // formulas (THeaderCustomAddon)
    interpolate_formulas(&mut s, h, w);
    s
}

/// `isAngleType`: option types 3..6 and 12 are angles in degrees.
fn is_angle_type(ty: u8) -> bool {
    matches!(ty, 3..=6 | 12)
}

/// Option values and their angle flags of a formula slot.
fn formula_values(f: &crate::formulas::Formula) -> Vec<(f64, bool)> {
    match f {
        crate::formulas::Formula::Custom(c) => {
            c.values.iter().zip(&c.def.options).map(|(v, o)| (*v, is_angle_type(o.ty))).collect()
        }
        _ => f.options().into_iter().map(|(_, v)| (v, false)).collect(),
    }
}

fn set_formula_value(f: &mut crate::formulas::Formula, i: usize, v: f64) {
    match f {
        crate::formulas::Formula::Custom(c) => {
            if let Some(x) = c.values.get_mut(i) {
                *x = v;
            }
        }
        _ => {
            if let Some((name, _)) = f.options().into_iter().nth(i) {
                let _ = f.set_option(&name, v);
            }
        }
    }
}

/// `bInterpolateFormula` for all keyframes against the first, then the
/// option values and iteration counts of the matching slots.
fn interpolate_formulas(s: &mut Scene, h: &[&Scene], w: &[f64]) {
    let same_kind = h.iter().all(|x| x.decomb.is_some() == h[0].decomb.is_some() && x.interpolation.is_some() == h[0].interpolation.is_some());
    if !same_kind {
        return;
    }
    let ipol = h[0].interpolation.is_some();
    fn slot(x: &Scene, i: usize) -> Option<(String, &crate::scene::FormulaEntry)> {
        x.formulas.get(i).filter(|e| e.iterations > 0).map(|e| (e.formula.name(), e))
    }
    for i in 0..s.formulas.len().min(6) {
        let Some((name0, _)) = slot(h[0], i) else { continue };
        let entries: Vec<_> = h.iter().map(|x| slot(x, i)).collect();
        if entries.iter().any(|e| e.as_ref().is_none_or(|(n, _)| *n != name0)) {
            continue;
        }
        let vals: Vec<Vec<(f64, bool)>> = entries.iter().map(|e| formula_values(&e.as_ref().unwrap().1.formula)).collect();
        let n = vals.iter().map(Vec::len).min().unwrap_or(0).min(16);
        for o in 0..n {
            let angle = vals[0][o].1;
            let mut d: Vec<f64> = vals.iter().map(|v| v[o].0).collect();
            if angle {
                for x in d.iter_mut() {
                    if x.abs() > 10000.0 {
                        *x = 0.0;
                    }
                }
                // D2 towards D1, D3 towards D2 (both interpolations)
                for k in 1..d.len() {
                    d[k] = wrap_to(d[k - 1], d[k], 360.0, 0.1);
                }
            }
            let v: f64 = d.iter().zip(w).map(|(d, w)| d * w).sum();
            set_formula_value(&mut s.formulas[i].formula, o, v);
        }
        if ipol {
            // the weights are interpolated as singles
            if i < 2 {
                let v: f64 = h.iter().zip(w).map(|(x, w)| x.interpolation.unwrap_or_default()[i] as f64 * w).sum();
                if let Some(iw) = s.interpolation.as_mut() {
                    iw[i] = v as f32;
                }
            }
            continue;
        }
        let its: f64 = entries.iter().zip(w).map(|(e, w)| e.as_ref().unwrap().1.iterations as f64 * w).sum();
        s.formulas[i].iterations = delphi_round(its).max(1) as i32;
    }
}

/// `Interpolate2frames`: linear interpolation from `h1` (t = 0) to `h2`
/// (t = 1); everything else comes from `h1`.
pub fn interpolate_linear(h1: &Scene, h2: &Scene, t: f64) -> Scene {
    let w = [1.0 - t, t];
    let rot = slerp_quat(&matrix_to_quat(&h1.vgrads), &matrix_to_quat(&h2.vgrads), t);
    let mut s = interpolate_header(h1, &[h1, h2], &w, rot, false);
    s.light_blend = Some(Arc::new(LightBlend {
        keys: vec![strip(h1), strip(h2)],
        weights: w.to_vec(),
        t,
        bezier: false,
    }));
    s
}

/// The weights of `Interpolate3framesBezier` for sub-frame position `t`
/// (0..1) between keyframes 1 and 2 of `counts` (frame counts of keyframes
/// 0..3): which three keyframes are used (0 = keyframes 0..2, 1 = 1..3),
/// their weights and the curve parameter.
pub fn bezier_weights(counts: [f64; 4], t: f64) -> (usize, [f64; 3], f64) {
    let (first, w1, w2, w3) = if t >= 0.5 {
        (1, counts[1] * 0.5, counts[1], counts[1] + counts[2] * 0.5)
    } else {
        (0, counts[0] * -0.5, 0.0, counts[1] * 0.5)
    };
    let d1 = w1 - 2.0 * w2 + w3;
    let d3 = counts[1] * t - w1;
    let ts = if d1.abs() < 0.0001 {
        d3 / (w3 - w1)
    } else {
        let d2 = w1 - w2;
        let d3 = (d2 * d2 + d1 * d3).abs().sqrt();
        let ts = (d2 + d3) / d1;
        if !(0.0..=1.0).contains(&ts) {
            (d2 - d3) / d1
        } else {
            ts
        }
    };
    let a = 0.5 * (1.0 - ts) * (1.0 - ts);
    let b = a + 2.0 * ts * (1.0 - ts) + 0.5 * ts * ts;
    (first, [a, b, 1.0 - a - b], ts)
}

/// `Interpolate3framesBezier`: sub-frame `t` between keyframes `h[1]` and
/// `h[2]`, with their neighbours `h[0]` and `h[3]` and the keyframes' frame
/// counts; everything not interpolated comes from `h[1]`.
pub fn interpolate_bezier(h: [&Scene; 4], counts: [f64; 4], t: f64) -> Scene {
    let (first, w, ts) = bezier_weights(counts, t);
    let k = [h[first], h[first + 1], h[first + 2]];
    let q: Vec<Quat> = k.iter().map(|x| matrix_to_quat(&x.vgrads)).collect();
    let rot = bezier_quat(&q[0], &q[1], &q[2], ts);
    let mut s = interpolate_header(h[1], &k, &w, rot, true);
    s.light_blend = Some(Arc::new(LightBlend {
        keys: k.iter().map(|x| strip(x)).collect(),
        weights: w.to_vec(),
        t: ts,
        bezier: true,
    }));
    s
}

/// Writes the light blend of a frame into its light settings, so the frame
/// can be saved as a parameter file: the light sliders, colours, palette and
/// light angles/positions of the keyframes are interpolated with the frame's
/// weights.  This approximates the blended light values (MB3D itself writes
/// the light settings of the frame's keyframe unchanged).
pub fn bake_light_blend(s: &mut Scene) {
    let Some(b) = s.light_blend.take() else { return };
    let w = &b.weights;
    let ls: Vec<&crate::lighting::Lighting> = b.keys.iter().map(|k| &k.lighting).collect();
    let l = &mut s.lighting;
    let f = |g: &dyn Fn(&crate::lighting::Lighting) -> f32| -> f32 { ls.iter().zip(w).map(|(x, w)| g(x) * *w as f32).sum() };
    let c = |g: &dyn Fn(&crate::lighting::Lighting) -> [u8; 3]| -> [u8; 3] {
        let mut r = [0f64; 3];
        for (x, w) in ls.iter().zip(w) {
            for (k, v) in r.iter_mut().enumerate() {
                *v += g(x)[k] as f64 * w;
            }
        }
        r.map(|v| v.round().clamp(0.0, 255.0) as u8)
    };
    l.amb_top = c(&|x| x.amb_top);
    l.amb_bottom = c(&|x| x.amb_bottom);
    l.depth_col = c(&|x| x.depth_col);
    l.depth_col2 = c(&|x| x.depth_col2);
    l.dyn_fog_col = c(&|x| x.dyn_fog_col);
    l.dyn_fog_col2 = c(&|x| x.dyn_fog_col2);
    l.fog_offset = f(&|x| x.fog_offset);
    l.depth_fog = f(&|x| x.depth_fog);
    l.diffuse = f(&|x| x.diffuse);
    l.dyn_fog = f(&|x| x.dyn_fog);
    l.specular = f(&|x| x.specular);
    l.ambient = f(&|x| x.ambient);
    l.color_start = f(&|x| x.color_start);
    l.color_end = f(&|x| x.color_end);
    l.amb_shadow = f(&|x| x.amb_shadow);
    l.ind_light = f(&|x| x.ind_light);
    l.gamma = f(&|x| x.gamma).round();
    l.roughness = f(&|x| x.roughness);
    l.var_col_z = f(&|x| x.var_col_z);
    l.interior_start = f(&|x| x.interior_start);
    l.interior_end = f(&|x| x.interior_end);
    l.diffuse_shadowing = f(&|x| x.diffuse_shadowing);
    for i in 0..10 {
        l.palette[i].diffuse = c(&|x| x.palette[i].diffuse);
        l.palette[i].specular = c(&|x| x.palette[i].specular);
        l.palette[i].position = f(&|x| x.palette[i].position as f32).round().clamp(0.0, 32767.0) as u16;
    }
    for i in 0..4 {
        l.interior[i].1 = c(&|x| x.interior[i].1);
        l.interior[i].0 = f(&|x| x.interior[i].0 as f32).round().clamp(0.0, 32767.0) as u16;
    }
    for i in 0..6 {
        let on_any = ls.iter().any(|x| x.lights[i].on);
        if !on_any {
            continue;
        }
        let li = &mut l.lights[i];
        li.on = true;
        // an off light fades: its amplitude counts as 0
        li.amplitude = f(&|x| if x.lights[i].on { x.lights[i].amplitude } else { 0.0 });
        li.color = c(&|x| x.lights[i].color);
        if ls.iter().all(|x| x.lights[i].positional) {
            for k in 0..3 {
                li.position[k] = ls.iter().zip(w).map(|(x, w)| x.lights[i].position[k] * w).sum();
            }
        } else {
            let mut xa: Vec<f64> = ls.iter().map(|x| x.lights[i].x_angle).collect();
            let mut ya: Vec<f64> = ls.iter().map(|x| x.lights[i].y_angle).collect();
            for k in 1..xa.len() {
                xa[k] = wrap_to(xa[k - 1], xa[k], 2.0 * PI, 0.0);
                ya[k] = wrap_to(ya[k - 1], ya[k], 2.0 * PI, 0.0);
            }
            li.x_angle = xa.iter().zip(w).map(|(a, w)| a * w).sum();
            li.y_angle = ya.iter().zip(w).map(|(a, w)| a * w).sum();
        }
    }
}

/// A keyframe scene for the light blend (without its own blend).
fn strip(s: &Scene) -> Scene {
    let mut s = s.clone();
    s.light_blend = None;
    s
}

/// The view matrix of a frame as a unit matrix (for tests and the editor).
pub fn unit_vgrads(s: &Scene) -> Mat3 {
    crate::math::normalise_matrix_to(1.0, &s.vgrads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::vgrads_from_angles;

    fn key(zoom: f64, x: f64, rot: f64, frames: u32) -> Keyframe {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.zoom = zoom;
        s.mid = [x, 0.0, 0.0];
        s.vgrads = vgrads_from_angles(rot, 0.0, 0.0);
        s.width = 64;
        s.height = 48;
        Keyframe::new(s, frames)
    }

    fn anim(keys: Vec<Keyframe>, ipol: Interpolation, looped: bool) -> Animation {
        let mut a = Animation { keyframes: keys, interpolation: ipol, looped, ..Default::default() };
        a.width = 64;
        a.height = 48;
        a
    }

    #[test]
    fn frame_schedule() {
        let a = anim(vec![key(1.0, 0.0, 0.0, 10), key(1.0, 1.0, 0.0, 5), key(1.0, 2.0, 0.0, 7)], Interpolation::Linear, false);
        assert_eq!(a.frame_count(), 16);
        assert_eq!(a.frame_pos(0), Some(FramePos { key: 0, sub: 0 }));
        assert_eq!(a.frame_pos(9), Some(FramePos { key: 0, sub: 9 }));
        assert_eq!(a.frame_pos(10), Some(FramePos { key: 1, sub: 0 }));
        assert_eq!(a.frame_pos(15), Some(FramePos { key: 2, sub: 0 }));
        assert_eq!(a.frame_pos(16), None);
        let mut l = a.clone();
        l.looped = true;
        assert_eq!(l.frame_count(), 22);
        assert_eq!(l.frame_pos(21), Some(FramePos { key: 2, sub: 6 }));
        // keyframes with 0 frames are skipped
        let z = anim(vec![key(1.0, 0.0, 0.0, 3), key(1.0, 0.0, 0.0, 0), key(1.0, 0.0, 0.0, 2), key(1.0, 0.0, 0.0, 4)], Interpolation::Linear, false);
        assert_eq!(z.frame_count(), 6);
        assert_eq!(z.frame_pos(3), Some(FramePos { key: 2, sub: 0 }));
        assert_eq!(z.frame_pos(5), Some(FramePos { key: 3, sub: 0 }));
        assert_eq!(a.file_index(3), 4);
        assert_eq!(a.frame_of_index(4), Some(3));
        assert_eq!(a.frame_file(0).to_string_lossy(), "new000001.png");
    }

    #[test]
    fn linear_values() {
        let a = anim(vec![key(1.0, 0.0, 0.0, 4), key(100.0, 2.0, 1.0, 4)], Interpolation::Linear, false);
        let s0 = a.frame_scene(0).unwrap();
        assert!((s0.zoom - 1.0).abs() < 1e-12 && s0.mid[0].abs() < 1e-12);
        let s2 = a.frame_scene(2).unwrap();
        assert!((s2.mid[0] - 1.0).abs() < 1e-12);
        // zoom logarithmically
        assert!((s2.zoom - 10.0).abs() < 1e-9, "{}", s2.zoom);
        // rotation halfway
        let m = vgrads_from_angles(0.5, 0.0, 0.0);
        let r = unit_vgrads(&s2);
        for i in 0..3 {
            for j in 0..3 {
                assert!((r[i][j] - m[i][j]).abs() < 1e-6);
            }
        }
        // the last frame is the last keyframe
        let s4 = a.frame_scene(4).unwrap();
        assert!((s4.zoom - 100.0).abs() < 1e-9 && (s4.mid[0] - 2.0).abs() < 1e-12);
        assert!(s4.light_blend.is_none());
    }

    #[test]
    fn bezier_weights_continuous() {
        // equal counts: at t = 0 the B-spline weights 1/8, 3/4, 1/8
        let (f, w, ts) = bezier_weights([10.0; 4], 0.0);
        assert_eq!(f, 0);
        assert!((ts - 0.5).abs() < 1e-12);
        assert!((w[0] - 0.125).abs() < 1e-12 && (w[1] - 0.75).abs() < 1e-12);
        // the weights of keyframe 1 / 2 are continuous at t = 0.5
        let (_, a, _) = bezier_weights([10.0, 20.0, 5.0, 7.0], 0.4999999);
        let (_, b, _) = bezier_weights([10.0, 20.0, 5.0, 7.0], 0.5);
        assert!((a[1] - b[0]).abs() < 1e-5 && (a[2] - b[1]).abs() < 1e-5, "{a:?} {b:?}");
        // a position runs monotonically through a sequence of keyframes
        let keys: Vec<Keyframe> = (0..5).map(|i| key(1.0, i as f64 * i as f64, 0.0, 3 + i as u32 * 4)).collect();
        let a = anim(keys, Interpolation::Bezier, false);
        let xs: Vec<f64> = (0..a.frame_count()).map(|f| a.frame_scene(f).unwrap().mid[0]).collect();
        for p in xs.windows(2) {
            assert!(p[1] >= p[0] - 1e-12, "{xs:?}");
        }
        // velocity has no big jumps at keyframes despite different counts
        let v: Vec<f64> = xs.windows(2).map(|p| p[1] - p[0]).collect();
        for p in v.windows(2) {
            assert!((p[1] - p[0]).abs() < 0.6, "{v:?}");
        }
    }

    #[test]
    fn angles_short_way() {
        let mut k1 = key(1.0, 0.0, 0.0, 2);
        let mut k2 = key(1.0, 0.0, 0.0, 2);
        k1.scene.rot_4d = [3.0, 0.0, 0.0];
        k2.scene.rot_4d = [-3.0, 0.0, 0.0];
        let a = anim(vec![k1, k2], Interpolation::Linear, false);
        let s = a.frame_scene(1).unwrap();
        // 3 and -3 rad meet at pi, not at 0
        assert!((s.rot_4d[0] - PI).abs() < 1e-9, "{}", s.rot_4d[0]);
    }

    #[test]
    fn formula_options() {
        let mut k1 = key(1.0, 0.0, 0.0, 2);
        let mut k2 = key(1.0, 0.0, 0.0, 2);
        k1.scene.formulas = vec![crate::scene::FormulaEntry { formula: crate::formulas::lookup("Amazing Box").unwrap(), iterations: 2 }];
        k2.scene.formulas = k1.scene.formulas.clone();
        k2.scene.formulas[0].formula.set_option("Scale", 3.0).unwrap();
        k2.scene.formulas[0].iterations = 4;
        let a = anim(vec![k1, k2.clone()], Interpolation::Linear, false);
        let s = a.frame_scene(1).unwrap();
        assert_eq!(s.formulas[0].formula.options()[0].1, 2.5);
        assert_eq!(s.formulas[0].iterations, 3);
        // different formulas: no interpolation
        let mut k3 = k2;
        k3.scene.formulas[0].formula = crate::formulas::lookup("Bulbox").unwrap();
        let mut k1 = key(1.0, 0.0, 0.0, 2);
        k1.scene.formulas = vec![crate::scene::FormulaEntry { formula: crate::formulas::lookup("Amazing Box").unwrap(), iterations: 2 }];
        let a = anim(vec![k1, k3], Interpolation::Linear, false);
        assert_eq!(a.frame_scene(1).unwrap().formulas[0].formula.name(), "Amazing Box");
    }

    #[test]
    fn scale_multiplies_size() {
        let mut a = anim(vec![key(1.0, 0.0, 0.0, 2), key(2.0, 0.0, 0.0, 2)], Interpolation::Bezier, true);
        a.scale = 2;
        let s = a.frame_scene(3).unwrap();
        assert_eq!((s.width, s.height), (128, 96));
    }
}
