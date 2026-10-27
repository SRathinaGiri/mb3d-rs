# Several graphics cards (plan, parked)

Today `gpu.rs` opens one card (`gpu()`, a `OnceLock<Gpu>`, the
`HighPerformance` adapter). The work is already shared from a pool:
in Auto mode the card and the CPU threads claim bands of rows from one
`free` range (`render.rs`, `march_bands` / `post_bands` with a `bands`
closure), the faster one taking more. More cards = more workers on the
same pool.

## Steps

No manual choice of cards (the user's decision, 2026-10-27): every
usable card is found and used on its own, the splitting and load
balancing happen behind the scenes.

1. **Finding the cards** (small): `enumerate_adapters`, one entry per
   physical card (the same card can show up under Vulkan and DirectX 12:
   keep one), software renderers left out; each opened as its own
   device; scenes needing doubles only on the cards that have them.
2. **All cards on every picture: march, shadows, occlusion** (medium):
   `gpu()` becomes a list of devices; pipelines compiled per card (in
   parallel); one worker per card claiming bands from the existing pool,
   so a faster card takes more on its own; a failed card's claimed bands
   go back to the pool (as they go to the CPU today); the status / LED
   counts the cards.
3. **Monte Carlo and reflections** (medium): `mc_batch` / `reflect_batch`
   task lists split between the cards by their measured speed.
4. **Animations: whole frames per card** (small to medium): best scaling,
   no seams (the VJ use of discussion #10/#11: thousands of frames).

## Points to watch

- Integrated + discrete (laptops): the integrated card shares power,
  heat and memory bandwidth with the CPU; measure each card on the first
  picture and leave out, automatically, one that adds under ~10 %.
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

Order: 1, 2, 4, 3.

## Done (local branch)

- Step 1 (1cc245e): all cards found and opened, `mb3d --list-cards`.
- Step 2 (Auto mode): each further card a worker of its own
  (`gpu::on_card`, a thread-local card index; pipelines and errors per
  card) in `march_mixed` (waits for the first card's choice of precision)
  and in the shadow / occlusion pass (`post_bands`); a failing extra card
  is left out alone (`FAILED`), its rows redone on the CPU.
  Tested with `MB3D_GPU_TWICE=1` (the RTX 2060 opened twice): the cards
  share the rows about evenly, pictures as close to the CPU's as with one
  card. `MB3D_CARDS_DEBUG=1` prints the rows of each further card.
- "Graphics card" mode: with further cards, `march_card_rest` (bands
  from a row counter, all cards; a failed further card's bands done again
  on the first) instead of `gpu::march`; one card: `gpu::march` as before.
  With the card opened twice: byte-identical to one card (shadows, AO).
- Step 3 (78141b6): Monte Carlo and reflection batches (`on_cards`): each
  further card a slice at the front by its measured rays per second, the
  first card and the CPU share the rest as before (the first card starts
  behind the slices, `offset`); resumers cached per card. With the card
  opened twice: Monte Carlo byte-identical to one card (with
  `MB3D_MC_CPU_SHARE=0`); reflections differ only as two one-card runs do.
- Step 4: animations a frame per card (`frames::render_frames`,
  `gpu::solo_card`: the frame stays on its card), the CPU threads split;
  not with map sequences (their frame number is global). Command line and
  animation window; the swarm renders its frames one at a time (each on
  all cards). `mb3d animate --card / --cpu`. Card opened twice: all frames
  byte-identical to one card.
- The threads a render starts that call the card carry the picture's card
  (`gpu::card_here` / `as_card`: Monte Carlo's card thread, reflections,
  volumetric light); background compiles and their failures are per card.
- Integrated graphics left out when a dedicated card is there (the
  user's decision), a dedicated card first (`card_order`).
- lavapipe as a second device (`MB3D_GPU_ALLOW_SOFTWARE=1`): pictures as
  close to the CPU's as with one card; but a much slower card holds up
  the others at the end of a picture (DEcomb1 Auto 19 s -> 91 s, a small
  scene in card mode 0.4 s -> 14 s): a speed gate is still needed (leave
  out a card much slower than the first, small first bands until a card
  is measured).
