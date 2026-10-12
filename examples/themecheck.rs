//! Paints every window of the program in the themes Glossy and Windows
//! into OUTDIR (STYLE_FORM.png), for comparing two builds pixel by pixel.
//! `cargo run --release --example themecheck -- OUTDIR`
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    std::fs::create_dir_all(&out).unwrap();
    let (mut ui, mut app) = mb3d::app::build(&[]).unwrap().unwrap();
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        mb3d::vcl::App::idle(&mut app, &mut ui);
        app.process(&mut ui);
    }
    for st in ["Glossy", "Windows"] {
        ui.set_style(mb3d::vcl::theme::Style::from_name(st).unwrap());
        app.process(&mut ui);
        let names: Vec<String> = ui.forms.iter().map(|f| f.name.clone()).collect();
        for n in names {
            ui.save_form_png(&n, &out.join(format!("{st}_{n}.png")), 1.0).unwrap();
        }
    }
}
