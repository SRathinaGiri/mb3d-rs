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

use super::engine::Inner;
use crate::scene::Scene;
use std::sync::atomic::Ordering;

pub const FEATURES: usize = 25;

/// The feature saying the main rendering ran on the CPU alone because the
/// card's shaders were still compiling (its time is then the CPU's, and the
/// later stages often run on the card).
pub const CPU_ALONE: usize = 24;

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
