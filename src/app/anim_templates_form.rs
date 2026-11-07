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
//!   the view shows only sky or only fractal;
//! * Fly to the view: from outside the fractal (all of it in view) or from
//!   the animation's last keyframe to the main window's view, on the
//!   straightest free way, round what is in the way and back onto the
//!   course, arriving exactly at the view ([`crate::approach`]);
//! * Zoom out: Fly to the view backwards, from the main window's view or
//!   the last keyframe to outside the fractal.
//!
//! Both flights can roll: the picture turns around the line of sight at
//! the degrees a second given (Fly to the view: whole turns, so that it
//! still arrives exactly at the view).
//!
//! VR comfort (on at first when the Animation window's 360° VR is): the
//! view never turns by itself (the orbit goes round with one rotation, the
//! flights fly the same way without turning), and no roll.
//!
//! The frames per second are the Animation window's (its video's).
//!
//! A template starts from the main window's view or from the animation's
//! last keyframe (Fly to the view: from outside the fractal or from the
//! last keyframe, to the main window's view). The flights are flown on a thread of its own (a long
//! one took the window with it): a bar shows the frames flown, Stop ends
//! the flight there and adds what was flown.
//!
//! Fly to the view can start near the view instead of outside: as far back
//! as the flight goes at the speed in the duration (slow: near, fast: far);
//! Zoom out likewise ends as far out as the speed goes.
//!
//! The keyframes made play as a small preview in the window (50 pixels
//! wide, without shadows, ambient occlusion, reflections, volume light or
//! depth of field), rendered on a thread while it plays.

use super::util::{pti, ptf};
use super::{Mb3d, Ui};
use crate::vcl::lang::{tr, trf};
use crate::vcl::Event;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A fly-through being flown, and the preview of the keyframes made.
#[derive(Default)]
pub struct State {
    flying: Option<Flying>,
    preview: Option<Preview>,
}

/// The small preview: its frames as they are rendered, played in real time.
struct Preview {
    frames: Arc<Mutex<Vec<Option<crate::vcl::bitmap::Bitmap>>>>,
    stop: Arc<AtomicBool>,
    /// the preview's frames a second (the animation's over the frames left out)
    fps: f64,
    started: Instant,
    shown: Option<usize>,
}

/// The preview's width in pixels (shown twice as large).
const PREVIEW_W: f64 = 50.0;
/// At most this many frames are rendered for the preview.
const PREVIEW_FRAMES: usize = 150;

struct Flying {
    done: Arc<AtomicUsize>,
    total: usize,
    cancel: Arc<AtomicBool>,
    result: Arc<Mutex<Option<Result<crate::autopilot::Flight, String>>>>,
    from_last: bool,
    asked: usize,
    /// Fly to the view or zoom out (else the fly-through)
    to_view: bool,
    /// Zoom out
    out: bool,
}

const F: &str = "AnimTemplatesFrm";
const ANIM: &str = "AnimationForm";

/// Opens the window with the settings of last time.
pub fn open(app: &mut Mb3d, ui: &mut Ui) {
    let get = |k: &str, d: &str| app.ini.get_extra(k).filter(|v| !v.is_empty()).unwrap_or(d).to_string();
    ui.set_item_index(F, "TemplateCmb", get("AniTplKind", "0").parse::<i32>().unwrap_or(0).clamp(0, 3));
    ui.set_text(F, "DurationEdit", &get("AniTplSecs", "10"));
    ui.set_item_index(F, "AxisCmb", get("AniTplAxis", "0").parse::<i32>().unwrap_or(0).clamp(0, 6));
    ui.set_text(F, "StepsEdit", &get("AniTplSteps", "8"));
    ui.set_item_index(F, "PivotCmb", get("AniTplPivot", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    ui.set_checked(F, "UntilCheck", get("AniTplUntil", "0") == "1");
    ui.set_text(F, "SpeedEdit", &get("AniTplSpeed", "4"));
    ui.set_text(F, "TurnEdit", &get("AniTplTurn", "2"));
    ui.set_item_index(F, "StartCmb", get("AniTplStart", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    ui.set_text(F, "RollEdit", &get("AniTplRoll", "0"));
    let vr = if ui.checked(ANIM, "Vr360Check") { "1" } else { "0" };
    ui.set_checked(F, "ComfortCheck", get("AniTplComfort", vr) == "1");
    ui.set_item_index(F, "FromCmb", get("AniTplFrom", "0").parse::<i32>().unwrap_or(0).clamp(0, 2));
    ui.set_item_index(F, "ToCmb", get("AniTplTo", "0").parse::<i32>().unwrap_or(0).clamp(0, 1));
    show_kind(ui);
    ui.show(F);
}

fn save(app: &mut Mb3d, ui: &Ui) {
    let kind = ui.item_index(F, "TemplateCmb").to_string();
    let pivot = ui.item_index(F, "PivotCmb").to_string();
    let axis = ui.item_index(F, "AxisCmb").to_string();
    let until = if ui.checked(F, "UntilCheck") { "1" } else { "0" };
    let start = ui.item_index(F, "StartCmb").to_string();
    let from = ui.item_index(F, "FromCmb").to_string();
    let to = ui.item_index(F, "ToCmb").to_string();
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
        ("AniTplFrom", from.as_str()),
        ("AniTplTo", to.as_str()),
        ("AniTplRoll", ui.text(F, "RollEdit").trim()),
        ("AniTplComfort", if ui.checked(F, "ComfortCheck") { "1" } else { "0" }),
    ] {
        app.ini.set_extra(k, v);
    }
}

fn is_orbit(ui: &Ui) -> bool {
    !matches!(ui.item_index(F, "TemplateCmb"), 1..=3)
}

fn is_fly(ui: &Ui) -> bool {
    ui.item_index(F, "TemplateCmb") == 1
}

fn is_to_view(ui: &Ui) -> bool {
    ui.item_index(F, "TemplateCmb") == 2
}

fn is_out(ui: &Ui) -> bool {
    ui.item_index(F, "TemplateCmb") == 3
}

/// Fly to the view from near the view, or zoom out as far as the speed
/// goes: the flight's length from the speed and the duration.
fn by_speed(ui: &Ui) -> bool {
    (is_to_view(ui) && ui.item_index(F, "FromCmb") == 2) || (is_out(ui) && ui.item_index(F, "ToCmb") == 1)
}

/// The speed typed in, % of the distance a frame (4 when not a number).
fn speed_pct(ui: &Ui) -> f64 {
    let v = ptf(&ui.text(F, "SpeedEdit"));
    if v.is_finite() { v.clamp(0.2, 50.0) } else { 4.0 }
}

/// "Until the view shows only sky ..." (the fly-through's).
fn until(ui: &Ui) -> bool {
    is_fly(ui) && ui.checked(F, "UntilCheck")
}

/// The controls of the chosen template.
fn show_kind(ui: &mut Ui) {
    let (orbit, fly, to_view) = (is_orbit(ui), is_fly(ui), is_to_view(ui));
    let note = if orbit {
        "@MB:AWholeTurnOfTheCamera"
    } else if fly {
        "@MB:TheCameraFliesOnIntoThe"
    } else if to_view {
        "@MB:TheCameraFliesFromOutsideThe"
    } else {
        "@MB:TheCameraFliesBackFromThe"
    };
    ui.set_caption(F, "NoteLabel", &tr(note));
    for c in ["AxisLabel", "AxisCmb", "StepsLabel", "StepsEdit", "PivotLabel", "PivotCmb"] {
        ui.set_visible(F, c, orbit);
    }
    for c in ["UntilCheck", "TurnLabel", "TurnEdit"] {
        ui.set_visible(F, c, fly);
    }
    let speed = fly || by_speed(ui);
    for c in ["SpeedLabel", "SpeedEdit"] {
        ui.set_visible(F, c, speed);
    }
    for c in ["ToLabel", "ToCmb"] {
        ui.set_visible(F, c, is_out(ui));
    }
    for c in ["RollLabel", "RollEdit", "RollNoteLabel"] {
        ui.set_visible(F, c, !orbit);
        ui.set_enabled(F, c, !comfort(ui));
    }
    ui.set_visible(F, "StartCmb", !to_view);
    ui.set_visible(F, "FromCmb", to_view);
    let until = until(ui);
    ui.set_enabled(F, "DurationEdit", !until);
    ui.set_enabled(F, "DurationLabel", !until);
    show_frames(ui);
}

/// The Animation window's frames per second (25 when it has none).
fn fps(ui: &Ui) -> f64 {
    ui.text(ANIM, "VideoFpsEdit").trim().replace(',', ".").parse::<f64>().ok().filter(|f| *f > 0.0 && *f <= 1000.0).unwrap_or(25.0)
}

/// VR comfort: the view never turns by itself.
fn comfort(ui: &Ui) -> bool {
    ui.checked(F, "ComfortCheck")
}

/// The roll typed in, degrees a second (0 when none or not a number; none
/// with VR comfort).
fn roll(ui: &Ui) -> f64 {
    if comfort(ui) {
        return 0.0;
    }
    let r = ptf(&ui.text(F, "RollEdit"));
    if r.is_finite() { r.clamp(-720.0, 720.0) } else { 0.0 }
}

/// The duration typed in, above 0.
fn seconds(ui: &Ui) -> Option<f64> {
    ui.text(F, "DurationEdit").trim().replace(',', ".").parse::<f64>().ok().filter(|d| d.is_finite() && *d > 0.0 && *d <= 36_000.0)
}

fn show_frames(ui: &mut Ui) {
    let until = until(ui);
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
    let until = until(ui);
    let secs = if until { Some(120.0) } else { seconds(ui) };
    let Some(secs) = secs else {
        ui.show_message(&tr("@MB:ValueNotGuilty"));
        return;
    };
    save(app, ui);
    if is_to_view(ui) || is_out(ui) {
        return add_to_view(app, ui, secs, fps);
    }
    let (sc, from_last) = start_scene(app, ui);
    if is_orbit(ui) {
        let steps = pti(&ui.text(F, "StepsEdit")).clamp(3, 360) as usize;
        let steps = if comfort(ui) { steps.max(crate::autopilot::STEADY_ORBIT_STEPS) } else { steps };
        let pivot = if ui.item_index(F, "PivotCmb") == 1 { [0.0; 3] } else { crate::autopilot::turntable_pivot(&sc) };
        let total = (secs * fps).round().max(steps as f64) as u32;
        let axis = crate::autopilot::OrbitAxis::ALL[ui.item_index(F, "AxisCmb").clamp(0, 6) as usize];
        let mut keys = crate::autopilot::orbit_keyframes_axis(&sc, steps, 1, pivot, axis);
        for (k, f) in keys.iter_mut().zip(crate::autopilot::spread_frames(total, steps)) {
            k.frames = f;
        }
        if comfort(ui) {
            let rows = keys[0].scene.vgrads;
            crate::autopilot::steady_keyframes(&mut keys, rows);
        }
        start_preview(app, ui, &keys, true);
        super::anim_form::add_template(app, ui, keys, true, from_last);
    } else {
        let speed = ptf(&ui.text(F, "SpeedEdit"));
        let turn = ptf(&ui.text(F, "TurnEdit"));
        let fp = crate::autopilot::FlightParams {
            frames: (secs * fps).round().max(2.0) as usize,
            keyframe_every: crate::autopilot::roll_keyframe_every((fps * 0.5).round().max(1.0) as usize, roll(ui), fps),
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
            to_view: false,
            out: false,
        };
        let (done, cancel, result, waker) = (fl.done.clone(), fl.cancel.clone(), fl.result.clone(), ui.waker());
        let (roll, steady) = (roll(ui), comfort(ui));
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
            let r = crate::autopilot::fly_through(&sc, &fp, &progress, &|| cancel.load(Ordering::Relaxed)).map(|mut f| {
                crate::autopilot::roll_keyframes(&mut f.keyframes, roll, fps, false);
                if steady && !f.keyframes.is_empty() {
                    let rows = f.keyframes[0].scene.vgrads;
                    crate::autopilot::steady_keyframes(&mut f.keyframes, rows);
                }
                f
            });
            *result.lock().unwrap() = Some(r);
            if let Some(w) = waker {
                w.wake();
            }
        });
        app.anim_tpl.flying = Some(fl);
        show_flying(app, ui);
    }
}

/// Fly to the view (from outside the fractal or the last keyframe to the
/// main window's view) or zoom out (from the main window's view or the
/// last keyframe to outside), on a thread like the fly-through.
fn add_to_view(app: &mut Mb3d, ui: &mut Ui, secs: f64, fps: f64) {
    let out = is_out(ui);
    let (target, start, from_last) = if out {
        let (sc, from_last) = start_scene(app, ui);
        (sc, None, from_last)
    } else {
        app.make_scene(ui);
        let start = if ui.item_index(F, "FromCmb") == 1 { super::anim_form::last_keyframe_scene(app) } else { None };
        let from_last = start.is_some();
        (app.scene.clone(), start, from_last)
    };
    let roll = roll(ui);
    let frames = (secs * fps).round().max(2.0) as usize;
    let ap = crate::approach::ApproachParams {
        frames,
        keyframe_every: crate::autopilot::roll_keyframe_every((fps * 0.5).round().max(1.0) as usize, roll, fps),
        steady: comfort(ui),
        reach: by_speed(ui).then(|| speed_pct(ui) / 100.0 * frames as f64),
        ..Default::default()
    };
    let fl = Flying {
        done: Arc::new(AtomicUsize::new(0)),
        total: ap.frames,
        cancel: Arc::new(AtomicBool::new(false)),
        result: Arc::new(Mutex::new(None)),
        from_last,
        asked: ap.frames,
        to_view: true,
        out,
    };
    let (done, cancel, result, waker) = (fl.done.clone(), fl.cancel.clone(), fl.result.clone(), ui.waker());
    std::thread::spawn(move || {
        let w = waker.clone();
        let total = ap.frames.max(1);
        let progress = move |d: usize, _: usize| {
            let old = done.swap(d, Ordering::Relaxed);
            if let Some(w) = &w {
                if d * 25 / total != old * 25 / total {
                    w.wake();
                }
            }
        };
        let stop = || cancel.load(Ordering::Relaxed);
        let r = if out { crate::approach::fly_out(&target, &ap, &progress, &stop) } else { crate::approach::fly_to(&target, start.as_ref(), &ap, &progress, &stop) };
        let r = r.map(|a| {
            let mut f = a.flight;
            // (flying to a view whole turns, so that it arrives exactly)
            crate::autopilot::roll_keyframes(&mut f.keyframes, roll, fps, !out);
            f
        });
        *result.lock().unwrap() = Some(r);
        if let Some(w) = waker {
            w.wake();
        }
    });
    app.anim_tpl.flying = Some(fl);
    show_flying(app, ui);
}

/// The controls while a flight is flown (Add becomes Stop).
fn show_flying(app: &Mb3d, ui: &mut Ui) {
    let fl = app.anim_tpl.flying.as_ref();
    ui.set_visible(F, "FlightBar", fl.is_some());
    ui.set_caption(F, "AddBtn", &tr(if fl.is_some() { "@MB:Stop" } else { "@MB:AddTheKeyframes" }));
    for c in ["TemplateCmb", "DurationEdit", "UntilCheck", "SpeedEdit", "TurnEdit", "StartCmb", "FromCmb", "ToCmb", "RollEdit", "ComfortCheck"] {
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
            let until = until(ui);
            ui.set_enabled(F, "DurationEdit", !until);
            ui.set_enabled(F, "RollEdit", !comfort(ui));
        }
    }
}

/// The preview of the template's keyframes: small frames rendered on a
/// thread (the Animation window's size and 360° VR, one eye), played as
/// they come.
fn start_preview(app: &mut Mb3d, ui: &mut Ui, keys: &[crate::anim::Keyframe], looped: bool) {
    stop_preview(app, ui);
    if keys.len() < 2 {
        return;
    }
    let mut a = app.anim.anim.clone();
    a.vr360 = ui.checked(ANIM, "Vr360Check");
    if a.width < 1 || a.height < 1 {
        a.set_size_from(&keys[0].scene);
    }
    a.keyframes = keys.to_vec();
    for k in a.keyframes.iter_mut() {
        k.scene.stereo_mode = 0;
    }
    a.looped = looped;
    a.interpolation = crate::anim::Interpolation::CatmullRom;
    a.scale = 1;
    a.motion_blur = 0;
    let (w, h) = if a.vr360 { (2.0, 1.0) } else { (a.width as f64, a.height as f64) };
    let total = a.frame_count();
    if total == 0 {
        return;
    }
    let step = total.div_ceil(PREVIEW_FRAMES).max(1);
    let list: Vec<usize> = (0..total).step_by(step).collect();
    // shown twice as large, at most 100 x 100
    let (bw, bh) = if w >= h { (100, (100.0 * h / w).round().max(1.0) as i32) } else { ((100.0 * w / h).round().max(1.0) as i32, 100) };
    let c = ui.cm(F, "PreviewImage");
    c.set_bounds(12, 360, bw, bh);
    c.stretch = true;
    ui.set_picture(F, "PreviewImage", None);
    let frames = Arc::new(Mutex::new(vec![None; list.len()]));
    let stop = Arc::new(AtomicBool::new(false));
    let run = crate::frames::FrameRun { preview: PREVIEW_W / a.width.max(1) as f64, ..Default::default() };
    let (fr, st, waker) = (frames.clone(), stop.clone(), ui.waker());
    std::thread::spawn(move || {
        let cancel = || st.load(Ordering::Relaxed);
        for (k, &f) in list.iter().enumerate() {
            if cancel() {
                return;
            }
            let r = crate::frames::frame_render_scene(&a, f, &run).and_then(|mut s| {
                s.stereo_mode = 0;
                s.shadows = None;
                s.mc.reflections = false;
                s.vol_light = None;
                s.deao = None;
                s.ao = None;
                s.dof = None;
                crate::frames::render_scaled(&s, 1, false, false, &|_, _| {}, &cancel)
            });
            let Ok(img) = r else { return };
            fr.lock().unwrap()[k] = Some(crate::vcl::bitmap::Bitmap::from_rgb(img.width, img.height, &img.rgb));
            if let Some(w) = &waker {
                w.wake();
            }
        }
    });
    app.anim_tpl.preview = Some(Preview { frames, stop, fps: fps(ui) / step as f64, started: Instant::now(), shown: None });
    ui.set_timer(F, "PreviewTimer", true);
}

fn stop_preview(app: &mut Mb3d, ui: &mut Ui) {
    if let Some(p) = app.anim_tpl.preview.take() {
        p.stop.store(true, Ordering::Relaxed);
    }
    ui.set_timer(F, "PreviewTimer", false);
}

/// The preview's next frame, in real time (looping over the frames there).
fn preview_tick(app: &mut Mb3d, ui: &mut Ui) {
    let Some(p) = &mut app.anim_tpl.preview else { return };
    let fr = p.frames.lock().unwrap();
    let ready = fr.iter().take_while(|f| f.is_some()).count();
    if ready == 0 {
        return;
    }
    let k = (p.started.elapsed().as_secs_f64() * p.fps) as usize % ready;
    if p.shown == Some(k) {
        return;
    }
    let b = fr[k].clone();
    drop(fr);
    p.shown = Some(k);
    ui.set_picture(F, "PreviewImage", b);
}

/// The flight's progress, and its keyframes added when it has ended.
pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    let Some(fl) = &app.anim_tpl.flying else { return };
    let r = fl.result.lock().unwrap().take();
    let Some(r) = r else {
        show_flying(app, ui);
        return;
    };
    let (from_last, asked, to_view, out) = (fl.from_last, fl.asked, fl.to_view, fl.out);
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
            start_preview(app, ui, &fl.keyframes, false);
            super::anim_form::add_template(app, ui, fl.keyframes, false, from_last);
            if let Some(w) = why {
                app.message(ui, &trf("@MB:FlyThroughTheFlightEndedAfter", &[&flown, &asked, &w]));
            }
        }
        // (a stopped flight to the view adds nothing: it would not arrive)
        Err(e) if to_view && e == "cancelled" => {}
        Err(e) if out => ui.show_message(&trf("@MB:NoWayOutOfTheFractal", &[&e])),
        Err(e) if to_view => ui.show_message(&trf("@MB:TheViewCannotBeReached", &[&e])),
        Err(e) => ui.show_message(&trf("@MB:TheFlyThroughCannotStartHere", &[&e])),
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "FormShow" => show_kind(ui),
        "TemplateCmbChange" | "UntilCheckClick" | "ComfortCheckClick" | "FromCmbChange" | "ToCmbChange" => show_kind(ui),
        "PreviewTimerTimer" => preview_tick(app, ui),
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
            stop_preview(app, ui);
            ui.hide(F);
        }
        _ => {}
    }
}
