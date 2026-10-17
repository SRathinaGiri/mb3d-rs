//! Painting of the forms: window frame, every control kind and the
//! overlays (drop-down lists, popup menus, hints).

use super::canvas::{line_height, strip_amp, Canvas, HAlign, Rect, VAlign};
use super::control::{Control, Id, Kind};
use super::font::{self, Font};
use super::form::*;
use super::theme::{BtnState, Dir, Theme};

/// Scroll bar width / height.
pub const SB: i32 = 17;

/// Geometry of a scroll bar: (arrow 1, arrow 2, track, thumb).
pub fn sb_geom(r: Rect, vertical: bool, min: i64, max: i64, page: i64, pos: i64) -> (Rect, Rect, Rect, Rect) {
    let (len, thick) = if vertical { (r.h, r.w) } else { (r.w, r.h) };
    let a = thick.min(len / 2);
    let track_len = (len - 2 * a).max(0);
    let range = (max - min).max(0) as f64;
    let page = page.max(0) as f64;
    let tl = if range + page > 0.0 { ((page / (range + page)) * track_len as f64).round() as i32 } else { track_len };
    let tl = tl.clamp(8.min(track_len), track_len);
    let span = (range - 0.0).max(1e-9);
    let off = if range > 0.0 { (((pos - min) as f64 / span).clamp(0.0, 1.0) * (track_len - tl) as f64).round() as i32 } else { 0 };
    if vertical {
        (
            Rect::new(r.x, r.y, r.w, a),
            Rect::new(r.x, r.bottom() - a, r.w, a),
            Rect::new(r.x, r.y + a, r.w, track_len),
            Rect::new(r.x, r.y + a + off, r.w, tl),
        )
    } else {
        (
            Rect::new(r.x, r.y, a, r.h),
            Rect::new(r.right() - a, r.y, a, r.h),
            Rect::new(r.x + a, r.y, track_len, r.h),
            Rect::new(r.x + a + off, r.y, tl, r.h),
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_scrollbar(cv: &mut Canvas, th: &Theme, r: Rect, vertical: bool, min: i64, max: i64, page: i64, pos: i64, hot: i32, pressed: i32, enabled: bool) {
    let (a1, a2, track, thumb) = sb_geom(r, vertical, min, max, page, pos);
    th.scroll_track(cv, r);
    let st = |part: i32| BtnState { hot: hot == part, pressed: pressed == part, enabled, ..Default::default() };
    th.small_button(cv, a1, Some(if vertical { Dir::Up } else { Dir::Left }), st(PART_UP));
    th.small_button(cv, a2, Some(if vertical { Dir::Down } else { Dir::Right }), st(PART_DOWN));
    if enabled && max > min && track.w > 0 && track.h > 0 {
        th.scroll_thumb(cv, thumb, st(PART_THUMB));
    }
}

/// Track bar geometry: (channel, thumb, first and last thumb centre).
pub fn track_geom(c: &Control) -> (Rect, Rect, i32, i32) {
    let ticks = c.tick_style != "tsNone";
    let both = c.tick_marks == "tmBoth";
    let top_ticks = c.tick_marks == "tmTopLeft";
    let (len, thick) = if c.vertical { (c.height, c.width) } else { (c.width, c.height) };
    let tl = c.thumb_length.min(thick - if ticks { 6 } else { 2 }).max(8);
    let tw = (tl / 2 + 1).clamp(7, 11);
    let x0 = 4 + tw / 2 + 4;
    let x1 = (len - 4 - tw / 2 - 5).max(x0);
    let range = (c.max - c.min).max(1) as f64;
    let p = x0 + (((c.position - c.min) as f64 / range).clamp(0.0, 1.0) * (x1 - x0) as f64).round() as i32;
    // thumb across: with ticks on one side it is shifted away from them
    let free = thick - tl;
    let t0 = if !ticks || both { free / 2 } else if top_ticks { free - free / 3 } else { free / 3 };
    let ch_c = t0 + tl / 2;
    if c.vertical {
        (
            Rect::new(ch_c - 2, x0 - tw / 2 - 2, 4, x1 - x0 + tw + 4),
            Rect::new(t0, p - tw / 2, tl, tw),
            x0,
            x1,
        )
    } else {
        (
            Rect::new(x0 - tw / 2 - 2, ch_c - 2, x1 - x0 + tw + 4, 4),
            Rect::new(p - tw / 2, t0, tw, tl),
            x0,
            x1,
        )
    }
}

/// Visual lines of a memo: (index of the first character, text).
pub fn memo_lines(f: &Form, id: Id) -> Vec<(usize, String)> {
    let c = &f.ctl[id];
    let font = f.font(id);
    let wrap = c.word_wrap && c.scroll_bars & 1 == 0;
    let width = c.width - 6 - if c.scroll_bars & 2 != 0 { SB } else { 0 };
    let chars: Vec<char> = c.text.chars().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i <= chars.len() {
        if i == chars.len() || chars[i] == '\n' {
            let line: String = chars[start..i].iter().collect();
            let line_t = line.trim_end_matches('\r').to_string();
            if wrap && f.measure(&font, &line_t) > width && width > 20 {
                // break at spaces
                let lc: Vec<char> = line_t.chars().collect();
                let mut s = 0;
                while s < lc.len() {
                    let mut e = s;
                    let mut last_space = None;
                    while e < lc.len() {
                        let w = f.measure(&font, &lc[s..=e].iter().collect::<String>());
                        if w > width && e > s {
                            break;
                        }
                        if lc[e] == ' ' {
                            last_space = Some(e);
                        }
                        e += 1;
                    }
                    if e < lc.len() {
                        if let Some(sp) = last_space {
                            e = sp + 1;
                        }
                    }
                    out.push((start + s, lc[s..e].iter().collect()));
                    s = e;
                }
                if lc.is_empty() {
                    out.push((start, String::new()));
                }
            } else {
                out.push((start, line_t));
            }
            start = i + 1;
        }
        i += 1;
    }
    out
}

/// Logical x positions of the caret before each character (n + 1 values).
pub fn char_xs(font: &Font, s: &str) -> Vec<i32> {
    let mut v = Vec::with_capacity(s.len() + 1);
    let mut x = 0f32;
    v.push(0);
    for ch in s.chars() {
        x += font::glyph(font.face(), font.em(), ch).advance;
        v.push(x.round() as i32);
    }
    v
}

/// Radio group item rectangles (VCL's ArrangeButtons), control coordinates.
pub fn radio_items(f: &Form, id: Id) -> Vec<Rect> {
    let c = &f.ctl[id];
    let n = c.items.len() as i32;
    if n == 0 {
        return Vec::new();
    }
    let cols = c.columns.max(1);
    let per_col = (n + cols - 1) / cols;
    let bw = (c.width - 10) / cols;
    let tmh = line_height(&f.font(id));
    let i = c.height - tmh - 5;
    let bh = i / per_col.max(1);
    let top = tmh + 1 + (i % per_col.max(1)) / 2;
    (0..n).map(|k| Rect::new((k / per_col) * bw + 8, (k % per_col) * bh + top, bw, bh)).collect()
}

/// Rectangle of the drop-down list of a combo box (form coordinates).  The
/// list is drawn inside the window: below the box, or above it when it
/// does not fit below; when it fits neither, it gets the larger side and
/// shows as many rows as fit (it scrolls).
pub fn combo_list_rect(f: &Form, id: Id) -> Rect {
    let c = &f.ctl[id];
    let r = f.abs_rect(id);
    let ih = line_height(&f.font(id)) + 2;
    let n = (c.items.len() as i32).clamp(1, c.drop_down_count.max(1));
    let (_, fh) = f.client_size();
    let (below, above) = (fh - r.bottom(), r.y);
    let fits = |room: i32| ((room - 2) / ih).clamp(1, n);
    let (rows, up) = if n * ih + 2 <= below {
        (n, false)
    } else if n * ih + 2 <= above {
        (n, true)
    } else if above > below {
        (fits(above), true)
    } else {
        (fits(below), false)
    };
    let h = rows * ih + 2;
    Rect::new(r.x, if up { r.y - h } else { r.bottom() }, r.w.max(40), h)
}

/// The rows the drop-down list of a combo box shows at once.
pub fn combo_rows(f: &Form, id: Id) -> i32 {
    (combo_list_rect(f, id).h - 2) / (line_height(&f.font(id)) + 2)
}

pub const MENU_ITEM_H: i32 = 22;
pub const MENU_SEP_H: i32 = 9;

/// Visible items of a menu (popup menu or menu item children).
pub fn menu_items(f: &Form, parent: Id) -> Vec<Id> {
    f.ctl[parent].children.iter().copied().filter(|&k| f.ctl[k].kind == Kind::MenuItem && f.ctl[k].visible).collect()
}

/// Size of a menu level.
pub fn menu_size(f: &Form, parent: Id) -> (i32, i32) {
    let font = f.font(0);
    let mut w = 80;
    let mut h = 4;
    for k in menu_items(f, parent) {
        let c = &f.ctl[k];
        if c.caption == "-" {
            h += MENU_SEP_H;
        } else {
            w = w.max(f.measure(&font, &strip_amp(&c.caption)) + 60);
            h += MENU_ITEM_H;
        }
    }
    (w, h)
}

/// Item rectangles of a menu level (form coordinates).
pub fn menu_item_rects(f: &Form, lv: &MenuLevel) -> Vec<(Id, Rect)> {
    let mut y = lv.rect.y + 2;
    menu_items(f, lv.parent)
        .into_iter()
        .map(|k| {
            let h = if f.ctl[k].caption == "-" { MENU_SEP_H } else { MENU_ITEM_H };
            let r = Rect::new(lv.rect.x + 2, y, lv.rect.w - 4, h);
            y += h;
            (k, r)
        })
        .collect()
}

/// Rectangles of the title bar buttons (window coordinates): close, max, min.
pub fn caption_buttons(f: &Form, th: &Theme, win_w: i32) -> Vec<(i32, Rect)> {
    if !f.has_caption() {
        return Vec::new();
    }
    let ch = f.caption_h(th);
    let bw = if th.glossy() { ch - 2 } else { 46 };
    let mut out = Vec::new();
    let mut x = win_w - f.frame_w() - bw - if th.glossy() { 3 } else { 0 };
    let y = f.frame_w() + if th.glossy() { 1 } else { 0 };
    let has = |s: &str| f.border_icons.iter().any(|b| b == s);
    if has("biSystemMenu") {
        out.push((0, Rect::new(x, y, bw, ch - if th.glossy() { 2 } else { 0 })));
        x -= bw;
        if !f.tool_window() && f.border_style != "bsDialog" {
            if has("biMaximize") && f.sizeable() {
                out.push((1, Rect::new(x, y, bw, ch - if th.glossy() { 2 } else { 0 })));
                x -= bw;
            }
            if has("biMinimize") {
                out.push((2, Rect::new(x, y, bw, ch - if th.glossy() { 2 } else { 0 })));
            }
        }
    }
    out
}

impl Form {
    /// Paints the whole window (frame, client area, overlays).
    pub fn paint(&self, cv: &mut Canvas, th: &Theme, win_w: i32, win_h: i32) {
        let b = self.frame_w();
        let ch = self.caption_h(th);
        if self.has_caption() {
            let active = self.active;
            cv.fill(Rect::new(0, 0, win_w, win_h), th.window_border(active));
            th.caption(cv, Rect::new(b, b, win_w - 2 * b, ch), active);
            let mut tf = Font { bold: th.glossy(), ..Font::default() };
            tf.height = if self.tool_window() { -11 } else { -12 };
            let mut tx = b + 6;
            if !self.tool_window() {
                draw_app_icon(cv, tx, b + (ch - 16) / 2);
                tx += 22;
            }
            let lh = line_height(&tf);
            let buttons = caption_buttons(self, th, win_w);
            let right = buttons.iter().map(|(_, r)| r.x).min().unwrap_or(win_w);
            let old = cv.clip_to(Rect::new(tx, b, right - tx - 4, ch));
            cv.text(tx, b + (ch - lh) / 2, self.caption(), &tf, th.caption_text(active));
            if let Some(c) = self.caption_led {
                let lx = tx + cv.text_width(&tf, self.caption()) + 10;
                let r = Rect::new(lx, b + (ch - 9) / 2, 18, 9);
                draw_led(cv, r, c, self.led_on);
                self.led_rect.set(r);
            }
            cv.set_clip(old);
            for (k, r) in buttons {
                th.caption_button(cv, r, k as u8, self.nc_hot == k, self.nc_pressed == k);
            }
        }
        cv.translate(b, b + ch);
        let (cw, chh) = self.client_size();
        let old = cv.clip_to(Rect::new(0, 0, cw, chh));
        self.paint_ctl(cv, th, 0);
        // overlays
        match &self.popup {
            Some(Popup::Combo { id, hot, top }) => self.paint_combo_list(cv, th, *id, *hot, *top),
            Some(Popup::Menu { levels, .. }) => {
                for lv in levels {
                    self.paint_menu(cv, th, lv);
                }
            }
            None => {}
        }
        if let Some((text, x, y)) = &self.hint {
            self.paint_hint(cv, th, text, *x, *y);
        }
        cv.set_clip(old);
        cv.translate(-b, -b - ch);
    }

    fn paint_ctl(&self, cv: &mut Canvas, th: &Theme, id: Id) {
        let r = self.abs_rect(id);
        let old = cv.clip_to(r);
        if !cv.clip_empty() {
            self.paint_self(cv, th, id, r);
        }
        cv.set_clip(old);
        let c = &self.ctl[id];
        if c.kind.container() || c.kind == Kind::ScrollBox {
            let clip = self.clip_rect(id);
            let old = cv.clip_to(clip);
            if !cv.clip_empty() {
                for k in self.paint_order(id) {
                    self.paint_ctl(cv, th, k);
                }
            }
            cv.set_clip(old);
            if c.kind == Kind::ScrollBox {
                self.paint_scrollbox_bars(cv, th, id, r);
            }
        }
    }

    fn paint_self(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect) {
        let c = &self.ctl[id];
        let en = self.enabled(id);
        let font = self.font(id);
        let tc = if en { self.text_color(id, th) } else { th.text_disabled() };
        let focused = self.focus == Some(id) && self.active;
        let hot = self.hot == Some(id);
        let pressed = self.capture == Some(id) && self.hot == Some(id) && self.cap_button == Some(MouseButton::Left);
        match c.kind {
            Kind::Form | Kind::TabSheet | Kind::GridPanel => {
                let bg = self.bg(id, th);
                if c.kind == Kind::TabSheet && !th.glossy() && c.color.is_none() {
                    cv.fill(r, th.v("tab.body.bg"));
                } else {
                    th.fill_face(cv, r, bg);
                }
                if c.kind == Kind::GridPanel {
                    self.paint_panel_frame(cv, th, c, r);
                }
            }
            Kind::Panel => {
                let bg = self.bg(id, th);
                th.fill_face(cv, r, bg);
                self.paint_panel_frame(cv, th, c, r);
                if !c.caption.is_empty() {
                    let ar = r.inset(c.bevel_width.max(1) + c.border_width);
                    cv.text_in(ar, &strip_amp(&c.caption), &font, tc, c.alignment, c.layout, c.word_wrap);
                }
            }
            Kind::Label | Kind::StaticText => {
                if let Some(col) = c.color {
                    if !c.transparent {
                        cv.fill(r, col);
                    }
                }
                if c.kind == Kind::StaticText && c.border_style != "sbsNone" && c.border_style != "bsNone" {
                    th.bevel(cv, r, c.border_style == "sbsSingle");
                }
                let text = if c.props.iter().any(|(k, v)| k == "ShowAccelChar" && v.as_bool() == Some(false)) {
                    c.caption.clone()
                } else {
                    strip_amp(&c.caption)
                };
                cv.text_in(r, &text, &font, tc, c.alignment, c.layout, c.word_wrap);
            }
            Kind::Button => {
                let st = BtnState { hot, pressed, enabled: en, focused, default: c.default, ..Default::default() };
                th.button(cv, r, st);
                let off = if pressed && !th.glossy() { 1 } else { 0 };
                cv.text_in(r.inset(2).offset(off, off), &strip_amp(&c.caption), &font, th.button_text(&st), HAlign::Center, VAlign::Center, c.word_wrap);
            }
            Kind::SpeedButton => self.paint_speed_button(cv, th, id, r, en, hot, pressed),
            Kind::ColorButton => {
                let st = BtnState { hot, pressed, enabled: en, ..Default::default() };
                th.button(cv, r, st);
                let sw = Rect::new(r.x + 4, r.y + 4, r.w - 18, r.h - 8);
                cv.fill(sw, c.brush_color);
                cv.frame(sw, th.shadow());
                th.arrow(cv, Rect::new(r.right() - 14, r.y, 12, r.h), Dir::Down, th.button_text(&st));
            }
            Kind::Edit => self.paint_edit(cv, th, id, r, en, focused, &font, tc),
            Kind::Memo => self.paint_memo(cv, th, id, r, en, focused, &font, tc),
            Kind::CheckBox | Kind::RadioButton => {
                let box_right = c.prop("Alignment").and_then(|v| v.as_str()) == Some("taLeftJustify");
                let bs = 13;
                let by = r.y + (r.h - bs) / 2;
                let bx = if box_right { r.right() - bs - 1 } else { r.x };
                let br = Rect::new(bx, by, bs, bs);
                if c.kind == Kind::CheckBox {
                    th.checkbox(cv, br, c.state, en, hot);
                } else {
                    th.radio(cv, br, c.checked, en, hot);
                }
                let tr = if box_right { Rect::new(r.x, r.y, r.w - bs - 5, r.h) } else { Rect::new(r.x + bs + 4, r.y, r.w - bs - 4, r.h) };
                let old = cv.clip_to(tr);
                cv.text_in(tr, &strip_amp(&c.caption), &font, tc, if box_right { HAlign::Left } else { HAlign::Left }, VAlign::Center, c.word_wrap);
                cv.set_clip(old);
            }
            Kind::GroupBox | Kind::RadioGroup => {
                let bg = self.bg(id, th);
                th.fill_face(cv, r, bg);
                let lh = line_height(&font);
                let cap = strip_amp(&c.caption);
                let tw = if cap.is_empty() { 0 } else { self.measure(&font, &cap) + 4 };
                let fr = Rect::new(r.x, r.y + lh / 2, r.w, r.h - lh / 2);
                th.groupbox(cv, fr, if tw > 0 { Some((r.x + 6, tw)) } else { None });
                if tw > 0 {
                    cv.text(r.x + 8, r.y, &cap, &font, tc);
                }
                if c.kind == Kind::RadioGroup {
                    for (i, ir) in radio_items(self, id).iter().enumerate() {
                        let ir = ir.offset(r.x, r.y);
                        let bs = 13;
                        let hot_i = hot && self.hot_part == i as i32;
                        th.radio(cv, Rect::new(ir.x, ir.y + (ir.h - bs) / 2, bs, bs), c.item_index == i as i32, en, hot_i);
                        let old = cv.clip_to(ir);
                        cv.text_in(Rect::new(ir.x + bs + 4, ir.y, ir.w - bs - 4, ir.h), &strip_amp(&c.items[i]), &font, tc, HAlign::Left, VAlign::Center, false);
                        cv.set_clip(old);
                    }
                }
            }
            Kind::ComboBox => {
                let inner = th.edit_frame(cv, r, en, focused, c.color);
                let br = Rect::new(r.right() - 18, r.y + 1, 17, r.h - 2);
                let open = matches!(self.popup, Some(Popup::Combo { id: pid, .. }) if pid == id);
                th.small_button(cv, br, Some(Dir::Down), BtnState { hot: hot && self.hot_part == PART_BUTTON, pressed: open, enabled: en, ..Default::default() });
                let tr = Rect::new(inner.x + 1, inner.y, br.x - inner.x - 2, inner.h);
                if c.style == "csDropDownList" || c.style == "csOwnerDrawFixed" {
                    let text = c.item_text();
                    let col = c.item_colors.get(c.item_index.max(0) as usize).copied().flatten().unwrap_or(tc);
                    if focused && !open {
                        th.selection(cv, tr, true);
                        cv.text_in(tr.offset(2, 0), &text, &font, th.selection_text(true), HAlign::Left, VAlign::Center, false);
                    } else {
                        let old = cv.clip_to(tr);
                        cv.text_in(tr.offset(2, 0), &text, &font, col, HAlign::Left, VAlign::Center, false);
                        cv.set_clip(old);
                    }
                } else {
                    self.paint_edit_text(cv, th, id, tr, focused, &font, tc);
                }
            }
            Kind::ListBox => self.paint_listbox(cv, th, id, r, en, focused, &font, tc),
            Kind::TrackBar => {
                let (chn, thumb, x0, x1) = track_geom(c);
                let chn = chn.offset(r.x, r.y);
                th.track_channel(cv, chn);
                if c.sel_end > c.sel_start {
                    let range = (c.max - c.min).max(1) as f64;
                    let p = |v: i64| x0 + (((v - c.min) as f64 / range).clamp(0.0, 1.0) * (x1 - x0) as f64).round() as i32;
                    let (a, b) = (p(c.sel_start), p(c.sel_end));
                    let sr = if c.vertical { Rect::new(chn.x + 1, r.y + a, chn.w - 2, b - a) } else { Rect::new(r.x + a, chn.y + 1, b - a, chn.h - 2) };
                    cv.fill(sr, th.highlight());
                }
                // ticks
                if c.tick_style != "tsNone" {
                    let range = (c.max - c.min).max(1);
                    let mut vals: Vec<i64> = vec![c.min, c.max];
                    if c.tick_style == "tsAuto" && c.frequency > 0 && range / c.frequency <= 200 {
                        let mut v = c.min;
                        while v <= c.max {
                            vals.push(v);
                            v += c.frequency;
                        }
                    }
                    let tr = thumb.offset(r.x, r.y);
                    for v in vals {
                        let p = x0 + (((v - c.min) as f64 / range as f64) * (x1 - x0) as f64).round() as i32;
                        let sides: &[bool] = match c.tick_marks.as_str() {
                            "tmBoth" => &[true, false],
                            "tmTopLeft" => &[true],
                            _ => &[false],
                        };
                        for &before in sides {
                            if c.vertical {
                                let x = if before { tr.x - 5 } else { tr.right() + 2 };
                                cv.hline(x, x + 3, r.y + p, th.tick());
                            } else {
                                let y = if before { tr.y - 5 } else { tr.bottom() + 2 };
                                cv.vline(r.x + p, y, y + 3, th.tick());
                            }
                        }
                    }
                }
                if c.slider_visible {
                    let st = BtnState { hot: hot && self.hot_part == PART_THUMB, pressed: self.capture == Some(id) && self.cap_part == PART_THUMB, enabled: en, ..Default::default() };
                    th.track_thumb(cv, thumb.offset(r.x, r.y), st);
                }
                if focused && !th.glossy() {
                    // the native control shows no focus rectangle in MB3D's style
                }
            }
            Kind::UpDown => {
                let (a, b) = updown_rects(c, r);
                let st = |part: i32| BtnState {
                    hot: hot && self.hot_part == part,
                    pressed: self.capture == Some(id) && self.cap_part == part && self.hot_part == part,
                    enabled: en,
                    ..Default::default()
                };
                let (d1, d2) = if c.vertical { (Dir::Left, Dir::Right) } else { (Dir::Up, Dir::Down) };
                th.small_button(cv, a, Some(d1), st(PART_UP));
                th.small_button(cv, b, Some(d2), st(PART_DOWN));
            }
            Kind::ProgressBar => {
                let range = (c.max - c.min).max(1) as f32;
                th.progress(cv, r, (c.position - c.min) as f32 / range, c.vertical);
            }
            Kind::ScrollBar => {
                let hp = if hot { self.hot_part } else { PART_NONE };
                let pp = if self.capture == Some(id) { self.cap_part } else { PART_NONE };
                paint_scrollbar(cv, th, r, c.vertical, c.min, c.max - c.page_size.max(1) + 1, c.page_size.max(1), c.position, hp, pp, en);
            }
            Kind::PageControl | Kind::TabControl => self.paint_tabs(cv, th, id, r, &font),
            Kind::ScrollBox => {
                let bg = c.color.unwrap_or(if th.glossy() { th.image_bg() } else { th.face() });
                cv.fill(r, bg);
                if c.border {
                    th.bevel(cv, r, false);
                }
            }
            Kind::Image => {
                if let Some(p) = &c.picture {
                    let (pw, ph) = (p.w as i32, p.h as i32);
                    if c.device_pixels {
                        let (x0, y0) = (cv.dx(r.x), cv.dy(r.y));
                        cv.image_dev(x0, y0, pw, ph, p, false);
                    } else if c.stretch || (c.proportional && (pw > r.w || ph > r.h)) {
                        let dr = if c.proportional {
                            let s = (r.w as f32 / pw as f32).min(r.h as f32 / ph as f32);
                            let (w, h) = ((pw as f32 * s) as i32, (ph as f32 * s) as i32);
                            if c.center {
                                Rect::new(r.x + (r.w - w) / 2, r.y + (r.h - h) / 2, w, h)
                            } else {
                                Rect::new(r.x, r.y, w, h)
                            }
                        } else {
                            r
                        };
                        cv.image_stretch(dr, p, true);
                    } else if c.center {
                        cv.image(r.x + (r.w - pw) / 2, r.y + (r.h - ph) / 2, p);
                    } else {
                        cv.image(r.x, r.y, p);
                    }
                }
            }
            Kind::Shape => {
                let pw = c.pen_width;
                let fill = if c.brush_clear { None } else { Some(c.brush_color) };
                let pen = if pw > 0 { Some(c.pen_color) } else { None };
                let sr = match c.shape.as_str() {
                    "stSquare" | "stCircle" | "stRoundSquare" => {
                        let s = r.w.min(r.h);
                        Rect::new(r.x + (r.w - s) / 2, r.y + (r.h - s) / 2, s, s)
                    }
                    _ => r,
                };
                match c.shape.as_str() {
                    "stCircle" | "stEllipse" => cv.ellipse(sr, fill, pen),
                    "stRoundRect" | "stRoundSquare" => {
                        let rad = sr.w.min(sr.h) as f32 / 4.0;
                        if let Some(f) = fill {
                            cv.round_rect(sr, rad, f, f, pen);
                        } else if let Some(p) = pen {
                            cv.round_rect(sr, rad, 0, 0, Some(p));
                        }
                    }
                    _ => {
                        if let Some(f) = fill {
                            cv.fill(sr, f);
                        }
                        if let Some(p) = pen {
                            for i in 0..pw.max(1) {
                                cv.frame(sr.inset(i), p);
                            }
                        }
                    }
                }
            }
            Kind::Bevel => {
                let raised = c.bevel_style == "bsRaised";
                let (l, d) = (th.v("bevel.light"), th.v("bevel.dark"));
                let (a, b) = if raised { (l, d) } else { (d, l) };
                match c.bevel_shape.as_str() {
                    "bsBox" => th.bevel(cv, r, raised),
                    "bsFrame" => {
                        th.bevel(cv, r, raised);
                        th.bevel(cv, r.inset(1), !raised);
                    }
                    "bsTopLine" => {
                        cv.hline(r.x, r.right(), r.y, a);
                        cv.hline(r.x, r.right(), r.y + 1, b);
                    }
                    "bsBottomLine" => {
                        cv.hline(r.x, r.right(), r.bottom() - 2, a);
                        cv.hline(r.x, r.right(), r.bottom() - 1, b);
                    }
                    "bsLeftLine" => {
                        cv.vline(r.x, r.y, r.bottom(), a);
                        cv.vline(r.x + 1, r.y, r.bottom(), b);
                    }
                    "bsRightLine" => {
                        cv.vline(r.right() - 2, r.y, r.bottom(), a);
                        cv.vline(r.right() - 1, r.y, r.bottom(), b);
                    }
                    _ => {}
                }
            }
            Kind::StringGrid => self.paint_grid(cv, th, id, r, en, focused, &font, tc),
            Kind::ListView => self.paint_listview(cv, th, id, r, en, focused, &font, tc),
            Kind::CategoryPanelGroup => {
                th.fill_face(cv, r, self.bg(id, th));
            }
            Kind::CategoryPanel => {
                let hh = super::form::category_header_h(self, id);
                th.fill_face(cv, r, self.bg(id, th));
                let hr = Rect::new(r.x, r.y, r.w, hh);
                th.button(cv, hr, BtnState { hot: hot && self.hot_part == PART_HEADER, enabled: true, ..Default::default() });
                // chevrons
                let cx = hr.x + 10;
                let cy = hr.y + hh / 2;
                let col = th.text();
                for k in 0..2 {
                    let o = k * 4;
                    if c.collapsed {
                        cv.line_aa((cx - 3) as f32, (cy - 4 + o) as f32 - 2.0, cx as f32, (cy - 1 + o) as f32 - 2.0, 1.2, col);
                        cv.line_aa(cx as f32, (cy - 1 + o) as f32 - 2.0, (cx + 3) as f32, (cy - 4 + o) as f32 - 2.0, 1.2, col);
                    } else {
                        cv.line_aa((cx - 3) as f32, (cy + 1 + o) as f32 - 2.0, cx as f32, (cy - 2 + o) as f32 - 2.0, 1.2, col);
                        cv.line_aa(cx as f32, (cy - 2 + o) as f32 - 2.0, (cx + 3) as f32, (cy + 1 + o) as f32 - 2.0, 1.2, col);
                    }
                }
                cv.text_in(hr, &strip_amp(&c.caption), &font, tc, HAlign::Center, VAlign::Center, false);
            }
            _ => {}
        }
    }

    fn paint_panel_frame(&self, cv: &mut Canvas, th: &Theme, c: &Control, r: Rect) {
        let mut rr = r;
        let bw = c.bevel_width.max(1);
        if c.bevel_outer != "bvNone" {
            for _ in 0..bw {
                th.bevel(cv, rr, c.bevel_outer == "bvRaised");
                rr = rr.inset(1);
            }
        }
        rr = rr.inset(c.border_width);
        if c.bevel_inner != "bvNone" {
            for _ in 0..bw {
                th.bevel(cv, rr, c.bevel_inner == "bvRaised");
                rr = rr.inset(1);
            }
        }
        if c.border_style == "bsSingle" {
            cv.frame(r, th.shadow());
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_speed_button(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, en: bool, hot: bool, pressed: bool) {
        let c = &self.ctl[id];
        let st = BtnState { hot, pressed, down: c.down, enabled: en, flat: c.flat, ..Default::default() };
        // a flat button with Transparent = False hides what lies under it
        if c.flat && !c.transparent {
            cv.fill(r, self.bg(id, th));
        }
        th.button(cv, r, st);
        let font = self.font(id);
        let tc = if en { th.button_text(&st) } else { th.text_disabled() };
        let glyph = c.glyph_image(if !en { 1 } else if c.down || pressed { if c.num_glyphs > 2 { 2 } else { 0 } } else { 0 });
        let glyph = match (glyph, en, c.num_glyphs) {
            (Some(g), false, 1) => Some(g.disabled(th.v("glyph.disabled.shade"), th.text_disabled())),
            (g, _, _) => g,
        };
        let cap = strip_amp(&c.caption);
        let lh = line_height(&font);
        let tw = if cap.is_empty() { 0 } else { self.measure(&font, &cap) };
        let off = if (pressed || c.down) && !th.glossy() { 1 } else { 0 };
        let (gw, gh) = glyph.as_ref().map(|g| (g.w as i32, g.h as i32)).unwrap_or((0, 0));
        let sp = if gw > 0 && tw > 0 { c.spacing.max(0) } else { 0 };
        let layout = c.style.as_str();
        let vertical = layout == "blGlyphTop" || layout == "blGlyphBottom";
        if vertical {
            let total = gh + sp + if tw > 0 { lh } else { 0 };
            let mut y = r.y + (r.h - total) / 2 + off;
            if layout == "blGlyphBottom" {
                if tw > 0 {
                    cv.text(r.x + (r.w - tw) / 2 + off, y, &cap, &font, tc);
                    y += lh + sp;
                }
                if let Some(g) = &glyph {
                    cv.image(r.x + (r.w - gw) / 2 + off, y, g);
                }
            } else {
                if let Some(g) = &glyph {
                    cv.image(r.x + (r.w - gw) / 2 + off, y, g);
                    y += gh + sp;
                }
                if tw > 0 {
                    cv.text(r.x + (r.w - tw) / 2 + off, y, &cap, &font, tc);
                }
            }
        } else {
            let total = gw + sp + tw;
            let mut x = if c.margin >= 0 { r.x + c.margin } else { r.x + (r.w - total) / 2 };
            x += off;
            let gy = r.y + (r.h - gh) / 2 + off;
            let ty = r.y + (r.h - lh) / 2 + off;
            let old = cv.clip_to(r.inset(1));
            if layout == "blGlyphRight" {
                // VCL (TButtonGlyph.CalcButtonLayout): the margin is measured
                // from the edge the glyph is on; a negative spacing centres
                // the caption in the space left on the other side
                let gx = if c.margin >= 0 { r.x + r.w - c.margin - gw + off } else { x + tw + sp };
                if let Some(g) = &glyph {
                    cv.image(gx, gy, g);
                }
                if tw > 0 {
                    let tx = if c.spacing < 0 && gw > 0 { r.x + (gx - r.x - tw) / 2 + off } else if gw > 0 { gx - sp - tw } else { x };
                    cv.text(tx, ty, &cap, &font, tc);
                }
            } else if c.margin >= 0 && c.spacing < 0 && gw > 0 && tw > 0 {
                // glyph on the left at the margin, the caption centred beside it
                if let Some(g) = &glyph {
                    cv.image(x, gy, g);
                }
                let gr = x + gw;
                cv.text(gr + (r.x + r.w - gr - tw) / 2, ty, &cap, &font, tc);
            } else {
                if let Some(g) = &glyph {
                    cv.image(x, gy, g);
                    x += gw + sp;
                }
                if tw > 0 {
                    if c.word_wrap && tw > r.w - 4 {
                        cv.text_in(r.inset(2), &cap, &font, tc, HAlign::Center, VAlign::Center, true);
                    } else {
                        cv.text(x, ty, &cap, &font, tc);
                    }
                }
            }
            cv.set_clip(old);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_edit(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, en: bool, focused: bool, font: &Font, tc: u32) {
        let c = &self.ctl[id];
        let inner = if c.border {
            th.edit_frame(cv, r, en, focused, c.color)
        } else {
            cv.fill(r, c.color.unwrap_or(th.window()));
            r
        };
        let tc = if c.read_only && !c.font.as_ref().map(|f| f.custom_color).unwrap_or(false) { tc } else { tc };
        self.paint_edit_text(cv, th, id, Rect::new(inner.x + 1, inner.y, inner.w - 2, inner.h), focused, font, tc);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn paint_edit_text(&self, cv: &mut Canvas, th: &Theme, id: Id, tr: Rect, focused: bool, font: &Font, tc: u32) {
        let c = &self.ctl[id];
        let shown: String = match c.password_char {
            Some(p) => c.text.chars().map(|_| p).collect(),
            None => c.text.clone(),
        };
        let xs = char_xs(font, &shown);
        let lh = line_height(font);
        let ty = tr.y + (tr.h - lh) / 2;
        let total = *xs.last().unwrap_or(&0);
        let base_x = match c.alignment {
            HAlign::Right if c.kind == Kind::Edit && total < tr.w => tr.right() - total - 1,
            HAlign::Center if c.kind == Kind::Edit && total < tr.w => tr.x + (tr.w - total) / 2,
            _ => tr.x - c.scroll_x,
        };
        let old = cv.clip_to(tr);
        let (s0, s1) = c.selection();
        if focused && s1 > s0 {
            let a = base_x + xs[s0.min(xs.len() - 1)];
            let b = base_x + xs[s1.min(xs.len() - 1)];
            th.selection(cv, Rect::new(a, ty, b - a, lh), true);
            cv.text(base_x, ty, &shown, font, tc);
            // selected part in the selection colour
            let o2 = cv.clip_to(Rect::new(a, ty, b - a, lh));
            cv.text(base_x, ty, &shown, font, th.selection_text(true));
            cv.set_clip(o2);
        } else {
            cv.text(base_x, ty, &shown, font, tc);
        }
        if focused && self.caret_on && !(c.kind == Kind::ComboBox && c.style == "csDropDownList") {
            let x = base_x + xs[c.caret.min(xs.len() - 1)];
            cv.vline(x, ty, ty + lh, tc);
        }
        cv.set_clip(old);
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_memo(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, en: bool, focused: bool, font: &Font, tc: u32) {
        let c = &self.ctl[id];
        let inner = if c.border {
            th.edit_frame(cv, r, en, focused, c.color)
        } else {
            cv.fill(r, c.color.unwrap_or(th.window()));
            r
        };
        let vs = c.scroll_bars & 2 != 0;
        let hs = c.scroll_bars & 1 != 0;
        let tr = Rect::new(inner.x + 2, inner.y + 1, inner.w - 3 - if vs { SB } else { 0 }, inner.h - 1 - if hs { SB } else { 0 });
        let lines = memo_lines(self, id);
        let lh = line_height(font);
        let first = c.scroll_y.max(0) as usize;
        let (s0, s1) = c.selection();
        let old = cv.clip_to(tr);
        let mut y = tr.y;
        for (li, (start, text)) in lines.iter().enumerate().skip(first) {
            if y > tr.bottom() {
                break;
            }
            let x0 = tr.x - c.scroll_x;
            let n = text.chars().count();
            let xs = char_xs(font, text);
            if focused && s1 > s0 && s0 <= start + n && s1 >= *start {
                let a = s0.max(*start) - start;
                let b = (s1.min(start + n + 1) - start).min(n);
                let ax = x0 + xs[a.min(n)];
                let bx = x0 + xs[b.min(n)] + if s1 > start + n { 4 } else { 0 };
                th.selection(cv, Rect::new(ax, y, bx - ax, lh), true);
                cv.text(x0, y, text, font, tc);
                let o2 = cv.clip_to(Rect::new(ax, y, bx - ax, lh));
                cv.text(x0, y, text, font, th.selection_text(true));
                cv.set_clip(o2);
            } else {
                cv.text(x0, y, text, font, tc);
            }
            let next_start = lines.get(li + 1).map(|l| l.0).unwrap_or(usize::MAX);
            if focused && self.caret_on && c.caret >= *start && (c.caret < next_start || li + 1 == lines.len()) && c.caret <= start + n {
                let x = x0 + xs[(c.caret - start).min(n)];
                cv.vline(x, y, y + lh, tc);
            }
            y += lh;
        }
        cv.set_clip(old);
        if vs {
            let vis = (tr.h / lh.max(1)) as i64;
            let sr = Rect::new(inner.right() - SB, inner.y, SB, inner.h - if hs { SB } else { 0 });
            let max = (lines.len() as i64 - vis).max(0);
            let hp = if self.hot == Some(id) && self.hot_part <= PART_UP { self.hot_part } else { PART_NONE };
            paint_scrollbar(cv, th, sr, true, 0, max, vis, c.scroll_y as i64, hp, if self.capture == Some(id) { self.cap_part } else { PART_NONE }, en && max > 0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_listbox(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, en: bool, focused: bool, font: &Font, tc: u32) {
        let c = &self.ctl[id];
        let inner = if c.border { th.edit_frame(cv, r, en, focused, c.color) } else {
            cv.fill(r, c.color.unwrap_or(th.window()));
            r
        };
        let ih = c.item_height.max(8);
        let vis = (inner.h / ih).max(1);
        let need_sb = c.items.len() as i32 > vis;
        let lr = Rect::new(inner.x, inner.y, inner.w - if need_sb { SB } else { 0 }, inner.h);
        let old = cv.clip_to(lr);
        for (i, it) in c.items.iter().enumerate().skip(c.top_index.max(0) as usize) {
            let y = lr.y + (i as i32 - c.top_index) * ih;
            if y > lr.bottom() {
                break;
            }
            let ir = Rect::new(lr.x, y, lr.w, ih);
            let sel = c.item_index == i as i32 || (c.multi_select && c.selected.get(i).copied().unwrap_or(false));
            let col = c.item_colors.get(i).copied().flatten().unwrap_or(tc);
            if sel {
                th.selection(cv, ir, focused);
                cv.text_in(ir.offset(2, 0), it, font, th.selection_text(focused), HAlign::Left, VAlign::Center, false);
            } else {
                cv.text_in(ir.offset(2, 0), it, font, col, HAlign::Left, VAlign::Center, false);
            }
        }
        cv.set_clip(old);
        if need_sb {
            let sr = Rect::new(inner.right() - SB, inner.y, SB, inner.h);
            let max = c.items.len() as i64 - vis as i64;
            let hp = if self.hot == Some(id) { self.hot_part } else { PART_NONE };
            paint_scrollbar(cv, th, sr, true, 0, max, vis as i64, c.top_index as i64, hp, if self.capture == Some(id) { self.cap_part } else { PART_NONE }, en);
        }
    }

    fn paint_tabs(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, font: &Font) {
        let c = &self.ctl[id];
        let buttons = c.style == "tsButtons" || c.style == "tsFlatButtons";
        let rects = self.tab_rects(id);
        let bottom = rects.iter().map(|(_, rr)| rr.bottom()).max().unwrap_or(0);
        th.fill_face(cv, r, self.bg(id, th));
        if !buttons {
            th.tab_body(cv, Rect::new(r.x, r.y + bottom, r.w, r.h - bottom));
        }
        let sel_key = if c.kind == Kind::PageControl { c.active_page.unwrap_or(usize::MAX) } else { c.tab_index.max(-1) as usize };
        for (i, (key, tr)) in rects.iter().enumerate() {
            let sel = *key == sel_key;
            let hot = self.hot == Some(id) && self.hot_part == i as i32 && self.enabled(id);
            let mut tr = tr.offset(r.x, r.y);
            if sel && !buttons {
                tr = Rect::new(tr.x - 2, tr.y - 2, tr.w + 4, tr.h + 3);
            }
            th.tab(cv, tr, sel, hot, buttons);
            let text = if c.kind == Kind::PageControl { strip_amp(&self.ctl[*key].caption) } else { strip_amp(&c.tabs[*key]) };
            let col = if !self.enabled(id) { th.text_disabled() } else if buttons && (sel || hot) && th.glossy() { th.v("button.active.text") } else { th.tab_text(sel) };
            cv.text_in(tr, &text, font, col, HAlign::Center, VAlign::Center, false);
        }
    }

    fn paint_scrollbox_bars(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect) {
        let c = &self.ctl[id];
        let (vs, hs) = scrollbox_bars(self, id);
        let b = if c.border { 2 } else { 0 };
        let (ex, ey) = scrollbox_extent(self, id);
        let cw = r.w - 2 * b - if vs { SB } else { 0 };
        let ch = r.h - 2 * b - if hs { SB } else { 0 };
        let hp = if self.hot == Some(id) { self.hot_part } else { PART_NONE };
        let pp = if self.capture == Some(id) { self.cap_part } else { PART_NONE };
        if vs {
            let sr = Rect::new(r.right() - b - SB, r.y + b, SB, ch);
            let (hp, pp) = if self.cap_value == 1 || self.hot_part_v() { (hp, pp) } else { (PART_NONE, PART_NONE) };
            paint_scrollbar(cv, th, sr, true, 0, (ey - ch).max(0) as i64, ch as i64, c.scroll_y as i64, hp, pp, true);
        }
        if hs {
            let sr = Rect::new(r.x + b, r.bottom() - b - SB, cw, SB);
            paint_scrollbar(cv, th, sr, false, 0, (ex - cw).max(0) as i64, cw as i64, c.scroll_x as i64, PART_NONE, PART_NONE, true);
        }
        if vs && hs {
            cv.fill(Rect::new(r.right() - b - SB, r.bottom() - b - SB, SB, SB), th.face());
        }
    }

    fn hot_part_v(&self) -> bool {
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_grid(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, en: bool, focused: bool, font: &Font, tc: u32) {
        let c = &self.ctl[id];
        let inner = if c.border { th.edit_frame(cv, r, en, focused, c.color) } else { r };
        let rh = c.default_row_height.max(10);
        let old = cv.clip_to(inner);
        let line = th.v("grid.line");
        let mut y = inner.y;
        for (ri, row) in c.cells.iter().enumerate() {
            let rr = ri as i32;
            if rr >= c.fixed_rows && rr < c.fixed_rows + c.scroll_y {
                continue;
            }
            let mut x = inner.x;
            for (ci, col) in c.cols.iter().enumerate() {
                let cr = Rect::new(x, y, col.width, rh);
                let fixed = rr < c.fixed_rows || (ci as i32) < c.fixed_cols;
                if fixed {
                    th.fill_face(cv, cr, th.face());
                    th.bevel(cv, cr, true);
                } else if rr == c.row && ci as i32 == c.col && (focused || th.glossy()) {
                    th.selection(cv, cr, focused);
                }
                let txt = row.get(ci).map(String::as_str).unwrap_or("");
                let col_t = if !fixed && rr == c.row && ci as i32 == c.col { th.selection_text(focused) } else { tc };
                let o2 = cv.clip_to(cr.inset(1));
                cv.text_in(Rect::new(cr.x + 3, cr.y, cr.w - 4, cr.h), txt, font, col_t, HAlign::Left, VAlign::Center, false);
                cv.set_clip(o2);
                cv.hline(cr.x, cr.right(), cr.bottom() - 1, line);
                cv.vline(cr.right() - 1, cr.y, cr.bottom(), line);
                x += col.width;
            }
            y += rh;
            if y > inner.bottom() {
                break;
            }
        }
        cv.set_clip(old);
    }

    #[allow(clippy::too_many_arguments)]
    /// A list view's rows: (header height, row height, rows shown, the
    /// largest first row, the first row shown).
    pub(crate) fn listview_rows(&self, id: Id) -> (i32, i32, i32, i32, i32) {
        let font = self.font(id);
        let hh = line_height(&font) + 8;
        let rh = line_height(&font) + 6;
        let r = self.abs_rect(id);
        let vis = ((r.h - 4 - hh) / rh).max(1);
        let c = &self.ctl[id];
        let max = (c.cells.len() as i32 - vis).max(0);
        (hh, rh, vis, max, c.scroll_y.clamp(0, max))
    }

    fn paint_listview(&self, cv: &mut Canvas, th: &Theme, id: Id, r: Rect, en: bool, focused: bool, font: &Font, tc: u32) {
        let c = &self.ctl[id];
        let inner = th.edit_frame(cv, r, en, focused, c.color);
        let (hh, rh, vis, max, top) = self.listview_rows(id);
        let need_sb = c.cells.len() as i32 > vis;
        let lw = inner.w - if need_sb { SB } else { 0 };
        let old = cv.clip_to(inner);
        let mut x = inner.x;
        let widths = c.column_widths(lw);
        for (i, col) in c.cols.iter().enumerate() {
            let w = widths[i];
            let hr = Rect::new(x, inner.y, w, hh);
            th.button(cv, hr, BtnState { enabled: true, ..Default::default() });
            cv.text_in(hr.offset(5, 0), &col.caption, font, tc, HAlign::Left, VAlign::Center, false);
            x += w;
        }
        if need_sb {
            // the header goes on above the scroll bar
            th.button(cv, Rect::new(inner.x + lw, inner.y, SB, hh), BtnState { enabled: true, ..Default::default() });
        }
        let mut y = inner.y + hh;
        for (ri, row) in c.cells.iter().enumerate().skip(top as usize) {
            let rr = Rect::new(inner.x, y, lw, rh);
            let sel = c.item_index == ri as i32;
            let o2 = cv.clip_to(Rect::new(inner.x, inner.y + hh, lw, inner.h - hh));
            if sel {
                th.selection(cv, rr, focused);
            }
            let mut x = inner.x;
            for (ci, _) in c.cols.iter().enumerate() {
                let w = widths[ci];
                let txt = row.get(ci).map(String::as_str).unwrap_or("");
                let o3 = cv.clip_to(Rect::new(x, y, w - 2, rh));
                let mut tx = x + 5;
                if ci == 0 && c.checkboxes {
                    let on = c.checks.get(ri).copied().unwrap_or(false);
                    th.checkbox(cv, Rect::new(x + 3, y + (rh - 13) / 2, 13, 13), on as u8, en, false);
                    tx += 17;
                }
                cv.text_in(Rect::new(tx, y, w - 6 - (tx - x - 5), rh), txt, font, if sel { th.selection_text(focused) } else { tc }, HAlign::Left, VAlign::Center, false);
                cv.set_clip(o3);
                x += w;
            }
            cv.set_clip(o2);
            y += rh;
            if y > inner.bottom() {
                break;
            }
        }
        cv.set_clip(old);
        if need_sb {
            let sr = Rect::new(inner.right() - SB, inner.y + hh, SB, inner.h - hh);
            let hp = if self.hot == Some(id) { self.hot_part } else { PART_NONE };
            paint_scrollbar(cv, th, sr, true, 0, max as i64, vis as i64, top as i64, hp, if self.capture == Some(id) { self.cap_part } else { PART_NONE }, en);
        }
    }

    fn paint_combo_list(&self, cv: &mut Canvas, th: &Theme, id: Id, hot: i32, top: i32) {
        let c = &self.ctl[id];
        let font = self.font(id);
        let lr = combo_list_rect(self, id);
        cv.fill(lr, th.window());
        cv.frame(lr, th.menu_border());
        let ih = line_height(&font) + 2;
        let vis = (lr.h - 2) / ih;
        let need_sb = c.items.len() as i32 > vis;
        let ir0 = Rect::new(lr.x + 1, lr.y + 1, lr.w - 2 - if need_sb { SB } else { 0 }, lr.h - 2);
        let old = cv.clip_to(ir0);
        for (i, it) in c.items.iter().enumerate().skip(top.max(0) as usize) {
            let y = ir0.y + (i as i32 - top) * ih;
            if y > ir0.bottom() {
                break;
            }
            let ir = Rect::new(ir0.x, y, ir0.w, ih);
            let col = c.item_colors.get(i).copied().flatten().unwrap_or(th.text());
            if i as i32 == hot {
                th.selection(cv, ir, true);
                cv.text_in(ir.offset(3, 0), it, &font, th.selection_text(true), HAlign::Left, VAlign::Center, false);
            } else {
                cv.text_in(ir.offset(3, 0), it, &font, col, HAlign::Left, VAlign::Center, false);
            }
        }
        cv.set_clip(old);
        if need_sb {
            let sr = Rect::new(lr.right() - 1 - SB, lr.y + 1, SB, lr.h - 2);
            paint_scrollbar(cv, th, sr, true, 0, c.items.len() as i64 - vis as i64, vis as i64, top as i64, PART_NONE, PART_NONE, true);
        }
    }

    fn paint_menu(&self, cv: &mut Canvas, th: &Theme, lv: &MenuLevel) {
        let font = self.font(0);
        cv.fill(lv.rect, th.menu_bg());
        cv.frame(lv.rect, th.menu_border());
        for (i, (k, ir)) in menu_item_rects(self, lv).into_iter().enumerate() {
            let c = &self.ctl[k];
            if c.caption == "-" {
                cv.hline(ir.x + 24, ir.right() - 4, ir.y + 4, th.shadow());
                cv.hline(ir.x + 24, ir.right() - 4, ir.y + 5, th.light());
                continue;
            }
            let hot = lv.hot == i as i32 && c.enabled;
            if hot {
                th.menu_hot(cv, ir);
            }
            let col = if !c.enabled { th.text_disabled() } else if hot { th.menu_hot_text() } else { th.text() };
            if c.checked {
                let cr = Rect::new(ir.x + 6, ir.y + (ir.h - 12) / 2, 12, 12);
                if c.radio_item {
                    cv.ellipse(cr.inset(3), Some(col), None);
                } else {
                    cv.line_aa((cr.x + 2) as f32, (cr.y + 6) as f32, (cr.x + 5) as f32, (cr.y + 9) as f32, 1.5, col);
                    cv.line_aa((cr.x + 5) as f32, (cr.y + 9) as f32, (cr.x + 10) as f32, (cr.y + 3) as f32, 1.5, col);
                }
            }
            cv.text_in(Rect::new(ir.x + 26, ir.y, ir.w - 40, ir.h), &strip_amp(&c.caption), &font, col, HAlign::Left, VAlign::Center, false);
            if !menu_items(self, k).is_empty() {
                th.arrow(cv, Rect::new(ir.right() - 14, ir.y, 12, ir.h), Dir::Right, col);
            }
        }
    }

    fn paint_hint(&self, cv: &mut Canvas, th: &Theme, text: &str, x: i32, y: i32) {
        let font = Font::default();
        let lines = cv.layout_lines(text, &font, 400, true);
        let lh = line_height(&font);
        let w = lines.iter().map(|l| cv.text_width(&font, l)).max().unwrap_or(0) + 8;
        let h = lh * lines.len() as i32 + 4;
        let (cw, ch) = self.client_size();
        let x = x.min(cw - w - 1).max(0);
        let y = if y + h > ch { (y - h - 24).max(0) } else { y };
        let r = Rect::new(x, y, w, h);
        cv.fill(r, th.hint_bg());
        cv.frame(r, th.v("hint.border"));
        let mut yy = y + 2;
        for l in lines {
            cv.text(x + 4, yy, &l, &font, th.hint_text());
            yy += lh;
        }
    }
}

/// The two button rectangles of an up-down (up/left first).
pub fn updown_rects(c: &Control, r: Rect) -> (Rect, Rect) {
    if c.vertical {
        let w = r.w / 2;
        (Rect::new(r.x, r.y, w, r.h), Rect::new(r.x + w, r.y, r.w - w, r.h))
    } else {
        let h = r.h / 2;
        (Rect::new(r.x, r.y, r.w, h), Rect::new(r.x, r.y + h, r.w, r.h - h))
    }
}

/// The application icon of the title bar: the orange "bulb" of MB3D's icon
/// (mb3d-rs.ico; without the RS badge at this size).
fn draw_app_icon(cv: &mut Canvas, x: i32, y: i32) {
    use std::sync::OnceLock;
    static ICON: OnceLock<Option<super::bitmap::Bitmap>> = OnceLock::new();
    let icon = ICON.get_or_init(|| decode_ico(include_bytes!("../../assets/mb3d-rs.ico"), 16));
    match icon {
        Some(b) => cv.image_stretch(Rect::new(x, y, 16, 16), b, true),
        None => cv.ellipse(Rect::new(x + 2, y + 2, 12, 12), Some(0xFFE07020), None),
    }
}

/// Decodes the image of an .ico file closest to `size` pixels.
pub fn decode_ico(d: &[u8], size: usize) -> Option<super::bitmap::Bitmap> {
    if d.len() < 6 || d[2] != 1 {
        return None;
    }
    let n = u16::from_le_bytes([d[4], d[5]]) as usize;
    let mut best: Option<(usize, usize, usize)> = None; // (diff, offset, len)
    for i in 0..n {
        let e = 6 + i * 16;
        let e = d.get(e..e + 16)?;
        let w = if e[0] == 0 { 256 } else { e[0] as usize };
        let len = u32::from_le_bytes([e[8], e[9], e[10], e[11]]) as usize;
        let off = u32::from_le_bytes([e[12], e[13], e[14], e[15]]) as usize;
        let bpp = u16::from_le_bytes([e[6], e[7]]) as usize;
        let diff = w.abs_diff(size) * 64 + (32 - bpp.min(32));
        if best.is_none_or(|b| diff < b.0) {
            best = Some((diff, off, len));
        }
    }
    let (_, off, len) = best?;
    let img = d.get(off..off + len)?;
    if img.starts_with(&[0x89, b'P', b'N', b'G']) {
        return super::bitmap::decode_any(img);
    }
    // a BMP without file header, height doubled (XOR + AND mask)
    let hs = u32::from_le_bytes(img.get(0..4)?.try_into().ok()?) as usize;
    let w = i32::from_le_bytes(img.get(4..8)?.try_into().ok()?) as usize;
    let h2 = i32::from_le_bytes(img.get(8..12)?.try_into().ok()?) as usize;
    let bpp = u16::from_le_bytes(img.get(14..16)?.try_into().ok()?) as usize;
    let h = h2 / 2;
    let ncol = if bpp <= 8 { 1usize << bpp } else { 0 };
    let pal_len = ncol * 4;
    let mut bmp = Vec::new();
    let pix_off = 14 + hs + pal_len;
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((14 + img.len()) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]);
    bmp.extend_from_slice(&(pix_off as u32).to_le_bytes());
    let mut hdr = img[..hs].to_vec();
    hdr[8..12].copy_from_slice(&(h as i32).to_le_bytes());
    bmp.extend_from_slice(&hdr);
    bmp.extend_from_slice(&img[hs..]);
    let mut b = super::bitmap::decode_bmp(&bmp)?;
    // AND mask after the XOR bitmap
    if bpp < 32 {
        let xor_stride = (w * bpp).div_ceil(32) * 4;
        let mask_off = hs + pal_len + xor_stride * h;
        let mstride = w.div_ceil(32) * 4;
        for y in 0..h {
            for x in 0..w {
                let byte = img.get(mask_off + (h - 1 - y) * mstride + x / 8).copied().unwrap_or(0);
                if byte >> (7 - x % 8) & 1 == 1 {
                    b.px[y * w + x] = 0;
                }
            }
        }
    }
    Some(b)
}

/// The title bar LED: a sunken frame with the lamp lit in `c` (with a
/// highlight stripe) or dark.
fn draw_led(cv: &mut Canvas, r: Rect, c: u32, on: bool) {
    let mix = |c: u32, t: u32, k: u32| {
        let ch = |s: u32| ((c >> s & 255) * (256 - k) + (t >> s & 255) * k) >> 8 << s;
        ch(16) | ch(8) | ch(0)
    };
    let c = c & 0xFF_FFFF;
    cv.fill(r, 0xFF20_2020);
    let lamp = Rect::new(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    if on {
        cv.fill(lamp, 0xFF00_0000 | c);
        cv.fill(Rect::new(lamp.x + 1, lamp.y, lamp.w - 2, 2), 0xFF00_0000 | mix(c, 0xFFFFFF, 140));
        cv.fill(Rect::new(lamp.x, lamp.y + lamp.h - 1, lamp.w, 1), 0xFF00_0000 | mix(c, 0, 80));
    } else {
        cv.fill(lamp, 0xFF00_0000 | mix(c, 0, 170));
    }
}
