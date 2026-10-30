//! The user's history (the title bar's clock icon, the History window):
//! the parameters of every Calculate 3D as MB3D keeps them
//! (`History/<date>/<time> <title>.m3p`, see [`crate::appdirs::store_history`]),
//! with a thumbnail of the finished picture beside them (the same name
//! as `.png`, written when the calculation ends), and a log of the files
//! opened and saved through the file dialogs (`History/<date>/log.txt`,
//! a line `<seconds since 1970>\t<open|save>\t<path>` each).  The
//! window reads it day by day on a thread ([`Scan`]), the newest first,
//! and compares files ("Show unique") on another, with the keys of their
//! contents kept between sessions ([`Keys`]): a history of a million
//! entries lists while it is read and never holds the window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

/// What an entry records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// a Calculate 3D: the entry's path is the parameter file of History
    Calc,
    Open,
    Save,
}

impl Action {
    fn key(self) -> &'static str {
        match self {
            Action::Calc => "calc",
            Action::Open => "open",
            Action::Save => "save",
        }
    }

    fn parse(s: &str) -> Option<Action> {
        match s {
            "calc" => Some(Action::Calc),
            "open" => Some(Action::Open),
            "save" => Some(Action::Save),
            _ => None,
        }
    }
}

/// What a file is, by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Params,
    /// an image with its parameters
    M3i,
    Image,
    Animation,
    Mesh,
    Formula,
    Other,
}

impl Kind {
    pub fn of(path: &Path) -> Kind {
        match path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref() {
            Some("m3p" | "txt") => Kind::Params,
            Some("m3i") => Kind::M3i,
            Some("png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff" | "pgm") => Kind::Image,
            Some("m3a") => Kind::Animation,
            Some("obj" | "ply" | "stl") => Kind::Mesh,
            Some("m3f") => Kind::Formula,
            _ => Kind::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// seconds since 1970 (UTC, as the folder and file names of History)
    pub secs: i64,
    pub action: Action,
    pub path: PathBuf,
    /// the parameter file's name without its time, or the file's name
    pub title: String,
    pub kind: Kind,
    /// the file's size when it was listed (None: not there)
    pub bytes: Option<u64>,
    /// when the file was last written, seconds since 1970 (0: unknown)
    pub modified: i64,
}

impl Entry {
    /// `yyyy-mm-dd hh:mm:ss`
    pub fn when(&self) -> String {
        let (d, t) = crate::appdirs::stamp_of(self.secs);
        format!("{d} {}:{}:{}", &t[0..2], &t[3..5], &t[6..8])
    }

    /// The thumbnail written with a calculated picture, if there is one.
    pub fn thumb(&self) -> Option<PathBuf> {
        (self.action == Action::Calc).then(|| self.path.with_extension("png")).filter(|p| p.is_file())
    }

}

/// The size and the time of a file, as [`Entry`] keeps them.
fn size_and_time(m: Option<std::fs::Metadata>) -> (Option<u64>, i64) {
    match m.filter(|m| m.is_file()) {
        Some(m) => {
            let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
            (Some(m.len()), t)
        }
        None => (None, 0),
    }
}

/// A day's folder of History.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Day {
    /// `yyyy-mm-dd`, the folder's name
    pub date: String,
    /// seconds since 1970 of the day's start
    pub secs: i64,
    pub path: PathBuf,
}

/// The History folder.
pub fn root() -> PathBuf {
    crate::appdirs::work_folder("History")
}

/// Records an opened or saved file.
pub fn log(action: Action, path: &Path) {
    crate::log::info("file", &format!("{} {}", if action == Action::Save { "saved" } else { "opened" }, path.display()));
    log_in(&root(), action, path, now());
}

fn log_in(root: &Path, action: Action, path: &Path, secs: i64) {
    let (date, _) = crate::appdirs::stamp_of(secs);
    let dir = root.join(date);
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let line = format!("{secs}\t{}\t{}\n", action.key(), path.display());
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(dir.join("log.txt")) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// The days of the history, newest first (their folders' names alone:
/// instant, however big the history).
pub fn days() -> Vec<Day> {
    days_in(&root())
}

fn days_in(root: &Path) -> Vec<Day> {
    let mut out = Vec::new();
    let Ok(dirs) = std::fs::read_dir(root) else { return out };
    for d in dirs.flatten() {
        let date = d.file_name().to_string_lossy().into_owned();
        let Some(secs) = day_secs(&date) else { continue };
        if d.file_type().is_ok_and(|t| t.is_dir()) {
            out.push(Day { date, secs, path: d.path() });
        }
    }
    out.sort_by(|a, b| b.secs.cmp(&a.secs));
    out
}

/// A day's entries, newest first.
pub fn list_day(day: &Day) -> Vec<Entry> {
    let mut out = Vec::new();
    let Ok(files) = std::fs::read_dir(&day.path) else { return out };
    for f in files.flatten() {
        let p = f.path();
        let name = f.file_name().to_string_lossy().into_owned();
        if name == "log.txt" {
            for line in std::fs::read_to_string(&p).unwrap_or_default().lines() {
                let mut it = line.splitn(3, '\t');
                let (Some(s), Some(a), Some(path)) = (it.next(), it.next(), it.next()) else { continue };
                let (Ok(secs), Some(action)) = (s.parse::<i64>(), Action::parse(a)) else { continue };
                let path = PathBuf::from(path);
                let title = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let (bytes, modified) = size_and_time(std::fs::metadata(&path).ok());
                out.push(Entry { secs, action, kind: Kind::of(&path), path, title, bytes, modified });
            }
        } else if name.to_ascii_lowercase().ends_with(".m3p") {
            let stem = &name[..name.len() - 4];
            let (secs, title) = match time_of(stem) {
                Some(t) => (day.secs + t, stem[9..].trim().to_string()),
                None => (day.secs, stem.to_string()),
            };
            let (bytes, modified) = size_and_time(f.metadata().ok());
            out.push(Entry { secs, action: Action::Calc, kind: Kind::Params, path: p, title, bytes, modified });
        }
    }
    out.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.title.cmp(&b.title)));
    out
}

/// All entries, newest first (the whole history at once; the window
/// uses [`Scan`]).
pub fn list() -> Vec<Entry> {
    list_in(&root())
}

fn list_in(root: &Path) -> Vec<Entry> {
    days_in(root).iter().flat_map(list_day).collect()
}

/// What a [`Scan`] found: the days first, then each day's entries in
/// the days' order (the newest day first), then the end.
pub enum Found {
    Days(Vec<Day>),
    /// the index of the day in `Days`, its entries
    Day(usize, Vec<Entry>),
    Done,
}

/// The history read on a thread of its own, day by day; dropping the
/// scan stops it.
pub struct Scan {
    rx: mpsc::Receiver<Found>,
    stop: Arc<AtomicBool>,
}

impl Scan {
    /// Starts reading; `wake` is called whenever something was found.
    pub fn start(wake: Box<dyn Fn() + Send>) -> Scan {
        Scan::start_in(root(), wake)
    }

    fn start_in(root: PathBuf, wake: Box<dyn Fn() + Send>) -> Scan {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        std::thread::spawn(move || {
            let days = days_in(&root);
            if tx.send(Found::Days(days.clone())).is_err() {
                return;
            }
            wake();
            for (i, d) in days.iter().enumerate() {
                if stopped.load(Ordering::Relaxed) || tx.send(Found::Day(i, list_day(d))).is_err() {
                    return;
                }
                wake();
            }
            let _ = tx.send(Found::Done);
            wake();
        });
        Scan { rx, stop }
    }

    /// The next thing found, if any yet.
    pub fn next(&self) -> Option<Found> {
        self.rx.try_recv().ok()
    }
}

impl Drop for Scan {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The keys of the files' contents known so far ("Show unique"), kept
/// between sessions in the cache folder (`History/keys.txt`, a line
/// `<key or ->\t<size>\t<modified>\t<path>` each); a key is used again
/// while the file's size and time are the same.
#[derive(Default)]
pub struct Keys {
    /// path -> (size, modified, key; None: the file could not be read)
    map: HashMap<PathBuf, (u64, i64, Option<u64>)>,
    dirty: bool,
    /// the file the keys are kept in (None: not kept)
    file: Option<PathBuf>,
}

fn keys_file() -> PathBuf {
    crate::appdirs::cache_folder("History").join("keys.txt")
}

impl Keys {
    /// The keys kept from earlier sessions.
    pub fn load() -> Keys {
        let mut k = Keys::load_from(&keys_file());
        k.file = Some(keys_file());
        k
    }

    fn load_from(p: &Path) -> Keys {
        let mut k = Keys::default();
        for line in std::fs::read_to_string(p).unwrap_or_default().lines() {
            let mut it = line.splitn(4, '\t');
            let (Some(key), Some(size), Some(time), Some(path)) = (it.next(), it.next(), it.next(), it.next()) else { continue };
            let (Ok(size), Ok(time)) = (size.parse::<u64>(), time.parse::<i64>()) else { continue };
            let key = match key {
                "-" => None,
                k => match k.parse::<u64>() {
                    Ok(k) => Some(k),
                    Err(_) => continue,
                },
            };
            k.map.insert(PathBuf::from(path), (size, time, key));
        }
        k
    }

    /// The key of a file as it is now: None while it is not known yet,
    /// Some(None) for a file that could not be read.
    pub fn get(&self, path: &Path, bytes: u64, modified: i64) -> Option<Option<u64>> {
        self.map.get(path).filter(|(b, m, _)| *b == bytes && *m == modified).map(|t| t.2)
    }

    pub fn put(&mut self, path: PathBuf, bytes: u64, modified: i64, key: Option<u64>) {
        self.map.insert(path, (bytes, modified, key));
        self.dirty = true;
    }

    /// How many keys are known.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Writes the keys for the next session, when they changed (and
    /// they came from [`Keys::load`]).
    pub fn save(&mut self) {
        if let Some(f) = self.file.clone() {
            self.save_to(&f);
        }
    }

    fn save_to(&mut self, p: &Path) {
        if !self.dirty {
            return;
        }
        let mut out = String::new();
        for (path, (b, m, k)) in &self.map {
            let k = k.map_or("-".to_string(), |k| k.to_string());
            out.push_str(&format!("{k}\t{b}\t{m}\t{}\n", path.display()));
        }
        if std::fs::write(p, out).is_ok() {
            self.dirty = false;
        }
    }
}

/// Computes the keys of `jobs` (path, size, modified) on a thread into
/// `keys`, which is saved at the end; `wake` is called as they come in.
/// The flag returned is set while the thread works.
pub fn hash_keys(keys: Arc<Mutex<Keys>>, jobs: Vec<(PathBuf, u64, i64)>, wake: Box<dyn Fn() + Send>) -> Arc<AtomicBool> {
    let busy = Arc::new(AtomicBool::new(true));
    let flag = busy.clone();
    std::thread::spawn(move || {
        for (i, (path, bytes, modified)) in jobs.into_iter().enumerate() {
            let key = content_key(&path);
            keys.lock().unwrap().put(path, bytes, modified, key);
            if i % 32 == 31 {
                wake();
            }
        }
        keys.lock().unwrap().save();
        flag.store(false, Ordering::Relaxed);
        wake();
    });
    busy
}

/// Seconds since 1970 of the start of a day `yyyy-mm-dd` (None for
/// another name).
pub fn day_secs_of(name: &str) -> Option<i64> {
    day_secs(name)
}

/// Seconds since 1970 of the start of a day `yyyy-mm-dd`.
fn day_secs(name: &str) -> Option<i64> {
    let b = name.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (name[0..4].parse::<i64>().ok()?, name[5..7].parse::<i64>().ok()?, name[8..10].parse::<i64>().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // days since 1970-01-01 (H. Hinnant)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146097 + doe - 719468) * 86400)
}

/// The seconds into the day of a name starting with `hhhmmmsss`.
fn time_of(stem: &str) -> Option<i64> {
    let b = stem.as_bytes();
    if b.len() < 9 || b[2] != b'h' || b[5] != b'm' || b[8] != b's' {
        return None;
    }
    let (h, m, s) = (stem[0..2].parse::<i64>().ok()?, stem[3..5].parse::<i64>().ok()?, stem[6..8].parse::<i64>().ok()?);
    Some(h * 3600 + m * 60 + s)
}

/// Takes an entry out of the history: a calculation's parameter file and
/// thumbnail are deleted (they are the history's own), a logged file stays
/// where it is, only its line goes.
pub fn remove(e: &Entry) -> bool {
    remove_in(&root(), e)
}

fn remove_in(root: &Path, e: &Entry) -> bool {
    match e.action {
        Action::Calc => {
            let _ = std::fs::remove_file(e.path.with_extension("png"));
            std::fs::remove_file(&e.path).is_ok()
        }
        Action::Open | Action::Save => {
            let (date, _) = crate::appdirs::stamp_of(e.secs);
            let p = root.join(date).join("log.txt");
            let Ok(text) = std::fs::read_to_string(&p) else { return false };
            let line = format!("{}\t{}\t{}", e.secs, e.action.key(), e.path.display());
            let mut gone = false;
            let kept: Vec<&str> = text
                .lines()
                .filter(|l| {
                    let hit = !gone && *l == line;
                    gone |= hit;
                    !hit
                })
                .collect();
            if !gone {
                return false;
            }
            let mut out = kept.join("\n");
            if !out.is_empty() {
                out.push('\n');
            }
            std::fs::write(&p, out).is_ok()
        }
    }
}

/// A key of a file's contents, the same for files with the same bytes
/// (the "Show unique" of the History window: calculations of the same
/// parameters, a picture saved twice); None for a file not there.  A big
/// file is keyed by its size with its first and last 64 kB.
pub fn content_key(path: &Path) -> Option<u64> {
    use std::hash::{Hash, Hasher};
    use std::io::{Read, Seek, SeekFrom};
    const PART: u64 = 65536;
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    len.hash(&mut h);
    let mut buf = Vec::new();
    if len <= 4 * PART {
        f.read_to_end(&mut buf).ok()?;
        buf.hash(&mut h);
    } else {
        buf.resize(PART as usize, 0);
        f.read_exact(&mut buf).ok()?;
        buf.hash(&mut h);
        f.seek(SeekFrom::End(-(PART as i64))).ok()?;
        f.read_exact(&mut buf).ok()?;
        buf.hash(&mut h);
    }
    Some(h.finish())
}

/// The thumbnails' size.
pub const THUMB_W: usize = 240;
pub const THUMB_H: usize = 180;

/// Writes the thumbnail of a calculated picture beside its parameter file
/// of History.
pub fn write_thumb(m3p: &Path, rgb: &[u8], w: usize, h: usize) -> std::io::Result<()> {
    let (t, tw, th) = downscale(rgb, w, h, THUMB_W, THUMB_H);
    crate::png::write_rgb(&m3p.with_extension("png").to_string_lossy(), tw, th, &t)
}

/// `rgb` (`w` x `h`) scaled down to fit `max_w` x `max_h` (a box filter;
/// a smaller picture stays as it is).
pub fn downscale(rgb: &[u8], w: usize, h: usize, max_w: usize, max_h: usize) -> (Vec<u8>, usize, usize) {
    if w == 0 || h == 0 || rgb.len() < w * h * 3 {
        return (Vec::new(), 0, 0);
    }
    let f = (max_w as f64 / w as f64).min(max_h as f64 / h as f64).min(1.0);
    let (tw, th) = (((w as f64 * f).round() as usize).max(1), ((h as f64 * f).round() as usize).max(1));
    if (tw, th) == (w, h) {
        return (rgb[..w * h * 3].to_vec(), w, h);
    }
    let mut out = vec![0u8; tw * th * 3];
    for ty in 0..th {
        let (y0, y1) = (ty * h / th, ((ty + 1) * h / th).max(ty * h / th + 1).min(h));
        for tx in 0..tw {
            let (x0, x1) = (tx * w / tw, ((tx + 1) * w / tw).max(tx * w / tw + 1).min(w));
            let mut sum = [0u64; 3];
            for y in y0..y1 {
                let row = &rgb[(y * w + x0) * 3..(y * w + x1) * 3];
                for px in row.chunks_exact(3) {
                    sum[0] += px[0] as u64;
                    sum[1] += px[1] as u64;
                    sum[2] += px[2] as u64;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u64;
            let o = (ty * tw + tx) * 3;
            for k in 0..3 {
                out[o + k] = ((sum[k] + n / 2) / n) as u8;
            }
        }
    }
    (out, tw, th)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mb3d-history-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn kinds_by_extension() {
        assert_eq!(Kind::of(Path::new("a/b.M3P")), Kind::Params);
        assert_eq!(Kind::of(Path::new("x.m3i")), Kind::M3i);
        assert_eq!(Kind::of(Path::new("x.JPG")), Kind::Image);
        assert_eq!(Kind::of(Path::new("x.m3a")), Kind::Animation);
        assert_eq!(Kind::of(Path::new("x.stl")), Kind::Mesh);
        assert_eq!(Kind::of(Path::new("x")), Kind::Other);
    }

    #[test]
    fn days_and_times() {
        assert_eq!(day_secs("1970-01-01"), Some(0));
        assert_eq!(day_secs("2026-10-29"), Some(1793232000));
        assert_eq!(day_secs("2026-13-01"), None);
        assert_eq!(day_secs("README.txt"), None);
        assert_eq!(time_of("07h09m22s 26_1"), Some(7 * 3600 + 9 * 60 + 22));
        assert_eq!(time_of("picture"), None);
        // the stamp of a second and the parsing of its names agree
        let secs = 1793232000 + 7 * 3600 + 9 * 60 + 22;
        let (d, t) = crate::appdirs::stamp_of(secs);
        assert_eq!(day_secs(&d).unwrap() + time_of(&t).unwrap(), secs);
    }

    #[test]
    fn calculations_and_logged_files_are_listed_newest_first_and_removed() {
        let root = temp_root("list");
        let day = root.join("2026-10-29");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(day.join("07h09m22s my picture.m3p"), b"x").unwrap();
        std::fs::write(day.join("08h24m24s.m3p"), b"x").unwrap();
        std::fs::write(day.join("README.txt"), b"notes").unwrap();
        let secs = day_secs("2026-10-29").unwrap();
        log_in(&root, Action::Open, Path::new("/pics/a.m3i"), secs + 8 * 3600);
        log_in(&root, Action::Save, Path::new("/pics/b.png"), secs + 9 * 3600);
        let l = list_in(&root);
        let titles: Vec<&str> = l.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["b.png", "", "a.m3i", "my picture"]);
        assert_eq!(l[0].action, Action::Save);
        assert_eq!(l[0].kind, Kind::Image);
        assert_eq!(l[0].bytes, None, "a logged file that is not there");
        assert_eq!(l[2].kind, Kind::M3i);
        assert_eq!(l[3].when(), "2026-10-29 07:09:22");
        assert!(l[3].thumb().is_none());
        assert_eq!(l[3].bytes, Some(1));
        assert!(l[3].modified > 1_700_000_000);
        // the days, and a scan finds the same
        let days = days_in(&root);
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].date, "2026-10-29");
        assert_eq!(list_day(&days[0]), l);
        let scan = Scan::start_in(root.clone(), Box::new(|| {}));
        let mut found = Vec::new();
        for _ in 0..200 {
            while let Some(f) = scan.next() {
                found.push(f);
            }
            if matches!(found.last(), Some(Found::Done)) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(found.len(), 3);
        assert!(matches!(&found[0], Found::Days(d) if d.len() == 1));
        assert!(matches!(&found[1], Found::Day(0, e) if *e == l));
        // a removed logged file: its line goes, the other stays
        assert!(remove_in(&root, &l[2]));
        assert!(!remove_in(&root, &l[2]));
        let l2 = list_in(&root);
        assert_eq!(l2.len(), 3);
        assert_eq!(l2[0].title, "b.png");
        // a removed calculation: its file goes
        assert!(remove_in(&root, &l2[2]));
        assert!(!day.join("07h09m22s my picture.m3p").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn content_keys_tell_the_same_bytes() {
        let root = temp_root("keys");
        std::fs::write(root.join("a.m3p"), b"same bytes").unwrap();
        std::fs::write(root.join("b.m3p"), b"same bytes").unwrap();
        std::fs::write(root.join("c.m3p"), b"other bytes").unwrap();
        let big: Vec<u8> = (0..400_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(root.join("big1.png"), &big).unwrap();
        let mut big2 = big.clone();
        big2[200_000] ^= 1; // (a change in the middle of a big file goes unseen)
        std::fs::write(root.join("big2.png"), &big2).unwrap();
        big2[5] ^= 1;
        std::fs::write(root.join("big3.png"), &big2).unwrap();
        let k = |n: &str| content_key(&root.join(n));
        assert_eq!(k("a.m3p"), k("b.m3p"));
        assert_ne!(k("a.m3p"), k("c.m3p"));
        assert_eq!(k("big1.png"), k("big2.png"));
        assert_ne!(k("big1.png"), k("big3.png"));
        assert_eq!(k("none.m3p"), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thumbnails_fit_the_box() {
        let (w, h) = (480, 180);
        let rgb: Vec<u8> = (0..w * h).flat_map(|i| [(i % 256) as u8, 7, 200]).collect();
        let (t, tw, th) = downscale(&rgb, w, h, THUMB_W, THUMB_H);
        assert_eq!((tw, th), (240, 90));
        assert_eq!(t.len(), 240 * 90 * 3);
        assert_eq!(t[1], 7);
        assert_eq!(t[2], 200);
        // a small picture is kept
        let (s, sw, sh) = downscale(&rgb[..30 * 3], 10, 3, THUMB_W, THUMB_H);
        assert_eq!((sw, sh), (10, 3));
        assert_eq!(s, &rgb[..90]);
        let root = temp_root("thumb");
        let m3p = root.join("08h00m00s a.m3p");
        write_thumb(&m3p, &rgb, w, h).unwrap();
        let e = Entry { secs: 0, action: Action::Calc, path: m3p, title: "a".into(), kind: Kind::Params, bytes: None, modified: 0 };
        assert!(e.thumb().is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn keys_are_kept_between_sessions_and_computed_on_a_thread() {
        let root = temp_root("keyfile");
        let a = root.join("a.m3p");
        std::fs::write(&a, b"same bytes").unwrap();
        let file = root.join("keys.txt");
        let mut k = Keys::default();
        assert_eq!(k.get(&a, 10, 5), None);
        k.put(a.clone(), 10, 5, Some(77));
        k.put(root.join("gone.m3p"), 3, 4, None);
        k.save_to(&file);
        let k2 = Keys::load_from(&file);
        assert_eq!(k2.len(), 2);
        assert_eq!(k2.get(&a, 10, 5), Some(Some(77)));
        assert_eq!(k2.get(&a, 11, 5), None, "a changed file is keyed again");
        assert_eq!(k2.get(&root.join("gone.m3p"), 3, 4), Some(None));
        // the thread computes what is missing and saves
        let shared = Arc::new(Mutex::new(Keys::default()));
        let busy = hash_keys(shared.clone(), vec![(a.clone(), 10, 5), (root.join("none.m3p"), 1, 1)], Box::new(|| {}));
        for _ in 0..500 {
            if !busy.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!busy.load(Ordering::Relaxed));
        let k3 = shared.lock().unwrap();
        assert_eq!(k3.get(&a, 10, 5), Some(content_key(&a)));
        assert_eq!(k3.get(&root.join("none.m3p"), 1, 1), Some(None));
        let _ = std::fs::remove_dir_all(&root);
    }
}
