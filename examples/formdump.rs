//! Renders MB3D's forms to PNG files (checking the layouts):
//! `cargo run --example formdump -- OUTDIR [glossy|windows] [scale]`

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| "forms".into()));
    let style = mb3d::vcl::Style::from_name(args.get(1).map(String::as_str).unwrap_or("glossy")).unwrap_or_default();
    let scale: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    std::fs::create_dir_all(&out).unwrap();
    let mut ui = mb3d::vcl::Ui::new(style);
    for (name, text) in mb3d::app::forms::ALL {
        match ui.add_form(text) {
            Ok(i) => {
                let fname = ui.forms[i].name.clone();
                let p = out.join(format!("{name}.png"));
                ui.save_form_png(&fname, &p, scale).unwrap();
                println!("{}", p.display());
            }
            Err(e) => eprintln!("{name}: {e}"),
        }
    }
}
