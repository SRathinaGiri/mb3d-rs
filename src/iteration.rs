//! The iteration record (`TIteration3Dext`) and the hybrid iteration loops
//! (`doHybridPas`, `doHybridPasDE`, `doHybrid4DPas`) from `formulas.pas`,
//! plus `CalcSmoothIterations`.

use crate::custom::{self, off};
use crate::formulas::Formula;
use crate::x86::Machine;
use std::sync::atomic::{AtomicU64, Ordering};

/// Number of custom formula calls that failed in the interpreter.
pub static CUSTOM_ERRORS: AtomicU64 = AtomicU64::new(0);
use crate::math::{rotate_4dex, Mat4, Vec3};

/// One slot of the alternating hybrid (`nHybrid[n]`, `fHybrid[n]`).
#[derive(Clone, Debug)]
pub struct HybridSlot {
    pub formula: Formula,
    /// Number of consecutive iterations of this formula (`iItCount`).
    pub iterations: i32,
    /// If true, the iterations are not counted against max iterations and do
    /// not update `Rout` (negative iteration count / high bit in MB3D).
    pub uncounted: bool,
    /// Use the analytic-DE variant of the formula (derivative in `w`).
    pub ade: bool,
}

/// How the iteration loop is evaluated (`mMandFunction`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HybridMode {
    /// 3D alternating hybrid (`doHybridPas`)
    Alt3D,
    /// 4D alternating hybrid (`doHybrid4DPas`)
    Alt4D,
    /// interpolation of two formulas (`doInterpolHybridPas`)
    Ipol3D,
    /// 4D interpolation hybrid (`doInterpolHybridPas4D`)
    Ipol4D,
}

/// The per-thread iteration state, a subset of `TIteration3Dext`.
#[derive(Clone, Debug, Default)]
pub struct Iteration {
    /// Iterated vector x, y, z, w.
    pub v: [f64; 4],
    /// Start position (`C1..C3`).
    pub c: Vec3,
    /// Constant added in each iteration (`J1..J3`, `J4`).
    pub j: [f64; 4],
    /// Julia seed (`Ju1..Ju4`).
    pub ju: [f64; 4],
    pub do_julia: bool,
    pub rout: f64,
    pub rold: f64,
    pub otrap: f64,
    pub it_result: i32,
    pub max_it: i32,
    /// Squared bailout radius (`RStop`, kept in single precision like MB3D).
    pub rstop: f64,
    pub calc_sit: bool,
    pub smooth_it: f32,
    /// `LNRStop` = ln(ln(RStop))  (single precision)
    pub ln_rstop: f32,
    pub de_option: i32,
    pub deriv1: f64,
    pub deriv2: f64,
    pub deriv3: f64,
    pub smatrix4: Mat4,
    pub start_from: usize,
    pub end_to: usize,
    pub repeat_from: usize,
    /// `fHln[n]` = 1 / ln(power) for the smooth iteration count.
    pub fhln: [f32; 6],
    pub vary_scale: f64,
    pub first_it: i32,
    pub dfree: [f64; 2],
    /// Interpreter for custom (.m3f) formulas, if the hybrid uses any.
    pub emu: Option<Box<Machine>>,
    /// Weights of the two formulas of an interpolation hybrid
    /// (`PSingle(@nHybrid[0..1])^`).
    pub ipol: [f32; 2],
}

impl Iteration {
    /// `CalcSmoothIterations`
    #[inline]
    fn calc_smooth_iterations(&mut self, n: usize) {
        if self.rout <= 1.0 {
            self.smooth_it = self.it_result as f32;
        } else if self.rold < 1.0 {
            let d = (0.5 * self.rout.ln()).ln() * self.fhln[n] as f64;
            self.smooth_it = (self.it_result as f64 + self.ln_rstop as f64 - d) as f32;
        } else {
            let d = (0.5 * self.rout.ln()).ln();
            let e = d - (0.5 * self.rold.ln()).ln();
            self.smooth_it =
                ((self.ln_rstop as f64 - d) / (e + 1e-100) + self.it_result as f64) as f32;
        }
    }

    /// The core alternating hybrid loop shared by all variants.  Returns the
    /// index of the last formula used.
    #[inline]
    fn run(&mut self, slots: &[HybridSlot], four_d: bool) -> usize {
        let mut n = self.start_from;
        let mut btmp = slots[n].iterations.max(0);
        self.it_result = 0;
        self.first_it = 0;
        // guard against a hybrid that never counts an iteration
        let mut uncounted_guard = 0u32;
        loop {
            self.rold = self.rout;
            let mut guard = 0;
            while btmp <= 0 {
                n += 1;
                if n > self.end_to {
                    n = self.repeat_from;
                }
                btmp = slots[n].iterations.max(0);
                guard += 1;
                if guard > 64 {
                    return n; // no formula with iterations > 0
                }
            }
            let slot = &slots[n];
            self.call_slot(slots, n);
            btmp -= 1;
            if slot.uncounted {
                uncounted_guard += 1;
                if uncounted_guard > 100_000 {
                    return n;
                }
                continue;
            }
            self.it_result += 1;
            let [x, y, z, w] = self.v;
            self.rout = if four_d { x * x + y * y + z * z + w * w } else { x * x + y * y + z * z };
            if self.rout < self.otrap {
                self.otrap = self.rout;
            }
            // NaN compares false like the x87 code -> keeps iterating
            if self.it_result >= self.max_it || self.rout > self.rstop {
                return n;
            }
        }
    }

    /// `fHybrid[n](x, y, z, w, PIteration3D)` with `PVar = fHPVar[n]`.
    #[inline]
    fn call_slot(&mut self, slots: &[HybridSlot], n: usize) {
        let slot = &slots[n];
        if let Formula::Custom(c) = &slot.formula {
            match &c.def.jit {
                Some(p) => p.run(self, &c.def, &c.values),
                None => self.run_custom(n),
            }
        } else {
            slot.formula.iterate(&mut self.v, &mut self.j, self.rout, slot.ade);
        }
    }

    /// The common loop of the interpolation hybrids: both formulas are
    /// applied to the same vector, the results (and their lengths) are
    /// blended with the weights.  `de`: blend the derivative (`w` in 3D,
    /// `Deriv1` in 4D) of the analytic-DE variants.
    fn run_ipol(&mut self, slots: &[HybridSlot], four_d: bool, de: bool) {
        let (s1, s2) = (self.ipol[0] as f64, self.ipol[1] as f64);
        self.first_it = 0;
        self.it_result = 0;
        loop {
            self.rold = self.rout;
            let y1 = self.v;
            let dt = self.deriv1;
            self.call_slot(slots, 0);
            let x1 = self.v;
            self.v = y1;
            let dd = self.deriv1;
            if four_d && de {
                self.deriv1 = dt;
            }
            self.call_slot(slots, 1);
            let [x, y, z, w] = self.v;
            let (xx, yy) = if four_d {
                ((x1[0] * x1[0] + x1[1] * x1[1] + x1[2] * x1[2] + x1[3] * x1[3]).sqrt(), (x * x + y * y + z * z + w * w).sqrt())
            } else {
                ((x1[0] * x1[0] + x1[1] * x1[1] + x1[2] * x1[2]).sqrt(), (x * x + y * y + z * z).sqrt())
            };
            let xx = xx * s1 + yy * s2;
            let mut v = [x1[0] * s1 + x * s2, x1[1] * s1 + y * s2, x1[2] * s1 + z * s2, x1[3] * s1 + w * s2];
            if de && !four_d {
                v[3] = x1[3].abs() * s1 + w.abs() * s2;
            }
            if de && four_d {
                self.deriv1 = dd.abs() * s1 + self.deriv1.abs() * s2;
            }
            let l2 = if four_d { v[0] * v[0] + v[1] * v[1] + v[2] * v[2] + v[3] * v[3] } else { v[0] * v[0] + v[1] * v[1] + v[2] * v[2] };
            let yy = xx / (l2 + 1e-40).sqrt();
            v[0] *= yy;
            v[1] *= yy;
            v[2] *= yy;
            if four_d {
                v[3] *= yy;
            }
            self.v = v;
            self.it_result += 1;
            self.rout = xx * xx;
            if self.rout < self.otrap {
                self.otrap = self.rout;
            }
            if self.it_result >= self.max_it || self.rout > self.rstop {
                break;
            }
        }
    }

    fn ipol_start_3d(&mut self) {
        let c = self.c;
        if self.do_julia {
            self.j = [self.ju[0], self.ju[1], self.ju[2], self.j[3]];
        } else {
            self.j = [c[0], c[1], c[2], self.j[3]];
        }
        self.v[0] = c[0];
        self.v[1] = c[1];
        self.v[2] = c[2];
        self.rout = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
        self.otrap = self.rout;
    }

    fn start_4d(&mut self) {
        let v4 = rotate_4dex(&self.c, &self.smatrix4);
        self.v = v4;
        if self.do_julia {
            self.j = self.ju;
        } else {
            self.j = v4;
        }
        let [x, y, z, w] = v4;
        self.rout = x * x + y * y + z * z + w * w;
        self.otrap = self.rout;
    }

    /// `doInterpolHybridPas`
    pub fn ipol_3d(&mut self, slots: &[HybridSlot]) {
        self.ipol_start_3d();
        self.v[3] = 0.0;
        self.run_ipol(slots, false, false);
        if self.calc_sit {
            self.calc_smooth_iterations(0);
        }
    }

    /// `doInterpolHybridPasDE`
    pub fn ipol_3d_de(&mut self, slots: &[HybridSlot]) -> f64 {
        self.ipol_start_3d();
        self.v[3] = if (self.de_option & 0x18) == 16 { self.rout } else { 1.0 };
        self.run_ipol(slots, false, true);
        let w = self.v[3];
        let r = match self.de_option & 7 {
            4 => self.v[2].abs() * self.v[2].abs().ln() / w,
            _ => self.rout.sqrt() / w.abs(),
        };
        if self.calc_sit {
            self.calc_smooth_iterations(0);
        }
        r
    }

    /// `doInterpolHybridPas4D`
    pub fn ipol_4d(&mut self, slots: &[HybridSlot]) {
        self.start_4d();
        self.run_ipol(slots, true, false);
        if self.calc_sit {
            self.calc_smooth_iterations(0);
        }
    }

    /// `doInterpolHybridPas4DDE`
    pub fn ipol_4d_de(&mut self, slots: &[HybridSlot]) -> f64 {
        self.start_4d();
        self.init_deriv_4d();
        self.run_ipol(slots, true, true);
        let r = self.de_4d_result();
        if self.calc_sit {
            self.calc_smooth_iterations(0);
        }
        r
    }

    fn init_deriv_4d(&mut self) {
        match self.de_option & 0x38 {
            16 => self.deriv1 = self.rout,
            32 => {
                self.deriv1 = 1.0;
                self.deriv2 = 0.0;
                self.deriv3 = 0.0;
            }
            _ => self.deriv1 = 1.0,
        }
    }

    fn de_4d_result(&self) -> f64 {
        match self.de_option & 7 {
            4 => self.v[2].abs() * self.v[2].abs().ln() / self.deriv1,
            _ => self.rout.sqrt() / self.deriv1.abs(),
        }
    }

    /// `doHybrid4DDEPas` – 4D alternating hybrid with analytic DE (the
    /// derivative is kept in `Deriv1`).
    pub fn hybrid_4d_de(&mut self, slots: &[HybridSlot]) -> f64 {
        self.start_4d();
        self.init_deriv_4d();
        let n = self.run(slots, true);
        let r = self.de_4d_result();
        if self.calc_sit {
            self.calc_smooth_iterations(n);
        }
        r
    }

    /// `mMandFunctionDE` for the analytic-DE modes.
    #[inline]
    pub fn mand_function_de(&mut self, mode: HybridMode, slots: &[HybridSlot]) -> f64 {
        match mode {
            HybridMode::Alt3D => self.hybrid_3d_de(slots),
            HybridMode::Alt4D => self.hybrid_4d_de(slots),
            HybridMode::Ipol3D => self.ipol_3d_de(slots),
            HybridMode::Ipol4D => self.ipol_4d_de(slots),
        }
    }

    /// `doHybridPas` – 3D alternating hybrid without analytic DE.
    pub fn hybrid_3d(&mut self, slots: &[HybridSlot]) {
        let c = self.c;
        if self.do_julia {
            self.j = [self.ju[0], self.ju[1], self.ju[2], self.j[3]];
        } else {
            self.j = [c[0], c[1], c[2], self.j[3]];
        }
        self.v = [c[0], c[1], c[2], 0.0];
        self.rout = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
        self.otrap = self.rout;
        let n = self.run(slots, false);
        if self.calc_sit {
            self.calc_smooth_iterations(n);
        }
    }

    /// `doHybridPasDE` – 3D alternating hybrid with analytic DE, the running
    /// derivative is kept in `w`.  Returns the distance estimate.
    pub fn hybrid_3d_de(&mut self, slots: &[HybridSlot]) -> f64 {
        let c = self.c;
        if self.do_julia {
            self.j = [self.ju[0], self.ju[1], self.ju[2], self.j[3]];
        } else {
            self.j = [c[0], c[1], c[2], self.j[3]];
        }
        self.v[0] = c[0];
        self.v[1] = c[1];
        self.v[2] = c[2];
        self.rout = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
        self.otrap = self.rout;
        match self.de_option & 0x38 {
            16 => self.v[3] = self.rout,
            32 => {
                self.deriv1 = 1.0;
                self.deriv2 = 0.0;
                self.deriv3 = 0.0;
            }
            _ => self.v[3] = 1.0,
        }
        let n = self.run(slots, false);
        let w = self.v[3];
        let result = if (self.de_option & 0x38) == 32 {
            self.rout.sqrt() * 0.5 * self.rout.ln() / self.deriv1
        } else {
            match self.de_option & 7 {
                4 => self.v[1].abs() * self.v[1].abs().ln() / w,
                _ => self.rout.sqrt() / w.abs(),
            }
        };
        if self.calc_sit {
            self.calc_smooth_iterations(n);
        }
        result
    }

    /// `doHybridIFS3D` – the dIFS hybrid loop.  All slots are custom
    /// formulas with the dIFS calling convention; each counted iteration
    /// yields a distance estimate in `Rout` relative to `VaryScale`.  Returns
    /// the smallest estimate; `it_result` / `smooth_it` hold the iteration
    /// where it occurred and `otrap` the colour value stored by the formula.
    /// `rstopd` is the absolute DE stop (`RStopD`).
    /// `vec_ini = false` is `doHybridIFS3DnoVecIni`: the iterated vector of a
    /// preceding hybrid part is used as the start.
    pub fn hybrid_difs(&mut self, slots: &[HybridSlot], rstopd: f64, inside: bool, vec_ini: bool) -> f64 {
        let c = self.c;
        if vec_ini {
            if self.do_julia {
                self.j = [self.ju[0], self.ju[1], self.ju[2], self.j[3]];
            } else {
                self.j = [c[0], c[1], c[2], self.j[3]];
            }
        }
        let Some(m) = self.emu.as_deref_mut() else {
            return 0.0;
        };
        let b = custom::IT_BASE;
        let esi = b + 144;
        for k in 0..3 {
            m.put_f64(b + off::X + 8 * k as u32, if vec_ini { c[k] } else { self.v[k] });
            m.put_f64(b + off::C1 + 8 * k as u32, c[k]);
            m.put_f64(b + off::J1 + 8 * k as u32, self.j[k]);
        }
        m.put_u32(b + off::BTMP, 0);
        m.put_f64(b + off::J4, self.j[3]);
        m.put_f64(b + off::ROLD, rstopd);
        m.put_f64(b + off::RSTOPD, rstopd);
        m.put_u32(b + off::MAX_IT, self.max_it as u32);
        m.put_u32(b + off::RSTOP, (self.rstop as f32).to_bits());
        m.put_u32(b + off::INSIDE_RENDER, if inside { u32::MAX } else { 0 });
        m.put_u32(b + off::SMOOTH_IT, if inside { u32::MAX } else { 0 });
        m.put_u32(b + off::FIRST_IT, 0);
        m.put_u32(b + off::IT_RESULT, 0);
        m.put_f64(b + off::VARY_SCALE, 1.0);
        m.put_f64(b + off::DFREE1, 0.0);
        m.put_f64(b + off::DERIV1, 1.0);
        let mut min_de = 65535.0f64;
        let mut dfree2 = 0.0;
        let mut it_min = 0i32;
        let mut it = 0i32;
        // registers as in doHybridIFS3D: ebx = slot, ecx = remaining count,
        // edi = variable buffer; the formula's values of these registers are
        // used after each call (some formulas change ecx)
        let mut n = self.start_from.min(slots.len() - 1);
        let mut cnt = slots[n].iterations.max(0);
        let mut edi = custom::pconst_addr(n);
        let mut guard = 0u32;
        let mut failed = false;
        'outer: loop {
            // @Repeat / @While: next slot with iterations
            let mut g = 0;
            while cnt <= 0 {
                n += 1;
                if n > self.end_to {
                    n = self.repeat_from;
                }
                cnt = slots[n].iterations.max(0);
                edi = custom::pconst_addr(n);
                g += 1;
                if g > 64 {
                    break 'outer;
                }
            }
            let entry = custom::code_addr(n);
            if m.call_difs(entry, esi, edi, n as u32, cnt as u32, custom::STACK_TOP, custom::MAX_STEPS).is_err() {
                failed = true;
                break;
            }
            cnt = m.reg(1) as i32 - 1;
            let nb = m.reg(3) as usize;
            if nb < slots.len() {
                n = nb;
            }
            edi = m.reg(7);
            guard += 1;
            if guard > 1_000_000 {
                break;
            }
            if slots[n].uncounted {
                continue;
            }
            it += 1;
            m.put_u32(b + off::IT_RESULT, it as u32);
            let de = m.get_f64(b + off::ROUT) / m.get_f64(b + off::VARY_SCALE);
            if de < min_de {
                it_min = it;
                dfree2 = m.get_f64(b + off::DFREE1);
                min_de = de;
                if !inside && de < rstopd {
                    break;
                }
            }
            if it >= self.max_it {
                break;
            }
        }
        if failed {
            CUSTOM_ERRORS.fetch_add(1, Ordering::Relaxed);
            min_de = f64::INFINITY;
        }
        for k in 0..4 {
            self.v[k] = m.get_f64(b + off::X + 8 * k as u32);
        }
        self.rout = m.get_f64(b + off::ROUT);
        self.it_result = it_min;
        self.smooth_it = it_min as f32;
        self.otrap = dfree2;
        min_de
    }

    /// `doHybrid4DPas` – 4D alternating hybrid (quaternion etc.).
    pub fn hybrid_4d(&mut self, slots: &[HybridSlot]) {
        let v4 = rotate_4dex(&self.c, &self.smatrix4);
        self.v = v4;
        if self.do_julia {
            self.j = self.ju;
        } else {
            self.j = v4;
        }
        let [x, y, z, w] = v4;
        self.rout = x * x + y * y + z * z + w * w;
        self.otrap = self.rout;
        let n = self.run(slots, true);
        if self.calc_sit {
            self.calc_smooth_iterations(n);
        }
    }

    /// One iteration of the custom formula in hybrid slot `n`: the iteration
    /// record is mirrored into the emulated memory, the machine code is run
    /// and the fields a formula may change are read back.
    fn run_custom(&mut self, n: usize) {
        let Some(m) = self.emu.as_deref_mut() else {
            self.rout = f64::INFINITY;
            return;
        };
        let b = custom::IT_BASE;
        let code = custom::code_addr(n);
        // a translated formula touches only the record cells it names, the
        // others need not be copied (all cells for the interpreter)
        let (rd, wr) = m.formula_fields(code).unwrap_or((u64::MAX, u64::MAX));
        let has = |mask: u64, o: u32| mask & (1 << (o / 8)) != 0;
        // the iteration record lies at fixed addresses inside the image,
        // so the unchecked accessors are used here
        for k in 0..4 {
            let o = off::X + 8 * k as u32;
            if has(rd, o) {
                m.put_f64(b + o, self.v[k]);
            }
        }
        for k in 0..3 {
            if has(rd, off::C1 + 8 * k as u32) {
                m.put_f64(b + off::C1 + 8 * k as u32, self.c[k]);
            }
            if has(rd, off::J1 + 8 * k as u32) {
                m.put_f64(b + off::J1 + 8 * k as u32, self.j[k]);
            }
        }
        let f64s: [(u32, f64); 10] = [
            (off::J4, self.j[3]),
            (off::ROLD, self.rold),
            (off::ROUT, self.rout),
            (off::OTRAP, self.otrap),
            (off::VARY_SCALE, self.vary_scale),
            (off::DERIV1, self.deriv1),
            (off::DERIV1 + 8, self.deriv2),
            (off::DERIV1 + 16, self.deriv3),
            (off::DFREE1, self.dfree[0]),
            (off::DFREE2, self.dfree[1]),
        ];
        for (o, v) in f64s {
            if has(rd, o) {
                m.put_f64(b + o, v);
            }
        }
        let u32s: [(u32, u32); 5] = [
            (off::PVAR, custom::pconst_addr(n)),
            (off::IT_RESULT, self.it_result as u32),
            (off::MAX_IT, self.max_it as u32),
            (off::RSTOP, (self.rstop as f32).to_bits()),
            (off::FIRST_IT, self.first_it as u32),
        ];
        for (o, v) in u32s {
            if has(rd, o) {
                m.put_u32(b + o, v);
            }
        }
        let res = m.call_formula(code, custom::IT_C1, custom::STACK_TOP, custom::MAX_STEPS);
        if res.is_ok() {
            for k in 0..4 {
                let o = off::X + 8 * k as u32;
                if has(wr, o) {
                    self.v[k] = m.get_f64(b + o);
                }
            }
            for k in 0..3 {
                if has(wr, off::J1 + 8 * k as u32) {
                    self.j[k] = m.get_f64(b + off::J1 + 8 * k as u32);
                }
            }
            let get = |o: u32, x: &mut f64| {
                if has(wr, o) {
                    *x = m.get_f64(b + o);
                }
            };
            get(off::J4, &mut self.j[3]);
            get(off::ROUT, &mut self.rout);
            get(off::OTRAP, &mut self.otrap);
            get(off::VARY_SCALE, &mut self.vary_scale);
            get(off::DERIV1, &mut self.deriv1);
            get(off::DERIV1 + 8, &mut self.deriv2);
            get(off::DERIV1 + 16, &mut self.deriv3);
            get(off::DFREE1, &mut self.dfree[0]);
            get(off::DFREE2, &mut self.dfree[1]);
            if has(wr, off::FIRST_IT) {
                self.first_it = m.get_u32(b + off::FIRST_IT) as i32;
            }
        }
        if res.is_err() {
            CUSTOM_ERRORS.fetch_add(1, Ordering::Relaxed);
            self.v = [f64::INFINITY; 4];
            self.rout = f64::INFINITY;
        }
    }

    /// Calls the iteration function selected by `mode` (`mMandFunction`).
    #[inline]
    pub fn mand_function(&mut self, mode: HybridMode, slots: &[HybridSlot]) {
        match mode {
            HybridMode::Alt3D => self.hybrid_3d(slots),
            HybridMode::Alt4D => self.hybrid_4d(slots),
            HybridMode::Ipol3D => self.ipol_3d(slots),
            HybridMode::Ipol4D => self.ipol_4d(slots),
        }
    }
}
