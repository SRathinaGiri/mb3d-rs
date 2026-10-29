//! The preview of parameter files in the open dialogs (not in MB3D) and
//! of the History window's entries: a clicked `.m3p`, `.m3i` or text
//! parameter file is calculated as a small picture in a background
//! thread, on the CPU (a new formula's shader would take seconds to
//! compile for a tiny picture), an image file is read and scaled down
//! there, and the result is shown in the dialog's preview column (or the
//! window's image) with the file's size and formulas.  The History
//! window's rows and tiles ask for many pictures at once ([`thumb`]):
//! they queue up and are calculated one at a time, the most wanted first.
//! The pictures are kept for the session (read again when the file
//! changed).

use super::Mb3d;
use crate::vcl::lang::{tr, trf};
use crate::vcl::{bitmap::Bitmap, Ui};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

const DLG: &str = "FileDlg";

/// The preview size (the dialog's `PreviewImage`, the History window's).
pub const W: usize = 240;
pub const H: usize = 180;

/// A finished preview: the picture (or why there is none) and its text.
type Done = (PathBuf, Result<(Vec<u8>, usize, usize), String>, String);

/// The form, image and label a preview goes to.
type Target = (String, String, String);

#[derive(Default)]
pub struct State {
    cache: HashMap<PathBuf, (Option<Bitmap>, String, Option<SystemTime>)>,
    /// the file shown, and where
    shown: Option<(PathBuf, Target)>,
    /// the files wanted and not calculated yet, the first next
    queue: Vec<PathBuf>,
    busy: bool,
    result: Arc<Mutex<Option<Done>>>,
    /// counts the finished previews: a view of many pictures paints again
    /// when it changed
    pub version: u64,
}

fn is_params(p: &Path) -> bool {
    matches!(p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref(), Some("m3p" | "m3i" | "txt"))
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
fn calculate(path: &Path) -> (Result<(Vec<u8>, usize, usize), String>, String) {
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
    let m = match crate::m3p::load(path) {
        Ok(m) => m,
        Err(e) => return (Err(e), String::new()),
    };
    let text = describe(&m.scene);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).saturating_sub(1).max(1);
    (crate::mutagen::preview(&m.scene, W, H, false, threads), text)
}

fn start(app: &mut Mb3d, ui: &Ui, path: PathBuf) {
    app.preview.busy = true;
    let slot = app.preview.result.clone();
    let waker = ui.waker();
    std::thread::spawn(move || {
        // (a panic must not leave `busy` set: no more previews then)
        let (r, text) = std::panic::catch_unwind(|| calculate(&path)).unwrap_or_else(|_| (Err(tr("the calculation failed")), String::new()));
        *slot.lock().unwrap() = Some((path, r, text));
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
    if app.preview.cache.get(path).is_some_and(|(_, _, m)| *m != modified(path)) {
        app.preview.cache.remove(path);
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
    match app.preview.cache.get(path) {
        Some((b, _, _)) => b.clone(),
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
        show(ui, &t, None, &tr("No preview for this file."));
        return;
    }
    freshen(app, &path);
    app.preview.shown = Some((path.clone(), t.clone()));
    match app.preview.cache.get(&path) {
        Some((b, text, _)) => show(ui, &t, b.clone(), text),
        None => {
            show(ui, &t, None, &tr("Calculating the preview..."));
            want(app, path, true);
        }
    }
}

/// The idle work: a file clicked in a preview dialog, a finished preview,
/// the next of the queue.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if let Some(p) = ui.file_preview.take() {
        let t = dialog_target();
        if is_params(&p) {
            request(app, ui, p, &t.0, &t.1, &t.2);
        } else {
            app.preview.shown = None;
            show(ui, &t, None, &tr("No preview for this file."));
        }
    }
    let done = app.preview.result.lock().unwrap().take();
    if let Some((p, r, text)) = done {
        app.preview.busy = false;
        let entry = match r {
            Ok((rgb, w, h)) => (Some(Bitmap::from_rgb(w, h, &rgb)), text),
            Err(e) => (None, trf("No preview: {}", &[&e])),
        };
        if let Some((shown, t)) = &app.preview.shown {
            if *shown == p {
                show(ui, &t.clone(), entry.0.clone(), &entry.1);
            }
        }
        let m = modified(&p);
        app.preview.cache.insert(p, (entry.0, entry.1, m));
        app.preview.version += 1;
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
