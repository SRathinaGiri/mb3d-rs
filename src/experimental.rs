//! Experimental speed-ups (not in MB3D): faster ways of calculating that
//! change the picture a little, so they are off unless the scene switches
//! them on (`experimental = on`) and then each of them on its own.
//!
//! * Step relax: the march's steps enlarged by a factor (over-relaxation);
//!   a step whose distance estimate no longer overlaps the last one is
//!   taken again at its safe length.  About 10 % faster; scenes whose
//!   distance estimate is not a true bound (thin shells) can show holes.
//! * Secant search: the surface after the march is found by interpolating
//!   between the last two probes where they lie on both sides of it, not
//!   only by halving.  A few per cent faster; the point found moves a
//!   little where the distance estimate is not smooth.
//! * Free steps: without dynamic fog and volumetric light the steps are
//!   not limited to the fog's step length.  Faster in open space; not used
//!   with "disable analytic DE", where the object could be lost.
//! * Occlusion at half resolution: the DE ambient occlusion is calculated
//!   on every second pixel of every second row, the others are filled from
//!   their neighbours of a similar depth.  Up to 4 times faster on that
//!   stage, softer in fine detail.
//! * Cone march: a coarse ray per block of pixels (16 by 16, then 4 by 4)
//!   marches with the block's spread taken off the distance estimate, so
//!   it stops where the first pixel of the block could meet the surface;
//!   every pixel's march starts there instead of at the start plane
//!   (`crate::cone`).  Faster through open space; the pixels' marches
//!   can only miss what the distance estimate hid from the coarse ray,
//!   so scenes whose estimate is no true bound can show small holes.

/// The experimental settings of a scene (`Scene::experimental`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Experimental {
    /// the switch of them all: off, none of them is used
    pub on: bool,
    /// the march steps enlarged by this, 1 .. 2 (1: not)
    pub step_relax: f64,
    /// the secant search for the surface
    pub secant: bool,
    /// no step limit without dynamic fog
    pub free_steps: bool,
    /// the DE occlusion at half resolution
    pub ao_half: bool,
    /// the march of each block of pixels started where its coarse ray
    /// stopped (`crate::cone`)
    pub cone: bool,
}

impl Default for Experimental {
    fn default() -> Self {
        Experimental { on: false, step_relax: 1.0, secant: false, free_steps: false, ao_half: false, cone: false }
    }
}

impl Experimental {
    /// The march's step factor in use (1: none).
    pub fn relax(&self) -> f64 {
        if self.on {
            self.step_relax.clamp(1.0, 2.0)
        } else {
            1.0
        }
    }

    pub fn secant(&self) -> bool {
        self.on && self.secant
    }

    pub fn free_steps(&self) -> bool {
        self.on && self.free_steps
    }

    pub fn ao_half(&self) -> bool {
        self.on && self.ao_half
    }

    pub fn cone(&self) -> bool {
        self.on && self.cone
    }

    /// Any of them in use.
    pub fn active(&self) -> bool {
        self.relax() > 1.0 || self.secant() || self.free_steps() || self.ao_half() || self.cone()
    }

    /// The settings as the program keeps them between its starts (the
    /// `Experimental` item of Mandelbulb3D.ini): the switch, the step relax
    /// and the four others, as `1 1.3 0 1 1 0` (v0.2.91 wrote five).
    pub fn to_ini(&self) -> String {
        let b = |v: bool| if v { "1" } else { "0" };
        format!("{} {} {} {} {} {}", b(self.on), self.step_relax, b(self.secant), b(self.free_steps), b(self.ao_half), b(self.cone))
    }

    /// [`Experimental::to_ini`] read back (None for anything else).
    pub fn from_ini(s: &str) -> Option<Experimental> {
        let v: Vec<&str> = s.split_whitespace().collect();
        if !(5..=6).contains(&v.len()) {
            return None;
        }
        let (on, relax, secant, free, half, cone) = (v[0], v[1], v[2], v[3], v[4], v.get(5));
        let b = |x: &str| match x {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        };
        let r: f64 = relax.parse().ok().filter(|r: &f64| r.is_finite())?;
        Some(Experimental {
            on: b(on)?,
            step_relax: r.clamp(1.0, 2.0),
            secant: b(secant)?,
            free_steps: b(free)?,
            ao_half: b(half)?,
            cone: match cone {
                Some(c) => b(c)?,
                None => false,
            },
        })
    }

    /// The scene keys ("" for the defaults).
    pub fn to_text(&self) -> String {
        if *self == Self::default() {
            return String::new();
        }
        let b = |v: bool| if v { "on" } else { "off" };
        format!(
            "experimental = {}\nstep_relax = {}\nsecant_search = {}\nfree_steps = {}\nocclusion_half = {}\ncone_march = {}\n",
            b(self.on),
            self.step_relax,
            b(self.secant),
            b(self.free_steps),
            b(self.ao_half),
            b(self.cone)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_kept_between_starts_read_back() {
        let e = Experimental { on: true, step_relax: 1.3, secant: false, free_steps: true, ao_half: true, cone: true };
        assert_eq!(e.to_ini(), "1 1.3 0 1 1 1");
        assert_eq!(Experimental::from_ini(&e.to_ini()), Some(e));
        // v0.2.91 wrote five values
        assert_eq!(Experimental::from_ini("1 1.3 0 1 1"), Some(Experimental { cone: false, ..e }));
        assert_eq!(Experimental::from_ini("1 1.3 0 1 1 1 1"), None);
        assert_eq!(Experimental::from_ini(&Experimental::default().to_ini()), Some(Experimental::default()));
        assert_eq!(Experimental::from_ini(""), None);
        assert_eq!(Experimental::from_ini("1 x 0 1 1"), None);
        assert_eq!(Experimental::from_ini("1 9 0 1 1").map(|e| e.step_relax), Some(2.0));
    }

    #[test]
    fn nothing_is_used_while_switched_off() {
        let e = Experimental { on: false, step_relax: 1.4, secant: true, free_steps: true, ao_half: true, cone: true };
        assert!(!e.active());
        assert_eq!(e.relax(), 1.0);
        let on = Experimental { on: true, ..e };
        assert!(on.active() && on.secant() && on.free_steps() && on.ao_half());
        assert_eq!(on.relax(), 1.4);
        assert_eq!(Experimental::default().to_text(), "");
    }
}
