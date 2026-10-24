//! Text rendering: the VCL fonts mapped to embedded TrueType faces
//! (Liberation Sans / Mono, metric compatible with Arial / Courier New; on
//! Windows Tahoma is used when installed, as MB3D's styles do).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Sans,
    SansBold,
    Mono,
}

/// A font as the form files describe it.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    pub name: String,
    /// VCL `Font.Height`: negative = character height in pixels, positive =
    /// cell height
    pub height: i32,
    pub color: u32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// the colour was set explicitly (not a system colour)
    pub custom_color: bool,
}

impl Default for Font {
    fn default() -> Self {
        Font { name: "MS Sans Serif".into(), height: -11, color: 0, bold: false, italic: false, underline: false, custom_color: false }
    }
}

impl Font {
    pub fn face(&self) -> Face {
        let n = self.name.to_ascii_lowercase();
        if n.contains("courier") || n.contains("consol") || n.contains("mono") || n.contains("fixedsys") || n.contains("terminal") {
            Face::Mono
        } else if self.bold {
            Face::SansBold
        } else {
            Face::Sans
        }
    }

    /// VCL `TextHeight` in logical pixels.
    pub fn line_height(&self) -> i32 {
        let n = self.name.to_ascii_lowercase();
        let h = if (n == "ms sans serif" || n == "ms serif") && self.height == -12 { 11 } else { self.height.abs() };
        if self.height > 0 { self.height } else { (h as f32 * 1.18).round() as i32 }
    }

    /// Em size in logical pixels.
    pub fn em(&self) -> f32 {
        let n = self.name.to_ascii_lowercase();
        if (n == "ms sans serif" || n == "ms serif") && self.height == -12 {
            // the bitmap font has no 12 pixel size: Windows uses the 8 pt one
            return if cfg!(windows) { 11.0 } else { 10.25 };
        }
        // the embedded Liberation Sans is wider than Tahoma / MS Sans Serif
        let k = if cfg!(windows) || self.face() == Face::Mono { 1.0 } else { 0.93 };
        if self.height < 0 {
            -self.height as f32 * k
        } else if self.height > 0 {
            self.height as f32 * 0.86
        } else {
            11.0
        }
    }
}

const SANS: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Regular.ttf");
const SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Bold.ttf");
const MONO: &[u8] = include_bytes!("../../assets/fonts/LiberationMono-Regular.ttf");

pub struct Glyph {
    pub w: usize,
    pub h: usize,
    /// offset of the bitmap from the pen position (x) and the baseline (y, down)
    pub ox: i32,
    pub oy: i32,
    pub advance: f32,
    pub cov: Vec<u8>,
}

struct Fonts {
    faces: [fontdue::Font; 3],
    glyphs: HashMap<(Face, u32, char), std::rc::Rc<Glyph>>,
    /// the change count of [`FALLBACKS`] the cache was filled under
    fallback_gen: u64,
}

/// Fonts with the glyphs the embedded faces lack (Japanese, Chinese,
/// Korean ...), named by the language files (see `lang`), the current
/// language's first; `.0` counts the changes, the glyph caches follow.
static FALLBACKS: RwLock<(u64, Vec<Arc<fontdue::Font>>)> = RwLock::new((0, Vec::new()));

/// Reads a font file (.ttf, .otf; of a collection the first font).
pub fn load_font(data: Vec<u8>) -> Option<Arc<fontdue::Font>> {
    fontdue::Font::from_bytes(data, fontdue::FontSettings::default()).ok().map(Arc::new)
}

/// Sets the fallback fonts, in the order they are searched.
pub fn set_fallbacks(fonts: Vec<Arc<fontdue::Font>>) {
    let mut w = FALLBACKS.write().unwrap();
    w.0 += 1;
    w.1 = fonts;
}

fn system_face(names: &[&str]) -> Option<fontdue::Font> {
    if !cfg!(windows) {
        return None;
    }
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    names.iter().find_map(|n| {
        let d = std::fs::read(format!("{dir}\\Fonts\\{n}")).ok()?;
        fontdue::Font::from_bytes(d, fontdue::FontSettings::default()).ok()
    })
}

thread_local! {
    static FONTS: RefCell<Fonts> = RefCell::new({
        let load = |d: &[u8]| fontdue::Font::from_bytes(d, fontdue::FontSettings::default()).expect("embedded font");
        Fonts {
            faces: [
                system_face(&["tahoma.ttf"]).unwrap_or_else(|| load(SANS)),
                system_face(&["tahomabd.ttf"]).unwrap_or_else(|| load(SANS_BOLD)),
                load(MONO),
            ],
            glyphs: HashMap::new(),
            fallback_gen: 0,
        }
    });
}

fn idx(f: Face) -> usize {
    match f {
        Face::Sans => 0,
        Face::SansBold => 1,
        Face::Mono => 2,
    }
}

/// The glyph of `c` at `px` device pixels (em size).
pub fn glyph(face: Face, px: f32, c: char) -> std::rc::Rc<Glyph> {
    let key = (face, (px * 16.0).round() as u32, c);
    FONTS.with(|f| {
        let mut f = f.borrow_mut();
        let fallbacks = FALLBACKS.read().unwrap();
        if f.fallback_gen != fallbacks.0 {
            f.glyphs.clear();
            f.fallback_gen = fallbacks.0;
        }
        if let Some(g) = f.glyphs.get(&key) {
            return g.clone();
        }
        let font = &f.faces[idx(face)];
        let (m, cov) = if font.lookup_glyph_index(c) == 0 && c != ' ' {
            match fallbacks.1.iter().find(|fb| fb.lookup_glyph_index(c) != 0) {
                Some(fb) => fb.rasterize(c, px),
                None => font.rasterize('?', px),
            }
        } else {
            font.rasterize(c, px)
        };
        let g = std::rc::Rc::new(Glyph {
            w: m.width,
            h: m.height,
            ox: m.xmin,
            oy: -(m.ymin + m.height as i32),
            advance: m.advance_width,
            cov,
        });
        if f.glyphs.len() > 20000 {
            f.glyphs.clear();
        }
        f.glyphs.insert(key, g.clone());
        g
    })
}

/// Ascent and descent (positive) at `px`.
pub fn metrics(face: Face, px: f32) -> (f32, f32) {
    FONTS.with(|f| {
        let f = f.borrow();
        match f.faces[idx(face)].horizontal_line_metrics(px) {
            Some(m) => (m.ascent, -m.descent),
            None => (px * 0.9, px * 0.2),
        }
    })
}

/// Width of `text` in device pixels (pen positions are rounded per glyph,
/// as they are drawn).
pub fn text_width(face: Face, px: f32, text: &str) -> f32 {
    let mut x = 0.0f32;
    for c in text.chars() {
        x += glyph(face, px, c).advance;
    }
    x.round()
}
