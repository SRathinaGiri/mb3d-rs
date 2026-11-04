//! The Unfinished renders window (`UnfinishedFrm`, this program's own,
//! the Resume button of the Open tab): the renders that keep a checkpoint
//! (see [`crate::checkpoint`]), to continue one or delete them.

use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Event};

const F: &str = "UnfinishedFrm";

/// The listed renders, in the order of the list.
#[derive(Default)]
pub struct State {
    entries: Vec<crate::checkpoint::Entry>,
    /// the key of the render whose image the save dialog is for
    saving: Option<String>,
}

fn refresh(app: &mut Mb3d, ui: &mut Ui) {
    app.unfinished.entries = crate::checkpoint::list();
    let g = |e: &crate::checkpoint::Entry, k: &str| e.info.get(k).cloned().unwrap_or_default();
    let rows: Vec<Vec<String>> = app
        .unfinished
        .entries
        .iter()
        .map(|e| {
            let kind = match g(e, "kind").as_str() {
                "calc" => tr("@MB:Picture"),
                "mc" => tr("@MB:MonteCarlo2"),
                "swarm" => tr("@MB:SwarmJob"),
                "mesh" => tr("@MB:Mesh"),
                k => k.to_string(),
            };
            vec![
                g(e, "name"),
                kind,
                e.size_text(),
                e.done_text(),
                if e.busy { tr("@MB:RenderingNow") } else { crate::checkpoint::ago(e.age()) },
                format!("{:.1}", e.bytes as f64 / 1e6),
            ]
        })
        .collect();
    let total: u64 = app.unfinished.entries.iter().map(|e| e.bytes).sum();
    let c = ui.cm(F, "ListView1");
    c.checks.clear();
    c.cells = rows;
    if c.item_index >= c.cells.len() as i32 || c.item_index < 0 {
        c.item_index = if c.cells.is_empty() { -1 } else { 0 };
    }
    ui.set_caption(F, "Label2", &trf("@MB:RenderSMB", &[&app.unfinished.entries.len(), &format!("{:.1}", total as f64 / 1e6)]));
}

/// The selected render.
fn selected(app: &Mb3d, ui: &Ui) -> Option<crate::checkpoint::Entry> {
    let i = ui.item_index(F, "ListView1");
    (i >= 0).then(|| app.unfinished.entries.get(i as usize).cloned()).flatten()
}

/// Loads the parameters of `e` into the main window.
fn load(app: &mut Mb3d, ui: &mut Ui, e: &crate::checkpoint::Entry) -> Result<(), String> {
    let text = e.scene_text().ok_or_else(|| tr("@MB:ThisRenderKeptNoParameters"))?;
    app.scene = crate::scene::Scene::parse(&text)?;
    app.title = e.info.get("name").cloned().unwrap_or_default();
    app.eng.clear();
    super::main_form::scene_loaded(app, ui);
    Ok(())
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => refresh(app, ui),
        "Button1Click" | "Button2Click" => {
            let Some(en) = selected(app, ui) else { return };
            if en.busy {
                app.message(ui, &tr("@MB:ThisRenderIsRunningRightNow"));
                return;
            }
            if let Err(m) = load(app, ui, &en) {
                app.message(ui, &m);
                return;
            }
            if e.handler == "Button1Click" {
                match en.info.get("kind").map(String::as_str) {
                    Some("calc") => {
                        ui.hide(F);
                        super::main_form::continue_calc(app, ui, &en.key());
                    }
                    Some("mc") => {
                        ui.hide(F);
                        super::mc_form::continue_render(app, ui);
                    }
                    _ => app.message(ui, &tr("@MB:ParametersLoadedThisRenderContinuesWhere")),
                }
            }
        }
        "Button3Click" => {
            if let Some(en) = selected(app, ui) {
                if !crate::checkpoint::remove(&en) {
                    app.message(ui, &tr("@MB:ThisRenderIsRunningRightNow"));
                }
                refresh(app, ui);
            }
        }
        "Button4Click" => {
            if !app.unfinished.entries.is_empty() {
                ui.confirm("unfinished:clear", &tr("@MB:DeleteWhatAllUnfinishedRendersKeptQuestion"));
            }
        }
        "Button5Click" => {
            let Some(en) = selected(app, ui) else { return };
            if !matches!(en.info.get("kind").map(String::as_str), Some("calc") | Some("tiles")) {
                app.message(ui, &tr("@MB:OnlyAPictureIsKeptAs"));
                return;
            }
            let o = crate::vcl::dialogs::FileOptions {
                filter: tr("@MB:MB3DImageM3iM3i"),
                default_ext: "m3i".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
                file_name: super::main_form::stem(&en.info.get("name").cloned().unwrap_or_default()).to_string(),
                ..Default::default()
            };
            app.unfinished.saving = Some(en.key());
            ui.save_dialog("unfinished:m3i", &o);
        }
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("clear", DialogResult::Button(crate::vcl::form::MR_YES)) => {
            for e in crate::checkpoint::list() {
                crate::checkpoint::remove(&e);
            }
            refresh(app, ui);
        }
        ("m3i", DialogResult::File(Some(p))) => {
            let Some(key) = app.unfinished.saving.take() else { return };
            let r = match crate::checkpoint::list().into_iter().find(|e| e.key() == key) {
                Some(e) => crate::checkpoint::export_m3i(&e, p),
                None => Err(tr("@MB:ThisRenderKeepsNoImage")),
            };
            if let Err(m) = r {
                app.message(ui, &m);
            }
        }
        _ => {}
    }
}
