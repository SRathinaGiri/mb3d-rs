//! Paints the program's windows off screen into PNG files (for checking
//! layouts without clicking through the windows).
//! `cargo run --release --example snapform -- [APP ARGS] -- STEPS...` with
//! the steps `show:FORM`, `page:FORM:PAGECONTROL:SHEET`, `click:FORM:CONTROL`,
//! `text:FORM:CONTROL:TEXT`, `check:FORM:CONTROL:0|1`, `item:FORM:CONTROL:INDEX`, `pos:FORM:UPDOWN:N`,
//! `items:FORM:CONTROL:A|B|...` (a list's lines), `drop:FORM:COMBOBOX` (its list opened),
//! `post:FORM:CONTROL:HANDLER` (an event as from the control, e.g. an OnMouseDown),
//! `file:TAG:PATH` (a file dialog answered with that file, e.g. `file:tiling:import:x.m3p`),
//! `key:FORM:CONTROL:HANDLER:KEYCODE` (a key down event), `hit:FORM:X:Y` (the control there),
//! `mouse:FORM:X:Y[:right]` (a click at client coordinates, as from the mouse),
//! `drag:FORM:X1:Y1:X2:Y2:STEPS` (a left drag), `wheel:FORM:X:Y:DELTA`,
//! `snap:FORM:FILE.png[:SCALE]`, `wait:SECONDS` (the program's idle work, e.g.
//! a running calculation; `lagwait:SECONDS` also prints the longest
//! blocking of the window), `print:FORM:CONTROL` (its caption),
//! `printtext:FORM:CONTROL` (its text), `rect:FORM:CONTROL` (where it is),
//! `formpos:FORM` (the window's position and size) and `led`
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
            "text" => ui.set_text(p[1], p[2], &p[3..].join(":")),
            "check" => ui.set_checked(p[1], p[2], p[3] == "1"),
            "item" => ui.set_item_index(p[1], p[2], p[3].parse().unwrap()),
            "pos" => ui.set_position(p[1], p[2], p[3].parse().unwrap()),
            "post" => ui.post(p[1], p[2], p[3], mb3d::vcl::Ev::Click),
            // a file dialog's answer: `file:TAG:PATH` (the tag as the program opened it, e.g. tiling:import)
            "file" => ui.post("", &p[1..3].join(":"), "", mb3d::vcl::Ev::DialogResult(mb3d::vcl::DialogResult::File(Some(std::path::PathBuf::from(p[3..].join(":")))))),
            "hit" => {
                let fi = ui.form_index(p[1]).unwrap();
                let f = &ui.forms[fi];
                let h = f.hit(p[2].parse().unwrap(), p[3].parse().unwrap());
                println!("hit {}:{}:{} = {:?}", p[1], p[2], p[3], h.map(|i| f.ctl[i].name.clone()));
            }
            "key" => ui.post(p[1], p[2], p[3], mb3d::vcl::Ev::KeyDown { key: p[4].parse().unwrap(), shift: 0 }),
            "mouse" => {
                let fi = ui.form_index(p[1]).unwrap();
                let (x, y): (i32, i32) = (p[2].parse().unwrap(), p[3].parse().unwrap());
                let mut raws = ui.forms[fi].mouse_move(x, y, 0);
                let b = if p.get(4) == Some(&"right") { mb3d::vcl::MouseButton::Right } else { mb3d::vcl::MouseButton::Left };
                raws.extend(ui.forms[fi].mouse_down(x, y, b, 0));
                raws.extend(ui.forms[fi].mouse_up(x, y, b, 0));
                if std::env::var_os("SNAPFORM_EVENTS").is_some() {
                    let f = &ui.forms[fi];
                    for r in &raws {
                        let c = &f.ctl[r.id];
                        println!("raw {}.{} -> {:?}", c.name, r.name, c.events.get(r.name));
                    }
                }
                ui.dispatch(fi, raws);
            }
            "wheel" => {
                let fi = ui.form_index(p[1]).unwrap();
                let (x, y, d): (i32, i32, i32) = (p[2].parse().unwrap(), p[3].parse().unwrap(), p[4].parse().unwrap());
                let raws = ui.forms[fi].wheel(d, x, y, 0);
                ui.dispatch(fi, raws);
            }
            // a left drag from X1:Y1 to X2:Y2 in STEPS mouse moves
            "drag" => {
                let fi = ui.form_index(p[1]).unwrap();
                let v: Vec<i32> = p[2..7].iter().map(|v| v.parse().unwrap()).collect();
                let mut raws = ui.forms[fi].mouse_move(v[0], v[1], 0);
                raws.extend(ui.forms[fi].mouse_down(v[0], v[1], mb3d::vcl::MouseButton::Left, 0));
                ui.dispatch(fi, raws);
                for k in 1..=v[4] {
                    let (x, y) = (v[0] + (v[2] - v[0]) * k / v[4], v[1] + (v[3] - v[1]) * k / v[4]);
                    let raws = ui.forms[fi].mouse_move(x, y, mb3d::vcl::form::SS_LEFT);
                    ui.dispatch(fi, raws);
                }
                let raws = ui.forms[fi].mouse_up(v[2], v[3], mb3d::vcl::MouseButton::Left, 0);
                ui.dispatch(fi, raws);
            }
            "items" => ui.set_items(p[1], p[2], p[3..].join(":").split('|').filter(|s| !s.is_empty()).map(String::from).collect()),
            "drop" => {
                let f = ui.fm(p[1]);
                let id = f.id(p[2]).unwrap();
                f.popup = Some(mb3d::vcl::form::Popup::Combo { id, hot: f.ctl[id].item_index, top: 0 });
            }
            "wait" => {
                let t = std::time::Instant::now();
                while t.elapsed().as_secs_f64() < p[1].parse().unwrap() {
                    app.idle(&mut ui);
                    app.process(&mut ui);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            // `memreset`: the peak memory starts again from now (Linux)
            "memreset" => {
                let _ = std::fs::write("/proc/self/clear_refs", "5");
            }
            // `mem`: the peak memory so far (Linux)
            "mem" => {
                let st = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
                for l in st.lines().filter(|l| l.starts_with("VmHWM") || l.starts_with("VmRSS")) {
                    println!("{l}");
                }
            }
            // `calcwait:MAXSECONDS`: as wait, until the calculation is done
            "calcwait" => {
                let t = std::time::Instant::now();
                std::thread::sleep(std::time::Duration::from_millis(500));
                while t.elapsed().as_secs_f64() < p[1].parse().unwrap() && app.eng.running() {
                    app.idle(&mut ui);
                    app.process(&mut ui);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                for _ in 0..10 {
                    app.idle(&mut ui);
                    app.process(&mut ui);
                }
                println!("calcwait: {:.1} s", t.elapsed().as_secs_f64());
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
            "printtext" => println!("{}.{} = {:?}", p[1], p[2], ui.text(p[1], p[2])),
            "formpos" => {
                let f = ui.f(p[1]);
                println!("{} at {}, {} size {:?}", p[1], f.left, f.top, f.outer_size(&ui.theme));
            }
            "rect" => {
                let f = ui.fm(p[1]);
                let id = f.id(p[2]).unwrap();
                println!("{}.{} at {:?}", p[1], p[2], f.abs_rect(id));
            }
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
