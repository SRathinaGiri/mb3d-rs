//! MutaGen (mutagen/MutaGen.pas, MutaGenGUI.pas): random variations of a
//! parameter set.  A generation is a family tree of 15 parameter sets: the
//! parent (1), its two children (1.1, 1.2), four grandchildren and eight
//! great-grandchildren, each a mutation of its parent.  A mutation is a
//! random choice of operations: add, replace or remove a hybrid formula,
//! change formula options, switch julia mode and its constant, change the
//! iteration counts.  Each child is "probed" first: up to 9 candidates are
//! rendered as tiny images, and the one that shows the most structure
//! (edge coverage) while differing enough from the parent is kept.  Picking
//! a set and mutating again breeds the next generation.

use crate::formulas::Formula;
use crate::scene::{FormulaEntry, Scene};
use std::sync::OnceLock;

/// `TMutationConfig` with MB3D's defaults.
#[derive(Clone, Debug)]
pub struct MutationConfig {
    pub formula_weight: f64,
    pub params_weight: f64,
    pub params_strength: f64,
    pub julia_weight: f64,
    pub julia_strength: f64,
    pub iterations_weight: f64,
    pub iterations_strength: f64,
    pub probing: bool,
    pub probe_width: usize,
    pub probe_height: usize,
    pub probe_max: usize,
    pub probe_min_coverage: f64,
}

impl Default for MutationConfig {
    fn default() -> Self {
        MutationConfig {
            formula_weight: 0.75,
            params_weight: 1.0,
            params_strength: 1.0,
            julia_weight: 0.5,
            julia_strength: 1.0,
            iterations_weight: 0.5,
            iterations_strength: 1.0,
            probing: true,
            probe_width: 40,
            probe_height: 32,
            probe_max: 9,
            probe_min_coverage: 0.32,
        }
    }
}

/// A small random generator (MB3D uses Delphi's `Random`).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }

    /// Seeded from the clock.
    pub fn from_time() -> Rng {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1);
        Rng::new(t)
    }

    /// 0 <= x < 1
    pub fn f64(&mut self) -> f64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `NextRandomInt`: 0 <= i < max
    pub fn int(&mut self, max: usize) -> usize {
        ((max as f64 * self.f64()) as usize).min(max.saturating_sub(1))
    }
}

/// Formula families (`TFormulaCategory`), from the DE option.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    D3,
    D3a,
    D4,
    D4a,
    Ads,
    Difs,
    DifsA,
}

/// `TFormulaNamesLoader.AddFormulaName`
pub fn category_of_de(de: i32) -> Category {
    match de {
        2 | 11 => Category::D3a,
        4 => Category::D4,
        5 | 6 => Category::D4a,
        -1 | -2 => Category::Ads,
        20 => Category::Difs,
        21 | 22 => Category::DifsA,
        _ => Category::D3,
    }
}

/// All formulas with their categories: the built-in ones and the `.m3f`
/// files of the formula folders (`TFormulaNamesLoader.LoadFormulas`).
pub fn formula_names() -> &'static [(String, Category)] {
    static N: OnceLock<Vec<(String, Category)>> = OnceLock::new();
    N.get_or_init(|| {
        let mut v: Vec<(String, Category)> = Formula::all_names()
            .iter()
            .map(|n| {
                let f = Formula::default_for(n).unwrap();
                (n.to_string(), category_of_de(f.de_option()))
            })
            .collect();
        for name in crate::formulas::list_custom() {
            if let Ok(def) = crate::formulas::load_custom(&name) {
                // [SOURCE] formulas cannot run here
                if !def.code.is_empty() || def.jit.is_some() {
                    v.push((name, category_of_de(def.de_option)));
                }
            }
        }
        v
    })
}

fn names_of(cat: Category) -> Vec<&'static str> {
    formula_names().iter().filter(|(_, c)| *c == cat).map(|(n, _)| n.as_str()).collect()
}

fn category_of_name(name: &str) -> Category {
    formula_names().iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, c)| *c).unwrap_or(Category::D3)
}

fn is_empty(s: &Scene, i: usize) -> bool {
    s.formulas.get(i).is_none_or(|f| f.iterations == 0)
}

fn non_empty(s: &Scene) -> Vec<usize> {
    (0..s.formulas.len().min(6)).filter(|&i| !is_empty(s, i)).collect()
}

/// `SetFormulaName`: a formula with its default options in slot `i`
/// (at least one iteration); false if the name cannot be loaded.
fn set_formula(s: &mut Scene, i: usize, name: &str) -> bool {
    let Ok(f) = crate::formulas::lookup(name) else { return false };
    while s.formulas.len() <= i {
        s.formulas.push(FormulaEntry { formula: Formula::default_for("Integer Power").unwrap(), iterations: 0 });
    }
    let its = s.formulas[i].iterations.max(1);
    s.formulas[i] = FormulaEntry { formula: f, iterations: its };
    true
}

/// `Clear` of a formula slot.
fn clear_formula(s: &mut Scene, i: usize) {
    if let Some(f) = s.formulas.get_mut(i) {
        f.iterations = 0;
    }
    while s.formulas.len() > 1 && s.formulas.last().is_some_and(|f| f.iterations == 0) {
        s.formulas.pop();
    }
}

fn is_int_type(t: u8) -> bool {
    matches!(t, 2 | 10 | 20)
}

/// `RandomizeParamValue`
fn randomize_value(old: f64, integer: bool, strength: f64, rng: &mut Rng) -> f64 {
    let delta = if integer {
        let d = (1.0 + rng.f64() * 1.5) * strength;
        if rng.f64() > 0.5 {
            -d
        } else {
            d
        }
    } else if old.abs() > 1e-12 {
        let mag = (old * 0.1).abs().log10();
        (mag + (0.5 + rng.f64())).exp() * (0.5 - rng.f64()) * strength
    } else {
        (0.5 - rng.f64()) * strength
    };
    let v = old + delta;
    if integer {
        v.round()
    } else {
        v
    }
}

/// `TModifySingleParamMutation`
fn modify_params(s: &mut Scene, strength: f64, rng: &mut Rng) {
    let with_params: Vec<usize> = non_empty(s).into_iter().filter(|&i| !s.formulas[i].formula.options().is_empty()).collect();
    if with_params.is_empty() {
        return;
    }
    let i = with_params[rng.int(with_params.len())];
    let f = &mut s.formulas[i].formula;
    let n = f.options().len();
    let types = f.option_types();
    // one change, more for more options and higher strengths
    let mut changes = 1;
    for (k, th) in [(2, 0.25), (3, 0.5), (4, 0.75), (5, 1.0), (6, 1.25), (7, 1.5), (8, 1.75)] {
        if n > k && strength > th {
            changes += 1;
        }
    }
    for _ in 0..changes {
        let p = rng.int(n);
        let opts = f.options();
        let v = randomize_value(opts[p].1, types.get(p).is_some_and(|&t| is_int_type(t)), strength, rng);
        set_option_at(f, p, v);
    }
}

fn set_option_at(f: &mut Formula, p: usize, v: f64) {
    match f {
        Formula::Custom(c) => {
            if let Some(x) = c.values.get_mut(p) {
                *x = v;
            }
        }
        _ => {
            if let Some((name, _)) = f.options().into_iter().nth(p) {
                let _ = f.set_option(&name, v);
            }
        }
    }
}

fn has_category(s: &Scene, cats: &[Category]) -> bool {
    non_empty(s).iter().any(|&i| cats.contains(&category_of_name(&s.formulas[i].formula.name())))
}

/// `GuessFormulaCategory`
fn guess_category(i: usize, s: &Scene, rng: &mut Rng) -> Category {
    // MB3D picks with NextRandomInt(High(...)), so the last entry never comes
    let pick = |cats: &[Category], rng: &mut Rng| cats[rng.int(cats.len() - 1)];
    if has_category(s, &[Category::Difs, Category::DifsA]) {
        if i > 0 {
            pick(&[Category::Difs, Category::DifsA], rng)
        } else {
            Category::Difs
        }
    } else if has_category(s, &[Category::D4, Category::D4a]) {
        if i > 0 {
            pick(&[Category::D4, Category::D4a], rng)
        } else {
            Category::D4
        }
    } else if has_category(s, &[Category::D3, Category::D3a, Category::Ads]) && i > 0 {
        pick(&[Category::D3, Category::Ads, Category::D3a], rng)
    } else {
        Category::D3
    }
}

/// `TAddFormulaMutation`: a formula in the first empty slot.
fn add_formula(s: &mut Scene, rng: &mut Rng) {
    if let Some(i) = (0..6).find(|&i| is_empty(s, i)) {
        let names = names_of(guess_category(i, s, rng));
        if !names.is_empty() {
            for _ in 0..25 {
                let n = names[rng.int(names.len())];
                if !n.starts_with('_') {
                    set_formula(s, i, n);
                    break;
                }
            }
        }
    }
}

/// `TReplaceFormulaMutation`: the first formula by another of its family.
fn replace_formula(s: &mut Scene, rng: &mut Rng) {
    if let Some(&i) = non_empty(s).first() {
        let old = s.formulas[i].formula.name();
        let names = names_of(category_of_name(&old));
        if names.is_empty() {
            return;
        }
        for _ in 0..24 {
            let n = names[rng.int(names.len())];
            if !n.eq_ignore_ascii_case(&old) && n.starts_with('_') == old.starts_with('_') {
                set_formula(s, i, n);
                break;
            }
        }
    }
}

/// `TRemoveFormulaMutation`: only with more than 3 or 4 formulas; helper
/// formulas (`_name`) and the "add" families go first.
fn remove_formula(s: &mut Scene, rng: &mut Rng) {
    let n = non_empty(s).len();
    if !(n > 4 || (n > 3 && rng.f64() > 0.5)) {
        return;
    }
    use Category::*;
    for pass in 0..2 {
        for cat in [DifsA, D4a, D3a, Ads, Difs, D4, D3] {
            let mut idx: Vec<usize> = non_empty(s).into_iter().filter(|&i| category_of_name(&s.formulas[i].formula.name()) == cat).collect();
            if pass == 0 {
                idx.retain(|&i| s.formulas[i].formula.name().starts_with('_'));
            }
            if !idx.is_empty() {
                let i = idx[rng.int(idx.len())];
                clear_formula(s, i);
                return;
            }
        }
    }
}

/// `TModifyJuliaModeMutation`
fn modify_julia(s: &mut Scene, strength: f64, rng: &mut Rng) {
    const SCALE: f64 = 1.5;
    s.julia = rng.f64() > 0.25;
    if s.julia {
        let r = |rng: &mut Rng| strength * (0.5 - rng.f64()) * SCALE;
        for (k, th) in [(0, 0.5), (1, 0.5), (2, 0.5), (3, 0.75)] {
            if rng.f64() > th {
                s.julia_c[k] = r(rng);
            }
        }
        if s.julia_c[..3].iter().all(|v| v.abs() < 1e-12) {
            let k = if rng.f64() > 0.5 {
                if rng.f64() > 0.5 {
                    0
                } else {
                    2
                }
            } else if rng.f64() > 0.5 {
                1
            } else {
                3
            };
            s.julia_c[k] = r(rng);
        }
    } else {
        s.julia_c = [0.0; 4];
    }
}

/// `TModifyIterationCountMutation`
fn modify_iterations(s: &mut Scene, strength: f64, rng: &mut Rng) {
    let idx = non_empty(s);
    if idx.len() < 2 {
        return;
    }
    let passes = 1 + (strength > 0.5) as usize + (strength > 1.0) as usize + (strength > 1.5) as usize;
    for &i in &idx {
        s.formulas[i].iterations = 1;
    }
    for _ in 0..passes {
        let i = idx[rng.int(idx.len())];
        s.formulas[i].iterations = 1 + rng.int((2.0 + 2.0 * strength).round() as usize) as i32;
    }
}

/// `TMutaGenFrm.MutateParams` with `TMutationCreator.CreateMutations`.
pub fn mutate(cfg: &MutationConfig, s: &Scene, rng: &mut Rng) -> Scene {
    let mut r = s.clone();
    r.light_blend = None;
    if cfg.formula_weight > rng.f64() {
        add_formula(&mut r, rng);
        replace_formula(&mut r, rng);
        remove_formula(&mut r, rng);
    }
    if cfg.params_weight > rng.f64() {
        modify_params(&mut r, cfg.params_strength, rng);
    }
    if cfg.julia_weight > rng.f64() {
        modify_julia(&mut r, cfg.julia_strength, rng);
    }
    if cfg.iterations_weight > rng.f64() {
        modify_iterations(&mut r, cfg.iterations_strength, rng);
    }
    r
}

/// `TPreviewRenderer.RenderPreview`: an image of at most `w` x `h` pixels
/// (aspect kept, DE stop scaled), without volumetric light; `blank`
/// removes background picture, depth and fog colours (probing images).
pub fn preview(s: &Scene, w: usize, h: usize, blank: bool, threads: usize) -> Result<(Vec<u8>, usize, usize), String> {
    let mut p = s.clone();
    p.tiling = None;
    p.calc_rect = None;
    p.light_blend = None;
    p.vol_light = None;
    p.z_step_div = 0.5;
    p.raystep_limiter = 1.0;
    if threads > 0 {
        p.threads = threads;
    }
    if blank {
        let l = &mut p.lighting;
        l.bg_image.clear();
        l.depth_col = [0; 3];
        l.depth_col2 = [0; 3];
        l.dyn_fog_col = [0; 3];
        l.dyn_fog_col2 = [0; 3];
    }
    let d = (h as f64 / p.height.max(1) as f64).min(w as f64 / p.width.max(1) as f64);
    let (pw, ph) = (((p.width as f64 * d).round() as i32).max(2), ((p.height as f64 * d).round() as i32).max(2));
    let f = pw as f64 / p.width.max(1) as f64;
    p.width = pw;
    p.height = ph;
    p.de_stop = (p.de_stop * f).max(0.001);
    if let Some(dof) = p.dof.as_mut() {
        dof.clip_r = (dof.clip_r * f as f32).max(0.1);
    }
    // on the CPU: the mutations bring ever new formula combinations, and
    // compiling a shader for each takes seconds for a tiny image
    #[cfg(feature = "gpu")]
    let was = crate::gpu::cpu_in_thread(true);
    let r = crate::render::render(&p, &|_, _| {});
    #[cfg(feature = "gpu")]
    crate::gpu::cpu_in_thread(was);
    let r = r?;
    Ok((r.rgb, pw as usize, ph as usize))
}

/// `CalcFilteredCoverage`: share of pixels with a strong horizontal edge
/// (Sobel filter, luminance > 20).
pub fn filtered_coverage(rgb: &[u8], w: usize, h: usize) -> f64 {
    if w < 3 || h < 3 {
        return 0.0;
    }
    let px = |x: usize, y: usize, c: usize| rgb[(y * w + x) * 3 + c] as f64;
    let mut n = 0;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let f = |c| -px(x - 1, y - 1, c) + px(x + 1, y - 1, c) - 2.0 * px(x - 1, y, c) + 2.0 * px(x + 1, y, c) - px(x - 1, y + 1, c) + px(x + 1, y + 1, c);
            if 0.299 * f(0) + 0.587 * f(1) + 0.114 * f(2) > 20.0 {
                n += 1;
            }
        }
    }
    n as f64 / ((h - 2) * (w - 2)) as f64
}

/// `CalcDiffCoverage`: share of pixels that differ from the other image.
pub fn diff_coverage(a: &[u8], b: &[u8]) -> f64 {
    let t = [5.0 * 0.299, 5.0 * 0.587, 5.0 * 0.114];
    let n = a.len().min(b.len()) / 3;
    if n == 0 {
        return 1.0;
    }
    let mut c = 0;
    for i in 0..n {
        if (0..3).any(|k| (a[i * 3 + k] as f64 - b[i * 3 + k] as f64).abs() > t[k]) {
            c += 1;
        }
    }
    c as f64 / n as f64
}

/// A parameter set of a generation with its probing image.
#[derive(Clone, Debug)]
pub struct Member {
    pub scene: Scene,
    pub probe: Vec<u8>,
    pub coverage: f64,
}

/// The family tree: label and parent index (`CreatePanelList`), in the order
/// MB3D calculates them.
pub const TREE: [(&str, Option<usize>); 15] = [
    ("1", None),
    ("1.1", Some(0)),
    ("1.2", Some(0)),
    ("1.1.1", Some(1)),
    ("1.2.1", Some(2)),
    ("1.1.2", Some(1)),
    ("1.2.2", Some(2)),
    ("1.1.1.1", Some(3)),
    ("1.1.2.1", Some(5)),
    ("1.2.1.1", Some(4)),
    ("1.2.2.1", Some(6)),
    ("1.1.1.2", Some(3)),
    ("1.1.2.2", Some(5)),
    ("1.2.1.2", Some(4)),
    ("1.2.2.2", Some(6)),
];

/// `CreateMutation` for one child: probes up to `probe_max` mutations of the
/// parent and keeps the one with the most structure among those that
/// differ enough from the parent.
pub fn breed(cfg: &MutationConfig, parent: &Member, rng: &mut Rng, threads: usize) -> Result<Member, String> {
    if !cfg.probing {
        let s = mutate(cfg, &parent.scene, rng);
        return Ok(Member { scene: s, probe: Vec::new(), coverage: 0.0 });
    }
    let mut valid: Vec<Member> = Vec::new();
    let mut invalid: Vec<Member> = Vec::new();
    for _ in 0..cfg.probe_max.max(1) {
        let s = mutate(cfg, &parent.scene, rng);
        // a mutation that cannot be calculated counts as empty
        let (img, _, _) = preview(&s, cfg.probe_width, cfg.probe_height, true, threads)
            .unwrap_or_else(|_| (vec![0; parent.probe.len()], 0, 0));
        let (pw, ph) = probe_size(&s, cfg);
        let coverage = filtered_coverage(&img, pw, ph);
        let diff = diff_coverage(&img, &parent.probe);
        let m = Member { scene: s, probe: img, coverage };
        // AddProbedParam compares the difference with the coverage minimum
        if diff >= cfg.probe_min_coverage {
            valid.push(m);
        } else {
            invalid.push(m);
        }
        if valid.iter().any(|m| m.coverage >= cfg.probe_min_coverage) {
            break;
        }
    }
    let best = |v: Vec<Member>| v.into_iter().max_by(|a, b| a.coverage.partial_cmp(&b.coverage).unwrap_or(std::cmp::Ordering::Equal));
    best(valid).or_else(|| best(invalid)).ok_or_else(|| "no mutation".into())
}

fn probe_size(s: &Scene, cfg: &MutationConfig) -> (usize, usize) {
    let d = (cfg.probe_height as f64 / s.height.max(1) as f64).min(cfg.probe_width as f64 / s.width.max(1) as f64);
    (((s.width as f64 * d).round() as usize).max(2), ((s.height as f64 * d).round() as usize).max(2))
}

/// The parent of a generation (`CreateInitialSet`).
pub fn root(cfg: &MutationConfig, s: &Scene, threads: usize) -> Member {
    let probe = if cfg.probing {
        preview(s, cfg.probe_width, cfg.probe_height, true, threads).map(|r| r.0).unwrap_or_default()
    } else {
        Vec::new()
    };
    let (pw, ph) = probe_size(s, cfg);
    let coverage = filtered_coverage(&probe, pw, ph);
    Member { scene: s.clone(), probe, coverage }
}

/// A whole generation (15 members, `TREE` order); `progress(done, 15)`
/// after each member, `cancel` checked between them.
pub fn generation(
    cfg: &MutationConfig,
    parent: &Scene,
    rng: &mut Rng,
    threads: usize,
    progress: &mut dyn FnMut(usize, &Member),
    cancel: &dyn Fn() -> bool,
) -> Result<Vec<Member>, String> {
    let mut out: Vec<Member> = Vec::with_capacity(TREE.len());
    for (i, (_, p)) in TREE.iter().enumerate() {
        if cancel() {
            return Err("cancelled".into());
        }
        let m = match p {
            None => root(cfg, parent, threads),
            Some(p) => breed(cfg, &out[*p], rng, threads)?,
        };
        progress(i + 1, &m);
        out.push(m);
    }
    Ok(out)
}

/// Centres of the members in MB3D's window (`CreatePanelList`), in cells of
/// a 5 x 4 grid.
pub const LAYOUT: [(f64, f64); 15] = [
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

/// The generation as one picture in MB3D's layout, with lines from the
/// parents to their children.  `imgs`: RGB previews (all `w` x `h`).
pub fn contact_sheet(imgs: &[Vec<u8>], w: usize, h: usize) -> (Vec<u8>, usize, usize) {
    let (cw, ch) = ((w as f64 / 0.9).ceil() as usize, (h as f64 / 0.8).ceil() as usize);
    let (sw, sh) = (cw * 5, ch * 4);
    let mut out = vec![0x26u8; sw * sh * 3];
    let centre = |i: usize| (((LAYOUT[i].0 + 2.5) * cw as f64) as i64, ((LAYOUT[i].1 + 2.0) * ch as f64) as i64);
    let put = |out: &mut Vec<u8>, x: i64, y: i64, c: [u8; 3]| {
        if x >= 0 && y >= 0 && (x as usize) < sw && (y as usize) < sh {
            let o = (y as usize * sw + x as usize) * 3;
            out[o..o + 3].copy_from_slice(&c);
        }
    };
    for (i, (_, p)) in TREE.iter().enumerate() {
        if let Some(p) = p {
            let (a, b) = (centre(*p), centre(i));
            let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).max(1);
            for k in 0..=n {
                let x = a.0 + (b.0 - a.0) * k / n;
                let y = a.1 + (b.1 - a.1) * k / n;
                put(&mut out, x, y, [0xE0, 0xA2, 0x4A]);
            }
        }
    }
    for (i, img) in imgs.iter().enumerate().take(15) {
        let (cx, cy) = centre(i);
        let (x0, y0) = (cx - w as i64 / 2, cy - h as i64 / 2);
        for y in 0..h {
            for x in 0..w {
                let o = (y * w + x) * 3;
                if o + 3 <= img.len() {
                    put(&mut out, x0 + x as i64, y0 + y as i64, [img[o], img[o + 1], img[o + 2]]);
                }
            }
        }
    }
    (out, sw, sh)
}

/// A short description of a member's formulas (`CreateParamsCaption`).
pub fn caption(s: &Scene) -> String {
    let f: Vec<String> = non_empty(s).iter().map(|&i| format!("{} x{}", s.formulas[i].formula.name(), s.formulas[i].iterations)).collect();
    let mut c = f.join(" + ");
    if s.julia {
        c.push_str(&format!(" | julia {:.2} {:.2} {:.2}", s.julia_c[0], s.julia_c[1], s.julia_c[2]));
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_range() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let x = r.f64();
            assert!((0.0..1.0).contains(&x));
            assert!(r.int(5) < 5);
        }
    }

    #[test]
    fn mutations_change_something() {
        let s = Scene::preset("Amazing Box").unwrap();
        let cfg = MutationConfig::default();
        let mut rng = Rng::new(42);
        let mut changed = 0;
        for _ in 0..30 {
            let m = mutate(&cfg, &s, &mut rng);
            assert!(!non_empty(&m).is_empty());
            if caption(&m) != caption(&s) || m.formulas[0].formula.options() != s.formulas[0].formula.options() {
                changed += 1;
            }
        }
        assert!(changed > 15, "{changed}");
    }

    #[test]
    fn coverage_measures() {
        let w = 10;
        let flat = vec![100u8; w * w * 3];
        assert_eq!(filtered_coverage(&flat, w, w), 0.0);
        let mut stripes = flat.clone();
        for y in 0..w {
            for x in 0..w {
                // dark-to-bright edges (the filter is one-sided, like MB3D's)
                if x % 4 >= 2 {
                    stripes[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&[255, 255, 255]);
                }
            }
        }
        assert!(filtered_coverage(&stripes, w, w) > 0.2, "{}", filtered_coverage(&stripes, w, w));
        assert_eq!(diff_coverage(&flat, &flat), 0.0);
        assert!((diff_coverage(&flat, &stripes) - 0.5).abs() < 0.11);
    }

    #[test]
    fn a_generation() {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.width = 80;
        s.height = 60;
        let cfg = MutationConfig { probe_max: 3, ..Default::default() };
        let mut rng = Rng::new(3);
        let mut n = 0;
        let g = generation(&cfg, &s, &mut rng, 2, &mut |_, _| n += 1, &|| false).unwrap();
        assert_eq!(g.len(), 15);
        assert_eq!(n, 15);
        assert_eq!(caption(&g[0].scene), caption(&s));
        assert!(g.iter().all(|m| m.probe.len() == g[0].probe.len()));
    }
}
