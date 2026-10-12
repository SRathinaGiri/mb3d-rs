//! MB3D's Monte Carlo renderer: path traced images with ambient bounces,
//! soft shadows from area lights, reflections, transparency with
//! refraction and absorption, depth of field with bokeh shapes, and the
//! progressive, noise-adaptive accumulation of rays per pixel.
//!
//! * `TMCCalcThread` (CalcMonteCarlo.pas): `Execute`, `CalcRay`, `CalcHSMC`,
//!   `CalcPhongLight[NoHS]`, `CalcVisLights`, `CalcBGLight`, `CalcColor`,
//!   `CalcN`, `DoDOF`, `CalcBokehMC`, the Halton sequences
//! * `TMCForm` (MonteCarloForm.pas): `StartCalc`, `CalcAvrgNoise`, `.m3c`
//!   files
//! * `TPaintThreadMC.PaintColorBuffer` (PaintThread.pas): exposure,
//!   saturation, gamma and soft clipping in Lab colour space
//!
//! The image is refined in passes: the first pass shoots 4 rays per pixel,
//! every further pass adds rays where the noise estimate asks for them.

use crate::calc::{CalcParams, Marcher};
use crate::gbuffer::SiLight;
use crate::lighting::{pos_light_shape, LightVals, PaintCamera};
use crate::math::*;
use crate::scene::{CameraOptic, McSettings, Scene};
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::OnceLock;
use std::task::{Context, Poll};

type S3 = [f32; 3];
type S4 = [f32; 4];

const SEED_MUL: f64 = 1.0 / 0x7FFF_FFFF as f64;
const D1D65535: f32 = 1.0 / 65535.0;
/// largest value of MB3D's 24 bit record encoding (`MCRGBtoDouble`)
const MC_MAX: f64 = 1096.6331584284585 - std::f64::consts::E + 2.0;

// ---------------------------------------------------------------------------
// small vector helpers (TSVec = 4 singles, TVec3D = 3 doubles)

#[inline]
fn s3(v: &Vec3) -> S3 {
    [v[0] as f32, v[1] as f32, v[2] as f32]
}
#[inline]
fn d3(v: S3) -> Vec3 {
    [v[0] as f64, v[1] as f64, v[2] as f64]
}
#[inline]
fn add3(a: S3, b: S3) -> S3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
fn sub3(a: S3, b: S3) -> S3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
fn mul3(a: S3, b: S3) -> S3 {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}
#[inline]
fn sc3(a: S3, s: f32) -> S3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
fn addw3(a: &mut S3, b: S3, w: f32) {
    a[0] += b[0] * w;
    a[1] += b[1] * w;
    a[2] += b[2] * w;
}
#[inline]
fn dot3(a: S3, b: S3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
fn first3(a: S4) -> S3 {
    [a[0], a[1], a[2]]
}
/// `YofSVec` (negative results are 0)
#[inline]
fn luma(v: S3) -> f32 {
    (v[0] * 0.3 + v[1] * 0.59 + v[2] * 0.11).max(0.0)
}
/// `SVecPow`
#[inline]
fn spow(v: S3, e: f32) -> S3 {
    [v[0].powf(e), v[1].powf(e), v[2].powf(e)]
}
/// `LinInterpolate2SVecs(sv1, sv2, w1)` = sv1 * w1 + sv2 * (1 - w1)
#[inline]
fn lin2(sv1: S3, sv2: S3, w1: f32) -> S3 {
    [sv2[0] + w1 * (sv1[0] - sv2[0]), sv2[1] + w1 * (sv1[1] - sv2[1]), sv2[2] + w1 * (sv1[2] - sv2[2])]
}
/// `NormaliseVectorTo`
#[inline]
fn norm_to(n: f64, v: Vec3) -> Vec3 {
    let d = n / (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + 1e-100).sqrt();
    [v[0] * d, v[1] * d, v[2] * d]
}
#[inline]
fn not_zero(d: f64) -> f64 {
    if d.abs() < 1e-40 {
        if d < 0.0 {
            -1e-40
        } else {
            1e-40
        }
    } else {
        d
    }
}
/// `FracSingle`
#[inline]
fn frac_s(s: f32) -> f32 {
    s - s.trunc()
}
/// `MakeOrthoVecs`: two unit vectors orthogonal to `v` (and each other)
fn make_ortho(v: &Vec3) -> (Vec3, Vec3) {
    let n = normalize(*v);
    let o0 = if n[0].abs() > 0.1 {
        let d = 1.0 / (n[0] * n[0] + n[2] * n[2]).sqrt();
        [n[2] * d, 0.0, -n[0] * d]
    } else {
        let d = 1.0 / (n[1] * n[1] + n[2] * n[2]).sqrt();
        [0.0, -n[2] * d, n[1] * d]
    };
    let o0 = [o0[0] as f32 as f64, o0[1] as f32 as f64, o0[2] as f32 as f64];
    let o1 = [o0[1] * n[2] - o0[2] * n[1], o0[2] * n[0] - o0[0] * n[2], o0[0] * n[1] - o0[1] * n[0]];
    (o0, [o1[0] as f32 as f64, o1[1] as f32 as f64, o1[2] as f32 as f64])
}
/// `SqrDistSV`: squared distance of point `a` from the line along unit `b`
#[inline]
fn sqr_dist_sv(a: S3, b: S3) -> f32 {
    let c = [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    dot3(c, c)
}

// ---------------------------------------------------------------------------
// Halton sequences and per pixel shifts

#[derive(Clone, Copy, Default)]
struct HaltonRec {
    hrx: u16,
    hry: u16,
    hdx: u16,
    hdy: u16,
}

fn halton(index: i32, base: i32) -> f64 {
    let mut r = 0.0;
    let mut f = 1.0 / base as f64;
    let d = f;
    let mut i = index;
    while i > 0 {
        r += f * (i % base) as f64;
        i = (i as f64 * d).trunc() as i32;
        f *= d;
    }
    r
}

/// `PreComputeHaltonSequence`: bases 2, 3 for the pixel area, 5, 7 for discs
fn halton_seq() -> &'static [HaltonRec] {
    static H: OnceLock<Vec<HaltonRec>> = OnceLock::new();
    H.get_or_init(|| {
        (0..65536)
            .map(|i| {
                let q = |b: i32| (halton(i + 1, b) * 65535.0).round_ties_even() as u16;
                HaltonRec { hrx: q(2), hry: q(3), hdx: q(5), hdy: q(7) }
            })
            .collect()
    })
}

/// `GetHalton2Dshifts`: a random but fixed offset of the sequence per pixel
fn halton_2d_shifts(x: i32, y: i32) -> (f32, f32) {
    let i = x.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    let i2 = y.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    let mut i2 = x.wrapping_shl(15) ^ y ^ i.wrapping_shl(9) ^ i2.wrapping_shl(5);
    i2 = i2.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    i2 = i2.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    let s1 = ((i2 & 0x7FFF_FFFF) as f64 * SEED_MUL) as f32;
    i2 = i2.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    let s2 = ((i2 & 0x7FFF_FFFF) as f64 * SEED_MUL) as f32;
    (s1, s2)
}

/// `MakeDiscFromHalton`: square 0..1 -> unit disc (concentric mapping)
fn make_disc(xx: &mut f32, yy: &mut f32) {
    let pi025 = std::f32::consts::PI * 0.25;
    let nz = |v: f32| if v == 0.0 { 1e-30 } else { v };
    let mut x = *xx * 2.0 - 1.0;
    let mut y = *yy * 2.0 - 1.0;
    let (sd, cd);
    if x > y.abs() {
        (sd, cd) = (pi025 * y / nz(x)).sin_cos();
        y = x;
    } else if x > y {
        (sd, cd) = (pi025 * (6.0 - x / nz(y))).sin_cos();
        y = -y;
    } else if x > -y {
        (sd, cd) = (pi025 * (2.0 - x / nz(y))).sin_cos();
    } else {
        (sd, cd) = (pi025 * (4.0 + y / nz(x))).sin_cos();
        y = -x;
    }
    x = sd * y;
    *xx = x;
    *yy = cd * y;
}

/// `CalcBokeh`: brightness of the aperture at (xx, yy) of the unit disc for
/// the bokeh shapes 0..5
pub fn calc_bokeh(xx: f32, yy: f32, nr: i32) -> f32 {
    let r = (xx * xx + yy * yy).sqrt();
    let mut result = if nr & 1 == 0 { 1.0 } else { 1.5 - r * 0.5 };
    if nr == 0 {
        if r < 0.94 {
            result = 1.03;
        } else if r < 1.0 {
            result = 0.9682 + (r - 0.94) * 0.5;
        }
    } else if nr > 1 {
        let a = xx.atan2(yy);
        let mut u = if nr < 4 {
            0.92 + ((a * 5.0).cos() * 0.5 + 0.5).powi(2) * 0.12
        } else {
            0.94 + ((a * 7.0).cos() * 0.5 + 0.5).powi(2) * 0.08
        };
        let s = u - 0.04;
        u = if r < s { 1.0 } else { (r - s) / (1.0 - s) * 0.05 + s };
        result *= u;
    }
    result
}

// ---------------------------------------------------------------------------
// the accumulated image (`TMCrecord`)

/// One pixel of the Monte Carlo image (`TMCrecord`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct McRecord {
    /// average colour (square roots of the light with internal gamma 2)
    pub col: [f32; 3],
    /// average luminance and average squared luminance of the rays
    pub ysum: f64,
    pub ysqr: f64,
    pub ray_count: u16,
    /// a ray hit the object (`Zbyte` bit 7)
    pub hit: bool,
}

/// `MCRGBtoDouble`: 24 bit value -> -1..~1097
fn mcrgb_to_f64(c: u32) -> f64 {
    let r = (c & 0xFF_FFFF) as f64 * 4.7683718662483612447000291764754e-7 - 1.0;
    if r > 2.0 {
        (r - 1.0).exp() - std::f64::consts::E + 2.0
    } else {
        r
    }
}

/// `DoubleToMCRGB`
fn f64_to_mcrgb(d: f64) -> u32 {
    let dt = if d > 2.0 { (d - 2.0 + std::f64::consts::E).ln() + 2.0 } else { d + 1.0 };
    (dt.clamp(0.0, 8.0) * 2097151.875).round_ties_even() as u32 & 0xFF_FFFF
}

#[inline]
fn clamp_mc(d: f64) -> f64 {
    d.clamp(-1.0, MC_MAX)
}

/// The state of a Monte Carlo rendering: the records plus the statistics
/// the next pass is steered by.
#[derive(Clone, Debug)]
pub struct McImage {
    pub width: usize,
    pub height: usize,
    pub recs: Vec<McRecord>,
    /// finished passes and calculation time (seconds)
    pub passes: u32,
    pub seconds: f64,
    avrg_sqr_noise: f64,
    avrg_rcount: f32,
}

/// Noise statistics (`CalcAvrgNoise`), shown by MB3D below the image.
#[derive(Clone, Copy, Debug, Default)]
pub struct McStats {
    pub avg_noise: f64,
    pub avg_rays: f64,
    pub max_noise: f64,
    pub max_rays: u32,
    /// some pixels have no ray yet
    pub zero_counts: bool,
}

impl McImage {
    pub fn new(width: usize, height: usize) -> McImage {
        McImage {
            width,
            height,
            recs: vec![McRecord::default(); width * height],
            passes: 0,
            seconds: 0.0,
            avrg_sqr_noise: 1.0,
            avrg_rcount: 0.0,
        }
    }

    /// `CalcAvrgNoise`
    pub fn stats(&self) -> McStats {
        let l = self.recs.len().max(1) as f64;
        let mut st = McStats::default();
        let mut rsum = 0f64;
        for r in &self.recs {
            if r.ray_count == 0 {
                st.zero_counts = true;
                continue;
            }
            rsum += r.ray_count as f64;
            st.max_rays = st.max_rays.max(r.ray_count as u32);
            let v = ((r.ysqr - r.ysum * r.ysum) / r.ray_count as f64).max(0.0);
            st.avg_noise += v.sqrt();
            st.max_noise = st.max_noise.max(v.sqrt());
        }
        st.avg_rays = rsum / l;
        st.avg_noise /= l;
        st
    }

    fn avg_sqr_noise(&self) -> f64 {
        let l = self.recs.len().max(1) as f64;
        let s: f64 = self
            .recs
            .iter()
            .filter(|r| r.ray_count != 0)
            .map(|r| ((r.ysqr - r.ysum * r.ysum) / r.ray_count as f64).max(0.0))
            .sum();
        (s / l + 0.002).max(0.002)
    }
}

// ---------------------------------------------------------------------------
// per render constants (`CalcMCT`)

struct McCtx<'a> {
    p: &'a CalcParams,
    lv: &'a LightVals,
    cam: &'a PaintCamera,
    mid: Vec3,
    /// `HSvecs`: direction towards global lights (-StepWidth * light) or the
    /// absolute position of positional lights, per entry of `lv.lights`
    hs_vecs: Vec<Vec3>,
    max_amb_depth: i32,
    max_spec_depth: i32,
    calc_reflects: bool,
    calc_trans: bool,
    only_difs: bool,
    secant_search: bool,
    norm_sd_amount: bool,
    diff_reflects: i32,
    trans_di_const: f32,
    absorption: f32,
    light_scattering_mul: f32,
    sr_light_amount: f32,
    amb_max_l: f64,
    do_dof: bool,
    dof_aperture: f32,
    dof_zsharp: f32,
    fovy1d: f32,
    diff_reflects_big_enough: bool,
    bokeh_nr: i32,
    gauss_aa: bool,
    soft_shadow_radius: f32,
    hs_max_len_mul: f32,
    hs_max_lmul: f32,
    vol: bool,
    width: i32,
    height: i32,
}

// ---------------------------------------------------------------------------
// the marches of a ray: what the CPU's marcher or the graphics card does
// for the tracer (the tracer itself keeps no iteration state)

/// A march of the tracer: `kind` 0 a ray of `CalcRay` (from `pos` along
/// `dir`, at most `len`, with `zz` the depth along the view, `zz2` the
/// ray's length so far and `zzplus` the depth per step; `probe`: a primary
/// ray, which first looks whether it starts inside the set), 1 a shadow ray
/// of `CalcHSMC` towards a light (`len` its length, `zz` the depth for the
/// DE stop of its start, `zz2`/`zzplus` the depth along it; `trans`: the
/// surface it hits is found, coloured and handed back), 2 the way through
/// a transparent object hit by a shadow ray (`len` the length left,
/// `ms_de_stop` the DE stop).
#[derive(Clone, Copy, Debug, Default)]
pub struct McReq {
    pub kind: u8,
    pub pos: Vec3,
    pub dir: Vec3,
    pub zz: f64,
    pub zz2: f64,
    pub zzplus: f64,
    pub len: f64,
    pub seed: i32,
    pub inside: bool,
    pub probe: bool,
    pub trans: bool,
    pub ms_de_stop: f32,
}

/// The result of a march: `kind` 0 open air (or a shadow ray without a
/// coloured surface), 1 a surface, 2 a primary ray that starts inside the
/// set; the position, depths and length left where it ended, the step
/// count of the dynamic fog, the normal, gradient and orbit trap of the
/// surface, the random seed after the march, the inside flag (a primary ray
/// that starts inside the set with inside-and-outside rendering turns
/// around), whether the first and the last DE came from a dIFS part, and
/// the DE stop where it ended.
#[derive(Clone, Copy, Debug, Default)]
pub struct McRes {
    pub kind: u8,
    pub pos: Vec3,
    pub zz: f64,
    pub zz2: f64,
    pub len: f64,
    pub dyn_fog: f32,
    pub de_limited: bool,
    pub normals: Vec3,
    pub si_grad: u16,
    pub otrap: u16,
    pub seed: i32,
    pub inside: bool,
    pub start_difs: bool,
    pub last_difs: bool,
    pub ms_de_stop: f32,
}

/// `GetRand` on a seed
#[inline]
fn rand_of(seed: &mut i32) -> f64 {
    *seed = seed.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    (*seed & 0x7FFF_FFFF) as f64 * SEED_MUL
}

/// The CPU's marcher of a thread.
struct McMarch<'a, 'c> {
    c: &'c McCtx<'a>,
    m: Marcher<'a>,
}

impl<'a, 'c> McMarch<'a, 'c> {
    fn new(c: &'c McCtx<'a>) -> Self {
        McMarch { c, m: Marcher::new(c.p, 0) }
    }

    fn run(&mut self, r: &McReq, vfov: Vec3) -> McRes {
        let m = &mut self.m;
        m.it.c = r.pos;
        m.inside = r.inside;
        m.calc_inside = r.inside;
        m.it.calc_sit = false;
        m.vfov = vfov;
        match r.kind {
            0 => self.ray(r),
            1 => self.shadow(r),
            _ => self.through(r),
        }
    }

    #[inline]
    fn update_de_stop(&mut self, zz: f64) {
        let p = self.c.p;
        self.m.ms_de_stop = (p.de_stop as f64 * (1.0 + zz.abs() * p.de_stop_factor as f64)) as f32;
    }

    #[inline]
    fn advance(&mut self, dir: &Vec3, d: f64) {
        add_weight(&mut self.m.it.c, dir, d);
    }

    /// `DoDynFog` for volumetric light: the step limit and the light
    /// collected on the last step
    fn do_dyn_fog(&mut self, act_de: &mut f64, rsfmul: f32, last_sw: f64, dyn_fog: &mut f32) {
        let p = self.c.p;
        *act_de = ((*act_de - p.ms_de_sub as f64 * self.m.ms_de_stop as f64) * p.z_step_div as f64 * rsfmul as f64).max(0.11);
        let s1 = (self.m.ms_de_stop.max(0.4) * p.mh04zsd) as f64;
        if s1 < *act_de {
            *act_de = s1;
        }
        if let Some(vl) = &p.vol {
            let c = self.m.it.c;
            let vf = self.m.vfov;
            vl.integrate(p, &c, &vf, last_sw, self.m.mzz, dyn_fog);
        }
    }

    /// `CalculateNormals` (on the DE) or `CalculateNormalsOnSmoothIt`: one
    /// sided gradient on the world axes, its side chosen at random; returns
    /// the smoothed iteration count and the normal
    fn normals(&mut self, raydir: &Vec3, seed: &mut i32) -> (f32, Vec3) {
        let p = self.c.p;
        let on_de = p.normals_on_de;
        let m = &mut self.m;
        let mut sca: f32 = if *seed & 128 == 0 { 0.15 } else { -0.15 };
        m.it.calc_sit = true;
        let (no, nn) = if on_de {
            let no = m.calc_de();
            let nn = m.it.smooth_it;
            m.it.calc_sit = false;
            (no, nn)
        } else {
            m.mand_function();
            (0.0, m.it.smooth_it)
        };
        let mut i = 4;
        let mut n = [0.0; 3];
        let mut normals = [0.0; 3];
        loop {
            if i < 4 {
                sca = ((rand_of(seed) - 0.5) * 0.5) as f32;
            }
            let off = (p.de_stop.min(1.0) as f64) * (1.0 + m.mzz * p.de_stop_factor as f64) * sca as f64 * p.step_width;
            for k in [2usize, 0, 1] {
                let t = m.it.c[k];
                m.it.c[k] = t + off;
                n[k] = if on_de {
                    (m.calc_de() - no) * sca as f64
                } else {
                    m.mand_function();
                    (nn - m.it.smooth_it) as f64 * sca as f64
                };
                m.it.c[k] = t;
            }
            let l = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
            if l > 1e-100 && dot(&n, raydir) < 0.0 {
                normals = scale(n, 1.0 / l.sqrt());
                break;
            }
            i -= 1;
            if i == 0 {
                break;
            }
        }
        if !on_de {
            m.it.calc_sit = false;
        }
        if i == 0 {
            normals = norm_to(-1.0, *raydir);
        }
        (nn, normals)
    }

    /// `doBinSearchIt`: the surface of an iteration limited (non DE) object
    fn bin_search_it(&mut self, zz: &mut f64, dir: &Vec3) {
        let m = &mut self.m;
        let saved = m.it.max_it;
        let yp = saved as f32 - 0.99;
        m.it.max_it += 1;
        let mut itmp = 10;
        m.it.calc_sit = true;
        let mut dt1 = 0f64;
        let mut dmul = 1f64;
        let mut first = true;
        let (mut last_si, mut last_dif) = (0f32, 0f32);
        loop {
            *zz += dt1;
            add_weight(&mut m.it.c, dir, dt1);
            m.calc_de();
            if !first && last_dif < (yp - m.it.smooth_it).abs() {
                *zz -= dt1;
                add_weight(&mut m.it.c, dir, -dt1);
                m.it.smooth_it = last_si;
                if dt1 > 0.0 {
                    dmul *= 0.5;
                } else {
                    dmul *= 0.7;
                }
            }
            last_dif = (yp - m.it.smooth_it).abs();
            last_si = m.it.smooth_it;
            if m.it.smooth_it > m.it.max_it as f32 - 0.1 {
                dt1 = -3.0;
            } else {
                *zz -= 0.001;
                add_weight(&mut m.it.c, dir, -0.001);
                m.calc_de();
                let r = last_si - m.it.smooth_it;
                dt1 = if r.abs() < 1e-30 {
                    (if last_si < m.it.smooth_it { 1.0 } else { 0.0 }) - 0.5
                } else if r < 0.0 {
                    ((yp - m.it.smooth_it) / (r * 500.0)) as f64
                } else {
                    ((yp - m.it.smooth_it) / (r * 1000.0)) as f64
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
        m.it.max_it = saved;
    }

    /// The gradient of a surface from its smoothed iteration count, then
    /// the colour (`doColorOnIt`, `RMdoColor`): (gradient, orbit trap)
    fn surface_colour(&mut self, nn: f32, de_limited: bool) -> (u16, u16) {
        let p = self.c.p;
        let sw = p.step_width;
        let m = &mut self.m;
        let v = if de_limited {
            32767.0 - (nn + p.d_col_plus + p.col_var_de_stop_mul * ((m.ms_de_stop as f64 * sw).ln() as f32)) * p.mcts_m
        } else {
            32767.0 - nn * p.mcts_m
        };
        let mut si = SiLight { si_gradient: min_max_clip_15bit(v), ..Default::default() };
        if p.color_on_it != 0 {
            m.do_color_on_it();
        }
        m.do_color(&mut si);
        (si.si_gradient, si.otrap)
    }

    /// The march of `CalcRay` (and the start of a primary ray in `Execute`)
    fn ray(&mut self, r: &McReq) -> McRes {
        let c = self.c;
        let p = c.p;
        let mut seed = r.seed;
        let mut o = McRes::default();
        let dir = r.dir;
        let mut zz = r.zz;
        let mut zz2 = r.zz2;
        let max_l = r.len;
        let zzplus = r.zzplus;
        self.update_de_stop(zz);
        let mut dtmp = self.m.calc_de();
        if r.probe && (self.m.it.it_result >= self.m.max_its_result || dtmp < self.m.ms_de_stop as f64) {
            // the start is inside the set (cutting plane or start plane)
            if p.in_and_outside {
                let m = &mut self.m;
                m.inside = !m.inside;
                m.calc_inside = !m.calc_inside;
                dtmp = m.calc_de();
            } else {
                let m = &mut self.m;
                let mut si = SiLight::default();
                if p.color_on_it != 0 {
                    m.do_color_on_it();
                }
                m.do_color(&mut si);
                m.mzz = zz;
                if p.color_option > 4 {
                    si.si_gradient |= 32768;
                } else if m.inside {
                    m.it.calc_sit = true;
                    m.calc_de();
                    si.si_gradient = min_max_clip_15bit(m.it.smooth_it * p.mcts_m) | 32768;
                } else {
                    si.si_gradient = 32768 + (32767.0 * (m.it.rout / p.d_rstop).clamp(0.0, 1.0)).round_ties_even() as u16;
                }
                o.kind = 2;
                o.pos = m.it.c;
                o.si_grad = si.si_gradient;
                o.otrap = si.otrap;
                o.inside = m.inside;
                o.last_difs = m.last_difs;
                o.seed = seed;
                return o;
            }
        }
        o.start_difs = self.m.last_difs;
        let mut dyn_fog = 0f32;
        let mut open_air = false;
        let mut de_limited = true;
        let mut first_step = true;
        let mut rsfmul = 1f32;
        let mut rlast_sw = dtmp * p.z_step_div as f64;
        let mut rlast_de;
        let mut guard = 0u32;
        loop {
            rlast_de = dtmp;
            let mut dstep;
            if c.vol {
                dstep = dtmp;
                self.m.mzz = zz.abs();
                self.do_dyn_fog(&mut dstep, rsfmul, rlast_sw, &mut dyn_fog);
            } else {
                let m = &self.m;
                dstep = ((dtmp - p.ms_de_sub as f64 * m.ms_de_stop as f64) * p.z_step_div as f64 * rsfmul as f64).max(0.11);
                let d1 = (m.ms_de_stop.max(0.4) * p.mh04zsd) as f64;
                let count = p.dfog_on_it == 0 || m.it.it_result == p.dfog_on_it as i32;
                if d1 < dstep {
                    if count {
                        dyn_fog += (d1 / dstep) as f32;
                    }
                    dstep = d1;
                } else if count {
                    dyn_fog += 1.0;
                }
            }
            if first_step {
                first_step = false;
                dstep *= rand_of(&mut seed);
            }
            rlast_sw = dstep;
            zz2 += dstep;
            if zz2 > max_l {
                if c.vol {
                    dstep = max_l - zz2 + dstep;
                    self.advance(&dir, dstep);
                    self.do_dyn_fog(&mut rlast_de, rsfmul, dstep, &mut dyn_fog);
                }
                open_air = true;
                break;
            }
            self.advance(&dir, dstep);
            zz += dstep * zzplus;
            self.update_de_stop(zz);
            dtmp = self.m.calc_de();
            if self.m.it.it_result >= self.m.max_its_result {
                de_limited = false;
                break;
            }
            if !(dtmp >= self.m.ms_de_stop as f64) && self.m.it.it_result >= p.min_it {
                break;
            }
            if dtmp > rlast_de + rlast_sw {
                dtmp = rlast_de + rlast_sw;
            }
            rsfmul = if rlast_de > dtmp + 1e-30 {
                let d = rlast_sw / (rlast_de - dtmp);
                if d < 1.0 {
                    d.max(0.5) as f32
                } else {
                    1.0
                }
            } else {
                1.0
            };
            guard += 1;
            if guard > 1_000_000 {
                open_air = true;
                break;
            }
        }
        if self.m.inside && !c.vol {
            dyn_fog *= 200.0 * p.de_stop / p.width as f32;
        }
        o.dyn_fog = dyn_fog;
        o.zz2 = zz2;
        o.inside = self.m.inside;
        if open_air {
            o.pos = self.m.it.c;
            o.zz = zz;
            o.last_difs = self.m.last_difs;
            o.seed = seed;
            return o;
        }
        if de_limited {
            if c.secant_search {
                let mut itmp = 4;
                while itmp > 0 && (dtmp - self.m.ms_de_stop as f64).abs() > 0.001 {
                    rlast_de = not_zero(rlast_de - dtmp);
                    let f = rlast_sw * (dtmp - self.m.ms_de_stop as f64) / rlast_de;
                    if dtmp < self.m.ms_de_stop as f64 {
                        rlast_sw = if f >= 0.0 || f < rlast_sw.abs() * -0.94 { rlast_sw.abs() * -0.5 } else { f };
                    } else {
                        rlast_sw = if f <= 0.0 || f > rlast_sw.abs() * 0.94 { rlast_sw.abs() * 0.5 } else { f };
                    }
                    rlast_de = dtmp;
                    zz += rlast_sw * zzplus;
                    self.advance(&dir, rlast_sw);
                    self.update_de_stop(zz);
                    itmp -= 1;
                    if itmp <= 0 {
                        break;
                    }
                    dtmp = self.m.calc_de();
                }
            } else {
                let mut itmp = 10;
                let mut ds = rlast_sw * -0.5;
                while (dtmp - self.m.ms_de_stop as f64).abs() > 0.001 {
                    zz += ds * zzplus;
                    self.advance(&dir, ds);
                    self.update_de_stop(zz);
                    itmp -= 1;
                    if itmp <= 0 {
                        break;
                    }
                    dtmp = self.m.calc_de();
                    ds = if dtmp < self.m.ms_de_stop as f64 { ds.abs() * -0.55 } else { ds.abs() * 0.55 };
                }
            }
        } else {
            let mut ds = zz;
            self.bin_search_it(&mut ds, &dir);
            zz += (ds - zz) * zzplus;
        }
        self.m.mzz = zz.abs();
        let (nn, normals) = self.normals(&dir, &mut seed);
        o.pos = self.m.it.c;
        (o.si_grad, o.otrap) = self.surface_colour(nn, de_limited);
        o.kind = 1;
        o.zz = zz;
        o.de_limited = de_limited;
        o.normals = normals;
        o.last_difs = self.m.last_difs;
        o.seed = seed;
        o
    }

    /// A shadow ray of `CalcHSMC`, and the surface it hits when that may
    /// be transparent
    fn shadow(&mut self, r: &McReq) -> McRes {
        let p = self.c.p;
        let mut seed = r.seed;
        let mut o = McRes::default();
        let v = r.dir;
        let mut dt1 = r.len;
        let zz2mul = r.zzplus;
        let mut zz2 = r.zz2;
        self.update_de_stop(r.zz);
        let mut rsfd = 2.0f32;
        let mut dtmp = self.m.calc_de();
        let (mut rlast_de, mut rlast_sw);
        loop {
            rlast_de = dtmp;
            dtmp = (dtmp * p.z_step_div as f64 * rsfd as f64).min((self.m.ms_de_stop.max(0.4) * p.mh04zsd) as f64);
            rlast_sw = dtmp;
            dt1 -= dtmp;
            self.advance(&v, dtmp);
            zz2 += dtmp * zz2mul;
            self.update_de_stop(zz2);
            dtmp = self.m.calc_de();
            // a march that started inside goes backwards and overflows: it
            // ends at a DE that is not a number (Delphi's comparisons are
            // true for a NaN) or beyond any scene
            if self.m.it.it_result >= self.m.max_its_result || !(dtmp >= self.m.ms_de_stop as f64) || !(dtmp < 1e30) {
                break;
            }
            if dtmp > rlast_de + rlast_sw {
                dtmp = rlast_de + rlast_sw;
            }
            rsfd = if rlast_de > dtmp + 1e-30 {
                let s = rlast_sw / (rlast_de - dtmp);
                if s < 1.0 {
                    s.max(0.5) as f32
                } else {
                    1.0
                }
            } else {
                1.0
            };
            if !(dt1 >= 0.0) {
                break;
            }
        }
        o.len = dt1;
        o.seed = seed;
        if dt1 <= 0.01 || !r.trans {
            return o;
        }
        // a transparent object between surface and light
        let de_limited = self.m.it.it_result < self.m.max_its_result;
        if de_limited {
            let mut itmp = 3;
            while itmp > 0 && (dtmp - self.m.ms_de_stop as f64).abs() > 0.01 {
                rlast_de = not_zero(rlast_de - dtmp);
                let f = rlast_sw * (dtmp - self.m.ms_de_stop as f64) / rlast_de;
                if dtmp < self.m.ms_de_stop as f64 {
                    rlast_sw = if f >= 0.0 || f < rlast_sw.abs() * -0.94 { rlast_sw.abs() * -0.5 } else { f };
                } else {
                    rlast_sw = if f <= 0.0 || f > rlast_sw.abs() * 0.94 { rlast_sw.abs() * 0.5 } else { f };
                }
                rlast_de = dtmp;
                zz2 += rlast_sw * zz2mul;
                self.advance(&v, rlast_sw);
                self.update_de_stop(zz2);
                dtmp = self.m.calc_de();
                itmp -= 1;
            }
        } else {
            let mut d = zz2;
            self.bin_search_it(&mut d, &v);
            zz2 += (d - zz2) * zz2mul;
        }
        self.m.mzz = zz2.abs();
        let (nn, normals) = self.normals(&v, &mut seed);
        o.pos = self.m.it.c;
        // the iteration count after the normals, as in MB3D's CalcHSMC
        let de_limited = self.m.it.it_result < self.m.max_its_result;
        (o.si_grad, o.otrap) = self.surface_colour(nn, de_limited);
        o.kind = 1;
        o.zz2 = zz2;
        o.normals = normals;
        o.last_difs = self.m.last_difs;
        o.ms_de_stop = self.m.ms_de_stop;
        o.seed = seed;
        o
    }

    /// Through a transparent object towards the light: steps on until
    /// outside again
    fn through(&mut self, r: &McReq) -> McRes {
        let mut seed = r.seed;
        let v = r.dir;
        let mut dt1 = r.len;
        self.m.ms_de_stop = r.ms_de_stop;
        self.advance(&v, dt1);
        let mut d = rand_of(&mut seed) * 8.0 * (self.m.ms_de_stop as f64).max(1.0);
        loop {
            d = -1.0 - d.abs();
            dt1 += d;
            self.advance(&v, d);
            d = self.m.calc_de();
            if d > self.m.ms_de_stop as f64 || !(dt1 > 0.0) {
                break;
            }
        }
        McRes { len: dt1, last_difs: self.m.last_difs, seed, ..Default::default() }
    }
}

/// A tracer's march waiting for the card: the request, then its result
#[derive(Default)]
struct Link {
    req: Cell<Option<McReq>>,
    res: Cell<Option<McRes>>,
}

/// Waits for the result of a link
struct Answer(Rc<Link>);

impl Future for Answer {
    type Output = McRes;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<McRes> {
        match self.0.res.take() {
            Some(r) => Poll::Ready(r),
            None => Poll::Pending,
        }
    }
}

/// Polls a future once: with the CPU's marcher it is finished then
fn now<F: Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    match f.as_mut().poll(&mut Context::from_waker(std::task::Waker::noop())) {
        Poll::Ready(v) => v,
        Poll::Pending => unreachable!("a march of the CPU waits"),
    }
}

/// All rays of a pixel's round at once: their results in order
struct JoinAll<'x, T> {
    futs: Vec<Option<Pin<Box<dyn Future<Output = T> + 'x>>>>,
    out: Vec<Option<T>>,
}

impl<'x, T: Unpin> Future for JoinAll<'x, T> {
    type Output = Vec<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Vec<T>> {
        let s = self.get_mut();
        let mut all = true;
        for (f, o) in s.futs.iter_mut().zip(s.out.iter_mut()) {
            if let Some(fut) = f {
                match fut.as_mut().poll(cx) {
                    Poll::Ready(v) => {
                        *o = Some(v);
                        *f = None;
                    }
                    Poll::Pending => all = false,
                }
            }
        }
        if all {
            Poll::Ready(s.out.iter_mut().map(|o| o.take().unwrap()).collect())
        } else {
            Poll::Pending
        }
    }
}

// ---------------------------------------------------------------------------
// one thread of the calculation

struct Tracer<'a, 'c> {
    c: &'c McCtx<'a>,
    /// the marches: the thread's marcher, or the card (`link`, collected
    /// in `outbox` for the next batch)
    cpu: Option<&'c RefCell<McMarch<'a, 'c>>>,
    link: Option<(Rc<Link>, &'c RefCell<Vec<Rc<Link>>>)>,
    /// the position (`Iteration::c` of MB3D's thread), the pixel's view
    /// vector, the depth along it and the inside flags (`bInsideRendering`
    /// and `bCalcInside` are always the same here)
    pos: Vec3,
    vfov: Vec3,
    mzz: f64,
    inside: bool,
    /// the last DE came from a dIFS part
    last_difs: bool,
    seed: i32,
    halton_x: f32,
    halton_y: f32,
    disc_x: f32,
    disc_y: f32,
    shift_x: f32,
    shift_y: f32,
    act_ray_nr: usize,
    normals: Vec3,
    hs6: Vec<S4>,
    hs_vecs: Vec<Vec3>,
    /// `tmpObjCol`: [specular (+ alpha), diffuse] of the last surface
    tmp_obj_col: [S4; 2],
    total_light: S3,
    calc_trans_r: bool,
    trans_flip_inside: bool,
    rit1: bool,
    calc_amb_shadow: bool,
    dyn_fog: f32,
    zpos_dyn_fog: f32,
    start_override: Option<Vec3>,
    /// `mPsiLight`: the colouring record of the primary ray
    main_si: SiLight,
    hit: bool,
    row_y: i32,
    guard: u32,
    pass_nr: u32,
}

/// The seed of the random numbers of a ray (`b` its number in the pixel)
/// or of a pixel's ray count in a pass: every ray has its own, so the rays
/// of a pixel do not depend on each other
fn seed_of(x: i32, y: i32, b: u32) -> i32 {
    let mix = |mut h: u32| {
        h ^= h >> 16;
        h = h.wrapping_mul(0x7FEB_352D);
        h ^= h >> 15;
        h = h.wrapping_mul(0x846C_A68B);
        h ^ (h >> 16)
    };
    mix((x as u32).wrapping_mul(0x9E37_79B1) ^ mix((y as u32).wrapping_mul(0x85EB_CA77) ^ mix(b.wrapping_add(0x1656_67B1)))) as i32
}

/// `CalcZposAndRough` without roughness (the MC renderer sets
/// `iSmNormals := 0`).
fn zpos_of(p: &CalcParams, zz: f64) -> u32 {
    let v = (p.zc_mul * ((zz.max(0.0) * p.zcorr + 1.0).sqrt() - 1.0)).round_ties_even();
    ((8388352.0 - v).clamp(0.0, 8388352.0) as u32) << 8
}

fn set_zpos_word(si: &mut SiLight, z: u32) {
    si.zpos_fine = (z.min(65535)) << 16;
}

impl<'a, 'c> Tracer<'a, 'c> {
    fn new(c: &'c McCtx<'a>, seed: i32) -> Self {
        let n = c.lv.lights.len();
        Tracer {
            c,
            cpu: None,
            link: None,
            pos: [0.0; 3],
            vfov: [0.0, 0.0, 1.0],
            mzz: 0.0,
            inside: c.p.inside_rendering,
            last_difs: false,
            seed,
            halton_x: 0.0,
            halton_y: 0.0,
            disc_x: 0.0,
            disc_y: 0.0,
            shift_x: 0.0,
            shift_y: 0.0,
            act_ray_nr: 0,
            normals: [0.0, 0.0, -1.0],
            hs6: vec![[0.0; 4]; n],
            hs_vecs: c.hs_vecs.clone(),
            tmp_obj_col: [[0.0; 4]; 2],
            total_light: [0.0; 3],
            calc_trans_r: false,
            trans_flip_inside: c.p.inside_rendering,
            rit1: true,
            calc_amb_shadow: false,
            dyn_fog: 0.0,
            zpos_dyn_fog: 0.0,
            start_override: None,
            main_si: SiLight::default(),
            hit: false,
            row_y: 0,
            guard: 0,
            pass_nr: 0,
        }
    }

    /// `GetRand`
    #[inline]
    fn rand(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
        (self.seed & 0x7FFF_FFFF) as f64 * SEED_MUL
    }

    #[inline]
    fn sw(&self) -> f64 {
        self.c.p.step_width
    }

    /// A tracer for one ray of the pixel: the same marcher (or a link of
    /// its own to the card)
    fn fork(&self) -> Self {
        let mut t = Tracer::new(self.c, 0);
        t.cpu = self.cpu;
        t.link = self.link.as_ref().map(|(_, outbox)| (Rc::new(Link::default()), *outbox));
        t.row_y = self.row_y;
        t.pass_nr = self.pass_nr;
        t
    }

    #[inline]
    fn toggle_inside(&mut self) {
        self.inside = !self.inside;
    }

    /// A march by the CPU's marcher, or by the card (waits for the batch)
    async fn march(&mut self, r: McReq) -> McRes {
        if let Some(m) = self.cpu {
            return m.borrow_mut().run(&r, self.vfov);
        }
        let (link, outbox) = self.link.as_ref().expect("a marcher");
        link.req.set(Some(r));
        outbox.borrow_mut().push(link.clone());
        Answer(link.clone()).await
    }

    /// `GenSphereSVecOm`: a direction on the unit sphere
    fn gen_sphere(&mut self) -> S3 {
        let (u, v) = if !self.c.do_dof {
            (self.disc_x as f64, self.disc_y as f64)
        } else {
            let v = self.rand();
            let u = self.rand();
            (u, v)
        };
        let (s, co) = (u * 2.0 * PI).sin_cos();
        let r = 2.0 * ((1.0 - v) * v).sqrt();
        [(r * co) as f32, (r * s) as f32, (1.0 - 2.0 * v) as f32]
    }

    /// `VaryVecSphere`: a cosine weighted direction around `vec`
    fn vary_vec_sphere(&mut self, vec: &mut Vec3) {
        let (o0, o1) = make_ortho(vec);
        let (a, s3v) = if !self.calc_amb_shadow {
            (self.halton_x as f64 * 2.0 * PI, self.halton_y as f64)
        } else {
            let a = self.rand() * 2.0 * PI;
            (a, self.rand())
        };
        let (s1, s2) = (a.sin() as f32 as f64, a.cos() as f32 as f64);
        let s4 = s3v.sqrt();
        let w = (1.0 - s3v).sqrt();
        for k in 0..3 {
            vec[k] = o0[k] * s1 * s4 + o1[k] * s2 * s4 + vec[k] * w;
        }
    }

    /// `CalcColor`: [specular (alpha = transparency), diffuse] of a surface
    fn calc_color(&self, si: &SiLight, z_pos: f32) -> [S4; 2] {
        let lv = self.c.lv;
        let idif0 = lv.s_col_zmul * z_pos;
        let (mut dif, spe, a) = if si.si_gradient > 32767 {
            let (d, s) = lv.calc_colors_inside(si, idif0);
            (d, s, s[0])
        } else {
            lv.calc_colors_alpha(si, idif0)
        };
        if let Some(d) = &lv.diff_map {
            let st = dif;
            let lns = self.c.cam.to_view(s3(&self.normals));
            let obj = s3(&sub(self.pos, self.c.mid));
            dif = lv.diff_map_color(d, si, lns, obj, 0.0, self.c.cam);
            if lv.yc_comb {
                dif = sc3(st, luma(dif) / (0.01 + luma(st)));
            }
        }
        let mut r = [[spe[0], spe[1], spe[2], a], [dif[0], dif[1], dif[2], 0.0]];
        if self.c.norm_sd_amount {
            let d2 = if self.c.calc_trans { 1.0 - r[0][3] } else { 1.0 };
            let sr = self.c.sr_light_amount;
            let m = (0..3).map(|k| r[0][k] * sr + r[1][k] * d2).fold(f32::MIN, f32::max);
            if m > 1.0 {
                for k in 0..3 {
                    r[0][k] /= m;
                    r[1][k] /= m;
                }
            }
        }
        r
    }

    /// `minLengthToCutPlane`
    fn min_length_to_cut_plane(&self, len: &mut f64, limit: f64, pos: &Vec3, v: &Vec3) {
        let p = self.c.p;
        for k in 0..3 {
            if (p.cut_options >> k) & 1 != 0 && v[k].abs() > 1e-20 {
                let t = (p.cut_pos[k] - pos[k]) / v[k];
                if t > limit && t < *len {
                    *len = t;
                }
            }
        }
    }

    /// `CalcVisLights`: visible light sources, depth fog and dynamic fog on
    /// the way of a ray to `vpos` (relative to the scene middle).
    fn calc_vis_lights(&mut self, vpos: S3, view: &Vec3, si: &SiLight, bg_dec: &mut f32, depth_dec: &mut f32) -> S3 {
        let lv = self.c.lv;
        let p = self.c.p;
        let vs = s3(&normalize(*view));
        let zp = si.zpos() as i32;
        let mut bgz = if zp < 32768 {
            (1.0 + (zp - 28000) as f32 * lv.s_depth).max(0.0)
        } else {
            (1.0 - (60768 - zp) as f32 * lv.s_depth).max(0.0)
        };
        if bgz < 1.0 {
            bgz = if lv.sqr { (1.0 - bgz) * (1.0 - bgz) } else { 1.0 - bgz };
            if lv.far_fog {
                bgz *= bgz;
            }
            bgz = 1.0 - bgz;
        }
        *depth_dec = bgz;
        *bg_dec = 1.0;
        let svbg = self.calc_bg_light(view, false, false, true);
        let mut res = [0f32; 3];
        if !self.calc_amb_shadow {
            for (ir, l) in lv.lights.iter().enumerate() {
                if l.visible == 0 {
                    continue;
                }
                let pos = l.positional;
                let stmp = if pos {
                    sqr_dist_sv(sub3(vpos, l.ln), vs)
                } else {
                    if zp < 32768 {
                        continue;
                    }
                    let sz = self.c.cam.to_view(vs);
                    (1.0 - dot3(l.ln, sz)).max(0.0)
                };
                let mut ldis = l.lmax_l * 1e-8;
                if stmp >= ldis {
                    continue;
                }
                let bgztmp = if pos {
                    let sz = sub3(l.ln, vpos);
                    if dot3(sz, vs) < 0.0 {
                        continue; // light behind the viewer
                    }
                    let pick = |a: [f64; 3]| -> f64 {
                        if vs[0].abs() > 0.5 {
                            a[0] / vs[0] as f64
                        } else if vs[1].abs() > 0.5 {
                            a[1] / vs[1] as f64
                        } else {
                            a[2] / vs[2] as f64
                        }
                    };
                    if zp < 32768 {
                        let c = self.pos;
                        let h = self.c.hs_vecs[ir];
                        let t = pick([c[0] - h[0], c[1] - h[1], c[2] - h[2]]);
                        if -((ldis - stmp).sqrt() as f64) > t {
                            continue; // light behind the object
                        }
                    }
                    let mut b = pick(d3(sz));
                    if b < 0.0 {
                        continue;
                    }
                    b = (8388352.0 - p.zc_mul * ((b / p.step_width * p.zcorr + 1.0).sqrt() - 1.0)) / 256.0;
                    let mut b = (1.0 + (b.clamp(0.0, 32767.0) as f32 - 28000.0) * lv.s_depth).max(0.0);
                    if b < 1.0 {
                        b = if lv.sqr { (1.0 - b) * (1.0 - b) } else { 1.0 - b };
                        if lv.far_fog {
                            b *= b;
                        }
                        b = 1.0 - b * 0.9;
                    }
                    b
                } else {
                    bgz * 0.9 + 0.1
                };
                let mut flux = stmp;
                pos_light_shape(&mut flux, &mut ldis, l.visible, pos, lv.sqr);
                addw3(&mut res, l.col, flux / 255.0 * bgztmp * *bg_dec);
                *bg_dec *= ldis.min(1.0);
            }
        }
        let mut dfog = (self.dyn_fog - lv.s_shad - lv.s_shad_zmul * self.zpos_dyn_fog) * lv.s_shad_gr;
        if lv.dfog_options & 2 != 0 {
            dfog = dfog.max(0.0);
        }
        let mut stmp = (self.dyn_fog * lv.s_dyn_fog_mul).min(1.0) * dfog;
        addw3(&mut res, svbg, (1.0 - bgz).max(0.0));
        if lv.dfog_options & 1 != 0 {
            dfog = dfog.clamp(0.0, 1.0);
            stmp = stmp.clamp(0.0, 1.0);
            res = sc3(res, 1.0 - dfog);
            *depth_dec *= 1.0 - dfog;
        }
        addw3(&mut res, lv.dyn_fog_col, (dfog - stmp) / 255.0);
        addw3(&mut res, lv.dyn_fog_col2, stmp / 255.0);
        res
    }

    /// `CalcBGLight`: background picture, ambient (`use_amb`) or depth
    /// colours in the direction `vec`; `hide_bmp` ignores the picture (only
    /// the depth colours, for blending).
    fn calc_bg_light(&self, vec: &Vec3, use_amb: bool, nn: bool, hide_bmp: bool) -> S3 {
        let lv = self.c.lv;
        let p = self.c.p;
        let mut res;
        if let Some(bg) = lv.bg.as_ref().filter(|_| !hide_bmp) {
            if lv.bg_direct {
                let t = normalize(rotate_vector(vec, &p.vgrads));
                let w = self.c.width as f64;
                let h = self.c.height as f64;
                let fovy1d = self.c.fovy1d as f64;
                let dtmp2 = h * fovy1d / w;
                let (mut x, mut y);
                if self.rit1 {
                    match p.optic {
                        CameraOptic::Planar => {
                            let d = p.pl_optic_z as f64 / not_zero(t[2]);
                            y = t[1] * d * fovy1d + 0.5;
                            x = t[0] * dtmp2 * d + 0.5;
                        }
                        CameraOptic::Panorama => {
                            y = arcsin_safe(t[1]) * fovy1d + 0.5;
                            x = t[0].atan2(t[2]) * (0.5 / PI) + 0.5;
                        }
                        CameraOptic::Common => {
                            let d = (0.5 + (0.25 + (t[0] * t[1]).powi(2)).sqrt()).sqrt();
                            y = arcsin_safe(t[1] * d) * fovy1d + 0.5;
                            x = arcsin_safe(t[0] * d) * dtmp2 + 0.5;
                        }
                    }
                } else {
                    y = arcsin_safe(t[1]) * fovy1d + 0.5;
                    let d = p.fov_y / (2.0 * PI - p.fov_y).max(0.1);
                    if y > 1.0 {
                        y = 1.0 + d * (1.0 - y);
                        if y < 0.75 {
                            y = 1.5 - y;
                        }
                    } else if y < 0.0 {
                        y = y.abs() * d;
                        if y > 0.25 {
                            y = 0.5 - y;
                        }
                    }
                    x = t[0].atan2(t[2]) * dtmp2 + 0.5;
                    let d = 1.0 / (dtmp2 * (2.0 * PI - 1.0 / dtmp2).max(0.1));
                    if x > 1.0 {
                        x = 1.0 + d * (1.0 - x);
                    } else if x < 0.0 {
                        x = x.abs() * d;
                    }
                }
                let (x, y) = (x.clamp(0.0, 1.0) as f32, y.clamp(0.0, 1.0) as f32);
                res = if nn { bg.map.pixel_nn(x, y, lv.sqr) } else { bg.map.pixel_t(x, y, 0, lv.sqr) };
            } else {
                let v = s3(vec);
                res = if nn { bg.map.sphere_pixel_nn(v, Some(&bg.rot), lv.sqr) } else { bg.map.sphere_pixel_t(v, Some(&bg.rot), lv.sqr) };
            }
            res = sc3(res, bg.intensity);
        } else {
            let mut t = *vec;
            if !lv.amb_rel_obj {
                t = rotate_vector(&t, &p.vgrads);
            }
            let t = normalize(t);
            if use_amb {
                let d = (t[1] * 0.5 + 0.5) as f32;
                res = sc3(lin2(lv.amb_col2, lv.amb_col, d), 1.0 / 256.0);
            } else {
                let mut d = if !lv.amb_rel_obj {
                    if self.rit1 {
                        self.row_y as f32 / self.c.height as f32
                    } else {
                        (arcsin_safe(t[1]) * self.c.fovy1d as f64 + 0.5).clamp(0.0, 1.0) as f32
                    }
                } else {
                    (arcsin_safe(t[1]) / PI + 0.5).clamp(0.0, 1.0) as f32
                };
                if !lv.amb_rel_obj {
                    match lv.depth_func {
                        1 => d *= d,
                        2 => d = d.max(0.0).sqrt(),
                        _ => {}
                    }
                }
                res = sc3(lin2(lv.depth_col2, lv.depth_col, d), 1.0 / 256.0);
            }
        }
        if use_amb {
            // light maps
            for lm in &lv.map_lights {
                let sv = s3(&rotate_vector_reverse(vec, &p.vgrads));
                addw3(&mut res, lm.map.sphere_pixel_nn(sv, Some(&lm.rot), lv.sqr), lm.intensity);
            }
        }
        res
    }

    /// `CalcHSMC`: shadows (and coloured light through transparent objects)
    /// towards each light, as factors in `hs6`
    async fn calc_hs_mc(&mut self) {
        let c = self.c;
        let p = c.p;
        let lv = c.lv;
        let zz = self.mzz.abs() as f32;
        let ic = self.pos;
        let max_lhs = c.hs_max_lmul * (1.0 + 0.5 * zz.min(p.zend as f32 * 0.4) * (p.fov_y as f32).max(0.0) / c.height as f32);
        let sw = p.step_width;
        for i in 0..lv.lights.len() {
            let l = &lv.lights[i];
            self.hs6[i] = [1.0; 4];
            self.pos = ic;
            let zz2 = self.mzz;
            let mut dt1 = max_lhs as f64;
            let v = if l.positional {
                let v = sub(self.hs_vecs[i], self.pos);
                let d = sqr_len(&v);
                if d > (l.lmax_l * c.hs_max_len_mul) as f64 {
                    continue;
                }
                if d < (dt1 * sw).powi(2) {
                    dt1 = d.sqrt() / sw;
                }
                norm_to(sw, v)
            } else {
                scale(self.hs_vecs[i], -1.0)
            };
            if p.cut_options != 0 {
                // HSminLengthToCutPlane
                for k in 0..3 {
                    if (p.cut_options >> k) & 1 != 0 && v[k].abs() > 1e-20 {
                        let t = (p.cut_pos[k] - self.pos[k]) / v[k];
                        if t > 0.0 && t < dt1 {
                            dt1 = t;
                        }
                    }
                }
            }
            if dt1 <= 0.001 {
                continue;
            }
            if dot(&self.normals, &v) < 0.0 {
                self.hs6[i] = [0.0; 4];
                continue;
            }
            let zz2mul = dot(&v, &self.vfov) / (sqr_len(&v) * sqr_len(&self.vfov)).sqrt();
            let dmaxl = dt1;
            let h = self
                .march(McReq { kind: 1, pos: ic, dir: v, zz: zz as f64, zz2, zzplus: zz2mul, len: dt1, seed: self.seed, inside: self.inside, trans: self.calc_trans_r, ..Default::default() })
                .await;
            self.seed = h.seed;
            dt1 = h.len;
            if dt1 <= 0.01 {
                continue;
            }
            let fade = (((dmaxl - dt1) / max_lhs as f64) as f32).powi(8).min(1.0);
            if !self.calc_trans_r {
                for k in 0..3 {
                    self.hs6[i][k] *= fade;
                }
                continue;
            }
            // a transparent object between surface and light
            self.mzz = h.zz2.abs();
            self.pos = h.pos;
            self.last_difs = h.last_difs;
            let nvec = self.normals;
            self.normals = h.normals;
            let mut hsi = SiLight { si_gradient: h.si_grad, otrap: h.otrap, ..Default::default() };
            hsi.zpos_fine = zpos_of(p, h.zz2);
            let obj = self.calc_color(&hsi, (self.mzz * sw + p.zz_stmit_dif) as f32);
            let difs = self.last_difs;
            if obj[0][3] > 0.0 && (!c.only_difs || difs) {
                if p.in_and_outside {
                    let a = spow(first3(obj[1]), c.absorption);
                    for k in 0..3 {
                        self.hs6[i][k] *= a[k];
                    }
                    self.hs6[i][3] = 0.0;
                } else {
                    let w = ((dot(&self.normals, &v) / sw) as f32).powi(2);
                    self.hs6[i][3] = w;
                    for k in 0..3 {
                        self.hs6[i][k] *= w;
                    }
                    // through the object: step on until outside again
                    let t = self
                        .march(McReq { kind: 2, pos: self.pos, dir: v, len: dt1, seed: self.seed, inside: !self.inside, ms_de_stop: h.ms_de_stop, ..Default::default() })
                        .await;
                    self.seed = t.seed;
                    dt1 = t.len;
                    self.last_difs = t.last_difs;
                    if difs != self.last_difs {
                        self.hs6[i] = [0.0; 4];
                    } else {
                        let a = spow(first3(obj[1]), (dt1.max(0.0) as f32) * c.absorption);
                        let s = obj[0][3] * obj[0][0].max(obj[0][1]).max(obj[0][2]);
                        for k in 0..3 {
                            self.hs6[i][k] *= a[k] * s;
                        }
                        self.hs6[i][3] = 0.0;
                    }
                }
            } else {
                for k in 0..3 {
                    self.hs6[i][k] *= fade;
                }
            }
            self.normals = nvec;
        }
        self.pos = ic;
    }

    /// `CalcPhongLight`: direct light with soft shadows from lights with a
    /// size (`MCSoftShadowRadius`): [specular, diffuse]
    async fn calc_phong_light(&mut self, reflect: &Vec3) -> [S3; 2] {
        let c = self.c;
        let lv = c.lv;
        let sw = self.sw();
        let mut res = [[0f32; 3]; 2];
        let saved = c.hs_vecs.clone();
        for (i, l) in lv.lights.iter().enumerate() {
            if l.positional {
                let d = if l.visible == 2 || l.visible == 8 {
                    self.rand().powi(2) * 1e-4 * c.soft_shadow_radius as f64
                } else {
                    self.rand().sqrt() * 3e-4 * c.soft_shadow_radius as f64
                };
                let g = self.gen_sphere();
                let t = norm_to((l.lmax_l as f64).sqrt() * d, d3(g));
                self.hs_vecs[i] = add(saved[i], [t[0] as f32 as f64, t[1] as f32 as f64, t[2] as f32 as f64]);
            } else {
                let (stmp, s1) = if !c.do_dof {
                    (self.disc_x as f64, self.disc_y as f64)
                } else {
                    let a = self.rand();
                    (a, self.rand())
                };
                let (s1, s2) = ((2.0 * PI * s1).sin(), (2.0 * PI * s1).cos());
                let k = if l.visible == 8 { 1e-5 } else { 2.5e-5 };
                let d = stmp.sqrt() * (l.lmax_l as f64).sqrt() * k * c.soft_shadow_radius as f64 * sw;
                let (o0, o1) = make_ortho(&saved[i]);
                let mut v = saved[i];
                for q in 0..3 {
                    v[q] += ((o0[q] * s1 * d) as f32 + (o1[q] * s2 * d) as f32) as f64;
                }
                self.hs_vecs[i] = norm_to(sw, v);
            }
        }
        self.calc_hs_mc().await;
        for (i, l) in lv.lights.iter().enumerate() {
            let hs = self.hs6[i];
            if hs[0] != 0.0 || hs[1] != 0.0 || hs[2] != 0.0 {
                let mut l0 = l.col;
                let t = if l.positional {
                    let t = sub(self.pos, saved[i]);
                    l0 = sc3(l0, (1.0 / (sqr_len(&t) + 1e-26)) as f32);
                    norm_to(sw, t)
                } else {
                    saved[i]
                };
                l0 = mul3(l0, first3(hs));
                let sp = fast_int_pow((dot(reflect, &t) * hs[3] as f64 / -(sw * sw)) as f32, l.pow_func) * lv.s_spec / 255.0;
                addw3(&mut res[0], l0, sp);
                let df = ((dot(&self.normals, &t) / -sw).max(0.0) as f32).powi(2) * lv.s_diff / 255.0;
                addw3(&mut res[1], l0, df);
            }
            self.hs_vecs[i] = saved[i];
        }
        res
    }

    /// `CalcPhongLightNoHS`: light on a cutting plane surface
    fn calc_phong_light_no_hs(&mut self, reflect: &Vec3) -> [S3; 2] {
        let c = self.c;
        let lv = c.lv;
        let sw = self.sw();
        let mut res = [[0f32; 3]; 2];
        for (i, l) in lv.lights.iter().enumerate() {
            let (sl, v) = if l.positional {
                let v = sub(self.pos, self.hs_vecs[i]);
                (sc3(l.col, (1.0 / (sqr_len(&v) + 1e-60)) as f32), norm_to(sw, v))
            } else {
                (l.col, self.hs_vecs[i])
            };
            addw3(&mut res[0], sl, fast_int_pow((dot(reflect, &v) / -(sw * sw)) as f32, l.pow_func) * lv.s_spec / 255.0);
            addw3(&mut res[1], sl, ((dot(&self.normals, &v) / -sw).max(0.0) as f32).powi(2) * lv.s_diff / 255.0);
        }
        let mut v = self.normals;
        self.vary_vec_sphere(&mut v);
        let sl = self.calc_bg_light(&v, true, true, false);
        addw3(&mut res[1], sl, lv.s_diff);
        res
    }

    /// `AddLight`: light scattered inside transparent material on the way
    /// from `cam` to `vpos`
    fn add_light(&self, cam: S3, vpos: S3) -> S3 {
        let lv = self.c.lv;
        let mut res = [0f32; 3];
        let s3w = (self.sw() as f32).powi(2);
        for l in &lv.lights {
            let stmp = if l.positional {
                let sv = sub3(vpos, cam);
                let sv2 = sub3(l.ln, cam);
                let sv3 = sub3(l.ln, vpos);
                let d2 = dot3(sv2, sv2) + s3w;
                let d3v = dot3(sv3, sv3) + s3w;
                let st = dot3(sv2, sv);
                if st <= 0.0 {
                    0.25 / d3v + 1.0 / d2
                } else {
                    let st2 = dot3(sv, sv);
                    if st2 <= st {
                        0.25 / d2 + 1.0 / d3v
                    } else {
                        let q = sub3(l.ln, add3(cam, sc3(sv, st / st2)));
                        1.0 / (dot3(q, q) + s3w) + 0.25 / d3v.max(d2)
                    }
                }
            } else {
                1.0
            };
            addw3(&mut res, l.col, stmp);
        }
        mul3(res, first3(self.tmp_obj_col[1]))
    }


    /// Random direction jitter of reflections (`MCdiffReflects`)
    fn diffuse_jitter(&mut self, v: &Vec3) -> Vec3 {
        let sw = self.sw();
        let (o0, o1) = make_ortho(v);
        let a = 2.0 * PI * self.rand();
        let (zs, zc) = (a.sin(), a.cos());
        let ml = self.rand() * self.c.diff_reflects as f64 * 0.00004;
        let d = ml.sqrt() * sw;
        let w = (1.0 - ml).sqrt();
        let mut r = [0.0; 3];
        for k in 0..3 {
            r[k] = ((o0[k] * zs * d) as f32 + (o1[k] * zc * d) as f32) as f64 + v[k] * w;
        }
        norm_to(sw, r)
    }

    /// `CalcRay`: follows a ray (length StepWidth per step in `dir`) and
    /// adds its light, weighted by `t_absorb`, to `total_light`; recurses
    /// for reflections, transmission and the ambient bounce.  `probe`: the
    /// primary ray, which first looks whether it starts inside the set (then
    /// it returns the march's result for `ray`).
    fn calc_ray<'x>(&'x mut self, zz: f64, dir: Vec3, t_absorb: S3, rit: i32, probe: bool) -> Pin<Box<dyn Future<Output = Option<McRes>> + 'x>> {
        Box::pin(self.calc_ray_at(zz, dir, t_absorb, rit, probe))
    }

    async fn calc_ray_at(&mut self, mut zz: f64, dir: Vec3, mut t_absorb: S3, mut rit: i32, probe: bool) -> Option<McRes> {
        let c = self.c;
        let p = c.p;
        let lv = c.lv;
        let sw = p.step_width;
        if t_absorb[0].abs() * 0.3 + t_absorb[1].abs() * 0.59 + t_absorb[2].abs() * 0.11 < 1e-4 {
            return None;
        }
        self.guard += 1;
        if self.guard > 4096 {
            return None;
        }
        let mut max_l = if self.calc_amb_shadow {
            c.amb_max_l * (self.rand().powi(2) + 0.75)
        } else {
            p.zend
        };
        rit += 1;
        self.rit1 = rit == 1;
        let rit1 = self.rit1;
        let mut cr = if rit1 { self.main_si } else { SiLight::default() };
        let mut zz2;
        if rit1 {
            zz2 = zz;
        } else {
            if p.cut_options != 0 {
                let pos = self.pos;
                self.min_length_to_cut_plane(&mut max_l, 0.1, &pos, &dir);
            }
            zz2 = 0.0;
        }
        let mut bg_dec = 1f32;
        let start = self.start_override.unwrap_or(self.pos);
        let vpos = s3(&sub(start, c.mid));
        let zzplus = dot(&dir, &self.vfov) / (sw * sw);
        let h = self.march(McReq { kind: 0, pos: self.pos, dir, zz, zz2, zzplus, len: max_l, seed: self.seed, inside: self.inside, probe, ..Default::default() }).await;
        self.seed = h.seed;
        self.inside = h.inside;
        self.pos = h.pos;
        self.last_difs = h.last_difs;
        if h.kind == 2 {
            return Some(h);
        }
        self.start_override = None;
        self.calc_trans_r = if !c.calc_trans {
            false
        } else if !c.only_difs {
            true
        } else {
            h.start_difs
        };
        let mut calc_t = self.calc_trans_r;
        zz = h.zz;
        zz2 = h.zz2;
        self.dyn_fog = h.dyn_fog;
        let open_air = h.kind == 0;
        let mut d1 = 1f32;
        let cc;
        let obj;
        if !open_air {
            self.hit = true;
            self.mzz = zz.abs();
            self.normals = h.normals;
            cr.si_gradient = h.si_grad;
            cr.otrap = h.otrap;
            cc = self.pos;
            cr.zpos_fine = zpos_of(p, zz2);
            if rit1 {
                self.main_si = cr;
            }
            obj = self.calc_color(&cr, (self.mzz * sw + p.zz_stmit_dif) as f32);
            self.zpos_dyn_fog = (zz2 * sw + p.zz_stmit_dif) as f32;
            let sv = self.calc_vis_lights(vpos, &dir, &cr, &mut bg_dec, &mut d1);
            self.total_light = add3(self.total_light, mul3(sv, t_absorb));
        } else {
            // open air: background
            self.zpos_dyn_fog = (p.zend * sw + p.zz_stmit_dif) as f32;
            let main_z = if rit1 { cr.zpos() } else { self.main_si.zpos() } as i32;
            if (lv.far_fog || lv.sqr) && lv.s_depth.abs() > 1e-10 {
                let both = lv.far_fog && lv.sqr;
                let mut e1 = 1.0 - (1.0 + (main_z - 28000) as f32 * lv.s_depth).max(0.0);
                if e1 > 0.0 {
                    e1 *= e1;
                    if both {
                        e1 *= e1;
                    }
                }
                let mut e2 = 1.0 - (1.0 - 28000.0 * lv.s_depth).max(0.0);
                if e2 > 0.0 {
                    e2 *= e2;
                    if both {
                        e2 *= e2;
                    }
                }
                let mut e = e2 - e1;
                if e > 0.0 {
                    e = e.sqrt();
                    if both {
                        e = e.sqrt();
                    }
                }
                set_zpos_word(&mut cr, (60768.0 - e / lv.s_depth).clamp(32768.0, 65535.0).round_ties_even() as u32);
            } else {
                set_zpos_word(&mut cr, 32768);
            }
            if rit1 {
                self.main_si = cr;
            }
            let mut rsf = 0f32;
            let mut sv = self.calc_vis_lights(vpos, &dir, &cr, &mut bg_dec, &mut rsf);
            let bgl = self.calc_bg_light(&dir, self.calc_amb_shadow, c.diff_reflects_big_enough && !rit1, false);
            if lv.bg.is_none() || !lv.bg_add_light {
                bg_dec *= rsf;
            }
            addw3(&mut sv, bgl, bg_dec);
            if calc_t && !p.in_and_outside && (self.trans_flip_inside ^ self.inside) {
                // light scattering inside the material
                let sv2 = spow(first3(self.tmp_obj_col[1]), (zz2.min(max_l) as f32) * c.absorption);
                t_absorb = mul3(t_absorb, sv2);
                let r = (1.0 - luma(sv2)) * c.light_scattering_mul;
                if r != 0.0 {
                    let pos = s3(&sub(self.pos, c.mid));
                    let al = self.add_light(vpos, pos);
                    let add = mul3(sc3(mul3(al, first3(self.tmp_obj_col[1])), r), t_absorb);
                    self.total_light = add3(self.total_light, add);
                }
            }
            self.total_light = add3(self.total_light, mul3(sv, t_absorb));
            return None;
        }

        if calc_t && !p.in_and_outside && (self.trans_flip_inside ^ self.inside) {
            // absorption inside the material (colour of the last surface)
            let sv = spow(first3(self.tmp_obj_col[1]), (zz2.min(max_l) as f32) * c.absorption);
            let sv2 = t_absorb;
            let r = (1.0 - luma(sv)) * c.light_scattering_mul;
            t_absorb = mul3(t_absorb, sv);
            let pos = s3(&sub(self.pos, c.mid));
            let al = self.add_light(vpos, pos);
            let add = mul3(al, lin2(t_absorb, sv2, 0.5));
            addw3(&mut self.total_light, add, r);
            d1 = 1.0;
        } else {
            t_absorb = sc3(t_absorb, bg_dec * d1);
        }
        let reflect = sub(dir, scale(self.normals, 2.0 * dot(&self.normals, &dir)));
        self.mzz = zz;
        calc_t = if !c.calc_trans {
            false
        } else if !c.only_difs {
            true
        } else {
            self.last_difs
        };
        let light_sd = self.calc_phong_light(&reflect).await;
        let mut d2 = 1f32;
        let d3v;
        let mut d4 = 0f32;
        let mut rsf_max = 0f32;
        let mut tvec2 = [0.0; 3];
        if calc_t {
            rsf_max = obj[0][0].max(obj[0][1]).max(obj[0][2]);
            let dstep = if !p.in_and_outside && (self.trans_flip_inside ^ self.inside) {
                c.trans_di_const as f64
            } else {
                1.0 / c.trans_di_const as f64
            };
            let dl = len(&dir);
            if dl < 1e-100 {
                return None;
            }
            let dd3 = -dot(&dir, &self.normals) / dl;
            let k = 1.0 - dstep * dstep * (1.0 - dd3 * dd3);
            if k <= 0.0 {
                // total internal reflection
                d3v = 1.0 - obj[0][3] + obj[0][3] * obj[0][3] / (rsf_max + 0.01);
                calc_t = false;
            } else {
                tvec2 = norm_to(dl, sub(dir, scale(self.normals, dl / dstep * (k.sqrt() - dstep * dd3))));
                let dt = dot(&tvec2, &self.normals).abs() / dl;
                d4 = dd3.abs() as f32;
                if dt.abs() > 1e-16 {
                    let (a, b, ds) = (d4 as f64, dt, dstep);
                    d4 = ((((a - ds * b) / (a + ds * b)).powi(2) + ((b - ds * a) / (b + ds * a)).powi(2)) * 0.5) as f32;
                }
                d3v = d4 + (1.0 - d4) * (1.0 - obj[0][3]);
                if p.in_and_outside {
                    tvec2 = dir;
                }
            }
            d2 = 1.0 - obj[0][3] * c.sr_light_amount;
        } else {
            d3v = 1.0;
        }
        let spec = mul3(sc3(first3(obj[0]), (0.25 + d3v) * 0.8), light_sd[0]);
        let diff = mul3(sc3(first3(obj[1]), d2), light_sd[1]);
        self.total_light = add3(self.total_light, mul3(add3(spec, diff), t_absorb));
        if d1 > 1.0 {
            t_absorb = sc3(t_absorb, 1.0 / d1);
        }
        let nvec = self.normals;
        let mut skip_trans = false;
        if rit <= c.max_spec_depth {
            if c.calc_reflects {
                self.tmp_obj_col = obj;
                let backup = self.calc_amb_shadow;
                let mut rv = reflect;
                let mut skip = false;
                if c.diff_reflects > 0 {
                    rv = self.diffuse_jitter(&rv);
                    if dot(&rv, &self.normals) <= 0.0 {
                        skip = true;
                    }
                }
                if !skip {
                    let w = sc3(first3(obj[0]), d3v * c.sr_light_amount);
                    self.calc_ray(zz, rv, mul3(t_absorb, w), rit, false).await;
                    self.calc_amb_shadow = backup;
                }
            }
            if calc_t {
                if c.diff_reflects > 0 {
                    tvec2 = self.diffuse_jitter(&tvec2);
                    if dot(&tvec2, &nvec) >= 0.0 {
                        skip_trans = true;
                    }
                }
                if !skip_trans {
                    let d4b = (1.0 - d4 * rsf_max) * obj[0][3];
                    self.pos = cc;
                    self.tmp_obj_col = obj;
                    let backup = self.calc_amb_shadow;
                    self.toggle_inside();
                    self.calc_ray(zz, tvec2, sc3(t_absorb, d4b * c.sr_light_amount), rit, false).await;
                    self.calc_amb_shadow = backup;
                    self.toggle_inside();
                }
            }
        }
        if rit < c.max_amb_depth || !self.calc_amb_shadow {
            let (a, ml) = if !self.calc_amb_shadow {
                (2.0 * PI * self.halton_x as f64, self.halton_y as f64)
            } else {
                let a = 2.0 * PI * self.rand();
                (a, self.rand())
            };
            let (d4s, d3c) = (a.sin() as f32 as f64, a.cos() as f32 as f64);
            let dd = ml.sqrt();
            let (o0, o1) = make_ortho(&nvec);
            let w = (1.0 - ml).sqrt();
            let mut nv = [0.0; 3];
            for k in 0..3 {
                nv[k] = ((o0[k] * d3c * dd) as f32 + (o1[k] * d4s * dd) as f32) as f64 + nvec[k] * w;
            }
            let nv = norm_to(sw, nv);
            self.pos = cc;
            self.tmp_obj_col = obj;
            self.calc_amb_shadow = true;
            let w = sc3(first3(obj[1]), d2);
            self.calc_ray(zz, nv, mul3(t_absorb, w), rit, false).await;
        }
        None
    }

    /// `GetSubPixelShift`: -0.5..0.5 (box) or gaussian
    fn sub_pixel_shift(&mut self) -> (f32, f32) {
        if !self.c.gauss_aa {
            let x = (self.rand() - 0.5) as f32;
            let y = (self.rand() - 0.5) as f32;
            (x, y)
        } else {
            let (mut x, mut y, mut r);
            loop {
                x = (self.rand() * 2.0 - 1.0) as f32;
                y = (self.rand() * 2.0 - 1.0) as f32;
                r = x * x + y * y;
                if r <= 1.0 {
                    break;
                }
            }
            r = r * 0.99 + 0.01;
            r = (-2.0 * r.ln() / r).sqrt() * 0.5;
            (x * r, y * r)
        }
    }

    /// `CalculateVgradsFOV` for a fractional pixel position
    fn calc_vgrads_fov(&mut self, x: f32, y: f32) {
        let p = self.c.p;
        let cafx = ((p.fovx_off - x - 1.0) * p.fovx_mul) as f64;
        let cafy = (y as f64 / p.height as f64 - 0.5) * p.fov_y;
        let v = match p.optic {
            CameraOptic::Planar => normalize([-cafx, cafy, p.pl_optic_z as f64]),
            CameraOptic::Panorama => build_view_vector_dsphere_fov(cafy, cafx),
            CameraOptic::Common => build_view_vector_dfov(cafy, cafx),
        };
        self.vfov = rotate_vector_reverse(&v, &p.vgrads);
    }

    /// `DoDOF`: start the ray on the lens disc, aimed at the focus point
    fn do_dof(&mut self) {
        let sw = self.sw();
        let mut zpos = self.pos;
        add_weight(&mut zpos, &self.vfov, self.c.dof_zsharp as f64);
        let (o0, o1) = make_ortho(&self.vfov);
        // CalcBokehMC
        let h = halton_seq()[self.act_ray_nr];
        let mut xx = frac_s(h.hdx as f32 * D1D65535 + self.shift_x);
        let mut yy = frac_s(h.hdy as f32 * D1D65535 + self.shift_y);
        make_disc(&mut xx, &mut yy);
        let s = calc_bokeh(xx, yy, self.c.bokeh_nr) * self.c.dof_aperture * self.c.dof_zsharp * sw as f32;
        self.disc_x = xx * s;
        self.disc_y = yy * s;
        for k in 0..3 {
            self.pos[k] += ((o0[k] as f32 * self.disc_x) as f64) + ((o1[k] as f32 * self.disc_y) as f64);
        }
        self.vfov = norm_to(sw, sub(zpos, self.pos));
    }

    /// `CalcNormalsOnCutMC`
    fn normals_on_cut(&mut self, cut_plane: usize) {
        let vg = &self.c.p.vgrads;
        let n = if cut_plane != 0 {
            let k = cut_plane - 1;
            let nn = if vg[2][k].abs() < 1e-40 { -1e40 } else { -1.0 / vg[2][k] };
            [vg[0][k] * nn, vg[1][k] * nn, -1.0]
        } else {
            [0.0, 0.0, -1.0]
        };
        self.normals = normalize(rotate_vector_reverse(&n, vg));
    }

    /// One ray through pixel (x1 = 1-based column, y = row); returns the
    /// light of the ray.
    async fn ray(&mut self, x1: i32, y: i32, rec_rays: u16, itcount: i32) -> S3 {
        let c = self.c;
        let p = c.p;
        let sw = p.step_width;
        self.guard = 0;
        self.act_ray_nr = (rec_rays as usize + itcount as usize).min(65535);
        self.seed = seed_of(x1, y, rec_rays as u32 + itcount as u32);
        let (sx, sy) = halton_2d_shifts(x1, y);
        self.shift_x = sx;
        self.shift_y = sy;
        let h = halton_seq()[self.act_ray_nr];
        self.halton_x = frac_s(h.hrx as f32 * D1D65535 + sx);
        self.halton_y = frac_s(h.hry as f32 * D1D65535 + sy);
        self.inside = p.inside_rendering;
        set_zpos_word(&mut self.main_si, 32768);
        self.main_si.shadow = 0;
        self.main_si.si_gradient = 0;
        let (xx, yy) = if rec_rays == 0 {
            (x1 as f32 - (itcount & 1) as f32 * 0.5 - 0.75, y as f32 + (itcount & 2) as f32 * 0.25 - 0.25)
        } else {
            let (gx, gy) = self.sub_pixel_shift();
            (gx + x1 as f32 - 1.0, gy + y as f32)
        };
        self.calc_vgrads_fov(xx, yy);
        if p.optic == CameraOptic::Panorama {
            self.pos = p.ystart;
        } else {
            for k in 0..3 {
                self.pos[k] = p.ystart[k] + p.vgrads[1][k] * yy as f64 + p.vgrads[0][k] * xx as f64;
            }
        }
        if c.do_dof {
            self.do_dof();
        } else {
            self.disc_x = frac_s(h.hdx as f32 * D1D65535 + sx);
            self.disc_y = frac_s(h.hdy as f32 * D1D65535 + sy);
        }
        self.total_light = [0.0; 3];
        self.calc_amb_shadow = false;
        self.rit1 = true;
        let mut zz = 0f64;
        self.dyn_fog = 0.0;
        let mut cut_plane = 0usize;
        self.start_override = None;
        let vf = self.vfov;
        if p.cut_options != 0 {
            let (len, plane) = self.max_length_to_cut_plane();
            zz = len;
            cut_plane = plane;
            self.start_override = Some(self.pos);
            let mut beyond = false;
            if zz >= p.zend {
                beyond = true;
                zz = p.zend;
            }
            add_weight(&mut self.pos, &vf, zz);
            if beyond {
                self.rit1 = false;
                self.calc_ray(zz, vf, [1.0; 3], 0, false).await;
                return self.total_light;
            }
        }
        let Some(h) = self.calc_ray(zz, vf, [1.0; 3], 0, true).await else {
            return self.total_light;
        };
        // the start is inside the set (cutting plane or start plane)
        self.hit = true;
        self.zpos_dyn_fog = (zz * sw + p.zz_stmit_dif) as f32;
        let mut si = self.main_si;
        si.si_gradient = h.si_grad;
        si.otrap = h.otrap;
        self.normals_on_cut(cut_plane);
        self.mzz = zz;
        let lsd = self.calc_color(&si, zz as f32);
        si.zpos_fine = zpos_of(p, zz);
        self.main_si = si;
        let start = self.start_override.unwrap_or(self.pos);
        let (mut a, mut b) = (0f32, 0f32);
        let mut tl = self.calc_vis_lights(s3(&sub(start, c.mid)), &vf, &si, &mut a, &mut b);
        let tmpv = sub(vf, scale(self.normals, 2.0 * dot(&self.normals, &vf)));
        let lsd2 = self.calc_phong_light_no_hs(&tmpv);
        let w = a * b;
        let add = add3(mul3(first3(lsd[0]), lsd2[0]), mul3(first3(lsd[1]), lsd2[1]));
        addw3(&mut tl, add, w);
        if c.calc_reflects {
            let dt = w;
            let mut si2 = si;
            set_zpos_word(&mut si2, 32768);
            self.main_si = si2;
            let (mut a2, mut b2) = (0f32, 0f32);
            let pos = s3(&sub(self.pos, c.mid));
            let sv = self.calc_vis_lights(pos, &tmpv, &si2, &mut a2, &mut b2);
            let mut x2 = a2;
            if c.lv.bg.is_none() || !c.lv.bg_add_light {
                x2 *= b2;
            }
            let bgl = self.calc_bg_light(&tmpv, false, false, false);
            let r = mul3(add3(sv, sc3(bgl, x2)), first3(lsd[0]));
            addw3(&mut tl, r, c.sr_light_amount * dt);
        }
        self.total_light = tl;
        tl
    }

    /// `RMmaxLengthToCutPlane`: distance (in steps) from the start position
    /// to the cutting planes along the view ray, and the plane (1..3) hit.
    fn max_length_to_cut_plane(&self) -> (f64, usize) {
        let p = self.c.p;
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
                let t = (p.cut_pos[k] - self.pos[k]) / self.vfov[k];
                if t > len {
                    len = t;
                    plane = k + 1;
                }
            }
        }
        (len, plane)
    }

    /// `CalcN`: how many more rays the pixel gets in this pass, from its
    /// noise and the contrast to its neighbours (`snap` = records at the
    /// start of the pass)
    fn calc_n(&mut self, x1: i32, y1: i32, rec: &McRecord, snap: &[McRecord], avrg_vari: f64, avrg_rcount: f32) -> i32 {
        let w = self.c.width;
        let h = self.c.height;
        let at = |x: i32, y: i32| -> &McRecord { &snap[((y - 1) * w + (x - 1)) as usize] };
        let noise = |r: &McRecord| r.ysqr - r.ysum * r.ysum;
        let (mut maxyd, mut maxyd2, mut maxnoise) = (0f64, 0f64, 0f64);
        let (mut n, mut n2) = (0, 0);
        let mut avg_rc = 0i32;
        let dt2 = rec.ysum;
        if y1 > 1 {
            let r = at(x1, y1 - 1);
            let dt = r.ysum;
            maxyd = (dt - dt2).powi(2);
            maxnoise = noise(r);
            avg_rc = r.ray_count as i32;
            n += 1;
            if x1 > 1 {
                let r3 = at(x1 - 1, y1 - 1);
                let dt3 = r3.ysum;
                maxyd += (dt2 - dt3).powi(2);
                maxyd2 = (dt - dt3).powi(2);
                maxnoise = maxnoise.max(noise(r3));
                avg_rc += r3.ray_count as i32;
                n += 1;
                n2 += 1;
            }
            if x1 < w {
                let r3 = at(x1 + 1, y1 - 1);
                let dt3 = r3.ysum;
                maxyd += (dt2 - dt3).powi(2);
                maxyd2 += (dt - dt3).powi(2);
                maxnoise = maxnoise.max(noise(r3));
                avg_rc += r3.ray_count as i32;
                n += 1;
                n2 += 1;
            }
        }
        let mut dt = 0f64;
        if x1 > 1 {
            let r = at(x1 - 1, y1);
            dt = r.ysum;
            maxyd += (dt - dt2).powi(2);
            maxnoise = maxnoise.max(noise(r));
            avg_rc += r.ray_count as i32;
            n += 1;
        }
        if x1 < w {
            let r = at(x1 + 1, y1);
            let dt3 = r.ysum;
            maxyd += (dt3 - dt2).powi(2);
            if x1 > 1 {
                maxyd2 += (dt - dt3).powi(2);
                n2 += 1;
            }
            maxnoise = maxnoise.max(noise(r));
            avg_rc += r.ray_count as i32;
            n += 1;
        }
        if y1 < h {
            let r = at(x1, y1 + 1);
            let dt = r.ysum;
            maxyd += (dt - dt2).powi(2);
            maxnoise = maxnoise.max(noise(r));
            avg_rc += r.ray_count as i32;
            n += 1;
            if x1 > 1 {
                let r3 = at(x1 - 1, y1 + 1);
                let dt3 = r3.ysum;
                maxyd += (dt2 - dt3).powi(2);
                maxyd2 += (dt - dt3).powi(2);
                maxnoise = maxnoise.max(noise(r3));
                avg_rc += r3.ray_count as i32;
                n += 1;
                n2 += 1;
            }
            if x1 < w {
                let r3 = at(x1 + 1, y1 + 1);
                let dt3 = r3.ysum;
                maxyd += (dt2 - dt3).powi(2);
                maxyd2 += (dt - dt3).powi(2);
                maxnoise = maxnoise.max(noise(r3));
                avg_rc += r3.ray_count as i32;
                n += 1;
                n2 += 1;
            }
        }
        let n = n.max(1) as f64;
        let n2 = n2.max(1) as f64;
        maxyd /= n;
        maxyd2 /= n2;
        let dt3 = (rec.ysqr - dt2 * dt2) / (dt2.max(0.0) + 0.01);
        let dm = if !rec.hit {
            0.0
        } else {
            (maxyd.max(maxyd2) - 20.0 * avrg_vari).max(0.0) * (rec.ray_count as f64 - avg_rc as f64 / n).clamp(-4.0, 4.0)
        };
        let d = (dt3.max(maxnoise) / ((dt2 / (1.0 + (dt2 * 0.9).powi(2)).sqrt()) * rec.ray_count as f64).max(0.3) - dm).max(0.0);
        let d2 = d * 4.0 / avrg_vari;
        if d2 >= 1.0 {
            (avrg_rcount as f64 * 0.5 + 8.0).min(d2).round_ties_even() as i32
        } else if self.rand() * 1.1 - 0.1 > d2.max(0.0) {
            0
        } else {
            1
        }
    }

    /// One pixel of `TMCCalcThread.Execute` (rays until the noise is low).
    #[allow(clippy::too_many_arguments)]
    async fn pixel(&mut self, x1: i32, y: i32, rec: &mut McRecord, snap: &[McRecord], skip_nonzero: bool, avrg_vari: f64, avrg_rcount: f32, max_new: i32) {
        let sqr = self.c.lv.sqr;
        self.seed = seed_of(x1, y, 0x8000_0000 | self.pass_nr);
        let mut total = if rec.ray_count == 0 {
            4
        } else if skip_nonzero {
            return;
        } else {
            self.calc_n(x1, y + 1, rec, snap, avrg_vari, avrg_rcount)
        };
        self.hit = rec.hit;
        loop {
            if total + rec.ray_count as i32 > 65535 {
                total = 65535 - rec.ray_count as i32;
            }
            let mut at = [0f32; 3];
            let (mut sum, mut sqr_sum) = (0f64, 0f64);
            let mut itcount = 0;
            // the rays of the round, each with its own tracer (and seed)
            let rc = rec.ray_count;
            let rays = (0..total.max(0))
                .map(|i| {
                    let mut t = self.fork();
                    Some(Box::pin(async move {
                        let l = t.ray(x1, y, rc, i).await;
                        (l, t.hit)
                    }) as Pin<Box<dyn Future<Output = (S3, bool)> + 'c>>)
                })
                .collect::<Vec<_>>();
            let n = rays.len();
            let lights = JoinAll { futs: rays, out: (0..n).map(|_| None).collect() }.await;
            for (l, hit) in lights {
                self.hit |= hit;
                let sv = [l[0].clamp(0.0, 400.0), l[1].clamp(0.0, 400.0), l[2].clamp(0.0, 400.0)];
                let yv = luma(sv) as f64;
                sqr_sum += yv * yv;
                sum += yv;
                at = add3(at, sv);
                itcount += 1;
            }
            rec.hit = self.hit;
            if itcount == 0 {
                return;
            }
            let dtmp = rec.ysum;
            let zz = rec.ysqr;
            let itmp = rec.ray_count as i32;
            let yy = if itmp != 0 { ((zz - dtmp * dtmp) * 10.0).max(0.03) } else { 0.0 };
            let light = if sqr {
                let a = sc3(at, 1.0 / itcount as f32);
                sc3([if a[0] > 0.0 { a[0].sqrt() } else { a[0] }, if a[1] > 0.0 { a[1].sqrt() } else { a[1] }, if a[2] > 0.0 { a[2].sqrt() } else { a[2] }], itcount as f32)
            } else {
                at
            };
            let xx = 1.0 / (itmp + itcount) as f64;
            for k in 0..3 {
                rec.col[k] = clamp_mc((rec.col[k] as f64 * itmp as f64 + light[k] as f64) * xx) as f32;
            }
            rec.ysum = clamp_mc((dtmp * itmp as f64 + sum) * xx);
            rec.ysqr = clamp_mc((zz * itmp as f64 + sqr_sum) * xx);
            rec.ray_count = (itmp + itcount) as u16;
            let mean = sum / itcount as f64;
            if itmp != 0 && (rec.ray_count as i32) < max_new && (mean - dtmp).powi(2) > yy {
                total = (((mean - dtmp).powi(2) / yy).sqrt() * (itmp.min(itcount).max(4)) as f64).round_ties_even() as i32;
                continue;
            }
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// passes

/// The scene as the MC renderer calculates it (`CalcMCT`): the far plane
/// at least 10 % of the image diagonal away, no tiling.
fn mc_scene(sc: &Scene) -> Scene {
    let mut s = sc.clone();
    let sw = s.step_width();
    let los = ((s.width as f64).powi(2) + (s.height as f64).powi(2)).sqrt();
    s.z_end = (s.z_end - s.z_start).max(los * 0.1 * sw) + s.z_start;
    s.tiling = None;
    s.calc_rect = None;
    s
}

/// One pass of the Monte Carlo calculation over the whole image
/// (`TMCForm.StartCalc` + `CalcMCT`).  The first pass shoots 4 rays per
/// pixel, the following ones add rays where the noise is high.
/// `progress(done_rows, rows)`; `cancel` is checked per row.
pub fn pass(
    sc: &Scene,
    img: &mut McImage,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(), String> {
    pass_rows(sc, img, threads, progress, cancel, None)
}

/// [`pass`], handing every finished row (its number and records) to
/// `rows`, so the image can be shown while the pass runs (MB3D draws the
/// lines as they are calculated).
pub fn pass_rows(
    sc: &Scene,
    img: &mut McImage,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    rows: Option<&(dyn Fn(usize, &[McRecord]) + Sync)>,
) -> Result<(), String> {
    let t0 = std::time::Instant::now();
    let sc = mc_scene(sc);
    if img.width != sc.width as usize || img.height != sc.height as usize {
        *img = McImage::new(sc.width as usize, sc.height as usize);
    }
    let mut p = CalcParams::new(&sc)?;
    if p.slice_2d != 0 {
        return Err("the Monte Carlo renderer needs a 3D calculation".into());
    }
    p.sm_normals = 0;
    let threads = if threads == 0 { crate::render::thread_count(&sc) } else { threads }.min(sc.height as usize).max(1);
    let l = sc.lighting.clone();
    let cam = crate::render::paint_camera(&sc, &p);
    let lv = crate::render::light_vals(&sc, &l, &cam);
    if let Some(vp) = sc.vol_light.as_ref() {
        if let Some((ln, positional)) = lv.light_ln(vp.light) {
            let hs_len = sc.shadows.map(|h| h.max_len_mul).unwrap_or(1.0);
            let map = crate::vollight::build(&p, sc.stereo_mid(), ln, positional, l.lights[vp.light].amplitude, vp, hs_len, threads);
            p.vol = Some(std::sync::Arc::new(map));
        }
    }
    let mid = sc.stereo_mid();
    // CalcHSVecsFromLights
    let m1 = normalise_matrix_to(1.0, &p.vgrads);
    let hs_vecs: Vec<Vec3> = lv
        .lights
        .iter()
        .map(|l| {
            if l.positional {
                add(d3(l.ln), mid)
            } else {
                let v = norm_to(-p.step_width, d3(l.ln));
                rotate_vector_reverse(&v, &m1)
            }
        })
        .collect();
    let mc: McSettings = sc.mc;
    let calc_reflects = mc.reflections;
    let los = ((sc.width as f64).powi(2) + (sc.height as f64).powi(2)).sqrt();
    let deao_max = sc.deao.map(|d| d.max_len).unwrap_or(crate::deao::DeaoParams::default().max_len) as f64;
    let hs_max_len_mul = sc.shadows.map(|h| h.max_len_mul).unwrap_or(1.0);
    let (do_dof, dof_ap, dof_z) = match &sc.dof {
        Some(d) => (true, (d.aperture * 0.5).max(1e-7), (((d.z_sharp + d.z_sharp2) * 0.5) * sc.width as f32).max(0.01)),
        None => (false, 1e-7, 0.01),
    };
    let ctx = McCtx {
        p: &p,
        lv: &lv,
        cam: &cam,
        mid,
        hs_vecs,
        max_amb_depth: mc.depth as i32,
        max_spec_depth: mc.reflection_depth as i32,
        calc_reflects,
        calc_trans: calc_reflects && mc.transparency,
        only_difs: mc.only_difs,
        secant_search: mc.options & 2 != 0,
        norm_sd_amount: calc_reflects && mc.options & 4 != 0,
        diff_reflects: mc.diffuse_reflects as i32,
        trans_di_const: mc.refraction_index,
        absorption: (mc.absorption as f64 * p.step_width) as f32,
        light_scattering_mul: mc.scattering / 330.0,
        sr_light_amount: mc.reflection_amount.clamp(0.0, 100.0),
        amb_max_l: (deao_max * los * 3.0).min(p.zend * 255.0 / 1.75),
        do_dof,
        dof_aperture: dof_ap,
        dof_zsharp: dof_z,
        fovy1d: (1.0 / if p.fov_y == 0.0 { 1e-30 } else { p.fov_y }) as f32,
        diff_reflects_big_enough: lv.bg.as_ref().is_some_and(|b| b.map.width * mc.diffuse_reflects as usize > 5000),
        bokeh_nr: ((mc.options >> 4) & 7) as i32,
        gauss_aa: mc.options & 8 != 0,
        soft_shadow_radius: mc.soft_shadow_radius,
        hs_max_len_mul,
        hs_max_lmul: (sc.width + sc.height) as f32 * 0.6 * hs_max_len_mul,
        vol: p.vol.is_some(),
        width: sc.width,
        height: sc.height,
    };
    // StartCalc / CalcAvrgNoise
    let stats = img.stats();
    let fresh = img.recs.iter().all(|r| r.ray_count == 0);
    let (avrg_sqr_noise, skip_nonzero) = if fresh { (1.0, true) } else { (img.avg_sqr_noise(), stats.zero_counts) };
    if !fresh {
        img.avrg_rcount = stats.avg_rays as f32;
    }
    img.avrg_sqr_noise = avrg_sqr_noise;
    let avrg_vari = avrg_sqr_noise.max(0.001);
    let avrg_rcount = img.avrg_rcount;
    let max_new = ((avrg_rcount + avrg_rcount * avrg_rcount * 0.2) * 10.0).round() as i32;
    let snap = img.recs.clone();
    let w = img.width;
    let h = img.height;
    let pass_nr = img.passes;
    let done = std::sync::atomic::AtomicUsize::new(0);
    let ctx = &ctx;
    let snap = &snap;
    let done = &done;
    #[cfg(feature = "gpu")]
    if crate::gpu::enabled() && crate::gpu::unsupported(&p).is_none() {
        let pa = PassArgs { snap, skip_nonzero, avrg_vari, avrg_rcount, max_new, pass_nr: pass_nr as u32 };
        match pass_gpu(ctx, img, threads, &pa, progress, cancel, rows) {
            Ok(status) => {
                *MC_STATUS.lock().unwrap() = status;
                img.seconds += t0.elapsed().as_secs_f64();
                if cancel() {
                    return Err("cancelled".into());
                }
                img.passes += 1;
                return Ok(());
            }
            Err(e) if e == "cancelled" => return Err(e),
            // nothing of the pass is kept: it runs on the CPU again
            Err(e) => eprintln!("Monte Carlo on the GPU failed, calculated on the CPU: {e}"),
        }
    }
    *MC_STATUS.lock().unwrap() = "CPU".into();
    std::thread::scope(|s| {
        let mut per: Vec<Vec<(usize, &mut [McRecord])>> = (0..threads).map(|_| Vec::new()).collect();
        for (y, row) in img.recs.chunks_mut(w).enumerate() {
            per[y % threads].push((y, row));
        }
        for list in per {
            s.spawn(move || {
                let march = RefCell::new(McMarch::new(ctx));
                for (y, row) in list {
                    if cancel() {
                        break;
                    }
                    let mut t = Tracer::new(ctx, 0);
                    t.cpu = Some(&march);
                    t.row_y = y as i32;
                    t.pass_nr = pass_nr as u32;
                    for (x, rec) in row.iter_mut().enumerate() {
                        now(t.pixel(x as i32 + 1, y as i32, rec, snap, skip_nonzero, avrg_vari, avrg_rcount, max_new));
                    }
                    if let Some(f) = rows {
                        f(y, row);
                    }
                    let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    progress(d, h);
                }
            });
        }
    });
    img.seconds += t0.elapsed().as_secs_f64();
    if cancel() {
        return Err("cancelled".into());
    }
    img.passes += 1;
    Ok(())
}

/// Where the last pass was calculated ("CPU" or the GPU's status).
static MC_STATUS: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Where the last pass was calculated.
pub fn last_status() -> String {
    MC_STATUS.lock().unwrap().clone()
}

/// The per-pass values of [`Tracer::pixel`].
#[cfg(feature = "gpu")]
struct PassArgs<'s> {
    snap: &'s [McRecord],
    skip_nonzero: bool,
    avrg_vari: f64,
    avrg_rcount: f32,
    max_new: i32,
    pass_nr: u32,
}

/// Rounds smaller than this run on the CPU (a batch of the card has a fixed
/// cost).
#[cfg(feature = "gpu")]
const MC_CPU_RAYS: usize = 2000;

/// The pixels each thread keeps going at once (their rays wait for the
/// card together).
#[cfg(feature = "gpu")]
const MC_ACTIVE: usize = 1 << 16;

/// A round's batch shared by the card and the threads: the card starts
/// the tasks from the front, the threads take pieces from the end
/// (`share`), and then those the card gave up (`left`, once `published`).
#[cfg(feature = "gpu")]
struct SharedRound {
    reqs: Vec<McReq>,
    cpu_res: Vec<std::sync::OnceLock<McRes>>,
    share: crate::gpu::RaySharing,
    left: std::sync::Mutex<Vec<usize>>,
    published: std::sync::atomic::AtomicBool,
}

#[cfg(feature = "gpu")]
impl SharedRound {
    /// A thread's part: pieces from the end until the card is done, then
    /// the tasks the card gave up
    fn help(&self, m: &mut McMarch) {
        use std::sync::atomic::Ordering;
        const PIECE: usize = 16;
        loop {
            let piece = {
                let mut st = self.share.state.lock().unwrap();
                let lo = st.1.saturating_sub(PIECE).max(st.0);
                let r = lo..st.1;
                st.1 = lo;
                r
            };
            if piece.is_empty() {
                break;
            }
            for i in piece.rev() {
                let _ = self.cpu_res[i].set(m.run(&self.reqs[i], [0.0, 0.0, 1.0]));
            }
        }
        while !self.published.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        loop {
            let Some(i) = self.left.lock().unwrap().pop() else { break };
            let _ = self.cpu_res[i].set(m.run(&self.reqs[i], [0.0, 0.0, 1.0]));
        }
    }
}

/// What a thread tells the pass: its marches of the round (None: its
/// pixels are done), or that it finished helping with a batch.
#[cfg(feature = "gpu")]
enum ToMain {
    Round(usize, Option<Vec<McReq>>),
    Helped,
}

/// The answer to a thread's round: the results, or "march them yourself",
/// or "help with the batch, then wait for the results".
#[cfg(feature = "gpu")]
enum ToWorker {
    Results(Vec<McRes>),
    Cpu,
    Help(std::sync::Arc<SharedRound>),
}

/// A pass with the marches on the graphics card: every thread runs many
/// pixels at once (each a future that waits for its marches); in every
/// round the threads hand their marches to this thread, which sends them
/// to the card as one batch while the threads march from its end.
/// Returns the status of the card.
#[cfg(feature = "gpu")]
fn pass_gpu(
    ctx: &McCtx,
    img: &mut McImage,
    threads: usize,
    pa: &PassArgs,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    rows: Option<&(dyn Fn(usize, &[McRecord]) + Sync)>,
) -> Result<String, String> {
    use std::sync::atomic::Ordering;
    use std::sync::mpsc;
    let w = img.width;
    let h = img.height;
    let p = ctx.p;
    let done_rows = std::sync::atomic::AtomicUsize::new(0);
    let (to_main, from_workers) = mpsc::channel::<ToMain>();
    let mut replies = Vec::new();
    let mut inboxes = Vec::new();
    for _ in 0..threads {
        let (tx, rx) = mpsc::channel::<ToWorker>();
        replies.push(tx);
        inboxes.push(rx);
    }
    let failed: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    let mut status = String::new();
    let share_cpu = std::env::var("MB3D_MC_CPU_SHARE").map_or(true, |v| v != "0");
    std::thread::scope(|s| {
        let mut per: Vec<Vec<(usize, &mut [McRecord])>> = (0..threads).map(|_| Vec::new()).collect();
        for (y, row) in img.recs.chunks_mut(w).enumerate() {
            per[y % threads].push((y, row));
        }
        for ((wi, list), inbox) in per.into_iter().enumerate().zip(inboxes) {
            let to_main = to_main.clone();
            let done_rows = &done_rows;
            s.spawn(move || {
                let march = RefCell::new(McMarch::new(ctx));
                let outbox: RefCell<Vec<Rc<Link>>> = RefCell::new(Vec::new());
                let active_max = (MC_ACTIVE / threads).max(64);
                // the pixels in row order: (row, column)
                let mut queue = list.iter().flat_map(|(y, row)| (0..row.len()).map(move |x| (*y, x))).collect::<std::collections::VecDeque<_>>();
                let mut row_of: std::collections::HashMap<usize, (Vec<McRecord>, usize)> = list.iter().map(|(y, row)| (*y, (row.to_vec(), row.len()))).collect();
                let mut slices: std::collections::HashMap<usize, &mut [McRecord]> = list.into_iter().collect();
                type Px<'x> = Pin<Box<dyn Future<Output = McRecord> + 'x>>;
                let mut active: Vec<(usize, usize, Px)> = Vec::new();
                let mut cx = Context::from_waker(std::task::Waker::noop());
                loop {
                    if cancel() {
                        let _ = to_main.send(ToMain::Round(wi, None));
                        return;
                    }
                    while active.len() < active_max {
                        let Some((y, x)) = queue.pop_front() else { break };
                        let rec = row_of[&y].0[x];
                        let outbox = &outbox;
                        let fut = Box::pin(async move {
                            let mut t = Tracer::new(ctx, 0);
                            t.link = Some((Rc::new(Link::default()), outbox));
                            t.row_y = y as i32;
                            t.pass_nr = pa.pass_nr;
                            let mut r = rec;
                            t.pixel(x as i32 + 1, y as i32, &mut r, pa.snap, pa.skip_nonzero, pa.avrg_vari, pa.avrg_rcount, pa.max_new).await;
                            r
                        }) as Px;
                        active.push((y, x, fut));
                    }
                    // the futures run until they wait for a march
                    let mut k = 0;
                    while k < active.len() {
                        if let Poll::Ready(r) = active[k].2.as_mut().poll(&mut cx) {
                            let (y, x, _) = active.swap_remove(k);
                            let e = row_of.get_mut(&y).unwrap();
                            e.0[x] = r;
                            e.1 -= 1;
                            if e.1 == 0 {
                                let (recs, _) = row_of.remove(&y).unwrap();
                                let row = slices.remove(&y).unwrap();
                                row.copy_from_slice(&recs);
                                if let Some(f) = rows {
                                    f(y, row);
                                }
                                let d = done_rows.fetch_add(1, Ordering::Relaxed) + 1;
                                progress(d, h);
                            }
                        } else {
                            k += 1;
                        }
                    }
                    let links: Vec<Rc<Link>> = std::mem::take(&mut *outbox.borrow_mut());
                    if links.is_empty() {
                        if active.is_empty() && queue.is_empty() {
                            let _ = to_main.send(ToMain::Round(wi, None));
                            return;
                        }
                        // only new pixels: start them in the next turn
                        continue;
                    }
                    let reqs: Vec<McReq> = links.iter().map(|l| l.req.take().unwrap()).collect();
                    if to_main.send(ToMain::Round(wi, Some(reqs.clone()))).is_err() {
                        return;
                    }
                    let mut answer = inbox.recv();
                    if let Ok(ToWorker::Help(round)) = &answer {
                        round.help(&mut march.borrow_mut());
                        let _ = to_main.send(ToMain::Helped);
                        answer = inbox.recv();
                    }
                    match answer {
                        Ok(ToWorker::Results(res)) => {
                            for (l, r) in links.iter().zip(res) {
                                l.res.set(Some(r));
                            }
                        }
                        // the CPU does this round
                        Ok(ToWorker::Cpu) => {
                            let mut m = march.borrow_mut();
                            for (l, r) in links.iter().zip(&reqs) {
                                // (the view vector only matters for volumetric light)
                                l.res.set(Some(m.run(r, [0.0, 0.0, 1.0])));
                            }
                        }
                        _ => return,
                    }
                }
            });
        }
        drop(to_main);
        // (dropped when this thread panics: the workers end then)
        let replies = std::mem::take(&mut replies);
        // the batches of the rounds
        let mut live = threads;
        let mut on_card = true;
        let mut batches = 0usize;
        let (mut n_card, mut n_cpu) = (0usize, 0usize);
        let timing = std::env::var_os("MB3D_MC_TIMING").is_some();
        let (mut t_card, mut t_wait) = (0f64, 0f64);
        while live > 0 {
            let tw = std::time::Instant::now();
            let mut round: Vec<(usize, Vec<McReq>)> = Vec::new();
            // one message from every live worker
            while round.len() < live {
                match from_workers.recv() {
                    Ok(ToMain::Round(wi, Some(r))) => round.push((wi, r)),
                    Ok(ToMain::Round(_, None)) => live -= 1,
                    Ok(ToMain::Helped) => {}
                    Err(_) => {
                        live = 0;
                        break;
                    }
                }
            }
            t_wait += tw.elapsed().as_secs_f64();
            let total: usize = round.iter().map(|r| r.1.len()).sum();
            if total == 0 {
                continue;
            }
            if !on_card || total < MC_CPU_RAYS {
                n_cpu += total;
                for (wi, _) in &round {
                    let _ = replies[*wi].send(ToWorker::Cpu);
                }
                continue;
            }
            let all: Vec<McReq> = round.iter().flat_map(|r| r.1.iter().copied()).collect();
            let sr = std::sync::Arc::new(SharedRound {
                cpu_res: (0..all.len()).map(|_| std::sync::OnceLock::new()).collect(),
                share: crate::gpu::RaySharing {
                    // (without sharing the card's limit is all of them)
                    state: std::sync::Mutex::new((if share_cpu { 0 } else { all.len() }, all.len())),
                    done: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    tail: if share_cpu { threads * 4 } else { 0 },
                },
                reqs: all,
                left: std::sync::Mutex::new(Vec::new()),
                published: std::sync::atomic::AtomicBool::new(false),
            });
            for (wi, _) in &round {
                let _ = replies[*wi].send(ToWorker::Help(sr.clone()));
            }
            let tc = std::time::Instant::now();
            let r = crate::gpu::mc_batch(p, &sr.reqs, ctx.secant_search, Some(&sr.share));
            let card_s = tc.elapsed().as_secs_f64();
            t_card += card_s;
            let (res, took, gave_up) = match r {
                Ok(v) => v,
                Err(e) => {
                    // the rest of the pass on the CPU (this batch: the
                    // threads march what the card did not)
                    if e == "cancelled" {
                        *failed.lock().unwrap() = Some(e);
                    } else {
                        eprintln!("Monte Carlo on the GPU failed, calculated on the CPU: {e}");
                    }
                    on_card = false;
                    (Vec::new(), 0, Vec::new())
                }
            };
            {
                // what the card did not finish: its own part it gave up, or
                // all of it when it failed
                let mut left = sr.left.lock().unwrap();
                *left = gave_up;
                if !on_card {
                    let lim = sr.share.state.lock().unwrap().1;
                    left.extend(0..lim.min(total));
                }
            }
            sr.published.store(true, Ordering::SeqCst);
            // the threads finish their parts
            let mut helped = 0;
            while helped < round.len() {
                match from_workers.recv() {
                    Ok(ToMain::Helped) => helped += 1,
                    Ok(_) => unreachable!("a thread with a round of its own while helping"),
                    Err(_) => break,
                }
            }
            let n_cpu_b = sr.cpu_res.iter().filter(|c| c.get().is_some()).count();
            if timing {
                eprintln!("  batch of {total}: {:.1} ms on the card ({took} taken, {n_cpu_b} on the CPU), {:.1} ms in all", card_s * 1000.0, tc.elapsed().as_secs_f64() * 1000.0);
            }
            batches += 1;
            n_cpu += n_cpu_b;
            n_card += total - n_cpu_b;
            let mut it = (0..total).map(|i| sr.cpu_res[i].get().copied().unwrap_or_else(|| res[i]));
            for (wi, r) in &round {
                let _ = replies[*wi].send(ToWorker::Results(it.by_ref().take(r.len()).collect()));
            }
        }
        if timing {
            eprintln!("  pass: {t_card:.2} s on the card, {t_wait:.2} s waiting for the threads");
        }
        status = format!("GPU {} ({batches} batches, {n_card} marches on the card, {n_cpu} on the CPU)", crate::gpu::name().unwrap_or_default());
    });
    if let Some(e) = failed.into_inner().unwrap() {
        return Err(e);
    }
    Ok(status)
}

// ---------------------------------------------------------------------------
// painting (`TPaintThreadMC.PaintColorBuffer`)

/// `SVecRGB2Lab` (lightness 0..1)
fn rgb_to_lab(v: S3) -> S3 {
    let sftc = 216.0 / (29.0f32 * 29.0 * 29.0);
    let s4d29 = 4.0 / 29.0f32;
    let smul = 29.0 * 29.0 / 108.0f32;
    let mut x = [
        (0.412453 * v[0] + 0.35758 * v[1] + 0.180423 * v[2]) * 1.052111,
        0.212671 * v[0] + 0.71516 * v[1] + 0.072169 * v[2],
        (0.019334 * v[0] + 0.119193 * v[1] + 0.950227 * v[2]) * 0.918417,
    ];
    for c in x.iter_mut() {
        *c = if *c > sftc { c.cbrt() } else { smul * *c + s4d29 };
    }
    [1.16 * x[1] - 0.16, x[0] - x[1], x[1] - x[2]]
}

/// `SVecLab2RGB`
fn lab_to_rgb(v: S3) -> S3 {
    let sftc = 6.0 / 29.0f32;
    let s4d29 = 4.0 / 29.0f32;
    let smul = 108.0 / 841.0f32;
    let y = (v[0] + 0.16) / 1.16;
    let mut x = [y + v[1], y, y - v[2]];
    for c in x.iter_mut() {
        *c = if *c > sftc { *c * *c * *c } else { (*c - s4d29) * smul };
    }
    x[0] *= 0.95047;
    x[2] *= 1.08883;
    [
        3.240479 * x[0] - 1.537150 * x[1] - 0.498535 * x[2],
        -0.969256 * x[0] + 1.875992 * x[1] + 0.041556 * x[2],
        0.055648 * x[0] - 0.204043 * x[1] + 1.057311 * x[2],
    ]
}

/// The displayed image of the records: exposure (`MCcontrast`), colour
/// saturation, the gamma slider of the lighting and HDR soft clipping.
/// Pixels without rays are black.
pub fn paint(img: &McImage, mc: &McSettings, gamma: f32) -> Vec<u8> {
    let mut contrast = (mc.contrast as f32 / 256.0 + 0.5).powi(2);
    let soft_clip = mc.options & 1 != 0;
    let saturation = (mc.saturation & 0x7F) as f32 / 32.0;
    let mut igamma = ((gamma.round() as i32).clamp(0, 63) << 2) & 0xFC;
    if soft_clip {
        igamma = (igamma - 20).max(0);
        contrast *= 1.1;
    }
    let (gmode, sgamma) = if igamma > 128 {
        (1, (igamma - 128) as f32 / 127.0)
    } else if igamma < 128 {
        (-1, 1.0 - igamma as f32 / 128.0)
    } else {
        (0, 1.0)
    };
    let mut out = vec![0u8; img.width * img.height * 3];
    for (o, r) in out.chunks_mut(3).zip(&img.recs) {
        if r.ray_count == 0 {
            continue;
        }
        let mut v = sc3(r.col, contrast).map(|c| c.max(0.0) * c.max(0.0));
        v = rgb_to_lab(v);
        let mut scol = saturation;
        if v[0] > 0.0 {
            if gmode != 0 {
                let s2 = v[0];
                let s = if gmode > 0 { v[0].sqrt() } else { v[0] * v[0] };
                v[0] += (s - v[0]) * sgamma;
                if gmode > 0 && s2 > 1.0 {
                    scol *= v[0] / s2;
                }
            }
            if soft_clip {
                let s = v[0];
                let x2 = s * s;
                v[0] = (x2 / ((x2 * 0.9).powi(2) + 1.0).sqrt()).sqrt();
                scol *= v[0] / s;
            }
        }
        v[1] *= scol;
        v[2] *= scol;
        let v = lab_to_rgb(v).map(|c| c.max(0.0).sqrt());
        for k in 0..3 {
            o[k] = (v[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// .m3c files: parameters + records (`SaveM3C`, `Button9Click`)

const REC_SIZE: usize = 18;

/// MB3D's `.m3c` file: the parameters (`.m3p` layout) followed by the
/// records, 18 bytes each.
pub fn write_m3c(sc: &Scene, img: &McImage) -> Vec<u8> {
    let mut s = sc.clone();
    s.width = img.width as i32;
    s.height = img.height as i32;
    let mut d = crate::m3p::write(&s);
    d.reserve(img.recs.len() * REC_SIZE);
    for r in &img.recs {
        for v in [r.col[0] as f64, r.col[1] as f64, r.col[2] as f64, r.ysum, r.ysqr] {
            d.extend_from_slice(&f64_to_mcrgb(v).to_le_bytes()[..3]);
        }
        d.extend_from_slice(&r.ray_count.to_le_bytes());
        d.push(if r.hit { 128 } else { 0 });
    }
    d
}

/// Reads a `.m3c` file: the scene and the records rendered so far.
pub fn read_m3c(data: &[u8]) -> Result<(crate::m3p::M3pFile, McImage), String> {
    let hl = crate::m3p::write(&Scene::default()).len();
    if data.len() < hl {
        return Err("not a Monte Carlo file (.m3c): too short".into());
    }
    let f = crate::m3p::parse(&data[..hl])?;
    let (w, h) = (f.scene.width as usize, f.scene.height as usize);
    if data[124] & 128 != 0 {
        return Err("this .m3c file uses the experimental record format of a MB3D development version".into());
    }
    let recs = &data[hl..];
    if recs.len() < w * h * REC_SIZE {
        return Err(format!("the .m3c file is incomplete ({} of {} pixels)", recs.len() / REC_SIZE, w * h));
    }
    let mut img = McImage::new(w, h);
    let u24 = |b: &[u8]| b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16;
    for (i, r) in img.recs.iter_mut().enumerate() {
        let b = &recs[i * REC_SIZE..(i + 1) * REC_SIZE];
        r.col = [mcrgb_to_f64(u24(&b[0..3])) as f32, mcrgb_to_f64(u24(&b[3..6])) as f32, mcrgb_to_f64(u24(&b[6..9])) as f32];
        r.ysum = mcrgb_to_f64(u24(&b[9..12]));
        r.ysqr = mcrgb_to_f64(u24(&b[12..15]));
        r.ray_count = u16::from_le_bytes([b[15], b[16]]);
        r.hit = b[17] & 128 != 0;
    }
    img.passes = 1;
    Ok((f, img))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halton_values() {
        assert!((halton(1, 2) - 0.5).abs() < 1e-12);
        assert!((halton(2, 2) - 0.25).abs() < 1e-12);
        assert!((halton(3, 2) - 0.75).abs() < 1e-12);
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-12);
        assert!((halton(4, 3) - 4.0 / 9.0).abs() < 1e-12);
        let h = halton_seq();
        assert_eq!(h.len(), 65536);
        assert_eq!(h[0].hrx, 32768);
    }

    #[test]
    fn disc_stays_in_unit_circle() {
        for i in 0..2000 {
            let h = halton_seq()[i];
            let (mut x, mut y) = (h.hdx as f32 * D1D65535, h.hdy as f32 * D1D65535);
            make_disc(&mut x, &mut y);
            assert!(x * x + y * y <= 1.0001, "{x} {y}");
        }
    }

    #[test]
    fn mcrgb_roundtrip() {
        for v in [-1.0, -0.5, 0.0, 0.123, 1.0, 1.99, 2.5, 10.0, 399.0] {
            let r = mcrgb_to_f64(f64_to_mcrgb(v));
            assert!((r - v).abs() < 1e-5 * (1.0 + v.abs()), "{v} -> {r}");
        }
    }

    #[test]
    fn lab_roundtrip() {
        for v in [[0.2f32, 0.5, 0.8], [1.0, 1.0, 1.0], [0.01, 0.0, 0.3]] {
            let r = lab_to_rgb(rgb_to_lab(v));
            for k in 0..3 {
                assert!((r[k] - v[k]).abs() < 1e-3, "{v:?} {r:?}");
            }
        }
    }

    fn small_scene() -> Scene {
        let mut sc = Scene::preset("bulb").unwrap();
        sc.width = 48;
        sc.height = 36;
        sc.threads = 2;
        sc
    }

    #[test]
    fn passes_converge() {
        let sc = small_scene();
        let mut img = McImage::new(48, 36);
        pass(&sc, &mut img, 2, &|_, _| {}, &|| false).unwrap();
        assert!(img.recs.iter().all(|r| r.ray_count == 4));
        let hits = img.recs.iter().filter(|r| r.hit).count();
        assert!(hits > 200 && hits < 48 * 36, "hits {hits}");
        let s1 = img.stats();
        pass(&sc, &mut img, 2, &|_, _| {}, &|| false).unwrap();
        pass(&sc, &mut img, 2, &|_, _| {}, &|| false).unwrap();
        let s2 = img.stats();
        assert!(s2.avg_rays > s1.avg_rays, "{s1:?} {s2:?}");
        let rgb = paint(&img, &sc.mc, sc.lighting.gamma);
        let lit = rgb.chunks(3).filter(|c| c.iter().any(|&v| v > 20)).count();
        assert!(lit > 500, "lit {lit}");
        // thread count does not matter
        let mut a = McImage::new(48, 36);
        let mut b = McImage::new(48, 36);
        pass(&sc, &mut a, 1, &|_, _| {}, &|| false).unwrap();
        pass(&sc, &mut b, 3, &|_, _| {}, &|| false).unwrap();
        assert_eq!(a.recs, b.recs);
    }

    #[test]
    fn m3c_roundtrip() {
        let sc = small_scene();
        let mut img = McImage::new(48, 36);
        pass(&sc, &mut img, 2, &|_, _| {}, &|| false).unwrap();
        let d = write_m3c(&sc, &img);
        let (f, img2) = read_m3c(&d).unwrap();
        assert_eq!(f.scene.width, 48);
        for (a, b) in img.recs.iter().zip(&img2.recs) {
            assert_eq!(a.ray_count, b.ray_count);
            assert_eq!(a.hit, b.hit);
            assert!((a.ysum - b.ysum).abs() < 1e-4);
            for k in 0..3 {
                assert!((a.col[k] - b.col[k]).abs() < 1e-4);
            }
        }
    }
}
