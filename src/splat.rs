//! Gaussian splat export (not in MB3D): the object calculated from many
//! directions around the scene middle, every object pixel of every view a
//! flat Gaussian lying on the surface with the pixel's colour, written as
//! the PLY file of 3D Gaussian splatting (the layout of the INRIA training
//! code that Blender, SuperSplat and the other splat viewers read).
//!
//! The views are the scene's own camera turned around the middle (as the
//! turntable does), spread evenly over a sphere, the first one the scene's
//! view itself.  Each view is square; its field of view is the scene's
//! widest, so what the scene shows across is in it.  The lights that turn
//! with the camera in MB3D are fixed to the object as they light the first
//! view, so every side is lit alike, and so are the ambient light's top
//! and bottom colours.  The depth of field, the clouds, the
//! glow, the vignette and the colour fringes are left out (they belong to a
//! picture, not to the object), and so is the ground plane.
//!
//! A splat's size is the pixel's footprint on the surface: one pixel across
//! where the surface faces the camera, stretched along the slope where it
//! is seen at an angle.  Where views overlap, one view keeps each patch of
//! the surface (the one that saw it most squarely), so that the views'
//! slightly different pictures do not mix into speckles; and the colours
//! change with the direction they are seen from (spherical harmonics fitted
//! to what the views saw), so that highlights move as in the pictures.
//!
//! The file is in the first view's frame: x to the right, y into the
//! picture, z up (Blender's front view shows the scene's picture), the
//! scene middle at 0 and the object scaled to fit a sphere of radius 1.

use crate::gbuffer::SiLight;
use crate::lighting::Lighting;
use crate::math::{normalise_matrix_to, rotate_vector, Mat3, Vec3};
use crate::scene::{CameraOptic, Scene};
use std::collections::{HashMap, HashSet};
use std::io::Write;

/// `SH_C0`: the colour of a splat is 0.5 + SH_C0 * f_dc.
const SH_C0: f64 = 0.282_094_791_773_878_14;

/// The settings of a splat export.
#[derive(Clone, Debug)]
pub struct SplatParams {
    /// The number of directions the object is calculated from.
    pub views: usize,
    /// The width and height of each view in pixels.
    pub size: i32,
    /// A splat's width as a part of the pixel footprint (the Gaussian's
    /// standard deviation; with the half pixel that splat renderers add
    /// for their anti-aliasing, 0.5 closes the gaps without blurring).
    pub sigma: f64,
    /// The degree of the spherical harmonics (0..3) that let the colour
    /// change with the direction it is seen from (highlights, reflections):
    /// 0 one colour per splat; each degree more follows the light more
    /// closely and makes the file larger.
    pub degree: u8,
    /// Where the views are calculated from.
    pub around: Around,
}

impl Default for SplatParams {
    fn default() -> Self {
        SplatParams { views: 16, size: 640, sigma: 0.5, degree: 2, around: Around::Auto }
    }
}

/// Where the views of a splat export are calculated from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Around {
    /// The camera goes around the scene middle, looking at it: an object
    /// seen from outside.
    Middle,
    /// The camera stays where it is and looks in every direction: a scene
    /// seen from inside (a cave, a landscape, a hall), its views
    /// [`CAMERA_FOV`] degrees wide (16 views or more leave no gaps).
    Camera,
    /// [`Around::Camera`] when the object fills the scene's whole picture,
    /// else [`Around::Middle`].
    Auto,
}

/// The field of view of the views around the camera: of 16 directions,
/// every direction is within 46 degrees of one, and a square picture this
/// wide reaches 55 degrees from its middle even towards its sides.
pub const CAMERA_FOV: f64 = 110.0;

impl Around {
    pub fn name(self) -> &'static str {
        match self {
            Around::Middle => "middle",
            Around::Camera => "camera",
            Around::Auto => "auto",
        }
    }

    pub fn parse(s: &str) -> Option<Around> {
        match s.trim().to_ascii_lowercase().as_str() {
            "middle" | "object" => Some(Around::Middle),
            "camera" | "inside" => Some(Around::Camera),
            "auto" => Some(Around::Auto),
            _ => None,
        }
    }
}

/// A splat in scene units relative to the scene middle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Splat {
    pub pos: Vec3,
    /// unit surface normal, towards the camera that saw it
    pub normal: Vec3,
    /// the colour as shown, 0..1; with spherical harmonics the colour
    /// without them (the constant part, may be outside 0..1)
    pub color: [f32; 3],
    /// unit direction from the splat to the camera that saw it
    pub eye: Vec3,
    /// the spherical harmonics of degree 1..3 (in the file's frame)
    pub rest: [[f32; 3]; 15],
    /// unit direction of the long axis (in the surface)
    pub along: Vec3,
    /// standard deviations along `along`, across it in the surface, and
    /// along the normal
    pub sigma: [f64; 3],
    /// the view it came from and how squarely that view saw it (the cosine
    /// between the ray and the normal)
    pub view: u16,
    pub facing: f32,
}

/// `n` directions spread evenly over the sphere (a Fibonacci lattice) in
/// the first view's frame (x right, y down the picture, z the view
/// direction), turned so that the first one is the view direction itself.
pub fn view_dirs(n: usize) -> Vec<Vec3> {
    let n = n.max(1);
    if n == 1 {
        return vec![[0.0, 0.0, 1.0]];
    }
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    let pts: Vec<Vec3> = (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
            let r = (1.0 - y * y).max(0.0).sqrt();
            let a = golden * i as f64;
            [r * a.cos(), y, r * a.sin()]
        })
        .collect();
    // the point nearest the equator in front first, then everything turned
    // so that it is the view direction (Rodrigues about their cross product)
    let first = (0..n).min_by(|&a, &b| pts[a][1].abs().total_cmp(&pts[b][1].abs())).unwrap_or(0);
    let p0 = pts[first];
    let z = [0.0, 0.0, 1.0];
    let k = cross(p0, z);
    let s = len(k);
    let c = dot(p0, z);
    let turn = |v: Vec3| -> Vec3 {
        if s < 1e-12 {
            return if c > 0.0 { v } else { [-v[0], v[1], -v[2]] };
        }
        let k = k.map(|x| x / s);
        let kv = cross(k, v);
        let kd = dot(k, v);
        [0, 1, 2].map(|i| v[i] * c + kv[i] * s + k[i] * kd * (1.0 - c))
    };
    let mut out = vec![turn(p0)];
    out.extend((0..n).filter(|&i| i != first).map(|i| turn(pts[i])));
    out[0] = z;
    out
}

/// The scene's camera turned around the middle to look along `dir` (in the
/// frame of the scene's view: x right, y down, z the view direction),
/// keeping the picture's up as near the scene's as the direction allows.
pub fn view_scene(sc: &Scene, dir: Vec3) -> Scene {
    let f = normalize(dir);
    let y = [0.0, 1.0, 0.0];
    let mut r = cross(y, f);
    if len(r) < 1e-6 {
        // straight up or down: the scene's view direction as the picture's up
        r = cross([0.0, 0.0, if f[1] > 0.0 { -1.0 } else { 1.0 }], f);
    }
    let r = normalize(r);
    let u = cross(f, r);
    let g = sc.vgrads;
    let l = [len(g[0]), len(g[1]), len(g[2])];
    let e = [0, 1, 2].map(|i| g[i].map(|v| v / l[i].max(1e-300)));
    let row = |a: Vec3, n: f64| -> Vec3 { [0, 1, 2].map(|k| n * (a[0] * e[0][k] + a[1] * e[1][k] + a[2] * e[2][k])) };
    let mut s = sc.clone();
    s.vgrads = [row(r, l[0]), row(u, l[1]), row(f, l[2])];
    s
}

/// The scene prepared for the views: square pictures of `size` pixels with
/// the scene's widest field of view, the lights fixed to the object as they
/// light the scene's view, and no picture effects (see the module notes).
pub fn prepare(sc: &Scene, size: i32) -> Scene {
    let mut s = sc.clone();
    let size = size.max(16);
    let aspect = sc.width.max(1) as f64 / sc.height.max(1) as f64;
    if aspect > 1.0 {
        s.fov_y = (sc.fov_y * aspect).min(170.0);
    }
    s.de_stop = (sc.de_stop * size as f64 / sc.width.max(1) as f64).max(0.001);
    s.width = size;
    s.height = size;
    if matches!(s.optic, CameraOptic::Panorama | CameraOptic::Fisheye) {
        s.optic = CameraOptic::Common;
    }
    s.stereo_mode = 0;
    s.tiling = None;
    s.calc_rect = None;
    s.light_blend = None;
    s.dof = None;
    s.clouds = None;
    s.ground = None;
    s.slice_2d = 0;
    if let Some(f) = s.finish.as_mut() {
        f.glow = 0.0;
        f.vignette = 0.0;
        f.chromatic = 0.0;
    }
    fix_lights(&mut s.lighting, &sc.vgrads);
    // the ambient light's top and bottom and the depth colours, also given
    // relative to the viewer in MB3D, fixed the same way
    if !s.lighting.amb_rel_obj {
        s.lighting.amb_rel_obj = true;
        let fov = if sc.optic == CameraOptic::Panorama { 180.0 } else { sc.fov_y.clamp(1.0, 179.0) };
        s.lighting.amb_frame = Some((normalise_matrix_to(1.0, &sc.vgrads), fov.to_radians()));
    }
    s
}

/// The lights given relative to the viewer made relative to the object,
/// pointing as they do in the view `vgrads`.
pub fn fix_lights(l: &mut Lighting, vgrads: &Mat3) {
    let m = normalise_matrix_to(1.0, vgrads);
    for li in l.lights.iter_mut().filter(|li| !li.positional && !li.relative_to_object) {
        let (xa, ya) = rotate_angles(li.x_angle, li.y_angle, &m);
        li.x_angle = xa;
        li.y_angle = ya;
        li.relative_to_object = true;
    }
}

/// The direction of a light's angles (as `LightVals` makes it: towards the
/// light, `BuildViewVectorFOV(LY, -LX)` negated, unnormalised).
fn light_vec(xa: f64, ya: f64) -> Vec3 {
    [-xa.sin(), -ya.sin(), -(ya.cos() * xa.cos())]
}

/// The angles of a viewer's light turned into the object's frame: the
/// object light's vector, turned back by the view (`rotate_object_lights`),
/// is the viewer light's vector.
fn rotate_angles(xa: f64, ya: f64, m: &Mat3) -> (f64, f64) {
    let v = normalize(light_vec(xa, ya));
    let o = rotate_vector(&v, m);
    // angles of -o: (sin xa, sin ya, cos ya cos xa) = k d, |d| = 1
    let d = normalize(o.map(|x| -x));
    let p = d[0] * d[0] * d[1] * d[1];
    let k2 = if p < 1e-15 { 1.0 } else { (1.0 - (1.0 - 4.0 * p).max(0.0).sqrt()) / (2.0 * p) };
    let k = k2.sqrt();
    let sx = (k * d[0]).clamp(-1.0, 1.0);
    let sy = (k * d[1]).clamp(-1.0, 1.0);
    let xa = if d[2] >= 0.0 { sx.asin() } else { std::f64::consts::PI - sx.asin() };
    (xa, sy.asin())
}

/// The splats of one calculated view: `gbuf` and `rgb` (as shown, 1 =
/// white) of the scene `sc` calculated with `p`.
/// `keep_cut`: the pixels where the ray met the object at once (it starts
/// inside it, the camera's start plane cuts it) are kept; else they are
/// left out (from any other view, that cut is somewhere else).
pub fn splats_of_view(sc: &Scene, p: &crate::calc::CalcParams, gbuf: &[SiLight], rgb: &[f32], view: u16, sigma: f64, keep_cut: bool) -> Vec<Splat> {
    let cam = crate::render::paint_camera(sc, p);
    let (w, h) = (sc.width as usize, sc.height as usize);
    let pix = cam.fov as f64 / h as f64;
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let si = &gbuf[i];
            if si.is_background() || si.is_ground() {
                continue;
            }
            let Some(pos) = cam.object_pos(si, x as i32, y as i32) else { continue };
            let z1 = (((8388352 - (si.zpos_fine >> 8) as i64) as f64 / cam.zc_mul + 1.0).powi(2) - 1.0) * cam.step_width / cam.zcorr;
            // a ray that starts inside the object meets it at once: a cut
            // through the solid where this view's camera is
            if !keep_cut && z1 < 2.0 * cam.step_width {
                continue;
            }
            let dir = cam.pixel_dir(x as f32, y as f32);
            let nv = cam.to_abs(si.normal.map(|v| v as f32 / 32767.0));
            let mut n = normalize([nv[0] as f64, nv[1] as f64, nv[2] as f64]);
            if len(n) < 0.5 {
                n = dir.map(|v| -v);
            }
            if dot(n, dir) > 0.0 {
                n = n.map(|v| -v);
            }
            let facing = (-dot(n, dir)).clamp(0.0, 1.0);
            let foot = cam.step_width + z1.max(0.0) * pix;
            // the long axis: the ray's direction in the surface
            let t = [0, 1, 2].map(|k| dir[k] - n[k] * dot(dir, n));
            let along = if len(t) > 1e-9 { normalize(t) } else { any_perpendicular(n) };
            let stretch = 1.0 / facing.max(0.25);
            let s = foot * sigma;
            let c = [0, 1, 2].map(|k| rgb[i * 3 + k].clamp(0.0, 1.0));
            out.push(Splat { pos, normal: n, color: c, eye: dir.map(|v| -v), rest: [[0.0; 3]; 15], along, sigma: [s * stretch, s, s * 0.1], view, facing: facing as f32 });
        }
    }
    out
}

/// Where views overlap, one view keeps a patch of the surface: the grid's
/// cells are [`PATCH`] times the median splat width across, and in each the
/// view that saw the most of it the most squarely (the sum of its splats'
/// facing) keeps its splats.  The other views keep theirs only where that
/// view has none near them (a part of the surface it did not see), so the
/// surface is whole and each patch shows one view's picture, not a mixture
/// of slightly different ones.
pub fn merge(splats: &[Splat]) -> Vec<Splat> {
    if splats.is_empty() {
        return Vec::new();
    }
    let fine = cell_size(splats);
    let coarse = fine * PATCH;
    let fkey = |p: Vec3| p.map(|v| (v / fine).floor() as i64);
    let ckey = |p: Vec3| p.map(|v| (v / coarse).floor() as i64);
    let mut score: HashMap<[i64; 3], Vec<(u16, f64)>> = HashMap::new();
    let mut seen: HashSet<([i64; 3], u16)> = HashSet::new();
    for s in splats {
        let e = score.entry(ckey(s.pos)).or_default();
        match e.iter_mut().find(|x| x.0 == s.view) {
            Some(x) => x.1 += s.facing as f64,
            None => e.push((s.view, s.facing as f64)),
        }
        seen.insert((fkey(s.pos), s.view));
    }
    let winner: HashMap<[i64; 3], u16> =
        score.into_iter().map(|(k, v)| (k, v.iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).map_or(0, |x| x.0))).collect();
    let covered = |c: [i64; 3], view: u16| {
        (-1..=1).any(|dx| (-1..=1).any(|dy| (-1..=1).any(|dz| seen.contains(&([c[0] + dx, c[1] + dy, c[2] + dz], view)))))
    };
    splats
        .iter()
        .filter(|s| {
            let w = winner[&ckey(s.pos)];
            s.view == w || !covered(fkey(s.pos), w)
        })
        .copied()
        .collect()
}

/// The size of [`merge`]'s patches in its cells.
const PATCH: f64 = 6.0;

/// The grid of [`merge`]: twice the median splat width.
fn cell_size(splats: &[Splat]) -> f64 {
    let mut feet: Vec<f64> = splats.iter().map(|s| s.sigma[1]).collect();
    let mid = feet.len() / 2;
    feet.select_nth_unstable_by(mid, f64::total_cmp);
    (feet[mid] * 2.0).max(1e-300)
}

const SH_C1: f64 = 0.488_602_511_902_919_9;
const SH_C2: [f64; 5] = [1.092_548_430_592_079_2, -1.092_548_430_592_079_2, 0.315_391_565_252_520_05, -1.092_548_430_592_079_2, 0.546_274_215_296_039_6];
const SH_C3: [f64; 7] = [
    -0.590_043_589_926_643_5,
    2.890_611_442_640_554,
    -0.457_045_799_464_465_8,
    0.373_176_332_590_115_4,
    -0.457_045_799_464_465_8,
    1.445_305_721_320_277,
    -0.590_043_589_926_643_5,
];

/// The spherical harmonics of degree 1..3 of 3D Gaussian splatting (the
/// INRIA code's `eval_sh`) at the unit direction `d` from the camera to the
/// splat: the colour seen is 0.5 + SH_C0 f_dc + the sum of these times the
/// splat's `f_rest`.
pub fn sh_basis(d: Vec3) -> [f64; 15] {
    let [x, y, z] = d;
    let (xx, yy, zz, xy, yz, xz) = (x * x, y * y, z * z, x * y, y * z, x * z);
    [
        -SH_C1 * y,
        SH_C1 * z,
        -SH_C1 * x,
        SH_C2[0] * xy,
        SH_C2[1] * yz,
        SH_C2[2] * (2.0 * zz - xx - yy),
        SH_C2[3] * xz,
        SH_C2[4] * (xx - yy),
        SH_C3[0] * y * (3.0 * xx - yy),
        SH_C3[1] * xy * z,
        SH_C3[2] * y * (4.0 * zz - xx - yy),
        SH_C3[3] * z * (2.0 * zz - 3.0 * xx - 3.0 * yy),
        SH_C3[4] * x * (4.0 * zz - xx - yy),
        SH_C3[5] * z * (xx - yy),
        SH_C3[6] * x * (xx - 3.0 * yy),
    ]
}

/// The number of `f_rest` values per colour channel of a degree.
pub fn rest_count(degree: u8) -> usize {
    let d = degree.min(3) as usize;
    (d + 1) * (d + 1) - 1
}

/// Colours that change with the direction: the surface is cut into cells
/// (three of [`merge`]'s across), and in each cell the colours the views
/// saw (each view's mean there, from its mean direction) are fitted by the
/// spherical harmonics up to `degree` (a ridge fit: the higher degrees held
/// small where few views saw the cell, so that directions between and
/// beyond the views stay near what was seen).  Each kept splat takes the
/// cell's harmonics and keeps its own colour in its own view's direction:
/// the picture's detail stays, the highlights move with the viewer.
/// `fr` is the file's frame (the harmonics are written in it).
pub fn fit_view_colours(all: &[Splat], kept: &mut [Splat], degree: u8, fr: &[Vec3; 3]) {
    let k = rest_count(degree);
    if k == 0 || all.is_empty() {
        return;
    }
    let cell = cell_size(all) * 3.0;
    let key = |p: Vec3| p.map(|v| (v / cell).floor() as i64);
    let to = |v: Vec3| [dot(fr[0], v), dot(fr[1], v), dot(fr[2], v)];
    // per cell and view: the sum of the colours and of the directions, the count
    let mut obs: HashMap<[i64; 3], Vec<Seen>> = HashMap::new();
    for s in all {
        let e = obs.entry(key(s.pos)).or_default();
        let o = match e.iter().position(|o| o.0 == s.view) {
            Some(i) => &mut e[i],
            None => {
                e.push((s.view, [0.0; 3], [0.0; 3], 0));
                e.last_mut().unwrap()
            }
        };
        for c in 0..3 {
            o.1[c] += s.color[c] as f64;
            o.2[c] += s.eye[c];
        }
        o.3 += 1;
    }
    let mut fits: HashMap<[i64; 3], Option<Vec<[f64; 3]>>> = HashMap::new();
    for s in kept.iter_mut() {
        let kc = key(s.pos);
        let fit = fits.entry(kc).or_insert_with(|| obs.get(&kc).and_then(|o| ridge_fit(o, k, &to)));
        let Some(a) = fit else { continue };
        let y = sh_basis(to(s.eye).map(|v| -v));
        for c in 0..3 {
            let mut v = s.color[c] as f64;
            for (i, ai) in a.iter().enumerate() {
                v -= ai[c] * y[i];
                s.rest[i][c] = ai[c] as f32;
            }
            s.color[c] = v as f32;
        }
    }
}

/// What one view saw of a cell: the view, the sum of the colours and of
/// the directions to the camera, their count.
type Seen = (u16, [f64; 3], Vec3, u32);

/// The ridge fit of one cell's views (see [`fit_view_colours`]): the
/// constant and `k` harmonics; None where fewer than two views saw it.
fn ridge_fit(obs: &[Seen], k: usize, to: &dyn Fn(Vec3) -> Vec3) -> Option<Vec<[f64; 3]>> {
    if obs.len() < 2 {
        return None;
    }
    let n = k + 1;
    let mut m = vec![vec![0.0; n]; n];
    let mut r = vec![[0.0; 3]; n];
    for (_, c, e, cnt) in obs {
        let f = 1.0 / *cnt as f64;
        let d = normalize(to(*e)).map(|v| -v);
        let y = sh_basis(d);
        let row: Vec<f64> = std::iter::once(1.0).chain(y[..k].iter().copied()).collect();
        for i in 0..n {
            for j in 0..n {
                m[i][j] += row[i] * row[j];
            }
            for ch in 0..3 {
                r[i][ch] += row[i] * c[ch] * f;
            }
        }
    }
    // the higher degrees held small: l (l + 1) times the strength
    let lam = 0.05;
    for (i, row) in m.iter_mut().enumerate().skip(1) {
        let l = if i <= 3 { 1.0 } else if i <= 8 { 2.0 } else { 3.0 };
        row[i] += lam * l * (l + 1.0);
    }
    let x = solve(m, r)?;
    Some(x[1..].to_vec())
}

/// Gaussian elimination with partial pivoting, three right-hand sides.
fn solve(mut m: Vec<Vec<f64>>, mut r: Vec<[f64; 3]>) -> Option<Vec<[f64; 3]>> {
    let n = m.len();
    for c in 0..n {
        let p = (c..n).max_by(|&a, &b| m[a][c].abs().total_cmp(&m[b][c].abs()))?;
        if m[p][c].abs() < 1e-12 {
            return None;
        }
        m.swap(c, p);
        r.swap(c, p);
        let (top, rest) = m.split_at_mut(c + 1);
        let (rtop, rrest) = r.split_at_mut(c + 1);
        let (pivot, rp) = (&top[c], rtop[c]);
        for (row, rr) in rest.iter_mut().zip(rrest.iter_mut()) {
            let f = row[c] / pivot[c];
            if f == 0.0 {
                continue;
            }
            for (a, b) in row[c..].iter_mut().zip(&pivot[c..]) {
                *a -= f * b;
            }
            for (a, b) in rr.iter_mut().zip(rp) {
                *a -= f * b;
            }
        }
    }
    let mut x = vec![[0.0; 3]; n];
    for i in (0..n).rev() {
        for ch in 0..3 {
            let mut v = r[i][ch];
            for j in i + 1..n {
                v -= m[i][j] * x[j][ch];
            }
            x[i][ch] = v / m[i][i];
        }
    }
    Some(x)
}

/// What an export made.
pub struct SplatResult {
    pub splats: Vec<Splat>,
    /// splats before the overlaps were removed
    pub raw: usize,
    /// views in which the object touches the picture's edge (it may be cut
    /// off there; only around the middle)
    pub cut_views: usize,
    /// where the views were calculated from (never [`Around::Auto`])
    pub around: Around,
    /// the point that is 0 in the file, relative to the scene middle: the
    /// middle, or the camera's eye
    pub origin: Vec3,
    /// the frame of the file (see [`frame`]): the scene's view
    pub vgrads: Mat3,
}

/// Calculates the views and makes the splats.  `progress(view, views)` is
/// called before each view.
pub fn export(sc: &Scene, sp: &SplatParams, progress: &(dyn Fn(usize, usize) + Sync), cancel: &(dyn Fn() -> bool + Sync)) -> Result<SplatResult, String> {
    let mut base = prepare(sc, sp.size);
    let dirs = view_dirs(sp.views);
    let calc = |s: &Scene| -> Result<_, String> {
        if cancel() {
            return Err("cancelled".to_string());
        }
        let (p, gbuf) = crate::render::calculate_cancellable(s, &|_, _| {}, cancel)?;
        let rgb = crate::render::paint_f32(s, &p, &gbuf);
        Ok((p, gbuf, rgb))
    };
    progress(0, dirs.len());
    let mut first = Some(calc(&base)?);
    let around = match sp.around {
        Around::Auto => {
            let g = &first.as_ref().expect("first view").1;
            let filled = g.iter().filter(|s| !s.is_background()).count() as f64 / g.len().max(1) as f64;
            if filled > 0.97 { Around::Camera } else { Around::Middle }
        }
        a => a,
    };
    // around the camera: MB3D's camera is a pinhole at an eye behind its
    // start plane, whose rays start on that plane (what is nearer is cut
    // away); the wide views keep the eye and the plane's distance from it
    // (wider, the plane comes nearer the eye: the zoom makes up for it)
    let eye0 = eye(&base)?;
    if around == Around::Camera {
        let near = |s: &Scene| -> Result<f64, String> {
            let e = eye(s)?;
            let o = start_plane(s)?.0;
            Ok(len([0, 1, 2].map(|k| o[k] - e[k])))
        };
        let d0 = near(&base)?;
        base.fov_y = base.fov_y.max(CAMERA_FOV);
        let d1 = near(&base)?;
        if d0 > 0.0 && d1 > 0.0 {
            base.zoom *= d1 / d0;
        }
        first = None;
    }
    let mut all = Vec::new();
    let mut cut_views = 0;
    for (v, d) in dirs.iter().enumerate() {
        progress(v, dirs.len());
        let s = view_from(&base, *d, around, eye0)?;
        let (p, gbuf, rgb) = match first.take() {
            Some(f) if v == 0 => f,
            _ => calc(&s)?,
        };
        if around == Around::Middle && touches_edge(&gbuf, s.width as usize, s.height as usize) {
            cut_views += 1;
        }
        let off = [0, 1, 2].map(|k| s.mid[k] - base.mid[k]);
        let keep_cut = around == Around::Middle && v == 0;
        all.extend(splats_of_view(&s, &p, &gbuf, &rgb, v as u16, sp.sigma, keep_cut).into_iter().map(|mut x| {
            x.pos = [0, 1, 2].map(|k| x.pos[k] + off[k]);
            x
        }));
    }
    progress(dirs.len(), dirs.len());
    // around the middle: what the scene's start plane cuts away (in front of
    // it) is not in its picture, so not in the file either
    if around == Around::Middle {
        let (o, f) = start_plane(&base)?;
        all.retain(|x| dot([0, 1, 2].map(|k| x.pos[k] - o[k]), f) >= -x.sigma[1]);
    }
    let raw = all.len();
    let mut kept = merge(&all);
    fit_view_colours(&all, &mut kept, sp.degree, &frame(&base.vgrads));
    let origin = if around == Around::Camera { eye0 } else { [0.0; 3] };
    Ok(SplatResult { splats: kept, raw, cut_views, around, origin, vgrads: base.vgrads })
}

/// The view of `base` (prepared) looking along `dir` (see [`view_scene`]):
/// around the middle, or around the camera with its eye at `eye0`
/// (relative to the middle; the middle moved with the view).
pub fn view_from(base: &Scene, dir: Vec3, around: Around, eye0: Vec3) -> Result<Scene, String> {
    let mut s = view_scene(base, dir);
    if around == Around::Camera {
        // (MB3D's start plane is at z_start - mid z: moved with the middle,
        // so that the eye keeps its place relative to it)
        let e = eye(&s)?;
        let target = [0, 1, 2].map(|k| base.mid[k] + eye0[k]);
        let dz = target[2] - e[2] - s.mid[2];
        s.mid = [0, 1, 2].map(|k| target[k] - e[k]);
        s.z_start += dz;
        s.z_end += dz;
    }
    Ok(s)
}

/// The start plane of the scene's camera (where its rays start): a point of
/// it relative to the middle and its normal, the view direction.
pub fn start_plane(sc: &Scene) -> Result<(Vec3, Vec3), String> {
    let p = crate::calc::CalcParams::new(sc)?;
    let cam = crate::render::paint_camera(sc, &p);
    let start = SiLight { zpos_fine: 8_388_352u32 << 8, ..Default::default() };
    let o = cam.object_pos(&start, cam.width / 2, cam.height / 2).unwrap_or([0.0; 3]);
    let m = normalise_matrix_to(1.0, &sc.vgrads);
    Ok((o, m[2]))
}

/// The eye of the scene's camera relative to the scene middle: the point
/// nearest to the rays of a grid of its pixels (MB3D's rays start on a
/// plane, spread like the pixels, but they meet in a point closely enough).
pub fn eye(sc: &Scene) -> Result<Vec3, String> {
    let p = crate::calc::CalcParams::new(sc)?;
    let cam = crate::render::paint_camera(sc, &p);
    // a point of each ray: its start (the object position of depth 0)
    let start = SiLight { zpos_fine: 8_388_352u32 << 8, ..Default::default() };
    let mut m = [[0.0f64; 3]; 3];
    let mut b = [0.0f64; 3];
    for gy in 0..=8 {
        for gx in 0..=8 {
            let (x, y) = ((cam.width - 1) as f32 * gx as f32 / 8.0, (cam.height - 1) as f32 * gy as f32 / 8.0);
            let d = cam.pixel_dir(x, y);
            let o = cam.object_pos(&start, x as i32, y as i32).unwrap_or([0.0; 3]);
            // sum of (I - d d^T) e = (I - d d^T) o
            for i in 0..3 {
                for j in 0..3 {
                    let pij = if i == j { 1.0 } else { 0.0 } - d[i] * d[j];
                    m[i][j] += pij;
                    b[i] += pij * o[j];
                }
            }
        }
    }
    let x = solve(m.iter().map(|r| r.to_vec()).collect(), b.iter().map(|v| [*v, 0.0, 0.0]).collect()).ok_or("the camera's rays are parallel")?;
    Ok([x[0][0], x[1][0], x[2][0]])
}

/// Object pixels on the picture's border.
fn touches_edge(g: &[SiLight], w: usize, h: usize) -> bool {
    let obj = |x: usize, y: usize| {
        let s = &g[y * w + x];
        !s.is_background() && !s.is_ground()
    };
    (0..w).any(|x| obj(x, 0) || obj(x, h - 1)) || (0..h).any(|y| obj(0, y) || obj(w - 1, y))
}

/// The output frame of the scene's view `vgrads`: x right, y into the
/// picture, z up.
pub fn frame(vgrads: &Mat3) -> [Vec3; 3] {
    let m = normalise_matrix_to(1.0, vgrads);
    [m[0], m[2], m[1].map(|v| -v)]
}

/// Writes the splats of an export as a binary 3D Gaussian splatting PLY in
/// the frame of the scene's view, its origin (the middle or the eye) at 0,
/// scaled so that the farthest splat (of the 99.9 % nearest) is 1 away.
/// Returns that scale.
pub fn write_ply(path: &std::path::Path, r: &SplatResult, degree: u8) -> Result<f64, String> {
    let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut w = std::io::BufWriter::new(f);
    let scale = fit_scale(&r.splats, r.origin);
    write_ply_to(&mut w, &r.splats, &r.vgrads, r.origin, scale, degree).map_err(|e| format!("{}: {e}", path.display()))?;
    w.flush().map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(scale)
}

/// 1 / the distance from `origin` of the farthest of the nearest 99.9 %.
fn fit_scale(splats: &[Splat], origin: Vec3) -> f64 {
    if splats.is_empty() {
        return 1.0;
    }
    let mut d: Vec<f64> = splats.iter().map(|s| len([0, 1, 2].map(|k| s.pos[k] - origin[k]))).collect();
    let k = ((d.len() as f64 * 0.999) as usize).min(d.len() - 1);
    d.select_nth_unstable_by(k, f64::total_cmp);
    1.0 / d[k].max(1e-300)
}

/// The PLY's properties: the INRIA layout with the spherical harmonics up
/// to `degree` (`f_rest_*`, by colour channel).
pub fn ply_properties(degree: u8) -> Vec<String> {
    let mut p: Vec<String> = ["x", "y", "z", "nx", "ny", "nz", "f_dc_0", "f_dc_1", "f_dc_2"].map(String::from).to_vec();
    p.extend((0..3 * rest_count(degree)).map(|i| format!("f_rest_{i}")));
    p.extend(["opacity", "scale_0", "scale_1", "scale_2", "rot_0", "rot_1", "rot_2", "rot_3"].map(String::from));
    p
}

pub fn write_ply_to(w: &mut dyn Write, splats: &[Splat], vgrads: &Mat3, origin: Vec3, scale: f64, degree: u8) -> std::io::Result<()> {
    let fr = frame(vgrads);
    let to = |v: Vec3| [dot(fr[0], v), dot(fr[1], v), dot(fr[2], v)];
    let mut head = format!("ply\nformat binary_little_endian 1.0\ncomment mb3d-rs {} Gaussian splats\nelement vertex {}\n", env!("CARGO_PKG_VERSION"), splats.len());
    let props = ply_properties(degree);
    for p in &props {
        head.push_str(&format!("property float {p}\n"));
    }
    let k = rest_count(degree);
    head.push_str("end_header\n");
    w.write_all(head.as_bytes())?;
    // opacity 0.99 as the logit the format stores
    let opacity = (0.99f64 / 0.01).ln();
    let mut rec = Vec::with_capacity(props.len() * 4);
    for s in splats {
        let p = to([0, 1, 2].map(|k| s.pos[k] - origin[k])).map(|v| v * scale);
        let n = normalize(to(s.normal));
        let a = normalize(to(s.along));
        let b = cross(n, a);
        let q = quat_from_axes(a, b, n);
        let mut vals = vec![
            p[0],
            p[1],
            p[2],
            n[0],
            n[1],
            n[2],
            (s.color[0] as f64 - 0.5) / SH_C0,
            (s.color[1] as f64 - 0.5) / SH_C0,
            (s.color[2] as f64 - 0.5) / SH_C0,
        ];
        for c in 0..3 {
            vals.extend((0..k).map(|i| s.rest[i][c] as f64));
        }
        vals.extend([
            opacity,
            (s.sigma[0] * scale).max(1e-30).ln(),
            (s.sigma[1] * scale).max(1e-30).ln(),
            (s.sigma[2] * scale).max(1e-30).ln(),
            q[0],
            q[1],
            q[2],
            q[3],
        ]);
        rec.clear();
        for v in vals {
            rec.extend_from_slice(&(v as f32).to_le_bytes());
        }
        w.write_all(&rec)?;
    }
    Ok(())
}

/// The unit quaternion (w, x, y, z) of the rotation whose matrix has the
/// columns `a`, `b`, `c` (a right-handed orthonormal basis).
pub fn quat_from_axes(a: Vec3, b: Vec3, c: Vec3) -> [f64; 4] {
    // m[row][col]
    let m = [[a[0], b[0], c[0]], [a[1], b[1], c[1]], [a[2], b[2], c[2]]];
    let tr = m[0][0] + m[1][1] + m[2][2];
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [0.25 * s, (m[2][1] - m[1][2]) / s, (m[0][2] - m[2][0]) / s, (m[1][0] - m[0][1]) / s]
    } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
        let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
        [(m[2][1] - m[1][2]) / s, 0.25 * s, (m[0][1] + m[1][0]) / s, (m[0][2] + m[2][0]) / s]
    } else if m[1][1] > m[2][2] {
        let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
        [(m[0][2] - m[2][0]) / s, (m[0][1] + m[1][0]) / s, 0.25 * s, (m[1][2] + m[2][1]) / s]
    } else {
        let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
        [(m[1][0] - m[0][1]) / s, (m[0][2] + m[2][0]) / s, (m[1][2] + m[2][1]) / s, 0.25 * s]
    };
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-300);
    q.map(|v| v / l)
}

fn any_perpendicular(n: Vec3) -> Vec3 {
    let t = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    normalize(cross(n, t))
}

#[inline]
fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
#[inline]
fn len(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}
#[inline]
fn normalize(a: Vec3) -> Vec3 {
    let l = len(a);
    if l > 0.0 { a.map(|v| v / l) } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3, eps: f64) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < eps)
    }

    #[test]
    fn the_views_cover_the_sphere_and_the_first_is_the_scenes() {
        let d = view_dirs(16);
        assert_eq!(d.len(), 16);
        assert_eq!(d[0], [0.0, 0.0, 1.0]);
        for v in &d {
            assert!((len(*v) - 1.0).abs() < 1e-9);
        }
        // every direction of the sphere is near one of them
        for t in view_dirs(200) {
            let near = d.iter().map(|v| dot(*v, t)).fold(-1.0, f64::max);
            assert!(near > 0.7, "{t:?} is {near}");
        }
    }

    #[test]
    fn the_first_view_is_the_scene_itself() {
        let sc = Scene { vgrads: crate::math::vgrads_from_angles(0.3, -0.7, 1.1), ..Default::default() };
        let s = view_scene(&sc, [0.0, 0.0, 1.0]);
        for i in 0..3 {
            assert!(close(s.vgrads[i], sc.vgrads[i], 1e-12));
        }
        // a view from the side is still a rotation of the same handedness
        let s = view_scene(&sc, normalize([1.0, -0.5, 0.2]));
        let g = s.vgrads;
        let det = |m: Mat3| dot(m[0], cross(m[1], m[2]));
        assert!((det(g) - det(sc.vgrads)).abs() < 1e-9);
    }

    #[test]
    fn fixed_lights_light_the_first_view_as_before() {
        let sc = Scene { vgrads: crate::math::vgrads_from_angles(0.4, 1.2, -0.3), ..Default::default() };
        let mut l = sc.lighting.clone();
        for (i, li) in l.lights.iter_mut().enumerate() {
            li.on = true;
            li.x_angle = -2.5 + i as f64 * 0.9;
            li.y_angle = -1.2 + i as f64 * 0.45;
        }
        let mut fixed = l.clone();
        fix_lights(&mut fixed, &sc.vgrads);
        let m = normalise_matrix_to(1.0, &sc.vgrads);
        let mk = |l: &Lighting| {
            let mut lv = crate::lighting::LightVals::new_all(l, 1.0, 0, 1.0, 1.0, 100, 1.0);
            lv.rotate_object_lights(l, &m);
            lv.light_dirs()
        };
        let (a, b) = (mk(&l), mk(&fixed));
        assert_eq!(a.len(), b.len());
        for ((_, x), (_, y)) in a.iter().zip(&b) {
            for k in 0..3 {
                assert!((x[k] - y[k]).abs() < 1e-5, "{a:?} vs {b:?}");
            }
        }
        assert!(fixed.lights.iter().all(|li| li.relative_to_object));
    }

    #[test]
    fn views_around_the_camera_keep_its_eye() {
        let sc = Scene { vgrads: crate::math::vgrads_from_angles(0.3, -0.7, 1.1), mid: [0.2, -0.1, 0.4], ..Default::default() };
        let base = prepare(&sc, 64);
        let e0 = eye(&base).unwrap();
        for d in view_dirs(6) {
            let s = view_from(&base, d, Around::Camera, e0).unwrap();
            let e = eye(&s).unwrap();
            let at = [0, 1, 2].map(|k| s.mid[k] + e[k] - base.mid[k]);
            assert!(close(at, e0, 1e-9 * (1.0 + len(e0))), "{at:?} {e0:?}");
            // the start plane as far from the eye as in the scene's view
            let o = start_plane(&s).unwrap().0;
            let o0 = start_plane(&base).unwrap().0;
            let (d1, d0) = (len([0, 1, 2].map(|k| o[k] - e[k])), len([0, 1, 2].map(|k| o0[k] - e0[k])));
            // (the camera is in single precision)
            assert!((d1 - d0).abs() < 1e-5 * d0, "{d1} {d0}");
        }
        // around the middle the middle stays
        let s = view_from(&base, view_dirs(6)[3], Around::Middle, e0).unwrap();
        assert_eq!(s.mid, base.mid);
    }

    #[test]
    fn the_quaternion_turns_the_axes() {
        let a = normalize([1.0, 2.0, -0.5]);
        let c = normalize(cross(a, [0.3, -0.2, 1.0]));
        let b = cross(c, a);
        let q = quat_from_axes(a, b, c);
        // rotate the unit x axis by q: it is a
        let [w, x, y, z] = q;
        let rx = [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + w * z), 2.0 * (x * z - w * y)];
        assert!(close(rx, a, 1e-9), "{rx:?} {a:?}");
        let rz = [2.0 * (x * z + w * y), 2.0 * (y * z - w * x), 1.0 - 2.0 * (x * x + y * y)];
        assert!(close(rz, c, 1e-9));
    }

    #[test]
    fn the_ply_has_the_splat_properties() {
        let s = Splat {
            pos: [0.5, 0.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            color: [1.0, 0.5, 0.0],
            eye: [0.0, 0.0, -1.0],
            rest: [[0.0; 3]; 15],
            along: [1.0, 0.0, 0.0],
            sigma: [0.02, 0.01, 0.001],
            view: 0,
            facing: 1.0,
        };
        let mut v = Vec::new();
        write_ply_to(&mut v, &[s, s], &crate::math::IDENTITY3, [0.0; 3], 2.0, 0).unwrap();
        let text = String::from_utf8_lossy(&v);
        let end = text.find("end_header\n").unwrap() + "end_header\n".len();
        assert!(text.starts_with("ply\nformat binary_little_endian 1.0\n"));
        assert!(text.contains("element vertex 2\n"));
        assert!(text.contains("property float f_dc_0\n") && text.contains("property float rot_3\n"));
        assert_eq!(v.len() - end, 2 * 17 * 4);
        let f = |i: usize| f32::from_le_bytes(v[end + i * 4..end + i * 4 + 4].try_into().unwrap());
        // x right, y into the picture (the view's z), z up (minus the view's y)
        assert_eq!((f(0), f(1), f(2)), (1.0, 0.0, 0.0));
        assert_eq!((f(3), f(4), f(5)), (0.0, -1.0, 0.0));
        // the colour back from the spherical harmonic
        assert!((0.5 + SH_C0 as f32 * f(6) - 1.0).abs() < 1e-6);
        assert!((f(10) - (0.04f32).ln()).abs() < 1e-6);
        // with the harmonics of degree 2: 24 values more, after the colour
        let mut v = Vec::new();
        write_ply_to(&mut v, &[s], &crate::math::IDENTITY3, [0.0; 3], 2.0, 2).unwrap();
        let text = String::from_utf8_lossy(&v);
        assert!(text.contains("property float f_dc_2\nproperty float f_rest_0\n") && text.contains("f_rest_23\nproperty float opacity\n"));
        assert!(!text.contains("f_rest_24"));
    }

    #[test]
    fn the_harmonics_follow_the_colour_seen_from_each_side() {
        // one cell seen by four views: brighter the more it is seen from +x
        let fr = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mk = |eye: Vec3, view: u16| {
            let c = 0.5 + 0.3 * eye[0] as f32;
            Splat { pos: [0.0; 3], normal: eye, color: [c; 3], eye, rest: [[0.0; 3]; 15], along: any_perpendicular(eye), sigma: [1.0, 1.0, 0.1], view, facing: 1.0 }
        };
        let eyes = [normalize([1.0, 0.0, -1.0]), normalize([-1.0, 0.0, -1.0]), [0.0, 0.0, -1.0], normalize([0.0, 1.0, -1.0])];
        let all: Vec<Splat> = eyes.iter().enumerate().map(|(i, e)| mk(*e, i as u16)).collect();
        let mut kept = vec![all[2]];
        fit_view_colours(&all, &mut kept, 2, &fr);
        let seen = |s: &Splat, eye: Vec3| {
            let y = sh_basis(eye.map(|v| -v));
            s.color[0] as f64 + (0..15).map(|i| s.rest[i][0] as f64 * y[i]).sum::<f64>()
        };
        // its own colour in its own direction, brighter from +x, darker from -x
        assert!((seen(&kept[0], eyes[2]) - 0.5).abs() < 1e-5);
        assert!(seen(&kept[0], eyes[0]) > 0.6, "{}", seen(&kept[0], eyes[0]));
        assert!(seen(&kept[0], eyes[1]) < 0.4);
    }

    #[test]
    fn overlapping_views_keep_the_squarest() {
        let mk = |x: f64, view: u16, facing: f32| Splat {
            pos: [x, 0.0, 0.0],
            normal: [0.0, 0.0, -1.0],
            color: [0.0; 3],
            eye: [0.0, 0.0, -1.0],
            rest: [[0.0; 3]; 15],
            along: [1.0, 0.0, 0.0],
            sigma: [1.0, 1.0, 0.1],
            view,
            facing,
        };
        // two views on the same spot, one far away, two of the same view in one cell
        let m = merge(&[mk(0.1, 0, 0.5), mk(0.2, 1, 0.9), mk(0.3, 1, 0.4), mk(50.0, 0, 0.5)]);
        assert_eq!(m.len(), 3);
        assert!(m.iter().filter(|s| s.pos[0] < 1.0).all(|s| s.view == 1));
    }
}
