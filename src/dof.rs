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
    apply_reporting(rgb, gbuf, width, height, zc_mul, zcorr, fov_y_deg, p, &|_, _| {}, cancel);
}

/// [`apply`] that counts the rows done over all its passes in `progress`,
/// weighed by their surface pixels (a pass on the card counts as a whole).
#[allow(clippy::too_many_arguments)]
pub fn apply_reporting<S: Sample>(
    rgb: &mut [S],
    gbuf: &[SiLight],
    width: usize,
    height: usize,
    zc_mul: f64,
    zcorr: f64,
    fov_y_deg: f64,
    p: &DofParams,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) {
    let zc = ZConv { zc_mul_inv: 1.0 / zc_mul, zcorr_inv: 1.0 / zcorr };
    let sw = &crate::render::RowWeights::surface(gbuf, width, height);
    let passes = p.passes.clamp(1, 4) as u32;
    let total = passes as usize * sw.total();
    for pass in 0..passes {
        if cancel() {
            return;
        }
        let base = pass as usize * sw.total();
        let report = |done: usize| progress(base + done, total);
        if p.forward {
            dof_forward(rgb, gbuf, width, height, &zc, fov_y_deg, p, pass, sw, &report, cancel);
        } else {
            dof_sorted(rgb, gbuf, width, height, &zc, fov_y_deg, p, pass, sw, &report, cancel);
        }
    }
    if !cancel() {
        progress(total, total);
    }
}

/// `doDOF`; `report`: the weights of the rows done so far (`sw`).
#[allow(clippy::too_many_arguments)]
fn dof_forward<S: Sample>(
    rgb: &mut [S],
    gbuf: &[SiLight],
    width: usize,
    height: usize,
    zc: &ZConv,
    fov: f64,
    p: &DofParams,
    pass: u32,
    sw: &crate::render::RowWeights,
    report: &(dyn Fn(usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) {
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
    #[cfg(feature = "gpu")]
    if card::possible() {
        let r_of: Vec<u16> = rbuf.iter().map(|rr| (rr.abs() + 0.5).round_ties_even() as u16).collect();
        let record = |i: usize| {
            let rr = rbuf[i];
            let s_ms = (rr.abs() + 1.0).powi(2);
            let c = [rgb[i * 3].to_255(), rgb[i * 3 + 1].to_255(), rgb[i * 3 + 2].to_255()];
            [c[0].to_bits(), c[1].to_bits(), c[2].to_bits(), s_ms.to_bits(), (0.5 / s_ms).to_bits(), rr.to_bits(), r_of[i] as u32, 0]
        };
        let rmax = (max_r + 0.5).round_ties_even() as i64;
        report(0);
        if let Some(out) = card::run::<S>(width, height, &r_of, None, &record, rmax, 1, true, sm, cancel) {
            rgb.copy_from_slice(&out);
            report(sw.total());
            return;
        }
        if cancel() {
            return;
        }
    }
    #[cfg(not(feature = "gpu"))]
    let _ = cancel;
    let mut transp = vec![0f32; n];
    let mut col = vec![0f32; n * 3];
    let (wi, hi) = (width as i64, height as i64);
    for y in 0..hi {
        report(sw.before(y as usize));
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

/// One source pixel of a sorted pass: its disk and weights (shared by the
/// CPU and the card, so both blend the same numbers).
#[derive(Clone, Copy)]
struct SortedSrc {
    s_ms: f32,
    sw: f32,
    ims: i64,
    r: i64,
    descale: bool,
    bg: bool,
}

#[inline]
fn sorted_src(iz: i32, zc: &ZConv, z_sharp: f32, ap: f32, dsf: f32, r_sub: f32, max_r: f32) -> SortedSrc {
    let w = zc.steps((8388352 - iz as i64) as f64);
    let mut s_radius = (w - z_sharp) * ap / (1.0 + w * dsf);
    let descale = s_radius < 0.0;
    s_radius = (s_radius.abs() - r_sub).clamp(0.0, max_r);
    let s_ms = s_radius + 1.0;
    SortedSrc {
        s_ms,
        sw: 1.0 / (2.5 * s_radius * s_radius + 1.0),
        ims: (s_ms * s_ms) as i64,
        r: (s_radius.abs() + 0.5).round_ties_even() as i64,
        descale,
        bg: iz == -1,
    }
}

/// `doDOFsort`
#[allow(clippy::too_many_arguments)]
fn dof_sorted<S: Sample>(
    rgb: &mut [S],
    gbuf: &[SiLight],
    width: usize,
    height: usize,
    zc: &ZConv,
    fov: f64,
    p: &DofParams,
    pass: u32,
    sw: &crate::render::RowWeights,
    report: &(dyn Fn(usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) {
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
    let (wi, hi) = (width as i64, height as i64);
    let rmax = (max_r.abs() + 0.5).round_ties_even() as i64;
    #[cfg(feature = "gpu")]
    if card::possible() {
        let src_of = |i: usize| sorted_src(depth(&gbuf[i]), zc, z_sharp, ap, dsf, r_sub, max_r);
        let r_of: Vec<u16> = (0..n).map(|i| src_of(i).r as u16).collect();
        let record = |i: usize| {
            let t = src_of(i);
            let c = [rgb[i * 3].to_255(), rgb[i * 3 + 1].to_255(), rgb[i * 3 + 2].to_255()];
            [c[0].to_bits(), c[1].to_bits(), c[2].to_bits(), t.s_ms.to_bits(), t.sw.to_bits(), t.ims as u32, t.r as u32, t.descale as u32 | (t.bg as u32) << 1]
        };
        let max_ims = ((max_r + 1.0) * (max_r + 1.0)) as usize + 1;
        report(0);
        if let Some(out) = card::run::<S>(width, height, &r_of, Some(&order), &record, rmax, max_ims, false, 0.0, cancel) {
            rgb.copy_from_slice(&out);
            report(sw.total());
            return;
        }
        if cancel() {
            return;
        }
    }
    // The pixels spread over at most `rmax` rows, so the image is painted in
    // bands of rows, each by one thread over the whole list: a pixel of a
    // band still gets its contributions in the same order (the result is
    // the same as one pass over the image, which took hours at 15000x20000).
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let band_rows = height.div_ceil(threads * 4).max(1);
    let n_bands = height.div_ceil(band_rows);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let finished = std::sync::atomic::AtomicUsize::new(0);
    let src: &[S] = rgb;
    let (order, next, finished) = (&order, &next, &finished);
    let bands: Vec<(usize, Vec<S>)> = std::thread::scope(|s| {
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
                            let (x, y) = ((i % width) as i64, (i / width) as i64);
                            let c = [src[i * 3].to_255(), src[i * 3 + 1].to_255(), src[i * 3 + 2].to_255()];
                            let SortedSrc { s_ms, sw, ims, r, descale: b_descale, bg: b_bg } = sorted_src(depth(&gbuf[i]), zc, z_sharp, ap, dsf, r_sub, max_r);
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
                        // the band's colours (not its sums: 16 bytes a pixel
                        // for every band until all are done)
                        let px: Vec<S> = (0..bn * 3)
                            .map(|k| {
                                let t = transp[k / 3];
                                S::from_dof(col[k] * if t < 1e-10 { 1e10 } else { 1.0 / t })
                            })
                            .collect();
                        done.push((by0 as usize, px));
                        let bw = sw.before(by1 as usize) - sw.before(by0 as usize);
                        report(finished.fetch_add(bw, std::sync::atomic::Ordering::Relaxed) + bw);
                    }
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    for (by0, px) in bands {
        let o = by0 * width * 3;
        rgb[o..o + px.len()].copy_from_slice(&px);
    }
}

/// Depth of field on the graphics card (`gpu_dof.wgsl`): the image in
/// strips of bins of 64x64 pixels, each bin with the list of the source
/// pixels that reach it, in the order the CPU spreads them.
#[cfg(feature = "gpu")]
mod card {
    use super::Sample;

    const BIN: usize = 64;
    /// list entries of a strip (4 bytes each)
    const ENTRY_BUDGET: u64 = 48 << 20;
    /// source records of a strip (32 bytes each)
    const RECORD_BUDGET: usize = 12 << 20;
    /// below this many pixel pairs the CPU is about as fast
    const MIN_WORK: f64 = 2e8;

    /// The card is on for this calculation (Auto or card only).
    pub fn possible() -> bool {
        match std::env::var("MB3D_DOF_CARD").as_deref() {
            Ok("0") => false,
            _ => crate::gpu::enabled(),
        }
    }

    fn threads() -> usize {
        std::thread::available_parallelism().map_or(4, |t| t.get())
    }

    /// Runs one pass on the card: `r_of` the disk radius of every pixel,
    /// `order` the order of the sources (None: row by row), `record` the
    /// card's 8 words of a source.  None when the CPU should do it (too
    /// little work in Auto, no card, a card error, or "Stop").
    #[allow(clippy::too_many_arguments)]
    pub fn run<S: Sample>(
        width: usize,
        height: usize,
        r_of: &[u16],
        order: Option<&[u32]>,
        record: &(dyn Fn(usize) -> [u32; 8] + Sync),
        rmax: i64,
        max_ims: usize,
        forward: bool,
        sm: f32,
        cancel: &(dyn Fn() -> bool + Sync),
    ) -> Option<Vec<S>> {
        let work: f64 = r_of.iter().map(|&r| ((2 * r as u64 + 1) * (2 * r as u64 + 1)) as f64).sum();
        let forced = std::env::var("MB3D_DOF_CARD").as_deref() == Ok("1");
        if !forced && crate::gpu::auto() && work < MIN_WORK {
            return None;
        }
        let t0 = std::time::Instant::now();
        let bins_x = width.div_ceil(BIN);
        let bin_rows = height.div_ceil(BIN);
        let rmax = rmax.max(0) as usize;
        // the list entries of every bin row, for the strips
        let per_row = count_entries(width, height, r_of, bin_rows);
        let roots: Vec<f32> = if forward { vec![0.0] } else { (0..=max_ims).map(|i| (i as f32).sqrt()).collect() };
        let mut out = vec![S::default(); width * height * 3];
        let (mut t_lists, mut t_card) = (0.0, 0.0);
        let mut br0 = 0;
        while br0 < bin_rows {
            // as many bin rows as the budgets allow (at least one)
            let mut br1 = br0 + 1;
            let mut entries = per_row[br0];
            while br1 < bin_rows {
                let rows = ((br1 + 1) * BIN).min(height) - br0 * BIN + 2 * rmax;
                if entries + per_row[br1] > ENTRY_BUDGET || rows * width > RECORD_BUDGET {
                    break;
                }
                entries += per_row[br1];
                br1 += 1;
            }
            let (y0, y1) = (br0 * BIN, (br1 * BIN).min(height));
            let (ys0, ys1) = (y0.saturating_sub(rmax), (y1 + rmax).min(height));
            let mut records = vec![0u32; (ys1 - ys0) * width * 8];
            let chunk_rows = (ys1 - ys0).div_ceil(threads()).max(1);
            std::thread::scope(|sc| {
                for (k, part) in records.chunks_mut(chunk_rows * width * 8).enumerate() {
                    sc.spawn(move || {
                        let first = (ys0 + k * chunk_rows) * width;
                        for (j, rec) in part.chunks_exact_mut(8).enumerate() {
                            rec.copy_from_slice(&record(first + j));
                        }
                    });
                }
            });
            let t = std::time::Instant::now();
            let (offsets, list) = lists(width, height, r_of, order, br0, br1, ys0, ys1);
            t_lists += t.elapsed().as_secs_f64();
            let t = std::time::Instant::now();
            let strip = crate::gpu::DofStrip {
                width,
                y0,
                rows: y1 - y0,
                src_y0: ys0,
                records: &records,
                offsets: &offsets,
                entries: &list,
                bins_x,
                roots: &roots,
                forward,
                sm,
            };
            let sums = match crate::gpu::dof_strip(&strip, cancel) {
                Ok(v) => v,
                Err(e) => {
                    if !cancel() {
                        crate::gpu::set_status(format!("{} (depth of field on the CPU: {e})", crate::gpu::last_status()));
                    }
                    return None;
                }
            };
            t_card += t.elapsed().as_secs_f64();
            for (k, px) in sums.chunks_exact(4).enumerate() {
                let t = px[3];
                let m = if t < 1e-10 { 1e10 } else { 1.0 / t };
                let o = (y0 * width + k) * 3;
                for c in 0..3 {
                    out[o + c] = S::from_dof(px[c] * m);
                }
            }
            br0 = br1;
        }
        if std::env::var_os("MB3D_GPU_TIMING").is_some() {
            eprintln!("  depth of field on the card: {:.2} s (lists {t_lists:.2} s, card {t_card:.2} s), {:.1e} pixel pairs", t0.elapsed().as_secs_f64(), work);
        }
        Some(out)
    }

    /// The bins of 64x64 pixels a source's disk can reach, as columns
    /// `bx0..=bx1` and rows `by0..=by1`.
    #[inline]
    fn reach(i: usize, width: usize, height: usize, r: usize) -> (usize, usize, usize, usize) {
        let (x, y) = (i % width, i / width);
        ((x.saturating_sub(r)) / BIN, (x + r).min(width - 1) / BIN, (y.saturating_sub(r)) / BIN, (y + r).min(height - 1) / BIN)
    }

    /// The list entries each bin row will hold.
    fn count_entries(width: usize, height: usize, r_of: &[u16], bin_rows: usize) -> Vec<u64> {
        let n = r_of.len();
        let part = n.div_ceil(threads()).max(1);
        let parts: Vec<Vec<u64>> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..n.div_ceil(part))
                .map(|k| {
                    sc.spawn(move || {
                        let mut c = vec![0u64; bin_rows];
                        for (i, &r) in r_of.iter().enumerate().take(((k + 1) * part).min(n)).skip(k * part) {
                            let (bx0, bx1, by0, by1) = reach(i, width, height, r as usize);
                            for row in &mut c[by0..=by1] {
                                *row += (bx1 - bx0 + 1) as u64;
                            }
                        }
                        c
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        (0..bin_rows).map(|r| parts.iter().map(|p| p[r]).sum()).collect()
    }

    /// The lists of the bins of bin rows `br0..br1`: offsets (one per bin,
    /// plus the end) and entries (source index from row `ys0`), each list in
    /// the sources' order.
    #[allow(clippy::too_many_arguments)]
    fn lists(width: usize, height: usize, r_of: &[u16], order: Option<&[u32]>, br0: usize, br1: usize, ys0: usize, ys1: usize) -> (Vec<u32>, Vec<u32>) {
        let bins_x = width.div_ceil(BIN);
        let next = std::sync::atomic::AtomicUsize::new(br0);
        // one bin row at a time per thread: counted, then filled
        let rows: Vec<(usize, Vec<u32>, Vec<u32>)> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads().min(br1 - br0))
                .map(|_| {
                    let next = &next;
                    sc.spawn(move || {
                        let mut done = Vec::new();
                        loop {
                            let br = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if br >= br1 {
                                break done;
                            }
                            let sources = |f: &mut dyn FnMut(usize, usize, usize)| {
                                let mut visit = |i: usize| {
                                    let (bx0, bx1, by0, by1) = reach(i, width, height, r_of[i] as usize);
                                    if by0 <= br && br <= by1 {
                                        f(i, bx0, bx1);
                                    }
                                };
                                match order {
                                    Some(o) => {
                                        let (lo, hi) = (ys0 * width, ys1 * width);
                                        for &i in o {
                                            let i = i as usize;
                                            if i >= lo && i < hi {
                                                visit(i);
                                            }
                                        }
                                    }
                                    None => {
                                        for i in ys0 * width..ys1 * width {
                                            visit(i);
                                        }
                                    }
                                }
                            };
                            let mut count = vec![0u32; bins_x + 1];
                            sources(&mut |_, bx0, bx1| {
                                for c in &mut count[bx0 + 1..=bx1 + 1] {
                                    *c += 1;
                                }
                            });
                            for k in 1..=bins_x {
                                count[k] += count[k - 1];
                            }
                            let mut list = vec![0u32; count[bins_x] as usize];
                            let mut at: Vec<u32> = count[..bins_x].to_vec();
                            sources(&mut |i, bx0, bx1| {
                                for a in &mut at[bx0..=bx1] {
                                    list[*a as usize] = (i - ys0 * width) as u32;
                                    *a += 1;
                                }
                            });
                            done.push((br, count, list));
                        }
                    })
                })
                .collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut rows = rows;
        rows.sort_by_key(|r| r.0);
        let mut offsets = Vec::with_capacity((br1 - br0) * bins_x + 1);
        let mut entries = Vec::with_capacity(rows.iter().map(|r| r.2.len()).sum());
        for (_, count, list) in &rows {
            let base = entries.len() as u32;
            offsets.extend(count[..bins_x].iter().map(|c| base + c));
            entries.extend_from_slice(list);
        }
        offsets.push(entries.len() as u32);
        (offsets, entries)
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
