// Depth of field on the graphics card (dof.rs: `dof_sorted`, `dof_forward`).
//
// The CPU spreads every pixel over a disk, in a fixed order; here every
// pixel gathers the disks that reach it, in the same order.  The image is
// cut into bins of 64x64 pixels; for each bin the CPU lists the source
// pixels whose disk can reach it, in the order the CPU spreads them.  A
// workgroup is one 16x16 tile of a bin: it loads the bin's list in chunks
// of 256 into workgroup memory, and each thread runs through the chunk for
// its own pixel, so the blending happens in exactly the CPU's order.
//
// Source records (8 words, from dof.rs):
//   sorted:  c0 c1 c2 (f32), s_ms (f32), sw (f32), ims (i32), r (i32),
//            flags (bit 0: descale, bit 1: background)
//   forward: c0 c1 c2, s_ms (= (|rr| + 1)^2), sw, rr (f32), r, 0
// Output per target pixel: the colour sums and the transparency sum.

const BIN: u32 = 64u;
const CHUNK: u32 = 256u;

// P: width, y0 (first target row), rows (target rows), src_y0 (row of
// record 0), bins_x, bin0 (first bin of this dispatch), forward, sm (f32
// bits)
@group(0) @binding(0) var<storage, read> P: array<u32>;
@group(0) @binding(1) var<storage, read> records: array<u32>;
@group(0) @binding(2) var<storage, read> offsets: array<u32>;
@group(0) @binding(3) var<storage, read> entries: array<u32>;
// sqrt of 0, 1, 2, ... as the CPU rounds it (f32)
@group(0) @binding(4) var<storage, read> roots: array<f32>;
@group(0) @binding(5) var<storage, read_write> out: array<f32>;

var<workgroup> chunk: array<u32, 2560>; // CHUNK x 10 words
var<workgroup> range: array<u32, 2>;
// the chunk's sources that reach this tile: a prefix sum of the flags
// gives each its place, so the order stays the CPU's
var<workgroup> keep: array<u32, 256>;

@compute @workgroup_size(16, 16)
fn main(@builtin(workgroup_id) wid: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    let width = P[0];
    let y0 = P[1];
    let rows = P[2];
    let src_y0 = P[3];
    let bins_x = P[4];
    let forward = P[6] != 0u;
    let sm = bitcast<f32>(P[7]);

    let bin = P[5] + wid.x / 16u;
    let tile = wid.x % 16u;
    let tx = (bin % bins_x) * BIN + (tile % 4u) * 16u + lid.x;
    let trow = (bin / bins_x) * BIN + (tile / 4u) * 16u + lid.y; // within the strip
    let ty = y0 + trow;
    let inside = tx < width && trow < rows;
    let li = lid.y * 16u + lid.x;

    if (li == 0u) {
        range[0] = offsets[bin];
        range[1] = offsets[bin + 1u];
    }
    let start = workgroupUniformLoad(&range[0]);
    let end = workgroupUniformLoad(&range[1]);

    // forward: the target's own radius decides how near pixels cover it
    var own_rr = 0.0;
    if (forward && inside) {
        own_rr = bitcast<f32>(records[((ty - src_y0) * width + tx) * 8u + 5u]);
    }

    var c0 = 0.0;
    var c1 = 0.0;
    var c2 = 0.0;
    var pt = 0.0;
    // the tile's pixels
    let tx0 = i32((bin % bins_x) * BIN + (tile % 4u) * 16u);
    let ty0 = i32(y0 + (bin / bins_x) * BIN + (tile / 4u) * 16u);
    for (var base = start; base < end; base += CHUNK) {
        let k = base + li;
        var s = 0u;
        var sx = 0u;
        var sy = 0u;
        var hit = 0u;
        if (k < end) {
            s = entries[k];
            sx = s % width;
            sy = s / width + src_y0;
            let r = i32(records[s * 8u + 6u]);
            let x = i32(sx);
            let y = i32(sy);
            if (x + r >= tx0 && x - r <= tx0 + 15 && y + r >= ty0 && y - r <= ty0 + 15) {
                hit = 1u;
            }
        }
        keep[li] = hit;
        workgroupBarrier();
        for (var step = 1u; step < CHUNK; step = step * 2u) {
            var v = keep[li];
            if (li >= step) {
                v = v + keep[li - step];
            }
            workgroupBarrier();
            keep[li] = v;
            workgroupBarrier();
        }
        // keep[li] is now the number of kept sources up to and with li
        if (hit != 0u) {
            let at = (keep[li] - 1u) * 10u;
            for (var w = 0u; w < 8u; w++) {
                chunk[at + w] = records[s * 8u + w];
            }
            chunk[at + 8u] = sx;
            chunk[at + 9u] = sy;
        }
        let n = workgroupUniformLoad(&keep[CHUNK - 1u]);
        if (inside) {
            for (var q = 0u; q < n; q++) {
                let o = q * 10u;
                let r = i32(chunk[o + 6u]);
                let dx = i32(tx) - i32(chunk[o + 8u]);
                let dy = i32(ty) - i32(chunk[o + 9u]);
                if (abs(dx) > r || abs(dy) > r) {
                    continue;
                }
                let cr = bitcast<f32>(chunk[o]);
                let cg = bitcast<f32>(chunk[o + 1u]);
                let cb = bitcast<f32>(chunk[o + 2u]);
                let s_ms = bitcast<f32>(chunk[o + 3u]);
                let sw = bitcast<f32>(chunk[o + 4u]);
                if (forward) {
                    var w = s_ms - f32(dx * dx) - f32(dy * dy);
                    if (w > 0.0) {
                        w = min(w, 1.0);
                        let rr = bitcast<f32>(chunk[o + 5u]);
                        if (own_rr >= rr) {
                            w = w * sw;
                        } else {
                            w = w * (sw * abs(own_rr) * sm);
                        }
                        pt = pt + w;
                        c0 = c0 + cr * w;
                        c1 = c1 + cg * w;
                        c2 = c2 + cb * w;
                    }
                    continue;
                }
                let ii = dx * dx + dy * dy;
                if (ii > i32(chunk[o + 5u])) {
                    continue;
                }
                let flags = chunk[o + 7u];
                let descale = (flags & 1u) != 0u;
                var w = s_ms - roots[ii];
                var dw = 0.0;
                if (descale) {
                    let t = 0.8 * w / s_ms;
                    dw = 1.0 - t * t;
                }
                if (w > 1.0) {
                    w = sw;
                } else {
                    w = w * sw;
                }
                if (descale && pt > dw) {
                    let f = dw / pt;
                    c0 = c0 * f;
                    c1 = c1 * f;
                    c2 = c2 * f;
                    pt = pt * f;
                }
                if ((flags & 2u) == 0u && w + pt > 1.01) {
                    let st = (1.01 - w) / pt;
                    c0 = c0 * st + cr * w;
                    c1 = c1 * st + cg * w;
                    c2 = c2 * st + cb * w;
                    pt = 1.01;
                } else {
                    c0 = c0 + cr * w;
                    c1 = c1 + cg * w;
                    c2 = c2 + cb * w;
                    pt = pt + w;
                }
            }
        }
        workgroupBarrier();
    }
    if (inside) {
        let i = (trow * width + tx) * 4u;
        out[i] = c0;
        out[i + 1u] = c1;
        out[i + 2u] = c2;
        out[i + 3u] = pt;
    }
}
