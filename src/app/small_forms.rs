//! The smaller windows: visual theme (VisualThemesGUI.pas), initial
//! directories (IniDirsForm.pas), map sequences (MapSequencesGUI.pas),
//! text parameters (TextBox.pas) and the 16 bit z-buffer generator
//! (ZBuf16BitGenUI.pas).

use crate::vcl::lang::tr;
use super::util::{fts, parse_float, pti, ptf};
use super::Mb3d;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{DialogResult, Event, Style, Ui};

// ---------------------------------------------------------------------------
// VisualThemesFrm
// ---------------------------------------------------------------------------

const TH: &str = "VisualThemesFrm";

/// The interface sizes offered (percent), `UIScale` in the ini file.
pub const UI_SCALES: [u32; 6] = [75, 100, 125, 150, 175, 200];

/// The interface scale of the ini file (percent), 100 when unset.
pub fn ini_ui_scale(ini: &super::ini::Ini) -> u32 {
    ini.get_extra("UIScale").and_then(|v| v.trim().parse().ok()).unwrap_or(100)
}

pub fn themes_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let set = |ui: &mut Ui, name: &str| {
        let i = ui.c(TH, "StylesCmb").items.iter().position(|s| s == name).map(|i| i as i32).unwrap_or(0);
        ui.set_item_index(TH, "StylesCmb", i);
        if let Some(s) = Style::from_name(name) {
            ui.set_style(s);
        }
    };
    match e.handler.as_str() {
        "FormShow" => {
            ui.set_items(TH, "StylesCmb", Style::all().into_iter().map(|s| s.name().to_string()).collect());
            let cur = ui.theme.style.name().to_string();
            set(ui, &cur);
            ui.set_items(TH, "UIScaleCmb", UI_SCALES.iter().map(|p| format!("{p} %")).collect());
            let pct = (ui.ui_scale * 100.0).round() as u32;
            let i = UI_SCALES.iter().position(|p| *p == pct).unwrap_or(1);
            ui.set_item_index(TH, "UIScaleCmb", i as i32);
        }
        "UIScaleCmbChange" => {
            let i = ui.item_index(TH, "UIScaleCmb").clamp(0, UI_SCALES.len() as i32 - 1) as usize;
            let pct = UI_SCALES[i];
            ui.set_ui_scale(pct as f64 / 100.0);
            app.ini.set_extra("UIScale", &pct.to_string());
        }
        "StylesCmbChange" => {
            let s = ui.c(TH, "StylesCmb").item_text();
            set(ui, &s);
        }
        "DefaultThemeBtnClick" => set(ui, "Glossy"),
        "ThemesOffBtnClick" => set(ui, "Windows"),
        "SaveAndExitBtnClick" => {
            app.ini.set("VisualTheme", ui.theme.style.name());
            let _ = app.ini.save();
            ui.hide(TH);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// IniDirForm
// ---------------------------------------------------------------------------

const ID: &str = "IniDirForm";

pub fn inidirs_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => {
            for i in 0..13 {
                let d = app.ini.dirs[i].display().to_string();
                ui.set_text(ID, &format!("Edit{}", i + 1), &d);
            }
        }
        "Button3Click" => {
            let t = ui.tag(ID, &e.sender).clamp(0, 12) as usize;
            let cur = std::path::PathBuf::from(ui.text(ID, &format!("Edit{}", t + 1)));
            ui.folder_dialog(&format!("inidirs:{t}"), &ui.caption(ID, &e.sender), Some(cur));
        }
        "Button1Click" => {
            let old_formulas = app.ini.dirs[super::ini::DIR_FORMULAS].clone();
            for i in 0..13 {
                app.ini.dirs[i] = ui.text(ID, &format!("Edit{}", i + 1)).trim().into();
            }
            let _ = app.ini.save();
            crate::formulas::add_formula_dir(app.ini.dir(super::ini::DIR_FORMULAS));
            crate::maps::add_map_dir(app.ini.dir(super::ini::DIR_MAPS));
            ui.hide(ID);
            if old_formulas != app.ini.dirs[super::ini::DIR_FORMULAS] {
                super::formula_form::load_formula_names(app, ui);
            }
        }
        _ => {}
    }
}

pub fn inidirs_dialog(_app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let (Ok(t), DialogResult::File(Some(p))) = (what.parse::<usize>(), r) {
        ui.set_text(ID, &format!("Edit{}", t + 1), &p.display().to_string());
    }
}

// ---------------------------------------------------------------------------
// MapSequencesFrm
// ---------------------------------------------------------------------------

const MS: &str = "MapSequencesFrm";

#[derive(Default)]
pub struct MapSeqState {
    list: Vec<crate::maps::MapSequence>,
    refreshing: bool,
}

fn ms_caption(q: &crate::maps::MapSequence) -> String {
    let p = std::path::Path::new(&q.filename);
    let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = p.parent().and_then(|d| d.file_name()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = if dir.chars().count() > 16 { format!("..{}", dir.chars().rev().take(16).collect::<String>().chars().rev().collect::<String>()) } else { dir };
    if dir.is_empty() {
        name
    } else {
        format!("/{dir}/{name}")
    }
}

fn ms_refresh(app: &mut Mb3d, ui: &mut Ui) {
    let items = app.mapseq.list.iter().map(ms_caption).collect();
    let i = ui.item_index(MS, "MapSequencesList");
    ui.set_items(MS, "MapSequencesList", items);
    ui.set_item_index(MS, "MapSequencesList", i.min(app.mapseq.list.len() as i32 - 1));
    ms_detail(app, ui);
}

fn ms_detail(app: &mut Mb3d, ui: &mut Ui) {
    app.mapseq.refreshing = true;
    let i = ui.item_index(MS, "MapSequencesList");
    let q = (i >= 0).then(|| app.mapseq.list.get(i as usize).cloned()).flatten();
    for c in ["ImageFilenameEdit", "FirstImageEdit", "LastImageEdit", "IncrementEdit", "DestChannelEdit", "LoopCheckBox", "DeleteBtn"] {
        ui.set_enabled(MS, c, q.is_some());
    }
    match q {
        Some(q) => {
            ui.set_text(MS, "ImageFilenameEdit", &q.filename);
            ui.set_text(MS, "FirstImageEdit", &q.first.to_string());
            ui.set_text(MS, "LastImageEdit", &q.last.to_string());
            ui.set_text(MS, "IncrementEdit", &q.increment.to_string());
            ui.set_text(MS, "DestChannelEdit", &q.channel.to_string());
            ui.set_checked(MS, "LoopCheckBox", q.looped);
            let img = crate::image::load(std::path::Path::new(&crate::maps::MapSequence::format_frame(&q.filename, q.first).unwrap_or(q.filename.clone())))
                .ok()
                .map(|im| thumb(&im, ui.c(MS, "Image3").width.max(8) as usize, ui.c(MS, "Image3").height.max(8) as usize));
            ui.set_picture(MS, "Image3", img);
        }
        None => {
            for c in ["ImageFilenameEdit", "FirstImageEdit", "LastImageEdit", "IncrementEdit", "DestChannelEdit"] {
                ui.set_text(MS, c, "");
            }
            ui.set_picture(MS, "Image3", None);
        }
    }
    app.mapseq.refreshing = false;
}

/// A small preview of a decoded image.
pub fn thumb(im: &crate::image::Image, w: usize, h: usize) -> Bitmap {
    let s = (w as f64 / im.width as f64).min(h as f64 / im.height as f64);
    let (tw, th) = (((im.width as f64 * s) as usize).max(1), ((im.height as f64 * s) as usize).max(1));
    let sh = if im.deep { 8 } else { 0 };
    let mut b = Bitmap::new(tw, th, 0);
    for y in 0..th {
        for x in 0..tw {
            let p = im.pixel((x * im.width / tw).min(im.width - 1), (y * im.height / th).min(im.height - 1));
            b.px[y * tw + x] = 0xFF00_0000 | ((p[0] >> sh) as u32) << 16 | ((p[1] >> sh) as u32) << 8 | (p[2] >> sh) as u32;
        }
    }
    b
}

fn ms_cur(app: &mut Mb3d, ui: &Ui) -> Option<usize> {
    let i = ui.item_index(MS, "MapSequencesList");
    (i >= 0 && (i as usize) < app.mapseq.list.len()).then_some(i as usize)
}

pub fn mapseq_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    match h {
        "FormShow" => {
            app.mapseq.list = crate::maps::map_sequences();
            ui.set_item_index(MS, "MapSequencesList", if app.mapseq.list.is_empty() { -1 } else { 0 });
            ms_refresh(app, ui);
        }
        "MapSequencesListClick" => ms_detail(app, ui),
        "NewBtnClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "Images|*.png;*.jpg;*.jpeg;*.bmp|All files (*.*)|*.*".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
                ..Default::default()
            };
            ui.open_dialog("mapseq:new", &o);
        }
        "DeleteBtnClick" => {
            if let Some(i) = ms_cur(app, ui) {
                app.mapseq.list.remove(i);
                ms_refresh(app, ui);
            }
        }
        "FirstImageEditExit" | "LastImageEditExit" | "IncrementEditExit" | "DestChannelEditExit" | "LoopCheckBoxExit" | "IncrementUpDownClick" | "DestChannelUpDownClick" => {
            if app.mapseq.refreshing {
                return;
            }
            if let Some(i) = ms_cur(app, ui) {
                let q = &mut app.mapseq.list[i];
                q.first = pti(&ui.text(MS, "FirstImageEdit"));
                q.last = pti(&ui.text(MS, "LastImageEdit"));
                q.increment = pti(&ui.text(MS, "IncrementEdit")).max(1);
                q.channel = pti(&ui.text(MS, "DestChannelEdit")).max(1);
                q.looped = ui.checked(MS, "LoopCheckBox");
            }
        }
        "SaveAndExitBtnClick" => {
            if let Err(e) = crate::maps::set_map_sequences(app.mapseq.list.clone()) {
                ui.show_message(&e);
            }
            ui.hide(MS);
        }
        "CancelAndExitBtnClick" => ui.hide(MS),
        _ => {}
    }
}

pub fn mapseq_dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("new", DialogResult::File(Some(p))) = (what, r) {
        let f = p.display().to_string();
        let exists = |n: i32| crate::maps::MapSequence::format_frame(&f, n).map(|s| std::path::Path::new(&s).exists()).unwrap_or(false);
        let mut first = 1;
        while !exists(first) && first < 1000 {
            first += 1;
        }
        if !exists(first) {
            first = 1;
        }
        let mut last = first;
        while exists(last + 1) {
            last += 1;
        }
        let channel = app.mapseq.list.iter().map(|q| q.channel).max().unwrap_or(0) + 1;
        app.mapseq.list.push(crate::maps::MapSequence { channel, filename: f, first, last, increment: 1, looped: true });
        let n = app.mapseq.list.len() as i32 - 1;
        ui.set_item_index(MS, "MapSequencesList", n);
        ms_refresh(app, ui);
        ui.set_item_index(MS, "MapSequencesList", n);
        ms_detail(app, ui);
    }
}

// ---------------------------------------------------------------------------
// FTextBox
// ---------------------------------------------------------------------------

const TB: &str = "FTextBox";

pub fn textbox_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "Button1Click" => ui.hide(TB),
        "Memo1Change" => {
            let t = ui.text(TB, "Memo1");
            if t.lines().count() > 5 && t.trim_end().ends_with('}') {
                if let Ok((raw, title)) = crate::m3p::raw_from_text(&t) {
                    if let Ok(m) = crate::m3p::parse(&raw) {
                        app.scene = m.scene;
                        app.title = title;
                        app.eng.clear();
                        app.scene_to_forms(ui);
                        super::main_form::set_caption(app, ui);
                        app.message(ui, "Parameters loaded, press \"Calculate 3D\" to render.");
                        super::main_form::show_image(app, ui);
                        ui.hide(TB);
                    }
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// ZBuf16BitGenFrm
// ---------------------------------------------------------------------------

const ZB: &str = "ZBuf16BitGenFrm";

/// MB3D's 16 bit z-buffer (`MakeZbufPGM`): gray = (z * scale / 1000 +
/// offset) * 65535 from the distance z of each pixel; also the value ranges.
pub fn zbuf16(app: &Mb3d, offset: f64, scale: f64, invert: bool) -> Option<(usize, usize, Vec<u16>, [f64; 6])> {
    let b = app.eng.base()?;
    let (zcm, zc) = (b.params.zc_mul, b.params.zcorr);
    let mut info = [f64::MAX, f64::MIN, f64::MAX, f64::MIN, f64::MAX, f64::MIN];
    let z: Vec<u16> = b
        .post
        .iter()
        .map(|s| {
            if s.is_background() {
                return 0;
            }
            let iz = (s.zpos_fine >> 8) as f64;
            let zz = (((8388351.5 - iz) / zcm + 1.0).powi(2) - 1.0) / zc;
            let g = ((zz * scale / 1000.0 + offset) * 65535.0).round();
            let c = g.clamp(0.0, 65535.0);
            let c = if invert { 65535.0 - c } else { c };
            info[0] = info[0].min(zz);
            info[1] = info[1].max(zz);
            info[2] = info[2].min(g);
            info[3] = info[3].max(g);
            info[4] = info[4].min(c);
            info[5] = info[5].max(c);
            c as u16
        })
        .collect();
    Some((b.w, b.h, z, info))
}

fn zb_values(ui: &Ui) -> (f64, f64, bool) {
    (ptf(&ui.text(ZB, "ZOffsetEdit")), ptf(&ui.text(ZB, "ZScaleEdit")), ui.checked(ZB, "InvertZBufferCBx"))
}

fn zb_refresh(app: &mut Mb3d, ui: &mut Ui) {
    let (o, s, inv) = zb_values(ui);
    let Some((w, h, z, info)) = zbuf16(app, o, s, inv) else {
        ui.set_text(ZB, "InfoMemo", &tr("Calculate the image first."));
        return;
    };
    let mut b = Bitmap::new(w, h, 0);
    for (p, v) in b.px.iter_mut().zip(&z) {
        let g = (*v >> 8) as u32;
        *p = 0xFF00_0000 | g << 16 | g << 8 | g;
    }
    let c = ui.cm(ZB, "MainPreviewImg");
    c.width = w as i32;
    c.height = h as i32;
    c.picture = Some(b);
    ui.set_text(
        ZB,
        "InfoMemo",
        &format!(
            "Raw Values:\n  {}..\n  {}\nMapped Values:\n  {}..\n  {}\nClamped Values:\n  {}..\n  {}\n",
            fts(info[0]),
            fts(info[1]),
            info[2] as i64,
            info[3] as i64,
            info[4] as i64,
            info[5] as i64
        ),
    );
}

pub fn zbuf_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "RefreshPreviewBtnClick" | "ZOffsetEditChange" | "ZScaleEditChange" | "InvertZBufferCBxClick" => zb_refresh(app, ui),
        "ZOffsetEditUpDownClick" | "ZScaleEditUpDownClick" => {
            if let crate::vcl::Ev::UpDown { up } = e.ev {
                let ed = if e.handler.starts_with("ZOffset") { "ZOffsetEdit" } else { "ZScaleEdit" };
                let v = parse_float(&ui.text(ZB, ed)).unwrap_or(0.0) + if up { 0.01 } else { -0.01 };
                ui.set_text(ZB, ed, &fts(v));
                zb_refresh(app, ui);
            }
        }
        "GuessBtnClick" => {
            let (o, s, inv) = zb_values(ui);
            if let Some((_, _, _, info)) = zbuf16(app, o, s, inv) {
                let (mn, mx) = (info[0], info[1]);
                if mx > mn {
                    let (zmin, zmax) = (1000.0, 65535.0 - 1000.0);
                    let off = (mx * zmin - zmax * mn) / (mx - mn) / 65535.0;
                    let sc = 200.0 / 13107.0 * (zmax - zmin) / (mx - mn);
                    ui.set_text(ZB, "ZOffsetEdit", &fts(off));
                    ui.set_text(ZB, "ZScaleEdit", &fts(sc));
                    zb_refresh(app, ui);
                }
            }
        }
        "SaveImgBtnClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "PNG 16 bit (*.png)|*.png|PGM 16 bit (*.pgm)|*.pgm".into(),
                default_ext: "png".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
                ..Default::default()
            };
            ui.save_dialog("zbuf:save", &o);
        }
        _ => {}
    }
}

pub fn zbuf_dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("save", DialogResult::File(Some(p))) = (what, r) {
        let (o, s, inv) = zb_values(ui);
        let Some((w, h, z, _)) = zbuf16(app, o, s, inv) else { return };
        let res = if p.extension().map(|e| e.eq_ignore_ascii_case("pgm")) == Some(true) {
            let mut d = format!("P5\n{w} {h}\n65535\n").into_bytes();
            for v in &z {
                d.extend_from_slice(&v.to_be_bytes());
            }
            std::fs::write(p, d).map_err(|e| e.to_string())
        } else {
            crate::png::write_gray16(&p.to_string_lossy(), w, h, &z).map_err(|e| e.to_string())
        };
        match res {
            Ok(()) => app.message(ui, &format!("Saved {}", p.display())),
            Err(e) => app.message(ui, &format!("{}: {e}", p.display())),
        }
    }
}
