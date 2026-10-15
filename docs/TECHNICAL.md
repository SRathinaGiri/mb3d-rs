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
- [Output: videos, 16 bit pictures, noise filter](#output-videos-16-bit-pictures-noise-filter)
- [Known differences and gaps](#known-differences-and-gaps)
- [Building from source](#building-from-source)

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
  shader is compiled, until they meet; the card then calculates the
  shadows and the ambient occlusion of the whole image. The first time a
  formula combination is used its shaders compile (10 to 20 s; the
  compiled pipelines are kept in the `ShaderCache` folder next to the
  program for the next time) while the CPU already calculates, so
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
of an `.m3f` formula is analysed (`src/lift.rs`: every memory access
resolved to the iteration record, the formula's constants or the stack)
and translated to WGSL (`src/lift_wgsl.rs`), including the SSE2 code many
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
before: the formulas the lifter cannot resolve and volumetric light. Views zoomed so far that single precision would show
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
*Graphics card* the calculation waits for it. Since 0.2.34 *Auto* leaves
a scene the probe sends to double precision to the CPU threads
altogether (consumer cards calculate doubles 16 to 32 times slower:
*AboxShapes-Misty* took the card 203 s in double precision against 45 s
on the CPU), *Graphics card* calculates it on the card in double
precision. Monte Carlo renders use the probe too when the scene's march
is compiled. `MB3D_GPU_NOPROBE=1` turns it off and `MB3D_PROBE_DEBUG=1`
prints its result. The command line program waits for a shader still
compiling before it ends (`gpu::wait_compiles`): a compile the CPU
threads had overtaken used to die with the process and never reach the
pipeline cache, so a short scene's next run was just as cold.
`mb3d` prints which was used and why; errors of the graphics card or its
driver also send the calculation to the CPU, and software renderers
(llvmpipe, WARP) are not used. The first calculation of a scene compiles
its shader (one to twenty seconds depending on the driver; the compiled
pipeline is kept in `ShaderCache` next to the program, independently of
the driver's own cache, which may be full or off). The desktop shares
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
| `m3f.rs` | CustomFormulas.pas (`LoadCustomFormula`, `FillCustomVBufWithVars`) | `.m3f` custom formula files, all 23 option types, constants |
| `x86.rs` | (new) | IA-32 interpreter for the formulas' machine code: integer, x87 and SSE/SSE2 subset; plus a compiler that resolves the x87 stack statically into micro-ops |
| `custom.rs` | formulas.pas / TypeDefinitions.pas | memory layout of `TIteration3Dext` and the calling convention used by MB3D's hybrid loop, host functions |
| `anim.rs` | Animation.pas, Interpolation.pas, Math3D.pas | keyframes, sub-frame schedule (`TotalBMPsToRender`, `Timer2Timer`), `Interpolate2frames`, `Interpolate3framesBezier` (header and formula values), `bInterpolateFormula`, quaternions (`MatrixToQuat`, `SlerpQuat`, `SlerpSVec`, `BezierIpol3SVecs`) |
| `lighting.rs` (`blend`) | Interpolation.pas | the light value part of both interpolations (`TLightVals`, `Slerp2SMatrices`, `Slerp3SMatrices`) |
| `animfile.rs` | Animation.pas (`LoadAni`, `SpeedButton9Click`) | `.m3a` animation files incl. preview images; the `.m3k` text format |
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

1. **Translated to Rust** (`src/native.rs`, generated): 441 formulas. Their
   machine code was translated ahead of time into ordinary Rust functions,
   which the Rust compiler turns into native code for whatever CPU the crate
   is built for (x86-64, ARM64, …); nothing in it is specific to Intel CPUs.
   At run time a formula is recognised by a hash of its code bytes.
2. **Compiled micro ops** (`x86.rs`): the same analysis, executed by a small
   loop. Used for formula files that have no translation (for example new
   files added later), or with `MB3D_NO_NATIVE=1`.
3. **Interpreter** (`x86.rs`): an IA-32 emulator (integer, x87, SSE/SSE2
   subset) for code that does not fit the static model (a handful of formula files).

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

Verification: a small native 32-bit program runs the same machine code on
the real CPU from the same memory image, and the results are compared with
the port's over random inputs for every formula (see
[Building from source](#building-from-source)).
All 449 testable formulas, dIFS formulas included, agree with the CPU to
1e-9 relative, in every tier (the check runs the translated code by default,
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

## Output: videos, 16 bit pictures, noise filter

`mb3d animate ... --video FILE [--fps N]` and the Animation maker's *Also
make a video* (`src/video.rs`) run ffmpeg on the finished frame images
(PNG, BMP or JPEG; not for `--format m3p` or stereo pairs), which stay in
place. ffmpeg is the one on the `PATH`, or `MB3D_FFMPEG`; it is checked
before the first frame. The frames go in through an ffconcat list (each
for 1/fps seconds), so `--every`, `--from`/`--to` and index steps work;
odd sizes are padded by one pixel. By extension: `.mp4`/`.mov` H.264
(`yuv420p`, CRF 18, faststart), `.mkv` H.264, `.webm` VP9 (CRF 30),
anything else ffmpeg's default for it (e.g. `.gif`). The window writes
`<output folder>/<project name>.mp4|.webm` and keeps its settings in the
ini (`AniVideo`, `AniVideoFormat`, `AniVideoFps`).

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
* **Animation:** `.m3a` files before version 5 are not supported. Map sequences (`Mandelbulb3DMSeq.ini` in the working folder or
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

## Building from source

This section is for developers; the release downloads above need none of
it. The renderer has **no external dependencies** (std only, including its
own PNG and JPEG encoders); the windows use four crates (winit, softbuffer,
fontdue, arboard) and the graphics card two (wgpu, pollster). Everything
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
cargo run --release --bin m3f2rs -- path/to/M3Formulas -o src/native.rs
cargo build --release
```

Checking the formulas against the real CPU: `tools/oracle/oracle.asm` is a
small native 32-bit Linux program that runs the formulas' machine code from
the same memory image, and `src/bin/m3fcheck.rs` compares interpreter and
CPU over random inputs for every formula:

```sh
nasm -f elf32 tools/oracle/oracle.asm -o /tmp/oracle.o && ld -m elf_i386 /tmp/oracle.o -o /tmp/mb3d_oracle
./target/release/m3fcheck path/to/M3Formulas --oracle /tmp/mb3d_oracle --trials 12
#   460 formulas: 449 ok, 0 mismatch, ...
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
