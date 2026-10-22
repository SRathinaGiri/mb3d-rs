//! The main window (Mand.pas, `TMand3DForm`).

use super::engine::Job;
use super::util::{fts, fts_single, parse_float, pti, ptf, time_str};
use super::{Mb3d, MAIN};
use crate::math::{build_rot_matrix, mat_mul, matrix_to_angles, normalise_matrix_to, rotate_vector_reverse};
use crate::scene::{CameraOptic, Scene};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::form::{MouseButton, MR_CANCEL, MR_NO, MR_YES, SS_LEFT};
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::path::PathBuf;
use std::time::Instant;

const F: &str = MAIN;
const PID180: f64 = std::f64::consts::PI / 180.0;

/// `TqualPreset`: the quality presets preview / video / mid / high.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub smooth_normals: i64,
    pub de_stop: f64,
    pub ray_multiplier: f64,
    pub bin_search: i64,
    pub image_width: i32,
    pub image_scale: i32,
    pub ray_limiter: f64,
}

pub const ACC_PRESETS: [Preset; 4] = [
    Preset { smooth_normals: 0, de_stop: 1.0, ray_multiplier: 0.5, bin_search: 6, image_width: 480, image_scale: 1, ray_limiter: 1.0 },
    Preset { smooth_normals: 1, de_stop: 1.0, ray_multiplier: 0.4, bin_search: 8, image_width: 640, image_scale: 1, ray_limiter: 0.75 },
    Preset { smooth_normals: 2, de_stop: 1.2, ray_multiplier: 0.3, bin_search: 10, image_width: 1600, image_scale: 2, ray_limiter: 0.5 },
    Preset { smooth_normals: 3, de_stop: 1.2, ray_multiplier: 0.25, bin_search: 12, image_width: 3072, image_scale: 3, ray_limiter: 0.3 },
];
const PRESET_FILES: [&str; 4] = ["preset_preview.txt", "preset_video.txt", "preset_mid.txt", "preset_high.txt"];

pub struct State {
    pub image_scale: i32,
    pub presets: [Preset; 4],
    pub slice_calc: u8,
    pub calculating: bool,
    pub calc3d: bool,
    pub img_ver: u64,
    pub finished_seen: u64,
    pub last_partial: Instant,
    pub calc_start: Instant,
    /// the stage the window shows and when it began, for the time left
    pub stage_shown: String,
    pub stage_start: Instant,
    /// the stages after the main rendering this calculation has, the time
    /// a whole main rendering takes (once known) and the time left in
    /// total as shown, with when (for the smoothing)
    later: Vec<Later>,
    passed: Vec<Later>,
    stage_base: f64,
    main_full: Option<f64>,
    total_shown: Option<(f64, Instant)>,
    /// how long each later stage takes relative to the main rendering (the
    /// model's, from [`super::eta::later_ratios`]), and the whole
    /// calculation's progress as the bar shows it (it only grows)
    later_ratio: [f64; 3],
    progress_shown: f64,
    /// the card's shaders compiled during the main rendering, and it ended
    /// before its pace without compiling was known (the CPU did it alone:
    /// the model's `CPU_ALONE` ratios then)
    main_cold: bool,
    main_unmeasured: bool,
    /// (for `MB3D_ETA_LOG`) the main rendering's own time and its progress
    /// when the shaders had compiled
    main_secs: f64,
    f_compiled: Option<f64>,
    /// the later stages' times in this calculation, and in the last one
    /// finished with the picture it had (`last_sig`): a picture calculated
    /// again expects them (the main rendering's own time varies more, e.g.
    /// with the card's shaders compiled); not those that ran while shaders
    /// compiled (`later_cold`: the same picture again finds them compiled)
    later_secs: [Option<f64>; 3],
    later_cold: [bool; 3],
    last_secs: [Option<f64>; 3],
    calc_sig: String,
    last_sig: String,
    /// the shaders' compile time when the stage began (see
    /// [`compile_secs`]), and whether the main rendering compiled hardly
    /// any (its time is then a measure for the later stages' ratios)
    stage_compile: f64,
    /// the running stage's progress over its time, (progress, seconds); the
    /// stages' of this calculation and of the last one finished (`last_sig`)
    /// by name: the same picture again progresses alike (its hard rows
    /// where they were)
    samples: Vec<(f64, f64)>,
    /// the calculation's features for the later stages' times (see
    /// [`super::eta`]) and the shaders' compile time when it began
    eta_f: [f32; super::eta::FEATURES],
    calc_compile: f64,
    curves: Vec<(String, Vec<(f64, f64)>)>,
    last_curves: Vec<(String, Vec<(f64, f64)>)>,
    /// the title bar LED: its state, blinking during a 3D calculation
    pub led: Led,
    pub led_running: bool,
    /// the note on the card's shaders compiling was shown for this calculation
    pub compile_note: bool,
    pub user_aspect: (i32, i32),
    pub intern_aspect: f64,
    pub user_change: bool,
    /// MB3D's iGetPosFromImage: 1/10 DOF z, 2 light position, 3 julia,
    /// 4 stereo, 6 cutting, 22 midpoint
    pub get_pos: i32,
    /// the mouse button went down for a "get ... from image" pick: its
    /// release must not move or zoom the view (issue #6)
    pub pos_picked: bool,
    pub mouse_start: (i32, i32),
    pub max_shape_w: i32,
    pub z_translate: i32,
    pub undo: Vec<Scene>,
    pub undo_pos: usize,
    pub authors: [String; 2],
    pub auto_m3i: Option<PathBuf>,
    /// Destination selected before a stereo-pair render starts.  Saving at
    /// completion means a long unattended render is not lost afterwards.
    pub auto_stereo: Option<PathBuf>,
    pub waker_set: bool,
    pub image_text: bool,
    /// full size image (for saving)
    pub rgb: Option<(std::sync::Arc<Vec<u8>>, usize, usize)>,
    pub last_history: Instant,
    /// `FormsSticky`: the Formulas, Lighting and Post processing windows
    /// beside the main window: 0 free, 1 right, 2 left side
    pub sticky: [u8; 3],
    /// the window whose right click menu is open (`StickIt`)
    pub stick_it: usize,
    /// the calculation that waits for the answer whether to continue its
    /// unfinished one: its scene and 2D slice
    pub resume_ask: Option<(Scene, u8)>,
    /// the next Calculate 3D continues the unfinished calculation of this
    /// key without asking (the Unfinished renders window's Continue)
    pub resume_key: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        State {
            image_scale: 1,
            presets: ACC_PRESETS,
            slice_calc: 2,
            calculating: false,
            calc3d: false,
            img_ver: 0,
            finished_seen: 0,
            last_partial: Instant::now(),
            calc_start: Instant::now(),
            stage_shown: String::new(),
            stage_start: Instant::now(),
            later: Vec::new(),
            passed: Vec::new(),
            stage_base: 0.0,
            main_full: None,
            total_shown: None,
            later_ratio: [0.0; 3],
            progress_shown: 0.0,
            main_cold: false,
            main_unmeasured: false,
            main_secs: 0.0,
            f_compiled: None,
            later_secs: [None; 3],
            later_cold: [false; 3],
            last_secs: [None; 3],
            calc_sig: String::new(),
            last_sig: String::new(),
            stage_compile: 0.0,
            samples: Vec::new(),
            eta_f: [0.0; super::eta::FEATURES],
            calc_compile: 0.0,
            curves: Vec::new(),
            last_curves: Vec::new(),
            led: Led::Idle,
            led_running: false,
            compile_note: false,
            user_aspect: (0, 0),
            intern_aspect: 4.0 / 3.0,
            user_change: true,
            get_pos: 0,
            pos_picked: false,
            mouse_start: (0, 0),
            max_shape_w: 0,
            z_translate: 0,
            undo: Vec::new(),
            undo_pos: 0,
            authors: [String::new(), String::new()],
            auto_m3i: None,
            auto_stereo: None,
            waker_set: false,
            image_text: false,
            rgb: None,
            last_history: Instant::now(),
            sticky: [1, 1, 1],
            stick_it: 0,
            resume_ask: None,
            resume_key: None,
        }
    }
}

/// The program's version as it is shown (title bar, main window).
pub fn version_str() -> String {
    format!("mb3d-rs {}", env!("CARGO_PKG_VERSION"))
}

fn t(ui: &Ui, c: &str) -> String {
    ui.text(F, c)
}

fn set(ui: &mut Ui, c: &str, s: &str) {
    ui.set_text(F, c, s);
}

/// The main window's caption: the parameter name, as MB3D shows it; the
/// program's version stays at the right of the title bar (also while a
/// calculation shows its state in the caption), so a screenshot tells it.
pub fn set_caption(app: &Mb3d, ui: &mut Ui) {
    let cap = if app.title.is_empty() { "Mandelbulb 3D".to_string() } else { app.title.clone() };
    let f = ui.fm(F);
    f.set_caption(&cap);
    f.caption_right = version_str();
}

// ---------------------------------------------------------------------------
// start-up
// ---------------------------------------------------------------------------

/// FormCreate, SetM3Dini, FirstShowUpdate and LoadStartupParas.
pub fn start(app: &mut Mb3d, ui: &mut Ui) {
    app.message(ui, "Welcome to Mandelbulb 3D!");
    app.message(ui, "Visit the official web site for news and updates: https://mb3d.overwhale.com");
    app.message(ui, "");
    ui.set_caption(F, "Label3", &version_str());
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(64) as i64;
    ui.set_position(F, "UpDown3", cpus);
    set(ui, "Edit21", &cpus.to_string());
    ui.set_active_page(F, "PageControl2", "TabSheet7");
    init_compute(ui);
    // SetM3Dini
    set(ui, "Edit4", app.ini.get("MandRotDeg"));
    if let Some((a, b)) = app.ini.get("UserAspect").split_once(':') {
        let (x, y) = (pti(a), pti(b));
        if y > 0 && x > 0 {
            app.main.user_aspect = (x, y);
        }
    }
    update_aspect_caption(app, ui);
    ui.set_checked(F, "CheckBox13", app.ini.get("SavePNGtextPars") != "No");
    ui.set_checked(F, "CheckBox14", app.ini.get("DisableTBoost") != "No");
    ui.set_checked(F, "CheckBox15", app.ini.get("ThreadPriority") != "-1");
    ui.set_checked(F, "CheckBox16", app.ini.get("SaveImgInM3I") == "Yes");
    if ui.checked(F, "CheckBox15") {
        let i = app.ini.get("ThreadPriority").parse().unwrap_or(2);
        ui.set_item_index(F, "ComboBox2", i);
    }
    // no thread priority (MB3D's "Thread Priority", "Disable threadboost"):
    // the calculation threads run at normal priority
    for c in ["Label33", "ComboBox2", "CheckBox14", "CheckBox15"] {
        ui.set_visible(F, c, false);
    }
    if let Ok(i) = app.ini.get("ImageSharp").parse::<i32>() {
        ui.set_caption(F, "Label23", &i.to_string());
    }
    if app.ini.get("ScaleDEstop") == "0" {
        ui.set_checked(F, "CheckBox11", false);
    }
    let calc_on = match app.ini.get_extra("CalcOn") {
        Some("CPU") => 1,
        Some("GPU") => 2,
        _ => 0,
    };
    set_calc_on(app, ui, calc_on);
    if app.ini.get("ThreadCount") != "Auto" {
        ui.set_checked(F, "CheckBox12", false);
        let n = app.ini.get("ThreadCount").parse::<i64>().unwrap_or(cpus).clamp(1, 64);
        ui.set_position(F, "UpDown3", n);
        set(ui, "Edit21", &n.to_string());
    }
    app.main.authors[0] = app.ini.get("Author").to_string();
    // StickOption: Lighting bits 0-1, Formulas 4-5, Post processing 6-7
    let j = app.ini.get("StickOption").trim().parse::<u32>().unwrap_or(81);
    app.main.sticky = [((j >> 4) & 3) as u8, (j & 3) as u8, ((j >> 6) & 3) as u8];
    // window position and size of the last session
    let (l, tp) = super::ini::two_ints(app.ini.get("m3dPos"), (65, 100));
    let (w, h) = super::ini::two_ints(app.ini.get("m3dSize"), (779, 671));
    {
        let f = ui.fm(F);
        f.left = l;
        f.top = tp;
        f.position = "poDesigned".into();
        f.ctl[0].width = (w - 16).max(400);
        f.ctl[0].height = (h - 38).max(300);
    }
    // FirstShowUpdate
    ui.cm(F, "RadioGroup2").hint = "Panorama mode:\n- The FOV is choosen automatically\n- Use the 24bit SSAO or DEAO for ambient shadows\n- A 2:1 aspect ratio is recommended, but not necessary.".into();
    ui.cm(F, "Edit19").hint = "If set to a higher value than 0, the surface normals will be calculated with an average\nof distributed points and a roughness factor is calculated too, resulting in lower aliasing.\nA value of 8 will average over a volume, what is very slow but can be used in very\ncritical situations.  You could also use the Normals on ZBuf in postprocessings.".into();
    ui.cm(F, "Edit20").hint = "Defines how accurate the position with the defined distance to the surface will be calculated.\nHigher values leads to more accuracy what leads also to a better normals calculation.\nMuch more than 12 are rarely needed.".into();
    ui.cm(F, "UpDown4").hint = "Sharpen factor of the saved output image,\nworks only with downscales of 1:2 and 1:3 !\n0: no sharpening ... 3: maximum sharpening".into();
    ui.cm(F, "Edit16").hint = "Dynamic fog:\nThe lower the number the farer away from the object the fog shows up.\nWith dIFS, the fog is around the object part calculated at this iteration.\nZero to disable this feature and do the dynamic fog on raystep count as usual.\nHint: Check the 'First step random' option to prevent steps.\nVolumetric light:\nChoose the light nr from which lightscattering will be calculated.".into();
    for i in 0..4 {
        load_acc_preset(app, i);
    }
    // the start parameters: a file of the command line or Default.m3p
    let mut start_timer = true;
    let file = app.startup_file.clone().or_else(|| {
        let a = crate::appdirs::config_file("Default.m3p");
        let b = app.ini.dir(super::ini::DIR_M3P).join("Default.m3p");
        [a, b].into_iter().find(|p| p.is_file())
    });
    // FirstShowUpdate: MakeHeader from the start values of the forms, the
    // formula Integer Power in slot 1
    {
        let st = &mut app.formula;
        st.slots = vec![None; 6];
        st.slots[0] = crate::formulas::Formula::default_for("Integer Power");
        st.its = [1, 0, 0, 0, 0, 0];
    }
    app.make_scene(ui);
    app.scene.vgrads = build_rot_matrix(-0.7, -0.0001, 0.0);
    app.scene_to_forms(ui);
    if let Some(p) = file {
        match app.load_params(ui, &p) {
            Ok(()) => {
                if app.startup_file.is_some() {
                    app.message(ui, "Parameters loaded, press \"Calculate 3D\" to render.");
                    start_timer = false;
                }
            }
            Err(e) => app.message(ui, &e),
        }
    }
    app.main.intern_aspect = app.scene.width.max(1) as f64 / app.scene.height.max(1) as f64;
    set_caption(app, ui);
    page_control1_change(app, ui);
    ui.show(F);
    ui.show("LightAdjustForm");
    ui.show("FormulaGUIForm");
    if start_timer && app.eng.base().is_none() {
        // Timer1: a first 2D calculation of the start parameters
        app.main.slice_calc = 2;
        calc_mand(app, ui, false);
    }
    show_image(app, ui);
}

fn load_acc_preset(app: &mut Mb3d, i: usize) {
    let p = crate::appdirs::config_file(PRESET_FILES[i]);
    let Ok(t) = std::fs::read_to_string(p) else { return };
    let pr = &mut app.main.presets[i];
    for l in t.lines() {
        let mut it = l.split_whitespace();
        let (Some(k), Some(v)) = (it.next(), it.next()) else { continue };
        match k {
            "SmoothNormals" => pr.smooth_normals = pti(v) as i64,
            "DEstop" => pr.de_stop = ptf(v),
            "DEaccuracy" | "RayStepFactor" => pr.ray_multiplier = ptf(v),
            "BinSearch" => pr.bin_search = pti(v) as i64,
            "ImageWidth" => pr.image_width = pti(v),
            "ImageScale" => pr.image_scale = pti(v),
            "RayLimiter" => pr.ray_limiter = ptf(v),
            _ => {}
        }
    }
}

fn save_acc_preset(app: &Mb3d, i: usize) {
    let p = &app.main.presets[i];
    let t = format!(
        "SmoothNormals {}\r\nDEstop {}\r\nRayStepFactor {}\r\nBinSearch {}\r\nImageWidth {}\r\nImageScale {}\r\nRayLimiter {}\r\n",
        p.smooth_normals,
        fts_single(p.de_stop),
        fts_single(p.ray_multiplier),
        p.bin_search,
        p.image_width,
        p.image_scale,
        fts_single(p.ray_limiter)
    );
    let _ = std::fs::write(crate::appdirs::config_folder().join(PRESET_FILES[i]), t);
}

/// Where the calculations run (not in MB3D): on the graphics card when the
/// scene allows it, otherwise on the CPU; `MB3D_GPU=0` keeps everything on
/// the CPU.  The LED right of the window title starts unlit (grey).
fn init_compute(ui: &mut Ui) {
    #[cfg(feature = "gpu")]
    crate::gpu::set_enabled(crate::gpu::default_on());
    show_led(ui, Led::Idle, false);
}

/// The "Calculate on" setting (Internal tab, `CalcOn` in the ini):
/// 0 Auto, 1 CPU, 2 graphics card.
fn set_calc_on(app: &mut Mb3d, ui: &mut Ui, i: i64) {
    let i = i.clamp(0, 2);
    ui.set_item_index(F, "CalcOnCombo", i as i32);
    app.ini.set_extra("CalcOn", ["Auto", "CPU", "GPU"][i as usize]);
    #[cfg(feature = "gpu")]
    {
        crate::gpu::set_enabled(i != 1 && crate::gpu::default_on());
        crate::gpu::set_auto(i == 0);
    }
}

/// The title bar LED: where the last calculation ran.
#[derive(Clone, Copy, PartialEq)]
pub enum Led {
    /// nothing calculated yet
    Idle,
    /// everything on the graphics card
    Gpu,
    /// the ray march on the graphics card, post processing on the CPU
    Mixed,
    /// everything on the CPU
    Cpu,
}

/// Sets the LED's colour; it blinks while a calculation runs.
pub fn show_led(ui: &mut Ui, led: Led, blinking: bool) {
    let f = ui.fm(F);
    let col = Some(match led {
        Led::Idle => 0x808080,
        Led::Gpu => 0x30E040,
        Led::Mixed => 0xFF9A1A,
        Led::Cpu => 0xF02828,
    });
    if f.caption_led != col || f.led_blink != blinking {
        if f.led_blink != blinking {
            f.led_on = true;
        }
        f.caption_led = col;
        f.led_blink = blinking;
        f.dirty = true;
    }
}

/// Shown when a calculation starts on the CPU while the card's shaders
/// compile.
pub const COMPILE_NOTE: &str = "First calculation of this formula combination: the graphics card's shaders are being compiled (10-30 s, only once - the driver keeps them). The CPU calculates meanwhile, the card joins when they are ready.";

/// The LED's hint: where the last calculation ran.
pub fn set_led_hint(ui: &mut Ui, text: &str) {
    let f = ui.fm(F);
    if f.led_hint != text {
        f.led_hint = text.to_string();
        if f.led_hint_shown {
            f.hint = None;
            f.led_hint_shown = false;
            f.dirty = true;
        }
    }
}

/// Where the calculation runs, from the GPU status (which also says which
/// post calculations the card ran): `None` while the GPU has not decided.
fn running_led(app: &Mb3d) -> Option<Led> {
    #[cfg(feature = "gpu")]
    {
        let s = crate::gpu::last_status();
        if s.is_empty() {
            return None;
        }
        if !s.starts_with("GPU") {
            return Some(Led::Cpu);
        }
        let sc = &app.scene;
        let shadows = sc.shadows.is_some() && !s.contains("+shadows");
        let ao = sc.ao.is_some() && !(s.contains("+AO") || s.contains("+SSAO"));
        Some(if sc.normals_on_zbuf || shadows || ao { Led::Mixed } else { Led::Gpu })
    }
    #[cfg(not(feature = "gpu"))]
    {
        let _ = app;
        Some(Led::Cpu)
    }
}

// ---------------------------------------------------------------------------
// header <-> controls
// ---------------------------------------------------------------------------

fn set_euler_edits(app: &Mb3d, ui: &mut Ui) {
    match matrix_to_angles(&app.scene.vgrads) {
        Some(v) => {
            set(ui, "Edit27", &fts(v[0] / PID180));
            set(ui, "Edit32", &fts(v[2] / PID180));
            set(ui, "Edit31", &fts(v[1] / PID180));
        }
        None => {
            set(ui, "Edit27", "?");
            set(ui, "Edit32", "?");
        }
    }
}

fn set_edit16(app: &Mb3d, ui: &mut Ui) {
    let w35 = ui.c(F, "Edit35").width;
    match &app.scene.vol_light {
        None => {
            set(ui, "Edit16", &app.scene.dfog_on_it.to_string());
            ui.set_caption(F, "ButtonVolLight", "Dyn. fog on It.:");
            ui.cm(F, "Edit16").width = w35;
            ui.set_visible(F, "UpDown5", false);
            ui.set_visible(F, "Label61", false);
        }
        Some(v) => {
            set(ui, "Edit16", &(v.light + 1).to_string());
            ui.set_caption(F, "ButtonVolLight", "Volume light nr:");
            let ud = ui.c(F, "UpDown5").left;
            let el = ui.c(F, "Edit16").left;
            ui.cm(F, "Edit16").width = ud - el - 2;
            ui.set_visible(F, "UpDown5", true);
            ui.set_position(F, "UpDown5", v.map_size as i64);
            let p = v.map_size;
            ui.set_caption(F, "Label61", &if p > 0 { format!("+{p}") } else { p.to_string() });
            ui.set_visible(F, "Label61", true);
        }
    }
}

/// `SetEditsFromHeader` (the main window's part).
/// A value times 10 as MB3D shows it (`IntToStr(v div 10) + '.' +
/// IntToStr(v mod 10)`).
fn tenths(v: i32) -> String {
    format!("{}.{}", v / 10, v % 10)
}

pub fn set_edits_from_header(app: &mut Mb3d, ui: &mut Ui) {
    let sc = app.scene.clone();
    // the statistics of the calculation the parameters come from (none
    // known: the labels stay empty, as at MB3D's start)
    let st = sc.stats;
    if st != Default::default() {
        ui.set_caption(F, "Label31", &tenths(st.avg_de_steps10));
        ui.set_caption(F, "Label32", &tenths(st.avg_its10));
        ui.set_caption(F, "Label40", &if st.max_its > 0 { st.max_its.to_string() } else { "?".into() });
    }
    app.main.user_change = false;
    // a size that the kept ratio rounds to keeps that ratio (a 2000x1500
    // image halved to 250x187 is still 4:3, not 1.337:1)
    if sc.width > 0 && sc.height > 0 && ((sc.width as f64 / app.main.intern_aspect).round() as i32 - sc.height).abs() > 1 {
        app.main.intern_aspect = sc.width as f64 / sc.height as f64;
    }
    set(ui, "Edit11", &sc.width.to_string());
    set(ui, "Edit12", &sc.height.to_string());
    set(ui, "Edit1", &fts(sc.z_start));
    set(ui, "Edit3", &fts(sc.z_end));
    set(ui, "Edit6", &fts_single(sc.z_step_div));
    set(ui, "Edit9", &fts(sc.mid[0]));
    set(ui, "Edit10", &fts(sc.mid[1]));
    set(ui, "Edit17", &fts(sc.mid[2]));
    set(ui, "Edit5", &fts(sc.zoom));
    set(ui, "Edit14", &fts(sc.fov_y));
    set(ui, "Edit2", &fts_single(sc.color_mul));
    set(ui, "Edit8", &fts_single(sc.raystep_limiter));
    set(ui, "Edit15", &fts_single(sc.stereo_screen[0] as f64));
    set(ui, "Edit18", &fts_single(sc.stereo_screen[1] as f64));
    set(ui, "Edit13", &fts_single(sc.stereo_screen[2] as f64));
    set(ui, "Edit25", &fts_single(sc.de_stop));
    set(ui, "Edit33", &app.main.authors[0].clone());
    set(ui, "Edit34", &app.main.authors[1].clone());
    ui.set_position(F, "SpinEdit2", sc.smooth_normals as i64);
    set(ui, "Edit19", &sc.smooth_normals.to_string());
    ui.set_position(F, "SpinEdit5", sc.bin_search_steps as i64);
    set(ui, "Edit20", &sc.bin_search_steps.to_string());
    ui.set_checked(F, "CheckBox1", sc.normals_on_de);
    ui.set_checked(F, "CheckBox3", sc.first_step_random);
    ui.set_checked(F, "CheckBox2", sc.step_sub_de_stop);
    set(ui, "Edit22", &fts(sc.cut_pos[2]));
    set(ui, "Edit23", &fts(sc.cut_pos[0]));
    set(ui, "Edit24", &fts(sc.cut_pos[1]));
    ui.set_caption(F, "SpeedButton35", &format!("1:{}", app.main.image_scale));
    ui.set_checked(F, "CheckBox4", sc.cut_options & 1 != 0);
    ui.set_checked(F, "CheckBox5", sc.cut_options & 2 != 0);
    ui.set_checked(F, "CheckBox6", sc.cut_options & 4 != 0);
    ui.set_checked(F, "CheckBox9", sc.vary_de_stop_on_fov);
    ui.set_checked(F, "CheckBox7", sc.julia);
    ui.set_item_index(F, "RadioGroup2", sc.optic as i32);
    ui.set_item_index(F, "RadioGroup1", sc.color_option as i32);
    set(ui, "Edit28", &fts(sc.julia_c[0]));
    set(ui, "Edit29", &fts(sc.julia_c[1]));
    set(ui, "Edit30", &fts(sc.julia_c[2]));
    set(ui, "Edit7", &fts(sc.julia_c[3]));
    set_edit16(app, ui);
    set(ui, "Edit35", &(sc.color_on_it as i32 - 1).to_string());
    page_control1_change(app, ui);
    set_euler_edits(app, ui);
    app.main.user_change = true;
}

/// `MakeHeader` (the main window's part).
pub fn make_header(app: &mut Mb3d, ui: &mut Ui) {
    let sc = &mut app.scene;
    let w = pti(&t(ui, "Edit11"));
    let h = pti(&t(ui, "Edit12"));
    if w > 0 && h > 0 && sc.tiling.is_none() {
        sc.width = w;
        sc.height = h;
    }
    sc.normals_on_de = ui.checked(F, "CheckBox1");
    sc.optic = match ui.item_index(F, "RadioGroup2") {
        1 => CameraOptic::Planar,
        2 => CameraOptic::Panorama,
        _ => CameraOptic::Common,
    };
    sc.bin_search_steps = pti(&t(ui, "Edit20")).clamp(0, 50);
    sc.smooth_normals = pti(&t(ui, "Edit19")).clamp(0, 8);
    sc.first_step_random = ui.checked(F, "CheckBox3");
    sc.step_sub_de_stop = ui.checked(F, "CheckBox2");
    sc.z_step_div = ptf(&t(ui, "Edit6")).clamp(0.001, 1.0);
    sc.z_start = ptf(&t(ui, "Edit1"));
    sc.z_end = ptf(&t(ui, "Edit3"));
    sc.mid = [ptf(&t(ui, "Edit9")), ptf(&t(ui, "Edit10")), ptf(&t(ui, "Edit17"))];
    sc.zoom = ptf(&t(ui, "Edit5"));
    sc.fov_y = ptf(&t(ui, "Edit14"));
    sc.cut_pos = [ptf(&t(ui, "Edit23")), ptf(&t(ui, "Edit24")), ptf(&t(ui, "Edit22"))];
    sc.cut_options = ui.checked(F, "CheckBox4") as u8 | (ui.checked(F, "CheckBox5") as u8) << 1 | (ui.checked(F, "CheckBox6") as u8) << 2;
    sc.de_stop = ptf(&t(ui, "Edit25")).max(0.001);
    sc.julia = ui.checked(F, "CheckBox7");
    sc.julia_c = [ptf(&t(ui, "Edit28")), ptf(&t(ui, "Edit29")), ptf(&t(ui, "Edit30")), ptf(&t(ui, "Edit7"))];
    sc.color_mul = ptf(&t(ui, "Edit2"));
    sc.raystep_limiter = ptf(&t(ui, "Edit8"));
    sc.stereo_screen = [ptf(&t(ui, "Edit15")) as f32, ptf(&t(ui, "Edit18")) as f32, ptf(&t(ui, "Edit13")) as f32];
    sc.vary_de_stop_on_fov = ui.checked(F, "CheckBox9");
    sc.color_option = ui.item_index(F, "RadioGroup1").clamp(0, 5) as u8;
    sc.color_on_it = (pti(&t(ui, "Edit35")) + 1).clamp(0, 255) as u8;
    if ui.caption(F, "ButtonVolLight") == "Dyn. fog on It.:" {
        sc.vol_light = None;
        sc.dfog_on_it = pti(&t(ui, "Edit16")).clamp(0, 65535) as u16;
    } else {
        sc.vol_light = Some(crate::vollight::VolLightParams {
            light: (pti(&t(ui, "Edit16")).clamp(1, 6) - 1) as usize,
            map_size: ui.position(F, "UpDown5") as i32,
        });
    }
    let n = ui.position(F, "UpDown3").clamp(1, 64) as usize;
    sc.threads = if n == std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0) { 0 } else { n };
}

/// `PageControl1Change`: tab captions show active options.
pub fn page_control1_change(_app: &Mb3d, ui: &mut Ui) {
    let julia = ui.checked(F, "CheckBox7");
    ui.cm(F, "TabSheet9").caption = if julia { "Julia On".into() } else { "Julia Off".into() };
    let cut = ui.checked(F, "CheckBox4") || ui.checked(F, "CheckBox5") || ui.checked(F, "CheckBox6");
    ui.cm(F, "TabSheet8").caption = if cut { "Cutting*".into() } else { "Cutting".into() };
}

fn update_aspect_caption(app: &Mb3d, ui: &mut Ui) {
    let (x, y) = app.main.user_aspect;
    let c = if x > 0 { format!("{x}:{y}") } else { "user".into() };
    ui.set_caption(F, "SpeedButton21", &c);
}

fn all_presets_up(ui: &mut Ui) {
    for b in ["SpeedButton3", "SpeedButton13", "SpeedButton5", "SpeedButton6"] {
        ui.cm(F, b).down = false;
    }
}

fn scale_de_stop(ui: &mut Ui, s: f64) {
    if ui.checked(F, "CheckBox11") {
        if let Some(d) = parse_float(&t(ui, "Edit25")) {
            set(ui, "Edit25", &fts_single(d * s));
        }
    }
}

fn scale_rclip(ui: &mut Ui, s: f64) {
    if ui.checked(F, "CheckBox11") && ui.has_form("PostProForm") {
        if let Some(d) = parse_float(&ui.text("PostProForm", "Edit2")) {
            ui.set_text("PostProForm", "Edit2", &fts_single(d * s));
        }
    }
}

// ---------------------------------------------------------------------------
// calculation
// ---------------------------------------------------------------------------

const CALC_BUTTONS: [&str; 22] = [
    "SpeedButton32", "SpeedButton33", "SpeedButton34", "Button1", "Button6", "Button5", "Button9", "Button11", "Button12", "SpeedButton8",
    "SpeedButton11", "SpeedButton16", "SpeedButton17", "SpeedButton18", "SpeedButton22", "SpeedButton23", "SpeedButton1", "SpeedButton2",
    "SpeedButton4", "IniDirsBtn", "MapSequencesBtn", "VisualThemesBtn",
];

pub fn disable_buttons(app: &mut Mb3d, ui: &mut Ui) {
    for b in CALC_BUTTONS {
        ui.set_enabled(F, b, false);
    }
    ui.set_enabled(F, "SpeedButton9", false);
    ui.set_caption(F, "Button2", "Stop");
    ui.cm(F, "Button2").hint = "Stop the current calculation.".into();
    ui.cm(F, "Image1").cursor = "crHourGlass".into();
    ui.set_position(F, "ProgressBar1", 0);
    // (the frame number makes room for the stage and its time left)
    ui.set_visible(F, "Panel7", false);
    app.main.calculating = true;
}

pub fn enable_buttons(app: &mut Mb3d, ui: &mut Ui) {
    ui.set_caption(F, "Label6", "");
    ui.set_caption(F, "Button2", "Calculate 3D");
    ui.cm(F, "Button2").hint = "Sart the main rendering of the image.".into();
    for b in CALC_BUTTONS {
        ui.set_enabled(F, b, true);
    }
    let undo_ok = app.main.undo.len() > 1;
    ui.set_enabled(F, "SpeedButton9", undo_ok);
    ui.set_visible(F, "ProgressBar1", false);
    ui.set_visible(F, "Panel7", true);
    app.main.calculating = false;
    // a stopped or failed calculation saves nothing later
    app.main.auto_m3i = None;
    app.main.auto_stereo = None;
    set_image_cursor(app, ui);
}

fn set_image_cursor(app: &Mb3d, ui: &mut Ui) {
    let c = if app.main.get_pos > 0 || ui.visible(F, "Shape2") || selecting(ui) {
        "crCross"
    } else if ui.checked(F, "SpeedButton2") {
        "crHandPoint"
    } else {
        "crDefault"
    };
    ui.cm(F, "Image1").cursor = c.into();
}

/// "Paint on image" of the colour window on / off: the brush (`Shape2`)
/// on the image, the navigation buttons up (`MButtonsUp`).
pub fn show_paint_brush(app: &mut Mb3d, ui: &mut Ui, on: bool) {
    if on {
        for b in ["SpeedButton1", "SpeedButton2", "SpeedButton4"] {
            ui.set_checked(F, b, false);
        }
    }
    ui.set_visible(F, "Shape2", on);
    set_image_cursor(app, ui);
}

/// `SetShape2Size`: the brush size (3..31), around its middle.
fn set_brush_size(ui: &mut Ui, size: i32) {
    if !(3..=31).contains(&size) {
        return;
    }
    let r = ui.c(F, "Shape2").rect();
    let (mx, my) = (r.x + r.w / 2, r.y + r.w / 2);
    ui.cm(F, "Shape2").set_bounds(mx - size / 2, my - size / 2, size, size);
}

/// `ModColOnImage` at the image position `x`, `y` (view pixels).
fn paint_on_image(app: &mut Mb3d, ui: &mut Ui, x: i32, y: i32) {
    if app.main.calculating {
        return;
    }
    let s = app.main.image_scale.max(1);
    let b = ui.c(F, "Shape2");
    let (radius, rect) = ((b.width / 2) * s, b.shape == "stRectangle");
    if super::palette_form::paint_on_image(app, x * s, y * s, radius, rect) {
        super::light_form::trigger_repaint(app, ui);
    }
}

/// The windows that can stick to the main window, in `FormsSticky` order,
/// with the ini item of their free position.
const STICKY: [(&str, &str); 3] = [("FormulaGUIForm", "FormulaPos"), ("LightAdjustForm", "LightPos"), ("PostProForm", "PostpPos")];

/// Puts the sticking windows beside the main window (MB3D's `WM_Move` and
/// `FormResize`); `free` also puts a free one shown now at its position of
/// the last session.
pub fn place_sticky(app: &Mb3d, ui: &mut Ui, only: Option<&str>, free: bool) {
    let m = ui.f(F);
    let (ml, mt) = (m.left, m.top);
    let (mw, _) = m.outer_size(&ui.theme);
    for (k, (form, pos)) in STICKY.iter().enumerate() {
        if only.is_some_and(|o| o != *form) || !ui.has_form(form) {
            continue;
        }
        let (w, _) = ui.f(form).outer_size(&ui.theme);
        match app.main.sticky[k] {
            1 => ui.move_form(form, ml + mw, mt),
            2 => ui.move_form(form, ml - w, mt),
            _ if free => {
                let (x, y) = super::ini::two_ints(app.ini.get(pos), (844, 100));
                ui.move_form(form, x, y);
            }
            _ => {}
        }
    }
}

/// `StickOption` of the ini (keeps the Animation window's bits).
fn store_sticky(app: &mut Mb3d) {
    let j = app.ini.get("StickOption").trim().parse::<u32>().unwrap_or(81);
    let s = app.main.sticky.map(|v| v as u32 & 3);
    let j = (j & 0b1100) | s[1] | (s[0] << 4) | (s[2] << 6);
    app.ini.set("StickOption", &j.to_string());
}

/// Post processing > "Recalculate a selection" is enabled: the mouse marks
/// the selection on the image.
fn selecting(ui: &Ui) -> bool {
    ui.has_form("PostProForm") && (ui.checked("PostProForm", "CheckBox21") || ui.checked("PostProForm", "CheckBox25"))
}

fn not_all_buttons_up(ui: &Ui) -> bool {
    ui.checked(F, "SpeedButton1") || ui.checked(F, "SpeedButton2") || ui.checked(F, "SpeedButton4")
}

fn store_undo(app: &mut Mb3d) {
    let s = app.scene.clone();
    app.main.undo.truncate(app.main.undo_pos);
    if app.main.undo.last().map(|l| l.to_text()) != Some(s.to_text()) {
        app.main.undo.push(s);
        if app.main.undo.len() > 60 {
            app.main.undo.remove(0);
        }
    }
    app.main.undo_pos = app.main.undo.len();
}

/// `CalcMand`: starts the calculation (3D or the 2D slice of `slice_calc`).
pub fn calc_mand(app: &mut Mb3d, ui: &mut Ui, calc3d: bool) {
    // a new calculation: only the automatic post processings
    app.post = super::postpro_form::State::default();
    app.make_scene(ui);
    if calc3d {
        store_undo(app);
    }
    if app.scene.width < 1 || app.scene.height < 1 {
        app.main.auto_m3i = None;
        return;
    }
    // the whole picture is held here (with the swarm too): one that does
    // not fit would only swap until the system stops the program
    if calc3d {
        if let Err(e) = crate::render::check_memory(app.scene.width as usize, app.scene.height as usize) {
            app.main.auto_m3i = None;
            app.message(ui, &format!("Not calculated: {e}.\n\nBig render (Tools tab) calculates it in tiles saved one by one, without holding the whole picture (with the swarm too)."));
            return;
        }
    }
    app.main.calc3d = calc3d;
    app.main.calc_start = Instant::now();
    app.main.stage_shown = "main rendering".into();
    app.main.stage_start = Instant::now();
    app.main.stage_compile = compile_secs();
    app.main.later = if calc3d { later_stages(&app.scene) } else { Vec::new() };
    app.main.passed.clear();
    app.main.samples.clear();
    app.main.curves.clear();
    app.main.eta_f = super::eta::scene_features(&app.scene);
    app.main.later_ratio = super::eta::later_ratios(&app.main.eta_f);
    app.main.progress_shown = 0.0;
    app.main.main_cold = false;
    app.main.main_unmeasured = false;
    app.main.main_secs = 0.0;
    app.main.f_compiled = None;
    app.main.calc_compile = compile_secs();
    app.main.later_secs = [None; 3];
    app.main.later_cold = [false; 3];
    app.main.calc_sig = if calc3d { picture_sig(&app.scene) } else { String::new() };
    app.main.stage_base = 0.0;
    app.main.main_full = None;
    app.main.total_shown = None;
    ui.set_visible(F, "Shape1", false);
    disable_buttons(app, ui);
    for l in ["Label8", "Label48", "Label50"] {
        ui.set_caption(F, l, "-");
    }
    ui.set_visible(F, "ProgressBar1", calc3d);
    if calc3d {
        ui.set_caption(F, "Label6", "main rendering");
        #[cfg(feature = "gpu")]
        crate::gpu::set_status(String::new());
        app.main.led_running = true;
        app.main.compile_note = false;
    }
    let slice = if calc3d { 0 } else { app.main.slice_calc.clamp(1, 3) };
    if let (true, Some(dir)) = (calc3d, super::swarm_form::calc_folder(app)) {
        let (cols, rows) = app.swarm.tiles;
        app.eng.start(Job::Swarm { scene: app.scene.clone(), dir, cols, rows, name: app.swarm.name.clone() });
        return;
    }
    start_calc(app, ui, app.scene.clone(), slice);
}

/// Starts a two-eye render after its output filename was selected.
fn start_stereo_pair(app: &mut Mb3d, ui: &mut Ui) {
    app.make_scene(ui);
    store_undo(app);
    let layout = if ui.checked(F, "StereoCrossCheck") {
        crate::render::StereoLayout::Cross
    } else {
        crate::render::StereoLayout::Parallel
    };
    app.main.calc3d = true;
    app.main.calc_start = Instant::now();
    app.main.stage_shown = "stereo: rendering left eye".into();
    app.main.stage_start = Instant::now();
    ui.set_visible(F, "Shape1", false);
    disable_buttons(app, ui);
    ui.set_visible(F, "ProgressBar1", true);
    ui.set_caption(F, "Label6", "stereo: rendering left eye");
    app.main.led_running = true;
    app.main.compile_note = false;
    #[cfg(feature = "gpu")]
    crate::gpu::set_status(String::new());
    app.eng.start(Job::Stereo { scene: app.scene.clone(), layout });
}

/// Starts the calculation of `sc`; when an unfinished calculation of it
/// was kept, asks first whether to continue it.
fn start_calc(app: &mut Mb3d, ui: &mut Ui, sc: Scene, slice: u8) {
    if let Some(key) = app.main.resume_key.take() {
        if slice == 0 && super::engine::unfinished_calc(&sc).is_some_and(|e| e.key() == key) {
            run_calc(app, sc, slice, crate::checkpoint::Policy::Continue);
        } else {
            not_started(app, ui);
            app.message(ui, "The parameters do not match the unfinished calculation any more: press \"Calculate 3D\" to calculate them.");
        }
        return;
    }
    // (the Batch window goes on without asking)
    if slice == 0 && app.batch.status == 0 {
        if let Some(e) = super::engine::unfinished_calc(&sc) {
            app.main.resume_ask = Some((sc, slice));
            ui.message_box(
                "main:resume",
                "Unfinished calculation",
                &format!("An unfinished calculation of this picture was kept ({}).\n\nContinue it?  \"No\" calculates it again from the start.", e.summary()),
                &[MR_YES, MR_NO, MR_CANCEL],
            );
            return;
        }
    }
    run_calc(app, sc, slice, crate::checkpoint::Policy::Continue);
}

/// A calculation prepared by [`calc_mand`] is not started after all.
fn not_started(app: &mut Mb3d, ui: &mut Ui) {
    enable_buttons(app, ui);
    ui.set_visible(F, "ProgressBar1", false);
    if app.main.led_running {
        app.main.led_running = false;
        show_led(ui, app.main.led, false);
    }
}

/// Calculate 3D of the current parameters, continuing the unfinished
/// calculation of `key` (the Unfinished renders window).
pub fn continue_calc(app: &mut Mb3d, ui: &mut Ui, key: &str) {
    if app.eng.running() {
        app.message(ui, "A calculation is running: stop it first.");
        return;
    }
    app.main.resume_key = Some(key.to_string());
    calc_mand(app, ui, true);
    // (not started: the key is not left for a later calculation)
    app.main.resume_key = None;
}

/// Parameters were loaded (not from a file): into the forms.
pub fn scene_loaded(app: &mut Mb3d, ui: &mut Ui) {
    text_pars_load_success(app, ui);
}

fn run_calc(app: &mut Mb3d, sc: Scene, slice: u8, resume: crate::checkpoint::Policy) {
    app.eng.set_preview_step(app.main.image_scale.max(1) as usize);
    app.eng.start(Job::Calc { scene: sc, slice, resume, name: stem(&app.title) });
}

/// The stages of a 3D calculation after the main rendering, in their
/// order: hard shadows and ambient occlusion, reflections, depth of field.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Later {
    Post,
    Reflections,
    Dof,
}

fn later_of(stage: &str) -> Option<Later> {
    if stage.starts_with("reflections") {
        Some(Later::Reflections)
    } else if stage.starts_with("depth of field") {
        Some(Later::Dof)
    } else if stage.contains("shadows") || stage.contains("ambient occlusion") {
        Some(Later::Post)
    } else {
        None
    }
}

fn later_stages(sc: &Scene) -> Vec<Later> {
    let mut v = Vec::new();
    if sc.shadows.is_some() || sc.ao.is_some() {
        v.push(Later::Post);
    }
    if sc.mc.reflections {
        v.push(Later::Reflections);
    }
    if sc.dof.is_some() {
        v.push(Later::Dof);
    }
    v
}

/// What makes a picture the same for the stages' times: its parameters
/// (not the statistics of its last calculation).
fn picture_sig(sc: &Scene) -> String {
    let mut s = sc.clone();
    s.stats = Default::default();
    s.threads = 0;
    format!("{s:?}")
}

/// The time later stage `k` is expected to take: as the last time for the
/// same picture, else from the main rendering's and the model's ratio.
fn expected(m: &State, k: Later) -> Option<f64> {
    if m.calc_sig == m.last_sig {
        if let Some(t) = m.last_secs[k as usize] {
            return Some(t);
        }
    }
    m.main_full.map(|full| m.later_ratio[k as usize] * full)
}

/// The time left in the stage: `f` its progress, `base` the part done
/// before it began, `secs` its time so far (None while too early).
fn stage_left(f: f64, base: f64, secs: f64, samples: &[(f64, f64)], last: Option<&[(f64, f64)]>) -> Option<f64> {
    let g = (f - base) / (1.0 - base).max(1e-6);
    if secs < 3.0 || g < 0.02 || f >= 1.0 {
        return None;
    }
    // the same stage of the same picture before: its time from here on,
    // at the pace of this one so far
    if let Some(c) = last.filter(|c| base == 0.0 && c.len() >= 4) {
        let at = |f: f64| {
            let i = c.partition_point(|s| s.0 < f).clamp(1, c.len() - 1);
            let ((f0, t0), (f1, t1)) = (c[i - 1], c[i]);
            t0 + (t1 - t0) * ((f - f0) / (f1 - f0).max(1e-9)).clamp(0.0, 1.0)
        };
        let (then, end) = (at(f), c[c.len() - 1].1);
        if then > 1.0 {
            return Some((end - then) * (secs / then).clamp(0.33, 3.0));
        }
    }
    // the rate of the last stretch (a third of the time so far, at least 5
    // seconds): it changes, e.g. when the card joins the CPU threads
    let from = secs - (secs / 3.0).max(5.0);
    // (else since the first sample: those of the card's compile are gone)
    let rate = match samples.iter().rev().find(|s| s.1 <= from).or(samples.first()) {
        Some(&(f0, t0)) if f > f0 && secs - t0 >= 2.0 => (f - f0) / (secs - t0),
        Some(_) => return None,
        None => (f - base) / secs,
    };
    Some((1.0 - f) / rate.max(1e-9))
}

/// A stage's name as the bar's caption shows it: without where it runs.
fn short_stage(stage: &str) -> String {
    stage.split(" (").next().unwrap_or(stage).replace("hard shadows and ambient occlusion", "shadows and AO")
}

/// "reflections · 6 min left in total", "hard shadows: 40 % · 20 s left":
/// the stage, and once the rate is known the time left in the whole
/// calculation, else its progress and time left (short: it has the room
/// beside the bar; the LED shows where it runs).
fn stage_caption(stage: &str, f: f64, left: Option<f64>, total: Option<f64>) -> String {
    let name = short_stage(stage);
    if let Some(tl) = total {
        return format!("{name} · {} left in total", super::util::about_str(tl));
    }
    let mut t = if f <= 0.0 { name } else { format!("{name}: {} %", (f * 100.0).floor() as i64) };
    if let Some(l) = left {
        t += &format!(" · {} left", super::util::about_str(l));
    }
    t
}

/// The time left in the stage, and in total when stages follow it (from
/// the time a whole main rendering takes and the stages' ratios); and the
/// whole calculation's progress, once its time left is known.
fn time_left(app: &mut Mb3d, stage: &str, f: f64, base: f64) -> (Option<f64>, Option<f64>, Option<f64>) {
    let m = &mut app.main;
    let secs = m.stage_start.elapsed().as_secs_f64();
    let main = stage.starts_with("main rendering");
    if main {
        super::eta::march_features(&mut m.eta_f, &app.eng.0, secs);
        m.later_ratio = super::eta::later_ratios(&m.eta_f);
    }
    if f > 0.0 && m.samples.last().is_none_or(|s| secs - s.1 >= 0.25 && f > s.0) {
        m.samples.push((f, secs));
    }
    let last = if m.calc_sig == m.last_sig { m.last_curves.iter().find(|c| c.0 == stage).map(|c| c.1.as_slice()) } else { None };
    let mut left = stage_left(f, base, secs, &m.samples, last);
    // the card's shaders compiling: the CPU threads alone so far, much
    // slower than with the card once it joins
    let compiling = shaders_compiling();
    if main && m.main_cold && !compiling && m.f_compiled.is_none() {
        m.f_compiled = Some(f);
    }
    // (later stages go on as usual: their times come from the main
    // rendering's, the CPU's alone if it ran while compiling)
    if compiling && main {
        m.samples.clear();
        m.main_cold = true;
        return compiling_estimate(m, f, base, secs);
    }
    // a later stage without progress (e.g. the reflections on the card):
    // its expected time (for the bar also when it takes longer)
    let mut bar_left = left;
    if let (None, Some(k)) = (left, later_of(stage)) {
        let l = expected(m, k).unwrap_or(0.0) - secs;
        if f <= 0.0 && l > 1.0 {
            left = Some(l);
        }
        bar_left = expected(m, k).map(|e| (e - secs).max(0.0));
    }
    let at = if main {
        // (the whole picture, also when continued; after the card's
        // shaders compiled, at the pace since)
        if let Some(l) = left {
            m.main_full = Some(if m.main_cold { l / (1.0 - f).max(0.02) } else { (secs + l) / (1.0 - base).max(0.02) });
        }
        None
    } else {
        match later_of(stage) {
            Some(k) => Some(k),
            None => return (left, None, None),
        }
    };
    let after: Vec<Later> = match at {
        None => m.later.clone(),
        Some(k) => m.later.iter().copied().skip_while(|l| *l != k).skip(1).collect(),
    };
    let Some(rest) = after.iter().map(|k| expected(m, *k)).sum::<Option<f64>>() else { return (left, None, None) };
    // the time the calculation took so far: the main rendering's part done
    // (also of a continued one), the later stages', this one's
    let done = match m.main_full {
        Some(full) if main => left.map(|l| (full - l).max(0.0)),
        Some(full) => Some(full + m.later_secs.iter().flatten().sum::<f64>() + secs),
        None => None,
    };
    let Some(l) = left else {
        let p = done.zip(bar_left).map(|(d, l)| d / (d + l + rest).max(1e-6));
        return (left, None, p);
    };
    // smoothed over a few seconds: the rate of a stage changes as it goes
    let shown = super::util::smooth_left(m.total_shown, l + rest, 4.0);
    m.total_shown = Some(shown);
    let p = done.map(|d| d / (d + shown.0).max(1e-6));
    // (no total when the stages after it are short)
    if rest < 2.0 {
        return (left, None, p);
    }
    (left, Some(shown.0), p)
}

/// The time left in total while the card's shaders compile during the
/// main rendering (None for the stage's own time): the CPU threads' pace so
/// far finishes it, unless the shaders are predicted done clearly before
/// (by half again): then the card joins and the rest runs `SPEEDUP` times
/// faster. The later stages at the model's ratios to that main rendering.
/// The whole calculation's progress with it.
fn compiling_estimate(m: &mut State, f: f64, base: f64, secs: f64) -> (Option<f64>, Option<f64>, Option<f64>) {
    let g = f - base;
    if secs < 2.0 || g < 0.01 {
        return (None, None, None);
    }
    let rate = g / secs;
    let cpu_left = (1.0 - f) / rate;
    #[cfg(feature = "gpu")]
    let tc = super::eta::compile_left(m.calc_start);
    #[cfg(not(feature = "gpu"))]
    let tc: Option<f64> = None;
    let (main_left, full, alone) = match tc {
        Some(tc) if cpu_left > 1.5 * tc => {
            let warm = 1.0 / rate / super::eta_model::SPEEDUP;
            (tc + (1.0 - (f + rate * tc).min(1.0)) * warm, warm, 0.0)
        }
        _ => (cpu_left, 1.0 / rate, 1.0),
    };
    let mut ef = m.eta_f;
    ef[super::eta::CPU_ALONE] = alone;
    let ratio = super::eta::later_ratios(&ef);
    let rest: f64 = m.later.iter().map(|k| ratio[*k as usize] * full).sum();
    let shown = super::util::smooth_left(m.total_shown, main_left + rest, 4.0);
    m.total_shown = Some(shown);
    // (a continued calculation's rows done before, at this pace)
    let done = secs + base / rate;
    (None, Some(shown.0), Some(done / (done + shown.0).max(1e-6)))
}

/// The whole calculation's progress before its time left is known: the
/// main rendering's part of it by the stages' expected ratios (None in a
/// later stage, `f` in one of its own, e.g. the swarm's).
fn rough_progress(m: &State, stage: &str, f: f64) -> Option<f64> {
    if stage.starts_with("main rendering") {
        let rest: f64 = m.later.iter().map(|k| m.later_ratio[*k as usize]).sum();
        Some(f / (1.0 + rest))
    } else if later_of(stage).is_some() {
        None
    } else {
        Some(f)
    }
}

/// Whether the graphics card's shaders are being compiled.
fn shaders_compiling() -> bool {
    #[cfg(feature = "gpu")]
    return crate::gpu::compiling();
    #[cfg(not(feature = "gpu"))]
    false
}

/// The time the graphics card's shaders took to compile so far.
fn compile_secs() -> f64 {
    #[cfg(feature = "gpu")]
    return crate::gpu::compile_ms() as f64 / 1000.0;
    #[cfg(not(feature = "gpu"))]
    0.0
}

/// A stage ended after `secs`: the main rendering's whole time, a later
/// stage's time (and whether shaders compiled meanwhile).
fn stage_ended(app: &mut Mb3d, stage: &str, secs: f64, base: f64) {
    let m = &mut app.main;
    let compiled = compile_secs() - m.stage_compile;
    // (its curve ends where it ended; not one of a continued stage)
    let mut c = std::mem::take(&mut m.samples);
    // (its pace since the card's shaders compiled: the samples start there)
    let after_compile = c.first().filter(|s| m.main_cold && s.0 < 0.9).map(|&(f0, t0)| (secs - t0) / (1.0 - f0));
    if base == 0.0 && c.len() >= 4 {
        c.push((1.0, secs));
        m.curves.push((stage.to_string(), c));
    }
    if stage.starts_with("main rendering") {
        // the whole main rendering's time; when the card's shaders compiled
        // meanwhile, at the pace it had after (the later stages' ratios are
        // to a main rendering without compiling)
        m.main_full = Some(after_compile.unwrap_or(secs / (1.0 - base).max(0.02)));
        m.main_unmeasured = m.main_cold && after_compile.is_none();
        // (the later stages then at the model's ratios to a CPU-only main rendering)
        if m.main_unmeasured {
            m.eta_f[super::eta::CPU_ALONE] = 1.0;
            m.later_ratio = super::eta::later_ratios(&m.eta_f);
        }
        m.main_secs = secs;
    } else if let Some(k) = later_of(stage) {
        // (the shadows and the occlusion one after the other on the CPU:
        // together)
        let t = m.later_secs[k as usize].unwrap_or(0.0) + secs;
        m.later_secs[k as usize] = Some(t);
        m.later_cold[k as usize] |= compiled > 0.0 || shaders_compiling();
    }
}

/// Polls the engine (MB3D's Timer4 / WM_ThreadReady).
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    let (ver, finished, running, stage, base, err, stats, cancelled) = {
        let o = app.eng.0.out.lock().unwrap();
        (o.ver, o.finished, o.running, o.stage.clone(), o.stage_base, o.error.clone(), o.stats, o.cancelled)
    };
    if app.main.calculating && app.main.led_running {
        let led = running_led(app).unwrap_or(app.main.led);
        app.main.led = led;
        show_led(ui, led, true);
        // Auto started on the CPU: the first calculation of these formulas
        // compiles the card's shaders
        #[cfg(feature = "gpu")]
        if !app.main.compile_note && crate::gpu::last_status().contains("shaders are compiling") {
            app.main.compile_note = true;
            app.message(ui, COMPILE_NOTE);
        }
    }
    if app.main.calculating {
        let f = app.eng.progress();
        let mut bar = f as f64;
        if !stage.is_empty() && app.main.calc3d {
            if stage != app.main.stage_shown {
                let was = std::mem::replace(&mut app.main.stage_shown, stage.clone());
                stage_ended(app, &was, app.main.stage_start.elapsed().as_secs_f64(), app.main.stage_base);
                // the stages it skipped (run with the main rendering on the
                // card): none next time
                if let Some(k) = later_of(&stage) {
                    for l in app.main.later.clone().into_iter().take_while(|l| *l != k) {
                        if !app.main.passed.contains(&l) {
                            app.main.later_secs[l as usize] = Some(0.0);
                        }
                    }
                    app.main.passed.push(k);
                }
                app.main.stage_start = Instant::now();
                app.main.stage_compile = compile_secs();
                app.main.total_shown = None;
            }
            app.main.stage_base = base as f64;
            let (left, total, p) = time_left(app, &stage, f as f64, base as f64);
            // the card's shaders compiling: the time left with them, once
            // the CPU threads' pace is known
            let compiling = shaders_compiling();
            let caption = match (compiling, total.or(left)) {
                (true, Some(tl)) => format!("shaders compiling · {} left", super::util::about_str(tl)),
                (true, None) => format!("{} · shaders compiling", short_stage(&stage)),
                _ => stage_caption(&stage, f as f64, left, total),
            };
            ui.set_caption(F, "Label6", &caption);
            // the bar: the whole calculation's progress, not the stage's
            // (it does not go back when an estimate grows); none without
            // an estimate of the whole
            let p = if compiling { p } else { p.or_else(|| rough_progress(&app.main, &stage, f as f64)) };
            if let Some(p) = p {
                app.main.progress_shown = app.main.progress_shown.max(p.clamp(0.0, 1.0));
            }
            ui.set_visible(F, "ProgressBar1", p.is_some());
            bar = app.main.progress_shown;
        }
        ui.set_position(F, "ProgressBar1", (bar * ui.c(F, "ProgressBar1").max as f64) as i64);
        // the rows calculated so far (MB3D shows the image while it grows)
        let iv = if app.scene.width * app.scene.height > 2_000_000 { 900 } else { 300 };
        if running && app.main.last_partial.elapsed().as_millis() >= iv {
            app.main.last_partial = Instant::now();
            if let Some((rgb, w, h)) = app.eng.partial_image() {
                // (already at the size shown)
                show_bitmap(ui, &rgb, w, h);
            }
        }
    }
    if ver != app.main.img_ver {
        app.main.img_ver = ver;
        show_image(app, ui);
    }
    if finished != app.main.finished_seen && !running {
        app.main.finished_seen = finished;
        if app.main.calculating {
            // (saved only after a calculation that finished)
            let (auto_m3i, auto_stereo) = (app.main.auto_m3i.take(), app.main.auto_stereo.take());
            enable_buttons(app, ui);
            if app.main.led_running {
                app.main.led_running = false;
                show_led(ui, app.main.led, false);
            }
            if cancelled {
                // (new parameters were loaded during it: nothing of it is kept)
            } else if !err.is_empty() {
                app.message(ui, &err);
            } else if app.main.calc3d {
                let secs = app.main.calc_start.elapsed().as_secs_f64();
                // the last stage's time, and the later stages that did not
                // run (with the main rendering on the card): none next time
                let last = std::mem::take(&mut app.main.stage_shown);
                stage_ended(app, &last, app.main.stage_start.elapsed().as_secs_f64(), app.main.stage_base);
                if app.main.main_full.is_some() {
                    for l in app.main.later.clone() {
                        if !app.main.passed.contains(&l) {
                            app.main.later_secs[l as usize] = Some(0.0);
                        }
                    }
                    {
                        let m = &app.main;
                        let has = |k: Later| m.later.contains(&k);
                        let t = |k: Later| if has(k) { m.later_secs[k as usize].or(Some(0.0)) } else { None };
                        let cold = compile_secs() - m.calc_compile > 0.2;
                        let x = super::eta::LogExtra { start: m.calc_start, main_secs: m.main_secs, f_compiled: m.f_compiled };
                        super::eta::log(&m.eta_f, m.main_full.unwrap_or(0.0), [t(Later::Post), t(Later::Reflections), t(Later::Dof)], cold, &stem(&app.title), &x);
                    }
                    let m = &mut app.main;
                    m.last_secs = std::array::from_fn(|k| m.later_secs[k].filter(|_| !m.later_cold[k]));
                    app.main.last_curves = std::mem::take(&mut app.main.curves);
                    app.main.last_sig = std::mem::take(&mut app.main.calc_sig);
                }
                // CalcStatistic: kept in the parameters (iAvrgDEsteps,
                // iAvrgIts, iMaxIts)
                app.scene.stats = crate::scene::CalcStats {
                    avg_de_steps10: (stats.avg_steps * 10.0).round() as i32,
                    avg_its10: (stats.avg_its * 10.0).round() as i32,
                    max_its: stats.max_its,
                };
                let st = app.scene.stats;
                ui.set_caption(F, "Label31", &if st.avg_de_steps10 == 0 { "?".into() } else { tenths(st.avg_de_steps10) });
                ui.set_caption(F, "Label32", &tenths(st.avg_its10));
                ui.set_caption(F, "Label40", &st.max_its.to_string());
                ui.set_caption(F, "Label52", &time_str(((stats.calc_s * 10.0) as i64).max(1)));
                app.main.led = running_led(app).unwrap_or(Led::Cpu);
                show_led(ui, app.main.led, false);
                #[cfg(feature = "gpu")]
                set_led_hint(ui, &format!("Calculated on: {}", crate::gpu::last_status()));
                #[cfg(not(feature = "gpu"))]
                set_led_hint(ui, "Calculated on the CPU");
                // the stages after the main calculation: "-" when the scene
                // has none, "incl. in main" when the card ran it with the
                // march, "with HS" when it ran with the hard shadows
                let stage = |t: Option<Option<f64>>| match t {
                    None => "-".to_string(),
                    Some(None) => "incl. in main".to_string(),
                    Some(Some(s)) => time_str(((s * 10.0) as i64).max(1)),
                };
                ui.set_caption(F, "Label8", &stage(stats.stages.shadows));
                let ao = if stats.stages.ao_with_shadows { "with HS".to_string() } else { stage(stats.stages.ao) };
                ui.set_caption(F, "Label48", &ao);
                ui.set_caption(F, "Label50", &stage(stats.stages.reflections.map(Some)));
                let _ = secs;
                if let Some(p) = auto_m3i {
                    save_m3i(app, ui, &p);
                }
                if let Some(p) = auto_stereo {
                    save_image(app, ui, &p);
                }
            } else {
                super::postpro_form::calc_finished(app, ui);
            }
            if app.batch.status != 0 && !cancelled {
                super::tools_forms::batch_finished(app, ui);
            }
        }
        super::light_form::calc_finished(app, ui);
    }
    // the history keeps the parameters after lighting changes (Timer2)
    if !app.main.calculating && app.main.last_history.elapsed().as_secs() >= 300 {
        app.main.last_history = Instant::now();
    }
}

/// Shows the engine's image at the viewing scale (`UpdateScaledImage`).
pub fn show_image(app: &mut Mb3d, ui: &mut Ui) {
    let (rgb, w, h) = {
        let o = app.eng.0.out.lock().unwrap();
        (o.rgb.clone(), o.w, o.h)
    };
    match rgb {
        Some(rgb) => {
            // Post processing > Noise Filter > "Filter new pictures automatically"
            let rgb = if super::postpro_form::noise_auto(ui) {
                std::sync::Arc::new(denoised(&rgb, w, h, super::postpro_form::noise_sigma(ui)))
            } else {
                rgb
            };
            app.main.rgb = Some((rgb.clone(), w, h));
            set_image(app, ui, rgb, w, h);
        }
        None => {
            app.main.rgb = None;
            clear_screen(app, ui);
        }
    }
}

fn denoised(rgb: &[u8], w: usize, h: usize, sigma: f32) -> Vec<u8> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    crate::denoise::dct_denoise(rgb, w, h, sigma, threads)
}

/// Post processing > Noise Filter > "Filter now": the DCT noise filter
/// ([`crate::denoise`]) on the shown picture, which is also the saved one.
pub fn denoise_picture(app: &mut Mb3d, ui: &mut Ui, sigma: f32) {
    let Some((rgb, w, h)) = app.main.rgb.clone() else {
        return app.message(ui, "Calculate the image first.");
    };
    let out = std::sync::Arc::new(denoised(&rgb, w, h, sigma));
    app.main.rgb = Some((out.clone(), w, h));
    set_image(app, ui, out, w, h);
    app.message(ui, &format!("Noise filter: strength {sigma}"));
}

/// Shows a picture of another window in the main window (voxel slices).
pub fn show_external(app: &mut Mb3d, ui: &mut Ui, rgb: std::sync::Arc<Vec<u8>>, w: usize, h: usize) {
    app.main.rgb = Some((rgb.clone(), w, h));
    app.main.image_text = false;
    ui.set_caption(F, "Label6", "");
    set_image(app, ui, rgb, w, h);
}

fn set_image(app: &Mb3d, ui: &mut Ui, rgb: std::sync::Arc<Vec<u8>>, w: usize, h: usize) {
    let s = app.main.image_scale.max(1) as usize;
    if s > 1 {
        let (px, dw, dh) = crate::render::downsample(&rgb, w, h, s);
        show_bitmap(ui, &px, dw, dh);
    } else {
        show_bitmap(ui, &rgb, w, h);
    }
}

/// Shows `px` as it is (the size shown).
fn show_bitmap(ui: &mut Ui, px: &[u8], dw: usize, dh: usize) {
    // (the old picture goes first: two of 15360 x 15360 are 1.9 GB)
    ui.cm(F, "Image1").picture = None;
    let b = Bitmap::from_rgb(dw, dh, px);
    let c = ui.cm(F, "Image1");
    c.width = dw as i32;
    c.height = dh as i32;
    c.picture = Some(b);
}

/// `ClearScreen` + `ParasChanged`.
fn clear_screen(app: &mut Mb3d, ui: &mut Ui) {
    let s = app.main.image_scale.max(1);
    let (w, h) = ((app.scene.width / s).max(1), (app.scene.height / s).max(1));
    let c = ui.cm(F, "Image1");
    c.width = w;
    c.height = h;
    c.picture = None;
    paras_changed(app, ui);
}

/// `ParasChanged`: the hint on the image.
fn paras_changed(app: &mut Mb3d, ui: &mut Ui) {
    app.main.image_text = true;
    ui.set_caption(F, "Label6", "Press 'Calculate 3D' to render");
}

pub fn update_scale(app: &mut Mb3d, ui: &mut Ui, scale: i32) {
    app.main.image_scale = scale.clamp(1, 10);
    ui.set_caption(F, "SpeedButton35", &format!("1:{}", app.main.image_scale));
    match app.main.rgb.clone() {
        Some((rgb, w, h)) => set_image(app, ui, rgb, w, h),
        None => {
            let s = app.main.image_scale;
            let c = ui.cm(F, "Image1");
            c.width = (app.scene.width / s).max(1);
            c.height = (app.scene.height / s).max(1);
        }
    }
}

// ---------------------------------------------------------------------------
// events
// ---------------------------------------------------------------------------

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    match h {
        "Button2Click" => button2_click(app, ui),
        "CalcOnComboChange" => {
            let i = ui.item_index(F, "CalcOnCombo") as i64;
            set_calc_on(app, ui, i);
        }
        "Button1Click" => {
            // the 2D slices at z mid / start / end
            app.main.slice_calc = ui.tag(F, &e.sender) as u8;
            if !app.main.calculating {
                calc_mand(app, ui, false);
            }
        }
        "Button11Click" => {
            // "Calc-": a fast rough image of 8x8 blocks
            app.post = super::postpro_form::State::default();
            app.make_scene(ui);
            if app.scene.width < 1 || app.scene.height < 1 {
                return;
            }
            app.main.calc3d = false;
            app.main.calc_start = Instant::now();
            ui.set_visible(F, "Shape1", false);
            disable_buttons(app, ui);
            app.eng.set_preview_step(app.main.image_scale.max(1) as usize);
            app.eng.start(Job::Blocky { scene: app.scene.clone() });
        }
        "Button12Click" => {
            // stereo images: very left / left / right eye
            app.make_scene(ui);
            store_undo(app);
            app.scene.stereo_mode = ui.tag(F, &e.sender) as u8;
            let sc = app.scene.clone();
            app.main.calc3d = true;
            disable_buttons(app, ui);
            ui.set_visible(F, "ProgressBar1", true);
            start_calc(app, ui, sc, 0);
            app.scene.stereo_mode = 0;
        }
        "StereoPairClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "PNG (*.png)|*.png|JPEG (*.jpg)|*.jpg|Bitmap (*.bmp)|*.bmp".into(),
                default_ext: "png".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
                file_name: format!("{}-stereo", stem(&app.title)),
                ..Default::default()
            };
            ui.save_dialog("main:autostereo", &o);
        }
        "Button2MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, x, y, .. } = e.ev {
                if ui.caption(F, "Button2") == "Calculate 3D" {
                    ui.popup_menu(F, "PopupMenu3", "Button2", x, y);
                }
            }
        }
        "StartrenderingandsaveafterwardstheM3Ifile1Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Image + Parameter (*.m3i)|*.m3i".into(),
                default_ext: "m3i".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3I)),
                ..Default::default()
            };
            ui.save_dialog("main:autom3i", &o);
        }
        "PositionBtnClick" => {
            let v = !ui.visible(F, "PositionPnl");
            ui.set_visible(F, "PositionPnl", v);
        }
        "RotationBtnClick" => {
            let v = !ui.visible(F, "RotationPnl");
            ui.set_visible(F, "RotationPnl", v);
        }
        "SpeedButton30Click" => {
            if ui.caption(F, "SpeedButton30") == "click image" {
                ui.set_caption(F, "SpeedButton30", "get midpoint");
                app.main.get_pos = 0;
                set_image_cursor(app, ui);
            } else {
                ui.set_caption(F, "SpeedButton30", "click image");
                app.main.get_pos = 22;
                ui.cm(F, "Image1").cursor = "crCross".into();
            }
        }
        "Button14Click" => {
            // reset the position
            set(ui, "Edit1", "-2.0");
            set(ui, "Edit17", "0.0");
            set(ui, "Edit3", "30.0");
            set(ui, "Edit9", "0.0");
            set(ui, "Edit10", "0.0");
            set(ui, "Edit5", "0.8");
            set(ui, "Edit14", "30");
            app.scene.vgrads = build_rot_matrix(0.0001, -0.0001, 0.0);
            if parse_float(&ui.text("FormulaGUIForm", "RBailoutEdit")).unwrap_or(0.0) > 500.0 {
                set(ui, "Edit1", "-8.0");
                set(ui, "Edit17", "0.0");
                set(ui, "Edit3", "120.0");
                set(ui, "Edit5", "0.18");
            }
            paras_changed(app, ui);
            set_euler_edits(app, ui);
        }
        "Button7Click" => {
            let x = ptf(&t(ui, "Edit27")) * PID180;
            let y = ptf(&t(ui, "Edit31")) * PID180;
            let z = ptf(&t(ui, "Edit32")) * PID180;
            app.scene.vgrads = build_rot_matrix(x, y, z);
            set_euler_edits(app, ui);
            paras_changed(app, ui);
        }
        "ButtonR0Click" => {
            app.scene.vgrads = build_rot_matrix(0.0, 0.0, 0.0);
            set_euler_edits(app, ui);
            paras_changed(app, ui);
        }
        "SpinEdit2Change" | "SpinEdit2ChangingEx" => all_presets_up(ui),
        "SpeedButton3Click" => speed_button3_click(app, ui, &e.sender),
        "SpeedButton10Click" => {
            let arrow = ui.c(F, "SpeedButton3").cursor == "crUpArrow";
            for b in ["SpeedButton3", "SpeedButton13", "SpeedButton5", "SpeedButton6"] {
                ui.cm(F, b).cursor = if arrow { "crDefault".into() } else { "crUpArrow".into() };
            }
            if arrow {
                ui.cm(F, "SpeedButton10").down = false;
            } else {
                all_presets_up(ui);
            }
        }
        "UpDown1Click" => {
            if let Ev::UpDown { up } = e.ev {
                let s = if up { app.main.image_scale - 1 } else { app.main.image_scale + 1 };
                update_scale(app, ui, s);
            }
        }
        "SpeedButton35Click" => {
            let hgt = ui.c(F, "SpeedButton35").height;
            ui.popup_menu(F, "PopupMenu1", "SpeedButton35", 0, hgt);
        }
        "N111Click" => {
            let s = ui.tag(F, &e.sender) as i32;
            update_scale(app, ui, s);
        }
        "UpDown2Click" => {
            if let Ev::UpDown { up } = e.ev {
                app.scene.tiling = None;
                let (w, h) = (pti(&t(ui, "Edit11")), pti(&t(ui, "Edit12")));
                app.main.user_change = false;
                let (w2, h2) = if up { (w * 2, h * 2) } else { ((w / 2).max(1), (h / 2).max(1)) };
                // with "keep aspect" the height follows the kept ratio, so halving
                // an odd height and doubling it again does not change the shape
                let h2 = if ui.checked(F, "CheckBox10") { ((w2 as f64 / app.main.intern_aspect).round() as i32).max(1) } else { h2 };
                set(ui, "Edit11", &w2.to_string());
                set(ui, "Edit12", &h2.to_string());
                let s = if up { 2.0 } else { 0.5 };
                scale_de_stop(ui, s);
                scale_rclip(ui, s);
                app.scene.width = pti(&t(ui, "Edit11"));
                app.scene.height = pti(&t(ui, "Edit12"));
                app.main.user_change = true;
            }
        }
        "UpDown4Click" => {
            if let Ev::UpDown { up } = e.ev {
                let v = ui.caption(F, "Label23").parse::<i32>().unwrap_or(0);
                let v = if up { (v + 1).min(3) } else { (v - 1).max(0) };
                ui.set_caption(F, "Label23", &v.to_string());
                app.ini.set("ImageSharp", &v.to_string());
            }
        }
        "UpDown5Click" => {
            let i = ui.position(F, "UpDown5");
            ui.set_caption(F, "Label61", &if i > 0 { format!("+{i}") } else { i.to_string() });
        }
        "Edit11Change" => edit11_change(app, ui, &e.sender),
        "CheckBox10Click" => {
            if ui.checked(F, "CheckBox10") {
                let (w, h) = (ptf(&t(ui, "Edit11")), ptf(&t(ui, "Edit12")));
                if w > 0.5 && h > 0.5 {
                    app.main.intern_aspect = w / h;
                }
            }
        }
        "SpeedButton19Click" => {
            let tg = ui.tag(F, &e.sender);
            let wid = pti(&t(ui, "Edit11"));
            app.main.user_change = false;
            let (ux, uy) = app.main.user_aspect;
            match tg {
                1 => set(ui, "Edit12", &(wid * 3 / 4).to_string()),
                2 => set(ui, "Edit12", &(wid * 3 / 5).to_string()),
                _ => {
                    if ux > 0 {
                        set(ui, "Edit12", &(wid * uy / ux).to_string())
                    }
                }
            }
            app.scene.height = pti(&t(ui, "Edit12")).max(1);
            app.scene.tiling = None;
            let (w, h) = (ptf(&t(ui, "Edit11")), ptf(&t(ui, "Edit12")));
            if w > 0.5 && h > 0.5 {
                app.main.intern_aspect = w / h;
            }
            app.main.user_change = true;
        }
        "SpeedButton21MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, .. } = e.ev {
                ui.input_query("main:aspectx", "User defined aspect ratio", "Input the width factor:", "16");
            }
        }
        "SpeedButton18MouseUp" => {
            if let Ev::MouseUp { button, .. } = e.ev {
                rotate_buttons(app, ui, ui.tag(F, &e.sender), button);
            }
        }
        "SpeedButton1Click" => set_image_cursor(app, ui),
        "Image1MouseDown" => image_mouse_down(app, ui, &e.ev),
        "Image1MouseMove" => image_mouse_move(app, ui, &e.ev),
        "Image1MouseUp" => image_mouse_up(app, ui, &e.ev),
        "Shape1MouseDown" => ui.set_visible(F, "Shape1", false),
        // the brush of "Paint on image" is over the image: its events are
        // the image's (a right click chooses its shape)
        "Shape2MouseDown" | "Shape2MouseMove" => {
            let img = ui.c(F, "Image1").rect();
            let b = ui.c(F, "Shape2").rect();
            let (dx, dy) = (b.x - img.x, b.y - img.y);
            match e.ev {
                Ev::MouseDown { button: MouseButton::Right, x, y, .. } => ui.popup_menu(F, "PopupMenu4", "Shape2", x, y),
                Ev::MouseDown { x, y, button, shift } => image_mouse_down(app, ui, &Ev::MouseDown { x: x + dx, y: y + dy, button, shift }),
                Ev::MouseMove { x, y, shift } => image_mouse_move(app, ui, &Ev::MouseMove { x: x + dx, y: y + dy, shift }),
                _ => {}
            }
        }
        "ShapeDisc1Click" => ui.cm(F, "Shape2").shape = "stCircle".into(),
        "ShapeBox1Click" => ui.cm(F, "Shape2").shape = "stRectangle".into(),
        "FormMouseWheel" if ui.visible(F, "Shape2") => {
            if let Ev::Wheel { delta, .. } = e.ev {
                let w = ui.c(F, "Shape2").width;
                set_brush_size(ui, if delta < 0 { w - 2 } else { w + 2 });
            }
        }
        "FormMouseWheel" => {
            if let Ev::Wheel { delta, x, y, .. } = e.ev {
                let r = ui.ctl_rect(F, "ScrollBox1");
                if r.contains(x, y) {
                    let s = if delta < 0 { app.main.image_scale + 1 } else { app.main.image_scale - 1 };
                    update_scale(app, ui, s);
                }
            }
        }
        "Button16Click" => {
            let tg = ui.tag(F, &e.sender) as i32;
            if ui.caption(F, &e.sender) == "Click on image" {
                let c = if tg == 4 { "Get min.dist. from image" } else { "Get values from image" };
                ui.set_caption(F, &e.sender, c);
                app.main.get_pos = 0;
                set_image_cursor(app, ui);
            } else {
                ui.set_caption(F, &e.sender, "Click on image");
                app.main.get_pos = tg;
                ui.cm(F, "Image1").cursor = "crCross".into();
            }
        }
        "Button13Click" => {
            let v = rotate_4d(ui, [ptf(&t(ui, "Edit9")), ptf(&t(ui, "Edit10")), ptf(&t(ui, "Edit17")), 0.0]);
            set(ui, "Edit28", &fts(v[0]));
            set(ui, "Edit29", &fts(v[1]));
            set(ui, "Edit30", &fts(v[2]));
            set(ui, "Edit7", &fts(v[3]));
        }
        "Button19Click" => {
            let (a, b, c) = (t(ui, "Edit9"), t(ui, "Edit10"), t(ui, "Edit17"));
            set(ui, "Edit23", &a);
            set(ui, "Edit24", &b);
            set(ui, "Edit22", &c);
        }
        "CheckBox7Click" | "PageControl1Change" => page_control1_change(app, ui),
        "ButtonVolLightClick" => {
            if ui.caption(F, "ButtonVolLight") == "Dyn. fog on It.:" {
                app.scene.vol_light = Some(crate::vollight::VolLightParams { light: 0, map_size: 0 });
            } else {
                app.scene.vol_light = None;
            }
            set_edit16(app, ui);
        }
        "ButtonAuthorClick" => {
            let a = app.ini.get("Author").to_string();
            ui.input_query("main:author", "Author", "Your author name (stored in the parameters):", &a);
        }
        "ButtonInsertAuthorClick" => {
            let a = app.ini.get("Author").to_string();
            if !a.is_empty() {
                if app.main.authors[0].is_empty() || app.main.authors[0] == a {
                    set(ui, "Edit33", &a);
                    app.main.authors[0] = a;
                } else {
                    set(ui, "Edit34", &a);
                    app.main.authors[1] = a;
                }
            }
        }
        "Edit33Change" => app.main.authors[0] = t(ui, "Edit33"),
        "Edit34Change" => app.main.authors[1] = t(ui, "Edit34"),
        "SpeedButton9MouseUp" => {
            if let Ev::MouseUp { button, .. } = e.ev {
                let n = app.main.undo.len();
                match button {
                    MouseButton::Left if app.main.undo_pos > 1 => app.main.undo_pos -= 1,
                    MouseButton::Right if app.main.undo_pos < n => app.main.undo_pos += 1,
                    _ => return,
                }
                if let Some(s) = app.main.undo.get(app.main.undo_pos.saturating_sub(1)).cloned() {
                    app.scene = s;
                    app.scene_to_forms(ui);
                    paras_changed(app, ui);
                    all_presets_up(ui);
                }
            }
        }
        // ---- windows
        "Button10Click" => ui.show("FormulaGUIForm"),
        "Button18Click" => ui.show("LightAdjustForm"),
        "Button15Click" => ui.show("PostProForm"),
        "SpeedButton12Click" => ui.show("AnimationForm"),
        "SpeedButton15Click" => ui.show("FNavigator"),
        "MeshExportBtnClick" => ui.show("BulbTracer2Frm"),
        "MutaGenBtnClick" => {
            super::mutagen_form::opening(app, ui);
            ui.show("MutaGenFrm");
        }
        "ZBufferGenBtnClick" => ui.show("ZBuf16BitGenFrm"),
        "HeightMapGenBtnClick" => ui.show("HeightMapGenFrm"),
        "SpeedButton25Click" => ui.show("BatchForm1"),
        "SpeedButton24Click" => ui.show("FVoxelExport"),
        "SpeedButton27Click" => ui.show("TilingForm"),
        "SpeedButton28Click" => ui.show("MCForm"),
        "IniDirsBtnClick" => ui.show("IniDirForm"),
        "MapSequencesBtnClick" => ui.show("MapSequencesFrm"),
        "VisualThemesBtnClick" => ui.show("VisualThemesFrm"),
        "SwarmBtnClick" => ui.show("SwarmFrm"),
        "ResumeBtnClick" => ui.show("UnfinishedFrm"),
        // ---- files
        "Button5Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Parameter (*.m3p)|*.m3p|All files (*.*)|*.*".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3P)),
                preview: true,
                ..Default::default()
            };
            ui.open_dialog("main:openm3p", &o);
        }
        "Button9Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Image + Parameter (*.m3i)|*.m3i".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3I)),
                preview: true,
                ..Default::default()
            };
            ui.open_dialog("main:openm3i", &o);
        }
        "Button4Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Parameter (*.m3p)|*.m3p".into(),
                default_ext: "m3p".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3P)),
                file_name: stem(&app.title),
                ..Default::default()
            };
            ui.save_dialog("main:savem3p", &o);
        }
        "Button8Click" => {
            if app.eng.base().is_none() {
                app.message(ui, "Calculate the image first.");
                return;
            }
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Image + Parameter (*.m3i)|*.m3i".into(),
                default_ext: "m3i".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3I)),
                file_name: stem(&app.title),
                ..Default::default()
            };
            ui.save_dialog("main:savem3i", &o);
        }
        "SpeedButton7Click" => {
            app.make_scene(ui);
            let raw = crate::m3p::write(&app.scene);
            let name = stem(&app.title);
            crate::vcl::clipboard::set(&crate::m3p::raw_to_text(&raw, if name.is_empty() { "Mandelbulb 3D" } else { &name }));
            app.message(ui, "Parameters copied to the clipboard.");
        }
        "SpeedButton8Click" => {
            let text = crate::vcl::clipboard::get().unwrap_or_default();
            match crate::m3p::raw_from_text(&text).and_then(|(raw, title)| crate::m3p::parse(&raw).map(|m| (m, title))) {
                Ok((m, title)) => {
                    app.scene = m.scene;
                    app.title = title;
                    app.eng.clear();
                    text_pars_load_success(app, ui);
                    for n in m.warnings {
                        app.message(ui, &n);
                    }
                }
                Err(_) => {
                    if ui.has_form("FTextBox") {
                        ui.show("FTextBox");
                    } else {
                        app.message(ui, "No text parameters found in the clipboard.");
                    }
                }
            }
        }
        "Button3Click" => save_pic_dialog(app, ui, "png"),
        "SBsaveJPEGClick" => save_pic_dialog(app, ui, "jpg"),
        "SpeedButton29Click" => save_pic_dialog(app, ui, "jpgp"),
        "SpeedButton26Click" => save_pic_dialog(app, ui, "zbuf"),
        "FrameUpDownClick" => {
            if let Ev::UpDown { up } = e.ev {
                let f = pti(&t(ui, "FrameEdit"));
                let f = if up { f + 1 } else { (f - 1).max(1) };
                set(ui, "FrameEdit", &f.to_string());
                crate::maps::set_current_frame(f);
            }
        }
        "FrameEditExit" => crate::maps::set_current_frame(pti(&t(ui, "FrameEdit")).max(1)),
        "FormKeyPress" => {}
        "WMMove" => place_sticky(app, ui, None, false),
        // a right click on Formulas, Lighting, Postprocess: stick its window
        // to the right or left side of this one, or not
        "Button10MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, x, y, .. } = e.ev {
                let k = ui.tag(F, &e.sender).clamp(0, 2) as usize;
                app.main.stick_it = k;
                let v = app.main.sticky[k];
                ui.set_checked(F, "Stickthiswindowtotherightside1", v == 1);
                ui.set_checked(F, "Stickthiswindowtotheleftside1", v == 2);
                ui.set_checked(F, "Donotmakethiswindowsticky1", v == 0);
                ui.popup_menu(F, "PopupMenu2", &e.sender, x, y);
            }
        }
        "Stickthiswindowtotherightside1Click" => {
            let k = app.main.stick_it;
            app.main.sticky[k] = ui.tag(F, &e.sender).clamp(0, 2) as u8;
            store_sticky(app);
            place_sticky(app, ui, Some(STICKY[k].0), false);
        }
        "FormResize" => {
            place_sticky(app, ui, None, false);
            let ph = ui.c(F, "Panel1").height;
            let mt = ui.c(F, "Memo1").top;
            let p4 = ui.c(F, "Panel4").height;
            ui.cm(F, "Memo1").height = (ph - mt - p4 - 1).clamp(80, 240);
        }
        "FormCloseQuery" => {
            if app.eng.running() && app.main.calc_start.elapsed().as_secs() > 900 {
                ui.message_box("main:closequery", "Warning", "Do you really want to stop the calculations?", &[MR_YES, crate::vcl::form::MR_NO]);
            } else {
                app.eng.stop();
                ui.close_ok(F);
            }
        }
        "FormClose" => form_close(app, ui),
        _ => {}
    }
}

fn stem(title: &str) -> String {
    std::path::Path::new(title).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn form_close(app: &mut Mb3d, ui: &mut Ui) {
    app.eng.stop();
    super::tools_forms::stop_tiling(app);
    super::mc_form::stop_for_exit(app);
    super::voxel_form::stop(app);
    // a swarm worker gives its unit up; another computer takes it
    super::swarm_form::stop_worker(app, true);
    app.make_scene(ui);
    crate::appdirs::store_history(&app.scene, &stem(&app.title));
    let f = ui.f(F);
    let (w, h) = f.outer_size(&ui.theme);
    let (l, tp) = (f.left, f.top);
    app.ini.set("m3dPos", &format!("{l} {tp}"));
    app.ini.set("m3dSize", &format!("{} {}", w + 16 - 2 * f.frame_w(), h + 38 - 2 * f.frame_w() - f.caption_h(&ui.theme)));
    app.ini.set("ScaleDEstop", if ui.checked(F, "CheckBox11") { "1" } else { "0" });
    app.ini.set("ThreadPriority", &if ui.checked(F, "CheckBox15") { ui.item_index(F, "ComboBox2").to_string() } else { "-1".into() });
    app.ini.set("ThreadCount", &if ui.checked(F, "CheckBox12") { "Auto".into() } else { ui.position(F, "UpDown3").to_string() });
    app.ini.set("MandRotDeg", &t(ui, "Edit4"));
    app.ini.set("SavePNGtextPars", if ui.checked(F, "CheckBox13") { "Yes" } else { "No" });
    app.ini.set("DisableTBoost", if ui.checked(F, "CheckBox14") { "Yes" } else { "No" });
    app.ini.set("SaveImgInM3I", if ui.checked(F, "CheckBox16") { "Yes" } else { "No" });
    app.ini.set("VisualTheme", ui.theme.style.name());
    let _ = app.ini.save();
}

fn button2_click(app: &mut Mb3d, ui: &mut Ui) {
    crate::maps::set_current_frame(pti(&t(ui, "FrameEdit")).max(1));
    if ui.caption(F, "Button2") == "Stop" {
        let anim = app.anim.job.lock().unwrap().running;
        app.anim.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        app.eng.stop();
        super::voxel_form::stop(app);
        super::tools_forms::stop_tiling(app);
        enable_buttons(app, ui);
        if app.main.led_running {
            app.main.led_running = false;
            show_led(ui, app.main.led, false);
        }
        if anim {
            ui.set_caption("AnimationForm", "Button2", "Start rendering animation images");
        }
        app.message(ui, if anim { "Animation rendering stopped." } else { "Calculation stopped." });
        return;
    }
    app.main.image_text = false;
    app.scene.stereo_mode = 0;
    app.make_scene(ui);
    let title = stem(&app.title);
    crate::appdirs::store_history(&app.scene, &title);
    calc_mand(app, ui, true);
}

fn speed_button3_click(app: &mut Mb3d, ui: &mut Ui, sender: &str) {
    let tg = ui.tag(F, sender).clamp(0, 3) as usize;
    if ui.c(F, sender).cursor == "crUpArrow" {
        // store the current values as this preset
        for b in ["SpeedButton3", "SpeedButton13", "SpeedButton5", "SpeedButton6"] {
            ui.cm(F, b).cursor = "crDefault".into();
        }
        ui.cm(F, "SpeedButton10").down = false;
        app.main.presets[tg] = Preset {
            smooth_normals: ui.position(F, "SpinEdit2"),
            de_stop: ptf(&t(ui, "Edit25")),
            ray_multiplier: ptf(&t(ui, "Edit6")),
            bin_search: ui.position(F, "SpinEdit5"),
            image_width: pti(&t(ui, "Edit11")),
            image_scale: app.main.image_scale,
            ray_limiter: ptf(&t(ui, "Edit8")),
        };
        save_acc_preset(app, tg);
        return;
    }
    if !ui.c(F, sender).down {
        return;
    }
    // SetPreset
    let p = app.main.presets[tg];
    ui.set_position(F, "SpinEdit2", p.smooth_normals);
    set(ui, "Edit19", &p.smooth_normals.to_string());
    set(ui, "Edit25", &fts_single(p.de_stop));
    set(ui, "Edit6", &fts_single(p.ray_multiplier));
    set(ui, "Edit8", &fts_single(p.ray_limiter));
    ui.set_position(F, "SpinEdit5", p.bin_search);
    set(ui, "Edit20", &p.bin_search.to_string());
    let w = pti(&t(ui, "Edit11")).max(1);
    app.main.user_change = false;
    let h = pti(&t(ui, "Edit12"));
    set(ui, "Edit11", &p.image_width.to_string());
    set(ui, "Edit12", &((h as i64 * p.image_width as i64) / w as i64).to_string());
    app.main.user_change = true;
    update_scale(app, ui, p.image_scale.clamp(1, 5));
}

fn edit11_change(app: &mut Mb3d, ui: &mut Ui, sender: &str) {
    if !app.main.user_change {
        return;
    }
    all_presets_up(ui);
    let tg = ui.tag(F, sender);
    if ui.checked(F, "CheckBox10") && tg > 0 {
        let old_w = app.scene.width.max(1) as f64;
        if tg == 1 {
            let i = pti(&t(ui, "Edit11"));
            if i > 0 {
                let s = i as f64 / old_w;
                app.main.user_change = false;
                set(ui, "Edit12", &((i as f64 / app.main.intern_aspect).round() as i32).to_string());
                scale_de_stop(ui, s);
                scale_rclip(ui, s);
                app.main.user_change = true;
            }
        } else {
            let i = pti(&t(ui, "Edit12"));
            if i > 0 {
                app.main.user_change = false;
                let w = (i as f64 * app.main.intern_aspect).round() as i32;
                set(ui, "Edit11", &w.to_string());
                let s = w as f64 / old_w;
                scale_de_stop(ui, s);
                scale_rclip(ui, s);
                app.main.user_change = true;
            }
        }
    }
    let (w, h) = (pti(&t(ui, "Edit11")), pti(&t(ui, "Edit12")));
    if w > 0 {
        app.scene.width = w;
    }
    if h > 0 {
        app.scene.height = h;
    }
    app.scene.tiling = None;
}

/// The rotation buttons: left click turns around the viewer's axes, right
/// click around the object's axes.
fn rotate_buttons(app: &mut Mb3d, ui: &mut Ui, tag: i64, button: MouseButton) {
    app.make_scene(ui);
    let rot = ptf(&t(ui, "Edit4")) * -PID180;
    let m = match tag {
        11 => build_rot_matrix(rot, 0.0, 0.0),
        12 => build_rot_matrix(-rot, 0.0, 0.0),
        13 => build_rot_matrix(0.0, -rot, 0.0),
        14 => build_rot_matrix(0.0, rot, 0.0),
        15 => build_rot_matrix(0.0, 0.0, rot),
        _ => build_rot_matrix(0.0, 0.0, -rot),
    };
    let sc = &mut app.scene;
    if button == MouseButton::Left {
        sc.vgrads = mat_mul(&m, &sc.vgrads);
        set_euler_edits(app, ui);
    } else {
        sc.vgrads = mat_mul(&sc.vgrads, &m);
        sc.mid = rotate_vector_reverse(&sc.mid, &m);
        set_edits_from_header(app, ui);
    }
    app.main.slice_calc = 2;
    calc_mand(app, ui, false);
}

fn rotate_4d(ui: &Ui, v: [f64; 4]) -> [f64; 4] {
    let x = ptf(&ui.text("FormulaGUIForm", "XWEdit")) * PID180;
    let y = ptf(&ui.text("FormulaGUIForm", "YWEdit")) * PID180;
    let z = ptf(&ui.text("FormulaGUIForm", "ZWEdit")) * PID180;
    if x == 0.0 && y == 0.0 && z == 0.0 {
        return v;
    }
    let m = crate::math::build_smatrix4(x, y, z);
    crate::math::rotate_4dex(&[v[0], v[1], v[2]], &m)
}

// ---------------------------------------------------------------------------
// mouse on the image
// ---------------------------------------------------------------------------

fn image_mouse_down(app: &mut Mb3d, ui: &mut Ui, ev: &Ev) {
    let Ev::MouseDown { x, y, button, shift } = *ev else { return };
    if ui.c(F, "Image1").cursor == "crHourGlass" {
        return;
    }
    app.main.mouse_start = (x, y);
    app.main.max_shape_w = 0;
    app.main.z_translate = 0;
    let cursor = ui.c(F, "Image1").cursor.clone();
    let img = ui.c(F, "Image1").rect();
    let s = app.main.image_scale;
    if cursor == "crHandPoint" {
        ui.cm(F, "Shape1").set_bounds(img.x, img.y, img.w, img.h);
        if shift & SS_LEFT != 0 {
            ui.set_visible(F, "Shape1", true);
        }
    } else if cursor == "crCross" && button == MouseButton::Left && app.main.get_pos > 0 && !selecting(ui) {
        let gp = app.main.get_pos;
        let pick = app.eng.pick(x * s, y * s);
        match gp {
            1 | 10 => {
                if let Some((_, z)) = pick {
                    if ui.has_form("PostProForm") {
                        if gp == 1 {
                            ui.set_text("PostProForm", "Edit1", &fts_single(z));
                            ui.set_caption("PostProForm", "Button2", "Get Z1");
                        } else {
                            ui.set_text("PostProForm", "Edit10", &fts_single(z));
                            ui.set_caption("PostProForm", "Button18", "Get Z2");
                        }
                    }
                }
            }
            2 => {
                if let Some((p, _)) = pick {
                    super::light_form::set_pos_light_from_image(app, ui, p);
                }
            }
            3 | 6 | 22 => {
                if let Some((p, _)) = pick {
                    match gp {
                        3 => {
                            let v = rotate_4d(ui, [p[0], p[1], p[2], 0.0]);
                            set(ui, "Edit28", &fts(v[0]));
                            set(ui, "Edit29", &fts(v[1]));
                            set(ui, "Edit30", &fts(v[2]));
                            set(ui, "Edit7", &fts(v[3]));
                            ui.set_checked(F, "CheckBox7", true);
                            ui.set_caption(F, "Button16", "Get values from image");
                        }
                        22 => {
                            let (zs, ze) = (app.scene.mid[2] - app.scene.z_start, app.scene.z_end - app.scene.mid[2]);
                            set(ui, "Edit1", &fts(p[2] - zs));
                            set(ui, "Edit17", &fts(p[2]));
                            set(ui, "Edit3", &fts(p[2] + ze));
                            set(ui, "Edit9", &fts(p[0]));
                            set(ui, "Edit10", &fts(p[1]));
                            ui.set_caption(F, "SpeedButton30", "get midpoint");
                        }
                        _ => {
                            set(ui, "Edit23", &fts(p[0]));
                            set(ui, "Edit24", &fts(p[1]));
                            set(ui, "Edit22", &fts(p[2]));
                            ui.set_checked(F, "CheckBox4", true);
                            ui.set_checked(F, "CheckBox5", true);
                            ui.set_checked(F, "CheckBox6", true);
                            ui.set_caption(F, "Button20", "Get values from image");
                        }
                    }
                    page_control1_change(app, ui);
                    paras_changed(app, ui);
                } else {
                    app.message(ui, "No object at this point.");
                }
            }
            _ => {
                ui.set_caption(F, "Button17", "Get min.dist. from image");
            }
        }
        app.main.get_pos = 0;
        app.main.pos_picked = true;
        set_image_cursor(app, ui);
    } else if cursor == "crCross" && button == MouseButton::Left && selecting(ui) {
        // selection for "recalculate a selection"
        ui.cm(F, "Shape1").set_bounds(img.x + x, img.y + y, 0, 0);
        ui.set_visible(F, "Shape1", true);
    } else if cursor == "crCross" && button == MouseButton::Left && ui.visible(F, "Shape2") {
        paint_on_image(app, ui, x, y);
    }
}

fn image_mouse_move(app: &mut Mb3d, ui: &mut Ui, ev: &Ev) {
    let Ev::MouseMove { x, y, shift } = *ev else { return };
    if app.main.pos_picked {
        return;
    }
    let cursor = ui.c(F, "Image1").cursor.clone();
    let img = ui.c(F, "Image1").rect();
    let (sx, sy) = app.main.mouse_start;
    if ui.visible(F, "Shape2") && !not_all_buttons_up(ui) {
        // the brush follows the mouse and paints while the button is down
        let half = ui.c(F, "Shape2").width / 2;
        ui.cm(F, "Shape2").set_bounds(x - half + img.x, y - half + img.y, half * 2 + 1, half * 2 + 1);
        if shift & SS_LEFT != 0 && cursor != "crHourGlass" {
            paint_on_image(app, ui, x, y);
        }
        return;
    }
    if shift & SS_LEFT != 0 && cursor != "crHourGlass" && cursor != "crCross" && not_all_buttons_up(ui) {
        if cursor == "crHandPoint" {
            ui.cm(F, "Shape1").set_bounds(x - sx + img.x, y - sy + img.y, img.w, img.h);
        } else if ui.checked(F, "SpeedButton4") {
            app.main.z_translate = sy - y;
            ui.set_caption(F, "Label20", &(-app.main.z_translate).to_string());
        } else {
            let ih = ((sx - x).abs() * img.h) / img.w.max(1);
            let iy = if y > sy { 0 } else { ih };
            ui.cm(F, "Shape1").set_bounds(sx.min(x) + img.x, sy - iy + img.y, (sx - x).abs() + 1, ih);
            app.main.max_shape_w = app.main.max_shape_w.max((sx - x).abs() + 1);
        }
        if !ui.checked(F, "SpeedButton4") {
            ui.set_visible(F, "Shape1", true);
        }
    } else if cursor == "crCross" && shift & SS_LEFT != 0 && ui.visible(F, "Shape1") {
        ui.cm(F, "Shape1").set_bounds(sx.min(x) + img.x, sy.min(y) + img.y, (x - sx).abs() + 1, (y - sy).abs() + 1);
    }
}

fn image_mouse_up(app: &mut Mb3d, ui: &mut Ui, ev: &Ev) {
    let Ev::MouseUp { x, y, button, .. } = *ev else { return };
    if std::mem::take(&mut app.main.pos_picked) {
        return;
    }
    let cursor = ui.c(F, "Image1").cursor.clone();
    if cursor == "crHourGlass" || cursor == "crCross" || !not_all_buttons_up(ui) {
        return;
    }
    let img = ui.c(F, "Image1").rect();
    let sh = ui.c(F, "Shape1").rect();
    if sh.w < 8 {
        ui.set_visible(F, "Shape1", false);
    }
    let shape_vis = ui.visible(F, "Shape1");
    let (il, it) = (sh.x - img.x, sh.y - img.y);
    let (xh, yh) = (img.w as f64 * 0.5, img.h as f64 * 0.5);
    let (mut xx, mut yy, mut dz) = (0.0, 0.0, 1.0);
    let mut update = false;
    if cursor == "crHandPoint" {
        if button == MouseButton::Left {
            xx = -il as f64;
            yy = -it as f64;
            update = true;
        }
    } else if shape_vis {
        xx = il as f64 + sh.w as f64 * 0.5 - xh;
        yy = it as f64 + sh.h as f64 * 0.5 - yh;
        update = true;
    } else if app.main.max_shape_w < 8 {
        xx = x as f64 - xh;
        yy = y as f64 - yh;
        update = true;
    }
    if update {
        if ui.checked(F, "SpeedButton4") {
            ui.set_caption(F, "Label20", "");
            xx = 0.0;
            yy = 0.0;
            update = app.main.z_translate != 0;
        } else {
            app.main.z_translate = 0;
            dz = if shape_vis {
                img.w as f64 / sh.w.max(1) as f64
            } else if button == MouseButton::Left {
                1.4
            } else {
                1.0 / 1.4
            };
        }
    }
    if update {
        app.make_scene(ui);
        let s = app.main.image_scale as f64;
        let (xx, yy, zt) = (xx * s, yy * s, app.main.z_translate as f64 * s);
        let sc = &mut app.scene;
        let mut yh2 = 2.1345 / (sc.zoom * sc.width as f64);
        yh2 *= 1.0 + (sc.fov_y * PID180).sin() * (sc.mid[2] - sc.z_start) / (yh2 * sc.height as f64);
        let m = normalise_matrix_to(yh2, &sc.vgrads);
        sc.mid[0] += xx * m[0][0] + yy * m[1][0] - zt * m[2][0];
        sc.mid[1] += xx * m[0][1] + yy * m[1][1] - zt * m[2][1];
        let d = xx * m[0][2] + yy * m[1][2] - zt * m[2][2];
        if (dz - 1.0f64).abs() > 1e-4 {
            sc.zoom *= dz;
            sc.z_start = sc.mid[2] + (sc.z_start - sc.mid[2]) / dz;
            sc.z_end = sc.mid[2] + (sc.z_end - sc.mid[2]) / dz;
        }
        sc.z_end += d;
        sc.mid[2] += d;
        sc.z_start += d;
        let sc = app.scene.clone();
        set(ui, "Edit5", &fts(sc.zoom));
        set(ui, "Edit1", &fts(sc.z_start));
        set(ui, "Edit3", &fts(sc.z_end));
        set(ui, "Edit9", &fts(sc.mid[0]));
        set(ui, "Edit10", &fts(sc.mid[1]));
        set(ui, "Edit17", &fts(sc.mid[2]));
        app.main.slice_calc = 2;
        calc_mand(app, ui, false);
    }
    ui.set_visible(F, "Shape1", false);
}

// ---------------------------------------------------------------------------
// files
// ---------------------------------------------------------------------------

fn text_pars_load_success(app: &mut Mb3d, ui: &mut Ui) {
    app.main.intern_aspect = app.scene.width as f64 / app.scene.height.max(1) as f64;
    app.scene_to_forms(ui);
    all_presets_up(ui);
    app.message(ui, "Parameters loaded, press \"Calculate 3D\" to render.");
    show_image(app, ui);
    set_caption(app, ui);
}

fn save_pic_dialog(app: &mut Mb3d, ui: &mut Ui, what: &str) {
    if what != "jpgp" && what != "zbuf" && app.main.rgb.is_none() {
        app.message(ui, "Calculate the image first.");
        return;
    }
    let (filter, ext) = match what {
        "png" => ("PNG (*.png)|*.png|PNG 16 bit colour (*.png)|*.png|PNG transparent background (*.png)|*.png|Bitmap (*.bmp)|*.bmp", "png"),
        "jpg" => ("JPEG (*.jpg)|*.jpg", "jpg"),
        "jpgp" => ("JPEG + M3P (*.jpg)|*.jpg|PNG + M3P (*.png)|*.png", "jpg"),
        _ => ("PNG 16 bit (*.png)|*.png", "png"),
    };
    let o = crate::vcl::dialogs::FileOptions {
        filter: filter.into(),
        default_ext: ext.into(),
        initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
        file_name: stem(&app.title),
        ..Default::default()
    };
    ui.save_dialog(&format!("main:save{what}"), &o);
}

/// The image as saved: reduced by the viewing scale (MB3D saves Image1).
fn saved_image(app: &Mb3d) -> Option<(Vec<u8>, usize, usize)> {
    let (rgb, w, h) = app.main.rgb.clone()?;
    let s = app.main.image_scale.max(1) as usize;
    Some(if s > 1 { crate::render::downsample(&rgb, w, h, s) } else { ((*rgb).clone(), w, h) })
}

fn save_image(app: &mut Mb3d, ui: &mut Ui, path: &std::path::Path) {
    let Some((rgb, w, h)) = saved_image(app) else { return };
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let res: Result<(), String> = match ext.as_str() {
        "jpg" | "jpeg" => {
            let q = pti(&t(ui, "Edit26")).clamp(1, 100) as u8;
            std::fs::write(path, crate::jpeg::encode(w, h, &rgb, q)).map_err(|e| e.to_string())
        }
        "bmp" => std::fs::write(path, crate::frames::encode_bmp(w, h, &rgb)).map_err(|e| e.to_string()),
        _ => {
            // "png par": the text parameters as the PNG comment (SavePNG)
            let mut data = crate::png::encode_rgb(w, h, &rgb);
            if ui.checked(F, "CheckBox13") {
                let sc = app.eng.base().map(|b| b.scene.clone()).unwrap_or_else(|| app.scene.clone());
                let text = crate::m3p::raw_to_text(&crate::m3p::write(&sc), ui.f(F).caption());
                data = crate::png::with_text(data, "Comment", &text);
            }
            std::fs::write(path, data).map_err(|e| e.to_string())
        }
    };
    match res {
        Ok(()) => app.message(ui, &format!("Saved {}", path.display())),
        Err(e) => app.message(ui, &format!("{}: {e}", path.display())),
    }
}

/// "PNG 16 bit colour": the picture painted again from the kept G-buffer
/// with the colours as calculated (before the rounding to 8 bit), reduced
/// by the viewing scale as the 8 bit picture.
fn save_image16(app: &mut Mb3d, ui: &mut Ui, path: &std::path::Path) {
    let Some(b) = app.eng.base() else {
        app.message(ui, "Calculate the image first.");
        return;
    };
    let rgb = crate::render::paint16(&b.scene, &b.params, &b.post);
    let s = app.main.image_scale.max(1) as usize;
    let (rgb, w, h) = if s > 1 { crate::render::downsample(&rgb, b.w, b.h, s) } else { (rgb, b.w, b.h) };
    let mut data = crate::png::encode_rgb16(w, h, &rgb);
    if ui.checked(F, "CheckBox13") {
        let text = crate::m3p::raw_to_text(&crate::m3p::write(&b.scene), ui.f(F).caption());
        data = crate::png::with_text(data, "Comment", &text);
    }
    match std::fs::write(path, data) {
        Ok(()) => app.message(ui, &format!("Saved {} (16 bit colour)", path.display())),
        Err(e) => app.message(ui, &format!("{}: {e}", path.display())),
    }
}

/// "PNG transparent background": the shown picture as RGBA, the background
/// transparent ([`crate::frames::rgba_of`]), reduced by the viewing scale.
fn save_transparent(app: &mut Mb3d, ui: &mut Ui, path: &std::path::Path) {
    let (Some((rgb, w, h)), Some(b)) = (app.main.rgb.clone(), app.eng.base()) else {
        app.message(ui, "Calculate the image first.");
        return;
    };
    if b.post.len() != w * h {
        return app.message(ui, "The picture does not match its calculation: calculate it again.");
    }
    let s = app.main.image_scale.max(1) as usize;
    let (rgba, ow, oh) = crate::frames::rgba_of(&rgb, &b.post, w, h, s);
    let mut data = crate::png::encode_rgba(ow, oh, &rgba);
    if ui.checked(F, "CheckBox13") {
        let text = crate::m3p::raw_to_text(&crate::m3p::write(&b.scene), ui.f(F).caption());
        data = crate::png::with_text(data, "Comment", &text);
    }
    match std::fs::write(path, data) {
        Ok(()) => app.message(ui, &format!("Saved {} (transparent background)", path.display())),
        Err(e) => app.message(ui, &format!("{}: {e}", path.display())),
    }
}

fn save_zbuf(app: &mut Mb3d, ui: &mut Ui, path: &std::path::Path) {
    let Some(b) = app.eng.base() else {
        app.message(ui, "Calculate the image first.");
        return;
    };
    let z: Vec<u16> = b.post.iter().map(|s| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 }).collect();
    match crate::png::write_gray16(&path.to_string_lossy(), b.w, b.h, &z) {
        Ok(()) => app.message(ui, &format!("Saved {}", path.display())),
        Err(e) => app.message(ui, &format!("{}: {e}", path.display())),
    }
}

pub fn save_m3i(app: &mut Mb3d, ui: &mut Ui, path: &std::path::Path) {
    let Some(b) = app.eng.base() else { return };
    let data = crate::m3p::write_m3i(&b.scene, &b.post);
    match std::fs::write(path, data) {
        Ok(()) => app.message(ui, &format!("Saved {}", path.display())),
        Err(e) => app.message(ui, &format!("{}: {e}", path.display())),
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("resume", DialogResult::Button(b)) => {
            let Some((sc, slice)) = app.main.resume_ask.take() else { return };
            match *b {
                MR_YES => run_calc(app, sc, slice, crate::checkpoint::Policy::Continue),
                MR_NO => run_calc(app, sc, slice, crate::checkpoint::Policy::Fresh),
                _ => not_started(app, ui),
            }
        }
        (_, DialogResult::File(Some(p))) if what.starts_with("open") || what == "" => {
            let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            if let Some(d) = p.parent() {
                let k = if ext == "m3i" { super::ini::DIR_M3I } else { super::ini::DIR_M3P };
                app.ini.dirs[k] = d.to_path_buf();
            }
            match app.load_params(ui, p) {
                Ok(()) => {
                    all_presets_up(ui);
                    if app.eng.base().is_none() {
                        app.message(ui, "Parameters loaded, press \"Calculate 3D\" to render.");
                    }
                    show_image(app, ui);
                }
                Err(e) => app.message(ui, &e),
            }
        }
        ("savem3p", DialogResult::File(Some(p))) => {
            app.make_scene(ui);
            let p = p.with_extension("m3p");
            match std::fs::write(&p, crate::m3p::write(&app.scene)) {
                Ok(()) => {
                    app.title = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    set_caption(app, ui);
                    app.message(ui, &format!("Saved {}", p.display()));
                }
                Err(e) => app.message(ui, &format!("{}: {e}", p.display())),
            }
        }
        ("savem3i", DialogResult::File(Some(p))) => save_m3i(app, ui, &p.with_extension("m3i")),
        ("autom3i", DialogResult::File(Some(p))) => {
            app.main.auto_m3i = Some(p.with_extension("m3i"));
            button2_click(app, ui);
        }
        ("autostereo", DialogResult::File(Some(p))) => {
            if let Some(d) = p.parent() {
                app.ini.dirs[super::ini::DIR_IMG] = d.to_path_buf();
            }
            app.main.auto_stereo = Some(if p.extension().is_some() { p.to_path_buf() } else { p.with_extension("png") });
            start_stereo_pair(app, ui);
        }
        ("savepng" | "savejpg", DialogResult::File(Some(p))) => {
            if let Some(d) = p.parent() {
                app.ini.dirs[super::ini::DIR_IMG] = d.to_path_buf();
            }
            let png = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"));
            if what == "savepng" && ui.filter_index == 2 && png {
                save_image16(app, ui, p)
            } else if what == "savepng" && ui.filter_index == 3 && png {
                save_transparent(app, ui, p)
            } else {
                save_image(app, ui, p)
            }
        }
        ("savejpgp", DialogResult::File(Some(p))) => {
            app.make_scene(ui);
            let _ = std::fs::write(p.with_extension("m3p"), crate::m3p::write(&app.scene));
            save_image(app, ui, p);
        }
        ("savezbuf", DialogResult::File(Some(p))) => save_zbuf(app, ui, p),
        ("aspectx", DialogResult::Text(Some(s))) => {
            let i = pti(s);
            if i >= 1 {
                app.main.user_aspect.0 = i;
                ui.input_query("main:aspecty", "User defined aspect ratio", "Input the height divisor:", "9");
            }
        }
        ("aspecty", DialogResult::Text(v)) => {
            let i = v.as_deref().map(pti).unwrap_or(0);
            if i < 1 {
                app.main.user_aspect = (0, 0);
            } else {
                app.main.user_aspect.1 = i;
            }
            update_aspect_caption(app, ui);
            app.ini.set("UserAspect", &ui.caption(F, "SpeedButton21"));
        }
        ("author", DialogResult::Text(Some(s))) => {
            app.ini.set("Author", s.trim());
        }
        ("closequery", DialogResult::Button(b)) => {
            if *b == MR_YES {
                app.eng.stop();
                ui.close_ok(F);
            } else {
                ui.close_cancel(F);
            }
        }
        _ => {}
    }
}
