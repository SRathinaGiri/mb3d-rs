//! Multithreaded rendering: calculation of the G-buffer (`CalcMandT` +
//! `TMandCalcThread`) followed by painting (`PaintRows`).
//!
//! Like MB3D, rows are distributed interleaved over the threads
//! (thread t calculates rows t, t + n, t + 2n, ...).

use crate::calc::{CalcParams, HsLight, Marcher, PostJob, ShadowJob};
use crate::gbuffer::SiLight;
use crate::lighting::{LightVals, PaintCamera};
use crate::math::normalise_matrix_to;
use crate::scene::{CameraOptic, Scene};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

pub struct RenderResult {
    pub width: usize,
    pub height: usize,
    pub gbuffer: Vec<SiLight>,
    /// RGB, 8 bit per channel, row major
    pub rgb: Vec<u8>,
    pub calc_seconds: f64,
    pub paint_seconds: f64,
}

impl RenderResult {
    /// Fraction of pixels that hit the object.
    pub fn coverage(&self) -> f64 {
        let hits = self.gbuffer.iter().filter(|s| !s.is_background()).count();
        hits as f64 / self.gbuffer.len() as f64
    }
}

pub(crate) fn thread_count(sc: &Scene) -> usize {
    if sc.threads > 0 {
        sc.threads
    } else {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
    }
}

/// Calculates the G-buffer.  `progress` is called from the worker threads with
/// the number of finished rows.
pub fn calculate(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<(CalcParams, Vec<SiLight>), String> {
    calculate_cancellable(sc, progress, &|| false)
}

/// Like [`calculate`]; stops early with `Err("cancelled")` as soon as
/// `cancel` returns true (checked once per row and between passes).
pub fn calculate_cancellable(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(CalcParams, Vec<SiLight>), String> {
    calculate_inner(sc, progress, cancel, 2, None)
}

/// Like [`calculate_cancellable`] but without the post calculations (hard
/// shadows, ambient occlusion, normals on the z-buffer): run
/// [`post_process`] on (a copy of) the result.  The editor keeps the raw
/// G-buffer to redo only the post calculations or the painting after a
/// change, like MB3D's lighting and post processing windows.
pub fn calculate_raw_cancellable(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(CalcParams, Vec<SiLight>), String> {
    calculate_inner(sc, progress, cancel, 0, None)
}

/// A finished row of the calculation: its index in the calculated
/// rectangle and its pixels.
pub type RowSink<'a> = &'a (dyn Fn(usize, &[SiLight]) + Sync);

/// Like [`calculate_raw_cancellable`], handing every finished row to `rows`
/// (MB3D shows the image while it is calculated).
pub fn calculate_raw_rows(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    rows: RowSink,
) -> Result<(CalcParams, Vec<SiLight>), String> {
    calculate_inner(sc, progress, cancel, 0, Some(rows))
}

fn calculate_inner(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    post: u8,
    sink: Option<RowSink>,
) -> Result<(CalcParams, Vec<SiLight>), String> {
    let mut p = CalcParams::new(sc)?;
    let threads = thread_count(sc).min(p.rect[3] as usize).max(1);
    let two_d = p.slice_2d != 0;
    if let Some(vp) = sc.vol_light.as_ref().filter(|_| !two_d) {
        let l = effective_lighting(sc);
        let lv = light_vals(sc, &l, &paint_camera(sc, &p));
        if let Some((ln, positional)) = lv.light_ln(vp.light) {
            let hs_len = sc.shadows.map(|h| h.max_len_mul).unwrap_or(1.0);
            let t = Instant::now();
            let map = crate::vollight::build(&p, sc.stereo_mid(), ln, positional, l.lights[vp.light].amplitude, vp, hs_len, threads);
            if std::env::var_os("MB3D_GPU_TIMING").is_some() {
                eprintln!("  volumetric light map: {} side(s) of {}x{} in {:.2} s", map.sides.len(), map.cube_size, map.cube_size, t.elapsed().as_secs_f64());
            }
            p.vol = Some(std::sync::Arc::new(map));
        }
    }
    #[allow(unused_mut)] // changed by the GPU path only
    let mut p = p;
    let [x0, y0, w, h] = p.rect;
    let (x0, y0, w, h) = (x0 as usize, y0 as usize, w as usize, h as usize);
    #[cfg(feature = "gpu")]
    if two_d {
        crate::gpu::set_status("CPU: 2D calculation (not on the GPU yet)".into());
    } else if !crate::gpu::enabled() {
        crate::gpu::set_status("CPU: the GPU is switched off".into());
    } else {
        // the shadows and the DE ambient occlusion run on the card with
        // the main calculation (also for a raw calculation: post_process
        // skips what is done)
        let job = post_job(sc, &p);
        // single precision too coarse for this scene (a probe on the CPU
        // and the card disagrees): double precision (the probe waits for
        // its shader only when the calculation waits for the card anyway)
        p.gpu_double = double_probe(sc, &p, threads, !crate::gpu::auto());
        // the reflection pass's pipelines compile meanwhile
        if sc.mc.reflections {
            crate::gpu::reflect_prepare(&p, sc.shadows.is_some(), sc.ao.is_some());
        }
        // Auto: the CPU threads and the card share the march (the CPU
        // starts at once, the card joins when its shader is compiled);
        // then the card runs the shadows and the occlusion of the whole
        // image. When the card never joined (its shaders were still
        // compiling), the post calculations run as after a CPU
        // calculation.
        if crate::gpu::auto() {
            if let Ok(flags) = crate::gpu::march_prepare(&p, &job) {
                // the probe when its shader was not compiled yet: when the
                // card can join, before it does.  A scene too coarse in
                // single precision joins in double, when that is compiled:
                // the card is 3 to 5 times slower in double, but sharing the
                // rows it still helps (MengerTrees 16.4 -> 11.4 s, Shrooms
                // 6.8 -> 5.4 s); a slow card just takes fewer rows
                // (AboxShapes-Misty: 11 %, as fast as the CPU alone).
                let probe = || {
                    if p.gpu_double || !double_probe(sc, &p, threads, false) {
                        return None;
                    }
                    let mut pd = p.clone();
                    pd.gpu_double = true;
                    crate::gpu::march_prepare(&pd, &job).ok().map(|f| (pd, f))
                };
                let t_march = Instant::now();
                let (mut gbuf, n_card, switched, card_secs) = march_mixed(&p, threads, progress, cancel, sink, &flags[0], &probe)?;
                let march_secs = t_march.elapsed().as_secs_f64();
                let timing = std::env::var_os("MB3D_GPU_TIMING").is_some();
                let rate = |n: usize, t: f64| n as f64 / t.max(1e-6);
                // The shadows and the DE occlusion march the same formula:
                // they go to the card when it marched clearly faster than the
                // CPU threads (rows per second while it marched). Measured
                // over the example files, at a ratio below 1.5 the CPU did
                // them about as fast or faster (DEcomb1: 20.6 s on the card,
                // 12.2 s on the CPU), above it the card won (up to 15 times).
                // (not after a march in double precision: the card is slow
                // in it, and the reflections stay in single precision, as
                // they were before the card joined in double)
                let card_posts = n_card > 0 && switched.is_none() && rate(n_card, card_secs) >= POST_ON_CARD_RATIO * rate(h - n_card, march_secs);
                if timing {
                    eprintln!(
                        "  march (card and CPU): {march_secs:.2} s, {n_card} of {h} rows on the card in {card_secs:.2} s; rows/s card {:.1}, CPU {:.1}",
                        rate(n_card, card_secs),
                        rate(h - n_card, march_secs)
                    );
                }
                if card_posts && (job.shadows.is_some() || job.deao.is_some()) {
                    // the post passes' pipelines compiled with the march's
                    while !flags.iter().all(|f| f.load(Ordering::SeqCst)) && !cancel() {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    let mut s = crate::gpu::last_status();
                    let t_post = Instant::now();
                    let r = crate::gpu::post_on_card(&p, &mut gbuf, &job, cancel);
                    if timing {
                        eprintln!("  shadows and DE occlusion on the card: {:.2} s", t_post.elapsed().as_secs_f64());
                    }
                    match r {
                        Ok(()) => {
                            if job.shadows.is_some() {
                                s.push_str(" +shadows");
                            }
                            if job.deao.is_some() {
                                s.push_str(" +AO");
                            }
                            crate::gpu::set_status(s);
                            p.gpu_post = Some(job);
                        }
                        Err(e) => {
                            if cancel() {
                                return Err("cancelled".into());
                            }
                            crate::gpu::set_status(format!("{s}; post calculations on the CPU: GPU error: {e}"));
                        }
                    }
                }
                return finish_calculation(sc, p, gbuf, post, threads, cancel);
            }
        }
        let gpu = crate::gpu::march(&p, &job, progress, cancel, |y, row| {
            if let Some(f) = sink {
                f(y, row);
            }
        });
        if let Some(gbuf) = gpu {
            p.gpu_post = Some(job);
            return finish_calculation(sc, p, gbuf, post, threads, cancel);
        }
    }
    let p = p;
    let done = AtomicUsize::new(0);
    let mut rows: Vec<Vec<SiLight>> = vec![Vec::new(); h];
    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for t in 0..threads {
            let p = &p;
            let done = &done;
            handles.push(s.spawn(move || {
                crate::prio::calc_thread();
                // MB3D: seed := Round(Random * (iThreadId + 1) * $324594A1 + $24563487)
                let mut out = Vec::new();
                let mut y = t;
                while y < h {
                    if cancel() {
                        break;
                    }
                    let yy = y + y0;
                    let mut m = Marcher::new(p, 0);
                    let row: Vec<SiLight> = if two_d {
                        (x0..x0 + w).map(|x| m.slice_pixel(x as i32, yy as i32)).collect()
                    } else {
                        (x0..x0 + w)
                            .map(|x| {
                                m.seed = crate::calc::pixel_seed(yy as i64, x as i64);
                                m.march_pixel(x as i32, yy as i32)
                            })
                            .collect()
                    };
                    if let Some(f) = sink {
                        f(y, &row);
                    }
                    out.push((y, row));
                    let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                    progress(d, h);
                    y += threads;
                }
                out
            }));
        }
        for hnd in handles {
            for (y, row) in hnd.join().expect("render thread panicked") {
                rows[y] = row;
            }
        }
    });
    if cancel() {
        return Err("cancelled".into());
    }
    let gbuf: Vec<SiLight> = rows.into_iter().flatten().collect();
    finish_calculation(sc, p, gbuf, post, threads, cancel)
}

/// One row of the main calculation on the CPU (MB3D's per-row seed).
fn cpu_row(p: &CalcParams, y: usize) -> Vec<SiLight> {
    let [x0, y0, w, _] = p.rect;
    let yy = y + y0 as usize;
    // MB3D: seed := Round(Random * (iThreadId + 1) * $324594A1 + $24563487)
    let mut m = Marcher::new(p, 0);
    (x0..x0 + w)
        .map(|x| {
            m.seed = crate::calc::pixel_seed(yy as i64, x as i64);
            m.march_pixel(x, yy as i32)
        })
        .collect()
}

/// The mean colour difference of the probe's pixels (a share of full
/// scale) above which a scene runs in double precision on the card (see
/// [`double_probe`]).
#[cfg(feature = "gpu")]
const PROBE_LIMIT: f64 = 0.013;

/// The probe's pixels (about; the CPU marches fewer when they take longer
/// than [`PROBE_MS`]).
#[cfg(feature = "gpu")]
const PROBE_PIXELS: usize = 3000;

/// The time the probe's CPU marches may take (ms).
#[cfg(feature = "gpu")]
const PROBE_MS: u64 = 150;

/// The time the probe's march on the card may take (ms): its slowest
/// pixels decide (no result when it takes longer).
#[cfg(feature = "gpu")]
const PROBE_CARD_MS: u64 = 1500;

/// The probe's decisions by scene ([`crate::gpu::scene_key`]).
#[cfg(feature = "gpu")]
static PROBED: std::sync::Mutex<Vec<(u64, bool)>> = std::sync::Mutex::new(Vec::new());

/// Whether the card calculates the scene in double precision although its
/// zoom allows single: some scenes' fine detail (not deep zooms, mostly
/// the roughness of smoothed normals) comes out clearly different in
/// single precision.  A probe marches a sample of pixels on the CPU and
/// the card (single precision) and compares their colours; its decision
/// is kept for the scene (its shader, view and lighting).  The probe needs
/// the card's single precision march: `compile` waits for it, otherwise
/// (Auto's CPU start must not wait) there is no probe until it is
/// compiled.
/// MB3D_GPU_NOPROBE: no probe; MB3D_PROBE_LIMIT: the limit (a share);
/// MB3D_PROBE_MS: the time of its CPU marches;
/// MB3D_PROBE_DEBUG: prints the probe's result.
#[cfg(feature = "gpu")]
pub fn double_probe(sc: &Scene, p: &CalcParams, threads: usize, compile: bool) -> bool {
    if p.gpu_double || p.slice_2d != 0 || !crate::gpu::enabled() || std::env::var_os("MB3D_GPU_NOPROBE").is_some() || !crate::gpu::double_possible(p) || crate::gpu::unsupported(p).is_some() {
        return p.gpu_double;
    }
    let Some(key) = crate::gpu::scene_key(p) else { return false };
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (key, format!("{:?}", effective_lighting(sc))).hash(&mut h);
        h.finish()
    };
    if let Some(&(_, d)) = PROBED.lock().unwrap().iter().find(|(k, _)| *k == key) {
        return d;
    }
    let ready = if compile {
        match crate::gpu::march_prepare(p, &PostJob::default()) {
            Ok(flags) => {
                while !flags[0].load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                true
            }
            Err(_) => false,
        }
    } else {
        crate::gpu::march_compiled(p)
    };
    if !ready {
        return false;
    }
    let t = Instant::now();
    let f = probe_single(sc, p, threads);
    let limit = std::env::var("MB3D_PROBE_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(PROBE_LIMIT);
    let d = f.is_some_and(|(f, _)| f > limit);
    if std::env::var_os("MB3D_PROBE_DEBUG").is_some() {
        match f {
            Some((f, n)) => eprintln!("  probe: {n} pixels, colours differ by {:.2} % in single precision ({:.0} ms): {}", f * 100.0, t.elapsed().as_secs_f64() * 1000.0, if d { "double" } else { "single" }),
            None => eprintln!("  probe: no result ({:.0} ms)", t.elapsed().as_secs_f64() * 1000.0),
        }
    }
    let mut list = PROBED.lock().unwrap();
    if list.len() >= 256 {
        list.remove(0);
    }
    list.push((key, d));
    d
}

/// The probe of [`double_probe`]: the mean difference of the colours of a
/// sample of pixels marched by the card in single precision and by the
/// CPU (painted without shadows and occlusion; a share of full scale), and
/// the number of pixels compared.  The pixels are a grid of about
/// [`PROBE_PIXELS`] over the image: the card marches all of them, the CPU
/// as many as it can in [`PROBE_MS`] (in a spread order), every pixel with
/// a fresh [`Marcher`] and the seed the card's pixel has.  None when the
/// card cannot march the scene (or not within [`PROBE_CARD_MS`]) or the
/// CPU did too few pixels in its time.
#[cfg(feature = "gpu")]
pub fn probe_single(sc: &Scene, p: &CalcParams, threads: usize) -> Option<(f64, usize)> {
    use std::sync::OnceLock;
    let [x0, y0, w, h] = p.rect.map(|v| v.max(0) as usize);
    if w == 0 || h == 0 {
        return None;
    }
    // a grid of about PROBE_PIXELS (the card's sample, see
    // [`crate::gpu::march_sample`])
    let sxy = ((w * h) as f64 / PROBE_PIXELS as f64).sqrt().max(1.0) as usize;
    let (nx, ny) = (w / sxy, h / sxy);
    let samples: Vec<(usize, usize)> = (0..ny).flat_map(|r| (0..nx).map(move |c| (r * sxy + sxy / 2, c * sxy + (r * 5) % sxy))).collect();
    let ns = samples.len();
    if ns == 0 {
        return None;
    }
    // a spread order (a step coprime to the count)
    let step = [7919usize, 7907, 7901].into_iter().find(|s| ns % s != 0).unwrap_or(1) % ns;
    let step = step.max(1);
    let budget = std::time::Duration::from_millis(std::env::var("MB3D_PROBE_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(PROBE_MS));
    let cpu: Vec<OnceLock<SiLight>> = (0..ns).map(|_| OnceLock::new()).collect();
    let mut card: Vec<(usize, Vec<SiLight>)> = Vec::new();
    let next = AtomicUsize::new(0);
    let t = Instant::now();
    let card_ok = std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                crate::prio::calc_thread();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= ns || t.elapsed() > budget {
                        break;
                    }
                    let j = i * step % ns;
                    let (y, x) = samples[j];
                    let yy = y + y0;
                    // the card's seed: the row's, advanced by the column
                    let seed = crate::calc::pixel_seed(yy as i64, (x + x0) as i64);
                    let _ = cpu[j].set(Marcher::new(p, seed).march_pixel((x + x0) as i32, yy as i32));
                }
            });
        }
        let card_budget = std::time::Duration::from_millis(PROBE_CARD_MS);
        crate::gpu::march_sample(p, sxy, sxy, &|| t.elapsed() > card_budget, &mut |y, r| card.push((y, r.to_vec()))).is_ok()
    });
    if !card_ok {
        return None;
    }
    let cam = paint_camera(sc, p);
    let lv = light_vals(sc, &effective_lighting(sc), &cam);
    let soft = lv.lights.iter().any(|l| l.hs_mask == u16::MAX);
    // (MB3D_PROBE_DEBUG: the counts of clear differences: any, the object
    // or not, depth, normal, gradient, roughness)
    let mut n = [0usize; 6];
    let (mut total, mut diff) = (0usize, 0f64);
    for (&(y, x), c) in samples.iter().zip(&cpu) {
        let (Some(b), Some(a)) = (c.get(), card.iter().find(|(yy, _)| *yy == y).and_then(|(_, r)| r.get(x / sxy))) else { continue };
        total += 1;
        // (painted lit: the soft shadow's light in full, no hard shadows)
        let lit = |si: &SiLight| SiLight { shadow: if soft { 0xFC00 } else { 0 }, ..*si };
        let (ca, cb) = (lv.pixel_color(&lit(a), (x + x0) as i32, (y + y0) as i32, &cam), lv.pixel_color(&lit(b), (x + x0) as i32, (y + y0) as i32, &cam));
        diff += ca.iter().zip(&cb).map(|(&u, &v)| (u as f64 - v as f64).abs()).sum::<f64>() / (3.0 * 255.0);
        let d = if a.is_background() || b.is_background() {
            [a.is_background() != b.is_background(), false, false, false, false]
        } else {
            let (u, v) = (a.normal.map(|x| x as f64), b.normal.map(|x| x as f64));
            let cos = (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / ((u[0] * u[0] + u[1] * u[1] + u[2] * u[2]) * (v[0] * v[0] + v[1] * v[1] + v[2] * v[2])).sqrt().max(1.0);
            [
                false,
                (a.zpos_fine as i64 - b.zpos_fine as i64).abs() > 256 << 8,
                cos < 0.9,
                ((a.si_gradient & 0x7FFF) as i32 - (b.si_gradient & 0x7FFF) as i32).abs() > 1000,
                ((a.zpos_fine & 0xFF) as i32 - (b.zpos_fine & 0xFF) as i32).abs() > 20,
            ]
        };
        n[0] += d.iter().any(|&v| v) as usize;
        for (k, &v) in d.iter().enumerate() {
            n[k + 1] += v as usize;
        }
    }
    if std::env::var_os("MB3D_PROBE_DEBUG").is_some() {
        let pc = |k: usize| n[k] as f64 * 100.0 / total.max(1) as f64;
        eprintln!("  probe: any {:.2} %, object {:.2} %, depth {:.2} %, normal {:.2} %, gradient {:.2} %, roughness {:.2} %", pc(0), pc(1), pc(2), pc(3), pc(4), pc(5));
    }
    (total >= 100).then(|| (diff / total as f64, total))
}

/// How much faster than the CPU threads the card must march for Auto to give
/// it the shadows and the DE occlusion (see `calculate_inner`).
#[cfg(feature = "gpu")]
const POST_ON_CARD_RATIO: f64 = 1.5;

/// The march with the CPU threads taking rows from the bottom at once and
/// the card from the top once `ready` (its shader compiled) - until they
/// meet.  Returns the G-buffer and the number of rows the card marched
/// (0: it never joined, or its rows were redone on the CPU).
#[cfg(feature = "gpu")]
fn march_mixed(
    p: &CalcParams,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    sink: Option<RowSink>,
    ready: &std::sync::atomic::AtomicBool,
    on_ready: &(dyn Fn() -> Option<(CalcParams, Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>)> + Sync),
) -> Result<(Vec<SiLight>, usize, Option<(CalcParams, Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>)>, f64), String> {
    use std::sync::Mutex;
    // how long the card marched (its rows per second against the CPU's)
    let card_secs = Mutex::new(0.0f64);
    let h = p.rect[3].max(0) as usize;
    // the rows still free: [top, bottom)
    let free = Mutex::new((0usize, h));
    let rows: Vec<Mutex<Vec<SiLight>>> = (0..h).map(|_| Mutex::new(Vec::new())).collect();
    let done = AtomicUsize::new(0);
    let on_card = AtomicUsize::new(0);
    let put = |y: usize, row: Vec<SiLight>| {
        if let Some(f) = sink {
            f(y, &row);
        }
        *rows[y].lock().unwrap() = row;
        progress((done.fetch_add(1, Ordering::Relaxed) + 1).min(h), h);
    };
    let mut card_err = None;
    let mut switched = None;
    let name = crate::gpu::name().unwrap_or_default();
    crate::gpu::set_status(if ready.load(Ordering::SeqCst) {
        format!("GPU {name}: the march shared with the CPU")
    } else {
        // (the program shows a note on this: see `main_form::COMPILE_NOTE`)
        "CPU (the graphics card's shaders are compiling)".into()
    });
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                crate::prio::calc_thread();
                if cancel() {
                    break;
                }
                let y = {
                    let mut f = free.lock().unwrap();
                    if f.1 <= f.0 {
                        break;
                    }
                    f.1 -= 1;
                    f.1
                };
                put(y, cpu_row(p, y));
            });
        }
        // the card, once its shader is ready
        let wait = |ready: &std::sync::atomic::AtomicBool| loop {
            let left = {
                let f = free.lock().unwrap();
                f.0 < f.1
            };
            if !left || cancel() {
                return false;
            }
            if ready.load(Ordering::SeqCst) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        if !wait(ready) {
            return;
        }
        // (or in double precision, once that shader is ready)
        if let Some((pd, flags)) = on_ready() {
            let go = wait(&flags[0]);
            switched = Some((pd, flags));
            if !go {
                return;
            }
        }
        let cp = switched.as_ref().map_or(p, |(pd, _)| pd);
        let claimed = Mutex::new(Vec::new());
        let t_card = Instant::now();
        let r = crate::gpu::march_bands(
            cp,
            &|band| {
                let mut f = free.lock().unwrap();
                (f.0 < f.1).then(|| {
                    let r = (f.0, band.min(f.1 - f.0));
                    f.0 += r.1;
                    claimed.lock().unwrap().push(r);
                    r
                })
            },
            cancel,
            &mut |y, row| {
                on_card.fetch_add(1, Ordering::Relaxed);
                put(y, row.to_vec());
            },
        );
        *card_secs.lock().unwrap() = t_card.elapsed().as_secs_f64();
        if let Err(e) = r {
            if e != "cancelled" {
                // the card's rows (also those it delivered: overflows) on
                // the CPU, by all threads
                let redo: Vec<usize> = claimed.into_inner().unwrap().into_iter().flat_map(|(r0, n)| r0..r0 + n).collect();
                let next = AtomicUsize::new(0);
                std::thread::scope(|s2| {
                    for _ in 0..threads {
                        s2.spawn(|| loop {
                            crate::prio::calc_thread();
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            if i >= redo.len() || cancel() {
                                break;
                            }
                            put(redo[i], cpu_row(p, redo[i]));
                        });
                    }
                });
                on_card.store(0, Ordering::Relaxed);
            }
            card_err = Some(e);
        }
    });
    if cancel() {
        return Err("cancelled".into());
    }
    let n_card = on_card.load(Ordering::Relaxed);
    let pct = n_card * 100 / h.max(1);
    crate::gpu::set_status(match card_err {
        Some(e) => format!("CPU: GPU error: {e}"),
        None if n_card == 0 && switched.is_some() => "CPU (the graphics card's double precision shader was still compiling)".into(),
        None if n_card == 0 => "CPU (the graphics card's shader was still compiling)".into(),
        None if switched.is_some() => format!("GPU {name}: {pct} % of the rows on the card (double precision), {} % on the CPU", 100 - pct),
        None => format!("GPU {name}: {pct} % of the rows on the card, {} % on the CPU", 100 - pct),
    });
    // (the card's parameters when it joined in double precision)
    let switched = switched.filter(|_| n_card > 0);
    Ok((rows.into_iter().flat_map(|r| r.into_inner().unwrap()).collect(), n_card, switched, card_secs.into_inner().unwrap()))
}

/// The post calculations after the main calculation (CPU or GPU).
fn finish_calculation(
    sc: &Scene,
    p: CalcParams,
    mut gbuf: Vec<SiLight>,
    post: u8,
    threads: usize,
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(CalcParams, Vec<SiLight>), String> {
    let two_d = p.slice_2d != 0;
    if two_d || post == 0 {
        // MB3D runs no post calculations after a 2D calculation
    } else if post == 2 {
        post_process(sc, &p, &mut gbuf, threads);
    } else if sc.shadows.is_some() || sc.deao.is_some() {
        let mut s2 = sc.clone();
        if s2.deao.is_none() {
            s2.ao = None;
        }
        post_process(&s2, &p, &mut gbuf, threads);
    }
    if cancel() {
        return Err("cancelled".into());
    }
    Ok((p, gbuf))
}

/// The post calculations the scene asks for that the graphics card can
/// run with the main calculation.
/// Starts compiling the card's shaders for the scene in the background
/// (opening a file, changing a formula), so that they are ready when it is
/// calculated: the march with its shadow and occlusion passes, and the
/// reflection pass.  Parameter changes need no new shaders, formula
/// changes do (the shaders are made per formula combination).
pub fn prewarm_gpu(sc: Scene) {
    #[cfg(feature = "gpu")]
    std::thread::spawn(move || {
        crate::prio::calc_thread();
        if !crate::gpu::enabled() {
            return;
        }
        let Ok(p) = CalcParams::new(&sc) else { return };
        if p.slice_2d != 0 {
            return;
        }
        let _ = crate::gpu::march_prepare(&p, &post_job(&sc, &p));
        if sc.mc.reflections {
            crate::gpu::reflect_prepare(&p, sc.shadows.is_some(), sc.ao.is_some());
        }
    });
    #[cfg(not(feature = "gpu"))]
    let _ = sc;
}

pub fn post_job(sc: &Scene, p: &CalcParams) -> PostJob {
    PostJob { shadows: shadow_job(sc, p), deao: sc.ao.and(sc.deao) }
}

/// Whether [`post_process`] has anything left to do for the scene (the
/// graphics card may have run the shadows and the ambient occlusion).
pub fn post_pending(sc: &Scene, p: &CalcParams) -> bool {
    if p.slice_2d != 0 {
        return false;
    }
    let job = post_job(sc, p);
    let done = p.gpu_post.as_ref();
    let skip = |want: bool, same: bool| want && done.is_some() && same;
    let shadows = sc.shadows.is_some() && !skip(job.shadows.is_some(), done.is_some_and(|d| d.shadows == job.shadows));
    let ao = sc.ao.is_some() && !skip(job.deao.is_some(), done.is_some_and(|d| d.deao == job.deao));
    sc.normals_on_zbuf || shadows || ao
}

/// The times of the stages after the main calculation (the Infos tab):
/// None when the scene has no such stage, `Some(None)` when the graphics
/// card ran it with the main calculation.
#[derive(Clone, Copy, Debug, Default)]
pub struct StageTimes {
    pub shadows: Option<Option<f64>>,
    pub ao: Option<Option<f64>>,
    pub reflections: Option<f64>,
}

/// The automatic post calculations on the G-buffer (hard shadows, ambient
/// occlusion), as MB3D runs them after the main calculation.  What the
/// graphics card already ran with the same settings (`p.gpu_post`) is kept;
/// what it ran with other settings is undone first.  Returns the times of
/// the shadows and the occlusion.
pub fn post_process(sc: &Scene, p: &CalcParams, gbuf: &mut [SiLight], threads: usize) -> StageTimes {
    let mut times = StageTimes::default();
    if sc.normals_on_zbuf {
        crate::reflect::normals_on_zbuf(sc, p, gbuf);
    }
    let job = post_job(sc, p);
    let (mut shadows_done, mut deao_done) = (false, false);
    if let Some(d) = &p.gpu_post {
        shadows_done = job.shadows.is_some() && d.shadows == job.shadows;
        deao_done = job.deao.is_some() && d.deao == job.deao;
        if d.shadows.is_some() && !shadows_done {
            for s in gbuf.iter_mut() {
                s.shadow &= 0x3FF;
            }
        }
        if d.deao.is_some() && !deao_done {
            for s in gbuf.iter_mut() {
                s.amb_shadow = 5000;
            }
        }
    }
    if sc.shadows.is_some() {
        times.shadows = Some(None);
    }
    if sc.shadows.is_some() && !shadows_done {
        let t = Instant::now();
        hard_shadows(sc, p, gbuf, threads);
        if std::env::var_os("MB3D_GPU_TIMING").is_some() {
            eprintln!("  hard shadows (CPU): {:.2} s", t.elapsed().as_secs_f64());
        }
        times.shadows = Some(Some(t.elapsed().as_secs_f64()));
    }
    if let Some(ao) = &sc.ao {
        times.ao = Some(None);
        let t = Instant::now();
        if let Some(d) = &sc.deao {
            if !deao_done {
                crate::deao::deao(p, gbuf, p.rect[2] as usize, p.rect[3] as usize, d, threads);
                times.ao = Some(Some(t.elapsed().as_secs_f64()));
            }
        } else {
            ssao(gbuf, p.rect[2] as usize, p.rect[3] as usize, p, ao, threads);
            times.ao = Some(Some(t.elapsed().as_secs_f64()));
        }
    }
    times
}

fn ssao(gbuf: &mut [SiLight], w: usize, h: usize, p: &CalcParams, ao: &crate::ssao::SsaoParams, threads: usize) {
    #[cfg(feature = "gpu")]
    if crate::gpu::enabled() && !crate::gpu::compiling() {
        // only the depth buffer is needed: also after a CPU calculation
        // (but not while the card compiles a shader: the pass would wait
        // behind the compile)
        let r = if ao.bits15 { crate::gpu::ssao15(gbuf, w, h, p.zc_mul, p.zcorr, ao) } else { crate::gpu::ssao24(gbuf, w, h, p.zc_mul, p.zcorr, ao) };
        match r {
            Ok(()) => {
                let s = crate::gpu::last_status();
                if s.starts_with("GPU") && !s.contains("+SSAO") {
                    crate::gpu::set_status(format!("{s} +SSAO"));
                }
                return;
            }
            Err(e) => eprintln!("SSAO on the GPU failed, calculated on the CPU: {e}"),
        }
    }
    if ao.bits15 {
        crate::ssao15::ssao15(gbuf, w, h, p.zc_mul, p.zcorr, ao, threads);
    } else {
        crate::ssao::ssao24(gbuf, w, h, p.zc_mul, p.zcorr, ao, threads);
    }
}

/// The lighting with the hard shadow state of the scene applied
/// (`iHScalced`, `iHSmask`, "set diffuse function to cos").
pub(crate) fn effective_lighting(sc: &Scene) -> crate::lighting::Lighting {
    let mut l = sc.lighting.clone();
    if let Some(hs) = &sc.shadows {
        l.hs_calced = hs.lights & 0x3F;
        l.hs_soft = hs.soft;
        l.hs_set_cos = hs.set_cos;
    }
    l
}

pub(crate) fn light_vals(sc: &Scene, l: &crate::lighting::Lighting, cam: &PaintCamera) -> LightVals {
    let z_range = sc.z_end - sc.z_start;
    let vol = |s: &Scene, l: &crate::lighting::Lighting| s.vol_light.is_some_and(|v| l.lights[v.light].on);
    let make = |s: &Scene, l: &crate::lighting::Lighting, all: bool| {
        let zr = s.z_end - s.z_start;
        let mut lv = if all {
            LightVals::new_all(l, s.z_step_div, s.dfog_on_it, zr, s.step_width(), s.width, 1.0)
        } else {
            LightVals::new(l, s.z_step_div, s.dfog_on_it, zr, s.step_width(), s.width, 1.0)
        };
        if vol(s, l) {
            lv.set_vol_light(l, zr);
        }
        lv
    };
    let blended_l;
    let (mut lv, l) = match &sc.light_blend {
        // an animation frame: MB3D interpolates the light values of the keyframes
        Some(b) => {
            let mut lv = make(sc, l, true);
            let keys: Vec<LightVals> = b.keys.iter().map(|k| make(k, &effective_lighting(k), true)).collect();
            lv.blend(&keys, &b.weights, b.t, b.bezier);
            blended_l = blend_light_positions(l, b);
            (lv, &blended_l)
        }
        None => (make(sc, l, false), l),
    };
    lv.rotate_object_lights(l, &normalise_matrix_to(1.0, &sc.vgrads));
    let vz = normalise_matrix_to(1.0, &sc.vgrads)[2];
    lv.place_lights(l, sc.stereo_mid(), cam, z_range, vz);
    lv
}

/// Positional lights of an animation frame: the absolute positions of the
/// keyframes interpolated (lights that are positional in all keyframes).
fn blend_light_positions(l: &crate::lighting::Lighting, b: &crate::anim::LightBlend) -> crate::lighting::Lighting {
    let mut l = l.clone();
    for (i, li) in l.lights.iter_mut().enumerate() {
        if b.keys.iter().all(|k| k.lighting.lights[i].positional) {
            let mut p = [0.0; 3];
            for (k, w) in b.keys.iter().zip(&b.weights) {
                for (c, v) in p.iter_mut().enumerate() {
                    *v += k.lighting.lights[i].position[c] * w;
                }
            }
            li.position = p;
        }
    }
    l
}

/// `PaintParameter` / `GetStartSPosAndAddVecs`
pub fn paint_camera(sc: &Scene, p: &CalcParams) -> PaintCamera {
    let w = sc.width;
    let h = sc.height;
    let fov = if sc.optic == CameraOptic::Panorama {
        std::f32::consts::PI
    } else {
        sc.fov_y.to_radians() as f32
    };
    let d = (fov as f64 * 0.5).clamp(0.01, 1.5);
    let m = normalise_matrix_to(1.0, &p.vgrads);
    let mf = [
        [m[0][0] as f32, m[0][1] as f32, m[0][2] as f32],
        [m[1][0] as f32, m[1][1] as f32, m[1][2] as f32],
        [m[2][0] as f32, m[2][1] as f32, m[2][2] as f32],
    ];
    let sw = p.step_width;
    let sp = if sc.optic == CameraOptic::Panorama {
        [0.0, 0.0, p.zz_stmit_dif]
    } else {
        [-0.5 * w as f64 * sw, -0.5 * h as f64 * sw, p.zz_stmit_dif]
    };
    let sp = crate::math::rotate_vector_reverse(&sp, &m);
    PaintCamera {
        width: w,
        height: h,
        fov,
        aspect: if sc.optic == CameraOptic::Panorama { 2.0 } else { w as f32 / h as f32 },
        x_off: sc.stereo_xoff(),
        planar: sc.optic as i32,
        pl_optic_z: (d.cos() * d / d.sin()) as f32,
        zcorr: p.zcorr,
        zc_mul: p.zc_mul,
        step_width: p.step_width,
        zz_stmit_dif: p.zz_stmit_dif,
        m: mf,
        start_pos: [sp[0] as f32, sp[1] as f32, sp[2] as f32],
        x_add: [(m[0][0] * sw) as f32, (m[0][1] * sw) as f32, (m[0][2] * sw) as f32],
        y_add: [(m[1][0] * sw) as f32, (m[1][1] * sw) as f32, (m[1][2] * sw) as f32],
    }
}

/// The shadow pass of the scene: the selected lights as `HSvecs`, soft or
/// hard; `None` when the scene has no shadows (or no selected light).
pub fn shadow_job(sc: &Scene, p: &CalcParams) -> Option<ShadowJob> {
    let hs = sc.shadows.as_ref()?;
    let l = effective_lighting(sc);
    let lv = light_vals(sc, &l, &paint_camera(sc, p));
    // CalcHSVecsFromLights: HSvecs = RotateVectorReverse(-StepWidth * LN, M)
    let m = normalise_matrix_to(1.0, &p.vgrads);
    // positional lights: their position (`LN + Xmit`); the direction is
    // taken at each pixel
    let mid = sc.stereo_mid();
    let mut lights: Vec<HsLight> = lv
        .lights
        .iter()
        .filter(|l| (hs.lights >> l.idx) & 1 != 0)
        .map(|l| {
            let ln = [l.ln[0] as f64, l.ln[1] as f64, l.ln[2] as f64];
            if l.positional {
                let pos = [ln[0] + mid[0], ln[1] + mid[1], ln[2] + mid[2]];
                return HsLight { idx: l.idx, vec: [0.0; 3], pos: Some((pos, l.lmax_l as f64 * hs.max_len_mul as f64)), visible: l.visible };
            }
            let v = crate::math::normalize(ln);
            let v = [-v[0] * p.step_width, -v[1] * p.step_width, -v[2] * p.step_width];
            HsLight { idx: l.idx, vec: crate::math::rotate_vector_reverse(&v, &m), pos: None, visible: l.visible }
        })
        .collect();
    if lights.is_empty() {
        return None;
    }
    if hs.soft {
        // calcHSsoft uses the last selected light
        lights = vec![*lights.last().unwrap()];
    }
    Some(ShadowJob { lights, soft_radius: if hs.soft { hs.soft_radius.max(0.001) } else { 0.0 }, max_len_mul: hs.max_len_mul })
}

/// `CalcHardShadowT`: hard (or one soft) shadow for the selected global
/// lights, stored in the shadow bits of the G-buffer.
pub fn hard_shadows(sc: &Scene, p: &CalcParams, gbuf: &mut [SiLight], threads: usize) {
    let Some(hs) = &sc.shadows else { return };
    let Some(job) = shadow_job(sc, p) else { return };
    // the HS calculation always uses 8 binary search steps
    let mut p = p.clone();
    p.de_add_steps = 8;
    let p = &p;
    let w = p.rect[2] as usize;
    let h = p.rect[3] as usize;
    let (x0, y0) = (p.rect[0], p.rect[1] as usize);
    let threads = threads.min(h).max(1);
    let lights = &job.lights;
    std::thread::scope(|s| {
        let mut rows: Vec<(usize, &mut [SiLight])> = gbuf.chunks_mut(w).enumerate().collect();
        let mut per: Vec<Vec<(usize, &mut [SiLight])>> = (0..threads).map(|_| Vec::new()).collect();
        for (i, r) in rows.drain(..) {
            per[i % threads].push((i, r));
        }
        for list in per {
            s.spawn(move || {
                crate::prio::calc_thread();
                for (y, row) in list {
                    let y = y + y0;
                    let seed = (0x24563487u32 as i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
                    let mut m = Marcher::new(p, seed);
                    if hs.soft {
                        m.soft_shadow_row(y as i32, x0, row, &lights[0], hs.soft_radius, hs.max_len_mul);
                    } else {
                        m.hard_shadow_row(y as i32, x0, row, lights, hs.max_len_mul);
                    }
                }
            });
        }
    });
}

/// Paints the G-buffer with the scene lighting.
pub fn paint(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> Vec<u8> {
    paint_with::<u8>(sc, p, gbuf, true).0
}

/// [`paint`] in 16 bit per colour channel (the colours as calculated,
/// before MB3D's rounding to 8 bit): `mb3d --bits 16`, "PNG 16 bit colour".
pub fn paint16(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> Vec<u16> {
    paint_with::<u16>(sc, p, gbuf, true).0
}

/// [`paint`], and the time of the reflections (None without them).
pub fn paint_timed(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> (Vec<u8>, Option<f64>) {
    paint_with::<u8>(sc, p, gbuf, true)
}

/// [`paint`] without the reflections and the depth of field: the preview
/// of the rows calculated so far (both need the whole image, and the
/// reflections march again, taking as long as the calculation itself).
pub fn paint_preview(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> Vec<u8> {
    paint_with::<u8>(sc, p, gbuf, false).0
}

fn paint_with<S: crate::lighting::Sample>(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], full: bool) -> (Vec<S>, Option<f64>) {
    let [x0, ry0, w, h] = p.rect;
    let l = effective_lighting(sc);
    let cam = paint_camera(sc, p);
    let lv = light_vals(sc, &l, &cam);
    let threads = thread_count(sc).min(h as usize).max(1);
    let mut rgb = vec![S::default(); (w * h * 3) as usize];
    let chunk_rows = ((h as usize) + threads - 1) / threads;
    std::thread::scope(|s| {
        for (ci, chunk) in rgb.chunks_mut(chunk_rows * w as usize * 3).enumerate() {
            let lv = &lv;
            let cam = &cam;
            s.spawn(move || {
                crate::prio::calc_thread();
                let y0 = ci * chunk_rows;
                for (ry, row) in chunk.chunks_mut(w as usize * 3).enumerate() {
                    let y = y0 + ry;
                    for x in 0..w as usize {
                        let c = lv.pixel_color_s::<S>(&gbuf[y * w as usize + x], x as i32 + x0, y as i32 + ry0, cam);
                        row[x * 3..x * 3 + 3].copy_from_slice(&c);
                    }
                }
            });
        }
    });
    if !full {
        return (rgb, None);
    }
    let mut refl_s = None;
    if sc.mc.reflections && p.slice_2d == 0 {
        let t = Instant::now();
        crate::reflect::reflections(sc, p, gbuf, &mut rgb, &cam, &lv, threads);
        refl_s = Some(t.elapsed().as_secs_f64());
    }
    if let Some(d) = sc.dof.as_ref().filter(|_| p.slice_2d == 0) {
        crate::dof::apply(&mut rgb, gbuf, w as usize, h as usize, p.zc_mul, p.zcorr, sc.fov_y, d);
    }
    (rgb, refl_s)
}

/// Chooses `color_start` / `color_end` so that the palette spans the 2nd to
/// 98th percentile of the smoothed iteration gradient of the visible surface
/// (a convenience of this port, MB3D only has the manual sliders).
pub fn auto_color_range(sc: &mut Scene, gbuf: &[SiLight]) {
    let mut v: Vec<f64> = gbuf
        .iter()
        .filter(|s| !s.is_background() && s.si_gradient < 32768)
        .map(|s| s.si_gradient as f64)
        .collect();
    if v.len() < 16 {
        return;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |f: f64| v[((v.len() - 1) as f64 * f) as usize];
    // inverse of sCStart := Sqr((TB + 30) / 90) * 32767 - 10900
    let tb = |si: f64| 90.0 * ((si + 10900.0).max(0.0) / 32767.0).sqrt() - 30.0;
    let (lo, hi) = (q(0.02), q(0.98).max(q(0.02) + 64.0));
    sc.lighting.color_start = tb(lo) as f32;
    sc.lighting.color_end = tb(hi) as f32;
}

/// Full render: calculation + painting.
pub fn render(sc: &Scene, progress: &(dyn Fn(usize, usize) + Sync)) -> Result<RenderResult, String> {
    let t0 = Instant::now();
    let (p, gbuffer) = calculate(sc, progress)?;
    let t1 = Instant::now();
    let rgb = paint(sc, &p, &gbuffer);
    let t2 = Instant::now();
    Ok(RenderResult {
        width: p.rect[2] as usize,
        height: p.rect[3] as usize,
        gbuffer,
        rgb,
        calc_seconds: (t1 - t0).as_secs_f64(),
        paint_seconds: (t2 - t1).as_secs_f64(),
    })
}

/// Box filter downsampling of an RGB image by an integer factor (used for
/// anti-aliasing: render at n times the size, then downsample).
pub fn downsample<S: Copy + Default + Into<u32> + TryFrom<u32>>(rgb: &[S], w: usize, h: usize, n: usize) -> (Vec<S>, usize, usize) {
    let (w2, h2) = (w / n, h / n);
    let mut out = vec![S::default(); w2 * h2 * 3];
    let nn = (n * n) as u32;
    for y in 0..h2 {
        for x in 0..w2 {
            for c in 0..3 {
                let mut sum = 0u32;
                for dy in 0..n {
                    for dx in 0..n {
                        sum += rgb[((y * n + dy) * w + x * n + dx) * 3 + c].into();
                    }
                }
                out[(y * w2 + x) * 3 + c] = S::try_from((sum + nn / 2) / nn).unwrap_or_default();
            }
        }
    }
    (out, w2, h2)
}

/// Result of [`render_tiled`]: the stitched image of the rendered region
/// (the whole image, or one tile) and optionally its G-buffer.
pub struct TiledResult {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
    pub gbuffer: Option<Vec<SiLight>>,
    pub calc_seconds: f64,
    pub paint_seconds: f64,
    /// Calc parameters of the last tile (z conversion for statistics)
    pub params: Option<CalcParams>,
}

/// Overlap around a single tile so that the image space effects (SSAO,
/// DOF) see some context; the overlap is calculated but cut away.
pub const TILE_BORDER: i32 = 24;

/// Tiled rendering of `sc.tiling`.
///
/// All tiles (`tiling.pos` = None): the tiles are calculated one after
/// another into one G-buffer; SSAO, painting and DOF then run on the whole
/// image, so the result equals an untiled render.
///
/// One tile: only that tile is calculated (with a border of
/// [`TILE_BORDER`] pixels) and painted, like MB3D's big-render tiles; the
/// image space effects only see the tile.
///
/// `tile_progress(done, total, rect)` is called after each tile, `progress`
/// per row as in [`calculate`].
pub fn render_tiled(
    sc: &Scene,
    keep_gbuffer: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    tile_progress: &dyn Fn(usize, usize, [i32; 4]),
) -> Result<TiledResult, String> {
    let tl = sc.tiling.ok_or("no tiling set")?;
    let (w, h) = (sc.width, sc.height);
    if let Some((c, r)) = tl.pos {
        let t = tl.tile_rect(w, h, c, r);
        if t[2] <= 0 || t[3] <= 0 {
            return Err("empty tile".into());
        }
        let border = TILE_BORDER.max(tl.downscale as i32);
        let x0 = (t[0] - border).max(0);
        let y0 = (t[1] - border).max(0);
        let x1 = (t[0] + t[2] + border).min(w);
        let y1 = (t[1] + t[3] + border).min(h);
        let mut ts = sc.clone();
        ts.calc_rect = Some([x0, y0, x1 - x0, y1 - y0]);
        let t0 = Instant::now();
        let (p, gbuf) = calculate(&ts, progress)?;
        let t1 = Instant::now();
        let trgb = paint(&ts, &p, &gbuf);
        let paint_s = t1.elapsed().as_secs_f64();
        let tw = (x1 - x0) as usize;
        let (ow, oh) = (t[2] as usize, t[3] as usize);
        let (sx, sy) = ((t[0] - x0) as usize, (t[1] - y0) as usize);
        let mut rgb = Vec::with_capacity(ow * oh * 3);
        let mut g = Vec::with_capacity(if keep_gbuffer { ow * oh } else { 0 });
        for y in 0..oh {
            let o = (sy + y) * tw + sx;
            rgb.extend_from_slice(&trgb[o * 3..(o + ow) * 3]);
            if keep_gbuffer {
                g.extend_from_slice(&gbuf[o..o + ow]);
            }
        }
        tile_progress(1, 1, t);
        return Ok(TiledResult {
            width: ow,
            height: oh,
            rgb,
            gbuffer: keep_gbuffer.then_some(g),
            calc_seconds: (t1 - t0).as_secs_f64(),
            paint_seconds: paint_s,
            params: Some(p),
        });
    }
    let (wu, hu) = (w as usize, h as usize);
    let mut full = vec![SiLight::default(); wu * hu];
    let total = (tl.cols * tl.rows) as usize;
    let t0 = Instant::now();
    let mut n = 0;
    for r in 0..tl.rows {
        for c in 0..tl.cols {
            let t = tl.tile_rect(w, h, c, r);
            n += 1;
            if t[2] <= 0 || t[3] <= 0 {
                continue;
            }
            let mut ts = sc.clone();
            ts.calc_rect = Some(t);
            let (_, gbuf) = calculate_inner(&ts, progress, &|| false, 1, None)?;
            let tw = t[2] as usize;
            for y in 0..t[3] as usize {
                let d = (t[1] as usize + y) * wu + t[0] as usize;
                full[d..d + tw].copy_from_slice(&gbuf[y * tw..(y + 1) * tw]);
            }
            tile_progress(n, total, t);
        }
    }
    let mut fs = sc.clone();
    fs.calc_rect = None;
    let p = CalcParams::new(&fs)?;
    if let (Some(ao), None) = (&sc.ao, &sc.deao) {
        ssao(&mut full, wu, hu, &p, ao, thread_count(sc));
    }
    let t1 = Instant::now();
    let rgb = paint(&fs, &p, &full);
    Ok(TiledResult {
        width: wu,
        height: hu,
        rgb,
        gbuffer: keep_gbuffer.then_some(full),
        calc_seconds: (t1 - t0).as_secs_f64(),
        paint_seconds: t1.elapsed().as_secs_f64(),
        params: Some(p),
    })
}

/// How [`compose_stereo`] puts a left and a right eye image together.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StereoLayout {
    /// left image left, right image right (parallel viewing)
    Parallel,
    /// right image left (cross-eyed viewing)
    Cross,
    /// red-cyan anaglyph (half colour: red from the left eye's luminance)
    Anaglyph,
}

impl StereoLayout {
    pub fn parse(s: &str) -> Result<StereoLayout, String> {
        match s.to_ascii_lowercase().as_str() {
            "parallel" | "side-by-side" | "sbs" => Ok(StereoLayout::Parallel),
            "cross" | "cross-eyed" | "crosseyed" => Ok(StereoLayout::Cross),
            "anaglyph" | "red-cyan" => Ok(StereoLayout::Anaglyph),
            _ => Err(format!("bad stereo layout '{s}' (parallel, cross, anaglyph)")),
        }
    }
}

/// Combines the images of the left and right eye (MB3D renders them one at
/// a time; this is an addition of the port).
pub fn compose_stereo(left: &[u8], right: &[u8], w: usize, h: usize, layout: StereoLayout) -> (Vec<u8>, usize, usize) {
    match layout {
        StereoLayout::Anaglyph => {
            let mut out = right.to_vec();
            for (o, l) in out.chunks_mut(3).zip(left.chunks(3)) {
                o[0] = (0.299 * l[0] as f32 + 0.587 * l[1] as f32 + 0.114 * l[2] as f32).round().min(255.0) as u8;
            }
            (out, w, h)
        }
        _ => {
            let (a, b) = if layout == StereoLayout::Parallel { (left, right) } else { (right, left) };
            let mut out = vec![0u8; w * 2 * h * 3];
            for y in 0..h {
                out[y * w * 6..y * w * 6 + w * 3].copy_from_slice(&a[y * w * 3..(y + 1) * w * 3]);
                out[y * w * 6 + w * 3..(y + 1) * w * 6].copy_from_slice(&b[y * w * 3..(y + 1) * w * 3]);
            }
            (out, w * 2, h)
        }
    }
}
