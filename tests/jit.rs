//! Formulas written in Pascal (`[SOURCE]`, MB3D's JIT formulas).

use mb3d::iteration::Iteration;
use mb3d::m3f::M3f;
use mb3d::scene::Scene;
use std::path::{Path, PathBuf};

fn formula(name: &str) -> M3f {
    M3f::load(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formulas")).join(format!("{name}.m3f"))).unwrap()
}

fn step(def: &M3f, v: [f64; 4], j: [f64; 3]) -> [f64; 4] {
    let mut it = Iteration { v, j: [j[0], j[1], j[2], 0.0], ..Default::default() };
    def.jit.as_ref().unwrap().run(&mut it, def, &def.defaults());
    it.v
}

#[test]
fn iq_bulb_matches_its_definition() {
    let def = formula("JIT_IQ_Bulb");
    assert!(def.jit.as_ref().unwrap().name.eq_ignore_ascii_case("MyFormula"));
    let p = 8.0f64;
    for (v, j) in [([0.3f64, -0.7, 0.5, 1.0], [0.1, 0.2, -0.3]), ([-1.1, 0.2, 0.05, 1.0], [0.0, 0.0, 0.0])] {
        let [x, y, z, _] = v;
        let sq_r = (x * x + y * y + z * z).sqrt();
        let sq_xz = (x * x + z * z).sqrt();
        let r = sq_r.powf(p);
        let theta = sq_xz.atan2(y) * p;
        let zangle = x.atan2(z) * p;
        let want = [zangle.sin() * theta.sin() * r + j[0], theta.cos() * r + j[1], theta.sin() * zangle.cos() * r + j[2]];
        let got = step(&def, v, j);
        for k in 0..3 {
            assert!((got[k] - want[k]).abs() <= 1e-12 * want[k].abs().max(1.0), "{k}: {got:?} {want:?}");
        }
        assert_eq!(got[3], 1.0);
    }
}

#[test]
fn benesi_pine_tree_matches_its_definition() {
    let def = formula("JITBenesiPineTree");
    // constants come from [CONSTANTS], options Offset = 2, Scale = 2
    #[allow(clippy::approx_constant)]
    let (s12, s13, s23) = (0.707106781186548f64, 0.577350269189626, 0.816496580927726);
    let v = [0.4f64, -0.3, 0.8, 1.0];
    let j = [0.25, 0.5, 0.75];
    let [x, y, z, _] = v;
    let tx = x * s23 - z * s13;
    let nz = (x * s13 + z * s23).abs();
    let nx = ((tx - y) * s12).abs();
    let ny = ((tx + y) * s12).abs();
    let tx = (nx + ny) * s12;
    let ny = (-nx + ny) * s12;
    let nx = tx * s23 + nz * s13;
    let nz = -tx * s13 + nz * s23;
    let (x, y, z) = (2.0 * nx - 2.0, 2.0 * ny, 2.0 * nz);
    let t = (x + x) / (y * y + z * z).sqrt();
    let want = [x * x - y * y - z * z + j[0], t * 2.0 * y * z, t * (y * y - z * z)];
    let got = step(&def, v, j);
    for k in 0..3 {
        assert!((got[k] - want[k]).abs() < 1e-12, "{k}: {got:?} {want:?}");
    }
}

#[test]
fn jit_formulas_render() {
    mb3d::formulas::add_formula_dir(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formulas")));
    for name in ["JIT_IQ_Bulb", "JITBenesiPineTree"] {
        let mut s = Scene::preset(name).unwrap();
        s.width = 64;
        s.height = 48;
        let r = mb3d::render(&s, &|_, _| {}).unwrap();
        assert!(r.coverage() > 0.02, "{name}: coverage {}", r.coverage());
    }
}

/// All `[SOURCE]` formulas of the MB3D repository compile (set MB3D_REPO or
/// clone it next to this one).
#[test]
fn all_mb3d_jit_formulas_compile() {
    let repo = std::env::var("MB3D_REPO").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../mb3d")));
    let mut n = 0;
    for dir in ["M3Formulas", "EM_JIT_M3Formulas"] {
        let Ok(rd) = std::fs::read_dir(repo.join(dir)) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "m3f") || !has_source(&p) {
                continue;
            }
            let f = M3f::load(&p).unwrap_or_else(|e| panic!("{e}"));
            assert!(f.jit.is_some());
            // one step from a few points must not panic
            for v in [[0.5, -0.25, 0.75, 1.0], [0.0, 0.0, 0.0, 1.0], [-2.0, 1.0, 3.0, 0.0]] {
                step(&f, v, [0.1, 0.2, 0.3]);
            }
            n += 1;
        }
    }
    eprintln!("{n} JIT formulas compiled");
}

fn has_source(p: &Path) -> bool {
    std::fs::read(p).map(|b| b.windows(8).any(|w| w.eq_ignore_ascii_case(b"[SOURCE]"))).unwrap_or(false)
}
