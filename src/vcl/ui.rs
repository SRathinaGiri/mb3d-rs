//! The application side of the toolkit: all forms, event dispatch, timers,
//! modal dialogs and the API the program uses to read and change controls.

use super::bitmap::Bitmap;
use super::canvas::Canvas;
use super::control::{Control, Id, Kind};
use super::form::*;
use super::input::Raw;
use super::theme::{Style, Theme};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The program behind the forms.
pub trait App {
    /// A form event with a handler (`e.handler` is the method name from the
    /// form file), or a dialog result.
    fn event(&mut self, ui: &mut Ui, e: &Event);
    /// Called when a background thread woke the loop (see [`Ui::waker`])
    /// and regularly while windows are open.
    fn idle(&mut self, _ui: &mut Ui) {}
}

/// Wakes the event loop from other threads.
#[derive(Clone)]
pub struct Waker(pub(crate) Arc<dyn Fn() + Send + Sync>);

impl Waker {
    pub fn wake(&self) {
        (self.0)()
    }
}

/// Requests to the window system, applied by the runner.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Raise(usize),
    Minimize(usize),
    Maximize(usize, bool),
    Move(usize, i32, i32),
    Cursor(usize, &'static str),
    /// hide and lock the mouse in the window and report its relative
    /// motion as `@MouseDelta` events (the Navigator's mouse look)
    MouseLook(usize, bool),
}

pub struct Ui {
    pub theme: Theme,
    pub forms: Vec<Form>,
    by_name: HashMap<String, usize>,
    pub(crate) events: VecDeque<Event>,
    timers: HashMap<(usize, Id), Instant>,
    pub(crate) quit: bool,
    pub(crate) requests: Vec<Request>,
    pub(crate) waker: Option<Waker>,
    pub main_form: usize,
    /// forms in the order they were shown (z-order for modality)
    pub(crate) modal_stack: Vec<usize>,
    /// close requests waiting for the program's OnCloseQuery answer
    close_pending: Vec<usize>,
    /// internal handlers of the toolkit's own dialogs
    pub(crate) dialogs: HashMap<usize, super::dialogs::DialogState>,
    pub busy: bool,
    /// The user's interface scale (1 = the size of the .dfm files), on top
    /// of the screen's scale factor: windows, controls and text all grow
    /// with it, the layouts stay as they are.
    pub ui_scale: f64,
}

impl Default for Ui {
    fn default() -> Self {
        Ui::new(Style::default())
    }
}

impl Ui {
    pub fn new(style: Style) -> Ui {
        Ui {
            theme: Theme::new(style),
            forms: Vec::new(),
            by_name: HashMap::new(),
            events: VecDeque::new(),
            timers: HashMap::new(),
            quit: false,
            requests: Vec::new(),
            waker: None,
            main_form: 0,
            modal_stack: Vec::new(),
            close_pending: Vec::new(),
            dialogs: HashMap::new(),
            busy: false,
            ui_scale: 1.0,
        }
    }

    /// Sets the interface scale (see `ui_scale`); all windows are resized
    /// and redrawn.
    pub fn set_ui_scale(&mut self, scale: f64) {
        self.ui_scale = scale.clamp(0.5, 4.0);
        for f in &mut self.forms {
            f.dirty = true;
        }
    }

    /// Adds a form from its form file text; OnCreate is queued.
    pub fn add_form(&mut self, dfm: &str) -> Result<usize, String> {
        let f = Form::from_dfm(dfm, &self.theme)?;
        Ok(self.add_built(f))
    }

    pub(crate) fn add_built(&mut self, f: Form) -> usize {
        let i = self.forms.len();
        self.by_name.insert(f.name.to_ascii_lowercase(), i);
        if f.ctl[0].events.contains_key("OnCreate") {
            self.events.push_back(Event { form: f.name.clone(), sender: f.name.clone(), handler: f.ctl[0].events["OnCreate"].clone(), ev: Ev::Create });
        }
        self.forms.push(f);
        i
    }

    /// Switches between the Glossy and the Windows look: the forms are
    /// rebuilt from their form files keeping the state of the controls.
    pub fn set_style(&mut self, style: Style) {
        self.theme = Theme::new(style);
        for f in &mut self.forms {
            f.dirty = true;
        }
    }

    pub fn form_index(&self, name: &str) -> Option<usize> {
        self.by_name.get(&name.to_ascii_lowercase()).copied()
    }

    fn fi(&self, name: &str) -> usize {
        match self.form_index(name) {
            Some(i) => i,
            None => panic!("no form '{name}'"),
        }
    }

    pub fn f(&self, form: &str) -> &Form {
        &self.forms[self.fi(form)]
    }

    pub fn fm(&mut self, form: &str) -> &mut Form {
        let i = self.fi(form);
        self.forms[i].dirty = true;
        &mut self.forms[i]
    }

    pub fn has_form(&self, form: &str) -> bool {
        self.form_index(form).is_some()
    }

    pub fn c(&self, form: &str, ctl: &str) -> &Control {
        self.f(form).c(ctl)
    }

    pub fn cm(&mut self, form: &str, ctl: &str) -> &mut Control {
        self.fm(form).c_mut(ctl)
    }

    // ---- convenience accessors (VCL property style)

    pub fn text(&self, form: &str, ctl: &str) -> String {
        self.c(form, ctl).text.clone()
    }
    pub fn set_text(&mut self, form: &str, ctl: &str, s: &str) {
        self.cm(form, ctl).set_text(s);
    }
    pub fn caption(&self, form: &str, ctl: &str) -> String {
        self.c(form, ctl).caption.clone()
    }
    pub fn set_caption(&mut self, form: &str, ctl: &str, s: &str) {
        self.cm(form, ctl).caption = s.to_string();
    }
    pub fn checked(&self, form: &str, ctl: &str) -> bool {
        let c = self.c(form, ctl);
        if c.kind == Kind::SpeedButton {
            c.down
        } else {
            c.checked
        }
    }
    pub fn set_checked(&mut self, form: &str, ctl: &str, v: bool) {
        let f = self.fm(form);
        let Some(id) = f.id(ctl) else {
            eprintln!("{form}: no control '{ctl}'");
            return;
        };
        match f.ctl[id].kind {
            Kind::RadioButton if v => f.radio_check(id),
            Kind::SpeedButton => {
                if v && f.ctl[id].group_index != 0 {
                    let (gi, p) = (f.ctl[id].group_index, f.ctl[id].parent);
                    if let Some(p) = p {
                        for k in f.ctl[p].children.clone() {
                            if f.ctl[k].kind == Kind::SpeedButton && f.ctl[k].group_index == gi {
                                f.ctl[k].down = false;
                            }
                        }
                    }
                }
                f.ctl[id].down = v;
            }
            _ => {
                f.ctl[id].checked = v;
                f.ctl[id].state = v as u8;
            }
        }
    }
    pub fn item_index(&self, form: &str, ctl: &str) -> i32 {
        self.c(form, ctl).item_index
    }
    pub fn set_item_index(&mut self, form: &str, ctl: &str, i: i32) {
        self.cm(form, ctl).set_item_index(i);
    }
    pub fn position(&self, form: &str, ctl: &str) -> i64 {
        self.c(form, ctl).position
    }
    pub fn set_position(&mut self, form: &str, ctl: &str, p: i64) {
        self.cm(form, ctl).set_position(p);
    }
    pub fn enabled(&self, form: &str, ctl: &str) -> bool {
        self.c(form, ctl).enabled
    }
    pub fn set_enabled(&mut self, form: &str, ctl: &str, v: bool) {
        self.cm(form, ctl).enabled = v;
    }
    pub fn visible(&self, form: &str, ctl: &str) -> bool {
        self.c(form, ctl).visible
    }
    pub fn set_visible(&mut self, form: &str, ctl: &str, v: bool) {
        self.cm(form, ctl).visible = v;
    }
    pub fn tag(&self, form: &str, ctl: &str) -> i64 {
        self.c(form, ctl).tag
    }
    pub fn set_items(&mut self, form: &str, ctl: &str, items: Vec<String>) {
        let c = self.cm(form, ctl);
        c.selected = vec![false; items.len()];
        c.items = items;
        if c.item_index >= c.items.len() as i32 {
            c.item_index = -1;
        }
        c.top_index = c.top_index.min((c.items.len() as i32 - 1).max(0));
    }
    pub fn set_picture(&mut self, form: &str, ctl: &str, b: Option<Bitmap>) {
        self.cm(form, ctl).picture = b;
    }
    /// The active tab sheet's name.
    pub fn active_page(&self, form: &str, ctl: &str) -> String {
        let f = self.f(form);
        f.c(ctl).active_page.map(|i| f.ctl[i].name.clone()).unwrap_or_default()
    }
    pub fn set_active_page(&mut self, form: &str, ctl: &str, page: &str) {
        let f = self.fm(form);
        let p = f.id(page);
        f.c_mut(ctl).active_page = p;
    }
    pub fn set_focus(&mut self, form: &str, ctl: &str) {
        let fi = self.fi(form);
        let id = self.forms[fi].id(ctl);
        let mut out = Vec::new();
        self.forms[fi].set_focus(id, &mut out);
        self.dispatch(fi, out);
    }
    pub fn focused(&self, form: &str) -> Option<String> {
        let f = self.f(form);
        f.focus.map(|i| f.ctl[i].name.clone())
    }

    /// `PopupMenu.Popup`: opens a popup menu at a point of a control
    /// (control coordinates).
    pub fn popup_menu(&mut self, form: &str, menu: &str, at: &str, x: i32, y: i32) {
        let f = self.fm(form);
        let (Some(m), Some(c)) = (f.id(menu), f.id(at)) else { return };
        f.layout();
        let r = f.abs_rect(c);
        f.open_menu(m, c, r.x + x, r.y + y);
    }

    /// The control the last popup menu of a form was opened for
    /// (`TPopupMenu.PopupComponent`).
    pub fn popup_component(&self, form: &str) -> String {
        let f = self.f(form);
        f.popup_owner.map(|i| f.ctl[i].name.clone()).unwrap_or_default()
    }

    /// Rectangle of a control in form client coordinates.
    pub fn ctl_rect(&mut self, form: &str, ctl: &str) -> super::canvas::Rect {
        let f = self.fm(form);
        f.layout();
        match f.id(ctl) {
            Some(i) => f.abs_rect(i),
            None => super::canvas::Rect::default(),
        }
    }

    /// Calls a control's OnClick handler as if it was clicked (VCL `Click`).
    pub fn click(&mut self, form: &str, ctl: &str) {
        let fi = self.fi(form);
        let Some(id) = self.forms[fi].id(ctl) else { return };
        let out = self.forms[fi].click_control(id);
        self.dispatch(fi, out);
    }

    /// Queues an event as if it came from a control.
    pub fn post(&mut self, form: &str, sender: &str, handler: &str, ev: Ev) {
        self.events.push_back(Event { form: form.into(), sender: sender.into(), handler: handler.into(), ev });
    }

    // ---- forms

    pub fn showing(&self, form: &str) -> bool {
        self.form_index(form).map(|i| self.forms[i].visible).unwrap_or(false)
    }

    pub fn show(&mut self, form: &str) {
        let i = self.fi(form);
        self.show_index(i);
    }

    pub(crate) fn show_index(&mut self, i: usize) {
        let f = &mut self.forms[i];
        if f.visible {
            self.requests.push(Request::Raise(i));
            return;
        }
        f.visible = true;
        f.dirty = true;
        f.minimized = false;
        if let Some(h) = f.ctl[0].events.get("OnShow").cloned() {
            self.events.push_back(Event { form: f.name.clone(), sender: f.name.clone(), handler: h, ev: Ev::Show });
        }
        self.modal_stack.retain(|&k| k != i);
        self.modal_stack.push(i);
    }

    /// Shows a form modally: the other forms take no input until it closes.
    pub fn show_modal(&mut self, form: &str) {
        let i = self.fi(form);
        self.forms[i].modal = true;
        self.forms[i].modal_result = MR_NONE;
        self.show_index(i);
    }

    pub fn hide(&mut self, form: &str) {
        let i = self.fi(form);
        self.hide_index(i);
    }

    pub(crate) fn hide_index(&mut self, i: usize) {
        let f = &mut self.forms[i];
        if !f.visible {
            return;
        }
        f.visible = false;
        f.modal = false;
        f.popup = None;
        f.capture = None;
        f.hint = None;
        if let Some(h) = f.ctl[0].events.get("OnHide").cloned() {
            self.events.push_back(Event { form: f.name.clone(), sender: f.name.clone(), handler: h, ev: Ev::Hide });
        }
        self.modal_stack.retain(|&k| k != i);
        if i == self.main_form {
            self.quit = true;
        }
    }

    /// VCL `Close`: OnCloseQuery (if the form has one the program answers
    /// with [`Ui::close_ok`]), OnClose, hide.
    pub fn close(&mut self, form: &str) {
        let i = self.fi(form);
        self.close_index(i);
    }

    pub(crate) fn close_index(&mut self, i: usize) {
        let f = &self.forms[i];
        if let Some(h) = f.ctl[0].events.get("OnCloseQuery").cloned() {
            if !self.close_pending.contains(&i) {
                self.close_pending.push(i);
                self.events.push_back(Event { form: f.name.clone(), sender: f.name.clone(), handler: h, ev: Ev::CloseQuery });
            }
            return;
        }
        self.finish_close(i);
    }

    /// The answer to OnCloseQuery: the form may close.
    pub fn close_ok(&mut self, form: &str) {
        let i = self.fi(form);
        self.close_pending.retain(|&k| k != i);
        self.finish_close(i);
    }

    /// The answer to OnCloseQuery: the form stays open.
    pub fn close_cancel(&mut self, form: &str) {
        let i = self.fi(form);
        self.close_pending.retain(|&k| k != i);
    }

    fn finish_close(&mut self, i: usize) {
        let f = &self.forms[i];
        if let Some(h) = f.ctl[0].events.get("OnClose").cloned() {
            self.events.push_back(Event { form: f.name.clone(), sender: f.name.clone(), handler: h, ev: Ev::Close });
        }
        self.dialog_closed(i);
        self.hide_index(i);
    }

    /// Ends the program.
    pub fn quit(&mut self) {
        self.quit = true;
    }

    pub fn request(&mut self, r: Request) {
        self.requests.push(r);
    }

    pub fn move_form(&mut self, form: &str, x: i32, y: i32) {
        let i = self.fi(form);
        self.forms[i].left = x;
        self.forms[i].top = y;
        self.requests.push(Request::Move(i, x, y));
    }

    pub fn set_client_size(&mut self, form: &str, w: i32, h: i32) {
        let f = self.fm(form);
        f.ctl[0].width = w.max(1);
        f.ctl[0].height = h.max(1);
    }

    /// A handle to wake the event loop from a background thread.
    pub fn waker(&self) -> Option<Waker> {
        self.waker.clone()
    }

    /// The modal form that takes the input, if any.
    pub fn modal_top(&self) -> Option<usize> {
        self.modal_stack.iter().rev().copied().find(|&i| self.forms[i].visible && self.forms[i].modal)
    }

    // ---- events

    /// Turns raw control events into program events.
    pub fn dispatch(&mut self, fi: usize, raws: Vec<Raw>) {
        for r in raws {
            let f = &mut self.forms[fi];
            if r.id >= f.ctl.len() {
                continue;
            }
            match r.name {
                "@ModalResult" => {
                    let mr = f.ctl[r.id].modal_result;
                    f.modal_result = mr;
                    if f.modal || self.dialogs.contains_key(&fi) {
                        self.close_index(fi);
                    }
                    continue;
                }
                "@Cancel" => {
                    f.modal_result = MR_CANCEL;
                    self.close_index(fi);
                    continue;
                }
                "@ColorDialog" => {
                    let tag = format!("@color:{}:{}", f.name, f.ctl[r.id].name);
                    let col = f.ctl[r.id].brush_color;
                    self.pick_color(&tag, col);
                    continue;
                }
                _ => {}
            }
            if self.dialogs.contains_key(&fi) {
                self.dialog_event(fi, &r);
                continue;
            }
            let f = &self.forms[fi];
            let c = &f.ctl[r.id];
            let handler = c.events.get(r.name).cloned();
            if let Some(h) = handler {
                self.events.push_back(Event { form: f.name.clone(), sender: c.name.clone(), handler: h, ev: r.ev });
            }
        }
    }

    /// Delivers dialog results for internal tags (colour buttons).
    pub fn internal_result(&mut self, e: &Event) -> bool {
        if let Some(rest) = e.sender.strip_prefix("@color:") {
            if let Ev::DialogResult(DialogResult::Color(Some(col))) = &e.ev {
                if let Some((form, ctl)) = rest.split_once(':') {
                    if let Some(fi) = self.form_index(form) {
                        if let Some(id) = self.forms[fi].id(ctl) {
                            self.forms[fi].ctl[id].brush_color = *col;
                            self.forms[fi].dirty = true;
                            self.dispatch(fi, vec![Raw { id, name: "OnColorChange", ev: Ev::ColorChange }]);
                        }
                    }
                }
            }
            return true;
        }
        false
    }

    /// Fires due timers; returns the next deadline.
    pub(crate) fn run_timers(&mut self, now: Instant) -> Option<Instant> {
        let mut next: Option<Instant> = None;
        let mut fire = Vec::new();
        for (fi, f) in self.forms.iter().enumerate() {
            for (id, c) in f.ctl.iter().enumerate() {
                if c.kind != Kind::Timer {
                    continue;
                }
                let key = (fi, id);
                if !c.enabled || !c.events.contains_key("OnTimer") {
                    self.timers.remove(&key);
                    continue;
                }
                let iv = Duration::from_millis(c.interval.max(1) as u64);
                let t = *self.timers.entry(key).or_insert(now + iv);
                if t <= now {
                    fire.push((fi, id));
                    self.timers.insert(key, now + iv);
                    next = Some(next.map_or(now + iv, |n| n.min(now + iv)));
                } else {
                    next = Some(next.map_or(t, |n| n.min(t)));
                }
            }
        }
        for (fi, id) in fire {
            let f = &self.forms[fi];
            let c = &f.ctl[id];
            self.events.push_back(Event { form: f.name.clone(), sender: c.name.clone(), handler: c.events["OnTimer"].clone(), ev: Ev::Timer });
        }
        next
    }

    /// Restarts a timer's interval (VCL: setting Enabled / Interval).
    pub fn reset_timer(&mut self, form: &str, timer: &str) {
        let fi = self.fi(form);
        if let Some(id) = self.forms[fi].id(timer) {
            self.timers.remove(&(fi, id));
        }
    }

    pub fn set_timer(&mut self, form: &str, timer: &str, enabled: bool) {
        self.reset_timer(form, timer);
        self.cm(form, timer).enabled = enabled;
    }

    /// Takes the queued events (the runner hands them to the program).
    pub fn take_event(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    // ---- rendering a form without a window (tests, screenshots)

    /// Paints a form into an RGB image of its window size.
    pub fn render_form(&mut self, form: &str, scale: f32) -> (usize, usize, Vec<u32>) {
        let theme = self.theme;
        let f = self.fm(form);
        f.layout();
        let (w, h) = f.outer_size(&theme);
        let (dw, dh) = ((w as f32 * scale).round() as usize, (h as f32 * scale).round() as usize);
        let mut buf = vec![0u32; dw * dh];
        let mut cv = Canvas::new(&mut buf, dw, dh, scale);
        f.active = true;
        f.paint(&mut cv, &theme, w, h);
        (dw, dh, buf)
    }

    /// Writes a form as PNG (for checking the layouts).
    pub fn save_form_png(&mut self, form: &str, path: &std::path::Path, scale: f32) -> std::io::Result<()> {
        let (w, h, buf) = self.render_form(form, scale);
        let rgb: Vec<u8> = buf.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect();
        crate::png::write_rgb(&path.to_string_lossy(), w, h, &rgb)
    }
}
