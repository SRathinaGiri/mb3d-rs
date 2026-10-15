//! Developer tool: differential test of the x86 interpreter against native
//! execution of the formula machine code.
//!
//! usage: m3fcheck <formula dir or .m3f files...> [--oracle PATH] [--trials N] [--translated] [-v]
//!        m3fcheck <files...> --bench N --oracle PATH_TO_oracle_bench
//!
//! `--bench N` times N calls of each formula from the same start (the
//! iteration record restored before each call): MB3D's machine code natively
//! (tools/oracle/oracle_bench.s) and the translated code here, in ns per call
//! with the cost of an empty call taken off both.
//!
//! `--translated` compares the translated native code (`native/`) with
//! the interpreter: memory and registers must be identical bit for bit.
//!
//! The oracle is the 32-bit static program built from tools/oracle/oracle.asm
//! (Linux x86/x86-64 with 32-bit support only).  Without an oracle the tool
//! only checks that every formula runs in the interpreter.

use mb3d::custom::*;
use mb3d::m3f::M3f;
use mb3d::x86::{Machine, BASE, MEM_SIZE, RETURN_SENTINEL};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
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

fn setup(m: &mut Machine, rng: &mut Rng, de_option: i32, first_it: i32, slot: usize) {
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
    // dIFS formulas get PVar in edi: the record's field points elsewhere, so
    // that code reading the wrong one shows
    let pv_field = if de_option >= 20 && slot != 0 { pconst_addr(4) } else { pconst_addr(slot) };
    m.wr32(IT_BASE + off::PVAR, pv_field).unwrap();
    w(m, off::ROUT, v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    m.wr32(IT_BASE + off::IT_RESULT, 3).unwrap();
    m.wr32(IT_BASE + off::MAX_IT, 60).unwrap();
    m.wrf32(IT_BASE + off::RSTOP, 1024.0 * 1024.0).unwrap();
    m.wr32(IT_BASE + off::N_HYBRID, 1).unwrap();
    m.wr32(IT_BASE + off::FHPVAR, pconst_addr(slot)).unwrap();
    m.wr32(IT_BASE + off::DE_OPTION, de_option as u32).unwrap();
    w(m, off::OTRAP, 0.5);
    w(m, off::VARY_SCALE, 1.0);
    m.wr32(IT_BASE + off::FIRST_IT, first_it as u32).unwrap();
    w(m, off::DERIV1, 1.0);
    w(m, off::DERIV1 + 8, 0.0);
    w(m, off::DERIV1 + 16, 0.0);
    for i in 0..4u32 {
        m.wrf32(IT_BASE + off::SMATRIX4 + (i * 4 + i) * 4, 1.0).unwrap();
    }
    // header for the oracle
    let x = IT_C1 - 32;
    for (i, v) in [code_addr(0), x, x + 8, x + 16, x + 24, IT_C1, STACK_TOP].iter().enumerate() {
        m.wr32(BASE + 4 * i as u32, *v).unwrap();
    }
}

fn run_native(oracle: &str, mem: &[u8]) -> Option<Vec<u8>> {
    let mut ch = Command::new(oracle)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    ch.stdin.take()?.write_all(mem).ok()?;
    let out = ch.wait_with_output().ok()?;
    if out.status.success() && out.stdout.len() == MEM_SIZE {
        Some(out.stdout)
    } else {
        None
    }
}

fn close(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a == b {
        return true;
    }
    let d = (a - b).abs();
    let s = a.abs().max(b.abs());
    d <= 1e-9 * s.max(1e-3) || (s > 1e100 && d / s < 1e-6)
}

fn main() {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut oracle: Option<String> = None;
    let mut trials = 4;
    let mut verbose = false;
    let mut translated = false;
    let mut bench: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--oracle" => oracle = args.next(),
            "--trials" => trials = args.next().and_then(|s| s.parse().ok()).unwrap_or(4),
            "-v" => verbose = true,
            "--translated" => translated = true,
            "--bench" => bench = args.next().and_then(|s| s.parse().ok()),
            p => {
                let pb = PathBuf::from(p);
                if pb.is_dir() {
                    let mut v: Vec<PathBuf> = std::fs::read_dir(&pb)
                        .unwrap()
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("m3f")).unwrap_or(false))
                        .collect();
                    v.sort();
                    files.extend(v);
                } else {
                    files.push(pb);
                }
            }
        }
    }
    if let Some(n) = bench {
        return run_bench(&files, n, oracle.as_deref());
    }
    let (mut ok, mut mismatch, mut emu_fail, mut skipped, mut native_fail) = (0, 0, 0, 0, 0);
    let mut compiled = 0;
    let mut native = 0;
    for path in &files {
        let def = match M3f::load(path) {
            Ok(d) => Arc::new(d),
            Err(e) => {
                skipped += 1;
                if verbose {
                    println!("SKIP  {e}");
                }
                continue;
            }
        };
        let difs = def.de_option >= 20;
        let cf = CustomFormula::new(def.clone());
        // --translated: in slot 2 (slot dependent addresses must not be baked
        // into the translated code); the oracle runs slot 0
        let slot = if translated { 2 } else { 0 };
        let mut slots: Vec<Option<&CustomFormula>> = vec![None; slot];
        slots.push(Some(&cf));
        let base = build_machine(&slots);
        if base.prog(code_addr(slot)).is_some_and(|p| p.is_native()) {
            native += 1;
        }
        if base.is_compiled(code_addr(slot)) {
            compiled += 1;
        } else if verbose {
            println!("      {}: not compilable, interpreted", def.name);
        }
        let mut rng = Rng(0x1234_5678 ^ def.name.len() as u64 * 7919);
        let mut status = "ok".to_string();
        let mut fail_kind = 0;
        for t in 0..trials {
            let mut m = base.clone();
            setup(&mut m, &mut rng, def.de_option, if t == 0 { 0 } else { t as i32 }, slot);
            if difs {
                // doHybridIFS3D convention (see the oracle header)
                for (o, v) in [(12u32, 1u32), (28, 1), (32, IT_BASE + 144), (36, pconst_addr(slot)), (40, 0)] {
                    m.wr32(BASE + o, v).unwrap();
                }
                m.wrf64(IT_BASE + off::ROLD, 0.01).unwrap();
                m.wrf64(IT_BASE + off::RSTOPD, 0.01).unwrap();
            }
            let start = m.mem.clone();
            let mut em = m.clone();
            let r = if difs {
                em.call_difs(code_addr(slot), IT_BASE + 144, pconst_addr(slot), slot as u32, 1, STACK_TOP, MAX_STEPS)
            } else {
                em.call_formula(code_addr(slot), IT_C1, STACK_TOP, MAX_STEPS)
            };
            if let Err(e) = r {
                status = format!("EMU ERROR {e}");
                fail_kind = 1;
                break;
            }
            if em.reg(4) != STACK_TOP {
                status = format!("EMU stack imbalance esp={:#x}", em.reg(4));
                fail_kind = 1;
                break;
            }
            let _ = RETURN_SENTINEL;
            if translated && base.prog(code_addr(slot)).is_some_and(|p| p.is_native()) {
                // the same start once more in the interpreter
                let mut ei = m.clone();
                mb3d::x86::NATIVE_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
                let r2 = if difs {
                    ei.call_difs(code_addr(slot), IT_BASE + 144, pconst_addr(slot), slot as u32, 1, STACK_TOP, MAX_STEPS)
                } else {
                    ei.call_formula(code_addr(slot), IT_C1, STACK_TOP, MAX_STEPS)
                };
                mb3d::x86::NATIVE_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
                // a direct formula keeps only esp (callers use nothing else)
                let direct = base.prog(code_addr(slot)).is_some_and(|p| p.is_direct());
                let regs_differ = (0..8).any(|r| em.reg(r) != ei.reg(r) && (!direct || r == 4));
                // memory below the final stack pointer is dead (the
                // interpreter pushes the arguments of host calls, e.g. the
                // map functions, that the translated code passes directly)
                let esp = em.reg(4).min(ei.reg(4));
                let dead = |i: usize| {
                    let a = i as u32 + BASE;
                    a < esp && a >= esp.saturating_sub(0x1000)
                };
                let first_mem = (0..MEM_SIZE).find(|&i| em.mem[i] != ei.mem[i] && !dead(i));
                if r2.is_err() || regs_differ || first_mem.is_some() {
                    status = format!(
                        "TRANSLATED DIFFERS trial {t}: interpreter {:?}, registers differ {regs_differ}, first memory difference {:?}",
                        r2.err().map(|e| e.to_string()),
                        first_mem.map(|i| format!("{:#x}", i as u32 + BASE))
                    );
                    fail_kind = 2;
                    break;
                }
            }
            if let Some(o) = &oracle {
                let native = match run_native(o, &start[..MEM_SIZE]) {
                    Some(n) => n,
                    None => {
                        status = "native run failed".into();
                        fail_kind = 3;
                        break;
                    }
                };
                // compare iteration record + variable buffers as doubles
                let mut bad = Vec::new();
                for a in (0x1000..0x4000).step_by(8) {
                    let x = f64::from_le_bytes(em.mem[a..a + 8].try_into().unwrap());
                    let y = f64::from_le_bytes(native[a..a + 8].try_into().unwrap());
                    if em.mem[a..a + 8] != native[a..a + 8] && !close(x, y) {
                        // maybe an integer / single field
                        let xi = u32::from_le_bytes(em.mem[a..a + 4].try_into().unwrap());
                        let yi = u32::from_le_bytes(native[a..a + 4].try_into().unwrap());
                        let xf = f32::from_bits(xi) as f64;
                        let yf = f32::from_bits(yi) as f64;
                        if xi != yi && !(close(xf, yf) && (xf - yf).abs() <= 1e-5 * xf.abs().max(1e-3)) || em.mem[a + 4..a + 8] != native[a + 4..a + 8] && !close(x, y) {
                            bad.push(format!("+{:#x}: emu {x:e} native {y:e}", a - 0x1000));
                        }
                    }
                }
                if !bad.is_empty() {
                    status = format!("MISMATCH trial {t}: {}", bad.iter().take(4).cloned().collect::<Vec<_>>().join("; "));
                    fail_kind = 2;
                    break;
                }
            }
        }
        match fail_kind {
            0 => ok += 1,
            1 => emu_fail += 1,
            2 => mismatch += 1,
            _ => native_fail += 1,
        }
        if fail_kind != 0 || verbose {
            println!("{:5} {}: {status}", if fail_kind == 0 { "OK" } else { "FAIL" }, def.name);
        }
    }
    println!("{compiled} formulas run compiled ({native} as translated native code), the others interpreted");
    println!(
        "\n{} formulas: {ok} ok, {mismatch} mismatch, {emu_fail} interpreter errors, {native_fail} native failures, {skipped} skipped",
        files.len()
    );
}

/// `--bench`: nanoseconds per call of each formula, natively (MB3D's code
/// through the timing oracle) and translated (here), from the same start.
fn run_bench(files: &[PathBuf], n: u64, oracle: Option<&str>) {
    // the record restored before each call (both sides)
    let (r1, l1, r2, l2) = (IT_BASE, 36u32, IT_BASE + 144, 76u32);
    let header = |m: &mut Machine, entry: u32| {
        m.wr32(BASE, entry).unwrap();
        for (o, v) in [(44u32, n as u32), (48, r1), (52, l1), (56, r2), (60, l2)] {
            m.wr32(BASE + o, v).unwrap();
        }
    };
    let native_ns = |m: &Machine| -> Option<f64> {
        let out = run_native(oracle?, &m.mem[..MEM_SIZE])?;
        let ns = u64::from_le_bytes(out[64..72].try_into().unwrap());
        Some(ns as f64 / n as f64)
    };
    println!("{:<28} {:>10} {:>10} {:>7}", "formula", "MB3D ns", "ours ns", "ratio");
    let mut empty: Option<(f64, f64)> = None;
    for path in files {
        let Ok(def) = M3f::load(path) else { continue };
        let def = Arc::new(def);
        let difs = def.de_option >= 20;
        let cf = CustomFormula::new(def.clone());
        let base = build_machine(&[Some(&cf)]);
        let mut rng = Rng(0x1234_5678 ^ def.name.len() as u64 * 7919);
        let mut m = base.clone();
        setup(&mut m, &mut rng, def.de_option, 1, 0);
        if difs {
            for (o, v) in [(12u32, 1u32), (28, 1), (32, IT_BASE + 144), (36, pconst_addr(0)), (40, 0)] {
                m.wr32(BASE + o, v).unwrap();
            }
            m.wrf64(IT_BASE + off::ROLD, 0.01).unwrap();
            m.wrf64(IT_BASE + off::RSTOPD, 0.01).unwrap();
        }
        // the cost of an empty call (a `ret` in slot 5's code), once
        if empty.is_none() {
            let mut e = m.clone();
            e.mem[(code_addr(5) - BASE) as usize] = 0xC3;
            e.wr32(BASE + 28, 0).unwrap();
            header(&mut e, code_addr(5));
            let nat = native_ns(&e).unwrap_or(0.0);
            let t = std::time::Instant::now();
            let saved = e.mem.clone();
            for _ in 0..n {
                restore(&mut e, &saved[..], r1, l1, r2, l2);
            }
            empty = Some((nat, t.elapsed().as_nanos() as f64 / n as f64));
        }
        let (e_nat, e_ours) = empty.unwrap();
        header(&mut m, code_addr(0));
        let nat = native_ns(&m).map(|v| v - e_nat);
        // ours: the same calls through the machine (translated code)
        let saved = m.mem.clone();
        let mut em = m.clone();
        let t = std::time::Instant::now();
        let mut ok = true;
        for _ in 0..n {
            restore(&mut em, &saved[..], r1, l1, r2, l2);
            let r = if difs {
                em.call_difs(code_addr(0), IT_BASE + 144, pconst_addr(0), 0, 1, STACK_TOP, MAX_STEPS)
            } else {
                em.call_formula(code_addr(0), IT_C1, STACK_TOP, MAX_STEPS)
            };
            ok &= r.is_ok();
        }
        let mut ours = t.elapsed().as_nanos() as f64 / n as f64 - e_ours;
        // as the render calls it: the direct version on the iteration record
        let rd = |o: u32| f64::from_le_bytes(saved[(IT_BASE + o - BASE) as usize..][..8].try_into().unwrap());
        let mut it0 = mb3d::iteration::Iteration::default();
        for k in 0..4 {
            it0.v[k] = rd(off::X + 8 * k as u32);
        }
        for k in 0..3 {
            it0.c[k] = rd(off::C1 + 8 * k as u32);
            it0.j[k] = rd(off::J1 + 8 * k as u32);
            it0.ju[k] = rd(off::JU1 + 8 * k as u32);
        }
        it0.j[3] = rd(off::J4);
        it0.rold = rd(off::ROLD);
        it0.rout = rd(off::ROUT);
        it0.otrap = rd(off::OTRAP);
        it0.vary_scale = rd(off::VARY_SCALE);
        it0.deriv1 = 1.0;
        it0.it_result = 3;
        it0.max_it = 60;
        it0.rstop = 1024.0 * 1024.0;
        it0.first_it = 1;
        it0.de_option = def.de_option;
        let mut dm = m.clone();
        let mut it = it0.clone();
        if !difs && dm.call_direct(code_addr(0), &mut it, pconst_addr(0), MAX_STEPS).is_some() {
            let t = std::time::Instant::now();
            for _ in 0..n {
                it.clone_from(&it0);
                let _ = dm.call_direct(code_addr(0), &mut it, pconst_addr(0), MAX_STEPS);
            }
            ours = t.elapsed().as_nanos() as f64 / n as f64;
            // (the copy of the record taken off)
            let t = std::time::Instant::now();
            for _ in 0..n {
                it.clone_from(&it0);
                std::hint::black_box(&mut it);
            }
            ours -= t.elapsed().as_nanos() as f64 / n as f64;
        }
        let kind = if base.prog(code_addr(0)).is_some_and(|p| p.is_native()) { "" } else { " (interpreted)" };
        match nat {
            Some(nat) => println!("{:<28} {:>10.1} {:>10.1} {:>7.2}{kind}{}", def.name, nat, ours, ours / nat.max(0.1), if ok { "" } else { " ERROR" }),
            None => println!("{:<28} {:>10} {:>10.1}{kind}", def.name, "-", ours),
        }
    }
}

fn restore(m: &mut Machine, saved: &[u8], r1: u32, l1: u32, r2: u32, l2: u32) {
    for (a, l) in [(r1, l1), (r2, l2)] {
        let (a, l) = ((a - BASE) as usize, l as usize * 4);
        m.mem[a..a + l].copy_from_slice(&saved[a..a + l]);
    }
}
