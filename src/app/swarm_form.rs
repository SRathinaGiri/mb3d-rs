//! The Swarm window (`SwarmFrm`, this program's own): the shared folder,
//! whether this computer takes part, who is in the swarm and what the jobs
//! are at. While "Share this computer" is on, the worker runs in a thread
//! of this process and this computer's own renders (Calculate 3D, Big
//! renders, Monte Carlo, animations) go to the swarm when another computer
//! is in it.

use super::eta::ItemsLeft;
use super::util::{left_str, smooth_shown, Shown};
use super::{Mb3d, Ui};
use crate::vcl::{DialogResult, Event};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const F: &str = "SwarmFrm";

struct Worker {
    /// the folder and the name it shares under
    dir: PathBuf,
    name: String,
    control: Arc<crate::swarm::WorkerControl>,
    /// the worker's error, if it ended by itself
    error: Arc<Mutex<Option<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

pub struct State {
    pub folder: String,
    pub name: String,
    pub threads: usize,
    pub share: bool,
    /// Calculate 3D (and the Batch window's renders) with the swarm, in
    /// `tiles` (columns, rows)
    pub tiles: (u32, u32),
    worker: Option<Worker>,
    /// the shown lists, to avoid repainting unchanged ones
    shown: (Vec<Vec<String>>, Vec<Vec<String>>),
    /// the job whose picture `Image1` shows
    preview_of: Option<String>,
    /// the Monte Carlo blocks merged for the preview
    merge: crate::swarm::BlockMerge,
    /// the time left of the job shown: the units finished while the window
    /// looked (the clock's time between them: the computers work side by
    /// side), how many were done when, and the time left as shown
    units: ItemsLeft,
    units_seen: Option<(String, usize, std::time::Instant)>,
    left_shown: Option<Shown>,
}

impl Default for State {
    fn default() -> Self {
        State {
            folder: String::new(),
            name: String::new(),
            threads: 0,
            share: false,
            tiles: (4, 3),
            worker: None,
            shown: (Vec::new(), Vec::new()),
            preview_of: None,
            merge: crate::swarm::BlockMerge::default(),
            units: ItemsLeft::default(),
            units_seen: None,
            left_shown: None,
        }
    }
}

impl State {
    pub fn from_ini(ini: &super::ini::Ini) -> State {
        let g = |k: &str| ini.get_extra(k).unwrap_or("").to_string();
        State {
            folder: g("SwarmFolder"),
            name: g("SwarmName"),
            threads: g("SwarmThreads").parse().unwrap_or(0),
            // sharing is never on by itself at the start
            share: false,
            tiles: {
                let (c, r) = super::ini::two_ints(&g("SwarmTiles"), (4, 3));
                (c.clamp(1, 64) as u32, r.clamp(1, 64) as u32)
            },
            ..Default::default()
        }
    }

    /// The swarm folder, if one is set and exists.
    pub fn folder(&self) -> Option<PathBuf> {
        let p = PathBuf::from(self.folder.trim());
        (!self.folder.trim().is_empty() && p.is_dir()).then_some(p)
    }

    /// Sharing is on (a worker runs and takes units).
    pub fn sharing(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| !w.control.stop.load(Ordering::Relaxed))
    }
    /// A worker still finishes its last unit after sharing was switched off.
    pub fn finishing(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.control.stop.load(Ordering::Relaxed) && w.thread.as_ref().is_some_and(|t| !t.is_finished()))
    }
}

fn save_ini(app: &mut Mb3d) {
    let s = &app.swarm;
    app.ini.set_extra("SwarmFolder", &s.folder);
    app.ini.set_extra("SwarmName", &s.name);
    app.ini.set_extra("SwarmThreads", &s.threads.to_string());
    app.ini.set_extra("SwarmTiles", &format!("{} {}", s.tiles.0, s.tiles.1));
}

/// Where this computer's renders go to the swarm: while it shares, and
/// only when another computer is there to help (alone, a render with the
/// swarm would only be slower).
fn render_folder(app: &Mb3d) -> Option<PathBuf> {
    if !app.swarm.sharing() {
        return None;
    }
    app.swarm.folder().filter(|dir| crate::swarm::others_online(dir, &app.swarm.name))
}

/// Whether Calculate 3D should render with the swarm, and where.
pub fn calc_folder(app: &Mb3d) -> Option<PathBuf> {
    render_folder(app)
}

/// Whether the Animation maker should render with the swarm, and where.
pub fn anim_folder(app: &Mb3d) -> Option<PathBuf> {
    render_folder(app)
}

/// Fills the window from the state (before it is shown).
pub fn opening(app: &mut Mb3d, ui: &mut Ui) {
    let s = &app.swarm;
    ui.set_text(F, "Edit1", &s.folder);
    let name = if s.name.is_empty() { crate::swarm::machine_name() } else { s.name.clone() };
    ui.set_text(F, "Edit2", &name);
    ui.set_text(F, "Edit3", &s.threads.to_string());
    ui.set_checked(F, "CheckBox1", s.sharing());
    ui.set_text(F, "Edit5", &s.tiles.0.to_string());
    ui.set_text(F, "Edit6", &s.tiles.1.to_string());
}

/// Reads the fields into the state.
fn read_fields(app: &mut Mb3d, ui: &Ui) {
    let s = &mut app.swarm;
    s.folder = ui.text(F, "Edit1").trim().to_string();
    s.name = ui.text(F, "Edit2").trim().to_string();
    s.threads = ui.text(F, "Edit3").trim().parse().unwrap_or(0);
    let n = |e: &str| ui.text(F, e).trim().parse::<u32>().map(|v| v.clamp(1, 64)).ok();
    if let (Some(c), Some(r)) = (n("Edit5"), n("Edit6")) {
        s.tiles = (c, r);
    }
}

fn start_worker(app: &mut Mb3d) -> Result<(), String> {
    let Some(dir) = app.swarm.folder() else {
        return Err("choose a shared folder first (one every computer of the swarm can see)".into());
    };
    std::fs::create_dir_all(dir.join("jobs")).map_err(|e| format!("{}: {e}", dir.display()))?;
    // a worker that still finishes its last unit in the same folder goes
    // on sharing, instead of a second one beside it
    if let Some(w) = app.swarm.worker.as_ref() {
        let running = w.thread.as_ref().is_some_and(|t| !t.is_finished()) && !w.control.abort.load(Ordering::SeqCst);
        if running && w.dir == dir && w.name == app.swarm.name {
            w.control.stop.store(false, Ordering::SeqCst);
            return Ok(());
        }
    }
    // another one is given up and waited for
    stop_worker(app, true);
    let control = Arc::new(crate::swarm::WorkerControl::default());
    let error = Arc::new(Mutex::new(None));
    let (name, threads) = (app.swarm.name.clone(), app.swarm.threads);
    let (control2, error2, dir2) = (control.clone(), error.clone(), dir.clone());
    let thread = std::thread::spawn(move || {
        if let Err(e) = crate::swarm::worker_controlled(&dir2, &name, threads, false, true, &control2) {
            *error2.lock().unwrap_or_else(|e| e.into_inner()) = Some(e);
        }
    });
    app.swarm.worker = Some(Worker { dir, name: app.swarm.name.clone(), control, error, thread: Some(thread) });
    Ok(())
}

/// Sharing off: the worker takes no further unit. With `abort` (the
/// program closes) its current unit is given up at once and the thread
/// waited for; otherwise the thread goes on until that unit is done and
/// is reaped by the window's refresh.
pub fn stop_worker(app: &mut Mb3d, abort: bool) {
    let Some(w) = app.swarm.worker.as_mut() else { return };
    w.control.stop.store(true, Ordering::SeqCst);
    if abort {
        w.control.abort.store(true, Ordering::SeqCst);
    }
    let finished = w.thread.as_ref().is_some_and(|t| t.is_finished());
    if abort || finished {
        if let Some(mut w) = app.swarm.worker.take() {
            if let Some(t) = w.thread.take() {
                let _ = t.join();
            }
        }
    }
}

/// The lists: who is in the swarm, what the jobs are at.
fn refresh(app: &mut Mb3d, ui: &mut Ui) {
    read_fields(app, ui);
    let worker_error = app.swarm.worker.as_ref().and_then(|w| w.error.lock().ok().and_then(|e| e.clone()));
    if let Some(e) = worker_error {
        // the worker ended by itself (the folder went away, ...)
        stop_worker(app, true);
        ui.set_checked(F, "CheckBox1", false);
        ui.set_caption(F, "Label7", &format!("sharing stopped: {e}"));
    }
    // a worker that ended just as sharing was switched on again
    if app.swarm.sharing() && app.swarm.worker.as_ref().is_some_and(|w| w.thread.as_ref().is_some_and(|t| t.is_finished())) {
        stop_worker(app, true);
        if let Err(e) = start_worker(app) {
            ui.set_checked(F, "CheckBox1", false);
            ui.set_caption(F, "Label7", &format!("sharing stopped: {e}"));
        }
    }
    // a worker that finished its last unit after sharing was switched off
    if app.swarm.worker.as_ref().is_some_and(|w| w.control.stop.load(Ordering::Relaxed) && w.thread.as_ref().is_some_and(|t| t.is_finished())) {
        stop_worker(app, true);
    }
    let note = app.swarm.worker.as_ref().map(|w| w.control.note()).unwrap_or_default();
    let Some(dir) = app.swarm.folder() else {
        set_rows(ui, "ListView1", Vec::new(), &mut app.swarm.shown.0);
        set_rows(ui, "ListView2", Vec::new(), &mut app.swarm.shown.1);
        ui.set_caption(F, "Label7", if app.swarm.folder.is_empty() { "no shared folder chosen" } else { "the folder does not exist (or is not reachable)" });
        return;
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut peers = crate::swarm::peers(&dir);
    // this computer's own worker, before its first heartbeat is written
    if app.swarm.sharing() && !peers.iter().any(|p| !p.stale(now) && p.name == app.swarm.name) {
        peers.insert(
            0,
            crate::swarm::Peer {
                id: String::new(),
                name: app.swarm.name.clone(),
                version: env!("CARGO_PKG_VERSION").into(),
                cores: std::thread::available_parallelism().map_or(1, |n| n.get()),
                card: String::new(),
                state: "starting".into(),
                time: now,
            },
        );
    }
    let rows: Vec<Vec<String>> = peers
        .iter()
        .map(|p| {
            let gone = p.stale(now);
            vec![
                p.name.clone(),
                p.version.clone(),
                p.cores.to_string(),
                if p.card.is_empty() { "processor".into() } else { p.card.clone() },
                if gone { format!("gone (last seen {} s ago)", now.saturating_sub(p.time)) } else { p.state.clone() },
            ]
        })
        .collect();
    set_rows(ui, "ListView1", rows, &mut app.swarm.shown.0);
    let jobs: Vec<Vec<String>> = crate::swarm::jobs(&dir)
        .into_iter()
        .filter(|j| !j.done())
        .map(|j| {
            let (d, n) = j.progress();
            let errs = j.errors();
            vec![
                j.name.clone(),
                j.kind_name().to_string(),
                j.owner.clone(),
                format!(
                    "{d} of {n} units done{}{}",
                    if j.same_version() { String::new() } else { format!(", version {}", j.version) },
                    if errs.is_empty() { String::new() } else { format!(", error at {}: {}", errs[0].0, errs[0].1) }
                ),
            ]
        })
        .collect();
    set_rows(ui, "ListView2", jobs, &mut app.swarm.shown.1);
    show_preview(app, ui, &dir);
    // one sentence: whether this computer shares, and with whom
    let others = crate::swarm::others(&dir, &app.swarm.name);
    let names = |v: &[&crate::swarm::Peer]| -> String {
        let n: Vec<&str> = v.iter().map(|p| p.name.as_str()).collect();
        match n.len() {
            0 => String::new(),
            1 => n[0].to_string(),
            k => format!("{} and {}", n[..k - 1].join(", "), n[k - 1]),
        }
    };
    let helpers: Vec<&crate::swarm::Peer> = others.iter().filter(|p| p.version == env!("CARGO_PKG_VERSION")).collect();
    let foreign: Vec<&crate::swarm::Peer> = others.iter().filter(|p| p.version != env!("CARGO_PKG_VERSION")).collect();
    let mut text = if app.swarm.sharing() {
        if helpers.is_empty() {
            "Sharing, but no other computer is in the swarm yet: renders run on this computer alone".to_string()
        } else {
            format!("Sharing with {}: renders are split between the {} computers", names(&helpers), helpers.len() + 1)
        }
    } else if app.swarm.finishing() {
        "Stopping: this computer finishes the part it is rendering".to_string()
    } else if helpers.is_empty() {
        "Not sharing: tick \"Share this computer\" to render together with other computers".to_string()
    } else {
        format!("Not sharing: tick \"Share this computer\" to render together with {}", names(&helpers))
    };
    if !foreign.is_empty() {
        text += &format!("; {} cannot take part (another version of the program)", names(&foreign));
    }
    if !note.is_empty() {
        text += &format!("; {note}");
    }
    ui.set_caption(F, "Label7", &text);
}

/// The newest job's picture so far in `Image1`: tiles rendered, under way
/// (with a progress bar), missing.
fn show_preview(app: &mut Mb3d, ui: &mut Ui, dir: &Path) {
    let job = crate::swarm::jobs(dir).into_iter().filter(|j| !j.done()).next_back();
    let Some(job) = job else {
        if app.swarm.preview_of.take().is_some() {
            ui.set_picture(F, "Image1", None);
            ui.set_caption(F, "Label8", "");
        }
        return;
    };
    let (iw, ih) = (ui.c(F, "Image1").width.max(16) as usize, ui.c(F, "Image1").height.max(16) as usize);
    let Some(mut p) = crate::swarm::job_preview_merged(&job, iw, Some(&mut app.swarm.merge)) else {
        ui.set_caption(F, "Label8", &format!("{}: nothing rendered yet", job.name));
        return;
    };
    // a progress bar at the bottom of every unit under way
    for (r, f) in &p.working {
        let bar = (r[2] as f32 * f.clamp(0.0, 1.0)) as usize;
        for y in (r[1] + r[3]).saturating_sub(4)..(r[1] + r[3]).min(p.height) {
            for x in r[0]..(r[0] + r[2]).min(p.width) {
                let i = (y * p.width + x) * 3;
                p.rgb[i..i + 3].copy_from_slice(if x - r[0] < bar { &[90, 200, 90] } else { &[50, 50, 60] });
            }
        }
    }
    // tall pictures: fit the height too
    if p.height > ih {
        let (rgb, w, h) = crate::swarm::fit_rgb(&p.rgb, p.width, p.height, p.width * ih / p.height);
        p.rgb = rgb;
        p.width = w;
        p.height = h;
    }
    ui.set_picture(F, "Image1", Some(crate::vcl::bitmap::Bitmap::from_rgb(p.width, p.height, &p.rgb)));
    let what = match job.kind_name() {
        "tiles" => "tiles",
        "montecarlo" => "blocks",
        "animation" => "frames",
        _ => "pictures",
    };
    let left = units_left(&mut app.swarm, &job.id, p.done, p.total, std::time::Instant::now()).map(|l| format!(" · {}", left_str(l))).unwrap_or_default();
    ui.set_caption(F, "Label8", &format!("{} ({}, by {}): {} of {} {what} done{}{left}", job.name, job.kind_name(), job.owner, p.done, p.total, if p.working.is_empty() { String::new() } else { format!(", {} under way", p.working.len()) }));
    app.swarm.preview_of = Some(job.id);
}

/// The time left in the job `id` with `done` of `total` units at `now`:
/// from the units finished since the window first saw the job (None before
/// one did, or once all are done).
fn units_left(sw: &mut State, id: &str, done: usize, total: usize, now: std::time::Instant) -> Option<f64> {
    match &mut sw.units_seen {
        Some((seen, d, at)) if seen == id => {
            if done > *d {
                let each = now.saturating_duration_since(*at).as_secs_f64() / (done - *d) as f64;
                for _ in *d..done {
                    sw.units.done(each);
                }
                *d = done;
                *at = now;
            }
            let left = (done < total).then(|| sw.units.left(0.0, now.saturating_duration_since(*at).as_secs_f64(), None, total - done - 1)).flatten();
            sw.left_shown = left.map(|l| smooth_shown(sw.left_shown, l, now)).or(sw.left_shown);
            left.and(sw.left_shown.map(|s| s.0))
        }
        _ => {
            sw.units_seen = Some((id.to_string(), done, now));
            sw.units = ItemsLeft::default();
            sw.left_shown = None;
            None
        }
    }
}

fn set_rows(ui: &mut Ui, ctl: &str, rows: Vec<Vec<String>>, shown: &mut Vec<Vec<String>>) {
    if *shown == rows {
        return;
    }
    let c = ui.cm(F, ctl);
    c.checks.clear();
    c.cells = rows.clone();
    if c.item_index >= c.cells.len() as i32 {
        c.item_index = c.cells.len() as i32 - 1;
    }
    *shown = rows;
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            opening(app, ui);
            refresh(app, ui);
            ui.cm(F, "Timer1").enabled = true;
        }
        "Timer1Timer" => {
            if ui.showing(F) {
                refresh(app, ui);
            }
        }
        "Button1Click" => {
            let cur = app.swarm.folder().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            ui.folder_dialog("swarm:folder", "The shared folder of the swarm", Some(cur));
        }
        "CheckBox1Click" => {
            read_fields(app, ui);
            if ui.checked(F, "CheckBox1") {
                if let Err(e) = start_worker(app) {
                    ui.set_checked(F, "CheckBox1", false);
                    app.message(ui, &e);
                }
            } else {
                stop_worker(app, false);
            }
            save_ini(app);
            refresh(app, ui);
        }
        "Edit1Change" | "Edit2Change" | "Edit3Change" | "Edit5Change" | "Edit6Change" => {
            read_fields(app, ui);
            save_ini(app);
        }
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("folder", DialogResult::File(Some(p))) = (what, r) {
        ui.set_text(F, "Edit1", &p.display().to_string());
        read_fields(app, ui);
        save_ini(app);
        refresh(app, ui);
    }
}

/// Whether the Monte Carlo window should render with the swarm, and where.
pub fn mc_folder(app: &Mb3d) -> Option<PathBuf> {
    render_folder(app)
}

/// A short text for titles: "swarm: 3 computers".
pub fn summary(dir: &Path) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let n = crate::swarm::peers(dir).iter().filter(|p| !p.stale(now)).count();
    format!("swarm: {n} computer{}", if n == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_left_from_the_units_finished() {
        let t0 = std::time::Instant::now();
        let at = |s: f64| t0 + std::time::Duration::from_secs_f64(s);
        let mut sw = State::default();
        // the job first seen with 2 of 12 done: nothing to say yet
        assert_eq!(units_left(&mut sw, "job1", 2, 12, at(0.0)), None);
        // two more after 20 s (10 s each): the running one and 7 more
        let l = units_left(&mut sw, "job1", 4, 12, at(20.0)).unwrap();
        assert!((l - 80.0).abs() < 1e-6, "{l}");
        // another job: from the start again
        assert_eq!(units_left(&mut sw, "job2", 0, 3, at(21.0)), None);
        let l = units_left(&mut sw, "job2", 1, 3, at(26.0)).unwrap();
        assert!((l - 10.0).abs() < 1e-6, "{l}");
        assert_eq!(units_left(&mut sw, "job2", 3, 3, at(36.0)), None);
    }
}
