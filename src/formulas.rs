//! The internal ("built-in") formulas of Mandelbulb3D, ported from
//! `formulas.pas` (the x87/SSE2 assembler was translated back to scalar code,
//! keeping the evaluation order) and their meta data from
//! `CustomFormulas.pas` / `HeaderTrafos.pas`.
//!
//! Each formula performs one iteration on the vector (x, y, z, w); `j` holds
//! the constant that is added (the pixel position C, or the Julia seed) with
//! `j[3]` being the 4D constant `J4`, and `rout` is the squared vector length
//! after the previous counted iteration (`TIteration3D.Rout`).

use crate::custom::CustomFormula;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Formula option types relevant for DE scaling / colouring heuristics
/// (`byOptionTypes[0]` in MB3D).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FirstOptionType {
    /// type 0 (Double) – may be a power or scale
    Double,
    /// type 10 (no variable, integer power)
    NoVar,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Formula {
    /// 'Integer Power' – the triplex sine bulb, power 2..8 (f = 0).
    IntPow { power: i32, z_mul: f64 },
    /// 'Real Power' – arbitrary power sine bulb (f = 1).
    RealPower { power: f64, z_mul: f64 },
    /// 'Quaternion' – 4D quaternion (f = 2).
    Quaternion { yw_mul: f64, w_add: f64 },
    /// 'Tricorn' (f = 3).
    Tricorn { z_mul: f64, cz_mul: f64 },
    /// 'Amazing Box' aka Mandbox (f = 4).
    AmazingBox { scale: f64, min_r: f64, fold: f64 },
    /// 'Bulbox' – sine bulb power 2 / amazing box hybrid (f = 5).
    Bulbox {
        scale: f64,
        min_r: f64,
        fold: f64,
        bulb_scale: f64,
        r_threshold: f64,
        r_threshold2: f64,
    },
    /// 'Folding Int Pow' (f = 6).
    FoldingIntPow { power: i32, z_mul: f64, fold: f64 },
    /// 'Aexion C' (f = 9): a real power bulb that also turns the constant C
    /// by a power of its angles ("iterating C").
    AexionC {
        power: f64,
        z_mul: f64,
        /// rotate C each iteration
        rot_c: bool,
        /// conditional phi: negate the C angle when the chosen component is >= 0
        cond_phi: bool,
        power_c: f64,
        cz_mul: f64,
        /// multiply power C by the distance of Z and C
        powc_dist: bool,
        /// bits: 1 flip theta atan, 2 flip phi atan, 4 swap theta/phi,
        /// 8 swap Cy/Cz, 16 use Z - C instead of C for the angles
        mode: i32,
    },
    /// A clean-room, angularly warped power bulb.  The three variants use
    /// distinct radial modulations; the names follow recent formula-family
    /// terminology, but the implementation belongs to mb3d-rs.
    SinCosBulb {
        variant: u8,
        power: f64,
        warp: f64,
        frequency: f64,
        z_mul: f64,
    },
    /// A power bulb with a radius-dependent "tail" modulation.
    TailsBulb { power: f64, tail: f64, frequency: f64, z_mul: f64 },
    /// A three-dimensional Mandelbar: a power bulb conjugated across the
    /// xz-plane before each power step.
    MandelbarBulb { power: f64, z_mul: f64 },
    /// Independently implemented from the published Lambdabulb definition.
    Lambdabulb { power: f64, phase: f64, c_scale: f64 },
    /// A four-dimensional Hopf-coordinate power bulb.
    Hopfbrot { power: f64 },
    /// A clean-room 4D box fold projected into the tetrahedral hyperplane.
    TetrahedralBox { scale: f64, min_r: f64, fold: f64 },
    /// A custom formula from a `.m3f` file, run by the x86 interpreter.
    Custom(Box<CustomFormula>),
}

fn formula_dirs() -> &'static Mutex<Vec<PathBuf>> {
    static D: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    D.get_or_init(|| {
        let mut v = Vec::new();
        if let Ok(e) = std::env::var("MB3D_FORMULAS") {
            v.extend(std::env::split_paths(&e));
        }
        v.push(PathBuf::from("M3Formulas"));
        // the user's own (newer versions replace the shipped ones), the
        // program's, a system-wide installation's
        v.extend(crate::appdirs::user_data_dir().map(|d| d.join("M3Formulas")).filter(|d| d.is_dir()));
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() {
                v.push(d.join("M3Formulas"));
            }
        }
        v.extend(crate::appdirs::system_data_dirs().into_iter().map(|d| d.join("M3Formulas")).filter(|d| d.is_dir()));
        Mutex::new(v)
    })
}

/// Folders found from the parameter files that were loaded (searched after
/// the regular ones).
fn scene_formula_dirs() -> &'static Mutex<Vec<PathBuf>> {
    static D: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    D.get_or_init(|| Mutex::new(Vec::new()))
}

/// The folder `download.sh` installs the release into (`~/mb3d-rs`), last
/// in the search: a program built from source finds the formulas there.
fn installed_formula_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join("mb3d-rs").join("M3Formulas"))
}

/// Remembers the `M3Formulas` folder that belongs to a parameter file: in
/// its own folder or the one above (MB3D keeps `M3Parameter` next to
/// `M3Formulas`). Such folders are searched after the regular ones.
pub fn note_scene_file(path: &Path) {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let Some(dir) = path.parent() else { return };
    let mut found = scene_formula_dirs().lock().unwrap();
    for c in [Some(dir.join("M3Formulas")), dir.parent().map(|p| p.join("M3Formulas"))].into_iter().flatten() {
        if c.is_dir() && !found.contains(&c) {
            found.push(c);
        }
    }
}

/// The formula search directories.
pub fn formula_dir_list() -> Vec<PathBuf> {
    let mut v = formula_dirs().lock().unwrap().clone();
    v.extend(scene_formula_dirs().lock().unwrap().iter().cloned());
    if let Some(d) = installed_formula_dir() {
        if d.is_dir() && !v.contains(&d) {
            v.push(d);
        }
    }
    v
}

/// Names of all `.m3f` formulas in the formula directories (sorted, unique).
pub fn list_custom() -> Vec<String> {
    let dirs = formula_dir_list();
    let mut v: Vec<String> = dirs
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .filter_map(|e| {
            let p = e.ok()?.path();
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("m3f")) {
                return None;
            }
            Some(p.file_stem()?.to_string_lossy().into_owned())
        })
        .collect();
    v.sort_by_key(|a| a.to_ascii_lowercase());
    v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    v
}

/// The custom formulas with their `.DEoption` (MB3D sorts its formula
/// lists by it: 2/11 3Da, 4 4D, 5/6 4Da, -1/-2 Ads, 20 dIFS, 21/22 dIFS
/// transformations, others 3D).
pub fn list_custom_with_de() -> Vec<(String, i32)> {
    let dirs = formula_dir_list();
    list_custom()
        .into_iter()
        .map(|name| {
            let de = dirs
                .iter()
                .find_map(|d| std::fs::read(d.join(format!("{name}.m3f"))).ok())
                .and_then(|b| {
                    let t = String::from_utf8_lossy(&b).to_ascii_lowercase();
                    let i = t.find(".deoption")?;
                    let rest = t[i + 9..].trim_start().strip_prefix('=')?;
                    rest.trim_start().split(|c: char| !(c == '-' || c.is_ascii_digit())).next()?.parse::<i32>().ok()
                })
                .unwrap_or(0);
            (name, de)
        })
        .collect()
}

/// Adds a directory that is searched for `.m3f` files (searched first).
pub fn add_formula_dir(dir: PathBuf) {
    formula_dirs().lock().unwrap().insert(0, dir);
}

/// Loads `<name>.m3f` from the formula directories (cached).
fn custom_cache() -> &'static Mutex<Vec<Arc<crate::m3f::M3f>>> {
    static CACHE: OnceLock<Mutex<Vec<Arc<crate::m3f::M3f>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Drops a formula from the cache (after it was edited and saved).
pub fn forget_custom(name: &str) {
    custom_cache().lock().unwrap().retain(|f| !f.name.eq_ignore_ascii_case(name.trim()));
}

/// The file of a custom formula, if found in the formula directories.
pub fn custom_path(name: &str) -> Option<PathBuf> {
    crate::m3f::find_formula(name, &formula_dir_list())
}

pub fn load_custom(name: &str) -> Result<Arc<crate::m3f::M3f>, String> {
    let cache = custom_cache();
    if let Some(f) = cache.lock().unwrap().iter().find(|f| f.name.eq_ignore_ascii_case(name.trim())) {
        return Ok(f.clone());
    }
    let dirs = formula_dir_list();
    let path = crate::m3f::find_formula(name, &dirs).ok_or_else(|| {
        format!(
            "formula '{name}' not found (searched {}); use --formulas DIR or MB3D_FORMULAS",
            dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
        )
    })?;
    let f = Arc::new(crate::m3f::M3f::load(&path)?);
    cache.lock().unwrap().push(f.clone());
    Ok(f)
}

/// Built-in formula by name, or a custom `.m3f` formula with its defaults.
pub fn lookup(name: &str) -> Result<Formula, String> {
    if let Some(f) = Formula::default_for(name) {
        return Ok(f);
    }
    let def = load_custom(name)?;
    Ok(Formula::Custom(Box::new(CustomFormula::new(def))))
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Formula {
    /// Names as used by MB3D (`InternFormulaNames`).
    pub fn name(&self) -> String {
        match self {
            Formula::Custom(c) => c.name().to_string(),
            _ => self.builtin_name().to_string(),
        }
    }

    fn builtin_name(&self) -> &'static str {
        match self {
            Formula::Custom(_) => "custom",
            Formula::IntPow { .. } => "Integer Power",
            Formula::RealPower { .. } => "Real Power",
            Formula::Quaternion { .. } => "Quaternion",
            Formula::Tricorn { .. } => "Tricorn",
            Formula::AmazingBox { .. } => "Amazing Box",
            Formula::Bulbox { .. } => "Bulbox",
            Formula::FoldingIntPow { .. } => "Folding Int Pow",
            Formula::AexionC { .. } => "Aexion C",
            Formula::SinCosBulb { variant: 1, .. } => "SinCos Bulb",
            Formula::SinCosBulb { variant: 2, .. } => "SinCos Bulb V2",
            Formula::SinCosBulb { .. } => "SinCos Bulb V3",
            Formula::TailsBulb { .. } => "Tails Bulb",
            Formula::MandelbarBulb { .. } => "Mandelbar Bulb",
            Formula::Lambdabulb { .. } => "Lambdabulb",
            Formula::Hopfbrot { .. } => "Hopfbrot",
            Formula::TetrahedralBox { .. } => "Tetrahedral Box",
        }
    }

    /// All built-in formula names (for the CLI help).
    pub fn all_names() -> &'static [&'static str] {
        &[
            "Integer Power",
            "Real Power",
            "Quaternion",
            "Tricorn",
            "Amazing Box",
            "Bulbox",
            "Folding Int Pow",
            "Aexion C",
            "SinCos Bulb",
            "SinCos Bulb V2",
            "SinCos Bulb V3",
            "Tails Bulb",
            "Mandelbar Bulb",
            "Lambdabulb",
            "Hopfbrot",
            "Tetrahedral Box",
            "Lambdabulb",
            "Hopfbrot",
            "Tetrahedral Box",
        ]
    }

    /// Default options, from `GetHAddOnFromInternFormula`.
    pub fn default_for(name: &str) -> Option<Formula> {
        let n = name.trim().to_ascii_lowercase().replace(['_', '-'], " ");
        Some(match n.as_str() {
            "integer power" | "intpow" | "mandelbulb" | "bulb" => {
                Formula::IntPow { power: 8, z_mul: -1.0 }
            }
            "real power" | "realpower" | "floatpow" => {
                Formula::RealPower { power: 8.0, z_mul: -1.0 }
            }
            "quaternion" | "quat" => Formula::Quaternion { yw_mul: -1.0, w_add: 0.0 },
            "tricorn" => Formula::Tricorn { z_mul: -2.0, cz_mul: 1.0 },
            "amazing box" | "amazingbox" | "mandelbox" | "box" => {
                Formula::AmazingBox { scale: 2.0, min_r: 0.5, fold: 1.0 }
            }
            "bulbox" => Formula::Bulbox {
                scale: 2.0,
                min_r: 0.5,
                fold: 1.0,
                bulb_scale: 1.0,
                r_threshold: 2.0,
                r_threshold2: 2.0,
            },
            "folding int pow" | "foldingintpow" | "foldintpow" => {
                Formula::FoldingIntPow { power: 2, z_mul: -1.0, fold: 2.0 }
            }
            // GetHAddOnFromInternFormula: oa[9] = (8, 1), options 2..5 = 1, 1, 8, 1
            "aexion c" | "aexionc" | "aexion" => Formula::AexionC {
                power: 8.0,
                z_mul: 1.0,
                rot_c: true,
                cond_phi: true,
                power_c: 8.0,
                cz_mul: 1.0,
                powc_dist: false,
                mode: 0,
            },
            "sincos bulb" | "sincos" => Formula::SinCosBulb {
                variant: 1,
                power: 8.0,
                warp: 0.22,
                frequency: 3.0,
                z_mul: 1.0,
            },
            "sincos bulb v2" | "sincos v2" => Formula::SinCosBulb {
                variant: 2,
                power: 8.0,
                warp: 0.22,
                frequency: 3.0,
                z_mul: 1.0,
            },
            "sincos bulb v3" | "sincos v3" => Formula::SinCosBulb {
                variant: 3,
                power: 8.0,
                warp: 0.22,
                frequency: 3.0,
                z_mul: 1.0,
            },
            "tails bulb" | "tails" => Formula::TailsBulb {
                power: 8.0,
                tail: 0.35,
                frequency: 2.5,
                z_mul: 1.0,
            },
            "mandelbar bulb" | "mandelbar" => Formula::MandelbarBulb { power: 8.0, z_mul: 1.0 },
            "lambdabulb" | "lambda bulb" => Formula::Lambdabulb { power: 4.0, phase: 1.815_142, c_scale: 1.0 },
            "hopfbrot" => Formula::Hopfbrot { power: 2.0 },
            "tetrahedral box" | "tetra box" => Formula::TetrahedralBox { scale: 2.0, min_r: 0.5, fold: 1.0 },
            _ => return None,
        })
    }

    /// Option names as shown in MB3D's formula window (`SetCFoptionsFromOldF`),
    /// paired with the current values.
    pub fn options(&self) -> Vec<(String, f64)> {
        if let Formula::Custom(c) = self {
            return c.def.options.iter().zip(&c.values).map(|(o, v)| (o.name.clone(), *v)).collect();
        }
        let v: Vec<(&'static str, f64)> = match *self {
            Formula::Custom(_) => unreachable!(),
            Formula::IntPow { power, z_mul } => {
                vec![("Integer power", power as f64), ("Z multiplier", z_mul)]
            }
            Formula::RealPower { power, z_mul } => {
                vec![("Float power", power), ("Z multiplier", z_mul)]
            }
            Formula::Quaternion { yw_mul, w_add } => {
                vec![("YW multiplier", yw_mul), ("W add", w_add)]
            }
            Formula::Tricorn { z_mul, cz_mul } => {
                vec![("Z multiplier", z_mul), ("CZ multiplier", cz_mul)]
            }
            Formula::AmazingBox { scale, min_r, fold } => {
                vec![("Scale", scale), ("Min R", min_r), ("Fold", fold)]
            }
            Formula::Bulbox { scale, min_r, fold, bulb_scale, r_threshold, r_threshold2 } => vec![
                ("Box Scale", scale),
                ("Box Min R", min_r),
                ("Box fold", fold),
                ("Bulb scaling", bulb_scale),
                ("Box/Bulb R threshold", r_threshold),
                ("Box/Bulb R threshold 2", r_threshold2),
            ],
            Formula::FoldingIntPow { power, z_mul, fold } => vec![
                ("Integer power", power as f64),
                ("Z multiplier", z_mul),
                ("R fold", fold),
            ],
            Formula::AexionC { power, z_mul, rot_c, cond_phi, power_c, cz_mul, powc_dist, mode } => vec![
                ("Float power", power),
                ("Z multiplier", z_mul),
                ("Enable rotate C (0,1)", rot_c as i32 as f64),
                ("Condi. Phi (0,1)", cond_phi as i32 as f64),
                ("Float power C", power_c),
                ("Cz multiplier", cz_mul),
                ("PowC on dist Vec-C (0,1)", powc_dist as i32 as f64),
                ("Mode (0..31)", mode as f64),
            ],
            Formula::SinCosBulb { power, warp, frequency, z_mul, .. } => vec![
                ("Power", power),
                ("Angular warp", warp),
                ("Frequency", frequency),
                ("Z multiplier", z_mul),
            ],
            Formula::TailsBulb { power, tail, frequency, z_mul } => vec![
                ("Power", power),
                ("Tail strength", tail),
                ("Tail frequency", frequency),
                ("Z multiplier", z_mul),
            ],
            Formula::MandelbarBulb { power, z_mul } => {
                vec![("Power", power), ("Z multiplier", z_mul)]
            }
            Formula::Lambdabulb { power, phase, c_scale } => vec![
                ("Power", power), ("Phase (radians)", phase), ("C multiplier", c_scale),
            ],
            Formula::Hopfbrot { power } => vec![("Power", power)],
            Formula::TetrahedralBox { scale, min_r, fold } => vec![
                ("Scale", scale), ("Min radius", min_r), ("Fold", fold),
            ],
        };
        v.into_iter().map(|(k, x)| (k.to_string(), x)).collect()
    }

    /// Set an option by (case insensitive, loosely matched) name.
    pub fn set_option(&mut self, key: &str, value: f64) -> Result<(), String> {
        let k = key.trim().to_ascii_lowercase().replace(['_', '-'], " ");
        if let Formula::Custom(c) = self {
            let norm = |s: &str| s.trim().to_ascii_lowercase().replace(['_', '-'], " ");
            let idx = c.def.options.iter().position(|o| norm(&o.name) == k).or_else(|| {
                k.strip_prefix("option").and_then(|n| n.trim().parse::<usize>().ok())
            });
            return match idx {
                Some(i) if i < c.values.len() => {
                    c.values[i] = value;
                    Ok(())
                }
                _ => Err(format!(
                    "unknown option '{key}' for {} (options: {})",
                    c.name(),
                    c.def.options.iter().map(|o| o.name.clone()).collect::<Vec<_>>().join(", ")
                )),
            };
        }
        if let Some(i) = k.strip_prefix("option").and_then(|n| n.trim().parse::<usize>().ok()) {
            let Some(name) = self.options().get(i).map(|(name, _)| name.clone()) else {
                return Err(format!("unknown option '{key}' for {}", self.name()));
            };
            return self.set_option(&name, value);
        }
        let ok = match self {
            Formula::IntPow { power, z_mul } | Formula::FoldingIntPow { power, z_mul, .. }
                if matches!(k.as_str(), "power" | "integer power" | "z mul" | "z multiplier" | "zmul") =>
            {
                if k.contains('z') {
                    *z_mul = value;
                } else {
                    *power = (value.round() as i32).clamp(2, 8);
                }
                true
            }
            Formula::FoldingIntPow { fold, .. } if matches!(k.as_str(), "fold" | "r fold") => {
                *fold = value;
                true
            }
            Formula::AexionC { power, z_mul, rot_c, cond_phi, power_c, cz_mul, powc_dist, mode } => {
                // integer options (type 2) are rounded into the variable buffer
                let on = value.round() != 0.0;
                match k.as_str() {
                    "power" | "float power" => *power = value,
                    "z mul" | "z multiplier" | "zmul" => *z_mul = value,
                    "rotate c" | "enable rotate c" | "enable rotate c (0,1)" | "rot c" => *rot_c = on,
                    "cond phi" | "condi. phi" | "condi. phi (0,1)" | "conditional phi" => *cond_phi = on,
                    "power c" | "float power c" => *power_c = value,
                    "cz mul" | "cz multiplier" | "czmul" => *cz_mul = value,
                    "powc on dist" | "powc on dist vec c (0,1)" | "powc on dist vec c" => *powc_dist = on,
                    "mode" | "mode (0..31)" => *mode = (value.round() as i32).clamp(0, 31),
                    _ => return Err(format!("unknown option '{key}' for Aexion C")),
                }
                true
            }
            Formula::SinCosBulb { power, warp, frequency, z_mul, .. } => {
                match k.as_str() {
                    "power" => *power = value,
                    "angular warp" | "warp" => *warp = value,
                    "frequency" => *frequency = value,
                    "z mul" | "z multiplier" | "zmul" => *z_mul = value,
                    _ => return Err(format!("unknown option '{key}' for {}", self.name())),
                }
                true
            }
            Formula::TailsBulb { power, tail, frequency, z_mul } => {
                match k.as_str() {
                    "power" => *power = value,
                    "tail strength" | "tail" => *tail = value,
                    "tail frequency" | "frequency" => *frequency = value,
                    "z mul" | "z multiplier" | "zmul" => *z_mul = value,
                    _ => return Err(format!("unknown option '{key}' for Tails Bulb")),
                }
                true
            }
            Formula::MandelbarBulb { power, z_mul } => {
                match k.as_str() {
                    "power" => *power = value,
                    "z mul" | "z multiplier" | "zmul" => *z_mul = value,
                    _ => return Err(format!("unknown option '{key}' for Mandelbar Bulb")),
                }
                true
            }
            Formula::Lambdabulb { power, phase, c_scale } => {
                match k.as_str() {
                    "power" => *power = value,
                    "phase" | "phase radians" | "phase (radians)" => *phase = value,
                    "c multiplier" | "c scale" => *c_scale = value,
                    _ => return Err(format!("unknown option '{key}' for Lambdabulb")),
                }
                true
            }
            Formula::Hopfbrot { power } => {
                if k != "power" { return Err(format!("unknown option '{key}' for Hopfbrot")); }
                *power = value;
                true
            }
            Formula::TetrahedralBox { scale, min_r, fold } => {
                match k.as_str() {
                    "scale" => *scale = value,
                    "min radius" | "min r" => *min_r = value,
                    "fold" => *fold = value,
                    _ => return Err(format!("unknown option '{key}' for Tetrahedral Box")),
                }
                true
            }
            Formula::RealPower { power, z_mul } => match k.as_str() {
                "power" | "float power" => {
                    *power = value;
                    true
                }
                "z mul" | "z multiplier" | "zmul" => {
                    *z_mul = value;
                    true
                }
                _ => false,
            },
            Formula::Quaternion { yw_mul, w_add } => match k.as_str() {
                "yw mul" | "yw multiplier" => {
                    *yw_mul = value;
                    true
                }
                "w add" => {
                    *w_add = value;
                    true
                }
                _ => false,
            },
            Formula::Tricorn { z_mul, cz_mul } => match k.as_str() {
                "z mul" | "z multiplier" | "zmul" => {
                    *z_mul = value;
                    true
                }
                "cz mul" | "cz multiplier" | "czmul" => {
                    *cz_mul = value;
                    true
                }
                _ => false,
            },
            Formula::AmazingBox { scale, min_r, fold } => match k.as_str() {
                "scale" => {
                    *scale = value;
                    true
                }
                "min r" | "minr" => {
                    *min_r = value;
                    true
                }
                "fold" => {
                    *fold = value;
                    true
                }
                _ => false,
            },
            Formula::Bulbox { scale, min_r, fold, bulb_scale, r_threshold, r_threshold2 } => {
                match k.as_str() {
                    "scale" | "box scale" => *scale = value,
                    "min r" | "minr" | "box min r" => *min_r = value,
                    "fold" | "box fold" => *fold = value,
                    "bulb scale" | "bulb scaling" => *bulb_scale = value,
                    "r threshold" | "box/bulb r threshold" => *r_threshold = value,
                    "r threshold2" | "r threshold 2" | "box/bulb r threshold 2" => {
                        *r_threshold2 = value
                    }
                    _ => return Err(format!("unknown option '{key}' for {}", self.name())),
                }
                true
            }
            _ => false,
        };
        if ok {
            Ok(())
        } else {
            Err(format!("unknown option '{key}' for {}", self.name()))
        }
    }

    /// MB3D's option types (`byOptionTypes`): 0 double, 2 integer, 10 no
    /// variable (the integer power of the bulbs), ... (see `m3f`).
    pub fn option_types(&self) -> Vec<u8> {
        match self {
            Formula::Custom(c) => c.def.options.iter().map(|o| o.ty).collect(),
            Formula::IntPow { .. } => vec![10, 0],
            Formula::FoldingIntPow { .. } => vec![10, 0, 0],
            Formula::AexionC { .. } => vec![0, 0, 2, 2, 0, 0, 2, 2],
            _ => vec![0; self.options().len()],
        }
    }

    /// `iDEoption` (`ParseCFfromOld`): 0 = 3D numerical gradient DE,
    /// 4 = 4D numerical DE, 11 = analytic DE (amazing box, derivative in w).
    pub fn de_option(&self) -> i32 {
        match self {
            Formula::Custom(c) => c.def.de_option,
            Formula::Quaternion { .. } | Formula::Hopfbrot { .. } | Formula::TetrahedralBox { .. } => 4,
            Formula::AmazingBox { .. } => 11,
            _ => 0,
        }
    }

    /// `dDEscale` of the formula (`ds` table in `ParseCFfromOld`).
    pub fn de_scale(&self) -> f64 {
        match self {
            Formula::Custom(c) => c.def.de_scale,
            Formula::AmazingBox { .. } | Formula::Bulbox { .. } => 0.2,
            Formula::FoldingIntPow { .. } => 0.5,
            _ => 1.0,
        }
    }

    /// `dADEscale` (analytic DE scale), 1 for all internal formulas.
    pub fn ade_scale(&self) -> f64 {
        match self {
            Formula::Custom(c) => c.def.ade_scale,
            _ => 1.0,
        }
    }

    /// `dRstop` – the default bailout radius of the formula.
    pub fn default_rstop(&self) -> f64 {
        match self {
            Formula::Custom(c) => c.def.rstop,
            Formula::AmazingBox { .. } | Formula::Bulbox { .. } | Formula::FoldingIntPow { .. } => {
                1024.0
            }
            _ => 16.0,
        }
    }

    /// `dSIpow` – used for the smooth iteration count.
    pub fn si_pow(&self) -> f64 {
        match *self {
            Formula::Custom(ref c) => c.def.si_pow,
            Formula::IntPow { power, .. } => power.clamp(2, 8) as f64,
            Formula::RealPower { power, .. } | Formula::AexionC { power, .. } => {
                if power == 0.0 {
                    1e-40
                } else {
                    power
                }
            }
            Formula::SinCosBulb { power, .. }
            | Formula::TailsBulb { power, .. }
            | Formula::MandelbarBulb { power, .. }
            | Formula::Lambdabulb { power, .. }
            | Formula::Hopfbrot { power } => power,
            _ => 2.0,
        }
    }

    /// Value of the first option (what MB3D reads at `PVar - 16`).
    pub fn first_option(&self) -> f64 {
        match self {
            Formula::Custom(c) => {
                let b = c.def.var_buffer(&c.values, &crate::custom::int_pow_addr);
                let o = crate::m3f::CONST_OFFSET - 16;
                f64::from_le_bytes(b[o..o + 8].try_into().unwrap())
            }
            // option 0 (the integer power) uses no memory
            Formula::IntPow { z_mul, .. } | Formula::FoldingIntPow { z_mul, .. } => *z_mul,
            _ => self.options()[0].1,
        }
    }

    pub fn first_option_type(&self) -> FirstOptionType {
        match self {
            Formula::Custom(c) => match c.def.options.first().map(|o| o.ty) {
                Some(0) | Some(14) => FirstOptionType::Double,
                _ => FirstOptionType::NoVar,
            },
            Formula::IntPow { .. } | Formula::FoldingIntPow { .. } => FirstOptionType::NoVar,
            _ => FirstOptionType::Double,
        }
    }

    /// One iteration.  `ade` selects the analytic-DE variant of the amazing
    /// box (`HybridCubeDE`, which also scales the running derivative in `w`);
    /// MB3D switches to the plain `HybridCube` when the hybrid as a whole is
    /// not calculated with analytic DE.
    #[inline]
    pub fn iterate(&self, v: &mut [f64; 4], j: &mut [f64; 4], rout: f64, ade: bool) {
        match *self {
            Formula::IntPow { power, z_mul } => int_pow(power, z_mul, v, j),
            Formula::RealPower { power, z_mul } => real_power(power, z_mul, v, j, rout),
            Formula::Quaternion { yw_mul, w_add } => {
                let (x, y, z, w) = (v[0], v[1], v[2], v[3]);
                v[0] = x * x - y * y - z * z - w * w + j[0];
                v[1] = 2.0 * (y * x + z * w) + j[1];
                v[2] = 2.0 * (z * x + yw_mul * y * w) + j[2];
                v[3] = 2.0 * (w * x + y * z) + w_add + j[3];
            }
            Formula::Tricorn { z_mul, cz_mul } => {
                let (x, y, z) = (v[0], v[1], v[2]);
                v[2] = z * x * z_mul + j[2] * cz_mul;
                v[0] = x * x - (y * y + z * z) + j[0];
                v[1] = 2.0 * (y * x) + j[1];
            }
            Formula::AmazingBox { scale, min_r, fold } => {
                amazing_box(scale, min_r, fold, v, j, ade);
            }
            Formula::Bulbox { scale, min_r, fold, bulb_scale, r_threshold, r_threshold2 } => {
                // HybridSuperCube2
                let rt1 = r_threshold * r_threshold;
                let rt2 = r_threshold2 * r_threshold2;
                if rout < rt1 {
                    if rout < rt2 {
                        int_pow2_scale(bulb_scale, v, j);
                    } else {
                        let m = (rout - rt2) / (rt1 - rt2);
                        let xyz_in = [v[0], v[1], v[2]];
                        int_pow2_scale(bulb_scale, v, j);
                        let out = [v[0], v[1], v[2]];
                        v[0] = xyz_in[0];
                        v[1] = xyz_in[1];
                        v[2] = xyz_in[2];
                        amazing_box(scale, min_r, fold, v, j, false);
                        let r1 = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                        let m2 = 1.0 - m;
                        let r2 = r1 * m
                            + m2 * (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
                        v[0] = v[0] * m + m2 * out[0];
                        v[1] = v[1] * m + m2 * out[1];
                        v[2] = v[2] * m + m2 * out[2];
                        let r1 = r2 / (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + 1e-40).sqrt();
                        v[0] *= r1;
                        v[1] *= r1;
                        v[2] *= r1;
                    }
                } else {
                    amazing_box(scale, min_r, fold, v, j, false);
                }
            }
            Formula::Custom(_) => {} // run by Iteration::run_custom
            Formula::FoldingIntPow { power, z_mul, fold } => {
                for c in v.iter_mut().take(3) {
                    *c = box_fold(*c, fold);
                }
                int_pow(power, z_mul, v, j);
            }
            Formula::AexionC { .. } => aexion_c(self, v, j),
            Formula::SinCosBulb { variant, power, warp, frequency, z_mul } => {
                sincos_bulb(variant, power, warp, frequency, z_mul, v, j, rout)
            }
            Formula::TailsBulb { power, tail, frequency, z_mul } => {
                tails_bulb(power, tail, frequency, z_mul, v, j, rout)
            }
            Formula::MandelbarBulb { power, z_mul } => mandelbar_bulb(power, z_mul, v, j, rout),
            Formula::Lambdabulb { power, phase, c_scale } => lambdabulb(power, phase, c_scale, v, j),
            Formula::Hopfbrot { power } => hopfbrot(power, v, j),
            Formula::TetrahedralBox { scale, min_r, fold } => tetrahedral_box(scale, min_r, fold, v, j),
        }
    }
}

/// Lambdabulb, from the mathematical iteration published by mathr.co.uk.
/// This is an independent implementation, not a translation of source code.
fn lambdabulb(power: f64, phase: f64, c_scale: f64, v: &mut [f64; 4], j: &[f64; 4]) {
    let r = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if r <= 1e-300 { *v = [0.0; 4]; return; }
    let theta = v[1].atan2(v[0]) * power;
    let phi = (v[2] / r).clamp(-1.0, 1.0).acos() * power + phase;
    let rp = r.powf(power);
    let p = [rp * phi.sin() * theta.cos(), rp * phi.sin() * theta.sin(), rp * phi.cos()];
    let a = [v[0] - p[0], v[1] - p[1], v[2] - p[2]];
    let c = [j[0] * c_scale, j[1] * c_scale, j[2] * c_scale];
    let (rc, ra) = ((c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt(), (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt());
    if rc <= 1e-300 || ra <= 1e-300 { *v = [0.0; 4]; return; }
    let q = 1.0 - c[2] * a[2] / (rc * ra);
    v[0] = q * (c[0] * a[0] - c[1] * a[1]);
    v[1] = q * (a[0] * c[1] + c[0] * a[1]);
    v[2] = ra * c[2] + rc * a[2];
}

/// Hopfbrot in four-dimensional Hopf coordinates (clean-room from the
/// published definition); numerical 4D DE is used by the renderer.
fn hopfbrot(power: f64, v: &mut [f64; 4], j: &[f64; 4]) {
    let r = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if r <= 1e-300 { *v = *j; return; }
    let t = (v[2] * v[2] + v[3] * v[3]).sqrt().atan2((v[0] * v[0] + v[1] * v[1]).sqrt()) * power;
    let u = v[1].atan2(v[0]) * power;
    let q = v[3].atan2(v[2]) * power;
    let rp = r.powf(power);
    *v = [rp * t.cos() * u.cos() + j[0], rp * t.cos() * u.sin() + j[1], rp * t.sin() * q.cos() + j[2], rp * t.sin() * q.sin() + j[3]];
}

/// A 4D box fold projected into the sum-zero tetrahedral hyperplane. This is
/// a clean-room formula inspired by Jeannot's linked-coordinate experiments.
fn tetrahedral_box(scale: f64, min_r: f64, fold: f64, v: &mut [f64; 4], j: &[f64; 4]) {
    let mut p = *v;
    let avg = p.iter().sum::<f64>() * 0.25;
    for x in &mut p { *x = box_fold(*x - avg, fold); }
    let r2 = p.iter().map(|x| x * x).sum::<f64>();
    let k = if r2 < min_r * min_r { scale / (min_r * min_r) } else if r2 < 1.0 { scale / r2 } else { scale };
    *v = [p[0] * k + j[0], p[1] * k + j[1], p[2] * k + j[2], p[3] * k + j[3]];
}

/// Clean-room angular power bulb variants. Numerical DE is used by the
/// renderer, so these intentionally do not claim an analytic derivative.
fn sincos_bulb(variant: u8, power: f64, warp: f64, frequency: f64, z_mul: f64, v: &mut [f64; 4], j: &[f64; 4], rout: f64) {
    let (x, y, z) = (v[0], v[1], v[2]);
    let base_theta = y.atan2(x);
    let base_phi = z.atan2((x * x + y * y).sqrt());
    let theta = power * base_theta + warp * (frequency * base_phi).sin();
    let phi = power * base_phi + warp * (frequency * base_theta).cos();
    let mut radius = rout.max(1e-300).powf(power * 0.5);
    match variant {
        2 => radius *= 1.0 + warp * (frequency * rout.sqrt()).sin(),
        3 => radius *= 1.0 + warp * (frequency * base_theta).cos() * (frequency * base_phi).sin(),
        _ => {}
    }
    v[0] = theta.cos() * phi.cos() * radius + j[0];
    v[1] = theta.sin() * phi.cos() * radius + j[1];
    v[2] = phi.sin() * radius * z_mul + j[2];
}

fn tails_bulb(power: f64, tail: f64, frequency: f64, z_mul: f64, v: &mut [f64; 4], j: &[f64; 4], rout: f64) {
    let (x, y, z) = (v[0], v[1], v[2]);
    let theta = power * y.atan2(x);
    let phi = power * z.atan2((x * x + y * y).sqrt());
    let r = rout.sqrt();
    let radius = rout.max(1e-300).powf(power * 0.5) * (1.0 + tail * (frequency * r).sin() / (1.0 + r));
    v[0] = theta.cos() * phi.cos() * radius + j[0];
    v[1] = theta.sin() * phi.cos() * radius + j[1];
    v[2] = phi.sin() * radius * z_mul + j[2];
}

fn mandelbar_bulb(power: f64, z_mul: f64, v: &mut [f64; 4], j: &[f64; 4], rout: f64) {
    let (x, y, z) = (v[0], v[1], v[2]);
    let theta = -power * y.atan2(x);
    let phi = power * z.atan2((x * x + y * y).sqrt());
    let radius = rout.max(1e-300).powf(power * 0.5);
    v[0] = theta.cos() * phi.cos() * radius + j[0];
    v[1] = theta.sin() * phi.cos() * radius + j[1];
    v[2] = phi.sin() * radius * z_mul + j[2];
}

/// `AexionC` (formulas.pas, x87 code): a real power bulb step (angles as
/// atan2, theta from the y axis), then, with "rotate C", the constant is
/// replaced by a power of its own angles, so C changes from iteration to
/// iteration.
fn aexion_c(f: &Formula, v: &mut [f64; 4], j: &mut [f64; 4]) {
    let Formula::AexionC { power, z_mul, rot_c, cond_phi, power_c, cz_mul, powc_dist, mode } = *f else { return };
    let (x, y, z) = (v[0], v[1], v[2]);
    let r1 = x * x + y * y + z * z;
    let th = (x * x + z * z).sqrt().atan2(y) * power;
    let ph = z.atan2(x) * power;
    // fyl2x / f2xm1 power: r1^(power / 2)
    let r1 = (r1.ln() * power * 0.5).exp();
    let (st, ct) = th.sin_cos();
    let (sp, cp) = ph.sin_cos();
    v[0] = cp * ct * r1 + j[0];
    v[2] = st * r1 * z_mul + j[2];
    v[1] = ct * sp * r1 + j[1];
    if !rot_c {
        return;
    }
    let mut pd = power_c;
    if powc_dist {
        let d = [v[0] - j[0], v[1] - j[1], v[2] - j[2]];
        pd *= (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    }
    let r = (j[0] * j[0] + j[1] * j[1] + j[2] * j[2]).sqrt();
    // the components for the angles: (a, b, c) = (Cy, Cz, Cx) or the
    // differences of Z and C; bit 8 swaps the first two
    let (mut a, mut b, c) = if mode & 16 != 0 { (v[1] - j[1], v[2] - j[2], v[0] - j[0]) } else { (j[1], j[2], j[0]) };
    // offset of the component tested by "conditional phi": 0 = x, 8 = y, 16 = z
    let mut ofs = 0;
    let mut ecx = 8;
    if mode & 8 != 0 {
        std::mem::swap(&mut a, &mut b);
        ecx = 16;
    }
    let s = (b * b + c * c).sqrt();
    let mut th = if mode & 1 != 0 { a.atan2(s) } else { s.atan2(a) };
    let mut ph = if mode & 2 != 0 {
        ofs = 24 - ecx;
        c.atan2(b)
    } else {
        b.atan2(c)
    };
    if mode & 4 != 0 {
        std::mem::swap(&mut th, &mut ph);
        ofs = ecx;
    }
    if cond_phi {
        let t = match ofs {
            0 => v[0],
            8 => v[1],
            _ => v[2],
        };
        if !t.is_sign_negative() {
            ph = -ph;
        }
    }
    let (sa, ca) = (ph * pd).sin_cos();
    let (sb, cb) = (th * pd).sin_cos();
    j[0] = ca * cb * r;
    j[2] = sb * r * cz_mul;
    j[1] = cb * sa * r;
}

/// `x = abs(x+fold) - abs(x-fold) - x`
#[inline(always)]
fn box_fold(x: f64, fold: f64) -> f64 {
    (x + fold).abs() - ((x - fold).abs() + x)
}

/// `HybridCube` / `HybridCubeDE` (amazing box).
#[inline]
fn amazing_box(scale: f64, min_r: f64, fold: f64, v: &mut [f64; 4], j: &[f64; 4], ade: bool) {
    let x = box_fold(v[0], fold);
    let y = box_fold(v[1], fold);
    let z = box_fold(v[2], fold);
    let r = z * z + y * y + x * x;
    let sqr_min_r = min_r * min_r;
    let mul = if r < sqr_min_r {
        scale / sqr_min_r
    } else if 1.0 < r {
        scale
    } else {
        scale / r
    };
    if ade {
        v[3] *= mul;
    }
    v[2] = z * mul + j[2];
    v[1] = y * mul + j[1];
    v[0] = x * mul + j[0];
}

/// `HybridItIntPow2scale`: sine bulb power 2 with scaling (used by Bulbox).
#[inline]
fn int_pow2_scale(scaling: f64, v: &mut [f64; 4], j: &[f64; 4]) {
    let s = 1.0 / scaling;
    let x = v[0] * s;
    let y = v[1] * s;
    let z = v[2] * s;
    let xx = x * x;
    let yy = y * y;
    let r = xx + yy;
    v[2] = -((r.sqrt() * z) * 2.0) * scaling + j[2];
    let a = (r - z * z) / r;
    v[0] = (xx - yy) * a * scaling + j[0];
    v[1] = (x * y * a) * 2.0 * scaling + j[1];
}

/// `HybridFloatPow` ('Real Power'). Uses the stored squared radius `rout`.
#[inline]
fn real_power(pow: f64, z_mul: f64, v: &mut [f64; 4], j: &[f64; 4], rout: f64) {
    let (x, y, z) = (v[0], v[1], v[2]);
    let theta = y.atan2(x) * pow;
    let (sin_t, cos_t) = theta.sin_cos();
    let phi = z.atan2((x * x + y * y).sqrt()) * pow;
    let (sin_p, cos_p) = phi.sin_cos();
    let r = rout.powf(pow * 0.5);
    v[0] = cos_t * cos_p * r + j[0];
    v[1] = sin_t * cos_p * r + j[1];
    v[2] = sin_p * r * z_mul + j[2];
}

/// `fHIntFunctions[power]`: the fast integer power triplex formulas.
#[inline]
fn int_pow(power: i32, zmul: f64, v: &mut [f64; 4], j: &[f64; 4]) {
    let (x, y, z) = (v[0], v[1], v[2]);
    const EPS: f64 = 1e-40;
    match power {
        i32::MIN..=2 => {
            // HybridItIntPow2 (sine bulb)
            let yy = y * y;
            let xx = x * x;
            let r = xx + yy;
            v[2] = (r.sqrt() * zmul * z) * 2.0 + j[2];
            let a = (r - z * z) / r;
            v[0] = (xx - yy) * a + j[0];
            v[1] = (x * y * a) * 2.0 + j[1];
        }
        3 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let a = 1.0 - 3.0 * sz / (r + EPS);
            v[0] = (sx - 3.0 * sy) * a * x + j[0];
            v[2] = j[2] - (sz - 3.0 * r) * z * zmul;
            v[1] = a * (3.0 * sx - sy) * y + j[1];
        }
        4 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let a = 1.0 + sz * (sz - 6.0 * r) / (r * r + EPS);
            v[0] = (sx * (sx - 6.0 * sy) + sy * sy) * a + j[0];
            v[2] = r.sqrt() * (r - sz) * z * 4.0 * zmul + j[2];
            v[1] = a * x * (sx - sy) * 4.0 * y + j[1];
        }
        5 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let a = 1.0 + (sz * sz - sz * r * 2.0) * 5.0 / (r * r + EPS);
            v[1] = (5.0 * sx * sx - sy * (10.0 * sx - sy)) * a * y + j[1];
            v[2] = (sz * (sz - 10.0 * r) + 5.0 * r * r) * z * zmul + j[2];
            v[0] = a * x * (sx * (sx - 10.0 * sy) + 5.0 * sy * sy) + j[0];
        }
        6 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let rr = r * r;
            let a = 1.0 - sz * (15.0 * rr + sz * (sz - r * 15.0)) / (rr * r + EPS);
            v[1] = (3.0 * sy * sy + sx * (3.0 * sx - 10.0 * sy)) * a * y * x * 2.0 + j[1];
            v[2] = (sz * (3.0 * sz - 10.0 * r) + 3.0 * rr) * r.sqrt() * z * zmul * 2.0 + j[2];
            v[0] = (sy * sy * (15.0 * sx - sy) + sx * sx * (sx - 15.0 * sy)) * a + j[0];
        }
        7 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let rr = r * r;
            let a = 1.0 - 7.0 * (sz * (3.0 * rr + sz * (sz - 5.0 * r))) / (rr * r + EPS);
            v[1] = (sx * (21.0 * sy * sy + sx * (7.0 * sx - 35.0 * sy)) - sy * sy * sy) * a * y
                + j[1];
            v[2] = j[2] - (sz * sz * sz - 7.0 * r * (rr + sz * (3.0 * sz - 5.0 * r))) * z * zmul;
            v[0] = a * x * (sx * (35.0 * sy * sy + sx * (sx - 21.0 * sy)) - 7.0 * sy * sy * sy)
                + j[0];
        }
        _ => {
            // HybridIntP8 (White's / Nylander's power 8)
            let xx = x * x;
            let yy = y * y;
            let zz = z * z;
            let r = xx + yy;
            let rr = r * r;
            let zzzz = zz * zz;
            let t = (zzzz - 6.0 * r * zz + rr) * (zz - r) * r.sqrt();
            v[2] = -(t * z * 8.0 * zmul) + j[2];
            let num = (rr * 70.0 + zzzz) * zzzz - 28.0 * zz * r * (zzzz + rr);
            let a = num / (rr * rr + EPS) + 1.0;
            let xxxx = xx * xx;
            let yyyy = yy * yy;
            v[1] = (yyyy * (7.0 * xx - yy) + xxxx * (xx - 7.0 * yy)) * 8.0 * x * y * a + j[1];
            v[0] = a
                * (xxxx * xxxx + yyyy * (70.0 * xxxx + yyyy) - 28.0 * (xx * yy * (yyyy + xxxx)))
                + j[0];
        }
    }
}

#[cfg(test)]
mod tests {

    /// `AexionC` against the Pascal reference kept in formulas.pas (mode 0,
    /// which the x87 code extends with the mode bits).
    #[test]
    fn aexion_c_matches_pascal_reference() {
        let f = Formula::default_for("Aexion C").unwrap();
        for &(x, y, z, cx, cy, cz) in &[(0.3, -0.4, 0.5, 0.2, 0.1, -0.3), (-0.7, 0.2, 0.1, -0.5, 0.4, 0.6), (0.05, 0.9, -0.2, 0.3, -0.8, 0.1)] {
            let mut v = [x, y, z, 0.0];
            let mut j = [cx, cy, cz, 0.0];
            f.iterate(&mut v, &mut j, 0.0, false);
            // reference
            let p = 8.0f64;
            let r1 = x * x + y * y + z * z;
            let th = (x * x + z * z).sqrt().atan2(y) * p;
            let ph = z.atan2(x) * p;
            let r1 = r1.powf(p * 0.5);
            let (xn, yn, zn) = (th.cos() * ph.cos() * r1 + cx, th.cos() * ph.sin() * r1 + cy, r1 * th.sin() + cz);
            let pd = 8.0;
            let r = (cx * cx + cy * cy + cz * cz).sqrt();
            let th2 = (cx * cx + cz * cz).sqrt().atan2(cy) * pd;
            let mut ph2 = cz.atan2(cx) * pd;
            if xn > 0.0 {
                ph2 = -ph2;
            }
            let j_ref = [th2.cos() * ph2.cos() * r, th2.cos() * ph2.sin() * r, r * th2.sin()];
            for (a, b) in v.iter().zip([xn, yn, zn]) {
                assert!((a - b).abs() < 1e-9, "{v:?} vs {:?}", [xn, yn, zn]);
            }
            for k in 0..3 {
                assert!((j[k] - j_ref[k]).abs() < 1e-9, "{j:?} vs {j_ref:?}");
            }
        }
    }

    use super::*;

    #[test]
    fn modern_bulbs_produce_finite_steps() {
        for name in [
            "SinCos Bulb",
            "SinCos Bulb V2",
            "SinCos Bulb V3",
            "Tails Bulb",
            "Mandelbar Bulb",
        ] {
            let f = Formula::default_for(name).unwrap();
            let mut v = [0.31, -0.42, 0.27, 0.0];
            let mut j = [0.12, -0.08, 0.20, 0.0];
            let rout = v[..3].iter().map(|x| x * x).sum();
            f.iterate(&mut v, &mut j, rout, false);
            assert!(v[..3].iter().all(|x| x.is_finite()), "{name}: {v:?}");
        }
    }

    /// Reference triplex power formula (spherical coordinates, as used by
    /// White/Nylander):  r^n (cos(n th) cos(n ph), sin(n th) cos(n ph), -sin(n ph))
    fn triplex(n: i32, v: [f64; 3]) -> [f64; 3] {
        let r = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let th = v[1].atan2(v[0]) * n as f64;
        let ph = v[2].atan2((v[0] * v[0] + v[1] * v[1]).sqrt()) * n as f64;
        let rn = r.powi(n);
        [rn * th.cos() * ph.cos(), rn * th.sin() * ph.cos(), -rn * ph.sin()]
    }

    #[test]
    fn int_powers_match_spherical_triplex() {
        let p = [0.31, -0.42, 0.27];
        for n in 2..=8 {
            let mut v = [p[0], p[1], p[2], 0.0];
            let f = Formula::IntPow { power: n, z_mul: -1.0 };
            f.iterate(&mut v, &mut [0.0; 4], 0.0, false);
            let r = triplex(n, p);
            for k in 0..3 {
                let expect = r[k];
                assert!(
                    (v[k] - expect).abs() < 1e-9,
                    "power {n} comp {k}: {} vs {}",
                    v[k],
                    expect
                );
            }
        }
    }

    #[test]
    fn real_power_matches_int_power8() {
        let p = [0.31, -0.42, 0.27];
        let rout = p.iter().map(|a| a * a).sum::<f64>();
        let mut a = [p[0], p[1], p[2], 0.0];
        let mut b = a;
        Formula::IntPow { power: 8, z_mul: -1.0 }.iterate(&mut a, &mut [0.0; 4], rout, false);
        Formula::RealPower { power: 8.0, z_mul: -1.0 }.iterate(&mut b, &mut [0.0; 4], rout, false);
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-9);
        }
    }

    #[test]
    fn box_scales_derivative() {
        let mut v = [0.1, 0.1, 0.1, 1.0];
        Formula::AmazingBox { scale: 2.0, min_r: 0.5, fold: 1.0 }.iterate(
            &mut v,
            &mut [0.0; 4],
            0.0,
            true,
        );
        assert!((v[3] - 8.0).abs() < 1e-12); // r < minR^2 -> scale / minR^2
    }
}
