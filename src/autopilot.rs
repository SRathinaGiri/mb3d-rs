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

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// Rodrigues: `v` turned by `angle` around the unit axis `u`.
fn turn(v: Vec3, u: Vec3, angle: f64) -> Vec3 {
    let (s, c) = angle.sin_cos();
    let d = dot(&u, &v);
    let x = cross(u, v);
    [0, 1, 2].map(|k| v[k] * c + x[k] * s + u[k] * d * (1.0 - c))
}

/// Sphere tracing from `from` along the unit `dir`: the free distance up to
/// `reach` (`reach` when nothing is in the way), with the distance
/// estimate taken at a safe part (`step` of it) and a surface at `stop`.
fn free_distance(p: &CalcParams, from: Vec3, dir: Vec3, reach: f64, stop: f64, step: f64, max_steps: usize) -> f64 {
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

/// The turntable around `pivot` (an offset from the middle): `steps`
/// keyframes of `sc` turned around the view's vertical axis through it in
/// equal steps, a whole turn, `frames` frames from each to the next (as a
/// loop the last leads back to the first).  With a zero pivot this is
/// [`crate::anim::orbit_keyframes`].
pub fn orbit_keyframes_around(sc: &Scene, steps: usize, frames: u32, pivot: Vec3) -> Vec<Keyframe> {
    let steps = steps.max(3);
    let u = normalize(sc.vgrads[1]);
    let centre = add(sc.mid, pivot);
    (0..steps)
        .map(|i| {
            let a = 2.0 * PI * i as f64 / steps as f64;
            let mut k = sc.clone();
            k.vgrads = sc.vgrads.map(|r| turn(r, u, a));
            // the middle turned around the pivot as the view
            let m = add(centre, turn(sub(sc.mid, centre), u, a));
            move_middle(&mut k, sub(m, sc.mid));
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
    let steps = steps.max(3);
    let total = (seconds * fps).round().max(steps as f64) as u32;
    let mut ks = orbit_keyframes_around(sc, steps, 1, turntable_pivot(sc));
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
}

impl Default for FlightParams {
    fn default() -> Self {
        FlightParams { frames: 250, keyframe_every: 12, speed: 0.04, max_turn_deg: 2.0, look_ahead: 8.0 }
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
}

/// Why a flight ended, in English (the command line).
pub fn flight_end_text(e: FlightEnd) -> &'static str {
    match e {
        FlightEnd::Done => "all frames flown",
        FlightEnd::DeadEnd => "no way ahead was left",
        FlightEnd::Precision => "the steps came to the limit of double precision",
        FlightEnd::LeftFractal => "the camera left the fractal",
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

/// The view matrix turned so that its forward row points along `nf`, by
/// the smallest rotation (the roll is kept).
fn turn_view_to(sc: &mut Scene, nf: Vec3) {
    let f = normalize(sc.vgrads[2]);
    let axis = cross(f, nf);
    let s = len(&axis);
    if s < 1e-15 {
        return;
    }
    let a = s.atan2(dot(&f, &nf));
    let u = scale(axis, 1.0 / s);
    sc.vgrads = sc.vgrads.map(|r| turn(r, u, a));
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
    let mut key_frame: Vec<usize> = vec![0];
    let (mut blocked, mut outside) = (0, 0);
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
        // the course: a way free for a few distance estimates (the longer
        // the better: a gap leading into the fractal), near the heading (a
        // turn back is the last resort), that does not end at a wall (the
        // distance estimate where it gets to much smaller than here) nor
        // lead out into the empty space around the fractal (free on for
        // many distance estimates beyond)
        let need = (2.5 * de).min(reach);
        let out = fp.look_ahead * 5.0 * de;
        let is_void = |d: Vec3, l: f64| l >= reach * 0.999 && free_distance(&p, add(cam, scale(d, l)), d, out, stop, 0.9, 64) >= out * 0.999;
        let fwd_void = is_void(f, fwd);
        // straight on while the way ahead is free and leads into the
        // fractal (the approach from outside, a gap already taken)
        let best = if fwd >= reach * 0.999 && !fwd_void {
            f
        } else {
            free
            .iter()
            .map(|&(d, l)| {
                let open = (l / reach).min(1.0).sqrt() * (l / need).min(1.0);
                let (de_end, _) = de_at(&p, add(cam, scale(d, l.min(need))));
                let c = (de_end.max(1e-300) / de).ln().min(0.0);
                let wall = 1.0 / (1.0 + c * c);
                let void = if is_void(d, l) { 1e-3 } else { 1.0 };
                (d, open * open * wall * void * (0.5 + 0.5 * dot(&d, &f)))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|x| x.0)
            .unwrap()
        };
        // out in the empty space for a while (a second at 25 frames):
        // the flight has left the fractal
        if fwd_void {
            outside += 1;
            if outside >= 25 {
                end = FlightEnd::LeftFractal;
                break;
            }
        } else {
            outside = 0;
        }
        // steering: a turn of at most `max_turn` a frame
        if dot(&best, &f) < 0.999_999 {
            let a = dot(&f, &best).clamp(-1.0, 1.0).acos();
            let t = (max_turn / a).min(1.0);
            let nf = normalize(add(scale(f, 1.0 - t), scale(best, t)));
            turn_view_to(&mut s, nf);
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
            keys.push(flight_keyframe(&s));
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
}
