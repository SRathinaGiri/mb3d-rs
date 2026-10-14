// Screen space ambient occlusion on the graphics card: a port of ssao.rs
// (MB3D's AmbHiQ, the "24 bit" ambient shadow).  The levels of the
// smoothed depth buffer are made by `blur_h` / `blur_v` (NextATlevelHiQ),
// `sample` searches the maximum elevation angle in 32 directions on a
// level and keeps it in `ang`; on the last level the angles are summed
// into `acc`.

@group(0) @binding(0) var<storage, read> P: array<u32>;
// the depth of every pixel (FirstATlevelHiQ), bit 0 set for object pixels
@group(0) @binding(1) var<storage, read> z0: array<u32>;
// the depth buffer of the level (whole image); blur_h reads pa and writes
// pb, blur_v reads pb and writes pa
@group(0) @binding(2) var<storage, read_write> pa: array<u32>;
@group(0) @binding(3) var<storage, read_write> pb: array<u32>;
// the angles of the pixels of the row block: 32 x i16 per pixel as 16 words
@group(0) @binding(4) var<storage, read_write> ang: array<u32>;
// the ambient shadow sum of every pixel (whole image)
@group(0) @binding(5) var<storage, read_write> acc: array<u32>;

const W: u32 = 0u;
const H: u32 = 1u;
const ROW0: u32 = 2u;
const ROWS: u32 = 3u;
const SUM_UP: u32 = 4u;
const SIT: u32 = 5u;
const SZRT: u32 = 6u;
const SMUL: u32 = 7u;
const IMIN: u32 = 8u;
const IAND: u32 = 9u;
const SSUB: u32 = 10u;
const ISTEP: u32 = 11u;
const SMIN_RAD: u32 = 12u;
const STEP_COUNT: u32 = 13u;
const RMA: u32 = 14u; // 5 values
const RND: u32 = 19u;
const T0: u32 = 20u;
const WLO: u32 = 21u;
const WHI: u32 = 22u;
const HLO: u32 = 23u;
const HHI: u32 = 24u;
const MW2: u32 = 25u;
const MH2: u32 = 26u;
const PASS: u32 = 27u;
const BLUR_STEP: u32 = 28u;

fn pf(i: u32) -> f32 { return bitcast<f32>(P[i]); }
fn pi(i: u32) -> i32 { return bitcast<i32>(P[i]); }

// SmoothH: (v[x] + (v[x - s] + v[x + s]) / 2) / 2 on a row
@compute @workgroup_size(8, 8)
fn blur_h(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w = P[W];
    let h = P[H];
    let x = gid.x;
    let y = gid.y;
    if (x >= w || y >= h) {
        return;
    }
    _ = z0[0]; _ = ang[0]; _ = acc[0]; // all pipelines share one bind group
    let s = P[BLUR_STEP];
    let a = select(x - s, 0u, x < s);
    let b = min(x + s, w - 1u);
    let o = y * w;
    pb[o + x] = (pa[o + x] + ((pa[o + a] + pa[o + b]) >> 1u)) >> 1u;
}

// SmoothV on a column
@compute @workgroup_size(8, 8)
fn blur_v(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w = P[W];
    let h = P[H];
    let x = gid.x;
    let y = gid.y;
    if (x >= w || y >= h) {
        return;
    }
    _ = z0[0]; _ = ang[0]; _ = acc[0];
    let s = P[BLUR_STEP];
    let a = select(y - s, 0u, y < s);
    let b = min(y + s, h - 1u);
    pa[y * w + x] = (pb[y * w + x] + ((pb[a * w + x] + pb[b * w + x]) >> 1u)) >> 1u;
}

fn ang_get(o: u32, d: u32) -> i32 {
    let v = ang[o + d / 2u];
    if (d % 2u == 0u) {
        return i32(v << 16u) >> 16u;
    }
    return i32(v) >> 16u;
}

fn ang_set(o: u32, d: u32, a: i32) {
    let i = o + d / 2u;
    let v = ang[i];
    if (d % 2u == 0u) {
        ang[i] = (v & 0xFFFF0000u) | (u32(a) & 0xFFFFu);
    } else {
        ang[i] = (v & 0xFFFFu) | (u32(a) << 16u);
    }
}

// n steps of seed = seed * 214013 + 2531011 (mod 2^32) at once: the step
// is the affine map (a, c); its powers are made by squaring, (a, c) twice
// is (a * a, c * (a + 1)), so n steps cost about log2(n) rounds instead of n
fn lcg_skip(seed: i32, n: u32) -> i32 {
    var a = 214013u;
    var c = 2531011u;
    var ma = 1u;
    var mc = 0u;
    var k = n;
    while (k != 0u) {
        if ((k & 1u) != 0u) {
            ma = ma * a;
            mc = mc * a + c;
        }
        c = c * (a + 1u);
        a = a * a;
        k = k >> 1u;
    }
    return bitcast<i32>(ma * bitcast<u32>(seed) + mc);
}

// TAmbHiQCalc / TAmbHiQCalcT0 / TAmbHiQCalcR for one pixel on one level
@compute @workgroup_size(8, 8)
fn sample(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w = P[W];
    let h = P[H];
    let lx = gid.x;
    let ly = gid.y;
    if (lx >= w || ly >= P[ROWS]) {
        return;
    }
    let x = lx;
    let y = P[ROW0] + ly;
    _ = pb[0];
    let i = y * w + x;
    let z = z0[i];
    if ((z & 1u) == 0u) {
        return; // background
    }
    let zp0 = f32(z & 0xFFFFFFFEu);
    let o = (ly * w + lx) * 16u;
    // level 1 (the sample pass that starts a block) clears the angles
    if (P[BLUR_STEP] == 0u) {
        for (var k = 0u; k < 16u; k++) {
            ang[o + k] = 0x80008000u;
        }
    }
    let rnd = P[RND] != 0u;
    let t0 = P[T0] != 0u;
    let sit = pf(SIT);
    let szrt = pf(SZRT);
    let ssub = pf(SSUB);
    let istep = f32(P[ISTEP]);
    let iand = i32(P[IAND]);
    let smin_rad = pf(SMIN_RAD);
    let steps = P[STEP_COUNT];
    // the random sequence of the row, advanced to this pixel (the CPU
    // draws along the row; the numbers differ, the statistics do not)
    var seed = i32(0x24563487u + (y + 1u) * 0x324594A1u + P[PASS] * 0x5851F42Du);
    if (rnd) {
        seed = lcg_skip(seed, x * 32u * steps);
    }
    let wlo = pi(WLO);
    let whi = pi(WHI);
    let hlo = pi(HLO);
    let hhi = pi(HHI);
    for (var d = 0u; d < 32u; d++) {
        let an = f32(d) * 3.14159265358979 / 16.0;
        let dx = sin(an);
        let dy = cos(an);
        var sx = f32(x) + dx * smin_rad;
        var sy = f32(y) + dy * smin_rad;
        var ox = dx * smin_rad - ssub;
        var oy = dy * smin_rad - ssub;
        var am = ang_get(o, d);
        for (var k = 0u; k < steps; k++) {
            var r = pf(RMA + k);
            var x2 = i32(round(sx));
            var y2 = i32(round(sy));
            if (rnd) {
                seed = seed * 214013 + 2531011;
                let jx = i32(round(ox)) + (i32(u32(seed) >> 16u) & iand);
                let jy = i32(round(oy)) + (i32(u32(seed) >> 10u) & iand);
                ox += dx * istep;
                oy += dy * istep;
                let dd = f32(jx * jx + jy * jy);
                if (dd == 0.0) {
                    sx += dx * istep;
                    sy += dy * istep;
                    continue;
                }
                r = 1.0 / sqrt(dd);
                x2 = i32(x) + jx;
                y2 = i32(y) + jy;
            }
            if (x2 < 0) {
                x2 = -x2;
                if (x2 >= wlo) {
                    break;
                }
            } else if (x2 >= i32(w)) {
                x2 = pi(MW2) - x2;
                if (x2 < whi) {
                    break;
                }
            }
            if (y2 < 0) {
                y2 = -y2;
                if (y2 >= hlo) {
                    break;
                }
            } else if (y2 >= i32(h)) {
                y2 = pi(MH2) - y2;
                if (y2 < hhi) {
                    break;
                }
            }
            let st = (f32(pa[u32(y2) * w + u32(x2)]) - zp0) * r;
            var it: f32;
            if (t0) {
                it = round(st * sit / (st * st + szrt));
            } else {
                it = round(st * sit / (szrt + abs(st)));
            }
            if (!(it == it)) {
                it = -32768.0;
            }
            if (f32(am) < it) {
                am = i32(clamp(it, -32768.0, 32767.0));
            }
            sx += dx * istep;
            sy += dy * istep;
        }
        ang_set(o, d, am);
    }
    if (P[SUM_UP] != 0u) {
        var s = 0.0;
        for (var d = 0u; d < 32u; d++) {
            let v = ang_get(o, d);
            if (v > -32768) {
                s += atan(f32(v) * 0.0002441406);
            }
        }
        let v = clamp(i32(round(s * pf(SMUL))), 0, pi(IMIN));
        acc[i] += u32(v);
    }
}
