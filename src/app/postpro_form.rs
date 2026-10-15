//! The post processing window (PostProcessForm.pas, `TPostProForm`):
//! normals on the z-buffer, hard shadows, ambient shadows, reflections and
//! transparency, depth of field and the recalculation of a selection.
//! "Calculate now" applies a processing to the kept image; the
//! "automatically" boxes put it into the parameters of the next calculation.

use super::util::{d2byte, d2byte_str, fts_single, parse_float, pti, ptf};
use super::{Mb3d, MAIN};
use crate::dof::DofParams;
use crate::scene::ShadowParams;
use crate::ssao::SsaoParams;
use crate::vcl::{Event, Ui};

const F: &str = "PostProForm";

/// Processings applied with "Calculate now" to the image on screen (kept
/// until the next calculation, like MB3D's bits in the G-buffer).
#[derive(Default)]
pub struct State {
    pub hs_now: bool,
    pub ao_now: bool,
    pub normals_now: bool,
    pub dof_now: bool,
    pub refl_now: bool,
}

const PANELS: [(&str, &str, &str); 7] = [
    ("RecalcSectionBtn", "RecalcSectionPnl", "CheckBox21"),
    ("NormalsOnZBufferBtn", "NormalsOnZBufferPnl", "CheckBox23"),
    ("HardShadowsBtn", "HardShadowsPnl", "CheckBox9"),
    ("AmbientShadowsBtn", "AmbientShadowsPnl", "CheckBox11"),
    ("ReflTransparencyBtn", "ReflTransparencyPnl", "CheckBox24"),
    ("DepthOfFieldBtn", "DepthOfFieldPnl", "CheckBox1"),
    ("NoiseFilterBtn", "NoiseFilterPnl", "NoiseAutoCbx"),
];

/// The strength of the noise filter (Noise Filter section), 0 when off.
pub fn noise_sigma(ui: &Ui) -> f32 {
    ui.text(F, "NoiseSigmaEdit").trim().replace(',', ".").parse::<f32>().map_or(4.0, |v| v.clamp(0.0, 255.0))
}

/// "Filter new pictures automatically" of the Noise Filter section.
pub fn noise_auto(ui: &Ui) -> bool {
    ui.has_form(F) && ui.checked(F, "NoiseAutoCbx")
}

fn shadow_params(ui: &Ui) -> ShadowParams {
    let mut lights = 0u8;
    for i in 0..6 {
        if ui.checked(F, &format!("CheckBox{}", i + 2)) {
            lights |= 1 << i;
        }
    }
    ShadowParams {
        lights,
        soft: ui.checked(F, "CheckBox29") && ui.enabled(F, "CheckBox29"),
        soft_radius: ptf(&ui.text(F, "Edit7")).clamp(0.01, 20.0) as f32,
        max_len_mul: ptf(&ui.text(F, "Edit5")).max(0.0) as f32,
        set_cos: ui.checked(F, "CheckBox10"),
    }
}

fn dof_params(ui: &Ui) -> DofParams {
    DofParams {
        z_sharp: ptf(&ui.text(F, "Edit1")) as f32,
        z_sharp2: ptf(&ui.text(F, "Edit10")) as f32,
        aperture: ptf(&ui.text(F, "Edit3")) as f32,
        clip_r: ptf(&ui.text(F, "Edit2")) as f32,
        passes: (ui.item_index(F, "RadioGroup1").clamp(0, 2) + 1) as u8,
        forward: ui.item_index(F, "RadioGroup2") == 1,
    }
}

/// The post processing parameters into the scene (`PutDOFparsToHeader`,
/// `PutAmbientParsToHeader`, `PutReflectionParsToHeader`, HS options).
pub fn make_header(app: &mut Mb3d, ui: &mut Ui) {
    let st = &app.post;
    let sc = &mut app.scene;
    sc.shadows = (ui.checked(F, "CheckBox9") || st.hs_now).then(|| shadow_params(ui));
    let tab = ui.c(F, "TabControl1").tab_index;
    let ao_on = ui.checked(F, "CheckBox11") || st.ao_now;
    sc.ao = ao_on.then(|| SsaoParams {
        threshold: ptf(&ui.text(F, "Edit34")).max(0.01) as f32,
        border_mirror: d2byte(&ui.text(F, "Edit9")) as f32 * 0.01,
        t0: ui.checked(F, "CheckBox12"),
        random: if tab == 2 { ui.position(F, "UpDown3").clamp(1, 9) as u8 } else { 0 },
        bits15: tab == 0,
        wrap_x: false,
    });
    sc.deao = (ao_on && tab == 3).then(|| crate::deao::DeaoParams {
        quality: ui.position(F, "UpDown1").clamp(0, 3) as u8,
        dither: ui.item_index(F, "RadioGroup5").clamp(0, 2) as u8,
        max_len: ptf(&ui.text(F, "Edit8")).clamp(1.0 / 255.0, 100.0) as f32,
        first_step_random: ui.checked(F, "CheckBox22"),
    });
    sc.normals_on_zbuf = ui.checked(F, "CheckBox23") || st.normals_now;
    sc.dof = (ui.checked(F, "CheckBox1") || st.dof_now).then(|| dof_params(ui));
    let m = &mut sc.mc;
    m.reflections = ui.checked(F, "CheckBox24") || st.refl_now;
    m.transparency = ui.checked(F, "CheckBox27");
    m.only_difs = ui.checked(F, "CheckBox28");
    m.reflection_amount = ptf(&ui.text(F, "Edit6")).clamp(0.0, 100.0) as f32;
    m.reflection_depth = ui.position(F, "UpDown2").clamp(1, 8) as u8;
    m.absorption = ptf(&ui.text(F, "Edit13")).clamp(1e-30, 1e10) as f32;
    m.refraction_index = ptf(&ui.text(F, "Edit14")).clamp(0.1, 10.0) as f32;
    m.scattering = ptf(&ui.text(F, "Edit17")).max(0.0) as f32;
    m.soft_shadow_radius = parse_float(&ui.text(F, "Edit7")).unwrap_or(1.0).clamp(0.01, 20.0) as f32;
}

/// The scene's post processing into the window (`SetEditsFromHeader`'s part).
pub fn set_from_header(app: &mut Mb3d, ui: &mut Ui) {
    app.post = State::default();
    let sc = app.scene.clone();
    ui.set_checked(F, "CheckBox1", sc.dof.is_some());
    let d = sc.dof.unwrap_or_default();
    ui.set_item_index(F, "RadioGroup2", d.forward as i32);
    ui.set_item_index(F, "RadioGroup1", (d.passes as i32 - 1).clamp(0, 2));
    ui.set_text(F, "Edit1", &fts_single(d.z_sharp as f64));
    ui.set_text(F, "Edit2", &fts_single(d.clip_r as f64));
    ui.set_text(F, "Edit3", &fts_single(d.aperture as f64));
    ui.set_text(F, "Edit10", &fts_single(d.z_sharp2 as f64));
    let h = sc.shadows.unwrap_or_default();
    ui.set_text(F, "Edit5", &fts_single(h.max_len_mul as f64));
    ui.set_checked(F, "CheckBox9", sc.shadows.is_some());
    ui.set_checked(F, "CheckBox10", h.set_cos);
    for i in 0..6 {
        ui.set_checked(F, &format!("CheckBox{}", i + 2), h.lights >> i & 1 != 0);
    }
    ui.set_checked(F, "CheckBox29", h.soft);
    let a = sc.ao.unwrap_or_default();
    ui.set_checked(F, "CheckBox11", sc.ao.is_some());
    ui.set_checked(F, "CheckBox12", a.t0);
    let tab = if sc.deao.is_some() {
        3
    } else if a.bits15 {
        0
    } else if a.random > 0 {
        2
    } else {
        1
    };
    ui.cm(F, "TabControl1").tab_index = tab;
    if let Some(de) = &sc.deao {
        ui.set_position(F, "UpDown1", de.quality as i64);
        ui.set_item_index(F, "RadioGroup5", de.dither as i32);
        ui.set_text(F, "Edit8", &fts_single(de.max_len as f64));
        ui.set_checked(F, "CheckBox22", de.first_step_random);
    }
    ui.set_position(F, "UpDown3", (a.random as i64).clamp(1, 9));
    ui.set_text(F, "Edit21", &(a.random as i64).clamp(1, 9).to_string());
    ui.set_text(F, "Edit34", &fts_single(a.threshold.abs() as f64));
    ui.set_text(F, "Edit9", &d2byte_str((a.border_mirror * 100.0).round().clamp(0.0, 250.0) as u8));
    radio_group3_click(ui);
    ui.set_checked(F, "CheckBox23", sc.normals_on_zbuf);
    let m = sc.mc;
    ui.set_checked(F, "CheckBox24", m.reflections);
    ui.set_text(F, "Edit6", &fts_single(m.reflection_amount as f64));
    ui.set_position(F, "UpDown2", (m.reflection_depth as i64).clamp(1, 8));
    ui.set_text(F, "Edit11", &(m.reflection_depth as i64).clamp(1, 8).to_string());
    ui.set_text(F, "Edit13", &fts_single(m.absorption as f64));
    ui.set_text(F, "Edit14", &fts_single(m.refraction_index as f64));
    ui.set_text(F, "Edit17", &fts_single(m.scattering as f64));
    ui.set_checked(F, "CheckBox27", m.transparency);
    ui.set_checked(F, "CheckBox28", m.only_difs);
    ui.set_text(F, "Edit7", &fts_single(m.soft_shadow_radius as f64));
    check_box2(ui);
    update_captions(ui);
}

fn radio_group3_click(ui: &mut Ui) {
    let t = ui.c(F, "TabControl1").tab_index;
    let b = t < 3;
    for c in ["RadioGroup5", "CheckBox22", "Label15", "Label16", "Label17", "Edit8", "UpDown1"] {
        ui.set_visible(F, c, !b);
    }
    for c in ["CheckBox12", "Edit34", "Label50"] {
        ui.set_visible(F, c, b);
    }
    for c in ["UpDown3", "Edit21", "Label14"] {
        ui.set_visible(F, c, t == 2);
    }
    for c in ["Label20", "Edit9"] {
        ui.set_visible(F, c, t == 1 || t == 2);
    }
    let rays = [3, 7, 17, 33][ui.position(F, "UpDown1").clamp(0, 3) as usize];
    ui.set_caption(F, "Label17", &rays.to_string());
}

fn check_box2(ui: &mut Ui) {
    let n = (2..8).filter(|i| ui.checked(F, &format!("CheckBox{i}"))).count();
    for c in ["CheckBox29", "Edit7", "Label6"] {
        ui.set_enabled(F, c, n == 1);
    }
}

/// `UpdateButtonCaption`: a check mark glyph on the section buttons whose
/// processing runs automatically.
fn update_captions(ui: &mut Ui) {
    let img = ui.c(F, "ImageList1").images.first().cloned();
    for (btn, _, cb) in PANELS {
        let on = ui.checked(F, cb);
        let c = ui.cm(F, btn);
        c.glyph = if on { img.clone() } else { None };
        c.num_glyphs = 1;
        c.alignment = crate::vcl::canvas::HAlign::Left;
    }
}

/// `AlignPanels`: the window's height follows the open sections.
fn align_panels(ui: &mut Ui) {
    let mut h = ui.c(F, "PostProcessHintPnl").height;
    for (btn, pnl, _) in PANELS {
        h += ui.c(F, btn).height;
        if ui.visible(F, pnl) {
            h += ui.c(F, pnl).height;
        }
    }
    let w = ui.f(F).client_size().0;
    ui.set_client_size(F, w, h);
}

fn toggle(ui: &mut Ui, btn: &str, pnl: &str) {
    let r = ui.c(F, btn).rect();
    let v = !ui.visible(F, pnl);
    let p = ui.cm(F, pnl);
    // just below its button in the alTop order
    p.top = r.bottom() - 1;
    p.visible = v;
    align_panels(ui);
    update_captions(ui);
}

/// The image is repainted with the post processing of the parameters.
pub fn repaint(app: &mut Mb3d, ui: &mut Ui) {
    if app.main.calculating || app.eng.base().is_none() {
        app.message(ui, "Calculate the image first.");
        return;
    }
    app.make_scene(ui);
    app.eng.start(super::engine::Job::Repaint { scene: app.scene.clone() });
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    for (btn, pnl, _) in PANELS {
        if h == format!("{btn}Click") {
            toggle(ui, btn, pnl);
            return;
        }
    }
    match h {
        "NoiseFilterNowBtnClick" => super::main_form::denoise_picture(app, ui, noise_sigma(ui)),
        "NoiseAutoCbxClick" => update_captions(ui),
        "FormShow" => {
            ui.cm(F, "Edit11").hint = "Depth of reflections: 1 to see only one reflection,\nwith 2 you could see reflections in the reflected object, ... and so on.".into();
            ui.cm(F, "Edit12").hint = "Divides the actual \"Raystep multiplier\" by this value,\nincrease the value to reduce overstepping.".into();
            align_panels(ui);
            update_captions(ui);
        }
        "FormHide" => {
            let f = ui.f(F);
            let s = format!("{} {}", f.left, f.top);
            app.ini.set("PostpPos", &s);
        }
        "CheckBox23Click" | "CheckBox9Click" | "CheckBox11Click" | "CheckBox24Click" | "CheckBox1Click" => update_captions(ui),
        "CheckBox2Click" => check_box2(ui),
        "RadioGroup3Click" => radio_group3_click(ui),
        "UpDown1ChangingEx" => radio_group3_click(ui),
        "Button14Click" => {
            app.post.normals_now = true;
            repaint(app, ui);
        }
        "Button3Click" => {
            if shadow_params(ui).lights == 0 {
                ui.show_message("Please select the lights first.  No HS for lightmaps.");
                return;
            }
            app.post.hs_now = true;
            repaint(app, ui);
        }
        "Button4Click" => {
            // reset the hard shadow of a light
            let t = ui.tag(F, &e.sender) as usize;
            ui.set_checked(F, &format!("CheckBox{}", t + 1), false);
            if shadow_params(ui).lights == 0 {
                app.post.hs_now = false;
            }
            repaint(app, ui);
        }
        "Button10Click" => {
            app.post.ao_now = true;
            repaint(app, ui);
        }
        "Button11Click" => {
            app.post.refl_now = true;
            repaint(app, ui);
        }
        "Button1Click" => {
            if app.scene.tiling.is_some() {
                ui.show_message("No DoF on tiles.");
                return;
            }
            app.post.dof_now = true;
            repaint(app, ui);
        }
        "Button2Click" => {
            let tg = ui.tag(F, &e.sender) as i32;
            if ui.caption(F, &e.sender) == "Click on image" {
                ui.set_caption(F, &e.sender, if tg == 1 { "Get Z1" } else { "Get Z2" });
                app.main.get_pos = 0;
                ui.cm(MAIN, "Image1").cursor = "crDefault".into();
            } else {
                app.main.get_pos = tg;
                ui.set_caption(F, &e.sender, "Click on image");
                ui.cm(MAIN, "Image1").cursor = "crCross".into();
            }
        }
        "Button19Click" => {
            let t = ui.text(F, "Edit1");
            ui.set_text(F, "Edit10", &t);
        }
        "Button16Click" => {
            // "Reset now": the processings applied by hand are dropped
            app.post = State::default();
            repaint(app, ui);
        }
        "CheckBox21Click" | "CheckBox25Click" => {
            update_captions(ui);
            let on = ui.checked(F, &e.sender);
            ui.set_enabled(F, "Button13", ui.checked(F, "CheckBox21"));
            if on {
                for b in ["SpeedButton1", "SpeedButton2", "SpeedButton4"] {
                    ui.cm(MAIN, b).down = false;
                }
                ui.cm(MAIN, "Image1").cursor = "crCross".into();
            } else {
                ui.set_visible(MAIN, "Shape1", false);
                ui.cm(MAIN, "Image1").cursor = "crDefault".into();
            }
        }
        "Button13Click" => {
            let sh = ui.c(MAIN, "Shape1").rect();
            let img = ui.c(MAIN, "Image1").rect();
            if !ui.visible(MAIN, "Shape1") {
                ui.show_message("Please make a selection in the image first.");
                return;
            }
            let s = app.main.image_scale;
            let rect = [(sh.x - img.x) * s, (sh.y - img.y) * s, sh.w * s, sh.h * s];
            if rect[2] <= 2 || rect[3] <= 2 {
                ui.show_message("The selection must be bigger than 2 pixels in width and height.");
                return;
            }
            app.make_scene(ui);
            let div = pti(&ui.text(F, "Edit12")).max(1) as f64;
            let nearer = ui.checked(F, "CheckBox19");
            super::main_form::disable_buttons(app, ui);
            app.main.calc3d = false;
            app.eng.start(super::engine::Job::Recalc { scene: app.scene.clone(), rect, div, nearer });
        }
        "Button12Click" => {
            // DoubleImageSize: the kept G-buffer interpolated to twice the size
            if app.main.calculating {
                return;
            }
            let Some(b) = app.eng.base() else { return ui.show_message("No image calculated yet.") };
            let g = crate::gbuffer::double_size(&b.post, b.w, b.h);
            let mut sc = b.scene.clone();
            sc.width *= 2;
            sc.height *= 2;
            sc.de_stop *= 2.0;
            app.make_scene(ui);
            let light = app.scene.lighting.clone();
            sc.lighting = light;
            match app.eng.set_gbuffer(&sc, g) {
                Ok(()) => {
                    app.scene.width = sc.width;
                    app.scene.height = sc.height;
                    app.scene.de_stop = sc.de_stop;
                    let s = match app.main.image_scale {
                        1 => 2,
                        2 => 4,
                        s => s,
                    };
                    super::main_form::set_edits_from_header(app, ui);
                    super::main_form::update_scale(app, ui, s);
                    super::main_form::show_image(app, ui);
                }
                Err(e) => ui.show_message(&e),
            }
        }
        "FormMouseWheel" => {}
        _ => {}
    }
}
