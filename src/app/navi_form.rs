//! The navigator (Navigator.pas, `TFNavigator`): its own copy of the
//! parameters (`NaviHeader`) with a fast progressive preview; walking,
//! sliding and looking with steps scaled by the distance estimate at the
//! camera, the mouse look mode, the adjustment panel (julia, formula, 4D
//! rotation and other values) and sending the view back to the main window.

use super::engine::{Engine, Job};
use super::util::{fts, fts_single, parse_float, pti, ptf};
use super::{Mb3d, MAIN};
use crate::lighting::Lighting;
use crate::math::{build_rot_matrix, len, mat_mul, normalize, Vec3};
use crate::scene::Scene;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{Ev, Event, MouseButton, Ui};

const F: &str = "FNavigator";
const PID180: f64 = std::f64::consts::PI / 180.0;

pub struct State {
    pub scene: Scene,
    pub eng: Engine,
    pub ver: u64,
    pub orig: (i32, i32),
    pub de_mul: f64,
    pub moving: bool,
    pub look: bool,
    pub double_click: bool,
    pub quality: i32,
    pub lightness: f64,
    pub first: bool,
    pub vals: [f64; 14],
    pub pos0: [f64; 14],
    pub range: [f64; 14],
    pub vtype: [i32; 14],
    pub sub_top: usize,
    pub focused: usize,
    pub findex: (usize, usize),
    pub presets: [Option<Lighting>; 3],
    pub storing: bool,
    pub dynfog_changed: bool,
    pub destop_changed: bool,
    pub adjust_first: bool,
    pub waker_set: bool,
    /// the view taken from the main window and "Fixed zoom" then ("Reset view")
    pub start: Option<Scene>,
    pub start_fixed: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            scene: Scene::default(),
            eng: Engine::new(),
            ver: 0,
            orig: (640, 480),
            de_mul: 1.0,
            moving: false,
            look: false,
            double_click: true,
            quality: 1,
            lightness: 1.0,
            first: true,
            vals: [0.0; 14],
            pos0: [0.0; 14],
            range: [1.0; 14],
            vtype: [0; 14],
            sub_top: 0,
            focused: 0,
            findex: (0, 0),
            presets: [None, None, None],
            storing: false,
            dynfog_changed: false,
            destop_changed: false,
            adjust_first: true,
            waker_set: false,
            start: None,
            start_fixed: false,
        }
    }
}

fn is_int_type(t: i32) -> bool {
    matches!(t, 2 | 10 | 20)
}

fn is_angle_type(t: i32) -> bool {
    matches!(t, 3..=6 | 12)
}

fn length_of_size(sc: &Scene) -> f64 {
    ((sc.width as f64).powi(2) + (sc.height as f64).powi(2)).sqrt()
}

fn unit_rows(sc: &Scene) -> [Vec3; 3] {
    [normalize(sc.vgrads[0]), normalize(sc.vgrads[1]), normalize(sc.vgrads[2])]
}

fn cam_pos(sc: &Scene) -> Vec3 {
    let vz = normalize(sc.vgrads[2]);
    let d = sc.z_start - sc.mid[2];
    [sc.mid[0] + vz[0] * d, sc.mid[1] + vz[1] * d, sc.mid[2] + vz[2] * d]
}

/// `GetLocalAbsoluteDE`: the median of the distance estimates around the
/// camera (absolute units); None when it cannot be estimated.
fn local_de(sc: &Scene) -> Option<f64> {
    let mut s = sc.clone();
    s.slice_2d = 0;
    let p = crate::calc::CalcParams::new(&s).ok()?;
    let sw = s.step_width();
    let u = unit_rows(&s);
    let ct = cam_pos(&s);
    let mut list = Vec::new();
    let mut dmul = 1.0;
    while list.len() <= 3 && dmul <= 40.0 {
        for i in 0..9 {
            let pos = if i == 8 {
                ct
            } else {
                let o = [((i & 1) * 32) as f64 - 16.0, ((i & 2) * 16) as f64 - 16.0, ((i & 4) * 8) as f64 - 16.0];
                let mut q = ct;
                for k in 0..3 {
                    for a in 0..3 {
                        q[a] += u[k][a] * o[k] * dmul * sw;
                    }
                }
                q
            };
            let (d, _) = crate::calc::de_at(&p, pos);
            if d.is_finite() && d > 0.0 && d < 1e15 * sw && list.len() < 11 {
                list.push(d / sw);
            }
        }
        dmul *= 2.0;
    }
    if list.is_empty() {
        return None;
    }
    list.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let m = list[((list.len() - 1) as f64 * 0.6).round() as usize];
    let r = m.clamp(3.0, 20000.0) * sw;
    Some(r.min(len(&ct).max(2.0)))
}

// ---------------------------------------------------------------------------
// header handling
// ---------------------------------------------------------------------------

fn navi_scale(ui: &Ui) -> f64 {
    let s = ui.c(F, "NaviSizeCmb").item_text();
    (s.trim_end_matches('%').parse::<f64>().unwrap_or(100.0) / 100.0).clamp(0.2, 2.0)
}

/// `SetHeaderSize`: the preview size (640 pixels wide at 100 %).
fn set_header_size(app: &mut Mb3d, ui: &Ui) {
    let st = &mut app.navi;
    let p2h = if ui.visible(F, "Panel2") { ui.c(F, "Panel2").height } else { 0 };
    let hmax = (760 - ui.c(F, "Panel1").height - p2h).max(200);
    let (ow, oh) = (st.orig.0.max(1), st.orig.1.max(1));
    let (mut w, mut h);
    if oh * 640 / ow > hmax {
        h = (hmax + 7) & !7;
        w = ((h * ow / oh + 4) & !7).clamp(8, 640);
    } else {
        w = 640;
        h = ((oh * 640 / ow + 4).min(hmax) & !7).max(8);
    }
    let s = navi_scale(ui);
    w = (w as f64 * s).round() as i32;
    h = (h as f64 * s).round() as i32;
    st.scene.width = w;
    st.scene.height = h;
}

/// `TransformNHeader`: the camera onto the middle point.
fn transform_header(app: &mut Mb3d, ui: &mut Ui) {
    let sc = &mut app.navi.scene;
    app.navi.orig = (sc.width, sc.height);
    set_header_size(app, ui);
    let sc = &mut app.navi.scene;
    sc.bin_search_steps = sc.bin_search_steps.min(7);
    sc.de_stop = sc.de_stop.max(0.01);
    ui.set_text(F, "Edit3", &fts(sc.fov_y));
    ui.set_item_index(F, "RadioGroup2", sc.optic as i32);
    let vz = normalize(sc.vgrads[2]);
    let d = sc.z_start - sc.mid[2];
    for k in 0..3 {
        sc.mid[k] += vz[k] * d;
    }
    sc.z_end = sc.z_end - sc.z_start + sc.mid[2];
    sc.z_start = sc.mid[2];
    app.navi.de_mul = 1.0;
    app.navi.moving = false;
    let sc = app.navi.scene.clone();
    let fp = match local_de(&sc) {
        Some(de) => ((sc.z_end - sc.z_start) / de).clamp(1.0, 10000.0),
        None => 50.0,
    };
    ui.set_text(F, "Edit4", &format!("{fp:.1}"));
}

/// `SetZoom`: the zoom from the local DE, the far plane.
fn set_zoom(app: &mut Mb3d, ui: &mut Ui) {
    let fixed = ui.checked(F, "CheckBox1");
    let st = &mut app.navi;
    let sc = &mut st.scene;
    let mut de = None;
    if !fixed {
        if let Some(d) = local_de(sc) {
            let d = d * st.de_mul;
            sc.zoom = length_of_size(sc) * 2.0 / (d * sc.width as f64);
            de = Some(d);
        }
    }
    let de = de.unwrap_or_else(|| sc.step_width() * length_of_size(sc));
    let fp = ptf(&ui.text(F, "Edit4")).clamp(1.0, 90000.0);
    sc.z_end = sc.z_start + (de * fp).min(9999.0);
    st.de_mul = (st.de_mul - 1.0) * 0.8 + 1.0;
}

/// `SetWindowSize`: window and image positions for the preview size.
fn set_window_size(app: &mut Mb3d, ui: &mut Ui, panel2: bool) {
    ui.set_visible(F, "Panel2", panel2);
    set_header_size(app, ui);
    let (w, h) = (app.navi.scene.width, app.navi.scene.height);
    let p2h = if panel2 { ui.c(F, "Panel2").height } else { 0 };
    let p1h = ui.c(F, "Panel1").height;
    // MB3D: Max(580, height + Panel1 + Panel2): below 580 the bottom panel
    // pushed the bar with its open/close arrows up, away from under the
    // mouse (they looked gone); the window grows by the panel instead, so
    // the bar stays where it is
    let ch = (h + p1h).max(580) + p2h;
    // MB3D: Max(646 (+ Panel3), width + Panel5.Width) kept room for the
    // adjustment panel also while it is hidden, which left a gap between
    // the image and the panel; the panel is added only when it is shown
    let pw = if ui.visible(F, "Panel3") { ui.c(F, "Panel3").width } else { 0 };
    let cw = 646.max(w) + pw;
    ui.set_client_size(F, cw, ch);
    let p1top = ch - p2h - p1h;
    ui.cm(F, "Panel1").top = p1top;
    ui.cm(F, "Panel2").top = ch - p2h;
    let (x, y) = if w > 640 { (0, 0) } else { ((640 - w) / 2, (p1top - h) / 2) };
    let c = ui.cm(F, "Image1");
    c.set_bounds(x, y, w, h);
    c.stretch = true;
    c.anchored = false;
}

fn new_calc(app: &mut Mb3d, ui: &mut Ui) {
    let st = &mut app.navi;
    let mut sc = st.scene.clone();
    let hiq = ui.checked(F, "CheckBox4");
    sc.z_step_div = 0.7 - if hiq { (st.quality + 6) as f64 * 0.05 } else { 0.0 };
    sc.raystep_limiter = sc.raystep_limiter.max(sc.z_step_div * 0.5);
    sc.fov_y = ptf(&ui.text(F, "Edit3"));
    sc.optic = match ui.item_index(F, "RadioGroup2") {
        1 => crate::scene::CameraOptic::Planar,
        2 => crate::scene::CameraOptic::Panorama,
        _ => crate::scene::CameraOptic::Common,
    };
    st.scene.fov_y = sc.fov_y;
    st.scene.optic = sc.optic;
    // MB3D's navigator renders with simpler lighting: no shadows, no
    // ambient occlusion, no reflections, no depth of field
    sc.shadows = None;
    sc.ao = None;
    sc.deao = None;
    sc.dof = None;
    sc.mc.reflections = false;
    sc.vol_light = None;
    sc.normals_on_zbuf = false;
    sc.stereo_mode = 0;
    sc.slice_2d = 0;
    ui.set_caption(F, "Label24", &crate::app::util::float_general(sc.zoom, 4));
    let red = sc.zoom > 1e13;
    ui.cm(F, "Label24").font = red.then(|| crate::vcl::font::Font { color: 0xFFFF_0000, custom_color: true, ..Default::default() });
    st.eng.start(Job::Preview { scene: sc });
}

/// The navigator's image (polled from the idle loop).
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    if !app.navi.waker_set {
        if let Some(w) = ui.waker() {
            app.navi.eng.set_waker(Some(w));
            app.navi.waker_set = true;
        }
    }
    let (ver, rgb, w, h, err) = {
        let o = app.navi.eng.0.out.lock().unwrap();
        (o.ver, o.rgb.clone(), o.w, o.h, o.error.clone())
    };
    if ver == app.navi.ver {
        return;
    }
    app.navi.ver = ver;
    if !err.is_empty() {
        app.message(ui, &format!("Navigator: {err}"));
    }
    let Some(rgb) = rgb else { return };
    let l = app.navi.lightness;
    let px: Vec<u8> = if (l - 1.0).abs() > 1e-3 { rgb.iter().map(|&v| (v as f64 * l).min(255.0) as u8).collect() } else { (*rgb).clone() };
    let mut b = Bitmap::from_rgb(w, h, &px);
    paint_guides(ui, &mut b);
    ui.set_picture(F, "Image1", Some(b));
}

/// `PaintGuides`: thirds and the golden ratio lines over the preview.
fn paint_guides(ui: &Ui, b: &mut Bitmap) {
    if !ui.checked(F, "ShowGuidesCBx") {
        return;
    }
    let (w, h) = (b.w, b.h);
    let mut vline = |x: usize, c: u32| {
        for y in 0..h {
            b.px[y * w + x.min(w - 1)] = c;
        }
    };
    for f in [1.0 / 3.0, 2.0 / 3.0] {
        vline((w as f64 * f) as usize, 0xFFFF_FF00);
    }
    for f in [0.382, 0.618] {
        vline((w as f64 * f) as usize, 0xFFFF_0000);
    }
    vline(w / 2, 0xFFFF_FFFF);
    let mut hline = |y: usize, c: u32| {
        for x in 0..w {
            b.px[y.min(h - 1) * w + x] = c;
        }
    };
    for f in [1.0 / 3.0, 2.0 / 3.0] {
        hline((h as f64 * f) as usize, 0xFFFF_FF00);
    }
    for f in [0.382, 0.618] {
        hline((h as f64 * f) as usize, 0xFFFF_0000);
    }
    hline(h / 2, 0xFFFF_FFFF);
}

/// `SpeedButton11Click`: the main parameters into the navigator.
fn insert_main(app: &mut Mb3d, ui: &mut Ui) {
    app.navi.eng.stop();
    app.make_scene(ui);
    app.navi.scene = app.scene.clone();
    transform_header(app, ui);
    app.navi.start = Some(app.navi.scene.clone());
    app.navi.start_fixed = ui.checked(F, "CheckBox1");
    reset_julia_vals(app, ui);
    update_adjust(app, ui, "");
    adjust_panel3_positions(ui);
    app.navi.dynfog_changed = false;
    app.navi.destop_changed = false;
    let df = app.navi.scene.lighting.dyn_fog as i64;
    ui.set_position(F, "UpDown1", df);
    ui.set_caption(F, "Label38", &(df - 53).to_string());
    let cap = if app.title.is_empty() { "main paras".to_string() } else { app.title.clone() };
    ui.fm(F).set_caption(&cap);
    set_zoom(app, ui);
    set_window_size(app, ui, ui.visible(F, "Panel2"));
    new_calc(app, ui);
}

/// `ModRotPoint`: the middle in front of the camera (for the main window's
/// rotations).
fn mod_rot_point(sc: &mut Scene) {
    let sw = sc.step_width();
    let ds = sc.z_start - sc.mid[2];
    let de = sc.z_end - sc.mid[2];
    let l = ((sc.z_end - sc.mid[2]) / sw).min(length_of_size(sc) * 1.5);
    let vz = normalize(sc.vgrads[2]);
    for k in 0..3 {
        sc.mid[k] += vz[k] * sw * l;
    }
    sc.z_start = sc.mid[2] + ds - sw * l;
    sc.z_end = sc.mid[2] + de - sw * l;
}

/// `SpeedButton14Click`: the view to the main window.
fn view_to_main(app: &mut Mb3d, ui: &mut Ui) {
    app.make_scene(ui);
    let n = app.navi.scene.clone();
    let m = &mut app.scene;
    let (mw, mh) = (m.width, m.height);
    m.z_start = n.z_start;
    m.z_end = n.z_end;
    m.mid = n.mid;
    m.zoom = n.zoom;
    m.vgrads = n.vgrads;
    m.fov_y = n.fov_y;
    m.optic = n.optic;
    m.stereo_mode = 0;
    m.iterations = n.iterations;
    m.julia = n.julia;
    if n.julia {
        m.julia_c = n.julia_c;
    }
    m.dfog_on_it = n.dfog_on_it;
    m.rstop = n.rstop;
    if let (Some(a), Some(b)) = (m.decomb.as_mut(), n.decomb.as_ref()) {
        a.smooth = b.smooth;
    }
    if app.navi.destop_changed {
        m.de_stop = n.de_stop;
    }
    m.color_on_it = n.color_on_it;
    if app.navi.dynfog_changed {
        m.lighting.dyn_fog = n.lighting.dyn_fog;
    }
    m.formulas = round_fvals(&n.formulas);
    m.rot_4d = n.rot_4d;
    let _ = (mw, mh);
    let mut mm = m.clone();
    mod_rot_point(&mut mm);
    *m = mm;
    app.scene_to_forms(ui);
    let t = ui.f(F).caption().to_string();
    app.title = if t.is_empty() || t == "main paras" { app.title.clone() } else if t.ends_with('~') { t } else { format!("{t}~") };
    super::main_form::set_caption(app, ui);
    app.message(ui, "Navigator view sent to the main window, press \"Calculate 3D\".");
}

fn round_fvals(f: &[crate::scene::FormulaEntry]) -> Vec<crate::scene::FormulaEntry> {
    f.iter()
        .map(|e| {
            let mut e = e.clone();
            let types = e.formula.option_types();
            for (i, (_, v)) in e.formula.options().into_iter().enumerate() {
                if types.get(i).is_some_and(|&t| is_int_type(t as i32)) {
                    super::formula_form::set_option_index(&mut e.formula, i, v.round());
                }
            }
            e
        })
        .collect()
}

// ---------------------------------------------------------------------------
// moving
// ---------------------------------------------------------------------------

/// `SpeedButton1Click`: tag 1/2 slide up/down, 3/4 left/right, 5/6 walk
/// forward/back, 7..10 look, 11/12 roll.
fn navigate(app: &mut Mb3d, ui: &mut Ui, t: i64, fine: bool) {
    let dm = if fine { 0.125 } else { 1.0 };
    let fixed = ui.checked(F, "CheckBox1");
    let min_d = ptf(&ui.text(F, "Edit6"));
    let st = &mut app.navi;
    let sc = &mut st.scene;
    let u = unit_rows(sc);
    let backstep = matches!(t, 1 | 3 | 6);
    st.moving = t < 7;
    if st.moving {
        let i = pti(&ui.text(F, "Edit1")).clamp(1, 90) as f64;
        let mut valid = !fixed;
        let mut de = 0.0;
        if valid {
            match local_de(sc) {
                Some(d) => de = d * st.de_mul,
                None => valid = false,
            }
        }
        if !valid {
            de = length_of_size(sc) * sc.step_width();
        }
        let v0 = sc.mid;
        let (zs0, zm0) = (sc.z_start, sc.mid[2]);
        let n = if t < 3 { 1 } else if t < 5 { 0 } else { 2 };
        let step = |sc: &mut Scene, d: f64| {
            for k in 0..3 {
                sc.mid[k] = v0[k] + u[n][k] * d * dm;
            }
            sc.z_start = zs0 + sc.mid[2] - zm0;
        };
        let mut d = 0.01 * i * de * if backstep { -1.0 } else { 1.0 };
        step(sc, d);
        if valid {
            let nd = local_de(sc).map(|x| x * st.de_mul);
            match nd {
                Some(nd) if nd < min_d => {
                    // keep the minimum distance
                    if de > min_d {
                        let de2 = de * (de - min_d) / (de - nd).max(1e-300);
                        d = 0.01 * i * de2 * if backstep { -1.0 } else { 1.0 };
                        step(sc, d);
                    } else {
                        step(sc, 0.0);
                    }
                }
                Some(mut nd) => {
                    let min_de = (de * (100.0 - i) * 0.008).max(1e-13);
                    let max_de = (de * (100.0 + i) * 0.0125).max(min_de * 1.1);
                    let mut ds = 1.25;
                    for _ in 0..100 {
                        if nd > min_de && nd < max_de {
                            break;
                        }
                        ds *= 0.8;
                        let de2 = 0.5 * (de + nd);
                        d = 0.01 * i * de2 * ds * if backstep { -1.0 } else { 1.0 };
                        step(sc, d);
                        match local_de(sc) {
                            Some(x) => nd = x * st.de_mul,
                            None => break,
                        }
                    }
                }
                None => step(sc, 0.0),
            }
        }
        let dz = sc.mid[2] - zm0;
        sc.z_start = zs0 + dz;
        sc.z_end += dz;
        set_zoom(app, ui);
    } else {
        let d = ptf(&ui.text(F, "Edit2")) * -PID180 * dm;
        let d = if matches!(t, 8 | 9 | 11) { -d } else { d };
        let m = match t {
            7 | 8 => build_rot_matrix(d, 0.0, 0.0),
            9 | 10 => build_rot_matrix(0.0, d, 0.0),
            _ => build_rot_matrix(0.0, 0.0, d),
        };
        sc.vgrads = mat_mul(&m, &sc.vgrads);
    }
    new_calc(app, ui);
}

fn rotate_view(app: &mut Mb3d, v: f64, h: f64, roll: f64) {
    let m = if roll != 0.0 { build_rot_matrix(0.0, 0.0, roll) } else { build_rot_matrix(v, h, 0.0) };
    let sc = &mut app.navi.scene;
    sc.vgrads = mat_mul(&m, &sc.vgrads);
    app.navi.moving = false;
}

fn set_look(app: &mut Mb3d, ui: &mut Ui, on: bool) {
    app.navi.look = on;
    let fi = ui.form_index(F).unwrap();
    ui.request(crate::vcl::ui::Request::MouseLook(fi, on));
}

// ---------------------------------------------------------------------------
// the adjustment panel
// ---------------------------------------------------------------------------

fn reset_julia_vals(app: &mut Mb3d, ui: &mut Ui) {
    let st = &mut app.navi;
    for k in 0..3 {
        st.pos0[k] = st.scene.julia_c[k];
        st.vals[k] = st.pos0[k];
    }
    update_julia_labels(app, ui);
    ui.set_checked(F, "CheckBox7", app.navi.scene.julia);
}

fn update_julia_labels(app: &Mb3d, ui: &mut Ui) {
    for (k, l) in ["Label39", "Label40", "Label41"].iter().enumerate() {
        ui.set_caption(F, l, &fts_single(app.navi.vals[k]));
    }
}

fn reset_4d(app: &mut Mb3d, ui: &mut Ui) {
    let st = &mut app.navi;
    for k in 0..3 {
        st.pos0[6 + k] = st.scene.rot_4d[k];
        st.vals[6 + k] = st.pos0[6 + k];
    }
    update_4d_labels(app, ui);
}

fn update_4d_labels(app: &Mb3d, ui: &mut Ui) {
    for (k, l) in ["Label50", "Label51", "Label52"].iter().enumerate() {
        ui.set_caption(F, l, &fts_single(app.navi.vals[6 + k] / PID180));
    }
}

fn formula_nr(ui: &Ui) -> usize {
    (pti(&ui.caption(F, "Label49")) - 1).clamp(0, 5) as usize
}

fn formula_index(app: &Mb3d, ui: &Ui, slider: usize) -> (usize, usize) {
    (formula_nr(ui), (app.navi.sub_top + slider).min(15))
}

fn formula_value(app: &Mb3d, fi: (usize, usize)) -> (f64, i32, String) {
    match app.navi.scene.formulas.get(fi.0) {
        Some(e) => {
            let o = e.formula.options();
            let t = e.formula.option_types();
            match o.get(fi.1) {
                Some((k, v)) => (*v, t.get(fi.1).copied().unwrap_or(0) as i32, k.clone()),
                None => (0.0, 0, String::new()),
            }
        }
        None => (0.0, 0, String::new()),
    }
}

fn option_count(app: &Mb3d, f: usize) -> usize {
    app.navi.scene.formulas.get(f).map(|e| e.formula.options().len()).unwrap_or(0)
}

/// `UpdateFormulaLabels`.
fn update_formula_labels(app: &Mb3d, ui: &mut Ui) {
    let f = formula_nr(ui);
    let n = option_count(app, f);
    let used = app.navi.scene.interpolation.is_some() || app.navi.scene.formulas.get(f).map(|e| e.iterations > 0).unwrap_or(false);
    for i in 0..3 {
        let i2 = app.navi.sub_top + i;
        let b = i2 < n && used;
        for c in [format!("LabelF{i}"), format!("LabelV{i}"), format!("TrackBarEx{}", i + 4)] {
            ui.set_visible(F, &c, b);
        }
        if b {
            let (_, t, name) = formula_value(app, (f, i2));
            ui.set_caption(F, &format!("LabelF{i}"), &name);
            let v = app.navi.vals[3 + i];
            ui.set_caption(F, &format!("LabelV{i}"), &if is_int_type(t) { (v.round() as i64).to_string() } else { fts_single(v) });
        }
    }
    ui.set_visible(F, "Image4", app.navi.sub_top > 0);
    ui.set_visible(F, "Image5", n > app.navi.sub_top + 3);
}

fn update_divers_labels(app: &Mb3d, ui: &mut Ui) {
    let sc = &app.navi.scene;
    let fog = ui.caption(F, "SpeedButton33") == "Dyn Fog on its:";
    ui.set_caption(F, "LabelV3", &if fog { sc.dfog_on_it.to_string() } else { (sc.color_on_it as i32 - 1).to_string() });
    ui.set_caption(F, "LabelV4", &fts_single(sc.effective_rstop()));
    ui.set_caption(F, "LabelV5", &fts_single(sc.decomb.map(|d| d.smooth as f64).unwrap_or(0.5)));
    ui.set_caption(F, "LabelV6", &fts_single(sc.de_stop));
    ui.set_caption(F, "LabelV7", &sc.iterations.to_string());
}

/// `SpeedButton26Click` as the general update of the adjustment panel.
fn update_adjust(app: &mut Mb3d, ui: &mut Ui, sender: &str) {
    if app.navi.focused > 2 {
        app.navi.focused = 0;
    }
    if sender == "SpeedButton26" {
        // the focused value from the main window
        let fi = formula_index(app, ui, app.navi.focused);
        if let (Some(src), Some(dst)) = (app.scene.formulas.get(fi.0).cloned(), app.navi.scene.formulas.get_mut(fi.0)) {
            if let Some((_, v)) = src.formula.options().get(fi.1) {
                super::formula_form::set_option_index(&mut dst.formula, fi.1, *v);
            }
        }
    } else if sender == "SpinEdit2" {
        app.navi.focused = 0;
        app.navi.sub_top = 0;
    }
    let fi = formula_index(app, ui, app.navi.focused);
    app.navi.findex = fi;
    let name = app.navi.scene.formulas.get(fi.0).map(|e| if e.iterations > 0 || fi.0 == 0 { e.formula.name() } else { String::new() }).unwrap_or_default();
    ui.set_caption(F, "Label44", &name);
    match app.navi.scene.interpolation {
        Some(w) => {
            ui.set_caption(F, "Label47", &fts_single(w.get(fi.0).copied().unwrap_or(0.0) as f64));
            ui.set_caption(F, "Label42", "Weight:");
        }
        None => {
            let its = app.navi.scene.formulas.get(fi.0).map(|e| e.iterations).unwrap_or(0);
            ui.set_caption(F, "Label47", &its.to_string());
            ui.set_caption(F, "Label42", "Iterations:");
        }
    }
    for i in 3..6 {
        let (v, t, _) = formula_value(app, formula_index(app, ui, i - 3));
        app.navi.vals[i] = v;
        app.navi.pos0[i] = v;
        app.navi.vtype[i] = t;
        app.navi.range[i] = if is_angle_type(t) { 15.0 } else { 1.0 };
    }
    update_formula_labels(app, ui);
    let sc = &app.navi.scene;
    let fog = ui.caption(F, "SpeedButton33") == "Dyn Fog on its:";
    let p = [
        if fog { sc.dfog_on_it as f64 } else { sc.color_on_it as f64 - 1.0 },
        sc.effective_rstop(),
        sc.decomb.map(|d| d.smooth as f64).unwrap_or(0.5),
        sc.de_stop,
        sc.iterations as f64,
    ];
    for (k, v) in p.into_iter().enumerate() {
        app.navi.pos0[9 + k] = v;
        app.navi.vals[9 + k] = v;
    }
    update_divers_labels(app, ui);
    if sender == "SpeedButton26" || sender == "UpDown2" {
        new_calc(app, ui);
    }
}

fn adjust_panel3_positions(ui: &mut Ui) {
    let mut i = ui.c(F, "RadioGroup1").height + 1 + ui.c(F, "SizeGroup").height;
    let place = |ui: &mut Ui, c: &str, i: &mut i32, panel: bool| {
        ui.cm(F, c).top = *i;
        if !panel || ui.visible(F, c) {
            *i += ui.c(F, c).height;
        }
    };
    place(ui, "Button2", &mut i, false);
    place(ui, "Panel4", &mut i, true);
    i += 1;
    place(ui, "Button3", &mut i, false);
    place(ui, "Panel5", &mut i, true);
    i += 1;
    let fourd = ui.visible("FormulaGUIForm", "Panel2");
    ui.set_enabled(F, "Button4", fourd);
    if !fourd {
        ui.set_visible(F, "Panel6", false);
    }
    place(ui, "Button4", &mut i, false);
    place(ui, "Panel6", &mut i, true);
    i += 1;
    place(ui, "Button5", &mut i, false);
    ui.cm(F, "Panel7").top = i;
}

/// `RxSlider1Change`: a value slider moved (-60..60 around the start value).
fn slider_change(app: &mut Mb3d, ui: &mut Ui, sender: &str) {
    let t = ui.tag(F, sender) as usize;
    let pos = ui.position(F, sender) as f64;
    let mut d = ((640 * 16) >> (ui.item_index(F, "RadioGroup1").clamp(0, 3) * 4)) as f64;
    if is_int_type(app.navi.vtype[t]) {
        d = (d * 0.5).clamp(3.0, 50.0);
    }
    let st = &mut app.navi;
    if t < 3 && ui.checked(F, "CheckBox8") {
        // julia values in spherical coordinates
        let p0 = [st.pos0[0], st.pos0[1], st.pos0[2]];
        let r = len(&p0).max(1e-10);
        let ph = (p0[2] / r).clamp(-1.0, 1.0).asin();
        let th = p0[1].atan2(if p0[0] == 0.0 { 1e-300 } else { p0[0] });
        match t {
            0 => {
                let f = (r + pos / d).max(1e-10) / r;
                for k in 0..3 {
                    st.vals[k] = p0[k] * f;
                }
            }
            1 => {
                let r2 = (p0[0] * p0[0] + p0[1] * p0[1]).sqrt();
                let a = th + pos * 10.0 / d;
                st.vals[0] = a.cos() * r2;
                st.vals[1] = a.sin() * r2;
            }
            _ => {
                let a = ph + pos * 4.0 / d;
                st.vals[0] = a.cos() * th.cos() * r;
                st.vals[1] = a.cos() * th.sin() * r;
                st.vals[2] = a.sin() * r;
            }
        }
        for k in 0..3 {
            st.scene.julia_c[k] = st.vals[k];
        }
    } else if st.vtype[t] == -5 {
        st.vals[t] = st.pos0[t] * 3f64.powf(pos * st.range[t] / d);
    } else {
        st.vals[t] = st.pos0[t] + pos * st.range[t] / d;
    }
    let formula_shown = ui.visible(F, sender);
    apply_value(app, ui, t, formula_shown);
}

/// The value `app.navi.vals[t]` of a slider into the navigator's scene,
/// its label and the preview (from the slider or typed into the label).
fn apply_value(app: &mut Mb3d, ui: &mut Ui, t: usize, formula_shown: bool) {
    let st = &mut app.navi;
    let mut update = false;
    if t < 3 {
        let v = st.vals[t];
        st.scene.julia_c[t] = v;
        update_julia_labels(app, ui);
        update = app.navi.scene.julia;
    } else if (6..9).contains(&t) {
        st.scene.rot_4d[t - 6] = st.vals[t];
        update_4d_labels(app, ui);
        update = true;
    } else if (9..14).contains(&t) {
        st.focused = t - 6;
        let lv = format!("LabelV{}", t - 6);
        let old = ui.caption(F, &lv);
        let fog = ui.caption(F, "SpeedButton33") == "Dyn Fog on its:";
        let sc = &mut st.scene;
        let cap = match t {
            9 => {
                let i = st.vals[9].clamp(0.0, 255.0).round() as i32;
                st.vals[9] = i as f64;
                if fog {
                    sc.dfog_on_it = i as u16;
                } else {
                    sc.color_on_it = (i + 1).clamp(0, 255) as u8;
                }
                i.to_string()
            }
            10 => {
                st.vals[10] = st.vals[10].max(0.0);
                sc.rstop = Some(st.vals[10]);
                fts_single(st.vals[10])
            }
            11 => {
                st.vals[11] = st.vals[11].clamp(0.0, 100.0);
                if let Some(dc) = sc.decomb.as_mut() {
                    dc.smooth = st.vals[11] as f32;
                }
                fts_single(st.vals[11])
            }
            12 => {
                st.vals[12] = st.vals[12].clamp(0.1, 300.0);
                sc.de_stop = st.vals[12];
                st.destop_changed = true;
                fts_single(st.vals[12])
            }
            _ => {
                st.vals[13] = st.vals[13].clamp(1.0, 2000.0);
                sc.iterations = st.vals[13].round() as i32;
                sc.iterations.to_string()
            }
        };
        ui.set_caption(F, &lv, &cap);
        update = old != cap;
    } else if formula_shown {
        st.focused = t - 3;
        let fi = (formula_nr(ui), (st.sub_top + t - 3).min(15));
        let lv = format!("LabelV{}", t - 3);
        let old = ui.caption(F, &lv);
        let v = st.vals[t];
        if let Some(e) = st.scene.formulas.get_mut(fi.0) {
            super::formula_form::set_option_index(&mut e.formula, fi.1, v);
        }
        update_formula_labels(app, ui);
        update = old != ui.caption(F, &lv);
    }
    let foc = app.navi.focused;
    ui.set_enabled(F, "SpeedButton26", foc <= 2);
    ui.set_enabled(F, "SpeedButton27", foc <= 2);
    ui.set_enabled(F, "SpeedButton31", (3..=7).contains(&foc));
    ui.set_enabled(F, "SpeedButton32", (3..=7).contains(&foc));
    if update {
        new_calc(app, ui);
    }
}

// ---------------------------------------------------------------------------
// events
// ---------------------------------------------------------------------------

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    match h {
        "FormCreate" => {
            let q = app.ini.get("NaviSize").to_string();
            let i = ui.c(F, "NaviSizeCmb").items.iter().position(|s| *s == q).map(|i| i as i32).unwrap_or(6);
            ui.set_item_index(F, "NaviSizeCmb", i);
            ui.set_caption(F, "CheckBox4", "HiQual 1");
        }
        "FormShow" => {
            if app.navi.first {
                app.navi.first = false;
                ui.set_text(F, "Edit1", &app.ini.get("NavSlideStep").to_string());
                ui.set_text(F, "Edit2", &app.ini.get("NavLookAngle").to_string());
                ui.set_text(F, "Edit3", &app.ini.get("NavFOVy").to_string());
                ui.set_checked(F, "CheckBox2", app.ini.get("NaviAZERTY") == "1");
                ui.set_checked(F, "CheckBox5", app.ini.get("NavFkey") == "0");
                ui.set_checked(F, "CheckBox4", app.ini.get("NavHiQ") != "No");
                app.navi.double_click = app.ini.get("NavDoubleClickMode") != "No";
                for k in 0..3 {
                    let p = crate::appdirs::app_folder().join(format!("NaviLightPreset{}.m3l", k + 1));
                    app.navi.presets[k] = std::fs::read(p).ok().and_then(|d| crate::m3p::light_from_record(&d));
                    ui.set_enabled(F, &format!("SpeedButton{}", 19 + k), app.navi.presets[k].is_some());
                }
                insert_main(app, ui);
                if app.ini.get("NaviPanelShow") == "1" {
                    toggle_adjust_panel(app, ui);
                }
            }
            enable_size_buttons(ui);
        }
        "FormHide" => {
            app.navi.eng.stop();
            app.ini.set("NavSlideStep", &ui.text(F, "Edit1"));
            app.ini.set("NavLookAngle", &ui.text(F, "Edit2"));
            app.ini.set("NavFOVy", &ui.text(F, "Edit3"));
            app.ini.set("NaviAZERTY", if ui.checked(F, "CheckBox2") { "1" } else { "0" });
            app.ini.set("NavFkey", if ui.checked(F, "CheckBox5") { "0" } else { "1" });
            app.ini.set("NavHiQ", if ui.checked(F, "CheckBox4") { "Yes" } else { "No" });
            if app.navi.look {
                set_look(app, ui, false);
            }
        }
        "FormCloseQuery" => {
            app.navi.eng.stop();
            ui.close_ok(F);
        }
        "SpeedButton1Click" => {
            let t = ui.tag(F, &e.sender);
            navigate(app, ui, t, false);
        }
        "SpeedButton11Click" => insert_main(app, ui),
        "SpeedButton14Click" => view_to_main(app, ui),
        "SpeedButton15Click" => {
            // the lighting of the main window
            super::light_form::put_light_in_header(app, ui);
            app.navi.scene.lighting = app.scene.lighting.clone();
            let df = app.navi.scene.lighting.dyn_fog as i64;
            ui.set_position(F, "UpDown1", df);
            ui.set_caption(F, "Label38", &(df - 53).to_string());
            app.navi.dynfog_changed = false;
            new_calc(app, ui);
        }
        "SpeedButton16Click" => {
            // the formulas of the main window
            app.make_scene(ui);
            let m = app.scene.clone();
            let n = &mut app.navi.scene;
            n.formulas = m.formulas;
            n.interpolation = m.interpolation;
            n.decomb = m.decomb;
            n.repeat_from = m.repeat_from;
            n.iterations = m.iterations;
            n.min_iterations = m.min_iterations;
            n.rstop = m.rstop;
            n.de_stop = m.de_stop;
            app.navi.destop_changed = false;
            update_adjust(app, ui, "");
            adjust_panel3_positions(ui);
            new_calc(app, ui);
        }
        "SpeedButton17Click" => {
            let h0 = app.navi.scene.height;
            let v = !ui.visible(F, "Panel2");
            set_window_size(app, ui, v);
            if h0 != app.navi.scene.height {
                new_calc(app, ui);
            }
        }
        "SpeedButton18Click" => {
            // a keyframe for the animation
            let mut s = app.navi.scene.clone();
            mod_rot_point(&mut s);
            s.width = app.scene.width;
            s.height = app.scene.height;
            super::anim_form::insert_keyframe(app, ui, s);
        }
        "SpeedButton23Click" => toggle_adjust_panel(app, ui),
        "FormKeyDown" => {
            if let Ev::KeyDown { key, shift } = e.ev {
                form_key_down(app, ui, key, shift & crate::vcl::form::SS_SHIFT != 0);
            }
        }
        "FormMouseWheel" => {
            if let Ev::Wheel { delta, x, y, .. } = e.ev {
                if app.navi.look && ui.visible(F, "UpDown3") {
                    zoom_step(app, ui, delta > 0);
                    return;
                }
                let r = ui.ctl_rect(F, "Image1");
                if r.contains(x, y) {
                    if delta < 0 {
                        navigate(app, ui, 6, false);
                    } else {
                        if !app.navi.look {
                            // steer towards the mouse
                            let fs = (app.navi.scene.fov_y * PID180).sin();
                            let v = fs * ((y - r.y) - r.h / 2) as f64 / r.h as f64;
                            let hh = -fs * ((x - r.x) - r.w / 2) as f64 / r.h as f64;
                            rotate_view(app, v, hh, 0.0);
                        }
                        navigate(app, ui, 5, false);
                    }
                }
            }
        }
        "Image1Click" => {
            if !app.navi.double_click {
                let on = !app.navi.look;
                set_look(app, ui, on);
            }
        }
        "Image1DblClick" => {
            if app.navi.double_click {
                let on = !app.navi.look;
                set_look(app, ui, on);
            }
        }
        "Image1MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Right, x, y, .. } = e.ev {
                if !app.navi.look {
                    let dc = app.navi.double_click;
                    ui.set_checked(F, "Singleclicktochangethenavimode1", !dc);
                    ui.set_checked(F, "Doubleclicktochangethenavimode1", dc);
                    ui.popup_menu(F, "PopupMenu1", "Image1", x, y);
                }
            }
        }
        "Doubleclicktochangethenavimode1Click" => {
            app.navi.double_click = true;
            app.ini.set("NavDoubleClickMode", "Yes");
        }
        "Singleclicktochangethenavimode1Click" => {
            app.navi.double_click = false;
            app.ini.set("NavDoubleClickMode", "No");
        }
        "@MouseDelta" => {
            if let Ev::MouseDelta { dx, dy, shift } = e.ev {
                let r = ui.c(F, "Image1").height.max(1) as f64;
                let mut hh = app.navi.scene.fov_y;
                if hh.abs() < 15.0 {
                    hh = 15.0f64.copysign(hh);
                }
                let k = hh.clamp(-180.0, 180.0) * PID180 * 2.0 / r;
                if shift & crate::vcl::form::SS_RIGHT != 0 {
                    rotate_view(app, 0.0, 0.0, k * (dx - dy));
                } else {
                    rotate_view(app, k * dy, -k * dx, 0.0);
                }
                new_calc(app, ui);
            }
        }
        "Edit1Change" => {
            let tg = ui.tag(F, &e.sender);
            let s = ui.text(F, &e.sender);
            if parse_float(&s).is_none() {
                let only: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
                ui.set_text(F, &e.sender, &only);
            } else if e.sender == "Edit4" {
                set_zoom(app, ui);
            }
            if tg == 1 {
                new_calc(app, ui);
            }
        }
        "CheckBox2Click" => {
            let az = ui.checked(F, "CheckBox2");
            ui.set_caption(F, "Label6", if az { "(q)" } else { "(a)" });
            ui.set_caption(F, "Label5", if az { "(c,a)" } else { "(c,q)" });
            ui.set_caption(F, "Label12", if az { "(z)" } else { "(w)" });
        }
        "CheckBox5Click" => {
            let c = if ui.checked(F, "CheckBox5") { "Ani keyframe" } else { "Ani keyfr. (f)" };
            ui.set_caption(F, "SpeedButton18", c);
        }
        "CheckBox1Click" => {
            let v = ui.checked(F, "CheckBox1");
            ui.set_visible(F, "UpDown3", v);
        }
        "CheckBox3Click" | "RadioGroup2Click" => new_calc(app, ui),
        "ShowCoordsCBxClick" | "ShowGuidesCBxClick" => {
            app.navi.ver = 0;
            idle(app, ui);
        }
        "TrackBar1Change" => {
            let p = ui.position(F, "TrackBar1") as f64;
            app.navi.lightness = (p * -0.05 + 0.85).powi(2) + 0.2775;
            app.navi.ver = 0;
            idle(app, ui);
        }
        "Button1Click" => {
            let d = local_de(&app.navi.scene).map(|d| d * app.navi.de_mul).unwrap_or(0.0);
            ui.set_text(F, "Edit6", &fts_single(d));
        }
        "SpeedButton22Click" => {
            app.navi.storing = !app.navi.storing;
            for k in 0..3 {
                let b = format!("SpeedButton{}", 19 + k);
                ui.cm(F, &b).cursor = if app.navi.storing { "crUpArrow".into() } else { "crDefault".into() };
                let en = app.navi.storing || app.navi.presets[k].is_some();
                ui.set_enabled(F, &b, en);
            }
        }
        "SpeedButton19Click" => {
            let t = (ui.tag(F, &e.sender) - 1).clamp(0, 2) as usize;
            if ui.c(F, &e.sender).cursor == "crUpArrow" {
                let rec = crate::m3p::write(&app.navi.scene);
                let p = crate::appdirs::app_folder().join(format!("NaviLightPreset{}.m3l", t + 1));
                let _ = std::fs::write(p, &rec[432..840]);
                app.navi.presets[t] = Some(app.navi.scene.lighting.clone());
                app.navi.storing = false;
                for k in 0..3 {
                    let b = format!("SpeedButton{}", 19 + k);
                    ui.cm(F, &b).cursor = "crDefault".into();
                    let en = app.navi.presets[k].is_some();
                    ui.set_enabled(F, &b, en);
                }
            } else if let Some(l) = app.navi.presets[t].clone() {
                app.navi.scene.lighting = l;
                let df = app.navi.scene.lighting.dyn_fog as i64;
                ui.set_position(F, "UpDown1", df);
                ui.set_caption(F, "Label38", &(df - 53).to_string());
                ui.set_position(F, "TrackBar1", 0);
                app.navi.dynfog_changed = false;
                new_calc(app, ui);
            }
        }
        "UpDown3Click" => {
            if let Ev::UpDown { up } = e.ev {
                zoom_step(app, ui, up);
            }
        }
        "UpDown4Click" => {
            if let Ev::UpDown { up } = e.ev {
                let q = (app.navi.quality + if up { 1 } else { -1 }).clamp(1, 7);
                if q != app.navi.quality {
                    app.navi.quality = q;
                    ui.set_caption(F, "CheckBox4", &format!("HiQual {q}"));
                    new_calc(app, ui);
                }
            }
        }
        "UpDown1Click" => {
            if let Ev::UpDown { up } = e.ev {
                let l = &mut app.navi.scene.lighting;
                l.dyn_fog = if up { (l.dyn_fog + 1.0).min(153.0) } else { (l.dyn_fog - 1.0).max(0.0) };
                let df = l.dyn_fog as i64;
                ui.set_position(F, "UpDown1", df);
                ui.set_caption(F, "Label38", &(df - 53).to_string());
                app.navi.dynfog_changed = true;
                new_calc(app, ui);
            }
        }
        "NaviSizeCmbChange" | "DecreaseNaviSizeBtnClick" | "IncreaseNaviSizeBtnClick" => {
            let i = ui.item_index(F, "NaviSizeCmb");
            let n = ui.c(F, "NaviSizeCmb").items.len() as i32;
            match h {
                "DecreaseNaviSizeBtnClick" if i > 0 => ui.set_item_index(F, "NaviSizeCmb", i - 1),
                "IncreaseNaviSizeBtnClick" if i < n - 1 => ui.set_item_index(F, "NaviSizeCmb", i + 1),
                _ => {}
            }
            let s = ui.c(F, "NaviSizeCmb").item_text();
            app.ini.set("NaviSize", &s);
            enable_size_buttons(ui);
            let h0 = app.navi.scene.height;
            let p2 = ui.visible(F, "Panel2");
            set_window_size(app, ui, p2);
            if h0 != app.navi.scene.height {
                new_calc(app, ui);
            }
        }
        // ---- the adjustment panel
        "Button2Click" | "Button3Click" | "Button4Click" | "Button5Click" => {
            let p = match h {
                "Button2Click" => "Panel4",
                "Button3Click" => "Panel5",
                "Button4Click" => "Panel6",
                _ => "Panel7",
            };
            let v = !ui.visible(F, p);
            ui.set_visible(F, p, v);
            if h == "Button5Click" && ui.visible(F, "Panel4") && ui.visible(F, "Panel5") && ui.visible(F, "Panel7") {
                ui.set_visible(F, "Panel4", false);
            }
            adjust_panel3_positions(ui);
        }
        "RxSlider1Change" => slider_change(app, ui, &e.sender),
        "RxSlider1MouseUp" => {
            let t = ui.tag(F, &e.sender) as usize;
            if t < 3 && ui.checked(F, "CheckBox8") {
                for k in 0..3 {
                    app.navi.pos0[k] = app.navi.vals[k];
                }
            } else {
                app.navi.pos0[t] = app.navi.vals[t];
            }
            ui.set_position(F, &e.sender, 0);
        }
        "CheckBox7Click" => {
            app.navi.scene.julia = ui.checked(F, "CheckBox7");
            new_calc(app, ui);
        }
        "SpeedButton24Click" => {
            app.navi.scene.julia = ui.checked(MAIN, "CheckBox7");
            app.make_scene(ui);
            app.navi.scene.julia_c = app.scene.julia_c;
            reset_julia_vals(app, ui);
            if app.navi.scene.julia {
                new_calc(app, ui);
            }
        }
        "SpeedButton25Click" => {
            let j = app.navi.scene.julia_c;
            ui.set_text(MAIN, "Edit28", &fts(j[0]));
            ui.set_text(MAIN, "Edit29", &fts(j[1]));
            ui.set_text(MAIN, "Edit30", &fts(j[2]));
            ui.set_checked(MAIN, "CheckBox7", app.navi.scene.julia);
        }
        "SpeedButton26Click" => update_adjust(app, ui, "SpeedButton26"),
        "SpeedButton27Click" => {
            let fi = app.navi.findex;
            let v = app.navi.vals[3 + app.navi.focused.min(2)];
            if let Some(slot) = app.formula.slots.get_mut(fi.0).and_then(|s| s.as_mut()) {
                super::formula_form::set_option_index(slot, fi.1, v);
            }
            super::formula_form::tab_control1_change(app, ui);
        }
        "SpeedButton28Click" => {
            let f = round_fvals(&app.navi.scene.formulas);
            for (i, e) in f.into_iter().enumerate().take(6) {
                if app.formula.slots.get(i).is_some_and(|s| s.is_some()) {
                    app.formula.slots[i] = Some(e.formula);
                }
            }
            super::formula_form::tab_control1_change(app, ui);
        }
        "SpinEdit2Click" => {
            if let Ev::UpDown { up } = e.ev {
                let n = (pti(&ui.caption(F, "Label49")) + if up { 1 } else { -1 }).clamp(1, 6);
                ui.set_caption(F, "Label49", &n.to_string());
                update_adjust(app, ui, "SpinEdit2");
            }
        }
        "UpDown2Click" => {
            if let Ev::UpDown { up } = e.ev {
                let fx = app.navi.findex.0;
                let sc = &mut app.navi.scene;
                match sc.interpolation.as_mut() {
                    Some(w) if fx < 2 => w[fx] = (w[fx] + if up { 0.1 } else { -0.1 }).max(0.0),
                    _ => {
                        if let Some(e) = sc.formulas.get_mut(fx) {
                            e.iterations = (e.iterations + if up { 1 } else { -1 }).max(0);
                        }
                    }
                }
                update_adjust(app, ui, "UpDown2");
            }
        }
        "ScrollBar1Change" => {
            let n = option_count(app, app.navi.findex.0);
            let i = (ui.position(F, "ScrollBar1") as usize).min(n.saturating_sub(2));
            if i != app.navi.sub_top {
                app.navi.sub_top = i;
                update_adjust(app, ui, "");
            }
        }
        "FormMouseMove" => {
            // the option scroll bar appears over the arrows
            if ui.visible(F, "Panel5") {
                let r = ui.ctl_rect(F, "ScrollBar1");
                let (mx, my) = ui.f(F).mouse;
                let over = mx >= r.x - 4 && my >= r.y && my <= r.bottom();
                let arrows = ui.visible(F, "Image4") || ui.visible(F, "Image5");
                if over && arrows && !ui.visible(F, "ScrollBar1") {
                    let n = option_count(app, app.navi.findex.0).min(15) as i64;
                    let c = ui.cm(F, "ScrollBar1");
                    c.min = 0;
                    c.max = (n - 2).max(app.navi.sub_top as i64);
                    c.position = app.navi.sub_top as i64;
                    ui.set_visible(F, "Image4", false);
                    ui.set_visible(F, "Image5", false);
                    ui.set_visible(F, "ScrollBar1", true);
                } else if !over && ui.visible(F, "ScrollBar1") {
                    ui.set_visible(F, "ScrollBar1", false);
                    update_formula_labels(app, ui);
                }
            }
        }
        "SpeedButton29Click" => {
            app.make_scene(ui);
            app.navi.scene.rot_4d = app.scene.rot_4d;
            reset_4d(app, ui);
            new_calc(app, ui);
        }
        "SpeedButton30Click" => {
            let r = app.navi.scene.rot_4d;
            ui.set_text("FormulaGUIForm", "XWEdit", &fts(r[0] / PID180));
            ui.set_text("FormulaGUIForm", "YWEdit", &fts(r[1] / PID180));
            ui.set_text("FormulaGUIForm", "ZWEdit", &fts(r[2] / PID180));
        }
        "SpeedButton31Click" => {
            // a value back from the main window
            app.make_scene(ui);
            let m = app.scene.clone();
            let foc = app.navi.focused;
            let fog = ui.caption(F, "SpeedButton33") == "Dyn Fog on its:";
            let n = &mut app.navi.scene;
            match foc {
                3 => {
                    if fog {
                        n.dfog_on_it = m.dfog_on_it;
                    } else {
                        n.color_on_it = m.color_on_it;
                    }
                }
                4 => n.rstop = m.rstop,
                5 | 6 => {
                    if let (Some(a), Some(b)) = (n.decomb.as_mut(), m.decomb) {
                        a.smooth = b.smooth;
                    }
                }
                7 => n.iterations = m.iterations,
                _ => return,
            }
            update_adjust(app, ui, "");
            new_calc(app, ui);
        }
        "SpeedButton32Click" => {
            let foc = app.navi.focused;
            let n = app.navi.scene.clone();
            let fog = ui.caption(F, "SpeedButton33") == "Dyn Fog on its:";
            match foc {
                3 => {
                    if fog {
                        ui.set_text(MAIN, "Edit16", &n.dfog_on_it.to_string())
                    } else {
                        ui.set_text(MAIN, "Edit35", &(n.color_on_it as i32 - 1).to_string())
                    }
                }
                4 => ui.set_text("FormulaGUIForm", "RBailoutEdit", &fts_single(n.effective_rstop())),
                5 => ui.set_text("FormulaGUIForm", "Edit23", &fts_single(n.decomb.map(|d| d.smooth as f64).unwrap_or(0.5))),
                6 => ui.set_text(MAIN, "Edit25", &fts_single(n.de_stop)),
                7 => ui.set_text("FormulaGUIForm", "MaxIterEdit", &n.iterations.to_string()),
                _ => {}
            }
        }
        "SpeedButton33Click" => {
            let fog = ui.caption(F, "SpeedButton33") == "Dyn Fog on its:";
            ui.set_caption(F, "SpeedButton33", if fog { "Color on its:" } else { "Dyn Fog on its:" });
            ui.set_visible(F, "Label38", !fog);
            ui.set_visible(F, "UpDown1", !fog);
            let sc = &app.navi.scene;
            let v = if fog { sc.color_on_it as f64 - 1.0 } else { sc.dfog_on_it as f64 };
            app.navi.vals[9] = v;
            app.navi.pos0[9] = v;
            ui.set_caption(F, "LabelV3", &(v.round() as i64).to_string());
        }
        "Label24Click" => {
            let z = crate::app::util::float_general(app.navi.scene.zoom, 6);
            ui.input_query("navi:zoom", "Zoom", "New zoom:", &z);
        }
        "ResetViewBtnClick" => reset_view(app, ui),
        "Label39MouseDown" => {
            // a value label: focus its slider, and type its value (MB3D's
            // InputQuery; angles in degrees)
            let t = ui.tag(F, &e.sender) as usize;
            if t < 14 {
                let v = app.navi.vals[t];
                let s = if (6..9).contains(&t) { fts_single(v.to_degrees()) } else if is_int_type(app.navi.vtype[t]) { (v.round() as i64).to_string() } else { fts_single(v) };
                ui.input_query(&format!("navi:val{t}"), "Input", "New value:", &s);
            }
            if (3..=5).contains(&t) {
                app.navi.focused = t - 3;
            } else if (9..=13).contains(&t) {
                app.navi.focused = t - 6;
            }
            let foc = app.navi.focused;
            ui.set_enabled(F, "SpeedButton26", foc <= 2);
            ui.set_enabled(F, "SpeedButton27", foc <= 2);
            ui.set_enabled(F, "SpeedButton31", (3..=7).contains(&foc));
            ui.set_enabled(F, "SpeedButton32", (3..=7).contains(&foc));
        }
        _ => {}
    }
}

/// "Reset view": the position, direction and zoom taken from the main
/// window (the adjusted values stay).
fn reset_view(app: &mut Mb3d, ui: &mut Ui) {
    let Some(s0) = app.navi.start.clone() else { return };
    let sc = &mut app.navi.scene;
    sc.mid = s0.mid;
    sc.vgrads = s0.vgrads;
    sc.z_start = s0.z_start;
    sc.z_end = s0.z_end;
    sc.zoom = s0.zoom;
    // the zoom as when the view was taken: fixed, or from the distance
    ui.set_checked(F, "CheckBox1", app.navi.start_fixed);
    app.navi.de_mul = 1.0;
    set_zoom(app, ui);
    new_calc(app, ui);
}

/// Answers of the navigator's input boxes: a typed zoom (it stays: fixed
/// zoom) or a typed slider value.
pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &crate::vcl::DialogResult) {
    let crate::vcl::DialogResult::Text(Some(s)) = r else { return };
    let Some(v) = parse_float(s.trim()) else { return };
    if what == "zoom" {
        if v > 0.0 {
            app.navi.scene.zoom = v;
            ui.set_checked(F, "CheckBox1", true);
            new_calc(app, ui);
        }
        return;
    }
    let Some(t) = what.strip_prefix("val").and_then(|n| n.parse::<usize>().ok()).filter(|t| *t < 14) else { return };
    let v = if (6..9).contains(&t) { v.to_radians() } else { v };
    let st = &mut app.navi;
    st.vals[t] = v;
    st.pos0[t] = v;
    if t < 3 {
        st.scene.julia_c[t] = v;
    }
    // the slider starts again from the typed value
    for i in 1..=14 {
        let n = format!("TrackBarEx{i}");
        if ui.f(F).id(&n).is_some() && ui.tag(F, &n) as usize == t {
            ui.set_position(F, &n, 0);
        }
    }
    apply_value(app, ui, t, true);
}

fn enable_size_buttons(ui: &mut Ui) {
    let i = ui.item_index(F, "NaviSizeCmb");
    let n = ui.c(F, "NaviSizeCmb").items.len() as i32;
    ui.set_enabled(F, "DecreaseNaviSizeBtn", i > 0);
    ui.set_enabled(F, "IncreaseNaviSizeBtn", i < n - 1);
}

fn zoom_step(app: &mut Mb3d, ui: &mut Ui, up: bool) {
    app.navi.scene.zoom *= if up { 1.414 } else { 0.707 };
    set_zoom(app, ui);
    new_calc(app, ui);
}

fn toggle_adjust_panel(app: &mut Mb3d, ui: &mut Ui) {
    let panel2 = ui.visible(F, "Panel2");
    if ui.visible(F, "Panel3") {
        ui.set_visible(F, "Panel3", false);
        set_window_size(app, ui, panel2);
        app.ini.set("NaviPanelShow", "0");
    } else {
        ui.set_visible(F, "Panel3", true);
        set_window_size(app, ui, panel2);
        app.ini.set("NaviPanelShow", "1");
        if app.navi.adjust_first {
            app.navi.adjust_first = false;
            app.navi.findex = (0, 0);
            reset_julia_vals(app, ui);
            reset_4d(app, ui);
            app.navi.range = [1.0; 14];
            app.navi.vtype = [0; 14];
            app.navi.vtype[9] = 2;
            app.navi.range[9] = 5.0;
            app.navi.vtype[13] = 2;
            for k in 10..14 {
                app.navi.vtype[k] = -5;
            }
            let j = app.navi.scene.julia;
            ui.set_visible(F, "Panel4", j);
            adjust_panel3_positions(ui);
            update_adjust(app, ui, "");
        }
    }
}

/// `FormKeyDown`: the navigation keys.
fn form_key_down(app: &mut Mb3d, ui: &mut Ui, key: u16, shift: bool) {
    if key == 27 && app.navi.look {
        set_look(app, ui, false);
        return;
    }
    let mut k = key;
    if (k == 112 || k == 113) && parse_float(&ui.text(F, "Edit4")).is_some() {
        let d = ptf(&ui.text(F, "Edit4")) * if k == 112 { 0.8 } else { 1.25 };
        ui.set_text(F, "Edit4", &format!("{:.1}", d.clamp(1.0, 10000.0)));
        set_zoom(app, ui);
        new_calc(app, ui);
        return;
    }
    if (k == 114 || k == 115) && parse_float(&ui.text(F, "Edit3")).is_some() {
        let d = ptf(&ui.text(F, "Edit3")) * if k == 114 { 0.8 } else { 1.25 };
        ui.set_text(F, "Edit3", &format!("{:.1}", d.clamp(1.0, 360.0)));
        new_calc(app, ui);
        return;
    }
    if k == 116 {
        let i = (ui.item_index(F, "RadioGroup2") + 1) % 3;
        ui.set_item_index(F, "RadioGroup2", i);
        new_calc(app, ui);
        return;
    }
    if k == 117 {
        let c = !ui.checked(F, "CheckBox1");
        ui.set_checked(F, "CheckBox1", c);
        ui.set_visible(F, "UpDown3", c);
        return;
    }
    // the edits take their keys
    if let Some(fc) = ui.focused(F) {
        if fc.starts_with("Edit") {
            return;
        }
    }
    if ui.checked(F, "CheckBox2") {
        k = match k {
            90 => 87,
            65 => 81,
            81 => 65,
            87 => 0,
            k => k,
        };
    }
    let t = match k {
        69 => 1,
        67 | 81 => 2,
        65 => 3,
        68 => 4,
        73 => 7,
        75 => 8,
        74 => 9,
        76 => 10,
        87 => 5,
        83 => 6,
        85 => 11,
        79 => 12,
        70 => {
            if !ui.checked(F, "CheckBox5") {
                ui.post(F, "SpeedButton18", "SpeedButton18Click", Ev::Click);
            }
            return;
        }
        _ => return,
    };
    navigate(app, ui, t, shift);
}
