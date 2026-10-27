//! Unfinished renders kept on disk, so that a stopped, crashed or killed
//! calculation continues where it was instead of starting again.
//!
//! A checkpoint is a folder in the cache (`resume/<key>`), never in a swarm
//! folder.  The key is a hash of what decides the result (the kind of
//! render, the parameters, the program version), so running the same
//! render again finds it without being told a file name.  It holds:
//!
//! * `data.bin`: the records of every pixel (`rec` bytes each, row major),
//!   written at their place as soon as a unit (a row, a tile) is finished;
//!   for a picture the file is `image.m3i` instead, a valid MB3D image
//!   file at every moment ([`Image`]: its header before the records, the
//!   rest of the parameters after them, the rows not done as background);
//! * `marks.bin`: one mark per unit, written after its data: a magic word,
//!   a hash of the unit's data, a generation (a Monte Carlo pass) and the
//!   rectangle it covers.  A unit whose data does not match its hash (the
//!   computer went off between the two writes) counts as not done, so the
//!   files need no particular order on the disk;
//! * `info.txt`: what the render is (shown by `mb3d resume`);
//! * further named files of the render kind ([`Store::put_extra`]).
//!
//! The data goes to the files as it comes (the system holds it in its
//! cache), so even a killed program leaves a usable checkpoint; every
//! [`autosave_interval`] the files are flushed to the disk as well.  A
//! finished render deletes its checkpoint, a stopped one keeps it (unless it
//! ran for less than [`keep_after`]).  Old ones go after [`MAX_AGE_DAYS`],
//! and the oldest when all of them take more than [`max_bytes`].

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Whether a render keeps a checkpoint and uses an existing one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Policy {
    /// No checkpoint
    Off,
    /// An existing checkpoint is continued, else a new one is made
    #[default]
    Continue,
    /// An existing checkpoint is deleted, a new one is made
    Fresh,
}

/// A stopped render shorter than this deletes its checkpoint (the editor's
/// many short calculations leave nothing behind): 10 seconds; a minute
/// lost most stopped renders, which then could not be continued
/// (MB3D_RESUME_KEEP_SECS sets another).
pub fn keep_after() -> Duration {
    Duration::from_secs(std::env::var("MB3D_RESUME_KEEP_SECS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(10))
}

/// Checkpoints not touched for this many days are deleted.
pub const MAX_AGE_DAYS: u64 = 30;

const MAGIC: u32 = 0x4B43_334D; // "M3CK"
const MARK: usize = 28;

/// How often the files are flushed to the disk: 5 minutes
/// (MB3D_AUTOSAVE_SECS sets another).
pub fn autosave_interval() -> Duration {
    Duration::from_secs(std::env::var("MB3D_AUTOSAVE_SECS").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(300))
}

/// The space all checkpoints may take: 50 GB (MB3D_RESUME_MAX_GB sets
/// another).
pub fn max_bytes() -> u64 {
    let gb: f64 = std::env::var("MB3D_RESUME_MAX_GB").ok().and_then(|v| v.trim().parse().ok()).unwrap_or(50.0);
    (gb.max(0.0) * 1e9) as u64
}

/// The folder of all checkpoints (MB3D_RESUME_DIR sets another).
pub fn folder() -> PathBuf {
    match std::env::var_os("MB3D_RESUME_DIR") {
        Some(d) => {
            let d = PathBuf::from(d);
            let _ = std::fs::create_dir_all(&d);
            d
        }
        None => crate::appdirs::cache_folder("resume"),
    }
}

/// 64 bit FNV-1a: the same on every computer and with every compiler (the
/// standard hasher is neither).
pub fn fnv64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

fn fnv32(data: &[u8]) -> u32 {
    let h = fnv64(data);
    (h ^ (h >> 32)) as u32
}

/// The key of a render: its kind, the program version and `text` (what
/// decides the result).
pub fn key(kind: &str, text: &str) -> String {
    let mut t = format!("{kind}\n{}\n", env!("CARGO_PKG_VERSION"));
    t.push_str(text);
    format!("{kind}-{:016x}", fnv64(t.as_bytes()))
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(unix)]
fn write_at(f: &File, buf: &[u8], off: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::write_all_at(f, buf, off)
}

#[cfg(windows)]
fn write_at(f: &File, mut buf: &[u8], mut off: u64) -> std::io::Result<()> {
    while !buf.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_write(f, buf, off)?;
        if n == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        buf = &buf[n..];
        off += n as u64;
    }
    Ok(())
}

#[cfg(unix)]
fn read_at(f: &File, buf: &mut [u8], off: u64) -> std::io::Result<()> {
    std::os::unix::fs::FileExt::read_exact_at(f, buf, off)
}

#[cfg(windows)]
fn read_at(f: &File, mut buf: &mut [u8], mut off: u64) -> std::io::Result<()> {
    while !buf.is_empty() {
        let n = std::os::windows::fs::FileExt::seek_read(f, buf, off)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        buf = &mut buf[n..];
        off += n as u64;
    }
    Ok(())
}

/// Writes a file so that a reader never sees it half written.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, data).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn parse_kv(text: &str) -> BTreeMap<String, String> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect()
}

fn format_kv(map: &BTreeMap<String, String>) -> String {
    map.iter().map(|(k, v)| format!("{k} = {}\n", v.replace('\n', " "))).collect()
}

/// What a render keeps: its kind and key, a name to show, and the size of
/// its data: `width` x `height` records of `rec` bytes in `units` units.
#[derive(Clone, Debug)]
pub struct Spec {
    pub kind: String,
    pub key: String,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub rec: usize,
    pub units: usize,
    /// the data file as an image file (see [`Image`]), else `data.bin`
    pub image: Option<Image>,
}

/// What makes the data file of a checkpoint a `.m3i` image file: `head`
/// goes before the records, `tail` after them, and `fill` is the record
/// of a pixel not calculated yet (the background), so that the file can
/// be copied out and opened as a picture at any time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub head: Vec<u8>,
    pub tail: Vec<u8>,
    pub fill: Vec<u8>,
}

/// The data file of `spec` in its folder.
fn data_name(spec: &Spec) -> &'static str {
    if spec.image.is_some() { "image.m3i" } else { "data.bin" }
}

/// A unit that was done: its generation and rectangle `[x, y, w, h]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Done {
    pub gen: u32,
    pub rect: [usize; 4],
}

/// The checkpoint of a running render.
pub struct Store {
    dir: PathBuf,
    spec: Spec,
    data: File,
    /// where the records start in `data` (after an image's header)
    data_off: u64,
    marks: File,
    _lock: File,
    done: Mutex<Vec<Option<Done>>>,
    n_done: AtomicUsize,
    info: Mutex<BTreeMap<String, String>>,
    last_sync: Mutex<Instant>,
    opened: Instant,
    resumed: usize,
    failed: AtomicBool,
    closed: bool,
}

impl Store {
    /// Opens the checkpoint of `spec` (`Policy::Continue`: the existing one,
    /// with the units whose data is intact) or makes a new one.  None with
    /// `Policy::Off`, when another process is calculating the same render,
    /// or when the folder cannot be written (the render then runs without).
    pub fn open(spec: Spec, policy: Policy) -> Option<Store> {
        if policy == Policy::Off {
            return None;
        }
        let root = folder();
        prune(&root, Some(&spec.key));
        let dir = root.join(&spec.key);
        std::fs::create_dir_all(&dir).ok()?;
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(dir.join("lock")).ok()?;
        // (a render that was just killed may hold it for a moment longer)
        let mut tries = 0;
        while lock.try_lock().is_err() {
            tries += 1;
            if tries > 30 {
                return None;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let old = std::fs::read_to_string(dir.join("info.txt")).map(|t| parse_kv(&t)).unwrap_or_default();
        let fits = |k: &str, v: usize| old.get(k).and_then(|s| s.parse::<usize>().ok()) == Some(v);
        let same = fits("width", spec.width) && fits("height", spec.height) && fits("rec", spec.rec) && fits("units", spec.units) && old.get("version").map(|v| v.as_str()) == Some(env!("CARGO_PKG_VERSION"));
        let keep = policy == Policy::Continue && same;
        let records = (spec.width * spec.height * spec.rec) as u64;
        let (head, tail) = spec.image.as_ref().map_or((0, 0), |i| (i.head.len() as u64, i.tail.len() as u64));
        let open = |name: &str, len: u64| -> Option<File> {
            let f = OpenOptions::new().read(true).write(true).create(true).truncate(!keep).open(dir.join(name)).ok()?;
            if f.metadata().ok()?.len() != len {
                f.set_len(len).ok()?;
            }
            Some(f)
        };
        // (the data file of the other name: from another version)
        let _ = std::fs::remove_file(dir.join(if spec.image.is_some() { "data.bin" } else { "image.m3i" }));
        let data = open(data_name(&spec), head + records + tail)?;
        let marks = open("marks.bin", (spec.units * MARK) as u64)?;
        if let Some(im) = &spec.image {
            // the image's header and the rest of its parameters as they
            // are now (a continued render may have changed the lighting,
            // which the key leaves out); the rows not done as background
            write_at(&data, &im.head, 0).ok()?;
            write_at(&data, &im.tail, head + records).ok()?;
            if !keep && im.fill.iter().any(|&b| b != 0) && records <= 1 << 30 {
                let row: Vec<u8> = im.fill.iter().cycle().take(spec.width * spec.rec).copied().collect();
                for y in 0..spec.height {
                    write_at(&data, &row, head + (y * spec.width * spec.rec) as u64).ok()?;
                }
            }
        }
        let mut done = vec![None; spec.units];
        if keep {
            read_marks(&data, head, &marks, &spec, &mut done);
        } else {
            for f in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let n = f.file_name();
                let n = n.to_string_lossy();
                if n.starts_with("extra-") {
                    let _ = std::fs::remove_file(f.path());
                }
            }
        }
        let resumed = done.iter().filter(|d| d.is_some()).count();
        let mut info = if keep { old } else { BTreeMap::new() };
        info.insert("kind".into(), spec.kind.clone());
        info.insert("name".into(), spec.name.clone());
        info.insert("version".into(), env!("CARGO_PKG_VERSION").into());
        info.insert("width".into(), spec.width.to_string());
        info.insert("height".into(), spec.height.to_string());
        info.insert("rec".into(), spec.rec.to_string());
        info.insert("units".into(), spec.units.to_string());
        info.insert("data".into(), data_name(&spec).into());
        info.entry("created".into()).or_insert_with(|| now_secs().to_string());
        let s = Store {
            dir,
            spec,
            data,
            data_off: head,
            marks,
            _lock: lock,
            n_done: AtomicUsize::new(resumed),
            done: Mutex::new(done),
            info: Mutex::new(info),
            last_sync: Mutex::new(Instant::now()),
            opened: Instant::now(),
            resumed,
            failed: AtomicBool::new(false),
            closed: false,
        };
        s.write_info();
        Some(s)
    }

    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// The number of units that were done when it was opened (0: a new
    /// render).
    pub fn resumed(&self) -> usize {
        self.resumed
    }

    pub fn done_count(&self) -> usize {
        self.n_done.load(Ordering::Relaxed)
    }

    pub fn is_done(&self, unit: usize) -> bool {
        self.done.lock().unwrap().get(unit).is_some_and(|d| d.is_some())
    }

    pub fn get(&self, unit: usize) -> Option<Done> {
        self.done.lock().unwrap().get(unit).copied().flatten()
    }

    /// Forgets all units (the data stays, it is overwritten).
    pub fn reset(&self) {
        let mut d = self.done.lock().unwrap();
        d.iter_mut().for_each(|u| *u = None);
        self.n_done.store(0, Ordering::Relaxed);
        let _ = write_at(&self.marks, &vec![0u8; self.spec.units * MARK], 0);
    }

    /// A text value of the render (`info.txt`).
    pub fn value(&self, k: &str) -> Option<String> {
        self.info.lock().unwrap().get(k).cloned()
    }

    pub fn set_value(&self, k: &str, v: &str) {
        let changed = self.info.lock().unwrap().insert(k.into(), v.into()).as_deref() != Some(v);
        if changed {
            self.write_info();
        }
    }

    fn write_info(&self) {
        let mut info = self.info.lock().unwrap();
        info.insert("done".into(), self.done_count().to_string());
        info.insert("saved".into(), now_secs().to_string());
        let _ = write_atomic(&self.dir.join("info.txt"), format_kv(&info).as_bytes());
    }

    /// Writes the records of `rect` (`[x, y, w, h]`, row by row) as `unit`
    /// of generation `gen`.
    pub fn put(&self, unit: usize, gen: u32, rect: [usize; 4], recs: &[u8]) {
        if self.failed.load(Ordering::Relaxed) || unit >= self.spec.units {
            return;
        }
        let [x, y, w, h] = rect;
        let r = self.spec.rec;
        debug_assert_eq!(recs.len(), w * h * r);
        let ok = (|| {
            for k in 0..h {
                let off = self.data_off + (((y + k) * self.spec.width + x) * r) as u64;
                write_at(&self.data, &recs[k * w * r..(k + 1) * w * r], off)?;
            }
            write_at(&self.marks, &mark(fnv32(recs), gen, rect), (unit * MARK) as u64)
        })();
        if let Err(e) = ok {
            // (a full disk: the render goes on without its checkpoint)
            if !self.failed.swap(true, Ordering::Relaxed) {
                eprintln!("  checkpoint not written any more: {e}");
            }
            return;
        }
        let mut d = self.done.lock().unwrap();
        if d[unit].is_none() {
            self.n_done.fetch_add(1, Ordering::Relaxed);
        }
        d[unit] = Some(Done { gen, rect });
        drop(d);
        self.autosave();
    }

    /// Reads the records of `rect`.
    pub fn read(&self, rect: [usize; 4]) -> Vec<u8> {
        let [x, y, w, h] = rect;
        let r = self.spec.rec;
        let mut out = vec![0u8; w * h * r];
        for k in 0..h {
            let off = self.data_off + (((y + k) * self.spec.width + x) * r) as u64;
            let _ = read_at(&self.data, &mut out[k * w * r..(k + 1) * w * r], off);
        }
        out
    }

    /// A named file of the render (its state besides the records), written
    /// so that it is never seen half written.
    pub fn put_extra(&self, name: &str, data: &[u8]) {
        if write_atomic(&self.dir.join(format!("extra-{name}")), data).is_err() && !self.failed.swap(true, Ordering::Relaxed) {
            eprintln!("  checkpoint not written any more");
        }
        self.autosave();
    }

    pub fn get_extra(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.extra_path(name)).ok()
    }

    /// The file of [`Store::put_extra`] (it may not be there).
    pub fn extra_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("extra-{name}"))
    }

    /// Copies the file `src` in as a named file of the render (as
    /// [`Store::put_extra`], without holding it in memory): false when it
    /// could not be copied.
    pub fn put_extra_file(&self, name: &str, src: &Path) -> bool {
        let dst = self.extra_path(name);
        let tmp = self.dir.join(format!(".extra-{name}.tmp"));
        let ok = std::fs::copy(src, &tmp).is_ok() && std::fs::rename(&tmp, &dst).is_ok();
        if !ok {
            let _ = std::fs::remove_file(&tmp);
        }
        self.autosave();
        ok
    }

    /// Flushes the files to the disk when the last time is
    /// [`autosave_interval`] ago.
    pub fn autosave(&self) {
        let Ok(mut last) = self.last_sync.try_lock() else { return };
        if last.elapsed() < autosave_interval() {
            return;
        }
        *last = Instant::now();
        drop(last);
        self.sync();
    }

    /// Flushes the files to the disk.
    pub fn sync(&self) {
        let _ = self.data.sync_data();
        let _ = self.marks.sync_data();
        self.write_info();
    }

    /// The render is finished: its checkpoint goes.
    pub fn finish(mut self) {
        self.closed = true;
        let _ = std::fs::remove_dir_all(&self.dir);
    }

    /// The render stopped: the checkpoint is kept for the next time (not
    /// after a short calculation that continued nothing).
    pub fn stop(mut self) {
        self.closed = true;
        self.close_stopped();
    }

    /// The render paused: the checkpoint is kept whatever it holds.
    pub fn keep(mut self) {
        self.closed = true;
        self.sync();
    }

    fn close_stopped(&self) {
        if self.resumed == 0 && self.opened.elapsed() < keep_after() || self.done_count() == 0 && self.resumed == 0 {
            let _ = std::fs::remove_dir_all(&self.dir);
        } else {
            self.sync();
        }
    }

    /// Ends it as the result of a render says: `Ok` finished, an error
    /// stopped.
    pub fn end<T>(self, r: &Result<T, String>) {
        match r {
            Ok(_) => self.finish(),
            Err(_) => self.stop(),
        }
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        // (a panic or an early return: as stopped)
        if !self.closed {
            self.close_stopped();
        }
    }
}

fn mark(hash: u32, gen: u32, rect: [usize; 4]) -> [u8; MARK] {
    let mut m = [0u8; MARK];
    for (k, v) in [MAGIC, hash, gen, rect[0] as u32, rect[1] as u32, rect[2] as u32, rect[3] as u32].into_iter().enumerate() {
        m[k * 4..k * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    m
}

/// The units whose mark is written and whose data matches it.
fn read_marks(data: &File, data_off: u64, marks: &File, spec: &Spec, done: &mut [Option<Done>]) {
    let mut all = vec![0u8; spec.units * MARK];
    if read_at(marks, &mut all, 0).is_err() {
        return;
    }
    let r = spec.rec;
    let mut buf = Vec::new();
    for (u, m) in all.chunks_exact(MARK).enumerate() {
        let v = |k: usize| u32::from_le_bytes(m[k * 4..k * 4 + 4].try_into().unwrap());
        if v(0) != MAGIC {
            continue;
        }
        let rect = [v(3) as usize, v(4) as usize, v(5) as usize, v(6) as usize];
        let [x, y, w, h] = rect;
        if x + w > spec.width || y + h > spec.height {
            continue;
        }
        buf.resize(w * h * r, 0);
        let ok = (0..h).all(|k| read_at(data, &mut buf[k * w * r..(k + 1) * w * r], data_off + (((y + k) * spec.width + x) * r) as u64).is_ok());
        if ok && fnv32(&buf) == v(1) {
            done[u] = Some(Done { gen: v(2), rect });
        }
    }
}

/// A checkpoint on the disk.
#[derive(Clone, Debug)]
pub struct Entry {
    pub dir: PathBuf,
    pub info: BTreeMap<String, String>,
    pub bytes: u64,
    /// a running render holds it
    pub busy: bool,
}

impl Entry {
    /// The image file of a picture's checkpoint (`image.m3i`, a `.m3i`
    /// at any time, its rows not done as background); None for the other
    /// kinds.
    pub fn image_file(&self) -> Option<PathBuf> {
        let p = self.dir.join("image.m3i");
        (self.info.get("data").map(|d| d == "image.m3i").unwrap_or(false) && p.is_file()).then_some(p)
    }

    pub fn key(&self) -> String {
        self.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// The parameters of the render (the scene text), if it kept them.
    pub fn scene_text(&self) -> Option<String> {
        std::fs::read_to_string(self.dir.join("extra-scene.m3s")).ok()
    }

    /// The share of its units done (0..1).
    pub fn progress(&self) -> f64 {
        let n = |k: &str| self.info.get(k).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        if n("units") > 0.0 { (n("done") / n("units")).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Seconds since it was last written.
    pub fn age(&self) -> u64 {
        let saved = self.info.get("saved").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        now_secs().saturating_sub(saved)
    }

    /// The picture's size, "1920 x 1080" (a render can tell another than
    /// its data's: `size` in its info).
    pub fn size_text(&self) -> String {
        match self.info.get("size") {
            Some(s) => s.clone(),
            None => format!("{} x {}", self.info.get("width").map_or("?", |v| v.as_str()), self.info.get("height").map_or("?", |v| v.as_str())),
        }
    }

    /// How far it is: "73 %" (a render can tell otherwise: `done_text`
    /// in its info, e.g. the passes of a Monte Carlo render).
    pub fn done_text(&self) -> String {
        self.info.get("done_text").cloned().unwrap_or_else(|| format!("{:.0} %", self.progress() * 100.0))
    }

    /// "73 % done, saved 2 hours ago".
    pub fn summary(&self) -> String {
        format!("{} done, saved {}", self.done_text(), ago(self.age()))
    }
}

/// "5 minutes ago".
pub fn ago(secs: u64) -> String {
    let (n, unit) = match secs {
        0..60 => return "just now".into(),
        60..3600 => (secs / 60, "minute"),
        3600..86400 => (secs / 3600, "hour"),
        _ => (secs / 86400, "day"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

fn entry(dir: &Path) -> Option<Entry> {
    let mut info = parse_kv(&std::fs::read_to_string(dir.join("info.txt")).ok()?);
    // (a killed render did not update its info: the marks and the time of
    // the last write tell)
    if let Ok(m) = std::fs::read(dir.join("marks.bin")) {
        let n = m.chunks_exact(MARK).filter(|c| c[..4] == MAGIC.to_le_bytes()).count();
        info.insert("done".into(), n.to_string());
    }
    let written = ["marks.bin", "info.txt"].iter().filter_map(|f| std::fs::metadata(dir.join(f)).ok()?.modified().ok()).max();
    if let Some(t) = written.and_then(|t| t.duration_since(UNIX_EPOCH).ok()) {
        info.insert("saved".into(), t.as_secs().to_string());
    }
    let bytes = std::fs::read_dir(dir).into_iter().flatten().flatten().filter_map(|f| f.metadata().ok()).map(|m| m.len()).sum();
    let busy = File::open(dir.join("lock")).map(|f| f.try_lock_shared().is_err()).unwrap_or(false);
    Some(Entry { dir: dir.to_path_buf(), info, bytes, busy })
}

/// All checkpoints, the newest first.
pub fn list() -> Vec<Entry> {
    let mut v: Vec<Entry> = std::fs::read_dir(folder()).into_iter().flatten().flatten().filter_map(|f| entry(&f.path())).collect();
    v.sort_by_key(|e| e.age());
    v
}

/// The checkpoint of `key` with at least one unit done (that no render
/// holds).
pub fn find(key: &str) -> Option<Entry> {
    entry(&folder().join(key)).filter(|e| !e.busy && e.progress() > 0.0)
}

/// Deletes a checkpoint (not one a render holds).
pub fn remove(e: &Entry) -> bool {
    !e.busy && std::fs::remove_dir_all(&e.dir).is_ok()
}

/// Writes the picture of a checkpoint (`calc` or `tiles`, of this or an
/// earlier version: `data.bin` as well as `image.m3i`) as a `.m3i` file
/// to `out`: the rows done as they are, the others as background.
pub fn export_m3i(e: &Entry, out: &Path) -> Result<(), String> {
    use std::io::Write;
    let kind = e.info.get("kind").map(String::as_str).unwrap_or("");
    if kind != "calc" && kind != "tiles" {
        return Err("only a picture is kept as an image (not a Monte Carlo render, a voxel or mesh export)".into());
    }
    let n = |k: &str| e.info.get(k).and_then(|v| v.parse::<usize>().ok()).ok_or_else(|| format!("{k} missing in info.txt"));
    let (width, height, rec, units) = (n("width")?, n("height")?, n("rec")?, n("units")?);
    let text = e.scene_text().ok_or("this render kept no parameters")?;
    let sc = crate::scene::Scene::parse(&text)?;
    let (head, tail) = crate::m3p::m3i_parts(&sc);
    let fill = crate::gbuffer::pack(&[crate::gbuffer::SiLight::default()]);
    if rec != fill.len() || width != sc.width.max(0) as usize || height != sc.height.max(0) as usize {
        return Err("the checkpoint does not match its parameters".into());
    }
    let (name, data_off) = if e.dir.join("image.m3i").is_file() { ("image.m3i", head.len() as u64) } else { ("data.bin", 0) };
    let data = File::open(e.dir.join(name)).map_err(|er| format!("{name}: {er}"))?;
    let marks = File::open(e.dir.join("marks.bin")).map_err(|er| format!("marks.bin: {er}"))?;
    let spec = Spec { kind: kind.into(), key: e.key(), name: String::new(), width, height, rec, units, image: None };
    let mut done = vec![None; units];
    read_marks(&data, data_off, &marks, &spec, &mut done);
    // which rows are there (a tile covers several rows in part: its
    // pixels are read, the rest of the row is background)
    let mut have = vec![false; width * height];
    for d in done.iter().flatten() {
        let [x, y, w, h] = d.rect;
        for yy in y..(y + h).min(height) {
            for xx in x..(x + w).min(width) {
                have[yy * width + xx] = true;
            }
        }
    }
    let mut f = std::io::BufWriter::new(File::create(out).map_err(|er| format!("{}: {er}", out.display()))?);
    let w = |f: &mut std::io::BufWriter<File>, b: &[u8]| f.write_all(b).map_err(|er| format!("{}: {er}", out.display()));
    w(&mut f, &head)?;
    let mut row = vec![0u8; width * rec];
    for y in 0..height {
        let _ = read_at(&data, &mut row, data_off + (y * width * rec) as u64);
        for x in 0..width {
            if !have[y * width + x] {
                row[x * rec..(x + 1) * rec].copy_from_slice(&fill);
            }
        }
        w(&mut f, &row)?;
    }
    w(&mut f, &tail)?;
    f.flush().map_err(|er| format!("{}: {er}", out.display()))
}

/// Deletes the old checkpoints, then the oldest ones over [`max_bytes`]
/// (not `keep`, not those a render holds).
fn prune(root: &Path, keep: Option<&str>) {
    let mut all: Vec<Entry> = std::fs::read_dir(root).into_iter().flatten().flatten().filter_map(|f| entry(&f.path())).collect();
    all.retain(|e| !e.busy && Some(e.key().as_str()) != keep);
    let mut total: u64 = all.iter().map(|e| e.bytes).sum();
    all.sort_by_key(|e| std::cmp::Reverse(e.age()));
    let cap = max_bytes();
    for e in all {
        if e.age() > MAX_AGE_DAYS * 86400 || total > cap {
            if std::fs::remove_dir_all(&e.dir).is_ok() {
                total = total.saturating_sub(e.bytes);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (the tests set MB3D_RESUME_DIR, which is process wide: one at a time)
    static ONE: Mutex<()> = Mutex::new(());

    fn spec(key: &str) -> Spec {
        Spec { kind: "test".into(), key: key.into(), name: "t".into(), width: 4, height: 3, rec: 2, units: 3, image: None }
    }

    #[test]
    fn an_image_checkpoint_is_an_image_file() {
        let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
        let d = std::env::temp_dir().join(format!("mb3d-ck-img-{}", std::process::id()));
        std::env::set_var("MB3D_RESUME_DIR", &d);
        let key = "test-0002";
        let image = || Some(Image { head: b"HEAD".to_vec(), tail: b"tail!".to_vec(), fill: vec![7, 8] });
        let sp = || Spec { image: image(), ..spec(key) };
        let s = Store::open(sp(), Policy::Fresh).unwrap();
        s.put(1, 0, [0, 1, 4, 1], &[1, 2, 1, 2, 1, 2, 1, 2]);
        s.keep();
        let f = std::fs::read(d.join(key).join("image.m3i")).unwrap();
        assert_eq!(f.len(), 4 + 24 + 5);
        assert_eq!(&f[..4], b"HEAD");
        assert_eq!(&f[4..12], &[7, 8, 7, 8, 7, 8, 7, 8], "a row not done: the fill");
        assert_eq!(&f[12..20], &[1, 2, 1, 2, 1, 2, 1, 2]);
        assert_eq!(&f[28..], b"tail!");
        assert!(entry(&d.join(key)).unwrap().image_file().is_some());
        // continued: the row is still there, the header written again
        let s = Store::open(sp(), Policy::Continue).unwrap();
        assert_eq!(s.resumed(), 1);
        assert_eq!(s.read([0, 1, 4, 1]), vec![1, 2, 1, 2, 1, 2, 1, 2]);
        s.finish();
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn units_survive_and_bad_ones_are_dropped() {
        let _one = ONE.lock().unwrap_or_else(|e| e.into_inner());
        let d = std::env::temp_dir().join(format!("mb3d-ck-{}", std::process::id()));
        std::env::set_var("MB3D_RESUME_DIR", &d);
        let key = "test-0001";
        let s = Store::open(spec(key), Policy::Fresh).unwrap();
        // another render of the same key does not get it meanwhile
        assert!(Store::open(spec(key), Policy::Continue).is_none());
        s.put(0, 0, [0, 0, 4, 1], &[1; 8]);
        s.put(2, 5, [0, 2, 4, 1], &[3; 8]);
        s.keep();
        let s = Store::open(spec(key), Policy::Continue).unwrap();
        assert_eq!(s.resumed(), 2);
        assert_eq!(s.get(0), Some(Done { gen: 0, rect: [0, 0, 4, 1] }));
        assert_eq!(s.get(1), None);
        assert_eq!(s.get(2), Some(Done { gen: 5, rect: [0, 2, 4, 1] }));
        assert_eq!(s.read([0, 2, 4, 1]), vec![3; 8]);
        assert!(find(key).is_none(), "held by a render");
        s.keep();
        assert!((find(key).unwrap().progress() - 2.0 / 3.0).abs() < 1e-9);
        // damaged data: the unit counts as not done
        let f = OpenOptions::new().write(true).open(d.join(key).join("data.bin")).unwrap();
        write_at(&f, &[9], 17).unwrap();
        let s = Store::open(spec(key), Policy::Continue).unwrap();
        assert_eq!(s.resumed(), 1);
        assert_eq!(s.get(2), None);
        // another size: a new checkpoint
        let mut sp = spec(key);
        sp.height = 4;
        sp.units = 4;
        drop(s);
        let s = Store::open(sp, Policy::Continue).unwrap();
        assert_eq!(s.resumed(), 0);
        s.finish();
        assert!(!d.join(key).exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
