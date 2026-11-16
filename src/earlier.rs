//! Your files from an earlier version.  A release is a folder of its own
//! (`mb3d-rs-<version>-<platform>`), and a copy unpacked where it can
//! write keeps everything in that folder (the one-folder way, as MB3D):
//! the settings, the History, the logs, the parameters saved into
//! `M3Parameter`, the maps added, the Big renders, the meshes.  The next
//! version unpacked beside it starts empty: the History seems to have
//! stopped and the settings to be lost.  On its first start it finds the
//! earlier folder beside it and asks once whether to bring them over: the
//! settings are copied (the earlier program keeps working with its own),
//! what the user made is moved (never overwriting a file of the new
//! version; a day of the History or a log both have is joined), and what
//! cannot be moved is told.  `download.sh` updates its folder in place,
//! and an installed program keeps its data in the user's folders: there
//! is nothing to bring over.

use std::path::{Path, PathBuf};

/// The note in the program's folder that the question was answered: it is
/// asked once per folder.
pub const MARKER: &str = "earlier-version.txt";

/// The settings, copied (the ini with its folders moved along).
const SETTINGS: [&str; 4] = ["Mandelbulb3D.ini", "color_preset_", "preset_", "NaviLightPreset"];

/// What the program brings with every release, or is made again by
/// itself (caches): left where it is.
const PROGRAM: [&str; 12] = ["MB3D-sources", "Languages", "Themes", "oidn", "man", "completions", "ShaderCache", "formula-book", "compiled_shaders.txt", "mb3d_crash.txt", MARKER, "EM_JIT_M3Formulas"];

/// An earlier version's folder found beside this one.
#[derive(Clone, Debug, PartialEq)]
pub struct Earlier {
    pub dir: PathBuf,
}

impl Earlier {
    /// Its folder's name (as the user knows it).
    pub fn name(&self) -> String {
        self.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

/// What was brought over.
#[derive(Debug, Default)]
pub struct Report {
    /// moved or joined (files and folders, counted at the top)
    pub moved: usize,
    /// what could not be moved (still in the earlier folder)
    pub left: Vec<String>,
}

/// The earlier version to ask about, if this copy keeps its files in its
/// own folder and was not asked yet.
pub fn find() -> Option<Earlier> {
    if !crate::appdirs::portable() {
        return None;
    }
    find_beside(&crate::appdirs::app_folder(), env!("CARGO_PKG_VERSION"))
}

/// The most recently used mb3d-rs folder beside `app` that holds settings
/// or a History, and is not of a newer version (by its name, where it
/// says one); `None` once `app` holds the answer ([`MARKER`]).
pub fn find_beside(app: &Path, version: &str) -> Option<Earlier> {
    if app.join(MARKER).exists() {
        return None;
    }
    let parent = app.parent()?;
    let own = std::fs::canonicalize(app).unwrap_or_else(|_| app.to_path_buf());
    let mine = parse_version(version);
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for e in std::fs::read_dir(parent).ok()?.flatten() {
        let d = e.path();
        if !d.is_dir() || std::fs::canonicalize(&d).unwrap_or_else(|_| d.clone()) == own {
            continue;
        }
        // mb3d-rs (its command line program), not MB3D 1.99
        if !(d.join("mb3d").is_file() || d.join("mb3d.exe").is_file()) {
            continue;
        }
        let used = [d.join("Mandelbulb3D.ini"), d.join("History")]
            .iter()
            .filter_map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
            .max();
        let Some(used) = used else { continue };
        let name = d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let (Some(theirs), Some(mine)) = (version_in_name(&name), mine) {
            if theirs >= mine {
                continue;
            }
        }
        if best.as_ref().is_none_or(|(t, _)| used > *t) {
            best = Some((used, d));
        }
    }
    best.map(|(_, dir)| Earlier { dir })
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut it = v.split('.').map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next()??, it.next()??))
}

/// The version in a release folder's name (`mb3d-rs-v0.3.10-x86_64-...`).
fn version_in_name(name: &str) -> Option<(u32, u32, u32)> {
    let i = name.find("-v")?;
    let rest = &name[i + 2..];
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
    parse_version(rest[..end].trim_end_matches('.'))
}

/// Notes the answer in `app` ([`MARKER`]): never asked again.
pub fn note(app: &Path, e: &Earlier, brought: bool) {
    let text = if brought {
        format!("The settings and files of the earlier version in\n{}\nwere brought over into this folder.\n", e.dir.display())
    } else {
        format!("The settings and files of the earlier version in\n{}\nwere not brought over (asked once, answered No).\nTo bring them over later, delete this file and start mb3d-rs.\n", e.dir.display())
    };
    let _ = std::fs::write(app.join(MARKER), text);
}

/// Brings the settings and files of `e` into `app`: the settings are
/// copied (in the ini, the earlier folder's paths become this one's), the
/// rest is moved where `app` has no file of the name; a day of the History
/// (`log.txt`) or a log both have is joined.  Notes the answer.
pub fn bring_over(app: &Path, e: &Earlier) -> Report {
    let mut r = Report::default();
    let Ok(rd) = std::fs::read_dir(&e.dir) else {
        r.left.push(e.dir.display().to_string());
        return r;
    };
    let mut names: Vec<std::ffi::OsString> = rd.flatten().map(|x| x.file_name()).collect();
    names.sort();
    for n in names {
        let s = n.to_string_lossy();
        let from = e.dir.join(&n);
        if SETTINGS.iter().any(|p| s.starts_with(p)) && from.is_file() {
            let ok = if s == "Mandelbulb3D.ini" { copy_ini(&from, &app.join(&n), &e.dir, app) } else { std::fs::copy(&from, app.join(&n)).is_ok() };
            if ok {
                r.moved += 1;
            } else {
                r.left.push(s.into_owned());
            }
            continue;
        }
        if PROGRAM.contains(&s.as_ref()) || is_scratch(&s) {
            continue;
        }
        merge(&from, &app.join(&n), &s, &mut r);
    }
    note(app, e, true);
    r
}

/// Files of the program's own making that belong to one run.
fn is_scratch(name: &str) -> bool {
    name.ends_with(".lock") || name.starts_with(".mb3d-write-test")
}

/// Moves `from` to `to`; where both are folders, their entries one by one;
/// a log both have is joined; another file of the name stays (the new
/// version's is kept).
fn merge(from: &Path, to: &Path, shown: &str, r: &mut Report) {
    let name = from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if is_scratch(&name) {
        return;
    }
    // (a folder entry by entry: what belongs to one run stays behind)
    if from.is_dir() && !to.exists() && std::fs::create_dir(to).is_err() {
        r.left.push(shown.to_string());
        return;
    }
    if !to.exists() {
        match std::fs::rename(from, to) {
            Ok(()) => r.moved += 1,
            Err(_) => r.left.push(shown.to_string()),
        }
        return;
    }
    if from.is_dir() && to.is_dir() {
        let Ok(rd) = std::fs::read_dir(from) else {
            r.left.push(shown.to_string());
            return;
        };
        for x in rd.flatten() {
            let n = x.file_name();
            merge(&x.path(), &to.join(&n), &format!("{shown}/{}", n.to_string_lossy()), r);
        }
        // (empty now: gone; else what stayed is in it)
        let _ = std::fs::remove_dir(from);
        return;
    }
    if from.is_file() && to.is_file() && (name == "log.txt" || name.ends_with(".log")) {
        // the earlier lines first
        let joined = std::fs::read(from).and_then(|mut a| {
            a.extend(std::fs::read(to)?);
            std::fs::write(to, a)
        });
        match joined {
            Ok(()) => {
                let _ = std::fs::remove_file(from);
                r.moved += 1;
            }
            Err(_) => r.left.push(shown.to_string()),
        }
    }
}

/// The ini of the earlier version with its folder's paths turned into
/// `app`'s (a path elsewhere, the user's own folder, stays).
fn copy_ini(from: &Path, to: &Path, old: &Path, app: &Path) -> bool {
    let Ok(t) = std::fs::read_to_string(from) else { return false };
    std::fs::write(to, move_paths(&t, &old.to_string_lossy(), &app.to_string_lossy())).is_ok()
}

/// `old` (a folder) replaced by `new` where a path starts with it: the
/// folder itself or a path in it, not another folder whose name begins
/// the same.
fn move_paths(text: &str, old: &str, new: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut rest = line;
        while let Some(k) = rest.find(old) {
            let after = &rest[k + old.len()..];
            let whole = after.is_empty() || after.starts_with(['/', '\\', '\r']) || after.trim().is_empty();
            let starts = k == 0 || rest[..k].ends_with([' ', '\t']);
            out.push_str(&rest[..k]);
            out.push_str(if whole && starts { new } else { old });
            rest = after;
        }
        out.push_str(rest);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mb3d-earlier-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn put(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    /// A release folder: the programs and what it ships.
    fn release(dir: &Path) {
        put(&dir.join("mb3d"), "");
        put(&dir.join("Mandelbulb3D"), "");
        put(&dir.join("M3Parameter/Shipped.m3p"), "shipped");
        put(&dir.join("Languages/English.lng"), "");
    }

    #[test]
    fn the_earlier_version_beside_is_found_once() {
        let root = tmp("find");
        let old = root.join("mb3d-rs-v0.3.10-x86_64-pc-windows-msvc");
        let new = root.join("mb3d-rs-v0.3.11-x86_64-pc-windows-msvc");
        let newer = root.join("mb3d-rs-v0.3.12-x86_64-pc-windows-msvc");
        let mb3d = root.join("Mandelbulb3Dv199sr37");
        for d in [&old, &new, &newer] {
            release(d);
        }
        // MB3D 1.99 with its History: not this program
        put(&mb3d.join("Mandelbulb3D.exe"), "");
        put(&mb3d.join("History/2020-01-01/x.m3p"), "");
        // nothing used yet: nothing to bring
        assert_eq!(find_beside(&new, "0.3.11"), None);
        put(&old.join("Mandelbulb3D.ini"), "");
        // a newer version's folder is never taken from
        put(&newer.join("Mandelbulb3D.ini"), "");
        assert_eq!(find_beside(&new, "0.3.11"), Some(Earlier { dir: old.clone() }));
        note(&new, &Earlier { dir: old }, false);
        assert_eq!(find_beside(&new, "0.3.11"), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn version_from_the_folder_name() {
        assert_eq!(version_in_name("mb3d-rs-v0.3.10-x86_64-pc-windows-msvc"), Some((0, 3, 10)));
        assert_eq!(version_in_name("mb3d-rs-v1.2.3"), Some((1, 2, 3)));
        assert_eq!(version_in_name("my fractals"), None);
    }

    #[test]
    fn paths_of_the_earlier_folder_move_along() {
        let t = "/x/mb3d-rs-v0.3.10\n/x/mb3d-rs-v0.3.10/M3Parameter\n/x/mb3d-rs-v0.3.10-other/M3Maps\nLightMapsFolder  /x/mb3d-rs-v0.3.10/M3Maps\nBigRendersFolder  /home/me/big\n";
        assert_eq!(
            move_paths(t, "/x/mb3d-rs-v0.3.10", "/x/new"),
            "/x/new\n/x/new/M3Parameter\n/x/mb3d-rs-v0.3.10-other/M3Maps\nLightMapsFolder  /x/new/M3Maps\nBigRendersFolder  /home/me/big\n"
        );
        let w = "C:\\mb\\old\r\nM3LFolder  C:\\mb\\old\\M3Lights\r\n";
        assert_eq!(move_paths(w, "C:\\mb\\old", "C:\\mb\\new"), "C:\\mb\\new\r\nM3LFolder  C:\\mb\\new\\M3Lights\r\n");
    }

    #[test]
    fn settings_are_copied_and_the_users_files_moved() {
        let root = tmp("bring");
        let old = root.join("mb3d-rs-v0.3.10-x");
        let new = root.join("mb3d-rs-v0.3.11-x");
        release(&old);
        release(&new);
        let o = old.to_string_lossy().into_owned();
        put(&old.join("Mandelbulb3D.ini"), &format!("{o}\n{o}/M3Parameter\nLightMapsFolder  {o}/M3Maps\nBigRendersFolder  /elsewhere\n"));
        put(&old.join("color_preset_1.bin"), "c");
        put(&old.join("History/2026-11-10/10h00m00s.m3p"), "a");
        put(&old.join("History/2026-11-10/log.txt"), "old line\n");
        put(&old.join("History/2026-11-11/09h00m00s.m3p"), "b");
        put(&new.join("History/2026-11-10/log.txt"), "new line\n");
        put(&old.join("M3Parameter/Mine.m3p"), "mine");
        put(&old.join("M3Parameter/Shipped.m3p"), "old shipped");
        put(&old.join("BigRenders/big1/tile.png"), "t");
        put(&old.join("my picture.png"), "p");
        put(&old.join("logs/mb3d-2026-11-10.log"), "log");
        put(&old.join("logs/session-123.lock"), "");
        put(&old.join("Languages/Mine.lng"), "");
        let r = bring_over(&new, &Earlier { dir: old.clone() });
        assert!(r.left.is_empty(), "{:?}", r.left);
        let read = |p: &str| std::fs::read_to_string(new.join(p)).unwrap();
        let n = new.to_string_lossy().into_owned();
        assert_eq!(read("Mandelbulb3D.ini"), format!("{n}\n{n}/M3Parameter\nLightMapsFolder  {n}/M3Maps\nBigRendersFolder  /elsewhere\n"));
        assert_eq!(read("color_preset_1.bin"), "c");
        // the settings stay with the earlier program, what was made moves
        assert!(old.join("Mandelbulb3D.ini").is_file());
        assert_eq!(read("History/2026-11-10/10h00m00s.m3p"), "a");
        assert_eq!(read("History/2026-11-11/09h00m00s.m3p"), "b");
        assert_eq!(read("History/2026-11-10/log.txt"), "old line\nnew line\n");
        assert!(!old.join("History").exists());
        assert_eq!(read("M3Parameter/Mine.m3p"), "mine");
        // a file the new version ships is its own
        assert_eq!(read("M3Parameter/Shipped.m3p"), "shipped");
        assert_eq!(read("BigRenders/big1/tile.png"), "t");
        assert_eq!(read("my picture.png"), "p");
        assert_eq!(read("logs/mb3d-2026-11-10.log"), "log");
        assert!(!new.join("logs/session-123.lock").exists());
        assert!(!new.join("Languages/Mine.lng").exists());
        assert!(new.join(MARKER).is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
