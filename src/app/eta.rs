//! The time the stages after the main rendering will take, for the main
//! window's time left: what the scene asks of them (shadows, occlusion,
//! reflections, depth of field), what the main rendering found (how much
//! of the picture is surface, how far its rays marched) and the computer
//! (threads, the graphics card and how much it added to the CPU threads).
//! The model ([`later_ratios`]) was fitted on 1418 calculations of 366
//! scenes, 270 of them with the card's shaders compiling (`eta_model.rs`).
//!
//! `MB3D_ETA_LOG=FILE`: each finished calculation adds a line to FILE
//! (these values and the stages' times, to fit the model on).
//!
//! The estimator's pieces every window shares ([`RateEma`], [`blend`],
//! [`slowdown`], [`ItemsLeft`]): a stage's time is its measurement blended
//! with a prior (last run's time, or the model's), and the stages still to
//! come are the priors scaled by how much slower this run goes.

use super::engine::Inner;
use crate::scene::Scene;
use std::sync::atomic::Ordering;

pub const FEATURES: usize = 25;

/// The feature saying the main rendering ran on the CPU alone because the
/// card's shaders were still compiling (its time is then the CPU's, and the
/// later stages often run on the card).
pub const CPU_ALONE: usize = 24;

/// A pace as tqdm smooths it: the progress made and the time it took are
/// averaged apart, each sample weighted by `exp(-dt / TAU)` of the ones
/// before, so the rate follows a change (the card joining) in seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RateEma {
    df: f64,
    dt: f64,
}

impl RateEma {
    /// The time constant in seconds.
    pub const TAU: f64 = 8.0;

    /// A sample: `df` more progress in `dt` seconds.
    pub fn add(&mut self, df: f64, dt: f64) {
        let w = (-dt.max(0.0) / Self::TAU).exp();
        self.df = w * self.df + df.max(0.0);
        self.dt = w * self.dt + dt.max(0.0);
    }

    /// Progress per second (None before progress was made).
    pub fn rate(&self) -> Option<f64> {
        (self.df > 0.0 && self.dt > 0.0).then(|| self.df / self.dt)
    }
}

/// The progress within a stage at which its measurement and its prior
/// count the same (see [`blend`]).
pub const BLEND_HALF: f64 = 0.08;

/// A stage's whole time: its measured one and its prior (what it took
/// last time, or the model's) blended by the progress `g` (0..1) it made,
/// half and half at [`BLEND_HALF`]; the one of them that exists when the
/// other is missing (None without both).
pub fn blend(measured: Option<f64>, prior: Option<f64>, g: f64) -> Option<f64> {
    match (measured, prior) {
        (Some(m), Some(p)) => {
            let a = g.clamp(0.0, 1.0) / (g.clamp(0.0, 1.0) + BLEND_HALF);
            Some(a * m + (1.0 - a) * p)
        }
        (m, p) => m.or(p),
    }
}

/// How much slower a stage ran than its prior said (1 for a prior too
/// short to tell), held within 0.1..10: the factor for the priors of the
/// stages still to come.
pub fn slowdown(measured: f64, prior: f64) -> f64 {
    if prior < 0.5 || measured <= 0.0 {
        return 1.0;
    }
    (measured / prior).clamp(0.1, 10.0)
}

/// The model's stage ratios as this computer finds them: the log of a
/// stage's time over the model's expectation, smoothed over the stages
/// measured (hard shadows and ambient occlusion, reflections, depth of
/// field; on the card and on the CPU apart). Kept in the ini, so the model's
/// bias on this computer is gone after a few renders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Calib {
    /// the log ratios, and how many stages each was learnt from
    pub log: [[f64; 3]; 2],
    pub n: [[u32; 3]; 2],
}

impl Calib {
    /// From the ini's line (six numbers and their counts), none without one.
    pub fn parse(s: Option<&str>) -> Calib {
        let v: Vec<f64> = s.unwrap_or("").split_whitespace().filter_map(|x| x.parse().ok()).collect();
        if v.len() != 12 {
            return Calib::default();
        }
        let mut c = Calib::default();
        for i in 0..6 {
            c.log[i / 3][i % 3] = v[i];
            c.n[i / 3][i % 3] = v[6 + i] as u32;
        }
        c
    }

    pub fn text(&self) -> String {
        let logs = self.log.iter().flatten().map(|x| format!("{x:.3}"));
        let ns = self.n.iter().flatten().map(|n| n.to_string());
        logs.chain(ns).collect::<Vec<_>>().join(" ")
    }

    /// What the model's expectation of stage `k` is multiplied by.
    pub fn factor(&self, card: bool, k: usize) -> f64 {
        self.log[card as usize][k].exp()
    }

    /// A stage took `measured` where the model expected `expected`: the
    /// mean of the first few, then moving (a computer changes).
    pub fn learn(&mut self, card: bool, k: usize, measured: f64, expected: f64) {
        if measured < 0.5 || expected < 0.5 {
            return;
        }
        let l = (measured / expected).ln().clamp(0.05f64.ln(), 20f64.ln());
        let (c, n) = (&mut self.log[card as usize][k], &mut self.n[card as usize][k]);
        let w = (1.0 / (*n as f64 + 1.0)).max(0.3);
        *c = (1.0 - w) * *c + w * l;
        *n += 1;
    }
}

/// Items of about the same cost one after another (Monte Carlo passes,
/// animation frames, tiles): the time each takes from those finished, and
/// the time left with the running item's own estimate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ItemsLeft {
    per: Option<f64>,
    n: u32,
}

impl ItemsLeft {
    /// An item finished after `secs`: averaged with the last few.
    pub fn done(&mut self, secs: f64) {
        self.n += 1;
        let w = 1.0 / self.n.min(4) as f64;
        self.per = Some(self.per.map_or(secs, |p| p + (secs - p) * w));
    }

    /// The time an item takes (None before one finished).
    pub fn per_item(&self) -> Option<f64> {
        self.per
    }

    /// The time left: the running item's (its progress `g`, its time so far
    /// `secs` and its measured whole time `measured`, None without one,
    /// blended with the finished items' time) and `after` items more (None
    /// while nothing says how long an item takes).
    pub fn left(&self, g: f64, secs: f64, measured: Option<f64>, after: usize) -> Option<f64> {
        let total = blend(measured, self.per, g)?;
        Some((total - secs).max(0.0) + after as f64 * self.per.unwrap_or(total))
    }
}

/// The running item's pace from its progress reports (a Monte Carlo pass,
/// a tile, a frame): what [`ItemsLeft::left`] asks about it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pace {
    /// the first and the last sample (progress, seconds)
    first: Option<(f64, f64)>,
    last: Option<(f64, f64)>,
}

impl Pace {
    /// The item is at the progress `f` (0..1) after `secs`: a sample at
    /// most every quarter second.
    pub fn at(&mut self, f: f64, secs: f64) {
        if f <= 0.0 {
            return;
        }
        match self.last {
            Some((f0, t0)) if secs - t0 < 0.25 || f <= f0 => return,
            Some(_) => {}
            None => self.first = Some((f, secs)),
        }
        self.last = Some((f, secs));
    }

    /// The time left in this item and `after` more: its progress `f` after
    /// `secs`, its whole time at its pace so far (none before progress was
    /// made, or past its end) blended with the finished items' time.
    /// The pace is the average since the first sample: an item's progress
    /// comes in clumps (the card's rows, a band at a time) that the
    /// recent pace of [`RateEma`] would follow, a clump at a time.
    pub fn left(&self, items: &ItemsLeft, f: f64, secs: f64, after: usize) -> Option<f64> {
        // the progress made under measurement (not what was done before,
        // e.g. the rows of a continued pass)
        let from = self.first.map_or(0.0, |s| s.0);
        let g = ((f - from) / (1.0 - from).max(1e-6)).clamp(0.0, 1.0);
        let rate = match (self.first, self.last) {
            (Some((f0, t0)), Some((f1, t1))) if f1 > f0 && t1 > t0 => Some((f1 - f0) / (t1 - t0)),
            _ => None,
        };
        let measured = rate.filter(|_| f < 1.0).map(|r| secs + (1.0 - f) / r);
        items.left(g, secs, measured, after)
    }
}

/// The scene's part of the features (known when the calculation starts).
pub fn scene_features(sc: &Scene) -> [f32; FEATURES] {
    let mut f = [0f32; FEATURES];
    let px = (sc.width.max(1) as f64 * sc.height.max(1) as f64) / 1e6;
    f[0] = px.ln() as f32;
    f[1] = (crate::render::thread_count(sc) as f32).ln();
    #[cfg(feature = "gpu")]
    {
        f[2] = (crate::gpu::enabled() && crate::gpu::auto()) as u8 as f32;
    }
    if let Some(h) = &sc.shadows {
        f[7] = 1.0;
        f[8] = h.lights.count_ones() as f32;
        f[9] = h.soft as u8 as f32;
        f[10] = h.max_len_mul.max(0.01).ln();
    }
    if sc.ao.is_some() {
        match &sc.deao {
            Some(d) => {
                f[12] = 1.0;
                f[13] = d.quality as f32;
                f[14] = d.dither as f32;
                f[15] = d.max_len.max(0.01).ln();
            }
            None => {
                let a = sc.ao.as_ref().unwrap();
                f[11] = 1.0;
                f[14] = a.random as f32;
                f[15] = a.bits15 as u8 as f32;
            }
        }
    }
    if sc.mc.reflections {
        f[16] = 1.0;
        f[17] = sc.mc.reflection_depth as f32;
        f[18] = sc.mc.transparency as u8 as f32;
        f[19] = sc.mc.diffuse_reflects as f32 / 250.0;
    }
    if let Some(d) = &sc.dof {
        f[20] = 1.0;
        f[21] = (d.clip_r.max(0.0) + 1.0).ln();
        f[22] = d.passes as f32;
    }
    f[23] = sc.vol_light.is_some() as u8 as f32;
    f
}

/// The main rendering's part, from the rows calculated so far and its
/// time so far (`secs`): how much the card took, how much is surface, how
/// far the rays marched and the pixels per second.
pub fn march_features(f: &mut [f32; FEATURES], e: &Inner, secs: f64) {
    let rows = e.rows.load(Ordering::Relaxed).max(1) as f32;
    if e.rows_px.load(Ordering::Relaxed) == 0 {
        // no row yet: the probe that weighed the rows saw the picture (its
        // pace is the calculation's, it took the same path)
        if let Some(p) = crate::render::work_probe() {
            f[4] = p.surface as f32;
            f[5] = (p.mean_steps + 1.0).ln() as f32;
            f[6] = (p.pixels as f64 / p.seconds.max(0.001) / 1e6).max(1e-6).ln() as f32;
        }
        return;
    }
    let px = e.rows_px.load(Ordering::Relaxed).max(1) as f64;
    f[3] = (crate::render::CARD_ROWS.load(Ordering::Relaxed) as f32 / rows).min(1.0);
    f[4] = (e.rows_hits.load(Ordering::Relaxed) as f64 / px) as f32;
    f[5] = (e.rows_steps.load(Ordering::Relaxed) as f64 / px + 1.0).ln() as f32;
    f[6] = (px / secs.max(0.05) / 1e6).max(1e-6).ln() as f32;
}

/// The later stages' times relative to the main rendering (hard shadows
/// and ambient occlusion, reflections, depth of field) the model expects
/// for these features: the mean of its networks' log ratios.
pub fn later_ratios(f: &[f32; FEATURES]) -> [f64; 3] {
    use super::eta_model::*;
    // (standardized; held in the range the fitted calculations had, e.g.
    // the march's part before its first rows)
    let x: Vec<f32> = (0..FEATURES).map(|i| ((f[i] - MEAN[i]) / STD[i]).clamp(-4.0, 4.0)).collect();
    let mut sum = [0f64; 3];
    for n in 0..NETS {
        let mut h = B1[n];
        for (i, xi) in x.iter().enumerate() {
            for (j, hj) in h.iter_mut().enumerate() {
                *hj += xi * W1[n][i * HIDDEN + j];
            }
        }
        let mut o = B2[n];
        for (j, hj) in h.iter().enumerate() {
            let t = hj.tanh();
            for (k, ok) in o.iter_mut().enumerate() {
                *ok += t * W2[n][j * 3 + k];
            }
        }
        for k in 0..3 {
            sum[k] += o[k] as f64;
        }
    }
    // (fitted on the log of ratio + 0.01)
    sum.map(|s| ((s / NETS as f64).exp() - 0.01).max(0.0))
}

/// What a finished calculation did besides its stages' times, for
/// `MB3D_ETA_LOG`: its whole time and the main rendering's, the main
/// rendering's progress when the card's shaders had compiled, and the
/// compiles that ran meanwhile.
pub struct LogExtra {
    pub start: std::time::Instant,
    pub main_secs: f64,
    pub f_compiled: Option<f64>,
}

/// The time the shaders now compiling still take (seconds; None when none
/// compiles), predicted from each running job's shader size, precision,
/// constants and how many compiled at once when it began. A job past its
/// prediction is given another 30 % of its time so far.
#[cfg(feature = "gpu")]
pub fn compile_left(since: std::time::Instant) -> Option<f64> {
    use super::eta_model::COMPILE_W;
    let jobs = crate::gpu::compile_jobs(since);
    let now = std::time::Instant::now();
    let running = |at: std::time::Instant| jobs.iter().filter(|q| q.start <= at && q.secs.is_none_or(|s| q.start + std::time::Duration::from_secs_f64(s) > at)).count();
    jobs.iter()
        .filter(|j| j.secs.is_none())
        .map(|j| {
            let x = [(j.bytes.max(1) as f64).ln(), j.double as u8 as f64, j.constants as f64, (running(j.start).max(1) as f64).ln(), 1.0];
            let pred = (x.iter().zip(COMPILE_W).map(|(a, w)| a * w).sum::<f64>()).exp() - 0.01;
            let ran = now.saturating_duration_since(j.start).as_secs_f64();
            if pred > ran { pred - ran } else { 0.3 * ran }
        })
        .reduce(f64::max)
}

/// A finished calculation for `MB3D_ETA_LOG`: its features, the main
/// rendering's time (at its pace without compiling), the later stages'
/// times (None: not in it), whether shaders were compiled meanwhile, and
/// the scene's name.
pub fn log(f: &[f32; FEATURES], main: f64, later: [Option<f64>; 3], cold: bool, name: &str, x: &LogExtra) {
    let Some(path) = std::env::var_os("MB3D_ETA_LOG") else { return };
    let feats: Vec<String> = f.iter().map(|v| format!("{v:.5}")).collect();
    let t = |v: Option<f64>| v.map_or("null".to_string(), |v| format!("{v:.3}"));
    // the compiles: [began after the calculation's start (s), time, bytes,
    // lines, double precision, constants]
    #[cfg(feature = "gpu")]
    let jobs: Vec<String> = crate::gpu::compile_jobs(x.start)
        .iter()
        .map(|j| {
            let at = j.start.saturating_duration_since(x.start).as_secs_f64() - x.start.saturating_duration_since(j.start).as_secs_f64();
            format!("[{at:.3},{},{},{},{},{}]", t(j.secs), j.bytes, j.lines, j.double, j.constants)
        })
        .collect();
    #[cfg(not(feature = "gpu"))]
    let jobs: Vec<String> = Vec::new();
    let line = format!(
        "{{\"name\":{:?},\"cold\":{},\"main\":{main:.3},\"post\":{},\"refl\":{},\"dof\":{},\"f\":[{}],\"total\":{:.3},\"main_secs\":{:.3},\"f_compiled\":{},\"jobs\":[{}]}}\n",
        name,
        cold,
        t(later[0]),
        t(later[1]),
        t(later[2]),
        feats.join(","),
        x.start.elapsed().as_secs_f64(),
        x.main_secs,
        t(x.f_compiled),
        jobs.join(",")
    );
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_ema_follows_a_constant_speed() {
        // 2 % every quarter second: 8 % per second, whatever the length
        let mut r = RateEma::default();
        assert_eq!(r.rate(), None);
        for _ in 0..40 {
            r.add(0.02, 0.25);
            assert!((r.rate().unwrap() - 0.08).abs() < 1e-9);
        }
        // the pace halves: within a few time constants the rate follows
        for _ in 0..100 {
            r.add(0.01, 0.25);
        }
        assert!((r.rate().unwrap() - 0.04).abs() < 0.002, "{:?}", r.rate());
        // samples with the same speed but at different intervals agree
        let mut u = RateEma::default();
        u.add(0.1, 1.0);
        u.add(0.3, 3.0);
        assert!((u.rate().unwrap() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn blend_moves_from_the_prior_to_the_measurement() {
        assert_eq!(blend(None, None, 0.5), None);
        assert_eq!(blend(Some(10.0), None, 0.0), Some(10.0));
        assert_eq!(blend(None, Some(20.0), 0.9), Some(20.0));
        let at = |g| blend(Some(10.0), Some(20.0), g).unwrap();
        assert!((at(0.0) - 20.0).abs() < 1e-9);
        assert!((at(BLEND_HALF) - 15.0).abs() < 1e-9);
        assert!(at(0.02) > at(0.04) && at(0.04) > at(0.5));
        assert!((at(1.0) - 10.0).abs() < 1.0);
    }

    #[test]
    fn calibration_learns_the_models_bias() {
        let mut c = Calib::default();
        // the model expects 10 s, the stage takes 40 s: learnt at once, and
        // a stage of 20 s then averages in
        c.learn(true, 1, 40.0, 10.0);
        assert!((c.factor(true, 1) - 4.0).abs() < 1e-9);
        c.learn(true, 1, 20.0, 10.0);
        let f = c.factor(true, 1);
        assert!(f > 2.7 && f < 2.9, "{f}");
        assert_eq!(c.factor(false, 1), 1.0);
        // round trip through the ini, and a bad line
        let back = Calib::parse(Some(&c.text()));
        assert!((back.factor(true, 1) - f).abs() < 0.01);
        assert_eq!(back.n, c.n);
        assert_eq!(Calib::parse(Some("1 2")), Calib::default());
        // too short to tell: unchanged
        c.learn(false, 0, 0.1, 10.0);
        assert_eq!(c.factor(false, 0), 1.0);
    }

    #[test]
    fn slowdown_scales_the_next_stages() {
        // the shadows took half again as long as last time: so will the rest
        let k = slowdown(30.0, 20.0);
        assert!((k - 1.5).abs() < 1e-9);
        let priors = [40.0, 10.0];
        let rest: f64 = priors.iter().map(|p| p * k).sum();
        assert!((rest - 75.0).abs() < 1e-9);
        assert_eq!(slowdown(100.0, 1.0), 10.0);
        assert_eq!(slowdown(1.0, 100.0), 0.1);
        // (a prior too short to tell, no measurement: as expected)
        assert_eq!(slowdown(3.0, 0.1), 1.0);
        assert_eq!(slowdown(0.0, 10.0), 1.0);
    }

    #[test]
    fn items_left_from_the_finished_ones() {
        let mut it = ItemsLeft::default();
        assert_eq!(it.per_item(), None);
        // the first item, half done at its own pace: three more like it
        assert_eq!(it.left(0.5, 5.0, Some(10.0), 3), Some(5.0 + 30.0));
        assert_eq!(it.left(0.0, 0.0, None, 3), None);
        it.done(10.0);
        it.done(12.0);
        assert!((it.per_item().unwrap() - 11.0).abs() < 1e-9);
        // the running item just began: the finished ones' time for all
        let l = it.left(0.0, 0.0, Some(30.0), 2).unwrap();
        assert!((l - 33.0).abs() < 1e-9, "{l}");
        // nearly done and measured slower: its own time for itself
        let l = it.left(0.9, 18.0, Some(20.0), 1).unwrap();
        assert!((l - (20.0 * 0.9 / 0.98 + 11.0 * 0.08 / 0.98 - 18.0 + 11.0)).abs() < 1e-6, "{l}");
    }

    #[test]
    fn pace_measures_the_running_item() {
        let (mut p, mut it) = (Pace::default(), ItemsLeft::default());
        assert_eq!(p.left(&it, 0.0, 0.0, 2), None);
        // 10 % a second: after 3 seconds, 7 more and two items of 10
        for i in 0..=6 {
            p.at(i as f64 * 0.05, i as f64 * 0.5);
        }
        let l = p.left(&it, 0.3, 3.0, 2).unwrap();
        assert!((l - 27.0).abs() < 1e-6, "{l}");
        // (samples closer than a quarter second, or without progress, are skipped)
        let q = p;
        p.at(0.31, 3.1);
        p.at(0.2, 3.5);
        assert_eq!(p, q);
        // finished items of 20 s: at the start of the next one their time
        // counts, as it goes the measurement takes over
        it.done(20.0);
        it.done(20.0);
        let mut p = Pace::default();
        assert_eq!(p.left(&it, 0.0, 0.0, 1), Some(40.0));
        for i in 0..=10 {
            p.at(i as f64 * 0.05, i as f64 * 0.5);
        }
        let l = p.left(&it, 0.5, 5.0, 1).unwrap();
        assert!(l > 25.0 && l < 27.0, "{l}");
        // a continued pass: the rows done before do not count as measured
        let mut p = Pace::default();
        p.at(0.5, 0.0);
        p.at(0.55, 1.0);
        // (a tenth of it measured at 10 s a pass, the rest the 20 s of before)
        let l = p.left(&it, 0.55, 1.0, 0).unwrap();
        assert!((l - ((10.0 * 0.1 + 20.0 * 0.08) / 0.18 - 1.0)).abs() < 1e-6, "{l}");
    }

    #[test]
    fn model_as_fitted() {
        // "Snailmail" (CPU+card, occlusion): the fitting script's ratios
        let mut f = [0f32; FEATURES];
        f[..7].copy_from_slice(&[-1.61098, 2.48491, 1.0, 0.0, 1.0, 4.79103, -2.55766]);
        f[11] = 1.0;
        let r = later_ratios(&f);
        for (a, b) in r.iter().zip([0.061912, 1.104704, 0.004416]) {
            assert!((a - b).abs() < 1e-3 * b.max(0.01), "{r:?}");
        }
        // the same with the main rendering on the CPU alone
        f[CPU_ALONE] = 1.0;
        let r = later_ratios(&f);
        for (a, b) in r.iter().zip([0.067129, 1.178014, 0.006012]) {
            assert!((a - b).abs() < 1e-3 * b.max(0.01), "{r:?}");
        }
    }
}
