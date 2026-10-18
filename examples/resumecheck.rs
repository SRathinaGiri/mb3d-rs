//! Checks that Calculate 3D continues a stopped calculation: the program
//! without windows calculates a picture, stops it half way, calculates it
//! again ("Continue?" answered with Yes) and compares the picture with one
//! calculated in one go.
//! With `--mc` the Monte Carlo window instead: started, stopped, the
//! program made again (as after a restart), started ("Continue?" Yes): the
//! render goes on from the passes it had.
//! `cargo run --release --example resumecheck -- [WIDTH HEIGHT] [--gpu] [--mc]`

const F: &str = "Mand3DForm";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let num = |k: usize, d: &str| args.get(k).filter(|a| !a.starts_with('-')).cloned().unwrap_or_else(|| d.into());
    let (w, h) = (num(0, "2400"), num(1, "1800"));
    let dir = std::env::temp_dir().join(format!("mb3d-resumecheck-{}", std::process::id()));
    std::env::set_var("MB3D_RESUME_DIR", &dir);
    // (a stopped calculation keeps its checkpoint however short it was)
    std::env::set_var("MB3D_RESUME_KEEP_SECS", "0");
    let (mut ui, mut app) = mb3d::app::build(&[]).unwrap().unwrap();
    #[cfg(feature = "gpu")]
    mb3d::gpu::set_enabled(args.iter().any(|a| a == "--gpu"));
    let step = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mb3d::vcl::App::idle(app, ui);
        app.process(ui);
    };
    let wait = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        for _ in 0..6000 {
            step(ui, app);
            if !app.eng.running() && !app.main.calculating {
                break;
            }
        }
    };
    wait(&mut ui, &mut app);
    ui.set_text(F, "Edit11", &w);
    ui.set_text(F, "Edit12", &h);
    if args.iter().any(|a| a == "--mc") {
        return mc_check(ui, app, &dir, &w, &h);
    }
    let image = |app: &mb3d::app::Mb3d| app.main.rgb.as_ref().map(|(rgb, w, h)| ((**rgb).clone(), *w, *h)).expect("no picture");
    let calc = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        ui.click(F, "Button2");
        app.process(ui);
    };

    // in one go
    let t = std::time::Instant::now();
    calc(&mut ui, &mut app);
    wait(&mut ui, &mut app);
    let whole = image(&app);
    let secs = t.elapsed().as_secs_f64();
    println!("in one go: {}x{} in {secs:.1} s", whole.1, whole.2);
    assert!(mb3d::checkpoint::list().is_empty(), "a finished calculation left its checkpoint");

    // stopped half way
    calc(&mut ui, &mut app);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < secs / 2.0 {
        step(&mut ui, &mut app);
    }
    ui.click(F, "Button2");
    app.process(&mut ui);
    wait(&mut ui, &mut app);
    let kept = mb3d::checkpoint::list();
    assert_eq!(kept.len(), 1, "the stopped calculation kept no checkpoint");
    println!("stopped: {}", kept[0].summary());

    // continued from the Unfinished renders window (its parameters loaded
    // back: they must find the same checkpoint)
    ui.show("UnfinishedFrm");
    app.process(&mut ui);
    ui.click("UnfinishedFrm", "Button1");
    app.process(&mut ui);
    let t = std::time::Instant::now();
    wait(&mut ui, &mut app);
    let listed = image(&app);
    println!("continued from the list in {:.1} s", t.elapsed().as_secs_f64());
    assert!(mb3d::checkpoint::list().is_empty(), "the calculation continued from the list left its checkpoint");
    assert!(whole.0 == listed.0, "the calculation continued from the list differs");

    // stopped again
    calc(&mut ui, &mut app);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < secs / 2.0 {
        step(&mut ui, &mut app);
    }
    ui.click(F, "Button2");
    app.process(&mut ui);
    wait(&mut ui, &mut app);
    assert_eq!(mb3d::checkpoint::list().len(), 1);

    // again: the question, answered with Yes
    calc(&mut ui, &mut app);
    assert!(app.main.resume_ask.is_some(), "no question whether to continue");
    ui.click("MessageBoxDlg", "Btn0");
    app.process(&mut ui);
    let t = std::time::Instant::now();
    wait(&mut ui, &mut app);
    let again = image(&app);
    println!("continued in {:.1} s", t.elapsed().as_secs_f64());
    assert!(mb3d::checkpoint::list().is_empty(), "the continued calculation left its checkpoint");
    let diff = whole.0.iter().zip(&again.0).filter(|(a, b)| a != b).count();
    let _ = std::fs::remove_dir_all(&dir);
    if diff == 0 {
        println!("identical");
    } else {
        println!("{diff} of {} bytes differ", whole.0.len());
        std::process::exit(1);
    }
}

/// The Monte Carlo window's part (see the module comment).
fn mc_check(mut ui: mb3d::vcl::Ui, mut app: mb3d::app::Mb3d, dir: &std::path::Path, w: &str, h: &str) {
    const M: &str = "MCForm";
    let step = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d, secs: f64| {
        let t = std::time::Instant::now();
        while t.elapsed().as_secs_f64() < secs {
            std::thread::sleep(std::time::Duration::from_millis(100));
            mb3d::vcl::App::idle(app, ui);
            app.process(ui);
        }
    };
    let passes = || {
        let l = mb3d::checkpoint::list();
        assert_eq!(l.len(), 1, "one kept render expected");
        l[0].info.get("passes").and_then(|v| v.parse::<u32>().ok()).unwrap_or(0)
    };
    ui.show(M);
    app.process(&mut ui);
    ui.click(M, "Button3");
    app.process(&mut ui);
    ui.click(M, "Button2");
    app.process(&mut ui);
    step(&mut ui, &mut app, 8.0);
    ui.click(M, "Button2");
    app.process(&mut ui);
    step(&mut ui, &mut app, 2.0);
    let before = passes();
    println!("stopped after {before} passes");
    assert!(before > 0);
    // the program again: an empty image, Start asks
    drop((ui, app));
    let gpu = mb3d::gpu::enabled();
    let (mut ui, mut app) = mb3d::app::build(&[]).unwrap().unwrap();
    // (as before: the checkpoint is of the card's or the processor's render)
    mb3d::gpu::set_enabled(gpu);
    step(&mut ui, &mut app, 1.0);
    ui.set_text(F, "Edit11", w);
    ui.set_text(F, "Edit12", h);
    ui.show(M);
    app.process(&mut ui);
    ui.click(M, "Button3");
    app.process(&mut ui);
    ui.click(M, "Button2");
    app.process(&mut ui);
    assert!(ui.showing("MessageBoxDlg"), "no question whether to continue");
    ui.click("MessageBoxDlg", "Btn0");
    app.process(&mut ui);
    step(&mut ui, &mut app, 6.0);
    ui.click(M, "Button2");
    app.process(&mut ui);
    step(&mut ui, &mut app, 2.0);
    let after = passes();
    println!("continued: {after} passes");
    let _ = std::fs::remove_dir_all(dir);
    if after > before {
        println!("continued from the kept passes");
    } else {
        println!("did not continue");
        std::process::exit(1);
    }
}
