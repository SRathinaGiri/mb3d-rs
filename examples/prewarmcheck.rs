//! The card's shaders compiled ahead: opens a parameter file in the
//! application (without windows), waits WAIT seconds (default 0), clicks
//! Calculate 3D and prints where the calculation ran and how long it took.
//! `cargo run --release --example prewarmcheck -- file.m3p`
//! (__GL_SHADER_DISK_CACHE=0 keeps NVIDIA's shader cache out of it)
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut ui, mut app) = mb3d::app::build(&args).unwrap().unwrap();
    let step = |ui: &mut mb3d::vcl::Ui, app: &mut mb3d::app::Mb3d| {
        std::thread::sleep(std::time::Duration::from_millis(100));
        mb3d::vcl::App::idle(app, ui);
        app.process(ui);
    };
    // (the opening 2D preview first)
    for _ in 0..600 {
        step(&mut ui, &mut app);
        if !app.eng.running() && !app.main.calculating {
            break;
        }
    }
    let wait: f64 = std::env::var("WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < wait {
        step(&mut ui, &mut app);
    }
    let t = std::time::Instant::now();
    ui.click("Mand3DForm", "Button2");
    app.process(&mut ui);
    for _ in 0..6000 {
        step(&mut ui, &mut app);
        if !app.eng.running() && !app.main.calculating {
            break;
        }
    }
    #[cfg(feature = "gpu")]
    println!("waited {wait} s, then calculated in {:.1} s: {}", t.elapsed().as_secs_f64(), mb3d::gpu::last_status());
}
