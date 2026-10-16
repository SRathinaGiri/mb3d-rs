//! Where the program keeps its files, and MB3D's working folders
//! (`IniDirs` in `FileHandling.pas`): `History` (the parameters of every
//! calculated image), `Meshes` (BulbTracer output) and `BigRenders` (tiled
//! renders).
//!
//! **One folder** (MB3D's way, the release archives): when the program's
//! folder is writable, everything is next to the program as in MB3D:
//! `Mandelbulb3D.ini`, the presets, History, the shader cache.
//!
//! **Installed** (a program folder the user cannot write, e.g. a
//! distribution package): the user's files go to the standard folders,
//! settings and presets to `$XDG_CONFIG_HOME/mb3d-rs` (`~/.config`),
//! History, BigRenders and Meshes to `$XDG_DATA_HOME/mb3d-rs`
//! (`~/.local/share`), the shader cache to `$XDG_CACHE_HOME/mb3d-rs`
//! (`~/.cache`); on Windows `%APPDATA%` and `%LOCALAPPDATA%`.
//! `MB3D_PORTABLE=1` / `0` chooses one way regardless of the folder.
//!
//! Either way the shipped files (formulas, maps, themes, parameters) are
//! also looked for in `$XDG_DATA_HOME/mb3d-rs` (the user's own: newer
//! formulas found there replace the shipped ones) and in
//! `$XDG_DATA_DIRS/mb3d-rs` (`/usr/local/share`, `/usr/share`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The folder of the executable (MB3D's `AppFolder`).
pub fn app_folder() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

/// The one-folder way (see the module notes): `MB3D_PORTABLE`, else
/// whether a file can be made in the program's folder.  A program below
/// `/usr` is a system installation even where it can write (run as root:
/// its settings do not belong into /usr/bin).
pub fn portable() -> bool {
    static P: OnceLock<bool> = OnceLock::new();
    *P.get_or_init(|| match std::env::var("MB3D_PORTABLE").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => {
            let app = app_folder();
            !(cfg!(unix) && app.starts_with("/usr")) && writable(&app)
        }
    })
}

fn writable(dir: &Path) -> bool {
    let p = dir.join(format!(".mb3d-write-test-{}", std::process::id()));
    match std::fs::OpenOptions::new().write(true).create_new(true).open(&p) {
        Ok(_) => {
            let _ = std::fs::remove_file(&p);
            true
        }
        Err(_) => false,
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).filter(|h| !h.is_empty()).map(PathBuf::from)
}

/// `$var`/mb3d-rs, or `default` (below the home folder)/mb3d-rs; relative
/// values are ignored, as the XDG specification says.
fn xdg(var: &str, default: &str) -> Option<PathBuf> {
    let base = std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute());
    let base = if cfg!(windows) { base } else { base.or_else(|| home().map(|h| h.join(default))) };
    base.map(|b| b.join("mb3d-rs"))
}

/// The user's settings folder of the installed way (`~/.config/mb3d-rs`).
pub fn user_config_dir() -> Option<PathBuf> {
    if cfg!(windows) { xdg("APPDATA", "") } else { xdg("XDG_CONFIG_HOME", ".config") }
}

/// The user's data folder (`~/.local/share/mb3d-rs`): History and the
/// other working folders of the installed way, and the user's own
/// formulas and maps either way.
pub fn user_data_dir() -> Option<PathBuf> {
    if cfg!(windows) { xdg("APPDATA", "") } else { xdg("XDG_DATA_HOME", ".local/share") }
}

/// The user's cache folder of the installed way (`~/.cache/mb3d-rs`).
pub fn user_cache_dir() -> Option<PathBuf> {
    if cfg!(windows) { xdg("LOCALAPPDATA", "") } else { xdg("XDG_CACHE_HOME", ".cache") }
}

/// The folders of a system-wide installation: `$XDG_DATA_DIRS/mb3d-rs`
/// (`/usr/local/share/mb3d-rs`, `/usr/share/mb3d-rs`).
pub fn system_data_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        return Vec::new();
    }
    let v = std::env::var_os("XDG_DATA_DIRS").filter(|v| !v.is_empty()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    std::env::split_paths(&v).filter(|p| p.is_absolute()).map(|p| p.join("mb3d-rs")).collect()
}

/// The program's folder (one folder) or `dir` (installed), created.
fn base(dir: Option<PathBuf>) -> PathBuf {
    if !portable() {
        if let Some(d) = dir {
            if std::fs::create_dir_all(&d).is_ok() {
                return d;
            }
        }
    }
    app_folder()
}

/// Where `Mandelbulb3D.ini` and the presets are kept.
pub fn config_folder() -> PathBuf {
    base(user_config_dir())
}

/// Where History, BigRenders, Meshes are kept.
pub fn data_folder() -> PathBuf {
    base(user_data_dir())
}

/// A settings file to read: the user's (`config_folder`), else the one
/// next to the program (a default shipped with it).
pub fn config_file(name: &str) -> PathBuf {
    let p = config_folder().join(name);
    if p.exists() {
        return p;
    }
    let shipped = app_folder().join(name);
    if shipped.exists() { shipped } else { p }
}

/// A shipped folder (`Themes`, `M3Parameter`, ...): next to the program,
/// in the user's data folder, or in a system-wide installation.
pub fn resource(name: &str) -> Option<PathBuf> {
    std::iter::once(app_folder().join(name))
        .chain(user_data_dir().map(|d| d.join(name)))
        .chain(system_data_dirs().into_iter().map(|d| d.join(name)))
        .find(|p| p.exists())
}

/// `name` in [`data_folder`], created if missing; falls back to the
/// working directory when that cannot be written.
pub fn work_folder(name: &str) -> PathBuf {
    let d = data_folder().join(name);
    if std::fs::create_dir_all(&d).is_ok() {
        return d;
    }
    let d = std::env::current_dir().unwrap_or_default().join(name);
    let _ = std::fs::create_dir_all(&d);
    d
}

/// `name` in the cache folder (next to the program, or
/// `~/.cache/mb3d-rs`), created if missing.
pub fn cache_folder(name: &str) -> PathBuf {
    let d = base(user_cache_dir()).join(name);
    if std::fs::create_dir_all(&d).is_ok() {
        return d;
    }
    work_folder(name)
}

/// UTC date and time as (`yyyy-mm-dd`, `hhhmmmsss`), MB3D's history names.
pub fn history_stamp() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // civil date from days since 1970-01-01 (H. Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}h{:02}m{:02}s", rem / 3600, rem / 60 % 60, rem % 60))
}

/// `StoreHistoryPars`: writes the parameters of a calculated image to
/// `History/<date>/<time> <title>.m3p`; returns the file.
pub fn store_history(sc: &crate::scene::Scene, title: &str) -> Option<PathBuf> {
    let (date, time) = history_stamp();
    let dir = work_folder("History").join(date);
    std::fs::create_dir_all(&dir).ok()?;
    let t: String = title.chars().filter(|c| !"/\\:*?\"<>|".contains(*c)).collect();
    let name = if t.trim().is_empty() || t == "untitled" { format!("{time}.m3p") } else { format!("{time} {}.m3p", t.trim()) };
    let p = dir.join(name);
    std::fs::write(&p, crate::m3p::write(sc)).ok()?;
    Some(p)
}

#[cfg(test)]
mod tests {
    #[test]
    fn stamp_format() {
        let (d, t) = super::history_stamp();
        assert_eq!(d.len(), 10);
        assert!(d.starts_with("20"));
        assert_eq!(t.len(), 9);
        assert_eq!(&t[2..3], "h");
    }
}
