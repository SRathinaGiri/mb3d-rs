//! The interface language window (LanguageGUI.dfm, new in this port): the
//! languages of the `Languages` folder to choose from, and a new language
//! file with the English labels for the user to translate (the labels:
//! `crate::vcl::lang`; English.lng is kept with `tools/labels.py`).

use super::Mb3d;
use crate::vcl::lang::{self, encode, tr, trf};
#[cfg(test)]
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
        None => tr("@MB:BuiltIn"),
    };
    ui.set_caption(LF, "InfoLabel", &trf("@MB:CodeFile", &[&l.code, &file]));
    let dir = l.file.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(lang::new_file_dir);
    ui.set_text(LF, "FolderEdit", &dir.display().to_string());
}

/// Writes a new language file with the English texts.
fn create(ui: &mut Ui) {
    let name = ui.text(LF, "NameEdit").trim().to_string();
    if name.is_empty() || name.contains(|c: char| "/\\:*?\"<>|".contains(c)) {
        ui.set_caption(LF, "InfoLabel", &tr("@MB:TypeTheNameOfTheNew"));
        return;
    }
    let dir = lang::new_file_dir();
    let path = dir.join(format!("{name}.lng"));
    if path.exists() {
        ui.set_caption(LF, "InfoLabel", &trf("@MB:ThereIsAFileAlready", &[&path.display()]));
        return;
    }
    // the English labels, every text of the program (built in)
    let text = template(&name);
    let r = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, text));
    match r {
        Ok(()) => {
            ui.reload_languages();
            refresh(ui);
            ui.set_text(LF, "FolderEdit", &dir.display().to_string());
            ui.set_caption(LF, "InfoLabel", &trf("@MB:WrittenTranslateTheTextsRightOf", &[&path.file_name().unwrap_or_default().to_string_lossy()]));
        }
        Err(e) => ui.set_caption(LF, "InfoLabel", &trf("@MB:CannotWrite", &[&path.display(), &e])),
    }
}

/// A new language's file: the English labels (built into the program, as
/// English.lng), named `name`, for translating.
pub fn template(name: &str) -> String {
    let mut out = String::new();
    for line in include_str!("../../Languages/English.lng").lines() {
        let t = line.trim_start();
        if t.starts_with("language =") || t.starts_with("language=") {
            out.push_str(&format!("language = {}\n", encode(name, false)));
        } else if t.starts_with("code =") || t.starts_with("code=") {
            out.push_str(&format!("code = {}\n", lang::code_of(name)));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The keys of the forms' control texts (`Form.Control.Property`) with
    /// their texts as the form files have them.
    fn form_texts() -> Vec<(String, String)> {
        let theme = Theme::new(crate::vcl::Style::default());
        let mut out = Vec::new();
        for name in super::super::FORMS {
            let mut f = crate::vcl::form::Form::from_dfm(super::super::forms::get(name), &theme).unwrap();
            let form = f.name.clone();
            for (i, c) in f.ctl.iter_mut().enumerate() {
                lang::visit_texts(&form, i == 0, c, |key, text| out.push((key.to_string(), text.clone())));
            }
        }
        out
    }

    /// Every text of the forms is a label reference, and every label the
    /// forms and the code refer to has an English text (else add it:
    /// `python3 tools/labels.py add "Text"`).
    #[test]
    fn labels_complete() {
        let en = lang::builtin_english();
        assert_eq!(lang::Language::english().name, "English");
        for (key, text) in form_texts() {
            let id = text.strip_prefix(lang::LABEL).unwrap_or_else(|| panic!("{key}: {text:?} is not a label (@MB:Id)"));
            assert!(en.label(id).is_some(), "{key}: label {id} is not in English.lng");
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for s in scan_sources(&root.join("src")) {
            if let Some(id) = s.strip_prefix(lang::LABEL) {
                assert!(en.label(id).is_some(), "tr(\"{s}\"): label {id} is not in English.lng");
            } else {
                assert!(en.label_of(&s).is_some(), "tr(\"{s}\") is no label and no label's English text");
            }
        }
        assert_eq!(en.tr("@MB:Cancel"), "Cancel");
        assert_eq!(en.tr("Cancel"), "Cancel");
    }

    /// Every shipped language file reads, its labels are labels of the
    /// English file, and its control-specific texts name controls the forms
    /// have.
    #[test]
    fn shipped_languages() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("Languages");
        let en = lang::builtin_english();
        let keys: std::collections::HashSet<String> = form_texts().into_iter().map(|(k, _)| k).collect();
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().map_or(true, |x| x != "lng") {
                continue;
            }
            n += 1;
            let l = lang::Language::parse(&std::fs::read_to_string(&p).unwrap(), Some(&p)).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert!(!l.name.is_empty() && l.code.len() >= 2, "{}", p.display());
            for id in l.labels() {
                assert!(en.label(id).is_some(), "{}: label {id} is not in English.lng", p.display());
            }
            for k in l.keys() {
                assert!(keys.contains(k), "{}: {k} is not a text of the forms", p.display());
            }
        }
        assert!(n >= 2, "{n} language files");
    }

    /// A new language's file is the English one under its name.
    #[test]
    fn new_language_file() {
        let t = template("Esperanto");
        let l = lang::Language::parse(&t, None).unwrap();
        assert_eq!((l.name.as_str(), l.code.as_str()), ("Esperanto", "ES"));
        assert_eq!(l.labels().len(), lang::builtin_english().labels().len());
    }

    #[test]
    fn literals() {
        let mut out = Vec::new();
        let src = "x = tr(\"A b\\nc\"); o.str(\"Caption\"); l.tr(\"no\"); &trf(\"{} of {}\", &[&a]); tr(\"q\\\"uote\")";
        scan_text(src, &mut out);
        assert_eq!(out, ["A b\nc", "q\"uote", "{} of {}"]);
    }
}
