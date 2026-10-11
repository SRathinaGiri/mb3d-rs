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
/// `MB3D_GPU_BAND_MS`, default 25): the desktop shares the card, its
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
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let queue = g.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("queue"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let queue_read = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("queue read"), size: 16, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let ms: f64 = std::env::var("MB3D_GPU_BAND_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(25.0);
        // MB3D_GPU_TURNS: a fixed budget (a check of the suspending)
        let fixed: Option<u32> = std::env::var("MB3D_GPU_TURNS").ok().and_then(|v| v.parse().ok());
        Resumer { state, queue, queue_read, slots, budget: fixed.unwrap_or(64), fixed: fixed.is_some(), target: ms / 1000.0, timing: std::env::var_os("MB3D_GPU_TIMING").is_some() }
    }

    /// Runs a job of `items` pixels (tasks) until all are done: `base` is
    /// its parameter block (written to `par`), `encode` records its compute
    /// pass of the given work groups (x, 1), `done(n)` is called when the
    /// first `n` items are finished.
    fn run(
        &mut self,
        g: &Gpu,
        par: &wgpu::Buffer,
        mut base: Vec<u32>,
        items: usize,
        cancel: &dyn Fn() -> bool,
        encode: impl Fn(&mut wgpu::CommandEncoder, u32),
        mut done: impl FnMut(usize) -> Result<(), String>,
    ) -> Result<(), String> {
        let groups = self.slots / 64;
        base[SLOTS_WORD + 1] = self.slots as u32;
        base[SLOTS_WORD + 2] = items as u32;
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
            let (left, next) = (q[0] as usize, (q[1] as usize).min(items));
            let dt = t.elapsed().as_secs_f64();
            if self.timing {
                eprintln!("  submission (budget {}): {:.1} ms, {next} of {items} started, {left} suspended", self.budget, dt * 1000.0);
            }
            // the first suspended item or the next to start: all before it
            // are finished
            done(if left > 0 { (q[2] as usize).min(next) } else { next })?;
            if left == 0 && next >= items {
                return Ok(());
            }
            if !self.fixed {
                let f = (self.target / dt.max(1e-4)).clamp(0.25, 2.0);
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

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed) && !CPU_HERE.with(|c| c.get())
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
    if p.vol.is_some() {
        return r("volumetric light (not on the GPU yet)");
    }
    // MB3D_GPU_MIN_PRECISION / MB3D_GPU_MIN_STEP: experiments with the limits
    let env = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let numeric = !(p.is_custom_de || p.difs) || p.decomb.as_ref().is_some_and(|d| !(d.part2.is_custom_de || d.part2.difs));
    if numeric && precision(p) < env("MB3D_GPU_MIN_PRECISION", MIN_PRECISION) {
        return r("the zoom needs double precision");
    }
    if step_precision(p) < env("MB3D_GPU_MIN_STEP", MIN_STEP_PRECISION) {
        return r("the zoom needs double precision");
    }
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

fn scene_shader(p: &CalcParams) -> Result<SceneShader, String> {
    use crate::x86::lift_wgsl::{record_globals, CellTy, WgslFormula, PRELUDE};
    use std::fmt::Write;
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
            cst.extend(wf.constants_from(&m.mem[vb..vb + 0x400]));
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
                cst.extend(wf.constants_from(&m.mem[vb..vb + 0x400]));
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
                CellTy::Whole => rtpl[2 * *c as usize] = (f64::from_bits(lo as u64 | (hi as u64) << 32) as f32).to_bits(),
                CellTy::Halves => {
                    rtpl[2 * *c as usize] = lo;
                    rtpl[2 * *c as usize + 1] = hi;
                }
            }
        }
    }
    let mut g = String::new();
    g.push_str(PRELUDE);
    g.push_str(&globals);
    // init_record, marshal_in, marshal_out
    let has = |c: u32| ty.contains_key(&c);
    let _ = writeln!(g, "fn init_record() {{");
    for (c, t) in &ty {
        match t {
            CellTy::Whole => {
                let _ = writeln!(g, "    r{c}f = bitcast<f32>(rtpl[{}u]);", 2 * c);
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
                let _ = writeln!(save, "    STATE[o + {record_words}u * ns] = bitcast<u32>(r{c}f);");
                let _ = writeln!(load, "    r{c}f = bitcast<f32>(STATE[o + {record_words}u * ns]);");
                record_words += 1;
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
    for (_, w) in &wfs {
        g.push_str(&w.code);
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
                let _ = writeln!(g, "        case {n}u: {{ v = int_pow(v, j, bitcast<i32>(slot({n}u, 4u)), bitcast<f32>(slot({n}u, 5u))); }}");
            }
            2 => {
                let _ = writeln!(g, "        case {n}u: {{ marshal_in(); {regs} fsw = 0u; fm{n}(slot({n}u, 2u)); marshal_out(); }}");
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
    let code = include_str!("gpu_march.wgsl").replace("//@FORMULAS@", &g);
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
}

/// The pipeline of the shader `code` (compiled once, then cached).
fn pipeline(g: &Gpu, code: &str, constants: &[(&str, f64)]) -> Result<std::sync::Arc<wgpu::ComputePipeline>, String> {
    pipeline_entry(g, code, "main", None, constants)
}

/// The pipeline of the entry point `entry` of the shader `code`; `layout`
/// when several pipelines share one bind group (the automatic layout of a
/// pipeline is exclusive to it).
fn pipeline_entry(g: &Gpu, code: &str, entry: &str, layout: Option<&wgpu::PipelineLayout>, constants: &[(&str, f64)]) -> Result<std::sync::Arc<wgpu::ComputePipeline>, String> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (code, entry).hash(&mut h);
    for (k, v) in constants {
        (k, v.to_bits()).hash(&mut h);
    }
    let key = h.finish();
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
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("mb3d"),
            required_limits: adapter.limits(),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("{}: {e}", info.name))?;
    // errors (a shader the driver cannot compile, a lost device) must not
    // end the program: they are kept and the calculation goes to the CPU
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| set_error(e.to_string())));
    Ok(Gpu { device, queue, pipelines: Mutex::new(Default::default()), name: format!("{} ({:?})", info.name, info.backend) })
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
    i(&mut v, [p.de_add_steps, 8, 5, p.de_add_steps, p.de_add_steps][mode as usize]);
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
                for &x in &l.vec {
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
    while v.len() < PARAM_WORDS {
        v.push(0);
    }
    v
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
const PARAM_WORDS: usize = 296;

thread_local! {
    /// the depth of [`de_probe`] (a parameter of the shader)
    static PROBE_MZZ: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
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
    let t = std::time::Instant::now();
    let r = run(g, p, &sh, post, None, progress, cancel, &mut sink);
    if timing {
        eprintln!("  GPU run: {:.2} s", t.elapsed().as_secs_f64());
    }
    let r = r.and_then(|v| {
        // pixels whose DE overflowed single precision: a formula not made
        // for f32 (folds of huge values); a few are harmless, many are not
        let n = OVERFLOW.with(|c| c.get());
        if timing {
            eprintln!("  DE overflow in {n} of {} pixels", v.len());
        }
        if n * 500 > v.len() {
            Err(format!("single precision overflows in {} ({n} of {} pixels)", p.slots.iter().filter(|s| s.iterations != 0).map(|s| s.formula.name()).collect::<Vec<_>>().join(" + "), v.len()))
        } else {
            Ok(v)
        }
    });
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
    pub origin: [f32; 3],
    pub dir: [f32; 3],
    pub zz: f32,
    pub pixel: u32,
}

/// The result of a task: the G-buffer value of the hit (normal, depth,
/// colour, shadow bits, ambient occlusion), its position, the view depth,
/// the distance along the ray, and what happened.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReflectHit {
    pub si: SiLight,
    pub pos: [f32; 3],
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

/// Runs the tasks of the reflection pass on the graphics card (the march
/// of every bounce with the normals, colour, shadows and ambient occlusion
/// of its hit, as `reflect.rs` does on the CPU).
pub fn reflect_batch(p: &CalcParams, tasks: &[ReflectTask], o: &ReflectOpts) -> Result<Vec<ReflectHit>, String> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }
    if let Some(r) = unsupported(p) {
        return Err(r);
    }
    let g = gpu()?;
    let sh = scene_shader(p)?;
    let difs = p.difs || p.decomb.as_ref().is_some_and(|d| d.part2.difs);
    // the shadow and occlusion states only where the batch uses them, and
    // then in a pipeline of their own after the march (MB3D_GPU_NOSPLIT:
    // one pipeline, for comparisons): smaller shaders keep more rays on
    // the card at once
    let r_hs = !o.lights.is_empty() || o.soft_radius > 0.0;
    let r_ao = o.ao.is_some();
    let split = (r_hs || r_ao) && std::env::var_os("MB3D_GPU_NOSPLIT").is_none();
    let kind = |march: bool, hs: bool, ao: bool| {
        pipeline(
            g,
            &sh.code,
            &[
                ("MODE_KIND", 3.0),
                ("HAS_DIFS", difs as u8 as f64),
                ("HAS_DECOMB", p.decomb.is_some() as u8 as f64),
                ("HAS_4D", four_d(p) as u8 as f64),
                ("HAS_IPOL", has_ipol(p) as u8 as f64),
                ("R_HS", hs as u8 as f64),
                ("R_AO", ao as u8 as f64),
                ("R_MARCH", march as u8 as f64),
            ],
        )
    };
    let mut pls = vec![if split { kind(true, false, false)? } else { kind(true, r_hs, r_ao)? }];
    if split {
        pls.push(kind(false, r_hs, r_ao)?);
    }
    let n = tasks.len();
    let words: Vec<u32> = tasks
        .iter()
        .flat_map(|t| {
            let mut w = [0u32; 24];
            for k in 0..3 {
                w[k] = t.origin[k].to_bits();
                w[3 + k] = t.dir[k].to_bits();
            }
            w[6] = t.zz.to_bits();
            w[7] = t.pixel;
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
    let mut res = Resumer::new(g, &sh);
    let par = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("params"), size: (PARAM_WORDS * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let mut base = params(p, 0, n.div_ceil(R_W), &sh, 4, &PostJob::default(), Some((o, n)));
    base[SLOTS_WORD] = split as u32;
    for pl in &pls {
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
        res.run(
            g,
            &par,
            base.clone(),
            n,
            &|| false,
            |enc, groups| {
                let mut pass = enc.begin_compute_pass(&Default::default());
                pass.set_pipeline(pl);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(groups, 1, 1);
            },
            |_| Ok(()),
        )?;
    }
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
        data.chunks_exact(96)
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
                    pos: [f(13), f(14), f(15)],
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
    Ok(out)
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
    let v = run(g, p, &sh, &PostJob::default(), Some(3), &|_, _| {}, &|| false, &mut |_, _| {})?;
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
) -> Result<Vec<SiLight>, String> {
    let (w, h) = (p.rect[2].max(0) as usize, p.rect[3].max(0) as usize);
    let mut res = Resumer::new(g, sh);
    let modes = only_mode.map(|m| vec![m]).unwrap_or_else(|| modes(post));
    // the rows of a job: as many as the G-buffer allows (a job ends with
    // its slowest pixels alone on the card); with post passes (which need
    // the march of the band done) a few slot generations, so that the rows
    // still come in steps
    let band = if modes.len() == 1 { (96 << 20) / (w.max(1) * 20) } else { 4 * res.slots / w.max(1) }.clamp(8, h.max(8));
    let out_size = (w * band * 20) as u64;
    let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor { label: Some("gbuffer"), size: out_size.max(16), usage, mapped_at_creation: false });
    let read = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read"),
        size: out_size.max(16),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // one parameter buffer per pass (a buffer write before a submission
    // takes effect before all of its passes), and a pipeline per kind of pass
    let difs = p.difs || p.decomb.as_ref().is_some_and(|d| d.part2.difs);
    let decomb = p.decomb.is_some();
    let mut pls = Vec::new();
    for &m in &modes {
        let kind = match m {
            1 => 1.0,
            2 => 2.0,
            _ => 0.0,
        };
        pls.push(pipeline(g, &sh.code, &[("MODE_KIND", kind), ("HAS_DIFS", difs as u8 as f64), ("HAS_DECOMB", decomb as u8 as f64), ("HAS_4D", four_d(p) as u8 as f64), ("HAS_IPOL", has_ipol(p) as u8 as f64)])?);
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
    let maps = words("maps", &sh.maps);
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
    let mut row0 = 0;
    while row0 < h {
        if cancel() {
            return Err("cancelled".into());
        }
        let rows = band.min(h - row0);
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
                    progress(row0 + sunk, h);
                    Ok(())
                },
            )?;
        }
        row0 += rows;
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
        let transient = ["state_slot", "cur_item", "resuming", "suspended", "r_task_origin", "r_task_dir", "r_task", "r_rec", "r_keep", "dv"];
        let mut names: Vec<&str> = code.lines().filter_map(|l| l.strip_prefix("var<private> ")).map(|l| l.split(':').next().unwrap()).collect();
        let body = &code[code.find("fn pixel(").unwrap()..];
        let lp = body.find("    for (var turns").unwrap();
        names.extend(body[..lp].lines().filter_map(|l| l.strip_prefix("    var ")).map(|l| l.split([':', ' ', ';']).next().unwrap()));
        let load = &body[body.find("if (resuming) {").unwrap()..lp];
        let save = &body[lp..body.find("save_record(").unwrap()];
        let has = |block: &str, n: &str| block.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == n);
        for n in names.iter().filter(|n| !n.starts_with("dbg_") && !transient.contains(n)) {
            assert!(has(save, n), "{n} is not saved");
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
