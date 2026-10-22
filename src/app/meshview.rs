//! The mesh windows of MB3D, which draw with OpenGL there: the mesh preview
//! of BulbTracer2 (opengl/MeshPreviewUI.pas) and the HeightMap generator
//! (heightmapgen/HeightMapGenUI.pas).  This program draws the meshes with
//! a small z-buffer rasteriser using the same projection and transforms.

use super::util::{fts, parse_float, pti};
use super::Mb3d;
use crate::mesh::Mesh;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::form::{SS_LEFT, SS_MIDDLE, SS_RIGHT};
use crate::vcl::{DialogResult, Ev, Event, Ui};
use std::path::Path;

const MP: &str = "MeshPreviewFrm";
const HM: &str = "HeightMapGenFrm";
const MOVE_SCALE: f64 = 0.001;
const SIZE_SCALE: f64 = 0.01;
const ROTATE_SCALE: f64 = 0.2;

type V3 = [f64; 3];
type M4 = [[f64; 4]; 4];

/// A mesh prepared for drawing (`UpdateMesh`): y and z negated as MB3D does.
#[derive(Clone, Default)]
pub struct GlMesh {
    verts: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    faces: Vec<[u32; 3]>,
    colors: Option<Vec<[f32; 3]>>,
    max_size: f64,
}

impl GlMesh {
    pub fn new(m: &Mesh) -> GlMesh {
        let verts: Vec<[f32; 3]> = m.vertices.iter().map(|v| [v[0], -v[1], -v[2]]).collect();
        let normals = m.normals().iter().map(|n| [n[0], n[1], -n[2]]).collect();
        let mut lo = [f64::MAX; 3];
        let mut hi = [f64::MIN; 3];
        for v in &m.vertices {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k] as f64);
                hi[k] = hi[k].max(v[k] as f64);
            }
        }
        let max_size = (0..3).map(|k| hi[k] - lo[k]).fold(0.0, f64::max).max(1e-9);
        let colors = m.colors.as_ref().map(|c| c.iter().map(|c| [c[0], c[1], c[2]]).collect());
        GlMesh { verts, normals, faces: m.faces.clone(), colors, max_size }
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

/// `TMeshAppearance`
#[derive(Clone, Debug)]
pub struct Appearance {
    pub surface: V3,
    pub edges: V3,
    pub wireframe: V3,
    pub points: V3,
    pub mat_ambient: V3,
    pub mat_diffuse: V3,
    pub mat_specular: V3,
    pub shininess: f64,
    pub light_ambient: V3,
    pub light_diffuse: V3,
    pub light_pos: V3,
    pub att_const: f64,
    pub att_linear: f64,
    pub att_quadratic: f64,
    pub lighting: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            surface: [0.776_470_588, 0.764_705_882, 0.717_647_059],
            edges: [0.2, 0.2, 0.2],
            wireframe: [0.2, 0.2, 0.7],
            points: [0.2, 0.7, 0.2],
            mat_ambient: [32.0 / 255.0, 32.0 / 255.0, 36.0 / 255.0],
            mat_diffuse: [0.49, 0.49, 0.49],
            mat_specular: [1.0, 240.0 / 255.0, 230.0 / 255.0],
            shininess: 2.0,
            light_ambient: [0.36, 0.36, 0.36],
            light_diffuse: [0.8, 0.8, 0.8],
            light_pos: [-1.3, 7.4, 17.0],
            att_const: 0.12,
            att_linear: 0.0025,
            att_quadratic: 0.00002,
            lighting: true,
        }
    }
}

/// Display styles of the mesh preview (`TDisplayStyle`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Points,
    Wireframe,
    Flat,
    FlatEdges,
    Smooth,
    SmoothEdges,
    /// the height map shader: brightness from the depth
    Depth,
}

/// The view (`FPosition`, `FAngle`, `FScale`, `FFOV`).
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub pos: V3,
    pub angle: [f64; 2],
    pub scale: f64,
    pub fov: f64,
    /// the mesh preview moves the camera back by 7 / scale, the height map
    /// generator scales the object
    pub scale_distance: bool,
}

fn mul(a: &M4, b: &M4) -> M4 {
    let mut r = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

fn ident() -> M4 {
    let mut m = [[0.0; 4]; 4];
    for (i, r) in m.iter_mut().enumerate() {
        r[i] = 1.0;
    }
    m
}

fn translate(x: f64, y: f64, z: f64) -> M4 {
    let mut m = ident();
    m[0][3] = x;
    m[1][3] = y;
    m[2][3] = z;
    m
}

fn scale(s: f64) -> M4 {
    let mut m = ident();
    for (i, r) in m.iter_mut().enumerate().take(3) {
        r[i] = s;
    }
    m
}

/// `glRotatef`
fn rotate(deg: f64, axis: V3) -> M4 {
    let l = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt().max(1e-12);
    let (x, y, z) = (axis[0] / l, axis[1] / l, axis[2] / l);
    let (s, c) = deg.to_radians().sin_cos();
    let t = 1.0 - c;
    [
        [x * x * t + c, x * y * t - z * s, x * z * t + y * s, 0.0],
        [y * x * t + z * s, y * y * t + c, y * z * t - x * s, 0.0],
        [x * z * t - y * s, y * z * t + x * s, z * z * t + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn xf(m: &M4, v: [f64; 4]) -> [f64; 4] {
    let mut r = [0.0; 4];
    for (i, ri) in r.iter_mut().enumerate() {
        *ri = (0..4).map(|k| m[i][k] * v[k]).sum();
    }
    r
}

/// What [`render`] returns: the picture and the window depth (0..1, 1 =
/// nothing) per pixel, top row first.
pub struct Frame {
    pub img: Bitmap,
    pub depth: Vec<f32>,
}

const NEAR: f64 = 1.0;
const FAR: f64 = 100.0;

/// Draws the mesh (`ApplicationEventsIdle`).
pub fn render(m: &GlMesh, v: &View, style: Style, ap: &Appearance, w: usize, h: usize) -> Frame {
    let (w, h) = (w.max(1), h.max(1));
    let mut img = Bitmap::new(w, h, 0xFF00_0000);
    let mut depth = vec![1.0f32; w * h];
    if m.is_empty() {
        return Frame { img, depth };
    }
    const ZOFF: f64 = -7.0;
    let (zoff, scl) = if v.scale_distance { (ZOFF / v.scale.max(1e-6), 3.0 / m.max_size) } else { (ZOFF, v.scale * 3.0 / m.max_size) };
    let mv = mul(
        &mul(&mul(&translate(v.pos[0], v.pos[1], zoff), &rotate(v.angle[0], [0.0, 1.0, 0.0])), &rotate(-v.angle[1], [1.0, 0.0, 1.0])),
        &mul(&scale(scl), &translate(0.0, 0.0, v.pos[2])),
    );
    let f = 1.0 / (v.fov.to_radians() * 0.5).tan();
    let aspect = w as f64 / h as f64;
    let mut pr = [[0.0; 4]; 4];
    pr[0][0] = f / aspect;
    pr[1][1] = f;
    pr[2][2] = (FAR + NEAR) / (NEAR - FAR);
    pr[2][3] = 2.0 * FAR * NEAR / (NEAR - FAR);
    pr[3][2] = -1.0;
    // eye space positions, normals and window coordinates
    let n = m.verts.len();
    let mut eye = Vec::with_capacity(n);
    let mut win: Vec<Option<[f64; 3]>> = Vec::with_capacity(n);
    for p in &m.verts {
        let e = xf(&mv, [p[0] as f64, p[1] as f64, p[2] as f64, 1.0]);
        let c = xf(&pr, e);
        eye.push([e[0], e[1], e[2]]);
        // (in front of the near plane: no clipping, its faces and edges go;
        // a tiny w would put it millions of pixels away)
        if c[3] < NEAR {
            win.push(None);
        } else {
            let (nx, ny, nz) = (c[0] / c[3], c[1] / c[3], c[2] / c[3]);
            win.push(Some([(nx + 1.0) * 0.5 * w as f64, (1.0 - ny) * 0.5 * h as f64, (nz + 1.0) * 0.5]));
        }
    }
    // normals into eye space (the rotation part; uniform scale)
    let nrm: Vec<V3> = m
        .normals
        .iter()
        .map(|q| {
            let e = xf(&mv, [q[0] as f64, q[1] as f64, q[2] as f64, 0.0]);
            let l = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt().max(1e-12);
            [e[0] / l, e[1] / l, e[2] / l]
        })
        .collect();
    // glLightfv(GL_POSITION) after the transforms: in object space
    let lpe = xf(&mv, [ap.light_pos[0], ap.light_pos[1], ap.light_pos[2], 1.0]);
    let light = |p: V3, nv: V3, base: Option<V3>| -> V3 {
        if !ap.lighting {
            return base.unwrap_or(ap.surface);
        }
        let lp = [lpe[0], lpe[1], lpe[2]];
        let d = [lp[0] - p[0], lp[1] - p[1], lp[2] - p[2]];
        let dl = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-12);
        let l = [d[0] / dl, d[1] / dl, d[2] / dl];
        // capped at 1: the uncapped OpenGL factor of MB3D's defaults washes the mesh out
        let att = (1.0 / (ap.att_const + ap.att_linear * dl + ap.att_quadratic * dl * dl).max(1e-9)).min(1.0);
        let nl = (nv[0] * l[0] + nv[1] * l[1] + nv[2] * l[2]).max(0.0);
        // Blinn half vector with the viewer at infinity on +z
        let hv = [l[0], l[1], l[2] + 1.0];
        let hl = (hv[0] * hv[0] + hv[1] * hv[1] + hv[2] * hv[2]).sqrt().max(1e-12);
        let nh = ((nv[0] * hv[0] + nv[1] * hv[1] + nv[2] * hv[2]) / hl).max(0.0);
        let spec = if nl > 0.0 { nh.powf(ap.shininess) } else { 0.0 };
        let dif = base.unwrap_or(ap.mat_diffuse);
        let mut c = [0.0; 3];
        for k in 0..3 {
            c[k] = ap.mat_ambient[k] * ap.light_ambient[k]
                + att * (dif[k] * ap.light_diffuse[k] * nl + ap.mat_specular[k] * ap.light_diffuse[k] * spec * 0.5);
        }
        c
    };
    let to_px = |c: V3| -> u32 {
        let b = |x: f64| ((x.clamp(0.0, 1.0) * 255.0).round() as u32) & 255;
        0xFF00_0000 | b(c[0]) << 16 | b(c[1]) << 8 | b(c[2])
    };
    let depth_shade = |d: f64| -> u32 {
        // the height map shader: linearizeDepth(z) / 16 with near 1, far 36
        let (n, fa) = (1.0, 36.0);
        let lin = (2.0 * n * fa) / (fa + n - (d * 2.0 - 1.0) * (fa - n)) / 16.0;
        let g = 1.0 - lin;
        to_px([g, g, g])
    };
    let plot = |x: i64, y: i64, z: f64, col: u32, img: &mut Bitmap, depth: &mut [f32]| {
        if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
            return;
        }
        let i = y as usize * w + x as usize;
        if (z as f32) < depth[i] && z >= 0.0 {
            depth[i] = z as f32;
            img.px[i] = col;
        }
    };
    let solid = matches!(style, Style::Flat | Style::FlatEdges | Style::Smooth | Style::SmoothEdges | Style::Depth);
    if solid {
        for fc in &m.faces {
            let idx = [fc[0] as usize, fc[1] as usize, fc[2] as usize];
            if idx.iter().any(|&i| i >= n) {
                continue;
            }
            let (Some(a), Some(b), Some(c)) = (win[idx[0]], win[idx[1]], win[idx[2]]) else { continue };
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if area.abs() < 1e-12 {
                continue;
            }
            // colours at the corners (flat: the face normal at the first)
            let cols: [u32; 3] = match style {
                Style::Depth => [0; 3],
                Style::Flat | Style::FlatEdges => {
                    let (p0, p1, p2) = (eye[idx[0]], eye[idx[1]], eye[idx[2]]);
                    let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                    let q = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
                    let mut fnm = [u[1] * q[2] - u[2] * q[1], u[2] * q[0] - u[0] * q[2], u[0] * q[1] - u[1] * q[0]];
                    let l = (fnm[0] * fnm[0] + fnm[1] * fnm[1] + fnm[2] * fnm[2]).sqrt().max(1e-12);
                    fnm = [fnm[0] / l, fnm[1] / l, fnm[2] / l];
                    if fnm[2] < 0.0 {
                        fnm = [-fnm[0], -fnm[1], -fnm[2]];
                    }
                    let base = m.colors.as_ref().map(|c| c[idx[0]].map(|x| x as f64));
                    let c = to_px(light(p0, fnm, base));
                    [c; 3]
                }
                _ => {
                    let mut out = [0u32; 3];
                    for k in 0..3 {
                        let mut nv = nrm.get(idx[k]).copied().unwrap_or([0.0, 0.0, 1.0]);
                        if nv[2] < 0.0 {
                            nv = [-nv[0], -nv[1], -nv[2]];
                        }
                        let base = m.colors.as_ref().map(|c| c[idx[k]].map(|x| x as f64));
                        out[k] = to_px(light(eye[idx[k]], nv, base));
                    }
                    out
                }
            };
            let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i64;
            let x1 = a[0].max(b[0]).max(c[0]).ceil().min(w as f64 - 1.0) as i64;
            let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i64;
            let y1 = a[1].max(b[1]).max(c[1]).ceil().min(h as f64 - 1.0) as i64;
            for y in y0..=y1 {
                let py = y as f64 + 0.5;
                for x in x0..=x1 {
                    let px = x as f64 + 0.5;
                    let w0 = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) / area;
                    let w1 = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * a[2] + w1 * b[2] + w2 * c[2];
                    let col = match style {
                        Style::Depth => depth_shade(z),
                        Style::Flat | Style::FlatEdges => cols[0],
                        _ => {
                            let ch = |s: u32| -> f64 {
                                let g = |cv: u32| ((cv >> s) & 255) as f64;
                                w0 * g(cols[0]) + w1 * g(cols[1]) + w2 * g(cols[2])
                            };
                            0xFF00_0000 | (ch(16).round() as u32) << 16 | (ch(8).round() as u32) << 8 | ch(0).round() as u32
                        }
                    };
                    plot(x, y, z, col, &mut img, &mut depth);
                }
            }
        }
    }
    // edges and points (drawn slightly in front, as OpenGL draws them on top)
    let line_col = match style {
        Style::Wireframe => Some(ap.wireframe),
        Style::FlatEdges | Style::SmoothEdges => Some(ap.edges),
        _ => None,
    };
    if let Some(lc) = line_col {
        let col = to_px(lc);
        for fc in &m.faces {
            for (p, q) in [(fc[0], fc[1]), (fc[1], fc[2])] {
                let (Some(a), Some(b)) = (win.get(p as usize).copied().flatten(), win.get(q as usize).copied().flatten()) else { continue };
                // the part of the edge on the screen (Liang-Barsky)
                let (mut t0, mut t1) = (0.0f64, 1.0f64);
                for (d, lo, hi) in [(b[0] - a[0], -1.0 - a[0], w as f64 + 1.0 - a[0]), (b[1] - a[1], -1.0 - a[1], h as f64 + 1.0 - a[1])] {
                    if d.abs() < 1e-12 {
                        if lo > 0.0 || hi < 0.0 {
                            t1 = -1.0;
                        }
                    } else {
                        let (u, v) = if d > 0.0 { (lo / d, hi / d) } else { (hi / d, lo / d) };
                        t0 = t0.max(u);
                        t1 = t1.min(v);
                    }
                }
                if t0 > t1 {
                    continue;
                }
                let steps = ((b[0] - a[0]).abs().max((b[1] - a[1]).abs()) * (t1 - t0)).ceil().max(1.0) as i64;
                for k in 0..=steps {
                    let t = t0 + (t1 - t0) * k as f64 / steps as f64;
                    let z = a[2] + (b[2] - a[2]) * t - 2e-5;
                    plot((a[0] + (b[0] - a[0]) * t) as i64, (a[1] + (b[1] - a[1]) * t) as i64, z, col, &mut img, &mut depth);
                }
            }
        }
    }
    if style == Style::Points {
        for (i, p) in win.iter().enumerate() {
            let Some(p) = p else { continue };
            let c = m.colors.as_ref().map(|c| c[i].map(|x| x as f64)).unwrap_or(ap.points);
            let col = to_px(c);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                plot(p[0] as i64 + dx, p[1] as i64 + dy, p[2], col, &mut img, &mut depth);
            }
        }
    }
    Frame { img, depth }
}

/// Reads a Wavefront OBJ file (`TWavefrontObjFileReader`): vertices and
/// faces (polygons as fans).
pub fn read_obj(text: &str) -> Result<Mesh, String> {
    let mut m = Mesh::default();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let v: Vec<f32> = it.take(3).filter_map(|s| s.parse().ok()).collect();
                if v.len() == 3 {
                    m.vertices.push([v[0], v[1], v[2]]);
                }
            }
            Some("f") => {
                let n = m.vertices.len() as i64;
                let idx: Vec<u32> = it
                    .filter_map(|s| s.split('/').next()?.parse::<i64>().ok())
                    .map(|i| if i < 0 { n + i } else { i - 1 })
                    .filter(|&i| i >= 0 && i < n)
                    .map(|i| i as u32)
                    .collect();
                for k in 1..idx.len().saturating_sub(1) {
                    m.faces.push([idx[0], idx[k], idx[k + 1]]);
                }
            }
            _ => {}
        }
    }
    if m.faces.is_empty() {
        return Err("no faces in the file".into());
    }
    Ok(m)
}

/// The area the mesh is drawn into: an image control behind the panel
/// that takes the form's mouse events (the GL surface of MB3D).
fn ensure_view(ui: &mut Ui, form: &str) {
    let f = ui.fm(form);
    if f.id("GLView").is_some() {
        return;
    }
    let mut c = crate::vcl::control::Control::new("GLView", "TImage");
    c.align = crate::vcl::control::Align::Client;
    for (ev, h) in [
        ("OnMouseDown", "FormMouseDown"),
        ("OnMouseMove", "FormMouseMove"),
        ("OnMouseUp", "FormMouseUp"),
        ("OnDblClick", "FormDblClick"),
    ] {
        c.events.insert(ev.into(), h.into());
    }
    let i = f.add_control(0, c);
    f.ctl[0].children.retain(|&k| k != i);
    f.ctl[0].children.insert(0, i);
}

/// Mouse navigation state of a window (`FStartMouseX` ...).
#[derive(Clone, Copy, Default)]
struct Drag {
    active: bool,
    x: i32,
    y: i32,
    pos: [f64; 2],
    angle: [f64; 2],
    scale: f64,
}

pub struct State {
    // mesh preview
    pub preview_mesh: GlMesh,
    pub preview_view: View,
    pub style: Style,
    pub ap: Appearance,
    preview_drag: Drag,
    preview_title: String,
    // height map generator
    hm_mesh: GlMesh,
    hm_view: View,
    hm_drag: Drag,
    hm_depth: Vec<f32>,
    hm_size: (usize, usize),
    pending_save: Option<std::path::PathBuf>,
    dirty: [bool; 2],
    refreshing: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            preview_mesh: GlMesh::default(),
            preview_view: View { pos: [0.0; 3], angle: [0.0; 2], scale: 1.0, fov: 30.0, scale_distance: true },
            style: Style::SmoothEdges,
            ap: Appearance::default(),
            preview_drag: Drag::default(),
            preview_title: "Generated Mesh".into(),
            hm_mesh: GlMesh::default(),
            hm_view: View { pos: [0.0; 3], angle: [0.0; 2], scale: 0.5, fov: 10.0, scale_distance: false },
            hm_drag: Drag::default(),
            hm_depth: Vec::new(),
            hm_size: (0, 0),
            pending_save: None,
            dirty: [true; 2],
            refreshing: false,
        }
    }
}

/// Shows a mesh in the preview window (`MeshPreviewFrm.UpdateMesh`).
pub fn show_mesh(app: &mut Mb3d, ui: &mut Ui, m: &Mesh, title: &str) {
    app.meshview.preview_mesh = GlMesh::new(m);
    app.meshview.preview_title = title.into();
    app.meshview.dirty[0] = true;
    ui.show(MP);
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    for (k, form) in [MP, HM].iter().enumerate() {
        if !ui.showing(form) {
            continue;
        }
        ensure_view(ui, form);
        let r = ui.ctl_rect(form, "GLView");
        let (w, h) = (r.w.max(1) as usize, r.h.max(1) as usize);
        let st = &mut app.meshview;
        let size_changed = ui.c(form, "GLView").picture.as_ref().map(|p| (p.w, p.h)) != Some((w, h));
        if !st.dirty[k] && !size_changed {
            continue;
        }
        st.dirty[k] = false;
        let fr = if k == 0 {
            render(&st.preview_mesh, &st.preview_view, st.style, &st.ap, w, h)
        } else {
            render(&st.hm_mesh, &st.hm_view, Style::Depth, &st.ap, w, h)
        };
        if k == 1 {
            st.hm_depth = fr.depth;
            st.hm_size = (w, h);
        }
        ui.set_picture(form, "GLView", Some(fr.img));
        let (mesh, title) = if k == 0 { (&st.preview_mesh, st.preview_title.clone()) } else { (&st.hm_mesh, "HeightMap Generator Preview".to_string()) };
        let cap = format!("{title} [{} Vertices, {} Faces]", mesh.verts.len(), mesh.faces.len());
        ui.fm(form).set_caption(&cap);
    }
}

/// The mouse navigation of both windows (`FormMouseDown` ... `FormMouseWheelUp`).
fn navigate(view: &mut View, drag: &mut Drag, e: &Event) -> bool {
    match e.ev {
        Ev::MouseDown { x, y, .. } => {
            *drag = Drag { active: true, x, y, pos: [view.pos[0], view.pos[1]], angle: view.angle, scale: view.scale };
            false
        }
        Ev::MouseMove { x, y, shift } if drag.active => {
            if shift & SS_MIDDLE != 0 {
                view.scale = drag.scale - SIZE_SCALE * (drag.x - x) as f64;
            }
            if shift & SS_LEFT != 0 {
                view.pos[0] = drag.pos[0] - MOVE_SCALE * (drag.x - x) as f64;
                view.pos[1] = drag.pos[1] + MOVE_SCALE * (drag.y - y) as f64;
            }
            if shift & SS_RIGHT != 0 {
                view.angle[0] = drag.angle[0] - ROTATE_SCALE * (drag.x - x) as f64;
                view.angle[1] = drag.angle[1] + ROTATE_SCALE * (drag.y - y) as f64;
            }
            true
        }
        Ev::MouseUp { .. } => {
            drag.active = false;
            false
        }
        Ev::Wheel { delta, .. } => {
            view.scale += SIZE_SCALE * 3.0 * if delta > 0 { 1.0 } else { -1.0 };
            true
        }
        _ => false,
    }
}

fn col_to_vec(c: u32) -> V3 {
    [((c >> 16) & 255) as f64 / 255.0, ((c >> 8) & 255) as f64 / 255.0, (c & 255) as f64 / 255.0]
}

fn vec_to_col(v: V3) -> u32 {
    let b = |x: f64| ((x * 255.0).round().clamp(0.0, 255.0)) as u32;
    0xFF00_0000 | b(v[0]) << 16 | b(v[1]) << 8 | b(v[2])
}

const COLOR_BTNS: [&str; 9] = [
    "SurfaceColorBtn",
    "EdgesColorBtn",
    "WireframeColorBtn",
    "PointsColorBtn",
    "MatAmbientColorBtn",
    "MatDiffuseColorBtn",
    "MatSpecularColorBtn",
    "LightAmbientBtn",
    "LightDiffuseBtn",
];

fn ap_color(ap: &mut Appearance, i: usize) -> &mut V3 {
    match i {
        0 => &mut ap.surface,
        1 => &mut ap.edges,
        2 => &mut ap.wireframe,
        3 => &mut ap.points,
        4 => &mut ap.mat_ambient,
        5 => &mut ap.mat_diffuse,
        6 => &mut ap.mat_specular,
        7 => &mut ap.light_ambient,
        _ => &mut ap.light_diffuse,
    }
}

const NUM_EDITS: [&str; 7] = [
    "MatShininessEdit",
    "LightPositionXEdit",
    "LightPositionYEdit",
    "LightPositionZEdit",
    "ConstAttenuationEdit",
    "LinearAttenuationEdit",
    "QuadraticAttenuationEdit",
];

fn ap_num(ap: &mut Appearance, i: usize) -> &mut f64 {
    match i {
        0 => &mut ap.shininess,
        1 => &mut ap.light_pos[0],
        2 => &mut ap.light_pos[1],
        3 => &mut ap.light_pos[2],
        4 => &mut ap.att_const,
        5 => &mut ap.att_linear,
        _ => &mut ap.att_quadratic,
    }
}

/// `AppearanceToUI`
fn appearance_to_ui(app: &mut Mb3d, ui: &mut Ui) {
    app.meshview.refreshing = true;
    let mut ap = app.meshview.ap.clone();
    for (i, b) in COLOR_BTNS.iter().enumerate() {
        ui.cm(MP, b).brush_color = vec_to_col(*ap_color(&mut ap, i));
    }
    for (i, e) in NUM_EDITS.iter().enumerate() {
        ui.set_text(MP, e, &fts(*ap_num(&mut ap, i)));
    }
    ui.set_checked(MP, "LightingEnabledCBx", ap.lighting);
    app.meshview.refreshing = false;
}

/// `UIToAppearance`
fn ui_to_appearance(app: &mut Mb3d, ui: &Ui) {
    if app.meshview.refreshing {
        return;
    }
    let ap = &mut app.meshview.ap;
    for (i, b) in COLOR_BTNS.iter().enumerate() {
        *ap_color(ap, i) = col_to_vec(ui.c(MP, b).brush_color);
    }
    for (i, e) in NUM_EDITS.iter().enumerate() {
        if let Some(v) = parse_float(&ui.text(MP, e)) {
            *ap_num(ap, i) = v;
        }
    }
    ap.lighting = ui.checked(MP, "LightingEnabledCBx");
    app.meshview.dirty[0] = true;
}

pub fn preview_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    match h {
        "FormCreate" => {
            ui.set_active_page(MP, "AppearancePageCtrl", "MaterialSheet");
            appearance_to_ui(app, ui);
            if ui.item_index(MP, "DisplayStyleGrp") < 0 {
                ui.set_item_index(MP, "DisplayStyleGrp", 5);
            }
            display_style(app, ui);
        }
        "FormShow" => {
            ensure_view(ui, MP);
            appearance_to_ui(app, ui);
            app.meshview.dirty[0] = true;
        }
        "FormDblClick" | "NaviPanelDblClick" => {
            let v = !ui.visible(MP, "NaviPanel");
            ui.set_visible(MP, "NaviPanel", v);
            app.meshview.dirty[0] = true;
        }
        "FormMouseDown" | "FormMouseMove" | "FormMouseUp" | "FormMouseWheelDown" | "FormMouseWheelUp" => {
            let st = &mut app.meshview;
            if navigate(&mut st.preview_view, &mut st.preview_drag, e) {
                st.dirty[0] = true;
            }
        }
        "DisplayStyleGrpClick" => display_style(app, ui),
        "SurfaceColorBtnColorChange" | "MatShininessEditExit" | "LightingEnabledCBxClick" => ui_to_appearance(app, ui),
        "SurfaceColorBtnClick" => {
            let c = ui.c(MP, &e.sender).brush_color;
            ui.pick_color(&format!("@color:{MP}:{}", e.sender), c);
        }
        "MatShininessBtnClick" | "LightPositionXBtnClick" | "LightPositionYBtnClick" | "LightPositionZBtnClick" | "ConstAttenuationBtnClick"
        | "LinearAttenuationBtnClick" | "QuadraticAttenuationBtnClick" => {
            let up = matches!(e.ev, Ev::UpDown { up: true, .. });
            let (edit, step, min) = match h {
                "MatShininessBtnClick" => ("MatShininessEdit", 1.0, Some(0.0)),
                "LightPositionXBtnClick" => ("LightPositionXEdit", 0.5, None),
                "LightPositionYBtnClick" => ("LightPositionYEdit", 0.5, None),
                "LightPositionZBtnClick" => ("LightPositionZEdit", 0.5, None),
                "ConstAttenuationBtnClick" => ("ConstAttenuationEdit", 0.05, Some(0.0)),
                "LinearAttenuationBtnClick" => ("LinearAttenuationEdit", 0.0005, Some(0.0)),
                _ => ("QuadraticAttenuationEdit", 0.000005, Some(0.0)),
            };
            let mut v = parse_float(&ui.text(MP, edit)).unwrap_or(0.0) + if up { step } else { -step };
            if let Some(m) = min {
                v = v.max(m);
            }
            ui.set_text(MP, edit, &fts(v));
            ui_to_appearance(app, ui);
        }
        _ => {}
    }
}

fn display_style(app: &mut Mb3d, ui: &Ui) {
    app.meshview.style = match ui.item_index(MP, "DisplayStyleGrp") {
        0 => Style::Points,
        1 => Style::Wireframe,
        2 => Style::Flat,
        3 => Style::FlatEdges,
        4 => Style::Smooth,
        _ => Style::SmoothEdges,
    };
    app.meshview.dirty[0] = true;
}

// ---- HeightMap generator

/// `FindHeightMap`: a height map file with this number.
fn find_height_map(dir: &Path, nr: i32) -> Option<std::path::PathBuf> {
    let n = nr.to_string();
    let ok = |p: &Path| {
        p.extension().map(|e| ["jpg", "png", "bmp", "jpeg", "pgm"].contains(&e.to_string_lossy().to_ascii_lowercase().as_str())).unwrap_or(false)
    };
    let rd: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).collect();
    rd.iter()
        .find(|p| p.file_stem().map(|s| s.to_string_lossy() == n).unwrap_or(false) && ok(p))
        .or_else(|| {
            rd.iter().find(|p| {
                let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                ok(p) && name.len() > n.len() && name.starts_with(&n) && !name.as_bytes()[n.len()].is_ascii_digit()
            })
        })
        .cloned()
}

fn hm_ext(ui: &Ui) -> &'static str {
    if ui.item_index(HM, "MapTypeCmb") == 0 {
        "png"
    } else {
        "pgm"
    }
}

/// `SaveHeightMap`: the depth of the visible mesh, near = bright.
fn save_height_map(app: &Mb3d, path: &Path) -> Result<(), String> {
    let st = &app.meshview;
    let (w, h) = st.hm_size;
    if st.hm_depth.len() != w * h || w == 0 {
        return Err("nothing to save".into());
    }
    let (zn, zf) = (NEAR, FAR);
    let lin: Vec<f64> = st.hm_depth.iter().map(|&d| (2.0 * zn) / (zf + zn - d as f64 * (zf - zn))).collect();
    let mut dmax = 0.0f64;
    for &d in &lin {
        if d > dmax && d < 0.99 {
            dmax = d;
        }
    }
    let vals: Vec<f64> = lin.iter().map(|&d| if d > dmax { 0.0 } else { dmax - d }).collect();
    let (mut fmin, mut fmax) = (1.0f64, 0.0f64);
    for &v in &vals {
        fmin = fmin.min(v);
        fmax = fmax.max(v);
    }
    let delta = (fmax - fmin).max(1e-12);
    let norm = |v: f64| ((v - fmin) / delta).clamp(0.0, 1.0);
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let data = if ext == "pgm" {
        // TPGM16Writer: plain text PGM, 16 bit
        let mut s = format!("P2\n# MB3D\n{w} {h}\n65535\n");
        for y in 0..h {
            let row: Vec<String> = (0..w).map(|x| ((norm(vals[y * w + x]) * 65535.0).round() as u32).to_string()).collect();
            s.push_str(&row.join(" "));
            s.push('\n');
        }
        s.into_bytes()
    } else {
        let g: Vec<u8> = vals.iter().map(|&v| (norm(v) * 255.0).round() as u8).collect();
        crate::png::encode_gray8(w, h, &g)
    };
    std::fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

fn hm_enable(app: &Mb3d, ui: &mut Ui) {
    let any = !app.meshview.hm_mesh.is_empty();
    ui.set_enabled(HM, "SaveMapBtn", any);
    ui.set_enabled(HM, "SaveImgBtn", any);
}

pub fn heightmap_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    match h {
        "FormCreate" => {
            ui.set_items(HM, "MapTypeCmb", vec!["8 Bit PNG".into(), "16 Bit PGM".into()]);
            ui.set_item_index(HM, "MapTypeCmb", 1);
            let dir = app.ini.dir(super::ini::DIR_MAPS);
            let mut i = 1;
            while find_height_map(&dir, i).is_some() {
                i += 1;
            }
            ui.set_position(HM, "MapNumberUpDown", i as i64);
            ui.set_text(HM, "MapNumberEdit", &i.to_string());
            hm_enable(app, ui);
        }
        "FormShow" => {
            ensure_view(ui, HM);
            app.meshview.dirty[1] = true;
        }
        "FormDblClick" => {
            let v = !ui.visible(HM, "NavigatePnl");
            ui.set_visible(HM, "NavigatePnl", v);
            app.meshview.dirty[1] = true;
        }
        "NavigatePnlClick" => {
            ui.set_visible(HM, "NavigatePnl", false);
            app.meshview.dirty[1] = true;
        }
        "FormMouseDown" | "FormMouseMove" | "FormMouseUp" | "FormMouseWheelDown" | "FormMouseWheelUp" => {
            let st = &mut app.meshview;
            if navigate(&mut st.hm_view, &mut st.hm_drag, e) {
                st.dirty[1] = true;
            }
        }
        "ResetBtnClick" => {
            let v = &mut app.meshview.hm_view;
            v.pos = [0.0; 3];
            v.angle = [0.0; 2];
            v.scale = 0.5;
            app.meshview.dirty[1] = true;
        }
        "LoadMeshBtnClick" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: "Wavefront OBJ (*.obj)|*.obj".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_MESHES)),
                ..Default::default()
            };
            ui.open_dialog("heightmap:load", &o);
        }
        "SaveImgBtnClick" => {
            let ext = hm_ext(ui);
            let o = crate::vcl::dialogs::FileOptions {
                filter: if ext == "png" { "Portable Network Graphic|*.png".into() } else { "Portable Gray Map|*.pgm".into() },
                default_ext: ext.into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_MAPS)),
                ..Default::default()
            };
            ui.save_dialog("heightmap:save", &o);
        }
        "SaveMapBtnClick" => {
            let nr = pti(&ui.text(HM, "MapNumberEdit")).max(1);
            let dir = app.ini.dir(super::ini::DIR_MAPS);
            let p = dir.join(format!("{nr}.{}", hm_ext(ui)));
            if p.exists() || find_height_map(&dir, nr).is_some() {
                app.meshview.pending_save = Some(p);
                ui.confirm("heightmap:overwrite", "This file already exists. Do you really want to overwrite it?");
            } else if let Err(e) = save_height_map(app, &p) {
                ui.show_message(&e);
            }
        }
        _ => {}
    }
}

pub fn heightmap_dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("load", DialogResult::File(Some(p))) => {
            let r = std::fs::read_to_string(p).map_err(|e| e.to_string()).and_then(|t| read_obj(&t));
            match r {
                Ok(mut m) => {
                    // DoCenter(2.0), DoScale(1, -1, 1)
                    m.center(2.0);
                    for v in m.vertices.iter_mut() {
                        v[1] = -v[1];
                    }
                    app.meshview.hm_mesh = GlMesh::new(&m);
                    app.meshview.dirty[1] = true;
                    hm_enable(app, ui);
                }
                Err(e) => ui.show_message(&format!("{}: {e}", p.display())),
            }
        }
        ("save", DialogResult::File(Some(p))) => {
            if let Err(e) = save_height_map(app, p) {
                ui.show_message(&e);
            }
        }
        ("overwrite", DialogResult::Button(crate::vcl::form::MR_YES)) => {
            if let Some(p) = app.meshview.pending_save.take() {
                if let Err(e) = save_height_map(app, &p) {
                    ui.show_message(&e);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obj_and_render() {
        let m = read_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 2 3\nf 1 3 4 2\n").unwrap();
        assert_eq!(m.faces.len(), 3);
        let g = GlMesh::new(&m);
        let v = View { pos: [0.0; 3], angle: [20.0, 10.0], scale: 1.0, fov: 30.0, scale_distance: true };
        let f = render(&g, &v, Style::Smooth, &Appearance::default(), 64, 48);
        assert!(f.depth.iter().any(|&d| d < 1.0));
    }
}
