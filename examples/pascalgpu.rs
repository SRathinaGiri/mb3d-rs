//! The Pascal formulas translated to WGSL (`jit_wgsl`) against the closures:
//! every formula runs on random states on the graphics card (single
//! precision, or with `--dbl` double) and in the closures (double
//! precision); the record cells must agree within the precision.
//! `cargo run --release --example pascalgpu -- <formula dirs...> [--dbl] [-v]`
use mb3d::iteration::Iteration;
use mb3d::m3f::M3f;
use mb3d::x86::lift_wgsl::{prelude, record_globals, CellTy};

const TRIALS: usize = 256;

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

/// The double of a record cell (None: an integer cell).
fn cell(it: &Iteration, c: u32) -> Option<f64> {
    Some(match c {
        0 => it.j[3],
        1 => it.rold,
        3..=6 => it.v[c as usize - 3],
        7..=9 => it.c[c as usize - 7],
        10..=12 => it.j[c as usize - 10],
        14 => it.rout,
        31 => it.otrap,
        32 => it.vary_scale,
        34 | 35 => it.dfree[c as usize - 34],
        36 => it.deriv1,
        37 => it.deriv2,
        38 => it.deriv3,
        _ => return None,
    })
}

/// The two words of an integer cell.
fn int_cell(it: &Iteration, c: u32) -> [u32; 2] {
    match c {
        15 => [it.it_result as u32, it.max_it as u32],
        16 => [(it.rstop as f32).to_bits(), 0],
        33 => [it.first_it as u32, 0],
        _ => [0, 0],
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v");
    let dbl = args.iter().any(|a| a == "--dbl");
    let mut files: Vec<std::path::PathBuf> = args
        .iter()
        .filter(|a| !a.starts_with('-'))
        .flat_map(|d| std::fs::read_dir(d).unwrap().filter_map(|e| e.ok().map(|e| e.path())))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3f")))
        .collect();
    files.sort_by_key(|p| p.file_name().map(|s| s.to_os_string()));
    files.dedup_by_key(|p| p.file_name().map(|s| s.to_os_string()));
    let (mut translated, mut pass, mut fail, mut gpu_err, mut not) = (0, 0, 0, 0, 0);
    for f in &files {
        let Ok(def) = M3f::load(f) else { continue };
        let Some(prog) = def.jit.clone() else { continue };
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        let values = def.defaults();
        let (wf, words) = match mb3d::jit_wgsl::emit(&prog, &def, &values, "fm", dbl, &Default::default()) {
            Ok(x) => x,
            Err(e) => {
                not += 1;
                println!("NOT   {name}: {e}");
                continue;
            }
        };
        translated += 1;
        if wf.record.is_empty() {
            // nothing read or written (a formula that does nothing)
            pass += 1;
            continue;
        }
        let mut rng = Rng(0x9E37_79B9 ^ (name.len() as u64 * 7919));
        let (mut inp, mut want) = (Vec::new(), Vec::new());
        let put = |out: &mut Vec<u32>, it: &Iteration| {
            for (c, t) in &wf.record {
                match (cell(it, *c), t) {
                    (Some(x), CellTy::Whole) if dbl => out.extend([x.to_bits() as u32, (x.to_bits() >> 32) as u32]),
                    (Some(x), CellTy::Whole) => out.push((x as f32).to_bits()),
                    (Some(x), CellTy::Halves) => out.extend([x.to_bits() as u32, (x.to_bits() >> 32) as u32]),
                    (None, _) => out.extend(int_cell(it, *c)),
                }
            }
        };
        for _ in 0..TRIALS {
            let mut it = Iteration::default();
            for k in 0..4 {
                it.v[k] = rng.range(-1.5, 1.5);
            }
            it.v[3] = rng.range(0.5, 2.0);
            for k in 0..3 {
                it.c[k] = rng.range(-1.2, 1.2);
                it.j[k] = it.c[k];
            }
            it.j[3] = 0.1;
            it.rold = 0.7;
            it.rout = it.v[0] * it.v[0] + it.v[1] * it.v[1] + it.v[2] * it.v[2];
            it.it_result = 3;
            it.max_it = 60;
            it.rstop = 1024.0 * 1024.0;
            it.otrap = 0.5;
            it.vary_scale = 1.0;
            it.deriv1 = 1.0;
            put(&mut inp, &it);
            prog.run(&mut it, &def, &values);
            put(&mut want, &it);
        }
        let per: usize = wf.record.iter().map(|(c, t)| if cell(&Iteration::default(), *c).is_some() && *t == CellTy::Whole && !dbl { 1 } else { 2 }).sum();
        let mut load = String::new();
        let mut store = String::new();
        let mut i = 0;
        for (c, t) in &wf.record {
            match t {
                CellTy::Whole if dbl => {
                    load.push_str(&format!("    r{c}f = bitcast<f64>((u64(inp[b0 + {}u]) << 32u) | u64(inp[b0 + {i}u]));\n", i + 1));
                    store.push_str(&format!("    outp[b0 + {i}u] = u32(bitcast<u64>(r{c}f) & 0xFFFFFFFFlu); outp[b0 + {}u] = u32(bitcast<u64>(r{c}f) >> 32u);\n", i + 1));
                    i += 2;
                }
                CellTy::Whole => {
                    load.push_str(&format!("    r{c}f = bitcast<f32>(inp[b0 + {i}u]);\n"));
                    store.push_str(&format!("    outp[b0 + {i}u] = bitcast<u32>(r{c}f);\n"));
                    i += 1;
                }
                CellTy::Halves => {
                    load.push_str(&format!("    r{c}l = inp[b0 + {i}u]; r{c}h = inp[b0 + {}u];\n", i + 1));
                    store.push_str(&format!("    outp[b0 + {i}u] = r{c}l; outp[b0 + {}u] = r{c}h;\n", i + 1));
                    i += 2;
                }
            }
        }
        let head = if dbl { "alias R = f64;\n" } else { "alias R = f32;\n" };
        let wgsl = format!(
            "{head}@group(0) @binding(0) var<storage, read> inp: array<u32>;\n@group(0) @binding(1) var<storage, read> cst: array<u32>;\n@group(0) @binding(2) var<storage, read_write> outp: array<u32>;\n{}\n{}\n{}\n{}\n@compute @workgroup_size(64)\nfn main(@builtin(global_invocation_id) g: vec3<u32>) {{\n    let i = g.x;\n    if (i >= {TRIALS}u) {{ return; }}\n    _ = cst[0];\n    let b0 = i * {per}u;\n{load}    fm(0u);\n{store}}}\n",
            prelude(dbl),
            mb3d::jit_wgsl::prelude(dbl),
            record_globals(&[&wf]).unwrap(),
            wf.code
        );
        let cst = if words.is_empty() { vec![0u32] } else { words.clone() };
        let got = match mb3d::gpu::run_compute(&wgsl, &[&inp, &cst], TRIALS * per, (TRIALS as u32).div_ceil(64)) {
            Ok(v) => v,
            Err(e) => {
                gpu_err += 1;
                println!("GPU ERROR {name}: {}", e.lines().take(8).collect::<Vec<_>>().join(" | "));
                if verbose {
                    let p = std::env::temp_dir().join(format!("pascal_{name}.wgsl"));
                    std::fs::write(&p, &wgsl).ok();
                    println!("  shader in {}", p.display());
                }
                continue;
            }
        };
        let tol = if dbl { 1e-9 } else { 2e-3 };
        let (mut good, mut first_bad) = (0, String::new());
        for t in 0..TRIALS {
            let mut ok = true;
            let mut j = t * per;
            for (c, ty) in &wf.record {
                let wide = !(cell(&Iteration::default(), *c).is_some() && *ty == CellTy::Whole && !dbl);
                let (a, b) = if cell(&Iteration::default(), *c).is_none() {
                    ((want[j] as i32) as f64, (got[j] as i32) as f64)
                } else if wide {
                    (f64::from_bits(want[j] as u64 | (want[j + 1] as u64) << 32), f64::from_bits(got[j] as u64 | (got[j + 1] as u64) << 32))
                } else {
                    (f32::from_bits(want[j]) as f64, f32::from_bits(got[j]) as f64)
                };
                let close = (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= tol * a.abs().max(1.0) || (a.is_infinite() && b.is_infinite() && a.signum() == b.signum());
                if !close {
                    ok = false;
                    if first_bad.is_empty() {
                        first_bad = format!("trial {t} cell r{c}: cpu {a} gpu {b}");
                    }
                }
                j += if wide { 2 } else { 1 };
            }
            if ok {
                good += 1;
            }
        }
        if good * 100 >= TRIALS * 95 {
            pass += 1;
            if verbose {
                println!("ok    {name} ({good}/{TRIALS})");
            }
        } else {
            fail += 1;
            println!("FAIL  {name}: {good}/{TRIALS} agree; {first_bad}");
        }
    }
    println!("{translated} translated to WGSL: {pass} agree with the closures (95% of {TRIALS} inputs), {fail} do not, {gpu_err} shader errors; {not} not translated");
}
