//! Transparent water of the ground plane (`mb3d::water`): the normal
//! render's reflection pass and Monte Carlo see the fractal under it, the
//! graphics card leaves both to the CPU.  (A test binary of its own: it
//! switches the card on, which is global.)

use mb3d::scene::{Ground, Scene};

/// A bulb half under water, its reflections on.
fn half_submerged(transparency: f32) -> Scene {
    let mut s = Scene::preset("Integer Power").unwrap();
    s.width = 96;
    s.height = 72;
    s.mc.reflections = true;
    s.mc.reflection_amount = 0.8;
    s.ground = Some(Ground { axis: 1, pos: 0.0, color: [40, 60, 80], reflect: 0.5, wave_height: 0.004, wave_len: 0.5, transparency, absorption: 1.0, ..Default::default() });
    s
}

fn picture(s: &Scene) -> (Vec<u8>, Vec<mb3d::gbuffer::SiLight>) {
    let r = mb3d::render(s, &|_, _| {}).unwrap();
    (r.rgb, r.gbuffer)
}

#[test]
fn water_lets_the_fractal_show_through() {
    let (opaque, g0) = picture(&half_submerged(0.0));
    let (clear, g1) = picture(&half_submerged(1.0));
    // the calculation is the same: only the reflection pass sees the water
    assert_eq!(g0, g1);
    let ground: Vec<bool> = g0.iter().map(|g| g.is_ground()).collect();
    assert!(ground.iter().filter(|g| **g).count() > 500);
    let changed = opaque.chunks(3).zip(clear.chunks(3)).zip(&ground).filter(|((a, b), g)| **g && a != b).count();
    assert!(changed > 500, "only {changed} water pixels changed");
    // the background is not touched
    for ((a, b), g) in opaque.chunks(3).zip(clear.chunks(3)).zip(&g0) {
        if g.is_background() {
            assert_eq!(a, b);
        }
    }
    // without the reflection pass the water stays opaque
    let (mut a, mut b) = (half_submerged(0.0), half_submerged(1.0));
    a.mc.reflections = false;
    b.mc.reflections = false;
    assert_eq!(picture(&a).0, picture(&b).0);
}

/// The light under water fades with the absorption: more absorption, a
/// picture nearer to the water's own colour.
#[test]
fn absorption_fades_what_lies_under_water() {
    let mut a = half_submerged(1.0);
    let mut b = a.clone();
    a.ground.as_mut().unwrap().absorption = 0.0;
    b.ground.as_mut().unwrap().absorption = 50.0;
    let (pa, g) = picture(&a);
    let (pb, _) = picture(&b);
    // deep water: every water pixel turns into the same few colours (its own
    // light and the reflections); clear water shows the fractal's variety
    let spread = |p: &[u8]| {
        let px: Vec<&[u8]> = p.chunks(3).zip(&g).filter(|(_, g)| g.is_ground()).map(|(c, _)| c).collect();
        let mean = px.iter().map(|c| c[0] as f64 + c[1] as f64 + c[2] as f64).sum::<f64>() / px.len() as f64;
        px.iter().map(|c| (c[0] as f64 + c[1] as f64 + c[2] as f64 - mean).abs()).sum::<f64>() / px.len() as f64
    };
    assert!(spread(&pa) > spread(&pb), "{} {}", spread(&pa), spread(&pb));
}

/// With transparent water the reflection pass and Monte Carlo run on the
/// CPU even when the card is on: the pictures are the CPU's (the card's
/// shader marches no rays under the water).
#[test]
fn the_card_leaves_transparent_water_to_the_cpu() {
    let s = half_submerged(0.8);
    mb3d::gpu::set_enabled(false);
    let (p, mut gbuf) = mb3d::render::calculate_raw_cancellable(&s, &|_, _| {}, &|| false).unwrap();
    mb3d::render::post_process(&s, &p, &mut gbuf, 2);
    let cpu = mb3d::render::paint(&s, &p, &gbuf);
    let mut mc_cpu = mb3d::mc::McImage::new(0, 0);
    let mut mcs = s.clone();
    (mcs.width, mcs.height) = (48, 36);
    mb3d::mc::pass(&mcs, &mut mc_cpu, 2, &|_, _| {}, &|| false).unwrap();
    assert!(mb3d::reflect::water_on_cpu(&p));
    mb3d::gpu::set_enabled(true);
    let card = mb3d::render::paint(&s, &p, &gbuf);
    let mut mc_card = mb3d::mc::McImage::new(0, 0);
    mb3d::mc::pass(&mcs, &mut mc_card, 2, &|_, _| {}, &|| false).unwrap();
    let mc_status = mb3d::mc::last_status();
    mb3d::gpu::set_enabled(false);
    assert_eq!(mc_status, "CPU");
    assert_eq!(cpu, card);
    assert!(mc_cpu.recs.iter().zip(&mc_card.recs).all(|(a, b)| a.col == b.col && a.ray_count == b.ray_count));
    // opaque water and no ground: the card may take them
    let mut o = s.clone();
    o.ground.as_mut().unwrap().transparency = 0.0;
    assert!(!mb3d::reflect::water_on_cpu(&mb3d::calc::CalcParams::new(&o).unwrap()));
    o.ground = None;
    assert!(!mb3d::reflect::water_on_cpu(&mb3d::calc::CalcParams::new(&o).unwrap()));
}

/// The new keys in the scene text, and their defaults writing nothing.
#[test]
fn water_keys_round_trip() {
    let s = half_submerged(0.0);
    let t = s.to_text();
    assert!(!t.contains("ground_transparency"), "{t}");
    let mut w = s.clone();
    let g = w.ground.as_mut().unwrap();
    (g.transparency, g.absorption, g.absorb_color, g.ior) = (0.75, 2.5, [1, 2, 3], 1.4);
    let t = w.to_text();
    for k in ["ground_transparency = 0.75", "ground_absorption = 2.5", "ground_absorb_color = #010203", "ground_ior = 1.4"] {
        assert!(t.contains(k), "{k} in {t}");
    }
    let back = Scene::parse(&t).unwrap();
    assert_eq!(back.ground, w.ground);
    // the keys alone switch the ground on with its defaults for the rest
    let mut none = s.clone();
    none.ground = None;
    let a = none.apply("ground_transparency = 2\nground_ior = 0.5").unwrap();
    let g = a.ground.unwrap();
    assert_eq!((g.transparency, g.ior), (1.0, 1.0));
}
