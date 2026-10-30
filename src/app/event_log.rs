//! What the user does, for the log ([`crate::log`]): every event a window
//! handles (a button pressed, a check box set, a value typed, a file chosen,
//! a question answered), described by its window, its control's caption
//! and the value it has now.  The many events of one movement (the
//! characters typed into a field, a slider dragged, the wheel turned) make
//! one line, written when another control is used or a second later.  The
//! mouse moving, timers and the like are not logged.

use crate::log;
use crate::vcl::control::{Control, Kind};
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A line held back while its control keeps changing: (control, line,
/// the wheel's notches, when it last changed, and that time for the log).
struct Pending {
    key: String,
    line: String,
    notches: i32,
    at: Instant,
    ms: i64,
}

fn write(p: Pending) {
    let line = if p.notches != 0 { format!("{} ({} notches)", p.line, p.notches) } else { p.line };
    log::log_at(p.ms, log::Level::Info, "ui", &line);
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

fn strip_amp(s: &str) -> String {
    crate::vcl::canvas::strip_amp(s).replace('\n', " ").trim().to_string()
}

/// The window's title.
fn window(ui: &Ui, form: &str) -> String {
    match ui.form_index(form) {
        Some(i) => {
            let c = strip_amp(ui.forms[i].caption());
            if c.is_empty() { form.to_string() } else { c }
        }
        None => form.to_string(),
    }
}

fn control<'a>(ui: &'a Ui, form: &str, name: &str) -> Option<&'a Control> {
    let f = &ui.forms[ui.form_index(form)?];
    f.id(name).map(|id| &f.ctl[id])
}

/// What a control is called on the screen.
fn label(c: &Control, name: &str) -> String {
    let cap = strip_amp(&c.caption);
    if !cap.is_empty() {
        return format!("\"{cap}\"");
    }
    let hint = strip_amp(&c.hint);
    if !hint.is_empty() {
        let h: String = hint.chars().take(60).collect();
        return format!("\"{h}{}\"", if hint.chars().count() > 60 { "…" } else { "" });
    }
    name.to_string()
}

/// The control's value now.
fn value(c: &Control) -> Option<String> {
    let item = |i: i32| c.items.get(i.max(0) as usize).map(|s| strip_amp(s));
    Some(match c.kind {
        Kind::CheckBox | Kind::RadioButton => if c.checked { "on" } else { "off" }.to_string(),
        Kind::SpeedButton => if c.down { "down" } else { "up" }.to_string(),
        Kind::ComboBox => {
            if c.item_index >= 0 { item(c.item_index)? } else { format!("\"{}\"", c.text) }
        }
        Kind::RadioGroup | Kind::ListBox => item(c.item_index)?,
        Kind::ListView => c.cell_row(c.item_index.max(0) as usize).and_then(|r| r.first().cloned())?,
        Kind::Edit | Kind::Memo => {
            let t: String = c.text.chars().take(200).collect();
            format!("\"{t}\"")
        }
        Kind::TrackBar | Kind::ScrollBar | Kind::UpDown => c.position.to_string(),
        Kind::PageControl | Kind::TabControl => c.tabs.get(c.item_index.max(0) as usize).cloned().unwrap_or_default(),
        _ => return None,
    })
}

/// Writes the held-back line.
pub fn flush() {
    let p = PENDING.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(p) = p {
        write(p);
    }
}

/// The idle work: a line held back for a second is written.
pub fn idle() {
    let due = PENDING.lock().unwrap_or_else(|e| e.into_inner()).as_ref().is_some_and(|p| p.at.elapsed() > Duration::from_secs(1));
    if due {
        flush();
    }
}

fn hold(key: String, line: String, notches: i32) {
    let mut g = PENDING.lock().unwrap_or_else(|e| e.into_inner());
    match g.as_mut() {
        Some(p) if p.key == key => {
            p.line = line;
            p.notches += notches;
            p.at = Instant::now();
            p.ms = log::now_ms();
        }
        _ => {
            let old = g.replace(Pending { key, line, notches, at: Instant::now(), ms: log::now_ms() });
            drop(g);
            if let Some(p) = old {
                write(p);
            }
        }
    }
}

/// Logs an event a window handles.
pub fn event(ui: &Ui, e: &Event) {
    if !log::active() {
        return;
    }
    let win = window(ui, &e.form);
    // (the title bar's icons and badge post their clicks as the window's)
    if e.sender == e.form && e.handler.ends_with("Click") && e.handler != "FormClick" {
        flush();
        let what = match e.handler.as_str() {
            "LogIconClick" => "log icon",
            "HistoryIconClick" => "history icon",
            "LanguageBadgeClick" => "language badge",
            h => h.trim_end_matches("Click"),
        };
        log::info("ui", &format!("click the title bar's {what} in {win}"));
        return;
    }
    let c = control(ui, &e.form, &e.sender);
    let what = c.map(|c| label(c, &e.sender)).unwrap_or_else(|| e.sender.clone());
    let at = if e.sender == e.form { format!("{win} [{}]", e.form) } else { format!("{win}: {what} [{}.{}]", e.form, e.sender) };
    // a control with a value (a check box, a list, a field): one line with
    // its new value for the click, select and change events it sends
    let valued = c.is_some_and(|c| matches!(c.kind, Kind::CheckBox | Kind::RadioButton | Kind::RadioGroup | Kind::Edit | Kind::Memo | Kind::ComboBox | Kind::ListBox | Kind::ListView | Kind::TrackBar | Kind::ScrollBar | Kind::UpDown | Kind::PageControl | Kind::TabControl));
    match &e.ev {
        Ev::Click | Ev::Select | Ev::Change | Ev::UpDown { .. } | Ev::Scroll if valued => {
            let v = c.and_then(value).unwrap_or_default();
            let line = if v.is_empty() { format!("change {at}") } else { format!("set {at} = {v}") };
            hold(format!("{}.{}", e.form, e.sender), line, 0);
        }
        Ev::Click | Ev::DblClick | Ev::Select | Ev::Expand | Ev::Collapse | Ev::ColorChange => {
            let verb = match e.ev {
                Ev::DblClick => "double click",
                Ev::Select => "select",
                Ev::Expand => "open",
                Ev::Collapse => "close",
                Ev::ColorChange => "colour",
                _ => "click",
            };
            let v = c.filter(|c| c.kind == Kind::SpeedButton && c.group_index != 0).and_then(value);
            flush();
            log::info("ui", &format!("{verb} {at}{}", v.map(|v| format!(" = {v}")).unwrap_or_default()));
        }
        Ev::Change | Ev::UpDown { .. } | Ev::Scroll => {
            let v = c.and_then(value).unwrap_or_default();
            let line = if v.is_empty() { format!("change {at}") } else { format!("set {at} = {v}") };
            hold(format!("{}.{}", e.form, e.sender), line, 0);
        }
        Ev::Wheel { delta, .. } => {
            hold(format!("{}.{}:wheel", e.form, e.sender), format!("wheel {at}"), delta / 120);
        }
        Ev::MouseDown { x, y, button, .. } => {
            // (on pictures and drawn areas: a button's press is its click)
            if c.is_some_and(|c| matches!(c.kind, Kind::Image | Kind::Shape | Kind::Panel | Kind::Form)) {
                flush();
                log::info("ui", &format!("{button:?} mouse button at {x},{y} on {at}"));
            }
        }
        Ev::KeyDown { key, shift } => {
            // (keys typed into a field are its text)
            if c.is_some_and(|c| matches!(c.kind, Kind::Edit | Kind::Memo | Kind::ComboBox)) {
                return;
            }
            flush();
            log::info("ui", &format!("key {}{} in {at}", if *shift != 0 { format!("{shift:#x}+") } else { String::new() }, key_name(*key)));
        }
        Ev::Close => {
            flush();
            log::info("ui", &format!("close {win} [{}]", e.form));
        }
        Ev::DialogResult(r) => {
            flush();
            let answer = match r {
                DialogResult::Button(b) => button_name(*b).to_string(),
                DialogResult::File(Some(p)) => p.display().to_string(),
                DialogResult::File(None) | DialogResult::Color(None) | DialogResult::Text(None) => "cancelled".into(),
                DialogResult::Files(v) => v.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", "),
                DialogResult::Color(Some(c)) => format!("#{c:06X}"),
                DialogResult::Text(Some(t)) => format!("\"{t}\""),
            };
            log::info("ui", &format!("answer to {} \"{}\": {answer}", if e.form.is_empty() { "dialog".to_string() } else { win }, e.sender));
        }
        _ => {}
    }
}

/// A hotkey of the program.
pub fn hotkey(ui: &Ui, form: usize, key: u16, shift: u8) {
    if !log::active() {
        return;
    }
    flush();
    let win = ui.forms.get(form).map(|f| strip_amp(f.caption())).unwrap_or_default();
    log::info("ui", &format!("hotkey {}{} in {win}", if shift != 0 { format!("{shift:#x}+") } else { String::new() }, key_name(key)));
}

fn button_name(b: i32) -> &'static str {
    use crate::vcl::form::*;
    match b {
        MR_OK => "OK",
        MR_CANCEL => "Cancel",
        MR_YES => "Yes",
        MR_NO => "No",
        _ => "other",
    }
}

fn key_name(k: u16) -> String {
    match k {
        8 => "Backspace".into(),
        9 => "Tab".into(),
        13 => "Enter".into(),
        27 => "Esc".into(),
        32 => "Space".into(),
        33 => "PageUp".into(),
        34 => "PageDown".into(),
        35 => "End".into(),
        36 => "Home".into(),
        37 => "Left".into(),
        38 => "Up".into(),
        39 => "Right".into(),
        40 => "Down".into(),
        45 => "Insert".into(),
        46 => "Delete".into(),
        48..=57 | 65..=90 => (k as u8 as char).to_string(),
        112..=123 => format!("F{}", k - 111),
        k => format!("key {k}"),
    }
}
