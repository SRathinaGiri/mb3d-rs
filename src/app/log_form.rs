//! The Log window (`LogFrm`, this program's own, the page icon of the main
//! window's title bar): the log ([`crate::log`]) of the running session,
//! growing as things happen, or of an earlier day; filtered by level, kind
//! and words, the selected line in full below the list.  The list view
//! holds the rows in sight only, as the History window's.

use super::{Mb3d, Ui};
use crate::log::{Entry, Level};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const F: &str = "LogFrm";
/// The rows of the list view filled beyond those in sight.
const FILL: usize = 200;
/// The live list is read again at most so often.
const RELOAD: Duration = Duration::from_millis(400);

/// The kinds of `ComboBox3`, in its order (after "All kinds").
const KINDS: [&[&str]; 7] = [&["ui"], &["calc"], &["message"], &["file"], &["window"], &["console"], &["session", "crash"]];

#[derive(Default)]
pub struct State {
    /// the lines of the day or the session shown, oldest first
    entries: Vec<Entry>,
    /// the listed lines: indexes into `entries`
    shown: Vec<usize>,
    /// the earlier days' files (newest first), `ComboBox2`'s items after
    /// the first
    days: Vec<(String, PathBuf)>,
    /// the log's version the live list was read at, and when
    ver: u64,
    loaded: Option<Instant>,
}

/// Opens the window (the icon's click).
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    if ui.showing(F) {
        reload(app, ui);
    }
    ui.show(F);
}

fn live(ui: &Ui) -> bool {
    ui.item_index(F, "ComboBox2") <= 0
}

/// The days for `ComboBox2`.
fn fill_days(app: &mut Mb3d, ui: &mut Ui) {
    app.logview.days = crate::log::days();
    let keep = ui.item_index(F, "ComboBox2").max(0);
    let mut items = vec![tr("@MB:ThisSessionLive")];
    items.extend(app.logview.days.iter().map(|(d, _)| d.clone()));
    let c = ui.cm(F, "ComboBox2");
    c.items = items;
    c.set_item_index(keep.min(c.items.len() as i32 - 1));
}

/// Reads the lines again and lists them.
fn reload(app: &mut Mb3d, ui: &mut Ui) {
    let st = &mut app.logview;
    if live(ui) {
        let (v, e) = crate::log::recent();
        st.ver = v;
        st.entries = e;
    } else {
        let i = (ui.item_index(F, "ComboBox2") - 1) as usize;
        st.entries = st.days.get(i).map(|(_, p)| crate::log::read(p)).unwrap_or_default();
    }
    st.loaded = Some(Instant::now());
    apply(app, ui);
}

fn level_wanted(choice: i32, l: Level) -> bool {
    match choice {
        1 => l >= Level::Warn,
        2 => l >= Level::Error,
        _ => true,
    }
}

/// The list from the lines, the level, the kind and the search words.
fn apply(app: &mut Mb3d, ui: &mut Ui) {
    let follow = ui.checked(F, "CheckBox1");
    let before = sel_entry(app, ui);
    let words: Vec<String> = ui.text(F, "Edit1").to_lowercase().split_whitespace().map(String::from).collect();
    let level = ui.item_index(F, "ComboBox1");
    let kind = ui.item_index(F, "ComboBox3");
    let kinds: Option<&[&str]> = (kind >= 1).then(|| KINDS.get(kind as usize - 1).copied()).flatten();
    let st = &mut app.logview;
    st.shown = (0..st.entries.len())
        .filter(|&i| {
            let e = &st.entries[i];
            level_wanted(level, e.level)
                && kinds.is_none_or(|k| k.contains(&e.kind.as_str()))
                && (words.is_empty() || {
                    let hay = format!("{} {} {}", e.kind, e.program, e.text).to_lowercase();
                    words.iter().all(|w| hay.contains(w.as_str()))
                })
        })
        .collect();
    let n = st.shown.len();
    // the selection: the newest line when following, else the line it was
    let sel = if follow {
        n as i32 - 1
    } else {
        before.and_then(|b| st.shown.iter().position(|&i| st.entries[i] == b)).map_or(-1, |p| p as i32)
    };
    {
        let c = ui.cm(F, "ListView1");
        c.checks.clear();
        c.virtual_rows = n;
        c.cells.clear();
        c.cells_from = 0;
        c.item_index = sel;
        if follow {
            c.scroll_y = i32::MAX / 2;
        }
    }
    ui.listview_show_selection(F, "ListView1");
    fill_cells(app, ui, true);
    let total = app.logview.entries.len();
    let text = if n == total { trf("@MB:Lines", &[&n]) } else { trf("@MB:OfLines", &[&n, &total]) };
    ui.set_caption(F, "Label5", &text);
    show_selected(app, ui);
}

fn level_text(l: Level) -> String {
    match l {
        Level::Debug => tr("@MB:Debug"),
        Level::Info => tr("@MB:Info"),
        Level::Warn => tr("@MB:WarningLower"),
        Level::Error => tr("@MB:Error"),
    }
}

fn cells_of(e: &Entry) -> Vec<String> {
    let text: String = e.text.chars().take(400).collect();
    vec![e.when(), level_text(e.level), e.program.clone(), e.kind.clone(), text]
}

/// Fills the list view's rows in sight and [`FILL`] beyond when the sight
/// left the rows filled (or `force`).
fn fill_cells(app: &Mb3d, ui: &mut Ui, force: bool) {
    let (top, vis) = ui.listview_window(F, "ListView1");
    let n = app.logview.shown.len();
    {
        let c = ui.c(F, "ListView1");
        let have = c.cells_from..c.cells_from + c.cells.len();
        if !force && have.start <= top && (top + vis).min(n) <= have.end {
            return;
        }
    }
    let from = top.saturating_sub(FILL);
    let to = (top + vis + FILL).min(n);
    let rows: Vec<Vec<String>> = (from..to.max(from)).map(|i| cells_of(&app.logview.entries[app.logview.shown[i]])).collect();
    let c = ui.cm(F, "ListView1");
    c.cells = rows;
    c.cells_from = from;
    c.virtual_rows = n;
}

fn sel_entry(app: &Mb3d, ui: &Ui) -> Option<Entry> {
    let i = ui.item_index(F, "ListView1");
    (i >= 0).then(|| app.logview.shown.get(i as usize)).flatten().map(|&k| app.logview.entries[k].clone())
}

/// The selected line in full below the list.
fn show_selected(app: &Mb3d, ui: &mut Ui) {
    let t = match sel_entry(app, ui) {
        Some(e) => format!("{}  ·  {}  ·  {}  ·  {}\n{}", e.when(), level_text(e.level), e.program, e.kind, e.text.replace(" ⏎ ", "\n")),
        None => String::new(),
    };
    ui.set_text(F, "Memo1", &t);
}

/// The idle work: new lines of the live log, the list view's rows filled
/// where it was scrolled to.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.showing(F) {
        return;
    }
    if live(ui) && crate::log::version() != app.logview.ver && app.logview.loaded.is_none_or(|t| t.elapsed() >= RELOAD) {
        reload(app, ui);
    }
    fill_cells(app, ui, false);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            fill_days(app, ui);
            reload(app, ui);
        }
        "Edit1Change" | "ComboBox1Change" | "ComboBox3Change" => apply(app, ui),
        "ComboBox2Change" => {
            // (an earlier day does not grow: no following there)
            reload(app, ui);
        }
        "CheckBox1Click" => apply(app, ui),
        "ListView1Change" => {
            // a line picked by hand: the list stops following
            let n = app.logview.shown.len() as i32;
            if ui.item_index(F, "ListView1") != n - 1 && ui.checked(F, "CheckBox1") {
                ui.set_checked(F, "CheckBox1", false);
            }
            show_selected(app, ui);
        }
        "Button1Click" => {
            let text = match sel_entry(app, ui) {
                Some(e) => e.line(),
                None => app.logview.shown.iter().map(|&i| app.logview.entries[i].line()).collect::<Vec<_>>().join("\n"),
            };
            crate::vcl::clipboard::set(&text);
            app.message(ui, &tr("@MB:CopiedToTheClipboard"));
        }
        "Button2Click" => {
            let _ = open::that_detached(crate::log::folder());
        }
        "Button3Click" => ui.hide(F),
        _ => {}
    }
}
