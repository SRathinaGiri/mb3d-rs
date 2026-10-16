//! Videos of rendered animation frames, made without other programs:
//!
//! * [`frames_to_avi`] joins the frames into an AVI file with Motion JPEG
//!   (every frame a JPEG picture, encoded here; JPEG frames are copied as
//!   they are). VLC and mpv play it, video editors open it; the files are
//!   big, as each frame is a whole picture.
//! * [`ffmpeg_command`] is the ffmpeg command line for a small MP4 (or a
//!   WebM with transparency) of the frames, for the user to run: this
//!   program starts no other programs.

use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// JPEG quality of the frames the AVI encodes itself.
pub const AVI_QUALITY: u8 = 92;

/// An AVI file (RIFF, Motion JPEG, one video stream) written frame by
/// frame; the sizes and the index are completed by [`AviWriter::finish`].
pub struct AviWriter {
    f: std::fs::File,
    w: u32,
    h: u32,
    /// (offset from the `movi` fourcc, size) of each frame
    index: Vec<(u32, u32)>,
    movi_pos: u64,
    /// where the next frame goes
    next: u64,
    max_frame: u32,
}

/// Bytes of the header in front of the first frame (see [`AviWriter::new`]).
const HEADER_LEN: u64 = 12 + 8 + 4 + (8 + 56) + 8 + 4 + (8 + 56) + (8 + 40) + 8 + 4;

/// AVI 1.0 sizes are 32 bit: the frames must stay below this.
const MAX_MOVI: u64 = 0xFFFF_0000;

fn put32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

impl AviWriter {
    /// A new file `path` of `w` x `h` frames at `fps` frames per second.
    pub fn new(path: &Path, w: u32, h: u32, fps: f64) -> Result<AviWriter, String> {
        let mut f = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        // (fps as a fraction rate / scale)
        let (rate, scale) = ((fps * 1000.0).round().max(1.0) as u32, 1000u32);
        let mut v = Vec::with_capacity(HEADER_LEN as usize);
        v.extend_from_slice(b"RIFF");
        put32(&mut v, 0); // file size - 8, set by finish
        v.extend_from_slice(b"AVI LIST");
        put32(&mut v, 4 + (8 + 56) + 8 + 4 + (8 + 56) + (8 + 40));
        v.extend_from_slice(b"hdrlavih");
        put32(&mut v, 56);
        put32(&mut v, (1_000_000.0 / fps).round() as u32); // microseconds per frame
        put32(&mut v, 0); // max bytes per second
        put32(&mut v, 0); // padding granularity
        put32(&mut v, 0x10); // AVIF_HASINDEX
        put32(&mut v, 0); // total frames, set by finish
        put32(&mut v, 0); // initial frames
        put32(&mut v, 1); // streams
        put32(&mut v, 0); // suggested buffer size, set by finish
        put32(&mut v, w);
        put32(&mut v, h);
        v.extend_from_slice(&[0; 16]);
        v.extend_from_slice(b"LIST");
        put32(&mut v, 4 + (8 + 56) + (8 + 40));
        v.extend_from_slice(b"strlstrh");
        put32(&mut v, 56);
        v.extend_from_slice(b"vidsMJPG");
        put32(&mut v, 0); // flags
        put32(&mut v, 0); // priority, language
        put32(&mut v, 0); // initial frames
        put32(&mut v, scale);
        put32(&mut v, rate);
        put32(&mut v, 0); // start
        put32(&mut v, 0); // length in frames, set by finish
        put32(&mut v, 0); // suggested buffer size, set by finish
        put32(&mut v, u32::MAX); // quality: default
        put32(&mut v, 0); // sample size
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes());
        v.extend_from_slice(&(w as u16).to_le_bytes());
        v.extend_from_slice(&(h as u16).to_le_bytes());
        v.extend_from_slice(b"strf");
        put32(&mut v, 40);
        put32(&mut v, 40); // BITMAPINFOHEADER
        put32(&mut v, w);
        put32(&mut v, h);
        v.extend_from_slice(&1u16.to_le_bytes()); // planes
        v.extend_from_slice(&24u16.to_le_bytes()); // bits per pixel
        v.extend_from_slice(b"MJPG");
        put32(&mut v, w * h * 3);
        v.extend_from_slice(&[0; 16]);
        v.extend_from_slice(b"LIST");
        put32(&mut v, 0); // movi size, set by finish
        v.extend_from_slice(b"movi");
        debug_assert_eq!(v.len() as u64, HEADER_LEN);
        f.write_all(&v).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(AviWriter { f, w, h, index: Vec::new(), movi_pos: HEADER_LEN - 4, next: HEADER_LEN, max_frame: 0 })
    }

    /// Appends one frame, a JPEG picture of the video's size.
    pub fn add_jpeg(&mut self, jpeg: &[u8]) -> Result<(), String> {
        let len = jpeg.len() as u32;
        let pos = self.next;
        if pos + 8 + len as u64 >= MAX_MOVI {
            return Err("the video would be larger than 4 GB (AVI's limit): use the ffmpeg command for a smaller MP4".into());
        }
        let mut v = Vec::with_capacity(jpeg.len() + 9);
        v.extend_from_slice(b"00dc");
        put32(&mut v, len);
        v.extend_from_slice(jpeg);
        if len % 2 == 1 {
            v.push(0);
        }
        self.f.write_all(&v).map_err(|e| e.to_string())?;
        self.index.push(((pos - self.movi_pos) as u32, len));
        self.next += v.len() as u64;
        self.max_frame = self.max_frame.max(len);
        Ok(())
    }

    /// Appends an RGB frame (encoded as JPEG here).
    pub fn add_rgb(&mut self, rgb: &[u8]) -> Result<(), String> {
        let j = crate::jpeg::encode(self.w as usize, self.h as usize, rgb, AVI_QUALITY);
        self.add_jpeg(&j)
    }

    /// Writes the index and the sizes; returns the number of frames.
    pub fn finish(mut self) -> Result<usize, String> {
        let mut idx = Vec::with_capacity(8 + self.index.len() * 16);
        idx.extend_from_slice(b"idx1");
        put32(&mut idx, (self.index.len() * 16) as u32);
        for (off, len) in &self.index {
            idx.extend_from_slice(b"00dc");
            put32(&mut idx, 0x10); // AVIIF_KEYFRAME
            put32(&mut idx, *off);
            put32(&mut idx, *len);
        }
        let io = |e: std::io::Error| e.to_string();
        let movi_end = self.f.stream_position().map_err(io)?;
        self.f.write_all(&idx).map_err(io)?;
        let end = self.f.stream_position().map_err(io)?;
        let frames = self.index.len() as u32;
        let patch = |f: &mut std::fs::File, at: u64, v: u32| -> Result<(), String> {
            f.seek(SeekFrom::Start(at)).map_err(io)?;
            f.write_all(&v.to_le_bytes()).map_err(io)
        };
        patch(&mut self.f, 4, (end - 8) as u32)?;
        // avih: total frames, suggested buffer size
        patch(&mut self.f, 12 + 8 + 4 + 8 + 16, frames)?;
        patch(&mut self.f, 12 + 8 + 4 + 8 + 28, self.max_frame)?;
        // strh: length, suggested buffer size
        let strh = 12 + 8 + 4 + (8 + 56) + 8 + 4 + 8;
        patch(&mut self.f, strh + 32, frames)?;
        patch(&mut self.f, strh + 36, self.max_frame)?;
        // the movi list
        patch(&mut self.f, self.movi_pos - 4, (movi_end - self.movi_pos) as u32)?;
        self.f.flush().map_err(io)?;
        Ok(frames as usize)
    }
}

/// Joins the frame pictures `files` (PNG, BMP or JPG, all of one size) into
/// the AVI file `out` at `fps` frames per second. JPEG frames are copied,
/// the others encoded at [`AVI_QUALITY`] (a transparent background becomes
/// black: AVI has no transparency). `progress(done, total)` after each
/// frame; `cancel` stops it (the file is then incomplete).
pub fn frames_to_avi(files: &[PathBuf], fps: f64, out: &Path, progress: &dyn Fn(usize, usize), cancel: &dyn Fn() -> bool) -> Result<usize, String> {
    let first = files.first().ok_or("no frames to join")?;
    let mut writer: Option<AviWriter> = None;
    let (mut w, mut h) = (0usize, 0usize);
    for (i, f) in files.iter().enumerate() {
        if cancel() {
            return Err("cancelled".into());
        }
        let bytes = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let img = crate::image::decode(&bytes).map_err(|e| format!("{}: {e}", f.display()))?;
        if writer.is_none() {
            (w, h) = (img.width, img.height);
            writer = Some(AviWriter::new(out, w as u32, h as u32, fps)?);
        } else if (img.width, img.height) != (w, h) {
            return Err(format!("{}: {}x{} pixels, but {} has {w}x{h}: the frames of a video must be of one size", f.display(), img.width, img.height, first.display()));
        }
        let wr = writer.as_mut().unwrap();
        if bytes.starts_with(&[0xFF, 0xD8]) {
            wr.add_jpeg(&bytes)?;
        } else {
            let sh = if img.deep { 8 } else { 0 };
            let rgb: Vec<u8> = img.data.iter().flat_map(|p| [(p[0] >> sh) as u8, (p[1] >> sh) as u8, (p[2] >> sh) as u8]).collect();
            wr.add_rgb(&rgb)?;
        }
        progress(i + 1, files.len());
    }
    writer.ok_or("no frames to join")?.finish()
}

/// The ffmpeg command for a small video of an animation's frames:
/// `<folder>/<name><6 digit index>.<ext>` from `first_index` on, `step`
/// apart, at `fps` frames per second. An MP4 (H.264), or with
/// `transparent` PNG frames a WebM (VP9 with transparency).
pub fn ffmpeg_command(folder: &Path, name: &str, ext: &str, first_index: i64, step: i64, fps: f64, transparent: bool) -> String {
    let win = cfg!(windows);
    let quote = |s: &str| if win { format!("\"{s}\"") } else { format!("'{}'", s.replace('\'', "'\\''")) };
    let dir = folder.to_string_lossy();
    let join = |file: &str| if dir.is_empty() { file.to_string() } else { format!("{}{}{file}", dir.trim_end_matches(['/', '\\']), std::path::MAIN_SEPARATOR) };
    let fps = format!("{}", (fps * 1000.0).round() / 1000.0);
    // ffmpeg's numbered input needs consecutive indices; else a pattern
    let input = if step == 1 || win {
        format!("-start_number {first_index} -i {}", quote(&join(&format!("{}%06d.{ext}", name.replace('%', "%%")))))
    } else {
        format!("-pattern_type glob -i {}", quote(&join(&format!("{name}[0-9][0-9][0-9][0-9][0-9][0-9].{ext}"))))
    };
    if transparent && ext == "png" {
        format!("ffmpeg -framerate {fps} {input} -c:v libvpx-vp9 -pix_fmt yuva420p -crf 30 -b:v 0 {}", quote(&join(&format!("{name}.webm"))))
    } else {
        format!("ffmpeg -framerate {fps} {input} -c:v libx264 -crf 18 -pix_fmt yuv420p {}", quote(&join(&format!("{name}.mp4"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The AVI's structure: RIFF sizes, frame count, index offsets.
    #[test]
    fn avi_structure() {
        let dir = std::env::temp_dir().join(format!("mb3d-avi-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.avi");
        let mut wr = AviWriter::new(&p, 16, 8, 25.0).unwrap();
        for k in 0..3u8 {
            let rgb: Vec<u8> = (0..16 * 8 * 3).map(|i| (i as u8).wrapping_mul(k + 1)).collect();
            wr.add_rgb(&rgb).unwrap();
        }
        assert_eq!(wr.finish().unwrap(), 3);
        let d = std::fs::read(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir(&dir);
        let u32at = |o: usize| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
        assert_eq!(&d[0..4], b"RIFF");
        assert_eq!(u32at(4) as usize, d.len() - 8);
        assert_eq!(&d[8..12], b"AVI ");
        assert_eq!(u32at(12 + 8 + 4 + 8 + 16), 3);
        // the movi list ends where idx1 starts; each index entry points at its frame
        let movi = HEADER_LEN as usize - 4;
        let idx = movi + u32at(movi - 4) as usize;
        assert_eq!(&d[idx..idx + 4], b"idx1");
        for k in 0..3 {
            let e = idx + 8 + k * 16;
            let off = movi + u32at(e + 8) as usize;
            assert_eq!(&d[off..off + 4], b"00dc");
            assert_eq!(u32at(off + 4), u32at(e + 12));
            assert_eq!(&d[off + 8..off + 10], &[0xFF, 0xD8]);
        }
    }

    #[test]
    fn ffmpeg_commands() {
        let c = ffmpeg_command(Path::new("/home/you/frames"), "flight", "png", 1, 1, 25.0, false);
        if !cfg!(windows) {
            assert_eq!(c, "ffmpeg -framerate 25 -start_number 1 -i '/home/you/frames/flight%06d.png' -c:v libx264 -crf 18 -pix_fmt yuv420p '/home/you/frames/flight.mp4'");
            let t = ffmpeg_command(Path::new("/f"), "a", "png", 10, 2, 29.97, true);
            assert!(t.contains("-pattern_type glob") && t.contains("libvpx-vp9") && t.contains("-framerate 29.97"), "{t}");
        }
    }
}
