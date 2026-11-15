//! A scene whose formulas overflow single precision, marched on the CPU,
//! its shadows and DE occlusion then offered to the graphics card (as when
//! the card's shader was still compiling during the march, or after a
//! repaint): the card's rows must be given up for the overflow and the
//! result be the CPU's.
//! `cargo run --release --example overflowcheck -- SCENE FORMULA_DIR`
//! Prints how many pixels differ from the passes on the CPU alone and
//! whether the picture was marked (no later pass on the card); fails when
//! a shadow differs or the picture was not marked.
use std::sync::atomic::Ordering;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    mb3d::formulas::add_formula_dir(std::path::PathBuf::from(&a[1]));
    let (sc, _) = mb3d::animfile::load_scene(std::path::Path::new(&a[0])).unwrap();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let none = |_: usize, _: usize| {};
    let stage = |_: &str| {};
    let go = || false;
    // the march and the passes on the CPU alone
    mb3d::gpu::set_enabled(false);
    let (p, raw) = mb3d::render::calculate_raw_cancellable(&sc, &none, &go).unwrap();
    let mut cpu = raw.clone();
    mb3d::render::post_process_reporting(&sc, &p, &mut cpu, threads, &none, &stage, &go).unwrap();
    // (the CPU's passes twice give the same, also with parameters made anew)
    let d = |x: &[mb3d::gbuffer::SiLight], y: &[mb3d::gbuffer::SiLight]| x.iter().zip(y).filter(|(x, y)| x.shadow != y.shadow || x.amb_shadow != y.amb_shadow).count();
    let mut cpu2 = raw.clone();
    mb3d::render::post_process_reporting(&sc, &p, &mut cpu2, threads, &none, &stage, &go).unwrap();
    let p0 = mb3d::calc::CalcParams::new(&sc).unwrap();
    let mut cpu3 = raw.clone();
    mb3d::render::post_process_reporting(&sc, &p0, &mut cpu3, threads, &none, &stage, &go).unwrap();
    println!("CPU twice: {} differ; CPU with parameters made anew: {} differ", d(&cpu, &cpu2), d(&cpu, &cpu3));
    // the passes with the card: once to start its compile, then for real
    mb3d::gpu::set_enabled(true);
    let mut first = raw.clone();
    let p1 = mb3d::calc::CalcParams::new(&sc).unwrap();
    mb3d::render::post_process_reporting(&sc, &p1, &mut first, threads, &none, &stage, &go).unwrap();
    while mb3d::gpu::compiling() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let p2 = mb3d::calc::CalcParams::new(&sc).unwrap();
    let mut card = raw.clone();
    mb3d::render::post_process_reporting(&sc, &p2, &mut card, threads, &none, &stage, &go).unwrap();
    let differ = cpu.iter().zip(&card).filter(|(x, y)| x.shadow != y.shadow || x.amb_shadow != y.amb_shadow).count();
    let marked = p2.gpu_overflow.load(Ordering::Relaxed);
    println!("{} of {} pixels differ from the CPU's passes; picture marked as overflowing: {marked}", differ, cpu.len());
    // then nothing of it goes to the card
    let mut again = raw.clone();
    mb3d::render::post_process_reporting(&sc, &p2, &mut again, threads, &none, &stage, &go).unwrap();
    let differ2 = cpu.iter().zip(&again).filter(|(x, y)| x.shadow != y.shadow || x.amb_shadow != y.amb_shadow).count();
    println!("a second time (as a repaint): {differ2} pixels differ");
    let sh = cpu.iter().zip(&card).filter(|(x, y)| x.shadow != y.shadow).count();
    let big = cpu.iter().zip(&card).filter(|(x, y)| (x.amb_shadow as i32 - y.amb_shadow as i32).abs() > 1).count();
    println!("of those: shadows {sh}, occlusion off by more than 1: {big}");
    // (the screen space occlusion of the card rounds some values by one)
    std::process::exit((sh + big > 0 || !marked) as i32);
}
