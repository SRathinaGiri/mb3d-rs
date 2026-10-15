//! DE ambient occlusion, ported from `CalcAmbShadowDE.pas`
//! (`TCalcAmbShadowDEThreadGeneral.Execute`).
//!
//! For every object pixel, 3 to 33 rays are cast over the hemisphere around
//! the surface normal.  Along each ray the distance estimate is sampled with
//! growing steps; the smallest ratio "free distance / ray length" is the
//! openness of that direction.  Neighbouring rays then share their
//! openness (the "correction"), and the mean gives `AmbShadow`.

use crate::calc::{CalcParams, Marcher};
use crate::gbuffer::SiLight;
use crate::math::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeaoParams {
    /// 0..3 = 3, 7, 17, 33 rays (`bCalcAmbShadowAutomatic shr 4 and 3`)
    pub quality: u8,
    /// 0 = off, 1 = 2x2, 2 = 3x3 dithering of the ray directions
    pub dither: u8,
    /// maximum ray length multiplier (`sDEAOmaxL`)
    pub max_len: f32,
    /// random first step (`bCalcAmbShadowAutomatic bit 7`)
    pub first_step_random: bool,
}

impl Default for DeaoParams {
    fn default() -> Self {
        DeaoParams { quality: 1, dither: 0, max_len: 1.0, first_step_random: false }
    }
}

type SVec = [f32; 3];

/// `-BuildRotMatrixS(0, ya, za)[2]`
fn ray_dir(ya: f64, za: f64) -> SVec {
    let (sy, cy) = ya.sin_cos();
    let (sz, cz) = za.sin_cos();
    [(sy * cz) as f32, (-sy * sz) as f32, (-cy) as f32]
}

/// `MakeRotQuatFromSNormals` + `CreateSMatrixFromQuat`
pub(crate) fn normal_matrix(n: SVec) -> [[f32; 3]; 3] {
    let a = (-n[2] as f64).clamp(-1.0, 1.0).acos() * 0.5;
    let (mut sa, ca) = a.sin_cos();
    let nn = ((n[1] * n[1] + n[0] * n[0]) as f64).sqrt();
    let q = if nn < 1e-25 {
        [0.0, 0.0, 0.0, ca]
    } else {
        sa /= nn;
        [-(n[1] as f64) * sa, n[0] as f64 * sa, 0.0, ca]
    };
    let m = [
        [1.0 - 2.0 * (q[1] * q[1] + q[2] * q[2]), 2.0 * (q[0] * q[1] + q[2] * q[3]), 2.0 * (q[0] * q[2] - q[1] * q[3])],
        [2.0 * (q[0] * q[1] - q[2] * q[3]), 1.0 - 2.0 * (q[0] * q[0] + q[2] * q[2]), 2.0 * (q[2] * q[1] + q[0] * q[3])],
        [2.0 * (q[0] * q[2] + q[1] * q[3]), 2.0 * (q[1] * q[2] - q[0] * q[3]), 1.0 - 2.0 * (q[0] * q[0] + q[1] * q[1])],
    ];
    m.map(|r| r.map(|v| v as f32))
}

#[inline]
fn sdot(a: SVec, b: SVec) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The ray directions (in the hemisphere around -z) for a quality level,
/// optionally shifted by the dither offsets.
struct Rays {
    dirs: Vec<SVec>,
    row_count: [usize; 4],
    d_step_mul: f32,
    d_min_a_dif: f32,
    correction_weight: f32,
}

fn build_rays(quality: u8) -> Rays {
    let mut dirs = Vec::new();
    let mut row_count = [0usize; 4];
    let q = quality.min(3) as usize;
    if q == 0 {
        let abr = 60f64.to_radians();
        for x in 0..3 {
            dirs.push(ray_dir(0.5 * abr, x as f64 * std::f64::consts::TAU / 3.0));
        }
        Rays { dirs, row_count, d_step_mul: 1.8, d_min_a_dif: -1.0, correction_weight: 0.3 }
    } else {
        dirs.push(ray_dir(0.0, 0.0));
        let abr = std::f64::consts::FRAC_PI_2 / (q as f64 + 0.9);
        for (y, rc) in row_count.iter_mut().enumerate().take(q + 1).skip(1) {
            let dt1 = y as f64 * abr;
            let n = (dt1.sin() * std::f64::consts::TAU / abr).round_ties_even() as usize;
            *rc = n;
            for x in 0..n {
                dirs.push(ray_dir(dt1, x as f64 * std::f64::consts::TAU / n as f64));
            }
        }
        Rays {
            dirs,
            row_count,
            d_step_mul: (1.0 + abr.sin()) as f32,
            d_min_a_dif: (abr * 1.2).cos() as f32,
            correction_weight: if q == 1 { 0.2 } else { 0.1666 },
        }
    }
}

/// Calculates `amb_shadow` for all object pixels.
pub fn deao(p: &CalcParams, gbuf: &mut [SiLight], width: usize, height: usize, dp: &DeaoParams, threads: usize) {
    // DEAO uses 5 binary search steps
    let mut pc = p.clone();
    pc.de_add_steps = 5;
    let p = &pc;
    let base = build_rays(dp.quality);
    let threads = threads.min(height).max(1);
    std::thread::scope(|s| {
        let rows: Vec<(usize, &mut [SiLight])> = gbuf.chunks_mut(width).enumerate().collect();
        let mut per: Vec<Vec<(usize, &mut [SiLight])>> = (0..threads).map(|_| Vec::new()).collect();
        for (i, r) in rows {
            per[i % threads].push((i, r));
        }
        for list in per {
            let base = &base;
            s.spawn(move || {
                for (y, row) in list {
                    let y = y + p.rect[1] as usize;
                    let seed = (0x24563487u32 as i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
                    let mut m = Marcher::new(p, seed);
                    let mut rnd = seed;
                    deao_row(&mut m, y as i32, row, dp, base, &mut rnd);
                }
            });
        }
    });
}

fn get_rand(seed: &mut i32) -> f64 {
    *seed = seed.wrapping_mul(0x343FD).wrapping_add(0x269EC3);
    ((*seed as u32 >> 8) & 0x7FFFFF) as f64 / 0x7FFFFF as f64
}

fn deao_row(m: &mut Marcher, y: i32, row: &mut [SiLight], dp: &DeaoParams, base: &Rays, rnd: &mut i32) {
    let p = m.p;
    let sw = p.step_width;
    let rot_v = normalise_matrix_to(sw, &p.vgrads);
    let s_max_d = dp.max_len as f64 * 0.5 * ((p.height as f64).powi(2) + (p.width as f64).powi(2)).sqrt();
    let quality = dp.quality.min(3) as usize;
    let de_stop = p.de_stop as f64;
    for (x, si) in row.iter_mut().enumerate() {
        let x = x as i32 + p.rect[0];
        si.amb_shadow = 0;
        if si.zpos() >= 32768 || si.si_gradient >= 32768 {
            continue;
        }
        if !m.surface_point(x, y, si, 5, 0.004) {
            continue;
        }
        let ms_de_stop = (m.ms_de_stop as f64).min(1e6);
        let ic = m.it.c;
        let step_ao = ms_de_stop / de_stop;
        let max_dist = s_max_d * step_ao.sqrt();
        let d_step_mul = base.d_step_mul as f64;
        let ms_de_stop = if p.de_stop_factor != 0.0 { ms_de_stop / (d_step_mul * d_step_mul) } else { de_stop / (d_step_mul * d_step_mul) };

        // ray directions (dithered per pixel)
        let mut dirs: Vec<SVec>;
        let mut first = 0usize;
        if dp.dither > 0 {
            let d = dp.dither as i32;
            let dt1 = (y % (d + 1)) as f64 * 0.5 / d as f64;
            let dt2 = (x % (d + 1)) as f64 * 0.5 / d as f64;
            if quality == 0 {
                dirs = (0..3)
                    .map(|i| ray_dir((dt1 + 0.5) * 50f64.to_radians(), (i as f64 + dt2) * std::f64::consts::TAU / 3.0))
                    .collect();
            } else {
                let abr = std::f64::consts::FRAC_PI_2 / (quality as f64 + 0.9);
                dirs = vec![ray_dir(0.0, 0.0)];
                if dt1 > 0.1 {
                    first = 1;
                }
                for r in 1..=quality {
                    let n = base.row_count[r];
                    for i in 0..n {
                        dirs.push(ray_dir(abr * (r as f64 + dt1 - 0.25), (i as f64 + dt2) * std::f64::consts::TAU / n as f64));
                    }
                }
            }
        } else {
            dirs = base.dirs.clone();
        }
        let dirs = &dirs[first..];
        let ray_count = dirs.len();
        let de_mul = (ray_count as f64 * 0.5).sqrt();
        let abr = 1.2 / (1.0 / de_mul).clamp(-1.0, 1.0).asin();
        let md_d10 = 0.1 / (max_dist * de_mul);

        let n = {
            let v = [si.normal[0] as f32, si.normal[1] as f32, si.normal[2] as f32];
            let l = sdot(v, v).sqrt().max(1e-30);
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let rot_w = normal_matrix(n);
        let mut min_ra = vec![0f32; ray_count];
        for (k, d) in dirs.iter().enumerate() {
            // RotateSVectorReverseS(RotW), then RotateSVectorS(RotV)
            let v = [sdot(rot_w[0], *d), sdot(rot_w[1], *d), sdot(rot_w[2], *d)];
            let sv = rotate_vector_reverse(&[v[0] as f64, v[1] as f64, v[2] as f64], &rot_v);
            let mut dt1 = step_ao * d_step_mul;
            let mut s_tmp = 1f64;
            let mut b_end = false;
            let mut b_first = dp.first_step_random;
            loop {
                if b_first {
                    b_first = false;
                    dt1 *= get_rand(rnd) * 1.5 + 0.5;
                } else if dt1 > max_dist {
                    dt1 = max_dist;
                    b_end = true;
                }
                m.it.c = ic;
                add_weight(&mut m.it.c, &sv, dt1);
                if m.at_cut_plane(dt1 * sw * 0.5) {
                    break;
                }
                let dt2 = m.calc_de();
                s_tmp = s_tmp.min((dt2 - ms_de_stop + dt1 * md_d10) / dt1);
                if s_tmp < 0.02 {
                    break;
                }
                dt1 += dt2.max(dt1 * d_step_mul);
                if b_end {
                    break;
                }
            }
            min_ra[k] = (s_tmp.max(0.0) * de_mul) as f32;
        }
        // correction: open neighbour rays lighten a closed one
        let mut amount = 0f64;
        for k in 0..ray_count {
            let mut s_add = 0f32;
            if min_ra[k] < 1.0 {
                let max_add = 1.0 - min_ra[k];
                for k2 in 0..ray_count {
                    if k2 == k {
                        continue;
                    }
                    let dt = sdot(dirs[k], dirs[k2]);
                    if dt > base.d_min_a_dif {
                        let overlap = min_ra[k2] - (dt.clamp(-1.0, 1.0).acos() as f64 * abr) as f32 + 1.0;
                        if overlap > 0.0 {
                            s_add += max_add.min(overlap) * base.correction_weight;
                        }
                    }
                }
            }
            amount += (s_add + min_ra[k]).min(1.0) as f64;
        }
        si.amb_shadow = (16383.0 * (1.0 - amount / ray_count as f64)).round_ties_even().max(0.0) as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_ray_follows_the_normal() {
        for n in [[0.0f32, 0.0, -1.0], [0.6, 0.0, -0.8], [0.0, -0.6, 0.8], [0.36, 0.48, -0.8]] {
            let m = normal_matrix(n);
            let d = ray_dir(0.0, 0.0);
            let v = [sdot(m[0], d), sdot(m[1], d), sdot(m[2], d)];
            for k in 0..3 {
                assert!((v[k] - n[k]).abs() < 1e-5, "{n:?} -> {v:?}");
            }
        }
    }

    #[test]
    fn ray_counts() {
        assert_eq!(build_rays(0).dirs.len(), 3);
        let r1 = build_rays(1).dirs.len();
        let r3 = build_rays(3).dirs.len();
        assert!(r1 >= 5 && r3 <= 33 && r3 > r1, "{r1} {r3}");
    }
}
