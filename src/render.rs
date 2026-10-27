//! Multithreaded rendering: calculation of the G-buffer (`CalcMandT` +
//! `TMandCalcThread`) followed by painting (`PaintRows`).
//!
//! Like MB3D, rows are distributed interleaved over the threads
//! (thread t calculates rows t, t + n, t + 2n, ...).

use crate::calc::{CalcParams, HsLight, Marcher, PostJob, ShadowJob};
use crate::checkpoint::{Policy, Store};
use crate::gbuffer::SiLight;
use crate::lighting::{LightVals, PaintCamera};
use crate::math::normalise_matrix_to;
use crate::scene::{CameraOptic, Scene, ShadowParams};
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

/// The rows of the running march the card calculated (for the window's
/// time estimate: how much the card adds to the CPU threads).
pub static CARD_ROWS: AtomicUsize = AtomicUsize::new(0);

/// The cost of each row of a calculation, for `progress` to count the work
/// done rather than the rows: a row of sky takes milliseconds, a row of
/// the object seconds, and the rows are handed out in order (the CPU from
/// one end, the card from the other), so a time estimate from the rows
/// done is off by the cost ratio of the rows to come and the rows done.
/// `progress(done, total)` gets the weights of the rows done and of all
/// rows; the weights are integers, each at least 1.
#[derive(Clone, Debug, PartialEq)]
pub struct RowWeights {
    /// `cum[y]`: the weights of the rows before `y` (`cum[h]`: of all)
    cum: Vec<usize>,
}

impl RowWeights {
    /// Every row the same (what the progress counted before: rows).
    pub fn equal(h: usize) -> Self {
        Self::new(vec![1; h])
    }

    /// The rows' weights (0 counts as 1).
    pub fn new(w: Vec<usize>) -> Self {
        let mut cum = Vec::with_capacity(w.len() + 1);
        cum.push(0);
        for v in w {
            cum.push(cum.last().unwrap() + v.max(1));
        }
        RowWeights { cum }
    }

    /// The costs of the rows of a picture of `h` rows from the G-buffer
    /// of a probe of `pw` x `ph` (the same view marched smaller): a
    /// pixel's cost is its DE steps plus a few for the ray itself (a
    /// background pixel costs something too), a row's the sum over its
    /// probe row.  The weights are scaled to a mean of about 100.
    pub fn from_probe(g: &[SiLight], pw: usize, ph: usize, h: usize) -> Self {
        if pw == 0 || ph == 0 || g.len() < pw * ph || h == 0 {
            return Self::equal(h);
        }
        let cost: Vec<f64> = (0..ph).map(|r| g[r * pw..(r + 1) * pw].iter().map(|s| (s.shadow & 0x3FF) as f64 + PIXEL_BASE_COST).sum()).collect();
        let rows: Vec<f64> = (0..h).map(|y| cost[(y * ph / h).min(ph - 1)]).collect();
        let mean = rows.iter().sum::<f64>() / h as f64;
        let k = if mean > 0.0 { 100.0 / mean } else { 1.0 };
        Self::new(rows.iter().map(|c| (c * k).round() as usize).collect())
    }

    /// The surface pixels of each row of `g` (`w` x `h`) plus one: what
    /// the passes after the march (shadows, occlusion, reflections) work
    /// on; a row of sky is done at once.
    pub fn surface(g: &[SiLight], w: usize, h: usize) -> Self {
        let w = w.max(1);
        Self::new((0..h).map(|y| g.get(y * w..(y + 1) * w).map_or(0, |r| r.iter().filter(|s| !s.is_background()).count()) + 1).collect())
    }

    pub fn rows(&self) -> usize {
        self.cum.len() - 1
    }

    pub fn weight(&self, y: usize) -> usize {
        self.cum.get(y + 1).map_or(0, |c| c - self.cum[y])
    }

    pub fn total(&self) -> usize {
        *self.cum.last().unwrap()
    }

    /// The weights of the first `n` rows (rows done in order).
    pub fn before(&self, n: usize) -> usize {
        self.cum[n.min(self.rows())]
    }

    /// The weights of the rows `ck` has.
    fn done_in(&self, ck: Option<&Store>) -> usize {
        match ck.filter(|s| s.done_count() > 0) {
            Some(s) => (0..self.rows()).filter(|&y| s.is_done(y)).map(|y| self.weight(y)).sum(),
            None => 0,
        }
    }
}

/// What a pixel costs besides its DE steps (the ray's setup, a background
/// pixel's march to the far plane), in steps.
const PIXEL_BASE_COST: f64 = 4.0;

/// The least pixels a picture has for its rows to be weighed by a probe
/// (`row_weights`): smaller ones are quick anyway.
const WORKMAP_MIN_PIXELS: usize = 100_000;

/// What the probe of the main calculation found ([`row_weights`]), for the
/// window's time estimate: the share of the pixels on the object, the
/// mean DE steps of a pixel, and the probe's own time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorkProbe {
    pub surface: f64,
    pub mean_steps: f64,
    pub seconds: f64,
    /// the pixels it marched (its pace: `pixels / seconds`)
    pub pixels: usize,
}

static WORK_PROBE: std::sync::Mutex<Option<WorkProbe>> = std::sync::Mutex::new(None);

/// The probe of the main calculation under way (None before it ran, or
/// when it was skipped).
pub fn work_probe() -> Option<WorkProbe> {
    *WORK_PROBE.lock().unwrap()
}

/// The weights of the rows of the calculation of `sc` (`p` its
/// parameters): a probe marches the view at 1/8 of its size (1.6 % of the
/// pixels) through the same path the calculation takes, raw, and its DE
/// steps weigh the rows ([`RowWeights::from_probe`]).  Equal weights for
/// small pictures, 2D slices, when the probe fails, and with
/// MB3D_NO_WORKMAP set.
fn row_weights(sc: &Scene, p: &CalcParams, cancel: &(dyn Fn() -> bool + Sync)) -> RowWeights {
    let (w, h) = (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize);
    *WORK_PROBE.lock().unwrap() = None;
    if w * h < WORKMAP_MIN_PIXELS || p.slice_2d != 0 || std::env::var_os("MB3D_NO_WORKMAP").is_some() {
        return RowWeights::equal(h);
    }
    // the march alone (as in `march_sig`), and no volumetric light map
    let mut s = sc.clone();
    s.ao = None;
    s.shadows = None;
    s.deao = None;
    s.normals_on_zbuf = false;
    s.dof = None;
    s.mc = Default::default();
    s.vol_light = None;
    s.scale_image(1.0 / 8.0);
    let f = s.width as f64 / sc.width.max(1) as f64;
    if let Some(r) = s.calc_rect {
        let at = |v: i32| (v as f64 * f).round() as i32;
        let size = |v: i32, max: i32| ((v as f64 * f).round() as i32).clamp(1, max.max(1));
        let (x, y) = (at(r[0]).clamp(0, s.width - 1), at(r[1]).clamp(0, s.height - 1));
        s.calc_rect = Some([x, y, size(r[2], s.width - x), size(r[3], s.height - y)]);
    }
    let t = Instant::now();
    let Ok((q, g)) = calculate_rows(&s, &|_, _| {}, &no_stage, cancel, 0, None, None, None) else {
        return RowWeights::equal(h);
    };
    let (pw, ph) = (q.rect[2].max(0) as usize, q.rect[3].max(0) as usize);
    let n = g.len().max(1) as f64;
    let probe = WorkProbe {
        surface: g.iter().filter(|s| !s.is_background()).count() as f64 / n,
        mean_steps: g.iter().map(|s| (s.shadow & 0x3FF) as f64).sum::<f64>() / n,
        seconds: t.elapsed().as_secs_f64(),
        pixels: g.len(),
    };
    if std::env::var_os("MB3D_GPU_TIMING").is_some() {
        eprintln!("  work probe: {pw}x{ph} in {:.0} ms, {:.0} % surface, {:.0} DE steps per pixel", probe.seconds * 1000.0, probe.surface * 100.0, probe.mean_steps);
    }
    *WORK_PROBE.lock().unwrap() = Some(probe);
    RowWeights::from_probe(&g, pw, ph, h)
}

pub(crate) fn thread_count(sc: &Scene) -> usize {
    if sc.threads > 0 {
        sc.threads
    } else {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
    }
}

/// Calculates the G-buffer.  `progress` is called from the worker threads
/// with the work done and the work of the whole (the rows done, weighed by
/// what they cost: [`RowWeights`]).
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
    calculate_inner(sc, progress, &no_stage, cancel, 2, None, None)
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
    calculate_inner(sc, progress, &no_stage, cancel, 0, None, None)
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
    calculate_inner(sc, progress, &no_stage, cancel, 0, Some(rows), None)
}

/// Where a calculation runs (the card's rows differ a little from the CPU's).
fn gpu_mode() -> &'static str {
    #[cfg(feature = "gpu")]
    return if !crate::gpu::enabled() {
        "cpu"
    } else if crate::gpu::auto() {
        "auto"
    } else {
        "card"
    };
    #[cfg(not(feature = "gpu"))]
    "cpu"
}

/// The text that decides a whole result (Monte Carlo, tiles, voxels), for
/// its checkpoint key: the scene without what only chooses how it is
/// calculated, and where it is calculated.
pub(crate) fn checkpoint_text(sc: &Scene) -> String {
    // (the scene text: what the swarm hands on to calculate the same
    // picture, and the same again when it is loaded)
    let mut s = sc.clone();
    s.threads = 0;
    format!("{}\ngpu = {}", s.to_text(), gpu_mode())
}

/// Everything the ray marching (the raw G-buffer) depends on: the scene
/// without the lighting (unless the volumetric light needs it) and without
/// the passes after the march.
pub fn march_sig(sc: &Scene) -> String {
    let mut s = sc.clone();
    if s.vol_light.is_none() {
        s.lighting = Default::default();
    }
    s.ao = None;
    s.shadows = None;
    s.deao = None;
    s.normals_on_zbuf = false;
    s.dof = None;
    s.mc = Default::default();
    s.threads = 0;
    s.to_text()
}

/// The checkpoint key of a calculation of `sc` (Calculate 3D: the same for
/// the program and the command line).  Its rows are the raw G-buffer, so
/// the key leaves out the lighting and the passes after the march: a
/// stopped render changed only in those still continues (rows the card
/// shadowed during the march carry their shadows' settings, checked when
/// continuing).
pub fn checkpoint_key(sc: &Scene) -> String {
    // (the part of the picture calculated: the scene text leaves it out,
    // and the tiles of a tiled render differ only in it)
    let rect = sc.calc_rect.map(|r| format!("\ncalc_rect = {} {} {} {}", r[0], r[1], r[2], r[3])).unwrap_or_default();
    crate::checkpoint::key("calc", &format!("{}\ngpu = {}{rect}", march_sig(sc), gpu_mode()))
}

/// The checkpoint of a calculation of `sc` (its G-buffer, row by row; see
/// [`calculate_with`]), `name` shown for it.  None with `Policy::Off` or
/// when it cannot be kept.
pub fn open_checkpoint(sc: &Scene, name: &str, policy: Policy) -> Option<Store> {
    let p = CalcParams::new(sc).ok()?;
    let (w, h) = (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let spec = crate::checkpoint::Spec { kind: "calc".into(), key: checkpoint_key(sc), name: name.into(), width: w, height: h, rec: crate::gbuffer::REC_BYTES, units: h };
    let s = Store::open(spec, policy)?;
    s.put_extra("scene.m3s", sc.to_text().as_bytes());
    Some(s)
}

/// The calculation of [`calculate_raw_rows`] (`post` 0) or
/// [`calculate_cancellable`] (`post` 2) with its rows kept in `ck` (from
/// [`open_checkpoint`]): the rows it holds are not calculated again.  The
/// caller ends `ck` when the result is saved ([`Store::finish`]); dropped,
/// it is kept for the next time.
///
/// `stage` is told the name of each stage after the march as it starts
/// ("hard shadows (graphics card)", ...): from then on `progress` counts
/// that stage's rows, from zero again (weighed by their surface pixels).
pub fn calculate_with(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    post: u8,
    rows: Option<RowSink>,
    ck: Option<&Store>,
) -> Result<(CalcParams, Vec<SiLight>), String> {
    calculate_inner(sc, progress, stage, cancel, post, rows, ck)
}

/// A stage callback that ignores the stages.
pub fn no_stage(name: &str) {
    stage_mark(name);
}

/// `MB3D_STAGE_TIMES=1`: the time of every stage of a render is measured
/// (the stage names that the calculation reports, [`no_stage`]) and
/// printed when it ends ([`stage_times_print`]); for finding where a
/// scene's time goes. Off: nothing is measured.
fn stage_times_on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("MB3D_STAGE_TIMES").is_some_and(|v| v != "0"))
}

type StageLog = (Instant, String, Vec<(String, f64)>);
static STAGE_LOG: std::sync::Mutex<Option<StageLog>> = std::sync::Mutex::new(None);

/// A stage begins (the one before ends): see [`stage_times_on`].
pub fn stage_mark(name: &str) {
    if !stage_times_on() {
        return;
    }
    let mut g = STAGE_LOG.lock().unwrap();
    let now = Instant::now();
    match g.as_mut() {
        Some((t, n, v)) => {
            if n != name {
                v.push((std::mem::take(n), now.duration_since(*t).as_secs_f64()));
                *t = now;
                *n = name.to_string();
            }
        }
        None => *g = Some((now, name.to_string(), Vec::new())),
    }
}

/// Ends the measured render and prints its stages on stderr, one line:
/// `stage times: march 1.23 | hard shadows (graphics card) 4.56 | ...`
/// (seconds; a stage reported twice is summed).
pub fn stage_times_print() {
    if !stage_times_on() {
        return;
    }
    stage_mark("");
    let Some((_, _, v)) = STAGE_LOG.lock().unwrap().take() else { return };
    let mut sums: Vec<(String, f64)> = Vec::new();
    for (n, t) in v {
        match sums.iter_mut().find(|(m, _)| *m == n) {
            Some(e) => e.1 += t,
            None => sums.push((n, t)),
        }
    }
    let parts: Vec<String> = sums.iter().filter(|(n, _)| !n.is_empty()).map(|(n, t)| format!("{n} {t:.3}")).collect();
    eprintln!("stage times: {}", parts.join(" | "));
}

/// The name of the post passes `shadows` / `occlusion`, run `where_`
/// ("" for the CPU).
pub fn post_stage_name(shadows: bool, occlusion: bool, where_: &str) -> String {
    let what = match (shadows, occlusion) {
        (true, true) => "hard shadows and ambient occlusion",
        (true, false) => "hard shadows",
        (false, true) => "ambient occlusion",
        (false, false) => "post calculations",
    };
    if where_.is_empty() {
        what.to_string()
    } else {
        format!("{what} ({where_})")
    }
}

/// The rows of a calculation in a checkpoint: what its rows hold ("raw":
/// the march; "card ...": with the card's post passes) is `tag`; rows
/// that hold something else are forgotten.
fn ck_tag(ck: Option<&Store>, tag: &str) {
    if let Some(s) = ck {
        if s.done_count() > 0 && s.value("rows").as_deref() != Some(tag) {
            s.reset();
        }
        s.set_value("rows", tag);
    }
}

/// A G-buffer of `w` x `h` with the rows of `ck` filled in (each also to
/// `sink`); and their number.
fn ck_rows(ck: Option<&Store>, w: usize, h: usize, sink: Option<RowSink>) -> (Vec<SiLight>, usize) {
    let mut gbuf = vec![SiLight::default(); w * h];
    let Some(s) = ck.filter(|s| s.done_count() > 0) else { return (gbuf, 0) };
    let mut n = 0;
    for y in 0..h {
        if s.is_done(y) {
            let row = crate::gbuffer::unpack(&s.read([0, y, w, 1]));
            gbuf[y * w..(y + 1) * w].copy_from_slice(&row);
            if let Some(f) = sink {
                f(y, &row);
            }
            n += 1;
        }
    }
    (gbuf, n)
}

fn ck_put(ck: Option<&Store>, y: usize, row: &[SiLight]) {
    if let Some(s) = ck {
        s.put(y, 0, [0, y, row.len(), 1], &crate::gbuffer::pack(row));
    }
}

fn ck_done(ck: Option<&Store>, y: usize) -> bool {
    ck.is_some_and(|s| s.is_done(y))
}

fn calculate_inner(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    post: u8,
    sink: Option<RowSink>,
    ck: Option<&Store>,
) -> Result<(CalcParams, Vec<SiLight>), String> {
    // the rows weighed first (the probe is part of the main calculation:
    // no stage of its own, the progress stays at zero meanwhile)
    let p = CalcParams::new(sc)?;
    let wm = row_weights(sc, &p, cancel);
    if cancel() {
        return Err("cancelled".into());
    }
    calculate_rows(sc, progress, stage, cancel, post, sink, ck, Some(&wm))
}

/// The calculation itself; `weights` the rows' weights for `progress`
/// (None: this is the probe of [`row_weights`]: equal weights, and the
/// card's precision is not probed either).
#[allow(clippy::too_many_arguments)]
fn calculate_rows(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    post: u8,
    sink: Option<RowSink>,
    ck: Option<&Store>,
    weights: Option<&RowWeights>,
) -> Result<(CalcParams, Vec<SiLight>), String> {
    let mut p = CalcParams::new(sc)?;
    #[cfg(feature = "gpu")]
    let is_probe = weights.is_none();
    let equal = RowWeights::equal(p.rect[3].max(0) as usize);
    let wm = weights.unwrap_or(&equal);
    let ck = ck.filter(|s| (s.spec().width, s.spec().height) == (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize));
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
        // (not for the work probe: its rows only weigh the calculation's)
        if !is_probe {
            p.gpu_double = double_probe(sc, &p, threads, !crate::gpu::auto());
        }
        // such a scene: the card marches in single precision all the same
        // and the CPU finishes its pixels in double (the surface, normal,
        // roughness and colour: what single precision gets wrong; its march
        // finds the same surfaces)
        // (the passes after the march, e.g. the reflections on the card,
        // stay in double: `double_after`)
        let finish = std::sync::atomic::AtomicBool::new(p.gpu_double && mixed_for(sc, &p));
        let double_after = finish.load(Ordering::SeqCst);
        if double_after {
            p.gpu_double = false;
        }
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
                    if is_probe || p.gpu_double || finish.load(Ordering::SeqCst) || !double_probe(sc, &p, threads, false) {
                        return None;
                    }
                    if mixed_for(sc, &p) {
                        finish.store(true, Ordering::SeqCst);
                        return None;
                    }
                    let mut pd = p.clone();
                    pd.gpu_double = true;
                    crate::gpu::march_prepare(&pd, &job).ok().map(|f| (pd, f))
                };
                let t_march = Instant::now();
                ck_tag(ck, "raw");
                // (a continued calculation with every row in its checkpoint
                // marches nothing: the card's pace against the threads' is
                // not measured, the post passes go to the card as they
                // mostly do when it joins)
                let unmeasured = ck.is_some_and(|s| s.done_count() >= h);
                let (mut gbuf, n_card, switched, card_secs) = march_mixed(&p, threads, progress, wm, cancel, sink, &flags[0], &probe, ck, true, &finish)?;
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
                let card_posts = switched.is_none()
                    && !finish.load(Ordering::SeqCst)
                    && (n_card > 0 && rate(n_card, card_secs) >= POST_ON_CARD_RATIO * rate(h - n_card, march_secs) || unmeasured && n_card == 0);
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
                    stage(&post_stage_name(job.shadows.is_some(), job.deao.is_some(), "graphics card"));
                    let t_post = Instant::now();
                    let sp = surface_progress(&gbuf, w, h, progress);
                    let r = crate::gpu::post_on_card(&p, &mut gbuf, &job, &sp, cancel);
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
                            p.gpu_post_secs = Some(t_post.elapsed().as_secs_f64());
                        }
                        Err(e) => {
                            if cancel() {
                                return Err("cancelled".into());
                            }
                            crate::gpu::set_status(format!("{s}; post calculations on the CPU: GPU error: {e}"));
                        }
                    }
                } else if n_card > 0
                    && switched.is_none()
                    && !finish.load(Ordering::SeqCst)
                    && (job.shadows.is_some() || job.deao.is_some())
                    && flags.iter().all(|f| f.load(Ordering::SeqCst))
                {
                    // the card is about as fast as the threads (or slower):
                    // they share the post passes as they shared the march
                    // (its pipelines compiled: else the CPU does them alone,
                    // as the card would only wait for its compile)
                    let s = crate::gpu::last_status();
                    stage(&post_stage_name(job.shadows.is_some(), job.deao.is_some(), "graphics card and CPU"));
                    let t_post = Instant::now();
                    let card_rows = post_shared(sc, &p, &mut gbuf, &job, threads, progress, cancel)?;
                    if timing {
                        eprintln!("  shadows and DE occlusion shared: {:.2} s, {card_rows} of {h} rows on the card", t_post.elapsed().as_secs_f64());
                    }
                    if card_rows > 0 {
                        let what = match (job.shadows.is_some(), job.deao.is_some()) {
                            (true, true) => "shadows and AO",
                            (true, false) => "shadows",
                            _ => "AO",
                        };
                        crate::gpu::set_status(format!("{s} +{what} ({} % on the card)", card_rows * 100 / h.max(1)));
                    }
                    p.gpu_post = Some(job);
                    p.gpu_post_secs = Some(t_post.elapsed().as_secs_f64());
                }
                p.gpu_double |= double_after;
                return finish_calculation(sc, p, gbuf, post, threads, progress, stage, cancel);
            }
        }
        let card_tag = format!("card {job:?}");
        if finish.load(Ordering::SeqCst) {
            // the card marches all rows, the threads finish them (and the
            // post passes run on the CPU, as after a march in double)
            if let Ok(flags) = crate::gpu::march_prepare(&p, &PostJob::default()) {
                ck_tag(ck, "mixed");
                let (gbuf, n_card, ..) = march_mixed(&p, threads, progress, wm, cancel, sink, &flags[0], &|| None, ck, false, &finish)?;
                if n_card > 0 || ck.is_some_and(|s| s.done_count() == h) {
                    p.gpu_double |= double_after;
                    return finish_calculation(sc, p, gbuf, post, threads, progress, stage, cancel);
                }
            }
            // (the card failed: double precision as before)
            p.gpu_double = true;
        }
        if let Some(s) = ck.filter(|s| s.done_count() > 0 && s.value("rows").is_some_and(|t| t == "raw" || t == card_tag)) {
            // continued: the card marches the rows the checkpoint lacks
            // (with its post passes as before, or without them, and then
            // they run on the whole picture)
            let with_post = s.value("rows").as_deref() == Some(card_tag.as_str());
            let none = PostJob::default();
            if let Some(gbuf) = march_card_rest(&p, if with_post { &job } else { &none }, h, progress, wm, cancel, sink, Some(s), true)? {
                let mut gbuf = gbuf;
                if with_post {
                    p.gpu_post = Some(job);
                } else if job.shadows.is_some() || job.deao.is_some() {
                    stage(&post_stage_name(job.shadows.is_some(), job.deao.is_some(), "graphics card"));
                    let t_post = Instant::now();
                    let sp = surface_progress(&gbuf, w, h, progress);
                    if crate::gpu::post_on_card(&p, &mut gbuf, &job, &sp, cancel).is_ok() {
                        p.gpu_post = Some(job);
                        p.gpu_post_secs = Some(t_post.elapsed().as_secs_f64());
                    }
                }
                if cancel() {
                    return Err("cancelled".into());
                }
                return finish_calculation(sc, p, gbuf, post, threads, progress, stage, cancel);
            }
            // (the card failed: all on the CPU, from the start)
            s.reset();
        } else {
            ck_tag(ck, &card_tag);
            let gpu = if crate::gpu::extra_cards(&p).is_empty() {
                // (the card's rows come in order from the top)
                crate::gpu::march(&p, &job, &|d, _| progress(wm.before(d), wm.total()), cancel, |y, row| {
                    if let Some(f) = sink {
                        f(y, row);
                    }
                    ck_put(ck, y, row);
                })
            } else {
                // all the cards of the computer, taking bands of the rows
                march_card_rest(&p, &job, h, progress, wm, cancel, sink, ck, false)?
            };
            if let Some(gbuf) = gpu {
                p.gpu_post = Some(job);
                return finish_calculation(sc, p, gbuf, post, threads, progress, stage, cancel);
            }
            // (stopped, not failed: the card's rows stay in the checkpoint,
            // unless too many of their pixels overflowed single precision)
            if cancel() || crate::gpu::aborted() {
                if crate::gpu::rows_overflowed() {
                    ck_tag(ck, "raw");
                }
                return Err("cancelled".into());
            }
        }
    }
    let p = p;
    ck_tag(ck, "raw");
    // the rows go straight into the G-buffer (no copy of it: big pictures)
    let (mut gbuf, _) = ck_rows(ck, w, h, sink);
    let done = AtomicUsize::new(wm.done_in(ck));
    std::thread::scope(|s| {
        // thread t takes the rows t, t + threads, ...
        let mut per: Vec<Vec<(usize, &mut [SiLight])>> = (0..threads).map(|_| Vec::new()).collect();
        for (y, row) in gbuf.chunks_mut(w.max(1)).enumerate() {
            per[y % threads].push((y, row));
        }
        for list in per {
            let p = &p;
            let done = &done;
            s.spawn(move || {
                // MB3D: seed := Round(Random * (iThreadId + 1) * $324594A1 + $24563487)
                for (y, row) in list {
                    if cancel() {
                        break;
                    }
                    if ck_done(ck, y) {
                        continue;
                    }
                    let yy = y + y0;
                    let mut m = Marcher::new(p, 0);
                    for (k, px) in row.iter_mut().enumerate() {
                        let x = x0 + k;
                        *px = if two_d {
                            m.slice_pixel(x as i32, yy as i32)
                        } else {
                            m.seed = crate::calc::pixel_seed(yy as i64, x as i64);
                            m.march_pixel(x as i32, yy as i32)
                        };
                    }
                    if let Some(f) = sink {
                        f(y, row);
                    }
                    ck_put(ck, y, row);
                    let d = done.fetch_add(wm.weight(y), Ordering::Relaxed) + wm.weight(y);
                    progress(d, wm.total());
                }
            });
        }
    });
    if cancel() {
        return Err("cancelled".into());
    }
    finish_calculation(sc, p, gbuf, post, threads, progress, stage, cancel)
}

/// `progress` for a pass over the G-buffer `g` (`w` x `h`) whose rows
/// come in order from the top and that counts them: weighed by their
/// surface pixels ([`RowWeights::surface`]).
#[cfg(feature = "gpu")]
fn surface_progress<'a>(g: &[SiLight], w: usize, h: usize, progress: &'a (dyn Fn(usize, usize) + Sync)) -> impl Fn(usize, usize) + Sync + 'a {
    let sw = RowWeights::surface(g, w, h);
    move |d, _| progress(sw.before(d), sw.total())
}

/// "Calc-" (MB3D's `CalcMandBlocky`): a rough image of the whole view, one
/// pixel calculated for each 8x8 block (its pixel 3, 3) and copied to the
/// block, on the CPU, without the post calculations.  `rows` gets every
/// finished band of 8 rows.
pub fn calculate_blocky(
    sc: &Scene,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    rows: RowSink,
) -> Result<(CalcParams, Vec<SiLight>), String> {
    let mut sc = sc.clone();
    sc.stereo_mode = 0;
    let p = CalcParams::new(&sc)?;
    let [x0, y0, w, h] = p.rect;
    let (w, h) = (w as usize, h as usize);
    let bands = h.div_ceil(8);
    let threads = thread_count(&sc).min(bands).max(1);
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let mut gbuf = vec![SiLight::default(); w * h];
    let out: Vec<std::sync::Mutex<&mut [SiLight]>> = gbuf.chunks_mut((8 * w).max(1)).map(std::sync::Mutex::new).collect();
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut m = Marcher::new(&p, 0);
                loop {
                    let b = next.fetch_add(1, Ordering::Relaxed);
                    if b >= bands || cancel() {
                        break;
                    }
                    let by0 = b * 8;
                    let y = (by0 + 3).min(h - 1) as i32 + y0;
                    let mut band = out[b].lock().unwrap();
                    for bx0 in (0..w).step_by(8) {
                        let x = (bx0 + 3).min(w - 1) as i32 + x0;
                        m.seed = crate::calc::pixel_seed(y as i64, x as i64);
                        let si = m.march_pixel(x, y);
                        for row in band.chunks_mut(w) {
                            row[bx0..(bx0 + 8).min(w)].fill(si);
                        }
                    }
                    for (k, row) in band.chunks(w).enumerate() {
                        rows(by0 + k, row);
                    }
                    drop(band);
                    progress(done.fetch_add(1, Ordering::Relaxed) + 1, bands);
                }
            });
        }
    });
    drop(out);
    if cancel() {
        return Err("cancelled".into());
    }
    Ok((p, gbuf))
}

/// Scenes that single precision is too coarse for: the card's march in
/// single precision finished on the CPU in double (`finish_row`), not the
/// card in double.  (MB3D_GPU_MIXED=0: in double, as before)
#[cfg(feature = "gpu")]
fn mixed_precision() -> bool {
    std::env::var("MB3D_GPU_MIXED").map_or(true, |v| v != "0")
}

/// A row of the card's single precision march finished on the CPU in
/// double precision ([`Marcher::finish_pixel`]).
#[cfg(feature = "gpu")]
fn finish_row(p: &CalcParams, y: usize, card: &[SiLight]) -> Vec<SiLight> {
    let [x0, y0, ..] = p.rect;
    let yy = y + y0 as usize;
    let mut m = Marcher::new(p, 0);
    card.iter()
        .enumerate()
        .map(|(k, si)| {
            let x = x0 as usize + k;
            m.seed = crate::calc::pixel_seed(yy as i64, x as i64);
            m.finish_pixel(x as i32, yy as i32, si)
        })
        .collect()
}

/// One row of the main calculation on the CPU (MB3D's per-row seed).
#[cfg(feature = "gpu")]
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

/// The post passes of `job` (hard or soft shadows, DE ambient occlusion)
/// shared by the card and the CPU threads, as [`march_mixed`] shares the
/// march: the card takes bands of rows from the top (`gpu::post_bands`),
/// the threads rows from the bottom, both passes of a row on one side; the
/// rows of a band the card failed are done here.  So the slower of the two
/// takes fewer rows, and neither waits.  The rows the card did.
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
fn post_shared(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &mut [SiLight],
    job: &PostJob,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<usize, String> {
    use std::sync::Mutex;
    let (w, h) = (p.rect[2].max(1) as usize, p.rect[3].max(0) as usize);
    // rows finished by either side, for `progress` (weighed by their
    // surface pixels: the passes work on those)
    let sw = RowWeights::surface(gbuf, w, h);
    let finished = AtomicUsize::new(0);
    let tick = |y: usize| progress(finished.fetch_add(sw.weight(y), Ordering::Relaxed) + sw.weight(y), sw.total());
    let shadows = job.shadows.as_ref().and_then(|_| ShadowRows::new(sc, p));
    let deao = match (&job.deao, &sc.deao) {
        (Some(_), Some(d)) if sc.ao.is_some() => Some(crate::deao::DeaoRows::new(p, d)),
        _ => None,
    };
    // the CPU's passes of a row
    let cpu = |y: usize, row: &mut [SiLight]| {
        if let Some(s) = &shadows {
            s.row(y, row);
        }
        if let Some(d) = &deao {
            d.row(y, row);
        }
    };
    let rows: Vec<Mutex<&mut [SiLight]>> = gbuf.chunks_mut(w).map(Mutex::new).collect();
    // the rows still free: [top, bottom)
    let free = Mutex::new((0usize, h));
    let claimed = Mutex::new(Vec::new());
    // which rows are where (see `Board`), and how many the cards did
    let board = Board::new(h);
    let card_rows = AtomicUsize::new(0);
    let mut card_err = None;
    let left = || {
        let f = free.lock().unwrap();
        f.0 < f.1
    };
    // a band of rows for card `k` (smaller as the rows run out, so that
    // all end together; then rows another card took and has not
    // delivered), the G-buffer rows it needs, a row it finished (the
    // first result of the row)
    let band = |k: usize, band: usize| {
        let mut f = free.lock().unwrap();
        let r = if f.0 < f.1 {
            let r = (f.0, band.min(((f.1 - f.0) / 4).max(16)).min(f.1 - f.0));
            f.0 += r.1;
            board.took(k, r);
            Some(r)
        } else {
            drop(f);
            board.steal(Some(k), band)
        };
        if let Some(r) = r {
            claimed.lock().unwrap().push(r);
        }
        r
    };
    let upload = |row0: usize, n: usize| (row0..(row0 + n).min(h)).flat_map(|y| rows[y].lock().unwrap().to_vec()).collect();
    let done_row = |y: usize, row: &[SiLight]| {
        if y < h && board.deliver(y) {
            let mut dst = rows[y].lock().unwrap();
            let n = row.len().min(dst.len());
            dst[..n].copy_from_slice(&row[..n]);
            drop(dst);
            card_rows.fetch_add(1, Ordering::Relaxed);
            tick(y);
        }
    };
    // card `k` stops once the free rows are gone and all of its rows came
    let stop = |k: usize| cancel() || !left() && board.done_for(k);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                if cancel() {
                    break;
                }
                let y = {
                    let mut f = free.lock().unwrap();
                    (f.1 > f.0).then(|| {
                        f.1 -= 1;
                        f.1
                    })
                };
                match y {
                    Some(y) => {
                        board.set_delivered(y);
                        cpu(y, &mut rows[y].lock().unwrap());
                        tick(y);
                    }
                    // a row a card took and has not delivered: on a copy,
                    // kept when it comes first
                    None => match board.steal(None, 1) {
                        Some((y, _)) => {
                            let mut row = rows[y].lock().unwrap().to_vec();
                            cpu(y, &mut row);
                            if board.deliver(y) {
                                rows[y].lock().unwrap().copy_from_slice(&row);
                                tick(y);
                            }
                        }
                        None => break,
                    },
                }
            });
        }
        // the other cards of the computer too, once their pipelines are
        // compiled (what one of them took and did not deliver is done
        // below, as the first card's)
        for k in crate::gpu::extra_cards(p) {
            let (band, upload, done_row, stop, left, board) = (&band, &upload, &done_row, &stop, &left, &board);
            s.spawn(move || {
                crate::gpu::on_card(k, || {
                    let Ok(flags) = crate::gpu::march_prepare(p, job) else { return };
                    if compiled_in_time(&flags, left, cancel) {
                        let _ = board.keep_going(cancel, || crate::gpu::post_bands(p, job, &|n| band(k, n), upload, &|| stop(k), &mut |y, row| done_row(y, row)));
                    }
                })
            });
        }
        let r = board.keep_going(cancel, || crate::gpu::post_bands(p, job, &|n| band(0, n), &upload, &|| stop(0), &mut |y, row| done_row(y, row)));
        if let Err(e) = r.or_else(|e| if e == "cancelled" && !cancel() { Ok(()) } else { Err(e) }) {
            card_err = Some(e);
        }
    });
    if cancel() {
        return Err("cancelled".into());
    }
    // the rows the card took and did not deliver (it failed): here
    if let Some(e) = &card_err {
        eprintln!("shadows and occlusion on the GPU failed, calculated on the CPU: {e}");
    }
    let redo: Vec<usize> = claimed.into_inner().unwrap().into_iter().flat_map(|(r0, n)| r0..r0 + n).filter(|&y| y < h && !board.delivered[y].load(Ordering::Relaxed)).collect();
    if !redo.is_empty() {
        let next = AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..threads.max(1) {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= redo.len() || cancel() {
                        break;
                    }
                    cpu(redo[i], &mut rows[redo[i]].lock().unwrap());
                });
            }
        });
        if cancel() {
            return Err("cancelled".into());
        }
    }
    Ok(card_rows.into_inner())
}

/// The probe's limit: the mean difference of the colours (a share of full
/// scale) above which single precision is too coarse.
#[cfg(feature = "gpu")]
fn probe_limit() -> f64 {
    std::env::var("MB3D_PROBE_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(PROBE_LIMIT)
}

/// The probe's decisions by scene ([`crate::gpu::scene_key`]).
#[cfg(feature = "gpu")]
static PROBED: std::sync::Mutex<Vec<(u64, bool, bool)>> = std::sync::Mutex::new(Vec::new());

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
    if let Some(&(_, d, _)) = PROBED.lock().unwrap().iter().find(|(k, ..)| *k == key) {
        return d;
    }
    let ready = if compile {
        match crate::gpu::march_prepare(p, &PostJob::default()) {
            Ok(flags) => {
                while !flags[0].load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                // (the card failed in the compile: no probe)
                crate::gpu::enabled()
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
    let limit = probe_limit();
    let d = f.is_some_and(|f| f.single > limit);
    // (the card's march finished on the CPU is close enough: `finish_row`)
    let mixed = d && f.is_some_and(|f| f.mixed <= limit);
    if std::env::var_os("MB3D_PROBE_DEBUG").is_some() {
        match f {
            Some(f) => eprintln!(
                "  probe: {} pixels, colours differ by {:.2} % in single precision, {:.2} % finished in double ({:.0} ms): {}",
                f.pixels,
                f.single * 100.0,
                f.mixed * 100.0,
                t.elapsed().as_secs_f64() * 1000.0,
                if mixed { "single, finished in double" } else if d { "double" } else { "single" }
            ),
            None => eprintln!("  probe: no result ({:.0} ms)", t.elapsed().as_secs_f64() * 1000.0),
        }
    }
    let mut list = PROBED.lock().unwrap();
    if list.len() >= 256 {
        list.remove(0);
    }
    list.push((key, d, mixed));
    d
}

/// For a scene [`double_probe`] sent to double precision: whether the
/// card marches it in single precision, finished on the CPU in double
/// (`finish_row`): when its probe found that close enough (as single
/// precision is for other scenes), and the scene has no reflections (their
/// rays from the slightly other points within the depth's last digit
/// sparkle differently on fine detail: *Blood on the dancefloor* differed
/// from the CPU's picture by 1.8 % on average, more than any scene in
/// single precision does).
#[cfg(feature = "gpu")]
fn mixed_for(sc: &Scene, p: &CalcParams) -> bool {
    mixed_precision() && !sc.mc.reflections && mixed_ok(sc, p)
}

/// The probe of [`double_probe`] found the card's march finished on the CPU
/// in double precision close enough.
#[cfg(feature = "gpu")]
fn mixed_ok(sc: &Scene, p: &CalcParams) -> bool {
    // (the key of the probe's parameters: in single precision)
    let mut ps = p.clone();
    ps.gpu_double = false;
    let Some(key) = crate::gpu::scene_key(&ps) else { return false };
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (key, format!("{:?}", effective_lighting(sc))).hash(&mut h);
        h.finish()
    };
    PROBED.lock().unwrap().iter().find(|(k, ..)| *k == key).is_some_and(|&(_, _, m)| m)
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
pub fn probe_single(sc: &Scene, p: &CalcParams, threads: usize) -> Option<Probe> {
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
                // the probe does not count in the calculation's statistics
                crate::calc::statistics_off(true);
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
    // (painted lit: the soft shadow's light in full, no hard shadows)
    let lit = |si: &SiLight| SiLight { shadow: if soft { 0xFC00 } else { 0 }, ..*si };
    let colour = |si: &SiLight, x: usize, y: usize| lv.pixel_color(&lit(si), (x + x0) as i32, (y + y0) as i32, &cam);
    let off = |u: &[u8; 3], v: &[u8; 3]| u.iter().zip(v).map(|(&u, &v)| (u as f64 - v as f64).abs()).sum::<f64>() / (3.0 * 255.0);
    let mut n = [0usize; 6];
    let (mut total, mut diff) = (0usize, 0f64);
    for (&(y, x), c) in samples.iter().zip(&cpu) {
        let (Some(b), Some(a)) = (c.get(), card.iter().find(|(yy, _)| *yy == y).and_then(|(_, r)| r.get(x / sxy))) else { continue };
        total += 1;
        diff += off(&colour(a, x, y), &colour(b, x, y));
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
    if total < 100 {
        return None;
    }
    let single = diff / total as f64;
    if single <= probe_limit() {
        return Some(Probe { single, mixed: single, pixels: total });
    }
    // the card's pixels finished here in double precision (`finish_row`)
    let fin: Vec<OnceLock<SiLight>> = (0..ns).map(|_| OnceLock::new()).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                crate::calc::statistics_off(true);
                loop {
                    let j = next.fetch_add(1, Ordering::Relaxed);
                    if j >= ns {
                        break;
                    }
                    let (y, x) = samples[j];
                    let Some(a) = card.iter().find(|(yy, _)| *yy == y).and_then(|(_, r)| r.get(x / sxy)) else { continue };
                    let (xx, yy) = ((x + x0) as i32, (y + y0) as i32);
                    let mut m = Marcher::new(p, crate::calc::pixel_seed(yy as i64, xx as i64));
                    let _ = fin[j].set(m.finish_pixel(xx, yy, a));
                }
            });
        }
    });
    let mut diff_mixed = 0f64;
    for ((&(y, x), c), f) in samples.iter().zip(&cpu).zip(&fin) {
        if let (Some(b), Some(f)) = (c.get(), f.get()) {
            diff_mixed += off(&colour(f, x, y), &colour(b, x, y));
        }
    }
    Some(Probe { single, mixed: diff_mixed / total as f64, pixels: total })
}

/// What [`probe_single`] found: the mean differences of the colours of the
/// card's single precision march, and of it finished on the CPU in double
/// precision (`finish_row`), against the CPU's (shares of full scale; the
/// latter only where the former is over the probe's limit, else the same);
/// the pixels compared.
#[cfg(feature = "gpu")]
#[derive(Clone, Copy, Debug)]
pub struct Probe {
    pub single: f64,
    pub mixed: f64,
    pub pixels: usize,
}

/// How much faster than the CPU threads the card must march for Auto to give
/// it the shadows and the DE occlusion (see `calculate_inner`).
#[cfg(feature = "gpu")]
const POST_ON_CARD_RATIO: f64 = 1.5;

/// The rows of a picture shared out to the cards (and the CPU's threads):
/// the bands each card took, which rows are delivered (the first result of
/// a row counts, a later one is dropped) and which have a second worker on
/// them.  Once the free rows are gone, a worker without work takes rows a
/// card took and has not delivered yet, so that a slow card no longer
/// holds up the picture; a card whose rows all came from others stops.
#[cfg(feature = "gpu")]
struct Board {
    delivered: Vec<std::sync::atomic::AtomicBool>,
    backup: Vec<std::sync::atomic::AtomicBool>,
    /// (card, first row, rows)
    bands: std::sync::Mutex<Vec<(usize, usize, usize)>>,
}

#[cfg(feature = "gpu")]
impl Board {
    fn new(h: usize) -> Self {
        let flags = || (0..h).map(|_| std::sync::atomic::AtomicBool::new(false)).collect();
        Board { delivered: flags(), backup: flags(), bands: Default::default() }
    }

    /// `card` took the band `r` (first row, rows).
    fn took(&self, card: usize, r: (usize, usize)) {
        self.bands.lock().unwrap().push((card, r.0, r.1));
    }

    /// Row `y` arrived: true for its first result (that one counts).
    fn deliver(&self, y: usize) -> bool {
        !self.delivered[y].swap(true, Ordering::SeqCst)
    }

    /// Row `y` is delivered (also a result that replaces the first).
    fn set_delivered(&self, y: usize) {
        self.delivered[y].store(true, Ordering::SeqCst);
    }

    /// Up to `max` rows in a row that another card took and has not
    /// delivered, nobody else backing them up: now also `card`'s (noted as
    /// its band; `card` None: a CPU thread, one row).
    fn steal(&self, card: Option<usize>, max: usize) -> Option<(usize, usize)> {
        let bands = self.bands.lock().unwrap().clone();
        let open = |y: usize| !self.delivered[y].load(Ordering::SeqCst) && !self.backup[y].load(Ordering::SeqCst);
        for (k, r0, n) in bands {
            if Some(k) == card {
                continue;
            }
            let Some(y) = (r0..r0 + n).find(|&y| open(y)) else { continue };
            let mut m = 0;
            while y + m < r0 + n && m < max.max(1) && open(y + m) && !self.backup[y + m].swap(true, Ordering::SeqCst) {
                m += 1;
            }
            if m > 0 {
                if let Some(c) = card {
                    self.took(c, (y, m));
                }
                return Some((y, m));
            }
        }
        None
    }

    /// Some row is neither delivered nor backed up (work for a card).
    fn has_open(&self) -> bool {
        (0..self.delivered.len()).any(|y| !self.delivered[y].load(Ordering::SeqCst) && !self.backup[y].load(Ordering::SeqCst))
    }

    /// A card's part of a pass (`go`: its march over bands): when it was
    /// stopped (its rows all came from others) while rows are still open,
    /// it goes on with those instead of waiting idle for the pass's end.
    fn keep_going(&self, cancel: &(dyn Fn() -> bool + Sync), mut go: impl FnMut() -> Result<(), String>) -> Result<(), String> {
        loop {
            let r = go();
            if r.as_ref().is_err_and(|e| e == "cancelled") && !cancel() && self.has_open() {
                continue;
            }
            return r;
        }
    }

    /// All the rows `card` took are delivered (by it or by others).
    fn done_for(&self, card: usize) -> bool {
        self.bands.lock().unwrap().iter().filter(|b| b.0 == card).all(|&(_, r0, n)| (r0..r0 + n).all(|y| self.delivered[y].load(Ordering::SeqCst)))
    }
}

/// Whether the pipelines of `flags` (compiling in the background on a
/// further card) are ready before the rows run out (`left`): the card
/// joins then; else it stays out of this picture and its compile goes on
/// for the next (a picture does not wait for a slow compile).
#[cfg(feature = "gpu")]
fn compiled_in_time(flags: &[std::sync::Arc<std::sync::atomic::AtomicBool>], left: &dyn Fn() -> bool, cancel: &(dyn Fn() -> bool + Sync)) -> bool {
    loop {
        if flags.iter().all(|f| f.load(Ordering::SeqCst)) {
            return true;
        }
        if !left() || cancel() {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// A card's march that ended because others delivered its rows ("cancelled"
/// without a Stop) as an ended one; its rows are unusable all the same when
/// they overflowed single precision.
#[cfg(feature = "gpu")]
fn stopped_as_done(r: Result<(), String>, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    match r {
        Err(e) if e == "cancelled" && !cancel() => {
            if crate::gpu::rows_overflowed() {
                Err("single precision overflows".into())
            } else {
                Ok(())
            }
        }
        r => r,
    }
}

/// The march with the CPU threads taking rows from the bottom at once and
/// the card from the top once `ready` (its shader compiled) - until they
/// meet.  Returns the G-buffer and the number of rows the card marched
/// (0: it never joined, or its rows were redone on the CPU).
#[cfg(feature = "gpu")]
fn march_mixed(
    p: &CalcParams,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    wm: &RowWeights,
    cancel: &(dyn Fn() -> bool + Sync),
    sink: Option<RowSink>,
    ready: &std::sync::atomic::AtomicBool,
    on_ready: &(dyn Fn() -> Option<(CalcParams, Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>)> + Sync),
    ck: Option<&Store>,
    cpu_rows: bool,
    finish: &std::sync::atomic::AtomicBool,
) -> Result<(Vec<SiLight>, usize, Option<(CalcParams, Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>)>, f64), String> {
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;
    // `finish`: the card marches in single precision and its rows are
    // finished here in double (`finish_row`), the threads take them before
    // rows of their own (`cpu_rows`: else they only finish the card's)
    let queue: Mutex<std::collections::VecDeque<(usize, Vec<SiLight>)>> = Mutex::new(std::collections::VecDeque::new());
    let card_done = AtomicBool::new(false);
    // how long the card marched (its rows per second against the CPU's)
    let card_secs = Mutex::new(0.0f64);
    let h = p.rect[3].max(0) as usize;
    // the rows still free: [top, bottom)
    let free = Mutex::new((0usize, h));
    // the rows go straight into the G-buffer (no copy of it: big pictures)
    let w = p.rect[2].max(1) as usize;
    // (the rows of a checkpoint are there: the free rows skip them)
    let (mut gbuf, had) = ck_rows(ck, w, h, sink);
    let rows: Vec<Mutex<&mut [SiLight]>> = gbuf.chunks_mut(w).map(Mutex::new).collect();
    let done = AtomicUsize::new(wm.done_in(ck));
    let on_card = AtomicUsize::new(0);
    CARD_ROWS.store(0, Ordering::Relaxed);
    // the card's rows go to the checkpoint after its verdict on them (see
    // `gpu::rows_overflowed`): they wait here
    let pending = Mutex::new(Vec::new());
    // which rows are where (see `Board`)
    let board = Board::new(h);
    // (the callers decide whether the row counts: `Board::deliver`)
    let put = |y: usize, row: &[SiLight], card: bool| {
        if let Some(f) = sink {
            f(y, row);
        }
        if card {
            pending.lock().unwrap().push(y);
        } else {
            ck_put(ck, y, row);
        }
        let mut dst = rows[y].lock().unwrap();
        let n = row.len().min(dst.len());
        dst[..n].copy_from_slice(&row[..n]);
        drop(dst);
        let d = done.fetch_add(wm.weight(y), Ordering::Relaxed) + wm.weight(y);
        progress(d.min(wm.total()), wm.total());
    };
    let mut card_err = None;
    let mut switched = None;
    let mut card_ok = false;
    // the first card's choice of precision, for the others (see below)
    let decided: Mutex<Option<Option<CalcParams>>> = Mutex::new(None);
    let left = || {
        let f = free.lock().unwrap();
        f.0 < f.1
    };
    // a band of free rows for card `k` (up to the next row of the
    // checkpoint), noted in `claimed`; once they are gone, rows another
    // card took and has not delivered
    let claim = |k: usize, band: usize, claimed: &Mutex<Vec<(usize, usize)>>| {
        let mut f = free.lock().unwrap();
        while f.0 < f.1 && ck_done(ck, f.0) {
            f.0 += 1;
        }
        let r = if f.0 < f.1 {
            let n = (f.0..f.1).take(band).take_while(|&y| !ck_done(ck, y)).count();
            let r = (f.0, n);
            f.0 += r.1;
            board.took(k, r);
            Some(r)
        } else {
            drop(f);
            board.steal(Some(k), band)
        };
        if let Some(r) = r {
            claimed.lock().unwrap().push(r);
        }
        r
    };
    // card `k` stops once the free rows are gone and all of its rows came
    let stop = |k: usize| cancel() || !left() && board.done_for(k);
    // a row a card marched (the first result of the row)
    let card_row = |y: usize, row: &[SiLight]| {
        if !board.deliver(y) {
            return;
        }
        on_card.fetch_add(1, Ordering::Relaxed);
        CARD_ROWS.fetch_add(1, Ordering::Relaxed);
        if finish.load(Ordering::SeqCst) {
            queue.lock().unwrap().push_back((y, row.to_vec()));
        } else {
            put(y, row, true);
        }
    };
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
                if cancel() {
                    break;
                }
                let card_row = queue.lock().unwrap().pop_front();
                if let Some((y, row)) = card_row {
                    put(y, &finish_row(p, y, &row), true);
                    continue;
                }
                // (rows a card took and has not delivered: see `Board`)
                let stolen = || if cpu_rows && !left() { board.steal(None, 1).map(|r| r.0) } else { None };
                let y = if cpu_rows {
                    let mut f = free.lock().unwrap();
                    while f.1 > f.0 && ck_done(ck, f.1 - 1) {
                        f.1 -= 1;
                    }
                    (f.1 > f.0).then(|| {
                        f.1 -= 1;
                        f.1
                    })
                } else {
                    None
                };
                match y {
                    Some(y) => {
                        board.set_delivered(y);
                        put(y, &cpu_row(p, y), false)
                    }
                    None => match stolen() {
                        Some(y) => {
                            let row = cpu_row(p, y);
                            if board.deliver(y) {
                                put(y, &row, false);
                            }
                        }
                        // (rows of the card still to come)
                        None if finish.load(Ordering::SeqCst) && !card_done.load(Ordering::SeqCst) => std::thread::sleep(std::time::Duration::from_micros(500)),
                        None => break,
                    },
                }
            });
        }
        // the other cards of the computer take bands from the same rows,
        // once the first card has chosen the precision (the parameters in
        // double precision, or None: as they are)
        for k in crate::gpu::extra_cards(p) {
            let (decided, claim, card_row, free, stop, board, left) = (&decided, &claim, &card_row, &free, &stop, &board, &left);
            s.spawn(move || {
                crate::gpu::on_card(k, || {
                    let cp = loop {
                        let left = {
                            let f = free.lock().unwrap();
                            f.0 < f.1
                        };
                        if !left || cancel() {
                            return;
                        }
                        if let Some(d) = decided.lock().unwrap().clone() {
                            break d;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    };
                    let cp = cp.as_ref().unwrap_or(p);
                    let Ok(flags) = crate::gpu::march_prepare(cp, &PostJob::default()) else { return };
                    if !compiled_in_time(&flags, &left, cancel) {
                        return;
                    }
                    let claimed = Mutex::new(Vec::new());
                    let mine = AtomicUsize::new(0);
                    let r = board.keep_going(cancel, || {
                        crate::gpu::march_bands(cp, &PostJob::default(), &|band| claim(k, band, &claimed), &|| stop(k), &mut |y, row| {
                            mine.fetch_add(1, Ordering::Relaxed);
                            card_row(y, row)
                        })
                    });
                    let r = stopped_as_done(r, cancel);
                    // (MB3D_CARDS_DEBUG: what each of the other cards did)
                    if std::env::var_os("MB3D_CARDS_DEBUG").is_some() {
                        eprintln!("card {}: {} rows marched ({r:?})", k + 1, mine.load(Ordering::Relaxed));
                    }
                    // failed (or its rows overflowed single precision):
                    // what it took, on the CPU here
                    if r.as_ref().is_err_and(|e| e != "cancelled") || crate::gpu::rows_overflowed() {
                        for (r0, n) in claimed.into_inner().unwrap() {
                            for y in r0..r0 + n {
                                if cancel() {
                                    return;
                                }
                                board.set_delivered(y);
                                put(y, &cpu_row(p, y), false);
                            }
                        }
                    }
                });
            });
        }
        // the card, once its shader is ready
        let mut run_card = || {
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
        *decided.lock().unwrap() = Some(switched.as_ref().map(|(pd, _)| pd.clone()));
        let claimed = Mutex::new(Vec::new());
        let t_card = Instant::now();
        let r = board.keep_going(cancel, || crate::gpu::march_bands(cp, &PostJob::default(), &|band| claim(0, band, &claimed), &|| stop(0), &mut |y, row| card_row(y, row)));
        let r = stopped_as_done(r, cancel);
        *card_secs.lock().unwrap() = t_card.elapsed().as_secs_f64();
        // (stopped: its rows are kept when the card's verdict on them so
        // far is good)
        card_ok = r.is_ok() || (r.as_ref().is_err_and(|e| e == "cancelled") && !crate::gpu::rows_overflowed());
        if let Err(e) = r {
            if e != "cancelled" {
                // the card's rows (also those it delivered: overflows) on
                // the CPU, by all threads
                queue.lock().unwrap().clear();
                let redo: Vec<usize> = claimed.into_inner().unwrap().into_iter().flat_map(|(r0, n)| r0..r0 + n).collect();
                let next = AtomicUsize::new(0);
                std::thread::scope(|s2| {
                    for _ in 0..threads {
                        s2.spawn(|| loop {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            if i >= redo.len() || cancel() {
                                break;
                            }
                            put(redo[i], &cpu_row(p, redo[i]), false);
                        });
                    }
                });
                on_card.store(0, Ordering::Relaxed);
                CARD_ROWS.store(0, Ordering::Relaxed);
            }
            card_err = Some(e);
        }
        };
        run_card();
        card_done.store(true, Ordering::SeqCst);
    });
    // the card's rows the threads did not get to
    let left: Vec<(usize, Vec<SiLight>)> = queue.into_inner().unwrap().into();
    if !left.is_empty() && !cancel() {
        let next = AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= left.len() || cancel() {
                        break;
                    }
                    put(left[i].0, &finish_row(p, left[i].0, &left[i].1), true);
                });
            }
        });
    }
    if card_ok {
        for y in pending.into_inner().unwrap() {
            ck_put(ck, y, &rows[y].lock().unwrap());
        }
    }
    if cancel() {
        return Err("cancelled".into());
    }
    let n_card = on_card.load(Ordering::Relaxed);
    // (of the rows calculated now: not those of a checkpoint)
    let pct = n_card * 100 / (h - had).max(1);
    crate::gpu::set_status(match card_err {
        Some(e) => format!("CPU: GPU error: {e}"),
        None if n_card == 0 && switched.is_some() => "CPU (the graphics card's double precision shader was still compiling)".into(),
        None if n_card == 0 => "CPU (the graphics card's shader was still compiling)".into(),
        None if switched.is_some() => format!("GPU {name}: {pct} % of the rows on the card (double precision), {} % on the CPU", 100 - pct),
        None if finish.load(Ordering::SeqCst) => format!("GPU {name}: {pct} % of the rows on the card (finished on the CPU in double precision), {} % on the CPU", 100 - pct),
        None => format!("GPU {name}: {pct} % of the rows on the card, {} % on the CPU", 100 - pct),
    });
    // (the card's parameters when it joined in double precision)
    let switched = switched.filter(|_| n_card > 0);
    drop(rows);
    Ok((gbuf, n_card, switched, card_secs.into_inner().unwrap()))
}

/// The rows that `ck` lacks (all without one) marched on the cards (with
/// the post passes of `post`), into a G-buffer with the rows of `ck`: all
/// cards of the computer take bands of the rows, what a further card took
/// and could not do goes to the first.  None when the first card cannot
/// march them.
#[cfg(feature = "gpu")]
#[allow(clippy::too_many_arguments)]
fn march_card_rest(p: &CalcParams, post: &PostJob, h: usize, progress: &(dyn Fn(usize, usize) + Sync), wm: &RowWeights, cancel: &(dyn Fn() -> bool + Sync), sink: Option<RowSink>, ck: Option<&Store>, continued: bool) -> Result<Option<Vec<SiLight>>, String> {
    use std::sync::Mutex;
    let w = p.rect[2].max(1) as usize;
    let (mut gbuf, _) = ck_rows(ck, w, h, sink);
    let rows: Vec<Mutex<&mut [SiLight]>> = gbuf.chunks_mut(w).map(Mutex::new).collect();
    let next = Mutex::new(0usize);
    let done = AtomicUsize::new(wm.done_in(ck));
    progress(done.load(Ordering::Relaxed), wm.total());
    // which rows are where (see `Board`)
    let board = Board::new(h);
    let left = || {
        let mut y = next.lock().unwrap();
        while *y < h && ck_done(ck, *y) {
            *y += 1;
        }
        *y < h
    };
    // a band for card `k`: free rows, then rows another card took and has
    // not delivered
    let band = |k: usize, band: usize| {
        let mut y = next.lock().unwrap();
        while *y < h && ck_done(ck, *y) {
            *y += 1;
        }
        if *y < h {
            let n = (*y..h).take(band).take_while(|&r| !ck_done(ck, r)).count();
            let r = (*y, n);
            *y += n;
            board.took(k, r);
            Some(r)
        } else {
            drop(y);
            board.steal(Some(k), band)
        }
    };
    // card `k` stops once the free rows are gone and all of its rows came
    let stop = |k: usize| cancel() || !left() && board.done_for(k);
    // a row: its first result counts (`force`: it replaces that one)
    let row_in = |y: usize, row: &[SiLight], force: bool| {
        if force {
            board.set_delivered(y);
        } else if !board.deliver(y) {
            return;
        }
        if let Some(f) = sink {
            f(y, row);
        }
        ck_put(ck, y, row);
        rows[y].lock().unwrap()[..w].copy_from_slice(&row[..w]);
        let d = done.fetch_add(wm.weight(y), Ordering::Relaxed) + wm.weight(y);
        progress(d.min(wm.total()), wm.total());
    };
    // the bands of the further cards that failed
    let failed = Mutex::new(Vec::new());
    let mut r = Ok(());
    std::thread::scope(|s| {
        for k in crate::gpu::extra_cards(p) {
            let (band, row_in, failed, stop, left, board) = (&band, &row_in, &failed, &stop, &left, &board);
            s.spawn(move || {
                crate::gpu::on_card(k, || {
                    // (once its pipelines are compiled)
                    let Ok(flags) = crate::gpu::march_prepare(p, post) else { return };
                    if !compiled_in_time(&flags, left, cancel) {
                        return;
                    }
                    let claimed = Mutex::new(Vec::new());
                    let mine = AtomicUsize::new(0);
                    let rk = board.keep_going(cancel, || {
                        crate::gpu::march_bands(
                            p,
                            post,
                            &|n| {
                                let b = band(k, n);
                                if let Some(b) = b {
                                    claimed.lock().unwrap().push(b);
                                }
                                b
                            },
                            &|| stop(k),
                            &mut |y, row| {
                                mine.fetch_add(1, Ordering::Relaxed);
                                row_in(y, row, false)
                            },
                        )
                    });
                    let rk = stopped_as_done(rk, cancel);
                    if std::env::var_os("MB3D_CARDS_DEBUG").is_some() {
                        eprintln!("card {}: {} rows marched ({rk:?})", k + 1, mine.load(Ordering::Relaxed));
                    }
                    if rk.as_ref().is_err_and(|e| e != "cancelled") || crate::gpu::rows_overflowed() {
                        failed.lock().unwrap().extend(claimed.into_inner().unwrap());
                    }
                });
            });
        }
        r = stopped_as_done(board.keep_going(cancel, || crate::gpu::march_bands(p, post, &|n| band(0, n), &|| stop(0), &mut |y, row| row_in(y, row, false))), cancel);
    });
    // (on the first card again)
    let redo = failed.into_inner().unwrap();
    if r.is_ok() && !redo.is_empty() && !cancel() {
        let i = Mutex::new(0usize);
        r = crate::gpu::march_bands(
            p,
            post,
            &|_| {
                let mut i = i.lock().unwrap();
                *i += 1;
                redo.get(*i - 1).copied()
            },
            cancel,
            &mut |y, row| row_in(y, row, true),
        );
    }
    drop(rows);
    match r {
        Ok(()) => {
            let mut s = format!("GPU {}", crate::gpu::name().unwrap_or_default());
            if continued {
                s.push_str(" (continued)");
            }
            if post.shadows.is_some() {
                s.push_str(" +shadows");
            }
            if post.deao.is_some() {
                s.push_str(" +AO");
            }
            crate::gpu::set_status(s);
            Ok(Some(gbuf))
        }
        Err(_) if cancel() || crate::gpu::aborted() => {
            // (the rows of a march that overflowed are not usable: all on
            // the CPU next time, as when it fails)
            if crate::gpu::rows_overflowed() {
                if let Some(ck) = ck {
                    ck.reset();
                }
            }
            Err("cancelled".into())
        }
        Err(e) => {
            crate::gpu::set_status(format!("CPU: GPU error: {e}"));
            Ok(None)
        }
    }
}

/// The post calculations after the main calculation (CPU or GPU).
#[allow(clippy::too_many_arguments)]
fn finish_calculation(
    sc: &Scene,
    p: CalcParams,
    mut gbuf: Vec<SiLight>,
    post: u8,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<(CalcParams, Vec<SiLight>), String> {
    let two_d = p.slice_2d != 0;
    if two_d || post == 0 {
        // MB3D runs no post calculations after a 2D calculation
    } else if post == 2 {
        post_process_reporting(sc, &p, &mut gbuf, threads, progress, stage, cancel)?;
    } else if sc.shadows.is_some() || sc.deao.is_some() {
        // (a tile: the normals on the Z-buffer need the neighbour tiles,
        // they run on the whole picture)
        let mut s2 = sc.clone();
        s2.normals_on_zbuf = false;
        if s2.deao.is_none() {
            s2.ao = None;
        }
        post_process_reporting(&s2, &p, &mut gbuf, threads, progress, stage, cancel)?;
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
    /// The card ran the occlusion in one pass with the shadows: its time
    /// is in `shadows`.
    pub ao_with_shadows: bool,
}

/// The times of the post passes the card ran with the main calculation
/// (`p.gpu_post`): after the march they have a time (`p.gpu_post_secs`),
/// within it they have none (`Some(None)`).
pub fn card_post_times(sc: &Scene, p: &CalcParams) -> StageTimes {
    let mut t = StageTimes::default();
    let Some(job) = &p.gpu_post else { return t };
    let (hs, ao) = (sc.shadows.is_some() && job.shadows.is_some(), sc.ao.is_some() && job.deao.is_some());
    if hs {
        t.shadows = Some(p.gpu_post_secs);
    }
    if ao {
        t.ao_with_shadows = hs && p.gpu_post_secs.is_some();
        t.ao = Some(if t.ao_with_shadows { None } else { p.gpu_post_secs });
    }
    t
}

/// The automatic post calculations on the G-buffer (hard shadows, ambient
/// occlusion), as MB3D runs them after the main calculation.  What the
/// graphics card already ran with the same settings (`p.gpu_post`) is kept;
/// what it ran with other settings is undone first.  Returns the times of
/// the shadows and the occlusion.
pub fn post_process(sc: &Scene, p: &CalcParams, gbuf: &mut [SiLight], threads: usize) -> StageTimes {
    post_process_cancellable(sc, p, gbuf, threads, &|| false).unwrap_or_default()
}

/// [`post_process`] that stops (`Err("cancelled")`, the G-buffer half
/// done) when `cancel` returns true: the "Stop" button.
pub fn post_process_cancellable(sc: &Scene, p: &CalcParams, gbuf: &mut [SiLight], threads: usize, cancel: &(dyn Fn() -> bool + Sync)) -> Result<StageTimes, String> {
    post_process_reporting(sc, p, gbuf, threads, &|_, _| {}, &no_stage, cancel)
}

/// [`post_process_cancellable`] that names each pass to `stage` as it
/// starts and counts its rows in `progress` (weighed by their surface
/// pixels).
pub fn post_process_reporting(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &mut [SiLight],
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<StageTimes, String> {
    let stopped = || if cancel() { Err("cancelled".to_string()) } else { Ok(()) };
    let mut times = card_post_times(sc, p);
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
    if sc.shadows.is_some() && !shadows_done {
        stage(&post_stage_name(true, false, ""));
        let t = Instant::now();
        hard_shadows_reporting(sc, p, gbuf, threads, progress, cancel);
        stopped()?;
        if std::env::var_os("MB3D_GPU_TIMING").is_some() {
            eprintln!("  hard shadows (CPU): {:.2} s", t.elapsed().as_secs_f64());
        }
        times.shadows = Some(Some(t.elapsed().as_secs_f64()));
        // (the card's occlusion keeps the time of its pass, which it no
        // longer shares with the shadows)
        if times.ao_with_shadows {
            times.ao_with_shadows = false;
            times.ao = Some(p.gpu_post_secs);
        }
    }
    if let Some(ao) = &sc.ao {
        let t = Instant::now();
        if let Some(d) = &sc.deao {
            if !deao_done {
                stage(&post_stage_name(false, true, ""));
                crate::deao::deao(p, gbuf, p.rect[2] as usize, p.rect[3] as usize, d, threads, progress, cancel);
                times.ao = Some(Some(t.elapsed().as_secs_f64()));
                times.ao_with_shadows = false;
            }
        } else {
            stage(&post_stage_name(false, true, ""));
            ssao(gbuf, p.rect[2] as usize, p.rect[3] as usize, p, ao, threads, progress);
            times.ao = Some(Some(t.elapsed().as_secs_f64()));
            times.ao_with_shadows = false;
        }
        stopped()?;
    }
    Ok(times)
}

/// `progress`: the 15 bit kind counts its rows; the others are one step.
fn ssao(gbuf: &mut [SiLight], w: usize, h: usize, p: &CalcParams, ao: &crate::ssao::SsaoParams, threads: usize, progress: &(dyn Fn(usize, usize) + Sync)) {
    // a panorama wraps around: its left and right edges are neighbours
    let ao = &crate::ssao::SsaoParams { wrap_x: p.optic == CameraOptic::Panorama, ..*ao };
    progress(0, 1);
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
                progress(1, 1);
                return;
            }
            Err(e) => eprintln!("SSAO on the GPU failed, calculated on the CPU: {e}"),
        }
    }
    if ao.bits15 {
        crate::ssao15::ssao15(gbuf, w, h, p.zc_mul, p.zcorr, ao, threads, progress);
    } else {
        crate::ssao::ssao24(gbuf, w, h, p.zc_mul, p.zcorr, ao, threads);
        progress(1, 1);
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
    hard_shadows_cancellable(sc, p, gbuf, threads, &|| false)
}

/// [`hard_shadows`] whose rows stop when `cancel` returns true.
pub fn hard_shadows_cancellable(sc: &Scene, p: &CalcParams, gbuf: &mut [SiLight], threads: usize, cancel: &(dyn Fn() -> bool + Sync)) {
    hard_shadows_reporting(sc, p, gbuf, threads, &|_, _| {}, cancel)
}

/// [`hard_shadows_cancellable`] that counts the rows done in `progress`,
/// weighed by their surface pixels.
pub fn hard_shadows_reporting(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &mut [SiLight],
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) {
    let Some(pass) = ShadowRows::new(sc, p) else { return };
    let w = p.rect[2] as usize;
    let h = p.rect[3] as usize;
    let threads = threads.min(h).max(1);
    let pass = &pass;
    // (the rows weighed by their surface pixels: the pass works on those)
    let sw = &RowWeights::surface(gbuf, w, h);
    let finished = AtomicUsize::new(0);
    let finished = &finished;
    std::thread::scope(|s| {
        let mut rows: Vec<(usize, &mut [SiLight])> = gbuf.chunks_mut(w).enumerate().collect();
        let mut per: Vec<Vec<(usize, &mut [SiLight])>> = (0..threads).map(|_| Vec::new()).collect();
        for (i, r) in rows.drain(..) {
            per[i % threads].push((i, r));
        }
        for list in per {
            s.spawn(move || {
                for (y, row) in list {
                    if cancel() {
                        break;
                    }
                    pass.row(y, row);
                    progress(finished.fetch_add(sw.weight(y), Ordering::Relaxed) + sw.weight(y), sw.total());
                }
            });
        }
    });
}

/// The pass of [`hard_shadows_cancellable`] row by row: the rows in any
/// order, by any thread, with the same results.
struct ShadowRows<'a> {
    p: CalcParams,
    hs: &'a ShadowParams,
    lights: Vec<HsLight>,
}

impl<'a> ShadowRows<'a> {
    fn new(sc: &'a Scene, p: &CalcParams) -> Option<Self> {
        let hs = sc.shadows.as_ref()?;
        let job = shadow_job(sc, p)?;
        // the HS calculation always uses 8 binary search steps
        let mut p = p.clone();
        p.de_add_steps = 8;
        Some(ShadowRows { p, hs, lights: job.lights })
    }

    /// The row `y` (of the calculated rectangle).
    fn row(&self, y: usize, row: &mut [SiLight]) {
        let (x0, y) = (self.p.rect[0], y + self.p.rect[1] as usize);
        let seed = (0x24563487u32 as i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
        let mut m = Marcher::new(&self.p, seed);
        if self.hs.soft {
            m.soft_shadow_row(y as i32, x0, row, &self.lights[0], self.hs.soft_radius, self.hs.max_len_mul);
        } else {
            m.hard_shadow_row(y as i32, x0, row, &self.lights, self.hs.max_len_mul);
        }
    }
}

/// Paints the G-buffer with the scene lighting.
pub fn paint(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> Vec<u8> {
    paint_with::<u8>(sc, p, gbuf, true, &|_, _| {}, &no_stage, &|| false).0
}

/// [`paint`] that stops (`Err("cancelled")`) when `cancel` returns true,
/// also during the reflections and the depth of field: the "Stop" button.
pub fn paint_cancellable(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], cancel: &(dyn Fn() -> bool + Sync)) -> Result<Vec<u8>, String> {
    let rgb = paint_with::<u8>(sc, p, gbuf, true, &|_, _| {}, &no_stage, cancel).0;
    if cancel() { Err("cancelled".into()) } else { Ok(rgb) }
}

/// [`paint`] in 16 bit per colour channel (the colours as calculated,
/// before MB3D's rounding to 8 bit): `mb3d --bits 16`, "PNG 16 bit colour".
pub fn paint16(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> Vec<u16> {
    paint_with::<u16>(sc, p, gbuf, true, &|_, _| {}, &no_stage, &|| false).0
}

/// [`paint`], and the time of the reflections (None without them).
pub fn paint_timed(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> (Vec<u8>, Option<f64>) {
    paint_with::<u8>(sc, p, gbuf, true, &|_, _| {}, &no_stage, &|| false)
}

/// [`paint_timed`] that `cancel` ends early (the picture is then
/// incomplete: the caller checks `cancel` and drops it).
pub fn paint_timed_cancellable(sc: &Scene, p: &CalcParams, gbuf: &[SiLight], cancel: &(dyn Fn() -> bool + Sync)) -> (Vec<u8>, Option<f64>) {
    paint_with::<u8>(sc, p, gbuf, true, &|_, _| {}, &no_stage, cancel)
}

/// [`paint_timed_cancellable`] that names the passes after the lighting
/// ("reflections", "depth of field") to `stage` and counts their rows in
/// `progress`.
pub fn paint_reporting(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &[SiLight],
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> (Vec<u8>, Option<f64>) {
    paint_with::<u8>(sc, p, gbuf, true, progress, stage, cancel)
}

/// [`paint`] without the reflections and the depth of field: the preview
/// of the rows calculated so far (both need the whole image, and the
/// reflections march again, taking as long as the calculation itself),
/// and MB3D's `PaintRows` of a recalculated selection.
pub fn paint_preview(sc: &Scene, p: &CalcParams, gbuf: &[SiLight]) -> Vec<u8> {
    paint_with::<u8>(sc, p, gbuf, false, &|_, _| {}, &no_stage, &|| false).0
}

/// The live picture of a calculation at the size it is shown (every
/// `step`th pixel of every `step`th row, as the window shows it): paints
/// the display rows `rows` (display row, its pixels) into `rgb` (`dw`
/// pixels wide); `rows` None paints every row as background (the start).
/// Base colours only, as [`paint_preview`].  Keeps big pictures (15000 x
/// 15000) from needing a copy of the whole G-buffer for every update.
pub fn paint_display_rows(sc: &Scene, p: &CalcParams, step: usize, rows: Option<&[(usize, Vec<SiLight>)]>, rgb: &mut [u8], dw: usize) {
    let [x0, y0, _, _] = p.rect;
    let step = step.max(1);
    let lv = light_vals(sc, &effective_lighting(sc), &paint_camera(sc, p));
    let cam = paint_camera(sc, p);
    let bg = SiLight::default();
    // the work: (display row, its pixels or None for background, its colours)
    let mut slots: Vec<Option<&mut [u8]>> = rgb.chunks_mut((dw * 3).max(1)).map(Some).collect();
    let mut work: Vec<(usize, Option<&[SiLight]>, &mut [u8])> = Vec::new();
    match rows {
        None => {
            for (dy, s) in slots.iter_mut().enumerate() {
                if let Some(out) = s.take() {
                    work.push((dy, None, out));
                }
            }
        }
        // (a row marched twice: the last one)
        Some(rows) => {
            for (dy, px) in rows.iter().rev() {
                if let Some(out) = slots.get_mut(*dy).and_then(Option::take) {
                    work.push((*dy, Some(px.as_slice()), out));
                }
            }
        }
    }
    let threads = thread_count(sc).min(work.len()).max(1);
    let per = work.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let mut it = work.into_iter();
        loop {
            let part: Vec<_> = it.by_ref().take(per).collect();
            if part.is_empty() {
                break;
            }
            let (lv, cam, bg) = (&lv, &cam, &bg);
            s.spawn(move || {
                for (dy, px, out) in part {
                    for (dx, c) in out.chunks_exact_mut(3).enumerate() {
                        let si = px.and_then(|r| r.get(dx)).unwrap_or(bg);
                        c.copy_from_slice(&lv.pixel_color_s::<u8>(si, (dx * step) as i32 + x0, (dy * step) as i32 + y0, cam));
                    }
                }
            });
        }
    });
}

fn paint_with<S: crate::lighting::Sample>(
    sc: &Scene,
    p: &CalcParams,
    gbuf: &[SiLight],
    full: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> (Vec<S>, Option<f64>) {
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
        stage("reflections");
        let t = Instant::now();
        crate::reflect::reflections(sc, p, gbuf, &mut rgb, &cam, &lv, threads, progress, cancel);
        refl_s = Some(t.elapsed().as_secs_f64());
    }
    if let Some(d) = sc.dof.as_ref().filter(|_| p.slice_2d == 0 && !cancel()) {
        stage("depth of field");
        crate::dof::apply_reporting(&mut rgb, gbuf, w as usize, h as usize, p.zc_mul, p.zcorr, sc.fov_y, d, progress, cancel);
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

/// The memory a whole picture of `w` x `h` takes while it is calculated
/// (measured: the buffers, the card's copies, the picture; on the safe side).
pub fn memory_needed(w: usize, h: usize) -> u64 {
    (w as u64).saturating_mul(h as u64).saturating_mul(80).saturating_add(300 << 20)
}

/// The memory the system can give without swapping (`MemAvailable`); None
/// where it cannot be told (other systems than Linux).
pub fn memory_available() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb: u64 = text.lines().find_map(|l| l.strip_prefix("MemAvailable:"))?.trim().trim_end_matches("kB").trim().parse().ok()?;
    Some(kb * 1024)
}

/// Err when a picture of `w` x `h` does not fit into the free memory: the
/// calculation would only swap until the system stops it.
pub fn check_memory(w: usize, h: usize) -> Result<(), String> {
    let need = memory_needed(w, h);
    match memory_available() {
        Some(free) if need > free => Err(format!("a {w} x {h} picture needs about {} of memory, this computer has {} free", gigabytes(need), gigabytes(free))),
        _ => Ok(()),
    }
}

/// "12 GB", "0.5 GB".
pub fn gigabytes(bytes: u64) -> String {
    let g = bytes as f64 / (1u64 << 30) as f64;
    if g >= 10.0 {
        format!("{g:.0} GB")
    } else {
        format!("{g:.1} GB")
    }
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
    render_tiled_cancellable(sc, keep_gbuffer, progress, tile_progress, &|| false)
}

/// Like [`render_tiled`]; stops early with `Err("cancelled")` as soon as
/// `cancel` returns true.
pub fn render_tiled_cancellable(
    sc: &Scene,
    keep_gbuffer: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    tile_progress: &dyn Fn(usize, usize, [i32; 4]),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<TiledResult, String> {
    render_tiled_with(sc, keep_gbuffer, progress, tile_progress, cancel, None, Policy::Off)
}

/// The calculation of one tile of `tl` (with its border, see
/// [`render_tiled`]): its scene and the tile's rectangle.
fn single_tile(sc: &Scene, tl: &crate::scene::Tiling, c: u32, r: u32) -> Result<(Scene, [i32; 4]), String> {
    let (w, h) = (sc.width, sc.height);
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
    Ok((ts, t))
}

/// The checkpoint of a tiled render of `sc` (for [`render_tiled_with`]):
/// one tile, its rows; all tiles, the finished tiles (the tile under way
/// keeps its rows in a checkpoint of its own).
pub fn open_tiles_checkpoint(sc: &Scene, name: &str, policy: Policy) -> Option<Store> {
    let tl = sc.tiling?;
    if let Some((c, r)) = tl.pos {
        return open_checkpoint(&single_tile(sc, &tl, c, r).ok()?.0, name, policy);
    }
    let (w, h) = (sc.width.max(0) as usize, sc.height.max(0) as usize);
    let spec = crate::checkpoint::Spec {
        kind: "tiles".into(),
        key: crate::checkpoint::key("tiles", &checkpoint_text(sc)),
        name: name.into(),
        width: w,
        height: h,
        rec: crate::gbuffer::REC_BYTES,
        units: (tl.cols * tl.rows) as usize,
    };
    let s = Store::open(spec, policy)?;
    s.put_extra("scene.m3s", sc.to_text().as_bytes());
    Some(s)
}

/// [`render_tiled_cancellable`] with a checkpoint from
/// [`open_tiles_checkpoint`]: the tiles (and rows) it holds are not
/// calculated again; `policy` is that of the tiles' own checkpoints.  The
/// caller ends `ck` when the result is saved.
pub fn render_tiled_with(
    sc: &Scene,
    keep_gbuffer: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    tile_progress: &dyn Fn(usize, usize, [i32; 4]),
    cancel: &(dyn Fn() -> bool + Sync),
    ck: Option<&Store>,
    policy: Policy,
) -> Result<TiledResult, String> {
    let tl = sc.tiling.ok_or("no tiling set")?;
    let (w, h) = (sc.width, sc.height);
    if let Some((c, r)) = tl.pos {
        let (ts, t) = single_tile(sc, &tl, c, r)?;
        let [x0, y0] = [ts.calc_rect.unwrap()[0], ts.calc_rect.unwrap()[1]];
        let t0 = Instant::now();
        let (p, gbuf) = calculate_inner(&ts, progress, &no_stage, cancel, 2, None, ck)?;
        let t1 = Instant::now();
        let trgb = paint(&ts, &p, &gbuf);
        let paint_s = t1.elapsed().as_secs_f64();
        let tw = ts.calc_rect.unwrap()[2] as usize;
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
    let ck = ck.filter(|s| s.spec().kind == "tiles" && s.spec().units == total && (s.spec().width, s.spec().height) == (wu, hu));
    let t0 = Instant::now();
    let mut n = 0;
    for r in 0..tl.rows {
        for c in 0..tl.cols {
            let t = tl.tile_rect(w, h, c, r);
            n += 1;
            if t[2] <= 0 || t[3] <= 0 {
                continue;
            }
            let rect = t.map(|v| v as usize);
            let tw = t[2] as usize;
            let k = (r * tl.cols + c) as usize;
            let put = |full: &mut [SiLight], gbuf: &[SiLight]| {
                for y in 0..t[3] as usize {
                    let d = (t[1] as usize + y) * wu + t[0] as usize;
                    full[d..d + tw].copy_from_slice(&gbuf[y * tw..(y + 1) * tw]);
                }
            };
            // (a tile of the checkpoint)
            if let Some(s) = ck.filter(|s| s.get(k).is_some_and(|d| d.rect == rect)) {
                put(&mut full, &crate::gbuffer::unpack(&s.read(rect)));
                tile_progress(n, total, t);
                continue;
            }
            let mut ts = sc.clone();
            ts.calc_rect = Some(t);
            // (the DE occlusion uses the normals: after them, on the
            // whole picture)
            if sc.normals_on_zbuf {
                ts.deao = None;
            }
            let tck = ck.and_then(|s| open_checkpoint(&ts, &s.spec().name, policy));
            let (_, gbuf) = calculate_inner(&ts, progress, &no_stage, cancel, 1, None, tck.as_ref())?;
            put(&mut full, &gbuf);
            if let Some(s) = ck {
                s.put(k, 0, rect, &crate::gbuffer::pack(&gbuf));
            }
            if let Some(s) = tck {
                s.finish();
            }
            tile_progress(n, total, t);
        }
    }
    let mut fs = sc.clone();
    fs.calc_rect = None;
    let p = CalcParams::new(&fs)?;
    // (in the order of post_process: the normals, the shadows (per tile),
    // the occlusion)
    if sc.normals_on_zbuf && p.slice_2d == 0 {
        crate::reflect::normals_on_zbuf(&fs, &p, &mut full);
        if let (Some(_), Some(d)) = (&sc.ao, &sc.deao) {
            crate::deao::deao(&p, &mut full, wu, hu, d, thread_count(sc), progress, cancel);
            if cancel() {
                return Err("cancelled".into());
            }
        }
    }
    if let (Some(ao), None) = (&sc.ao, &sc.deao) {
        ssao(&mut full, wu, hu, &p, ao, thread_count(sc), &|_, _| {});
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

/// One tile of `sc.tiling` (its `pos` set) as [`render_tiled`] calculates
/// it, with its border, for the swarm: `stage` is told what runs after the
/// march (which reports through `progress`), so a long post calculation
/// does not look like a hang.  `full`: painted with the reflections and
/// the depth of field (the tile is the final picture); else a preview
/// (the G-buffer is the result, the whole picture is painted from it).
/// `ck`: the tile's checkpoint ([`open_tiles_checkpoint`]), whose rows
/// are not calculated again; the caller ends it when the tile is saved.
pub fn render_tile_staged(
    sc: &Scene,
    full: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    stage: &(dyn Fn(&str) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    ck: Option<&Store>,
) -> Result<TiledResult, String> {
    let tl = sc.tiling.ok_or("no tiling set")?;
    let (c, r) = tl.pos.ok_or("no tile set")?;
    let (w, h) = (sc.width, sc.height);
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
    let (p, mut gbuf) = calculate_inner(&ts, progress, &no_stage, cancel, 0, None, ck)?;
    if post_pending(&ts, &p) {
        stage(match (ts.shadows.is_some(), ts.ao.is_some()) {
            (true, true) => "hard shadows and ambient occlusion",
            (true, false) => "hard shadows",
            _ => "ambient occlusion",
        });
        post_process_cancellable(&ts, &p, &mut gbuf, thread_count(&ts), cancel)?;
    }
    let t1 = Instant::now();
    let trgb = if full {
        let what = match (ts.mc.reflections, ts.dof.is_some()) {
            (true, true) => "painting: reflections and depth of field",
            (true, false) => "painting: reflections",
            (false, true) => "painting: depth of field",
            (false, false) => "painting",
        };
        stage(what);
        let rgb = paint_with::<u8>(&ts, &p, &gbuf, true, &|_, _| {}, &no_stage, cancel).0;
        if cancel() {
            return Err("cancelled".into());
        }
        rgb
    } else {
        stage("painting a preview");
        paint_preview(&ts, &p, &gbuf)
    };
    let paint_s = t1.elapsed().as_secs_f64();
    let tw = (x1 - x0) as usize;
    let (ow, oh) = (t[2] as usize, t[3] as usize);
    let (sx, sy) = ((t[0] - x0) as usize, (t[1] - y0) as usize);
    let mut rgb = Vec::with_capacity(ow * oh * 3);
    let mut g = Vec::with_capacity(ow * oh);
    for y in 0..oh {
        let o = (sy + y) * tw + sx;
        rgb.extend_from_slice(&trgb[o * 3..(o + ow) * 3]);
        g.extend_from_slice(&gbuf[o..o + ow]);
    }
    Ok(TiledResult { width: ow, height: oh, rgb, gbuffer: Some(g), calc_seconds: (t1 - t0).as_secs_f64(), paint_seconds: paint_s, params: Some(p) })
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A probe G-buffer of `pw` x `ph`: the top `sky` rows background, the
    /// rows below on the object with `steps` DE steps per pixel.
    fn probe_gbuf(pw: usize, ph: usize, sky: usize, steps: u16) -> Vec<SiLight> {
        let mut g = vec![SiLight::default(); pw * ph];
        for (y, row) in g.chunks_mut(pw).enumerate() {
            if y >= sky {
                for s in row {
                    s.zpos_fine = 1000 << 16;
                    s.shadow = steps;
                }
            }
        }
        g
    }

    /// The progress a render of `h` rows done in order reports (per
    /// mille, as the window stores it) after each row.
    fn per_mille(wm: &RowWeights, h: usize) -> Vec<usize> {
        let done = AtomicUsize::new(0);
        (0..h).map(|y| (done.fetch_add(wm.weight(y), Ordering::Relaxed) + wm.weight(y)) * 1000 / wm.total()).collect()
    }

    #[test]
    fn probe_weights_follow_the_de_steps() {
        // the probe at 1/4: 2 rows of sky, 2 of object; 16 rows in all
        let g = probe_gbuf(8, 4, 2, 200);
        let wm = RowWeights::from_probe(&g, 8, 4, 16);
        assert_eq!(wm.rows(), 16);
        // the full rows of a probe row weigh the same
        assert_eq!(wm.weight(0), wm.weight(7));
        assert_eq!(wm.weight(8), wm.weight(15));
        // an object row costs (200 + 4) / 4 times a sky row (rounded)
        let ratio = wm.weight(8) as f64 / wm.weight(0) as f64;
        assert!((ratio - 51.0).abs() < 4.0, "{ratio}");
        // the mean about 100
        assert!((wm.total() as i64 - 1600).abs() <= 16, "{}", wm.total());
        assert_eq!(wm.before(8), 8 * wm.weight(0));
        assert_eq!(wm.before(16), wm.total());
        // equal weights without a probe
        assert_eq!(RowWeights::from_probe(&[], 0, 0, 5), RowWeights::equal(5));
        assert_eq!(RowWeights::equal(5).total(), 5);
    }

    #[test]
    fn progress_is_monotonic_and_reaches_1000() {
        let g = probe_gbuf(8, 4, 1, 300);
        for wm in [RowWeights::from_probe(&g, 8, 4, 37), RowWeights::equal(37), RowWeights::surface(&g, 8, 4)] {
            let p = per_mille(&wm, wm.rows());
            assert!(p.windows(2).all(|w| w[0] <= w[1]), "{p:?}");
            assert_eq!(*p.last().unwrap(), 1000);
        }
    }

    #[test]
    fn half_sky_is_far_below_half_the_work() {
        let g = probe_gbuf(8, 4, 2, 200);
        let wm = RowWeights::from_probe(&g, 8, 4, 16);
        let p = per_mille(&wm, 16);
        // the sky rows done: a few per cent, not 50 (4 / (4 + 204))
        assert!(p[7] < 50, "{}", p[7]);
        assert!(p[7] >= 10, "{}", p[7]);
        assert_eq!(p[15], 1000);
    }

    #[test]
    fn surface_weights_count_the_object_pixels() {
        let g = probe_gbuf(8, 4, 1, 10);
        let sw = RowWeights::surface(&g, 8, 4);
        assert_eq!((0..4).map(|y| sw.weight(y)).collect::<Vec<_>>(), vec![1, 9, 9, 9]);
        assert_eq!(sw.total(), 28);
        assert_eq!(sw.before(2), 10);
    }

    #[test]
    fn checkpoint_key_differs_per_tile() {
        let mut sc = Scene { width: 64, height: 64, ..Default::default() };
        let tl = crate::scene::Tiling { cols: 2, rows: 2, pos: None, downscale: 1 };
        let whole = checkpoint_key(&sc);
        let mut keys = Vec::new();
        for (c, r) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            sc.calc_rect = Some(tl.tile_rect(64, 64, c, r));
            keys.push(checkpoint_key(&sc));
        }
        keys.push(whole);
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n);
    }

    #[test]
    fn tiles_with_normals_on_zbuf_match_the_whole_picture() {
        let mut sc = Scene::preset("Integer Power").unwrap();
        (sc.width, sc.height, sc.threads) = (64, 48, 2);
        sc.normals_on_zbuf = true;
        let whole = crate::render(&sc, &|_, _| {}).unwrap();
        sc.tiling = Some(crate::scene::Tiling { cols: 2, rows: 2, pos: None, downscale: 1 });
        let t = render_tiled(&sc, true, &|_, _| {}, &|_, _, _| {}).unwrap();
        assert!(t.gbuffer.unwrap() == whole.gbuffer);
        assert_eq!(t.rgb, whole.rgb);
    }
}
