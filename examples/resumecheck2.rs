//! More of the program's resume paths without windows (see resumecheck.rs):
//! the question answered No and Cancel, Delete and Delete all in the
//! Unfinished renders window, the autosave keeping the list up to date, a
//! Monte Carlo render continued from the list after a restart, a voxel
//! export and a Bulb Tracer mesh stopped and continued (both compared with
//! one made in one go).
//! `cargo run --release --example resumecheck2 [-- only:NAME,...]`

use mb3d::app::Mb3d;
use mb3d::vcl::Ui;

const F: &str = "Mand3DForm";
const U: &str = "UnfinishedFrm";

fn step(ui: &mut Ui, app: &mut Mb3d) {
    std::thread::sleep(std::time::Duration::from_millis(100));
    mb3d::vcl::App::idle(app, ui);
    app.process(ui);
}

fn run_for(ui: &mut Ui, app: &mut Mb3d, secs: f64) {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < secs {
        step(ui, app);
    }
}

fn until(ui: &mut Ui, app: &mut Mb3d, secs: f64, f: &dyn Fn(&Ui, &Mb3d) -> bool) -> bool {
    let t = std::time::Instant::now();
    while t.elapsed().as_secs_f64() < secs {
        step(ui, app);
        if f(ui, app) {
            return true;
        }
    }
    false
}

fn calc_idle(_: &Ui, app: &Mb3d) -> bool {
    !app.eng.running() && !app.main.calculating
}

fn click(ui: &mut Ui, app: &mut Mb3d, form: &str, ctl: &str) {
    ui.click(form, ctl);
    app.process(ui);
}

fn build() -> (Ui, Mb3d) {
    let (mut ui, mut app) = mb3d::app::build(&[]).unwrap().unwrap();
    // (the processor: continued pictures must be identical)
    #[cfg(feature = "gpu")]
    mb3d::gpu::set_enabled(false);
    until(&mut ui, &mut app, 600.0, &calc_idle);
    (ui, app)
}

struct T {
    pass: usize,
    fail: usize,
}
impl T {
    fn check(&mut self, ok: bool, what: &str) {
        println!("{}  {what}", if ok { "PASS" } else { "FAIL" });
        if ok {
            self.pass += 1
        } else {
            self.fail += 1
        }
    }
}

fn files(d: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(d)
        .map(|r| r.flatten().filter(|e| e.path().is_file()).map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap_or_default())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

fn main() {
    let only: Vec<String> = std::env::args().skip(1).filter_map(|a| a.strip_prefix("only:").map(|s| s.split(',').map(String::from).collect::<Vec<_>>())).flatten().collect();
    let want = |n: &str| only.is_empty() || only.iter().any(|o| o == n);
    let dir = std::env::temp_dir().join(format!("mb3d-resumecheck2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("MB3D_RESUME_DIR", dir.join("ck"));
    std::env::set_var("MB3D_RESUME_KEEP_SECS", "0");
    std::env::set_var("MB3D_AUTOSAVE_SECS", "1");
    let mut t = T { pass: 0, fail: 0 };
    let n = || mb3d::checkpoint::list().len();
    let (mut ui, mut app) = build();
    let size = |ui: &mut Ui, w: &str, h: &str| {
        ui.set_text(F, "Edit11", w);
        ui.set_text(F, "Edit12", h);
    };

    if want("question") {
        size(&mut ui, "3000", "2200");
        // one go, for its time and picture
        let t0 = std::time::Instant::now();
        click(&mut ui, &mut app, F, "Button2");
        until(&mut ui, &mut app, 600.0, &calc_idle);
        let secs = t0.elapsed().as_secs_f64();
        let whole = app.main.rgb.as_ref().map(|(r, _, _)| (**r).clone());
        // stopped half way; while it runs the list is kept up to date
        click(&mut ui, &mut app, F, "Button2");
        run_for(&mut ui, &mut app, secs * 0.3);
        let a = mb3d::checkpoint::list();
        run_for(&mut ui, &mut app, (secs * 0.2).max(1.5));
        let b = mb3d::checkpoint::list();
        t.check(
            a.len() == 1 && b.len() == 1 && a[0].busy && b[0].progress() > a[0].progress() && b[0].age() <= 2,
            &format!("autosave: the running render is listed and kept up to date ({:.0} % → {:.0} %, saved {} s ago)", a.first().map_or(0.0, |e| e.progress() * 100.0), b.first().map_or(0.0, |e| e.progress() * 100.0), b.first().map_or(99, |e| e.age())),
        );
        click(&mut ui, &mut app, F, "Button2");
        until(&mut ui, &mut app, 60.0, &calc_idle);
        let kept = mb3d::checkpoint::list().first().map_or(0.0, |e| e.progress());
        t.check(kept > 0.0, &format!("stopped: {:.0} % kept", kept * 100.0));
        // Cancel: nothing happens, the kept render stays
        click(&mut ui, &mut app, F, "Button2");
        let asked = ui.showing("MessageBoxDlg");
        click(&mut ui, &mut app, "MessageBoxDlg", "Btn2");
        run_for(&mut ui, &mut app, 1.0);
        let still = mb3d::checkpoint::list().first().map_or(0.0, |e| e.progress());
        t.check(asked && calc_idle(&ui, &app) && n() == 1 && still == kept, "Cancel: nothing calculated, the kept render stays");
        // No: from the start (the kept rows are not shown first)
        click(&mut ui, &mut app, F, "Button2");
        click(&mut ui, &mut app, "MessageBoxDlg", "Btn1");
        run_for(&mut ui, &mut app, 0.5);
        let fresh = mb3d::checkpoint::list().first().map_or(1.0, |e| e.progress());
        until(&mut ui, &mut app, 600.0, &calc_idle);
        let again = app.main.rgb.as_ref().map(|(r, _, _)| (**r).clone());
        t.check(fresh < kept && n() == 0 && again == whole, &format!("No: calculated from the start ({:.0} % after 0.5 s), same picture, nothing left", fresh * 100.0));
    }

    if want("list") {
        for (w, h) in [("2800", "2000"), ("2600", "1900")] {
            size(&mut ui, w, h);
            click(&mut ui, &mut app, F, "Button2");
            run_for(&mut ui, &mut app, 2.0);
            click(&mut ui, &mut app, F, "Button2");
            until(&mut ui, &mut app, 60.0, &calc_idle);
        }
        t.check(n() == 2, &format!("two stopped renders kept ({})", n()));
        ui.show(U);
        app.process(&mut ui);
        let rows = ui.cm(U, "ListView1").cells.len();
        // (a click on a row selects it)
        ui.cm(U, "ListView1").item_index = 0;
        click(&mut ui, &mut app, U, "Button3");
        let rows1 = ui.cm(U, "ListView1").cells.len();
        println!("      (after Delete: {} kept, message box: {}, selected {})", n(), ui.showing("MessageBoxDlg"), ui.item_index(U, "ListView1"));
        t.check(rows == 2 && rows1 == 1 && n() == 1, &format!("Delete: {rows} rows, then {rows1}"));
        click(&mut ui, &mut app, U, "Button4");
        let asked = ui.showing("MessageBoxDlg");
        click(&mut ui, &mut app, "MessageBoxDlg", "Btn0");
        let rows2 = ui.cm(U, "ListView1").cells.len();
        t.check(asked && rows2 == 0 && n() == 0, "Delete all: asked, then empty");
        ui.hide(U);
        app.process(&mut ui);
    }

    if want("mclist") {
        size(&mut ui, "800", "600");
        ui.show("MCForm");
        app.process(&mut ui);
        click(&mut ui, &mut app, "MCForm", "Button3");
        click(&mut ui, &mut app, "MCForm", "Button2");
        run_for(&mut ui, &mut app, 8.0);
        click(&mut ui, &mut app, "MCForm", "Button2");
        run_for(&mut ui, &mut app, 2.0);
        let passes = |l: &[mb3d::checkpoint::Entry]| l.iter().find(|e| e.info.get("kind").map(String::as_str) == Some("mc")).and_then(|e| e.info.get("passes")).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let before = passes(&mb3d::checkpoint::list());
        // the program again, the render continued from the list
        drop((ui, app));
        (ui, app) = build();
        ui.show(U);
        app.process(&mut ui);
        let i = mb3d::checkpoint::list().iter().position(|e| e.info.get("kind").map(String::as_str) == Some("mc")).unwrap_or(0);
        ui.cm(U, "ListView1").item_index = i as i32;
        click(&mut ui, &mut app, U, "Button1");
        run_for(&mut ui, &mut app, 6.0);
        click(&mut ui, &mut app, "MCForm", "Button2");
        run_for(&mut ui, &mut app, 2.0);
        let after = passes(&mb3d::checkpoint::list());
        for e in mb3d::checkpoint::list() {
            println!("      ({} {} {:?} {})", e.key(), e.info.get("kind").cloned().unwrap_or_default(), e.info.get("passes"), e.summary());
        }
        println!("      (MC window: running {}, message box {})", app.mc.running, ui.showing("MessageBoxDlg"));
        t.check(before > 0 && after > before, &format!("Monte Carlo continued from the list after a restart: {before} → {after} passes"));
        ui.hide("MCForm");
        for e in mb3d::checkpoint::list() {
            mb3d::checkpoint::remove(&e);
        }
    }

    if want("voxel") {
        let export = |ui: &mut Ui, app: &mut Mb3d, out: &std::path::Path, stop_after: Option<f64>| {
            ui.show("FVoxelExport");
            app.process(ui);
            // (the slice count first: the import derives the DE threshold from it)
            ui.set_text("FVoxelExport", "Edit10", "500");
            click(ui, app, "FVoxelExport", "Button4");
            until(ui, app, 120.0, &|ui, _| ui.caption("FVoxelExport", "Button5") != "Stop");
            ui.set_text("FVoxelExport", "Edit2", &format!("{}/", out.display()));
            click(ui, app, "FVoxelExport", "Button2");
            if let Some(s) = stop_after {
                run_for(ui, app, s);
                mb3d::app::voxel_form::stop(app);
            }
            until(ui, app, 600.0, &|_, app| !mb3d::app::voxel_form::running(app));
        };
        let (a, b, c) = (dir.join("vox_one"), dir.join("vox_stop"), dir.join("vox_two"));
        for d in [&a, &b, &c] {
            std::fs::create_dir_all(d).unwrap();
        }
        let t0 = std::time::Instant::now();
        export(&mut ui, &mut app, &a, None);
        let secs = t0.elapsed().as_secs_f64();
        // (two exports in one go agree?)
        export(&mut ui, &mut app, &c, None);
        let fc = files(&c);
        let same2 = files(&a).iter().zip(&fc).filter(|(x, y)| x != y).count();
        println!("      (two exports in one go: {same2} of {} files differ)", fc.len());
        export(&mut ui, &mut app, &b, Some(secs * 0.5));
        let part = files(&b).len();
        export(&mut ui, &mut app, &b, None);
        let (fa, fb) = (files(&a), files(&b));
        let differ: Vec<&String> = fa.iter().zip(&fb).filter(|(x, y)| x != y).map(|(x, _)| &x.0).collect();
        println!("      ({} / {} files, differing: {:?}, kept: {})", fa.len(), fb.len(), &differ[..differ.len().min(5)], n());
        t.check(part > 0 && part < fa.len() && fa == fb && n() == 0, &format!("voxel window: stopped at {part} files, continued, {} files identical to one go", fb.len()));
        ui.hide("FVoxelExport");
    }

    if want("tracer") {
        let trace = |ui: &mut Ui, app: &mut Mb3d, out: &std::path::Path, stop_after: Option<f64>| {
            ui.show("BulbTracer2Frm");
            app.process(ui);
            click(ui, app, "BulbTracer2Frm", "ImportParamsFromMainBtn");
            until(ui, app, 120.0, &|ui, _| ui.caption("BulbTracer2Frm", "RefreshPreviewBtn") != "Stop");
            ui.set_text("BulbTracer2Frm", "MeshVResolutionEdit", "900");
            ui.set_item_index("BulbTracer2Frm", "SaveTypeCmb", 1);
            ui.set_text("BulbTracer2Frm", "FilenameREd", &out.display().to_string());
            ui.set_caption("BulbTracer2Frm", "Label13", "");
            click(ui, app, "BulbTracer2Frm", "CalculateBtn");
            if let Some(s) = stop_after {
                run_for(ui, app, s);
                click(ui, app, "BulbTracer2Frm", "CancelBtn");
                run_for(ui, app, 3.0);
            } else {
                until(ui, app, 900.0, &|ui, _| ui.caption("BulbTracer2Frm", "Label13").starts_with("Elapsed"));
            }
        };
        let (a, b) = (dir.join("mesh_one.obj"), dir.join("mesh_stop.obj"));
        let t0 = std::time::Instant::now();
        trace(&mut ui, &mut app, &a, None);
        let secs = t0.elapsed().as_secs_f64();
        println!("      (mesh in one go: {secs:.0} s)");
        trace(&mut ui, &mut app, &b, Some((secs * 0.7).max(25.0)));
        let kept = mb3d::checkpoint::list().iter().find(|e| e.info.get("kind").map(String::as_str) == Some("mesh")).map(|e| e.done_text()).unwrap_or_default();
        let t1 = std::time::Instant::now();
        trace(&mut ui, &mut app, &b, None);
        let rest = t1.elapsed().as_secs_f64();
        let same = std::fs::read(&a).ok() == std::fs::read(&b).ok() && std::fs::metadata(&a).map(|m| m.len() > 0).unwrap_or(false);
        t.check(!kept.is_empty() && same && rest < secs * 0.8 && n() == 0, &format!("Bulb Tracer: stopped with {kept} kept, continued in {rest:.0} s, the same mesh"));
        ui.hide("BulbTracer2Frm");
    }

    if std::env::var_os("KEEP").is_none() {
        let _ = std::fs::remove_dir_all(&dir);
    } else {
        println!("kept {}", dir.display());
    }
    println!("== {} passed, {} failed", t.pass, t.fail);
    if t.fail > 0 {
        std::process::exit(1);
    }
}
