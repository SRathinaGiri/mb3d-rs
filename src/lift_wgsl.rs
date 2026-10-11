//! WGSL code from the lifting analysis (stage 2 xof the GPU work): lifted
//! formulas as shader functions in single precision.
//!
//! The cells xof the lifted formula (see `lift_emit`) get a type: a cell
//! read and written only as a whole double becomes an `f32`; any other cell
//! two `u32` halves (integers, singles via bitcast, and doubles that are
//! also accessed in parts: their exact bits, converted to f32 where the
//! formula uses them as a double).
//! Formulas with SSE code are not translated yet.  The record cells are private globals shared by all
//! formulas xof a shader (and the iteration loop); the constants come from a
//! storage buffer the CPU fills ([`WgslFormula::constants`]); the stack
//! cells are locals.  Registers, flags and the FPU condition bits are
//! private globals with the exact x86 rules (32-bit integers are exact on
//! the GPU); the x87 registers are an `f32` array.

use super::lift::Analysis;
use super::*;
use crate::custom::STACK_TOP;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

const STACK_LOW: u32 = BASE + 0x4000;

/// How a cell is represented on the GPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellTy {
    /// a double, as f32 (`<name>f`)
    Whole,
    /// two u32 halves (`<name>l`, `<name>h`)
    Halves,
}

/// A formula translated to WGSL.
#[derive(Debug)]
pub struct WgslFormula {
    /// the function `fn <name>(cb: u32)`; `cb` = first index xof the slot's
    /// constants in the storage buffer `cst`
    pub code: String,
    /// the record cells it uses (cell index = byte offset / 8 from J4)
    pub record: Vec<(u32, CellTy)>,
    /// the constant cells (8 bytes each, from the start xof the variable
    /// buffer): cell, type, index in the slot's constants
    pub constants: Vec<(u32, CellTy, u32)>,
    /// number xof u32 entries xof the slot's constants
    pub const_len: u32,
    /// the formula writes its variable buffer (state between calls)
    pub writes_vars: bool,
}

impl WgslFormula {
    /// The u32 constants xof a slot from its variable buffer (the bytes from
    /// the buffer start): a whole double is its f32 bits (and 0), or with
    /// `dbl` its own two words (the shader's `cst_r`).
    pub fn constants_from(&self, var_buf: &[u8], dbl: bool) -> Vec<u32> {
        let mut v = vec![0u32; self.const_len as usize];
        for &(c, ty, i) in &self.constants {
            let o = c as usize * 8;
            let b = |k: usize| var_buf.get(o + k).copied().unwrap_or(0);
            let lo = u32::from_le_bytes([b(0), b(1), b(2), b(3)]);
            let hi = u32::from_le_bytes([b(4), b(5), b(6), b(7)]);
            match ty {
                CellTy::Whole if dbl => {
                    v[i as usize] = lo;
                    v[i as usize + 1] = hi;
                }
                CellTy::Whole => v[i as usize] = (f64::from_bits(lo as u64 | (hi as u64) << 32) as f32).to_bits(),
                CellTy::Halves => {
                    v[i as usize] = lo;
                    v[i as usize + 1] = hi;
                }
            }
        }
        v
    }
}

/// The flag rules xof `Flags` and helpers, in WGSL (shared by all formulas),
/// for the shader's real type (`dbl`: R = f64).
pub fn prelude(dbl: bool) -> String {
    format!("{}{}", PRELUDE_COMMON, if dbl { HELPERS_F64 } else { HELPERS_F32 })
}

const HELPERS_F32: &str = r#"
fn r_sin(x: R) -> R { return sin(x); }
fn r_cos(x: R) -> R { return cos(x); }
fn r_tan(x: R) -> R { return tan(x); }
fn r_asin(x: R) -> R { return asin(x); }
fn r_acos(x: R) -> R { return acos(x); }
fn r_atan(x: R) -> R { return atan(x); }
fn r_atan2(y: R, x: R) -> R { return atan2(y, x); }
fn r_exp(x: R) -> R { return exp(x); }
fn r_exp2(x: R) -> R { return exp2(x); }
fn r_log(x: R) -> R { return log(x); }
fn r_log2(x: R) -> R { return log2(x); }
fn r_pow(a: R, b: R) -> R { return pow(a, b); }
// the halves of the double an f32 converts to (exact), and a double from
// its halves rounded to f32 (nearest even): integer access to parts of a
// double held as f32 (abs / negation on the sign, copies as two halves)
fn f64_hi(x: f32) -> u32 {
    let b = bitcast<u32>(x);
    let s = b & 0x80000000u;
    let e = (b >> 23u) & 0xFFu;
    let m = b & 0x7FFFFFu;
    if (e == 0u) {
        if (m == 0u) { return s; }
        let sh = countLeadingZeros(m) - 8u;
        let mn = (m << sh) & 0x7FFFFFu;
        return s | ((897u - sh) << 20u) | (mn >> 3u);
    }
    if (e == 255u) { return s | 0x7FF00000u | (m >> 3u); }
    return s | ((e + 896u) << 20u) | (m >> 3u);
}

fn f64_lo(x: f32) -> u32 {
    let b = bitcast<u32>(x);
    let e = (b >> 23u) & 0xFFu;
    var m = b & 0x7FFFFFu;
    if (e == 0u && m != 0u) {
        let sh = countLeadingZeros(m) - 8u;
        m = (m << sh) & 0x7FFFFFu;
    }
    return (m & 7u) << 29u;
}

fn from_f64_bits(hi: u32, lo: u32) -> f32 {
    let s = hi & 0x80000000u;
    let e = (hi >> 20u) & 0x7FFu;
    let mh = hi & 0xFFFFFu;
    if (e == 0x7FFu) {
        if (mh == 0u && lo == 0u) { return bitcast<f32>(s | 0x7F800000u); }
        return bitcast<f32>(s | 0x7FC00000u);
    }
    if (e == 0u) { return bitcast<f32>(s); }
    var m = (mh << 3u) | (lo >> 29u);
    let rest = lo & 0x1FFFFFFFu;
    if (rest > 0x10000000u || (rest == 0x10000000u && (m & 1u) == 1u)) { m += 1u; }
    var ee = i32(e) - 896;
    if (m == 0x800000u) { m = 0u; ee += 1; }
    if (ee >= 255) { return bitcast<f32>(s | 0x7F800000u); }
    if (ee <= 0) {
        let v = ldexp(1.0 + f32(m) / 8388608.0, ee - 127);
        return bitcast<f32>(s | bitcast<u32>(v));
    }
    return bitcast<f32>(s | (u32(ee) << 23u) | m);
}

fn fnan(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7FFFFFFFu) > 0x7F800000u;
}
fn cst_r(i: u32) -> R {
    return bitcast<f32>(cst[i]);
}
fn rlit(f: u32, lo: u32, hi: u32) -> R {
    return bitcast<f32>(f);
}

"#;

const HELPERS_F64: &str = r#"

// the transcendental functions in double precision (GPUs have them for f32
// only): range reduction and polynomials, about 1e-16 relative
fn r_pow2i(k: i32) -> R {
    // 2^k for -1022 <= k <= 1023
    return bitcast<f64>(u64(k + 1023) << 52u);
}
fn r_exp(x: R) -> R {
    if (x != x) { return x; }
    if (x > 709.782712893384) { return bitcast<f64>(0x7FF0000000000000lu); }
    if (x < -745.2) { return 0.0lf; }
    let k = round(x * 1.4426950408889634lf);
    let r = (x - k * 0.6931471803691238lf) - k * 1.9082149292705877e-10lf;
    var p = 1.0lf / 6227020800.0lf;
    p = p * r + 1.0lf / 479001600.0lf;
    p = p * r + 1.0lf / 39916800.0lf;
    p = p * r + 1.0lf / 3628800.0lf;
    p = p * r + 1.0lf / 362880.0lf;
    p = p * r + 1.0lf / 40320.0lf;
    p = p * r + 1.0lf / 5040.0lf;
    p = p * r + 1.0lf / 720.0lf;
    p = p * r + 1.0lf / 120.0lf;
    p = p * r + 1.0lf / 24.0lf;
    p = p * r + 1.0lf / 6.0lf;
    p = p * r + 0.5lf;
    p = p * r + 1.0lf;
    p = p * r + 1.0lf;
    let ki = i32(k);
    if (ki > 1023) { return p * r_pow2i(1023) * r_pow2i(ki - 1023); }
    if (ki < -1022) { return p * r_pow2i(-1022) * r_pow2i(ki + 1022); }
    return p * r_pow2i(ki);
}
fn r_log(x0: R) -> R {
    if (x0 != x0 || x0 < 0.0lf) { return bitcast<f64>(0x7FF8000000000000lu); }
    if (x0 == 0.0lf) { return bitcast<f64>(0xFFF0000000000000lu); }
    if (x0 > 1.7976931348623157e308lf) { return x0; }
    var x = x0;
    var e = 0;
    if (x < 2.2250738585072014e-308lf) { x = x * 18014398509481984.0lf; e = -54; }
    let b = bitcast<u64>(x);
    e += i32((b >> 52u) & 0x7FFlu) - 1023;
    var m = bitcast<f64>((b & 0x000FFFFFFFFFFFFFlu) | 0x3FF0000000000000lu);
    if (m > 1.4142135623730951lf) { m = m * 0.5lf; e += 1; }
    let z = (m - 1.0lf) / (m + 1.0lf);
    let z2 = z * z;
    var s = 2.0lf / 23.0lf;
    s = s * z2 + 2.0lf / 21.0lf;
    s = s * z2 + 2.0lf / 19.0lf;
    s = s * z2 + 2.0lf / 17.0lf;
    s = s * z2 + 2.0lf / 15.0lf;
    s = s * z2 + 2.0lf / 13.0lf;
    s = s * z2 + 2.0lf / 11.0lf;
    s = s * z2 + 2.0lf / 9.0lf;
    s = s * z2 + 2.0lf / 7.0lf;
    s = s * z2 + 2.0lf / 5.0lf;
    s = s * z2 + 2.0lf / 3.0lf;
    s = s * z2 + 2.0lf;
    let fe = f64(e);
    return (z * s + fe * 1.9082149292705877e-10lf) + fe * 0.6931471803691238lf;
}
fn r_exp2(x: R) -> R { return r_exp(x * 0.6931471805599453lf); }
fn r_log2(x: R) -> R { return r_log(x) * 1.4426950408889634lf; }
fn r_pow(a: R, b: R) -> R {
    if (a == 0.0lf) { return select(0.0lf, 1.0lf, b == 0.0lf); }
    return r_exp(b * r_log(a));
}
// sin and cos of the reduced argument (|r| <= pi/4)
fn r_sin_k(r: R) -> R {
    let r2 = r * r;
    var p = -1.0lf / 1307674368000.0lf;
    p = p * r2 + 1.0lf / 6227020800.0lf;
    p = p * r2 - 1.0lf / 39916800.0lf;
    p = p * r2 + 1.0lf / 362880.0lf;
    p = p * r2 - 1.0lf / 5040.0lf;
    p = p * r2 + 1.0lf / 120.0lf;
    p = p * r2 - 1.0lf / 6.0lf;
    return r + r * r2 * p;
}
fn r_cos_k(r: R) -> R {
    let r2 = r * r;
    var p = 1.0lf / 20922789888000.0lf;
    p = p * r2 - 1.0lf / 87178291200.0lf;
    p = p * r2 + 1.0lf / 479001600.0lf;
    p = p * r2 - 1.0lf / 3628800.0lf;
    p = p * r2 + 1.0lf / 40320.0lf;
    p = p * r2 - 1.0lf / 720.0lf;
    p = p * r2 + 1.0lf / 24.0lf;
    p = p * r2 - 0.5lf;
    return 1.0lf + r2 * p;
}
// x - n * pi / 2 (Cody-Waite, three parts) and the quadrant n
fn r_reduce(x: R) -> vec2<f64> {
    let n = round(x * 0.6366197723675814lf);
    let r = ((x - n * 1.5707963267341256lf) - n * 6.077100506506192e-11lf) - n * 2.0222662487959506e-21lf;
    return vec2<f64>(r, n);
}
fn r_sin(x: R) -> R {
    if (abs(x) > 1.0e15lf || x != x) { return R(sin(f32(x))); }
    let q = r_reduce(x);
    let n = i32(q.y) & 3;
    if (n == 0) { return r_sin_k(q.x); }
    if (n == 1) { return r_cos_k(q.x); }
    if (n == 2) { return -r_sin_k(q.x); }
    return -r_cos_k(q.x);
}
fn r_cos(x: R) -> R {
    if (abs(x) > 1.0e15lf || x != x) { return R(cos(f32(x))); }
    let q = r_reduce(x);
    let n = i32(q.y) & 3;
    if (n == 0) { return r_cos_k(q.x); }
    if (n == 1) { return -r_sin_k(q.x); }
    if (n == 2) { return -r_cos_k(q.x); }
    return r_sin_k(q.x);
}
fn r_tan(x: R) -> R { return r_sin(x) / r_cos(x); }
// atan for 0 <= a <= 1
fn r_atan01(a0: R) -> R {
    var a = a0;
    var off = 0.0lf;
    if (a > 0.2679491924311227lf) {
        // atan(a) = pi/6 + atan((a - 1/sqrt3) / (1 + a/sqrt3))
        a = (a - 0.5773502691896258lf) / (1.0lf + a * 0.5773502691896258lf);
        off = 0.5235987755982989lf;
    }
    let t2 = a * a;
    var p = -1.0lf / 31.0lf;
    p = p * t2 + 1.0lf / 29.0lf;
    p = p * t2 - 1.0lf / 27.0lf;
    p = p * t2 + 1.0lf / 25.0lf;
    p = p * t2 - 1.0lf / 23.0lf;
    p = p * t2 + 1.0lf / 21.0lf;
    p = p * t2 - 1.0lf / 19.0lf;
    p = p * t2 + 1.0lf / 17.0lf;
    p = p * t2 - 1.0lf / 15.0lf;
    p = p * t2 + 1.0lf / 13.0lf;
    p = p * t2 - 1.0lf / 11.0lf;
    p = p * t2 + 1.0lf / 9.0lf;
    p = p * t2 - 1.0lf / 7.0lf;
    p = p * t2 + 1.0lf / 5.0lf;
    p = p * t2 - 1.0lf / 3.0lf;
    p = p * t2 + 1.0lf;
    return off + a * p;
}
fn r_atan(x: R) -> R {
    if (x != x) { return x; }
    let a = abs(x);
    var r: R;
    if (a > 1.0lf) { r = 1.5707963267948966lf - r_atan01(1.0lf / a); } else { r = r_atan01(a); }
    return select(r, -r, x < 0.0lf);
}
fn r_atan2(y: R, x: R) -> R {
    if (x != x || y != y) { return x + y; }
    if (x == 0.0lf) {
        if (y == 0.0lf) {
            // the signs of the zeros as atan2 has them
            let neg_x = (bitcast<u64>(x) >> 63u) != 0lu;
            let r = select(0.0lf, 3.141592653589793lf, neg_x);
            return select(r, -r, (bitcast<u64>(y) >> 63u) != 0lu);
        }
        return select(1.5707963267948966lf, -1.5707963267948966lf, y < 0.0lf);
    }
    let r = r_atan(y / x);
    if (x > 0.0lf) { return r; }
    return select(r - 3.141592653589793lf, r + 3.141592653589793lf, (bitcast<u64>(y) >> 63u) == 0lu);
}
fn r_asin(x: R) -> R { return r_atan2(x, sqrt(max(1.0lf - x * x, 0.0lf))); }
fn r_acos(x: R) -> R { return r_atan2(sqrt(max(1.0lf - x * x, 0.0lf)), x); }
// doubles as such (R = f64): the halves are the exact bits
fn f64_hi(x: R) -> u32 {
    return u32(bitcast<u64>(x) >> 32u);
}
fn f64_lo(x: R) -> u32 {
    return u32(bitcast<u64>(x) & 0xFFFFFFFFlu);
}
fn from_f64_bits(hi: u32, lo: u32) -> R {
    return bitcast<f64>((u64(hi) << 32u) | u64(lo));
}
fn fnan(x: R) -> bool {
    return (bitcast<u64>(x) & 0x7FFFFFFFFFFFFFFFlu) > 0x7FF0000000000000lu;
}
fn cst_r(i: u32) -> R {
    return from_f64_bits(cst[i + 1u], cst[i]);
}
fn rlit(f: u32, lo: u32, hi: u32) -> R {
    return from_f64_bits(hi, lo);
}

"#;

/// The part of the prelude that does not depend on the real type.
const PRELUDE_COMMON: &str = r#"
var<private> rg: array<u32, 8>;
var<private> xcf: bool;
var<private> xzf: bool;
var<private> xsf: bool;
var<private> xof: bool;
var<private> xpf: bool;
var<private> xaf: bool;
var<private> fsw: u32;
// the SSE registers: two doubles each as f32 (x the low lane)
var<private> xmm: array<vec2<R>, 8>;

fn szmask(size: u32) -> u32 {
    if (size >= 4u) { return 0xFFFFFFFFu; }
    return (1u << (size * 8u)) - 1u;
}

fn set_szp(r: u32, size: u32) {
    xzf = (r & szmask(size)) == 0u;
    xsf = ((r >> (size * 8u - 1u)) & 1u) == 1u;
    xpf = (countOneBits(r & 0xFFu) % 2u) == 0u;
}

// op: 0 add, 1 sub / cmp, 2 and, 3 or, 4 xor
fn alu(op: u32, a0: u32, b0: u32, size: u32) -> u32 {
    let m = szmask(size);
    let sign = 1u << (size * 8u - 1u);
    let a = a0 & m;
    let b = b0 & m;
    var r: u32;
    if (op == 0u) {
        r = (a + b) & m;
        if (size >= 4u) { xcf = r < a; } else { xcf = (a + b) > m; }
        xof = ((a ^ r) & (b ^ r) & sign) != 0u;
        xaf = ((a ^ b ^ r) & 0x10u) != 0u;
    } else if (op == 1u) {
        r = (a - b) & m;
        xcf = b > a;
        xof = ((a ^ b) & (a ^ r) & sign) != 0u;
        xaf = ((a ^ b ^ r) & 0x10u) != 0u;
    } else {
        if (op == 2u) { r = a & b; } else if (op == 3u) { r = a | b; } else { r = a ^ b; }
        xcf = false;
        xof = false;
    }
    set_szp(r, size);
    return r;
}

fn cond(cc: u32) -> bool {
    var r: bool;
    switch (cc >> 1u) {
        case 0u: { r = xof; }
        case 1u: { r = xcf; }
        case 2u: { r = xzf; }
        case 3u: { r = xcf || xzf; }
        case 4u: { r = xsf; }
        case 5u: { r = xpf; }
        case 6u: { r = xsf != xof; }
        default: { r = xzf || (xsf != xof); }
    }
    if ((cc & 1u) == 1u) { return !r; }
    return r;
}

fn reg_get(r: u32, size: u32) -> u32 {
    if (size == 1u) {
        if (r < 4u) { return rg[r] & 0xFFu; }
        return (rg[r - 4u] >> 8u) & 0xFFu;
    }
    if (size == 2u) { return rg[r] & 0xFFFFu; }
    return rg[r];
}

fn reg_set(r: u32, size: u32, v: u32) {
    if (size == 1u) {
        if (r < 4u) { rg[r] = (rg[r] & ~0xFFu) | (v & 0xFFu); }
        else { rg[r - 4u] = (rg[r - 4u] & ~0xFF00u) | ((v & 0xFFu) << 8u); }
    } else if (size == 2u) {
        rg[r] = (rg[r] & ~0xFFFFu) | (v & 0xFFFFu);
    } else {
        rg[r] = v;
    }
}

fn sign_ext(v: u32, size: u32) -> i32 {
    let s = 32u - size * 8u;
    return bitcast<i32>(v << s) >> s;
}

// op: 0 shl, 1 shr, 2 sar (Flags shift rules)
fn shift_reg(op: u32, reg: u32, c0: u32, size: u32) {
    let c = c0 & 31u;
    if (c == 0u) { return; }
    let bits = size * 8u;
    let v = reg_get(reg, size);
    let m = szmask(size);
    var r: u32;
    if (op == 0u) {
        xcf = c <= bits && ((v >> (bits - c)) & 1u) == 1u;
        if (c >= 32u) { r = 0u; } else { r = (v << c) & m; }
        xof = (((r >> (bits - 1u)) & 1u) == 1u) != xcf;
    } else if (op == 1u) {
        xcf = ((v >> (c - 1u)) & 1u) == 1u;
        xof = ((v >> (bits - 1u)) & 1u) == 1u;
        if (c >= 32u) { r = 0u; } else { r = v >> c; }
    } else {
        let sv = sign_ext(v, size);
        xcf = ((sv >> min(c - 1u, 31u)) & 1) == 1;
        xof = false;
        r = bitcast<u32>(sv >> min(c, 31u)) & m;
    }
    set_szp(r, size);
    reg_set(reg, size, r);
}

// C3 C2 C0 xof fcom
// an SSE register as the bits of its doubles (low word first), and back
fn xbits(v: vec2<R>) -> vec4<u32> {
    return vec4<u32>(f64_lo(v.x), f64_hi(v.x), f64_lo(v.y), f64_hi(v.y));
}
fn xfrom(b: vec4<u32>) -> vec2<R> {
    return vec2<R>(from_f64_bits(b.y, b.x), from_f64_bits(b.w, b.z));
}
fn x_isnan(x: R) -> bool {
    return fnan(x);
}
// minsd / maxsd: the second operand unless the first is smaller / larger
fn xmin(a: R, b: R) -> R {
    return select(b, a, a < b);
}
fn xmax(a: R, b: R) -> R {
    return select(b, a, a > b);
}

fn fcompare_cc(a: R, b: R) -> u32 {
    if (fnan(a) || fnan(b)) { return 0x4500u; }
    if (a < b) { return 0x0100u; }
    if (a == b) { return 0x4000u; }
    return 0u;
}

// fist: rounded, out xof range or NaN -> the integer indefinite
fn fist32(v: R) -> u32 {
    let x = round(v);
    if (fnan(x) || x < -2147483648.0 || x >= 2147483648.0) { return 0x80000000u; }
    return bitcast<u32>(i32(x));
}

fn fist16(v: R) -> u32 {
    let x = round(v);
    if (fnan(x) || x < -32768.0 || x > 32767.0) { return 0x8000u; }
    return bitcast<u32>(i32(x)) & 0xFFFFu;
}
"#;

/// Region and byte offset as in `lift_emit` ('r' record from IT_BASE, 'v'
/// variable buffer from its start, 's' stack from STACK_LOW).
fn region(loc: super::lift::Loc) -> Option<(char, u32)> {
    use super::lift::Loc;
    match loc {
        Loc::Field(o) => Some(('r', o)),
        Loc::Var(o) => Some(('v', (o + crate::m3f::CONST_OFFSET as i32) as u32)),
        Loc::Stack(o) => Some(('s', (STACK_TOP as i32 + o) as u32 - STACK_LOW)),
        _ => None,
    }
}

fn wmask(n: u32) -> String {
    if n >= 4 {
        "0xFFFFFFFFu".into()
    } else {
        format!("{:#x}u", (1u32 << (8 * n)) - 1)
    }
}

/// A double constant: `rlit` picks the f32 or the double bits (R).
fn f32_lit(v: f64) -> String {
    let b = v.to_bits();
    format!("rlit({:#x}u, {:#x}u, {:#x}u)", (v as f32).to_bits(), b as u32, (b >> 32) as u32)
}

struct Typed<'a> {
    ty: &'a BTreeMap<(char, u32), CellTy>,
    /// constant cells: cell -> index
    cidx: &'a HashMap<u32, u32>,
    written_vars: bool,
}

impl Typed<'_> {
    /// The WGSL name xof a u32 half (`l`/`h`) or the f32 xof a cell.
    fn name(&self, p: char, c: u32, part: char) -> String {
        if p == 'v' && !self.written_vars {
            let i = self.cidx[&c] + if part == 'h' { 1 } else { 0 };
            return match part {
                'f' => format!("cst_r(cb + {i}u)"),
                _ => format!("cst[cb + {i}u]"),
            };
        }
        format!("{p}{c}{part}")
    }

    fn ty(&self, (p, off): (char, u32)) -> Option<CellTy> {
        self.ty.get(&(p, off / 8)).copied()
    }

    /// A whole double (f32 expression); of a halves cell: its bits as a
    /// double, rounded to f32.
    fn rd_f(&self, r: (char, u32)) -> Option<String> {
        let c = r.1 / 8;
        if r.1.is_multiple_of(8) && self.ty(r) == Some(CellTy::Whole) {
            return Some(self.name(r.0, c, 'f'));
        }
        // the bits form, also across cells or words
        Some(format!("from_f64_bits({}, {})", self.rd_u((r.0, r.1 + 4), 4)?, self.rd_u(r, 4)?))
    }

    fn wr_f(&self, r: (char, u32), v: &str) -> Option<String> {
        let c = r.1 / 8;
        if r.1.is_multiple_of(8) && self.ty(r) == Some(CellTy::Whole) {
            return Some(format!("{} = {v};", self.name(r.0, c, 'f')));
        }
        Some(format!("{{ let nv = {v}; {} {} }}", self.wr_u(r, 4, "f64_lo(nv)")?, self.wr_u((r.0, r.1 + 4), 4, "f64_hi(nv)")?))
    }

    /// The u32 half `part` ('l' / 'h') of a cell as an expression (of a
    /// whole cell: the halves of its double).
    fn half(&self, p: char, c: u32, part: char) -> Option<String> {
        let Some(ty) = self.ty.get(&(p, c)) else {
            super::lift_emit::lift_debug(&format!("WGSL: cell {p}{c} not typed"));
            return None;
        };
        Some(match ty {
            CellTy::Halves => self.name(p, c, part),
            // whole cells have no partial accesses (see the typing)
            CellTy::Whole => {
                super::lift_emit::lift_debug(&format!("WGSL: part of the whole cell {p}{c}"));
                return None;
            }
        })
    }

    /// The u32 word `wi` (counted from the region start) of the cells.
    fn word(&self, p: char, wi: u32) -> Option<String> {
        self.half(p, wi / 2, if wi.is_multiple_of(2) { 'l' } else { 'h' })
    }

    /// `n` bytes (1, 2, 4) as a u32 expression; an access across two words
    /// combines them.
    fn rd_u(&self, (p, off): (char, u32), n: u32) -> Option<String> {
        let (wi, s) = (off / 4, off % 4);
        let w0 = self.word(p, wi)?;
        if s + n <= 4 {
            return Some(if s == 0 && n == 4 { w0 } else { format!("(({w0} >> {}u) & {})", 8 * s, wmask(n)) });
        }
        let w1 = self.word(p, wi + 1)?;
        Some(format!("((({w0} >> {sh}u) | ({w1} << {}u)) & {})", 32 - 8 * s, wmask(n), sh = 8 * s))
    }

    fn wr_u(&self, (p, off): (char, u32), n: u32, v: &str) -> Option<String> {
        let (wi, s) = (off / 4, off % 4);
        let cur = self.word(p, wi)?;
        let mk = wmask(n);
        if s + n <= 4 {
            let new = if s == 0 && n == 4 { format!("({v})") } else { format!("(({cur} & ~({mk} << {sh}u)) | ((({v}) & {mk}) << {sh}u))", sh = 8 * s) };
            return Some(format!("{cur} = {new};"));
        }
        // across two words: 4 - s bytes in the first, the rest in the next
        let cur1 = self.word(p, wi + 1)?;
        let mk0 = wmask(4 - s);
        let mk1 = wmask(s + n - 4);
        Some(format!(
            "{{ let wv = {v}; {cur} = ({cur} & ~({mk0} << {sh}u)) | ((wv & {mk0}) << {sh}u); {cur1} = ({cur1} & ~{mk1}) | ((wv >> {b}u) & {mk1}); }}",
            sh = 8 * s,
            b = 32 - 8 * s
        ))
    }

    /// x87 load xof `kind` (f32 expression).
    fn fload(&self, r: (char, u32), kind: FKind) -> Option<String> {
        Some(match kind {
            FKind::F64 => self.rd_f(r)?,
            FKind::F32 => format!("R(bitcast<f32>({}))", self.rd_u(r, 4)?),
            FKind::I32 => format!("R(bitcast<i32>({}))", self.rd_u(r, 4)?),
            FKind::I16 => format!("R(sign_ext({}, 2u))", self.rd_u(r, 2)?),
            _ => return None,
        })
    }

    fn fstore(&self, r: (char, u32), kind: FKind, v: &str) -> Option<String> {
        Some(match kind {
            FKind::F64 => self.wr_f(r, v)?,
            FKind::F32 => self.wr_u(r, 4, &format!("bitcast<u32>(f32({v}))"))?,
            FKind::I32 => self.wr_u(r, 4, &format!("fist32({v})"))?,
            FKind::I16 => self.wr_u(r, 2, &format!("fist16({v})"))?,
            _ => return None,
        })
    }
}

fn fop(op: &FOp, a: &str, b: &str) -> String {
    match op {
        FOp::Add => format!("{a} + {b}"),
        FOp::Mul => format!("{a} * {b}"),
        FOp::Sub => format!("{a} - {b}"),
        FOp::SubR => format!("{b} - {a}"),
        FOp::Div => format!("{a} / {b}"),
        FOp::DivR => format!("{b} / {a}"),
    }
}

fn rot(n: u8) -> String {
    let perm: Vec<String> = (0..8).map(|k| format!("f[{}]", (k + 8 - (n as usize) % 8) % 8)).collect();
    format!("f = array<R, 8>({});", perm.join(", "))
}

/// The address expression xof a memory operand on the registers (for lea).
fn ea_wgsl(m: &Mem) -> String {
    let mut e = format!("{:#x}u", m.disp as u32);
    if let Some(b) = m.base {
        e = format!("(rg[{b}] + {e})");
    }
    if let Some((i, sc)) = m.index {
        e = format!("({e} + rg[{i}] * {sc}u)");
    }
    e
}

fn alu_code(op: Alu) -> u32 {
    match op {
        Alu::Add | Alu::Adc => 0,
        Alu::Sub | Alu::Sbb | Alu::Cmp => 1,
        Alu::And => 2,
        Alu::Or => 3,
        Alu::Xor => 4,
    }
}

impl Prog {
    /// The formula as a WGSL function `fn <name>(cb: u32)`, or None when it
    /// is not liftable or not typable (see the module documentation).
    pub fn emit_wgsl(&self, name: &str, difs: bool) -> Option<WgslFormula> {
        self.emit_wgsl_with(name, difs, &BTreeMap::new())
    }

    /// `emit_wgsl` with record cells forced to the halves (bits) form, so
    /// that all formulas of a shader agree on the record's cell types.
    pub fn emit_wgsl_with(&self, name: &str, difs: bool, force_halves: &BTreeMap<u32, CellTy>) -> Option<WgslFormula> {
        let (_, cells) = self.emit_lifted_cells("typing", difs)?;
        // cell types
        let mut ty: BTreeMap<(char, u32), CellTy> = BTreeMap::new();
        let mut whole: BTreeMap<(char, u32), (bool, bool)> = BTreeMap::new();
        for (cell, off, n, k) in &cells.log {
            let w = whole.entry(*cell).or_insert((false, false));
            if *k == 'd' && *off == 0 && *n == 8 {
                w.0 = true;
            } else {
                w.1 = true;
            }
        }
        for (cell, (d, other)) in &whole {
            // cells used only as whole doubles are f32; cells also used in
            // parts keep the exact bits of the double (two u32), the whole
            // double accesses convert (f64_hi, f64_lo, from_f64_bits)
            let forced = cell.0 == 'r' && force_halves.get(&cell.1) == Some(&CellTy::Halves);
            ty.insert(*cell, if *d && !*other && !forced { CellTy::Whole } else { CellTy::Halves });
        }
        let written_vars = self.var_written(&cells);
        // constants: indices in the slot's constants
        let mut cidx: HashMap<u32, u32> = HashMap::new();
        let mut constants = Vec::new();
        let mut len = 0u32;
        for ((p, c), t) in &ty {
            if *p == 'v' {
                cidx.insert(*c, len);
                constants.push((*c, *t, len));
                len += 2; // a whole double: its f32 or its double (constants_from)
            }
        }
        let tz = Typed { ty: &ty, cidx: &cidx, written_vars };
        let an = self.analyse(difs);
        let n = self.ops.len();
        let mut start = vec![false; n + 1];
        start[0] = true;
        for (k, op) in self.ops.iter().enumerate() {
            match op {
                Op::Jcc { target, .. } | Op::Jmp(target) | Op::Call { target, .. } | Op::JccRot { target, .. } | Op::JmpRot { target, .. } | Op::CallRot { target, .. } => {
                    start[*target as usize] = true;
                    start[k + 1] = true;
                }
                Op::Ret(_) => start[k + 1] = true,
                _ => {}
            }
        }
        let mut keys: Vec<(usize, Vec<usize>)> = an.states.keys().filter(|(k, _)| start[*k]).cloned().collect();
        keys.sort();
        let ids: HashMap<(usize, Vec<usize>), usize> = keys.iter().enumerate().map(|(i, k)| (k.clone(), i)).collect();
        let id = |k: usize, cs: &[usize]| ids.get(&(k, cs.to_vec())).copied();
        let mut body = String::new();
        for (bi, (k0, cs)) in keys.iter().enumerate() {
            let _ = writeln!(body, "            case {bi}u: {{");
            let mut k = *k0;
            loop {
                let (line, ended) = match self.wgsl_op(k, cs, &an, &tz, &id) {
                    Some(x) => x,
                    None => {
                        super::lift_emit::lift_debug(&format!("WGSL: op {k} {:?}", self.ops[k]));
                        return None;
                    }
                };
                if !line.is_empty() {
                    let _ = writeln!(body, "                {line}");
                }
                k += 1;
                if ended {
                    break;
                }
                if k >= n {
                    let _ = writeln!(body, "                return;");
                    break;
                }
                if start[k] {
                    let _ = writeln!(body, "                b = {}u;", id(k, cs)?);
                    break;
                }
            }
            let _ = writeln!(body, "            }}");
        }
        let mut o = String::new();
        let _ = writeln!(o, "fn {name}(cb: u32) {{");
        let _ = writeln!(o, "    var f: array<R, 8>;");
        for ((p, c), t) in &ty {
            let local = *p == 's' || (*p == 'v' && written_vars);
            if !local {
                continue;
            }
            // the stack cells xof the call (return address, PIteration3D, @w;
            // dIFS: the return address) start with the values the call pushes
            let call_cells: Vec<(u32, u32)> = if difs {
                vec![(STACK_TOP - 4, RETURN_SENTINEL)]
            } else {
                let x = crate::custom::IT_C1 - 32;
                vec![(STACK_TOP - 12, RETURN_SENTINEL), (STACK_TOP - 8, crate::custom::IT_C1), (STACK_TOP - 4, x + 24)]
            };
            let init = |part: char| -> String {
                if *p == 's' && part != 'f' {
                    let a = STACK_LOW + c * 8 + if part == 'h' { 4 } else { 0 };
                    if let Some((_, v)) = call_cells.iter().find(|(ca, _)| *ca == a) {
                        return format!("{v:#x}u");
                    }
                }
                if *p == 'v' {
                    let i = cidx[c] + if part == 'h' { 1 } else { 0 };
                    if part == 'f' { format!("cst_r(cb + {i}u)") } else { format!("cst[cb + {i}u]") }
                } else if part == 'f' {
                    "0.0".into()
                } else {
                    "0u".into()
                }
            };
            match t {
                CellTy::Whole => {
                    let _ = writeln!(o, "    var {p}{c}f: R = {};", init('f'));
                }
                CellTy::Halves => {
                    let _ = writeln!(o, "    var {p}{c}l: u32 = {};", init('l'));
                    let _ = writeln!(o, "    var {p}{c}h: u32 = {};", init('h'));
                }
            }
        }
        let _ = writeln!(o, "    var b: u32 = {}u;", id(0, &[])?);
        let _ = writeln!(o, "    for (var guard = 0u; guard < 200000u; guard++) {{");
        let _ = writeln!(o, "        switch (b) {{");
        o.push_str(&body);
        let _ = writeln!(o, "            default: {{ return; }}");
        let _ = writeln!(o, "        }}");
        let _ = writeln!(o, "    }}");
        let _ = writeln!(o, "}}");
        let record = ty.iter().filter(|((p, _), _)| *p == 'r').map(|((_, c), t)| (*c, *t)).collect();
        Some(WgslFormula { code: o, record, constants, const_len: len, writes_vars: written_vars })
    }

    fn var_written(&self, cells: &super::lift_emit::Cells) -> bool {
        cells.written_cells().any(|(p, _)| p == 'v')
    }

    #[allow(clippy::too_many_arguments)]
    /// An x87 double access to a record field chosen by a small index
    /// (`Loc::FieldIdx`): a select chain / switch over the possible cells.
    fn wgsl_idx_op(&self, k: usize, cs: &[usize], an: &Analysis, t: &Typed) -> Option<String> {
        let (mem, kind): (&Mem, Option<FKind>) = match &self.ops[k] {
            Op::FLd64 { mem, .. } | Op::FSt64 { mem, .. } | Op::FArithM64 { mem, .. } => (mem, None),
            Op::FLd { mem, kind, .. } | Op::FSt { mem, kind, .. } | Op::FArithM { mem, kind, .. } | Op::FComM { mem, kind, .. } => (mem, Some(*kind)),
            _ => return None,
        };
        if kind.is_some_and(|k| k != FKind::F64) {
            return None;
        }
        let ((p, off), sc, mask) = super::lift_emit::idx_region(an.loc(k, cs, mem))?;
        let (ireg, _) = mem.index?;
        if sc != 8 || off % 8 != 0 {
            return None;
        }
        let sel = format!("(rg[{ireg}] & {mask}u)");
        let fx = |i: &u8| format!("f[{}]", i & 7);
        let rd = || -> Option<String> {
            let mut e = t.rd_f((p, off))?;
            for i in 1..=mask as u32 {
                e = format!("select({e}, {}, {sel} == {i}u)", t.rd_f((p, off + 8 * i))?);
            }
            Some(e)
        };
        Some(match &self.ops[k] {
            Op::FLd64 { dst, .. } | Op::FLd { dst, .. } => format!("f[{}] = {};", dst & 7, rd()?),
            Op::FArithM64 { op, dst, .. } | Op::FArithM { op, dst, .. } => {
                let d = fx(dst);
                format!("{d} = {};", fop(op, &d, &format!("({})", rd()?)))
            }
            Op::FComM { a, .. } => format!("fsw = fcompare_cc(f[{}], {});", a & 7, rd()?),
            Op::FSt64 { src, .. } | Op::FSt { src, .. } => {
                let mut arms = String::new();
                for i in 0..=mask as u32 {
                    arms.push_str(&format!("case {i}u: {{ {} }} ", t.wr_f((p, off + 8 * i), &fx(src))?));
                }
                format!("switch {sel} {{ {arms}default: {{}} }}")
            }
            _ => return None,
        })
    }

    fn wgsl_op(
        &self,
        k: usize,
        cs: &[usize],
        an: &Analysis,
        t: &Typed,
        id: &dyn Fn(usize, &[usize]) -> Option<usize>,
    ) -> Option<(String, bool)> {
        let loc = |mm: &Mem| region(an.loc(k, cs, mm));
        let fx = |i: &u8| format!("f[{}]", i & 7);
        if let Some(line) = self.wgsl_idx_op(k, cs, an, t) {
            return Some((line, false));
        }
        let line = match &self.ops[k] {
            Op::MovRM { r, mem } => format!("rg[{r}] = {};", t.rd_u(loc(mem)?, 4)?),
            Op::MovMR { mem, r } => t.wr_u(loc(mem)?, 4, &format!("rg[{r}]"))?,
            Op::MovRR { d, s } => format!("rg[{d}] = rg[{s}];"),
            Op::Lea { r, mem } => format!("rg[{r}] = {};", ea_wgsl(mem)),
            Op::PushR(r) => {
                let w = t.wr_u(region(an.stack_loc(k, cs, true))?, 4, &format!("rg[{r}]"))?;
                format!("rg[4] = rg[4] - 4u; {w}")
            }
            Op::PopR(r) => {
                let v = t.rd_u(region(an.stack_loc(k, cs, false))?, 4)?;
                format!("rg[{r}] = {v}; rg[4] = rg[4] + 4u;")
            }
            Op::FLd64 { dst, mem } => format!("f[{}] = {};", dst & 7, t.rd_f(loc(mem)?)?),
            Op::FSt64 { src, mem } => t.wr_f(loc(mem)?, &fx(src))?,
            Op::FArithM64 { op, dst, mem } => {
                let d = fx(dst);
                format!("{d} = {};", fop(op, &d, &format!("({})", t.rd_f(loc(mem)?)?)))
            }
            Op::FLd { dst, kind, mem } => format!("f[{}] = {};", dst & 7, t.fload(loc(mem)?, *kind)?),
            Op::FSt { src, kind, mem } => t.fstore(loc(mem)?, *kind, &fx(src))?,
            Op::FArithM { op, dst, kind, mem } => {
                let d = fx(dst);
                format!("{d} = {};", fop(op, &d, &format!("({})", t.fload(loc(mem)?, *kind)?)))
            }
            Op::FMov { dst, src } => format!("f[{}] = f[{}];", dst & 7, src & 7),
            Op::FSwap { a, b } => format!("{{ let t = f[{0}]; f[{0}] = f[{1}]; f[{1}] = t; }}", a & 7, b & 7),
            Op::FArith { op, dst, a, b } => format!("f[{}] = {};", dst & 7, fop(op, &fx(a), &fx(b))),
            Op::FConst { dst, v } => format!("f[{}] = {};", dst & 7, f32_lit(*v)),
            Op::FChs(i) => format!("f[{0}] = -f[{0}];", i & 7),
            Op::FAbs(i) => format!("f[{0}] = abs(f[{0}]);", i & 7),
            Op::FSqrt(i) => format!("f[{0}] = sqrt(f[{0}]);", i & 7),
            Op::FSin(i) => format!("f[{0}] = r_sin(f[{0}]); fsw = fsw & ~0x0400u;", i & 7),
            Op::FCos(i) => format!("f[{0}] = r_cos(f[{0}]); fsw = fsw & ~0x0400u;", i & 7),
            Op::FRndint(i) => format!("f[{0}] = round(f[{0}]);", i & 7),
            Op::FPrem { a, b, ieee } => format!(
                "{{ let x = f[{0}]; let y = f[{1}]; let q = {2}; f[{0}] = x - q * y; let qi = u32(min(abs(q), 4294967000.0)); \
                 fsw = select(0u, 0x100u, (qi & 4u) != 0u) | select(0u, 0x4000u, (qi & 2u) != 0u) | select(0u, 0x200u, (qi & 1u) != 0u); }}",
                a & 7,
                b & 7,
                if *ieee { "round(x / y)" } else { "trunc(x / y)" }
            ),
            Op::F2xm1(i) => format!("f[{0}] = r_exp2(f[{0}]) - 1.0;", i & 7),
            Op::FSincos { src, dst } => format!("{{ let a = f[{0}]; f[{0}] = r_sin(a); f[{1}] = r_cos(a); fsw = fsw & ~0x0400u; }}", src & 7, dst & 7),
            Op::FPatan { x, y } => format!("f[{0}] = r_atan2(f[{0}], f[{1}]);", y & 7, x & 7),
            Op::FYl2x { x, y } => format!("f[{0}] = f[{0}] * r_log2(f[{1}]);", y & 7, x & 7),
            Op::FScale { a, s } => format!("f[{0}] = f[{0}] * r_exp2(trunc(f[{1}]));", a & 7, s & 7),
            Op::FCom { a, b } => format!("fsw = fcompare_cc(f[{}], f[{}]);", a & 7, b & 7),
            Op::FComM { a, kind, mem } => format!("fsw = fcompare_cc(f[{}], {});", a & 7, t.fload(loc(mem)?, *kind)?),
            Op::FTst(a) => format!("fsw = fcompare_cc(f[{}], 0.0);", a & 7),
            Op::Fnstsw { mem: None, depth } => {
                format!("rg[0] = (rg[0] & 0xFFFF0000u) | (fsw | {:#x}u);", ((8 - *depth as u32) & 7) << 11)
            }
            Op::Fnstsw { mem: Some(mm), depth } => t.wr_u(loc(mm)?, 2, &format!("(fsw | {:#x}u)", ((8 - *depth as u32) & 7) << 11))?,
            Op::Jcc { cc, target } => {
                let next = id(k + 1, cs)?;
                return Some((format!("if (cond({cc}u)) {{ b = {}u; }} else {{ b = {next}u; }}", id(*target as usize, cs)?), true));
            }
            Op::JccRot { cc, target, rot: r } => {
                let next = id(k + 1, cs)?;
                return Some((format!("if (cond({cc}u)) {{ {} b = {}u; }} else {{ b = {next}u; }}", rot(*r), id(*target as usize, cs)?), true));
            }
            Op::Jmp(tg) => return Some((format!("b = {}u;", id(*tg as usize, cs)?), true)),
            Op::JmpRot { target, rot: r } => return Some((format!("{} b = {}u;", rot(*r), id(*target as usize, cs)?), true)),
            Op::Call { target, ret } | Op::CallRot { target, ret, .. } => {
                let ri = self.rets.iter().find(|e| e.0 == *ret)?.1;
                let mut c2 = cs.to_vec();
                c2.push(ri);
                let w = t.wr_u(region(an.stack_loc(k, cs, true))?, 4, &format!("{ret:#x}u"))?;
                let r = match &self.ops[k] {
                    Op::CallRot { rot: r, .. } => rot(*r),
                    _ => String::new(),
                };
                return Some((format!("{r} rg[4] = rg[4] - 4u; {w} b = {}u;", id(*target as usize, &c2)?), true));
            }
            Op::Ret(nb) => {
                let esp = format!("rg[4] = rg[4] + {}u;", 4 + *nb as u32);
                return Some(match cs.split_last() {
                    None => (format!("{esp} return;"), true),
                    Some((&ri, rest)) => (format!("{esp} b = {}u;", id(ri, rest)?), true),
                });
            }
            Op::Gen(ins, _, _) => self.wgsl_gen(ins, k, cs, an, t)?,
            // a map function of the record: map_px of the shader (gpu.rs
            // uploads the maps)
            Op::CallHost(Rm::Mem(m)) => {
                let sphere = super::lift::map_call_loc(&an.loc(k, cs, m))?;
                let arg = |r: usize, i: i32| Mem { base: Some(r as u8), index: None, disp: 8 * i };
                let mut v = Vec::new();
                for i in 0..3 {
                    v.push(t.rd_f(loc(&arg(EAX, i))?)?);
                }
                let mut w = String::new();
                for (i, c) in ["x", "y", "z"].iter().enumerate() {
                    w.push_str(&t.wr_f(loc(&arg(ECX, i as i32))?, &format!("mr.{c}"))?);
                    w.push(' ');
                }
                format!("{{ let mr = map_px({sphere}, bitcast<i32>(rg[2]), vec3<f32>({})); {w}}}", v.join(", "))
            }
            _ => return None,
        };
        Some((line, false))
    }

    /// One SSE op on the `xmm` registers (the semantics of `emit_sse_with`):
    /// the lanes are f32; the logic ops and the dword shuffle work on the
    /// bits of the doubles (masks).
    fn wgsl_sse(&self, op: SseOp, d: u32, src: &Rm, imm: u8, loc: &dyn Fn(&Mem) -> Option<(char, u32)>, t: &Typed) -> Option<String> {
        use SseOp::*;
        let mem = match src {
            Rm::Reg(_) => None,
            Rm::Mem(mm) => Some(loc(mm)?),
        };
        // the source as lanes (vec2<R>), as the low lane (f32), as bits
        let sf2 = |t: &Typed| -> Option<String> {
            Some(match (src, mem) {
                (Rm::Reg(r), _) => format!("xmm[{r}]"),
                (_, Some((p, off))) => format!("vec2<R>({}, {})", t.rd_f((p, off))?, t.rd_f((p, off + 8))?),
                _ => return None,
            })
        };
        let sf1 = |t: &Typed| -> Option<String> {
            Some(match (src, mem) {
                (Rm::Reg(r), _) => format!("xmm[{r}].x"),
                (_, Some((p, off))) => t.rd_f((p, off))?,
                _ => return None,
            })
        };
        let sb4 = |t: &Typed| -> Option<String> {
            Some(match (src, mem) {
                (Rm::Reg(r), _) => format!("xbits(xmm[{r}])"),
                (_, Some((p, off))) => format!("vec4<u32>({}, {}, {}, {})", t.rd_u((p, off), 4)?, t.rd_u((p, off + 4), 4)?, t.rd_u((p, off + 8), 4)?, t.rd_u((p, off + 12), 4)?),
                _ => return None,
            })
        };
        let bin = |o: &str, a: &str, b: &str| -> String {
            match o {
                "min" => format!("xmin({a}, {b})"),
                "max" => format!("xmax({a}, {b})"),
                _ => format!("({a} {o} {b})"),
            }
        };
        let pd = |o: &str| -> Option<String> {
            Some(format!("{{ let s = {}; let a = xmm[{d}]; xmm[{d}] = vec2<R>({}, {}); }}", sf2(t)?, bin(o, "a.x", "s.x"), bin(o, "a.y", "s.y")))
        };
        let sd = |o: &str| -> Option<String> { Some(format!("xmm[{d}].x = {};", bin(o, &format!("xmm[{d}].x"), &format!("({})", sf1(t)?)))) };
        let logic = |o: &str| -> Option<String> { Some(format!("xmm[{d}] = xfrom({}xbits(xmm[{d}]) {o} {});", if o == "&~" { "~" } else { "" }, sb4(t)?)) };
        Some(match op {
            UComiSd | ComiSd => format!(
                "{{ let b = {}; let a = xmm[{d}].x; xof = false; xsf = false; xaf = false; \
                 let un = x_isnan(a) || x_isnan(b); xzf = un || a == b; xpf = un; xcf = un || a < b; }}",
                sf1(t)?
            ),
            Pshufd => {
                let sel = [imm & 3, (imm >> 2) & 3, (imm >> 4) & 3, (imm >> 6) & 3];
                let lane = |a: u8, b: u8| -> Option<&'static str> {
                    match (a, b) {
                        (0, 1) => Some("x"),
                        (2, 3) => Some("y"),
                        _ => None,
                    }
                };
                match (lane(sel[0], sel[1]), lane(sel[2], sel[3])) {
                    (Some(l0), Some(l1)) => format!("{{ let s = {}; xmm[{d}] = vec2<R>(s.{l0}, s.{l1}); }}", sf2(t)?),
                    _ => {
                        let dw = |i: u8| ["x", "y", "z", "w"][i as usize];
                        format!("{{ let s = {}; xmm[{d}] = xfrom(vec4<u32>(s.{}, s.{}, s.{}, s.{})); }}", sb4(t)?, dw(sel[0]), dw(sel[1]), dw(sel[2]), dw(sel[3]))
                    }
                }
            }
            MovU => format!("xmm[{d}] = {};", sf2(t)?),
            MovSdLoad => match src {
                Rm::Reg(r) => format!("xmm[{d}].x = xmm[{r}].x;"),
                Rm::Mem(_) => format!("xmm[{d}] = vec2<R>({}, 0.0);", sf1(t)?),
            },
            MovqLoad => format!("xmm[{d}] = vec2<R>({}, 0.0);", sf1(t)?),
            MovLpdLoad => format!("xmm[{d}].x = {};", sf1(t)?),
            MovHpdLoad => format!("xmm[{d}].y = {};", sf1(t)?),
            MovHlps => format!("xmm[{d}].x = {}.y;", sf2(t)?),
            MovLhps => format!("xmm[{d}].y = {}.x;", sf2(t)?),
            AddPd => pd("+")?,
            SubPd => pd("-")?,
            MulPd => pd("*")?,
            DivPd => pd("/")?,
            MinPd => pd("min")?,
            MaxPd => pd("max")?,
            SqrtPd => format!("xmm[{d}] = sqrt({});", sf2(t)?),
            AddSd => sd("+")?,
            SubSd => sd("-")?,
            MulSd => sd("*")?,
            DivSd => sd("/")?,
            MinSd => sd("min")?,
            MaxSd => sd("max")?,
            SqrtSd => format!("xmm[{d}].x = sqrt({});", sf1(t)?),
            And => logic("&")?,
            AndN => logic("&~")?,
            Or => logic("|")?,
            Xor => logic("^")?,
            UnpckLpd => format!("xmm[{d}].y = {}.x;", sf2(t)?),
            UnpckHpd => format!("{{ let s = {}; xmm[{d}] = vec2<R>(xmm[{d}].y, s.y); }}", sf2(t)?),
            ShufPd => {
                let a = if imm & 1 == 0 { "x" } else { "y" };
                let b = if (imm >> 1) & 1 == 0 { "x" } else { "y" };
                format!("{{ let s = {}; let a = xmm[{d}]; xmm[{d}] = vec2<R>(a.{a}, s.{b}); }}", sf2(t)?)
            }
            HAddPd => format!("{{ let s = {}; let a = xmm[{d}]; xmm[{d}] = vec2<R>(a.x + a.y, s.x + s.y); }}", sf2(t)?),
            _ => return None,
        })
    }

    fn wgsl_gen(&self, ins: &Ins, k: usize, cs: &[usize], an: &Analysis, t: &Typed) -> Option<String> {
        let loc = |mm: &Mem| region(an.loc(k, cs, mm));
        let rd = |rm: &Rm, size: u8| -> Option<String> {
            Some(match rm {
                Rm::Reg(r) if size == 4 => format!("rg[{r}]"),
                Rm::Reg(r) => format!("reg_get({r}u, {size}u)"),
                Rm::Mem(mm) => t.rd_u(loc(mm)?, size as u32)?,
            })
        };
        let wr = |rm: &Rm, size: u8, v: &str| -> Option<String> {
            Some(match rm {
                Rm::Reg(r) if size == 4 => format!("rg[{r}] = {v};"),
                Rm::Reg(r) => format!("reg_set({r}u, {size}u, {v});"),
                Rm::Mem(mm) => t.wr_u(loc(mm)?, size as u32, v)?,
            })
        };
        let src = |s: &Src, size: u8| -> Option<String> {
            Some(match s {
                Src::Imm(v) => format!("{:#x}u", super::mask(*v, size)),
                Src::Rm(rm) => rd(rm, size)?,
            })
        };
        Some(match ins {
            Ins::Sse { op, dst, src: s, imm } if super::sse_native(*op, s) => self.wgsl_sse(*op, *dst as u32, s, *imm, &loc, t)?,
            Ins::SseStore { op, dst, src: s } => {
                let (p, off) = loc(dst)?;
                match op {
                    SseOp::MovStore => format!("{} {}", t.wr_f((p, off), &format!("xmm[{s}].x"))?, t.wr_f((p, off + 8), &format!("xmm[{s}].y"))?),
                    SseOp::MovSdStore | SseOp::MovLpdStore | SseOp::MovqStore => t.wr_f((p, off), &format!("xmm[{s}].x"))?,
                    SseOp::MovHpdStore => t.wr_f((p, off), &format!("xmm[{s}].y"))?,
                    _ => return None,
                }
            }
            Ins::Nop => String::new(),
            Ins::Sahf => "{ let ah = (rg[0] >> 8u) & 0xFFu; xsf = (ah & 0x80u) != 0u; xzf = (ah & 0x40u) != 0u; xaf = (ah & 0x10u) != 0u; xpf = (ah & 0x04u) != 0u; xcf = (ah & 0x01u) != 0u; }".into(),
            Ins::Alu { op, dst, src: s, size } if !matches!(op, Alu::Adc | Alu::Sbb) => {
                let b = src(s, *size)?;
                let a = rd(dst, *size)?;
                let calc = format!("let bv = {b}; let av = {a}; let r = alu({}u, av, bv, {size}u);", alu_code(*op));
                match op {
                    Alu::Cmp => format!("{{ {calc} }}"),
                    _ => format!("{{ {calc} {} }}", wr(dst, *size, "r")?),
                }
            }
            Ins::Test { a, b, size } => {
                format!("{{ let bv = {}; let av = {}; _ = alu(2u, av, bv, {size}u); }}", src(b, *size)?, rd(a, *size)?)
            }
            Ins::Mov { dst, src: s, size } => {
                let v = src(s, *size)?;
                format!("{{ let v = {v}; {} }}", wr(dst, *size, "v")?)
            }
            Ins::Shift { op: Shift::Rcl, dst: Rm::Reg(r), cnt: Some(1), size: 4 } => {
                format!("{{ let x = rg[{r}]; let out = ((x >> 31u) & 1u) == 1u; rg[{r}] = (x << 1u) | select(0u, 1u, xcf); xcf = out; }}")
            }
            Ins::Shift { op: Shift::Rcr, dst: Rm::Reg(r), cnt: Some(1), size: 4 } => {
                format!("{{ let x = rg[{r}]; let out = (x & 1u) == 1u; rg[{r}] = (x >> 1u) | (select(0u, 1u, xcf) << 31u); xcf = out; }}")
            }
            Ins::Shift { op: sop @ (Shift::Shl | Shift::Shr | Shift::Sar), dst: Rm::Reg(r), cnt: Some(c), size } => {
                let o = match sop {
                    Shift::Shl => 0,
                    Shift::Shr => 1,
                    _ => 2,
                };
                format!("shift_reg({o}u, {r}u, {c}u, {size}u);")
            }
            Ins::Inc(rm, 4) | Ins::Dec(rm, 4) => {
                let o = if matches!(ins, Ins::Inc(..)) { 0 } else { 1 };
                format!("{{ let v = {}; let c = xcf; let r = alu({o}u, v, 1u, 4u); xcf = c; {} }}", rd(rm, 4)?, wr(rm, 4, "r")?)
            }
            _ => return None,
        })
    }
}

/// The record cells as WGSL private globals (for the cells xof all
/// formulas xof a shader); None if a cell has two different types.
pub fn record_globals(formulas: &[&WgslFormula]) -> Option<String> {
    let mut ty: BTreeMap<u32, CellTy> = BTreeMap::new();
    for f in formulas {
        for (c, t) in &f.record {
            if let Some(old) = ty.insert(*c, *t) {
                if old != *t {
                    return None;
                }
            }
        }
    }
    let mut o = String::new();
    for (c, t) in ty {
        match t {
            CellTy::Whole => {
                let _ = writeln!(o, "var<private> r{c}f: R;");
            }
            CellTy::Halves => {
                let _ = writeln!(o, "var<private> r{c}l: u32;");
                let _ = writeln!(o, "var<private> r{c}h: u32;");
            }
        }
    }
    Some(o)
}
