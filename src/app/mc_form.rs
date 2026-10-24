//! The Monte Carlo rendering window (MonteCarloForm.pas) and the colour
//! scaling question (ColorOptionForm.pas).

use crate::vcl::lang::{tr, trf};
use super::light_form::short_float_str;
use super::eta::{ItemsLeft, Pace};
use super::util::{d2byte, d2byte_str, fts_single, left_str, parse_float, smooth_shown, time_str, Shown};
use super::{Mb3d, MAIN};
use crate::mc::{McImage, McRecord};
use crate::scene::Scene;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::form::MR_YES;
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const F: &str = "MCForm";
const CO: &str = "FColorOptions";
const BATCH_PNL_WIDTH: i32 = 262;

/// What the render thread hands back.
#[derive(Default)]
struct Shared {
    /// the records after the last finished pass, with a version number
    img: Option<Arc<McImage>>,
    ver: u64,
    /// what a swarm render is at (shown in the window)
    status: String,
    /// the blocks of a swarm render merged so far, with a version number
    live: Option<Arc<McImage>>,
    live_ver: u64,
    /// the rows of the running pass not shown yet; `rows_reset`: a pass
    /// began (the window shows the image at its start under its rows)
    rows: Vec<(usize, Vec<McRecord>)>,
    rows_reset: bool,
    error: String,
    finished: bool,
}

/// An entry of the batch list (`TBatchEntry`).
#[derive(Clone)]
pub struct BatchEntry {
    m3p: PathBuf,
    out: PathBuf,
    elapsed: f64,
    finished: bool,
    image_exists: bool,
}

pub struct State {
    /// the parameters (`MCparas`)
    pub paras: Option<Scene>,
    /// the DoF settings when DoF is switched off in this window
    dof: crate::dof::DofParams,
    /// the records after the last finished pass (while a render runs also
    /// the records its pass started from, shared with it)
    img: Option<Arc<McImage>>,
    name: String,
    save_name: Option<PathBuf>,
    /// what `img` started from besides the parameters, for its checkpoint's
    /// key (see `McCheckpoint::open`): "" for an empty image, else the .m3c
    /// it was loaded from
    ck_start: String,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    /// the running pass's progress: the weight of its rows done, of all
    progress: Arc<AtomicUsize>,
    total: Arc<AtomicUsize>,
    seen_ver: u64,
    seen_live: u64,
    live_painted: Instant,
    /// the picture is not the painting of `img` (rows of the running pass
    /// or the noise filter are on it)
    picture_changed: bool,
    pub running: bool,
    /// the note on the card's shaders compiling was shown for this run
    pub compile_note: bool,
    start: Instant,
    /// the passes finished in this run, when the running one began and how
    /// long the last one took
    passes: u32,
    pass_start: Instant,
    last_pass: Option<f64>,
    /// the time left: the running pass's pace, the passes' times, and the
    /// time left in the pass and in total (a render with a target) as shown
    pace: Pace,
    items: ItemsLeft,
    pass_shown: Option<Shown>,
    total_shown: Option<Shown>,
    /// the rays per pixel (the most) when the run began and now: how many
    /// a pass adds, for the passes a target still needs
    rays0: u32,
    rays: u32,
    last_saved: Instant,
    /// total calculation time in tenths of seconds (`iCalcTime`)
    calc_time: i64,
    user_change: bool,
    bokeh: usize,
    batch: Vec<BatchEntry>,
    work: Vec<usize>,
    in_batch: bool,
    batch_max_rays: u32,
    batch_t0: Instant,
    pending_jpg: Option<PathBuf>,
    /// Start waits for the answer whether to continue the unfinished render
    resume_ask: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            paras: None,
            dof: crate::dof::DofParams::default(),
            img: None,
            name: String::new(),
            save_name: None,
            ck_start: String::new(),
            shared: Arc::new(Mutex::new(Shared::default())),
            stop: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            total: Arc::new(AtomicUsize::new(0)),
            seen_ver: 0,
            seen_live: 0,
            live_painted: Instant::now(),
            picture_changed: true,
            running: false,
            compile_note: false,
            start: Instant::now(),
            passes: 0,
            pass_start: Instant::now(),
            last_pass: None,
            pace: Pace::default(),
            items: ItemsLeft::default(),
            pass_shown: None,
            total_shown: None,
            rays0: 0,
            rays: 0,
            last_saved: Instant::now(),
            calc_time: 0,
            user_change: true,
            bokeh: 0,
            batch: Vec::new(),
            work: Vec::new(),
            in_batch: false,
            batch_max_rays: 4,
            batch_t0: Instant::now(),
            pending_jpg: None,
            resume_ask: false,
        }
    }
}

/// `CalcBokeDiscOnBMP`: the shape of the bokeh `nr` as a 33x33 picture.
fn bokeh_bitmap(nr: usize) -> Bitmap {
    let mut sa = [[0f32; 32]; 32];
    let n = 70i32;
    let sm = 1.0 / n as f32;
    for y in -n..=n {
        for x in -n..=n {
            if x * x + y * y >= n * n {
                continue;
            }
            let r = crate::mc::calc_bokeh(x as f32 * sm, y as f32 * sm, nr as i32) * sm * 14.0;
            let sx = x as f32 * r + 15.0;
            let sy = y as f32 * r + 15.0;
            let (c, cy) = (sx.floor() as usize, sy.floor() as usize);
            if c > 30 || cy > 30 {
                continue;
            }
            let (fx, fy) = (sx - c as f32, sy - cy as f32);
            sa[cy][c] += (1.0 - fx) * (1.0 - fy);
            sa[cy][c + 1] += fx * (1.0 - fy);
            sa[cy + 1][c] += (1.0 - fx) * fy;
            sa[cy + 1][c + 1] += fx * fy;
        }
    }
    let sm = sm * sm * 21000.0;
    let mut b = Bitmap::new(33, 33, 0xFF00_0000);
    for x in 0..31 {
        for y in 0..31 {
            let c = (sa[y][x] * sm).round().clamp(0.0, 255.0) as u32;
            b.px[(x + 1) * 33 + y + 1] = 0xFF00_0000 | c << 16 | c << 8 | c;
        }
    }
    b
}

fn set_bokeh(app: &mut Mb3d, ui: &mut Ui, i: usize) {
    app.mc.bokeh = i.min(5);
    ui.set_caption(F, "Label22", &(app.mc.bokeh + 1).to_string());
    ui.set_picture(F, "Image2", Some(bokeh_bitmap(app.mc.bokeh)));
}

/// `SetParas`: parameters -> controls.
fn set_paras(app: &mut Mb3d, ui: &mut Ui) {
    let Some(p) = app.mc.paras.clone() else { return };
    app.mc.user_change = false;
    let m = &p.mc;
    ui.set_position(F, "TrackBar1", m.contrast as i64);
    ui.set_position(F, "TrackBar4", (m.saturation & 0x7F) as i64);
    ui.set_position(F, "TrackBar3", (p.lighting.gamma.round() as i64).clamp(0, 63));
    ui.set_text(F, "Edit3", &short_float_str(m.soft_shadow_radius));
    ui.set_text(F, "Edit2", &d2byte_str(m.diffuse_reflects));
    ui.set_position(F, "UpDown1", m.reflection_depth as i64);
    ui.set_text(F, "Edit1", &m.reflection_depth.to_string());
    ui.set_position(F, "UpDown3", m.depth as i64 - 1);
    ui.set_text(F, "Edit21", &(m.depth as i64 - 1).to_string());
    ui.set_enabled(F, "Button2", true);
    ui.set_enabled(F, "Button8", true);
    ui.set_enabled(F, "Button4", !app.main.calculating);
    ui.set_checked(F, "CheckBox8", m.options & 1 != 0);
    ui.set_checked(F, "CheckBox2", m.reflections);
    ui.set_checked(F, "CheckBox3", m.transparency);
    ui.set_checked(F, "CheckBox4", p.dof.is_some());
    ui.set_checked(F, "CheckBox6", m.options & 2 != 0);
    ui.set_checked(F, "CheckBox7", m.options & 4 != 0);
    ui.set_item_index(F, "ComboBox1", ((m.options >> 3) & 1) as i32);
    set_bokeh(app, ui, (((m.options >> 4) & 7) as usize).min(5));
    app.mc.user_change = true;
    ui.set_caption(F, "Label14", "");
    ui.set_caption(F, "Label15", "");
    check_box2(ui);
    check_box4(app, ui);
    show_total_time(app, ui);
}

/// `UpdateParas`: controls -> parameters.
fn update_paras(app: &mut Mb3d, ui: &Ui) {
    let bokeh = app.mc.bokeh;
    let dof = app.mc.dof;
    let Some(p) = app.mc.paras.as_mut() else { return };
    let m = &mut p.mc;
    m.reflections = ui.checked(F, "CheckBox2");
    m.transparency = ui.checked(F, "CheckBox3");
    if ui.checked(F, "CheckBox4") {
        if p.dof.is_none() {
            p.dof = Some(dof);
        }
    } else if let Some(d) = p.dof.take() {
        app.mc.dof = d;
    }
    p.slice_2d = 0;
    let d = parse_float(&ui.text(F, "Edit3")).unwrap_or(1.0);
    m.soft_shadow_radius = crate::m3p::short_float_round(d as f32);
    m.depth = (ui.position(F, "UpDown3") + 1).clamp(1, 255) as u8;
    m.reflection_depth = ui.position(F, "UpDown1").clamp(0, 255) as u8;
    m.saturation = ui.position(F, "TrackBar4").clamp(0, 127) as u8;
    m.options = (ui.checked(F, "CheckBox8") as u8)
        | (ui.checked(F, "CheckBox6") as u8) << 1
        | (ui.checked(F, "CheckBox7") as u8) << 2
        | ((ui.item_index(F, "ComboBox1") & 1) as u8) << 3
        | (bokeh as u8) << 4;
    m.contrast = ui.position(F, "TrackBar1").clamp(0, 255) as u8;
    m.diffuse_reflects = d2byte(&ui.text(F, "Edit2"));
    p.lighting.gamma = ui.position(F, "TrackBar3").clamp(0, 63) as f32;
}

/// `CategoryPanel2Collapse`: the category group as high as its panels,
/// Label33 below it.
fn category_group_height(ui: &mut Ui) {
    let f = ui.fm(F);
    f.layout();
    let Some(g) = f.id("CategoryPanelGroup2") else { return };
    let h: i32 = ["CategoryPanel1", "CategoryPanel2", "CategoryPanel5", "CategoryPanel6"].iter().filter_map(|c| f.id(c)).map(|k| f.ctl[k].height).sum();
    // (plus the group's border)
    let h = h + f.ctl[g].height - f.align_rect(g).h;
    f.ctl[g].height = h;
    ui.cm(F, "Label33").top = h + 8;
}

fn check_box2(ui: &mut Ui) {
    let on = ui.checked(F, "CheckBox2");
    for c in ["Edit1", "Edit2", "UpDown1", "Button5", "CheckBox3"] {
        ui.set_enabled(F, c, on);
    }
}

fn check_box4(app: &mut Mb3d, ui: &mut Ui) {
    let on = ui.checked(F, "CheckBox4");
    ui.set_enabled(F, "UpDown2", on);
    ui.set_visible(F, "Image2", on);
    let _ = app;
}

fn show_total_time(app: &Mb3d, ui: &mut Ui) {
    let t = if app.mc.calc_time == 0 { String::new() } else { time_str(app.mc.calc_time) };
    ui.set_caption(F, "Label19", &t);
}

/// `SetFormSize`: the window fits the image.
fn set_form_size(app: &Mb3d, ui: &mut Ui) {
    let Some(p) = &app.mc.paras else { return };
    let side = ui.c(F, "Panel3").width + if ui.visible(F, "BatchPnl") { BATCH_PNL_WIDTH } else { 0 };
    let w = (side + p.width + 4).max(600);
    let h = (ui.c(F, "ToolbarPnl").height + ui.c(F, "BottomPnl").height + p.height + 4).max(562);
    ui.set_client_size(F, w.min(1900), h.min(1150));
}

/// `FitImageSize` + paint: the records into Image1 (`PaintMC`).
fn repaint(app: &mut Mb3d, ui: &mut Ui) {
    let Some(p) = &app.mc.paras else { return };
    let Some(img) = &app.mc.img else { return };
    let rgb = crate::mc::paint(img, &p.mc, p.lighting.gamma);
    set_image(ui, img.width, img.height, &rgb);
    app.mc.picture_changed = false;
}

/// Rows of the running pass into the shown picture (of `w` x `h`).
fn paint_rows(ui: &mut Ui, p: &Scene, w: usize, h: usize, rows: &[(usize, Vec<McRecord>)]) {
    let c = ui.cm(F, "Image1");
    if !c.picture.as_ref().is_some_and(|b| b.w == w && b.h == h) {
        c.width = w as i32;
        c.height = h as i32;
        c.picture = Some(Bitmap::new(w, h, 0xFF00_0000));
    }
    let Some(b) = c.picture.as_mut() else { return };
    for (y, recs) in rows {
        let rgb = crate::mc::paint_recs(recs, &p.mc, p.lighting.gamma);
        for (d, s) in b.px[y * w..y * w + recs.len()].iter_mut().zip(rgb.chunks(3)) {
            *d = 0xFF00_0000 | (s[0] as u32) << 16 | (s[1] as u32) << 8 | s[2] as u32;
        }
    }
}

fn set_image(ui: &mut Ui, w: usize, h: usize, rgb: &[u8]) {
    let mut b = Bitmap::new(w, h, 0xFF00_0000);
    for (d, s) in b.px.iter_mut().zip(rgb.chunks(3)) {
        *d = 0xFF00_0000 | (s[0] as u32) << 16 | (s[1] as u32) << 8 | s[2] as u32;
    }
    let c = ui.cm(F, "Image1");
    c.width = w as i32;
    c.height = h as i32;
    c.picture = Some(b);
}

/// `Button6Click` ("-noise"): a noise filter on the shown picture, the
/// DCT denoising of GIMP's "Denoise DCT" ([`crate::denoise`]) with the
/// strength estimated from the rays ([`crate::mc::noise_sigma`]).  (MB3D
/// has this button hidden, with a 4-neighbour blur behind it.)  Only the
/// picture changes: the next update of a running render shows the records
/// again, and saving saves the shown picture.
fn noise_filter(app: &mut Mb3d, ui: &mut Ui) {
    let (Some(p), Some(img)) = (&app.mc.paras, &app.mc.img) else { return };
    let sigma = crate::mc::noise_sigma(img);
    let rgb = shown_rgb(ui, img, p);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let out = crate::denoise::dct_denoise(&rgb, img.width, img.height, sigma, threads);
    set_image(ui, img.width, img.height, &out);
    app.mc.picture_changed = true;
    app.message(ui, &trf("Noise filter: strength {}", &[&format!("{sigma:.1}")]));
}

/// The shown picture as RGB (MB3D saves `Image1`, so also a picture the
/// noise filter changed), or the painted records when there is none.
fn shown_rgb(ui: &Ui, img: &crate::mc::McImage, p: &crate::scene::Scene) -> Vec<u8> {
    match &ui.c(F, "Image1").picture {
        Some(b) if b.w == img.width && b.h == img.height => b.px.iter().flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8]).collect(),
        _ => crate::mc::paint(img, &p.mc, p.lighting.gamma),
    }
}

fn message_image(ui: &mut Ui, w: usize, h: usize, text: &str) {
    let mut b = Bitmap::new(w.max(1), h.max(1), 0xFFFF_FFFF);
    {
        let font = crate::vcl::font::Font::default();
        let (bw, bh) = (b.w, b.h);
        let mut cv = crate::vcl::canvas::Canvas::new(&mut b.px, bw, bh, 1.0);
        cv.text(8, 8, text, &font, 0xFF00_C000);
    }
    let c = ui.cm(F, "Image1");
    c.width = w as i32;
    c.height = h as i32;
    c.picture = Some(b);
}

/// `CalcAvrgNoise`: the statistics labels.
fn show_stats(app: &mut Mb3d, ui: &mut Ui) -> u32 {
    let Some(img) = &app.mc.img else { return 0 };
    let st = img.stats();
    ui.set_caption(F, "Label2avrgnoise", &fts_single(st.avg_noise));
    ui.set_caption(F, "Label7avrgrays", &fts_single(st.avg_rays));
    ui.set_caption(F, "Label12", &fts_single(st.max_noise));
    ui.set_caption(F, "Label13", &st.max_rays.to_string());
    st.max_rays
}

/// `Button3Click`: the parameters of the main window.
fn import(app: &mut Mb3d, ui: &mut Ui) {
    app.make_scene(ui);
    let mut p = app.scene.clone();
    p.tiling = None;
    p.calc_rect = None;
    if let Some(d) = p.dof.as_mut() {
        d.z_sharp = (d.z_sharp + d.z_sharp2) * 0.5;
        d.z_sharp2 = d.z_sharp;
        app.mc.dof = *d;
    }
    app.mc.calc_time = 0;
    app.mc.img = None;
    app.mc.ck_start.clear();
    app.mc.paras = Some(p);
    proof_light_amount(app, ui, false);
    set_paras(app, ui);
    let mut name = app.title.clone();
    if let Some(i) = name.rfind('.') {
        name.truncate(i);
    }
    if name.is_empty() {
        name = "main params".into();
    }
    app.mc.name = name.clone();
    ui.fm(F).set_caption(&name);
    let (w, h) = (app.scene.width as usize, app.scene.height as usize);
    message_image(ui, w, h, &tr("Press 'Start rendering' to calculate the image"));
    set_form_size(app, ui);
    ui.set_enabled(F, "SpeedButton1", false);
    ui.set_enabled(F, "CheckBox5", false);
    for l in ["Label7avrgrays", "Label2avrgnoise", "Label12", "Label13", "Label19", "Label33"] {
        ui.set_caption(F, l, "");
    }
}

/// Starts the render thread: passes until stopped (`StartCalc` + Timer1).
fn start(app: &mut Mb3d, ui: &mut Ui) {
    start_with(app, ui, None);
}

/// The Unfinished renders window's Continue: the main window's parameters
/// (loaded from the render) imported, and their render continued.
pub fn continue_render(app: &mut Mb3d, ui: &mut Ui) {
    if app.mc.running {
        app.message(ui, &tr("The Monte Carlo window is rendering: stop it first."));
        return;
    }
    ui.show(F);
    import(app, ui);
    start_with(app, ui, Some(crate::checkpoint::Policy::Continue));
}

/// [`start`]; `resume`: what to do with an unfinished render of the
/// parameters (None: ask when the image is empty and one is kept).
fn start_with(app: &mut Mb3d, ui: &mut Ui, resume: Option<crate::checkpoint::Policy>) {
    // (the thread of a running render shares `stop` and `shared`)
    if app.mc.running {
        return;
    }
    update_paras(app, ui);
    let Some(p) = app.mc.paras.clone() else { return };
    let mut img = match app.mc.img.take() {
        Some(i) if i.width == p.width as usize && i.height == p.height as usize => Arc::try_unwrap(i).unwrap_or_else(|i| (*i).clone()),
        _ => McImage::new(p.width as usize, p.height as usize),
    };
    let swarm = super::swarm_form::mc_folder(app).is_some();
    let empty = img.recs.iter().all(|r| r.ray_count == 0);
    let resume = match resume {
        Some(r) => r,
        // (a batch continues what it has without asking)
        None if empty && !swarm && !app.mc.in_batch => match crate::checkpoint::find(&crate::mc::McCheckpoint::key_of(&p, img.seed_salt, "")) {
            Some(e) => {
                app.mc.img = Some(Arc::new(img));
                app.mc.resume_ask = true;
                ui.message_box(
                    "mc:resume",
                    &tr("Unfinished render"),
                    &trf("An unfinished Monte Carlo render of these parameters was kept ({}).\n\nContinue it?  \"No\" renders it again from the start.", &[&e.summary()]),
                    &[crate::vcl::form::MR_YES, crate::vcl::form::MR_NO, crate::vcl::form::MR_CANCEL],
                );
                return;
            }
            None => crate::checkpoint::Policy::Continue,
        },
        None => crate::checkpoint::Policy::Continue,
    };
    // the checkpoint (the swarm's blocks are not kept: the job's units are)
    let ck_start = if empty { "" } else { app.mc.ck_start.as_str() };
    let ck = if swarm { None } else { crate::mc::McCheckpoint::open(&p, img.seed_salt, ck_start, &app.mc.name, resume) };
    if let Some(c) = &ck {
        if c.store().resumed() > 0 && c.restore(&mut img) {
            app.mc.calc_time = (img.seconds * 10.0) as i64;
        } else {
            c.start(&img);
        }
    }
    let first = img.recs.iter().all(|r| r.ray_count == 0);
    // (the pass's own copy of the records it starts from)
    let start = Arc::new(img.clone());
    app.mc.img = Some(start.clone());
    app.mc.picture_changed = true;
    // a continued render: its picture at once, with the rows of the pass
    // it stopped in (they come again with the pass's first rows)
    if let (Some(c), Some(p)) = (&ck, &app.mc.paras) {
        if c.partial_rows() > 0 {
            let shown = c.with_partial(&img);
            set_image(ui, shown.width, shown.height, &crate::mc::paint(&shown, &p.mc, p.lighting.gamma));
            app.mc.picture_changed = false;
        }
    }
    app.mc.stop.store(false, Ordering::SeqCst);
    app.mc.progress.store(0, Ordering::SeqCst);
    app.mc.total.store(0, Ordering::SeqCst);
    *app.mc.shared.lock().unwrap() = Shared::default();
    app.mc.seen_ver = 0;
    app.mc.seen_live = 0;
    app.mc.running = true;
    enable_batch_controls(app, ui);
    app.mc.compile_note = false;
    app.main.led = super::main_form::Led::Cpu;
    super::main_form::show_led(ui, app.main.led, true);
    app.mc.start = Instant::now();
    app.mc.passes = 0;
    app.mc.pass_start = app.mc.start;
    app.mc.last_pass = None;
    app.mc.pace = Pace::default();
    app.mc.items = ItemsLeft::default();
    app.mc.pass_shown = None;
    app.mc.total_shown = None;
    app.mc.rays0 = start.stats().max_rays;
    app.mc.rays = app.mc.rays0;
    ui.set_caption(F, "Button2", &tr("Stop rendering"));
    for c in ["Button3", "Button8", "Button9"] {
        ui.set_enabled(F, c, false);
    }
    ui.set_caption(F, "Label15", "");
    ui.set_caption(F, "Label8", &if first { tr("first 4 rays (noisy)") } else { tr("..til next update") });
    ui.set_visible(F, "Label8", true);
    ui.set_visible(F, "ProgressBar1", true);
    {
        let c = ui.cm(F, "ProgressBar1");
        c.max = p.height as i64;
        c.position = 0;
    }
    let shared = app.mc.shared.clone();
    let stop = app.mc.stop.clone();
    let progress = app.mc.progress.clone();
    let total = app.mc.total.clone();
    let waker = ui.waker();
    let max_rays = if app.mc.in_batch { app.mc.batch_max_rays } else { u32::MAX };
    if let Some(dir) = super::swarm_form::mc_folder(app) {
        // the swarm renders a block of passes on every computer, until
        // Stop (in a batch: until the blocks together have the batch's
        // rays), and averages the blocks
        let units = crate::swarm::suggested_units(&dir);
        let rays = if app.mc.in_batch { (app.mc.batch_max_rays as f64 / units as f64).max(1.0) } else { 0.0 };
        let name = app.swarm.name.clone();
        let cancel = stop.clone();
        let cancel_fn: std::sync::Arc<dyn Fn() -> bool + Send + Sync> = {
            let c = cancel.clone();
            std::sync::Arc::new(move || c.load(Ordering::Relaxed))
        };
        ui.set_caption(F, "Label8", &if rays > 0.0 { trf("swarm: {} blocks of {} rays/pixel", &[&units, &format!("{rays:.0}")]) } else { tr("swarm: a block on every computer, until Stop") });
        std::thread::spawn(move || {
            let report = |st: &str| {
                shared.lock().unwrap().status = st.to_string();
                if let Some(w) = &waker {
                    w.wake();
                }
            };
            let opts = crate::swarm::RunOptions { name, threads: 0, quiet: true, keep: false, cancel: Some(cancel_fn), resume: crate::checkpoint::Policy::Continue };
            // the blocks merged so far, shown as the running pass is
            let live = |img: McImage| {
                let mut s = shared.lock().unwrap();
                s.live = Some(Arc::new(img));
                s.live_ver += 1;
                drop(s);
                if let Some(w) = &waker {
                    w.wake();
                }
            };
            let r = crate::swarm::run_montecarlo_live(&dir, &p, units, rays, 0, 0.0, &opts, &report, Some(&live));
            let mut s = shared.lock().unwrap();
            match r {
                Ok(merged) => {
                    s.img = Some(Arc::new(merged));
                    s.ver += 1;
                }
                Err(e) => {
                    if !cancel.load(Ordering::Relaxed) {
                        s.error = e;
                    }
                }
            }
            s.finished = true;
            drop(s);
            if let Some(w) = &waker {
                w.wake();
            }
        });
        return;
    }
    std::thread::spawn(move || {
        // the records the pass starts from (`img` as it is), the window's
        // image too
        let mut start = start;
        loop {
            let pr = |d: usize, t: usize| {
                total.store(t, Ordering::Relaxed);
                progress.store(d, Ordering::Relaxed);
            };
            let cancel = || stop.load(Ordering::Relaxed);
            {
                let mut s = shared.lock().unwrap();
                s.rows.clear();
                s.rows_reset = true;
            }
            let row = |y: usize, recs: &[McRecord]| {
                shared.lock().unwrap().rows.push((y, recs.to_vec()));
                if let Some(wk) = &waker {
                    wk.wake();
                }
            };
            let r = crate::mc::pass_with(&p, &mut img, 0, &pr, &cancel, Some(&row), ck.as_ref(), Some(&start.recs));
            let mut s = shared.lock().unwrap();
            // the image of the pass to the window (the next one starts from
            // it), and the image before it
            let mut before = None;
            match r {
                Ok(()) => {
                    before = Some(std::mem::replace(&mut start, Arc::new(std::mem::replace(&mut img, McImage::new(0, 0)))));
                    s.img = Some(start.clone());
                    s.ver += 1;
                }
                Err(e) => {
                    if stop.load(Ordering::Relaxed) {
                        // the picture as far as the pass got, for the
                        // window (else it shows the pass's start: black on
                        // a first pass)
                        s.img = Some(Arc::new(img.clone()));
                        s.ver += 1;
                    } else {
                        s.error = e;
                    }
                    s.finished = true;
                }
            }
            let cur: &McImage = if before.is_some() { &start } else { &img };
            let done = s.finished || stop.load(Ordering::Relaxed) || cur.stats().max_rays >= max_rays;
            if done {
                s.finished = true;
            }
            drop(s);
            if let Some(w) = &waker {
                w.wake();
            }
            if done {
                // (a batch entry with its rays: finished, its checkpoint
                // goes; a stopped render keeps it)
                if cur.stats().max_rays >= max_rays {
                    if let Some(c) = ck {
                        c.finish();
                    }
                }
                break;
            }
            // the next pass in the buffer of the image before, once the
            // window let go of it (so that there are not three images at
            // once), else in a new one
            if let Some(b) = before {
                img = match reclaim(b, &stop) {
                    Some(mut i) => {
                        i.clone_from(&start);
                        i
                    }
                    None => (*start).clone(),
                };
            }
            progress.store(0, Ordering::Relaxed);
        }
    });
}

/// The image of `old` once nothing else holds it (the window lets go of it
/// when it takes the next one); None after a while or on Stop.
fn reclaim(mut old: Arc<McImage>, stop: &AtomicBool) -> Option<McImage> {
    let t = Instant::now();
    loop {
        old = match Arc::try_unwrap(old) {
            Ok(i) => return Some(i),
            Err(o) => o,
        };
        if stop.load(Ordering::Relaxed) || t.elapsed().as_secs() >= 10 {
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The program ends: a running render stops (its thread notices the flag).
pub fn stop_for_exit(app: &mut Mb3d) {
    app.mc.stop.store(true, Ordering::SeqCst);
}

/// `StopRendering`: the render thread ends its pass and hands over the
/// picture as far as it got; [`idle`] then finishes.
fn stop(app: &mut Mb3d, ui: &mut Ui) {
    app.mc.stop.store(true, Ordering::SeqCst);
    if app.mc.running {
        ui.set_caption(F, "Button2", &tr("Stopping..."));
        ui.set_enabled(F, "Button2", false);
    }
}

fn finish(app: &mut Mb3d, ui: &mut Ui) {
    if !app.mc.running {
        return;
    }
    app.mc.running = false;
    enable_batch_controls(app, ui);
    ui.set_enabled(F, "Button2", true);
    super::main_form::show_led(ui, app.main.led, false);
    app.mc.calc_time += (app.mc.start.elapsed().as_secs_f64() * 10.0) as i64;
    ui.set_visible(F, "ProgressBar1", false);
    ui.set_visible(F, "Label8", false);
    ui.set_caption(F, "Label14", "");
    ui.set_caption(F, "Label15", "");
    ui.set_caption(F, "Button2", &tr("Start rendering"));
    for c in ["Button3", "Button8", "Button9", "SpeedButton1"] {
        ui.set_enabled(F, c, true);
    }
    show_total_time(app, ui);
    show_stats(app, ui);
    repaint(app, ui);
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !app.mc.running {
        return;
    }
    let el = app.mc.start.elapsed();
    ui.set_caption(F, "Label14", &time_str((el.as_secs_f64() * 10.0) as i64));
    // (the rows weighed by their pixels on the object: the bar's range
    // is their weight)
    let pr = app.mc.progress.load(Ordering::Relaxed) as i64;
    let tot = app.mc.total.load(Ordering::Relaxed) as i64;
    if tot > 0 && ui.c(F, "ProgressBar1").max != tot {
        ui.cm(F, "ProgressBar1").max = tot;
    }
    if ui.c(F, "ProgressBar1").position != pr {
        ui.cm(F, "ProgressBar1").position = pr;
    }
    let (ver, finished, err, img, live, rows, status) = {
        let mut s = app.mc.shared.lock().unwrap();
        let img = if s.ver != app.mc.seen_ver { s.img.take() } else { None };
        // the lines of the running pass (or the swarm's merged blocks), a
        // few times a second
        let due = img.is_none() && app.mc.live_painted.elapsed().as_millis() >= 250;
        let live = if due && s.live_ver != app.mc.seen_live {
            app.mc.seen_live = s.live_ver;
            s.live.clone()
        } else {
            None
        };
        let rows = if due && (s.rows_reset || !s.rows.is_empty()) { Some((std::mem::take(&mut s.rows_reset), std::mem::take(&mut s.rows))) } else { None };
        (s.ver, s.finished, std::mem::take(&mut s.error), img, live, rows, std::mem::take(&mut s.status))
    };
    if !status.is_empty() {
        ui.set_caption(F, "Label15", &status);
    } else if super::swarm_form::mc_folder(app).is_none() {
        // the time left in the pass, and in total when the render has a
        // target (shown smoothed; the rows of the threads end unevenly)
        let f = pr as f64 / tot.max(1) as f64;
        let now = Instant::now();
        let secs = now.saturating_duration_since(app.mc.pass_start).as_secs_f64();
        let m = &mut app.mc;
        m.pace.at(f, secs);
        let target = m.in_batch.then_some(m.batch_max_rays);
        let left = m.pace.left(&m.items, f, secs, 0);
        let total = passes_after(target, m.rays0, m.rays, m.passes).and_then(|n| m.pace.left(&m.items, f, secs, n));
        // (MB3D_ETA_DEBUG=1: the estimator's inputs, to tune it on)
        if std::env::var_os("MB3D_ETA_DEBUG").is_some() {
            eprintln!("eta pass {}: secs {secs:.1} f {f:.4} ({pr}/{tot}) pace {:?} per {:?} left {left:?} total {total:?}", m.passes, m.pace, m.items.per_item());
        }
        m.pass_shown = left.map(|l| smooth_shown(m.pass_shown, l, now)).or(m.pass_shown);
        m.total_shown = total.map(|t| smooth_shown(m.total_shown, t, now)).or(m.total_shown);
        let caption = pass_caption(m.passes, m.pass_shown.map(|s| s.0), m.total_shown.map(|s| s.0), m.last_pass);
        ui.set_caption(F, "Label15", &caption);
    }
    if let (Some(l), Some(p)) = (&live, &app.mc.paras) {
        let rgb = crate::mc::paint(l, &p.mc, p.lighting.gamma);
        set_image(ui, l.width, l.height, &rgb);
        app.mc.live_painted = Instant::now();
    }
    if let Some((reset, rows)) = rows {
        // a new pass: under its rows the image it started from
        if reset && app.mc.picture_changed {
            repaint(app, ui);
        }
        if let (Some(p), Some(img)) = (&app.mc.paras, &app.mc.img) {
            paint_rows(ui, p, img.width, img.height, &rows);
            app.mc.picture_changed = true;
        }
        app.mc.live_painted = Instant::now();
    }
    if let Some(img) = img {
        app.mc.seen_ver = ver;
        if !finished {
            let secs = app.mc.pass_start.elapsed().as_secs_f64();
            app.mc.passes += 1;
            app.mc.last_pass = Some(secs);
            app.mc.items.done(secs);
            app.mc.pace = Pace::default();
            app.mc.pass_shown = None;
            app.mc.pass_start = Instant::now();
        }
        app.mc.img = Some(img);
        // the LED: the marches of the pass on the graphics card or the CPU
        let led = if crate::mc::last_status().contains("on the card") { super::main_form::Led::Gpu } else { super::main_form::Led::Cpu };
        if led != app.main.led {
            app.main.led = led;
            super::main_form::show_led(ui, led, app.mc.running);
        }
        super::main_form::set_led_hint(ui, &trf("Monte Carlo calculated on: {}", &[&crate::mc::last_status()]));
        if !app.mc.compile_note && crate::mc::last_status().contains("still compiling") {
            app.mc.compile_note = true;
            app.message(ui, super::main_form::COMPILE_NOTE);
        }
        repaint(app, ui);
        let max_rays = show_stats(app, ui);
        app.mc.rays = max_rays;
        ui.set_caption(F, "Label8", &tr("..til next update"));
        ui.set_enabled(F, "SpeedButton1", true);
        let total = app.mc.calc_time + (el.as_secs_f64() * 10.0) as i64;
        ui.set_caption(F, "Label19", &time_str(total));
        // auto-saving every 5 minutes
        if ui.enabled(F, "CheckBox5") && ui.checked(F, "CheckBox5") && app.mc.last_saved.elapsed().as_secs() > 300 {
            save_m3c(app, ui);
            app.mc.last_saved = Instant::now();
        }
        if app.mc.in_batch && max_rays >= app.mc.batch_max_rays {
            app.mc.stop.store(true, Ordering::SeqCst);
        }
    }
    if !err.is_empty() {
        app.message(ui, &trf("Monte Carlo: {}", &[&err]));
        ui.show_message(&err);
        app.mc.in_batch = false;
    }
    if finished {
        finish(app, ui);
        if app.mc.in_batch {
            save_batch_image(app, ui);
            ui.cm(F, "BatchProgressBar").position += 1;
            next_batch_entry(app, ui);
        }
    }
}

/// The passes after the running one until the most rays a pixel has reach
/// `target`: from `rays0` when the run began to `rays` now in `passes`
/// passes (None without a target, or before a finished pass told how many
/// rays a pass adds).
fn passes_after(target: Option<u32>, rays0: u32, rays: u32, passes: u32) -> Option<usize> {
    let target = target?;
    if rays >= target {
        return Some(0);
    }
    if passes == 0 || rays <= rays0 {
        return None;
    }
    let per = (rays - rays0) as f64 / passes as f64;
    // (the running pass adds its share too)
    Some((((target - rays) as f64 / per).ceil() as usize).saturating_sub(1))
}

/// "pass 3 · 40 s left · last pass 1:12.4", or with a target "pass 3 ·
/// about 4 min left in total · last pass 1:12.4": the running pass, the
/// time left in it (or in the whole render) and how long the last pass
/// took.
fn pass_caption(passes: u32, left: Option<f64>, total: Option<f64>, last: Option<f64>) -> String {
    let mut t = trf("pass {}", &[&(passes + 1)]);
    match (total, left) {
        (Some(tl), _) => t += &trf(" · {} in total", &[&left_str(tl)]),
        (None, Some(l)) => t += &format!(" · {}", left_str(l)),
        _ => {}
    }
    if let Some(l) = last {
        t += &trf(" · last pass {}", &[&time_str((l * 10.0) as i64)]);
    }
    t
}

fn save_m3c(app: &mut Mb3d, ui: &mut Ui) {
    let (Some(p), Some(img), Some(f)) = (&app.mc.paras, &app.mc.img, &app.mc.save_name) else { return };
    let f = f.with_extension("m3c");
    if let Err(e) = std::fs::write(&f, crate::mc::write_m3c(p, img)) {
        ui.show_message(&format!("{}: {e}", f.display()));
    }
}

fn open_m3c(app: &mut Mb3d, ui: &mut Ui, path: &Path) {
    app.mc.stop.store(true, Ordering::SeqCst);
    let r = std::fs::read(path).map_err(|e| e.to_string()).and_then(|d| crate::mc::read_m3c(&d).map(|v| (v, crate::mc::McCheckpoint::m3c_start(&d))));
    let ((f, img), ck_start) = match r {
        Ok(v) => v,
        Err(e) => return ui.show_message(&format!("{}: {e}", path.display())),
    };
    app.mc.save_name = Some(path.to_path_buf());
    app.mc.ck_start = ck_start;
    ui.set_enabled(F, "CheckBox5", true);
    ui.set_checked(F, "WithZBufferCbx", false);
    let mut info = trf("Parameters: MandId {}", &[&f.mand_id]);
    let notes = f.warnings.clone();
    if !notes.is_empty() {
        info.push('\n');
        info.push_str(&notes.join("\n"));
    }
    ui.set_caption(F, "Label33", &info);
    app.mc.calc_time = (img.seconds * 10.0) as i64;
    app.mc.paras = Some(f.scene);
    if let Some(d) = app.mc.paras.as_ref().and_then(|p| p.dof) {
        app.mc.dof = d;
    }
    app.mc.img = Some(Arc::new(img));
    set_paras(app, ui);
    ui.set_caption(F, "Button2", &tr("Start rendering"));
    repaint(app, ui);
    show_stats(app, ui);
    ui.set_enabled(F, "SpeedButton1", true);
    app.mc.name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let n = app.mc.name.clone();
    ui.fm(F).set_caption(&n);
    set_form_size(app, ui);
}

/// The displayed image (and the z-buffer of a normal calculation).
fn save_picture(app: &mut Mb3d, ui: &mut Ui, path: &Path, jpeg_q: u8) {
    let (Some(p), Some(img)) = (&app.mc.paras, &app.mc.img) else { return };
    let rgb = shown_rgb(ui, img, p);
    let (w, h) = (img.width, img.height);
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let data = match ext.as_str() {
        "jpg" | "jpeg" => crate::jpeg::encode(w, h, &rgb, jpeg_q),
        "bmp" => crate::frames::encode_bmp(w, h, &rgb),
        _ => crate::png::encode_rgb(w, h, &rgb),
    };
    if let Err(e) = std::fs::write(path, data) {
        return ui.show_message(&format!("{}: {e}", path.display()));
    }
    if ui.checked(F, "WithZBufferCbx") {
        let mut s = p.clone();
        s.dof = None;
        s.ao = None;
        s.deao = None;
        s.shadows = None;
        match crate::render::calculate(&s, &|_, _| {}) {
            Ok((_, g)) => {
                let z: Vec<u16> = g.iter().map(|s| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 }).collect();
                let name = format!("ZBuf {}", path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
                let zp = path.with_file_name(name).with_extension("png");
                let _ = std::fs::write(zp, crate::png::encode_gray16(w, h, &z));
            }
            Err(e) => app.message(ui, &trf("Z-buffer: {}", &[&e])),
        }
    }
}

/// `ProofTotalLightAmount`: diffuse + specular colours bigger than 1?
fn proof_light_amount(app: &mut Mb3d, ui: &mut Ui, verbose: bool) {
    let Some(p) = &app.mc.paras else { return };
    if !p.mc.reflections {
        return;
    }
    if p.lighting.diff_map != 0 {
        if !ui.checked(F, "CheckBox7") {
            ui.show_message(&tr("When using a map for the diffuse color,\nyou have to check 'Autoclip Spec+Diff'\nfor an automatic clipping of colors."));
            ui.set_checked(F, "CheckBox7", true);
        }
        return;
    }
    let too_much = (0..10).any(|i| {
        let (d, s, sd, spec) = col_vecs(p, i);
        (0..3).any(|k| d[k] * sd + s[k] * spec > 1.005)
    });
    if too_much {
        ui.confirm("mc:colors", &tr("The amount of added diffuse and specular colors are bigger 1,\nshould i downscale them for a more realistic coloring?"));
    } else if verbose {
        ui.show_message(&tr("All colors are ok."));
    }
}

/// Diffuse and specular colour of palette entry `i` as 0..1 vectors, the
/// diffuse factor of transparency and the specular multiplier.
fn col_vecs(p: &Scene, i: usize) -> ([f32; 3], [f32; 3], f32, f32) {
    let l = &p.lighting;
    let sq = l.internal_gamma2;
    let v = |c: [u8; 3]| c.map(|x| if sq { (x as f32 / 255.0).powi(2) } else { x as f32 / 255.0 });
    let spec = p.mc.reflection_amount.max(0.001);
    let pc = &l.palette[i];
    let alpha = l.palette_alpha.map(|a| a[i]).unwrap_or(*pc.specular.iter().max().unwrap());
    let sd = if p.mc.reflections && p.mc.transparency { (1.0 - alpha as f32 / 255.0 * spec).max(0.001) } else { 1.0 };
    (v(pc.diffuse), v(pc.specular), sd, spec)
}

/// `ModLight`: scales the colours (1 diffuse, 2 both, 3 specular).
fn mod_light(p: &Scene, option: i32) -> crate::lighting::Lighting {
    let mut l = p.lighting.clone();
    let sq = l.internal_gamma2;
    let back = |v: [f32; 3]| v.map(|x| ((if sq { x.max(0.0).sqrt() } else { x }) * 255.0).round().clamp(0.0, 255.0) as u8);
    for i in 0..10 {
        let (mut d, mut s, sd, spec) = col_vecs(p, i);
        scale_pair(&mut d, &mut s, sd, spec, option);
        l.palette[i].diffuse = back(d);
        l.palette[i].specular = back(s);
    }
    for i in 0..4 {
        let v = |c: [u8; 3]| c.map(|x| if sq { (x as f32 / 255.0).powi(2) } else { x as f32 / 255.0 });
        let mut d = v(l.interior[i].1);
        let g = l.interior_spec[i];
        let mut s = v([g, g, g]);
        let spec = p.mc.reflection_amount.max(0.001);
        scale_pair(&mut d, &mut s, 1.0, spec, option);
        l.interior[i].1 = back(d);
        l.interior_spec[i] = back(s)[0];
    }
    l
}

fn scale_pair(d: &mut [f32; 3], s: &mut [f32; 3], sd: f32, spec: f32, option: i32) {
    match option {
        1 => {
            let mut m = 1f32;
            for k in 0..3 {
                if d[k] * sd + s[k] * spec > 1.0 {
                    m = ((1.001 - s[k] * spec) / (d[k] * sd)).clamp(0.0, m);
                }
            }
            if m < 1.0 {
                d.iter_mut().for_each(|x| *x *= m);
            }
        }
        2 => {
            let m = (0..3).map(|k| d[k] * sd + s[k] * spec).fold(0f32, f32::max);
            if m > 1.0 {
                d.iter_mut().for_each(|x| *x /= m);
                s.iter_mut().for_each(|x| *x /= m);
            }
        }
        3 => {
            let mut m = 1f32;
            for k in 0..3 {
                if d[k] * sd + s[k] * spec > 1.0 {
                    m = ((1.001 - d[k] * sd) / (s[k] * spec)).clamp(0.0, m);
                }
            }
            if m < 1.0 {
                s.iter_mut().for_each(|x| *x *= m);
            }
        }
        _ => {}
    }
}

/// `PaintSDPreviewColors`: the palette's diffuse and specular colours.
fn sd_preview(l: &crate::lighting::Lighting, spec: bool) -> Bitmap {
    let (w, h) = (64usize, 13usize);
    let mut b = Bitmap::new(w, h, 0xFF00_0000);
    let mut pal: Vec<_> = l.palette.iter().collect();
    pal.sort_by_key(|p| p.position);
    for x in 1..63 {
        let pos = ((x - 1) as f32 / 61.0 * 65535.0) as u16;
        let mut c = pal[0];
        for p in &pal {
            if p.position <= pos {
                c = p;
            }
        }
        let rgb = if spec { c.specular } else { c.diffuse };
        for y in 0..h {
            b.px[y * w + x] = 0xFF00_0000 | (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
        }
    }
    b
}

fn show_color_options(app: &mut Mb3d, ui: &mut Ui) {
    let Some(p) = app.mc.paras.clone() else { return };
    for o in 1..=3 {
        let l = mod_light(&p, o);
        ui.set_picture(CO, &format!("Image{}", o * 2 - 1), Some(sd_preview(&l, false)));
        ui.set_picture(CO, &format!("Image{}", o * 2), Some(sd_preview(&l, true)));
    }
    ui.show_modal(CO);
}

pub fn color_options_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    if e.handler == "Button1Click" {
        let tag = ui.c(CO, &e.sender).tag as i32;
        ui.hide(CO);
        if (1..=3).contains(&tag) {
            if let Some(p) = app.mc.paras.as_mut() {
                p.lighting = mod_light(p, tag);
            }
            if !app.mc.running {
                repaint(app, ui);
            }
        }
    }
}

// ---- batch rendering (`BatchPnl`)

fn refresh_batch_grid(app: &Mb3d, ui: &mut Ui) {
    let mut rows = vec![vec![tr("Name"), tr("Elapsed"), tr("Status")]];
    for b in &app.mc.batch {
        let name = b.m3p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let (el, st) = if b.finished {
            (
                if b.elapsed > 0.0 { format!("{}s", super::util::fts((b.elapsed * 1000.0).round() / 1000.0)) } else { String::new() },
                if b.image_exists { tr("Image exists") } else { tr("Done") },
            )
        } else {
            (String::new(), String::new())
        };
        rows.push(vec![name, el, st]);
    }
    let c = ui.cm(F, "BatchEntriesGrid");
    c.cols = vec![
        crate::vcl::control::Column { caption: tr("Name"), width: 120, auto: false },
        crate::vcl::control::Column { caption: tr("Elapsed"), width: 50, auto: false },
        crate::vcl::control::Column { caption: tr("Status"), width: 100, auto: false },
    ];
    c.cells = rows;
    if c.row < 1 && !app.mc.batch.is_empty() {
        c.row = 1;
    }
    if c.row > app.mc.batch.len() as i32 {
        c.row = app.mc.batch.len() as i32;
    }
}

fn enable_batch_controls(app: &Mb3d, ui: &mut Ui) {
    let any = !app.mc.batch.is_empty();
    ui.set_enabled(F, "ClearBatchLstBtn", any);
    ui.set_enabled(F, "RemoveBatchLstEntryBtn", any);
    ui.set_enabled(F, "BatchRenderBtn", !app.mc.in_batch && !app.mc.running);
}

fn save_batch_image(app: &mut Mb3d, ui: &mut Ui) {
    if app.mc.work.is_empty() {
        return;
    }
    let i = app.mc.work.remove(0);
    let out = app.mc.batch[i].out.clone();
    save_picture(app, ui, &out, 95);
    let b = &mut app.mc.batch[i];
    b.elapsed = app.mc.batch_t0.elapsed().as_secs_f64();
    b.finished = true;
}

/// `ProcessNextBatchEntry` (with `FetchNextWorkEntry`).
fn next_batch_entry(app: &mut Mb3d, ui: &mut Ui) {
    let network = ui.checked(F, "NetworkRenderCBx");
    let mut next = None;
    while let Some(&i) = app.mc.work.first() {
        if network {
            let out = &app.mc.batch[i].out;
            if out.exists() {
                app.mc.work.remove(0);
                continue;
            }
            // a placeholder so that other computers skip this image
            let _ = std::fs::write(out, b"");
        }
        next = Some(i);
        break;
    }
    match next {
        Some(i) => {
            let p = app.mc.batch[i].m3p.clone();
            if let Err(e) = app.load_params(ui, &p) {
                app.message(ui, &e);
                app.mc.work.remove(0);
                return next_batch_entry(app, ui);
            }
            import(app, ui);
            app.mc.batch_t0 = Instant::now();
            refresh_batch_grid(app, ui);
            start(app, ui);
        }
        None => {
            app.mc.in_batch = false;
            refresh_batch_grid(app, ui);
            enable_batch_controls(app, ui);
        }
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormCreate" => {
            ui.set_visible(F, "BatchPnl", false);
            ui.cm(F, "ComboBox1").hint = tr("Box: sharp, good AA\nGauss: bit blurry, very good AA");
            set_bokeh(app, ui, 0);
            message_image(ui, 250, 30, "");
            refresh_batch_grid(app, ui);
        }
        "FormShow" => enable_batch_controls(app, ui),
        "FormHide" => {
            if app.mc.running {
                ui.confirm("mc:stophide", &tr("Should i stop calculations?"));
            }
        }
        "Button1Click" => ui.hide(F),
        "TrackBar1Change" => {
            if app.mc.user_change {
                update_paras(app, ui);
                repaint(app, ui);
            }
        }
        "TrackBar18KeyPress" => {
            if let Ev::KeyPress('1') = e.ev {
                let s = ui.c(F, &e.sender).sel_start;
                ui.set_position(F, &e.sender, s);
            }
        }
        "Button3Click" => import(app, ui),
        "Button4Click" => {
            // send the parameters to the main window
            update_paras(app, ui);
            if let Some(p) = app.mc.paras.clone() {
                app.scene = p;
                app.eng.clear();
                app.scene_to_forms(ui);
                app.title = app.mc.name.clone();
                super::main_form::set_caption(app, ui);
            }
        }
        "Button5Click" => {
            update_paras(app, ui);
            proof_light_amount(app, ui, true);
        }
        "Button6Click" => noise_filter(app, ui),
        "Button2Click" => {
            if app.mc.running {
                app.mc.in_batch = false;
                stop(app, ui);
            } else {
                app.mc.last_saved = Instant::now();
                start(app, ui);
            }
        }
        "Button8Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3c", tr("M3D monte carlo file (*.m3c)")),
                default_ext: "m3c".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3C)),
                file_name: app.mc.name.clone(),
                ..Default::default()
            };
            ui.save_dialog("mc:save", &o);
        }
        "Button9Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3c", tr("M3D monte carlo file (*.m3c)")),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3C)),
                ..Default::default()
            };
            ui.open_dialog("mc:open", &o);
        }
        "SpeedButton1Click" => {
            let fi = app.ini.get("SaveImageType").parse::<i32>().map(|i| i + 1).unwrap_or(2).clamp(1, 3) as usize;
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.bmp|{}|*.png|{}|*.jpg", tr("Bitmap (*.bmp)"), tr("PNG (*.png)"), tr("JPEG (*.jpg)")),
                filter_index: fi,
                default_ext: ["bmp", "png", "jpg"][fi - 1].into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
                file_name: app.mc.name.clone(),
                ..Default::default()
            };
            ui.save_dialog("mc:pic", &o);
        }
        "CheckBox2Click" => check_box2(ui),
        "CheckBox4Click" => check_box4(app, ui),
        "UpDown2Click" => {
            let up = matches!(e.ev, Ev::UpDown { up: true, .. });
            let i = if up { app.mc.bokeh + 1 } else { app.mc.bokeh.saturating_sub(1) };
            set_bokeh(app, ui, i);
        }
        "CategoryPanel2Expand" => {
            for c in ["CategoryPanel1", "CategoryPanel2", "CategoryPanel5", "CategoryPanel6"] {
                if c != e.sender {
                    ui.cm(F, c).collapsed = true;
                }
            }
            category_group_height(ui);
        }
        "CategoryPanel2Collapse" => category_group_height(ui),
        "ToggleBatchPnlBtnClick" => {
            let v = !ui.visible(F, "BatchPnl");
            ui.set_visible(F, "BatchPnl", v);
            let w = ui.c(F, "MCForm").width + if v { BATCH_PNL_WIDTH } else { -BATCH_PNL_WIDTH };
            let h = ui.c(F, "MCForm").height;
            ui.set_client_size(F, w, h);
        }
        "BatchOpenM3PSequenceBtnClick" => {
            if !app.mc.in_batch {
                let o = crate::vcl::dialogs::FileOptions {
                    filter: format!("{}|*.m3p", tr("M3D Parameter (*.m3p)")),
                    initial_dir: Some(app.ini.dir(super::ini::DIR_M3P)),
                    multi: true,
                    ..Default::default()
                };
                ui.open_dialog("mc:batchopen", &o);
            }
        }
        "ClearBatchLstBtnClick" => ui.confirm("mc:batchclear", &tr("Do you really want to clear the batch list?")),
        "RemoveBatchLstEntryBtnClick" => {
            let r = ui.c(F, "BatchEntriesGrid").row;
            if r > 0 && (r as usize) <= app.mc.batch.len() && !app.mc.in_batch {
                app.mc.batch.remove(r as usize - 1);
                refresh_batch_grid(app, ui);
                enable_batch_controls(app, ui);
            }
        }
        "BatchRenderBtnClick" if !app.mc.in_batch && !app.mc.running => {
            app.mc.batch_max_rays = ui.text(F, "BatchMaxRayCountEdit").trim().parse().unwrap_or(20);
            app.mc.work = (0..app.mc.batch.len()).filter(|&i| !app.mc.batch[i].finished).collect();
            {
                let c = ui.cm(F, "BatchProgressBar");
                c.max = app.mc.work.len() as i64;
                c.position = 0;
            }
            app.mc.in_batch = true;
            enable_batch_controls(app, ui);
            next_batch_entry(app, ui);
        }
        "WithZBufferCbxClick" => {}
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("resume", DialogResult::Button(b)) if app.mc.resume_ask => {
            app.mc.resume_ask = false;
            match *b {
                crate::vcl::form::MR_YES => start_with(app, ui, Some(crate::checkpoint::Policy::Continue)),
                crate::vcl::form::MR_NO => start_with(app, ui, Some(crate::checkpoint::Policy::Fresh)),
                _ => {}
            }
        }
        ("open", DialogResult::File(Some(p))) => open_m3c(app, ui, p),
        ("save", DialogResult::File(Some(p))) => {
            app.mc.save_name = Some(p.with_extension("m3c"));
            app.mc.name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            update_paras(app, ui);
            save_m3c(app, ui);
            let n = app.mc.name.clone();
            ui.fm(F).set_caption(&n);
            ui.set_enabled(F, "CheckBox5", true);
        }
        ("pic", DialogResult::File(Some(p))) => {
            let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if ext == "jpg" || ext == "jpeg" {
                app.mc.pending_jpg = Some(p.clone());
                ui.input_query("mc:jpgq", &tr("JPEG quality or size"), &tr("Type in the quality (0..100) or the maximal output filesize in KB (>100):"), "95");
            } else {
                save_picture(app, ui, p, 95);
            }
        }
        ("jpgq", DialogResult::Text(Some(s))) => {
            if let Some(p) = app.mc.pending_jpg.take() {
                let q = s.trim().parse::<i32>().unwrap_or(95).clamp(1, 100) as u8;
                save_picture(app, ui, &p, q);
            }
        }
        ("colors", DialogResult::Button(MR_YES)) => show_color_options(app, ui),
        ("stophide", DialogResult::Button(MR_YES)) => {
            app.mc.in_batch = false;
            stop(app, ui);
        }
        ("batchclear", DialogResult::Button(MR_YES)) => {
            app.mc.batch.clear();
            refresh_batch_grid(app, ui);
            enable_batch_controls(app, ui);
        }
        ("batchopen", DialogResult::Files(v)) => {
            let mut added = 0;
            for p in v {
                if app.mc.batch.iter().any(|b| &b.m3p == p) {
                    continue;
                }
                let out = p.with_extension("png");
                let exists = std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false);
                app.mc.batch.push(BatchEntry { m3p: p.clone(), out, elapsed: 0.0, finished: exists, image_exists: exists });
                added += 1;
            }
            refresh_batch_grid(app, ui);
            if added == 0 {
                ui.show_message(&tr("No file added"));
            }
            enable_batch_controls(app, ui);
        }
        _ => {}
    }
    let _ = MAIN;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_until_the_target() {
        // no target: nothing to say; the first pass: not yet
        assert_eq!(passes_after(None, 0, 4, 1), None);
        assert_eq!(passes_after(Some(20), 0, 0, 0), None);
        // 4 rays a pass, 20 wanted: after the running one three more
        assert_eq!(passes_after(Some(20), 0, 4, 1), Some(3));
        assert_eq!(passes_after(Some(20), 0, 12, 2), Some(1));
        // the target reached: the running pass is the last
        assert_eq!(passes_after(Some(20), 0, 20, 4), Some(0));
        // a continued render: the passes before do not tell
        assert_eq!(passes_after(Some(20), 8, 8, 0), None);
        assert_eq!(passes_after(Some(20), 8, 14, 1), Some(0));
    }

    #[test]
    fn pass_captions() {
        assert_eq!(pass_caption(0, None, None, None), "pass 1");
        assert_eq!(pass_caption(2, Some(40.0), None, Some(72.4)), "pass 3 · 40 s left · last pass 1:12.4");
        assert_eq!(pass_caption(2, Some(40.0), Some(250.0), Some(72.4)), "pass 3 · about 4 min left in total · last pass 1:12.4");
    }
}
