# mb3d-rs — Mandelbulb3D for Linux, Windows and macOS

mb3d-rs is [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D), the
3D fractal program, rewritten so that it runs on today's computers: on
**Linux, Windows and macOS**, and with the picture calculated on the
**graphics card** where MB3D could only use the processor. It opens MB3D's
parameter files (`.m3p`, `.m3i`, text parameters from forums), uses MB3D's
formulas and maps, and makes the same pictures.

<a href="docs/img/main_window.jpg"><img src="docs/img/main_window.jpg" width="760" alt="The main window"></a>

It comes as two programs:

* **Mandelbulb3D** — the desktop program, with MB3D's own windows: the
  main window, Formulas, Lighting, Post processing, Navigator, Animation,
  Batch, Big renders, Monte Carlo, Voxel export, Bulb Tracer 2, MutaGen
  and the rest. If you know MB3D, you know this program; MB3D's tutorials
  apply.
* **mb3d** — a command line renderer for scripts and batch jobs: pictures,
  animations, batch lists, Monte Carlo renders, voxel and mesh exports,
  all without a window.

## What is new compared with Mandelbulb3D

* **The graphics card calculates the picture** — the image itself, the
  shadows, the ambient occlusion, the reflections and the Monte Carlo
  renders. On a typical computer this is several times faster than the
  processor alone; see [Speed](#speed) below.
* **Auto mode**: the graphics card and the processor work on every picture
  together, and nothing ever waits — if the card is not ready, the
  processor starts anyway and the card joins in.
* **An LED in the title bar** shows where the last picture was calculated.
* **Interface size** from 75 to 200 % for high resolution screens.
* **Eight looks** for the windows, all of them theme files you can copy and change: two reproduce MB3D's looks, six are new.
* **Linux, Windows and macOS** (Intel and Apple Silicon).
* **A command line program** for everything that needs no window.

## Download

Ready-to-run programs for the latest release, no installation needed:

| System | Download |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| macOS, Apple silicon (M1 and later) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Unpack it and start **Mandelbulb3D** (`Mandelbulb3D.exe` on Windows) in
the unpacked folder. The folder is laid out like an MB3D installation,
with MB3D's formulas, maps and 80 example parameter files.

* If Windows shows "Windows protected your PC", click *More info* ▸
  *Run anyway*.
* macOS blocks downloaded programs that are not signed: run
  `xattr -dr com.apple.quarantine .` in the unpacked folder once.

**Alternative for Linux and macOS:** instead of downloading by hand, this
one line fetches the latest release into `~/mb3d-rs` (run it again later
to update; the script [download.sh](download.sh) is also in the program's
folder):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Then start `~/mb3d-rs/Mandelbulb3D`.

[Getting started](GETTING_STARTED.md) walks through the first steps; older
versions and the release notes are on the
[releases](https://github.com/thr32423/mb3d-rs/releases) page.

## How the rendering works

A fractal picture is calculated pixel by pixel: for every pixel a ray is
followed into the scene until it hits the fractal's surface, the formula
being evaluated dozens of times at every step. MB3D does this on the
processor, using all its cores. mb3d-rs can do the same, and it can also
hand the work to the **graphics card**, which has thousands of small
processors and is built for exactly this kind of job.

The **"Calculate on"** setting (Internal tab of the main window) chooses:

| | |
|---|---|
| **Auto** (default) | The card and the processor share the picture: the card takes rows from the top, the processor's threads from the bottom, until they meet. The card then does the shadows and the ambient occlusion. |
| **CPU** | The processor only, as in MB3D. |
| **Graphics card** | The card only (it waits until its program is ready). |

The first time a combination of formulas is used, the graphics driver
has to prepare a program for the card — the *shader* — which takes 10 to
15 seconds on the computer measured below. In Auto mode the processor
simply starts without it and the card joins when it is ready, so a small
picture never waits. The prepared shader is kept for the next time (on
Linux in the `ShaderCache` folder next to the program, on Windows and
macOS by the graphics driver), so the next picture with the same formulas
starts on the card at once.

The **LED** right of the window title tells where the last picture was
calculated: green on the graphics card, orange partly, red on the
processor. Pointing at it shows the details.

[How the rendering works](docs/RENDERING.md) tells the whole story with
pictures: what Auto does the first time, why the card is not always the
faster choice, and what the colours of the LED mean.

Not everything can go to the card: a few formulas cannot be translated for
it, scenes with volumetric light stay on the processor, and views zoomed
in very deeply are calculated in double precision (which only some cards
can do). The program decides all of this by itself; you only see it in the
LED.

## Speed

All measurements on one computer: an AMD Ryzen 9 3900X (12 cores, 24
threads) with an NVIDIA GeForce RTX 2060, on Linux. The original
**Mandelbulb3D 1.99sr37** was run on the same machine under Wine (its
Windows program runs there unchanged, at full speed, using all cores),
and timed from the click on *Calculate 3D* to the finished picture.
Each parameter file was rendered at the same image size by all three:
MB3D, mb3d-rs on the processor only (**CPU**) and mb3d-rs in **Auto**
mode (graphics card and processor together). The Auto times are those of
the second render of a file, with the card's shader already prepared; the
first render of a new formula combination adds the 10–15 s of shader
preparation on this card.

### MB3D's 80 example parameter files

The example files that come with MB3D (and in this program's
`M3Parameter` folder), each at its own image size.

MB3D could not calculate one of the files (*Test8*: it stops with a "Division by zero" error), so the comparisons count 79 files.

* **mb3d-rs on the processor alone is faster than MB3D** in 68 of the 79 files, a median **1.6 times**; all files together took MB3D 18 min 21 s and mb3d-rs on the processor 18 min 17 s.
* **Auto mode** is faster than MB3D in 75 of 79 files (median 2.0 times); all files together 9 min 32 s. For 29 of the files the processor finishes in under two seconds, before the card gets a turn — there Auto is simply the processor plus a moment's overhead. The card joined in 48 files, the slower ones, where it made the picture a median 1.5 times faster than the processor (up to 14 times: *Insect Menger*, 4.9 s instead of 66.5 s).
* In 2 files Auto was slower than the processor alone (*AboxShapes-Misty*, *DEcomb1*): there the card is slower than the 24 processor threads for this scene, or the ambient occlusion of a very large picture runs slower on the card.
* "Rows on the card" is the share of the picture the graphics card marched in Auto mode (– when the processor finished alone).

<details>
<summary>All 80 files (click to open)</summary>

| Parameter file | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | rows on the card | Auto vs MB3D |
|:--|--:|--:|--:|--:|--:|
| 0Encrypted0 - ABoxSOff4d example | 12.8 s | 6.8 s | 2.0 s | 81 % | 6.4× |
| 6 AM - Torii temple | 23.0 s | 20.7 s | 19.1 s | 60 % | 1.2× |
| Abominog-Ua - 12.06.2014 | 7.6 s | 8.4 s | 7.4 s | 50 % | 1.0× |
| ABoxPartlyJuliaRoff | 2.5 s | 1.4 s | 1.7 s | – | 1.5× |
| ABoxScale2Start | 1.0 s | 0.1 s | 0.4 s | – | 2.5× |
| ABoxScale3Start | 1.5 s | 0.1 s | 0.3 s | – | 5.0× |
| AboxShapes-Misty | 27.5 s | 45.0 s | 59.1 s | – | 0.5× |
| Aexion 10bulbs | 11.5 s | 10.0 s | 2.8 s | 73 % | 4.1× |
| AkuraPare - 463b | 4.0 s | 2.3 s | 1.4 s | 63 % | 2.9× |
| ApolloBalloons dIFS | 75.8 s | 81.8 s | 27.8 s | 61 % | 2.7× |
| AureliusCat - Bamboo | 2.0 s | 1.2 s | 0.8 s | 50 % | 2.5× |
| BatJorge - Pong 703 - Swirls | 4.0 s | 3.0 s | 3.4 s | – | 1.2× |
| Bill-Snowzell - Tweak - Canyon | 4.0 s | 1.9 s | 2.0 s | 38 % | 2.0× |
| Blood on the dancefloor | 7.6 s | 5.1 s | 3.9 s | – | 1.9× |
| BulboxCut | 4.0 s | 2.3 s | 2.3 s | – | 1.7× |
| Chrystal | 5.6 s | 5.6 s | 1.7 s | 79 % | 3.3× |
| cutted sphere over carpet | 7.1 s | 20.7 s | 4.2 s | 52 % | 1.7× |
| Dainbramage - Home of the Ancients | 3.0 s | 1.5 s | 1.7 s | 25 % | 1.8× |
| Dainbramage - Hydra | 3.5 s | 1.8 s | 1.8 s | 40 % | 1.9× |
| Dainbramage - Nothing left but bones | 4.0 s | 3.0 s | 2.6 s | 49 % | 1.5× |
| DEcomb1 | 83.9 s | 92.0 s | 110.4 s | 47 % | 0.8× |
| dIFS heart shrub | 2.5 s | 3.8 s | 3.0 s | – | 0.8× |
| dIFS heart shrub otrap | 5.1 s | 17.0 s | 3.3 s | 93 % | 1.5× |
| dIFS shrub | 4.0 s | 19.6 s | 3.4 s | 93 % | 1.2× |
| Dodeca Torus mix | 2.0 s | 0.6 s | 0.7 s | – | 2.9× |
| DsyneGrafix - Getting Loopy | 2.0 s | 0.8 s | 0.9 s | 24 % | 2.2× |
| Ellarien - Embellished | 3.5 s | 1.1 s | 1.3 s | 32 % | 2.7× |
| Ellarien - Shrooms are coming | 8.1 s | 6.2 s | 6.9 s | – | 1.2× |
| ExcludeBulbMeng | 17.3 s | 12.9 s | 8.6 s | – | 2.0× |
| Fishbowl | 4.6 s | 3.5 s | 2.0 s | 49 % | 2.3× |
| Fractales75 - Kiosk | 3.0 s | 1.8 s | 2.1 s | 41 % | 1.4× |
| Genetic Menger | 1.5 s | 0.5 s | 0.7 s | 32 % | 2.1× |
| genQuat helix mix | 4.0 s | 3.1 s | 1.6 s | 64 % | 2.5× |
| HalTenny - Freak Of Nature | 2.5 s | 1.1 s | 1.4 s | – | 1.8× |
| Hal-Tenny-Resistance (slow) | 18.9 s | 19.7 s | 8.8 s | 80 % | 2.1× |
| Hyperben2 - Ozosphere tweak | 2.0 s | 0.8 s | 0.7 s | 50 % | 2.9× |
| ImpossibleWorld dIFS | 399.8 s | 396.2 s | 99.5 s | 74 % | 4.0× |
| InAndOutside box | 5.1 s | 2.7 s | 1.5 s | 67 % | 3.4× |
| Insect Menger | 4.0 s | 66.5 s | 4.9 s | 93 % | 0.8× |
| Jost1 - Tweak - New tiny temple | 3.5 s | 1.5 s | 1.1 s | 57 % | 3.2× |
| julia on box | 4.6 s | 3.8 s | 1.5 s | 68 % | 3.1× |
| JuliaSetSample | 3.0 s | 1.9 s | 2.2 s | – | 1.4× |
| KentPress - Parameter 3 tweak fogits | 4.0 s | 2.7 s | 3.1 s | – | 1.3× |
| KochSurf Sample 6 | 3.0 s | 1.3 s | 1.0 s | 71 % | 3.0× |
| Lenord - ASurf 2 | 5.1 s | 2.4 s | 2.4 s | 42 % | 2.1× |
| Lenord - Between Dark and Light | 5.6 s | 4.1 s | 2.6 s | 57 % | 2.2× |
| Lenord - The Beanstalk | 3.5 s | 2.2 s | 2.6 s | – | 1.3× |
| LightBulbMoon - A Fabulous Excuse | 3.5 s | 2.0 s | 1.6 s | – | 2.2× |
| LightBulbMoon - No, Never | 5.1 s | 3.9 s | 1.4 s | 80 % | 3.6× |
| Mandelwerk - Makin seashell | 12.0 s | 8.3 s | 8.5 s | 40 % | 1.4× |
| Mandelwerk - TgladTetra inner spirals | 8.6 s | 4.8 s | 1.5 s | 79 % | 5.7× |
| MarkJayBee - Curvichrome | 2.5 s | 1.2 s | 1.2 s | – | 2.1× |
| material colors | 3.0 s | 1.3 s | 1.8 s | – | 1.7× |
| Melting spot bloxx | 23.2 s | 19.8 s | 6.6 s | 75 % | 3.5× |
| Mengerplus for MC | 3.5 s | 2.1 s | 2.6 s | – | 1.3× |
| MengerTrees | 19.3 s | 15.2 s | 16.5 s | – | 1.2× |
| MutaGenD4B12E71C70D4FD00357_canary~ | 7.1 s | 3.7 s | 3.9 s | – | 1.8× |
| Oxnot - Flexing | 2.5 s | 0.8 s | 0.9 s | 32 % | 2.8× |
| Oxnot - Shells | 5.6 s | 3.3 s | 3.7 s | – | 1.5× |
| PaMonk - Ringlets | 5.1 s | 2.7 s | 2.9 s | – | 1.8× |
| QuatP4hybridJulia | 1.5 s | 0.2 s | 0.5 s | – | 3.0× |
| Rama - Elysium | 11.5 s | 10.4 s | 4.6 s | 76 % | 2.5× |
| Recycledrelatives - Fractal Fan | 2.0 s | 0.5 s | 0.7 s | 23 % | 2.9× |
| Recycledrelatives - Test for SurfMod | 4.6 s | 3.1 s | 3.2 s | – | 1.4× |
| Rowratty - Spirals | 4.0 s | 2.8 s | 1.6 s | 57 % | 2.5× |
| Sabine62 - Housing problem | 11.7 s | 10.5 s | 6.8 s | – | 1.7× |
| spineJulia | 91.9 s | 71.9 s | 44.3 s | 45 % | 2.1× |
| Surreal shell | 2.0 s | 0.6 s | 0.9 s | – | 2.2× |
| Test8 | error in MB3D | 0.2 s | 0.4 s |  | – |
| Theli-At - Black Mandelbulb | 1.0 s | 0.3 s | 0.7 s | 6 % | 1.4× |
| Theli-At - Bone Menger using MH | 2.0 s | 0.9 s | 1.3 s | – | 1.5× |
| Theli-At - MengerSpheres | 2.0 s | 0.6 s | 0.7 s | 41 % | 2.9× |
| ThePearl dIFS | 2.5 s | 1.4 s | 1.2 s | 30 % | 2.1× |
| TimeMachine | 1.0 s | 0.4 s | 0.8 s | – | 1.2× |
| toricaleggs | 8.6 s | 5.5 s | 6.1 s | 40 % | 1.4× |
| TreePlanet | 7.6 s | 6.6 s | 7.4 s | – | 1.0× |
| UndeadAcademy tweak - Peacock in blue | 7.6 s | 4.5 s | 1.3 s | 75 % | 5.8× |
| Virtual tubes world (UndeadAcademy style) | 9.1 s | 6.9 s | 4.3 s | 49 % | 2.1× |
| Wada basin - Reflect test | 6.6 s | 5.2 s | 4.7 s | 35 % | 1.4× |
| Waste-and-tragedy - Mandalex | 5.1 s | 3.6 s | 4.0 s | – | 1.3× |

</details>

The pictures are the same: mb3d-rs on the processor makes MB3D's picture
to within rounding, and the Auto pictures differ from the processor's by a
fraction of a percent of the colour range on average (fine detail may
sparkle in different pixels; the [technical notes](docs/TECHNICAL.md)
have the measurements).

## The windows

**Prefs ▸ Visual themes** offers eight looks, each a file in the `Themes`
folder: *Glossy* (the dark default) and *Windows* reproduce MB3D's own
two looks, the other six are new. Click a picture for its full size.

| | |
|---|---|
| <a href="docs/img/themes/Glossy.jpg"><img src="docs/img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="docs/img/themes/Windows.jpg"><img src="docs/img/themes/Windows.jpg" width="380" alt="Windows"></a> |
| **Glossy**: MB3D's default | **Windows**: MB3D's "Themes off" |
| <a href="docs/img/themes/Graphite.jpg"><img src="docs/img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="docs/img/themes/Daylight.jpg"><img src="docs/img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
| **Graphite**: lighter neutral grey, steel-blue accents | **Daylight**: warm light grey, teal accents |
| <a href="docs/img/themes/Midnight.jpg"><img src="docs/img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="docs/img/themes/Ember.jpg"><img src="docs/img/themes/Ember.jpg" width="380" alt="Ember"></a> |
| **Midnight**: deep navy, blue accents | **Ember**: warm charcoal, orange accents |
| <a href="docs/img/themes/Forest.jpg"><img src="docs/img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="docs/img/themes/Highcontrast.jpg"><img src="docs/img/themes/Highcontrast.jpg" width="380" alt="High contrast"></a> |
| **Forest**: dark green-grey, green accents | **High contrast**: black, white text, yellow highlights |

A theme is a small text file in the `Themes` folder next to the program;
copy one, change its colours, and it appears in the list at the next
start.

**Prefs ▸ Visual themes ▸ Interface size** makes all windows and their
text larger or smaller (75 to 200 %), for high resolution screens; the
layouts stay exactly as in MB3D.

| 100 % | 150 % |
|---|---|
| ![Visual themes at 100 %](docs/img/themes_100.png) | ![Visual themes at 150 %](docs/img/themes_150.png) |

The windows are MB3D's own — made from MB3D's original window
definitions, so every button, slider and hint is where it is in MB3D:

* **Main window:** open and save parameters (`.m3p`, `.m3i`, text, the
  clipboard), save pictures (PNG, JPEG, BMP, 16 bit Z-buffer), the mouse
  modes, the rotation buttons, image size, the quality presets, undo,
  *Calculate 3D* with the growing picture and the statistics, and all
  the pages (Calculation, Internal, Infos, Cutting, Julia, Camera,
  Coloring, Stereo).
* **Formulas:** the six formula slots, MB3D's formula lists and options,
  hybrid types, favourites, and the JIT formula editor.
* **Lighting:** lights 1–6, presets, the palette, the Object, Ambient,
  d.Fog and Back pic pages; changes repaint the picture at once, without a
  new calculation.
* **Post processing:** normals on the z-buffer, hard and ambient shadows,
  reflections and transparency, depth of field.
* **Navigator:** walk and look around with keys and mouse, then *View to
  main*.
* **Animation maker, Batch processing, Big renders, Monte Carlo, Voxel
  export, Bulb Tracer 2** (meshes) with the mesh preview, the **HeightMap
  generator, MutaGen, ZBuf16Bit, Map sequences, Ini dirs** — all as in
  MB3D.

Settings are kept in `Mandelbulb3D.ini` next to the program, as MB3D
does. Not in this program: MB3D's script editor, the "shortdistance check
DE" option, the random light strokes and the ambient light pass of the
post processing window.

## The command line

`mb3d` renders without a window. Open a terminal in the program's folder
(`cd ~/mb3d-rs` after the one-line install; on Windows a command prompt
in the unpacked folder, and type `mb3d` instead of `./mb3d`):

```sh
./mb3d "M3Parameter/6 AM - Torii temple.m3p" -o torii.png      # full size
./mb3d "M3Parameter/6 AM - Torii temple.m3p" --scale 0.25 -o small.png
./mb3d pasted.txt -o pasted.png        # parameters copied from a forum post
./mb3d City.m3p --tiles 4x4 -o city.png    # a big picture in tiles
./mb3d --cpu City.m3p -o city.png          # processor only
./mb3d --card City.m3p -o city.png         # graphics card only
./mb3d --help                              # all options
```

Everything else has a sub-command with its own `--help`:

```sh
./mb3d batch M3Parameter --scale 0.2 -o previews        # a whole folder
./mb3d animate start.m3p middle.m3p end.m3p --frames 60 -o frames
./mb3d animate project.m3a -o frames                     # MB3D animations
./mb3d montecarlo scene.m3p --rays 64 -o mc.png
./mb3d voxel scene.m3p -o voxels --slices 256            # for 3D printing
./mb3d mesh scene.m3p -o object.stl                      # Bulb Tracer 2
./mb3d mutagen scene.m3p -o muta                         # 15 variations
```

The formulas and maps are found next to the program; those of another
MB3D installation can be given with `--formulas` and `--maps`.
Animations are MB3D's: keyframes with MB3D's interpolation, `.m3a`
projects opened and saved, frames written like MB3D so that an
interrupted animation continues and several computers can share one.

## Good to know

* **Same pictures, not bit-identical.** MB3D calculated in the 80-bit
  precision of old x87 arithmetic; this program uses today's 64-bit. The
  pictures match in every structure, but single pixels can differ in the
  last digit of their colour. Monte Carlo renders and MutaGen use their
  own random numbers, so their results are not MB3D's.
* **The graphics card's pictures** differ slightly from the processor's
  (single precision); in very fine detail the sparkle can fall on other
  pixels. Deep zooms switch to double precision by themselves.
* **Speed on the processor** is close to MB3D's: the formulas' machine
  code from MB3D's `.m3f` files was translated so that it runs on any
  processor (also Apple Silicon), at a small cost; the ray marching
  itself is faster. See the tables above.
* **Big pictures** can be rendered in tiles, also on several computers
  (`--tiles`, and the Big renders window).

## For developers

The [technical notes](docs/TECHNICAL.md) describe how the graphics card
is used, what was ported from MB3D's Delphi sources and how, the three
ways MB3D's formula machine code is run, the scene file format and its
keys, the checks against MB3D, and building from source:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

## License

MB3D is licensed under the LGPL 2.1. This port is a derivative work and uses
the same license.
