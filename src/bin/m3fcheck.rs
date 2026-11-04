//! Developer tool: checks the x86 interpreter and the translated formula
//! code on the formulas' machine code.
//!
//! usage: m3fcheck <formula dir or .m3f files...> [--trials N] [--translated] [-v]
//!
//! Every formula must run in the interpreter.  `--translated` also compares
//! the translated native code (`native/`) with the interpreter: memory and
//! registers must be identical bit for bit.

use mb3d::custom::*;
use mb3d::m3f::M3f;
use mb3d::x86::{Machine, BASE, MEM_SIZE, RETURN_SENTINEL};
use std::path::PathBuf;
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
    // garbage below the call's stack: a formula must not read stack memory
    // it did not write (the translated code keeps its stack in locals)
    for a in (STACK_TOP - 0x1000..STACK_TOP).step_by(4) {
        m.wr32(a, (rng.range(0.0, 4294967295.0)) as u32).unwrap();
    }
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
}

fn main() {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut trials = 4;
    let mut verbose = false;
    let mut translated = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--trials" => trials = args.next().and_then(|s| s.parse().ok()).unwrap_or(4),
            "-v" => verbose = true,
            "--translated" => translated = true,
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
    let (mut ok, mut mismatch, mut emu_fail, mut skipped) = (0, 0, 0, 0);
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
        // into the translated code)
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
                // doHybridIFS3D's values
                for (o, v) in [(12u32, 1u32), (28, 1), (32, IT_BASE + 144), (36, pconst_addr(slot)), (40, 0)] {
                    m.wr32(BASE + o, v).unwrap();
                }
                m.wrf64(IT_BASE + off::ROLD, 0.01).unwrap();
                m.wrf64(IT_BASE + off::RSTOPD, 0.01).unwrap();
            }
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
        }
        match fail_kind {
            0 => ok += 1,
            1 => emu_fail += 1,
            _ => mismatch += 1,
        }
        if fail_kind != 0 || verbose {
            println!("{:5} {}: {status}", if fail_kind == 0 { "OK" } else { "FAIL" }, def.name);
        }
    }
    println!("{compiled} formulas run compiled ({native} as translated native code), the others interpreted");
    println!(
        "\n{} formulas: {ok} ok, {mismatch} mismatch, {emu_fail} interpreter errors, {skipped} skipped",
        files.len()
    );
}
