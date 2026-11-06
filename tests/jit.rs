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

/// The community formulas shipped with the program (M3Formulas-community,
/// see its CREDITS.md) all load, and those with a `[SOURCE]` compile; one
/// step from a few points does not panic.
#[test]
fn the_community_formulas_load() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/M3Formulas-community"));
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "m3f") {
            continue;
        }
        let f = M3f::load(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        if has_source(&p) {
            assert!(f.jit.is_some(), "{}: the JIT source did not compile", p.display());
            for v in [[0.5, -0.25, 0.75, 1.0], [0.0, 0.0, 0.0, 1.0]] {
                step(&f, v, [0.1, 0.2, 0.3]);
            }
        }
        n += 1;
    }
    assert_eq!(n, 129);
}

/// Every community light preset is a light record the program reads.
#[test]
fn the_community_light_presets_load() {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/M3Lights-community"));
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "m3l") {
            continue;
        }
        let d = std::fs::read(&p).unwrap();
        assert!(mb3d::m3p::light_from_record(&d).is_some(), "{}: not a light record", p.display());
        n += 1;
    }
    assert_eq!(n, 608);
}

/// The Pascal formulas compiled with Cranelift (`jit_pascal`) give the
/// closures' results bit for bit (NaN payloads aside) from random states,
/// with the default option values and changed ones; and the statements of
/// every kind (loops with break / continue / exit, case, integer and
/// Boolean operations) in a test formula.
#[cfg(jit)]
#[test]
fn the_compiled_pascal_formulas_give_the_closures_results() {
    let mut defs: Vec<M3f> = Vec::new();
    for dir in ["/M3Formulas-community", "/tests/formulas"] {
        let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR")).to_string() + dir);
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "m3f") {
                if let Ok(f) = M3f::load(&e.path()) {
                    if f.jit.is_some() {
                        defs.push(f);
                    }
                }
            }
        }
    }
    defs.push(M3f::parse("statements", STATEMENTS).unwrap());
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut rnd = move || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let same = |a: f64, b: f64| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
    for def in &defs {
        let prog = def.jit.as_ref().unwrap();
        for k in 0..3 {
            let values: Vec<f64> = def.defaults().iter().map(|d| if k == 0 { *d } else if d.fract() == 0.0 { d + k as f64 } else { d * (0.7 + 0.3 * k as f64) }).collect();
            let c = mb3d::jit_pascal::compile(prog, def, &values).unwrap_or_else(|| panic!("{}: not compiled", def.name));
            for _ in 0..200 {
                let mut it = Iteration::default();
                for (v, j) in it.v.iter_mut().zip(it.j.iter_mut()) {
                    *v = rnd() * 3.0 - 1.5;
                    *j = rnd() * 2.4 - 1.2;
                }
                it.c = [rnd() - 0.5, rnd() - 0.5, rnd() - 0.5];
                it.rout = rnd() * 4.0;
                it.deriv1 = 1.0 + rnd();
                it.max_it = 60;
                it.it_result = (rnd() * 60.0) as i32;
                let mut a = it.clone();
                let mut b = it.clone();
                prog.run(&mut a, def, &values);
                c.run(&mut b);
                let fa = [a.v[0], a.v[1], a.v[2], a.v[3], a.j[0], a.j[1], a.j[2], a.rout, a.deriv1, a.otrap];
                let fb = [b.v[0], b.v[1], b.v[2], b.v[3], b.j[0], b.j[1], b.j[2], b.rout, b.deriv1, b.otrap];
                assert!(fa.iter().zip(&fb).all(|(x, y)| same(*x, *y)) && a.it_result == b.it_result, "{} {values:?}: closures {fa:?}, compiled {fb:?}", def.name);
            }
        }
    }
}

#[cfg(jit)]
const STATEMENTS: &str = r#"[OPTIONS]
.Version = 9
.Double Mode = 2
.Double Count = 4
[SOURCE]
procedure T(var x, y, z, w: Double; PIteration3D: TPIteration3D);
const two = 2;
var i, n, m: integer; k: int64; s, c: double; b: boolean;
begin
  n := 0;
  for i := 1 to round(Count) * 3 do begin
    if odd(i) then continue;
    n := n + i;
    if n > 20 then break;
  end;
  m := 7;
  for i := 10 downto 1 do Dec(m, i mod 3);
  x := x + n div 4 + n mod 4 + (m shl 2) + (m shr 1) - (not m) + (m xor 5) + (m and 6) + (m or 1);
  k := trunc(x * 1000000);
  y := y + 7 / two + k mod 1000 + Abs(-m) + Sqr(m) + Sign(z) + IntPower(1.1, m) + Power(Abs(y) + 0.5, 3) + Power(2, z);
  SinCos(x, s, c);
  z := z + s*s + c*c + Max(x, y) - Min(m, n) + Frac(y) + Int(z) + Hypot(x, y) + ArcTan2(y, x);
  b := (x > 0.5) and not (y < 0) or (z = 1) xor (Mode >= 2);
  if b then w := round(z) + Ceil(x) + Floor(y) else w := -1;
  i := 0;
  while true do begin Inc(i, 3); if i > 10 then break; end;
  repeat Dec(i) until i < 5;
  case trunc(Mode + w) mod 5 of
    0..1: x := x * 0.5;
    2, 3: y := y * 0.5;
  else z := -z;
  end;
  if IsNan(x) or IsInfinite(y) then x := 0;
  PIteration3D^.J1 := PIteration3D^.J2 * 2 + PIteration3D^.Rout;
  PIteration3D^.Deriv1 := PIteration3D^.Deriv1 * 2;
  if PIteration3D^.ItResultI > 30 then exit;
  w := w + 99;
end;
[END]
"#;

/// The compiled hybrid loop (`clif_jit::compile_hybrid`) with Pascal
/// formulas inline, alone and between built-in ones, gives the results of
/// the formulas called one by one (NaN payloads aside).
#[cfg(jit)]
#[test]
fn the_compiled_hybrid_loop_with_pascal_formulas_gives_the_formulas_results() {
    mb3d::formulas::add_formula_dir(PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/formulas")));
    let base = Scene::preset("Integer Power").unwrap().to_text();
    let head: String = base.split("\n[formula]").next().unwrap().to_string();
    for (k, hybrid) in [vec![("JIT_IQ_Bulb", 1)], vec![("JIT_IQ_Bulb", 2), ("Integer Power", 1), ("JITBenesiPineTree", 1)]].into_iter().enumerate() {
        let mut text = head.clone();
        for (name, its) in &hybrid {
            text.push_str(&format!("\n[formula]\nname = {name}\niterations = {its}\n"));
        }
        let path = std::env::temp_dir().join(format!("mb3d-pascal-hybrid-{}-{k}.m3s", std::process::id()));
        std::fs::write(&path, &text).unwrap();
        let (mut sc, _) = mb3d::animfile::load_scene(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        sc.width = 64;
        sc.height = 48;
        let plain = mb3d::calc::CalcParams::new(&sc).unwrap();
        let mut fused = plain.clone();
        let mut m = plain.machine.clone().unwrap_or_else(|| mb3d::custom::build_machine(&[]));
        let h = mb3d::x86::clif_jit::compile_hybrid(&m, &fused.slots, 0, fused.end_to, fused.repeat_from, false).unwrap_or_else(|| panic!("{hybrid:?}: not compiled"));
        m.jit = Some(std::sync::Arc::new(h));
        fused.machine = Some(m);
        let mut plain = plain;
        if let Some(mm) = plain.machine.as_mut() {
            mm.jit = None;
        }
        let (mut a, mut b) = (mb3d::calc::Marcher::new(&fused, 1), mb3d::calc::Marcher::new(&plain, 1));
        let same = |x: f64, y: f64| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan());
        for i in 0..4000u64 {
            let c = [(i % 20) as f64 * 0.12 - 1.2, ((i / 20) % 20) as f64 * 0.12 - 1.2, (i / 400) as f64 * 0.25 - 1.2];
            let (na, nb) = (a.iterations_at(c), b.iterations_at(c));
            assert!(na == nb && same(a.it.rout, b.it.rout) && a.it.v.iter().zip(&b.it.v).all(|(x, y)| same(*x, *y)), "{hybrid:?} at {c:?}: {na} {nb}, {:?} {:?}", a.it.v, b.it.v);
        }
    }
}
