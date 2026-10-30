//! The animation maker (Animation.pas, `TAnimationForm`), the animation
//! preview (AniPreviewWindow.pas) and the keyframe processing window
//! (AniProcess.pas).  Keyframes are complete parameter sets with the
//! number of subframes to the next one; the frames are interpolated with
//! MB3D's functions ([`crate::anim`]) and written into the output folder.

use crate::vcl::lang::{tr, trf};
use super::eta::{ItemsLeft, Pace};
use super::util::{fts_single, left_str, parse_float, pti, ptf, smooth_shown, Shown};
use super::{Mb3d, MAIN};
use crate::anim::{Animation, Interpolation, Keyframe, OutputFormat};
use crate::frames::{FrameResult, FrameRun};
use crate::scene::Scene;
use crate::vcl::bitmap::Bitmap;
use crate::vcl::{DialogResult, Ev, Event, MouseButton, Ui};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const F: &str = "AnimationForm";
const PV: &str = "AniPreviewForm";
const PR: &str = "AniProcessForm";

/// The state of a background job (frame rendering, preview, thumbnails).
#[derive(Default)]
pub struct Job {
    pub running: bool,
    pub done: usize,
    pub total: usize,
    pub error: String,
    pub last: Option<(Vec<u8>, usize, usize)>,
    pub frames: Vec<Option<Bitmap>>,
    pub thumbs: Vec<(usize, u64, Bitmap)>,
    pub changed: bool,
    /// the frames are done, the video is being made
    pub making_video: bool,
    /// the video written ("Also make a video")
    pub video: Option<std::path::PathBuf>,
}

fn save_output_settings(app: &mut Mb3d, ui: &Ui) {
    app.ini.set_extra("AniTransparent", if ui.checked(F, "TransparentCheck") { "1" } else { "0" });
    app.ini.set_extra("AniVideo", if ui.checked(F, "VideoCheck") { "1" } else { "0" });
    app.ini.set_extra("AniVideoFps", ui.text(F, "VideoFpsEdit").trim());
}

/// The frames per second of the video and the ffmpeg command.
fn video_fps(ui: &Ui) -> Result<f64, String> {
    ui.text(F, "VideoFpsEdit").trim().replace(',', ".").parse::<f64>().ok().filter(|f| *f > 0.0 && *f <= 1000.0).ok_or_else(|| tr("@MB:TheFramesPerSecondMustBe"))
}

/// A video needs images, one per frame.
fn video_check(a: &Animation) -> Result<(), String> {
    if a.format == OutputFormat::M3p {
        return Err(tr("@MB:AVideoNeedsImagesChooseBMP"));
    }
    if a.stereo_eyes().len() != 1 {
        return Err(tr("@MB:AVideoOfAStereoAnimation"));
    }
    Ok(())
}

/// "Also make a video": the AVI in the output folder, named after the
/// project, and its frame rate; None without the option.
fn video_settings(ui: &Ui, a: &Animation) -> Result<Option<(std::path::PathBuf, f64)>, String> {
    if !ui.checked(F, "VideoCheck") {
        return Ok(None);
    }
    video_check(a)?;
    let fps = video_fps(ui)?;
    Ok(Some((std::path::Path::new(&a.output_folder).join(format!("{}.avi", a.name)), fps)))
}

/// The animation to render with this window's settings, its range and
/// its frames (`start_rendering`, the ffmpeg command).
fn output_plan(app: &mut Mb3d, ui: &Ui) -> Result<(Animation, FrameRun, Vec<usize>), String> {
    read_settings(app, ui);
    let mut a = app.anim.anim.clone();
    if a.keyframes.len() < 2 {
        return Err(tr("@MB:TheAnimationNeedsAtLeastTwo"));
    }
    if a.output_folder.trim().is_empty() {
        a.output_folder = app.ini.dir(super::ini::DIR_ANIOUT).display().to_string();
    }
    let to = ui.text(F, "Edit11").trim().parse::<i64>().ok();
    let from = ui.text(F, "Edit8").trim().parse::<i64>().ok();
    // (a stopped animation continues in the middle of its frame)
    let run = FrameRun { from_index: from, to_index: to, resume: crate::checkpoint::Policy::Continue, ..Default::default() };
    let frames = run.frames(&a);
    if frames.is_empty() {
        return Err(tr("@MB:NoFramesToRenderInThis"));
    }
    Ok((a, run, frames))
}

/// `FfmpegBtnClick`: the ffmpeg command for a small video of the frames,
/// to the clipboard and the messages (run by the user: this program
/// starts no other programs).
fn ffmpeg_command(app: &mut Mb3d, ui: &mut Ui) {
    let (mut a, _, frames) = match output_plan(app, ui) {
        Ok(p) => p,
        Err(e) => return ui.show_message(&e),
    };
    a.transparent = ui.checked(F, "TransparentCheck") && a.format == OutputFormat::Png;
    if let Err(e) = video_check(&a) {
        return ui.show_message(&e);
    }
    let fps = match video_fps(ui) {
        Ok(f) => f,
        Err(e) => return ui.show_message(&e),
    };
    let step = if frames.len() > 1 { a.file_index(frames[1]) - a.file_index(frames[0]) } else { 1 };
    let cmd = crate::video::ffmpeg_command(std::path::Path::new(&a.output_folder), &a.name, a.format.extension(), a.file_index(frames[0]), step, fps, a.transparent);
    crate::vcl::clipboard::set(&cmd);
    app.message(ui, &tr("@MB:AnimationTheFfmpegCommandIsOn"));
    app.message(ui, &cmd);
}

pub struct State {
    pub anim: Animation,
    pub thumbs: Vec<Option<Bitmap>>,
    /// keyframe ids (to match finished thumbnails)
    pub ids: Vec<u64>,
    pub next_id: u64,
    pub current: usize,
    pub job: Arc<Mutex<Job>>,
    pub stop: Arc<AtomicBool>,
    pub pause: Arc<AtomicBool>,
    pub progress: Arc<AtomicU32>,
    pub started: Instant,
    /// the time left: the frames' times (the clock's between the frames
    /// finished, the swarm's come several at once), the running frame's
    /// pace and when it began, and the time left as shown
    pub items: ItemsLeft,
    pub pace: Pace,
    pub frame_start: Instant,
    pub last_done: usize,
    pub left_shown: Option<Shown>,
    pub first_show: bool,
    pub drag: Option<(i32, i32)>,
    pub preview_step: usize,
    pub preview_start: usize,
    pub play_frame: usize,
    pub playing: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            anim: Animation::default(),
            thumbs: Vec::new(),
            ids: Vec::new(),
            next_id: 1,
            current: 1,
            job: Arc::new(Mutex::new(Job::default())),
            stop: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicU32::new(0)),
            started: Instant::now(),
            items: ItemsLeft::default(),
            pace: Pace::default(),
            frame_start: Instant::now(),
            last_done: 0,
            left_shown: None,
            first_show: true,
            drag: None,
            preview_step: 1,
            preview_start: 1,
            play_frame: 0,
            playing: false,
        }
    }
}

fn count(app: &Mb3d) -> usize {
    app.anim.anim.keyframes.len()
}

/// The animation settings of the window into `anim` (size, output, ...).
fn read_settings(app: &mut Mb3d, ui: &Ui) {
    let a = &mut app.anim.anim;
    a.width = pti(&ui.text(F, "Edit6")).max(8);
    a.height = pti(&ui.text(F, "Edit7")).max(8);
    a.scale = ui.position(F, "UpDown3").clamp(1, 3) as u32;
    a.interpolation = if ui.item_index(F, "RadioGroup2") == 0 { Interpolation::Linear } else { Interpolation::Bezier };
    a.looped = ui.checked(F, "CheckBox2");
    a.format = match ui.item_index(F, "RadioGroup3") {
        0 => OutputFormat::Bmp,
        2 => OutputFormat::Jpg,
        3 => OutputFormat::M3p,
        _ => OutputFormat::Png,
    };
    a.output_folder = ui.text(F, "Edit2");
    a.start_index = pti(&ui.text(F, "Edit3")) as i64;
    a.index_step = pti(&ui.text(F, "Edit4")).max(1) as i64;
    a.overwrite = ui.checked(F, "CheckBox6");
    a.save_depth = ui.checked(F, "CheckBox7");
    a.transparent = ui.checked(F, "TransparentCheck") && a.format == OutputFormat::Png;
    a.stereo_bits = if ui.checked(F, "CheckBox4") { 0x40 | if ui.checked(F, "CheckBox5") { 0x80 } else { 0 } } else { 0 };
}

fn write_settings(app: &Mb3d, ui: &mut Ui) {
    let a = &app.anim.anim;
    ui.set_text(F, "Edit6", &a.width.to_string());
    ui.set_text(F, "Edit7", &a.height.to_string());
    ui.set_position(F, "UpDown3", a.scale as i64);
    ui.set_text(F, "Edit21", &a.scale.to_string());
    ui.set_item_index(F, "RadioGroup2", if a.interpolation == Interpolation::Linear { 0 } else { 1 });
    ui.set_checked(F, "CheckBox2", a.looped);
    ui.set_item_index(F, "RadioGroup3", a.format as i32);
    ui.set_text(F, "Edit2", &a.output_folder);
    ui.set_text(F, "Edit3", &a.start_index.to_string());
    ui.set_text(F, "Edit4", &a.index_step.to_string());
    ui.set_checked(F, "CheckBox6", a.overwrite);
    ui.set_checked(F, "CheckBox7", a.save_depth);
    ui.set_checked(F, "CheckBox4", a.stereo_bits & 0x40 != 0);
    ui.set_checked(F, "CheckBox5", a.stereo_bits & 0x80 != 0);
    ui.set_enabled(F, "CheckBox5", a.stereo_bits & 0x40 != 0);
}

fn show_project_name(app: &Mb3d, ui: &mut Ui) {
    let n = app.anim.anim.name.clone();
    ui.fm(F).set_caption(&trf("@MB:AnimationMakerParen", &[&n]));
}

/// Renders the small keyframe image in the background (`RenderPrevBMP`).
fn render_thumb(app: &mut Mb3d, ui: &Ui, i: usize) {
    read_settings(app, ui);
    let a = app.anim.anim.clone();
    let id = app.anim.ids[i];
    let job = app.anim.job.clone();
    let waker = ui.waker();
    let two_d = ui.item_index(F, "RadioGroup1") == 1;
    std::thread::spawn(move || {
        let mut a = a;
        if two_d {
            a.keyframes[i].scene.slice_2d = 2;
        }
        crate::maps::set_current_frame(a.first_frame_of(i) as i32 + 1);
        if let Ok(p) = crate::frames::keyframe_preview(&a, i) {
            let b = Bitmap::from_rgb(p.width, p.height, &p.rgb);
            let mut j = job.lock().unwrap();
            j.thumbs.push((i, id, b));
            j.changed = true;
        }
        if let Some(w) = waker {
            w.wake();
        }
    });
}

/// `PaintBox1Paint`: five keyframe images around the current one.
fn paint_box(app: &Mb3d, ui: &mut Ui) {
    let st = &app.anim;
    let n = count(app);
    let cur = st.current;
    let kf = |i: usize| st.anim.keyframes.get(i).map(|k| k.frames.to_string()).unwrap_or_default();
    let set = |ui: &mut Ui, l: &str, s: String| ui.set_caption(F, l, &s);
    set(ui, "Label1", if cur > 2 { (cur - 2).to_string() } else { String::new() });
    set(ui, "Label23", if cur > 2 { kf(cur - 3) } else { String::new() });
    set(ui, "Label2", if cur > 1 { (cur - 1).to_string() } else { String::new() });
    set(ui, "Label26", if cur > 1 { kf(cur - 2) } else { String::new() });
    set(ui, "Label3", cur.to_string());
    let sub: u32 = st.anim.keyframes.iter().take(cur.min(n)).map(|k| k.frames).sum();
    set(ui, "Label29", sub.to_string());
    set(ui, "Label4", if cur < n { (cur + 1).to_string() } else { String::new() });
    set(ui, "Label27", if cur < n { kf(cur) } else { String::new() });
    set(ui, "Label5", if cur + 1 < n { (cur + 2).to_string() } else { String::new() });
    set(ui, "Label28", if cur + 1 < n { kf(cur + 1) } else { String::new() });
    let vis = n >= cur;
    ui.set_visible(F, "Edit1", vis);
    if vis {
        ui.set_text(F, "Edit1", &st.anim.keyframes[cur - 1].frames.to_string());
    }
    let running = st.job.lock().unwrap().running;
    if !running {
        ui.set_enabled(F, "SpeedButton2", cur > 1 && cur <= n);
        ui.set_enabled(F, "SpeedButton3", cur < n);
        ui.set_enabled(F, "SpeedButton4", cur < n);
        ui.set_enabled(F, "SpeedButton5", cur <= n);
        ui.set_enabled(F, "SpeedButton8", cur <= n);
        ui.set_enabled(F, "SpeedButton14", cur <= n);
        ui.set_enabled(F, "SpeedButton12", cur < n);
        ui.set_enabled(F, "Button2", n > 1);
        ui.set_enabled(F, "Button4", n > 1);
    }
    ui.set_enabled(F, "SpeedButton6", cur <= n);
    ui.set_enabled(F, "SpeedButton7", cur > 1);
    // the images
    let pb = ui.c(F, "PaintBox1").rect();
    let (w, h) = (pb.w.max(1) as usize, pb.h.max(1) as usize);
    let mut b = Bitmap::new(w, h, 0);
    let dx = ui.c(F, "Label2").left - ui.c(F, "Label1").left;
    let top = ui.c(F, "SpeedButton5").top + ui.c(F, "SpeedButton5").height + 2;
    for i in 0..5usize {
        let k = i as i64 + cur as i64 - 3;
        let x0 = i as i32 * dx + 14;
        if k >= 0 && (k as usize) < n {
            if let Some(Some(t)) = st.thumbs.get(k as usize) {
                for y in 0..t.h {
                    for x in 0..t.w {
                        let (px, py) = (x0 as usize + x, top as usize + y);
                        if px < w && py < h {
                            b.px[py * w + px] = t.px[y * t.w + x];
                        }
                    }
                }
            }
        }
    }
    ui.set_picture(F, "PaintBox1", Some(b));
    set_scroll_button(app, ui);
}

fn set_scroll_button(app: &Mb3d, ui: &mut Ui) {
    let n = count(app);
    if n > 1 {
        let pw = ui.c(F, "Panel3").width;
        let sw = ui.c(F, "Shape1").width;
        let l = (((app.anim.current as f64 - 0.5) * (pw - sw) as f64 / n as f64).round() as i32 - sw / 2).clamp(0, pw - sw);
        ui.cm(F, "Shape1").left = l;
    }
}

/// `calcTimeForPreviewRender`: frames and memory of the preview.
fn calc_time(app: &Mb3d, ui: &mut Ui) {
    let n = count(app);
    if n == 0 {
        ui.set_caption(F, "Label14", "0:00");
        ui.set_caption(F, "Label25", &tr("@MB:MB3"));
        return;
    }
    let every = ui.position(F, "UpDown1").max(1) as f64;
    let down = ui.position(F, "UpDown2").max(1) as f64;
    let frames: u32 = app.anim.anim.keyframes.iter().map(|k| k.frames).sum();
    let mem = frames as f64 * 4.0 * app.anim.anim.width as f64 * app.anim.anim.height as f64 / (every * down * down);
    ui.set_caption(F, "Label25", &format!("{} MB", (mem * 0.954e-6).round() as i64));
}

/// Inserts a keyframe at the current position (`InsertFromHeader`).
pub fn insert_keyframe(app: &mut Mb3d, ui: &mut Ui, sc: Scene) {
    ui.show(F);
    read_settings(app, ui);
    let cur = app.anim.current;
    let n = count(app);
    let frames = pti(&ui.text(F, "Edit5")).max(0) as u32;
    let mut sc = sc;
    sc.stereo_mode = 0;
    if ui.item_index(F, "RadioGroup1") == 1 {
        sc.slice_2d = 2;
    }
    if cur == n + 1 {
        app.anim.anim.keyframes.push(Keyframe::new(sc.clone(), frames));
        app.anim.thumbs.push(None);
        app.anim.ids.push(app.anim.next_id);
        app.anim.next_id += 1;
        ui.set_text(F, "Edit1", &frames.to_string());
    } else {
        app.anim.anim.keyframes[cur - 1].scene = sc.clone();
    }
    if cur == 1 {
        // the first keyframe gives the animation size
        let s = (app.main.image_scale.max(1)) as i32;
        let a = &mut app.anim.anim;
        a.scale = s as u32;
        a.width = sc.width / s;
        a.height = sc.height / s;
        write_settings(app, ui);
    }
    render_thumb(app, ui, cur - 1);
    if ui.checked(F, "CheckBox1") {
        app.anim.current += 1;
    }
    paint_box(app, ui);
    calc_time(app, ui);
}

/// The main window's parameters as a keyframe.
fn main_scene(app: &mut Mb3d, ui: &mut Ui) -> Scene {
    app.make_scene(ui);
    app.scene.clone()
}

pub fn idle(app: &mut Mb3d, ui: &mut Ui) {
    let (making_video, video) = {
        let mut j = app.anim.job.lock().unwrap();
        let v = if j.running { None } else { j.video.take() };
        (j.making_video, v)
    };
    if let Some(v) = video {
        app.message(ui, &trf("@MB:AnimationVideoWritten", &[&v.display()]));
    }
    let (changed, thumbs, running, done, total, err, last, frames_len) = {
        let mut j = app.anim.job.lock().unwrap();
        let c = j.changed;
        j.changed = false;
        (c, std::mem::take(&mut j.thumbs), j.running, j.done, j.total, j.error.clone(), j.last.take(), j.frames.len())
    };
    for (i, id, b) in thumbs {
        if let Some(k) = app.anim.ids.iter().position(|&x| x == id) {
            let _ = i;
            app.anim.thumbs[k] = Some(b);
        }
    }
    if changed {
        paint_box(app, ui);
    }
    if !changed && !running {
        // the flip book of the preview
        if app.anim.playing && ui.showing(PV) {
            play_tick(app, ui);
        }
        return;
    }
    if let Some((rgb, w, h)) = last {
        // MB3D shows the frames in the main window while rendering
        if !app.main.calculating {
            let c = ui.cm(MAIN, "Image1");
            c.width = w as i32;
            c.height = h as i32;
            c.picture = Some(Bitmap::from_rgb(w, h, &rgb));
        }
    }
    if ui.showing(PV) && frames_len > 0 {
        ui.set_position(PV, "ProgressBar1", done as i64);
        let c = ui.cm(PV, "ScrollBar1");
        c.max = (frames_len as i64 - 1).max(0);
    }
    let el = app.anim.started.elapsed().as_secs();
    if making_video {
        ui.set_caption(F, "Label19", &tr("@MB:MakingTheVideo"));
    } else if running || done > 0 {
        let now = Instant::now();
        let a = &mut app.anim;
        if done > a.last_done {
            let n = done - a.last_done;
            let each = now.saturating_duration_since(a.frame_start).as_secs_f64() / n as f64;
            for _ in 0..n {
                a.items.done(each);
            }
            a.last_done = done;
            a.frame_start = now;
            a.pace = Pace::default();
            a.progress.store(0, Ordering::Relaxed);
        }
        let mut left = String::new();
        if running && done < total {
            let f = a.progress.load(Ordering::Relaxed) as f64 / 1000.0;
            let secs = now.saturating_duration_since(a.frame_start).as_secs_f64();
            a.pace.at(f, secs);
            let l = a.pace.left(&a.items, f, secs, total - done - 1);
            a.left_shown = l.map(|l| smooth_shown(a.left_shown, l, now)).or(a.left_shown);
            if let Some((l, _)) = a.left_shown {
                left = format!(" · {}", left_str(l));
            }
        }
        ui.set_caption(F, "Label19", &trf("@MB:DoneOf", &[&done, &total, &left]));
        ui.set_caption(F, "Label22", &format!("{}:{:02}:{:02}", el / 3600, el / 60 % 60, el % 60));
    }
    if !running {
        end_rendering(app, ui);
        if !err.is_empty() {
            app.message(ui, &trf("@MB:Animation2", &[&err]));
        }
        if ui.showing(PV) {
            ui.set_visible(PV, "ProgressBar1", false);
            ui.set_visible(PV, "SpeedButton1", frames_len > 0);
            app.anim.playing = true;
            app.anim.play_frame = 0;
        }
    }
}

fn end_rendering(app: &mut Mb3d, ui: &mut Ui) {
    ui.set_caption(F, "Button2", &tr("@MB:StartRenderingAnimationImages"));
    if !app.main.calculating && ui.caption(MAIN, "Button2") == tr("@MB:Stop") {
        super::main_form::enable_buttons(app, ui);
    }
    ui.set_enabled(F, "Button2", count(app) > 1);
    for b in ["SpeedButton1", "SpeedButton9", "SpeedButton10", "SpeedButton11", "Edit1", "Button3"] {
        ui.set_enabled(F, b, true);
    }
    ui.set_enabled(F, "Button4", count(app) > 1);
    paint_box(app, ui);
}

/// `Button2Click`: renders the frames into the output folder.
/// "waiting for 3 of 12 units ..." -> (3, 12)
fn parse_waiting(st: &str) -> Option<(usize, usize)> {
    let rest = st.strip_prefix("waiting for ")?;
    let (left, rest) = rest.split_once(" of ")?;
    let all = rest.split_whitespace().next()?;
    Some((left.parse().ok()?, all.parse().ok()?))
}

fn start_rendering(app: &mut Mb3d, ui: &mut Ui) {
    let cap = ui.caption(F, "Button2");
    if cap == tr("@MB:PauseRendering") {
        app.anim.pause.store(true, Ordering::SeqCst);
        ui.set_caption(F, "Button2", &tr("@MB:PausedPressToContinue"));
        return;
    }
    if cap == tr("@MB:PausedPressToContinue") {
        app.anim.pause.store(false, Ordering::SeqCst);
        ui.set_caption(F, "Button2", &tr("@MB:PauseRendering"));
        return;
    }
    // (a preview still runs)
    if app.anim.job.lock().unwrap().running {
        return;
    }
    let (a, run, frames) = match output_plan(app, ui) {
        Ok(p) => p,
        // (fewer than two keyframes: nothing to render, as before)
        Err(_) if app.anim.anim.keyframes.len() < 2 => return,
        Err(e) => return ui.show_message(&e),
    };
    let _ = std::fs::create_dir_all(&a.output_folder);
    if ui.checked(F, "TransparentCheck") && a.format != OutputFormat::Png {
        return ui.show_message(&tr("@MB:ATransparentBackgroundNeedsPNGFrames"));
    }
    let video = match video_settings(ui, &a) {
        Ok(v) => v,
        Err(e) => return ui.show_message(&e),
    };
    save_output_settings(app, ui);
    for b in ["SpeedButton1", "SpeedButton2", "SpeedButton3", "SpeedButton4", "SpeedButton5", "SpeedButton8", "SpeedButton9", "SpeedButton10", "SpeedButton11", "SpeedButton12", "SpeedButton14", "Edit1", "Button3", "Button4"] {
        ui.set_enabled(F, b, false);
    }
    ui.set_caption(F, "Button2", &tr("@MB:PauseRendering"));
    // as in MB3D, the main window's Calculate button stops the animation
    ui.set_caption(MAIN, "Button2", &tr("@MB:Stop"));
    ui.cm(MAIN, "Button2").hint = tr("@MB:StopRenderingTheAnimation");
    app.anim.started = Instant::now();
    app.anim.stop.store(false, Ordering::SeqCst);
    app.anim.pause.store(false, Ordering::SeqCst);
    app.anim.progress.store(0, Ordering::Relaxed);
    app.anim.items = ItemsLeft::default();
    app.anim.pace = Pace::default();
    app.anim.frame_start = app.anim.started;
    app.anim.last_done = 0;
    app.anim.left_shown = None;
    {
        let mut j = app.anim.job.lock().unwrap();
        *j = Job { running: true, total: frames.len(), changed: true, ..Default::default() };
    }
    let job = app.anim.job.clone();
    let stop = app.anim.stop.clone();
    let pause = app.anim.pause.clone();
    let prog = app.anim.progress.clone();
    let waker = ui.waker();
    let swarm_dir = super::swarm_form::anim_folder(app);
    let swarm_name = app.swarm.name.clone();
    std::thread::spawn(move || {
        let cancel = || stop.load(Ordering::SeqCst);
        if let Some(dir) = swarm_dir {
            // all frames with the swarm; the owner's waiting reports give the count
            let cancel_fn: std::sync::Arc<dyn Fn() -> bool + Send + Sync> = {
                let s = stop.clone();
                std::sync::Arc::new(move || s.load(Ordering::SeqCst))
            };
            let opts = crate::swarm::RunOptions { name: swarm_name, threads: run.threads, quiet: true, keep: false, cancel: Some(cancel_fn), resume: crate::checkpoint::Policy::Continue };
            let report = |st: &str| {
                if let Some((left, all)) = parse_waiting(st) {
                    let mut j = job.lock().unwrap();
                    j.total = all;
                    j.done = all.saturating_sub(left);
                    j.changed = true;
                }
                if let Some(w) = &waker {
                    w.wake();
                }
            };
            let r = crate::swarm::run_animation(&dir, &a, Some(&frames), &opts, &report);
            let mut j = job.lock().unwrap();
            match r {
                Ok(files) => {
                    j.done = files.len();
                    j.total = files.len();
                    if let Some(img) = files.last().and_then(|p| crate::image::load(p).ok()) {
                        let s = if img.deep { 8 } else { 0 };
                        let rgb: Vec<u8> = img.data.iter().flat_map(|p| [(p[0] >> s) as u8, (p[1] >> s) as u8, (p[2] >> s) as u8]).collect();
                        j.last = Some((rgb, img.width, img.height));
                    }
                }
                Err(e) => {
                    if e != "cancelled" {
                        j.error = e;
                    }
                }
            }
            j.changed = true;
            drop(j);
            if let Some(w) = &waker {
                w.wake();
            }
        } else {
        // (several frames at once on a computer with several graphics
        // cards: see `frames::render_frames`)
        let before = || {
            while pause.load(Ordering::SeqCst) && !cancel() {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        };
        let progress = |_f: usize, d: usize, t: usize| prog.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
        crate::frames::render_frames(&a, &frames, &run, &before, &progress, &cancel, &mut |_f, r| {
            let mut j = job.lock().unwrap();
            let go = match r {
                Ok(FrameResult::Written { path, .. }) => {
                    j.done += 1;
                    if let Ok(img) = crate::image::load(&path) {
                        let rgb: Vec<u8> = img.data.iter().flat_map(|p| {
                            let s = if img.deep { 8 } else { 0 };
                            [(p[0] >> s) as u8, (p[1] >> s) as u8, (p[2] >> s) as u8]
                        }).collect();
                        j.last = Some((rgb, img.width, img.height));
                    }
                    true
                }
                Ok(FrameResult::Skipped { .. }) => {
                    j.done += 1;
                    true
                }
                Err(e) => {
                    if e != "cancelled" {
                        j.error = e;
                    }
                    false
                }
            };
            j.changed = true;
            drop(j);
            if let Some(w) = &waker {
                w.wake();
            }
            go
        });
        }
        // the video of the frames, when all of them are there
        if let Some((path, fps)) = video.filter(|_| !cancel() && job.lock().unwrap().error.is_empty()) {
            {
                let mut j = job.lock().unwrap();
                j.making_video = true;
                j.changed = true;
            }
            if let Some(w) = &waker {
                w.wake();
            }
            let files: Vec<std::path::PathBuf> = frames.iter().map(|&f| a.frame_file(f)).filter(|p| p.is_file()).collect();
            let progress = |d: usize, t: usize| prog.store((d * 1000 / t.max(1)) as u32, Ordering::Relaxed);
            let r = crate::video::frames_to_avi(&files, fps, &path, &progress, &cancel);
            let mut j = job.lock().unwrap();
            j.making_video = false;
            match r {
                Ok(_) => j.video = Some(path),
                Err(e) if e == "cancelled" => {}
                Err(e) => j.error = trf("@MB:VideoLower", &[&e]),
            }
        }
        let mut j = job.lock().unwrap();
        j.running = false;
        j.changed = true;
        drop(j);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

/// `Button4Click` / `AniPreviewForm.StartRendering`: small frames to play.
fn start_preview(app: &mut Mb3d, ui: &mut Ui) {
    read_settings(app, ui);
    let a = app.anim.anim.clone();
    if a.keyframes.len() < 2 || app.anim.job.lock().unwrap().running {
        return;
    }
    let every = ui.position(F, "UpDown1").max(1) as usize;
    let down = ui.position(F, "UpDown2").max(1) as f64;
    let n = a.keyframes.len();
    let k0 = pti(&ui.text(F, "Edit9")).clamp(1, n as i32) as usize - 1;
    let k1 = ui.text(F, "Edit10").trim().parse::<usize>().ok().filter(|&v| v >= 1 && v <= n).unwrap_or(n) - 1;
    let (f0, f1) = (a.first_frame_of(k0), if k1 + 1 >= n { a.frame_count() } else { a.first_frame_of(k1) + 1 });
    let fast = ui.checked(F, "CheckBox3");
    let frames: Vec<usize> = (f0..f1.min(a.frame_count())).step_by(every).collect();
    let run = FrameRun { every: 1, preview: 1.0 / down, ..Default::default() };
    app.anim.preview_step = every;
    app.anim.preview_start = f0 + 1;
    app.anim.playing = false;
    app.anim.stop.store(false, Ordering::SeqCst);
    {
        let mut j = app.anim.job.lock().unwrap();
        *j = Job { running: true, total: frames.len(), frames: vec![None; frames.len()], changed: true, ..Default::default() };
    }
    // no new job while this one runs (end_rendering enables them again)
    ui.set_enabled(F, "Button2", false);
    ui.set_enabled(F, "Button4", false);
    // the preview window: an image of the frame size
    let (w, h) = ((a.width as f64 / down).round() as i32, (a.height as f64 / down).round() as i32);
    setup_preview_window(ui, w, h, frames.len());
    // (the AniGif button shows when the frames are done)
    ui.set_visible(PV, "SpeedButton1", false);
    ui.show(PV);
    ui.set_visible(PV, "ProgressBar1", true);
    ui.cm(PV, "ProgressBar1").max = frames.len() as i64;
    let job = app.anim.job.clone();
    let stop = app.anim.stop.clone();
    let waker = ui.waker();
    std::thread::spawn(move || {
        let cancel = || stop.load(Ordering::SeqCst);
        for (k, &f) in frames.iter().enumerate() {
            if cancel() {
                break;
            }
            crate::maps::set_current_frame(f as i32 + 1);
            let r = crate::frames::frame_render_scene(&a, f, &run).and_then(|mut s| {
                if fast {
                    s.shadows = None;
                    s.mc.reflections = false;
                    s.vol_light = None;
                    s.deao = None;
                    s.ao = None;
                    s.dof = None;
                }
                crate::frames::render_scaled(&s, 1, false, false, &|_, _| {}, &cancel)
            });
            let mut j = job.lock().unwrap();
            match r {
                Ok(img) => {
                    if let Some(s) = j.frames.get_mut(k) {
                        *s = Some(Bitmap::from_rgb(img.width, img.height, &img.rgb));
                    }
                    j.done = k + 1;
                    j.last = None;
                }
                Err(e) => {
                    if e != "cancelled" {
                        j.error = e;
                    }
                    break;
                }
            }
            j.changed = true;
            drop(j);
            if let Some(w) = &waker {
                w.wake();
            }
        }
        let mut j = job.lock().unwrap();
        j.running = false;
        j.changed = true;
        drop(j);
        if let Some(w) = &waker {
            w.wake();
        }
    });
}

fn setup_preview_window(ui: &mut Ui, w: i32, h: i32, n: usize) {
    let f = ui.fm(PV);
    if f.id("FrameImage").is_none() {
        let mut c = crate::vcl::control::Control::new("FrameImage", "TImage");
        c.set_bounds(8, 8, w, h);
        f.add_control(0, c);
    }
    let ph = ui.c(PV, "Panel1").height;
    let c = ui.cm(PV, "FrameImage");
    c.set_bounds(8, 8, w, h);
    c.stretch = true;
    let cw = (w + 16).max(ui.c(PV, "Panel1").width);
    ui.set_client_size(PV, cw, h + 16 + ph);
    let sb = ui.cm(PV, "ScrollBar1");
    sb.min = 0;
    sb.max = (n as i64 - 1).max(0);
    sb.position = 0;
}

/// Preview frame `k` as shown: with its number when "Frame nr." is on.
fn frame_picture(app: &Mb3d, ui: &Ui, k: usize) -> Option<Bitmap> {
    let j = app.anim.job.lock().unwrap();
    let mut b = j.frames.get(k)?.clone()?;
    drop(j);
    if ui.checked(PV, "CheckBox1") {
        // the frame number in the corner
        let nr = k * app.anim.preview_step + app.anim.preview_start;
        let col = if ui.checked(PV, "CheckBox2") { 0xFFFF_FFFF } else { 0xFF00_0000 };
        draw_digits(&mut b, 2, 2, nr, col);
    }
    Some(b)
}

fn show_frame(app: &Mb3d, ui: &mut Ui, k: usize) {
    if let Some(b) = frame_picture(app, ui, k) {
        ui.set_picture(PV, "FrameImage", Some(b));
    }
}

/// `SpeedButton1Click` (AniGif): the preview frames as an animated GIF at
/// the preview's speed.
fn save_ani_gif(app: &mut Mb3d, ui: &mut Ui, p: &std::path::Path) {
    let n = app.anim.job.lock().unwrap().frames.len();
    let pics: Vec<Bitmap> = (0..n).filter_map(|k| frame_picture(app, ui, k)).collect();
    if pics.is_empty() {
        return;
    }
    let rgbs: Vec<Vec<u8>> = pics.iter().map(|b| b.px.iter().flat_map(|&c| [(c >> 16) as u8, (c >> 8) as u8, c as u8]).collect()).collect();
    let frames: Vec<crate::gif::Frame> = pics.iter().zip(&rgbs).map(|(b, rgb)| crate::gif::Frame { rgb, w: b.w, h: b.h }).collect();
    let fps = pti(&ui.text(PV, "Edit1")).clamp(1, 60) as f64;
    let gif = crate::gif::encode_animation(&frames, (100.0 / fps).round() as u16);
    if let Err(e) = std::fs::write(p, gif) {
        ui.show_message(&trf("@MB:CouldNotWrite", &[&p.display(), &e]));
    }
}

/// A tiny 3x5 pixel font for the frame numbers of the preview.
fn draw_digits(b: &mut Bitmap, x: usize, y: usize, n: usize, col: u32) {
    const G: [u16; 10] = [0x7B6F, 0x2C97, 0x73E7, 0x73CF, 0x5BC9, 0x79CF, 0x79EF, 0x7249, 0x7BEF, 0x7BCF];
    for (i, ch) in n.to_string().chars().enumerate() {
        let g = G[ch.to_digit(10).unwrap_or(0) as usize];
        for r in 0..5 {
            for c in 0..3 {
                if g >> (14 - (r * 3 + c)) & 1 != 0 {
                    let (px, py) = (x + i * 4 + c, y + r);
                    if px < b.w && py < b.h {
                        b.px[py * b.w + px] = col;
                    }
                }
            }
        }
    }
}

fn play_tick(app: &mut Mb3d, ui: &mut Ui) {
    if !ui.checked(PV, "CheckBox3") {
        return;
    }
    let fps = pti(&ui.text(PV, "Edit1")).clamp(1, 60) as u128;
    if app.anim.started.elapsed().as_millis() < 1000 / fps {
        return;
    }
    app.anim.started = Instant::now();
    let n = app.anim.job.lock().unwrap().frames.len();
    if n == 0 {
        return;
    }
    app.anim.play_frame = (app.anim.play_frame + 1) % n;
    let k = app.anim.play_frame;
    ui.set_position(PV, "ScrollBar1", k as i64);
    show_frame(app, ui, k);
}

pub fn preview_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    match e.handler.as_str() {
        "Button1Click" => {
            app.anim.stop.store(true, Ordering::SeqCst);
            app.anim.playing = false;
            ui.hide(PV);
        }
        "FormHide" => {
            app.anim.stop.store(true, Ordering::SeqCst);
            app.anim.playing = false;
            // (while the preview thread still runs, end_rendering does this)
            if !app.anim.job.lock().unwrap().running {
                ui.set_enabled(F, "Button4", count(app) > 1);
            }
        }
        "SpeedButton1Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.gif", tr("@MB:AnimatedGif")),
                default_ext: "gif".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_IMG)),
                file_name: app.anim.anim.name.clone(),
                ..Default::default()
            };
            ui.save_dialog("anim:gif", &o);
        }
        "ScrollBar1Scroll" => {
            let k = ui.position(PV, "ScrollBar1").max(0) as usize;
            app.anim.play_frame = k;
            show_frame(app, ui, k);
        }
        _ => {}
    }
}

/// `AniProcessForm.Button2Click`: values put into keyframes.
pub fn process_event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    let n = count(app);
    match h {
        "FormShow" => {
            ui.set_enabled(PR, "Button2", n > 0);
            ui.set_enabled(PR, "Button5", n > 0);
            let c = app.anim.current <= n;
            for b in ["Button3", "Button4", "Button6"] {
                ui.set_enabled(PR, b, c);
            }
        }
        "Button1Click" => ui.hide(PR),
        "Button7Click" => {
            app.anim.anim.keyframes.reverse();
            app.anim.thumbs.reverse();
            app.anim.ids.reverse();
            paint_box(app, ui);
        }
        "Button5Click" => {
            for i in 0..n {
                render_thumb(app, ui, i);
            }
        }
        "Button2Click" => {
            if n == 0 {
                return;
            }
            let kf = app.anim.current.clamp(1, n) - 1;
            let (start, stop) = match ui.tag(PR, &e.sender) {
                2 => (kf, n - 1),
                3 => (0, kf),
                4 => (kf, kf),
                _ => (0, n - 1),
            };
            app.make_scene(ui);
            let m = app.scene.clone();
            let ck = |c: &str| ui.checked(PR, c);
            for i in start..=stop {
                let k = &mut app.anim.anim.keyframes[i];
                let s = &mut k.scene;
                if ck("CheckBox1") {
                    s.smooth_normals = ui.position(PR, "UpDown1") as i32;
                }
                if ck("CheckBox2") {
                    s.de_stop = ptf(&ui.text(PR, "Edit25")).max(0.001);
                }
                if ck("CheckBox3") {
                    s.ao = m.ao;
                    s.deao = m.deao;
                }
                if ck("CheckBox4") {
                    s.z_step_div = ptf(&ui.text(PR, "Edit7")).clamp(0.01, 1.0);
                }
                if ck("CheckBox5") {
                    s.bin_search_steps = ui.position(PR, "UpDown3") as i32;
                }
                if ck("CheckBox6") {
                    k.frames = pti(&ui.text(PR, "Edit1")).max(0) as u32;
                }
                let s = &mut k.scene;
                if ck("CheckBox7") {
                    s.raystep_limiter = ptf(&ui.text(PR, "Edit2"));
                }
                if ck("CheckBox8") {
                    s.lighting = m.lighting.clone();
                }
                if ck("CheckBox9") {
                    s.mc = m.mc;
                }
                if ck("CheckBox11") {
                    s.iterations = pti(&ui.text(PR, "Edit4")).max(1);
                }
                if ck("CheckBox12") {
                    s.min_iterations = pti(&ui.text(PR, "Edit5")).max(0);
                }
                if ck("CheckBox14") {
                    s.shadows = m.shadows;
                }
                if ck("CheckBox15") {
                    s.first_step_random = ck("CheckBox17");
                }
                if ck("CheckBox16") {
                    s.dof = m.dof;
                }
                if ck("CheckBox18") {
                    s.color_option = m.color_option;
                    s.color_on_it = m.color_on_it;
                    s.color_mul = m.color_mul;
                    s.vol_light = m.vol_light;
                    s.dfog_on_it = m.dfog_on_it;
                }
                if ck("CheckBox19") {
                    s.stereo_screen = m.stereo_screen;
                }
                if ck("CheckBox20") {
                    s.fov_y = m.fov_y;
                    s.optic = m.optic;
                }
                if ck("CheckBox21") {
                    s.normals_on_zbuf = ck("CheckBox10");
                }
                if ck("CheckBox13") {
                    k.frames = (k.frames as f64 * ptf(&ui.text(PR, "Edit6"))).round().max(0.0) as u32;
                }
            }
            paint_box(app, ui);
            calc_time(app, ui);
        }
        _ => {}
    }
}

pub fn event(app: &mut Mb3d, ui: &mut Ui, e: &Event) {
    let h = e.handler.as_str();
    let n = count(app);
    match h {
        "FormShow" => {
            if app.anim.first_show {
                app.anim.first_show = false;
                ui.cm(F, "Edit21").hint = tr("@MB:N1None22x233x3");
                ui.set_item_index(F, "RadioGroup2", app.ini.get("AniSmoothPar").parse::<i32>().unwrap_or(1).clamp(0, 1));
                let out = app.ini.dir(super::ini::DIR_ANIOUT).display().to_string();
                ui.set_text(F, "Edit2", &out);
                app.anim.anim.output_folder = out;
                ui.set_item_index(F, "RadioGroup3", 1);
                show_project_name(app, ui);
                ui.set_checked(F, "TransparentCheck", app.ini.get_extra("AniTransparent") == Some("1"));
                ui.set_checked(F, "VideoCheck", app.ini.get_extra("AniVideo") == Some("1"));
                let fps = app.ini.get_extra("AniVideoFps").filter(|f| !f.is_empty()).unwrap_or("25").to_string();
                ui.set_text(F, "VideoFpsEdit", &fps);
            }
            let fc = app.ini.get("AniFrameCount").to_string();
            ui.set_text(F, "Edit5", &fc);
            paint_box(app, ui);
            calc_time(app, ui);
        }
        "FormDestroy" | "Button1Click" => {
            app.ini.set("AniFrameCount", &ui.text(F, "Edit5"));
            app.ini.set("AniSmoothPar", &ui.item_index(F, "RadioGroup2").to_string());
            save_output_settings(app, ui);
            if h == "Button1Click" {
                ui.hide(F);
            }
        }
        "SpeedButton1Click" => {
            let s = main_scene(app, ui);
            insert_keyframe(app, ui, s);
        }
        "SpeedButton4Click" => {
            // a keyframe between this and the next (as the button's state:
            // it is not refreshed while a preview runs)
            let cur = app.anim.current;
            if cur >= n {
                return;
            }
            let s = main_scene(app, ui);
            let frames = pti(&ui.text(F, "Edit5")).max(0) as u32;
            app.anim.anim.keyframes.insert(cur, Keyframe::new(s, frames));
            app.anim.thumbs.insert(cur, None);
            app.anim.ids.insert(cur, app.anim.next_id);
            app.anim.next_id += 1;
            render_thumb(app, ui, cur);
            app.anim.current += 1;
            paint_box(app, ui);
            calc_time(app, ui);
        }
        "SpeedButton5Click" => {
            let cur = app.anim.current;
            if cur <= n {
                app.anim.anim.keyframes.remove(cur - 1);
                app.anim.thumbs.remove(cur - 1);
                app.anim.ids.remove(cur - 1);
                paint_box(app, ui);
                calc_time(app, ui);
            }
        }
        "SpeedButton2Click" | "SpeedButton3Click" => {
            let cur = app.anim.current;
            let (a, b) = if h == "SpeedButton2Click" { (cur - 1, cur.saturating_sub(2)) } else { (cur - 1, cur) };
            if a < n && b < n && a != b {
                app.anim.anim.keyframes.swap(a, b);
                app.anim.thumbs.swap(a, b);
                app.anim.ids.swap(a, b);
                paint_box(app, ui);
            }
        }
        "SpeedButton6Click" => {
            if app.anim.current <= n {
                app.anim.current += 1;
                paint_box(app, ui);
            }
        }
        "SpeedButton7Click" => {
            if app.anim.current > 1 {
                app.anim.current -= 1;
                paint_box(app, ui);
            }
        }
        "Shape1MouseDown" => {
            if let Ev::MouseDown { button: MouseButton::Left, .. } = e.ev {
                let mx = ui.f(F).mouse.0;
                app.anim.drag = Some((mx, ui.c(F, "Shape1").left));
            }
        }
        "Shape1MouseMove" => {
            if let (Ev::MouseMove { shift, .. }, Some((mx0, l0))) = (&e.ev, app.anim.drag) {
                if shift & crate::vcl::form::SS_LEFT == 0 {
                    app.anim.drag = None;
                    return;
                }
                let mx = ui.f(F).mouse.0;
                let pw = ui.c(F, "Panel3").width;
                let sw = ui.c(F, "Shape1").width;
                let l = (l0 + mx - mx0).clamp(0, pw - sw);
                ui.cm(F, "Shape1").left = l;
                let cn = ((l as f64 * n as f64 / (pw - sw).max(1) as f64).round() as usize).min(n) + 1;
                if cn != app.anim.current {
                    app.anim.current = cn;
                    paint_box(app, ui);
                    ui.cm(F, "Shape1").left = l;
                }
            }
        }
        "SpeedButton8Click" => {
            // the parameters of the keyframe to the main window
            let cur = app.anim.current;
            if let Some(k) = app.anim.anim.keyframes.get(cur - 1) {
                let mut s = k.scene.clone();
                let a = &app.anim.anim;
                s.width = a.width * a.scale as i32;
                s.height = a.height * a.scale as i32;
                s.stereo_mode = 0;
                s.slice_2d = 0;
                app.scene = s;
                app.main.image_scale = a.scale as i32;
                app.eng.clear();
                app.scene_to_forms(ui);
                app.title = format!("Keyframe #{cur}");
                super::main_form::set_caption(app, ui);
                super::main_form::show_image(app, ui);
            }
        }
        "SpeedButton14Click" => {
            // only the view to the main window
            let cur = app.anim.current;
            if let Some(k) = app.anim.anim.keyframes.get(cur - 1) {
                let k = k.scene.clone();
                app.make_scene(ui);
                let m = &mut app.scene;
                m.z_start = k.z_start;
                m.z_end = k.z_end;
                m.mid = k.mid;
                m.zoom = k.zoom;
                m.vgrads = k.vgrads;
                m.julia = k.julia;
                m.julia_c = k.julia_c;
                m.formulas = k.formulas;
                m.stereo_mode = 0;
                app.scene_to_forms(ui);
                app.title = format!("View of keyframe #{cur}");
                super::main_form::set_caption(app, ui);
            }
        }
        "SpeedButton12Click" => {
            let cur = app.anim.current;
            if cur >= 1 && cur < n {
                let c = app.anim.anim.keyframes[cur - 1].frames;
                ui.input_query("anim:between", &tr("@MB:LinearInterpolateAKeyframeInbetween"), &tr("@MB:AtWhichSubframe"), &((c + 1) / 2).to_string());
            }
        }
        "SpeedButton13Click" => {
            if n > 0 {
                ui.confirm("anim:clear", &tr("@MB:DoYouReallyWantToDelete"));
            }
        }
        "Edit1Change" => {
            let cur = app.anim.current;
            if cur <= n {
                if let Ok(v) = ui.text(F, "Edit1").trim().parse::<u32>() {
                    app.anim.anim.keyframes[cur - 1].frames = v;
                    calc_time(app, ui);
                }
            }
        }
        "Edit1Exit" => {
            let cur = app.anim.current;
            if cur < n {
                render_thumb(app, ui, cur);
            }
        }
        "Edit2Change" => app.anim.anim.output_folder = ui.text(F, "Edit2"),
        "Button3Click" => {
            let d = std::path::PathBuf::from(ui.text(F, "Edit2"));
            ui.folder_dialog("anim:outdir", &tr("@MB:OutputFolder2"), Some(d));
        }
        "Edit3Change" => {
            let t = ui.text(F, "Edit3");
            ui.set_text(F, "Edit8", &t);
        }
        "Edit5Change" => {
            let ok = ui.text(F, &e.sender).trim().parse::<i64>().is_ok() || ui.text(F, &e.sender).trim() == "last";
            ui.cm(F, &e.sender).font = (!ok).then(|| crate::vcl::font::Font { color: 0xFF80_0000, custom_color: true, ..Default::default() });
        }
        "Edit6Change" => {
            read_settings(app, ui);
            calc_time(app, ui);
        }
        "SpinEdit1Change" => calc_time(app, ui),
        "CheckBox4Click" => {
            let c = ui.checked(F, "CheckBox4");
            ui.set_enabled(F, "CheckBox5", c);
        }
        "Button2Click" => start_rendering(app, ui),
        "FfmpegBtnClick" => ffmpeg_command(app, ui),
        "Button4Click" => start_preview(app, ui),
        "SpeedButton10Click" => ui.show(PR),
        "SpeedButton9Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3a", tr("@MB:M3DAnimationM3a")),
                default_ext: "m3a".into(),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3A)),
                file_name: app.anim.anim.name.clone(),
                ..Default::default()
            };
            ui.save_dialog("anim:save", &o);
        }
        "SpeedButton11Click" => {
            let o = crate::vcl::dialogs::FileOptions {
                filter: format!("{}|*.m3a|{}|*.m3k", tr("@MB:M3DAnimationM3a"), tr("@MB:KeyframeListM3k")),
                initial_dir: Some(app.ini.dir(super::ini::DIR_M3A)),
                ..Default::default()
            };
            ui.open_dialog("anim:open", &o);
        }
        "FormKeyPress" => {}
        _ => {}
    }
}

/// Opens an animation file (`LoadAni`).
pub fn load_animation(app: &mut Mb3d, ui: &mut Ui, p: &std::path::Path) {
    match crate::animfile::load(p) {
        Ok(af) => {
            app.anim.anim = af.anim;
            let n = app.anim.anim.keyframes.len();
            app.anim.thumbs = (0..n).map(|i| af.previews.get(i).cloned().flatten().map(|pv| Bitmap::from_rgb(pv.width, pv.height, &pv.rgb))).collect();
            app.anim.ids = (0..n as u64).map(|i| app.anim.next_id + i).collect();
            app.anim.next_id += n as u64;
            app.anim.current = 1;
            app.anim.anim.name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "new".into());
            write_settings(app, ui);
            show_project_name(app, ui);
            for w in af.warnings {
                app.message(ui, &w);
            }
            for i in 0..n {
                if app.anim.thumbs[i].is_none() {
                    render_thumb(app, ui, i);
                }
            }
            ui.show(F);
            paint_box(app, ui);
            calc_time(app, ui);
        }
        Err(e) => ui.show_message(&e),
    }
}

pub fn dialog(app: &mut Mb3d, ui: &mut Ui, what: &str, r: &DialogResult) {
    match (what, r) {
        ("outdir", DialogResult::File(Some(p))) => {
            ui.set_text(F, "Edit2", &p.display().to_string());
            app.anim.anim.output_folder = p.display().to_string();
            app.ini.dirs[super::ini::DIR_ANIOUT] = p.clone();
        }
        ("gif", DialogResult::File(Some(p))) => {
            if let Some(d) = p.parent() {
                app.ini.dirs[super::ini::DIR_IMG] = d.to_path_buf();
            }
            save_ani_gif(app, ui, &p.with_extension("gif"));
        }
        ("open", DialogResult::File(Some(p))) => {
            if let Some(d) = p.parent() {
                app.ini.dirs[super::ini::DIR_M3A] = d.to_path_buf();
            }
            load_animation(app, ui, p);
        }
        ("save", DialogResult::File(Some(p))) => {
            read_settings(app, ui);
            let p = p.with_extension("m3a");
            let previews: Vec<Option<crate::animfile::Preview>> = app
                .anim
                .thumbs
                .iter()
                .map(|t| {
                    t.as_ref().map(|b| crate::animfile::Preview {
                        width: b.w,
                        height: b.h,
                        rgb: b.px.iter().flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8]).collect(),
                    })
                })
                .collect();
            match crate::animfile::save(&p, &app.anim.anim, &previews) {
                Ok(()) => {
                    app.anim.anim.name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    show_project_name(app, ui);
                }
                Err(e) => ui.show_message(&e),
            }
        }
        ("clear", DialogResult::Button(b)) if *b == crate::vcl::form::MR_YES => {
            app.anim.anim.keyframes.clear();
            app.anim.thumbs.clear();
            app.anim.ids.clear();
            app.anim.current = 1;
            let fc = app.ini.get("AniFrameCount").to_string();
            ui.set_text(F, "Edit5", &fc);
            paint_box(app, ui);
            calc_time(app, ui);
        }
        ("between", DialogResult::Text(Some(s))) => {
            let cur = app.anim.current;
            let n = count(app);
            let Ok(sf) = s.trim().parse::<u32>() else { return };
            if cur < 1 || cur >= n {
                return;
            }
            let total = app.anim.anim.keyframes[cur - 1].frames;
            // (subframes count from 1: 0 is no subframe)
            if sf < 1 || sf - 1 >= total {
                ui.show_message(&tr("@MB:ValueNotGuilty"));
                return;
            }
            let sf = sf - 1;
            // linear interpolation between this and the next keyframe
            let w2 = sf as f64 / total.max(1) as f64;
            let s1 = app.anim.anim.keyframes[cur - 1].scene.clone();
            let s2 = app.anim.anim.keyframes[cur].scene.clone();
            let mid = crate::anim::interpolate_linear(&s1, &s2, w2);
            app.anim.anim.keyframes[cur - 1].frames = sf;
            app.anim.anim.keyframes.insert(cur, Keyframe::new(mid, total - sf));
            app.anim.thumbs.insert(cur, None);
            app.anim.ids.insert(cur, app.anim.next_id);
            app.anim.next_id += 1;
            render_thumb(app, ui, cur);
            app.anim.current += 1;
            paint_box(app, ui);
            calc_time(app, ui);
        }
        _ => {}
    }
    let _ = fts_single;
    let _ = parse_float;
}
