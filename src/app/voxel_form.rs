//! The voxel export window (VoxelExport.pas): PNG slices of the object.

use crate::vcl::lang::{tr, trf};
use super::eta::ItemsLeft;
use super::util::{fts, fts_single, left_str, parse_float, pti, smooth_shown, Shown};
use super::{Mb3d, MAIN};
use crate::scene::{InsideMode, Scene};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{DialogResult, Ev, Event, Ui};
use crate::voxel::{ObjectTest, VoxelParams};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const F: &str = "FVoxelExport";
const PV_BG: u32 = 0xFF40_3020;

#[derive(Default)]
struct Shared {
    /// the last finished slice (number, of how many, pixels) of the export
    slice: Option<(u32, u32, Vec<bool>)>,
    /// the preview picture and whether it changed
    preview: Option<Bitmap>,
    preview_changed: bool,
    done: bool,
    error: String,
}

pub struct State {
    /// the parameters of the export (`M3Vfile.VHeader`)
    scene: Option<Scene>,
    vp: VoxelParams,
    project: String,
    user_change: bool,
    first_show: bool,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    /// 1 = slices, 2 = preview
    running: u8,
    /// the preview should be calculated again when the current one is stopped
    restart_preview: bool,
    slice_size: (usize, usize),
    /// the time left: the slices' times, when the last one finished, which
    /// one it was (number, of how many), and the time left as shown
    items: ItemsLeft,
    slice_at: Instant,
    slice_nr: Option<(u32, u32)>,
    left_shown: Option<Shown>,
}

impl Default for State {
    fn default() -> Self {
        State {
            scene: None,
            vp: VoxelParams::from_scene(&Scene::default(), 100),
            project: "new".into(),
            user_change: true,
            first_show: true,
            shared: Arc::new(Mutex::new(Shared::default())),
            stop: Arc::new(AtomicBool::new(false)),
            running: 0,
            restart_preview: false,
            slice_size: (0, 0),
            items: ItemsLeft::default(),
            slice_at: Instant::now(),
            slice_nr: None,
            left_shown: None,
        }
    }
}

/// `MakeM3V`: controls -> settings.
fn make_m3v(app: &mut Mb3d, ui: &Ui) {
    let v = &mut app.voxel.vp;
    let f = |c: &str| parse_float(&ui.text(F, c)).unwrap_or(0.0);
    v.offset = [f("Edit1"), f("Edit3"), f("Edit4")];
    v.scale = [f("Edit5"), f("Edit6"), f("Edit7")].map(|s| if s.abs() < 1e-10 { 1e-10 } else { s });
    v.default_orientation = ui.checked(F, "CheckBox3");
    v.leading_zeros = ui.checked(F, "CheckBox4");
    v.slices = pti(&ui.text(F, "Edit10")).clamp(2, 100_000) as u32;
    v.test = if ui.item_index(F, "RadioGroup1") == 0 { ObjectTest::Iterations } else { ObjectTest::De };
    v.max_its = pti(&ui.text(F, "Edit8"));
    v.de = f("Edit9");
    v.min_its = pti(&ui.text(F, "Edit11"));
    v.min_de = (v.de * 0.254).max(f("Edit12"));
    v.white_outside = ui.checked(F, "CheckBox1");
    v.output_folder = ui.text(F, "Edit2");
    if let Some(s) = app.voxel.scene.as_mut() {
        s.inside = match ui.item_index(F, "RadioGroup4") {
            1 => InsideMode::Inside,
            2 => InsideMode::Both,
            _ => InsideMode::Outside,
        };
    }
}

/// `SetFromM3V`: settings -> controls.
fn set_from_m3v(app: &mut Mb3d, ui: &mut Ui) {
    let b = app.voxel.user_change;
    app.voxel.user_change = false;
    let v = app.voxel.vp.clone();
    ui.set_text(F, "Edit1", &fts(v.offset[0]));
    ui.set_text(F, "Edit5", &fts(v.scale[0]));
    ui.set_text(F, "Edit3", &fts(v.offset[1]));
    ui.set_text(F, "Edit6", &fts(v.scale[1]));
    ui.set_text(F, "Edit4", &fts(v.offset[2]));
    ui.set_text(F, "Edit7", &fts(v.scale[2]));
    ui.set_text(F, "Edit10", &v.slices.to_string());
    ui.set_item_index(F, "RadioGroup1", if v.test == ObjectTest::Iterations { 0 } else { 1 });
    ui.set_text(F, "Edit8", &v.max_its.to_string());
    ui.set_text(F, "Edit9", &fts(v.de));
    ui.set_text(F, "Edit11", &v.min_its.to_string());
    ui.set_text(F, "Edit12", &fts_single(v.min_de));
    let io = match app.voxel.scene.as_ref().map(|s| s.inside) {
        Some(InsideMode::Inside) => 1,
        Some(InsideMode::Both) => 2,
        _ => 0,
    };
    ui.set_item_index(F, "RadioGroup4", io);
    ui.set_text(F, "Edit2", &v.output_folder);
    ui.set_checked(F, "CheckBox1", v.white_outside);
    ui.set_checked(F, "CheckBox3", v.default_orientation);
    ui.set_checked(F, "CheckBox4", v.leading_zeros);
    show_size(app, ui);
    radio_group4(app, ui);
    app.voxel.user_change = b;
}

fn show_size(app: &Mb3d, ui: &mut Ui) {
    let (w, h) = app.voxel.vp.size();
    ui.set_caption(F, "Label12", &format!("{w} x {h}"));
}

fn set_project_name(app: &mut Mb3d, ui: &mut Ui, name: &str) {
    app.voxel.project = name.to_string();
    // ("new": no project file yet, the word shown in the language)
    let shown = if name == "new" { tr("new") } else { name.to_string() };
    ui.set_caption(F, "Label7", &trf("Project: {}", &[&shown]));
}

fn enable(ui: &mut Ui) {
    for c in ["Button2", "Button5", "SpeedButton9"] {
        ui.set_enabled(F, c, true);
    }
}

/// `RadioGroup4Click`: the minimum values of in+out rendering.
fn radio_group4(app: &mut Mb3d, ui: &mut Ui) {
    let b = ui.item_index(F, "RadioGroup4") == 2;
    let b2 = ui.item_index(F, "RadioGroup1") == 0;
    for c in ["Label15", "Edit11", "UpDown10"] {
        ui.set_visible(F, c, b && b2);
    }
    for c in ["Label16", "Edit12", "UpDown11"] {
        ui.set_visible(F, c, b && !b2);
    }
    if b {
        let u = app.voxel.user_change;
        app.voxel.user_change = false;
        ui.set_text(F, "Edit11", &app.voxel.vp.min_its.to_string());
        ui.set_text(F, "Edit12", &fts_single(app.voxel.vp.min_de));
        app.voxel.user_change = u;
    }
}

/// `StartNewPreview`: with "auto update" the preview starts again.
fn start_new_preview(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.checked(F, "CheckBox2") || !ui.enabled(F, "Button5") {
        return;
    }
    if app.voxel.running == 2 {
        app.voxel.stop.store(true, Ordering::SeqCst);
        app.voxel.restart_preview = true;
    } else if app.voxel.running == 0 {
        start_preview(app, ui);
    }
}

/// The scene the slices are calculated with.
fn calc_scene(app: &Mb3d) -> Option<Scene> {
    let mut s = app.voxel.scene.clone()?;
    s.tiling = None;
    s.threads = app.scene.threads;
    Some(s)
}

/// `Button5Click` + `StartSlicePreview` + `PaintNextPreviewSlice`.
fn start_preview(app: &mut Mb3d, ui: &mut Ui) {
    make_m3v(app, ui);
    let Some(sc) = calc_scene(app) else { return };
    let vp = app.voxel.vp.clone();
    let size = 64usize << ui.item_index(F, "RadioGroup2").clamp(0, 2);
    let mx = vp.scale.iter().cloned().fold(1e-40, f64::max);
    let pv = vp.scale.map(|s| ((size as f64 * s / mx).round() as usize).max(1));
    let (pw, ph, pd) = (pv[0].min(size), pv[1].min(size), pv[2].min(size));
    let mut v = vp.clone();
    v.slices = pd.max(2) as u32;
    v.scale = [pw as f64 / pd as f64 * vp.scale[2], ph as f64 / pd as f64 * vp.scale[2], vp.scale[2]];
    let mut s = crate::voxel::export_scene(&sc, &v);
    let (full_w, _) = vp.size();
    s.de_stop = vp.de * pw as f64 / full_w.max(1) as f64;
    v.de = s.de_stop;
    // fresh ones for each run: a stopped run still finishing must not report into this one
    app.voxel.shared = Arc::new(Mutex::new(Shared::default()));
    app.voxel.stop = Arc::new(AtomicBool::new(false));
    let shared = app.voxel.shared.clone();
    let stop = app.voxel.stop.clone();
    app.voxel.running = 2;
    ui.set_caption(F, "Button5", &tr("Stop"));
    ui.set_picture(F, "Image1", Some(Bitmap::new(322, 322, PV_BG)));
    let waker = ui.waker();
    std::thread::spawn(move || {
        let r = (|| -> Result<(), String> {
            let p = crate::calc::CalcParams::new(&s)?;
            let grid = crate::voxel::Grid::new(&s, &v);
            let threads = crate::render::thread_count(&s);
            let mut bmp = Bitmap::new(322, 322, PV_BG);
            let ik = size / 64;
            let im = match ik {
                1 => 4,
                2 => 2,
                _ => 1,
            };
            let off_y = ((size - ph) * im) >> 1;
            let off_x = ((size - pw) * im) >> 1;
            for nr in (1..=pd).rev() {
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                let on = crate::voxel::calc_slice(&p, &grid, &v, nr as u32, threads, &|| stop.load(Ordering::Relaxed));
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                // brighter in front
                let g = |v: i64| {
                    let v = v.clamp(0, 255) as u32;
                    0xFF00_0000 | v << 16 | v << 8 | v
                };
                let fr = nr as f64 / pd as f64;
                let i = ((165.0 - fr * 128.0) * 1.5).round() as i64;
                let mut i2 = (165.0 - fr * 128.0).round() as i64;
                if size > 128 {
                    i2 = (255.0 - fr * 212.0).round() as i64;
                }
                let (c, c4, c2, c3) = (g(i), g(i >> 1), g(i2), g(i2 >> 1));
                let shift = nr / ik;
                let x0 = off_x + ((nr * im) & !3) / 4;
                let mut put = |x: usize, y: usize, col: u32| {
                    if x < 322 && y < 322 {
                        bmp.px[y * 322 + x] = col;
                    }
                };
                for y in 0..ph {
                    let yb = (y * im + 65).wrapping_sub(shift) + off_y;
                    for x in 0..pw {
                        if !on[y * grid.width + x] {
                            continue;
                        }
                        let xb = x0 + x * im;
                        match im {
                            4 => {
                                for k in 0..4 {
                                    let (a, b) = if k == 0 { (c, c4) } else { (c2, c3) };
                                    put(xb, yb + k, a);
                                    put(xb + 1, yb + k, a);
                                    put(xb + 2, yb + k, a);
                                    put(xb + 3, yb + k, b);
                                }
                            }
                            2 => {
                                put(xb, yb, c);
                                put(xb + 1, yb, c);
                                put(xb, yb + 1, c2);
                                put(xb + 1, yb + 1, c3);
                            }
                            _ => put(xb, yb, c2),
                        }
                    }
                }
                let mut sh = shared.lock().unwrap();
                sh.preview = Some(bmp.clone());
                sh.preview_changed = true;
                drop(sh);
                if let Some(w) = &waker {
                    w.wake();
                }
            }
            Ok(())
        })();
        let mut sh = shared.lock().unwrap();
        if let Err(e) = r {
            sh.error = e;
        }
        sh.done = true;
        drop(sh);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

/// `Button2Click` + `StartSlice` + `Timer1Timer`: all slices into the folder.
fn start_slices(app: &mut Mb3d, ui: &mut Ui) {
    make_m3v(app, ui);
    let Some(sc) = calc_scene(app) else { return };
    let vp = app.voxel.vp.clone();
    let dir = PathBuf::from(&vp.output_folder);
    let name = app.voxel.project.clone();
    // fresh ones for each run: a stopped run still finishing must not report into this one
    app.voxel.shared = Arc::new(Mutex::new(Shared::default()));
    app.voxel.stop = Arc::new(AtomicBool::new(false));
    let shared = app.voxel.shared.clone();
    let stop = app.voxel.stop.clone();
    app.voxel.running = 1;
    app.voxel.slice_size = vp.size();
    app.voxel.items = ItemsLeft::default();
    app.voxel.slice_at = Instant::now();
    app.voxel.slice_nr = None;
    app.voxel.left_shown = None;
    super::main_form::disable_buttons(app, ui);
    let waker = ui.waker();
    std::thread::spawn(move || {
        let r = (|| -> Result<(), String> {
            let threads = crate::render::thread_count(&sc);
            // (a stopped export continues with the slices it has not written)
            let slice = |nr: u32, n: u32, on: Option<&[bool]>| {
                if let Some(on) = on {
                    shared.lock().unwrap().slice = Some((nr, n, on.to_vec()));
                    if let Some(w) = &waker {
                        w.wake();
                    }
                }
            };
            match crate::voxel::export_with(&sc, &vp, &dir, &name, threads, &slice, &|| stop.load(Ordering::Relaxed), crate::checkpoint::Policy::Continue) {
                Ok(_) => Ok(()),
                Err(e) if e == "cancelled" => Ok(()),
                Err(e) => Err(e),
            }
        })();
        let mut sh = shared.lock().unwrap();
        if let Err(e) = r {
            sh.error = e;
        }
        sh.done = true;
        drop(sh);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if app.voxel.running == 0 {
        return;
    }
    let (slice, preview, done, err) = {
        let mut sh = app.voxel.shared.lock().unwrap();
        let pv = if sh.preview_changed {
            sh.preview_changed = false;
            sh.preview.clone()
        } else {
            None
        };
        (sh.slice.take(), pv, sh.done, std::mem::take(&mut sh.error))
    };
    if let Some(b) = preview {
        ui.set_picture(F, "Image1", Some(b));
    }
    if let Some((nr, n, on)) = slice {
        let (w, h) = app.voxel.slice_size;
        // the slice in the main window (white = object)
        let rgb: Vec<u8> = on.iter().flat_map(|&b| if b { [255u8; 3] } else { [0u8; 3] }).collect();
        super::main_form::show_external(app, ui, Arc::new(rgb), w, h);
        app.voxel.items.done(app.voxel.slice_at.elapsed().as_secs_f64());
        app.voxel.slice_at = Instant::now();
        app.voxel.slice_nr = Some((nr, n));
    }
    // the slice under way and the time left (the slices' times so far)
    if let (1, Some((nr, n))) = (app.voxel.running, app.voxel.slice_nr) {
        let now = Instant::now();
        let v = &mut app.voxel;
        let left = (nr + 1 < n).then(|| v.items.left(0.0, now.saturating_duration_since(v.slice_at).as_secs_f64(), None, (n - nr - 2) as usize)).flatten();
        v.left_shown = left.map(|l| smooth_shown(v.left_shown, l, now)).or(v.left_shown);
        ui.set_caption(F, "Label13", &slice_caption(nr, n, v.left_shown.map(|s| s.0)));
    }
    if !err.is_empty() {
        app.message(ui, &err);
    }
    if done {
        let was = app.voxel.running;
        app.voxel.running = 0;
        if was == 1 {
            super::main_form::enable_buttons(app, ui);
            ui.set_caption(F, "Label13", "");
            if err.is_empty() && !app.voxel.stop.load(Ordering::SeqCst) {
                app.message(ui, &tr("Finished rendering slices."));
            }
        } else {
            ui.set_caption(F, "Button5", &tr("Calculate preview"));
            if app.voxel.restart_preview {
                app.voxel.restart_preview = false;
                start_preview(app, ui);
            }
        }
    }
}

/// "Rendering slice: 4 of 120 · about 7 min left": the slice under way
/// after `nr` (0-based) finished, of `n`, and the time left.
fn slice_caption(nr: u32, n: u32, left: Option<f64>) -> String {
    let mut t = trf("Rendering slice: {} of {}", &[&(nr + 2).min(n), &n]);
    if let Some(l) = left.filter(|_| nr + 1 < n) {
        t += &format!(" · {}", left_str(l));
    }
    t
}

/// Stops the slice export (the main window's Stop button).
pub fn stop(app: &mut Mb3d) {
    if app.voxel.running == 1 {
        app.voxel.stop.store(true, Ordering::SeqCst);
    }
}

pub fn running(app: &Mb3d) -> bool {
    app.voxel.running == 1
}

fn scale_step(ui: &Ui) -> f64 {
    if ui.item_index(F, "RadioGroup3") == 0 {
        1.1
    } else {
        1.01
    }
}

fn pv_step(app: &Mb3d, ui: &Ui) -> f64 {
    let v = &app.voxel.vp;
    let size = 64usize << ui.item_index(F, "RadioGroup2").clamp(0, 2);
    let mx = v.scale.iter().cloned().fold(1e-40, f64::max);
    let pd = ((size as f64 * v.scale[2] / mx).round()).max(1.0);
    let zoom = app.voxel.scene.as_ref().map(|s| s.zoom).unwrap_or(1.0);
    2.2 / (zoom * v.scale[2] * (pd - 1.0).max(1.0))
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let up = matches!(e.ev, Ev::UpDown { up: true, .. });
    match e.handler.as_str() {
        "FormShow" => {
            app.voxel.user_change = true;
            if app.voxel.first_show {
                app.voxel.first_show = false;
                ui.cm(F, "RadioGroup1").hint = tr(" Type of object determination, lowering Max its or increasing DE\n(dependend on the choosen option), will make the object thicker.\nNote:  DEcombinate formulas work only in the Distance estimation mode!");
                let d = app.ini.dir(super::ini::DIR_VOXEL);
                let mut s = d.display().to_string();
                if !s.ends_with(std::path::MAIN_SEPARATOR) {
                    s.push(std::path::MAIN_SEPARATOR);
                }
                ui.set_text(F, "Edit2", &s);
                app.voxel.vp.output_folder = s;
                set_project_name(app, ui, "new");
            }
        }
        "Button1Click" => ui.hide(F),
        "Button3Click" => {
            let start = ui.text(F, "Edit2");
            ui.folder_dialog("voxel:folder", &tr("Output folder"), Some(PathBuf::from(start)));
        }
        "Button4Click" => {
            // import the parameters of the main window
            app.make_scene(ui);
            let mut s = app.scene.clone();
            s.tiling = None;
            let slices = pti(&ui.text(F, "Edit10")).max(2) as u32;
            let mut v = VoxelParams::from_scene(&s, slices);
            v.leading_zeros = ui.checked(F, "CheckBox4");
            v.white_outside = ui.checked(F, "CheckBox1");
            v.output_folder = ui.text(F, "Edit2");
            app.voxel.vp = v;
            app.voxel.scene = Some(s);
            set_project_name(app, ui, "new");
            set_from_m3v(app, ui);
            enable(ui);
            start_new_preview(app, ui);
        }
        "SpeedButton11Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3v", tr("M3D voxel project (*.m3v)")),
                initial_dir: Some(app.ini.dir(super::ini::DIR_VOXEL)),
                ..Default::default()
            };
            ui.open_dialog("voxel:open", &o);
        }
        "SpeedButton9Click" => {
            make_m3v(app, ui);
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3v", tr("M3D voxel project (*.m3v)")),
                default_ext: "m3v".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_VOXEL)),
                file_name: if app.voxel.project == "new" { String::new() } else { app.voxel.project.clone() },
                ..Default::default()
            };
            ui.save_dialog("voxel:save", &o);
        }
        "Edit1Change" => {
            if app.voxel.user_change {
                make_m3v(app, ui);
                show_size(app, ui);
                start_new_preview(app, ui);
            }
        }
        "Edit10Change" => {
            if app.voxel.user_change {
                make_m3v(app, ui);
                show_size(app, ui);
            }
        }
        "Button2Click" => {
            if app.voxel.running == 2 {
                app.voxel.stop.store(true, Ordering::SeqCst);
                app.voxel.restart_preview = false;
                ui.set_caption(F, "Button5", &tr("Calculate preview"));
            }
            if app.voxel.running != 1 {
                start_slices(app, ui);
            }
        }
        "Button5Click" => {
            if app.voxel.running == 2 {
                app.voxel.stop.store(true, Ordering::SeqCst);
                app.voxel.restart_preview = false;
            } else if app.voxel.running == 0 {
                start_preview(app, ui);
            }
        }
        "UpDown4Click" => {
            // overall scale
            make_m3v(app, ui);
            let d = if up { scale_step(ui) } else { 1.0 / scale_step(ui) };
            app.voxel.vp.scale.iter_mut().for_each(|s| *s *= d);
            let b = app.voxel.user_change;
            app.voxel.user_change = false;
            let sc = app.voxel.vp.scale;
            ui.set_text(F, "Edit5", &fts_single(sc[0]));
            ui.set_text(F, "Edit6", &fts_single(sc[1]));
            ui.set_text(F, "Edit7", &fts_single(sc[2]));
            app.voxel.user_change = b;
            show_size(app, ui);
            if b {
                start_new_preview(app, ui);
            }
        }
        "UpDown5Click" | "UpDown6Click" | "UpDown7Click" => {
            make_m3v(app, ui);
            let mut d = pv_step(app, ui);
            let (k, ed) = match e.handler.as_str() {
                "UpDown5Click" => (0, "Edit1"),
                "UpDown6Click" => {
                    d = -d;
                    (1, "Edit3")
                }
                _ => (2, "Edit4"),
            };
            if up {
                d = -d;
            }
            app.voxel.vp.offset[k] += d;
            ui.set_text(F, ed, &fts_single(app.voxel.vp.offset[k]));
            post_change(app, ui);
        }
        "UpDown8Click" => {
            make_m3v(app, ui);
            app.voxel.vp.max_its += if up { 1 } else { -1 };
            ui.set_text(F, "Edit8", &app.voxel.vp.max_its.to_string());
            post_change(app, ui);
        }
        "UpDown9Click" => {
            make_m3v(app, ui);
            let d = if up { 2f64.sqrt() } else { 1.0 / 2f64.sqrt() };
            let v = &mut app.voxel.vp;
            v.de *= d;
            v.min_de = v.min_de.max(v.de * 0.254);
            let (de, mde) = (v.de, v.min_de);
            app.voxel.user_change = false;
            ui.set_text(F, "Edit12", &fts_single(mde));
            app.voxel.user_change = true;
            ui.set_text(F, "Edit9", &fts_single(de));
            post_change(app, ui);
        }
        "UpDown10Click" => {
            make_m3v(app, ui);
            app.voxel.vp.min_its += if up { 1 } else { -1 };
            ui.set_text(F, "Edit11", &app.voxel.vp.min_its.to_string());
            post_change(app, ui);
        }
        "UpDown11Click" => {
            make_m3v(app, ui);
            let d = if up { 2f64.sqrt() } else { 1.0 / 2f64.sqrt() };
            let v = &mut app.voxel.vp;
            v.min_de = (v.min_de * d).max(v.de * 0.254);
            let m = v.min_de;
            ui.set_text(F, "Edit12", &fts_single(m));
            post_change(app, ui);
        }
        "UpDown1Click" => {
            make_m3v(app, ui);
            let d = if up { scale_step(ui) } else { 1.0 / scale_step(ui) };
            let t = ui.tag(F, &e.sender).clamp(0, 2) as usize;
            app.voxel.vp.scale[t] *= d;
            ui.set_text(F, &format!("Edit{}", 5 + t), &fts_single(app.voxel.vp.scale[t]));
            post_change(app, ui);
        }
        "Button6Click" => {
            app.voxel.user_change = false;
            for (c, v) in [("Edit1", "0"), ("Edit3", "0"), ("Edit4", "0"), ("Edit5", "1"), ("Edit6", "1"), ("Edit7", "1")] {
                ui.set_text(F, c, v);
            }
            app.voxel.user_change = true;
            make_m3v(app, ui);
            show_size(app, ui);
            start_new_preview(app, ui);
        }
        "RadioGroup4Click" => {
            radio_group4(app, ui);
            if app.voxel.user_change {
                make_m3v(app, ui);
                start_new_preview(app, ui);
            }
        }
        _ => {}
    }
}

/// Programmatic edit changes fire no event: what `Edit1Change` does.
fn post_change(app: &mut Mb3d, ui: &mut Ui) {
    if app.voxel.user_change {
        make_m3v(app, ui);
        show_size(app, ui);
        start_new_preview(app, ui);
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("folder", DialogResult::File(Some(p))) => {
            let mut s = p.display().to_string();
            if !s.ends_with(std::path::MAIN_SEPARATOR) {
                s.push(std::path::MAIN_SEPARATOR);
            }
            ui.set_text(F, "Edit2", &s);
            app.voxel.vp.output_folder = s;
        }
        ("open", DialogResult::File(Some(p))) => {
            let r = std::fs::read(p).map_err(|e| e.to_string()).and_then(|d| crate::voxel::read_m3v(&d));
            match r {
                Ok((v, s, notes)) => {
                    app.voxel.vp = v;
                    app.voxel.scene = Some(s);
                    let n = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    set_project_name(app, ui, &n);
                    set_from_m3v(app, ui);
                    for n in notes {
                        app.message(ui, &n);
                    }
                    enable(ui);
                    start_new_preview(app, ui);
                }
                Err(e) => ui.show_message(&format!("{}: {e}", p.display())),
            }
        }
        ("save", DialogResult::File(Some(p))) => {
            let Some(s) = app.voxel.scene.clone() else { return };
            let p = p.with_extension("m3v");
            match std::fs::write(&p, crate::voxel::write_m3v(&app.voxel.vp, &s)) {
                Ok(()) => {
                    let n = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    set_project_name(app, ui, &n);
                }
                Err(e) => ui.show_message(&format!("{}: {e}", p.display())),
            }
        }
        _ => {}
    }
    let _ = MAIN;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_captions() {
        assert_eq!(slice_caption(0, 120, None), "Rendering slice: 2 of 120");
        assert_eq!(slice_caption(2, 120, Some(449.0)), "Rendering slice: 4 of 120 · about 7 min left");
        // the last slice finished: nothing left to say
        assert_eq!(slice_caption(119, 120, Some(3.0)), "Rendering slice: 120 of 120");
    }
}
