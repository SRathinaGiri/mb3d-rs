//! The formulas compiled with Cranelift (`x86::lift_clif`) against their
//! translated direct versions: both run from the same random states (the
//! stack below the call full of garbage); the iteration fields and the
//! memory image must be identical bit for bit.  Also says which formulas
//! cannot be compiled and why.
//! `cargo run --release --features jit --example clifcheck -- <M3Formulas dir> [--trials N] [-v]`
use mb3d::custom::*;
use mb3d::iteration::Iteration;
use mb3d::m3f::M3f;
use mb3d::x86::clif_jit::Jit;
use std::collections::BTreeMap;
use std::sync::Arc;

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

/// The fields of `it` a formula may read or write, as bits.
fn fields(it: &Iteration) -> Vec<u64> {
    let canon = std::env::var_os("CANON_NAN").is_some();
    let fx = |x: &f64| if canon && x.is_nan() { 0x7ff8_0000_0000_0000 } else { x.to_bits() };
    let mut v: Vec<u64> = Vec::new();
    v.extend(it.v.iter().map(fx));
    v.extend(it.j.iter().map(fx));
    v.extend([it.rout, it.otrap, it.vary_scale, it.deriv1, it.deriv2, it.deriv3, it.dfree[0], it.dfree[1]].iter().map(fx));
    if canon {
        return v;
    }
    v.clear();
    v.extend(it.v.iter().map(|x| x.to_bits()));
    v.extend(it.j.iter().map(|x| x.to_bits()));
    v.extend(it.c.iter().map(|x| x.to_bits()));
    v.extend([it.rout, it.rold, it.otrap, it.vary_scale, it.deriv1, it.deriv2, it.deriv3, it.dfree[0], it.dfree[1]].iter().map(|x| x.to_bits()));
    v.extend([it.it_result as u64, it.max_it as u64, it.first_it as u64]);
    v
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let dir = &a[0];
    let trials: usize = a.iter().position(|x| x == "--trials").and_then(|i| a.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(16);
    let verbose = a.iter().any(|x| x == "-v");
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3f"))).collect();
    files.sort();
    let mut jit = Jit::new().expect("cranelift");
    if a.iter().any(|x| x == "--builtins") {
        builtins(&mut jit, trials.max(1000));
        return;
    }
    let (mut same, mut differ, mut direct) = (0, 0, 0);
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    let slot = 2;
    for path in &files {
        let Ok(def) = M3f::load(path) else { continue };
        let def = Arc::new(def);
        if def.de_option >= 20 {
            continue; // dIFS: not yet
        }
        let cf = CustomFormula::new(def.clone());
        let mut slots: Vec<Option<&CustomFormula>> = vec![None; slot];
        slots.push(Some(&cf));
        let base = build_machine(&slots);
        let Some(prog) = base.prog(code_addr(slot)) else { continue };
        if !prog.is_direct() {
            continue;
        }
        direct += 1;
        if let Some(why) = prog.clif_unsupported() {
            let key = why.split(':').next_back().unwrap_or("").trim().split([' ', '(', '{']).next().unwrap_or("").to_string();
            *reasons.entry(if why.starts_with("op") { key } else { why.clone() }).or_insert(0) += 1;
            if verbose {
                println!("SKIP  {}: {why}", def.name);
            }
            continue;
        }
        let Some(f) = jit.compile_direct(&base, code_addr(slot), pconst_addr(slot)) else {
            *reasons.entry("compile failed".into()).or_insert(0) += 1;
            println!("COMPILE FAILED {}", def.name);
            continue;
        };
        let mut rng = Rng(0x1234_5678 ^ (def.name.len() as u64 * 7919));
        let mut status = None;
        for t in 0..trials {
            let mut m = base.clone();
            for a in (STACK_TOP - 0x1000..STACK_TOP).step_by(4) {
                m.wr32(a, rng.range(0.0, 4294967295.0) as u32).unwrap();
            }
            let mut it = Iteration::default();
            it.v = [rng.range(-1.5, 1.5), rng.range(-1.5, 1.5), rng.range(-1.5, 1.5), rng.range(-0.5, 0.5)];
            it.c = [rng.range(-1.2, 1.2), rng.range(-1.2, 1.2), rng.range(-1.2, 1.2)];
            it.j = [it.c[0], it.c[1], it.c[2], 0.1];
            it.rout = it.v[0] * it.v[0] + it.v[1] * it.v[1] + it.v[2] * it.v[2];
            it.rold = 0.7;
            it.otrap = 0.5;
            it.it_result = 3 + t as i32;
            it.max_it = 60;
            it.rstop = 1024.0 * 1024.0;
            it.vary_scale = 1.0;
            it.first_it = t as i32 % 2;
            it.deriv1 = 1.0;
            // NaN and infinities: the unordered compares
            if t % 4 == 3 {
                let special = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY][(t / 4) % 3];
                it.v[(t / 12) % 3] = special;
                it.rout = it.v[0] * it.v[0] + it.v[1] * it.v[1] + it.v[2] * it.v[2];
            }
            let mut m2 = m.clone();
            let mut it2 = it.clone();
            let r1 = m.call_direct(code_addr(slot), &mut it, pconst_addr(slot), MAX_STEPS);
            // SAFETY: the compiled function takes these pointers
            let r2 = unsafe { f(&mut it2, m2.mem.as_mut_ptr(), &m2, MAX_STEPS) };
            let ok1 = matches!(r1, Some(Ok(())));
            let mem_diff = (0..m.mem.len()).find(|&i| m.mem[i] != m2.mem[i]);
            // NaN results compare by their bits (both sides run the same
            // operations, so even the NaN payloads agree)
            if ok1 != (r2 == 0) || (ok1 && (fields(&it) != fields(&it2) || mem_diff.is_some())) {
                let fd: Vec<usize> = fields(&it).iter().zip(fields(&it2)).enumerate().filter(|(_, (a, b))| *a != b).map(|(i, _)| i).collect();
                status = Some(format!("trial {t}: translated {r1:?}, compiled {r2}, fields differ {fd:?}, memory differs at {mem_diff:?}"));
                break;
            }
        }
        match status {
            None => same += 1,
            Some(s) => {
                differ += 1;
                println!("DIFF  {}: {s}", def.name);
            }
        }
    }
    println!("{direct} direct formulas: {same} compiled and identical, {differ} differ; not compiled:");
    for (k, v) in reasons {
        println!("  {v:4} {k}");
    }
}

/// The compiled built-in formulas against `Formula::iterate`.
fn builtins(jit: &mut Jit, trials: usize) {
    use mb3d::formulas::Formula;
    let mut fs: Vec<Formula> = Vec::new();
    for power in 2..=8 {
        for z_mul in [-1.0, 1.0, 0.7] {
            fs.push(Formula::IntPow { power, z_mul });
            fs.push(Formula::FoldingIntPow { power, z_mul, fold: 1.0 });
            fs.push(Formula::FoldingIntPow { power, z_mul, fold: 0.6 });
        }
    }
    for (scale, min_r, fold) in [(2.0, 0.5, 1.0), (-1.5, 0.5, 1.0), (-2.4, 0.25, 0.9), (3.0, 0.0, 1.2)] {
        fs.push(Formula::AmazingBox { scale, min_r, fold });
    }
    fs.push(Formula::Quaternion { yw_mul: -1.0, w_add: 0.0 });
    fs.push(Formula::Quaternion { yw_mul: 0.5, w_add: 0.3 });
    fs.push(Formula::Tricorn { z_mul: -1.0, cz_mul: 1.0 });
    fs.push(Formula::Tricorn { z_mul: 0.8, cz_mul: -0.5 });
    let mut rng = Rng(0x5eed);
    let (mut ok, mut bad) = (0, 0);
    for f in &fs {
        for ade in [false, true] {
            let c = jit.compile_builtin(f, ade).expect("compiled built-in");
            let mut diff = None;
            for t in 0..trials {
                let mut v = [rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), rng.range(-1.0, 3.0)];
                let mut j = [rng.range(-1.2, 1.2), rng.range(-1.2, 1.2), rng.range(-1.2, 1.2), rng.range(-0.5, 0.5)];
                if t % 50 == 7 {
                    v[t % 3] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0][(t / 50) % 4];
                }
                if t % 97 == 5 {
                    v = [0.0; 4];
                }
                let rout = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
                let (mut v2, mut j2) = (v, j);
                f.iterate(&mut v, &mut j, rout, ade);
                // SAFETY: the compiled function takes these arrays
                unsafe { c(&mut v2, &mut j2, rout) };
                let canon = |x: f64| if x.is_nan() { u64::MAX } else { x.to_bits() };
                if v.map(canon) != v2.map(canon) || j.map(canon) != j2.map(canon) {
                    diff = Some(format!("trial {t}: {v:?} vs {v2:?}"));
                    break;
                }
            }
            match diff {
                None => ok += 1,
                Some(d) => {
                    bad += 1;
                    println!("DIFF  {f:?} ade {ade}: {d}");
                }
            }
        }
    }
    println!("{} built-in variants: {ok} identical, {bad} differ ({trials} trials each; NaNs compared as NaN)", ok + bad);
}
