//! Animation templates (not in MB3D): keyframes made from one view.
//!
//! * The turntable's pivot ([`turntable_pivot`], [`orbit_keyframes_around`]):
//!   the camera goes once around a point of the view.  The scene middle is
//!   that point where it lies at or behind the surface the middle of the
//!   picture shows (a whole object seen from outside: the middle is its
//!   centre); a middle in the empty space before that surface (after a
//!   flight in the Navigator, for example) would make the camera swing
//!   around nothing, so the point of the surface is taken instead.
//! * A fly-through ([`fly_through`]): the camera flies forward through the
//!   fractal as the Navigator's walk does (each step a part of the
//!   distance estimate around the camera, the zoom following it, so the
//!   details keep their sharpness the deeper it goes) and steers away from
//!   what comes in its way: ahead of it the free distance along a few dozen
//!   directions is traced, and the camera turns, at most a few degrees a
//!   frame, towards the freest of them that is near its course.  It stops
//!   when no direction is free any more (a dead end), when the double
//!   precision of the positions is used up, or when it leaves the fractal.
//!
//! The camera of a scene here is the Navigator's (`TransformNHeader`): the
//! start plane's middle, `mid + view * (z_start - mid z)`; keyframes are
//! given back with the middle in front of it again (`ModRotPoint`) as the
//! Navigator's views are.

use crate::anim::Keyframe;
use crate::calc::{de_at, CalcParams};
use crate::math::{add, dot, len, normalize, scale, sub, Vec3};
use crate::scene::Scene;
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// the camera (shared with the Navigator)

/// `Sqrt(width² + height²)` of the picture.
pub fn length_of_size(sc: &Scene) -> f64 {
    ((sc.width as f64).powi(2) + (sc.height as f64).powi(2)).sqrt()
}

/// The unit rows of the view matrix: right, down, forward.
pub fn unit_rows(sc: &Scene) -> [Vec3; 3] {
    [normalize(sc.vgrads[0]), normalize(sc.vgrads[1]), normalize(sc.vgrads[2])]
}

/// The Navigator's camera position: the start plane's middle.
pub fn cam_pos(sc: &Scene) -> Vec3 {
    let vz = normalize(sc.vgrads[2]);
    let d = sc.z_start - sc.mid[2];
    [sc.mid[0] + vz[0] * d, sc.mid[1] + vz[1] * d, sc.mid[2] + vz[2] * d]
}

/// `GetLocalAbsoluteDE` with calculation values `p` of the scene (the
/// distance estimate in absolute units does not depend on the camera): the
/// median of the distance estimates around the camera; None when it cannot
/// be estimated.
pub fn local_de_with(p: &CalcParams, sc: &Scene) -> Option<f64> {
    let sw = sc.step_width();
    let u = unit_rows(sc);
    let ct = cam_pos(sc);
    let mut list = Vec::new();
    let mut dmul = 1.0;
    while list.len() <= 3 && dmul <= 40.0 {
        for i in 0..9 {
            let pos = if i == 8 {
                ct
            } else {
                let o = [((i & 1) * 32) as f64 - 16.0, ((i & 2) * 16) as f64 - 16.0, ((i & 4) * 8) as f64 - 16.0];
                let mut q = ct;
                for k in 0..3 {
                    for a in 0..3 {
                        q[a] += u[k][a] * o[k] * dmul * sw;
                    }
                }
                q
            };
            let (d, _) = de_at(p, pos);
            if d.is_finite() && d > 0.0 && d < 1e15 * sw && list.len() < 11 {
                list.push(d / sw);
            }
        }
        dmul *= 2.0;
    }
    if list.is_empty() {
        return None;
    }
    list.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let m = list[((list.len() - 1) as f64 * 0.6).round() as usize];
    let r = m.clamp(3.0, 20000.0) * sw;
    Some(r.min(len(&ct).max(2.0)))
}

/// [`local_de_with`] with the scene's own calculation values.
pub fn local_de(sc: &Scene) -> Option<f64> {
    let mut s = sc.clone();
    s.slice_2d = 0;
    let p = CalcParams::new(&s).ok()?;
    local_de_with(&p, &s)
}

/// `TransformNHeader`'s move: the middle onto the camera.
pub fn camera_on_middle(sc: &mut Scene) {
    let c = cam_pos(sc);
    sc.z_end = sc.z_end - sc.z_start + c[2];
    sc.z_start = c[2];
    sc.mid = c;
}

/// `ModRotPoint`: the middle in front of the camera (for the main window's
/// rotations and the keyframes).
pub fn mod_rot_point(sc: &mut Scene) {
    let sw = sc.step_width();
    let ds = sc.z_start - sc.mid[2];
    let de = sc.z_end - sc.mid[2];
    let l = ((sc.z_end - sc.mid[2]) / sw).min(length_of_size(sc) * 1.5);
    let vz = normalize(sc.vgrads[2]);
    for k in 0..3 {
        sc.mid[k] += vz[k] * sw * l;
    }
    sc.z_start = sc.mid[2] + ds - sw * l;
    sc.z_end = sc.mid[2] + de - sw * l;
}

/// Moves the middle by `t`, the start and end planes with it (they lie at
/// `z_start - mid z` and `z_end - mid z`).
fn move_middle(sc: &mut Scene, t: Vec3) {
    sc.mid = add(sc.mid, t);
    sc.z_start += t[2];
    sc.z_end += t[2];
}

pub(crate) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Rodrigues: `v` turned by `angle` around the unit axis `u`.
pub(crate) fn turn(v: Vec3, u: Vec3, angle: f64) -> Vec3 {
    let (s, c) = angle.sin_cos();
    let d = dot(&u, &v);
    let x = cross(u, v);
    [0, 1, 2].map(|k| v[k] * c + x[k] * s + u[k] * d * (1.0 - c))
}

/// Sphere tracing from `from` along the unit `dir`: the free distance up to
/// `reach` (`reach` when nothing is in the way), with the distance
/// estimate taken at a safe part (`step` of it) and a surface at `stop`.
///
/// The distance estimate is not a bound: MB3D marches with the scene's ray
/// step (`z_step_div`, 0.5 by default) times it, and a whole one can jump
/// over a surface (from 5 units away the Integer Power bulb's estimate is
/// 8: the trace passed through it, found nothing ahead, and the orbit's
/// automatic pivot fell back to an empty middle). So the steps are taken
/// at `step` times the scene's ray step, and as many more of them allowed.
pub(crate) fn free_distance(p: &CalcParams, from: Vec3, dir: Vec3, reach: f64, stop: f64, step: f64, max_steps: usize) -> f64 {
    let ray = (p.z_step_div as f64).clamp(0.05, 1.0);
    let step = step * ray;
    let max_steps = (max_steps as f64 / ray).ceil() as usize;
    let mut t = 0.0;
    for _ in 0..max_steps {
        let (d, _) = de_at(p, add(from, scale(dir, t)));
        if !d.is_finite() {
            return reach;
        }
        if d <= stop {
            return t;
        }
        t += d * step;
        if t >= reach {
            return reach;
        }
    }
    t.min(reach)
}

// ---------------------------------------------------------------------------
// the turntable

/// The point the turntable turns around, as an offset from the scene
/// middle (zero: the middle, as before; see the module notes).
pub fn turntable_pivot(sc: &Scene) -> Vec3 {
    let mut s = sc.clone();
    s.slice_2d = 0;
    let Ok(p) = CalcParams::new(&s) else { return [0.0; 3] };
    let cam = cam_pos(&s);
    let f = unit_rows(&s)[2];
    // the middle's depth along the view (it lies on the middle ray)
    let mid_depth = dot(&sub(s.mid, cam), &f);
    let reach = (s.z_end - s.z_start).abs().max(mid_depth.abs() * 4.0).max(1e-9);
    let stop = (s.de_stop.max(0.01) * s.step_width()).max(reach * 1e-12);
    let hit = free_distance(&p, cam, f, reach, stop, 0.9, 4000);
    if hit >= reach || hit <= mid_depth {
        // nothing ahead, or the middle at or behind the surface
        return [0.0; 3];
    }
    scale(f, hit - mid_depth)
}

/// The axis a 360 degree orbit turns around (through its pivot).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbitAxis {
    /// the view's vertical axis: around the object from left to right (the
    /// Turntable's)
    Yaw,
    /// the view's horizontal axis: over the top and under the bottom
    Pitch,
    /// the line of sight: the picture spins
    Roll,
    /// yaw, pitch and roll at once, each a whole turn: a tumble
    All,
    /// the fractal's own x, y or z axis
    WorldX,
    WorldY,
    WorldZ,
}

impl OrbitAxis {
    pub const ALL: [OrbitAxis; 7] = [OrbitAxis::Yaw, OrbitAxis::Pitch, OrbitAxis::Roll, OrbitAxis::All, OrbitAxis::WorldX, OrbitAxis::WorldY, OrbitAxis::WorldZ];

    pub fn parse(s: &str) -> Result<OrbitAxis, String> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "yaw" | "horizontal" | "h" => OrbitAxis::Yaw,
            "pitch" | "vertical" | "v" => OrbitAxis::Pitch,
            "roll" => OrbitAxis::Roll,
            "all" | "tumble" => OrbitAxis::All,
            "x" => OrbitAxis::WorldX,
            "y" => OrbitAxis::WorldY,
            "z" => OrbitAxis::WorldZ,
            o => return Err(format!("orbit axis '{o}' (yaw, pitch, roll, all, x, y or z)")),
        })
    }

    /// The rotation by `a` radians of this orbit, for the view `sc` it
    /// starts from (the view's axes as they are there).
    fn turn(self, sc: &Scene, v: Vec3, a: f64) -> Vec3 {
        let [r, u, f] = unit_rows(sc);
        match self {
            OrbitAxis::Yaw => turn(v, u, a),
            OrbitAxis::Pitch => turn(v, r, a),
            OrbitAxis::Roll => turn(v, f, a),
            // (about fixed axes, one after the other: back where it began
            // after the whole turn)
            OrbitAxis::All => turn(turn(turn(v, f, a), r, a), u, a),
            OrbitAxis::WorldX => turn(v, [1.0, 0.0, 0.0], a),
            OrbitAxis::WorldY => turn(v, [0.0, 1.0, 0.0], a),
            OrbitAxis::WorldZ => turn(v, [0.0, 0.0, 1.0], a),
        }
    }
}

/// The turntable around `pivot` (an offset from the middle): `steps`
/// keyframes of `sc` turned around the view's vertical axis through it in
/// equal steps, a whole turn, `frames` frames from each to the next (as a
/// loop the last leads back to the first).  With a zero pivot this is
/// [`crate::anim::orbit_keyframes`].
pub fn orbit_keyframes_around(sc: &Scene, steps: usize, frames: u32, pivot: Vec3) -> Vec<Keyframe> {
    orbit_keyframes_axis(sc, steps, frames, pivot, OrbitAxis::Yaw)
}

/// The middle moved `s` along the line of sight with the camera where it
/// is (the start and end planes keep their distance from the camera): a
/// pivot on the middle ray becomes the middle.
fn centre_on_sight(sc: &mut Scene, s: f64) {
    let f = unit_rows(sc)[2];
    sc.mid = add(sc.mid, scale(f, s));
    sc.z_start += f[2] * s - s;
    sc.z_end += f[2] * s - s;
}

/// [`orbit_keyframes_around`] around `axis`: the view and the camera
/// turned together around the pivot, so the camera keeps its distance and
/// its look at the pivot.
///
/// A pivot on the middle ray (as [`turntable_pivot`] gives) first becomes
/// the middle, the camera staying where it is; then the keyframes differ
/// in their rotation alone, as the Turntable's, and every interpolation
/// keeps the camera at the same distance from the pivot (the rotation is
/// interpolated on the unit sphere). Moving the middle around the pivot
/// in each keyframe left the camera's position to the interpolation of
/// the middles, which cuts the circle short between the keyframes (Catmull-
/// Rom: 0.85 % of its radius with 8 keyframes, 12 % with 4, 31 % with 3;
/// Linear: a polygon). A pivot off that ray is still turned that way.
pub fn orbit_keyframes_axis(sc: &Scene, steps: usize, frames: u32, pivot: Vec3, axis: OrbitAxis) -> Vec<Keyframe> {
    let steps = steps.max(3);
    let f = unit_rows(sc)[2];
    let along = dot(&pivot, &f);
    let mut base = sc.clone();
    let pivot = if len(&sub(pivot, scale(f, along))) <= 1e-9 * len(&pivot).max(sc.step_width()) {
        centre_on_sight(&mut base, along);
        [0.0; 3]
    } else {
        pivot
    };
    let centre = add(base.mid, pivot);
    (0..steps)
        .map(|i| {
            let a = 2.0 * PI * i as f64 / steps as f64;
            let mut k = base.clone();
            k.vgrads = base.vgrads.map(|r| axis.turn(&base, r, a));
            if pivot != [0.0; 3] {
                // the middle turned around the pivot as the view
                let m = add(centre, axis.turn(&base, sub(base.mid, centre), a));
                move_middle(&mut k, sub(m, base.mid));
            }
            Keyframe::new(k, frames)
        })
        .collect()
}

/// `total` frames spread over `parts` keyframe gaps as evenly as whole
/// frames allow (their sum is `total`).
pub fn spread_frames(total: u32, parts: usize) -> Vec<u32> {
    let parts = parts.max(1) as u64;
    (0..parts).map(|i| (((i + 1) * total as u64) / parts - (i * total as u64) / parts) as u32).collect()
}

/// A turntable of `seconds` at `fps` frames a second: `steps` keyframes
/// around [`turntable_pivot`], their frames adding up to the duration (a
/// loop: the last keyframe's frames lead back to the first).
pub fn turntable(sc: &Scene, steps: usize, seconds: f64, fps: f64) -> Vec<Keyframe> {
    turntable_axis(sc, steps, seconds, fps, OrbitAxis::Yaw)
}

/// [`turntable`] around `axis`.
pub fn turntable_axis(sc: &Scene, steps: usize, seconds: f64, fps: f64, axis: OrbitAxis) -> Vec<Keyframe> {
    let steps = steps.max(3);
    let total = (seconds * fps).round().max(steps as f64) as u32;
    let mut ks = orbit_keyframes_axis(sc, steps, 1, turntable_pivot(sc), axis);
    for (k, f) in ks.iter_mut().zip(spread_frames(total, steps)) {
        k.frames = f;
    }
    ks
}

// ---------------------------------------------------------------------------
// the fly-through

/// Settings of a fly-through.
#[derive(Clone, Copy, Debug)]
pub struct FlightParams {
    /// Frames of the whole flight (at most; it may end before).
    pub frames: usize,
    /// A keyframe every this many frames.
    pub keyframe_every: usize,
    /// The step of a frame, a part of the distance estimate around the
    /// camera (the Navigator's walk at speed 10 moves 0.1 of it).
    pub speed: f64,
    /// The largest turn of a frame, in degrees.
    pub max_turn_deg: f64,
    /// How far ahead the way is traced, in distance estimates.
    pub look_ahead: f64,
    /// End the flight when the view has become only sky or only fractal
    /// (checked at every keyframe; `frames` stays the longest flight).
    pub until_view: bool,
}

impl Default for FlightParams {
    fn default() -> Self {
        FlightParams { frames: 250, keyframe_every: 12, speed: 0.04, max_turn_deg: 2.0, look_ahead: 8.0, until_view: false }
    }
}

/// Why a fly-through ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlightEnd {
    /// All its frames were flown.
    Done,
    /// No direction ahead was free any more.
    DeadEnd,
    /// The steps fell under what double precision resolves at the camera.
    Precision,
    /// The camera left the fractal (the distance to it kept growing).
    LeftFractal,
    /// The view showed only sky ([`FlightParams::until_view`]).
    OnlySky,
    /// The view was filled with the fractal ([`FlightParams::until_view`]).
    OnlyFractal,
}

/// Why a flight ended, in English (the command line).
pub fn flight_end_text(e: FlightEnd) -> &'static str {
    match e {
        FlightEnd::Done => "all frames flown",
        FlightEnd::DeadEnd => "no way ahead was left",
        FlightEnd::Precision => "the steps came to the limit of double precision",
        FlightEnd::LeftFractal => "the camera left the fractal",
        FlightEnd::OnlySky => "the view shows only sky",
        FlightEnd::OnlyFractal => "the view is filled with the fractal",
    }
}

/// A fly-through's keyframes and how it went.
#[derive(Clone, Debug)]
pub struct Flight {
    pub keyframes: Vec<Keyframe>,
    /// Frames flown (the animation's frame count).
    pub frames: usize,
    pub end: FlightEnd,
}

/// The part of the view `sc` shows of the fractal: rays of a grid of 24 by
/// 16 of its pixels traced to the far plane (0: only sky, 1: filled).
pub fn view_coverage(sc: &Scene) -> f64 {
    let mut s = sc.clone();
    s.slice_2d = 0;
    let Ok(p) = CalcParams::new(&s) else { return 0.5 };
    let sw = s.step_width();
    let far = ((s.z_end - s.z_start).abs() * 2.0).max(sw * 10.0);
    let stop = (s.de_stop.max(0.01) as f64 * sw).max(far * 1e-12);
    let (gx, gy) = (24, 16);
    let mut hits = 0;
    for j in 0..gy {
        for i in 0..gx {
            let x = ((i as f64 + 0.5) * s.width as f64 / gx as f64) as i32;
            let y = ((j as f64 + 0.5) * s.height as f64 / gy as f64) as i32;
            let (o, d) = crate::calc::pixel_ray(&p, x, y);
            let d = normalize(d);
            if free_distance(&p, o, d, far, stop, 0.9, 400) < far * 0.999 {
                hits += 1;
            }
        }
    }
    hits as f64 / (gx * gy) as f64
}

/// Only sky (0) or only fractal (1) in the view, else None.
fn pure_view(c: f64) -> Option<bool> {
    if c <= 0.003 {
        Some(false)
    } else if c >= 0.997 {
        Some(true)
    } else {
        None
    }
}

/// The end of a flight that goes on until the view is pure: a pure view
/// other than the one it started with.
fn view_end(c: f64, start: Option<bool>) -> Option<FlightEnd> {
    match pure_view(c) {
        Some(v) if start != Some(v) => Some(if v { FlightEnd::OnlyFractal } else { FlightEnd::OnlySky }),
        _ => None,
    }
}

/// The directions traced ahead: the course, then rings around it.
fn probe_dirs(r: Vec3, u: Vec3, f: Vec3) -> Vec<Vec3> {
    let mut v = vec![f];
    for (ring, &deg) in [12.0f64, 25.0, 45.0, 70.0].iter().enumerate() {
        let th = deg.to_radians();
        let (st, ct) = th.sin_cos();
        let n = 8;
        for j in 0..n {
            // every second ring turned by half a step: no gaps in line
            let ph = 2.0 * PI * (j as f64 + 0.5 * (ring % 2) as f64) / n as f64;
            let (sp, cp) = ph.sin_cos();
            v.push(normalize([0, 1, 2].map(|k| f[k] * ct + (r[k] * cp + u[k] * sp) * st)));
        }
    }
    v
}

/// Frames ahead the camera looks for what it would hit (at its speed):
/// nearer than that it turns, farther it flies straight on.
const REACT_FRAMES: f64 = 50.0;
/// Frames after a turn in which a turn the other way is far less wanted.
const HOLD_FRAMES: usize = 75;
/// The approach ends where the distance to the fractal is this part of
/// the distance at the start.
const APPROACH: f64 = 0.3;
/// Frames heading out into the empty space (turning back all the while)
/// after which the flight has left the fractal.
const LEAVE_FRAMES: usize = 90;
/// Frames between two looks at the way under way.
const RECHECK_FRAMES: usize = 6;
/// Frames a turn takes to reach its full rate (and to stop again).
const EASE_FRAMES: f64 = 12.0;

/// The steering of a flight: the way it turns to (kept until the heading
/// is on it and nothing is in the way), its turn (axis times radians a
/// frame, eased) and the side of the last turn and the frame it ended.
#[derive(Default)]
struct Steering {
    target: Option<Vec3>,
    /// the side of the turn under way (across the heading it began from)
    side: Vec3,
    /// the frame from which the way under way is looked at again
    check_at: usize,
    w: Vec3,
    last_side: Option<(Vec3, usize)>,
}

/// The part of `d` across the heading `f`, as a unit vector (zero when
/// along it).
fn sideways(d: Vec3, f: Vec3) -> Vec3 {
    let x = sub(d, scale(f, dot(&d, &f)));
    let l = len(&x);
    if l < 1e-9 {
        [0.0; 3]
    } else {
        scale(x, 1.0 / l)
    }
}

/// The camera at frame `i` as a keyframe (the middle in front of it).
fn flight_keyframe(cam: &Scene) -> Scene {
    let mut k = cam.clone();
    mod_rot_point(&mut k);
    k
}

/// A fly-through of `sc` (see the module notes): keyframes every
/// `fp.keyframe_every` frames of the flight and its last frame, for a
/// Catmull-Rom animation that is not looped.  `progress(done, total)` is
/// called every frame; `cancel` stops it (the keyframes so far are given).
pub fn fly_through(sc: &Scene, fp: &FlightParams, progress: &dyn Fn(usize, usize), cancel: &dyn Fn() -> bool) -> Result<Flight, String> {
    let mut s = sc.clone();
    s.slice_2d = 0;
    s.stereo_mode = 0;
    let p = CalcParams::new(&s)?;
    camera_on_middle(&mut s);
    let de0 = local_de_with(&p, &s).ok_or("no distance to the fractal can be estimated at the camera")?;
    let ahead0 = (s.z_end - s.z_start) / de0;
    let far = if ahead0.is_finite() { ahead0.clamp(1.0, 10000.0) } else { 50.0 };
    let frames = fp.frames.max(2);
    let every = fp.keyframe_every.max(1);
    let max_turn = fp.max_turn_deg.to_radians().max(1e-4);
    let mut keys: Vec<Scene> = vec![flight_keyframe(&s)];
    // the view at the start (a view that starts as only sky or only
    // fractal ends the flight when it becomes the other)
    let start_view = if fp.until_view { pure_view(view_coverage(&keys[0])) } else { None };
    let mut key_frame: Vec<usize> = vec![0];
    let (mut blocked, mut outside) = (0, 0);
    let mut steer = Steering::default();
    let mut end = FlightEnd::Done;
    let mut flown = 1;
    let mut de = de0;
    for i in 1..frames {
        if cancel() {
            break;
        }
        progress(i, frames);
        let cam = s.mid;
        let [r, u, f] = unit_rows(&s);
        let reach = fp.look_ahead * de;
        let stop = (de * 0.05).max(len(&cam) * 1e-13);
        let free: Vec<(Vec3, f64)> = probe_dirs(r, u, f).into_iter().map(|d| (d, free_distance(&p, cam, d, reach, stop, 0.8, 48))).collect();
        let fwd = free[0].1;
        // a dead end: no direction free beyond the distance estimate
        let most_free = free.iter().map(|x| x.1).fold(0.0, f64::max);
        if most_free < 1.5 * de.min(reach) {
            blocked += 1;
            if blocked >= 3 {
                end = FlightEnd::DeadEnd;
                break;
            }
        } else {
            blocked = 0;
        }
        // the course (see `Steering`): straight on while nothing is near
        // ahead; a turn only when the camera would hit something within
        // `REACT_FRAMES` frames or head out into the empty space around the
        // fractal, and then to one chosen way, kept until it is free
        let need = (2.5 * de).min(reach);
        let out = fp.look_ahead * 5.0 * de;
        let is_void = |d: Vec3, l: f64| l >= reach * 0.999 && free_distance(&p, add(cam, scale(d, l)), d, out, stop, 0.9, 64) >= out * 0.999;
        let fwd_void = is_void(f, fwd);
        let clear = (REACT_FRAMES * fp.speed * de).max(2.0 * de).min(reach * 0.999);
        // (the approach: from a view of the object from outside the camera
        // flies straight at it, zooming in, until it is in among it, at a
        // part of the distance it started from)
        let approach = de > de0 * APPROACH;
        let danger = (fwd < clear && !approach) || fwd_void;
        // the ways to choose from: free for a few distance estimates (the
        // longer the better: a gap leading into the fractal), near the
        // heading, not ending at a wall nor leading out into the empty
        // space. `near`: a way near this one wanted (the way under way:
        // no swinging between two), `avoid_side`: a turn to the other side
        // of this one far less wanted (shortly after a turn)
        let choose = |near: Option<Vec3>, avoid_side: Option<Vec3>| -> Option<Vec3> {
            free.iter()
                .skip(1)
                .map(|&(d, l)| {
                    let open = (l / reach).min(1.0).sqrt() * (l / need).min(1.0);
                    let (de_end, _) = de_at(&p, add(cam, scale(d, l.min(need))));
                    let c = (de_end.max(1e-300) / de).ln().min(0.0);
                    let wall = 1.0 / (1.0 + c * c);
                    let void = if is_void(d, l) { 1e-3 } else { 1.0 };
                    let keep = near.map_or(1.0, |t| ((1.0 + dot(&d, &t)) * 0.5).powi(4));
                    let side = match avoid_side {
                        Some(sd) if dot(&sideways(d, f), &sd) < -0.2 => 0.05,
                        _ => 1.0,
                    };
                    (d, open * open * wall * void * keep * side * (0.5 + 0.5 * dot(&d, &f)))
                })
                .filter(|x| x.1 > 0.0)
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|x| x.0)
        };
        // (back from the empty space to the fractal is no swing: allowed
        // to either side)
        // heading out into the empty space with no way back in sight: back
        // towards the nearest surface (against the distance estimate's
        // gradient) at a slant, keeping on the way it goes, a glide along
        // the object rather than a turn back
        let back_in = || -> Option<Vec3> {
            let h = de * 0.05;
            let g = [0, 1, 2].map(|k| {
                let mut a = cam;
                let mut b = cam;
                a[k] += h;
                b[k] -= h;
                de_at(&p, a).0 - de_at(&p, b).0
            });
            let lg = len(&g);
            if !(lg > 0.0) || !lg.is_finite() {
                return None;
            }
            let toward = scale(g, -1.0 / lg);
            let along = sideways(f, toward);
            Some(normalize(add(along, scale(toward, 0.6))))
        };
        let choose_in = |near: Option<Vec3>, avoid_side: Option<Vec3>| -> Option<Vec3> {
            let c = choose(near, avoid_side);
            match c {
                Some(d) if fwd_void && is_void(d, free_distance(&p, cam, d, reach, stop, 0.8, 48)) => back_in().or(c),
                None if fwd_void => back_in(),
                _ => c,
            }
        };
        let recent_side = steer.last_side.filter(|(_, at)| i < at + HOLD_FRAMES && !fwd_void).map(|x| x.0);
        match steer.target {
            None if danger => {
                steer.target = choose_in(None, recent_side);
                if let Some(t) = steer.target {
                    steer.side = sideways(t, f);
                }
                steer.check_at = i + RECHECK_FRAMES;
            }
            Some(t) if i >= steer.check_at => {
                let left = dot(&f, &t).clamp(-1.0, 1.0).acos();
                let t_free = free_distance(&p, cam, t, reach, stop, 0.8, 48);
                if t_free < 0.5 * need.min(clear) || is_void(t, t_free) || (left < 0.5f64.to_radians() && danger) {
                    // the way taken closed, or reached and still in the
                    // way of something: the nearest free one to it
                    steer.target = choose_in(Some(t), Some(steer.side)).or(Some(t));
                    steer.check_at = i + RECHECK_FRAMES;
                } else if left < 0.5f64.to_radians() {
                    steer.last_side = Some((steer.side, i));
                    steer.target = None;
                }
            }
            _ => {}
        }
        // out in the empty space for a while (a second at 25 frames):
        // the flight has left the fractal
        if fwd_void {
            outside += 1;
            if outside >= LEAVE_FRAMES {
                end = FlightEnd::LeftFractal;
                break;
            }
        } else {
            outside = 0;
        }
        // the turn: its rate eased in and out (at most `max_turn` a frame,
        // reached in `EASE_FRAMES`, slowing as the heading nears the way)
        let wd = match steer.target {
            Some(t) => {
                let ax = cross(f, t);
                let sa = len(&ax);
                if sa < 1e-12 {
                    [0.0; 3]
                } else {
                    let left = sa.atan2(dot(&f, &t));
                    scale(ax, (left * 0.12).min(max_turn) / sa)
                }
            }
            None => [0.0; 3],
        };
        let dw = sub(wd, steer.w);
        let acc = max_turn / EASE_FRAMES;
        let ldw = len(&dw);
        steer.w = if ldw > acc { add(steer.w, scale(dw, acc / ldw)) } else { wd };
        let rate = len(&steer.w);
        if rate > 1e-12 {
            let ax = scale(steer.w, 1.0 / rate);
            s.vgrads = s.vgrads.map(|row| turn(row, ax, rate));
        }
        // the step: slower where the way ahead is short, never more than
        // half of it
        let f = unit_rows(&s)[2];
        let fwd_now = free_distance(&p, cam, f, reach, stop, 0.8, 48);
        let slow = ((fwd_now - 0.5 * de) / reach).clamp(0.1, 1.0);
        let step = (fp.speed * de * slow).min(0.5 * fwd_now);
        if step <= len(&cam).max(1.0) * 1e-14 {
            end = FlightEnd::Precision;
            break;
        }
        let dz = f[2] * step;
        s.mid = add(cam, scale(f, step));
        s.z_start += dz;
        s.z_end += dz;
        // the zoom from the local distance estimate (`SetZoom`), the far
        // plane as many of them away as at the start
        let Some(nd) = local_de_with(&p, &s) else {
            end = FlightEnd::Precision;
            break;
        };
        de = nd;
        s.zoom = length_of_size(&s) * 2.0 / (de * s.width as f64);
        s.z_end = s.z_start + (de * far).min(9999.0);
        flown = i + 1;
        if i % every == 0 {
            let k = flight_keyframe(&s);
            if fp.until_view {
                if let Some(e) = view_end(view_coverage(&k), start_view) {
                    keys.push(k);
                    key_frame.push(i);
                    end = e;
                    break;
                }
            }
            keys.push(k);
            key_frame.push(i);
        }
        if de > de0 * 50.0 && fwd >= reach * 0.999 {
            end = FlightEnd::LeftFractal;
            break;
        }
    }
    // the last frame flown is the last keyframe
    if *key_frame.last().unwrap() != flown - 1 {
        keys.push(flight_keyframe(&s));
        key_frame.push(flown - 1);
    }
    let n = keys.len();
    let keyframes = keys
        .into_iter()
        .enumerate()
        .map(|(k, sc)| {
            let frames = if k + 1 < n { (key_frame[k + 1] - key_frame[k]) as u32 } else { 1 };
            Keyframe::new(sc, frames)
        })
        .collect::<Vec<_>>();
    Ok(Flight { keyframes, frames: flown, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eye_abs(sc: &Scene) -> Vec3 {
        cam_pos(sc)
    }

    #[test]
    fn frames_are_spread_evenly() {
        assert_eq!(spread_frames(250, 8).iter().sum::<u32>(), 250);
        let v = spread_frames(250, 8);
        assert!(v.iter().all(|&f| f == 31 || f == 32), "{v:?}");
        assert_eq!(spread_frames(7, 7), vec![1; 7]);
    }

    #[test]
    fn a_zero_pivot_is_the_old_turntable() {
        let sc = Scene::preset("Integer Power").unwrap();
        let a = orbit_keyframes_around(&sc, 8, 10, [0.0; 3]);
        let b = crate::anim::orbit_keyframes(&sc, 8, 10);
        for (x, y) in a.iter().zip(&b) {
            for r in 0..3 {
                for k in 0..3 {
                    assert!((x.scene.vgrads[r][k] - y.scene.vgrads[r][k]).abs() < 1e-12);
                }
            }
            assert_eq!(x.scene.mid, y.scene.mid);
            assert_eq!(x.scene.z_start, y.scene.z_start);
        }
    }

    #[test]
    fn the_pivot_is_found_from_far_away() {
        // the camera 5 units from the bulb's centre (its distance estimate
        // there is 8, past the bulb), the middle 1 unit before the camera
        let mut sc = Scene::preset("Integer Power").unwrap();
        sc.z_start = sc.mid[2] - 5.0;
        let f = unit_rows(&sc)[2];
        let c = cam_pos(&sc);
        let a = -4.0;
        let (ds, de) = (sc.z_start - sc.mid[2], sc.z_end - sc.mid[2]);
        sc.mid = add(sc.mid, scale(f, a));
        sc.z_start = sc.mid[2] + ds - a;
        sc.z_end = sc.mid[2] + de - a;
        assert!(len(&sub(cam_pos(&sc), c)) < 1e-12);
        let at = add(sc.mid, turntable_pivot(&sc));
        let p = CalcParams::new(&sc).unwrap();
        assert!(de_at(&p, at).0 < 0.05, "the pivot on the surface");
        let d = len(&sub(at, c));
        assert!(d > 3.0 && d < 4.5, "the bulb's near side: {d}");
    }

    #[test]
    fn the_orbit_keeps_its_radius_between_keyframes() {
        use crate::anim::{Animation, Interpolation};
        let sc = Scene::preset("Integer Power").unwrap();
        let f = unit_rows(&sc)[2];
        let cam0 = cam_pos(&sc);
        for along in [-0.7, 0.4] {
            let pivot = add(sc.mid, scale(f, along));
            let r0 = len(&sub(cam0, pivot));
            for axis in OrbitAxis::ALL {
                for steps in [3, 4, 8] {
                    let ks = orbit_keyframes_axis(&sc, steps, 10, scale(f, along), axis);
                    // the first keyframe is the view as it was
                    assert!(len(&sub(cam_pos(&ks[0].scene), cam0)) < 1e-12 * r0.max(1.0));
                    for ipol in [Interpolation::Linear, Interpolation::Bezier, Interpolation::CatmullRom] {
                        let a = Animation { keyframes: ks.clone(), looped: true, interpolation: ipol, ..Default::default() };
                        for fr in 0..a.frame_count() {
                            let s = a.frame_scene(fr).unwrap();
                            let r = len(&sub(cam_pos(&s), pivot));
                            assert!((r - r0).abs() < 1e-9 * r0, "{axis:?} {steps} {ipol:?} frame {fr}: {r} vs {r0}");
                            assert!(len(&sub(s.mid, pivot)) < 1e-12 * r0.max(1.0));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_camera_circles_the_pivot() {
        let sc = Scene::preset("Integer Power").unwrap();
        let pivot = scale(unit_rows(&sc)[2], -0.7);
        let centre = add(sc.mid, pivot);
        let r0 = len(&sub(eye_abs(&sc), centre));
        let ks = orbit_keyframes_around(&sc, 8, 10, pivot);
        let u = unit_rows(&sc)[1];
        for (i, k) in ks.iter().enumerate() {
            let e = eye_abs(&k.scene);
            let v = sub(e, centre);
            // the same distance, in the plane through the centre
            assert!((len(&v) - r0).abs() < 1e-9 * r0.max(1.0), "{i}");
            assert!((dot(&v, &u) - dot(&sub(eye_abs(&sc), centre), &u)).abs() < 1e-9, "{i}");
            // still looking at the centre as the first view did
            let f = unit_rows(&k.scene)[2];
            let c0 = dot(&normalize(sub(centre, eye_abs(&sc))), &unit_rows(&sc)[2]);
            assert!((dot(&normalize(sub(centre, e)), &f) - c0).abs() < 1e-9, "{i}");
        }
        // half way round: the camera on the other side
        let e4 = eye_abs(&ks[4].scene);
        let a = sub(eye_abs(&sc), centre);
        let b = sub(e4, centre);
        let ah = sub(a, scale(u, dot(&a, &u)));
        let bh = sub(b, scale(u, dot(&b, &u)));
        assert!((dot(&normalize(ah), &normalize(bh)) + 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_preset_turns_around_its_middle() {
        // a whole object seen from outside: its middle is behind the
        // surface, the turntable as before
        let sc = Scene::preset("Integer Power").unwrap();
        assert_eq!(turntable_pivot(&sc), [0.0; 3]);
        let ks = turntable(&sc, 8, 10.0, 25.0);
        assert_eq!(ks.iter().map(|k| k.frames).sum::<u32>(), 250);
    }

    #[test]
    fn a_middle_before_the_surface_turns_around_the_surface() {
        // the middle pulled towards the camera, into the empty space before
        // the bulb: the pivot is the surface point the middle ray meets
        let mut sc = Scene::preset("Integer Power").unwrap();
        let f = unit_rows(&sc)[2];
        let c = cam_pos(&sc);
        // (the camera kept: the start plane's distance from the middle
        // grows by what the middle moves towards it, as in ModRotPoint)
        let a = -dot(&sub(sc.mid, c), &f) * 0.9;
        let (ds, de) = (sc.z_start - sc.mid[2], sc.z_end - sc.mid[2]);
        sc.mid = add(sc.mid, scale(f, a));
        sc.z_start = sc.mid[2] + ds - a;
        sc.z_end = sc.mid[2] + de - a;
        assert!(len(&sub(cam_pos(&sc), c)) < 1e-12);
        let pv = turntable_pivot(&sc);
        assert!(len(&pv) > 0.0, "the middle before the surface");
        let at = add(sc.mid, pv);
        let p = CalcParams::new(&sc).unwrap();
        let (d, _) = de_at(&p, at);
        assert!(d < 0.05, "the pivot on the surface: {d}");
    }

    #[test]
    fn a_fly_through_moves_forward_and_closer() {
        let sc = Scene::preset("Integer Power").unwrap();
        let fp = FlightParams { frames: 60, keyframe_every: 12, ..Default::default() };
        let fl = fly_through(&sc, &fp, &|_, _| {}, &|| false).unwrap();
        assert!(fl.frames > 10, "{:?} after {}", fl.end, fl.frames);
        assert_eq!(fl.keyframes.iter().take(fl.keyframes.len() - 1).map(|k| k.frames as usize).sum::<usize>() + 1, fl.frames);
        let a = crate::anim::Animation { keyframes: fl.keyframes.clone(), interpolation: crate::anim::Interpolation::CatmullRom, ..Default::default() };
        assert_eq!(a.frame_count(), fl.frames);
        // the camera came nearer to the bulb (the distance estimate fell)
        let first = local_de(&fl.keyframes[0].scene).unwrap();
        let last = local_de(&fl.keyframes.last().unwrap().scene).unwrap();
        assert!(last < first, "{first} -> {last}");
        // never into the surface
        let p = CalcParams::new(&sc).unwrap();
        for k in &fl.keyframes {
            let (d, _) = de_at(&p, cam_pos(&k.scene));
            assert!(d > 0.0);
        }
    }

    #[test]
    fn the_view_of_the_preset_is_part_fractal() {
        let sc = Scene::preset("Integer Power").unwrap();
        let c = view_coverage(&sc);
        assert!(c > 0.05 && c < 0.95, "{c}");
        // turned away from the bulb: only sky
        let mut away = sc.clone();
        away.vgrads = sc.vgrads.map(|r| turn(r, unit_rows(&sc)[1], PI));
        let m = sub(cam_pos(&sc), sc.mid);
        // (the middle behind the camera now: moved to keep the camera)
        let (ds, de) = (sc.z_start - sc.mid[2], sc.z_end - sc.mid[2]);
        away.mid = add(cam_pos(&sc), m);
        away.z_start = away.mid[2] + ds;
        away.z_end = away.mid[2] + de;
        assert!(len(&sub(cam_pos(&away), cam_pos(&sc))) < 1e-9);
        assert_eq!(pure_view(view_coverage(&away)), Some(false));
    }

    #[test]
    fn a_flight_until_the_view_is_pure_ends_on_it() {
        let sc = Scene::preset("Integer Power").unwrap();
        let fp = FlightParams { frames: 3000, keyframe_every: 12, until_view: true, ..Default::default() };
        let fl = fly_through(&sc, &fp, &|_, _| {}, &|| false).unwrap();
        assert!(fl.frames > 12);
        if matches!(fl.end, FlightEnd::OnlySky | FlightEnd::OnlyFractal) {
            let c = view_coverage(&fl.keyframes.last().unwrap().scene);
            assert!(pure_view(c).is_some(), "{c}");
        }
    }

    #[test]
    fn every_orbit_axis_circles_the_pivot_and_comes_back() {
        let sc = Scene::preset("Integer Power").unwrap();
        let pivot = scale(unit_rows(&sc)[2], 0.3);
        let centre = add(sc.mid, pivot);
        let r0 = len(&sub(cam_pos(&sc), centre));
        for axis in OrbitAxis::ALL {
            let ks = orbit_keyframes_axis(&sc, 12, 10, pivot, axis);
            for (i, k) in ks.iter().enumerate() {
                let e = cam_pos(&k.scene);
                assert!((len(&sub(e, centre)) - r0).abs() < 1e-9, "{axis:?} {i}");
                // looking at the pivot as at the start
                let c0 = dot(&normalize(sub(centre, cam_pos(&sc))), &unit_rows(&sc)[2]);
                assert!((dot(&normalize(sub(centre, e)), &unit_rows(&k.scene)[2]) - c0).abs() < 1e-9, "{axis:?} {i}");
            }
            // a whole turn: the step after the last is the start again
            let last = axis.turn(&sc, sc.vgrads[2], 2.0 * PI);
            for k in 0..3 {
                assert!((last[k] - sc.vgrads[2][k]).abs() < 1e-9, "{axis:?}");
            }
        }
        // pitch: half way the camera is on the other side, upside down
        let ks = orbit_keyframes_axis(&sc, 12, 10, pivot, OrbitAxis::Pitch);
        let (u0, u6) = (unit_rows(&sc)[1], unit_rows(&ks[6].scene)[1]);
        assert!((dot(&u0, &u6) + 1.0).abs() < 1e-9);
        // roll: the camera stays on the line of sight, the picture turns
        let ks = orbit_keyframes_axis(&sc, 12, 10, pivot, OrbitAxis::Roll);
        assert!(len(&sub(cam_pos(&ks[3].scene), cam_pos(&sc))) < 1e-9);
        // yaw is the turntable's turn
        let a = orbit_keyframes_axis(&sc, 8, 10, [0.0; 3], OrbitAxis::Yaw);
        let b = crate::anim::orbit_keyframes(&sc, 8, 10);
        for (x, y) in a.iter().zip(&b) {
            for r in 0..3 {
                for k in 0..3 {
                    assert!((x.scene.vgrads[r][k] - y.scene.vgrads[r][k]).abs() < 1e-12);
                }
            }
        }
    }
}
#[cfg(test)]
mod steering_tests {
    use super::*;
    /// (turns, reversals): between keyframes the sideways turn (yaw, about
    /// the view's up) and the up-down one (pitch); a reversal is a turn the
    /// other way right after one.
    fn wobble(ks: &[Keyframe]) -> (usize, usize, f64) {
        let mut turns = 0;
        let mut rev = 0;
        let mut last: Option<(f64, f64)> = None;
        let mut total = 0.0;
        for w in ks.windows(2) {
            let [r, u, f0] = unit_rows(&w[0].scene);
            let f1 = unit_rows(&w[1].scene)[2];
            let (yaw, pitch) = (dot(&f1, &r), -dot(&f1, &u));
            let _ = f0;
            let m = (yaw * yaw + pitch * pitch).sqrt();
            total += m.asin().to_degrees();
            if m < 0.3f64.to_radians().sin() {
                continue;
            }
            turns += 1;
            if let Some((y, p)) = last {
                if y * yaw + p * pitch < 0.0 {
                    rev += 1;
                }
            }
            last = Some((yaw, pitch));
        }
        (turns, rev, total)
    }
    #[test]
    fn a_flight_keeps_its_course_and_does_not_swing() {
        // flights that once swung left and right 5 to 18 times in 30 s
        for name in ["Integer Power", "Amazing Box"] {
            let sc = Scene::preset(name).unwrap();
            let fp = FlightParams { frames: 750, keyframe_every: 12, ..Default::default() };
            let fl = fly_through(&sc, &fp, &|_, _| {}, &|| false).unwrap();
            assert_eq!(fl.end, FlightEnd::Done, "{name}");
            let (turns, rev, _) = wobble(&fl.keyframes);
            // most of the way straight on, and no swinging back and forth
            assert!(turns * 3 < fl.keyframes.len(), "{name}: turning in {turns} of {}", fl.keyframes.len());
            assert!(rev <= 2, "{name}: {rev} reversals");
        }
    }
}
