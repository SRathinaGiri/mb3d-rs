//! The JIT formula editor (formula/JITFormulaEditGUI.pas) and its value
//! dialog (formula/ParamValueEditGUI.pas).

use super::util::fts;
use super::Mb3d;
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::path::PathBuf;

const F: &str = "JITFormulaEditorForm";
const PV: &str = "ParamValueEditFrm";

/// `TJITValueDatatype`
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Datatype {
    Int64,
    Integer,
    Double,
    Single,
}

impl Datatype {
    fn parse(s: &str) -> Option<Datatype> {
        match s.to_ascii_uppercase().as_str() {
            "INT64" => Some(Datatype::Int64),
            "INTEGER" => Some(Datatype::Integer),
            "DOUBLE" => Some(Datatype::Double),
            "SINGLE" => Some(Datatype::Single),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Datatype::Int64 => "Int64",
            Datatype::Integer => "Integer",
            Datatype::Double => "Double",
            Datatype::Single => "Single",
        }
    }
}

/// `TNameValuePair`
#[derive(Clone, Debug)]
pub struct Pair {
    pub name: String,
    pub ty: Datatype,
    pub value: f64,
}

impl Pair {
    /// `ValueToString`: integers without decimals.
    fn value_str(&self) -> String {
        if self.ty == Datatype::Int64 || self.ty == Datatype::Integer || self.value.fract() == 0.0 && self.value.abs() < 1e15 {
            format!("{}", self.value as i64)
        } else {
            fts(self.value)
        }
    }
}

/// `TJITFormula`
#[derive(Clone, Debug, Default)]
pub struct JitFormula {
    pub name: String,
    pub options: Vec<Pair>,
    pub params: Vec<Pair>,
    pub consts: Vec<Pair>,
    pub code: String,
    pub description: String,
}

impl JitFormula {
    /// `NewFormula`
    fn new_formula() -> JitFormula {
        JitFormula {
            name: "JITMyNewFormula".into(),
            options: vec![
                Pair { name: "Version".into(), ty: Datatype::Int64, value: 9.0 },
                Pair { name: "DEscale".into(), ty: Datatype::Double, value: 1.0 },
                Pair { name: "SIpower".into(), ty: Datatype::Double, value: 2.0 },
            ],
            code: "procedure MyFormula(var x, y, z, w: Double; PIteration3D: TPIteration3D);\nbegin\n\nend;\n".into(),
            ..Default::default()
        }
    }

    /// Reads the sections of a `[SOURCE]` formula file.
    pub fn parse(name: &str, text: &str) -> Result<JitFormula, String> {
        let mut f = JitFormula { name: name.into(), ..Default::default() };
        let lines: Vec<&str> = text.lines().collect();
        let mut section = String::new();
        let mut code = Vec::new();
        let mut desc = Vec::new();
        let mut after_end = false;
        for l in lines {
            let t = l.trim();
            if !after_end && t.len() > 1 && t.starts_with('[') && t.ends_with(']') {
                section = t.to_ascii_uppercase();
                if section == "[END]" {
                    after_end = true;
                }
                continue;
            }
            if after_end {
                desc.push(l);
                continue;
            }
            match section.as_str() {
                "[OPTIONS]" | "[CONSTANTS]" => {
                    let Some(rest) = t.strip_prefix('.') else { continue };
                    let (lhs, val) = match rest.split_once('=') {
                        Some((a, b)) => (a.trim(), b.trim()),
                        None => (rest.trim(), ""),
                    };
                    let mut words = lhs.split_whitespace();
                    let first = words.next().unwrap_or("");
                    let second: Vec<&str> = words.collect();
                    let value = super::util::parse_float(val).unwrap_or(0.0);
                    match (Datatype::parse(first), second.is_empty()) {
                        (Some(ty), false) => {
                            let p = Pair { name: second.join(" "), ty, value };
                            if section == "[CONSTANTS]" {
                                f.consts.push(p);
                            } else {
                                f.params.push(p);
                            }
                        }
                        _ => {
                            let ty = if val.contains('.') || val.contains('e') || val.contains('E') { Datatype::Double } else { Datatype::Int64 };
                            f.options.push(Pair { name: lhs.into(), ty, value });
                        }
                    }
                }
                "[SOURCE]" => code.push(l),
                _ => {}
            }
        }
        if code.is_empty() {
            return Err(trf("@MB:NoSOURCESectionThisIsNot", &[&name]));
        }
        f.code = code.join("\n");
        f.description = desc.join("\n");
        Ok(f)
    }

    /// `TJITFormulaWriter.SaveFormula`
    pub fn to_text(&self) -> String {
        let mut w: Vec<String> = Vec::new();
        if !self.options.is_empty() || !self.params.is_empty() {
            w.push("[OPTIONS]".into());
            for p in &self.options {
                w.push(format!(".{} = {}", p.name, p.value_str()));
            }
            for p in &self.params {
                w.push(format!(".{} {} = {}", p.ty.name(), p.name, p.value_str()));
            }
        }
        if !self.consts.is_empty() {
            w.push("[CONSTANTS]".into());
            for p in &self.consts {
                w.push(format!(".{} {} = {}", p.ty.name(), p.name, p.value_str()));
            }
        }
        w.push("[SOURCE]".into());
        w.push(self.code.trim_end_matches(['\r', '\n']).to_string());
        w.push("[END]".into());
        w.push(self.description.trim_end_matches(['\r', '\n']).to_string());
        let mut s = w.join("\r\n");
        s.push_str("\r\n");
        s
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Option,
    Const,
    Param,
}

pub struct State {
    f: JitFormula,
    edit_name: Option<String>,
    /// the value dialog is open for: kind, index (None = add)
    pending: Option<(Kind, Option<usize>)>,
    saving_and_exit: bool,
}

impl Default for State {
    fn default() -> Self {
        State { f: JitFormula::new_formula(), edit_name: None, pending: None, saving_and_exit: false }
    }
}

fn list_name(k: Kind) -> &'static str {
    match k {
        Kind::Option => "OptionsList",
        Kind::Const => "ConstantsList",
        Kind::Param => "ParamsList",
    }
}

fn values(st: &mut State, k: Kind) -> &mut Vec<Pair> {
    match k {
        Kind::Option => &mut st.f.options,
        Kind::Const => &mut st.f.consts,
        Kind::Param => &mut st.f.params,
    }
}

/// `PopulateList`
fn populate(app: &mut Mb3d, ui: &mut Ui, k: Kind) {
    let items: Vec<String> = values(&mut app.jit, k).iter().map(|p| format!("{} = {}", p.name, p.value_str())).collect();
    ui.set_items(F, list_name(k), items);
}

/// `PopulateFields`
fn populate_fields(app: &mut Mb3d, ui: &mut Ui) {
    let f = app.jit.f.clone();
    ui.set_text(F, "CodeEdit", &f.code);
    ui.set_text(F, "DescriptionEdit", &f.description);
    ui.set_text(F, "FormulanameEdit", &f.name);
    for k in [Kind::Param, Kind::Const, Kind::Option] {
        populate(app, ui, k);
        let n = values(&mut app.jit, k).len();
        ui.set_item_index(F, list_name(k), if n > 0 { 0 } else { -1 });
    }
}

/// `EnableControls`
fn enable_controls(ui: &mut Ui) {
    let count = |ui: &Ui, l: &str| ui.c(F, l).items.len();
    let sel = |ui: &Ui, l: &str| ui.item_index(F, l) >= 0;
    let (pc, cc) = (count(ui, "ParamsList"), count(ui, "ConstantsList"));
    let (ps, cs, os) = (sel(ui, "ParamsList"), sel(ui, "ConstantsList"), sel(ui, "OptionsList"));
    ui.set_enabled(F, "ParamAddBtn", pc < 16);
    ui.set_enabled(F, "ParamEditBtn", ps);
    ui.set_enabled(F, "ParamDeleteBtn", ps);
    ui.set_enabled(F, "ConstantAddBtn", cc < 16);
    ui.set_enabled(F, "ConstantEditBtn", cs);
    ui.set_enabled(F, "ConstantDeleteBtn", cs);
    ui.set_enabled(F, "OptionAddBtn", true);
    ui.set_enabled(F, "OptionEditBtn", os);
    ui.set_enabled(F, "OptionDeleteBtn", os);
}

fn info(ui: &mut Ui, msg: &str) {
    let mut t = ui.text(F, "InfoMemo");
    if !t.is_empty() {
        t.push('\n');
    }
    t.push_str(msg);
    ui.set_text(F, "InfoMemo", &t);
}

fn formula_dir(app: &Mb3d) -> PathBuf {
    app.ini.dir(super::ini::DIR_FORMULAS)
}

/// Opens the editor for a new formula or the formula `name` (`forms_jit_open`).
pub fn open(app: &mut Mb3d, ui: &mut Ui, name: Option<String>) {
    app.jit.edit_name = name;
    ui.show_modal(F);
}

/// `Init` (FormShow)
fn init(app: &mut Mb3d, ui: &mut Ui) {
    ui.set_active_page(F, "MainPageControl", "CodeSheet");
    ui.cm(F, "PreprocessedCodeSheet").tab_visible = false;
    let r = match app.jit.edit_name.clone() {
        Some(n) => {
            let p = formula_dir(app).join(format!("{n}.m3f"));
            let p = if p.exists() { p } else { crate::formulas::custom_path(&n).unwrap_or(p) };
            std::fs::read(&p)
                .map_err(|e| trf("@MB:CouldNotLoadFormula", &[&p.display(), &e]))
                .and_then(|d| JitFormula::parse(&n, &String::from_utf8_lossy(&d)))
        }
        None => Ok(JitFormula::new_formula()),
    };
    match r {
        Ok(f) => app.jit.f = f,
        Err(e) => {
            app.jit.f = JitFormula::new_formula();
            ui.show_message(&e);
        }
    }
    populate_fields(app, ui);
    enable_controls(ui);
}

fn take_fields(app: &mut Mb3d, ui: &Ui) {
    app.jit.f.name = ui.text(F, "FormulanameEdit").trim().to_string();
    app.jit.f.code = ui.text(F, "CodeEdit");
    app.jit.f.description = ui.text(F, "DescriptionEdit");
}

/// `Compile`: the formula as the program would load it.
fn compile(app: &mut Mb3d, ui: &mut Ui) -> bool {
    take_fields(app, ui);
    let f = &app.jit.f;
    match crate::m3f::M3f::parse(&f.name, &f.to_text()) {
        Ok(_) => {
            info(ui, &tr("@MB:CompilingSucceeded"));
            true
        }
        Err(e) => {
            ui.show_message(&e);
            false
        }
    }
}

/// `SaveCode` (asks before overwriting).
fn save_code(app: &mut Mb3d, ui: &mut Ui) {
    take_fields(app, ui);
    if !app.jit.f.name.starts_with("JIT") {
        ui.show_message(&tr("@MB:TheFormulaNameShouldStartWith"));
        app.jit.saving_and_exit = false;
        return;
    }
    let p = formula_dir(app).join(format!("{}.m3f", app.jit.f.name));
    if p.exists() {
        ui.confirm("jit:overwrite", &trf("@MB:TheFileM3fAlreadyExistsDo", &[&app.jit.f.name]));
    } else {
        write_formula(app, ui);
    }
}

fn write_formula(app: &mut Mb3d, ui: &mut Ui) {
    let dir = formula_dir(app);
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join(format!("{}.m3f", app.jit.f.name));
    match std::fs::write(&p, app.jit.f.to_text()) {
        Ok(()) => {
            info(ui, &tr("@MB:SavingSucceeded"));
            crate::formulas::forget_custom(&app.jit.f.name);
            if std::mem::take(&mut app.jit.saving_and_exit) {
                ui.hide(F);
                super::formula_form::load_formula_names(app, ui);
            }
        }
        Err(e) => {
            app.jit.saving_and_exit = false;
            ui.show_message(&format!("{}: {e}", p.display()));
        }
    }
}

/// Opens the value dialog (`ParamValueEditFrm.Clear` + `ShowModal`).
fn edit_value(app: &mut Mb3d, ui: &mut Ui, k: Kind, idx: Option<usize>) {
    let title = match (k, idx.is_some()) {
        (Kind::Option, false) => tr("@MB:AddOptionValue"),
        (Kind::Option, true) => tr("@MB:EditOptionValue"),
        (Kind::Const, false) => tr("@MB:AddNamedConstantValue"),
        (Kind::Const, true) => tr("@MB:EditNamedConstantValue"),
        (Kind::Param, false) => tr("@MB:AddNamedParam"),
        (Kind::Param, true) => tr("@MB:EditNamedParam"),
    };
    ui.cm(PV, "ParamnameEdit").read_only = idx.is_some();
    let (name, ty, val) = match idx.and_then(|i| values(&mut app.jit, k).get(i).cloned()) {
        Some(p) => (p.name.clone(), p.ty.name(), fts(p.value)),
        None => (String::new(), "Double", "0".into()),
    };
    ui.set_text(PV, "ParamnameEdit", &name);
    let ti = ui.c(PV, "TypeCmb").items.iter().position(|s| s == ty).map(|i| i as i32).unwrap_or(-1);
    ui.set_item_index(PV, "TypeCmb", ti);
    ui.set_text(PV, "ValueEdit", &val);
    ui.fm(PV).set_caption(&title);
    app.jit.pending = Some((k, idx));
    ui.show_modal(PV);
}

/// `ParamValueEditFrm` OK: validate and apply.
fn value_ok(app: &mut Mb3d, ui: &mut Ui) {
    let name = ui.text(PV, "ParamnameEdit").trim().to_string();
    if name.is_empty() {
        return ui.show_message(&tr("@MB:ParamNameMustNotBeEmpty"));
    }
    let ti = ui.item_index(PV, "TypeCmb");
    let Some(ty) = ui.c(PV, "TypeCmb").items.get(ti.max(0) as usize).and_then(|s| Datatype::parse(s)).filter(|_| ti >= 0) else {
        return ui.show_message(&tr("@MB:TypeMustNotBeEmpty"));
    };
    let vt = ui.text(PV, "ValueEdit");
    let value = if ty == Datatype::Int64 {
        match vt.trim().parse::<i64>() {
            Ok(v) => v as f64,
            Err(_) => return ui.show_message(&trf("@MB:IsNotAValidIntegerValue", &[&vt.trim()])),
        }
    } else {
        match super::util::parse_float(&vt) {
            Some(v) => v,
            None => return ui.show_message(&trf("@MB:IsNotAValidFloatingPoint", &[&vt.trim()])),
        }
    };
    let Some((k, idx)) = app.jit.pending.take() else { return ui.hide(PV) };
    let list = values(&mut app.jit, k);
    let sel = match idx {
        Some(i) => {
            if let Some(p) = list.get_mut(i) {
                p.ty = ty;
                p.value = value;
            }
            i
        }
        None => {
            if list.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
                let what = match k {
                    Kind::Option => tr("@MB:Option"),
                    Kind::Const => tr("@MB:Constant"),
                    Kind::Param => tr("@MB:Param"),
                };
                ui.hide(PV);
                return ui.show_message(&trf("@MB:AlreadyExists", &[&what, &name]));
            }
            list.push(Pair { name, ty, value });
            list.len() - 1
        }
    };
    ui.hide(PV);
    populate(app, ui, k);
    ui.set_item_index(F, list_name(k), sel as i32);
    enable_controls(ui);
}

fn delete_value(app: &mut Mb3d, ui: &mut Ui, k: Kind) {
    let i = ui.item_index(F, list_name(k));
    let list = values(&mut app.jit, k);
    if i >= 0 && (i as usize) < list.len() {
        list.remove(i as usize);
    }
    let n = list.len();
    populate(app, ui, k);
    ui.set_item_index(F, list_name(k), if n > 0 { 0 } else { -1 });
    enable_controls(ui);
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let sel = |ui: &Ui, k: Kind| {
        let i = ui.item_index(F, list_name(k));
        (i >= 0).then_some(i as usize)
    };
    match e.handler.as_str() {
        "FormShow" => init(app, ui),
        "CancelAndExitBtnClick" => ui.hide(F),
        "SaveAndExitBtnClick" => {
            if compile(app, ui) {
                app.jit.saving_and_exit = true;
                save_code(app, ui);
            }
        }
        "SaveBtnClick" => save_code(app, ui),
        "CompileBtnClick" => {
            compile(app, ui);
        }
        "FormKeyDown" => match e.ev {
            Ev::KeyDown { key: 120, .. } => {
                compile(app, ui);
            }
            Ev::KeyDown { key: 116, .. } => save_code(app, ui),
            _ => {}
        },
        "LoadFormulaBtnClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3f", tr("@MB:FormulaM3f")),
                initial_dir: Some(formula_dir(app)),
                ..Default::default()
            };
            ui.open_dialog("jit:load", &o);
        }
        "OptionsListClick" | "ConstantsListClick" | "ParamsListClick" => enable_controls(ui),
        "OptionAddBtnClick" => edit_value(app, ui, Kind::Option, None),
        "ConstantAddBtnClick" => edit_value(app, ui, Kind::Const, None),
        "ParamAddBtnClick" => edit_value(app, ui, Kind::Param, None),
        "OptionEditBtnClick" | "OptionsListDblClick" => {
            if let Some(i) = sel(ui, Kind::Option) {
                edit_value(app, ui, Kind::Option, Some(i));
            }
        }
        "ConstantEditBtnClick" | "ConstantsListDblClick" => {
            if let Some(i) = sel(ui, Kind::Const) {
                edit_value(app, ui, Kind::Const, Some(i));
            }
        }
        "ParamEditBtnClick" | "ParamsListDblClick" => {
            if let Some(i) = sel(ui, Kind::Param) {
                edit_value(app, ui, Kind::Param, Some(i));
            }
        }
        "OptionDeleteBtnClick" => delete_value(app, ui, Kind::Option),
        "ConstantDeleteBtnClick" => delete_value(app, ui, Kind::Const),
        "ParamDeleteBtnClick" => delete_value(app, ui, Kind::Param),
        _ => {}
    }
}

pub fn value_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "SaveAndExitBtnClick" => value_ok(app, ui),
        "CancelAndExitBtnClick" => {
            app.jit.pending = None;
            ui.hide(PV);
        }
        "FormKeyDown" => {
            if let Ev::KeyDown { key: 27, .. } = e.ev {
                app.jit.pending = None;
                ui.hide(PV);
            }
        }
        "FormShow" => ui.set_focus(PV, "SaveAndExitBtn"),
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("load", DialogResult::File(Some(p))) => {
            app.jit.edit_name = p.file_stem().map(|s| s.to_string_lossy().into_owned());
            let r = std::fs::read(p).map_err(|e| e.to_string()).and_then(|d| JitFormula::parse(app.jit.edit_name.as_deref().unwrap_or(""), &String::from_utf8_lossy(&d)));
            match r {
                Ok(f) => {
                    app.jit.f = f;
                    populate_fields(app, ui);
                    enable_controls(ui);
                }
                Err(e) => ui.show_message(&e),
            }
        }
        ("overwrite", DialogResult::Button(b)) => {
            if *b == crate::vcl::form::MR_YES {
                write_formula(app, ui);
            } else {
                app.jit.saving_and_exit = false;
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_formula_roundtrip_and_compile() {
        let mut f = JitFormula::new_formula();
        f.params.push(Pair { name: "Scale".into(), ty: Datatype::Double, value: 2.5 });
        f.consts.push(Pair { name: "Half".into(), ty: Datatype::Double, value: 0.5 });
        f.code = "procedure MyFormula(var x, y, z, w: Double; PIteration3D: TPIteration3D);\nbegin\n  x := x * Scale * Half;\nend;\n".into();
        f.description = "test".into();
        let t = f.to_text();
        let g = JitFormula::parse("JITMyNewFormula", &t).unwrap();
        assert_eq!(g.options.len(), 3);
        assert_eq!(g.params[0].name, "Scale");
        assert_eq!(g.consts[0].value, 0.5);
        assert_eq!(g.description.trim(), "test");
        crate::m3f::M3f::parse("JITMyNewFormula", &t).unwrap();
    }
}
