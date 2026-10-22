//! Batch processing (BatchForm.pas) and big renders in tiles (Tiling.pas).

use super::eta::ItemsLeft;
use super::util::{fts, left_str, pti, ptf, smooth_shown, Shown};
use super::{Mb3d, MAIN};
use crate::scene::Scene;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{DialogResult, Ev, Event, MouseButton, Ui};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------------------------------------------------------------------------
// BatchForm1
// ---------------------------------------------------------------------------

const BF: &str = "BatchForm1";

#[derive(Default)]
pub struct BatchState {
    /// status > 0: rendering, < 0: stopping
    pub status: i32,
    pub index: usize,
    /// the picture under way (1-based) and how many the run has
    pub nr: usize,
    pub total: usize,
}

fn batch_rows(ui: &Ui) -> Vec<Vec<String>> {
    ui.c(BF, "ListView1").cells.clone()
}

fn batch_set_rows(ui: &mut Ui, rows: Vec<Vec<String>>) {
    let c = ui.cm(BF, "ListView1");
    c.checks.resize(rows.len(), true);
    c.cells = rows;
    if c.item_index >= c.cells.len() as i32 {
        c.item_index = c.cells.len() as i32 - 1;
    }
}

fn batch_add(ui: &mut Ui, p: &Path) {
    let s = p.display().to_string();
    let mut rows = batch_rows(ui);
    if rows.iter().any(|r| r[0] == s) {
        return;
    }
    if p.extension().map(|e| e.eq_ignore_ascii_case("m3p")) != Some(true) {
        return;
    }
    if crate::m3p::load(p).is_err() {
        return;
    }
    rows.push(vec![s, "Selected".into()]);
    batch_set_rows(ui, rows);
}

/// Where the .m3i of a batch file goes.
fn m3i_target(app: &Mb3d, ui: &Ui, src: &Path) -> PathBuf {
    let name = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = if ui.checked(BF, "CheckBox2") { app.ini.dir(super::ini::DIR_M3I) } else { src.parent().map(Path::to_path_buf).unwrap_or_default() };
    dir.join(format!("{name}.m3i"))
}

/// `NextFile`: the next checked file into the main window and calculate.
pub fn batch_next(app: &mut Mb3d, ui: &mut Ui) {
    let mut rows = batch_rows(ui);
    let mut checks = ui.c(BF, "ListView1").checks.clone();
    checks.resize(rows.len(), false);
    let mut i = app.batch.index;
    if app.batch.status < 0 {
        i = rows.len();
    }
    if i < rows.len() {
        checks[i] = false;
        rows[i][1] = "M3I done".into();
    }
    loop {
        i = if i == usize::MAX { 0 } else { i + 1 };
        if i >= rows.len() {
            break;
        }
        if checks[i] {
            match app.load_params(ui, Path::new(&rows[i][0])) {
                Ok(()) => break,
                Err(e) => app.message(ui, &e),
            }
        }
    }
    app.batch.index = i;
    batch_set_rows(ui, rows.clone());
    ui.cm(BF, "ListView1").checks = checks;
    if i < rows.len() {
        app.batch.nr += 1;
        super::main_form::calc_mand(app, ui, true);
    } else {
        app.batch.status = 0;
        app.batch.index = usize::MAX;
        ui.set_enabled(BF, "Button1", true);
        ui.set_caption(BF, "Button1", "Start batch rendering");
        ui.fm(BF).set_caption(BATCH_TITLE);
    }
}

const BATCH_TITLE: &str = "Batch processing";

/// The window's title while the batch renders: "Batch processing: picture
/// 3 of 8 · about 7 min left", the picture's time left as the main window
/// shows it (the pictures differ too much for a total).
pub fn batch_idle(app: &mut Mb3d, ui: &mut Ui) {
    if app.batch.status <= 0 {
        return;
    }
    let left = picture_left(&ui.caption(MAIN, "Label6")).map(|l| format!(" · {l}")).unwrap_or_default();
    let text = format!("{BATCH_TITLE}: picture {} of {}{left}", app.batch.nr, app.batch.total);
    if ui.fm(BF).caption() != text {
        ui.fm(BF).set_caption(&text);
    }
}

/// The time left in the main window's stage line ("main rendering · about
/// 7 min left in total", "hard shadows: 40 % · 20 s left"): "about 7 min
/// left" (None while it says none).
fn picture_left(caption: &str) -> Option<&str> {
    let (_, l) = caption.rsplit_once(" · ")?;
    let l = l.strip_suffix(" in total").unwrap_or(l);
    l.ends_with(" left").then_some(l)
}

/// A batch image is calculated: store the .m3i (and a PNG preview).
pub fn batch_finished(app: &mut Mb3d, ui: &mut Ui) {
    if app.batch.status == 0 {
        return;
    }
    let rows = batch_rows(ui);
    if let Some(r) = rows.get(app.batch.index) {
        let m3i = m3i_target(app, ui, Path::new(&r[0]));
        super::main_form::save_m3i(app, ui, &m3i);
        if ui.checked(BF, "SavePNGPreviewCBx") {
            let name = m3i.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let dir = m3i.parent().map(Path::to_path_buf).unwrap_or_default();
            if let Some((rgb, w, h)) = app.main.rgb.clone() {
                let s = app.main.image_scale.max(1) as usize;
                let (px, w, h) = if s > 1 { crate::render::downsample(&rgb, w, h, s) } else { ((*rgb).clone(), w, h) };
                let _ = crate::png::write_rgb(&dir.join(format!("{name}.png")).to_string_lossy(), w, h, &px);
            }
        }
    }
    batch_next(app, ui);
}

pub fn batch_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormCreate" => {
            let c = ui.cm(BF, "ListView1");
            c.cells.clear();
            c.checks.clear();
        }
        "Button2Click" => ui.hide(BF),
        "Button3Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Parameter (*.m3p)|*.m3p".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3P)),
                multi: true,
                ..Default::default()
            };
            ui.open_dialog("batch:open", &o);
        }
        "Button4Click" => {
            let d = app.ini.dir(if ui.checked(BF, "CheckBox2") { super::ini::DIR_M3I } else { super::ini::DIR_M3P });
            open_folder(&d);
        }
        "Button1Click" => {
            if ui.caption(BF, "Button1") == "Stop batch rendering" {
                app.batch.status = 0;
                app.batch.index = usize::MAX;
                app.eng.stop();
                super::main_form::enable_buttons(app, ui);
                ui.set_caption(BF, "Button1", "Start batch rendering");
                ui.fm(BF).set_caption(BATCH_TITLE);
            } else {
                ui.set_caption(BF, "Button1", "Stop batch rendering");
                app.batch.status = 1;
                app.batch.index = usize::MAX;
                app.batch.nr = 0;
                app.batch.total = ui.c(BF, "ListView1").checks.iter().filter(|c| **c).count();
                batch_next(app, ui);
            }
        }
        "ListView1KeyDown" => {
            if let Ev::KeyDown { key: 46, .. } = e.ev {
                delete_selected(app, ui);
            }
        }
        "ListView1Change" => {
            if app.batch.status == 0 {
                let c = ui.cm(BF, "ListView1");
                let n = c.cells.len();
                c.checks.resize(n, true);
                for i in 0..n {
                    if c.cells[i].len() < 2 {
                        c.cells[i].resize(2, String::new());
                    }
                    c.cells[i][1] = if c.checks[i] { "Selected" } else { "Not selected" }.into();
                }
            }
        }
        "Deleteselectedfilesfromlist1Click" => delete_selected(app, ui),
        "Clearthewholelist1Click" => {
            if app.batch.status == 0 {
                batch_set_rows(ui, Vec::new());
            }
        }
        _ => {}
    }
}

fn delete_selected(app: &Mb3d, ui: &mut Ui) {
    if app.batch.status != 0 {
        return;
    }
    let i = ui.c(BF, "ListView1").item_index;
    let mut rows = batch_rows(ui);
    if i >= 0 && (i as usize) < rows.len() {
        rows.remove(i as usize);
        let checks = &mut ui.cm(BF, "ListView1").checks;
        if (i as usize) < checks.len() {
            checks.remove(i as usize);
        }
        batch_set_rows(ui, rows);
    }
}

pub fn batch_dialog(_app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("open", DialogResult::Files(v)) => {
            for p in v {
                batch_add(ui, p);
            }
        }
        ("open", DialogResult::File(Some(p))) => batch_add(ui, p),
        _ => {}
    }
}

/// Opens a folder in the system's file manager.
pub fn open_folder(d: &Path) {
    let _ = open::that_detached(d);
}

// ---------------------------------------------------------------------------
// TilingForm (big renders)
// ---------------------------------------------------------------------------

const TF: &str = "TilingForm";

/// `TBigRenderData` (256 bytes in the .big file).
#[derive(Clone, Debug)]
pub struct BigData {
    pub scale: f64,
    pub width: i32,
    pub height: i32,
    pub cols: i32,
    pub rows: i32,
    pub tile_w: i32,
    pub tile_h: i32,
    pub downscale: i32,
    pub output: i32,
    pub scale_de: bool,
    pub save_zbuf: bool,
    pub save_m3i: bool,
    pub jpeg_q: i32,
    pub sharp: i32,
    pub rows_from: i32,
    pub rows_to: i32,
    pub render_rows: bool,
    pub single_number: bool,
}

impl Default for BigData {
    fn default() -> Self {
        BigData {
            scale: 3.0,
            width: 0,
            height: 0,
            cols: 4,
            rows: 4,
            tile_w: 0,
            tile_h: 0,
            downscale: 1,
            output: 0,
            scale_de: true,
            save_zbuf: false,
            save_m3i: false,
            jpeg_q: 95,
            sharp: 0,
            rows_from: 1,
            rows_to: 999,
            render_rows: false,
            single_number: false,
        }
    }
}

impl BigData {
    fn to_bytes(&self) -> Vec<u8> {
        let mut d = Vec::with_capacity(256);
        d.extend_from_slice(&self.scale.to_le_bytes());
        for v in [self.width, self.height, self.cols, self.rows, self.tile_w, self.tile_h, self.downscale, self.output] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        for b in [self.scale_de, self.save_zbuf, self.save_m3i] {
            d.extend_from_slice(&(b as i32).to_le_bytes());
        }
        d.extend_from_slice(&1.99f32.to_le_bytes());
        for v in [self.jpeg_q, self.sharp, 0, 3, self.rows_from, self.rows_to, self.render_rows as i32, self.single_number as i32] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.resize(256, 0);
        d
    }

    fn from_bytes(d: &[u8]) -> Option<BigData> {
        if d.len() < 256 {
            return None;
        }
        let i = |o: usize| i32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        let version = i(68);
        Some(BigData {
            scale: f64::from_le_bytes(d[0..8].try_into().unwrap()),
            width: i(8),
            height: i(12),
            cols: i(16),
            rows: i(20),
            tile_w: i(24),
            tile_h: i(28),
            downscale: i(32).clamp(1, 3),
            output: i(36),
            scale_de: i(40) != 0,
            save_zbuf: i(44) != 0,
            save_m3i: i(48) != 0,
            jpeg_q: i(56),
            sharp: i(60),
            rows_from: if version < 2 { 1 } else { i(72) },
            rows_to: if version < 2 { 999 } else { i(76) },
            render_rows: version >= 2 && i(80) != 0,
            single_number: version >= 3 && i(84) != 0,
        })
    }
}

pub struct TilingState {
    pub orig: Option<Scene>,
    pub data: BigData,
    pub project: String,
    pub dir: PathBuf,
    pub done: Vec<Vec<bool>>,
    pub act: (i32, i32),
    pub selected: (i32, i32),
    pub only_this: bool,
    pub job: Arc<Mutex<Option<Result<(i32, i32), String>>>>,
    pub running: bool,
    pub user_change: bool,
    /// set by "Stop": the render (and the swarm job) is given up
    pub cancel: Arc<AtomicBool>,
    /// a swarm render's state and the tiles it saved so far
    pub status: Arc<Mutex<String>>,
    pub saved: Arc<AtomicUsize>,
    pub shown_saved: usize,
    /// the time left: when the running tile began, the tiles' times, and
    /// the time left as shown
    pub tile_start: Instant,
    pub items: ItemsLeft,
    pub left_shown: Option<Shown>,
}

impl Default for TilingState {
    fn default() -> Self {
        TilingState {
            orig: None,
            data: BigData::default(),
            project: "(new)".into(),
            dir: PathBuf::new(),
            done: Vec::new(),
            act: (0, 0),
            selected: (0, 0),
            only_this: false,
            job: Arc::new(Mutex::new(None)),
            running: false,
            user_change: true,
            cancel: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(String::new())),
            saved: Arc::new(AtomicUsize::new(0)),
            shown_saved: 0,
            tile_start: Instant::now(),
            items: ItemsLeft::default(),
            left_shown: None,
        }
    }
}

/// `MakeFilePointIndizes`: "X01Y02" or the single number "0005".
fn tile_index(d: &BigData, x: i32, y: i32) -> String {
    if d.single_number {
        format!("{:04}", x + (y - 1) * d.cols)
    } else {
        format!("X{x:02}Y{y:02}")
    }
}

fn tile_pos_of(d: &BigData, name: &str) -> Option<(i32, i32)> {
    let stem = Path::new(name).file_stem()?.to_string_lossy().into_owned();
    let l = stem.len();
    if d.single_number {
        let i: i32 = stem.get(l.checked_sub(4)?..)?.parse().ok()?;
        let y = (i - 1) / d.cols.max(1) + 1;
        Some((i - (y - 1) * d.cols, y))
    } else {
        let y: i32 = stem.get(l.checked_sub(2)?..)?.parse().ok()?;
        let x: i32 = stem.get(l.checked_sub(5)?..l - 3)?.parse().ok()?;
        Some((x, y))
    }
}

fn set_names(app: &mut Mb3d, ui: &mut Ui, file: &Path) {
    let st = &mut app.tiling;
    st.project = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "(new)".into());
    st.dir = file.parent().map(Path::to_path_buf).unwrap_or_default().join(&st.project);
    ui.set_caption(TF, "Label13", &format!("Project:  {}", st.project));
    scan_project(app, ui);
}

fn new_project(app: &mut Mb3d, ui: &mut Ui) {
    let p = app.ini.dir(super::ini::DIR_BIG).join("(new)");
    set_names(app, ui, &p);
}

/// Writes a tile's picture whole or not at all: into a hidden temporary
/// file first (a half written tile would count as done, see
/// [`scan_project`]), then renamed.
fn write_tile(p: &std::path::Path, data: &[u8]) -> Result<(), String> {
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = p.with_file_name(format!(".{name}.tmp{}", std::process::id()));
    std::fs::write(&tmp, data).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, p).map_err(|e| format!("{}: {e}", p.display()))
}

/// `ScanProjectFolder`: which tiles are already there.
fn scan_project(app: &mut Mb3d, ui: &mut Ui) {
    let st = &mut app.tiling;
    let (c, r) = (st.data.cols.clamp(1, 99) as usize, st.data.rows.clamp(1, 99) as usize);
    st.done = vec![vec![false; c]; r];
    if let Ok(rd) = std::fs::read_dir(&st.dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if !n.starts_with(&st.project) {
                continue;
            }
            if let Some((x, y)) = tile_pos_of(&st.data, &n) {
                if x >= 1 && y >= 1 && (x as usize) <= c && (y as usize) <= r {
                    st.done[y as usize - 1][x as usize - 1] = true;
                }
            }
        }
    }
    paint_tiling(app, ui);
}

/// `PaintImageTiling`: the tile grid, finished tiles light.
fn paint_tiling(app: &Mb3d, ui: &mut Ui) {
    let d = &app.tiling.data;
    if d.width <= 0 || d.height <= 0 {
        return;
    }
    let (w, h) = if d.width as f64 / d.height as f64 > 324.0 / 280.0 {
        (324usize, (324.0 * d.height as f64 / d.width as f64).round() as usize)
    } else {
        ((280.0 * d.width as f64 / d.height as f64).round() as usize, 280usize)
    };
    let (w, h) = (w.max(2), h.max(2));
    let mut b = Bitmap::new(w, h, 0xFFA0_A0A0);
    let (tc, tr) = (d.cols.max(1) as usize, d.rows.max(1) as usize);
    for (i, row) in app.tiling.done.iter().enumerate() {
        for (j, &done) in row.iter().enumerate() {
            if done {
                let (x0, y0) = (j * w / tc, i * h / tr);
                let (x1, y1) = ((j + 1) * w / tc, (i + 1) * h / tr);
                for y in y0..y1.min(h) {
                    for x in x0..x1.min(w) {
                        b.px[y * w + x] = 0xFFF0_F0F0;
                    }
                }
            }
        }
    }
    for i in 1..tc {
        let x = i * w / tc;
        for y in 0..h {
            b.px[y * w + x] = 0xFF40_4040;
        }
    }
    for i in 1..tr {
        let y = i * h / tr;
        for x in 0..w {
            b.px[y * w + x] = 0xFF40_4040;
        }
    }
    for x in 0..w {
        b.px[x] = 0xFF40_4040;
        b.px[(h - 1) * w + x] = 0xFF40_4040;
    }
    for y in 0..h {
        b.px[y * w] = 0xFF40_4040;
        b.px[y * w + w - 1] = 0xFF40_4040;
    }
    let c = ui.cm(TF, "Image1");
    c.width = w as i32;
    c.height = h as i32;
    c.picture = Some(b);
}

/// `MakeData`.
fn make_data(app: &mut Mb3d, ui: &Ui) {
    let Some(o) = &app.tiling.orig else { return };
    let d = &mut app.tiling.data;
    d.scale = ptf(&ui.text(TF, "Edit1")).max(0.01);
    d.cols = ui.position(TF, "UpDown3").clamp(1, 99) as i32;
    d.rows = ui.position(TF, "UpDown2").clamp(1, 99) as i32;
    d.downscale = ui.position(TF, "UpDown1").clamp(1, 3) as i32;
    d.output = ui.item_index(TF, "RadioGroup1").max(0);
    d.scale_de = ui.checked(TF, "CheckBox1");
    d.save_zbuf = ui.checked(TF, "CheckBox2");
    d.save_m3i = ui.checked(TF, "CheckBox3");
    d.tile_w = ((o.width as f64 * d.scale / d.cols as f64).round() as i32 / d.downscale) * d.downscale;
    d.tile_h = ((o.height as f64 * d.scale / d.rows as f64).round() as i32 / d.downscale) * d.downscale;
    d.width = d.tile_w * d.cols;
    d.height = d.tile_h * d.rows;
    d.jpeg_q = ui.position(TF, "UpDown4").clamp(1, 100) as i32;
    d.sharp = ui.position(TF, "UpDownSharp") as i32;
    d.rows_from = pti(&ui.text(TF, "Edit2"));
    d.rows_to = pti(&ui.text(TF, "Edit3"));
    d.render_rows = ui.checked(TF, "CheckBox6");
    d.single_number = ui.checked(TF, "CheckBox7");
}

fn set_data(app: &mut Mb3d, ui: &mut Ui) {
    app.tiling.user_change = false;
    let d = app.tiling.data.clone();
    ui.set_text(TF, "Edit1", &fts(d.scale));
    let set_ud = |ui: &mut Ui, ud: &str, ed: &str, v: i32| {
        ui.set_position(TF, ud, v as i64);
        ui.set_text(TF, ed, &v.to_string());
    };
    set_ud(ui, "UpDown3", "Edit21", d.cols);
    set_ud(ui, "UpDown2", "Edit6", d.rows);
    set_ud(ui, "UpDown1", "Edit5", d.downscale);
    set_ud(ui, "UpDown4", "Edit7", d.jpeg_q);
    set_ud(ui, "UpDownSharp", "Edit4", d.sharp);
    ui.set_item_index(TF, "RadioGroup1", d.output);
    ui.set_checked(TF, "CheckBox1", d.scale_de);
    ui.set_checked(TF, "CheckBox2", d.save_zbuf);
    ui.set_checked(TF, "CheckBox3", d.save_m3i);
    ui.set_checked(TF, "CheckBox6", d.render_rows);
    ui.set_checked(TF, "CheckBox7", d.single_number);
    ui.set_text(TF, "Edit2", &d.rows_from.to_string());
    ui.set_text(TF, "Edit3", &d.rows_to.to_string());
    app.tiling.user_change = true;
}

fn set_sizes(app: &Mb3d, ui: &mut Ui) {
    let d = &app.tiling.data;
    if let Some(o) = &app.tiling.orig {
        ui.set_caption(TF, "Label10", &format!("{} x {}", o.width, o.height));
    }
    ui.set_caption(TF, "Label11", &format!("{} x {}", d.tile_w / d.downscale.max(1), d.tile_h / d.downscale.max(1)));
    ui.set_caption(TF, "Label12", &format!("{} x {}", d.width / d.downscale.max(1), d.height / d.downscale.max(1)));
    let big = d.tile_w as i64 * d.tile_h as i64 > 25_000_000 || d.tile_w > 32000 || d.tile_h > 32000;
    ui.cm(TF, "Label11").font = big.then(|| crate::vcl::font::Font { color: 0xFFFF_0000, custom_color: true, ..Default::default() });
    paint_tiling(app, ui);
}

fn import(app: &mut Mb3d, ui: &mut Ui) {
    let Some(o) = app.tiling.orig.as_mut() else { return };
    o.tiling = None;
    let mut notes = Vec::new();
    if o.ao.is_some() && o.deao.is_none() {
        // only DEAO works on tiles
        o.deao = Some(crate::deao::DeaoParams { quality: 3, dither: 0, max_len: 1.0, first_step_random: false });
        notes.push("Only DEAO ambient shadows available in tiling mode.\nIs now selected and set to 33 rays, please change it for your own needs.");
    }
    if o.dof.is_some() {
        o.dof = None;
        notes.push("DoF calculation turned off, not possible in tiling mode.");
    }
    make_data(app, ui);
    set_sizes(app, ui);
    for c in ["Panel2", "Button2", "Button3", "SpeedButton9"] {
        ui.set_enabled(TF, c, true);
    }
    for n in notes {
        ui.show_message(n);
    }
}

fn next_tile(st: &TilingState, ui: &Ui, from: (i32, i32)) -> (i32, i32) {
    let d = &st.data;
    let (ya, ye) = if ui.checked(TF, "CheckBox6") {
        (from.1.max(pti(&ui.text(TF, "Edit2"))).max(1), d.rows.min(pti(&ui.text(TF, "Edit3"))))
    } else {
        (from.1.max(1), d.rows)
    };
    for y in ya..=ye {
        let xa = if y == ya { from.0.max(1) } else { 1 };
        for x in xa..=d.cols {
            let done = st.done.get(y as usize - 1).and_then(|r| r.get(x as usize - 1)).copied().unwrap_or(false);
            if !done {
                return (x, y);
            }
        }
    }
    (0, 0)
}

/// The tiles still to do after the running one (none when only it is).
fn tiles_after(st: &TilingState, ui: &Ui) -> usize {
    if st.only_this || !(ui.checked(TF, "CheckBox4") || ui.checked(TF, "CheckBox6")) {
        return 0;
    }
    let d = &st.data;
    let (ya, ye) = if ui.checked(TF, "CheckBox6") { (pti(&ui.text(TF, "Edit2")).max(1), d.rows.min(pti(&ui.text(TF, "Edit3")))) } else { (1, d.rows) };
    let (ax, ay) = st.act;
    (ya..=ye)
        .flat_map(|y| (1..=d.cols).map(move |x| (x, y)))
        .filter(|&(x, y)| (y, x) > (ay, ax))
        .filter(|&(x, y)| !st.done.get(y as usize - 1).and_then(|r| r.get(x as usize - 1)).copied().unwrap_or(false))
        .count()
}

/// "tile X=1 Y=2 · about 25 min left in total" (tiles follow), "tile X=2
/// Y=2 · 2 min left" (the last one): the main window's stage line of a big
/// render (the first tile has no time left: the tile's progress restarts
/// with each of its stages, nothing to measure a whole tile by).
fn tile_caption(x: i32, y: i32, left: Option<f64>, more: bool) -> String {
    let name = format!("tile X={x} Y={y}");
    match left {
        Some(l) if more => format!("{name} · {} in total", left_str(l)),
        Some(l) => format!("{name} · {}", left_str(l)),
        None => name,
    }
}

/// A run of tiles begins: the tiles' times start afresh.
fn begin_tiles(app: &mut Mb3d) {
    app.tiling.items = ItemsLeft::default();
    app.tiling.left_shown = None;
}

fn save_project(app: &Mb3d, p: &Path) -> Result<(), String> {
    let Some(o) = &app.tiling.orig else { return Err("no parameters".into()) };
    let mut d = app.tiling.data.to_bytes();
    d.extend_from_slice(&crate::m3p::write(o));
    std::fs::write(p.with_extension("big"), d).map_err(|e| format!("{}: {e}", p.display()))
}

/// `LoadBig`.
pub fn load_big(app: &mut Mb3d, ui: &mut Ui, p: &Path) {
    let res = std::fs::read(p).map_err(|e| e.to_string()).and_then(|d| {
        let bd = BigData::from_bytes(&d).ok_or("not a big render project")?;
        let m = crate::m3p::parse(&d[256..])?;
        Ok((bd, m.scene))
    });
    match res {
        Ok((bd, sc)) => {
            app.tiling.data = bd;
            app.tiling.orig = Some(sc);
            set_names(app, ui, p);
            set_data(app, ui);
            set_sizes(app, ui);
            for c in ["Panel2", "Button2", "Button3", "SpeedButton9"] {
                ui.set_enabled(TF, c, true);
            }
            ui.show(TF);
        }
        Err(e) => ui.show_message(&format!("{}: {e}", p.display())),
    }
}

/// `RenderActualTile`: calculates a tile in the background and saves it.
fn render_tile(app: &mut Mb3d, ui: &mut Ui) {
    let (x, y) = app.tiling.act;
    if x == 0 {
        ui.show_message("No tiles to render.");
        return;
    }
    if app.tiling.project == "(new)" {
        ui.show_message("Please save the project first.");
        return;
    }
    let Some(o) = app.tiling.orig.clone() else { return };
    let d = app.tiling.data.clone();
    let _ = std::fs::create_dir_all(&app.tiling.dir);
    let big_file = app.tiling.dir.with_extension("big");
    if !big_file.exists() {
        let _ = save_project(app, &big_file);
    }
    let mut sc = o.clone();
    let f = d.width as f64 / o.width.max(1) as f64;
    if d.scale_de {
        sc.scale_image(f);
    }
    sc.width = d.width;
    sc.height = d.height;
    sc.tiling = Some(crate::scene::Tiling { cols: d.cols as u32, rows: d.rows as u32, pos: Some(((x - 1) as u32, (y - 1) as u32)), downscale: d.downscale as u32 });
    let base = app.tiling.dir.join(format!("{}{}", app.tiling.project, tile_index(&d, x, y)));
    let zbase = app.tiling.dir.join(format!("ZBuf {}{}", app.tiling.project, tile_index(&d, x, y)));
    let job = app.tiling.job.clone();
    let waker = ui.waker();
    // with the swarm: all the tiles still to do, as one job, each tile's
    // files written as this window writes them
    let swarm = super::swarm_form::calc_folder(app).map(|dir| {
        let mut list = vec![(x, y)];
        if !app.tiling.only_this && (ui.checked(TF, "CheckBox4") || ui.checked(TF, "CheckBox6")) {
            let mut cur = (x, y);
            loop {
                let after = if cur.0 < d.cols { (cur.0 + 1, cur.1) } else { (1, cur.1 + 1) };
                let n = next_tile(&app.tiling, ui, after);
                if n.0 <= 0 || list.contains(&n) {
                    break;
                }
                list.push(n);
                cur = n;
            }
        }
        (dir, list, app.swarm.name.clone(), app.tiling.dir.clone(), app.tiling.project.clone())
    });
    app.tiling.running = true;
    app.tiling.cancel.store(false, Ordering::SeqCst);
    app.tiling.saved.store(0, Ordering::SeqCst);
    app.tiling.shown_saved = 0;
    *app.tiling.status.lock().unwrap() = String::new();
    app.tiling.tile_start = Instant::now();
    let cancel = app.tiling.cancel.clone();
    super::main_form::disable_buttons(app, ui);
    let title = match &swarm {
        Some((_, list, ..)) => format!("{}  {} tiles with the swarm", app.tiling.project, list.len()),
        None => format!("{}  Tile X={x} Y={y}", app.tiling.project),
    };
    ui.fm(MAIN).set_caption(&title);
    for c in ["Button2", "SpeedButton1", "SpeedButton2", "SpeedButton11", "SpeedButton9"] {
        ui.set_enabled(TF, c, false);
    }
    if let Some((dir, list, name, pdir, project)) = swarm {
        let (status, saved) = (app.tiling.status.clone(), app.tiling.saved.clone());
        let mut full = sc.clone();
        full.tiling = Some(crate::scene::Tiling { cols: d.cols as u32, rows: d.rows as u32, pos: None, downscale: d.downscale as u32 });
        std::thread::spawn(move || {
            let last = *list.last().unwrap_or(&(x, y));
            let r = (|| -> Result<(i32, i32), String> {
                let subset: Vec<usize> = list.iter().map(|&(tx, ty)| ((ty - 1) * d.cols + (tx - 1)) as usize).collect();
                let stop = cancel.clone();
                let opts = crate::swarm::RunOptions { name, threads: 0, quiet: true, cancel: Some(Arc::new(move || stop.load(Ordering::SeqCst))), ..Default::default() };
                let ds = d.downscale.max(1) as usize;
                let collect = |k: usize, m3i: &std::path::Path, png: &std::path::Path| -> Result<(), String> {
                    let (tx, ty) = ((k as i32 % d.cols) + 1, (k as i32 / d.cols) + 1);
                    let base = pdir.join(format!("{project}{}", tile_index(&d, tx, ty)));
                    let zbase = pdir.join(format!("ZBuf {project}{}", tile_index(&d, tx, ty)));
                    let img = crate::image::load(png)?;
                    let s = if img.deep { 8 } else { 0 };
                    let rgb: Vec<u8> = img.data.iter().flat_map(|p| [(p[0] >> s) as u8, (p[1] >> s) as u8, (p[2] >> s) as u8]).collect();
                    let (rgb, w, h) = if ds > 1 { crate::render::downsample(&rgb, img.width, img.height, ds) } else { (rgb, img.width, img.height) };
                    let (data, ext) = match d.output {
                        1 => (crate::jpeg::encode(w, h, &rgb, d.jpeg_q.clamp(1, 100) as u8), "jpg"),
                        2 => (crate::frames::encode_bmp(w, h, &rgb), "bmp"),
                        _ => (crate::png::encode_rgb(w, h, &rgb), "png"),
                    };
                    let p = base.with_extension(ext);
                    write_tile(&p, &data)?;
                    if d.save_zbuf || d.save_m3i {
                        let data = std::fs::read(m3i).map_err(|e| format!("{}: {e}", m3i.display()))?;
                        let (zw, zh, g) = crate::m3p::gbuffer_from_m3i(&data).ok_or("the tile has no image buffer")?;
                        if d.save_zbuf {
                            let z: Vec<u16> = g.iter().map(|s| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 }).collect();
                            let _ = std::fs::write(zbase.with_extension("png"), crate::png::encode_gray16(zw, zh, &z));
                        }
                        if d.save_m3i {
                            let mut ts = sc.clone();
                            ts.width = zw as i32;
                            ts.height = zh as i32;
                            ts.tiling = None;
                            let _ = std::fs::write(base.with_extension("m3i"), crate::m3p::write_m3i(&ts, &g));
                        }
                    }
                    saved.fetch_add(1, Ordering::SeqCst);
                    if let Some(w) = &waker {
                        w.wake();
                    }
                    Ok(())
                };
                let report = |st: &str| {
                    *status.lock().unwrap() = st.to_string();
                    if let Some(w) = &waker {
                        w.wake();
                    }
                };
                crate::swarm::run_tiles_files(&dir, &full, d.cols as u32, d.rows as u32, Some(&subset), &opts, &report, None, &collect)?;
                Ok(last)
            })();
            *job.lock().unwrap() = Some(r);
            if let Some(w) = waker {
                w.wake();
            }
        });
        return;
    }
    std::thread::spawn(move || {
        let r = (|| -> Result<(i32, i32), String> {
            let stop = || cancel.load(Ordering::SeqCst);
            let res = crate::render::render_tiled_cancellable(&sc, d.save_zbuf || d.save_m3i, &|_, _| {}, &|_, _, _| {}, &stop)?;
            let ds = d.downscale.max(1) as usize;
            let (rgb, w, h) = if ds > 1 { crate::render::downsample(&res.rgb, res.width, res.height, ds) } else { (res.rgb.clone(), res.width, res.height) };
            let (data, ext) = match d.output {
                1 => (crate::jpeg::encode(w, h, &rgb, d.jpeg_q.clamp(1, 100) as u8), "jpg"),
                2 => (crate::frames::encode_bmp(w, h, &rgb), "bmp"),
                _ => (crate::png::encode_rgb(w, h, &rgb), "png"),
            };
            let p = base.with_extension(ext);
            write_tile(&p, &data)?;
            if let Some(g) = &res.gbuffer {
                if d.save_zbuf {
                    let z: Vec<u16> = g.iter().map(|s| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 }).collect();
                    let (zw, zh) = (res.width, res.height);
                    let _ = std::fs::write(zbase.with_extension("png"), crate::png::encode_gray16(zw, zh, &z));
                }
                if d.save_m3i {
                    let mut ts = sc.clone();
                    ts.width = res.width as i32;
                    ts.height = res.height as i32;
                    ts.tiling = None;
                    let _ = std::fs::write(base.with_extension("m3i"), crate::m3p::write_m3i(&ts, g));
                }
            }
            Ok((x, y))
        })();
        *job.lock().unwrap() = Some(r);
        if let Some(w) = waker {
            w.wake();
        }
    });
}

/// "Stop" in the main window: the big render ends after the tiles done.
pub fn stop_tiling(app: &mut Mb3d) {
    if app.tiling.running {
        app.tiling.cancel.store(true, Ordering::SeqCst);
    }
}

pub fn tiling_idle(app: &mut Mb3d, ui: &mut Ui) {
    // (the Batch window's title too: the same idle call)
    batch_idle(app, ui);
    if !app.tiling.running {
        return;
    }
    // a swarm render: the tiles saved so far light up, its state shows
    let saved = app.tiling.saved.load(Ordering::SeqCst);
    if saved != app.tiling.shown_saved {
        app.tiling.shown_saved = saved;
        scan_project(app, ui);
    }
    let st = app.tiling.status.lock().unwrap().clone();
    if !st.is_empty() && !app.tiling.cancel.load(Ordering::SeqCst) {
        let text = format!("{saved} {} saved; {st}", if saved == 1 { "tile" } else { "tiles" });
        if ui.caption(MAIN, "Label6") != text {
            ui.set_caption(MAIN, "Label6", &text);
        }
    }
    // a render here: the tile under way and the tiles to come at the
    // finished ones' time (shown smoothed, like a calculation's)
    if st.is_empty() && !app.tiling.cancel.load(Ordering::SeqCst) {
        let after = tiles_after(&app.tiling, ui);
        let now = Instant::now();
        let t = &mut app.tiling;
        let secs = now.saturating_duration_since(t.tile_start).as_secs_f64();
        let left = t.items.left(0.0, secs, None, after);
        t.left_shown = left.map(|l| smooth_shown(t.left_shown, l, now)).or(t.left_shown);
        let text = tile_caption(t.act.0, t.act.1, t.left_shown.map(|s| s.0), after > 0);
        if ui.caption(MAIN, "Label6") != text {
            ui.set_caption(MAIN, "Label6", &text);
        }
    }
    let Some(r) = app.tiling.job.lock().unwrap().take() else { return };
    app.tiling.running = false;
    app.tiling.items.done(app.tiling.tile_start.elapsed().as_secs_f64());
    super::main_form::enable_buttons(app, ui);
    for c in ["Button2", "SpeedButton1", "SpeedButton2", "SpeedButton11", "SpeedButton9"] {
        ui.set_enabled(TF, c, true);
    }
    match r {
        Ok((x, y)) => {
            app.message(ui, &format!("Tile X={x} Y={y} saved."));
            scan_project(app, ui);
            ui.fm(MAIN).set_caption(&app.title.clone());
            let auto = ui.checked(TF, "CheckBox4") || ui.checked(TF, "CheckBox6");
            if !app.tiling.only_this && auto {
                let n = next_tile(&app.tiling, ui, app.tiling.act);
                if n.0 > 0 {
                    app.tiling.act = n;
                    render_tile(app, ui);
                }
            }
        }
        Err(_) if app.tiling.cancel.load(Ordering::SeqCst) => {
            scan_project(app, ui);
            ui.fm(MAIN).set_caption(&app.title.clone());
            app.message(ui, "Big render stopped.");
        }
        Err(e) => app.message(ui, &format!("Big render: {e}")),
    }
}

pub fn tiling_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            ui.cm(TF, "Edit5").hint = "1:  Full size, no anti aliasing\n2:  2x2 anti aliasing\n3:  3x3 anti aliasing".into();
            ui.cm(TF, "Edit4").hint = "Sharpen factor of the saved output image,\nworks only with downscales of 1:2 and 1:3 !\n0: no sharpening ... 3: maximum sharpening".into();
        }
        "Button1Click" => ui.hide(TF),
        "SpeedButton2Click" => {
            app.make_scene(ui);
            let mut s = app.scene.clone();
            s.tiling = None;
            app.tiling.orig = Some(s);
            ui.set_position(TF, "UpDown1", app.main.image_scale.clamp(1, 3) as i64);
            ui.set_text(TF, "Edit5", &app.main.image_scale.clamp(1, 3).to_string());
            new_project(app, ui);
            import(app, ui);
        }
        "SpeedButton1Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Parameter (*.m3p)|*.m3p".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3P)),
                ..Default::default()
            };
            ui.open_dialog("tiling:import", &o);
        }
        "SpeedButton11Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Big render project (*.big)|*.big".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_BIG)),
                ..Default::default()
            };
            ui.open_dialog("tiling:open", &o);
        }
        "SpeedButton9Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D Big render project (*.big)|*.big".into(),
                default_ext: "big".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_BIG)),
                file_name: if app.tiling.project == "(new)" { String::new() } else { app.tiling.project.clone() },
                ..Default::default()
            };
            ui.save_dialog("tiling:save", &o);
        }
        "Button3Click" => {
            if let Some(o) = app.tiling.orig.clone() {
                app.scene = o;
                app.eng.clear();
                app.scene_to_forms(ui);
                app.title = app.tiling.project.clone();
                super::main_form::set_caption(app, ui);
                super::main_form::show_image(app, ui);
            }
        }
        "SpinEdit2Change" | "SpinEdit1Change" => {
            if app.tiling.user_change && app.tiling.orig.is_some() {
                if e.handler == "SpinEdit2Change" {
                    make_data(app, ui);
                    set_sizes(app, ui);
                    if e.sender == "CheckBox7" {
                        scan_project(app, ui);
                    }
                }
            }
            let en = ui.position(TF, "UpDown1") > 1;
            ui.set_enabled(TF, "UpDownSharp", en);
            ui.set_enabled(TF, "Edit4", en);
        }
        "Button2Click" => {
            let _ = std::fs::create_dir_all(&app.tiling.dir);
            scan_project(app, ui);
            app.tiling.act = next_tile(&app.tiling, ui, (0, 0));
            app.tiling.only_this = false;
            begin_tiles(app);
            render_tile(app, ui);
        }
        "Image1MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, x, y, .. } = e.ev {
                let r = ui.c(TF, "Image1").rect();
                let d = &app.tiling.data;
                let t = ((x * d.cols) / r.w.max(1) + 1, (y * d.rows) / r.h.max(1) + 1);
                app.tiling.selected = t;
                ui.set_caption(TF, "ileXY1", &format!("Tile: X={} Y={}", t.0, t.1));
                let done = app.tiling.done.get(t.1 as usize - 1).and_then(|row| row.get(t.0 as usize - 1)).copied().unwrap_or(false);
                ui.set_caption(TF, "Renderthistile1", if done { "Render this tile again" } else { "Render this tile" });
                ui.set_enabled(TF, "Deletethistilesfiles1", done);
                ui.popup_menu(TF, "PopupMenu1", "Image1", x, y);
            }
        }
        "Renderthistile1Click" => {
            begin_tiles(app);
            app.tiling.act = app.tiling.selected;
            app.tiling.only_this = !ui.checked(TF, "CheckBox4");
            render_tile(app, ui);
        }
        "Deletethistilesfiles1Click" => {
            let (x, y) = app.tiling.selected;
            let idx = tile_index(&app.tiling.data, x, y);
            for pre in ["", "ZBuf "] {
                let stem = format!("{pre}{}{idx}", app.tiling.project);
                if let Ok(rd) = std::fs::read_dir(&app.tiling.dir) {
                    for e in rd.flatten() {
                        if e.path().file_stem().map(|s| s.to_string_lossy() == stem) == Some(true) {
                            let _ = std::fs::remove_file(e.path());
                        }
                    }
                }
            }
            scan_project(app, ui);
        }
        "CheckBox6Click" => {
            if ui.checked(TF, "CheckBox6") {
                ui.set_checked(TF, "CheckBox4", false);
            }
        }
        "CheckBox4Click" => {
            if ui.checked(TF, "CheckBox4") {
                ui.set_checked(TF, "CheckBox6", false);
            }
        }
        _ => {}
    }
}

pub fn tiling_dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("import", DialogResult::File(Some(p))) => match crate::m3p::load(p) {
            Ok(m) => {
                app.tiling.orig = Some(m.scene);
                set_names(app, ui, p);
                import(app, ui);
            }
            Err(e) => ui.show_message(&e),
        },
        ("open", DialogResult::File(Some(p))) => load_big(app, ui, p),
        ("save", DialogResult::File(Some(p))) => {
            make_data(app, ui);
            set_names(app, ui, &p.with_extension("big"));
            if let Err(e) = save_project(app, &p.with_extension("big")) {
                ui.show_message(&e);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_data_round_trip() {
        let d = BigData { rows_from: 3, rows_to: 7, render_rows: true, single_number: true, sharp: 2, ..BigData::default() };
        let b = BigData::from_bytes(&d.to_bytes()).unwrap();
        assert_eq!((b.rows_from, b.rows_to, b.render_rows, b.single_number), (3, 7, true, true));
        assert_eq!((b.jpeg_q, b.sharp, b.cols, b.rows), (95, 2, 4, 4));
    }

    #[test]
    fn time_left_captions() {
        assert_eq!(tile_caption(1, 2, None, true), "tile X=1 Y=2");
        assert_eq!(tile_caption(1, 2, Some(1500.0), true), "tile X=1 Y=2 · about 25 min left in total");
        assert_eq!(tile_caption(3, 3, Some(120.0), false), "tile X=3 Y=3 · 2 min left");
        // the picture's time left from the main window's stage line
        assert_eq!(picture_left("main rendering · about 7 min left in total"), Some("about 7 min left"));
        assert_eq!(picture_left("hard shadows: 40 % · 20 s left"), Some("20 s left"));
        assert_eq!(picture_left("main rendering · shaders compiling"), None);
        assert_eq!(picture_left("main rendering"), None);
    }
}
