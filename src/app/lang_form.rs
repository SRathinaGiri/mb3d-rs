//! The interface language window (LanguageGUI.dfm, new in this port): the
//! languages of the `Languages` folder to choose from, and a new language
//! file with the English texts for the user to translate.  Also the
//! generator of the English file, the complete list of the texts
//! (`cargo run --example langfile`).

use super::Mb3d;
use crate::vcl::lang::{self, encode, tr, trf};
use crate::vcl::theme::Theme;
use crate::vcl::{Event, Ui};
use std::path::Path;

const LF: &str = "LanguageFrm";

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => refresh(ui),
        "LanguagesListClick" => {
            let name = ui.c(LF, "LanguagesList").item_text();
            if !name.is_empty() && ui.set_language(&name) {
                app.ini.set_extra("Language", &name);
                super::main_form::set_language_badge(ui);
                info(ui);
            }
        }
        "CreateBtnClick" => create(ui),
        "OpenFolderBtnClick" => {
            let d = lang::new_file_dir();
            let _ = std::fs::create_dir_all(&d);
            let _ = open::that_detached(d);
        }
        "ReloadBtnClick" => {
            ui.reload_languages();
            super::main_form::set_language_badge(ui);
            refresh(ui);
        }
        "SaveAndExitBtnClick" => {
            let _ = app.ini.save();
            ui.hide(LF);
        }
        _ => {}
    }
}

/// The list of the languages, the current one selected.
fn refresh(ui: &mut Ui) {
    let all = lang::all();
    let cur = lang::current();
    let i = all.iter().position(|l| l.name == cur.name).map(|i| i as i32).unwrap_or(-1);
    ui.set_items(LF, "LanguagesList", all.iter().map(|l| l.name.clone()).collect());
    ui.set_item_images(LF, "LanguagesList", all.iter().map(|l| l.flag.as_ref().map(|b| (**b).clone()).unwrap_or_default()).collect());
    ui.set_item_index(LF, "LanguagesList", i);
    info(ui);
}

/// The current language's code and file, and the folder of the files.
fn info(ui: &mut Ui) {
    let l = lang::current();
    let file = match &l.file {
        Some(p) => p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        None => tr("built in"),
    };
    ui.set_caption(LF, "InfoLabel", &trf("Code: {}\nFile: {}", &[&l.code, &file]));
    let dir = l.file.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(lang::new_file_dir);
    ui.set_text(LF, "FolderEdit", &dir.display().to_string());
}

/// Writes a new language file with the English texts.
fn create(ui: &mut Ui) {
    let name = ui.text(LF, "NameEdit").trim().to_string();
    if name.is_empty() || name.contains(|c: char| "/\\:*?\"<>|".contains(c)) {
        ui.set_caption(LF, "InfoLabel", &tr("Type the name of the new language first; it is the name of its file."));
        return;
    }
    let dir = lang::new_file_dir();
    let path = dir.join(format!("{name}.lng"));
    if path.exists() {
        ui.set_caption(LF, "InfoLabel", &trf("There is a file {} already.", &[&path.display()]));
        return;
    }
    // the English file has every text; without it, the forms' texts
    let strings = lang::all().iter().find(|l| l.name.eq_ignore_ascii_case("English") && l.file.is_some()).map(|l| l.strings()).unwrap_or_default();
    let text = template(&name, &strings, &ui.theme);
    let r = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text));
    match r {
        Ok(()) => {
            ui.reload_languages();
            refresh(ui);
            ui.set_text(LF, "FolderEdit", &dir.display().to_string());
            ui.set_caption(LF, "InfoLabel", &trf("Written: {}\nTranslate the texts right of the \"=\" with a text editor, then press Reload.", &[&path.file_name().unwrap_or_default().to_string_lossy()]));
        }
        Err(e) => ui.set_caption(LF, "InfoLabel", &trf("Cannot write {}: {}", &[&path.display(), &e])),
    }
}

/// A language file with every text of the program in English: the texts
/// of the forms the program creates ([`super::FORMS`]) and `strings`, the
/// texts the program sets while it runs (see [`scan_sources`]).
pub fn template(name: &str, strings: &[String], theme: &Theme) -> String {
    let mut t = String::new();
    t.push_str("# Mandelbulb3D interface language (UTF-8)\n#\n");
    t.push_str("# One text per line. Left of the \"=\" is where the text is used: the\n");
    t.push_str("# window ([section]), the control and its property, or in [Strings] the\n");
    t.push_str("# English text the program shows while it runs. Right of the \"=\" is the\n");
    t.push_str("# text in this language. A line left out keeps the English text.\n");
    t.push_str("# \"\\n\" is a line break, \"\\\\\" a backslash, \"\\s\" a space at the start or\n");
    t.push_str("# end of a text, \"\\=\" an equals sign left of the \"=\". \"&\" before a letter\n");
    t.push_str("# underlines it (the Alt key shortcut); {} and {0}, {1} ... are values\n");
    t.push_str("# the program fills in (a translation may change their order).\n#\n");
    t.push_str("# Your own language: click the language badge in the title bar of the\n");
    t.push_str("# main window, type the language's name and press \"Create language\n");
    t.push_str("# file\"; translate the new file with a text editor and press \"Reload\".\n");
    t.push_str("# The file's name is the language's; \"code\" is shown on the badge. A\n");
    t.push_str("# picture <name>.png next to the file is the language's flag (18 x 12).\n");
    t.push_str("# \"font = file.ttf\" names a font next to the file for characters the\n");
    t.push_str("# program's fonts lack (Japanese, Chinese, Korean ...).\n\n");
    t.push_str(&format!("language = {}\ncode = {}\n", encode(name, false), lang::code_of(name)));
    for name in super::FORMS {
        let Ok(mut f) = crate::vcl::form::Form::from_dfm(super::forms::get(name), theme) else { continue };
        let form = f.name.clone();
        let mut lines = Vec::new();
        for (i, c) in f.ctl.iter_mut().enumerate() {
            lang::visit_texts(&form, i == 0, c, |key, text| {
                let k = key.strip_prefix(&format!("{form}.")).unwrap_or(key);
                lines.push(format!("{} = {}\n", encode(k, true), encode(text, false)));
            });
        }
        if !lines.is_empty() {
            t.push_str(&format!("\n[{form}]\n"));
            t.extend(lines);
        }
    }
    if !strings.is_empty() {
        t.push_str("\n[Strings]\n");
        for s in strings {
            t.push_str(&format!("{} = {}\n", encode(s, true), encode(s, false)));
        }
    }
    t
}

/// The texts the program sets while it runs: the string literals the `tr`
/// and `trf` calls of the source files below `dir` have, sorted.
pub fn scan_sources(dir: &Path) -> Vec<String> {
    fn walk(d: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().map_or(false, |x| x == "rs") {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    scan_text(&t, out);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out.dedup();
    out
}

fn scan_text(t: &str, out: &mut Vec<String>) {
    for pat in ["tr(\"", "trf(\""] {
        for (i, _) in t.match_indices(pat) {
            // not `o.str("Caption")`, not a method `l.tr("x")`
            if t[..i].chars().next_back().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == '.') {
                continue;
            }
            if let Some(s) = string_literal(&t[i + pat.len()..]) {
                out.push(s);
            }
        }
    }
}

/// The rest of a Rust string literal after its opening quote.
fn string_literal(rest: &str) -> Option<String> {
    let mut s = String::new();
    let mut it = rest.chars();
    while let Some(c) = it.next() {
        match c {
            '"' => return Some(s),
            '\\' => match it.next()? {
                'n' => s.push('\n'),
                't' => s.push('\t'),
                '0' => s.push('\0'),
                '\n' => {
                    // a line continuation: the next line's indentation goes
                    it = it.as_str().trim_start().chars();
                }
                o => s.push(o),
            },
            c => s.push(c),
        }
    }
    None
}

/// Writes the English language file (`Languages/English.lng` of the source
/// tree): `cargo run --example langfile`.
pub fn write_english(src: &Path, out: &Path) -> std::io::Result<()> {
    let theme = Theme::new(crate::vcl::Style::default());
    std::fs::write(out, template("English", &scan_sources(src), &theme))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped English file is the current one (else run
    /// `cargo run --example langfile`).
    #[test]
    fn english_file_current() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let theme = Theme::new(crate::vcl::Style::default());
        let want = template("English", &scan_sources(&root.join("src")), &theme);
        let have = std::fs::read_to_string(root.join("Languages/English.lng")).unwrap_or_default().replace("\r\n", "\n");
        assert!(have == want, "Languages/English.lng is not current: run `cargo run --example langfile`");
        // and it reads back to the same texts
        let l = lang::Language::parse(&have, None).unwrap();
        assert_eq!((l.name.as_str(), l.code.as_str()), ("English", "EN"));
        assert_eq!(l.get("Mand3DForm.Button2.Caption"), Some("Calculate 3D"));
        assert_eq!(l.get("LanguageFrm.Caption"), Some("Interface language"));
        assert_eq!(l.tr("Cancel"), "Cancel");
        for s in scan_sources(&root.join("src")) {
            assert!(l.english_of(&s).is_some(), "{s:?} is not in the file");
        }
    }

    /// Every shipped language file reads, and has only texts the English
    /// file has (a key the forms do not know would never show).
    #[test]
    fn shipped_languages() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("Languages");
        let en = lang::Language::parse(&std::fs::read_to_string(dir.join("English.lng")).unwrap(), None).unwrap();
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().map_or(true, |x| x != "lng") {
                continue;
            }
            n += 1;
            let l = lang::Language::parse(&std::fs::read_to_string(&p).unwrap(), Some(&p)).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert!(!l.name.is_empty() && l.code.len() >= 2, "{}", p.display());
            for k in l.keys() {
                assert!(en.get(k).is_some(), "{}: {k} is not a text of the forms", p.display());
            }
            for s in l.strings() {
                assert!(en.english_of(&s).is_some(), "{}: [Strings] {s:?} is not a text of the program", p.display());
            }
        }
        assert!(n >= 2, "{n} language files");
    }

    #[test]
    fn literals() {
        let mut out = Vec::new();
        let src = "x = tr(\"A b\\nc\"); o.str(\"Caption\"); l.tr(\"no\"); &trf(\"{} of {}\", &[&a]); tr(\"q\\\"uote\")";
        scan_text(src, &mut out);
        assert_eq!(out, ["A b\nc", "q\"uote", "{} of {}"]);
    }
}
