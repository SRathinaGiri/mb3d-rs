//! Runs the application without windows: starts it, waits for the first
//! calculation and writes the main windows as PNG files.
//! `cargo run --example apptest -- OUTDIR [file.m3p]`

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| "apptest".into()));
    std::fs::create_dir_all(&out).unwrap();
    let (mut ui, mut app) = mb3d::app::build(&args[1.min(args.len())..]).unwrap().unwrap();
    let wait = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        for _ in 0..600 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
            if !app.eng.running() && !app.main.calculating {
                break;
            }
        }
    };
    wait(&mut ui, &mut app);
    ui.save_form_png("Mand3DForm", &out.join("main_2d.png"), 1.0).unwrap();
    // Calculate 3D
    ui.click("Mand3DForm", "Button2");
    app.process(&mut ui);
    wait(&mut ui, &mut app);
    #[cfg(feature = "gpu")]
    eprintln!("calculate 3D: {}", mb3d::gpu::last_status());
    for f in ["Mand3DForm", "FormulaGUIForm", "LightAdjustForm"] {
        ui.save_form_png(f, &out.join(format!("{f}.png")), 1.0).unwrap();
    }
    let only = std::env::var("APPTEST_ONLY").unwrap_or_default();
    let want = |n: &str| only.is_empty() || only.split(',').any(|s| s == n);
    // Navigator
    if want("navi") {
        ui.show("FNavigator");
        app.process(&mut ui);
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(&mut app, &mut ui);
            app.process(&mut ui);
        }
        ui.save_form_png("FNavigator", &out.join("FNavigator.png"), 1.0).unwrap();
        // the adjustment panel
        ui.click("FNavigator", "SpeedButton23");
        app.process(&mut ui);
        ui.save_form_png("FNavigator", &out.join("FNavigator_panel.png"), 1.0).unwrap();
        ui.click("FNavigator", "SpeedButton23");
        app.process(&mut ui);
    }
    // Monte Carlo: import, one pass or two, stop
    if want("mc") {
    ui.show("MCForm");
    ui.click("MCForm", "Button3");
    app.process(&mut ui);
    ui.click("MCForm", "Button2");
    app.process(&mut ui);
    // the lines appear while the first pass runs
    for _ in 0..15 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mb3d::vcl::App::idle(&mut app, &mut ui);
        app.process(&mut ui);
    }
    ui.save_form_png("MCForm", &out.join("MCForm_first_pass.png"), 1.0).unwrap();
    for _ in 0..300 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mb3d::vcl::App::idle(&mut app, &mut ui);
        app.process(&mut ui);
        if ui.caption("MCForm", "Label13").parse::<u32>().unwrap_or(0) >= 8 {
            break;
        }
    }
    ui.click("MCForm", "Button2");
    app.process(&mut ui);
    ui.save_form_png("MCForm", &out.join("MCForm.png"), 1.0).unwrap();
    }
    // Big renders
    ui.show("TilingForm");
    ui.click("TilingForm", "SpeedButton2");
    app.process(&mut ui);
    ui.save_form_png("TilingForm", &out.join("TilingForm.png"), 1.0).unwrap();
    ui.show("BatchForm1");
    app.process(&mut ui);
    ui.save_form_png("BatchForm1", &out.join("BatchForm1.png"), 1.0).unwrap();
    // Mesh export: import, preview, a small mesh
    if want("btracer") {
        ui.show("BulbTracer2Frm");
        app.process(&mut ui);
        ui.click("BulbTracer2Frm", "ImportParamsFromMainBtn");
        app.process(&mut ui);
        let idle_until = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, f: &dyn Fn(&mb3d::vcl::Ui) -> bool| {
            for _ in 0..1200 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                mb3d::vcl::App::idle(app, ui);
                app.process(ui);
                if f(ui) {
                    break;
                }
            }
        };
        idle_until(&mut ui, &mut app, &|ui| ui.caption("BulbTracer2Frm", "RefreshPreviewBtn") != "Stop");
        ui.set_text("BulbTracer2Frm", "MeshVResolutionEdit", "48");
        let mesh = out.join("mesh.obj");
        ui.set_item_index("BulbTracer2Frm", "SaveTypeCmb", 1);
        ui.set_text("BulbTracer2Frm", "FilenameREd", &mesh.display().to_string());
        ui.click("BulbTracer2Frm", "CalculateBtn");
        app.process(&mut ui);
        idle_until(&mut ui, &mut app, &|ui| ui.caption("BulbTracer2Frm", "Label13").starts_with("Elapsed"));
        ui.save_form_png("BulbTracer2Frm", &out.join("BulbTracer2Frm.png"), 1.0).unwrap();
        println!("mesh: {} bytes", std::fs::metadata(&mesh).map(|m| m.len()).unwrap_or(0));
        // the mesh preview (shown after the export) and the height map generator
        mb3d::vcl::App::idle(&mut app, &mut ui);
        app.process(&mut ui);
        mb3d::vcl::App::idle(&mut app, &mut ui);
        if ui.showing("MeshPreviewFrm") {
            ui.save_form_png("MeshPreviewFrm", &out.join("MeshPreviewFrm.png"), 1.0).unwrap();
        }
        ui.show("HeightMapGenFrm");
        app.process(&mut ui);
        mb3d::app::meshview::heightmap_dialog(&mut app, &mut ui, "load", &mb3d::vcl::DialogResult::File(Some(mesh.clone())));
        mb3d::vcl::App::idle(&mut app, &mut ui);
        app.process(&mut ui);
        mb3d::vcl::App::idle(&mut app, &mut ui);
        ui.save_form_png("HeightMapGenFrm", &out.join("HeightMapGenFrm.png"), 1.0).unwrap();
        mb3d::app::meshview::heightmap_dialog(&mut app, &mut ui, "save", &mb3d::vcl::DialogResult::File(Some(out.join("hm.png"))));
    }
    // MutaGen: one generation from the main parameters
    if want("mutagen") {
        ui.show("MutaGenFrm");
        app.process(&mut ui);
        mb3d::vcl::App::idle(&mut app, &mut ui);
        ui.click("MutaGenFrm", "MutateBtn");
        app.process(&mut ui);
        for _ in 0..3000 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(&mut app, &mut ui);
            app.process(&mut ui);
            if ui.caption("MutaGenFrm", "MutateBtn") == "Mutate!" {
                break;
            }
        }
        ui.save_form_png("MutaGenFrm", &out.join("MutaGenFrm.png"), 1.0).unwrap();
    }
    // the experimental speed-ups window, switched on
    if want("experimental") {
        ui.show("ExperimentalFrm");
        app.process(&mut ui);
        if !ui.checked("ExperimentalFrm", "OnBox") {
            ui.click("ExperimentalFrm", "OnBox");
            app.process(&mut ui);
        }
        ui.save_form_png("ExperimentalFrm", &out.join("ExperimentalFrm.png"), 1.0).unwrap();
    }
    // Voxel export: import and preview
    if want("voxel") {
    ui.show("FVoxelExport");
    app.process(&mut ui);
    ui.click("FVoxelExport", "Button4");
    app.process(&mut ui);
    for _ in 0..600 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mb3d::vcl::App::idle(&mut app, &mut ui);
        app.process(&mut ui);
        if ui.caption("FVoxelExport", "Button5") != "Stop" {
            break;
        }
    }
    ui.save_form_png("FVoxelExport", &out.join("FVoxelExport.png"), 1.0).unwrap();
    }
    println!("{}", ui.text("Mand3DForm", "Memo1"));
}
