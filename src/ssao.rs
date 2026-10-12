//! Screen space ambient occlusion on the z-buffer, ported from `AmbHiQ.pas`
//! (MB3D's "24 bit" ambient shadow, `TAmbHiQCalc` / `TAmbHiQCalcT0`).
//!
//! For every object pixel the maximum elevation angle of the depth buffer is
//! searched in 32 directions.  This is done on several levels: on each level
//! the depth buffer is smoothed further (an à-trous style wavelet blur,
//! `NextATlevelHiQ`) and the sampling radius doubles, so large and small
//! scale occlusion are both found at bounded cost.  The angles are summed on
//! the last level and stored as `AmbShadow` (0..16383) in the G-buffer.

use crate::gbuffer::SiLight;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SsaoParams {
    /// `sAmbShadowThreshold` (z/r)
    pub threshold: f32,
    /// `bSSAO24BorderMirrorSize * 0.01` (0..0.9)
    pub border_mirror: f32,
    /// the "threshold down to 0" variant (`TAmbHiQCalcT0`)
    pub t0: bool,
    /// random sampling with this many passes (`TAmbHiQCalcR`, `SSAORcount`),
    /// 0 = regular sampling
    pub random: u8,
    /// the older "15 bit" wavelet SSAO (`AmbShadowCalcThreadN`) instead of
    /// the 24 bit one
    pub bits15: bool,
}

impl Default for SsaoParams {
    fn default() -> Self {
        SsaoParams { threshold: 2.0, border_mirror: 0.0, t0: false, random: 0, bits15: false }
    }
}

/// Z values of the depth buffer as used by the AO (`FirstATlevelHiQ`).
fn first_level(gbuf: &[SiLight]) -> Vec<u32> {
    gbuf.iter().map(|s| if s.zpos() < 32768 { (s.zpos_fine & 0xFFFF_FF00) >> 1 } else { 0 }).collect()
}

/// `SmoothH` / `SmoothV`: one blur pass with the given step on a line.
fn smooth_line(dst: &mut [u32], src: &[u32], step: usize) {
    let n = src.len();
    if n == 0 {
        return;
    }
    let last = n - 1;
    for x in 0..n {
        let a = x.saturating_sub(step);
        let b = (x + step).min(last);
        dst[x] = ((dst[x] as u64 + ((src[a] as u64 + src[b] as u64) >> 1)) >> 1) as u32;
    }
}

/// `NextATlevelHiQ`
fn next_level(pia: &mut [u32], w: usize, h: usize, step: usize) {
    let mut sa = vec![0u32; w.max(h)];
    for y in 0..h {
        sa[..w].copy_from_slice(&pia[y * w..(y + 1) * w]);
        smooth_line(&mut pia[y * w..(y + 1) * w], &sa[..w], step);
    }
    let mut col = vec![0u32; h];
    for x in 0..w {
        for y in 0..h {
            sa[y] = pia[y * w + x];
            col[y] = pia[y * w + x];
        }
        // SmoothV writes PIA[y] := (SA[y] + (SA[a] + SA[b]) / 2) / 2
        smooth_line(&mut col, &sa[..h], step);
        for y in 0..h {
            pia[y * w + x] = col[y];
        }
    }
}

/// Number of levels (`aATlevelCount`).
pub(crate) fn level_count(w: usize, h: usize) -> usize {
    let ymin = (((w * w + h * h) as f64).sqrt() * 0.5).round_ties_even() as i64;
    let mut y = 1;
    let mut x: i64 = 5;
    loop {
        y += 1;
        x <<= 1;
        if y == 15 || x > ymin {
            break;
        }
    }
    y
}

/// Calculates the ambient shadow for all object pixels of `gbuf`.
/// `zc_mul`, `zcorr` are the z-buffer constants (`CalcPPZvals`).
pub fn ssao24(gbuf: &mut [SiLight], w: usize, h: usize, zc_mul: f64, zcorr: f64, p: &SsaoParams, threads: usize) {
    if w == 0 || h == 0 {
        return;
    }
    let levels = level_count(w, h);
    let z_scale = ((256.0 / zc_mul + 1.0).powi(2) - 1.0) / zcorr;
    let thr = p.threshold.max(0.01);
    let wlo = (w as f32 * p.border_mirror).round_ties_even() as i64;
    let whi = w as i64 - 1 - wlo;
    let hlo = (h as f32 * p.border_mirror).round_ties_even() as i64;
    let hhi = h as i64 - 1 - hlo;
    let (mw2, mh2) = (2 * (w as i64 - 1), 2 * (h as i64 - 1));
    let dirs: Vec<(f32, f32)> = (0..32)
        .map(|i| {
            let (s, c) = (i as f64 * std::f64::consts::PI / 16.0).sin_cos();
            (s as f32, c as f32)
        })
        .collect();

    // process in row blocks to bound the memory of the angle arrays
    let max_rows = (64 * 1024 * 1024 / (w * 64)).clamp(1, h);
    let blocks = (h - 1) / max_rows + 1;
    let rows = h.div_ceil(blocks);
    let objects: Vec<bool> = gbuf.iter().map(|s| s.zpos() < 32768).collect();
    let zfine: Vec<u32> = gbuf.iter().map(|s| (s.zpos_fine & 0xFFFF_FF00) >> 1).collect();
    let mut result = vec![None::<u16>; w * h];

    let passes = if p.random > 0 { p.random as usize } else { 1 };
    let mut acc = vec![0u32; w * h];
    for pass in 0..passes {
    let mut y0 = 0;
    while y0 < h {
        let y1 = (y0 + rows).min(h);
        let mut pia = first_level(gbuf);
        let mut ang: Vec<[i16; 32]> = vec![[-32768i16; 32]; w * (y1 - y0)];
        for level in 1..=levels {
            let sum_up = level == levels;
            let mut sit = (z_scale / 22000.0 * 4096.0) as f32;
            let mut szrt = thr * if p.t0 { 1.0 } else { 0.7 } * 4096.0 / sit
                * ((levels as f32 / level as f32).sqrt().sqrt());
            if p.t0 {
                szrt *= szrt;
            }
            let at = (thr * 0.6 * (levels as f32).sqrt().sqrt()).atan();
            let rnd = p.random > 0;
            let at = if rnd {
                (thr * if p.t0 { 0.64 } else { 0.65 } * (levels as f32).sqrt().sqrt()).atan()
            } else {
                at
            };
            let smul = 1.5 * 32767.0
                / (std::f32::consts::PI * 32.0 * if p.t0 { at } else { at.powf(0.9) })
                / passes as f32;
            let imin = 16383 / passes as i32;
            let iand = (1i64 << (level - 1)) - 1;
            let ssub = iand as f32 * 0.5;
            let istep = 1i64 << (level - 1);
            let smin_rad = if istep < 2 { 1.0f32 } else { 3.25 * istep as f32 };
            let step_count = if istep < 2 { 5 } else { 3 };
            let rma: Vec<f32> = (0..step_count).map(|k| 1.0 / (smin_rad + (k as i64 * istep) as f32 + 0.1)).collect();
            sit *= szrt;
            let pia_ref = &pia;
            let ncpu = threads.max(1).min(y1 - y0);
            let chunk = (y1 - y0).div_ceil(ncpu);
            std::thread::scope(|sc| {
                let mut rest: &mut [[i16; 32]] = &mut ang;
                let mut handles = Vec::new();
                let mut start = y0;
                while start < y1 {
                    let n = chunk.min(y1 - start);
                    let (part, r) = rest.split_at_mut(n * w);
                    rest = r;
                    let rma = &rma;
                    let dirs = &dirs;
                    let objects = &objects;
                    let zfine = &zfine;
                    let ys = start;
                    handles.push(sc.spawn(move || {
                        crate::prio::calc_thread();
                        let mut out: Vec<(usize, u16)> = Vec::new();
                        for yy in 0..n {
                            let y = ys + yy;
                            let mut seed: i32 = (0x24563487i64 + (y as i64 + 1) * 0x324594A1 + pass as i64 * 0x5851F42D) as i32;
                            for x in 0..w {
                                let i = y * w + x;
                                if !objects[i] {
                                    continue;
                                }
                                let zp = zfine[i] as i64;
                                let am = &mut part[yy * w + x];
                                for (d, &(dx, dy)) in dirs.iter().enumerate() {
                                    let mut sx = x as f32 + dx * smin_rad;
                                    let mut sy = y as f32 + dy * smin_rad;
                                    let (mut ox, mut oy) = (dx * smin_rad - ssub, dy * smin_rad - ssub);
                                    for &r in rma.iter() {
                                        let mut x2 = sx.round_ties_even() as i64;
                                        let mut y2 = sy.round_ties_even() as i64;
                                        let mut r = r;
                                        if rnd {
                                            // TAmbHiQCalcR: jittered sample inside the step cell
                                            seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                                            let jx = ox.round_ties_even() as i64 + ((seed as u32 >> 16) as i64 & iand);
                                            let jy = oy.round_ties_even() as i64 + ((seed as u32 >> 10) as i64 & iand);
                                            ox += dx * istep as f32;
                                            oy += dy * istep as f32;
                                            let dd = (jx * jx + jy * jy) as f32;
                                            if dd == 0.0 {
                                                sx += dx * istep as f32;
                                                sy += dy * istep as f32;
                                                continue;
                                            }
                                            r = 1.0 / dd.sqrt();
                                            x2 = x as i64 + jx;
                                            y2 = y as i64 + jy;
                                        }
                                        if x2 < 0 {
                                            x2 = -x2;
                                            if x2 >= wlo {
                                                break;
                                            }
                                        } else if x2 >= w as i64 {
                                            x2 = mw2 - x2;
                                            if x2 < whi {
                                                break;
                                            }
                                        }
                                        if y2 < 0 {
                                            y2 = -y2;
                                            if y2 >= hlo {
                                                break;
                                            }
                                        } else if y2 >= h as i64 {
                                            y2 = mh2 - y2;
                                            if y2 < hhi {
                                                break;
                                            }
                                        }
                                        let st = (pia_ref[(y2 * w as i64 + x2) as usize] as i64 - zp) as f32 * r;
                                        let it = if p.t0 {
                                            (st * sit / (st * st + szrt)).round_ties_even()
                                        } else {
                                            (st * sit / (szrt + st.abs())).round_ties_even()
                                        };
                                        let it = if it.is_nan() { -32768.0 } else { it };
                                        if (am[d] as f32) < it {
                                            am[d] = if it >= 32767.0 { 32767 } else { it as i16 };
                                        }
                                        sx += dx * istep as f32;
                                        sy += dy * istep as f32;
                                    }
                                }
                                if sum_up {
                                    let mut s = 0f32;
                                    for &v in am.iter() {
                                        if v > -32768 {
                                            s += (v as f32 * 0.000_244_140_6).atan();
                                        }
                                    }
                                    out.push((i, ((s * smul).round_ties_even() as i32).clamp(0, imin) as u16));
                                }
                            }
                        }
                        out
                    }));
                    start += n;
                }
                for hd in handles {
                    for (i, v) in hd.join().unwrap() {
                        acc[i] += v as u32;
                        result[i] = Some(0);
                    }
                }
            });
            if level < levels {
                // NextATlevelHiQ(..., 1 shl (newLevel - 1))
                next_level(&mut pia, w, h, 1 << level);
            }
        }
        y0 = y1;
    }
    }
    for ((s, r), a) in gbuf.iter_mut().zip(result).zip(acc) {
        if r.is_some() {
            s.amb_shadow = a.min(16383) as u16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(level_count(100, 100), 5);
        assert!(level_count(6000, 6000) <= 15);
    }

    #[test]
    fn flat_plane_has_no_occlusion_and_a_pit_has() {
        let (w, h) = (64, 64);
        let mut g = vec![SiLight::default(); w * h];
        for (i, s) in g.iter_mut().enumerate() {
            let (x, y) = ((i % w) as i64 - 32, (i / w) as i64 - 32);
            let pit = if x * x + y * y < 36 { 20000 } else { 0 };
            s.zpos_fine = ((4_000_000 - pit) as u32) << 8;
        }
        ssao24(&mut g, w, h, 300.0, 0.001, &SsaoParams::default(), 2);
        let center = g[32 * w + 32].amb_shadow;
        let far = g[5 * w + 5].amb_shadow;
        assert!(center > far + 1000, "center {center} far {far}");
    }
}
