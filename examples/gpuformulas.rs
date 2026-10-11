//! The formulas translated to WGSL (`x86::lift_wgsl`) against the
//! interpreter: every formula runs once on random inputs on the graphics
//! card (single precision) and in the interpreter (double precision); the
//! record cells must agree within single precision tolerance.
//! `cargo run --release --example gpuformulas -- <M3Formulas dir> [-v]`
use mb3d::custom::*;
use mb3d::m3f::M3f;
use mb3d::x86::lift_wgsl::{prelude, record_globals, CellTy};
use mb3d::x86::{Machine, BASE};
use std::sync::Arc;

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

/// The iteration record as `m3fcheck` sets it up.
fn setup(m: &mut Machine, rng: &mut Rng, de_option: i32) {
    let w = |m: &mut Machine, o: u32, v: f64| m.wrf64(IT_BASE + o, v).unwrap();
    let c = [rng.range(-1.2, 1.2), rng.range(-1.2, 1.2), rng.range(-1.2, 1.2)];
    let v = [rng.range(-1.5, 1.5), rng.range(-1.5, 1.5), rng.range(-1.5, 1.5), rng.range(-0.5, 0.5)];
    for k in 0..4 {
        w(m, off::X + 8 * k as u32, v[k]);
    }
    for k in 0..3 {
        w(m, off::C1 + 8 * k as u32, c[k]);
        w(m, off::J1 + 8 * k as u32, c[k]);
        w(m, off::JU1 + 8 * k as u32, c[k]);
    }
    w(m, off::J4, 0.1);
    w(m, off::ROLD, 0.7);
    w(m, off::RSTOPD, 1024.0);
    m.wr32(IT_BASE + off::PVAR, pconst_addr(0)).unwrap();
    w(m, off::ROUT, v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    m.wr32(IT_BASE + off::IT_RESULT, 3).unwrap();
    m.wr32(IT_BASE + off::MAX_IT, 60).unwrap();
    m.wrf32(IT_BASE + off::RSTOP, 1024.0 * 1024.0).unwrap();
    m.wr32(IT_BASE + off::N_HYBRID, 1).unwrap();
    m.wr32(IT_BASE + off::FHPVAR, pconst_addr(0)).unwrap();
    m.wr32(IT_BASE + off::DE_OPTION, de_option as u32).unwrap();
    w(m, off::OTRAP, 0.5);
    w(m, off::VARY_SCALE, 1.0);
    m.wr32(IT_BASE + off::FIRST_IT, 0).unwrap();
    w(m, off::DERIV1, 1.0);
    w(m, off::DERIV1 + 8, 0.0);
    w(m, off::DERIV1 + 16, 0.0);
    for i in 0..4u32 {
        m.wrf32(IT_BASE + off::SMATRIX4 + (i * 4 + i) * 4, 1.0).unwrap();
    }
}

fn cell_words(m: &Machine, c: u32, t: CellTy) -> Vec<u32> {
    let a = (IT_BASE + 8 * c - BASE) as usize;
    let lo = u32::from_le_bytes(m.mem[a..a + 4].try_into().unwrap());
    let hi = u32::from_le_bytes(m.mem[a + 4..a + 8].try_into().unwrap());
    match t {
        CellTy::Whole => vec![(f64::from_bits(lo as u64 | (hi as u64) << 32) as f32).to_bits()],
        CellTy::Halves => vec![lo, hi],
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v");
    let dir = &args[0];
    let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3f"))).collect();
    files.sort();
    let (mut translated, mut pass, mut fail, mut gpu_err) = (0, 0, 0, 0);
    for f in &files {
        let Ok(def) = M3f::load(f) else { continue };
        let name = def.name.clone();
        let difs = def.de_option >= 20;
        let de_option = def.de_option;
        let cf = CustomFormula::new(Arc::new(def));
        let base = build_machine(&[Some(&cf)]);
        let Some(p) = base.prog(code_addr(0)) else { continue };
        let Some(wf) = p.emit_wgsl("fm", difs) else {
            if verbose {
                println!("NOT   {name}");
            }
            continue;
        };
        translated += 1;
        // CPU runs, collecting the inputs and the results
        let mut rng = Rng(0x9E37_79B9 ^ name.len() as u64 * 7919);
        let (mut inp, mut want) = (Vec::new(), Vec::new());
        let mut ok_trials = Vec::new();
        for _ in 0..TRIALS {
            let mut m = base.clone();
            setup(&mut m, &mut rng, de_option);
            if difs {
                m.wrf64(IT_BASE + off::ROLD, 0.01).unwrap();
                m.wrf64(IT_BASE + off::RSTOPD, 0.01).unwrap();
            }
            for (c, t) in &wf.record {
                inp.extend(cell_words(&m, *c, *t));
            }
            let r = if difs {
                m.call_difs(code_addr(0), IT_BASE + 144, pconst_addr(0), 0, 1, STACK_TOP, MAX_STEPS)
            } else {
                m.call_formula(code_addr(0), IT_C1, STACK_TOP, MAX_STEPS)
            };
            ok_trials.push(r.is_ok());
            for (c, t) in &wf.record {
                want.extend(cell_words(&m, *c, *t));
            }
        }
        let vb = (var_buf_addr(0) - BASE) as usize;
        let cst = wf.constants_from(&base.mem[vb..vb + 0x400], false);
        // the shader: load the cells, the calling convention, the formula, store
        let words: usize = wf.record.iter().map(|(_, t)| if *t == CellTy::Whole { 1 } else { 2 }).sum();
        let mut load = String::new();
        let mut store = String::new();
        let mut i = 0;
        for (c, t) in &wf.record {
            match t {
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
        let regs = if difs {
            format!("rg = array<u32, 8>(0u, 1u, {:#x}u, 0u, {:#x}u, 0u, {:#x}u, {:#x}u);", IT_BASE + 272, STACK_TOP - 4, IT_BASE + 144, pconst_addr(0))
        } else {
            let x = IT_C1 - 32;
            format!("rg = array<u32, 8>({:#x}u, {:#x}u, {:#x}u, 0u, {:#x}u, 0u, 0u, 0u);", x, x + 16, x + 8, STACK_TOP - 12)
        };
        let wgsl = format!(
            "@group(0) @binding(0) var<storage, read> inp: array<u32>;\n@group(0) @binding(1) var<storage, read> cst: array<u32>;\n@group(0) @binding(2) var<storage, read_write> outp: array<u32>;\nalias R = f32;\n{}\n// no maps are loaded here: the interpreter's map functions give 0\nfn map_px(sphere: bool, nr: i32, v: vec3<R>) -> vec3<R> {{ return vec3<R>(0.0); }}\n{}\n{}\n@compute @workgroup_size(64)\nfn main(@builtin(global_invocation_id) g: vec3<u32>) {{\n    let i = g.x;\n    if (i >= {TRIALS}u) {{ return; }}\n    _ = cst[0];\n    let b0 = i * {words}u;\n{load}    {regs}\n    xcf = false; xzf = false; xsf = false; xof = false; xpf = false; xaf = false; fsw = 0u;\n    fm(0u);\n{store}}}\n",
            prelude(false),
            record_globals(&[&wf]).unwrap(),
            wf.code.replace("@VARBUF@", &format!("{:#x}u", var_buf_addr(0)))
        );
        let got = match mb3d::gpu::run_compute(&wgsl, &[&inp, &cst], TRIALS * words, (TRIALS as u32).div_ceil(64)) {
            Ok(v) => v,
            Err(e) => {
                gpu_err += 1;
                println!("GPU ERROR {name}: {}", e.lines().take(6).collect::<Vec<_>>().join(" | "));
                if verbose {
                    std::fs::write(std::env::temp_dir().join(format!("wgsl_{name}.wgsl")), &wgsl).ok();
                }
                continue;
            }
        };
        // compare
        let mut good = 0;
        let mut first_bad = String::new();
        let mut counted = 0;
        for t in 0..TRIALS {
            if !ok_trials[t] {
                continue;
            }
            counted += 1;
            let mut ok = true;
            let mut j = t * words;
            for (c, ty) in &wf.record {
                match ty {
                    CellTy::Whole => {
                        let (a, b) = (f32::from_bits(want[j]), f32::from_bits(got[j]));
                        let close = (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= 1e-3 * a.abs().max(1.0) || (a.is_infinite() && b.is_infinite());
                        if !close {
                            ok = false;
                            if first_bad.is_empty() {
                                first_bad = format!("trial {t} cell r{c}: cpu {a} gpu {b}");
                            }
                        }
                        j += 1;
                    }
                    CellTy::Halves => {
                        // integers exact; doubles kept as bits within single precision
                        let (a, b) = (f64::from_bits(want[j] as u64 | (want[j + 1] as u64) << 32), f64::from_bits(got[j] as u64 | (got[j + 1] as u64) << 32));
                        let close = a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-3 * a.abs().max(1.0) && a.abs() > 1e-300;
                        if (want[j] != got[j] || want[j + 1] != got[j + 1]) && !close {
                            ok = false;
                            if first_bad.is_empty() {
                                first_bad = format!("trial {t} cell r{c}: cpu {:#x}/{:#x} gpu {:#x}/{:#x}", want[j], want[j + 1], got[j], got[j + 1]);
                            }
                        }
                        j += 2;
                    }
                }
            }
            if ok {
                good += 1;
            }
        }
        let frac = good as f64 / counted.max(1) as f64;
        if frac >= 0.95 {
            pass += 1;
            if verbose {
                println!("OK    {name}: {good}/{counted}");
            }
        } else {
            fail += 1;
            println!("FAIL  {name}: {good}/{counted} agree; {first_bad}");
        }
    }
    println!("\n{} formula files: {translated} translated to WGSL: {pass} agree with the interpreter (95% of {TRIALS} inputs), {fail} do not, {gpu_err} shader errors", files.len());
}
