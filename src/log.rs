//! The program's log: every event of a session, one line each, in
//! `logs/mb3d-<date>.log` of the data folder (a file a day, UTC, kept 30
//! days), and the newest lines in memory for the Log window.  A line is
//! `<date> <time>\t<level>\t<program>\t<kind>\t<text>`, tab-separated so
//! that a text with spaces stays one field; a text's own line breaks
//! become ` ⏎ `.
//!
//! A desktop session holds `logs/session-<pid>.lock` locked while it runs
//! and deletes it at its end; a lock file the next start can lock again
//! was left by a session that never ended (a freeze, a kill, a crash, a
//! power cut): [`init`] reports it.  A panic writes its message into the
//! lock file ([`mark_crash`]), so a crash is told apart from the rest.  Nothing is logged before [`init`] (the
//! library's functions log for whichever program uses them).  Writing is
//! synchronous under a lock: nothing is logged per pixel or per row, and a
//! line written is on disk even if the process dies the moment after.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// How many lines the Log window can show of the running sessions.
pub const KEEP_LINES: usize = 20_000;
/// Days a log file is kept.
pub const KEEP_DAYS: i64 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn tag(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    fn parse(s: &str) -> Option<Level> {
        Some(match s.trim() {
            "DEBUG" => Level::Debug,
            "INFO" => Level::Info,
            "WARN" => Level::Warn,
            "ERROR" => Level::Error,
            _ => return None,
        })
    }
}

/// One logged event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// milliseconds since 1970 (UTC)
    pub ms: i64,
    pub level: Level,
    /// the program: `gui` (the desktop program) or `cli` (the command line)
    pub program: String,
    /// what it is about: `session`, `ui`, `calc`, `file`, `message` ...
    pub kind: String,
    pub text: String,
}

impl Entry {
    /// `yyyy-mm-dd hh:mm:ss.mmm` (UTC)
    pub fn when(&self) -> String {
        let (d, t) = crate::appdirs::stamp_of(self.ms.div_euclid(1000));
        format!("{d} {}:{}:{}.{:03}", &t[0..2], &t[3..5], &t[6..8], self.ms.rem_euclid(1000))
    }

    /// The line in the file.
    pub fn line(&self) -> String {
        let text = self.text.trim_end().replace("\r\n", "\n").replace('\n', " ⏎ ").replace('\t', " ");
        format!("{}\t{}\t{}\t{}\t{}", self.when(), self.level.tag(), self.program, self.kind, text)
    }

    /// An entry from a line of a log file (None for another line).
    pub fn parse(line: &str) -> Option<Entry> {
        let mut it = line.splitn(5, '\t');
        let (when, level, program, kind, text) = (it.next()?, it.next()?, it.next()?, it.next()?, it.next()?);
        Some(Entry { ms: parse_when(when)?, level: Level::parse(level)?, program: program.into(), kind: kind.into(), text: text.into() })
    }
}

fn parse_when(s: &str) -> Option<i64> {
    // yyyy-mm-dd hh:mm:ss.mmm
    let b = s.as_bytes();
    if b.len() < 23 || b[10] != b' ' {
        return None;
    }
    let day = crate::history::day_secs_of(&s[0..10])?;
    let num = |a: usize, z: usize| s.get(a..z)?.parse::<i64>().ok();
    let (h, m, sec, ms) = (num(11, 13)?, num(14, 16)?, num(17, 19)?, num(20, 23)?);
    Some((day + h * 3600 + m * 60 + sec) * 1000 + ms)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

struct Logger {
    dir: PathBuf,
    program: String,
    state: Mutex<State>,
    /// counts the lines logged: the Log window reads again when it changed
    version: AtomicU64,
}

struct State {
    recent: VecDeque<Entry>,
    /// the open file and its day
    file: Option<(String, File)>,
    /// the session's lock file (desktop program only), kept locked
    lock: Option<(PathBuf, File)>,
}

static LOGGER: OnceLock<Logger> = OnceLock::new();
/// What [`init`] found of a session that never ended, for the program to
/// tell (taken once).
static UNCLEAN: Mutex<Option<Unclean>> = Mutex::new(None);

/// Sessions before this one that never ended.
#[derive(Clone, Debug, Default)]
pub struct Unclean {
    /// the program, process and start of each
    pub sessions: String,
    /// the panic message one of them left ([`mark_crash`]): it crashed
    pub crash: Option<String>,
}

/// A session found by [`stale_sessions`].
#[derive(Debug)]
struct Stale {
    desc: String,
    crash: Option<String>,
}

/// The folder of the log files.
pub fn folder() -> PathBuf {
    crate::appdirs::data_folder().join("logs")
}

/// Starts the log of program `program` (`gui` or `cli`), in [`folder`].
/// The desktop program (`lock`) also finds the sessions that never ended.
pub fn init(program: &str, lock: bool) {
    init_in(folder(), program, lock);
}

fn init_in(dir: PathBuf, program: &str, lock: bool) {
    if LOGGER.get().is_some() || std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    prune(&dir, now_ms() / 1000);
    let mut unclean = Vec::new();
    let mut own = None;
    if lock {
        unclean = stale_sessions(&dir);
        let p = dir.join(format!("session-{}.lock", std::process::id()));
        if let Ok(mut f) = OpenOptions::new().create(true).truncate(true).write(true).open(&p) {
            if f.try_lock().is_ok() {
                let _ = writeln!(f, "{program}\t{}\t{}", std::process::id(), Entry { ms: now_ms(), level: Level::Info, program: String::new(), kind: String::new(), text: String::new() }.when());
                own = Some((p, f));
            }
        }
    }
    let logger = Logger { dir, program: program.into(), state: Mutex::new(State { recent: VecDeque::new(), file: None, lock: own }), version: AtomicU64::new(0) };
    if LOGGER.set(logger).is_err() {
        return;
    }
    if !unclean.is_empty() {
        let sessions = unclean.iter().map(|u| u.desc.as_str()).collect::<Vec<_>>().join("; ");
        let crash = unclean.into_iter().find_map(|u| u.crash);
        match &crash {
            Some(c) => log(Level::Warn, "session", &format!("an earlier session crashed: {sessions}: {c} (details in {})", crate::crash::file().display())),
            None => log(Level::Warn, "session", &format!("an earlier session never ended (froze, was killed, crashed or lost its power): {sessions}")),
        }
        *UNCLEAN.lock().unwrap_or_else(|e| e.into_inner()) = Some(Unclean { sessions, crash });
    }
}

/// The lock files of sessions that are not running any more (their lock
/// can be taken): what they say, and they are deleted.
fn stale_sessions(dir: &Path) -> Vec<Stale> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("session-") && name.ends_with(".lock")) {
            continue;
        }
        let p = e.path();
        let Ok(mut f) = OpenOptions::new().read(true).write(true).open(&p) else { continue };
        if f.try_lock().is_err() {
            continue; // a session running now
        }
        // read through the locking handle: Windows locks are mandatory, a
        // second handle's read fails while this one holds the lock
        let mut info = String::new();
        let _ = f.read_to_string(&mut info);
        let mut lines = info.lines();
        let mut it = lines.next().unwrap_or("").trim().split('\t');
        let (prog, pid, start) = (it.next().unwrap_or("?"), it.next().unwrap_or("?"), it.next().unwrap_or("?"));
        // the first panic written (the later ones often follow from it)
        let crash = lines.find_map(|l| l.strip_prefix("crash\t")).map(|c| c.trim().to_string());
        out.push(Stale { desc: format!("{prog} (process {pid}) started {start} UTC"), crash });
        drop(f);
        let _ = std::fs::remove_file(&p);
    }
    out
}

/// Deletes the log files older than [`KEEP_DAYS`].
fn prune(dir: &Path, now_s: i64) {
    for (day, p) in days_in(dir) {
        if crate::history::day_secs_of(&day).is_some_and(|d| now_s - d > (KEEP_DAYS + 1) * 86400) {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// The session's first line: the program, its version and the system.
pub fn session_start(program_name: &str) {
    if !active() {
        return;
    }
    let (os, memory) = system();
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    let args: Vec<String> = std::env::args().skip(1).collect();
    info(
        "session",
        &format!(
            "{program_name} {} started (process {}) on {os}, {} {threads} threads, {:.1} GB memory; data folder {}{}",
            env!("CARGO_PKG_VERSION"),
            std::process::id(),
            std::env::consts::ARCH,
            memory as f64 / 1e9,
            crate::appdirs::data_folder().display(),
            if args.is_empty() { String::new() } else { format!("; arguments: {}", args.join(" ")) }
        ),
    );
}

/// The system's name and its memory in bytes.
#[cfg(any(windows, target_os = "macos"))]
fn system() -> (String, u64) {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    (sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.to_string()), sys.total_memory())
}

/// The system's name (`/etc/os-release`) and its memory (`/proc/meminfo`).
#[cfg(not(any(windows, target_os = "macos")))]
fn system() -> (String, u64) {
    let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let name = release.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=")).map(|n| n.trim_matches('"').to_string()).unwrap_or_else(|| std::env::consts::OS.to_string());
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease").map(|k| format!(" (kernel {})", k.trim())).unwrap_or_default();
    let mem = std::fs::read_to_string("/proc/meminfo").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("MemTotal:")).and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())).unwrap_or(0) * 1024;
    (format!("{name}{kernel}"), mem)
}

/// Writes a panic's message into the session's lock file: the next start
/// tells that this session crashed, and why.  It only tries the log's lock
/// (the panic may have come while it was held) and keeps one line.
pub fn mark_crash(text: &str) {
    let Some(l) = LOGGER.get() else { return };
    let Ok(mut st) = l.state.try_lock() else { return };
    if let Some((_, f)) = st.lock.as_mut() {
        let one: String = text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(300).collect();
        let _ = writeln!(f, "crash\t{one}");
        let _ = f.sync_data();
    }
}

/// A session that never ended, found at the start (once).
pub fn take_unclean() -> Option<Unclean> {
    UNCLEAN.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Whether the log is running.
pub fn active() -> bool {
    LOGGER.get().is_some()
}

/// Logs an event.
pub fn log(level: Level, kind: &str, text: &str) {
    log_at(now_ms(), level, kind, text);
}

/// Logs an event that happened at `ms` (milliseconds since 1970), a little
/// before now (a line held back while its control kept changing).
pub fn log_at(ms: i64, level: Level, kind: &str, text: &str) {
    let Some(l) = LOGGER.get() else { return };
    let e = Entry { ms, level, program: l.program.clone(), kind: kind.into(), text: text.into() };
    // (a panic elsewhere while the lock was held does not stop the log)
    let mut st = l.state.lock().unwrap_or_else(|p| p.into_inner());
    write_line(l, &mut st, &e);
    st.recent.push_back(e);
    while st.recent.len() > KEEP_LINES {
        st.recent.pop_front();
    }
    drop(st);
    l.version.fetch_add(1, Ordering::Release);
}

pub fn info(kind: &str, text: &str) {
    log(Level::Info, kind, text);
}

pub fn warn(kind: &str, text: &str) {
    log(Level::Warn, kind, text);
}

pub fn error(kind: &str, text: &str) {
    log(Level::Error, kind, text);
}

/// A line of the console (`elog!`): its level guessed from its words.
pub fn console(text: &str) {
    if !active() {
        return;
    }
    let low = text.to_ascii_lowercase();
    let level = if low.contains("panic") || low.contains("error") || low.contains("failed") || low.contains("cannot") || low.contains("could not") {
        Level::Error
    } else if low.contains("warning") || low.contains("not ") || low.contains("fall") {
        Level::Warn
    } else {
        Level::Info
    };
    log(level, "console", text);
}

fn write_line(l: &Logger, st: &mut State, e: &Entry) {
    let day = crate::appdirs::stamp_of(e.ms.div_euclid(1000)).0;
    if st.file.as_ref().is_none_or(|(d, _)| *d != day) {
        st.file = OpenOptions::new().create(true).append(true).open(l.dir.join(format!("mb3d-{day}.log"))).ok().map(|f| (day, f));
    }
    if let Some((_, f)) = st.file.as_mut() {
        // (one write of the whole line: two programs appending to the
        // same day's file do not mix their lines)
        let _ = f.write_all(format!("{}\n", e.line()).as_bytes());
    }
}

/// Logs the end of the session and gives its lock file up.
pub fn end(text: &str) {
    if !active() {
        return;
    }
    log(Level::Info, "session", text);
    if let Some(l) = LOGGER.get() {
        let mut st = l.state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some((p, f)) = st.lock.take() {
            drop(f);
            let _ = std::fs::remove_file(p);
        }
        if let Some((_, f)) = st.file.as_mut() {
            let _ = f.flush();
        }
    }
}

/// The lines of the running session(s) in memory, oldest first, and the
/// log's version.
pub fn recent() -> (u64, Vec<Entry>) {
    let Some(l) = LOGGER.get() else { return (0, Vec::new()) };
    let st = l.state.lock().unwrap_or_else(|p| p.into_inner());
    (l.version.load(Ordering::Acquire), st.recent.iter().cloned().collect())
}

/// The log's version (counts the lines logged).
pub fn version() -> u64 {
    LOGGER.get().map_or(0, |l| l.version.load(Ordering::Acquire))
}

/// The newest `n` lines as in the file, for a crash note (the lock is
/// only tried: a panic may have happened while it was held).
pub fn tail(n: usize) -> Vec<String> {
    let Some(l) = LOGGER.get() else { return Vec::new() };
    let Ok(st) = l.state.try_lock() else { return Vec::new() };
    let skip = st.recent.len().saturating_sub(n);
    st.recent.iter().skip(skip).map(Entry::line).collect()
}

/// The days that have a log file, newest first: (`yyyy-mm-dd`, file).
pub fn days() -> Vec<(String, PathBuf)> {
    days_in(&folder())
}

fn days_in(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            let day = n.strip_prefix("mb3d-")?.strip_suffix(".log")?.to_string();
            crate::history::day_secs_of(&day).map(|_| (day, e.path()))
        })
        .collect();
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out
}

/// The entries of a log file, oldest first.
pub fn read(path: &Path) -> Vec<Entry> {
    std::fs::read_to_string(path).unwrap_or_default().lines().filter_map(Entry::parse).collect()
}

/// `eprintln!` that also goes to the log (as a `console` line).
#[macro_export]
macro_rules! elog {
    ($($t:tt)*) => {{
        let s = format!($($t)*);
        eprintln!("{s}");
        $crate::log::console(&s);
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_go_to_the_file_and_come_back() {
        let e = Entry { ms: 1_793_232_000_123, level: Level::Warn, program: "gui".into(), kind: "calc".into(), text: "two\nlines\twith a tab".into() };
        let l = e.line();
        assert_eq!(l, "2026-10-29 00:00:00.123\tWARN\tgui\tcalc\ttwo ⏎ lines with a tab");
        let back = Entry::parse(&l).unwrap();
        assert_eq!(back.ms, e.ms);
        assert_eq!(back.level, Level::Warn);
        assert_eq!(back.text, "two ⏎ lines with a tab");
        assert!(Entry::parse("not a log line").is_none());
    }

    #[test]
    fn a_session_left_locked_is_found_and_old_files_go() {
        let dir = std::env::temp_dir().join(format!("mb3d-log-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // a lock file nobody holds: a session that never ended
        std::fs::write(dir.join("session-999999.lock"), "gui\t999999\t2026-10-29 10:00:00.000\n").unwrap();
        // a lock file held now: a session running
        let held = dir.join("session-888888.lock");
        let f = OpenOptions::new().create(true).truncate(true).write(true).open(&held).unwrap();
        f.try_lock().unwrap();
        std::fs::write(dir.join("mb3d-2020-01-01.log"), "old\n").unwrap();
        std::fs::write(dir.join("mb3d-2099-01-01.log"), "new\n").unwrap();
        let found = stale_sessions(&dir);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].desc.contains("999999") && found[0].desc.contains("2026-10-29"));
        assert!(found[0].crash.is_none());
        assert!(!dir.join("session-999999.lock").exists());
        assert!(held.exists());
        prune(&dir, now_ms() / 1000);
        assert!(!dir.join("mb3d-2020-01-01.log").exists());
        assert!(dir.join("mb3d-2099-01-01.log").exists());
        assert_eq!(days_in(&dir)[0].0, "2099-01-01");
        drop(f);
        // a session that crashed: the panic's line after the first
        std::fs::write(dir.join("session-777777.lock"), "gui\t777777\t2026-10-29 11:00:00.000\ncrash\tpanicked at src/x.rs:1:2: boom\ncrash\tlater\n").unwrap();
        let found = stale_sessions(&dir);
        assert_eq!(found.len(), 2, "{found:?}");
        let c = found.iter().find(|s| s.desc.contains("777777")).unwrap();
        assert_eq!(c.crash.as_deref(), Some("panicked at src/x.rs:1:2: boom"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
