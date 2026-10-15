//! The older "15 bit" screen space ambient occlusion of MB3D
//! (`AmbShadowCalcThreadN`: `BuildATlevels`, `TAmbShadowCalc`,
//! `TAmbShadowCalcT0`).
//!
//! The depth buffer is scaled to 0..128, single peaks are removed and a
//! pyramid of smoothed copies ("a trous" levels, step 1, 2, 4, ...) is
//! built.  For every object pixel, rings of growing radius are sampled on
//! the level whose step matches the ring; the steepest elevation angle per
//! 1/32 of the circle is kept and the arc tangents of those slopes are
//! summed to the ambient shadow.

use crate::gbuffer::SiLight;
use crate::ssao::SsaoParams;

/// `pavgw`
#[inline]
fn avg(a: u16, b: u16) -> u16 {
    ((a as u32 + b as u32 + 1) >> 1) as u16
}

/// `fastIntArcTan2`: angle in 1/32 circles (0..31).
pub fn fast_int_arctan2(y: i32, x: i32) -> usize {
    let r = if x == 0 {
        (y >= 0) as i32 * 16 + 8
    } else if y == 0 {
        (x >= 0) as i32 * 16
    } else if y < 0 {
        if x < 0 {
            if x >= y {
                7 - (x * 4) / y
            } else {
                (y * 4) / x
            }
        } else if -y < x {
            15 + (y * 4) / x
        } else {
            8 - (x * 4) / y
        }
    } else if x >= 0 {
        if x > y {
            16 + (y * 4) / x
        } else {
            23 - (x * 4) / y
        }
    } else if y < -x {
        31 + (y * 4) / x
    } else {
        24 - (x * 4) / y
    };
    (r & 31) as usize
}

pub(crate) struct Levels {
    pub(crate) count: usize,
    pub(crate) corr_mul: f32,
    pub(crate) zsub: i64,
    pub(crate) lev: Vec<Vec<u16>>,
}

impl Levels {
    /// The scaled depth of a pixel as the sampling compares it (32768 for
    /// the background).
    pub(crate) fn zp_of(&self, s: &SiLight) -> i32 {
        if s.zpos() < 32768 {
            (((s.zpos_fine >> 8) as i64 - self.zsub) as f64 * self.corr_mul as f64).round_ties_even() as i32
        } else {
            32768
        }
    }
}

/// The constants of the sampling: `(szrt, smul, st2)`.
pub(crate) fn sampling_constants(lv: &Levels, zc_mul: f64, zcorr: f64, p: &SsaoParams) -> (f32, f32, f32) {
    let n = lv.count as f32;
    let thr = p.threshold.max(0.01);
    let zscale = (((256.0 / zc_mul + 1.0).powi(2) - 1.0) / zcorr) as f32;
    let st2 = zscale / (lv.corr_mul * 256.0);
    let pi = std::f32::consts::PI;
    if p.t0 {
        ((thr * 2.0 / st2).max(0.01), 1.35 * 32767.0 / (pi * 32.0 * (thr * 0.8 * n.sqrt().sqrt()).atan().powf(0.9)), st2)
    } else {
        (thr / st2, 1.35 * 32767.0 / (pi * 32.0 * (thr * n.sqrt().sqrt()).atan().powf(0.8)), st2)
    }
}

/// `BuildATlevels`
pub(crate) fn build_levels(gbuf: &[SiLight], w: usize, h: usize, wrap: bool) -> Levels {
    let mut count = 1usize;
    let mut x = w / 16;
    loop {
        count += 1;
        x >>= 1;
        if count == 8 || x < 4 {
            break;
        }
    }
    let (mut zmin, mut zmax) = (32767u32, 0u32);
    for s in gbuf {
        let z = s.zpos();
        if z < 32768 {
            zmax = zmax.max(z);
            zmin = zmin.min(z);
        }
    }
    let (za, zp) = if zmax < zmin { (0, 32768) } else { (zmin, zmax + 1) };
    let s1 = 128.0f32 / (zp - za) as f32;
    let zsub = (za as i64) << 8;
    let mut l1: Vec<u16> = gbuf
        .iter()
        .map(|s| {
            if s.zpos() < 32768 {
                (((s.zpos_fine >> 8) as i64 - zsub) as f64 * s1 as f64).round_ties_even() as u16
            } else {
                0
            }
        })
        .collect();
    // remove single peaks (3x3, in place)
    if w >= 3 && h >= 3 {
        let offs: [isize; 8] = {
            let w = w as isize;
            [-w - 1, -w, -w + 1, -1, 1, w - 1, w, w + 1]
        };
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let c = l1[i];
                let mut m = 0u16;
                for o in offs {
                    let v = l1[(i as isize + o) as usize];
                    if v >= c {
                        m = 0;
                        break;
                    } else if v > m {
                        m = v;
                    }
                }
                if m > 0 {
                    l1[i] = m + 1;
                }
            }
        }
    }
    let mut lev = vec![l1];
    let mut tmp = vec![0u16; w * h];
    let mut step = 1usize;
    for _ in 2..=count {
        let src = lev.last().unwrap();
        // x direction
        for y in 0..h {
            let r = &src[y * w..(y + 1) * w];
            let d = &mut tmp[y * w..(y + 1) * w];
            let at = |i: isize| r[i.clamp(0, w as isize - 1) as usize];
            for i in 0..w {
                let ii = i as isize;
                let s = step as isize;
                d[i] = if wrap {
                    // a panorama's row is a circle
                    avg(r[i], avg(r[(ii - s).rem_euclid(w as isize) as usize], r[(ii + s).rem_euclid(w as isize) as usize]))
                } else if i < step {
                    avg(r[i], avg(r[0], at(ii + s)))
                } else if i + step < w {
                    avg(r[i], avg(at(ii - s), at(ii + s)))
                } else {
                    ((r[i] as u32 + 1 + ((at(ii - s) as u32 + r[w - 1] as u32) >> 1)) >> 1) as u16
                };
            }
        }
        // y direction
        let mut dst = vec![0u16; w * h];
        for x in 0..w {
            let at = |j: isize| tmp[j.clamp(0, h as isize - 1) as usize * w + x];
            let s = step as isize;
            for j in 0..h {
                let jj = j as isize;
                let c = tmp[j * w + x];
                dst[j * w + x] = if j < step {
                    avg(avg(at(jj + s), tmp[x]), c)
                } else if j + step < h {
                    avg(avg(at(jj - s), at(jj + s)), c)
                } else {
                    avg(avg(at(jj - s), tmp[(h - 1) * w + x]), c)
                };
            }
        }
        lev.push(dst);
        step *= 2;
    }
    Levels { count, corr_mul: s1, zsub, lev }
}

/// The sample positions of one axis (`ya`/`ya2`/`ye` logic): the clamped
/// border first, then the grid of `step` inside the image, the last one
/// snapped to the border.
fn axis_samples(pos: i32, size: i32, max_rad: i32, step: i32, out: &mut Vec<i32>) {
    out.clear();
    let mut a = -max_rad;
    let mut a2 = i32::MAX;
    if a + pos < 0 {
        a2 = a;
        while a2 + pos < 0 {
            a2 += step;
        }
        if a2 + pos >= size {
            a2 = size - pos - 1;
        }
        a = -pos;
        if a == a2 {
            a2 = i32::MAX;
        }
    }
    let e = if pos + max_rad >= size { size - pos - 1 } else { max_rad };
    let mut v = a;
    loop {
        if v > a2 {
            v = a2;
            a2 = i32::MAX;
        }
        out.push(v);
        let vt = v;
        v += step;
        if v > e && vt < e {
            v = e;
        }
        if v > e {
            break;
        }
    }
}

/// One sample of a ring for pixels whose rings lie inside the image: the
/// offset in the level, `1 / radius` and the range of the 1/32 circle
/// buckets it updates (everything that does not depend on the pixel).
struct RingSample {
    off: isize,
    r1d: f32,
    lo: u8,
    hi: u8,
}

/// The rings of one level.
struct Ring {
    /// outer radius: the samples need `max_rad` pixels on every side
    max_rad: i32,
    min_rad: i32,
    step: i32,
    rm: f32,
    multi: bool,
    zrt: f32,
    samples: Vec<RingSample>,
}

/// The rings of all levels (the sample pattern of `TAmbShadowCalc`).
fn rings(lv: &Levels, w: usize, szrt: f32) -> Vec<Ring> {
    let n = lv.count as f32;
    let (mut max_rad, mut min_rad) = (0i32, 0i32);
    let mut out = Vec::new();
    let mut axis = Vec::new();
    for atl in 1..=lv.count {
        let step = 1i32 << (atl - 1);
        let rm = (step as f32).sqrt() * 5.0;
        max_rad += 4 * step;
        let multi = (rm / (min_rad + 1) as f32).round_ties_even() > 0.0;
        let zrt = szrt * (n / atl as f32).sqrt().sqrt();
        // the offsets of a pixel away from the borders
        axis_samples(max_rad, 2 * max_rad + 1, max_rad, step, &mut axis);
        let (maxs, mins) = (max_rad * max_rad, min_rad * min_rad);
        let mut samples = Vec::new();
        for &y2 in &axis {
            for &x2 in &axis {
                let rads = y2 * y2 + x2 * x2;
                if rads <= mins || rads > maxs {
                    continue;
                }
                let r1d = 1.0 / (rads as f32).sqrt();
                let c = fast_int_arctan2(y2, x2);
                let (lo, hi) = if multi {
                    let ic = (rm * r1d).round_ties_even() as usize;
                    let start = (c as i32 - (ic as i32 >> 1)) as usize & 31;
                    (start, (start + ic).min(39))
                } else {
                    (c, c)
                };
                samples.push(RingSample { off: y2 as isize * w as isize + x2 as isize, r1d, lo: lo as u8, hi: hi as u8 });
            }
        }
        out.push(Ring { max_rad, min_rad, step, rm, multi, zrt, samples });
        min_rad = max_rad;
    }
    out
}

/// Calculates `amb_shadow` for all object pixels.
pub fn ssao15(gbuf: &mut [SiLight], w: usize, h: usize, zc_mul: f64, zcorr: f64, p: &SsaoParams, threads: usize) {
    if w < 4 || h < 4 {
        return;
    }
    let lv = build_levels(gbuf, w, h, p.wrap_x);
    let (szrt, smul, st2) = sampling_constants(&lv, zc_mul, zcorr, p);
    let rings = rings(&lv, w, szrt);
    let threads = threads.clamp(1, h);
    let (lv, rings) = (&lv, &rings);
    let zps: Vec<i32> = gbuf.iter().map(|s| lv.zp_of(s)).collect();
    let zps = &zps;
    std::thread::scope(|sc| {
        let mut per: Vec<Vec<(usize, &mut [SiLight])>> = (0..threads).map(|_| Vec::new()).collect();
        for (i, r) in gbuf.chunks_mut(w).enumerate() {
            per[i % threads].push((i, r));
        }
        for list in per {
            sc.spawn(move || {
                crate::prio::calc_thread();
                let mut ys = Vec::new();
                let mut xs = Vec::new();
                for (y, row) in list {
                    for (x, si) in row.iter_mut().enumerate() {
                        if si.zpos() >= 32768 {
                            continue;
                        }
                        let zp = zps[y * w + x];
                        // the steepest slope per 1/32 circle: kept if larger
                        // (the order of the samples does not matter)
                        let update = |ang: &mut [f32; 40], mut st: f32, zrt: f32, lo: usize, hi: usize| {
                            if p.t0 {
                                if st >= zrt {
                                    return;
                                }
                                let s3 = st / zrt;
                                st *= 1.0 - s3 * s3 * s3;
                            } else if st > zrt {
                                st = zrt;
                            }
                            for a in &mut ang[lo..=hi] {
                                if *a < st {
                                    *a = st;
                                }
                            }
                        };
                        // 32 directions + overflow rows (MB3D merges 32..35)
                        let mut ang = [-1e10f32; 40];
                        let (xi, yi) = (x as i32, y as i32);
                        for (ri, ring) in rings.iter().enumerate() {
                            let l = &lv.lev[ri];
                            let r = ring.max_rad;
                            if xi >= r && xi + r < w as i32 && yi >= r && yi + r < h as i32 {
                                let base = (y * w + x) as isize;
                                for s in &ring.samples {
                                    let st = (l[(base + s.off) as usize] as i32 - zp) as f32 * s.r1d;
                                    update(&mut ang, st, ring.zrt, s.lo as usize, s.hi as usize);
                                }
                                continue;
                            }
                            // near the borders: the clipped sample grid
                            let (maxs, mins) = (r * r, ring.min_rad * ring.min_rad);
                            axis_samples(yi, h as i32, r, ring.step, &mut ys);
                            if p.wrap_x {
                                // the whole grid: x continues on the other side
                                axis_samples(r, 2 * r + 1, r, ring.step, &mut xs);
                            } else {
                                axis_samples(xi, w as i32, r, ring.step, &mut xs);
                            }
                            for &y2 in &ys {
                                let rowi = (yi + y2) as usize * w;
                                for &x2 in &xs {
                                    let rads = y2 * y2 + x2 * x2;
                                    if rads <= mins || rads > maxs {
                                        continue;
                                    }
                                    let r1d = 1.0 / (rads as f32).sqrt();
                                    let xx = (xi + x2).rem_euclid(w as i32) as usize;
                                    let st = (l[rowi + xx] as i32 - zp) as f32 * r1d;
                                    let c = fast_int_arctan2(y2, x2);
                                    if ring.multi {
                                        let ic = (ring.rm * r1d).round_ties_even() as usize;
                                        let start = (c as i32 - (ic as i32 >> 1)) as usize & 31;
                                        update(&mut ang, st, ring.zrt, start, (start + ic).min(39));
                                    } else {
                                        update(&mut ang, st, ring.zrt, c, c);
                                    }
                                }
                            }
                        }
                        for k in 0..4 {
                            if ang[k + 32] > ang[k] {
                                ang[k] = ang[k + 32];
                            }
                        }
                        let s: f32 = ang[..32].iter().filter(|&&a| a > -1e9).map(|&a| (a * st2).atan()).sum();
                        si.amb_shadow = (s * smul).round_ties_even().clamp(0.0, 16383.0) as u16;
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
    fn arctan_buckets() {
        assert_eq!(fast_int_arctan2(0, 1), 16);
        assert_eq!(fast_int_arctan2(1, 0), 24);
        assert_eq!(fast_int_arctan2(0, -1), 0);
        assert_eq!(fast_int_arctan2(-1, 0), 8);
        for (y, x) in [(3, 7), (-5, 2), (-1, -9), (6, -6)] {
            assert!(fast_int_arctan2(y, x) < 32);
        }
    }

    #[test]
    fn axis_grid() {
        let mut v = Vec::new();
        axis_samples(50, 100, 12, 2, &mut v);
        assert_eq!(v.first(), Some(&-12));
        assert_eq!(v.last(), Some(&12));
        assert_eq!(v.len(), 13);
        axis_samples(3, 100, 12, 4, &mut v);
        assert_eq!(&v[..3], &[-3, 0, 4]);
        axis_samples(97, 100, 12, 4, &mut v);
        assert_eq!(*v.last().unwrap(), 2);
    }

    #[test]
    fn hole_is_darker_than_plane() {
        let (w, h) = (64, 64);
        let mut g = vec![SiLight::default(); w * h];
        for y in 0..h {
            for x in 0..w {
                let d = ((x as f64 - 32.0).powi(2) + (y as f64 - 32.0).powi(2)).sqrt();
                let pit = if d < 10.0 { 3000.0 * (1.0 - d / 10.0) } else { 0.0 };
                g[y * w + x].zpos_fine = ((4_000_000.0 - pit) as u32) << 8;
            }
        }
        ssao15(&mut g, w, h, 300.0, 0.001, &SsaoParams { bits15: true, ..Default::default() }, 2);
        let center = g[32 * w + 32].amb_shadow;
        let far = g[5 * w + 5].amb_shadow;
        assert!(center > far, "center {center} far {far}");
    }
}
