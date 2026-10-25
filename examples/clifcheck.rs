//! The formulas compiled with Cranelift (`x86::lift_clif`) against their
//! translated direct versions: both run from the same random states (the
//! stack below the call full of garbage); the iteration fields and the
//! memory image must be identical bit for bit.  Also says which formulas
//! cannot be compiled and why.
//! `cargo run --release --example clifcheck -- <M3Formulas dir> [--trials N] [--builtins] [-v]`
//! (not on macOS: no JIT there yet)
fn main() {
    #[cfg(jit)]
    imp::main();
    #[cfg(not(jit))]
    eprintln!("clifcheck: this build has no formulas compiled at run time (macOS, or without the feature \"jit\")");
}

#[cfg(jit)]
mod imp {
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

    pub fn main() {
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
                difs_one(&mut jit, &def, trials, &mut reasons, &mut same, &mut differ, &mut direct);
                continue;
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
                // the SSE registers a formula may find from the one before
                for x in m.xmm.iter_mut() {
                    *x = [rng.range(-3.0, 3.0).to_bits(), rng.range(-3.0, 3.0).to_bits()];
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
                let canon = |x: u64| if std::env::var_os("CANON_NAN").is_some() && f64::from_bits(x).is_nan() { u64::MAX } else { x };
                let xmm_diff = (0..8).find(|&r| m.xmm[r].map(canon) != m2.xmm[r].map(canon));
                // NaN results compare by their bits (both sides run the same
                // operations, so even the NaN payloads agree)
                if ok1 != (r2 == 0) || (ok1 && (fields(&it) != fields(&it2) || mem_diff.is_some() || xmm_diff.is_some())) {
                    let fd: Vec<usize> = fields(&it).iter().zip(fields(&it2)).enumerate().filter(|(_, (a, b))| *a != b).map(|(i, _)| i).collect();
                    status = Some(format!("trial {t}: translated {r1:?}, compiled {r2}, fields differ {fd:?}, memory differs at {mem_diff:?}, xmm differs {xmm_diff:?}"));
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
        println!("{direct} formulas (direct and dIFS): {same} compiled and identical, {differ} differ; not compiled:");
        for (k, v) in reasons {
            println!("  {v:4} {k}");
        }
    }

    /// A dIFS formula compiled on its own against its translated version
    /// (`Machine::call_difs`): registers, memory (above the stack) and SSE
    /// registers.
    fn difs_one(jit: &mut Jit, def: &Arc<M3f>, trials: usize, reasons: &mut BTreeMap<String, usize>, same: &mut usize, differ: &mut usize, count: &mut usize) {
        let slot = 2;
        let cf = CustomFormula::new(def.clone());
        let mut slots: Vec<Option<&CustomFormula>> = vec![None; slot];
        slots.push(Some(&cf));
        let base = build_machine(&slots);
        let Some(prog) = base.prog(code_addr(slot)) else { return };
        if !prog.is_native() {
            return;
        }
        *count += 1;
        if let Some(why) = prog.clif_unsupported_mode(true) {
            *reasons.entry(format!("dIFS: {}", why.split(':').next().unwrap_or(""))).or_insert(0) += 1;
            return;
        }
        let Some(f) = jit.compile_difs_one(&base, code_addr(slot), pconst_addr(slot)) else {
            *reasons.entry("dIFS compile failed".into()).or_insert(0) += 1;
            println!("COMPILE FAILED {}", def.name);
            return;
        };
        let mut rng = Rng(0x9e37 ^ (def.name.len() as u64 * 7919));
        let canon = |x: u64| if std::env::var_os("CANON_NAN").is_some() && f64::from_bits(x).is_nan() { u64::MAX } else { x };
        for t in 0..trials {
            let mut m = base.clone();
            for a in (STACK_TOP - 0x1000..STACK_TOP).step_by(4) {
                m.wr32(a, rng.range(0.0, 4294967295.0) as u32).unwrap();
            }
            for x in m.xmm.iter_mut() {
                *x = [rng.range(-3.0, 3.0).to_bits(), rng.range(-3.0, 3.0).to_bits()];
            }
            // the record as doHybridIFS3D leaves it (as m3fcheck)
            let w = |m: &mut mb3d::x86::Machine, o: u32, v: f64| m.wrf64(IT_BASE + o, v).unwrap();
            let c = [rng.range(-1.2, 1.2), rng.range(-1.2, 1.2), rng.range(-1.2, 1.2)];
            for (k, ck) in c.iter().enumerate() {
                w(&mut m, off::X + 8 * k as u32, rng.range(-1.5, 1.5));
                w(&mut m, off::C1 + 8 * k as u32, *ck);
                w(&mut m, off::J1 + 8 * k as u32, *ck);
            }
            w(&mut m, off::ROLD, 0.01);
            w(&mut m, off::RSTOPD, 0.01);
            w(&mut m, off::VARY_SCALE, 1.0);
            w(&mut m, off::DERIV1, 1.0);
            m.wr32(IT_BASE + off::MAX_IT, 60).unwrap();
            m.wr32(IT_BASE + off::IT_RESULT, t as u32 % 5).unwrap();
            for (o, v) in [(12u32, 1u32), (28, 1), (32, IT_BASE + 144), (36, pconst_addr(slot)), (40, 0)] {
                m.wr32(mb3d::x86::BASE + o, v).unwrap();
            }
            let mut m2 = m.clone();
            let r1 = m.call_difs(code_addr(slot), IT_BASE + 144, pconst_addr(slot), slot as u32, 1, STACK_TOP, MAX_STEPS);
            let mut regs = [0u32; 8];
            regs[6] = IT_BASE + 144;
            regs[7] = pconst_addr(slot);
            regs[3] = slot as u32;
            regs[1] = 1;
            regs[2] = IT_BASE + 144 + 128;
            regs[4] = STACK_TOP - 4;
            // SAFETY: the compiled function takes these pointers
            let r2 = unsafe { f(&mut regs, m2.mem.as_mut_ptr(), &m2, MAX_STEPS) };
            // the stack below the top is dead (the call pushes there)
            let dead = |i: usize| {
                let a = i as u32 + mb3d::x86::BASE;
                (STACK_TOP - 0x1000..STACK_TOP).contains(&a)
            };
            let mem_diff = (0..m.mem.len()).find(|&i| m.mem[i] != m2.mem[i] && !dead(i));
            let mem_diff = mem_diff.filter(|&i| {
                // NaN payloads (CANON_NAN): compare the 8-byte cell as doubles
                let a = i & !7;
                let x = u64::from_le_bytes(m.mem[a..a + 8].try_into().unwrap());
                let y = u64::from_le_bytes(m2.mem[a..a + 8].try_into().unwrap());
                canon(x) != canon(y)
            });
            let regs_out: Vec<u32> = (0..8).map(|r| m.reg(r)).collect();
            // esp: the native call returns with the stack pointer at the top
            let regs_ok = (0..8).all(|r| r == 4 || regs_out[r] == regs[r]);
            let xmm_diff = (0..8).find(|&r| m.xmm[r].map(canon) != m2.xmm[r].map(canon));
            let ok1 = r1.is_ok();
            if ok1 != (r2 == 0) || (ok1 && (mem_diff.is_some() || !regs_ok || xmm_diff.is_some())) {
                *differ += 1;
                println!("DIFF  {} (dIFS): trial {t}: translated {r1:?}, compiled {r2}, memory differs at {mem_diff:?}, registers {regs_out:x?} vs {regs:x?}, xmm {xmm_diff:?}", def.name);
                return;
            }
        }
        *same += 1;
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
        for (power, z_mul) in [(8.0, -1.0), (3.5, 1.0), (-2.2, 0.7)] {
            fs.push(Formula::RealPower { power, z_mul });
        }
        for (r_threshold, r_threshold2) in [(1.0, 0.5), (2.0, 1.2), (0.6, 0.6)] {
            fs.push(Formula::Bulbox { scale: 2.0, min_r: 0.5, fold: 1.0, bulb_scale: 1.0, r_threshold, r_threshold2 });
            fs.push(Formula::Bulbox { scale: -1.7, min_r: 0.3, fold: 0.8, bulb_scale: 1.4, r_threshold, r_threshold2 });
        }
        if let Some(Formula::AexionC { power, z_mul, power_c, cz_mul, .. }) = Formula::default_for("Aexion C") {
            for mode in 0..32 {
                for (rot_c, cond_phi, powc_dist) in [(false, false, false), (true, false, false), (true, true, false), (true, true, true), (true, false, true)] {
                    fs.push(Formula::AexionC { power, z_mul, rot_c, cond_phi, power_c, cz_mul, powc_dist, mode });
                }
            }
        }
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
}
