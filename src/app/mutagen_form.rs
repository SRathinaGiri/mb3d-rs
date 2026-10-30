//! MutaGen (mutagen/MutaGenGUI.pas): random variations of the parameters.

use super::Mb3d;
use crate::mutagen::{Member, MutationConfig, Rng, TREE};
use crate::vcl::bitmap::Bitmap;
use crate::vcl::lang::{tr, trf};
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const F: &str = "MutaGenFrm";
const TBAR_SCALE: f64 = 1000.0;

/// `CreatePanelList`: centre (x, y up) of each panel in a 5 x 4 grid, in
/// `TREE` order.
const POS: [(f64, f64); 15] = [
    (0.0, -1.0),
    (-0.5, 0.0),
    (0.5, 0.0),
    (-1.5, -0.5),
    (1.5, -0.5),
    (-1.5, 0.5),
    (1.5, 0.5),
    (-2.0, -1.5),
    (-2.0, 1.5),
    (1.0, -1.5),
    (1.0, 1.5),
    (-1.0, -1.5),
    (-1.0, 1.5),
    (2.0, -1.5),
    (2.0, 1.5),
];

/// The link lines (`AddPanelLink`): from panel, from x/y, to panel, to x/y,
/// vertical (top/bottom) or horizontal (left/right) connection.
const LINKS: [(usize, f64, f64, usize, f64, f64, bool); 14] = {
    const D: f64 = 0.05;
    [
        (0, 0.5 - D, 1.0, 1, 0.5, 0.0, true),
        (1, 0.0, 0.5 - D, 3, 1.0, 0.5, false),
        (3, 0.5 - D, 0.0, 7, 0.5, 1.0, true),
        (3, 0.5 + D, 0.0, 11, 0.5, 1.0, true),
        (1, 0.0, 0.5 + D, 5, 1.0, 0.5, false),
        (5, 0.5 - D, 1.0, 8, 0.5, 0.0, true),
        (5, 0.5 + D, 1.0, 12, 0.5, 0.0, true),
        (0, 0.5 + D, 1.0, 2, 0.5, 0.0, true),
        (2, 1.0, 0.5 - D, 4, 0.0, 0.5, false),
        (4, 0.5 - D, 0.0, 9, 0.5, 1.0, true),
        (4, 0.5 + D, 0.0, 13, 0.5, 1.0, true),
        (2, 1.0, 0.5 + D, 6, 0.0, 0.5, false),
        (6, 0.5 - D, 1.0, 10, 0.5, 0.0, true),
        (6, 0.5 + D, 1.0, 14, 0.5, 0.0, true),
    ]
};

fn panel_name(i: usize) -> String {
    format!("Panel_{}", TREE[i].0.replace('.', "_"))
}

fn image_name(i: usize) -> String {
    format!("Image_{}", TREE[i].0.replace('.', "_"))
}

#[derive(Clone)]
struct GenMember {
    member: Member,
    img: Bitmap,
}

#[derive(Default)]
struct Shared {
    /// finished members (index, member, picture)
    done: Vec<(usize, Member, Bitmap)>,
    finished: bool,
    error: String,
}

pub struct State {
    generations: Vec<Vec<Option<GenMember>>>,
    current: usize,
    running: bool,
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
    layout_size: (i32, i32),
    uuid: u32,
}

impl Default for State {
    fn default() -> Self {
        State {
            generations: Vec::new(),
            current: 0,
            running: false,
            shared: Arc::new(Mutex::new(Shared::default())),
            cancel: Arc::new(AtomicBool::new(false)),
            layout_size: (0, 0),
            uuid: 0,
        }
    }
}

/// `DoLayout`: the panels in MainPnl and the link lines behind them.
fn layout(app: &mut Mb3d, ui: &mut Ui) {
    let r = ui.ctl_rect(F, "MainPnl");
    if (r.w, r.h) == app.mutagen.layout_size || r.w < 50 || r.h < 50 {
        return;
    }
    app.mutagen.layout_size = (r.w, r.h);
    const B: i32 = 10;
    let (rw, rh) = (r.w - 2 * B, r.h - 2 * B);
    let (cx, cy) = (B + rw / 2, B + rh / 2);
    let (sx, sy) = (rw as f64 / 5.0, rh as f64 / 4.0);
    let mut rects = [(0i32, 0i32, 0i32, 0i32); 15];
    for (i, &(px, py)) in POS.iter().enumerate() {
        let w = (0.9 * sx).round() as i32;
        let h = (0.8 * sy).round() as i32;
        let l = (cx as f64 + px * sx).round() as i32 - w / 2;
        let t = (cy as f64 - py * sy).round() as i32 - h / 2;
        rects[i] = (l, t, w, h);
        let c = ui.cm(F, &panel_name(i));
        c.set_bounds(l, t, w, h);
        c.caption = if i == 0 { tr("@MB:Root") } else { trf("@MB:Mutation", &[&&TREE[i].0[2..]]) };
        // the picture fills the panel (alClient with 5 pixel margins), the
        // caption below it
        ui.cm(F, &image_name(i)).set_bounds(5, 5, (w - 10).max(8), (h - 10 - 14).max(8));
    }
    // the lines (`TPanelLinkLine`), red
    let mut b = Bitmap::new(r.w.max(1) as usize, r.h.max(1) as usize, 0);
    let mut line = |x1: i32, y1: i32, x2: i32, y2: i32| {
        let n = (x2 - x1).abs().max((y2 - y1).abs()).max(1);
        for k in 0..=n {
            let x = x1 + (x2 - x1) * k / n;
            let y = y1 + (y2 - y1) * k / n;
            if x >= 0 && y >= 0 && (x as usize) < b.w && (y as usize) < b.h {
                b.px[y as usize * b.w + x as usize] = 0xFFFF_0000;
            }
        }
    };
    for &(a, ax, ay, z, zx, zy, vertical) in LINKS.iter() {
        let p = |i: usize, fx: f64, fy: f64| {
            let (l, t, w, h) = rects[i];
            (l + (fx * w as f64).round() as i32, t + ((1.0 - fy) * h as f64).round() as i32)
        };
        let (x1, y1) = p(a, ax, ay);
        let (x2, y2) = p(z, zx, zy);
        if vertical {
            let ym = (y1 as f64 + (y2 - y1) as f64 * 0.5).round() as i32;
            line(x1, y1, x1, ym);
            line(x1, ym, x2, ym);
            line(x2, ym, x2, y2);
        } else {
            let xm = (x1 as f64 + (x2 - x1) as f64 * 0.5).round() as i32;
            line(x1, y1, xm, y1);
            line(xm, y1, xm, y2);
            line(xm, y2, x2, y2);
        }
    }
    let f = ui.fm(F);
    let pnl = f.id("MainPnl").unwrap();
    let id = match f.id("LinkLines") {
        Some(i) => i,
        None => {
            let c = crate::vcl::control::Control::new("LinkLines", "TImage");
            let i = f.add_control(pnl, c);
            // behind the panels
            f.ctl[pnl].children.retain(|&k| k != i);
            f.ctl[pnl].children.insert(0, i);
            i
        }
    };
    f.ctl[id].set_bounds(0, 0, r.w, r.h);
    f.ctl[id].transparent = true;
    f.ctl[id].picture = Some(b);
    f.dirty = true;
}

/// The size of the panel pictures (`ImageWidth`, `ImageHeight`).
fn image_size(ui: &mut Ui) -> (usize, usize) {
    let r = ui.ctl_rect(F, "Image_1");
    ((r.w.max(8)) as usize, (r.h.max(8)) as usize)
}

fn blank(ui: &mut Ui) -> Bitmap {
    let (w, h) = image_size(ui);
    Bitmap::new(w, h, 0xFF00_0000)
}

fn refresh_generation_label(app: &Mb3d, ui: &mut Ui) {
    let t = trf("@MB:GenerationOf", &[&(app.mutagen.current + 1), &app.mutagen.generations.len()]);
    ui.set_text(F, "GenerationEdit", &t);
}

fn refresh_mutate_caption(app: &Mb3d, ui: &mut Ui) {
    ui.set_caption(F, "MutateBtn", &if app.mutagen.running { tr("@MB:Cancel") } else { tr("@MB:MutateExclamation") });
}

fn enable_controls(app: &Mb3d, ui: &mut Ui) {
    let r = app.mutagen.running;
    for c in ["GenerationBtn", "ClearPrevGenerations", "DisableAllBtn"] {
        ui.set_enabled(F, c, !r);
    }
}

/// `ReDisplayCurrGeneration`
fn redisplay(app: &Mb3d, ui: &mut Ui) {
    let Some(g) = app.mutagen.generations.get(app.mutagen.current) else { return };
    let g = g.clone();
    for (i, m) in g.iter().enumerate() {
        let b = m.as_ref().map(|m| m.img.clone()).unwrap_or_else(|| blank(ui));
        ui.set_picture(F, &image_name(i), Some(b));
    }
}

fn config(ui: &Ui) -> MutationConfig {
    let p = |c: &str| ui.position(F, c) as f64 / TBAR_SCALE;
    MutationConfig {
        formula_weight: p("ModifyFormulaWeightTBar"),
        params_weight: p("ModifyParamsWeightTBar"),
        params_strength: p("ModifyParamsStrengthTBar"),
        julia_weight: p("ModifyJuliaModeWeightTBar"),
        julia_strength: p("ModifyJuliaModeStrengthTBar"),
        iterations_weight: p("ModifyIterationCountWeightTBar"),
        iterations_strength: p("ModifyIterationCountStrengthTBar"),
        ..MutationConfig::default()
    }
}

/// `InitOptionsPanel`
fn init_options(ui: &mut Ui) {
    let c = MutationConfig::default();
    for (n, v) in [
        ("ModifyFormulaWeightTBar", c.formula_weight),
        ("ModifyParamsWeightTBar", c.params_weight),
        ("ModifyParamsStrengthTBar", c.params_strength),
        ("ModifyJuliaModeWeightTBar", c.julia_weight),
        ("ModifyJuliaModeStrengthTBar", c.julia_strength),
        ("ModifyIterationCountWeightTBar", c.iterations_weight),
        ("ModifyIterationCountStrengthTBar", c.iterations_strength),
    ] {
        let t = ui.cm(F, n);
        if t.max < TBAR_SCALE as i64 {
            t.max = TBAR_SCALE as i64;
        }
        t.set_position((TBAR_SCALE * v).round() as i64);
    }
}

fn to_bitmap(rgb: &[u8], w: usize, h: usize, bw: usize, bh: usize) -> Bitmap {
    // centred on black in the panel picture size
    let mut b = Bitmap::new(bw, bh, 0xFF00_0000);
    let (ox, oy) = (bw.saturating_sub(w) / 2, bh.saturating_sub(h) / 2);
    for y in 0..h.min(bh) {
        for x in 0..w.min(bw) {
            let o = (y * w + x) * 3;
            b.px[(y + oy) * bw + x + ox] = 0xFF00_0000 | (rgb[o] as u32) << 16 | (rgb[o + 1] as u32) << 8 | rgb[o + 2] as u32;
        }
    }
    b
}

/// The window is opened (again): the main window's parameters become the
/// root of a first generation and its preview is rendered (MB3D's
/// `RestartFromMain` / `CreateInitialSet`, which it never called: its root
/// tile stayed black until the first mutation).  When the parameters are
/// still those the generations started from, nothing changes.
pub fn opening(app: &mut Mb3d, ui: &mut Ui) {
    if app.mutagen.running || ui.showing(F) {
        return;
    }
    app.make_scene(ui);
    if let Some(root) = app.mutagen.generations.first().and_then(|g| g.first().cloned().flatten()) {
        if root.member.scene.to_text() == app.scene.to_text() {
            return;
        }
    }
    app.mutagen.generations.clear();
    app.mutagen.current = 0;
    for i in 0..TREE.len() {
        let b = blank(ui);
        ui.set_picture(F, &image_name(i), Some(b));
        ui.set_caption(F, &panel_name(i), "");
    }
    create_mutation(app, ui, 0, true);
}

/// The parameters of the main window as the root (`CreateInitialSet`).
fn initial_from_main(app: &mut Mb3d, ui: &mut Ui) -> Option<GenMember> {
    app.make_scene(ui);
    let s = app.scene.clone();
    Some(GenMember { member: Member { scene: s, probe: Vec::new(), coverage: 0.0 }, img: blank(ui) })
}

/// `CreateMutation(Sender)`: a new generation from the member of a panel;
/// `root_only` renders just the root (`CreateInitialSet`).
fn create_mutation(app: &mut Mb3d, ui: &mut Ui, from: usize, root_only: bool) {
    if app.mutagen.running {
        app.mutagen.cancel.store(true, Ordering::SeqCst);
        return;
    }
    // the picture size follows the window's layout
    layout(app, ui);
    let initial = if app.mutagen.generations.is_empty() {
        initial_from_main(app, ui)
    } else {
        app.mutagen.generations.get(app.mutagen.current).and_then(|g| g.get(from).cloned().flatten())
    };
    let Some(initial) = initial else { return };
    let has_img = !app.mutagen.generations.is_empty();
    app.mutagen.running = true;
    enable_controls(app, ui);
    refresh_mutate_caption(app, ui);
    // ClearPanels + AddGeneration
    for i in 1..TREE.len() {
        let b = blank(ui);
        ui.set_picture(F, &image_name(i), Some(b));
    }
    app.mutagen.generations.push(vec![None; TREE.len()]);
    app.mutagen.current = app.mutagen.generations.len() - 1;
    refresh_generation_label(app, ui);
    {
        let c = ui.cm(F, "ProgressBar");
        c.max = if root_only { 1 } else { TREE.len() as i64 };
        c.position = 0;
    }
    let cfg = config(ui);
    let (bw, bh) = image_size(ui);
    let threads = crate::render::thread_count(&app.scene);
    let cancel = app.mutagen.cancel.clone();
    cancel.store(false, Ordering::SeqCst);
    let shared = app.mutagen.shared.clone();
    *shared.lock().unwrap() = Shared::default();
    let waker = ui.waker();
    std::thread::spawn(move || {
        let mut rng = Rng::from_time();
        let mut out: Vec<Member> = Vec::new();
        let render = |m: &Member| -> Bitmap {
            match crate::mutagen::preview(&m.scene, bw, bh, false, threads) {
                Ok((rgb, w, h)) => to_bitmap(&rgb, w, h, bw, bh),
                Err(e) => {
                    crate::elog!("MutaGen preview: {e}");
                    Bitmap::new(bw, bh, 0xFF00_0000)
                }
            }
        };
        let r = (|| -> Result<(), String> {
            for (i, (_, p)) in TREE.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) || (root_only && i > 0) {
                    return Ok(());
                }
                let (m, img) = match p {
                    None => {
                        let mut m = initial.member.clone();
                        if cfg.probing && m.probe.is_empty() {
                            m = crate::mutagen::root(&cfg, &m.scene, threads);
                        }
                        let img = if has_img { initial.img.clone() } else { render(&m) };
                        (m, img)
                    }
                    Some(p) => {
                        let m = crate::mutagen::breed(&cfg, &out[*p], &mut rng, threads)?;
                        if cancel.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        let img = render(&m);
                        (m, img)
                    }
                };
                out.push(m.clone());
                shared.lock().unwrap().done.push((i, m, img));
                if let Some(w) = &waker {
                    w.wake();
                }
            }
            Ok(())
        })();
        let mut s = shared.lock().unwrap();
        s.finished = true;
        if let Err(e) = r {
            s.error = e;
        }
        drop(s);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.showing(F) {
        return;
    }
    layout(app, ui);
    if !app.mutagen.running {
        return;
    }
    let (done, finished, err) = {
        let mut s = app.mutagen.shared.lock().unwrap();
        (std::mem::take(&mut s.done), s.finished, std::mem::take(&mut s.error))
    };
    let gi = app.mutagen.generations.len() - 1;
    for (i, m, img) in done {
        if i == 0 {
            app.mutagen.uuid += 1;
        }
        if app.mutagen.current == gi {
            ui.set_picture(F, &image_name(i), Some(img.clone()));
        }
        app.mutagen.generations[gi][i] = Some(GenMember { member: m, img });
        ui.cm(F, "ProgressBar").position += 1;
    }
    if !err.is_empty() {
        app.message(ui, &trf("@MB:MutaGen2", &[&err]));
    }
    if finished {
        app.mutagen.running = false;
        ui.cm(F, "ProgressBar").position = 0;
        refresh_mutate_caption(app, ui);
        enable_controls(app, ui);
    }
}

/// The member of the panel a popup menu was opened for.
fn popup_member(app: &Mb3d, ui: &Ui) -> Option<(usize, Member)> {
    let owner = ui.popup_component(F);
    let i = (0..TREE.len()).find(|&i| owner == panel_name(i) || owner == image_name(i))?;
    let m = app.mutagen.generations.get(app.mutagen.current)?.get(i)?.as_ref()?;
    Some((i, m.member.clone()))
}

fn caption(i: usize, gen: usize) -> String {
    format!("MutaGen G{}_{}", gen + 1, TREE[i].0)
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormCreate" => {
            init_options(ui);
            refresh_mutate_caption(app, ui);
            ui.cm(F, "ProgressBar").position = 0;
        }
        "FormCloseQuery" => {
            if !app.mutagen.running {
                ui.close_ok(F);
            } else {
                ui.close_cancel(F);
            }
        }
        "MutateBtnClick" => create_mutation(app, ui, 0, false),
        "Panel_1DblClick" => {
            let s = e.sender.as_str();
            if let Some(i) = (0..TREE.len()).find(|&i| s == panel_name(i) || s == image_name(i)) {
                create_mutation(app, ui, i, false);
            }
        }
        "DisableAllBtnClick" => {
            for c in ["ModifyFormulaWeightTBar", "ModifyParamsWeightTBar", "ModifyJuliaModeWeightTBar", "ModifyIterationCountWeightTBar"] {
                ui.set_position(F, c, 0);
            }
        }
        "GenerationBtnClick" => {
            let up = matches!(e.ev, Ev::UpDown { up: true, .. });
            let n = app.mutagen.generations.len();
            if up && app.mutagen.current + 1 < n {
                app.mutagen.current += 1;
            } else if !up && app.mutagen.current > 0 {
                app.mutagen.current -= 1;
            } else {
                return;
            }
            redisplay(app, ui);
            refresh_generation_label(app, ui);
        }
        "ClearPrevGenerationsClick" => {
            if app.mutagen.current > 0 && app.mutagen.current < app.mutagen.generations.len() {
                ui.confirm("mutagen:clear", &tr("@MB:DoYouReallyWantToClear"));
            }
        }
        "ToClipboardItmClick" => {
            let Some((i, m)) = popup_member(app, ui) else { return ui.show_message(&tr("@MB:NoParamsToSendToMain")) };
            let t = crate::m3p::raw_to_text(&crate::m3p::write(&m.scene), &caption(i, app.mutagen.current));
            crate::vcl::clipboard::set(&t);
        }
        "SendtoMainItmClick" => {
            if app.main.calculating {
                return ui.show_message(&tr("@MB:TheMainEditorIsStillRendering"));
            }
            let Some((i, m)) = popup_member(app, ui) else { return ui.show_message(&tr("@MB:NoParamsToSendToMain")) };
            let t = crate::m3p::raw_to_text(&crate::m3p::write(&m.scene), &caption(i, app.mutagen.current));
            crate::vcl::clipboard::set(&t);
            app.scene = m.scene;
            app.eng.clear();
            app.scene_to_forms(ui);
            app.title = caption(i, app.mutagen.current);
            super::main_form::set_caption(app, ui);
        }
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    if let ("clear", DialogResult::Button(crate::vcl::form::MR_YES)) = (what, r) {
        let c = app.mutagen.current;
        app.mutagen.generations.drain(0..c);
        app.mutagen.current = 0;
        refresh_generation_label(app, ui);
    }
}
