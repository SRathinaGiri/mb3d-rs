//! The mesh export window (bulbtracer2/BulbTracer2UI.pas).

use super::util::{fts, fts_single, parse_float, pti};
use super::Mb3d;
use crate::mesh::{Mesh, MeshParams};
use crate::scene::Scene;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const F: &str = "BulbTracer2Frm";
const PV_BG: u32 = 0xFF40_3020;
const SAVE_EXT: [&str; 4] = ["ply", "obj", "btr2cache", ""];

/// The settings of the window (`TBTracer2Header`).
#[derive(Clone, Debug)]
pub struct Header {
    pub mp: MeshParams,
    pub save_type: i32,
    pub max_preview_vertices: i32,
    pub params_text: String,
    pub with_gl_preview: bool,
    pub preview_de_stop: f64,
    pub preview_size_idx: i32,
    pub auto_preview: bool,
    pub output: String,
    pub use_gl_for_auto: bool,
    pub gl_preview_size_idx: i32,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            mp: MeshParams::default(),
            save_type: 0,
            max_preview_vertices: 5_000_000,
            params_text: String::new(),
            with_gl_preview: false,
            preview_de_stop: 1.5,
            preview_size_idx: 2,
            auto_preview: true,
            output: String::new(),
            use_gl_for_auto: false,
            gl_preview_size_idx: 4,
        }
    }
}

/// `SaveBTracer2Header` ("BTR3").
fn write_btrace2(h: &Header) -> Vec<u8> {
    let mut d = b"BTR3".to_vec();
    let f = |d: &mut Vec<u8>, v: f64| d.extend_from_slice(&v.to_le_bytes());
    let i = |d: &mut Vec<u8>, v: i32| d.extend_from_slice(&v.to_le_bytes());
    let m = &h.mp;
    for v in [m.offset[0], m.offset[1], m.offset[2], m.scale, m.angles[0], m.angles[1], m.angles[2], m.sharpness] {
        f(&mut d, v);
    }
    i(&mut d, m.resolution as i32);
    i(&mut d, m.colors as i32);
    i(&mut d, h.save_type);
    i(&mut d, h.max_preview_vertices);
    i(&mut d, h.params_text.len() as i32);
    d.extend_from_slice(h.params_text.as_bytes());
    i(&mut d, h.with_gl_preview as i32);
    f(&mut d, h.preview_de_stop);
    i(&mut d, h.preview_size_idx);
    i(&mut d, h.auto_preview as i32);
    let u: Vec<u16> = h.output.encode_utf16().collect();
    i(&mut d, u.len() as i32);
    for c in u {
        d.extend_from_slice(&c.to_le_bytes());
    }
    for b in m.bounds {
        f(&mut d, b[0]);
        f(&mut d, b[1]);
    }
    i(&mut d, m.close as i32);
    i(&mut d, h.use_gl_for_auto as i32);
    i(&mut d, h.gl_preview_size_idx);
    d
}

/// `LoadBTracer2Header`
fn read_btrace2(d: &[u8]) -> Result<Header, String> {
    let mut o = 4usize;
    let id = d.get(0..4).ok_or("file too short")?;
    if id != b"BTR1" && id != b"BTR2" && id != b"BTR3" {
        return Err("Missing <BTR1/2/3>-header".into());
    }
    let ver = id[3] - b'0';
    let mut f = || -> Result<f64, String> {
        let v = d.get(o..o + 8).ok_or("file too short")?;
        o += 8;
        Ok(f64::from_le_bytes(v.try_into().unwrap()))
    };
    let mut h = Header::default();
    let mut v8 = [0f64; 8];
    for v in v8.iter_mut() {
        *v = f()?;
    }
    let mut o = 4 + 64;
    let i = |o: &mut usize| -> Result<i32, String> {
        let v = d.get(*o..*o + 4).ok_or("file too short")?;
        *o += 4;
        Ok(i32::from_le_bytes(v.try_into().unwrap()))
    };
    let fd = |o: &mut usize| -> Result<f64, String> {
        let v = d.get(*o..*o + 8).ok_or("file too short")?;
        *o += 8;
        Ok(f64::from_le_bytes(v.try_into().unwrap()))
    };
    h.mp.offset = [v8[0], v8[1], v8[2]];
    h.mp.scale = v8[3];
    h.mp.angles = [v8[4], v8[5], v8[6]];
    h.mp.sharpness = v8[7];
    h.mp.resolution = i(&mut o)?.clamp(4, 4096) as usize;
    h.mp.colors = i(&mut o)? != 0;
    h.save_type = i(&mut o)?;
    h.max_preview_vertices = i(&mut o)?;
    let l = i(&mut o)?.max(0) as usize;
    h.params_text = String::from_utf8_lossy(d.get(o..o + l).ok_or("file too short")?).into_owned();
    o += l;
    h.with_gl_preview = i(&mut o)? != 0;
    h.preview_de_stop = fd(&mut o)?;
    h.preview_size_idx = i(&mut o)?;
    h.auto_preview = i(&mut o)? != 0;
    let l = i(&mut o)?.max(0) as usize;
    let u: Vec<u16> = d.get(o..o + l * 2).ok_or("file too short")?.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    h.output = String::from_utf16_lossy(&u);
    o += l * 2;
    for k in 0..3 {
        h.mp.bounds[k] = [fd(&mut o)?, fd(&mut o)?];
    }
    if ver >= 2 {
        h.mp.close = i(&mut o)? != 0;
    }
    if ver >= 3 {
        h.use_gl_for_auto = i(&mut o)? != 0;
        h.gl_preview_size_idx = i(&mut o)?;
    }
    Ok(h)
}

#[derive(Default)]
struct Shared {
    preview: Option<Bitmap>,
    preview_changed: bool,
    preview_done: bool,
    mesh: Option<Result<Mesh, String>>,
}

pub struct State {
    pub h: Header,
    scene: Option<Scene>,
    user_change: bool,
    first_show: bool,
    enabled: bool,
    shared: Arc<Mutex<Shared>>,
    preview_stop: Arc<AtomicBool>,
    calc_stop: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    previewing: bool,
    restart_preview: bool,
    calculating: bool,
    /// cancel: 0 = show the result, 1 = immediately
    cancel_type: i32,
    t0: Instant,
    pv_dep: usize,
    calc_after_preview: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            h: Header::default(),
            scene: None,
            user_change: true,
            first_show: true,
            enabled: false,
            shared: Arc::new(Mutex::new(Shared::default())),
            preview_stop: Arc::new(AtomicBool::new(false)),
            calc_stop: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            previewing: false,
            restart_preview: false,
            calculating: false,
            cancel_type: 0,
            t0: Instant::now(),
            pv_dep: 32,
            calc_after_preview: false,
        }
    }
}

fn fv(ui: &Ui, c: &str) -> f64 {
    parse_float(&ui.text(F, c)).unwrap_or(0.0)
}

/// `UpdateBTracer2HeaderFromUI`
fn header_from_ui(app: &mut Mb3d, ui: &Ui) {
    let h = &mut app.btracer.h;
    h.mp.offset = [fv(ui, "XOffsetEdit"), fv(ui, "YOffsetEdit"), fv(ui, "ZOffsetEdit")];
    h.mp.scale = fv(ui, "ScaleEdit");
    h.mp.angles = [fv(ui, "XRotateEdit"), fv(ui, "YRotateEdit"), fv(ui, "ZRotateEdit")];
    h.mp.sharpness = fv(ui, "SurfaceSharpnessEdit");
    h.mp.resolution = pti(&ui.text(F, "MeshVResolutionEdit")).clamp(4, 4096) as usize;
    h.mp.colors = ui.checked(F, "CalculateColorsCBx");
    h.save_type = ui.item_index(F, "SaveTypeCmb").max(0);
    h.max_preview_vertices = pti(&ui.text(F, "MaxVerticeCountEdit"));
    h.with_gl_preview = ui.checked(F, "OpenGLPreviewCBx");
    h.preview_de_stop = fv(ui, "PreviewDEAdjust");
    h.preview_size_idx = ui.item_index(F, "VoxelPreviewSizeGrp").max(0);
    h.gl_preview_size_idx = ui.item_index(F, "OpenGLPreviewSizeGrp").max(0);
    h.auto_preview = ui.checked(F, "AutoCalcPreviewCbx");
    h.use_gl_for_auto = ui.checked(F, "OpenGLPreviewCheckbox");
    h.mp.bounds = [
        [fv(ui, "TraceXMinEdit"), fv(ui, "TraceXMaxEdit")],
        [fv(ui, "TraceYMinEdit"), fv(ui, "TraceYMaxEdit")],
        [fv(ui, "TraceZMinEdit"), fv(ui, "TraceZMaxEdit")],
    ];
    h.mp.close = ui.checked(F, "CloseMeshCheckbox");
    h.output = ui.text(F, "FilenameREd");
}

/// `UpdateUIFromBTracer2Header`
fn ui_from_header(app: &mut Mb3d, ui: &mut Ui) {
    let b = app.btracer.user_change;
    app.btracer.user_change = false;
    let h = app.btracer.h.clone();
    let m = &h.mp;
    for (c, v) in [
        ("XOffsetEdit", m.offset[0]),
        ("YOffsetEdit", m.offset[1]),
        ("ZOffsetEdit", m.offset[2]),
        ("ScaleEdit", m.scale),
        ("XRotateEdit", m.angles[0]),
        ("YRotateEdit", m.angles[1]),
        ("ZRotateEdit", m.angles[2]),
        ("SurfaceSharpnessEdit", m.sharpness),
        ("PreviewDEAdjust", h.preview_de_stop),
        ("TraceXMinEdit", m.bounds[0][0]),
        ("TraceXMaxEdit", m.bounds[0][1]),
        ("TraceYMinEdit", m.bounds[1][0]),
        ("TraceYMaxEdit", m.bounds[1][1]),
        ("TraceZMinEdit", m.bounds[2][0]),
        ("TraceZMaxEdit", m.bounds[2][1]),
    ] {
        ui.set_text(F, c, &fts(v));
    }
    ui.set_text(F, "MeshVResolutionEdit", &m.resolution.to_string());
    resolution_label(ui);
    ui.set_checked(F, "CalculateColorsCBx", m.colors);
    ui.set_item_index(F, "SaveTypeCmb", h.save_type);
    ui.set_text(F, "MaxVerticeCountEdit", &h.max_preview_vertices.to_string());
    ui.set_checked(F, "OpenGLPreviewCBx", h.with_gl_preview);
    ui.set_item_index(F, "VoxelPreviewSizeGrp", h.preview_size_idx);
    ui.set_item_index(F, "OpenGLPreviewSizeGrp", h.gl_preview_size_idx);
    ui.set_checked(F, "AutoCalcPreviewCbx", h.auto_preview);
    ui.set_checked(F, "OpenGLPreviewCheckbox", false);
    ui.set_text(F, "FilenameREd", &h.output);
    ui.set_checked(F, "CloseMeshCheckbox", m.close);
    app.btracer.user_change = b;
}

fn resolution_label(ui: &mut Ui) {
    let t = ui.text(F, "MeshVResolutionEdit");
    ui.set_caption(F, "MeshVResolutionLbl", &format!(" x {t} x {t}"));
}

/// `EnableControls`
fn enable_controls(app: &Mb3d, ui: &mut Ui, en: bool) {
    for c in [
        "ImportParamsFromMainBtn",
        "Button2",
        "XOffsetEdit",
        "YOffsetEdit",
        "ZOffsetEdit",
        "ScaleEdit",
        "VoxelPreviewSizeGrp",
        "AutoCalcPreviewCbx",
        "MeshVResolutionEdit",
        "MeshVResolutionUpDown",
        "SurfaceSharpnessEdit",
        "SurfaceSharpnessUpDown",
        "CalculateColorsCBx",
        "SaveTypeCmb",
        "SelectOutputFilenameBtn",
        "FilenameREd",
        "CloseMeshCheckbox",
        "EditModeCmb",
        "TraceXMinEdit",
        "TraceXMaxEdit",
        "TraceYMinEdit",
        "TraceYMaxEdit",
        "TraceZMinEdit",
        "TraceZMaxEdit",
        "PreviewDEAdjust",
        "CloseButton",
        "XRotateEdit",
        "YRotateEdit",
        "ZRotateEdit",
        "ResetOffsetAndScaleBtn",
        "Panel3",
    ] {
        ui.set_enabled(F, c, en);
    }
    for c in ["OpenGLPreviewCBx", "MeshPreviewBtn", "MaxVerticeCountEdit"] {
        ui.set_enabled(F, c, en);
    }
    // the mesh as the auto preview is not available
    ui.set_enabled(F, "OpenGLPreviewCheckbox", false);
    ui.set_enabled(F, "RefreshPreviewBtn", en && app.btracer.enabled);
    if en {
        ui.set_caption(F, "RefreshPreviewBtn", "Calculate preview");
    }
    ui.set_enabled(F, "CancelBtn", app.btracer.calculating);
    ui.set_enabled(F, "CalculateBtn", app.btracer.enabled && !app.btracer.calculating);
}

fn import(app: &mut Mb3d, ui: &mut Ui, title: &str) {
    cancel_preview(app);
    app.make_scene(ui);
    let mut s = app.scene.clone();
    s.tiling = None;
    app.btracer.h.params_text = crate::m3p::raw_to_text(&crate::m3p::write(&s), &app.title);
    app.btracer.scene = Some(s);
    ui.fm(F).set_caption(&format!("Bulb Tracer 2 [ {title} ]"));
    ui_from_header(app, ui);
    app.btracer.enabled = true;
    enable_controls(app, ui, true);
    start_preview(app, ui);
}

fn cancel_preview(app: &mut Mb3d) {
    app.btracer.preview_stop.store(true, Ordering::SeqCst);
    app.btracer.restart_preview = false;
}

/// `XOffsetEditChange`: the preview follows the settings.
fn settings_changed(app: &mut Mb3d, ui: &mut Ui) {
    if !app.btracer.user_change {
        return;
    }
    header_from_ui(app, ui);
    if app.btracer.previewing {
        app.btracer.preview_stop.store(true, Ordering::SeqCst);
        app.btracer.restart_preview = true;
    } else {
        start_preview(app, ui);
    }
}

/// `RefreshPreviewBtnClick` + `StartSlicePreview` + `PaintNextPreviewSlice`.
fn start_preview(app: &mut Mb3d, ui: &mut Ui) {
    if !app.btracer.enabled || app.btracer.calculating {
        return;
    }
    header_from_ui(app, ui);
    let Some(sc) = app.btracer.scene.clone() else { return };
    let h = app.btracer.h.clone();
    let size = 16usize << ui.item_index(F, "VoxelPreviewSizeGrp").clamp(0, 4);
    app.btracer.pv_dep = size;
    // fresh ones for each preview: a stopped one still finishing must not report into this one
    app.btracer.preview_stop = Arc::new(AtomicBool::new(false));
    app.btracer.shared = Arc::new(Mutex::new(Shared::default()));
    app.btracer.progress = Arc::new(AtomicUsize::new(0));
    let stop = app.btracer.preview_stop.clone();
    let shared = app.btracer.shared.clone();
    app.btracer.previewing = true;
    ui.set_caption(F, "RefreshPreviewBtn", "Stop");
    ui.set_picture(F, "Image1", Some(Bitmap::new(322, 322, PV_BG)));
    {
        let c = ui.cm(F, "PreviewProgressBar");
        c.max = 10;
        c.position = 0;
    }
    let progress = app.btracer.progress.clone();
    let waker = ui.waker();
    std::thread::spawn(move || {
        let r = (|| -> Result<(), String> {
            let sampler = crate::mesh::Sampler::new(&sc, &h.mp)?;
            let threads = crate::render::thread_count(&sc);
            let mut bmp = Bitmap::new(322, 322, PV_BG);
            let ik = size / 16;
            let im = match ik {
                1 => 16,
                2 => 8,
                4 => 4,
                8 => 2,
                _ => 1,
            };
            let (pw, ph, pd) = (size, size, size);
            // preview voxel -> trace grid (0..100)
            let g = |v: usize| 50.0 + (v as f64 - size as f64 * 0.5) * 99.0 / (size as f64 - 1.0).max(1.0);
            let de_stop = h.preview_de_stop;
            for nr in (1..=pd).rev() {
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                let mut on = vec![false; pw * ph];
                let rows = Mutex::new(&mut on);
                std::thread::scope(|s| {
                    for t in 0..threads.clamp(1, ph) {
                        let rows = &rows;
                        let sampler = &sampler;
                        let stop = &stop;
                        s.spawn(move || {
                            let mut m = sampler.marcher();
                            let mut buf = vec![false; pw];
                            let mut y = t;
                            while y < ph {
                                if stop.load(Ordering::Relaxed) {
                                    return;
                                }
                                for (x, b) in buf.iter_mut().enumerate() {
                                    *b = sampler.de(&mut m, g(x), g(y), g(nr)) < de_stop;
                                }
                                rows.lock().unwrap()[y * pw..(y + 1) * pw].copy_from_slice(&buf);
                                y += threads.clamp(1, ph);
                            }
                        });
                    }
                });
                if stop.load(Ordering::Relaxed) {
                    return Ok(());
                }
                paint_slice(&mut bmp, &on, size, nr, pw, ph, pd, ik, im);
                progress.store(((10.0 * (size - nr + 1) as f64) / size as f64).round() as usize, Ordering::Relaxed);
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
        sh.preview_done = true;
        if let Err(e) = r {
            sh.mesh = Some(Err(e));
        }
        drop(sh);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

/// `PaintNextPreviewSlice`: slice `nr`, nearer slices brighter and shifted.
#[allow(clippy::too_many_arguments)]
fn paint_slice(bmp: &mut Bitmap, on: &[bool], size: usize, nr: usize, pw: usize, ph: usize, pd: usize, ik: usize, im: usize) {
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
    let off_y = ((size - ph) * im) >> 1;
    let off_x = ((size - pw) * im) >> 1;
    let x0 = off_x + ((nr * im) & !3) / 4;
    let w = bmp.w;
    let mut put = |x: usize, y: usize, col: u32| {
        if x < w && y < bmp.h {
            bmp.px[y * w + x] = col;
        }
    };
    for y in 0..ph {
        let yb = (y * im + 65 + off_y).wrapping_sub(nr / ik);
        for x in 0..pw {
            if !on[y * pw + x] {
                continue;
            }
            let xb = x0 + x * im;
            match im {
                16 | 8 => {
                    // Solid4 on the top row, Solid16 / Solid8 below
                    for k in 0..4 {
                        put(xb + k, yb, if k < 3 { c } else { c4 });
                    }
                    let rows = if im == 16 { 11 } else { 6 };
                    for j in 0..rows {
                        for k in 0..im {
                            put(xb + k, yb + j, if k + 1 < im { c2 } else { c3 });
                        }
                    }
                }
                4 => {
                    for k in 0..4 {
                        let (a, b) = if k == 0 { (c, c4) } else { (c2, c3) };
                        for q in 0..3 {
                            put(xb + q, yb + k, a);
                        }
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
}

/// `CalculateBtnClick` + `StartCalc` + `StartPLYRender`.
fn start_calc(app: &mut Mb3d, ui: &mut Ui) {
    if app.btracer.calculating || app.btracer.previewing {
        return;
    }
    header_from_ui(app, ui);
    let Some(sc) = app.btracer.scene.clone() else { return };
    let mp = app.btracer.h.mp.clone();
    let stop = app.btracer.calc_stop.clone();
    stop.store(false, Ordering::SeqCst);
    let progress = app.btracer.progress.clone();
    progress.store(0, Ordering::SeqCst);
    let shared = app.btracer.shared.clone();
    shared.lock().unwrap().mesh = None;
    app.btracer.calculating = true;
    app.btracer.t0 = Instant::now();
    enable_controls(app, ui, false);
    ui.set_enabled(F, "CancelBtn", true);
    {
        let c = ui.cm(F, "ProgressBar");
        c.max = 1000;
        c.position = 0;
    }
    super::main_form::disable_buttons(app, ui);
    ui.set_caption(F, "Label13", "Tracing object...");
    let name = format!("mesh {}", app.title);
    let waker = ui.waker();
    std::thread::spawn(move || {
        let pr = |d: usize, t: usize| progress.store(d * 1000 / t.max(1), Ordering::Relaxed);
        // (a stopped trace of the same object continues where it was)
        let ck = crate::mesh::MeshCheckpoint::open(&sc, &mp, &name, crate::checkpoint::Policy::Continue);
        let r = crate::mesh::trace_with(&sc, &mp, 0, &pr, &|| stop.load(Ordering::Relaxed), ck.as_ref());
        if let (Ok(_), Some(c)) = (&r, ck) {
            c.finish();
        }
        shared.lock().unwrap().mesh = Some(r);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !app.btracer.previewing && !app.btracer.calculating {
        return;
    }
    let (pv, pv_done, mesh) = {
        let mut sh = app.btracer.shared.lock().unwrap();
        let pv = if sh.preview_changed {
            sh.preview_changed = false;
            sh.preview.clone()
        } else {
            None
        };
        let done = std::mem::take(&mut sh.preview_done);
        (pv, done, sh.mesh.take())
    };
    let p = app.btracer.progress.load(Ordering::Relaxed) as i64;
    if let Some(b) = pv {
        ui.set_picture(F, "Image1", Some(b));
    }
    if app.btracer.previewing {
        ui.cm(F, "PreviewProgressBar").position = p;
        if pv_done {
            app.btracer.previewing = false;
            ui.set_caption(F, "RefreshPreviewBtn", "Calculate preview");
            if let Some(Err(e)) = &mesh {
                app.message(ui, e);
            }
            if std::mem::take(&mut app.btracer.calc_after_preview) {
                app.btracer.restart_preview = false;
                start_calc(app, ui);
            } else if app.btracer.restart_preview {
                app.btracer.restart_preview = false;
                start_preview(app, ui);
            }
        }
        return;
    }
    ui.cm(F, "ProgressBar").position = p;
    let Some(r) = mesh else { return };
    app.btracer.calculating = false;
    super::main_form::enable_buttons(app, ui);
    let secs = app.btracer.t0.elapsed().as_secs_f64().round() as i64;
    let cancelled = app.btracer.calc_stop.load(Ordering::SeqCst);
    match r {
        Ok(mut m) => {
            m.center(1.0);
            let st = app.btracer.h.save_type;
            if !cancelled && (0..=1).contains(&st) {
                let out = PathBuf::from(&app.btracer.h.output);
                match m.save(&out) {
                    Ok(()) => app.message(ui, &format!("Mesh saved: {} ({} vertices, {} faces)", out.display(), m.vertices.len(), m.faces.len())),
                    Err(e) => ui.show_message(&e),
                }
            } else if st == 2 {
                app.message(ui, "The BTracer2 cache format is not supported, the mesh was not saved.");
            }
            ui.set_caption(F, "Label13", &if cancelled { "Operation cancelled".to_string() } else { format!("Elapsed time: {secs} s") });
            if ui.checked(F, "OpenGLPreviewCBx") && (!cancelled || app.btracer.cancel_type == 0) {
                let max = pti(&ui.text(F, "MaxVerticeCountEdit"));
                if max <= 0 || m.vertices.len() <= max as usize {
                    super::meshview::show_mesh(app, ui, &m, "Generated Mesh");
                }
            }
            ui.cm(F, "ProgressBar").position = 1000;
            app.message(ui, "Finished tracing object.");
        }
        Err(e) => {
            ui.set_caption(F, "Label13", if cancelled { "Operation cancelled" } else { "" });
            if !cancelled {
                ui.show_message(&e);
            }
        }
    }
    enable_controls(app, ui, true);
}

fn set_export_ext(ui: &mut Ui) {
    let t = ui.text(F, "FilenameREd");
    if t.trim().is_empty() {
        return;
    }
    let ext = SAVE_EXT[ui.item_index(F, "SaveTypeCmb").clamp(0, 3) as usize];
    let p = Path::new(&t).with_extension(ext);
    ui.set_text(F, "FilenameREd", &p.display().to_string());
}

/// `UpDownBtnValue`: +-step on an edit.
fn step_edit(ui: &mut Ui, c: &str, up: bool, step: f64, clamp: Option<(f64, f64)>) {
    let mut v = fv(ui, c) + if up { step } else { -step };
    if let Some((a, b)) = clamp {
        v = v.clamp(a, b);
    }
    ui.set_text(F, c, &fts(v));
}

/// The step of a move in the preview (`IncXOffsetBtnClick`).
fn move_step(app: &Mb3d) -> f64 {
    let zoom = app.btracer.scene.as_ref().map(|s| s.zoom).unwrap_or(1.0);
    2.2 / (zoom * app.btracer.h.mp.scale * (app.btracer.pv_dep as f64 - 1.0).max(1.0))
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let up = matches!(e.ev, Ev::UpDown { up: true, .. });
    let h = e.handler.as_str();
    match h {
        "FormCreate" => {
            ui.set_item_index(F, "CancelTypeCmb", 0);
            resolution_label(ui);
            enable_controls(app, ui, true);
        }
        "FormShow" => {
            app.btracer.user_change = true;
            if app.btracer.first_show {
                app.btracer.first_show = false;
                ui.set_item_index(F, "SaveTypeCmb", 0);
                let d = app.ini.dir(super::ini::DIR_MESHES);
                ui.set_text(F, "FilenameREd", &d.join("mb3d_mesh.ply").display().to_string());
                header_from_ui(app, ui);
            }
            enable_controls(app, ui, !app.btracer.calculating);
        }
        "CloseButtonClick" => ui.hide(F),
        "ImportParamsFromMainBtnClick" => import(app, ui, "Imported from Main"),
        "Button2Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Parameter (*.m3p)|*.m3p".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3P)),
                ..Default::default()
            };
            ui.open_dialog("btracer:m3p", &o);
        }
        "SelectOutputFilenameBtnClick" => {
            let st = ui.item_index(F, "SaveTypeCmb").clamp(0, 3) as usize;
            let filter = ["Mesh (*.ply)|*.ply", "Mesh (*.obj)|*.obj", "BTracer2 cache (*.btr2cache)|*.btr2cache", "All files|*.*"][st];
            let t = ui.text(F, "FilenameREd");
            let p = Path::new(&t);
            let o = crate::vcl::dialogs::FileOptions {
                filter: filter.into(),
                default_ext: SAVE_EXT[st].into(),
                initial_dir: p.parent().map(Path::to_path_buf),
                file_name: p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                ..Default::default()
            };
            ui.save_dialog("btracer:out", &o);
        }
        "SaveTypeCmbChange" => set_export_ext(ui),
        "XOffsetEditChange" | "TraceXMaxEditExit" | "TraceXMinEditExit" | "TraceYMaxEditExit" | "TraceYMinEditExit"
        | "TraceZMaxEditExit" | "TraceZMinEditExit" | "SurfaceSharpnessEditExit" | "CloseMeshCheckboxClick" => settings_changed(app, ui),
        "XOffsetUpDownClick" | "YOffsetUpDownClick" | "ZOffsetUpDownClick" | "ScaleEditUpDownClick" | "SurfaceSharpnessUpDownClick" => {
            let c = match h {
                "XOffsetUpDownClick" => "XOffsetEdit",
                "YOffsetUpDownClick" => "YOffsetEdit",
                "ZOffsetUpDownClick" => "ZOffsetEdit",
                "ScaleEditUpDownClick" => "ScaleEdit",
                _ => "SurfaceSharpnessEdit",
            };
            step_edit(ui, c, up, 0.05, None);
            settings_changed(app, ui);
        }
        "XRotateUpDownClick" | "YRotateUpDownClick" | "ZRotateUpDownClick" => {
            let c = &format!("{}RotateEdit", &h[..1]);
            step_edit(ui, c, up, 3.0, None);
            settings_changed(app, ui);
        }
        "TraceXMaxUpDownClick" | "TraceXMinEditUpDownClick" | "TraceYMaxUpDownClick" | "TraceYMinEditUpDownClick"
        | "TraceZMaxEditUpDownClick" | "TraceZMinUpDownClick" => {
            let c = format!("Trace{}{}Edit", &h[5..6], if h.contains("Max") { "Max" } else { "Min" });
            step_edit(ui, &c, up, 1.0, Some((0.0, 100.0)));
            settings_changed(app, ui);
        }
        "PreviewDEAdjustUpDownClick" => {
            let v = (fv(ui, "PreviewDEAdjust") + if up { 0.05 } else { -0.05 }).max(0.05);
            ui.set_text(F, "PreviewDEAdjust", &fts_single(v));
            settings_changed(app, ui);
        }
        "MeshVResolutionEditChange" => resolution_label(ui),
        "MeshVResolutionUpDownClick" => {
            let v = (pti(&ui.text(F, "MeshVResolutionEdit")) + if up { 8 } else { -8 }).max(8);
            ui.set_text(F, "MeshVResolutionEdit", &v.to_string());
            resolution_label(ui);
        }
        "ResetOffsetAndScaleBtnClick" => {
            app.btracer.user_change = false;
            for (c, v) in [("XOffsetEdit", "0"), ("YOffsetEdit", "0"), ("ZOffsetEdit", "0"), ("ScaleEdit", "0.5"), ("XRotateEdit", "0"), ("YRotateEdit", "0"), ("ZRotateEdit", "0")] {
                ui.set_text(F, c, v);
            }
            app.btracer.user_change = true;
            settings_changed(app, ui);
        }
        "IncXOffsetBtnClick" | "IncYOffsetBtnClick" | "IncZOffsetBtnClick" => {
            header_from_ui(app, ui);
            let axis = &h[3..4];
            let s = e.sender.as_str();
            let plus = s.starts_with("Inc");
            if ui.item_index(F, "EditModeCmb") <= 0 {
                let mut d = move_step(app);
                let (k, c) = match axis {
                    "X" => (0, "XOffsetEdit"),
                    "Y" => {
                        d = -d;
                        (1, "YOffsetEdit")
                    }
                    _ => (2, "ZOffsetEdit"),
                };
                // IncX/IncZ subtract, DecY subtracts (MB3D's signs)
                let neg = if axis == "Y" { !plus } else { plus };
                if neg {
                    d = -d;
                }
                let v = app.btracer.h.mp.offset[k] + d;
                ui.set_text(F, c, &fts_single(v));
            } else {
                let c = match axis {
                    "X" => "YRotateEdit",
                    "Y" => "XRotateEdit",
                    _ => "ZRotateEdit",
                };
                let neg = if axis == "Y" { !plus } else { plus };
                let d = if neg { -5.0 } else { 5.0 };
                let v = fv(ui, c) + d;
                ui.set_text(F, c, &fts_single(v));
            }
            settings_changed(app, ui);
        }
        "ScaleDownBtnClick" => {
            header_from_ui(app, ui);
            let d = if e.sender == "ScaleUpBtn" { 1.1 } else { 1.0 / 1.1 };
            let v = app.btracer.h.mp.scale * d;
            ui.set_text(F, "ScaleEdit", &fts_single(v));
            settings_changed(app, ui);
        }
        "RefreshPreviewBtnClick" => {
            if app.btracer.previewing {
                cancel_preview(app);
            } else {
                start_preview(app, ui);
            }
        }
        "VoxelPreviewSizeGrpClick" => {
            if ui.checked(F, "AutoCalcPreviewCbx") {
                settings_changed(app, ui);
            }
        }
        "CalculateBtnClick" => {
            let st = ui.item_index(F, "SaveTypeCmb");
            if st == 2 {
                ui.show_message("The BTracer2 cache format is not supported by this program.\nPlease choose a mesh format (*.ply or *.obj).");
                return;
            }
            if st < 2 && ui.text(F, "FilenameREd").trim().is_empty() {
                ui.show_message("Please choose an output file.");
                return;
            }
            cancel_preview(app);
            if app.btracer.previewing {
                // starts when the preview thread has stopped
                app.btracer.calc_after_preview = true;
                return;
            }
            start_calc(app, ui);
        }
        "CancelBtnClick" => {
            if app.btracer.calculating {
                app.btracer.calc_stop.store(true, Ordering::SeqCst);
            }
        }
        "CancelTypeCmbChange" => app.btracer.cancel_type = ui.item_index(F, "CancelTypeCmb"),
        "SaveBTracer2FileBtnClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "BulbTracer2 settings (*.btrace2)|*.btrace2".into(),
                default_ext: "btrace2".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_MESHES)),
                ..Default::default()
            };
            ui.save_dialog("btracer:save", &o);
        }
        "LoadBTracer2FileBtnClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "BulbTracer2 settings (*.btrace2)|*.btrace2".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_MESHES)),
                ..Default::default()
            };
            ui.open_dialog("btracer:load", &o);
        }
        "MeshPreviewBtnClick" => ui.show("MeshPreviewFrm"),
        "OpenGLPreviewCBxClick" => {
            if !ui.checked(F, "OpenGLPreviewCBx") {
                ui.hide("MeshPreviewFrm");
            }
        }
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("m3p", DialogResult::File(Some(p))) => match app.load_params(ui, p) {
            Ok(()) => {
                let n = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                import(app, ui, &n);
            }
            Err(e) => ui.show_message(&e),
        },
        ("out", DialogResult::File(Some(p))) => ui.set_text(F, "FilenameREd", &p.display().to_string()),
        ("save", DialogResult::File(Some(p))) => {
            header_from_ui(app, ui);
            let p = p.with_extension("btrace2");
            if let Err(e) = std::fs::write(&p, write_btrace2(&app.btracer.h)) {
                ui.show_message(&format!("{}: {e}", p.display()));
            }
        }
        ("load", DialogResult::File(Some(p))) => {
            let r = std::fs::read(p).map_err(|e| e.to_string()).and_then(|d| read_btrace2(&d));
            match r {
                Ok(h) => {
                    let mut err = false;
                    if !h.params_text.is_empty() {
                        // the parameters into the main window, then import them
                        match crate::m3p::raw_from_text(&h.params_text).and_then(|(raw, _)| crate::m3p::parse(&raw)) {
                            Ok(m) => {
                                app.scene = m.scene;
                                app.eng.clear();
                                app.scene_to_forms(ui);
                                app.btracer.h = h.clone();
                                ui_from_header(app, ui);
                                import(app, ui, "Imported from Main");
                            }
                            Err(_) => err = true,
                        }
                    }
                    app.btracer.h = h;
                    ui_from_header(app, ui);
                    if err {
                        ui.show_message("Failed to import fractal parameters. All other settings where imported?");
                    }
                }
                Err(e) => ui.show_message(&format!("{}: {e}", p.display())),
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btrace2_roundtrip() {
        let mut h = Header::default();
        h.mp.offset = [0.25, -1.0, 2.0];
        h.mp.close = true;
        h.output = "C:\\meshes\\a.ply".into();
        h.params_text = "Mandelbulb3Dv18{abc}".into();
        let r = read_btrace2(&write_btrace2(&h)).unwrap();
        assert_eq!(r.mp.offset, h.mp.offset);
        assert_eq!(r.output, h.output);
        assert_eq!(r.params_text, h.params_text);
        assert!(r.mp.close);
        assert_eq!(r.gl_preview_size_idx, 4);
    }
}
