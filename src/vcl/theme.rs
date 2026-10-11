//! The look of the controls: MB3D's default VCL style "Glossy" (dark,
//! glossy buttons) and "Windows" (the system look, MB3D's "Themes off").

use super::canvas::{mix, Canvas, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Glossy,
    Windows,
}

impl Style {
    pub fn name(self) -> &'static str {
        match self {
            Style::Glossy => "Glossy",
            Style::Windows => "Windows",
        }
    }
    pub fn from_name(s: &str) -> Option<Style> {
        match s.trim().to_ascii_lowercase().as_str() {
            "glossy" => Some(Style::Glossy),
            "windows" => Some(Style::Windows),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BtnState {
    pub hot: bool,
    pub pressed: bool,
    /// a speed button that is down (GroupIndex)
    pub down: bool,
    pub enabled: bool,
    pub focused: bool,
    pub default: bool,
    pub flat: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

const fn c(rgb: u32) -> u32 {
    0xFF00_0000 | rgb
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub style: Style,
}

impl Theme {
    pub fn new(style: Style) -> Theme {
        Theme { style }
    }

    pub fn glossy(&self) -> bool {
        self.style == Style::Glossy
    }

    /// Colour of a VCL colour value (`clBtnFace`, `$00BBGGRR`, ...).
    pub fn color(&self, v: i64) -> u32 {
        if v < 0 || v as u32 & 0xFF00_0000 != 0 {
            // system colour: index in the low byte
            return self.sys_index((v as u32 & 0xFF) as u8);
        }
        let v = v as u32;
        c((v & 0xFF) << 16 | (v & 0xFF00) | (v >> 16 & 0xFF))
    }

    fn sys_index(&self, i: u8) -> u32 {
        match i {
            5 => self.window(),                  // clWindow
            8 => self.window_text(),             // clWindowText
            13 => self.highlight(),              // clHighlight
            14 => self.highlight_text(),         // clHighlightText
            17 => self.text_disabled(),          // clGrayText
            18 => self.text(),                   // clBtnText
            16 => self.shadow(),                 // clBtnShadow
            20 => self.light(),                  // clBtnHighlight
            21 => c(0x000000),                   // cl3DDkShadow
            22 => self.face(),                   // cl3DLight
            23 => self.text(),                   // clInfoText
            24 => self.hint_bg(),                // clInfoBk
            9 => self.text(),                    // clCaptionText
            1 | 2 | 3 | 27 | 28 => self.face(),  // desktop, active/inactive caption
            10 | 11 => self.shadow(),            // borders
            12 => self.window(),                 // clAppWorkSpace
            _ => self.face(),                    // clBtnFace, clMenu, ...
        }
    }

    /// Named colour constants of the form files.
    pub fn named(&self, name: &str) -> Option<u32> {
        let sys = |i: u8| Some(self.sys_index(i));
        match name.to_ascii_lowercase().as_str() {
            "clbtnface" | "clmenu" | "clform" | "cl3dlight" | "clactiveborder" | "clinactiveborder" | "clscrollbar" | "clmenubar" => sys(15),
            "clwindow" => sys(5),
            "clwindowtext" | "clbtntext" | "clmenutext" | "clcaptiontext" | "clinfotext" => sys(8),
            "clhighlight" | "clmenuhighlight" | "clhotlight" => sys(13),
            "clhighlighttext" => sys(14),
            "clgraytext" | "clinactivecaptiontext" => sys(17),
            "clbtnshadow" => sys(16),
            "clbtnhighlight" => sys(20),
            "cl3ddkshadow" => sys(21),
            "clinfobk" => sys(24),
            "clappworkspace" => sys(12),
            "clblack" => Some(c(0x000000)),
            "clmaroon" => Some(c(0x800000)),
            "clgreen" => Some(c(0x008000)),
            "clolive" => Some(c(0x808000)),
            "clnavy" => Some(c(0x000080)),
            "clpurple" => Some(c(0x800080)),
            "clteal" => Some(c(0x008080)),
            "clgray" | "clgrey" | "cldkgray" => Some(c(0x808080)),
            "clsilver" | "clltgray" => Some(c(0xC0C0C0)),
            "clred" => Some(c(0xFF0000)),
            "cllime" => Some(c(0x00FF00)),
            "clyellow" => Some(c(0xFFFF00)),
            "clblue" => Some(c(0x0000FF)),
            "clfuchsia" => Some(c(0xFF00FF)),
            "claqua" => Some(c(0x00FFFF)),
            "clwhite" => Some(c(0xFFFFFF)),
            "clmoneygreen" => Some(c(0xC0DCC0)),
            "clskyblue" => Some(c(0xA6CAF0)),
            "clcream" => Some(c(0xFFFBF0)),
            "clmedgray" => Some(c(0xA0A0A4)),
            "clnone" | "cldefault" => None,
            _ => None,
        }
    }

    /// Whether a named colour is a system colour (follows the style).
    pub fn is_system(name: &str) -> bool {
        let n = name.to_ascii_lowercase();
        n.starts_with("cl") && !matches!(
            n.as_str(),
            "clblack" | "clmaroon" | "clgreen" | "clolive" | "clnavy" | "clpurple" | "clteal" | "clgray" | "clgrey" | "cldkgray"
                | "clsilver" | "clltgray" | "clred" | "cllime" | "clyellow" | "clblue" | "clfuchsia" | "claqua" | "clwhite"
                | "clmoneygreen" | "clskyblue" | "clcream" | "clmedgray"
        )
    }

    // ---- basic colours

    pub fn face(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x252525),
            Style::Windows => c(0xF0F0F0),
        }
    }
    pub fn face_dither(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x282828),
            Style::Windows => c(0xF0F0F0),
        }
    }
    pub fn text(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0xE6E6E6),
            Style::Windows => c(0x000000),
        }
    }
    pub fn text_disabled(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x6E6E6E),
            Style::Windows => c(0xA0A0A0),
        }
    }
    pub fn window(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x121212),
            Style::Windows => c(0xFFFFFF),
        }
    }
    pub fn window_text(&self) -> u32 {
        self.text()
    }
    pub fn highlight(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x3EE8E0),
            Style::Windows => c(0x0078D7),
        }
    }
    pub fn highlight_text(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x000000),
            Style::Windows => c(0xFFFFFF),
        }
    }
    pub fn shadow(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x0C0C0C),
            Style::Windows => c(0xA0A0A0),
        }
    }
    pub fn light(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x3C3C3C),
            Style::Windows => c(0xFFFFFF),
        }
    }
    pub fn hint_bg(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x3A3A3A),
            Style::Windows => c(0xFFFFE1),
        }
    }
    pub fn hint_text(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0xF0F0F0),
            Style::Windows => c(0x000000),
        }
    }
    pub fn image_bg(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x121212),
            Style::Windows => c(0xABABAB),
        }
    }
    /// The colour of a text that the form gave explicitly: dark colours
    /// would vanish on the dark style and are replaced by the text colour.
    pub fn fix_text(&self, col: u32) -> u32 {
        if self.glossy() {
            let l = ((col >> 16 & 255) * 30 + (col >> 8 & 255) * 59 + (col & 255) * 11) / 100;
            if l < 90 {
                return self.text();
            }
        }
        col
    }

    /// Background of a form or panel (Glossy: fine dither pattern).
    pub fn fill_face(&self, cv: &mut Canvas, r: Rect, col: u32) {
        cv.fill(r, col);
        if self.glossy() && col == self.face() {
            let alt = self.face_dither() & 0xFF_FFFF;
            let (x0, y0, x1, y1) = (cv.dx(r.x), cv.dy(r.y), cv.dx(r.right()), cv.dy(r.bottom()));
            let cl = cv.clip();
            let (x0, y0, x1, y1) = (x0.max(cl[0]), y0.max(cl[1]), x1.min(cl[2]), y1.min(cl[3]));
            for y in y0..y1 {
                let row = (y * cv.bw) as usize;
                let mut x = x0 + ((x0 + y) & 1);
                while x < x1 {
                    cv.buf[row + x as usize] = alt;
                    x += 2;
                }
            }
        }
    }

    // ---- buttons

    pub fn button(&self, cv: &mut Canvas, r: Rect, s: BtnState) {
        // a disabled button does not light up under the mouse
        let s = BtnState { hot: s.hot && s.enabled, ..s };
        match self.style {
            Style::Glossy => {
                if s.flat && !s.hot && !s.down && !s.pressed {
                    return;
                }
                let (mut t0, mut t1, mut b0, mut b1) = (c(0x4E4E4E), c(0x3A3A3A), c(0x1D1D1D), c(0x2C2C2C));
                if !s.enabled {
                    (t0, t1, b0, b1) = (c(0x333333), c(0x2D2D2D), c(0x222222), c(0x262626));
                } else if s.down || s.pressed {
                    // the style's "pressed" look (also a speed button that
                    // is down): blue gloss
                    (t0, t1, b0, b1) = (c(0x6A9BEA), c(0x3E74D6), c(0x1A4FB8), c(0x2E64CC));
                } else if s.hot {
                    // the style's "hot" look under the mouse: lighter blue gloss
                    (t0, t1, b0, b1) = (c(0x8DB6F2), c(0x5B8FE4), c(0x2A62CC), c(0x4479DC));
                }
                let border = if s.focused || s.default { c(0x5A7FBF) } else { c(0x0A0A0A) };
                // gloss: upper half lighter, sharp split, lower half dark
                let rad = 3.0;
                cv.round_rect(r, rad, t0, b1, Some(border));
                let inner = r.inset(1);
                let half = inner.h / 2;
                let old = cv.clip_to(Rect::new(inner.x, inner.y, inner.w, half));
                cv.round_rect(inner, rad - 1.0, t0, t1, None);
                cv.set_clip(old);
                let old = cv.clip_to(Rect::new(inner.x, inner.y + half, inner.w, inner.h - half));
                cv.round_rect(inner, rad - 1.0, b0, b1, None);
                cv.set_clip(old);
            }
            Style::Windows => {
                if s.flat && !s.hot && !s.down && !s.pressed {
                    return;
                }
                let (bg, br) = if !s.enabled {
                    (c(0xCCCCCC), c(0xBFBFBF))
                } else if s.pressed {
                    (c(0xCCE4F7), c(0x005499))
                } else if s.down {
                    (c(0xC4DDF2), c(0x2C628B))
                } else if s.hot {
                    (c(0xE5F1FB), c(0x0078D7))
                } else if s.default || s.focused {
                    (c(0xE1E1E1), c(0x0078D7))
                } else {
                    (c(0xE1E1E1), c(0xADADAD))
                };
                cv.fill(r, bg);
                cv.frame(r, br);
                if (s.default || s.focused) && s.enabled && !s.hot && !s.pressed {
                    cv.frame(r.inset(1), br);
                }
            }
        }
    }

    pub fn button_text(&self, s: &BtnState) -> u32 {
        if !s.enabled {
            return self.text_disabled();
        }
        match self.style {
            Style::Glossy if s.down || s.pressed || s.hot => c(0xFFFFFF),
            _ => self.text(),
        }
    }

    /// Small triangle arrow centred in `r`.
    pub fn arrow(&self, cv: &mut Canvas, r: Rect, d: Dir, col: u32) {
        let (cx, cy) = (r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0);
        let s = (r.w.min(r.h) as f32 * 0.22).clamp(2.0, 5.0);
        let pts = match d {
            Dir::Up => vec![(cx - s, cy + s * 0.5), (cx + s, cy + s * 0.5), (cx, cy - s * 0.5)],
            Dir::Down => vec![(cx - s, cy - s * 0.5), (cx + s, cy - s * 0.5), (cx, cy + s * 0.5)],
            Dir::Left => vec![(cx + s * 0.5, cy - s), (cx + s * 0.5, cy + s), (cx - s * 0.5, cy)],
            Dir::Right => vec![(cx - s * 0.5, cy - s), (cx - s * 0.5, cy + s), (cx + s * 0.5, cy)],
        };
        cv.polygon(&pts, col);
    }

    /// The small buttons of up-downs, scroll bars and combo boxes.
    pub fn small_button(&self, cv: &mut Canvas, r: Rect, d: Option<Dir>, s: BtnState) {
        match self.style {
            Style::Glossy => {
                let (t, b) = if !s.enabled {
                    (c(0x3A3A3A), c(0x303030))
                } else if s.pressed {
                    (c(0x505050), c(0x5C5C5C))
                } else if s.hot {
                    (c(0x8A8A8A), c(0x666666))
                } else {
                    (c(0x737373), c(0x535353))
                };
                cv.round_rect(r, 2.0, t, b, Some(c(0x101010)));
                if let Some(d) = d {
                    let col = if s.enabled { c(0x141414) } else { c(0x555555) };
                    self.arrow(cv, r, d, col);
                }
            }
            Style::Windows => {
                let (bg, br) = if s.pressed {
                    (c(0xCCE4F7), c(0x569DE5))
                } else if s.hot {
                    (c(0xE5F1FB), c(0x7EB4EA))
                } else {
                    (c(0xF0F0F0), c(0xD9D9D9))
                };
                cv.fill(r, bg);
                cv.frame(r, br);
                if let Some(d) = d {
                    let col = if s.enabled { c(0x606060) } else { c(0xBFBFBF) };
                    self.arrow(cv, r, d, col);
                }
            }
        }
    }

    // ---- edits, lists

    /// Frame and background of an edit / list box / memo; returns the
    /// inner rectangle.
    pub fn edit_frame(&self, cv: &mut Canvas, r: Rect, enabled: bool, focused: bool, bg: Option<u32>) -> Rect {
        let bgc = bg.unwrap_or(if enabled || self.glossy() { self.window() } else { c(0xF0F0F0) });
        match self.style {
            Style::Glossy => {
                cv.fill(r, bgc);
                cv.edge(r, c(0x0A0A0A), c(0x3A3A3A));
                if focused {
                    cv.frame(r, c(0x4A6A9A));
                }
            }
            Style::Windows => {
                cv.fill(r, bgc);
                cv.frame(r, if focused { c(0x0078D7) } else { c(0x7A7A7A) });
            }
        }
        r.inset(2)
    }

    // ---- check boxes, radio buttons

    pub fn checkbox(&self, cv: &mut Canvas, r: Rect, state: u8, enabled: bool, hot: bool) {
        match self.style {
            Style::Glossy => {
                let (t, b) = if !enabled {
                    (c(0x3C3C3C), c(0x333333))
                } else if hot {
                    (c(0x7A7A7A), c(0x5A5A5A))
                } else {
                    (c(0x6A6A6A), c(0x4C4C4C))
                };
                cv.round_rect(r, 1.5, t, b, Some(c(0x101010)));
                let col = if enabled { c(0xF4F4F4) } else { c(0x777777) };
                self.check_mark(cv, r, state, col);
            }
            Style::Windows => {
                cv.fill(r, if enabled { c(0xFFFFFF) } else { c(0xE6E6E6) });
                cv.frame(r, if hot { c(0x0078D7) } else if enabled { c(0x333333) } else { c(0xBCBCBC) });
                let col = if enabled { c(0x000000) } else { c(0xBCBCBC) };
                self.check_mark(cv, r, state, col);
            }
        }
    }

    fn check_mark(&self, cv: &mut Canvas, r: Rect, state: u8, col: u32) {
        match state {
            1 => {
                let (x, y, w, h) = (r.x as f32, r.y as f32, r.w as f32, r.h as f32);
                cv.line_aa(x + w * 0.22, y + h * 0.52, x + w * 0.42, y + h * 0.72, 1.6, col);
                cv.line_aa(x + w * 0.42, y + h * 0.72, x + w * 0.80, y + h * 0.26, 1.6, col);
            }
            2 => cv.fill(r.inset(3), col),
            _ => {}
        }
    }

    pub fn radio(&self, cv: &mut Canvas, r: Rect, on: bool, enabled: bool, hot: bool) {
        match self.style {
            Style::Glossy => {
                let fill = if !enabled { c(0x3A3A3A) } else if hot { c(0x777777) } else { c(0x626262) };
                cv.ellipse(r, Some(fill), Some(c(0x101010)));
                if on {
                    cv.ellipse(r.inset(3), Some(if enabled { c(0xF0F0F0) } else { c(0x808080) }), None);
                }
            }
            Style::Windows => {
                cv.ellipse(r, Some(if enabled { c(0xFFFFFF) } else { c(0xE6E6E6) }), Some(if hot { c(0x0078D7) } else { c(0x333333) }));
                if on {
                    cv.ellipse(r.inset(3), Some(if enabled { c(0x000000) } else { c(0xBCBCBC) }), None);
                }
            }
        }
    }

    // ---- frames

    pub fn groupbox(&self, cv: &mut Canvas, r: Rect, gap: Option<(i32, i32)>) {
        let col = match self.style {
            Style::Glossy => c(0x3C3C3C),
            Style::Windows => c(0xDCDCDC),
        };
        let old = cv.clip();
        // leave the caption gap open
        let mut parts = vec![r];
        if let Some((gx, gw)) = gap {
            parts = vec![
                Rect::new(r.x, r.y, gx - r.x, r.h),
                Rect::new(gx + gw, r.y, r.right() - gx - gw, r.h),
                Rect::new(gx, r.y + 2, gw, r.h - 2),
            ];
        }
        for p in parts {
            cv.clip_to(p);
            if self.glossy() {
                cv.round_rect(r, 4.0, self.face(), self.face(), Some(col));
                // keep the face pattern inside
                self.fill_face(cv, r.inset(2), self.face());
            } else {
                cv.frame(r, col);
            }
            cv.set_clip(old);
        }
    }

    /// TPanel / TBevel edges: `raised` true = bvRaised, false = bvLowered.
    pub fn bevel(&self, cv: &mut Canvas, r: Rect, raised: bool) {
        let (l, d) = match self.style {
            Style::Glossy => (c(0x3E3E3E), c(0x0E0E0E)),
            Style::Windows => (c(0xFFFFFF), c(0xA0A0A0)),
        };
        if raised {
            cv.edge(r, l, d);
        } else {
            cv.edge(r, d, l);
        }
    }

    // ---- tabs

    pub fn tab(&self, cv: &mut Canvas, r: Rect, selected: bool, hot: bool, buttons: bool) {
        match self.style {
            Style::Glossy => {
                if buttons || !selected {
                    let s = BtnState { hot, enabled: true, down: selected && buttons, ..Default::default() };
                    if selected && !buttons {
                        cv.round_rect(r, 3.0, c(0x4A4A4A), c(0x2E2E2E), Some(c(0x0A0A0A)));
                    } else {
                        let (t, b) = if hot { (c(0x3E3E3E), c(0x2A2A2A)) } else { (c(0x313131), c(0x222222)) };
                        if buttons {
                            self.button(cv, r, s);
                        } else {
                            cv.round_rect(r, 3.0, t, b, Some(c(0x0A0A0A)));
                        }
                    }
                } else {
                    cv.round_rect(r, 3.0, c(0x4A4A4A), c(0x2E2E2E), Some(c(0x0A0A0A)));
                }
            }
            Style::Windows => {
                let bg = if selected { c(0xFFFFFF) } else if hot { c(0xD8EAF9) } else { c(0xF0F0F0) };
                cv.fill(r, bg);
                cv.frame(r, c(0xD9D9D9));
            }
        }
    }

    pub fn tab_body(&self, cv: &mut Canvas, r: Rect) {
        match self.style {
            Style::Glossy => {
                self.fill_face(cv, r, self.face());
                cv.frame(r, c(0x3A3A3A));
            }
            Style::Windows => {
                cv.fill(r, c(0xFFFFFF));
                cv.frame(r, c(0xD9D9D9));
            }
        }
    }

    pub fn tab_text(&self, selected: bool) -> u32 {
        match self.style {
            Style::Glossy if !selected => c(0x9A9A9A),
            _ => self.text(),
        }
    }

    // ---- track bars, progress bars, scroll bars

    pub fn track_channel(&self, cv: &mut Canvas, r: Rect) {
        match self.style {
            Style::Glossy => {
                cv.fill(r, c(0x101010));
                cv.edge(r, c(0x0A0A0A), c(0x3A3A3A));
            }
            Style::Windows => {
                cv.fill(r, c(0xE7EAEA));
                cv.frame(r, c(0xD6D6D6));
            }
        }
    }

    pub fn track_thumb(&self, cv: &mut Canvas, r: Rect, s: BtnState) {
        match self.style {
            Style::Glossy => {
                let (t, b) = if s.pressed {
                    (c(0x5A5A5A), c(0x4A4A4A))
                } else if s.hot {
                    (c(0x9A9A9A), c(0x6A6A6A))
                } else {
                    (c(0x868686), c(0x585858))
                };
                cv.round_rect(r, 2.0, t, b, Some(c(0x0E0E0E)));
            }
            Style::Windows => {
                let col = if s.pressed { c(0xCCCCCC) } else if s.hot { c(0x171717) } else { c(0x007AD9) };
                cv.fill(r, col);
            }
        }
    }

    pub fn tick(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x8A8A8A),
            Style::Windows => c(0xC4C4C4),
        }
    }

    pub fn progress(&self, cv: &mut Canvas, r: Rect, frac: f32, vertical: bool) {
        match self.style {
            Style::Glossy => {
                cv.fill(r, c(0x101010));
                cv.edge(r, c(0x0A0A0A), c(0x303030));
                let inner = r.inset(1);
                let f = if vertical {
                    let h = (inner.h as f32 * frac.clamp(0.0, 1.0)).round() as i32;
                    Rect::new(inner.x, inner.bottom() - h, inner.w, h)
                } else {
                    Rect::new(inner.x, inner.y, (inner.w as f32 * frac.clamp(0.0, 1.0)).round() as i32, inner.h)
                };
                cv.gradient(f, c(0x6FF8F0), c(0x18C8C0));
            }
            Style::Windows => {
                cv.fill(r, c(0xE6E6E6));
                cv.frame(r, c(0xBCBCBC));
                let inner = r.inset(1);
                let f = if vertical {
                    let h = (inner.h as f32 * frac.clamp(0.0, 1.0)).round() as i32;
                    Rect::new(inner.x, inner.bottom() - h, inner.w, h)
                } else {
                    Rect::new(inner.x, inner.y, (inner.w as f32 * frac.clamp(0.0, 1.0)).round() as i32, inner.h)
                };
                cv.fill(f, c(0x06B025));
            }
        }
    }

    pub fn scroll_track(&self, cv: &mut Canvas, r: Rect) {
        cv.fill(r, match self.style {
            Style::Glossy => c(0x1A1A1A),
            Style::Windows => c(0xF0F0F0),
        });
    }

    pub fn scroll_thumb(&self, cv: &mut Canvas, r: Rect, s: BtnState) {
        match self.style {
            Style::Glossy => {
                let (t, b) = if s.pressed || s.hot { (c(0x7A7A7A), c(0x5A5A5A)) } else { (c(0x5E5E5E), c(0x444444)) };
                cv.round_rect(r.inset(1), 2.0, t, b, Some(c(0x101010)));
            }
            Style::Windows => {
                let col = if s.pressed { c(0x606060) } else if s.hot { c(0xA6A6A6) } else { c(0xCDCDCD) };
                cv.fill(r.inset(1), col);
            }
        }
    }

    // ---- menus, hints, list selection

    pub fn menu_bg(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x2B2B2B),
            Style::Windows => c(0xF2F2F2),
        }
    }
    pub fn menu_border(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0x0A0A0A),
            Style::Windows => c(0xCCCCCC),
        }
    }
    pub fn menu_hot(&self, cv: &mut Canvas, r: Rect) {
        match self.style {
            Style::Glossy => cv.round_rect(r, 2.0, c(0x6A9BEA), c(0x2E64CC), None),
            Style::Windows => cv.fill(r, c(0x91C9F7)),
        }
    }
    pub fn menu_hot_text(&self) -> u32 {
        match self.style {
            Style::Glossy => c(0xFFFFFF),
            Style::Windows => c(0x000000),
        }
    }

    pub fn selection(&self, cv: &mut Canvas, r: Rect, focused: bool) {
        let col = if focused || self.glossy() { self.highlight() } else { c(0xCDE8FF) };
        cv.fill(r, col);
    }
    pub fn selection_text(&self, focused: bool) -> u32 {
        if focused || self.glossy() {
            self.highlight_text()
        } else {
            c(0x000000)
        }
    }

    // ---- window frame

    pub fn caption_height(&self) -> i32 {
        match self.style {
            Style::Glossy => 24,
            Style::Windows => 30,
        }
    }
    pub fn tool_caption_height(&self) -> i32 {
        match self.style {
            Style::Glossy => 20,
            Style::Windows => 24,
        }
    }

    /// Title bar background.
    pub fn caption(&self, cv: &mut Canvas, r: Rect, active: bool) {
        match self.style {
            Style::Glossy => {
                let (t, m, b) = if active { (c(0x4A4A48), c(0x2A2A2C), c(0x161818)) } else { (c(0x3A3A3A), c(0x262626), c(0x1A1A1A)) };
                let half = r.h / 2;
                cv.gradient(Rect::new(r.x, r.y, r.w, half), t, m);
                cv.gradient(Rect::new(r.x, r.y + half, r.w, r.h - half), mix(m, b, 0.6), b);
                cv.hline(r.x, r.right(), r.bottom() - 1, c(0x080808));
            }
            Style::Windows => {
                cv.fill(r, if active { c(0xFFFFFF) } else { c(0xF3F3F3) });
                cv.hline(r.x, r.right(), r.bottom() - 1, c(0xE5E5E5));
            }
        }
    }
    pub fn caption_text(&self, active: bool) -> u32 {
        match self.style {
            Style::Glossy => {
                if active {
                    c(0xF2F2F2)
                } else {
                    c(0x9A9A9A)
                }
            }
            Style::Windows => {
                if active {
                    c(0x000000)
                } else {
                    c(0x999999)
                }
            }
        }
    }

    /// Window border colour (around the client area).
    pub fn window_border(&self, active: bool) -> u32 {
        match self.style {
            Style::Glossy => {
                if active {
                    c(0x3A3A3A)
                } else {
                    c(0x2A2A2A)
                }
            }
            Style::Windows => {
                if active {
                    c(0x0078D7)
                } else {
                    c(0xAAAAAA)
                }
            }
        }
    }

    /// Title bar buttons: 0 close, 1 maximise, 2 minimise.
    pub fn caption_button(&self, cv: &mut Canvas, r: Rect, kind: u8, hot: bool, pressed: bool) {
        match self.style {
            Style::Glossy => {
                let d = r.w.min(r.h) - 4;
                let cr = Rect::new(r.x + (r.w - d) / 2, r.y + (r.h - d) / 2, d, d);
                let fill = if pressed { c(0x202020) } else if hot { if kind == 0 { c(0xB03030) } else { c(0x5A5A5A) } } else { c(0x3E3E3E) };
                cv.ellipse(cr, Some(fill), Some(c(0x0A0A0A)));
                let col = c(0xD8D8D8);
                let (x, y, w) = (cr.x as f32, cr.y as f32, cr.w as f32);
                match kind {
                    0 => {
                        cv.line_aa(x + w * 0.33, y + w * 0.33, x + w * 0.67, y + w * 0.67, 1.4, col);
                        cv.line_aa(x + w * 0.67, y + w * 0.33, x + w * 0.33, y + w * 0.67, 1.4, col);
                    }
                    1 => {
                        let q = Rect::new(cr.x + d / 3, cr.y + d / 3, d - 2 * (d / 3), d - 2 * (d / 3));
                        cv.frame(q, col);
                    }
                    _ => cv.line_aa(x + w * 0.32, y + w * 0.5, x + w * 0.68, y + w * 0.5, 1.6, col),
                }
            }
            Style::Windows => {
                if hot || pressed {
                    cv.fill(r, if kind == 0 { c(0xE81123) } else { c(0xE5E5E5) });
                }
                let col = if kind == 0 && (hot || pressed) { c(0xFFFFFF) } else { c(0x000000) };
                let (cx, cy) = (r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0);
                match kind {
                    0 => {
                        cv.line_aa(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.0, col);
                        cv.line_aa(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.0, col);
                    }
                    1 => cv.frame(Rect::new(cx as i32 - 5, cy as i32 - 5, 10, 10), col),
                    _ => cv.hline(cx as i32 - 5, cx as i32 + 5, cy as i32, col),
                }
            }
        }
    }
}
