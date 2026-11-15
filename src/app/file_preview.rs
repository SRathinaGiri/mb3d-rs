//! The preview of parameter files in the open dialogs (not in MB3D) and
//! of the History window's entries: a clicked `.m3p`, `.m3i` or text
//! parameter file is calculated as a small picture in a background
//! thread, on the CPU (a new formula's shader would take seconds to
//! compile for a tiny picture), an image file is read and scaled down
//! there, and the result is shown in the dialog's preview column (or the
//! window's image) with the file's size and formulas.  The History
//! window's rows and tiles ask for many pictures at once ([`thumb`]):
//! they queue up and are calculated one at a time, the most wanted first.
//! A preview leaves out the reflections (a scene of a slow formula took
//! minutes for them, issue 32), and the one asked for by a dialog or the
//! History window's image stops when that shows something else or closes.
//! The last [`KEEP`] pictures are kept (read again when the file
//! changed).

use crate::sync::Locked;
use super::Mb3d;
use crate::vcl::lang::{tr, trf};
use crate::vcl::{bitmap::Bitmap, Ui};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const DLG: &str = "FileDlg";

/// The preview size (the dialog's `PreviewImage`, the History window's).
pub const W: usize = 240;
pub const H: usize = 180;

/// A preview's picture (RGB, width, height) or why there is none.
type Picture = Result<(Vec<u8>, usize, usize), String>;

/// A finished preview: the picture (or why there is none) and its text.
type Done = (PathBuf, Picture, String);

/// The form, image and label a preview goes to.
type Target = (String, String, String);

/// How many pictures are kept (a history's picture is 240 x 180 x 4
/// bytes: 170 kB each, 70 MB in all).
pub const KEEP: usize = 400;

/// A preview kept: the picture (None: none could be made), its text,
/// the file's time, and when it was last wanted.
struct Cached {
    picture: Option<Bitmap>,
    text: String,
    modified: Option<SystemTime>,
    used: u64,
}

/// The preview being calculated: its file, its stop, and the view that
/// asked for it (None: only the History window's queue).
struct Job {
    path: PathBuf,
    cancel: Arc<AtomicBool>,
    view: Option<Target>,
}

#[derive(Default)]
pub struct State {
    cache: HashMap<PathBuf, Cached>,
    /// counts the uses of the cache: the picture wanted longest ago goes
    /// first
    tick: u64,
    /// the file shown, and where
    shown: Option<(PathBuf, Target)>,
    /// the files wanted and not calculated yet, the first next
    queue: Vec<PathBuf>,
    busy: bool,
    /// the preview being calculated
    job: Option<Job>,
    result: Arc<Mutex<Option<Done>>>,
    /// counts the finished previews: a view of many pictures paints again
    /// when it changed
    pub version: u64,
}

fn is_params(p: &Path) -> bool {
    matches!(p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref(), Some("m3p" | "m3i" | "txt" | "fract"))
}

fn is_image(p: &Path) -> bool {
    crate::history::Kind::of(p) == crate::history::Kind::Image
}

fn dialog_target() -> Target {
    (DLG.into(), "PreviewImage".into(), "PreviewLabel".into())
}

fn show(ui: &mut Ui, t: &Target, picture: Option<Bitmap>, text: &str) {
    if !ui.showing(&t.0) || ui.f(&t.0).id(&t.1).is_none() {
        return;
    }
    ui.set_picture(&t.0, &t.1, picture);
    ui.set_caption(&t.0, &t.2, text);
}

/// The size and formulas of a scene, the preview's text.
fn describe(sc: &crate::scene::Scene) -> String {
    let names: Vec<String> = sc.formulas.iter().filter(|f| f.iterations > 0).map(|f| f.formula.name().to_string()).collect();
    format!("{} x {}\n{}", sc.width, sc.height, names.join(", "))
}

/// Calculates the preview of `path` (in the calling thread): parameters
/// rendered small, an image read and scaled down.
fn calculate(path: &Path, cancel: &(dyn Fn() -> bool + Sync)) -> (Picture, String) {
    if is_image(path) {
        return match crate::image::load(path) {
            Ok(img) => {
                let rgb: Vec<u8> = img.data.iter().flat_map(|p| p.map(|v| if img.deep { (v >> 8) as u8 } else { v as u8 })).collect();
                let (t, w, h) = crate::history::downscale(&rgb, img.width, img.height, W, H);
                (Ok((t, w, h)), format!("{} x {}", img.width, img.height))
            }
            Err(e) => (Err(e), String::new()),
        };
    }
    // (any kind of parameters: MB3D's, text scenes, Mandelbulber's)
    let mut sc = match crate::animfile::load_scene(path) {
        Ok((sc, _)) => sc,
        Err(e) => return (Err(e), String::new()),
    };
    let text = describe(&sc);
    // no reflections: with a slow formula they take minutes even this small
    sc.mc.reflections = false;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).saturating_sub(1).max(1);
    (crate::mutagen::preview_cancellable(&sc, W, H, false, threads, cancel), text)
}

fn start(app: &mut Mb3d, ui: &Ui, path: PathBuf) {
    app.preview.busy = true;
    let slot = app.preview.result.clone();
    let waker = ui.waker();
    let cancel = Arc::new(AtomicBool::new(false));
    // (a window that closed since showing the file does not wait for it:
    // the History queue's picture of the file must not stop for it)
    let view = app.preview.shown.as_ref().filter(|(p, t)| *p == path && ui.showing(&t.0)).map(|(_, t)| t.clone());
    app.preview.job = Some(Job { path: path.clone(), cancel: cancel.clone(), view });
    std::thread::spawn(move || {
        let stop = || cancel.load(Ordering::Relaxed);
        // (a panic must not leave `busy` set: no more previews then)
        let (r, text) = std::panic::catch_unwind(|| calculate(&path, &stop)).unwrap_or_else(|_| (Err(tr("@MB:TheCalculationFailedLower")), String::new()));
        *slot.locked() = Some((path, r, text));
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}

/// Whether a preview can be made of the file.
pub fn possible(p: &Path) -> bool {
    is_params(p) || is_image(p)
}

/// Forgets a cached preview whose file changed since.
fn freshen(app: &mut Mb3d, path: &Path) {
    if app.preview.cache.get(path).is_some_and(|c| c.modified != modified(path)) {
        app.preview.cache.remove(path);
    }
}

/// The cached preview of `path`, noted as wanted now.
fn cached(app: &mut Mb3d, path: &Path) -> Option<(Option<Bitmap>, String)> {
    app.preview.tick += 1;
    let tick = app.preview.tick;
    let c = app.preview.cache.get_mut(path)?;
    c.used = tick;
    Some((c.picture.clone(), c.text.clone()))
}

/// Keeps the cache at [`KEEP`] pictures: the ones wanted longest ago go.
fn trim(app: &mut Mb3d) {
    while app.preview.cache.len() > KEEP {
        let Some(oldest) = app.preview.cache.iter().min_by_key(|(_, c)| c.used).map(|(p, _)| p.clone()) else { break };
        app.preview.cache.remove(&oldest);
    }
}

/// Puts `path` into the queue: in front (wanted next) or at the end.
fn want(app: &mut Mb3d, path: PathBuf, first: bool) {
    if app.preview.cache.contains_key(&path) {
        return;
    }
    app.preview.queue.retain(|p| *p != path);
    if first {
        app.preview.queue.insert(0, path);
    } else {
        app.preview.queue.push(path);
    }
}

/// The queue is emptied: a view asks again for what it shows.
pub fn clear_queue(app: &mut Mb3d) {
    app.preview.queue.clear();
}

/// The picture of `path` from the cache; None while it is not there yet
/// (then it is queued, behind what was asked before) or when none could
/// be made.
pub fn thumb(app: &mut Mb3d, path: &Path) -> Option<Bitmap> {
    freshen(app, path);
    match cached(app, path) {
        Some((b, _)) => b,
        None => {
            if possible(path) {
                want(app, path.to_path_buf(), false);
            }
            None
        }
    }
}

/// Shows the preview of `path` in `image` of `form` and its text in
/// `label`: from the session's cache, else calculated next.
pub fn request(app: &mut Mb3d, ui: &mut Ui, path: PathBuf, form: &str, image: &str, label: &str) {
    let t: Target = (form.into(), image.into(), label.into());
    if !possible(&path) {
        app.preview.shown = None;
        show(ui, &t, None, &tr("@MB:NoPreviewForThisFile"));
        return;
    }
    freshen(app, &path);
    app.preview.shown = Some((path.clone(), t.clone()));
    match cached(app, &path) {
        Some((b, text)) => show(ui, &t, b, &text),
        None => {
            show(ui, &t, None, &tr("@MB:CalculatingThePreview"));
            want(app, path, true);
        }
    }
}

/// The idle work: a file clicked in a preview dialog, a finished preview,
/// the next of the queue.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    idle_work(app, ui);
    // a dialog that shows pictures as tiles: the ones in sight that are
    // there by now (the others are queued, and this runs again when one
    // is finished)
    for (k, path) in ui.file_tiles_wanted() {
        if let Some(b) = thumb(app, &path) {
            ui.set_file_tile(k, &path, &b);
        }
    }
}

fn idle_work(app: &mut Mb3d, ui: &mut Ui) {
    if let Some(p) = ui.file_preview.take() {
        let t = dialog_target();
        // (parameters, and pictures: the dialogs that choose a map or a
        // background picture, whose file names are numbers)
        if possible(&p) {
            request(app, ui, p, &t.0, &t.1, &t.2);
        } else {
            app.preview.shown = None;
            show(ui, &t, None, &tr("@MB:NoPreviewForThisFile"));
        }
    }
    // the view's preview nobody waits for any more: its window closed or
    // shows another file, and the History window does not want it either
    if let Some(j) = &app.preview.job {
        if let Some(t) = &j.view {
            let shown = ui.showing(&t.0) && app.preview.shown.as_ref().is_some_and(|(p, _)| *p == j.path);
            if !shown && !app.preview.queue.contains(&j.path) {
                j.cancel.store(true, Ordering::Relaxed);
            }
        }
    }
    let done = app.preview.result.locked().take();
    if let Some((p, r, text)) = done {
        app.preview.busy = false;
        let stopped = app.preview.job.take().is_some_and(|j| j.cancel.load(Ordering::Relaxed));
        // a stopped preview is not kept: it is calculated again when it is
        // wanted again
        if stopped && r.is_err() {
            crate::log::info("preview", &format!("stopped the preview of {} (not wanted any more)", p.display()));
            // the History window paints again and asks for what it still
            // shows (it may want the stopped file's picture as well)
            app.preview.version += 1;
        } else {
            keep(app, ui, p, r, text);
        }
    }
    if !app.preview.busy {
        while !app.preview.queue.is_empty() {
            let p = app.preview.queue.remove(0);
            if !app.preview.cache.contains_key(&p) {
                start(app, ui, p);
                break;
            }
        }
    }
}

/// A finished preview: shown when its view still shows the file, and kept.
fn keep(app: &mut Mb3d, ui: &mut Ui, p: PathBuf, r: Picture, text: String) {
    let entry = match r {
        Ok((rgb, w, h)) => (Some(Bitmap::from_rgb(w, h, &rgb)), text),
        Err(e) => (None, trf("@MB:NoPreview", &[&e])),
    };
    if let Some((shown, t)) = &app.preview.shown {
        if *shown == p {
            show(ui, &t.clone(), entry.0.clone(), &entry.1);
            super::formula_book_form::picture_caption(app, ui);
        }
    }
    let m = modified(&p);
    app.preview.tick += 1;
    let used = app.preview.tick;
    app.preview.cache.insert(p, Cached { picture: entry.0, text: entry.1, modified: m, used });
    trim(app);
    app.preview.version += 1;
}
