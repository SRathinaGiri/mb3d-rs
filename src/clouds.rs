//! Volumetric clouds (an idea from Mandelbulber, not in MB3D): a layer of
//! clouds between two heights along one of the fractal's axes, shaped by 3D
//! noise and lit by the scene's first global light, painted over the
//! picture as a pass after the lighting and the reflections (and before
//! the depth of field): every pixel's view ray steps through the layer up
//! to the surface it hit (or the end of the layer), so the fractal hides
//! the clouds behind it and the clouds in front veil the fractal.  The
//! clouds are not part of the ray march: they shade the surfaces under them
//! (the sunlight through them) and show in the reflections
//! (`reflect::SrTracer::cloud_segment`); Monte Carlo sees them along its view rays (`mc::Tracer::with_clouds`)
//! and its reflected, refracted and bounced rays (`cloud_segment`) and shades the
//! sun's light at every hit with them.

use crate::gbuffer::SiLight;
use crate::lighting::{LightVals, PaintCamera, Sample};

/// The cloud layer (`Scene::clouds`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clouds {
    /// the axis the layer lies across: 0 = x, 1 = y, 2 = z
    pub axis: u8,
    /// the layer's two heights on that axis (either order)
    pub from: f64,
    pub to: f64,
    /// how thick the clouds are (1: a full layer lets about 2 % through)
    pub density: f32,
    /// how much of the sky they cover (0..1)
    pub coverage: f32,
    /// the size of the clouds' features (fractal units)
    pub size: f64,
    pub color: [u8; 3],
    /// a shift of the noise (fractal units): animated between keyframes,
    /// the clouds drift
    pub offset: [f64; 3],
}

impl Default for Clouds {
    fn default() -> Self {
        Clouds { axis: 1, from: -1.0, to: -1.5, density: 1.0, coverage: 0.5, size: 0.5, color: [255, 255, 255], offset: [0.0; 3] }
    }
}

/// A lattice value in -1..1 (an integer hash).
#[inline]
fn lattice(x: i64, y: i64, z: i64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ (z as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h >> 40) as f32 / (1u64 << 23) as f32 - 1.0
}

/// Smooth 3D value noise, -1..1.
fn noise(p: [f64; 3]) -> f32 {
    let f = p.map(f64::floor);
    let i = f.map(|v| v as i64);
    let t = [0, 1, 2].map(|k| {
        let u = (p[k] - f[k]) as f32;
        u * u * (3.0 - 2.0 * u)
    });
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let mut x = [[0f32; 2]; 2];
    for (dz, row) in x.iter_mut().enumerate() {
        for (dy, v) in row.iter_mut().enumerate() {
            let a = lattice(i[0], i[1] + dy as i64, i[2] + dz as i64);
            let b = lattice(i[0] + 1, i[1] + dy as i64, i[2] + dz as i64);
            *v = lerp(a, b, t[0]);
        }
    }
    lerp(lerp(x[0][0], x[0][1], t[1]), lerp(x[1][0], x[1][1], t[1]), t[2])
}

impl Clouds {
    fn bounds(&self) -> (f64, f64) {
        (self.from.min(self.to), self.from.max(self.to))
    }

    /// The extinction at `p` (per fractal unit).
    fn density_at(&self, p: [f64; 3]) -> f32 {
        let (lo, hi) = self.bounds();
        let k = self.axis as usize % 3;
        let h = (p[k] - lo) / (hi - lo).max(1e-30);
        if !(0.0..=1.0).contains(&h) {
            return 0.0;
        }
        let s = 1.0 / self.size.max(1e-12);
        let q = [0, 1, 2].map(|j| (p[j] + self.offset[j]) * s);
        // four octaves, rounder at the top of the layer than at the bottom
        let mut n = 0f32;
        let mut amp = 0.5f32;
        let mut freq = 1.0;
        for o in 0..4 {
            n += amp * noise([q[0] * freq + o as f64 * 17.1, q[1] * freq, q[2] * freq - o as f64 * 9.3]);
            amp *= 0.5;
            freq *= 2.03;
        }
        let edge = (h.min(1.0 - h) * 5.0).min(1.0) as f32;
        let v = (n * 0.5 + 0.5) * edge - (1.0 - self.coverage.clamp(0.0, 1.0));
        if v <= 0.0 {
            return 0.0;
        }
        // full coverage of the whole layer: optical depth 4 per unit density
        v * self.density.max(0.0) * 4.0 / ((hi - lo).max(1e-30) as f32 * self.coverage.clamp(0.05, 1.0))
    }

    /// The part `t0..t1` of the ray `o + d t` inside the layer.
    fn span(&self, o: [f64; 3], d: [f64; 3], tmax: f64) -> Option<(f64, f64)> {
        let (lo, hi) = self.bounds();
        let k = self.axis as usize % 3;
        let (a, b) = if d[k].abs() < 1e-12 {
            if (lo..=hi).contains(&o[k]) { (0.0, tmax) } else { return None }
        } else {
            let (ta, tb) = ((lo - o[k]) / d[k], (hi - o[k]) / d[k]);
            (ta.min(tb).max(0.0), ta.max(tb).min(tmax))
        };
        (b > a).then_some((a, b))
    }
}

/// The light at the clouds: the first global light's direction (towards
/// it, in scene coordinates) and colour (0..1), and the ambient colours
/// from above and below.
pub(crate) struct CloudLight {
    sun: Option<([f64; 3], [f32; 3])>,
    /// the sun's place in `LightVals::lights` (Monte Carlo shades it)
    pub(crate) sun_index: Option<usize>,
    amb_top: [f32; 3],
    amb_bottom: [f32; 3],
}

impl Clouds {
    /// The light of the scene at the clouds.
    pub(crate) fn light(&self, l: &crate::lighting::Lighting, lv: &LightVals, cam: &PaintCamera) -> CloudLight {
        let sun_index = lv.lights.iter().position(|v| v.on && !v.positional);
        let sun = sun_index.map(|i| {
            let v = &lv.lights[i];
            let a = cam.to_abs(v.ln);
            let n = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-30);
            (a.map(|x| (x / n) as f64), l.lights[v.idx].color.map(|x| x as f32 / 255.0))
        });
        CloudLight { sun, sun_index, amb_top: l.amb_top.map(|x| x as f32 / 255.0), amb_bottom: l.amb_bottom.map(|x| x as f32 / 255.0) }
    }

    /// The part of the sunlight that reaches the point `p` through the
    /// clouds (1 without a sun or clouds in its way).
    pub(crate) fn sunlight(&self, p: [f64; 3], cl: &CloudLight) -> f32 {
        let Some((sd, _)) = &cl.sun else { return 1.0 };
        let Some((t0, t1)) = self.span(p, *sd, self.far()) else { return 1.0 };
        let n = ((t1 - t0) / (self.size * 0.5)).clamp(6.0, 32.0) as usize;
        let dt = (t1 - t0) / n as f64;
        let mut od = 0f32;
        for i in 0..n {
            let t = t0 + (i as f64 + 0.5) * dt;
            od += self.density_at([0, 1, 2].map(|k| p[k] + sd[k] * t)) * dt as f32;
            if od > 6.0 {
                break;
            }
        }
        (-od).exp()
    }

    /// How far a ray that hits nothing looks into the layer: through it at
    /// a slant.
    pub(crate) fn far(&self) -> f64 {
        let (lo, hi) = self.bounds();
        (hi - lo).max(1e-30) * 20.0 + self.size * 40.0
    }

    /// The ray `o + d t` (`d` of length 1) through the layer up to `tmax`:
    /// the part of the light behind that gets through and the light of the
    /// clouds (0..1); None when it misses the layer.  `jitter` (0..1)
    /// shifts the steps against bands.
    pub(crate) fn ray(&self, o: [f64; 3], d: [f64; 3], tmax: f64, cl: &CloudLight, jitter: f64) -> Option<(f32, [f32; 3])> {
        let (t0, t1) = self.span(o, d, tmax)?;
        let (lo, hi) = self.bounds();
        let thick = (hi - lo).max(1e-30);
        let col = self.color.map(|x| x as f32 / 255.0);
        // steps of about a third of the features' size, 8 to 96
        let n = ((t1 - t0) / (self.size * 0.3)).clamp(8.0, 96.0) as usize;
        let dt = (t1 - t0) / n as f64;
        let (mut tr, mut acc) = (1f32, [0f32; 3]);
        for i in 0..n {
            let t = t0 + (i as f64 + jitter) * dt;
            let p = [0, 1, 2].map(|k| o[k] + d[k] * t);
            let sigma = self.density_at(p);
            if sigma <= 0.0 {
                continue;
            }
            // the light reaching this point through the cloud
            let mut light = [0f32; 3];
            if let Some((sd, sc)) = &cl.sun {
                let ls = thick * 0.15;
                let mut od = 0f32;
                for k in 1..=5 {
                    let q = [0, 1, 2].map(|m| p[m] + sd[m] * ls * k as f64);
                    od += self.density_at(q) * ls as f32;
                }
                let tl = (-od).exp();
                // a little brighter towards the sun (forward scattering)
                let cosv = (sd[0] * d[0] + sd[1] * d[1] + sd[2] * d[2]) as f32;
                let phase = 0.8 + 0.6 * cosv.max(0.0).powi(4);
                for c in 0..3 {
                    light[c] = sc[c] * tl * phase;
                }
            }
            // ambient: the top colour from above, the bottom one from below
            let k = self.axis as usize % 3;
            let up = ((p[k] - lo) / thick).clamp(0.0, 1.0) as f32;
            let a = (1.0 - (-sigma * dt as f32).exp()) * tr;
            for c in 0..3 {
                let amb = cl.amb_bottom[c] + (cl.amb_top[c] - cl.amb_bottom[c]) * up;
                acc[c] += a * col[c] * (light[c] + 0.35 * amb);
            }
            tr *= (-sigma * dt as f32).exp();
            if tr < 0.01 {
                break;
            }
        }
        Some((tr, acc))
    }
}

/// The part of a painted surface's light that stays in a cloud's shadow
/// (its ambient light; the painting has no separate sunlight to dim).
pub(crate) const SHADE_AMBIENT: f32 = 0.35;

/// The jitter of a pixel's cloud steps (0..1).
pub(crate) fn jitter(x: i64, y: i64) -> f64 {
    (lattice(x, y, 7) * 0.5 + 0.5) as f64
}

/// Paints the clouds over `rgb` (the picture of the G-buffer `gbuf`,
/// `w` x `h` from column `x0`, row `y0` of the whole picture), from the
/// scene's middle `mid` (the camera positions are relative to it).
#[allow(clippy::too_many_arguments)]
pub fn apply<S: Sample>(
    cl: &Clouds,
    l: &crate::lighting::Lighting,
    lv: &LightVals,
    cam: &PaintCamera,
    mid: [f64; 3],
    gbuf: &[SiLight],
    rgb: &mut [S],
    w: usize,
    [x0, y0]: [i32; 2],
    threads: usize,
    cancel: &(dyn Fn() -> bool + Sync),
) {
    let light = cl.light(l, lv, cam);
    let far = cl.far();
    let rows = gbuf.len() / w.max(1);
    let threads = threads.clamp(1, rows.max(1));
    let per = rows.div_ceil(threads);
    std::thread::scope(|s| {
        for (ci, (gc, oc)) in gbuf.chunks(per * w).zip(rgb.chunks_mut(per * w * 3)).enumerate() {
            let light = &light;
            s.spawn(move || {
                for (ry, (grow, orow)) in gc.chunks(w).zip(oc.chunks_mut(w * 3)).enumerate() {
                    if cancel() {
                        return;
                    }
                    let y = (ci * per + ry) as i32 + y0;
                    for (xi, si) in grow.iter().enumerate() {
                        let x = xi as i32 + x0;
                        let x_pos = (x + 1) as f32 / cam.width as f32;
                        let y_pos = y as f32 / cam.height as f32;
                        let view = cam.to_abs(cam.view_vec(x_pos, y_pos, cam.aspect));
                        let cp = cam.cam_pos(x as f32, y as f32);
                        let o = [0, 1, 2].map(|k| mid[k] + cp[k] as f64);
                        let d = view.map(|v| v as f64);
                        let tmax = if si.is_background() {
                            far
                        } else {
                            (((8388352 - (si.zpos_fine >> 8) as i64) as f64 / cam.zc_mul + 1.0).powi(2) - 1.0) * cam.step_width / cam.zcorr
                        };
                        let px = &mut orow[xi * 3..xi * 3 + 3];
                        // the clouds' shadow on the surface: its sunlight
                        // dimmed (the ambient light, about a third, stays)
                        if !si.is_background() {
                            let sl = cl.sunlight([0, 1, 2].map(|k| o[k] + d[k] * tmax), light);
                            if sl < 1.0 {
                                let f = SHADE_AMBIENT + (1.0 - SHADE_AMBIENT) * sl;
                                for v in px.iter_mut() {
                                    *v = S::from_dof(v.to_255() * f);
                                }
                            }
                        }
                        let Some((tr, acc)) = cl.ray(o, d, tmax, light, jitter(x as i64, y as i64)) else { continue };
                        for c in 0..3 {
                            let v = px[c].to_255() * tr + acc[c] * 255.0;
                            px[c] = S::from_dof(v);
                        }
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_smooth_and_bounded() {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..2000 {
            let p = [i as f64 * 0.137, (i % 37) as f64 * 0.21, (i % 11) as f64 * 0.33];
            let v = noise(p);
            lo = lo.min(v);
            hi = hi.max(v);
            // nearby points have nearby values
            assert!((noise([p[0] + 1e-4, p[1], p[2]]) - v).abs() < 0.01);
        }
        assert!(lo >= -1.0 && hi <= 1.0 && hi - lo > 1.0);
    }

    #[test]
    fn the_layer_holds_the_clouds() {
        let c = Clouds { from: 1.0, to: 2.0, coverage: 1.0, ..Default::default() };
        assert_eq!(c.density_at([0.0, 0.5, 0.0]), 0.0);
        assert_eq!(c.density_at([0.0, 2.5, 0.0]), 0.0);
        assert!((0..50).any(|i| c.density_at([i as f64 * 0.1, 1.5, 0.0]) > 0.0));
        // a ray along y from below crosses the layer from 1 to 2
        let (a, b) = c.span([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 10.0).unwrap();
        assert!((a - 1.0).abs() < 1e-9 && (b - 2.0).abs() < 1e-9);
        // stopped by a surface before it: nothing
        assert!(c.span([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.5).is_none());
    }
}
