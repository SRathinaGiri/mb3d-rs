//! Paints the program's windows off screen into PNG files (for checking
//! layouts without clicking through the windows).
//! `cargo run --release --example snapform -- [APP ARGS] -- STEPS...` with
//! the steps `show:FORM`, `page:FORM:PAGECONTROL:SHEET`, `click:FORM:CONTROL`,
//! `snap:FORM:FILE.png[:SCALE]`, `wait:SECONDS` (the program's idle work, e.g.
//! a running calculation; `lagwait:SECONDS` also prints the longest
//! blocking of the window), `print:FORM:CONTROL` (its caption) and `led`
//! (the title bar LED of the main window).
use mb3d::vcl::canvas::Canvas;
use mb3d::vcl::App;

fn main() {
    let all: Vec<String> = std::env::args().skip(1).collect();
    let split = all.iter().position(|a| a == "--").unwrap_or(all.len());
    let (app_args, steps) = (&all[..split], all.get(split + 1..).unwrap_or(&[]));
    let (mut ui, mut app) = mb3d::app::build(app_args).unwrap().unwrap();
    for s in steps {
        let p: Vec<&str> = s.split(':').collect();
        match p[0] {
            "show" => ui.show(p[1]),
            "page" => ui.set_active_page(p[1], p[2], p[3]),
            "click" => ui.click(p[1], p[2]),
            "wait" => {
                let t = std::time::Instant::now();
                while t.elapsed().as_secs_f64() < p[1].parse().unwrap() {
                    app.idle(&mut ui);
                    app.process(&mut ui);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            // as wait, and prints how long the program's idle work blocked
            // the window at most (what a user feels as a frozen window)
            "lagwait" => {
                let t = std::time::Instant::now();
                let (mut worst, mut n, mut over) = (0f64, 0, 0);
                while t.elapsed().as_secs_f64() < p[1].parse().unwrap() {
                    let t1 = std::time::Instant::now();
                    app.idle(&mut ui);
                    app.process(&mut ui);
                    let d = t1.elapsed().as_secs_f64();
                    worst = worst.max(d);
                    n += 1;
                    over += (d > 0.1) as usize;
                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
                println!("idle work: {n} turns, longest {:.0} ms, {over} longer than 100 ms", worst * 1000.0);
            }
            "print" => println!("{}.{} = {:?}", p[1], p[2], ui.caption(p[1], p[2])),
            "status" => {
                let sc = &app.scene;
                println!("gpu status {:?}; normals on zbuf {}, shadows {}, ao {:?}, deao {:?}", mb3d::gpu::last_status(), sc.normals_on_zbuf, sc.shadows.is_some(), sc.ao, sc.deao);
            }
            "led" => {
                let f = ui.fm(mb3d::app::MAIN);
                println!("led {:?} blinking {}", f.caption_led.map(|c| format!("{c:06x}")), f.led_blink);
            }
            "snap" => {
                let scale: f32 = p.get(3).and_then(|v| v.parse().ok()).unwrap_or(2.0);
                let theme = ui.theme;
                let f = ui.fm(p[1]);
                f.layout();
                let b = f.frame_w();
                let (cw, ch) = f.client_size();
                let (lw, lh) = (cw + 2 * b, ch + 2 * b + f.caption_h(&theme));
                let (pw, ph) = ((lw as f32 * scale) as usize, (lh as f32 * scale) as usize);
                let mut buf = vec![0u32; pw * ph];
                let mut cv = Canvas::new(&mut buf, pw, ph, scale);
                f.paint(&mut cv, &theme, lw, lh);
                let rgb: Vec<u8> = buf.iter().flat_map(|c| [(c >> 16) as u8, (c >> 8) as u8, *c as u8]).collect();
                std::fs::write(p[2], mb3d::png::encode_rgb(pw, ph, &rgb)).unwrap();
            }
            _ => panic!("unknown step {s}"),
        }
        app.process(&mut ui);
    }
}
