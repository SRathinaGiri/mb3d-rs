//! The water of the ground plane as a ray tracer sees it (not in MB3D).
//!
//! The fractal is marched as in MB3D: every ray starts on the start plane,
//! in front of the eye its directions spread from, and the ground plane
//! takes part in the march as a second distance estimate.  For the water
//! that left gaps: with the camera low over it, the water between the eye
//! and the start plane was a cut surface without reflections (a dark band
//! across the bottom of the picture), rays that grazed the waves near the
//! horizon could stop short of them or pass them (a dark line between the
//! water and the sky), and the edge between the two showed as a seam.
//!
//! A ray tracer (Blender's Cycles) starts its rays at the eye and meets a
//! surface where it is.  This pass does that for the water, after the march
//! and before the shadows and the occlusion: each pixel's ray is followed
//! from the eye ([`Marcher::eye_t`]) to the analytic surface of the plane
//! with its waves ([`Marcher::water_hit`]), and where that lies in front of
//! what the march found (the fractal, or nothing), the pixel becomes water
//! with the surface's normal and depth.  The fractal's pixels stay as the
//! march made them, so renders without water are unchanged; everything the
//! water does with the fractal (its mirror image, what shows through clear
//! water, the shadows and light shafts in it, the glints) is computed from
//! the water pixels as before, by the reflection pass, which starts their
//! rays at the exact point too (`reflect::SrTracer::start_pixel`).

use crate::calc::{CalcParams, Marcher};
use crate::gbuffer::SiLight;
use crate::scene::CameraOptic;

/// The march's depth of a G-buffer entry (along its ray, in steps; the
/// inverse of `Marcher::calc_zpos_and_rough`).
pub fn depth_of(p: &CalcParams, si: &SiLight) -> f64 {
    let zf = (si.zpos_fine >> 8) as f64;
    (((8388351.5 - zf) / p.zc_mul + 1.0).powi(2) - 1.0) / p.zcorr
}

/// Whether the pass runs for these parameters: a ground plane, a picture
/// (not a 2D slice) and a camera whose rays come from one eye.
pub fn wanted(p: &CalcParams) -> bool {
    p.ground.is_some() && p.slice_2d == 0 && p.optic != CameraOptic::Panorama && std::env::var_os("MB3D_NO_WATER_LAYER").is_none()
}

/// The centre ray of the picture (for [`Marcher::eye_t`]).
pub fn centre_ray(m: &mut Marcher) -> ([f64; 3], [f64; 3]) {
    let p = m.p;
    m.pixel_ray(p.width / 2, p.height / 2)
}

/// The exact water hit of pixel x, y: (t along its ray, the surface point);
/// None where the ray does not meet the water.
pub fn hit(m: &mut Marcher, x: i32, y: i32, centre: &([f64; 3], [f64; 3])) -> Option<(f64, [f64; 3])> {
    hit_before(m, x, y, centre, f64::INFINITY)
}

/// [`hit`] in front of `t_limit` only (a surface the march found there).
pub fn hit_before(m: &mut Marcher, x: i32, y: i32, centre: &([f64; 3], [f64; 3]), t_limit: f64) -> Option<(f64, [f64; 3])> {
    let g = m.p.ground?;
    let (s, v) = m.pixel_ray(x, y);
    let mut t0 = Marcher::eye_t(&s, &v, centre);
    if !t0.is_finite() {
        // the centre's own ray: the eye of a ray a quarter width beside it
        let (s2, v2) = m.pixel_ray(m.p.width / 4, y);
        t0 = Marcher::eye_t(&s2, &v2, centre);
        if !t0.is_finite() {
            t0 = 0.0;
        }
    }
    let t = m.water_hit(x, y, &g, t0, t_limit)?;
    Some((t, m.it.c))
}

/// Whether the water point the march found for a pixel (its G-buffer
/// entry `si`, a ground hit behind the start plane) lies where the ray
/// meets the water: within a few of its stop distances above the surface
/// (far away the march's stop distance grows and it stops high above the
/// waves).  Such pixels stay as the march made them.
pub fn march_found(p: &CalcParams, si: &SiLight, x: i32, y: i32, m: &mut Marcher) -> bool {
    let Some(g) = p.ground.as_ref() else { return false };
    if !si.is_ground() || si.si_gradient >= 32768 || si.zpos() >= 32768 {
        return false;
    }
    let t = depth_of(p, si);
    let (s, v) = m.pixel_ray(x, y);
    let q = [s[0] + v[0] * t, s[1] + v[1] * t, s[2] + v[2] * t];
    let stop = p.de_stop as f64 * (1.0 + t * p.de_stop_factor as f64) * p.step_width;
    let h = -g.depth(&q);
    h > -stop && h < 4.0 * stop
}

/// The pass: the water pixels of the G-buffer `gbuf` (the rows of `p.rect`)
/// from the water's exact surface.  A pixel the march found as water keeps
/// its shadows and occlusion (when they were calculated with the march, on
/// the card); returns the number of pixels that became water from
/// something else (they have none yet).
pub fn apply(p: &CalcParams, gbuf: &mut [SiLight], threads: usize) -> usize {
    if !wanted(p) {
        return 0;
    }
    let [x0, y0, w, h] = p.rect;
    let (w, h) = (w.max(0) as usize, h.max(0) as usize);
    if w == 0 || gbuf.len() < w * h {
        return 0;
    }
    let rows_per = h.div_ceil(threads.max(1)).max(1);
    let t_start = std::time::Instant::now();
    let traced = std::sync::atomic::AtomicUsize::new(0);
    let changed = std::sync::atomic::AtomicUsize::new(0);
    let new = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|s| {
        for (ci, chunk) in gbuf[..w * h].chunks_mut(rows_per * w).enumerate() {
            let (new, traced, changed) = (&new, &traced, &changed);
            s.spawn(move || {
                let mut m = Marcher::new(p, 0);
                let centre = centre_ray(&mut m);
                let (mut n, mut tr, mut ch) = (0, 0, 0);
                for (i, si) in chunk.iter_mut().enumerate() {
                    let (x, y) = (x0 + (i % w) as i32, y0 + (ci * rows_per + i / w) as i32);
                    // what the march found in front of the water (the water
                    // need only be looked for up to there)
                    let front = si.zpos() < 32768 && !si.is_ground();
                    let limit = if front { depth_of(p, si) } else { f64::INFINITY };
                    tr += 1;
                    let Some((t, _)) = hit_before(&mut m, x, y, &centre, limit) else {
                        // the march stopped short of water the ray does not
                        // meet (far away its stop distance grows, it stopped
                        // above the waves near the horizon): the sky, so the
                        // water ends where the sky begins
                        if si.is_ground() {
                            *si = SiLight::default();
                        }
                        continue;
                    };
                    if front && limit <= t {
                        continue;
                    }
                    // the water the march found where the ray meets it stays
                    // as the march made it
                    if march_found(p, si, x, y, &mut m) {
                        continue;
                    }
                    ch += 1;
                    let mut w = m.water_si(t);
                    if si.is_ground() {
                        (w.shadow, w.amb_shadow) = (si.shadow, si.amb_shadow);
                    } else {
                        n += 1;
                    }
                    *si = w;
                }
                new.fetch_add(n, std::sync::atomic::Ordering::Relaxed);
                traced.fetch_add(tr, std::sync::atomic::Ordering::Relaxed);
                changed.fetch_add(ch, std::sync::atomic::Ordering::Relaxed);
            });
        }
    });
    if std::env::var_os("MB3D_STAGE_TIMES").is_some() {
        crate::elog!(
            "  water layer: {:.3} s, {} of {} pixels traced, {} made water ({} new)",
            t_start.elapsed().as_secs_f64(),
            traced.into_inner(),
            w * h,
            changed.into_inner(),
            new.load(std::sync::atomic::Ordering::Relaxed)
        );
    }
    new.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    /// A low camera over waves: every pixel under the horizon is water, the
    /// ones close to the camera too (before the start plane), and none of
    /// the sky above it.
    #[test]
    fn the_water_reaches_the_camera_and_the_horizon() {
        let mut sc = Scene::preset("Integer Power").unwrap();
        sc.width = 64;
        sc.height = 48;
        sc = sc.apply("rotation = 4, 0, 0\nmid = 0, -1.6, 0\nground = y, -1.4\nground_waves = 0.01, 0.12\nz_end = 3000").unwrap();
        let p = CalcParams::new(&sc).unwrap();
        let mut gbuf = vec![SiLight::default(); 64 * 48];
        for si in gbuf.iter_mut() {
            si.zpos_fine = 32768 << 16;
        }
        apply(&p, &mut gbuf, 2);
        let water: Vec<bool> = gbuf.iter().map(|s| s.is_ground()).collect();
        // the bottom row (the water right in front of the camera) is water
        assert!(water[47 * 64..].iter().all(|w| *w), "the bottom row is water");
        // the top row (the sky) is not
        assert!(water[..64].iter().all(|w| !*w), "the top row is sky");
        // each column: sky above, water below, one change (no gap or line)
        for x in 0..64 {
            let col: Vec<bool> = (0..48).map(|y| water[y * 64 + x]).collect();
            let changes = col.windows(2).filter(|w| w[0] != w[1]).count();
            assert_eq!(changes, 1, "column {x}: {col:?}");
        }
    }
}
