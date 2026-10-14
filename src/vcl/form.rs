//! A form: the control tree built from a form file, VCL's alignment and
//! anchor layout, geometry queries and the run-time interaction state.

use super::canvas::{line_height, Rect};
use super::control::{self, Align, Control, Id, Kind, AK_BOTTOM, AK_LEFT, AK_RIGHT, AK_TOP};
use super::dfm::{Obj, Value};
use super::font::{self, Font};
use super::theme::Theme;
use std::collections::HashMap;
use std::time::Instant;

/// Mouse buttons and modifier keys (`TShiftState`).
pub const SS_SHIFT: u8 = 1;
pub const SS_ALT: u8 = 2;
pub const SS_CTRL: u8 = 4;
pub const SS_LEFT: u8 = 8;
pub const SS_RIGHT: u8 = 16;
pub const SS_MIDDLE: u8 = 32;
pub const SS_DOUBLE: u8 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// What happened (the VCL event and its parameters).
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Click,
    DblClick,
    Change,
    Enter,
    Exit,
    MouseDown { x: i32, y: i32, button: MouseButton, shift: u8 },
    MouseMove { x: i32, y: i32, shift: u8 },
    MouseUp { x: i32, y: i32, button: MouseButton, shift: u8 },
    MouseEnter,
    MouseLeave,
    /// `delta` in WHEEL_DELTA units (120 per notch), positive = up
    Wheel { delta: i32, x: i32, y: i32, shift: u8 },
    KeyDown { key: u16, shift: u8 },
    KeyUp { key: u16, shift: u8 },
    KeyPress(char),
    /// relative mouse motion while the form has the mouse locked
    /// ([`Request::MouseLook`](super::ui::Request)); `buttons` = SS_LEFT ...
    MouseDelta { dx: f64, dy: f64, shift: u8 },
    /// up-down button: `up` = btNext
    UpDown { up: bool },
    Timer,
    Create,
    Show,
    Hide,
    Close,
    CloseQuery,
    Resize,
    /// the window was moved (handler "WMMove", like a WM_MOVE in WndProc)
    Move,
    Activate,
    Deactivate,
    Select,
    DropDown,
    Expand,
    Collapse,
    ColorChange,
    Scroll,
    /// a modal dialog of the toolkit finished (message box button, file
    /// name, colour, input text); `sender` is the tag given when opening it
    DialogResult(DialogResult),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DialogResult {
    /// mrOk, mrCancel, mrYes, mrNo, ...
    Button(i32),
    File(Option<std::path::PathBuf>),
    Files(Vec<std::path::PathBuf>),
    Color(Option<u32>),
    Text(Option<String>),
}

pub const MR_NONE: i32 = 0;
pub const MR_OK: i32 = 1;
pub const MR_CANCEL: i32 = 2;
pub const MR_ABORT: i32 = 3;
pub const MR_RETRY: i32 = 4;
pub const MR_IGNORE: i32 = 5;
pub const MR_YES: i32 = 6;
pub const MR_NO: i32 = 7;

/// An event for the application: `handler` is the method name from the
/// form file (`Button1Click`), empty for events without one.
#[derive(Clone, Debug)]
pub struct Event {
    pub form: String,
    pub sender: String,
    pub handler: String,
    pub ev: Ev,
}

#[derive(Clone, Debug)]
pub struct MenuLevel {
    /// the popup menu or the menu item whose children are shown
    pub parent: Id,
    /// position in form client coordinates
    pub rect: Rect,
    pub hot: i32,
}

#[derive(Clone, Debug)]
pub enum Popup {
    Combo { id: Id, hot: i32, top: i32 },
    Menu { owner: Id, levels: Vec<MenuLevel> },
}

/// Parts of controls under the mouse (`hot_part`).
pub const PART_NONE: i32 = -1;
pub const PART_UP: i32 = -10;
pub const PART_DOWN: i32 = -11;
pub const PART_THUMB: i32 = -12;
pub const PART_PAGE_UP: i32 = -13;
pub const PART_PAGE_DOWN: i32 = -14;
pub const PART_BUTTON: i32 = -15;
pub const PART_VSCROLL: i32 = -16;
pub const PART_HSCROLL: i32 = -17;
pub const PART_HEADER: i32 = -18;

pub struct Form {
    pub name: String,
    pub class: String,
    pub ctl: Vec<Control>,
    by_name: HashMap<String, Id>,
    pub border_style: String,
    pub border_icons: Vec<String>,
    pub position: String,
    pub stay_on_top: bool,
    pub key_preview: bool,
    /// screen position of the window (VCL Left / Top)
    pub left: i32,
    pub top: i32,
    pub visible: bool,
    /// shown with ShowModal
    pub modal: bool,
    pub modal_result: i32,
    /// tag of a toolkit dialog (returned in the DialogResult event)
    pub dialog_tag: String,

    // ---- interaction state
    pub focus: Option<Id>,
    pub hot: Option<Id>,
    pub hot_part: i32,
    pub capture: Option<Id>,
    pub cap_part: i32,
    pub cap_button: Option<MouseButton>,
    pub cap_start: (i32, i32),
    pub cap_value: i64,
    pub popup: Option<Popup>,
    /// the control the last popup menu was opened for (`PopupComponent`)
    pub popup_owner: Option<Id>,
    pub hint: Option<(String, i32, i32)>,
    pub hint_for: Option<Id>,
    pub hover_since: Instant,
    pub mouse: (i32, i32),
    pub dirty: bool,
    pub active: bool,
    pub caret_on: bool,
    /// LED in the title bar right of the caption (colour 0xRRGGBB); while
    /// `led_blink` it blinks at half the caret's rate, `led_on` is its phase.
    pub caption_led: Option<u32>,
    pub led_blink: bool,
    pub led_on: bool,
    /// the LED's hint (shown while the mouse is on it) and where it was
    /// painted (window coordinates)
    pub led_hint: String,
    pub led_rect: std::cell::Cell<super::canvas::Rect>,
    pub led_hint_shown: bool,
    pub repeat_at: Option<Instant>,
    pub last_click: (Instant, i32, i32, Option<Id>),
    /// title bar button under the mouse / pressed (0 close, 1 max, 2 min)
    pub nc_hot: i32,
    pub nc_pressed: i32,
    pub maximized: bool,
    pub minimized: bool,
    /// the window was resized: size in logical pixels to apply
    pub created: bool,
    dummy: Control,
}

fn walk_build(f: &mut Form, o: &Obj, parent: Option<Id>, theme: &Theme) {
    let pfont = parent.and_then(|p| f.font_opt(p));
    let mut c = control::from_obj(o, theme, pfont.as_ref());
    c.parent = parent;
    let id = f.ctl.len();
    if let Some(p) = parent {
        if f.ctl[p].kind == Kind::GridPanel {
            c.grid_cell = None;
        }
    }
    f.by_name.insert(o.name.to_ascii_lowercase(), id);
    f.ctl.push(c);
    if let Some(p) = parent {
        f.ctl[p].children.push(id);
    }
    for ch in &o.children {
        walk_build(f, ch, Some(id), theme);
    }
}

impl Form {
    /// Builds a form from its form file text.
    pub fn from_dfm(text: &str, theme: &Theme) -> Result<Form, String> {
        let o = super::dfm::parse(text)?;
        Ok(Form::from_obj(&o, theme))
    }

    pub fn from_obj(o: &Obj, theme: &Theme) -> Form {
        let mut f = Form {
            name: o.name.clone(),
            class: o.class.clone(),
            ctl: Vec::new(),
            by_name: HashMap::new(),
            border_style: o.str("BorderStyle").unwrap_or("bsSizeable").to_string(),
            border_icons: match o.get("BorderIcons") {
                Some(Value::Set(s)) => s.clone(),
                _ => vec!["biSystemMenu".into(), "biMinimize".into(), "biMaximize".into()],
            },
            position: o.str("Position").unwrap_or("poDefaultPosOnly").to_string(),
            stay_on_top: o.str("FormStyle") == Some("fsStayOnTop"),
            key_preview: o.bool("KeyPreview").unwrap_or(false),
            left: o.int("Left").unwrap_or(100) as i32,
            top: o.int("Top").unwrap_or(100) as i32,
            visible: false,
            modal: false,
            modal_result: 0,
            dialog_tag: String::new(),
            focus: None,
            hot: None,
            hot_part: PART_NONE,
            capture: None,
            cap_part: PART_NONE,
            cap_button: None,
            cap_start: (0, 0),
            cap_value: 0,
            popup: None,
            popup_owner: None,
            hint: None,
            hint_for: None,
            hover_since: Instant::now(),
            mouse: (-1, -1),
            dirty: true,
            active: false,
            caret_on: true,
            caption_led: None,
            led_blink: false,
            led_on: true,
            led_hint: String::new(),
            led_rect: Default::default(),
            led_hint_shown: false,
            repeat_at: None,
            last_click: (Instant::now(), 0, 0, None),
            nc_hot: -1,
            nc_pressed: -1,
            maximized: false,
            minimized: false,
            created: false,
            dummy: Control::new("", "TUnknown"),
        };
        walk_build(&mut f, o, None, theme);
        let root = &mut f.ctl[0];
        root.kind = Kind::Form;
        root.left = 0;
        root.top = 0;
        root.width = o.int("ClientWidth").or_else(|| o.int("Width").map(|w| w - 16)).unwrap_or(400) as i32;
        root.height = o.int("ClientHeight").or_else(|| o.int("Height").map(|h| h - 38)).unwrap_or(300) as i32;
        root.design = root.rect();
        root.align = Align::None;
        root.border_style = f.border_style.clone();
        if root.font.is_none() {
            root.font = Some(Font::default());
        }
        // radio buttons of a parent: the checked one is the tab stop
        f.layout();
        f
    }

    // ---- lookup

    pub fn id(&self, name: &str) -> Option<Id> {
        self.by_name.get(&name.to_ascii_lowercase()).copied()
    }

    pub fn has(&self, name: &str) -> bool {
        self.id(name).is_some()
    }

    /// The control `name` (a dummy if there is none, reported on stderr).
    pub fn c(&self, name: &str) -> &Control {
        match self.id(name) {
            Some(i) => &self.ctl[i],
            None => {
                eprintln!("{}: no control '{name}'", self.name);
                &self.dummy
            }
        }
    }

    pub fn c_mut(&mut self, name: &str) -> &mut Control {
        self.dirty = true;
        match self.id(name) {
            Some(i) => &mut self.ctl[i],
            None => {
                eprintln!("{}: no control '{name}'", self.name);
                self.dummy = Control::new("", "TUnknown");
                &mut self.dummy
            }
        }
    }

    /// Adds a control at run time (e.g. formula option rows).
    pub fn add_control(&mut self, parent: Id, mut c: Control) -> Id {
        let id = self.ctl.len();
        c.parent = Some(parent);
        c.design = c.rect();
        self.by_name.insert(c.name.to_ascii_lowercase(), id);
        self.ctl.push(c);
        self.ctl[parent].children.push(id);
        self.dirty = true;
        id
    }

    pub fn root(&self) -> &Control {
        &self.ctl[0]
    }

    pub fn caption(&self) -> &str {
        &self.ctl[0].caption
    }

    pub fn set_caption(&mut self, s: &str) {
        self.ctl[0].caption = s.to_string();
        self.dirty = true;
    }

    pub fn client_size(&self) -> (i32, i32) {
        (self.ctl[0].width, self.ctl[0].height)
    }

    pub fn sizeable(&self) -> bool {
        matches!(self.border_style.as_str(), "bsSizeable" | "bsSizeToolWin")
    }

    pub fn tool_window(&self) -> bool {
        matches!(self.border_style.as_str(), "bsToolWindow" | "bsSizeToolWin")
    }

    pub fn has_caption(&self) -> bool {
        self.border_style != "bsNone"
    }

    /// Height of the title bar (logical pixels).
    pub fn caption_h(&self, theme: &Theme) -> i32 {
        if !self.has_caption() {
            0
        } else if self.tool_window() {
            theme.tool_caption_height()
        } else {
            theme.caption_height()
        }
    }

    /// Width of the window border (logical pixels).
    pub fn frame_w(&self) -> i32 {
        if !self.has_caption() || self.maximized {
            0
        } else if self.sizeable() {
            3
        } else {
            1
        }
    }

    /// Outer window size for the client size.
    pub fn outer_size(&self, theme: &Theme) -> (i32, i32) {
        let (w, h) = self.client_size();
        let b = self.frame_w();
        (w + 2 * b, h + 2 * b + self.caption_h(theme))
    }

    /// Sets the client size from the outer window size.
    pub fn set_outer_size(&mut self, w: i32, h: i32, theme: &Theme) {
        let b = self.frame_w();
        let cw = (w - 2 * b).max(1);
        let ch = (h - 2 * b - self.caption_h(theme)).max(1);
        if (cw, ch) != self.client_size() {
            self.ctl[0].width = cw;
            self.ctl[0].height = ch;
            self.dirty = true;
        }
    }

    // ---- inherited properties

    pub(crate) fn font_opt(&self, id: Id) -> Option<Font> {
        let mut i = id;
        loop {
            if let Some(f) = &self.ctl[i].font {
                return Some(f.clone());
            }
            i = self.ctl[i].parent?;
        }
    }

    pub fn font(&self, id: Id) -> Font {
        self.font_opt(id).unwrap_or_default()
    }

    pub fn text_color(&self, id: Id, theme: &Theme) -> u32 {
        let f = self.font(id);
        if f.custom_color {
            f.color
        } else {
            theme.text()
        }
    }

    /// Background colour of a control (inherited from the parent).
    pub fn bg(&self, id: Id, theme: &Theme) -> u32 {
        let mut i = id;
        loop {
            let c = &self.ctl[i];
            if let Some(col) = c.color {
                return col;
            }
            if matches!(c.kind, Kind::Edit | Kind::Memo | Kind::ListBox | Kind::ComboBox | Kind::StringGrid | Kind::ListView) {
                return theme.window();
            }
            match c.parent {
                Some(p) => i = p,
                None => return theme.face(),
            }
        }
    }

    pub fn enabled(&self, id: Id) -> bool {
        let mut i = id;
        loop {
            if !self.ctl[i].enabled {
                return false;
            }
            match self.ctl[i].parent {
                Some(p) => i = p,
                None => return true,
            }
        }
    }

    /// Visible on the screen: the control and all parents visible, on the
    /// active page of a page control, not in a collapsed category panel.
    pub fn showing(&self, id: Id) -> bool {
        let mut i = id;
        loop {
            let c = &self.ctl[i];
            if !c.visible || c.kind.non_visual() {
                return false;
            }
            let Some(p) = c.parent else { return true };
            let pc = &self.ctl[p];
            if c.kind == Kind::TabSheet && pc.kind == Kind::PageControl && pc.active_page != Some(i) {
                return false;
            }
            if pc.kind == Kind::CategoryPanel && pc.collapsed {
                return false;
            }
            i = p;
        }
    }

    pub fn show_hint(&self, id: Id) -> bool {
        let mut i = id;
        loop {
            if let Some(s) = self.ctl[i].show_hint {
                return s;
            }
            match self.ctl[i].parent {
                Some(p) => i = p,
                None => return false,
            }
        }
    }

    // ---- geometry

    /// Offset of the children's coordinate origin inside a container.
    pub fn child_origin(&self, id: Id) -> (i32, i32) {
        let c = &self.ctl[id];
        match c.kind {
            Kind::ScrollBox => {
                let b = if c.border { 2 } else { 0 };
                (b - c.scroll_x, b - c.scroll_y)
            }
            Kind::CategoryPanel => (0, category_header_h(self, id)),
            _ => (0, 0),
        }
    }

    /// Rectangle of the control in form client coordinates.
    pub fn abs_rect(&self, id: Id) -> Rect {
        let c = &self.ctl[id];
        let mut x = c.left;
        let mut y = c.top;
        let mut p = c.parent;
        while let Some(pi) = p {
            let (ox, oy) = self.child_origin(pi);
            let pc = &self.ctl[pi];
            x += ox + if pc.parent.is_some() { pc.left } else { 0 };
            y += oy + if pc.parent.is_some() { pc.top } else { 0 };
            p = pc.parent;
        }
        if c.parent.is_none() {
            return Rect::new(0, 0, c.width, c.height);
        }
        Rect::new(x, y, c.width, c.height)
    }

    /// Visible part of a control's children area in form coordinates.
    pub fn clip_rect(&self, id: Id) -> Rect {
        let mut r = self.content_rect(id);
        let mut p = self.ctl[id].parent;
        while let Some(pi) = p {
            r = r.intersect(&self.content_rect(pi));
            p = self.ctl[pi].parent;
        }
        r
    }

    /// The area in which a container shows its children (form coordinates).
    pub fn content_rect(&self, id: Id) -> Rect {
        let r = self.abs_rect(id);
        let c = &self.ctl[id];
        match c.kind {
            Kind::ScrollBox => {
                let b = if c.border { 2 } else { 0 };
                let (vs, hs) = scrollbox_bars(self, id);
                Rect::new(r.x + b, r.y + b, r.w - 2 * b - if vs { 17 } else { 0 }, r.h - 2 * b - if hs { 17 } else { 0 })
            }
            Kind::CategoryPanel => {
                let hh = category_header_h(self, id);
                Rect::new(r.x, r.y + hh, r.w, r.h - hh)
            }
            _ => r,
        }
    }

    /// Alignment rectangle of a container in its children's coordinates.
    pub fn align_rect(&self, id: Id) -> Rect {
        let c = &self.ctl[id];
        let (w, h) = (c.width, c.height);
        match c.kind {
            Kind::Panel | Kind::CategoryPanelGroup => {
                let mut d = c.border_width;
                if c.bevel_outer != "bvNone" {
                    d += c.bevel_width;
                }
                if c.bevel_inner != "bvNone" {
                    d += c.bevel_width;
                }
                if c.border_style == "bsSingle" {
                    d += 1;
                }
                Rect::new(d, d, w - 2 * d, h - 2 * d)
            }
            Kind::GroupBox | Kind::RadioGroup => {
                let th = line_height(&self.font(id));
                Rect::new(2, th + 1, w - 4, h - th - 3)
            }
            Kind::ScrollBox => {
                let b = if c.border { 2 } else { 0 };
                let (vs, hs) = scrollbox_bars(self, id);
                Rect::new(0, 0, w - 2 * b - if vs { 17 } else { 0 }, h - 2 * b - if hs { 17 } else { 0 })
            }
            Kind::TabControl => display_rect(self, id),
            Kind::CategoryPanel => Rect::new(0, 0, w, h - category_header_h(self, id)),
            _ => Rect::new(0, 0, w, h),
        }
    }

    pub fn measure(&self, f: &Font, s: &str) -> i32 {
        font::text_width(f.face(), f.em(), s).ceil() as i32
    }

    /// VCL layout: auto-sized labels, aligned controls, anchors, pages.
    pub fn layout(&mut self) {
        self.layout_children(0);
    }

    fn autosize(&mut self, id: Id) {
        let c = &self.ctl[id];
        if !matches!(c.kind, Kind::Label | Kind::StaticText) || !c.auto_size || c.align != Align::None {
            return;
        }
        let f = self.font(id);
        let lh = line_height(&f);
        let text = super::canvas::strip_amp(&c.caption);
        if c.word_wrap {
            // keep the width, fit the height
            let w = c.width.max(1);
            let mut n = 0;
            for para in text.replace("\r\n", "\n").split('\n') {
                let mut line = String::new();
                n += 1;
                for word in para.split(' ') {
                    let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                    if self.measure(&f, &cand) > w && !line.is_empty() {
                        n += 1;
                        line = word.to_string();
                    } else {
                        line = cand;
                    }
                }
            }
            self.ctl[id].height = n * lh;
            return;
        }
        let lines: Vec<&str> = text.split(['\n']).map(|l| l.trim_end_matches('\r')).collect();
        let w = lines.iter().map(|l| self.measure(&f, l)).max().unwrap_or(0).max(if text.is_empty() { 0 } else { 1 });
        let h = lh * lines.len().max(1) as i32;
        let c = &mut self.ctl[id];
        if c.alignment == super::canvas::HAlign::Right {
            c.left += c.width - w;
        }
        c.width = w;
        c.height = h;
    }

    fn layout_children(&mut self, id: Id) {
        let kind = self.ctl[id].kind;
        let kids: Vec<Id> = self.ctl[id].children.clone();
        for &k in &kids {
            self.autosize(k);
        }
        match kind {
            Kind::PageControl => {
                let r = display_rect(self, id);
                // the first visible sheet is active when none is set
                if self.ctl[id].active_page.is_none() {
                    let ap = self.ctl[id].prop("ActivePage").and_then(Value::as_str).map(String::from);
                    let found = ap.and_then(|n| self.id(&n));
                    self.ctl[id].active_page = found.or_else(|| kids.iter().copied().find(|&k| self.ctl[k].kind == Kind::TabSheet && self.ctl[k].tab_visible));
                }
                for &k in &kids {
                    if self.ctl[k].kind == Kind::TabSheet {
                        self.ctl[k].set_bounds(r.x, r.y, r.w, r.h);
                    }
                }
            }
            Kind::CategoryPanelGroup => {
                let ar = self.align_rect(id);
                let mut y = ar.y;
                for &k in &kids {
                    if !self.ctl[k].visible || self.ctl[k].kind != Kind::CategoryPanel {
                        continue;
                    }
                    let full = self.ctl[k].design.h.max(category_header_h(self, k));
                    let h = if self.ctl[k].collapsed { category_header_h(self, k) } else { full };
                    self.ctl[k].set_bounds(ar.x, y, ar.w, h);
                    y += h;
                }
            }
            Kind::GridPanel => self.layout_grid(id),
            _ => self.align_controls(id, &kids),
        }
        for &k in &kids {
            if !self.ctl[k].kind.non_visual() {
                self.layout_children(k);
            }
        }
    }

    fn align_controls(&mut self, id: Id, kids: &[Id]) {
        let mut r = self.align_rect(id);
        let (pw, ph) = {
            let ar = self.align_rect(id);
            (ar.right(), ar.bottom())
        };
        let vis: Vec<Id> = kids.iter().copied().filter(|&k| self.ctl[k].visible && !self.ctl[k].kind.non_visual()).collect();
        let aligns: Vec<(Id, Align)> = vis.iter().map(|&k| (k, self.ctl[k].align)).collect();
        let by = |al: Align| -> Vec<Id> { aligns.iter().filter(|(_, a)| *a == al).map(|(k, _)| *k).collect() };
        let mut tops = by(Align::Top);
        tops.sort_by_key(|&k| self.ctl[k].top);
        for k in tops {
            let h = self.ctl[k].height;
            self.ctl[k].set_bounds(r.x, r.y, r.w, h);
            r.y += h;
            r.h -= h;
        }
        let mut bottoms = by(Align::Bottom);
        bottoms.sort_by_key(|&k| -(self.ctl[k].top + self.ctl[k].height));
        for k in bottoms {
            let h = self.ctl[k].height;
            self.ctl[k].set_bounds(r.x, r.bottom() - h, r.w, h);
            r.h -= h;
        }
        let mut lefts = by(Align::Left);
        lefts.sort_by_key(|&k| self.ctl[k].left);
        for k in lefts {
            let w = self.ctl[k].width;
            self.ctl[k].set_bounds(r.x, r.y, w, r.h);
            r.x += w;
            r.w -= w;
        }
        let mut rights = by(Align::Right);
        rights.sort_by_key(|&k| -(self.ctl[k].left + self.ctl[k].width));
        for k in rights {
            let w = self.ctl[k].width;
            self.ctl[k].set_bounds(r.right() - w, r.y, w, r.h);
            r.w -= w;
        }
        for k in by(Align::Client) {
            self.ctl[k].set_bounds(r.x, r.y, r.w.max(0), r.h.max(0));
        }
        // anchors of the other controls
        for &k in kids {
            let c = &mut self.ctl[k];
            if c.align != Align::None || c.kind.non_visual() || c.kind == Kind::TabSheet {
                continue;
            }
            if !c.anchored {
                c.anchor_r = pw - (c.left + c.width);
                c.anchor_b = ph - (c.top + c.height);
                c.anchored = true;
                continue;
            }
            if c.anchors & AK_RIGHT != 0 {
                if c.anchors & AK_LEFT != 0 {
                    c.width = (pw - c.left - c.anchor_r).max(0);
                } else {
                    c.left = pw - c.width - c.anchor_r;
                }
            }
            if c.anchors & AK_BOTTOM != 0 {
                if c.anchors & AK_TOP != 0 {
                    c.height = (ph - c.top - c.anchor_b).max(0);
                } else {
                    c.top = ph - c.height - c.anchor_b;
                }
            }
        }
    }

    fn layout_grid(&mut self, id: Id) {
        // TGridPanel: ColumnCollection / RowCollection (percent values) and
        // ControlCollection (control -> column, row)
        let c = &self.ctl[id];
        let items = |k: &str| match c.prop(k) {
            Some(Value::Items(v)) => v.clone(),
            _ => Vec::new(),
        };
        let pct = |v: &Vec<Vec<(String, Value)>>| -> Vec<f32> {
            v.iter()
                .map(|it| it.iter().find(|(k, _)| k == "Value").and_then(|(_, v)| v.as_f64()).unwrap_or(50.0) as f32)
                .collect()
        };
        let cols = pct(&items("ColumnCollection"));
        let rows = pct(&items("RowCollection"));
        let ctrls = items("ControlCollection");
        let ar = self.align_rect(id);
        let span = |v: &[f32], total: i32| -> Vec<(i32, i32)> {
            let sum: f32 = v.iter().sum::<f32>().max(1e-3);
            let mut pos = 0f32;
            v.iter()
                .map(|p| {
                    let a = pos;
                    pos += p / sum * total as f32;
                    (a.round() as i32, (pos.round() - a.round()) as i32)
                })
                .collect()
        };
        let cs = span(if cols.is_empty() { &[100.0] } else { &cols }, ar.w);
        let rs = span(if rows.is_empty() { &[100.0] } else { &rows }, ar.h);
        for it in ctrls {
            let get = |k: &str| it.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
            let Some(name) = get("Control").and_then(|v| v.as_str().map(String::from)) else { continue };
            let col = get("Column").and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let row = get("Row").and_then(|v| v.as_int()).unwrap_or(0) as usize;
            let Some(k) = self.id(&name) else { continue };
            if let (Some(&(x, w)), Some(&(y, h))) = (cs.get(col), rs.get(row)) {
                let c = &mut self.ctl[k];
                if c.align == Align::Client {
                    c.set_bounds(ar.x + x, ar.y + y, w, h);
                } else {
                    // centred in the cell (the default anchors of a grid panel)
                    let (cw, chh) = (c.width.min(w), c.height.min(h));
                    c.set_bounds(ar.x + x + (w - cw) / 2, ar.y + y + (h - chh) / 2, cw, chh);
                }
            }
        }
    }

    /// Children in painting order: graphic controls first, then windowed
    /// controls (each group in creation order).
    pub fn paint_order(&self, id: Id) -> Vec<Id> {
        let kids = &self.ctl[id].children;
        let mut v: Vec<Id> = kids.iter().copied().filter(|&k| self.ctl[k].kind.graphic()).collect();
        v.extend(kids.iter().copied().filter(|&k| !self.ctl[k].kind.graphic()));
        v.retain(|&k| self.showing(k));
        v
    }

    /// The deepest showing control at form client coordinates.
    pub fn hit(&self, x: i32, y: i32) -> Option<Id> {
        self.hit_in(0, x, y)
    }

    fn hit_in(&self, id: Id, x: i32, y: i32) -> Option<Id> {
        if !self.abs_rect(id).contains(x, y) {
            return None;
        }
        let clip = self.clip_rect(id);
        if clip.contains(x, y) {
            for &k in self.paint_order(id).iter().rev() {
                if let Some(h) = self.hit_in(k, x, y) {
                    return Some(h);
                }
            }
        }
        // a transparent label with an empty caption lets the clicks through
        Some(id)
    }

    /// Focusable controls in tab order.
    pub fn tab_list(&self) -> Vec<Id> {
        let mut out = Vec::new();
        self.tab_walk(0, &mut out);
        out
    }

    fn tab_walk(&self, id: Id, out: &mut Vec<Id>) {
        let mut kids: Vec<Id> = self.ctl[id].children.iter().copied().filter(|&k| self.showing(k)).collect();
        kids.sort_by_key(|&k| self.ctl[k].tab_order);
        for k in kids {
            let c = &self.ctl[k];
            if c.kind.focusable() && c.tab_stop && self.enabled(k) {
                // only the checked radio button of a group is a tab stop
                if c.kind != Kind::RadioButton || c.checked {
                    out.push(k);
                }
            }
            if c.kind.container() {
                self.tab_walk(k, out);
            }
        }
    }

    /// Tab rectangles of a page control (sheets) or tab control (tabs), in
    /// the control's coordinates: (sheet id or tab index, rect).
    pub fn tab_rects(&self, id: Id) -> Vec<(usize, Rect)> {
        tab_rects(self, id)
    }
}

pub(crate) fn category_header_h(_f: &Form, _id: Id) -> i32 {
    24
}

/// Whether a scroll box needs vertical / horizontal scroll bars.
pub(crate) fn scrollbox_bars(f: &Form, id: Id) -> (bool, bool) {
    let c = &f.ctl[id];
    let b = if c.border { 2 } else { 0 };
    let (cw, ch) = (c.width - 2 * b, c.height - 2 * b);
    let (mut ex_r, mut ex_b) = (0, 0);
    for &k in &c.children {
        let k = &f.ctl[k];
        if !k.visible || k.kind.non_visual() || k.align == Align::Client {
            continue;
        }
        ex_r = ex_r.max(k.left + k.width);
        ex_b = ex_b.max(k.top + k.height);
    }
    let mut vs = ex_b > ch;
    let mut hs = ex_r > cw;
    if vs && !hs {
        hs = ex_r > cw - 17;
    }
    if hs && !vs {
        vs = ex_b > ch - 17;
    }
    (vs, hs)
}

/// Extent of the children of a scroll box (for the scroll ranges).
pub(crate) fn scrollbox_extent(f: &Form, id: Id) -> (i32, i32) {
    let c = &f.ctl[id];
    let (mut ex_r, mut ex_b) = (0, 0);
    for &k in &c.children {
        let k = &f.ctl[k];
        if !k.visible || k.kind.non_visual() || k.align == Align::Client {
            continue;
        }
        ex_r = ex_r.max(k.left + k.width);
        ex_b = ex_b.max(k.top + k.height);
    }
    (ex_r, ex_b)
}

fn tab_h(f: &Form, id: Id) -> i32 {
    let c = &f.ctl[id];
    if c.tab_height > 0 {
        c.tab_height
    } else {
        line_height(&f.font(id)) + 7
    }
}

fn tab_labels(f: &Form, id: Id) -> Vec<(usize, String)> {
    let c = &f.ctl[id];
    if c.kind == Kind::PageControl {
        c.children
            .iter()
            .copied()
            .filter(|&k| f.ctl[k].kind == Kind::TabSheet && f.ctl[k].tab_visible && f.ctl[k].visible)
            .map(|k| (k, super::canvas::strip_amp(&f.ctl[k].caption)))
            .collect()
    } else {
        c.tabs.iter().enumerate().map(|(i, t)| (i, super::canvas::strip_amp(t))).collect()
    }
}

pub(crate) fn tab_rects(f: &Form, id: Id) -> Vec<(usize, Rect)> {
    let c = &f.ctl[id];
    let font = f.font(id);
    let th = tab_h(f, id);
    let buttons = c.style == "tsButtons" || c.style == "tsFlatButtons";
    let labels = tab_labels(f, id);
    let mut out = Vec::new();
    let (mut x, mut y) = (if buttons { 0 } else { 2 }, if buttons { 0 } else { 2 });
    let maxw = c.width - 4;
    for (key, text) in labels {
        let w = if c.tab_width > 0 { c.tab_width } else { f.measure(&font, &text) + if buttons { 14 } else { 12 } };
        if c.multi_line && x > 2 && x + w > maxw {
            x = if buttons { 0 } else { 2 };
            y += th + if buttons { 3 } else { 0 };
        }
        out.push((key, Rect::new(x, y, w, th)));
        x += w + if buttons { 3 } else { 0 };
    }
    out
}

/// The client area of a page control / tab control below the tabs.
pub(crate) fn display_rect(f: &Form, id: Id) -> Rect {
    let c = &f.ctl[id];
    let rects = tab_rects(f, id);
    let bottom = rects.iter().map(|(_, r)| r.bottom()).max().unwrap_or(tab_h(f, id) + 2);
    let buttons = c.style == "tsButtons" || c.style == "tsFlatButtons";
    if buttons {
        Rect::new(0, bottom + 3, c.width, (c.height - bottom - 3).max(0))
    } else {
        Rect::new(4, bottom + 2, (c.width - 8).max(0), (c.height - bottom - 6).max(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scroll range of a scroll box does not change while scrolling
    /// (it grew with the position, so the thumb shrank and stopped).
    #[test]
    fn scroll_range_is_fixed() {
        let text = "object F: TForm\n  ClientWidth = 300\n  ClientHeight = 200\n  object S: TScrollBox\n    Left = 0\n    Top = 0\n    Width = 300\n    Height = 200\n    object I: TImage\n      Left = 0\n      Top = 0\n      Width = 900\n      Height = 1000\n    end\n  end\nend\n";
        let mut f = Form::from_dfm(text, &Theme::new(super::super::theme::Style::default())).unwrap();
        let s = f.id("S").unwrap();
        let before = scrollbox_extent(&f, s);
        assert_eq!(before, (900, 1000));
        for y in [100, 500, 800] {
            f.ctl[s].scroll_y = y;
            f.ctl[s].scroll_x = y / 2;
            assert_eq!(scrollbox_extent(&f, s), before);
        }
    }

    /// Glossy: a button under the mouse turns blue, a disabled one does not.
    #[test]
    fn hot_button_is_blue() {
        use super::super::canvas::Canvas;
        let text = "object F: TForm\n  ClientWidth = 200\n  ClientHeight = 100\n  object A: TButton\n    Left = 10\n    Top = 10\n    Width = 80\n    Height = 25\n    Caption = 'A'\n  end\n  object B: TButton\n    Left = 100\n    Top = 10\n    Width = 80\n    Height = 25\n    Caption = 'B'\n    Enabled = False\n  end\nend\n";
        let th = Theme::new(super::super::theme::Style::default());
        let mut f = Form::from_dfm(text, &th).unwrap();
        // a pixel in the upper half of a button, clear of the caption
        let px = |f: &mut Form, x: i32| {
            f.layout();
            let (w, h) = f.outer_size(&th);
            let mut buf = vec![0u32; (w * h) as usize];
            let mut cv = Canvas::new(&mut buf, w as usize, h as usize, 1.0);
            f.paint(&mut cv, &th, w, h);
            let p = buf[((f.caption_h(&th) + f.frame_w() + 15) * w + f.frame_w() + x + 5) as usize];
            ((p >> 16) & 255, p & 255)
        };
        let blue = |(r, b): (u32, u32)| b > r + 40;
        assert!(!blue(px(&mut f, 10)));
        f.mouse_move(50, 20, 0);
        assert!(blue(px(&mut f, 10)));
        f.mouse_move(140, 20, 0);
        assert!(!blue(px(&mut f, 10)));
        assert!(!blue(px(&mut f, 100)));
    }

    /// A drop-down list that fits neither below nor above its box stays
    /// inside the window and scrolls (the theme window's list of themes
    /// ran out of the window: its last theme was cut off).
    #[test]
    fn combo_list_fits_the_window() {
        let th = Theme::new(super::super::theme::Style::default());
        let mut f = Form::from_dfm(include_str!("../app/forms/VisualThemesGUI.dfm"), &th).unwrap();
        let c = f.id("StylesCmb").unwrap();
        f.ctl[c].items = (0..8).map(|i| format!("Theme {i}")).collect();
        f.ctl[c].set_item_index(0);
        f.layout();
        let (_, fh) = f.client_size();
        let lr = super::super::paint::combo_list_rect(&f, c);
        assert!(lr.y >= 0 && lr.bottom() <= fh, "{lr:?} in a window {fh} high");
        let rows = super::super::paint::combo_rows(&f, c);
        assert!(rows > 1 && rows < 8, "{rows} rows");
        // the list opens and End scrolls to the last item
        f.popup = None;
        let r = f.abs_rect(c);
        f.mouse_down(r.right() - 5, r.y + 5, MouseButton::Left, 0);
        f.mouse_up(r.right() - 5, r.y + 5, MouseButton::Left, 0);
        assert!(matches!(f.popup, Some(Popup::Combo { .. })));
        f.key_down(super::super::input::vk::END, 0);
        let Some(Popup::Combo { hot, top, .. }) = f.popup else { panic!("the list closed") };
        assert_eq!(hot, 7);
        assert!(top + rows > 7 && top >= 0, "top {top}, {rows} rows");
    }
}
