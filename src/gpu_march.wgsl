// The ray marcher of calc.rs (`Marcher::march_pixel`, MB3D's
// TMandCalcThread.Execute) for the graphics card, in single precision.
// The alternating 3D hybrid of 'Integer Power' and .m3f formulas (lifted
// to WGSL, x86/lift_wgsl.rs; gpu.rs inserts them at the FORMULAS marker
// marker), numerical or analytic DE; no cutting planes, inside rendering
// or volumetric light (gpu.rs checks).
// The structure follows calc.rs closely so the two can be compared line by
// line; the iteration state is kept in private variables like the CPU's
// `Marcher::it`.

@group(0) @binding(0) var<storage, read> P: array<u32>;
@group(0) @binding(1) var<storage, read_write> OUT: array<u32>;
// the constants of the .m3f formulas (all slots) and the CPU's image of
// the iteration record (u32 words from J4)
@group(0) @binding(2) var<storage, read> cst: array<u32>;
@group(0) @binding(3) var<storage, read> rtpl: array<u32>;

// parameter indices (gpu.rs writes them in this order)
const RECT_X0: u32 = 0u;
const RECT_Y0: u32 = 1u;
const RECT_W: u32 = 2u;
const ROW0: u32 = 3u;
const ROWS: u32 = 4u;
const HEIGHT: u32 = 5u;
const OPTIC: u32 = 6u;
const FOV_Y: u32 = 7u;
const FOVX_OFF: u32 = 8u;
const FOVX_MUL: u32 = 9u;
const PL_OPTIC_Z: u32 = 10u;
const VG: u32 = 11u; // 9 values, vgrads[0..3][0..3]
const YSTART: u32 = 20u; // 3 values
const MAX_IT: u32 = 23u;
const MIN_IT: u32 = 24u;
const D_RSTOP: u32 = 25u;
const RSTOP3D: u32 = 26u;
const DO_JULIA: u32 = 27u;
const JU: u32 = 28u; // 3 values
const POWER: u32 = 31u;
const Z_MUL: u32 = 32u;
const DE_STOP: u32 = 33u;
const DE_STOP_FACTOR: u32 = 34u;
const Z_STEP_DIV: u32 = 35u;
const MS_DE_SUB: u32 = 36u;
const MH04ZSD: u32 = 37u;
const DFOG_ON_IT: u32 = 38u;
const FIRST_STEP_RANDOM: u32 = 39u;
const DE_ADD_STEPS: u32 = 40u;
const NORMALS_ON_DE: u32 = 41u;
const SM_NORMALS: u32 = 42u;
const DE_OFFSET: u32 = 43u;
const DE_OFFSET006: u32 = 44u;
const D_DE_SCALE: u32 = 45u;
const ZCORR: u32 = 46u;
const ZC_MUL: u32 = 47u;
const ZEND: u32 = 48u;
const COL_VAR_DE_STOP_MUL: u32 = 49u;
const D_COL_PLUS: u32 = 50u;
const MCTS_M: u32 = 51u;
const STEP_WIDTH: u32 = 52u;
const COLOR_OPTION: u32 = 53u;
const MCT_COLOR_MUL: u32 = 54u;
const LN_RSTOP: u32 = 55u;
const FHLN0: u32 = 56u;
const IS_CUSTOM_DE: u32 = 57u;
const DE_OPTION: u32 = 58u;
const END_TO: u32 = 59u;
const REPEAT_FROM: u32 = 60u;
const J4: u32 = 61u;
// the post calculations (hard shadows, DE ambient occlusion)
const MODE: u32 = 112u;          // M_MARCH, M_SHADOW, M_DEAO
const HS_N: u32 = 113u;          // lights of the shadow pass
const HS_MAX_LEN_MUL: u32 = 114u;
const HS_SOFT_RADIUS: u32 = 115u; // > 0: one soft shadow
const AO_QUALITY: u32 = 116u;
const AO_DITHER: u32 = 117u;
const AO_MAX_LEN: u32 = 118u;
const AO_FIRST_RANDOM: u32 = 119u;
const WIDTH: u32 = 120u;
const HS_LIGHTS: u32 = 124u;     // 6 x (light index, HSvec)
// dIFS and the DE combination (calc.rs DeComb / HybridPart)
const DIFS: u32 = 148u;          // the hybrid is a dIFS hybrid
const START_FROM: u32 = 149u;
const DC_KIND: u32 = 150u;       // 0: no combination; 1 min, 2 max, 3 max inverted, 4 / 5 smooth min, 6 mix
const DC_SMOOTH: u32 = 151u;
const DC_FMIX_POW: u32 = 152u;
const DC_MIX_COL: u32 = 153u;
const P2_START: u32 = 154u;      // the second part
const P2_END: u32 = 155u;
const P2_REPEAT: u32 = 156u;
const P2_MAX_IT: u32 = 157u;
const P2_DE_OPTION: u32 = 158u;
const P2_CUSTOM: u32 = 159u;
const P2_DIFS: u32 = 160u;
const P2_DDS: u32 = 161u;        // d_de_scale of the part
const PROBE_MZZ: u32 = 162u;     // M_PROBE: the depth along the ray
const EVAL_TURNS: u32 = 164u;    // the turns of evaluate's loop (a parameter so that the driver does not unroll it)
const M_MARCH: u32 = 0u;
const M_SHADOW: u32 = 1u;
const M_DEAO: u32 = 2u;
const M_PROBE: u32 = 3u;         // the DE at the ray start (a check against the CPU)
// 6 slots of 8 words: iterations, uncounted, constants base, fHln,
// Integer Power power, z multiplier, kind (0 none, 1 Integer Power, 2 .m3f,
// 3 Amazing Box: see amazing_box)
const SLOTS: u32 = 64u;
fn slot(n: u32, k: u32) -> u32 { return P[SLOTS + n * 8u + k]; }

const PI: f32 = 3.14159265358979;

// Pipeline constants: a pipeline per kind of pass (0 the march, 1 the
// shadows, 2 the ambient occlusion) and per scene feature, so that the
// driver compiles only the states and loops a pass uses (the whole state
// machine in one function compiles ten times slower).
override MODE_KIND: u32 = 0u;
override HAS_DIFS: bool = false;
override HAS_DECOMB: bool = false;

fn pf(i: u32) -> f32 { return bitcast<f32>(P[i]); }
fn pi(i: u32) -> i32 { return bitcast<i32>(P[i]); }
fn pv(i: u32) -> vec3<f32> { return vec3<f32>(pf(i), pf(i + 1u), pf(i + 2u)); }

// iteration state (Marcher::it)
var<private> c: vec3<f32>;
var<private> v: vec3<f32>;
var<private> rout: f32;
// log(rout) of the last iteration, finite where rout overflows single
// precision (orbits that escaped far: |v| beyond 1e19)
var<private> lrout: f32;
var<private> rold: f32;
var<private> otrap: f32;
var<private> it_result: i32;
var<private> max_it: i32;
var<private> rstop: f32;
var<private> calc_sit: bool;
var<private> smooth_it: f32;
// the rest of the iteration record the formulas see (Iteration)
var<private> w: f32;
var<private> j: vec3<f32>;
var<private> j4: f32;
var<private> vary_scale: f32;
var<private> first_it: i32;
var<private> deriv: vec3<f32>;
var<private> dfree: vec2<f32>;
// marcher state
var<private> ms_de_stop: f32;
// the hybrid part being iterated (set_part)
var<private> part_start: u32;
var<private> part_end: u32;
var<private> part_repeat: u32;
var<private> de_option: i32;
// the absolute DE stop of a dIFS loop (RStopD)
var<private> rstopd: f32;
// the parts' own results of the last DE combination (M_PROBE)
var<private> dbg_r1: f32;
var<private> dbg_it1: i32;
var<private> dbg_rst: f32;
var<private> dbg_it2: i32;
var<private> dbg_rout: f32;
var<private> dbg_g: f32;
var<private> mzz: f32;
var<private> vfov: vec3<f32>;
var<private> base: vec3<f32>;
var<private> max_its_result: i32;
var<private> s_roughness: f32;
var<private> seed: i32;
// the result (SiLight)
var<private> si_normal: vec3<i32>;
var<private> si_zpos: u32;
var<private> si_shadow: u32;
var<private> si_grad: u32;
var<private> si_otrap: u32;
var<private> si_amb: u32;
// the rays of the ambient occlusion
var<private> ao_dirs: array<vec3<f32>, 33>;
var<private> ao_min: array<f32, 33>;

fn is_nan(x: f32) -> bool {
    let b = bitcast<u32>(x) & 0x7FFFFFFFu;
    return b > 0x7F800000u;
}

// min_max_clip_15bit
fn clip15(s: f32) -> u32 {
    if (is_nan(s) || s < 0.0) { return 0u; }
    if (s > 32767.0) { return 32767u; }
    return u32(round(s));
}

// the position on the ray (CPU: c, moved with add_weight)
fn setc() {
    c = base + vfov * mzz;
}

// fHIntFunctions[power] (formulas.rs int_pow)
fn int_pow(p: vec3<f32>, j: vec3<f32>, power: i32, zmul: f32) -> vec3<f32> {
    let x = p.x;
    let y = p.y;
    let z = p.z;
    let EPS = 1e-30;
    var o: vec3<f32>;
    if (power <= 2) {
        let yy = y * y;
        let xx = x * x;
        let r = xx + yy;
        o.z = (sqrt(r) * zmul * z) * 2.0 + j.z;
        let a = (r - z * z) / r;
        o.x = (xx - yy) * a + j.x;
        o.y = (x * y * a) * 2.0 + j.y;
    } else if (power == 3) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let a = 1.0 - 3.0 * sz / (r + EPS);
        o.x = (sx - 3.0 * sy) * a * x + j.x;
        o.z = j.z - (sz - 3.0 * r) * z * zmul;
        o.y = a * (3.0 * sx - sy) * y + j.y;
    } else if (power == 4) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let a = 1.0 + sz * (sz - 6.0 * r) / (r * r + EPS);
        o.x = (sx * (sx - 6.0 * sy) + sy * sy) * a + j.x;
        o.z = sqrt(r) * (r - sz) * z * 4.0 * zmul + j.z;
        o.y = a * x * (sx - sy) * 4.0 * y + j.y;
    } else if (power == 5) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let a = 1.0 + (sz * sz - sz * r * 2.0) * 5.0 / (r * r + EPS);
        o.y = (5.0 * sx * sx - sy * (10.0 * sx - sy)) * a * y + j.y;
        o.z = (sz * (sz - 10.0 * r) + 5.0 * r * r) * z * zmul + j.z;
        o.x = a * x * (sx * (sx - 10.0 * sy) + 5.0 * sy * sy) + j.x;
    } else if (power == 6) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let rr = r * r;
        let a = 1.0 - sz * (15.0 * rr + sz * (sz - r * 15.0)) / (rr * r + EPS);
        o.y = (3.0 * sy * sy + sx * (3.0 * sx - 10.0 * sy)) * a * y * x * 2.0 + j.y;
        o.z = (sz * (3.0 * sz - 10.0 * r) + 3.0 * rr) * sqrt(r) * z * zmul * 2.0 + j.z;
        o.x = (sy * sy * (15.0 * sx - sy) + sx * sx * (sx - 15.0 * sy)) * a + j.x;
    } else if (power == 7) {
        let sy = y * y;
        let sx = x * x;
        let r = sx + sy;
        let sz = z * z;
        let rr = r * r;
        let a = 1.0 - 7.0 * (sz * (3.0 * rr + sz * (sz - 5.0 * r))) / (rr * r + EPS);
        o.y = (sx * (21.0 * sy * sy + sx * (7.0 * sx - 35.0 * sy)) - sy * sy * sy) * a * y + j.y;
        o.z = j.z - (sz * sz * sz - 7.0 * r * (rr + sz * (3.0 * sz - 5.0 * r))) * z * zmul;
        o.x = a * x * (sx * (35.0 * sy * sy + sx * (sx - 21.0 * sy)) - 7.0 * sy * sy * sy) + j.x;
    } else {
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let r = xx + yy;
        let rr = r * r;
        let zzzz = zz * zz;
        let t = (zzzz - 6.0 * r * zz + rr) * (zz - r) * sqrt(r);
        o.z = -(t * z * 8.0 * zmul) + j.z;
        let num = (rr * 70.0 + zzzz) * zzzz - 28.0 * zz * r * (zzzz + rr);
        let a = num / (rr * rr + EPS) + 1.0;
        let xxxx = xx * xx;
        let yyyy = yy * yy;
        o.y = (yyyy * (7.0 * xx - yy) + xxxx * (xx - 7.0 * yy)) * 8.0 * x * y * a + j.y;
        o.x = a * (xxxx * xxxx + yyyy * (70.0 * xxxx + yyyy) - 28.0 * (xx * yy * (yyyy + xxxx))) + j.x;
    }
    return o;
}

// HybridCube / HybridCubeDE (formulas.rs amazing_box); slot words: 4 scale,
// 5 min R, 7 fold, 2 analytic DE (the running derivative in w)
fn box_fold(x: f32, fold: f32) -> f32 {
    return abs(x + fold) - (abs(x - fold) + x);
}

fn amazing_box(n: u32) {
    let scale = bitcast<f32>(slot(n, 4u));
    let min_r = bitcast<f32>(slot(n, 5u));
    let fold = bitcast<f32>(slot(n, 7u));
    let x = box_fold(v.x, fold);
    let y = box_fold(v.y, fold);
    let z = box_fold(v.z, fold);
    let r = z * z + y * y + x * x;
    let sqr_min_r = min_r * min_r;
    var mul: f32;
    if (r < sqr_min_r) {
        mul = scale / sqr_min_r;
    } else if (1.0 < r) {
        mul = scale;
    } else {
        mul = scale / r;
    }
    if (slot(n, 2u) != 0u) {
        w *= mul;
    }
    v = vec3<f32>(x * mul + j.x, y * mul + j.y, z * mul + j.z);
}

// CalcSmoothIterations (fHln of the last formula n)
fn calc_smooth_iterations(n: u32) {
    if (rout <= 1.0) {
        smooth_it = f32(it_result);
    } else if (rold < 1.0) {
        let d = log(0.5 * lrout) * bitcast<f32>(slot(n, 3u));
        smooth_it = f32(it_result) + pf(LN_RSTOP) - d;
    } else {
        let d = log(0.5 * lrout);
        let e = d - log(0.5 * log(rold));
        smooth_it = (pf(LN_RSTOP) - d) / (e + 1e-30) + f32(it_result);
    }
}

//@FORMULAS@

// doHybridPas / doHybridPasDE (Iteration::run, hybrid_3d, hybrid_3d_de):
// the alternating hybrid; `de`: with the analytic DE (returned)
fn hybrid(de: bool) -> f32 {
    if (pi(DO_JULIA) != 0) {
        j = pv(JU);
    } else {
        j = c;
    }
    v = c;
    w = 0.0;
    rout = dot(c, c);
    otrap = rout;
    let deo = de_option;
    if (de) {
        if ((deo & 0x38) == 16) {
            w = rout;
        } else if ((deo & 0x38) == 32) {
            deriv = vec3<f32>(1.0, 0.0, 0.0);
        } else {
            w = 1.0;
        }
    }
    var n = part_start;
    var btmp = max(bitcast<i32>(slot(n, 0u)), 0);
    it_result = 0;
    first_it = 0;
    var uncounted_guard = 0u;
    loop {
        rold = rout;
        var guard = 0;
        while (btmp <= 0) {
            n += 1u;
            if (n > part_end) {
                n = part_repeat;
            }
            btmp = max(bitcast<i32>(slot(n, 0u)), 0);
            guard += 1;
            if (guard > 64) {
                break;
            }
        }
        if (btmp <= 0) {
            break;
        }
        call_slot(n);
        btmp -= 1;
        if (slot(n, 1u) != 0u) {
            uncounted_guard += 1u;
            if (uncounted_guard > 100000u) {
                break;
            }
            continue;
        }
        it_result += 1;
        rout = v.x * v.x + v.y * v.y + v.z * v.z;
        if (rout < otrap) {
            otrap = rout;
        }
        if (it_result >= max_it || rout > rstop) {
            break;
        }
    }
    let mabs = max(abs(v.x), max(abs(v.y), abs(v.z)));
    if (mabs > 1e18) {
        let sv = v / mabs;
        lrout = log(dot(sv, sv)) + 2.0 * log(mabs);
    } else {
        lrout = log(rout);
    }
    var result = 0.0;
    if (de) {
        if ((deo & 0x38) == 32) {
            result = sqrt(rout) * 0.5 * log(rout) / deriv.x;
        } else if ((deo & 7) == 4) {
            result = abs(v.y) * log(abs(v.y)) / w;
        } else {
            result = sqrt(rout) / abs(w);
        }
    }
    if (calc_sit) {
        calc_smooth_iterations(n);
    }
    return result;
}

// PrepareF1DEcomb / RestoreF1DEcomb: the hybrid part to iterate
fn set_part(part: u32) {
    if (part == 2u) {
        part_start = P[P2_START];
        part_end = P[P2_END];
        part_repeat = P[P2_REPEAT];
        max_it = pi(P2_MAX_IT);
        de_option = pi(P2_DE_OPTION);
    } else {
        part_start = P[START_FROM];
        part_end = P[END_TO];
        part_repeat = P[REPEAT_FROM];
        max_it = pi(MAX_IT);
        de_option = pi(DE_OPTION);
    }
}

// doHybridIFS3D (Iteration::hybrid_difs): the dIFS loop of the part; each
// counted iteration leaves a distance estimate in rout (relative to
// vary_scale), the smallest is the result.  `vec_ini`: start from c
// (else from the vector of the preceding part).
fn hybrid_difs(vec_ini: bool, rs: f32) -> f32 {
    if (vec_ini) {
        if (pi(DO_JULIA) != 0) {
            j = pv(JU);
        } else {
            j = c;
        }
        v = c;
    }
    rstopd = rs;
    rold = rs;
    it_result = 0;
    first_it = 0;
    vary_scale = 1.0;
    dfree.x = 0.0;
    deriv.x = 1.0;
    difs_init();
    var min_de = 65535.0;
    var dfree2 = 0.0;
    var it_min = 0;
    var it = 0;
    var n = min(part_start, 5u);
    var cnt = max(bitcast<i32>(slot(n, 0u)), 0);
    var guard = 0u;
    loop {
        var g = 0u;
        while (cnt <= 0) {
            n += 1u;
            if (n > part_end) {
                n = part_repeat;
            }
            cnt = max(bitcast<i32>(slot(n, 0u)), 0);
            g += 1u;
            if (g > 64u) {
                break;
            }
        }
        if (cnt <= 0) {
            break;
        }
        call_slot_difs(n, u32(cnt));
        // the formula's ecx and ebx: the remaining count and the slot
        cnt = bitcast<i32>(rg[1]) - 1;
        if (rg[3] < 6u) {
            n = rg[3];
        }
        guard += 1u;
        if (guard > 1000000u) {
            break;
        }
        if (slot(n, 1u) != 0u) {
            continue;
        }
        it += 1;
        it_result = it;
        let de = rout / vary_scale;
        if (de < min_de) {
            it_min = it;
            dfree2 = dfree.x;
            min_de = de;
            if (de < rs) {
                break;
            }
        }
        if (it >= max_it) {
            break;
        }
    }
    it_result = it_min;
    smooth_it = f32(it_min);
    otrap = dfree2;
    return min_de;
}

// The one place that iterates the formulas: the driver inlines functions,
// so `hybrid` and `hybrid_difs` (and through them the formula code) must
// have a single textual call site each.
fn run_part(difs: bool, de: bool, vec_ini: bool, rs: f32) -> f32 {
    if (HAS_DIFS && difs) {
        return hybrid_difs(vec_ini, rs);
    }
    return hybrid(de);
}

// Marcher::calc_de (`on_de`: the DE of the hybrid parts, combined as the
// DE combination says) or mMandFunction with calc_sit (the smoothed
// iteration count).  Written as one loop with one `run_part` site: each
// turn iterates a part once (CalcDEanalytic, or CalcDEnoADE's first
// iteration and its three gradient samples, or the dIFS loop).
fn evaluate(on_de: bool) -> f32 {
    let kind = P[DC_KIND];
    let nparts = select(1u, 2u, HAS_DECOMB && on_de && kind != 0u);
    var part = 1u;
    var part_begin = true;
    // the part
    var difs = false;
    var custom = false;
    var dds = 0.0;
    var invert = false;
    var no_vec_ini = false;
    var k = 0u;
    var result = 0.0;
    var bc = c;
    var buf_rout = 0.0;
    var buf_lr = 0.0;
    var buf_sit = false;
    var buf_max_it = 0;
    var g = 0.0;
    // the first part's values for the combination
    var r1 = 0.0;
    var wt = 0.0;
    var dt = 0.0;
    var buf_it = 0;
    var buf_otrap = 0.0;
    var buf_sit1 = 0.0;
    var buf_mir = 0;
    for (var guard = 0u; guard < P[EVAL_TURNS]; guard++) {
        if (part_begin) {
            part_begin = false;
            set_part(part);
            if (part == 2u) {
                difs = pi(P2_DIFS) != 0;
                custom = pi(P2_CUSTOM) != 0;
                dds = pf(P2_DDS);
                invert = kind == 3u;
                no_vec_ini = kind == 6u;
            } else {
                difs = pi(DIFS) != 0;
                custom = pi(IS_CUSTOM_DE) != 0;
                dds = pf(D_DE_SCALE);
                invert = false;
                no_vec_ini = false;
            }
            k = 0u;
            bc = c;
            g = 0.0;
        }
        if (k > 0u) {
            var o = vec3<f32>(0.0);
            o[k - 1u] = pf(DE_OFFSET);
            c = bc + o;
        }
        let hres = run_part(difs, on_de && custom, !no_vec_ini, ms_de_stop * pf(STEP_WIDTH) * 1.03);
        if (!on_de) {
            return smooth_it;
        }
        var part_done = true;
        if (difs) {
            // DE option 20: the absolute DE stop as RStopD
            result = hres * dds;
            if (is_nan(result)) {
                result = ms_de_stop * 0.25;
            }
            // dIFS never counts as "inside by iterations"
            max_its_result = max_it + 1;
            if (invert) {
                result = ms_de_stop * 2.0 - result;
            }
        } else {
            if (custom) {
                result = hres * dds;
            } else if (k == 0u) {
                // the numerical gradient of the iteration count
                if (rout <= 0.0) {
                    result = 0.0;
                } else {
                    buf_rout = rout;
                    buf_lr = lrout;
                    buf_sit = calc_sit;
                    buf_max_it = max_it;
                    calc_sit = false;
                    max_it = it_result;
                    rstop = pf(RSTOP3D);
                    k = 1u;
                    part_done = false;
                }
            } else {
                // the differences relative to rout, from the logarithms:
                // rout itself overflows for orbits that escaped far
                let d = 1.0 - exp(lrout - buf_lr);
                g += d * d;
                k += 1u;
                if (k < 4u) {
                    part_done = false;
                } else {
                    c = bc;
                    dbg_rout = buf_rout;
                    dbg_g = g;
                    // = buf_rout * log(buf_rout) * dds / (sqrt(g * buf_rout^2) + de_offset006)
                    result = buf_lr * dds / (sqrt(g) + pf(DE_OFFSET006) * exp(-buf_lr));
                    rout = buf_rout;
                    it_result = max_it;
                    max_it = buf_max_it;
                    calc_sit = buf_sit;
                    rstop = pf(D_RSTOP);
                }
            }
            if (part_done) {
                max_its_result = max_it;
                let mn = ms_de_stop * 0.25;
                if (result < mn || is_nan(result)) {
                    result = mn;
                }
                if (invert) {
                    result = ms_de_stop * 4.0 - result * 3.0;
                    if (result >= ms_de_stop) {
                        it_result -= 1;
                    }
                }
            }
        }
        if (!part_done) {
            continue;
        }
        if (part == 1u) {
            if (nparts == 1u) {
                return result;
            }
            // PrepareF1DEcomb
            r1 = result;
            dbg_r1 = result;
            dbg_it1 = it_result;
            wt = abs(w);
            dt = abs(deriv.x);
            buf_it = it_result;
            buf_otrap = otrap;
            buf_sit1 = smooth_it;
            buf_mir = max_its_result;
            part = 2u;
            part_begin = true;
            continue;
        }
        // CalcDEfull with FormulaType > 0: the second part combined
        let rst = result;
        dbg_rst = rst;
        dbg_it2 = it_result;
        result = r1;
        let ms = ms_de_stop;
        var restore = false;
        switch (kind) {
            case 1u: {
                if (rst >= result) {
                    restore = true;
                } else {
                    result = rst;
                }
            }
            case 2u, 3u: {
                if (rst > result) {
                    result = rst;
                } else {
                    restore = true;
                }
            }
            case 4u, 5u: {
                if (!(rst < result)) {
                    it_result = buf_it;
                    max_its_result = buf_mir;
                }
                let sm = pf(DC_SMOOTH);
                if (kind == 4u) {
                    result = min(rst - max(sm - result, 0.0), result - max(sm - rst, 0.0));
                } else {
                    let rd1 = max(sm - result - ms, 0.0);
                    let d2 = max(sm - rst - ms, 0.0);
                    result = min(rst - rd1 * (sm - d2) / sm, result - d2 * (sm - rd1) / sm);
                }
                let r2 = abs(rst);
                let ww = abs(result);
                let f = 1.0 / (ww + r2 + 1e-10);
                smooth_it = (smooth_it * ww + buf_sit1 * r2) * f;
                otrap = (otrap * ww + buf_otrap * r2) * f;
            }
            default: {
                // mix: part 1 first, then the dIFS part for the distance
                let mc = P[DC_MIX_COL];
                if (mc == 0u) {
                    smooth_it += buf_sit1;
                    otrap = (otrap + buf_otrap) * 0.5;
                } else if (mc == 1u) {
                    smooth_it = buf_sit1;
                    otrap = buf_otrap;
                }
                let deo = pi(DE_OPTION);
                if (deo == 5 || deo == 6) {
                    result = rst / dt;
                } else if (deo == 2 || deo == 11) {
                    result = rst / wt;
                } else {
                    result = rst * pow(abs(pf(DC_FMIX_POW)), f32(-1 - buf_it));
                }
            }
        }
        if (restore) {
            it_result = buf_it;
            smooth_it = buf_sit1;
            otrap = buf_otrap;
            max_its_result = buf_mir;
        }
        set_part(1u);
        return result;
    }
    return ms_de_stop * 0.25;
}

fn vg(i: u32) -> vec3<f32> {
    return pv(VG + i * 3u);
}

// rotate_vector_reverse(v, vgrads)
fn rot_rev(a: vec3<f32>) -> vec3<f32> {
    return a.x * vg(0u) + a.y * vg(1u) + a.z * vg(2u);
}

fn update_de_stop() {
    ms_de_stop = pf(DE_STOP) * (1.0 + mzz * pf(DE_STOP_FACTOR));
}

// MakeWNormalsFromDVec
fn store_normal(n: vec3<f32>) {
    let d = 32767.0 / sqrt(dot(n, n) + 1e-30);
    var o: vec3<i32>;
    for (var k = 0; k < 3; k++) {
        let x = round(n[k] * d);
        if (is_nan(x)) {
            o[k] = 0;
        } else {
            o[k] = i32(clamp(x, -32767.0, 32767.0));
        }
    }
    if (o.x == 0 && o.y == 0) {
        o.z = select(-32767, 32767, o.z > 0);
    }
    si_normal = o;
}

// RMCalcRoughness
fn calc_roughness(n: vec3<f32>, dt2: f32, dsg: f32) -> f32 {
    let a = (dsg * 7.0 * dt2 * dt2 + 1e-30) / (dot(n, n) + 1e-30);
    return clamp(sqrt(max(a, 0.0)) - 0.05, 0.0, 1.0);
}

// create_xy_vecs_from_normals
fn xy_vecs(n: vec3<f32>, vx: ptr<function, vec3<f32>>, vy: ptr<function, vec3<f32>>) {
    let d = n.y * n.y + n.x * n.x;
    if (d < 1e-30) {
        *vx = vec3<f32>(1.0, 0.0, 0.0);
        *vy = vec3<f32>(0.0, 1.0, 0.0);
        return;
    }
    let a = acos(clamp(-n.z / sqrt(d + n.z * n.z + 1e-30), -1.0, 1.0)) * 0.5;
    let sin_a = sin(a) / sqrt(d);
    let cos_a = cos(a);
    let dd = -n.y * sin_a;
    let d1 = n.x * sin_a;
    let x = vec3<f32>(1.0 - 2.0 * d1 * d1, 2.0 * dd * d1, 2.0 * d1 * cos_a);
    *vx = x;
    *vy = vec3<f32>(x.y, 1.0 - 2.0 * dd * dd, -2.0 * dd * cos_a);
}

// RMdoColor
fn do_color() {
    let x = v.x;
    let y = v.y;
    let z = v.z;
    var s: f32;
    switch (pi(COLOR_OPTION)) {
        case 1: {
            s = log(rout / (rold + 1.0)) * pf(MCT_COLOR_MUL);
        }
        case 2: {
            s = (atan2(y - c.y, x - c.x) + PI) * 5200.0;
        }
        case 3: {
            s = (atan2(z - c.z, x - c.x) + PI) * 5200.0;
        }
        case 4: {
            s = (atan2(z - c.z, y - c.y) + PI) * 5200.0;
        }
        case 5: {
            s = (atan2(x, y) + PI) * 5215.0;
            let r = sqrt(x * x + y * y + 1e-30 + z * z);
            let s2 = (PI + asin(clamp(z / r, -1.0, 1.0)) * 2.0) * 5215.0;
            si_grad = clip15(s2);
        }
        default: {
            s = otrap * 4096.0;
        }
    }
    si_otrap = clip15(s);
}

fn calc_zpos_and_rough() {
    let zz = max(mzz, 0.0);
    // zc_mul * (sqrt(zz * zcorr + 1) - 1) without the cancellation
    let a = zz * pf(ZCORR);
    let vv = round(pf(ZC_MUL) * a / (sqrt(a + 1.0) + 1.0));
    let itmp = u32(clamp(8388352.0 - vv, 0.0, 8388352.0));
    var r = itmp << 8u;
    if (pi(SM_NORMALS) > 0) {
        r |= u32(round(s_roughness * 255.0)) & 0xFFu;
    }
    si_zpos = r;
}

// deao.rs get_rand
fn get_rand() -> f32 {
    seed = seed * 0x343FD + 0x269EC3;
    return f32((u32(seed) >> 8u) & 0x7FFFFFu) / f32(0x7FFFFFu);
}

// -BuildRotMatrixS(0, ya, za)[2]
fn ray_dir(ya: f32, za: f32) -> vec3<f32> {
    return vec3<f32>(sin(ya) * cos(za), -sin(ya) * sin(za), -cos(ya));
}

// MakeRotQuatFromSNormals + CreateSMatrixFromQuat (deao.rs normal_matrix)
fn normal_matrix(n: vec3<f32>) -> mat3x3<f32> {
    let a = acos(clamp(-n.z, -1.0, 1.0)) * 0.5;
    var sa = sin(a);
    let ca = cos(a);
    let nn = sqrt(n.y * n.y + n.x * n.x);
    var q = vec4<f32>(0.0, 0.0, 0.0, ca);
    if (nn >= 1e-25) {
        sa /= nn;
        q = vec4<f32>(-n.y * sa, n.x * sa, 0.0, ca);
    }
    // the rows of the CPU's matrix (m[i] dot d)
    return mat3x3<f32>(
        vec3<f32>(1.0 - 2.0 * (q.y * q.y + q.z * q.z), 2.0 * (q.x * q.y + q.z * q.w), 2.0 * (q.x * q.z - q.y * q.w)),
        vec3<f32>(2.0 * (q.x * q.y - q.z * q.w), 1.0 - 2.0 * (q.x * q.x + q.z * q.z), 2.0 * (q.z * q.y + q.x * q.w)),
        vec3<f32>(2.0 * (q.x * q.z + q.y * q.w), 2.0 * (q.y * q.z - q.x * q.w), 1.0 - 2.0 * (q.x * q.x + q.y * q.y)));
}

// MaxLHS
fn hs_max_len(y: i32) -> f32 {
    return f32(pi(WIDTH) + y) * 0.6 * (1.0 + 0.5 * min(mzz, pf(ZEND) * 0.4) * max(pf(FOV_Y), 0.0) / f32(pi(HEIGHT))) * pf(HS_MAX_LEN_MUL);
}

// states of march_pixel; a state may request an evaluation (`need`, at
// `c`, DE or iteration count by `want_de`), whose result the next state gets
const P_START: u32 = 0u;     // result: the DE at the start
const P_LOOP: u32 = 1u;      // the top of the march loop
const P_HALF: u32 = 2u;      // result: the DE after the half step back
const P_DECIDE: u32 = 3u;    // step or surface
const P_STEP: u32 = 4u;      // result: the DE after a step
const P_BS: u32 = 5u;        // RMdoBinSearch: the loop
const P_BS_R: u32 = 6u;      // result of a binary search step
const P_BSI: u32 = 7u;       // RMdoBinSearchIt: the loop
const P_BSI_1: u32 = 8u;     // result: the first DE of a step
const P_BSI_2: u32 = 9u;     // result: the DE 0.001 back
const P_BSI_END: u32 = 10u;
const P_N_INIT: u32 = 11u;   // normals: the centre sample
const P_N_0: u32 = 12u;      // result: the centre sample
const P_N8: u32 = 13u;       // 5x5x5 samples
const P_N8_R: u32 = 14u;
const P_NA: u32 = 15u;       // the 6 axis samples
const P_NA_R: u32 = 16u;
const P_NSM: u32 = 17u;      // smoothing: the 4 samples around
const P_NSM_R: u32 = 18u;
const P_SW: u32 = 19u;       // smoothing: the sweeps
const P_SW_R: u32 = 20u;
const P_NEND: u32 = 21u;
// the post calculations start on the surface of the calculated pixel
const P_PS: u32 = 22u;       // hs_surface_start / surface_point
const P_PS_R: u32 = 23u;     // result: the DE at the stored depth
// hard_shadow_row / soft_shadow_row
const P_HS_INIT: u32 = 24u;
const P_HS_LIGHT: u32 = 25u; // the next light
const P_HS_R0: u32 = 26u;    // result: the DE at the start of the shadow ray
const P_HS_STEP: u32 = 27u;  // a step towards the light
const P_HS_R: u32 = 28u;     // result: the DE after the step
const P_HS_DONE: u32 = 29u;  // the ray ended (hit or open)
// deao_row
const P_AO_INIT: u32 = 30u;
const P_AO_RAY: u32 = 31u;   // the next ray
const P_AO_STEP: u32 = 32u;  // a step along the ray
const P_AO_R: u32 = 33u;     // result: the DE after the step
const P_AO_CORR: u32 = 34u;  // the correction and the mean


// The march of a pixel (M_MARCH), or a post calculation on the G-buffer
// pixel in si_* (M_SHADOW: the shadow bits, M_DEAO: si_amb); the DE is
// evaluated at the single site of `evaluate`.
fn pixel(x: i32, y: i32, mode: u32) {
    set_part(1u);
    rstop = pf(D_RSTOP);
    max_its_result = max_it;

    // RMCalculateVgradsFOV
    let cafy = (f32(y) / f32(pi(HEIGHT)) - 0.5) * pf(FOV_Y);
    let cafx = (pf(FOVX_OFF) - f32(x + 1)) * pf(FOVX_MUL);
    var dv: vec3<f32>;
    let optic = pi(OPTIC);
    if (optic == 1) {
        dv = normalize(vec3<f32>(-cafx, cafy, pf(PL_OPTIC_Z)));
    } else if (optic == 2) {
        dv = vec3<f32>(-sin(cafx) * cos(cafy), sin(cafy), cos(cafy) * cos(cafx));
    } else {
        dv = normalize(vec3<f32>(-sin(cafx), sin(cafy), cos(cafy) * cos(cafx)));
    }
    vfov = rot_rev(dv);
    // RMCalculateStartPos
    if (optic == 2) {
        base = pv(YSTART);
    } else {
        base = pv(YSTART) + vg(0u) * f32(x) + vg(1u) * f32(y);
    }
    calc_sit = false;
    var step_count = 0.0;
    mzz = 0.0;
    if (mode == M_PROBE) {
        mzz = pf(PROBE_MZZ);
    }
    setc();
    ms_de_stop = pf(DE_STOP);
    var first_step = pi(FIRST_STEP_RANDOM) != 0;
    let dfog_on_it = pi(DFOG_ON_IT);
    let on_de = pi(NORMALS_ON_DE) != 0;
    let sm = pi(SM_NORMALS);

    // the state
    var pc = P_START;
    var need = true;
    var want_de = true;
    if (mode == M_SHADOW || mode == M_DEAO) {
        pc = P_PS;
        need = false;
    }
    var after_bs = P_N_INIT;
    var bs_thr = 0.001;
    var zf = 0.0;
    // shadows
    let soft = pf(HS_SOFT_RADIUS) > 0.0;
    var ic = vec3<f32>(0.0);
    var nvec = vec3<f32>(0.0);
    var max_lhs = 0.0;
    var li = 0u;
    var lidx = 0u;
    var lvec = vec3<f32>(0.0);
    var zz2 = 0.0;
    var zz2mul = 0.0;
    var rsf = 1.0;
    var st = 0.0;
    var zr_soft = 1.0;
    var zrs_mul = 0.0;
    // ambient occlusion
    var ray_count = 0u;
    var k = 0u;
    var step_ao = 0.0;
    var max_dist = 0.0;
    var d_step_mul = 1.0;
    var ms2 = 0.0;
    var de_mul = 1.0;
    var abr_c = 0.0;
    var md_d10 = 0.0;
    var d_min_a_dif = -1.0;
    var corr_w = 0.0;
    var rot_w = mat3x3<f32>();
    var rv0 = vec3<f32>(0.0);
    var rv1 = vec3<f32>(0.0);
    var rv2 = vec3<f32>(0.0);
    var sv = vec3<f32>(0.0);
    var s_tmp = 1.0;
    var b_end = false;
    var b_first = false;
    var r = 0.0;
    var done = false;
    // march
    var dtmp = 0.0;
    var rsf_mul = 1.0;
    var last_step = 0.0;
    var last_de = 0.0;
    var dt1 = 0.0;
    var de_limited = false;
    var itmp = 0;
    // RMdoBinSearchIt
    var yp = 0.0;
    var saved_max = 0;
    var dmul = 1.0;
    var first = true;
    var last_si = 0.0;
    var last_dif = 0.0;
    // normals
    var noffset = 0.0;
    var ct1 = vec3<f32>(0.0);
    var dnn = 0.0;
    var nn = 0.0;
    var n = vec3<f32>(0.0);
    var cnt = 0;
    var na = 0.0;
    var acc = 0.0;
    var ssn = 0.0;
    var dm = 0.0;
    var vx = vec3<f32>(0.0);
    var vy = vec3<f32>(0.0);
    var s1 = vec2<f32>(0.0);
    var ds = vec2<f32>(0.0);
    var axis = 0;
    var kk = 0;

    for (var guard = 0; guard < 2000000; guard++) {
        if (need) {
            r = evaluate(want_de);
            need = false;
        }
        switch (pc) {
            case P_START: {
                if (MODE_KIND == 0u) {
                    dtmp = r;
                    if (mode == M_PROBE) {
                        si_zpos = bitcast<u32>(r);
                        si_grad = u32(it_result) & 0xFFFFu;
                        let rb = bitcast<u32>(dbg_rst);
                        si_normal = vec3<i32>(i32(rb << 16u) >> 16u, i32(rb) >> 16u, dbg_it2);
                        if (P[PROBE_MZZ + 1u] != 0u) {
                            // the details of the numerical DE of part 1 instead
                            let gb = bitcast<u32>(dbg_g);
                            si_normal = vec3<i32>(i32(gb << 16u) >> 16u, i32(gb) >> 16u, i32(round(log(dbg_rout))));
                        }
                        let rb1 = bitcast<u32>(dbg_r1);
                        si_amb = rb1 & 0xFFFFu;
                        si_shadow = rb1 >> 16u;
                        si_otrap = u32(dbg_it1) & 0xFFFFu;
                        return;
                    }
                    if (it_result >= max_its_result || dtmp < ms_de_stop) {
                        // inside the set at the start plane
                        si_zpos = 0x7FFF0000u;
                        si_normal = vec3<i32>(0, 0, -32767);
                        do_color();
                        if (pi(COLOR_OPTION) > 4) {
                            si_grad |= 32768u;
                        } else {
                            let t = clamp(rout / pf(D_RSTOP), 0.0, 1.0);
                            si_grad = 32768u + u32(round(32767.0 * t));
                        }
                        return;
                    }
                    rsf_mul = 1.0;
                    last_step = dtmp * pf(Z_STEP_DIV);
                    pc = P_LOOP;
                }
            }
            case P_LOOP: {
                if (MODE_KIND == 0u) {
                    if (it_result >= max_its_result) {
                        dt1 = -0.5 * last_step;
                        mzz += dt1;
                        setc();
                        update_de_stop();
                        need = true;
                        want_de = true;
                        pc = P_HALF;
                    } else {
                        pc = P_DECIDE;
                    }
                }
            }
            case P_HALF: {
                if (MODE_KIND == 0u) {
                    dtmp = r;
                    last_step = -dt1;
                    pc = P_DECIDE;
                }
            }
            case P_DECIDE: {
                if (MODE_KIND == 0u) {
                    if (it_result < pi(MIN_IT) || (it_result < max_its_result && dtmp >= ms_de_stop)) {
                        // next step
                        last_de = dtmp;
                        var d = max(0.11, (dtmp - pf(MS_DE_SUB) * ms_de_stop) * pf(Z_STEP_DIV) * rsf_mul);
                        let st1 = max(ms_de_stop, 0.4) * pf(MH04ZSD);
                        let count = dfog_on_it == 0 || it_result == dfog_on_it;
                        if (st1 < d) {
                            if (count) {
                                step_count += st1 / d;
                            }
                            d = st1;
                        } else if (count) {
                            step_count += 1.0;
                        }
                        if (first_step) {
                            first_step = false;
                            seed = seed * 214013 + 2531011;
                            d *= f32(u32(seed) & 0x7FFFFFFFu) * (1.0 / 2147483647.0);
                        }
                        mzz += d;
                        if (mzz > pf(ZEND)) {
                            // background (`break` here would only leave the switch)
                            done = true;
                        } else {
                            last_step = d;
                            setc();
                            update_de_stop();
                            need = true;
                            want_de = true;
                            pc = P_STEP;
                        }
                    } else {
                        // surface found
                        de_limited = it_result < max_its_result || dtmp < ms_de_stop;
                        if (pi(DE_ADD_STEPS) != 0) {
                            itmp = pi(DE_ADD_STEPS);
                            if (de_limited) {
                                dt1 = last_step * -0.5;
                                pc = P_BS;
                            } else {
                                yp = f32(max_it) - 0.99;
                                saved_max = max_it;
                                max_it += 1;
                                calc_sit = true;
                                dt1 = 0.0;
                                dmul = 1.0;
                                first = true;
                                last_si = 0.0;
                                last_dif = 0.0;
                                pc = P_BSI;
                            }
                        } else {
                            pc = P_N_INIT;
                        }
                    }
                }
            }
            case P_STEP: {
                if (MODE_KIND == 0u) {
                    dtmp = r;
                    if (dtmp > last_de + last_step) {
                        dtmp = last_de + last_step;
                    }
                    rsf_mul = 1.0;
                    if (last_de > dtmp + 1e-30) {
                        let t = last_step / (last_de - dtmp);
                        if (t < 1.0) {
                            rsf_mul = max(0.5, t);
                        }
                    }
                    pc = P_LOOP;
                }
            }
            // RMdoBinSearch
            case P_BS: {
                if (abs(dtmp - ms_de_stop) <= bs_thr) {
                    pc = after_bs;
                } else {
                    mzz += dt1;
                    setc();
                    update_de_stop();
                    itmp -= 1;
                    if (itmp <= 0) {
                        pc = after_bs;
                    } else {
                        need = true;
                        want_de = true;
                        pc = P_BS_R;
                    }
                }
            }
            case P_BS_R: {
                dtmp = r;
                if (mode != M_DEAO && it_result >= max_its_result) {
                    dt1 = -abs(dt1);
                } else if (dtmp < ms_de_stop) {
                    dt1 = abs(dt1) * -0.55;
                } else {
                    dt1 = abs(dt1) * 0.55;
                }
                pc = P_BS;
            }
            // RMdoBinSearchIt
            case P_BSI: {
                mzz += dt1;
                setc();
                need = true;
                want_de = true;
                pc = P_BSI_1;
            }
            case P_BSI_1: {
                if (!first && last_dif < abs(yp - smooth_it)) {
                    mzz -= dt1;
                    setc();
                    smooth_it = last_si;
                    if (dt1 > 0.0) {
                        dmul *= 0.5;
                    } else {
                        dmul *= 0.7;
                    }
                }
                last_dif = abs(yp - smooth_it);
                last_si = smooth_it;
                if (smooth_it > f32(max_it) - 0.1) {
                    dt1 = -3.0;
                    pc = P_BSI_END;
                } else {
                    mzz -= 0.001;
                    setc();
                    need = true;
                    want_de = true;
                    pc = P_BSI_2;
                }
            }
            case P_BSI_2: {
                let rr = last_si - smooth_it;
                if (abs(rr) < 1e-30) {
                    dt1 = select(0.0, 1.0, last_si < smooth_it) - 0.5;
                } else if (rr < 0.0) {
                    dt1 = (yp - smooth_it) / (rr * 500.0);
                } else {
                    dt1 = (yp - smooth_it) / (rr * 1000.0);
                }
                if (dt1 > 4.0) {
                    dt1 = sqrt(dt1) * 2.0;
                } else if (dt1 < -9.0) {
                    dt1 = sqrt(-dt1) * -3.0;
                }
                dt1 = dt1 * dmul + 0.0005;
                pc = P_BSI_END;
            }
            case P_BSI_END: {
                first = false;
                itmp -= 1;
                if (itmp < 0) {
                    max_it = saved_max;
                    pc = after_bs;
                } else {
                    pc = P_BSI;
                }
            }
            // RMCalculateNormals (on_de) / RMCalculateNormalsOnSmoothIt
            case P_N_INIT: {
                if (MODE_KIND == 0u) {
                    noffset = min(pf(DE_STOP), 1.0) * (1.0 + mzz * pf(DE_STOP_FACTOR)) * 0.15;
                    calc_sit = true;
                    ct1 = c;
                    need = true;
                    want_de = on_de;
                    pc = P_N_0;
                }
            }
            case P_N_0: {
                if (MODE_KIND == 0u) {
                    dnn = r;
                    nn = smooth_it;
                    if (on_de) {
                        calc_sit = false;
                    }
                    n = vec3<f32>(0.0);
                    cnt = 0;
                    if (sm == 8) {
                        pc = P_N8;
                    } else {
                        pc = P_NA;
                    }
                }
            }
            case P_N8: {
                if (MODE_KIND == 0u) {
                    // the next of the 124 points of the 5x5x5 cube (not the centre)
                    if (cnt >= 125) {
                        n *= 0.0075;
                        pc = P_NSM;
                    } else {
                        let a = cnt / 25 - 2;
                        let b = (cnt / 5) % 5 - 2;
                        let cc = cnt % 5 - 2;
                        if ((a | b | cc) == 0) {
                            cnt += 1;
                        } else {
                            let sn = noffset * 1.3333;
                            c = ct1 + vg(2u) * (f32(a) * sn) + vg(1u) * (f32(b) * sn) + vg(0u) * (f32(cc) * sn);
                            need = true;
                            want_de = on_de;
                            pc = P_N8_R;
                        }
                    }
                }
            }
            case P_N8_R: {
                if (MODE_KIND == 0u) {
                    let a = cnt / 25 - 2;
                    let b = (cnt / 5) % 5 - 2;
                    let cc = cnt % 5 - 2;
                    let d = r * select(-1.0, 1.0, on_de);
                    if (a != 0) { n.z += d / f32(a); }
                    if (b != 0) { n.y += d / f32(b); }
                    if (cc != 0) { n.x += d / f32(cc); }
                    cnt += 1;
                    pc = P_N8;
                }
            }
            case P_NA: {
                if (MODE_KIND == 0u) {
                    // z, x, y as on the CPU; + then - the offset
                    if (cnt >= 6) {
                        pc = P_NSM;
                    } else {
                        let ax = (u32(cnt / 2) + 2u) % 3u;
                        if (cnt % 2 == 0) {
                            c = ct1 + vg(ax) * noffset;
                        } else {
                            c = ct1 - vg(ax) * noffset;
                        }
                        need = true;
                        want_de = on_de;
                        pc = P_NA_R;
                    }
                }
            }
            case P_NA_R: {
                if (MODE_KIND == 0u) {
                    let ax = (u32(cnt / 2) + 2u) % 3u;
                    if (cnt % 2 == 0) {
                        na = r;
                    } else {
                        n[ax] = select(r - na, na - r, on_de) * 0.5;
                    }
                    cnt += 1;
                    pc = P_NA;
                }
            }
            case P_NSM: {
                if (MODE_KIND == 0u) {
                    if (sm <= 0) {
                        pc = P_NEND;
                    } else {
                        if (cnt < 1000) {
                            // first visit: start the 4 samples around
                            noffset *= 2.0;
                            cnt = 1000;
                            acc = nn;
                        }
                        let i = cnt - 1000;
                        if ((on_de && sm >= 8) || i >= 4) {
                            if (on_de) {
                                if (sm < 8) {
                                    dnn = dnn * 0.2;
                                }
                            } else {
                                nn = acc * 0.2;
                                dnn = nn;
                            }
                            ssn = noffset * 3.0 / (f32(sm) + 0.5);
                            dm = f32(sm) * 2.0;
                            xy_vecs(n, &vx, &vy);
                            vx = rot_rev(vx);
                            vy = rot_rev(vy);
                            s1 = vec2<f32>(0.0);
                            ds = vec2<f32>(0.0);
                            axis = 0;
                            kk = -sm;
                            pc = P_SW;
                        } else {
                            let ax = u32(i / 2);
                            if (i % 2 == 0) {
                                c = ct1 - vg(ax) * noffset;
                            } else {
                                c = ct1 + vg(ax) * noffset;
                            }
                            need = true;
                            want_de = on_de;
                            pc = P_NSM_R;
                        }
                    }
                }
            }
            case P_NSM_R: {
                if (MODE_KIND == 0u) {
                    if (on_de) {
                        dnn += r;
                    } else {
                        acc += r;
                    }
                    cnt += 1;
                    pc = P_NSM;
                }
            }
            case P_SW: {
                if (MODE_KIND == 0u) {
                    if (axis >= 2) {
                        let dsg = ds.x * dm - s1.x * s1.x + ds.y * dm - s1.y * s1.y;
                        let dt2 = noffset * 0.5 / (dm * ssn);
                        s_roughness = calc_roughness(n, dt2, dsg);
                        if (sm < 8) {
                            n.x += s1.x * dt2;
                            n.y += s1.y * dt2;
                        }
                        pc = P_NEND;
                    } else if (kk > sm) {
                        axis += 1;
                        kk = -sm;
                    } else if (kk == 0) {
                        kk += 1;
                    } else {
                        let dir = select(vy, vx, axis == 0);
                        c = ct1 + dir * (f32(kk) * ssn);
                        need = true;
                        want_de = on_de;
                        pc = P_SW_R;
                    }
                }
            }
            case P_SW_R: {
                if (MODE_KIND == 0u) {
                    var d: f32;
                    if (on_de) {
                        d = (r - dnn) / f32(kk);
                    } else {
                        d = (dnn - r) / f32(kk);
                    }
                    s1[axis] += d;
                    ds[axis] += d * d;
                    kk += 1;
                    pc = P_SW;
                }
            }
            case P_NEND: {
                if (MODE_KIND == 0u) {
                    c = ct1;
                    store_normal(n);
                    var g: f32;
                    if (de_limited) {
                        g = 32767.0 - (nn + pf(D_COL_PLUS) + pf(COL_VAR_DE_STOP_MUL)
                            * log(max(pf(DE_STOP), ms_de_stop) * pf(STEP_WIDTH))) * pf(MCTS_M);
                    } else {
                        g = 32767.0 - nn * pf(MCTS_M);
                    }
                    si_grad = clip15(g);
                    do_color();
                    calc_zpos_and_rough();
                    done = true;
                }
            }
            // ---- the post calculations: the surface of the pixel
            case P_PS: {
                if (MODE_KIND != 0u) {
                    // the depth of the G-buffer (the inverse of calc_zpos_and_rough)
                    zf = f32(si_zpos >> 8u);
                    let a = (8388351.5 - zf) / pf(ZC_MUL);
                    mzz = a * (a + 2.0) / pf(ZCORR);
                    setc();
                    update_de_stop();
                    need = true;
                    want_de = true;
                    pc = P_PS_R;
                }
            }
            case P_PS_R: {
                if (MODE_KIND != 0u) {
                    dtmp = r;
                    if (mode == M_DEAO) {
                        dtmp = min(dtmp, ms_de_stop * 2.0);
                    }
                    de_limited = it_result < max_its_result || dtmp < ms_de_stop;
                    itmp = pi(DE_ADD_STEPS);
                    after_bs = select(P_AO_INIT, P_HS_INIT, mode == M_SHADOW);
                    if (de_limited) {
                        let a = (8388352.0 - zf) / pf(ZC_MUL);
                        let q = a * (a + 2.0) / pf(ZCORR) - mzz;
                        if (mode == M_SHADOW) {
                            dt1 = q * -0.5;
                            bs_thr = 0.001;
                        } else {
                            dt1 = q;
                            bs_thr = 0.004;
                        }
                        pc = P_BS;
                    } else {
                        yp = f32(max_it) - 0.99;
                        saved_max = max_it;
                        max_it += 1;
                        calc_sit = true;
                        dt1 = 0.0;
                        dmul = 1.0;
                        first = true;
                        last_si = 0.0;
                        last_dif = 0.0;
                        pc = P_BSI;
                    }
                }
            }
            // ---- hard_shadow_row / soft_shadow_row
            case P_HS_INIT: {
                if (MODE_KIND == 1u) {
                    calc_sit = false;
                    nvec = rot_rev(vec3<f32>(si_normal) / 32767.0);
                    let mz0 = mzz;
                    if (soft) {
                        mzz -= 0.1;
                    } else {
                        mzz = max(mzz - 0.1, 0.0);
                    }
                    c = base + vfov * (mz0 - 0.1);
                    update_de_stop();
                    ic = c;
                    max_lhs = hs_max_len(y);
                    zrs_mul = 80.0 / max(pf(HS_SOFT_RADIUS), 0.001);
                    li = 0u;
                    pc = P_HS_LIGHT;
                }
            }
            case P_HS_LIGHT: {
                if (MODE_KIND == 1u) {
                    if (li >= P[HS_N]) {
                        done = true;
                    } else {
                        lidx = P[HS_LIGHTS + li * 4u];
                        lvec = pv(HS_LIGHTS + li * 4u + 1u);
                        c = ic;
                        zz2 = mzz;
                        zz2mul = -dot(lvec, vfov) / max(sqrt(dot(lvec, lvec) * dot(vfov, vfov)), 1e-30);
                        if (dot(nvec, lvec) > 0.0) {
                            // the surface faces away from the light
                            if (soft) {
                                si_shadow &= 0x3FFu;
                                done = true;
                            } else {
                                si_shadow |= 0x400u << lidx;
                                li += 1u;
                            }
                        } else {
                            dt1 = max_lhs;
                            zr_soft = 1.0;
                            if (dt1 > 0.0) {
                                ms_de_stop = pf(DE_STOP) * (1.0 + abs(zz2) * pf(DE_STOP_FACTOR));
                                rsf = 2.0;
                                need = true;
                                want_de = true;
                                pc = P_HS_R0;
                            } else {
                                pc = P_HS_DONE;
                            }
                        }
                    }
                }
            }
            case P_HS_R0: {
                if (MODE_KIND == 1u) {
                    dtmp = r;
                    pc = P_HS_STEP;
                }
            }
            case P_HS_STEP: {
                if (MODE_KIND == 1u) {
                    last_de = dtmp;
                    st = min(max(0.11, (dtmp - pf(MS_DE_SUB) * ms_de_stop) * pf(Z_STEP_DIV) * rsf), max(ms_de_stop, 0.4) * pf(MH04ZSD));
                    dt1 -= st;
                    c -= lvec * st;
                    zz2 += st * zz2mul;
                    ms_de_stop = pf(DE_STOP) * (1.0 + abs(zz2) * pf(DE_STOP_FACTOR));
                    need = true;
                    want_de = true;
                    pc = P_HS_R;
                }
            }
            case P_HS_R: {
                if (MODE_KIND == 1u) {
                    dtmp = r;
                    if (soft) {
                        let rr = (max_lhs - dt1) / max_lhs;
                        let r8 = (rr * rr) * (rr * rr);
                        zr_soft = min(zr_soft, (dtmp - ms_de_stop) * zrs_mul / (max_lhs - dt1 + 0.11) + r8 * r8);
                    }
                    if (it_result >= max_its_result || dtmp <= ms_de_stop) {
                        pc = P_HS_DONE;
                    } else {
                        if (dtmp > last_de + st) {
                            dtmp = last_de + st;
                        }
                        rsf = 1.0;
                        if (last_de > dtmp + 1e-30) {
                            let t = st / (last_de - dtmp);
                            if (t < 1.0) {
                                rsf = max(0.5, t);
                            }
                        }
                        if (dt1 < 0.0) {
                            pc = P_HS_DONE;
                        } else {
                            pc = P_HS_STEP;
                        }
                    }
                }
            }
            case P_HS_DONE: {
                if (MODE_KIND == 1u) {
                    if (soft) {
                        si_shadow = (si_shadow & 0x3FFu) | (u32(round(clamp(zr_soft, 0.0, 1.0) * 63.4)) << 10u);
                        done = true;
                    } else {
                        if (dt1 > 0.0) {
                            si_shadow |= 0x400u << lidx; // in shadow
                        }
                        li += 1u;
                        pc = P_HS_LIGHT;
                    }
                }
            }
            // ---- deao_row
            case P_AO_INIT: {
                if (MODE_KIND == 2u) {
                    calc_sit = false;
                    update_de_stop();
                    let msl = min(ms_de_stop, 1e6);
                    ic = c;
                    step_ao = msl / pf(DE_STOP);
                    let s_max_d = pf(AO_MAX_LEN) * 0.5 * sqrt(f32(pi(HEIGHT)) * f32(pi(HEIGHT)) + f32(pi(WIDTH)) * f32(pi(WIDTH)));
                    max_dist = s_max_d * sqrt(step_ao);
                    let q = min(u32(pi(AO_QUALITY)), 3u);
                    let abr = 0.5 * PI / (f32(q) + 0.9);
                    if (q == 0u) {
                        d_step_mul = 1.8;
                        d_min_a_dif = -1.0;
                        corr_w = 0.3;
                    } else {
                        d_step_mul = 1.0 + sin(abr);
                        d_min_a_dif = cos(abr * 1.2);
                        corr_w = select(0.1666, 0.2, q == 1u);
                    }
                    ms2 = select(pf(DE_STOP), msl, pf(DE_STOP_FACTOR) != 0.0) / (d_step_mul * d_step_mul);
                    // the ray directions, dithered per pixel
                    let dither = pi(AO_DITHER);
                    var dt1d = 0.0;
                    var dt2d = 0.0;
                    if (dither > 0) {
                        dt1d = f32(y % (dither + 1)) * 0.5 / f32(dither);
                        dt2d = f32(x % (dither + 1)) * 0.5 / f32(dither);
                    }
                    ray_count = 0u;
                    if (q == 0u) {
                        for (var i = 0u; i < 3u; i++) {
                            if (dither > 0) {
                                ao_dirs[i] = ray_dir((dt1d + 0.5) * radians(50.0), (f32(i) + dt2d) * 2.0 * PI / 3.0);
                            } else {
                                ao_dirs[i] = ray_dir(0.5 * radians(60.0), f32(i) * 2.0 * PI / 3.0);
                            }
                        }
                        ray_count = 3u;
                    } else {
                        if (!(dither > 0 && dt1d > 0.1)) {
                            ao_dirs[0] = ray_dir(0.0, 0.0);
                            ray_count = 1u;
                        }
                        for (var rr = 1u; rr <= q; rr++) {
                            let n = u32(round(sin(f32(rr) * abr) * 2.0 * PI / abr));
                            for (var i = 0u; i < n; i++) {
                                if (ray_count < 33u) {
                                    ao_dirs[ray_count] = ray_dir(abr * (f32(rr) + dt1d - select(0.0, 0.25, dither > 0)), (f32(i) + dt2d) * 2.0 * PI / f32(n));
                                    ray_count += 1u;
                                }
                            }
                        }
                    }
                    de_mul = sqrt(f32(ray_count) * 0.5);
                    abr_c = 1.2 / asin(clamp(1.0 / de_mul, -1.0, 1.0));
                    md_d10 = 0.1 / (max_dist * de_mul);
                    rot_w = normal_matrix(normalize(vec3<f32>(si_normal)));
                    // normalise_matrix_to(step_width, vgrads)
                    rv0 = vg(0u) * (pf(STEP_WIDTH) / length(vg(0u)));
                    rv1 = vg(1u) * (pf(STEP_WIDTH) / length(vg(1u)));
                    rv2 = vg(2u) * (pf(STEP_WIDTH) / length(vg(2u)));
                    k = 0u;
                    pc = P_AO_RAY;
                }
            }
            case P_AO_RAY: {
                if (MODE_KIND == 2u) {
                    if (k >= ray_count) {
                        pc = P_AO_CORR;
                    } else {
                        let d = ao_dirs[k];
                        let vv = vec3<f32>(dot(rot_w[0], d), dot(rot_w[1], d), dot(rot_w[2], d));
                        sv = vv.x * rv0 + vv.y * rv1 + vv.z * rv2;
                        dt1 = step_ao * d_step_mul;
                        s_tmp = 1.0;
                        b_end = false;
                        b_first = pi(AO_FIRST_RANDOM) != 0;
                        pc = P_AO_STEP;
                    }
                }
            }
            case P_AO_STEP: {
                if (MODE_KIND == 2u) {
                    if (b_first) {
                        b_first = false;
                        dt1 *= get_rand() * 1.5 + 0.5;
                    } else if (dt1 > max_dist) {
                        dt1 = max_dist;
                        b_end = true;
                    }
                    c = ic + sv * dt1;
                    need = true;
                    want_de = true;
                    pc = P_AO_R;
                }
            }
            case P_AO_R: {
                if (MODE_KIND == 2u) {
                    let dt2 = r;
                    s_tmp = min(s_tmp, (dt2 - ms2 + dt1 * md_d10) / dt1);
                    var fin = s_tmp < 0.02;
                    if (!fin) {
                        dt1 += max(dt2, dt1 * d_step_mul);
                        fin = b_end;
                    }
                    if (fin) {
                        ao_min[k] = max(s_tmp, 0.0) * de_mul;
                        k += 1u;
                        pc = P_AO_RAY;
                    } else {
                        pc = P_AO_STEP;
                    }
                }
            }
            case P_AO_CORR: {
                if (MODE_KIND == 2u) {
                    // open neighbour rays lighten a closed one
                    var amount = 0.0;
                    for (var i = 0u; i < ray_count; i++) {
                        var s_add = 0.0;
                        if (ao_min[i] < 1.0) {
                            let max_add = 1.0 - ao_min[i];
                            for (var i2 = 0u; i2 < ray_count; i2++) {
                                if (i2 == i) {
                                    continue;
                                }
                                let dt = dot(ao_dirs[i], ao_dirs[i2]);
                                if (dt > d_min_a_dif) {
                                    let overlap = ao_min[i2] - acos(clamp(dt, -1.0, 1.0)) * abr_c + 1.0;
                                    if (overlap > 0.0) {
                                        s_add += min(max_add, overlap) * corr_w;
                                    }
                                }
                            }
                        }
                        amount += min(s_add + ao_min[i], 1.0);
                    }
                    si_amb = u32(max(round(16383.0 * (1.0 - amount / f32(ray_count))), 0.0));
                    done = true;
                }
            }
            default: {
                done = true;
            }
        }
        if (done) {
            break;
        }
    }
    if (mode == M_MARCH) {
        si_shadow = u32(round(clamp(step_count, 0.0, 1023.0)));
    }
}


@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let lx = i32(gid.x);
    let ly = i32(gid.y);
    let w = pi(RECT_W);
    if (lx >= w || ly >= pi(ROWS)) {
        return;
    }
    let x = pi(RECT_X0) + lx;
    let y = pi(RECT_Y0) + pi(ROW0) + ly;
    // MB3D seeds per row; the CPU advances the seed along the row, here the
    // seed is advanced by the column (the same random sequence, but not
    // exactly the same numbers where pixels have no first step)
    seed = bitcast<i32>(0x24563487u + u32(y + 1) * 0x324594A1u);
    for (var k = 0; k < lx; k++) {
        seed = seed * 214013 + 2531011;
    }
    _ = cst[0];
    _ = rtpl[0];
    init_record();
    j4 = pf(J4);
    vary_scale = 1.0;
    deriv = vec3<f32>(1.0, 0.0, 0.0);
    dfree = vec2<f32>(0.0, 0.0);
    let o = u32(ly * w + lx) * 5u;
    let mode = P[MODE];
    // one call of pixel() only: the driver inlines it at every call site
    var run = true;
    if (mode == M_MARCH || mode == M_PROBE) {
        si_normal = vec3<i32>(0);
        si_zpos = 32768u << 16u;
        si_shadow = 0u;
        si_grad = 0u;
        si_otrap = 0u;
        si_amb = 5000u; // MB3D's value when no ambient shadow is calculated
        s_roughness = 0.0;
    } else {
        // a post calculation on the pixel of the G-buffer
        let a = OUT[o];
        let b = OUT[o + 1u];
        si_normal = vec3<i32>(i32(a << 16u) >> 16u, i32(a) >> 16u, i32(b << 16u) >> 16u);
        si_shadow = b >> 16u;
        si_zpos = OUT[o + 2u];
        si_grad = OUT[o + 3u] & 0xFFFFu;
        si_amb = OUT[o + 4u];
        if (mode == M_SHADOW) {
            if (pf(HS_SOFT_RADIUS) > 0.0) {
                si_shadow |= 0xFC00u;
            } else {
                for (var i = 0u; i < P[HS_N]; i++) {
                    si_shadow &= ~(0x400u << P[HS_LIGHTS + i * 4u]);
                }
            }
        } else {
            si_amb = 0u;
        }
        run = (si_zpos >> 16u) < 32768u && si_grad < 32768u;
    }
    if (run) {
        pixel(x, y, mode);
    }
    if (mode == M_MARCH || mode == M_PROBE) {
        OUT[o] = (u32(si_normal.x) & 0xFFFFu) | (u32(si_normal.y) << 16u);
        OUT[o + 2u] = si_zpos;
        OUT[o + 3u] = si_grad | (si_otrap << 16u);
    }
    OUT[o + 1u] = (u32(si_normal.z) & 0xFFFFu) | (si_shadow << 16u);
    OUT[o + 4u] = si_amb;
}
