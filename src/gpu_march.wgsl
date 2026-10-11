// The ray marcher of calc.rs (`Marcher::march_pixel`, MB3D's
// TMandCalcThread.Execute) for the graphics card, in single precision.
// The alternating 3D hybrid of 'Integer Power' and .m3f formulas (lifted
// to WGSL, x86/lift_wgsl.rs; gpu.rs inserts them at the FORMULAS marker
// marker), numerical or analytic DE; cutting planes, inside rendering;
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
// resumable marching: every thread is a slot for a pixel (or task) of the
// job, taken from QUEUE[1]; a pixel that has
// used the turns of a submission (TURN_BUDGET) saves its state in the slot
// (word 0: the pixel + 1, 0 when the slot is free; word k of slot s at
// k * N_SLOTS + s), counts itself in QUEUE[0] and its index in QUEUE[2]
// (the lowest suspended), and the next submission continues it.  A
// submission thus lasts a short time even where one pixel takes long (the
// desktop shares the card), and the slots of finished pixels take new ones
// meanwhile.
@group(0) @binding(4) var<storage, read_write> STATE: array<u32>;
@group(0) @binding(5) var<storage, read_write> QUEUE: array<atomic<u32>, 4>;
// the maps the formulas read (map_px): count, per map (number, width,
// height, index of its pixels), the pixels as f32 RGB 0..255 in rows of
// width + 1 (LightMap's layout)
@group(0) @binding(6) var<storage, read> MAPS: array<u32>;

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
const R_SPLIT: u32 = 165u;        // the reflection pass runs in two pipelines (R_MARCH)
const N_SLOTS: u32 = 166u;        // resumable marching: the slots (64 per work group)
const ITEM_END: u32 = 167u;       // the pixels (from ROW0, RECT_W per row) or tasks of the job
const TURN_BUDGET: u32 = 272u;    // the turns of a pixel in a submission
// the words of the marcher's state in a slot (after the suspended flag);
// the record cells follow (gpu.rs reads this number)
const STATE_FIXED: u32 = 563u;
const HYB_KIND: u32 = 273u;       // the hybrid: 0 alternating 3D, 1 alternating 4D (doHybrid4DPas),
                                  // 2 interpolation 3D, 3 interpolation 4D (doInterpolHybridPas[4D])
const P2_HYB_KIND: u32 = 274u;    // the second part's
// the reflection pass (M_REFLECT): tasks instead of pixels
const R_FAR_FOG: u32 = 168u;     // far fog / squared depth: the depth of a hit from the fog formula
const R_SURFACE: u32 = 169u;     // 1: the tasks are the pixels' surfaces (no march)
const R_N: u32 = 170u;           // the number of tasks
const R_LIGHTS: u32 = 172u;      // 6 x (index, HSvec or position, positional, lmax_l, visible, 0)
const R_WORDS: u32 = 24u;        // a task: 0-2 origin, 3-5 direction, 6 zz (surface: zpos), 7 pixel;
                                 // out 8-12 the G-buffer value, 13-15 position, 16 zz, 17 zz2, 18 flags, 19 step count
const R_W: u32 = 256u;           // tasks per dispatch row
const M_MARCH: u32 = 0u;
const M_SHADOW: u32 = 1u;
const M_DEAO: u32 = 2u;
const M_PROBE: u32 = 3u;         // the DE at the ray start (a check against the CPU)
const M_REFLECT: u32 = 4u;       // the reflection pass: a bounce (or the surface) of every task
// 6 slots of 8 words: iterations, uncounted, constants base, fHln,
// Integer Power power, z multiplier, kind (0 none, 1 Integer Power, 2 .m3f,
// 3 Amazing Box: see amazing_box)
const SLOTS: u32 = 64u;
fn slot(n: u32, k: u32) -> u32 { return P[SLOTS + n * 8u + k]; }
// the parameters of the built-in formulas (8 per slot)
const SLOTX: u32 = 224u;
fn slotx(n: u32, k: u32) -> R { return R(bitcast<f32>(P[SLOTX + n * 8u + k])); }
fn slotxi(n: u32, k: u32) -> i32 { return bitcast<i32>(P[SLOTX + n * 8u + k]); }
// the 4D rotation (smatrix4, 4 rows of 3) of the 4D hybrids
const SMATRIX4: u32 = 275u;
const IPOL_W: u32 = 287u;         // the weights of the two formulas of an interpolation hybrid
const INSIDE_FLAGS: u32 = 289u;   // 1 inside rendering, 2 inside and outside
const COLOR_ON_IT: u32 = 290u;    // doColorOnIt: 0 off, 1 the start vector, n the vector after n - 1 iterations
const CUT_OPT: u32 = 291u;        // the cutting planes: bits 0-2 the axes, 4-6 their sides
const CUT_POS: u32 = 292u;        // 3 values
const DP: u32 = 296u;             // the coordinates as doubles: start (3), view gradients (9), julia (4)

const PI: R = 3.14159265358979;

// Pipeline constants: a pipeline per kind of pass (0 the march, 1 the
// shadows, 2 the ambient occlusion) and per scene feature, so that the
// driver compiles only the states and loops a pass uses (the whole state
// machine in one function compiles ten times slower).
override MODE_KIND: u32 = 0u;
override HAS_DIFS: bool = false;
override HAS_DECOMB: bool = false;
override HAS_4D: bool = false;
override HAS_IPOL: bool = false;
// the reflection pass (MODE_KIND 3): the shadow rays / the ambient
// occlusion at the hits; without them their states compile away
override R_HS: bool = true;
override R_AO: bool = true;
// the reflection pass in two pipelines where it casts shadow or occlusion
// rays (R_SPLIT): the march to the hits (R_MARCH), then the shadows and the
// occlusion of the hits (!R_MARCH; P_R_POST takes up the hit from the task
// record): each shader is smaller, more rays fit on the card at once
override R_MARCH: bool = true;

fn pf(i: u32) -> R { return R(bitcast<f32>(P[i])); }
fn pi(i: u32) -> i32 { return bitcast<i32>(P[i]); }
fn pv(i: u32) -> vec3<R> { return vec3<R>(pf(i), pf(i + 1u), pf(i + 2u)); }

// iteration state (Marcher::it)
var<private> c: vec3<R>;
var<private> v: vec3<R>;
var<private> rout: R;
// r_log(rout) of the last iteration, finite where rout overflows single
// precision (orbits that escaped far: |v| beyond 1e19)
var<private> lrout: R;
var<private> rold: R;
var<private> otrap: R;
var<private> it_result: i32;
var<private> max_it: i32;
var<private> rstop: R;
var<private> calc_sit: bool;
var<private> smooth_it: R;
// the rest of the iteration record the formulas see (Iteration)
var<private> w: R;
var<private> j: vec3<R>;
var<private> j4: R;
var<private> vary_scale: R;
var<private> first_it: i32;
var<private> deriv: vec3<R>;
var<private> dfree: vec2<R>;
// marcher state
var<private> ms_de_stop: R;
// the hybrid part being iterated (set_part)
var<private> part_start: u32;
var<private> part_end: u32;
var<private> part_repeat: u32;
var<private> de_option: i32;
// the absolute DE stop of a dIFS loop (RStopD)
var<private> rstopd: R;
// the pixel's view vector (the shadow rays measure their depth along it)
var<private> vfov_px: vec3<R>;
// the depth along the view for the DE stop: zz_base + mzz * zz_mul (a
// reflected ray runs at an angle to the view); abs for the reflection pass
var<private> zz_base: R;
var<private> zz_mul: R;
var<private> zz_abs: bool;
// the reflection pass: its outputs
var<private> r_flags: u32;
var<private> r_zz2: R;
var<private> r_step_count: R;
// the parts' own results of the last DE combination (M_PROBE)
var<private> dbg_r1: R;
var<private> dbg_it1: i32;
var<private> dbg_rst: R;
var<private> dbg_it2: i32;
var<private> dbg_rout: R;
var<private> dbg_g: R;
// a DE of the pixel overflowed single precision (NaN / infinite before
// the clamp): the CPU decides whether the image is trustworthy
var<private> de_overflow: bool;
// inside rendering (Marcher::inside / calc_inside): the DE is inverted
var<private> inside: bool;
var<private> mzz: R;
var<private> vfov: vec3<R>;
var<private> base: vec3<R>;
var<private> max_its_result: i32;
var<private> s_roughness: R;
var<private> seed: i32;
// the result (SiLight)
var<private> si_normal: vec3<i32>;
var<private> si_zpos: u32;
var<private> si_shadow: u32;
var<private> si_grad: u32;
var<private> si_otrap: u32;
var<private> si_amb: u32;
// the rays of the ambient occlusion
var<private> ao_dirs: array<vec3<R>, 33>;
var<private> ao_min: array<R, 33>;

fn is_nan(x: R) -> bool {
    let b = bitcast<u32>(f32(x)) & 0x7FFFFFFFu;
    return b > 0x7F800000u;
}

// min_max_clip_15bit
fn clip15(s: R) -> u32 {
    if (is_nan(s) || s < 0.0) { return 0u; }
    if (s > 32767.0) { return 32767u; }
    return u32(round(s));
}

// the position on the ray (CPU: c, moved with add_weight)
fn setc() {
    c = base + vfov * mzz;
}

// fHIntFunctions[power] (formulas.rs int_pow)
fn int_pow(p: vec3<R>, j: vec3<R>, power: i32, zmul: R) -> vec3<R> {
    let x = p.x;
    let y = p.y;
    let z = p.z;
    let EPS = R(1e-30);
    var o: vec3<R>;
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
fn box_fold(x: R, fold: R) -> R {
    return abs(x + fold) - (abs(x - fold) + x);
}

fn abox(scale: R, min_r: R, fold: R, ade: bool) {
    let x = box_fold(v.x, fold);
    let y = box_fold(v.y, fold);
    let z = box_fold(v.z, fold);
    let r = z * z + y * y + x * x;
    let sqr_min_r = min_r * min_r;
    var mul: R;
    if (r < sqr_min_r) {
        mul = scale / sqr_min_r;
    } else if (1.0 < r) {
        mul = scale;
    } else {
        mul = scale / r;
    }
    if (ade) {
        w *= mul;
    }
    v = vec3<R>(x * mul + j.x, y * mul + j.y, z * mul + j.z);
}

fn amazing_box(n: u32) {
    abox(R(bitcast<f32>(slot(n, 4u))), R(bitcast<f32>(slot(n, 5u))), R(bitcast<f32>(slot(n, 7u))), slot(n, 2u) != 0u);
}

// 'Tricorn' (formulas.rs): z multiplier, cz multiplier
fn tricorn(n: u32) {
    let x = v.x;
    let y = v.y;
    let z = v.z;
    v.z = z * x * slotx(n, 0u) + j.z * slotx(n, 1u);
    v.x = x * x - (y * y + z * z) + j.x;
    v.y = 2.0 * (y * x) + j.y;
}

// 'Real Power' (HybridFloatPow): power, z multiplier; rout is the squared radius
fn real_power(n: u32) {
    let pw = slotx(n, 0u);
    let x = v.x;
    let y = v.y;
    let z = v.z;
    let theta = r_atan2(y, x) * pw;
    let phi = r_atan2(z, sqrt(x * x + y * y)) * pw;
    let r = r_pow(rout, pw * 0.5);
    v = vec3<R>(r_cos(theta) * r_cos(phi) * r + j.x, r_sin(theta) * r_cos(phi) * r + j.y, r_sin(phi) * r * slotx(n, 1u) + j.z);
}

// HybridItIntPow2scale: sine bulb power 2 with scaling (Bulbox)
fn int_pow2_scale(scaling: R) {
    let s = 1.0 / scaling;
    let x = v.x * s;
    let y = v.y * s;
    let z = v.z * s;
    let xx = x * x;
    let yy = y * y;
    let r = xx + yy;
    let a = (r - z * z) / r;
    v = vec3<R>((xx - yy) * a * scaling + j.x, (x * y * a) * 2.0 * scaling + j.y, -((sqrt(r) * z) * 2.0) * scaling + j.z);
}

// 'Bulbox' (HybridSuperCube2): box scale, box min R, box fold, bulb scaling,
// R threshold, R threshold 2
fn bulbox(n: u32) {
    let scale = slotx(n, 0u);
    let min_r = slotx(n, 1u);
    let fold = slotx(n, 2u);
    let bulb_scale = slotx(n, 3u);
    let rt1 = slotx(n, 4u) * slotx(n, 4u);
    let rt2 = slotx(n, 5u) * slotx(n, 5u);
    if (rout < rt1) {
        if (rout < rt2) {
            int_pow2_scale(bulb_scale);
        } else {
            let m = (rout - rt2) / (rt1 - rt2);
            let xyz_in = v;
            int_pow2_scale(bulb_scale);
            let o = v;
            v = xyz_in;
            abox(scale, min_r, fold, false);
            let r1 = length(v);
            let m2 = 1.0 - m;
            let r2 = r1 * m + m2 * length(o);
            v = v * m + m2 * o;
            v *= r2 / sqrt(dot(v, v) + 1e-40);
        }
    } else {
        abox(scale, min_r, fold, false);
    }
}

// 'Folding Int Pow': power, z multiplier, R fold
fn folding_int_pow(n: u32) {
    let fold = slotx(n, 2u);
    v = vec3<R>(box_fold(v.x, fold), box_fold(v.y, fold), box_fold(v.z, fold));
    v = int_pow(v, j, slotxi(n, 0u), slotx(n, 1u));
}

// 'Aexion C' (formulas.rs aexion_c): float power, z multiplier, rotate C,
// conditional phi, float power C, Cz multiplier, PowC on distance, mode
fn aexion_c(n: u32) {
    let pw = slotx(n, 0u);
    let z_mul = slotx(n, 1u);
    let x = v.x;
    let y = v.y;
    let z = v.z;
    var r1 = x * x + y * y + z * z;
    let th = r_atan2(sqrt(x * x + z * z), y) * pw;
    let ph = r_atan2(z, x) * pw;
    r1 = r_exp(r_log(r1) * pw * 0.5);
    v = vec3<R>(r_cos(ph) * r_cos(th) * r1 + j.x, r_cos(th) * r_sin(ph) * r1 + j.y, r_sin(th) * r1 * z_mul + j.z);
    if (slotxi(n, 2u) == 0) {
        return;
    }
    var pd = slotx(n, 4u);
    if (slotxi(n, 6u) != 0) {
        pd *= length(v - j);
    }
    let r = length(j);
    let mode = slotxi(n, 7u);
    var a: R;
    var b: R;
    var cc: R;
    if ((mode & 16) != 0) {
        a = v.y - j.y;
        b = v.z - j.z;
        cc = v.x - j.x;
    } else {
        a = j.y;
        b = j.z;
        cc = j.x;
    }
    var ofs = 0;
    var ecx = 8;
    if ((mode & 8) != 0) {
        let t = a;
        a = b;
        b = t;
        ecx = 16;
    }
    let s = sqrt(b * b + cc * cc);
    var th2 = select(r_atan2(s, a), r_atan2(a, s), (mode & 1) != 0);
    var ph2: R;
    if ((mode & 2) != 0) {
        ofs = 24 - ecx;
        ph2 = r_atan2(cc, b);
    } else {
        ph2 = r_atan2(b, cc);
    }
    if ((mode & 4) != 0) {
        let t = th2;
        th2 = ph2;
        ph2 = t;
        ofs = ecx;
    }
    if (slotxi(n, 3u) != 0) {
        var t = v.z;
        if (ofs == 0) {
            t = v.x;
        } else if (ofs == 8) {
            t = v.y;
        }
        if ((bitcast<u32>(f32(t)) >> 31u) == 0u) {
            ph2 = -ph2;
        }
    }
    let sa = r_sin(ph2 * pd);
    let ca = r_cos(ph2 * pd);
    let sb = r_sin(th2 * pd);
    let cb = r_cos(th2 * pd);
    j = vec3<R>(ca * cb * r, cb * sa * r, sb * r * slotx(n, 5u));
}

// Quaternion (4D): the quaternion square plus the constant; `w` is the
// fourth coordinate here, `j4` the fourth constant
fn quaternion(n: u32) {
    let x = v.x;
    let y = v.y;
    let z = v.z;
    let q = w;
    v.x = x * x - y * y - z * z - q * q + j.x;
    v.y = 2.0 * (y * x + z * q) + j.y;
    v.z = 2.0 * (z * x + slotx(n, 0u) * y * q) + j.z;
    w = 2.0 * (q * x + y * z) + slotx(n, 1u) + j4;
}

// rotate_4Dex: the start vector of a 4D hybrid (smatrix4 * c)
fn rotate_4d(p: vec3<R>) -> vec4<R> {
    var r: vec4<R>;
    for (var i = 0u; i < 4u; i++) {
        r[i] = p.x * pf(SMATRIX4 + i * 3u) + p.y * pf(SMATRIX4 + i * 3u + 1u) + p.z * pf(SMATRIX4 + i * 3u + 2u);
    }
    return r;
}

// MakeSplineCoeff
fn map_coeff(xs: R) -> vec4<R> {
    let r3 = xs * xs * xs / 6.0;
    let r0 = 1.0 / 6.0 + 0.5 * xs * (xs - 1.0) - r3;
    let r2 = xs + r0 - 2.0 * r3;
    return vec4<R>(r0, 1.0 - r0 - r2 - r3, r2, r3);
}

// GetMapPixelSphereSpline (`sphere`) / GetMapPixelDirectXYspline, called by
// the formulas through the record (custom.rs map_px): the colour (0..1) of
// map `nr` at the direction `v` / at x, y; 0 for a map that is not loaded
fn map_px(sphere: bool, nr: i32, v: vec3<R>) -> vec3<R> {
    var hd = 0u;
    for (var i = 0u; i < MAPS[0]; i++) {
        if (MAPS[1u + i * 4u] == u32(nr)) {
            hd = 1u + i * 4u;
            break;
        }
    }
    if (hd == 0u || nr < 1) {
        return vec3<R>(0.0);
    }
    let wi = i32(MAPS[hd + 1u]);
    let hi = i32(MAPS[hd + 2u]);
    let base = MAPS[hd + 3u];
    if (wi < 1 || hi < 1) {
        return vec3<R>(0.0);
    }
    let w = R(wi);
    let h = R(hi);
    var x: R;
    var y: R;
    if (sphere) {
        let d = v / max(length(v), 1e-30);
        x = clamp((r_atan2(d.x, d.y) * (-0.5 / PI) + 0.5) * w, 0.0, w);
        y = clamp((0.5 - r_asin(clamp(d.z, -1.0, 1.0)) / PI) * h, 0.0, h);
    } else {
        x = fract(v.x) * w;
        y = fract(v.y) * h;
    }
    let xf = i32(x);
    let yf = i32(y);
    let xv = map_coeff(x - R(xf));
    let yv = map_coeff(y - R(yf));
    var r = vec3<R>(0.0);
    for (var j = 0; j < 4; j++) {
        let row = u32(((yf + j - 1) % hi + hi) % hi);
        var s = vec3<R>(0.0);
        for (var i = 0; i < 4; i++) {
            let col = u32(((xf + i - 1) % wi + wi) % wi);
            let o = base + (row * u32(wi + 1) + col) * 3u;
            s += vec3<R>(R(bitcast<f32>(MAPS[o])), R(bitcast<f32>(MAPS[o + 1u])), R(bitcast<f32>(MAPS[o + 2u]))) * xv[i];
        }
        r += s * yv[j];
    }
    return r / 255.0;
}

// CalcSmoothIterations (fHln of the last formula n)
fn calc_smooth_iterations(n: u32) {
    if (rout <= 1.0) {
        smooth_it = R(it_result);
    } else if (rold < 1.0) {
        let d = r_log(0.5 * lrout) * R(bitcast<f32>(slot(n, 3u)));
        smooth_it = R(it_result) + pf(LN_RSTOP) - d;
    } else {
        let d = r_log(0.5 * lrout);
        let e = d - r_log(0.5 * r_log(rold));
        smooth_it = (pf(LN_RSTOP) - d) / (e + 1e-30) + R(it_result);
    }
}

//@FORMULAS@

// doHybridPas / doHybridPasDE (Iteration::run, hybrid_3d, hybrid_3d_de):
// the alternating hybrid; `de`: with the analytic DE (returned).
// `four_d` (doHybrid4DPas, hybrid_4d / hybrid_4d_de): the vector is the
// rotated 4D start, `w` its fourth coordinate (the derivative of the
// analytic DE is deriv.x then), the lengths are 4D.
// `ipol` (doInterpolHybridPas[4D][DE], Iteration::run_ipol): the first two
// formulas are applied to the same vector and the results blended with
// the weights IPOL_W (the two calls go through the loop's one call site)
fn hybrid(de: bool, four_d: bool, ipol: bool) -> R {
    let deo = de_option;
    if (four_d) {
        let v4 = rotate_4d(c);
        v = v4.xyz;
        w = v4.w;
        if (pi(DO_JULIA) != 0) {
            j = p_ju();
            j4 = p_j4();
        } else {
            j = v4.xyz;
            j4 = v4.w;
        }
        rout = dot(v4, v4);
        otrap = rout;
        if (de) {
            if ((deo & 0x38) == 16) {
                deriv.x = rout;
            } else if ((deo & 0x38) == 32) {
                deriv = vec3<R>(1.0, 0.0, 0.0);
            } else {
                deriv.x = 1.0;
            }
        }
    } else {
        if (pi(DO_JULIA) != 0) {
            j = p_ju();
        } else {
            j = c;
        }
        v = c;
        w = 0.0;
        rout = dot(c, c);
        otrap = rout;
        if (de && ipol) {
            w = select(1.0, rout, (deo & 0x18) == 16);
        } else if (de) {
            if ((deo & 0x38) == 16) {
                w = rout;
            } else if ((deo & 0x38) == 32) {
                deriv = vec3<R>(1.0, 0.0, 0.0);
            } else {
                w = 1.0;
            }
        }
    }
    // interpolation: the vector before the iteration, the result of the
    // first formula, the derivatives; `phase` 1 between the two calls
    let s1 = pf(IPOL_W);
    let s2 = pf(IPOL_W + 1u);
    var phase = 0u;
    var y1v = vec3<R>(0.0);
    var y1w = R(0.0);
    var x1v = vec3<R>(0.0);
    var x1w = R(0.0);
    var dt = R(0.0);
    var dd = R(0.0);
    var n = part_start;
    var btmp = max(bitcast<i32>(slot(n, 0u)), 0);
    it_result = 0;
    first_it = 0;
    var uncounted_guard = 0u;
    loop {
        if (ipol) {
            if (phase == 0u) {
                rold = rout;
                y1v = v;
                y1w = w;
                dt = deriv.x;
                n = 0u;
            } else {
                x1v = v;
                x1w = w;
                v = y1v;
                w = y1w;
                dd = deriv.x;
                if (four_d && de) {
                    deriv.x = dt;
                }
                n = 1u;
            }
            btmp = 1;
        } else {
            rold = rout;
        }
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
        if (ipol) {
            if (phase == 0u) {
                phase = 1u;
                continue;
            }
            phase = 0u;
            var xx: R;
            var yy: R;
            if (four_d) {
                xx = sqrt(dot(x1v, x1v) + x1w * x1w);
                yy = sqrt(dot(v, v) + w * w);
            } else {
                xx = length(x1v);
                yy = length(v);
            }
            xx = xx * s1 + yy * s2;
            var nv = x1v * s1 + v * s2;
            var nw = x1w * s1 + w * s2;
            if (de && !four_d) {
                nw = abs(x1w) * s1 + abs(w) * s2;
            }
            if (de && four_d) {
                deriv.x = abs(dd) * s1 + abs(deriv.x) * s2;
            }
            var l2 = dot(nv, nv);
            if (four_d) {
                l2 += nw * nw;
            }
            // (MB3D adds 1e-40, below the smallest normal f32)
            let f = xx / sqrt(max(l2, 1e-37));
            nv *= f;
            if (four_d) {
                nw *= f;
            }
            v = nv;
            w = nw;
            it_result += 1;
            rout = xx * xx;
            if (rout < otrap) {
                otrap = rout;
            }
            if (it_result >= max_it || rout > rstop) {
                break;
            }
            continue;
        }
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
        if (four_d) {
            rout += w * w;
        }
        if (rout < otrap) {
            otrap = rout;
        }
        if (it_result >= max_it || rout > rstop) {
            break;
        }
    }
    var mabs = max(abs(v.x), max(abs(v.y), abs(v.z)));
    if (four_d) {
        mabs = max(mabs, abs(w));
    }
    if (mabs > 1e18) {
        let sv = v / mabs;
        let sw = select(0.0, w / mabs, four_d);
        lrout = r_log(dot(sv, sv) + sw * sw) + 2.0 * r_log(mabs);
    } else {
        lrout = r_log(rout);
    }
    var result = R(0.0);
    if (ipol) {
        // the smooth iterations use fHln of the first formula
        n = 0u;
    }
    if (four_d && de) {
        // de_4d_result: the derivative is Deriv1
        if ((deo & 7) == 4) {
            result = abs(v.z) * r_log(abs(v.z)) / deriv.x;
        } else {
            result = sqrt(rout) / abs(deriv.x);
        }
    } else if (de && ipol) {
        if ((deo & 7) == 4) {
            result = abs(v.z) * r_log(abs(v.z)) / w;
        } else {
            result = sqrt(rout) / abs(w);
        }
    } else if (de) {
        if ((deo & 0x38) == 32) {
            result = sqrt(rout) * 0.5 * r_log(rout) / deriv.x;
        } else if ((deo & 7) == 4) {
            result = abs(v.y) * r_log(abs(v.y)) / w;
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
// `ins` (Iteration::hybrid_difs `inside`): rendered from inside the set
// (inside rendering or the inverted maximum combination): the loop does not
// stop at the first DE below the stop
fn hybrid_difs(vec_ini: bool, rs: R, ins: bool) -> R {
    if (vec_ini) {
        if (pi(DO_JULIA) != 0) {
            j = p_ju();
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
    difs_init(ins);
    var min_de = R(65535.0);
    var dfree2 = R(0.0);
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
            if (!ins && de < rs) {
                break;
            }
        }
        if (it >= max_it) {
            break;
        }
    }
    it_result = it_min;
    smooth_it = R(it_min);
    otrap = dfree2;
    return min_de;
}

// The one place that iterates the formulas: the driver inlines functions,
// so `hybrid` and `hybrid_difs` (and through them the formula code) must
// have a single textual call site each.
fn run_part(difs: bool, de: bool, vec_ini: bool, rs: R, kind: u32, ins: bool) -> R {
    if (HAS_DIFS && difs) {
        return hybrid_difs(vec_ini, rs, ins);
    }
    return hybrid(de, HAS_4D && (kind & 1u) != 0u, HAS_IPOL && kind >= 2u);
}

// Marcher::calc_de (`on_de`: the DE of the hybrid parts, combined as the
// DE combination says) or mMandFunction with calc_sit (the smoothed
// iteration count).  Written as one loop with one `run_part` site: each
// turn iterates a part once (CalcDEanalytic, or CalcDEnoADE's first
// iteration and its three gradient samples, or the dIFS loop).
fn evaluate(on_de: bool) -> R {
    let kind = P[DC_KIND];
    let nparts = select(1u, 2u, HAS_DECOMB && on_de && kind != 0u);
    var part = 1u;
    var part_begin = true;
    // the part
    var difs = false;
    var custom = false;
    var hkind = 0u;
    var dds = R(0.0);
    var invert = false;
    var no_vec_ini = false;
    var k = 0u;
    var result = R(0.0);
    var bc = c;
    var buf_rout = R(0.0);
    var buf_lr = R(0.0);
    var buf_v = vec3<R>(0.0);
    var buf_w = R(0.0);
    var buf_sit = false;
    var buf_max_it = 0;
    var g = R(0.0);
    // the first part's values for the combination
    var r1 = R(0.0);
    var wt = R(0.0);
    var dt = R(0.0);
    var buf_it = 0;
    var buf_otrap = R(0.0);
    var buf_sit1 = R(0.0);
    var buf_mir = 0;
    for (var guard = 0u; guard < P[EVAL_TURNS]; guard++) {
        if (part_begin) {
            part_begin = false;
            // (part 1 keeps the caller's values, e.g. RMdoBinSearchIt's
            // max_it + 1, as the CPU, which switches only for part 2)
            if (part == 2u) {
                set_part(2u);
            }
            if (part == 2u) {
                difs = pi(P2_DIFS) != 0;
                custom = pi(P2_CUSTOM) != 0;
                hkind = P[P2_HYB_KIND];
                dds = pf(P2_DDS);
                invert = kind == 3u;
                no_vec_ini = kind == 6u;
            } else {
                difs = pi(DIFS) != 0;
                custom = pi(IS_CUSTOM_DE) != 0;
                hkind = P[HYB_KIND];
                dds = pf(D_DE_SCALE);
                invert = false;
                no_vec_ini = false;
            }
            k = 0u;
            bc = c;
            g = 0.0;
        }
        if (k > 0u) {
            var o = vec3<R>(0.0);
            o[k - 1u] = pf(DE_OFFSET);
            c = bc + o;
        }
        let hres = run_part(difs, on_de && custom, !no_vec_ini, ms_de_stop * pf(STEP_WIDTH) * 1.03, hkind, invert || inside);
        let four_d = (hkind & 1u) != 0u;
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
                if ((inside && it_result == max_it) || rout <= 0.0) {
                    result = 0.0;
                } else {
                    buf_rout = rout;
                    buf_lr = lrout;
                    buf_v = v;
                    buf_w = w;
                    buf_sit = calc_sit;
                    buf_max_it = max_it;
                    calc_sit = false;
                    max_it = it_result;
                    rstop = pf(RSTOP3D);
                    k = 1u;
                    part_done = false;
                }
            } else {
                // the difference of the squared lengths relative to rout, from
                // the vectors themselves (|a|^2 - |b|^2 = (a - b) . (a + b):
                // exact where the squares would lose precision); from the
                // logarithms where the vectors overflowed single precision
                var d: R;
                if (buf_rout < 1e30 && rout < 1e30) {
                    d = dot(buf_v - v, buf_v + v);
                    if (four_d) {
                        d += (buf_w - w) * (buf_w + w);
                    }
                    d /= buf_rout;
                } else {
                    d = 1.0 - r_exp(lrout - buf_lr);
                }
                g += d * d;
                k += 1u;
                if (k < 4u) {
                    part_done = false;
                } else {
                    c = bc;
                    dbg_rout = buf_rout;
                    dbg_g = g;
                    // = buf_rout * r_log(buf_rout) * dds / (sqrt(g * buf_rout^2) + de_offset006)
                    result = buf_lr * dds / (sqrt(g) + pf(DE_OFFSET006) * r_exp(-buf_lr));
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
                // an infinite DE is an escaped point (harmless); NaN is
                // not, the formula's arithmetic broke down
                if (is_nan(result)) {
                    de_overflow = true;
                }
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
                return invert_inside(result, difs);
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
                    result = rst * r_pow(abs(pf(DC_FMIX_POW)), R(-1 - buf_it));
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
        return invert_inside(result, difs);
    }
    return ms_de_stop * 0.25;
}

// Marcher::invert_inside: the distance to the surface from inside the set
fn invert_inside(r: R, difs: bool) -> R {
    if (!inside) {
        return r;
    }
    let ms = ms_de_stop;
    if (difs) {
        return ms * 2.0 - r;
    }
    let q = ms * 4.0 - r * 3.0;
    if (q >= ms) {
        it_result -= 1;
    }
    return q;
}

// rotate_vector_reverse(v, vgrads)
fn rot_rev(a: vec3<R>) -> vec3<R> {
    return a.x * vg(0u) + a.y * vg(1u) + a.z * vg(2u);
}

fn update_de_stop() {
    var z = zz_base + mzz * zz_mul;
    if (zz_abs) {
        z = abs(z);
    }
    ms_de_stop = pf(DE_STOP) * (1.0 + z * pf(DE_STOP_FACTOR));
}

// MakeWNormalsFromDVec
fn store_normal(n: vec3<R>) {
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
fn calc_roughness(n: vec3<R>, dt2: R, dsg: R) -> R {
    let a = (dsg * 7.0 * dt2 * dt2 + 1e-30) / (dot(n, n) + 1e-30);
    return clamp(sqrt(max(a, 0.0)) - 0.05, 0.0, 1.0);
}

// create_xy_vecs_from_normals
fn xy_vecs(n: vec3<R>, vx: ptr<function, vec3<R>>, vy: ptr<function, vec3<R>>) {
    let d = n.y * n.y + n.x * n.x;
    if (d < 1e-30) {
        *vx = vec3<R>(1.0, 0.0, 0.0);
        *vy = vec3<R>(0.0, 1.0, 0.0);
        return;
    }
    let a = r_acos(clamp(-n.z / sqrt(d + n.z * n.z + 1e-30), -1.0, 1.0)) * 0.5;
    let sin_a = r_sin(a) / sqrt(d);
    let cos_a = r_cos(a);
    let dd = -n.y * sin_a;
    let d1 = n.x * sin_a;
    let x = vec3<R>(1.0 - 2.0 * d1 * d1, 2.0 * dd * d1, 2.0 * d1 * cos_a);
    *vx = x;
    *vy = vec3<R>(x.y, 1.0 - 2.0 * dd * dd, -2.0 * dd * cos_a);
}

// RMmaxLengthToCutPlane: the distance (in steps) from the start to the
// cutting planes along the ray (x) and the plane hit (y, 1..3)
fn max_length_to_cut_plane() -> vec2<R> {
    let opts = P[CUT_OPT];
    var len = R(0.0);
    var plane = R(0.0);
    for (var k = 0u; k < 3u; k++) {
        if (((opts >> k) & 1u) == 0u || abs(vfov[k]) <= 1e-20) {
            continue;
        }
        let side = ((opts >> (4u + k)) & 1u) != 0u;
        let neg = (bitcast<u32>(f32(vfov[k])) >> 31u) != 0u;
        if (side == neg) {
            len = 1e20;
        } else {
            let t = (pf(CUT_POS + k) - c[k]) / vfov[k];
            if (t > len) {
                len = t;
                plane = R(k + 1u);
            }
        }
    }
    return vec2<R>(len, plane);
}

// AtCutPlane2: closer than `plus_d` to a cutting plane
fn at_cut_plane(plus_d: R) -> bool {
    for (var k = 0u; k < 3u; k++) {
        if (((P[CUT_OPT] >> k) & 1u) != 0u && abs(c[k] - pf(CUT_POS + k)) - plus_d < 0.0) {
            return true;
        }
    }
    return false;
}

// RMcalcNanglesForCut: depth and normal of a pixel that starts inside the
// set, on the cutting plane `plane` (1..3) or the start plane (0)
fn calc_nangles_for_cut(plane: u32) {
    if (plane > 0u) {
        let nn = 8388352.0 - pf(ZC_MUL) * (sqrt(mzz * pf(ZCORR) + 1.0) - 1.0);
        si_zpos = u32(max(round(nn), 0.0)) << 8u;
        let k = plane - 1u;
        let g = vg(2u)[k];
        let m = select(-1.0 / g, -1e30, abs(g) < 1e-30);
        let n = vec3<R>(vg(0u)[k] * m, vg(1u)[k] * m, -1.0);
        let l = 32767.0 / sqrt(dot(n, n) + 1e-30);
        si_normal = vec3<i32>(round(n * l));
    } else {
        si_zpos = 0x7FFF0000u;
        si_normal = vec3<i32>(0, 0, -32767);
    }
}

// RMdoColor
fn do_color() {
    let x = v.x;
    let y = v.y;
    let z = v.z;
    var s: R;
    switch (pi(COLOR_OPTION)) {
        case 1: {
            s = r_log(rout / (rold + 1.0)) * pf(MCT_COLOR_MUL);
        }
        case 2: {
            s = (r_atan2(y - c.y, x - c.x) + PI) * 5200.0;
        }
        case 3: {
            s = (r_atan2(z - c.z, x - c.x) + PI) * 5200.0;
        }
        case 4: {
            s = (r_atan2(z - c.z, y - c.y) + PI) * 5200.0;
        }
        case 5: {
            s = (r_atan2(x, y) + PI) * 5215.0;
            let r = sqrt(x * x + y * y + 1e-30 + z * z);
            let s2 = (PI + r_asin(clamp(z / r, -1.0, 1.0)) * 2.0) * 5215.0;
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
fn get_rand() -> R {
    seed = seed * 0x343FD + 0x269EC3;
    return R((u32(seed) >> 8u) & 0x7FFFFFu) / R(0x7FFFFFu);
}

// -BuildRotMatrixS(0, ya, za)[2]
fn ray_dir(ya: R, za: R) -> vec3<R> {
    return vec3<R>(r_sin(ya) * r_cos(za), -r_sin(ya) * r_sin(za), -r_cos(ya));
}

// MakeRotQuatFromSNormals + CreateSMatrixFromQuat (deao.rs normal_matrix)
fn normal_matrix(n: vec3<R>) -> mat3x3<R> {
    let a = r_acos(clamp(-n.z, -1.0, 1.0)) * 0.5;
    var sa = r_sin(a);
    let ca = r_cos(a);
    let nn = sqrt(n.y * n.y + n.x * n.x);
    var q = vec4<R>(0.0, 0.0, 0.0, ca);
    if (nn >= 1e-25) {
        sa /= nn;
        q = vec4<R>(-n.y * sa, n.x * sa, 0.0, ca);
    }
    // the rows of the CPU's matrix (m[i] dot d)
    return mat3x3<R>(
        vec3<R>(1.0 - 2.0 * (q.y * q.y + q.z * q.z), 2.0 * (q.x * q.y + q.z * q.w), 2.0 * (q.x * q.z - q.y * q.w)),
        vec3<R>(2.0 * (q.x * q.y - q.z * q.w), 1.0 - 2.0 * (q.x * q.x + q.z * q.z), 2.0 * (q.z * q.y + q.x * q.w)),
        vec3<R>(2.0 * (q.x * q.z + q.y * q.w), 2.0 * (q.y * q.z - q.x * q.w), 1.0 - 2.0 * (q.x * q.x + q.y * q.y)));
}

// MaxLHS
fn hs_max_len(y: i32) -> R {
    return R(pi(WIDTH) + y) * 0.6 * (1.0 + 0.5 * min(mzz, pf(ZEND) * 0.4) * max(pf(FOV_Y), 0.0) / R(pi(HEIGHT))) * pf(HS_MAX_LEN_MUL);
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
// the reflection pass (reflect.rs calc_ray)
const P_R_START: u32 = 35u;  // the task: the pixel's surface or a bounce
const P_R_SURF_R: u32 = 36u; // result: the DE at the stored depth of the surface
const P_R_SURF_END: u32 = 37u;
const P_R_M0: u32 = 38u;     // result: the DE at the start of the bounce
const P_R_LOOP: u32 = 39u;   // a step along the bounce
const P_R_STEP: u32 = 40u;   // result: the DE after the step
const P_R_HIT: u32 = 41u;    // the surface: the binary search
const P_R_HIT2: u32 = 42u;   // the normals, colour and depth of the hit
const P_R_AFTER_N: u32 = 43u; // then the shadows
const P_R_AFTER_HS: u32 = 44u; // then the ambient occlusion
const P_DONE: u32 = 45u;
const P_R_END: u32 = 46u;    // the outputs of the task
const P_CUT: u32 = 47u;      // the march: to the cutting planes, then the DE at the start
const P_IN_COL: u32 = 48u;   // the pixel starts inside the set: its colour
const P_IN_SIT: u32 = 49u;   // result: the smoothed iteration count (inside rendering)
const P_COI: u32 = 50u;      // doColorOnIt, then after_coi
const P_COI_R: u32 = 51u;    // result of its iterations
const P_SURF_COL: u32 = 52u; // the surface: colour and depth
const P_R_POST: u32 = 53u;   // the second reflection pipeline: the hit from the task record


// The march of a pixel (M_MARCH), or a post calculation on the G-buffer
// pixel in si_* (M_SHADOW: the shadow bits, M_DEAO: si_amb); the DE is
// evaluated at the single site of `evaluate`.
fn pixel(x: i32, y: i32, mode: u32) {
    set_part(1u);
    rstop = pf(D_RSTOP);
    max_its_result = max_it;

    // RMCalculateVgradsFOV
    let cafy = (R(y) / R(pi(HEIGHT)) - 0.5) * pf(FOV_Y);
    let cafx = (pf(FOVX_OFF) - R(x + 1)) * pf(FOVX_MUL);
    var dv: vec3<R>;
    let optic = pi(OPTIC);
    if (optic == 1) {
        dv = normalize(vec3<R>(-cafx, cafy, pf(PL_OPTIC_Z)));
    } else if (optic == 2) {
        dv = vec3<R>(-r_sin(cafx) * r_cos(cafy), r_sin(cafy), r_cos(cafy) * r_cos(cafx));
    } else {
        dv = normalize(vec3<R>(-r_sin(cafx), r_sin(cafy), r_cos(cafy) * r_cos(cafx)));
    }
    vfov = rot_rev(dv);
    // RMCalculateStartPos
    if (optic == 2) {
        base = p_ystart();
    } else {
        base = p_ystart() + vg(0u) * R(x) + vg(1u) * R(y);
    }
    calc_sit = false;
    var step_count = R(0.0);
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
    // the post passes use the scene's setting for every pixel (the CPU's
    // Marcher::new); the march may switch it (inside and outside)
    inside = (P[INSIDE_FLAGS] & 1u) != 0u;
    var cut_plane = 0u;
    var after_coi = P_DONE;
    var coi_max = 0;
    if (mode == M_MARCH) {
        pc = P_CUT;
        need = false;
    } else if (mode == M_SHADOW || mode == M_DEAO) {
        pc = P_PS;
        need = false;
    } else if (mode == M_REFLECT) {
        pc = select(P_R_POST, P_R_START, R_MARCH);
        need = false;
    }
    vfov_px = vfov;
    zz_base = 0.0;
    zz_mul = 1.0;
    zz_abs = mode == M_REFLECT;
    let reflect = mode == M_REFLECT;
    var after_hs = P_DONE;
    var after_ao = P_DONE;
    var last_zz = R(0.0);
    var rlast_sw = R(0.0);
    var after_bs = P_N_INIT;
    var bs_thr = R(0.001);
    var bs_it_check = mode != M_DEAO;
    var zf = R(0.0);
    // shadows
    let soft = pf(HS_SOFT_RADIUS) > 0.0;
    var ic = vec3<R>(0.0);
    var nvec = vec3<R>(0.0);
    var max_lhs = R(0.0);
    var li = 0u;
    var lidx = 0u;
    var lvec = vec3<R>(0.0);
    var zz2 = R(0.0);
    var zz2mul = R(0.0);
    var rsf = R(1.0);
    var st = R(0.0);
    var zr_soft = R(1.0);
    var zrs_mul = R(0.0);
    // ambient occlusion
    var ray_count = 0u;
    var k = 0u;
    var step_ao = R(0.0);
    var max_dist = R(0.0);
    var d_step_mul = R(1.0);
    var ms2 = R(0.0);
    var de_mul = R(1.0);
    var abr_c = R(0.0);
    var md_d10 = R(0.0);
    var d_min_a_dif = R(-1.0);
    var corr_w = R(0.0);
    var rot_w = mat3x3<R>();
    var rv0 = vec3<R>(0.0);
    var rv1 = vec3<R>(0.0);
    var rv2 = vec3<R>(0.0);
    var sv = vec3<R>(0.0);
    var s_tmp = R(1.0);
    var b_end = false;
    var b_first = false;
    var r = R(0.0);
    var done = false;
    // march
    var dtmp = R(0.0);
    var rsf_mul = R(1.0);
    var last_step = R(0.0);
    var last_de = R(0.0);
    var dt1 = R(0.0);
    var de_limited = false;
    var itmp = 0;
    // RMdoBinSearchIt
    var yp = R(0.0);
    var saved_max = 0;
    var dmul = R(1.0);
    var first = true;
    var last_si = R(0.0);
    var last_dif = R(0.0);
    // normals
    var noffset = R(0.0);
    var ct1 = vec3<R>(0.0);
    var dnn = R(0.0);
    var nn = R(0.0);
    var n = vec3<R>(0.0);
    var cnt = 0;
    var na = R(0.0);
    var acc = R(0.0);
    var ssn = R(0.0);
    var dm = R(0.0);
    var vx = vec3<R>(0.0);
    var vy = vec3<R>(0.0);
    var s1 = vec2<R>(0.0);
    var ds = vec2<R>(0.0);
    var axis = 0;
    var kk = 0;

    // resumable marching (STATE): the state of every variable that lives
    // across turns (a test checks the lists against the declarations)
    // word k of the slot at k * ns (the threads of a warp access adjacent words)
    let so = state_slot;
    let ns = P[N_SLOTS];
    var guard = 0;
    if (resuming) {
        c.x = rget(so + 1u * ns, ns);
        c.y = rget(so + 3u * ns, ns);
        c.z = rget(so + 5u * ns, ns);
        v.x = rget(so + 7u * ns, ns);
        v.y = rget(so + 9u * ns, ns);
        v.z = rget(so + 11u * ns, ns);
        rout = rget(so + 13u * ns, ns);
        lrout = rget(so + 15u * ns, ns);
        rold = rget(so + 17u * ns, ns);
        otrap = rget(so + 19u * ns, ns);
        it_result = bitcast<i32>(STATE[so + 21u * ns]);
        max_it = bitcast<i32>(STATE[so + 22u * ns]);
        rstop = rget(so + 23u * ns, ns);
        calc_sit = (STATE[so + 25u * ns] != 0u);
        smooth_it = rget(so + 26u * ns, ns);
        w = rget(so + 28u * ns, ns);
        j.x = rget(so + 30u * ns, ns);
        j.y = rget(so + 32u * ns, ns);
        j.z = rget(so + 34u * ns, ns);
        j4 = rget(so + 36u * ns, ns);
        vary_scale = rget(so + 38u * ns, ns);
        first_it = bitcast<i32>(STATE[so + 40u * ns]);
        deriv.x = rget(so + 41u * ns, ns);
        deriv.y = rget(so + 43u * ns, ns);
        deriv.z = rget(so + 45u * ns, ns);
        dfree.x = rget(so + 47u * ns, ns);
        dfree.y = rget(so + 49u * ns, ns);
        ms_de_stop = rget(so + 51u * ns, ns);
        part_start = STATE[so + 53u * ns];
        part_end = STATE[so + 54u * ns];
        part_repeat = STATE[so + 55u * ns];
        de_option = bitcast<i32>(STATE[so + 56u * ns]);
        rstopd = rget(so + 57u * ns, ns);
        vfov_px.x = rget(so + 59u * ns, ns);
        vfov_px.y = rget(so + 61u * ns, ns);
        vfov_px.z = rget(so + 63u * ns, ns);
        zz_base = rget(so + 65u * ns, ns);
        zz_mul = rget(so + 67u * ns, ns);
        zz_abs = (STATE[so + 69u * ns] != 0u);
        r_flags = STATE[so + 70u * ns];
        r_zz2 = rget(so + 71u * ns, ns);
        r_step_count = rget(so + 73u * ns, ns);
        de_overflow = (STATE[so + 75u * ns] != 0u);
        inside = (STATE[so + 76u * ns] != 0u);
        mzz = rget(so + 77u * ns, ns);
        vfov.x = rget(so + 79u * ns, ns);
        vfov.y = rget(so + 81u * ns, ns);
        vfov.z = rget(so + 83u * ns, ns);
        base.x = rget(so + 85u * ns, ns);
        base.y = rget(so + 87u * ns, ns);
        base.z = rget(so + 89u * ns, ns);
        max_its_result = bitcast<i32>(STATE[so + 91u * ns]);
        s_roughness = rget(so + 92u * ns, ns);
        seed = bitcast<i32>(STATE[so + 94u * ns]);
        si_normal.x = bitcast<i32>(STATE[so + 95u * ns]);
        si_normal.y = bitcast<i32>(STATE[so + 96u * ns]);
        si_normal.z = bitcast<i32>(STATE[so + 97u * ns]);
        si_zpos = STATE[so + 98u * ns];
        si_shadow = STATE[so + 99u * ns];
        si_grad = STATE[so + 100u * ns];
        si_otrap = STATE[so + 101u * ns];
        si_amb = STATE[so + 102u * ns];
        for (var i = 0u; i < 33u; i++) { let q = so + (103u + 6u * i) * ns; ao_dirs[i] = vec3<R>(rget(q, ns), rget(q + 2u * ns, ns), rget(q + 4u * ns, ns)); }
        for (var i = 0u; i < 33u; i++) { ao_min[i] = rget(so + (301u + 2u * i) * ns, ns); }
        step_count = rget(so + 367u * ns, ns);
        first_step = (STATE[so + 369u * ns] != 0u);
        pc = STATE[so + 370u * ns];
        need = (STATE[so + 371u * ns] != 0u);
        want_de = (STATE[so + 372u * ns] != 0u);
        cut_plane = STATE[so + 373u * ns];
        after_coi = STATE[so + 374u * ns];
        coi_max = bitcast<i32>(STATE[so + 375u * ns]);
        after_hs = STATE[so + 376u * ns];
        after_ao = STATE[so + 377u * ns];
        last_zz = rget(so + 378u * ns, ns);
        rlast_sw = rget(so + 380u * ns, ns);
        after_bs = STATE[so + 382u * ns];
        bs_thr = rget(so + 383u * ns, ns);
        bs_it_check = (STATE[so + 385u * ns] != 0u);
        zf = rget(so + 386u * ns, ns);
        ic.x = rget(so + 388u * ns, ns);
        ic.y = rget(so + 390u * ns, ns);
        ic.z = rget(so + 392u * ns, ns);
        nvec.x = rget(so + 394u * ns, ns);
        nvec.y = rget(so + 396u * ns, ns);
        nvec.z = rget(so + 398u * ns, ns);
        max_lhs = rget(so + 400u * ns, ns);
        li = STATE[so + 402u * ns];
        lidx = STATE[so + 403u * ns];
        lvec.x = rget(so + 404u * ns, ns);
        lvec.y = rget(so + 406u * ns, ns);
        lvec.z = rget(so + 408u * ns, ns);
        zz2 = rget(so + 410u * ns, ns);
        zz2mul = rget(so + 412u * ns, ns);
        rsf = rget(so + 414u * ns, ns);
        st = rget(so + 416u * ns, ns);
        zr_soft = rget(so + 418u * ns, ns);
        zrs_mul = rget(so + 420u * ns, ns);
        ray_count = STATE[so + 422u * ns];
        k = STATE[so + 423u * ns];
        step_ao = rget(so + 424u * ns, ns);
        max_dist = rget(so + 426u * ns, ns);
        d_step_mul = rget(so + 428u * ns, ns);
        ms2 = rget(so + 430u * ns, ns);
        de_mul = rget(so + 432u * ns, ns);
        abr_c = rget(so + 434u * ns, ns);
        md_d10 = rget(so + 436u * ns, ns);
        d_min_a_dif = rget(so + 438u * ns, ns);
        corr_w = rget(so + 440u * ns, ns);
        rot_w[0].x = rget(so + 442u * ns, ns);
        rot_w[0].y = rget(so + 444u * ns, ns);
        rot_w[0].z = rget(so + 446u * ns, ns);
        rot_w[1].x = rget(so + 448u * ns, ns);
        rot_w[1].y = rget(so + 450u * ns, ns);
        rot_w[1].z = rget(so + 452u * ns, ns);
        rot_w[2].x = rget(so + 454u * ns, ns);
        rot_w[2].y = rget(so + 456u * ns, ns);
        rot_w[2].z = rget(so + 458u * ns, ns);
        rv0.x = rget(so + 460u * ns, ns);
        rv0.y = rget(so + 462u * ns, ns);
        rv0.z = rget(so + 464u * ns, ns);
        rv1.x = rget(so + 466u * ns, ns);
        rv1.y = rget(so + 468u * ns, ns);
        rv1.z = rget(so + 470u * ns, ns);
        rv2.x = rget(so + 472u * ns, ns);
        rv2.y = rget(so + 474u * ns, ns);
        rv2.z = rget(so + 476u * ns, ns);
        sv.x = rget(so + 478u * ns, ns);
        sv.y = rget(so + 480u * ns, ns);
        sv.z = rget(so + 482u * ns, ns);
        s_tmp = rget(so + 484u * ns, ns);
        b_end = (STATE[so + 486u * ns] != 0u);
        b_first = (STATE[so + 487u * ns] != 0u);
        r = rget(so + 488u * ns, ns);
        done = (STATE[so + 490u * ns] != 0u);
        dtmp = rget(so + 491u * ns, ns);
        rsf_mul = rget(so + 493u * ns, ns);
        last_step = rget(so + 495u * ns, ns);
        last_de = rget(so + 497u * ns, ns);
        dt1 = rget(so + 499u * ns, ns);
        de_limited = (STATE[so + 501u * ns] != 0u);
        itmp = bitcast<i32>(STATE[so + 502u * ns]);
        yp = rget(so + 503u * ns, ns);
        saved_max = bitcast<i32>(STATE[so + 505u * ns]);
        dmul = rget(so + 506u * ns, ns);
        first = (STATE[so + 508u * ns] != 0u);
        last_si = rget(so + 509u * ns, ns);
        last_dif = rget(so + 511u * ns, ns);
        noffset = rget(so + 513u * ns, ns);
        ct1.x = rget(so + 515u * ns, ns);
        ct1.y = rget(so + 517u * ns, ns);
        ct1.z = rget(so + 519u * ns, ns);
        dnn = rget(so + 521u * ns, ns);
        nn = rget(so + 523u * ns, ns);
        n.x = rget(so + 525u * ns, ns);
        n.y = rget(so + 527u * ns, ns);
        n.z = rget(so + 529u * ns, ns);
        cnt = bitcast<i32>(STATE[so + 531u * ns]);
        na = rget(so + 532u * ns, ns);
        acc = rget(so + 534u * ns, ns);
        ssn = rget(so + 536u * ns, ns);
        dm = rget(so + 538u * ns, ns);
        vx.x = rget(so + 540u * ns, ns);
        vx.y = rget(so + 542u * ns, ns);
        vx.z = rget(so + 544u * ns, ns);
        vy.x = rget(so + 546u * ns, ns);
        vy.y = rget(so + 548u * ns, ns);
        vy.z = rget(so + 550u * ns, ns);
        s1.x = rget(so + 552u * ns, ns);
        s1.y = rget(so + 554u * ns, ns);
        ds.x = rget(so + 556u * ns, ns);
        ds.y = rget(so + 558u * ns, ns);
        axis = bitcast<i32>(STATE[so + 560u * ns]);
        kk = bitcast<i32>(STATE[so + 561u * ns]);
        guard = bitcast<i32>(STATE[so + 562u * ns]);
        load_record(so + (1u + STATE_FIXED) * ns, ns);
    }
    for (var turns = 0u; guard < 2000000; guard++) {
        if (turns >= P[TURN_BUDGET]) {
            rput(so + 1u * ns, ns, c.x);
            rput(so + 3u * ns, ns, c.y);
            rput(so + 5u * ns, ns, c.z);
            rput(so + 7u * ns, ns, v.x);
            rput(so + 9u * ns, ns, v.y);
            rput(so + 11u * ns, ns, v.z);
            rput(so + 13u * ns, ns, rout);
            rput(so + 15u * ns, ns, lrout);
            rput(so + 17u * ns, ns, rold);
            rput(so + 19u * ns, ns, otrap);
            STATE[so + 21u * ns] = bitcast<u32>(it_result);
            STATE[so + 22u * ns] = bitcast<u32>(max_it);
            rput(so + 23u * ns, ns, rstop);
            STATE[so + 25u * ns] = select(0u, 1u, calc_sit);
            rput(so + 26u * ns, ns, smooth_it);
            rput(so + 28u * ns, ns, w);
            rput(so + 30u * ns, ns, j.x);
            rput(so + 32u * ns, ns, j.y);
            rput(so + 34u * ns, ns, j.z);
            rput(so + 36u * ns, ns, j4);
            rput(so + 38u * ns, ns, vary_scale);
            STATE[so + 40u * ns] = bitcast<u32>(first_it);
            rput(so + 41u * ns, ns, deriv.x);
            rput(so + 43u * ns, ns, deriv.y);
            rput(so + 45u * ns, ns, deriv.z);
            rput(so + 47u * ns, ns, dfree.x);
            rput(so + 49u * ns, ns, dfree.y);
            rput(so + 51u * ns, ns, ms_de_stop);
            STATE[so + 53u * ns] = part_start;
            STATE[so + 54u * ns] = part_end;
            STATE[so + 55u * ns] = part_repeat;
            STATE[so + 56u * ns] = bitcast<u32>(de_option);
            rput(so + 57u * ns, ns, rstopd);
            rput(so + 59u * ns, ns, vfov_px.x);
            rput(so + 61u * ns, ns, vfov_px.y);
            rput(so + 63u * ns, ns, vfov_px.z);
            rput(so + 65u * ns, ns, zz_base);
            rput(so + 67u * ns, ns, zz_mul);
            STATE[so + 69u * ns] = select(0u, 1u, zz_abs);
            STATE[so + 70u * ns] = r_flags;
            rput(so + 71u * ns, ns, r_zz2);
            rput(so + 73u * ns, ns, r_step_count);
            STATE[so + 75u * ns] = select(0u, 1u, de_overflow);
            STATE[so + 76u * ns] = select(0u, 1u, inside);
            rput(so + 77u * ns, ns, mzz);
            rput(so + 79u * ns, ns, vfov.x);
            rput(so + 81u * ns, ns, vfov.y);
            rput(so + 83u * ns, ns, vfov.z);
            rput(so + 85u * ns, ns, base.x);
            rput(so + 87u * ns, ns, base.y);
            rput(so + 89u * ns, ns, base.z);
            STATE[so + 91u * ns] = bitcast<u32>(max_its_result);
            rput(so + 92u * ns, ns, s_roughness);
            STATE[so + 94u * ns] = bitcast<u32>(seed);
            STATE[so + 95u * ns] = bitcast<u32>(si_normal.x);
            STATE[so + 96u * ns] = bitcast<u32>(si_normal.y);
            STATE[so + 97u * ns] = bitcast<u32>(si_normal.z);
            STATE[so + 98u * ns] = si_zpos;
            STATE[so + 99u * ns] = si_shadow;
            STATE[so + 100u * ns] = si_grad;
            STATE[so + 101u * ns] = si_otrap;
            STATE[so + 102u * ns] = si_amb;
            for (var i = 0u; i < 33u; i++) { let q = so + (103u + 6u * i) * ns; rput(q, ns, ao_dirs[i].x); rput(q + 2u * ns, ns, ao_dirs[i].y); rput(q + 4u * ns, ns, ao_dirs[i].z); }
            for (var i = 0u; i < 33u; i++) { rput(so + (301u + 2u * i) * ns, ns, ao_min[i]); }
            rput(so + 367u * ns, ns, step_count);
            STATE[so + 369u * ns] = select(0u, 1u, first_step);
            STATE[so + 370u * ns] = pc;
            STATE[so + 371u * ns] = select(0u, 1u, need);
            STATE[so + 372u * ns] = select(0u, 1u, want_de);
            STATE[so + 373u * ns] = cut_plane;
            STATE[so + 374u * ns] = after_coi;
            STATE[so + 375u * ns] = bitcast<u32>(coi_max);
            STATE[so + 376u * ns] = after_hs;
            STATE[so + 377u * ns] = after_ao;
            rput(so + 378u * ns, ns, last_zz);
            rput(so + 380u * ns, ns, rlast_sw);
            STATE[so + 382u * ns] = after_bs;
            rput(so + 383u * ns, ns, bs_thr);
            STATE[so + 385u * ns] = select(0u, 1u, bs_it_check);
            rput(so + 386u * ns, ns, zf);
            rput(so + 388u * ns, ns, ic.x);
            rput(so + 390u * ns, ns, ic.y);
            rput(so + 392u * ns, ns, ic.z);
            rput(so + 394u * ns, ns, nvec.x);
            rput(so + 396u * ns, ns, nvec.y);
            rput(so + 398u * ns, ns, nvec.z);
            rput(so + 400u * ns, ns, max_lhs);
            STATE[so + 402u * ns] = li;
            STATE[so + 403u * ns] = lidx;
            rput(so + 404u * ns, ns, lvec.x);
            rput(so + 406u * ns, ns, lvec.y);
            rput(so + 408u * ns, ns, lvec.z);
            rput(so + 410u * ns, ns, zz2);
            rput(so + 412u * ns, ns, zz2mul);
            rput(so + 414u * ns, ns, rsf);
            rput(so + 416u * ns, ns, st);
            rput(so + 418u * ns, ns, zr_soft);
            rput(so + 420u * ns, ns, zrs_mul);
            STATE[so + 422u * ns] = ray_count;
            STATE[so + 423u * ns] = k;
            rput(so + 424u * ns, ns, step_ao);
            rput(so + 426u * ns, ns, max_dist);
            rput(so + 428u * ns, ns, d_step_mul);
            rput(so + 430u * ns, ns, ms2);
            rput(so + 432u * ns, ns, de_mul);
            rput(so + 434u * ns, ns, abr_c);
            rput(so + 436u * ns, ns, md_d10);
            rput(so + 438u * ns, ns, d_min_a_dif);
            rput(so + 440u * ns, ns, corr_w);
            rput(so + 442u * ns, ns, rot_w[0].x);
            rput(so + 444u * ns, ns, rot_w[0].y);
            rput(so + 446u * ns, ns, rot_w[0].z);
            rput(so + 448u * ns, ns, rot_w[1].x);
            rput(so + 450u * ns, ns, rot_w[1].y);
            rput(so + 452u * ns, ns, rot_w[1].z);
            rput(so + 454u * ns, ns, rot_w[2].x);
            rput(so + 456u * ns, ns, rot_w[2].y);
            rput(so + 458u * ns, ns, rot_w[2].z);
            rput(so + 460u * ns, ns, rv0.x);
            rput(so + 462u * ns, ns, rv0.y);
            rput(so + 464u * ns, ns, rv0.z);
            rput(so + 466u * ns, ns, rv1.x);
            rput(so + 468u * ns, ns, rv1.y);
            rput(so + 470u * ns, ns, rv1.z);
            rput(so + 472u * ns, ns, rv2.x);
            rput(so + 474u * ns, ns, rv2.y);
            rput(so + 476u * ns, ns, rv2.z);
            rput(so + 478u * ns, ns, sv.x);
            rput(so + 480u * ns, ns, sv.y);
            rput(so + 482u * ns, ns, sv.z);
            rput(so + 484u * ns, ns, s_tmp);
            STATE[so + 486u * ns] = select(0u, 1u, b_end);
            STATE[so + 487u * ns] = select(0u, 1u, b_first);
            rput(so + 488u * ns, ns, r);
            STATE[so + 490u * ns] = select(0u, 1u, done);
            rput(so + 491u * ns, ns, dtmp);
            rput(so + 493u * ns, ns, rsf_mul);
            rput(so + 495u * ns, ns, last_step);
            rput(so + 497u * ns, ns, last_de);
            rput(so + 499u * ns, ns, dt1);
            STATE[so + 501u * ns] = select(0u, 1u, de_limited);
            STATE[so + 502u * ns] = bitcast<u32>(itmp);
            rput(so + 503u * ns, ns, yp);
            STATE[so + 505u * ns] = bitcast<u32>(saved_max);
            rput(so + 506u * ns, ns, dmul);
            STATE[so + 508u * ns] = select(0u, 1u, first);
            rput(so + 509u * ns, ns, last_si);
            rput(so + 511u * ns, ns, last_dif);
            rput(so + 513u * ns, ns, noffset);
            rput(so + 515u * ns, ns, ct1.x);
            rput(so + 517u * ns, ns, ct1.y);
            rput(so + 519u * ns, ns, ct1.z);
            rput(so + 521u * ns, ns, dnn);
            rput(so + 523u * ns, ns, nn);
            rput(so + 525u * ns, ns, n.x);
            rput(so + 527u * ns, ns, n.y);
            rput(so + 529u * ns, ns, n.z);
            STATE[so + 531u * ns] = bitcast<u32>(cnt);
            rput(so + 532u * ns, ns, na);
            rput(so + 534u * ns, ns, acc);
            rput(so + 536u * ns, ns, ssn);
            rput(so + 538u * ns, ns, dm);
            rput(so + 540u * ns, ns, vx.x);
            rput(so + 542u * ns, ns, vx.y);
            rput(so + 544u * ns, ns, vx.z);
            rput(so + 546u * ns, ns, vy.x);
            rput(so + 548u * ns, ns, vy.y);
            rput(so + 550u * ns, ns, vy.z);
            rput(so + 552u * ns, ns, s1.x);
            rput(so + 554u * ns, ns, s1.y);
            rput(so + 556u * ns, ns, ds.x);
            rput(so + 558u * ns, ns, ds.y);
            STATE[so + 560u * ns] = bitcast<u32>(axis);
            STATE[so + 561u * ns] = bitcast<u32>(kk);
            STATE[so + 562u * ns] = bitcast<u32>(guard);
            save_record(so + (1u + STATE_FIXED) * ns, ns);
            STATE[so] = cur_item + 1u;
            atomicAdd(&QUEUE[0], 1u);
            atomicMin(&QUEUE[2], cur_item);
            suspended = true;
            return;
        }
        // an evaluation costs more than a step of the state machine
        turns += select(1u, 8u, need);
        if (need) {
            r = evaluate(want_de);
            need = false;
        }
        switch (pc) {
            case P_START: {
                if (MODE_KIND == 0u) {
                    dtmp = r;
                    if (mode == M_PROBE) {
                        si_zpos = bitcast<u32>(f32(r));
                        si_grad = u32(it_result) & 0xFFFFu;
                        let rb = bitcast<u32>(f32(dbg_rst));
                        si_normal = vec3<i32>(i32(rb << 16u) >> 16u, i32(rb) >> 16u, dbg_it2);
                        if (P[PROBE_MZZ + 1u] != 0u) {
                            // the details of the numerical DE of part 1 instead
                            let gb = bitcast<u32>(f32(dbg_g));
                            si_normal = vec3<i32>(i32(gb << 16u) >> 16u, i32(gb) >> 16u, i32(round(r_log(dbg_rout))));
                        }
                        let rb1 = bitcast<u32>(f32(dbg_r1));
                        si_amb = rb1 & 0xFFFFu;
                        si_shadow = rb1 >> 16u;
                        si_otrap = u32(dbg_it1) & 0xFFFFu;
                        return;
                    }
                    let start_in = it_result >= max_its_result || dtmp < ms_de_stop;
                    if (start_in && (P[INSIDE_FLAGS] & 2u) != 0u && inside == ((P[INSIDE_FLAGS] & 1u) != 0u)) {
                        // inside and outside: the start is in the set, so
                        // render from the other side (once)
                        inside = !inside;
                        need = true;
                        want_de = true;
                    } else if (start_in) {
                        // inside the set at the start plane (or the cutting plane)
                        after_coi = P_IN_COL;
                        pc = select(P_IN_COL, P_COI, P[COLOR_ON_IT] != 0u);
                    } else {
                        rsf_mul = 1.0;
                        last_step = dtmp * pf(Z_STEP_DIV);
                        pc = P_LOOP;
                    }
                }
            }
            case P_CUT: {
                if (MODE_KIND == 0u) {
                    // move to the begin of the cutting planes
                    if (P[CUT_OPT] != 0u) {
                        let cl = max_length_to_cut_plane();
                        cut_plane = u32(cl.y);
                        if (cl.x > pf(ZEND) - mzz) {
                            return; // background
                        }
                        mzz = cl.x;
                        setc();
                        update_de_stop();
                    }
                    need = true;
                    want_de = true;
                    pc = P_START;
                }
            }
            case P_IN_COL: {
                if (MODE_KIND == 0u) {
                    calc_nangles_for_cut(cut_plane);
                    do_color();
                    if (pi(COLOR_OPTION) > 4) {
                        si_grad |= 32768u;
                        return;
                    } else if (inside) {
                        calc_sit = true;
                        need = true;
                        want_de = true;
                        pc = P_IN_SIT;
                    } else {
                        let t = clamp(rout / pf(D_RSTOP), 0.0, 1.0);
                        si_grad = 32768u + u32(round(32767.0 * t));
                        return;
                    }
                }
            }
            case P_IN_SIT: {
                if (MODE_KIND == 0u) {
                    calc_sit = false;
                    si_grad = 32768u | clip15(smooth_it * pf(MCTS_M));
                    return;
                }
            }
            // doColorOnIt: the start vector, or the vector after
            // COLOR_ON_IT - 1 iterations
            case P_COI: {
                if (MODE_KIND == 0u) {
                    if (P[COLOR_ON_IT] == 1u) {
                        v = c;
                        pc = after_coi;
                    } else {
                        coi_max = max_it;
                        max_it = i32(P[COLOR_ON_IT]) - 1;
                        rstop = pf(RSTOP3D);
                        need = true;
                        want_de = true;
                        pc = P_COI_R;
                    }
                }
            }
            case P_COI_R: {
                if (MODE_KIND == 0u) {
                    max_it = coi_max;
                    rstop = pf(D_RSTOP);
                    pc = after_coi;
                }
            }
            case P_SURF_COL: {
                if (MODE_KIND == 0u) {
                    do_color();
                    calc_zpos_and_rough();
                    if ((P[INSIDE_FLAGS] & 2u) != 0u && !inside) {
                        si_otrap |= 0x8000u;
                    }
                    done = true;
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
                            d *= R(u32(seed) & 0x7FFFFFFFu) * (1.0 / 2147483647.0);
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
                                yp = R(max_it) - 0.99;
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
                if (bs_it_check && it_result >= max_its_result) {
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
                if (smooth_it > R(max_it) - 0.1) {
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
                    dt1 = select(R(0.0), R(1.0), last_si < smooth_it) - 0.5;
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
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
                    noffset = min(pf(DE_STOP), 1.0) * (1.0 + mzz * pf(DE_STOP_FACTOR)) * 0.15;
                    calc_sit = true;
                    ct1 = c;
                    need = true;
                    want_de = on_de;
                    pc = P_N_0;
                }
            }
            case P_N_0: {
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
                    dnn = r;
                    nn = smooth_it;
                    if (on_de) {
                        calc_sit = false;
                    }
                    n = vec3<R>(0.0);
                    cnt = 0;
                    if (sm == 8) {
                        pc = P_N8;
                    } else {
                        pc = P_NA;
                    }
                }
            }
            case P_N8: {
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
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
                            c = ct1 + vg(2u) * (R(a) * sn) + vg(1u) * (R(b) * sn) + vg(0u) * (R(cc) * sn);
                            need = true;
                            want_de = on_de;
                            pc = P_N8_R;
                        }
                    }
                }
            }
            case P_N8_R: {
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
                    let a = cnt / 25 - 2;
                    let b = (cnt / 5) % 5 - 2;
                    let cc = cnt % 5 - 2;
                    let d = r * select(R(-1.0), R(1.0), on_de);
                    if (a != 0) { n.z += d / R(a); }
                    if (b != 0) { n.y += d / R(b); }
                    if (cc != 0) { n.x += d / R(cc); }
                    cnt += 1;
                    pc = P_N8;
                }
            }
            case P_NA: {
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
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
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
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
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
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
                            ssn = noffset * 3.0 / (R(sm) + 0.5);
                            dm = R(sm) * 2.0;
                            xy_vecs(n, &vx, &vy);
                            vx = rot_rev(vx);
                            vy = rot_rev(vy);
                            s1 = vec2<R>(0.0);
                            ds = vec2<R>(0.0);
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
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
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
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
                    if (axis >= 2) {
                        // = ds1 * dm - nn1^2 + ds2 * dm - nn2^2 of the CPU
                        let dsg = (ds.x + ds.y) * dm;
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
                        c = ct1 + dir * (R(kk) * ssn);
                        need = true;
                        want_de = on_de;
                        pc = P_SW_R;
                    }
                }
            }
            case P_SW_R: {
                if (MODE_KIND == 0u || (MODE_KIND == 3u && R_MARCH)) {
                    var d: R;
                    if (on_de) {
                        d = (r - dnn) / R(kk);
                    } else {
                        d = (dnn - r) / R(kk);
                    }
                    // ds: the sum of squared deviations from the mean
                    // (Welford; the CPU's n * sum(d^2) - sum(d)^2 cancels
                    // in single precision and came out too small)
                    let cprev = R(kk + sm - select(0, 1, kk > 0));
                    let mean_old = select(0.0, s1[axis] / cprev, cprev > 0.0);
                    s1[axis] += d;
                    ds[axis] += (d - mean_old) * (d - s1[axis] / (cprev + 1.0));
                    kk += 1;
                    pc = P_SW;
                }
            }
            case P_NEND: {
                if (MODE_KIND == 0u) {
                    c = ct1;
                    // (the normals on the smoothed iteration count point
                    // out of the set: inverted inside, the DE ones are not)
                    store_normal(select(n, -n, inside && !on_de));
                    var g: R;
                    if (de_limited) {
                        g = 32767.0 - (nn + pf(D_COL_PLUS) + pf(COL_VAR_DE_STOP_MUL)
                            * r_log(max(pf(DE_STOP), ms_de_stop) * pf(STEP_WIDTH))) * pf(MCTS_M);
                    } else {
                        g = 32767.0 - nn * pf(MCTS_M);
                    }
                    si_grad = clip15(g);
                    after_coi = P_SURF_COL;
                    pc = select(P_SURF_COL, P_COI, P[COLOR_ON_IT] != 0u);
                } else if (MODE_KIND == 3u && R_MARCH) {
                    // the hit of a reflected ray (calc_ray)
                    c = ct1;
                    store_normal(n);
                    var g: R;
                    if (de_limited) {
                        g = 32767.0 - (nn + pf(D_COL_PLUS) + pf(COL_VAR_DE_STOP_MUL) * r_log(ms_de_stop * pf(STEP_WIDTH))) * pf(MCTS_M);
                    } else {
                        g = 32767.0 - nn * pf(MCTS_M);
                    }
                    si_grad = clip15(g);
                    do_color();
                    var ds = r_zz2;
                    if (P[R_FAR_FOG] != 0u) {
                        ds = sqrt(max((r_zz2 + last_zz) * (r_zz2 + last_zz) - last_zz + 1e-30, 0.0));
                    }
                    let mz = mzz;
                    mzz = ds;
                    calc_zpos_and_rough();
                    mzz = mz;
                    pc = P_R_AFTER_N;
                }
            }
            // ---- the post calculations: the surface of the pixel
            case P_PS: {
                if (MODE_KIND != 0u) {
                    // the depth of the G-buffer (the inverse of calc_zpos_and_rough)
                    zf = R(si_zpos >> 8u);
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
                        yp = R(max_it) - 0.99;
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
                if (MODE_KIND == 1u || (MODE_KIND == 3u && R_HS)) {
                    calc_sit = false;
                    nvec = rot_rev(vec3<R>(si_normal) / 32767.0);
                    if (reflect) {
                        // calc_hs: from the hit itself, the depth along the view
                        if (soft) {
                            si_shadow |= 0xFC00u;
                        } else {
                            for (var i = 0u; i < P[HS_N]; i++) {
                                si_shadow &= ~(0x400u << P[R_LIGHTS + i * 8u]);
                            }
                        }
                        update_de_stop();
                        ic = c;
                        max_lhs = R(pi(WIDTH) + pi(HEIGHT)) * 0.6 * (1.0 + 0.5 * min(mzz, pf(ZEND) * 0.4) * max(pf(FOV_Y), 0.0) / R(pi(HEIGHT))) * pf(HS_MAX_LEN_MUL);
                    } else {
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
                    }
                    zrs_mul = 80.0 / max(pf(HS_SOFT_RADIUS), 0.001);
                    li = 0u;
                    pc = P_HS_LIGHT;
                }
            }
            case P_HS_LIGHT: {
                if (MODE_KIND == 1u || (MODE_KIND == 3u && R_HS)) {
                    if (li >= P[HS_N]) {
                        c = ic;
                        pc = after_hs;
                    } else {
                        c = ic;
                        zz2 = mzz;
                        dt1 = max_lhs;
                        var too_far = false;
                        if (reflect) {
                            let lb = R_LIGHTS + li * 8u;
                            lidx = P[lb];
                            lvec = pv(lb + 1u);
                            if (P[lb + 4u] != 0u) {
                                // a positional light: towards its position, at most its reach
                                let v = lvec - c;
                                let d = dot(v, v);
                                too_far = d > pf(lb + 5u) * pf(HS_MAX_LEN_MUL);
                                let sw = pf(STEP_WIDTH);
                                if (d < dt1 * sw * dt1 * sw) {
                                    dt1 = sqrt(d) / sw;
                                }
                                lvec = v * (-sw / max(sqrt(d), 1e-30));
                                if (soft) {
                                    zrs_mul = select(R(40.0), R(70.0), (P[lb + 6u] & 6u) == 2u) / max(pf(HS_SOFT_RADIUS), 0.001);
                                }
                            }
                        } else {
                            lidx = P[HS_LIGHTS + li * 4u];
                            lvec = pv(HS_LIGHTS + li * 4u + 1u);
                        }
                        zz2mul = -dot(lvec, vfov_px) / max(sqrt(dot(lvec, lvec) * dot(vfov_px, vfov_px)), 1e-30);
                        if (too_far) {
                            if (soft) {
                                c = ic;
                                pc = after_hs;
                            } else {
                                si_shadow |= 0x400u << lidx;
                                li += 1u;
                            }
                        } else if (dot(nvec, lvec) > 0.0) {
                            // the surface faces away from the light
                            if (soft) {
                                si_shadow &= 0x3FFu;
                                c = ic;
                                pc = after_hs;
                            } else {
                                si_shadow |= 0x400u << lidx;
                                li += 1u;
                            }
                        } else {
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
                if (MODE_KIND == 1u || (MODE_KIND == 3u && R_HS)) {
                    dtmp = r;
                    if (reflect) {
                        rsf = 1.0;
                    }
                    pc = P_HS_STEP;
                }
            }
            case P_HS_STEP: {
                if (MODE_KIND == 1u || (MODE_KIND == 3u && R_HS)) {
                    last_de = dtmp;
                    if (reflect) {
                        st = min(dtmp * pf(Z_STEP_DIV) * rsf, max(ms_de_stop, 0.4) * pf(MH04ZSD));
                    } else {
                        st = min(max(0.11, (dtmp - pf(MS_DE_SUB) * ms_de_stop) * pf(Z_STEP_DIV) * rsf), max(ms_de_stop, 0.4) * pf(MH04ZSD));
                    }
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
                if (MODE_KIND == 1u || (MODE_KIND == 3u && R_HS)) {
                    dtmp = r;
                    if (soft) {
                        let rr = (max_lhs - dt1) / max_lhs;
                        let r8 = (rr * rr) * (rr * rr);
                        zr_soft = min(zr_soft, (dtmp - ms_de_stop) * zrs_mul / (max_lhs - dt1 + 0.11) + r8 * r8);
                    }
                    if (it_result >= max_its_result || dtmp <= ms_de_stop || (reflect && dtmp < ms_de_stop)) {
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
                if (MODE_KIND == 1u || (MODE_KIND == 3u && R_HS)) {
                    if (soft) {
                        si_shadow = (si_shadow & 0x3FFu) | (u32(round(clamp(zr_soft, 0.0, 1.0) * 63.4)) << 10u);
                        c = ic;
                        pc = after_hs;
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
                if (MODE_KIND == 2u || (MODE_KIND == 3u && R_AO)) {
                    calc_sit = false;
                    update_de_stop();
                    let msl = min(ms_de_stop, 1e6);
                    ic = c;
                    step_ao = msl / pf(DE_STOP);
                    let s_max_d = pf(AO_MAX_LEN) * 0.5 * sqrt(R(pi(HEIGHT)) * R(pi(HEIGHT)) + R(pi(WIDTH)) * R(pi(WIDTH)));
                    max_dist = s_max_d * sqrt(step_ao);
                    let q = min(u32(pi(AO_QUALITY)), 3u);
                    let abr = 0.5 * PI / (R(q) + 0.9);
                    if (q == 0u) {
                        d_step_mul = 1.8;
                        d_min_a_dif = -1.0;
                        corr_w = 0.3;
                    } else {
                        d_step_mul = 1.0 + r_sin(abr);
                        d_min_a_dif = r_cos(abr * 1.2);
                        corr_w = select(R(0.1666), R(0.2), q == 1u);
                    }
                    ms2 = select(pf(DE_STOP), msl, pf(DE_STOP_FACTOR) != 0.0) / (d_step_mul * d_step_mul);
                    // the ray directions, dithered per pixel
                    let dither = pi(AO_DITHER);
                    var dt1d = R(0.0);
                    var dt2d = R(0.0);
                    if (dither > 0) {
                        dt1d = R(y % (dither + 1)) * 0.5 / R(dither);
                        dt2d = R(x % (dither + 1)) * 0.5 / R(dither);
                    }
                    ray_count = 0u;
                    if (q == 0u) {
                        for (var i = 0u; i < 3u; i++) {
                            if (dither > 0) {
                                ao_dirs[i] = ray_dir((dt1d + 0.5) * radians(50.0), (R(i) + dt2d) * 2.0 * PI / 3.0);
                            } else {
                                ao_dirs[i] = ray_dir(0.5 * radians(60.0), R(i) * 2.0 * PI / 3.0);
                            }
                        }
                        ray_count = 3u;
                    } else {
                        if (!(dither > 0 && dt1d > 0.1)) {
                            ao_dirs[0] = ray_dir(0.0, 0.0);
                            ray_count = 1u;
                        }
                        for (var rr = 1u; rr <= q; rr++) {
                            // CalcAmbShadowDEfor1pos (reflections) takes r_sin(0.25) for every row
                            let n = u32(round(r_sin(select(R(rr) * abr, 0.25, reflect)) * 2.0 * PI / abr));
                            for (var i = 0u; i < n; i++) {
                                if (ray_count < 33u) {
                                    ao_dirs[ray_count] = ray_dir(abr * (R(rr) + dt1d - select(R(0.0), R(0.25), dither > 0)), (R(i) + dt2d) * 2.0 * PI / R(n));
                                    ray_count += 1u;
                                }
                            }
                        }
                    }
                    de_mul = sqrt(R(ray_count) * 0.5);
                    abr_c = 1.2 / r_asin(clamp(1.0 / de_mul, -1.0, 1.0));
                    md_d10 = 0.1 / (max_dist * de_mul);
                    rot_w = normal_matrix(normalize(vec3<R>(si_normal)));
                    // normalise_matrix_to(step_width, vgrads)
                    rv0 = vg(0u) * (pf(STEP_WIDTH) / length(vg(0u)));
                    rv1 = vg(1u) * (pf(STEP_WIDTH) / length(vg(1u)));
                    rv2 = vg(2u) * (pf(STEP_WIDTH) / length(vg(2u)));
                    k = 0u;
                    pc = P_AO_RAY;
                }
            }
            case P_AO_RAY: {
                if (MODE_KIND == 2u || (MODE_KIND == 3u && R_AO)) {
                    if (k >= ray_count) {
                        pc = P_AO_CORR;
                    } else {
                        let d = ao_dirs[k];
                        let vv = vec3<R>(dot(rot_w[0], d), dot(rot_w[1], d), dot(rot_w[2], d));
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
                if (MODE_KIND == 2u || (MODE_KIND == 3u && R_AO)) {
                    if (b_first) {
                        b_first = false;
                        dt1 *= get_rand() * 1.5 + 0.5;
                    } else if (dt1 > max_dist) {
                        dt1 = max_dist;
                        b_end = true;
                    }
                    c = ic + sv * dt1;
                    if (P[CUT_OPT] != 0u && at_cut_plane(dt1 * pf(STEP_WIDTH) * 0.5)) {
                        ao_min[k] = max(s_tmp, 0.0) * de_mul;
                        k += 1u;
                        pc = P_AO_RAY;
                    } else {
                        need = true;
                        want_de = true;
                        pc = P_AO_R;
                    }
                }
            }
            case P_AO_R: {
                if (MODE_KIND == 2u || (MODE_KIND == 3u && R_AO)) {
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
                if (MODE_KIND == 2u || (MODE_KIND == 3u && R_AO)) {
                    // open neighbour rays lighten a closed one
                    var amount = R(0.0);
                    for (var i = 0u; i < ray_count; i++) {
                        var s_add = R(0.0);
                        if (ao_min[i] < 1.0) {
                            let max_add = 1.0 - ao_min[i];
                            for (var i2 = 0u; i2 < ray_count; i2++) {
                                if (i2 == i) {
                                    continue;
                                }
                                let dt = dot(ao_dirs[i], ao_dirs[i2]);
                                if (dt > d_min_a_dif) {
                                    let overlap = ao_min[i2] - r_acos(clamp(dt, -1.0, 1.0)) * abr_c + 1.0;
                                    if (overlap > 0.0) {
                                        s_add += min(max_add, overlap) * corr_w;
                                    }
                                }
                            }
                        }
                        amount += min(s_add + ao_min[i], 1.0);
                    }
                    si_amb = u32(max(round(16383.0 * (1.0 - amount / R(ray_count))), 0.0));
                    c = ic;
                    pc = after_ao;
                }
            }
            case P_DONE: {
                done = true;
            }
            // ---- the reflection pass: calc_ray of reflect.rs
            case P_R_START: {
                if (MODE_KIND == 3u && R_MARCH) {
                    r_flags = 0u;
                    r_step_count = 0.0;
                    r_zz2 = 0.0;
                    calc_sit = false;
                    if (P[R_SURFACE] != 0u) {
                        // the pixel's surface: from the G-buffer depth, refined
                        zf = r_task.z;
                        let a = (8388351.5 - zf) / pf(ZC_MUL);
                        mzz = a * (a + 2.0) / pf(ZCORR);
                        setc();
                        update_de_stop();
                        need = true;
                        want_de = true;
                        pc = P_R_SURF_R;
                    } else {
                        base = r_task_origin;
                        vfov = r_task_dir;
                        zz_base = r_task.z;
                        zz_mul = dot(vfov_px, vfov) / sqrt(dot(vfov_px, vfov_px) * dot(vfov, vfov) + 1e-30);
                        last_zz = abs(zz_base);
                        mzz = min(pf(DE_STOP) * 0.25, 1.0);
                        setc();
                        update_de_stop();
                        need = true;
                        want_de = true;
                        pc = P_R_M0;
                    }
                }
            }
            case P_R_SURF_R: {
                if (MODE_KIND == 3u && R_MARCH) {
                    dtmp = r;
                    if (dtmp < ms_de_stop * 0.5) {
                        r_flags = 4u; // keep the painted colour
                        done = true;
                    } else {
                        de_limited = it_result < max_its_result || dtmp <= ms_de_stop;
                        itmp = pi(DE_ADD_STEPS);
                        after_bs = P_R_SURF_END;
                        if (de_limited) {
                            let a1 = (8388351.5 - (zf - 1.0)) / pf(ZC_MUL);
                            dt1 = (mzz - a1 * (a1 + 2.0) / pf(ZCORR)) * -0.5;
                            bs_thr = 0.001;
                            pc = P_BS;
                        } else {
                            yp = R(max_it) - 0.99;
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
            }
            case P_R_SURF_END: {
                if (MODE_KIND == 3u && R_MARCH) {
                    r_flags = 1u;
                    r_zz2 = mzz;
                    pc = P_R_END;
                }
            }
            case P_R_M0: {
                if (MODE_KIND == 3u && R_MARCH) {
                    dtmp = r;
                    rlast_sw = dtmp * pf(Z_STEP_DIV);
                    de_limited = true;
                    rsf = 1.0;
                    if (dtmp > 1e-10) {
                        pc = P_R_LOOP;
                    } else {
                        pc = P_R_HIT;
                    }
                }
            }
            case P_R_LOOP: {
                if (MODE_KIND == 3u && R_MARCH) {
                    last_de = dtmp;
                    var dstep = max((dtmp - pf(MS_DE_SUB) * ms_de_stop) * pf(Z_STEP_DIV) * rsf, 0.11);
                    let s1 = max(ms_de_stop, 0.4) * pf(MH04ZSD);
                    let count = dfog_on_it == 0 || it_result == dfog_on_it;
                    if (s1 < dstep) {
                        if (count) {
                            r_step_count += s1 / dstep;
                        }
                        dstep = s1;
                    } else if (count) {
                        r_step_count += 1.0;
                    }
                    rlast_sw = dstep;
                    mzz += dstep;
                    if (mzz > pf(ZEND)) {
                        // open air
                        r_flags = 0u;
                        r_zz2 = mzz;
                        pc = P_R_END;
                    } else {
                        setc();
                        update_de_stop();
                        need = true;
                        want_de = true;
                        pc = P_R_STEP;
                    }
                }
            }
            case P_R_STEP: {
                if (MODE_KIND == 3u && R_MARCH) {
                    dtmp = r;
                    if (it_result >= max_its_result) {
                        de_limited = false;
                        pc = P_R_HIT;
                    } else if (dtmp < ms_de_stop && it_result >= pi(MIN_IT)) {
                        pc = P_R_HIT;
                    } else {
                        if (dtmp > last_de + rlast_sw) {
                            dtmp = last_de + rlast_sw;
                        }
                        rsf = 1.0;
                        if (last_de > dtmp + 1e-10) {
                            let t = rlast_sw / (last_de - dtmp);
                            if (t < 1.0) {
                                rsf = max(0.5, t);
                            }
                        }
                        pc = P_R_LOOP;
                    }
                }
            }
            case P_R_HIT: {
                if (MODE_KIND == 3u && R_MARCH) {
                    after_bs = P_R_HIT2;
                    if (pi(DE_ADD_STEPS) != 0) {
                        itmp = pi(DE_ADD_STEPS);
                        if (de_limited) {
                            // dtmp is the DE at the surface (the march's last)
                            dt1 = rlast_sw * -0.5;
                            bs_thr = 0.001;
                            bs_it_check = false;
                            pc = P_BS;
                        } else {
                            yp = R(max_it) - 0.99;
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
                        pc = P_R_HIT2;
                    }
                }
            }
            case P_R_HIT2: {
                if (MODE_KIND == 3u && R_MARCH) {
                    r_flags = 1u | select(0u, 2u, de_limited);
                    r_zz2 = mzz;
                    // from here the depth is the one along the view (m.mzz = zz.abs())
                    mzz = abs(zz_base + mzz * zz_mul);
                    zz_base = 0.0;
                    zz_mul = 1.0;
                    si_shadow = u32(round(clamp(r_step_count, 0.0, 1023.0)));
                    after_hs = P_R_AFTER_HS;
                    after_ao = P_R_END;
                    pc = P_N_INIT;
                }
            }
            case P_R_POST: {
                if (MODE_KIND == 3u && !R_MARCH) {
                    if (OUT[r_rec + 22u] == 0u) {
                        // no hit (or the surface task): nothing to add
                        r_keep = true;
                        done = true;
                    } else {
                        // the bounce as P_R_START set it up, and the hit as
                        // P_R_HIT2 leaves it (mzz: the depth along the view)
                        base = r_task_origin;
                        vfov = r_task_dir;
                        last_zz = abs(r_task.z);
                        zz_base = 0.0;
                        zz_mul = 1.0;
                        after_hs = P_R_AFTER_HS;
                        after_ao = P_R_END;
                        let o = r_rec;
                        let a = OUT[o + 8u];
                        let b = OUT[o + 9u];
                        si_normal = vec3<i32>(i32(a << 16u) >> 16u, i32(a) >> 16u, i32(b << 16u) >> 16u);
                        si_shadow = b >> 16u;
                        si_zpos = OUT[o + 10u];
                        si_grad = OUT[o + 11u] & 0xFFFFu;
                        si_otrap = OUT[o + 11u] >> 16u;
                        si_amb = OUT[o + 12u];
                        c = vec3<R>(R(bitcast<f32>(OUT[o + 13u])), R(bitcast<f32>(OUT[o + 14u])), R(bitcast<f32>(OUT[o + 15u])));
                        r_zz2 = R(bitcast<f32>(OUT[o + 17u]));
                        r_flags = OUT[o + 18u];
                        r_step_count = R(bitcast<f32>(OUT[o + 19u]));
                        mzz = R(bitcast<f32>(OUT[o + 20u]));
                        seed = bitcast<i32>(OUT[o + 21u]);
                        pc = P_R_AFTER_N;
                    }
                }
            }
            case P_R_AFTER_N: {
                if (MODE_KIND == 3u && R_MARCH && P[R_SPLIT] != 0u) {
                    // the first pipeline: the hit for the second
                    OUT[r_rec + 20u] = bitcast<u32>(f32(mzz));
                    OUT[r_rec + 21u] = bitcast<u32>(seed);
                    OUT[r_rec + 22u] = 1u;
                }
                if (MODE_KIND == 3u) {
                    if (R_HS && P[HS_N] > 0u) {
                        pc = P_HS_INIT;
                    } else {
                        pc = P_R_AFTER_HS;
                    }
                }
            }
            case P_R_AFTER_HS: {
                if (MODE_KIND == 3u) {
                    if (R_AO && pi(AO_QUALITY) >= 0) {
                        pc = P_AO_INIT;
                    } else {
                        si_amb = 5000u;
                        pc = P_R_END;
                    }
                }
            }
            case P_R_END: {
                if (MODE_KIND == 3u) {
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
        if (inside) {
            step_count *= 200.0 * pf(DE_STOP) / R(pi(WIDTH));
        }
        si_shadow = u32(round(clamp(step_count, 0.0, 1023.0)));
    }
}


// the task of the reflection pass
var<private> r_task_origin: vec3<R>;
var<private> r_task_dir: vec3<R>;
var<private> r_task: vec4<R>; // z: zz (or the surface depth)

var<private> state_slot: u32;
// the reflection pass: the task's record in OUT, and whether the task
// leaves it alone (the second pipeline on a task without a hit)
var<private> r_rec: u32;
var<private> r_keep: bool;
var<private> cur_item: u32;
var<private> resuming: bool;
var<private> suspended: bool;

fn reflect_task(idx: u32) {
    let o = idx * R_WORDS;
    r_rec = o;
    r_keep = false;
    r_task_origin = vec3<R>(R(bitcast<f32>(OUT[o])), R(bitcast<f32>(OUT[o + 1u])), R(bitcast<f32>(OUT[o + 2u])));
    r_task_dir = vec3<R>(R(bitcast<f32>(OUT[o + 3u])), R(bitcast<f32>(OUT[o + 4u])), R(bitcast<f32>(OUT[o + 5u])));
    r_task = vec4<R>(0.0, 0.0, R(bitcast<f32>(OUT[o + 6u])), 0.0);
    let pix = OUT[o + 7u];
    let x = pi(RECT_X0) + i32(pix % u32(pi(RECT_W)));
    let y = pi(RECT_Y0) + i32(pix / u32(pi(RECT_W)));
    si_normal = vec3<i32>(0);
    si_zpos = 32768u << 16u;
    si_shadow = 0u;
    si_grad = 0u;
    si_otrap = 0u;
    si_amb = 5000u;
    s_roughness = 0.0;
    pixel(x, y, M_REFLECT);
    if (suspended || r_keep) {
        return;
    }
    OUT[o + 8u] = (u32(si_normal.x) & 0xFFFFu) | (u32(si_normal.y) << 16u);
    OUT[o + 9u] = (u32(si_normal.z) & 0xFFFFu) | (si_shadow << 16u);
    OUT[o + 10u] = si_zpos;
    OUT[o + 11u] = si_grad | (si_otrap << 16u);
    OUT[o + 12u] = si_amb;
    OUT[o + 13u] = bitcast<u32>(f32(c.x));
    OUT[o + 14u] = bitcast<u32>(f32(c.y));
    OUT[o + 15u] = bitcast<u32>(f32(c.z));
    OUT[o + 16u] = bitcast<u32>(f32(zz_base + mzz * zz_mul));
    OUT[o + 17u] = bitcast<u32>(f32(r_zz2));
    OUT[o + 18u] = r_flags;
    OUT[o + 19u] = bitcast<u32>(f32(r_step_count));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(workgroup_id) wid: vec3<u32>, @builtin(local_invocation_index) li: u32) {
    // the slot: a suspended pixel or the next one of the job (each slot
    // takes its own: reflected rays of neighbouring pixels differ widely in
    // length, and a slot that waited for its group's slowest wasted the card)
    state_slot = wid.x * 64u + li;
    let so = state_slot;
    let st = STATE[so];
    resuming = st != 0u;
    suspended = false;
    if (resuming) {
        cur_item = st - 1u;
    } else {
        // (idle slots do not count on: the counter would wrap in long jobs)
        if (atomicLoad(&QUEUE[1]) >= P[ITEM_END]) {
            return;
        }
        cur_item = atomicAdd(&QUEUE[1], 1u);
        if (cur_item >= P[ITEM_END]) {
            return;
        }
    }
    let w = pi(RECT_W);
    // the pixel (the reflection pass: the seed of the task's place in
    // rows of R_W)
    let rw = select(u32(w), R_W, P[MODE] == M_REFLECT);
    let cx = i32(cur_item % rw);
    let ly = i32(cur_item / rw);
    let x = pi(RECT_X0) + cx;
    let y = pi(RECT_Y0) + pi(ROW0) + ly;
    // MB3D seeds per row; the CPU advances the seed along the row, here the
    // seed is advanced by the column (the same random sequence, but not
    // exactly the same numbers where pixels have no first step)
    // (a resumed pixel loads its seed and record from the state)
    if (!resuming) {
        seed = bitcast<i32>(0x24563487u + u32(y + 1) * 0x324594A1u);
        for (var k = 0; k < cx; k++) {
            seed = seed * 214013 + 2531011;
        }
        init_record();
    }
    _ = cst[0];
    _ = rtpl[0];
    _ = MAPS[0];
    j4 = p_j4();
    vary_scale = 1.0;
    deriv = vec3<R>(1.0, 0.0, 0.0);
    dfree = vec2<R>(0.0, 0.0);
    let mode = P[MODE];
    if (mode == M_REFLECT) {
        reflect_task(cur_item);
        if (!suspended) {
            STATE[so] = 0u;
        }
        return;
    }
    let o = cur_item * 5u;
    // one call of pixel() only: the driver inlines it at every call site
    var run = true;
    de_overflow = false;
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
    if (suspended) {
        return;
    }
    STATE[so] = 0u;
    if (mode == M_MARCH || mode == M_PROBE) {
        OUT[o] = (u32(si_normal.x) & 0xFFFFu) | (u32(si_normal.y) << 16u);
        OUT[o + 2u] = si_zpos;
        OUT[o + 3u] = si_grad | (si_otrap << 16u);
    }
    OUT[o + 1u] = (u32(si_normal.z) & 0xFFFFu) | (si_shadow << 16u);
    OUT[o + 4u] = (si_amb & 0xFFFFu) | select(0u, 0x10000u, de_overflow) | (OUT[o + 4u] & 0x10000u & select(0u, 0xFFFFFFFFu, mode != M_MARCH && mode != M_PROBE));
}
