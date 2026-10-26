//! Text rendering: the VCL fonts mapped to embedded TrueType faces
//! (Liberation Sans / Mono, metric compatible with Arial / Courier New; on
//! Windows Tahoma is used when installed, as MB3D's styles do).  Scripts
//! whose letters join and move (Devanagari) are shaped with rustybuzz in
//! the language's font; all else is drawn a character at a time.

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

/// A glyph of the cache: a character of a face (or of a fallback font), or
/// a glyph a shaped run names in fallback font `.0`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum GlyphKey {
    Char(char),
    Index(usize, u16),
}

/// A shaped run in font units: glyph, advance, x and y offset, and the
/// byte offset in the run of the cluster's first character.
type Shaped = std::rc::Rc<Vec<(u16, i32, i32, i32, usize)>>;

struct Fonts {
    faces: [fontdue::Font; 3],
    glyphs: HashMap<(Face, u32, GlyphKey), std::rc::Rc<Glyph>>,
    shaped: HashMap<(usize, String), Shaped>,
    /// the change count of [`FALLBACKS`] the cache was filled under
    fallback_gen: u64,
}

/// A font file of a language: for drawing, and its bytes for shaping.
pub struct FontFile {
    font: fontdue::Font,
    data: Vec<u8>,
}

/// Fonts with the glyphs the embedded faces lack (Japanese, Chinese,
/// Korean, Hindi ...), named by the language files (see `lang`), the current
/// language's first; `.0` counts the changes, the glyph caches follow.
static FALLBACKS: RwLock<(u64, Vec<Arc<FontFile>>)> = RwLock::new((0, Vec::new()));

/// Reads a font file (.ttf, .otf; of a collection the first font).
pub fn load_font(data: Vec<u8>) -> Option<Arc<FontFile>> {
    let font = fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).ok()?;
    Some(Arc::new(FontFile { font, data }))
}

/// Sets the fallback fonts, in the order they are searched.
pub fn set_fallbacks(fonts: Vec<Arc<FontFile>>) {
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
            shaped: HashMap::new(),
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
    glyph_of(face, px, GlyphKey::Char(c))
}

fn glyph_of(face: Face, px: f32, key: GlyphKey) -> std::rc::Rc<Glyph> {
    let ckey = (face, (px * 16.0).round() as u32, key);
    FONTS.with(|f| {
        let mut f = f.borrow_mut();
        let fallbacks = FALLBACKS.read().unwrap();
        if f.fallback_gen != fallbacks.0 {
            f.glyphs.clear();
            f.shaped.clear();
            f.fallback_gen = fallbacks.0;
        }
        if let Some(g) = f.glyphs.get(&ckey) {
            return g.clone();
        }
        let font = &f.faces[idx(face)];
        let (m, cov) = match key {
            GlyphKey::Char(c) if font.lookup_glyph_index(c) == 0 && c != ' ' => match fallbacks.1.iter().find(|fb| fb.font.lookup_glyph_index(c) != 0) {
                Some(fb) => fb.font.rasterize(c, px),
                None => font.rasterize('?', px),
            },
            GlyphKey::Char(c) => font.rasterize(c, px),
            GlyphKey::Index(i, g) => match fallbacks.1.get(i) {
                Some(fb) => fb.font.rasterize_indexed(g, px),
                None => font.rasterize('?', px),
            },
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
        f.glyphs.insert(ckey, g.clone());
        g
    })
}

/// Characters of the scripts that need shaping: Devanagari (with its
/// extensions and the Vedic marks), and the joiners between them.
fn needs_shaping(c: char) -> bool {
    matches!(c as u32, 0x0900..=0x097F | 0xA8E0..=0xA8FF | 0x1CD0..=0x1CFF | 0x200C | 0x200D)
}

/// `run` shaped in the first fallback font that has its first letter (its
/// index and the glyphs; none: no font has it).
fn shape(run: &str) -> Option<(usize, Shaped, f32)> {
    let first = run.chars().find(|&c| c != '\u{200C}' && c != '\u{200D}')?;
    FONTS.with(|f| {
        let mut f = f.borrow_mut();
        let fallbacks = FALLBACKS.read().unwrap();
        if f.fallback_gen != fallbacks.0 {
            f.glyphs.clear();
            f.shaped.clear();
            f.fallback_gen = fallbacks.0;
        }
        let i = fallbacks.1.iter().position(|fb| fb.font.lookup_glyph_index(first) != 0)?;
        let fb = &fallbacks.1[i];
        let face = rustybuzz::Face::from_slice(&fb.data, 0)?;
        let upem = face.units_per_em() as f32;
        if let Some(s) = f.shaped.get(&(i, run.to_string())) {
            return Some((i, s.clone(), upem));
        }
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(run);
        buf.guess_segment_properties();
        let out = rustybuzz::shape(&face, &[], buf);
        let s: Shaped = std::rc::Rc::new(
            out.glyph_infos()
                .iter()
                .zip(out.glyph_positions())
                .map(|(g, p)| (g.glyph_id as u16, p.x_advance, p.x_offset, p.y_offset, g.cluster as usize))
                .collect(),
        );
        if f.shaped.len() > 5000 {
            f.shaped.clear();
        }
        f.shaped.insert((i, run.to_string()), s.clone());
        Some((i, s, upem))
    })
}

/// A glyph placed on a line: its pen position and the offset of the
/// bitmap (device pixels, y up), and the characters it stands for (the
/// first's index; several for a joined cluster).
pub struct Placed {
    pub g: std::rc::Rc<Glyph>,
    pub x: f32,
    pub dx: f32,
    pub dy: f32,
    pub ch: usize,
}

/// The glyphs of one line of `text` at `px` with the pen starting at `x0`,
/// and the pen after the last: a glyph per character, but runs of a script
/// that needs shaping are shaped.
pub fn layout(face: Face, px: f32, text: &str, x0: f32) -> (Vec<Placed>, f32) {
    let mut out = Vec::with_capacity(text.len());
    let mut pen = x0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i].1;
        if needs_shaping(c) {
            let mut j = i;
            while j < chars.len() && needs_shaping(chars[j].1) {
                j += 1;
            }
            let start = chars[i].0;
            let end = chars.get(j).map_or(text.len(), |c| c.0);
            if let Some((fi, glyphs, upem)) = shape(&text[start..end]) {
                let k = px / upem;
                for &(gid, adv, xo, yo, cluster) in glyphs.iter() {
                    let ch = i + text[start..start + cluster].chars().count();
                    out.push(Placed { g: glyph_of(face, px, GlyphKey::Index(fi, gid)), x: pen, dx: xo as f32 * k, dy: yo as f32 * k, ch });
                    pen += adv as f32 * k;
                }
                i = j;
                continue;
            }
        }
        let g = glyph(face, px, c);
        let adv = g.advance;
        out.push(Placed { g, x: pen, dx: 0.0, dy: 0.0, ch: i });
        pen += adv;
        i += 1;
    }
    (out, pen)
}

/// The pen positions before each character of `text` and after the last
/// (n + 1 values); the characters inside a joined cluster stand at its start.
pub fn caret_xs(face: Face, px: f32, text: &str) -> Vec<f32> {
    let n = text.chars().count();
    let (glyphs, width) = layout(face, px, text, 0.0);
    let mut xs = vec![f32::NAN; n + 1];
    for g in &glyphs {
        if g.ch < n && !(xs[g.ch] <= g.x) {
            xs[g.ch] = g.x;
        }
    }
    xs[n] = width;
    for i in 0..n {
        if xs[i].is_nan() {
            xs[i] = if i == 0 { 0.0 } else { xs[i - 1] };
        }
    }
    xs
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
    if !text.chars().any(needs_shaping) {
        let mut x = 0.0f32;
        for c in text.chars() {
            x += glyph(face, px, c).advance;
        }
        return x.round();
    }
    layout(face, px, text, 0.0).1.round()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devanagari_is_shaped() {
        let data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/Languages/Hindi.ttf")).unwrap();
        set_fallbacks(vec![load_font(data).unwrap()]);
        // कि: the vowel sign i stands before the consonant it follows
        let (g, w) = layout(Face::Sans, 20.0, "कि", 0.0);
        assert_eq!(g.len(), 2);
        assert!(g.iter().all(|p| p.ch == 0) && w > 0.0);
        // क्ष: consonant, virama, consonant joined into one glyph
        assert_eq!(layout(Face::Sans, 20.0, "क्ष", 0.0).0.len(), 1);
        // Latin around it as before, the caret only at cluster boundaries
        let xs = caret_xs(Face::Sans, 20.0, "a कि b");
        assert_eq!(xs.len(), 7);
        assert_eq!(xs[2], xs[3]);
        assert!(xs.windows(2).all(|p| p[0] <= p[1]));
        assert_eq!(text_width(Face::Sans, 20.0, "ab"), (glyph(Face::Sans, 20.0, 'a').advance + glyph(Face::Sans, 20.0, 'b').advance).round());
    }
}
