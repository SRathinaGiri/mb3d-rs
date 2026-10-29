//! Mouse and keyboard handling of the controls (VCL behaviour), producing
//! raw events `(control, event name, parameters)`.

use super::canvas::{line_height, Rect};
use super::control::{Id, Kind};
use super::form::*;
use super::paint::{self, char_xs, combo_list_rect, memo_lines, menu_item_rects, menu_items, menu_size, sb_geom, track_geom, updown_rects, SB};
use std::time::{Duration, Instant};

/// A raw event: the control, the VCL event property (`OnClick`) and its
/// parameters; names starting with '@' are requests to the toolkit.
#[derive(Clone, Debug)]
pub struct Raw {
    pub id: Id,
    pub name: &'static str,
    pub ev: Ev,
}

pub mod vk {
    pub const BACK: u16 = 0x08;
    pub const TAB: u16 = 0x09;
    pub const RETURN: u16 = 0x0D;
    pub const SHIFT: u16 = 0x10;
    pub const CONTROL: u16 = 0x11;
    pub const MENU: u16 = 0x12;
    pub const ESCAPE: u16 = 0x1B;
    pub const SPACE: u16 = 0x20;
    pub const PRIOR: u16 = 0x21;
    pub const NEXT: u16 = 0x22;
    pub const END: u16 = 0x23;
    pub const HOME: u16 = 0x24;
    pub const LEFT: u16 = 0x25;
    pub const UP: u16 = 0x26;
    pub const RIGHT: u16 = 0x27;
    pub const DOWN: u16 = 0x28;
    pub const INSERT: u16 = 0x2D;
    pub const DELETE: u16 = 0x2E;
    pub const F1: u16 = 0x70;
    pub const F4: u16 = 0x73;
    pub const ADD: u16 = 0x6B;
    pub const SUBTRACT: u16 = 0x6D;
}

fn raw(id: Id, name: &'static str, ev: Ev) -> Raw {
    Raw { id, name, ev }
}

/// Adds a wheel event's `delta` to what was left over and returns the
/// whole notches (120 each, at most 10) in it, the sign of the turn kept;
/// the rest waits for the next event (touchpads, hi-res wheels).
pub fn wheel_steps(rest: &mut i32, delta: i32) -> i32 {
    if delta.signum() != rest.signum() {
        *rest = 0;
    }
    *rest += delta;
    let n = (rest.abs() / 120).min(10);
    let steps = n * rest.signum();
    *rest = if n == 10 { 0 } else { *rest - steps * 120 };
    steps
}

const REPEAT_FIRST: Duration = Duration::from_millis(400);
const REPEAT_NEXT: Duration = Duration::from_millis(60);

impl Form {
    fn local(&self, id: Id, x: i32, y: i32) -> (i32, i32) {
        let r = self.abs_rect(id);
        (x - r.x, y - r.y)
    }

    fn has_handler(&self, id: Id, ev: &str) -> bool {
        self.ctl[id].events.contains_key(ev)
    }

    /// Part of a control under client coordinates (for hot tracking).
    pub fn part_at(&self, id: Id, x: i32, y: i32) -> i32 {
        let c = &self.ctl[id];
        let r = self.abs_rect(id);
        let (lx, ly) = (x - r.x, y - r.y);
        match c.kind {
            Kind::UpDown => {
                let (a, _) = updown_rects(c, Rect::new(0, 0, r.w, r.h));
                if a.contains(lx, ly) {
                    PART_UP
                } else {
                    PART_DOWN
                }
            }
            Kind::TrackBar => {
                let (_, thumb, _, _) = track_geom(c);
                if thumb.contains(lx, ly) {
                    PART_THUMB
                } else {
                    PART_NONE
                }
            }
            Kind::ScrollBar => {
                let page = c.page_size.max(1);
                sb_part(Rect::new(0, 0, r.w, r.h), c.vertical, c.min, c.max - page + 1, page, c.position, lx, ly)
            }
            Kind::ComboBox => {
                if lx >= r.w - 18 {
                    PART_BUTTON
                } else {
                    PART_NONE
                }
            }
            Kind::RadioGroup => paint::radio_items(self, id).iter().position(|ir| ir.contains(lx, ly)).map(|i| i as i32).unwrap_or(PART_NONE),
            Kind::PageControl | Kind::TabControl => {
                self.tab_rects(id).iter().position(|(_, tr)| tr.contains(lx, ly)).map(|i| i as i32).unwrap_or(PART_NONE)
            }
            Kind::ListBox => {
                let inner = r.inset(if c.border { 2 } else { 0 });
                let vis = (inner.h / c.item_height.max(8)).max(1);
                if c.items.len() as i32 > vis && x >= inner.right() - SB {
                    let sr = Rect::new(inner.right() - SB, inner.y, SB, inner.h);
                    return sb_part(sr, true, 0, c.items.len() as i64 - vis as i64, vis as i64, c.top_index as i64, x, y);
                }
                PART_NONE
            }
            Kind::ListView => {
                let (hh, _, vis, max, top) = self.listview_rows(id);
                let inner = r.inset(2);
                if c.cells.len() as i32 > vis && x >= inner.right() - SB && y >= inner.y + hh {
                    let sr = Rect::new(inner.right() - SB, inner.y + hh, SB, inner.h - hh);
                    return sb_part(sr, true, 0, max as i64, vis as i64, top as i64, x, y);
                }
                PART_NONE
            }
            Kind::Memo => {
                if c.scroll_bars & 2 != 0 {
                    let inner = r.inset(if c.border { 2 } else { 0 });
                    if x >= inner.right() - SB {
                        let (vis, max) = self.memo_range(id);
                        let sr = Rect::new(inner.right() - SB, inner.y, SB, inner.h);
                        return sb_part(sr, true, 0, max, vis, c.scroll_y as i64, x, y);
                    }
                }
                PART_NONE
            }
            Kind::ScrollBox => {
                let (vs, hs) = scrollbox_bars(self, id);
                let b = if c.border { 2 } else { 0 };
                if vs && x >= r.right() - b - SB {
                    return PART_VSCROLL;
                }
                if hs && y >= r.bottom() - b - SB {
                    return PART_HSCROLL;
                }
                PART_NONE
            }
            Kind::CategoryPanel => {
                if ly < category_header_h(self, id) {
                    PART_HEADER
                } else {
                    PART_NONE
                }
            }
            _ => PART_NONE,
        }
    }

    fn memo_range(&self, id: Id) -> (i64, i64) {
        let c = &self.ctl[id];
        let lh = line_height(&self.font(id)).max(1);
        let inner_h = c.height - if c.border { 4 } else { 0 } - 1 - if c.scroll_bars & 1 != 0 { SB } else { 0 };
        let vis = (inner_h / lh) as i64;
        let n = memo_lines(self, id).len() as i64;
        (vis, (n - vis).max(0))
    }

    // ---- focus

    pub fn set_focus(&mut self, id: Option<Id>, out: &mut Vec<Raw>) {
        if self.focus == id {
            return;
        }
        if let Some(old) = self.focus {
            if old < self.ctl.len() {
                out.push(raw(old, "OnExit", Ev::Exit));
            }
        }
        self.focus = id;
        self.caret_on = true;
        self.dirty = true;
        if let Some(n) = id {
            out.push(raw(n, "OnEnter", Ev::Enter));
        }
    }

    fn focus_next(&mut self, back: bool, out: &mut Vec<Raw>) {
        let list = self.tab_list();
        if list.is_empty() {
            return;
        }
        let pos = self.focus.and_then(|f| list.iter().position(|&k| k == f));
        let next = match pos {
            None => 0,
            Some(p) if back => (p + list.len() - 1) % list.len(),
            Some(p) => (p + 1) % list.len(),
        };
        let id = list[next];
        self.set_focus(Some(id), out);
        // select the whole text of an edit (AutoSelect)
        let c = &mut self.ctl[id];
        if c.kind == Kind::Edit {
            c.sel_anchor = 0;
            c.caret = c.text.chars().count();
        }
    }

    // ---- mouse

    pub fn mouse_down(&mut self, x: i32, y: i32, button: MouseButton, mut shift: u8) -> Vec<Raw> {
        let mut out = Vec::new();
        self.hint = None;
        self.hint_for = None;
        self.dirty = true;
        self.layout();
        if let Some(p) = self.popup.clone() {
            if self.popup_contains(&p, x, y) {
                if let Popup::Combo { id, .. } = p {
                    let _ = id;
                }
                return out;
            }
            // a click outside closes the popup and is consumed
            let on_combo = matches!(p, Popup::Combo { id, .. } if self.abs_rect(id).contains(x, y));
            self.popup = None;
            if on_combo || matches!(p, Popup::Menu { .. }) {
                return out;
            }
        }
        let Some(id) = self.hit(x, y) else { return out };
        if !self.enabled(id) {
            return out;
        }
        // double click
        let now = Instant::now();
        let (t, lx0, ly0, lid) = self.last_click;
        let dbl = lid == Some(id) && now.duration_since(t) < Duration::from_millis(500) && (lx0 - x).abs() < 4 && (ly0 - y).abs() < 4 && button == MouseButton::Left;
        self.last_click = if dbl { (now - Duration::from_secs(10), x, y, None) } else { (now, x, y, Some(id)) };
        if dbl {
            shift |= SS_DOUBLE;
        }
        shift |= match button {
            MouseButton::Left => SS_LEFT,
            MouseButton::Right => SS_RIGHT,
            MouseButton::Middle => SS_MIDDLE,
        };
        self.capture = Some(id);
        self.cap_button = Some(button);
        self.cap_part = self.part_at(id, x, y);
        self.cap_start = (x, y);
        self.hot = Some(id);
        self.hot_part = self.cap_part;
        let kind = self.ctl[id].kind;
        if kind.focusable() && button == MouseButton::Left {
            self.set_focus(Some(id), &mut out);
        }
        let (lx, ly) = self.local(id, x, y);
        // A ListBox scrollbar is part of the control, but it is not an item.
        // Do not forward its press to application-level list handlers: those
        // commonly select (or drag) the current row on OnMouseDown.
        if kind != Kind::ListBox || self.cap_part == PART_NONE {
            out.push(raw(id, "OnMouseDown", Ev::MouseDown { x: lx, y: ly, button, shift }));
        }
        // a double click on a list view's column headers is two column
        // clicks, not a double click on its selected row
        let on_header = kind == Kind::ListView && ly <= self.listview_rows(id).0 + 2;
        if dbl && !on_header {
            out.push(raw(id, "OnDblClick", Ev::DblClick));
        }
        if button != MouseButton::Left {
            return out;
        }
        match kind {
            Kind::Edit => {
                let p = self.edit_pos_at(id, x);
                let c = &mut self.ctl[id];
                if dbl {
                    let (a, b) = word_at(&c.chars(), p);
                    c.sel_anchor = a;
                    c.caret = b;
                } else {
                    c.caret = p;
                    if shift & SS_SHIFT == 0 {
                        c.sel_anchor = p;
                    }
                }
            }
            Kind::Memo => {
                if self.cap_part != PART_NONE {
                    self.scroll_part(id, self.cap_part, &mut out);
                    self.repeat_at = Some(now + REPEAT_FIRST);
                } else {
                    let p = self.memo_pos_at(id, x, y);
                    let c = &mut self.ctl[id];
                    if dbl {
                        let (a, b) = word_at(&c.chars(), p);
                        c.sel_anchor = a;
                        c.caret = b;
                    } else {
                        c.caret = p;
                        if shift & SS_SHIFT == 0 {
                            c.sel_anchor = p;
                        }
                    }
                }
            }
            Kind::ComboBox => {
                let c = &self.ctl[id];
                if c.style == "csDropDownList" || c.style.starts_with("csOwnerDraw") || self.cap_part == PART_BUTTON {
                    self.open_combo(id, &mut out);
                } else {
                    let p = self.edit_pos_at(id, x);
                    let c = &mut self.ctl[id];
                    c.caret = p;
                    c.sel_anchor = p;
                }
            }
            Kind::ListBox => {
                if self.cap_part != PART_NONE {
                    self.scroll_part(id, self.cap_part, &mut out);
                    self.repeat_at = Some(now + REPEAT_FIRST);
                } else {
                    self.listbox_select_at(id, y, shift, &mut out);
                }
            }
            Kind::TrackBar => {
                if self.cap_part == PART_THUMB {
                    let c = &self.ctl[id];
                    let (_, thumb, _, _) = track_geom(c);
                    let r = self.abs_rect(id);
                    let centre = if c.vertical { r.y + thumb.y + thumb.h / 2 } else { r.x + thumb.x + thumb.w / 2 };
                    self.cap_value = (if c.vertical { y } else { x } - centre) as i64;
                } else {
                    self.track_page(id, x, y, &mut out);
                    self.repeat_at = Some(now + REPEAT_FIRST);
                }
            }
            Kind::UpDown => {
                self.updown_step(id, self.cap_part == PART_UP, &mut out);
                self.repeat_at = Some(now + REPEAT_FIRST);
            }
            Kind::ScrollBar => {
                if self.cap_part == PART_THUMB {
                    self.cap_value = self.ctl[id].position;
                } else {
                    self.scroll_part(id, self.cap_part, &mut out);
                    self.repeat_at = Some(now + REPEAT_FIRST);
                }
            }
            Kind::ScrollBox => {
                if self.cap_part == PART_VSCROLL || self.cap_part == PART_HSCROLL {
                    self.scrollbox_click(id, x, y, &mut out);
                }
            }
            Kind::PageControl | Kind::TabControl => {
                if self.cap_part >= 0 {
                    let rects = self.tab_rects(id);
                    let key = rects[self.cap_part as usize].0;
                    let c = &mut self.ctl[id];
                    if c.kind == Kind::PageControl {
                        if c.active_page != Some(key) {
                            out.push(raw(id, "OnChanging", Ev::Change));
                            c.active_page = Some(key);
                            out.push(raw(id, "OnChange", Ev::Change));
                        }
                    } else if c.tab_index != key as i32 {
                        out.push(raw(id, "OnChanging", Ev::Change));
                        c.tab_index = key as i32;
                        out.push(raw(id, "OnChange", Ev::Change));
                    }
                }
            }
            Kind::CategoryPanel => {
                if self.cap_part == PART_HEADER {
                    let c = &mut self.ctl[id];
                    c.collapsed = !c.collapsed;
                    let name = if c.collapsed { "OnCollapse" } else { "OnExpand" };
                    let ev = if c.collapsed { Ev::Collapse } else { Ev::Expand };
                    out.push(raw(id, name, ev));
                    self.layout();
                }
            }
            Kind::StringGrid => {
                let (lx, ly) = self.local(id, x, y);
                let c = &mut self.ctl[id];
                let rh = c.default_row_height.max(10);
                let row = ly / rh + if ly / rh >= c.fixed_rows { c.scroll_y } else { 0 };
                let mut cx = 0;
                let mut col = -1;
                for (i, k) in c.cols.iter().enumerate() {
                    if lx >= cx && lx < cx + k.width {
                        col = i as i32;
                    }
                    cx += k.width;
                }
                if row >= c.fixed_rows && (row as usize) < c.cells.len() && col >= c.fixed_cols {
                    c.row = row;
                    c.col = col;
                    out.push(raw(id, "OnSelectCell", Ev::Select));
                    out.push(raw(id, "OnClick", Ev::Click));
                }
            }
            Kind::ListView if self.cap_part != PART_NONE => {
                self.scroll_part(id, self.cap_part, &mut out);
                self.repeat_at = Some(now + REPEAT_FIRST);
            }
            Kind::ListView => {
                let (hh, rh, _, _, top) = self.listview_rows(id);
                let (lx, ly) = self.local(id, x, y);
                if ly <= hh + 2 {
                    // A report-style ListView uses its column headers as
                    // ordinary clickable controls.  The chosen column is
                    // exposed through `col` for the OnColumnClick handler.
                    let r = self.abs_rect(id);
                    let c = &self.ctl[id];
                    let widths = c.column_widths(r.w - 4);
                    let mut x = 0;
                    let col = widths.iter().position(|w| {
                        let hit = lx >= x && lx < x + *w;
                        x += *w;
                        hit
                    });
                    if let Some(col) = col {
                        self.ctl[id].col = col as i32;
                        out.push(raw(id, "OnColumnClick", Ev::Click));
                    }
                } else {
                    let i = (ly - hh - 2) / rh + top;
                    let c = &mut self.ctl[id];
                    if (i as usize) < c.cells.len() {
                        if c.multi_select {
                            let n = c.cells.len();
                            c.selected.resize(n, false);
                            if shift & SS_CTRL != 0 {
                                c.selected[i as usize] ^= true;
                            } else if shift & SS_SHIFT != 0 && c.item_index >= 0 {
                                let (a, b) = (c.item_index.min(i), c.item_index.max(i));
                                c.selected.fill(false);
                                for selected in &mut c.selected[a as usize..=b as usize] {
                                    *selected = true;
                                }
                            } else {
                                c.selected.fill(false);
                                c.selected[i as usize] = true;
                            }
                        }
                        if c.item_index != i {
                            c.item_index = i;
                            out.push(raw(id, "OnSelectItem", Ev::Select));
                            out.push(raw(id, "OnChange", Ev::Change));
                        }
                    }
                    if c.checkboxes && lx < 22 && (i as usize) < c.cells.len() {
                        let n = c.cells.len();
                        c.checks.resize(n, false);
                        c.checks[i as usize] ^= true;
                        out.push(raw(id, "OnChange", Ev::Change));
                    }
                }
                out.push(raw(id, "OnClick", Ev::Click));
            }
            _ => {}
        }
        out
    }

    pub fn mouse_move(&mut self, x: i32, y: i32, shift: u8) -> Vec<Raw> {
        let mut out = Vec::new();
        if (x, y) == self.mouse {
            return out;
        }
        self.mouse = (x, y);
        self.layout();
        if let Some(Popup::Combo { id, top, .. }) = self.popup {
            let lr = combo_list_rect(self, id);
            if lr.contains(x, y) {
                let ih = line_height(&self.font(id)) + 2;
                let h = top + (y - lr.y - 1) / ih;
                if h >= 0 && (h as usize) < self.ctl[id].items.len() {
                    self.popup = Some(Popup::Combo { id, hot: h, top });
                    self.dirty = true;
                }
            }
            return out;
        }
        if let Some(Popup::Menu { owner, mut levels }) = self.popup.clone() {
            self.menu_hover(&mut levels, x, y);
            self.popup_owner = Some(owner);
        self.popup = Some(Popup::Menu { owner, levels });
            self.dirty = true;
            return out;
        }
        let hit = self.hit(x, y);
        if let Some(cap) = self.capture {
            let inside = self.abs_rect(cap).contains(x, y);
            let hot = if inside { Some(cap) } else { None };
            // A list-box scrollbar keeps the part captured on mouse-down.
            // In particular, the pointer naturally leaves the moving thumb
            // while it is dragged.  Re-testing it here used to turn that
            // drag into a list-row drag/selection, which application popup
            // lists may interpret as an accepted choice.
            let keep_list_scroll_part = self.ctl[cap].kind == Kind::ListBox && self.cap_part != PART_NONE;
            let part = if keep_list_scroll_part {
                self.cap_part
            } else if inside {
                self.part_at(cap, x, y)
            } else {
                PART_NONE
            };
            if hot != self.hot || part != self.hot_part {
                self.hot = hot;
                self.hot_part = part;
                self.dirty = true;
            }
            let (lx, ly) = self.local(cap, x, y);
            out.push(raw(cap, "OnMouseMove", Ev::MouseMove { x: lx, y: ly, shift }));
            self.drag(cap, x, y, &mut out);
            return out;
        }
        if hit != self.hot {
            if let Some(old) = self.hot {
                if old < self.ctl.len() {
                    out.push(raw(old, "OnMouseLeave", Ev::MouseLeave));
                }
            }
            if let Some(n) = hit {
                out.push(raw(n, "OnMouseEnter", Ev::MouseEnter));
            }
            self.hot = hit;
            self.hint = None;
            self.hint_for = None;
            self.hover_since = Instant::now();
            self.dirty = true;
        } else if self.hint.is_none() {
            self.hover_since = Instant::now();
        }
        if let Some(h) = hit {
            let part = self.part_at(h, x, y);
            if part != self.hot_part {
                self.hot_part = part;
                self.dirty = true;
            }
            if self.enabled(h) {
                let (lx, ly) = self.local(h, x, y);
                out.push(raw(h, "OnMouseMove", Ev::MouseMove { x: lx, y: ly, shift }));
            }
        }
        out
    }

    fn drag(&mut self, id: Id, x: i32, y: i32, out: &mut Vec<Raw>) {
        let kind = self.ctl[id].kind;
        if self.cap_button != Some(MouseButton::Left) {
            return;
        }
        match kind {
            Kind::Edit | Kind::ComboBox => {
                if kind == Kind::ComboBox && self.ctl[id].style == "csDropDownList" {
                    return;
                }
                let p = self.edit_pos_at(id, x);
                self.ctl[id].caret = p;
                self.ensure_caret_visible(id);
                self.dirty = true;
            }
            Kind::Memo if self.cap_part == PART_NONE => {
                let p = self.memo_pos_at(id, x, y);
                self.ctl[id].caret = p;
                self.ensure_caret_visible(id);
                self.dirty = true;
            }
            Kind::Memo | Kind::ListBox if self.cap_part == PART_THUMB => {
                let r = self.abs_rect(id);
                let (vis, max) = if kind == Kind::Memo {
                    self.memo_range(id)
                } else {
                    let c = &self.ctl[id];
                    let vis = ((r.h - 4) / c.item_height.max(8)).max(1) as i64;
                    (vis, (c.items.len() as i64 - vis).max(0))
                };
                let track = r.h - 4 - 2 * SB;
                let tl = ((vis as f64 / (max + vis).max(1) as f64) * track as f64).max(8.0);
                let frac = (y - self.cap_start.1) as f64 / (track as f64 - tl).max(1.0);
                let v = (self.cap_value as f64 + frac * max as f64).round().clamp(0.0, max as f64) as i32;
                let c = &mut self.ctl[id];
                if kind == Kind::Memo {
                    c.scroll_y = v;
                } else {
                    c.top_index = v;
                }
                self.dirty = true;
            }
            Kind::ListView if self.cap_part == PART_THUMB => {
                let (hh, _, vis, max, _) = self.listview_rows(id);
                let r = self.abs_rect(id);
                let track = r.h - 4 - hh - 2 * SB;
                let tl = ((vis as f64 / (max + vis).max(1) as f64) * track as f64).max(8.0);
                let frac = (y - self.cap_start.1) as f64 / (track as f64 - tl).max(1.0);
                self.ctl[id].scroll_y = (self.cap_value as f64 + frac * max as f64).round().clamp(0.0, max as f64) as i32;
                self.dirty = true;
            }
            Kind::ListBox if self.cap_part == PART_NONE => {
                let c = &self.ctl[id];
                if !c.multi_select {
                    self.listbox_select_at(id, y, 0, out);
                }
            }
            Kind::TrackBar if self.cap_part == PART_THUMB => {
                let c = &self.ctl[id];
                let r = self.abs_rect(id);
                let (_, _, x0, x1) = track_geom(c);
                let p = if c.vertical { y - r.y } else { x - r.x } - self.cap_value as i32;
                let range = (c.max - c.min) as f64;
                let v = c.min + (((p - x0) as f64 / (x1 - x0).max(1) as f64).clamp(0.0, 1.0) * range).round() as i64;
                if v != c.position {
                    self.ctl[id].position = v;
                    out.push(raw(id, "OnChange", Ev::Change));
                    self.dirty = true;
                }
            }
            Kind::ScrollBar if self.cap_part == PART_THUMB => {
                let c = &self.ctl[id];
                let r = self.abs_rect(id);
                let page = c.page_size.max(1);
                let (_, _, track, thumb) = sb_geom(Rect::new(0, 0, r.w, r.h), c.vertical, c.min, c.max - page + 1, page, c.position);
                let (d, len) = if c.vertical { (y - self.cap_start.1, track.h - thumb.h) } else { (x - self.cap_start.0, track.w - thumb.w) };
                let range = (c.max - page + 1 - c.min).max(0) as f64;
                let v = (self.cap_value as f64 + d as f64 / len.max(1) as f64 * range).round() as i64;
                let v = v.clamp(c.min, (c.max - page + 1).max(c.min));
                if v != c.position {
                    self.ctl[id].position = v;
                    out.push(raw(id, "OnScroll", Ev::Scroll));
                    out.push(raw(id, "OnChange", Ev::Change));
                    self.dirty = true;
                }
            }
            Kind::ScrollBox if self.cap_part == PART_VSCROLL || self.cap_part == PART_HSCROLL => {
                if self.cap_value != 0 {
                    self.scrollbox_drag(id, x, y);
                }
            }
            _ => {}
        }
    }

    pub fn mouse_up(&mut self, x: i32, y: i32, button: MouseButton, shift: u8) -> Vec<Raw> {
        let mut out = Vec::new();
        self.dirty = true;
        self.repeat_at = None;
        if let Some(Popup::Combo { id, hot, .. }) = self.popup {
            let lr = combo_list_rect(self, id);
            if lr.contains(x, y) {
                self.popup = None;
                self.capture = None;
                self.combo_select(id, hot, &mut out);
            }
            self.capture = None;
            return out;
        }
        if let Some(Popup::Menu { owner, mut levels }) = self.popup.clone() {
            self.menu_hover(&mut levels, x, y);
            for lv in levels.iter().rev() {
                if !lv.rect.contains(x, y) {
                    continue;
                }
                // hot -1: the border or padding, no item
                if let Some((k, _)) = usize::try_from(lv.hot).ok().and_then(|h| menu_item_rects(self, lv).into_iter().nth(h)) {
                    let c = &self.ctl[k];
                    if c.enabled && c.caption != "-" && menu_items(self, k).is_empty() {
                        self.popup = None;
                        self.capture = None;
                        out.push(raw(k, "OnClick", Ev::Click));
                        return out;
                    }
                }
            }
            let _ = owner;
            self.popup_owner = Some(owner);
        self.popup = Some(Popup::Menu { owner, levels });
            self.capture = None;
            return out;
        }
        let Some(id) = self.capture.take() else { return out };
        if self.cap_button != Some(button) {
            self.capture = Some(id);
            return out;
        }
        self.cap_button = None;
        let inside = self.abs_rect(id).contains(x, y);
        let (lx, ly) = self.local(id, x, y);
        let kind = self.ctl[id].kind;
        let part = self.cap_part;
        self.cap_part = PART_NONE;
        self.cap_value = 0;
        if button == MouseButton::Left && inside && self.enabled(id) {
            match kind {
                Kind::Button => {
                    out.push(raw(id, "OnClick", Ev::Click));
                    if self.ctl[id].modal_result != 0 {
                        out.push(raw(id, "@ModalResult", Ev::Click));
                    }
                }
                Kind::SpeedButton => {
                    self.speed_click(id);
                    out.push(raw(id, "OnClick", Ev::Click));
                }
                Kind::CheckBox => {
                    let c = &mut self.ctl[id];
                    c.state = match (c.state, c.allow_grayed) {
                        (0, _) => 1,
                        (1, true) => 2,
                        _ => 0,
                    };
                    c.checked = c.state == 1;
                    out.push(raw(id, "OnClick", Ev::Click));
                }
                Kind::RadioButton => {
                    if !self.ctl[id].checked {
                        self.radio_check(id);
                        out.push(raw(id, "OnClick", Ev::Click));
                    }
                }
                Kind::RadioGroup => {
                    let p = self.part_at(id, x, y);
                    if p >= 0 && p == part && self.ctl[id].item_index != p {
                        self.ctl[id].item_index = p;
                        out.push(raw(id, "OnClick", Ev::Click));
                    }
                }
                Kind::ColorButton => out.push(raw(id, "@ColorDialog", Ev::Click)),
                Kind::ComboBox | Kind::ListBox | Kind::TrackBar | Kind::UpDown | Kind::ScrollBar | Kind::PageControl | Kind::TabControl
                | Kind::StringGrid | Kind::ListView => {}
                Kind::Memo | Kind::Edit if part != PART_NONE => {}
                _ => out.push(raw(id, "OnClick", Ev::Click)),
            }
        }
        if self.enabled(id) {
            out.push(raw(id, "OnMouseUp", Ev::MouseUp { x: lx, y: ly, button, shift }));
        }
        if button == MouseButton::Right && inside {
            // the popup menu of the control or of a parent
            let mut k = Some(id);
            while let Some(i) = k {
                if let Some(m) = self.ctl[i].popup_menu.clone() {
                    if let Some(mid) = self.id(&m) {
                        out.push(raw(mid, "OnPopup", Ev::Click));
                        self.open_menu(mid, id, x, y);
                    }
                    break;
                }
                k = self.ctl[i].parent;
            }
        }
        self.hot = self.hit(x, y);
        self.hot_part = self.hot.map(|h| self.part_at(h, x, y)).unwrap_or(PART_NONE);
        out
    }

    /// A wheel event of whole notches.
    pub fn wheel(&mut self, delta: i32, x: i32, y: i32, shift: u8) -> Vec<Raw> {
        self.wheel_by(delta, wheel_steps(&mut 0, delta), x, y, shift)
    }

    /// `steps`: the whole notches of `delta` (see `wheel_steps`), the
    /// controls scroll by them; `OnMouseWheel` gets `delta` itself.
    pub fn wheel_by(&mut self, delta: i32, steps: i32, x: i32, y: i32, shift: u8) -> Vec<Raw> {
        let mut out = Vec::new();
        self.layout();
        self.hint = None;
        if let Some(Popup::Combo { id, hot, top }) = self.popup {
            let n = self.ctl[id].items.len() as i32;
            let lr = combo_list_rect(self, id);
            let ih = line_height(&self.font(id)) + 2;
            let vis = (lr.h - 2) / ih;
            let t = (top - steps * 3).clamp(0, (n - vis).max(0));
            self.popup = Some(Popup::Combo { id, hot, top: t });
            self.dirty = true;
            return out;
        }
        if self.popup.is_some() {
            return out;
        }
        // A closed drop-down list follows the normal desktop convention:
        // wheeling over its text area changes the selected item.  Handle it
        // before the form-wide wheel handler so an unrelated focused edit
        // does not consume the wheel while the pointer is over the combo.
        if let Some(id) = self.hit(x, y) {
            if self.enabled(id) && self.ctl[id].kind == Kind::ComboBox && !self.ctl[id].items.is_empty() {
                let n = self.ctl[id].items.len() as i32;
                let i = (self.ctl[id].item_index - steps).clamp(0, n - 1);
                if i != self.ctl[id].item_index {
                    self.combo_select(id, i, &mut out);
                    self.dirty = true;
                }
                return out;
            }
        }
        out.push(raw(0, "OnMouseWheel", Ev::Wheel { delta, x, y, shift }));
        out.push(raw(0, if delta > 0 { "OnMouseWheelUp" } else { "OnMouseWheelDown" }, Ev::Wheel { delta, x, y, shift }));
        // the control under the mouse scrolls
        let mut k = self.hit(x, y);
        while let Some(id) = k {
            let (lx, ly) = self.local(id, x, y);
            if id != 0 && self.has_handler(id, "OnMouseWheel") {
                out.push(raw(id, "OnMouseWheel", Ev::Wheel { delta, x: lx, y: ly, shift }));
                break;
            }
            if !self.enabled(id) {
                k = self.ctl[id].parent;
                continue;
            }
            let kind = self.ctl[id].kind;
            let done = match kind {
                Kind::ListBox => {
                    let r = self.abs_rect(id);
                    let c = &mut self.ctl[id];
                    let vis = ((r.h - 4) / c.item_height.max(8)).max(1);
                    c.top_index = (c.top_index - steps * 3).clamp(0, (c.items.len() as i32 - vis).max(0));
                    true
                }
                Kind::Memo => {
                    let (_, max) = self.memo_range(id);
                    let c = &mut self.ctl[id];
                    c.scroll_y = (c.scroll_y - steps * 3).clamp(0, max as i32);
                    true
                }
                Kind::ListView => {
                    let (_, _, _, max, top) = self.listview_rows(id);
                    self.ctl[id].scroll_y = (top - steps * 3).clamp(0, max);
                    true
                }
                Kind::TrackBar if self.focus == Some(id) => {
                    let c = &mut self.ctl[id];
                    let old = c.position;
                    let v = c.position + if c.vertical { -steps } else { steps } as i64 * c.line_size.max(1) * -1;
                    c.set_position(v);
                    if c.position != old {
                        out.push(raw(id, "OnChange", Ev::Change));
                    }
                    true
                }
                Kind::ScrollBox => {
                    let (vs, hs) = scrollbox_bars(self, id);
                    if vs {
                        let (_, ey) = scrollbox_extent(self, id);
                        let c = &mut self.ctl[id];
                        let ch = c.height - if c.border { 4 } else { 0 } - if hs { SB } else { 0 };
                        c.scroll_y = (c.scroll_y - steps * 40).clamp(0, (ey - ch).max(0));
                        true
                    } else {
                        false
                    }
                }
                Kind::CategoryPanelGroup => false,
                _ => false,
            };
            if done {
                self.dirty = true;
                break;
            }
            k = self.ctl[id].parent;
        }
        out
    }

    /// Auto-repeat of held arrow buttons; returns events when it fired.
    pub fn tick(&mut self, now: Instant) -> Vec<Raw> {
        let mut out = Vec::new();
        let Some(t) = self.repeat_at else { return out };
        if now < t {
            return out;
        }
        let Some(id) = self.capture else {
            self.repeat_at = None;
            return out;
        };
        if self.hot != Some(id) {
            self.repeat_at = Some(now + REPEAT_NEXT);
            return out;
        }
        match self.ctl[id].kind {
            Kind::UpDown => self.updown_step(id, self.cap_part == PART_UP, &mut out),
            Kind::ScrollBar | Kind::ListBox | Kind::ListView | Kind::Memo => {
                if self.cap_part != PART_THUMB {
                    self.scroll_part(id, self.cap_part, &mut out)
                }
            }
            Kind::TrackBar => {
                let (x, y) = self.mouse;
                if self.cap_part != PART_THUMB {
                    self.track_page(id, x, y, &mut out)
                }
            }
            _ => {}
        }
        self.repeat_at = Some(now + REPEAT_NEXT);
        self.dirty = true;
        out
    }

    // ---- control helpers

    fn speed_click(&mut self, id: Id) {
        let (gi, parent, down, all_up) = {
            let c = &self.ctl[id];
            (c.group_index, c.parent, c.down, c.allow_all_up)
        };
        if gi == 0 {
            return;
        }
        if down {
            if all_up {
                self.ctl[id].down = false;
            }
        } else {
            if let Some(p) = parent {
                for k in self.ctl[p].children.clone() {
                    let c = &mut self.ctl[k];
                    if k != id && c.kind == Kind::SpeedButton && c.group_index == gi {
                        c.down = false;
                    }
                }
            }
            self.ctl[id].down = true;
        }
    }

    pub fn radio_check(&mut self, id: Id) {
        if let Some(p) = self.ctl[id].parent {
            for k in self.ctl[p].children.clone() {
                if self.ctl[k].kind == Kind::RadioButton {
                    self.ctl[k].checked = false;
                    self.ctl[k].state = 0;
                }
            }
        }
        self.ctl[id].checked = true;
        self.ctl[id].state = 1;
        self.dirty = true;
    }

    fn updown_step(&mut self, id: Id, up: bool, out: &mut Vec<Raw>) {
        // the associated edit's text is the current value
        let assoc = self.ctl[id].associate.clone().and_then(|a| self.id(&a));
        if let Some(e) = assoc {
            let t: String = self.ctl[e].text.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
            if let Ok(v) = t.parse::<i64>() {
                self.ctl[id].position = v;
            }
        }
        let c = &mut self.ctl[id];
        let inc = c.increment.max(1);
        let mut p = c.position + if up { inc } else { -inc };
        if p > c.max {
            p = if c.wrap { c.min } else { c.max };
        }
        if p < c.min {
            p = if c.wrap { c.max } else { c.min };
        }
        let changed = p != c.position;
        c.position = p;
        if changed {
            out.push(raw(id, "OnChanging", Ev::UpDown { up }));
            out.push(raw(id, "OnChangingEx", Ev::UpDown { up }));
        }
        if let Some(e) = assoc {
            let s = if self.ctl[id].thousands && p.abs() >= 1000 { thousands(p) } else { p.to_string() };
            if changed {
                self.ctl[e].set_text(&s);
                out.push(raw(e, "OnChange", Ev::Change));
            }
        }
        out.push(raw(id, "OnClick", Ev::UpDown { up }));
        self.dirty = true;
    }

    fn track_page(&mut self, id: Id, x: i32, y: i32, out: &mut Vec<Raw>) {
        let r = self.abs_rect(id);
        let c = &self.ctl[id];
        let (_, thumb, _, _) = track_geom(c);
        let thumb = thumb.offset(r.x, r.y);
        let before = if c.vertical { y < thumb.y } else { x < thumb.x };
        let after = if c.vertical { y >= thumb.bottom() } else { x >= thumb.right() };
        if !before && !after {
            return;
        }
        let ps = c.page_size.max(1);
        let v = c.position + if before { -ps } else { ps };
        let old = c.position;
        self.ctl[id].set_position(v);
        if self.ctl[id].position != old {
            out.push(raw(id, "OnChange", Ev::Change));
        }
    }

    /// Scroll bar part action on a scroll bar, list box or memo.
    fn scroll_part(&mut self, id: Id, part: i32, out: &mut Vec<Raw>) {
        let kind = self.ctl[id].kind;
        let r = self.abs_rect(id);
        match kind {
            Kind::ScrollBar => {
                let c = &mut self.ctl[id];
                let page = c.page_size.max(1);
                let d = match part {
                    PART_UP => -c.line_size.max(1),
                    PART_DOWN => c.line_size.max(1),
                    PART_PAGE_UP => -c.large_change.max(1),
                    PART_PAGE_DOWN => c.large_change.max(1),
                    _ => 0,
                };
                let v = (c.position + d).clamp(c.min, (c.max - page + 1).max(c.min));
                if v != c.position {
                    c.position = v;
                    out.push(raw(id, "OnScroll", Ev::Scroll));
                    out.push(raw(id, "OnChange", Ev::Change));
                }
            }
            Kind::ListBox => {
                let c = &mut self.ctl[id];
                let vis = ((r.h - 4) / c.item_height.max(8)).max(1);
                let max = (c.items.len() as i32 - vis).max(0);
                let d = match part {
                    PART_UP => -1,
                    PART_DOWN => 1,
                    PART_PAGE_UP => -vis,
                    PART_PAGE_DOWN => vis,
                    PART_THUMB => {
                        self.cap_value = c.top_index as i64;
                        0
                    }
                    _ => 0,
                };
                c.top_index = (c.top_index + d).clamp(0, max);
            }
            Kind::ListView => {
                let (_, _, vis, max, top) = self.listview_rows(id);
                let d = match part {
                    PART_UP => -1,
                    PART_DOWN => 1,
                    PART_PAGE_UP => -vis,
                    PART_PAGE_DOWN => vis,
                    PART_THUMB => {
                        self.cap_value = top as i64;
                        0
                    }
                    _ => 0,
                };
                self.ctl[id].scroll_y = (top + d).clamp(0, max);
            }
            Kind::Memo => {
                let (vis, max) = self.memo_range(id);
                let c = &mut self.ctl[id];
                let d = match part {
                    PART_UP => -1,
                    PART_DOWN => 1,
                    PART_PAGE_UP => -(vis as i32),
                    PART_PAGE_DOWN => vis as i32,
                    PART_THUMB => {
                        self.cap_value = c.scroll_y as i64;
                        0
                    }
                    _ => 0,
                };
                c.scroll_y = (c.scroll_y + d).clamp(0, max as i32);
            }
            _ => {}
        }
        self.dirty = true;
    }

    fn scrollbox_click(&mut self, id: Id, x: i32, y: i32, out: &mut Vec<Raw>) {
        let _ = out;
        let r = self.abs_rect(id);
        let c = &self.ctl[id];
        let b = if c.border { 2 } else { 0 };
        let (vs, hs) = scrollbox_bars(self, id);
        let (ex, ey) = scrollbox_extent(self, id);
        let cw = r.w - 2 * b - if vs { SB } else { 0 };
        let ch = r.h - 2 * b - if hs { SB } else { 0 };
        let vert = self.cap_part == PART_VSCROLL;
        let sr = if vert { Rect::new(r.right() - b - SB, r.y + b, SB, ch) } else { Rect::new(r.x + b, r.bottom() - b - SB, cw, SB) };
        let (max, page, pos) = if vert { ((ey - ch).max(0), ch, c.scroll_y) } else { ((ex - cw).max(0), cw, c.scroll_x) };
        let part = sb_part(sr, vert, 0, max as i64, page as i64, pos as i64, x, y);
        let d = match part {
            PART_UP => -20,
            PART_DOWN => 20,
            PART_PAGE_UP => -page,
            PART_PAGE_DOWN => page,
            PART_THUMB => {
                self.cap_value = 1 + pos as i64;
                0
            }
            _ => 0,
        };
        let c = &mut self.ctl[id];
        if vert {
            c.scroll_y = (c.scroll_y + d).clamp(0, max);
        } else {
            c.scroll_x = (c.scroll_x + d).clamp(0, max);
        }
        self.dirty = true;
    }

    fn scrollbox_drag(&mut self, id: Id, x: i32, y: i32) {
        let r = self.abs_rect(id);
        let c = &self.ctl[id];
        let b = if c.border { 2 } else { 0 };
        let (vs, hs) = scrollbox_bars(self, id);
        let (ex, ey) = scrollbox_extent(self, id);
        let cw = r.w - 2 * b - if vs { SB } else { 0 };
        let ch = r.h - 2 * b - if hs { SB } else { 0 };
        let vert = self.cap_part == PART_VSCROLL;
        let (max, page, len, d) = if vert {
            ((ey - ch).max(0), ch, ch - 2 * SB, y - self.cap_start.1)
        } else {
            ((ex - cw).max(0), cw, cw - 2 * SB, x - self.cap_start.0)
        };
        let tl = ((page as f64 / (max + page).max(1) as f64) * len as f64).max(8.0);
        let v = ((self.cap_value - 1) as f64 + d as f64 / (len as f64 - tl).max(1.0) * max as f64).round() as i32;
        let c = &mut self.ctl[id];
        if vert {
            c.scroll_y = v.clamp(0, max);
        } else {
            c.scroll_x = v.clamp(0, max);
        }
        self.dirty = true;
    }

    fn listbox_select_at(&mut self, id: Id, y: i32, shift: u8, out: &mut Vec<Raw>) {
        let r = self.abs_rect(id);
        let c = &mut self.ctl[id];
        let inner_y = r.y + if c.border { 2 } else { 0 };
        let i = (y - inner_y) / c.item_height.max(8) + c.top_index;
        if i < 0 || i as usize >= c.items.len() {
            return;
        }
        if c.multi_select {
            c.selected.resize(c.items.len(), false);
            if shift & SS_CTRL != 0 {
                c.selected[i as usize] = !c.selected[i as usize];
            } else if shift & SS_SHIFT != 0 && c.item_index >= 0 {
                let (a, b) = (c.item_index.min(i), c.item_index.max(i));
                for k in 0..c.items.len() {
                    c.selected[k] = (k as i32) >= a && (k as i32) <= b;
                }
            } else {
                for s in c.selected.iter_mut() {
                    *s = false;
                }
                c.selected[i as usize] = true;
            }
        }
        if c.item_index != i || c.multi_select {
            c.item_index = i;
            out.push(raw(id, "OnClick", Ev::Click));
        }
        self.dirty = true;
    }

    fn open_combo(&mut self, id: Id, out: &mut Vec<Raw>) {
        if matches!(self.popup, Some(Popup::Combo { id: p, .. }) if p == id) {
            self.popup = None;
            return;
        }
        out.push(raw(id, "OnDropDown", Ev::DropDown));
        let c = &self.ctl[id];
        let n = c.items.len() as i32;
        let vis = paint::combo_rows(self, id).max(1);
        let hot = c.item_index;
        let top = if hot >= vis { (hot - vis / 2).clamp(0, (n - vis).max(0)) } else { 0 };
        self.popup = Some(Popup::Combo { id, hot, top });
        self.dirty = true;
    }

    fn combo_select(&mut self, id: Id, i: i32, out: &mut Vec<Raw>) {
        if i < 0 || i as usize >= self.ctl[id].items.len() {
            return;
        }
        let changed = self.ctl[id].item_index != i;
        self.ctl[id].set_item_index(i);
        out.push(raw(id, "OnSelect", Ev::Select));
        if changed {
            out.push(raw(id, "OnChange", Ev::Change));
        }
        out.push(raw(id, "OnClick", Ev::Click));
        self.dirty = true;
    }

    /// Opens a popup menu at client coordinates.
    pub fn open_menu(&mut self, menu: Id, owner: Id, x: i32, y: i32) {
        let (w, h) = menu_size(self, menu);
        let (cw, ch) = self.client_size();
        let x = if x + w > cw { (x - w).max(0) } else { x };
        let y = if y + h > ch { (ch - h).max(0) } else { y };
        self.popup_owner = Some(owner);
        self.popup = Some(Popup::Menu { owner, levels: vec![MenuLevel { parent: menu, rect: Rect::new(x, y, w, h), hot: -1 }] });
        self.hint = None;
        self.dirty = true;
    }

    fn menu_hover(&self, levels: &mut Vec<MenuLevel>, x: i32, y: i32) {
        // the deepest level under the mouse
        let Some(li) = levels.iter().rposition(|lv| lv.rect.contains(x, y)) else { return };
        let rects = menu_item_rects(self, &levels[li]);
        let hot = rects.iter().position(|(_, r)| r.contains(x, y)).map(|i| i as i32).unwrap_or(-1);
        levels[li].hot = hot;
        levels.truncate(li + 1);
        if hot >= 0 {
            let (k, r) = rects[hot as usize];
            if !menu_items(self, k).is_empty() && self.ctl[k].enabled {
                let (w, h) = menu_size(self, k);
                let (cw, ch) = self.client_size();
                let lr = levels[li].rect;
                let nx = if lr.right() + w > cw { (lr.x - w).max(0) } else { lr.right() - 2 };
                let ny = if r.y + h > ch { (ch - h).max(0) } else { r.y - 2 };
                levels.push(MenuLevel { parent: k, rect: Rect::new(nx, ny, w, h), hot: -1 });
            }
        }
    }

    fn popup_contains(&self, p: &Popup, x: i32, y: i32) -> bool {
        match p {
            Popup::Combo { id, .. } => combo_list_rect(self, *id).contains(x, y),
            Popup::Menu { levels, .. } => levels.iter().any(|l| l.rect.contains(x, y)),
        }
    }

    // ---- text editing

    fn edit_text_rect(&self, id: Id) -> Rect {
        let r = self.abs_rect(id);
        let c = &self.ctl[id];
        let b = if c.border || c.kind == Kind::ComboBox { 2 } else { 0 };
        let right = if c.kind == Kind::ComboBox { 18 } else { 0 };
        Rect::new(r.x + b + 1, r.y + b, r.w - 2 * b - 2 - right, r.h - 2 * b)
    }

    fn edit_pos_at(&self, id: Id, x: i32) -> usize {
        let c = &self.ctl[id];
        let tr = self.edit_text_rect(id);
        let font = self.font(id);
        let shown: String = match c.password_char {
            Some(p) => c.text.chars().map(|_| p).collect(),
            None => c.text.clone(),
        };
        let xs = char_xs(&font, &shown);
        let total = *xs.last().unwrap_or(&0);
        let base = match c.alignment {
            super::canvas::HAlign::Right if c.kind == Kind::Edit && total < tr.w => tr.right() - total - 1,
            super::canvas::HAlign::Center if c.kind == Kind::Edit && total < tr.w => tr.x + (tr.w - total) / 2,
            _ => tr.x - c.scroll_x,
        };
        nearest(&xs, x - base)
    }

    fn memo_pos_at(&self, id: Id, x: i32, y: i32) -> usize {
        let c = &self.ctl[id];
        let r = self.abs_rect(id);
        let b = if c.border { 2 } else { 0 };
        let font = self.font(id);
        let lh = line_height(&font).max(1);
        let lines = memo_lines(self, id);
        let li = ((y - r.y - b - 1) / lh + c.scroll_y).clamp(0, lines.len() as i32 - 1).max(0) as usize;
        let Some((start, text)) = lines.get(li) else { return 0 };
        let xs = char_xs(&font, text);
        start + nearest(&xs, x - (r.x + b + 2 - c.scroll_x))
    }

    /// Scrolls an edit or memo so that the caret is visible.
    pub fn ensure_caret_visible(&mut self, id: Id) {
        let kind = self.ctl[id].kind;
        let font = self.font(id);
        if kind == Kind::Memo {
            let lines = memo_lines(self, id);
            let caret = self.ctl[id].caret;
            let li = lines.iter().rposition(|(s, _)| *s <= caret).unwrap_or(0) as i32;
            let (vis, _) = self.memo_range(id);
            let c = &mut self.ctl[id];
            if li < c.scroll_y {
                c.scroll_y = li;
            } else if li >= c.scroll_y + vis as i32 {
                c.scroll_y = li - vis as i32 + 1;
            }
            // horizontal (no word wrap)
            if let Some((s, text)) = lines.get(li as usize) {
                let xs = char_xs(&font, text);
                let cx = xs[(caret - s).min(xs.len() - 1)];
                let w = c.width - 8 - if c.scroll_bars & 2 != 0 { SB } else { 0 };
                if cx - c.scroll_x > w {
                    c.scroll_x = cx - w + 20;
                } else if cx < c.scroll_x {
                    c.scroll_x = (cx - 20).max(0);
                }
            }
            return;
        }
        let tr = self.edit_text_rect(id);
        let c = &mut self.ctl[id];
        let shown: String = match c.password_char {
            Some(p) => c.text.chars().map(|_| p).collect(),
            None => c.text.clone(),
        };
        let xs = char_xs(&font, &shown);
        let cx = xs[c.caret.min(xs.len() - 1)];
        let total = *xs.last().unwrap_or(&0);
        if cx - c.scroll_x > tr.w - 2 {
            c.scroll_x = cx - tr.w + 2;
        } else if cx < c.scroll_x {
            c.scroll_x = cx;
        }
        if total - c.scroll_x < tr.w - 2 {
            c.scroll_x = (total - tr.w + 2).max(0);
        }
    }

    fn editable(&self, id: Id) -> bool {
        let c = &self.ctl[id];
        match c.kind {
            Kind::Edit | Kind::Memo => !c.read_only,
            Kind::ComboBox => c.style.is_empty() || c.style == "csDropDown" || c.style == "csSimple",
            _ => false,
        }
    }

    fn delete_selection(&mut self, id: Id) -> bool {
        let c = &mut self.ctl[id];
        let (a, b) = c.selection();
        if a == b {
            return false;
        }
        let mut ch = c.chars();
        ch.drain(a..b.min(ch.len()));
        c.text = ch.into_iter().collect();
        c.caret = a;
        c.sel_anchor = a;
        true
    }

    fn insert_text(&mut self, id: Id, s: &str) -> bool {
        let max_len = self.ctl[id].max_length;
        let numbers = self.ctl[id].numbers_only;
        let multi = self.ctl[id].kind == Kind::Memo;
        let s: String = s.chars().filter(|ch| (multi && (*ch == '\n' || *ch == '\t')) || !ch.is_control()).filter(|ch| !numbers || ch.is_ascii_digit()).collect();
        let had = self.delete_selection(id);
        if s.is_empty() {
            return had;
        }
        let c = &mut self.ctl[id];
        let mut ch = c.chars();
        let room = if max_len > 0 { max_len.saturating_sub(ch.len()) } else { usize::MAX };
        let ins: Vec<char> = s.chars().take(room).collect();
        if ins.is_empty() {
            return had;
        }
        let at = c.caret.min(ch.len());
        let n = ins.len();
        ch.splice(at..at, ins);
        c.text = ch.into_iter().collect();
        c.caret = at + n;
        c.sel_anchor = c.caret;
        true
    }

    fn text_changed(&mut self, id: Id, out: &mut Vec<Raw>) {
        self.ensure_caret_visible(id);
        out.push(raw(id, "OnChange", Ev::Change));
        self.dirty = true;
    }

    /// Text typed on the keyboard (one or more characters).
    pub fn text_input(&mut self, s: &str) -> Vec<Raw> {
        let mut out = Vec::new();
        if self.popup.is_some() {
            return out;
        }
        for ch in s.chars() {
            if ch.is_control() {
                continue;
            }
            if self.key_preview && self.has_handler(0, "OnKeyPress") {
                out.push(raw(0, "OnKeyPress", Ev::KeyPress(ch)));
            }
            let Some(id) = self.focus else { continue };
            out.push(raw(id, "OnKeyPress", Ev::KeyPress(ch)));
            if self.editable(id) {
                if self.insert_text(id, &ch.to_string()) {
                    self.text_changed(id, &mut out);
                }
            } else if self.ctl[id].kind == Kind::ListBox || (self.ctl[id].kind == Kind::ComboBox && !self.editable(id)) {
                // jump to the next item starting with the character
                let c = &self.ctl[id];
                let lc = ch.to_lowercase().next().unwrap_or(ch);
                let n = c.items.len();
                let start = (c.item_index + 1).max(0) as usize;
                if let Some(i) = (0..n).map(|k| (start + k) % n.max(1)).find(|&i| c.items[i].to_lowercase().starts_with(lc)) {
                    if c.kind == Kind::ComboBox {
                        self.combo_select(id, i as i32, &mut out);
                    } else {
                        self.ctl[id].item_index = i as i32;
                        out.push(raw(id, "OnClick", Ev::Click));
                    }
                }
            } else if (ch == ' ') && matches!(self.ctl[id].kind, Kind::Button | Kind::CheckBox | Kind::RadioButton) {
                out.extend(self.click_control(id));
            }
        }
        out
    }

    /// A programmatic click as the keyboard does it (Space / Enter).
    pub fn click_control(&mut self, id: Id) -> Vec<Raw> {
        let mut out = Vec::new();
        if !self.enabled(id) || !self.showing(id) {
            return out;
        }
        match self.ctl[id].kind {
            Kind::CheckBox => {
                let c = &mut self.ctl[id];
                c.state = if c.state == 1 { 0 } else { 1 };
                c.checked = c.state == 1;
                out.push(raw(id, "OnClick", Ev::Click));
            }
            Kind::RadioButton => {
                if !self.ctl[id].checked {
                    self.radio_check(id);
                    out.push(raw(id, "OnClick", Ev::Click));
                }
            }
            Kind::SpeedButton => {
                self.speed_click(id);
                out.push(raw(id, "OnClick", Ev::Click));
            }
            _ => {
                out.push(raw(id, "OnClick", Ev::Click));
                if self.ctl[id].modal_result != 0 {
                    out.push(raw(id, "@ModalResult", Ev::Click));
                }
            }
        }
        self.dirty = true;
        out
    }

    pub fn key_down(&mut self, key: u16, shift: u8) -> Vec<Raw> {
        let mut out = Vec::new();
        self.hint = None;
        self.dirty = true;
        self.caret_on = true;
        // popups
        if let Some(p) = self.popup.clone() {
            match p {
                Popup::Combo { id, hot, top } => {
                    let n = self.ctl[id].items.len() as i32;
                    let vis = paint::combo_rows(self, id).max(1);
                    let h = match key {
                        vk::UP => (hot - 1).max(0),
                        vk::DOWN => (hot + 1).min(n - 1),
                        vk::PRIOR => (hot - vis).max(0),
                        vk::NEXT => (hot + vis).min(n - 1),
                        vk::HOME => 0,
                        vk::END => n - 1,
                        vk::RETURN => {
                            self.popup = None;
                            self.combo_select(id, hot, &mut out);
                            return out;
                        }
                        vk::ESCAPE | vk::F4 | vk::TAB => {
                            self.popup = None;
                            return out;
                        }
                        _ => hot,
                    };
                    let t = if h < top { h } else if h >= top + vis { h - vis + 1 } else { top };
                    self.popup = Some(Popup::Combo { id, hot: h, top: t.max(0) });
                }
                Popup::Menu { owner, mut levels } => {
                    let li = levels.len() - 1;
                    let items = menu_item_rects(self, &levels[li]);
                    let n = items.len() as i32;
                    match key {
                        vk::ESCAPE => {
                            levels.pop();
                            if levels.is_empty() {
                                self.popup = None;
                                return out;
                            }
                        }
                        vk::UP | vk::DOWN => {
                            let d = if key == vk::UP { -1 } else { 1 };
                            let mut h = levels[li].hot;
                            for _ in 0..n {
                                h = (h + d).rem_euclid(n.max(1));
                                if self.ctl[items[h as usize].0].caption != "-" {
                                    break;
                                }
                            }
                            levels[li].hot = h;
                        }
                        vk::RETURN | vk::RIGHT => {
                            if levels[li].hot >= 0 {
                                let (k, r) = items[levels[li].hot as usize];
                                if !menu_items(self, k).is_empty() {
                                    let (w, h) = menu_size(self, k);
                                    let lr = levels[li].rect;
                                    levels.push(MenuLevel { parent: k, rect: Rect::new(lr.right() - 2, r.y - 2, w, h), hot: 0 });
                                } else if key == vk::RETURN && self.ctl[k].enabled {
                                    self.popup = None;
                                    out.push(raw(k, "OnClick", Ev::Click));
                                    return out;
                                }
                            }
                        }
                        vk::LEFT if levels.len() > 1 => {
                            levels.pop();
                        }
                        _ => {}
                    }
                    self.popup_owner = Some(owner);
        self.popup = Some(Popup::Menu { owner, levels });
                }
            }
            return out;
        }
        if self.key_preview {
            out.push(raw(0, "OnKeyDown", Ev::KeyDown { key, shift }));
            if matches!(key, vk::RETURN | vk::ESCAPE | vk::BACK) {
                out.push(raw(0, "OnKeyPress", Ev::KeyPress(key as u8 as char)));
            }
        }
        let focus = self.focus;
        if let Some(id) = focus {
            out.push(raw(id, "OnKeyDown", Ev::KeyDown { key, shift }));
            if matches!(key, vk::RETURN | vk::ESCAPE | vk::BACK) {
                out.push(raw(id, "OnKeyPress", Ev::KeyPress(key as u8 as char)));
            }
        }
        let ctrl = shift & SS_CTRL != 0;
        let sh = shift & SS_SHIFT != 0;
        // keyboard navigation
        if key == vk::TAB && !ctrl && !(focus.map(|f| self.ctl[f].kind == Kind::Memo && self.ctl[f].want_tabs).unwrap_or(false)) {
            self.focus_next(sh, &mut out);
            return out;
        }
        if key == vk::TAB && ctrl {
            // next page of the page control containing the focus
            let mut k = focus;
            while let Some(i) = k {
                if self.ctl[i].kind == Kind::PageControl {
                    let tabs = self.tab_rects(i);
                    if let Some(p) = tabs.iter().position(|(s, _)| Some(*s) == self.ctl[i].active_page) {
                        let n = tabs.len();
                        let next = if sh { (p + n - 1) % n } else { (p + 1) % n };
                        self.ctl[i].active_page = Some(tabs[next].0);
                        out.push(raw(i, "OnChange", Ev::Change));
                    }
                    break;
                }
                k = self.ctl[i].parent;
            }
            return out;
        }
        let Some(id) = focus else {
            if key == vk::RETURN || key == vk::ESCAPE {
                self.default_button(key, &mut out);
            }
            return out;
        };
        let kind = self.ctl[id].kind;
        // up-down buttons attached to the focused edit
        if kind == Kind::Edit && (key == vk::UP || key == vk::DOWN) {
            let name = self.ctl[id].name.to_ascii_lowercase();
            if let Some(ud) = (0..self.ctl.len()).find(|&k| self.ctl[k].kind == Kind::UpDown && self.ctl[k].associate.as_deref().map(|a| a.to_ascii_lowercase()) == Some(name.clone())) {
                if self.enabled(ud) {
                    self.updown_step(ud, key == vk::UP, &mut out);
                }
                return out;
            }
        }
        match kind {
            Kind::Edit | Kind::Memo | Kind::ComboBox if self.editable(id) || kind != Kind::ComboBox => {
                if kind == Kind::ComboBox && (key == vk::UP || key == vk::DOWN) {
                    let c = &self.ctl[id];
                    if !c.items.is_empty() {
                        let i = (c.item_index + if key == vk::UP { -1 } else { 1 }).clamp(0, c.items.len() as i32 - 1);
                        self.combo_select(id, i, &mut out);
                    }
                    return out;
                }
                if kind == Kind::ComboBox && (key == vk::F4 || (key == vk::DOWN && shift & SS_ALT != 0)) {
                    self.open_combo(id, &mut out);
                    return out;
                }
                self.edit_key(id, key, shift, &mut out);
            }
            Kind::ComboBox => match key {
                vk::UP | vk::DOWN | vk::HOME | vk::END | vk::PRIOR | vk::NEXT if shift & SS_ALT == 0 => {
                    let c = &self.ctl[id];
                    let n = c.items.len() as i32;
                    if n == 0 {
                        return out;
                    }
                    let i = match key {
                        vk::UP => c.item_index - 1,
                        vk::DOWN => c.item_index + 1,
                        vk::HOME => 0,
                        vk::END => n - 1,
                        vk::PRIOR => c.item_index - c.drop_down_count,
                        _ => c.item_index + c.drop_down_count,
                    }
                    .clamp(0, n - 1);
                    if i != c.item_index {
                        self.combo_select(id, i, &mut out);
                    }
                }
                vk::F4 | vk::DOWN => self.open_combo(id, &mut out),
                vk::RETURN | vk::ESCAPE => self.default_button(key, &mut out),
                _ => {}
            },
            Kind::ListBox => {
                let r = self.abs_rect(id);
                let c = &mut self.ctl[id];
                let n = c.items.len() as i32;
                let vis = ((r.h - 4) / c.item_height.max(8)).max(1);
                let i = match key {
                    vk::UP => c.item_index - 1,
                    vk::DOWN => c.item_index + 1,
                    vk::HOME => 0,
                    vk::END => n - 1,
                    vk::PRIOR => c.item_index - vis,
                    vk::NEXT => c.item_index + vis,
                    vk::RETURN | vk::ESCAPE => {
                        self.default_button(key, &mut out);
                        return out;
                    }
                    _ => c.item_index,
                }
                .clamp(0, (n - 1).max(0));
                if n > 0 && i != c.item_index {
                    c.item_index = i;
                    if i < c.top_index {
                        c.top_index = i;
                    } else if i >= c.top_index + vis {
                        c.top_index = i - vis + 1;
                    }
                    out.push(raw(id, "OnClick", Ev::Click));
                }
            }
            Kind::TrackBar => {
                let c = &mut self.ctl[id];
                let old = c.position;
                let v = match key {
                    vk::LEFT | vk::UP => c.position - c.line_size.max(1),
                    vk::RIGHT | vk::DOWN => c.position + c.line_size.max(1),
                    vk::PRIOR => c.position - c.page_size.max(1),
                    vk::NEXT => c.position + c.page_size.max(1),
                    vk::HOME => c.min,
                    vk::END => c.max,
                    vk::RETURN | vk::ESCAPE => {
                        self.default_button(key, &mut out);
                        return out;
                    }
                    _ => c.position,
                };
                c.set_position(v);
                if c.position != old {
                    out.push(raw(id, "OnChange", Ev::Change));
                }
            }
            Kind::RadioButton if matches!(key, vk::UP | vk::DOWN | vk::LEFT | vk::RIGHT) => {
                if let Some(p) = self.ctl[id].parent {
                    let sibs: Vec<Id> = self.ctl[p].children.iter().copied().filter(|&k| self.ctl[k].kind == Kind::RadioButton && self.showing(k) && self.enabled(k)).collect();
                    if let Some(pos) = sibs.iter().position(|&k| k == id) {
                        let n = sibs.len();
                        let next = if key == vk::UP || key == vk::LEFT { sibs[(pos + n - 1) % n] } else { sibs[(pos + 1) % n] };
                        self.set_focus(Some(next), &mut out);
                        self.radio_check(next);
                        out.push(raw(next, "OnClick", Ev::Click));
                    }
                }
            }
            Kind::RadioGroup if matches!(key, vk::UP | vk::DOWN | vk::LEFT | vk::RIGHT) => {
                let c = &mut self.ctl[id];
                let n = c.items.len() as i32;
                let i = (c.item_index + if key == vk::UP || key == vk::LEFT { -1 } else { 1 }).clamp(0, (n - 1).max(0));
                if n > 0 && i != c.item_index {
                    c.item_index = i;
                    out.push(raw(id, "OnClick", Ev::Click));
                }
            }
            Kind::Button if key == vk::RETURN => out.extend(self.click_control(id)),
            Kind::ListView | Kind::StringGrid if matches!(key, vk::UP | vk::DOWN) => {
                let c = &mut self.ctl[id];
                let d = if key == vk::UP { -1 } else { 1 };
                if kind == Kind::ListView {
                    let i = (c.item_index + d).clamp(0, (c.cells.len() as i32 - 1).max(0));
                    if !c.cells.is_empty() && i != c.item_index {
                        c.item_index = i;
                        out.push(raw(id, "OnChange", Ev::Change));
                    }
                    // the selected row stays in view
                    let (_, _, vis, max, top) = self.listview_rows(id);
                    let c = &mut self.ctl[id];
                    c.scroll_y = top.clamp(c.item_index - vis + 1, c.item_index.max(0)).clamp(0, max);
                } else if (c.cells.len() as i32) > c.fixed_rows {
                    c.row = (c.row + d).clamp(c.fixed_rows, c.cells.len() as i32 - 1);
                    out.push(raw(id, "OnSelectCell", Ev::Select));
                }
            }
            _ => {
                if key == vk::RETURN || key == vk::ESCAPE {
                    self.default_button(key, &mut out);
                }
            }
        }
        out
    }

    /// Enter clicks the default button, Escape the cancel button.
    fn default_button(&mut self, key: u16, out: &mut Vec<Raw>) {
        let want_default = key == vk::RETURN;
        let found = (0..self.ctl.len()).find(|&k| {
            let c = &self.ctl[k];
            c.kind == Kind::Button && if want_default { c.default } else { c.cancel } && self.showing(k) && self.enabled(k)
        });
        match found {
            Some(b) => out.extend(self.click_control(b)),
            None if !want_default && self.modal => out.push(raw(0, "@Cancel", Ev::Click)),
            None => {}
        }
    }

    fn edit_key(&mut self, id: Id, key: u16, shift: u8, out: &mut Vec<Raw>) {
        let ctrl = shift & SS_CTRL != 0;
        let sh = shift & SS_SHIFT != 0;
        let kind = self.ctl[id].kind;
        let editable = self.editable(id);
        let n = self.ctl[id].text.chars().count();
        let caret = self.ctl[id].caret;
        let chars = self.ctl[id].chars();
        let move_to = |f: &mut Form, p: usize| {
            let c = &mut f.ctl[id];
            c.caret = p.min(n);
            if !sh {
                c.sel_anchor = c.caret;
            }
            f.ensure_caret_visible(id);
        };
        match key {
            vk::LEFT => {
                let (a, b) = self.ctl[id].selection();
                let p = if !sh && a != b { a } else if ctrl { word_left(&chars, caret) } else { caret.saturating_sub(1) };
                move_to(self, p)
            }
            vk::RIGHT => {
                let (a, b) = self.ctl[id].selection();
                let p = if !sh && a != b { b } else if ctrl { word_right(&chars, caret) } else { (caret + 1).min(n) };
                move_to(self, p)
            }
            vk::HOME | vk::END if kind == Kind::Memo && !ctrl => {
                let lines = memo_lines(self, id);
                let li = lines.iter().rposition(|(s, _)| *s <= caret).unwrap_or(0);
                let (s, t) = &lines[li];
                let p = if key == vk::HOME { *s } else { s + t.chars().count() };
                move_to(self, p)
            }
            vk::HOME => move_to(self, 0),
            vk::END => move_to(self, n),
            vk::UP | vk::DOWN | vk::PRIOR | vk::NEXT if kind == Kind::Memo => {
                let lines = memo_lines(self, id);
                let font = self.font(id);
                let li = lines.iter().rposition(|(s, _)| *s <= caret).unwrap_or(0);
                let (vis, _) = self.memo_range(id);
                let d: i64 = match key {
                    vk::UP => -1,
                    vk::DOWN => 1,
                    vk::PRIOR => -vis.max(1),
                    _ => vis.max(1),
                };
                let nl = (li as i64 + d).clamp(0, lines.len() as i64 - 1) as usize;
                let xs = char_xs(&font, &lines[li].1);
                let x = xs[(caret - lines[li].0).min(xs.len() - 1)];
                let xs2 = char_xs(&font, &lines[nl].1);
                let p = lines[nl].0 + nearest(&xs2, x);
                move_to(self, p)
            }
            vk::BACK if editable => {
                if !self.delete_selection(id) {
                    if caret == 0 {
                        return;
                    }
                    let p = if ctrl { word_left(&chars, caret) } else { caret - 1 };
                    let c = &mut self.ctl[id];
                    let mut ch = chars.clone();
                    ch.drain(p..caret);
                    c.text = ch.into_iter().collect();
                    c.caret = p;
                    c.sel_anchor = p;
                }
                self.text_changed(id, out);
            }
            vk::DELETE if editable => {
                if sh {
                    self.copy_selection(id);
                }
                if !self.delete_selection(id) {
                    if caret >= n {
                        return;
                    }
                    let e = if ctrl { word_right(&chars, caret) } else { caret + 1 };
                    let c = &mut self.ctl[id];
                    let mut ch = chars.clone();
                    ch.drain(caret..e.min(n));
                    c.text = ch.into_iter().collect();
                }
                self.text_changed(id, out);
            }
            vk::RETURN => {
                if kind == Kind::Memo && editable && self.ctl[id].want_returns {
                    self.insert_text(id, "\n");
                    self.text_changed(id, out);
                } else {
                    self.default_button(key, out);
                }
            }
            vk::ESCAPE => self.default_button(key, out),
            vk::TAB if kind == Kind::Memo && editable => {
                self.insert_text(id, "\t");
                self.text_changed(id, out);
            }
            0x41 if ctrl => {
                // Ctrl+A
                let c = &mut self.ctl[id];
                c.sel_anchor = 0;
                c.caret = n;
            }
            0x43 if ctrl => self.copy_selection(id),
            vk::INSERT if ctrl => self.copy_selection(id),
            0x58 if ctrl => {
                self.copy_selection(id);
                if editable && self.delete_selection(id) {
                    self.text_changed(id, out);
                }
            }
            0x56 if ctrl && editable => self.paste(id, out),
            vk::INSERT if sh && editable => self.paste(id, out),
            _ => {}
        }
    }

    fn copy_selection(&mut self, id: Id) {
        let c = &self.ctl[id];
        if c.password_char.is_some() {
            return;
        }
        let (a, b) = c.selection();
        if a < b {
            let s: String = c.chars()[a..b].iter().collect();
            super::clipboard::set(&s.replace('\n', if cfg!(windows) { "\r\n" } else { "\n" }));
        }
    }

    fn paste(&mut self, id: Id, out: &mut Vec<Raw>) {
        if let Some(s) = super::clipboard::get() {
            let s = s.replace("\r\n", "\n").replace('\r', "\n");
            let s = if self.ctl[id].kind == Kind::Memo { s } else { s.lines().next().unwrap_or("").to_string() };
            if self.insert_text(id, &s) {
                self.text_changed(id, out);
            }
        }
    }
}

fn sb_part(r: Rect, vertical: bool, min: i64, max: i64, page: i64, pos: i64, x: i32, y: i32) -> i32 {
    let (a1, a2, _, thumb) = sb_geom(r, vertical, min, max, page, pos);
    if a1.contains(x, y) {
        PART_UP
    } else if a2.contains(x, y) {
        PART_DOWN
    } else if thumb.contains(x, y) {
        PART_THUMB
    } else if (vertical && y < thumb.y) || (!vertical && x < thumb.x) {
        PART_PAGE_UP
    } else {
        PART_PAGE_DOWN
    }
}

fn nearest(xs: &[i32], x: i32) -> usize {
    let mut best = 0;
    let mut bd = i32::MAX;
    for (i, &p) in xs.iter().enumerate() {
        let d = (p - x).abs();
        if d < bd {
            bd = d;
            best = i;
        }
    }
    best
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '-'
}

fn word_at(ch: &[char], p: usize) -> (usize, usize) {
    let p = p.min(ch.len());
    let mut a = p;
    while a > 0 && is_word(ch[a - 1]) {
        a -= 1;
    }
    let mut b = p;
    while b < ch.len() && is_word(ch[b]) {
        b += 1;
    }
    (a, b)
}

fn word_left(ch: &[char], p: usize) -> usize {
    let mut i = p.min(ch.len());
    while i > 0 && !is_word(ch[i - 1]) {
        i -= 1;
    }
    while i > 0 && is_word(ch[i - 1]) {
        i -= 1;
    }
    i
}

fn word_right(ch: &[char], p: usize) -> usize {
    let mut i = p;
    while i < ch.len() && is_word(ch[i]) {
        i += 1;
    }
    while i < ch.len() && !is_word(ch[i]) {
        i += 1;
    }
    i
}

fn thousands(v: i64) -> String {
    let s = v.abs().to_string();
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            o.push(',');
        }
        o.push(c);
    }
    if v < 0 {
        format!("-{o}")
    } else {
        o
    }
}

#[cfg(test)]
mod tests {
    use super::wheel_steps;

    #[test]
    fn wheel_notches_both_ways() {
        let mut r = 0;
        assert_eq!(wheel_steps(&mut r, 240), 2);
        assert_eq!(wheel_steps(&mut r, -240), -2);
        assert_eq!(wheel_steps(&mut r, -120), -1);
        assert_eq!(wheel_steps(&mut r, 120 * 30), 10);
        assert_eq!(wheel_steps(&mut r, -120 * 30), -10);
        assert_eq!(r, 0);
    }

    #[test]
    fn wheel_partial_deltas_add_up() {
        let mut r = 0;
        assert_eq!(wheel_steps(&mut r, 40), 0);
        assert_eq!(wheel_steps(&mut r, 40), 0);
        assert_eq!(wheel_steps(&mut r, 60), 1);
        assert_eq!(r, 20);
        // turning back drops the rest
        assert_eq!(wheel_steps(&mut r, -100), 0);
        assert_eq!(wheel_steps(&mut r, -30), -1);
        assert_eq!(r, -10);
    }
}
