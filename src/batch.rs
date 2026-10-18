//! Batch rendering (BatchForm.pas): a list of parameter files is rendered
//! one after another, each to an image next to it or in an output folder.
//! Like MB3D's batch list, files that cannot be loaded are reported and
//! the others still rendered.  Outputs are claimed while they are
//! calculated (see [`crate::frames::claim`]), so with `skip_existing` an
//! interrupted batch continues where it stopped and several processes can
//! work on the same list.
//!
//! List files hold one parameter file per line, optionally followed by
//! scene changes separated by `|`:
//!
//! ```text
//! # my renders
//! City.m3p
//! 6 AM - Torii temple.m3p | width = 1920 | height = 1080
//! scenes/bulb.m3s | iterations = 40
//! ```

use crate::anim::OutputFormat;
use crate::frames::{claim, render_scaled_with, Skip};
use std::path::{Path, PathBuf};

/// One entry of the batch list.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchItem {
    pub input: PathBuf,
    /// Scene changes for this file (`key = value`)
    pub overrides: Vec<String>,
}

/// Settings for the whole batch.
#[derive(Clone, Debug)]
pub struct BatchOptions {
    /// Output folder; None = next to each parameter file
    pub output_dir: Option<PathBuf>,
    pub format: OutputFormat,
    /// Keep images that exist (continue a batch / share it between processes)
    pub skip_existing: bool,
    /// Size factor (previews), applied after the overrides
    pub scale: Option<f64>,
    /// Calculate n times larger and reduce
    pub aa: usize,
    /// Also write `<name>_depth.png`
    pub depth: bool,
    /// Scene changes for every file
    pub overrides: Vec<String>,
    pub threads: usize,
    /// The item under way keeps a checkpoint (see [`crate::checkpoint`])
    pub resume: crate::checkpoint::Policy,
}

impl Default for BatchOptions {
    fn default() -> Self {
        BatchOptions {
            output_dir: None,
            format: OutputFormat::Png,
            skip_existing: false,
            scale: None,
            aa: 1,
            depth: false,
            overrides: Vec::new(),
            threads: 0,
            resume: crate::checkpoint::Policy::Off,
        }
    }
}

/// Is this a file the batch can render?
pub fn is_param_file(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_ascii_lowercase().as_str(), "m3p" | "m3i" | "m3s" | "txt"))
        .unwrap_or(false)
}

/// Expands the command line inputs: parameter files as given, directories
/// to the parameter files in them (sorted by name).
pub fn expand_inputs(inputs: &[String]) -> Result<Vec<BatchItem>, String> {
    let mut items = Vec::new();
    for i in inputs {
        let p = PathBuf::from(i);
        if p.is_dir() {
            let mut v: Vec<PathBuf> = std::fs::read_dir(&p)
                .map_err(|e| format!("{i}: {e}"))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && is_param_file(p))
                // text files only when they hold MB3D text parameters
                .filter(|p| {
                    !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("txt"))
                        || std::fs::read(p).map(|d| String::from_utf8_lossy(&d).contains("Mandelbulb3Dv")).unwrap_or(false)
                })
                .collect();
            v.sort();
            items.extend(v.into_iter().map(|input| BatchItem { input, overrides: Vec::new() }));
        } else {
            items.push(BatchItem { input: p, overrides: Vec::new() });
        }
    }
    Ok(items)
}

/// Parses a list file (paths relative to `base`).
pub fn parse_list(text: &str, base: &Path) -> Result<Vec<BatchItem>, String> {
    let mut items = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with(';') {
            continue;
        }
        let mut parts = l.split('|').map(str::trim);
        let file = parts.next().unwrap_or("");
        if file.is_empty() {
            return Err(format!("list line {}: no file name", n + 1));
        }
        let overrides: Vec<String> = parts.filter(|s| !s.is_empty()).map(String::from).collect();
        if let Some(bad) = overrides.iter().find(|o| !o.contains('=')) {
            return Err(format!("list line {}: expected key = value, got '{bad}'", n + 1));
        }
        items.push(BatchItem { input: base.join(file), overrides });
    }
    Ok(items)
}

/// The output image of each item; names that would collide in a common
/// output folder get `_2`, `_3`, ... appended.
pub fn output_paths(items: &[BatchItem], opts: &BatchOptions) -> Vec<PathBuf> {
    let mut seen: std::collections::HashMap<PathBuf, usize> = std::collections::HashMap::new();
    items
        .iter()
        .map(|it| {
            let stem = it.input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "image".into());
            let dir = match &opts.output_dir {
                Some(d) => d.clone(),
                None => it.input.parent().map(Path::to_path_buf).unwrap_or_default(),
            };
            let ext = opts.format.extension();
            let mut p = dir.join(format!("{stem}.{ext}"));
            let n = seen.entry(p.clone()).or_insert(0);
            *n += 1;
            if *n > 1 {
                p = dir.join(format!("{stem}_{n}.{ext}"));
            }
            p
        })
        .collect()
}

/// The scene of an item as it is rendered, with the anti-aliasing factor.
pub fn item_scene(item: &BatchItem, opts: &BatchOptions) -> Result<(crate::scene::Scene, usize, Vec<String>), String> {
    let (mut s, notes) = crate::animfile::load_scene(&item.input)?;
    let ov: Vec<String> = opts.overrides.iter().chain(&item.overrides).cloned().collect();
    if !ov.is_empty() {
        s = s.apply(&ov.join("\n"))?;
    }
    if let Some(f) = opts.scale {
        s.scale_image(f);
    }
    let mut aa = opts.aa.max(1);
    if let Some(tl) = s.tiling {
        if tl.downscale > 1 {
            s.scale_image(tl.downscale as f64);
            aa *= tl.downscale as usize;
        }
    }
    if aa > 1 {
        s.scale_image(aa as f64);
    }
    if opts.threads > 0 {
        s.threads = opts.threads;
    }
    Ok((s, aa, notes))
}

/// Result of one item.
#[derive(Clone, Debug, PartialEq)]
pub enum ItemResult {
    Done { output: PathBuf, seconds: f64, notes: Vec<String> },
    Skipped { output: PathBuf, why: Skip },
    Failed { error: String },
}

/// Renders one item into `output`.
pub fn render_item(
    item: &BatchItem,
    output: &Path,
    opts: &BatchOptions,
    progress: &(dyn Fn(usize, usize) + Sync),
    cancel: &(dyn Fn() -> bool + Sync),
) -> ItemResult {
    let run = || -> Result<ItemResult, String> {
        // load first: a broken file must not leave a claimed output behind
        let (s, aa, mut notes) = item_scene(item, opts)?;
        let c = match claim(output, !opts.skip_existing)? {
            Ok(c) => c,
            Err(why) => return Ok(ItemResult::Skipped { output: output.to_path_buf(), why }),
        };
        if opts.format == OutputFormat::M3p {
            c.finish(&crate::m3p::write(&s))?;
            return Ok(ItemResult::Done { output: output.to_path_buf(), seconds: 0.0, notes });
        }
        let name = output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let ck = crate::frames::open_frame_checkpoint(&s, &name, opts.resume);
        notes.extend(crate::frames::continued_note(ck.as_ref()));
        let img = render_scaled_with(&s, aa, opts.depth, false, progress, cancel, ck.as_ref(), opts.resume)?;
        // (the depth first: an item is done once its file is written)
        if let Some(z) = &img.depth {
            let stem = output.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let p = output.with_file_name(format!("{stem}_depth.png"));
            std::fs::write(&p, crate::png::encode_gray16(img.width, img.height, z)).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        c.finish(&crate::frames::encode(opts.format, &img)?)?;
        if let Some(ck) = ck {
            ck.finish();
        }
        Ok(ItemResult::Done { output: output.to_path_buf(), seconds: img.seconds, notes })
    };
    match run() {
        Ok(r) => r,
        Err(error) => ItemResult::Failed { error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_file() {
        let t = "# c\nCity.m3p\n\n6 AM - Torii temple.m3p | width = 1920 | height=1080\n";
        let v = parse_list(t, Path::new("base")).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].input, Path::new("base").join("6 AM - Torii temple.m3p"));
        assert_eq!(v[1].overrides, vec!["width = 1920", "height=1080"]);
        assert!(parse_list("a.m3p | nonsense\n", Path::new(".")).is_err());
    }

    #[test]
    fn output_names() {
        let items = vec![
            BatchItem { input: "a/x.m3p".into(), overrides: vec![] },
            BatchItem { input: "b/x.m3p".into(), overrides: vec![] },
            BatchItem { input: "b/y.m3s".into(), overrides: vec![] },
        ];
        let o = output_paths(&items, &BatchOptions::default());
        assert_eq!(o, vec![PathBuf::from("a/x.png"), PathBuf::from("b/x.png"), PathBuf::from("b/y.png")]);
        let o = output_paths(&items, &BatchOptions { output_dir: Some("out".into()), ..Default::default() });
        assert_eq!(o, vec![PathBuf::from("out/x.png"), PathBuf::from("out/x_2.png"), PathBuf::from("out/y.png")]);
    }

    #[test]
    fn renders_and_reports_failures() {
        let dir = std::env::temp_dir().join(format!("mb3d_batch_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = crate::scene::Scene::preset("Integer Power").unwrap();
        s.width = 40;
        s.height = 30;
        std::fs::write(dir.join("ok.m3s"), s.to_text()).unwrap();
        std::fs::write(dir.join("bad.m3s"), "bogus line\n").unwrap();
        let items = expand_inputs(&[dir.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(items.len(), 2);
        let opts = BatchOptions { skip_existing: true, ..Default::default() };
        let outs = output_paths(&items, &opts);
        let r: Vec<ItemResult> = items.iter().zip(&outs).map(|(i, o)| render_item(i, o, &opts, &|_, _| {}, &|| false)).collect();
        assert!(matches!(r[0], ItemResult::Failed { .. }), "{r:?}");
        assert!(!outs[0].exists());
        assert!(matches!(r[1], ItemResult::Done { .. }), "{r:?}");
        assert!(outs[1].exists());
        // a second run skips the finished image
        let again = render_item(&items[1], &outs[1], &opts, &|_, _| {}, &|| false);
        assert_eq!(again, ItemResult::Skipped { output: outs[1].clone(), why: Skip::Exists });
        let _ = std::fs::remove_dir_all(&dir);
    }
}
