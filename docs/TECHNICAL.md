# mb3d-rs: technical notes

The technical side of [mb3d-rs](../README.md): how the graphics card is used, what was ported from MB3D's Delphi sources and how, the custom formula tiers, the scene file keys, building from source and the checks against MB3D. [How the rendering works](RENDERING.md) explains the rendering for users.

## Contents

- [Graphics card](#graphics-card)
- [What is ported](#what-is-ported)
- [Custom formulas (.m3f)](#custom-formulas-m3f)
- [Scene files (.m3s)](#scene-files-m3s)
- [Ambient occlusion and shadows](#ambient-occlusion-and-shadows)
- [Lights, volumetric light, cutting planes, depth of field](#lights-volumetric-light-cutting-planes-depth-of-field)
- [Reflections and transparency, normals on the z-buffer](#reflections-and-transparency-normals-on-the-z-buffer)
- [Hybrids, inside rendering, maps](#hybrids-inside-rendering-maps)
- [Output: 16 bit pictures, noise filter](#output-16-bit-pictures-noise-filter)
- [Known differences and gaps](#known-differences-and-gaps)
- [Videos](#videos)
- [Time left and the progress bar](#time-left-and-the-progress-bar)
- [Unfinished renders (`checkpoint.rs`)](#unfinished-renders-checkpointrs)
- [Where the program keeps its files](#where-the-program-keeps-its-files)
- [Linux packaging and the command line](#linux-packaging-and-the-command-line)
- [Building from source](#building-from-source)
- [Nightly tests](#nightly-tests)

## Graphics card

[How the rendering works](RENDERING.md) explains the graphics card,
the "Calculate on" setting and the LED in the title bar with pictures,
without technical terms.

The main calculation (the ray marching of every pixel) runs on the
graphics card through [wgpu](https://wgpu.rs) (Vulkan, Metal, DirectX 12)
whenever the scene allows it, and on the CPU otherwise:

* in the program, a blinking LED right of the window title shows where the
  last calculation ran: **green** on the graphics card, **orange** mixed
  (ray marching, shadows, ambient occlusion and reflections on the
  graphics card, normals on the z-buffer on the CPU), **red** on the CPU,
  grey before the first calculation; the mouse on it shows the details
  (for Monte Carlo renders the parts of the marches on the card and on
  the CPU);
* "Calculate on" on the Internal tab: **Auto** shares a normal render
  between the card and the CPU: the CPU threads start at once and take
  rows from the bottom, the card takes rows from the top as soon as its
  shader is compiled, until they meet. The hard shadows and the DE
  ambient occlusion then go to the card when it marched clearly faster
  (1.5 times the threads' rows per second); otherwise the two share them
  the same way, the card bands of rows from the top (smaller as the rows
  run out), the threads rows from the bottom, so that a card as fast as
  the CPU or slower (a deep zoom in double precision) still helps and
  neither waits. The first time a
  formula combination is used its shaders compile (10 to 20 s; the
  driver keeps them in `ShaderCache` next to the program for the next
  time) while the CPU already calculates, so
  small images never wait for a compile (when the CPU finishes before the
  card joins, the post calculations run on the CPU too). Monte Carlo
  renders go to the card from the moment its shader is ready. **CPU**
  never uses the graphics card; **Graphics card** waits for the shader.
  Kept in `Mandelbulb3D.ini`;
* `mb3d` uses it too (it prints `calculated on: ...`); `--cpu` calculates
  on the CPU only, `--card` on the graphics card only (waiting for its
  shaders, as the program's *Graphics card* setting);
* the environment variable `MB3D_GPU=0` switches it off for both.

The shader (`src/gpu_march.wgsl`) is a port of the CPU's per-pixel loop
and fills the same G-buffer, then runs the hard (or soft) shadows and the
DE ambient occlusion on it (the same shader, started on the surface of
each pixel). The screen space ambient occlusion (both the 24 bit and the
older 15 bit kind) has shaders of its own (`src/gpu_ssao.wgsl`,
`src/gpu_ssao15.wgsl`) that only need the depth buffer, so it runs on the
card after a CPU calculation too; with regular sampling it gives the
CPU's values exactly, with random sampling the random numbers differ.
The reflections of the normal renderer run on the card too: every
bounce of all pixels is one batch that marches the reflected rays and
calculates the normals, colours, shadows and ambient occlusion of the
hits, while the shading and the blending of the bounces stay on the CPU
(as in MB3D's `CalcSR`). With transparency every surface sends a
refracted and a reflected ray, marched in rounds over all pixels;
cutting planes end the rays on the card, and deep zooms keep their
positions in double precision. The CPU takes rays from the other end of
each round and finishes the last few long ones the card would wait on
(`MB3D_REFLECT_CPU_SHARE=0` leaves them all to the card); scenes with
volumetric light keep the CPU pass. The painting stays on
the CPU. The marcher calculates in single precision. The formulas reach it through the lifter: the x86 code
of an `.m3f` formula is analysed (`src/x86/lift.rs`: every memory access
resolved to the iteration record, the formula's constants or the stack)
and translated to WGSL (`src/x86/lift_wgsl.rs`), including the SSE2 code many
formulas use and record fields chosen by an index; the built-in formulas
except the JIT example are written by hand. The formulas' control flow
is emitted structured (loops and one-pass loops as scopes, with flag
variables where WGSL's unlabelled `break` and `continue` do not reach the
target; `structure_wgsl`), as the Rust code is; the `switch` dispatcher
in a loop that it replaced made the driver's code 2.4 times slower on the
dIFS march and 1.8 times on the reflection pass (ApolloBalloons on a
GeForce RTX 2060) and took longer to compile. The dispatcher remains for
the few formulas whose flow does not nest or nests beyond naga's brace
limit (`M3F2RS_DISPATCH=1` forces it, for comparisons). A scene runs on the GPU when it
is a hybrid of such formulas, alternating or interpolating, 3D or 4D, with
the numerical or an analytic DE (options 0, 2, 5, 6 and 11), a dIFS hybrid
(option 20) or a DE combination of two such parts (minimum, maximum,
inverted maximum, the two smooth minima and the dIFS mix), also with
cutting planes, inside or inside-and-outside rendering and colour on
iteration; formulas that read maps through MB3D's map functions get the
maps uploaded to the card. Everything else is calculated on the CPU as
before: the formulas the lifter cannot resolve. Volumetric light runs on the
card too: its map is marched there as shadow rays shared with the CPU
threads (the CPU only takes it when the map is too big for one buffer).
Views zoomed so far that single precision would show
as noise are calculated in double precision on cards with doubles in
shaders (Vulkan and DirectX 12 on NVIDIA, AMD and Intel; not Metal): the
shader's real type becomes f64, with sine, cosine, logarithm and
exponential written out in double precision. The limit depends on the DE:
a numerical DE samples the formula at a tiny offset, which must span a few
single precision steps of the coordinates (about zoom 7 at 800 pixels
width); an analytic DE or dIFS only needs the pixel steps themselves to be
distinct (64 single precision steps), which allows far deeper views.
Some scenes that are not deep zooms still come out visibly different in
single precision, mostly through the roughness that smoothed normals
measure from the formula's values at tiny offsets around the surface. No
parameter of the scene tells them apart, so a probe decides: before a
calculation on the card, about 3000 pixels (a grid over the image) are
marched on the card in single precision and on the CPU (at most 150 ms
there, 1.5 s on the card), painted and compared; when their colours
differ by more than 1.3 % on average, the card calculates the scene in
double precision. The decision is kept for the scene (its shader, view,
size and lighting), so calculating it again does not probe again. The
probe needs the card's single precision shader: with *Auto* it runs once
that is compiled (from the next calculation of the scene on), with
*Graphics card* the calculation waits for it.

In such scenes single precision mostly finds the same surfaces; what it
gets wrong are the values measured at them. So the card marches them in
single precision all the same and the CPU threads finish its pixels in
double (`Marcher::finish_pixel`): from the card's depth, the binary
search within its last digit, the normal, roughness, gradient and colour,
as the CPU's own march has them when it gets there; the step count (the
dynamic fog) stays the card's. That costs the CPU a few DE calculations
a pixel against hundreds for a whole march. The probe finishes its
sample the same way, and the scene stays in double precision when even
that differs by more than the limit (2 of the 21 example scenes the
probe sends to double, *Waste-and-tragedy* and *Bone Menger*: their
colours come from orbit traps that change with the tiniest move along
the ray). Scenes with reflections stay in double precision too: their
rays start from points that differ within the depth's last digit, and on
fine detail they sparkle differently (*Blood on the dancefloor* differed
from the CPU's picture by 1.8 % on average, more than any of the 46
other example scenes does in single precision). With *Auto* the CPU
threads finish the card's rows before marching their own; with
*Graphics card* they only finish. The shadows and the occlusion then run
on the CPU, as after a march in double. Over the 21 scenes (card only,
480 pixels wide, compiles left out) 49.1 s against 37.5 s: the 13 that
are finished on the CPU 1.2 to 2.5 times faster (*Shrooms* 5.8 -> 2.3 s,
*TreePlanet* 1.35 -> 0.59 s), and their pictures differ from the CPU's
no more than the card's single precision pictures of the other example
scenes do (at most 1.2 % on average, 5.6 % of the pixels by more than a
tenth). `MB3D_GPU_MIXED=0` keeps the card in double precision. Monte
Carlo renders use the probe too (in double precision, as before) when
the scene's march is compiled. `MB3D_GPU_NOPROBE=1` turns the probe off
and `MB3D_PROBE_DEBUG=1` prints its result. The command line program waits for a shader still
compiling before it ends (`gpu::wait_compiles`): a compile the CPU
threads had overtaken used to die with the process and never reach the
driver's cache, so a short scene's next run was just as cold.
`mb3d` prints which was used and why; errors of the graphics card or its
driver also send the calculation to the CPU, and software renderers
(llvmpipe, WARP) are not used. The first calculation of a scene compiles
its shader (one to twenty seconds depending on the driver). The driver
keeps it for the next run: the programs give it a cache of its own in
`ShaderCache` next to the program (`gpu::use_own_driver_cache`, the
NVIDIA `__GL_SHADER_DISK_CACHE_*` and Mesa `MESA_SHADER_CACHE_*`
settings, unless the user set them), because the shared cache in the
user's folder can be full (NVIDIA keeps about 1 GB) and a full one made
every run compile again. The desktop shares
the graphics card and waits for each piece of work sent to it, so the
calculation is sent in pieces of about 50 ms (`MB3D_GPU_BAND_MS`): a pixel
that needs longer (a single dIFS pixel can take 100 ms on the card) saves
its state and continues in the next piece, with identical results.

Of the 80 example parameter files of MB3D, 79 run on the GPU at their
own image size (449 of the 460 formula files translate). The times below
compare the graphics card with this program's own CPU renderer, not with
MB3D: MB3D runs the formulas' machine code directly and is often much
faster on the CPU (*Chrystal* at 1600 pixels takes 19.3 s in MB3D under
Wine on the same machine, 23.8 s in this program's CPU renderer, where
the translated formulas are called directly on the iteration state). At their own
size *LightBulbMoon - No, Never* takes 2.0 s instead of 58 s; with the
reflections on the card, *Rama - Elysium* at 160
pixels takes 4.5 s instead of 24.7 s and *LightBulbMoon - A Fabulous
Excuse* 1.1 s instead of 6.8 s. On a GeForce GTX 1650 (laptop) against a
Ryzen 5 3550H (8 threads), at 640 pixels: *TimeMachine* 1.5 s instead of
19.8 s, *Theli-At - Bone Menger* 1.4 s instead of 11.8 s,
*Hal-Tenny-Resistance* 16.5 s instead of 99 s; with the shadows on the
card too, *TreePlanet* at 400 pixels takes 0.8 s instead of 6 s; at 320
pixels the dIFS scenes *ApolloBalloons* and *AureliusCat - Bamboo* take
4.2 s instead of 54 s and 0.6 s instead of 6.7 s, the DE combination
*ExcludeBulbMeng* 0.8 s instead of 14.8 s. Scenes with many iterations of
strongly expanding formulas show single precision noise (*MengerTrees*),
and DE combinations whose parts are long plain hybrids gain little
(*Fishbowl* 6.7 s against 6.9 s, *DEcomb1* 168 s against 189 s). The shadow rays differ from MB3D in one
point: MB3D stops a ray early when the neighbouring pixel found open air at
that distance; the card marches every ray to its end, so fine structures
throw slightly more shadow (a few percent of their pixels).

### CPU and graphics card compared (GTX 1650, earlier measurement)

All 80 example parameter files of MB3D (`M3Parameter`), each rendered
three ways at up to 480 pixels wide on a GeForce GTX 1650 (laptop) with a
Ryzen 5 3550H (8 threads): on the **CPU**; **mixed**, as *Auto* shares
an image (the card marches rows from the top, the CPU threads from the
bottom; when this table was made the post calculations then ran on the
CPU, today *Auto* runs them on the card); and on the **card** ("Calculate
on: Graphics card", the shaders compiled beforehand). *Faster* is the CPU's
time divided by the card's; the differences are the mean difference of
the final images to the CPU's.

* 73 of the 80 files run on the card; the other 7 stay on the CPU
  (volumetric light, a built-in JIT example, and four whose formulas
  overflow single precision: the card's result is discarded and the CPU
  calculates them, in mixed mode too).
* The card is a median **5.4 times** faster than the CPU (up to 87 times;
  62 files at least twice as fast). Two DE combinations of long plain
  hybrids are a bit slower on the card (*Abominog-Ua*, *DEcomb1*).
* Half of the files differ by less than 0.5 %, three quarters by less
  than 1 %; the mean is 0.8 % on the card and 0.7 % mixed (where the
  post calculations are the CPU's). Mixed differs from the CPU no more
  than the card does, so sharing an image adds no differences of its own.
* The larger differences (1 to 5 %: *Blood on the dancefloor*,
  *AboxShapes-Misty*, *MarkJayBee - Curvichrome*, *JuliaSetSample*, ...)
  are single precision in very fine detail: the same structures, with
  some sparkle in different pixels. In double precision on the card
  (`MB3D_GPU_DOUBLE=1`) the three largest drop to 0.3, 0.3 and 0.65 %.
  The shadow rule described above and the random numbers of the SSAO with
  random sampling add a little. The table was made without the precision
  probe (see above); with it, 20 of the 73 files switch to double
  precision, among them all ten from 1.7 to 5 %, which drop to 0.03 to
  0.85 % (*Waste-and-tragedy* 1.4 %), at 1 to 7 times the card's time
  (*AboxShapes-Misty* no slower, *KentPress* 1.25 times, *JuliaSetSample*
  and *BulboxCut* 6 to 7 times). Of the files under 0.7 % two switch as
  well (*Lenord - The Beanstalk*, *InAndOutside box*; the probe paints
  without shadows and occlusion, which can hide much of a difference).

<details>
<summary>All 80 files (click to open)</summary>

| Parameter file | CPU | Card | Faster | Card vs CPU | Mixed vs CPU |
|:--|--:|--:|--:|--:|--:|
| 0Encrypted0 - ABoxSOff4d example | 7.4 s | 1.8 s | 4.1× | 0.05 % | 0.02 % |
| 6 AM - Torii temple | 50.0 s | 44.7 s | 1.1× | 0.22 % | 0.22 % |
| Abominog-Ua - 12.06.2014 | 28.3 s | 35.7 s | 0.8× | 0.61 % | 0.57 % |
| ABoxPartlyJuliaRoff | 2.2 s | 0.4 s | 5.4× | 1.33 % | 1.28 % |
| ABoxScale2Start | 0.5 s | 0.1 s | 5.7× | 0.00 % | 0.00 % |
| ABoxScale3Start | 0.5 s | 0.1 s | 5.3× | 0.00 % | 0.00 % |
| AboxShapes-Misty | 7.1 s | 3.2 s | 2.2× | 3.82 % | 2.73 % |
| Aexion 10bulbs | 4.1 s | 0.7 s | 6.1× | 0.53 % | 0.52 % |
| AkuraPare - 463b | 3.2 s | 0.3 s | 12.3× | 0.28 % | 0.28 % |
| ApolloBalloons dIFS | 86.9 s | 19.3 s | 4.5× | 1.20 % | 1.03 % |
| AureliusCat - Bamboo | 6.5 s | 0.7 s | 9.8× | 0.08 % | 0.08 % |
| BatJorge - Pong 703 - Swirls | 11.0 s | 1.0 s | 10.9× | 2.86 % | 2.74 % |
| Bill-Snowzell - Tweak - Canyon | 19.9 s | 17.9 s | 1.1× | 0.42 % | 0.42 % |
| Blood on the dancefloor | 150.4 s | 12.3 s | 12.2× | 4.93 % | 4.52 % |
| BulboxCut | 0.9 s | 0.2 s | 5.4× | 1.76 % | 1.63 % |
| Chrystal | 2.3 s | 0.3 s | 7.5× | 0.20 % | 0.17 % |
| cutted sphere over carpet | 16.1 s | 3.7 s | 4.4× | 0.15 % | 0.13 % |
| Dainbramage - Home of the Ancients | 6.5 s | 2.4 s | 2.7× | 0.30 % | 0.30 % |
| Dainbramage - Hydra | 14.9 s | 4.9 s | 3.1× | 0.16 % | 0.16 % |
| Dainbramage - Nothing left but bones | 6.1 s | 3.0 s | 2.0× | 0.01 % | 0.01 % |
| DEcomb1 | 18.4 s | 19.8 s | 0.9× | 0.00 % | 0.00 % |
| dIFS heart shrub | 8.7 s | 2.1 s | 4.1× | 1.11 % | 0.94 % |
| dIFS heart shrub otrap | 9.7 s | 2.3 s | 4.2× | 1.19 % | 1.08 % |
| dIFS shrub | 12.4 s | 2.5 s | 5.0× | 0.21 % | 0.15 % |
| Dodeca Torus mix | 1.2 s | 0.1 s | 9.0× | 0.00 % | 0.00 % |
| DsyneGrafix - Getting Loopy | 2.6 s | 1.6 s | 1.6× | 0.16 % | 0.15 % |
| Ellarien - Embellished | 5.0 s | 3.1 s | 1.6× | 0.00 % | 0.00 % |
| Ellarien - Shrooms are coming | 27.2 s | 3.1 s | 8.7× | 1.32 % | 1.03 % |
| ExcludeBulbMeng | 22.4 s | – | – | CPU only: single precision overflows | |
| Fishbowl | 15.8 s | 3.5 s | 4.5× | 1.10 % | 0.84 % |
| Fractales75 - Kiosk | 2.7 s | 1.8 s | 1.5× | 0.09 % | 0.09 % |
| Genetic Menger | 2.4 s | 0.4 s | 6.8× | 0.01 % | 0.00 % |
| genQuat helix mix | 8.2 s | 1.2 s | 6.5× | 0.22 % | 0.18 % |
| HalTenny - Freak Of Nature | 2.2 s | – | – | CPU only: single precision overflows | |
| Hal-Tenny-Resistance (slow) | 66.8 s | 18.3 s | 3.7× | 1.06 % | 1.05 % |
| Hyperben2 - Ozosphere tweak | 1.9 s | 0.3 s | 6.2× | 0.64 % | 0.61 % |
| ImpossibleWorld dIFS | 246.6 s | 123.6 s | 2.0× | 0.61 % | 0.48 % |
| InAndOutside box | 0.7 s | 0.1 s | 7.7× | 0.13 % | 0.11 % |
| Insect Menger | 35.7 s | 0.4 s | 87.1× | 0.01 % | 0.01 % |
| Jost1 - Tweak - New tiny temple | 5.9 s | 1.4 s | 4.2× | 0.00 % | 0.00 % |
| julia on box | 8.1 s | 1.1 s | 7.1× | 0.80 % | 0.41 % |
| JuliaSetSample | 4.1 s | 0.6 s | 6.6× | 3.08 % | 2.73 % |
| KentPress - Parameter 3 tweak fogits | 34.6 s | 15.5 s | 2.2× | 2.93 % | 2.28 % |
| KochSurf Sample 6 | 6.9 s | – | – | CPU only: volumetric light | |
| Lenord - ASurf 2 | 5.8 s | 3.3 s | 1.7× | 0.15 % | 0.15 % |
| Lenord - Between Dark and Light | 10.4 s | – | – | CPU only: volumetric light | |
| Lenord - The Beanstalk | 4.0 s | 0.5 s | 7.5× | 0.52 % | 0.51 % |
| LightBulbMoon - A Fabulous Excuse | 9.1 s | 1.7 s | 5.4× | 2.86 % | 2.79 % |
| LightBulbMoon - No, Never | 28.2 s | 2.9 s | 9.9× | 0.10 % | 0.10 % |
| Mandelwerk - Makin seashell | 13.5 s | 11.9 s | 1.1× | 0.21 % | 0.21 % |
| Mandelwerk - TgladTetra inner spirals | 17.4 s | 1.2 s | 14.9× | 0.96 % | 0.90 % |
| MarkJayBee - Curvichrome | 8.2 s | 1.3 s | 6.2× | 3.42 % | 3.01 % |
| material colors | 3.1 s | 1.5 s | 2.1× | 0.03 % | 0.02 % |
| Melting spot bloxx | 107.9 s | 18.5 s | 5.8× | 0.71 % | 0.66 % |
| Mengerplus for MC | 4.3 s | 1.9 s | 2.3× | 0.75 % | 0.64 % |
| MengerTrees | 10.6 s | 2.0 s | 5.3× | 1.27 % | 1.05 % |
| MutaGenD4B12E71C70D4FD00357_canary~ | 7.0 s | – | – | CPU only: JIT formula | |
| Oxnot - Flexing | 3.2 s | 0.2 s | 15.1× | 0.00 % | 0.00 % |
| Oxnot - Shells | 6.9 s | – | – | CPU only: single precision overflows | |
| PaMonk - Ringlets | 7.1 s | 0.3 s | 26.4× | 0.87 % | 0.84 % |
| QuatP4hybridJulia | 0.8 s | 0.1 s | 6.5× | 0.19 % | 0.17 % |
| Rama - Elysium | 68.7 s | 14.8 s | 4.6× | 0.14 % | 0.05 % |
| Recycledrelatives - Fractal Fan | 0.9 s | 0.1 s | 6.7× | 0.04 % | 0.04 % |
| Recycledrelatives - Test for SurfMod | 5.5 s | 0.5 s | 10.4× | 0.99 % | 0.95 % |
| Rowratty - Spirals | 4.0 s | 0.6 s | 6.8× | 0.65 % | 0.56 % |
| Sabine62 - Housing problem | 34.1 s | 12.5 s | 2.7× | 3.09 % | 2.95 % |
| spineJulia | 20.8 s | – | – | CPU only: single precision overflows | |
| Surreal shell | 1.8 s | 0.3 s | 6.6× | 1.25 % | 1.17 % |
| Test8 | 0.6 s | 0.1 s | 10.2× | 0.00 % | 0.00 % |
| Theli-At - Black Mandelbulb | 1.4 s | 0.1 s | 15.6× | 0.37 % | 0.29 % |
| Theli-At - Bone Menger using MH | 4.8 s | 0.8 s | 5.6× | 0.78 % | 0.78 % |
| Theli-At - MengerSpheres | 3.7 s | 0.6 s | 6.6× | 0.05 % | 0.05 % |
| ThePearl dIFS | 2.7 s | 0.8 s | 3.2× | 0.03 % | 0.03 % |
| TimeMachine | 1.6 s | 0.5 s | 3.0× | 1.02 % | 0.62 % |
| toricaleggs | 14.2 s | 5.5 s | 2.6× | 0.29 % | 0.28 % |
| TreePlanet | 4.4 s | 0.7 s | 6.6× | 0.87 % | 0.84 % |
| UndeadAcademy tweak - Peacock in blue | 8.2 s | 1.0 s | 7.9× | 0.68 % | 0.62 % |
| Virtual tubes world (UndeadAcademy style) | 23.1 s | 4.8 s | 4.9× | 0.84 % | 0.59 % |
| Wada basin - Reflect test | 1.2 s | 0.4 s | 2.8× | 0.02 % | 0.02 % |
| Waste-and-tragedy - Mandalex | 36.8 s | 22.2 s | 1.7× | 2.52 % | 1.45 % |

</details>

## What is ported

Each Delphi routine was ported from its assembler or Pascal source. Where only
x87/SSE2 assembler existed, the stack code was translated back to scalar code
and kept in the original evaluation order.

| Rust | Delphi source | Contents |
|---|---|---|
| `math.rs` | Math3D.pas | view vectors (`BuildViewVectorDFOV`, sphere pano), `RotateVectorReverse`, `NormaliseMatrixTo`, `BuildSMatrix4`/`Rotate4Dex`, `CreateXYVecsFromNormals`, `FastIntPow`, spline weights |
| `formulas.rs` | formulas.pas, CustomFormulas.pas, HeaderTrafos.pas | Integer Power 2–8 (`HybridItIntPow2..P8`), Real Power (`HybridFloatPow`), Quaternion, Tricorn, Amazing Box (`HybridCube`/`HybridCubeDE`), Bulbox (`HybridSuperCube2`), Folding Int Pow, with their default options, DE options, DE scales and bailouts |
| `iteration.rs` | formulas.pas, TypeDefinitions.pas | `TIteration3Dext`, `doHybridPas`, `doHybridPasDE`, `doHybrid4DPas`, `CalcSmoothIterations` |
| `calc.rs` | HeaderTrafos.pas, Calc.pas, CalcThread.pas | `GetMCTparasFromHeader` (DE scale, colour heuristics, camera, z buffer constants), `CheckFormulaOptions`, `CalcDEanalytic`, `CalcDEnoADE` (4-point gradient DE), `RMCalculateNormals[OnSmoothIt]` incl. smooth normals and roughness, `RMdoBinSearch[It]`, `RMdoColor`, `CalcZposAndRough`, the main ray-march loop of `TMandCalcThread.Execute` (random first step, step-width regulation, dynamic fog step counting) |
| `lighting.rs` | HeaderTrafos.pas, LightAdjust.pas, PaintThread.pas | `MakeLightValsFromHeaderLight`, `SetCosTabFunction`/`GetCosTabVal`, `CalcColors[Inside]`, `CalcPixelColor2` and `CalcPixelColorSqr` ("internal gamma 2"): global lights with diffuse + specular, light maps, background pictures, diffuse colour maps, ambient top/bottom or from the background, depth fog, dynamic fog options, both total-light modes, gamma; MB3D's start-up light preset |
| `render.rs` | Calc.pas, PaintThread.pas, DivUtils.pas | threaded calculation (rows interleaved per thread like `CalcMandT`), painting, tiled rendering (`CalcRect`, `TilingOptions`) |
| `ssao.rs` | AmbHiQ.pas | 24 bit screen space ambient occlusion (`TAmbHiQCalc`, `TAmbHiQCalcT0`, the random variants `TAmbHiQCalcR`/`RT0`, `NextATlevelHiQ`) |
| `ssao15.rs` | AmbShadowCalcThreadN.pas | 15 bit SSAO (`BuildATlevels`, `TAmbShadowCalc`, `TAmbShadowCalcT0`) |
| `iteration.rs` (dIFS) | formulas.pas | `doHybridIFS3D` and the dIFS calling convention |
| `calc.rs` (DE combination, inside) | Calc.pas, CalcThread.pas, HeaderTrafos.pas | DE combinations of two hybrid parts (min, max, inverted max, smooth, mix), inside and in-and-outside rendering, `CheckHybridOptions` |
| `image.rs` | (new) | PNG, JPEG (baseline, progressive), BMP and PGM decoders for the maps |
| `maps.rs` | ImageProcess.pas, maps/Maps.pas | `LoadLightMap`, bicubic `GetLightMapPixel[Sphere]`, `SplineIpolMap`, `MakeSmallLMimage`; map formulas get MB3D's `PMapFunc` host functions |
| `calc.rs` (shadows), `render.rs` | CalcHardShadow.pas, HeaderTrafos.pas | hard shadows for up to 6 global lights incl. the "open air" shortcut, the one-light soft shadow (`calcHSsoft`), `CalcHSVecsFromLights` |
| `lighting.rs` (positional) | PaintThread.pas, HeaderTrafos.pas | positional lights with 1/d² falloff, visible light sources (`CalcPosLightShape`, `CalcXYZposForLight`, `SortLights`) |
| `vollight.rs` | maps/Maps.pas, CalcThread.pas | volumetric light: light shadow map (cube map for positional lights) and its integration along the view rays (`DoDynFog`) |
| `deao.rs` | CalcAmbShadowDE.pas | DE ambient occlusion with 3–33 rays, dithering and ray correction |
| `dof.rs` | DOF.pas | depth of field, sorted and forward variants, 1–4 passes |
| `m3p.rs` (text) | FileHandling.pas, DivUtils.pas | text parameters (`GetHeaderFromText`, `MakeTextparas`), `.m3i` parameter loading |
| `gbuffer.rs` | TypeDefinitions.pas | `TsiLight5` |
| `app/*.rs`, `app/forms/*.dfm` | Mand.pas, FormulaGUI.pas, LightAdjust.pas, PostProcessForm.pas, Navigator.pas, Animation.pas, BatchForm.pas, Tiling.pas, MonteCarloForm.pas, VoxelExport.pas, BulbTracer2UI.pas, MutaGenGUI.pas, HeightMapGenUI.pas, MeshPreviewUI.pas, JITFormulaEditGUI.pas, ... | the Mandelbulb3D program: MB3D's forms and their event handlers |
| `vcl/*.rs` | Delphi VCL | the toolkit: `.dfm` reader, the controls (Glossy and Windows looks), layout (align, anchors), input, dialogs, windows |
| `jpeg.rs` | (new) | baseline JPEG encoder |
| `scene.rs` | TMandHeader10 + GUI defaults | scene parameters, defaults, the `.m3s` text format |
| `m3p.rs` | FileHandling.pas (`LoadParameter`, `UpdateLightParasAbove3`), DivUtils.pas (tiling), TypeDefinitions.pas | binary `.m3p` files (MandId ≥ 20): `TMandHeader10`, `TLightingParas9`, `THeaderCustomAddon`; reading with the upgrades of older versions, and writing (MandId 44) |
| `m3p_old.rs` | FileHandling.pas (`ConvertHeaderFromOldParas`, `ConvertLight8to9`, `UpdateLightVersion8`, `Convert4to7paras`, `LoadHAddon`, `LoadFullM3I`), HeaderTrafos.pas (`CalcVGradsFromHeader8rots`, `GetHAddOnFromInternFormula`) | parameters from before MB3D 1.6 (MandId < 20: `TMandHeader8`, `TMandHeader4`, `TMandHeaderOld`, `TLightingParas8`, `THeaderCustomAddonOld`) in `.m3p`, `.m3i` (with the `TsiLight3`/`TsiLight4` G-buffer) and text parameters before version 16, converted to `TMandHeader10` |
| `m3f.rs` | CustomFormulas.pas (`LoadCustomFormula`, `FillCustomVBufWithVars`) | `.m3f` custom formula files, all 23 option types, constants |
| `x86.rs` | (new) | IA-32 interpreter for the formulas' machine code: integer, x87 and SSE/SSE2 subset; plus a compiler that resolves the x87 stack statically into micro-ops |
| `custom.rs` | formulas.pas / TypeDefinitions.pas | memory layout of `TIteration3Dext` and the calling convention used by MB3D's hybrid loop, host functions |
| `anim.rs` | Animation.pas, Interpolation.pas, Math3D.pas | keyframes, sub-frame schedule (`TotalBMPsToRender`, `Timer2Timer`), `Interpolate2frames`, `Interpolate3framesBezier` (header and formula values), `bInterpolateFormula`, quaternions (`MatrixToQuat`, `SlerpQuat`, `SlerpSVec`, `BezierIpol3SVecs`) |
| `lighting.rs` (`blend`) | Interpolation.pas | the light value part of both interpolations (`TLightVals`, `Slerp2SMatrices`, `Slerp3SMatrices`) |
| `animfile.rs` | Animation.pas (`LoadAni`, `SpeedButton9Click`) | `.m3a` animation files (versions 1 to 5, keyframe formula blocks in both layouts) incl. preview images; the `.m3k` text format |
| `frames.rs` | Animation.pas, Mand.pas (`DoSaveAniImage`, `AniFileAlreadyExists`, `OccupyDFile`) | frame files, claiming/locking, image scale, BMP output, keyframe previews (`RenderPrevBMP`) |
| `batch.rs` | BatchForm.pas | batch lists |
| `voxel.rs` | VoxelExport.pas | voxel slice stacks (`TVoxelExportCalcThread`, object test, in-and-outside limits), `.m3v` projects incl. older versions, stack preview |
| `mesh.rs`, `mclut.rs` | BulbTracer2.pas, ObjectScanner2.pas, VertexList.pas, MeshWriter.pas | the DE grid (`TObjectScanner2`), marching cubes with BulbTracer2's tables, vertex merging, centring, OBJ and PLY writers; STL and Taubin smoothing (new) |
| `formulas.rs` (Aexion C) | formulas.pas | the internal formula Aexion C (`HybridAexionC`, translated from x87 assembler) with all its modes |
| `mutagen.rs` | mutagen/MutaGen.pas, MutaGenGUI.pas, PreviewRenderer.pas, FormulaNames.pas | mutation operators, probing (Sobel and difference coverage), the 15-member tree and its layout, formula categories |
| `jit.rs` | paxCompiler (JIT formulas), formulas/JIT*.m3f | a compiler for the Delphi subset of `[SOURCE]` formulas: preprocessor (options and constants), Delphi typing, `Math`/`System` functions, compiled to closures over the iteration state |
| `iteration.rs` (interpolation) | formulas.pas | `doInterpolHybridPas`, `doInterpolHybridPasDE` and the 4D variants; `doHybrid4DDEPas` |
| `calc.rs` (2D, colour on iteration) | CalcThread2D.pas, Calc.pas | `T2DcalcThread` (plane at Z start, middle or end), `doColorOnIt` |
| `scene.rs`, `render.rs` (stereo) | HeaderTrafos.pas (`StereoChange`, `CalcXoff`) | the stereo eyes: shifted middle and off-axis view centre; left and right eye images combined (an addition) |
| `reflect.rs` | CalcSR.pas, ImageProcess.pas (`NormalsOnZbuf`), Calc.pas (`CalcHS`, `CalcHSsoft`), CalcAmbShadowDE.pas (`CalcAmbShadowDEfor1pos`) | reflections and transparency of the normal renderer, normals on the z-buffer |
| `lighting.rs` (`shade`) | PaintThread.pas | `CalcPixelColorSvec` and `CalcPixelColorSvecTrans` (the painter for any view ray) |
| `mc.rs` | CalcMonteCarlo.pas, MonteCarloForm.pas, PaintThread.pas (`PaintMC`) | the Monte Carlo renderer: `CalcRay`, `CalcHSMC`, `CalcPhongLight`, `CalcVisLights`, `CalcBGLight`, `CalcN`, `DoDOF`, `CalcBokeh`, Halton sequences; `CalcAvrgNoise`, `.m3c` files; painting in Lab space |

Tests check that the translated assembler matches the reference spherical
triplex formulas for all integer powers, that renders are deterministic and
independent of the thread count, that every preset renders, and that the DEs
are sane. Animation tests check the frame schedule, the interpolation (first
and last frames equal their keyframes, logarithmic zoom, slerped rotations
and light directions, fading lights, angles the short way), the file
formats and the output claiming. Phase 8 tests cut voxel slices through the
bulb, round-trip `.m3v` files, check the marching-cubes tables, that an
analytic sphere becomes a closed, round mesh with outward normals, that a
bulb mesh is found, that Aexion C matches MB3D's Pascal reference, and that
mutations, the coverage measures and a whole generation work.

## Custom formulas (`.m3f`)

MB3D's 460 formula files store their iteration step as 32-bit x86 machine
code. This port runs that code in three tiers:

1. **Translated to Rust** (`src/x86/native/`, generated): 441 formulas. Their
   machine code was translated ahead of time into ordinary Rust functions,
   which the Rust compiler turns into native code for whatever CPU the crate
   is built for (x86-64, ARM64, …); nothing in it is specific to Intel CPUs.
   At run time a formula is recognised by a hash of its code bytes.
2. **Compiled micro ops** (`x86.rs`): the same analysis, executed by a small
   loop. Used for formula files that have no translation (for example new
   files added later), or with `MB3D_NO_NATIVE=1`.
3. **Interpreter** (`x86.rs`): an IA-32 emulator (integer, x87, SSE/SSE2
   subset) for code that does not fit the static model (a handful of formula files).

Formulas new in this program (`M3Formulas-rs/`, copied into `M3Formulas`
by the release workflow; so far **SphereTree**, TGlad's sphere tree, and
**UlamSpiral**, Ulam's prime spiral in 3D, **PseudoKleinian**,
knighty's pseudo Kleinian, and **DomeTree**, TGlad's dome tree) are
written in Rust and WGSL (`src/x86/handwritten.rs`). Their `.m3f` file has
the options and a placeholder `[CODE]` that only stores a huge `Rout` (the
original MB3D loads the formula and shows nothing) and carries the
formula's name in `mov eax, imm32` instructions. The program of that code
is recognised by its hash, like the translated formulas, and runs the Rust
function as a `DifsFn`; on the GPU the hand-written WGSL takes the place
of the lifter's translation (same record cells and constants). SphereTree
reads PackRatio (pConst−16), Rotated (−24), Levels (−20) and Colour scale
(−32), computes the whole tree in one call (the dIFS loop's minimum over
iterations would otherwise give the outer sphere), writes the distance
times the inverse scale to `Rout` and a per-branch colour value (a hash of
the lattice cell at each level, the first level weighted most) to
`dFree1`, the orbit trap of dIFS formulas.

UlamSpiral reads Mode (pConst−12), Rings (−16), Height (−24), Gap (−32) and
Colour scale (−40). The numbers are on unit cells: the square spiral by
its closed form (`ulam_number`), mode 3 on cube shells (`ulam_number_3d`:
shell k after (2k−1)³, its bottom face in square-spiral order, the ring of
each layer, its top face). Each shape stays inside its cell, so the
shapes beyond the cells next to the point are at least half a cell away:
the distance is the minimum over the 4 nearest cells (8 in mode 3),
capped at 0.5, and bounded by the highest possible shape (the divisor
count is at most 2√n; the skyline grows with its logarithm). A distance
cut at the cell border instead would stop the marcher there (false hits).
The colour value is the logarithm of the divisor count (trial division).

PseudoKleinian is the first of them that is a whole fractal rather than a
shape: it is still a dIFS formula (DE option 20) because it returns a
distance of its own, knighty's `Thing2`: per level `p = 2 clamp(p, -CSize,
CSize) - p`, `k = max(Size / |p|², 1)`, `p *= k` (the scale accumulates
k), `p += Julia`, stopping when p no longer changes; then
`|0.5 d(p) / scale - DEoffset|` with d the distance to the floor plane
z = Offset.z (or the cylinder-plane or rounded box). Its 15 options are
laid out with the two integers first (pConst−12, −16) so that every
double sits on an 8-byte cell, which the GPU constants need (cells 29
down to 17). The colour value is the smallest |p|² over the levels, as a
radius.

DomeTree reads Levels (pConst−12), Children (−16), Bend angle (−24),
Colour scale (−32) and Start curved (−36). Per level the point is turned
into the first of n sectors, inverted out of the sector's circle, and
either scaled up by 1/minr when it is inside the child dome on the axis
(the next level starts by bending its floor into a dome) or inverted in
the circle of radius L4; the distance is the height over the scale, as
in TGlad's `DomeTree.frag`. The GPU's double precision shader has no
trigonometry, so the WGSL takes the constants per child count (r, l, L4
and the sector's cosine and sine) from a table written by
`dome_tree_table`, the bend's in single precision, and turns the point
into its sector by k multiplications (the sector k from a single
precision `atan2`). The JIT leaves the formulas of this file alone: their
code is only the placeholder (`clif_jit::compile_difs` checks
`handwritten::lookup_difs`).


All tiers emulate the flat 32-bit address space with the `TIteration3Dext`
record, the option and constant buffer and a stack, using the same memory
layout and calling convention as MB3D. The translated plain formulas whose
memory accesses the lifting analysis resolves also have a *direct* version
that works on the iteration state itself (`Iteration`) instead of copying
the record through the emulated memory for every call (`DirectFn`). The
same was tried for the dIFS formulas (a version on the iteration state
used by the hybrid loop): it rendered the 80 example files
pixel-identically but no faster (ApolloBalloons 83 s either way), the
loop's cost being its control logic and the `Rout / VaryScale` division on
the critical path, not the copying; it was not kept.

Two shortcuts in the translated code give the same results with less work.
Both concern the symmetry IFS formulas (Apollo2D-IFS, the PolyFold IFS,
discoballIFS and a few more), which turn a point by a multiple of a
sector's angle:

* The sector is `frndint` of the point's angle (`fpatan`) after a few
  scaling steps (negation, multiplication, division by and addition of
  other values). The translator finds such chains in a block; the code then
  takes an interval around a fast approximation of the angle
  (`atan2_bracket`, within 1e-5 of it) through the same steps. Each step is
  monotonic, so if both ends round to the same integer the exact angle does
  too; otherwise (near a sector border) it calls `atan2`. `M3F2RS_EXACT_ATAN=1`
  at translation time turns this off.
* The sine and cosine of the sector's angle (`fsincos` of a value made from
  rounded values, integers and parameters) are remembered per thread by the
  exact bits of the argument (`Machine::sin_cos_cached`).

ApolloBalloons on the CPU: main calculation 36.5 s → 31.7 s, reflections
and occlusion 45.0 s → 40.3 s (MB3D under Wine: 30.8 s and 41.4 s); the 80
example files render byte-identically.

The translation and the micro ops rely on the x87 register stack depth being
known at every instruction. Where code joins with different depths (for
example paths for invalid option values, or helper routines called at
different depths), the register file is rotated exactly as a different x87
TOP pointer would, so behaviour matches the CPU on every path.

Speed on the 2-core test machine, 120-pixel previews of the three test scenes
(volumetric light off), compared with phase 4: City 35.2 s → 4.6 s, Eagle
18.3 s → 5.4 s, Electric 28.0 s → 5.2 s. The images are pixel-identical.
Formulas with very short code (Eagle) gain less, because the cost of passing
the iteration state in and out of the emulated memory remains.

Verification: a small native 32-bit program ran the same machine code on
the real CPU from the same memory image, and the results were compared with
the port's over random inputs for every formula (the program and that mode
of `m3fcheck` were removed from the repository in 0.2.44; they are in its
history).
All 449 testable formulas, dIFS formulas included, agreed with the CPU to
1e-9 relative, in every tier (the check ran the translated code by default,
`MB3D_NO_NATIVE=1` checks the micro ops). The only differences are in the
last bits, because the x87 computes with 80-bit precision and the port uses
f64. Not checked by the oracle: the formulas compiled from Pascal source
(`[SOURCE]`, no machine code; see below) and the formulas that read image
maps through `PMapFunc` (the oracle has no maps; they work in the renderer).

Formulas written in Pascal (`[SOURCE]`, compiled by paxCompiler in MB3D) are
compiled by `jit.rs` from the Delphi subset they use; all 40 of the MB3D
repository compile, and the tests compare two of them with equivalent
formulas.

Formulas are looked up in `M3Formulas` next to the program, and also in
`--formulas DIR`, `$MB3D_FORMULAS` and `M3Formulas` in the current folder.
After those come `M3Formulas` in the folder of a loaded parameter file or
the folder above it (MB3D keeps `M3Parameter` next to `M3Formulas`) and,
last, `~/mb3d-rs/M3Formulas`, where `download.sh` installs the release.

## The swarm (`swarm.rs`)

Rendering shared between computers through a shared folder; no sockets,
so the program stays without network code (the user's choice, for the
attack surface and the repository scanners). Everything is files:

| path in the swarm folder | content |
|---|---|
| `peers/<id>.txt` | a heartbeat per running instance (name, version, cores, card, state, time), written every 4 s while a unit is calculated and when idle; stale after 30 s, removed after 300 s |
| `jobs/<id>/job.txt` | the job: `kind = animation | batch | montecarlo | tiles`, name, owner, settings, the unit output names |
| `jobs/<id>/in/` | the inputs: `anim.m3k` (the keyframes written inline), the parameter files and `list.txt` of a batch, `scene.m3s` of a Monte Carlo render; `M3Formulas/` and `M3Maps/` with the custom formulas and maps the scenes use (`copy_scene_files`) |
| `jobs/<id>/out/` | one output file per unit; a tile is `tile_<k>.m3i` (its G-buffer, with the shadows and occlusion calculated with the tile's border as a single tile of `render_tiled`) plus `tile_<k>.png` for the previews |
| `jobs/<id>/done` | written by the owner when the results are collected (the folder is removed unless kept) |

Units are claimed with a marker next to the output (`claim_unit`:
`<output>.claim`, opened with `create_new` and holding the claimer's
heartbeat id): an exclusive create is atomic on the server whether a
computer reaches the folder directly or over the network. Tiles and
Monte Carlo blocks are written under a
temporary name and renamed into place (`write_atomic`), frames and batch
pictures by `frames`; the marker is removed after that (`UnitClaim`'s
drop), so a unit is done when its output is there and its marker is not
(`is_done_file`). A marker whose heartbeat is stale or missing (an empty
one by its age) is taken over: renamed to a name of the taker's first,
so only one computer takes it, and a half written output is removed.
Each program keeps its heartbeat going from a thread (`Presence::get`),
so a long step without progress does not look like a computer gone. The
owner (`run_animation`,
`run_batch`, `run_montecarlo`) creates the job, takes part through the
same `work_on` as a worker, and polls every 5 s until all unit files are
non-empty, then copies the frames or pictures to the user's output or
merges the Monte Carlo blocks. A Monte Carlo block k renders the whole
picture with `McImage::seed_salt = (k + 1) * 0x9E37`, which is XORed into
every ray's seed (`Tracer::pixel`), until it has the block's share of the
rays (`--rays` divided by the blocks, default two blocks per instance);
`McImage::merge` averages the records weighted by their ray counts and
adds the pass counts. The worker (`mb3d swarm worker DIR`) loops over the
jobs oldest first, adds a job's `M3Formulas`/`M3Maps` to the search paths
once, and sleeps 2 s when nothing is left; `--once` makes one round (the
tests use it). `mb3d swarm list DIR` prints the peers and the jobs.

The Swarm window (`app/swarm_form.rs`, `forms/Swarm.dfm`, the `Swarm`
button on the main window's Tools tab) keeps the folder, this computer's
name, the thread count and the tiles in the ini (`SwarmFolder`,
`SwarmName`, `SwarmThreads`, `SwarmTiles`; sharing is never on at the
start). *Share this computer* is the one switch: it runs
`swarm::worker_controlled` in a thread, and while it is on this
computer's renders go to the swarm when another computer of this version
is in it (`swarm::others_online`). The parts of one program (the worker,
the renders that made jobs) share one heartbeat (`Presence`), so a
computer is listed once, and the worker leaves its own program's jobs to
the part that made them. Switching sharing off sets `stop`: the unit
under way is finished and no further one taken; switching it on again
meanwhile keeps that worker. A timer refreshes the two lists every 2 s
from the heartbeat and job files, and the status line says with whom
this computer shares. The Monte Carlo window's start (`mc_form::start`)
runs `swarm::run_montecarlo_live` in its thread instead of the pass
loop. Without a rays target (the window's normal, open-ended render) the
job is open-ended: every computer renders one block until the owner
writes `stop` (the window's Stop), saving a snapshot (`<unit>.snap`)
every 30 s that the owner merges into the live picture (`BlockMerge`);
after Stop the owner waits up to 2 minutes for the blocks under way and
merges the saved blocks, or a block's last snapshot when its computer
went away. In batch mode the blocks stop at the batch's ray count. The
owner's waiting reports arrive through `Shared::status`, and the merged
image through the usual `Shared::img`. Calculate 3D does not start a
picture that does not fit into the free memory (`render::check_memory`,
80 bytes per pixel and 300 MB against `MemAvailable`), and a worker
leaves a tile that does not fit to the others.

Calculate 3D with the swarm is `engine::Job::Swarm`: the engine thread
runs `swarm::run_tiles` (the picture in the window's tiles, a `tiles`
job), publishes the preview the owner gets while it waits (`job_preview`:
the tiles' pictures put together, units under way as dimmed rectangles
with the progress their worker's heartbeat says, the rest dark), and
keeps the stitched G-buffer as a calculated one (`Base`), so lighting
changes, post processing and saving work as after a local calculation.
The Batch window goes through the same path, picture by picture. The
Animation maker's thread calls `swarm::run_animation` for all frames
when its switch is on, and reads the frame count from the owner's
waiting reports. The Swarm window shows `job_preview` of the newest job
every 2 s. Pictures by the swarm are not bit-identical to local ones
only where a tile's border changes an image-space effect, as with
MB3D's big-render tiles.

Robustness: `job.txt` carries the owner's heartbeat id and the program
version. A worker removes a job whose owner has sent no heartbeat for
`ORPHAN_SECS` (30 min; the job must be that old too), skips jobs of
another version (noted once in `WorkerControl::note`, shown in the
window's status line), and when a unit fails it writes
`errors/<peer id>.txt` (peer name, message), gives that job up for this
process and goes on; the owner fails its run as soon as every helper of
its version has an error noted, with that message. An animation job
carries the frame list (`frames`), so the Animation maker's and
`animate`'s selections apply. `WorkerControl` has `stop` (no further
unit, the current one is finished) and `abort` (the current one is given
up): the window's switch uses `stop`, closing the program `abort`.

## Scene files (`.m3s`)

The INI-style format holds global keys, then optional `[formula]` sections (up
to 6, forming an alternating hybrid) and `[light]` sections (up to 6). See
`examples/`. Any key can be overridden on the command line with
`-s key=value`. `Scene::to_text()` writes the format, and `.m3p` → `.m3s` →
`.m3p` keeps every setting the port uses (checked over the parameter
collection). In `[light]` sections, `slot = N` keeps MB3D's light numbering,
`lights = none` turns all lights off, and formula options whose names are
not valid keys are written as `optionN = value ; name`.

The light values use MB3D's slider units (for example `diffuse = 50` means
1.0, `dyn_fog = 53` means off), so the binary `.m3p` light record can be mapped
onto them directly in phase 2. `color_start`/`color_end` are the colour-range
sliders (`TBpos[9]`, `TBpos[10]`). `--auto-color` fits them to the rendered
surface; this convenience is not in MB3D.

## Ambient occlusion and shadows

Like MB3D, both run on the G-buffer after the main calculation (as with the
"calculate automatically" options), and the painter then applies them.

| key | values | MB3D |
|---|---|---|
| `ao` | `off`, `ssao24` (default), `ssao24t0`, `ssao15`, `ssao15t0`, `deao` | ambient shadow, "SSAO 24 bit" / "threshold to 0" / "15 bit" / "DEAO" |
| `ao_random` | 0 = off, 1..n passes | random 24 bit SSAO with n accumulated passes |
| `deao_quality` | 0..3 (3, 7, 17, 33 rays) | DEAO quality |
| `deao_dither` | 0, 1 (2×2), 2 (3×3) | DEAO dithering |
| `deao_max_len` | default 1 | DEAO max. ray length |
| `ao_threshold` | z/r threshold, default 2 | `sAmbShadowThreshold` |
| `ao_border` | 0..0.9 | border mirror size |
| `shadows` | `off` (default), `all`, or light numbers like `1,3` | hard shadows for the selected lights |
| `shadow_soft` | `true`/`false` | "1 soft HS" (uses the last selected light) |
| `shadow_soft_radius` | 0.01..20 | soft shadow radius |
| `shadow_max_len` | default 1 | max. shadow length multiplier |
| `shadow_set_cos` | `true`/`false` | set the diffuse function of shadowed lights to cos |

```sh
./mb3d examples/mandelbulb.m3s -s shadows=1 -s shadow_soft=true -o soft.png
```

`.m3p` files bring their own settings. Shadows are only calculated when the
file has "calculate automatically" set. Hard shadows run the DE ray march
again for every pixel and light, so they cost about as much as the image
itself per light.

## Lights, volumetric light, cutting planes, depth of field

| key | values | MB3D |
|---|---|---|
| `position` (in `[light]`) | x, y, z | positional light at this world position |
| `visible` (in `[light]`) | 0 = no, 1..4 = MB3D's visible light shapes | visible light source |
| `shadow` (in `[light]`) | `true`/`false` | light uses hard shadows |
| `vol_light` | `off` or light number 1-6 | volumetric light of that light (shown with the dynamic fog colours, `dyn_fog` sets the amount) |
| `vol_light_map_size` | -7..7 | size of the light's shadow map in 20 % steps |
| `cut_x`, `cut_y`, `cut_z` | position or `off` | cutting planes; the side away from the camera is kept |
| `dof` | `off`, `sorted`, `forward` | depth of field |
| `dof_focus`, `dof_focus2` | distance as fraction of the image width | sharp range (`--stats` prints the value for the image centre) |
| `dof_aperture` | 0.0001..2 | aperture |
| `dof_max_radius` | pixels | maximum blur radius |
| `dof_passes` | 1..4 | passes |

```sh
./mb3d examples/mandelbulb.m3s -s dof=sorted -s dof_focus=0.61 -s dof_aperture=0.3 -o dof.png
./mb3d examples/mandelbulb.m3s -s cut_y=-0.2 -o cut.png
```

The volumetric light map is calculated before the image, with a ray march
per map pixel, so it adds noticeable time.

**Changing the lighting after a calculation** (the Lighting window,
`light_form.rs`: `trigger_repaint`, `idle`; `engine.rs`: `Job::Relight`)
repaints at once from the kept G-buffer, as MB3D does: the hard shadows as
they were calculated, without reflections and depth of field. Only what
casts the shadows counts for them (`post_sig`: on, shadowing, direction or
position, a positional light's reach), so colour, intensity and shading
never calculate them again. When the lighting has not changed for 1.5 s,
a full repaint adds the reflections, the DoF and the shadows of a moved
light (the line says "reflections next" / "shadows next", then "adding
reflections" / "redoing shadows"); the next change stops it.

## Reflections and transparency, normals on the z-buffer

| key | values | MB3D |
|---|---|---|
| `mc_reflections` | `true`/`false` | "Calculate reflections" (post processing, and in the Monte Carlo renderer) |
| `mc_reflection_depth` | 0.. | reflections of reflections |
| `mc_reflection_amount` | 0..100 (0..1 realistic) | amount of reflected light |
| `mc_transparency` | `true`/`false` | transparency: the alpha of the palette's specular colours (`palette_alpha`) |
| `mc_only_difs` | `true`/`false` | only dIFS formulas are transparent |
| `mc_refraction_index`, `mc_absorption`, `mc_scattering` | | refraction, absorption and light scattering inside transparent material |
| `normals_on_zbuf` | `true`/`false` | "Normals on Z-buffer": normals from the positions of the neighbour pixels |

```sh
./mb3d "material colors.m3p" -o glass.png
./mb3d examples/mandelbulb.m3s -s cut_y=-0.6 -s mc_reflections=true -s mc_reflection_amount=0.8 -o mirror.png
```

Like in MB3D these run after the calculation: the normals on the z-buffer
before shadows and ambient occlusion, the reflections after painting (and
before the depth of field). For every object pixel the view ray is
reflected, or refracted into transparent material with Fresnel reflection,
marched to the next surface, which gets its own normal, colour, hard or
soft shadow and DE ambient occlusion and is lit like a painted pixel. The
specular colours of the palette say how much light a surface reflects.

## Hybrids, inside rendering, maps

| key | values | MB3D |
|---|---|---|
| `de_combination` | `min`, `max`, `max_inverted`, `smooth_linear`, `smooth`, `mix` | DE combination of hybrid part 1 (formulas 1..`decomb_end1`) and part 2 |
| `decomb_end1`, `decomb_start2`, `decomb_end2`, `decomb_repeat2` | formula slots 1..6 | the two parts |
| `decomb_iterations2`, `decomb_smooth`, `decomb_mix_pow`, `decomb_mix_color` | | part 2 iterations, smoothing, mix options |
| `inside` | `outside`, `inside`, `both` | inside rendering, in-and-outside |
| `background_image` | file name | background picture (sphere or `background_direct = true` image coordinates) |
| `background_rotation`, `background_brightness`, `background_add_light`, `background_ambient` | | picture rotation (bytes, 128 = half turn), brightness 1.04^(v-40), add instead of fog, small blurred copy as ambient light |
| `map`, `map_rotation` (in `[light]`) | map number, 3 bytes | light map: an image lighting from all directions |
| `diffuse_map` | map number | diffuse colour map |
| `diffuse_map_mode` | `iterations_otrap`, `normals`, `wrap_sine`, `wrap` | how the map is placed |
| `diffuse_map_offset`, `diffuse_map_rotation`, `diffuse_map_scale`, `diffuse_map_brightness_only` | | placement and "Y + colour" combination |
| `internal_gamma2` | `true`/`false` | light calculation in squared colour space |
| `ambient_relative_to_object`, `dynfog_options`, `light_mode`, `color_interpolation` | | further light options of MB3D's lighting editor |
| `tiles`, `tile`, `tile_downscale` | `4x3`, `col, row` (1-based), 1..3 | tiled rendering (also `--tiles`, `--tile`) |

Map numbers refer to the files of MB3D's `M3Maps` folder (`12.png`,
`129 name.jpg`, ...). Map formulas (height maps, map transforms) find their
images the same way. A missing image is reported and the feature is skipped.

Tiled rendering: with all tiles, the tiles are calculated one after another
into one G-buffer and SSAO, painting and DOF run on the whole image, so the
result equals an untiled render. A single tile is calculated with a border of
24 pixels for the image space effects, like MB3D's big-render tiles, and the
tile files that MB3D writes for big renders are rendered as that tile.

## Output: 16 bit pictures, noise filter

16 bit colour (`mb3d --bits 16`, single pictures; the window's *PNG 16 bit
colour* file type): painting, the reflection pass and the depth of field
are generic over the sample type (`lighting::Sample`, `u8` and `u16`;
`render::paint16`). They calculate in `f32` and round only at the end of
each stage, every stage as MB3D does for 8 bit (`from_paint`, `from_unit`,
`from_dof`), so the 8 bit pictures are bit for bit as before; 16 bit keeps
the value times 257. A 16 bit picture rounded to 8 bit differs from the
8 bit one by at most 1 (about 3 to 5 % of the values: 8 bit rounds between
the stages). The window paints the 16 bit picture again from the kept
G-buffer when it is saved (with the reflections, they are marched again),
reduced by the viewing scale like the 8 bit one. `png::encode_rgb16`
writes it.

The Open m3p / m3i dialogs have a preview column (`FileOptions::preview`;
`app::file_preview`): a clicked (or arrow-key selected) `.m3p`, `.m3i` or
`.txt` file goes through `Ui::file_preview` to the application, which
renders it with `mutagen::preview` (at most 240 x 180, on the CPU via
`gpu::cpu_in_thread`, all cores but one) in a background thread, one at a
time, the last clicked file next; results (or the reason there is none)
are kept per path for the session.

Transparent backgrounds (`frames::rgba_of`, `png::encode_rgba`): an 8 bit
RGBA PNG with straight alpha. A pixel's opacity is the share of object
pixels (G-buffer not background) in its anti-aliasing block (`--aa`, the
viewing scale, the animation's scale), its colour the mean of only those
object pixels, so no fringe of the background is left; fully transparent
pixels keep the background's colour. Background effects (volumetric light,
fog over the background) are dropped with the background. `mb3d
--transparent` (single pictures and tiles; not with `--bits 16` or
`--stereo-pair`), `mb3d animate --transparent` (PNG frames; WebM gets VP9
`yuva420p` with alpha, MP4 has none), Save pic > PNG transparent
background, the Animation maker's Transparent background (`AniTransparent`
in the ini).

The noise filter (`src/denoise.rs`) is the DCT image denoising of
G. Yu and G. Sapiro (IPOL 2011, doi:10.5201/ipol.2011.ys-dct), the
algorithm of GIMP's *Denoise DCT* (GEGL `denoise-dct`), implemented here
from the paper: every 8x8 patch (all overlapping positions) of each of R,
G and B is transformed with the orthonormal 2D DCT, coefficients below
3 sigma are set to zero (the mean stays), the patch is transformed back,
and each pixel is the mean of the patches that cover it. As in GIMP the
channels are filtered separately; the paper's colour transform was tried
and made the results slightly worse and unlike GIMP's. Against GIMP's own
output (`gegl ... -- gegl:denoise-dct sigma=S`) the results differ by
0.42 levels on average (rounding). The image is split into bands of rows,
each band computing all patches that overlap it, so threads never share
pixels. It filters 8 bit pictures: `mb3d --denoise S`, `mb3d montecarlo
--denoise [S]`, the Monte Carlo window's *-noise* and Post processing ▸
Noise Filter (*Filter now*, *Filter new pictures automatically*, applied
in `main_form::show_image`, so the saved picture is the filtered one;
16 bit saves are painted again and not filtered).

The choice was measured on Monte Carlo pictures against 512 (256) ray
references, RMSE of the 8 bit pictures (TreePlanet at 0.3 size, the
default Mandelbulb 480x360, rays per pixel):

| filter (GIMP 3.2 / GEGL 0.4.72) | Tree 8 | Tree 16 | Tree 32 | Mand 4 | Mand 8 | Mand 16 | Mand 32 |
|---|---|---|---|---|---|---|---|
| none | 6.20 | 4.54 | 3.06 | 9.46 | 4.29 | 2.81 | 1.84 |
| Denoise DCT, sigma 4 | 5.97 | 4.40 | 3.06 | 9.28 | 4.11 | 2.76 | 1.95 |
| Denoise DCT, sigma 8 | 5.75 | 4.58 | 3.76 | 8.55 | 4.11 | 3.22 | 2.80 |
| Noise Reduction (default, 4 iterations) | 10.33 | 10.15 | 9.99 | 10.88 | 10.15 | 10.00 | 9.92 |
| Smooth by Domain Transform | 5.80 | 4.53 | 3.54 | 8.79 | 4.28 | 3.30 | 2.77 |
| MB3D's -noise (4-neighbour mean) | 6.99 | 6.57 | 6.26 | 7.68 | 6.03 | 5.73 | 5.59 |

Selective Gaussian Blur, Median, Mean Curvature Blur and Symmetric Nearest
Neighbour were all worse than none from 8 rays on.

A self-made variance and normal/depth guided a-trous filter (SVGF style)
was worse than none from 8 rays on and was dropped. For Monte Carlo the
strength is `mc::noise_sigma` = 64 / average rays per pixel, at most 12:
over four scenes (also "cutted sphere over carpet" and "julia on box",
the latter not used to choose the rule) the best strength followed the
ray count (about 12 at 4 rays, 8 at 8, 4 at 16, 2 at 32) much more than
an estimate from each pixel's ray spread, which missed by far on the
sphere scene (RMSE 12.4 -> 31.8). With the rule all 13 cases improve:
-16 to -23 % at 4 rays, -5 to -10 % at 8, -2 to -5 % at 16, -1 to -3 %
at 32.

Licences: GEGL's operations are LGPL-3.0-or-later; MB3D's core is LGPL 2.1
(its sources state no "or later"), so GEGL code is not used: the filter
is written from the published algorithm only.

## Known differences and gaps

* **Coverage:** all of MB3D's rendering modes are ported (with phase 9:
  JIT formulas, interpolation hybrids, 2D slices, stereo, the reflection
  and z-buffer normal post processing and the Monte Carlo renderer), and all
  80 example parameter files render. The Mandelbulb3D program has MB3D's
  windows. Not ported: the script editor (not reachable from MB3D's main
  window either), the "shortdistance check DE" option, MB3D's random light
  strokes and the ambient light pass of the post processing window. The
  BulbTracer2 cache format (`.btr2cache`) is not written, and its OpenGL
  auto preview of the mesh is not available (the voxel preview is).
* **Monte Carlo:** the random numbers are seeded per ray and pass, so an
  image is the same with any number of threads and with or without the
  graphics card (up to its precision), but not MB3D's; neighbour
  pixels steer the ray counts from their state at the start of a pass (MB3D
  reads them while other threads change them). The z-buffer that the Monte
  Carlo window saves with "With ZBuffer" comes from a normal calculation of
  the same parameters. MB3D's development record format (`MCoptions` bit 8)
  is not read.
* **Exports and MutaGen:** the mesh export reads but does not reproduce
  BulbTracer2's floating-point order exactly, so meshes match MB3D's in shape,
  not vertex for vertex. MutaGen uses its own random generator, so
  a seed gives other mutations than in MB3D; mutating map or light settings
  (not in MB3D's MutaGen either) is not done.
* **Animation:** Map sequences (`Mandelbulb3DMSeq.ini` in the working folder or
  `$MB3D_MAP_SEQUENCES`) follow the file index of animation frames. Without a loop, MB3D's render loop also counts the sub-frames of
  the last keyframe (repeating it); the port renders the last keyframe once,
  as MB3D's own frame count (`TotalBMPsToRender`) and time estimate do.
  MB3D's per-frame `.m3p` files carry the keyframe's light settings
  unchanged; the port writes the interpolated sliders instead. `mb3d
  batch` writes images (or `.m3p`); the Batch processing window writes
  `.m3i` files with the G-buffer, as MB3D.
* **Speed:** custom formulas run as translated native code (including their
  SSE2 and x87 math) on the iteration state; what remains slower than
  MB3D's machine code is the emulation of the x86 flags and of the few
  values a formula keeps in the emulated memory. The [README](../README.md#speed)
  compares the CPU renderer with MB3D under Wine. Use `--scale` for
  previews; speed scales with the number of cores (and with SMT: 24
  threads on 12 cores are 1.7 times faster than 12, the formula code being
  bound by its dependency chains).
* **Pixel exactness:** the x87 code computed in 80-bit precision; this port
  uses f64, and f32 wherever MB3D stored values as `Single`. Images match in
  structure but are not bit-identical. The per-thread random seed of the
  "first step random" option is deterministic per row instead of using
  Delphi's `Random`. Verifying against MB3D needs reference renders made with
  the original program.
* Points exactly on the z axis are degenerate in MB3D's triplex formulas
  (division by `x² + y²`). This is kept as in the original.

## Videos

`src/video.rs` writes animation frames into an AVI file (RIFF, one Motion
JPEG stream): PNG and BMP frames are encoded with the program's own JPEG
encoder at quality 92, JPEG frames are copied as they are; the index
(`idx1`) and the sizes are completed at the end. AVI 1.0's sizes are 32
bit, so a video stops before 4 GB (about 10000 frames of 1920x1080) with
a message pointing to the ffmpeg command. `mb3d animate --video FILE.avi
--fps N`, and the Animation maker's *Also make a video (AVI)*. A
transparent background is not kept (AVI has none).

`video::ffmpeg_command` gives the command for a small video of the frames
(`-c:v libx264 -crf 18 -pix_fmt yuv420p` MP4; with transparent PNG frames
a VP9 WebM with `yuva420p`), with `-start_number` of the first frame, or
a `-pattern_type glob` input when the file index steps by more than 1.
`mb3d animate` prints it at the end; the Animation maker's *FFmpeg
command* copies it to the clipboard. The program itself starts no other
programs (until 0.2.42 it ran ffmpeg, which code scanners flag).

## Time left and the progress bar

The main window's bar shows the whole 3D calculation, and the line beside
it the stage and the time left in total (`main_form.rs`: `time_left`,
`stage_caption`).

- **The main rendering's time left** comes from its own progress: the
  rate of the last third of its time (at least 5 s), or, for a picture
  calculated again, the progress curve of the last time at the pace of
  this one.
- **The later stages** (hard shadows and ambient occlusion together,
  reflections, depth of field) are expected at a ratio to the whole main
  rendering's time. The ratios come from a model (`src/app/eta.rs`,
  weights in `eta_model.rs`): 25 features of the scene and the computer
  (picture size, threads, graphics card, the shadow, occlusion, reflection
  and DOF settings, from the rows marched so far the part on a surface,
  the DE steps per pixel and the pixels per second, and whether the CPU
  did the main rendering alone while the card's shaders compiled) into
  five small networks (25 → 16 tanh → 3 log ratios), averaged. It was
  fitted on 1418 renders of 366 scenes, 270 of them compiling shaders; on
  scenes it had not seen, the later stages' time at the end of the main
  rendering was off by a median 0.7 % of the whole render (90 %: 8 %),
  against 20 % (29 %) for the fixed ratios before and 1.1 % (12 %) for a
  linear fit. A picture calculated again expects its stages' times of the
  last time instead.
- **The bar** is the time done over the time done plus the time left
  (smoothed over a few seconds as the caption); before the time left is
  known, the main rendering's part by the expected ratios. It never goes
  back.
- **Shader compiles**: while the graphics card's shaders compile, the CPU
  marches alone. The time left then comes from the CPU threads' pace,
  unless the shaders are predicted done clearly earlier (by half again):
  then the card joins and the rest runs about 3 times faster
  (`compiling_estimate`). A shader job's compile time is predicted from
  its size, precision, constants and how many compile at once
  (`eta::compile_left`; median error 28 % for jobs over a second). On the
  compiling renders of the data the total time left was off by a median
  1–4 % of the render's time; when the card joins part-way, the estimate
  is too high until the shaders are done (median 45 %). After them the
  main rendering's time is taken at the pace since; a main rendering the
  CPU finished alone gives the later stages the model's CPU-alone ratios.
  A stage that ran while shaders compiled is not kept as the time of the
  next render.
- `MB3D_ETA_LOG=FILE` appends a line per finished calculation (features,
  stage times, whether shaders compiled), `MB3D_COMPILE_LOG=FILE` a line
  per shader compile: the data the model is fitted on.

## Unfinished renders (`checkpoint.rs`)

A render keeps what it calculated in a checkpoint, a folder
`resume/<kind>-<hash>` in the cache folder (`MB3D_RESUME_DIR` sets
another), never in a swarm folder. The name is the key: an FNV-1a hash of
the kind, the program version and the scene text (`Scene::to_text`, the
text the swarm hands on, with `threads` cleared) plus where it is
calculated (`cpu`, `auto`, `card`). For a calculation (`calc`) only what
its rows depend on counts (`render::march_sig`: not the lighting unless
the volumetric light uses it, not the shadows, occlusion, reflections or
DoF), so a stopped render changed only in those still continues; rows the
card shadowed during the march carry their shadows' settings, checked when
continuing. Monte Carlo, tiles and voxels keep the whole scene. Running the
same render again finds it; the Unfinished renders window loads the scene text it kept
(`extra-scene.m3s`) and gets the same key back.

| file | content |
|---|---|
| `data.bin` | the records of all pixels, row major (Monte Carlo, voxels, the swarm) |
| `image.m3i` | the same for a picture or a big render, as a `.m3i` image file: the file's header before the records, the rest of the parameters after them, the rows not done as background (`checkpoint::Image`). A valid image at every moment: copy it out, open it in this program or in MB3D (`Entry::image_file`, "Save m3i" in the Unfinished renders window) |
| `marks.bin` | 28 bytes per unit (a row, a tile): magic, FNV hash of the unit's data, generation, rectangle |
| `info.txt` | kind, name, size, version, the units done, times; `rows = raw` or `card PostJob {...}` (what the stored rows hold); `secs`, the time the main rendering of a picture took so far (the window's progress and time left go on from it) |
| `extra-*` | further state, written atomically (`scene.m3s`) |
| `lock` | held (`try_lock`) by the render that uses it: a second process of the same render runs without a checkpoint |

**Writes.** A finished unit is written at its place (`write_at`), then its
mark. Nothing waits for the disk: the system's cache holds the data, so a
killed program leaves everything it finished. Every 5 minutes
(`MB3D_AUTOSAVE_SECS`) the files are flushed (`sync_data`). Should the
computer go off between the two writes, a unit whose data does not match
its mark's hash counts as not done: the files need no ordering on the
disk. Measured: no time difference with and without a checkpoint (CPU
and card, 2400x1800 to 3200x2400). A write error (a full disk) ends the
checkpoint, not the render.

**The G-buffer rows.** `render::calculate_with` takes the checkpoint; the
rows it holds are loaded (and shown) first and skipped:

- CPU: the threads' interleaved rows skip the done ones.
- Auto: the free range `[top, bottom)` of `march_mixed` skips done rows at
  both ends; a band of the card ends before the next done row.
- Card alone: rows marched with the card's post passes are tagged `card
  <job>`. A continued render marches the missing bands with the same
  passes (`gpu::march_bands` with the `PostJob`); rows of a raw
  checkpoint are marched raw and get `post_on_card` on the whole picture,
  as in Auto. Rows tagged otherwise than the path that continues them are
  forgotten (`Store::reset`).

The post calculations (shadows, occlusion), the painting, reflections and
depth of field run after the march as before and are calculated again
when a render continues. CPU rows are bit-identical after a continue
(`pixel_seed` from the picture's coordinates); kill-and-continue tests of
the CPU, the card and Auto, with shadows and DE occlusion, matched the
render in one go byte for byte (Auto with hard shadows differs between two
plain runs already: which device runs the shadows depends on timing).

**Tiles** (`--tiles` without `--tile`): the checkpoint holds the finished
tiles (their G-buffers after the tile's shadows and occlusion), one unit
per tile; the tile under way keeps its rows in a checkpoint of its own
(the key of the tile's scene). SSAO and painting run on the whole picture
after the tiles as before, so a continued render equals one in one go.
One tile (`--tile`) is a calculation of its rectangle with the border.

**Ends.** A finished render deletes its checkpoint after its result is
saved (the command line after the files are written, the program after
the picture is painted). A stopped render keeps it unless it ran for less
than 10 seconds and continued nothing (`MB3D_RESUME_KEEP_SECS`): the
editor's many short calculations leave nothing. Each new checkpoint
prunes those not written for 30 days and then the oldest above 50 GB in
all (`MB3D_RESUME_MAX_GB`), never one that is locked.

**The program.** Calculate 3D looks for an unfinished calculation of the
scene (`engine::unfinished_calc`) and asks whether to continue it (Yes:
continue, No: delete and start again, Cancel: nothing). Stop and the
post calculations and painting after it are cancellable now (they ran to
the end before). The Resume button of the Open tab opens the Unfinished
renders window (`unfinished_form.rs`): Continue loads the kept scene and
continues it without asking, Load parameters only loads it.
`examples/resumecheck.rs` drives this without windows: one go, Stop half
way, continue from the list, Stop, continue through the question;
compares the pictures.
`examples/resumecheck2.rs` adds the answers No and Cancel, Delete and
Delete all, the list kept up to date while rendering, a Monte Carlo
render continued from the list after a restart, and the voxel and Bulb
Tracer windows stopped and continued (compared with one go).

**Monte Carlo** (`mc::McCheckpoint`, `mc::pass_with`). A pass needs the
whole image as it was when the pass began: its noise statistics, and
every pixel's neighbours (`calc_n` reads them from the pass's snapshot).
The checkpoint has two planes of 32-byte records (`McRecord`: colour f32
x 3, the luminance sums f64 x 2, ray count, hit), one unit per row and
plane; a row after pass n goes into plane n % 2 with generation n, so the
other plane still has the row before the pass. On restore, P = the
smallest generation of all rows: the image after P passes is put
together from the planes, and the rows that have P + 1 already are not
calculated in the next pass (`pass_inner`'s skip, on the CPU and the
card) and get their kept records after it. Kill-and-continue at 3, 6, 9
and 12 s of a 4-pass render on the CPU matched the render in one go byte
for byte (on the card two plain runs differ already: the CPU takes a
share of the rays by timing). The key leaves out exposure and saturation
(they only paint the records); a render that starts from an `.m3c` has
that file's hash in its key and writes its first image as a generation.
The Monte Carlo window asks on Start with an empty image; a batch
continues without asking. A render is finished, and its checkpoint goes,
when it reaches its rays (`--rays`, `--passes`, a batch entry's rays);
one that ends on `--time` or Stop stays.

**Frames and batch pictures.** `frames::render_scaled_with` takes a
checkpoint of the picture (its tiles when tiled); `render_frame_to_file`
and `batch::render_item` open it with the frame's or item's file name
and end it after the file is written (the depth picture is written
before the frame now, so a frame counts as done only with it). The
command line's `animate` and `batch` and the Animation window keep one;
the swarm's units too (below). The Batch window's calculations
continue without asking.

**Voxel export.** A checkpoint of one-byte units, one per slice (only
the marks matter): a slice counts as done when it is marked and its file
is there; slices are written atomically. Killed after 280 of 1200
slices, the export went on with the rest; all slices matched.

**The swarm.** Nothing new goes into the swarm folder but a
`resume_key` in `job.txt`; checkpoints stay local, and claims, markers
and stale takeovers are unchanged.

- *The owner* (`swarm::Kept`, kind `swarm`): one one-byte unit per job
  unit (a tile, a frame's eye, a batch picture, a Monte Carlo block of a
  fixed size), and the unit's file copied in (`Store::put_extra_file`;
  a tile with its `.png`) as soon as it is finished in `out/`: in the
  owner's report callback (once a second) and after the job. Stop still
  removes the job, but every finished unit is kept, also those of other
  computers. Run again, the key (the job kind, the scene text without
  threads, the settings, the unit names) finds them: the kept tiles go to
  `collect` first, and the new job has only the missing units (`tiles`,
  `frames`, a batch's list, `blocks` for Monte Carlo; a batch item keeps
  its index in its job file name). All units kept: no job. Monte Carlo
  blocks are merged in block order, kept or not, so the result is the
  same bit for bit. The previews show the kept tiles and blocks.
- *A killed owner* leaves its job in the folder, and the other computers
  go on with it. The owner run again holds the checkpoint's lock, so a
  job with its `resume_key` has nobody else collecting it: its finished
  units are taken (`Kept::adopt`) and the job removed.
- *A unit under way* keeps a checkpoint of its own on the computer that
  renders it (a tile: `open_tiles_checkpoint` of the tile's scene, a frame
  or batch picture as on the command line, a Monte Carlo block of a fixed
  size: `McCheckpoint` with the block's seed salt). It is ended when the
  unit's file is written; when the job is stopped, the computer stops and
  keeps it, its claim goes, and if it gets the unit again (in the new
  job) it continues. On another computer the unit starts anew, with the
  same seeds. Open-ended Monte Carlo blocks keep their snapshots as before.
- Removing a job removes its `job.txt` first; a computer writes no claim
  and no error into a job without it, and folders without one for 2
  minutes are cleared away (`sweep_peers`), as a computer that was
  writing into a job while it was removed could leave one.

Kill tests with an owner and a worker on one folder (owner killed at 40 %
or 45 % of the time alone, run again): tiles (4x3, 1600x1200), Monte Carlo
(6 blocks of 12 rays), an animation of 9 frames and a batch of 6
pictures: the results matched a run in one go byte for byte, and neither
the swarm folder nor the checkpoints kept anything afterwards.

**Bulb Tracer meshes** (`mesh::MeshCheckpoint`, `mesh::trace_with`).
Keeping every sampled plane would take terabytes at high resolutions, so
every 20 s the trace keeps its state instead: the vertices (with their
dedup keys), colours and faces since the last save are appended to their
files, the samples of the plane the next layer starts at go to `plane-0`
or `plane-1` in turn, and `state` (plane, counts, plane file) is written
last. Restore cuts the files to the counts and rebuilds the vertex and
face maps exactly, so the mesh is the same. The key leaves out the
smoothing (it runs afterwards). Killed at 60 % of a 1000^3 trace and
again before its next save, and a 900^3 trace with colours: the files
matched the export in one go byte for byte. The Bulb Tracer window ends
the checkpoint when the trace is done, the command line when the file is
written.

**Big render tiles** are written to `.<name>.tmp<pid>` and renamed: a
half written file of a tile's name would count as done (`scan_project`).

**Command line.** Every single picture and tiled render continues by
default, as do `montecarlo`, `animate`, `batch` and `voxel`; `--fresh`
deletes the checkpoint first, `--no-resume` keeps none.
`mb3d resume` lists the checkpoints (the done share and the write time are
read from the marks and the files' times, as a killed render did not
update `info.txt`); `--clear [KEY]` deletes them.

## Where the program keeps its files

`src/appdirs.rs` decides between two ways:

* **One folder** (MB3D's way, the release archives, `download.sh`): the
  program's folder is writable, and everything stays next to the program
  as in MB3D: `Mandelbulb3D.ini`, the presets (`*.m3l`, `color_preset_*.bin`,
  the accuracy presets), `History`, `BigRenders`, `Meshes`, `ShaderCache`.
* **Installed** (a program folder the user cannot write, and always below
  `/usr`, so that running it as root does not write into `/usr/bin`):

  | what | where |
  |---|---|
  | `Mandelbulb3D.ini`, presets, `Default.m3p` | `$XDG_CONFIG_HOME/mb3d-rs` (`~/.config/mb3d-rs`) |
  | `History`, `BigRenders`, `Meshes` | `$XDG_DATA_HOME/mb3d-rs` (`~/.local/share/mb3d-rs`) |
  | `ShaderCache`, `resume` (unfinished renders) | `$XDG_CACHE_HOME/mb3d-rs` (`~/.cache/mb3d-rs`) |

  On Windows `%APPDATA%\mb3d-rs` and `%LOCALAPPDATA%\mb3d-rs`. A preset
  or `Default.m3p` next to the program is still read as a default when the
  user has none.

`MB3D_PORTABLE=1` or `0` chooses a way regardless of the folder.

The shipped folders are found next to the program, in the user's data
folder and in `$XDG_DATA_DIRS/mb3d-rs` (`/usr/local/share/mb3d-rs`,
`/usr/share/mb3d-rs`). Formulas are searched in this order:
`--formulas` / `MB3D_FORMULAS`, `./M3Formulas`, `~/.local/share/mb3d-rs/M3Formulas`,
the program's `M3Formulas`, the system-wide ones, next to the parameter
file's folder, `~/mb3d-rs/M3Formulas`; the first file of a name wins, so a
newer formula in the user's folder replaces the shipped one (formulas
whose code differs from the translated ones run in the interpreter). Maps
the same way. Themes are read from the shipped `Themes` folder and then
`~/.local/share/mb3d-rs/Themes`, where a file of the same name replaces
the shipped one and new files add themes.

## Linux packaging and the command line

**glibc.** The Linux programs are built with
[cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild) against
glibc 2.28 (`--target x86_64-unknown-linux-gnu.2.28`); a build on the
runner's own Ubuntu would need its glibc (2.39) and not start on Debian
12, RHEL 9 or Ubuntu 22.04. The release workflow stops if a symbol of a
newer glibc slips in (`objdump -T`). The programs link only libc, libm and
libgcc_s; Vulkan, X11 and Wayland are loaded at run time. `mb3d` contains
no code of the window libraries (the linker drops them).

**In the archive** (Linux and macOS): `man/mb3d.1` and
`completions/{mb3d.bash,_mb3d,mb3d.fish}`, made by `tools/cli_docs.py`
from `mb3d --help` and the sub-commands' `--help`, so they always match
the program; and `mb3d-rs.png`, the icon of the menu entry.

**download.sh** downloads the archive and the release's `SHA256SUMS`,
checks the archive (`sha256sum`/`shasum`) and unpacks nothing on a
mismatch or without checksums. It then installs, for the user only:

| what | where |
|---|---|
| `mb3d`, `Mandelbulb3D` (links into the folder) | `~/.local/bin` |
| menu entry, icon (Linux) | `$XDG_DATA_HOME/applications/mb3d-rs.desktop`, `.../icons/hicolor/{16x16,22x22,...,256x256}/apps/mb3d-rs.png` (all hicolor sizes from 16 to 256 px; the small ones with larger letters, 16 px without) |
| man page | `$XDG_DATA_HOME/man/man1/mb3d.1` |
| completions | `$XDG_DATA_HOME/bash-completion/completions/mb3d`, `$XDG_DATA_HOME/zsh/site-functions/_mb3d` (add to `fpath`), `$XDG_CONFIG_HOME/fish/completions/mb3d.fish` |

(`$XDG_DATA_HOME` is `~/.local/share` and `$XDG_CONFIG_HOME` `~/.config`
unless set.) A file in `~/.local/bin` that is not a link is left alone.
`download.sh --uninstall [FOLDER]` removes these files and links (only
links that point into the folder); the folder stays, as it may hold the
user's parameters and History. `MB3D_RS_URL` points the script to
another release folder (`file://` works, for tests).

**Packages.** The release workflow also builds `mb3d-rs-linux-x64.deb`
(`tools/build_deb.sh` from the Linux release folder; `dpkg-deb`, or `ar`
and `tar` where it is missing) and installs it on the runner as a test.
It lays the files out as a system program: `/usr/bin/mb3d`,
`/usr/bin/Mandelbulb3D`, MB3D's folders (`M3Formulas`, `M3Maps`,
`M3Parameter`, `EM_JIT_M3Formulas`, `Themes`, `examples`) in
`/usr/share/mb3d-rs`, the menu entry `mb3d-rs.desktop` and its icon, the
man page and the completions in Debian's folders (zsh:
`vendor-completions`). `mb3d-rs-linux-x64.rpm` (`tools/build_rpm.sh`,
`rpmbuild` on the runner) has the same files in the same places (zsh:
`site-functions`). It requires only glibc 2.28 and libgcc, since the
programs open Vulkan, X11 and Wayland while running, and only recommends
the Vulkan loader and libxkbcommon (each under its Fedora/RHEL or openSUSE
name). It is xz-compressed, not stripped again and has no debug package,
and the workflow installs and removes it in a Rocky Linux 8 container as a
test. Run by hand with a tag (`gh workflow run release.yml -f
tag=v0.2.N`), the workflow builds that tag again with the current
packaging scripts and adds the files that release is missing, together
with new checksums.

**The icon on the desktops.** The menu entry `mb3d-rs.desktop` names the
icon `mb3d-rs` (hicolor theme, 16 to 256 px). A running window is matched
to the entry by its app ID (native Wayland) or `WM_CLASS` (X11, also
XWayland, which the program prefers so that its windows open at their
places), both `mb3d-rs`, and `StartupWMClass=mb3d-rs`; the window also
sets its own icon (`_NET_WM_ICON`, 64 px), which X11 desktops show in
title bars and task lists. `download.sh` refreshes the desktop file
database, KDE's menu cache (`kbuildsycoca6`) and touches the icon folder
for GTK desktops; the .deb leaves that to Debian's triggers.

**Checking a download by hand:** every release has `SHA256SUMS`, and the
archives have build provenance attestations (made by the release workflow
with GitHub's `attest-build-provenance`):

```sh
sha256sum -c --ignore-missing SHA256SUMS
gh attestation verify mb3d-rs-linux-x64.tar.gz --repo thr32423/mb3d-rs
```

**Command line behaviour:**

* Progress (`\r` updates of the percentage) is written only when the
  error output is a terminal; in logs and pipes only the result lines
  appear.
* `-o -` writes the picture (PNG) to the standard output; the messages go
  to the error output. Not for `montecarlo`, which rewrites its picture
  after every pass.
* `mb3d --version` (`-V`) prints the version.
* Exit status: 0 success, 1 the calculation or another step failed, 2
  wrong command line (unknown option, missing or bad value, no file,
  options that do not go together), 3 the scene or parameter file could
  not be read, 4 a result could not be written. (`main.rs`: the status is
  2 until `args_read()`; `input_err` / `output_err` set 3 and 4.)
* Environment: `MB3D_FORMULAS` and `MB3D_MAPS` (folders, separated like
  `PATH`), `MB3D_GPU=0` (CPU only), `MB3D_PORTABLE` (see [Where the
  program keeps its files](#where-the-program-keeps-its-files), also for
  the order in which formulas are searched).

## Building from source

This section is for developers; the release downloads above need none of
it. The renderer has **no external dependencies** (std only, including its
own PNG and JPEG encoders); the windows use five crates (winit, softbuffer,
fontdue, rustybuzz for scripts with joined letters, arboard) and the graphics card two (wgpu, pollster). Everything
builds with any recent stable Rust:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

The programs are then in `target/release`. They look for `M3Formulas` and
`M3Maps` next to themselves or in the current folder, so either copy MB3D's
folders there (from [thargor6/mb3d](https://github.com/thargor6/mb3d) or an
MB3D installation) or pass `--formulas` / `--maps`. `.cargo/config.toml`
aligns functions and jump targets: without it the speed of some scenes
depended on where the compiler happened to place the hot loops (an
unrelated change made *Insect Menger* 15 % slower on the CPU). With a release installed
by `download.sh` in `~/mb3d-rs`, its formulas and maps are found
automatically, and so are those next to a parameter file's `M3Parameter`
folder.

The graphics card support is the cargo feature `gpu` (part of the default
build; a build with `--no-default-features` is the CPU renderer without
dependencies).

Before pushing: `tools/check.sh` runs the build and test steps of the CI
workflow locally (the build with and without the default features, the
tests, the graphics card tests; about 3 minutes once the build is cached).
`tools/check.sh --install-hook` makes git run it before every push that
changes the code (`src`, `examples`, `tests`, `Cargo.*`, `.cargo`,
`.github`; pushes of text only are let through, `git push --no-verify`
skips it once).

Regenerating the formula translation (only needed when the formula set
changes):

```sh
cargo run --release --bin m3f2rs -- path/to/M3Formulas -o src/x86/native
cargo build --release
```

Checking the formulas: `src/bin/m3fcheck.rs` runs every formula in the
interpreter over random inputs, and with `--translated` compares the
translated code with the interpreter bit for bit:

```sh
./target/release/m3fcheck path/to/M3Formulas --translated --trials 12
```

Graphics card checks: `cargo run --release --example gpuformulas --
M3Formulas` runs every translatable formula on the GPU against the
interpreter (all 449 agree), `cargo test` compares the GPU G-buffer of a
Mandelbulb with the CPU's, `cargo run --release --example postcheck -- 320
MB3D_DIR FILES...` compares the shadow bits and the ambient occlusion of
the card with the CPU's (`--de` the distance estimate along a ray),
`cargo run --release --example probecheck -- MB3D_DIR 480 FILES...` runs
the precision probe on parameter files, `cargo run --release --example
modecheck -- MB3D_DIR 480 OUT_DIR FILES...` repeats the CPU and graphics
card comparison, and `m3f2rs --gpu-report M3Formulas` says which formulas
translate and why not (`LIFT_DEBUG=1`).

## Nightly tests

`.github/workflows/nightly.yml` runs every night at 02:30 UTC (and by hand
from the Actions tab) on Linux, Windows and macOS, with the command line
renderer alone (the runners have no graphics card, so the processor's path
is tested). `tools/nightly.py` runs each render in a process of its own
with a time limit, so a crash or a hang is caught and the rest goes on:

- the 80 example parameter files of the MB3D repository at 15 % of their
  size (`mb3d batch --scale 0.15`);
- MutaGen on six of them, chosen by the date, which is also the seed,
  and every member of the family rendered; a mutation that is unusable is
  no failure, only crashes and hangs count;
- one small render of each other kind: a picture, `--tiles`, the swarm
  with a local folder, Monte Carlo, an animation, a mesh, voxels.

The example pictures are compared with those of the last successful night
(its `pictures-<system>` artifact): the number of changed pixels and the
largest difference. Results go to `results.json` per system.

`tools/nightly_issues.py` (with the run's own token, `issues: write`)
then:

- opens an issue for every failure without one (label `nightly`), with the
  system, the output, and the command that runs it again; the same failure
  (test, item, kind, message without numbers) on another system is added
  to its issue;
- closes a nightly issue, with a note, when its test passed on every
  system it had failed on;
- opens one issue per night that changed example pictures (label
  `picture-change`), to be closed when the change was meant;
- opens an issue for a system that left no results (its build failed).

It only touches issues labelled `nightly`; their bodies carry hidden
markers (`<!-- nightly-sig: ... -->`) by which they are found again.
`--dry-run` shows what it would do; `GH` names the `gh` program.

