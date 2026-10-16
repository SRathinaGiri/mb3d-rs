//! dIFS formulas written for mb3d-rs in Rust, without x86 code of their own.
//!
//! Their `.m3f` file (in `M3Formulas-rs/`) has the options and a short
//! placeholder `[CODE]`: it only stores a huge distance in `Rout` (so that
//! the original Mandelbulb3D shows nothing instead of crashing), and its
//! bytes carry the formula's name. The program of that code is recognised by
//! its hash and runs the Rust function here instead (as the translated
//! formulas of `native/` do).

use super::*;

/// The hash of `SphereTree.m3f`'s placeholder code (`Prog::hash`).
pub const SPHERE_TREE_HASH: u64 = 0x03f50568c0ace569;
/// The hash of `UlamSpiral.m3f`'s placeholder code.
pub const ULAM_SPIRAL_HASH: u64 = 0x12bc488dcd6d5f0e;

pub(super) fn lookup_difs(hash: u64, _ops: usize) -> Option<DifsFn> {
    match hash {
        SPHERE_TREE_HASH => Some(sphere_tree_difs as DifsFn),
        ULAM_SPIRAL_HASH => Some(ulam_spiral_difs as DifsFn),
        _ => None,
    }
}

/// The GPU version of a formula here, as the lifter's [`WgslFormula`]
/// would be: the function `name(cb)` on the record cells.
pub fn wgsl(hash: u64, name: &str) -> Option<lift_wgsl::WgslFormula> {
    use lift_wgsl::{CellTy, WgslFormula};
    match hash {
        SPHERE_TREE_HASH => Some(WgslFormula {
            code: SPHERE_TREE_WGSL.replace("@NAME@", name),
            // X, Y, Z, Rout, dFree1
            record: [3, 4, 5, 14, 34].iter().map(|&c| (c, CellTy::Whole)).collect(),
            // PackRatio (pConst-16, cell 30) at 0; Rotated and Levels
            // (pConst-24 and -20, cell 29) as the halves at 2 and 3
            // Colour scale (pConst-32, cell 28) at 4
            constants: vec![(30, CellTy::Whole, 0), (29, CellTy::Halves, 2), (28, CellTy::Whole, 4)],
            const_len: 6,
            writes_vars: false,
            dyn_var: None,
        }),
        ULAM_SPIRAL_HASH => Some(WgslFormula {
            code: ULAM_SPIRAL_WGSL.replace("@NAME@", name),
            record: [3, 4, 5, 14, 34].iter().map(|&c| (c, CellTy::Whole)).collect(),
            // Rings and Mode (pConst-16 and -12, cell 30) as the halves at 0
            // and 1; Height (-24, cell 29) at 2, Gap (-32, cell 28) at 4,
            // Colour scale (-40, cell 27) at 6
            constants: vec![(30, CellTy::Halves, 0), (29, CellTy::Whole, 2), (28, CellTy::Whole, 4), (27, CellTy::Whole, 6)],
            const_len: 8,
            writes_vars: false,
            dyn_var: None,
        }),
        _ => None,
    }
}

/// [`ulam_spiral`] in WGSL (the helpers carry the function's name, so that
/// two slots with the formula do not clash).
const ULAM_SPIRAL_WGSL: &str = r#"fn @NAME@_n2(x: i32, y: i32) -> u32 {
    let k = max(abs(x), abs(y));
    if (k == 0) { return 1u; }
    let m = (2 * k - 1) * (2 * k - 1);
    var n: i32;
    if (x == k && y > -k) { n = m + y + k; }
    else if (y == k) { n = m + 2 * k + (k - x); }
    else if (x == -k) { n = m + 4 * k + (k - y); }
    else { n = m + 6 * k + (x + k); }
    return u32(n);
}
fn @NAME@_n3(x: i32, y: i32, z: i32) -> u32 {
    let k = max(max(abs(x), abs(y)), abs(z));
    if (k == 0) { return 1u; }
    let b = u32(2 * k - 1);
    let face = u32((2 * k + 1) * (2 * k + 1));
    let ring = u32(8 * k);
    let u = @NAME@_n2(x, y) - 1u;
    var off: u32;
    if (z == -k) { off = u; }
    else if (z == k) { off = face + b * ring + u; }
    else { off = face + u32(z + k - 1) * ring + (u - b * b); }
    return b * b * b + off + 1u;
}
fn @NAME@_div(n: u32) -> u32 {
    var c = 0u;
    var i = 1u;
    loop {
        if (i * i > n) { break; }
        if (n % i == 0u) { c += select(2u, 1u, i * i == n); }
        i += 1u;
    }
    return c;
}
fn @NAME@_box(q: vec3<R>, b: vec3<R>) -> R {
    let d = abs(q) - b;
    return length(max(d, vec3<R>(0.0))) + min(max(d.x, max(d.y, d.z)), 0.0);
}
fn @NAME@(cb: u32) {
    let rings = i32(cst[cb + 0u]);
    let mode = i32(cst[cb + 1u]);
    let height = cst_r(cb + 2u);
    let gap = cst_r(cb + 4u);
    let cscale = cst_r(cb + 6u);
    let p = vec3<R>(r3f, r4f, r5f);
    let r = R(clamp(rings, 0, select(2000, 120, mode == 3)));
    let hs = 0.5 * (1.0 - clamp(gap, 0.0, 0.95));
    let c = floor(p + vec3<R>(0.5));
    let l = p - c;
    let sg = select(vec3<R>(1.0), vec3<R>(-1.0), l < vec3<R>(0.0));
    let lateral = max(abs(p.x) - (r + 0.5), abs(p.y) - (r + 0.5));
    var bd = R(0.5);
    var bc = R(0.0);
    var d: R;
    if (mode == 3) {
        let out = max(lateral, abs(p.z) - (r + 0.5));
        if (out > 0.5) {
            d = out;
        } else {
            for (var k = 0u; k < 8u; k++) {
                let o = vec3<R>(R(k & 1u), R((k >> 1u) & 1u), R((k >> 2u) & 1u)) * sg;
                let q = c + o;
                if (abs(q.x) > r || abs(q.y) > r || abs(q.z) > r) { continue; }
                let dc = @NAME@_div(@NAME@_n3(i32(q.x), i32(q.y), i32(q.z)));
                if (dc == 2u) {
                    let e = @NAME@_box(l - o, vec3<R>(hs));
                    if (e < bd) { bd = e; bc = log(R(dc)); }
                }
            }
            d = max(bd, out);
        }
    } else if (lateral > 0.5) {
        d = lateral;
    } else {
        let plate = R(0.05);
        var top = max(height, plate);
        if (mode == 1) { top = plate + height * 0.5 * log2(2.0 * r + 1.0); }
        if (mode == 2) { top = 2.0 * hs; }
        if (p.z - top > 0.5) {
            d = p.z - top;
        } else {
            for (var k = 0u; k < 4u; k++) {
                let o = vec2<R>(R(k & 1u), R((k >> 1u) & 1u)) * sg.xy;
                let q = c.xy + o;
                if (abs(q.x) > r || abs(q.y) > r) { continue; }
                let dc = @NAME@_div(@NAME@_n2(i32(q.x), i32(q.y)));
                let lq = l.xy - o;
                var e: R;
                if (mode == 1) {
                    let h = plate + height * 0.5 * log2(R(max(dc, 2u)) / 2.0);
                    e = @NAME@_box(vec3<R>(lq, p.z - h * 0.5), vec3<R>(hs, hs, h * 0.5));
                } else if (mode == 2) {
                    let rad = hs * min(sqrt(R(dc)) / 6.0, 1.0);
                    e = length(vec3<R>(lq, p.z - rad)) - rad;
                } else {
                    var h = plate;
                    if (dc == 2u) { h = height; }
                    e = @NAME@_box(vec3<R>(lq, p.z - h * 0.5), vec3<R>(hs, hs, h * 0.5));
                }
                if (e < bd) { bd = e; bc = log(R(dc)); }
            }
            d = max(bd, lateral);
        }
    }
    r14f = d;
    r34f = bc * cscale;
    rg[4] = rg[4] + 4u;
}
"#;

/// [`sphere_tree`] in WGSL (`R` is the shader's real type).
const SPHERE_TREE_WGSL: &str = r#"fn @NAME@(cb: u32) {
    let k = cst_r(cb + 0u);
    let rotated = cst[cb + 2u] != 0u;
    let levels = min(i32(cst[cb + 3u]), 200);
    var p = vec3<R>(r3f, r4f, r5f);
    let root3 = R(1.7320508075688772);
    let t1 = vec3<R>(root3 * 0.5, -0.5, 0.0);
    let t2 = vec3<R>(-root3 * 0.5, -0.5, 0.0);
    var scale = R(1.0);
    let scl = select(R(1.0), root3, rotated);
    let inner_scale = k * scl / (k * k + scl);
    let inner_scale_b = inner_scale * inner_scale * 0.25;
    let shift = (k * k + scl) / (k * (1.0 + scl));
    let mid = 0.5 * (k + 1.0) / 2.0;
    let buffer_rad = 0.6 * k;
    var i = 0;
    var col = R(0.0);
    var cw = R(1.0);
    loop {
        if (i >= levels) { break; }
        let pb = p - vec3<R>(0.0, 0.0, inner_scale * 0.5);
        if (dot(pb, pb) < inner_scale_b) { break; }
        var max_h = R(0.4);
        if (i == 0) { max_h = R(-100.0); }
        let pc = p - vec3<R>(0.0, 0.0, buffer_rad);
        if (p.z > max_h && dot(pc, pc) > buffer_rad * buffer_rad) { break; }
        let s = inner_scale;
        let p2 = p - vec3<R>(0.0, 0.0, k * 0.5);
        if (dot(p2, p2) < k * k * 0.25) {
            p.z -= k;
            let s2 = k * (k - s);
            let mag2 = dot(p, p);
            p = p * (s2 / mag2);
            scale *= mag2 / s2;
            p.z = -p.z;
            p.z -= k - s;
            let shft = 0.1 * (p.x * p.x + p.y * p.y);
            p.z += shft;
            if (p.z > 0.0) { p.z = p.z - s * floor(p.z / s); }
            p.z -= shft;
            p.z += k - s;
            p.z = -p.z;
            let mag3 = dot(p, p);
            p = p * (s2 / mag3);
            scale *= mag3 / s2;
            p.z += k;
        }
        let pd = p - vec3<R>(0.0, 0.0, mid);
        let sc = dot(p, p);
        let invert = p.z < max_h && dot(pd, pd) > mid * mid;
        scale *= sc;
        p = p / sc;
        if (!invert) {
            p.z -= shift;
            p.z = -p.z;
            p = p * scl;
            scale /= scl;
            p.z += shift;
            if (rotated) {
                let x = p.x;
                let y = p.y;
                p.x = x * 0.8660254037844387 + y * 0.5;
                p.y = -x * 0.5 + y * 0.8660254037844387;
            }
        }
        let h = p.z;
        var x = (0.5 * p.x - root3 * 0.5 * p.y) * 2.0 / root3;
        let cx = floor(x);
        x = x - cx;
        var y = (0.5 * p.x + root3 * 0.5 * p.y) * 2.0 / root3;
        let cy = floor(y);
        y = y - cy;
        var flip = R(0.0);
        if (x + y > 1.0) {
            x = 1.0 - x;
            y = 1.0 - y;
            flip = R(1.0);
        }
        let hv = cx * 0.6180339887 + cy * 0.4142135624 + flip * 0.5;
        col += cw * (hv - floor(hv));
        cw *= 0.35;
        p = x * t1 - y * t2;
        let l0 = dot(p, p);
        let d1 = p - t1;
        let d2 = p + t2;
        let l1 = dot(d1, d1);
        let l2 = dot(d2, d2);
        if (l1 < l0 && l1 < l2) {
            p = p - t1 * (2.0 * dot(t1, p) - 1.0);
        } else if (l2 < l0 && l2 < l1) {
            p = p - t2 * (2.0 * dot(p, t2) + 1.0);
        }
        p.z = h;
        i += 1;
    }
    let q = p - vec3<R>(0.0, 0.0, 0.5 * k);
    r14f = (length(q) - 0.5 * k) * scale;
    r34f = col * cst_r(cb + 4u);
    rg[4] = rg[4] + 4u;
}
"#;

/// `SphereTree.m3f`: options Double PackRatio, Integer Levels, Integer Rotated,
/// Double Colour scale.
fn sphere_tree_difs(m: &mut Machine, _p: &Prog, regs: &mut [u32; 8], _max_steps: u64) -> R<()> {
    let edi = regs[EDI];
    let k = m.rdf64(edi.wrapping_sub(16))?;
    let levels = m.rd32(edi.wrapping_sub(20))? as i32;
    let rotated = m.rd32(edi.wrapping_sub(24))? != 0;
    let colour_scale = m.rdf64(edi.wrapping_sub(32))?;
    let it = crate::custom::IT_BASE;
    use crate::custom::off;
    let p = [m.rdf64(it + off::X)?, m.rdf64(it + off::X + 8)?, m.rdf64(it + off::X + 16)?];
    let (d, size) = sphere_tree(p, levels, k, rotated);
    m.wrf64(it + off::ROUT, d)?;
    m.wrf64(it + off::DFREE1, size * colour_scale)?;
    regs[ESP] = regs[ESP].wrapping_add(4);
    Ok(())
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// GLSL's `mod`: the result has the sign of `y`.
fn fmod(x: f64, y: f64) -> f64 {
    x - y * (x / y).floor()
}

/// TGlad's sphere tree (fractalforums.org, "New sphere tree", June 2020;
/// `SphereTree.frag` in github.com/TGlad/FragmentariumExamples): recursive
/// Eisenstein Ford sphere packings on a Riemann sphere projection. Returns
/// the distance estimate and a colour value per branch (0..1.54: the lattice
/// cells the point passed, the first level weighted most).
pub fn sphere_tree(mut p: [f64; 3], levels: i32, k: f64, rotated: bool) -> (f64, f64) {
    let root3 = 3f64.sqrt();
    let t1 = [root3 / 2.0, -0.5, 0.0];
    let t2 = [-root3 / 2.0, -0.5, 0.0];
    let mut scale = 1.0;
    let scl = if rotated { root3 } else { 1.0 };
    let inner_scale = k * scl / (k * k + scl);
    let inner_scale_b = inner_scale * inner_scale * 0.25;
    let shift = (k * k + scl) / (k * (1.0 + scl));
    let mid = 0.5 * (k + 1.0) / 2.0;
    let buffer_rad = 0.6 * k;
    let (sin_a, cos_a) = (std::f64::consts::PI / 6.0).sin_cos();
    let mut i = 0;
    let mut col = 0.0;
    let mut cw = 1.0;
    while i < levels {
        let pb = [p[0], p[1], p[2] - inner_scale * 0.5];
        if dot(pb, pb) < inner_scale_b {
            break; // definitely inside
        }
        let max_h = if i == 0 { -100.0 } else { 0.4 };
        let pc = [p[0], p[1], p[2] - buffer_rad];
        if p[2] > max_h && dot(pc, pc) > buffer_rad * buffer_rad {
            break; // definitely outside
        }
        let s = inner_scale;
        let r = k / 2.0;
        let p2 = [p[0], p[1], p[2] - k / 2.0];
        if dot(p2, p2) < r * r {
            p[2] -= k;
            let s2 = k * (k - s);
            let mag2 = dot(p, p);
            p = p.map(|v| v * s2 / mag2);
            scale *= mag2 / s2;
            p[2] = -p[2];
            p[2] -= k - s;
            let shft = 0.1 * (p[0] * p[0] + p[1] * p[1]);
            p[2] += shft;
            if p[2] > 0.0 {
                p[2] = fmod(p[2], s);
            }
            p[2] -= shft;
            p[2] += k - s;
            p[2] = -p[2];
            let mag3 = dot(p, p);
            p = p.map(|v| v * s2 / mag3);
            scale *= mag3 / s2;
            p[2] += k;
        }
        let pd = [p[0], p[1], p[2] - mid];
        let sc = dot(p, p);
        if p[2] < max_h && dot(pd, pd) > mid * mid {
            // a sphere inversion
            scale *= sc;
            p = p.map(|v| v / sc);
        } else {
            // stretched onto a plane at zero
            scale *= sc;
            p = p.map(|v| v / sc);
            p[2] -= shift;
            p[2] = -p[2];
            p = p.map(|v| v * scl);
            scale /= scl;
            p[2] += shift;
            if rotated {
                // a twelfth of a revolution
                let (x, y) = (p[0], p[1]);
                p[0] = x * cos_a + y * sin_a;
                p[1] = -x * sin_a + y * cos_a;
            }
        }
        // into the central hexagon (inner radius 0.5) of the triangular lattice
        let h = p[2];
        let x0 = (0.5 * p[0] - root3 / 2.0 * p[1]) * 2.0 / root3;
        let y0 = (0.5 * p[0] + root3 / 2.0 * p[1]) * 2.0 / root3;
        let (cx, cy) = (x0.floor(), y0.floor());
        let (mut x, mut y) = (x0 - cx, y0 - cy);
        let mut flip = 0.0;
        if x + y > 1.0 {
            x = 1.0 - x;
            y = 1.0 - y;
            flip = 1.0;
        }
        // the colour: a value per branch (the lattice cell at each level)
        let hv = cx * 0.6180339887 + cy * 0.4142135624 + flip * 0.5;
        col += cw * (hv - hv.floor());
        cw *= 0.35;
        p = [x * t1[0] - y * t2[0], x * t1[1] - y * t2[1], 0.0];
        // folded into a kite
        let l0 = dot(p, p);
        let d1 = [p[0] - t1[0], p[1] - t1[1], 0.0];
        let d2 = [p[0] + t2[0], p[1] + t2[1], 0.0];
        let (l1, l2) = (dot(d1, d1), dot(d2, d2));
        if l1 < l0 && l1 < l2 {
            let f = 2.0 * dot(t1, p) - 1.0;
            p = [p[0] - t1[0] * f, p[1] - t1[1] * f, 0.0];
        } else if l2 < l0 && l2 < l1 {
            let f = 2.0 * dot(p, t2) + 1.0;
            p = [p[0] - t2[0] * f, p[1] - t2[1] * f, 0.0];
        }
        p[2] = h;
        i += 1;
    }
    let q = [p[0], p[1], p[2] - 0.5 * k];
    ((dot(q, q).sqrt() - 0.5 * k) * scale, col)
}

/// `UlamSpiral.m3f`: options Integer Mode, Integer Rings, Double Height,
/// Double Gap, Double Colour scale.
fn ulam_spiral_difs(m: &mut Machine, _p: &Prog, regs: &mut [u32; 8], _max_steps: u64) -> R<()> {
    let edi = regs[EDI];
    let mode = m.rd32(edi.wrapping_sub(12))? as i32;
    let rings = m.rd32(edi.wrapping_sub(16))? as i32;
    let height = m.rdf64(edi.wrapping_sub(24))?;
    let gap = m.rdf64(edi.wrapping_sub(32))?;
    let colour_scale = m.rdf64(edi.wrapping_sub(40))?;
    let it = crate::custom::IT_BASE;
    use crate::custom::off;
    let p = [m.rdf64(it + off::X)?, m.rdf64(it + off::X + 8)?, m.rdf64(it + off::X + 16)?];
    let (d, col) = ulam_spiral(p, mode, rings, height, gap);
    m.wrf64(it + off::ROUT, d)?;
    m.wrf64(it + off::DFREE1, col * colour_scale)?;
    regs[ESP] = regs[ESP].wrapping_add(4);
    Ok(())
}

/// The number at the cell (x, y) of Ulam's square spiral: 1 in the middle,
/// 2 to the right of it, then counter-clockwise.
pub fn ulam_number(x: i64, y: i64) -> u64 {
    let k = x.abs().max(y.abs());
    if k == 0 {
        return 1;
    }
    let m = (2 * k - 1) * (2 * k - 1);
    let n = if x == k && y > -k {
        m + y + k
    } else if y == k {
        m + 2 * k + (k - x)
    } else if x == -k {
        m + 4 * k + (k - y)
    } else {
        m + 6 * k + (x + k)
    };
    n as u64
}

/// The number of divisors of n (1 for 1, 2 for a prime).
pub fn divisor_count(n: u64) -> u32 {
    let mut c = 0;
    let mut i = 1;
    while i * i <= n {
        if n % i == 0 {
            c += if i * i == n { 1 } else { 2 };
        }
        i += 1;
    }
    c
}

fn sd_box(q: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [q[0].abs() - b[0], q[1].abs() - b[1], q[2].abs() - b[2]];
    let o = (d[0].max(0.0).powi(2) + d[1].max(0.0).powi(2) + d[2].max(0.0).powi(2)).sqrt();
    o + d[0].max(d[1]).max(d[2]).min(0.0)
}

/// The number at the cube (x, y, z) of a spiral over cube shells: shell k
/// (the cubes with max(|x|, |y|, |z|) = k) follows the numbers up to
/// (2k-1)^3; its bottom face is numbered as the square spiral, then the
/// ring of each layer above it, then its top face.
pub fn ulam_number_3d(x: i64, y: i64, z: i64) -> u64 {
    let k = x.abs().max(y.abs()).max(z.abs());
    if k == 0 {
        return 1;
    }
    let before = ((2 * k - 1) as u64).pow(3);
    let face = ((2 * k + 1) * (2 * k + 1)) as u64;
    let ring = 8 * k as u64;
    let u = ulam_number(x, y) - 1;
    let off = if z == -k {
        u
    } else if z == k {
        face + (2 * k - 1) as u64 * ring + u
    } else {
        face + (z + k - 1) as u64 * ring + (u - ((2 * k - 1) * (2 * k - 1)) as u64)
    };
    before + off + 1
}

/// Ulam's prime spiral in 3D. Modes 0..2: the numbers on a grid of unit
/// cells in the xy plane (z up, from 0), one shape per cell. 0: a plate per
/// number and a tower of `height` on each prime; 1: a column per number,
/// higher the more divisors it has (primes are the low streets); 2: a
/// sphere per number, larger the more divisors it has. 3: the numbers on
/// cube shells ([`ulam_number_3d`]), a cube on each prime. `gap`: the space
/// between the shapes (part of a cell). Returns the distance estimate and
/// the colour value (the logarithm of the divisor count of the nearest
/// shape: 0 for 1, 0.69 for the primes).
///
/// Every shape stays in its cell, so the shapes of the cells beyond the
/// ones next to the point are at least half a cell away: the distance is
/// the minimum over the 4 (mode 3: 8) cells nearest to the point, at most
/// 0.5.
pub fn ulam_spiral(p: [f64; 3], mode: i32, rings: i32, height: f64, gap: f64) -> (f64, f64) {
    let r = rings.clamp(0, if mode == 3 { 120 } else { 2000 }) as f64;
    let hs = 0.5 * (1.0 - gap.clamp(0.0, 0.95));
    let c = [(p[0] + 0.5).floor(), (p[1] + 0.5).floor(), (p[2] + 0.5).floor()];
    let l = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    let s = l.map(|v| if v < 0.0 { -1.0 } else { 1.0 });
    let lateral = (p[0].abs() - (r + 0.5)).max(p[1].abs() - (r + 0.5));
    let mut best = (0.5f64, 0.0f64);
    if mode == 3 {
        let out = lateral.max(p[2].abs() - (r + 0.5));
        if out > 0.5 {
            return (out, 0.0);
        }
        for k in 0..8 {
            let o = [(k & 1) as f64 * s[0], ((k >> 1) & 1) as f64 * s[1], ((k >> 2) & 1) as f64 * s[2]];
            let q = [c[0] + o[0], c[1] + o[1], c[2] + o[2]];
            if q[0].abs() > r || q[1].abs() > r || q[2].abs() > r {
                continue;
            }
            let dc = divisor_count(ulam_number_3d(q[0] as i64, q[1] as i64, q[2] as i64));
            if dc == 2 {
                let d = sd_box([l[0] - o[0], l[1] - o[1], l[2] - o[2]], [hs, hs, hs]);
                if d < best.0 {
                    best = (d, (dc as f64).ln());
                }
            }
        }
        return (best.0.max(out), best.1);
    }
    if lateral > 0.5 {
        return (lateral, 0.0);
    }
    let plate = 0.05;
    // the highest shape (the divisor count is at most 2 sqrt(n))
    let top = match mode {
        1 => plate + height * 0.5 * (2.0 * r + 1.0).log2(),
        2 => 2.0 * hs,
        _ => height.max(plate),
    };
    if p[2] - top > 0.5 {
        return (p[2] - top, 0.0);
    }
    for k in 0..4 {
        let o = [(k & 1) as f64 * s[0], ((k >> 1) & 1) as f64 * s[1]];
        let q = [c[0] + o[0], c[1] + o[1]];
        if q[0].abs() > r || q[1].abs() > r {
            continue;
        }
        let dc = divisor_count(ulam_number(q[0] as i64, q[1] as i64));
        let (lx, ly) = (l[0] - o[0], l[1] - o[1]);
        let d = match mode {
            1 => {
                let h = plate + height * 0.5 * (dc.max(2) as f64 / 2.0).log2();
                sd_box([lx, ly, p[2] - h * 0.5], [hs, hs, h * 0.5])
            }
            2 => {
                let rad = hs * ((dc as f64).sqrt() / 6.0).min(1.0);
                (lx * lx + ly * ly + (p[2] - rad) * (p[2] - rad)).sqrt() - rad
            }
            _ => {
                let h = if dc == 2 { height } else { plate };
                sd_box([lx, ly, p[2] - h * 0.5], [hs, hs, h * 0.5])
            }
        };
        if d < best.0 {
            best = (d, (dc as f64).ln());
        }
    }
    (best.0.max(lateral), best.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_tree_placeholder_is_recognised() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("M3Formulas-rs/SphereTree.m3f");
        let def = crate::m3f::M3f::load(&path).unwrap();
        assert_eq!(def.de_option, 20);
        let cf = crate::custom::CustomFormula::new(std::sync::Arc::new(def));
        let m = crate::custom::build_machine(&[Some(&cf)]);
        let p = m.prog(crate::custom::code_addr(0)).unwrap();
        assert_eq!(p.hash, SPHERE_TREE_HASH, "hash {:#018x}", p.hash);
    }

    #[test]
    fn ulam_spiral_placeholder_is_recognised() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("M3Formulas-rs/UlamSpiral.m3f");
        let def = crate::m3f::M3f::load(&path).unwrap();
        assert_eq!(def.de_option, 20);
        let cf = crate::custom::CustomFormula::new(std::sync::Arc::new(def));
        let m = crate::custom::build_machine(&[Some(&cf)]);
        let p = m.prog(crate::custom::code_addr(0)).unwrap();
        assert_eq!(p.hash, ULAM_SPIRAL_HASH, "hash {:#018x}", p.hash);
    }

    #[test]
    fn ulam_numbers() {
        // walking the spiral: right 1, up 1, left 2, down 2, right 3, ...
        let (mut x, mut y, mut n) = (0i64, 0i64, 1u64);
        let dirs = [(1, 0), (0, 1), (-1, 0), (0, -1)];
        let mut d = 0;
        for step in 1..60 {
            for _ in 0..2 {
                for _ in 0..step {
                    x += dirs[d % 4].0;
                    y += dirs[d % 4].1;
                    n += 1;
                    assert_eq!(ulam_number(x, y), n, "({x}, {y})");
                }
                d += 1;
            }
        }
        // the 3D spiral numbers every cube of the shells 0..6 once
        let mut seen = std::collections::HashSet::new();
        for x in -6..=6 {
            for y in -6..=6 {
                for z in -6..=6 {
                    assert!(seen.insert(ulam_number_3d(x, y, z)));
                }
            }
        }
        assert_eq!(seen.len(), 13 * 13 * 13);
        assert_eq!(*seen.iter().max().unwrap(), 13 * 13 * 13);
        assert_eq!(divisor_count(1), 1);
        assert_eq!(divisor_count(97), 2);
        assert_eq!(divisor_count(36), 9);
    }

    #[test]
    fn sphere_tree_distances() {
        // far outside: about the distance to the bounding sphere (radius 0.5 at z 0.5)
        let (d, _) = sphere_tree([0.0, 0.0, 3.0], 6, 1.0, false);
        assert!((d - 2.0).abs() < 0.6, "{d}");
        // positive outside, finite everywhere on a line through the tree
        for k in 0..200 {
            let z = -0.5 + k as f64 * 0.01;
            let (d, _) = sphere_tree([0.13, 0.07, z], 6, 1.0, false);
            assert!(d.is_finite(), "z {z}: {d}");
        }
    }
}
