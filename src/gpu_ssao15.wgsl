// The older "15 bit" screen space ambient occlusion on the graphics card:
// a port of ssao15.rs (MB3D's AmbShadowCalcThreadN).  The pyramid of
// smoothed depth levels is built on the CPU (`BuildATlevels`); here every
// object pixel samples rings of growing radius on the matching level,
// keeps the steepest slope per 1/32 of the circle and sums the arc
// tangents.

@group(0) @binding(0) var<storage, read> P: array<u32>;
// the levels, one after the other, w * h words each
@group(0) @binding(1) var<storage, read> lev: array<u32>;
// the scaled depth of every pixel (32768: background)
@group(0) @binding(2) var<storage, read> zps: array<i32>;
@group(0) @binding(3) var<storage, read_write> outp: array<u32>;

const W: u32 = 0u;
const H: u32 = 1u;
const COUNT: u32 = 2u;
const SZRT: u32 = 3u;
const ST2: u32 = 4u;
const SMUL: u32 = 5u;
const T0: u32 = 6u;
// 1: the picture wraps around horizontally (a panorama, SsaoParams::wrap_x)
const WRAP: u32 = 7u;
const BIG: i32 = 0x7FFFFFFF;

fn pf(i: u32) -> f32 { return bitcast<f32>(P[i]); }

// fastIntArcTan2: the angle in 1/32 circles
fn arctan2_32(y: i32, x: i32) -> u32 {
    var r: i32;
    if (x == 0) {
        r = select(0, 16, y >= 0) + 8;
    } else if (y == 0) {
        r = select(0, 16, x >= 0);
    } else if (y < 0) {
        if (x < 0) {
            if (x >= y) {
                r = 7 - (x * 4) / y;
            } else {
                r = (y * 4) / x;
            }
        } else if (-y < x) {
            r = 15 + (y * 4) / x;
        } else {
            r = 8 - (x * 4) / y;
        }
    } else if (x >= 0) {
        if (x > y) {
            r = 16 + (y * 4) / x;
        } else {
            r = 23 - (x * 4) / y;
        }
    } else if (y < -x) {
        r = 31 + (y * 4) / x;
    } else {
        r = 24 - (x * 4) / y;
    }
    return u32(r & 31);
}

// the first sample of an axis and the one the border forces (axis_samples)
fn axis_start(pos: i32, size: i32, max_rad: i32, step: i32, a2: ptr<function, i32>) -> i32 {
    var a = -max_rad;
    *a2 = BIG;
    if (a + pos < 0) {
        var b = a;
        while (b + pos < 0) {
            b += step;
        }
        if (b + pos >= size) {
            b = size - pos - 1;
        }
        a = -pos;
        if (a != b) {
            *a2 = b;
        }
    }
    return a;
}

var<private> ang: array<f32, 40>;

@compute @workgroup_size(8, 8)
fn sample15(@builtin(global_invocation_id) gid: vec3<u32>) {
    let w = i32(P[W]);
    let h = i32(P[H]);
    let x = i32(gid.x);
    let y = i32(gid.y);
    if (x >= w || y >= h) {
        return;
    }
    let i = u32(y * w + x);
    let zp = zps[i];
    if (zp >= 32768) {
        return;
    }
    let count = P[COUNT];
    let n = f32(count);
    let szrt = pf(SZRT);
    let t0 = P[T0] != 0u;
    for (var k = 0u; k < 40u; k++) {
        ang[k] = -1e10;
    }
    var max_rad = 0;
    var min_rad = 0;
    for (var atl = 1u; atl <= count; atl++) {
        let step = 1 << (atl - 1u);
        let rm = sqrt(f32(step)) * 5.0;
        max_rad += 4 * step;
        let maxs = max_rad * max_rad;
        let mins = min_rad * min_rad;
        let multi = round(rm / f32(min_rad + 1)) > 0.0;
        let zrt = szrt * sqrt(sqrt(n / f32(atl)));
        let lo = (atl - 1u) * u32(w * h);
        // the y samples
        var ya2 = 0;
        var vy = axis_start(y, h, max_rad, step, &ya2);
        let ey = select(max_rad, h - y - 1, y + max_rad >= h);
        loop {
            if (vy > ya2) {
                vy = ya2;
                ya2 = BIG;
            }
            let y2 = vy;
            let rowi = lo + u32((y + y2) * w);
            // the x samples
            var xa2 = 0;
            var vx: i32;
            var ex: i32;
            let wrap = P[WRAP] != 0u;
            if (wrap) {
                // the whole grid: x continues on the other side
                vx = axis_start(max_rad, 2 * max_rad + 1, max_rad, step, &xa2);
                ex = max_rad;
            } else {
                vx = axis_start(x, w, max_rad, step, &xa2);
                ex = select(max_rad, w - x - 1, x + max_rad >= w);
            }
            loop {
                if (vx > xa2) {
                    vx = xa2;
                    xa2 = BIG;
                }
                let x2 = vx;
                let rads = y2 * y2 + x2 * x2;
                if (rads > mins && rads <= maxs) {
                    let r1d = 1.0 / sqrt(f32(rads));
                    let xx = select(x + x2, ((x + x2) % w + w) % w, wrap);
                    var st = f32(i32(lev[rowi + u32(xx)]) - zp) * r1d;
                    var skip = false;
                    if (t0) {
                        if (st >= zrt) {
                            skip = true;
                        } else {
                            let s3 = st / zrt;
                            st *= 1.0 - s3 * s3 * s3;
                        }
                    } else if (st > zrt) {
                        st = zrt;
                    }
                    if (!skip) {
                        let c = arctan2_32(y2, x2);
                        if (multi) {
                            let ic = u32(round(rm * r1d));
                            let start = u32(i32(c) - i32(ic >> 1u)) & 31u;
                            let end = min(start + ic, 39u);
                            for (var k = start; k <= end; k++) {
                                if (ang[k] < st) {
                                    ang[k] = st;
                                }
                            }
                        } else if (ang[c] < st) {
                            ang[c] = st;
                        }
                    }
                }
                let vt = vx;
                vx += step;
                if (vx > ex && vt < ex) {
                    vx = ex;
                }
                if (vx > ex) {
                    break;
                }
            }
            let vt = vy;
            vy += step;
            if (vy > ey && vt < ey) {
                vy = ey;
            }
            if (vy > ey) {
                break;
            }
        }
        min_rad = max_rad;
    }
    for (var k = 0u; k < 4u; k++) {
        if (ang[k + 32u] > ang[k]) {
            ang[k] = ang[k + 32u];
        }
    }
    var s = 0.0;
    let st2 = pf(ST2);
    for (var k = 0u; k < 32u; k++) {
        if (ang[k] > -1e9) {
            s += atan(ang[k] * st2);
        }
    }
    outp[i] = u32(clamp(round(s * pf(SMUL)), 0.0, 16383.0));
}
