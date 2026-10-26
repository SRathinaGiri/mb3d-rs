//! The interface language.  The texts of the controls (captions, hints,
//! list items, tabs) come from the form files in English; a language file
//! of the `Languages` folder (`English.lng`, `Deutsch.lng`, ...) gives them
//! in another language, one text per line:
//!
//! ```text
//! language = Deutsch
//! code = DE
//!
//! [Mand3DForm]
//! Button2.Caption = 3D berechnen
//! Button2.Hint = Startet die Berechnung des Bildes.
//!
//! [Strings]
//! Stop = Anhalten
//! ```
//!
//! A section is a form (its object name), a line is a control's property
//! (`Items.1` is the first list item); `[Strings]` holds the texts the
//! program sets while it runs, with the English text on the left (see
//! [`tr`]).  A text that is missing stays English.  `\n` is a line break,
//! `\\` a backslash, `\s` a space at the start or end, `\=` an equals sign
//! in a `[Strings]` key.  The English file is the complete list, generated
//! from the form files and the program (`cargo run --example langfile`); a
//! new language starts as a copy of it.

use super::control::{Control, Kind};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// A language as read from its file.
#[derive(Debug, Default)]
pub struct Language {
    /// how the language calls itself (`language =`), the name of the file
    pub name: String,
    /// the short form shown in the title bar (`code =`), e.g. `EN`
    pub code: String,
    /// the file it was read from (none: the built-in English)
    pub file: Option<PathBuf>,
    /// a font file with the glyphs the program's fonts lack (`font =`,
    /// next to the language file): Japanese, Chinese, Korean ...
    pub font: Option<PathBuf>,
    /// the language's flag: `<name>.png` (or .bmp) next to the file
    pub flag: Option<Arc<super::bitmap::Bitmap>>,
    /// the form texts by `Form.Control.Property`
    texts: HashMap<String, String>,
    /// `[Strings]`: English text -> text
    strings: HashMap<String, String>,
    /// `[Strings]` the other way round
    reverse: HashMap<String, String>,
}

impl Language {
    /// The built-in English: no file, every text as the program has it.
    pub fn english() -> Language {
        Language { name: "English".into(), code: "EN".into(), ..Default::default() }
    }

    /// Reads a language file's text.
    pub fn parse(text: &str, file: Option<&Path>) -> Result<Language, String> {
        let mut l = Language { file: file.map(|p| p.to_path_buf()), ..Default::default() };
        let mut section = String::new();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim_start_matches('\u{feff}');
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
                continue;
            }
            if t.starts_with('[') && t.ends_with(']') {
                section = t[1..t.len() - 1].trim().to_string();
                continue;
            }
            let Some(eq) = split_at_equals(t) else {
                return Err(format!("line {}: expected 'key = text'", n + 1));
            };
            let key = decode(t[..eq].trim());
            let mut val = &t[eq + 1..];
            if let Some(v) = val.strip_prefix(' ') {
                val = v;
            }
            let val = decode(val);
            if section.is_empty() {
                match key.to_ascii_lowercase().as_str() {
                    "language" => l.name = val,
                    "code" => l.code = val,
                    "font" if !val.trim().is_empty() => l.font = Some(file.and_then(Path::parent).map(|d| d.join(val.trim())).unwrap_or_else(|| PathBuf::from(val.trim()))),
                    _ => {}
                }
            } else if section.eq_ignore_ascii_case("Strings") {
                l.reverse.insert(val.clone(), key.clone());
                l.strings.insert(key, val);
            } else {
                l.texts.insert(format!("{section}.{key}"), val);
            }
        }
        if l.name.is_empty() {
            l.name = file.and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "?".into());
        }
        if l.code.is_empty() {
            l.code = code_of(&l.name);
        }
        if let Some(p) = file {
            l.flag = ["png", "bmp"].iter().find_map(|ext| std::fs::read(p.with_extension(ext)).ok()).and_then(|d| super::bitmap::decode_any(&d)).map(Arc::new);
        }
        Ok(l)
    }

    /// The text of a control's property, if the file has it.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.texts.get(key).map(String::as_str)
    }

    /// A text the program sets while it runs, if the file has it.
    pub fn tr<'a>(&'a self, english: &'a str) -> &'a str {
        self.strings.get(english).map(String::as_str).unwrap_or(english)
    }

    /// The English of a text of `[Strings]` (for changing the language
    /// while the program runs).
    pub fn english_of(&self, text: &str) -> Option<&str> {
        self.reverse.get(text).map(String::as_str)
    }

    /// The keys of the form texts the file has (`Form.Control.Property`).
    pub fn keys(&self) -> Vec<&str> {
        self.texts.keys().map(String::as_str).collect()
    }

    /// The English texts the `[Strings]` section lists.
    pub fn strings(&self) -> Vec<String> {
        let mut v: Vec<String> = self.strings.keys().cloned().collect();
        v.sort();
        v
    }
}

/// The `code =` for a language without one: its first two letters.
pub fn code_of(name: &str) -> String {
    name.chars().filter(|c| c.is_alphabetic()).take(2).flat_map(char::to_uppercase).collect()
}

/// The first `=` that is not escaped.
fn split_at_equals(t: &str) -> Option<usize> {
    let b = t.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'=' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

/// Undoes [`encode`].
pub fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('s') => out.push(' '),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

/// A text as one line of a language file: line breaks as `\n`, spaces at
/// the ends as `\s`; in a key (`key`) also `\=`, `\#` and `\[` at the start.
pub fn encode(s: &str, key: bool) -> String {
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(s.len() + 4);
    let chars: Vec<char> = s.chars().collect();
    let lead = chars.iter().take_while(|c| **c == ' ').count();
    let trail = if lead == chars.len() { 0 } else { chars.iter().rev().take_while(|c| **c == ' ').count() };
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            ' ' if i < lead || i >= chars.len() - trail => out.push_str("\\s"),
            '=' if key => out.push_str("\\="),
            '#' | ';' | '[' if key && i == 0 => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// Whether a text of a form file is one to translate: two letters in a
/// row (not `-`, `0.5`, `X:`, `1e-5`).
pub fn wanted(s: &str) -> bool {
    let mut prev = false;
    for c in s.chars() {
        let a = c.is_alphabetic();
        if a && prev {
            return true;
        }
        prev = a;
    }
    false
}

/// Every text of a control that a language file can give, with its key
/// (`Form.Control.Property`; the form's own caption is `Form.Caption`).
pub fn visit_texts(form: &str, root: bool, c: &mut Control, mut f: impl FnMut(&str, &mut String)) {
    let prefix = if root { form.to_string() } else { format!("{form}.{}", c.name) };
    if c.kind == Kind::Timer || c.kind == Kind::ImageList || c.kind == Kind::Dialog {
        return;
    }
    if wanted(&c.caption) {
        f(&format!("{prefix}.Caption"), &mut c.caption);
    }
    if wanted(&c.hint) {
        f(&format!("{prefix}.Hint"), &mut c.hint);
    }
    for (i, it) in c.items.iter_mut().enumerate() {
        if wanted(it) {
            f(&format!("{prefix}.Items.{}", i + 1), it);
        }
    }
    for (i, it) in c.tabs.iter_mut().enumerate() {
        if wanted(it) {
            f(&format!("{prefix}.Tabs.{}", i + 1), it);
        }
    }
    for (i, col) in c.cols.iter_mut().enumerate() {
        if wanted(&col.caption) {
            f(&format!("{prefix}.Columns.{}", i + 1), &mut col.caption);
        }
    }
    // a memo's text from the form file (help texts), line by line
    if c.kind == Kind::Memo && wanted(&c.text) {
        let mut lines: Vec<String> = c.text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
        let mut changed = false;
        for (i, l) in lines.iter_mut().enumerate() {
            if wanted(l) {
                let before = l.clone();
                f(&format!("{prefix}.Lines.{}", i + 1), l);
                changed |= *l != before;
            }
        }
        if changed {
            c.text = lines.join("\n");
        }
    }
}

// ---------------------------------------------------------------------------
// the registry and the current language

static REGISTRY: RwLock<Vec<Arc<Language>>> = RwLock::new(Vec::new());
static CURRENT: RwLock<Option<Arc<Language>>> = RwLock::new(None);
/// the English texts of the forms, as they were built (for changing the
/// language while the program runs)
static ORIGINALS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// The shipped `Languages` folder: next to the program, else a system-wide
/// installation's, else (run from the build folder) the source tree's.
pub fn languages_dir() -> PathBuf {
    let d = crate::appdirs::app_folder().join("Languages");
    if d.is_dir() {
        return d;
    }
    if let Some(s) = crate::appdirs::system_data_dirs().into_iter().map(|s| s.join("Languages")).find(|s| s.is_dir()) {
        return s;
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("Languages");
    if src.is_dir() { src } else { d }
}

/// The user's own `Languages` folder (`~/.local/share/mb3d-rs/Languages`).
pub fn user_languages_dir() -> Option<PathBuf> {
    crate::appdirs::user_data_dir().map(|d| d.join("Languages"))
}

/// Where a new language file goes: the program's `Languages` folder when
/// the program runs from one folder (and it can be written), else the
/// user's own.
pub fn new_file_dir() -> PathBuf {
    let shipped = languages_dir();
    if crate::appdirs::portable() && std::fs::create_dir_all(&shipped).is_ok() {
        let probe = shipped.join(format!(".write-test-{}", std::process::id()));
        if std::fs::write(&probe, b"").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return shipped;
        }
    }
    user_languages_dir().unwrap_or(shipped)
}

fn language_dirs() -> Vec<PathBuf> {
    let mut v = vec![languages_dir()];
    if let Some(u) = user_languages_dir().filter(|u| u.is_dir()) {
        if !v.contains(&u) {
            v.push(u);
        }
    }
    v
}

/// The languages of the `.lng` files in `dirs` (a file of the same name
/// in a later folder replaces the earlier one), English first, then by
/// name; with the files that did not read.
pub fn load_dirs(dirs: &[PathBuf]) -> (Vec<Arc<Language>>, Vec<String>) {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x.eq_ignore_ascii_case("lng")).unwrap_or(false) {
                let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
                files.retain(|(s, _)| *s != stem);
                files.push((stem, p));
            }
        }
    }
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for (_, p) in files {
        match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|t| Language::parse(&t, Some(&p))) {
            Ok(l) => out.push(Arc::new(l)),
            Err(e) => errors.push(format!("{}: {e}", p.display())),
        }
    }
    out.sort_by(|a, b| b.name.eq_ignore_ascii_case("English").cmp(&a.name.eq_ignore_ascii_case("English")).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    (out, errors)
}

fn registry() -> Vec<Arc<Language>> {
    {
        let r = REGISTRY.read().unwrap();
        if !r.is_empty() {
            return r.clone();
        }
    }
    reload();
    REGISTRY.read().unwrap().clone()
}

/// Reads the language files again (the current language is kept by name,
/// with its new texts).
pub fn reload() {
    let (mut langs, errors) = load_dirs(&language_dirs());
    for e in errors {
        eprintln!("language {e}");
    }
    if !langs.iter().any(|l| l.name.eq_ignore_ascii_case("English")) {
        langs.insert(0, Arc::new(Language::english()));
    }
    *REGISTRY.write().unwrap() = langs;
    let name = current().name.clone();
    if !select(&name) {
        apply_fonts();
    }
}

/// The fonts of the language files (`font =`) as the fallbacks of the
/// program's fonts, the current language's first (the languages share
/// many characters, drawn in the shapes of the language in use).
fn apply_fonts() {
    static LOADED: RwLock<Option<HashMap<PathBuf, Option<Arc<super::font::FontFile>>>>> = RwLock::new(None);
    let cur = current();
    let mut order = vec![cur.clone()];
    order.extend(registry().into_iter().filter(|l| l.name != cur.name));
    let mut loaded = LOADED.write().unwrap();
    let loaded = loaded.get_or_insert_with(HashMap::new);
    let mut fonts = Vec::new();
    for l in order {
        let Some(p) = &l.font else { continue };
        let f = loaded.entry(p.clone()).or_insert_with(|| {
            let f = std::fs::read(p).ok().and_then(super::font::load_font);
            if f.is_none() {
                eprintln!("language {}: cannot read the font {}", l.name, p.display());
            }
            f
        });
        if let Some(f) = f {
            fonts.push(f.clone());
        }
    }
    super::font::set_fallbacks(fonts);
}

/// All languages: English first, then by name.
pub fn all() -> Vec<Arc<Language>> {
    registry()
}

/// The language in use (English until one is selected).
pub fn current() -> Arc<Language> {
    if let Some(l) = CURRENT.read().unwrap().as_ref() {
        return l.clone();
    }
    registry().into_iter().find(|l| l.name.eq_ignore_ascii_case("English")).unwrap_or_else(|| Arc::new(Language::english()))
}

/// Makes `name` the language the forms are built in (and [`tr`] answers
/// in); false if there is no such language file.
pub fn select(name: &str) -> bool {
    let Some(l) = registry().into_iter().find(|l| l.name.eq_ignore_ascii_case(name.trim())) else { return false };
    *CURRENT.write().unwrap() = Some(l);
    apply_fonts();
    true
}

/// The text of a form's control as the current language has it; `english`
/// (the form file's text) is remembered as the original and returned when
/// the language has no text for `key`.
pub fn text(key: &str, english: &str) -> String {
    ORIGINALS.write().unwrap().get_or_insert_with(HashMap::new).entry(key.to_string()).or_insert_with(|| english.to_string());
    current().get(key).map(str::to_string).unwrap_or_else(|| english.to_string())
}

/// The form file's text of a control property built with [`text`].
pub fn original(key: &str) -> Option<String> {
    ORIGINALS.read().unwrap().as_ref().and_then(|m| m.get(key).cloned())
}

/// A text the program shows, in the current language: `tr("Stop")`.  The
/// English is the key of the `[Strings]` section of the language files.
pub fn tr(english: &str) -> String {
    current().tr(english).to_string()
}

/// [`tr`] with values filled in for `{}` (in order) or `{0}`, `{1}` ...
/// (a translation may change their order): `trf("{} of {}", &[&a, &b])`.
pub fn trf(english: &str, args: &[&dyn std::fmt::Display]) -> String {
    let t = tr(english);
    let mut out = String::with_capacity(t.len() + 16);
    let mut next = 0;
    let mut rest = t.as_str();
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) if after[..j].chars().all(|c| c.is_ascii_digit()) => {
                let k = if j == 0 { next } else { after[..j].parse().unwrap_or(next) };
                next = k + 1;
                if let Some(a) = args.get(k) {
                    out.push_str(&a.to_string());
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Translates a control just built from its form file.
pub fn translate_new(form: &str, root: bool, c: &mut Control) {
    visit_texts(form, root, c, |key, t| *t = text(key, t));
}

/// Changes a control's texts from language `old` to `new`: a text the
/// program changed since (a number, a file name) is left as it is.
pub fn retranslate(form: &str, root: bool, c: &mut Control, old: &Language, new: &Language) {
    visit_texts(form, root, c, |key, t| {
        if let Some(orig) = original(key) {
            let was = old.get(key).unwrap_or(&orig);
            if *t == was {
                *t = new.get(key).unwrap_or(&orig).to_string();
                return;
            }
        }
        if let Some(en) = old.english_of(t) {
            *t = new.tr(en).to_string();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode() {
        for s in ["plain", "two\nlines", " lead and trail ", "a\\b", "x = y", "#hash", "  ", ""] {
            assert_eq!(decode(&encode(s, true)), s, "{s:?}");
            assert_eq!(decode(&encode(s, false)), s, "{s:?}");
        }
        assert_eq!(encode("a\r\nb", false), "a\\nb");
        assert_eq!(encode("x = y", true), "x \\= y");
        assert_eq!(encode("x = y", false), "x = y");
    }

    #[test]
    fn parse_file() {
        let t = "# comment\nlanguage = Test\n\n[Mand3DForm]\nCaption = Main\nButton2.Caption = Go\nEdit1.Hint = line one\\nline two\n\n[Strings]\nStop = Halt\nx \\= y = eq\n";
        let l = Language::parse(t, None).unwrap();
        assert_eq!((l.name.as_str(), l.code.as_str()), ("Test", "TE"));
        assert_eq!(l.get("Mand3DForm.Caption"), Some("Main"));
        assert_eq!(l.get("Mand3DForm.Button2.Caption"), Some("Go"));
        assert_eq!(l.get("Mand3DForm.Edit1.Hint"), Some("line one\nline two"));
        assert_eq!(l.tr("Stop"), "Halt");
        assert_eq!(l.tr("x = y"), "eq");
        assert_eq!(l.tr("Other"), "Other");
        assert_eq!(l.english_of("Halt"), Some("Stop"));
        assert!(Language::parse("[A]\nno equals here\n", None).is_err());
    }

    #[test]
    fn wanted_texts() {
        assert!(wanted("Calculate 3D") && wanted("Ds:") && wanted("&File"));
        assert!(!wanted("-") && !wanted("0.5") && !wanted("X:") && !wanted("1e-5") && !wanted(""));
    }

    #[test]
    fn fill_in() {
        let s = String::from("{} of {}");
        let (a, b) = (3, 7);
        assert_eq!(trf(&s, &[&a, &b]), "3 of 7");
        let r = String::from("{1} / {0}");
        assert_eq!(trf(&r, &[&a, &b]), "7 / 3");
        let x = String::from("{x} {}");
        assert_eq!(trf(&x, &[&a]), "{x} 3");
    }

    #[test]
    fn retranslation() {
        let mut c = Control::new("Button2", "TButton");
        c.caption = "Calculate 3D".into();
        translate_new("TestForm", false, &mut c);
        let de = Language::parse("language = Deutsch\n[TestForm]\nButton2.Caption = 3D berechnen\n[Strings]\nStop = Anhalten\n", None).unwrap();
        let en = Language::english();
        retranslate("TestForm", false, &mut c, &en, &de);
        assert_eq!(c.caption, "3D berechnen");
        retranslate("TestForm", false, &mut c, &de, &en);
        assert_eq!(c.caption, "Calculate 3D");
        // a text the program changed stays
        c.caption = "17".into();
        retranslate("TestForm", false, &mut c, &en, &de);
        assert_eq!(c.caption, "17");
        // a runtime text follows [Strings]
        c.caption = "Stop".into();
        let en2 = Language::parse("language = English\n[Strings]\nStop = Stop\n", None).unwrap();
        retranslate("TestForm", false, &mut c, &en2, &de);
        assert_eq!(c.caption, "Anhalten");
        retranslate("TestForm", false, &mut c, &de, &en2);
        assert_eq!(c.caption, "Stop");
    }
}
