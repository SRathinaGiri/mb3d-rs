//! The main and Lighting windows in every theme, after a calculation:
//! `themeshots OUTDIR [--formulas DIR --maps DIR] file.m3p` writes
//! THEME_main.png and THEME_light.png.
const F: &str = "Mand3DForm";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(&args[0]);
    std::fs::create_dir_all(&out).unwrap();
    let (mut ui, mut app) = mb3d::app::build(&args[1..]).unwrap().unwrap();
    let wait = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        for _ in 0..1200 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
            if !app.eng.running() && !app.main.calculating {
                break;
            }
        }
    };
    wait(&mut ui, &mut app);
    mb3d::gpu::set_enabled(false);
    ui.click(F, "Button2");
    app.process(&mut ui);
    wait(&mut ui, &mut app);
    ui.show("LightAdjustForm");
    app.process(&mut ui);
    for st in mb3d::vcl::theme::Style::all() {
        ui.set_style(st);
        app.process(&mut ui);
        let n = st.name().replace(' ', "");
        ui.save_form_png(F, &out.join(format!("{n}_main.png")), 1.0).unwrap();
        ui.save_form_png("LightAdjustForm", &out.join(format!("{n}_light.png")), 1.0).unwrap();
    }
}
