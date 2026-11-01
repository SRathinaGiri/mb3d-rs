<img src="assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D for Linux, Windows and macOS">

# mb3d-rs — Mandelbulb3D for Linux, Windows and macOS

<!-- languages:start -->
**English** · [Deutsch](docs/i18n/README.de.md) · [Español](docs/i18n/README.es.md) · [Français](docs/i18n/README.fr.md) · [Bahasa Indonesia](docs/i18n/README.id.md) · [Italiano](docs/i18n/README.it.md) · [Nederlands](docs/i18n/README.nl.md) · [Polski](docs/i18n/README.pl.md) · [Português](docs/i18n/README.pt.md) · [Kaszëbsczi](docs/i18n/README.csb.md) · [العربية](docs/i18n/README.ar.md) · [اردو](docs/i18n/README.ur.md) · [हिन्दी](docs/i18n/README.hi.md) · [日本語](docs/i18n/README.ja.md) · [한국어](docs/i18n/README.ko.md) · [简体中文](docs/i18n/README.zh-CN.md) · [繁體中文](docs/i18n/README.zh-TW.md)
<!-- languages:end -->

<!-- latest:start -->
**Latest release: [v0.2.90](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.90)** (1 Nov 2026, 09:15 UTC) · [Download](#download) · [What's new](docs/UPDATES.md)
<!-- latest:end -->

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

  <details><summary>Read more</summary>

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
  * **Swarm** (new, on the Tools tab): your computers render together,
    see [Rendering on several computers](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — a command line renderer for scripts and batch jobs: pictures,
  animations, batch lists, Monte Carlo renders, voxel and mesh exports,
  Gaussian splats, all without a window.

## Is it safe to run? Check it yourself

You do not have to take our word for it. [scanrepo.dev](https://www.scanrepo.dev)
is an independent service that examines a GitHub repository for the signs
of harmful software: code that starts other programs, connects to the
internet, hides what it does, or bypasses the safety checks of the
programming language. Anyone can run it on this repository at any time:

**[▶ Scan mb3d-rs on scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(click *Rescan* for a fresh result of the current code)

Version 0.2.85 was rated **safe: 5 out of 100** (scanned 30 October 2026,
rules of the scanner's engine 10). Its one warning is the line that starts
the formulas the program compiles at run time for speed: running freshly
made machine code cannot be written in Rust's checked form, and that line
and the bridge to Intel's noise filter are the only places the program
allows it; the rest is an informational note. (Earlier: 0.2.66 was rated
0 out of 100 by an older engine, see also
[the 0.2.44 result](docs/img/scan_0.2.44.png).) An earlier version was flagged,
and we changed the code in response ([issue #4](https://github.com/thr32423/mb3d-rs/issues/4));
it is now written to stay this way. The programs contain no code that
connects to the internet; the only other program they ask your system to
open is your file manager, when you click a button to open a folder.

A scan looks for known warning signs; it is not a guarantee. These let
you check further on your own:

* **All of the source code is here.** The downloads are built from it by
  GitHub's own build servers, by the steps you can read in
  [the release workflow](.github/workflows/release.yml), not on anyone's
  personal computer.
* **Every release lists checksums** (`SHA256SUMS`), so you can confirm
  that the file you downloaded is exactly the one that was published;
  the one-line install below does this for you. Releases from 0.2.45 on
  also carry GitHub's build attestations: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` shows which commit a download was built
  from.

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

  <a href="docs/img/themes_150.png"><img src="docs/img/themes_150.png" width="300" alt="Visual themes at 150 %"></a>

  <details><summary>Read more</summary>

  **Prefs ▸ Visual themes ▸ Interface size** makes all windows and their
  text larger or smaller (75 to 200 %), for high resolution screens; the
  layouts stay exactly as in MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Visual themes at 100 %](docs/img/themes_100.png) | ![Visual themes at 150 %](docs/img/themes_150.png) |

  </details>
* **Eight looks** for the windows, all of them theme files you can copy and
  change: two reproduce MB3D's looks, six are new.

  <a href="docs/img/themes/Midnight.jpg"><img src="docs/img/themes/Midnight.jpg" width="300" alt="The Midnight theme"></a>

  <details><summary>Read more</summary>

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

  </details>
* **A crash leaves a note on every system**: a panic of the program (or
  the error it ends with) is appended to `mb3d_crash.txt`, next to the
  program or in its data folder (before only on Windows). Send that file
  with a bug report.
* **All the graphics cards of the computer calculate**, with nothing to
  set: every card found takes bands of rows of the picture from the same
  pool as the first card and the processor, so a faster card simply
  takes more; the shadows, the occlusion, the Monte Carlo rays and the
  reflections are shared the same way, and an animation renders a frame
  per card at once. A slow card (integrated graphics beside a dedicated
  one) does what it can and never holds a picture up: once the free rows
  are gone, the idle cards and the processor take over the rows it has
  not delivered. A card that fails is left out, the others carry on.
  `mb3d --list-cards` shows what was found. (Tested with one card opened
  twice and with a software Vulkan driver as a second card; not yet on a
  computer with two real cards.)
* **An unfinished picture is a `.m3i` file.** What a stopped, crashed or
  killed render has calculated is kept in the cache (the Unfinished
  renders window, `mb3d resume`); for a picture that file is now a
  `.m3i` image file at every moment, `image.m3i` in the render's folder,
  with the rows not calculated yet as background. Copy it out whenever
  you like, or press *Save m3i* in the Unfinished renders window (`mb3d
  resume --m3i KEY file.m3i` on the command line; both also write the
  `data.bin` of an earlier version as a `.m3i`), and open it here or in
  Mandelbulb3D, no conversion needed. (Monte Carlo renders keep their
  own records, the `.m3c` of a pass is written with `--m3c`.)
* **Hotkeys for the windows**: a key combination brings the main window,
  the Navigator, the Animation, Formulas, Lighting, Post processing or
  Monte Carlo window to the front from any window (Ctrl+1 to Ctrl+7), and
  Ctrl+W closes the window you are in (on macOS Cmd). Set your own under *Prefs > Hotkeys*: click a field and
  press the keys, Backspace takes a hotkey away.
* **The interface in your language**: every text of the windows comes
  from a language file; twelve languages ship, and the program writes the
  file of a new language for you to translate.

  <details><summary>Read more</summary>

  The language badge at the right end of the main window's title bar
  shows the interface language (`EN`); a click opens the language window.
  A language is a text file in the `Languages` folder next to the program.
  Shipped: English, Deutsch, Français, Italiano, Nederlands, Polski,
  Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文 and 繁體中文 (each with its flag; the
  East Asian ones bring a subset of Noto Sans CJK for their characters,
  Hindi brings Noto Sans Devanagari, whose joined letters and vowel signs
  are set with the rustybuzz text shaper).
  The translations were machine made and checked for structure, not by
  native speakers: corrections are welcome, each is one line of the file. To translate: type the name of your language and
  press *Create language file*; the program writes `Languages/<name>.lng`
  with every text in English, one per line (`Button2.Caption = Calculate
  3D`). Translate the texts right of the `=` with a text editor and press
  *Reload*: the windows follow at once, without a restart. A line left out
  keeps the English text. *Save + exit* keeps the language for the next
  start.

  </details>
* **Linux, Windows and macOS** (Intel and Apple Silicon).
* **A command line program** for everything that needs no window.
* **Very big pictures in one piece**: 15000 x 20000 pixels and more
  without running out of memory, and the depth of field uses all
  processor cores (11 times faster on 24 threads than before 0.2.45, with
  the same picture).
* **16 bit pictures:** PNG files with 65,536 instead of 256 steps per
  colour, for editing without banding.

  <a href="docs/img/png16_compare.png"><img src="docs/img/png16_compare.png" width="300" alt="16 bit pictures"></a>

  <details><summary>Read more</summary>

  MB3D saves 8 bit pictures: 256 steps per colour. That is plenty to look
  at, but when a picture is edited afterwards — brightness, contrast,
  curves — smooth areas such as the background or fog break into visible
  bands. **Save pic ▸ PNG** offers a second file type, **PNG 16 bit
  colour**: the colours as the program calculated them, before they are
  rounded to 256 steps, with all shadows, reflections and depth of field.
  The file looks the same, and stays smooth when it is edited.
  
  <a href="docs/img/png16_compare.png"><img src="docs/img/png16_compare.png" width="760" alt="A background gradient after a strong contrast increase: steps in 8 bit, smooth in 16 bit"></a>
  
  <a href="docs/img/save_png16.png"><img src="docs/img/save_png16.png" width="420" alt="The save dialog with 'PNG 16 bit colour'"></a>

  </details>
* **A noise filter** for Monte Carlo and all other pictures, the DCT
  denoising known from GIMP, and for Monte Carlo renders also a neural
  one, Intel Open Image Denoise.

  <a href="docs/img/mc_noise.png"><img src="docs/img/mc_noise.png" width="300" alt="Noise filter"></a>

  <details><summary>Read more</summary>

  A **noise filter** removes grain from the pictures: the **DCT denoising**
  of GIMP's *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT*, which of
  GIMP's filters suited these pictures best (see the credits below).
  
  * **Monte Carlo window ▸ -noise** (a button MB3D had hidden): filters the
    shown picture, and the saved one, with a strength that follows the rays
    per pixel. The rendering itself goes on unfiltered, and its next update
    shows it unfiltered again.
  * **Post processing ▸ Noise Filter**: *Filter now* filters the picture of
    the main window; *Filter new pictures automatically* filters every new
    one. *Strength* is the strength of the noise to remove (4 is light;
    higher removes more grain and more fine detail).
  
  Monte Carlo pictures have the most grain. Against pictures rendered with
  512 rays per pixel, the filtered pictures were closer than the unfiltered
  ones in all 13 tests (four scenes, 4 to 32 rays): by up to 23 % at 4 rays,
  5 to 10 % at 8 rays, 1 to 3 % at 32 rays, where little grain is left.
  
  <a href="docs/img/mc_noise.png"><img src="docs/img/mc_noise.png" width="760" alt="A Monte Carlo picture with 4 rays per pixel, the same with the noise filter, and with 512 rays"></a>
  
  <a href="docs/img/postpro_noise.png"><img src="docs/img/postpro_noise.png" width="260" alt="Post processing: the Noise Filter section"></a>

  The **neural noise filter** is
  [Intel Open Image Denoise](https://www.openimagedenoise.org), the
  filter of Blender's Cycles and many other path tracers, trained on
  rendered pictures. It filters the rendering's averages of the rays
  (before exposure and saturation), so the picture keeps its brightness.
  Intel's library comes with the program (the `oidn` folder; in the .deb
  and .rpm, `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` can name another one.

  * **Monte Carlo window ▸ -noise AI**: filters the shown picture, and the
    saved one, as *-noise* does; the rendering goes on unfiltered.
  * **Command line**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * It takes 0.2 to 0.6 seconds for a 640 x 480 picture on the processor.

  The difference from the 512-ray picture (mean over all pixels, lower is
  better):

  | Scene | 4 rays | DCT filter | neural filter | 8 rays | DCT filter | neural filter |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  The neural filter is closest in three of the four scenes; on the Julia
  set's fine grain the DCT filter is a little closer. It shows most on
  glass and mirrors, where few rays leave the most grain:

  <a href="docs/img/mc_noise_ai.jpg"><img src="docs/img/mc_noise_ai.jpg" width="760" alt="A Monte Carlo picture with 4 rays per pixel, with the DCT filter, with the neural filter, and with 512 rays"></a>

  </details>
* **Transparent backgrounds** for pictures and animations.

  <a href="docs/img/transparent.png"><img src="docs/img/transparent.png" width="300" alt="Transparent background"></a>

  <details><summary>Read more</summary>

  Pictures and animation frames can be saved with a **transparent
  background**, to put the fractal onto another picture or into a video
  editor: **Save pic ▸ PNG ▸ PNG transparent background**, and in the
  Animation maker **Transparent background** (PNG frames). With anti-aliasing (a
  viewing scale of 1:2 or more, or `--aa`) the edges are soft, and they
  carry no fringe of the old background colour. Depth pictures for
  compositing were already there (*Save Z buffer too*, `--depth`).
  
  <a href="docs/img/transparent.png"><img src="docs/img/transparent.png" width="760" alt="A picture with its background, and with a transparent background shown over a checkerboard"></a>

  </details>
* **Videos of animations:** an AVI file straight from the program, or
  the ffmpeg command for a small MP4.

  <a href="docs/img/anim_video.png"><img src="docs/img/anim_video.png" width="300" alt="Video of an animation"></a>

  <details><summary>Read more</summary>

  In the Animation maker, **Also make a video (AVI)** joins the frames into
  a video when they are rendered, at the **Frames per second** you set:
  `<project name>.avi` in the output folder, next to the frames, which are
  kept. It plays in VLC and mpv and opens in video editors (Kdenlive,
  Shotcut, DaVinci Resolve). Each frame is stored as a whole picture, so the
  files are big: good for editing and keeping, less for sharing.
  
  For a small video to share, **FFmpeg command** puts the command for an MP4
  (or, with a transparent background, a WebM) on the clipboard and in the
  messages. Install [ffmpeg](https://ffmpeg.org), paste the command into a
  terminal when the frames are done, and it makes the video; this program
  starts no other programs itself.
  
  <a href="docs/img/anim_video.png"><img src="docs/img/anim_video.png" width="760" alt="The Animation maker with Also make a video (AVI), Frames per second and FFmpeg command"></a>

  </details>
* **A preview of parameter files** in the Open dialog.

  <a href="docs/img/open_preview.png"><img src="docs/img/open_preview.png" width="300" alt="Preview when opening parameters"></a>

  <details><summary>Read more</summary>

  **Open ▸ Open m3p** and **Open m3i** show a small picture of the
  clicked parameter file, with its size and formulas, before it is
  opened; the arrow keys move through the list with the preview. The
  picture is calculated in the background on the processor, so the
  dialog stays responsive, and each file's preview is kept while the
  program runs.
  
  <a href="docs/img/open_preview.png"><img src="docs/img/open_preview.png" width="760" alt="The Open dialog with the preview of a parameter file"></a>

  </details>
* **Navigator: exact values and Reset view.** Click a value and type it;
  go back to the view the Navigator started from.

  <a href="docs/img/navigator_values.png"><img src="docs/img/navigator_values.png" width="300" alt="Navigator: exact values and Reset view"></a>

  <details><summary>Read more</summary>

  * **Click a value** in the Navigator's panels — Julia values, formula
    values, 4D rotation, the Misc values — and type the exact number, as in
    MB3D (angles in degrees). The slider then continues from that value.
  * **Click the zoom** and type it, when the zoom steps are too coarse (a
    wish from MB3D's issue list); a typed zoom stays (*Fixed zoom*).
  * **Reset view** goes back to the position, direction and zoom the
    Navigator took from the main window; the adjusted values stay.
  
  <a href="docs/img/navigator_values.png"><img src="docs/img/navigator_values.png" width="640" alt="The Navigator with the formula values and the input box of a clicked value"></a>

  </details>
* **A new formula, SphereTree:** a tree of touching spheres that branch
  smaller and smaller, by Tom Lowe.

  <a href="docs/img/spheretree_ball.jpg"><img src="docs/img/spheretree_ball.jpg" width="300" alt="A new formula: SphereTree"></a>

  <details><summary>Read more</summary>

  **SphereTree** is a formula Mandelbulb3D does not have: a tree of spheres
  that touch each other and branch into smaller and smaller spheres, like
  foam or coral. Tom Lowe, who invented the Mandelbox, published it in 2020
  (see the credits below). It is in the formula list with the dIFS formulas
  and has four settings:
  
  * **PackRatio** (0 to 1.22): the size of the spheres; at 1 they just
    touch, above 1 the outer ones break off.
  * **Levels**: how deep the tree goes (6 is plenty for most pictures).
  * **Rotated**: 1 turns every level a twelfth of a turn, and the tree
    becomes a round ball of sphere clusters.
  * **Colour scale**: every branch of the tree has its own colour. Choose
    *2nd choice* (the orbit trap) in the colour settings of the Lighting to
    see them; this setting spreads them over the colour gradient.
  
  It works alone and after dIFS transforms (the third picture repeats it
  with *tilingbrickIFS*), and it is calculated on the graphics card as well
  as on the processor. Set the iterations to 1: the formula calculates the
  whole tree at once. Parameters with SphereTree open only in this program,
  not in Mandelbulb3D.
  
  <a href="docs/img/spheretree_ball.jpg"><img src="docs/img/spheretree_ball.jpg" width="760" alt="A SphereTree ball (Rotated) in the Monte Carlo renderer, at dusk"></a>
  
  | <a href="docs/img/spheretree_close.jpg"><img src="docs/img/spheretree_close.jpg" width="380" alt="A close-up of the sphere foam of SphereTree"></a> | <a href="docs/img/spheretree_swarm.jpg"><img src="docs/img/spheretree_swarm.jpg" width="380" alt="SphereTree repeated with tilingbrickIFS: a swarm of sphere trees"></a> |
  |:---:|:---:|

  </details>
* **A new formula, DomeTree:** a dome that sprouts a ring of smaller
  domes, each with its own ring, by Tom Lowe.

  <a href="docs/img/dometree_mc.jpg"><img src="docs/img/dometree_mc.jpg" width="300" alt="A new formula: DomeTree"></a>

  <details><summary>Read more</summary>

  **DomeTree** is Tom Lowe's second tree after SphereTree (see the
  credits): a ball whose one side grows a ring of domes, each dome its
  own smaller ring, level after level, made of circle inversions. It is
  in the formula list with the dIFS formulas and has five settings:

  * **Levels**: how deep the tree goes (16 is plenty).
  * **Children**: the domes in each ring, 4 to 10.
  * **Bend angle** (0.3 to 1.3 work best): how far the domes bulge; low
    values give flat discs, high ones round domes. Above about 1.3 the
    surface turns grainy.
  * **Colour scale**: every dome has its own colour. Choose *2nd choice*
    (the orbit trap) in the colour settings of the Lighting and a wide
    colour range (for example 0 to 100) to see them.
  * **Start curved**: 1 (the default) starts with the ball, 0 with a flat
    floor that grows the first ring.

  Use a small *Raystep* (0.2; 0.05 with 9 or 10 children): the shape's
  distance estimate is rough, as in TGlad's own Fragmentarium file. Like
  SphereTree it runs on the graphics card and the processor, needs only
  1 iteration, and parameters with it open only in this program.

  <a href="docs/img/dometree_mc.jpg"><img src="docs/img/dometree_mc.jpg" width="760" alt="DomeTree with 8 children in the Monte Carlo renderer, coloured per dome"></a>

  | <a href="docs/img/dometree_5.jpg"><img src="docs/img/dometree_5.jpg" width="250" alt="DomeTree, 5 children, bend angle 1.2"></a> | <a href="docs/img/dometree_flat.jpg"><img src="docs/img/dometree_flat.jpg" width="250" alt="DomeTree, 6 children, bend angle 0.5: flat domes"></a> | <a href="docs/img/dometree_10.jpg"><img src="docs/img/dometree_10.jpg" width="250" alt="DomeTree, 10 children: lace between the domes and a large ball"></a> |
  |:---:|:---:|:---:|

  </details>
* **A new formula, UlamSpiral:** Ulam's spiral of prime numbers in 3D, as
  a city, a skyline, spheres or a block.

  <a href="docs/img/ulam_2d.png"><img src="docs/img/ulam_2d.png" width="300" alt="A new formula: UlamSpiral"></a>

  <details><summary>Read more</summary>

  In 1963 the mathematician Stanisław Ulam sat through "a long and very
  boring paper" at a meeting and doodled on a sheet of squared paper. His
  first idea was to set up chess problems; instead he numbered the squares
  1, 2, 3, … in a spiral, 1 in the middle, and circled the prime numbers.
  The primes lined up on diagonal lines, as if they followed a pattern, and
  they still do when a computer draws millions of them (Ulam tried 100,000
  on the MANIAC II at Los Alamos; Martin Gardner put the picture on the
  cover of Scientific American in 1964). Nobody has fully explained it to
  this day.
  
  <a href="docs/img/ulam_2d.png"><img src="docs/img/ulam_2d.png" width="420" alt="The Ulam spiral: the primes among the numbers 1 to 78,961 on a square spiral, with the start of the spiral drawn in the corner"></a>
  
  **UlamSpiral** builds that spiral in 3D, one shape per number, in four
  forms (*Mode*):
  
  * **0, prime towers**: a plate for every number and a tower on each
    prime. The diagonal lines of primes become rows of towers.
  * **1, divisor skyline**: a column for every number, the higher the more
    divisors it has; the primes are the low streets between them.
  * **2, divisor spheres**: a sphere for every number, the larger the more
    divisors it has (the primes are the small ones).
  * **3, a block of primes**: the numbers wound around cube shells instead
    of a square, a cube on each prime.
  
  *Rings* sets the size of the spiral (60 rings are the numbers up to
  14,641), *Height* the height of the towers or columns, *Gap* the space
  between them. With *2nd choice* (the orbit trap) in the colour settings
  the colour follows the number of divisors, so the primes have a colour
  of their own. Like SphereTree it runs on the graphics card and the
  processor, works after dIFS transforms, needs only 1 iteration, and
  parameters with it open only in this program.
  
  <a href="docs/img/ulam_city.jpg"><img src="docs/img/ulam_city.jpg" width="760" alt="UlamSpiral, prime towers: the diagonal rows of prime numbers as a city, in the Monte Carlo renderer with fog, hard shadows and depth of field"></a>
  
  | <a href="docs/img/ulam_skyline.jpg"><img src="docs/img/ulam_skyline.jpg" width="250" alt="UlamSpiral, divisor skyline"></a> | <a href="docs/img/ulam_spheres.jpg"><img src="docs/img/ulam_spheres.jpg" width="250" alt="UlamSpiral, divisor spheres"></a> | <a href="docs/img/ulam_block.jpg"><img src="docs/img/ulam_block.jpg" width="250" alt="UlamSpiral, a block of primes on cube shells"></a> |
  |:---:|:---:|:---:|

  </details>
* **A stopped render continues where it was.** What a render calculated
  is kept when it is stopped, and also when the program is closed or
  crashes, or the computer goes off (it is saved to the disk every 5
  minutes): Calculate 3D, Monte Carlo (also in the middle of a pass), the
  frame of an animation or the picture of a batch under way, a voxel
  export, a Bulb Tracer mesh, Big render tiles, and a swarm render (Stop
  on the computer that made the job keeps every finished part, and each
  computer keeps the part it was at). Calculate 3D and Monte Carlo of the
  same parameters ask whether to continue, the others continue by
  themselves; the **Resume** button (Open tab) lists the unfinished
  renders, to continue or delete them. On the command line the same
  command continues by itself (also a picture in `--tiles`, `montecarlo`,
  `animate`, `batch`, `voxel` and `mesh`); `--fresh` starts again, `mb3d
  resume` lists them. A continued render is the same as one calculated in
  one go.
* **Rendering shared between your computers**, a "swarm": computers that
  see one shared folder render a picture, a Big render, a Monte Carlo
  picture or an animation together. See
  [Rendering on several computers](#rendering-on-several-computers-the-swarm).
* **A new formula, PseudoKleinian:** the "Kleinian architecture" of
  halls, arches and spheres as one formula with its own exact distance.

  <a href="docs/img/kleinian_hall.jpg"><img src="docs/img/kleinian_hall.jpg" width="300" alt="A new formula: PseudoKleinian"></a>

  <details><summary>Read more</summary>

  The halls, arches, pillars and nested spheres known as "Kleinian
  architecture" are among the best-loved 3D fractal pictures. In MB3D they
  can only be made as a hybrid of several formulas with carefully matched
  settings, the recipe Theli-at found in 2011; knighty then wrote the same
  thing as one formula with its own distance estimate, and that is
  **PseudoKleinian**. Each level folds the point into a box, inverts it in
  a sphere and adds an offset, and the distance is measured exactly to a
  floor plane after the folds, so the walls come out clean and the picture
  renders fast. The settings are knighty's: *Levels* (8), *Size* (the
  inversion sphere), *CSize X/Y/Z* (the box; his 0.924, 0.908, 0.924 give
  the classic halls, and small changes give new buildings), *Julia X/Y/Z*
  (an offset per level), *Shape* (0 the floor plane, 1 a cylinder-plane of
  *Thickness* through *Offset*, 2 a rounded box), *DE offset* (hollows the
  surface) and *Colour scale* (with *2nd choice*, the orbit trap: the
  smallest orbit radius, the classic Kleinian banding).
  
  The structure fills all of space, so you look at it from inside: start
  with the view centre near the origin and the camera a unit or two back,
  and move through the halls with the Navigator. In the Monte Carlo
  renderer an interior gets almost no sky light, so put a positional light
  into the hall (the first picture) or cut the structure open with a
  cutting plane and look in from above (the second). It runs on the graphics
  card and the processor, needs 1 iteration, works after dIFS transforms
  and in DE combinations like the other dIFS formulas (though inside a
  space-filling structure those change little), and parameters with it
  open only in this program.
  
  <a href="docs/img/kleinian_hall.jpg"><img src="docs/img/kleinian_hall.jpg" width="760" alt="PseudoKleinian from knighty's viewpoint: a hall of arches and nested spheres lit by a lamp inside, in the Monte Carlo renderer with fog, hard shadows and depth of field"></a>
  
  <a href="docs/img/kleinian_top.jpg"><img src="docs/img/kleinian_top.jpg" width="760" alt="PseudoKleinian cut open with a cutting plane and seen from above: the lattice of spheres and the cells between them"></a>

  </details>
* **Stereo pairs in one step**: both eye images rendered and shown side by
  side, for parallel or cross-eyed viewing. Contributed by Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="docs/img/stereo_pair.jpg"><img src="docs/img/stereo_pair.jpg" width="300" alt="A stereo pair of Dainbramage's Hydra, side by side for parallel viewing"></a>

  <details><summary>Read more</summary>

  **Stereo ▸ Render stereo pair** renders the left and then the right eye
  image with the tab's stereo settings and shows them side by side in the
  main window. They are placed for parallel viewing (left image on the
  left); **Cross-eyed preview** swaps them for cross-eyed viewing. It is a
  preview: lighting and colours
  cannot be changed afterwards without rendering again, and it does not
  work with tiles. The buttons for the single eye images are still there.
  
  <a href="docs/img/stereo_pair.jpg"><img src="docs/img/stereo_pair.jpg" width="760" alt="A stereo pair of Dainbramage's Hydra, side by side for parallel viewing"></a>

  </details>
* **Time left for the whole picture**: the bar under the picture fills once
  over the whole calculation, and beside it the time left until the picture
  is finished, shadows, occlusion and reflections included.

  <a href="docs/img/time_left.png"><img src="docs/img/time_left_bar.png" width="300" alt="The progress bar and the time left in total"></a>

  <details><summary>Read more</summary>

  MB3D's bar starts again at every stage. Here it shows the whole
  calculation: the main rendering, then the hard shadows and ambient
  occlusion, the reflections and the depth of field. How long the stages
  after the main rendering take is estimated from what the scene asks for
  and how the main rendering went; the estimate was fitted on about 1400
  renders of 366 scenes. A picture calculated again expects the times it
  took the last time.
  
  The first picture with new formulas on the graphics card waits for its
  shaders to compile: the line then says *shaders compiling* with the time
  left, which takes the processor's pace and, when the shaders will be
  ready in time, the card joining in. Details in
  [TECHNICAL.md](docs/TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="docs/img/time_left.png"><img src="docs/img/time_left.png" width="640" alt="The main window during the hard shadows and ambient occlusion, with the time left in total"></a>

  </details>
* **History** behind the clock icon of the title bar: every picture you
  calculated with its thumbnail, every file you opened or saved, as a list,
  as rows or as tiles of any size, grouped by day or in one run; search
  it, sort it, show only the pictures that differ, and load any entry
  with a double click. A history of hundreds of thousands of entries
  opens at once.
* **AI assistants can use mb3d** (`mb3d mcp`, the Model Context
  Protocol): ask Claude's desktop app or Claude Code in plain words, or by
  voice through the assistant, to describe, change and render your
  parameter files, breed variations or render what you worked on last;
  the assistant sees the pictures and can adjust them. The **AI** button
  on the Prefs tab connects it with a click (or `mb3d mcp --setup`). mb3d stays offline (the assistant's app starts it on your
  computer), writes only into the `AI` folder and never deletes anything;
  pictures above 4K need your agreement.
* **A log of everything** behind the page icon of the title bar: every
  click, setting and typed value, every calculation with its stages and
  times, files opened and saved, windows, messages, warnings and errors,
  in a window that follows them live and in a file a day. A session that
  froze or was killed is reported at the next start, and a crash report
  carries the log's last lines.
* **Lighting stays live** after hard shadows and reflections: move a light
  or change its colour and the picture changes at once, as in Mandelbulb3D.

  <a href="docs/img/lighting_live.png"><img src="docs/img/lighting_live_small.png" width="300" alt="The main window and the Lighting window right after a lighting change"></a>

  <details><summary>Read more</summary>

  Render plainly, then try out light positions and colours: each change in
  the Lighting window repaints at once with the shadows as they were
  calculated, as in Mandelbulb3D. Changing a light's colour or intensity
  never calculates the shadows again. The shadows of a light you moved,
  the reflections and the depth of field come back with the buttons of
  the Post processing window, as in Mandelbulb3D.
  
  A calculation you stopped can also still be continued after you changed
  the lighting. A way of working with it: [Lighting workflow](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow)
  in the wiki. Details in
  [TECHNICAL.md](docs/TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="docs/img/lighting_live.png"><img src="docs/img/lighting_live.png" width="760" alt="The main window and the Lighting window right after a lighting change: the picture already brighter"></a>

  </details>
* **A ground plane or water** under (or behind) the fractal, an idea from
  Mandelbulber: a plane across the X, Y or Z axis, calculated together
  with the fractal, with its shadows, ambient occlusion, reflections and
  Monte Carlo light, in its own colour, flat or with waves.

  <a href="docs/img/ground_water.jpg"><img src="docs/img/ground_water.jpg" width="760" alt="A floor with the fractal's shadow, reflecting water with waves, and a Monte Carlo render on a green floor"></a>

  <details><summary>Read more</summary>

  Main window, page *Cutting*: tick *Ground plane*, choose the axis and
  the position, the colour, how much it reflects (0 to 1: a mirror when
  the reflections are calculated) and the waves' height and length. The
  solid side is always the one away from the camera. It is calculated on
  the graphics card and on the processor, and left out of the voxel and
  mesh exports. Mandelbulb3D has no place for it in its parameters:
  mb3d-rs keeps it in a small block after them (in .m3p and .m3i files,
  and as `{mb3d-rs: ...}` after the title of text parameters), which
  Mandelbulb3D does not read. Scene keys `ground`, `ground_color`,
  `ground_reflect`, `ground_waves`; details in
  [TECHNICAL.md](docs/TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).

  </details>
* **Light from an HDR picture**: a 360° Radiance `.hdr` picture as the
  background lights a Monte Carlo render, with its full brightness, and
  its sun becomes a light with soft shadows (an idea from Mandelbulber).

  <a href="docs/img/hdr_sun.jpg"><img src="docs/img/hdr_sun.jpg" width="640" alt="A Monte Carlo render lit only by an HDR sky: left the sun found by chance with bright speckles, right the sun as a light"></a>

  <details><summary>Read more</summary>

  Choose the `.hdr` file as the background picture in the Lighting window
  (not "direct"). Monte Carlo then lights the scene with the sky, and the
  brightest spot of the picture, its sun, is taken out and added as a light
  with shadows of the sun's size: found only by chance, the sun would leave
  bright speckles (left). Background pictures of 8 or 16 bits stay as they
  were.

  </details>
* **OpenEXR pictures** for compositing (Blender, Nuke, DaVinci Resolve):
  the colours in floating point, not clipped above white, with the layers
  opacity, depth and normals. *Save pic* > *OpenEXR*, or `mb3d -o
  picture.exr` (also with tiles and the swarm).

  <a href="docs/img/exr_layers.jpg"><img src="docs/img/exr_layers.jpg" width="760" alt="The layers of an OpenEXR picture: the colours as saved, the colours as stored (linear), the normals and the depth"></a>

  <details><summary>Read more</summary>

  The colours are linear, as OpenEXR expects: shown with the usual display
  curve they look as the PNG, and the highlights keep their brightness above
  white. The layers: `A` (opacity, as for a transparent PNG), `Z` (the
  distance from the camera in fractal units) and `N.X`, `N.Y`, `N.Z` (the
  surface normals). Details in [TECHNICAL.md](docs/TECHNICAL.md).

  </details>
* **A fisheye camera and 360° stereo for VR** (ideas from Mandelbulber):
  a fisheye of up to 360°, and panoramas for VR headsets with depth in
  every direction.

  <a href="docs/img/fisheye_water.jpg"><img src="docs/img/fisheye_water.jpg" width="640" alt="A fractal on water through the fisheye at 120 and 200 degrees: the horizon curves"></a>
  <a href="docs/img/vr_panorama.jpg"><img src="docs/img/vr_panorama.jpg" width="300" alt="A 360 degree stereo panorama, the left eye above the right"></a>

  <details><summary>Read more</summary>

  Main window, page *Camera*: *Fisheye* (the field of view, FOVy, up to
  360°, spans the picture's height; a square picture shows a round view).
  A 360° panorama with stereo gives each ray its own eye position, turned
  with the direction it looks in, so the depth is right all around:
  `mb3d -s optic=2 --stereo-pair top-bottom` writes both eyes, the left one
  above, as VR players expect (a 2:1 picture per eye). The eyes are 6.5 cm
  apart, the point looked at as far away as the stereo viewing distance
  (`stereo_screen`'s second value, 2 m). Mandelbulb3D has no fisheye: the
  mb3d-rs block of the parameters keeps it.

  </details>
* **Volumetric clouds** (an idea from Mandelbulber): a layer of clouds
  above, below or around the fractal, lit by the scene's light, in front of
  and behind the fractal.

  <a href="docs/img/clouds.jpg"><img src="docs/img/clouds.jpg" width="480" alt="A fractal over the sea under a sky of clouds"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Clouds*: tick *Paint clouds*, set the
  layer (an axis and two heights on it), how thick the clouds are, how much
  of the sky they cover, the size of their shapes and their colour, then
  *Paint now*. They are painted over the finished picture in a fraction of
  a second, so trying out settings is quick; each pixel looks through the
  layer up to the surface it hit. An animation can move them
  (`cloud_offset` between keyframes). Reflections show them, and Monte Carlo too; Mandelbulb3D does not have them (the mb3d-rs block of the
  parameters keeps them). They cast their shadows on the fractal and the ground, also from outside the picture.

  </details>
* **Surface materials** (ideas from Mandelbulber) on top of the palette:
  Fresnel (more reflection at grazing angles, as glass and lacquer have),
  iridescence (the shimmering colours of a thin film) and glow.

  <a href="docs/img/material.jpg"><img src="docs/img/material.jpg" width="760" alt="Iridescence, iridescence with Fresnel and reflections, and glow"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Material*: tick *Surface material*, set
  Fresnel (0 to 1), iridescence (0 to 1, and a shift of its colours) and glow,
  then *Paint now*. Fresnel shows best with the reflections calculated. Off,
  the pictures are exactly Mandelbulb3D's; the ground plane keeps its own
  colour and reflection. Also in Monte Carlo.

  </details>
* **Two fractals in one scene** (an idea from Mandelbulber): with a DE
  combination, the second hybrid part can stand apart from the first,
  moved, turned and scaled, combined as union, intersection or cut.

  <a href="docs/img/two_objects.jpg"><img src="docs/img/two_objects.jpg" width="360" alt="A Mandelbulb and an Amazing Box side by side"></a>

  <details><summary>Read more</summary>

  Formula window, with a DE combination chosen: *Part 2 at* (its middle),
  *Size* and *Turned* (degrees about x, y and z). Union (*Min*) puts the two
  objects side by side, intersection (*Max*) keeps only what both share, the
  inverted max cuts the second out of the first, the smooth union melts them
  together. Calculated on the graphics card and the processor; the mb3d-rs
  block of the parameters keeps the place (Mandelbulb3D shows the two parts
  in one place).

  </details>
* **Animation curves** (an idea from Mandelbulber): a value of the scene
  over all frames of an animation, and its keyframes as handles to drag.

  <a href="docs/img/curves.png"><img src="docs/img/curves.png" width="560" alt="The Curves window: the zoom over all frames, the keyframes as squares, one chosen"></a>

  <details><summary>Read more</summary>

  *Animation maker* > *Curves*: choose the zoom, a coordinate of the position,
  an angle of the rotation, the field of view, Z start, the ground's position
  or the clouds' drift, and see it over every frame as the animation
  interpolates it. The keyframes are squares on their frames: drag one up or
  down, or click it and type a value. The quadratic Bezier interpolation
  passes beside the keyframes, and the curve shows that before anything is
  rendered. A third interpolation, *Catmull-Rom* (not in MB3D), is smooth and passes through every keyframe (`mb3d animate --catmull-rom`).

  </details>
* **Finishing the picture**: exposure, a glow around bright light, filmic
  tone mapping, contrast, saturation, warmth, a vignette and the colour
  fringes of a lens, as a last pass over the whole picture.

  <a href="docs/img/finishing.jpg"><img src="docs/img/finishing.jpg" width="760" alt="A temple at dawn as calculated, and finished: glow around the bright pillars, softer highlights, warmer light, darker corners"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Finishing*: tick *Finish the picture*,
  set the exposure in stops, *Filmic* (bright light rolls off softly instead
  of turning flat white), the glow (strength, reach as a part of the
  picture's diagonal, and from which brightness it glows), contrast,
  saturation, warmth, vignette and colour fringes, then *Paint now*. The
  painting then runs in floating point, so the light above white is kept
  until the tone mapping. It applies to every picture, animation frame and
  Monte Carlo render (after the noise filter); off, the pictures are exactly
  as before. Scene keys `finish_exposure`, `finish_tonemap = filmic`,
  `glow = strength, radius, threshold`, `finish_contrast`,
  `finish_saturation`, `finish_warmth`, `vignette`, `chromatic`. In a big
  render made of separate tile pictures each tile glows on its own.

  </details>
* **A fine relief on the surface**: rough, stone (ridges and cracks) or
  grain, finer than the calculation's detail, with a colour variation.

  <a href="docs/img/relief.jpg"><img src="docs/img/relief.jpg" width="760" alt="A smooth part of a Mandelbulb, and the same with the stone relief and colour variation"></a>

  <details><summary>Read more</summary>

  Post processing window, section *Material*: *Relief* with its strength, the
  size of its features (in the fractal's units, so it stays on the surface
  when the camera moves) and its fineness, and *Colour variation*. It tilts
  the surface where the light is calculated, so shading, highlights,
  reflections and Monte Carlo all see it; the shape itself is not changed
  (the outline stays smooth). Not on the ground plane. Scene keys
  `detail = stone, 1.5, 0.12, 0.5` (kind, strength, size, fineness) and
  `detail_color`.

  </details>
* **Turntables, recorded flights and motion blur** for animations.

  <a href="docs/img/motion_blur.jpg"><img src="docs/img/motion_blur.jpg" width="640" alt="A frame of a fast turntable, without and with motion blur"></a>

  <details><summary>Read more</summary>

  *Animation maker* > *Turntable*: eight keyframes of one whole turn around
  the middle of the main window's view, a smooth loop (`mb3d animate
  scene.m3p --orbit 8 --frames 25`). *Navigator* > *Rec*: while the button is
  down, every new view of the flight (at most two a second) becomes the next
  keyframe; with Catmull-Rom the animation passes through all of them.
  *Motion blur* in the Animation maker: each frame is the average of that
  many pictures within the *Shutter* time (a part of the time to the next
  frame), so what moves smears; it takes that many times as long
  (`mb3d animate --motion-blur 8,0.5`).

  </details>
* **Gaussian splats** of the fractal, to look at from every side in
  Blender (5.3 and later), SuperSplat and other splat viewers:
  `mb3d splat scene.m3p -o fractal.ply`.

  <a href="docs/img/splats.jpg"><img src="docs/img/splats.jpg" width="760" alt="The program's picture of dIFS heart shrub, and its Gaussian splats in Blender from the left and from behind"></a>

  <details><summary>Read more</summary>

  The fractal is calculated from 16 directions (`--views`), each a
  picture of 640 x 640 pixels (`--size`), with its lights, shadows,
  occlusion and reflections. Every pixel of the object becomes a flat
  Gaussian lying on the surface, with the colour it has in the picture.
  Where the pictures overlap, each patch of the surface comes from the
  one that saw it most squarely. The colours change with the direction you
  look from (spherical harmonics, `--sh`), so highlights move as they do in
  the pictures.

  An object seen from outside is calculated from all around the scene's
  middle. A scene seen from inside (a hall, a cave, where the fractal fills
  the whole picture) is calculated from the scene's camera, looking in
  every direction (`--around camera`; `auto`, the default, chooses). There,
  a larger `--size` keeps more detail, as each view is 110 degrees wide.

  The lights that turn with the camera, and the ambient light's top and
  bottom colours, are fixed to the object as they light the scene's own
  view, so all sides are lit alike. Depth of field, clouds, glow, vignette
  and the ground plane are left out. The file is in the scene's view (x to
  the right, y into the picture, z up: Blender's front view looks as the
  scene does), the middle or the camera at 0 and the fractal scaled to a
  radius of 1. It takes about 160 bytes per splat, typically 100 to 300 MB;
  `--sh 0` makes it a third of that, without the moving highlights.

  </details>

## Download

Ready-to-run programs for the latest release, no installation needed:

| System | Download |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (installs like other programs) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (installs like other programs) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
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

It checks the download before unpacking it, puts **Mandelbulb3D** in
your application menu (Linux) and makes `Mandelbulb3D` and `mb3d`
commands you can type in any folder. `sh ~/mb3d-rs/download.sh
--uninstall` takes the menu entry and the commands away again; your files
stay.

The Linux programs run on distributions from about 2018 on (Ubuntu 18.04,
Debian 10, Rocky/RHEL 8 and newer).

**Installed like other programs:** on Debian, Ubuntu and Mint, open the
`.deb` file above (or `sudo apt install ./mb3d-rs-linux-x64.deb`); on
Fedora, RHEL and Rocky the `.rpm` file (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, on openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D is then in the application menu and `mb3d` works in any
folder; your settings and History are kept in `~/.config/mb3d-rs` and
`~/.local/share/mb3d-rs`.

[Getting started](GETTING_STARTED.md) walks through the first steps; older
versions and the release notes are on the
[releases](https://github.com/thr32423/mb3d-rs/releases) page.

## Guides and help

Step-by-step guides are in the [wiki](https://github.com/thr32423/mb3d-rs/wiki):

* [Installing](https://github.com/thr32423/mb3d-rs/wiki/Installing): Windows, macOS and Linux, updating, where your files are
* [Lighting workflow](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow): render plainly, then try out lights, shadows and reflections
* [Swarm setup](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup): rendering on several computers through a shared folder
* [Troubleshooting and FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ): when something does not work as expected

Every release with its changes: [the whole list](docs/UPDATES.md).

## Rendering on several computers (the swarm)

Computers that see one shared folder render together: a picture in
tiles, a Big render, a Monte Carlo picture, an animation or a batch. Each
computer takes the parts nobody else has, so a fast one takes more of
them, and the computer that started the render puts the results
together. The folder does the sharing: the program opens no network
connection and talks to nobody, so it is as safe as your file sharing.

<a href="docs/img/swarm_window.png"><img src="docs/img/swarm_window.png" width="760" alt="Mandelbulb3D with the Swarm window, opened with the Swarm button on the Tools tab, while two computers render a picture in tiles: the shared folder, Share this computer, the computers and what each is doing, the job, and the preview with rendered tiles, a tile under way and missing ones"></a>

### Setting it up

Step by step with what to check: [Swarm setup](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup) in the wiki.

1. **A folder every computer can write to:** a share on a NAS, a Windows
   share, a Samba share or NFS. On Linux, mount it as a folder, for
   example
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   a share only opened in the file manager (`smb://...`) is not a folder
   the program can use.
2. **The same version of mb3d-rs on every computer.** A computer with
   another version is listed but takes no part.
3. **In Mandelbulb3D:** the **Swarm** button on the Tools tab. Choose the
   shared folder (the same share on every computer; its path may differ),
   give the computer a name for the list, and tick **Share this
   computer**.

That is all. The status line says what happens, for example *Sharing
with omarchy: renders are split between the 2 computers*. Alone in the
swarm, everything renders on this computer as usual.

### What is shared

While *Share this computer* is ticked, the computer renders the parts
other computers put into the folder, and its own renders go to the swarm
whenever another computer is in it:

* **Calculate 3D** (and the Batch window) — the picture in N x M tiles
  (set in the Swarm window), put together here with everything the usual
  calculation gives: lighting changes, post processing, saving as m3i (and
  its memory, see below).
* **Big render** — the tiles are saved one by one as they arrive; the
  whole picture is never held in memory, so this is the way for pictures
  of 100000 x 100000 pixels and more.
* **Monte Carlo** — as on one computer: every computer adds passes to its
  own block until you press Stop, the picture cleans up with everyone's
  rays, and Stop averages the blocks. In the window's batch mode the
  blocks together stop at the batch's ray count.
* **The Animation maker** — the frames, one by one.

The window lists the computers with their cores, graphics card and what
each is doing, the jobs with their progress, and a preview of the newest
job: the tiles rendered, the ones under way with a progress bar, and the
ones still missing.

### On the command line

On every computer that should help, start a worker on the folder:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Then, on any computer, render with `--swarm` and the same folder:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` shows who is in the swarm, what each computer is
doing and how far the jobs are. A Monte Carlo picture on the command line
is split into blocks with their own random rays: 4 blocks of 64 rays per
pixel give the 256 asked for with `--rays 256`.

### Good to know

* **If the other computers stay idle** and their job list is empty while
  this computer renders: the computer that serves the folder with Samba
  and uses it directly lets the others keep an old listing of it. Turn
  that caching off in `/etc/samba/smb.conf`: `smb3 directory leases = no`
  in `[global]`, and `oplocks = no` and `level2 oplocks = no` in the
  share's section, then `sudo systemctl restart smb` and mount the share
  again on the other computers.
* **Memory:** a picture takes about 20 bytes per pixel in this program
  (its image buffer), 10 more in the main window, and then the larger of
  two stages: the main calculation with its shadows and occlusion (16 per
  pixel and 1.2 GB for the card), or the painting (3 per pixel, 20 more
  with depth of field, and one band of the reflections: 3.5 GB at most,
  less when little memory is free, since the band is sized to half of what
  is free when the pass starts). A 12000 x 6000 picture: 6 GB. When a
  picture needs more than the free memory of the computer (asked of Linux,
  Windows and macOS), Calculate 3D says how much it needs and how much is
  free, and asks before it starts: the calculation would swap and the
  program could stop responding for minutes at a time. Big render, alone
  or with the swarm, is the way out: its tiles are saved one by one, the
  whole picture is never held. Calculate 3D with the swarm holds the
  picture here as the usual calculation does (without the main
  calculation's stage, which the tiles' computers do with each tile's
  memory); a computer of the swarm leaves a tile too big for its memory to
  the others.
* **Safety:** a computer renders only what a job asks for and writes only
  into the job's folder; formula code from a job runs in the formula
  emulator like every formula.
* **Speed:** tiles travel as full image buffers, a few hundred megabytes
  for a very big picture, so a fast share pays off. A sync service that
  copies with a delay works too, but slower.
* **Computers that go away:** the part a computer was rendering is taken
  by another once it has not been heard of for half a minute. A job whose
  owner has gone for half an hour is removed by whoever sees it. A job
  that fails on a computer is noted in the window and left to the
  others.
* **Stopping:** unticking *Share this computer* lets the part under way
  finish first; closing the program gives it up, and another computer
  takes it.
* **Stopping the job** (Stop on the computer that made it, or closing
  it, or it crashes): every part already finished is kept on that
  computer, and each computer keeps the part it was rendering. Starting
  the same render again makes a job of only the missing parts, and a
  computer that gets back a part it had begun continues it. The same
  works on the command line: run the same command again. An open-ended
  Monte Carlo render (the window's) puts its blocks together when it is
  stopped, as before.

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
| **Auto** (default) | The card and the processor share the picture: the card takes rows from the top, the processor's threads from the bottom, until they meet. The shadows and the ambient occlusion are shared the same way, or done by the card alone when it is clearly the faster one. |
| **CPU** | The processor only, as in MB3D. |
| **Graphics card** | The card only (it waits until its program is ready). |

The first time a combination of formulas is used, the graphics driver
has to prepare a program for the card — the *shader* — which takes 10 to
15 seconds on the computer measured below. In Auto mode the processor
simply starts without it and the card joins when it is ready, so a small
picture never waits. The prepared shader is kept for the next time (by
the graphics driver, on Linux in the `ShaderCache` folder next to the
program), so the next picture with the same formulas starts on the card
at once.

The **LED** right of the window title tells where the last picture was
calculated: green on the graphics card, orange partly, red on the
processor. Pointing at it shows the details.

[How the rendering works](docs/RENDERING.md) tells the whole story with
pictures: what Auto does the first time, why the card is not always the
faster choice, and what the colours of the LED mean.

Not everything can go to the card: a few formulas cannot be translated for
it, and views zoomed in very deeply are calculated in double precision
(which only some cards can do). The program decides all of this by itself; you only see it in the
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

* **mb3d-rs on the processor alone is faster than MB3D** in 71 of the 79 files, a median **1.7 times**; all files together took MB3D 18 min 21 s and mb3d-rs on the processor 17 min 10 s.
* **Auto mode** is faster than MB3D in 77 of 79 files (median 2.1 times); all files together 8 min 17 s. In 30 of the files the processor finished alone, mostly because it needs only a few seconds, before the card gets a turn — there Auto is simply the processor plus a moment's overhead. The card joined in 49 files, the slower ones, where it made the picture a median 1.6 times faster than the processor (up to 13 times: *Insect Menger*, 4.7 s instead of 63.0 s).
* In 4 files Auto was noticeably slower than the processor alone (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): there the card is slower than the 24 processor threads for the scene.
* "Rows on the card" is the share of the picture the graphics card marched in Auto mode (– when the processor finished alone).

<details>
<summary>All 80 files (click to open)</summary>

| Parameter file | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | rows on the card | Auto vs MB3D |
|:--|--:|--:|--:|--:|--:|
| 0Encrypted0 - ABoxSOff4d example | 12.8 s | 7.4 s | 1.9 s | 80 % | 6.7× |
| 6 AM - Torii temple | 23.0 s | 20.4 s | 17.8 s | 60 % | 1.3× |
| Abominog-Ua - 12.06.2014 | 7.6 s | 8.0 s | 7.4 s | 50 % | 1.0× |
| ABoxPartlyJuliaRoff | 2.5 s | 1.4 s | 1.6 s | – | 1.6× |
| ABoxScale2Start | 1.0 s | 0.1 s | 0.3 s | – | 3.3× |
| ABoxScale3Start | 1.5 s | 0.1 s | 0.3 s | – | 5.0× |
| AboxShapes-Misty | 27.5 s | 25.6 s | 17.7 s | – | 1.6× |
| Aexion 10bulbs | 11.5 s | 9.8 s | 2.6 s | 73 % | 4.4× |
| AkuraPare - 463b | 4.0 s | 2.0 s | 0.7 s | 72 % | 5.7× |
| ApolloBalloons dIFS | 75.8 s | 73.2 s | 24.9 s | 57 % | 3.0× |
| AureliusCat - Bamboo | 2.0 s | 1.2 s | 0.7 s | 58 % | 2.9× |
| BatJorge - Pong 703 - Swirls | 4.0 s | 2.8 s | 3.1 s | – | 1.3× |
| Bill-Snowzell - Tweak - Canyon | 4.0 s | 1.6 s | 1.7 s | 44 % | 2.4× |
| Blood on the dancefloor | 7.6 s | 4.4 s | 3.2 s | – | 2.4× |
| BulboxCut | 4.0 s | 2.3 s | 2.3 s | – | 1.7× |
| Chrystal | 5.6 s | 5.2 s | 1.7 s | 79 % | 3.3× |
| cutted sphere over carpet | 7.1 s | 20.7 s | 4.5 s | 48 % | 1.6× |
| Dainbramage - Home of the Ancients | 3.0 s | 1.3 s | 1.2 s | 25 % | 2.5× |
| Dainbramage - Hydra | 3.5 s | 1.7 s | 1.6 s | 41 % | 2.2× |
| Dainbramage - Nothing left but bones | 4.0 s | 3.0 s | 2.4 s | 49 % | 1.7× |
| DEcomb1 | 83.9 s | 93.5 s | 80.3 s | 48 % | 1.0× |
| dIFS heart shrub | 2.5 s | 3.5 s | 3.0 s | – | 0.8× |
| dIFS heart shrub otrap | 5.1 s | 15.6 s | 3.6 s | 93 % | 1.4× |
| dIFS shrub | 4.0 s | 17.3 s | 3.6 s | 93 % | 1.1× |
| Dodeca Torus mix | 2.0 s | 0.5 s | 0.6 s | 14 % | 3.3× |
| DsyneGrafix - Getting Loopy | 2.0 s | 0.8 s | 0.8 s | 25 % | 2.5× |
| Ellarien - Embellished | 3.5 s | 1.1 s | 1.2 s | 49 % | 2.9× |
| Ellarien - Shrooms are coming | 8.1 s | 6.2 s | 6.8 s | – | 1.2× |
| ExcludeBulbMeng | 17.3 s | 12.9 s | 8.5 s | – | 2.0× |
| Fishbowl | 4.6 s | 3.5 s | 1.9 s | 49 % | 2.4× |
| Fractales75 - Kiosk | 3.0 s | 1.8 s | 1.9 s | 47 % | 1.6× |
| Genetic Menger | 1.5 s | 0.5 s | 0.6 s | 46 % | 2.5× |
| genQuat helix mix | 4.0 s | 3.3 s | 1.8 s | 61 % | 2.2× |
| HalTenny - Freak Of Nature | 2.5 s | 0.9 s | 1.2 s | – | 2.1× |
| Hal-Tenny-Resistance (slow) | 18.9 s | 20.3 s | 8.7 s | 78 % | 2.2× |
| Hyperben2 - Ozosphere tweak | 2.0 s | 0.8 s | 0.6 s | 56 % | 3.3× |
| ImpossibleWorld dIFS | 399.8 s | 367.5 s | 99.0 s | 71 % | 4.0× |
| InAndOutside box | 5.1 s | 2.7 s | 1.4 s | 68 % | 3.6× |
| Insect Menger | 4.0 s | 63.0 s | 4.7 s | 93 % | 0.9× |
| Jost1 - Tweak - New tiny temple | 3.5 s | 1.6 s | 1.0 s | 60 % | 3.5× |
| julia on box | 4.6 s | 4.0 s | 1.6 s | 73 % | 2.9× |
| JuliaSetSample | 3.0 s | 1.9 s | 2.2 s | – | 1.4× |
| KentPress - Parameter 3 tweak fogits | 4.0 s | 2.5 s | 3.0 s | – | 1.3× |
| KochSurf Sample 6 | 3.0 s | 1.4 s | 0.8 s | 75 % | 3.8× |
| Lenord - ASurf 2 | 5.1 s | 1.7 s | 1.9 s | 48 % | 2.7× |
| Lenord - Between Dark and Light | 5.6 s | 3.8 s | 2.1 s | 60 % | 2.7× |
| Lenord - The Beanstalk | 3.5 s | 1.8 s | 2.0 s | – | 1.8× |
| LightBulbMoon - A Fabulous Excuse | 3.5 s | 2.0 s | 1.6 s | – | 2.2× |
| LightBulbMoon - No, Never | 5.1 s | 4.0 s | 1.2 s | 82 % | 4.2× |
| Mandelwerk - Makin seashell | 12.0 s | 8.1 s | 8.1 s | 30 % | 1.5× |
| Mandelwerk - TgladTetra inner spirals | 8.6 s | 4.5 s | 1.1 s | 80 % | 7.8× |
| MarkJayBee - Curvichrome | 2.5 s | 1.3 s | 1.3 s | – | 1.9× |
| material colors | 3.0 s | 1.4 s | 1.9 s | – | 1.6× |
| Melting spot bloxx | 23.2 s | 20.4 s | 7.6 s | 73 % | 3.1× |
| Mengerplus for MC | 3.5 s | 2.2 s | 2.5 s | – | 1.4× |
| MengerTrees | 19.3 s | 15.4 s | 16.9 s | – | 1.1× |
| MutaGenD4B12E71C70D4FD00357_canary~ | 7.1 s | 3.7 s | 4.1 s | – | 1.7× |
| Oxnot - Flexing | 2.5 s | 0.7 s | 0.7 s | 59 % | 3.6× |
| Oxnot - Shells | 5.6 s | 3.3 s | 3.6 s | – | 1.6× |
| PaMonk - Ringlets | 5.1 s | 2.7 s | 2.8 s | – | 1.8× |
| QuatP4hybridJulia | 1.5 s | 0.2 s | 0.4 s | – | 3.8× |
| Rama - Elysium | 11.5 s | 10.6 s | 5.5 s | 74 % | 2.1× |
| Recycledrelatives - Fractal Fan | 2.0 s | 0.4 s | 0.6 s | 31 % | 3.3× |
| Recycledrelatives - Test for SurfMod | 4.6 s | 3.0 s | 3.1 s | – | 1.5× |
| Rowratty - Spirals | 4.0 s | 2.5 s | 1.0 s | 69 % | 4.0× |
| Sabine62 - Housing problem | 11.7 s | 10.5 s | 6.8 s | – | 1.7× |
| spineJulia | 91.9 s | 71.7 s | 50.7 s | 35 % | 1.8× |
| Surreal shell | 2.0 s | 0.6 s | 0.8 s | – | 2.5× |
| Test8 | error in MB3D | 0.1 s | 0.3 s |  | – |
| Theli-At - Black Mandelbulb | 1.0 s | 0.2 s | 0.5 s | 22 % | 2.0× |
| Theli-At - Bone Menger using MH | 2.0 s | 0.7 s | 1.1 s | – | 1.8× |
| Theli-At - MengerSpheres | 2.0 s | 0.6 s | 0.6 s | 50 % | 3.3× |
| ThePearl dIFS | 2.5 s | 1.4 s | 1.2 s | 25 % | 2.1× |
| TimeMachine | 1.0 s | 0.5 s | 0.7 s | – | 1.4× |
| toricaleggs | 8.6 s | 5.6 s | 6.6 s | 40 % | 1.3× |
| TreePlanet | 7.6 s | 6.6 s | 7.5 s | – | 1.0× |
| UndeadAcademy tweak - Peacock in blue | 7.6 s | 4.5 s | 1.2 s | 74 % | 6.3× |
| Virtual tubes world (UndeadAcademy style) | 9.1 s | 6.9 s | 4.3 s | 49 % | 2.1× |
| Wada basin - Reflect test | 6.6 s | 4.1 s | 4.6 s | 40 % | 1.4× |
| Waste-and-tragedy - Mandalex | 5.1 s | 3.4 s | 3.8 s | – | 1.3× |

</details>

The pictures are the same: mb3d-rs on the processor makes MB3D's picture
to within rounding, and the Auto pictures differ from the processor's by a
fraction of a percent of the colour range on average (fine detail may
sparkle in different pixels; the [technical notes](docs/TECHNICAL.md)
have the measurements).

### Experimental speed-ups

A few faster ways of calculating change the picture a little, so they are
off unless you switch them on: **Internal** tab → **Experimental...**, tick
*Use the experimental speed-ups*, then the ones you want. They are saved
with the scene (so the command line, the swarm and the AI assistant use
them too); on the command line `-s experimental=on` with `-s step_relax=1.3`,
`-s secant_search=on`, `-s free_steps=on` or `-s occlusion_half=on`.

Measured on the 80 example files in Auto mode (about 0.4 megapixels,
the faster of two runs each), against the same files without them:

| Speed-up | Whole render | Pictures (pixels off by more than 8 of 255) |
|---|---|---|
| **Step relax 1.3**: longer march steps, an overstep taken again | −5 % (median), up to −20 %; 62 of 80 faster | median 2.6 %; fine shading. *Thin shells can show holes* (BatJorge – Pong 703 – Swirls) |
| **Secant search** for the surface | −2 % (median), up to −18 % | median 0.7 %; fine detail moves slightly |
| **No step limit without dynamic fog** | −1 % (median), up to −30 % in open scenes | mostly the same picture (median 0.03 %) |
| **DE occlusion at half resolution** | −18 % over the 10 files with DE occlusion (DEcomb1 −24 %); that stage up to 4 times faster | 0–27 % of pixels, softer occlusion in fine detail; it looks alike |

Compare with a render without them before relying on one for a picture.

## The command line

`mb3d` renders without a window. Open a terminal in the program's folder
(on Windows a command prompt in the unpacked folder, and type `mb3d`
instead of `./mb3d`). After the one-line install on Linux or macOS, `mb3d`
works in any folder, `man mb3d` shows its manual, and the Tab key
completes its options in bash, zsh and fish:

```sh
./mb3d "M3Parameter/6 AM - Torii temple.m3p" -o torii.png      # full size
./mb3d "M3Parameter/6 AM - Torii temple.m3p" --scale 0.25 -o small.png
./mb3d pasted.txt -o pasted.png        # parameters copied from a forum post
./mb3d City.m3p --tiles 4x4 -o city.png    # a big picture in tiles
./mb3d resume                              # stopped renders: the same command continues
./mb3d --cpu City.m3p -o city.png          # processor only
./mb3d --card City.m3p -o city.png         # graphics card only
./mb3d City.m3p --bits 16 -o city16.png    # 16 bit colour
./mb3d City.m3p --denoise 4 -o city.png    # with the noise filter
./mb3d City.m3p --aa 2 --transparent -o city.png   # transparent background
./mb3d City.m3p -o - | display -          # the picture into another program
./mb3d --help                              # all options
```

Everything else has a sub-command with its own `--help`:

```sh
./mb3d batch M3Parameter --scale 0.2 -o previews        # a whole folder
./mb3d animate start.m3p middle.m3p end.m3p --frames 60 -o frames
./mb3d animate project.m3a -o frames                     # MB3D animations
./mb3d animate project.m3a -o frames --transparent      # ... see-through
./mb3d animate project.m3a -o frames --video flight.avi  # ... and a video
./mb3d montecarlo scene.m3p --rays 64 -o mc.png
./mb3d montecarlo scene.m3p --rays 8 --denoise -o mc.png # ... with the noise filter
./mb3d voxel scene.m3p -o voxels --slices 256            # for 3D printing
./mb3d mesh scene.m3p -o object.stl                      # Bulb Tracer 2
./mb3d splat scene.m3p -o fractal.ply                    # Gaussian splats
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
  itself is faster. See the tables above. On Linux and Windows the
  formulas of a scene's hybrid are also compiled together into one
  function when a picture starts (in a few milliseconds, with Cranelift),
  so that they hand their values on in the processor's registers.
  Together with a faster loop over the formulas (on every system), CPU
  renders of the 80 example files take 20 % less time than in v0.2.74
  (up to 60 % in some), the pictures the same to the bit. A scene keeps the compiled hybrid only if it
  measures faster; `MB3D_NO_JIT=1` switches it off. On macOS it is on
  too, but still being tested there: `./jit_check.sh` in the unpacked
  folder checks it (it compares
  the compiled formulas with the translated ones and renders the example
  files with and without it; the pictures must be the same).
* **Big pictures** can be rendered in one piece (a 15000 x 20000 picture
  needs about 15 GB of memory; with depth of field it takes hours) or in
  tiles, also on several computers (`--tiles`, and the Big renders
  window), which needs less memory. Calculate 3D does not start a
  picture that does not fit into the free memory (on Linux, where the
  free memory can be told) and says so, and a computer of the swarm
  leaves a tile too big for its memory to the others.
* **Unfinished renders** are kept in the cache folder (`~/.cache/mb3d-rs/resume`,
  on Windows `%LOCALAPPDATA%\mb3d-rs\resume`, or next to the program):
  18 bytes per pixel for a picture (a 15360 x 15360 picture takes 4.2 GB),
  64 for a Monte Carlo render. What was calculated is written at once and
  flushed to the disk every 5 minutes. A render stopped within its first
  10 seconds keeps nothing; unfinished renders older than 30 days go, and the
  oldest ones when all of them take more than 50 GB. A render continues
  only with the same parameters and program version (and the same choice
  of card or processor; a Monte Carlo render's exposure and saturation
  may change). A Monte Carlo render stays unfinished until it reaches its
  rays (a batch, `--rays`, `--passes`) or is deleted in the Resume list.
* **Newer or your own formulas, maps, themes and languages** go into
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` and `.../Languages`
  (any way of installing; on Windows `%APPDATA%\mb3d-rs\...`). They
  are used in addition to the ones that
  come with the program, and replace a shipped file of the same name, so
  an updated formula from the forums takes effect without touching the
  program's folder.

## Updates

The newest releases with the time they were published and the changes
(commits) they brought; every release is in [the whole list](docs/UPDATES.md).
Each commit links to its full description and code on GitHub; the downloads
of a release are on its page. Times are in UTC.

<!-- updates:start -->
### [v0.2.90](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.90) · 1 Nov 2026, 09:15 UTC

| Commit | What it does |
|---|---|
| [`1d56285`](https://github.com/thr32423/mb3d-rs/commit/1d56285eeaf3acf23f8af626ea8285ac0be8005f) | Version number and release notes of this release |
| [`02c7d3d`](https://github.com/thr32423/mb3d-rs/commit/02c7d3de88dbf73faaf6bd1518378cd438854c83) | Experimental speed-ups (not in MB3D), off unless switched on, each on its own: step relax (longer march steps, an overstep taken again at its safe length; the dynamic fog keeps its count), secant search for the surface (a probe on the other side interpolated between the two), no step limit without dynamic fog and volumetric light (not with disable analytic DE), the DE occlusion at half resolution (every second pixel of every second row, the others filled from the four around them by depth and normal; on the card an even row's calculated pixels first, so its groups run together); on the processor and the card alike (card vs CPU as without them); the window Experimental speed-ups from the Internal tab's Experimental... button; scene keys experimental, step_relax, secant_search, free_steps, occlusion_half (also in the MB3D-RS block); measured on the 80 example files in Auto mode: step relax 1.3 median -5 %, secant -2 %, free steps -1 % (up to -30 %), half occlusion -18 % over the 10 files with DE occlusion; README (all seventeen languages), TECHNICAL.md, the AI guide; the labels in all languages and the font subsets; tests: the switch, the files, the fill |
| [`2d8ef91`](https://github.com/thr32423/mb3d-rs/commit/2d8ef91fd5dfcbd3c2fadbc113acbc824ded3aeb) | Wrapped texts in Chinese and Japanese break between characters (they have no spaces between words, so a long label was drawn in a smaller font on one line and cut off); not before closing marks, small kana and the wave dash, not after opening brackets; texts without these characters break at their spaces as before |
| [`a215f42`](https://github.com/thr32423/mb3d-rs/commit/a215f42d9fae6aaf2cd4075ca72497ab8198caa3) | Ground plane in double precision on the card: the waves' 2 pi a value of the shader's precision (in double the shader did not compile and every scene in double precision was calculated on the CPU) |
| [`94471ec`](https://github.com/thr32423/mb3d-rs/commit/94471ec8431713e2d3cb90c16b9b29784a2f1544) | README: Updates for v0.2.89 |

### [v0.2.89](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.89) · 1 Nov 2026, 05:45 UTC

| Commit | What it does |
|---|---|
| [`028adca`](https://github.com/thr32423/mb3d-rs/commit/028adcaf697ee35feddbe6009e098c856aca6a08) | Version number and release notes of this release |
| [`7b09cd4`](https://github.com/thr32423/mb3d-rs/commit/7b09cd45099f2558c886c3fe4eb42621f2588f24) | Finishing, surface relief, turntable, flights and motion blur in the documentation and in all seventeen languages: the labels translated (the Japanese, Korean and Chinese font subsets with their new characters; the relief's suggested size rounded to two digits), the READMEs with pictures, TECHNICAL.md's keys, the AI assistants' guide |
| [`d388096`](https://github.com/thr32423/mb3d-rs/commit/d388096434f4242f1f825b3cd2611492a00c8993) | Camera and animation (not in MB3D): a turntable, keyframes for one whole turn of the camera around the middle of the view (Animation window: Turntable, 8 keyframes in a Catmull-Rom loop; mb3d animate SCENE --orbit STEPS); a recorded flight (Navigator: Rec, every new view at most twice a second becomes the next keyframe); motion blur, each frame the average of several pictures within the shutter time (Animation window: Motion blur and Shutter; mb3d animate --motion-blur N[,SHUTTER]; in the text animation files motion_blur and shutter, in MB3D's .m3a bits 21-29 of the options MB3D ignores); the animation's scene between two frames (Animation::frame_scene_at); tests: the turntable, scenes between frames and across keyframes, both file formats |
| [`ae0431e`](https://github.com/thr32423/mb3d-rs/commit/ae0431e74b061de8b9b0401cdb32fa26c17cd88c) | Surface detail (not in MB3D): a fine relief on the fractal's surface from 3D gradient noise fixed to the fractal, rough, stone (ridges and cracks) or grain, with its strength, size and fineness, and a colour variation; it tilts the surface's normal where the light is calculated (the painting, the reflections' mirror on the processor and with the card's marches, Monte Carlo) and varies the colour, not on the ground plane; Post processing window, Material section: Relief and Colour variation; scene keys detail = kind, strength, size, roughness and detail_color (also in the MB3D-RS block); tests: the noise, the tilted normals, the files |
| [`b3c9061`](https://github.com/thr32423/mb3d-rs/commit/b3c9061999ec9164fd62922ba3efee9d40779050) | Finishing of the picture (not in MB3D): exposure in stops, glow around bright light (the light above a threshold blurred on a smaller copy, a wide and a near part), filmic tone mapping (highlights roll off instead of clipping: the painting runs in floating point when finishing is on), contrast, saturation, warmth, a vignette and colour fringes by the whole picture's middle (also on tiles); the last pass after the depth of field, also on Monte Carlo pictures (after the noise filters); Post processing window: a Finishing section; scene keys finish_exposure, finish_tonemap, glow, finish_contrast, finish_saturation, finish_warmth, vignette, chromatic (also in the MB3D-RS block of .m3p/.m3i); a scene without finishing paints as before; tests: neutral changes nothing, the filmic curve, glow and vignette, the files |
| [`2e43553`](https://github.com/thr32423/mb3d-rs/commit/2e435533f721601a32c72252d12b9b42b53581db) | README: Updates for v0.2.88 |

### [v0.2.88](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.88) · 31 Oct 2026, 11:01 UTC

| Commit | What it does |
|---|---|
| [`4026c6a`](https://github.com/thr32423/mb3d-rs/commit/4026c6af0e9c0a18c429d8af41f29d377ad83e33) | Version number and release notes of this release |
| [`ed37940`](https://github.com/thr32423/mb3d-rs/commit/ed37940cbfd2b3009368fb6aa2e5f53e61f300d2) | Clouds in the reflections of the painting: a reflected or refracted ray outside the material goes through the clouds up to its hit (or into the distance), their light weighted by the ray's share joins and what lies behind them gets dimmer, and the surface it hits lies in the clouds' shadow, on the processor and with the rays marched on the graphics card; water and mirrors show the clouds now in every render; scenes without clouds byte-identical, the clouds cost the test picture 54 ms; READMEs (all seventeen languages) and TECHNICAL.md say so |
| [`be186c2`](https://github.com/thr32423/mb3d-rs/commit/be186c248a12399c7c55a6570be9ba4a665915a7) | Clouds in Monte Carlo's diffuse bounces too: every ray after the view ray goes through the clouds, so they hide the sky's light from the surfaces under them and add their own; render time unchanged, scenes without clouds byte-identical (on the processor) |
| [`1499606`](https://github.com/thr32423/mb3d-rs/commit/1499606d0ed2befe3f98c200366b7ef79eca61f0) | Clouds in Monte Carlo's reflections: a reflected or refracted ray goes through the clouds up to its hit (or into the distance), their light weighted by the ray's share joins and what lies behind them gets dimmer, so water and mirrors show the clouds; the diffuse bounces still do not see them, the painting's reflections neither; scenes without clouds unchanged; READMEs (all seventeen languages) and TECHNICAL.md say so |
| [`55bc3af`](https://github.com/thr32423/mb3d-rs/commit/55bc3af1b6ac8fe5823678383e857793f7786a82) | Animation window: the interpolation hint's label id matches the language files (LinearStraightFromKeyframeToKeyframe); tools/check.sh passes |
| [`3d4aa74`](https://github.com/thr32423/mb3d-rs/commit/3d4aa74525b3307c74566c4faf3c186fdd3f1e05) | Catmull-Rom interpolation of animations (not in MB3D): smooth and through every keyframe (the four keyframes around a frame with Catmull-Rom's weights, the rotation as the weighted quaternions normalised, the zoom on its logarithmic scale); Animation window: a third Subframe interpolation choice with a hint on all three; mb3d animate --catmull-rom; the text animation files say catmull-rom, MB3D's binary ones keep Bezier for MB3D and a flag bit (1 << 20) for mb3d-rs; the Curves window's range rounded; labels in all seventeen languages, the READMEs mention it; tests: the weights, the keyframes hit exactly, both file formats |
| [`3d1bea6`](https://github.com/thr32423/mb3d-rs/commit/3d1bea682e090417b146ba929838f0bdcc869146) | Cloud shadows: the clouds shade the surfaces under them, also from outside the picture: in the painting each surface point's sunlight through the clouds (towards the first global light, 6 to 32 steps) dims its light, the ambient third staying; in Monte Carlo the sun's direct light at every hit; documented in the READMEs (all seventeen languages), TECHNICAL.md and the AI assistants' guide |
| [`b4db9d6`](https://github.com/thr32423/mb3d-rs/commit/b4db9d6880dcdab976ab15658c6f46c5cfe46f38) | Swarm pictures the same as one calculated in one piece: a swarm worker calculated the screen space occlusion (and the normals on the Z-buffer) on its tile alone, so the assembled picture's occlusion differed (in 64 % of the pixels of the test picture); the tiles of a picture the owner assembles are now calculated as a local tiled render's (shadows and DE occlusion per tile) and the passes over the whole picture run once on the assembled G-buffer (render::finish_tiles, shared with the local tiled render); with the normals on the Z-buffer the hard shadows now run after them on the whole picture, as in one piece (local tiles differed in 0.5 % of the pixels). Swarm, local tiles and one piece byte-identical with shadows, SSAO, DE occlusion and Z-buffer normals in five combinations; big renders (tiles as pictures of their own) unchanged |
| [`0bcb81c`](https://github.com/thr32423/mb3d-rs/commit/0bcb81c3aa94b63d421c9a98bd391a330f4241df) | README in all seventeen languages: animation curves, with a picture |
| [`3677f3d`](https://github.com/thr32423/mb3d-rs/commit/3677f3dd4e97aa6b6e0d77b1524d63daf8751786) | Animation curves (Mandelbulber's idea, not in MB3D): a Curves window from the Animation window's new Curves button shows a value (zoom, position, rotation as in the main window's panel, FOVy, Z start, the ground's position, the clouds' drift) over all frames as the animation interpolates it, the keyframes as squares on their frames (the quadratic Bezier passes beside them, as it renders); a square dragged up or down, or a value typed for the chosen keyframe, changes that keyframe (its picture and the times anew); the curve follows the Animation window's changes; labels in all seventeen languages, CJK subsets grown |
| [`d1fb818`](https://github.com/thr32423/mb3d-rs/commit/d1fb81857a99aa62bb9737917b863ea1a24ec963) | Monte Carlo: the surface material (Fresnel and iridescence on the hit's colours by the angle of view, glow as light emitted at the hit) and the clouds (composited along each view ray up to its first surface, in linear light; clouds::Clouds::ray shared with the painting, whose clouds stay byte-identical); the hints and the READMEs in all seventeen languages say so |
| [`da30fd2`](https://github.com/thr32423/mb3d-rs/commit/da30fd2c11eae681ab6e562bab98dda4c4283665) | README in all seventeen languages: volumetric clouds, surface materials and two fractals in one scene, with pictures |
| [`74078da`](https://github.com/thr32423/mb3d-rs/commit/74078da0fd1da0e40e29edb7a5dfc670e27e830c) | Two fractals in one scene (Mandelbulber's idea, not in MB3D): with a DE combination (not the mix), the second hybrid part as an object of its own, moved, turned and scaled (its point m (p - position) with the rotation divided by the size, its distance times the size), combined as the combination says (union, intersection, cut, smooth union); on the CPU, the graphics card (P2_PLACE .. words 417-430) and in Monte Carlo; Formula window, DE combination panel: Part 2 at, Size, Turned (labels in all seventeen languages); kept in the mb3d-rs block (part2_position, part2_rotation, part2_scale), in the AI assistants' guide. Card vs CPU: 0.008 % of the pixels differ; scenes without it: the same card output and speed as before today's shader changes |
| [`06f72a3`](https://github.com/thr32423/mb3d-rs/commit/06f72a375fdb9d83f7576ff3057091149ec11842) | Surface material (ideas from Mandelbulber, not in MB3D), on top of the palette: Fresnel (the specular colour, and with it the reflections, towards white where the surface turns from the view; Schlick's term with a third power), iridescence (a thin film's colours on the specular colour and half on the diffuse one, changing with the angle of view, with a shift) and glow (the surface's own light in its diffuse colour); the fractal only (the ground keeps its own), not in Monte Carlo; off: byte-identical pictures. Post processing window: a Material section (Surface material, Fresnel, Iridescence and its shift, Glow, Paint now; labels in all seventeen languages, CJK subsets grown); kept in the mb3d-rs block, blended between keyframes; scene keys material_fresnel, material_iridescence, material_glow; in the AI assistants' guide |
| [`a269f4a`](https://github.com/thr32423/mb3d-rs/commit/a269f4ac7d1b7318413040982afaef2b6bbd9507) | Volumetric clouds (Mandelbulber's idea, not in MB3D): a layer between two heights along the fractal's X, Y or Z axis, shaped by four octaves of 3D value noise (density, coverage, size, colour, an offset that keyframes animate: drifting clouds), lit by the first global light through the cloud (five steps towards it, forward scattering) and the ambient top and bottom colours; painted as a pass after the lighting and the reflections, before the depth of field: each pixel's ray steps through the layer up to its surface (8 to 96 steps, jittered per pixel), so the fractal hides the clouds behind it and the clouds in front veil it; in 8 and 16 bit and OpenEXR pictures, not in the previews, reflections and Monte Carlo. Post processing window: a Clouds section (Paint clouds, layer, density, cover, size, colour, Paint now; labels in all seventeen languages, CJK subsets grown); kept in the mb3d-rs block, blended between keyframes; scene keys clouds, cloud_density, cloud_coverage, cloud_size, cloud_color, cloud_offset; in the AI assistants' guide |
| [`2d64d90`](https://github.com/thr32423/mb3d-rs/commit/2d64d90f25f26439872dfe0948dc42f1b2815662) | Main window: the stereo pair of a 360 Panorama camera is the VR stereo panorama, both eyes one above the other (left on top), for Render stereo pair and Render & save stereo pair; their hints say so (label in all seventeen languages, CJK subsets grown) |
| [`42ae21e`](https://github.com/thr32423/mb3d-rs/commit/42ae21e570a9617f26930aa331fb961490fbc68c) | README in all seventeen languages: the fisheye camera and 360 degree stereo panoramas for VR, with pictures |
| [`1248c03`](https://github.com/thr32423/mb3d-rs/commit/1248c038e9df0e997916a077b306d259a6126b14) | Stereo panoramas: the eyes 6.5 cm apart with the middle at the stereo viewing distance (stereo_screen's second value, 2 m by default), not scaled to the picture's width, which spans 360 degrees in a panorama (the eyes were five times too far apart: 20 degrees of parallax at the middle, now 1-2) |
| [`82dee34`](https://github.com/thr32423/mb3d-rs/commit/82dee34184027a98865f5e611a7f0b6d4264c5b1) | Fisheye camera (not in MB3D): an equidistant fisheye, the field of view up to 360 degrees across the picture's height (beyond 180 degrees from the axis the view straight back), on the CPU, the graphics card and in Monte Carlo (also its direct background picture); main window, Camera page: Fisheye (labels in all seventeen languages, CJK subsets grown); the Navigator keeps a fisheye until another optic is chosen there; MB3D's parameters get the common optic, the mb3d-rs block the fisheye (scene key optic = 3 or fisheye). 360 degree stereo panoramas for VR: a panorama's stereo eyes are placed per ray, turned with its horizontal direction around the middle (omni-directional stereo; before, the whole panorama was shifted, right only straight ahead); --stereo-pair top-bottom (left eye above). Card vs CPU: 0.05 % (fisheye) and 0.03 % (stereo panorama) of the pixels differ, Monte Carlo the same brightness; scenes with neither: the same card output and speed |
| [`5b4588f`](https://github.com/thr32423/mb3d-rs/commit/5b4588f7a1437eb7b8c1944bee04801e30a1424c) | README in all seventeen languages: the ground plane and water, light from an HDR picture and OpenEXR pictures, with pictures; TECHNICAL.md: the ground keys, HDR backgrounds and the OpenEXR layers |
| [`c7ec579`](https://github.com/thr32423/mb3d-rs/commit/c7ec57993660781e96b4e7d56029e39668f350b3) | Ground plane in animations (position, colour, reflection and waves blended when every keyframe has one across the same axis) and in the AI assistants' scene guide; OpenEXR output also for tiled and swarm renders (the whole picture's floating point colours from the tiles' G-buffer; identical to a single render's); a background picture's name longer than MB3D's 23 characters kept in the mb3d-rs block (.hdr files often have long names), its not-found warning for the full name; animation files take the {mb3d-rs: ...} line of a keyframe's text parameters |
| [`a4638f6`](https://github.com/thr32423/mb3d-rs/commit/a4638f688086231822cc052f8620bbaeaad95cc5) | Ground plane on the graphics card: the shader adds the ground's distance after the DE (GROUND_OPT .. words 411-415 of the parameters), marks its surfaces in do_color (otrap 0xFFFF where the ground is within twice the DE stop), the start plane in the ground as a cut surface with the ground's normal (the Monte Carlo march's as McMarch::normals: towards the camera when it points away from the ray), and the ground's normal from its distance where the normals are on the smoothed iteration count; compiled only for scenes with a ground (HAS_GROUND), so others give the same G-buffer and picture checksums at the same speed (a mutable on_de had cost 14 %). Card vs CPU: 0.07-0.15 % of the pixels differ by more than 6 % (0.57 % for the scene without the ground), water waves 2.3 % (their sparkles in single precision); Monte Carlo the same mean brightness |
| [`f8cf80e`](https://github.com/thr32423/mb3d-rs/commit/f8cf80e09b9bb793fbba8657a8f279f9bef4f159) | Ground plane and water (Mandelbulber's idea, not in MB3D): a plane across the fractal's X, Y or Z axis at a position, calculated with the fractal as the smaller of the two distances, so it has shadows, ambient occlusion (DE and screen space), reflections and Monte Carlo light, and casts shadows on the fractal; its own colour and reflection amount (0..1: specular colour and mirror) instead of the palette (G-buffer pixels marked by otrap 0xFFFF, which the 15 bit orbit trap never reaches outside inside-and-outside rendering); waves of three crossing sines (height, length in fractal units; the distance divided by their steepest slope); the solid side is the one away from the camera, rays that start inside it show a cut surface as at MB3D's cutting planes. Main window, Cutting page: Ground plane, axis, position, colour, reflect, waves (labels in all seventeen languages, CJK subsets grown). Saved after MB3D's fixed-size parameters as an mb3d-rs block (MB3D-RS magic, length, scene keys) in .m3p and after the formulas at the end of .m3i, and as {mb3d-rs: ...} after the title of text parameters; scene keys ground, ground_color, ground_reflect, ground_waves. Not in the voxel and mesh exports; the graphics card leaves such scenes to the CPU for now. Scenes without a ground: byte-identical pictures and the same CPU speed (calc_de restructured: +1.5 % before, -0.3 % now) |
| [`61f9774`](https://github.com/thr32423/mb3d-rs/commit/61f9774c58d34d2269b788bdaae982b79cbed513) | HDR environment pictures (Radiance .hdr) as background pictures and maps: decoded without a library (flat and run length encoded, rows from the top or the bottom), the light above white kept (white = 255), linear (not squared in the square root lighting mode); the Lighting window offers *.hdr. Monte Carlo: an HDR background's sun (the pixels around the brightest one, within 15 degrees, brighter than 8x the mean and twice white) is taken out of the picture and lights the scene as a global light in a free light slot, with soft shadows of the sun's size when the soft shadow radius is not 0; its strength gives the light the rays used to find in it by chance (checked: same mean brightness, 80.0 vs 79.9), without their bright speckles. 8 bit background pictures unchanged (pixel-identical renders and Monte Carlo) |
| [`fccee83`](https://github.com/thr32423/mb3d-rs/commit/fccee83ba71e707c273501c32d0176535af81553) | OpenEXR pictures: the colours in floating point (linear, not clipped above white) with the layers A (opacity), Z (depth from the camera's start plane in fractal units, 1e10 on the background) and N.X/N.Y/N.Z (normals in camera space); written without a library (one part, scan lines, ZIP compression with the PNG writer's zlib, half floats, Z as 32 bit floats). Main window: OpenEXR (*.exr) in the save dialog of the picture, reduced by the viewing scale; mb3d render: an output name ending in .exr (with --aa: colours averaged, depth the nearest, normals the mean); the painting has an f32 sample type that keeps the light above white, 8 and 16 bit unchanged. Checked with the OpenEXR library: colours within 0.5/255 of the PNG; label in all seventeen languages |
| [`51a6598`](https://github.com/thr32423/mb3d-rs/commit/51a6598930f7f17541c9332cfc90755743b110f6) | MutaGen keeps a formula's integer options within the range their names state (Modes (0 to 3), Map channel (0..2), Mode 0-1, (0,1), (1 or 2), max 1000, <32, Integer 2+; 35 of the shipped formulas' integer options): MB3D let a variation move them anywhere (a mode of 0..3 to 7, a power of 2+ to 0); a value set outside the range by the user stays free, typing and the Navigator are unchanged |
| [`833eedf`](https://github.com/thr32423/mb3d-rs/commit/833eedfa915dfb546839da5f0546d2ec7760d31e) | A crash told at the next start: the panic hook writes the panic's message into the session's lock file, and the next start says "The last session crashed: <reason>. More in the Log window" instead of "did not end properly (it froze, was killed or crashed)"; the log line names the session and mb3d_crash.txt. The main window's message box starts empty (the welcome and the overwhale web site lines are gone, their labels removed from every language). snapform: the crash hook installed and a panic step |
| [`ea6e386`](https://github.com/thr32423/mb3d-rs/commit/ea6e38631b2c05e9dbce87a98bf0bb45a0484c77) | Navigator, experimental "Main window's lighting": switched off, the navigator goes back to its own lighting (it kept the main window's, e.g. a light added there); its own lighting is kept while the option is on, a preset, dynamic fog, the Light button or new main parameters replace it |
| [`785dde9`](https://github.com/thr32423/mb3d-rs/commit/785dde9534504053b4298dd00db2b8fdd8aafdae) | Navigator: the preview's height capped so the whole window (with the experimental strip) stays within the screen at large Navigator sizes (130 % on a 1080 screen pushed the bottom off the screen) |
| [`8352c4f`](https://github.com/thr32423/mb3d-rs/commit/8352c4fcd2c866c12163d5265952e36a02eb64ec) | Navigator, experimental options to match the main window (a new strip "Experimental: match the main window" at the bottom, each off by default and remembered in the ini): Main window's detail (the main scene's raystep, raystep limiter, binary search steps and DE stop instead of the navigator's faster ones; HiQual greyed), True brightness (the brightness slider not applied, greyed), Main window's lighting (the main lighting on every drawing, following the Lighting window's changes; a navigator light preset or dynamic fog turns it off); labels in all seventeen languages, the CJK font subsets grown by the new characters |
| [`b7307f7`](https://github.com/thr32423/mb3d-rs/commit/b7307f7f41769659a0d1e2dc404925c6c92a1ccf) | Event log on Windows: the stale session's program, process and start time are read through the handle that holds its lock (Windows locks are mandatory, a second handle could not read the file, the note said "? (process ?) started ? UTC"); fixes the Windows CI test a_session_left_locked_is_found_and_old_files_go |
| [`86dd4fe`](https://github.com/thr32423/mb3d-rs/commit/86dd4fefd53b20d89a87784a8b0dd96b679de525) | README: Updates for v0.2.87 |

All 101 releases, back to v0.1.0: [the whole list](docs/UPDATES.md).
<!-- updates:end -->

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

## Credits

Mandelbulb3D, which this program is a port of, was made by these people:

| Author | What they did |
|---|---|
| **Jesse** (fractalforums.com) | Wrote the original Mandelbulb3D (2010–2013, Delphi and 32-bit assembly): the distance estimation renderer, Navigator, formula and hybrid system, post processing (shadows, ambient occlusion, depth of field, reflections), Animation maker, Big renders, Monte Carlo renderer. Released it as freeware; his last release was 1.8.9 (April 2013) |
| **Andreas Maschke** (thargor6) | Maintainer since 2015: published Jesse's source code on GitHub, moved it to Delphi XE6, released 1.9.0 to 1.99.37 (2023). Added the JIT formula editor, MutaGen, BulbTracer and BulbTracer2 mesh export, the HeightMap generator, ZBuf16Bit, animated maps, the themed interface, point cloud export, Monte Carlo batch and network rendering. His code is under the LGPL 2.1 |
| **DarkBeam** (Luca GN) | By far the most formulas (credited in 210 of 464 `.m3f` files); the formula bundle of 1.8.2; the interim builds 1.8.9 x1 and x2 |
| **David Makin** | Helped Jesse implement the distance estimation; formulas (12 files) |
| **Tglad** (Tom Lowe) | Invented the Amazing Box (Mandelbox); formulas (13 files) |
| **Buddhi** | The fast distance estimation of the Amazing Box; formulas (6 files) |
| **Daniel White** | With Paul Nylander, the 2009 "triplex" Mandelbulb that MB3D's built-in bulb formula is based on; the fast integer power formulas Jesse used |
| **Paul Nylander** | With Daniel White, the Mandelbulb |
| **M. Benesi** | Formulas (33 files) |
| **Knighty** | Formulas (18 files), among them Knots2, koch_surf, the JCube3 approximation, and JosKn-KleinIFS with Jos Leys |
| **Aexion** | Formulas (12 files), among them the built-in "Aexion C" |
| **Kali** | Formulas (8 files), among them Amazing Surf |
| **Bethchen** | Formulas (8 files) |
| **msltoe** | The Riemann formula |
| **Fracmonk** | The CommQuat formula |
| **Trafassel** | The IdesFormula |
| **Syntopia** | Inspiration and testing; the Sierpinski4 and Menger4 formulas |
| **Jos Leys** | JosKn-KleinIFS, with Knighty |
| **Mark Townsend, Inigo Quilez, T. Gintz, Kerry Mitchell, Pseudogenius, dwsel** | Formulas |
| **Alef** (Edgar Malinovsky) | The JIT formula pack (`EM_JIT_M3Formulas`) |
| **Schmiegel** | Test parameters for the 1.99 releases |
| **Matteo** | Suggested and supported Monte Carlo batch rendering |
| **Zeyu-Li** | Documentation on GitHub (2020) |
| **NASA, GIMP** | Source material for the maps |

* **Contributors to this program**: Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)): the stereo pair
  preview ([#12](https://github.com/thr32423/mb3d-rs/pull/12)), saving stereo pairs, the
  Navigator's axes and the grouped taskbar
  ([#13](https://github.com/thr32423/mb3d-rs/pull/13)), the stereo controls
  ([#14](https://github.com/thr32423/mb3d-rs/pull/14)), sorting in the file dialogs
  ([#17](https://github.com/thr32423/mb3d-rs/pull/17)) and their sortable columns
  ([#19](https://github.com/thr32423/mb3d-rs/pull/19)), and the program icon in the Windows
  executable.
* **Bug reports and suggestions**:
  [kronpano](https://github.com/kronpano)
  ([#1](https://github.com/thr32423/mb3d-rs/issues/1),
  [#2](https://github.com/thr32423/mb3d-rs/issues/2),
  [#3](https://github.com/thr32423/mb3d-rs/issues/3),
  [#4](https://github.com/thr32423/mb3d-rs/issues/4),
  [#8](https://github.com/thr32423/mb3d-rs/issues/8)),
  [lukastrips](https://github.com/lukastrips)
  ([#6](https://github.com/thr32423/mb3d-rs/issues/6),
  [#20](https://github.com/thr32423/mb3d-rs/issues/20)),
  [Rodrigo-Spacecraft](https://github.com/Rodrigo-Spacecraft)
  ([#9](https://github.com/thr32423/mb3d-rs/issues/9)), Michael Pettovello,
  Geoffrey N. Barnes.
* **The PseudoKleinian formula** is knighty's (Fragmentarium, 2011,
  "Theli-at's Pseudo Kleinian" after Theli-at's hybrid discovery of the
  look on fractalforums.com); this program's version is written in Rust
  and WGSL from its algorithm.
* **The SphereTree formula** is by Tom Lowe (TGlad): recursive Eisenstein
  Ford sphere packings on a Riemann sphere projection, published on
  [fractalforums.org](https://fractalforums.org/fractal-mathematics-and-new-theories/28/new-sphere-tree/3557)
  in June 2020 with a Fragmentarium file
  ([TGlad/FragmentariumExamples](https://github.com/TGlad/FragmentariumExamples)).
  This program's version is written in Rust and WGSL from that
  algorithm; the colouring per branch was added.
* **The DomeTree formula** is by Tom Lowe (TGlad), `DomeTree.frag` in
  [TGlad/FragmentariumExamples](https://github.com/TGlad/FragmentariumExamples);
  this program's version is written in Rust and WGSL from that
  algorithm, with a colour per dome added.
* **The fonts of the languages** are Noto Sans CJK (subsets) and Noto
  Sans Devanagari by the Noto Project Authors (SIL Open Font License,
  `Languages/OFL-*.txt`); Devanagari is shaped with
  [rustybuzz](https://github.com/harfbuzz/rustybuzz) (MIT), the Rust port
  of HarfBuzz.
* **The icon** is Mandelbulb3D's own icon (`m3d128.ico` from
  [thargor6/mb3d](https://github.com/thargor6/mb3d), LGPL 2.1), with its
  background made transparent and the letters "RS" (for the Rust port)
  added; the letters are set in Liberation Sans Bold (SIL Open Font
  License).
* **The noise filter** is the DCT image denoising algorithm of Guoshen Yu
  and Guillermo Sapiro: *DCT image denoising: a simple and effective image
  denoising algorithm*, Image Processing On Line 1 (2011),
  [doi:10.5201/ipol.2011.ys-dct](https://doi.org/10.5201/ipol.2011.ys-dct).
  It is the algorithm of **GIMP**'s *Denoise DCT* filter (from GIMP's
  image library [GEGL](https://gegl.org)); GIMP's filters were compared
  on Monte Carlo pictures, and this one came out best. This program has
  its own implementation of the algorithm, written from the paper; it
  contains no code of GIMP or GEGL (GEGL is under the LGPL 3, this
  program under the LGPL 2.1).
* **The neural noise filter** is
  [Intel Open Image Denoise](https://www.openimagedenoise.org) by Intel
  (Apache License 2.0), shipped as Intel releases it, with the
  [oneTBB](https://github.com/uxlfoundation/oneTBB) library it uses
  (Apache License 2.0); their licences are in `oidn/doc`. The program
  loads it while running.
* **The formulas are compiled at run time** with
  [Cranelift](https://cranelift.dev) of the Bytecode Alliance
  (Apache License 2.0 with LLVM exception), on Linux and Windows.
