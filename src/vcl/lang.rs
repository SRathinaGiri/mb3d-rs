//! The interface language, as labels in the way of Microsoft Dynamics 365:
//! every text of the program has a label ID, and the form files and the
//! code refer to it as `@MB:Id` (a button's `Caption = '@MB:CalculateNow'`,
//! `tr("@MB:CalculationStopped")`), so one translation serves every place
//! the text is shown.  A language file of the `Languages` folder
//! (`English.lng`, `Deutsch.lng`, ...) gives the labels' texts:
//!
//! ```text
//! language = Deutsch
//! code = DE
//!
//! [Labels]
//! CalculateNow=Jetzt berechnen
//! ;Calculate now
//!
//! [MutaGenFrm]
//! Label4.Caption = toevoegen
//! ```
//!
//! A line under a label starting with `;` is a note (the English text, or
//! in English.lng where the label is used).  A form section (its object
//! name; `Control.Property`, `Items.1` the first list item) gives one
//! control a text of its own instead of its label's, where a place needs
//! other words.  A label a language lacks shows the English text, which is
//! built into the program (English.lng), so a form never shows an ID.  Old
//! language files keep working: their form sections are such
//! control-specific texts, and their `[Strings]` (English text = text)
//! translate the texts the program passes around as English.  `\n` is a
//! line break, `\\` a backslash, `\s` a space at the start or end, `\=` an
//! equals sign in a key.  `tools/labels.py` finds labels to reuse, adds new
//! ones, checks the references and puts the files in order.

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
    /// `[Labels]`: label ID -> text
    labels: HashMap<String, String>,
    /// `[Labels]` the other way round: text -> label ID
    label_ids: HashMap<String, String>,
}

/// The prefix of a label reference.
pub const LABEL: &str = "@MB:";

/// The English labels, built into the program (English.lng as it was
/// built), the fallback of every language.
pub fn builtin_english() -> &'static Language {
    static EN: std::sync::OnceLock<Language> = std::sync::OnceLock::new();
    EN.get_or_init(|| Language::parse(include_str!("../../Languages/English.lng"), None).unwrap_or_default())
}

impl Language {
    /// The built-in English (English.lng as the program was built with it).
    pub fn english() -> Language {
        let mut l = Language::parse(include_str!("../../Languages/English.lng"), None).unwrap_or_default();
        l.name = "English".into();
        l.code = "EN".into();
        l
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
            } else if section.eq_ignore_ascii_case("Labels") {
                l.label_ids.entry(val.clone()).or_insert_with(|| key.clone());
                l.labels.insert(key, val);
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

    /// The text of a label of this language (not the English fallback).
    pub fn label(&self, id: &str) -> Option<&str> {
        self.labels.get(id).map(String::as_str)
    }

    /// The label whose text this language shows as `text`.
    pub fn label_of(&self, text: &str) -> Option<&str> {
        self.label_ids.get(text).map(String::as_str)
    }

    /// The label IDs the file has.
    pub fn labels(&self) -> Vec<&str> {
        self.labels.keys().map(String::as_str).collect()
    }

    /// A text in this language: `@MB:Id` is the label's text (the English
    /// one when this language lacks it); an English text (one the program
    /// passes around) is its label's text, or an old file's `[Strings]`
    /// translation; anything else stays as it is.
    pub fn tr(&self, s: &str) -> String {
        let en = builtin_english();
        if let Some(id) = s.strip_prefix(LABEL) {
            if let Some(t) = self.label(id) {
                return t.to_string();
            }
            // (an old file translates the label's English text in [Strings])
            return match en.label(id) {
                Some(e) => self.strings.get(e).cloned().unwrap_or_else(|| e.to_string()),
                None => s.to_string(),
            };
        }
        if let Some(t) = self.strings.get(s) {
            return t.clone();
        }
        if let Some(id) = en.label_of(s) {
            if let Some(t) = self.label(id) {
                return t.to_string();
            }
        }
        s.to_string()
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
        // (one letter of another script is a word: 中, 幅; one Latin
        // letter is a name or a unit: "X:", "1e-5")
        if a && (prev || !c.is_ascii()) {
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
        crate::elog!("language {e}");
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
                crate::elog!("language {}: cannot read the font {}", l.name, p.display());
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
    let l = current();
    l.get(key).map(str::to_string).unwrap_or_else(|| l.tr(english))
}

/// The form file's text of a control property built with [`text`].
pub fn original(key: &str) -> Option<String> {
    ORIGINALS.read().unwrap().as_ref().and_then(|m| m.get(key).cloned())
}

/// A text the program shows, in the current language: `tr("@MB:Stop")`,
/// a label (see the module's notes); an English text the program passes
/// around finds its label by its text.
pub fn tr(text: &str) -> String {
    current().tr(text)
}

/// [`tr`] with values filled in for `{}` (in order) or `{0}`, `{1}` ...
/// (a translation may change their order): `trf("@MB:Of", &[&a, &b])`.
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
            let was = old.get(key).map(str::to_string).unwrap_or_else(|| old.tr(&orig));
            if *t == was {
                *t = new.get(key).map(str::to_string).unwrap_or_else(|| new.tr(&orig));
                return;
            }
        }
        // a text the program set: found again by its label
        let id = old.label_of(t).or_else(|| builtin_english().label_of(t)).map(str::to_string);
        if let Some(id) = id {
            *t = new.tr(&format!("{LABEL}{id}"));
        } else if let Some(en) = old.english_of(t) {
            *t = new.tr(en);
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
        // a one character text of another script (Japanese 中, 幅:)
        assert!(wanted("中") && wanted("幅:") && wanted("高"));
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
        // labels: a form's @MB:Id, a language with and without the label
        let mut b = Control::new("Button9", "TButton");
        b.caption = "@MB:Cancel".into();
        translate_new("TestForm2", false, &mut b);
        assert_eq!(b.caption, "Cancel");
        let fr = Language::parse("language = Francais\n[Labels]\nCancel=Annuler\n", None).unwrap();
        retranslate("TestForm2", false, &mut b, &en, &fr);
        assert_eq!(b.caption, "Annuler");
        let xx = Language::parse("language = X\n[Labels]\nOther=Y\n", None).unwrap();
        retranslate("TestForm2", false, &mut b, &fr, &xx);
        assert_eq!(b.caption, "Cancel", "a missing label shows the English text");
        // a runtime text set from a label follows the language too
        b.caption = fr.tr("@MB:Cancel");
        retranslate("TestForm2", false, &mut b, &fr, &en);
        assert_eq!(b.caption, "Cancel");
    }
}
