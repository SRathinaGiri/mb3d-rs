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

pub(super) fn lookup_difs(hash: u64, _ops: usize) -> Option<DifsFn> {
    match hash {
        SPHERE_TREE_HASH => Some(sphere_tree_difs as DifsFn),
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
        _ => None,
    }
}

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
