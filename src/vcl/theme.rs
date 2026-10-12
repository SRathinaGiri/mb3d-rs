//! The look of the controls.  A theme is a set of named colours and sizes
//! read from an XML file (`src/vcl/themes/*.xml` built in, more in the
//! program's `Themes` folder) drawn in one of two shape families: `gloss`
//! (MB3D's default VCL style "Glossy": dark, glossy buttons) or `flat` (the
//! system look, MB3D's "Themes off").

use super::canvas::{mix, Canvas, Rect};
use std::collections::HashMap;
use std::sync::RwLock;

/// How a theme draws its controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Gloss,
    Flat,
}

/// A theme as read from its XML file: the name, the shape family and the
/// named values (colours as 0xAARRGGBB, sizes in pixels).
#[derive(Debug)]
pub struct ThemeData {
    pub name: String,
    pub family: Family,
    vals: HashMap<String, u32>,
}

/// A theme of the registry: the built-in ones first, then those of the
/// `Themes` folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style(u16);

#[allow(non_upper_case_globals)]
impl Style {
    pub const Glossy: Style = Style(0);
    pub const Windows: Style = Style(1);

    pub fn name(self) -> &'static str {
        data(self).name.as_str()
    }
    pub fn from_name(s: &str) -> Option<Style> {
        let s = s.trim();
        registry().iter().position(|d| d.name.eq_ignore_ascii_case(s)).map(|i| Style(i as u16))
    }
    /// All themes, in the order of the registry.
    pub fn all() -> Vec<Style> {
        (0..registry().len()).map(|i| Style(i as u16)).collect()
    }
}

/// The built-in themes (the first two are Glossy and Windows, the defaults
/// of the two families; the others take what they leave out from them).
const BUILT_IN: [&str; 8] = [
    include_str!("themes/Glossy.xml"),
    include_str!("themes/Windows.xml"),
    include_str!("themes/Graphite.xml"),
    include_str!("themes/Midnight.xml"),
    include_str!("themes/Ember.xml"),
    include_str!("themes/Forest.xml"),
    include_str!("themes/Daylight.xml"),
    include_str!("themes/Highcontrast.xml"),
];

static REGISTRY: RwLock<Vec<&'static ThemeData>> = RwLock::new(Vec::new());

fn registry() -> std::sync::RwLockReadGuard<'static, Vec<&'static ThemeData>> {
    {
        let r = REGISTRY.read().unwrap();
        if !r.is_empty() {
            return r;
        }
    }
    {
        let mut w = REGISTRY.write().unwrap();
        if w.is_empty() {
            for t in BUILT_IN {
                let d = parse(t, &w).expect("built-in theme");
                w.push(Box::leak(Box::new(d)));
            }
        }
    }
    REGISTRY.read().unwrap()
}

fn data(s: Style) -> &'static ThemeData {
    let r = registry();
    r.get(s.0 as usize).copied().unwrap_or(r[0])
}

/// Adds the themes of the `.xml` files in `dir` (a theme whose name is
/// already known is skipped); returns the files that could not be read.
pub fn load_dir(dir: &std::path::Path) -> Vec<String> {
    let mut errors = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return errors };
    let mut files: Vec<_> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml"))).collect();
    files.sort();
    // (the built-in themes first: the bases of the files')
    drop(registry());
    for f in files {
        let text = match std::fs::read_to_string(&f) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{}: {e}", f.display()));
                continue;
            }
        };
        let mut w = REGISTRY.write().unwrap();
        match parse(&text, &w) {
            Ok(d) if !w.iter().any(|t| t.name.eq_ignore_ascii_case(&d.name)) => w.push(Box::leak(Box::new(d))),
            Ok(_) => {}
            Err(e) => errors.push(format!("{}: {e}", f.display())),
        }
    }
    errors
}

/// A theme file: `<theme name=".." family="gloss|flat" base="..">` with
/// `<color name=".." value="#RRGGBB"/>` and `<size name=".." value="24"/>`
/// entries.  Values it leaves out come from `base`, or else from the
/// built-in theme of its family.
fn parse(text: &str, known: &[&'static ThemeData]) -> Result<ThemeData, String> {
    let mut rest = text;
    let mut theme: Option<ThemeData> = None;
    while let Some(i) = rest.find('<') {
        rest = &rest[i..];
        if rest.starts_with("<!--") {
            let e = rest.find("-->").ok_or("unclosed comment")?;
            rest = &rest[e + 3..];
            continue;
        }
        let e = rest.find('>').ok_or("unclosed tag")?;
        let tag = rest[1..e].trim().trim_end_matches('/').trim();
        rest = &rest[e + 1..];
        if tag.starts_with('?') || tag.starts_with('/') {
            continue;
        }
        let (elem, attrs) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        let attr = |n: &str| -> Option<String> {
            let mut a = attrs;
            while let Some(eq) = a.find('=') {
                let key = a[..eq].trim();
                let v = a[eq + 1..].trim_start();
                let q = v.chars().next()?;
                let close = v[1..].find(q)? + 1;
                if key == n {
                    return Some(v[1..close].to_string());
                }
                a = &v[close + 1..];
            }
            None
        };
        match elem {
            "theme" => {
                let name = attr("name").ok_or("a theme without a name")?;
                let family = match attr("family").as_deref() {
                    Some("gloss") => Family::Gloss,
                    Some("flat") => Family::Flat,
                    f => return Err(format!("unknown family {f:?} (gloss or flat)")),
                };
                let base = match attr("base") {
                    Some(b) => Some(*known.iter().find(|t| t.name.eq_ignore_ascii_case(&b)).ok_or(format!("unknown base theme {b}"))?),
                    None => known.iter().find(|t| t.family == family).copied(),
                };
                theme = Some(ThemeData { name, family, vals: base.map(|b| b.vals.clone()).unwrap_or_default() });
            }
            "color" | "size" => {
                let t = theme.as_mut().ok_or("an entry outside <theme>")?;
                let n = attr("name").ok_or("an entry without a name")?;
                let v = attr("value").ok_or(format!("{n}: no value"))?;
                let v = if elem == "color" {
                    let h = v.trim().trim_start_matches('#');
                    let x = u32::from_str_radix(h, 16).map_err(|_| format!("{n}: not a colour: {v}"))?;
                    if h.len() != 6 {
                        return Err(format!("{n}: not a colour (#RRGGBB): {v}"));
                    }
                    0xFF00_0000 | x
                } else {
                    v.trim().parse::<u32>().map_err(|_| format!("{n}: not a size: {v}"))?
                };
                t.vals.insert(n, v);
            }
            other => return Err(format!("unknown element <{other}>")),
        }
    }
    theme.ok_or_else(|| "no <theme> element".into())
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
    d: &'static ThemeData,
}

impl Theme {
    pub fn new(style: Style) -> Theme {
        Theme { style, d: data(style) }
    }

    pub fn glossy(&self) -> bool {
        self.d.family == Family::Gloss
    }

    /// A named colour (or size) of the theme; magenta for a name it does
    /// not have (a theme file's typo shows).
    pub fn v(&self, name: &str) -> u32 {
        self.d.vals.get(name).copied().unwrap_or(0xFFFF_00FF)
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
            21 => self.v("dark.shadow"),                   // cl3DDkShadow
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
        match self.d.family {
            Family::Gloss => self.v("face"),
            Family::Flat => self.v("face"),
        }
    }
    pub fn face_dither(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("face.dither"),
            Family::Flat => self.v("face.dither"),
        }
    }
    pub fn text(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("text"),
            Family::Flat => self.v("text"),
        }
    }
    pub fn text_disabled(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("text.disabled"),
            Family::Flat => self.v("text.disabled"),
        }
    }
    pub fn window(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("window"),
            Family::Flat => self.v("window"),
        }
    }
    pub fn window_text(&self) -> u32 {
        self.text()
    }
    pub fn highlight(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("highlight"),
            Family::Flat => self.v("highlight"),
        }
    }
    pub fn highlight_text(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("highlight.text"),
            Family::Flat => self.v("highlight.text"),
        }
    }
    pub fn shadow(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("shadow"),
            Family::Flat => self.v("shadow"),
        }
    }
    pub fn light(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("light"),
            Family::Flat => self.v("light"),
        }
    }
    pub fn hint_bg(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("hint.bg"),
            Family::Flat => self.v("hint.bg"),
        }
    }
    pub fn hint_text(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("hint.text"),
            Family::Flat => self.v("hint.text"),
        }
    }
    pub fn image_bg(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("image.bg"),
            Family::Flat => self.v("image.bg"),
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
        match self.d.family {
            Family::Gloss => {
                if s.flat && !s.hot && !s.down && !s.pressed {
                    return;
                }
                let (mut t0, mut t1, mut b0, mut b1) = (self.v("button.normal.top1"), self.v("button.normal.top2"), self.v("button.normal.bottom1"), self.v("button.normal.bottom2"));
                if !s.enabled {
                    (t0, t1, b0, b1) = (self.v("button.disabled.top1"), self.v("button.disabled.top2"), self.v("button.disabled.bottom1"), self.v("button.disabled.bottom2"));
                } else if s.down || s.pressed {
                    // the style's "pressed" look (also a speed button that
                    // is down): blue gloss
                    (t0, t1, b0, b1) = (self.v("button.pressed.top1"), self.v("button.pressed.top2"), self.v("button.pressed.bottom1"), self.v("button.pressed.bottom2"));
                } else if s.hot {
                    // the style's "hot" look under the mouse: lighter blue gloss
                    (t0, t1, b0, b1) = (self.v("button.hot.top1"), self.v("button.hot.top2"), self.v("button.hot.bottom1"), self.v("button.hot.bottom2"));
                }
                let border = if s.focused || s.default { self.v("button.focused.border") } else { self.v("button.border") };
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
            Family::Flat => {
                if s.flat && !s.hot && !s.down && !s.pressed {
                    return;
                }
                let (bg, br) = if !s.enabled {
                    (self.v("button.disabled.bg"), self.v("button.disabled.border"))
                } else if s.pressed {
                    (self.v("button.pressed.bg"), self.v("button.pressed.border"))
                } else if s.down {
                    (self.v("button.down.bg"), self.v("button.down.border"))
                } else if s.hot {
                    (self.v("button.hot.bg"), self.v("button.hot.border"))
                } else if s.default || s.focused {
                    (self.v("button.focused.bg"), self.v("button.focused.border"))
                } else {
                    (self.v("button.normal.bg"), self.v("button.border"))
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
        match self.d.family {
            Family::Gloss if s.down || s.pressed || s.hot => self.v("button.active.text"),
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
        match self.d.family {
            Family::Gloss => {
                let (t, b) = if !s.enabled {
                    (self.v("small.disabled.top"), self.v("small.disabled.bottom"))
                } else if s.pressed {
                    (self.v("small.pressed.top"), self.v("small.pressed.bottom"))
                } else if s.hot {
                    (self.v("small.hot.top"), self.v("small.hot.bottom"))
                } else {
                    (self.v("small.normal.top"), self.v("small.normal.bottom"))
                };
                cv.round_rect(r, 2.0, t, b, Some(self.v("small.border")));
                if let Some(d) = d {
                    let col = if s.enabled { self.v("small.arrow") } else { self.v("small.arrow.disabled") };
                    self.arrow(cv, r, d, col);
                }
            }
            Family::Flat => {
                let (bg, br) = if s.pressed {
                    (self.v("small.pressed.bg"), self.v("small.pressed.border"))
                } else if s.hot {
                    (self.v("small.hot.bg"), self.v("small.hot.border"))
                } else {
                    (self.v("small.normal.bg"), self.v("small.border"))
                };
                cv.fill(r, bg);
                cv.frame(r, br);
                if let Some(d) = d {
                    let col = if s.enabled { self.v("small.arrow") } else { self.v("small.arrow.disabled") };
                    self.arrow(cv, r, d, col);
                }
            }
        }
    }

    // ---- edits, lists

    /// Frame and background of an edit / list box / memo; returns the
    /// inner rectangle.
    pub fn edit_frame(&self, cv: &mut Canvas, r: Rect, enabled: bool, focused: bool, bg: Option<u32>) -> Rect {
        let bgc = bg.unwrap_or(if enabled || self.glossy() { self.window() } else { self.v("edit.disabled.bg") });
        match self.d.family {
            Family::Gloss => {
                cv.fill(r, bgc);
                cv.edge(r, self.v("edit.edge.dark"), self.v("edit.edge.light"));
                if focused {
                    cv.frame(r, self.v("edit.focused.border"));
                }
            }
            Family::Flat => {
                cv.fill(r, bgc);
                cv.frame(r, if focused { self.v("edit.focused.border") } else { self.v("edit.border") });
            }
        }
        r.inset(2)
    }

    // ---- check boxes, radio buttons

    pub fn checkbox(&self, cv: &mut Canvas, r: Rect, state: u8, enabled: bool, hot: bool) {
        match self.d.family {
            Family::Gloss => {
                let (t, b) = if !enabled {
                    (self.v("check.disabled.top"), self.v("check.disabled.bottom"))
                } else if hot {
                    (self.v("check.hot.top"), self.v("check.hot.bottom"))
                } else {
                    (self.v("check.normal.top"), self.v("check.normal.bottom"))
                };
                cv.round_rect(r, 1.5, t, b, Some(self.v("check.border")));
                let col = if enabled { self.v("check.mark") } else { self.v("check.mark.disabled") };
                self.check_mark(cv, r, state, col);
            }
            Family::Flat => {
                cv.fill(r, if enabled { self.v("check.bg") } else { self.v("check.disabled.bg") });
                cv.frame(r, if hot { self.v("check.hot.border") } else if enabled { self.v("check.border") } else { self.v("check.disabled.border") });
                let col = if enabled { self.v("check.mark") } else { self.v("check.mark.disabled") };
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
        match self.d.family {
            Family::Gloss => {
                let fill = if !enabled { self.v("radio.disabled.bg") } else if hot { self.v("radio.hot.bg") } else { self.v("radio.bg") };
                cv.ellipse(r, Some(fill), Some(self.v("radio.border")));
                if on {
                    cv.ellipse(r.inset(3), Some(if enabled { self.v("radio.dot") } else { self.v("radio.dot.disabled") }), None);
                }
            }
            Family::Flat => {
                cv.ellipse(r, Some(if enabled { self.v("radio.bg") } else { self.v("radio.disabled.bg") }), Some(if hot { self.v("radio.hot.border") } else { self.v("radio.border") }));
                if on {
                    cv.ellipse(r.inset(3), Some(if enabled { self.v("radio.dot") } else { self.v("radio.dot.disabled") }), None);
                }
            }
        }
    }

    // ---- frames

    pub fn groupbox(&self, cv: &mut Canvas, r: Rect, gap: Option<(i32, i32)>) {
        let col = match self.d.family {
            Family::Gloss => self.v("group.border"),
            Family::Flat => self.v("group.border"),
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
        let (l, d) = match self.d.family {
            Family::Gloss => (self.v("bevel.light"), self.v("bevel.dark")),
            Family::Flat => (self.v("bevel.light"), self.v("bevel.dark")),
        };
        if raised {
            cv.edge(r, l, d);
        } else {
            cv.edge(r, d, l);
        }
    }

    // ---- tabs

    pub fn tab(&self, cv: &mut Canvas, r: Rect, selected: bool, hot: bool, buttons: bool) {
        match self.d.family {
            Family::Gloss => {
                if buttons || !selected {
                    let s = BtnState { hot, enabled: true, down: selected && buttons, ..Default::default() };
                    if selected && !buttons {
                        cv.round_rect(r, 3.0, self.v("tab.selected.top"), self.v("tab.selected.bottom"), Some(self.v("tab.border")));
                    } else {
                        let (t, b) = if hot { (self.v("tab.hot.top"), self.v("tab.hot.bottom")) } else { (self.v("tab.normal.top"), self.v("tab.normal.bottom")) };
                        if buttons {
                            self.button(cv, r, s);
                        } else {
                            cv.round_rect(r, 3.0, t, b, Some(self.v("tab.border")));
                        }
                    }
                } else {
                    cv.round_rect(r, 3.0, self.v("tab.selected.top"), self.v("tab.selected.bottom"), Some(self.v("tab.border")));
                }
            }
            Family::Flat => {
                let bg = if selected { self.v("tab.selected.bg") } else if hot { self.v("tab.hot.bg") } else { self.v("tab.normal.bg") };
                cv.fill(r, bg);
                cv.frame(r, self.v("tab.border"));
            }
        }
    }

    pub fn tab_body(&self, cv: &mut Canvas, r: Rect) {
        match self.d.family {
            Family::Gloss => {
                self.fill_face(cv, r, self.face());
                cv.frame(r, self.v("tab.body.border"));
            }
            Family::Flat => {
                cv.fill(r, self.v("tab.body.bg"));
                cv.frame(r, self.v("tab.body.border"));
            }
        }
    }

    pub fn tab_text(&self, selected: bool) -> u32 {
        match self.d.family {
            Family::Gloss if !selected => self.v("tab.unselected.text"),
            _ => self.text(),
        }
    }

    // ---- track bars, progress bars, scroll bars

    pub fn track_channel(&self, cv: &mut Canvas, r: Rect) {
        match self.d.family {
            Family::Gloss => {
                cv.fill(r, self.v("track.bg"));
                cv.edge(r, self.v("track.edge.dark"), self.v("track.edge.light"));
            }
            Family::Flat => {
                cv.fill(r, self.v("track.bg"));
                cv.frame(r, self.v("track.border"));
            }
        }
    }

    pub fn track_thumb(&self, cv: &mut Canvas, r: Rect, s: BtnState) {
        match self.d.family {
            Family::Gloss => {
                let (t, b) = if s.pressed {
                    (self.v("thumb.pressed.top"), self.v("thumb.pressed.bottom"))
                } else if s.hot {
                    (self.v("thumb.hot.top"), self.v("thumb.hot.bottom"))
                } else {
                    (self.v("thumb.normal.top"), self.v("thumb.normal.bottom"))
                };
                cv.round_rect(r, 2.0, t, b, Some(self.v("thumb.border")));
            }
            Family::Flat => {
                let col = if s.pressed { self.v("thumb.pressed") } else if s.hot { self.v("thumb.hot") } else { self.v("thumb.normal") };
                cv.fill(r, col);
            }
        }
    }

    pub fn tick(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("tick"),
            Family::Flat => self.v("tick"),
        }
    }

    pub fn progress(&self, cv: &mut Canvas, r: Rect, frac: f32, vertical: bool) {
        match self.d.family {
            Family::Gloss => {
                cv.fill(r, self.v("progress.bg"));
                cv.edge(r, self.v("progress.edge.dark"), self.v("progress.edge.light"));
                let inner = r.inset(1);
                let f = if vertical {
                    let h = (inner.h as f32 * frac.clamp(0.0, 1.0)).round() as i32;
                    Rect::new(inner.x, inner.bottom() - h, inner.w, h)
                } else {
                    Rect::new(inner.x, inner.y, (inner.w as f32 * frac.clamp(0.0, 1.0)).round() as i32, inner.h)
                };
                cv.gradient(f, self.v("progress.bar.top"), self.v("progress.bar.bottom"));
            }
            Family::Flat => {
                cv.fill(r, self.v("progress.bg"));
                cv.frame(r, self.v("progress.border"));
                let inner = r.inset(1);
                let f = if vertical {
                    let h = (inner.h as f32 * frac.clamp(0.0, 1.0)).round() as i32;
                    Rect::new(inner.x, inner.bottom() - h, inner.w, h)
                } else {
                    Rect::new(inner.x, inner.y, (inner.w as f32 * frac.clamp(0.0, 1.0)).round() as i32, inner.h)
                };
                cv.fill(f, self.v("progress.bar"));
            }
        }
    }

    pub fn scroll_track(&self, cv: &mut Canvas, r: Rect) {
        cv.fill(r, match self.d.family {
            Family::Gloss => self.v("scroll.track"),
            Family::Flat => self.v("scroll.track"),
        });
    }

    pub fn scroll_thumb(&self, cv: &mut Canvas, r: Rect, s: BtnState) {
        match self.d.family {
            Family::Gloss => {
                let (t, b) = if s.pressed || s.hot { (self.v("scroll.thumb.hot.top"), self.v("scroll.thumb.hot.bottom")) } else { (self.v("scroll.thumb.top"), self.v("scroll.thumb.bottom")) };
                cv.round_rect(r.inset(1), 2.0, t, b, Some(self.v("scroll.thumb.border")));
            }
            Family::Flat => {
                let col = if s.pressed { self.v("scroll.thumb.pressed") } else if s.hot { self.v("scroll.thumb.hot") } else { self.v("scroll.thumb") };
                cv.fill(r.inset(1), col);
            }
        }
    }

    // ---- menus, hints, list selection

    pub fn menu_bg(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("menu.bg"),
            Family::Flat => self.v("menu.bg"),
        }
    }
    pub fn menu_border(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("menu.border"),
            Family::Flat => self.v("menu.border"),
        }
    }
    pub fn menu_hot(&self, cv: &mut Canvas, r: Rect) {
        match self.d.family {
            Family::Gloss => cv.round_rect(r, 2.0, self.v("menu.hot.top"), self.v("menu.hot.bottom"), None),
            Family::Flat => cv.fill(r, self.v("menu.hot")),
        }
    }
    pub fn menu_hot_text(&self) -> u32 {
        match self.d.family {
            Family::Gloss => self.v("menu.hot.text"),
            Family::Flat => self.v("menu.hot.text"),
        }
    }

    pub fn selection(&self, cv: &mut Canvas, r: Rect, focused: bool) {
        let col = if focused || self.glossy() { self.highlight() } else { self.v("selection.unfocused") };
        cv.fill(r, col);
    }
    pub fn selection_text(&self, focused: bool) -> u32 {
        if focused || self.glossy() {
            self.highlight_text()
        } else {
            self.v("selection.unfocused.text")
        }
    }

    // ---- window frame

    pub fn caption_height(&self) -> i32 {
        self.v("caption.height") as i32
    }
    pub fn tool_caption_height(&self) -> i32 {
        self.v("caption.tool.height") as i32
    }

    /// Title bar background.
    pub fn caption(&self, cv: &mut Canvas, r: Rect, active: bool) {
        match self.d.family {
            Family::Gloss => {
                let (t, m, b) = if active { (self.v("caption.active.top"), self.v("caption.active.mid"), self.v("caption.active.bottom")) } else { (self.v("caption.inactive.top"), self.v("caption.inactive.mid"), self.v("caption.inactive.bottom")) };
                let half = r.h / 2;
                cv.gradient(Rect::new(r.x, r.y, r.w, half), t, m);
                cv.gradient(Rect::new(r.x, r.y + half, r.w, r.h - half), mix(m, b, 0.6), b);
                cv.hline(r.x, r.right(), r.bottom() - 1, self.v("caption.line"));
            }
            Family::Flat => {
                cv.fill(r, if active { self.v("caption.active") } else { self.v("caption.inactive") });
                cv.hline(r.x, r.right(), r.bottom() - 1, self.v("caption.line"));
            }
        }
    }
    pub fn caption_text(&self, active: bool) -> u32 {
        match self.d.family {
            Family::Gloss => {
                if active {
                    self.v("caption.active.text")
                } else {
                    self.v("caption.inactive.text")
                }
            }
            Family::Flat => {
                if active {
                    self.v("caption.active.text")
                } else {
                    self.v("caption.inactive.text")
                }
            }
        }
    }

    /// Window border colour (around the client area).
    pub fn window_border(&self, active: bool) -> u32 {
        match self.d.family {
            Family::Gloss => {
                if active {
                    self.v("window.border.active")
                } else {
                    self.v("window.border.inactive")
                }
            }
            Family::Flat => {
                if active {
                    self.v("window.border.active")
                } else {
                    self.v("window.border.inactive")
                }
            }
        }
    }

    /// Title bar buttons: 0 close, 1 maximise, 2 minimise.
    pub fn caption_button(&self, cv: &mut Canvas, r: Rect, kind: u8, hot: bool, pressed: bool) {
        match self.d.family {
            Family::Gloss => {
                let d = r.w.min(r.h) - 4;
                let cr = Rect::new(r.x + (r.w - d) / 2, r.y + (r.h - d) / 2, d, d);
                let fill = if pressed { self.v("caption.button.pressed") } else if hot { if kind == 0 { self.v("caption.button.close.hot") } else { self.v("caption.button.hot") } } else { self.v("caption.button") };
                cv.ellipse(cr, Some(fill), Some(self.v("caption.button.border")));
                let col = self.v("caption.button.glyph");
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
            Family::Flat => {
                if hot || pressed {
                    cv.fill(r, if kind == 0 { self.v("caption.button.close.hot") } else { self.v("caption.button.hot") });
                }
                let col = if kind == 0 && (hot || pressed) { self.v("caption.button.close.hot.glyph") } else { self.v("caption.button.glyph") };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every built-in theme reads, and has every value the drawing code
    /// asks for (a missing one would be drawn magenta).
    #[test]
    fn built_in_themes() {
        let all = Style::all();
        assert_eq!(all.len(), BUILT_IN.len());
        assert_eq!(Style::from_name("glossy"), Some(Style::Glossy));
        assert_eq!(Style::from_name("Windows"), Some(Style::Windows));
        let src = include_str!("theme.rs").to_string() + include_str!("paint.rs");
        let mut names: Vec<&str> = src.match_indices(".v(\"").map(|(i, m)| { let r = &src[i + m.len()..]; &r[..r.find('"').unwrap()] }).collect();
        names.sort();
        names.dedup();
        assert!(names.len() > 100);
        // the two families' built-in themes have them all between them
        let (g, w) = (data(Style::Glossy), data(Style::Windows));
        for n in &names {
            assert!(g.vals.contains_key(*n) || w.vals.contains_key(*n), "no theme has {n}");
        }
        // and every theme all of its family's
        for st in all {
            let d = data(st);
            let base = if d.family == Family::Gloss { g } else { w };
            for n in base.vals.keys() {
                assert!(d.vals.contains_key(n), "theme {} has no value {n}", d.name);
            }
        }
    }

    #[test]
    fn theme_files() {
        let known: Vec<&'static ThemeData> = registry().clone();
        let t = parse("<?xml version=\"1.0\"?>\n<!-- a test -->\n<theme name=\"T\" family=\"flat\" base=\"Windows\">\n  <color name=\"face\" value=\"#102030\"/>\n  <size name=\"caption.height\" value=\"28\"/>\n</theme>", &known).unwrap();
        assert_eq!(t.family, Family::Flat);
        assert_eq!(t.vals["face"], 0xFF10_2030);
        assert_eq!(t.vals["caption.height"], 28);
        assert_eq!(t.vals["text"], data(Style::Windows).vals["text"]);
        assert!(parse("<theme name=\"X\" family=\"round\"/>", &known).is_err());
        assert!(parse("<theme name=\"X\" family=\"gloss\"><color name=\"face\" value=\"red\"/></theme>", &known).is_err());
    }
}
