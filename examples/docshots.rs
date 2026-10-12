//! The screenshots of docs/RENDERING.md, without windows:
//! `docshots OUTDIR [--formulas DIR --maps DIR] file.m3p` writes main_card.png (calculated on the
//! graphics card), main_cpu.png ("Calculate on: CPU"), both with the LED's
//! hint and the Internal page, and themes_100.png / themes_150.png (the
//! Visual themes window at two interface sizes).

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
    // the main window with the LED's hint as when the mouse is on it
    let shot = |ui: &mut mb3d::vcl::Ui, name: &str| {
        let path = out.join(name);
        ui.save_form_png(F, &path, 1.0).unwrap();
        let f = ui.fm(F);
        let r = f.led_rect.get();
        f.hint = Some((f.led_hint.clone(), r.x - f.frame_w(), 2));
        f.led_hint_shown = true;
        ui.save_form_png(F, &path, 1.0).unwrap();
        let f = ui.fm(F);
        f.hint = None;
        f.led_hint_shown = false;
    };
    let calc = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        ui.click(F, "Button2");
        app.process(ui);
        wait(ui, app);
    };
    wait(&mut ui, &mut app);
    ui.set_active_page(F, "PageControl1", "TabSheet4");
    // until the card's shaders are compiled (Auto calculates on the CPU
    // meanwhile)
    for _ in 0..20 {
        calc(&mut ui, &mut app);
        if mb3d::gpu::last_status().starts_with("GPU") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
    shot(&mut ui, "main_card.png");
    ui.set_item_index(F, "CalcOnCombo", 1);
    mb3d::gpu::set_enabled(false);
    calc(&mut ui, &mut app);
    shot(&mut ui, "main_cpu.png");
    ui.show("VisualThemesFrm");
    app.process(&mut ui);
    for (i, pct) in [(1, 100), (3, 150)] {
        ui.set_item_index("VisualThemesFrm", "UIScaleCmb", i);
        ui.save_form_png("VisualThemesFrm", &out.join(format!("themes_{pct}.png")), pct as f32 / 100.0).unwrap();
    }
}
