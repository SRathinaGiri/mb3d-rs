//! The History window (`HistoryFrm`, this program's own, the clock icon of
//! the main window's title bar): what [`crate::history`] keeps, the
//! calculated pictures with their thumbnails and the files opened and
//! saved, searchable, sortable by its columns, as a list, as rows with
//! their pictures or as tiles (the rows and the tiles are painted by the
//! window into `Image2`), with a preview of the selected entry; a double
//! click loads it.

use super::{Mb3d, Ui};
use crate::history::{Action, Entry, Kind};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::canvas::{Canvas, Rect};
use crate::vcl::font::Font;
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Ev, Event, MouseButton};
use std::path::Path;

const F: &str = "HistoryFrm";

/// The views (`ComboBox2`).
const LIST: i32 = 0;
const ROWS: i32 = 1;
const TILES: i32 = 2;

/// A row: its picture box and height.
const ROW_PIC: (i32, i32) = (96, 72);
const ROW_H: i32 = 78;
/// A tile at the size slider's position: its picture box and its cell.
fn tile(ui: &Ui) -> (i32, i32, i32, i32) {
    let pw = 80 + 40 * ui.position(F, "TrackBar1").clamp(0, 4) as i32;
    let ph = pw * 3 / 4;
    (pw, ph, pw + 12, ph + 38)
}

/// An entry with what the window found out about its file.
struct Row {
    e: Entry,
    /// the file's size (None: gone)
    bytes: Option<u64>,
    /// the key of its contents ("Show unique")
    key: Option<u64>,
}

#[derive(Default)]
pub struct State {
    /// every entry, newest first
    all: Vec<Row>,
    /// the listed entries: indexes into `all`, in the list's order
    shown: Vec<usize>,
    /// the selected one (an index into `shown`; -1: none)
    sel: i32,
    /// the column sorted by and whether downwards
    sort: Option<(usize, bool)>,
    /// the view (`LIST`, `ROWS`, `TILES`), and the first row of rows or
    /// tiles in sight
    view: i32,
    top: i32,
    /// the previews' version the view was painted with
    painted: u64,
    /// the entry whose removal is being confirmed
    removing: Option<Entry>,
}

/// Opens the window (the icon's click), with the history read again.
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    if ui.showing(F) {
        refresh(app, ui);
    }
    ui.show(F);
}

fn refresh(app: &mut Mb3d, ui: &mut Ui) {
    app.history.all = crate::history::list().into_iter().map(|e| Row { bytes: e.bytes(), key: crate::history::content_key(&e.path), e }).collect();
    apply(app, ui);
}

fn action_text(a: Action) -> String {
    match a {
        Action::Calc => tr("calculated"),
        Action::Open => tr("opened"),
        Action::Save => tr("saved"),
    }
}

fn kind_text(k: Kind) -> String {
    match k {
        Kind::Params => tr("parameters"),
        Kind::M3i => tr("image + parameters"),
        Kind::Image => tr("image"),
        Kind::Animation => tr("animation"),
        Kind::Mesh => tr("mesh"),
        Kind::Formula => tr("formula"),
        Kind::Other => tr("file"),
    }
}

fn size_text(bytes: Option<u64>) -> String {
    match bytes {
        None => tr("missing"),
        Some(b) if b >= 10_000_000 => format!("{:.0} MB", b as f64 / 1e6),
        Some(b) if b >= 1_000_000 => format!("{:.1} MB", b as f64 / 1e6),
        Some(b) if b >= 1_000 => format!("{:.0} kB", b as f64 / 1e3),
        Some(b) => format!("{b} B"),
    }
}

fn folder_text(e: &Entry) -> String {
    if e.action == Action::Calc {
        return tr("History");
    }
    e.path.parent().map(|p| p.display().to_string()).unwrap_or_default()
}

fn name_text(e: &Entry) -> String {
    if e.title.is_empty() {
        tr("untitled")
    } else {
        e.title.clone()
    }
}

/// Whether the kind filter (`ComboBox1`) lets an entry through.
fn kind_wanted(choice: i32, e: &Entry) -> bool {
    match choice {
        1 => e.action == Action::Calc,
        2 => e.kind == Kind::Params,
        3 => matches!(e.kind, Kind::Image | Kind::M3i),
        4 => e.kind == Kind::Animation,
        5 => e.kind == Kind::Mesh,
        6 => matches!(e.kind, Kind::Formula | Kind::Other),
        _ => true,
    }
}

/// The list from the entries, the search text, the kind and the sort.
fn apply(app: &mut Mb3d, ui: &mut Ui) {
    let before = selected(app).map(|e| (e.secs, e.path));
    let search = ui.text(F, "Edit1").trim().to_lowercase();
    let choice = ui.item_index(F, "ComboBox1");
    let unique = ui.checked(F, "CheckBox1");
    // ("Show unique": of entries whose files have the same bytes only the
    // newest; the entries come newest first)
    let mut seen = std::collections::HashSet::new();
    let mut shown: Vec<usize> = (0..app.history.all.len())
        .filter(|&i| {
            let e = &app.history.all[i].e;
            if !kind_wanted(choice, e) {
                return false;
            }
            if unique {
                if let Some(k) = app.history.all[i].key {
                    if !seen.insert(k) {
                        return false;
                    }
                }
            }
            if search.is_empty() {
                return true;
            }
            let hay = format!("{} {} {} {}", name_text(e), e.path.display(), kind_text(e.kind), action_text(e.action)).to_lowercase();
            search.split_whitespace().all(|w| hay.contains(w))
        })
        .collect();
    if let Some((col, desc)) = app.history.sort {
        let all = &app.history.all;
        let key = |i: usize| -> (String, i64) {
            let (e, b) = (&all[i].e, &all[i].bytes);
            match col {
                1 => (action_text(e.action), 0),
                2 => (kind_text(e.kind), 0),
                3 => (name_text(e).to_lowercase(), 0),
                4 => (String::new(), b.map(|b| b as i64).unwrap_or(-1)),
                5 => (folder_text(e).to_lowercase(), 0),
                _ => (String::new(), e.secs),
            }
        };
        // (the entries come newest first: a stable sort keeps that within equal keys)
        shown.sort_by(|&a, &b| {
            let o = key(a).cmp(&key(b));
            if desc {
                o.reverse()
            } else {
                o
            }
        });
    }
    let rows: Vec<Vec<String>> = shown
        .iter()
        .map(|&i| {
            let (e, b) = (&app.history.all[i].e, &app.history.all[i].bytes);
            vec![e.when(), action_text(e.action), kind_text(e.kind), name_text(e), size_text(*b), folder_text(e)]
        })
        .collect();
    let sel = before.and_then(|(secs, path)| shown.iter().position(|&i| app.history.all[i].e.secs == secs && app.history.all[i].e.path == path));
    app.history.shown = shown;
    app.history.sel = match sel {
        Some(i) => i as i32,
        None if app.history.shown.is_empty() => -1,
        None => 0,
    };
    let c = ui.cm(F, "ListView1");
    c.checks.clear();
    c.cells = rows;
    c.item_index = app.history.sel;
    let n = app.history.shown.len();
    let text = if n == app.history.all.len() {
        trf("{} entries", &[&n])
    } else {
        trf("{} of {} entries", &[&n, &app.history.all.len()])
    };
    ui.set_caption(F, "Label5", &text);
    scroll_to_selection(app, ui);
    paint_view(app, ui);
    selection_changed(app, ui);
}

fn selected(app: &Mb3d) -> Option<Entry> {
    let i = app.history.sel;
    (i >= 0).then(|| app.history.shown.get(i as usize)).flatten().map(|&k| app.history.all[k].e.clone())
}

fn selected_bytes(app: &Mb3d) -> Option<u64> {
    let i = app.history.sel;
    (i >= 0).then(|| app.history.shown.get(i as usize)).flatten().and_then(|&k| app.history.all[k].bytes)
}

/// The preview and the details of the selected entry.
fn selection_changed(app: &mut Mb3d, ui: &mut Ui) {
    let Some(e) = selected(app) else {
        ui.set_picture(F, "Image1", None);
        ui.set_caption(F, "Label6", "");
        ui.set_text(F, "Edit2", "");
        ui.set_caption(F, "Label2", "");
        ui.set_caption(F, "Button1", &tr("Load"));
        ui.set_enabled(F, "Button1", false);
        ui.set_enabled(F, "Button2", false);
        ui.set_enabled(F, "Button3", false);
        return;
    };
    let bytes = selected_bytes(app);
    let there = bytes.is_some();
    let loads = matches!(e.kind, Kind::Params | Kind::M3i);
    ui.set_caption(F, "Button1", &if loads { tr("Load") } else { tr("Open") });
    ui.set_enabled(F, "Button1", there);
    ui.set_enabled(F, "Button2", e.path.parent().is_some_and(|d| d.is_dir()));
    ui.set_enabled(F, "Button3", true);
    ui.set_text(F, "Edit2", &e.path.display().to_string());
    let mut lines = vec![format!("{} {}", e.when(), action_text(e.action))];
    if there {
        lines.push(size_text(bytes));
    } else {
        lines.push(tr("The file is not there any more."));
    }
    ui.set_caption(F, "Label2", &lines.join("\n"));
    match preview_file(&e, there) {
        Some(p) => super::file_preview::request(app, ui, p, F, "Image1", "Label6"),
        None => {
            ui.set_picture(F, "Image1", None);
            ui.set_caption(F, "Label6", &tr("No preview for this file."));
        }
    }
}

/// The file a preview of an entry is made of: a calculated picture's
/// thumbnail, else the file itself (parameters are calculated small, an
/// image is scaled down).
fn preview_file(e: &Entry, there: bool) -> Option<std::path::PathBuf> {
    e.thumb().or_else(|| there.then(|| e.path.clone()))
}

// ---------------------------------------------------------------------------
// the rows and the tiles

/// Columns and rows of the view: (columns, rows in sight, rows in all).
fn geometry(app: &Mb3d, ui: &Ui) -> (i32, i32, i32) {
    let r = ui.c(F, "Image2").rect();
    let n = app.history.shown.len() as i32;
    match app.history.view {
        TILES => {
            let (_, _, tw, th) = tile(ui);
            let cols = (r.w / tw).max(1);
            (cols, (r.h / th).max(1), (n + cols - 1) / cols)
        }
        _ => (1, (r.h / ROW_H).max(1), n),
    }
}

/// The scroll bar and `top` after a change of the entries or the size.
fn set_scroll(app: &mut Mb3d, ui: &mut Ui) {
    let (_, vis, rows) = geometry(app, ui);
    let max = (rows - vis).max(0);
    app.history.top = app.history.top.clamp(0, max);
    let c = ui.cm(F, "ScrollBar1");
    c.min = 0;
    c.max = max as i64 + if max > 0 { vis as i64 - 1 } else { 0 };
    c.page_size = vis as i64;
    c.large_change = vis as i64;
    c.position = app.history.top as i64;
}

/// Scrolls so that the selected entry is in sight.
fn scroll_to_selection(app: &mut Mb3d, ui: &mut Ui) {
    if app.history.sel < 0 || app.history.view == LIST {
        return;
    }
    let (cols, vis, _) = geometry(app, ui);
    let row = app.history.sel / cols;
    if row < app.history.top {
        app.history.top = row;
    } else if row >= app.history.top + vis {
        app.history.top = row - vis + 1;
    }
}

/// The place of a picture `b` (or of the box) inside `r`, its proportions kept.
fn fit(r: Rect, w: usize, h: usize) -> Rect {
    if w == 0 || h == 0 {
        return r;
    }
    let f = (r.w as f64 / w as f64).min(r.h as f64 / h as f64);
    let (fw, fh) = ((w as f64 * f).round() as i32, (h as f64 * f).round() as i32);
    Rect::new(r.x + (r.w - fw) / 2, r.y + (r.h - fh) / 2, fw, fh)
}

/// Paints the rows or the tiles into `Image2`.
fn paint_view(app: &mut Mb3d, ui: &mut Ui) {
    if app.history.view == LIST {
        return;
    }
    set_scroll(app, ui);
    let r = ui.c(F, "Image2").rect();
    let (w, h) = (r.w.max(1) as usize, r.h.max(1) as usize);
    let (cols, vis, _) = geometry(app, ui);
    let (tile_pw, tile_ph, tile_w, tile_h) = tile(ui);
    let first = (app.history.top * cols) as usize;
    let last = (first + (vis * cols) as usize + cols as usize).min(app.history.shown.len());
    // the pictures of what is in sight, queued in that order
    super::file_preview::clear_queue(app);
    let mut pics: Vec<Option<Bitmap>> = Vec::with_capacity(last.saturating_sub(first));
    for i in first..last {
        let k = app.history.shown[i];
        let (e, b) = (&app.history.all[k].e, &app.history.all[k].bytes);
        let p = preview_file(e, b.is_some());
        pics.push(p.and_then(|p| super::file_preview::thumb(app, &p)));
    }
    let th = &ui.theme;
    let (bg, text, dim, pic_bg) = (th.window(), th.window_text(), th.text_disabled(), th.image_bg());
    let mut bm = Bitmap::new(w, h, bg);
    {
        let mut cv = Canvas::new(&mut bm.px, w, h, 1.0);
        let font = Font::default();
        let bold = Font { bold: true, ..Font::default() };
        let lh = cv.font_px(&font).ceil() as i32 + 3;
        for (j, i) in (first..last).enumerate() {
            let k = app.history.shown[i];
            let (e, bytes) = (&app.history.all[k].e, &app.history.all[k].bytes);
            let selected = i as i32 == app.history.sel;
            let (cell, pic_box) = if app.history.view == TILES {
                let (cx, cy) = ((j as i32 % cols) * tile_w, (j as i32 / cols) * tile_h);
                (Rect::new(cx, cy, tile_w, tile_h), Rect::new(cx + (tile_w - tile_pw) / 2, cy + 4, tile_pw, tile_ph))
            } else {
                let cy = j as i32 * ROW_H;
                (Rect::new(0, cy, w as i32, ROW_H), Rect::new(4, cy + (ROW_H - ROW_PIC.1) / 2, ROW_PIC.0, ROW_PIC.1))
            };
            if cell.y >= h as i32 {
                break;
            }
            if selected {
                th.selection(&mut cv, cell.inset(1), true);
            }
            let tc = if selected { th.selection_text(true) } else { text };
            let dc = if selected { th.selection_text(true) } else { dim };
            cv.fill(pic_box, pic_bg);
            match &pics[j] {
                Some(b) => cv.image_stretch(fit(pic_box, b.w, b.h), b, true),
                None => cv.frame(pic_box, dim),
            }
            let name = name_text(e);
            if app.history.view == TILES {
                // (the captions kept inside the tile; a narrow tile shows
                // the time alone)
                let old = cv.clip_to(cell);
                let tr_ = Rect::new(cell.x + 2, pic_box.bottom() + 2, cell.w - 4, lh);
                cv.text_in(tr_, &name, &bold, tc, crate::vcl::canvas::HAlign::Center, crate::vcl::canvas::VAlign::Top, false);
                let when = Rect::new(cell.x + 2, tr_.y + lh, cell.w - 4, lh);
                let full = format!("{} · {}", e.when(), action_text(e.action));
                let line = if cv.text_width(&font, &full) <= when.w { full } else { e.when() };
                let line = if cv.text_width(&font, &line) <= when.w { line } else { e.when()[11..].to_string() };
                cv.text_in(when, &line, &font, dc, crate::vcl::canvas::HAlign::Center, crate::vcl::canvas::VAlign::Top, false);
                cv.set_clip(old);
            } else {
                let x = pic_box.right() + 10;
                let y = cell.y + (ROW_H - 3 * lh) / 2;
                let tw = (w as i32 - x - 4).max(10);
                let line = |cv: &mut Canvas, row: i32, s: &str, f: &Font, c: u32| {
                    cv.text_in(Rect::new(x, y + row * lh, tw, lh), s, f, c, crate::vcl::canvas::HAlign::Left, crate::vcl::canvas::VAlign::Top, false);
                };
                line(&mut cv, 0, &format!("{name}   {}", kind_text(e.kind)), &bold, tc);
                line(&mut cv, 1, &format!("{} · {} · {}", e.when(), action_text(e.action), size_text(*bytes)), &font, tc);
                line(&mut cv, 2, &folder_text(e), &font, dc);
            }
        }
    }
    // (the canvas writes the window's format, without alpha: the picture
    // is opaque)
    for p in bm.px.iter_mut() {
        *p |= 0xFF00_0000;
    }
    ui.set_picture(F, "Image2", Some(bm));
    app.history.painted = app.preview.version;
}

/// The entry at a point of `Image2`.
fn index_at(app: &Mb3d, ui: &Ui, x: i32, y: i32) -> Option<i32> {
    let (cols, _, _) = geometry(app, ui);
    if x < 0 || y < 0 {
        return None;
    }
    let (col, row) = if app.history.view == TILES {
        let (_, _, tw, th) = tile(ui);
        (x / tw, y / th)
    } else {
        (0, y / ROW_H)
    };
    if col >= cols {
        return None;
    }
    let i = (app.history.top + row) * cols + col;
    ((i as usize) < app.history.shown.len()).then_some(i)
}

fn select(app: &mut Mb3d, ui: &mut Ui, i: i32) {
    if i == app.history.sel {
        return;
    }
    app.history.sel = i;
    ui.cm(F, "ListView1").item_index = i;
    paint_view(app, ui);
    selection_changed(app, ui);
}

fn scroll(app: &mut Mb3d, ui: &mut Ui, top: i32) {
    let (_, vis, rows) = geometry(app, ui);
    let top = top.clamp(0, (rows - vis).max(0));
    if top != app.history.top {
        app.history.top = top;
        paint_view(app, ui);
    }
}

/// `ComboBox2`: the list, the rows or the tiles.
fn set_view(app: &mut Mb3d, ui: &mut Ui, view: i32) {
    app.history.view = view;
    ui.set_visible(F, "ListView1", view == LIST);
    ui.set_visible(F, "Image2", view != LIST);
    ui.set_visible(F, "ScrollBar1", view != LIST);
    ui.set_visible(F, "Label4", view == LIST);
    ui.set_visible(F, "TrackBar1", view == TILES);
    if view != LIST {
        scroll_to_selection(app, ui);
        paint_view(app, ui);
    }
}

/// The idle work: the view painted again when previews came in.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if ui.showing(F) && app.history.view != LIST && app.history.painted != app.preview.version {
        paint_view(app, ui);
    }
}

// ---------------------------------------------------------------------------

/// Load (parameters, an image with them) or Open (any other file).
fn load(app: &mut Mb3d, ui: &mut Ui, e: &Entry) {
    if !e.path.is_file() {
        app.message(ui, &tr("The file is not there any more."));
        return;
    }
    if matches!(e.kind, Kind::Params | Kind::M3i) {
        super::main_form::open_file(app, ui, &e.path);
    } else {
        let _ = open::that_detached(&e.path);
    }
}

fn show_in_folder(p: &Path) {
    if let Some(d) = p.parent() {
        let _ = open::that_detached(d);
    }
}

fn remove(app: &mut Mb3d, ui: &mut Ui, e: &Entry) {
    if !crate::history::remove(e) {
        app.message(ui, &tr("The entry could not be removed."));
    }
    refresh(app, ui);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            set_view(app, ui, ui.item_index(F, "ComboBox2").max(0));
            refresh(app, ui);
        }
        "FormResize" => paint_view(app, ui),
        "Edit1Change" | "ComboBox1Change" | "CheckBox1Click" => apply(app, ui),
        "ComboBox2Change" => set_view(app, ui, ui.item_index(F, "ComboBox2").max(0)),
        "TrackBar1Change" => {
            scroll_to_selection(app, ui);
            paint_view(app, ui);
        }
        "ListView1Change" => {
            let i = ui.item_index(F, "ListView1");
            if i != app.history.sel {
                app.history.sel = i;
                selection_changed(app, ui);
            }
        }
        "ListView1ColumnClick" => {
            let col = ui.c(F, "ListView1").col.max(0) as usize;
            app.history.sort = Some(match app.history.sort {
                Some((c, desc)) if c == col => (col, !desc),
                // (the time: newest first at the first click)
                _ => (col, col == 0),
            });
            apply(app, ui);
        }
        "Image2MouseDown" => {
            if let Ev::MouseDown { x, y, button, .. } = &e.ev {
                if matches!(button, MouseButton::Left | MouseButton::Right) {
                    if let Some(i) = index_at(app, ui, *x, *y) {
                        select(app, ui, i);
                    }
                }
            }
        }
        "Image2MouseWheel" => {
            if let Ev::Wheel { delta, .. } = &e.ev {
                let top = app.history.top - delta / 120 * if app.history.view == ROWS { 3 } else { 1 };
                scroll(app, ui, top);
                ui.set_position(F, "ScrollBar1", app.history.top as i64);
            }
        }
        "ScrollBar1Change" | "ScrollBar1Scroll" => {
            let p = ui.position(F, "ScrollBar1") as i32;
            scroll(app, ui, p);
        }
        "ListView1DblClick" | "Image2DblClick" | "Button1Click" => {
            if let Some(en) = selected(app) {
                load(app, ui, &en);
            }
        }
        "Button2Click" => {
            if let Some(en) = selected(app) {
                show_in_folder(&en.path);
            }
        }
        "Button3Click" => {
            let Some(en) = selected(app) else { return };
            if en.action == Action::Calc {
                app.history.removing = Some(en.clone());
                ui.confirm("history:remove", &trf("Delete the parameters and the thumbnail of \"{}\" from the history?", &[&name_text(&en)]));
            } else {
                remove(app, ui, &en);
            }
        }
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("remove", DialogResult::Button(crate::vcl::form::MR_YES)) = (what, r) {
        if let Some(en) = app.history.removing.take() {
            remove(app, ui, &en);
        }
    }
}
