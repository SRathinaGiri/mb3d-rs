//! The "Animation templates" window (`AnimTemplatesFrm`, this program's
//! own, the Animation window's "Animation templates..." button): keyframes
//! made from the main window's view by [`crate::autopilot`], added after the
//! Animation window's keyframes:
//!
//! * 360° orbit: a whole turn around the object, the frames adding up to
//!   the duration (the Animation window's own Turntable button stays as it
//!   was);
//! * Fly-through: a flight on into the fractal for the duration, or until
//!   the view shows only sky or only fractal.
//!
//! The frames per second are the Animation window's (its video's).

use super::util::{pti, ptf};
use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;

const F: &str = "AnimTemplatesFrm";
const ANIM: &str = "AnimationForm";

/// Opens the window with the settings of last time.
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    let get = |k: &str, d: &str| app.ini.get_extra(k).filter(|v| !v.is_empty()).unwrap_or(d).to_string();
    ui.set_item_index(F, "TemplateCmb", get("AniTplKind", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    ui.set_text(F, "DurationEdit", &get("AniTplSecs", "10"));
    ui.set_text(F, "StepsEdit", &get("AniTplSteps", "8"));
    ui.set_item_index(F, "PivotCmb", get("AniTplPivot", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    ui.set_checked(F, "UntilCheck", get("AniTplUntil", "0") == "1");
    ui.set_text(F, "SpeedEdit", &get("AniTplSpeed", "4"));
    ui.set_text(F, "TurnEdit", &get("AniTplTurn", "2"));
    show_kind(ui);
    ui.show(F);
}

fn save(app: &mut Mb3d, ui: &Ui) {
    let kind = ui.item_index(F, "TemplateCmb").to_string();
    let pivot = ui.item_index(F, "PivotCmb").to_string();
    let until = if ui.checked(F, "UntilCheck") { "1" } else { "0" };
    for (k, v) in [
        ("AniTplKind", kind.as_str()),
        ("AniTplSecs", ui.text(F, "DurationEdit").trim()),
        ("AniTplSteps", ui.text(F, "StepsEdit").trim()),
        ("AniTplPivot", pivot.as_str()),
        ("AniTplUntil", until),
        ("AniTplSpeed", ui.text(F, "SpeedEdit").trim()),
        ("AniTplTurn", ui.text(F, "TurnEdit").trim()),
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
    for c in ["StepsLabel", "StepsEdit", "PivotLabel", "PivotCmb"] {
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

/// Makes the template's keyframes and adds them to the Animation window.
fn add(app: &mut Mb3d, ui: &mut Ui) {
    let fps = fps(ui);
    let until = !is_orbit(ui) && ui.checked(F, "UntilCheck");
    let secs = if until { Some(120.0) } else { seconds(ui) };
    let Some(secs) = secs else {
        ui.show_message(&tr("@MB:ValueNotGuilty"));
        return;
    };
    save(app, ui);
    app.make_scene(ui);
    let sc = app.scene.clone();
    if is_orbit(ui) {
        let steps = pti(&ui.text(F, "StepsEdit")).clamp(3, 360) as usize;
        let pivot = if ui.item_index(F, "PivotCmb") == 1 { [0.0; 3] } else { crate::autopilot::turntable_pivot(&sc) };
        let total = (secs * fps).round().max(steps as f64) as u32;
        let mut keys = crate::autopilot::orbit_keyframes_around(&sc, steps, 1, pivot);
        for (k, f) in keys.iter_mut().zip(crate::autopilot::spread_frames(total, steps)) {
            k.frames = f;
        }
        super::anim_form::add_template(app, ui, keys, true);
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
        match crate::autopilot::fly_through(&sc, &fp, &|_, _| {}, &|| false) {
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
                super::anim_form::add_template(app, ui, fl.keyframes, false);
                if let Some(w) = why {
                    app.message(ui, &trf("@MB:FlyThroughTheFlightEndedAfter", &[&fl.frames, &fp.frames, &w]));
                }
            }
            Err(e) => ui.show_message(&trf("@MB:TheFlyThroughCannotStartHere", &[&e])),
        }
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => show_kind(ui),
        "TemplateCmbChange" | "UntilCheckClick" => show_kind(ui),
        "DurationEditChange" => show_frames(ui),
        "AddBtnClick" => add(app, ui),
        "CloseBtnClick" => {
            save(app, ui);
            ui.hide(F);
        }
        _ => {}
    }
}
