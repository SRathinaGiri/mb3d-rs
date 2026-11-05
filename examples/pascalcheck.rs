//! The formulas written in Pascal (`[SOURCE]`): every formula runs from
//! the same random states (and option values near the defaults) and the
//! fields of the iteration after each call go into a checksum per formula,
//! so that a change of the compiler can be checked against the output
//! before it (`--save FILE`, then `--against FILE`).  With the JIT, the
//! compiled formulas (`jit_pascal`) are also compared with the closures
//! call by call (`--jit`).
//! `cargo run --release --example pascalcheck -- <formula dirs...> [--trials N] [--save F | --against F] [--jit] [-v]`
use mb3d::iteration::Iteration;
use mb3d::m3f::M3f;
use std::collections::BTreeMap;

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

/// A random iteration state.
fn random_state(r: &mut Rng) -> Iteration {
    let mut it = Iteration::default();
    for k in 0..4 {
        it.v[k] = r.range(-1.5, 1.5);
        it.j[k] = r.range(-1.2, 1.2);
        it.ju[k] = r.range(-1.0, 1.0);
    }
    if r.f() < 0.05 {
        it.v = [0.0; 4];
    }
    for k in 0..3 {
        it.c[k] = r.range(-1.2, 1.2);
    }
    it.rout = r.range(0.0, 4.0);
    it.rold = r.range(0.0, 4.0);
    it.otrap = r.range(0.0, 2.0);
    it.vary_scale = r.range(0.5, 2.0);
    it.deriv1 = r.range(0.5, 3.0);
    it.deriv2 = r.range(0.5, 3.0);
    it.deriv3 = r.range(0.5, 3.0);
    it.dfree = [r.range(-1.0, 1.0), r.range(-1.0, 1.0)];
    it.rstop = 16.0;
    it.max_it = 60;
    it.it_result = (r.f() * 60.0) as i32;
    it.first_it = (r.f() < 0.5) as i32;
    it.de_option = if r.f() < 0.5 { 0 } else { 2 };
    it.smooth_it = r.range(0.0, 1.0) as f32;
    it.do_julia = r.f() < 0.3;
    it.calc_sit = r.f() < 0.5;
    it
}

/// The fields a formula may change, as bits.
fn fields(it: &Iteration) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    v.extend(it.v.iter().chain(&it.j).chain(&it.ju).chain(&it.c).map(|x| x.to_bits()));
    v.extend([it.rout, it.rold, it.otrap, it.vary_scale, it.deriv1, it.deriv2, it.deriv3, it.dfree[0], it.dfree[1], it.rstop].iter().map(|x| x.to_bits()));
    v.extend([it.max_it as u64, it.it_result as u64, it.first_it as u64, it.de_option as u64, it.smooth_it.to_bits() as u64, it.do_julia as u64, it.calc_sit as u64]);
    v
}

/// Option values: the defaults, or each one changed a little (integers
/// stay integers).
fn option_values(def: &M3f, r: &mut Rng, k: usize) -> Vec<f64> {
    def.options
        .iter()
        .map(|o| {
            if k == 0 {
                return o.default;
            }
            let d = o.default;
            if d.fract() == 0.0 && d.abs() < 100.0 {
                (d + (r.f() * 5.0).floor() - 2.0).max(if d >= 0.0 { 0.0 } else { -100.0 })
            } else {
                d * r.range(0.6, 1.4)
            }
        })
        .collect()
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut dirs = Vec::new();
    let (mut trials, mut save, mut against, mut jit, mut verbose) = (2000usize, None, None, false, false);
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--trials" => {
                trials = a[i + 1].parse().unwrap();
                i += 1;
            }
            "--save" => {
                save = Some(a[i + 1].clone());
                i += 1;
            }
            "--against" => {
                against = Some(a[i + 1].clone());
                i += 1;
            }
            "--jit" => jit = true,
            "-v" => verbose = true,
            d => dirs.push(d.to_string()),
        }
        i += 1;
    }
    let mut files: Vec<std::path::PathBuf> = dirs
        .iter()
        .flat_map(|d| std::fs::read_dir(d).unwrap().filter_map(|e| e.ok().map(|e| e.path())))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3f")))
        .collect();
    files.sort_by_key(|p| p.file_name().map(|s| s.to_os_string()));
    files.dedup_by_key(|p| p.file_name().map(|s| s.to_os_string()));
    let mut sums: BTreeMap<String, u64> = BTreeMap::new();
    let (mut n, mut jit_ok, mut jit_bad, mut jit_none) = (0, 0, 0, 0);
    for f in &files {
        let Ok(def) = M3f::load(f) else { continue };
        let Some(prog) = def.jit.clone() else { continue };
        n += 1;
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let mut r = Rng(0x5eed ^ name.len() as u64);
        let mut h: u64 = 0xcbf29ce484222325;
        let mut bad = 0usize;
        let mut nan_only = 0usize;
        let mut compiled_any = false;
        let mut first_bad = String::new();
        for k in 0..8 {
            let values = option_values(&def, &mut r, k);
            #[cfg(jit)]
            let compiled = if jit { mb3d::jit_pascal::compile(&prog, &def, &values) } else { None };
            #[cfg(not(jit))]
            let compiled: Option<()> = None;
            compiled_any |= compiled.is_some();
            for t in 0..trials / 8 {
                let start = random_state(&mut r);
                let mut it = start.clone();
                prog.run(&mut it, &def, &values);
                let out = fields(&it);
                for x in &out {
                    h ^= *x;
                    h = h.wrapping_mul(0x100000001b3);
                }
                #[cfg(jit)]
                if let Some(c) = &compiled {
                    let mut it2 = start.clone();
                    c.run(&mut it2);
                    let out2 = fields(&it2);
                    // NaN payloads aside
                    let same = out.iter().zip(&out2).all(|(a, b)| a == b || (f64::from_bits(*a).is_nan() && f64::from_bits(*b).is_nan()));
                    if same && out != out2 {
                        nan_only += 1;
                    }
                    if !same {
                        if bad == 0 {
                            let k = out.iter().zip(&out2).position(|(a, b)| a != b).unwrap();
                            first_bad = format!("values {values:?} trial {t} field {k}: closures {} compiled {}", f64::from_bits(out[k]), f64::from_bits(out2[k]));
                        }
                        bad += 1;
                    }
                }
                #[cfg(not(jit))]
                let _ = (&compiled, t);
            }
        }
        if jit {
            if !compiled_any {
                jit_none += 1;
                println!("NOJIT {name}");
            } else if bad == 0 {
                jit_ok += 1;
                if nan_only > 0 {
                    println!("NaN   {name}: {nan_only} calls differ only in NaN bits");
                }
                if verbose {
                    println!("ok    {name}");
                }
            } else {
                jit_bad += 1;
                println!("DIFF  {name}: {bad} calls differ; first: {first_bad}");
            }
        }
        sums.insert(name, h);
    }
    if let Some(p) = &save {
        let s: String = sums.iter().map(|(k, v)| format!("{k}\t{v:016x}\n")).collect();
        std::fs::write(p, s).unwrap();
    }
    if let Some(p) = &against {
        let old: BTreeMap<String, String> = std::fs::read_to_string(p).unwrap().lines().filter_map(|l| l.split_once('\t').map(|(a, b)| (a.to_string(), b.to_string()))).collect();
        let mut differ = 0;
        for (k, v) in &sums {
            match old.get(k) {
                Some(o) if *o == format!("{v:016x}") => {}
                Some(_) => {
                    differ += 1;
                    println!("CHANGED {k}");
                }
                None => println!("NEW     {k}"),
            }
        }
        println!("against {p}: {} formulas, {differ} changed", sums.len());
    }
    println!("{n} Pascal formulas");
    if jit {
        println!("compiled: {jit_ok} identical, {jit_bad} differ, {jit_none} not compiled");
    }
}
