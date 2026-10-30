//! Text rendering: the VCL fonts mapped to embedded TrueType faces
//! (Liberation Sans / Mono, metric compatible with Arial / Courier New; on
//! Windows Tahoma is used when installed, as MB3D's styles do).  Scripts
//! whose letters join and move (Devanagari, Arabic) are shaped with
//! rustybuzz in the language's font; all else is drawn a character at a
//! time. A line with right-to-left letters (Arabic, Urdu, Hebrew) is put in
//! display order by the Unicode bidirectional algorithm (unicode-bidi).

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
    /// per fallback font: the runs shaped in it, and its shaping plan
    shaped: Vec<HashMap<String, Shaped>>,
    plans: HashMap<(usize, bool), rustybuzz::ShapePlan>,
    /// the widths of the lines with shaped runs (the windows measure the
    /// same texts over and over while they lay out and paint)
    widths: HashMap<(Face, u32), HashMap<String, f32>>,
    /// the change count of [`FALLBACKS`] the cache was filled under
    fallback_gen: u64,
}

impl Fonts {
    /// Empties the caches when the fallback fonts changed.
    fn follow(&mut self, gen: u64) {
        if self.fallback_gen != gen {
            self.glyphs.clear();
            self.shaped.clear();
            self.plans.clear();
            self.widths.clear();
            self.fallback_gen = gen;
        }
    }
}

/// A font file of a language: for drawing, and parsed for shaping.
pub struct FontFile {
    font: fontdue::Font,
    face: Option<rustybuzz::Face<'static>>,
}

/// Fonts with the glyphs the embedded faces lack (Japanese, Chinese,
/// Korean, Hindi ...), named by the language files (see `lang`), the current
/// language's first; `.0` counts the changes, the glyph caches follow.
static FALLBACKS: RwLock<(u64, Vec<Arc<FontFile>>)> = RwLock::new((0, Vec::new()));

/// Reads a font file (.ttf, .otf; of a collection the first font).
pub fn load_font(data: Vec<u8>) -> Option<Arc<FontFile>> {
    let font = fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).ok()?;
    // the shaper reads the file where it is: it stays for the program's
    // life (a language's font is read once, see `lang`)
    let data: &'static [u8] = Box::leak(data.into_boxed_slice());
    Some(Arc::new(FontFile { font, face: rustybuzz::Face::from_slice(data, 0) }))
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
            shaped: Vec::new(),
            plans: HashMap::new(),
            widths: HashMap::new(),
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
        f.follow(fallbacks.0);
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

/// Letters of the Arabic script (Arabic, Urdu, Persian ...), shaped right
/// to left: their forms depend on their neighbours.
fn is_arabic(c: char) -> bool {
    matches!(c as u32, 0x0600..=0x06FF | 0x0750..=0x077F | 0x0870..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Characters written right to left (Hebrew, Arabic and the scripts near
/// them): a line with one is put in display order.
fn is_rtl(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x200F | 0x202B | 0x202E | 0x2067 | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF)
}

/// Whether `text` has a right-to-left character (and so is put in display
/// order before it is drawn).
pub fn has_rtl(text: &str) -> bool {
    text.chars().any(is_rtl)
}

/// A bracket as it looks in right-to-left text (mirrored).
fn mirrored(c: char) -> char {
    match c {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        '‹' => '›',
        '›' => '‹',
        _ => c,
    }
}

/// `run` shaped in the first fallback font that has all its letters, else
/// the first that has its first letter (its index and the glyphs, in
/// display order; none: no font has it).
/// `rtl`: an Arabic run, else Devanagari.
fn shape(run: &str, rtl: bool) -> Option<(usize, Shaped, f32)> {
    let first = run.chars().find(|&c| c != '\u{200C}' && c != '\u{200D}')?;
    FONTS.with(|f| {
        let mut f = f.borrow_mut();
        let fallbacks = FALLBACKS.read().unwrap();
        f.follow(fallbacks.0);
        // (the first font with all the run's letters, else the first with
        // its first: Urdu's font may lack letters of Arabic)
        let has = |fb: &Arc<FontFile>, c: char| fb.font.lookup_glyph_index(c) != 0;
        let i = fallbacks
            .1
            .iter()
            .position(|fb| run.chars().all(|c| c == '\u{200C}' || c == '\u{200D}' || has(fb, c)))
            .or_else(|| fallbacks.1.iter().position(|fb| has(fb, first)))?;
        let face = fallbacks.1[i].face.as_ref()?;
        let upem = face.units_per_em() as f32;
        if f.shaped.len() <= i {
            f.shaped.resize_with(i + 1, HashMap::new);
        }
        if let Some(s) = f.shaped[i].get(run) {
            return Some((i, s.clone(), upem));
        }
        let (dir, script) = if rtl {
            (rustybuzz::Direction::RightToLeft, rustybuzz::script::ARABIC)
        } else {
            (rustybuzz::Direction::LeftToRight, rustybuzz::script::DEVANAGARI)
        };
        let plan = f.plans.entry((i, rtl)).or_insert_with(|| rustybuzz::ShapePlan::new(face, dir, Some(script), None, &[]));
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(run);
        buf.set_direction(dir);
        buf.set_script(script);
        let out = rustybuzz::shape_with_plan(face, plan, buf);
        let s: Shaped = std::rc::Rc::new(
            out.glyph_infos()
                .iter()
                .zip(out.glyph_positions())
                .map(|(g, p)| (g.glyph_id as u16, p.x_advance, p.x_offset, p.y_offset, g.cluster as usize))
                .collect(),
        );
        if f.shaped[i].len() > 50000 {
            f.shaped[i].clear();
        }
        f.shaped[i].insert(run.to_string(), s.clone());
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
/// that needs shaping are shaped. A line with right-to-left letters is laid
/// out as a right-to-left paragraph, in display order (its glyphs left to
/// right on the screen).
pub fn layout(face: Face, px: f32, text: &str, x0: f32) -> (Vec<Placed>, f32) {
    let mut out = Vec::with_capacity(text.len());
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    if !has_rtl(text) {
        let pen = layout_ltr(face, px, text, &chars, 0, chars.len(), x0, &mut out);
        return (out, pen);
    }
    // (a line with right-to-left letters is a right-to-left paragraph,
    // also where it starts with a Latin word or a number: "3D حساب کریں")
    let info = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::rtl()));
    let mut pen = x0;
    for para in &info.paragraphs {
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for run in runs {
            let a = chars.partition_point(|c| c.0 < run.start);
            let b = chars.partition_point(|c| c.0 < run.end);
            pen = if levels[run.start].is_rtl() {
                layout_rtl(face, px, text, &chars, a, b, pen, &mut out)
            } else {
                layout_ltr(face, px, text, &chars, a, b, pen, &mut out)
            };
        }
    }
    (out, pen)
}

/// Lays out the characters `a..b` of `chars` (of `text`) left to right.
#[allow(clippy::too_many_arguments)]
fn layout_ltr(face: Face, px: f32, text: &str, chars: &[(usize, char)], a: usize, b: usize, mut pen: f32, out: &mut Vec<Placed>) -> f32 {
    let mut i = a;
    while i < b {
        let c = chars[i].1;
        if needs_shaping(c) || is_arabic(c) {
            let arabic = is_arabic(c);
            let mut j = i;
            while j < b && (if arabic { is_arabic(chars[j].1) } else { needs_shaping(chars[j].1) }) {
                j += 1;
            }
            if let Some(p) = place_shaped(face, px, text, chars, i, j, arabic, pen, out) {
                pen = p;
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
    pen
}

/// Lays out the characters `a..b` of a right-to-left run: the last
/// character leftmost, runs of Arabic letters shaped, brackets mirrored.
#[allow(clippy::too_many_arguments)]
fn layout_rtl(face: Face, px: f32, text: &str, chars: &[(usize, char)], a: usize, b: usize, mut pen: f32, out: &mut Vec<Placed>) -> f32 {
    let mut j = b;
    while j > a {
        let c = chars[j - 1].1;
        if is_arabic(c) || c == '\u{200C}' || c == '\u{200D}' {
            let mut i = j - 1;
            while i > a && (is_arabic(chars[i - 1].1) || matches!(chars[i - 1].1, '\u{200C}' | '\u{200D}')) {
                i -= 1;
            }
            if let Some(p) = place_shaped(face, px, text, chars, i, j, true, pen, out) {
                pen = p;
                j = i;
                continue;
            }
        }
        let g = glyph(face, px, mirrored(c));
        let adv = g.advance;
        out.push(Placed { g, x: pen, dx: 0.0, dy: 0.0, ch: j - 1 });
        pen += adv;
        j -= 1;
    }
    pen
}

/// Places the shaped characters `i..j` (the glyphs in display order); the
/// pen after them, none when no font has them.
#[allow(clippy::too_many_arguments)]
fn place_shaped(face: Face, px: f32, text: &str, chars: &[(usize, char)], i: usize, j: usize, rtl: bool, mut pen: f32, out: &mut Vec<Placed>) -> Option<f32> {
    let start = chars[i].0;
    let end = chars.get(j).map_or(text.len(), |c| c.0);
    let (fi, glyphs, upem) = shape(&text[start..end], rtl)?;
    let k = px / upem;
    for &(gid, adv, xo, yo, cluster) in glyphs.iter() {
        let ch = i + text[start..start + cluster].chars().count();
        out.push(Placed { g: glyph_of(face, px, GlyphKey::Index(fi, gid)), x: pen, dx: xo as f32 * k, dy: yo as f32 * k, ch });
        pen += adv as f32 * k;
    }
    Some(pen)
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
    if !text.chars().any(|c| needs_shaping(c) || is_rtl(c)) {
        let mut x = 0.0f32;
        for c in text.chars() {
            x += glyph(face, px, c).advance;
        }
        return x.round();
    }
    let key = (face, (px * 16.0).round() as u32);
    let known = FONTS.with(|f| {
        let mut f = f.borrow_mut();
        f.follow(FALLBACKS.read().unwrap().0);
        f.widths.get(&key).and_then(|m| m.get(text)).copied()
    });
    if let Some(w) = known {
        return w;
    }
    let w = layout(face, px, text, 0.0).1.round();
    FONTS.with(|f| {
        let mut f = f.borrow_mut();
        let m = f.widths.entry(key).or_default();
        if m.len() > 50000 {
            m.clear();
        }
        m.insert(text.to_string(), w);
    });
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (the tests set the fallback fonts, which all threads share)
    static FALLBACK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn devanagari_is_shaped() {
        let _lock = FALLBACK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

    #[test]
    fn arabic_is_shaped_right_to_left() {
        let _lock = FALLBACK_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/Languages/Arabic.ttf")).unwrap();
        set_fallbacks(vec![load_font(data).unwrap()]);
        // لا: lam and alef joined into one glyph
        assert_eq!(layout(Face::Sans, 20.0, "لا", 0.0).0.len(), 1);
        // an Arabic line: its first word rightmost, the Latin word and the
        // number after it ("abc 12", one left-to-right run) left to right
        let text = "ملف abc 12";
        let (g, w) = layout(Face::Sans, 20.0, text, 0.0);
        let x_of = |ch: usize| g.iter().find(|p| p.ch == ch).unwrap().x;
        assert!(x_of(0) > x_of(4), "the Arabic word stands right of the Latin one");
        assert!(x_of(4) < x_of(5) && x_of(5) < x_of(6), "abc left to right");
        assert!(x_of(8) < x_of(9), "12 left to right");
        assert!(x_of(6) < x_of(8), "the number right of abc");
        assert!(w > 0.0 && g.iter().all(|p| p.x >= 0.0 && p.x < w));
        // a line starting with a Latin word: still right to left, the
        // Latin word rightmost
        let (g, _) = layout(Face::Sans, 20.0, "3D ملف", 0.0);
        let x_of = |ch: usize| g.iter().find(|p| p.ch == ch).unwrap().x;
        assert!(x_of(0) > x_of(3) && x_of(0) < x_of(1));
        assert_eq!(text_width(Face::Sans, 20.0, "File"), layout(Face::Sans, 20.0, "File", 0.0).1.round());
    }
}
