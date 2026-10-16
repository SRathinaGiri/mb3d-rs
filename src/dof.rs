//! Depth of field, ported from `DOF.pas` (`doDOF`, `doDOFsort`).
//!
//! A post process on the painted image: every pixel is spread over a disk
//! whose radius grows with the distance from the focus plane.  The default
//! "sorted" variant draws the pixels back to front with normalised
//! transparency, so near blurred objects cover the sharp ones behind them;
//! the "forward" variant spreads all pixels at once and is faster.  Both can
//! run in 1–4 passes with growing radius.

use crate::gbuffer::SiLight;
use crate::lighting::Sample;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DofParams {
    /// focus distances as fraction of the image width (`sDOFZsharp`,
    /// `sDOFZsharp2`); everything between the two is sharp
    pub z_sharp: f32,
    pub z_sharp2: f32,
    /// aperture (`sDOFaperture`, 0.0001..2)
    pub aperture: f32,
    /// maximum blur radius in pixels (`sDOFclipR`)
    pub clip_r: f32,
    /// number of passes 1..4 (`bCalcDOFtype shr 1 and 3` + 1)
    pub passes: u8,
    /// forward transformation (`bCalcDOFtype bit 3`) instead of sorted
    pub forward: bool,
}

impl Default for DofParams {
    fn default() -> Self {
        DofParams { z_sharp: 1.0, z_sharp2: 1.0, aperture: 0.1, clip_r: 30.0, passes: 1, forward: false }
    }
}

/// z values in step units (`(Sqr(z * ZcMul + 1) - 1) * Zcorr` with the
/// inverted constants of `CalcPPZvals`)
struct ZConv {
    zc_mul_inv: f64,
    zcorr_inv: f64,
}

impl ZConv {
    #[inline]
    fn steps(&self, z: f64) -> f32 {
        (((z * self.zc_mul_inv + 1.0).powi(2) - 1.0) * self.zcorr_inv) as f32
    }
}

/// The common focus setup: aperture for this pass, sharp z, radius offset.
fn focus(p: &DofParams, pass: u32, sorted: bool, width: usize, height: usize, fov_y_deg: f64) -> (f32, f32, f32, f32) {
    let ap0 = p.aperture.clamp(0.0001, 2.0);
    let lp = (p.passes.clamp(1, 4) - 1) as u32;
    let ap = match lp {
        0 => ap0 * 0.5,
        1 => {
            if sorted {
                ap0 * (0.25 + pass as f32 * 0.58) * 0.5
            } else {
                ap0 * (0.3 + pass as f32 * 0.55) * 0.5
            }
        }
        _ => {
            if sorted {
                ap0 / 22.0 * 3f32.powi(pass as i32)
            } else {
                ap0 / 12.0 * (1 << pass) as f32
            }
        }
    };
    let de_stop_factor = (fov_y_deg.to_radians().max(0.0) / height as f64) as f32;
    let dw = p.z_sharp.min(p.z_sharp2) * width as f32;
    let st = p.z_sharp.max(p.z_sharp2) * width as f32;
    let w = (1.0 + dw * de_stop_factor) / (1.0 + st * de_stop_factor);
    let z_sharp = (w * st + dw) / (1.0 + w);
    let r_sub = ((dw - z_sharp) / (1.0 + dw * de_stop_factor) * ap).abs();
    (ap, de_stop_factor, z_sharp, r_sub)
}

/// Applies all passes of the depth of field to `rgb` (8 or 16 bit RGB).
/// `cancel` ("Stop") ends it after the running pass (the sorted one also
/// within a pass; the picture is then half done and dropped by the callers).
#[allow(clippy::too_many_arguments)]
pub fn apply<S: Sample>(rgb: &mut [S], gbuf: &[SiLight], width: usize, height: usize, zc_mul: f64, zcorr: f64, fov_y_deg: f64, p: &DofParams, cancel: &(dyn Fn() -> bool + Sync)) {
    let zc = ZConv { zc_mul_inv: 1.0 / zc_mul, zcorr_inv: 1.0 / zcorr };
    for pass in 0..p.passes.clamp(1, 4) as u32 {
        if cancel() {
            return;
        }
        if p.forward {
            dof_forward(rgb, gbuf, width, height, &zc, fov_y_deg, p, pass);
        } else {
            dof_sorted(rgb, gbuf, width, height, &zc, fov_y_deg, p, pass, cancel);
        }
    }
}

/// `doDOF`
#[allow(clippy::too_many_arguments)]
fn dof_forward<S: Sample>(rgb: &mut [S], gbuf: &[SiLight], width: usize, height: usize, zc: &ZConv, fov: f64, p: &DofParams, pass: u32) {
    let (ap, dsf, z_sharp, r_sub) = focus(p, pass, false, width, height, fov);
    let clip_r = p.clip_r.clamp(0.1, 1000.0);
    let n = width * height;
    let bg = zc.steps(8388353.0);
    let mut rbuf = vec![0f32; n];
    let mut max_r = 0f32;
    for (r, s) in rbuf.iter_mut().zip(gbuf) {
        let w = if s.zpos() > 32767 { bg } else { zc.steps((8388352 - (s.zpos_fine >> 8) as i64) as f64) };
        let mut v = (w - z_sharp) / (1.0 + w * dsf) * ap;
        v = (v.abs() - r_sub).max(0.0) * v.signum();
        if v.abs() > clip_r {
            v = clip_r * v.signum();
        }
        max_r = max_r.max(v.abs());
        *r = v;
    }
    let sm = 1.0 / (1.0 + max_r);
    let mut transp = vec![0f32; n];
    let mut col = vec![0f32; n * 3];
    let (wi, hi) = (width as i64, height as i64);
    for y in 0..hi {
        for x in 0..wi {
            let i = (y * wi + x) as usize;
            let rr = rbuf[i];
            let s_ms = (rr.abs() + 1.0).powi(2);
            let c = [rgb[i * 3].to_255(), rgb[i * 3 + 1].to_255(), rgb[i * 3 + 2].to_255()];
            let sw = 0.5 / s_ms;
            let r = (rr.abs() + 0.5).round_ties_even() as i64;
            let xa = (-r).max(-x);
            let xe = r.min(wi - 1 - x);
            let ya = (-r).max(-y);
            let ye = r.min(hi - 1 - y);
            for yy in ya..=ye {
                for xx in xa..=xe {
                    let mut w = s_ms - (xx * xx) as f32 - (yy * yy) as f32;
                    if w > 0.0 {
                        if w > 1.0 {
                            w = 1.0;
                        }
                        let j = ((y + yy) * wi + x + xx) as usize;
                        if rbuf[j] >= rr {
                            w *= sw;
                        } else {
                            w *= sw * rbuf[j].abs() * sm;
                        }
                        transp[j] += w;
                        col[j * 3] += c[0] * w;
                        col[j * 3 + 1] += c[1] * w;
                        col[j * 3 + 2] += c[2] * w;
                    }
                }
            }
        }
    }
    for i in 0..n {
        let m = if transp[i] < 1e-10 { 1e10 } else { 1.0 / transp[i] };
        for k in 0..3 {
            rgb[i * 3 + k] = S::from_dof(col[i * 3 + k] * m);
        }
    }
}

/// `doDOFsort`
#[allow(clippy::too_many_arguments)]
fn dof_sorted<S: Sample>(rgb: &mut [S], gbuf: &[SiLight], width: usize, height: usize, zc: &ZConv, fov: f64, p: &DofParams, pass: u32, cancel: &(dyn Fn() -> bool + Sync)) {
    let (ap, dsf, z_sharp, r_sub) = focus(p, pass, true, width, height, fov);
    let clip_r = p.clip_r.clamp(0.1, 1000.0);
    let n = width * height;
    // nearest and farthest object pixel
    let (mut zmin, mut zmax) = (32768i64, 0i64);
    for s in gbuf {
        let z = s.zpos() as i64;
        if z < 32768 {
            zmin = zmin.min(z);
            zmax = zmax.max(z);
        }
    }
    let w = zc.steps(((32768 - zmin) * 256) as f64);
    let r0 = ((w - z_sharp) * ap / (1.0 + w * dsf)).abs();
    let w = zc.steps(((32768 - zmax) * 256) as f64);
    let max_r = clip_r.min(r0.max(((w - z_sharp) / (1.0 + w * dsf) * ap).abs()) * 1.5);

    // back to front: ascending fine z, background (-1) first, equal depths
    // in pixel order (a counting sort over the 24 bit depths)
    let depth = |s: &SiLight| {
        let z = (s.zpos_fine >> 8) as i32;
        if z > 8388352 || s.zpos() > 32767 { -1 } else { z }
    };
    let mut start = vec![0usize; 8388355];
    for s in gbuf {
        start[(depth(s) + 2) as usize] += 1;
    }
    for k in 1..start.len() {
        start[k] += start[k - 1];
    }
    let mut order = vec![0u32; n];
    for (i, s) in gbuf.iter().enumerate() {
        let k = (depth(s) + 1) as usize;
        order[start[k]] = i as u32;
        start[k] += 1;
    }
    drop(start);
    // The pixels spread over at most `rmax` rows, so the image is painted in
    // bands of rows, each by one thread over the whole list: a pixel of a
    // band still gets its contributions in the same order (the result is
    // the same as one pass over the image, which took hours at 15000x20000).
    let (wi, hi) = (width as i64, height as i64);
    let rmax = (max_r.abs() + 0.5).round_ties_even() as i64;
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let band_rows = height.div_ceil(threads * 4).max(1);
    let n_bands = height.div_ceil(band_rows);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let src: &[S] = rgb;
    let (order, next) = (&order, &next);
    let bands: Vec<(usize, Vec<f32>, Vec<f32>)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads.min(n_bands))
            .map(|_| {
                s.spawn(move || {
                    let mut done = Vec::new();
                    loop {
                        let b = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if b >= n_bands || cancel() {
                            break done;
                        }
                        let (by0, by1) = ((b * band_rows) as i64, ((b + 1) * band_rows).min(height) as i64);
                        let bn = (by1 - by0) as usize * width;
                        let mut transp = vec![0f32; bn];
                        let mut col = vec![0f32; bn * 3];
                        // the pixels that can reach the band
                        let lo = ((by0 - rmax).max(0) * wi) as u32;
                        let hi_i = ((by1 + rmax).min(hi) * wi) as u32;
                        for &idx in order.iter() {
                            if idx < lo || idx >= hi_i {
                                continue;
                            }
                            let i = idx as usize;
                            let iz = depth(&gbuf[i]);
                            let (x, y) = ((i % width) as i64, (i / width) as i64);
                            let c = [src[i * 3].to_255(), src[i * 3 + 1].to_255(), src[i * 3 + 2].to_255()];
                            let w = zc.steps((8388352 - iz as i64) as f64);
                            let mut s_radius = (w - z_sharp) * ap / (1.0 + w * dsf);
                            let b_descale = s_radius < 0.0;
                            s_radius = (s_radius.abs() - r_sub).clamp(0.0, max_r);
                            let s_ms = s_radius + 1.0;
                            let ims = (s_ms * s_ms) as i64;
                            let sw = 1.0 / (2.5 * s_radius * s_radius + 1.0);
                            let b_bg = iz == -1;
                            let r = (s_radius.abs() + 0.5).round_ties_even() as i64;
                            let xa = (-r).max(-x);
                            let xe = r.min(wi - 1 - x);
                            let ya = (-r).max(-y).max(by0 - y);
                            let ye = r.min(hi - 1 - y).min(by1 - 1 - y);
                            for yy in ya..=ye {
                                let iys = yy * yy;
                                for xx in xa..=xe {
                                    let ii = xx * xx + iys;
                                    if ii > ims {
                                        continue;
                                    }
                                    let j = ((y + yy - by0) * wi + x + xx) as usize;
                                    let mut w = s_ms - (ii as f32).sqrt();
                                    let mut dw = 0.0;
                                    if b_descale {
                                        dw = 1.0 - (0.8 * w / s_ms).powi(2);
                                    }
                                    w = if w > 1.0 { sw } else { w * sw };
                                    let pc = &mut col[j * 3..j * 3 + 3];
                                    let pt = &mut transp[j];
                                    if b_descale && *pt > dw {
                                        let f = dw / *pt;
                                        for v in pc.iter_mut() {
                                            *v *= f;
                                        }
                                        *pt *= f;
                                    }
                                    if !b_bg && w + *pt > 1.01 {
                                        let st = (1.01 - w) / *pt;
                                        for k in 0..3 {
                                            pc[k] = pc[k] * st + c[k] * w;
                                        }
                                        *pt = 1.01;
                                    } else {
                                        for k in 0..3 {
                                            pc[k] += c[k] * w;
                                        }
                                        *pt += w;
                                    }
                                }
                            }
                        }
                        done.push((by0 as usize, col, transp));
                    }
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    for (by0, col, transp) in bands {
        let o = by0 * width;
        for (i, t) in transp.iter().enumerate() {
            let m = if *t < 1e-10 { 1e10 } else { 1.0 / t };
            for k in 0..3 {
                rgb[(o + i) * 3 + k] = S::from_dof(col[i * 3 + k] * m);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(w: usize, h: usize) -> (Vec<u8>, Vec<SiLight>) {
        // left half near, right half far; a checker pattern
        let mut rgb = vec![0u8; w * h * 3];
        let mut g = vec![SiLight::default(); w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let v = if (x / 2 + y / 2) % 2 == 0 { 255 } else { 0 };
                rgb[i * 3..i * 3 + 3].copy_from_slice(&[v, v, v]);
                let z: u32 = if x < w / 2 { 8_000_000 } else { 6_000_000 };
                g[i].zpos_fine = z << 8;
            }
        }
        (rgb, g)
    }

    fn contrast(rgb: &[u8], w: usize, x0: usize, x1: usize, h: usize) -> f64 {
        let mut s = 0.0;
        for y in 1..h - 1 {
            for x in x0 + 1..x1 - 1 {
                let a = rgb[(y * w + x) * 3] as f64;
                let b = rgb[(y * w + x + 1) * 3] as f64;
                s += (a - b).abs();
            }
        }
        s
    }

    #[test]
    fn focus_keeps_one_depth_sharp() {
        for forward in [false, true] {
            let (w, h) = (64, 32);
            let (mut rgb, g) = scene(w, h);
            let before_far = contrast(&rgb, w, w / 2, w, h);
            let zc = ZConv { zc_mul_inv: 1.0 / 300.0, zcorr_inv: 1.0 / 0.001 };
            let near_z = zc.steps((8388352 - 8_000_000) as f64);
            let p = DofParams {
                z_sharp: near_z / w as f32,
                z_sharp2: near_z / w as f32,
                aperture: 0.5,
                clip_r: 20.0,
                passes: 1,
                forward,
            };
            apply(&mut rgb, &g, w, h, 300.0, 0.001, 30.0, &p, &|| false);
            let near = contrast(&rgb, w, 4, w / 2 - 4, h);
            let far = contrast(&rgb, w, w / 2 + 4, w, h);
            assert!(far < before_far * 0.5, "forward {forward}: far side not blurred");
            assert!(near > far * 2.0, "forward {forward}: near {near} far {far}");
        }
    }
}
