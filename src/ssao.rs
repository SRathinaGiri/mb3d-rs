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
    /// the picture wraps around horizontally (a 360 degree panorama): the
    /// left and the right edge are neighbours, so the samples and the
    /// smoothing continue on the other side instead of stopping at the
    /// border (which left the edge columns too bright: a seam)
    pub wrap_x: bool,
}

impl Default for SsaoParams {
    fn default() -> Self {
        SsaoParams { threshold: 2.0, border_mirror: 0.0, t0: false, random: 0, bits15: false, wrap_x: false }
    }
}

/// `x.round_ties_even()` without the library call (and its branch on the
/// sign) that it is on x86-64 without SSE4.1: for |x| < 2^23 adding and
/// subtracting 2^23 rounds to the nearest even integer; larger values are
/// integers already (and NaN stays NaN).
#[inline(always)]
fn rne(x: f32) -> f32 {
    const M: f32 = 8_388_608.0;
    let a = x.abs();
    if a < M { ((a + M) - M).copysign(x) } else { x }
}

/// Z values of the depth buffer as used by the AO (`FirstATlevelHiQ`).
fn first_level(gbuf: &[SiLight]) -> Vec<u32> {
    gbuf.iter().map(|s| if s.zpos() < 32768 { (s.zpos_fine & 0xFFFF_FF00) >> 1 } else { 0 }).collect()
}

/// `SmoothH` / `SmoothV`: one blur pass with the given step on a line
/// (`wrap`: the line is a circle, a panorama's row).
fn smooth_line(dst: &mut [u32], src: &[u32], step: usize, wrap: bool) {
    let n = src.len();
    if n == 0 {
        return;
    }
    let last = n - 1;
    for x in 0..n {
        let (a, b) = if wrap { ((x + n - step % n) % n, (x + step) % n) } else { (x.saturating_sub(step), (x + step).min(last)) };
        dst[x] = ((dst[x] as u64 + ((src[a] as u64 + src[b] as u64) >> 1)) >> 1) as u32;
    }
}

/// Runs `f(first_row, rows)` on row bands of `buf` (`w` values per row) in
/// parallel.
fn par_rows(buf: &mut [u32], w: usize, threads: usize, f: impl Fn(usize, &mut [u32]) + Sync) {
    let h = buf.len() / w.max(1);
    let n = threads.clamp(1, h.max(1));
    let band = h.div_ceil(n).max(1);
    std::thread::scope(|sc| {
        for (i, part) in buf.chunks_mut(band * w).enumerate() {
            let f = &f;
            sc.spawn(move || f(i * band, part));
        }
    });
}

/// `NextATlevelHiQ`: the horizontal pass, then the vertical one (both
/// `(p + (p[-step] + p[+step]) / 2) / 2`, clamped at the borders). The
/// vertical pass reads a copy of the horizontally smoothed image row by row
/// (MB3D walks the columns; the values are the same).
fn next_level(pia: &mut [u32], w: usize, h: usize, step: usize, threads: usize, wrap: bool) {
    par_rows(pia, w, threads, |_, rows| {
        let mut sa = vec![0u32; w];
        for row in rows.chunks_mut(w) {
            sa.copy_from_slice(row);
            smooth_line(row, &sa, step, wrap);
        }
    });
    let src = pia.to_vec();
    let src = &src;
    par_rows(pia, w, threads, |y0, rows| {
        for (k, row) in rows.chunks_mut(w).enumerate() {
            let y = y0 + k;
            let a = &src[y.saturating_sub(step) * w..][..w];
            let b = &src[(y + step).min(h - 1) * w..][..w];
            for x in 0..w {
                row[x] = ((row[x] as u64 + ((a[x] as u64 + b[x] as u64) >> 1)) >> 1) as u32;
            }
        }
    });
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

/// Memory for the smoothed depth levels kept over the row blocks and passes.
const PYRAMID_BYTES: usize = 1 << 30;

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
    // The smoothed depth levels do not depend on the pass or the row block:
    // kept when they fit in PYRAMID_BYTES, else smoothed again per block.
    let keep = (blocks > 1 || passes > 1) && levels * w * h * 4 <= PYRAMID_BYTES;
    let mut pyramid: Vec<Vec<u32>> = Vec::new();
    for pass in 0..passes {
    let mut y0 = 0;
    while y0 < h {
        let y1 = (y0 + rows).min(h);
        let mut pia = if pyramid.is_empty() { first_level(gbuf) } else { Vec::new() };
        let mut ang: Vec<[i16; 32]> = vec![[-32768i16; 32]; w * (y1 - y0)];
        for level in 1..=levels {
            if keep && pyramid.len() < level {
                pyramid.push(pia.clone());
            }
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
            // the random mode's cell corners (rounded `ox`, `oy` of every
            // direction and step) are the same for all pixels
            let cells: Vec<[(i64, i64); 5]> = dirs
                .iter()
                .map(|&(dx, dy)| {
                    let mut c = [(0i64, 0i64); 5];
                    let (mut ox, mut oy) = (dx * smin_rad - ssub, dy * smin_rad - ssub);
                    for ck in c.iter_mut().take(step_count) {
                        *ck = (rne(ox) as i64, rne(oy) as i64);
                        ox += dx * istep as f32;
                        oy += dy * istep as f32;
                    }
                    c
                })
                .collect();
            sit *= szrt;
            let pia_ref: &[u32] = if keep { &pyramid[level - 1] } else { &pia };
            let ncpu = threads.max(1).min(y1 - y0);
            // bands of a few rows handed out as the threads get free: the
            // rows with object pixels cost, the background rows do not
            const BAND: usize = 2;
            let jobs = std::sync::Mutex::new(ang.chunks_mut(BAND * w).enumerate());
            std::thread::scope(|sc| {
                let mut handles = Vec::new();
                for _ in 0..ncpu {
                    let rma = &rma;
                    let cells = &cells;
                    let dirs = &dirs;
                    let objects = &objects;
                    let zfine = &zfine;
                    let jobs = &jobs;
                    handles.push(sc.spawn(move || {
                        crate::prio::calc_thread();
                        let mut out: Vec<(usize, u16)> = Vec::new();
                        loop {
                        let Some((k, part)) = jobs.lock().unwrap().next() else { break };
                        let ys = y0 + k * BAND;
                        let n = part.len() / w;
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
                                    for (&r, &(cx, cy)) in rma.iter().zip(cells[d].iter()) {
                                        let (mut x2, mut y2);
                                        let mut r = r;
                                        if rnd {
                                            // TAmbHiQCalcR: jittered sample inside the step cell
                                            seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                                            let jx = cx + ((seed as u32 >> 16) as i64 & iand);
                                            let jy = cy + ((seed as u32 >> 10) as i64 & iand);
                                            let dd = (jx * jx + jy * jy) as f32;
                                            if dd == 0.0 {
                                                sx += dx * istep as f32;
                                                sy += dy * istep as f32;
                                                continue;
                                            }
                                            r = 1.0 / dd.sqrt();
                                            x2 = x as i64 + jx;
                                            y2 = y as i64 + jy;
                                        } else {
                                            x2 = rne(sx) as i64;
                                            y2 = rne(sy) as i64;
                                        }
                                        if p.wrap_x {
                                            x2 = x2.rem_euclid(w as i64);
                                        } else if x2 < 0 {
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
                                        let it = rne(if p.t0 { st * sit / (st * st + szrt) } else { st * sit / (szrt + st.abs()) });
                                        // MB3D: if am < it then am := min(it, 32767); as a max
                                        // without a branch (`as` saturates; a NaN counts as -32768)
                                        let it = if it.is_nan() { -32768.0 } else { it };
                                        am[d] = am[d].max(it as i16);
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
                        }
                        out
                    }));
                }
                for hd in handles {
                    for (i, v) in hd.join().unwrap() {
                        acc[i] += v as u32;
                        result[i] = Some(0);
                    }
                }
            });
            if level < levels && !(keep && pyramid.len() > level) {
                // NextATlevelHiQ(..., 1 shl (newLevel - 1))
                next_level(&mut pia, w, h, 1 << level, threads, p.wrap_x);
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
    fn rne_matches_round_ties_even() {
        let mut v: Vec<f32> = vec![0.0, -0.0, 0.5, -0.5, 1.5, 2.5, -2.5, 8_388_607.5, 4_194_304.5, -4_194_303.5, 1e10, -1e10, f32::INFINITY, f32::NEG_INFINITY];
        let mut seed = 12345u32;
        for _ in 0..200_000 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            v.push(f32::from_bits(seed) % 1e8);
        }
        for x in v {
            assert_eq!(rne(x).to_bits(), x.round_ties_even().to_bits(), "{x}");
        }
        assert!(rne(f32::NAN).is_nan());
    }

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

    /// A panorama (`wrap_x`) has no left or right border: the occlusion of
    /// a picture turned sideways by k columns is the turned occlusion of
    /// the picture (both SSAO kinds).
    #[test]
    fn wrapped_occlusion_turns_with_the_picture() {
        let (w, h) = (96, 48);
        let scene = |shift: usize| {
            let mut g = vec![SiLight::default(); w * h];
            for (i, s) in g.iter_mut().enumerate() {
                let (x, y) = ((i % w + shift) % w, i / w);
                // pits near the left border, so the samples cross the seam
                let pit = if ((x as i64 - 3).pow(2) + (y as i64 - 24).pow(2)) < 30 || (x as i64 - 60).pow(2) + (y as i64 - 10).pow(2) < 20 { 20000 } else { (x * 37 % 11) as i64 * 300 };
                s.zpos_fine = ((4_000_000 - pit) as u32) << 8;
            }
            g
        };
        for bits15 in [false, true] {
            let p = SsaoParams { wrap_x: true, bits15, ..Default::default() };
            let run = |shift: usize| {
                let mut g = scene(shift);
                if bits15 {
                    crate::ssao15::ssao15(&mut g, w, h, 300.0, 0.001, &p, 2);
                } else {
                    ssao24(&mut g, w, h, 300.0, 0.001, &p, 2);
                }
                g.iter().map(|s| s.amb_shadow).collect::<Vec<u16>>()
            };
            // an even turn: SSAO24 rounds its sample positions half to
            // even, so an odd one would change the rounding everywhere
            let (a, k) = (run(0), 38usize);
            let b = run(k);
            for y in 0..h {
                for x in 0..w {
                    // b at x shows a at x + k
                    assert_eq!(b[y * w + x], a[y * w + (x + k) % w], "bits15 {bits15}: x {x} y {y}");
                }
            }
        }
    }
}
