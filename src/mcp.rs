//! `mb3d mcp`: the program's tools for AI assistants (Model Context
//! Protocol): an assistant on this computer (Claude's desktop app, Claude
//! Code, ...) starts `mb3d mcp` and sends it JSON-RPC requests, a line each,
//! on the standard input; the answers go to the standard output.  No network:
//! the assistant's app starts the program, nothing listens anywhere.
//!
//! The tools read parameter files and write only into one folder (`AI` in
//! the data folder, or `--output`); they never delete anything.  A picture
//! larger than `--max-pixels` (4K) is calculated only after the user agreed
//! (`confirm_large`), and a calculation stops after `--max-minutes` (10).
//! Every call goes to the log (kind `mcp`).

use crate::json::{self, obj, s, Value};
use crate::scene::Scene;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The protocol versions this side speaks (the newest first).
const PROTOCOLS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// The previews returned to the assistant: at most this wide or high.
const THUMB: usize = 512;

pub struct Options {
    pub output: PathBuf,
    pub max_pixels: u64,
    pub max_secs: u64,
}

impl Default for Options {
    fn default() -> Options {
        Options { output: crate::appdirs::work_folder("AI"), max_pixels: 3840 * 2160, max_secs: 600 }
    }
}

const INSTRUCTIONS: &str = "mb3d renders 3D fractals (Mandelbulb3D). Work with parameter files \
(.m3p, .m3i, text parameters): describe_scene shows a file's settings as `key = value` lines; \
change_scene writes a copy with changed keys (the same `key = value` lines, e.g. light colours, \
zoom, iterations, formula values); render calculates a picture (preview: true for a quick small \
one) and shows it to you, so you can judge it and adjust. recent_pictures lists what the user \
calculated or opened lately in the desktop program, recent_log what happened there. \
Pictures and files are written only into the output folder named in the answers. Renders above \
the size limit need the user's agreement: ask, then pass confirm_large: true.";

/// Serves requests until the input ends.
pub fn serve(opt: Options) -> Result<(), String> {
    crate::log::info("mcp", &format!("MCP mode started; output folder {}", opt.output.display()));
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("standard input: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match json::parse(&line) {
            Ok(req) => handle(&opt, &req),
            Err(e) => Some(error_reply(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(r) = reply {
            writeln!(out, "{}", r.to_json()).map_err(|e| format!("standard output: {e}"))?;
            out.flush().map_err(|e| format!("standard output: {e}"))?;
        }
    }
    crate::log::info("mcp", "MCP mode ended (the assistant closed it)");
    Ok(())
}

fn error_reply(id: Value, code: i32, msg: &str) -> Value {
    obj([("jsonrpc", s("2.0")), ("id", id), ("error", obj([("code", Value::Num(code as f64)), ("message", s(msg))]))])
}

fn result_reply(id: Value, result: Value) -> Value {
    obj([("jsonrpc", s("2.0")), ("id", id), ("result", result)])
}

/// The answer to a request (None for a notification).
fn handle(opt: &Options, req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let id = req.get("id").cloned();
    let params = req.get("params").cloned().unwrap_or(Value::Obj(Vec::new()));
    let Some(id) = id else {
        return None; // a notification (initialized, cancelled, ...)
    };
    Some(match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOLS[0]);
            let version = if PROTOCOLS.contains(&asked) { asked } else { PROTOCOLS[0] };
            result_reply(
                id,
                obj([
                    ("protocolVersion", s(version)),
                    ("capabilities", obj([("tools", obj([("listChanged", Value::Bool(false))]))])),
                    ("serverInfo", obj([("name", s("mb3d")), ("title", s("Mandelbulb3D (mb3d-rs)")), ("version", s(env!("CARGO_PKG_VERSION")))])),
                    ("instructions", s(INSTRUCTIONS)),
                ]),
            )
        }
        "ping" => result_reply(id, Value::Obj(Vec::new())),
        "tools/list" => result_reply(id, obj([("tools", tools())])),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let args = params.get("arguments").cloned().unwrap_or(Value::Obj(Vec::new()));
            crate::log::info("mcp", &format!("tool {name} {}", args.to_json()));
            let t0 = Instant::now();
            let r = call(opt, &name, &args);
            let secs = t0.elapsed().as_secs_f64();
            let (content, is_error) = match r {
                Ok(c) => {
                    crate::log::info("mcp", &format!("tool {name} done in {secs:.1} s"));
                    (c, false)
                }
                Err(e) => {
                    crate::log::warn("mcp", &format!("tool {name}: {e}"));
                    (vec![text(&e)], true)
                }
            };
            result_reply(id, obj([("content", Value::Arr(content)), ("isError", Value::Bool(is_error))]))
        }
        _ => error_reply(id, -32601, &format!("unknown method {method}")),
    })
}

fn text(t: &str) -> Value {
    obj([("type", s("text")), ("text", s(t))])
}

fn image(rgb: &[u8], w: usize, h: usize) -> Value {
    let (t, tw, th) = crate::history::downscale(rgb, w, h, THUMB, THUMB);
    obj([("type", s("image")), ("data", s(json::base64(&crate::png::encode_rgb(tw, th, &t)))), ("mimeType", s("image/png"))])
}

/// A property of a tool's input: (name, JSON type, description).
fn schema(props: &[(&str, &str, &str)], required: &[&str]) -> Value {
    Value::Obj(vec![
        ("type".into(), s("object")),
        ("properties".into(), Value::Obj(props.iter().map(|(n, t, d)| (n.to_string(), obj([("type", s(*t)), ("description", s(*d))]))).collect())),
        ("required".into(), Value::Arr(required.iter().map(|r| s(*r)).collect())),
    ])
}

fn tool(name: &str, title: &str, description: &str, input: Value) -> Value {
    obj([("name", s(name)), ("title", s(title)), ("description", s(description)), ("inputSchema", input)])
}

fn tools() -> Value {
    Value::Arr(vec![
        tool(
            "describe_scene",
            "Describe a parameter file",
            "The settings of a Mandelbulb3D parameter file (.m3p, .m3i or text parameters) as `key = value` lines: size, camera, formulas and their values, lights, colours, quality. Use the same keys with change_scene.",
            schema(&[("file", "string", "the parameter file (a full path)")], &["file"]),
        ),
        tool(
            "change_scene",
            "Change settings and save a copy",
            "Writes a copy of a parameter file with some settings changed (the file itself stays as it is). `changes` are `key = value` lines as describe_scene shows them, one per line. The copy goes into the output folder; its path is in the answer.",
            schema(
                &[
                    ("file", "string", "the parameter file to start from"),
                    ("changes", "string", "`key = value` lines"),
                    ("name", "string", "a name for the copy (without extension); optional"),
                ],
                &["file", "changes"],
            ),
        ),
        tool(
            "render",
            "Render a picture",
            "Calculates the picture of a parameter file (with its shadows, ambient occlusion and other effects) and saves it as PNG in the output folder; the answer shows it (at most 512 pixels). `preview: true` renders a quick small one (640 wide) to judge a change. Without width/height the file's own size is used. Above the size limit (4K by default) ask the user first and pass `confirm_large: true`.",
            schema(
                &[
                    ("file", "string", "the parameter file"),
                    ("preview", "boolean", "a quick small picture"),
                    ("width", "number", "picture width in pixels (keeps the proportions unless height is given too)"),
                    ("height", "number", "picture height in pixels"),
                    ("name", "string", "a name for the PNG (without extension); optional"),
                    ("confirm_large", "boolean", "the user agreed to a picture above the size limit"),
                ],
                &["file"],
            ),
        ),
        tool(
            "variations",
            "Make random variations",
            "Breeds 15 random variations of a parameter file (Mandelbulb3D's MutaGen: other formulas, values, iterations) and shows them on one sheet. Each is saved as a parameter file in a folder in the output folder; render one of them to see it large.",
            schema(&[("file", "string", "the parameter file"), ("seed", "number", "a number for repeatable variations; optional")], &["file"]),
        ),
        tool(
            "recent_pictures",
            "What the user worked on lately",
            "The pictures the user calculated in the desktop program lately (their parameter files, kept in its History), and the files opened and saved, newest first.",
            schema(&[("count", "number", "how many (default 10)")], &[]),
        ),
        tool(
            "recent_log",
            "What happened in the program lately",
            "The newest lines of the program's log: what the user did in its windows, calculations with their times, messages, warnings and errors.",
            schema(
                &[
                    ("lines", "number", "how many lines (default 40)"),
                    ("kind", "string", "only one kind: ui, calc, message, file, window, console, session, mcp; optional"),
                ],
                &[],
            ),
        ),
    ])
}

fn arg_str<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty())
}

fn arg_num(a: &Value, k: &str) -> Option<f64> {
    a.get(k).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn load(file: &str) -> Result<(Scene, Vec<String>), String> {
    let p = Path::new(file);
    if !p.is_file() {
        return Err(format!("{file}: no such file"));
    }
    crate::animfile::load_scene(p)
}

/// A name usable as a file name.
fn clean(name: &str) -> String {
    let n: String = name.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
    let n = n.trim().trim_matches('.').to_string();
    if n.is_empty() { "picture".into() } else { n.chars().take(100).collect() }
}

/// A path in the output folder that does not exist yet.
fn free_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let p = dir.join(format!("{stem}.{ext}"));
    if !p.exists() {
        return p;
    }
    (2..10_000).map(|i| dir.join(format!("{stem} {i}.{ext}"))).find(|p| !p.exists()).unwrap_or(p)
}

fn stem_of(file: &str) -> String {
    Path::new(file).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "picture".into())
}

fn call(opt: &Options, name: &str, a: &Value) -> Result<Vec<Value>, String> {
    match name {
        "describe_scene" => {
            let file = arg_str(a, "file").ok_or("file is missing")?;
            let (sc, notes) = load(file)?;
            let mut t = format!("{file}\n{} x {} pixels; formulas: {}\n\n", sc.width, sc.height, formulas(&sc));
            for n in notes {
                t.push_str(&format!("note: {n}\n"));
            }
            t.push_str(&sc.to_text());
            Ok(vec![text(&t)])
        }
        "change_scene" => {
            let file = arg_str(a, "file").ok_or("file is missing")?;
            let changes = a.get("changes").and_then(Value::as_str).ok_or("changes are missing")?;
            let (sc, _) = load(file)?;
            let before = sc.to_text();
            let changed = sc.apply(changes)?;
            let after = changed.to_text();
            let diff: Vec<&str> = after.lines().filter(|l| !before.lines().any(|b| b == *l)).collect();
            std::fs::create_dir_all(&opt.output).map_err(|e| format!("{}: {e}", opt.output.display()))?;
            let stem = clean(arg_str(a, "name").map(String::from).as_deref().unwrap_or(&format!("{} changed", stem_of(file))));
            let p = free_path(&opt.output, &stem, "m3p");
            std::fs::write(&p, crate::m3p::write(&changed)).map_err(|e| format!("{}: {e}", p.display()))?;
            let t = if diff.is_empty() {
                format!("Saved {} (no setting changed: check the keys with describe_scene).", p.display())
            } else {
                format!("Saved {}\nChanged:\n{}", p.display(), diff.join("\n"))
            };
            Ok(vec![text(&t)])
        }
        "render" => render(opt, a),
        "variations" => variations(opt, a),
        "recent_pictures" => {
            let n = arg_num(a, "count").unwrap_or(10.0).clamp(1.0, 100.0) as usize;
            let mut lines = Vec::new();
            for e in crate::history::list().into_iter().take(n) {
                let what = match e.action {
                    crate::history::Action::Calc => "calculated",
                    crate::history::Action::Open => "opened",
                    crate::history::Action::Save => "saved",
                };
                lines.push(format!("{} UTC  {what}  {}", e.when(), e.path.display()));
            }
            Ok(vec![text(&if lines.is_empty() { "Nothing in the History yet.".into() } else { lines.join("\n") })])
        }
        "recent_log" => {
            let n = arg_num(a, "lines").unwrap_or(40.0).clamp(1.0, 500.0) as usize;
            let kind = arg_str(a, "kind");
            let mut out: Vec<String> = Vec::new();
            for (_, p) in crate::log::days() {
                let mut day: Vec<String> = crate::log::read(&p).into_iter().filter(|e| kind.is_none_or(|k| e.kind == k)).map(|e| e.line()).collect();
                day.append(&mut out);
                out = day;
                if out.len() >= n {
                    break;
                }
            }
            let skip = out.len().saturating_sub(n);
            Ok(vec![text(&if out.is_empty() { "The log is empty.".into() } else { out[skip..].join("\n") })])
        }
        _ => Err(format!("unknown tool {name}")),
    }
}

fn formulas(sc: &Scene) -> String {
    let v: Vec<String> = sc.formulas.iter().filter(|f| f.iterations > 0).map(|f| format!("{} x{}", f.formula.name(), f.iterations)).collect();
    v.join(" + ")
}

fn render(opt: &Options, a: &Value) -> Result<Vec<Value>, String> {
    let file = arg_str(a, "file").ok_or("file is missing")?;
    let (mut sc, _) = load(file)?;
    let preview = a.get("preview").and_then(Value::as_bool).unwrap_or(false);
    let (w0, h0) = (sc.width.max(1) as f64, sc.height.max(1) as f64);
    // the size: the file's, a preview's, or the one asked for (MB3D's own
    // scaling keeps the picture's look)
    let (w, h) = match (preview, arg_num(a, "width"), arg_num(a, "height")) {
        (true, _, _) => {
            let f = (640.0 / w0).min(1.0);
            ((w0 * f).round(), (h0 * f).round())
        }
        (false, Some(w), Some(h)) => (w.round(), h.round()),
        (false, Some(w), None) => (w.round(), (h0 * w / w0).round()),
        (false, None, Some(h)) => ((w0 * h / h0).round(), h.round()),
        (false, None, None) => (w0, h0),
    };
    if !(16.0..=32768.0).contains(&w) || !(16.0..=32768.0).contains(&h) {
        return Err(format!("{w} x {h} pixels is not a picture size (16 to 32768)"));
    }
    if (w * h) as u64 > opt.max_pixels && !a.get("confirm_large").and_then(Value::as_bool).unwrap_or(false) {
        return Err(format!(
            "{w} x {h} pixels is above the limit of {} megapixels set for assistants: ask the user whether to render it, then pass confirm_large: true.",
            opt.max_pixels / 1_000_000
        ));
    }
    if (w, h) != (w0, h0) {
        sc.scale_image(w / w0);
        sc.width = w as i32;
        sc.height = h as i32;
    }
    std::fs::create_dir_all(&opt.output).map_err(|e| format!("{}: {e}", opt.output.display()))?;
    let t0 = Instant::now();
    let limit = Duration::from_secs(opt.max_secs);
    let cancel = || t0.elapsed() > limit;
    let (p, g) = crate::render::calculate_with(&sc, &|_, _| {}, &crate::render::no_stage, &cancel, 2, None, None)?;
    if cancel() {
        return Err(format!("stopped after the time limit of {} minutes set for assistants", opt.max_secs / 60));
    }
    let rgb = crate::render::paint(&sc, &p, &g);
    let secs = t0.elapsed().as_secs_f64();
    let (pw, ph) = (sc.width as usize, sc.height as usize);
    let default_stem = format!("{} {pw}x{ph}{}", stem_of(file), if preview { " preview" } else { "" });
    let stem = clean(arg_str(a, "name").unwrap_or(&default_stem));
    let path = free_path(&opt.output, &stem, "png");
    crate::png::write_rgb(&path.to_string_lossy(), pw, ph, &rgb).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(feature = "gpu")]
    let on = if crate::gpu::enabled() { format!(" ({})", crate::gpu::last_status()) } else { " (on the CPU)".to_string() };
    #[cfg(not(feature = "gpu"))]
    let on = " (on the CPU)".to_string();
    let t = format!("Rendered {pw} x {ph} in {secs:.1} s{on}; saved as {}", path.display());
    crate::log::info("mcp", &t);
    Ok(vec![text(&t), image(&rgb, pw, ph)])
}

fn variations(opt: &Options, a: &Value) -> Result<Vec<Value>, String> {
    let file = arg_str(a, "file").ok_or("file is missing")?;
    let (sc, _) = load(file)?;
    let seed = arg_num(a, "seed").map(|v| v as u64).unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
    });
    let dir = opt.output.join(clean(&format!("variations of {} {seed}", stem_of(file))));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let size = 160usize;
    let ph = (size as f64 * sc.height as f64 / sc.width.max(1) as f64).round().max(2.0) as usize;
    let mut previews = Vec::new();
    let mut lines = Vec::new();
    let mut err = None;
    let t0 = Instant::now();
    let limit = Duration::from_secs(opt.max_secs);
    let cancel = || t0.elapsed() > limit;
    let mut rng = crate::mutagen::Rng::new(seed);
    let cfg = crate::mutagen::MutationConfig::default();
    let mut on_member = |i: usize, m: &crate::mutagen::Member| {
        let label = crate::mutagen::TREE[i - 1].0;
        let p = dir.join(format!("{label}.m3p"));
        if let Err(e) = std::fs::write(&p, crate::m3p::write(&m.scene)) {
            err = Some(format!("{}: {e}", p.display()));
        }
        let img = crate::mutagen::preview(&m.scene, size, ph, false, threads)
            .map(|(rgb, w, h)| if (w, h) == (size, ph) { rgb } else { vec![0; size * ph * 3] })
            .unwrap_or_else(|_| vec![0; size * ph * 3]);
        previews.push(img);
        lines.push(format!("{i:2}. {} — {}", p.display(), crate::mutagen::caption(&m.scene)));
    };
    crate::mutagen::generation(&cfg, &sc, &mut rng, threads, &mut on_member, &cancel)?;
    if let Some(e) = err {
        return Err(e);
    }
    let (sheet, sw, sh) = crate::mutagen::contact_sheet(&previews, size, ph);
    let _ = crate::png::write_rgb(&dir.join("sheet.png").to_string_lossy(), sw, sh, &sheet);
    let t = format!(
        "15 variations (seed {seed}) in {:.0} s, numbered left to right, top to bottom on the sheet:\n{}",
        t0.elapsed().as_secs_f64(),
        lines.join("\n")
    );
    Ok(vec![text(&t), image(&sheet, sw, sh)])
}

// ---------------------------------------------------------------------------
// the connection to the assistants

/// Where Claude's desktop app keeps its settings, on this system.
fn claude_desktop_config() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Claude").join("claude_desktop_config.json"))
    } else if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Application Support/Claude/claude_desktop_config.json"))
    } else {
        // (no official Linux app: the builds of others keep it here)
        let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home.map(|h| h.join(".config")));
        base.map(|b| b.join("Claude").join("claude_desktop_config.json"))
    }
}

/// The command line program `mb3d` (the one assistants start): this
/// program, or the `mb3d` next to it (from the desktop program).
pub fn cli_program() -> Result<PathBuf, String> {
    let p = std::env::current_exe().map_err(|e| format!("cannot tell where this program is: {e}"))?;
    let p = std::fs::canonicalize(&p).unwrap_or(p);
    let is_cli = p.file_stem().is_some_and(|s| s == "mb3d");
    if is_cli {
        return Ok(p);
    }
    let sibling = p.with_file_name(if cfg!(windows) { "mb3d.exe" } else { "mb3d" });
    if sibling.is_file() {
        Ok(sibling)
    } else {
        Err(format!("the command line program {} is missing next to this one", sibling.display()))
    }
}

/// Whether Claude's desktop app is installed (its settings folder is
/// there) and whether mb3d is in its settings.
pub fn claude_desktop_status() -> (bool, bool) {
    let Some(cfg) = claude_desktop_config() else { return (false, false) };
    let installed = cfg.parent().is_some_and(Path::is_dir);
    let connected = std::fs::read_to_string(&cfg).ok().and_then(|t| json::parse(&t).ok()).is_some_and(|v| v.get("mcpServers").and_then(|m| m.get("mb3d")).is_some());
    (installed, connected)
}

/// Whether Claude Code is installed (`claude` on the PATH, or its folder in
/// the home folder) and whether mb3d is among its servers for the user.
pub fn claude_code_status() -> (bool, bool) {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    let on_path = std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p).any(|d| d.join("claude").is_file() || d.join("claude.exe").is_file() || d.join("claude.cmd").is_file())
    });
    let installed = on_path || home.as_ref().is_some_and(|h| h.join(".claude").is_dir());
    let connected = home
        .and_then(|h| std::fs::read_to_string(h.join(".claude.json")).ok())
        .and_then(|t| json::parse(&t).ok())
        .is_some_and(|v| v.get("mcpServers").and_then(|m| m.get("mb3d")).is_some());
    (installed, connected)
}

/// The `mcpServers` entry for this program.
fn entry(exe: &Path) -> Value {
    obj([("command", s(exe.to_string_lossy())), ("args", Value::Arr(vec![s("mcp")]))])
}

/// Adds (or with `remove` takes out) mb3d in Claude's desktop app's
/// settings, keeping everything else; a copy of the old file is kept next
/// to it.
pub fn claude_desktop(remove: bool, exe: &Path) -> Result<String, String> {
    let cfg = claude_desktop_config().ok_or("cannot find the settings folder of Claude's desktop app")?;
    let dir = cfg.parent().ok_or("bad settings path")?;
    if !dir.is_dir() {
        return Err(format!("Claude's desktop app does not seem to be installed ({} is missing). Install it from claude.ai/download, start it once, then run this again.", dir.display()));
    }
    let old = std::fs::read_to_string(&cfg).unwrap_or_default();
    let mut v = if old.trim().is_empty() { Value::Obj(Vec::new()) } else { json::parse(&old).map_err(|e| format!("{} is not valid JSON ({e}); it was left as it is", cfg.display()))? };
    let Value::Obj(top) = &mut v else { return Err(format!("{} does not hold a JSON object; it was left as it is", cfg.display())) };
    if !top.iter().any(|(k, _)| k == "mcpServers") {
        top.push(("mcpServers".into(), Value::Obj(Vec::new())));
    }
    let servers = top.iter_mut().find(|(k, _)| k == "mcpServers").map(|(_, v)| v).ok_or("mcpServers")?;
    let Value::Obj(list) = servers else { return Err("mcpServers is not an object; the file was left as it is".into()) };
    list.retain(|(k, _)| k != "mb3d");
    if !remove {
        list.push(("mb3d".into(), entry(exe)));
    }
    if !old.is_empty() {
        let backup = cfg.with_extension("json.before-mb3d");
        std::fs::write(&backup, &old).map_err(|e| format!("{}: {e}", backup.display()))?;
    }
    std::fs::write(&cfg, json::pretty(&v)).map_err(|e| format!("{}: {e}", cfg.display()))?;
    crate::log::info("mcp", &format!("{} mb3d in {}", if remove { "removed" } else { "added" }, cfg.display()));
    Ok(if remove {
        format!("mb3d was taken out of Claude's desktop app ({}). Restart the app.", cfg.display())
    } else {
        format!("mb3d was added to Claude's desktop app ({}). Quit the app completely and start it again; then ask it for example: \"What did I render last in mb3d?\"", cfg.display())
    })
}

/// The command that adds mb3d to Claude Code (for the user to run: this
/// program starts no other programs).
pub fn claude_code_command(remove: bool, exe: &Path) -> Result<String, String> {
    let quoted = |p: &Path| {
        let t = p.to_string_lossy();
        if t.contains(' ') { format!("\"{t}\"") } else { t.into_owned() }
    };
    Ok(if remove { "claude mcp remove mb3d --scope user".to_string() } else { format!("claude mcp add mb3d --scope user -- {} mcp", quoted(exe)) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(opt: &Options, line: &str) -> Value {
        handle(opt, &json::parse(line).unwrap()).unwrap()
    }

    #[test]
    fn the_handshake_and_the_tools() {
        let opt = Options { output: std::env::temp_dir().join(format!("mb3d-mcp-test-{}", std::process::id())), ..Options::default() };
        let r = ask(&opt, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#);
        let res = r.get("result").unwrap();
        assert_eq!(res.get("protocolVersion").and_then(Value::as_str), Some("2025-03-26"));
        assert!(handle(&opt, &json::parse(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).unwrap()).is_none());
        let list = ask(&opt, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let Some(Value::Arr(tools)) = list.get("result").and_then(|r| r.get("tools")) else { panic!() };
        assert_eq!(tools.len(), 6);
        let bad = ask(&opt, r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#);
        assert_eq!(bad.get("error").and_then(|e| e.get("code")).and_then(Value::as_f64), Some(-32601.0));
        let missing = ask(&opt, r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"render","arguments":{"file":"/nonexistent.m3p"}}}"#);
        assert_eq!(missing.get("result").and_then(|r| r.get("isError")).and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn too_large_needs_the_users_agreement() {
        let dir = std::env::temp_dir().join(format!("mb3d-mcp-big-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("t.m3s");
        std::fs::write(&f, Scene::preset("Integer Power").unwrap().to_text()).unwrap();
        let opt = Options { output: dir.join("AI"), max_pixels: 100 * 100, max_secs: 60 };
        let args = json::parse(&format!(r#"{{"file":{},"width":400}}"#, s(f.to_string_lossy()).to_json())).unwrap();
        let e = call(&opt, "render", &args).unwrap_err();
        assert!(e.contains("confirm_large"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
