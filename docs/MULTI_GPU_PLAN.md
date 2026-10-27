# Several graphics cards (plan, parked)

Today `gpu.rs` opens one card (`gpu()`, a `OnceLock<Gpu>`, the
`HighPerformance` adapter). The work is already shared from a pool:
in Auto mode the card and the CPU threads claim bands of rows from one
`free` range (`render.rs`, `march_bands` / `post_bands` with a `bands`
closure), the faster one taking more. More cards = more workers on the
same pool.

## Steps

1. **Choosing the card** (small): list of the adapters
   (`enumerate_adapters`), checkboxes in a settings window, ini setting,
   `MB3D_GPU_ADAPTER=<index|name>` for the command line. Useful alone
   (laptops that pick the wrong card), and with the swarm (two local
   processes sharing a folder, one per card) it already renders big
   pictures and animations on two cards.
2. **All chosen cards on every picture: march, shadows, occlusion**
   (medium): `gpu()` becomes a list of devices; pipelines compiled per
   card (in parallel); one worker per card claiming bands; a failed card's
   claimed bands go back to the pool (as they go to the CPU today);
   status / LED shows the cards.
3. **Monte Carlo and reflections** (medium): `mc_batch` / `reflect_batch`
   task lists split between the cards by their measured speed.
4. **Animations: whole frames per card** (small to medium): best scaling,
   no seams (the VJ use of discussion #10/#11: thousands of frames).

## Points to watch

- Integrated + discrete (laptops): the integrated card shares power,
  heat and memory bandwidth with the CPU; measure each card on the first
  picture and leave out one that adds under ~10 % (user can override).
- Cards of different makers round differently: bands may differ in a few
  pixels (card vs CPU today: 0.8 % of the pixels in single precision on
  DomeTree); identical cards give identical pictures.
- Doubles (deep zooms): a card without SHADER_F64 is left out of those
  scenes (Metal has none).
- Each card holds its own copy of the buffers.

## Testing

The development PC has one RTX 2060 only. Logic: Mesa's lavapipe as a
stand-in second card (`MB3D_GPU_ALLOW_SOFTWARE`). Speed: needs the
user's machine with two cards (asked, not answered yet).

Recommended order: 1, 2, 4, 3.
