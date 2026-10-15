# Getting started with mb3d-rs

mb3d-rs is a Rust port of [Mandelbulb3D](https://github.com/thargor6/mb3d)
(MB3D). It has two programs: **Mandelbulb3D**, the desktop program with
MB3D's windows, and **mb3d**, the command line renderer. They need no
installation.

## 1. Get the program

On Linux and macOS, this line runs the script [download.sh](download.sh),
which downloads the latest release into `~/mb3d-rs` (the script is also in
the program's folder; run it again to update):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Then start it with `~/mb3d-rs/Mandelbulb3D`.

On Windows, download
[mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip)
from the [releases](../../releases) page (older versions are there too),
unpack it and start `Mandelbulb3D.exe`.

## 2. Get MB3D's formulas and maps

The release archives are laid out like a Mandelbulb3D installation, with
the whole original Mandelbulb3D repository (from
[thargor6/mb3d](https://github.com/thargor6/mb3d), see `MB3D-SOURCE.txt`):

```
Mandelbulb3D(.exe)    the program with MB3D's windows
mb3d(.exe)            the command line renderer
M3Formulas/           MB3D's formula files
M3Maps/               maps for light maps, colour maps and backgrounds
M3Parameter/          80 example parameter files
EM_JIT_M3Formulas/    more JIT formulas
History/              parameters of every "Calculate 3D" (as in MB3D)
Meshes/               meshes from BTracer2
BigRenders/           big renders
ShaderCache/          the graphics driver's compiled shaders (Linux; made on the first calculation)
MB3D-sources/         MB3D's Delphi sources and project files
README-MB3D.md, README_1.*.txt, CHANGELOG.txt, License.txt   MB3D's documents
examples/             scenes in this port's .m3s format
```

Nothing else is needed: the program finds the formulas and maps next to
itself.

### Where mb3d looks for them

Formulas or maps kept somewhere else (for example in an existing MB3D
installation) can be added with options; `mb3d` searches, in this order:

| what | searched in |
|---|---|
| formulas | `--formulas DIR` (may be repeated), `$MB3D_FORMULAS` (a path list), `./M3Formulas`, `M3Formulas` next to the program, next to the parameter file's folder, `~/mb3d-rs/M3Formulas` |
| maps | `--maps DIR`, `$MB3D_MAPS`, `./M3Maps`, `M3Maps` next to the program, `M3Maps` next to every formula folder |

In Mandelbulb3D, *Prefs ▸ Ini dirs* sets the folders, as in MB3D; they are
kept in `Mandelbulb3D.ini` next to the program.

## 3. The Mandelbulb3D program

Start `Mandelbulb3D` in the unpacked folder (double-click it;
`Mandelbulb3D.exe` on Windows), or from a terminal in that folder,
optionally with a parameter file:

```sh
cd ~/mb3d-rs
./Mandelbulb3D
./Mandelbulb3D "M3Parameter/6 AM - Torii temple.m3p"
```

The windows are MB3D's own (made from its form files), so MB3D's
tutorials apply. A short tour:

* **Main window:** the buttons at the top open parameters (`.m3p`,
  `.m3i`, text from the clipboard), save them and save pictures, and open
  the other windows: *Formulas*, *Lighting*, *Postprocess*, *Navigator*,
  *Animation*, *BTracer2*, *MutaGen*, *ZBuf16Bit*, *HMapGen* and the tools
  (batch, voxel export, big renders, Monte Carlo). On the right are the
  position, rotation, image size, the quality presets and *Calculate 3D*;
  below the image the mouse modes (2D zoom, X/Y, Z, get position) and the
  rotation buttons.
* **Navigator:** its own window and preview; walk with the keys (W/S,
  A/D, E/C, arrows) or the mouse (hold the right button to look around),
  then *View to main*.
* **Lighting:** changes repaint the calculated image right away; the
  presets and the palette are at the top.
* **Prefs ▸ Visual themes** switches between MB3D's Glossy look and the
  Windows look.

Every change marks the image as old; *Calculate 3D* calculates it at full
size. After that, changes in the Lighting window only repaint it and changes
of shadows or ambient occlusion only redo those, as in MB3D. *Save pic*
saves it reduced by the viewing scale (1:2 and 1:3 are anti-aliased).

## 4. The command line

Open a terminal in the program's folder (`cd ~/mb3d-rs` on Linux). On
Windows, open a command prompt in the unpacked folder and type `mb3d`
(`.\mb3d` in PowerShell) instead of `./mb3d`.

```sh
./mb3d "M3Parameter/6 AM - Torii temple.m3p" -o torii.png            # full size
./mb3d "M3Parameter/6 AM - Torii temple.m3p" --scale 0.25 -o small.png
./mb3d examples/mandelbulb.m3s -s dof=sorted -s dof_focus=0.6 -o dof.png
./mb3d batch M3Parameter --scale 0.2 -o previews                       # a folder
./mb3d animate flight.m3a                                              # MB3D animation
./mb3d montecarlo scene.m3p --rays 64 -o mc.png
./mb3d mesh scene.m3p -o object.stl
./mb3d --help                     # all options; also ./mb3d animate --help, ...
```

Parameters copied from a forum post (`Mandelbulb3Dv18{...}`) can be saved
into a text file and opened like a `.m3p`.

## 5. When something is missing

* *formula 'xyz' not found*: the formula folder is not found; see step 2,
  or add `--formulas path/to/M3Formulas`.
* *map N not found*: the same for `M3Maps` (`--maps`).
* A file renders, but looks different from MB3D: please open an issue with
  the parameter file (README ▸ "Good to know" and docs/TECHNICAL.md ▸ "Known differences" list what is not ported).

## 6. Building from source

Only needed if you want to change the program. With
[Rust](https://rustup.rs) (stable):

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release          # target/release/Mandelbulb3D and target/release/mb3d
```

The programs built this way have no formulas or maps next to them: get
MB3D's folders with `git clone https://github.com/thargor6/mb3d` (or use
an existing MB3D installation) and copy `M3Formulas` and `M3Maps` next to
the programs, or pass `--formulas` and `--maps`.
