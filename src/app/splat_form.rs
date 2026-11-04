//! The "Gaussian splats" window (`SplatFrm`, this program's own, opened from
//! the Bulb Tracer window's "Gaussian splats..." button): the main window's
//! scene exported as Gaussian splats (`crate::splat`), calculated in the
//! background with a progress bar and a Stop button.

use super::Mb3d;
use crate::splat::{Around, SplatParams};
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Event, Ui};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const F: &str = "SplatFrm";

/// What a finished export reports: the splats written and the file.
type Outcome = Result<(usize, PathBuf), String>;

pub struct State {
    running: bool,
    first_show: bool,
    stop: Arc<AtomicBool>,
    /// the view being calculated and the number of views
    view: Arc<AtomicUsize>,
    views: usize,
    result: Arc<Mutex<Option<Outcome>>>,
    t0: Instant,
}

impl Default for State {
    fn default() -> Self {
        State {
            running: false,
            first_show: true,
            stop: Arc::new(AtomicBool::new(false)),
            view: Arc::new(AtomicUsize::new(0)),
            views: 16,
            result: Arc::new(Mutex::new(None)),
            t0: Instant::now(),
        }
    }
}

/// Opens the window.
pub fn open(_app: &mut Mb3d, ui: &mut Ui) {
    ui.show(F);
}

/// The window's settings (the views and size within the command line's
/// limits).
fn params(ui: &Ui) -> SplatParams {
    let num = |c: &str, d: f64, lo: f64, hi: f64| ui.text(F, c).trim().parse::<f64>().ok().filter(|v| v.is_finite()).unwrap_or(d).clamp(lo, hi);
    SplatParams {
        views: num("ViewsEdit", 16.0, 1.0, 1000.0) as usize,
        size: num("SizeEdit", 640.0, 16.0, 8192.0) as i32,
        degree: ui.item_index(F, "ShCmb").clamp(0, 3) as u8,
        around: match ui.item_index(F, "AroundCmb") {
            1 => Around::Middle,
            2 => Around::Camera,
            _ => Around::Auto,
        },
        ..SplatParams::default()
    }
}

fn enable(ui: &mut Ui, running: bool) {
    for c in ["AroundCmb", "ViewsEdit", "SizeEdit", "ShCmb", "FileEdit", "FileBtn", "CalcBtn"] {
        ui.set_enabled(F, c, !running);
    }
    ui.set_enabled(F, "StopBtn", running);
}

fn start(app: &mut Mb3d, ui: &mut Ui) {
    if app.splat.running {
        return;
    }
    if app.main.calculating {
        ui.show_message(&tr("@MB:ACalculationIsRunningStopIt"));
        return;
    }
    let out = ui.text(F, "FileEdit").trim().to_string();
    if out.is_empty() {
        ui.show_message(&tr("@MB:PleaseChooseAnOutputFile"));
        return;
    }
    let mut out = PathBuf::from(out);
    if !out.extension().is_some_and(|e| e.eq_ignore_ascii_case("ply")) {
        out.set_extension("ply");
        ui.set_text(F, "FileEdit", &out.display().to_string());
    }
    // the main window's scene, as Calculate 3D would take it
    app.make_scene(ui);
    let mut sc = app.scene.clone();
    sc.tiling = None;
    sc.calc_rect = None;
    let sp = params(ui);
    let st = &mut app.splat;
    st.running = true;
    st.views = sp.views;
    st.t0 = Instant::now();
    st.stop = Arc::new(AtomicBool::new(false));
    st.view = Arc::new(AtomicUsize::new(0));
    st.result = Arc::new(Mutex::new(None));
    let (stop, view, result) = (st.stop.clone(), st.view.clone(), st.result.clone());
    enable(ui, true);
    ui.cm(F, "ProgressBar").position = 0;
    ui.set_caption(F, "StatusLabel", &trf("@MB:DirectionOf", &[&1, &sp.views]));
    let waker = ui.waker();
    std::thread::spawn(move || {
        let progress = |v: usize, _n: usize| {
            view.store(v, Ordering::Relaxed);
            if let Some(w) = &waker {
                w.wake();
            }
        };
        let r = crate::splat::export(&sc, &sp, &progress, &|| stop.load(Ordering::Relaxed)).and_then(|r| {
            if r.splats.is_empty() {
                return Err(tr("@MB:TheFractalIsInNoneOf"));
            }
            crate::splat::write_ply(&out, &r, sp.degree)?;
            Ok((r.splats.len(), out))
        });
        *result.lock().unwrap() = Some(r);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !app.splat.running {
        return;
    }
    let v = app.splat.view.load(Ordering::Relaxed);
    let n = app.splat.views.max(1);
    ui.cm(F, "ProgressBar").position = (v.min(n) * 1000 / n) as i64;
    let Some(r) = app.splat.result.lock().unwrap().take() else {
        ui.set_caption(F, "StatusLabel", &trf("@MB:DirectionOf", &[&(v + 1).min(n), &n]));
        return;
    };
    app.splat.running = false;
    enable(ui, false);
    let stopped = app.splat.stop.load(Ordering::SeqCst);
    match r {
        Ok((count, path)) => {
            let secs = app.splat.t0.elapsed().as_secs_f64();
            let time = if secs < 60.0 { format!("{secs:.0} s") } else { format!("{}:{:02} min", secs as u64 / 60, secs as u64 % 60) };
            let msg = trf("@MB:SplatsInWrittenTo", &[&count, &time, &path.display()]);
            ui.cm(F, "ProgressBar").position = 1000;
            ui.set_caption(F, "StatusLabel", &msg);
            app.message(ui, &msg);
        }
        Err(e) => {
            ui.cm(F, "ProgressBar").position = 0;
            if stopped {
                ui.set_caption(F, "StatusLabel", &tr("@MB:OperationCancelled"));
            } else {
                ui.set_caption(F, "StatusLabel", "");
                ui.show_message(&e);
            }
        }
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            if app.splat.first_show {
                app.splat.first_show = false;
                ui.set_item_index(F, "AroundCmb", 0);
                ui.set_item_index(F, "ShCmb", 2);
                let d = app.ini.dir(super::ini::DIR_MESHES);
                ui.set_text(F, "FileEdit", &d.join("mb3d_splats.ply").display().to_string());
            }
            enable(ui, app.splat.running);
        }
        "FileBtnClick" => {
            let t = PathBuf::from(ui.text(F, "FileEdit").trim());
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.ply", tr("@MB:GaussianSplatsPLY")),
                initial_dir: t.parent().map(|p| p.to_path_buf()).filter(|p| p.is_dir()).or_else(|| Some(app.ini.dir(super::ini::DIR_MESHES))),
                file_name: t.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                default_ext: "ply".into(),
                ..Default::default()
            };
            ui.save_dialog("splat:out", &o);
        }
        "CalcBtnClick" => start(app, ui),
        "StopBtnClick" => app.splat.stop.store(true, Ordering::SeqCst),
        "CloseBtnClick" => ui.hide(F),
        _ => {}
    }
}

pub fn dialog(_app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("out", DialogResult::File(Some(p))) = (what, r) {
        ui.set_text(F, "FileEdit", &p.display().to_string());
    }
}
