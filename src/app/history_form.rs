//! The History window (`HistoryFrm`, this program's own, the clock icon of
//! the main window's title bar): what [`crate::history`] keeps, the
//! calculated pictures with their thumbnails and the files opened and
//! saved, searchable, sortable by its columns, as a list, as rows with
//! their pictures or as tiles (the rows and the tiles are painted by the
//! window into `Image2`), grouped by day (a day's entries and pictures
//! show when it is opened) or in one run, with a preview of the selected
//! entry; a double click loads it.  Made for a history of any size: the
//! days are read on a thread and listed as they come (the newest first),
//! the list view holds a window of its rows only, the files are compared
//! ("Show unique") on a thread with their keys kept between sessions,
//! and the pictures are made for what is in sight.

use super::{Mb3d, Ui};
use crate::history::{Action, Day, Entry, Found, Keys, Kind, Scan};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::canvas::{Canvas, HAlign, Rect, VAlign};
use crate::vcl::font::Font;
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Ev, Event, MouseButton};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const F: &str = "HistoryFrm";

/// The views (`ComboBox2`).
const LIST: i32 = 0;
const ROWS: i32 = 1;
const TILES: i32 = 2;

/// A row: its picture box and height.
const ROW_PIC: (i32, i32) = (96, 72);
const ROW_H: i32 = 78;
/// A day's band in the rows and the tiles.
const DAY_H: i32 = 26;
/// The rows of the list view filled beyond those in sight (it holds a
/// window of its rows only).
const FILL: usize = 200;
/// Files compared per run of the comparing thread.
const HASH_BATCH: usize = 2000;
/// The list is made again at most so often while entries and keys come in.
const RELIST: Duration = Duration::from_millis(300);

/// A tile at the size slider's position: its picture box and its cell.
fn tile(ui: &Ui) -> (i32, i32, i32, i32) {
    let pw = 80 + 40 * ui.position(F, "TrackBar1").clamp(0, 4) as i32;
    let ph = pw * 3 / 4;
    (pw, ph, pw + 12, ph + 38)
}

/// An entry with its search text.
struct Row {
    e: Entry,
    /// the name, folder, kind and action in lower case, what the search
    /// looks through
    hay: String,
}

/// A day of the history: its entries are `all[first..first + n]`.
struct Group {
    day: Day,
    first: usize,
    n: usize,
    /// read by the scan
    scanned: bool,
    /// the entries the search and the kind let through
    matches: usize,
}

/// A listed item: a day's heading (the index of the group) or an entry
/// (the index into `all`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Item {
    Day(usize),
    Entry(usize),
}

/// A row of the rows or the tiles view: the items `shown[first..first +
/// n]`, a day's band (`day`) or entries side by side.
struct LRow {
    first: usize,
    n: usize,
    day: bool,
}

#[derive(Default)]
pub struct State {
    /// the days, newest first
    groups: Vec<Group>,
    /// every entry read so far, newest first
    all: Vec<Row>,
    /// the thread reading the history, while it runs
    scan: Option<Scan>,
    /// the days opened (their entries listed) when grouped
    open: HashSet<String>,
    /// the listed items, in the list's order
    shown: Vec<Item>,
    /// how many of `shown` are entries
    shown_entries: usize,
    /// the selected one (an index into `shown`; -1: none)
    sel: i32,
    /// the column sorted by and whether downwards
    sort: Option<(usize, bool)>,
    /// the view (`LIST`, `ROWS`, `TILES`), and the first row of rows or
    /// tiles in sight
    view: i32,
    top: i32,
    /// the rows of the rows or tiles view, and the columns they were laid
    /// out for (-1: to be made)
    layout: Vec<LRow>,
    layout_cols: i32,
    /// the previews' version the view was painted with
    painted: u64,
    /// the entry whose removal is being confirmed
    removing: Option<Entry>,
    /// the keys of the files' contents ("Show unique"), the thread
    /// comparing files (set while it runs) and how many keys there were
    /// at the last listing
    keys: Option<Arc<Mutex<Keys>>>,
    hashing: Option<Arc<AtomicBool>>,
    keys_seen: usize,
    /// when the list was made, and whether it is to be made again
    listed_at: Option<Instant>,
    stale: bool,
}

/// Opens the window (the icon's click), with the history read again.
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    if ui.showing(F) {
        refresh(app, ui);
    }
    ui.show(F);
}

/// Something to call from a thread that wakes the window.
fn waker(ui: &Ui) -> Box<dyn Fn() + Send> {
    let w = ui.waker();
    Box::new(move || {
        if let Some(w) = &w {
            w.wake();
        }
    })
}

/// Reads the history again, on its thread; the list fills as it comes.
fn refresh(app: &mut Mb3d, ui: &mut Ui) {
    let h = &mut app.history;
    h.groups.clear();
    h.all.clear();
    h.shown.clear();
    h.shown_entries = 0;
    h.sel = -1;
    h.scan = Some(Scan::start(waker(ui)));
    apply(app, ui);
}

/// Takes in what the scan found; the list is made again now and then.
fn receive(app: &mut Mb3d) {
    let Some(scan) = app.history.scan.take() else { return };
    let mut done = false;
    // (a slice of the days per call: the window stays alive)
    for _ in 0..50 {
        let Some(found) = scan.next() else { break };
        app.history.stale = true;
        match found {
            Found::Days(days) => {
                app.history.groups = days.into_iter().map(|day| Group { day, first: 0, n: 0, scanned: false, matches: 0 }).collect();
                // (the newest day opened when none is)
                if app.history.open.is_empty() {
                    if let Some(g) = app.history.groups.first() {
                        app.history.open.insert(g.day.date.clone());
                    }
                }
            }
            Found::Day(i, entries) => {
                // (the days come newest first: the entries are appended in order)
                let first = app.history.all.len();
                app.history.all.extend(entries.into_iter().map(|e| Row { hay: hay(&e), e }));
                if let Some(g) = app.history.groups.get_mut(i) {
                    g.first = first;
                    g.n = app.history.all.len() - first;
                    g.scanned = true;
                }
            }
            Found::Done => done = true,
        }
    }
    if !done {
        app.history.scan = Some(scan);
    }
}

fn hay(e: &Entry) -> String {
    format!("{} {} {} {}", name_text(e), e.path.display(), kind_text(e.kind), action_text(e.action)).to_lowercase()
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

/// A day's heading: the arrow of its state and its date.
fn day_text(app: &Mb3d, gi: usize) -> String {
    let g = &app.history.groups[gi];
    format!("{} {}", if app.history.open.contains(&g.day.date) { "▼" } else { "►" }, g.day.date)
}

/// How many entries a day has, of them listed (or to be listed when
/// it is opened).
fn count_text(app: &Mb3d, gi: usize) -> String {
    let g = &app.history.groups[gi];
    if !g.scanned {
        "...".into()
    } else if g.matches == g.n {
        trf("{} entries", &[&g.n])
    } else {
        trf("{} of {} entries", &[&g.matches, &g.n])
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

/// `idx` in the order of the sort column (the entries come newest
/// first: a stable sort keeps that within equal keys; the keys are made
/// once per entry).
fn sorted(app: &Mb3d, mut idx: Vec<usize>) -> Vec<usize> {
    let Some((col, desc)) = app.history.sort else { return idx };
    let all = &app.history.all;
    let key = |i: usize| -> (String, i64) {
        let e = &all[i].e;
        match col {
            1 => (action_text(e.action), 0),
            2 => (kind_text(e.kind), 0),
            3 => (name_text(e).to_lowercase(), 0),
            4 => (String::new(), e.bytes.map(|b| b as i64).unwrap_or(-1)),
            5 => (folder_text(e).to_lowercase(), 0),
            _ => (String::new(), e.secs),
        }
    };
    if desc {
        idx.sort_by_cached_key(|&i| std::cmp::Reverse(key(i)));
    } else {
        idx.sort_by_cached_key(|&i| key(i));
    }
    idx
}

/// What identifies the selected item across a new listing.
fn sel_mark(app: &Mb3d) -> Option<(Option<String>, i64, PathBuf)> {
    match item(app, app.history.sel)? {
        Item::Day(gi) => Some((Some(app.history.groups[gi].day.date.clone()), 0, PathBuf::new())),
        Item::Entry(i) => {
            let e = &app.history.all[i].e;
            Some((None, e.secs, e.path.clone()))
        }
    }
}

/// The list from the entries, the search text, the kind, the grouping,
/// "Show unique" and the sort.
fn apply(app: &mut Mb3d, ui: &mut Ui) {
    app.history.listed_at = Some(Instant::now());
    app.history.stale = false;
    let before = sel_mark(app);
    let search = ui.text(F, "Edit1").trim().to_lowercase();
    let words: Vec<&str> = search.split_whitespace().collect();
    let choice = ui.item_index(F, "ComboBox1");
    let unique = ui.checked(F, "CheckBox1");
    let grouped = ui.checked(F, "CheckBox2");
    // (the keys of the files' contents: read at the first use)
    if unique && app.history.keys.is_none() {
        app.history.keys = Some(Arc::new(Mutex::new(Keys::load())));
    }
    let keys = if unique { app.history.keys.clone() } else { None };
    let mut shown: Vec<Item> = Vec::new();
    let mut n_entries = 0;
    let mut matches: Vec<usize> = Vec::with_capacity(app.history.groups.len());
    // the files whose keys are not known yet, to be compared
    let mut missing: Vec<(PathBuf, u64, i64)> = Vec::new();
    {
        let keys = keys.as_ref().map(|k| k.lock().unwrap());
        let all = &app.history.all;
        let passes = |i: usize| -> bool {
            let r = &all[i];
            kind_wanted(choice, &r.e) && words.iter().all(|w| r.hay.contains(w))
        };
        // ("Show unique": of entries whose files have the same bytes only the
        // newest; the entries come newest first)
        let mut seen = HashSet::new();
        let mut unique_ok = |i: usize, missing: &mut Vec<(PathBuf, u64, i64)>| -> bool {
            let Some(keys) = &keys else { return true };
            let e = &all[i].e;
            let Some(b) = e.bytes else { return true };
            match keys.get(&e.path, b, e.modified) {
                Some(Some(k)) => seen.insert(k),
                Some(None) => true,
                None => {
                    if missing.len() < HASH_BATCH {
                        missing.push((e.path.clone(), b, e.modified));
                    }
                    true
                }
            }
        };
        if grouped {
            // (the closed days go through "Show unique" too: their counts
            // tell what opening them would list, and a file seen on a
            // newer day hides its copies on the older ones)
            for (gi, g) in app.history.groups.iter().enumerate() {
                let idx: Vec<usize> = (g.first..g.first + g.n).filter(|&i| passes(i) && unique_ok(i, &mut missing)).collect();
                matches.push(idx.len());
                // (a search hides the days without a hit)
                if !words.is_empty() && idx.is_empty() && g.scanned {
                    continue;
                }
                shown.push(Item::Day(gi));
                if app.history.open.contains(&g.day.date) {
                    n_entries += idx.len();
                    shown.extend(sorted(app, idx).into_iter().map(Item::Entry));
                }
            }
        } else {
            let idx: Vec<usize> = (0..all.len()).filter(|&i| passes(i) && unique_ok(i, &mut missing)).collect();
            n_entries = idx.len();
            shown.extend(sorted(app, idx).into_iter().map(Item::Entry));
        }
    }
    for (g, m) in app.history.groups.iter_mut().zip(matches) {
        g.matches = m;
    }
    // the files not compared yet, compared now (a batch at a time)
    if !missing.is_empty() && app.history.hashing.is_none() {
        if let Some(k) = &keys {
            app.history.keys_seen = k.lock().unwrap().len();
            app.history.hashing = Some(crate::history::hash_keys(k.clone(), missing, waker(ui)));
        }
    }
    // the selection kept, else the first entry
    let sel = before.and_then(|(date, secs, path)| {
        shown.iter().position(|it| match it {
            Item::Day(gi) => date.as_deref() == Some(app.history.groups[*gi].day.date.as_str()),
            Item::Entry(i) => date.is_none() && app.history.all[*i].e.secs == secs && app.history.all[*i].e.path == path,
        })
    });
    let sel = sel.or_else(|| shown.iter().position(|it| matches!(it, Item::Entry(_))));
    app.history.shown = shown;
    app.history.shown_entries = n_entries;
    app.history.sel = sel.map_or(-1, |i| i as i32);
    app.history.layout_cols = -1;
    {
        let c = ui.cm(F, "ListView1");
        c.checks.clear();
        c.item_index = app.history.sel;
        c.virtual_rows = app.history.shown.len();
        c.cells.clear();
        c.cells_from = 0;
    }
    ui.listview_show_selection(F, "ListView1");
    fill_cells(app, ui, true);
    let n = app.history.shown_entries;
    let total = app.history.all.len();
    let mut text = if app.history.scan.is_some() {
        trf("Reading the history... {} entries", &[&total])
    } else if n == total {
        trf("{} entries", &[&n])
    } else {
        trf("{} of {} entries", &[&n, &total])
    };
    if app.history.hashing.is_some() {
        text = format!("{text} · {}", tr("comparing files"));
    }
    ui.set_caption(F, "Label5", &text);
    scroll_to_selection(app, ui);
    paint_view(app, ui);
    selection_changed(app, ui);
}

/// The cells of a listed item.
fn cells_of(app: &Mb3d, it: Item) -> Vec<String> {
    match it {
        Item::Day(gi) => vec![day_text(app, gi), String::new(), String::new(), count_text(app, gi), String::new(), String::new()],
        Item::Entry(i) => {
            let e = &app.history.all[i].e;
            vec![e.when(), action_text(e.action), kind_text(e.kind), name_text(e), size_text(e.bytes), folder_text(e)]
        }
    }
}

/// Fills the list view's window of rows: those in sight and [`FILL`]
/// beyond, when the sight left the rows filled (or `force`).
fn fill_cells(app: &mut Mb3d, ui: &mut Ui, force: bool) {
    let (top, vis) = ui.listview_window(F, "ListView1");
    let n = app.history.shown.len();
    {
        let c = ui.c(F, "ListView1");
        let have = c.cells_from..c.cells_from + c.cells.len();
        if !force && have.start <= top && (top + vis).min(n) <= have.end {
            return;
        }
    }
    let from = top.saturating_sub(FILL);
    let to = (top + vis + FILL).min(n);
    let rows: Vec<Vec<String>> = (from..to.max(from)).map(|i| cells_of(app, app.history.shown[i])).collect();
    let c = ui.cm(F, "ListView1");
    c.cells = rows;
    c.cells_from = from;
    c.virtual_rows = n;
}

/// The item at an index of `shown`.
fn item(app: &Mb3d, i: i32) -> Option<Item> {
    (i >= 0).then(|| app.history.shown.get(i as usize)).flatten().copied()
}

/// The selected entry (None for a day's heading or nothing).
fn selected(app: &Mb3d) -> Option<Entry> {
    match item(app, app.history.sel)? {
        Item::Entry(i) => Some(app.history.all[i].e.clone()),
        Item::Day(_) => None,
    }
}

/// The preview and the details of the selected item.
fn selection_changed(app: &mut Mb3d, ui: &mut Ui) {
    let it = item(app, app.history.sel);
    let Some(it) = it else {
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
    let e = match it {
        Item::Day(gi) => {
            // a day: its folder and count
            let g = &app.history.groups[gi];
            let lines = vec![g.day.date.clone(), count_text(app, gi)];
            let dir = g.day.path.display().to_string();
            ui.set_picture(F, "Image1", None);
            ui.set_caption(F, "Label6", "");
            ui.set_text(F, "Edit2", &dir);
            ui.set_caption(F, "Label2", &lines.join("\n"));
            ui.set_caption(F, "Button1", &tr("Load"));
            ui.set_enabled(F, "Button1", false);
            ui.set_enabled(F, "Button2", Path::new(&dir).is_dir());
            ui.set_enabled(F, "Button3", false);
            return;
        }
        Item::Entry(i) => app.history.all[i].e.clone(),
    };
    let there = e.bytes.is_some();
    let loads = matches!(e.kind, Kind::Params | Kind::M3i);
    ui.set_caption(F, "Button1", &if loads { tr("Load") } else { tr("Open") });
    ui.set_enabled(F, "Button1", there);
    ui.set_enabled(F, "Button2", e.path.parent().is_some_and(|d| d.is_dir()));
    ui.set_enabled(F, "Button3", true);
    ui.set_text(F, "Edit2", &e.path.display().to_string());
    let mut lines = vec![format!("{} {}", e.when(), action_text(e.action))];
    if there {
        lines.push(size_text(e.bytes));
    } else {
        lines.push(tr("The file is not there any more."));
    }
    ui.set_caption(F, "Label2", &lines.join("\n"));
    match preview_file(&e) {
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
fn preview_file(e: &Entry) -> Option<PathBuf> {
    e.thumb().or_else(|| e.bytes.is_some().then(|| e.path.clone()))
}

/// Opens or closes a day.
fn toggle(app: &mut Mb3d, ui: &mut Ui, gi: usize) {
    let date = app.history.groups[gi].day.date.clone();
    if !app.history.open.remove(&date) {
        app.history.open.insert(date);
    }
    apply(app, ui);
}

// ---------------------------------------------------------------------------
// the rows and the tiles

/// The columns of the view.
fn cols(app: &Mb3d, ui: &Ui) -> i32 {
    if app.history.view == TILES {
        let (_, _, tw, _) = tile(ui);
        (ui.c(F, "Image2").rect().w / tw).max(1)
    } else {
        1
    }
}

/// The rows of the view from the listed items, for the columns there are.
fn ensure_layout(app: &mut Mb3d, ui: &Ui) {
    let cols = cols(app, ui) as usize;
    if app.history.layout_cols == cols as i32 {
        return;
    }
    let shown = &app.history.shown;
    let mut rows = Vec::new();
    let mut i = 0;
    while i < shown.len() {
        if let Item::Day(_) = shown[i] {
            rows.push(LRow { first: i, n: 1, day: true });
            i += 1;
            continue;
        }
        let start = i;
        while i < shown.len() && i - start < cols && matches!(shown[i], Item::Entry(_)) {
            i += 1;
        }
        rows.push(LRow { first: start, n: i - start, day: false });
    }
    app.history.layout = rows;
    app.history.layout_cols = cols as i32;
}

/// A row's height.
fn row_h(app: &Mb3d, ui: &Ui, r: usize) -> i32 {
    if app.history.layout[r].day {
        DAY_H
    } else if app.history.view == TILES {
        tile(ui).3
    } else {
        ROW_H
    }
}

/// How many rows from `top` start within the view's height (one at least).
fn rows_in_sight(app: &Mb3d, ui: &Ui, top: i32, h: i32) -> i32 {
    let (mut y, mut n) = (0, 0);
    for r in top.max(0) as usize..app.history.layout.len() {
        if y >= h && n > 0 {
            break;
        }
        y += row_h(app, ui, r);
        n += 1;
    }
    n.max(1)
}

/// The largest first row: from it the rows fill the height.
fn max_top(app: &Mb3d, ui: &Ui, h: i32) -> i32 {
    let n = app.history.layout.len();
    let (mut y, mut r) = (0, n);
    while r > 0 {
        let rh = row_h(app, ui, r - 1);
        if y + rh > h {
            break;
        }
        y += rh;
        r -= 1;
    }
    (r as i32).min(n as i32 - 1).max(0)
}

/// The row of a listed item.
fn row_of(app: &Mb3d, i: i32) -> i32 {
    app.history.layout.partition_point(|r| r.first + r.n <= i as usize) as i32
}

/// The scroll bar and `top` after a change of the entries or the size.
fn set_scroll(app: &mut Mb3d, ui: &mut Ui) {
    ensure_layout(app, ui);
    let h = ui.c(F, "Image2").rect().h;
    let max = max_top(app, ui, h);
    app.history.top = app.history.top.clamp(0, max);
    let vis = rows_in_sight(app, ui, app.history.top, h);
    let c = ui.cm(F, "ScrollBar1");
    c.min = 0;
    c.max = max as i64 + if max > 0 { vis as i64 - 1 } else { 0 };
    c.page_size = vis as i64;
    c.large_change = vis as i64;
    c.position = app.history.top as i64;
}

/// Scrolls so that the selected item is in sight.
fn scroll_to_selection(app: &mut Mb3d, ui: &mut Ui) {
    if app.history.sel < 0 || app.history.view == LIST {
        return;
    }
    ensure_layout(app, ui);
    let h = ui.c(F, "Image2").rect().h;
    let row = row_of(app, app.history.sel);
    if row < app.history.top {
        app.history.top = row;
    } else if row >= app.history.top + rows_in_sight(app, ui, app.history.top, h) {
        // (the rows up from the selected one that fit)
        let mut t = row as usize;
        let mut y = row_h(app, ui, t);
        while t > 0 && y + row_h(app, ui, t - 1) <= h {
            t -= 1;
            y += row_h(app, ui, t);
        }
        app.history.top = t as i32;
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

/// An arrow of a day's band: down when open, right when closed.
fn draw_arrow(cv: &mut Canvas, x: i32, y: i32, open: bool, col: u32) {
    for k in 0..5 {
        if open {
            cv.hline(x + k, x + 9 - k, y + 3 + k, col);
        } else {
            cv.vline(x + 2 + k, y + k, y + 9 - k, col);
        }
    }
}

/// Paints the rows or the tiles into `Image2`.
fn paint_view(app: &mut Mb3d, ui: &mut Ui) {
    if app.history.view == LIST {
        return;
    }
    set_scroll(app, ui);
    let r = ui.c(F, "Image2").rect();
    let (w, h) = (r.w.max(1) as usize, r.h.max(1) as usize);
    let (tile_pw, tile_ph, tile_w, _) = tile(ui);
    let top = app.history.top.max(0) as usize;
    // the rows in sight: (row, y)
    let mut rows: Vec<(usize, i32)> = Vec::new();
    let mut y = 0;
    for ri in top..app.history.layout.len() {
        if y >= h as i32 {
            break;
        }
        rows.push((ri, y));
        y += row_h(app, ui, ri);
    }
    // the pictures of the entries in sight, queued in that order
    super::file_preview::clear_queue(app);
    let mut pics: Vec<(usize, Option<Bitmap>)> = Vec::new();
    for &(ri, _) in &rows {
        let lr = &app.history.layout[ri];
        if lr.day {
            continue;
        }
        for i in lr.first..lr.first + lr.n {
            let Item::Entry(k) = app.history.shown[i] else { continue };
            let p = preview_file(&app.history.all[k].e);
            let b = p.and_then(|p| super::file_preview::thumb(app, &p));
            pics.push((i, b));
        }
    }
    let th = &ui.theme;
    let (bg, text, dim, pic_bg, band) = (th.window(), th.window_text(), th.text_disabled(), th.image_bg(), th.face());
    let mut bm = Bitmap::new(w, h, bg);
    {
        let mut cv = Canvas::new(&mut bm.px, w, h, 1.0);
        let font = Font::default();
        let bold = Font { bold: true, ..Font::default() };
        let lh = cv.font_px(&font).ceil() as i32 + 3;
        for &(ri, y) in &rows {
            let (first, n, day) = {
                let lr = &app.history.layout[ri];
                (lr.first, lr.n, lr.day)
            };
            if day {
                let Item::Day(gi) = app.history.shown[first] else { continue };
                let selected = first as i32 == app.history.sel;
                let cell = Rect::new(0, y, w as i32, DAY_H);
                cv.fill(cell, band);
                cv.hline(0, w as i32, y + DAY_H - 1, dim);
                if selected {
                    th.selection(&mut cv, cell.inset(1), true);
                }
                let tc = if selected { th.selection_text(true) } else { text };
                let dc = if selected { th.selection_text(true) } else { dim };
                let g = &app.history.groups[gi];
                draw_arrow(&mut cv, 8, y + (DAY_H - 10) / 2, app.history.open.contains(&g.day.date), tc);
                let dw = cv.text_width(&bold, &g.day.date);
                cv.text_in(Rect::new(24, y, dw + 4, DAY_H), &g.day.date, &bold, tc, HAlign::Left, VAlign::Center, false);
                cv.text_in(Rect::new(32 + dw, y, w as i32 - 36 - dw, DAY_H), &count_text(app, gi), &font, dc, HAlign::Left, VAlign::Center, false);
                continue;
            }
            for j in 0..n {
                let i = first + j;
                let Item::Entry(k) = app.history.shown[i] else { continue };
                let e = &app.history.all[k].e;
                let selected = i as i32 == app.history.sel;
                let (cell, pic_box) = if app.history.view == TILES {
                    let cx = j as i32 * tile_w;
                    let (_, _, _, tile_h) = tile(ui);
                    (Rect::new(cx, y, tile_w, tile_h), Rect::new(cx + (tile_w - tile_pw) / 2, y + 4, tile_pw, tile_ph))
                } else {
                    (Rect::new(0, y, w as i32, ROW_H), Rect::new(4, y + (ROW_H - ROW_PIC.1) / 2, ROW_PIC.0, ROW_PIC.1))
                };
                if selected {
                    th.selection(&mut cv, cell.inset(1), true);
                }
                let tc = if selected { th.selection_text(true) } else { text };
                let dc = if selected { th.selection_text(true) } else { dim };
                cv.fill(pic_box, pic_bg);
                match pics.iter().find(|(pi, _)| *pi == i).and_then(|(_, b)| b.as_ref()) {
                    Some(b) => cv.image_stretch(fit(pic_box, b.w, b.h), b, true),
                    None => cv.frame(pic_box, dim),
                }
                let name = name_text(e);
                if app.history.view == TILES {
                    // (the captions kept inside the tile; a narrow tile shows
                    // the time alone)
                    let old = cv.clip_to(cell);
                    let tr_ = Rect::new(cell.x + 2, pic_box.bottom() + 2, cell.w - 4, lh);
                    cv.text_in(tr_, &name, &bold, tc, HAlign::Center, VAlign::Top, false);
                    let when = Rect::new(cell.x + 2, tr_.y + lh, cell.w - 4, lh);
                    let full = format!("{} · {}", e.when(), action_text(e.action));
                    let line = if cv.text_width(&font, &full) <= when.w { full } else { e.when() };
                    let line = if cv.text_width(&font, &line) <= when.w { line } else { e.when()[11..].to_string() };
                    cv.text_in(when, &line, &font, dc, HAlign::Center, VAlign::Top, false);
                    cv.set_clip(old);
                } else {
                    let x = pic_box.right() + 10;
                    let ty = cell.y + (ROW_H - 3 * lh) / 2;
                    let tw = (w as i32 - x - 4).max(10);
                    let line = |cv: &mut Canvas, row: i32, s: &str, f: &Font, c: u32| {
                        cv.text_in(Rect::new(x, ty + row * lh, tw, lh), s, f, c, HAlign::Left, VAlign::Top, false);
                    };
                    line(&mut cv, 0, &format!("{name}   {}", kind_text(e.kind)), &bold, tc);
                    line(&mut cv, 1, &format!("{} · {} · {}", e.when(), action_text(e.action), size_text(e.bytes)), &font, tc);
                    line(&mut cv, 2, &folder_text(e), &font, dc);
                }
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

/// The item at a point of `Image2`.
fn index_at(app: &mut Mb3d, ui: &Ui, x: i32, y: i32) -> Option<i32> {
    if x < 0 || y < 0 {
        return None;
    }
    ensure_layout(app, ui);
    let (_, _, tile_w, _) = tile(ui);
    let mut ry = 0;
    for ri in app.history.top.max(0) as usize..app.history.layout.len() {
        let rh = row_h(app, ui, ri);
        if y < ry + rh {
            let lr = &app.history.layout[ri];
            let col = if lr.day || app.history.view != TILES { 0 } else { (x / tile_w) as usize };
            return (col < lr.n).then_some((lr.first + col) as i32);
        }
        ry += rh;
    }
    None
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
    ensure_layout(app, ui);
    let h = ui.c(F, "Image2").rect().h;
    let top = top.clamp(0, max_top(app, ui, h));
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
    app.history.layout_cols = -1;
    if view != LIST {
        scroll_to_selection(app, ui);
        paint_view(app, ui);
    }
}

/// The idle work: what the scan and the comparing thread brought, the
/// list made again for it, the view painted again when previews came
/// in, the list view's rows filled where it was scrolled to.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.showing(F) {
        // (a closed window stops the reading)
        app.history.scan = None;
        return;
    }
    receive(app);
    if let Some(h) = &app.history.hashing {
        if let Some(k) = &app.history.keys {
            let n = k.lock().unwrap().len();
            if n != app.history.keys_seen {
                app.history.keys_seen = n;
                app.history.stale = true;
            }
        }
        if !h.load(Ordering::Relaxed) {
            app.history.hashing = None;
            app.history.stale = true;
        }
    }
    if app.history.stale {
        let settled = app.history.scan.is_none() && app.history.hashing.is_none();
        if settled || app.history.listed_at.is_none_or(|t| t.elapsed() >= RELIST) {
            apply(app, ui);
        }
    }
    if app.history.view == LIST {
        fill_cells(app, ui, false);
    } else if app.history.painted != app.preview.version {
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

/// Takes an entry out of the history and the list (no reading again).
fn remove(app: &mut Mb3d, ui: &mut Ui, e: &Entry) {
    if !crate::history::remove(e) {
        app.message(ui, &tr("The entry could not be removed."));
        return;
    }
    if let Some(i) = app.history.all.iter().position(|r| r.e.secs == e.secs && r.e.path == e.path) {
        app.history.all.remove(i);
        for g in app.history.groups.iter_mut() {
            if g.first > i {
                g.first -= 1;
            } else if i < g.first + g.n {
                g.n -= 1;
            }
        }
    }
    apply(app, ui);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            set_view(app, ui, ui.item_index(F, "ComboBox2").max(0));
            refresh(app, ui);
        }
        "FormResize" => paint_view(app, ui),
        "Edit1Change" | "ComboBox1Change" | "CheckBox1Click" | "CheckBox2Click" => apply(app, ui),
        "ComboBox2Change" => set_view(app, ui, ui.item_index(F, "ComboBox2").max(0)),
        "TrackBar1Change" => {
            app.history.layout_cols = -1;
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
        "ListView1Click" => {
            // a day's heading opens or closes the day
            if let Some(Item::Day(gi)) = item(app, app.history.sel) {
                toggle(app, ui, gi);
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
                        if let (MouseButton::Left, Some(Item::Day(gi))) = (button, item(app, i)) {
                            toggle(app, ui, gi);
                        }
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
        "Button2Click" => match item(app, app.history.sel) {
            Some(Item::Entry(_)) => {
                if let Some(en) = selected(app) {
                    show_in_folder(&en.path);
                }
            }
            Some(Item::Day(gi)) => {
                let _ = open::that_detached(&app.history.groups[gi].day.path);
            }
            None => {}
        },
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
