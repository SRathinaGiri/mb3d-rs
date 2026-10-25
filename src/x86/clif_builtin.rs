//! The built-in formulas (`formulas.rs`) as Cranelift code, for the
//! compiled hybrids: the same expressions in the same order (they are
//! copied from `formulas.rs`; the values of type [`X`] are Cranelift values
//! with Rust's arithmetic operators), so the results are identical bit for
//! bit (checked by `clifcheck --builtins`).

use crate::formulas::Formula;
use cranelift_codegen::ir::condcodes::FloatCC;
use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use std::cell::Cell;
use std::ops::{Add, Div, Mul, Neg, Sub};

thread_local! {
    /// the builder the operators of [`X`] emit into (while `emit` runs)
    static FB: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
}

fn with_b<T>(f: impl FnOnce(&mut FunctionBuilder) -> T) -> T {
    let p = FB.with(|c| c.get());
    assert!(!p.is_null(), "X used outside clif_builtin::emit");
    // SAFETY: `emit` sets the pointer to its builder for the time it runs
    // and nothing else uses the builder meanwhile
    let b = unsafe { &mut *(p as *mut FunctionBuilder) };
    f(b)
}

/// A double of the compiled code.
#[derive(Clone, Copy)]
pub struct X(pub Value);

fn k(v: f64) -> X {
    X(with_b(|b| b.ins().f64const(v)))
}

impl X {
    fn sqrt(self) -> X {
        X(with_b(|b| b.ins().sqrt(self.0)))
    }
    fn abs(self) -> X {
        X(with_b(|b| b.ins().fabs(self.0)))
    }
    /// `if self < o { a } else { b }`
    fn lt_select(self, o: X, a: X, c: X) -> X {
        X(with_b(|b| {
            let t = b.ins().fcmp(FloatCC::LessThan, self.0, o.0);
            b.ins().select(t, a.0, c.0)
        }))
    }
}

macro_rules! binop {
    ($tr:ident, $m:ident, $ins:ident) => {
        impl $tr for X {
            type Output = X;
            fn $m(self, o: X) -> X {
                X(with_b(|b| b.ins().$ins(self.0, o.0)))
            }
        }
        impl $tr<f64> for X {
            type Output = X;
            fn $m(self, o: f64) -> X {
                let o = k(o);
                X(with_b(|b| b.ins().$ins(self.0, o.0)))
            }
        }
        impl $tr<X> for f64 {
            type Output = X;
            fn $m(self, o: X) -> X {
                let s = k(self);
                X(with_b(|b| b.ins().$ins(s.0, o.0)))
            }
        }
    };
}
binop!(Add, add, fadd);
binop!(Sub, sub, fsub);
binop!(Mul, mul, fmul);
binop!(Div, div, fdiv);

impl Neg for X {
    type Output = X;
    fn neg(self) -> X {
        X(with_b(|b| b.ins().fneg(self.0)))
    }
}

/// Whether [`emit`] compiles `f` (the others are called through
/// `mb3d_builtin`).
pub fn supported(f: &Formula) -> bool {
    matches!(f, Formula::IntPow { .. } | Formula::Quaternion { .. } | Formula::Tricorn { .. } | Formula::AmazingBox { .. } | Formula::FoldingIntPow { .. })
}

/// One iteration of the built-in formula `f` on `v`, `j` (`Formula::iterate`
/// with `rout` and `ade`); false if it is not one of the compiled ones.
pub fn emit(b: &mut FunctionBuilder, f: &Formula, v: &mut [Value; 4], j: &mut [Value; 4], rout: Value, ade: bool) -> bool {
    if !supported(f) {
        return false;
    }
    let _ = rout;
    FB.with(|c| c.set(b as *mut FunctionBuilder as *mut ()));
    let mut vx = v.map(X);
    let jx = j.map(X);
    match *f {
        Formula::IntPow { power, z_mul } => int_pow(power, z_mul, &mut vx, &jx),
        Formula::Quaternion { yw_mul, w_add } => {
            let (x, y, z, w) = (vx[0], vx[1], vx[2], vx[3]);
            vx[0] = x * x - y * y - z * z - w * w + jx[0];
            vx[1] = 2.0 * (y * x + z * w) + jx[1];
            vx[2] = 2.0 * (z * x + yw_mul * y * w) + jx[2];
            vx[3] = 2.0 * (w * x + y * z) + w_add + jx[3];
        }
        Formula::Tricorn { z_mul, cz_mul } => {
            let (x, y, z) = (vx[0], vx[1], vx[2]);
            vx[2] = z * x * z_mul + jx[2] * cz_mul;
            vx[0] = x * x - (y * y + z * z) + jx[0];
            vx[1] = 2.0 * (y * x) + jx[1];
        }
        Formula::AmazingBox { scale, min_r, fold } => amazing_box(scale, min_r, fold, &mut vx, &jx, ade),
        Formula::FoldingIntPow { power, z_mul, fold } => {
            for c in vx.iter_mut().take(3) {
                *c = box_fold(*c, fold);
            }
            int_pow(power, z_mul, &mut vx, &jx);
        }
        _ => unreachable!(),
    }
    FB.with(|c| c.set(std::ptr::null_mut()));
    *v = vx.map(|x| x.0);
    true
}

fn box_fold(x: X, fold: f64) -> X {
    (x + fold).abs() - ((x - fold).abs() + x)
}

fn amazing_box(scale: f64, min_r: f64, fold: f64, v: &mut [X; 4], j: &[X; 4], ade: bool) {
    let x = box_fold(v[0], fold);
    let y = box_fold(v[1], fold);
    let z = box_fold(v[2], fold);
    let r = z * z + y * y + x * x;
    let sqr_min_r = min_r * min_r;
    // if r < sqr_min_r { scale / sqr_min_r } else if 1.0 < r { scale } else { scale / r }
    let mul = r.lt_select(k(sqr_min_r), k(scale / sqr_min_r), k(1.0).lt_select(r, k(scale), scale / r));
    if ade {
        v[3] = v[3] * mul;
    }
    v[2] = z * mul + j[2];
    v[1] = y * mul + j[1];
    v[0] = x * mul + j[0];
}

fn int_pow(power: i32, zmul: f64, v: &mut [X; 4], j: &[X; 4]) {
    let (x, y, z) = (v[0], v[1], v[2]);
    const EPS: f64 = 1e-40;
    match power {
        i32::MIN..=2 => {
            let yy = y * y;
            let xx = x * x;
            let r = xx + yy;
            v[2] = (r.sqrt() * zmul * z) * 2.0 + j[2];
            let a = (r - z * z) / r;
            v[0] = (xx - yy) * a + j[0];
            v[1] = (x * y * a) * 2.0 + j[1];
        }
        3 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let a = 1.0 - 3.0 * sz / (r + EPS);
            v[0] = (sx - 3.0 * sy) * a * x + j[0];
            v[2] = j[2] - (sz - 3.0 * r) * z * zmul;
            v[1] = a * (3.0 * sx - sy) * y + j[1];
        }
        4 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let a = 1.0 + sz * (sz - 6.0 * r) / (r * r + EPS);
            v[0] = (sx * (sx - 6.0 * sy) + sy * sy) * a + j[0];
            v[2] = r.sqrt() * (r - sz) * z * 4.0 * zmul + j[2];
            v[1] = a * x * (sx - sy) * 4.0 * y + j[1];
        }
        5 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let a = 1.0 + (sz * sz - sz * r * 2.0) * 5.0 / (r * r + EPS);
            v[1] = (5.0 * sx * sx - sy * (10.0 * sx - sy)) * a * y + j[1];
            v[2] = (sz * (sz - 10.0 * r) + 5.0 * r * r) * z * zmul + j[2];
            v[0] = a * x * (sx * (sx - 10.0 * sy) + 5.0 * sy * sy) + j[0];
        }
        6 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let rr = r * r;
            let a = 1.0 - sz * (15.0 * rr + sz * (sz - r * 15.0)) / (rr * r + EPS);
            v[1] = (3.0 * sy * sy + sx * (3.0 * sx - 10.0 * sy)) * a * y * x * 2.0 + j[1];
            v[2] = (sz * (3.0 * sz - 10.0 * r) + 3.0 * rr) * r.sqrt() * z * zmul * 2.0 + j[2];
            v[0] = (sy * sy * (15.0 * sx - sy) + sx * sx * (sx - 15.0 * sy)) * a + j[0];
        }
        7 => {
            let sy = y * y;
            let sx = x * x;
            let r = sx + sy;
            let sz = z * z;
            let rr = r * r;
            let a = 1.0 - 7.0 * (sz * (3.0 * rr + sz * (sz - 5.0 * r))) / (rr * r + EPS);
            v[1] = (sx * (21.0 * sy * sy + sx * (7.0 * sx - 35.0 * sy)) - sy * sy * sy) * a * y + j[1];
            v[2] = j[2] - (sz * sz * sz - 7.0 * r * (rr + sz * (3.0 * sz - 5.0 * r))) * z * zmul;
            v[0] = a * x * (sx * (35.0 * sy * sy + sx * (sx - 21.0 * sy)) - 7.0 * sy * sy * sy) + j[0];
        }
        _ => {
            let xx = x * x;
            let yy = y * y;
            let zz = z * z;
            let r = xx + yy;
            let rr = r * r;
            let zzzz = zz * zz;
            let t = (zzzz - 6.0 * r * zz + rr) * (zz - r) * r.sqrt();
            v[2] = -(t * z * 8.0 * zmul) + j[2];
            let num = (rr * 70.0 + zzzz) * zzzz - 28.0 * zz * r * (zzzz + rr);
            let a = num / (rr * rr + EPS) + 1.0;
            let xxxx = xx * xx;
            let yyyy = yy * yy;
            v[1] = (yyyy * (7.0 * xx - yy) + xxxx * (xx - 7.0 * yy)) * 8.0 * x * y * a + j[1];
            v[0] = a * (xxxx * xxxx + yyyy * (70.0 * xxxx + yyyy) - 28.0 * (xx * yy * (yyyy + xxxx))) + j[0];
        }
    }
}
