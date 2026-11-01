//! The "Experimental speed-ups" window (`ExperimentalFrm`, this program's
//! own, the Internal tab's "Experimental..." button): the switch of the
//! experimental speed-ups and each of them (`crate::experimental`), saved
//! with the scene.  The window's controls are the scene's values: they are
//! read into it with the main window's fields (`make_header`) and set from
//! it when a scene is opened (`set_edits_from_header`).

use super::util::{fts_single, ptf};
use super::{Mb3d, Ui};
use crate::vcl::Event;

const F: &str = "ExperimentalFrm";

/// Opens the window.
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    show_values(app, ui);
    ui.show(F);
}

/// The settings kept in Mandelbulb3D.ini into the scene (at the start and
/// when a scene is opened); nothing kept, the scene's own.
pub fn apply_saved(app: &mut Mb3d) {
    if let Some(e) = app.ini.get_extra(INI_ITEM).and_then(crate::experimental::Experimental::from_ini) {
        app.scene.experimental = e;
    }
}

/// The scene's settings kept in Mandelbulb3D.ini.
fn save(app: &mut Mb3d) {
    let v = app.scene.experimental.to_ini();
    if app.ini.get_extra(INI_ITEM) != Some(v.as_str()) {
        app.ini.set_extra(INI_ITEM, &v);
        let _ = app.ini.save();
    }
}

const INI_ITEM: &str = "Experimental";

/// The scene's settings into the window's controls.
pub fn show_values(app: &Mb3d, ui: &mut Ui) {
    let e = app.scene.experimental;
    ui.set_checked(F, "OnBox", e.on);
    ui.set_text(F, "RelaxEdit", &fts_single(e.step_relax));
    ui.set_checked(F, "SecantBox", e.secant);
    ui.set_checked(F, "FreeBox", e.free_steps);
    ui.set_checked(F, "HalfBox", e.ao_half);
    enable(ui);
}

/// The window's controls into the scene.
pub fn read_values(app: &mut Mb3d, ui: &Ui) {
    let e = &mut app.scene.experimental;
    e.on = ui.checked(F, "OnBox");
    let r = ptf(&ui.text(F, "RelaxEdit"));
    e.step_relax = if r.is_finite() { r.clamp(1.0, 2.0) } else { 1.0 };
    e.secant = ui.checked(F, "SecantBox");
    e.free_steps = ui.checked(F, "FreeBox");
    e.ao_half = ui.checked(F, "HalfBox");
}

/// The single speed-ups only with the switch on.
fn enable(ui: &mut Ui) {
    let on = ui.checked(F, "OnBox");
    for c in ["RelaxLabel", "RelaxEdit", "RelaxNote", "SecantBox", "SecantNote", "FreeBox", "FreeNote", "HalfBox", "HalfNote"] {
        ui.set_enabled(F, c, on);
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => show_values(app, ui),
        "OnBoxClick" | "RelaxEditChange" => {
            enable(ui);
            read_values(app, ui);
            save(app);
        }
        "CloseBtnClick" => {
            read_values(app, ui);
            save(app);
            ui.hide(F);
        }
        _ => {}
    }
}
