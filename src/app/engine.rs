//! The calculations behind the main window: "Calculate 3D" in a background
//! thread with the image shown while it is calculated, the kept G-buffer
//! (MB3D's siLight5 array) that the lighting and post processing repaint
//! without a new ray march, 2D slices and recalculated selections.

use crate::calc::CalcParams;
use crate::gbuffer::SiLight;
use crate::scene::Scene;
use crate::vcl::ui::Waker;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub enum Job {
    /// main calculation (`slice` 0 = 3D, 1..3 = 2D plane at z start / mid / end)
    Calc { scene: Scene, slice: u8 },
    /// the Navigator's preview: the scene at 1/8, 1/4, 1/2 and full size
    /// (MB3D's NaviStep 8, 4, 2, 1), each pass shown when done
    Preview { scene: Scene },
    /// "Calc-": the rough image of 8x8 blocks (MB3D's `CalcMandBlocky`)
    Blocky { scene: Scene },
    /// lighting / post processing changed: repaint the kept G-buffer
    Repaint { scene: Scene },
    /// "Recalculate a selection": rectangle (left, top, width, height) of
    /// the image with the raystep divided, optionally only nearer parts
    Recalc { scene: Scene, rect: [i32; 4], div: f64, nearer: bool },
}

/// Statistics of the calculated image (Infos tab).
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub avg_steps: f64,
    pub avg_its: f64,
    pub max_its: i32,
    pub calc_s: f64,
    pub post_s: f64,
    pub hits: f64,
    /// the stages after the main calculation
    pub stages: crate::render::StageTimes,
}

/// The kept result of the last calculation.
#[derive(Clone)]
pub struct Base {
    pub scene: Scene,
    pub params: CalcParams,
    /// before the post calculations
    pub raw: Arc<Vec<SiLight>>,
    /// after hard shadows / ambient occlusion
    pub post: Arc<Vec<SiLight>>,
    pub post_sig: String,
    pub w: usize,
    pub h: usize,
}

#[derive(Default)]
pub struct Output {
    /// the image (full size RGB)
    pub rgb: Option<Arc<Vec<u8>>>,
    pub w: usize,
    pub h: usize,
    /// bumped when `rgb` changes
    pub ver: u64,
    pub running: bool,
    pub stage: String,
    pub error: String,
    pub stats: Stats,
    /// the last job finished (for "Calculate 3D" -> enable buttons)
    pub finished: u64,
    pub was_calc3d: bool,
}

struct Partial {
    params: CalcParams,
    scene: Scene,
    gbuf: Vec<SiLight>,
    w: usize,
    rows_new: bool,
}

pub struct Inner {
    job: Mutex<Option<(u64, Job)>>,
    cv: Condvar,
    gen: AtomicU64,
    pub progress: AtomicU32,
    pub out: Mutex<Output>,
    pub base: Mutex<Option<Arc<Base>>>,
    partial: Mutex<Option<Partial>>,
    /// the painted preview of the rows so far, and whether a painter runs
    preview: Mutex<Option<(Vec<u8>, usize, usize)>>,
    painting: AtomicBool,
    waker: Mutex<Option<Waker>>,
    busy: AtomicBool,
}

pub struct Engine(pub Arc<Inner>);

/// The program ends: the calculation stops first and is waited for (at
/// most a few seconds).  A calculation still waiting for the graphics card
/// when the process exits crashes in the driver.
impl Drop for Engine {
    fn drop(&mut self) {
        #[cfg(feature = "gpu")]
        crate::gpu::abort_all();
        self.stop();
        let t = Instant::now();
        while self.0.busy.load(Ordering::SeqCst) && t.elapsed().as_secs() < 5 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}

/// Everything the post calculations depend on.
pub fn post_sig(sc: &Scene) -> String {
    let mut s = sc.clone();
    let lights = s.lighting.lights.clone();
    s.lighting = Default::default();
    if s.shadows.is_some() {
        s.lighting.lights = lights;
    }
    s.dof = None;
    s.mc = Default::default();
    s.threads = 0;
    s.to_text()
}

/// Everything the ray marching depends on.
pub fn calc_sig(sc: &Scene) -> String {
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

fn needs_post(sc: &Scene, p: &CalcParams) -> bool {
    crate::render::post_pending(sc, p)
}

pub fn stats_of(g: &[SiLight], calc_s: f64, post_s: f64) -> Stats {
    let n = g.len().max(1) as f64;
    let hits = g.iter().filter(|s| !s.is_background()).count() as f64;
    let steps = g.iter().map(|s| (s.shadow & 0x3FF) as f64).sum::<f64>() / n;
    Stats { avg_steps: steps, avg_its: 0.0, max_its: 0, calc_s, post_s, hits: 100.0 * hits / n, stages: Default::default() }
}

impl Engine {
    pub fn new() -> Engine {
        let inner = Arc::new(Inner {
            job: Mutex::new(None),
            cv: Condvar::new(),
            gen: AtomicU64::new(0),
            progress: AtomicU32::new(0),
            out: Mutex::new(Output::default()),
            base: Mutex::new(None),
            partial: Mutex::new(None),
            preview: Mutex::new(None),
            painting: AtomicBool::new(false),
            waker: Mutex::new(None),
            busy: AtomicBool::new(false),
        });
        let i2 = inner.clone();
        std::thread::Builder::new().name("mb3d-calc".into()).spawn(move || worker(i2)).expect("calculation thread");
        Engine(inner)
    }

    pub fn set_waker(&self, w: Option<Waker>) {
        *self.0.waker.lock().unwrap() = w;
    }

    /// Starts a job; a running one is cancelled.
    pub fn start(&self, job: Job) {
        let gen = self.0.gen.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut o = self.0.out.lock().unwrap();
            o.running = true;
            o.error.clear();
            o.was_calc3d = matches!(job, Job::Calc { slice: 0, .. });
        }
        self.0.progress.store(0, Ordering::Relaxed);
        *self.0.job.lock().unwrap() = Some((gen, job));
        self.0.cv.notify_all();
    }

    /// Stops the running calculation (the "Stop" button).
    pub fn stop(&self) {
        self.0.gen.fetch_add(1, Ordering::SeqCst);
        *self.0.job.lock().unwrap() = None;
    }

    pub fn running(&self) -> bool {
        self.0.out.lock().unwrap().running || self.0.busy.load(Ordering::SeqCst)
    }

    /// Progress of the running job, 0..1.
    pub fn progress(&self) -> f32 {
        self.0.progress.load(Ordering::Relaxed) as f32 / 1000.0
    }

    /// The rows calculated so far, painted (None when nothing new).  The
    /// painting runs on a thread of its own, from a copy of the rows, so
    /// that the window stays responsive: a call starts it, a later call
    /// returns the image.
    pub fn partial_image(&self) -> Option<(Vec<u8>, usize, usize)> {
        if let Some(img) = self.0.preview.lock().unwrap().take() {
            return Some(img);
        }
        if self.0.painting.load(Ordering::SeqCst) {
            return None;
        }
        let (scene, params, gbuf, w) = {
            let mut p = self.0.partial.lock().unwrap();
            let p = p.as_mut()?;
            if !p.rows_new {
                return None;
            }
            p.rows_new = false;
            (p.scene.clone(), p.params.clone(), p.gbuf.clone(), p.w)
        };
        self.0.painting.store(true, Ordering::SeqCst);
        let e = self.0.clone();
        std::thread::spawn(move || {
            let rgb = crate::render::paint_preview(&scene, &params, &gbuf);
            let h = gbuf.len() / w.max(1);
            *e.preview.lock().unwrap() = Some((rgb, w, h));
            e.painting.store(false, Ordering::SeqCst);
            e.wake();
        });
        None
    }

    pub fn base(&self) -> Option<Arc<Base>> {
        self.0.base.lock().unwrap().clone()
    }

    /// The object position (absolute) under pixel `x`, `y` of the kept
    /// image and its distance from the camera plane as a fraction of the
    /// image width (MB3D's `GetZPos / (dStepWidth * Width)`); None over the
    /// background.
    pub fn pick(&self, x: i32, y: i32) -> Option<([f64; 3], f64)> {
        let b = self.base()?;
        if x < 0 || y < 0 || x as usize >= b.w || y as usize >= b.h {
            return None;
        }
        let cam = crate::render::paint_camera(&b.scene, &b.params);
        let si = b.post.get(y as usize * b.w + x as usize)?;
        let p = cam.object_pos(si, x, y)?;
        let sc = &b.scene;
        let pos = [p[0] + sc.mid[0], p[1] + sc.mid[1], p[2] + sc.mid[2]];
        let vz = crate::math::normalize(sc.vgrads[2]);
        let d = sc.z_start - sc.mid[2];
        let cam_pos = [sc.mid[0] + vz[0] * d, sc.mid[1] + vz[1] * d, sc.mid[2] + vz[2] * d];
        let z = (pos[0] - cam_pos[0]) * vz[0] + (pos[1] - cam_pos[1]) * vz[1] + (pos[2] - cam_pos[2]) * vz[2];
        Some((pos, z / (sc.step_width() * sc.width as f64)))
    }

    /// Changes the kept G-buffer in place (painting colours on the image):
    /// `f` gets the calculated scene, its parameters and the pixels, once
    /// for the buffer before and once for the one after the shadows when
    /// they differ.  The caller repaints.  False without a kept image.
    pub fn edit_gbuffer(&self, f: impl Fn(&Scene, &CalcParams, &mut [SiLight])) -> bool {
        let mut g = self.0.base.lock().unwrap();
        let Some(arc) = g.as_mut() else { return false };
        let b = Arc::make_mut(arc);
        let shared = Arc::ptr_eq(&b.raw, &b.post);
        if shared {
            // (one buffer for both: edited without copying it)
            b.raw = Arc::new(Vec::new());
        }
        f(&b.scene, &b.params, Arc::make_mut(&mut b.post).as_mut_slice());
        if shared {
            b.raw = b.post.clone();
        } else {
            f(&b.scene, &b.params, Arc::make_mut(&mut b.raw).as_mut_slice());
        }
        true
    }

    /// Drops the kept image (a new parameter set was loaded).
    pub fn clear(&self) {
        *self.0.base.lock().unwrap() = None;
        let mut o = self.0.out.lock().unwrap();
        o.rgb = None;
        o.ver += 1;
    }

    /// Sets the kept G-buffer from an .m3i file and paints it.
    pub fn set_gbuffer(&self, sc: &Scene, gbuf: Vec<SiLight>) -> Result<(), String> {
        let p = CalcParams::new(sc)?;
        let (w, h) = (sc.width as usize, sc.height as usize);
        if gbuf.len() != w * h {
            return Err("the image buffer has the wrong size".into());
        }
        let raw = Arc::new(gbuf);
        let rgb = crate::render::paint(sc, &p, &raw);
        *self.0.base.lock().unwrap() = Some(Arc::new(Base { scene: sc.clone(), params: p, raw: raw.clone(), post: raw, post_sig: post_sig(sc), w, h }));
        let mut o = self.0.out.lock().unwrap();
        o.rgb = Some(Arc::new(rgb));
        o.w = w;
        o.h = h;
        o.ver += 1;
        Ok(())
    }
}

impl Inner {
    fn wake(&self) {
        if let Some(w) = self.waker.lock().unwrap().as_ref() {
            w.wake();
        }
    }

    fn publish(&self, rgb: Vec<u8>, w: usize, h: usize, stage: &str, stats: Option<Stats>) {
        let mut o = self.out.lock().unwrap();
        o.rgb = Some(Arc::new(rgb));
        o.w = w;
        o.h = h;
        o.ver += 1;
        o.stage = stage.to_string();
        if let Some(s) = stats {
            o.stats = s;
        }
    }

    fn finish(&self, err: Option<String>) {
        {
            let mut o = self.out.lock().unwrap();
            o.running = false;
            o.finished += 1;
            if let Some(e) = err {
                if e != "cancelled" {
                    o.error = e;
                }
            }
        }
        *self.partial.lock().unwrap() = None;
        self.wake();
    }
}

fn worker(e: Arc<Inner>) {
    loop {
        let (gen, job) = {
            let mut j = e.job.lock().unwrap();
            while j.is_none() {
                j = e.cv.wait(j).unwrap();
            }
            j.take().unwrap()
        };
        e.busy.store(true, Ordering::SeqCst);
        let cancel = || e.gen.load(Ordering::SeqCst) != gen;
        // a panic ends the job with an error message instead of leaving
        // the window waiting for it
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job {
            Job::Calc { scene, slice } => calc(&e, &scene, slice, &cancel),
            Job::Repaint { scene } => repaint(&e, &scene, &cancel),
            Job::Blocky { scene } => blocky(&e, &scene, &cancel),
            Job::Preview { scene } => preview(&e, &scene, &cancel),
            Job::Recalc { scene, rect, div, nearer } => recalc(&e, &scene, rect, div, nearer, &cancel),
        }))
        .unwrap_or_else(|p| {
            let m = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
            Err(format!("The calculation failed: {}", m.lines().last().unwrap_or("").trim()))
        });
        e.busy.store(false, Ordering::SeqCst);
        // a newer job may already wait: only report when this one is the last
        if e.job.lock().unwrap().is_none() {
            e.finish(res.err());
        }
    }
}

fn calc(e: &Inner, sc: &Scene, slice: u8, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    let mut sc = sc.clone();
    sc.slice_2d = slice;
    sc.calc_rect = None;
    let t0 = Instant::now();
    let params = CalcParams::new(&sc)?;
    let [_, _, w, h] = params.rect;
    let (w, h) = (w as usize, h as usize);
    *e.partial.lock().unwrap() = Some(Partial { params, scene: sc.clone(), gbuf: vec![SiLight::default(); w * h], w, rows_new: false });
    {
        let mut o = e.out.lock().unwrap();
        o.stage = if slice == 0 { "main rendering".into() } else { "2D calculation".into() };
    }
    let progress = |d: usize, t: usize| e.progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
    let sink = |y: usize, row: &[SiLight]| {
        if let Some(p) = e.partial.lock().unwrap().as_mut() {
            let o = y * p.w;
            if o + row.len() <= p.gbuf.len() {
                p.gbuf[o..o + row.len()].copy_from_slice(row);
                p.rows_new = true;
            }
        }
    };
    let (p, raw) = crate::render::calculate_raw_rows(&sc, &progress, cancel, &sink)?;
    *e.partial.lock().unwrap() = None;
    let t1 = Instant::now();
    let raw = Arc::new(raw);
    let mut stages = crate::render::StageTimes::default();
    let post = if needs_post(&sc, &p) {
        e.out.lock().unwrap().stage = if sc.shadows.is_some() { "hard shadow calculation".into() } else { "ambient shadow calculation".into() };
        let mut g = (*raw).clone();
        stages = crate::render::post_process(&sc, &p, &mut g, crate::render::thread_count(&sc));
        Arc::new(g)
    } else {
        // the card ran the shadows and the occlusion with the march
        stages.shadows = sc.shadows.as_ref().map(|_| None);
        stages.ao = sc.ao.as_ref().map(|_| None);
        raw.clone()
    };
    if cancel() {
        return Err("cancelled".into());
    }
    let t2 = Instant::now();
    let (rgb, refl_s) = crate::render::paint_timed(&sc, &p, &post);
    stages.reflections = refl_s;
    let post_s = if needs_post(&sc, &p) { (t2 - t1).as_secs_f64() } else { 0.0 };
    let mut stats = stats_of(&post, (t1 - t0).as_secs_f64(), post_s);
    stats.stages = stages;
    e.publish(rgb, w, h, "", Some(stats));
    *e.base.lock().unwrap() = Some(Arc::new(Base { scene: sc.clone(), params: p, raw, post, post_sig: post_sig(&sc), w, h }));
    Ok(())
}

fn blocky(e: &Inner, sc: &Scene, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    let mut sc = sc.clone();
    sc.slice_2d = 0;
    sc.calc_rect = None;
    let t0 = Instant::now();
    let params = CalcParams::new(&sc)?;
    let [_, _, w, h] = params.rect;
    let (w, h) = (w as usize, h as usize);
    *e.partial.lock().unwrap() = Some(Partial { params, scene: sc.clone(), gbuf: vec![SiLight::default(); w * h], w, rows_new: false });
    e.out.lock().unwrap().stage = "rough image".into();
    let progress = |d: usize, t: usize| e.progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
    let sink = |y: usize, row: &[SiLight]| {
        if let Some(p) = e.partial.lock().unwrap().as_mut() {
            let o = y * p.w;
            if o + row.len() <= p.gbuf.len() {
                p.gbuf[o..o + row.len()].copy_from_slice(row);
                p.rows_new = true;
            }
        }
    };
    let (p, g) = crate::render::calculate_blocky(&sc, &progress, cancel, &sink)?;
    *e.partial.lock().unwrap() = None;
    // painted without the post calculations, as MB3D's PaintRowsNoThreadBlocky
    let rgb = crate::render::paint_preview(&sc, &p, &g);
    let stats = stats_of(&g, t0.elapsed().as_secs_f64(), 0.0);
    e.publish(rgb, w, h, "", Some(stats));
    let g = Arc::new(g);
    *e.base.lock().unwrap() = Some(Arc::new(Base { scene: sc.clone(), params: p, raw: g.clone(), post: g, post_sig: post_sig(&sc), w, h }));
    Ok(())
}

fn preview(e: &Inner, sc: &Scene, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    let full = sc.width.max(8);
    for div in [8, 4, 2, 1] {
        if cancel() {
            return Err("cancelled".into());
        }
        let w = full / div;
        if w < 24 && div > 1 {
            continue;
        }
        let mut ps = sc.clone();
        ps.calc_rect = None;
        ps.tiling = None;
        let f = w as f64 / sc.width as f64;
        ps.width = w;
        ps.height = ((sc.height as f64 * f).round() as i32).max(4);
        let progress = |d: usize, t: usize| e.progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
        let (p, g) = crate::render::calculate_cancellable(&ps, &progress, cancel)?;
        if cancel() {
            return Err("cancelled".into());
        }
        let rgb = crate::render::paint(&ps, &p, &g);
        let (pw, ph) = (ps.width as usize, ps.height as usize);
        e.publish(rgb, pw, ph, "", None);
        if div == 1 {
            let g = Arc::new(g);
            *e.base.lock().unwrap() = Some(Arc::new(Base { scene: ps.clone(), params: p, raw: g.clone(), post: g, post_sig: post_sig(&ps), w: pw, h: ph }));
        }
    }
    Ok(())
}

fn repaint(e: &Inner, sc: &Scene, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    let Some(b) = e.base.lock().unwrap().clone() else { return Ok(()) };
    let mut target = sc.clone();
    target.width = b.scene.width;
    target.height = b.scene.height;
    target.slice_2d = b.scene.slice_2d;
    target.calc_rect = None;
    let psig = post_sig(&target);
    let reuse = psig == b.post_sig || b.params.slice_2d != 0;
    let post = if reuse {
        b.post.clone()
    } else {
        e.out.lock().unwrap().stage = "post processing".into();
        let mut g = (*b.raw).clone();
        if needs_post(&target, &b.params) {
            crate::render::post_process(&target, &b.params, &mut g, crate::render::thread_count(&target));
        }
        Arc::new(g)
    };
    if cancel() {
        return Err("cancelled".into());
    }
    let rgb = crate::render::paint(&target, &b.params, &post);
    // stopped during the reflections: the image stays as it was
    if cancel() {
        return Err("cancelled".into());
    }
    e.publish(rgb, b.w, b.h, "", None);
    let mut g = e.base.lock().unwrap();
    // (the buffers painted on meanwhile ("Paint on image") are kept: their
    // own repaint follows)
    let (raw, post) = match g.as_ref() {
        Some(cur) if reuse => (cur.raw.clone(), cur.post.clone()),
        _ => (b.raw.clone(), post),
    };
    *g = Some(Arc::new(Base { scene: target, params: b.params.clone(), raw, post, post_sig: psig, w: b.w, h: b.h }));
    Ok(())
}

fn recalc(e: &Inner, sc: &Scene, rect: [i32; 4], div: f64, nearer: bool, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    let b = e.base.lock().unwrap().clone().ok_or("calculate the image first (Calculate 3D)")?;
    if sc.width != b.scene.width || sc.height != b.scene.height {
        return Err("the image size changed: calculate the image again".into());
    }
    let [x0, y0, rw, rh] = rect;
    if rw < 1 || rh < 1 {
        return Err("mark a selection in the image first".into());
    }
    e.out.lock().unwrap().stage = "recalculating the selection".into();
    let mut rs = sc.clone();
    rs.calc_rect = Some(rect);
    rs.z_step_div = (rs.z_step_div / div.max(1.0)).max(1e-4);
    let progress = |d: usize, t: usize| e.progress.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
    let (_, g) = crate::render::calculate_raw_cancellable(&rs, &progress, cancel)?;
    let mut merged = (*b.raw).clone();
    let w = b.w;
    for y in 0..rh as usize {
        for x in 0..rw as usize {
            let n = g[y * rw as usize + x];
            let o = &mut merged[(y0 as usize + y) * w + x0 as usize + x];
            if !nearer || n.zpos_fine < o.zpos_fine {
                *o = n;
            }
        }
    }
    let p = CalcParams::new(sc)?;
    let raw = Arc::new(merged);
    let post = if needs_post(sc, &p) {
        let mut g = (*raw).clone();
        crate::render::post_process(sc, &p, &mut g, crate::render::thread_count(sc));
        Arc::new(g)
    } else {
        raw.clone()
    };
    let rgb = crate::render::paint(sc, &p, &post);
    e.publish(rgb, b.w, b.h, "", None);
    *e.base.lock().unwrap() = Some(Arc::new(Base { scene: sc.clone(), params: p, raw, post, post_sig: post_sig(sc), w: b.w, h: b.h }));
    Ok(())
}
