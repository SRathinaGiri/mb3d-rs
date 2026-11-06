//! Fly to a view (an animation template, not in MB3D): the camera starts
//! outside the fractal, where all of it is in the picture, or at a given
//! view, and flies to the target view, arriving exactly at it: its place,
//! its direction and its zoom.
//!
//! The way is planned before the flight, not steered as it goes:
//!
//! 1. **A straight line** when there is one. From the target the lines back
//!    out to the start sphere are tried, first the one straight behind the
//!    target's camera (the flight then comes in looking where the view
//!    looks), then lines in rings around it, then the rest of the sphere.
//! 2. **Otherwise a way round** what is in between: a search through the
//!    free space from both ends (RRT-connect, the distance estimate giving
//!    the free room), then cut short to as few, as straight legs as it
//!    allows, and its corners rounded off.
//!
//! A line is free when along it the distance estimate keeps a clearance
//! that narrows towards the target like a funnel (`Space::clearance`):
//! far from it the camera keeps well away from the fractal (no near miss
//! that would make the zoom dive in and out), close to it it may come as
//! near as the target view itself is. The steps along a line are taken at
//! the scene's ray step times the estimate, as the march does (the
//! estimate is no bound, see `autopilot::free_distance`).
//!
//! The camera moves along the way at a speed that follows the distance to
//! the fractal (so the zoom into a deep view keeps its pace all the way
//! down), easing in at the start and out at the end; it looks a little
//! ahead along the way, and over the last part turns to the target's
//! direction. The zoom follows the distance as the Navigator's does; the
//! last keyframe is the target view itself.

use crate::anim::Keyframe;
use crate::autopilot::{camera_on_middle, cross, length_of_size, local_de_with, turn, unit_rows, Flight, FlightEnd};
use crate::calc::{de_at, CalcParams};
use crate::math::{add, dot, len, matrix_to_quat, normalize, rotation_matrix, scale, slerp_quat, sub, Mat3, Vec3};
use crate::scene::Scene;

/// Settings of a flight to a view.
#[derive(Clone, Copy, Debug)]
pub struct ApproachParams {
    /// Frames of the flight (the last one is the target view).
    pub frames: usize,
    /// A keyframe every this many frames.
    pub keyframe_every: usize,
    /// How far outside the start is: the distance at which the whole
    /// fractal fits into the picture, times this.
    pub outside: f64,
    /// VR comfort: the view does not turn on the way (the target's
    /// rotation all along; from a start view a slow turn from its rotation
    /// to the target's over the whole flight).
    pub steady: bool,
}

impl Default for ApproachParams {
    fn default() -> Self {
        ApproachParams { frames: 250, keyframe_every: 12, outside: 1.15, steady: false }
    }
}

/// The middle of the view on the way, in distance estimates in front of the
/// camera (negative: in front, as `z_start - mid z`): where `ModRotPoint`
/// puts it at the zoom that follows the estimate (1.5 picture diagonals of
/// steps). A 360 stereo panorama's eyes are set apart by the camera's
/// distance to the middle, so a middle on the camera gave no stereo.
const OPEN_AHEAD: f64 = -1.5 * 2.1345 / 2.0;

/// How a way to the view was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// One straight line.
    Straight,
    /// Round what was in the way, in this many legs.
    Round(usize),
}

/// A flight to a view and how its way was found.
#[derive(Clone, Debug)]
pub struct Approach {
    pub flight: Flight,
    pub route: Route,
}

/// The free space between the start and the target.
struct Space<'a> {
    p: &'a CalcParams,
    /// the scene's ray step (`z_step_div`)
    ray: f64,
    target: Vec3,
    /// the clearance at the target (half its distance estimate)
    near: f64,
    /// the funnel's slope: the clearance at a distance from the target
    slope: f64,
    /// the fractal's radius around the origin (a little more than found)
    radius: f64,
}

impl Space<'_> {
    /// The distance to the fractal as far as it can be told: the estimate
    /// within the bounds the fractal's sphere sets (outside it at least the
    /// distance to the sphere, at most to its far side). The estimate alone
    /// is no bound far out: the Integer Power bulb's says 50 at 17 units.
    fn de(&self, x: Vec3) -> f64 {
        let (d, _) = de_at(self.p, x);
        let d = if d.is_finite() { d } else { f64::MAX };
        let r = len(&x);
        let d = d.min(r + self.radius);
        if r > self.radius {
            d.max(r - self.radius)
        } else {
            d
        }
    }

    fn clearance(&self, x: Vec3) -> f64 {
        self.near.max(self.slope * len(&sub(x, self.target)))
    }

    /// The line from `a` to `b` keeps the clearance all the way.
    fn clear(&self, a: Vec3, b: Vec3) -> bool {
        let ab = sub(b, a);
        let l = len(&ab);
        if !(l > 0.0) {
            return true;
        }
        let u = scale(ab, 1.0 / l);
        let mut t = 0.0;
        for _ in 0..200_000 {
            let x = add(a, scale(u, t));
            let c = self.clearance(x);
            let r = len(&x);
            // outside the fractal's sphere the distance to it is safe to go
            let outside = r - self.radius;
            let step = if outside > c {
                outside - c + c * 0.02
            } else {
                let d = self.de(x);
                if d < c {
                    return false;
                }
                // (the room beyond the clearance, at the ray step, and a
                // little more so that a line just at the clearance gets on)
                (d - c) * self.ray + c * 0.02
            };
            t += step;
            if t >= l {
                return self.de(b) >= self.clearance(b);
            }
        }
        false
    }
}

/// A small, seeded random generator (the same scene gives the same way).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn dir(&mut self) -> Vec3 {
        let z = 2.0 * self.next() - 1.0;
        let a = 2.0 * std::f64::consts::PI * self.next();
        let r = (1.0 - z * z).max(0.0).sqrt();
        [r * a.cos(), r * a.sin(), z]
    }
}

/// Directions on the sphere, evenly spread (a Fibonacci lattice).
fn sphere_dirs(n: usize) -> Vec<Vec3> {
    let g = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..n)
        .map(|i| {
            let z = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let a = g * i as f64;
            [r * a.cos(), r * a.sin(), z]
        })
        .collect()
}

/// The estimate's floor: at and under the scene's own stop (its DE stop
/// in step widths) the estimate is the surface or inside; there it gives
/// that value and no smaller one.
fn surface(p: &CalcParams) -> f64 {
    p.de_stop as f64 * p.step_width * 1.5
}

/// The fractal's radius around the origin: the farthest surface hit by
/// rays from far outside towards the origin (None when none hits).
fn fractal_radius(p: &CalcParams, far: f64) -> Option<f64> {
    let ray = (p.z_step_div as f64).clamp(0.05, 1.0);
    sphere_dirs(26)
        .into_iter()
        .filter_map(|d| {
            // inwards from far out, each step at most 5 % of the way left
            // to the origin (the estimate far out is no bound)
            let mut r = far;
            for _ in 0..20_000 {
                let (e, _) = de_at(p, scale(d, r));
                let e = if e.is_finite() { e } else { r };
                if e <= (r * 1e-5).max(surface(p)) {
                    return Some(r);
                }
                r -= (e * ray).min(r * 0.05);
                if r <= far * 1e-6 {
                    return None;
                }
            }
            None
        })
        .fold(None, |m: Option<f64>, r| Some(m.map_or(r, |m| m.max(r))))
}

/// Where the line from `x` along the unit `d` leaves the sphere of radius
/// `r` around the origin (`x` inside it).
fn leave_sphere(x: Vec3, d: Vec3, r: f64) -> Vec3 {
    let b = dot(&d, &x);
    let c = dot(&x, &x) - r * r;
    let t = -b + (b * b - c).max(0.0).sqrt();
    add(x, scale(d, t.max(0.0)))
}

/// The directions to try a straight line back from the target along:
/// straight behind its camera first, then rings around that, then the rest
/// of the sphere, nearest first.
fn line_dirs(back: Vec3) -> Vec<Vec3> {
    let mut v = vec![back];
    let a = if back[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let e1 = normalize(cross(back, a));
    let e2 = cross(back, e1);
    for (deg, n) in [(8.0f64, 8), (16.0, 12), (25.0, 16), (35.0, 16), (50.0, 20), (70.0, 24)] {
        let (s, c) = deg.to_radians().sin_cos();
        for j in 0..n {
            let ph = 2.0 * std::f64::consts::PI * j as f64 / n as f64;
            let (sp, cp) = ph.sin_cos();
            v.push(normalize([0, 1, 2].map(|k| back[k] * c + (e1[k] * cp + e2[k] * sp) * s)));
        }
    }
    let mut rest = sphere_dirs(160);
    rest.sort_by(|a, b| dot(b, &back).partial_cmp(&dot(a, &back)).unwrap());
    v.extend(rest.into_iter().filter(|d| dot(d, &back) < 70f64.to_radians().cos()));
    v
}

/// A way round: RRT-connect from `a` and `b`, the free room from the
/// distance estimate. None when none is found within the tries.
fn search(sp: &Space, a: Vec3, b: Vec3, radius: f64, rng: &mut Rng, cancel: &dyn Fn() -> bool) -> Option<Vec<Vec3>> {
    let da = sp.de(a).max(sp.near);
    let db = sp.near * 2.0;
    // the trees: the points and their parents
    let mut trees: [Vec<(Vec3, usize)>; 2] = [vec![(a, usize::MAX)], vec![(b, usize::MAX)]];
    let nearest = |t: &Vec<(Vec3, usize)>, q: Vec3| -> usize {
        let mut best = (0, f64::MAX);
        for (i, (x, _)) in t.iter().enumerate() {
            let d = len(&sub(*x, q));
            if d < best.1 {
                best = (i, d);
            }
        }
        best.0
    };
    let path = |t: &Vec<(Vec3, usize)>, mut i: usize| -> Vec<Vec3> {
        let mut v = Vec::new();
        while i != usize::MAX {
            v.push(t[i].0);
            i = t[i].1;
        }
        v
    };
    for it in 0..6000 {
        if it % 64 == 0 && cancel() {
            return None;
        }
        let grow = it % 2;
        // a point to grow towards: around the target over all scales from
        // its own to the whole (a deep view needs the small ones), around
        // the start, anywhere around the fractal, or near a point already
        // reached
        let u = rng.next();
        let q = if u < 0.35 {
            let r = db * (radius / db).powf(rng.next());
            add(b, scale(rng.dir(), r))
        } else if u < 0.5 {
            let r = da * (radius / da).powf(rng.next());
            add(a, scale(rng.dir(), r))
        } else if u < 0.8 {
            scale(rng.dir(), radius * rng.next().cbrt())
        } else {
            let t = &trees[grow];
            let x = t[(rng.next() * t.len() as f64) as usize % t.len()].0;
            add(x, scale(rng.dir(), sp.de(x).max(sp.near) * 3.0))
        };
        let t = &trees[grow];
        let n = nearest(t, q);
        let from = t[n].0;
        let dq = sub(q, from);
        let l = len(&dq);
        if !(l > 0.0) {
            continue;
        }
        let room = sp.de(from).max(sp.clearance(from));
        let x = add(from, scale(dq, (room * 3.0).min(l) / l));
        if !sp.clear(from, x) {
            continue;
        }
        trees[grow].push((x, n));
        let new = trees[grow].len() - 1;
        // the other tree reached from the new point in one line?
        let other = 1 - grow;
        let m = nearest(&trees[other], x);
        if sp.clear(x, trees[other][m].0) {
            let mut pa = path(&trees[grow], new);
            let mut pb = path(&trees[other], m);
            pa.reverse();
            let mut way = pa;
            way.append(&mut pb);
            if grow == 1 {
                way.reverse();
            }
            return Some(way);
        }
    }
    None
}

/// The way cut short: from each point the farthest one still in a line.
fn shortcut(sp: &Space, way: &[Vec3]) -> Vec<Vec3> {
    let mut out = vec![way[0]];
    let mut i = 0;
    while i + 1 < way.len() {
        let mut j = way.len() - 1;
        while j > i + 1 && !sp.clear(way[i], way[j]) {
            j -= 1;
        }
        out.push(way[j]);
        i = j;
    }
    out
}

/// Corners rounded off (Chaikin), where the rounded legs are free too.
fn round_corners(sp: &Space, way: Vec<Vec3>) -> Vec<Vec3> {
    let mut w = way;
    for _ in 0..3 {
        if w.len() < 3 {
            return w;
        }
        let mut n = vec![w[0]];
        for k in 0..w.len() - 1 {
            let (a, b) = (w[k], w[k + 1]);
            let q = add(scale(a, 0.75), scale(b, 0.25));
            let r = add(scale(a, 0.25), scale(b, 0.75));
            if k > 0 {
                n.push(q);
            }
            if k + 1 < w.len() - 1 {
                n.push(r);
            }
        }
        n.push(*w.last().unwrap());
        if n.windows(2).all(|s| sp.clear(s[0], s[1])) {
            w = n;
        } else {
            return w;
        }
    }
    w
}

/// Points along the way and their distance from the start in distance
/// estimates (∫ ds / DE): the measure the camera moves at.
fn measure(sp: &Space, way: &[Vec3]) -> (Vec<Vec3>, Vec<f64>) {
    let mut pts = vec![way[0]];
    let mut us = vec![0.0];
    let mut u = 0.0;
    for s in way.windows(2) {
        let (a, b) = (s[0], s[1]);
        let l = len(&sub(b, a));
        let mut t = 0.0;
        while t < l {
            let x = add(a, scale(sub(b, a), t / l));
            let d = sp.de(x).max(sp.near);
            let dt = (d * 0.2).min(l - t).max(l * 1e-6);
            u += dt / d;
            t += dt;
            pts.push(add(a, scale(sub(b, a), (t / l).min(1.0))));
            us.push(u);
        }
    }
    (pts, us)
}

/// Keyframes at measures `ukeys` made such that neighbouring gaps on the
/// way differ at most threefold: where one is longer, a keyframe is put
/// into it at twice the other's length from their shared keyframe. (A
/// Catmull-Rom curve swings back where a gap is more than five times its
/// neighbour: the tangent at the shared keyframe, half the two gaps,
/// exceeds three times the shorter one. Far out, where the distance
/// estimate is clamped to the fractal's sphere, even steps of the measure
/// gave gaps of 3.7 and 0.4: 9 % back.)
fn even_gaps(pts: &[Vec3], us: &[f64], mut ukeys: Vec<f64>) -> Vec<f64> {
    // the arc length along the way at each point of the measure
    let mut ss = vec![0.0];
    for w in pts.windows(2) {
        ss.push(ss.last().unwrap() + len(&sub(w[1], w[0])));
    }
    let map = |from: &[f64], to: &[f64], x: f64| {
        let x = x.clamp(0.0, *from.last().unwrap());
        let i = from.partition_point(|&v| v < x).clamp(1, from.len() - 1);
        let (a, b) = (from[i - 1], from[i]);
        let w = if b > a { (x - a) / (b - a) } else { 1.0 };
        to[i - 1] + (to[i] - to[i - 1]) * w
    };
    for _ in 0..500 {
        let sk: Vec<f64> = ukeys.iter().map(|&u| map(us, &ss, u)).collect();
        let mut add = None;
        for i in 1..sk.len().saturating_sub(1) {
            let (a, b) = (sk[i] - sk[i - 1], sk[i + 1] - sk[i]);
            if a > 3.0 * b && b > 0.0 {
                add = Some((i, sk[i] - 2.0 * b));
                break;
            }
            if b > 3.0 * a && a > 0.0 {
                add = Some((i + 1, sk[i] + 2.0 * a));
                break;
            }
        }
        match add {
            Some((at_index, s)) => ukeys.insert(at_index, map(&ss, us, s)),
            None => break,
        }
    }
    ukeys
}

/// The unit rows `m` with the lengths of the rows of `like`.
fn rows_like(m: Mat3, like: &Mat3) -> Mat3 {
    [0, 1, 2].map(|r| scale(m[r], len(&like[r])))
}

/// The point at measure `u` along the way.
fn at(pts: &[Vec3], us: &[f64], u: f64) -> Vec3 {
    let u = u.clamp(0.0, *us.last().unwrap());
    let i = us.partition_point(|&x| x < u).clamp(1, us.len() - 1);
    let (u0, u1) = (us[i - 1], us[i]);
    let w = if u1 > u0 { (u - u0) / (u1 - u0) } else { 1.0 };
    add(scale(pts[i - 1], 1.0 - w), scale(pts[i], w))
}

/// The view's rows turned by the smallest rotation that takes the unit
/// `from` to the unit `to` (`up` the axis when they are opposite).
fn turned_rows(rows: [Vec3; 3], from: Vec3, to: Vec3, up: Vec3) -> [Vec3; 3] {
    let ax = cross(from, to);
    let s = len(&ax);
    let c = dot(&from, &to).clamp(-1.0, 1.0);
    if s < 1e-12 {
        if c > 0.0 {
            return rows;
        }
        let u = normalize(up);
        return rows.map(|r| turn(r, u, std::f64::consts::PI));
    }
    let u = scale(ax, 1.0 / s);
    let a = s.atan2(c);
    rows.map(|r| turn(r, u, a))
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The share of the way flown at the share `t` of the time: easing in over
/// the first 8 %, out over the last 15 %.
fn eased(t: f64) -> f64 {
    let (a, b) = (0.08, 0.15);
    // the speed rises and falls as half sines; the integral, normalised
    let v = |x: f64| -> f64 {
        if x < a {
            (0.5 - 0.5 * (std::f64::consts::PI * x / a).cos()).max(0.0)
        } else if x > 1.0 - b {
            (0.5 - 0.5 * (std::f64::consts::PI * (1.0 - x) / b).cos()).max(0.0)
        } else {
            1.0
        }
    };
    let n = 400;
    let integral = |to: f64| -> f64 { (0..n).map(|i| v((i as f64 + 0.5) / n as f64 * to)).sum::<f64>() * to / n as f64 };
    integral(t.clamp(0.0, 1.0)) / integral(1.0)
}

/// A flight to the view `target` (see the module notes), from `from` or,
/// without one, from outside the fractal. `progress(done, total)` is called
/// as it goes; `cancel` stops it.
pub fn fly_to(target: &Scene, from: Option<&Scene>, ap: &ApproachParams, progress: &dyn Fn(usize, usize), cancel: &dyn Fn() -> bool) -> Result<Approach, String> {
    let mut t = target.clone();
    t.slice_2d = 0;
    t.stereo_mode = 0;
    let p = CalcParams::new(&t)?;
    let ray = (p.z_step_div as f64).clamp(0.05, 1.0);
    // the target camera (the Navigator's: the start plane's middle)
    let mut tc = t.clone();
    camera_on_middle(&mut tc);
    let pt = tc.mid;
    let [_, ut, ft] = unit_rows(&tc);
    let dt = de_at(&p, pt).0;
    if !(dt > surface(&p)) || !dt.is_finite() {
        return Err("the view's camera is inside the fractal".into());
    }
    progress(0, ap.frames);
    // the fractal's sphere (a little more than the farthest surface found)
    let radius_obj = fractal_radius(&p, (len(&pt) * 4.0).max(20.0)).unwrap_or_else(|| len(&pt).max(1.0)) * 1.1;
    let sp = Space { p: &p, ray, target: pt, near: dt * 0.5, slope: 0.05, radius: radius_obj };
    let dt = sp.de(pt);
    let sp = Space { near: dt * 0.5, ..sp };
    let de_view = local_de_with(&p, &tc).unwrap_or(dt).min(dt.max(1e-300) * 4.0);

    // the start: the given view's camera, or outside the fractal
    let half = (t.fov_y.clamp(5.0, 120.0) * 0.5).to_radians();
    let outside = (radius_obj / half.sin() * ap.outside).max(radius_obj * 1.5).max(len(&pt) * 1.2);
    let (start, route) = match from {
        Some(f) => {
            let mut fc = f.clone();
            fc.slice_2d = 0;
            camera_on_middle(&mut fc);
            if de_at(&p, fc.mid).0 <= surface(&p) {
                return Err("the start view's camera is inside the fractal".into());
            }
            let s = fc.mid;
            if sp.clear(s, pt) {
                (vec![s, pt], Route::Straight)
            } else {
                let w = search(&sp, s, pt, outside, &mut Rng(0x9E37_79B9_7F4A_7C15), cancel).ok_or("no free way from the start view to the view was found")?;
                let w = round_corners(&sp, shortcut(&sp, &w));
                let n = w.len() - 1;
                (w, Route::Round(n))
            }
        }
        None => {
            let back = scale(ft, -1.0);
            let mut found = None;
            for (k, d) in line_dirs(back).into_iter().enumerate() {
                if k % 16 == 0 && cancel() {
                    return Err("cancelled".into());
                }
                let s = leave_sphere(pt, d, outside);
                if sp.clear(s, pt) {
                    found = Some(s);
                    break;
                }
            }
            match found {
                Some(s) => (vec![s, pt], Route::Straight),
                None => {
                    let s = leave_sphere(pt, back, outside);
                    let w = search(&sp, s, pt, outside, &mut Rng(0x9E37_79B9_7F4A_7C15), cancel).ok_or("no free way from outside the fractal to the view was found")?;
                    let w = round_corners(&sp, shortcut(&sp, &w));
                    let n = w.len() - 1;
                    (w, Route::Round(n))
                }
            }
        }
    };
    let way = start;

    // the camera along the way
    let (pts, us) = measure(&sp, &way);
    let total = *us.last().unwrap();
    let frames = ap.frames.max(2);
    let every = ap.keyframe_every.max(1);
    // (the middle's place in front of the camera and the far plane, in
    // distance estimates, as in the view)
    let ahead_rel = (t.z_start - t.mid[2]) / de_view;
    let range_rel = ((t.z_end - t.z_start) / de_view).clamp(1.0, 10000.0);
    let d_start = sp.de(way[0]).max(sp.near);
    // the turn to the view's direction over the last part
    let turn_from = (total * 0.8).max(total - 6.0).max(0.0);
    // the keyframes at even steps along the way (in distance estimates:
    // between neighbours the spacing then changes by the same factor, which
    // the Catmull-Rom curve follows without swinging back), each at the
    // frame the eased timing reaches it
    let n_keys = ((frames - 1) / every).max(1) + 1;
    let ukeys = even_gaps(&pts, &us, (0..n_keys).map(|j| total * j as f64 / (n_keys - 1) as f64).collect());
    let mut plan: Vec<(f64, usize)> = Vec::new();
    let last_key = ukeys.len() - 1;
    for (j, &uk) in ukeys.iter().enumerate() {
        let share = if total > 0.0 { (uk / total).clamp(0.0, 1.0) } else { j as f64 / last_key.max(1) as f64 };
        // the time at which the eased flight has flown this share
        let (mut lo, mut hi) = (0.0f64, 1.0f64);
        for _ in 0..50 {
            let m = 0.5 * (lo + hi);
            if eased(m) < share {
                lo = m;
            } else {
                hi = m;
            }
        }
        let fr = ((frames - 1) as f64 * hi).round() as usize;
        let fr = if j == last_key { frames - 1 } else { fr.min(frames - 2) };
        if plan.last().is_some_and(|&(_, f)| fr <= f) {
            continue;
        }
        plan.push((uk, fr));
    }
    if plan.last().map(|x| x.1) != Some(frames - 1) {
        plan.pop();
        plan.push((total, frames - 1));
    }
    let mut keys: Vec<Scene> = Vec::new();
    let mut key_frame: Vec<usize> = Vec::new();
    let mut d_prev = f64::MAX;
    for (j, &(u, fr)) in plan.iter().enumerate() {
        if cancel() {
            return Err("cancelled".into());
        }
        progress(fr, frames);
        if j + 1 == plan.len() {
            keys.push(target.clone());
            key_frame.push(fr);
            break;
        }
        if j == 0 {
            if let Some(f) = from {
                keys.push(f.clone());
                key_frame.push(0);
                continue;
            }
        }
        let x = at(&pts, &us, u);
        // looking ahead along the way, turning to the view's direction at
        // the end
        let ahead = at(&pts, &us, (u + 2.0).min(total));
        let dir = if len(&sub(ahead, x)) > 0.0 { normalize(sub(ahead, x)) } else { ft };
        let w = smoothstep(if total > turn_from { (u - turn_from) / (total - turn_from) } else { 1.0 });
        let f = normalize(add(scale(dir, 1.0 - w), scale(ft, w)));
        let f = if len(&f) > 0.0 { f } else { ft };
        // the distance the zoom follows: the estimate here, but never more
        // than the share of the way from the start's to the view's
        let geo = (d_start.ln() + (de_view.ln() - d_start.ln()) * (u / total.max(1e-12))).exp();
        // (and never more than at the keyframe before, nor under the
        // view's: the zoom rises all the way to the view's, without a dip)
        let d = sp.de(x).min(geo).min(d_prev).max(de_view);
        d_prev = d;
        let mut s = tc.clone();
        s.vgrads = match (ap.steady, from) {
            (false, _) => turned_rows(t.vgrads, ft, f, ut),
            (true, None) => t.vgrads,
            // (a slow turn from the start's rotation to the view's)
            (true, Some(fs)) => {
                let q = slerp_quat(&matrix_to_quat(&fs.vgrads), &matrix_to_quat(&t.vgrads), smoothstep(u / total.max(1e-12)));
                rows_like(rotation_matrix(&q), &t.vgrads)
            }
        };
        s.zoom = length_of_size(&s) * 2.0 / (d * s.width as f64);
        // the middle in front of the camera as ModRotPoint puts it, moving
        // to its place in the view at the end
        let fz = normalize(s.vgrads[2]);
        let off = d * (OPEN_AHEAD * (1.0 - w) + ahead_rel * w);
        s.mid = sub(x, scale(fz, off));
        s.z_start = s.mid[2] + off;
        // the far plane: on the way to the far side of the fractal, at the
        // end the view's own (as many distance estimates away)
        let far_open = (d * range_rel).max(len(&x) + radius_obj);
        s.z_end = s.z_start + (far_open * (1.0 - w) + d * range_rel * w).min(9999.0);
        keys.push(s);
        key_frame.push(fr);
    }
    progress(frames, frames);
    let n = keys.len();
    let keyframes = keys
        .into_iter()
        .enumerate()
        .map(|(k, sc)| {
            let fr = if k + 1 < n { (key_frame[k + 1] - key_frame[k]) as u32 } else { 1 };
            Keyframe::new(sc, fr)
        })
        .collect();
    Ok(Approach { flight: Flight { keyframes, frames, end: FlightEnd::Done }, route })
}

/// Zoom out: from the view `view` back to outside the fractal, where all of
/// it is in view. [`fly_to`] from outside played backwards: the same free
/// way, the camera facing the view's place as it pulls back; the first
/// keyframe is the view exactly.
pub fn fly_out(view: &Scene, ap: &ApproachParams, progress: &dyn Fn(usize, usize), cancel: &dyn Fn() -> bool) -> Result<Approach, String> {
    let a = fly_to(view, None, ap, progress, cancel)?;
    let mut keys = a.flight.keyframes;
    let n = keys.len();
    // the frames to the next keyframe, backwards: keyframe j is the old
    // n-1-j, the frames after it the old ones before that
    let gaps: Vec<u32> = keys.iter().take(n.saturating_sub(1)).map(|k| k.frames).collect();
    keys.reverse();
    for (j, k) in keys.iter_mut().enumerate() {
        k.frames = if j + 1 < n { gaps[n - 2 - j] } else { 1 };
    }
    Ok(Approach { flight: Flight { keyframes: keys, frames: a.flight.frames, end: FlightEnd::Done }, route: a.route })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::{Animation, Interpolation};
    use crate::autopilot::cam_pos;

    /// A view of the Integer Power bulb from close by: the camera moved
    /// towards the bulb along the view until the estimate is `d`.
    fn close_view(d: f64) -> Scene {
        let mut sc = Scene::preset("Integer Power").unwrap();
        camera_on_middle(&mut sc);
        // the zoom of a view that close (its DE stop is then small enough
        // to tell that distance), then looking at the origin
        let f0 = unit_rows(&sc)[2];
        let f = normalize(scale(sc.mid, -1.0));
        sc.vgrads = turned_rows(sc.vgrads, f0, f, unit_rows(&sc)[1]);
        sc.zoom = length_of_size(&sc) * 2.0 / (d * sc.width as f64);
        let p = CalcParams::new(&sc).unwrap();
        // (marching in steps of at most a twentieth of the way to the
        // origin: the estimate far out is no bound)
        let mut t = 0.0;
        for _ in 0..100_000 {
            let x = add(sc.mid, scale(f, t));
            let (e, _) = de_at(&p, x);
            if e <= d {
                break;
            }
            t += ((e - d) * 0.5).min(len(&x) * 0.05).max(d * 0.01);
        }
        let m = add(sc.mid, scale(f, t));
        sc.z_end += m[2] - sc.mid[2];
        sc.z_start = m[2];
        sc.mid = m;
        sc.zoom = length_of_size(&sc) * 2.0 / (d * sc.width as f64);
        sc
    }

    fn anim(fl: &Flight) -> Animation {
        Animation { keyframes: fl.keyframes.clone(), interpolation: Interpolation::CatmullRom, looped: false, ..Default::default() }
    }

    #[test]
    fn a_view_in_the_open_is_reached_on_a_straight_line_and_exactly() {
        let target = close_view(0.05);
        let ap = ApproachParams { frames: 120, ..Default::default() };
        let a = fly_to(&target, None, &ap, &|_, _| {}, &|| false).unwrap();
        assert_eq!(a.route, Route::Straight);
        let ks = &a.flight.keyframes;
        // the last keyframe is the view itself
        assert_eq!(ks.last().unwrap().scene.mid, target.mid);
        assert_eq!(ks.last().unwrap().scene.vgrads, target.vgrads);
        assert_eq!(ks.last().unwrap().scene.zoom, target.zoom);
        // the frames add up
        let an = anim(&a.flight);
        assert_eq!(an.frame_count(), 120);
        // from outside: the first camera far from the bulb, every camera of
        // every frame outside it, the distance to it falling to the view's
        let p = CalcParams::new(&target).unwrap();
        let first = cam_pos(&an.frame_scene(0).unwrap());
        assert!(len(&first) > 3.0, "{first:?}");
        let mut prev = f64::MAX;
        for i in 0..120 {
            let c = cam_pos(&an.frame_scene(i).unwrap());
            let (d, _) = de_at(&p, c);
            assert!(d > 0.0, "frame {i} inside");
            let to = len(&sub(c, cam_pos(&target)));
            assert!(to <= prev * 1.0001 + 1e-9, "frame {i}: {to} after {prev}");
            prev = to;
        }
    }

    #[test]
    fn on_the_way_the_middle_is_in_front_for_the_stereo() {
        // a 360 stereo panorama's eyes are as far apart as the camera is
        // from the middle: on the camera there was no stereo
        let view = close_view(1e-3);
        let ap = ApproachParams { frames: 150, ..Default::default() };
        let a = fly_to(&view, None, &ap, &|_, _| {}, &|| false).unwrap();
        let an = anim(&a.flight);
        let p = CalcParams::new(&view).unwrap();
        for i in 0..120 {
            let s = an.frame_scene(i).unwrap();
            let ahead = (s.z_start - s.mid[2]).abs();
            let de = de_at(&p, cam_pos(&s)).0;
            assert!(ahead > 0.3 * de, "frame {i}: middle {ahead}, estimate {de}");
        }
    }

    #[test]
    fn a_steady_flight_never_turns() {
        let view = close_view(1e-3);
        let ap = ApproachParams { frames: 120, steady: true, ..Default::default() };
        for out in [false, true] {
            let a = if out { fly_out(&view, &ap, &|_, _| {}, &|| false) } else { fly_to(&view, None, &ap, &|_, _| {}, &|| false) }.unwrap();
            let an = anim(&a.flight);
            let f = unit_rows(&view)[2];
            for i in 0..120 {
                let s = an.frame_scene(i).unwrap();
                assert!((dot(&unit_rows(&s)[2], &f) - 1.0).abs() < 1e-9, "frame {i} turned");
            }
            let end = if out { &a.flight.keyframes[0] } else { a.flight.keyframes.last().unwrap() };
            assert_eq!(end.scene.vgrads, view.vgrads);
            assert_eq!(end.scene.mid, view.mid);
        }
    }

    #[test]
    fn zooming_out_starts_at_the_view_and_ends_outside() {
        let view = close_view(1e-3);
        let ap = ApproachParams { frames: 150, ..Default::default() };
        let a = fly_out(&view, &ap, &|_, _| {}, &|| false).unwrap();
        let ks = &a.flight.keyframes;
        assert_eq!(ks[0].scene.mid, view.mid);
        assert_eq!(ks[0].scene.vgrads, view.vgrads);
        assert_eq!(ks[0].scene.zoom, view.zoom);
        let an = anim(&a.flight);
        assert_eq!(an.frame_count(), 150);
        // the distance from the view rising, the zoom falling, every camera
        // outside the bulb, the last far from it
        let p = CalcParams::new(&view).unwrap();
        let (mut prev, mut zoom) = (0.0, f64::MAX);
        for i in 0..150 {
            let s = an.frame_scene(i).unwrap();
            let c = cam_pos(&s);
            assert!(de_at(&p, c).0 > 0.0, "frame {i} inside");
            let from = len(&sub(c, cam_pos(&view)));
            assert!(from >= prev * 0.9999 - 1e-9, "frame {i}: {from} after {prev}");
            assert!(s.zoom.ln() <= zoom.ln() + 0.05, "frame {i}: zoom rose");
            prev = from;
            zoom = zoom.min(s.zoom);
        }
        assert!(len(&cam_pos(&an.frame_scene(149).unwrap())) > 3.0);
    }

    #[test]
    fn a_deep_view_is_approached_without_a_step_back() {
        // far out the gaps between keyframes at even measure were 3.7 and
        // 0.4: the Catmull-Rom curve swung 9 % back
        let view = close_view(1e-3);
        let ap = ApproachParams { frames: 150, ..Default::default() };
        let a = fly_to(&view, None, &ap, &|_, _| {}, &|| false).unwrap();
        let an = anim(&a.flight);
        let mut prev = f64::MAX;
        for i in 0..150 {
            let to = len(&sub(cam_pos(&an.frame_scene(i).unwrap()), cam_pos(&view)));
            assert!(to <= prev * 1.0001 + 1e-12, "frame {i}: {to} after {prev}");
            prev = to;
        }
    }

    #[test]
    fn a_view_behind_the_bulb_is_reached_round_it() {
        // from a view in front of the bulb to one behind it, looking back:
        // the straight line goes through the bulb
        let front = Scene::preset("Integer Power").unwrap();
        let mut back = front.clone();
        camera_on_middle(&mut back);
        let f = unit_rows(&back)[2];
        let c0 = back.mid;
        // the camera mirrored through the origin, looking at the bulb
        let m = scale(c0, -1.0);
        let dz = m[2] - back.mid[2];
        back.mid = m;
        back.z_start += dz;
        back.z_end += dz;
        back.vgrads = turned_rows(back.vgrads, f, scale(f, -1.0), unit_rows(&back)[1]);
        let p = CalcParams::new(&back).unwrap();
        let ap = ApproachParams { frames: 150, ..Default::default() };
        let a = fly_to(&back, Some(&front), &ap, &|_, _| {}, &|| false).unwrap();
        assert!(matches!(a.route, Route::Round(_)), "{:?}", a.route);
        let an = anim(&a.flight);
        for i in 0..150 {
            let c = cam_pos(&an.frame_scene(i).unwrap());
            assert!(de_at(&p, c).0 > 0.0, "frame {i} inside");
        }
        assert_eq!(a.flight.keyframes.last().unwrap().scene.mid, back.mid);
        assert_eq!(a.flight.keyframes[0].scene.mid, front.mid);
    }

    #[test]
    fn a_deep_view_is_reached_with_the_zoom_following() {
        let target = close_view(1e-4);
        let ap = ApproachParams { frames: 200, ..Default::default() };
        let a = fly_to(&target, None, &ap, &|_, _| {}, &|| false).unwrap();
        let an = anim(&a.flight);
        let p = CalcParams::new(&target).unwrap();
        // the zoom rises on the way in, about evenly in its logarithm
        let z: Vec<f64> = (0..200).map(|i| an.frame_scene(i).unwrap().zoom.ln()).collect();
        assert!(z[199] > z[0] + 5.0, "{} -> {}", z[0], z[199]);
        for i in 0..200 {
            let c = cam_pos(&an.frame_scene(i).unwrap());
            assert!(de_at(&p, c).0 > 0.0, "frame {i} inside");
        }
        for i in 1..200 {
            // (between the keyframes, which rise, the Catmull-Rom curve may
            // dip by a hair: at most 5 %, which does not show: the zoom sets
            // the detail and the start plane, the field of view stays)
            assert!(z[i] >= z[i - 1] - 0.05, "the zoom falls at frame {i}: {} -> {}", z[i - 1], z[i]);
        }
        let half = z[100] - z[0];
        let all = z[199] - z[0];
        assert!(half > all * 0.25 && half < all * 0.75, "{half} of {all}");
    }
}
