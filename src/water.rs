//! Transparent water of the ground plane (not in MB3D): the light that
//! the reflection pass (`reflect`) and the Monte Carlo renderer (`mc`)
//! see through the surface.
//!
//! Where a ray meets the water, Fresnel's equations (the exact form MB3D
//! uses for its transparent materials, `CalcSR.pas`) split the light
//! into a reflected part F and a refracted part 1 - F.  The refracted ray
//! goes on under the surface, bent by Snell's law at the waves' normal,
//! and sees the fractal only (the plane no longer stops it).  On its way
//! the light fades (Beer-Lambert: each channel keeps exp(-distance *
//! extinction / visibility), red much faster than blue in a blue-green
//! water), and what fades is replaced by the water's own scattered light,
//! so deep water turns into its colour, not black.  The lights' glints on
//! the waves stay on the surface, so the waterline shows.
//! The light reaching a surface under water has crossed the water above
//! it too: its depth over the cosine of the light's (refracted) angle.
//!
//! The surface of a water pixel then is
//! (1 - T) * its own colour as before + R * the reflection
//! + T (1 - F) * the light from under the surface,
//! with T the transparency and R = (1 - T) * the ground's reflection
//! + T * F, so that a transparency of 0 is the opaque ground of before
//! and 1 the clear water of Fresnel's equations.

use crate::math::*;
use crate::scene::Ground;

/// Fresnel's reflectance of unpolarised light that meets a surface at
/// `cos_i` (the cosine of the angle to its normal, 0..1) from a medium
/// with `n` times less refraction index than the one behind it (n = 1.33
/// for air into water): (reflected share, cosine of the refracted ray).
/// Total internal reflection (only from the denser side) gives (1, 0).
/// The same formula as the reflection pass' and Monte Carlo's
/// transparent materials (`CalcSR.pas`).
pub fn fresnel(cos_i: f64, n: f64) -> (f64, f64) {
    let a = cos_i.abs().min(1.0);
    let ds = 1.0 / n.max(1e-6);
    let k = 1.0 - ds * ds * (1.0 - a * a);
    if k <= 0.0 {
        return (1.0, 0.0);
    }
    let b = k.sqrt();
    let f = (((a - ds * b) / (a + ds * b)).powi(2) + ((b - ds * a) / (b + ds * a)).powi(2)) * 0.5;
    (f.clamp(0.0, 1.0), b)
}

/// The refracted direction (unit) of the unit direction `d` at a surface
/// with the unit normal `n` facing it (`dot(d, n) < 0`), the cosines of
/// [`fresnel`] and the ratio `eta` = n1 / n2 of the refraction indices.
pub fn refract(d: Vec3, n: Vec3, eta: f64, cos_i: f64, cos_t: f64) -> Vec3 {
    normalize(add(scale(d, eta), scale(n, eta * cos_i - cos_t)))
}

/// How much faster each channel fades in water of the colour `c` than
/// its clearest one (1): 1 + 5 ln(clearest / channel), so a blue-green
/// water (10, 60, 70) fades red, green and blue about 10 : 2 : 1, as pure
/// water absorbs about 0.45, 0.06 and 0.02 per metre.  A grey water fades
/// all alike.
pub fn extinction(c: [u8; 3]) -> [f64; 3] {
    let m = c.iter().copied().max().unwrap_or(0).max(1) as f64;
    c.map(|v| 1.0 + 5.0 * -(v as f64 / m).clamp(0.02, 1.0).ln())
}

/// The fraction of a channel of light that crosses `dist` fractal units
/// of the water (Beer-Lambert): exp(-dist * extinction / visibility), so
/// the clearest channel keeps 1/e at the visibility and the light turns
/// towards the water's colour as it fades.
pub fn transmittance(g: &Ground, dist: f64) -> [f32; 3] {
    let d = dist.max(0.0) / g.visibility.max(1e-6) as f64;
    extinction(g.water_color).map(|e| if d.is_infinite() { 0.0 } else { (-d * e).exp() as f32 })
}

/// The length of a straight ray under the water: from `o` along the unit
/// direction `d` for `len` fractal units (infinite: into the distance),
/// the part below the plane (the waves' height at `o` counted, the plane
/// flat beyond it).
pub fn length_in_water(g: &Ground, o: &Vec3, d: &Vec3, len: f64) -> f64 {
    let d0 = g.depth(o);
    // the depth's change per unit along the ray
    let r = -dot(d, &g.up());
    if r.abs() < 1e-12 {
        return if d0 > 0.0 { len } else { 0.0 };
    }
    let t0 = -d0 / r;
    if r > 0.0 {
        // going down: under water beyond t0
        (len - t0.max(0.0)).max(0.0)
    } else {
        // going up: under water until t0
        t0.clamp(0.0, len)
    }
}

/// The way of the light from a direction `l` (unit, towards the light)
/// through the water down to `c`: its depth over the cosine of the light's
/// angle under the surface (bent by the refraction); 0 above the water.
pub fn light_path(g: &Ground, c: &Vec3, l: &Vec3) -> f64 {
    let depth = g.depth(c);
    if depth <= 0.0 {
        return 0.0;
    }
    let cos_up = dot(l, &g.up()).clamp(0.0, 1.0);
    let n = g.ior.max(1.0) as f64;
    let cos_w = (1.0 - (1.0 - cos_up * cos_up) / (n * n)).max(0.0).sqrt();
    depth / cos_w.max(0.05)
}

/// The colours of a water hit: (the share F of the reflection, the
/// refracted ray's direction): `view` is the ray's direction (any
/// length), `c` the point on the surface.
pub fn split(g: &Ground, c: &Vec3, view: &Vec3) -> (f64, Vec3) {
    let d = normalize(*view);
    let mut n = g.normal(c);
    let mut cos_i = -dot(&d, &n);
    if cos_i < 0.0 {
        // (from below the plane: not in this stage; as if from above)
        n = scale(n, -1.0);
        cos_i = -cos_i;
    }
    let ior = g.ior.max(1.0) as f64;
    let (f, cos_t) = fresnel(cos_i, ior);
    (f, refract(d, n, 1.0 / ior, cos_i, cos_t))
}

// ---------------------------------------------------------------------------
// scattering (`Ground::scattering`): the water's light seen along a ray

/// Henyey-Greenstein's phase function times 4 pi (1 = the same in every
/// direction) for the cosine between the light's direction of travel and
/// the direction towards the eye, with the anisotropy `g` (forward > 0).
pub fn hg(cos: f64, g: f64) -> f64 {
    let g2 = g * g;
    (1.0 - g2) / (1.0 + g2 - 2.0 * g * cos.clamp(-1.0, 1.0)).max(1e-9).powf(1.5)
}

/// The strength of the light scattered once (the light shafts) over its
/// physical amount: an artistic gain, the same in both renderers.
pub const SHAFTS: f64 = 2.0;

/// The phase of the light scattered once and seen through more water:
/// half Henyey-Greenstein's with `g`, half with a third of it (the light
/// scattered on the way blurs the forward peak), times 4 pi.
pub fn phase_seen(cos: f64, g: f64) -> f64 {
    0.5 * hg(cos, g) + 0.5 * hg(cos, g / 3.0)
}

/// A direction around the unit direction `w` drawn from Henyey-Greenstein
/// (`u1`, `u2` uniform 0..1): the new direction of a scattered ray.
pub fn sample_hg(w: Vec3, g: f64, u1: f64, u2: f64) -> Vec3 {
    let cos = if g.abs() < 1e-3 {
        1.0 - 2.0 * u1
    } else {
        let s = (1.0 - g * g) / (1.0 - g + 2.0 * g * u1);
        ((1.0 + g * g - s * s) / (2.0 * g)).clamp(-1.0, 1.0)
    };
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let phi = std::f64::consts::TAU * u2;
    let a = if w[0].abs() > 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
    let t = normalize([w[1] * a[2] - w[2] * a[1], w[2] * a[0] - w[0] * a[2], w[0] * a[1] - w[1] * a[0]]);
    let b = [w[1] * t[2] - w[2] * t[1], w[2] * t[0] - w[0] * t[2], w[0] * t[1] - w[1] * t[0]];
    normalize(add(add(scale(t, sin * phi.cos()), scale(b, sin * phi.sin())), scale(w, cos)))
}

/// How much light the water scatters per fractal unit (grey): the
/// scattering amount over the visibility.
pub fn sigma_s(g: &Ground) -> f64 {
    g.scattering.max(0.0) as f64 / g.visibility.max(1e-6) as f64
}

/// [`transmittance`] with the scattering's loss too: what goes straight
/// on (the scattered part comes back as the water's light).
pub fn transmittance_s(g: &Ground, dist: f64) -> [f32; 3] {
    let s = (-sigma_s(g) * dist.max(0.0)).exp() as f32;
    transmittance(g, dist).map(|t| if s.is_finite() { t * s } else { 0.0 })
}

/// The direction of travel under the surface of the light coming from
/// the unit direction `l` (towards the light), bent by the refraction.
pub fn light_under(g: &Ground, l: &Vec3) -> Vec3 {
    let up = g.up();
    let cos_i = dot(l, &up).clamp(1e-3, 1.0);
    let ior = g.ior.max(1.0) as f64;
    let (_, cos_t) = fresnel(cos_i, ior);
    refract(scale(*l, -1.0), up, 1.0 / ior, cos_i, cos_t)
}

/// The waves' focusing of the light that reaches `c` from the unit
/// direction `l` (towards the light): where it entered the surface the
/// wave's curvature bends the light together (under a crest: brighter)
/// or apart, more so the deeper `c` lies: 0.3..2.5, 1 for flat water.
/// The bright and dark bands it makes are the caustics and the light
/// shafts' stripes.
pub fn focus(g: &Ground, c: &Vec3, l: &Vec3) -> f64 {
    if g.wave_height == 0.0 {
        return 1.0;
    }
    let depth = g.depth(c);
    if depth <= 0.0 {
        return 1.0;
    }
    let k = g.axis as usize % 3;
    let cos_up = dot(l, &g.up()).max(0.05);
    // where the light came through the surface
    let e = add(*c, scale(*l, depth / cos_up));
    let (u, v) = (e[(k + 1) % 3], e[(k + 2) % 3]);
    let h = g.wave_len.max(1e-30) * 0.05;
    let lap = (g.wave(u + h, v) + g.wave(u - h, v) + g.wave(u, v + h) + g.wave(u, v - h) - 4.0 * g.wave(u, v)) / (h * h);
    // a crest (curved down, lap < 0) is a converging lens
    (1.0 - 1.5 * lap * depth / g.ior.max(1.0) as f64).clamp(0.3, 2.5)
}

/// The strength of the light through the wave crests.
const CREST: f64 = 1.0;

/// Translucent wave crests: the light of a lamp from the unit direction
/// `l` (towards it) that goes through the thin water of a crest and comes
/// out towards the eye (looking along the unit direction `view`), per
/// channel and per unit of the lamp's light: strongest looking towards
/// the light (forward scattering) at the top of a crest, tinted by the
/// way through the crest (about a fifth of the wave length there).
pub fn crest(g: &Ground, c: &Vec3, view: &Vec3, l: &Vec3) -> [f32; 3] {
    if g.wave_height == 0.0 || g.scattering <= 0.0 {
        return [0.0; 3];
    }
    let k = g.axis as usize % 3;
    let w = g.wave(c[(k + 1) % 3], c[(k + 2) % 3]) / g.wave_height;
    let top = ((w + 1.0) * 0.5).clamp(0.0, 1.0).powi(2);
    // the lamp low over the water shines through the crests
    let low = 1.0 - dot(l, &g.up()).clamp(0.0, 1.0);
    let ph = hg(dot(l, view), g.scatter_dir as f64);
    let path = g.wave_len * 0.2 * (1.0 - 0.5 * top);
    let a = (g.scattering as f64 * top * low * ph * CREST) as f32;
    transmittance(g, path).map(|t| t * a)
}

/// How much brighter the water's own light is seen along the ray `view`
/// (unit, under the water) than without scattering: [`glow_dir`] around
/// 1, by half for the scattering amount (0.3..3).
pub fn glow_mod(g: &Ground, view: &Vec3, lamps: &[(Vec3, f64)]) -> f64 {
    let s = g.scattering.clamp(0.0, 1.0) as f64;
    (1.0 + s * 0.5 * (glow_dir(g, view, lamps) - 1.0)).clamp(0.3, 3.0)
}

/// The view-dependent strength of the water's own (many times scattered)
/// light: the lamps' light under the surface scattered towards the eye
/// (`view`, the unit direction of the ray under the water) with half of
/// the anisotropy (scattering many times makes it rounder), weighed by
/// the lamps' `(towards, weight)`; 1 without lamps.
pub fn glow_dir(g: &Ground, view: &Vec3, lamps: &[(Vec3, f64)]) -> f64 {
    let (mut a, mut w) = (0.0, 0.0);
    for (l, k) in lamps {
        let d = light_under(g, l);
        a += k * hg(-dot(&d, view), g.scatter_dir as f64 * 0.5);
        w += k;
    }
    if w > 1e-12 {
        a / w
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn water() -> Ground {
        Ground { axis: 1, pos: 0.0, above: true, transparency: 1.0, visibility: 0.5, water_color: [10, 60, 70], ior: 1.33, ..Default::default() }
    }

    /// Fresnel: 2 % at normal incidence into water ((n-1)/(n+1))^2, rising
    /// to all of it at grazing angles; Snell's law holds; total internal
    /// reflection from the dense side.
    #[test]
    fn fresnel_of_water() {
        let (f, ct) = fresnel(1.0, 1.33);
        assert!((f - ((0.33f64 / 2.33).powi(2))).abs() < 1e-9, "{f}");
        assert!((ct - 1.0).abs() < 1e-12);
        let (f, _) = fresnel(1e-4, 1.33);
        assert!(f > 0.99, "{f}");
        // 60 degrees: sin_t = sin_i / n
        let ci = 60f64.to_radians().cos();
        let (f, ct) = fresnel(ci, 1.33);
        let st = (1.0 - ct * ct).sqrt();
        assert!((st - 60f64.to_radians().sin() / 1.33).abs() < 1e-12);
        assert!(f > 0.05 && f < 0.07, "{f}");
        // out of water at 60 degrees: beyond the critical angle (48.8)
        assert_eq!(fresnel(ci, 1.0 / 1.33), (1.0, 0.0));
        // monotonic in the angle
        let mut last = 0.0;
        for i in (1..=100).rev() {
            let (f, _) = fresnel(i as f64 / 100.0, 1.33);
            assert!(f >= last - 1e-12);
            last = f;
        }
    }

    /// Beer-Lambert: multiplicative over distances; the clearest channel
    /// keeps 1/e at the visibility; a blue-green water fades red, green
    /// and blue about 10 : 2 : 1; the visibility scales the distance.
    #[test]
    fn absorption_of_water() {
        let g = water();
        let (a, b, ab) = (transmittance(&g, 0.3), transmittance(&g, 0.5), transmittance(&g, 0.8));
        for k in 0..3 {
            assert!((a[k] * b[k] - ab[k]).abs() < 1e-6);
        }
        assert_eq!(transmittance(&g, 0.0), [1.0; 3]);
        assert!(ab[2] > ab[1] && ab[1] > ab[0]);
        assert!((transmittance(&g, 0.5)[2] - (-1f32).exp()).abs() < 1e-6);
        let e = extinction([10, 60, 70]);
        assert_eq!(e[2], 1.0);
        assert!((9.0..12.0).contains(&e[0]) && (1.5..2.5).contains(&e[1]), "{e:?}");
        assert_eq!(extinction([90, 90, 90]), [1.0; 3]);
        assert_eq!(transmittance(&g, f64::INFINITY), [0.0; 3]);
        let mut g2 = g;
        g2.visibility = 1.0;
        let h = transmittance(&g2, 1.6);
        for k in 0..3 {
            assert!((h[k] - ab[k]).abs() < 1e-6);
        }
    }

    /// The water below y = 0 (above = true: the larger y are solid):
    /// the lengths of rays in it, the light's way down, the refraction.
    #[test]
    fn rays_in_the_water() {
        let g = water();
        assert_eq!(g.up(), [0.0, -1.0, 0.0]);
        assert!((g.depth(&[0.0, 0.5, 0.0]) - 0.5).abs() < 1e-12);
        // straight down from the surface: all of it under water
        let down = [0.0, 1.0, 0.0];
        assert!((length_in_water(&g, &[0.0; 3], &down, 2.0) - 2.0).abs() < 1e-12);
        assert!(length_in_water(&g, &[0.0; 3], &down, f64::INFINITY).is_infinite());
        // up from 0.5 deep at 60 degrees: 1 unit to the surface
        let up = normalize([3f64.sqrt() / 2.0, -0.5, 0.0]);
        assert!((length_in_water(&g, &[0.0, 0.5, 0.0], &up, f64::INFINITY) - 1.0).abs() < 1e-9);
        assert!((length_in_water(&g, &[0.0, 0.5, 0.0], &up, 0.25) - 0.25).abs() < 1e-9);
        // above the water going up: none
        assert_eq!(length_in_water(&g, &[0.0, -0.5, 0.0], &up, 3.0), 0.0);
        // the light from straight above: the depth
        assert!((light_path(&g, &[0.0, 0.5, 0.0], &[0.0, -1.0, 0.0]) - 0.5).abs() < 1e-12);
        assert!(light_path(&g, &[0.0, 0.5, 0.0], &up) > 0.5);
        assert_eq!(light_path(&g, &[0.0, -0.5, 0.0], &up), 0.0);
        // a ray at 45 degrees into calm water bends towards the normal
        let v = normalize([1.0, 1.0, 0.0]);
        let (f, t) = split(&g, &[0.0; 3], &v);
        assert!(f > 0.02 && f < 0.04, "{f}");
        let sin_t = t[0] / len(&t);
        assert!((sin_t - (0.5f64.sqrt() / g.ior as f64)).abs() < 1e-12, "{t:?}");
        assert!(t[1] > 0.0);
    }

    /// Henyey-Greenstein: 1 everywhere without anisotropy, its mean over
    /// the sphere 1, forward much stronger than backward; drawn directions
    /// have the mean cosine g.
    #[test]
    fn forward_scattering() {
        assert!((hg(0.3, 0.0) - 1.0).abs() < 1e-12);
        let n = 20000;
        let mean: f64 = (0..n).map(|i| hg(-1.0 + 2.0 * (i as f64 + 0.5) / n as f64, 0.8)).sum::<f64>() / n as f64;
        assert!((mean - 1.0).abs() < 0.01, "{mean}");
        assert!(hg(1.0, 0.8) > 100.0 * hg(-1.0, 0.8));
        let w = normalize([0.3, -0.5, 0.8]);
        let mut c = 0.0;
        for i in 0..64 {
            for j in 0..64 {
                let d = sample_hg(w, 0.8, (i as f64 + 0.5) / 64.0, (j as f64 + 0.5) / 64.0);
                assert!((len(&d) - 1.0).abs() < 1e-9);
                c += dot(&d, &w);
            }
        }
        assert!((c / 4096.0 - 0.8).abs() < 0.01, "{}", c / 4096.0);
        // the sun's light under the surface goes down, bent towards the normal
        let g = water();
        let l = normalize([1.0, -1.0, 0.0]);
        let d = light_under(&g, &l);
        assert!(d[1] > 0.0 && d[0] < 0.0 && d[0].abs() < 0.71, "{d:?}");
        // looking down away from the sun the water glows less than towards it
        let lamps = [(l, 1.0)];
        let towards = normalize([1.0, 1.0, 0.0]);
        let away = normalize([-1.0, 1.0, 0.0]);
        assert!(glow_dir(&g, &towards, &lamps) > glow_dir(&g, &away, &lamps));
    }

    /// Crests glow only with waves and scattering, most at their tops and
    /// looking towards the light; the waves focus the light under crests.
    #[test]
    fn crests_and_caustics() {
        let mut g = water();
        let l = normalize([1.0, -0.4, 0.0]);
        assert_eq!(crest(&g, &[0.0; 3], &[1.0, 0.0, 0.0], &l), [0.0; 3]);
        assert_eq!(focus(&g, &[0.0, 0.3, 0.0], &l), 1.0);
        g.wave_height = 0.01;
        g.wave_len = 0.5;
        g.scattering = 0.5;
        let k = |x: f64| g.wave(x, 0.0);
        let top = (0..1000).map(|i| i as f64 * 0.001).fold(0.0, |b: f64, x| if k(x) > k(b) { x } else { b });
        let dip = (0..1000).map(|i| i as f64 * 0.001).fold(0.0, |b: f64, x| if k(x) < k(b) { x } else { b });
        let view = normalize([1.0, 0.2, 0.0]);
        let ct = crest(&g, &[0.0, 0.0, top], &view, &l);
        assert!(ct[2] > crest(&g, &[0.0, 0.0, dip], &view, &l)[2]);
        assert!(ct[2] > crest(&g, &[0.0, 0.0, top], &scale(view, -1.0), &l)[2]);
        assert!(ct[2] > ct[0], "turquoise: {ct:?}");
        let f: Vec<f64> = (0..200).map(|i| focus(&g, &[0.0, 0.4, i as f64 * 0.005], &[0.0, -1.0, 0.0])).collect();
        assert!(f.iter().cloned().fold(0.0, f64::max) > 1.05 && f.iter().cloned().fold(9.0, f64::min) < 0.95, "{f:?}");
        // scattering takes light off the straight way
        assert!(transmittance_s(&g, 0.3)[2] < transmittance(&g, 0.3)[2]);
        assert_eq!(sigma_s(&g), 1.0);
    }

    /// The waves tilt the normal; flat water's normal is the plane's.
    #[test]
    fn the_waves_tilt_the_normal() {
        let mut g = water();
        assert_eq!(g.normal(&[0.3, 0.0, 0.7]), g.up());
        g.wave_height = 0.05;
        g.wave_len = 0.5;
        let n = g.normal(&[0.3, 0.0, 0.7]);
        assert!((len(&n) - 1.0).abs() < 1e-12);
        assert!(n[1] < -0.5 && (n[0].abs() > 1e-3 || n[2].abs() > 1e-3), "{n:?}");
    }
}
