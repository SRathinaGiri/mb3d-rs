//! The swarm: rendering shared between computers through a shared folder.
//!
//! No sockets: everything goes through files in one folder that all
//! machines can see (a NAS share, a Windows or Samba share, a sync
//! service), so the operating system's file sharing does the authentication
//! and the program opens no network port.
//!
//! | path in the swarm folder          | content |
//! |-----------------------------------|---------|
//! | `peers/<id>.txt`                  | a heartbeat per running program: name, version, cores, card, what it is doing, written every few seconds; stale after [`STALE_SECS`] |
//! | `jobs/<id>/job.txt`               | the job: its kind and settings (`key = value` lines) |
//! | `jobs/<id>/in/`                   | the inputs: the animation (`anim.m3k`), the parameter files and `list.txt` of a batch, the scene (`scene.m3s`) of a Monte Carlo render; `M3Formulas/` and `M3Maps/` with the formula and map files the scenes need |
//! | `jobs/<id>/out/`                  | the results, one file per unit: the frames, the pictures, or the pass blocks (`unit_<k>.m3c`) of a Monte Carlo render |
//! | `jobs/<id>/done`                  | written by the job's owner when the results are collected |
//!
//! A unit is claimed with a marker file next to its output (`<unit>.claim`,
//! holding the claimer's heartbeat id), created only if it is not there:
//! that is atomic whether a computer reaches the folder directly or over
//! the network, where file locks of the two kinds do not see each other.
//! The result is written under another name and renamed into place, then
//! the marker is removed: a unit is done when its output is there and its
//! marker is not. A unit whose claimer's heartbeat went stale is taken over.
//! A Monte Carlo render is split into blocks of passes with their own pass
//! numbers (the random rays differ), and the owner averages the blocks.
//!
//! A worker never writes outside the job's `out/` folder, never runs
//! anything but renders, and formula code from a job runs in the emulator
//! as every custom formula does.

use crate::anim::Animation;
use crate::batch::{BatchItem, BatchOptions, ItemResult};
use crate::frames::{FrameResult, FrameRun};
use crate::mc::McImage;
use crate::scene::Scene;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A heartbeat older than this means the instance is gone.
pub const STALE_SECS: u64 = 30;
/// Heartbeats are written this often while a unit is calculated.
const BEAT_SECS: f64 = 4.0;
/// The owner looks for finished units this often while it waits.
const WAIT_SECS: u64 = 5;
/// A job whose owner has sent no heartbeat for this long is abandoned:
/// any worker removes it.
pub const ORPHAN_SECS: u64 = 30 * 60;
/// An open-ended Monte Carlo block is saved as a snapshot this often, so
/// the owner shows the others' rays while they render (and keeps them if a
/// computer goes away).
const SNAP_SECS: f64 = 30.0;
/// After Stop, the owner waits this long at most for the other computers'
/// blocks of an open-ended Monte Carlo render.
const COLLECT_SECS: f64 = 120.0;
/// The blocks of an open-ended Monte Carlo job: one per computer, so many
/// that every computer that comes along gets one.
const OPEN_UNITS: usize = 64;
/// The seed salt of Monte Carlo block k is `(k + 1) * SALT_STRIDE`, so the
/// random rays of the blocks never repeat each other.
const SALT_STRIDE: u32 = 0x9E37;

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn io(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// `key = value` lines, as the scene files use them.
fn parse_kv(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                return None;
            }
            let (k, v) = l.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

fn format_kv(map: &BTreeMap<String, String>) -> String {
    map.iter().map(|(k, v)| format!("{k} = {v}\n")).collect()
}

/// Writes a file so that a reader never sees it half written: into a
/// temporary file next to it, then renamed over it.
fn write_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&tmp, data).map_err(|e| io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| io(path, e))
}

/// A name for this machine: the host name the system tells, else "pc".
pub fn machine_name() -> String {
    ["HOSTNAME", "COMPUTERNAME", "HOST"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .map(|s| s.trim().to_string())
        .find(|s| !s.is_empty())
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "pc".into())
}

/// Letters, digits, `-` and `_` only (for file names made from names).
fn sanitised(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    if t.is_empty() {
        "x".into()
    } else {
        t
    }
}

/// The last part of a file name another computer put into a job
/// ("a/b\\c.png" gives "c.png"); None for "", "." and "..". A job's
/// names never reach outside its folder.
fn bare_name(s: &str) -> Option<&str> {
    let n = s.trim().rsplit(['/', '\\', ':']).next()?;
    (!n.is_empty() && n != "." && n != "..").then_some(n)
}

/// A name that file names are made from, without path separators.
fn safe_stem(s: &str) -> String {
    s.chars().map(|c| if matches!(c, '/' | '\\' | ':') { '_' } else { c }).collect()
}

/// A scene from a job: its background picture is looked up by name only
/// (in the job's maps and this computer's map folders), never by a path
/// on this computer.
fn job_scene(scene: &mut Scene) {
    let l = &mut scene.lighting;
    if !l.bg_image.is_empty() {
        l.bg_image = bare_name(&l.bg_image).unwrap_or_default().to_string();
    }
}

fn random_suffix() -> String {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let x = (t as u64) ^ ((std::process::id() as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
    format!("{:06x}", (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) & 0xFF_FFFF)
}

// ---------------------------------------------------------------------------
// peers

/// One running instance, as its heartbeat file says.
#[derive(Clone, Debug, PartialEq)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub version: String,
    pub cores: usize,
    pub card: String,
    /// what it is doing ("idle", "rendering ...")
    pub state: String,
    /// seconds since the epoch of the last heartbeat
    pub time: u64,
}

impl Peer {
    pub fn stale(&self, now: u64) -> bool {
        now.saturating_sub(self.time) > STALE_SECS
    }
}

/// This program's presence in one swarm folder: one heartbeat file, shared
/// by every part of the program that takes part (the Swarm window's worker,
/// the renders that made jobs), so the computer is listed once.
struct Presence {
    dir: PathBuf,
    peer: Peer,
    /// what each part is doing, by [`Heart::part`]; the heartbeat file is
    /// written and removed under this lock
    parts: Mutex<BTreeMap<u64, String>>,
    last: Mutex<Option<Instant>>,
}

/// The presences of this program, by swarm folder and name.
type Presences = Vec<((PathBuf, String), Weak<Presence>)>;
static PRESENCES: Mutex<Presences> = Mutex::new(Vec::new());
static NEXT_PART: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl Presence {
    /// The presence for this folder and name, shared while any part holds it.
    fn get(swarm: &Path, name: &str) -> Arc<Presence> {
        let key = (swarm.canonicalize().unwrap_or_else(|_| swarm.to_path_buf()), name.to_string());
        let mut all = PRESENCES.lock().unwrap_or_else(|e| e.into_inner());
        all.retain(|(_, w)| w.strong_count() > 0);
        if let Some(p) = all.iter().find(|(k, _)| *k == key).and_then(|(_, w)| w.upgrade()) {
            return p;
        }
        let id = format!("{}-{}", sanitised(name), random_suffix());
        #[cfg(feature = "gpu")]
        let card = crate::gpu::name().unwrap_or_default();
        #[cfg(not(feature = "gpu"))]
        let card = String::new();
        let p = Arc::new(Presence {
            dir: swarm.join("peers"),
            peer: Peer {
                id,
                name: name.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                cores: std::thread::available_parallelism().map_or(1, |n| n.get()),
                card,
                state: "idle".into(),
                time: 0,
            },
            parts: Mutex::new(BTreeMap::new()),
            last: Mutex::new(None),
        });
        all.push((key, Arc::downgrade(&p)));
        // the heartbeat goes on while the program runs, also through long
        // steps without progress (a big tile being saved over the network):
        // the claims of a computer whose heartbeat stops are taken over
        let weak = Arc::downgrade(&p);
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_secs_f64(BEAT_SECS));
            let Some(p) = weak.upgrade() else { return };
            let parts = p.parts.lock().unwrap_or_else(|e| e.into_inner());
            let due = p.last.lock().unwrap_or_else(|e| e.into_inner()).is_none_or(|t| t.elapsed().as_secs_f64() >= BEAT_SECS);
            if !parts.is_empty() && due {
                let _ = p.write(&parts);
            }
        });
        p
    }

    fn file(&self) -> PathBuf {
        self.dir.join(format!("{}.txt", self.peer.id))
    }

    /// Writes the heartbeat with what the parts are doing ("idle" when
    /// none of them is busy).
    fn write(&self, parts: &BTreeMap<u64, String>) -> Result<(), String> {
        let busy: Vec<&str> = parts.values().map(|s| s.as_str()).filter(|s| *s != "idle").collect();
        std::fs::create_dir_all(&self.dir).map_err(|e| io(&self.dir, e))?;
        let mut m = BTreeMap::new();
        m.insert("name".to_string(), self.peer.name.clone());
        m.insert("version".to_string(), self.peer.version.clone());
        m.insert("cores".to_string(), self.peer.cores.to_string());
        m.insert("card".to_string(), self.peer.card.clone());
        m.insert("state".to_string(), if busy.is_empty() { "idle".to_string() } else { busy.join("; ").replace('\n', " ") });
        m.insert("time".to_string(), now_secs().to_string());
        write_atomic(&self.file(), format_kv(&m).as_bytes())?;
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        Ok(())
    }
}

/// A part of this program in the swarm (the worker, or a render that made
/// a job): what it is doing goes into the program's one heartbeat.
pub struct Heart {
    presence: Arc<Presence>,
    part: u64,
    state: Mutex<String>,
    left: std::sync::atomic::AtomicBool,
    /// the Monte Carlo block this process is rendering, after each pass
    /// (with a count that changes with it), when an owner shows it live
    block: Mutex<Option<(u64, McImage)>>,
    want_block: std::sync::atomic::AtomicBool,
    block_seq: std::sync::atomic::AtomicU64,
}

impl Heart {
    /// `name`: "" = the machine's name. Parts with the same folder and name
    /// share the heartbeat, and so the id.
    pub fn new(swarm: &Path, name: &str) -> Heart {
        let name = if name.trim().is_empty() { machine_name() } else { name.trim().to_string() };
        Heart {
            presence: Presence::get(swarm, &name),
            part: NEXT_PART.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            state: Mutex::new("idle".into()),
            left: std::sync::atomic::AtomicBool::new(false),
            block: Mutex::new(None),
            want_block: std::sync::atomic::AtomicBool::new(false),
            block_seq: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn id(&self) -> &str {
        &self.presence.peer.id
    }
    /// What this part last said it was doing.
    pub fn state(&self) -> String {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub fn name(&self) -> &str {
        &self.presence.peer.name
    }

    /// Writes the heartbeat now.
    pub fn beat_now(&self, state: &str) -> Result<(), String> {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = state.to_string();
        let mut parts = self.presence.parts.lock().unwrap_or_else(|e| e.into_inner());
        parts.insert(self.part, state.to_string());
        self.presence.write(&parts)
    }

    /// Writes the heartbeat if the last one is older than a few seconds
    /// (called from progress callbacks).
    pub fn beat(&self, state: &str) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = state.to_string();
        let mut parts = self.presence.parts.lock().unwrap_or_else(|e| e.into_inner());
        parts.insert(self.part, state.to_string());
        let due = self.presence.last.lock().unwrap_or_else(|e| e.into_inner()).is_none_or(|t| t.elapsed().as_secs_f64() >= BEAT_SECS);
        if due {
            let _ = self.presence.write(&parts);
        }
    }

    /// This part leaves the swarm; the heartbeat is removed with the last
    /// part of the program (also when a part is dropped without leaving).
    pub fn leave(&self) {
        if self.left.swap(true, std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        let mut parts = self.presence.parts.lock().unwrap_or_else(|e| e.into_inner());
        if parts.remove(&self.part).is_none() {
            return;
        }
        if parts.is_empty() {
            let _ = std::fs::remove_file(self.presence.file());
        } else {
            let _ = self.presence.write(&parts);
        }
    }
}

impl Drop for Heart {
    fn drop(&mut self) {
        self.leave();
    }
}

/// The other computers in the swarm: fresh heartbeats that are not this
/// program's (`name`: as for [`Heart::new`]), any version.
pub fn others(swarm: &Path, name: &str) -> Vec<Peer> {
    let name = if name.trim().is_empty() { machine_name() } else { name.trim().to_string() };
    let key = (swarm.canonicalize().unwrap_or_else(|_| swarm.to_path_buf()), name);
    let own = PRESENCES.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(k, _)| *k == key).and_then(|(_, w)| w.upgrade()).map(|p| p.peer.id.clone());
    let now = now_secs();
    peers(swarm).into_iter().filter(|p| !p.stale(now) && Some(&p.id) != own.as_ref()).collect()
}

/// Whether another computer that can help is in the swarm (one of
/// [`others`] with this version).
pub fn others_online(swarm: &Path, name: &str) -> bool {
    others(swarm, name).iter().any(|p| p.version == env!("CARGO_PKG_VERSION"))
}

/// The instances in the swarm folder, the fresh ones first; stale
/// heartbeats are listed too (`Peer::stale`), so the panel can show them.
pub fn peers(swarm: &Path) -> Vec<Peer> {
    let dir = swarm.join("peers");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut v: Vec<Peer> = rd
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "txt"))
        .filter_map(|e| {
            let text = std::fs::read_to_string(e.path()).ok()?;
            let m = parse_kv(&text);
            Some(Peer {
                id: e.path().file_stem()?.to_string_lossy().into_owned(),
                name: m.get("name").cloned().unwrap_or_default(),
                version: m.get("version").cloned().unwrap_or_default(),
                cores: m.get("cores").and_then(|c| c.parse().ok()).unwrap_or(0),
                card: m.get("card").cloned().unwrap_or_default(),
                state: m.get("state").cloned().unwrap_or_default(),
                time: m.get("time").and_then(|t| t.parse().ok()).unwrap_or(0),
            })
        })
        .collect();
    v.sort_by(|a, b| b.time.cmp(&a.time).then(a.name.cmp(&b.name)));
    v
}

/// Removes heartbeat files older than ten times the stale time.
fn sweep_peers(swarm: &Path) {
    let now = now_secs();
    for p in peers(swarm) {
        if now.saturating_sub(p.time) > 10 * STALE_SECS {
            let _ = std::fs::remove_file(swarm.join("peers").join(format!("{}.txt", p.id)));
        }
    }
}

// ---------------------------------------------------------------------------
// jobs

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobKind {
    Animation,
    Batch,
    MonteCarlo,
    /// one picture in tiles (Calculate 3D of a big picture)
    Tiles,
}

impl JobKind {
    fn as_str(self) -> &'static str {
        match self {
            JobKind::Animation => "animation",
            JobKind::Batch => "batch",
            JobKind::MonteCarlo => "montecarlo",
            JobKind::Tiles => "tiles",
        }
    }
    fn parse(s: &str) -> Option<JobKind> {
        match s {
            "animation" => Some(JobKind::Animation),
            "batch" => Some(JobKind::Batch),
            "montecarlo" => Some(JobKind::MonteCarlo),
            "tiles" => Some(JobKind::Tiles),
            _ => None,
        }
    }
}

/// A job in the swarm folder.
#[derive(Clone, Debug)]
pub struct Job {
    pub id: String,
    pub dir: PathBuf,
    pub kind: JobKind,
    /// a name for the panel (the animation's, the first file's, ...)
    pub name: String,
    /// the peer that created it (its name), and its heartbeat id
    pub owner: String,
    pub owner_id: String,
    /// the program version that made it (only that version renders it)
    pub version: String,
    pub created: u64,
    /// the kind's settings
    pub settings: BTreeMap<String, String>,
}

impl Job {
    pub fn in_dir(&self) -> PathBuf {
        self.dir.join("in")
    }
    pub fn out_dir(&self) -> PathBuf {
        self.dir.join("out")
    }
    pub fn done(&self) -> bool {
        self.dir.join("done").exists()
    }
    pub fn kind_name(&self) -> &'static str {
        self.kind.as_str()
    }
    /// An open-ended Monte Carlo render: every computer renders one block
    /// until the owner writes [`Job::stop_file`].
    pub fn open(&self) -> bool {
        self.kind == JobKind::MonteCarlo && self.setting("open").unwrap_or(false)
    }
    fn stop_file(&self) -> PathBuf {
        self.dir.join("stop")
    }
    /// The owner asked the blocks of an open-ended render to be saved.
    pub fn stopping(&self) -> bool {
        self.stop_file().exists()
    }
    /// The tiles of a tiles job (all of them, or the subset asked for).
    pub fn tile_indices(&self) -> Vec<usize> {
        match self.settings.get("tiles") {
            Some(list) => list.split_whitespace().filter_map(|k| k.parse().ok()).collect(),
            None => (0..self.setting("units").unwrap_or(0)).collect(),
        }
    }
    /// Made by this version of the program.
    pub fn same_version(&self) -> bool {
        self.version == env!("CARGO_PKG_VERSION")
    }
    /// The owner has sent no heartbeat for [`ORPHAN_SECS`] (and the job
    /// is at least that old): nobody will collect it.
    pub fn orphaned(&self, swarm: &Path, now: u64) -> bool {
        if now.saturating_sub(self.created) < ORPHAN_SECS {
            return false;
        }
        let owner = peers(swarm).into_iter().find(|p| p.id == self.owner_id);
        owner.is_none_or(|p| now.saturating_sub(p.time) > ORPHAN_SECS)
    }
    /// The errors workers met in this job: (peer name, message).
    pub fn errors(&self) -> Vec<(String, String)> {
        let Ok(rd) = std::fs::read_dir(self.dir.join("errors")) else { return Vec::new() };
        let mut v: Vec<(String, String)> = rd
            .flatten()
            .filter_map(|e| {
                let text = std::fs::read_to_string(e.path()).ok()?;
                let (who, msg) = text.split_once('\n')?;
                Some((who.to_string(), msg.trim().to_string()))
            })
            .collect();
        v.sort();
        v
    }
    /// Notes an error this peer met in the job (for the panels and the
    /// owner).
    pub fn note_error(&self, heart: &Heart, msg: &str) {
        let d = self.dir.join("errors");
        if std::fs::create_dir_all(&d).is_ok() {
            let _ = write_atomic(&d.join(format!("{}.txt", heart.id())), format!("{}\n{msg}\n", heart.name()).as_bytes());
        }
    }
    fn setting<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.settings.get(key).and_then(|v| v.parse().ok())
    }

    /// The output files the job's units write, in order (the unit count is
    /// their number).
    pub fn unit_files(&self) -> Vec<PathBuf> {
        let n: usize = self.setting("units").unwrap_or(0);
        let out = self.out_dir();
        match self.kind {
            JobKind::MonteCarlo => (0..n).map(|k| out.join(format!("unit_{k}.m3c"))).collect(),
            JobKind::Tiles => self.tile_indices().into_iter().map(|k| out.join(format!("tile_{k}.m3i"))).collect(),
            _ => (0..n)
                .map(|k| out.join(self.settings.get(&format!("unit_{k}")).and_then(|u| bare_name(u)).map(String::from).unwrap_or_else(|| format!("unit_{k}"))))
                .collect(),
        }
    }

    /// Finished units and all units (of an open-ended render: the blocks
    /// taken so far).
    pub fn progress(&self) -> (usize, usize) {
        let files = self.unit_files();
        let done = files.iter().filter(|f| is_done_file(f)).count();
        let all = if self.open() { files.iter().filter(|f| f.exists() || marker_of(f).exists()).count() } else { files.len() };
        (done, all)
    }
}

fn write_job(job: &Job) -> Result<(), String> {
    let mut m = job.settings.clone();
    m.insert("kind".into(), job.kind.as_str().into());
    m.insert("name".into(), job.name.replace('\n', " "));
    m.insert("owner".into(), job.owner.clone());
    m.insert("owner_id".into(), job.owner_id.clone());
    m.insert("created".into(), job.created.to_string());
    m.insert("version".into(), job.version.clone());
    write_atomic(&job.dir.join("job.txt"), format_kv(&m).as_bytes())
}

fn read_job(dir: &Path) -> Option<Job> {
    let text = std::fs::read_to_string(dir.join("job.txt")).ok()?;
    let mut m = parse_kv(&text);
    let kind = JobKind::parse(m.remove("kind")?.as_str())?;
    Some(Job {
        id: dir.file_name()?.to_string_lossy().into_owned(),
        dir: dir.to_path_buf(),
        kind,
        name: m.remove("name").unwrap_or_default(),
        owner: m.remove("owner").unwrap_or_default(),
        owner_id: m.remove("owner_id").unwrap_or_default(),
        version: m.remove("version").unwrap_or_default(),
        created: m.remove("created").and_then(|c| c.parse().ok()).unwrap_or(0),
        settings: m,
    })
}

/// The jobs in the swarm folder, oldest first; finished ones (with their
/// `done` marker) too.
pub fn jobs(swarm: &Path) -> Vec<Job> {
    let Ok(rd) = std::fs::read_dir(swarm.join("jobs")) else { return Vec::new() };
    let mut v: Vec<Job> = rd.flatten().filter(|e| e.path().is_dir()).filter_map(|e| read_job(&e.path())).collect();
    v.sort_by(|a, b| a.created.cmp(&b.created).then(a.id.cmp(&b.id)));
    v
}

/// Creates a job folder with `in/` and `out/`.
fn new_job(swarm: &Path, kind: JobKind, name: &str, owner: &Heart, settings: BTreeMap<String, String>) -> Result<Job, String> {
    let created = now_secs();
    let id = format!("{created}-{}-{}", sanitised(name).chars().take(24).collect::<String>(), random_suffix());
    let dir = swarm.join("jobs").join(&id);
    for d in [dir.join("in"), dir.join("out")] {
        std::fs::create_dir_all(&d).map_err(|e| io(&d, e))?;
    }
    let job = Job {
        id,
        dir,
        kind,
        name: name.to_string(),
        owner: owner.name().to_string(),
        owner_id: owner.id().to_string(),
        version: env!("CARGO_PKG_VERSION").into(),
        created,
        settings,
    };
    write_job(&job)?;
    Ok(job)
}

/// Copies the formula and map files the scene uses into the job's
/// `in/M3Formulas` and `in/M3Maps`, so machines without them can render it.
fn copy_scene_files(job: &Job, scene: &Scene) -> Result<(), String> {
    let copy_to = |src: &Path, sub: &str| -> Result<(), String> {
        let d = job.in_dir().join(sub);
        std::fs::create_dir_all(&d).map_err(|e| io(&d, e))?;
        let dst = d.join(src.file_name().ok_or("file without a name")?);
        if !dst.exists() {
            std::fs::copy(src, &dst).map_err(|e| io(src, e))?;
        }
        Ok(())
    };
    for f in &scene.formulas {
        if let crate::formulas::Formula::Custom(c) = &f.formula {
            if let Some(p) = crate::formulas::custom_path(c.name()) {
                copy_to(&p, "M3Formulas")?;
            }
        }
    }
    let l = &scene.lighting;
    let mut maps: Vec<i32> = l.lights.iter().filter(|x| x.on && x.map != 0).map(|x| x.map as i32).collect();
    if l.diff_map != 0 {
        maps.push(l.diff_map as i32);
    }
    for nr in maps {
        if let Some(p) = crate::maps::find_map_file(nr) {
            copy_to(&p, "M3Maps")?;
        }
    }
    if !l.bg_image.is_empty() {
        let name = Path::new(&l.bg_image).file_name().map(|n| n.to_os_string());
        if let Some(name) = name {
            let found = std::iter::once(PathBuf::from(&l.bg_image))
                .chain(crate::maps::map_dir_list().into_iter().map(|d| d.join(&name)))
                .find(|p| p.is_file());
            if let Some(p) = found {
                copy_to(&p, "M3Maps")?;
            }
        }
    }
    Ok(())
}

/// Makes a job's formula and map files known to this process (once per job).
fn use_job_files(job: &Job, seen: &mut HashSet<String>) {
    if !seen.insert(job.id.clone()) {
        return;
    }
    let f = job.in_dir().join("M3Formulas");
    if f.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&f) {
            for e in rd.flatten() {
                if let Some(stem) = e.path().file_stem() {
                    crate::formulas::forget_custom(&stem.to_string_lossy());
                }
            }
        }
        crate::formulas::add_formula_dir(f);
    }
    let m = job.in_dir().join("M3Maps");
    if m.is_dir() {
        crate::maps::add_map_dir(m);
    }
}

/// The marker of a unit being calculated: `<output>.claim`.
fn marker_of(out: &Path) -> PathBuf {
    let mut m = out.as_os_str().to_owned();
    m.push(".claim");
    PathBuf::from(m)
}

/// A finished unit: its output is there (renamed into place whole, or
/// written by an older claim) and no computer has its marker.
fn is_done_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.len() > 1).unwrap_or(false) && !marker_of(p).exists()
}

/// A unit some computer is calculating: its marker is there and the
/// computer's heartbeat is fresh.
fn is_busy_file(swarm: &Path, p: &Path) -> bool {
    let m = marker_of(p);
    m.exists() && !marker_stale(swarm, &m)
}

/// A marker whose computer is gone: its heartbeat is stale or missing (an
/// empty marker, from a computer that went away while making it: by its
/// age).
fn marker_stale(swarm: &Path, marker: &Path) -> bool {
    let id = std::fs::read_to_string(marker).unwrap_or_default();
    let id = id.trim();
    let now = now_secs();
    if id.is_empty() {
        let age = std::fs::metadata(marker).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).map_or(0, |d| d.as_secs());
        return age > STALE_SECS;
    }
    peers(swarm).iter().find(|p| p.id == id).is_none_or(|p| p.stale(now))
}

/// A unit claimed by this computer: its marker, removed when the claim is
/// dropped (after the result is in place, or when the unit is given up).
struct UnitClaim {
    marker: PathBuf,
}

impl Drop for UnitClaim {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.marker);
    }
}

/// Claims the unit with output `out`: None when it is done or another
/// computer has it. A unit whose computer is gone is taken over (its
/// marker renamed away first, so only one computer takes it; a half
/// written output of it is removed).
fn claim_unit(job: &Job, heart: &Heart, out: &Path) -> Result<Option<UnitClaim>, String> {
    let swarm = job.dir.parent().and_then(|p| p.parent()).unwrap_or(&job.dir);
    if is_done_file(out) {
        return Ok(None);
    }
    let marker = marker_of(out);
    if let Some(d) = marker.parent() {
        std::fs::create_dir_all(d).map_err(|e| io(d, e))?;
    }
    for attempt in 0..2 {
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&marker) {
            Ok(mut f) => {
                use std::io::Write;
                let claim = UnitClaim { marker: marker.clone() };
                f.write_all(heart.id().as_bytes()).map_err(|e| io(&marker, e))?;
                // finished by another computer just before (its marker
                // was removed after its result was in place)
                if std::fs::metadata(out).map(|m| m.len() > 1).unwrap_or(false) {
                    return Ok(None);
                }
                return Ok(Some(claim));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if attempt > 0 || !marker_stale(swarm, &marker) {
                    return Ok(None);
                }
                let stale_id = std::fs::read_to_string(&marker).unwrap_or_default();
                let mut private = marker.as_os_str().to_owned();
                private.push(format!(".{}", heart.id()));
                let private = PathBuf::from(private);
                if std::fs::rename(&marker, &private).is_err() {
                    // another computer took it over first
                    return Ok(None);
                }
                if std::fs::read_to_string(&private).unwrap_or_default() != stale_id {
                    // the marker renamed was a new one of another computer
                    let _ = std::fs::rename(&private, &marker);
                    return Ok(None);
                }
                let _ = std::fs::remove_file(&private);
                let _ = std::fs::remove_file(out);
            }
            Err(e) => return Err(io(&marker, e)),
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// the units of the three kinds

/// Renders the units of a job that nobody has: returns how many this call
/// rendered. `threads`: 0 = all cores. `cancel` gives the unit under way
/// up, `no_more` lets it finish but takes no further one.
#[allow(clippy::too_many_arguments)]
pub fn work_on(job: &Job, heart: &Heart, threads: usize, quiet: bool, seen: &mut HashSet<String>, cancel: &(dyn Fn() -> bool + Sync), no_more: &(dyn Fn() -> bool + Sync)) -> Result<usize, String> {
    if job.done() || cancel() || !job.dir.join("job.txt").exists() {
        return Ok(0);
    }
    // the owner finished or gave the job up: stop too (looked at once a
    // second, the folder may be on a network share)
    let ended = std::sync::atomic::AtomicBool::new(false);
    let looked = Mutex::new(Instant::now());
    let cancel_outer = cancel;
    let cancel = &|| {
        if cancel_outer() || ended.load(std::sync::atomic::Ordering::Relaxed) {
            return true;
        }
        let mut t = looked.lock().unwrap_or_else(|e| e.into_inner());
        if t.elapsed() >= Duration::from_secs(1) {
            *t = Instant::now();
            if job.done() || !job.dir.join("job.txt").exists() {
                ended.store(true, std::sync::atomic::Ordering::Relaxed);
                return true;
            }
        }
        false
    };
    use_job_files(job, seen);
    let label = |unit: &str| format!("rendering {} ({}): {unit}", job.name, job.kind.as_str());
    let mut n = 0;
    match job.kind {
        JobKind::Animation => {
            let f = crate::animfile::load(&job.in_dir().join("anim.m3k"))?;
            let mut a = f.anim;
            a.output_folder = job.out_dir().to_string_lossy().into_owned();
            a.overwrite = false;
            a.name = safe_stem(&a.name);
            for k in a.keyframes.iter_mut() {
                job_scene(&mut k.scene);
            }
            let run = FrameRun { threads, ..Default::default() };
            let frames: Vec<usize> = match job.settings.get("frames") {
                Some(list) => list.split_whitespace().filter_map(|f| f.parse().ok()).filter(|&f| f < a.frame_count()).collect(),
                None => (0..a.frame_count()).collect(),
            };
            for frame in frames {
                if cancel() || no_more() {
                    break;
                }
                if a.stereo_eyes().iter().all(|(_, eye)| is_done_file(&a.frame_file_eye(frame, eye))) {
                    continue;
                }
                // the frame's claim is on the file of its first eye
                let Some(first) = a.stereo_eyes().first().map(|(_, eye)| a.frame_file_eye(frame, eye)) else { continue };
                let Some(_claim) = claim_unit(job, heart, &first)? else { continue };
                let unit = format!("frame {}", a.file_index(frame));
                let progress = |d: usize, t: usize| heart.beat(&format!("{} {:.0} %", label(&unit), 100.0 * d as f64 / t.max(1) as f64));
                match crate::frames::render_frame_to_file(&a, frame, &run, &progress, cancel)? {
                    FrameResult::Written { seconds, .. } => {
                        n += 1;
                        if !quiet {
                            eprintln!("  {} done ({seconds:.1} s)", label(&unit));
                        }
                    }
                    FrameResult::Skipped { .. } => {}
                }
            }
        }
        JobKind::Batch => {
            let list = std::fs::read_to_string(job.in_dir().join("list.txt")).map_err(|e| io(&job.in_dir().join("list.txt"), e))?;
            let mut items = crate::batch::parse_list(&list, &job.in_dir())?;
            let mut opts = BatchOptions { output_dir: Some(job.out_dir()), skip_existing: true, threads, ..Default::default() };
            if let Some(f) = job.settings.get("format") {
                opts.format = crate::anim::OutputFormat::parse(f)?;
            }
            opts.scale = job.setting("scale");
            opts.aa = job.setting("aa").unwrap_or(1);
            opts.depth = job.setting("depth").unwrap_or(false);
            // the parameter files are the job's own, the background
            // pictures looked up by name
            for (line, it) in list.lines().filter(|l| !l.trim().is_empty() && !l.trim().starts_with(['#', ';'])).zip(items.iter_mut()) {
                let file = line.split('|').next().unwrap_or("").trim();
                if bare_name(file) != Some(file) {
                    return Err(format!("job list: '{file}' is not a file of the job"));
                }
                if let Ok((sc, _, _)) = crate::batch::item_scene(it, &opts) {
                    let bg = &sc.lighting.bg_image;
                    if !bg.is_empty() && bare_name(bg) != Some(bg.as_str()) {
                        it.overrides.push(format!("background_image = {}", bare_name(bg).unwrap_or_default()));
                    }
                }
            }
            let outputs = crate::batch::output_paths(&items, &opts);
            for (it, out) in items.iter().zip(&outputs) {
                if cancel() || no_more() {
                    break;
                }
                let Some(_claim) = claim_unit(job, heart, out)? else { continue };
                let unit = it.input.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let progress = |d: usize, t: usize| heart.beat(&format!("{} {:.0} %", label(&unit), 100.0 * d as f64 / t.max(1) as f64));
                match crate::batch::render_item(it, out, &opts, &progress, cancel) {
                    ItemResult::Done { seconds, .. } => {
                        n += 1;
                        if !quiet {
                            eprintln!("  {} done ({seconds:.1} s)", label(&unit));
                        }
                    }
                    ItemResult::Skipped { .. } => {}
                    ItemResult::Failed { error } => return Err(format!("{}: {error}", it.input.display())),
                }
            }
        }
        JobKind::MonteCarlo => {
            let text = std::fs::read_to_string(job.in_dir().join("scene.m3s")).map_err(|e| io(&job.in_dir().join("scene.m3s"), e))?;
            let mut scene = Scene::parse(&text)?;
            job_scene(&mut scene);
            if threads > 0 {
                scene.threads = threads;
            }
            let units: usize = job.setting("units").unwrap_or(1);
            let rays: f64 = job.setting("rays").unwrap_or(16.0);
            let passes: u32 = job.setting("passes").unwrap_or(0);
            let time: f64 = job.setting("time").unwrap_or(0.0);
            // an open-ended block ends when the owner asks for the blocks
            // or this computer stops sharing (looked at once a second)
            let open = job.open();
            let stopped = std::sync::atomic::AtomicBool::new(false);
            let looked = Mutex::new(Instant::now() - Duration::from_secs(2));
            let save = || {
                if !open {
                    return false;
                }
                if stopped.load(std::sync::atomic::Ordering::Relaxed) || no_more() {
                    return true;
                }
                let mut t = looked.lock().unwrap_or_else(|e| e.into_inner());
                if t.elapsed() >= Duration::from_secs(1) {
                    *t = Instant::now();
                    if job.stopping() {
                        stopped.store(true, std::sync::atomic::Ordering::Relaxed);
                        return true;
                    }
                }
                false
            };
            for k in 0..units {
                if cancel() || no_more() || save() {
                    break;
                }
                let out = job.out_dir().join(format!("unit_{k}.m3c"));
                let Some(claim) = claim_unit(job, heart, &out)? else { continue };
                let unit = format!("block {}/{units}", k + 1);
                let mut img = McImage::new(scene.width as usize, scene.height as usize);
                // the block's own random rays
                img.seed_salt = (k as u32 + 1).wrapping_mul(SALT_STRIDE);
                let t0 = Instant::now();
                let mut snapped = Instant::now();
                let mut done_passes = 0u32;
                loop {
                    let st = img.stats();
                    if open {
                        if save() {
                            break;
                        }
                    } else if (!st.zero_counts && st.avg_rays >= rays) || (passes > 0 && done_passes >= passes) || (time > 0.0 && t0.elapsed().as_secs_f64() >= time) {
                        break;
                    }
                    let progress = |d: usize, t: usize| heart.beat(&format!("{} pass {} {:.0} %", label(&unit), done_passes + 1, 100.0 * d as f64 / t.max(1) as f64));
                    let deadline = (time > 0.0).then(|| t0 + Duration::from_secs_f64(time));
                    // (a pass stopped half way keeps the rows it finished:
                    // the merge weighs every pixel by its rays)
                    let stop = || cancel() || save() || deadline.is_some_and(|d| Instant::now() > d);
                    match crate::mc::pass(&scene, &mut img, threads, &progress, &stop) {
                        Ok(()) => {}
                        Err(e) if e == "cancelled" => break,
                        Err(e) => return Err(e),
                    }
                    done_passes += 1;
                    if heart.want_block.load(std::sync::atomic::Ordering::Relaxed) {
                        let n = heart.block_seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        *heart.block.lock().unwrap_or_else(|e| e.into_inner()) = Some((n, img.clone()));
                    }
                    if open && snapped.elapsed().as_secs_f64() >= SNAP_SECS {
                        let _ = write_atomic(&out.with_extension("snap"), &crate::mc::write_m3c(&scene, &img));
                        snapped = Instant::now();
                    }
                }
                *heart.block.lock().unwrap_or_else(|e| e.into_inner()) = None;
                if cancel() || img.recs.iter().all(|r| r.ray_count == 0) {
                    // the claim is dropped empty: another computer takes the
                    // block (an open-ended one's snapshot stays for the owner)
                    break;
                }
                if let Err(e) = write_atomic(&out, &crate::mc::write_m3c(&scene, &img)) {
                    // the owner gave the job up meanwhile (its folder is gone)
                    if !job.dir.join("job.txt").exists() {
                        break;
                    }
                    return Err(e);
                }
                drop(claim);
                let _ = std::fs::remove_file(out.with_extension("snap"));
                n += 1;
                if !quiet {
                    eprintln!("  {} done ({} passes, {:.1} s)", label(&unit), done_passes, t0.elapsed().as_secs_f64());
                }
            }
        }
        JobKind::Tiles => {
            let text = std::fs::read_to_string(job.in_dir().join("scene.m3s")).map_err(|e| io(&job.in_dir().join("scene.m3s"), e))?;
            let mut scene = Scene::parse(&text)?;
            job_scene(&mut scene);
            if threads > 0 {
                scene.threads = threads;
            }
            let cols: u32 = job.setting::<u32>("cols").unwrap_or(1).max(1);
            let rows: u32 = job.setting::<u32>("rows").unwrap_or(1).max(1);
            let downscale: u32 = job.setting("downscale").unwrap_or(1);
            // the tiles are the final pictures (Big renders), or only their
            // G-buffers are (the owner paints the whole picture from them:
            // the tile's own reflections and depth of field would be
            // calculated for nothing)
            let full_paint = job.setting::<String>("paint").is_none_or(|p| p != "preview");
            let units = (cols as usize).saturating_mul(rows as usize);
            for k in job.tile_indices() {
                if cancel() || no_more() {
                    break;
                }
                if k >= units {
                    continue;
                }
                let out = job.out_dir().join(format!("tile_{k}.m3i"));
                if is_done_file(&out) {
                    continue;
                }
                // a tile that does not fit into this computer's memory is
                // left to the others (the owner notes it and waits for them)
                let t = crate::scene::Tiling { cols, rows, pos: None, downscale }.tile_rect(scene.width, scene.height, k as u32 % cols, k as u32 / cols);
                let b = 2 * crate::render::TILE_BORDER;
                if let Err(e) = crate::render::check_memory((t[2] + b).max(0) as usize, (t[3] + b).max(0) as usize) {
                    let msg = format!("tile {} left to the others: {e}", k + 1);
                    if job.owner_id != heart.id() {
                        return Err(msg);
                    }
                    job.note_error(heart, &msg);
                    continue;
                }
                let Some(claim) = claim_unit(job, heart, &out)? else { continue };
                let unit = format!("tile {}/{units}", k + 1);
                let mut ts = scene.clone();
                ts.tiling = Some(crate::scene::Tiling { cols, rows, pos: Some((k as u32 % cols, k as u32 / cols)), downscale });
                let progress = |d: usize, t: usize| heart.beat(&format!("{} {:.0} %", label(&unit), 100.0 * d as f64 / t.max(1) as f64));
                // (after the march: the post calculations and the painting)
                let stage = |s: &str| heart.beat(&format!("{}, {s}", label(&unit)));
                let t0 = Instant::now();
                let res = match crate::render::render_tile_staged(&ts, full_paint, &progress, &stage, cancel) {
                    Ok(r) => r,
                    // the claim is dropped empty: another computer takes the tile
                    Err(e) if e == "cancelled" => break,
                    Err(e) => return Err(e),
                };
                if cancel() {
                    break;
                }
                // the picture of the tile for the previews, then the buffer
                let _ = std::fs::write(job.out_dir().join(format!("tile_{k}.png")), crate::png::encode_rgb(res.width, res.height, &res.rgb));
                let mut tile_scene = scene.clone();
                tile_scene.width = res.width as i32;
                tile_scene.height = res.height as i32;
                tile_scene.tiling = None;
                let g = res.gbuffer.ok_or("the tile has no image buffer")?;
                if let Err(e) = write_atomic(&out, &crate::m3p::write_m3i(&tile_scene, &g)) {
                    // the owner gave the job up meanwhile (its folder is gone)
                    if !job.dir.join("job.txt").exists() {
                        break;
                    }
                    return Err(e);
                }
                drop(claim);
                n += 1;
                if !quiet {
                    eprintln!("  {} done ({:.1} s)", label(&unit), t0.elapsed().as_secs_f64());
                }
            }
        }
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// the owner's side: creating a job, taking part, collecting

/// How the owner reports while it waits.
pub type Report<'a> = &'a (dyn Fn(&str) + Sync);

/// Takes part in the job until every unit is done (the units nobody has
/// are rendered here, the others waited for), then returns.
fn participate(job: &Job, heart: &Heart, opts: &RunOptions, report: Report) -> Result<(), String> {
    let mut seen = HashSet::new();
    let cancel = || opts.cancel.as_ref().is_some_and(|c| c());
    loop {
        // the owner's own units, with its progress reported once a second
        // (the windows show it, and the parts others finished meanwhile)
        let working = std::sync::atomic::AtomicBool::new(true);
        let worked = std::thread::scope(|sc| {
            sc.spawn(|| {
                while working.load(std::sync::atomic::Ordering::Relaxed) {
                    report(&heart.state());
                    for _ in 0..4 {
                        if !working.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
            });
            let r = work_on(job, heart, opts.threads, opts.quiet, &mut seen, &cancel, &|| false);
            working.store(false, std::sync::atomic::Ordering::Relaxed);
            r
        });
        if worked? > 0 {
            report(&heart.state());
        }
        if cancel() {
            return Err("cancelled".into());
        }
        let (done, all) = job.progress();
        if done >= all {
            return Ok(());
        }
        // every computer that could help has failed on this job: no point
        // in waiting
        let errors = job.errors();
        let swarm = job.dir.parent().and_then(|p| p.parent()).unwrap_or(&job.dir);
        let now = now_secs();
        let all_peers = peers(swarm);
        let helpers: Vec<&Peer> = all_peers.iter().filter(|p| !p.stale(now) && p.id != heart.id() && p.version == env!("CARGO_PKG_VERSION")).collect();
        if !errors.is_empty() && helpers.iter().all(|p| errors.iter().any(|(who, _)| *who == p.name)) {
            let (who, msg) = &errors[0];
            return Err(format!("{who}: {msg}"));
        }
        let others: Vec<String> = helpers.iter().filter(|p| p.state.contains(&job.name)).map(|p| p.name.clone()).collect();
        let other_versions = all_peers.iter().filter(|p| !p.stale(now) && p.id != heart.id() && p.version != env!("CARGO_PKG_VERSION")).count();
        let state = format!(
            "waiting for {} of {all} units of {} ({}{}{})",
            all - done,
            job.name,
            if others.is_empty() { "nobody else working on it".to_string() } else { others.join(", ") },
            if other_versions > 0 { format!("; {other_versions} with another version cannot help") } else { String::new() },
            if errors.is_empty() { String::new() } else { format!("; {} failed: {}", errors[0].0, errors[0].1) }
        );
        report(&state);
        heart.beat_now(&state)?;
        for _ in 0..WAIT_SECS * 4 {
            if cancel() {
                return Err("cancelled".into());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}

/// The owner's part in an open-ended Monte Carlo render: its own block
/// until `opts.cancel`, then [`Job::stop_file`] asks every computer to save
/// its block, and the blocks under way are waited for (at most
/// [`COLLECT_SECS`]).
fn participate_open(job: &Job, heart: &Heart, opts: &RunOptions, report: Report) -> Result<(), String> {
    let cancel = || opts.cancel.as_ref().is_some_and(|c| c());
    let working = std::sync::atomic::AtomicBool::new(true);
    std::thread::scope(|sc| {
        sc.spawn(|| {
            while working.load(std::sync::atomic::Ordering::Relaxed) {
                if cancel() && !job.stopping() {
                    let _ = std::fs::write(job.stop_file(), b"stop\n");
                }
                report(&heart.state());
                for _ in 0..4 {
                    if !working.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        });
        let r = work_on(job, heart, opts.threads, opts.quiet, &mut HashSet::new(), &|| false, &|| false);
        working.store(false, std::sync::atomic::Ordering::Relaxed);
        r
    })?;
    if !job.stopping() {
        let _ = std::fs::write(job.stop_file(), b"stop\n");
    }
    let t0 = Instant::now();
    loop {
        let swarm = job.dir.parent().and_then(|p| p.parent()).unwrap_or(&job.dir);
        let busy = job.unit_files().iter().filter(|f| is_busy_file(swarm, f)).count();
        if busy == 0 || t0.elapsed().as_secs_f64() >= COLLECT_SECS {
            return Ok(());
        }
        let state = format!("waiting for the blocks of {busy} other computer{}", if busy == 1 { "" } else { "s" });
        report(&state);
        heart.beat_now(&state)?;
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn finish(job: &Job, keep: bool) {
    let _ = std::fs::write(job.dir.join("done"), b"done\n");
    if !keep {
        let _ = std::fs::remove_dir_all(&job.dir);
    }
}

/// Settings of the owner's run.
#[derive(Clone, Default)]
pub struct RunOptions {
    /// a name for the heartbeat ("" = the machine's)
    pub name: String,
    pub threads: usize,
    pub quiet: bool,
    /// keep the job folder after the results are collected
    pub keep: bool,
    /// true to stop: the job is given up and removed
    pub cancel: Option<std::sync::Arc<dyn Fn() -> bool + Send + Sync>>,
}

/// After a run: the job folder stays only when asked for (the error of a
/// failed run is reported; its folder would only be abandoned).
fn keep_after(opts: &RunOptions, _result_err: Option<&String>) -> bool {
    opts.keep
}

/// Renders an animation with the swarm: its frames end up where the
/// animation's output folder says. Returns the frame files, in order.
pub fn run_animation(swarm: &Path, anim: &Animation, frames: Option<&[usize]>, opts: &RunOptions, report: Report) -> Result<Vec<PathBuf>, String> {
    let heart = Heart::new(swarm, &opts.name);
    heart.beat_now("creating a job")?;
    let mut a = anim.clone();
    for k in a.keyframes.iter_mut() {
        k.source = None; // the keyframe scenes go into the file
    }
    a.output_folder = "out".into();
    a.overwrite = false;
    a.name = safe_stem(&a.name);
    let mut settings = BTreeMap::new();
    let frames: Vec<usize> = match frames {
        Some(f) => f.iter().copied().filter(|&f| f < a.frame_count()).collect(),
        None => (0..a.frame_count()).collect(),
    };
    if frames.is_empty() {
        return Err("no frames to render".into());
    }
    settings.insert("frames".into(), frames.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(" "));
    let mut k = 0;
    for &f in &frames {
        for (_, eye) in a.stereo_eyes() {
            let name = a.frame_file_eye(f, eye).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            settings.insert(format!("unit_{k}"), name);
            k += 1;
        }
    }
    settings.insert("units".into(), k.to_string());
    let job = new_job(swarm, JobKind::Animation, &anim.name, &heart, settings)?;
    let result = (|| -> Result<Vec<PathBuf>, String> {
        for key in &a.keyframes {
            copy_scene_files(&job, &key.scene)?;
        }
        let text = crate::animfile::write_m3k(&a, None);
        std::fs::write(job.in_dir().join("anim.m3k"), text).map_err(|e| io(&job.in_dir().join("anim.m3k"), e))?;
        participate(&job, &heart, opts, report)?;
        // the frames to where the animation wants them
        let target = crate::animfile::resolve_output(anim, Path::new("."));
        std::fs::create_dir_all(&target).map_err(|e| io(&target, e))?;
        let mut files = Vec::new();
        for &f in &frames {
            for (_, eye) in a.stereo_eyes() {
                let src = job.out_dir().join(a.frame_file_eye(f, eye).file_name().unwrap_or_default());
                let dst = target.join(src.file_name().unwrap_or_default());
                std::fs::copy(&src, &dst).map_err(|e| io(&src, e))?;
                files.push(dst);
            }
        }
        Ok(files)
    })();
    finish(&job, keep_after(opts, result.as_ref().err()));
    heart.leave();
    result
}

/// Renders a batch with the swarm: the pictures end up at `outputs`
/// (as [`crate::batch::output_paths`] gives them).
pub fn run_batch(swarm: &Path, items: &[BatchItem], outputs: &[PathBuf], bopts: &BatchOptions, opts: &RunOptions, report: Report) -> Result<(), String> {
    let heart = Heart::new(swarm, &opts.name);
    heart.beat_now("creating a job")?;
    let name = items.first().and_then(|i| i.input.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "batch".into());
    let mut settings = BTreeMap::new();
    settings.insert("format".into(), bopts.format.extension().to_string());
    if let Some(s) = bopts.scale {
        settings.insert("scale".into(), s.to_string());
    }
    settings.insert("aa".into(), bopts.aa.to_string());
    settings.insert("depth".into(), bopts.depth.to_string());
    settings.insert("units".into(), items.len().to_string());
    let job = new_job(swarm, JobKind::Batch, &name, &heart, settings)?;
    let result = (|| -> Result<(), String> {
        // the parameter files (and their formulas, maps) into in/, the list
        // with the overrides, the output names as the batch names them
        let mut list = String::new();
        let mut job_items = Vec::new();
        for (n, it) in items.iter().enumerate() {
            let (scene, _, _) = crate::batch::item_scene(it, bopts)?;
            copy_scene_files(&job, &scene)?;
            let ext = it.input.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "m3p".into());
            let fname = format!("{n:04}_{}.{ext}", sanitised(&it.input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()));
            std::fs::copy(&it.input, job.in_dir().join(&fname)).map_err(|e| io(&it.input, e))?;
            let ov: Vec<String> = bopts.overrides.iter().chain(&it.overrides).cloned().collect();
            list.push_str(&fname);
            for o in &ov {
                list.push_str(" | ");
                list.push_str(o);
            }
            list.push('\n');
            job_items.push(BatchItem { input: job.in_dir().join(&fname), overrides: Vec::new() });
        }
        std::fs::write(job.in_dir().join("list.txt"), list).map_err(|e| io(&job.in_dir().join("list.txt"), e))?;
        let job_outs = crate::batch::output_paths(&job_items, &BatchOptions { output_dir: Some(job.out_dir()), format: bopts.format, ..Default::default() });
        let mut job2 = job.clone();
        for (k, o) in job_outs.iter().enumerate() {
            job2.settings.insert(format!("unit_{k}"), o.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        }
        write_job(&job2)?;
        participate(&job2, &heart, opts, report)?;
        for (src, dst) in job_outs.iter().zip(outputs) {
            if let Some(d) = dst.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::fs::create_dir_all(d).map_err(|e| io(d, e))?;
            }
            std::fs::copy(src, dst).map_err(|e| io(src, e))?;
        }
        Ok(())
    })();
    finish(&job, keep_after(opts, result.as_ref().err()));
    heart.leave();
    result
}

/// Monte Carlo with the swarm: `units` blocks, each rendered until it has
/// `rays` rays per pixel (or `passes` passes, or `time` seconds), merged.
/// With none of the three (all 0) the render is open-ended, as the Monte
/// Carlo window's: every computer renders a block until `opts.cancel`,
/// then the blocks are saved and merged (`units` is not used).
pub fn run_montecarlo(swarm: &Path, scene: &Scene, units: usize, rays: f64, passes: u32, time: f64, opts: &RunOptions, report: Report) -> Result<McImage, String> {
    run_montecarlo_live(swarm, scene, units, rays, passes, time, opts, report, None)
}

/// [`run_montecarlo`] that hands the blocks merged so far to `live` while
/// the owner waits (the Monte Carlo window shows the picture cleaning up).
#[allow(clippy::too_many_arguments)]
pub fn run_montecarlo_live(swarm: &Path, scene: &Scene, units: usize, rays: f64, passes: u32, time: f64, opts: &RunOptions, report: Report, live: Option<&(dyn Fn(McImage) + Sync)>) -> Result<McImage, String> {
    let heart = Heart::new(swarm, &opts.name);
    heart.beat_now("creating a job")?;
    let open = rays <= 0.0 && passes == 0 && time <= 0.0;
    let units = if open { OPEN_UNITS } else { units.max(1) };
    let mut settings = BTreeMap::new();
    settings.insert("units".into(), units.to_string());
    settings.insert("open".into(), open.to_string());
    settings.insert("rays".into(), rays.to_string());
    settings.insert("passes".into(), passes.to_string());
    settings.insert("time".into(), time.to_string());
    let job = new_job(swarm, JobKind::MonteCarlo, "monte carlo", &heart, settings)?;
    let result = (|| -> Result<McImage, String> {
        copy_scene_files(&job, scene)?;
        std::fs::write(job.in_dir().join("scene.m3s"), scene.to_text()).map_err(|e| io(&job.in_dir().join("scene.m3s"), e))?;
        // the finished blocks, and the one this computer is at (after each
        // of its passes): the picture cleans up from the start
        heart.want_block.store(live.is_some(), std::sync::atomic::Ordering::Relaxed);
        let cache = Mutex::new((BlockMerge::default(), 0u64));
        let report2 = |st: &str| {
            report(st);
            if let Some(l) = live {
                let mut c = cache.lock().unwrap_or_else(|e| e.into_inner());
                let before = c.0.version();
                let done = c.0.update(&job).cloned();
                let own = heart.block.lock().unwrap_or_else(|e| e.into_inner()).clone();
                let own_new = own.as_ref().is_some_and(|o| o.0 != c.1);
                if c.0.version() == before && !own_new {
                    return;
                }
                c.1 = own.as_ref().map_or(0, |o| o.0);
                let m = match (done, own) {
                    (Some(mut d), Some((_, o))) => {
                        let _ = d.merge(&o);
                        Some(d)
                    }
                    (Some(d), None) => Some(d),
                    (None, Some((_, o))) => Some(o),
                    (None, None) => None,
                };
                if let Some(m) = m {
                    l(m);
                }
            }
        };
        if open {
            participate_open(&job, &heart, opts, &report2)?;
        } else {
            participate(&job, &heart, opts, &report2)?;
        }
        // the blocks; of an open-ended render the saved ones, and the last
        // snapshot of a block that was not saved (its computer went away)
        let mut merged: Option<McImage> = None;
        for f in job.unit_files() {
            let f = if !open || is_done_file(&f) {
                f
            } else if f.with_extension("snap").exists() {
                f.with_extension("snap")
            } else {
                continue;
            };
            let data = std::fs::read(&f).map_err(|e| io(&f, e))?;
            let (_, img) = crate::mc::read_m3c(&data).map_err(|e| format!("{}: {e}", f.display()))?;
            match &mut merged {
                None => merged = Some(img),
                Some(m) => m.merge(&img)?,
            }
        }
        merged.ok_or_else(|| "no blocks rendered".to_string())
    })();
    finish(&job, keep_after(opts, result.as_ref().err()));
    heart.leave();
    result
}

/// One picture in `cols` x `rows` tiles with the swarm: returns the
/// G-buffer of the whole picture (the tiles' buffers put together, each
/// with its shadows and occlusion calculated with the tile's border). `preview`, if given, gets the
/// picture so far (tiles done, in progress, missing) while the owner waits.
pub fn run_tiles(swarm: &Path, scene: &Scene, cols: u32, rows: u32, opts: &RunOptions, report: Report, preview: Option<&(dyn Fn(Preview) + Sync)>) -> Result<Vec<crate::gbuffer::SiLight>, String> {
    let (cols, rows) = (cols.max(1), rows.max(1));
    let (w, h) = (scene.width as usize, scene.height as usize);
    let tl = crate::scene::Tiling { cols, rows, pos: None, downscale: 1 };
    let full = Mutex::new(vec![crate::gbuffer::SiLight::default(); w * h]);
    let collect = |k: usize, m3i: &Path, _png: &Path| -> Result<(), String> {
        let data = std::fs::read(m3i).map_err(|e| io(m3i, e))?;
        let (tw, th, g) = crate::m3p::gbuffer_from_m3i(&data).ok_or_else(|| format!("{}: not a tile", m3i.display()))?;
        let t = tl.tile_rect(scene.width, scene.height, k as u32 % cols, k as u32 / cols);
        if tw != t[2] as usize || th != t[3] as usize {
            return Err(format!("{}: tile of {tw}x{th}, expected {}x{}", m3i.display(), t[2], t[3]));
        }
        let mut full = full.lock().unwrap_or_else(|e| e.into_inner());
        for y in 0..th {
            let d = (t[1] as usize + y) * w + t[0] as usize;
            full[d..d + tw].copy_from_slice(&g[y * tw..(y + 1) * tw]);
        }
        Ok(())
    };
    run_tiles_job(swarm, scene, cols, rows, None, false, opts, report, preview, &collect)?;
    // (each tile's buffer has its shadows and ambient occlusion already,
    // calculated with the tile's border as MB3D's big-render tiles are)
    Ok(full.into_inner().unwrap_or_else(|e| e.into_inner()))
}

/// The tiles job behind [`run_tiles`]: `subset`, if given, is the tiles
/// to render (index k = row * cols + col); as each is finished, `collect`
/// gets its `.m3i` (its G-buffer) and `.png` (its picture), while they are
/// still in the job folder (also when the render is stopped: the tiles
/// done so far). For pictures too big to hold whole (the Big renders
/// window).
#[allow(clippy::too_many_arguments)]
pub fn run_tiles_files(
    swarm: &Path,
    scene: &Scene,
    cols: u32,
    rows: u32,
    subset: Option<&[usize]>,
    opts: &RunOptions,
    report: Report,
    preview: Option<&(dyn Fn(Preview) + Sync)>,
    collect: &(dyn Fn(usize, &Path, &Path) -> Result<(), String> + Sync),
) -> Result<(), String> {
    run_tiles_job(swarm, scene, cols, rows, subset, true, opts, report, preview, collect)
}

/// [`run_tiles_files`]; `full_paint`: the tiles' pictures are painted in
/// full (they are the result), else only as previews (their G-buffers
/// are).
#[allow(clippy::too_many_arguments)]
fn run_tiles_job(
    swarm: &Path,
    scene: &Scene,
    cols: u32,
    rows: u32,
    subset: Option<&[usize]>,
    full_paint: bool,
    opts: &RunOptions,
    report: Report,
    preview: Option<&(dyn Fn(Preview) + Sync)>,
    collect: &(dyn Fn(usize, &Path, &Path) -> Result<(), String> + Sync),
) -> Result<(), String> {
    let heart = Heart::new(swarm, &opts.name);
    heart.beat_now("creating a job")?;
    let (cols, rows) = (cols.max(1), rows.max(1));
    let mut scene = scene.clone();
    let downscale = scene.tiling.map(|t| t.downscale.max(1)).unwrap_or(1);
    scene.tiling = None;
    scene.calc_rect = None;
    let all = (cols * rows) as usize;
    let tiles: Vec<usize> = match subset {
        Some(s) => s.iter().copied().filter(|&k| k < all).collect(),
        None => (0..all).collect(),
    };
    if tiles.is_empty() {
        return Err("no tiles to render".into());
    }
    let mut settings = BTreeMap::new();
    settings.insert("cols".into(), cols.to_string());
    settings.insert("rows".into(), rows.to_string());
    settings.insert("downscale".into(), downscale.to_string());
    settings.insert("units".into(), all.to_string());
    settings.insert("width".into(), scene.width.to_string());
    settings.insert("height".into(), scene.height.to_string());
    if subset.is_some() {
        settings.insert("tiles".into(), tiles.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(" "));
    }
    if !full_paint {
        settings.insert("paint".into(), "preview".into());
    }
    let job = new_job(swarm, JobKind::Tiles, "picture", &heart, settings)?;
    let result = (|| -> Result<(), String> {
        copy_scene_files(&job, &scene)?;
        std::fs::write(job.in_dir().join("scene.m3s"), scene.to_text()).map_err(|e| io(&job.in_dir().join("scene.m3s"), e))?;
        // each tile goes to `collect` as soon as it is finished, so a
        // stopped render keeps the tiles done so far
        let collected = Mutex::new(vec![false; tiles.len()]);
        let failed: Mutex<Option<String>> = Mutex::new(None);
        let collect_ready = || {
            let mut got = collected.lock().unwrap_or_else(|e| e.into_inner());
            for ((k, f), got) in tiles.iter().zip(job.unit_files()).zip(got.iter_mut()) {
                if *got || !is_done_file(&f) {
                    continue;
                }
                *got = true;
                if let Err(e) = collect(*k, &f, &f.with_extension("png")) {
                    failed.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert(e);
                }
            }
        };
        let report2 = |st: &str| {
            collect_ready();
            report(st);
            if let Some(pv) = preview {
                if let Some(p) = job_preview(&job, 0) {
                    pv(p);
                }
            }
        };
        let r = participate(&job, &heart, opts, &report2);
        collect_ready();
        if let Some(e) = failed.into_inner().unwrap_or_else(|e| e.into_inner()) {
            return Err(e);
        }
        r?;
        let got = collected.into_inner().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = got.iter().position(|g| !g) {
            return Err(format!("tile {} is missing", tiles[i] + 1));
        }
        Ok(())
    })();
    finish(&job, keep_after(opts, result.as_ref().err()));
    heart.leave();
    result
}

/// The finished blocks of a Monte Carlo job merged so far, with the last
/// snapshots of the open-ended blocks under way: the picture cleans up as
/// blocks arrive. Kept between refreshes, so a block is read once.
#[derive(Default)]
pub struct BlockMerge {
    job: String,
    /// the finished blocks merged
    merged: Option<McImage>,
    files: HashSet<PathBuf>,
    /// the snapshots of the blocks under way, by unit file, with their time
    snaps: BTreeMap<PathBuf, (SystemTime, McImage)>,
    /// the finished blocks and the snapshots together
    all: Option<McImage>,
    scene: Option<crate::scene::Scene>,
    version: u64,
}

impl BlockMerge {
    /// Reads what is new in `job`; the merged image, if there is any block.
    pub fn update(&mut self, job: &Job) -> Option<&McImage> {
        if self.job != job.id {
            *self = BlockMerge { job: job.id.clone(), ..Default::default() };
        }
        let mut changed = false;
        for f in job.unit_files() {
            if self.files.contains(&f) {
                continue;
            }
            if is_done_file(&f) {
                let Ok(data) = std::fs::read(&f) else { continue };
                let Ok((m3p, img)) = crate::mc::read_m3c(&data) else { continue };
                self.snaps.remove(&f);
                self.files.insert(f);
                self.scene.get_or_insert(m3p.scene);
                match &mut self.merged {
                    None => self.merged = Some(img),
                    Some(m) => {
                        let _ = m.merge(&img);
                    }
                }
                changed = true;
            } else if job.open() {
                let sp = f.with_extension("snap");
                let Ok(t) = std::fs::metadata(&sp).and_then(|m| m.modified()) else { continue };
                if self.snaps.get(&f).is_some_and(|(seen, _)| *seen == t) {
                    continue;
                }
                let Ok(data) = std::fs::read(&sp) else { continue };
                let Ok((m3p, img)) = crate::mc::read_m3c(&data) else { continue };
                self.scene.get_or_insert(m3p.scene);
                self.snaps.insert(f, (t, img));
                changed = true;
            }
        }
        if changed {
            self.version += 1;
            let mut all = self.merged.clone();
            for (_, img) in self.snaps.values() {
                match &mut all {
                    None => all = Some(img.clone()),
                    Some(m) => {
                        let _ = m.merge(img);
                    }
                }
            }
            self.all = all;
        }
        self.all.as_ref()
    }
    /// The merged image painted (the job's own Monte Carlo settings).
    pub fn painted(&self) -> Option<(Vec<u8>, usize, usize)> {
        let (m, sc) = (self.all.as_ref()?, self.scene.as_ref()?);
        Some((crate::mc::paint(m, &sc.mc, sc.lighting.gamma), m.width, m.height))
    }
    /// Changes whenever [`BlockMerge::update`] read something new.
    pub fn version(&self) -> u64 {
        self.version
    }
}

/// The picture of a job so far, for the panels: what is done, in progress
/// and missing.
#[derive(Clone, Debug, Default)]
pub struct Preview {
    pub width: usize,
    pub height: usize,
    /// the picture: done parts as rendered, the rest dark
    pub rgb: Vec<u8>,
    /// rectangles (left, top, width, height) of the units under way, with
    /// their progress 0..1 where a heartbeat tells it
    pub working: Vec<([usize; 4], f32)>,
    pub done: usize,
    pub total: usize,
}

/// The picture of the job so far, at most `max_w` pixels wide (0 = full
/// size): tiles put together from the tiles' pictures, a Monte Carlo
/// picture from the blocks done so far (the newest only, the merge of all
/// is the owner's), an animation's or batch's last picture.
pub fn job_preview(job: &Job, max_w: usize) -> Option<Preview> {
    job_preview_merged(job, max_w, None)
}

/// [`job_preview`] with a [`BlockMerge`] for Monte Carlo jobs: the blocks
/// done so far merged, instead of the newest one alone.
pub fn job_preview_merged(job: &Job, max_w: usize, merge: Option<&mut BlockMerge>) -> Option<Preview> {
    let files = job.unit_files();
    let total = files.len();
    let done = files.iter().filter(|f| is_done_file(f)).count();
    let in_progress = |f: &Path| !is_done_file(f) && marker_of(f).exists();
    // the progress of the units under way, from the heartbeats
    let swarm = job.dir.parent().and_then(|p| p.parent()).unwrap_or(&job.dir);
    let now = now_secs();
    let progress_of = |unit: &str| -> f32 {
        peers(swarm)
            .iter()
            .filter(|p| !p.stale(now) && p.state.contains(&job.name) && p.state.contains(unit))
            .find_map(|p| p.state.rsplit_once(' ').and_then(|(a, _)| a.rsplit_once(' ')).and_then(|(_, pct)| pct.parse::<f32>().ok()))
            .map(|v| v / 100.0)
            .unwrap_or(0.0)
    };
    let load_rgb = |p: &Path| -> Option<(Vec<u8>, usize, usize)> {
        let img = crate::image::load(p).ok()?;
        let s = if img.deep { 8 } else { 0 };
        let rgb = img.data.iter().flat_map(|px| [(px[0] >> s) as u8, (px[1] >> s) as u8, (px[2] >> s) as u8]).collect();
        Some((rgb, img.width, img.height))
    };
    match job.kind {
        JobKind::Tiles => {
            let (w, h): (usize, usize) = (job.setting("width")?, job.setting("height")?);
            let (cols, rows): (u32, u32) = (job.setting("cols")?, job.setting("rows")?);
            let f = if max_w > 0 && w > max_w { w as f64 / max_w as f64 } else { 1.0 };
            let (pw, ph) = ((w as f64 / f).round().max(1.0) as usize, (h as f64 / f).round().max(1.0) as usize);
            let mut rgb = vec![40u8; pw * ph * 3];
            let mut working = Vec::new();
            let tl = crate::scene::Tiling { cols, rows, pos: None, downscale: 1 };
            for (k, unit) in job.tile_indices().into_iter().zip(&files) {
                let t = tl.tile_rect(w as i32, h as i32, k as u32 % cols, k as u32 / cols);
                let r = [(t[0] as f64 / f) as usize, (t[1] as f64 / f) as usize, (t[2] as f64 / f).ceil() as usize, (t[3] as f64 / f).ceil() as usize];
                if is_done_file(unit) {
                    if let Some((trgb, tw, th)) = load_rgb(&unit.with_extension("png")) {
                        blit_scaled(&mut rgb, pw, ph, r, &trgb, tw, th);
                    }
                } else if in_progress(unit) {
                    for y in r[1]..(r[1] + r[3]).min(ph) {
                        for x in r[0]..(r[0] + r[2]).min(pw) {
                            let i = (y * pw + x) * 3;
                            rgb[i..i + 3].copy_from_slice(&[70, 70, 90]);
                        }
                    }
                    working.push((r, progress_of(&format!("tile {}/{}", k + 1, (cols * rows) as usize))));
                }
            }
            // a gentle grid between the tiles
            let soft = |v: &mut u8| *v = (*v as u16 / 2 + 88) as u8;
            for k in 0..files.len() {
                let (c, r) = (k as u32 % cols, k as u32 / cols);
                let t = tl.tile_rect(w as i32, h as i32, c, r);
                let x1 = ((t[0] + t[2]) as f64 / f) as usize;
                let y1 = ((t[1] + t[3]) as f64 / f) as usize;
                let (x0, y0) = ((t[0] as f64 / f) as usize, (t[1] as f64 / f) as usize);
                if c + 1 < cols && x1 > 0 && x1 <= pw {
                    for y in y0..y1.min(ph) {
                        let i = (y * pw + x1 - 1) * 3;
                        rgb[i..i + 3].iter_mut().for_each(soft);
                    }
                }
                if r + 1 < rows && y1 > 0 && y1 <= ph {
                    for x in x0..x1.min(pw) {
                        let i = ((y1 - 1) * pw + x) * 3;
                        rgb[i..i + 3].iter_mut().for_each(soft);
                    }
                }
            }
            Some(Preview { width: pw, height: ph, rgb, working, done, total })
        }
        JobKind::MonteCarlo => {
            let (rgb, w, h) = match merge {
                Some(m) => {
                    m.update(job)?;
                    m.painted()?
                }
                None => {
                    let newest = files.iter().filter(|f| is_done_file(f)).max_by_key(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())?;
                    let data = std::fs::read(newest).ok()?;
                    let (f, img) = crate::mc::read_m3c(&data).ok()?;
                    (crate::mc::paint(&img, &f.scene.mc, f.scene.lighting.gamma), img.width, img.height)
                }
            };
            let (rgb, w, h) = fit_rgb(&rgb, w, h, max_w);
            Some(Preview { width: w, height: h, rgb, working: Vec::new(), done, total })
        }
        JobKind::Animation | JobKind::Batch => {
            let newest = files.iter().filter(|f| is_done_file(f)).max_by_key(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok())?;
            let (rgb, w, h) = load_rgb(newest)?;
            let (rgb, w, h) = fit_rgb(&rgb, w, h, max_w);
            Some(Preview { width: w, height: h, rgb, working: Vec::new(), done, total })
        }
    }
}

/// `src` (sw x sh) scaled to rectangle `r` of `dst` (nearest pixel).
fn blit_scaled(dst: &mut [u8], dw: usize, dh: usize, r: [usize; 4], src: &[u8], sw: usize, sh: usize) {
    if sw == 0 || sh == 0 || r[2] == 0 || r[3] == 0 {
        return;
    }
    for y in 0..r[3] {
        let dy = r[1] + y;
        if dy >= dh {
            break;
        }
        let sy = (y * sh / r[3]).min(sh - 1);
        for x in 0..r[2] {
            let dx = r[0] + x;
            if dx >= dw {
                break;
            }
            let sx = (x * sw / r[2]).min(sw - 1);
            let (d, s) = ((dy * dw + dx) * 3, (sy * sw + sx) * 3);
            dst[d..d + 3].copy_from_slice(&src[s..s + 3]);
        }
    }
}

/// `rgb` reduced to at most `max_w` pixels wide (0: as is).
pub fn fit_rgb(rgb: &[u8], w: usize, h: usize, max_w: usize) -> (Vec<u8>, usize, usize) {
    if max_w == 0 || w <= max_w || w == 0 {
        return (rgb.to_vec(), w, h);
    }
    let f = w as f64 / max_w as f64;
    let (pw, ph) = (max_w, (h as f64 / f).round().max(1.0) as usize);
    let mut out = vec![0u8; pw * ph * 3];
    blit_scaled(&mut out, pw, ph, [0, 0, pw, ph], rgb, w, h);
    (out, pw, ph)
}

/// The number of blocks a Monte Carlo render is split into when the user
/// gives none: two per instance seen in the swarm (the owner included).
pub fn suggested_units(swarm: &Path) -> usize {
    let now = now_secs();
    (peers(swarm).iter().filter(|p| !p.stale(now)).count() + 1) * 2
}

// ---------------------------------------------------------------------------
// the worker

/// Renders what the jobs in the swarm folder have for this machine, for
/// ever (or once through, with `once`). `name`: "" = the machine's name.
pub fn worker(swarm: &Path, name: &str, threads: usize, once: bool, quiet: bool) -> Result<(), String> {
    worker_until(swarm, name, threads, once, quiet, &std::sync::atomic::AtomicBool::new(false))
}

/// How a running worker is stopped: `stop` takes no further unit and
/// returns when the current one is done; `abort` gives the current one up
/// as well (it is taken by another computer).
#[derive(Default)]
pub struct WorkerControl {
    pub stop: std::sync::atomic::AtomicBool,
    pub abort: std::sync::atomic::AtomicBool,
    /// what the worker last had to report (an error in a job, a job removed)
    pub note: Mutex<String>,
}

impl WorkerControl {
    fn stopping(&self) -> bool {
        self.stop.load(std::sync::atomic::Ordering::Relaxed) || self.abort.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn note(&self) -> String {
        self.note.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    fn set_note(&self, s: String) {
        *self.note.lock().unwrap_or_else(|e| e.into_inner()) = s;
    }
}

/// [`worker`] that returns when `stop` is set (the Swarm window's thread);
/// a unit under way is given up and taken by another computer.
pub fn worker_until(swarm: &Path, name: &str, threads: usize, once: bool, quiet: bool, stop: &std::sync::atomic::AtomicBool) -> Result<(), String> {
    let control = WorkerControl::default();
    control.abort.store(stop.load(std::sync::atomic::Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
    worker_controlled(swarm, name, threads, once, quiet, &control)
}

/// [`worker`] with a [`WorkerControl`]. A job from another version of the
/// program is skipped, a job whose owner is gone for [`ORPHAN_SECS`] is
/// removed, and a job in which a unit fails is noted (`errors/`) and left
/// to the others.
pub fn worker_controlled(swarm: &Path, name: &str, threads: usize, once: bool, quiet: bool, control: &WorkerControl) -> Result<(), String> {
    std::fs::create_dir_all(swarm.join("jobs")).map_err(|e| io(&swarm.join("jobs"), e))?;
    let cancel = || control.abort.load(std::sync::atomic::Ordering::Relaxed);
    let mut failed: HashSet<String> = HashSet::new();
    let mut noted: HashSet<String> = HashSet::new();
    let heart = Heart::new(swarm, name);
    heart.beat_now("idle")?;
    if !quiet {
        eprintln!("swarm worker {} in {}: {} cores{}", heart.name(), swarm.display(), std::thread::available_parallelism().map_or(1, |n| n.get()), if heart.presence.peer.card.is_empty() { String::new() } else { format!(", {}", heart.presence.peer.card) });
    }
    let mut seen = HashSet::new();
    let mut sweep = Instant::now();
    loop {
        let mut worked = 0;
        let now = now_secs();
        for job in jobs(swarm) {
            if control.stopping() {
                break;
            }
            // the jobs this program made are rendered by the part that made
            // them, with all cores already
            if job.done() || failed.contains(&job.id) || job.owner_id == heart.id() {
                continue;
            }
            if job.orphaned(swarm, now) {
                let _ = std::fs::remove_dir_all(&job.dir);
                let msg = format!("removed the abandoned job {} of {} (no sign of its owner for {} minutes)", job.name, job.owner, ORPHAN_SECS / 60);
                if !quiet {
                    eprintln!("  {msg}");
                }
                control.set_note(msg);
                continue;
            }
            if !job.same_version() {
                if noted.insert(job.id.clone()) {
                    let msg = format!("job {} of {} needs version {} (this is {})", job.name, job.owner, job.version, env!("CARGO_PKG_VERSION"));
                    if !quiet {
                        eprintln!("  {msg}");
                    }
                    control.set_note(msg);
                }
                continue;
            }
            match work_on(&job, &heart, threads, quiet, &mut seen, &cancel, &|| control.stopping()) {
                Ok(n) => worked += n,
                Err(e) => {
                    let msg = format!("job {} of {}: {e}", job.name, job.owner);
                    if !quiet {
                        eprintln!("  {msg}");
                    }
                    job.note_error(&heart, &e);
                    control.set_note(msg);
                    failed.insert(job.id.clone());
                }
            }
        }
        if once || control.stopping() {
            heart.leave();
            return Ok(());
        }
        if worked == 0 {
            heart.beat_now("idle")?;
            for _ in 0..8 {
                if control.stopping() {
                    heart.leave();
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        }
        if sweep.elapsed().as_secs() > STALE_SECS {
            sweep_peers(swarm);
            sweep = Instant::now();
        }
    }
}

/// The panel as text: who is in the swarm and what the jobs are at.
pub fn status_text(swarm: &Path) -> String {
    let now = now_secs();
    let mut t = String::new();
    let ps = peers(swarm);
    if ps.is_empty() {
        t.push_str("nobody in the swarm\n");
    }
    for p in &ps {
        let age = now.saturating_sub(p.time);
        t.push_str(&format!(
            "{:<20} {:<8} {:>3} cores  {:<28} {}{}\n",
            p.name,
            p.version,
            p.cores,
            if p.card.is_empty() { "processor".to_string() } else { p.card.chars().take(28).collect() },
            p.state,
            if p.stale(now) { format!("  (gone, last seen {age} s ago)") } else { String::new() }
        ));
    }
    let js: Vec<Job> = jobs(swarm).into_iter().filter(|j| !j.done()).collect();
    if js.is_empty() {
        t.push_str("no jobs\n");
    }
    for j in &js {
        let (d, n) = j.progress();
        let errs = j.errors();
        t.push_str(&format!(
            "job {:<24} {:<10} by {:<16} {d}/{n} units done{}{}\n",
            j.name.chars().take(24).collect::<String>(),
            j.kind.as_str(),
            j.owner,
            if j.same_version() { String::new() } else { format!("  (version {})", j.version) },
            if errs.is_empty() { String::new() } else { format!("  error at {}: {}", errs[0].0, errs[0].1) }
        ));
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mb3d_swarm_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn tiny_scene() -> Scene {
        let mut s = Scene::preset("Integer Power").unwrap();
        s.width = 24;
        s.height = 18;
        s
    }

    #[test]
    fn heartbeats_and_staleness() {
        let d = temp("peers");
        let h = Heart::new(&d, "alpha");
        h.beat_now("idle").unwrap();
        let ps = peers(&d);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].name, "alpha");
        assert_eq!(ps[0].state, "idle");
        assert!(!ps[0].stale(now_secs()));
        assert!(ps[0].stale(now_secs() + STALE_SECS + 1));
        h.leave();
        assert!(peers(&d).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_parts_of_one_program_are_listed_once() {
        let d = temp("once");
        let worker = Heart::new(&d, "alpha");
        let owner = Heart::new(&d, "alpha");
        assert_eq!(worker.id(), owner.id());
        worker.beat_now("idle").unwrap();
        owner.beat_now("rendering picture (tiles): tile 1/4").unwrap();
        let ps = peers(&d);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].state, "rendering picture (tiles): tile 1/4");
        worker.beat_now("rendering picture (tiles): tile 2/4").unwrap();
        // (in the order the parts came: the worker first)
        assert_eq!(peers(&d)[0].state, "rendering picture (tiles): tile 2/4; rendering picture (tiles): tile 1/4");
        // another computer (here: another name) has its own heartbeat
        let other = Heart::new(&d, "beta");
        other.beat_now("idle").unwrap();
        assert_eq!(peers(&d).len(), 2);
        drop(other);
        // the heartbeat stays while a part is left, and goes with the last
        owner.leave();
        let ps = peers(&d);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].state, "rendering picture (tiles): tile 2/4");
        drop(worker);
        assert!(peers(&d).is_empty());
        // a new part later is a new heartbeat
        let again = Heart::new(&d, "alpha");
        again.beat_now("idle").unwrap();
        assert_eq!(peers(&d).len(), 1);
        drop(again);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_tile_too_big_for_the_memory_is_left_to_the_others() {
        // (the free memory is known on Linux only)
        if crate::render::memory_available().is_none() {
            return;
        }
        assert!(crate::render::check_memory(1000, 1000).is_ok());
        assert!(crate::render::check_memory(10_000_000, 10_000_000).unwrap_err().contains("needs about"));
        let d = temp("toobig");
        let swarm = d.join("swarm");
        let owner = Heart::new(&swarm, "owner");
        let mut s = tiny_scene();
        s.width = 10_000_000;
        s.height = 10_000_000;
        let mut settings = BTreeMap::new();
        for (k, v) in [("cols", "2"), ("rows", "1"), ("units", "2"), ("downscale", "1")] {
            settings.insert(k.into(), v.into());
        }
        let job = new_job(&swarm, JobKind::Tiles, "huge", &owner, settings).unwrap();
        std::fs::write(job.in_dir().join("scene.m3s"), s.to_text()).unwrap();
        // another computer: an error for the job, nothing claimed
        let other = Heart::new(&swarm, "other");
        let e = work_on(&job, &other, 1, true, &mut HashSet::new(), &|| false, &|| false).unwrap_err();
        assert!(e.contains("left to the others"), "{e}");
        // the owner: noted, nothing claimed, and it goes on waiting
        assert_eq!(work_on(&job, &owner, 1, true, &mut HashSet::new(), &|| false, &|| false).unwrap(), 0);
        assert!(job.errors().iter().any(|(who, msg)| who == "owner" && msg.contains("left to the others")));
        assert!(job.unit_files().iter().all(|f| !f.exists()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_worker_leaves_its_own_programs_jobs_and_stops_when_asked() {
        let d = temp("ownjobs");
        let swarm = d.join("swarm");
        let owner = Heart::new(&swarm, "alpha");
        owner.beat_now("creating a job").unwrap();
        let mut settings = BTreeMap::new();
        for (k, v) in [("cols", "2"), ("rows", "2"), ("units", "4"), ("downscale", "1")] {
            settings.insert(k.into(), v.into());
        }
        let job = new_job(&swarm, JobKind::Tiles, "picture", &owner, settings).unwrap();
        std::fs::write(job.in_dir().join("scene.m3s"), tiny_scene().to_text()).unwrap();
        // the worker of the same program: nothing taken
        let control = WorkerControl::default();
        worker_controlled(&swarm, "alpha", 1, true, true, &control).unwrap();
        assert_eq!(job.progress().0, 0);
        // another computer that is told to take no more: one tile only
        let other = Heart::new(&swarm, "beta");
        let taken = std::sync::atomic::AtomicUsize::new(0);
        let no_more = || taken.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 1;
        assert_eq!(work_on(&job, &other, 1, true, &mut HashSet::new(), &|| false, &no_more).unwrap(), 1);
        drop(other);
        drop(owner);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn names_from_a_job_stay_in_its_folder() {
        assert_eq!(bare_name("c:\\maps\\sky.png"), Some("sky.png"));
        assert_eq!(bare_name("/home/x/sky.png"), Some("sky.png"));
        assert_eq!(bare_name("sky.png"), Some("sky.png"));
        assert_eq!(bare_name("a/.."), None);
        assert_eq!(bare_name(""), None);
        assert_eq!(safe_stem("../../x"), ".._.._x");
        // an animation job whose name points elsewhere renders into out/
        let d = temp("names");
        let swarm = d.join("swarm");
        let owner = Heart::new(&swarm, "owner");
        let mut s = tiny_scene();
        s.lighting.bg_image = d.join("secret.png").to_string_lossy().into_owned();
        let mut sc = s.clone();
        job_scene(&mut sc);
        assert_eq!(sc.lighting.bg_image, "secret.png");
        // (out/ is swarm/jobs/<id>/out: four levels up is outside)
        let mut a = Animation { name: "../../../../escaped".into(), ..Default::default() };
        a.keyframes.push(crate::anim::Keyframe::new(s.clone(), 1));
        a.keyframes.push(crate::anim::Keyframe::new(s.clone(), 1));
        a.set_size_from(&s);
        let mut settings = BTreeMap::new();
        settings.insert("units".into(), "1".into());
        settings.insert("unit_0".into(), "../../../../escaped000001.png".into());
        let job = new_job(&swarm, JobKind::Animation, "evil", &owner, settings).unwrap();
        std::fs::write(job.in_dir().join("anim.m3k"), crate::animfile::write_m3k(&a, None)).unwrap();
        assert!(job.unit_files()[0].starts_with(job.out_dir()));
        let worker = Heart::new(&swarm, "worker");
        let n = work_on(&job, &worker, 1, true, &mut HashSet::new(), &|| false, &|| false).unwrap();
        assert!(n > 0);
        let outside: Vec<_> = std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name()).filter(|n| n != "swarm").collect();
        assert!(outside.is_empty(), "{outside:?}");
        assert!(std::fs::read_dir(job.out_dir()).unwrap().count() > 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_worker_leaves_a_job_its_owner_gave_up() {
        let d = temp("givenup");
        let swarm = d.join("swarm");
        let owner = Heart::new(&swarm, "owner");
        let mut s = tiny_scene();
        s.width = 640;
        s.height = 480;
        let mut settings = BTreeMap::new();
        for (k, v) in [("cols", "4"), ("rows", "4"), ("units", "16"), ("downscale", "1")] {
            settings.insert(k.into(), v.into());
        }
        let job = new_job(&swarm, JobKind::Tiles, "picture", &owner, settings).unwrap();
        std::fs::write(job.in_dir().join("scene.m3s"), s.to_text()).unwrap();
        let worker = Heart::new(&swarm, "worker");
        // the owner removes the job while the worker is at its first tile
        let n = std::thread::scope(|sc| {
            let w = sc.spawn(|| work_on(&job, &worker, 1, true, &mut HashSet::new(), &|| false, &|| false));
            std::thread::sleep(Duration::from_millis(300));
            finish(&job, false);
            w.join().unwrap().unwrap()
        });
        assert!(n < 16, "rendered all {n} tiles of a removed job");
        assert!(!job.dir.join("job.txt").exists());
        assert_eq!(work_on(&job, &worker, 1, true, &mut HashSet::new(), &|| false, &|| false).unwrap(), 0);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_unit_being_written_is_not_done() {
        let d = temp("writing");
        let f = d.join("unit_0.m3c");
        std::fs::write(&f, [7u8; 100]).unwrap();
        std::fs::write(marker_of(&f), "someone").unwrap();
        assert!(!is_done_file(&f));
        std::fs::remove_file(marker_of(&f)).unwrap();
        assert!(is_done_file(&f));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_unit_is_claimed_once_and_taken_over_when_its_computer_is_gone() {
        let d = temp("claims");
        let swarm = d.join("swarm");
        let owner = Heart::new(&swarm, "owner");
        let mut settings = BTreeMap::new();
        settings.insert("units".into(), "1".into());
        let job = new_job(&swarm, JobKind::MonteCarlo, "claims", &owner, settings).unwrap();
        let out = job.unit_files()[0].clone();
        let a = Heart::new(&swarm, "a");
        let b = Heart::new(&swarm, "b");
        a.beat_now("idle").unwrap();
        b.beat_now("idle").unwrap();
        // one claim at a time, whoever asks
        let ca = claim_unit(&job, &a, &out).unwrap().expect("a claims");
        assert!(claim_unit(&job, &b, &out).unwrap().is_none());
        assert!(is_busy_file(&swarm, &out));
        // given up: free again
        drop(ca);
        assert!(!is_busy_file(&swarm, &out));
        let cb = claim_unit(&job, &b, &out).unwrap().expect("b claims");
        // the result in place, then the marker gone: done, not claimed again
        write_atomic(&out, &[1u8; 10]).unwrap();
        assert!(!is_done_file(&out));
        drop(cb);
        assert!(is_done_file(&out));
        assert!(claim_unit(&job, &a, &out).unwrap().is_none());
        // a claimer whose heartbeat is gone: its unit (and its half written
        // result) is taken over by one computer only
        std::fs::remove_file(&out).unwrap();
        std::fs::write(marker_of(&out), "gone-123456").unwrap();
        std::fs::write(&out, [9u8; 5]).unwrap();
        assert!(!is_busy_file(&swarm, &out));
        let ca = claim_unit(&job, &a, &out).unwrap().expect("a takes it over");
        assert!(!out.exists());
        assert_eq!(std::fs::read_to_string(marker_of(&out)).unwrap(), a.id());
        assert!(claim_unit(&job, &b, &out).unwrap().is_none());
        drop(ca);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn batch_job_renders_and_collects() {
        let d = temp("batch");
        let s = tiny_scene();
        std::fs::write(d.join("a.m3s"), s.to_text()).unwrap();
        std::fs::write(d.join("b.m3s"), s.to_text()).unwrap();
        let items = vec![BatchItem { input: d.join("a.m3s"), overrides: vec![] }, BatchItem { input: d.join("b.m3s"), overrides: vec!["width = 16".into()] }];
        let bopts = BatchOptions { output_dir: Some(d.join("pics")), ..Default::default() };
        let outs = crate::batch::output_paths(&items, &bopts);
        let swarm = d.join("swarm");
        run_batch(&swarm, &items, &outs, &bopts, &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}).unwrap();
        assert!(outs.iter().all(|o| o.exists()), "{outs:?}");
        assert!(jobs(&swarm).is_empty(), "the job folder is removed");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn worker_takes_a_montecarlo_block_and_the_owner_merges() {
        let d = temp("mc");
        let swarm = d.join("swarm");
        let s = tiny_scene();
        // a job with two blocks, created by hand; a worker pass renders them
        let mut settings = BTreeMap::new();
        settings.insert("units".into(), "2".into());
        settings.insert("passes".into(), "1".into());
        settings.insert("rays".into(), "1000".into());
        let job = new_job(&swarm, JobKind::MonteCarlo, "mc test", &Heart::new(&swarm, "owner"), settings).unwrap();
        std::fs::write(job.in_dir().join("scene.m3s"), s.to_text()).unwrap();
        worker(&swarm, "w1", 0, true, true).unwrap();
        let job = jobs(&swarm).pop().unwrap();
        assert_eq!(job.progress(), (2, 2));
        let mut imgs = Vec::new();
        for f in job.unit_files() {
            let (_, img) = crate::mc::read_m3c(&std::fs::read(f).unwrap()).unwrap();
            assert_eq!(img.passes, 1);
            imgs.push(img);
        }
        // different blocks, different rays
        assert!(imgs[0].recs.iter().zip(&imgs[1].recs).any(|(a, b)| a.col != b.col));
        let mut m = imgs[0].clone();
        m.merge(&imgs[1]).unwrap();
        assert_eq!(m.passes, 2);
        assert!(m.stats().avg_rays > imgs[0].stats().avg_rays);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn animation_job_from_the_owner() {
        let d = temp("anim");
        let s = tiny_scene();
        let mut s2 = s.clone();
        s2.zoom *= 1.5;
        let mut a = Animation { name: "swarmtest".into(), ..Default::default() };
        a.keyframes.push(crate::anim::Keyframe::new(s.clone(), 2));
        a.keyframes.push(crate::anim::Keyframe::new(s2, 1));
        a.set_size_from(&s);
        a.output_folder = d.join("frames").to_string_lossy().into_owned();
        let files = run_animation(&d.join("swarm"), &a, None, &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}).unwrap();
        assert_eq!(files.len(), a.frame_count());
        assert!(files.iter().all(|f| is_done_file(f)), "{files:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn tiles_job_renders_and_previews() {
        let d = temp("tiles");
        let swarm = d.join("swarm");
        let s = tiny_scene();
        // the owner's job, rendered by a worker pass; then its preview and
        // the stitched buffer as run_tiles builds it
        let mut settings = BTreeMap::new();
        for (k, v) in [("cols", "2"), ("rows", "2"), ("downscale", "1"), ("units", "4"), ("width", "24"), ("height", "18")] {
            settings.insert(k.into(), v.into());
        }
        let job = new_job(&swarm, JobKind::Tiles, "picture", &Heart::new(&swarm, "owner"), settings).unwrap();
        std::fs::write(job.in_dir().join("scene.m3s"), s.to_text()).unwrap();
        let p0 = job_preview(&job, 0).unwrap();
        assert_eq!((p0.done, p0.total, p0.width, p0.height), (0, 4, 24, 18));
        // nothing rendered: all dark, with the grid lines between the tiles
        // (108: the grid lines between the tiles, 142: where two of them cross)
        let distinct: std::collections::BTreeSet<u8> = p0.rgb.iter().copied().collect();
        assert!(distinct.iter().all(|&v| v == 40 || v == 108 || v == 142), "nothing rendered: all dark, but {distinct:?}");
        worker(&swarm, "w1", 0, true, true).unwrap();
        let job = jobs(&swarm).pop().unwrap();
        assert_eq!(job.progress(), (4, 4));
        let p = job_preview(&job, 12).unwrap();
        assert_eq!((p.done, p.total, p.width, p.height), (4, 4, 12, 9));
        assert!(p.rgb.iter().any(|&v| v != 40), "the tiles' pictures are in the preview");
        assert!(p.working.is_empty());
        // the whole picture through run_tiles equals the direct calculation
        let full = run_tiles(&swarm, &s, 2, 2, &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}, None).unwrap();
        let (_, direct) = crate::render::calculate(&s, &|_, _| {}).unwrap();
        assert_eq!(full.len(), direct.len());
        let same = full.iter().zip(&direct).filter(|(a, b)| a.zpos_fine == b.zpos_fine).count();
        assert!(same * 10 >= full.len() * 9, "{same} of {} pixels equal", full.len());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn tiles_subset_collects_each_tile() {
        let d = temp("tiles_subset");
        let swarm = d.join("swarm");
        let s = tiny_scene();
        // the Big renders window's way: tiles 1 and 3 of a 2x2 picture only,
        // each handed over as its own files
        let got = Mutex::new(Vec::new());
        let collect = |k: usize, m3i: &Path, png: &Path| -> Result<(), String> {
            let (tw, th, _) = crate::m3p::gbuffer_from_m3i(&std::fs::read(m3i).map_err(|e| e.to_string())?).ok_or("no buffer")?;
            let img = crate::image::load(png)?;
            assert_eq!((img.width, img.height), (tw, th));
            got.lock().unwrap().push((k, tw, th));
            Ok(())
        };
        run_tiles_files(&swarm, &s, 2, 2, Some(&[3, 1, 9]), &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}, None, &collect).unwrap();
        assert_eq!(*got.lock().unwrap(), vec![(3, 12, 9), (1, 12, 9)]);
        // nothing else was rendered, the job is gone (not kept)
        assert!(jobs(&swarm).is_empty());
        assert!(run_tiles_files(&swarm, &s, 2, 2, Some(&[7]), &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}, None, &collect).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn montecarlo_live_hands_over_merged_blocks() {
        let d = temp("mc_live");
        let swarm = d.join("swarm");
        let s = tiny_scene();
        let seen = Mutex::new(Vec::new());
        let live = |m: McImage| seen.lock().unwrap().push(m.passes);
        let img = run_montecarlo_live(&swarm, &s, 3, 1000.0, 1, 0.0, &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}, Some(&live)).unwrap();
        assert_eq!(img.passes, 3);
        let seen = seen.into_inner().unwrap();
        // growing merges, the last one holding every block
        assert!(!seen.is_empty());
        assert!(seen.windows(2).all(|w| w[0] < w[1]), "{seen:?}");
        assert_eq!(*seen.last().unwrap(), 3);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn open_montecarlo_renders_until_stop_and_merges_every_block() {
        let d = temp("mc_open");
        let swarm = d.join("swarm");
        let s = tiny_scene();
        let control = WorkerControl::default();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop2 = stop.clone();
        let opts = RunOptions { name: "owner".into(), quiet: true, keep: true, cancel: Some(std::sync::Arc::new(move || stop2.load(std::sync::atomic::Ordering::Relaxed))), ..Default::default() };
        let img = std::thread::scope(|sc| {
            sc.spawn(|| worker_controlled(&swarm, "w1", 1, false, true, &control).unwrap());
            // Stop once the other computer has its block under way
            sc.spawn(|| {
                let t0 = Instant::now();
                while t0.elapsed() < Duration::from_secs(20) && !jobs(&swarm).first().is_some_and(|j| marker_of(&j.unit_files()[1]).exists()) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                std::thread::sleep(Duration::from_millis(500));
                stop.store(true, std::sync::atomic::Ordering::Relaxed);
            });
            let img = run_montecarlo_live(&swarm, &s, 0, 0.0, 0, 0.0, &opts, &|_| {}, None).unwrap();
            control.stop.store(true, std::sync::atomic::Ordering::Relaxed);
            img
        });
        // both blocks saved, and the picture is their merge
        let job = jobs(&swarm).pop().unwrap();
        assert!(job.open());
        assert_eq!(job.progress(), (2, 2));
        let rays = |img: &McImage| img.recs.iter().map(|r| r.ray_count as u64).sum::<u64>();
        let blocks: Vec<McImage> = job.unit_files().iter().filter(|f| is_done_file(f)).map(|f| crate::mc::read_m3c(&std::fs::read(f).unwrap()).unwrap().1).collect();
        assert!(blocks.iter().all(|b| rays(b) > 0));
        assert_eq!(rays(&img), blocks.iter().map(rays).sum::<u64>());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn snapshots_of_open_blocks_are_merged_until_the_block_is_saved() {
        let d = temp("mc_snap");
        let swarm = d.join("swarm");
        let s = tiny_scene();
        let mut settings = BTreeMap::new();
        settings.insert("units".into(), "2".into());
        settings.insert("open".into(), "true".into());
        let job = new_job(&swarm, JobKind::MonteCarlo, "mc snap", &Heart::new(&swarm, "owner"), settings).unwrap();
        // a block of some passes, and its rays
        let block = |salt: u32, passes: u32| {
            let mut img = McImage::new(s.width as usize, s.height as usize);
            img.seed_salt = salt;
            for _ in 0..passes {
                crate::mc::pass(&s, &mut img, 1, &|_, _| {}, &|| false).unwrap();
            }
            (crate::mc::write_m3c(&s, &img), rays(&img))
        };
        fn rays(img: &McImage) -> u64 {
            img.recs.iter().map(|r| r.ray_count as u64).sum()
        }
        let files = job.unit_files();
        let (b0, r0) = block(1, 1);
        let (snap, r1) = block(2, 2);
        std::fs::write(&files[0], b0).unwrap();
        std::fs::write(files[1].with_extension("snap"), snap).unwrap();
        let mut m = BlockMerge::default();
        assert_eq!(rays(m.update(&job).unwrap()), r0 + r1);
        let v = m.version();
        assert!(m.painted().is_some());
        // nothing new: nothing read again
        m.update(&job);
        assert_eq!(m.version(), v);
        // the block saved: its snapshot is replaced by it
        let (b1, r1b) = block(2, 4);
        assert!(r1b > r1);
        std::fs::write(&files[1], b1).unwrap();
        std::fs::remove_file(files[1].with_extension("snap")).unwrap();
        assert_eq!(rays(m.update(&job).unwrap()), r0 + r1b);
        assert!(m.version() > v);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn frames_selection_renders_only_those() {
        let d = temp("frames");
        let s = tiny_scene();
        let mut s2 = s.clone();
        s2.zoom *= 1.5;
        let mut a = Animation { name: "sel".into(), ..Default::default() };
        a.keyframes.push(crate::anim::Keyframe::new(s.clone(), 3));
        a.keyframes.push(crate::anim::Keyframe::new(s2, 1));
        a.set_size_from(&s);
        a.output_folder = d.join("frames").to_string_lossy().into_owned();
        let files = run_animation(&d.join("swarm"), &a, Some(&[0, 2]), &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {}).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(std::fs::read_dir(d.join("frames")).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn abandoned_and_foreign_jobs() {
        let d = temp("orphans");
        let s = tiny_scene();
        let h = Heart::new(&d, "owner");
        let mut settings = BTreeMap::new();
        for (k, v) in [("units", "1"), ("passes", "1"), ("rays", "1")] {
            settings.insert(k.into(), v.into());
        }
        // a job of another version: left alone
        let mut foreign = new_job(&d, JobKind::MonteCarlo, "foreign", &h, settings.clone()).unwrap();
        std::fs::write(foreign.in_dir().join("scene.m3s"), s.to_text()).unwrap();
        foreign.version = "0.0.1".into();
        write_job(&foreign).unwrap();
        // a job whose owner was never heard of, old enough: removed
        let mut old = new_job(&d, JobKind::MonteCarlo, "old", &h, settings).unwrap();
        std::fs::write(old.in_dir().join("scene.m3s"), s.to_text()).unwrap();
        old.created = now_secs() - ORPHAN_SECS - 1;
        write_job(&old).unwrap();
        worker(&d, "w1", 0, true, true).unwrap();
        let left: Vec<String> = jobs(&d).into_iter().map(|j| j.name).collect();
        assert_eq!(left, vec!["foreign"]);
        assert_eq!(jobs(&d)[0].progress(), (0, 1), "the foreign job was not rendered");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_failing_job_is_noted_and_left() {
        let d = temp("errors");
        let h = Heart::new(&d, "owner");
        let mut settings = BTreeMap::new();
        for (k, v) in [("units", "1"), ("passes", "1"), ("rays", "1")] {
            settings.insert(k.into(), v.into());
        }
        let job = new_job(&d, JobKind::MonteCarlo, "broken", &h, settings).unwrap();
        std::fs::write(job.in_dir().join("scene.m3s"), "bogus line\n").unwrap();
        worker(&d, "w1", 0, true, true).unwrap();
        let job = jobs(&d).pop().unwrap();
        let errs = job.errors();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].0, "w1");
        assert!(status_text(&d).contains("error at w1"));
        // the owner's own failure on a bad file is reported, not waited on
        let items = vec![BatchItem { input: d.join("none.m3s"), overrides: vec![] }];
        let bopts = BatchOptions::default();
        let outs = crate::batch::output_paths(&items, &bopts);
        let r = run_batch(&d, &items, &outs, &bopts, &RunOptions { name: "owner".into(), quiet: true, ..Default::default() }, &|_| {});
        assert!(r.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn status_lists_peers_and_jobs() {
        let d = temp("status");
        let h = Heart::new(&d, "beta");
        h.beat_now("rendering x").unwrap();
        let mut settings = BTreeMap::new();
        settings.insert("units".into(), "3".into());
        new_job(&d, JobKind::MonteCarlo, "thing", &Heart::new(&d, "beta"), settings).unwrap();
        let t = status_text(&d);
        assert!(t.contains("beta") && t.contains("rendering x") && t.contains("thing") && t.contains("0/3"), "{t}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
