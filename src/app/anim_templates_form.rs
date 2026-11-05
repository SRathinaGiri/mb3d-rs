//! The "Animation templates" window (`AnimTemplatesFrm`, this program's
//! own, the Animation window's "Animation templates..." button): keyframes
//! made from the main window's view by [`crate::autopilot`], added after the
//! Animation window's keyframes:
//!
//! * 360° orbit: a whole turn around the object (left to right, over the
//!   top, a spin, all three at once, or around the fractal's X, Y or Z
//!   axis), the frames adding up to the duration (the Animation window's own Turntable button stays as it
//!   was);
//! * Fly-through: a flight on into the fractal for the duration, or until
//!   the view shows only sky or only fractal.
//!
//! The frames per second are the Animation window's (its video's).
//!
//! A template starts from the main window's view or from the animation's
//! last keyframe. The fly-through is flown on a thread of its own (a long
//! one took the window with it): a bar shows the frames flown, Stop ends
//! the flight there and adds what was flown.

use super::util::{pti, ptf};
use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A fly-through being flown.
#[derive(Default)]
pub struct State {
    flying: Option<Flying>,
}

struct Flying {
    done: Arc<AtomicUsize>,
    total: usize,
    cancel: Arc<AtomicBool>,
    result: Arc<Mutex<Option<Result<crate::autopilot::Flight, String>>>>,
    from_last: bool,
    asked: usize,
}

const F: &str = "AnimTemplatesFrm";
const ANIM: &str = "AnimationForm";

/// Opens the window with the settings of last time.
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    let get = |k: &str, d: &str| app.ini.get_extra(k).filter(|v| !v.is_empty()).unwrap_or(d).to_string();
    ui.set_item_index(F, "TemplateCmb", get("AniTplKind", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    ui.set_text(F, "DurationEdit", &get("AniTplSecs", "10"));
    ui.set_item_index(F, "AxisCmb", get("AniTplAxis", "0").parse::<i32>().unwrap_or(0).clamp(0, 6));
    ui.set_text(F, "StepsEdit", &get("AniTplSteps", "8"));
    ui.set_item_index(F, "PivotCmb", get("AniTplPivot", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    ui.set_checked(F, "UntilCheck", get("AniTplUntil", "0") == "1");
    ui.set_text(F, "SpeedEdit", &get("AniTplSpeed", "4"));
    ui.set_text(F, "TurnEdit", &get("AniTplTurn", "2"));
    ui.set_item_index(F, "StartCmb", get("AniTplStart", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    show_kind(ui);
    ui.show(F);
}

fn save(app: &mut Mb3d, ui: &Ui) {
    let kind = ui.item_index(F, "TemplateCmb").to_string();
    let pivot = ui.item_index(F, "PivotCmb").to_string();
    let axis = ui.item_index(F, "AxisCmb").to_string();
    let until = if ui.checked(F, "UntilCheck") { "1" } else { "0" };
    let start = ui.item_index(F, "StartCmb").to_string();
    for (k, v) in [
        ("AniTplKind", kind.as_str()),
        ("AniTplSecs", ui.text(F, "DurationEdit").trim()),
        ("AniTplSteps", ui.text(F, "StepsEdit").trim()),
        ("AniTplPivot", pivot.as_str()),
        ("AniTplAxis", axis.as_str()),
        ("AniTplUntil", until),
        ("AniTplSpeed", ui.text(F, "SpeedEdit").trim()),
        ("AniTplTurn", ui.text(F, "TurnEdit").trim()),
        ("AniTplStart", start.as_str()),
    ] {
        app.ini.set_extra(k, v);
    }
}

fn is_orbit(ui: &Ui) -> bool {
    ui.item_index(F, "TemplateCmb") != 1
}

/// The controls of the chosen template.
fn show_kind(ui: &mut Ui) {
    let orbit = is_orbit(ui);
    ui.set_caption(F, "NoteLabel", &tr(if orbit { "@MB:AWholeTurnOfTheCamera" } else { "@MB:TheCameraFliesOnIntoThe" }));
    for c in ["AxisLabel", "AxisCmb", "StepsLabel", "StepsEdit", "PivotLabel", "PivotCmb"] {
        ui.set_visible(F, c, orbit);
    }
    for c in ["UntilCheck", "SpeedLabel", "SpeedEdit", "TurnLabel", "TurnEdit"] {
        ui.set_visible(F, c, !orbit);
    }
    let until = !orbit && ui.checked(F, "UntilCheck");
    ui.set_enabled(F, "DurationEdit", !until);
    ui.set_enabled(F, "DurationLabel", !until);
    show_frames(ui);
}

/// The Animation window's frames per second (25 when it has none).
fn fps(ui: &Ui) -> f64 {
    ui.text(ANIM, "VideoFpsEdit").trim().replace(',', ".").parse::<f64>().ok().filter(|f| *f > 0.0 && *f <= 1000.0).unwrap_or(25.0)
}

/// The duration typed in, above 0.
fn seconds(ui: &Ui) -> Option<f64> {
    ui.text(F, "DurationEdit").trim().replace(',', ".").parse::<f64>().ok().filter(|d| d.is_finite() && *d > 0.0 && *d <= 36_000.0)
}

fn show_frames(ui: &mut Ui) {
    let until = !is_orbit(ui) && ui.checked(F, "UntilCheck");
    let f = fps(ui);
    let text = match (until, seconds(ui)) {
        (true, _) => String::new(),
        (false, Some(s)) => trf("@MB:FramesAtFramesASecond", &[&((s * f).round() as i64), &f]),
        (false, None) => tr("@MB:ValueNotGuilty"),
    };
    ui.set_caption(F, "FramesLabel", &text);
}

/// The view a template starts from: the last keyframe's (when chosen and
/// there is one; true) or the main window's.
fn start_scene(app: &mut Mb3d, ui: &mut Ui) -> (crate::scene::Scene, bool) {
    if ui.item_index(F, "StartCmb") == 1 {
        if let Some(sc) = super::anim_form::last_keyframe_scene(app) {
            return (sc, true);
        }
    }
    app.make_scene(ui);
    (app.scene.clone(), false)
}

/// Makes the template's keyframes and adds them to the Animation window.
fn add(app: &mut Mb3d, ui: &mut Ui) {
    if app.anim_tpl.flying.is_some() {
        return;
    }
    let fps = fps(ui);
    let until = !is_orbit(ui) && ui.checked(F, "UntilCheck");
    let secs = if until { Some(120.0) } else { seconds(ui) };
    let Some(secs) = secs else {
        ui.show_message(&tr("@MB:ValueNotGuilty"));
        return;
    };
    save(app, ui);
    let (sc, from_last) = start_scene(app, ui);
    if is_orbit(ui) {
        let steps = pti(&ui.text(F, "StepsEdit")).clamp(3, 360) as usize;
        let pivot = if ui.item_index(F, "PivotCmb") == 1 { [0.0; 3] } else { crate::autopilot::turntable_pivot(&sc) };
        let total = (secs * fps).round().max(steps as f64) as u32;
        let axis = crate::autopilot::OrbitAxis::ALL[ui.item_index(F, "AxisCmb").clamp(0, 6) as usize];
        let mut keys = crate::autopilot::orbit_keyframes_axis(&sc, steps, 1, pivot, axis);
        for (k, f) in keys.iter_mut().zip(crate::autopilot::spread_frames(total, steps)) {
            k.frames = f;
        }
        super::anim_form::add_template(app, ui, keys, true, from_last);
    } else {
        let speed = ptf(&ui.text(F, "SpeedEdit"));
        let turn = ptf(&ui.text(F, "TurnEdit"));
        let fp = crate::autopilot::FlightParams {
            frames: (secs * fps).round().max(2.0) as usize,
            keyframe_every: (fps * 0.5).round().max(1.0) as usize,
            speed: if speed.is_finite() { (speed / 100.0).clamp(0.002, 0.5) } else { 0.04 },
            max_turn_deg: if turn.is_finite() { turn.clamp(0.1, 30.0) } else { 2.0 },
            until_view: until,
            ..Default::default()
        };
        let fl = Flying {
            done: Arc::new(AtomicUsize::new(0)),
            total: fp.frames,
            cancel: Arc::new(AtomicBool::new(false)),
            result: Arc::new(Mutex::new(None)),
            from_last,
            asked: fp.frames,
        };
        let (done, cancel, result, waker) = (fl.done.clone(), fl.cancel.clone(), fl.result.clone(), ui.waker());
        std::thread::spawn(move || {
            let w = waker.clone();
            let progress = move |d: usize, _: usize| {
                // (the window shown anew some 25 times a flight)
                let old = done.swap(d, Ordering::Relaxed);
                if let Some(w) = &w {
                    if d * 25 / fp.frames.max(1) != old * 25 / fp.frames.max(1) {
                        w.wake();
                    }
                }
            };
            let r = crate::autopilot::fly_through(&sc, &fp, &progress, &|| cancel.load(Ordering::Relaxed));
            *result.lock().unwrap() = Some(r);
            if let Some(w) = waker {
                w.wake();
            }
        });
        app.anim_tpl.flying = Some(fl);
        show_flying(app, ui);
    }
}

/// The controls while a flight is flown (Add becomes Stop).
fn show_flying(app: &Mb3d, ui: &mut Ui) {
    let fl = app.anim_tpl.flying.as_ref();
    ui.set_visible(F, "FlightBar", fl.is_some());
    ui.set_caption(F, "AddBtn", &tr(if fl.is_some() { "@MB:Stop" } else { "@MB:AddTheKeyframes" }));
    for c in ["TemplateCmb", "DurationEdit", "UntilCheck", "SpeedEdit", "TurnEdit", "StartCmb"] {
        ui.set_enabled(F, c, fl.is_none());
    }
    match fl {
        Some(fl) => {
            let d = fl.done.load(Ordering::Relaxed);
            let c = ui.cm(F, "FlightBar");
            c.max = fl.total.max(1) as i64;
            c.position = d.min(fl.total) as i64;
            ui.set_caption(F, "FlightLabel", &trf("@MB:FlyingFrameOf", &[&d, &fl.total]));
        }
        None => {
            ui.set_caption(F, "FlightLabel", "");
            let until = !is_orbit(ui) && ui.checked(F, "UntilCheck");
            ui.set_enabled(F, "DurationEdit", !until);
        }
    }
}

/// The flight's progress, and its keyframes added when it has ended.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    let Some(fl) = &app.anim_tpl.flying else { return };
    let r = fl.result.lock().unwrap().take();
    let Some(r) = r else {
        show_flying(app, ui);
        return;
    };
    let (from_last, asked) = (fl.from_last, fl.asked);
    app.anim_tpl.flying = None;
    show_flying(app, ui);
    match r {
        Ok(fl) => {
            use crate::autopilot::FlightEnd;
            let why = match fl.end {
                FlightEnd::Done => None,
                FlightEnd::DeadEnd => Some(tr("@MB:NoWayAheadWasLeft")),
                FlightEnd::Precision => Some(tr("@MB:TheStepsCameToTheLimit")),
                FlightEnd::LeftFractal => Some(tr("@MB:TheCameraLeftTheFractal")),
                FlightEnd::OnlySky => Some(tr("@MB:TheViewShowsOnlySky")),
                FlightEnd::OnlyFractal => Some(tr("@MB:TheViewIsFilledWithThe")),
            };
            let flown = fl.frames;
            super::anim_form::add_template(app, ui, fl.keyframes, false, from_last);
            if let Some(w) = why {
                app.message(ui, &trf("@MB:FlyThroughTheFlightEndedAfter", &[&flown, &asked, &w]));
            }
        }
        Err(e) => ui.show_message(&trf("@MB:TheFlyThroughCannotStartHere", &[&e])),
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => show_kind(ui),
        "TemplateCmbChange" | "UntilCheckClick" => show_kind(ui),
        "DurationEditChange" => show_frames(ui),
        "AddBtnClick" => match &app.anim_tpl.flying {
            // Stop: the flight ends at its next frame, what was flown is added
            Some(fl) => fl.cancel.store(true, Ordering::Relaxed),
            None => add(app, ui),
        },
        "CloseBtnClick" => {
            if let Some(fl) = &app.anim_tpl.flying {
                fl.cancel.store(true, Ordering::Relaxed);
            }
            save(app, ui);
            ui.hide(F);
        }
        _ => {}
    }
}
