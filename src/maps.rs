//! Image maps (`M3Maps`): loading by number or file name, and the bicubic
//! spline lookups of MB3D (`LoadLightMap`, `GetLightMapPixel`,
//! `GetLightMapPixelSphere`, `SplineIpolMap`, `GetMapPixelSphereSpline`,
//! `GetMapPixelDirectXYspline`).  Maps are used by map formulas (height
//! maps, map transforms), light maps, the background picture and the
//! diffuse colour map.

use crate::math::make_spline_coeff;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

/// A loaded map with MB3D's layout: one extra column and row that repeat
/// the first ones, values in the 0..255 range (16 bit maps keep fractions).
#[derive(Debug)]
pub struct LightMap {
    pub width: usize,
    pub height: usize,
    data: Vec<[f32; 3]>,
    /// `LMavrgCol`
    pub avg: [f32; 3],
}

impl LightMap {
    pub fn from_image(img: &crate::image::Image) -> LightMap {
        let (w, h) = (img.width, img.height);
        let div = if img.deep { 1.0 / 256.0 } else { 1.0 };
        let mut data = Vec::with_capacity((w + 1) * (h + 1));
        for y in 0..=h {
            for x in 0..=w {
                let p = img.pixel(x % w, y % h);
                data.push([p[0] as f32 * div, p[1] as f32 * div, p[2] as f32 * div]);
            }
        }
        let mut avg = [0f64; 3];
        for y in 0..h {
            for x in 0..w {
                let p = data[y * (w + 1) + x];
                for k in 0..3 {
                    avg[k] += p[k] as f64;
                }
            }
        }
        let n = (w * h).max(1) as f64;
        LightMap { width: w, height: h, data, avg: avg.map(|v| (v / n) as f32) }
    }

    /// `MakeSmallLMimage`: a 16 x 8 box-filtered copy, smoothed
    /// horizontally towards the poles (ambient light from the background).
    pub fn small_copy(&self) -> LightMap {
        const W: usize = 16;
        const H: usize = 8;
        let (bw, bh) = (self.width, self.height.max(1));
        let to_col = |v: [f32; 3]| v.map(|c| c.round_ties_even().clamp(0.0, 255.0));
        // horizontal box filter: bh rows x 16
        let dx = (bw as f64 - 1.0) / W as f64;
        let mut tmp = vec![[0f32; 3]; W * bh];
        for y in 0..bh {
            let mut x2 = 1usize;
            for x in 1..=W {
                let xe = (dx * x as f64).round_ties_even() as usize;
                let (mut sv, mut n) = ([0f32; 3], 0usize);
                while x2 <= xe {
                    let p = self.at(x2 - 1, y);
                    for k in 0..3 {
                        sv[k] += p[k];
                    }
                    x2 += 1;
                    n += 1;
                }
                tmp[y * W + x - 1] = to_col(sv.map(|c| c / n.max(1) as f32));
            }
        }
        // vertical box filter into the 17 x 9 layout
        let dy = (bh as f64 - 1.0) / H as f64;
        let mut data = vec![[0f32; 3]; (W + 1) * (H + 1)];
        for x in 0..W {
            let mut y2 = 1usize;
            let mut first = [0f32; 3];
            for y in 1..=H {
                let ye = (dy * y as f64).round_ties_even() as usize;
                let (mut sv, mut n) = ([0f32; 3], 0usize);
                while y2 <= ye {
                    let p = tmp[(y2 - 1) * W + x];
                    for k in 0..3 {
                        sv[k] += p[k];
                    }
                    y2 += 1;
                    n += 1;
                }
                let c = to_col(sv.map(|c| c / n.max(1) as f32));
                data[(y - 1) * (W + 1) + x] = c;
                if y == 1 {
                    first = c;
                }
            }
            data[H * (W + 1) + x] = first;
        }
        // smooth the poles horizontally
        for y in 0..=H {
            let dxs = if y == 0 || y >= H - 1 {
                16.0
            } else {
                2.0 * W as f64 / ((y as f64 * std::f64::consts::PI / (H as f64 - 1.0)).sin() + 1.0) - W as f64
            };
            let row: Vec<[f32; 3]> = data[y * (W + 1)..y * (W + 1) + W].to_vec();
            if dxs > 0.5 {
                let bi = dxs.round_ties_even() as i32;
                let n = -(bi / 2);
                let bi = bi + n;
                for x in 0..=W {
                    let mut sv = [0f32; 3];
                    for x2 in n..=bi {
                        let p = row[((x as i32 + x2) & 15) as usize];
                        for k in 0..3 {
                            sv[k] += p[k];
                        }
                    }
                    data[y * (W + 1) + x] = to_col(sv.map(|c| c / (bi - n + 1) as f32));
                }
            }
            data[y * (W + 1) + W] = data[y * (W + 1)];
        }
        LightMap { width: W, height: H, data, avg: self.avg }
    }

    /// The pixels (0..255) in rows of `width + 1` (the first column
    /// repeated), `height + 1` rows.
    pub fn data(&self) -> &[[f32; 3]] {
        &self.data
    }

    #[inline]
    fn at(&self, x: usize, y: usize) -> [f32; 3] {
        self.data[y * (self.width + 1) + x]
    }

    /// 4x4 bicubic spline with the given column / row indices; `sqr`
    /// interpolates the squared colours (`ColToSVecSqr`).
    fn spline_t(&self, cols: [usize; 4], rows: [usize; 4], xv: [f32; 4], yv: [f32; 4], sqr: bool) -> [f32; 3] {
        let mut r = [0f32; 3];
        for (j, &row) in rows.iter().enumerate() {
            let mut s = [0f32; 3];
            for (i, &col) in cols.iter().enumerate() {
                let mut p = self.at(col, row);
                if sqr {
                    p = p.map(|v| v * v * (1.0 / 255.0));
                }
                for k in 0..3 {
                    s[k] += p[k] * xv[i];
                }
            }
            for k in 0..3 {
                r[k] += s[k] * yv[j];
            }
        }
        r.map(|v| v * (1.0 / 255.0))
    }

    fn spline(&self, cols: [usize; 4], rows: [usize; 4], xv: [f32; 4], yv: [f32; 4]) -> [f32; 3] {
        self.spline_t(cols, rows, xv, yv, false)
    }

    /// `GetLightMapPixel`: x, y in 0..1; `wrap` 0 = clamp, 1 = wrap both
    /// directions, 2 = wrap horizontally only.  Result in 0..1 (times the
    /// map intensity, applied by the caller).
    pub fn pixel(&self, x: f32, y: f32, wrap: u8) -> [f32; 3] {
        self.pixel_t(x, y, wrap, false)
    }

    /// [`pixel`](Self::pixel), optionally in squared colour space.
    pub fn pixel_t(&self, x: f32, y: f32, wrap: u8, sqr: bool) -> [f32; 3] {
        let (w, h) = (self.width as i64, self.height as i64);
        if w <= 4 {
            return [0.0; 3];
        }
        let xs = x.clamp(0.0, 1.0) * w as f32;
        let ys = y.clamp(0.0, 1.0) * h as f32;
        let xf = (xs as i64).min(w - 1);
        let yf = (ys as i64).min(h - 1);
        let xv = make_spline_coeff((xs - xf as f32) as f64);
        let yv = make_spline_coeff((ys - yf as f32) as f64);
        let col = |d: i64| -> usize {
            if wrap > 0 {
                (xf + d).rem_euclid(w) as usize
            } else {
                (xf + d).clamp(0, w - 1) as usize
            }
        };
        let row = |d: i64| -> usize {
            if wrap == 1 {
                (yf + d).rem_euclid(h) as usize
            } else {
                (yf + d).clamp(0, h - 1) as usize
            }
        };
        self.spline_t([col(-1), col(0), col(1), col(2)], [row(-1), row(0), row(1), row(2)], xv, yv, sqr)
    }

    /// `SplineIpolMap`: x in 0..width, y in 0..height (pixel units),
    /// wrapping in both directions.
    pub fn spline_px(&self, x: f64, y: f64) -> [f64; 3] {
        let (w, h) = (self.width as i64, self.height as i64);
        if w < 1 || h < 1 {
            return [0.0; 3];
        }
        let xf = x as i64;
        let yf = y as i64;
        let xv = make_spline_coeff(x - xf as f64);
        let yv = make_spline_coeff(y - yf as f64);
        let col = |d: i64| (xf + d).rem_euclid(w) as usize;
        let row = |d: i64| (yf + d).rem_euclid(h) as usize;
        let r = self.spline([col(-1), col(0), col(1), col(2)], [row(-1), row(0), row(1), row(2)], xv, yv);
        [r[0] as f64, r[1] as f64, r[2] as f64]
    }

    /// `GetLightMapPixelSphere`: direction -> equirectangular map lookup.
    /// `rot` rows are applied as `RotateSVectorS` (sum of v[i] * rot[i]).
    pub fn sphere_pixel(&self, v: [f32; 3], rot: Option<&[[f32; 3]; 3]>) -> [f32; 3] {
        self.sphere_pixel_t(v, rot, false)
    }

    /// [`sphere_pixel`](Self::sphere_pixel), optionally in squared colour space.
    pub fn sphere_pixel_t(&self, v: [f32; 3], rot: Option<&[[f32; 3]; 3]>, sqr: bool) -> [f32; 3] {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-30);
        let mut s = [v[0] / l, v[1] / l, v[2] / l];
        if let Some(m) = rot {
            s = [
                s[0] * m[0][0] + s[1] * m[1][0] + s[2] * m[2][0],
                s[0] * m[0][1] + s[1] * m[1][1] + s[2] * m[2][1],
                s[0] * m[0][2] + s[1] * m[1][2] + s[2] * m[2][2],
            ];
        }
        // MB3D: ArcTan2(sv[0], sv[2]) * MPi05d + 0.5 (MPi05d = -0.5 / Pi)
        let x = s[0].atan2(s[2]) * (-0.5 / std::f32::consts::PI) + 0.5;
        let y = 0.5 - s[1].clamp(-1.0, 1.0).asin() / std::f32::consts::PI;
        self.pixel_t(x, y, 2, sqr)
    }

    /// `GetLightMapPixelNN`: nearest neighbour lookup (x, y in 0..1).
    pub fn pixel_nn(&self, x: f32, y: f32, sqr: bool) -> [f32; 3] {
        if self.width == 0 || self.height == 0 {
            return [0.0; 3];
        }
        let xf = (x.clamp(0.0, 1.0) * self.width as f32).round_ties_even() as usize;
        let yf = (y.clamp(0.0, 1.0) * self.height as f32).round_ties_even() as usize;
        let p = self.at(xf.min(self.width), yf.min(self.height));
        if sqr {
            p.map(|v| v * v * (1.0 / 65025.0))
        } else {
            p.map(|v| v * (1.0 / 255.0))
        }
    }

    /// `GetLightMapPixelSphereNN`
    pub fn sphere_pixel_nn(&self, v: [f32; 3], rot: Option<&[[f32; 3]; 3]>, sqr: bool) -> [f32; 3] {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-30);
        let mut s = [v[0] / l, v[1] / l, v[2] / l];
        if let Some(m) = rot {
            s = [
                s[0] * m[0][0] + s[1] * m[1][0] + s[2] * m[2][0],
                s[0] * m[0][1] + s[1] * m[1][1] + s[2] * m[2][1],
                s[0] * m[0][2] + s[1] * m[1][2] + s[2] * m[2][2],
            ];
        }
        let x = s[0].atan2(s[2]) * (-0.5 / std::f32::consts::PI) + 0.5;
        let y = 0.5 - s[1].clamp(-1.0, 1.0).asin() / std::f32::consts::PI;
        self.pixel_nn(x, y, sqr)
    }
}

fn map_dirs() -> &'static Mutex<Vec<PathBuf>> {
    static D: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    D.get_or_init(|| {
        let mut v = Vec::new();
        if let Ok(e) = std::env::var("MB3D_MAPS") {
            v.extend(std::env::split_paths(&e));
        }
        v.push(PathBuf::from("M3Maps"));
        // as the formulas: the user's, the program's, system-wide
        v.extend(crate::appdirs::user_data_dir().map(|d| d.join("M3Maps")).filter(|d| d.is_dir()));
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() {
                v.push(d.join("M3Maps"));
            }
        }
        v.extend(crate::appdirs::system_data_dirs().into_iter().map(|d| d.join("M3Maps")).filter(|d| d.is_dir()));
        Mutex::new(v)
    })
}

/// Adds a directory searched for maps and background pictures (first).
pub fn add_map_dir(dir: PathBuf) {
    map_dirs().lock().unwrap().insert(0, dir);
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Release);
}

/// Bumped whenever the map of a number can change (folders, sequences, the
/// sequence frame): the threads' caches of [`with_map`] are then refilled.
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

thread_local! {
    /// This thread's maps by number, of the generation in the first field.
    static THREAD_MAPS: std::cell::RefCell<(u64, Vec<(i32, Option<Arc<LightMap>>)>)> = const { std::cell::RefCell::new((u64::MAX, Vec::new())) };
}

/// `f` with map number `nr` ([`by_number`]), None without the map.  For
/// the formulas' map functions, called for every iteration step on every
/// thread: each thread keeps its own references, so the calls take no lock
/// and touch no shared reference count (`by_number` formatted a key and
/// locked two mutexes per call; 24 threads queued on them).
pub fn with_map<R>(nr: i32, f: impl FnOnce(&LightMap) -> R) -> Option<R> {
    let g = GENERATION.load(std::sync::atomic::Ordering::Acquire);
    THREAD_MAPS.with(|c| {
        let mut c = c.borrow_mut();
        if c.0 != g {
            c.0 = g;
            c.1.clear();
        }
        let i = match c.1.iter().position(|(n, _)| *n == nr) {
            Some(i) => i,
            None => {
                c.1.push((nr, by_number(nr)));
                c.1.len() - 1
            }
        };
        c.1[i].1.as_deref().map(f)
    })
}

/// The directories searched for maps (for the editor's "Ini Dirs").
pub fn map_dir_list() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    for d in search_dirs() {
        let c = std::fs::canonicalize(&d).unwrap_or(d);
        if !v.contains(&c) {
            v.push(c);
        }
    }
    v
}

/// All directories searched, including `M3Maps` next to the formula dirs.
fn search_dirs() -> Vec<PathBuf> {
    let mut v = map_dirs().lock().unwrap().clone();
    for f in crate::formulas::formula_dir_list() {
        if let Some(p) = f.parent() {
            v.push(p.join("M3Maps"));
        }
    }
    v
}

const EXTS: [&str; 5] = ["jpg", "png", "bmp", "jpeg", "pgm"];

/// `FindHeightMap`: `<nr>.<ext>`, or a file starting with the number
/// followed by a non-digit.
/// The file of map number `nr` in the map folders.
pub fn find_map_file(nr: i32) -> Option<PathBuf> {
    let n = nr.to_string();
    for d in search_dirs() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        let mut names: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        names.sort();
        let ext_ok = |p: &PathBuf| p.extension().is_some_and(|e| EXTS.iter().any(|x| e.eq_ignore_ascii_case(x)));
        if let Some(p) = names.iter().find(|p| ext_ok(p) && p.file_stem().is_some_and(|s| s.to_string_lossy() == n)) {
            return Some(p.clone());
        }
        if let Some(p) = names.iter().find(|p| {
            let s = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            ext_ok(p) && s.len() > n.len() && s.starts_with(&n) && !s.as_bytes()[n.len()].is_ascii_digit()
        }) {
            return Some(p.clone());
        }
    }
    None
}

type Cache = Mutex<HashMap<String, Option<Arc<LightMap>>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn load_cached(key: String, find: impl FnOnce() -> Option<PathBuf>) -> Option<Arc<LightMap>> {
    if let Some(m) = cache().lock().unwrap().get(&key) {
        return m.clone();
    }
    let m = find().and_then(|p| match crate::image::load(&p) {
        Ok(img) if img.width > 3 && img.height > 3 => Some(Arc::new(LightMap::from_image(&img))),
        Ok(_) => None,
        Err(e) => {
            eprintln!("  warning: map {}: {e}", p.display());
            None
        }
    });
    cache().lock().unwrap().insert(key, m.clone());
    m
}

/// Map number `nr` (`LoadLightMapNr`), cached.  A map sequence for `nr`
/// replaces it by the image of the current frame.
pub fn by_number(nr: i32) -> Option<Arc<LightMap>> {
    if !(1..=32000).contains(&nr) {
        return None;
    }
    if let Some(seq) = sequences().lock().unwrap().iter().find(|q| q.channel == nr).cloned() {
        let name = seq.filename(current_frame())?;
        let key = format!("s:{name}");
        {
            // keep only a few frames of sequences in memory
            let mut c = cache().lock().unwrap();
            if !c.contains_key(&key) && c.keys().filter(|k| k.starts_with("s:")).count() >= 16 {
                c.retain(|k, _| !k.starts_with("s:"));
            }
        }
        return load_cached(key, || {
            let p = PathBuf::from(&name);
            if p.is_file() {
                return Some(p);
            }
            search_dirs().into_iter().map(|d| d.join(&name)).find(|p| p.is_file())
        });
    }
    load_cached(format!("#{nr}"), || find_map_file(nr))
}

// ---------------------------------------------------------------------------
// map sequences (MB3D's "Map Sequences": animated maps)
// ---------------------------------------------------------------------------

/// An image sequence that replaces map number `channel`: the frame number
/// is put into the digits before the extension of `filename`.
#[derive(Clone, Debug, PartialEq)]
pub struct MapSequence {
    pub channel: i32,
    pub filename: String,
    pub first: i32,
    pub last: i32,
    pub increment: i32,
    pub looped: bool,
}

impl MapSequence {
    /// `TMapSequence.FormatFrameFilename`: replaces the number before the
    /// extension, keeping its width; None without such a number.
    pub fn format_frame(filename: &str, frame: i32) -> Option<String> {
        let ext = filename.rfind('.')?;
        let head = &filename[..ext];
        let digits = head.chars().rev().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 {
            return None;
        }
        let start = ext - digits;
        Some(format!("{}{:0w$}{}", &filename[..start], frame, &filename[ext..], w = digits))
    }

    /// `TMapSequence.GetFilename`: the image of animation frame `frame` (1-based).
    pub fn filename(&self, frame: i32) -> Option<String> {
        let inc = self.increment.max(1);
        let mut f = (frame - 1) * inc + self.first;
        let last = (self.last / inc) * inc;
        if f < self.first {
            f = self.first;
        } else if f > last {
            f = if self.looped { (f - 1) % (last - self.first + 1).max(1) + self.first } else { last };
        }
        Self::format_frame(&self.filename, f)
    }
}

fn sequences() -> &'static Mutex<Vec<MapSequence>> {
    static S: OnceLock<Mutex<Vec<MapSequence>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(load_sequences(&sequence_file()).unwrap_or_default()))
}

static FRAME: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

/// The frame number used for map sequences (animations set it per frame).
pub fn set_current_frame(frame: i32) {
    if FRAME.swap(frame.max(1), std::sync::atomic::Ordering::Relaxed) != frame.max(1) {
        GENERATION.fetch_add(1, std::sync::atomic::Ordering::Release);
    }
}

pub fn current_frame() -> i32 {
    FRAME.load(std::sync::atomic::Ordering::Relaxed)
}

/// The sequence list file (MB3D's `Mandelbulb3DMSeq.ini` format), in the
/// working directory or `$MB3D_MAP_SEQUENCES`.
pub fn sequence_file() -> PathBuf {
    std::env::var("MB3D_MAP_SEQUENCES").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("Mandelbulb3DMSeq.ini"))
}

/// Reads a sequence list (`Count=`, `DestChannel#0=`, `ImageFilename#0=`, ...).
pub fn load_sequences(path: &std::path::Path) -> Option<Vec<MapSequence>> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(parse_sequences(&text))
}

pub fn parse_sequences(text: &str) -> Vec<MapSequence> {
    let vals: std::collections::HashMap<String, String> = text
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let int = |k: String| vals.get(&k.to_ascii_lowercase()).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
    (0..int("count".into()).clamp(0, 1000))
        .map(|i| MapSequence {
            channel: int(format!("DestChannel#{i}")),
            filename: vals.get(&format!("imagefilename#{i}")).cloned().unwrap_or_default(),
            first: int(format!("FirstImage#{i}")),
            last: int(format!("LastImage#{i}")),
            increment: int(format!("Increment#{i}")).max(1),
            looped: int(format!("Loop#{i}")) != 0,
        })
        .collect()
}

pub fn sequences_text(list: &[MapSequence]) -> String {
    let mut t = format!("Count={}\r\n", list.len());
    for (i, q) in list.iter().enumerate() {
        t += &format!(
            "DestChannel#{i}={}\r\nImageFilename#{i}={}\r\nFirstImage#{i}={}\r\nLastImage#{i}={}\r\nLoop#{i}={}\r\nIncrement#{i}={}\r\n",
            q.channel, q.filename, q.first, q.last, q.looped as i32, q.increment
        );
    }
    t
}

pub fn map_sequences() -> Vec<MapSequence> {
    sequences().lock().unwrap().clone()
}

/// Replaces the sequence list and saves it to [`sequence_file`].
pub fn set_map_sequences(list: Vec<MapSequence>) -> Result<(), String> {
    *sequences().lock().unwrap() = list.clone();
    cache().lock().unwrap().retain(|k, _| !k.starts_with("s:"));
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Release);
    let p = sequence_file();
    std::fs::write(&p, sequences_text(&list)).map_err(|e| format!("{}: {e}", p.display()))
}

/// A picture by file name (background pictures), searched in the map dirs.
pub fn by_name(name: &str) -> Option<Arc<LightMap>> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    load_cached(format!("f:{name}"), || {
        let p = PathBuf::from(&name);
        if p.is_file() {
            return Some(p);
        }
        search_dirs().into_iter().map(|d| d.join(&name)).find(|p| p.is_file())
    })
}

/// Map numbers that were requested but not found.
pub fn missing() -> Vec<String> {
    let mut v: Vec<String> =
        cache().lock().unwrap().iter().filter(|(_, m)| m.is_none()).map(|(k, _)| k.trim_start_matches("f:").to_string()).collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_sequence_names() {
        assert_eq!(MapSequence::format_frame("c:/maps/wave0001.png", 12).unwrap(), "c:/maps/wave0012.png");
        assert_eq!(MapSequence::format_frame("x9.jpg", 123).unwrap(), "x123.jpg");
        assert!(MapSequence::format_frame("plain.png", 3).is_none());
        let q = MapSequence { channel: 5, filename: "w001.png".into(), first: 1, last: 10, increment: 1, looped: true };
        assert_eq!(q.filename(1).unwrap(), "w001.png");
        assert_eq!(q.filename(10).unwrap(), "w010.png");
        assert_eq!(q.filename(11).unwrap(), "w001.png");
        let q2 = MapSequence { looped: false, ..q.clone() };
        assert_eq!(q2.filename(15).unwrap(), "w010.png");
        let t = sequences_text(&[q.clone(), q2.clone()]);
        assert_eq!(parse_sequences(&t), vec![q, q2]);
    }

    #[test]
    fn spline_of_a_constant_map_is_constant() {
        let img = crate::image::Image { width: 8, height: 6, deep: false, data: vec![[100, 50, 25]; 48] };
        let m = LightMap::from_image(&img);
        for (x, y) in [(0.0, 0.0), (0.3, 0.9), (1.0, 1.0)] {
            let p = m.pixel(x, y, 1);
            assert!((p[0] - 100.0 / 255.0).abs() < 1e-4 && (p[2] - 25.0 / 255.0).abs() < 1e-4, "{p:?}");
        }
        let q = m.spline_px(7.9, 5.5);
        assert!((q[1] - 50.0 / 255.0).abs() < 1e-4);
        assert!((m.avg[0] - 100.0).abs() < 1e-3);
    }
}
