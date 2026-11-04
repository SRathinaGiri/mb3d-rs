//! `Mandelbulb3D.ini` next to the program, in MB3D's format
//! (FileHandling.pas `LoadIni` / `SaveIni`): the first seven lines are the
//! working folders, then "Item  value" lines.

use std::path::{Path, PathBuf};

pub const ITEMS: [&str; 41] = [
    "StickOption", "MandRotDeg", "NavSlideStep", "NavLookAngle", "NavFarPlane", "NavFOVy", "NavMaxIts", "AniFrameCount",
    "AniPrevFPS", "AniSmoothPar", "NaviAZERTY", "UserAspect", "M3LFolder", "ImageSharp", "NavFkey", "ScaleDEstop",
    "SaveImagePNG", "BigRendersFolder", "LightMapsFolder", "NavHiQ", "NavDoubleClickMode", "ThreadCount", "VoxelFolder",
    "SavePNGtextPars", "m3dPos", "m3dSize", "FormulaPos", "LightPos", "PostpPos", "ThreadPriority", "DisableTBoost",
    "SaveImgInM3I", "M3CFolder", "Author", "NaviPanelShow", "VisualTheme", "NaviSize", "MeshesFolder",
    "NavExpDetail", "NavExpBright", "NavExpLight",
];

const DEFAULTS: [&str; 41] = [
    "81", "5", "20", "5", "30", "30", "60", "50", "10", "3", "0", "0:0", "", "0", "1", "1", "0", "", "", "No", "No", "Auto", "",
    "No", "65 100", "779 671", "844 100", "844 100", "844 100", "-1", "No", "Yes", "", "", "0", "Glossy", "100%", "",
    "No", "No", "No",
];

/// Indices of `dirs` (MB3D's `IniDirs`).
pub const DIR_M3I: usize = 0;
pub const DIR_M3P: usize = 1;
pub const DIR_IMG: usize = 2;
pub const DIR_FORMULAS: usize = 3;
pub const DIR_M3A: usize = 4;
pub const DIR_ANIOUT: usize = 5;
pub const DIR_BGPIC: usize = 6;
pub const DIR_LIGHTS: usize = 7;
pub const DIR_BIG: usize = 8;
pub const DIR_MAPS: usize = 9;
pub const DIR_VOXEL: usize = 10;
pub const DIR_M3C: usize = 11;
pub const DIR_MESHES: usize = 12;

pub struct Ini {
    pub val: Vec<String>,
    pub dirs: Vec<PathBuf>,
    /// lines of newer versions, kept when saving
    extra: Vec<String>,
    path: PathBuf,
}

impl Ini {
    pub fn load() -> Ini {
        // (one folder: both are the program's folder, as in MB3D)
        let data = crate::appdirs::data_folder();
        let mut ini = Ini::load_from(&crate::appdirs::config_folder().join("Mandelbulb3D.ini"), &data);
        // the shipped folders where they are (installed: not in the data folder)
        for (d, name) in [(DIR_M3P, "M3Parameter"), (DIR_FORMULAS, "M3Formulas"), (DIR_MAPS, "M3Maps")] {
            if ini.dirs[d] == data.join(name) && !ini.dirs[d].is_dir() {
                if let Some(r) = crate::appdirs::resource(name) {
                    ini.dirs[d] = r;
                }
            }
        }
        ini
    }

    pub fn load_from(path: &Path, app: &Path) -> Ini {
        let mut dirs = vec![app.to_path_buf(); 13];
        dirs[DIR_M3P] = app.join("M3Parameter");
        dirs[DIR_FORMULAS] = app.join("M3Formulas");
        dirs[DIR_BIG] = app.join("BigRenders");
        dirs[DIR_MAPS] = app.join("M3Maps");
        dirs[DIR_MESHES] = app.join("Meshes");
        let mut ini = Ini { val: DEFAULTS.iter().map(|s| s.to_string()).collect(), dirs, extra: Vec::new(), path: path.to_path_buf() };
        if let Ok(t) = std::fs::read_to_string(path) {
            let lines: Vec<&str> = t.lines().collect();
            for (i, l) in lines.iter().take(7).enumerate() {
                if !l.trim().is_empty() {
                    ini.dirs[i] = PathBuf::from(l.trim());
                }
            }
            for l in lines.iter().skip(7) {
                let first = l.split_whitespace().next().unwrap_or("");
                match ITEMS.iter().position(|it| *it == first) {
                    Some(i) => ini.val[i] = l.trim()[first.len()..].trim().to_string(),
                    None if l.trim().len() > 3 => ini.extra.push(l.to_string()),
                    None => {}
                }
            }
            for (v, d) in [(12, DIR_LIGHTS), (17, DIR_BIG), (18, DIR_MAPS), (22, DIR_VOXEL), (32, DIR_M3C), (37, DIR_MESHES)] {
                if !ini.val[v].is_empty() {
                    ini.dirs[d] = PathBuf::from(&ini.val[v]);
                }
            }
        }
        ini
    }

    pub fn get(&self, item: &str) -> &str {
        ITEMS.iter().position(|i| *i == item).map(|i| self.val[i].as_str()).unwrap_or("")
    }

    pub fn set(&mut self, item: &str, v: &str) {
        if let Some(i) = ITEMS.iter().position(|i| *i == item) {
            self.val[i] = v.to_string();
        }
    }

    /// A setting of this port that MB3D does not have (kept as an extra
    /// line, which MB3D ignores).
    pub fn get_extra(&self, item: &str) -> Option<&str> {
        self.extra.iter().find_map(|l| {
            let (k, v) = l.trim().split_once(char::is_whitespace)?;
            (k == item).then(|| v.trim())
        })
    }

    pub fn set_extra(&mut self, item: &str, v: &str) {
        let line = format!("{item}  {v}");
        match self.extra.iter_mut().find(|l| l.split_whitespace().next() == Some(item)) {
            Some(l) => *l = line,
            None => self.extra.push(line),
        }
    }

    pub fn dir(&self, i: usize) -> PathBuf {
        self.dirs[i].clone()
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        for (v, d) in [(12, DIR_LIGHTS), (17, DIR_BIG), (18, DIR_MAPS), (22, DIR_VOXEL), (32, DIR_M3C), (37, DIR_MESHES)] {
            self.val[v] = self.dirs[d].display().to_string();
        }
        let mut t = String::new();
        for d in self.dirs.iter().take(7) {
            t.push_str(&format!("{}\r\n", d.display()));
        }
        for (i, it) in ITEMS.iter().enumerate() {
            t.push_str(&format!("{it}  {}\r\n", self.val[i]));
        }
        for e in &self.extra {
            t.push_str(e);
            t.push_str("\r\n");
        }
        std::fs::write(&self.path, t)
    }
}

/// "a b" -> (a, b) for the position / size items.
pub fn two_ints(s: &str, d: (i32, i32)) -> (i32, i32) {
    let mut it = s.split_whitespace().map(|v| v.parse::<i32>().ok());
    (it.next().flatten().unwrap_or(d.0), it.next().flatten().unwrap_or(d.1))
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join(format!("mb3d-ini-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("Mandelbulb3D.ini");
        let mut a = super::Ini::load_from(&p, &dir);
        assert_eq!(a.get("VisualTheme"), "Glossy");
        a.set("Author", "someone");
        a.save().unwrap();
        let b = super::Ini::load_from(&p, &dir);
        assert_eq!(b.get("Author"), "someone");
        assert_eq!(b.dirs[1], dir.join("M3Parameter"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
