//! Rendering and saving animation frames (MB3D's `Timer2Timer`,
//! `DoSaveAniImage`, `AniFileAlreadyExists`, `OccupyDFile`) and the pieces
//! shared with batch rendering: output files that are claimed while they
//! are calculated, so several processes (or machines on a shared folder)
//! can work on the same animation or list, and the image encoders.

use crate::anim::{Animation, OutputFormat};
use crate::gbuffer::SiLight;
use crate::scene::Scene;
use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// An output file reserved for this process (`OccupyDFile`): created and
/// locked exclusively until it is written.  A file that is left empty (an
/// interrupted render) counts as not done.
pub struct Claim {
    file: File,
    path: PathBuf,
    written: bool,
}

/// Why an output was not claimed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Skip {
    /// The file exists (and is not empty) and overwriting is off
    Exists,
    /// Another process is calculating it right now
    Busy,
}

impl Claim {
    /// Writes the contents (replacing what the file had) and releases it.
    pub fn finish(mut self, data: &[u8]) -> Result<(), String> {
        let e = |e: std::io::Error| format!("{}: {e}", self.path.display());
        self.file.set_len(0).map_err(e)?;
        self.file.rewind().map_err(e)?;
        self.file.write_all(data).map_err(e)?;
        self.file.flush().map_err(e)?;
        self.written = true;
        let _ = self.file.unlock();
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        if !self.written {
            // `CloseOutPutStream`: an unfinished, empty file is removed again,
            // still locked (else another process may claim it first; Rust
            // opens files deletable on Windows too)
            if self.file.metadata().map(|m| m.len() <= 1).unwrap_or(false) {
                let _ = std::fs::remove_file(&self.path);
            }
            let _ = self.file.unlock();
        }
    }
}

/// `FileIsBigger1`
fn is_done(path: &Path) -> bool {
    std::fs::metadata(path).map(|m| m.len() > 1).unwrap_or(false)
}

/// Whether `path` still names the open `file` (not removed or replaced
/// since it was opened).
fn is_at(file: &File, path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (file.metadata(), std::fs::metadata(path)) {
            (Ok(a), Ok(b)) => (a.dev(), a.ino()) == (b.dev(), b.ino()),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        path.exists()
    }
}

/// Reserves `path` for writing.  Skips files that exist when `overwrite` is
/// off, and files that another process holds.
pub fn claim(path: &Path, overwrite: bool) -> Result<Result<Claim, Skip>, String> {
    // again when the file was removed under us by its last holder
    for _ in 0..3 {
        if let Some(r) = claim_once(path, overwrite)? {
            return Ok(r);
        }
    }
    Ok(Err(Skip::Busy))
}

fn claim_once(path: &Path, overwrite: bool) -> Result<Option<Result<Claim, Skip>>, String> {
    if !overwrite && is_done(path) {
        return Ok(Some(Err(Skip::Exists)));
    }
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Ok(Some(Err(Skip::Busy))),
        Err(std::fs::TryLockError::Error(e)) => return Err(format!("{}: {e}", path.display())),
    }
    if !is_at(&file, path) {
        return Ok(None);
    }
    // another process may have finished it between the check and the lock
    if !overwrite && file.metadata().map(|m| m.len() > 1).unwrap_or(false) {
        let _ = file.unlock();
        return Ok(Some(Err(Skip::Exists)));
    }
    Ok(Some(Ok(Claim { file, path: path.to_path_buf(), written: false })))
}

/// A 24 bit BMP file (`SaveBMP`, pf24bit).
pub fn encode_bmp(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
    let row = (width * 3 + 3) & !3;
    let size = 54 + row * height;
    let mut d = Vec::with_capacity(size);
    d.extend_from_slice(b"BM");
    d.extend_from_slice(&(size as u32).to_le_bytes());
    d.extend_from_slice(&[0; 4]);
    d.extend_from_slice(&54u32.to_le_bytes());
    d.extend_from_slice(&40u32.to_le_bytes());
    d.extend_from_slice(&(width as i32).to_le_bytes());
    d.extend_from_slice(&(height as i32).to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&24u16.to_le_bytes());
    d.extend_from_slice(&[0; 4]);
    d.extend_from_slice(&((row * height) as u32).to_le_bytes());
    d.extend_from_slice(&2835u32.to_le_bytes());
    d.extend_from_slice(&2835u32.to_le_bytes());
    d.extend_from_slice(&[0; 8]);
    for y in (0..height).rev() {
        let start = d.len();
        for x in 0..width {
            let p = &rgb[(y * width + x) * 3..(y * width + x) * 3 + 3];
            d.extend_from_slice(&[p[2], p[1], p[0]]);
        }
        d.resize(start + row, 0);
    }
    d
}

/// A rendered frame.
pub struct FrameImage {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
    /// 16 bit depth (0 = background, larger = nearer), at the image size
    pub depth: Option<Vec<u16>>,
    /// the picture with a transparent background (RGBA, see [`rgba_of`])
    pub rgba: Option<Vec<u8>>,
    pub seconds: f64,
}

/// A picture with a transparent background (RGBA, straight alpha), reduced
/// by `n` (anti-aliasing, 1 = none): the opacity of a pixel is the share
/// of object (not background) pixels in its n x n block, so edges are
/// soft; its colour is the mean of only those object pixels, so the old
/// background does not show as a fringe.  Pixels without an object keep
/// the background's colour (shown by viewers that ignore transparency).
pub fn rgba_of(rgb: &[u8], gbuf: &[SiLight], w: usize, h: usize, n: usize) -> (Vec<u8>, usize, usize) {
    let n = n.max(1);
    let (w2, h2) = (w / n, h / n);
    let nn = (n * n) as u32;
    let mut out = vec![0u8; w2 * h2 * 4];
    for y in 0..h2 {
        for x in 0..w2 {
            let (mut obj, mut all, mut k) = ([0u32; 3], [0u32; 3], 0u32);
            for dy in 0..n {
                for dx in 0..n {
                    let i = (y * n + dy) * w + x * n + dx;
                    let object = !gbuf[i].is_background();
                    k += object as u32;
                    for c in 0..3 {
                        let v = rgb[i * 3 + c] as u32;
                        all[c] += v;
                        if object {
                            obj[c] += v;
                        }
                    }
                }
            }
            let o = (y * w2 + x) * 4;
            for c in 0..3 {
                out[o + c] = if k > 0 { (obj[c] + k / 2) / k } else { (all[c] + nn / 2) / nn } as u8;
            }
            out[o + 3] = ((k * 255 + nn / 2) / nn) as u8;
        }
    }
    (out, w2, h2)
}

/// The 16 bit depth image of a G-buffer `w` x `h` reduced by `n`
/// (anti-aliasing, 1 = none) to the size of the picture: the depth of
/// the block centres (background 0).
pub fn depth_of(gbuf: &[SiLight], w: usize, h: usize, n: usize) -> (Vec<u16>, usize, usize) {
    let n = n.max(1);
    let z = |s: &SiLight| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 };
    let (ow, oh) = (w / n, h / n);
    ((0..ow * oh).map(|i| z(&gbuf[((i / ow) * n + n / 2) * w + (i % ow) * n + n / 2])).collect(), ow, oh)
}

/// Renders a scene that is `aa` times larger than the wanted image and
/// reduces it (`UpdateAndScaleImageFull`).  Tiled scenes are rendered tile
/// by tile.
pub fn render_scaled(
    sc: &Scene,
    aa: usize,
    want_depth: bool,
    want_alpha: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<FrameImage, String> {
    render_scaled_with(sc, aa, want_depth, want_alpha, progress, cancel, None, crate::checkpoint::Policy::Off)
}

/// The checkpoint of a frame's (or a batch item's) picture `sc`, `name`
/// shown for it: of its tiles when it is tiled.
pub fn open_frame_checkpoint(sc: &Scene, name: &str, policy: crate::checkpoint::Policy) -> Option<crate::checkpoint::Store> {
    if sc.tiling.is_some() {
        crate::render::open_tiles_checkpoint(sc, name, policy)
    } else {
        crate::render::open_checkpoint(sc, name, policy)
    }
}

/// [`render_scaled`] with the checkpoint `ck` of [`open_frame_checkpoint`]
/// (`policy`: that of a tiled picture's tiles); the caller ends `ck` when
/// the picture is saved.
#[allow(clippy::too_many_arguments)]
pub fn render_scaled_with(
    sc: &Scene,
    aa: usize,
    want_depth: bool,
    want_alpha: bool,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
    ck: Option<&crate::checkpoint::Store>,
    policy: crate::checkpoint::Policy,
) -> Result<FrameImage, String> {
    let t0 = Instant::now();
    let (rgb, w, h, gbuf) = if sc.tiling.is_some() {
        let r = crate::render::render_tiled_with(sc, want_depth || want_alpha, progress, &|_, _, _| {}, cancel, ck, policy)?;
        (r.rgb, r.width, r.height, r.gbuffer)
    } else {
        let (p, g) = crate::render::calculate_with(sc, progress, &crate::render::no_stage, cancel, 2, None, ck)?;
        let rgb = crate::render::paint(sc, &p, &g);
        (rgb, sc.width as usize, sc.height as usize, Some(g))
    };
    let rgba = if want_alpha { gbuf.as_ref().map(|g| rgba_of(&rgb, g, w, h, aa).0) } else { None };
    let depth = if want_depth { gbuf.map(|g| depth_of(&g, w, h, aa).0) } else { None };
    let (rgb, w, h) = if aa > 1 { crate::render::downsample(&rgb, w, h, aa) } else { (rgb, w, h) };
    Ok(FrameImage { width: w, height: h, rgb, depth, rgba, seconds: t0.elapsed().as_secs_f64() })
}

/// File contents of an image in an output format.
pub fn encode(format: OutputFormat, img: &FrameImage) -> Result<Vec<u8>, String> {
    match format {
        OutputFormat::Png => Ok(match &img.rgba {
            Some(rgba) => crate::png::encode_rgba(img.width, img.height, rgba),
            None => crate::png::encode_rgb(img.width, img.height, &img.rgb),
        }),
        OutputFormat::Bmp => Ok(encode_bmp(img.width, img.height, &img.rgb)),
        OutputFormat::Jpg => Ok(crate::jpeg::encode(img.width, img.height, &img.rgb, 95)),
        OutputFormat::M3p => Err("parameter output has no image".into()),
    }
}

/// Settings of an animation render run (MB3D's animation window).
#[derive(Clone, Debug)]
pub struct FrameRun {
    /// Render only file indices from / up to these (`Edit8`, `Edit11`)
    pub from_index: Option<i64>,
    pub to_index: Option<i64>,
    /// Only every n-th frame (for previews)
    pub every: usize,
    /// Size factor for quick previews (the DE stop is scaled along)
    pub preview: f64,
    pub threads: usize,
    /// The frame under way keeps a checkpoint (see [`crate::checkpoint`]):
    /// a stopped animation continues in the middle of its frame
    pub resume: crate::checkpoint::Policy,
}

impl Default for FrameRun {
    fn default() -> Self {
        FrameRun { from_index: None, to_index: None, every: 1, preview: 1.0, threads: 0, resume: crate::checkpoint::Policy::Off }
    }
}

impl FrameRun {
    /// The frames to render, in order.
    pub fn frames(&self, a: &Animation) -> Vec<usize> {
        (0..a.frame_count())
            .filter(|&f| {
                let i = a.file_index(f);
                self.from_index.is_none_or(|x| i >= x) && self.to_index.is_none_or(|x| i <= x)
            })
            .enumerate()
            .filter(|(n, _)| n % self.every.max(1) == 0)
            .map(|(_, f)| f)
            .collect()
    }
}

/// The scene of a frame as it is calculated (preview factor, threads).
pub fn frame_render_scene(a: &Animation, frame: usize, run: &FrameRun) -> Result<Scene, String> {
    let mut s = a.frame_scene(frame)?;
    if run.preview > 0.0 && (run.preview - 1.0).abs() > 1e-9 {
        s.scale_image(run.preview);
    }
    if run.threads > 0 {
        s.threads = run.threads;
    }
    Ok(s)
}

/// The parameters of a frame as written for the m3p output format (and by
/// `--save-m3p` for single frames): the size of the frame times the scale,
/// light settings interpolated on the sliders.
pub fn frame_params(a: &Animation, frame: usize) -> Result<Vec<u8>, String> {
    let mut s = a.frame_scene(frame)?;
    crate::anim::bake_light_blend(&mut s);
    let mut raw = crate::m3p::write(&s);
    raw[189] = a.scale.min(255) as u8;
    Ok(raw)
}

/// What happened to a frame.
#[derive(Clone, Debug, PartialEq)]
pub enum FrameResult {
    /// `continued`: what an unfinished render of the frame had done
    Written { path: PathBuf, seconds: f64, continued: Option<String> },
    Skipped { path: PathBuf, why: Skip },
}

/// What an unfinished render continued had done ("12 of 900 rows").
pub fn continued_note(ck: Option<&crate::checkpoint::Store>) -> Option<String> {
    let c = ck.filter(|c| c.resumed() > 0)?;
    Some(format!("continued an unfinished render: {} of {} {} were done", c.resumed(), c.spec().units, if c.spec().kind == "tiles" { "tiles" } else { "rows" }))
}

/// Calculates frame `frame` and writes it into its file (or skips it).
pub fn render_frame_to_file(
    a: &Animation,
    frame: usize,
    run: &FrameRun,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> Result<FrameResult, String> {
    let mut continued: Option<String> = None;
    let mut result: Option<FrameResult> = None;
    let mut seconds = 0.0;
    for (mode, eye) in a.stereo_eyes() {
        let path = a.frame_file_eye(frame, eye);
        let c = match claim(&path, a.overwrite)? {
            Ok(c) => c,
            Err(why) => {
                result.get_or_insert(FrameResult::Skipped { path, why });
                continue;
            }
        };
        if a.format == OutputFormat::M3p {
            let t0 = Instant::now();
            let mut raw = frame_params(a, frame)?;
            raw[126] = mode;
            c.finish(&raw)?;
            seconds += t0.elapsed().as_secs_f64();
        } else {
            // map sequences show the image of the file index (AniFileIndex)
            crate::maps::set_current_frame(a.file_index(frame) as i32);
            let mut s = frame_render_scene(a, frame, run)?;
            s.stereo_mode = mode;
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let ck = open_frame_checkpoint(&s, &name, run.resume);
            continued = continued.or_else(|| continued_note(ck.as_ref()));
            let img = render_scaled_with(&s, a.scale.max(1) as usize, a.save_depth, a.transparent && a.format == OutputFormat::Png, progress, cancel, ck.as_ref(), run.resume)?;
            // (the depth first: a frame is done once its file is written)
            if let Some(z) = &img.depth {
                let p = a.depth_file_eye(frame, eye);
                std::fs::write(&p, crate::png::encode_gray16(img.width, img.height, z)).map_err(|e| format!("{}: {e}", p.display()))?;
            }
            c.finish(&encode(a.format, &img)?)?;
            if let Some(ck) = ck {
                ck.finish();
            }
            seconds += img.seconds;
        }
        result = Some(FrameResult::Written { path, seconds, continued: continued.clone() });
    }
    Ok(result.expect("at least one eye"))
}

/// Renders the small keyframe images of a `.m3a` file (`RenderPrevBMP`).
pub fn keyframe_preview(a: &Animation, key: usize) -> Result<crate::animfile::Preview, String> {
    let (w, h) = crate::animfile::preview_size(a.width, a.height);
    let mut s = a.keyframes[key].scene.clone();
    s.light_blend = None;
    s.tiling = None;
    s.calc_rect = None;
    s.width = a.width.max(1);
    s.height = a.height.max(1);
    s.scale_image(w as f64 / s.width as f64);
    s.width = w as i32;
    s.height = h as i32;
    let img = render_scaled(&s, 1, false, false, &|_, _| {}, &|| false)?;
    Ok(crate::animfile::Preview { width: img.width, height: img.height, rgb: img.rgb })
}

#[cfg(test)]
mod tests {
    /// Opacity = share of object pixels in the block, colour = their mean.
    #[test]
    fn rgba_soft_edges_without_fringe() {
        use crate::gbuffer::SiLight;
        // the default is background; an object pixel has a nearer z
        let bg = SiLight::default();
        let obj = SiLight { zpos_fine: 1000 << 16, ..SiLight::default() };
        assert!(bg.is_background() && !obj.is_background());
        // a 6x2 picture in 2x2 blocks: none, one and all four object pixels
        let g = [bg, bg, obj, bg, obj, obj, bg, bg, bg, bg, obj, obj];
        let rgb: Vec<u8> = g.iter().flat_map(|s| if s.is_background() { [0, 0, 255] } else { [200, 100, 0] }).collect();
        let (out, w, h) = rgba_of(&rgb, &g, 6, 2, 2);
        assert_eq!((w, h), (3, 1));
        assert_eq!(&out[0..4], &[0, 0, 255, 0]);
        assert_eq!(&out[4..8], &[200, 100, 0, 64]);
        assert_eq!(&out[8..12], &[200, 100, 0, 255]);
    }

    use super::*;

    #[test]
    fn bmp_layout() {
        let d = encode_bmp(2, 1, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(&d[0..2], b"BM");
        assert_eq!(d.len(), 54 + 8);
        assert_eq!(&d[54..60], &[3, 2, 1, 6, 5, 4]);
    }

    #[test]
    fn claims_skip_and_lock() {
        let dir = std::env::temp_dir().join(format!("mb3d_claim_{}", std::process::id()));
        let p = dir.join("f.png");
        let _ = std::fs::remove_file(&p);
        let c = claim(&p, false).unwrap().unwrap();
        // held by us: a second claim (as from another process) is busy
        assert_eq!(claim(&p, true).unwrap().err(), Some(Skip::Busy));
        c.finish(b"done").unwrap();
        assert_eq!(claim(&p, false).unwrap().err(), Some(Skip::Exists));
        let c = claim(&p, true).unwrap().unwrap();
        drop(c); // not written: the old contents stay
        assert_eq!(std::fs::read(&p).unwrap(), b"done");
        // an interrupted (empty) file is removed and does not count as done
        std::fs::write(&p, b"").unwrap();
        let c = claim(&p, false).unwrap().unwrap();
        drop(c);
        assert!(!p.exists());
        // a file removed (and made again) since it was opened is not the claim
        #[cfg(unix)]
        {
            std::fs::write(&p, b"").unwrap();
            let f = File::open(&p).unwrap();
            assert!(is_at(&f, &p));
            std::fs::remove_file(&p).unwrap();
            std::fs::write(&p, b"").unwrap();
            assert!(!is_at(&f, &p));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn run_selection() {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.width = 32;
        s.height = 24;
        let a = Animation {
            keyframes: vec![crate::anim::Keyframe::new(s.clone(), 10), crate::anim::Keyframe::new(s, 10)],
            start_index: 5,
            ..Default::default()
        };
        let r = FrameRun { from_index: Some(7), to_index: Some(12), every: 2, ..Default::default() };
        assert_eq!(r.frames(&a), vec![2, 4, 6]);
        assert_eq!(FrameRun::default().frames(&a).len(), 11);
    }
}
