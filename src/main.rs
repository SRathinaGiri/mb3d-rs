//! Command line front end: `mb3d [options] [scene.m3s]`

use mb3d::formulas::Formula;
use mb3d::scene::Scene;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;

const USAGE: &str = "\
mb3d - Mandelbulb3D renderer (Rust port)

USAGE:
    mb3d [OPTIONS] [SCENE_FILE]                                    render one image
    mb3d animate ANIMATION | KEYFRAME_FILES... [OPTIONS]           render an animation
    mb3d batch FILES... | --list LISTFILE [OPTIONS]                render many files
    mb3d voxel FILE [OPTIONS]                                      voxel slices (PNG stack)
    mb3d mesh FILE -o mesh.obj|.ply|.stl [OPTIONS]                 triangle mesh
    mb3d mutagen FILE [-o DIR] [OPTIONS]                           random variations
    mb3d montecarlo FILE [-o image.png] [OPTIONS]                  path traced image
    (mb3d animate --help, mb3d batch --help, ...)

SCENE_FILE is an INI-style scene description (see examples/*.m3s), a
Mandelbulb3D parameter file (.m3p), image file (.m3i, parameters only) or a
text file containing MB3D text parameters (Mandelbulb3Dv18{...}).
Without a scene file the preset for --formula is rendered (run without any
arguments, mb3d renders the \"Integer Power\" preset to mb3d.png).

OPTIONS:
    -f, --formula <NAME>     Built-in formula preset (default: \"Integer Power\")
    -W, --width <N>          Image width
    -H, --height <N>         Image height
    -i, --iterations <N>     Max iterations
        --scale <F>          Scale the image size (e.g. 0.1 for a preview of a .m3p)
        --aa <N>             Anti-aliasing: render N times larger and downsample
    -o, --output <FILE>      Output PNG (default: mb3d.png)
        --depth <FILE>       Also write a 16 bit depth map PNG
        --bits <8|16>        Colour depth of the PNG: 16 keeps the colours as calculated,
                             before MB3D's rounding to 8 bit (smooth gradients for
                             editing; single pictures, not tiles or stereo pairs)
    -s, --set <KEY=VALUE>    Override a scene key (may be repeated)
    -t, --threads <N>        Number of threads (default: all cores)
        --auto-color         Fit the colour range to the rendered surface
        --stats              Print G-buffer statistics
        --cpu                Calculate on the CPU only (by default the graphics
                             card and the CPU share the scenes the card
                             supports; MB3D_GPU=0 also switches the card off)
        --card               Calculate on the graphics card only (waits for its
                             shaders; the CPU takes the scenes it cannot do)
        --save-scene <FILE>  Write the final scene as .m3s text (e.g. to convert a .m3p)
        --save-text <FILE>   Write MB3D text parameters (Mandelbulb3Dv18{...})
        --save-m3p <FILE>    Write a MB3D parameter file (.m3p); parameter inputs are
                             copied unchanged, .m3s scenes are encoded
        --tiles <CxR>        Tiled rendering: calculate the image in C x R tiles one after
                             another (less memory for big images) and stitch them
        --tile <C,R>         Only render tile C,R (1-based) of --tiles; with --save-m3p
                             this writes a MB3D tile parameter file
        --render             Also render when converting with --save-scene/--save-text/--save-m3p
        --stereo-pair <LAYOUT>  Render the left and the right eye image (MB3D's stereo
                             modes, see -s stereo_screen=width,distance,min) and combine
                             them: parallel, cross or anaglyph
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files (also: MB3D_FORMULAS;
                             M3Formulas in the current folder, next to the program,
                             next to the parameter file's folder and in ~/mb3d-rs
                             is searched too)
        --maps <DIR>         Directory with maps and background pictures (also: MB3D_MAPS;
                             M3Maps next to the formula directory is searched too)
        --list-formulas      List the built-in formulas and their options
    -h, --help               Show this help
";

fn main() -> ExitCode {
    let r = run();
    // a shader still compiling in the background is finished and cached
    // before the program ends (see gpu::wait_compiles)
    #[cfg(feature = "gpu")]
    mb3d::gpu::wait_compiles(std::time::Duration::from_secs(120));
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `--gpu` / `--cpu`: the main calculation on the graphics card or not.
fn use_gpu(on: bool) {
    #[cfg(feature = "gpu")]
    mb3d::gpu::set_enabled(on);
    #[cfg(not(feature = "gpu"))]
    if on {
        eprintln!("  --gpu: this program was built without the GPU support (cargo feature \"gpu\"), using the CPU");
    }
}

fn run() -> Result<(), String> {
    #[cfg(feature = "gpu")]
    mb3d::gpu::set_enabled(mb3d::gpu::default_on());
    let all: Vec<String> = std::env::args().skip(1).collect();
    // no arguments at all: render the default preset (with a hint to the help)
    if all.is_empty() {
        eprintln!("no scene file: rendering the \"Integer Power\" preset to mb3d.png (mb3d --help shows the options)");
    }
    match all.first().map(String::as_str) {
        Some("animate") | Some("anim") => return animate(&all[1..]),
        Some("batch") => return batch(&all[1..]),
        Some("voxel") | Some("voxels") => return voxel_cmd(&all[1..]),
        Some("mesh") => return mesh_cmd(&all[1..]),
        Some("mutagen") => return mutagen_cmd(&all[1..]),
        Some("montecarlo") | Some("mc") => return mc_cmd(&all[1..]),
        _ => {}
    }
    let mut args = std::env::args().skip(1);
    let mut formula: Option<String> = None;
    let mut scene_file: Option<String> = None;
    let mut output = "mb3d.png".to_string();
    let mut depth: Option<String> = None;
    let mut overrides: Vec<String> = Vec::new();
    let (mut quiet, mut stats, mut auto_color) = (false, false, false);
    let mut save_scene: Option<String> = None;
    let (mut save_text, mut save_m3p): (Option<String>, Option<String>) = (None, None);
    let mut render_too = false;
    let mut scale: Option<f64> = None;
    let mut aa: usize = 1;
    let mut bits16 = false;
    let mut stereo_pair: Option<mb3d::render::StereoLayout> = None;
    while let Some(a) = args.next() {
        let mut val = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "--list-formulas" => {
                for n in Formula::all_names() {
                    let f = Formula::default_for(n).unwrap();
                    let opts: Vec<String> =
                        f.options().iter().map(|(k, v)| format!("{k} = {v}")).collect();
                    println!("{n:16} {}", opts.join(", "));
                }
                return Ok(());
            }
            "-f" | "--formula" => formula = Some(val(&a)?),
            "-W" | "--width" => overrides.push(format!("width = {}", val(&a)?)),
            "-H" | "--height" => overrides.push(format!("height = {}", val(&a)?)),
            "-i" | "--iterations" => overrides.push(format!("iterations = {}", val(&a)?)),
            "-t" | "--threads" => overrides.push(format!("threads = {}", val(&a)?)),
            "-s" | "--set" => overrides.push(val(&a)?),
            "-o" | "--output" => output = val(&a)?,
            "--depth" => depth = Some(val(&a)?),
            "--bits" => {
                bits16 = match val(&a)?.trim() {
                    "8" => false,
                    "16" => true,
                    _ => return Err("--bits is 8 or 16".into()),
                }
            }
            "--formulas" => mb3d::formulas::add_formula_dir(val(&a)?.into()),
            "--maps" => mb3d::maps::add_map_dir(val(&a)?.into()),
            "--auto-color" => auto_color = true,
            "--aa" => aa = val(&a)?.parse::<usize>().map_err(|_| "bad --aa value".to_string())?.clamp(1, 8),
            "--scale" => scale = Some(val(&a)?.parse::<f64>().map_err(|_| "bad --scale value".to_string())?),
            "--stats" => stats = true,
            "--gpu" => use_gpu(true),
            "--cpu" => use_gpu(false),
            "--card" => {
                use_gpu(true);
                #[cfg(feature = "gpu")]
                mb3d::gpu::set_auto(false);
            }
            "--save-scene" => save_scene = Some(val(&a)?),
            "--save-text" => save_text = Some(val(&a)?),
            "--save-m3p" => save_m3p = Some(val(&a)?),
            "--render" => render_too = true,
            "--stereo-pair" => stereo_pair = Some(mb3d::render::StereoLayout::parse(&val(&a)?)?),
            "--tiles" => overrides.push(format!("tiles = {}", val(&a)?)),
            "--tile" => overrides.push(format!("tile = {}", val(&a)?)),
            "-q" | "--quiet" => quiet = true,
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{USAGE}")),
            s => scene_file = Some(s.to_string()),
        }
    }

    let is_m3p = scene_file
        .as_deref()
        .map(|f| {
            let l = f.to_ascii_lowercase();
            l.ends_with(".m3p")
                || l.ends_with(".m3i")
                || std::fs::read(f).map(|d| String::from_utf8_lossy(&d).contains("Mandelbulb3Dv")).unwrap_or(false)
        })
        .unwrap_or(false);
    let raw_copy = is_m3p && overrides.is_empty() && scale.is_none();
    if raw_copy && (save_text.is_some() || save_m3p.is_some()) {
        let path = scene_file.clone().unwrap();
        let (raw, title) = mb3d::m3p::read_raw(std::path::Path::new(&path))?;
        if let Some(f) = &save_m3p {
            std::fs::write(f, &raw).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote parameters {f}");
            }
        }
        if let Some(f) = &save_text {
            let title = title.unwrap_or_else(|| {
                std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
            });
            std::fs::write(f, mb3d::m3p::raw_to_text(&raw, &title)).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote text parameters {f}");
            }
        }
        if !render_too {
            return Ok(());
        }
    }
    let mut scene = if is_m3p {
        let path = scene_file.clone().unwrap();
        let m = mb3d::m3p::load(std::path::Path::new(&path))?;
        if !quiet {
            eprintln!(
                "loaded {path} (MB3D parameter version {}, {}x{})",
                m.mand_id, m.scene.width, m.scene.height
            );
            for w in &m.warnings {
                eprintln!("  note: {w}");
            }
        }
        let mut s = m.scene;
        if let Some(f) = scale {
            s.scale_image(f);
        }
        s.apply(&overrides.join("\n"))?
    } else {
        // file (or preset) text + command line overrides, which are inserted
        // in front of the first [section]
        let mut text = match (&scene_file, &formula) {
            (Some(f), _) => {
                mb3d::formulas::note_scene_file(std::path::Path::new(f));
                std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?
            }
            (None, f) => Scene::preset(f.as_deref().unwrap_or("Integer Power"))?.to_text(),
        };
        if scene_file.is_some() {
            if let Some(f) = &formula {
                if let Some(i) = text.find("[formula]") {
                    text.truncate(i);
                }
                text.push_str(&format!("[formula]\nname = {f}\n"));
            }
        }
        let idx = if text.starts_with('[') { 0 } else { text.find("\n[").map(|i| i + 1).unwrap_or(text.len()) };
        text.insert_str(idx, &format!("{}\n", overrides.join("\n")));
        let mut s = Scene::parse(&text)?;
        if let Some(f) = scale {
            s.scale_image(f);
        }
        s
    };
    if let Some(f) = &save_scene {
        std::fs::write(f, scene.to_text()).map_err(|e| format!("{f}: {e}"))?;
        if !quiet {
            eprintln!("  wrote scene {f}");
        }
        if !render_too && save_text.is_none() && save_m3p.is_none() {
            return Ok(());
        }
    }
    if !raw_copy && (save_text.is_some() || save_m3p.is_some()) {
        // a scene: encode it as MB3D parameters
        let raw = mb3d::m3p::write(&scene);
        if let Some(f) = &save_m3p {
            std::fs::write(f, &raw).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote parameters {f}");
            }
        }
        if let Some(f) = &save_text {
            let title = scene_file
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "mb3d-rs".into());
            std::fs::write(f, mb3d::m3p::raw_to_text(&raw, &title)).map_err(|e| format!("{f}: {e}"))?;
            if !quiet {
                eprintln!("  wrote text parameters {f}");
            }
        }
        if !render_too {
            return Ok(());
        }
    }
    if aa > 1 {
        scene.scale_image(aa as f64);
    }
    if !quiet {
        let names: Vec<String> =
            scene.formulas.iter().map(|f| format!("{} x{}", f.formula.name(), f.iterations)).collect();
        eprintln!(
            "rendering {}x{}  formulas: {}  max its: {}",
            scene.width,
            scene.height,
            names.join(" + "),
            scene.iterations
        );
    }
    let progress = |done: usize, total: usize| {
        if !quiet && (done % 8 == 0 || done == total) {
            eprint!("\r  {:5.1}%", 100.0 * done as f64 / total as f64);
            let _ = std::io::stderr().flush();
        }
    };
    if bits16 && (stereo_pair.is_some() || scene.tiling.is_some()) {
        return Err("--bits 16 is for single pictures (not with --stereo-pair or --tiles)".into());
    }
    if bits16 && !output.to_ascii_lowercase().ends_with(".png") {
        return Err("--bits 16 writes PNG files: give an output file ending in .png".into());
    }
    if let Some(layout) = stereo_pair {
        if scene.tiling.is_some() {
            return Err("--stereo-pair does not work with tiles".into());
        }
        if auto_color {
            let (_, g) = mb3d::render::calculate(&scene, &|_, _| {})?;
            mb3d::render::auto_color_range(&mut scene, &g);
        }
        let mut eyes = Vec::new();
        for (mode, name) in [(4u8, "left"), (3u8, "right")] {
            let mut s = scene.clone();
            s.stereo_mode = mode;
            let r = mb3d::render(&s, &progress)?;
            if !quiet {
                eprintln!("\r  {name} eye: calc {:.2}s, paint {:.2}s", r.calc_seconds, r.paint_seconds);
            }
            eyes.push(if aa > 1 { mb3d::render::downsample(&r.rgb, r.width, r.height, aa) } else { (r.rgb, r.width, r.height) });
        }
        let (w, h) = (eyes[0].1, eyes[0].2);
        let (rgb, ow, oh) = mb3d::render::compose_stereo(&eyes[0].0, &eyes[1].0, w, h, layout);
        mb3d::png::write_rgb(&output, ow, oh, &rgb).map_err(|e| format!("{output}: {e}"))?;
    } else if let Some(tl) = scene.tiling {
        // anti-aliasing of tile renders: --aa and the tile downscale
        let down = aa * tl.downscale.max(1) as usize;
        if tl.downscale > 1 {
            scene.scale_image(tl.downscale as f64);
        }
        if auto_color {
            // colour range from a small preview of the whole image
            let mut pv = scene.clone();
            pv.tiling = None;
            pv.scale_image((320.0 / pv.width as f64).min(1.0));
            let (_, g) = mb3d::render::calculate(&pv, &|_, _| {})?;
            mb3d::render::auto_color_range(&mut scene, &g);
        }
        if !quiet {
            match tl.pos {
                Some((c, r)) => eprintln!("  tile {}, {} of {}x{}", c + 1, r + 1, tl.cols, tl.rows),
                None => eprintln!("  {}x{} tiles", tl.cols, tl.rows),
            }
        }
        let tile_progress = |done: usize, total: usize, t: [i32; 4]| {
            if !quiet {
                eprintln!("\r  tile {done}/{total} done ({}x{} at {}, {})          ", t[2], t[3], t[0], t[1]);
            }
        };
        let res = mb3d::render::render_tiled(&scene, stats || depth.is_some(), &progress, &tile_progress)?;
        if !quiet {
            eprintln!("  done: calc {:.2}s, paint {:.2}s", res.calc_seconds, res.paint_seconds);
        }
        if let (true, Some(g)) = (stats, &res.gbuffer) {
            print_stats(&mb3d::RenderResult {
                width: res.width,
                height: res.height,
                gbuffer: g.clone(),
                rgb: Vec::new(),
                calc_seconds: 0.0,
                paint_seconds: 0.0,
            });
        }
        let (rgb, ow, oh) = if down > 1 {
            mb3d::render::downsample(&res.rgb, res.width, res.height, down)
        } else {
            (res.rgb, res.width, res.height)
        };
        mb3d::png::write_rgb(&output, ow, oh, &rgb).map_err(|e| format!("{output}: {e}"))?;
        if let (Some(d), Some(g)) = (depth, &res.gbuffer) {
            write_depth(&d, g, res.width, res.height)?;
        }
    } else {
        render_single(&mut scene, auto_color, stats, quiet, aa, bits16, &output, depth, &progress)?;
    }
    let errs = mb3d::iteration::CUSTOM_ERRORS.load(std::sync::atomic::Ordering::Relaxed);
    if errs > 0 {
        eprintln!("  warning: {errs} custom formula calls failed in the interpreter");
    }
    if !quiet {
        eprintln!("  wrote {output}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_single(
    scene: &mut Scene,
    auto_color: bool,
    stats: bool,
    quiet: bool,
    aa: usize,
    bits16: bool,
    output: &str,
    depth: Option<String>,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<(), String> {
    let t0 = Instant::now();
    let (params, gbuffer) = mb3d::render::calculate(scene, progress)?;
    let t1 = Instant::now();
    if auto_color {
        mb3d::render::auto_color_range(scene, &gbuffer);
        if !quiet {
            eprintln!(
                "\r  auto colour range: color_start = {:.1}, color_end = {:.1}",
                scene.lighting.color_start, scene.lighting.color_end
            );
        }
    }
    // 16 bit: the colours before the rounding to 8 bit
    let (rgb, rgb16) = if bits16 { (Vec::new(), Some(mb3d::render::paint16(scene, &params, &gbuffer))) } else { (mb3d::render::paint(scene, &params, &gbuffer), None) };
    let res = mb3d::RenderResult {
        width: scene.width as usize,
        height: scene.height as usize,
        gbuffer,
        rgb,
        calc_seconds: (t1 - t0).as_secs_f64(),
        paint_seconds: t1.elapsed().as_secs_f64(),
    };
    if !quiet {
        eprintln!(
            "\r  done: calc {:.2}s, paint {:.2}s, object coverage {:.1}%",
            res.calc_seconds,
            res.paint_seconds,
            res.coverage() * 100.0
        );
    }
    #[cfg(feature = "gpu")]
    if !quiet && mb3d::gpu::enabled() {
        eprintln!("  calculated on: {}", mb3d::gpu::last_status());
    }
    if stats {
        print_stats(&res);
        // the DOF focus value (sDOFZsharp) that makes the image centre sharp
        let c = &res.gbuffer[res.height / 2 * res.width + res.width / 2];
        if !c.is_background() {
            let z = (((8388352 - (c.zpos_fine >> 8) as i64) as f64 / params.zc_mul + 1.0).powi(2) - 1.0) / params.zcorr;
            eprintln!("  dof_focus for the image centre: {:.5}", z / res.width as f64);
        }
    }
    if let Some(rgb16) = rgb16 {
        let (px, ow, oh) = if aa > 1 { mb3d::render::downsample(&rgb16, res.width, res.height, aa) } else { (rgb16, res.width, res.height) };
        std::fs::write(output, mb3d::png::encode_rgb16(ow, oh, &px)).map_err(|e| format!("{output}: {e}"))?;
    } else {
        let (rgb, ow, oh) = if aa > 1 {
            mb3d::render::downsample(&res.rgb, res.width, res.height, aa)
        } else {
            (res.rgb.clone(), res.width, res.height)
        };
        mb3d::png::write_rgb(&output, ow, oh, &rgb).map_err(|e| format!("{output}: {e}"))?;
    }
    if let Some(d) = depth {
        write_depth(&d, &res.gbuffer, res.width, res.height)?;
    }
    Ok(())
}

fn write_depth(path: &str, g: &[mb3d::gbuffer::SiLight], w: usize, h: usize) -> Result<(), String> {
    let z: Vec<u16> =
        g.iter().map(|s| if s.is_background() { 0 } else { (65535 - (s.zpos() * 2).min(65535)) as u16 }).collect();
    mb3d::png::write_gray16(path, w, h, &z).map_err(|e| format!("{path}: {e}"))
}

fn print_stats(res: &mb3d::RenderResult) {
    let hits: Vec<&mb3d::gbuffer::SiLight> =
        res.gbuffer.iter().filter(|s| !s.is_background()).collect();
    if hits.is_empty() {
        eprintln!("  no object pixels");
        return;
    }
    let pct = |mut v: Vec<u32>| -> String {
        v.sort_unstable();
        let q = |f: f64| v[((v.len() - 1) as f64 * f) as usize];
        format!("min {} p10 {} p50 {} p90 {} max {}", q(0.0), q(0.1), q(0.5), q(0.9), q(1.0))
    };
    eprintln!("  si_gradient: {}", pct(hits.iter().map(|s| s.si_gradient as u32).collect()));
    eprintln!("  otrap:       {}", pct(hits.iter().map(|s| s.otrap as u32).collect()));
    eprintln!("  zpos:        {}", pct(hits.iter().map(|s| s.zpos()).collect()));
    eprintln!("  de steps:    {}", pct(res.gbuffer.iter().map(|s| (s.shadow & 0x3FF) as u32).collect()));
    eprintln!("  amb shadow:  {}", pct(hits.iter().map(|s| s.amb_shadow as u32).collect()));
    let lit: Vec<String> = (0..6)
        .map(|i| {
            let n = hits.iter().filter(|s| s.shadow & (0x400 << i) != 0).count();
            format!("{:.0}%", 100.0 * n as f64 / hits.len() as f64)
        })
        .collect();
    eprintln!("  shadow bits: {} (lights 1-6, or soft value bits)", lit.join(" "));
}

// ---------------------------------------------------------------------------
// mb3d animate

const ANIMATE_USAGE: &str = "\
mb3d animate - render the frames of an animation (MB3D's animation maker)

USAGE:
    mb3d animate ANIMATION [OPTIONS]            a .m3k animation (text) or MB3D .m3a file
    mb3d animate KEYFRAME KEYFRAME... [OPTIONS] keyframes from parameter files (.m3p, .m3s,
                                                text parameters), in this order

Frames are written as <output>/<name><index>.<format> with a 6 digit index,
like MB3D (e.g. frames/flight000001.png).

OPTIONS:
    -o, --output <DIR>       Output folder (default: the animation's, else the current folder)
        --name <NAME>        Project name = file name prefix (default: name of the animation)
        --format <F>         png (default), bmp, or m3p: a parameter file per frame (to
                             render the frames elsewhere, e.g. with mb3d batch)
    -W, --width <N>          Frame width   (default: the animation's / first keyframe's)
    -H, --height <N>         Frame height
        --aa <N>             Calculate frames N times larger and reduce them
                             (MB3D's image scale, anti-aliasing)
        --frames <N>         Sub-frames from each keyframe to the next (keyframe files;
                             with an animation file: for all keyframes)
        --linear             Linear interpolation (default: MB3D's quadratic bezier)
        --bezier             Quadratic bezier interpolation
        --loop               Loop: after the last keyframe the first follows again
        --no-loop
        --start-index <N>    File index of the first frame (default 1)
        --index-step <N>     Increment of the file index (default 1)
        --from <INDEX>       Render only frames from this file index ...
        --to <INDEX>         ... up to this file index
        --frame <INDEX>      Render only this frame
        --every <N>          Render only every N-th frame (quick preview)
        --scale <F>          Render the frames F times the size (e.g. 0.25 for a preview)
        --skip-existing      Keep frames that exist already; continues an interrupted
                             render, and lets several processes share the work
        --overwrite          Render all frames again (default unless the animation says no)
        --depth              Also write depth images (ZBuf <name><index>.png)
        --video <FILE>       Also join the frames into a video (.mp4 or .webm; needs
                             ffmpeg, MB3D_FFMPEG names it if it is not on the PATH);
                             the frame images are kept
        --fps <N>            Frames per second of the video (default 25)
        --stereo <MODE>      pair: a right and a left eye image per frame
                             (<name>Right000001, <name>Left000001), very_left: only
                             the 'very left' images, off (as MB3D's animation maker)
    -s, --set <KEY=VALUE>    Change a scene key in every keyframe (may be repeated)
        --save <FILE>        Save the animation: .m3k (text) or .m3a (MB3D); with
                             keyframe files only the file is written unless --render
        --render             Render too when saving
        --list               Print the frames (file index, keyframe, sub-frame) and exit
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps and background pictures
";

fn animate(argv: &[String]) -> Result<(), String> {
    use mb3d::anim::{Animation, Interpolation, Keyframe, OutputFormat};
    let mut files: Vec<String> = Vec::new();
    let mut overrides: Vec<String> = Vec::new();
    let mut run = mb3d::frames::FrameRun::default();
    let (mut output, mut name, mut format): (Option<String>, Option<String>, Option<String>) = (None, None, None);
    let (mut width, mut height, mut aa, mut frames): (Option<i32>, Option<i32>, Option<u32>, Option<u32>) = (None, None, None, None);
    let (mut ipol, mut looped): (Option<Interpolation>, Option<bool>) = (None, None);
    let (mut start_index, mut index_step): (Option<i64>, Option<i64>) = (None, None);
    let mut overwrite: Option<bool> = None;
    let mut stereo: Option<u32> = None;
    let (mut depth, mut list, mut render_too, mut quiet) = (false, false, false, false);
    let mut save: Option<String> = None;
    let (mut video, mut fps): (Option<std::path::PathBuf>, f64) = (None, mb3d::video::DEFAULT_FPS);
    let mut args = argv.iter();
    let num = |a: &str, v: Option<&String>| -> Result<i64, String> {
        v.ok_or_else(|| format!("{a} needs a value"))?.trim().parse::<i64>().map_err(|_| format!("bad value for {a}"))
    };
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{ANIMATE_USAGE}");
                return Ok(());
            }
            "-o" | "--output" => output = Some(args.next().ok_or("--output needs a value")?.clone()),
            "--name" => name = Some(args.next().ok_or("--name needs a value")?.clone()),
            "--format" => format = Some(args.next().ok_or("--format needs a value")?.clone()),
            "-W" | "--width" => width = Some(num(a, args.next())?.clamp(1, 65535) as i32),
            "-H" | "--height" => height = Some(num(a, args.next())?.clamp(1, 65535) as i32),
            "--aa" => aa = Some(num(a, args.next())?.clamp(1, 16) as u32),
            "--frames" => frames = Some(num(a, args.next())?.clamp(0, 1_000_000) as u32),
            "--stereo" => {
                stereo = Some(match args.next().ok_or("--stereo needs a value")?.to_ascii_lowercase().as_str() {
                    "pair" | "on" | "yes" => 0x40,
                    "very_left" | "very-left" => 0xC0,
                    "off" | "no" => 0,
                    v => return Err(format!("bad --stereo value '{v}' (pair, very_left, off)")),
                })
            }
            "--linear" => ipol = Some(Interpolation::Linear),
            "--bezier" => ipol = Some(Interpolation::Bezier),
            "--loop" => looped = Some(true),
            "--no-loop" => looped = Some(false),
            "--start-index" => start_index = Some(num(a, args.next())?),
            "--index-step" => index_step = Some(num(a, args.next())?.max(1)),
            "--from" => run.from_index = Some(num(a, args.next())?),
            "--to" => run.to_index = Some(num(a, args.next())?),
            "--frame" => {
                let i = num(a, args.next())?;
                run.from_index = Some(i);
                run.to_index = Some(i);
            }
            "--every" => run.every = num(a, args.next())?.max(1) as usize,
            "--scale" => {
                run.preview = args
                    .next()
                    .ok_or("--scale needs a value")?
                    .parse::<f64>()
                    .ok()
                    .filter(|f| *f > 0.0)
                    .ok_or("bad --scale value")?
            }
            "--skip-existing" => overwrite = Some(false),
            "--overwrite" => overwrite = Some(true),
            "--depth" => depth = true,
            "--video" => video = Some(args.next().ok_or("--video needs a file name")?.into()),
            "--fps" => fps = args.next().and_then(|v| v.trim().parse::<f64>().ok()).filter(|f| *f > 0.0 && *f <= 1000.0).ok_or("--fps needs a frame rate (e.g. 25)")?,
            "-s" | "--set" => overrides.push(args.next().ok_or("--set needs a value")?.clone()),
            "--save" => save = Some(args.next().ok_or("--save needs a value")?.clone()),
            "--render" => render_too = true,
            "--list" => list = true,
            "-t" | "--threads" => run.threads = num(a, args.next())?.max(0) as usize,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(args.next().ok_or("--formulas needs a value")?.into()),
            "--maps" => mb3d::maps::add_map_dir(args.next().ok_or("--maps needs a value")?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{ANIMATE_USAGE}")),
            s => files.push(s.to_string()),
        }
    }
    if files.is_empty() {
        return Err(format!("no animation or keyframe files given\n\n{ANIMATE_USAGE}"));
    }
    let is_anim = |f: &str| {
        let l = f.to_ascii_lowercase();
        l.ends_with(".m3a") || l.ends_with(".m3k")
    };
    let note = |m: &str| {
        if !quiet {
            eprintln!("  note: {m}");
        }
    };
    let mut previews: Vec<Option<mb3d::animfile::Preview>>;
    let from_file = files.len() == 1 && is_anim(&files[0]);
    let mut anim = if from_file {
        let path = std::path::Path::new(&files[0]);
        let f = mb3d::animfile::load(path)?;
        for w in &f.warnings {
            note(w);
        }
        previews = f.previews;
        let mut a = f.anim;
        if !quiet {
            eprintln!("loaded {} ({} keyframes, {} frames)", files[0], a.keyframes.len(), a.frame_count());
        }
        // MB3D's output folder is an absolute Windows path
        let foreign = a.output_folder.contains('\\') || a.output_folder.get(1..2) == Some(":");
        if foreign && !cfg!(windows) && output.is_none() {
            note(&format!("output folder '{}' of the .m3a file is not usable here, using the current folder (see --output)", a.output_folder));
            a.output_folder.clear();
        }
        if let Some(n) = frames {
            for k in a.keyframes.iter_mut() {
                k.frames = n;
            }
        }
        a
    } else {
        if files.iter().any(|f| is_anim(f)) {
            return Err("give either one animation file or keyframe parameter files".into());
        }
        let mut a = Animation { name: "anim".into(), ..Default::default() };
        for f in &files {
            let (s, w) = mb3d::animfile::load_scene(std::path::Path::new(f))?;
            for w in w {
                note(&format!("{f}: {w}"));
            }
            let mut k = Keyframe::new(s, frames.unwrap_or(50));
            k.source = Some(f.clone());
            a.keyframes.push(k);
        }
        let first = a.keyframes[0].scene.clone();
        a.set_size_from(&first);
        previews = vec![None; a.keyframes.len()];
        a
    };
    if !overrides.is_empty() {
        let text = overrides.join("\n");
        for (i, k) in anim.keyframes.iter_mut().enumerate() {
            k.scene = k.scene.clone().apply(&text).map_err(|e| format!("keyframe {}: {e}", i + 1))?;
            if let Some(src) = &mut k.source {
                for o in &overrides {
                    src.push('\u{0}');
                    src.push_str(o);
                }
            }
        }
        previews.iter_mut().for_each(|p| *p = None);
    }
    if let Some(w) = width {
        anim.width = w;
    }
    if let Some(h) = height {
        anim.height = h;
    }
    if let Some(n) = aa {
        anim.scale = n;
    }
    if let Some(i) = ipol {
        anim.interpolation = i;
    }
    if let Some(b) = stereo {
        anim.stereo_bits = b;
    }
    if let Some(l) = looped {
        anim.looped = l;
    }
    if let Some(v) = start_index {
        anim.start_index = v;
    }
    if let Some(v) = index_step {
        anim.index_step = v;
    }
    if let Some(o) = overwrite {
        anim.overwrite = o;
    }
    if depth {
        anim.save_depth = true;
    }
    if let Some(n) = name {
        anim.name = n;
    }
    if let Some(o) = output {
        anim.output_folder = o;
    }
    if let Some(f) = format {
        anim.format = OutputFormat::parse(&f)?;
    }
    if anim.keyframes.len() < 2 && !list && save.is_none() {
        return Err("an animation needs at least 2 keyframes".into());
    }

    if let Some(path) = &save {
        let p = std::path::Path::new(path);
        let m3a = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3a"));
        if m3a {
            for (i, pv) in previews.iter_mut().enumerate() {
                if pv.is_none() {
                    *pv = mb3d::frames::keyframe_preview(&anim, i).ok();
                }
            }
        }
        let mut a2 = anim.clone();
        if !m3a {
            // keyframe file references relative to the saved file
            if let Some(dir) = p.parent().and_then(|d| std::fs::canonicalize(if d.as_os_str().is_empty() { std::path::Path::new(".") } else { d }).ok()) {
                for k in a2.keyframes.iter_mut() {
                    if let Some(src) = &mut k.source {
                        let (f, rest) = src.split_once('\u{0}').map(|(a, b)| (a.to_string(), format!("\u{0}{b}"))).unwrap_or((src.clone(), String::new()));
                        if let Ok(abs) = std::fs::canonicalize(&f) {
                            let r = abs.strip_prefix(&dir).map(|r| r.to_path_buf()).unwrap_or(abs);
                            *src = format!("{}{rest}", r.to_string_lossy());
                        }
                    }
                }
                if let Ok(abs) = std::fs::canonicalize(if a2.output_folder.is_empty() { "." } else { &a2.output_folder }) {
                    if !a2.output_folder.is_empty() {
                        a2.output_folder = abs.strip_prefix(&dir).map(|r| r.to_path_buf()).unwrap_or(abs).to_string_lossy().into_owned();
                    }
                }
            }
        }
        mb3d::animfile::save(p, &a2, &previews)?;
        if !quiet {
            eprintln!("  wrote {path} ({} keyframes, {} frames)", anim.keyframes.len(), anim.frame_count());
        }
        if !render_too && !list {
            return Ok(());
        }
    }

    let frames_to_do = run.frames(&anim);
    if list {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{} frames, {} keyframes, {}x{}{}", anim.frame_count(), anim.keyframes.len(), anim.width, anim.height,
            if anim.scale > 1 { format!(" (calculated x{})", anim.scale) } else { String::new() });
        for &f in &frames_to_do {
            let p = anim.frame_pos(f).unwrap();
            let k = &anim.keyframes[p.key];
            if writeln!(out, "{:6}  keyframe {:3}  {:4}/{:<4}  {}", anim.file_index(f), p.key + 1, p.sub, k.frames, anim.frame_file(f).display()).is_err() {
                break;
            }
        }
        return Ok(());
    }
    if frames_to_do.is_empty() {
        return Err("no frames to render (check --from/--to/--frame)".into());
    }
    if video.is_some() {
        if anim.format == OutputFormat::M3p {
            return Err("--video needs images (--format png, bmp or jpg), not parameter files".into());
        }
        if anim.stereo_bits & 1 != 0 {
            return Err("--video makes no stereo videos (render the eyes separately)".into());
        }
        mb3d::video::check()?;
    }
    if !quiet {
        eprintln!(
            "rendering {} of {} frames at {}x{}{} to {}",
            frames_to_do.len(),
            anim.frame_count(),
            anim.width,
            anim.height,
            if anim.scale > 1 { format!(" (x{} anti-aliasing)", anim.scale) } else { String::new() },
            anim.frame_file(frames_to_do[0]).parent().map(|p| p.display().to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| ".".into())
        );
    }
    let t0 = Instant::now();
    let (mut done, mut skipped, mut secs) = (0usize, 0usize, 0.0f64);
    let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    for (n, &f) in frames_to_do.iter().enumerate() {
        let label = format!("frame {}/{} ({:06})", n + 1, frames_to_do.len(), anim.file_index(f));
        let progress = |d: usize, t: usize| {
            if !quiet && tty && (d % 8 == 0 || d == t) {
                eprint!("\r  {label} {:5.1}%", 100.0 * d as f64 / t as f64);
                let _ = std::io::stderr().flush();
            }
        };
        match mb3d::frames::render_frame_to_file(&anim, f, &run, &progress, &|| false)? {
            mb3d::frames::FrameResult::Written { path, seconds } => {
                done += 1;
                secs += seconds;
                if !quiet {
                    let left = frames_to_do.len() - n - 1;
                    let eta = if done > 0 { secs / done as f64 * left as f64 } else { 0.0 };
                    let cr = if tty { "\r" } else { "" };
                    eprintln!("{cr}  {label} {:.1}s  {}  (left: {})          ", seconds, path.display(), fmt_dur(eta));
                }
            }
            mb3d::frames::FrameResult::Skipped { path, why } => {
                skipped += 1;
                if !quiet {
                    let w = if why == mb3d::frames::Skip::Busy { "being rendered by another process" } else { "exists" };
                    eprintln!("  {label} skipped, {} {w}", path.display());
                }
            }
        }
    }
    if !quiet {
        eprintln!("done: {done} frames rendered, {skipped} skipped, {}", fmt_dur(t0.elapsed().as_secs_f64()));
    }
    if let Some(v) = video {
        let files: Vec<std::path::PathBuf> = frames_to_do.iter().map(|&f| anim.frame_file(f)).collect();
        let t = Instant::now();
        mb3d::video::encode(&files, fps, &v)?;
        if !quiet {
            eprintln!("video: {} ({} frames at {fps} per second, {})", v.display(), files.len(), fmt_dur(t.elapsed().as_secs_f64()));
        }
    }
    Ok(())
}

fn fmt_dur(s: f64) -> String {
    let s = s.max(0.0).round() as u64;
    if s >= 3600 {
        format!("{}h{:02}m", s / 3600, (s / 60) % 60)
    } else if s >= 60 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{s}s")
    }
}

const BATCH_USAGE: &str = "\
mb3d batch - render many parameter files (MB3D's batch rendering)

USAGE:
    mb3d batch FILE_OR_DIR... [OPTIONS]
    mb3d batch --list LISTFILE [OPTIONS]

Inputs are .m3p, .m3i, .m3s or text parameter files; a directory stands for
all parameter files in it. A list file has one parameter file per line,
optionally followed by scene changes separated by '|':
    6 AM - Torii temple.m3p | width = 1920 | height = 1080
Each file is rendered to <name>.png next to it (or in --output). Files that
fail are reported and the batch goes on.

OPTIONS:
        --list <FILE>        Read the files from a list file (may be repeated)
    -o, --output <DIR>       Output folder (default: next to each file)
        --format <F>         png (default), bmp, or m3p (convert to MB3D parameter files)
        --skip-existing      Keep images that exist already: continues an interrupted
                             batch, and lets several processes share a list
    -s, --set <KEY=VALUE>    Change a scene key in every file (may be repeated)
        --scale <F>          Size factor (e.g. 0.25 for previews)
        --aa <N>             Anti-aliasing: render N times larger and reduce
        --depth              Also write <name>_depth.png
        --dry-run            Only list what would be rendered
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps and background pictures
";

fn batch(argv: &[String]) -> Result<(), String> {
    use mb3d::batch::{BatchOptions, ItemResult};
    let mut opts = BatchOptions::default();
    let mut inputs: Vec<String> = Vec::new();
    let mut lists: Vec<String> = Vec::new();
    let (mut quiet, mut dry) = (false, false);
    let mut args = argv.iter();
    while let Some(a) = args.next() {
        let mut val = || args.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{BATCH_USAGE}");
                return Ok(());
            }
            "--list" => lists.push(val()?),
            "-o" | "--output" => opts.output_dir = Some(val()?.into()),
            "--format" => opts.format = mb3d::anim::OutputFormat::parse(&val()?)?,
            "--skip-existing" => opts.skip_existing = true,
            "-s" | "--set" => opts.overrides.push(val()?),
            "--scale" => opts.scale = Some(val()?.parse::<f64>().ok().filter(|f| *f > 0.0).ok_or("bad --scale value")?),
            "--aa" => opts.aa = val()?.parse::<usize>().map_err(|_| "bad --aa value".to_string())?.clamp(1, 8),
            "--depth" => opts.depth = true,
            "--dry-run" => dry = true,
            "-t" | "--threads" => opts.threads = val()?.parse::<usize>().map_err(|_| "bad --threads value".to_string())?,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{BATCH_USAGE}")),
            s => inputs.push(s.to_string()),
        }
    }
    let mut items = mb3d::batch::expand_inputs(&inputs)?;
    for l in &lists {
        let text = std::fs::read_to_string(l).map_err(|e| format!("{l}: {e}"))?;
        let base = std::path::Path::new(l).parent().unwrap_or(std::path::Path::new(""));
        items.extend(mb3d::batch::parse_list(&text, base).map_err(|e| format!("{l}: {e}"))?);
    }
    if items.is_empty() {
        return Err(format!("no parameter files given\n\n{BATCH_USAGE}"));
    }
    let outputs = mb3d::batch::output_paths(&items, &opts);
    if dry {
        let mut out = std::io::stdout().lock();
        for (it, o) in items.iter().zip(&outputs) {
            let extra = if it.overrides.is_empty() { String::new() } else { format!("  ({})", it.overrides.join(", ")) };
            let state = if o.exists() && opts.skip_existing { "  [exists, skipped]" } else { "" };
            if writeln!(out, "{} -> {}{extra}{state}", it.input.display(), o.display()).is_err() {
                break; // e.g. piped into head
            }
        }
        return Ok(());
    }
    let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    let t0 = Instant::now();
    let (mut done, mut skipped) = (0usize, 0usize);
    let mut failed: Vec<(String, String)> = Vec::new();
    for (n, (it, out)) in items.iter().zip(&outputs).enumerate() {
        let name = it.input.display().to_string();
        let label = format!("[{}/{}] {name}", n + 1, items.len());
        let progress = |d: usize, t: usize| {
            if !quiet && tty && (d % 8 == 0 || d == t) {
                eprint!("\r  {label} {:5.1}%", 100.0 * d as f64 / t as f64);
                let _ = std::io::stderr().flush();
            }
        };
        let cr = if tty { "\r" } else { "" };
        match mb3d::batch::render_item(it, out, &opts, &progress, &|| false) {
            ItemResult::Done { output, seconds, notes } => {
                done += 1;
                if !quiet {
                    eprintln!("{cr}  {label} -> {} ({:.1}s)          ", output.display(), seconds);
                    for w in notes {
                        eprintln!("      note: {w}");
                    }
                }
            }
            ItemResult::Skipped { output, why } => {
                skipped += 1;
                if !quiet {
                    let w = if why == mb3d::frames::Skip::Busy { "being rendered by another process" } else { "exists" };
                    eprintln!("  {label} skipped, {} {w}", output.display());
                }
            }
            ItemResult::Failed { error } => {
                if !quiet {
                    eprintln!("{cr}  {label} FAILED: {error}          ");
                }
                failed.push((name, error));
            }
        }
    }
    if !quiet {
        eprintln!(
            "batch done: {done} rendered, {skipped} skipped, {} failed, {}",
            failed.len(),
            fmt_dur(t0.elapsed().as_secs_f64())
        );
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} of {} files failed:\n{}",
            failed.len(),
            items.len(),
            failed.iter().map(|(n, e)| format!("  {n}: {e}")).collect::<Vec<_>>().join("\n")
        ))
    }
}

// ---------------------------------------------------------------------------
// mb3d voxel / mb3d mesh

/// "x, y, z" (or more values) as numbers.
fn parse_list_f64(s: &str, n: usize, what: &str) -> Result<Vec<f64>, String> {
    let v: Vec<f64> = s
        .split([',', ' '])
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().parse::<f64>().map_err(|_| format!("{what}: bad number '{t}'")))
        .collect::<Result<_, _>>()?;
    if v.len() != n {
        return Err(format!("{what}: expected {n} numbers, got {}", v.len()));
    }
    Ok(v)
}

/// Loads a scene for the exports and applies `-s` changes.
fn load_for_export(file: &str, overrides: &[String], quiet: bool) -> Result<Scene, String> {
    let (s, notes) = mb3d::animfile::load_scene(std::path::Path::new(file))?;
    if !quiet {
        for n in notes {
            eprintln!("  note: {n}");
        }
    }
    if overrides.is_empty() {
        Ok(s)
    } else {
        s.apply(&overrides.join("\n"))
    }
}

const VOXEL_USAGE: &str = "\
mb3d voxel - export the object as a stack of slice images (MB3D's voxel export)

USAGE:
    mb3d voxel FILE [OPTIONS]

FILE is a parameter file or scene, or a MB3D voxel project (.m3v) that brings
its own settings. Each slice is a 1 bit PNG (white = object), named
<name><index>.png. The stack covers 2.2 / zoom scene units around the scene
middle in the view direction (like MB3D).

OPTIONS:
    -o, --output <DIR>       Output folder (default: voxels)
        --name <NAME>        File name prefix (default: name of FILE)
        --slices <N>         Number of slices = resolution in z (default 100)
        --scale <X,Y,Z>      Box proportions (default 1,1,1; z = 1 is 2.2 / zoom)
        --offset <X,Y,Z>     Move the box (scene units)
        --axes               Slice along the scene axes instead of the view
        --iterations         Object = iterations reach the maximum (default: DE)
        --max-its <N>        Maximum iterations (default: the scene's)
        --de <F>             DE threshold in voxels (default: from the DE stop)
        --min-its <N>        In-and-outside rendering: inner limits
        --min-de <F>
        --white-outside      White background, black object
        --no-leading-zeros   name1.png instead of name000001.png
        --preview <FILE>     Only write a small preview image of the voxel object
        --save-m3v <FILE>    Save the settings as MB3D voxel project (no export
                             unless --render)
        --render             Export too when saving
    -s, --set <KEY=VALUE>    Change a scene key (may be repeated)
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps
";

fn voxel_cmd(argv: &[String]) -> Result<(), String> {
    use mb3d::voxel::{ObjectTest, VoxelParams};
    let mut file: Option<String> = None;
    let mut overrides: Vec<String> = Vec::new();
    let (mut out, mut name, mut preview, mut save): (Option<String>, Option<String>, Option<String>, Option<String>) = (None, None, None, None);
    let (mut quiet, mut render_too) = (false, false);
    let mut threads = 0usize;
    let mut changes: Vec<Box<dyn Fn(&mut VoxelParams)>> = Vec::new();
    let mut slices: Option<u32> = None;
    let mut args = argv.iter();
    while let Some(a) = args.next() {
        let mut val = || args.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{VOXEL_USAGE}");
                return Ok(());
            }
            "-o" | "--output" => out = Some(val()?),
            "--name" => name = Some(val()?),
            "--slices" => slices = Some(val()?.parse::<u32>().map_err(|_| "bad --slices value".to_string())?.clamp(2, 100_000)),
            "--scale" => {
                let v = parse_list_f64(&val()?, 3, "--scale")?;
                changes.push(Box::new(move |p| p.scale = [v[0].max(1e-6), v[1].max(1e-6), v[2].max(1e-6)]));
            }
            "--offset" => {
                let v = parse_list_f64(&val()?, 3, "--offset")?;
                changes.push(Box::new(move |p| p.offset = [v[0], v[1], v[2]]));
            }
            "--axes" => changes.push(Box::new(|p| p.default_orientation = true)),
            "--iterations" => changes.push(Box::new(|p| p.test = ObjectTest::Iterations)),
            "--max-its" => {
                let v: i32 = val()?.parse().map_err(|_| "bad --max-its value".to_string())?;
                changes.push(Box::new(move |p| p.max_its = v.max(1)));
            }
            "--min-its" => {
                let v: i32 = val()?.parse().map_err(|_| "bad --min-its value".to_string())?;
                changes.push(Box::new(move |p| p.min_its = v.max(0)));
            }
            "--de" => {
                let v: f64 = val()?.parse().map_err(|_| "bad --de value".to_string())?;
                changes.push(Box::new(move |p| {
                    p.de = v.max(1e-6);
                    p.min_de = p.min_de.max(p.de * 0.254);
                }));
            }
            "--min-de" => {
                let v: f64 = val()?.parse().map_err(|_| "bad --min-de value".to_string())?;
                changes.push(Box::new(move |p| p.min_de = v.max(p.de * 0.254)));
            }
            "--white-outside" => changes.push(Box::new(|p| p.white_outside = true)),
            "--no-leading-zeros" => changes.push(Box::new(|p| p.leading_zeros = false)),
            "--preview" => preview = Some(val()?),
            "--save-m3v" => save = Some(val()?),
            "--render" => render_too = true,
            "-s" | "--set" => overrides.push(val()?),
            "-t" | "--threads" => threads = val()?.parse().map_err(|_| "bad --threads value".to_string())?,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{VOXEL_USAGE}")),
            s => file = Some(s.to_string()),
        }
    }
    let file = file.ok_or_else(|| format!("no parameter file given\n\n{VOXEL_USAGE}"))?;
    let path = std::path::Path::new(&file);
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "voxel".into());
    let (mut vp, scene) = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3v")) {
        let data = std::fs::read(path).map_err(|e| format!("{file}: {e}"))?;
        let (mut vp, s, notes) = mb3d::voxel::read_m3v(&data)?;
        if !quiet {
            for n in notes {
                eprintln!("  note: {n}");
            }
        }
        let s = if overrides.is_empty() { s } else { s.apply(&overrides.join("\n"))? };
        if let Some(n) = slices {
            // keep the DE threshold in proportion to the resolution
            vp.de *= n as f64 / vp.slices as f64;
            vp.min_de *= n as f64 / vp.slices as f64;
            vp.slices = n;
        }
        (vp, s)
    } else {
        let s = load_for_export(&file, &overrides, quiet)?;
        (VoxelParams::from_scene(&s, slices.unwrap_or(100)), s)
    };
    for c in &changes {
        c(&mut vp);
    }
    let (w, h) = vp.size();
    if let Some(f) = &save {
        let mut v2 = vp.clone();
        if let Some(o) = &out {
            v2.output_folder = o.clone();
        }
        std::fs::write(f, mb3d::voxel::write_m3v(&v2, &scene)).map_err(|e| format!("{f}: {e}"))?;
        if !quiet {
            eprintln!("  wrote {f}");
        }
        if !render_too && preview.is_none() {
            return Ok(());
        }
    }
    if let Some(pf) = &preview {
        let (rgb, pw, ph) = mb3d::voxel::preview(&scene, &vp, 160, threads)?;
        mb3d::png::write_rgb(pf, pw, ph, &rgb).map_err(|e| format!("{pf}: {e}"))?;
        if !quiet {
            eprintln!("  wrote preview {pf} ({pw}x{ph})");
        }
        return Ok(());
    }
    let dir = std::path::PathBuf::from(out.unwrap_or_else(|| {
        let f = vp.output_folder.clone();
        if f.is_empty() || f.contains('\\') { "voxels".into() } else { f }
    }));
    let name = name.unwrap_or(stem);
    if !quiet {
        eprintln!(
            "voxel export: {} slices of {w}x{h} ({}, {}) to {}",
            vp.slices,
            if vp.test == ObjectTest::De { format!("DE < {:.3}", vp.de) } else { format!("{} iterations", vp.max_its) },
            if vp.default_orientation { "scene axes" } else { "view orientation" },
            vp.slice_file(&dir, &name, 1).display()
        );
    }
    let t0 = Instant::now();
    let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    let progress = |i: u32, n: u32| {
        if !quiet && tty {
            eprint!("\r  slice {i}/{n}");
            let _ = std::io::stderr().flush();
        }
    };
    let n = mb3d::voxel::export(&scene, &vp, &dir, &name, threads, &progress, &|| false)?;
    if !quiet {
        eprintln!("\r  {n} slices written in {}          ", fmt_dur(t0.elapsed().as_secs_f64()));
    }
    Ok(())
}

const MESH_USAGE: &str = "\
mb3d mesh - export the object as a triangle mesh (MB3D's BulbTracer2)

USAGE:
    mb3d mesh FILE -o OUTPUT [OPTIONS]

The distance estimate is sampled on a grid of N x N x N steps in a cube around
the scene middle (2.2 / (zoom * scale) scene units wide), and marching cubes
builds the surface where the DE is 1 / sharpness steps. OUTPUT is .obj or .ply
(as MB3D writes them, with normals and optional vertex colours) or binary .stl.
The mesh is centred and scaled to size 1.

OPTIONS:
    -o, --output <FILE>      Output mesh (.obj, .ply, .stl)
        --resolution <N>     Grid steps per side (default 128; time and size grow with N^3)
        --sharpness <F>      Surface sharpness (default 0.5; larger = closer to the
                             surface, more detail)
        --scale <F>          Size of the cube (default 0.5: twice the visible
                             2.2 / zoom; larger = smaller cube)
        --offset <X,Y,Z>     Move the cube (scene units)
        --rotate <X,Y,Z>     Turn the cube (degrees)
        --bounds <X0,X1,Y0,Y1,Z0,Z1>  Only this part of the cube (0..100 each)
        --close              Close the mesh at the bounds (for 3D printing)
        --colors             Vertex colours from the palette
        --smooth <N>         Taubin smoothing passes afterwards
    -s, --set <KEY=VALUE>    Change a scene key (may be repeated)
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps
";

fn mesh_cmd(argv: &[String]) -> Result<(), String> {
    let mut mp = mb3d::mesh::MeshParams::default();
    let mut file: Option<String> = None;
    let mut out: Option<String> = None;
    let mut overrides: Vec<String> = Vec::new();
    let mut quiet = false;
    let mut threads = 0usize;
    let mut args = argv.iter();
    while let Some(a) = args.next() {
        let mut val = || args.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        let num = |v: String, what: &str| v.trim().parse::<f64>().map_err(|_| format!("bad {what} value"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{MESH_USAGE}");
                return Ok(());
            }
            "-o" | "--output" => out = Some(val()?),
            "--resolution" => mp.resolution = num(val()?, "--resolution")?.clamp(4.0, 4096.0) as usize,
            "--sharpness" => mp.sharpness = num(val()?, "--sharpness")?,
            "--scale" => mp.scale = num(val()?, "--scale")?.max(1e-9),
            "--offset" => {
                let v = parse_list_f64(&val()?, 3, "--offset")?;
                mp.offset = [v[0], v[1], v[2]];
            }
            "--rotate" => {
                let v = parse_list_f64(&val()?, 3, "--rotate")?;
                mp.angles = [v[0], v[1], v[2]];
            }
            "--bounds" => {
                let v = parse_list_f64(&val()?, 6, "--bounds")?;
                mp.bounds = [[v[0], v[1]], [v[2], v[3]], [v[4], v[5]]];
            }
            "--close" => mp.close = true,
            "--colors" | "--colours" => mp.colors = true,
            "--smooth" => mp.smooth = num(val()?, "--smooth")?.clamp(0.0, 1000.0) as u32,
            "-s" | "--set" => overrides.push(val()?),
            "-t" | "--threads" => threads = val()?.parse().map_err(|_| "bad --threads value".to_string())?,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{MESH_USAGE}")),
            s => file = Some(s.to_string()),
        }
    }
    let file = file.ok_or_else(|| format!("no parameter file given\n\n{MESH_USAGE}"))?;
    let out = out.ok_or("no output file given (-o mesh.obj / .ply / .stl)")?;
    let ext = std::path::Path::new(&out).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !["obj", "ply", "stl"].contains(&ext.as_str()) {
        return Err(format!("{out}: use .obj, .ply or .stl"));
    }
    let scene = load_for_export(&file, &overrides, quiet)?;
    if !quiet {
        eprintln!("mesh export: {0}x{0}x{0} grid, sharpness {1}, scale {2}", mp.resolution, mp.sharpness, mp.scale);
        if let Some(d) = mb3d::mesh::de_stop_for(&scene, &mp) {
            eprintln!("  note: DE stop {} lowered to {d:.4} for this sharpness (inside points must stay inside)", scene.de_stop);
        }
    }
    let t0 = Instant::now();
    let tty = std::io::IsTerminal::is_terminal(&std::io::stderr());
    let progress = |i: usize, n: usize| {
        if !quiet && tty && (i % 4 == 0 || i == n) {
            eprint!("\r  {:5.1}%", 100.0 * i as f64 / n as f64);
            let _ = std::io::stderr().flush();
        }
    };
    let mesh = mb3d::mesh::trace(&scene, &mp, threads, &progress, &|| false)?;
    if mesh.faces.is_empty() {
        return Err("no surface found in the cube (check --scale, --offset and --sharpness)".into());
    }
    mesh.save(std::path::Path::new(&out))?;
    if !quiet {
        let (e, open, _) = mesh.edge_stats();
        eprintln!(
            "\r  {} vertices, {} triangles{} in {}, wrote {out}",
            mesh.vertices.len(),
            mesh.faces.len(),
            if open == 0 { ", closed".to_string() } else { format!(", {open} of {e} edges open") },
            fmt_dur(t0.elapsed().as_secs_f64())
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// mb3d mutagen

const MUTAGEN_USAGE: &str = "\
mb3d mutagen - random variations of a parameter set (MB3D's MutaGen)

USAGE:
    mb3d mutagen FILE [-o DIR] [OPTIONS]

Makes one generation: FILE is the parent (1), it gets two children (1.1, 1.2),
four grandchildren and eight great-grandchildren, each a mutation of its
parent (formulas added, replaced or removed, formula options, julia mode,
iteration counts). Each child is the best of up to 9 candidates that were
tried as tiny images. DIR gets <label>.m3p and <label>.png for every member,
sheet.png with the whole family and members.txt. Pick a member and run
mb3d mutagen on its .m3p for the next generation.

OPTIONS:
    -o, --output <DIR>       Output folder (default: mutagen)
        --size <W>           Preview width (default 160)
        --seed <N>           Random seed (default: from the clock)
        --formula-weight <F> Chance to change the formulas (default 0.75)
        --params-weight <F>  Chance to change formula options (default 1.0)
        --params-strength <F> How much (default 1.0)
        --julia-weight <F>   Chance to change julia mode (default 0.5)
        --julia-strength <F>
        --its-weight <F>     Chance to change iteration counts (default 0.5)
        --its-strength <F>
        --no-probing         Take the first mutation, no probe images
    -s, --set <KEY=VALUE>    Change a scene key of FILE first (may be repeated)
    -t, --threads <N>        Number of threads
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files (the formulas
                             that mutations can add come from here)
        --maps <DIR>         Directory with maps
";

fn mutagen_cmd(argv: &[String]) -> Result<(), String> {
    use mb3d::mutagen::{MutationConfig, Rng};
    let mut cfg = MutationConfig::default();
    let (mut file, mut out): (Option<String>, String) = (None, "mutagen".into());
    let mut overrides: Vec<String> = Vec::new();
    let (mut size, mut threads, mut quiet) = (160usize, 0usize, false);
    let mut seed: Option<u64> = None;
    let mut args = argv.iter();
    while let Some(a) = args.next() {
        let mut val = || args.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        let f = |v: String| v.trim().parse::<f64>().map_err(|_| format!("bad value for {a}"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{MUTAGEN_USAGE}");
                return Ok(());
            }
            "-o" | "--output" => out = val()?,
            "--size" => size = f(val()?)?.clamp(16.0, 1024.0) as usize,
            "--seed" => seed = Some(val()?.trim().parse().map_err(|_| "bad --seed value".to_string())?),
            "--formula-weight" => cfg.formula_weight = f(val()?)?,
            "--params-weight" => cfg.params_weight = f(val()?)?,
            "--params-strength" => cfg.params_strength = f(val()?)?,
            "--julia-weight" => cfg.julia_weight = f(val()?)?,
            "--julia-strength" => cfg.julia_strength = f(val()?)?,
            "--its-weight" => cfg.iterations_weight = f(val()?)?,
            "--its-strength" => cfg.iterations_strength = f(val()?)?,
            "--no-probing" => cfg.probing = false,
            "-s" | "--set" => overrides.push(val()?),
            "-t" | "--threads" => threads = val()?.parse().map_err(|_| "bad --threads value".to_string())?,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{MUTAGEN_USAGE}")),
            s => file = Some(s.to_string()),
        }
    }
    let file = file.ok_or_else(|| format!("no parameter file given\n\n{MUTAGEN_USAGE}"))?;
    let scene = load_for_export(&file, &overrides, quiet)?;
    let dir = std::path::PathBuf::from(&out);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{out}: {e}"))?;
    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
    });
    let mut rng = Rng::new(seed);
    if !quiet {
        eprintln!("mutagen: {} formulas to choose from, seed {seed}", mb3d::mutagen::formula_names().len());
    }
    let t0 = Instant::now();
    let ph = (size as f64 * scene.height as f64 / scene.width.max(1) as f64).round().max(2.0) as usize;
    let mut previews: Vec<Vec<u8>> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    let mut err: Option<String> = None;
    let mut on_member = |i: usize, m: &mb3d::mutagen::Member| {
        let label = mb3d::mutagen::TREE[i - 1].0;
        let p = dir.join(format!("{label}.m3p"));
        if let Err(e) = std::fs::write(&p, mb3d::m3p::write(&m.scene)) {
            err = Some(format!("{}: {e}", p.display()));
        }
        let img = mb3d::mutagen::preview(&m.scene, size, ph, false, threads)
            .map(|(rgb, w, h)| if (w, h) == (size, ph) { rgb } else { vec![0; size * ph * 3] })
            .unwrap_or_else(|_| vec![0; size * ph * 3]);
        let _ = mb3d::png::write_rgb(&dir.join(format!("{label}.png")).to_string_lossy(), size, ph, &img);
        previews.push(img);
        let cap = mb3d::mutagen::caption(&m.scene);
        lines.push(format!("{label:8} {cap}"));
        if !quiet {
            eprintln!("  {i:2}/15  {label:8} {cap}");
        }
    };
    mb3d::mutagen::generation(&cfg, &scene, &mut rng, threads, &mut on_member, &|| false)?;
    if let Some(e) = err {
        return Err(e);
    }
    let (sheet, sw, sh) = mb3d::mutagen::contact_sheet(&previews, size, ph);
    mb3d::png::write_rgb(&dir.join("sheet.png").to_string_lossy(), sw, sh, &sheet).map_err(|e| format!("sheet.png: {e}"))?;
    std::fs::write(dir.join("members.txt"), format!("# mutagen of {file}, seed {seed}\n{}\n", lines.join("\n")))
        .map_err(|e| format!("members.txt: {e}"))?;
    if !quiet {
        eprintln!("done in {}: {}/sheet.png, <label>.m3p for each member", fmt_dur(t0.elapsed().as_secs_f64()), out);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// mb3d montecarlo

const MC_USAGE: &str = "\
mb3d montecarlo - path traced rendering (MB3D's Monte Carlo renderer)

USAGE:
    mb3d montecarlo FILE [-o image.png] [OPTIONS]

FILE is a parameter file or scene, or a Monte Carlo file (.m3c) to continue.
The image is refined in passes: the first pass shoots 4 rays per pixel, each
further pass adds rays where the image is still noisy. Rendering stops when
the average ray count, the pass count or the time limit is reached; the
image is written after every pass. The MC settings are scene keys
(mc_depth, mc_reflections, mc_transparency, ... see README) and can be
changed with --set.

OPTIONS:
    -o, --output <FILE>      Output PNG (default: <name>.png)
        --rays <N>           Stop at N rays per pixel on average (default 64)
        --passes <N>         Stop after N passes
        --time <SECONDS>     Stop after this time
        --m3c <FILE>         Save the state as MB3D Monte Carlo file after every
                             pass (continue later, also in MB3D)
        --exposure <N>       0..255 (default: the file's, 128 = 1)
        --saturation <N>     0..127 (32 = 1)
        --scale <F>          Scale the image size
    -s, --set <KEY=VALUE>    Change a scene key (may be repeated)
    -t, --threads <N>        Number of threads (default: all cores)
    -q, --quiet              No progress output
        --formulas <DIR>     Directory with .m3f formula files
        --maps <DIR>         Directory with maps
";

fn mc_cmd(argv: &[String]) -> Result<(), String> {
    let (mut file, mut out, mut m3c): (Option<String>, Option<String>, Option<String>) = (None, None, None);
    let mut overrides: Vec<String> = Vec::new();
    let (mut threads, mut quiet) = (0usize, false);
    let (mut rays, mut passes, mut time) = (None::<f64>, None::<u32>, None::<f64>);
    let (mut exposure, mut saturation, mut scale) = (None::<u8>, None::<u8>, None::<f64>);
    let mut args = argv.iter();
    while let Some(a) = args.next() {
        let mut val = || args.next().cloned().ok_or_else(|| format!("{a} needs a value"));
        let f = |v: String| v.trim().parse::<f64>().map_err(|_| format!("bad value for {a}"));
        match a.as_str() {
            "-h" | "--help" => {
                print!("{MC_USAGE}");
                return Ok(());
            }
            "-o" | "--output" => out = Some(val()?),
            "--rays" => rays = Some(f(val()?)?.max(1.0)),
            "--passes" => passes = Some(f(val()?)?.max(1.0) as u32),
            "--time" => time = Some(f(val()?)?.max(0.0)),
            "--m3c" => m3c = Some(val()?),
            "--exposure" => exposure = Some(f(val()?)?.clamp(0.0, 255.0) as u8),
            "--saturation" => saturation = Some(f(val()?)?.clamp(0.0, 127.0) as u8),
            "--scale" => scale = Some(f(val()?)?).filter(|v| *v > 0.0),
            "-s" | "--set" => overrides.push(val()?),
            "-t" | "--threads" => threads = val()?.parse().map_err(|_| "bad --threads value".to_string())?,
            "-q" | "--quiet" => quiet = true,
            "--formulas" => mb3d::formulas::add_formula_dir(val()?.into()),
            "--maps" => mb3d::maps::add_map_dir(val()?.into()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n\n{MC_USAGE}")),
            s => file = Some(s.to_string()),
        }
    }
    let file = file.ok_or_else(|| format!("no parameter file given\n\n{MC_USAGE}"))?;
    let path = std::path::Path::new(&file);
    let is_m3c = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("m3c"));
    let (mut scene, mut img) = if is_m3c {
        let data = std::fs::read(path).map_err(|e| format!("{file}: {e}"))?;
        let (f, img) = mb3d::mc::read_m3c(&data)?;
        if !quiet {
            for n in &f.warnings {
                eprintln!("  note: {n}");
            }
        }
        let sc = if overrides.is_empty() { f.scene } else { f.scene.apply(&overrides.join("\n"))? };
        if scale.is_some() {
            return Err("--scale cannot be used when continuing a .m3c file".into());
        }
        (sc, Some(img))
    } else {
        let mut sc = load_for_export(&file, &overrides, quiet)?;
        if let Some(f) = scale {
            sc.scale_image(f);
        }
        (sc, None)
    };
    if let Some(e) = exposure {
        scene.mc.contrast = e;
    }
    if let Some(s) = saturation {
        scene.mc.saturation = s;
    }
    let mut img = img.take().unwrap_or_else(|| mb3d::mc::McImage::new(scene.width as usize, scene.height as usize));
    let out = out.unwrap_or_else(|| path.with_extension("png").file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "mc.png".into()));
    let target = if rays.is_none() && passes.is_none() && time.is_none() { Some(64.0) } else { rays };
    if !quiet {
        let m = &scene.mc;
        eprintln!(
            "monte carlo: {}x{}, ambient depth {}, reflections {}{}, until {}",
            scene.width,
            scene.height,
            m.depth,
            if m.reflections { format!("on (depth {})", m.reflection_depth) } else { "off".into() },
            if m.reflections && m.transparency { ", transparency" } else { "" },
            [
                target.map(|r| format!("{r} rays/pixel")),
                passes.map(|p| format!("{p} passes")),
                time.map(|t| format!("{t} s")),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" or ")
        );
    }
    let t0 = Instant::now();
    let start_passes = img.passes;
    let write_picture = |img: &mb3d::mc::McImage| -> Result<(), String> {
        let rgb = mb3d::mc::paint(img, &scene.mc, scene.lighting.gamma);
        mb3d::png::write_rgb(&out, img.width, img.height, &rgb).map_err(|e| format!("{out}: {e}"))
    };
    loop {
        let st = img.stats();
        if let Some(r) = target {
            if !st.zero_counts && st.avg_rays >= r {
                break;
            }
        }
        if passes.is_some_and(|p| img.passes - start_passes >= p) {
            break;
        }
        if time.is_some_and(|t| t0.elapsed().as_secs_f64() >= t) {
            break;
        }
        let progress = |d: usize, n: usize| {
            if !quiet && (d % 8 == 0 || d == n) {
                eprint!("\r  pass {:3}: {:5.1} %", img_passes_hint(), 100.0 * d as f64 / n as f64);
            }
        };
        PASS_HINT.store(img.passes + 1, std::sync::atomic::Ordering::Relaxed);
        let deadline = time.map(|t| t0 + std::time::Duration::from_secs_f64(t));
        let cancel = || deadline.is_some_and(|d| Instant::now() > d);
        match mb3d::mc::pass(&scene, &mut img, threads, &progress, &cancel) {
            Ok(()) => {}
            Err(e) if e == "cancelled" => {
                if !quiet {
                    eprintln!("\r  time limit reached during pass {}", img.passes + 1);
                }
            }
            Err(e) => return Err(e),
        }
        let st = img.stats();
        if !quiet && img.passes == start_passes + 1 {
            eprintln!("\r  calculated on: {}", mb3d::mc::last_status());
        }
        if !quiet {
            eprintln!(
                "\r  pass {:3}: {:6.1} rays/pixel (max {}), noise {:.4}, {}",
                img.passes,
                st.avg_rays,
                st.max_rays,
                st.avg_noise,
                fmt_dur(img.seconds)
            );
        }
        write_picture(&img)?;
        if let Some(m) = &m3c {
            std::fs::write(m, mb3d::mc::write_m3c(&scene, &img)).map_err(|e| format!("{m}: {e}"))?;
        }
        if time.is_some_and(|t| t0.elapsed().as_secs_f64() >= t) {
            break;
        }
    }
    // a continued .m3c that already had its rays: its picture (e.g. with
    // another exposure or saturation)
    if img.passes == start_passes {
        if img.stats().avg_rays == 0.0 {
            return Err("no rays calculated (nothing to write)".into());
        }
        write_picture(&img)?;
        if !quiet {
            eprintln!("wrote {out} (no new pass: the target was reached already)");
        }
        return Ok(());
    }
    if !quiet {
        eprintln!("wrote {out}{}", m3c.map(|m| format!(" and {m}")).unwrap_or_default());
    }
    Ok(())
}

static PASS_HINT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
fn img_passes_hint() -> u32 {
    PASS_HINT.load(std::sync::atomic::Ordering::Relaxed)
}
