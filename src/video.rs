//! Videos of rendered animation frames: the frame images (PNG, BMP, JPEG)
//! stay as they are, and ffmpeg, an external program (on the PATH, or
//! MB3D_FFMPEG), joins them into a video file.  The container and codec
//! follow the file's extension: .mp4/.mov/.mkv H.264, .webm VP9, others
//! ffmpeg's choice for them (e.g. .gif).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Frames per second when none is given.
pub const DEFAULT_FPS: f64 = 25.0;

/// The ffmpeg program: MB3D_FFMPEG, else `ffmpeg` on the PATH.
pub fn ffmpeg() -> PathBuf {
    std::env::var_os("MB3D_FFMPEG").map(PathBuf::from).unwrap_or_else(|| "ffmpeg".into())
}

const NOT_FOUND: &str = "ffmpeg was not found (install it from https://ffmpeg.org, or set MB3D_FFMPEG to the program)";

/// Whether ffmpeg runs: checked before the frames are rendered, so that a
/// missing ffmpeg is not found out only at the end.
pub fn check() -> Result<(), String> {
    match Command::new(ffmpeg()).arg("-version").output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(_) => Err(format!("{} does not run (ffmpeg -version failed)", ffmpeg().display())),
        Err(_) => Err(NOT_FOUND.into()),
    }
}

/// The video file extensions offered (the window's list).
pub const FORMATS: [&str; 2] = ["mp4", "webm"];

/// The encoder options for a video file (by its extension).
fn codec_args(out: &Path) -> Vec<&'static str> {
    match out.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref() {
        Some("mp4" | "mov") => vec!["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18", "-preset", "medium", "-movflags", "+faststart"],
        Some("mkv") => vec!["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18", "-preset", "medium"],
        Some("webm") => vec!["-c:v", "libvpx-vp9", "-pix_fmt", "yuv420p", "-crf", "30", "-b:v", "0", "-row-mt", "1"],
        _ => vec![],
    }
}

/// The ffconcat list of the frames, each shown for 1/fps seconds.
fn concat_list(frames: &[PathBuf], fps: f64) -> String {
    let mut s = String::from("ffconcat version 1.0\n");
    let quote = |p: &Path| format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"));
    for p in frames {
        s.push_str(&format!("file {}\nduration {:.6}\n", quote(p), 1.0 / fps));
    }
    // the concat demuxer ignores the duration of the last entry
    if let Some(p) = frames.last() {
        s.push_str(&format!("file {}\n", quote(p)));
    }
    s
}

/// Joins the frame images (in this order) into the video file `out` at
/// `fps` frames per second.  Odd sizes are padded by a pixel (H.264 and
/// VP9 need even ones).
pub fn encode(frames: &[PathBuf], fps: f64, out: &Path) -> Result<(), String> {
    if frames.is_empty() {
        return Err("no frames for the video".into());
    }
    if !(fps > 0.0 && fps <= 1000.0) {
        return Err(format!("the frame rate must be between 0 and 1000 (not {fps})"));
    }
    let missing: Vec<&PathBuf> = frames.iter().filter(|p| !p.is_file()).collect();
    if let Some(p) = missing.first() {
        return Err(format!("{} of the video's frames are missing, e.g. {}", missing.len(), p.display()));
    }
    let frames: Vec<PathBuf> = frames.iter().map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.clone())).collect();
    let list = PathBuf::from(format!("{}.frames.txt", out.display()));
    std::fs::write(&list, concat_list(&frames, fps)).map_err(|e| format!("{}: {e}", list.display()))?;
    let fps_s = format!("{fps}");
    let vf = format!("pad=ceil(iw/2)*2:ceil(ih/2)*2,fps={fps_s}");
    let r = Command::new(ffmpeg())
        .args(["-y", "-hide_banner", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list)
        .args(["-vf", &vf, "-r", &fps_s])
        .args(codec_args(out))
        .arg(out)
        .output();
    let _ = std::fs::remove_file(&list);
    match r {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("{NOT_FOUND}; the frames are kept"))
        }
        Err(e) => Err(format!("ffmpeg: {e}")),
        Ok(o) if !o.status.success() => {
            let err = String::from_utf8_lossy(&o.stderr);
            Err(format!("ffmpeg failed: {}", err.trim().lines().last().unwrap_or("(no message)")))
        }
        Ok(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_list_quotes_and_repeats_the_last_frame() {
        let l = concat_list(&[PathBuf::from("/a/it's 1.png"), PathBuf::from("/a/2.png")], 25.0);
        assert_eq!(l, "ffconcat version 1.0\nfile '/a/it'\\''s 1.png'\nduration 0.040000\nfile '/a/2.png'\nduration 0.040000\nfile '/a/2.png'\n");
    }

    /// Three frames of an odd size to an MP4 (only where ffmpeg is installed).
    #[test]
    fn encodes_frames_when_ffmpeg_is_there() {
        if check().is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("mb3d_video_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let frames: Vec<PathBuf> = (0..3u8)
            .map(|i| {
                let p = dir.join(format!("f{i:06}.png"));
                let rgb: Vec<u8> = (0..33 * 17).flat_map(|k| [i * 80, (k % 256) as u8, 40]).collect();
                std::fs::write(&p, crate::png::encode_rgb(33, 17, &rgb)).unwrap();
                p
            })
            .collect();
        let out = dir.join("v.mp4");
        encode(&frames, 10.0, &out).unwrap();
        assert!(std::fs::metadata(&out).unwrap().len() > 100);
        assert!(!PathBuf::from(format!("{}.frames.txt", out.display())).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
