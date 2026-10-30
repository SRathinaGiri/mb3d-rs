//! The Mandelbulb3D desktop application: MB3D's forms (from the original
//! form files) with their event handlers ported from the Delphi units.
//!
//! The parameters live in [`Mb3d::scene`] (MB3D's `MHeader`); every form
//! module has the equivalents of MB3D's `SetEditsFromHeader` (scene ->
//! controls) and `MakeHeader` (controls -> scene).

pub mod ai_form;
pub mod anim_form;
pub mod bulbtracer_form;
pub mod engine;
pub mod event_log;
pub mod eta;
mod eta_model;
pub mod file_preview;
pub mod forms;
pub mod formula_form;
pub mod hotkeys;
pub mod ini;
pub mod jit_form;
pub mod lang_form;
pub mod light_form;
pub mod main_form;
pub mod mc_form;
pub mod meshview;
pub mod mutagen_form;
pub mod navi_form;
pub mod palette_form;
pub mod postpro_form;
pub mod small_forms;
pub mod swarm_form;
pub mod tools_forms;
pub mod unfinished_form;
pub mod history_form;
pub mod log_form;
pub mod util;
pub mod voxel_form;

use crate::scene::Scene;
use crate::vcl::{App, DialogResult, Ev, Event, Style, Ui};
use std::path::PathBuf;

/// The forms MB3D creates at start (form file name, form name).
pub const FORMS: &[&str] = &[
    "Mand", "FormulaGUI", "LightAdjust", "PostProcessForm", "Navigator", "Animation", "AniPreviewWindow", "AniProcess",
    "BatchForm", "BRInfoWindow", "ColorOptionForm", "ColorPick", "FormulaParser", "IniDirsForm", "MapSequencesGUI",
    "MonteCarloForm", "MutaGenGUI", "ParamValueEditGUI", "Swarm", "TextBox", "Tiling", "Unfinished", "History", "Log", "AiConnect", "uMapCalcWindow", "VisualThemesGUI",
    "VoxelExport", "ZBuf16BitGenUI", "BulbTracer2UI", "HeightMapGenUI", "JITFormulaEditGUI", "MeshPreviewUI", "LanguageGUI", "HotkeysGUI",
];

pub const MAIN: &str = "Mand3DForm";

pub struct Mb3d {
    /// the current parameters (`MHeader`)
    pub scene: Scene,
    /// the parameter name shown in the main window's caption
    pub title: String,
    pub eng: engine::Engine,
    pub ini: ini::Ini,
    pub main: main_form::State,
    pub formula: formula_form::State,
    pub light: light_form::State,
    pub post: postpro_form::State,
    pub mapseq: small_forms::MapSeqState,
    pub navi: navi_form::State,
    pub anim: anim_form::State,
    pub palette: palette_form::State,
    pub batch: tools_forms::BatchState,
    pub tiling: tools_forms::TilingState,
    pub mc: mc_form::State,
    pub swarm: swarm_form::State,
    pub unfinished: unfinished_form::State,
    pub history: history_form::State,
    pub logview: log_form::State,
    pub voxel: voxel_form::State,
    pub btracer: bulbtracer_form::State,
    pub mutagen: mutagen_form::State,
    pub meshview: meshview::State,
    pub jit: jit_form::State,
    /// the parameter previews of the open dialogs
    pub preview: file_preview::State,
    /// notes from loading a parameter file (unsupported features)
    pub notes: Vec<String>,
    pub startup_file: Option<PathBuf>,
}

impl Mb3d {
    /// MB3D's `OutMessage`: a line in the message memo of the main window.
    pub fn message(&self, ui: &mut Ui, s: &str) {
        if !s.trim().is_empty() {
            crate::log::info("message", s);
        }
        let c = ui.cm(MAIN, "Memo1");
        let old = c.text.clone();
        let mut lines: Vec<&str> = if old.is_empty() { Vec::new() } else { old.split('\n').collect() };
        while lines.len() > 15 {
            lines.remove(0);
        }
        lines.push(s);
        let t = lines.join("\n");
        c.set_text(&t);
        c.caret = t.chars().count();
        c.sel_anchor = c.caret;
        c.scroll_y = (lines.len() as i32 - 5).max(0);
    }

    /// Reads all forms into the scene (`MakeHeader`).
    pub fn make_scene(&mut self, ui: &mut Ui) {
        main_form::make_header(self, ui);
        formula_form::make_header(self, ui);
        light_form::put_light_in_header(self, ui);
        postpro_form::make_header(self, ui);
    }

    /// Puts the scene into all forms (`SetEditsFromHeader`).
    pub fn scene_to_forms(&mut self, ui: &mut Ui) {
        main_form::set_edits_from_header(self, ui);
        formula_form::update_from_header(self, ui);
        light_form::set_from_header(self, ui);
        postpro_form::set_from_header(self, ui);
        // the card's shaders for it, compiled meanwhile
        crate::render::prewarm_gpu(self.scene.clone());
    }

    /// The scene as the forms describe it now (the current scene stays as
    /// it is), for compiling the card's shaders ahead.
    pub fn prewarm_from_forms(&mut self, ui: &mut Ui) {
        let saved = self.scene.clone();
        self.make_scene(ui);
        let sc = std::mem::replace(&mut self.scene, saved);
        crate::render::prewarm_gpu(sc);
    }

    /// Loads a parameter file (.m3p / .m3i / text) as the current scene.
    pub fn load_params(&mut self, ui: &mut Ui, path: &std::path::Path) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        let is_m3i = lower.ends_with(".m3i");
        let text = String::from_utf8_lossy(&data);
        let (sc, notes) = if lower.ends_with(".m3p") || is_m3i || text.contains("Mandelbulb3Dv") {
            let (raw, _) = crate::m3p::raw_from_bytes(data.clone(), is_m3i)?;
            let m = crate::m3p::parse(&raw)?;
            (m.scene, m.warnings)
        } else {
            (Scene::parse(&text)?, Vec::new())
        };
        self.scene = sc;
        self.notes = notes;
        self.title = name;
        self.eng.clear();
        self.scene_to_forms(ui);
        main_form::set_caption(self, ui);
        for n in self.notes.clone() {
            self.message(ui, &n);
        }
        if is_m3i {
            if let Some((w, h, g)) = crate::m3p::gbuffer_from_m3i(&data) {
                if w == self.scene.width as usize && h == self.scene.height as usize {
                    self.eng.set_gbuffer(&self.scene, g)?;
                }
            }
        }
        Ok(())
    }
}

/// Opens the JIT formula editor (new formula or `name`).
pub fn forms_jit_open(app: &mut Mb3d, ui: &mut Ui, name: Option<String>) {
    jit_form::open(app, ui, name);
}

impl App for Mb3d {
    fn event(&mut self, ui: &mut Ui, e: &Event) {
        if let Ev::DialogResult(r) = &e.ev {
            if e.form.is_empty() || e.sender == "@drop" {
                self.dialog_result(ui, &e.sender, r);
                return;
            }
        }
        if let Ev::Move = e.ev {
            return;
        }
        // the log: what the user did (first what no handler took before
        // this event, so the lines keep the order things happened in)
        for u in ui.take_unhandled() {
            event_log::event(ui, &u);
        }
        event_log::event(ui, e);
        match e.form.as_str() {
            MAIN => main_form::event(self, ui, e),
            "FormulaGUIForm" => formula_form::event(self, ui, e),
            "LightAdjustForm" => light_form::event(self, ui, e),
            "PostProForm" => postpro_form::event(self, ui, e),
            "VisualThemesFrm" => small_forms::themes_event(self, ui, e),
            "LanguageFrm" => lang_form::event(self, ui, e),
            "HotkeysFrm" => hotkeys::event(self, ui, e),
            "SwarmFrm" => swarm_form::event(self, ui, e),
            "UnfinishedFrm" => unfinished_form::event(self, ui, e),
            "HistoryFrm" => history_form::event(self, ui, e),
            "LogFrm" => log_form::event(self, ui, e),
            "AiConnectFrm" => ai_form::event(self, ui, e),
            "IniDirForm" => small_forms::inidirs_event(self, ui, e),
            "MapSequencesFrm" => small_forms::mapseq_event(self, ui, e),
            "FTextBox" => small_forms::textbox_event(self, ui, e),
            "ZBuf16BitGenFrm" => small_forms::zbuf_event(self, ui, e),
            "FNavigator" => navi_form::event(self, ui, e),
            "AnimationForm" => anim_form::event(self, ui, e),
            "ColorForm" => palette_form::event(self, ui, e),
            "AniPreviewForm" => anim_form::preview_event(self, ui, e),
            "AniProcessForm" => anim_form::process_event(self, ui, e),
            "BatchForm1" => tools_forms::batch_event(self, ui, e),
            "TilingForm" => tools_forms::tiling_event(self, ui, e),
            "MCForm" => mc_form::event(self, ui, e),
            "FVoxelExport" => voxel_form::event(self, ui, e),
            "BulbTracer2Frm" => bulbtracer_form::event(self, ui, e),
            "MutaGenFrm" => mutagen_form::event(self, ui, e),
            "MeshPreviewFrm" => meshview::preview_event(self, ui, e),
            "HeightMapGenFrm" => meshview::heightmap_event(self, ui, e),
            "JITFormulaEditorForm" => jit_form::event(self, ui, e),
            "ParamValueEditFrm" => jit_form::value_event(self, ui, e),
            "FColorOptions" => mc_form::color_options_event(self, ui, e),
            _ => {}
        }
    }

    fn hotkey(&mut self, ui: &mut Ui, form: usize, key: u16, shift: u8) -> bool {
        let done = hotkeys::hotkey(self, ui, form, key, shift);
        if done {
            event_log::hotkey(ui, form, key, shift);
        }
        done
    }

    fn idle(&mut self, ui: &mut Ui) {
        // the controls the windows have no handler for: the log sees them too
        for e in ui.take_unhandled() {
            event_log::event(ui, &e);
        }
        event_log::idle();
        main_form::idle(self, ui);
        if ui.showing("FNavigator") {
            navi_form::idle(self, ui);
        }
        anim_form::idle(self, ui);
        tools_forms::tiling_idle(self, ui);
        mc_form::idle(self, ui);
        voxel_form::idle(self, ui);
        bulbtracer_form::idle(self, ui);
        mutagen_form::idle(self, ui);
        meshview::idle(self, ui);
        file_preview::idle(self, ui);
        history_form::idle(self, ui);
        log_form::idle(self, ui);
    }
}

impl Mb3d {
    fn dialog_result(&mut self, ui: &mut Ui, tag: &str, r: &DialogResult) {
        // the history: every file opened or saved through a file dialog
        // (or dropped on the window); a chosen folder is none
        if let DialogResult::File(Some(p)) = r {
            if !p.is_dir() {
                let save = tag != "@drop" && ui.file_dialog_save;
                crate::history::log(if save { crate::history::Action::Save } else { crate::history::Action::Open }, p);
            }
        }
        if tag == "@drop" {
            return main_form::dialog(self, ui, "open", r);
        }
        let (form, what) = tag.split_once(':').unwrap_or(("", tag));
        match form {
            "main" => main_form::dialog(self, ui, what, r),
            "formula" => formula_form::dialog(self, ui, what, r),
            "light" => light_form::dialog(self, ui, what, r),
            "inidirs" => small_forms::inidirs_dialog(self, ui, what, r),
            "mapseq" => small_forms::mapseq_dialog(self, ui, what, r),
            "zbuf" => small_forms::zbuf_dialog(self, ui, what, r),
            "anim" => anim_form::dialog(self, ui, what, r),
            "palette" => palette_form::dialog(self, ui, what, r),
            "batch" => tools_forms::batch_dialog(self, ui, what, r),
            "tiling" => tools_forms::tiling_dialog(self, ui, what, r),
            "mc" => mc_form::dialog(self, ui, what, r),
            "navi" => navi_form::dialog(self, ui, what, r),
            "voxel" => voxel_form::dialog(self, ui, what, r),
            "btracer" => bulbtracer_form::dialog(self, ui, what, r),
            "mutagen" => mutagen_form::dialog(self, ui, what, r),
            "heightmap" => meshview::heightmap_dialog(self, ui, what, r),
            "jit" => jit_form::dialog(self, ui, what, r),
            "swarm" => swarm_form::dialog(self, ui, what, r),
            "unfinished" => unfinished_form::dialog(self, ui, what, r),
            "history" => history_form::dialog(self, ui, what, r),
            _ => {}
        }
    }
}

/// `Mandelbulb3D [file]`: opens the main window.
pub fn run(args: &[String]) -> Result<(), String> {
    let Some((ui, app)) = build(args)? else { return Ok(()) };
    crate::vcl::run::run(ui, Starter(app))
}

/// Creates the forms and the application state (without windows).
pub fn build(args: &[String]) -> Result<Option<(Ui, Mb3d)>, String> {
    let mut file = None;
    let mut scale_arg: Option<u32> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--formulas" => crate::formulas::add_formula_dir(it.next().ok_or("--formulas needs a folder")?.into()),
            "--maps" => crate::maps::add_map_dir(it.next().ok_or("--maps needs a folder")?.into()),
            "--scale" => scale_arg = Some(it.next().and_then(|v| v.trim_end_matches('%').parse().ok()).ok_or("--scale needs a percentage (e.g. 150)")?),
            "-h" | "--help" => {
                println!("Mandelbulb3D [--formulas DIR] [--maps DIR] [--scale PERCENT] [FILE.m3p|.m3i|.txt]");
                return Ok(None);
            }
            s => file = Some(PathBuf::from(s)),
        }
    }
    let ini = ini::Ini::load();
    // the interface scale: the command line, then MB3D_UI_SCALE, then the ini
    let scale_pct = scale_arg
        .or_else(|| std::env::var("MB3D_UI_SCALE").ok().and_then(|v| v.trim().trim_end_matches('%').parse().ok()))
        .unwrap_or_else(|| small_forms::ini_ui_scale(&ini));
    // MB3D's folders: formulas and maps of the ini (next to the program)
    crate::formulas::add_formula_dir(ini.dir(ini::DIR_FORMULAS));
    crate::maps::add_map_dir(ini.dir(ini::DIR_MAPS));
    // the themes of the `Themes` folder next to the program
    crate::vcl::theme::check()?;
    let style = Style::from_name(ini.get("VisualTheme")).unwrap_or_default();
    // the interface language (`Language` in the ini): the forms are built in it
    if let Some(l) = ini.get_extra("Language") {
        if !crate::vcl::lang::select(l) {
            crate::elog!("no language file for '{l}': English");
        }
    }
    let mut ui = Ui::new(style);
    ui.set_ui_scale(scale_pct.clamp(50, 400) as f64 / 100.0);
    for f in FORMS {
        if let Err(e) = ui.add_form(forms::get(f)) {
            crate::elog!("{f}: {e}");
        }
    }
    ui.main_form = ui.form_index(MAIN).ok_or("no main form")?;
    let swarm = swarm_form::State::from_ini(&ini);
    let mut app = Mb3d {
        scene: Scene::preset("Integer Power")?,
        title: String::new(),
        eng: engine::Engine::new(),
        ini,
        main: main_form::State::default(),
        formula: formula_form::State::default(),
        light: light_form::State::default(),
        post: postpro_form::State::default(),
        mapseq: small_forms::MapSeqState::default(),
        navi: navi_form::State::default(),
        anim: anim_form::State::default(),
        palette: palette_form::State::default(),
        batch: tools_forms::BatchState::default(),
        tiling: tools_forms::TilingState::default(),
        mc: mc_form::State::default(),
        swarm,
        unfinished: Default::default(),
        history: Default::default(),
        logview: Default::default(),
        voxel: voxel_form::State::default(),
        btracer: bulbtracer_form::State::default(),
        mutagen: mutagen_form::State::default(),
        meshview: meshview::State::default(),
        jit: jit_form::State::default(),
        preview: file_preview::State::default(),
        notes: Vec::new(),
        startup_file: file,
    };
    // the forms' OnCreate handlers
    app.process(&mut ui);
    main_form::start(&mut app, &mut ui);
    app.process(&mut ui);
    Ok(Some((ui, app)))
}

impl Mb3d {
    /// Hands the queued events to the handlers (outside the event loop).
    pub fn process(&mut self, ui: &mut Ui) {
        for _ in 0..10000 {
            let Some(e) = ui.take_event() else { break };
            if ui.internal_result(&e) {
                continue;
            }
            self.event(ui, &e);
        }
    }
}

/// Hands the waker to the engine once the event loop runs.
struct Starter(Mb3d);

impl App for Starter {
    fn event(&mut self, ui: &mut Ui, e: &Event) {
        self.0.event(ui, e)
    }
    fn hotkey(&mut self, ui: &mut Ui, form: usize, key: u16, shift: u8) -> bool {
        self.0.hotkey(ui, form, key, shift)
    }
    fn idle(&mut self, ui: &mut Ui) {
        if ui.waker().is_some() && !self.0.main.waker_set {
            self.0.eng.set_waker(ui.waker());
            self.0.main.waker_set = true;
        }
        self.0.idle(ui)
    }
}
