# How mb3d-rs calculates your image

The original Mandelbulb3D calculates everything on the processor (CPU).
mb3d-rs can also use the **graphics card**, which is usually several times
faster, and it decides by itself how to share the work. This page explains
what happens, what the little lamp in the title bar tells you, and the
one setting you may want to change. The other things that are new
compared with the original program are listed in the
[README](../README.md#new-compared-with-mandelbulb3d).

![The main window after a calculation on the graphics card](img/main_window.jpg)

## Two calculators

| | CPU | Graphics card |
|---|---|---|
| What it runs | MB3D's calculation, ported line by line | the same calculation, translated into a *shader* (a program for the card) |
| Speed | close to MB3D's own speed | in the 80 example files a median of 5 times faster (up to 87 times) |
| Scenes | all of them | 73 of the 80 examples (not: volumetric light, 2D slices, the few formulas that cannot be translated, formulas whose numbers grow beyond single precision - those go to the CPU by themselves) |
| Result | MB3D's result | the same image within rounding (see the [comparison of all 80 example files](../README.md#cpu-and-graphics-card-compared)) |
| Start-up | none | the **first** time a formula combination is used, its shader must be *compiled*: 10–20 s. The graphics driver keeps it, so the next time it is ready at once |

The compile time is the reason why the graphics card is not always the
faster choice: a small preview that the CPU finishes in two seconds would
wait 15 seconds for a shader. mb3d-rs avoids that waiting (see *Auto*
below).

## The "Calculate on" setting

On the **Internal** page of the main window (where the thread count is):

![The Internal page with "Calculate on"](img/internal_page.png)

| Setting | What happens |
|---|---|
| **Auto** (default) | Uses the graphics card. If its shader is not ready yet, the CPU starts at once and the card joins when it can - nothing waits. |
| **CPU** | Never uses the graphics card - exactly like the original MB3D. |
| **Graphics card** | Waits for the shader if needed, then calculates on the card. |

The choice is saved in `Mandelbulb3D.ini`. On the command line `mb3d
--cpu` calculates on the CPU, and the environment variable `MB3D_GPU=0`
switches the card off for both programs.

## What *Auto* does

```mermaid
flowchart TD
    A[Calculate 3D] --> B{Can the card do this scene?}
    B -- no --> CPU[CPU calculates everything]
    B -- yes --> C{Card's shaders compiled?}
    C -- "yes (or within 0.2 s)" --> GPU[Card calculates the image,<br/>shadows, ambient occlusion<br/>and reflections]
    C -- no --> D[CPU starts at once<br/>shaders compile in the background]
    D --> E{Shader ready before<br/>the CPU is done?}
    E -- no --> F[CPU finishes alone<br/>small images]
    E -- yes --> G[Card joins, the two<br/>share the rows]
```

When the two share an image, the CPU threads take rows from the bottom
and the card takes rows from the top, until they meet:

![Card and CPU share the rows of an image](img/rows-split.svg)

The message box of the main window says when this happens (*"First
calculation of this formula combination: the graphics card's shaders are
being compiled ..."*), so a slower first run is not a surprise.

All shaders a calculation needs (the march, shadows, ambient occlusion,
reflections) compile at the same time in the background, so the waiting
is paid once and not once per shader. In the shared case the shadows and
ambient occlusion are calculated on the CPU, like after a CPU calculation;
the next calculation of the scene runs entirely on the card.

## The lamp (LED) in the title bar

Right of the window title a small lamp shows where the last calculation
ran. It blinks while a calculation is running.

| Colour | Meaning |
|---|---|
| grey | nothing calculated yet |
| **green** | on the graphics card |
| **orange** | mixed: the image on the card, some steps on the CPU (for example normals on the z-buffer, or shadows after a shared start) |
| **red** | on the CPU (setting *CPU*, or a scene the card cannot do) |

**Hold the mouse over the lamp** to see the details: the card's name, the
steps it calculated, the reason for a CPU calculation, and for Monte Carlo
renders how the work was shared.

![Calculated on the graphics card: green lamp and its hint](img/led_card.png)

![Calculate on: CPU - red lamp and its hint](img/led_cpu.png)

## Thread priority

**Thread Priority** on the Internal page (Idle, Lowest, *Lower* by
default, Normal) works as in MB3D: the calculation threads run below the
windows, so the program stays responsive and the lamp blinks steadily
during long calculations. It costs no speed while nothing else needs the
processor; other programs that do get it first (Idle the most). On Linux
and Windows; macOS keeps normal priority.

## Monte Carlo renders

The Monte Carlo renderer follows many random light rays per pixel. The
rays are followed (*marched*) on the graphics card, while the CPU works
out the light, the colours and the noise of every pixel. To keep both
busy, every CPU thread splits its pixels into two halves that take turns:

![Monte Carlo: card and CPU take turns on two halves](img/mc-overlap.svg)

The CPU also takes a part of every batch of rays from its end. How big
that part is follows from how fast the card and the CPU were on the last
batch, so a slow formula (where the card is not much faster than the CPU)
gets more help from the CPU. The lamp's hint shows the result, for
example *"93 % of the marches on the card, 7 % on the CPU"*. With *Auto*
the first passes run on the CPU while the shader compiles.

Speed on a GTX 1650 with 8 CPU threads:

| Scene | CPU only | Card + CPU |
|---|---|---|
| Black Mandelbulb | 7.3 s | 1.9 s |
| ThePearl (dIFS, reflections, transparency) | 30.8 s | 10.5 s |
| toricaleggs (reflections, transparency) | 33 s | 11.8 s |
| Mengerplus | 12.7 s | 6.8 s |
| Mandelwerk seashell (a slow formula) | 87 s | 47.5 s |
