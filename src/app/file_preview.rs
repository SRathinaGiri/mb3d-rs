//! The preview of parameter files in the open dialogs (not in MB3D): a
//! clicked `.m3p`, `.m3i` or text parameter file is calculated as a small
//! picture in a background thread, on the CPU (a new formula's shader
//! would take seconds to compile for a tiny picture), and shown in the
//! dialog's preview column with the file's size and formulas.  One preview
//! is calculated at a time; while it runs, only the last clicked file
//! waits.  The pictures are kept for the session.

use super::Mb3d;
use crate::vcl::{bitmap::Bitmap, Ui};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const DLG: &str = "FileDlg";

/// The preview size (the dialog's `PreviewImage`).
const W: usize = 240;
const H: usize = 180;

/// A finished preview: the picture (or why there is none) and its text.
type Done = (PathBuf, Result<(Vec<u8>, usize, usize), String>, String);

#[derive(Default)]
pub struct State {
    cache: HashMap<PathBuf, (Option<Bitmap>, String)>,
    /// the file shown in the dialog
    shown: Option<PathBuf>,
    /// the last clicked file not calculated yet
    want: Option<PathBuf>,
    busy: bool,
    result: Arc<Mutex<Option<Done>>>,
}

fn is_params(p: &Path) -> bool {
    matches!(p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref(), Some("m3p" | "m3i" | "txt"))
}

fn show(ui: &mut Ui, picture: Option<Bitmap>, text: &str) {
    if !ui.showing(DLG) || ui.f(DLG).id("PreviewImage").is_none() {
        return;
    }
    ui.set_picture(DLG, "PreviewImage", picture);
    ui.set_caption(DLG, "PreviewLabel", text);
}

/// The size and formulas of a scene, the preview's text.
fn describe(sc: &crate::scene::Scene) -> String {
    let names: Vec<String> = sc.formulas.iter().filter(|f| f.iterations > 0).map(|f| f.formula.name().to_string()).collect();
    format!("{} x {}\n{}", sc.width, sc.height, names.join(", "))
}

/// Calculates the preview of `path` (in the calling thread).
fn calculate(path: &Path) -> (Result<(Vec<u8>, usize, usize), String>, String) {
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
        let (r, text) = std::panic::catch_unwind(|| calculate(&path)).unwrap_or_else(|_| (Err("the calculation failed".into()), String::new()));
        *slot.lock().unwrap() = Some((path, r, text));
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

/// The idle work: a file clicked in a preview dialog, a finished preview.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if let Some(p) = ui.file_preview.take() {
        if is_params(&p) {
            app.preview.shown = Some(p.clone());
            match app.preview.cache.get(&p) {
                Some((b, t)) => show(ui, b.clone(), t),
                None => {
                    show(ui, None, "Calculating the preview...");
                    app.preview.want = Some(p);
                }
            }
        } else {
            app.preview.shown = None;
            show(ui, None, "No preview for this file.");
        }
    }
    let done = app.preview.result.lock().unwrap().take();
    if let Some((p, r, text)) = done {
        app.preview.busy = false;
        let entry = match r {
            Ok((rgb, w, h)) => (Some(Bitmap::from_rgb(w, h, &rgb)), text),
            Err(e) => (None, format!("No preview: {e}")),
        };
        if app.preview.shown.as_ref() == Some(&p) {
            show(ui, entry.0.clone(), &entry.1);
        }
        app.preview.cache.insert(p, entry);
    }
    if !app.preview.busy {
        if let Some(p) = app.preview.want.take() {
            if !app.preview.cache.contains_key(&p) {
                start(app, ui, p);
            }
        }
    }
}
