//! Ray marching on the graphics card (prototype, cargo feature `gpu`).
//!
//! The main calculation (`Marcher::march_pixel` for every pixel) runs as a
//! wgpu compute shader (`gpu_march.wgsl`: Vulkan, Metal or DirectX 12) and
//! fills the same G-buffer as the CPU, then runs the hard shadows and the
//! DE ambient occlusion on it (the painting stays on the CPU).  The shader
//! calculates in single precision: the alternating 3D hybrid of 'Integer
//! Power', 'Amazing Box' and the .m3f formulas the lifter translates to
//! WGSL (`x86::lift_wgsl`), with the numerical or the analytic DE (options
//! 0, 2, 11), dIFS hybrids (option 20) and the DE combinations of two
//! parts; [`unsupported`] says why a scene is calculated on the CPU.

use crate::calc::{CalcParams, PostJob};
use crate::formulas::Formula;
use crate::gbuffer::SiLight;
use crate::iteration::HybridMode;
use crate::scene::CameraOptic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);
static LAST: Mutex<String> = Mutex::new(String::new());
/// The last error wgpu reported outside an error scope.
static ERROR: Mutex<Option<String>> = Mutex::new(None);

fn set_error(e: String) {
    if let Ok(mut l) = ERROR.lock() {
        l.get_or_insert(e);
    }
}

fn take_error() -> Option<String> {
    ERROR.lock().ok().and_then(|mut l| l.take())
}

/// The tasks per dispatch row of the reflection pass (`R_W` in the shader).
const R_W: usize = 256;

/// The parameter words `R_SPLIT` (the reflection pass), `N_SLOTS`, `ITEM_END` (resumable marching).
const SLOTS_WORD: usize = 165;

/// The parameter word `TURN_BUDGET`.
const BUDGET_WORD: usize = 272;

/// Resumable marching (`STATE` in the shader): every thread is a slot for
/// a pixel (or task) of a job; a submission gives each a budget of turns
/// of the marcher's state machine, and pixels that need more save their
/// state and continue in the next submission while the slots of finished
/// pixels take new ones.  The budget follows the time a
/// submission took so that one lasts about [`Resumer::target`] (env
/// `MB3D_GPU_BAND_MS`, default 50: shorter pieces cost much more, each
/// has a fixed cost of a few ms): the desktop shares the card, its
/// compositor waits for a running submission, and in slow scenes (dIFS) a
/// single pixel takes 100 ms or more, so a submission that finished its
/// pixels froze the screen for seconds.
struct Resumer {
    state: wgpu::Buffer,
    queue: wgpu::Buffer,
    queue_read: wgpu::Buffer,
    slots: usize,
    budget: u32,
    fixed: bool,
    target: f64,
    timing: bool,
}

/// The [`Resumer`] of the reflection batches with the hash of their shader
/// (another scene starts with a new one: its turns may take much longer).
static REFLECT_RESUMER: std::sync::Mutex<Option<(u64, Resumer)>> = std::sync::Mutex::new(None);

/// Slots at most (about 1.3 KB of state each).
const RESUME_SLOTS: usize = 1 << 15;

/// The words of the marcher's state (`STATE_FIXED` in the shader).
fn state_fixed() -> u32 {
    let code = include_str!("gpu_march.wgsl");
    let i = code.find("const STATE_FIXED: u32 = ").expect("STATE_FIXED") + 25;
    code[i..].split('u').next().unwrap().parse().unwrap()
}

impl Resumer {
    fn new(g: &Gpu, sh: &SceneShader) -> Self {
        let stride = 1 + state_fixed() + sh.record_words;
        // within the default storage binding limit (128 MB)
        let max: usize = std::env::var("MB3D_GPU_SLOTS").ok().and_then(|v| v.parse().ok()).unwrap_or(RESUME_SLOTS);
        let slots = max.min((96 << 20) / (stride as usize * 4)).max(64) / 64 * 64;
        let state = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("state"),
            size: (slots * stride as usize * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let queue = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("queue"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let queue_read = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("queue read"), size: 16, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let ms: f64 = std::env::var("MB3D_GPU_BAND_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(50.0);
        // MB3D_GPU_TURNS: a fixed budget (a check of the suspending)
        let fixed: Option<u32> = std::env::var("MB3D_GPU_TURNS").ok().and_then(|v| v.parse().ok());
        Resumer { state, queue, queue_read, slots, budget: fixed.unwrap_or(64), fixed: fixed.is_some(), target: ms / 1000.0, timing: std::env::var_os("MB3D_GPU_TIMING").is_some() }
    }

    /// Runs a job of `items` pixels (tasks) until all are done: `base` is
    /// its parameter block (written to `par`), `encode` records its compute
    /// pass of the given work groups (x, 1), `done(n)` is called when the
    /// first `n` items are finished.  `limit(started, free)` gives before
    /// every submission how many of the items the card may start (from
    /// `started` up to `items`; the others are left to someone else; `free`
    /// slots are idle); `give_up(left)` with `left` items still running
    /// after a submission stops the job early.  Returns the number of items
    /// the card took and those of them it did not finish.
    #[allow(clippy::too_many_arguments)]
    fn run(
        &mut self,
        g: &Gpu,
        par: &wgpu::Buffer,
        mut base: Vec<u32>,
        items: usize,
        limit: &dyn Fn(usize, usize) -> usize,
        give_up: &dyn Fn(usize) -> bool,
        cancel: &dyn Fn() -> bool,
        encode: impl Fn(&mut wgpu::CommandEncoder, u32),
        mut done: impl FnMut(usize) -> Result<(), String>,
    ) -> Result<(usize, Vec<usize>), String> {
        let groups = self.slots / 64;
        base[SLOTS_WORD + 1] = self.slots as u32;
        let mut started;
        let mut lim = limit(0, self.slots).clamp(0, items);
        let mut enc = g.device.create_command_encoder(&Default::default());
        // the slots' first words (the pixel + 1): all free
        enc.clear_buffer(&self.state, 0, Some(self.slots as u64 * 4));
        g.queue.write_buffer(&self.queue, 0, &bytes(&[0, 0, u32::MAX, 0]));
        g.queue.submit([enc.finish()]);
        loop {
            if cancel() || ABORT.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("cancelled".into());
            }
            let t = std::time::Instant::now();
            base[SLOTS_WORD + 2] = lim as u32;
            base[BUDGET_WORD] = self.budget;
            g.queue.write_buffer(par, 0, &bytes(&base));
            g.queue.write_buffer(&self.queue, 0, &bytes(&[0]));
            g.queue.write_buffer(&self.queue, 8, &bytes(&[u32::MAX]));
            let mut enc = g.device.create_command_encoder(&Default::default());
            encode(&mut enc, groups as u32);
            enc.copy_buffer_to_buffer(&self.queue, 0, &self.queue_read, 0, 16);
            g.queue.submit([enc.finish()]);
            let slice = self.queue_read.slice(..);
            let (tx, rx) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
            if let Some(e) = take_error() {
                return Err(e);
            }
            rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            let q: Vec<u32> = slice.get_mapped_range().map_err(|e| e.to_string())?.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect();
            self.queue_read.unmap();
            let (left, next) = (q[0] as usize, (q[1] as usize).min(lim));
            started = next;
            let dt = t.elapsed().as_secs_f64();
            if self.timing {
                eprintln!("  submission (budget {}): {:.1} ms, {next} of {lim} started, {left} suspended", self.budget, dt * 1000.0);
            }
            // the first suspended item or the next to start: all before it
            // are finished
            done(if left > 0 { (q[2] as usize).min(next) } else { next })?;
            lim = limit(started, self.slots.saturating_sub(left)).clamp(started, items);
            if left == 0 && next >= lim {
                return Ok((lim, Vec::new()));
            }
            if left > 0 && next >= lim && give_up(left) {
                // the items of the busy slots (their first state word: the
                // item + 1)
                let read = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("slots read"), size: self.slots as u64 * 4, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
                let v = read_back(g, &self.state, &read, 0, self.slots as u64 * 4)?;
                let busy = v.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).filter(|&x| x != 0).map(|x| x as usize - 1).collect();
                return Ok((lim, busy));
            }
            if !self.fixed {
                // towards the target time of a submission: down at once in
                // proportion (a card that also drives the screen loses its
                // device when one submission runs for seconds), up only
                // while the budget cut work short (items that all finish
                // within it say nothing about a heavier batch next)
                let mut f = (self.target / dt.max(1e-4)).clamp(0.02, 2.0);
                if left == 0 {
                    f = f.min(1.0);
                }
                self.budget = ((self.budget as f64 * f) as u32).clamp(8, 1 << 24);
            }
        }
    }
}

/// Calculate on the graphics card when the scene allows it.  Off in the
/// library; the programs switch it on (see [`default_on`]).
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// The graphics card's name (None without a usable card).
pub fn name() -> Option<String> {
    gpu().ok().map(|g| g.name.clone())
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed) && !CPU_HERE.with(|c| c.get()) && !LOST.load(Ordering::Relaxed)
}

/// The graphics card failed (its device was lost, e.g. after a driver
/// timeout): everything is calculated on the CPU until the program ends.
static LOST: AtomicBool = AtomicBool::new(false);

/// Runs card work so that a failure of the card (wgpu panics when its
/// device is lost) becomes an error for the caller's CPU fallback and
/// switches the card off, instead of ending the calculation's thread.
fn guard<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let lost = |e: &str| {
        LOST.store(true, Ordering::SeqCst);
        set_status(format!("CPU: the graphics card failed ({e}); calculating on the CPU until the program is restarted"));
    };
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Err(e)) if e.to_ascii_lowercase().contains("lost") => {
            lost(&e);
            Err(e)
        }
        Ok(r) => r,
        Err(p) => {
            let e = p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_else(|| "a panic".into());
            let e = e.lines().last().unwrap_or("").trim().to_string();
            lost(&e);
            Err(e)
        }
    }
}

thread_local! {
    /// the calculations of this thread stay on the CPU (cpu_in_thread)
    static CPU_HERE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The calculations of the calling thread stay on the CPU while `on`
/// (MutaGen's previews: tiny images of ever new formula combinations,
/// where compiling each one's shader takes seconds and the CPU a fraction
/// of one).  Returns the previous setting.
pub fn cpu_in_thread(on: bool) -> bool {
    CPU_HERE.with(|c| c.replace(on))
}

/// The programs' default: on, unless the environment variable `MB3D_GPU`
/// is 0.
pub fn default_on() -> bool {
    std::env::var_os("MB3D_GPU").is_none_or(|v| v != "0")
}

/// How the last calculation ran: "GPU (name)" or "CPU: reason".
pub fn last_status() -> String {
    LAST.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Set when the program ends: running GPU work returns an error at its
/// next submission (a process that exits while a calculation waits for
/// the card crashes in the driver).
static ABORT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Stops all GPU work for good (the program ends).
pub fn abort_all() {
    ABORT.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn set_status(s: String) {
    // (a thread kept on the CPU does not speak for the main calculation)
    if CPU_HERE.with(|c| c.get()) {
        return;
    }
    if let Ok(mut l) = LAST.lock() {
        *l = s;
    }
}

/// Why the scene cannot be calculated by the shader, `None` if it can.
/// What keeps the passes other than the march (Monte Carlo, the reflection
/// pass) off the card: what keeps the march off, and the volumetric light
/// (the march's only).
pub fn unsupported_beyond_march(p: &CalcParams) -> Option<String> {
    if p.vol.is_some() {
        return Some("volumetric light (only the main calculation on the GPU yet)".into());
    }
    unsupported(p)
}

pub fn unsupported(p: &CalcParams) -> Option<String> {
    let r = |s: &str| Some(s.to_string());
    if p.slice_2d != 0 {
        return r("2D calculation (not on the GPU yet)");
    }
    if p.is_custom_de && !p.difs && ![2, 5, 6, 11].contains(&p.de_option) {
        return r("this analytic DE option (not on the GPU yet)");
    }
    if let Some(dc) = &p.decomb {
        let p2 = &dc.part2;
        if p2.is_custom_de && !p2.difs && ![2, 5, 6, 11].contains(&p2.de_option) {
            return r("this analytic DE option of the DE combination (not on the GPU yet)");
        }
    }
    if let Some(v) = &p.vol {
        // (its map goes with the maps into one buffer)
        let words = v.sides.iter().map(|s| s.len()).sum::<usize>() as u64;
        if gpu().is_ok_and(|g| (words + 4096) * 4 > g.device.limits().max_storage_buffer_binding_size as u64) {
            return r("volumetric light: its map is too big for the graphics card");
        }
    }
    // the zooms beyond single precision use the double mode (scene_shader,
    // needs_double; MB3D_GPU_MIN_PRECISION / MB3D_GPU_MIN_STEP: experiments
    // with the limits)
    scene_shader(p).err()
}

/// The parts of the shader and its buffers for one scene.
struct SceneShader {
    /// the complete WGSL
    code: String,
    /// the constants of all .m3f slots (u32)
    cst: Vec<u32>,
    /// the record image: for cell c the f32 (whole doubles) at 2c, or the
    /// two u32 halves at 2c, 2c + 1
    rtpl: Vec<u32>,
    /// per slot: iterations, uncounted, constants base, fHln bits, power,
    /// z multiplier bits, kind, 0
    slots: [[u32; 8]; 6],
    /// per slot: the parameters of the built-in formulas (f32 / i32 bits)
    slotx: [[u32; 8]; 6],
    /// the words of the record cells in a pixel's state slot
    record_words: u32,
    /// the maps of the formulas' map functions (the shader's MAPS)
    maps: Vec<u32>,
}

/// Record cells (offset / 8 from J4) with integers the loop copies to the
/// record before a .m3f call (`Iteration::run_custom`): they must be halves.
const LOOP_HALVES: [u32; 4] = [13, 15, 16, 33];

/// The type of the shader's real numbers (`R`) and its helpers: f32, or
/// f64 for the views single precision cannot resolve (`dbl`).  The state
/// slots keep two words per real value either way.
fn real_block(dbl: bool) -> String {
    let mut o = String::new();
    if dbl {
        o.push_str("alias R = f64;\nfn rtpl_r(i: u32) -> R { return bitcast<f64>((u64(rtpl[i + 1u]) << 32u) | u64(rtpl[i])); }\nfn rput(q: u32, ns: u32, x: R) { let b = bitcast<u64>(x); STATE[q] = u32(b & 0xFFFFFFFFlu); STATE[q + ns] = u32(b >> 32u); }\nfn rget(q: u32, ns: u32) -> R { return bitcast<f64>((u64(STATE[q + ns]) << 32u) | u64(STATE[q])); }\n");
    } else {
        o.push_str("alias R = f32;\nfn rtpl_r(i: u32) -> R { return bitcast<f32>(rtpl[i]); }\nfn rput(q: u32, ns: u32, x: R) { STATE[q] = bitcast<u32>(x); }\nfn rget(q: u32, ns: u32) -> R { return bitcast<f32>(STATE[q]); }\n");
    }
    // the positions of the reflection tasks: as doubles (words 24-29 and
    // 30-35 of the record) or the f32 words
    if dbl {
        o.push_str("fn out_d(q: u32) -> R { return bitcast<f64>((u64(OUT[q + 1u]) << 32u) | u64(OUT[q])); }\nfn task_origin(o: u32) -> vec3<R> { return vec3<R>(out_d(o + 24u), out_d(o + 26u), out_d(o + 28u)); }\nfn put_hit_pos(o: u32, v: vec3<R>) { for (var k = 0u; k < 3u; k++) { let b = bitcast<u64>(v[k]); OUT[o + 30u + 2u * k] = u32(b & 0xFFFFFFFFlu); OUT[o + 31u + 2u * k] = u32(b >> 32u); } }\nfn mc_pos(o: u32) -> vec3<R> { return vec3<R>(out_d(o + 13u), out_d(o + 15u), out_d(o + 17u)); }\nfn mc_put_pos(o: u32, v: vec3<R>) { for (var k = 0u; k < 3u; k++) { let b = bitcast<u64>(v[k]); OUT[o + 13u + 2u * k] = u32(b & 0xFFFFFFFFlu); OUT[o + 14u + 2u * k] = u32(b >> 32u); } }\n");
    } else {
        o.push_str("fn task_origin(o: u32) -> vec3<R> { return vec3<R>(bitcast<f32>(OUT[o]), bitcast<f32>(OUT[o + 1u]), bitcast<f32>(OUT[o + 2u])); }\nfn put_hit_pos(o: u32, v: vec3<R>) {}\nfn mc_pos(o: u32) -> vec3<R> { return vec3<R>(bitcast<f32>(OUT[o]), bitcast<f32>(OUT[o + 1u]), bitcast<f32>(OUT[o + 2u])); }\nfn mc_put_pos(o: u32, v: vec3<R>) {}\n");
    }
    // the coordinates: as doubles (DP: start, view gradients, julia
    // constant) or the f32 parameters
    if dbl {
        o.push_str("fn pdd(i: u32) -> R { return bitcast<f64>((u64(P[DP + 2u * i + 1u]) << 32u) | u64(P[DP + 2u * i])); }\n");
        o.push_str("fn p_ystart() -> vec3<R> { return vec3<R>(pdd(0u), pdd(1u), pdd(2u)); }\n");
        o.push_str("fn vg(i: u32) -> vec3<R> { return vec3<R>(pdd(3u + 3u * i), pdd(4u + 3u * i), pdd(5u + 3u * i)); }\n");
        o.push_str("fn p_ju() -> vec3<R> { return vec3<R>(pdd(12u), pdd(13u), pdd(14u)); }\nfn p_j4() -> R { return pdd(15u); }\n");
        o.push_str("fn vol_pos() -> vec3<R> { return vec3<R>(pdd(16u), pdd(17u), pdd(18u)); }\nfn hs_light_pos(i: u32) -> vec3<R> { return vec3<R>(pdd(19u + 3u * i), pdd(20u + 3u * i), pdd(21u + 3u * i)); }\n");
    } else {
        o.push_str("fn p_ystart() -> vec3<R> { return pv(YSTART); }\nfn vg(i: u32) -> vec3<R> { return pv(VG + i * 3u); }\nfn p_ju() -> vec3<R> { return pv(JU); }\nfn p_j4() -> R { return pf(J4); }\n");
        o.push_str("fn vol_pos() -> vec3<R> { return pv(VOL_POS); }\nfn hs_light_pos(i: u32) -> vec3<R> { return pv(HS_LIGHTS + i * 4u + 1u); }\n");
    }
    o
}

/// The scene would run in single precision, and the card has doubles.
pub fn double_possible(p: &CalcParams) -> bool {
    !needs_double(p) && gpu().is_ok_and(|g| g.f64)
}

thread_local! {
    /// the last march overflowed single precision and ran in double mode
    /// (until the next march on this thread)
    static RETRY_DOUBLE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The scene needs the shader's double mode: its zoom is beyond single
/// precision (see [`unsupported`]), or a probe found single precision too
/// coarse for its detail (`p.gpu_double`); MB3D_GPU_DOUBLE=1 forces it
/// (checks).
fn needs_double(p: &CalcParams) -> bool {
    if std::env::var("MB3D_GPU_DOUBLE").as_deref() == Ok("1") || RETRY_DOUBLE.with(|c| c.get()) || p.gpu_double {
        return true;
    }
    let env = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let numeric = !(p.is_custom_de || p.difs) || p.decomb.as_ref().is_some_and(|d| !(d.part2.is_custom_de || d.part2.difs));
    (numeric && precision(p) < env("MB3D_GPU_MIN_PRECISION", MIN_PRECISION)) || step_precision(p) < env("MB3D_GPU_MIN_STEP", MIN_STEP_PRECISION)
}

fn scene_shader(p: &CalcParams) -> Result<SceneShader, String> {
    use crate::x86::lift_wgsl::{prelude, record_globals, CellTy, WgslFormula};
    use std::fmt::Write;
    let dbl = needs_double(p);
    if dbl && !gpu()?.f64 {
        return Err("the zoom needs double precision (the graphics card has no doubles)".into());
    }
    let mut slots = [[0u32; 8]; 6];
    let mut slotx = [[0u32; 8]; 6];
    let mut wfs: Vec<(usize, WgslFormula)> = Vec::new();
    let mut cst: Vec<u32> = Vec::new();
    if p.slots.len() > 6 {
        return Err("more than 6 formulas".into());
    }
    // the slots called with the dIFS convention (doHybridIFS3D) and the
    // ones called as plain formulas; a slot can be in both parts
    let part2 = p.decomb.as_ref().map(|d| &d.part2);
    let in_part1 = |n: usize| n <= p.end_to;
    let in_part2 = |n: usize| part2.is_some_and(|q| (q.start..=q.end).contains(&n));
    let difs_slot = |n: usize| (in_part1(n) && p.difs) || (in_part2(n) && part2.is_some_and(|q| q.difs));
    // an interpolation hybrid iterates the first two formulas
    let ipol = has_ipol(p);
    let plain_slot = |n: usize| (in_part1(n) && !p.difs) || (in_part2(n) && part2.is_some_and(|q| !q.difs)) || (ipol && n < 2);
    for (n, sl) in p.slots.iter().enumerate() {
        let row = &mut slots[n];
        row[0] = sl.iterations as u32;
        row[1] = sl.uncounted as u32;
        row[3] = p.fhln[n].to_bits();
        if sl.iterations == 0 && !(ipol && n < 2) {
            continue;
        }
        if difs_slot(n) {
            let Formula::Custom(c) = &sl.formula else {
                return Err(format!("{} in a dIFS hybrid", sl.formula.name()));
            };
            let m = p.machine.as_ref().ok_or("no formula machine")?;
            let prog = m.prog(crate::custom::code_addr(n)).ok_or_else(|| format!("{}: not compiled", c.name()))?;
            let wf = prog.emit_wgsl(&format!("fd{n}"), true).ok_or_else(|| format!("{} is not on the GPU yet", c.name()))?;
            if wf.writes_vars {
                return Err(format!("{} keeps state in its variables", c.name()));
            }
            let vb = (crate::custom::var_buf_addr(n) - crate::x86::BASE) as usize;
            row[7] = cst.len() as u32;
            cst.extend(wf.constants_from(&m.mem[vb..vb + 0x400], dbl));
            row[6] |= 16; // the dIFS translation (the kind is in the low bits)
            wfs.push((n, wf));
            if !plain_slot(n) {
                continue;
            }
        }
        match &sl.formula {
            Formula::IntPow { power, z_mul } => {
                row[4] = *power as u32;
                row[5] = (*z_mul as f32).to_bits();
                row[6] = 1;
            }
            Formula::AmazingBox { scale, min_r, fold } => {
                row[2] = sl.ade as u32;
                row[4] = (*scale as f32).to_bits();
                row[5] = (*min_r as f32).to_bits();
                row[6] = 3;
                row[7] = (*fold as f32).to_bits();
            }
            Formula::Tricorn { z_mul, cz_mul } => {
                row[6] = 5;
                slotx[n] = [(*z_mul as f32).to_bits(), (*cz_mul as f32).to_bits(), 0, 0, 0, 0, 0, 0];
            }
            Formula::Bulbox { scale, min_r, fold, bulb_scale, r_threshold, r_threshold2 } => {
                row[6] = 6;
                let f = |x: &f64| (*x as f32).to_bits();
                slotx[n] = [f(scale), f(min_r), f(fold), f(bulb_scale), f(r_threshold), f(r_threshold2), 0, 0];
            }
            Formula::AexionC { power, z_mul, rot_c, cond_phi, power_c, cz_mul, powc_dist, mode } => {
                row[6] = 7;
                let f = |x: &f64| (*x as f32).to_bits();
                slotx[n] = [f(power), f(z_mul), *rot_c as u32, *cond_phi as u32, f(power_c), f(cz_mul), *powc_dist as u32, *mode as u32];
            }
            Formula::RealPower { power, z_mul } => {
                row[6] = 4;
                slotx[n] = [(*power as f32).to_bits(), (*z_mul as f32).to_bits(), 0, 0, 0, 0, 0, 0];
            }
            Formula::Quaternion { yw_mul, w_add } => {
                row[6] = 8;
                slotx[n] = [(*yw_mul as f32).to_bits(), (*w_add as f32).to_bits(), 0, 0, 0, 0, 0, 0];
            }
            Formula::FoldingIntPow { power, z_mul, fold } => {
                row[6] = 9;
                slotx[n] = [*power as u32, (*z_mul as f32).to_bits(), (*fold as f32).to_bits(), 0, 0, 0, 0, 0];
            }
            Formula::Custom(c) if c.def.jit.is_none() => {
                let m = p.machine.as_ref().ok_or("no formula machine")?;
                let prog = m.prog(crate::custom::code_addr(n)).ok_or_else(|| format!("{}: not compiled", c.name()))?;
                let wf = prog.emit_wgsl(&format!("fm{n}"), false).ok_or_else(|| format!("{} is not on the GPU yet", c.name()))?;
                if wf.writes_vars {
                    return Err(format!("{} keeps state in its variables", c.name()));
                }
                let vb = (crate::custom::var_buf_addr(n) - crate::x86::BASE) as usize;
                row[2] = cst.len() as u32;
                cst.extend(wf.constants_from(&m.mem[vb..vb + 0x400], dbl));
                row[6] |= 2;
                wfs.push((n, wf));
            }
            f => return Err(format!("the built-in formula {} is not on the GPU yet", f.name())),
        }
    }
    // one cell type per record cell for the whole shader: the bits form
    // (halves) wins; formulas that typed a cell differently are made again
    let mut unified: std::collections::BTreeMap<u32, CellTy> = Default::default();
    for (_, w) in &wfs {
        for (c, t) in &w.record {
            let e = unified.entry(*c).or_insert(*t);
            if *t == CellTy::Halves {
                *e = CellTy::Halves;
            }
        }
    }
    // the cells the loop copies as integers must keep their bits
    for c in LOOP_HALVES {
        if let Some(e) = unified.get_mut(&c) {
            *e = CellTy::Halves;
        }
    }
    for (n, w) in wfs.iter_mut() {
        if w.record.iter().any(|(c, t)| unified[c] != *t) {
            let prog = p.machine.as_ref().and_then(|m| m.prog(crate::custom::code_addr(*n))).ok_or("not compiled")?;
            let difs = w.code.contains(&format!("fn fd{n}("));
            let name = if difs { format!("fd{n}") } else { format!("fm{n}") };
            *w = prog.emit_wgsl_with(&name, difs, &unified).ok_or("formula typing")?;
        }
    }
    let refs: Vec<&WgslFormula> = wfs.iter().map(|(_, w)| w).collect();
    let globals = record_globals(&refs).ok_or("formulas use a record field in different ways")?;
    // the cell types of the shader
    let mut ty: std::collections::BTreeMap<u32, CellTy> = Default::default();
    for w in &refs {
        for (c, t) in &w.record {
            ty.insert(*c, *t);
        }
    }
    for (c, t) in &ty {
        if LOOP_HALVES.contains(c) && *t != CellTy::Halves {
            return Err("a formula uses an integer of the record as a double".into());
        }
    }
    // the record image from the CPU's prepared iteration (static parts)
    let it = p.new_iteration();
    let mut rtpl = vec![0u32; 128];
    if let Some(m) = it.emu.as_deref() {
        let b = (crate::custom::IT_BASE - crate::x86::BASE) as usize;
        for (c, t) in &ty {
            let o = b + 8 * *c as usize;
            let lo = u32::from_le_bytes(m.mem[o..o + 4].try_into().unwrap());
            let hi = u32::from_le_bytes(m.mem[o + 4..o + 8].try_into().unwrap());
            match t {
                CellTy::Whole if !dbl => rtpl[2 * *c as usize] = (f64::from_bits(lo as u64 | (hi as u64) << 32) as f32).to_bits(),
                _ => {
                    rtpl[2 * *c as usize] = lo;
                    rtpl[2 * *c as usize + 1] = hi;
                }
            }
        }
    }
    let mut g = String::new();
    g.push_str(&prelude(dbl));
    g.push_str(&globals);
    // init_record, marshal_in, marshal_out
    let has = |c: u32| ty.contains_key(&c);
    let _ = writeln!(g, "fn init_record() {{");
    for (c, t) in &ty {
        match t {
            CellTy::Whole => {
                let _ = writeln!(g, "    r{c}f = rtpl_r({}u);", 2 * c);
            }
            CellTy::Halves => {
                let _ = writeln!(g, "    r{c}l = rtpl[{}u]; r{c}h = rtpl[{}u];", 2 * c, 2 * c + 1);
            }
        }
    }
    let _ = writeln!(g, "}}");
    // the record cells in a pixel's state slot (resumable marching)
    let mut record_words = 0u32;
    let mut save = String::new();
    let mut load = String::new();
    for (c, t) in &ty {
        match t {
            CellTy::Whole => {
                let _ = writeln!(save, "    rput(o + {record_words}u * ns, ns, r{c}f);");
                let _ = writeln!(load, "    r{c}f = rget(o + {record_words}u * ns, ns);");
                record_words += 2;
            }
            CellTy::Halves => {
                let _ = writeln!(save, "    STATE[o + {record_words}u * ns] = r{c}l; STATE[o + {}u * ns] = r{c}h;", record_words + 1);
                let _ = writeln!(load, "    r{c}l = STATE[o + {record_words}u * ns]; r{c}h = STATE[o + {}u * ns];", record_words + 1);
                record_words += 2;
            }
        }
    }
    let _ = writeln!(g, "fn save_record(o: u32, ns: u32) {{\n{save}}}");
    let _ = writeln!(g, "fn load_record(o: u32, ns: u32) {{\n{load}}}");
    let ins: [(u32, &str); 20] = [
        (3, "r3f = v.x;"), (4, "r4f = v.y;"), (5, "r5f = v.z;"), (6, "r6f = w;"),
        (7, "r7f = c.x;"), (8, "r8f = c.y;"), (9, "r9f = c.z;"),
        (10, "r10f = j.x;"), (11, "r11f = j.y;"), (12, "r12f = j.z;"), (0, "r0f = j4;"),
        (1, "r1f = rold;"), (14, "r14f = rout;"), (31, "r31f = otrap;"), (32, "r32f = vary_scale;"),
        (34, "r34f = dfree.x;"), (35, "r35f = dfree.y;"), (36, "r36f = deriv.x;"), (37, "r37f = deriv.y;"), (38, "r38f = deriv.z;"),
    ];
    // a double cell in the bits form gets / gives the bits of the value
    let bits_in = |l: &str| -> String {
        let (lhs, rhs) = l.trim_end_matches(';').split_once(" = ").unwrap();
        let cell = &lhs[..lhs.len() - 1];
        format!("{cell}l = f64_lo({rhs}); {cell}h = f64_hi({rhs});")
    };
    let bits_out = |l: &str| -> String {
        let (lhs, rhs) = l.trim_end_matches(';').split_once(" = ").unwrap();
        let cell = &rhs[..rhs.len() - 1];
        format!("{lhs} = from_f64_bits({cell}h, {cell}l);")
    };
    let _ = writeln!(g, "fn marshal_in() {{");
    for (c, l) in ins {
        match ty.get(&c) {
            Some(CellTy::Whole) => {
                let _ = writeln!(g, "    {l}");
            }
            Some(CellTy::Halves) => {
                let _ = writeln!(g, "    {}", bits_in(l));
            }
            None => {}
        }
    }
    if has(15) {
        let _ = writeln!(g, "    r15l = bitcast<u32>(it_result); r15h = bitcast<u32>(max_it);");
    }
    if has(16) {
        let _ = writeln!(g, "    r16l = bitcast<u32>(rstop);");
    }
    if has(33) {
        let _ = writeln!(g, "    r33l = bitcast<u32>(first_it);");
    }
    let _ = writeln!(g, "}}");
    let outs: [(u32, &str); 16] = [
        (3, "v.x = r3f;"), (4, "v.y = r4f;"), (5, "v.z = r5f;"), (6, "w = r6f;"),
        (10, "j.x = r10f;"), (11, "j.y = r11f;"), (12, "j.z = r12f;"), (0, "j4 = r0f;"),
        (14, "rout = r14f;"), (31, "otrap = r31f;"), (32, "vary_scale = r32f;"),
        (34, "dfree.x = r34f;"), (35, "dfree.y = r35f;"), (36, "deriv.x = r36f;"), (37, "deriv.y = r37f;"), (38, "deriv.z = r38f;"),
    ];
    let _ = writeln!(g, "fn marshal_out() {{");
    for (c, l) in outs {
        match ty.get(&c) {
            Some(CellTy::Whole) => {
                let _ = writeln!(g, "    {l}");
            }
            Some(CellTy::Halves) => {
                let _ = writeln!(g, "    {}", bits_out(l));
            }
            None => {}
        }
    }
    if has(33) {
        let _ = writeln!(g, "    first_it = bitcast<i32>(r33l);");
    }
    let _ = writeln!(g, "}}");
    // difs_init: the record fields a dIFS loop sets once (hybrid_difs):
    // RStopD, BTmp, SmoothIt / InsideRender (not inside)
    // `ins`: inside (bCalcInside): the record's SmoothIt (high half of
    // cell 13) and InsideRender (cell 55) are -1, as doHybridIFS3D sets them
    let _ = writeln!(g, "fn difs_init(ins: bool) {{");
    match ty.get(&2) {
        Some(CellTy::Whole) => {
            let _ = writeln!(g, "    r2f = rstopd;");
        }
        Some(CellTy::Halves) => {
            let _ = writeln!(g, "    r2l = f64_lo(rstopd); r2h = f64_hi(rstopd);");
        }
        None => {}
    }
    if has(33) {
        let _ = writeln!(g, "    r33h = 0u;");
    }
    if has(13) {
        let _ = writeln!(g, "    r13h = select(0u, 0xFFFFFFFFu, ins);");
    }
    if let Some(CellTy::Halves) = ty.get(&55) {
        let _ = writeln!(g, "    r55l = select(0u, 0xFFFFFFFFu, ins); r55h = 0u;");
    }
    let _ = writeln!(g, "}}");
    for (n, w) in &wfs {
        // (reads through a pointer into the slot's variable buffer)
        g.push_str(&w.code.replace("@VARBUF@", &format!("{:#x}u", crate::custom::var_buf_addr(*n))));
    }
    // call_slot
    let x = crate::custom::IT_C1 - 32;
    let regs = format!(
        "rg = array<u32, 8>({:#x}u, {:#x}u, {:#x}u, 0u, {:#x}u, 0u, 0u, 0u);",
        x,
        x + 16,
        x + 8,
        crate::custom::STACK_TOP - 12
    );
    let _ = writeln!(g, "fn call_slot(n: u32) {{");
    let _ = writeln!(g, "    switch (n) {{");
    for (n, row) in slots.iter().enumerate() {
        match row[6] & 15 {
            1 => {
                let _ = writeln!(g, "        case {n}u: {{ v = int_pow(v, j, bitcast<i32>(slot({n}u, 4u)), R(bitcast<f32>(slot({n}u, 5u)))); }}");
            }
            2 => {
                // PVar: the slot's variables, as Iteration::run_custom sets it
                let pv = if has(13) { format!("r13l = {:#x}u; ", crate::custom::pconst_addr(n)) } else { String::new() };
                let _ = writeln!(g, "        case {n}u: {{ marshal_in(); {pv}{regs} fsw = 0u; fm{n}(slot({n}u, 2u)); marshal_out(); }}");
            }
            3 => {
                let _ = writeln!(g, "        case {n}u: {{ amazing_box({n}u); }}");
            }
            5 => {
                let _ = writeln!(g, "        case {n}u: {{ tricorn({n}u); }}");
            }
            6 => {
                let _ = writeln!(g, "        case {n}u: {{ bulbox({n}u); }}");
            }
            7 => {
                let _ = writeln!(g, "        case {n}u: {{ aexion_c({n}u); }}");
            }
            4 => {
                let _ = writeln!(g, "        case {n}u: {{ real_power({n}u); }}");
            }
            8 => {
                let _ = writeln!(g, "        case {n}u: {{ quaternion({n}u); }}");
            }
            9 => {
                let _ = writeln!(g, "        case {n}u: {{ folding_int_pow({n}u); }}");
            }
            _ => {}
        }
    }
    let _ = writeln!(g, "        default: {{}}");
    let _ = writeln!(g, "    }}");
    let _ = writeln!(g, "}}");
    // call_slot_difs: Machine::call_difs (esi = the record + 144, edx =
    // esi + 128, edi = the variables, ebx = the slot, ecx = the count)
    let _ = writeln!(g, "fn call_slot_difs(n: u32, cnt: u32) {{");
    let _ = writeln!(g, "    switch (n) {{");
    for (n, row) in slots.iter().enumerate() {
        if row[6] & 16 != 0 {
            let _ = writeln!(
                g,
                "        case {n}u: {{ marshal_in(); rg = array<u32, 8>(0u, cnt, {:#x}u, {n}u, {:#x}u, 0u, {:#x}u, {:#x}u); fsw = 0u; fd{n}(slot({n}u, 7u)); marshal_out(); }}",
                crate::custom::IT_BASE + 272,
                crate::custom::STACK_TOP - 4,
                crate::custom::IT_BASE + 144,
                crate::custom::pconst_addr(n)
            );
        }
    }
    let _ = writeln!(g, "        default: {{ rg[1] = cnt; rg[3] = n; }}");
    let _ = writeln!(g, "    }}");
    let _ = writeln!(g, "}}");
    let code = format!("{}{}", real_block(dbl), include_str!("gpu_march.wgsl").replace("//@FORMULAS@", &g));
    // MB3D_GPU_DUMP=file: write the scene shader (for debugging)
    if let Some(f) = std::env::var_os("MB3D_GPU_DUMP") {
        let _ = std::fs::write(f, &code);
    }
    // the maps: those named by an integer option of a formula that reads
    // maps (the map number is an option; unknown numbers give 0 as on the
    // CPU)
    let mut nrs: Vec<i32> = Vec::new();
    for (n, wf) in &wfs {
        if !wf.code.contains("map_px(") {
            continue;
        }
        if let Formula::Custom(c) = &p.slots[*n].formula {
            for (o, v) in c.def.options.iter().zip(&c.values) {
                let nr = v.round() as i32;
                if o.ty == 2 && !nrs.contains(&nr) && crate::maps::by_number(nr).is_some() {
                    nrs.push(nr);
                }
            }
        }
    }
    let mut maps = vec![nrs.len() as u32];
    let mut pixels: Vec<u32> = Vec::new();
    let head = 1 + 4 * nrs.len();
    for nr in &nrs {
        let lm = crate::maps::by_number(*nr).unwrap();
        maps.extend_from_slice(&[*nr as u32, lm.width as u32, lm.height as u32, (head + pixels.len()) as u32]);
        pixels.extend(lm.data().iter().flat_map(|p| p.map(f32::to_bits)));
    }
    maps.extend(pixels);
    Ok(SceneShader { code, cst, rtpl, slots, slotx, record_words, maps })
}

/// The smallest [`precision`] for the GPU: below it the single precision
/// noise shows (in the example Mandelbulb about 2.5% of the pixels differ
/// clearly from the CPU at 3, 4.5% at 1, noise everywhere below 0.1).
const MIN_PRECISION: f64 = 3.0;

/// How many single precision steps (at the size of the coordinates) the
/// offset of the numerical DE gradient spans: the larger, the less the
/// rounding of the shader shows.  It falls with the zoom (the pixel size).
pub fn precision(p: &CalcParams) -> f64 {
    p.de_offset as f64 / (coord_size(p) * f32::EPSILON as f64)
}

/// The size of the coordinates the shader works with.
fn coord_size(p: &CalcParams) -> f64 {
    p.ystart.iter().fold(0.0f64, |a, v| a.max(v.abs())) + p.step_width * (p.width + p.height) as f64 + 2.0
}

/// How many single precision steps a pixel step spans: the limit for the
/// scenes with an analytic DE (no gradient offset).
pub fn step_precision(p: &CalcParams) -> f64 {
    p.step_width / (coord_size(p) * f32::EPSILON as f64)
}

/// The smallest [`step_precision`] for the GPU.
const MIN_STEP_PRECISION: f64 = 64.0;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// compiled scene shaders by a hash of their code
    pipelines: Mutex<std::collections::HashMap<u64, std::sync::Arc<wgpu::ComputePipeline>>>,
    name: String,
    /// the shaders can calculate in doubles (SHADER_F64, SHADER_INT64)
    f64: bool,
}

/// The pipeline of the shader `code` (compiled once, then cached).
fn pipeline(g: &Gpu, code: &str, constants: &[(&str, f64)]) -> Result<std::sync::Arc<wgpu::ComputePipeline>, String> {
    pipeline_entry(g, code, "main", None, constants)
}

/// The pipeline of the entry point `entry` of the shader `code`; `layout`
/// when several pipelines share one bind group (the automatic layout of a
/// pipeline is exclusive to it).
fn pipeline_entry(g: &Gpu, code: &str, entry: &str, layout: Option<&wgpu::PipelineLayout>, constants: &[(&str, f64)]) -> Result<std::sync::Arc<wgpu::ComputePipeline>, String> {
    let key = pipeline_key(code, entry, constants);
    // compiled in the background (see [`prepare`]): wait for it
    let pending = PREPARING.lock().unwrap().iter().find(|(k, _)| *k == key).map(|(_, f)| f.clone());
    if let Some(f) = pending {
        while !f.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    compile_entry(g, code, entry, layout, constants, key)
}

/// [`pipeline_entry`] without the wait (the background compile itself).
fn compile_entry(g: &Gpu, code: &str, entry: &str, layout: Option<&wgpu::PipelineLayout>, constants: &[(&str, f64)], key: u64) -> Result<std::sync::Arc<wgpu::ComputePipeline>, String> {
    if let Some(pl) = g.pipelines.lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(pl.clone());
    }
    let scope = g.device.push_error_scope(wgpu::ErrorFilter::Validation);
    // MB3D_GPU_NOCACHE: a different shader text every time (the driver's
    // cache misses: the compile time of a new scene), MB3D_GPU_TIMING: print it
    let t = std::time::Instant::now();
    let salted;
    let code = if std::env::var_os("MB3D_GPU_NOCACHE").is_some() {
        // a different constant inside the entry point every time
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u32).unwrap_or(1);
        let at = code.find(&format!("fn {entry}(")).and_then(|i| code[..i].rfind("@compute")).unwrap_or(0);
        salted = format!("{}const SALT_X: u32 = {t}u;\n{}", &code[..at], &code[at..]).replacen("    _ = cst[0];", "    _ = cst[0] + SALT_X;", 1);
        &salted
    } else {
        code
    };
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(entry), source: wgpu::ShaderSource::Wgsl(code.into()) });
    let pl = g.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions { constants, ..Default::default() },
        cache: None,
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        return Err(format!("the shader does not compile: {e}"));
    }
    if std::env::var_os("MB3D_GPU_TIMING").is_some() {
        eprintln!("  shader {entry} {constants:?}: {} lines, compiled in {:.2} s", code.lines().count(), t.elapsed().as_secs_f64());
    }
    let pl = std::sync::Arc::new(pl);
    g.pipelines.lock().map_err(|e| e.to_string())?.insert(key, pl.clone());
    Ok(pl)
}

fn pipeline_key(code: &str, entry: &str, constants: &[(&str, f64)]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (code, entry).hash(&mut h);
    for (k, v) in constants {
        (k, v.to_bits()).hash(&mut h);
    }
    h.finish()
}

/// The pipelines being compiled in the background, with their flags.
static PREPARING: Mutex<Vec<(u64, std::sync::Arc<AtomicBool>)>> = Mutex::new(Vec::new());

/// A flag that turns true once the pipeline of `code` is compiled (at once
/// when it is): a thread of its own compiles it meanwhile.  A shader that
/// does not compile sets it too (its calculation then finds the error).
fn prepare(g: &'static Gpu, code: String, entry: &'static str, constants: Vec<(&'static str, f64)>) -> std::sync::Arc<AtomicBool> {
    let key = pipeline_key(&code, entry, &constants);
    if g.pipelines.lock().is_ok_and(|m| m.contains_key(&key)) {
        return std::sync::Arc::new(AtomicBool::new(true));
    }
    let mut list = PREPARING.lock().unwrap();
    if let Some((_, f)) = list.iter().find(|(k, _)| *k == key) {
        return f.clone();
    }
    let flag = std::sync::Arc::new(AtomicBool::new(false));
    list.push((key, flag.clone()));
    let f = flag.clone();
    std::thread::spawn(move || {
        crate::prio::calc_thread();
        let _ = compile_entry(g, &code, entry, None, &constants, key);
        f.store(true, Ordering::SeqCst);
        PREPARING.lock().unwrap().retain(|(k, _)| *k != key);
    });
    flag
}

/// The constants of the Monte Carlo pipeline.
fn mc_constants(p: &CalcParams) -> Vec<(&'static str, f64)> {
    let difs = p.difs || p.decomb.as_ref().is_some_and(|d| d.part2.difs);
    vec![("MODE_KIND", 4.0), ("HAS_DIFS", difs as u8 as f64), ("HAS_DECOMB", p.decomb.is_some() as u8 as f64), ("HAS_4D", four_d(p) as u8 as f64), ("HAS_IPOL", has_ipol(p) as u8 as f64)]
}

/// Monte Carlo on the card: a flag that turns true once its shader is
/// compiled (see [`prepare`]); an error when the scene cannot run there.
pub fn mc_prepare(p: &CalcParams) -> Result<std::sync::Arc<AtomicBool>, String> {
    if let Some(r) = unsupported_beyond_march(p) {
        return Err(r);
    }
    let g = gpu()?;
    let sh = scene_shader(p)?;
    Ok(prepare(g, sh.code, "main", mc_constants(p)))
}

/// The constants of the pipeline of a normal calculation's pass `mode`
/// (0 the march, 1 shadows, 2 DE ambient occlusion).
fn march_constants(p: &CalcParams, mode: u32) -> Vec<(&'static str, f64)> {
    let difs = p.difs || p.decomb.as_ref().is_some_and(|d| d.part2.difs);
    let kind = match mode {
        1 => 1.0,
        2 => 2.0,
        _ => 0.0,
    };
    vec![("MODE_KIND", kind), ("HAS_DIFS", difs as u8 as f64), ("HAS_DECOMB", p.decomb.is_some() as u8 as f64), ("HAS_4D", four_d(p) as u8 as f64), ("HAS_IPOL", has_ipol(p) as u8 as f64)]
}

/// The single precision march of the scene is compiled already (no
/// compile is started).
pub fn march_compiled(p: &CalcParams) -> bool {
    let (Ok(g), Ok(sh)) = (gpu(), scene_shader(p)) else { return false };
    let key = pipeline_key(&sh.code, "main", &march_constants(p, 0));
    g.pipelines.lock().is_ok_and(|m| m.contains_key(&key))
}

/// A key of the scene as the card marches it: its shader, the formulas'
/// constants and the march's parameters (the view, the size, the limits).
pub fn scene_key(p: &CalcParams) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let sh = scene_shader(p).ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (&sh.code, &sh.cst, &sh.rtpl, &sh.slots, &sh.slotx, &sh.maps).hash(&mut h);
    params(p, 0, p.rect[3].max(0) as usize, &sh, 0, &PostJob::default(), None).hash(&mut h);
    Some(h.finish())
}

/// A normal calculation on the card: flags that turn true once the
/// shaders of its passes are compiled (the march first; see [`prepare`]);
/// an error when the scene cannot run there.
pub fn march_prepare(p: &CalcParams, post: &PostJob) -> Result<Vec<std::sync::Arc<AtomicBool>>, String> {
    if let Some(r) = unsupported(p) {
        return Err(r);
    }
    let g = gpu()?;
    let sh = scene_shader(p)?;
    Ok(modes(post).into_iter().map(|m| prepare(g, sh.code.clone(), "main", march_constants(p, m))).collect())
}

/// The constants of a reflection pipeline: the march and/or the shadows
/// and occlusion of the hits.
fn reflect_constants(p: &CalcParams, march: bool, hs: bool, ao: bool) -> Vec<(&'static str, f64)> {
    let difs = p.difs || p.decomb.as_ref().is_some_and(|d| d.part2.difs);
    vec![
        ("MODE_KIND", 3.0),
        ("HAS_DIFS", difs as u8 as f64),
        ("HAS_DECOMB", p.decomb.is_some() as u8 as f64),
        ("HAS_4D", four_d(p) as u8 as f64),
        ("HAS_IPOL", has_ipol(p) as u8 as f64),
        ("R_HS", hs as u8 as f64),
        ("R_AO", ao as u8 as f64),
        ("R_MARCH", march as u8 as f64),
    ]
}

/// Starts compiling the reflection pass's pipelines in the background:
/// the march, and the shadows (`hs`) and occlusion (`ao`) of the hits.
pub fn reflect_prepare(p: &CalcParams, hs: bool, ao: bool) {
    if unsupported_beyond_march(p).is_some() {
        return;
    }
    let (Ok(g), Ok(sh)) = (gpu(), scene_shader(p)) else { return };
    let split = (hs || ao) && std::env::var_os("MB3D_GPU_NOSPLIT").is_none();
    if split {
        prepare(g, sh.code.clone(), "main", reflect_constants(p, true, false, false));
        prepare(g, sh.code, "main", reflect_constants(p, false, hs, ao));
    } else {
        prepare(g, sh.code, "main", reflect_constants(p, true, hs, ao));
    }
}

/// The march alone on the card, of the bands that `bands` hands out (the
/// CPU calculates the other rows): their rows go to `sink`.  An error
/// after rows went to `sink` (single precision overflows) means they are
/// not usable.
pub fn march_bands(p: &CalcParams, bands: &dyn Fn(usize) -> Option<(usize, usize)>, cancel: &(dyn Fn() -> bool + Sync), sink: &mut dyn FnMut(usize, &[SiLight])) -> Result<(), String> {
    guard(|| march_bands_card(p, bands, cancel, sink))
}

fn march_bands_card(p: &CalcParams, bands: &dyn Fn(usize) -> Option<(usize, usize)>, cancel: &(dyn Fn() -> bool + Sync), sink: &mut dyn FnMut(usize, &[SiLight])) -> Result<(), String> {
    let g = gpu()?;
    let sh = scene_shader(p)?;
    let v = run(g, p, &sh, &PostJob::default(), Some(0), &|_, _| {}, cancel, sink, Some(bands), None)?;
    // pixels whose DE overflowed single precision (see [`march`]): the
    // card's rows are not usable then
    let n = OVERFLOW.with(|c| c.get());
    if n * 500 > v.len() && !needs_double(p) {
        return Err(format!("single precision overflows in {} ({n} of {} pixels)", p.slots.iter().filter(|s| s.iterations != 0).map(|s| s.formula.name()).collect::<Vec<_>>().join(" + "), v.len()));
    }
    Ok(())
}

/// The march alone on the card of a sample of the pixels: every `sx`-th
/// column (from (row * 5) % sx) of every `sy`-th row (from sy / 2).  The
/// sample rows go to `sink` with their row in the image; their pixels are
/// the sampled columns.
pub fn march_sample(p: &CalcParams, sx: usize, sy: usize, cancel: &(dyn Fn() -> bool + Sync), sink: &mut dyn FnMut(usize, &[SiLight])) -> Result<(), String> {
    guard(|| march_sample_card(p, sx, sy, cancel, sink))
}

fn march_sample_card(p: &CalcParams, sx: usize, sy: usize, cancel: &(dyn Fn() -> bool + Sync), sink: &mut dyn FnMut(usize, &[SiLight])) -> Result<(), String> {
    let (sx, sy) = (sx.max(1), sy.max(1));
    let mut q = p.clone();
    q.rect[2] = (p.rect[2].max(0) as usize / sx) as i32;
    q.rect[3] = (p.rect[3].max(0) as usize / sy) as i32;
    if q.rect[2] == 0 || q.rect[3] == 0 {
        return Ok(());
    }
    let g = gpu()?;
    let sh = scene_shader(&q)?;
    SAMPLE.with(|c| c.set((sx, sy)));
    let r = run(g, &q, &sh, &PostJob::default(), Some(0), &|_, _| {}, cancel, &mut |y, row| sink(y * sy + sy / 2, row), None, None);
    SAMPLE.with(|c| c.set((0, 0)));
    let v = r?;
    // pixels whose DE overflowed single precision (see [`march_bands`])
    let n = OVERFLOW.with(|c| c.get());
    if n * 500 > v.len() && !needs_double(p) {
        return Err("single precision overflows".into());
    }
    Ok(())
}

/// The post passes of `post` (shadows, DE ambient occlusion) on the card
/// for a G-buffer marched elsewhere (the rows the CPU took in a shared
/// march): it is uploaded band by band and gets the results.
pub fn post_on_card(p: &CalcParams, gbuf: &mut [SiLight], post: &PostJob, cancel: &(dyn Fn() -> bool + Sync)) -> Result<(), String> {
    guard(|| {
        let g = gpu()?;
        let sh = scene_shader(p)?;
        let w = p.rect[2].max(0) as usize;
        let mut done: Vec<(usize, Vec<SiLight>)> = Vec::new();
        run(g, p, &sh, post, None, &|_, _| {}, cancel, &mut |y, row| done.push((y, row.to_vec())), None, Some(&*gbuf))?;
        for (y, row) in done {
            gbuf[y * w..y * w + row.len()].copy_from_slice(&row);
        }
        Ok(())
    })
}

/// "Calculate on: Auto": the CPU and the card share a calculation - the
/// CPU starts at once, the card joins when its shader is compiled and
/// then runs the post passes (otherwise the calculation waits for the
/// shader and runs on the card alone).
static AUTO: AtomicBool = AtomicBool::new(true);

pub fn set_auto(on: bool) {
    AUTO.store(on, Ordering::Relaxed);
}

pub fn auto() -> bool {
    AUTO.load(Ordering::Relaxed)
}

fn gpu() -> Result<&'static Gpu, String> {
    static G: OnceLock<Result<Gpu, String>> = OnceLock::new();
    G.get_or_init(|| pollster::block_on(init())).as_ref().map_err(|e| e.clone())
}

async fn init() -> Result<Gpu, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no graphics card found: {e}"))?;
    let info = adapter.get_info();
    if info.device_type == wgpu::DeviceType::Cpu {
        // llvmpipe, WARP: slower than the CPU renderer
        return Err(format!("only a software renderer ({})", info.name));
    }
    // doubles in shaders (Vulkan, DirectX 12; not Metal): the deep zooms
    let dbl = wgpu::Features::SHADER_F64 | wgpu::Features::SHADER_INT64;
    let f64 = adapter.features().contains(dbl);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("mb3d"),
            required_limits: adapter.limits(),
            required_features: if f64 { dbl } else { wgpu::Features::empty() },
            ..Default::default()
        })
        .await
        .map_err(|e| format!("{}: {e}", info.name))?;
    // errors (a shader the driver cannot compile, a lost device) must not
    // end the program: they are kept and the calculation goes to the CPU
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| set_error(e.to_string())));
    Ok(Gpu { device, queue, pipelines: Mutex::new(Default::default()), name: format!("{} ({:?})", info.name, info.backend), f64 })
}

/// The parameters in the order of the indices in `gpu_march.wgsl`.
/// The words of the parameter buffer (`P` in the shader) for a band of
/// rows; `mode` 0 is the march, 1 the shadow pass and 2 the ambient
/// occlusion of `post`.
fn params(p: &CalcParams, row0: usize, rows: usize, scene: &SceneShader, mode: u32, post: &PostJob, reflect: Option<(&ReflectOpts, usize)>) -> Vec<u32> {
    let slots = &scene.slots;
    let probe_mzz = PROBE_MZZ.with(|m| m.get());
    let (power, z_mul) = match p.slots[0].formula {
        Formula::IntPow { power, z_mul } => (power, z_mul),
        _ => (8, -1.0),
    };
    let optic = match p.optic {
        CameraOptic::Common => 0,
        CameraOptic::Planar => 1,
        CameraOptic::Panorama => 2,
    };
    let mut v: Vec<u32> = Vec::with_capacity(64);
    let i = |v: &mut Vec<u32>, x: i32| v.push(x as u32);
    let f = |v: &mut Vec<u32>, x: f64| v.push((x as f32).to_bits());
    i(&mut v, p.rect[0]);
    i(&mut v, p.rect[1]);
    i(&mut v, p.rect[2]);
    i(&mut v, row0 as i32);
    i(&mut v, rows as i32);
    i(&mut v, p.height);
    i(&mut v, optic);
    f(&mut v, p.fov_y);
    f(&mut v, p.fovx_off as f64);
    f(&mut v, p.fovx_mul as f64);
    f(&mut v, p.pl_optic_z as f64);
    for r in &p.vgrads {
        for &x in r {
            f(&mut v, x);
        }
    }
    for &x in &p.ystart {
        f(&mut v, x);
    }
    i(&mut v, p.max_it);
    i(&mut v, p.min_it);
    f(&mut v, p.d_rstop);
    f(&mut v, p.rstop3d);
    i(&mut v, p.do_julia as i32);
    for &x in &p.ju[..3] {
        f(&mut v, x);
    }
    i(&mut v, power);
    f(&mut v, z_mul);
    f(&mut v, p.de_stop as f64);
    f(&mut v, p.de_stop_factor as f64);
    f(&mut v, p.z_step_div as f64);
    f(&mut v, p.ms_de_sub as f64);
    f(&mut v, p.mh04zsd as f64);
    i(&mut v, p.dfog_on_it as i32);
    i(&mut v, p.first_step_random as i32);
    // the post passes use their own binary search depth
    i(&mut v, [p.de_add_steps, 8, 5, p.de_add_steps, p.de_add_steps, p.de_add_steps][mode as usize]);
    i(&mut v, p.normals_on_de as i32);
    i(&mut v, p.sm_normals);
    f(&mut v, p.de_offset as f64);
    f(&mut v, p.de_offset006 as f64);
    f(&mut v, p.d_de_scale as f64);
    f(&mut v, p.zcorr);
    f(&mut v, p.zc_mul);
    f(&mut v, p.zend);
    f(&mut v, p.col_var_de_stop_mul as f64);
    f(&mut v, p.d_col_plus as f64);
    f(&mut v, p.mcts_m as f64);
    f(&mut v, p.step_width);
    i(&mut v, p.color_option as i32);
    f(&mut v, p.mct_color_mul as f64);
    f(&mut v, p.ln_rstop as f64);
    f(&mut v, p.fhln[0] as f64);
    i(&mut v, p.is_custom_de as i32);
    i(&mut v, p.de_option);
    i(&mut v, p.end_to as i32);
    i(&mut v, p.repeat_from as i32);
    f(&mut v, p.ju[3]);
    while v.len() < 64 {
        v.push(0);
    }
    for row in slots {
        v.extend_from_slice(row);
    }
    debug_assert_eq!(v.len(), 112);
    i(&mut v, mode as i32);
    let sh = post.shadows.as_ref();
    let ao = post.deao.as_ref();
    match reflect {
        Some((o, _)) => {
            i(&mut v, o.lights.len() as i32);
            f(&mut v, o.hs_mul as f64);
            f(&mut v, o.soft_radius as f64);
            i(&mut v, o.ao.map(|a| a.0).unwrap_or(-1));
            i(&mut v, 0);
            f(&mut v, o.ao.map(|a| a.1).unwrap_or(1.0) as f64);
            i(&mut v, o.ao.map(|a| a.2).unwrap_or(false) as i32);
        }
        None => {
            i(&mut v, sh.map(|s| s.lights.len()).unwrap_or(0) as i32);
            f(&mut v, sh.map(|s| s.max_len_mul).unwrap_or(1.0) as f64);
            f(&mut v, sh.map(|s| s.soft_radius).unwrap_or(0.0) as f64);
            i(&mut v, ao.map(|d| d.quality).unwrap_or(0) as i32);
            i(&mut v, ao.map(|d| d.dither).unwrap_or(0) as i32);
            f(&mut v, ao.map(|d| d.max_len).unwrap_or(1.0) as f64);
            i(&mut v, ao.map(|d| d.first_step_random).unwrap_or(false) as i32);
        }
    }
    i(&mut v, p.width);
    while v.len() < 124 {
        v.push(0);
    }
    for k in 0..6 {
        match sh.and_then(|s| s.lights.get(k)) {
            Some(l) => {
                i(&mut v, l.idx as i32);
                // (a positional light: its position, see HS_POS)
                for &x in &l.pos.map_or(l.vec, |(q, _)| q) {
                    f(&mut v, x);
                }
            }
            None => v.extend_from_slice(&[0; 4]),
        }
    }
    debug_assert_eq!(v.len(), 148);
    // dIFS and the DE combination
    i(&mut v, p.difs as i32);
    i(&mut v, 0); // START_FROM of the first part
    match &p.decomb {
        Some(dc) => {
            let q = &dc.part2;
            i(&mut v, dc.kind as i32);
            f(&mut v, dc.smooth);
            f(&mut v, dc.fmix_pow);
            i(&mut v, dc.mix_col as i32);
            i(&mut v, q.start as i32);
            i(&mut v, q.end as i32);
            i(&mut v, q.repeat as i32);
            i(&mut v, q.max_it);
            i(&mut v, q.de_option);
            i(&mut v, q.is_custom_de as i32);
            i(&mut v, q.difs as i32);
            f(&mut v, q.d_de_scale as f64);
        }
        None => v.extend_from_slice(&[0; 12]),
    }
    f(&mut v, probe_mzz);
    i(&mut v, std::env::var_os("MB3D_PROBE_DETAIL").is_some() as i32);
    i(&mut v, 16);
    // resumable marching (`Resumer::run` fills them in)
    debug_assert_eq!(v.len(), SLOTS_WORD);
    v.extend_from_slice(&[0; 3]);
    while v.len() < 168 {
        v.push(0);
    }
    // the reflection pass
    match reflect {
        Some((o, n)) => {
            i(&mut v, o.far_fog as i32);
            i(&mut v, o.surface as i32);
            i(&mut v, n as i32);
            i(&mut v, 0);
            for k in 0..6 {
                match o.lights.get(k) {
                    Some(l) => {
                        i(&mut v, l.idx as i32);
                        for &x in &l.vec {
                            f(&mut v, x as f64);
                        }
                        i(&mut v, l.positional as i32);
                        f(&mut v, l.lmax_l as f64);
                        i(&mut v, l.visible as i32);
                        i(&mut v, 0);
                    }
                    None => v.extend_from_slice(&[0; 8]),
                }
            }
        }
        None => v.resize(220, 0),
    }
    // a sample of the pixels (see [`march_sample`])
    let (sx, sy) = SAMPLE.with(|c| c.get());
    i(&mut v, sx as i32);
    i(&mut v, sy as i32);
    v.resize(224, 0);
    // the parameters of the built-in formulas
    for row in &scene.slotx {
        v.extend_from_slice(row);
    }
    // the turn budget (`Resumer::run`), the kinds of the parts' hybrids
    // and the 4D rotation of the 4D hybrids
    debug_assert_eq!(v.len(), BUDGET_WORD);
    i(&mut v, 0);
    i(&mut v, hybrid_kind(p.mode));
    i(&mut v, p.decomb.as_ref().map_or(0, |d| hybrid_kind(d.part2.mode)));
    for r in &p.smatrix4 {
        for &x in &r[..3] {
            f(&mut v, x);
        }
    }
    // the weights of the two formulas of an interpolation hybrid
    f(&mut v, p.ipol[0] as f64);
    f(&mut v, p.ipol[1] as f64);
    // inside rendering (1) / inside and outside (2), colour on iteration,
    // the cutting planes (options with the sides in bits 4-6, positions)
    i(&mut v, p.inside_rendering as i32 | (p.in_and_outside as i32) << 1);
    i(&mut v, p.color_on_it as i32);
    i(&mut v, p.cut_options as i32);
    for &x in &p.cut_pos {
        f(&mut v, x);
    }
    // the coordinates as doubles (the shader's double mode): start position,
    // view gradients, julia constant
    v.resize(DOUBLE_WORD, 0);
    let mut d = |x: f64| {
        let b = x.to_bits();
        v.push(b as u32);
        v.push((b >> 32) as u32);
    };
    for &x in &p.ystart {
        d(x);
    }
    for r in &p.vgrads {
        for &x in r {
            d(x);
        }
    }
    for &x in &p.ju {
        d(x);
    }
    // the volumetric light (the march only: VOL_ON and the rest, its map
    // after the scene's maps, see `vol_maps`)
    let vol = p.vol.as_deref().filter(|_| mode == 0);
    for k in 0..3 {
        d(vol.map_or(0.0, |m| m.light_pos[k]));
    }
    // the shadow lights' positions
    let hs = post.shadows.as_ref();
    for li in 0..6 {
        let pos = hs.and_then(|j| j.lights.get(li)).and_then(|l| l.pos).map_or([0.0; 3], |(q, _)| q);
        for k in 0..3 {
            d(pos[k]);
        }
    }
    debug_assert_eq!(v.len(), 370);
    if let Some(m) = vol {
        let f = |v: &mut Vec<u32>, x: f32| v.push(x.to_bits());
        v.push(1);
        for k in 0..3 {
            f(&mut v, m.light_pos[k] as f32);
        }
        for r in &m.rot {
            for &x in r {
                f(&mut v, x);
            }
        }
        v.push(m.cube_size as u32);
        v.push(m.half_size as u32);
        f(&mut v, m.size_factor);
        f(&mut v, m.stretch);
        v.push(m.is_pos_light as u32);
        f(&mut v, m.vl_mul);
        f(&mut v, m.vl_step_mul);
        v.push(scene.maps.len().max(1) as u32);
    }
    v.resize(391, 0);
    for li in 0..6 {
        match hs.and_then(|j| j.lights.get(li)) {
            Some(l) => {
                v.push(l.pos.is_some() as u32);
                v.push((l.pos.map_or(0.0, |(_, m)| m) as f32).to_bits());
                v.push(l.visible as u32);
            }
            None => v.extend_from_slice(&[0; 3]),
        }
    }
    while v.len() < PARAM_WORDS {
        v.push(0);
    }
    v
}

/// The scene's maps (`SceneShader::maps`) with the volumetric light's map
/// after them (`VOL_BASE`: the shader's buffer of the maps has at least one
/// word)
fn vol_maps(p: &CalcParams, sh: &SceneShader) -> Vec<u32> {
    let mut m = if sh.maps.is_empty() { vec![0u32] } else { sh.maps.clone() };
    if let Some(v) = &p.vol {
        for side in &v.sides {
            m.extend(side.iter().map(|x| x.to_bits()));
        }
    }
    m
}

/// The kind of a hybrid part for the shader (`HYB_KIND`): 0 alternating
/// 3D, 1 alternating 4D, 2 interpolation 3D, 3 interpolation 4D.
fn hybrid_kind(m: HybridMode) -> i32 {
    match m {
        HybridMode::Alt3D => 0,
        HybridMode::Alt4D => 1,
        HybridMode::Ipol3D => 2,
        HybridMode::Ipol4D => 3,
    }
}

/// A part of the hybrid is a 4D hybrid (the shader's HAS_4D).
fn four_d(p: &CalcParams) -> bool {
    let k = |m: HybridMode| matches!(m, HybridMode::Alt4D | HybridMode::Ipol4D);
    k(p.mode) || p.decomb.as_ref().is_some_and(|d| k(d.part2.mode))
}

/// A part of the hybrid is an interpolation hybrid (the shader's HAS_IPOL).
fn has_ipol(p: &CalcParams) -> bool {
    let k = |m: HybridMode| matches!(m, HybridMode::Ipol3D | HybridMode::Ipol4D);
    k(p.mode) || p.decomb.as_ref().is_some_and(|d| k(d.part2.mode))
}

/// The size of the parameter buffer.
const PARAM_WORDS: usize = 416;

/// The parameter word of the coordinates as doubles (`DP` in the shader).
const DOUBLE_WORD: usize = 296;

thread_local! {
    /// the depth of [`de_probe`] (a parameter of the shader)
    static PROBE_MZZ: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    /// the columns and rows of a sample of the pixels (0: all)
    static SAMPLE: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
    /// the pixels of the last [`run`] whose DE overflowed single precision
    static OVERFLOW: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn bytes(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Calculates the G-buffer of `p.rect` on the graphics card, with the
/// post calculations of `post` (shadows, ambient occlusion).  Returns
/// `None` when the scene or the computer cannot use it (the reason is in
/// [`last_status`]); the caller then calculates on the CPU.
pub fn march(
    p: &CalcParams,
    post: &PostJob,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    sink: impl FnMut(usize, &[SiLight]),
) -> Option<Vec<SiLight>> {
    guard(|| Ok(march_card(p, post, progress, cancel, sink))).ok().flatten()
}

fn march_card(
    p: &CalcParams,
    post: &PostJob,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    mut sink: impl FnMut(usize, &[SiLight]),
) -> Option<Vec<SiLight>> {
    if let Some(r) = unsupported(p) {
        set_status(format!("CPU: {r}"));
        return None;
    }
    let timing = std::env::var_os("MB3D_GPU_TIMING").is_some();
    let t = std::time::Instant::now();
    let g = match gpu() {
        Ok(g) => g,
        Err(e) => {
            set_status(format!("CPU: {e}"));
            return None;
        }
    };
    if timing {
        eprintln!("  GPU device: {:.2} s", t.elapsed().as_secs_f64());
    }
    RETRY_DOUBLE.with(|c| c.set(false));
    let mut r: Result<Vec<SiLight>, String> = Err(String::new());
    for attempt in 0..2 {
        let t = std::time::Instant::now();
        let sh = match scene_shader(p) {
            Ok(s) => s,
            Err(e) => {
                set_status(format!("CPU: {e}"));
                return None;
            }
        };
        if timing {
            eprintln!("  scene shader: {:.2} s", t.elapsed().as_secs_f64());
        }
        // the passes' pipelines compile in parallel
        for m in modes(post) {
            prepare(g, sh.code.clone(), "main", march_constants(p, m));
        }
        let t = std::time::Instant::now();
        r = run(g, p, &sh, post, None, progress, cancel, &mut sink, None, None);
        if timing {
            eprintln!("  GPU run: {:.2} s", t.elapsed().as_secs_f64());
        }
        // pixels whose DE overflowed single precision: a formula not made
        // for f32 (folds of huge values); a few are harmless, many are not.
        // In double mode a NaN DE is the formula's own, as on the CPU, which
        // takes it as a near surface too.
        let overflow = match &r {
            Ok(v) => {
                let n = OVERFLOW.with(|c| c.get());
                if timing {
                    eprintln!("  DE overflow in {n} of {} pixels", v.len());
                }
                n * 500 > v.len() && !needs_double(p)
            }
            Err(_) => false,
        };
        if !overflow {
            break;
        }
        let n = OVERFLOW.with(|c| c.get());
        let len = r.as_ref().map(|v| v.len()).unwrap_or(0);
        r = Err(format!("single precision overflows in {} ({n} of {len} pixels)", p.slots.iter().filter(|s| s.iterations != 0).map(|s| s.formula.name()).collect::<Vec<_>>().join(" + ")));
        // (MB3D_GPU_RETRY_DOUBLE=1: the NVIDIA driver crashes compiling the
        // double shader of some of these formulas, so not by default)
        if attempt > 0 || !g.f64 || std::env::var_os("MB3D_GPU_RETRY_DOUBLE").is_none() {
            break;
        }
        // again in double mode (it stays for the passes of this picture
        // on this thread: reflections)
        RETRY_DOUBLE.with(|c| c.set(true));
    }
    match r {
        Ok(v) => {
            let mut s = format!("GPU {}", g.name);
            if post.shadows.is_some() {
                s.push_str(" +shadows");
            }
            if post.deao.is_some() {
                s.push_str(" +AO");
            }
            set_status(s);
            Some(v)
        }
        Err(e) => {
            set_status(format!("CPU: GPU error: {e}"));
            None
        }
    }
}

/// The passes of a band: the march and the post passes of `post`.
fn modes(post: &PostJob) -> Vec<u32> {
    let mut m = vec![0];
    if post.shadows.is_some() {
        m.push(1);
    }
    if post.deao.is_some() {
        m.push(2);
    }
    m
}

/// One pixel of [`de_probe`]: the DE in step widths, the iteration count,
/// 0, and the two parts of a DE combination with their iteration counts.
pub type DeProbe = (f32, i32, i32, [(f32, i32); 2]);

/// A task of the reflection pass: a bounce from `origin` along `dir` at
/// the view depth `zz` for the pixel `pixel` (its index in the rect); with
/// [`ReflectOpts::surface`] the pixel's own surface (`zz` its G-buffer
/// depth `zpos_fine >> 8`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ReflectTask {
    pub origin: [f64; 3],
    pub dir: [f32; 3],
    pub zz: f32,
    pub pixel: u32,
    /// the ray runs inside the set (inside rendering: the DE inverted)
    pub inside: bool,
    /// the quality of the ambient occlusion at the hit (0..3, `deao_1pos`;
    /// the bounces use lower ones)
    pub ao_q: u8,
}

/// The result of a task: the G-buffer value of the hit (normal, depth,
/// colour, shadow bits, ambient occlusion), its position, the view depth,
/// the distance along the ray, and what happened.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReflectHit {
    pub si: SiLight,
    pub pos: [f64; 3],
    pub zz: f32,
    pub zz2: f32,
    pub hit: bool,
    pub de_limited: bool,
    /// a surface task: the pixel keeps its painted colour
    pub skip: bool,
    pub step_count: f32,
}

/// A light of the reflection pass's hard shadows: `vec` is the `HSvec`
/// of a global light or the position of a positional one.
#[derive(Clone, Debug)]
pub struct ReflectLight {
    pub idx: u32,
    pub vec: [f32; 3],
    pub positional: bool,
    pub lmax_l: f32,
    pub visible: u32,
}

/// The settings of a reflection batch.
#[derive(Clone, Debug, Default)]
pub struct ReflectOpts {
    pub surface: bool,
    pub lights: Vec<ReflectLight>,
    /// > 0: one soft shadow of this radius
    pub soft_radius: f32,
    pub hs_mul: f32,
    /// DE ambient occlusion: quality, maximum length, random first step
    pub ao: Option<(i32, f32, bool)>,
    pub far_fog: bool,
}

/// The tasks of a reflection batch shared with the CPU: (limit, cpu_next)
/// under a lock.  The card starts the tasks below `limit`, raising it before
/// each submission by the slots that became free; the CPU takes them from
/// the end (`cpu_next`, going down, never below `limit`).  `done`: the card
/// is finished, the limit final.
#[derive(Debug)]
pub struct RaySharing {
    pub state: Mutex<(usize, usize)>,
    pub done: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// when no more than this many rays are left on the card (and the CPU
    /// has none) the card gives them to the CPU: returned as unfinished
    pub tail: usize,
    /// the card starts no rays beyond this one while it is busy (the CPU's
    /// share at the end; usize::MAX: none)
    pub cap: usize,
}

/// Runs the tasks of the reflection pass on the graphics card (the march
/// of every bounce with the normals, colour, shadows and ambient occlusion
/// of its hit, as `reflect.rs` does on the CPU).
pub fn reflect_batch(p: &CalcParams, tasks: &[ReflectTask], o: &ReflectOpts, share: Option<&RaySharing>) -> Result<(Vec<ReflectHit>, usize, Vec<usize>), String> {
    guard(|| reflect_batch_card(p, tasks, o, share))
}

fn reflect_batch_card(p: &CalcParams, tasks: &[ReflectTask], o: &ReflectOpts, share: Option<&RaySharing>) -> Result<(Vec<ReflectHit>, usize, Vec<usize>), String> {
    if tasks.is_empty() {
        if let Some(sh) = share {
            sh.done.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        return Ok((Vec::new(), 0, Vec::new()));
    }
    if let Some(r) = unsupported_beyond_march(p) {
        return Err(r);
    }
    let dbl = needs_double(p);
    let g = gpu()?;
    let sh = scene_shader(p)?;
    // the shadow and occlusion states only where the batch uses them, and
    // then in a pipeline of their own after the march (MB3D_GPU_NOSPLIT:
    // one pipeline, for comparisons): smaller shaders keep more rays on
    // the card at once
    let r_hs = !o.lights.is_empty() || o.soft_radius > 0.0;
    let r_ao = o.ao.is_some();
    let split = (r_hs || r_ao) && std::env::var_os("MB3D_GPU_NOSPLIT").is_none();
    let kind = |march: bool, hs: bool, ao: bool| pipeline(g, &sh.code, &reflect_constants(p, march, hs, ao));
    let mut pls = vec![if split { kind(true, false, false)? } else { kind(true, r_hs, r_ao)? }];
    if split {
        pls.push(kind(false, r_hs, r_ao)?);
    }
    let n = tasks.len();
    let words: Vec<u32> = tasks
        .iter()
        .flat_map(|t| {
            let mut w = [0u32; 36];
            for k in 0..3 {
                w[k] = (t.origin[k] as f32).to_bits();
                w[3 + k] = t.dir[k].to_bits();
                let b = t.origin[k].to_bits();
                (w[24 + 2 * k], w[25 + 2 * k]) = (b as u32, (b >> 32) as u32);
            }
            w[6] = t.zz.to_bits();
            w[7] = t.pixel;
            w[23] = t.inside as u32 | (t.ao_q as u32) << 8;
            w
        })
        .collect();
    let size = (words.len() * 4) as u64;
    let buf = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("reflection tasks"), size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    g.queue.write_buffer(&buf, 0, &bytes(&words));
    let read = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("reflection read"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let make = |label: &'static str, d: &[u32]| {
        let d = if d.is_empty() { vec![0u32] } else { d.to_vec() };
        let b = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (d.len() * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        g.queue.write_buffer(&b, 0, &bytes(&d));
        b
    };
    let cst = make("constants", &sh.cst);
    let rtpl = make("record", &sh.rtpl);
    let maps = make("maps", &sh.maps);
    // the tasks in rows of R_W, resumable (a bounce marches a ray and
    // casts the shadow and occlusion rays of its hit)
    // the pacing from the last batch (its buffers and turn budget): the
    // reflection pass sends many batches, a new one each time ramped up
    // from a small budget again
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        sh.code.hash(&mut h);
        h.finish()
    };
    let mut res = match REFLECT_RESUMER.lock().unwrap().take() {
        Some((k, r)) if k == key => r,
        _ => Resumer::new(g, &sh),
    };
    let par = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("params"), size: (PARAM_WORDS * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut base = params(p, 0, n.div_ceil(R_W), &sh, 4, &PostJob::default(), Some((o, n)));
    base[SLOTS_WORD] = split as u32;
    let timing = std::env::var_os("MB3D_GPU_TIMING").is_some();
    use std::sync::atomic::Ordering;
    let n_card = std::cell::Cell::new(n);
    let mut unfinished = Vec::new();
    for (pi, pl) in pls.iter().enumerate() {
        let t_pl = std::time::Instant::now();
        let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pl.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: par.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: cst.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: rtpl.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: res.state.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: res.queue.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 6, resource: maps.as_entire_binding() },
            ],
        });
        // the march shares the tasks with the CPU (`RaySharing`), the
        // shadows and occlusion follow for the card's part
        let lim = |started: usize, free: usize| -> usize {
            let Some(sh) = share.filter(|_| pi == 0) else { return n_card.get() };
            let mut want = started + free;
            // the CPU's share: the card does not go beyond it (its rays
            // still need the shadow and occlusion pass after the march,
            // which the CPU does with each ray)
            want = want.min(sh.cap.max(started));
            let mut st = sh.state.lock().unwrap();
            st.0 = st.0.max(want.min(st.1));
            st.0
        };
        // the last few rays of the march: the CPU, when it has nothing else
        // to do, is faster for single long rays
        let give_up = |left: usize| -> bool {
            share.filter(|_| pi == 0).is_some_and(|sh| {
                let st = sh.state.lock().unwrap();
                left <= sh.tail && st.0 >= st.1
            })
        };
        let (done_n, busy) = res.run(
            g,
            &par,
            base.clone(),
            n,
            &lim,
            &give_up,
            &|| false,
            |enc, groups| {
                let mut pass = enc.begin_compute_pass(&Default::default());
                pass.set_pipeline(pl);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(groups, 1, 1);
            },
            |_| Ok(()),
        )?;
        if pi == 0 {
            n_card.set(done_n);
            unfinished = busy;
            if let Some(sh) = share {
                // the limit is final: the CPU does the rest
                sh.done.store(true, Ordering::SeqCst);
            }
        }
        if timing {
            let what = if pls.len() == 1 { "march, shadows, occlusion" } else if pi == 0 { "march" } else { "shadows, occlusion" };
            eprintln!("    reflection pipeline ({what}; lights {}, soft {}, AO {:?}): {:.2} s", o.lights.len(), o.soft_radius, o.ao, t_pl.elapsed().as_secs_f64());
        }
    }
    *REFLECT_RESUMER.lock().unwrap() = Some((key, res));
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(&buf, 0, &read, 0, size);
    g.queue.submit([enc.finish()]);
    let slice = read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    if let Some(e) = take_error() {
        return Err(e);
    }
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    let out = {
        let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
        data.chunks_exact(36 * 4)
            .map(|rec| {
                let u = |k: usize| u32::from_le_bytes(rec[k * 4..k * 4 + 4].try_into().unwrap());
                let f = |k: usize| f32::from_bits(u(k));
                let (a, b, z, s, amb) = (u(8), u(9), u(10), u(11), u(12));
                let flags = u(18);
                ReflectHit {
                    si: SiLight {
                        normal: [a as u16 as i16, (a >> 16) as u16 as i16, b as u16 as i16],
                        zpos_fine: z,
                        shadow: (b >> 16) as u16,
                        amb_shadow: amb as u16,
                        si_gradient: s as u16,
                        otrap: (s >> 16) as u16,
                    },
                    pos: if dbl {
                        let d = |k: usize| f64::from_bits(u(k) as u64 | (u(k + 1) as u64) << 32);
                        [d(30), d(32), d(34)]
                    } else {
                        [f(13) as f64, f(14) as f64, f(15) as f64]
                    },
                    zz: f(16),
                    zz2: f(17),
                    hit: flags & 1 != 0,
                    de_limited: flags & 2 != 0,
                    skip: flags & 4 != 0,
                    step_count: f(19),
                }
            })
            .collect()
    };
    read.unmap();
    Ok((out, n_card.get(), unfinished))
}

/// The [`Resumer`] of the Monte Carlo batches with the hash of their shader.
static MC_RESUMER: std::sync::Mutex<Option<(u64, Resumer)>> = std::sync::Mutex::new(None);

/// The words of a Monte Carlo task (`MC_WORDS` in the shader).
const MC_WORDS: usize = 20;

/// Runs marches of the Monte Carlo renderer on the graphics card (what
/// `mc.rs`'s `McMarch` does on the CPU).  `secant`: the secant search
/// refines the surfaces.  With `share` the card starts the tasks below the
/// shared limit and the CPU takes them from the end (see [`RaySharing`]);
/// returns the results (those the card did not take are default), the
/// number it took and those of them it did not finish.
pub fn mc_batch(p: &CalcParams, tasks: &[crate::mc::McReq], secant: bool, share: Option<&RaySharing>) -> Result<(Vec<crate::mc::McRes>, usize, Vec<usize>), String> {
    guard(|| mc_batch_card(p, tasks, secant, share))
}

fn mc_batch_card(p: &CalcParams, tasks: &[crate::mc::McReq], secant: bool, share: Option<&RaySharing>) -> Result<(Vec<crate::mc::McRes>, usize, Vec<usize>), String> {
    use crate::mc::McRes;
    if tasks.is_empty() {
        if let Some(sh) = share {
            sh.done.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        return Ok((Vec::new(), 0, Vec::new()));
    }
    if let Some(r) = unsupported_beyond_march(p) {
        return Err(r);
    }
    let dbl = needs_double(p);
    let g = gpu()?;
    let sh = scene_shader(p)?;
    let pl = pipeline(g, &sh.code, &mc_constants(p))?;
    let n = tasks.len();
    let words: Vec<u32> = tasks
        .iter()
        .flat_map(|t| {
            let mut w = [0u32; MC_WORDS];
            for k in 0..3 {
                w[k] = (t.pos[k] as f32).to_bits();
                w[3 + k] = (t.dir[k] as f32).to_bits();
                let b = t.pos[k].to_bits();
                (w[13 + 2 * k], w[14 + 2 * k]) = (b as u32, (b >> 32) as u32);
            }
            w[6] = (t.zz as f32).to_bits();
            w[7] = (t.zz2 as f32).to_bits();
            w[8] = (t.zzplus as f32).to_bits();
            w[9] = (t.len as f32).to_bits();
            w[10] = t.seed as u32;
            w[11] = t.kind as u32 | (t.inside as u32) << 2 | (t.probe as u32) << 3 | (t.trans as u32) << 4;
            w[12] = t.ms_de_stop.to_bits();
            w
        })
        .collect();
    let size = (words.len() * 4) as u64;
    let buf = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("mc tasks"), size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    g.queue.write_buffer(&buf, 0, &bytes(&words));
    let read = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("mc read"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let make = |label: &'static str, d: &[u32]| {
        let d = if d.is_empty() { vec![0u32] } else { d.to_vec() };
        let b = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (d.len() * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        g.queue.write_buffer(&b, 0, &bytes(&d));
        b
    };
    let cst = make("constants", &sh.cst);
    let rtpl = make("record", &sh.rtpl);
    let maps = make("maps", &sh.maps);
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        sh.code.hash(&mut h);
        h.finish()
    };
    let mut res = match MC_RESUMER.lock().unwrap().take() {
        Some((k, r)) if k == key => r,
        _ => Resumer::new(g, &sh),
    };
    let par = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("params"), size: (PARAM_WORDS * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut base = params(p, 0, n.div_ceil(R_W), &sh, 5, &PostJob::default(), None);
    base[171] = secant as u32;
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pl.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: par.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: cst.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: rtpl.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: res.state.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: res.queue.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 6, resource: maps.as_entire_binding() },
        ],
    });
    use std::sync::atomic::Ordering;
    // (the first call has all slots free)
    let slots = std::cell::Cell::new(0usize);
    let lim = |started: usize, free: usize| -> usize {
        let Some(sh) = share else { return n };
        slots.set(slots.get().max(free));
        let mut want = started + free;
        // the CPU's share: the card goes beyond it only once it has
        // started all of its own and runs low on rays
        if want > sh.cap && (started < sh.cap || slots.get() - free > slots.get() / 4) {
            want = sh.cap.max(started);
        }
        let mut st = sh.state.lock().unwrap();
        st.0 = st.0.max(want.min(st.1));
        st.0
    };
    let give_up = |left: usize| -> bool {
        share.is_some_and(|sh| {
            let st = sh.state.lock().unwrap();
            left <= sh.tail && st.0 >= st.1
        })
    };
    let r = res.run(
        g,
        &par,
        base,
        n,
        &lim,
        &give_up,
        &|| false,
        |enc, groups| {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pl);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(groups, 1, 1);
        },
        |_| Ok(()),
    );
    if let Some(sh) = share {
        sh.done.store(true, Ordering::SeqCst);
    }
    let (n_card, unfinished) = r?;
    *MC_RESUMER.lock().unwrap() = Some((key, res));
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(&buf, 0, &read, 0, size);
    g.queue.submit([enc.finish()]);
    let slice = read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    if let Some(e) = take_error() {
        return Err(e);
    }
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    let out = {
        let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
        data.chunks_exact(MC_WORDS * 4)
            .take(n_card)
            .map(|rec| {
                let u = |k: usize| u32::from_le_bytes(rec[k * 4..k * 4 + 4].try_into().unwrap());
                let f = |k: usize| f32::from_bits(u(k)) as f64;
                let flags = u(11);
                McRes {
                    kind: (flags & 3) as u8,
                    pos: if dbl {
                        let d = |k: usize| f64::from_bits(u(k) as u64 | (u(k + 1) as u64) << 32);
                        [d(13), d(15), d(17)]
                    } else {
                        [f(0), f(1), f(2)]
                    },
                    normals: [f(3), f(4), f(5)],
                    zz: f(6),
                    zz2: f(7),
                    dyn_fog: f(8) as f32,
                    len: f(9),
                    seed: u(10) as i32,
                    inside: flags & 4 != 0,
                    de_limited: flags & 8 != 0,
                    start_difs: flags & 16 != 0,
                    last_difs: flags & 32 != 0,
                    ms_de_stop: f(12) as f32,
                    si_grad: u(19) as u16,
                    otrap: (u(19) >> 16) as u16,
                }
            })
            .collect()
    };
    read.unmap();
    Ok((out, n_card, unfinished))
}

/// The DE of every pixel of `p.rect` at the start of its ray, as the
/// shader calculates it (a check against the CPU's `calc::de_at`): the DE
/// in step widths, the iteration count and `max_its_result`.
pub fn de_probe(p: &CalcParams, mzz: f64) -> Result<Vec<DeProbe>, String> {
    if let Some(r) = unsupported(p) {
        return Err(r);
    }
    let g = gpu()?;
    let sh = scene_shader(p)?;
    PROBE_MZZ.with(|m| m.set(mzz));
    let v = run(g, p, &sh, &PostJob::default(), Some(3), &|_, _| {}, &|| false, &mut |_, _| {}, None, None)?;
    Ok(v.iter()
        .map(|s| {
            let rst = f32::from_bits((s.normal[0] as u16 as u32) | (s.normal[1] as u16 as u32) << 16);
            let r1 = f32::from_bits(s.amb_shadow as u32 | (s.shadow as u32) << 16);
            (f32::from_bits(s.zpos_fine), s.si_gradient as i32, 0, [(r1, s.otrap as i32), (rst, s.normal[2] as i32)])
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
fn run(
    g: &Gpu,
    p: &CalcParams,
    sh: &SceneShader,
    post: &PostJob,
    only_mode: Option<u32>,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    sink: &mut dyn FnMut(usize, &[SiLight]),
    bands: Option<&dyn Fn(usize) -> Option<(usize, usize)>>,
    upload: Option<&[SiLight]>,
) -> Result<Vec<SiLight>, String> {
    let (w, h) = (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize);
    let mut res = Resumer::new(g, sh);
    let mut modes = only_mode.map(|m| vec![m]).unwrap_or_else(|| modes(post));
    // `upload`: the G-buffer marched elsewhere (by the CPU too) goes to the
    // card band by band, and only the post passes run on it
    if upload.is_some() {
        modes.retain(|&m| m != 0);
        if modes.is_empty() {
            return Ok(upload.unwrap().to_vec());
        }
    }
    // the rows of a job: as many as the G-buffer allows (a job ends with
    // its slowest pixels alone on the card); with post passes (which need
    // the march of the band done) a few slot generations, so that the rows
    // still come in steps
    // (shared with the CPU: small bands, so that both end together)
    let band = if modes.len() == 1 && bands.is_none() { (96 << 20) / (w.max(1) * 20) } else { 4 * res.slots / w.max(1) }.clamp(8, h.max(8));
    let out_size = (w * band * 20) as u64;
    let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
    if upload.is_some() {
        usage |= wgpu::BufferUsages::COPY_DST;
    }
    let out = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("gbuffer"), size: out_size.max(16), usage, mapped_at_creation: false });
    let read = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read"),
        size: out_size.max(16),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // one parameter buffer per pass (a buffer write before a submission
    // takes effect before all of its passes), and a pipeline per kind of pass
    let mut pls = Vec::new();
    for &m in &modes {
        pls.push(pipeline(g, &sh.code, &march_constants(p, m))?);
    }
    let pars: Vec<wgpu::Buffer> = modes
        .iter()
        .map(|_| {
            g.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("params"),
                size: (PARAM_WORDS * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
        .collect();
    let words = |label: &'static str, d: &[u32]| {
        let d = if d.is_empty() { vec![0u32] } else { d.to_vec() };
        let b = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (d.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        g.queue.write_buffer(&b, 0, &bytes(&d));
        b
    };
    let cst = words("constants", &sh.cst);
    let rtpl = words("record", &sh.rtpl);
    let maps = words("maps", &vol_maps(p, sh));
    let binds: Vec<wgpu::BindGroup> = pars
        .iter()
        .zip(&pls)
        .map(|(par, pl)| {
            g.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pl.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: par.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: out.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: cst.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: rtpl.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: res.state.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 5, resource: res.queue.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: maps.as_entire_binding() },
                ],
            })
        })
        .collect();
    let mut gbuf = Vec::with_capacity(w * h);
    let mut overflow = 0usize;
    let mut next = 0;
    // the next band: in order, or from the allocator (the rows from the
    // top that the CPU has not taken)
    let mut take = |band: usize| match bands {
        Some(f) => f(band),
        None => (next < h).then(|| {
            let r = (next, band.min(h - next));
            next += r.1;
            r
        }),
    };
    while let Some((row0, rows)) = take(band) {
        if cancel() {
            return Err("cancelled".into());
        }
        if let Some(gb) = upload {
            // as the march writes it (see the end of `main` in the shader)
            let words: Vec<u32> = gb[row0 * w..(row0 + rows) * w]
                .iter()
                .flat_map(|s| {
                    [
                        (s.normal[0] as u16 as u32) | (s.normal[1] as u16 as u32) << 16,
                        (s.normal[2] as u16 as u32) | (s.shadow as u32) << 16,
                        s.zpos_fine,
                        s.si_gradient as u32 | (s.otrap as u32) << 16,
                        s.amb_shadow as u32,
                    ]
                })
                .collect();
            g.queue.write_buffer(&out, 0, &bytes(&words));
        }
        // each pass on the band until its pixels are done; the rows the
        // last one has finished go to the sink as they come
        let mut sunk = 0;
        for (k, &mode) in modes.iter().enumerate() {
            let last = k + 1 == modes.len();
            res.run(
                g,
                &pars[k],
                params(p, row0, rows, sh, mode, post, None),
                w * rows,
                &|_, _| w * rows,
                &|_| false,
                cancel,
                |enc, groups| {
                    let mut pass = enc.begin_compute_pass(&Default::default());
                    pass.set_pipeline(&pls[k]);
                    pass.set_bind_group(0, &binds[k], &[]);
                    pass.dispatch_workgroups(groups, 1, 1);
                },
                |done| {
                    let ready = done / w.max(1);
                    if !last || ready <= sunk || (ready < rows && ready - sunk < 8) {
                        return Ok(());
                    }
                    let data = read_back(g, &out, &read, (sunk * w * 20) as u64, ((ready - sunk) * w * 20) as u64)?;
                    for (r, row) in data.chunks_exact(w * 20).enumerate() {
                        let start = gbuf.len();
                        for px in row.chunks_exact(20) {
                            let u = |k: usize| u32::from_le_bytes(px[k * 4..k * 4 + 4].try_into().unwrap());
                            let (a, b, z, s, amb) = (u(0), u(1), u(2), u(3), u(4));
                            overflow += (amb & 0x10000 != 0) as usize;
                            gbuf.push(SiLight {
                                normal: [a as u16 as i16, (a >> 16) as u16 as i16, b as u16 as i16],
                                zpos_fine: z,
                                shadow: (b >> 16) as u16,
                                amb_shadow: amb as u16,
                                si_gradient: s as u16,
                                otrap: (s >> 16) as u16,
                            });
                        }
                        sink(row0 + sunk + r, &gbuf[start..]);
                    }
                    sunk = ready;
                    if bands.is_none() {
                        progress(row0 + sunk, h);
                    }
                    Ok(())
                },
            )?;
        }
    }
    OVERFLOW.with(|c| c.set(overflow));
    Ok(gbuf)
}

/// Copies `len` bytes at `offset` of the storage buffer `out` through the
/// mappable buffer `read` (at least as large).
fn read_back(g: &Gpu, out: &wgpu::Buffer, read: &wgpu::Buffer, offset: u64, len: u64) -> Result<Vec<u8>, String> {
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(out, offset, read, 0, len);
    g.queue.submit([enc.finish()]);
    let slice = read.slice(..len);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    if let Some(e) = take_error() {
        return Err(e);
    }
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    let v = slice.get_mapped_range().map_err(|e| e.to_string())?.to_vec();
    read.unmap();
    Ok(v)
}

/// Screen space ambient occlusion (`ssao::ssao24`) on the graphics card:
/// the same levels, directions and angle sums as the CPU (with random
/// sampling the random numbers differ).  Sets `amb_shadow` of the object
/// pixels; `Err` leaves the G-buffer untouched (the caller uses the CPU).
pub fn ssao24(gbuf: &mut [SiLight], w: usize, h: usize, zc_mul: f64, zcorr: f64, p: &crate::ssao::SsaoParams) -> Result<(), String> {
    guard(|| ssao24_card(gbuf, w, h, zc_mul, zcorr, p))
}

fn ssao24_card(gbuf: &mut [SiLight], w: usize, h: usize, zc_mul: f64, zcorr: f64, p: &crate::ssao::SsaoParams) -> Result<(), String> {
    if w == 0 || h == 0 {
        return Ok(());
    }
    let g = gpu()?;
    let code = include_str!("gpu_ssao.wgsl");
    let entries: Vec<wgpu::BindGroupLayoutEntry> = (0..6)
        .map(|b| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: b < 2 }, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        })
        .collect();
    let bgl = g.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("ssao"), entries: &entries });
    let layout = g.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("ssao"), bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
    let (pl_h, pl_v, pl_s) = (
        pipeline_entry(g, code, "blur_h", Some(&layout), &[])?,
        pipeline_entry(g, code, "blur_v", Some(&layout), &[])?,
        pipeline_entry(g, code, "sample", Some(&layout), &[])?,
    );
    let levels = crate::ssao::level_count(w, h);
    let z_scale = ((256.0 / zc_mul + 1.0).powi(2) - 1.0) / zcorr;
    let thr = p.threshold.max(0.01);
    let wlo = (w as f32 * p.border_mirror).round_ties_even() as i32;
    let whi = w as i32 - 1 - wlo;
    let hlo = (h as f32 * p.border_mirror).round_ties_even() as i32;
    let hhi = h as i32 - 1 - hlo;
    let passes = if p.random > 0 { p.random as usize } else { 1 };
    let first: Vec<u32> = gbuf.iter().map(|s| if s.zpos() < 32768 { (s.zpos_fine & 0xFFFF_FF00) >> 1 } else { 0 }).collect();
    let z0: Vec<u32> = gbuf.iter().zip(&first).map(|(s, z)| z | (s.zpos() < 32768) as u32).collect();
    // the angles of a block of rows: 64 bytes per pixel, at most 64 MB
    let max_rows = (64 * 1024 * 1024 / (w * 64)).clamp(1, h);
    let blocks = (h - 1) / max_rows + 1;
    let rows = h.div_ceil(blocks);
    let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    let make = |label: &'static str, size: usize, usage: wgpu::BufferUsages| {
        g.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (size.max(1) * 4) as u64, usage, mapped_at_creation: false })
    };
    let par = make("ssao params", 32, storage);
    let bz0 = make("ssao z0", w * h, storage);
    let pa = make("ssao level a", w * h, storage);
    let pb = make("ssao level b", w * h, storage);
    let ang = make("ssao angles", w * rows * 16, storage);
    let acc = make("ssao sum", w * h, storage | wgpu::BufferUsages::COPY_SRC);
    let read = make("ssao read", w * h, wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST);
    g.queue.write_buffer(&bz0, 0, &bytes(&z0));
    g.queue.write_buffer(&acc, 0, &vec![0u8; w * h * 4]);
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: par.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: bz0.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: pa.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: pb.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: ang.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: acc.as_entire_binding() },
        ],
    });
    let (gx, gy) = (w.div_ceil(8) as u32, h.div_ceil(8) as u32);
    for pass in 0..passes {
        let mut y0 = 0;
        while y0 < h {
            let y1 = (y0 + rows).min(h);
            g.queue.write_buffer(&pa, 0, &bytes(&first));
            for level in 1..=levels {
                // the constants of the level (ssao::ssao24)
                let mut sit = (z_scale / 22000.0 * 4096.0) as f32;
                let mut szrt = thr * if p.t0 { 1.0 } else { 0.7 } * 4096.0 / sit * ((levels as f32 / level as f32).sqrt().sqrt());
                if p.t0 {
                    szrt *= szrt;
                }
                let rnd = p.random > 0;
                let at = if rnd {
                    (thr * if p.t0 { 0.64 } else { 0.65 } * (levels as f32).sqrt().sqrt()).atan()
                } else {
                    (thr * 0.6 * (levels as f32).sqrt().sqrt()).atan()
                };
                let smul = 1.5 * 32767.0 / (std::f32::consts::PI * 32.0 * if p.t0 { at } else { at.powf(0.9) }) / passes as f32;
                let imin = 16383 / passes as i32;
                let iand = (1i64 << (level - 1)) - 1;
                let ssub = iand as f32 * 0.5;
                let istep = 1i64 << (level - 1);
                let smin_rad = if istep < 2 { 1.0f32 } else { 3.25 * istep as f32 };
                let step_count = if istep < 2 { 5 } else { 3 };
                sit *= szrt;
                let mut v: Vec<u32> = Vec::with_capacity(32);
                let i = |v: &mut Vec<u32>, x: i32| v.push(x as u32);
                let f = |v: &mut Vec<u32>, x: f32| v.push(x.to_bits());
                i(&mut v, w as i32);
                i(&mut v, h as i32);
                i(&mut v, y0 as i32);
                i(&mut v, (y1 - y0) as i32);
                i(&mut v, (level == levels) as i32);
                f(&mut v, sit);
                f(&mut v, szrt);
                f(&mut v, smul);
                i(&mut v, imin);
                i(&mut v, iand as i32);
                f(&mut v, ssub);
                i(&mut v, istep as i32);
                f(&mut v, smin_rad);
                i(&mut v, step_count);
                for k in 0..5 {
                    f(&mut v, 1.0 / (smin_rad + (k as i64 * istep) as f32 + 0.1));
                }
                i(&mut v, rnd as i32);
                i(&mut v, p.t0 as i32);
                i(&mut v, wlo);
                i(&mut v, whi);
                i(&mut v, hlo);
                i(&mut v, hhi);
                i(&mut v, 2 * (w as i32 - 1));
                i(&mut v, 2 * (h as i32 - 1));
                i(&mut v, pass as i32);
                // the blur that made this level (0: the first level) and
                // the one that makes the next
                i(&mut v, if level == 1 { 0 } else { 1 << (level - 1) });
                v.resize(32, 0);
                g.queue.write_buffer(&par, 0, &bytes(&v));
                let mut enc = g.device.create_command_encoder(&Default::default());
                {
                    let mut ps = enc.begin_compute_pass(&Default::default());
                    ps.set_pipeline(&pl_s);
                    ps.set_bind_group(0, &bind, &[]);
                    ps.dispatch_workgroups(gx, (y1 - y0).div_ceil(8) as u32, 1);
                }
                if level < levels {
                    // NextATlevelHiQ(..., 1 shl level): the step is in the
                    // parameters of the next level, so it is sent now
                    let mut v2 = v.clone();
                    v2[28] = 1 << level;
                    g.queue.submit([enc.finish()]);
                    g.queue.write_buffer(&par, 0, &bytes(&v2));
                    enc = g.device.create_command_encoder(&Default::default());
                    for pl in [&pl_h, &pl_v] {
                        let mut ps = enc.begin_compute_pass(&Default::default());
                        ps.set_pipeline(pl);
                        ps.set_bind_group(0, &bind, &[]);
                        ps.dispatch_workgroups(gx, gy, 1);
                    }
                }
                g.queue.submit([enc.finish()]);
            }
            y0 = y1;
        }
    }
    let mut enc = g.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(&acc, 0, &read, 0, (w * h * 4) as u64);
    g.queue.submit([enc.finish()]);
    let slice = read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    if let Some(e) = take_error() {
        return Err(e);
    }
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    {
        let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
        for (s, c) in gbuf.iter_mut().zip(data.chunks_exact(4)) {
            if s.zpos() < 32768 {
                s.amb_shadow = u32::from_le_bytes(c.try_into().unwrap()).min(16383) as u16;
            }
        }
    }
    read.unmap();
    Ok(())
}

/// The older 15 bit screen space ambient occlusion (`ssao15::ssao15`) on
/// the graphics card: the levels are built on the CPU, the sampling of
/// every pixel runs in the shader.
pub fn ssao15(gbuf: &mut [SiLight], w: usize, h: usize, zc_mul: f64, zcorr: f64, p: &crate::ssao::SsaoParams) -> Result<(), String> {
    guard(|| ssao15_card(gbuf, w, h, zc_mul, zcorr, p))
}

fn ssao15_card(gbuf: &mut [SiLight], w: usize, h: usize, zc_mul: f64, zcorr: f64, p: &crate::ssao::SsaoParams) -> Result<(), String> {
    if w < 4 || h < 4 {
        return Ok(());
    }
    let g = gpu()?;
    let pl = pipeline_entry(g, include_str!("gpu_ssao15.wgsl"), "sample15", None, &[])?;
    let lv = crate::ssao15::build_levels(gbuf, w, h);
    let (szrt, smul, st2) = crate::ssao15::sampling_constants(&lv, zc_mul, zcorr, p);
    let levels: Vec<u32> = lv.lev.iter().flat_map(|l| l.iter().map(|&v| v as u32)).collect();
    let zps: Vec<u32> = gbuf.iter().map(|s| lv.zp_of(s) as u32).collect();
    let par = [w as u32, h as u32, lv.count as u32, szrt.to_bits(), st2.to_bits(), smul.to_bits(), p.t0 as u32, 0];
    let out = run_pipeline(g, &pl, &[&par, &levels, &zps], w * h, (w.div_ceil(8) as u32, h.div_ceil(8) as u32))?;
    for (s, v) in gbuf.iter_mut().zip(out) {
        if s.zpos() < 32768 {
            s.amb_shadow = v.min(16383) as u16;
        }
    }
    Ok(())
}

/// Runs `pipeline` once with the storage buffers `inputs` (bindings 0..,
/// read only) and an output buffer of `out_len` words (the next binding);
/// `groups` workgroups in x and y.
fn run_pipeline(g: &Gpu, pipeline: &wgpu::ComputePipeline, inputs: &[&[u32]], out_len: usize, groups: (u32, u32)) -> Result<Vec<u32>, String> {
    let mut bufs = Vec::new();
    for d in inputs {
        let words = if d.is_empty() { vec![0u32] } else { d.to_vec() };
        let b = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (words.len() * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        g.queue.write_buffer(&b, 0, &bytes(&words));
        bufs.push(b);
    }
    let size = (out_len.max(1) * 4) as u64;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
    let read = g.device.create_buffer(&wgpu::BufferDescriptor { label: None, size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut entries: Vec<wgpu::BindGroupEntry> = bufs.iter().enumerate().map(|(i, b)| wgpu::BindGroupEntry { binding: i as u32, resource: b.as_entire_binding() }).collect();
    entries.push(wgpu::BindGroupEntry { binding: bufs.len() as u32, resource: out.as_entire_binding() });
    let bind = g.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &pipeline.get_bind_group_layout(0), entries: &entries });
    let mut enc = g.device.create_command_encoder(&Default::default());
    {
        let mut pass = enc.begin_compute_pass(&Default::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(groups.0, groups.1, 1);
    }
    enc.copy_buffer_to_buffer(&out, 0, &read, 0, size);
    g.queue.submit([enc.finish()]);
    let slice = read.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    g.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    if let Some(e) = take_error() {
        return Err(e);
    }
    let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
    let v = data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    drop(data);
    read.unmap();
    Ok(v)
}

/// Compiles `wgsl` (entry point `entry`) as a new shader (the driver's
/// cache is defeated) and returns the time it took; for shader size
/// experiments.
pub fn compile_time(wgsl: &str, entry: &str) -> Result<f64, String> {
    let g = gpu()?;
    std::env::set_var("MB3D_GPU_NOCACHE", "1");
    let t = std::time::Instant::now();
    pipeline_entry(g, wgsl, entry, None, &[])?;
    Ok(t.elapsed().as_secs_f64())
}

/// Runs `wgsl` (entry point `main`) once with the storage buffers
/// `inputs` (bindings 0.., read only) and an output buffer of `out_len`
/// words (the next binding); `groups` workgroups.  For tests of generated
/// shader code.
pub fn run_compute(wgsl: &str, inputs: &[&[u32]], out_len: usize, groups: u32) -> Result<Vec<u32>, String> {
    let g = gpu()?;
    let scope = g.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("test"), source: wgpu::ShaderSource::Wgsl(wgsl.into()) });
    let pipeline = g.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("test"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    if let Some(e) = pollster::block_on(scope.pop()) {
        return Err(e.to_string());
    }
    run_pipeline(g, &pipeline, inputs, out_len, (groups, 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::Marcher;

    /// Resumable marching saves and loads every variable that lives across
    /// turns of the marcher: the private variables and the locals of
    /// `pixel` declared before its loop (a variable added later must join
    /// the lists, or a suspended pixel would continue with a stale value).
    #[test]
    fn state_lists_complete() {
        let code = include_str!("gpu_march.wgsl");
        // set before every pixel or submission, or the debug outputs
        let transient = ["state_slot", "cur_item", "resuming", "suspended", "r_task_origin", "r_task_dir", "r_task", "r_rec", "r_keep", "r_task_inside", "r_task_q", "dv", "mc_t_pos", "mc_t_dir", "mc_t", "mc_t_flags", "mc_t_stop"];
        let mut names: Vec<&str> = code.lines().filter_map(|l| l.strip_prefix("var<private> ")).map(|l| l.split(':').next().unwrap()).collect();
        let body = &code[code.find("fn pixel(").unwrap()..];
        let lp = body.find("    for (var turns").unwrap();
        names.extend(body[..lp].lines().filter_map(|l| l.strip_prefix("    var ")).map(|l| l.split([':', ' ', ';']).next().unwrap()));
        let load = &body[body.find("if (resuming) {").unwrap()..lp];
        let save = &body[lp..body.find("save_record(").unwrap()];
        let has = |block: &str, n: &str| block.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == n);
        for n in names.iter().filter(|n| !n.starts_with("dbg_") && !transient.contains(n)) {
            assert!(has(save, n), "{n} is not saved (python3 tools/march_state.py src/gpu_march.wgsl)");
            assert!(has(load, n), "{n} is not loaded");
        }
        assert!(state_fixed() > 100);
    }

    /// The GPU G-buffer against the CPU's for the example Mandelbulb (skipped
    /// when the computer has no usable graphics card).
    #[test]
    fn mandelbulb_matches_cpu() {
        gbuffer_matches_cpu(include_str!("../examples/mandelbulb.m3s"), 0.5);
    }

    /// The 4D hybrid loop (doHybrid4DPas) with the 4D rotation.
    #[test]
    fn quaternion_julia_matches_cpu() {
        gbuffer_matches_cpu(include_str!("../examples/quaternion_julia.m3s"), 0.5);
    }

    /// The interpolation hybrid (doInterpolHybridPasDE), 3D and, with two
    /// quaternions, 4D (doInterpolHybridPas4D).
    #[test]
    fn interpolation_matches_cpu() {
        gbuffer_matches_cpu(include_str!("../examples/interpolation_boxes.m3s"), 0.5);
        let quat = format!("{}\n[formula]\nname = Quaternion\nyw_multiplier = -1\nw_add = 0.3\n", include_str!("../examples/quaternion_julia.m3s").replace("iterations = 12\n", "iterations = 12\ninterpolation = 0.5, 0.5\n"));
        gbuffer_matches_cpu(&quat, 0.5);
    }

    /// Cutting planes, colour on iteration, inside and outside rendering
    /// (the Mandelbulb cut open: the rays start on the plane, inside the
    /// set where it is cut).
    #[test]
    fn cut_inside_matches_cpu() {
        let bulb = include_str!("../examples/mandelbulb.m3s");
        gbuffer_matches_cpu(&format!("cut_y = -0.3\ncolor_on_iteration = 2\n{bulb}"), 0.5);
        gbuffer_matches_cpu(&format!("cut_z = 0.1\ninside = both\n{bulb}"), 0.5);
        gbuffer_matches_cpu(&format!("cut_x = 0.2\ninside = inside\n{bulb}"), 0.5);
    }

    fn gbuffer_matches_cpu(scene: &str, scale: f64) {
        let mut sc = crate::scene::Scene::parse(scene).unwrap();
        sc.scale_image(scale);
        let p = CalcParams::new(&sc).unwrap();
        assert_eq!(unsupported(&p), None);
        if let Err(e) = gpu() {
            eprintln!("no GPU, test skipped: {e}");
            return;
        }
        let t = std::time::Instant::now();
        let g = march(&p, &PostJob::default(), &|_, _| {}, &|| false, |_, _| {}).expect(&last_status());
        let gpu_s = t.elapsed().as_secs_f64();
        let [x0, y0, w, h] = p.rect;
        let t = std::time::Instant::now();
        let mut cpu = Vec::new();
        for y in y0..y0 + h {
            let seed = (0x24563487u32 as i64 + (y as i64 + 1) * 0x324594A1i64) as i32;
            let mut m = Marcher::new(&p, seed);
            cpu.extend((x0..x0 + w).map(|x| m.march_pixel(x, y)));
        }
        let cpu_s = t.elapsed().as_secs_f64();
        assert_eq!(g.len(), cpu.len());
        let (mut same_hit, mut both, mut zdiff, mut ndot, mut bad_n) = (0usize, 0usize, 0f64, 0f64, 0usize);
        // the colour values: orbit trap (bit 15: the inside and outside flag)
        // and the gradient (bit 15: started inside the set)
        let (mut col_off, mut flag_off, mut flagged) = (0usize, 0usize, 0usize);
        for (a, b) in g.iter().zip(&cpu) {
            if a.is_background() == b.is_background() {
                same_hit += 1;
            }
            if !a.is_background() && !b.is_background() {
                both += 1;
                col_off += (((a.otrap & 0x7FFF) as i32 - (b.otrap & 0x7FFF) as i32).abs() > 500) as usize;
                flag_off += ((a.otrap ^ b.otrap) & 0x8000 != 0 || (a.si_gradient ^ b.si_gradient) & 0x8000 != 0) as usize;
                flagged += (b.otrap & 0x8000 != 0) as usize;
                zdiff += (a.zpos() as f64 - b.zpos() as f64).abs();
                let na = a.normal.map(|v| v as f64 / 32767.0);
                let nb = b.normal.map(|v| v as f64 / 32767.0);
                let d = na[0] * nb[0] + na[1] * nb[1] + na[2] * nb[2];
                ndot += d;
                if d < 0.9 {
                    bad_n += 1;
                }
            }
        }
        let n = g.len() as f64;
        let same = same_hit as f64 / n;
        let zd = zdiff / both.max(1) as f64;
        let nd = ndot / both.max(1) as f64;
        let bad = bad_n as f64 / both.max(1) as f64;
        eprintln!(
            "{}: GPU {gpu_s:.3}s (one thread on the CPU {cpu_s:.3}s); same background/object {:.2}%, \
             mean |zpos difference| {zd:.1} of 32768, mean normal cosine {nd:.4}, normals off by more than 25 degrees {:.2}%",
            last_status(),
            same * 100.0,
            bad * 100.0
        );
        let col = col_off as f64 / both.max(1) as f64;
        let flag = flag_off as f64 / both.max(1) as f64;
        eprintln!("  colour values off {:.2}%, flags differ {:.2}%, outside-flagged {flagged}", col * 100.0, flag * 100.0);
        assert!(same > 0.99, "background/object differ in {:.2}% of the pixels", (1.0 - same) * 100.0);
        assert!(col < 0.03 && flag < 0.01, "colour values differ in {:.2}%, flags in {:.2}% of the pixels", col * 100.0, flag * 100.0);
        assert!(zd < 20.0, "depth differs by {zd:.1} on average");
        assert!(nd > 0.97 && bad < 0.03, "normals differ: mean cosine {nd:.4}, {:.2}% off", bad * 100.0);
    }
}
