//! Mesh export (BulbTracer2: ObjectScanner2.pas, BulbTracer2.pas,
//! VertexList.pas, MeshWriter.pas): the distance estimate is sampled on a
//! grid, each grid point gets the weight `1 - sharpness * DE`, and marching
//! cubes builds the surface where the weight is 0.  The triangles are
//! written as OBJ or PLY (as MB3D) or binary STL, optionally with the
//! palette colours of the surface.
//!
//! Like BulbTracer2, the grid covers a cube of 100 units around the scene
//! middle, `2.2 / (zoom * scale)` scene units wide, turned by its own three
//! angles (not the camera), and the mesh is centred and scaled to size 1.

use crate::calc::{CalcParams, Marcher};
use crate::math::{normalise_matrix_to, Mat3, Vec3};
use crate::mclut::{AMBIGUOUS_CASES, FACES_AMBIGUOUS, FACES_STANDARD};
use crate::scene::Scene;
use std::collections::{HashMap, HashSet};
use std::io::Write;

/// `ZSlices`: the grid space is 0..100 in each direction.
const GRID: f64 = 100.0;

/// The settings of a mesh export (`TBTracer2Header`, the parts used here).
#[derive(Clone, Debug)]
pub struct MeshParams {
    /// Position of the cube in scene units (`XOff`, `YOff`, `ZOff`)
    pub offset: [f64; 3],
    /// Size: the cube is 2.2 / (zoom * scale) wide (`Scale`, default 0.5)
    pub scale: f64,
    /// Rotation of the cube in degrees (`XAngle`, `YAngle`, `ZAngle`)
    pub angles: [f64; 3],
    /// `SurfaceDetail` ("surface sharpness"): the surface lies where the DE
    /// is 1 / sharpness steps
    pub sharpness: f64,
    /// Grid steps in each direction (`VResolution`)
    pub resolution: usize,
    /// Vertex colours from the palette (`WithColors`)
    pub colors: bool,
    /// Only this part of the cube (grid units 0..100, `TraceXMin` ..)
    pub bounds: [[f64; 2]; 3],
    /// Close the mesh at the bounds (`CloseMesh`)
    pub close: bool,
    /// Taubin smoothing passes afterwards (not used by MB3D's export)
    pub smooth: u32,
}

impl Default for MeshParams {
    fn default() -> Self {
        MeshParams {
            offset: [0.0; 3],
            scale: 0.5,
            angles: [0.0; 3],
            sharpness: 0.5,
            resolution: 128,
            colors: false,
            bounds: [[0.0, GRID]; 3],
            close: false,
            smooth: 0,
        }
    }
}

/// A triangle mesh.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<[u32; 3]>,
    /// per vertex: red, green, blue (0..1) and the palette position
    pub colors: Option<Vec<[f32; 4]>>,
}

/// One grid sample: weight and colour.
#[derive(Clone, Copy, Default)]
struct Sample {
    w: f32,
    col: [f32; 4],
}

/// The sampling of the grid (`TObjectScanner2.Init`, `CalculateDistance`).
pub struct Sampler {
    p: CalcParams,
    lv: Option<crate::lighting::LightVals>,
    /// rows: scene vector per grid unit in x, y, z
    m: Mat3,
    origin: Vec3,
}

impl Sampler {
    pub fn new(sc: &Scene, mp: &MeshParams) -> Result<Sampler, String> {
        // VHeader: CalcImageSize makes it ZSlices x ZSlices pixels
        let mut s = sc.clone();
        s.width = GRID as i32;
        s.height = GRID as i32;
        s.tiling = None;
        s.calc_rect = None;
        s.light_blend = None;
        // CalcDE returns at least DEstop / 4: inside points get the weight
        // 1 - sharpness * DEstop / 4.  With a high sharpness nothing would be
        // inside (MB3D then finds no surface); the DE stop is lowered so
        // that inside points keep the weight 0.5.
        if let Some(d) = de_stop_for(sc, mp) {
            s.de_stop = d;
        }
        let p = CalcParams::new(&s)?;
        let d = 2.2 / (sc.zoom * mp.scale.max(1e-12) * (GRID - 1.0));
        let r = crate::m3f::build_rot_matrix(mp.angles[0].to_radians(), mp.angles[1].to_radians(), mp.angles[2].to_radians());
        let m = normalise_matrix_to(d, &r);
        let mut origin = sc.mid;
        let f = [-GRID * 0.5 + mp.offset[0] / d, -GRID * 0.5 + mp.offset[1] / d, -GRID * 0.5 + mp.offset[2] / d];
        for (k, fk) in f.iter().enumerate() {
            for c in 0..3 {
                origin[c] += m[k][c] * fk;
            }
        }
        let lv = mp.colors.then(|| {
            let l = &s.lighting;
            crate::lighting::LightVals::new(l, s.z_step_div, s.dfog_on_it, s.z_end - s.z_start, s.step_width(), s.width, 1.0)
        });
        Ok(Sampler { p, lv, m, origin })
    }

    /// A marcher for [`Sampler::de`].
    pub fn marcher(&self) -> Marcher<'_> {
        Marcher::new(&self.p, 1)
    }

    /// The distance estimate at a grid point (in grid steps of the trace).
    pub fn de(&self, m: &mut Marcher, x: f64, y: f64, z: f64) -> f64 {
        m.de_at_point(self.pos(x, y, z), false)
    }

    /// Scene position of a grid point.
    pub fn pos(&self, x: f64, y: f64, z: f64) -> Vec3 {
        let mut q = self.origin;
        for c in 0..3 {
            q[c] += self.m[0][c] * x + self.m[1][c] * y + self.m[2][c] * z;
        }
        q
    }

    fn sample(&self, m: &mut Marcher, x: f64, y: f64, z: f64, sharpness: f64) -> Sample {
        let de = m.de_at_point(self.pos(x, y, z), self.lv.is_some());
        let w = (1.0 - sharpness * de) as f32;
        let col = match &self.lv {
            Some(lv) => {
                let (si, ot) = m.color_values(de);
                let (c, idx) = lv.palette_color(si, ot);
                [c[0], c[1], c[2], idx]
            }
            None => [0.0; 4],
        };
        Sample { w, col }
    }
}

/// The DE stop the trace uses instead of the scene's, if the sharpness is
/// too high for it (see [`Sampler::new`]).
pub fn de_stop_for(sc: &Scene, mp: &MeshParams) -> Option<f64> {
    (mp.sharpness * sc.de_stop * 0.25 > 0.5).then(|| 2.0 / mp.sharpness.max(1e-9))
}

/// A corner or edge point of a cube (`TMCVertex`).
#[derive(Clone, Copy, Default)]
struct McVertex {
    pos: [f64; 3],
    w: f32,
    col: [f32; 4],
}

/// `ComputeEdgePoint`
fn edge_point(v1: &McVertex, v2: &McVertex) -> McVertex {
    let t = v2.w - v1.w;
    if t != 0.0 {
        let s = (0.0 - v1.w) / t;
        if (0.0..=1.0).contains(&s) {
            let s64 = s as f64;
            let mut e = McVertex::default();
            for c in 0..3 {
                e.pos[c] = v1.pos[c] + (v2.pos[c] - v1.pos[c]) * s64;
            }
            for c in 0..4 {
                e.col[c] = (v2.col[c] - v1.col[c]) * s + v1.col[c];
            }
            e
        } else if s < 0.0 {
            *v1
        } else {
            *v2
        }
    } else {
        *v2
    }
}

/// The edges of `ComputeEdgePoints` (BulbTracer2's numbering).
const EDGES: [(usize, usize); 12] =
    [(0, 1), (1, 2), (2, 3), (3, 0), (4, 5), (5, 6), (6, 7), (7, 4), (0, 4), (1, 5), (3, 7), (2, 6)];

/// Collects the triangles with shared vertices (`TFacesList`).
struct FaceBuilder {
    keys: HashMap<(i64, i64, i64), u32>,
    face_keys: HashSet<[u32; 3]>,
    mesh: Mesh,
}

impl FaceBuilder {
    fn new(colors: bool) -> Self {
        FaceBuilder {
            keys: HashMap::new(),
            face_keys: HashSet::new(),
            mesh: Mesh { colors: colors.then(Vec::new), ..Default::default() },
        }
    }

    /// `AddVertex`: vertices closer than 1e-4 are one (`MakeVertexKey`).
    fn vertex(&mut self, v: &McVertex) -> u32 {
        let key = ((v.pos[0] * 1e4).round() as i64, (v.pos[1] * 1e4).round() as i64, (v.pos[2] * 1e4).round() as i64);
        if let Some(&i) = self.keys.get(&key) {
            return i;
        }
        let i = self.mesh.vertices.len() as u32;
        self.mesh.vertices.push([v.pos[0] as f32, v.pos[1] as f32, v.pos[2] as f32]);
        if let Some(c) = self.mesh.colors.as_mut() {
            c.push(v.col);
        }
        self.keys.insert(key, i);
        i
    }

    /// `AddFace` with `ValidateFace` and without duplicates.
    fn face(&mut self, a: &McVertex, b: &McVertex, c: &McVertex) {
        let f = [self.vertex(a), self.vertex(b), self.vertex(c)];
        if f[0] == f[1] || f[1] == f[2] || f[0] == f[2] {
            return;
        }
        let mut k = f;
        k.sort_unstable();
        if self.face_keys.insert(k) {
            self.mesh.faces.push(f);
        }
    }

    /// `CreateFacesForCube`
    fn cube(&mut self, v: &[McVertex; 8]) {
        let mut case = 0usize;
        for (i, c) in v.iter().enumerate() {
            if c.w > 0.0 {
                case |= 1 << i;
            }
        }
        if case == 0 || case == 255 {
            return;
        }
        let e: Vec<McVertex> = EDGES.iter().map(|&(a, b)| edge_point(&v[a], &v[b])).collect();
        let list = if AMBIGUOUS_CASES.contains(&(case as u8)) { &FACES_AMBIGUOUS[case] } else { &FACES_STANDARD[case] };
        for t in list.chunks(3).take(5) {
            if t[0] >= 0 {
                self.face(&e[t[1] as usize], &e[t[0] as usize], &e[t[2] as usize]);
            }
        }
    }
}

/// Traces the object (`TParallelScanner2.ScannerScan3`).  `progress(done,
/// total)` counts grid planes.
pub fn trace(
    sc: &Scene,
    mp: &MeshParams,
    threads: usize,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<Mesh, String> {
    let n = mp.resolution.clamp(4, 4096);
    let sampler = Sampler::new(sc, mp)?;
    let step = GRID / (n - 1) as f64;
    let threads = if threads > 0 { threads } else { std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) };
    let side = n + 1;
    let b = mp.bounds;
    let outside = |x: f64, y: f64, z: f64| {
        let q = [x, y, z];
        (0..3).any(|k| q[k] <= b[k][0] || q[k] >= b[k][1])
    };
    // one plane of samples (x fixed), rows in parallel
    let plane = |i: usize| -> Vec<Sample> {
        let x = i as f64 * step;
        let mut out = vec![Sample::default(); side * side];
        let rows = std::sync::Mutex::new(&mut out);
        std::thread::scope(|s| {
            for t in 0..threads.min(side) {
                let rows = &rows;
                let sampler = &sampler;
                s.spawn(move || {
                    let mut m = Marcher::new(&sampler.p, 1);
                    let mut buf = vec![Sample::default(); side];
                    let mut j = t;
                    while j < side {
                        if cancel() {
                            return;
                        }
                        let y = j as f64 * step;
                        for (k, smp) in buf.iter_mut().enumerate() {
                            let z = k as f64 * step;
                            *smp = sampler.sample(&mut m, x, y, z, mp.sharpness);
                            // CloseMesh: the bounds count as outside
                            if mp.close && outside(x, y, z) {
                                smp.w = smp.w.min(0.0) - 100.0;
                            }
                        }
                        rows.lock().unwrap()[j * side..(j + 1) * side].copy_from_slice(&buf);
                        j += threads;
                    }
                });
            }
        });
        out
    };
    let mut fb = FaceBuilder::new(mp.colors);
    let mut a = plane(0);
    for i in 0..n {
        if cancel() {
            return Err("cancelled".into());
        }
        let bp = plane(i + 1);
        let x0 = i as f64 * step;
        for j in 0..n {
            let y0 = j as f64 * step;
            for k in 0..n {
                let z0 = k as f64 * step;
                // TraceXMin .. TraceZMax: cubes whose first corner is inside
                if x0 < b[0][0] || x0 > b[0][1] || y0 < b[1][0] || y0 > b[1][1] || z0 < b[2][0] || z0 > b[2][1] {
                    continue;
                }
                let at = |pl: &Vec<Sample>, jj: usize, kk: usize, x: f64| {
                    let s = pl[jj * side + kk];
                    McVertex { pos: [x, jj as f64 * step, kk as f64 * step], w: s.w, col: s.col }
                };
                let x1 = x0 + step;
                // InitializeCube: V0..V3 at z, V4..V7 at z + size
                let v = [
                    at(&a, j, k, x0),
                    at(&bp, j, k, x1),
                    at(&bp, j + 1, k, x1),
                    at(&a, j + 1, k, x0),
                    at(&a, j, k + 1, x0),
                    at(&bp, j, k + 1, x1),
                    at(&bp, j + 1, k + 1, x1),
                    at(&a, j + 1, k + 1, x0),
                ];
                fb.cube(&v);
            }
        }
        a = bp;
        progress(i + 1, n);
    }
    let mut mesh = fb.mesh;
    if mp.smooth > 0 {
        mesh.taubin(0.5, -0.53, mp.smooth);
    }
    mesh.center(1.0);
    Ok(mesh)
}

impl Mesh {
    /// `DoCenter`: centre the bounding box at 0 and scale its largest side
    /// to `size`.
    pub fn center(&mut self, size: f64) {
        if self.vertices.is_empty() {
            return;
        }
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for v in &self.vertices {
            for c in 0..3 {
                lo[c] = lo[c].min(v[c] as f64);
                hi[c] = hi[c].max(v[c] as f64);
            }
        }
        let s = (0..3).map(|c| hi[c] - lo[c]).fold(0.0, f64::max).max(1e-30);
        let scale = size / s;
        let d: Vec<f64> = (0..3).map(|c| -lo[c] - (hi[c] - lo[c]) / 2.0).collect();
        for v in self.vertices.iter_mut() {
            for c in 0..3 {
                v[c] = ((v[c] as f64 + d[c]) * scale) as f32;
            }
        }
    }

    /// `CalculateVertexNormals`: sums of the (area weighted) face normals.
    pub fn normals(&self) -> Vec<[f32; 3]> {
        let mut n = vec![[0f32; 3]; self.vertices.len()];
        for f in &self.faces {
            let [a, b, c] = f.map(|i| self.vertices[i as usize]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let cr = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            for &i in f {
                for k in 0..3 {
                    n[i as usize][k] += cr[k];
                }
            }
        }
        for v in n.iter_mut() {
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if l > 0.0 {
                v.iter_mut().for_each(|c| *c /= l);
            }
        }
        n
    }

    /// `TaubinSmooth`: passes of a Laplace step with `lambda` and one with `mu`.
    pub fn taubin(&mut self, lambda: f64, mu: f64, passes: u32) {
        let mut nb: Vec<Vec<u32>> = vec![Vec::new(); self.vertices.len()];
        for f in &self.faces {
            for (a, b) in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
                if !nb[a as usize].contains(&b) {
                    nb[a as usize].push(b);
                }
                if !nb[b as usize].contains(&a) {
                    nb[b as usize].push(a);
                }
            }
        }
        for _ in 0..passes {
            for strength in [lambda, mu] {
                let disp: Vec<[f64; 3]> = (0..self.vertices.len())
                    .map(|i| {
                        let mut d = [0f64; 3];
                        if nb[i].len() > 1 {
                            let w = 1.0 / nb[i].len() as f64;
                            let v = self.vertices[i];
                            for &j in &nb[i] {
                                let u = self.vertices[j as usize];
                                for c in 0..3 {
                                    d[c] += (u[c] - v[c]) as f64 * w;
                                }
                            }
                        }
                        d
                    })
                    .collect();
                for (v, d) in self.vertices.iter_mut().zip(&disp) {
                    for c in 0..3 {
                        v[c] += (d[c] * strength) as f32;
                    }
                }
            }
        }
    }

    /// `TObjFileWriter.SaveToFile`
    pub fn write_obj(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let n = self.normals();
        let colors = self.colors.as_ref().filter(|c| c.len() == self.vertices.len());
        writeln!(w, "####\n#\n# OBJ File Generated by Mandelbulb3D (mb3d-rs)\n#\n####\n#")?;
        writeln!(w, "# Vertices (with normals): {}\n# Faces: {}\n#\n####", self.vertices.len(), self.faces.len())?;
        for v in &self.vertices {
            writeln!(w, "v {} {} {}", v[0], v[1], v[2])?;
        }
        if let Some(c) = colors {
            // texture coordinate: the palette position, stretched to 0..1
            let (lo, hi) = c.iter().fold((1f32, 0f32), |(lo, hi), x| (lo.min(x[3]), hi.max(x[3])));
            let d = (hi - lo).max(0.0001);
            for x in c {
                writeln!(w, "vt {} 0.5", (x[3] - lo) / d)?;
            }
        }
        for v in &n {
            writeln!(w, "vn {} {} {}", v[0], v[1], v[2])?;
        }
        writeln!(w, "# {} vertices, {} vertices normals", self.vertices.len(), self.vertices.len())?;
        for f in &self.faces {
            let [a, b, c] = f.map(|i| i + 1);
            if colors.is_some() {
                writeln!(w, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}")?;
            } else {
                writeln!(w, "f {a}//{a} {b}//{b} {c}//{c}")?;
            }
        }
        writeln!(w, "# {} faces\n# End of File", self.faces.len())
    }

    /// `TPlyFileWriter.SaveToFile` (ASCII PLY with normals and colours)
    pub fn write_ply(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let n = self.normals();
        let colors = self.colors.as_ref().filter(|c| c.len() == self.vertices.len());
        write!(w, "ply\nformat ascii 1.0\nelement vertex {}\n", self.vertices.len())?;
        write!(w, "property float x\nproperty float y\nproperty float z\n")?;
        write!(w, "property float nx\nproperty float ny\nproperty float nz\n")?;
        if colors.is_some() {
            write!(w, "property uchar red\nproperty uchar green\nproperty uchar blue\n")?;
        }
        write!(w, "element face {}\nproperty list uchar uint vertex_indices\nend_header\n", self.faces.len())?;
        let rc = |v: f32| (255.0 * v).round().clamp(0.0, 255.0) as u8;
        for (i, v) in self.vertices.iter().enumerate() {
            write!(w, "{} {} {} {} {} {}", v[0], v[1], v[2], n[i][0], n[i][1], n[i][2])?;
            if let Some(c) = colors {
                write!(w, " {} {} {}", rc(c[i][0]), rc(c[i][1]), rc(c[i][2]))?;
            }
            writeln!(w)?;
        }
        for f in &self.faces {
            writeln!(w, "3 {} {} {}", f[0], f[1], f[2])?;
        }
        Ok(())
    }

    /// Binary STL (for 3D printing; not in MB3D).
    pub fn write_stl(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let mut head = [0u8; 80];
        let t = b"mb3d-rs mesh export";
        head[..t.len()].copy_from_slice(t);
        w.write_all(&head)?;
        w.write_all(&(self.faces.len() as u32).to_le_bytes())?;
        for f in &self.faces {
            let [a, b, c] = f.map(|i| self.vertices[i as usize]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let mut nn = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let l = (nn[0] * nn[0] + nn[1] * nn[1] + nn[2] * nn[2]).sqrt();
            if l > 0.0 {
                nn.iter_mut().for_each(|x| *x /= l);
            }
            for p in [nn, a, b, c] {
                for x in p {
                    w.write_all(&x.to_le_bytes())?;
                }
            }
            w.write_all(&[0, 0])?;
        }
        Ok(())
    }

    /// Writes the mesh by the file extension (.obj, .ply, .stl).
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut w = std::io::BufWriter::new(f);
        let r = match ext.as_str() {
            "obj" => self.write_obj(&mut w),
            "ply" => self.write_ply(&mut w),
            "stl" => self.write_stl(&mut w),
            _ => return Err(format!("{}: unknown mesh format (use .obj, .ply or .stl)", path.display())),
        };
        r.and_then(|_| w.flush()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Edges: (total, used by one face, used by more than two faces).  A
    /// closed surface has only edges shared by two faces.
    pub fn edge_stats(&self) -> (usize, usize, usize) {
        let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
        for f in &self.faces {
            for (a, b) in [(f[0], f[1]), (f[1], f[2]), (f[2], f[0])] {
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        let open = edges.values().filter(|&&c| c == 1).count();
        let more = edges.values().filter(|&&c| c > 2).count();
        (edges.len(), open, more)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_consistent() {
        // every case lists whole triangles, edges 0..11
        for t in FACES_STANDARD.iter().chain(FACES_AMBIGUOUS.iter()) {
            let n = t.iter().take_while(|&&e| e >= 0).count();
            assert_eq!(n % 3, 0);
            assert!(t.iter().all(|&e| (-1..12).contains(&e)));
        }
        assert!(FACES_STANDARD[0][0] < 0 && FACES_STANDARD[255][0] < 0);
    }

    #[test]
    fn analytic_sphere_is_watertight() {
        let mut fb = FaceBuilder::new(false);
        let n = 24;
        let at = |x: usize, y: usize, z: usize| {
            let p = [x as f64 / n as f64, y as f64 / n as f64, z as f64 / n as f64];
            let r = ((p[0] - 0.5).powi(2) + (p[1] - 0.52).powi(2) + (p[2] - 0.49).powi(2)).sqrt();
            McVertex { pos: p, w: (0.3 - r) as f32, col: [0.0; 4] }
        };
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    fb.cube(&[at(i, j, k), at(i + 1, j, k), at(i + 1, j + 1, k), at(i, j + 1, k),
                        at(i, j, k + 1), at(i + 1, j, k + 1), at(i + 1, j + 1, k + 1), at(i, j + 1, k + 1)]);
                }
            }
        }
        let (e, open, more) = fb.mesh.edge_stats();
        assert!(e > 1000 && open == 0 && more == 0, "{e} {open} {more}");
        // MB3D's triangle order gives outward normals
        let m = &fb.mesh;
        let n = m.normals();
        let outward = m.vertices.iter().zip(&n).filter(|(v, n)| (v[0] - 0.5) * n[0] + (v[1] - 0.52) * n[1] + (v[2] - 0.49) * n[2] > 0.0).count();
        assert_eq!(outward, m.vertices.len(), "normals must point outwards");
    }

    #[test]
    fn bulb_mesh() {
        let s = Scene::preset("Integer Power").unwrap();
        let mp = MeshParams { resolution: 32, colors: true, close: true, ..Default::default() };
        let m = trace(&s, &mp, 2, &|_, _| {}, &|| false).unwrap();
        assert!(m.faces.len() > 1000, "{} faces", m.faces.len());
        assert_eq!(m.colors.as_ref().unwrap().len(), m.vertices.len());
        // centred and scaled to 1
        let mx = m.vertices.iter().fold(0f32, |a, v| a.max(v[0].abs()).max(v[1].abs()).max(v[2].abs()));
        assert!((mx - 0.5).abs() < 1e-3, "{mx}");
        // the bulb has a size of about 2.2 units, the cube 2.2 / 0.5: it
        // fills about half of it
        let mut out = Vec::new();
        m.write_obj(&mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("\nf 1/1/1 ") || text.contains("\nf "));
        let mut ply = Vec::new();
        m.write_ply(&mut ply).unwrap();
        assert!(String::from_utf8_lossy(&ply).contains(&format!("element face {}", m.faces.len())));
        let mut stl = Vec::new();
        m.write_stl(&mut stl).unwrap();
        assert_eq!(stl.len(), 84 + 50 * m.faces.len());
    }

    #[test]
    fn sphere_is_closed_and_round() {
        // a quaternion julia with c = 0 at low iterations is close to a ball;
        // check the surface is closed and smooth after Taubin smoothing
        let mut s = Scene::preset("Quaternion").unwrap();
        s.julia = true;
        s.julia_c = [0.0; 4];
        s.iterations = 6;
        let mp = MeshParams { resolution: 40, scale: 0.8, close: true, smooth: 2, ..Default::default() };
        let m = trace(&s, &mp, 2, &|_, _| {}, &|| false).unwrap();
        // closed up to the rare slivers lost by merging vertices closer
        // than 1e-4 grid units (as in MB3D)
        let (e, open, more) = m.edge_stats();
        assert!(open * 500 < e && more == 0, "{e} edges, {open} open, {more} shared by more than 2");
        // all vertices about equally far from the centre
        let r: Vec<f32> = m.vertices.iter().map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()).collect();
        let (lo, hi) = r.iter().fold((f32::MAX, 0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(lo > 0.3 * hi, "radius {lo}..{hi}");
    }
}

