# The experimental speed-ups

mb3d-rs 0.2.92 has five experimental speed-ups that change the picture a
little, so they are off until you switch them on. Measured on the 80 example
files that ship with the program (2 November 2026, about 0.4 megapixels, one
PC: an RTX 2060 and a Ryzen 9 3900X), all five together make a render a tenth
faster on average and a third faster in open scenes; on the processor alone the
cone march cuts the march by 18 %, up to 64 %.

## Where they are

The main window → **Internal** tab → **Experimental...** opens the window
below. Tick *Use the experimental speed-ups*, then the ones you want. They are
saved with the scene, so the command line, the swarm and the AI assistant use
them too, and the program keeps your choice from one start to the next.

<img src="img/experimental_path.png" alt="The Internal tab with the Experimental... button" width="157"> <img src="img/experimental_window.png" alt="The Experimental speed-ups window with all five switched on" width="522">

On the command line: `mb3d scene.m3p -s experimental=on -s cone_march=on`,
with the other keys from the table.

| Speed-up | What it does | Scene key |
|---|---|---|
| Step relax | Longer march steps; an overstep is taken again at its safe length | `step_relax = 1.3` |
| Secant search | Finds the surface by interpolating between the last two probes instead of halving | `secant_search = on` |
| No step limit | Longer steps through open space when the scene has no dynamic fog | `free_steps = on` |
| DE occlusion at half resolution | Ambient occlusion on every second pixel of every second row, the others filled in | `occlusion_half = on` |
| Cone march (new in 0.2.92) | A coarse ray per block of pixels; every pixel's march starts where it stopped | `cone_march = on` |

All five work on the processor and on the graphics card.

## Each one on its own

On its own, each speed-up shaves 0.4–1.8 % off a whole render on this PC,
because the graphics card already does most of the marching and the shadows,
occlusion and reflections take their own time. Look at the stage each one
works on and the picture is clearer: step relax cuts the march by 9 %, the
cone march by 5 %, and the half-resolution occlusion cuts the occlusion stage
by 44 %.

![Time saved per speed-up: the whole render and the stage it works on](img/experimental_gains.png)

On the processor alone the cone march matters most: measured on the same
scenes without the card, it cuts the march by 18 % (median), up to 64 % in
open scenes.

| Speed-up | Where it shines | Pixels changed (median) | Watch out for |
|---|---|---|---|
| Step relax 1.3 | Fine, deep surfaces: up to −19 % of the whole render | 2.6 % | Holes in thin shells (Pong 703); one slow dIFS scene took 70 % longer |
| Secant search | Scenes with many surface hits: up to −8 % | 0.7 % | Fine detail moves slightly |
| No step limit | Open scenes without dynamic fog: up to −30 % | 0.03 % | Nothing visible |
| Occlusion at half resolution | Scenes with DE occlusion: up to −20 % of the whole render | 0.0 % (up to 23 % in fine detail) | Softer occlusion in fine detail |
| Cone march | Open scenes: up to −43 % of the whole render (−64 % of the march on the processor) | 0.9 % | Thin parts of folding formulas at small sizes; off by itself for inside+outside, disable analytic DE and volumetric light |

"Pixels changed" is the share of pixels that differ by more than 8 of 255
from the plain render; the march's own random first step already moves about
1 % of them.

## All five together

All five switched on cut the 79 renders from 161 s to 145 s, a tenth less,
and 68 of the 79 scenes got faster (the slowest of the rest lost 7 %). The
best scenes: Nothing left but bones −45 %, LightBulbMoon – No, Never −39 %,
DEcomb1 −35 %, Mandalex −33 %. The march stage alone shrinks by 16 %, the
occlusion stage by 46 %.

That is more than the five single gains add up to (they sum to 6 %), so they
help each other more than they get in each other's way. The price: a median
3.2 % of the pixels differ from the plain render, mostly in fine shading; in
noisy scenes (spineJulia, the Julia examples) it is far more, but those differ
as much between two plain renders with a different first step.

## How they affect each other

They work on different parts of a ray, so most pairs simply add up; two pairs
do not.

| Pair | Effect | Why |
|---|---|---|
| Cone march + step relax | **Helps**: together −5 % (median) where each alone gives −1 % and −2 %; better than the better one alone in 22 scenes, worse in 5 | The cone march skips the open space in front of the surface, step relax then lengthens the steps that are left near it: each shortens the part the other cannot |
| Cone march + no step limit | **Overlaps**: together −1.8 % (median), hardly more than the cone march alone; worse than the better one alone in 13 scenes | Both remove the cost of open space; once the cone march has jumped over it, there are few long steps left to free |
| Step relax + secant search | Independent | One changes the steps, the other only the last search for the surface |
| Occlusion at half resolution + anything | Independent | It works on the occlusion stage after the march; its gain comes on top of the others' |
| Secant search + cone march | Independent | The cone march ends before the surface search begins |

None of the pairs makes a picture worse than the single switches do: the pixel
changes of "all five" (3.2 % median) are about those of step relax (2.6 %)
plus the half-resolution occlusion, and the holes step relax can make in thin
shells appear with or without the others.

## What to switch on

- For previews and test renders: all five. A tenth off every render, a third
  off open scenes, and the differences are invisible at preview size.
- For a final picture: the cone march, no step limit, the secant search and
  the half-resolution occlusion are safe in practice; add step relax only
  after comparing with a plain render, since thin shells can show holes.
- On the processor alone (no graphics card, a laptop, the swarm), the cone
  march is the one that counts: −18 % of the march on average, up to −64 %.
- The switches are saved with the scene, so a parameter file you share carries
  them; tell people when you post one.

Every number here is from the 80 example files at about 0.4 megapixels on one
PC (one scene, Fishbowl, left out of the totals: its run hit a shader compile);
your own scenes and card will differ, but the ordering of the gains should
hold. How each speed-up works: [TECHNICAL.md](TECHNICAL.md#experimental-speed-ups).
