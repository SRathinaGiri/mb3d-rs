//! The formula window (formula/FormulaGUI.pas, `TFormulaGUIForm`): six
//! hybrid slots, the formula lists that pop up under the 3D / 3Da / 4D /
//! 4Da / Ads / dIFS buttons, the formula options, bailout, iterations and
//! the hybrid type (alternating, interpolated, DE combined).

use super::util::{fts, fts_single, parse_float, pti, ptf};
use super::Mb3d;
use crate::formulas::Formula;
use crate::scene::{DeCombParams, FormulaEntry, InsideMode};
use crate::vcl::{DialogResult, Ev, Event, Ui};

const F: &str = "FormulaGUIForm";
const PID180: f64 = std::f64::consts::PI / 180.0;
/// the buttons with a formula list (ListBoxEx1..12, ListBoxEx11 = search)
const LIST_COUNT: usize = 12;

#[derive(Clone, Default)]
pub struct State {
    /// the six slots; None = empty (`iFnr = -1`)
    pub slots: Vec<Option<Formula>>,
    pub its: [i32; 6],
    /// weights of the interpolation hybrid (`PSingle(@iItCount)^`)
    pub weights: [f32; 2],
    /// TabControl2: 0 alternating, 1 interpolation, 2 DE combination
    pub hybrid: i32,
    /// `bHybOpt1 shr 4`: repeat from slot
    pub repeat1: usize,
    /// `bHybOpt2`: start of the second part, repeat of the second part
    pub start2: usize,
    pub repeat2: usize,
    pub decomb_kind: i32,
    pub decomb_smooth: f32,
    pub mix_pow: f32,
    pub mix_color: i32,
    pub user_change: bool,
    /// the list that is shown (tag of the list box)
    pub open_list: Option<usize>,
    pub hide_ticks: i32,
    pub highlighted: String,
    pub names_loaded: bool,
}

fn slots_init(st: &mut State) {
    if st.slots.len() != 6 {
        st.slots = vec![None; 6];
    }
}

fn tab(ui: &Ui) -> usize {
    ui.c(F, "TabControl1").tab_index.clamp(0, 5) as usize
}

/// Sets an option of a formula by its index.
pub fn set_option_index(f: &mut Formula, i: usize, v: f64) {
    if let Formula::Custom(_) = f {
        let _ = f.set_option(&format!("option{i}"), v);
    } else if let Some((k, _)) = f.options().get(i) {
        let _ = f.set_option(k, v);
    }
}

/// `UpdateFromHeader` + the slots from the scene.
pub fn update_from_header(app: &mut Mb3d, ui: &mut Ui) {
    let sc = &app.scene;
    let st = &mut app.formula;
    slots_init(st);
    st.user_change = false;
    for i in 0..6 {
        st.slots[i] = None;
        st.its[i] = 0;
    }
    for (i, e) in sc.formulas.iter().take(6).enumerate() {
        let placeholder = i > 0 && e.iterations == 0 && e.formula.name() == "Integer Power";
        if !placeholder {
            st.slots[i] = Some(e.formula.clone());
            st.its[i] = e.iterations;
        }
    }
    st.hybrid = if sc.interpolation.is_some() { 1 } else if sc.decomb.is_some() { 2 } else { 0 };
    if let Some(w) = sc.interpolation {
        st.weights = w;
    }
    st.repeat1 = sc.repeat_from;
    if let Some(d) = &sc.decomb {
        st.start2 = d.start2;
        st.repeat2 = d.repeat2;
        st.decomb_kind = d.kind as i32 - 1;
        st.decomb_smooth = d.smooth;
        st.mix_pow = d.mix_pow;
        st.mix_color = d.mix_color as i32;
        ui.set_text(F, "MaxIterHybridPart2Edit", &d.iterations2.to_string());
    } else {
        st.start2 = 1;
        st.repeat2 = 1;
    }
    ui.set_text(F, "RBailoutEdit", &fts(sc.effective_rstop()));
    ui.set_text(F, "MaxIterEdit", &sc.iterations.to_string());
    ui.set_text(F, "MinIterEdit", &sc.min_iterations.to_string());
    ui.set_text(F, "XWEdit", &fts(sc.rot_4d[0] / PID180));
    ui.set_text(F, "YWEdit", &fts(sc.rot_4d[1] / PID180));
    ui.set_text(F, "ZWEdit", &fts(sc.rot_4d[2] / PID180));
    ui.set_checked(F, "CheckBox2", sc.disable_analytic_de);
    ui.set_item_index(F, "ComboBox1", match sc.inside {
        InsideMode::Outside => 0,
        InsideMode::Inside => 1,
        _ => 2,
    });
    ui.set_item_index(F, "DECombineCmb", st.decomb_kind.clamp(0, 5));
    ui.cm(F, "TabControl2").tab_index = st.hybrid;
    tab_control2_update(app, ui);
    app.formula.user_change = true;
}

/// Formula settings into the scene (`MakeHeader`'s part of this window).
pub fn make_header(app: &mut Mb3d, ui: &mut Ui) {
    let st = &mut app.formula;
    slots_init(st);
    let sc = &mut app.scene;
    sc.iterations = pti(&ui.text(F, "MaxIterEdit")).max(1);
    sc.min_iterations = pti(&ui.text(F, "MinIterEdit")).max(0);
    sc.rstop = parse_float(&ui.text(F, "RBailoutEdit")).filter(|v| *v > 0.0);
    sc.rot_4d = [ptf(&ui.text(F, "XWEdit")) * PID180, ptf(&ui.text(F, "YWEdit")) * PID180, ptf(&ui.text(F, "ZWEdit")) * PID180];
    sc.disable_analytic_de = ui.checked(F, "CheckBox2");
    sc.inside = match ui.item_index(F, "ComboBox1") {
        1 => InsideMode::Inside,
        2 => InsideMode::Both,
        _ => InsideMode::Outside,
    };
    let last = (0..6).rev().find(|&i| st.slots[i].is_some()).unwrap_or(0);
    sc.formulas = (0..=last)
        .map(|i| match &st.slots[i] {
            Some(f) => FormulaEntry { formula: f.clone(), iterations: if st.hybrid == 1 { (i < 2) as i32 } else { st.its[i] } },
            None => FormulaEntry { formula: Formula::default_for("Integer Power").unwrap(), iterations: 0 },
        })
        .collect();
    if sc.formulas.is_empty() {
        sc.formulas.push(FormulaEntry { formula: Formula::default_for("Integer Power").unwrap(), iterations: 1 });
    }
    sc.repeat_from = st.repeat1.min(5);
    sc.interpolation = if st.hybrid == 1 { Some(st.weights) } else { None };
    sc.decomb = if st.hybrid == 2 {
        let kind = (ui.item_index(F, "DECombineCmb").clamp(0, 5) + 1) as u8;
        let mut d = DeCombParams::new(pti(&ui.text(F, "MaxIterHybridPart2Edit")).max(1));
        d.kind = kind;
        d.start2 = st.start2.clamp(1, 5);
        d.end1 = d.start2 - 1;
        d.end2 = 5;
        d.repeat2 = st.repeat2.clamp(d.start2, 5);
        if kind < 6 {
            d.smooth = ptf(&ui.text(F, "Edit23")).clamp(0.0, 100.0) as f32;
        } else {
            d.mix_color = pti(&ui.text(F, "Edit23")).clamp(0, 2) as u8;
            let p = ptf(&ui.text(F, "Edit25")).clamp(-100.0, 100.0) as f32;
            d.mix_pow = if p == 0.0 { 1e-6 } else { p };
        }
        Some(d)
    } else {
        None
    };
}

fn formula_de(name: &str) -> i32 {
    crate::formulas::Formula::default_for(name).map(|f| f.de_option()).unwrap_or(0)
}

/// FavouriteList.txt in the formula folder: "status name" lines.
fn favourites(app: &Mb3d) -> std::collections::HashMap<String, i32> {
    let p = app.ini.dir(super::ini::DIR_FORMULAS).join("FavouriteList.txt");
    let mut m = std::collections::HashMap::new();
    if let Ok(t) = std::fs::read_to_string(p) {
        for l in t.lines() {
            let mut it = l.splitn(2, ' ');
            if let (Some(a), Some(b)) = (it.next(), it.next()) {
                if let Ok(v) = a.trim().parse::<i32>() {
                    m.insert(b.trim().to_ascii_lowercase(), v);
                }
            }
        }
    }
    m
}

fn store_favourite(app: &Mb3d, name: &str, status: i32) {
    let p = app.ini.dir(super::ini::DIR_FORMULAS).join("FavouriteList.txt");
    let mut m: Vec<(i32, String)> = std::fs::read_to_string(&p)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (a, b) = l.split_once(' ')?;
            Some((a.trim().parse().ok()?, b.trim().to_string()))
        })
        .filter(|(_, n)| !n.eq_ignore_ascii_case(name))
        .collect();
    if status != 1 {
        m.push((status, name.to_string()));
    }
    let t: String = m.iter().map(|(s, n)| format!("{s} {n}\r\n")).collect();
    let _ = std::fs::write(p, t);
}

/// `LoadFormulaNames`: fills the formula lists by DE type.
pub fn load_formula_names(app: &mut Mb3d, ui: &mut Ui) {
    let fav = favourites(app);
    let mut lists: Vec<Vec<(String, i32)>> = vec![Vec::new(); LIST_COUNT + 1];
    let max_h = {
        let tc = ui.c(F, "TabControl1").height;
        let sb = ui.c(F, "SpeedButtonEx1");
        tc - sb.top - sb.height - 4
    };
    let ih = ui.c(F, "ListBoxEx1").item_height.max(10);
    let full = |l: &Vec<(String, i32)>| (l.len() as i32 + 1) * ih >= max_h;
    lists[4].push((" ".into(), 1));
    let mut names: Vec<(String, i32)> = Formula::all_names().iter().map(|n| (n.to_string(), formula_de(n))).collect();
    let builtin: Vec<String> = names.iter().map(|n| n.0.to_ascii_lowercase()).collect();
    for (n, de) in crate::formulas::list_custom_with_de() {
        if !builtin.contains(&n.to_ascii_lowercase()) {
            names.push((n, de));
        }
    }
    for (n, de) in names {
        let stat = fav.get(&n.to_ascii_lowercase()).copied().unwrap_or(1);
        if stat <= -2 {
            continue;
        }
        let k = match de {
            2 | 11 => 2,
            4 => 3,
            5 | 6 => 4,
            -1 | -2 => {
                if !full(&lists[5]) {
                    5
                } else if !full(&lists[6]) {
                    6
                } else {
                    8
                }
            }
            20 => {
                if !full(&lists[9]) {
                    9
                } else {
                    10
                }
            }
            21 | 22 => 12,
            _ => {
                if !full(&lists[1]) {
                    1
                } else {
                    7
                }
            }
        };
        lists[k].push((n, stat));
    }
    for (k, list) in lists.iter().enumerate().skip(1) {
        let name = format!("ListBoxEx{k}");
        if !ui.f(F).has(&name) || k == 11 {
            continue;
        }
        set_list(ui, &name, list, max_h);
    }
    // the dIFS buttons (SSE2 machines in MB3D)
    let w9 = ui.c(F, "SpeedButtonEx9").width;
    if !ui.visible(F, "SpeedButtonEx9") {
        ui.cm(F, "Bevel1").width += w9 * 3;
        for b in ["SpeedButtonEx9", "SpeedButtonEx10", "SpeedButtonEx11"] {
            ui.set_visible(F, b, true);
        }
    }
    app.formula.names_loaded = true;
}

/// `SetListBoxWidth`: the items with the favourite colours.
fn set_list(ui: &mut Ui, name: &str, list: &[(String, i32)], max_h: i32) {
    let fav_col = |s: i32| match s {
        s if s > 1 => Some(0xFF60D060u32),
        s if s < 1 => Some(0xFFD07070u32),
        _ => None,
    };
    let items: Vec<String> = list.iter().map(|(n, _)| format!("    {n}")).collect();
    let colors: Vec<Option<u32>> = list.iter().map(|(_, s)| fav_col(*s)).collect();
    let font = crate::vcl::font::Font { height: -9, ..Default::default() };
    let w = items.iter().map(|s| crate::vcl::font::text_width(font.face(), font.em(), s).ceil() as i32).max().unwrap_or(40) + 28;
    let c = ui.cm(F, name);
    c.font = Some(font);
    c.item_height = 12;
    c.height = (items.len() as i32 * c.item_height + 4).min(max_h.max(40));
    c.width = w;
    c.item_colors = colors;
    c.selected = vec![false; items.len()];
    c.items = items;
    c.item_index = -1;
    c.top_index = 0;
}

fn last_words(s: &str) -> String {
    s.trim().to_string()
}

/// `SetTabNames`.
fn set_tab_names(app: &mut Mb3d, ui: &mut Ui) {
    let st = &app.formula;
    let n = if st.hybrid == 1 { 2 } else { 6 };
    let tabs: Vec<String> = (0..n)
        .map(|t| {
            let used = if st.hybrid == 1 { st.slots[t].is_some() } else { st.its[t] > 0 && st.slots[t].is_some() };
            let base = if t == 0 { "Formula1".to_string() } else { format!("Fo.{}", t + 1) };
            if used {
                format!("{base} \u{00BB}")
            } else {
                base
            }
        })
        .collect();
    let c = ui.cm(F, "TabControl1");
    c.tabs = tabs;
    c.tab_index = c.tab_index.clamp(0, n as i32 - 1);
    ui.set_caption(F, "LabelItCount", if n == 6 { "Iterationcount" } else { "Weight" });
    ui.set_visible(F, "CheckBox1", n == 6);
}

/// `TabControl1Change`: shows the formula of the selected slot.
pub fn tab_control1_change(app: &mut Mb3d, ui: &mut Ui) {
    slots_init(&mut app.formula);
    set_tab_names(app, ui);
    let t = tab(ui);
    let st = app.formula.clone();
    let alt = st.hybrid != 1;
    ui.set_enabled(F, "ExchangeFormulaRightBtn", t == 0 || (alt && t < 5));
    ui.set_enabled(F, "ExchangeFormulaLeftBtn", t > 0);
    app.formula.user_change = false;
    if alt {
        ui.set_text(F, "EditItCount", &st.its[t].max(0).to_string());
    } else {
        ui.set_text(F, "EditItCount", &fts_single(st.weights[t.min(1)] as f64));
    }
    let name = st.slots[t].as_ref().map(|f| f.name()).unwrap_or_default();
    ui.set_text(F, "ComboEdit1", &name);
    ui.set_enabled(F, "EditJITFormulaItm", name.starts_with("JIT"));
    let opts = st.slots[t].as_ref().map(|f| f.options()).unwrap_or_default();
    for i in 0..16 {
        let (e, l) = (format!("Edit{}", i + 1), format!("Label{}", i + 1));
        let vis = i < opts.len();
        ui.set_visible(F, &e, vis);
        ui.set_visible(F, &l, vis);
        if vis {
            ui.set_text(F, &e, &fts(opts[i].1));
            ui.cm(F, &e).font = None;
            ui.set_caption(F, &l, &opts[i].0);
        }
    }
    ui.set_visible(F, "Panel3", st.hybrid == 2);
    check_4d_and_info(app, ui);
    let checked = if st.hybrid == 2 { t == st.repeat1 || t == st.repeat2 } else { t == st.repeat1 };
    ui.set_checked(F, "CheckBox1", checked);
    ui.set_visible(F, "RichEdit1", false);
    ui.set_visible(F, "Button3", false);
    radio_group1_click(app, ui);
    app.formula.user_change = true;
}

/// `Check4DandInfo`: the 4D rotation panel for 4D formulas, the
/// "normalise" button.
fn check_4d_and_info(app: &mut Mb3d, ui: &mut Ui) {
    let is4d = app.formula.slots.iter().flatten().any(|f| matches!(f.de_option(), 4 | 5 | 6));
    ui.set_visible(F, "Panel2", is4d);
    adjust_tc1_height(ui);
    let t = tab(ui);
    let normal = app.formula.slots[t].as_ref().and_then(|f| f.options().iter().position(|(k, _)| k.to_ascii_uppercase().contains("NORMAL")));
    match normal {
        Some(i) if i + 2 < 16 => {
            let top = ui.c(F, &format!("Label{}", i + 1)).top;
            ui.set_visible(F, "Button4", true);
            ui.cm(F, "Button4").top = top;
        }
        _ => ui.set_visible(F, "Button4", false),
    }
    let has_descr = app.formula.slots[t].as_ref().map(|f| matches!(f, Formula::Custom(_))).unwrap_or(false);
    ui.set_enabled(F, "SpeedButton2", has_descr);
}

fn adjust_tc1_height(ui: &mut Ui) {
    let mut i = if ui.visible(F, "Panel3") { ui.c(F, "Panel3").height } else { 0 };
    if ui.visible(F, "Panel2") {
        i += ui.c(F, "Panel2").height;
    }
    let h = ui.c(F, "TabControl1").height + ui.c(F, "TabControl2").height + ui.c(F, "Panel1").height + i;
    let w = ui.f(F).client_size().0;
    ui.set_client_size(F, w, h);
}

/// `TabControl2Change` without a user change (header -> controls).
fn tab_control2_update(app: &mut Mb3d, ui: &mut Ui) {
    let h = app.formula.hybrid;
    ui.set_caption(F, "Label20", if h == 2 { "Maxits hybrid part1:" } else { "Max. iterations:" });
    ui.set_visible(F, "Label28", h == 2);
    ui.set_visible(F, "UpDown1", h == 2);
    let p = app.formula.start2.max(1) as i64 + 1;
    ui.set_position(F, "UpDown1", p);
    ui.set_caption(F, "Label28", &p.to_string());
    ui.set_visible(F, "Label19", h == 2);
    ui.set_visible(F, "MaxIterHybridPart2Edit", h == 2);
    tab_control1_change(app, ui);
    adjust_tc1_height(ui);
}

/// `RadioGroup1Click`: the DE combination options.
fn radio_group1_click(app: &mut Mb3d, ui: &mut Ui) {
    let k = ui.item_index(F, "DECombineCmb");
    if app.formula.user_change {
        app.formula.decomb_kind = k;
    }
    let l18 = k > 2;
    ui.set_visible(F, "Label18", l18);
    ui.set_visible(F, "Edit23", l18);
    ui.set_visible(F, "Edit25", k > 4);
    ui.set_visible(F, "Label27", k > 4);
    if l18 && k <= 4 {
        ui.set_caption(F, "Label18", "Ds:");
        ui.set_text(F, "Edit23", &fts_single(app.formula.decomb_smooth.clamp(0.0, 100.0) as f64));
        ui.cm(F, "Edit23").hint = "Absolute distance of the smooth combine functions,\nyou can use scientific notation like 3e-5 for small values.\nTry 1/zoom to get close to a working value.".into();
    } else if l18 {
        ui.set_caption(F, "Label18", "Co:");
        ui.cm(F, "Edit23").hint = "Color Option:\n0:  Average of both formulas.\n1:  Color of first formula.\n2:  Color of second formula.".into();
        ui.set_text(F, "Edit23", &app.formula.mix_color.to_string());
        ui.set_text(F, "Edit25", &fts_single(app.formula.mix_pow as f64));
    }
}

/// `CalcRstop`: the bailout from the used formulas.
fn calc_rstop(app: &mut Mb3d, ui: &mut Ui) {
    let st = &app.formula;
    let n = if st.hybrid == 1 { 2 } else { 6 };
    let mut d: f64 = 2.0;
    for i in 0..n {
        let used = if n == 6 { st.its[i] > 0 } else { st.slots[i].is_some() };
        if let (true, Some(f)) = (used, &st.slots[i]) {
            d = d.max(f.default_rstop());
        }
    }
    ui.set_text(F, "RBailoutEdit", &fts(d));
    set_tab_names(app, ui);
}

/// `SelectFormula`: puts a formula into the selected slot.
pub fn select_formula(app: &mut Mb3d, ui: &mut Ui, name: &str) {
    slots_init(&mut app.formula);
    ui.set_visible(F, "ListBoxEx15", false);
    let t = tab(ui);
    let s = name.trim();
    let f = if s.is_empty() { None } else { crate::formulas::lookup(s).ok() };
    match f {
        None => {
            app.formula.slots[t] = None;
            app.formula.its[t] = 0;
            if !s.is_empty() {
                app.message(ui, &format!("Formula '{s}' not found."));
            }
            tab_control1_change(app, ui);
        }
        Some(f) => {
            app.formula.slots[t] = Some(f);
            if app.formula.hybrid != 1 && app.formula.its[t] < 1 {
                app.formula.its[t] = 1;
            }
            tab_control1_change(app, ui);
            calc_rstop(app, ui);
        }
    }
    check_4d_and_info(app, ui);
    // a new formula combination: its card shaders, compiled meanwhile
    app.prewarm_from_forms(ui);
}

/// The number of a formula list from its name (`ListBoxEx12` has the tag
/// 11 in the .dfm, its button `SpeedButtonEx11` the tag 12).
fn list_no(name: &str) -> usize {
    name.trim_start_matches("ListBoxEx").parse().unwrap_or(0)
}

/// The button of formula list `n` (none for the search list 11).
fn list_button(n: usize) -> Option<String> {
    match n {
        11 => None,
        12 => Some("SpeedButtonEx11".into()),
        n => Some(format!("SpeedButtonEx{n}")),
    }
}

fn hide_lists_but(app: &mut Mb3d, ui: &mut Ui, n: usize) {
    for i in 1..=LIST_COUNT {
        let lb = format!("ListBoxEx{i}");
        if i != n && ui.f(F).has(&lb) && ui.visible(F, &lb) {
            ui.set_visible(F, &lb, false);
            if let Some(sb) = list_button(i) {
                if ui.f(F).has(&sb) {
                    ui.cm(F, &sb).down = false;
                }
            }
        }
    }
    if app.formula.open_list.is_some_and(|o| o != n) {
        app.formula.open_list = None;
    }
}

/// Shows a list box under its button (`ListboxPopup`).
fn popup_list(app: &mut Mb3d, ui: &mut Ui, lb: &str, x: i32, y: i32) {
    let tcw = ui.c(F, "TabControl1").width;
    let c = ui.cm(F, lb);
    if c.items.is_empty() {
        return;
    }
    c.left = x.min(tcw - c.width).max(0);
    c.top = y;
    c.visible = true;
    c.enabled = true;
    c.item_index = -1;
    app.formula.open_list = Some(list_no(lb));
    app.formula.hide_ticks = 5;
    ui.set_timer(F, "Timer1", true);
}

fn mouse_in(ui: &mut Ui, ctl: &str) -> bool {
    let (mx, my) = ui.f(F).mouse;
    let r = ui.ctl_rect(F, ctl);
    mx >= r.x && my >= r.y - 2 && mx < r.right() && my < r.bottom()
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    slots_init(&mut app.formula);
    let h = e.handler.as_str();
    match h {
        "FormShow" => {
            if !app.formula.names_loaded {
                load_formula_names(app, ui);
            }
            tab_control1_change(app, ui);
        }
        "FormCreate" => {
            ui.cm(F, "DECombineCmb").hint = "Combination Methods:\nMin: Both fractals are visible, minimum combine the DE's\nMax: Only overlapping parts, maximum combine the DE's\nInv max: Invert DE of second Hybrid and combine 'Max'\nMin lin: Minimum combine with a linear overlap function\nMin nlin: Like S1, nonlinear function, more smooth overlap\nMix: First F1..FendHybrid1 then use FstartHybrid2..F6 (dIFS) with modified vector.".into();
        }
        "FormHide" => {
            let f = ui.f(F);
            let s = format!("{} {}", f.left, f.top);
            app.ini.set("FormulaPos", &s);
        }
        "TabControl1Change" => tab_control1_change(app, ui),
        "TabControl1MouseDown" => {
            if let Ev::MouseDown { button: crate::vcl::MouseButton::Right, x, y, .. } = e.ev {
                let t = tab(ui);
                let items = ["Copythisformulatoformulanr11", "Copythisformulatoformula21", "Copythisformulatoformula31", "Copythisformulatoformula41", "Copythisformulatoformula51", "Copythisformulatoformula61"];
                for (i, it) in items.iter().enumerate() {
                    ui.set_enabled(F, it, i != t);
                }
                ui.set_caption(F, "Shiftallformulasonetotheright1", &format!("Shift formulas {} to 5 a position to the right", t + 1));
                ui.set_visible(F, "Shiftallformulasonetotheright1", t < 5);
                ui.set_caption(F, "Shifttotheleft1", &format!("Shift formulas {} to 6 a position to the left", t + 1));
                ui.popup_menu(F, "PopupMenu2", "TabControl1", x, y);
            }
        }
        "TabControl2Change" => {
            let new = ui.c(F, "TabControl2").tab_index;
            let old = app.formula.hybrid;
            app.formula.hybrid = new;
            if new != 1 && old == 1 {
                for i in 0..2 {
                    app.formula.its[i] = if app.formula.slots[i].is_none() { 0 } else { app.formula.weights[i].round().clamp(0.0, 100.0) as i32 };
                }
            } else if new == 1 && old != 1 {
                for i in 0..2 {
                    app.formula.weights[i] = app.formula.its[i].max(1) as f32;
                }
            }
            if new == 2 {
                app.formula.start2 = app.formula.start2.clamp(1, 5);
            }
            tab_control2_update(app, ui);
        }
        "EditItCountChange" => {
            if app.formula.user_change {
                let t = tab(ui);
                let s = ui.text(F, "EditItCount");
                if app.formula.hybrid != 1 {
                    if let Ok(v) = s.trim().parse::<i32>() {
                        app.formula.its[t] = v;
                    }
                } else if let Some(v) = parse_float(&s) {
                    app.formula.weights[t.min(1)] = v as f32;
                }
                calc_rstop(app, ui);
            }
        }
        "Edit1Change" => {
            if app.formula.user_change {
                let t = tab(ui);
                let i = e.sender.trim_start_matches("Edit").parse::<usize>().unwrap_or(1).saturating_sub(1);
                let txt = ui.text(F, &e.sender);
                match parse_float(&txt) {
                    Some(v) => {
                        if let Some(f) = app.formula.slots[t].as_mut() {
                            set_option_index(f, i, v);
                        }
                        ui.cm(F, &e.sender).font = None;
                    }
                    None => {
                        let f = crate::vcl::font::Font { color: 0xFF80_0000, custom_color: true, ..Default::default() };
                        ui.cm(F, &e.sender).font = Some(f);
                    }
                }
            }
        }
        "CheckBox1Click" => {
            if app.formula.user_change {
                let t = tab(ui);
                let i = if ui.checked(F, "CheckBox1") { t } else { 0 };
                let p = ui.position(F, "UpDown1") as usize;
                if app.formula.hybrid == 2 && t + 1 >= p {
                    app.formula.repeat2 = if i == 0 { p - 1 } else { i };
                } else {
                    app.formula.repeat1 = i;
                }
            }
        }
        "UpDown1Click" => {
            if let Ev::UpDown { up } = e.ev {
                let mut i = app.formula.start2 as i32 + 1;
                if up && i < 6 {
                    i += 1;
                } else if !up && i > 2 {
                    i -= 1;
                }
                ui.set_position(F, "UpDown1", i as i64);
                ui.set_caption(F, "Label28", &i.to_string());
                app.formula.start2 = (i - 1) as usize;
                app.formula.repeat1 = app.formula.repeat1.min(app.formula.start2 - 1);
                app.formula.repeat2 = app.formula.repeat2.clamp(app.formula.start2, 5);
            }
        }
        "RadioGroup1Click" => radio_group1_click(app, ui),
        "SpeedButton3Click" => {
            // reset the formulas
            let st = &mut app.formula;
            st.slots = vec![None; 6];
            st.its = [0; 6];
            st.hybrid = 0;
            st.repeat1 = 0;
            st.start2 = 1;
            st.repeat2 = 1;
            ui.set_text(F, "MinIterEdit", "1");
            ui.set_text(F, "MaxIterEdit", "60");
            ui.set_text(F, "RBailoutEdit", "16");
            ui.cm(F, "TabControl1").tab_index = 0;
            ui.cm(F, "TabControl2").tab_index = 0;
            select_formula(app, ui, "Integer Power");
            app.formula.its[0] = 1;
            tab_control2_update(app, ui);
        }
        "SpeedButtonEx1MouseEnter" => {
            let sb = e.sender.clone();
            let tg = ui.tag(F, &sb) as usize;
            let lb_n = if tg == 12 { 12 } else { tg };
            let lb = format!("ListBoxEx{lb_n}");
            if ui.f(F).has(&lb) && ui.enabled(F, &sb) {
                hide_lists_but(app, ui, lb_n);
                let s = ui.c(F, &sb).rect();
                popup_list(app, ui, &lb, s.x - 2, s.bottom());
                ui.cm(F, &sb).down = true;
            }
        }
        "SpeedButtonEx1MouseLeave" => {
            let tg = ui.tag(F, &e.sender) as usize;
            let lb = format!("ListBoxEx{}", if tg == 12 { 12 } else { tg });
            ui.cm(F, &e.sender).down = false;
            if ui.f(F).has(&lb) && !mouse_in(ui, &lb) {
                ui.set_visible(F, &lb, false);
                app.formula.open_list = None;
            }
        }
        "ListBoxEx1MouseEnter" => {
            ui.set_timer(F, "Timer4", false);
            hide_lists_but(app, ui, list_no(&e.sender));
        }
        "ListBoxEx1MouseLeave" => {
            if let Some(sb) = list_button(list_no(&e.sender)) {
                if ui.f(F).has(&sb) && !mouse_in(ui, &sb) && !mouse_in(ui, &e.sender) {
                    ui.set_timer(F, "Timer4", true);
                }
            }
        }
        "ListBoxEx1MouseMove" => {
            if let Ev::MouseMove { y, .. } = e.ev {
                let c = ui.c(F, &e.sender);
                let i = (y - 2) / c.item_height.max(8) + c.top_index;
                if i >= 0 && (i as usize) < c.items.len() {
                    ui.cm(F, &e.sender).item_index = i;
                }
            }
        }
        "ListBoxEx1MouseDown" | "ListBoxEx11MouseDown" => {
            let lb = e.sender.clone();
            match e.ev {
                Ev::MouseDown { button: crate::vcl::MouseButton::Left, .. } => {
                    let it = ui.c(F, &lb).item_text();
                    if !it.trim().is_empty() {
                        select_formula(app, ui, &last_words(&it));
                    }
                    ui.set_visible(F, &lb, false);
                    if let Some(sb) = list_button(list_no(&lb)) {
                        if ui.f(F).has(&sb) {
                            ui.cm(F, &sb).down = false;
                        }
                    }
                    app.formula.open_list = None;
                    ui.set_timer(F, "Timer1", false);
                }
                Ev::MouseDown { button: crate::vcl::MouseButton::Right, x, y, .. } => {
                    let it = ui.c(F, &lb).item_text();
                    if !it.trim().is_empty() {
                        app.formula.highlighted = last_words(&it);
                        ui.set_visible(F, "Hidethisformula1", true);
                        ui.popup_menu(F, "PopupMenu1", &lb, x + 3, y);
                    }
                }
                _ => {}
            }
        }
        "Timer1Timer" => {
            // hide the shown list when the mouse left it and its button
            let Some(n) = app.formula.open_list else {
                ui.set_timer(F, "Timer1", false);
                return;
            };
            let lb = format!("ListBoxEx{n}");
            let sb = format!("SpeedButtonEx{}", if n == 12 { 11 } else { n });
            let in_lb = mouse_in(ui, &lb);
            let in_sb = ui.f(F).has(&sb) && mouse_in(ui, &sb);
            if !in_lb && !in_sb {
                app.formula.hide_ticks -= 1;
                if app.formula.hide_ticks <= 0 {
                    ui.set_visible(F, &lb, false);
                    if ui.f(F).has(&sb) {
                        ui.cm(F, &sb).down = false;
                    }
                    app.formula.open_list = None;
                    ui.set_timer(F, "Timer1", false);
                }
            } else {
                app.formula.hide_ticks = 5;
            }
        }
        "Timer4Timer" => {
            hide_lists_but(app, ui, 0);
            ui.set_timer(F, "Timer4", false);
        }
        "ComboEdit1Change" => {
            if app.formula.user_change {
                let txt = ui.text(F, "ComboEdit1").trim().to_ascii_uppercase();
                if !txt.is_empty() {
                    let mut items = Vec::new();
                    for k in 1..=LIST_COUNT {
                        let n = format!("ListBoxEx{k}");
                        if k == 11 || !ui.f(F).has(&n) {
                            continue;
                        }
                        for it in &ui.c(F, &n).items {
                            if it.trim().to_ascii_uppercase().contains(&txt) {
                                items.push((it.trim().to_string(), 1));
                            }
                        }
                    }
                    if items.is_empty() {
                        ui.set_visible(F, "ListBoxEx11", false);
                    } else {
                        let tc = ui.c(F, "TabControl1").height;
                        let ce = ui.c(F, "ComboEdit1").rect();
                        set_list(ui, "ListBoxEx11", &items, tc - ce.bottom());
                        popup_list(app, ui, "ListBoxEx11", ce.x, ce.bottom());
                    }
                }
            }
        }
        "ComboEdit1KeyDown" => {
            if let Ev::KeyDown { key, .. } = e.ev {
                let vis = ui.visible(F, "ListBoxEx11");
                match key {
                    13 => {
                        let name = if vis && ui.item_index(F, "ListBoxEx11") >= 0 { ui.c(F, "ListBoxEx11").item_text() } else { ui.text(F, "ComboEdit1") };
                        select_formula(app, ui, &last_words(&name));
                        ui.set_visible(F, "ListBoxEx11", false);
                    }
                    40 if vis => {
                        let c = ui.cm(F, "ListBoxEx11");
                        c.item_index = (c.item_index + 1).min(c.items.len() as i32 - 1);
                    }
                    38 if vis => {
                        let c = ui.cm(F, "ListBoxEx11");
                        c.item_index = (c.item_index - 1).max(0);
                    }
                    _ => {}
                }
            }
        }
        "ComboEdit1Exit" => {
            if !mouse_in(ui, "ListBoxEx11") {
                ui.set_visible(F, "ListBoxEx11", false);
            }
        }
        "SpeedButton11Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "M3D formula (*.m3f)|*.m3f".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_FORMULAS)),
                ..Default::default()
            };
            ui.open_dialog("formula:loadm3f", &o);
        }
        "SpeedButton2Click" => {
            ui.set_visible(F, "ListBoxEx15", false);
            if ui.visible(F, "RichEdit1") {
                ui.set_visible(F, "Button3", false);
                ui.set_visible(F, "RichEdit1", false);
            } else {
                let t = tab(ui);
                let d = match &app.formula.slots[t] {
                    Some(Formula::Custom(c)) => c.def.description.clone(),
                    _ => String::new(),
                };
                ui.set_text(F, "RichEdit1", &d);
                ui.set_visible(F, "Button3", true);
                ui.set_visible(F, "RichEdit1", true);
            }
        }
        "Button3Click" => {
            for c in ["Button3", "RichEdit1", "ListBoxEx15"] {
                ui.set_visible(F, c, false);
            }
        }
        "Button4Click" => {
            // normalise the 3 values of a "normal" option
            let t = tab(ui);
            if let Some(f) = app.formula.slots[t].as_mut() {
                let o = f.options();
                if let Some(i) = o.iter().position(|(k, _)| k.to_ascii_uppercase().contains("NORMAL")) {
                    if i + 2 < o.len() {
                        let d = (o[i].1 * o[i].1 + o[i + 1].1 * o[i + 1].1 + o[i + 2].1 * o[i + 2].1).sqrt();
                        let d = if d > 0.0 { 1.0 / d } else { 0.0 };
                        for n in 0..3 {
                            set_option_index(f, i + n, o[i + n].1 * d);
                        }
                    }
                }
            }
            tab_control1_change(app, ui);
        }
        "SpeedButton4Click" => {
            // the hidden formulas
            if ui.visible(F, "ListBoxEx15") {
                ui.set_visible(F, "ListBoxEx15", false);
                ui.set_visible(F, "Button3", false);
            } else {
                let mut hidden: Vec<String> = favourites(app).into_iter().filter(|(_, s)| *s <= -2).map(|(n, _)| n).collect();
                hidden.sort();
                ui.set_items(F, "ListBoxEx15", hidden);
                ui.set_visible(F, "ListBoxEx15", true);
                ui.set_enabled(F, "ListBoxEx15", true);
                ui.set_visible(F, "Button3", true);
            }
        }
        "ListBoxEx15MouseDown" => {
            if let Ev::MouseDown { button: crate::vcl::MouseButton::Right, x, y, .. } = e.ev {
                let it = ui.c(F, "ListBoxEx15").item_text();
                if !it.trim().is_empty() {
                    app.formula.highlighted = it.trim().to_string();
                    ui.set_visible(F, "Hidethisformula1", false);
                    ui.popup_menu(F, "PopupMenu1", "ListBoxEx15", x + 3, y);
                }
            }
        }
        "Hidethisformula1Click" => {
            let tg = ui.tag(F, &e.sender) as i32;
            let name = app.formula.highlighted.clone();
            store_favourite(app, &name, tg);
            load_formula_names(app, ui);
            if ui.visible(F, "ListBoxEx15") {
                ui.set_visible(F, "ListBoxEx15", false);
            }
        }
        "Copythisformulatoformulanr11Click" => {
            let dest = ui.tag(F, &e.sender).clamp(0, 5) as usize;
            let src = tab(ui);
            if dest != src {
                app.formula.slots[dest] = app.formula.slots[src].clone();
                app.formula.its[dest] = app.formula.its[src];
                tab_control1_change(app, ui);
            }
        }
        "Shiftallformulasonetotheright1Click" => {
            let s = tab(ui);
            for i in (s + 1..6).rev() {
                app.formula.slots[i] = app.formula.slots[i - 1].clone();
                app.formula.its[i] = app.formula.its[i - 1];
            }
            app.formula.slots[s] = None;
            app.formula.its[s] = 0;
            if app.formula.repeat1 >= s {
                app.formula.repeat1 = (app.formula.repeat1 + 1).min(5);
            }
            tab_control1_change(app, ui);
        }
        "Shifttotheleft1Click" => {
            let s = tab(ui).saturating_sub(1);
            for i in s..5 {
                app.formula.slots[i] = app.formula.slots[i + 1].clone();
                app.formula.its[i] = app.formula.its[i + 1];
            }
            app.formula.slots[5] = None;
            app.formula.its[5] = 0;
            if app.formula.repeat1 >= s.max(1) {
                app.formula.repeat1 -= 1;
            }
            tab_control1_change(app, ui);
        }
        "Copythevaluesto1Click" => {
            let src = tab(ui);
            let d = if src == 0 { 2 } else { 1 };
            ui.input_query("formula:copyvalues", "Copy the values", "to formula nr:", &d.to_string());
        }
        "ExchangeFormulaLeftBtnClick" | "ExchangeFormulaRightBtnClick" => {
            let t = tab(ui);
            let o = if h.contains("Left") { t.checked_sub(1) } else { (t < 5).then_some(t + 1) };
            if let Some(o) = o {
                app.formula.slots.swap(t, o);
                app.formula.its.swap(t, o);
                ui.cm(F, "TabControl1").tab_index = o as i32;
                tab_control1_change(app, ui);
            }
        }
        "JITFormulaBtnClick" => {
            let r = ui.c(F, "JITFormulaBtn").rect();
            let _ = r;
            let hgt = ui.c(F, "JITFormulaBtn").height;
            ui.popup_menu(F, "JITPopupMenu", "JITFormulaBtn", 0, hgt);
        }
        "NewJITFormulaItmClick" => super::forms_jit_open(app, ui, None),
        "EditJITFormulaItmClick" => {
            let n = ui.text(F, "ComboEdit1");
            super::forms_jit_open(app, ui, Some(n));
        }
        "FormMouseWheel" => {}
        _ => {}
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("loadm3f", DialogResult::File(Some(p))) => {
            let dir = app.ini.dir(super::ini::DIR_FORMULAS);
            let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            if p.parent() != Some(dir.as_path()) {
                let _ = std::fs::create_dir_all(&dir);
                let dest = dir.join(p.file_name().unwrap_or_default());
                if !dest.exists() {
                    let _ = std::fs::copy(p, dest);
                }
            }
            if let Some(par) = p.parent() {
                crate::formulas::add_formula_dir(par.to_path_buf());
            }
            load_formula_names(app, ui);
            select_formula(app, ui, &name);
        }
        ("copyvalues", DialogResult::Text(Some(s))) => {
            let src = tab(ui);
            let dest = (pti(s) - 1).clamp(0, 5) as usize;
            if dest != src {
                if let (Some(a), Some(b)) = (app.formula.slots[src].clone(), app.formula.slots[dest].as_mut()) {
                    for (i, (_, v)) in a.options().into_iter().enumerate() {
                        set_option_index(b, i, v);
                    }
                }
                tab_control1_change(app, ui);
            }
        }
        _ => {}
    }
}
