# Updates

Every release of mb3d-rs, newest first, with the time it was published
(UTC) and the changes (commits) it brought. Each commit links to its full
description and code; the downloads of a release are on its page. Back to
the [README](../README.md).

## [v0.2.97](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.97) · 4 Nov 2026, 09:07 UTC

| Commit | What it does |
|---|---|
| [`90867f3`](https://github.com/thr32423/mb3d-rs/commit/90867f3d1e4939daaccf8cd5b36de63780bb902f) | Version number and release notes of this release |
| [`0e0e9cd`](https://github.com/thr32423/mb3d-rs/commit/0e0e9cd21e1b6f136790c65e72d6ac8ea02bd825) | Big renders: a project opened during a calculation keeps its render, save, import and settings controls greyed until the calculation ends, then enabled (main_form::enable_after_calc; MB3D's DisableButtons) |
| [`8895e01`](https://github.com/thr32423/mb3d-rs/commit/8895e0198b829d4b364fe918d62cdd8c1c10c7c5) | Fixes from checking the MB3D changes (review and GUI tests) |
| [`3a35241`](https://github.com/thr32423/mb3d-rs/commit/3a35241f31535b07c2cfe8c8a29a27fb1309de74) | JIT Amazing Box and Bulbox: Min R clamped before squaring as the interpreter and the card (MB3D's Sqr(Max(1e-40, MinR))); clifcheck --builtins checks negative Min R (12 of 490 variants differed, now 0) |
| [`7bf6d6b`](https://github.com/thr32423/mb3d-rs/commit/7bf6d6bcd4654f6d4b40e8acb9e52602d6394e7d) | The new labels in all 17 languages (MB3D's built-in formula descriptions, pausing on the next frame, not enough memory, tile file deletion, PNG 8 bit, the welcome line); Japanese, Korean and Chinese font subsets with their new characters |
| [`c3d0218`](https://github.com/thr32423/mb3d-rs/commit/c3d021821c4259e103600c539d04138de51b721a) | Animation and Lighting dialogs leave the folders of the settings alone (MB3D changes IniDirs only in the folders window, FileHandling.pas:575-637): the animation's output folder, GIF and .m3a dialogs and the background picture dialog no longer write ini.dirs; the picture's folder still joins the map search |
| [`a0da81e`](https://github.com/thr32423/mb3d-rs/commit/a0da81e939399fc9c738dc7d3c9af76745aadbbe) | main_form tests: the whole-program check of the m3i picture and the refused open taken out of the library tests (building the program there sets process-wide settings that made the card and swarm tests fail or hang); checked once without windows by a throwaway example instead |
| [`2bf2b9c`](https://github.com/thr32423/mb3d-rs/commit/2bf2b9c05b3532917abb4d0d1f6edf86a5e57785) | Main window as in MB3D |
| [`29d8a7e`](https://github.com/thr32423/mb3d-rs/commit/29d8a7e36dcf991c84648cbd855254eda1468469) | vcl: Ui::drag_resize, the window manager resizes a window from its bottom right corner while the button is held (for size grips such as MB3D's Image2, Mand.pas:3003-3032) |
| [`67af14c`](https://github.com/thr32423/mb3d-rs/commit/67af14cb088b2f3e7a58c24478ace397d41d4300) | m3i: the finished picture of "Save image in m3i" in MB3D's layout (Mand.pas:2866 SaveM3I: fullSizeImage after the formula addon, 4 bytes a pixel blue, green, red, 0, rows from the top): write_m3i_image_to writes it, image_from_m3i reads it (FileHandling.pas:2309, only when its size matches); this program's settings block follows the picture and is found behind it. Test: m3i_image_round_trip (also an MB3D file without the settings block) |
| [`112d077`](https://github.com/thr32423/mb3d-rs/commit/112d077cb3eb283f16f1f348b3715572483f0faa) | File dialogs: saving onto an existing file asks whether to replace it (MB3D's ofOverwritePrompt on every save dialog, Mand.dfm); No keeps the save dialog open. Test: save_asks_before_replacing |
| [`c180bec`](https://github.com/thr32423/mb3d-rs/commit/c180bec0e42969c3791d0c9283f7b4c0634283a4) | Monte Carlo window as MB3D's: "Send parameters to main" prepares the header for the main renderer (MonteCarloForm.pas Button4Click: HS of every light that is on with the cosine diffuse function, soft with one light, automatic DEAO 253 without dithering, iOptions 2 shl 6 + 1, no diffuse shadowing, ambient shadow 53 and second reflection 27, second mode off, small background picture as ambient, no tiling or SSAO border) and keeps the main picture with "Press 'Calculate 3D' to render" (ParasChanged, Mand.pas). Opening an .m3c with warnings makes Label33 blink red 8 times (Timer3Timer). Test of the sent parameters |
| [`de84767`](https://github.com/thr32423/mb3d-rs/commit/de84767b11b2c89f9fd08cd5a882abf759d5b4e8) | Old .m3l files (smaller than TLightingParas9, 408 bytes) are converted as MB3D's GetLightParaFile does (FileHandling.pas: read as TLightingParas8, ConvertFromOldLightParas with the file size, UpdateLightVersion8, ConvertLight8to9) instead of rejected. Test with synthetic TLightingParas (92 bytes) and TLightingParas8 records |
| [`b944c66`](https://github.com/thr32423/mb3d-rs/commit/b944c661a5389ae1ee460c83ce4e2b5c803e1dd1) | Lighting window as MB3D's: Open (.m3l) leaves the lights folder of the settings alone and clears the quick load selection (LightAdjust.pas Button1Click, ComboBox3.ItemIndex := -1), its dialog remembers the last folder for the session. The light placement "image" button (ButtonGetPosClick) unticks the post processing's selection modes (CheckBox21, CheckBox25), releases the main window's navigation buttons (MButtonsUp) and sets the relative sliders TrackBar15..17 to 0 (SetPosLightTo0). Undo keeps 64 steps (Undo.pas UndoLights) and a loaded lighting is a step (FileHandling.pas StoreUndoLight after loading). The wheel outside the window goes to the main window (FormMouseWheel, OverForm). A lighting repaint drops the hand-applied DoF and paints lights whose shadows were reset as without |
| [`9913c68`](https://github.com/thr32423/mb3d-rs/commit/9913c688ec97032dd4c82e55a8798f08d12e21b9) | Post processing as MB3D's: "Reset now" only repaints (TriggerRepaint, PostProcessForm.pas Button16Click): the reflections and the depth of field are undone, the hard and ambient shadows and normals calculated by hand stay in the G-buffer. DoF "Calculate now" accumulates (doDOF on fullSizeImage, Button1Click): a new engine job applies it once more to the picture shown, until the next repaint or calculation. A step's "Calculate now" (hard shadows, ambient shadows, normals) runs only that step on the kept G-buffer (the post signature in parts: what all steps depend on and each step's settings) and repaints without reflections and DoF, as MB3D's repaint; reflections "Calculate now" paints them (CalcSRT) without DoF. The per-light HS reset (Button4Click) clears that light's bits in the G-buffer (soft shadow: all lit again) and paints it as not calculated (bHScalculated), the light's box stays ticked. HS "Calculate now" leaves out light-map lights and says so (Button3Click). The mouse wheel goes to the main window (FormMouseWheel: brush size, zoom over the image). Tests: steps on the kept result, the light reset bits, the DoF count reset by a repaint |
| [`d26ab07`](https://github.com/thr32423/mb3d-rs/commit/d26ab077d42e2b543cd2ae950973ecc4bedb81ed) | Big renders: changing a tile setting makes the project "(new)", saved before a tile renders (Tiling.pas SpinEdit1Change, SpinEdit2Change: NewProject); "Delete this tiles files" asks for each file ("You want to delete: ...", Deletethistilesfiles1Click). Batch: Del and the menu delete all highlighted rows (BatchForm.pas ListView1KeyDown, Deleteselectedfilesfromlist1Click). Folders window: another lights folder refreshes the Lighting window's quick load list (prefs/IniDirsForm.pas Button3Click: UpdateQuickLoadCB; light_form's update_quick_load made pub(crate) for it) |
| [`7b12354`](https://github.com/thr32423/mb3d-rs/commit/7b12354cab478189105f2ddd81dfd49a0e6a12b0) | Animation as MB3D's: the preview shows each frame when it is done while Frames/sec is on, and plays the frames done after a cancel (AniPreviewWindow.pas IncSubFrame, StartRendering); scrubbing unticks the playing (ScrollBar1Scroll); the speed kept in the ini as AniPrevFPS (FormShow, FormHide: IniVal[8]); Close cancels a running preview first and closes a finished one (Button1Click). The preview's render time from the keyframe pictures' times (KFtime, now measured) with MB3D's formula, the memory figure of the keyframes Edit9..Edit10 (and the loop) only, red or orange when the memory is short, and the preview refused with "Not enough memory available" (calcTimeForPreviewRender, Button4Click); Edit9 maroon when it is no number. Pause shows "Pausing on next frame..." (button off) until the rendering waits, then "Paused, press to continue" (Animation.pas Button2Click, Timer2Timer). Test: the preview estimate |
| [`3015868`](https://github.com/thr32423/mb3d-rs/commit/301586808b2cfccff6f9514991d8c870c5153a85) | Formula window as MB3D's: the lists sorted by vote then name, built-in and custom formulas mixed (FormulaGUI.dfm Sorted = True, items "status name", FormulaGUI.pas AddFormulaName), no favourite colours (ListBoxEx9DrawItem's colour code is commented out); a list opens after the mouse rests on its button or moves on sideways (Timer5: 20 then 100 ms, less than 1 pixel up or down per 16 ms, FormulaGUI.pas SpeedButtonEx1MouseEnter, Timer5Timer); the built-in formulas' descriptions behind "?" (CFdescriptionIntern, DescrOfFName, CustomFormulas.pas), the button enabled only with a description; the 4D rotation only when the whole hybrid is 4D (Is4Dtype, CheckDEoption, HeaderTrafos.pas); shifting the formulas moves the second part's repeat too and checks the hybrid options (Shiftallformulasonetotheright1Click, Shifttotheleft1Click, CheckHybridOptions); the lists leave out TrafasselQuat, _FlipXY/XZ/YZ, ABoxModKaliFixed when the newer file is there, SqKoch, DJD-Quat and _IFS-Tess (LoadFormulaNames); the hidden formulas as FavouriteList.txt spells them (SpeedButton4Click), the first line of a name counts (GetFavouriteStatus). JIT editor: Save and exit of an edited formula selects it again, so the slot takes the saved code (EditJITFormulaItmClick). Tests: list order, exclusions, Is4Dtype, CheckHybridOptions, the hover test. Labels for the descriptions and for the animation's pause and memory messages |
| [`42a4125`](https://github.com/thr32423/mb3d-rs/commit/42a41255f9790d0a500f93bb532644c79af8c5ec) | MB3D's rules in five places, ported from MB3D's source; shader checks without a graphics card; Stop ends painting at once |
| [`29e63e2`](https://github.com/thr32423/mb3d-rs/commit/29e63e21651f11123635a251132125b7bcf4641a) | tools/fork_watch.py: watches the projects built on mb3d-rs (Mandelbulb3D SE with our engine in vendor/mb3d-rs, and the GitHub forks): which commit of ours a copy was taken from, its kept, changed and new files, the functions of ours its app calls, a fork's own commits; compares with the last run, writes ~/.local/share/mb3d-fork-watch/latest.md, notifies when something is new; --install adds a daily systemd user timer. Fetches and reads only, never builds or runs their code |

## [v0.2.96](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.96) · 4 Nov 2026, 04:25 UTC

| Commit | What it does |
|---|---|
| [`5d22064`](https://github.com/thr32423/mb3d-rs/commit/5d22064678f95c1eea85389c605a0d5767fea52b) | Community light presets: the 608 .m3l of the Mandelbulb3D 1.99.37 package Machina Infinitum offers as a free download (M3Lights-community, copied into M3Lights by the release workflow and the deb/rpm scripts, with CREDITS.md as CREDITS-community.md: no maker or terms named in the files, credit to the community and Machina Infinitum, makers can ask to be named or removed). M3Lights is the default lights folder (Lighting window's quick load list and Open), also for an ini that kept the old default, the program folder, unless presets were saved there; installed, the shipped folder. Tests: every preset is a light record; the old lights folder moves to M3Lights. README and its 16 translations, TECHNICAL.md, the 0.2.96 release notes |
| [`619189d`](https://github.com/thr32423/mb3d-rs/commit/619189de39d5b7ddc65f4c22de4d0a65deb8cfb4) | Version number and release notes of this release |
| [`d707630`](https://github.com/thr32423/mb3d-rs/commit/d707630e465ef212af87560448419ebfd5aa06c5) | The example scenes gone: hybrid_box_bulb.m3s (unused) deleted, mandelbulb, quaternion_julia and interpolation_boxes moved to tests/data (the card tests compare their pictures with the CPU's); TECHNICAL.md's commands use tests/data/mandelbulb.m3s, its scene file paragraph points to mb3d --save-scene, the packages' folder list without examples |
| [`7151462`](https://github.com/thr32423/mb3d-rs/commit/7151462cc66eb447fd31647f64c9dc49722f8e66) | build_deb.sh, build_rpm.sh: no examples folder to copy (releases no longer carry it) |
| [`4177c15`](https://github.com/thr32423/mb3d-rs/commit/4177c15f8b592506e01e7779c6049ec49f241663) | Community formulas and maps: 129 formulas and 154 maps that the original MB3D repository lacks, from the Mandelbulb3D 1.99.37 package Machina Infinitum offers as a free download (M3Formulas-community, M3Maps-community, copied into M3Formulas and M3Maps by the release workflow). Each formula's author as its file names them in M3Formulas-community/CREDITS.md (Edgar Malinovsky 40, gnj 18, DarkBeam/Luca GN, bugman, Aexion, Buddhi, pupukuusikko, Jos Leys, Knighty, Nrx, Eiffie, Quasihedron and others; teapot.m3f CC BY-SA 3.0); the maps under DarkBeam's terms of use (free use, not sold unchanged), Kronpano's 3046-3048 credited. Left out: three JIT files cut off in the package (end written en), JITtest, the package's older copies of six formulas (ours are the repository's newer ones) and its 608 light presets (no author or terms). Every formula checked: 125 calculate alone or with the Mandelbulb, the dIFS transforms with a dIFS shape; test: all load, the JIT ones compile. Credits in the README and its 16 translations, TECHNICAL.md |

## [v0.2.95](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.95) · 3 Nov 2026, 19:24 UTC

| Commit | What it does |
|---|---|
| [`d247eab`](https://github.com/thr32423/mb3d-rs/commit/d247eab643138e73674cc9775d2fcc46180d11d8) | Version number and release notes of this release |
| [`61f6fb2`](https://github.com/thr32423/mb3d-rs/commit/61f6fb21e33874ed8a3d9d6dcb4698d535e971da) | docs/EXPERIMENTAL.md: all five together and the pairs measured again after the cone march's fix (tools/bench.py, 78 scenes, Auto, 0.4 MP): whole renders 173 -> 150 s (-13 %, was -11 %), 75 of 78 faster, the march -21 %; cone march + step relax -8.9 % median (each alone -5.1 and -5.8 %), cone march + no step limit -6.0 %; pixels changed median 3.1 % (the random first step itself 1.4 %) |
| [`4f5a7b7`](https://github.com/thr32423/mb3d-rs/commit/4f5a7b776fd40404641a31639848be9f0d7c68f7) | Releases without the examples folder: its four .m3s scenes are of no use beside the 80 parameter files (the folder stays in the source with the development programs); GETTING_STARTED's depth of field example uses the built-in Mandelbulb, mb3d --help says --save-scene writes a scene file |
| [`63890cb`](https://github.com/thr32423/mb3d-rs/commit/63890cbe061c49facf3d71134f6216eb6200ffd8) | Shader compiles wait while memory is short: a compile takes up to 2 GB in the driver, and a batch of scenes new to the card piled them up (30 example files at 0.1: 11.4 GB). A background compile now starts only when the free memory has room for it besides those running (the first always); in a batch, while compiles wait, the reflections of a file whose shaders are not ready go to the CPU instead of waiting (a single picture still waits: big ones are far quicker on the card). Pretending 3 GB free (MB3D_FREE_MEMORY_MB, new): 1.3 GB peak and 125 s, against 11.4 GB and 196 s; with plenty of memory nothing changes. A batch (mb3d batch and the batch window) says first how many formula combinations are new to the card, from compiled_shaders.txt in the data folder (the pipelines each card and driver compiled); label in 17 languages; tests |
| [`f43e5ea`](https://github.com/thr32423/mb3d-rs/commit/f43e5ea36da5dc5bd141575ad39b7d3128144e2e) | 360 degree panorama: the vignette and the colour fringes are left out (its edges meet behind the viewer: Storm's vignette 0.5 darkened the view behind to 48 of the middle's 78, a dark band and dark poles in a 360 viewer); Scene::finish_for_camera, used by the render, Monte Carlo, the command line and the neural filter; docs/CLOUDS_WATER.md: the three scenes as panoramas (Camera tab, 2:1, depth of field off, camera level) with Golden hour as one; test |
| [`7a851fc`](https://github.com/thr32423/mb3d-rs/commit/7a851fc4a4bce43452b4a2fc4cd7c35809378eb4) | Graphics card: the compiled pipelines kept are the 24 used last, no longer every one since the start: each formula combination brings its own (about 6 MB of the driver's each), so a batch grew by 13 MB a file without end (mb3d batch, 60 example files: 353 -> 741 MB and flattening, was a steady 13 MB a file); a scene needs 4 or 5, a dropped one comes back out of the driver's cache; test |
| [`120a737`](https://github.com/thr32423/mb3d-rs/commit/120a737e1661aa0a3760873a6e2ad070fb7508a0) | From Clipboard: text parameters whose lines were cut in copying (a narrow window or a chat cut 12 characters off every other line of a pasted Storm scene) are refused with a message, no longer read as the default formula with nonsense values (a 1x12 tiled render that seemed stuck); MB3D writes every line but the last as wide as the first. The command line says which line. Label in all 17 languages; tools/labels.py sync also put the two cone march labels in place; test |
| [`2ce5616`](https://github.com/thr32423/mb3d-rs/commit/2ce561613c8734403e0f3573ae3a5daff32a82c3) | cone test: the formulas _Rotate and Amazing Surf from the MB3D repository (MB3D_REPO, as CI's MB3D job sets it) or the usual formula folders, otherwise the test is skipped; on CI's plain jobs it failed since v0.2.93 (formula '_Rotate' not found) |

## [v0.2.94](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.94) · 3 Nov 2026, 11:48 UTC

| Commit | What it does |
|---|---|
| [`d28955f`](https://github.com/thr32423/mb3d-rs/commit/d28955f7fdd9ad61f7b3c0723e5a28566ac260ca) | Version number and release notes of this release |
| [`a93555d`](https://github.com/thr32423/mb3d-rs/commit/a93555dc4a79329c0cc8b91776a603d31a67e76a) | Clouds in the program: every calculation painted them black (dark clouds, a black sky with dense ones such as a storm): the clouds' colour was kept in the Post processing window's state of the processings applied by hand, which Calculate 3D, Calc- and Reset now set back to its default (black); State::new_calc drops only those processings. The command line was not affected. Checked on three scenes with clouds, water and reflections: the program's picture as the command line's (two identical, one within the random first step's noise); test: a new calculation keeps the colour |
| [`7204881`](https://github.com/thr32423/mb3d-rs/commit/720488155af10c324adbe449a90fce572594a21c) | docs/CLOUDS_WATER.md: clouds, water and reflections in three pictures (golden hour, above the clouds, storm), each with its text parameters to paste in with From Clipboard (they render the pictures exactly) and the windows where the settings are (the main window's Cutting page, the Post processing window's Reflections and Clouds sections); linked from the README's clouds |
| [`efffeef`](https://github.com/thr32423/mb3d-rs/commit/efffeeff6634fd5b3869b4aca745aaef08811116) | Post processing and Animation windows: the single precision values (cloud density and coverage, the material's fresnel, iridescence and glow, the motion blur shutter) shown with 6 significant digits (fts_single), as 0.7, no longer 0.699999988079071 (the box showed 0.69999) |
| [`f6380bb`](https://github.com/thr32423/mb3d-rs/commit/f6380bbef400daf552272e385e4b42a6080c0595) | tools/bench.py: the device by mb3d's options alone (--cpu, --card, neither for Auto), no longer MB3D_GPU=0 besides --cpu (the two did the same for mb3d, and an option wins over the variable: MB3D_GPU=0 with --card calculated on the card); --cpu also refused in a variant |
| [`d151292`](https://github.com/thr32423/mb3d-rs/commit/d15129216fcd88c9c59b4e308bcaadc1e0d95d6c) | tools/bench.py: timing and picture sweeps over the example scenes that are not skewed (run SWEEP.json OUT_DIR, resumable; report OUT_DIR): a frozen copy of the program (checksum and commit in the header); before every run it waits while a build runs or the processor is busy; every scene and variant twice, the second round in reverse order, the smaller time counting, a pair more than 15 % apart run a third time (the first render of a new shader on the card includes its compile); the pictures from round 2 (in round 1 the processor fills in part of the first picture while the card's shaders compile: Torii's plain render differed in 34 % of the pixels from itself); the variants as argument lists, never through a shell, refused before anything runs when they repeat each other or the plain render, set a scene key twice, or set what the driver sets (--card, --scale, -o, MB3D_GPU; unknown keys too); every run checked for the device it was calculated on (MB3D_GPU=0 does not stop --card), the strings a variant expects (a scene where it is off: not in use) and a variant marked same (byte for byte on the processor, within the card's own variation, ~0.001 % of the pixels, on the card); the noise as the plain render with the scene's own random first step flipped (the other setting is the plain render itself), pictures and holes (the depth picture's background is 0) beyond 1.5 times it reported. TECHNICAL.md: Measuring a speed-up |
| [`1d49ad9`](https://github.com/thr32423/mb3d-rs/commit/1d49ad97413785f3ff00e3f89d8783256d26f059) | README: Updates for v0.2.93 |

## [v0.2.93](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.93) · 3 Nov 2026, 07:25 UTC

| Commit | What it does |
|---|---|
| [`24aa951`](https://github.com/thr32423/mb3d-rs/commit/24aa951e194632de1f266f1910cd4af1419cfbe0) | Version number and release notes of this release |
| [`ab95ca7`](https://github.com/thr32423/mb3d-rs/commit/ab95ca76ccf71a8e990dc6907cd56d90d3c54678) | Cone march: a coarse ray also ends where its step falls under a tenth of the step the march would take there (CONE_MIN_SHARE, the march's step at most its step limit; Marcher::cone_march and the card's cone pass): far off, its reach grows with the depth and left it crawling along a surface at a small fraction of the pixels' steps, and on the card a handful of such rays held up their band (6 AM - Torii temple, dynamic fog: one band's cone passes 1.5 s, the march 70 % longer than without the cone march, now 10 %). Measured on the 80 example files against none, 0.25 and 0.5 (pictures and holes against the plain render with the random first step toggled, the card's times again with warm shader caches): on the processor (0.2 megapixels, 69 files) the march total -19.5 %, median -22.5 %, best -66 %, worst +1 %, 68 faster (without it in the same sweep -16.7 %), pictures as close to the plain render as without it or closer (cutted sphere over carpet 0.40 % of the pixels instead of 0.59 %); on the card (1 megapixel) total -10.0 %, median -3.1 %, 52 of 69 faster (without it -6.6 %, 40); 0.25 the same on the card, a little further from the plain render on the processor (Abominog-Ua, LightBulbMoon), 0.5 slower. EXPERIMENTAL.md (the figures, the warning about fog on the card gone, the section on 0.2.92), TECHNICAL.md, the README row in all seventeen languages. |
| [`ab375cb`](https://github.com/thr32423/mb3d-rs/commit/ab375cb1e8603156c96c015745f69d2ae210c72c) | Cone march: the coarse rays within the march's step limit, margin 1.5: the coarse ray of a block (Marcher::cone_march, the card's cone pass) stepped by the distance estimate alone, without the step limit every pixel's march keeps (max(DE stop, 0.4) * mh04zsd, MB3D's raystep limiter), so where the estimate is far too large it passed thin parts the pixels would have found: in LightBulbMoon - No, Never (Amazing Surf, 1000 iterations) 1.8 % of the pixels became background at the released margin 3 (4.7 % at 1, still 0.4 % at 6, no margin made up for it); the release's hole check had taken the depth map's largest value for the background (it is 0) and missed them. With the limit a margin of 1.5 is at the noise level in all 70 files the cone march works in (pictures and holes against the plain render with the random first step toggled both ways; at 1 the fog of KentPress and the sphere of cutted sphere over carpet changed), and faster: the march on the processor (0.2 megapixels) total -17.9 %, median -21.1 %, best -66 % (Nothing left but bones), worst +1 %, 68 of 70 faster (v0.2.92 in the same sweep: -11.5 %); on the card (1 megapixel) every file at its noise level, LightBulbMoon's holes 0.1 % (v0.2.92 2.3 %), the march -5.4 % (v0.2.92 -3.8 %); 6 AM - Torii temple (dynamic fog) marches slower on the card with the cone march (+73 %, v0.2.92 +44 %); all five experimental speed-ups in Auto mode (78 files): -11.0 %, median -8.4 % (v0.2.92 -10.2 %, -7.1 %). Test: LightBulbMoon at 192 by 192, at most 1 % of the surface background with the cone march (v0.2.92: 225 of 12317). EXPERIMENTAL.md (the figures, the chart, a section on the cone march in 0.2.92), TECHNICAL.md, the README row in all seventeen languages, the AI guide. |
| [`4a79447`](https://github.com/thr32423/mb3d-rs/commit/4a79447913966110cfa253e523ee40b984c08765) | Windows: the auxiliary windows follow the main window's minimizing again (since pull request 29 they are no longer owned by it, so that it can cover them): hidden when it is minimized, by its own title bar button, the taskbar or Win+D alike (seen in its resize events), and shown again when it comes back, in the order they were activated, the window active before getting the focus back (for a second also from a shown window that takes it, as Wine shows them after the focus was given); a window minimized on its own stays minimized; hidden, not minimized, since a minimized window without a taskbar entry becomes a small title bar at the bottom of the screen; as Delphi's Application.Minimize and Restore did in MB3D. Tested on a Windows build (x86_64-pc-windows-gnullvm, llvm-mingw) under Wine 11.20 in a nested KWin session: the main window covers the Navigator when clicked, minimizing by the window manager and by the title bar button hides Formulas, Lighting and the Navigator, restoring shows them with the main window active, a minimized Lighting window stays minimized |
| [`5b6cd2e`](https://github.com/thr32423/mb3d-rs/commit/5b6cd2e6cf703bbff61554636ed97beb08766ea7) | Navigator, on pull request 29: Reset view takes back the zoom and far plane of the view as they were taken, no longer recomputed for the preview (as the inserted main parameters now keep them, so Reset view gives the opened view again); a right click on the axes' overlay (Image6, over the middle of the preview, which passes its mouse to Image1's handlers with its own coordinates) opens the menu where it was clicked, not 60 pixels up and to the left of it |
| [`4af64fa`](https://github.com/thr32423/mb3d-rs/commit/4af64fa18d0682229df48c1644129ecc95516c97) | docs/EXPERIMENTAL.md: the experimental speed-ups on one page: where they are (the Internal tab's Experimental... button, the window with all five on), each one's gain and picture change, all five together (a tenth off a whole render, a third in open scenes, more than the single gains add up to), how they affect each other (cone march and step relax help each other, cone march and no step limit overlap, the rest independent) and what to switch on; measured 2026-11-02 on the 80 example files; linked from the README; apptest renders the Internal tab and the window with all five on |
| [`f0d28ce`](https://github.com/thr32423/mb3d-rs/commit/f0d28ce9a2ad3eb7e49bc56cdb0bb25bdfd72ba0) | README: Updates for v0.2.92 |
| [`40772ec`](https://github.com/thr32423/mb3d-rs/commit/40772ecb06b1933b7815fe6931c49490489c3b19) | Allow main window to cover auxiliary forms |
| [`f021642`](https://github.com/thr32423/mb3d-rs/commit/f0216423c629baad7ab56e3b41a57bae608a2806) | Keep navigator view unchanged on import |

## [v0.2.92](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.92) · 2 Nov 2026, 07:27 UTC

| Commit | What it does |
|---|---|
| [`9235f9b`](https://github.com/thr32423/mb3d-rs/commit/9235f9b872de2b4057a2432a7f98df977fe04e66) | Version number and release notes of this release |
| [`0830c84`](https://github.com/thr32423/mb3d-rs/commit/0830c8424db50aacaad9b4f80dfba0a87c612d5b) | Cone march (experimental speed-up, not in MB3D): a coarse ray per block of 16 by 16, then 4 by 4 pixels marches from the block's middle with the block's spread (the pixels' starts and directions, computed from every pixel of the block) and the DE stop taken off the distance estimate, three times over the step factor because the estimate is no true bound between rays (at a margin of 1 the thin fins of Abominog-Ua were lost, at 2 still in small pictures), and every pixel's march starts where the coarse ray stopped, in one step; a pixel whose jump lands in the surface all the same is marched from the start; a block whose ray passes the end of the view is background; the dynamic fog's step count estimated along the coarse ray (the march's steps per length); not with the volumetric light, disable analytic DE, inside and outside, 2D slices, nor in the sample marches (the precision probe's CPU pixels without it too); the blocks calculated when a pixel of them is first marched (an atomic per block, another thread waits) per calculation rectangle; on the card the mode M_CONE of the march shader (states P_JUMPED, P_CONE, the CONE buffer at binding 7, the words CONE_ON .. CONE_P_OFF, the state word no_cone), two cone passes per band before its march; scene key cone_march, the window's Cone march box (Mandelbulb3D.ini Experimental with six values, five read as before), labels in all seventeen languages and the CJK font subsets, README (all seventeen languages), TECHNICAL.md, the AI guide; apptest renders the Experimental window; measured on the 80 example files (the march on the processor, 0.2 megapixels, 70 usable): total -18 %, median -18 %, best -64 % (Dainbramage - Nothing left but bones), worst +2 %, no holes, every picture within the march's own noise (the base against itself with the random first step toggled); on the card whole renders of open scenes up to -63 %; tests: the ray of a pixel, the blocks' depths before the surfaces and background blocks, the fog's count, when it is not used, the ini values |
| [`e4cab8e`](https://github.com/thr32423/mb3d-rs/commit/e4cab8e5012dd2e0cadbece012f3e333a6ed776d) | README: Updates for v0.2.91 |

## [v0.2.91](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.91) · 1 Nov 2026, 20:07 UTC

| Commit | What it does |
|---|---|
| [`f6d9b09`](https://github.com/thr32423/mb3d-rs/commit/f6d9b0904ea4e4e3e0bc4a410a31a0725d5911b5) | Version number and release notes of this release |
| [`b5cfa13`](https://github.com/thr32423/mb3d-rs/commit/b5cfa13aebc463ba1a3689177ef964efd5ef96f7) | The experimental speed-ups kept from one start to the next: the window's choice is saved in Mandelbulb3D.ini (Experimental, e.g. 1 1.3 0 1 1: the switch, step relax, secant, free steps, half occlusion) when it changes, and every scene the main window takes gets it, at the start and when a file is opened (MB3D's files have none, so an opened file switched them off); README (all seventeen languages), TECHNICAL.md; test: the ini value read back |
| [`f86042b`](https://github.com/thr32423/mb3d-rs/commit/f86042b93ef6ee706cd0004dde250267b548eb5a) | Gaussian splats window: the Bulb Tracer window's Gaussian splats... button opens it; it exports the main window's scene (Calculate from Auto, around the middle or from the camera; directions; size of each view; colours by direction off, 1, 2, 3; output file) in a thread, with a progress bar per direction, Stop (nothing written) and the result in the window and the messages; the settings and defaults those of mb3d splat; labels in all seventeen languages, the CJK font subsets; README (all seventeen languages) and TECHNICAL.md; the missing experimental speed-ups anchor in eight translations |
| [`a77088c`](https://github.com/thr32423/mb3d-rs/commit/a77088c411c8272914426b818bc969bf0fcd0381) | Gaussian splat export (not in MB3D): mb3d splat FILE -o OUTPUT.ply calculates the scene from 16 directions (--views) of 640 x 640 pixels (--size) and makes every object pixel a flat Gaussian on the surface with its colour in the picture (lights, shadows, occlusion, reflections), written as the PLY of 3D Gaussian splatting (INRIA layout) that Blender 5.3, SuperSplat and other splat viewers open; objects seen from outside around the scene middle, scenes seen from inside from the camera in every direction (--around middle\|camera\|auto: auto when the first picture is all object; 110 degree views with the eye and the start plane's distance of the scene); the lights relative to the viewer and the ambient light's top and bottom fixed to the object as in the scene's view (Lighting::amb_frame, not saved; pictures without it byte-identical); each patch of the surface from the view that saw it most squarely, colours that change with the direction by spherical harmonics fitted to the views (--sh 0..3, default 2), the start plane's cut kept only from the scene's view; in the scene's frame (x right, y into the picture, z up), scaled to radius 1; examples/splatcheck.rs compares a file rendered in a splat viewer with the program's picture from a direction between the views; README, TECHNICAL.md; tests: the views, the scene's own view, the fixed lights, the camera's eye kept, the quaternion, the PLY layout, the merge, the harmonics |
| [`47d1a5a`](https://github.com/thr32423/mb3d-rs/commit/47d1a5a3aeeece41770541112688beec77fafb78) | README: Updates for v0.2.90 |

## [v0.2.90](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.90) · 1 Nov 2026, 09:15 UTC

| Commit | What it does |
|---|---|
| [`1d56285`](https://github.com/thr32423/mb3d-rs/commit/1d56285eeaf3acf23f8af626ea8285ac0be8005f) | Version number and release notes of this release |
| [`02c7d3d`](https://github.com/thr32423/mb3d-rs/commit/02c7d3de88dbf73faaf6bd1518378cd438854c83) | Experimental speed-ups (not in MB3D), off unless switched on, each on its own: step relax (longer march steps, an overstep taken again at its safe length; the dynamic fog keeps its count), secant search for the surface (a probe on the other side interpolated between the two), no step limit without dynamic fog and volumetric light (not with disable analytic DE), the DE occlusion at half resolution (every second pixel of every second row, the others filled from the four around them by depth and normal; on the card an even row's calculated pixels first, so its groups run together); on the processor and the card alike (card vs CPU as without them); the window Experimental speed-ups from the Internal tab's Experimental... button; scene keys experimental, step_relax, secant_search, free_steps, occlusion_half (also in the MB3D-RS block); measured on the 80 example files in Auto mode: step relax 1.3 median -5 %, secant -2 %, free steps -1 % (up to -30 %), half occlusion -18 % over the 10 files with DE occlusion; README (all seventeen languages), TECHNICAL.md, the AI guide; the labels in all languages and the font subsets; tests: the switch, the files, the fill |
| [`2d8ef91`](https://github.com/thr32423/mb3d-rs/commit/2d8ef91fd5dfcbd3c2fadbc113acbc824ded3aeb) | Wrapped texts in Chinese and Japanese break between characters (they have no spaces between words, so a long label was drawn in a smaller font on one line and cut off); not before closing marks, small kana and the wave dash, not after opening brackets; texts without these characters break at their spaces as before |
| [`a215f42`](https://github.com/thr32423/mb3d-rs/commit/a215f42d9fae6aaf2cd4075ca72497ab8198caa3) | Ground plane in double precision on the card: the waves' 2 pi a value of the shader's precision (in double the shader did not compile and every scene in double precision was calculated on the CPU) |
| [`94471ec`](https://github.com/thr32423/mb3d-rs/commit/94471ec8431713e2d3cb90c16b9b29784a2f1544) | README: Updates for v0.2.89 |

## [v0.2.89](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.89) · 1 Nov 2026, 05:45 UTC

| Commit | What it does |
|---|---|
| [`028adca`](https://github.com/thr32423/mb3d-rs/commit/028adcaf697ee35feddbe6009e098c856aca6a08) | Version number and release notes of this release |
| [`7b09cd4`](https://github.com/thr32423/mb3d-rs/commit/7b09cd45099f2558c886c3fe4eb42621f2588f24) | Finishing, surface relief, turntable, flights and motion blur in the documentation and in all seventeen languages: the labels translated (the Japanese, Korean and Chinese font subsets with their new characters; the relief's suggested size rounded to two digits), the READMEs with pictures, TECHNICAL.md's keys, the AI assistants' guide |
| [`d388096`](https://github.com/thr32423/mb3d-rs/commit/d388096434f4242f1f825b3cd2611492a00c8993) | Camera and animation (not in MB3D): a turntable, keyframes for one whole turn of the camera around the middle of the view (Animation window: Turntable, 8 keyframes in a Catmull-Rom loop; mb3d animate SCENE --orbit STEPS); a recorded flight (Navigator: Rec, every new view at most twice a second becomes the next keyframe); motion blur, each frame the average of several pictures within the shutter time (Animation window: Motion blur and Shutter; mb3d animate --motion-blur N[,SHUTTER]; in the text animation files motion_blur and shutter, in MB3D's .m3a bits 21-29 of the options MB3D ignores); the animation's scene between two frames (Animation::frame_scene_at); tests: the turntable, scenes between frames and across keyframes, both file formats |
| [`ae0431e`](https://github.com/thr32423/mb3d-rs/commit/ae0431e74b061de8b9b0401cdb32fa26c17cd88c) | Surface detail (not in MB3D): a fine relief on the fractal's surface from 3D gradient noise fixed to the fractal, rough, stone (ridges and cracks) or grain, with its strength, size and fineness, and a colour variation; it tilts the surface's normal where the light is calculated (the painting, the reflections' mirror on the processor and with the card's marches, Monte Carlo) and varies the colour, not on the ground plane; Post processing window, Material section: Relief and Colour variation; scene keys detail = kind, strength, size, roughness and detail_color (also in the MB3D-RS block); tests: the noise, the tilted normals, the files |
| [`b3c9061`](https://github.com/thr32423/mb3d-rs/commit/b3c9061999ec9164fd62922ba3efee9d40779050) | Finishing of the picture (not in MB3D): exposure in stops, glow around bright light (the light above a threshold blurred on a smaller copy, a wide and a near part), filmic tone mapping (highlights roll off instead of clipping: the painting runs in floating point when finishing is on), contrast, saturation, warmth, a vignette and colour fringes by the whole picture's middle (also on tiles); the last pass after the depth of field, also on Monte Carlo pictures (after the noise filters); Post processing window: a Finishing section; scene keys finish_exposure, finish_tonemap, glow, finish_contrast, finish_saturation, finish_warmth, vignette, chromatic (also in the MB3D-RS block of .m3p/.m3i); a scene without finishing paints as before; tests: neutral changes nothing, the filmic curve, glow and vignette, the files |
| [`2e43553`](https://github.com/thr32423/mb3d-rs/commit/2e435533f721601a32c72252d12b9b42b53581db) | README: Updates for v0.2.88 |

## [v0.2.88](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.88) · 31 Oct 2026, 11:01 UTC

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

## [v0.2.87](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.87) · 30 Oct 2026, 10:40 UTC

| Commit | What it does |
|---|---|
| [`89f0cb2`](https://github.com/thr32423/mb3d-rs/commit/89f0cb2885697d417c2148525830416cef0c5d05) | Version number and release notes of this release |
| [`7023cb4`](https://github.com/thr32423/mb3d-rs/commit/7023cb43b2c373a19e3c9821a088ec87c2574305) | README in Spanish, Portuguese, Indonesian, Arabic and Urdu, translated from the README of 0.2.86 with each language's interface labels; the language bars of every README name the seventeen languages |
| [`660b0f1`](https://github.com/thr32423/mb3d-rs/commit/660b0f11a015def8d5803914abf80f4aa376b334) | Languages: Spanish, Portuguese, Indonesian, Arabic and Urdu, all 1,796 labels each, with the flags of Spain, Portugal and Indonesia (Arabic and Urdu without a flag, the badge shows the globe); text written right to left: the Arabic script shaped with rustybuzz right to left (its joined letter forms), a line with right-to-left letters laid out as a right-to-left paragraph in display order by the Unicode bidirectional algorithm (unicode-bidi; Latin words and numbers in it left to right, brackets mirrored), a wrapped right-to-left paragraph right-aligned; a shaped run in the first language font that has all its letters; Arabic.ttf and Urdu.ttf subsets of Noto Sans Arabic UI (tools/font_subsets.py, OFL-NotoSansArabic.txt); the windows themselves keep their left-to-right layout |
| [`e770a36`](https://github.com/thr32423/mb3d-rs/commit/e770a36bb5e2ba9fd6c5ef87adb01f3972b7c5d9) | .gitignore: Python's __pycache__ of the tools |
| [`8148548`](https://github.com/thr32423/mb3d-rs/commit/814854849561aad54a2659c1071bd7fdf2809c56) | Languages: the 109 labels the translations lacked (the History, Log and AI assistant windows, the shader clock, the language window's note) in all eleven, so every language now has all its labels; two texts the old generator and the conversion had missed (a call with its text on the next line: the AI window's note, the Z buffer window's values) as labels, the scanner and its test now see such calls; the AI window's message of a missing mb3d as a label; the Japanese, Korean and Chinese font subsets with the new characters, tools/font_subsets.py to make them (never drops a character); labels.py writes the untranslated labels under [Labels], so a line translated there is a label |
| [`ec16516`](https://github.com/thr32423/mb3d-rs/commit/ec165168a97b2abdeb4d8748c1af0f6ce3935bec) | Interface texts as labels, in the way of Microsoft Dynamics 365: every text has a label ID, the form files and the code refer to it as @MB:Id (Caption = '@MB:CalculateNow', tr("@MB:CalculationStopped")), each language file's [Labels] gives the texts once (Id=text, a note with the English text or where it is used under each); 2,042 control and program lines became 1,793 labels, a new window gets the translations a language already has (the German History window: Art, Größe, Ordner, Laden); a label a language lacks shows the English text, built into the program; a form section still gives one control a text of its own (the conversion kept the 7 to 29 translations per language that differed from their label's), old language files keep working; tools/labels.py finds labels to reuse, adds new ones, checks the references and the translations and puts the files in order; the tests check every reference; every window pixel for pixel as before in English, German and Japanese but for the gained translations |
| [`22263e0`](https://github.com/thr32423/mb3d-rs/commit/22263e041909ec5af4a3a9a542e2b8c75134f1c8) | README: Updates for v0.2.86 |

## [v0.2.86](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.86) · 30 Oct 2026, 08:45 UTC

| Commit | What it does |
|---|---|
| [`b126daf`](https://github.com/thr32423/mb3d-rs/commit/b126daf760bdcdc28a38ec293c7841eb54a4b1ad) | Version number and release notes of this release |
| [`7ee8b6b`](https://github.com/thr32423/mb3d-rs/commit/7ee8b6b777d64d61e7635074f46e639806a969a8) | AI assistants: a scene guide (which keys do what: light colours and strength, fog and background, camera, detail, effects, formulas; and how to change lights and formulas), change_scene also with the whole edited scene text (a [light] section of a change switched off the lights it did not list, a [formula] section added a formula), the view as rotation angles in the description so that an assistant turns it by some degrees, formula_help (the names, or a formula's values) and preview_folder (a folder's parameter files on one numbered sheet); tried with Claude Code: a warmer light, a turned view, a formula's iterations, a folder sheet |
| [`2015786`](https://github.com/thr32423/mb3d-rs/commit/2015786dec77e1490bdd02145a389816eac64499) | Technical notes: the Connect to an AI assistant window |
| [`265a0c7`](https://github.com/thr32423/mb3d-rs/commit/265a0c7d2b4035ab0840e7cb767c90c44fd3f9c1) | AI assistants: the "Connect to an AI assistant" window behind the Prefs tab's new AI button (the tab's buttons a little narrower to make room): whether Claude's desktop app and Claude Code are on this computer and connected, Connect / Disconnect for the desktop app (its settings merged, the old file kept), Claude Code's command to copy, things to ask, the output folder and the limits; the connection names the mb3d next to the desktop program; Claude Code's command with the name before its options |
| [`3b1807f`](https://github.com/thr32423/mb3d-rs/commit/3b1807fbc98f330cda8df9d675b67c6a6c39c5ee) | AI assistants: mb3d mcp, the Model Context Protocol over the standard input and output (no network: the assistant's app starts it), with the tools describe_scene, change_scene, render (the picture shown to the assistant, a quick preview or full size), variations (MutaGen, the sheet shown), recent_pictures (the History) and recent_log; files written only into the AI folder, nothing deleted, pictures above 4K only with the user's agreement, 10 minutes at most a calculation, every call logged; mb3d mcp --setup connects Claude's desktop app and prints the command for Claude Code; a small JSON reader and writer of our own (src/json.rs), as the command line program has no dependencies |
| [`ed3f2dd`](https://github.com/thr32423/mb3d-rs/commit/ed3f2dd253c2d9024c2531077a6423e0ee42abbd) | README: Updates for v0.2.85 |
| [`2da6ec0`](https://github.com/thr32423/mb3d-rs/commit/2da6ec048fc0ec91b6c1940edfb45bc22ea34e2f) | README in every language: the scanrepo.dev result of 0.2.85 (safe, 5 of 100, scanned 30 October 2026) |
| [`225a273`](https://github.com/thr32423/mb3d-rs/commit/225a273f9b740854a1ff2593ef5274a682e732c5) | README: the scanrepo.dev result of 0.2.85 (safe, 5 of 100, scanned 30 October 2026) |

## [v0.2.85](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.85) · 30 Oct 2026, 06:58 UTC

| Commit | What it does |
|---|---|
| [`29a1190`](https://github.com/thr32423/mb3d-rs/commit/29a1190ded95a8bbf3e809b6dd1985a03696e7aa) | Version number and release notes of this release |
| [`97b1503`](https://github.com/thr32423/mb3d-rs/commit/97b1503a44bf7a365006687be3d7afcce29e275d) | Status line while the graphics card's shaders compile: a clock that ticks every second since the compile began ("shaders compiling 0:23", "longer than usual, still working" after a minute) beside the time left, and the text's colour fading to the highlight colour and back every 1.2 s, so that a long compile is seen to be working and not hanging; the label's own font comes back when the compile is done |
| [`e6e3089`](https://github.com/thr32423/mb3d-rs/commit/e6e3089fa9f061157aea021191e4a5f2475f57a9) | Log: a check box, list or field whose window has no handler for it (read only when calculating) is logged too, in the order it was used: the toolkit keeps such clicks, changes and selections for the log, which writes them before the next handled event; a line held back while its control kept changing carries the time of its last change. i18n_docs.py: a language bar under the title of every wiki page |
| [`2c93de0`](https://github.com/thr32423/mb3d-rs/commit/2c93de0345d52374f14fbaae6edeb48664a38c5a) | Log: every event of a session in logs/mb3d-<date>.log of the data folder (a file a day, kept 30 days) and in the Log window (the page icon left of the History clock in the title bar): what the user did in every window (the control, its caption and its new value; typing, dragging and wheel turning one line each), windows opened and closed, calculations with their stages, times, where they ran and their statistics, stops and errors, the message list, files opened and saved, everything the programs print, crashes; the command line's runs too; a desktop session that froze, was killed or crashed found at the next start through its lock file and named in the message list; a crash note carries the log's last 50 lines; the window follows the session live or shows an earlier day, filtered by level, kind and words, the selected line in full, Copy and Open log folder |
| [`905be78`](https://github.com/thr32423/mb3d-rs/commit/905be78f20d2b77f43f398b707056367cd492507) | README in every language of the program: docs/i18n/README.<code>.md in German, French, Italian, Dutch, Polish, Kashubian, Hindi, Japanese, Korean, Simplified and Traditional Chinese, translated from the English README of 0.2.84 (all of it but the release list and the credits, which point to the English ones), with the interface labels of each language's .lng; a language bar under the title of every README; each section's English anchor kept as an <a id> line, so links work in every language; tools/i18n_docs.py writes the bars and the wiki's language list and reports a translation that lacks a section of the English README |
| [`0eed2e9`](https://github.com/thr32423/mb3d-rs/commit/0eed2e923eb1f4395a85cf9f0997c2222cd038b8) | History: "Show unique" goes through the closed days too, so a day's count tells what opening it would list (it kept the count before the comparison), and a file seen on a newer day hides its copies on the older ones |
| [`a8fd74a`](https://github.com/thr32423/mb3d-rs/commit/a8fd74a457684dc168fb6a6fd2b4c21950c6d52a) | JIT: the loops switched to the custom formula's return block a second time after the emitter had already left the builder in it; harmless in a release build, but Cranelift's debug build refuses a switch to a block begun and not ended, so a debug `cargo test` failed in mutagen::tests::a_generation for every formula that writes a variable (the stores stand in that block). The second switch is taken out of the single-formula, hybrid and both dIFS compilers |
| [`cd50029`](https://github.com/thr32423/mb3d-rs/commit/cd500290c0f9a11dcc301d83e0ad8d43c4b1ae08) | History: made for a history of any size, and grouped by day: the days are read one by one on a thread, the newest first, and listed as they come (a closed window stops the reading); "Group by day" (on by default) lists the entries under their days, a day's entries and pictures showing when it is opened with a click (the newest day open at the start; a search hides the days without a hit; the sort within a day); the list view holds a window of its rows only (Control::virtual_rows, the rows in sight and 200 beyond, filled as it scrolls); the sort keys made once per entry and the search text once when it is read; the files compared for "Show unique" on a thread, 2000 at a time and only the listed ones, their keys kept between sessions in the cache folder (History/keys.txt, used again while a file's size and time are the same); the last 400 preview pictures kept; a removed entry leaves the list without a reading again; a synthetic history of 200 days with 200,000 entries lists within a second and nothing blocks the window longer than 60 ms. The title bar's history icon moved to the left of the program version |
| [`468f290`](https://github.com/thr32423/mb3d-rs/commit/468f29089ef0e95066d5bcaf91b7f10c3b22c1be) | Navigator and drop-down lists, on pull request 28: the orientation triad is drawn at each preview pass's scale, so it keeps its size on the screen through the 1/8, 1/4 and 1/2 passes and the full one (the pull request left it out of the coarse passes); the mouse wheel changes a closed drop-down list only while it has the focus and the mouse is on it (as on Windows, where the wheel goes to the focused control), so the wheel over any other place still reaches the form's own handler (the Navigator's zoom, the main window's); the formula list's highlight ignores its scroll bar by the widget's own part test (the fixed 16 pixels were off by the border and applied without a scroll bar); the mouse-move hunk that kept the captured scroll bar part is taken out again, the drag and the row selection key on the captured part already, which is never tested again |
| [`e7dba49`](https://github.com/thr32423/mb3d-rs/commit/e7dba49a3c43373d43e92e85bf0a3ffec5f1ec91) | Polish navigator and drop-down interaction |
| [`ca4ca66`](https://github.com/thr32423/mb3d-rs/commit/ca4ca66ec9659808b3d49e0cf097eb35c449bf2e) | README: Updates for v0.2.84 |

## [v0.2.84](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.84) · 29 Oct 2026, 16:28 UTC

| Commit | What it does |
|---|---|
| [`c4b169d`](https://github.com/thr32423/mb3d-rs/commit/c4b169d340ea6728fd0123ddeefbabdf47de0df0) | Version number and release notes of this release |
| [`d92029e`](https://github.com/thr32423/mb3d-rs/commit/d92029ee5aa3e2b6f449d61460ab1e91ee3d491a) | History: a clock icon right of the title bar's LED opens the History window (this program's own): every Calculate 3D's parameter file of History (as MB3D keeps it) now with a thumbnail of the finished picture beside it, and a log of every file opened or saved through the file dialogs or dropped on the window (History/<date>/log.txt); listed newest first as a list sorted by a column header's click, as rows with their pictures or as tiles whose size a slider sets, searched by name, folder, kind and action, filtered by file type, "Show unique" keeping one of several entries whose files have the same bytes; a preview of the selected entry (a thumbnail, parameters rendered small, an image scaled down), Load/Open, Show in folder, Remove; the previews of the open dialogs now queue up for a view of many and preview image files too |
| [`24b9039`](https://github.com/thr32423/mb3d-rs/commit/24b90398c2830c2c82e30784c4cdf9f895071bb3) | Lighting: a lighting change only repaints, as MB3D does; the full repaint that followed 1.5 s after the last change (the hard shadows of a switched or moved light, the reflections, the depth of field) is taken out at the user's wish: on a big picture those passes take as long as the calculation, and MB3D leaves them to the post processing window's buttons; with it go the "shadows next" / "reflections next" line, the lighting window's idle work and the stop of its own repaint by the post processing window |
| [`efc288c`](https://github.com/thr32423/mb3d-rs/commit/efc288c501a35dffee335d71a3e04c25a9737897) | Formulas: a custom formula whose loop never ends with its options (an integer power of 0 in TrifoxComplexAngles, which the MutaGen can set; MB3D hangs there) cost the whole 2,000,000-step cap on every call, so a render of the scene crawled (nightly issue 24: a 40 x 32 MutaGen probe took 186 s on 24 cores, the generation 7 minutes); the machine's cap collapses to 0 after the first call over it, so the later calls on that thread fail at once (281 s -> 0.1 s), the compiled hybrid and dIFS loops hand over to the plain loop after a failed call (which sees the error's kind), the command line names the case in its warning |
| [`ed637c7`](https://github.com/thr32423/mb3d-rs/commit/ed637c7a0f3f50d073a960f99f01ce171f3c684b) | Flatpak: Flathub's rules on AI (no AI-written manifests, no AI-opened submissions); the drafted pull request text removed |
| [`91c0c7b`](https://github.com/thr32423/mb3d-rs/commit/91c0c7bf5cbd9274c68f865b2297ab6d16ca46da) | README: Updates for v0.2.83 |
| [`e1b66f9`](https://github.com/thr32423/mb3d-rs/commit/e1b66f9b564f70547be72a19b3320238eb685105) | Flatpak: the manifest at v0.2.83; the pull request's text |

## [v0.2.83](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.83) · 29 Oct 2026, 11:15 UTC

| Commit | What it does |
|---|---|
| [`97ea986`](https://github.com/thr32423/mb3d-rs/commit/97ea986904de3fba68ce4fdf3e452e71db3ece79) | Version number and release notes of this release |
| [`ac684ce`](https://github.com/thr32423/mb3d-rs/commit/ac684ceff12ce4288a7909efdd7fd382de43fc5e) | Flatpak README: the Platform in the user installation too |
| [`d03000d`](https://github.com/thr32423/mb3d-rs/commit/d03000dce36b2033ea2eae3ee10a4bc7aa7af2aa) | Flatpak: freedesktop runtime 26.08 |
| [`76d39b4`](https://github.com/thr32423/mb3d-rs/commit/76d39b468d1c9c2c4ca6c5cdfbae6edec8b34706) | Flatpak: the local build mirrors the screenshots as Flathub does (flatpak-builder commits them itself) |
| [`dfa6219`](https://github.com/thr32423/mb3d-rs/commit/dfa62197dd5c4405d4c935e6d36925e44c048a84) | Graphics cards: a card listed again under the first card's PCI bus id is the first card (a Flatpak's Vulkan loader finds the NVIDIA manifest through two paths and listed the RTX 2060 twice) |
| [`7c453f8`](https://github.com/thr32423/mb3d-rs/commit/7c453f8abdc43eef7493a039654810030dd27c78) | Flatpak: ISPC's headers installed next to it (Open Image Denoise's AMX kernel includes amx.isph) |
| [`1e3ef8d`](https://github.com/thr32423/mb3d-rs/commit/1e3ef8d64342cadd9c540cf8d05ec7635f124736) | Flatpak: the app ID is io.github.thr32423.mb3d-rs (the repository's name as it is, as Flathub derives it); the placeholder commit quoted |
| [`faa3d0f`](https://github.com/thr32423/mb3d-rs/commit/faa3d0fc894790102026342b29da63d701179cc0) | Safe Rust: the crate denies unsafe code except in oidn.rs (Intel's C library) and the formulas compiled at run time; the hybrid loop's separate loads go through black_box instead of volatile reads (same speed over the 80 example files, examples/formbench), the free memory of Windows and macOS comes from the sysinfo crate |
| [`ca4b3b5`](https://github.com/thr32423/mb3d-rs/commit/ca4b3b5977ff4baee2620b9c706e15bf28f01af5) | Flatpak: the manifest for Flathub (flatpak/, built from source: the crates of Cargo.lock, MB3D's folders, Open Image Denoise with its CPU device), the AppStream description, the menu entry named mb3d-rs, the window named after the Flatpak ID in a Flatpak; tools/flatpak_cargo_sources.py and tools/flatpak_local.sh |
| [`7752e3a`](https://github.com/thr32423/mb3d-rs/commit/7752e3a46de2af89900dbf3733b28b236a1da6e3) | README: Updates for v0.2.82 |

## [v0.2.82](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.82) · 29 Oct 2026, 08:28 UTC

| Commit | What it does |
|---|---|
| [`8b221e1`](https://github.com/thr32423/mb3d-rs/commit/8b221e13614b9e52522ce14859a447d6ee29c800) | Version number and release notes of this release |
| [`51d9893`](https://github.com/thr32423/mb3d-rs/commit/51d98939c84ea8eb78e213fb8857170f7eb02a15) | Lighting: the full repaint after a lighting change (shadows of a switched or moved light, reflections, depth of field) runs as a processing of the post processing window: Calculate 3D turns into Stop, the LED blinks, the line and the bar show the stage and its time left, the message list gets its time (a reporter saw a green LED, no way to stop and a 33 s shadow pass on the CPU at 4000 x 2000); a further lighting change or a Calculate now of the post processing window stops it instead of waiting; a repaint's hard shadows and DE occlusion share the card and the CPU as after the main calculation (post_shared), when the card's pipelines are compiled |
| [`62098cf`](https://github.com/thr32423/mb3d-rs/commit/62098cf606e334f63d554d36f62134e5ae443618) | Memory: the reflections' band is sized to half of the free memory (a 2M band peaks at 3.5 GB, measured; 1792 bytes per pixel, at least 256k pixels), the memory estimate models the stages (MemoryUse: G-buffer, window, main calculation's card buffers, paint with the reflections' band and the depth of field) so the question before Calculate 3D and the command line's note follow the scene and the swarm's owner, whose tiles' computers do the main calculation; the swarm's whole-picture reflections and depth of field report their progress and obey Stop; --swarm prints the memory note |
| [`995466a`](https://github.com/thr32423/mb3d-rs/commit/995466a4d3258ac51598566e044aba6cbeeeb9da) | README: Updates for v0.2.81 |

## [v0.2.81](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.81) · 28 Oct 2026, 06:45 UTC

| Commit | What it does |
|---|---|
| [`30158e3`](https://github.com/thr32423/mb3d-rs/commit/30158e3840ba84159e24f8bd94b62a7e7c944c8c) | Version number and release notes of this release |
| [`028b520`](https://github.com/thr32423/mb3d-rs/commit/028b5207fd27babd11002d03f876bb37b0f85b1b) | Memory: the card's ambient occlusion uploads its depth buffer and the parameters of all its levels once (an upload per block and level queued 60 GB and left 7 GB behind at 12000 x 6000: the Windows "freeze" of big pictures), the reflections in bands of 2M pixels at every size with their lists dropped as used (1.1 GB instead of up to 4.5 GB), a question before Calculate 3D when the picture needs more than the free memory (Windows and macOS too, no longer a Linux-only refusal), .m3i files written row by row; MB3D_STAGE_TIMES and MB3D_GPU_TIMING print the process memory |
| [`7948665`](https://github.com/thr32423/mb3d-rs/commit/79486652c981b41eb703e194149ad6bf54dbc88d) | Languages: a one character text of another script (Japanese 中, 幅:) is translated back when the language changes |
| [`f7c79d5`](https://github.com/thr32423/mb3d-rs/commit/f7c79d5b6ac78ff9f69f63f3abca5c49ced9e5ab) | README: Updates for v0.2.80 |

## [v0.2.80](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.80) · 27 Oct 2026, 17:55 UTC

| Commit | What it does |
|---|---|
| [`5ccdbf9`](https://github.com/thr32423/mb3d-rs/commit/5ccdbf95533640aca1cb65fa8e062352ad46c365) | Version number and release notes of this release |
| [`0b1efb3`](https://github.com/thr32423/mb3d-rs/commit/0b1efb3e45e00b31096cd8627de1e036c37d04aa) | A crash leaves a note in mb3d_crash.txt on every system, from both programs (before only the desktop program on Windows) |
| [`f80d128`](https://github.com/thr32423/mb3d-rs/commit/f80d128feef62f27e2ef628ab28f1aa438a609b8) | README, TECHNICAL: several graphics cards |
| [`e5f390c`](https://github.com/thr32423/mb3d-rs/commit/e5f390c086458dbc9ceacb7c66e0f8cf663b5b95) | Unfinished renders: Save m3i and mb3d resume --m3i write the picture of any checkpoint as a .m3i, also an earlier version's data.bin, the rows not done as background |
| [`8adb62c`](https://github.com/thr32423/mb3d-rs/commit/8adb62caf7a0e29d9d2af1ceef8a295ec6c8739c) | Unfinished renders: the checkpoint of a picture is a .m3i image file (image.m3i, the rows not done as background); Save m3i in the Unfinished renders window |
| [`7bc65d0`](https://github.com/thr32423/mb3d-rs/commit/7bc65d0697e71f370e22928085d8c279671bbe7a) | Revert "Step relax: over-relaxed march steps as a setting (scene key step_relax, Calculation tab), on the CPU and the card; the dynamic fog keeps its step count" |
| [`e8f0bcb`](https://github.com/thr32423/mb3d-rs/commit/e8f0bcb1b825cc870cfa4e89ba1dfadb3748daa8) | Step relax: over-relaxed march steps as a setting (scene key step_relax, Calculation tab), on the CPU and the card; the dynamic fog keeps its step count |
| [`a2401cb`](https://github.com/thr32423/mb3d-rs/commit/a2401cb0d7bd632baec7f15596ecf1b8da28855f) | Graphics cards: a card stopped because its rows came from others goes on with rows still open instead of idling until the pass ends |
| [`caadf59`](https://github.com/thr32423/mb3d-rs/commit/caadf59b65a488823e3fbebb1746b427961987aa) | Graphics cards: integrated graphics used too (a slow card no longer holds up a picture), a dedicated card first |
| [`cdef4e0`](https://github.com/thr32423/mb3d-rs/commit/cdef4e0893367f478e49b18359f78df00915c1d0) | Graphics cards: a slow card no longer holds up a picture: once the free rows are gone, idle cards and CPU threads take the rows a card took and has not delivered (the first result counts, the card whose rows all came stops); ray batches take back a slow card's unstarted rays and race its started ones; the last frames of an animation race on an idle card; further cards compile in the background and join when ready; compiling() per card |
| [`7f76ddc`](https://github.com/thr32423/mb3d-rs/commit/7f76ddc65f5dc86a28ecb5635181c1770e161a61) | Graphics cards: integrated graphics left out when there is a dedicated card, a dedicated card first |
| [`3162ae5`](https://github.com/thr32423/mb3d-rs/commit/3162ae55c27b1d0bfcffbd29242cd36695feb139) | Graphics cards: volumetric light keeps the picture's card; background compiles and their failures per card |
| [`b4dd914`](https://github.com/thr32423/mb3d-rs/commit/b4dd9145e8890011343a144ce5d4e65e726f5907) | Graphics cards: animations render a frame per card at once (not with map sequences), in the command line and the animation window; mb3d animate --card / --cpu |
| [`78141b6`](https://github.com/thr32423/mb3d-rs/commit/78141b6fd667a96a023c4e01e4baeb484782fd6e) | Graphics cards: Monte Carlo and reflection batches on all of them, each further card a slice at the front sized by its speed, the first card and the CPU sharing the rest as before; resumers kept per card |
| [`0c61ba3`](https://github.com/thr32423/mb3d-rs/commit/0c61ba3ab1d0ab7523d26df94e8c8d35092bfb69) | Graphics cards: all of them in the "Graphics card" mode too, taking bands of the rows (one card: as before) |
| [`cf9e10b`](https://github.com/thr32423/mb3d-rs/commit/cf9e10bb42e1d7c995475fa57ad74cb6268a5fc4) | Graphics cards: identical cards counted in the name (RTX 2060 ×2); the swarm's list of computers keeps the count when it shortens the name |
| [`a209e59`](https://github.com/thr32423/mb3d-rs/commit/a209e5957ce19d228756c2dd55c5aa814f560676) | Plan: what is done |
| [`2a6640e`](https://github.com/thr32423/mb3d-rs/commit/2a6640eefab7f9897dbfd7aec97cb810d4eec089) | Graphics cards: all of them march, shade and occlude every picture in Auto mode, taking bands from the shared rows; a failing card is left out alone (MB3D_GPU_TWICE opens the first card twice for tests) |
| [`1cc245e`](https://github.com/thr32423/mb3d-rs/commit/1cc245e177921eeda87733ae2a30368fac1cee23) | Graphics cards: all of them found and opened (the system's high performance card first, one entry per physical card); mb3d --list-cards |
| [`bfdc8fd`](https://github.com/thr32423/mb3d-rs/commit/bfdc8fdb8ff4ed7a0e80393360ea846541fb706a) | Plan: the cards found and used automatically, no manual choice |
| [`ebb7c9b`](https://github.com/thr32423/mb3d-rs/commit/ebb7c9ba846df288df1f9aa27e35ce79c370f07d) | Plan: several graphics cards (parked) |
| [`d4e151d`](https://github.com/thr32423/mb3d-rs/commit/d4e151df8083fa440c04342337d8ab5fca8663c3) | README: Updates for v0.2.79 |

## [v0.2.79](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.79) · 27 Oct 2026, 03:50 UTC

| Commit | What it does |
|---|---|
| [`af5c449`](https://github.com/thr32423/mb3d-rs/commit/af5c44969caa4c6df3c3d5a8eae9f366ed08bd40) | Version number and release notes of this release |
| [`86b0997`](https://github.com/thr32423/mb3d-rs/commit/86b09973f3551f83f559563c1e841bb8d1fd4bf1) | macOS: the formulas compiled at run time on by default, for testing there |
| [`988fc03`](https://github.com/thr32423/mb3d-rs/commit/988fc03b4f663c3518e3e2d7a84a72f14d87348e) | Hotkeys: a key combination per window brings it to the front, one closes the current window; set under Prefs > Hotkeys |
| [`1e821fa`](https://github.com/thr32423/mb3d-rs/commit/1e821fabaea88d376615e6363acf7cfd5f21d8eb) | README: Updates for v0.2.78 |

## [v0.2.78](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.78) · 26 Oct 2026, 18:41 UTC

| Commit | What it does |
|---|---|
| [`ec68907`](https://github.com/thr32423/mb3d-rs/commit/ec68907d338c04aab65db16b2684ce1562561b6f) | Version number and release notes of this release |
| [`de9eac4`](https://github.com/thr32423/mb3d-rs/commit/de9eac436b621786362186772c521862509dcbe0) | Hindi: the shaper's font, plan and the line widths kept, so the windows no longer freeze while they lay out |
| [`01ec189`](https://github.com/thr32423/mb3d-rs/commit/01ec18959de088704fb5b0cf5811d13fe3963343) | Languages: Hindi, its joined letters shaped with rustybuzz in Noto Sans Devanagari |
| [`0a20527`](https://github.com/thr32423/mb3d-rs/commit/0a20527dd9eaa2739a57a5cd0fd6de61cba25326) | README: Updates for v0.2.77 |

## [v0.2.77](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.77) · 26 Oct 2026, 15:04 UTC

| Commit | What it does |
|---|---|
| [`5433d50`](https://github.com/thr32423/mb3d-rs/commit/5433d5004f88714950d4786767b6055ec0b2f072) | Version number and release notes of this release |
| [`c4f9b0e`](https://github.com/thr32423/mb3d-rs/commit/c4f9b0ed86c0228b7cdcee8aaba4c446f772c261) | New formula DomeTree (TGlad's dome tree), on the CPU and the GPU |
| [`6c523cd`](https://github.com/thr32423/mb3d-rs/commit/6c523cd905cfb0447c04cc2f4befa96c5c2d8c76) | JIT: leave the formulas written in Rust alone |
| [`e593cdd`](https://github.com/thr32423/mb3d-rs/commit/e593cddd8c9c5e41e62892efe244624e171123d0) | Post processing: the bar and the time left while it runs (#2) |
| [`b172551`](https://github.com/thr32423/mb3d-rs/commit/b1725511f586d675bb1394a02aac34765b3426de) | README: Updates for v0.2.76 |

## [v0.2.76](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.76) · 25 Oct 2026, 15:31 UTC

| Commit | What it does |
|---|---|
| [`66617ba`](https://github.com/thr32423/mb3d-rs/commit/66617ba3c14579ad8acb2f564cb786d2882b1a71) | Version number and release notes of this release |
| [`e883349`](https://github.com/thr32423/mb3d-rs/commit/e883349dab09c59626caf6adc40e9eed1336ccf0) | Monte Carlo: the neural noise filter (Intel Open Image Denoise), shipped with the program; macOS packages with the JIT for testing |
| [`8368442`](https://github.com/thr32423/mb3d-rs/commit/8368442472ba1504e425e0959374534eb7eabd49) | README: Updates for v0.2.75 |

## [v0.2.75](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.75) · 25 Oct 2026, 12:46 UTC

| Commit | What it does |
|---|---|
| [`b1edcac`](https://github.com/thr32423/mb3d-rs/commit/b1edcac66c0ea62daaa8913cda8f0441e4360fe6) | Version number and release notes of this release |
| [`fee7b2f`](https://github.com/thr32423/mb3d-rs/commit/fee7b2f8a9f32760dceb33a70cc1100e5145eeeb) | JIT: SSE, indexed record fields and the dIFS loop compiled |
| [`9ba38c2`](https://github.com/thr32423/mb3d-rs/commit/9ba38c217ca49720c216aab404963a9789369f15) | JIT: all built-in formulas compiled, also in hybrids of built-ins only |
| [`09fc1f9`](https://github.com/thr32423/mb3d-rs/commit/09fc1f9c10bf0a7922b27ba560a056c87c87fc58) | The formulas compiled at run time by default, except on macOS |
| [`fe8cdd7`](https://github.com/thr32423/mb3d-rs/commit/fe8cdd7cc8e607120e6d461b04134e763c403311) | Formulas compiled at run time (Cranelift, cargo feature "jit") |
| [`3f4a17a`](https://github.com/thr32423/mb3d-rs/commit/3f4a17a90726386ad1f3bce9725fdf828813a72e) | CPU: no direct-call set up for hybrids with a formula that has none |
| [`4d1365b`](https://github.com/thr32423/mb3d-rs/commit/4d1365b4968f2d5610159a74b2396913c1c6277e) | Translated formulas keep their stack in locals |
| [`87a1523`](https://github.com/thr32423/mb3d-rs/commit/87a152371b6a3ae52db4682fa6cbb67c47cf3756) | CPU: the hybrid loops read the iterated vector as separate loads |
| [`180b318`](https://github.com/thr32423/mb3d-rs/commit/180b3187a38fa8c9c34c9a4b0a4809d4a8ce4fd0) | CPU: the hybrid loop calls the translated formulas without the per call set up |
| [`b16bf81`](https://github.com/thr32423/mb3d-rs/commit/b16bf81ef6f39ca9074d92a88a3016f78aa228fb) | Docs: volumetric light runs on the card (since 535d92a), no longer named as a reason for the CPU |
| [`d7b9621`](https://github.com/thr32423/mb3d-rs/commit/d7b962170b7f7fe377accffaac06bb23fa49111d) | Command line: MB3D_STAGE_TIMES=1 prints the time of every stage of a render |
| [`1770792`](https://github.com/thr32423/mb3d-rs/commit/17707927660cd8406199e6888ea366a840fa2e8b) | README: Updates for v0.2.74 |

## [v0.2.74](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.74) · 24 Oct 2026, 14:49 UTC

| Commit | What it does |
|---|---|
| [`2536bca`](https://github.com/thr32423/mb3d-rs/commit/2536bcaeb6316a3249554bacebb2e18fd9438c28) | Version number and release notes of this release |
| [`4b8aff1`](https://github.com/thr32423/mb3d-rs/commit/4b8aff1f4412b4a96e3eb0bb781aa34ea531e654) | The language badge in the theme's title bar colours, its flag muted there |
| [`c89147c`](https://github.com/thr32423/mb3d-rs/commit/c89147c25a4247ef4be59ac7d73dc4b9bf026222) | README: Updates for v0.2.73 |

## [v0.2.73](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.73) · 24 Oct 2026, 14:21 UTC

| Commit | What it does |
|---|---|
| [`af4fb2f`](https://github.com/thr32423/mb3d-rs/commit/af4fb2fd78fd622eb18c3188e756362e501c4e6e) | Version number and release notes of this release |
| [`ffa15a2`](https://github.com/thr32423/mb3d-rs/commit/ffa15a2889a1beb0f2eb769e129d5d98712f5c08) | README: credits for the pull requests #13, #14, #17 and #19 and the report #20 |
| [`39705af`](https://github.com/thr32423/mb3d-rs/commit/39705afb42b0f891f7dcd5fdb3ac53b200b9eb69) | The language badge at the right end of the title bar, next to the version |
| [`a5a2b85`](https://github.com/thr32423/mb3d-rs/commit/a5a2b8522c163cf690c44c399f333ca98b50c24a) | Tab strips: translated tab names no longer wrap onto more rows than the English |
| [`42cc7c7`](https://github.com/thr32423/mb3d-rs/commit/42cc7c7df0419510fa4dc424100a4c139b4bf52d) | Languages: the 308 run-time texts in the ten translations, captions consistent |
| [`b37f960`](https://github.com/thr32423/mb3d-rs/commit/b37f960d1772f8fcb4cc508bd3f2f7aefa8c752b) | Run-time texts: the stage names in the status line translated, the batch window's time left kept as a value |
| [`f26f01e`](https://github.com/thr32423/mb3d-rs/commit/f26f01e69cb037a6f2706f0868323185c6c81d68) | Run-time texts translatable: src/app/main_form.rs |
| [`f3187bb`](https://github.com/thr32423/mb3d-rs/commit/f3187bbd42a66f37d61d562668e2914932679924) | Run-time texts translatable: anim_form, bulbtracer_form, voxel_form, meshview |
| [`ec3940c`](https://github.com/thr32423/mb3d-rs/commit/ec3940c4152dddf8c191794bec68e38c5d0a9c6c) | Run-time texts translatable: tools_forms, small_forms, unfinished_form, file_preview, util, dialogs (Confirm) |
| [`c685c58`](https://github.com/thr32423/mb3d-rs/commit/c685c58c50f13680a6fee2d724623111b59fdd3b) | Run-time texts translatable: mc_form, swarm_form, mutagen_form, engine |
| [`7319ac5`](https://github.com/thr32423/mb3d-rs/commit/7319ac531dc1e7a36911513396524b31045bdd02) | Run-time texts translatable: formula_form.rs, jit_form.rs, navi_form.rs |
| [`06fdd41`](https://github.com/thr32423/mb3d-rs/commit/06fdd41bd13165fe5112833f1f103bb3589f9347) | Run-time texts translatable: light_form, postpro_form, palette_form |
| [`b1f3842`](https://github.com/thr32423/mb3d-rs/commit/b1f3842bc0a1b0fff1e2ea4cbf643c7be410ac56) | README: Updates for v0.2.72 |

## [v0.2.72](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.72) · 24 Oct 2026, 12:52 UTC

| Commit | What it does |
|---|---|
| [`5235f79`](https://github.com/thr32423/mb3d-rs/commit/5235f798e2c94c3f4572b5575e15ef08ebf02000) | Version number and release notes of this release |
| [`cff2dd9`](https://github.com/thr32423/mb3d-rs/commit/cff2dd966c8414a92709507cfc623005b84bc301) | A smaller picture is no longer shown shifted out of view after scrolling a big one (#20) |
| [`4d5a194`](https://github.com/thr32423/mb3d-rs/commit/4d5a19492c448972c74432032aee43c15ed3f752) | File dialog table: a double click on a column header no longer opens the selected file; arrow keys fill the name; texts translated |
| [`668c6c2`](https://github.com/thr32423/mb3d-rs/commit/668c6c21cc5318d7a1d6e0efafa4abde79f6ebe8) | Embed icon in Windows executable |
| [`d00cf66`](https://github.com/thr32423/mb3d-rs/commit/d00cf662a327447ef03e6bc66b3db9c42f7ecf38) | README: Updates for v0.2.71 |
| [`9b62e28`](https://github.com/thr32423/mb3d-rs/commit/9b62e283c8d581e43d4b1db2f5bbf9d3144d532f) | Keep translated file dialog labels |
| [`70aa2ce`](https://github.com/thr32423/mb3d-rs/commit/70aa2ce3c58aaaf6a36993970481f938b4143724) | Use sortable columns in file dialogs |

## [v0.2.71](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.71) · 24 Oct 2026, 11:07 UTC

| Commit | What it does |
|---|---|
| [`68e0b2b`](https://github.com/thr32423/mb3d-rs/commit/68e0b2b7de9a27d5094c95c80f8b91cdc856a292) | Version number and release notes of this release |
| [`b6f510d`](https://github.com/thr32423/mb3d-rs/commit/b6f510d3488fc66e72273a5b0d7fbe24fb65f988) | Languages: the flag pictures in the repository (they were ignored with *.png), spl-test.lng removed |
| [`8e19da0`](https://github.com/thr32423/mb3d-rs/commit/8e19da0bcb97882cf5d3de2a99b416eeb8dd5019) | README: Updates for v0.2.70, and the latest release at the top of the page |

## [v0.2.70](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.70) · 24 Oct 2026, 10:17 UTC

| Commit | What it does |
|---|---|
| [`1b250b3`](https://github.com/thr32423/mb3d-rs/commit/1b250b3142db7d6f9e4296e8b05bf7505e1be256) | Version number and release notes of this release |
| [`3775231`](https://github.com/thr32423/mb3d-rs/commit/37752317a048ac481edbdad21c4eb5ad012d4496) | Windows: the C runtime linked in, so the program needs no Visual C++ redistributable (#18) |
| [`759974c`](https://github.com/thr32423/mb3d-rs/commit/759974cf8ae476ce8159aaa470bafb14e695b362) | Interface language: eleven languages from text files, a badge and a window to choose, captions made to fit |
| [`639dfbf`](https://github.com/thr32423/mb3d-rs/commit/639dfbf2683fd8ff3c7e8d667673cec7f2b57f7e) | Add file dialog sorting |
| [`57f96a7`](https://github.com/thr32423/mb3d-rs/commit/57f96a7151ab9a72141497a304b0706dc31a3dd7) | A continued calculation's bar goes on where the stopped one's was |
| [`134f1c6`](https://github.com/thr32423/mb3d-rs/commit/134f1c6c2aeb28f82b9f5865e920f6a430f26e34) | README: Updates for v0.2.69 |

## [v0.2.69](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.69) · 23 Oct 2026, 14:10 UTC

| Commit | What it does |
|---|---|
| [`91a47b9`](https://github.com/thr32423/mb3d-rs/commit/91a47b9fabdaf59d618a4ed18168b117f76a917d) | Version number and release notes of this release |
| [`503ec76`](https://github.com/thr32423/mb3d-rs/commit/503ec76e16762bf37aef178d02eb6723f63926cf) | Stop ends the card's reflection batch at its next submission, not after the band's rays |
| [`56830e7`](https://github.com/thr32423/mb3d-rs/commit/56830e74e2184c47fc88a173d01080294d753e10) | Time left: one average pace, a bar that glides, and a stopped calculation keeps its status |
| [`837abce`](https://github.com/thr32423/mb3d-rs/commit/837abce95ccc30fd3a20531aed7a643eb13a9579) | README: Updates for v0.2.68 |

## [v0.2.68](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.68) · 22 Oct 2026, 16:35 UTC

| Commit | What it does |
|---|---|
| [`625cb00`](https://github.com/thr32423/mb3d-rs/commit/625cb009bd35528942c989f6862a3850df0cf0ba) | Version number and release notes of this release |
| [`67f0c4d`](https://github.com/thr32423/mb3d-rs/commit/67f0c4de66b704af608d30ff860a83f269656898) | Monte Carlo on the card: a pass no longer hangs when a thread's turn had nothing to march |
| [`11762fe`](https://github.com/thr32423/mb3d-rs/commit/11762fe38584735f968005c9fd69d4c24d15091e) | README: Updates for v0.2.67 |

## [v0.2.67](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.67) · 22 Oct 2026, 11:14 UTC

| Commit | What it does |
|---|---|
| [`1c377d7`](https://github.com/thr32423/mb3d-rs/commit/1c377d77924131a5f0098fa7d09d02a1fc4d165f) | Version number and release notes of this release |
| [`98a3b44`](https://github.com/thr32423/mb3d-rs/commit/98a3b44093e793098bf8251a197627bc1bff5b2f) | Time left: a stage that waits for its shader counts as cold while the compile runs |
| [`d8debbd`](https://github.com/thr32423/mb3d-rs/commit/d8debbd8a87d8119d615a1940bcb6150c3f2689f) | Big render: the tile under way at the finished tiles' time, not at its progress |
| [`3bd0c1d`](https://github.com/thr32423/mb3d-rs/commit/3bd0c1df663fb311073eea47b50c370494d5dd90) | ssaobench, postcheck: the 15 bit SSAO's progress argument |
| [`4e8f908`](https://github.com/thr32423/mb3d-rs/commit/4e8f9086a8b17fe10c227b6f39e5abdc0c01b8cd) | util: smooth_left and about_str go, nothing shows a time left with them any more |
| [`6450cbf`](https://github.com/thr32423/mb3d-rs/commit/6450cbfe5d67af422dd7cd9d8976a38434835374) | Voxel slices and the swarm's units: the time left from the ones finished |
| [`9b42756`](https://github.com/thr32423/mb3d-rs/commit/9b42756e04869e17a9d1f49a33d5b8396992a29f) | Big render: the time left in total from the tiles done; the Batch window says which picture |
| [`d1b7475`](https://github.com/thr32423/mb3d-rs/commit/d1b74757259165799434356f16da953d5107aa7c) | Animation window: the time left from the frames finished and the running frame's pace |
| [`05a5442`](https://github.com/thr32423/mb3d-rs/commit/05a5442ce36f370dd03d1d71fbc58b1f24a5f676) | Command line: the time left of the Monte Carlo passes, the frames and the tiles |
| [`35314d0`](https://github.com/thr32423/mb3d-rs/commit/35314d01fe4c5ee378dd47f898fd01832425ad94) | Monte Carlo window: the time left in the pass measured, and in total when the batch has a target |
| [`cde462f`](https://github.com/thr32423/mb3d-rs/commit/cde462ffd6399b40f4486d8b538683c05b3297fe) | The depth of field and the 15 bit occlusion report their rows, so their time left is measured |
| [`577ca59`](https://github.com/thr32423/mb3d-rs/commit/577ca590c87323377f5f9688f9ddcdeda9348428) | Time left: the model's stage ratios learn this computer's bias |
| [`bab3679`](https://github.com/thr32423/mb3d-rs/commit/bab36794b1f07aa407aa3d3674756be342fd8e4a) | Time left: the probe's measurements feed the model from the start, and a stage whose shaders compile is not the pace of the rest |
| [`54c5b73`](https://github.com/thr32423/mb3d-rs/commit/54c5b73be5390388e25b79192a40ffb934901653) | Time left: one estimate for all stages, shown the way downloads show it |
| [`33f886d`](https://github.com/thr32423/mb3d-rs/commit/33f886dd89235f7f52200e47b6e5db24aa633c55) | Time left: the shared pieces of the estimator, with tests |
| [`a411f42`](https://github.com/thr32423/mb3d-rs/commit/a411f425a18996aa3e6bfd9f75aa3a4805fc296c) | A continued calculation's base is the work kept, not the rows kept |
| [`8035f9a`](https://github.com/thr32423/mb3d-rs/commit/8035f9a5fa1c20db860a213f70b1ff536ced3a56) | Monte Carlo passes weigh their rows by the pixels on the object |
| [`d7f8d46`](https://github.com/thr32423/mb3d-rs/commit/d7f8d46ae2ca35cdfda14cdc4f327ba3884e164a) | The DE occlusion and the reflections weigh their rows by the surface pixels too |
| [`88bc844`](https://github.com/thr32423/mb3d-rs/commit/88bc844b32a19d98ba569fb10d588bd8cd2b9db6) | Progress counts the work done, not the rows: a quick probe weighs the rows first |
| [`91b1dd3`](https://github.com/thr32423/mb3d-rs/commit/91b1dd39b769587755463fe36f0041ccf060370e) | No warnings in the build without the graphics card part |
| [`35c1628`](https://github.com/thr32423/mb3d-rs/commit/35c1628fb17da176e00eb4f713abf83f0e44005d) | Stereo tab: room for the Render & save stereo pair button |
| [`3242046`](https://github.com/thr32423/mb3d-rs/commit/3242046deab2d2b758aac63c925893cdcdeadd26) | README: scan result of v0.2.66 |
| [`0d91268`](https://github.com/thr32423/mb3d-rs/commit/0d9126844b8be6cfe2d4fa1aa51aba858ed064d6) | README: Updates for v0.2.66 |
| [`529c48d`](https://github.com/thr32423/mb3d-rs/commit/529c48d48d0e2eabca292e5348ec21ac2b97113e) | Separate stereo preview and save actions |

## [v0.2.66](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.66) · 22 Oct 2026, 06:58 UTC

| Commit | What it does |
|---|---|
| [`f86a93b`](https://github.com/thr32423/mb3d-rs/commit/f86a93b043787d0f6806630893cf06792acffc68) | Version number and release notes of this release |
| [`3b32ebb`](https://github.com/thr32423/mb3d-rs/commit/3b32ebba024b7f520fd5e19b8f5aa4f415cef0e9) | Swarm test: no needless mut |
| [`00e4d36`](https://github.com/thr32423/mb3d-rs/commit/00e4d36672a76ee6712291764b3c46871af699df) | The program no longer crashes when it ends while the graphics card is being opened or a shader prepared |
| [`86ab723`](https://github.com/thr32423/mb3d-rs/commit/86ab72392f8292a3327ff8f4b4a6f41e3d435f6d) | Loading parameters also drops a calculation that finished but was not shown yet |
| [`c660b2c`](https://github.com/thr32423/mb3d-rs/commit/c660b2c0a422a7a415149e6e59d51a072fde80e7) | Swarm: a job of the same render is taken over only when its owner is gone, not from another computer running it too. |
| [`f0825c2`](https://github.com/thr32423/mb3d-rs/commit/f0825c2de12d1a6b5b63130e8be41d970421aea9) | Swarm: heartbeats, claims and jobs are aged by their files' times on the shared folder, not by the writers' clocks. |
| [`070fd31`](https://github.com/thr32423/mb3d-rs/commit/070fd31941090a033470f178e63466535c8c7f80) | An unfinished output is removed while still claimed; a claim checks that its file was not removed under it. |
| [`2226dc8`](https://github.com/thr32423/mb3d-rs/commit/2226dc89ba86f3349807bdf1d3c29441fee33cd7) | .m3k keyframe files are found from the .m3k's folder, also when saving it elsewhere. |
| [`03c2370`](https://github.com/thr32423/mb3d-rs/commit/03c23708f60f9a17774036c0d95b170e755f56f2) | Text parameters keep a last formula with negative iterations. |
| [`8d6924c`](https://github.com/thr32423/mb3d-rs/commit/8d6924c0afe472b9f8902bfd6781f3d8df26c001) | ffmpeg command on Windows: frames with gaps through a list file, not a numbered input that stops at the first gap. |
| [`f7417ca`](https://github.com/thr32423/mb3d-rs/commit/f7417ca3f4c8831ceb5a687dceed989cd07256cd) | GIF: the end clear code in the width the decoder expects. |
| [`06646c0`](https://github.com/thr32423/mb3d-rs/commit/06646c0831f90c35b75960eedadb531559535914) | Native formulas: regenerated with the div range check |
| [`3d63210`](https://github.com/thr32423/mb3d-rs/commit/3d63210afd8d8c4a99dbb6418207d09af1d8bb8f) | PseudoKleinian: the CPU starts the orbit trap at 1e30 like the GPU |
| [`8934f0d`](https://github.com/thr32423/mb3d-rs/commit/8934f0db697548c1a96bb171c4dd6623a070f833) | Lifted formulas: div and idiv halt on a quotient out of range |
| [`d6a2474`](https://github.com/thr32423/mb3d-rs/commit/d6a2474fca37bed396eeb48deda6d04339d690ba) | Lifting: joined index masks cover the bits of both |
| [`ad0f673`](https://github.com/thr32423/mb3d-rs/commit/ad0f6738951be4e0a19a4c357c090d7e232c2581) | Lifting: a 16-bit write makes its own register data |
| [`bb9e1ec`](https://github.com/thr32423/mb3d-rs/commit/bb9e1ec6f39288feb085761f0b73a17942d5b745) | Lifted formulas: push esp stores the old value, pop esp sets the popped one |
| [`32d67c7`](https://github.com/thr32423/mb3d-rs/commit/32d67c7a9acff31014f92f5b779fc72ea4d9bfa3) | GPU formulas: a jump back to a loop header is found by its block |
| [`705083f`](https://github.com/thr32423/mb3d-rs/commit/705083f78bedd6e321ca820cca2407116ed5fa9e) | GPU formulas: andnpd complements only its destination |
| [`53fa1e6`](https://github.com/thr32423/mb3d-rs/commit/53fa1e605a2cf753af2881dc1d3774f1905b7445) | Formulas: compiled code rotates the x87 stack on a return into a join point |
| [`5c303a6`](https://github.com/thr32423/mb3d-rs/commit/5c303a6507dbf1fee9ea73a0dc4a1dfbd2db9c7a) | Formulas: sqrtps, minps and maxps are not taken for other instructions |
| [`a3502b6`](https://github.com/thr32423/mb3d-rs/commit/a3502b65600f47404de4a51c35d727c85919bca0) | Formulas: ffreep pops the x87 stack |
| [`c9231ea`](https://github.com/thr32423/mb3d-rs/commit/c9231ea304ee4aa5a4cf8fef5c34b52ea70507f5) | Monte Carlo swarm blocks no longer repeat each other's primary rays, lens points and first bounces |
| [`95d98f9`](https://github.com/thr32423/mb3d-rs/commit/95d98f9205913709626833c4f9b87cc6820f1bc3) | DEAO on the card: the first step random of neighbouring pixels no longer shares random numbers |
| [`69c849f`](https://github.com/thr32423/mb3d-rs/commit/69c849f1a936ad80b660c2a8d4a348f60f6b31eb) | Reflection pass on the card: the shadow rays end at the cutting planes as on the CPU |
| [`0d85596`](https://github.com/thr32423/mb3d-rs/commit/0d85596133ba5785d98d28b98b58f43dfce227e9) | Animation help: --swarm renders the selected frames |
| [`96cc33d`](https://github.com/thr32423/mb3d-rs/commit/96cc33d38c2d23fd42a688d3b5a84ab33057372d) | The command line refuses options it would ignore; swarm tiles write --depth, --transparent, --denoise and --stats |
| [`1fc736d`](https://github.com/thr32423/mb3d-rs/commit/1fc736d0deb3b484333bf74b18a82acf0f9cfea5) | Depth images have the size of the picture |
| [`e3f5ddd`](https://github.com/thr32423/mb3d-rs/commit/e3f5ddddf625c3e010f31f029c89da4d0ab17499) | Normals on the Z-buffer are right along the top of a tile |
| [`c6dad62`](https://github.com/thr32423/mb3d-rs/commit/c6dad629f7eae87ac003c83234977dc4b1a5cb7e) | Tiled pictures get the normals on the Z-buffer of the whole picture |
| [`4e87780`](https://github.com/thr32423/mb3d-rs/commit/4e877807782a41473fe45138c8fa48b9c9fc62f5) | Navigator: 'value from main' on the DE stop slider takes the DE stop |
| [`336beba`](https://github.com/thr32423/mb3d-rs/commit/336beba53e95d23ef547a5f4e58eb2a2ac5ef2c5) | Animation: subframe 0 is rejected when interpolating a keyframe |
| [`2dad205`](https://github.com/thr32423/mb3d-rs/commit/2dad2052a136f9aa6b2dad33fd37997c22e1a20e) | Integer fields no longer read '2.5' as 25 |
| [`007832b`](https://github.com/thr32423/mb3d-rs/commit/007832b27503be25ee48785b9a700b9c226915bf) | Navigator: 'send value to formula window' sends the focused value |
| [`a32b22d`](https://github.com/thr32423/mb3d-rs/commit/a32b22dad8df382c6eaad1d8eaf02e4f1df4953f) | Lighting: 'relative to object' keeps the light direction again |
| [`60d0a02`](https://github.com/thr32423/mb3d-rs/commit/60d0a023025bfd418e192e77c24bfb7d5fd3b53f) | Lighting: undo and redo step through the changes again |
| [`d649f8c`](https://github.com/thr32423/mb3d-rs/commit/d649f8ce417c1d5fe9b62139528d358ae33a4260) | Animation: the stereo checkboxes work again |
| [`409dd4d`](https://github.com/thr32423/mb3d-rs/commit/409dd4db803c12c949e7e6f212e6d1a3594a9c22) | Moving or sizing a window does not leave the left button held |
| [`52c4a40`](https://github.com/thr32423/mb3d-rs/commit/52c4a40b65a61fa88aebedc8c2965e2e6d1b0ecb) | Option types characters on macOS |
| [`d1dfa9d`](https://github.com/thr32423/mb3d-rs/commit/d1dfa9db4a74afa2e767870294c9dd93e4c621a8) | A message opened over an unanswered dialog no longer swallows its answer |
| [`20e1fa9`](https://github.com/thr32423/mb3d-rs/commit/20e1fa97cb33ce52e800640a9f7f77aaa9a17b45) | Scrolling down moves as far as scrolling up; touchpad scrolling adds up to whole notches |
| [`4e4e780`](https://github.com/thr32423/mb3d-rs/commit/4e4e7802bbaa5ff2073a4e48e8ae6864d71d8f2c) | Opening a dialog from a mouse press releases the mouse |
| [`337174e`](https://github.com/thr32423/mb3d-rs/commit/337174eb1e3f0fccf17bff53f924c4ddbb43f58d) | Monte Carlo window: the category group follows the height of its panels |
| [`9277b32`](https://github.com/thr32423/mb3d-rs/commit/9277b3230409436d2322b154baeb02985cf30156) | Category panels: stacked in the order of their Top, not of the form file |
| [`4e811d2`](https://github.com/thr32423/mb3d-rs/commit/4e811d237be93fcb783d1583a1890a39c8b59bf1) | File preview: a panicking calculation no longer stops all later previews |
| [`a23f4dd`](https://github.com/thr32423/mb3d-rs/commit/a23f4ddc1ff23a05f5d3bc66ce6921b8b53f8b19) | Bulb Tracer: a preview still stopping no longer paints beside the next one |
| [`5c7551c`](https://github.com/thr32423/mb3d-rs/commit/5c7551c17e9dff6f920d1503603a43ed488fdbf3) | Voxel window: a preview still stopping no longer ends or unstops the slice export |
| [`1351e53`](https://github.com/thr32423/mb3d-rs/commit/1351e535d8c1da6cc7f951748c99dc53fc687041) | Palette: swap the alpha with the colour when dragging past a neighbour |
| [`4df3d8d`](https://github.com/thr32423/mb3d-rs/commit/4df3d8dbb490b518252f8f6bce7583e3688ada5d) | Swarm: a claim just made is not taken for a stale empty one |
| [`bfd70a2`](https://github.com/thr32423/mb3d-rs/commit/bfd70a2596cc10553c672159a57b08ce6242800b) | Swarm: a claim taken over leaves the new claimer's marker |
| [`19a88b6`](https://github.com/thr32423/mb3d-rs/commit/19a88b69e5f7ed2f841f136f20b1e6147b26e832) | Swarm: depth images come back with the pictures |
| [`92fca20`](https://github.com/thr32423/mb3d-rs/commit/92fca20c9dd25aaf260aab8b63919d99dde3e491) | Monte Carlo: re-exporting a finished .m3c leaves no checkpoint behind |
| [`4ff221b`](https://github.com/thr32423/mb3d-rs/commit/4ff221b64f227ea08032e2f8697da567eda41ae9) | Monte Carlo window: a loaded .m3c keeps a checkpoint of its own |
| [`5025ff9`](https://github.com/thr32423/mb3d-rs/commit/5025ff944cfdca6dfe86b73b23d6ac9901418be2) | Resume: each tile of a tiled render keeps a checkpoint of its own |
| [`c4fb861`](https://github.com/thr32423/mb3d-rs/commit/c4fb861dbf5e421d064bb254ac7d47d4d8eed462) | Resume: card rows that overflowed single precision stay out of the checkpoint |
| [`4690661`](https://github.com/thr32423/mb3d-rs/commit/4690661f85fc6cfc08a07f4640f5a595006ac81a) | Resume: stopping a card render keeps its checkpoint |
| [`77737e1`](https://github.com/thr32423/mb3d-rs/commit/77737e1ceec6509ffabafab91fe1b677885bf9a4) | Menus: a click on a menu's border no longer fires its first item |
| [`0b308a8`](https://github.com/thr32423/mb3d-rs/commit/0b308a8856a18391571687244ef4dbae1acc3392) | File dialog: double-clicking a folder no longer picks its first file |
| [`c081f8f`](https://github.com/thr32423/mb3d-rs/commit/c081f8f5ec98447d6fa60ac482659779d9d81135) | File dialog: Enter in the folder field only changes the folder |
| [`aa156e5`](https://github.com/thr32423/mb3d-rs/commit/aa156e57be57236fc61e1302847d6d222deb7c99) | Big renders: a from-row of 0 or less no longer picks a bogus tile |
| [`b87800d`](https://github.com/thr32423/mb3d-rs/commit/b87800dd9aa3aa58f5c4f52f3bcb59c8cae17a1b) | Batch render: deleting a row keeps the check marks of the rows below it |
| [`8a4589b`](https://github.com/thr32423/mb3d-rs/commit/8a4589b5ca0c931265b69cdc5548717ea7eea21f) | Big renders: .big files load their row settings again |
| [`bbf2612`](https://github.com/thr32423/mb3d-rs/commit/bbf2612e5f9d53f5542d586a08071235f9111443) | Swarm: a calculation started since stops the swarm job |
| [`ee79b86`](https://github.com/thr32423/mb3d-rs/commit/ee79b869b69427d1c07dc04d07b3440f883236e5) | Starting a calculation as the last one ends no longer reports the new one finished |
| [`de5161d`](https://github.com/thr32423/mb3d-rs/commit/de5161dbddddfb9ffe57bbb81e186106698892c7) | A stopped or failed calculation no longer leaves its save path for the next one |
| [`7d59abe`](https://github.com/thr32423/mb3d-rs/commit/7d59abe6252c87b31434dbe92d79109efb1a93cf) | Loading parameters during a calculation stops it first, its old picture is not kept |
| [`64453b3`](https://github.com/thr32423/mb3d-rs/commit/64453b33a3602a3119baa448704a85690370ca05) | Paste parameters no longer loads the last parameters copied here when the clipboard holds no text |
| [`7205fa8`](https://github.com/thr32423/mb3d-rs/commit/7205fa85f4ff9fa998827b0af59ff01ef6a669e5) | Formulas: 80-bit floats convert right for very small values |
| [`a224d5b`](https://github.com/thr32423/mb3d-rs/commit/a224d5b04450eea105f3acb2c55323bf509c2a36) | Monte Carlo files: a bad image size is an error instead of a crash |
| [`0a4f8cd`](https://github.com/thr32423/mb3d-rs/commit/0a4f8cdf11b410fe128f1733732c2d3cb38d822c) | Scenes: fewer tiles drop a chosen tile outside of the new grid instead of crashing the render |
| [`c222ab8`](https://github.com/thr32423/mb3d-rs/commit/c222ab8626d44d38e2cdc5e13b90ed0af7e92700) | Mutagen: a scene that cannot be previewed no longer crashes the first member |
| [`e33ef04`](https://github.com/thr32423/mb3d-rs/commit/e33ef04ea02a445c0b9e0453b40b7fd72c923dab) | Themes: an attribute value quoted with a non-ASCII character no longer crashes the start |
| [`de833de`](https://github.com/thr32423/mb3d-rs/commit/de833de393d69d12bbb36de774d15b550f3081f9) | Pictures: malformed PNG, BMP and JPEG files are errors instead of crashes, and PGM samples are scaled to their maximum value |
| [`6b222d6`](https://github.com/thr32423/mb3d-rs/commit/6b222d67f2cfb563add1bc7b037d45fc630229e9) | Old images: a huge size in the header is no G-buffer instead of a crash |
| [`c01a627`](https://github.com/thr32423/mb3d-rs/commit/c01a627a9a288fefddd1f5604019d3cf1f50588f) | Images: a huge size in an .m3i header is no G-buffer instead of a crash |
| [`78ab85a`](https://github.com/thr32423/mb3d-rs/commit/78ab85aac5d955b680952f1c949d5717198b7f1b) | Parameter files: a bad image size or zoom is an error instead of a crash |
| [`d337240`](https://github.com/thr32423/mb3d-rs/commit/d337240ff8e1bbee4028708fed7cf6df2db0e87c) | Mesh viewer: zooming into a mesh no longer freezes the window |
| [`b28d8eb`](https://github.com/thr32423/mb3d-rs/commit/b28d8ebaeeba5d116c17eb81a1a97bd86cd9d9c7) | Monte Carlo: Batch render waits until a running render has stopped |
| [`6e28c83`](https://github.com/thr32423/mb3d-rs/commit/6e28c83edce7913d955947961fb9c637014adf97) | Animation: inserting a keyframe past the last one no longer crashes |
| [`30d78d0`](https://github.com/thr32423/mb3d-rs/commit/30d78d0fcaaeedcf292929344805cb572498cff9) | Animation: no rendering or second preview while a preview runs |
| [`8e38435`](https://github.com/thr32423/mb3d-rs/commit/8e38435ee26ff2dc731beaa009be87d387781f5e) | Lists: Up and Down on an empty list no longer crash |
| [`9bc7ca2`](https://github.com/thr32423/mb3d-rs/commit/9bc7ca223eae720dc105164ab2b36662ad1516e9) | Formulas: a constant that uses a variable is a compile error instead of a crash |
| [`f14de1d`](https://github.com/thr32423/mb3d-rs/commit/f14de1d61258289a87be72b7f772826d98ca75ad) | Formulas: compiled code counts calls and returns against the step limit |
| [`cae790d`](https://github.com/thr32423/mb3d-rs/commit/cae790d7c126138b9f5bfc7524f21dc045a38dd4) | Formulas: idiv overflow halts the formula instead of crashing |
| [`b1e684d`](https://github.com/thr32423/mb3d-rs/commit/b1e684da8408928cded89399d381d76cf81e7101) | Monte Carlo on the card: a thread that panics ends the pass instead of leaving it waiting |
| [`faccf73`](https://github.com/thr32423/mb3d-rs/commit/faccf7372bf64ce8bcf63faf9244a3ecaf73c7c7) | Graphics card: a shader compile that fails in the background no longer leaves its waiters waiting forever |
| [`2e0ea4b`](https://github.com/thr32423/mb3d-rs/commit/2e0ea4bffc129965b7a38dfb665f51bfcc2f9bab) | Reflections: the CPU workers no longer wait forever when the card fails |
| [`c2ae0e7`](https://github.com/thr32423/mb3d-rs/commit/c2ae0e7b4ffe247e72a4a914868eeb2198342610) | .m3k files keep stereo and transparency |

## [v0.2.65](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.65) · 21 Oct 2026, 15:27 UTC

| Commit | What it does |
|---|---|
| [`f28eac1`](https://github.com/thr32423/mb3d-rs/commit/f28eac1602ef842b9e66b3f98a686d0e7a99ce00) | Version number and release notes of this release |
| [`6655c59`](https://github.com/thr32423/mb3d-rs/commit/6655c59db2127c1138b989b2a92cabffd8890e8a) | README: guides and help in the wiki |
| [`341d840`](https://github.com/thr32423/mb3d-rs/commit/341d8400a92ff04d98de61de900b781331b5a547) | README: newest releases only; the whole history in docs/UPDATES.md |
| [`92753a5`](https://github.com/thr32423/mb3d-rs/commit/92753a5b0c65d635fbb75073b75c3ec4bd3d047c) | README: Updates for v0.2.64 |
| [`be968e4`](https://github.com/thr32423/mb3d-rs/commit/be968e4897b3c0af6dd8c92bb121e825a25eae2d) | Group auxiliary Windows forms under main taskbar entry |
| [`0c6e8a0`](https://github.com/thr32423/mb3d-rs/commit/0c6e8a0b0fb9dd86d0ab7dcc1689991241bb0bb8) | Restore navigator coordinate triad |
| [`4a5aa72`](https://github.com/thr32423/mb3d-rs/commit/4a5aa724fb160d15cfe8ad490c10488d08ae5335) | Auto-save completed stereo renders |

## [v0.2.64](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.64) · 21 Oct 2026, 06:20 UTC

| Commit | What it does |
|---|---|
| [`eeefaee`](https://github.com/thr32423/mb3d-rs/commit/eeefaee2c6a00c3d678f97f93955cf5627b25799) | Version number and release notes of this release |
| [`083cf9c`](https://github.com/thr32423/mb3d-rs/commit/083cf9c2369bbbc8a5f9b64bc39888b4e6c769db) | README: lighting stays live; time left while shaders compile |
| [`8d80bd9`](https://github.com/thr32423/mb3d-rs/commit/8d80bd9f4d593c2fd92f7f30e886e500c13d73b7) | Time left while the card's shaders compile |
| [`a719a67`](https://github.com/thr32423/mb3d-rs/commit/a719a678825b8853698c1d56efa4a8b0aa053274) | ETA data: compile jobs and main pass times per calculation in MB3D_ETA_LOG |
| [`038aa76`](https://github.com/thr32423/mb3d-rs/commit/038aa769486efa94c87a6548f365c7d3d3fa1d82) | Lighting changes repaint at once, as in MB3D; stopped renders survive lighting changes |
| [`f664bd4`](https://github.com/thr32423/mb3d-rs/commit/f664bd49278e66487cb35be2f0057a96c08315b5) | README: Updates for v0.2.63 |

## [v0.2.63](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.63) · 21 Oct 2026, 00:24 UTC

| Commit | What it does |
|---|---|
| [`e0f3014`](https://github.com/thr32423/mb3d-rs/commit/e0f30142c7358127d82585a72c4eb176bcda0887) | Version number and release notes of this release |
| [`c9b9877`](https://github.com/thr32423/mb3d-rs/commit/c9b987788ce0f39ae76db2f5434a693cb466806a) | README: time left for the whole picture |
| [`56e9ee2`](https://github.com/thr32423/mb3d-rs/commit/56e9ee2a2055f0628db9a6634f345b6b903e34bd) | Main window: time left in total, progress bar over the whole calculation |
| [`300e981`](https://github.com/thr32423/mb3d-rs/commit/300e9819619fa38d44d3564d554e9b1c168f1e89) | README: issue reporters in the credits |

## [v0.2.62](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.62) · 20 Oct 2026, 11:50 UTC

| Commit | What it does |
|---|---|
| [`4700559`](https://github.com/thr32423/mb3d-rs/commit/4700559eb6d6943ee86d4ac50bc35debe084650c) | Version number and release notes of this release |
| [`4f74a5f`](https://github.com/thr32423/mb3d-rs/commit/4f74a5fb981cdf03d5047e7071063130e4588baa) | README: the stereo pair preview, with credit to its contributor |
| [`3fd7f33`](https://github.com/thr32423/mb3d-rs/commit/3fd7f332ee2e24ccb1348d1ecd2f9f66ca0d5fb1) | Main window's Stop stops an animation render (#9) |
| [`556d44e`](https://github.com/thr32423/mb3d-rs/commit/556d44ebf4dd0a6c6223208030281679d8e17d41) | Add GUI stereo pair preview |
| [`d32154f`](https://github.com/thr32423/mb3d-rs/commit/d32154faf83b3b785b448d63aaf6accfd7416aac) | README: Updates for v0.2.61 |

## [v0.2.61](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.61) · 19 Oct 2026, 19:09 UTC

| Commit | What it does |
|---|---|
| [`da47383`](https://github.com/thr32423/mb3d-rs/commit/da47383568624693fd66d078afb2079a36857cf8) | Version number and release notes of this release |
| [`69b796c`](https://github.com/thr32423/mb3d-rs/commit/69b796c7784e8720692a018489582f0c41b28893) | Monte Carlo: no black picture on Stop or when continuing |
| [`7fa56e4`](https://github.com/thr32423/mb3d-rs/commit/7fa56e463c25d71ff5f35a96c5687c8801ed2a64) | README: Updates for v0.2.60 |

## [v0.2.60](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.60) · 19 Oct 2026, 18:18 UTC

| Commit | What it does |
|---|---|
| [`0c4b844`](https://github.com/thr32423/mb3d-rs/commit/0c4b844dc8ac9f79e2322a85b014ae387892bd2d) | Version number and release notes of this release |
| [`5bfa9e2`](https://github.com/thr32423/mb3d-rs/commit/5bfa9e256860a3f3f45e61583411ec257ad4e134) | Continued renders: kept rows shown at once, stopped renders kept after 10 s |
| [`abd7263`](https://github.com/thr32423/mb3d-rs/commit/abd72636dd2c6966fa1c1a68c1b277562642ed45) | README: Updates for v0.2.59, scan result |

## [v0.2.59](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.59) · 19 Oct 2026, 15:15 UTC

| Commit | What it does |
|---|---|
| [`1a65ff6`](https://github.com/thr32423/mb3d-rs/commit/1a65ff68da41d7b02184e6a8e1419ba2e10090b5) | Version number and release notes of this release |
| [`1e4b894`](https://github.com/thr32423/mb3d-rs/commit/1e4b8941032c6fedb587764d37e62241d9c5537d) | Depth of field on the graphics card |
| [`45d7339`](https://github.com/thr32423/mb3d-rs/commit/45d733929a8d664dfcadd6225c8a8e987abf4a02) | README: Updates for v0.2.58 |

## [v0.2.58](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.58) · 19 Oct 2026, 10:58 UTC

| Commit | What it does |
|---|---|
| [`cfb1402`](https://github.com/thr32423/mb3d-rs/commit/cfb14029475054f303bc64618942c1b7f48b549f) | Version number and release notes of this release |
| [`d9cc235`](https://github.com/thr32423/mb3d-rs/commit/d9cc2359c5849aec9dee075e348681fdc24073d7) | Stage names, progress and time left for every stage (#8) |
| [`e5f3fed`](https://github.com/thr32423/mb3d-rs/commit/e5f3fed7a72932b1b0f55fd208ee48950a8b2383) | Release workflow: the .rpm in an artifact of its own |
| [`99ddfc3`](https://github.com/thr32423/mb3d-rs/commit/99ddfc3c4defc4a8b8620b1ae90ccdecd9329617) | Linux: .rpm package for Fedora, RHEL, Rocky and openSUSE |
| [`4d20136`](https://github.com/thr32423/mb3d-rs/commit/4d201362e62eafa2f5d61a0ff7383f3992a1fc81) | README: Updates for v0.2.57 |

## [v0.2.57](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.57) · 18 Oct 2026, 22:43 UTC

| Commit | What it does |
|---|---|
| [`6db02d1`](https://github.com/thr32423/mb3d-rs/commit/6db02d1e623002ec42e0f1f3397fa9e2c8fd49cf) | Version number and release notes of this release |
| [`99e848a`](https://github.com/thr32423/mb3d-rs/commit/99e848ab4768a08a37b0608ebdca977dfdf9ad57) | Auto: the card and the CPU share the shadows and the occlusion |
| [`397746e`](https://github.com/thr32423/mb3d-rs/commit/397746e6ecf642b1f8c00341b1d45d0bd505e5e0) | The version stays in the title bar |
| [`23cc639`](https://github.com/thr32423/mb3d-rs/commit/23cc639d81b254d22643dd3760eb4cf90dabc26e) | README: Updates for v0.2.56 |

## [v0.2.56](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.56) · 18 Oct 2026, 17:04 UTC

| Commit | What it does |
|---|---|
| [`4a4dcca`](https://github.com/thr32423/mb3d-rs/commit/4a4dcca3f0a65a15d92101fd04bcff981445a04a) | Version number and release notes of this release |
| [`5b82070`](https://github.com/thr32423/mb3d-rs/commit/5b820705f7403385ba4bd8cc18536bbb1255040c) | Graphics card: single precision finished on the CPU where double was needed |
| [`e486714`](https://github.com/thr32423/mb3d-rs/commit/e486714a0e41db5a6a22690b93b0d6d75e798152) | Reflections with the graphics card: the CPU finishes when it does most |
| [`af02f18`](https://github.com/thr32423/mb3d-rs/commit/af02f18fea3acdc29b420ec4cab0d2e11f932eef) | README: Updates for v0.2.55 |

## [v0.2.55](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.55) · 18 Oct 2026, 13:28 UTC

| Commit | What it does |
|---|---|
| [`bf00248`](https://github.com/thr32423/mb3d-rs/commit/bf00248f66b793f37f5feb8b30a0564bb62d31ab) | Version number and release notes of this release |
| [`4e344a4`](https://github.com/thr32423/mb3d-rs/commit/4e344a4bbf9d77dd72e92c75e0d385a9411aeb96) | Monte Carlo: much less memory for big images |
| [`dd6598e`](https://github.com/thr32423/mb3d-rs/commit/dd6598e786c7a36938238111259261409406ff81) | CI: no run for documentation-only pushes |
| [`02ba3c8`](https://github.com/thr32423/mb3d-rs/commit/02ba3c8fb4e9bd4c76fbb287465b061ca3fd9184) | README: Updates for v0.2.54 |

## [v0.2.54](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.54) · 18 Oct 2026, 11:24 UTC

| Commit | What it does |
|---|---|
| [`6752cd9`](https://github.com/thr32423/mb3d-rs/commit/6752cd95f40b5e5d9821fa914770632f8d2df71a) | Version number and release notes of this release |
| [`0f6b8ed`](https://github.com/thr32423/mb3d-rs/commit/0f6b8ed65b0a69c24ab84ba843f43ea289d51338) | Stopped renders continue where they were |
| [`13b421e`](https://github.com/thr32423/mb3d-rs/commit/13b421e420bfd64a36337f0b1d44ae39ce498441) | Nightly tests that report problems as issues |
| [`5e0da28`](https://github.com/thr32423/mb3d-rs/commit/5e0da286986a00970ad2bc073bfed952ea9865de) | README: Updates for v0.2.53 |

## [v0.2.53](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.53) · 18 Oct 2026, 08:22 UTC

| Commit | What it does |
|---|---|
| [`3134a1a`](https://github.com/thr32423/mb3d-rs/commit/3134a1a67f99a39064fb3acd93665b81504eb858) | Version number and release notes of this release |
| [`b30996b`](https://github.com/thr32423/mb3d-rs/commit/b30996b8b797f22143b401b8293db1713fc60e50) | Swarm: a computer no longer takes its own program's jobs on Windows and macOS |
| [`f2686e3`](https://github.com/thr32423/mb3d-rs/commit/f2686e3d994038041c80c60a79a32a9d7cddf0ce) | README: the credits without the separate line on MB3D |
| [`b0a12b6`](https://github.com/thr32423/mb3d-rs/commit/b0a12b6542d9607679843b2c997d08c80eac3724) | README: the credits table without its sources line |
| [`c336513`](https://github.com/thr32423/mb3d-rs/commit/c336513f5beacc045642a15b49af9488552c44cb) | README: the authors of Mandelbulb3D in the credits |
| [`ae8f0fe`](https://github.com/thr32423/mb3d-rs/commit/ae8f0fe56c0d5d7d2c3b74d314e69fb5088b19bc) | README: Updates for v0.2.52 |

## [v0.2.52](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.52) · 18 Oct 2026, 00:19 UTC

| Commit | What it does |
|---|---|
| [`3cbdfbc`](https://github.com/thr32423/mb3d-rs/commit/3cbdfbcd26cd9c20b7e3114f5e54e5653297dbaa) | Version number and release notes of this release |
| [`8c99c20`](https://github.com/thr32423/mb3d-rs/commit/8c99c20ad5f87ad6909c94f6b0833f8f009e0d05) | Swarm: tiles of Calculate 3D no longer paint what nobody uses |
| [`0e251aa`](https://github.com/thr32423/mb3d-rs/commit/0e251aadc4bfeab41a92f9c6a4911c4d1111826d) | README: the swarm in a section of its own, with how to set it up |
| [`a1c5e70`](https://github.com/thr32423/mb3d-rs/commit/a1c5e70d626988508bcfa552fdbdbb5d371c1a6d) | README: Updates for v0.2.51 |

## [v0.2.51](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.51) · 17 Oct 2026, 22:09 UTC

| Commit | What it does |
|---|---|
| [`03c760f`](https://github.com/thr32423/mb3d-rs/commit/03c760f67aaa2b952898ed1db223bc8957de451b) | Version number and release notes of this release |
| [`d8282fe`](https://github.com/thr32423/mb3d-rs/commit/d8282fe94eb30f086f50c6c8f3cebaf26da91ea2) | Swarm: rendering shared between your computers |
| [`d5d14e6`](https://github.com/thr32423/mb3d-rs/commit/d5d14e6f88d37415825120f21d39ba5787ec5b56) | Quitting during a Monte Carlo render no longer crashes in the driver |
| [`cdb708f`](https://github.com/thr32423/mb3d-rs/commit/cdb708fc941c37f00f33aee6ed6ce446ca51cf34) | README: Updates for v0.2.50 |

## [v0.2.50](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.50) · 17 Oct 2026, 12:15 UTC

| Commit | What it does |
|---|---|
| [`e12397d`](https://github.com/thr32423/mb3d-rs/commit/e12397d215316464d53dff19c7cb714ff81433a9) | Version number and release notes of this release |
| [`a97340e`](https://github.com/thr32423/mb3d-rs/commit/a97340e222edf496d7393d7011279bdf69adc3b8) | Big pictures: far less memory (a 15360x15360 picture fits in 32 GB) |
| [`8edc11f`](https://github.com/thr32423/mb3d-rs/commit/8edc11fa7ab6113d4a38bf04168b522952b75b89) | README: the windows section folded into the list (themes, interface size, the windows) |
| [`dc4a2fe`](https://github.com/thr32423/mb3d-rs/commit/dc4a2fe3ac548c8b65fa2758c2db18cd3ed00117) | README: one "What is new" list, with the detailed write-ups as collapsible items |
| [`2ee83df`](https://github.com/thr32423/mb3d-rs/commit/2ee83df437361a66cdf7a98a2924ee0863d27e64) | README: drop the "Restored Mandelbulb3D functionality" section and its pictures |
| [`49e2858`](https://github.com/thr32423/mb3d-rs/commit/49e28588b18ca637e9a3a6d99ea65ef113b2fabd) | README: Updates for v0.2.49 |

## [v0.2.49](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.49) · 17 Oct 2026, 05:37 UTC

| Commit | What it does |
|---|---|
| [`0e863eb`](https://github.com/thr32423/mb3d-rs/commit/0e863ebb66c1997c4dcb5bc3744330c51d1fe059) | Version number and release notes of this release |
| [`2f71f99`](https://github.com/thr32423/mb3d-rs/commit/2f71f99704f2235a78722be4843df0f699592d39) | New formula PseudoKleinian: knighty's pseudo Kleinian, on the CPU and the GPU |
| [`27c9634`](https://github.com/thr32423/mb3d-rs/commit/27c96346e428c399c6f074437540cefb2df7c583) | README: Updates for v0.2.48; the 2D Ulam spiral and its story |

## [v0.2.48](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.48) · 17 Oct 2026, 05:02 UTC

| Commit | What it does |
|---|---|
| [`993775a`](https://github.com/thr32423/mb3d-rs/commit/993775aa5c1a51ddfcc528ebf61aaf59d584096b) | Version number and release notes of this release |
| [`c5e53d2`](https://github.com/thr32423/mb3d-rs/commit/c5e53d2812c8af9e718597f290f107f0e87e1f48) | Main window: picking a point from the image no longer moves the view |
| [`6090b8a`](https://github.com/thr32423/mb3d-rs/commit/6090b8a7461afa79faf87294596df63b89fdeeb4) | New formula UlamSpiral: Ulam's prime spiral in 3D, on the CPU and the GPU |
| [`49cf64e`](https://github.com/thr32423/mb3d-rs/commit/49cf64e4dbc0742d61cb1867f45de6a95c7a5e9a) | README: Updates for v0.2.47 |

## [v0.2.47](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.47) · 16 Oct 2026, 19:16 UTC

| Commit | What it does |
|---|---|
| [`1b630d9`](https://github.com/thr32423/mb3d-rs/commit/1b630d93e5dcdcf18e479386cd0868336d13a86b) | Version number and release notes of this release |
| [`3d0cd99`](https://github.com/thr32423/mb3d-rs/commit/3d0cd998bbde720c4836d8c6b0554fc8305ccdc0) | New formula SphereTree (TGlad's sphere tree), on the CPU and the GPU |
| [`fb5f6af`](https://github.com/thr32423/mb3d-rs/commit/fb5f6af7e5de89f49362c7aded629b42fb6a1fdc) | README: banner with the RS icon |
| [`18877c8`](https://github.com/thr32423/mb3d-rs/commit/18877c893ea8007fbc4dfe3e2a525a0f697d75c6) | download.sh: say which version it installs |
| [`a87d266`](https://github.com/thr32423/mb3d-rs/commit/a87d2669ccf57c837495175b359c2eceeb7ae79e) | README: Updates for v0.2.46 |

## [v0.2.46](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.46) · 16 Oct 2026, 14:45 UTC

| Commit | What it does |
|---|---|
| [`f66b666`](https://github.com/thr32423/mb3d-rs/commit/f66b666ac1cce03672beae431c6d33cbf4610b32) | Version number and release notes of this release |
| [`fec44aa`](https://github.com/thr32423/mb3d-rs/commit/fec44aa5dce22e743bc2eb4b61f1033db4033c91) | Videos of animations without other programs: AVI, ffmpeg command |
| [`1d40e75`](https://github.com/thr32423/mb3d-rs/commit/1d40e750149a0354811ac8ee3508347c63eec0a4) | Post processing: selections as in MB3D, Stop ends every step |
| [`517f8ac`](https://github.com/thr32423/mb3d-rs/commit/517f8ac7ce0e0c9f668060210d51cbbd801d1404) | Icon: capital RS in all desktop sizes, window matched to its menu entry |
| [`cd76d6e`](https://github.com/thr32423/mb3d-rs/commit/cd76d6ebef893b293e21b64b8ad1de1c8f42cc01) | README: 0.2.45 (scan result, big pictures) |
| [`8cd70cd`](https://github.com/thr32423/mb3d-rs/commit/8cd70cd39d9353f0609a42b49b871a26337866b5) | Remove the Flatpak files |
| [`57e5baf`](https://github.com/thr32423/mb3d-rs/commit/57e5baf8356448d9eef568c5e7569c5e636dd8c6) | README: Updates for v0.2.45 |

## [v0.2.45](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.45) · 16 Oct 2026, 11:42 UTC

| Commit | What it does |
|---|---|
| [`d9c30ac`](https://github.com/thr32423/mb3d-rs/commit/d9c30ac0094c791bfb1fdbf1780e4a9625b067f7) | Version number and release notes of this release |
| [`aacab39`](https://github.com/thr32423/mb3d-rs/commit/aacab397208d60146b8b784f2b7172dbd850728f) | README and docs: checking safety, .deb, own formulas, packaging |
| [`b033779`](https://github.com/thr32423/mb3d-rs/commit/b033779f54da83e3414b15438186152096dff3d8) | Flatpak: manifest and metainfo (io.github.thr32423.mb3d-rs) |
| [`32863f4`](https://github.com/thr32423/mb3d-rs/commit/32863f4daa564c6c2bc5f2a9a4471da121f5b157) | Linux: glibc 2.28, checked download.sh, .deb, checksums, attestations |
| [`d4bf73d`](https://github.com/thr32423/mb3d-rs/commit/d4bf73d7f37d701bb5a4d5424b3e976aac4f2d11) | New icon: MB3D's icon, transparent, with "rs" |
| [`596f309`](https://github.com/thr32423/mb3d-rs/commit/596f309400d140f0f15dfe3ae3f79d69141d6715) | Standard folders for an installed program; own formulas, maps, themes |
| [`a480ad1`](https://github.com/thr32423/mb3d-rs/commit/a480ad145f0f7f81672bc5f1927dd0bed1f63d20) | mb3d: exit status, progress on a terminal only, output to a pipe |
| [`eb5f701`](https://github.com/thr32423/mb3d-rs/commit/eb5f70187e03327e02ca4cc20e111782fa9009d8) | Big pictures: reflections in bands, depth of field on all cores |
| [`3021a67`](https://github.com/thr32423/mb3d-rs/commit/3021a67924f3ca8ad9d25730b60bbd4d04a5ea89) | Infos: Avg. and Max. iterations as in MB3D |
| [`9d0b022`](https://github.com/thr32423/mb3d-rs/commit/9d0b0221c147e1133aa71230c1a48271a7653eba) | docs: scanrepo.dev result for 0.2.44 (issue #4) |
| [`2535b72`](https://github.com/thr32423/mb3d-rs/commit/2535b7225d04492e132eb21665b3108d0f8e591c) | README: Updates for v0.2.44 |

## [v0.2.44](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.44) · 15 Oct 2026, 23:01 UTC

| Commit | What it does |
|---|---|
| [`114dbb3`](https://github.com/thr32423/mb3d-rs/commit/114dbb30a067627ace13be2443aefa8a7b5de790) | Version number and release notes of this release |
| [`b0f94e1`](https://github.com/thr32423/mb3d-rs/commit/b0f94e13c4da29f0a8b742bed48f2b45a8b8191b) | Formula translation modules in src/x86/ (no #[path]) |
| [`f64b8f4`](https://github.com/thr32423/mb3d-rs/commit/f64b8f4d98abe8f7038a40e21f6cd8e43e754fe1) | No unsafe code and no started programs left (issue #4) |
| [`705b80b`](https://github.com/thr32423/mb3d-rs/commit/705b80ba3c2f8f50cae59413a8fe37f314e0e0cf) | Card: no skipped items when the CPU shares a batch |
| [`4906b50`](https://github.com/thr32423/mb3d-rs/commit/4906b5050481442116283bb61ca6c9634fa58f4c) | Monte Carlo window: Statistics and Reflections open again |
| [`1c4e174`](https://github.com/thr32423/mb3d-rs/commit/1c4e17421f9f0dc94f8d9a9351410ddcc2ea8880) | Cargo.toml: drop the removed pcsample example |
| [`f6a1a4c`](https://github.com/thr32423/mb3d-rs/commit/f6a1a4cf2a0a10f0c2a5e698f3c548ee7b0695ec) | README: Updates for v0.2.43 |

## [v0.2.43](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.43) · 15 Oct 2026, 21:36 UTC

| Commit | What it does |
|---|---|
| [`0515c31`](https://github.com/thr32423/mb3d-rs/commit/0515c313fc4a265a7b683fcc1259e9d1787ff69a) | Version number and release notes of this release |
| [`dd3c946`](https://github.com/thr32423/mb3d-rs/commit/dd3c94621f6088240b8dd10c71bfc9f3f7c9b6db) | Fewer patterns that code scanners flag (issue #4) |
| [`3658ae8`](https://github.com/thr32423/mb3d-rs/commit/3658ae8a79c30dfdec49fd2f4b0264d4238acd40) | Start-up lighting as in MB3D |
| [`2aa1bdc`](https://github.com/thr32423/mb3d-rs/commit/2aa1bdc4f5a76983085447e9a53b8c4584ca6631) | README: Updates for v0.2.42 |

## [v0.2.42](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.42) · 15 Oct 2026, 19:26 UTC

| Commit | What it does |
|---|---|
| [`ec17f4c`](https://github.com/thr32423/mb3d-rs/commit/ec17f4ce447e26b52f4d0fedc20375fe4c2a4547) | Version number and release notes of this release |
| [`6b324dc`](https://github.com/thr32423/mb3d-rs/commit/6b324dcc00f57af7c81591bfd44db652fa836ede) | .m3a animation files before version 5, old keyframe formula blocks |
| [`d1fec6f`](https://github.com/thr32423/mb3d-rs/commit/d1fec6f741f4fa90ca947204729a67aa5f9ddc88) | Parameters from before MB3D 1.6 (MandId < 20) |
| [`ca87e14`](https://github.com/thr32423/mb3d-rs/commit/ca87e14aee86ac0452378a0e3b85407eec501e85) | Post processing: shown while it runs and can be stopped (issue #2) |
| [`8487744`](https://github.com/thr32423/mb3d-rs/commit/84877445d274e24d41baf536172ecdc5198ef129) | Recalculate a selection: the selection can be marked on the image (issue #3) |
| [`a87225a`](https://github.com/thr32423/mb3d-rs/commit/a87225a6ba006e4cc826f6ab34adb4578c4347dd) | Sticky windows again: Formulas, Lighting and Post processing |
| [`e9b2def`](https://github.com/thr32423/mb3d-rs/commit/e9b2def0b52641e5086591c4f7b1b81da0f9015b) | Calc-: the rough 8x8 block picture of MB3D |
| [`7939a08`](https://github.com/thr32423/mb3d-rs/commit/7939a0867c174be783330f12192c57b2009c5f1b) | Save dialogs: the file type sets the name's extension |
| [`0e742a5`](https://github.com/thr32423/mb3d-rs/commit/0e742a5de281cde31777ae49d3f97b95c7f3d59b) | README: "Restored Mandelbulb3D functionality" section |
| [`7f62d0f`](https://github.com/thr32423/mb3d-rs/commit/7f62d0fa33cfe5b0eca00b6adae4370e07dbfa7c) | AniGif: save the animation preview as an animated GIF |
| [`95c0c58`](https://github.com/thr32423/mb3d-rs/commit/95c0c586fb6653b457096e42fc0861291ca6ffa8) | Paint on image: paint palette colours on the calculated picture |
| [`9b29d56`](https://github.com/thr32423/mb3d-rs/commit/9b29d56f796855c5cda5366df3d6b64e685b7562) | docs: before/after picture of the panorama seam (issue #1) |
| [`ad5550b`](https://github.com/thr32423/mb3d-rs/commit/ad5550bdd0081003f68a0fc0601f997a27fc2961) | README: Updates for v0.2.41 |

## [v0.2.41](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.41) · 15 Oct 2026, 13:11 UTC

| Commit | What it does |
|---|---|
| [`7a134a2`](https://github.com/thr32423/mb3d-rs/commit/7a134a25a9e8efd52a7fb0e878cfe05b4ddb6c14) | Version number and release notes of this release |
| [`3435f9d`](https://github.com/thr32423/mb3d-rs/commit/3435f9dd8af04b7d2b1f1ca1d0a1ac9fbba87dec) | Workflows: a GitHub token for curl on Windows (the DXC download) |
| [`a81f74c`](https://github.com/thr32423/mb3d-rs/commit/a81f74c8f6bfb5571016b182b3023380b70da73f) | Panoramas: no seam where the left and right edges meet |

## [v0.2.40](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.40) · 15 Oct 2026, 12:36 UTC

| Commit | What it does |
|---|---|
| [`3297426`](https://github.com/thr32423/mb3d-rs/commit/3297426c1d8556fde1577062b04aa21fb58447ea) | Version number and release notes of this release |
| [`a3c158a`](https://github.com/thr32423/mb3d-rs/commit/a3c158a2e9ec95b5932c39ba8947d491d86fa9a8) | Map formulas run as translated native code |
| [`5e05b21`](https://github.com/thr32423/mb3d-rs/commit/5e05b2157aa593aa62b76d50b0e7e40233633164) | README: Updates for v0.2.39 |

## [v0.2.39](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.39) · 15 Oct 2026, 10:59 UTC

| Commit | What it does |
|---|---|
| [`e0be9b5`](https://github.com/thr32423/mb3d-rs/commit/e0be9b5e8be9e10c4d7baf0194fc17c108f9b301) | Version number and release notes of this release |
| [`714d9e8`](https://github.com/thr32423/mb3d-rs/commit/714d9e83599442e7f719d224dbdf50b8cb217b3d) | Open dialog: a preview of parameter files |
| [`d9e33ca`](https://github.com/thr32423/mb3d-rs/commit/d9e33ca2efe2c33c71ed82eef0a815f5d77ae924) | README: Updates for v0.2.38 |

## [v0.2.38](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.38) · 15 Oct 2026, 10:26 UTC

| Commit | What it does |
|---|---|
| [`0c9986e`](https://github.com/thr32423/mb3d-rs/commit/0c9986eb9a12113a636984fdbb2d6b773cf9b85c) | Version number and release notes of this release |
| [`4099048`](https://github.com/thr32423/mb3d-rs/commit/40990480f9f0f720e9c5e991792e76e219be5086) | Tiles come out exactly as the whole picture |
| [`9ebe8b9`](https://github.com/thr32423/mb3d-rs/commit/9ebe8b9d527af2d94cf57a22f008359c9a866bf4) | Navigator: the bottom panel no longer moves the arrows away |
| [`98c2ed6`](https://github.com/thr32423/mb3d-rs/commit/98c2ed67622ea9d92cab8cf11e602fad31c5a9f6) | Formulas' map lookups without locks: Insect Menger 63.8 -> 3.6 s |
| [`7a4a6d6`](https://github.com/thr32423/mb3d-rs/commit/7a4a6d6165b81eb5a9982d8ab861881970859fb2) | Transparent background for pictures and animations |
| [`4ab267e`](https://github.com/thr32423/mb3d-rs/commit/4ab267e9a96a9b197c7ca4c295f97f05e7c9d4c0) | Navigator: type exact values and the zoom; Reset view |
| [`97ac2c7`](https://github.com/thr32423/mb3d-rs/commit/97ac2c7ef4911fc97f868b1862f99266d27d9a66) | README: Updates for v0.2.37 |

## [v0.2.37](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.37) · 15 Oct 2026, 06:49 UTC

| Commit | What it does |
|---|---|
| [`76b6da1`](https://github.com/thr32423/mb3d-rs/commit/76b6da12f5e386adcca6a06d2dd620e8bf9da12e) | Version number and release notes of this release |
| [`42e52a8`](https://github.com/thr32423/mb3d-rs/commit/42e52a8722c335746202dc86d99474d7fb958548) | Noise filter for all pictures: DCT denoising as GIMP's Denoise DCT |
| [`a1a8637`](https://github.com/thr32423/mb3d-rs/commit/a1a863742a4f4bfab3fbf33c4c75535851c751ad) | mb3d montecarlo: a continued .m3c that has its rays already writes its picture |
| [`9a8279c`](https://github.com/thr32423/mb3d-rs/commit/9a8279c417b49114e2c653eb4daa443cf042fe7e) | Monte Carlo: the -noise button works (MB3D's filter, hidden in MB3D) |
| [`a66857d`](https://github.com/thr32423/mb3d-rs/commit/a66857db9b3737d22c3f0f8cd6cd2f411542b0ea) | 16 bit colour pictures: mb3d --bits 16 and "PNG 16 bit colour" |
| [`953e65c`](https://github.com/thr32423/mb3d-rs/commit/953e65ce23be74e635390d0a8d4a8c122b1f68a2) | Animations: also a video (MP4 or WebM) of the frames |
| [`1b9752f`](https://github.com/thr32423/mb3d-rs/commit/1b9752f8ab16f5a69484c71bf90af2807adf3bbc) | Auto: the card helps in double precision where single is too coarse |
| [`0730a59`](https://github.com/thr32423/mb3d-rs/commit/0730a59f83f831b74ccd7d15871892733bfda113) | Drop-down lists stay inside their window and scroll |
| [`477ae4a`](https://github.com/thr32423/mb3d-rs/commit/477ae4a94189a2b4d6d74ef842c754023d403919) | Infos tab: the hard shadow, occlusion and reflection times each on their own |
| [`f177710`](https://github.com/thr32423/mb3d-rs/commit/f1777108a1fa37610698ca2f147b2a2ed719f5f6) | README: Updates for v0.2.36 |

## [v0.2.36](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.36) · 14 Oct 2026, 17:35 UTC

| Commit | What it does |
|---|---|
| [`28c8984`](https://github.com/thr32423/mb3d-rs/commit/28c8984298778887840ed7f1c9c5ffde5ba88614) | Version number and release notes of this release |
| [`e21e881`](https://github.com/thr32423/mb3d-rs/commit/e21e8815c3ec7851ec496f07a4162f5a3722abc4) | Keep the image's aspect ratio when the Scale buttons halve and double it |
| [`3aad0f0`](https://github.com/thr32423/mb3d-rs/commit/3aad0f0ac89c69614e25ef190208886a23875ba9) | tools/check.sh: the CI build and test steps locally, optionally as a pre-push hook |
| [`c1ebf8a`](https://github.com/thr32423/mb3d-rs/commit/c1ebf8aeb50121463a6c7c3177cd2243e1408543) | Build without the gpu feature again: the cfg back on march_mixed |
| [`1de0747`](https://github.com/thr32423/mb3d-rs/commit/1de0747571f3986bf05893df7d569d44388bf89e) | README: updates section with v0.2.35 |

## [v0.2.35](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.35) · 14 Oct 2026, 08:23 UTC

| Commit | What it does |
|---|---|
| [`023218c`](https://github.com/thr32423/mb3d-rs/commit/023218c80d0bd947da7896b475de9261cc522311) | Version number and release notes of this release |
| [`b282bd9`](https://github.com/thr32423/mb3d-rs/commit/b282bd9c0019d449c105a3c71a789b94995693a3) | README: the speed tables measured again (80 example files) |
| [`82d79b1`](https://github.com/thr32423/mb3d-rs/commit/82d79b17dd42170b7d822e0f56a28f6da627b975) | Build: align functions and jump targets; the sine cache per thread |
| [`bcf5aa3`](https://github.com/thr32423/mb3d-rs/commit/bcf5aa32a2018e4186cd4a5c58ed09e759699f76) | README: updates section refreshed |
| [`0de3ae6`](https://github.com/thr32423/mb3d-rs/commit/0de3ae6eedecbfc5050f55269ed1bcdca38356e9) | Translated formulas: exact shortcuts for the symmetry IFS angle and its sine |
| [`f8975ce`](https://github.com/thr32423/mb3d-rs/commit/f8975ce691e512006df39140c6d903413ecfe49f) | Auto: shadows and DE occlusion on the card only when it marched clearly faster |
| [`9f1ae05`](https://github.com/thr32423/mb3d-rs/commit/9f1ae05c9f48670684bd5580af781b2b3d4f670c) | README: an Updates section with every release, its time and its commits |
| [`57a1d1f`](https://github.com/thr32423/mb3d-rs/commit/57a1d1f2be0d6ac16efb2e5520efb5e3725d615d) | SSAO24 on the card: the random mode's seed jumps ahead instead of stepping |
| [`1c0cf6d`](https://github.com/thr32423/mb3d-rs/commit/1c0cf6d85d5df7e9425522e874ea0570a6f5f47c) | SSAO24 on the CPU about 2.7 times faster, same pixels |
| [`9afa081`](https://github.com/thr32423/mb3d-rs/commit/9afa08154ff254ee7c838199e8b59675076a2267) | mb3d: the preset render without arguments again; formulas found next to the parameter file and in ~/mb3d-rs |
| [`7f2e6fb`](https://github.com/thr32423/mb3d-rs/commit/7f2e6fb113699dc7e3e6f396a3986df27dac27fe) | README: download table first, the download.sh one-liner as the alternative |

## [v0.2.34](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.34) · 13 Oct 2026, 19:52 UTC

| Commit | What it does |
|---|---|
| [`1448c01`](https://github.com/thr32423/mb3d-rs/commit/1448c01b83aada508c5b5e63a8a8fba3052a660d) | Version number and release notes of this release |
| [`bae03aa`](https://github.com/thr32423/mb3d-rs/commit/bae03aae799df679ca9d2705064194a238266161) | README for users; the technical notes in docs/TECHNICAL.md; MB3D compared |
| [`4cf6967`](https://github.com/thr32423/mb3d-rs/commit/4cf6967271b8677051aa24715442417a433a89d3) | Auto: double precision scenes stay on the CPU; mb3d waits for compiles; --card |
| [`ed68b01`](https://github.com/thr32423/mb3d-rs/commit/ed68b017e823484fe3e2799234d0270331188f4a) | Card: the formulas' control flow structured in the shaders |

## [v0.2.33](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.33) · 13 Oct 2026, 13:00 UTC

| Commit | What it does |
|---|---|
| [`e584134`](https://github.com/thr32423/mb3d-rs/commit/e584134f99c7efddb6789515b6e92dfa46f2c04c) | Version number and release notes of this release |
| [`20e9edc`](https://github.com/thr32423/mb3d-rs/commit/20e9edcf5d23d0d5d5c71fda98274fcd1b0472b1) | profile: reflections and depth of field apart; paintbench; mb3d-wine finds the window |
| [`967fe36`](https://github.com/thr32423/mb3d-rs/commit/967fe3683f0da0ead618a8f46de19a076e5e7056) | Card: the compiled shaders kept in ShaderCache between runs |
| [`496901b`](https://github.com/thr32423/mb3d-rs/commit/496901b9fce7dc1e195c4d728e1dc86373a3b047) | tools/mb3d-wine: render and save a G-buffer in MB3D under Wine |
| [`e71a78b`](https://github.com/thr32423/mb3d-rs/commit/e71a78b425e8a3e00652a656f8f63b99f6b9492f) | Painting with internal gamma 2 as MB3D does: 180430 matches |

## [v0.2.32](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.32) · 13 Oct 2026, 11:16 UTC

| Commit | What it does |
|---|---|
| [`4e39d49`](https://github.com/thr32423/mb3d-rs/commit/4e39d49a85a5a8cccd4ec8ab5a43365f9229be88) | Version number and release notes of this release |
| [`7ec1842`](https://github.com/thr32423/mb3d-rs/commit/7ec1842175e8fa89591c63163221f6af3d844116) | Windows under Wine: no resize loop; windows paint again |
| [`e489830`](https://github.com/thr32423/mb3d-rs/commit/e489830140a7b6909ebc31da8616390b821be743) | README, Getting started: install with the download.sh one-liner |

## [v0.2.31](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.31) · 13 Oct 2026, 09:38 UTC

| Commit | What it does |
|---|---|
| [`337cdab`](https://github.com/thr32423/mb3d-rs/commit/337cdabc47d74db100dbff92cd57af85645b182e) | Version number and release notes of this release |
| [`b5757cb`](https://github.com/thr32423/mb3d-rs/commit/b5757cbc5bdcc71603cbac88a53414d0dbda30be) | Card shaders compiled when a scene is opened or a formula changes |
| [`0edb088`](https://github.com/thr32423/mb3d-rs/commit/0edb0888f67fb36dbed5b01ae81b6c1b530d943b) | download.sh: the one-line install as a script, also in the release folder |
| [`a94c6ed`](https://github.com/thr32423/mb3d-rs/commit/a94c6ed4afa3854959769a832c788f198c78e070) | README, Getting started: commands for the release folder, building from source at the end |
| [`b9fe839`](https://github.com/thr32423/mb3d-rs/commit/b9fe8399c720e4dd19df56fdd2dc57703bde4d98) | Card: no OpenGL by default; a software renderer for tests; window trace |
| [`c706ad3`](https://github.com/thr32423/mb3d-rs/commit/c706ad3d3cce7867764f34dbd3261d37090ac314) | gbcompare: our G-buffer against MB3D's (from an .m3i it saved) |
| [`050311d`](https://github.com/thr32423/mb3d-rs/commit/050311ddae778bc897fb610bc05c4fe557bad94e) | Tools: time formulas against MB3D's machine code |
| [`3bd8f27`](https://github.com/thr32423/mb3d-rs/commit/3bd8f27edd7e257a93effb50ae7614c34e951821) | Test: the Monte Carlo task's view vector is loaded per task |
| [`7e69757`](https://github.com/thr32423/mb3d-rs/commit/7e6975768aa601c8dd7cb344e9e35686fc502666) | Monte Carlo with volumetric light on the card; smaller Auto bands |
| [`27b1d2f`](https://github.com/thr32423/mb3d-rs/commit/27b1d2f4fe5386d780285413bf2df44066025094) | Card: volumetric light in the reflection pass |
| [`535d92a`](https://github.com/thr32423/mb3d-rs/commit/535d92a735e369a61acdb83afbefe865f9998b83) | Card: the volumetric light's map, shared with the CPU |
| [`c0350b8`](https://github.com/thr32423/mb3d-rs/commit/c0350b804201fab63c7f81ed65c96a343d610de6) | Card: volumetric light in the main calculation, shadows of positional lights |
| [`19073d6`](https://github.com/thr32423/mb3d-rs/commit/19073d6d993f4a44bff187f2adfe9df57ff4ac1e) | MB3D_GPU_TIMING: the volumetric light map and the CPU's hard shadows |
| [`ddc4851`](https://github.com/thr32423/mb3d-rs/commit/ddc48514207660f8ddb495c8c7dc5a01b74545a9) | Hard shadows of positional lights |
| [`062580e`](https://github.com/thr32423/mb3d-rs/commit/062580e2cf815aa3203f93ad41427774f863d1ae) | Reflections: the card's share from the measured rates |
| [`7dbf870`](https://github.com/thr32423/mb3d-rs/commit/7dbf87068ee30a19b2c6d9ce3327c16b16d7700b) | Auto: the precision probe before the card joins |
| [`5ec3d95`](https://github.com/thr32423/mb3d-rs/commit/5ec3d95579c850b2d2c621a6c0be24dace82ec0e) | Themes: all themes are files in the Themes folder |
| [`6277242`](https://github.com/thr32423/mb3d-rs/commit/6277242dc937fc2731ada694c0b797dd9bb8bdb7) | Card: map formulas in double precision shaders |
| [`78529a6`](https://github.com/thr32423/mb3d-rs/commit/78529a6a8b2bf10e054fcf93c703c578c9846b5c) | profile: print the formula error count with MAIN_ONLY |
| [`3756b00`](https://github.com/thr32423/mb3d-rs/commit/3756b00a2000f00ccf3f6d1b9ae5d855418b4bc2) | README: full-size theme screenshots, opened by a click |

## [v0.2.30](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.30) · 12 Oct 2026, 18:52 UTC

| Commit | What it does |
|---|---|
| [`35c7a5c`](https://github.com/thr32423/mb3d-rs/commit/35c7a5c1215a3f2e0af5fad82841bdbd8abcc6a9) | Version number and release notes of this release |
| [`c133917`](https://github.com/thr32423/mb3d-rs/commit/c133917ba7f19f9a1fe804adb2cb63daeeb197aa) | Auto: every image shared between card and CPU, post passes on the card |
| [`69f679b`](https://github.com/thr32423/mb3d-rs/commit/69f679b20f75904953fc1a045689b0b6e9aa2a7b) | Themes as XML files; six new themes |

## [v0.2.29](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.29) · 12 Oct 2026, 17:10 UTC

| Commit | What it does |
|---|---|
| [`2042c55`](https://github.com/thr32423/mb3d-rs/commit/2042c55ca2fad3f2285f1ca512536707a43d396d) | Version number and release notes of this release |
| [`447e764`](https://github.com/thr32423/mb3d-rs/commit/447e764920f7f194698fa1817de96ce6c71aa1a6) | Card: no runaway submissions; a lost card falls back to the CPU |
| [`5d4676b`](https://github.com/thr32423/mb3d-rs/commit/5d4676b5905a41c680ae2ea0926b5d72862fbe8b) | README: "New compared with Mandelbulb3D" section, Linux one-liner |
| [`e5dc427`](https://github.com/thr32423/mb3d-rs/commit/e5dc427edc0ab173571e3cc45f4ed7b99915a552) | Card: double precision where a probe finds single too coarse |
| [`db44845`](https://github.com/thr32423/mb3d-rs/commit/db4484548f33582cb50c45fb78a128104ac0d20d) | README: the CPU formula translator's state |
| [`f986d84`](https://github.com/thr32423/mb3d-rs/commit/f986d845910ce10af21fa6bd60eabb2774deb139) | lift_emit: tests of the structured layout |
| [`1ad89ef`](https://github.com/thr32423/mb3d-rs/commit/1ad89efe6ac7b09a6aea332f6884c30ce7a5b423) | README: planned work (XML themes, CPU formulas, double precision, ...) |
| [`9d84fc9`](https://github.com/thr32423/mb3d-rs/commit/9d84fc9eb80abaae38cde4a1b63a13dec1249760) | Translated formulas: structured control flow, direct dIFS calls |
| [`8128318`](https://github.com/thr32423/mb3d-rs/commit/81283188dd9c67cfe27d9c637f40fce0244bc0b2) | Profiling tools: real rows, main-only timing, width limits |
| [`87e05e5`](https://github.com/thr32423/mb3d-rs/commit/87e05e5984978cda1c36123aed57e661ef5536b6) | Thread priority applied, first-run note, steady LED |
| [`e6a9a38`](https://github.com/thr32423/mb3d-rs/commit/e6a9a387e954e71301910721fd72eb86f0afee23) | README: a readable CPU / card comparison table |

## [v0.2.28](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.28) · 12 Oct 2026, 11:39 UTC

| Commit | What it does |
|---|---|
| [`32735dd`](https://github.com/thr32423/mb3d-rs/commit/32735dd0d395c2c6ea1df2549047da9508723aed) | Version number and release notes of this release |
| [`914ca83`](https://github.com/thr32423/mb3d-rs/commit/914ca8381452d0a36d6ac9f44ebca306dde70916) | docs: the screenshots of RENDERING.md |
| [`9446a48`](https://github.com/thr32423/mb3d-rs/commit/9446a48c065db42bf1ca214d7eabdd426b5c5666) | CPU, mixed and card compared on all 80 examples; rendering explained |
| [`9a321b5`](https://github.com/thr32423/mb3d-rs/commit/9a321b5d43f1dc50430299951cd4d75bacdcf71e) | "Calculate on: Auto": the CPU starts while the card's shaders compile |
| [`4b317eb`](https://github.com/thr32423/mb3d-rs/commit/4b317eb04f24b3c9a096777434e0192863c0ba48) | mb3d: print the help when run without arguments |
| [`0e8f6a4`](https://github.com/thr32423/mb3d-rs/commit/0e8f6a4fcc56af9bddd7ed94e3db7f275b2bb7a5) | Monte Carlo: the card on its own thread, the next batch shared at once |
| [`bb5ec9c`](https://github.com/thr32423/mb3d-rs/commit/bb5ec9c86a44e1a5d4a4d1767da1bb979d6ce8c6) | Monte Carlo: card and CPU overlap; LED hint; "Calculate on" setting |

## [v0.2.27](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.27) · 12 Oct 2026, 07:44 UTC

| Commit | What it does |
|---|---|
| [`564ea17`](https://github.com/thr32423/mb3d-rs/commit/564ea176dd876e36dce2ba44cc034cd4b51dd8a1) | Version number and release notes of this release |
| [`f74e0a4`](https://github.com/thr32423/mb3d-rs/commit/f74e0a4cfdfa53218031369aac780e809992d241) | Monte Carlo marches on the GPU |

## [v0.2.26](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.26) · 12 Oct 2026, 02:27 UTC

| Commit | What it does |
|---|---|
| [`af93900`](https://github.com/thr32423/mb3d-rs/commit/af93900401b0a564d40cf776f56313857d31c266) | Version number and release notes of this release |
| [`3d5a53d`](https://github.com/thr32423/mb3d-rs/commit/3d5a53d4f8f5d1055f63033bb0a0f0859197c877) | Reflections on the GPU with transparency, cutting planes and doubles |
| [`805a226`](https://github.com/thr32423/mb3d-rs/commit/805a2263eb71b13ecb8e72e79162c55bc4ca0ad7) | SSAO15: per-level sample tables, twice as fast |
| [`07f82d7`](https://github.com/thr32423/mb3d-rs/commit/07f82d76ee9b6fac90982c5b818fa490896906a9) | Custom formulas called directly with the iteration state |
| [`b1ffc60`](https://github.com/thr32423/mb3d-rs/commit/b1ffc60e5ffc077cdbbfb5e260aca6f1e27a5583) | Custom formulas copy only the record fields they use |
| [`f37382c`](https://github.com/thr32423/mb3d-rs/commit/f37382c05993aaa482a699d4d686e11ecfa6b1e9) | Buttons light up blue under the mouse, as in MB3D's Glossy style |

## [v0.2.25](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.25) · 11 Oct 2026, 18:06 UTC

| Commit | What it does |
|---|---|
| [`aed2a31`](https://github.com/thr32423/mb3d-rs/commit/aed2a317ad055229c7830596ee9f62b05460e995) | Old parameter files as MB3D reads them; faster GPU pacing; version 0.2.25 |

## [v0.2.24](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.24) · 11 Oct 2026, 17:07 UTC

| Commit | What it does |
|---|---|
| [`44877a2`](https://github.com/thr32423/mb3d-rs/commit/44877a28151ae5294e96ac031f6297b71c0ad54c) | GPU: more formulas translated; version 0.2.24 |

## [v0.2.23](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.23) · 11 Oct 2026, 15:24 UTC

| Commit | What it does |
|---|---|
| [`f134fb4`](https://github.com/thr32423/mb3d-rs/commit/f134fb43437242c6fb76337863902b3287f6f923) | GPU: double precision for deep zooms; version 0.2.23 |

## [v0.2.22](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.22) · 11 Oct 2026, 13:52 UTC

| Commit | What it does |
|---|---|
| [`cd376c8`](https://github.com/thr32423/mb3d-rs/commit/cd376c88f2372988c7c814000b8c0121536124ad) | GPU: faster reflections; interface fixes; version 0.2.22 |
| [`4e5e70c`](https://github.com/thr32423/mb3d-rs/commit/4e5e70c56eb43e1495c5fadd73240dd62706b993) | Formulas: map functions compiled and on the graphics card |

## [v0.2.21](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.21) · 11 Oct 2026, 11:21 UTC

| Commit | What it does |
|---|---|
| [`b27b0e8`](https://github.com/thr32423/mb3d-rs/commit/b27b0e83c14d163b3908c562a5430a513cf369e5) | Version number and release notes of this release |
| [`bdb3e91`](https://github.com/thr32423/mb3d-rs/commit/bdb3e91caa1f678b2a9059c2c759c898feea8319) | GPU: cutting planes, inside rendering and colour on iteration on the graphics card |

## [v0.2.20](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.20) · 11 Oct 2026, 09:36 UTC

| Commit | What it does |
|---|---|
| [`d927069`](https://github.com/thr32423/mb3d-rs/commit/d9270691557d851876727c71921fd259962207da) | GPU: 4D and interpolation hybrids on the graphics card; version 0.2.20 |

## [v0.2.19](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.19) · 11 Oct 2026, 09:13 UTC

| Commit | What it does |
|---|---|
| [`57f42f2`](https://github.com/thr32423/mb3d-rs/commit/57f42f2d0aa02008d347ae27699f595cc17e1430) | Interface: the window stays responsive while a picture is calculated; version 0.2.19 |

## [v0.2.18](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.18) · 11 Oct 2026, 08:14 UTC

| Commit | What it does |
|---|---|
| [`daed4b6`](https://github.com/thr32423/mb3d-rs/commit/daed4b6663ad9a12a69b2d593b4a124b5ccec0ce) | GPU: resumable marching keeps the desktop responsive; version 0.2.18 |

## [v0.2.17](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.17) · 11 Oct 2026, 04:33 UTC

| Commit | What it does |
|---|---|
| [`6e07c4b`](https://github.com/thr32423/mb3d-rs/commit/6e07c4b884635b1675d36fe8249987470ba6df5e) | GPU: more formulas on the graphics card; version 0.2.17 |
| [`a1dd058`](https://github.com/thr32423/mb3d-rs/commit/a1dd05870ed4ff290d68a80d91faf052a1a10fca) | Interface: user interface scale (Prefs > Visual themes > Interface size, --scale, MB3D_UI_SCALE) |

## [v0.2.16](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.16) · 11 Oct 2026, 03:12 UTC

| Commit | What it does |
|---|---|
| [`080432c`](https://github.com/thr32423/mb3d-rs/commit/080432c1ceb5bf772f9ae40b7c7c0fa0ca48be43) | GPU: reflections on the graphics card; version 0.2.16 |

## [v0.2.15](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.15) · 11 Oct 2026, 02:58 UTC

| Commit | What it does |
|---|---|
| [`a072d7a`](https://github.com/thr32423/mb3d-rs/commit/a072d7a511c82e3d31307c6c725e7de3a2cb8d9c) | Interface: button alignment, glyph layout, memo word wrap, MutaGen tiles; version 0.2.15 |

## [v0.2.14](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.14) · 11 Oct 2026, 01:41 UTC

| Commit | What it does |
|---|---|
| [`2cc440d`](https://github.com/thr32423/mb3d-rs/commit/2cc440d4665e3482c9f8bc164d885ef2a020029d) | GPU: zoom limit by DE kind (analytic / dIFS scenes go much deeper); version 0.2.14 |

## [v0.2.13](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.13) · 11 Oct 2026, 01:20 UTC

| Commit | What it does |
|---|---|
| [`cf96c92`](https://github.com/thr32423/mb3d-rs/commit/cf96c9263a39f4c917875f337e1f86238a636366) | GPU: dIFS hybrids and DE combinations on the graphics card; version 0.2.13 |

## [v0.2.12](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.12) · 10 Oct 2026, 18:29 UTC

| Commit | What it does |
|---|---|
| [`a3e7377`](https://github.com/thr32423/mb3d-rs/commit/a3e73773604aa754bc239561fde8d7836aea84f7) | GPU: screen space ambient occlusion on the graphics card; version 0.2.12 |

## [v0.2.11](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.11) · 10 Oct 2026, 17:58 UTC

| Commit | What it does |
|---|---|
| [`d4aff31`](https://github.com/thr32423/mb3d-rs/commit/d4aff31e73928b52ad180bd7b2a1dd8a2a3c7eab) | GPU: hard shadows and DE ambient occlusion on the graphics card; version 0.2.11 |

## [v0.2.10](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.10) · 10 Oct 2026, 17:10 UTC

| Commit | What it does |
|---|---|
| [`d2c9731`](https://github.com/thr32423/mb3d-rs/commit/d2c9731dfcf8476660ee338e7b002618e92d5ca5) | GPU status LED, preset images, no sticky windows, png par; version 0.2.10 |

## [v0.2.9](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.9) · 10 Oct 2026, 15:47 UTC

| Commit | What it does |
|---|---|
| [`abb142d`](https://github.com/thr32423/mb3d-rs/commit/abb142dc83fadd7d60a836029b1f40c9cb38ca9a) | GPU: .m3f formulas on the graphics card through the lifter; version 0.2.9 |

## [v0.2.8](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.8) · 10 Oct 2026, 13:08 UTC

| Commit | What it does |
|---|---|
| [`cedc9d5`](https://github.com/thr32423/mb3d-rs/commit/cedc9d5b536a0619b6cfb04b08ee78ec83f23d18) | Formula lifter: memory accesses resolved, translated formulas with local memory; version 0.2.8 |

## [v0.2.7](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.7) · 10 Oct 2026, 11:38 UTC

| Commit | What it does |
|---|---|
| [`7e8d014`](https://github.com/thr32423/mb3d-rs/commit/7e8d0149d1eb75984d830466761d50f08cf16de9) | Translated formulas keep registers and flags in locals; version 0.2.7 |

## [v0.2.6](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.6) · 10 Oct 2026, 09:49 UTC

| Commit | What it does |
|---|---|
| [`cba2883`](https://github.com/thr32423/mb3d-rs/commit/cba28832b6431b32bac11028323111f026b43334) | Faster renders: dIFS step limit as in MB3D, faster translated formulas; version 0.2.6 |

## [v0.2.5](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.5) · 10 Oct 2026, 06:28 UTC

| Commit | What it does |
|---|---|
| [`ed94ea2`](https://github.com/thr32423/mb3d-rs/commit/ed94ea27a6be7c2b07199bb844ffd2f704828377) | Scroll bars keep their range; Navigator panel next to the image; mb3d is the command line renderer only; version 0.2.5 |

## [v0.2.4](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.4) · 10 Oct 2026, 06:00 UTC

| Commit | What it does |
|---|---|
| [`ebc72aa`](https://github.com/thr32423/mb3d-rs/commit/ebc72aad572b531e8d32cfadc82aeb8e64ef15de) | GPU on by default with a GPU box in the main window; Monte Carlo shows the lines as they are calculated |
| [`3d2e046`](https://github.com/thr32423/mb3d-rs/commit/3d2e0465fc950f67b3cc5700d80a954346297d7e) | GPU: errors fall back to the CPU instead of ending the program; DXC on Windows; no software renderers; version 0.2.4 |

## [v0.2.3](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.3) · 10 Oct 2026, 05:15 UTC

| Commit | What it does |
|---|---|
| [`7946d81`](https://github.com/thr32423/mb3d-rs/commit/7946d8150c31091c5b3a6646bff2c45d38fc3134) | GPU prototype: Mandelbulb ray marching on the graphics card (wgpu); version 0.2.3 |

## [v0.2.2](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.2) · 10 Oct 2026, 04:09 UTC

| Commit | What it does |
|---|---|
| [`5942fd9`](https://github.com/thr32423/mb3d-rs/commit/5942fd952b564f2189d5cb6561504c11726e1cda) | Release archives packed once, with fixed names; README links to the downloads; version 0.2.2 |

## [v0.2.1](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.1) · 10 Oct 2026, 03:58 UTC

| Commit | What it does |
|---|---|
| [`12ec446`](https://github.com/thr32423/mb3d-rs/commit/12ec446c69ce292bc0203dd9d4b0b9b472b5404b) | Sticky formula, lighting and post processing windows as in MB3D; plain mb3d opens the windows; version 0.2.1 |

## [v0.2.0](https://github.com/thr32423/mb3d-rs/releases/tag/v0.2.0) · 10 Oct 2026, 03:18 UTC

| Commit | What it does |
|---|---|
| [`9ffe24c`](https://github.com/thr32423/mb3d-rs/commit/9ffe24c3169b4837683b1fe9ebbf498212e0bea8) | Replace the web editor with a native Mandelbulb3D desktop program; version 0.2.0 |

## [v0.1.9](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.9) · 9 Oct 2026, 17:33 UTC

| Commit | What it does |
|---|---|
| [`e9c98a8`](https://github.com/thr32423/mb3d-rs/commit/e9c98a86554cb56f8179ecb131ce5a7978284112) | Fix the editor page (syntax error in the start message of 0.1.8); CI checks the page script; version 0.1.9 |

## [v0.1.8](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.8) · 9 Oct 2026, 17:20 UTC

| Commit | What it does |
|---|---|
| [`aa5dece`](https://github.com/thr32423/mb3d-rs/commit/aa5deceb82610b026d1a9ab36de5d5a9f7cb4424) | Editor: no stale editor or Navigator window size from older versions; version 0.1.8 |

## [v0.1.7](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.7) · 9 Oct 2026, 17:04 UTC

| Commit | What it does |
|---|---|
| [`dfee6cb`](https://github.com/thr32423/mb3d-rs/commit/dfee6cb6363a5bac69ed335a37118c5f37994bd1) | Navigator in its own window with its own parameters and preview, as in MB3D |

## [v0.1.6](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.6) · 9 Oct 2026, 16:25 UTC

| Commit | What it does |
|---|---|
| [`16a6309`](https://github.com/thr32423/mb3d-rs/commit/16a6309fb4d303956f27543194447392dc4d33ff) | Windows release archive contains run_gui.cmd (starts the editor and opens the browser); version 0.1.6 |

## [v0.1.5](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.5) · 9 Oct 2026, 16:20 UTC

| Commit | What it does |
|---|---|
| [`acbe17c`](https://github.com/thr32423/mb3d-rs/commit/acbe17c4028b825f455538d3b63263c1e995b92f) | Release archives contain run_gui.sh (uses the mb3d next to it); version 0.1.5 |
| [`5225cc8`](https://github.com/thr32423/mb3d-rs/commit/5225cc8b79613f61b808d8ba87c3f3048026307e) | Editor: run_gui.sh starts the editor and opens it in the browser |
| [`1464316`](https://github.com/thr32423/mb3d-rs/commit/1464316fbbb09590c06a0999971a41b543cb830f) | Version number and release notes of this release |
| [`c8b7824`](https://github.com/thr32423/mb3d-rs/commit/c8b78246f73fb33663f597948a8a837a4bde0b1a) | Editor: pop out windows into their own browser windows |

## [v0.1.4](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.4) · 9 Oct 2026, 15:57 UTC

| Commit | What it does |
|---|---|
| [`9e306c2`](https://github.com/thr32423/mb3d-rs/commit/9e306c2ecce32596569ea41d83538521679a0a90) | CI: actions on Node.js 24 (checkout v7, upload-artifact v7, download-artifact v8, rust-cache 2.9.2); version 0.1.4 |

## [v0.1.3](https://github.com/thr32423/mb3d-rs/releases/tag/v0.1.3) · 9 Oct 2026, 15:50 UTC

| Commit | What it does |
|---|---|
| [`c20b0ad`](https://github.com/thr32423/mb3d-rs/commit/c20b0ad9700afaf53a3fb11844c60ad90b5f6a9e) | Release folder laid out like a Mandelbulb3D installation; History, Meshes, BigRenders |

## v0.1.2 · 9 Oct 2026, 15:06 UTC (tag only, no GitHub release)

| Commit | What it does |
|---|---|
| [`ecd47c5`](https://github.com/thr32423/mb3d-rs/commit/ecd47c529308498d5b917d931d72f605291de1aa) | Release archives contain the original Mandelbulb3D folder |
| [`a825590`](https://github.com/thr32423/mb3d-rs/commit/a82559026feb9e78c307795a00cc14e015e43fae) | Version number and release notes of this release |
| [`5170ca6`](https://github.com/thr32423/mb3d-rs/commit/5170ca6a970ecc8069f2d103d36d03ae9810440d) | Editor: pop out windows into their own browser windows |

## v0.1.1 · 9 Oct 2026, 14:41 UTC (tag only, no GitHub release)

| Commit | What it does |
|---|---|
| [`1464316`](https://github.com/thr32423/mb3d-rs/commit/1464316fbbb09590c06a0999971a41b543cb830f) | Version number and release notes of this release |
| [`c8b7824`](https://github.com/thr32423/mb3d-rs/commit/c8b78246f73fb33663f597948a8a837a4bde0b1a) | Editor: pop out windows into their own browser windows |

## v0.1.0 · 9 Oct 2026, 13:47 UTC (tag only, no GitHub release)

| Commit | What it does |
|---|---|
| [`44cac48`](https://github.com/thr32423/mb3d-rs/commit/44cac4800279eb15dbc1107febfdb08b4b971a46) | README: the editor in MB3D's layout, phase 10 |
| [`a8cd49f`](https://github.com/thr32423/mb3d-rs/commit/a8cd49f816d89cb816a26e8c240ce9af580b893c) | Speed: translate the remaining common x87 ops (fst, fsin/fcos/fsincos, fpatan, fyl2x, f2xm1, fscale, frndint) |
| [`c1fce9f`](https://github.com/thr32423/mb3d-rs/commit/c1fce9f29a1bcda93be452367d04acb61952af55) | Speed: translate SSE2 double ops of custom formulas to native code |
| [`531a5af`](https://github.com/thr32423/mb3d-rs/commit/531a5af5267ce27da61a69c5d9b8806324a61125) | CI (Linux, Windows, macOS, MB3D data), release builds, getting-started guide |
| [`c01ed71`](https://github.com/thr32423/mb3d-rs/commit/c01ed71cc6ac220ca00cb0ddde529b2da36347e4) | Recalculate a selection, double image size, JIT formula editor, map sequences |
| [`07f6a2d`](https://github.com/thr32423/mb3d-rs/commit/07f6a2d41cfcd03653575843380f217604260822) | Editor: MB3D's main window layout and menu structure |
| [`73d76f1`](https://github.com/thr32423/mb3d-rs/commit/73d76f178b5494512e9d5f90963064e75a9c40f9) | Editor server: keep the G-buffer for repainting, MB3D mouse modes, Euler angles, picking, m3i export, ini dirs |
| [`6bd808b`](https://github.com/thr32423/mb3d-rs/commit/6bd808b9d320d6693159e8f673244d94c6a44b31) | Phase 9: README, reflection keys in the editor's Render tab |
| [`a85abe7`](https://github.com/thr32423/mb3d-rs/commit/a85abe79cf5453b47700f069fc9fd95a67c8cb29) | Phase 9: reflections and transparency in normal renders, normals on Z |
| [`6f0fb30`](https://github.com/thr32423/mb3d-rs/commit/6f0fb300c9f0dcb2211ac6f2b262e4162594ecda) | Editor: Monte Carlo tab |
| [`4f7f0ac`](https://github.com/thr32423/mb3d-rs/commit/4f7f0ac1237b8e9b0d7dcc96c04ac246d717e753) | Phase 9: Monte Carlo renderer |
| [`8ab8576`](https://github.com/thr32423/mb3d-rs/commit/8ab8576f1a36216bbd890be10ea5912156c0ef71) | Fix mirrored spherical map lookups; palette alpha |
| [`b0bb8ba`](https://github.com/thr32423/mb3d-rs/commit/b0bb8ba2f1651c5dcd0f5b030f4f34bc7e9d324d) | Phase 9: stereo images |
| [`8a57b86`](https://github.com/thr32423/mb3d-rs/commit/8a57b86daa7e542e127283d6ed0e71ecd312dcf7) | Phase 9: interpolation hybrids, 2D slices, colour on iteration, 4D analytic DE |
| [`72de294`](https://github.com/thr32423/mb3d-rs/commit/72de294745869a9535f348c818b1088dd314f568) | Phase 9: JIT formulas ([SOURCE] Pascal) |
| [`bb62130`](https://github.com/thr32423/mb3d-rs/commit/bb621300b583bdabcc20a2d0da259f40c9f864ef) | Phase 8: README for voxel and mesh export, Aexion C and MutaGen |
| [`571f73f`](https://github.com/thr32423/mb3d-rs/commit/571f73f24db50127b4d6f7a24d12c28b34b7edb5) | Editor: MutaGen and Export tabs |
| [`aacc112`](https://github.com/thr32423/mb3d-rs/commit/aacc1123355751531e9af2d6f10d31626ecce660) | MutaGen: random variations of a parameter set |
| [`d5296f6`](https://github.com/thr32423/mb3d-rs/commit/d5296f6ecbdd18812e9ff7d3e9a5e47bebb93738) | Aexion C, the internal formula #9 |
| [`ce7b171`](https://github.com/thr32423/mb3d-rs/commit/ce7b171aaf848429aac12417200f298da1021b1b) | Phase 8: voxel and mesh export |
| [`90e1d19`](https://github.com/thr32423/mb3d-rs/commit/90e1d19d9efb777f3b3fc089e5c93436073e93e3) | Phase 7: README for animation and batch rendering |
| [`3ca3e59`](https://github.com/thr32423/mb3d-rs/commit/3ca3e59480647998b6f79cb46987e9bc8a33c357) | Phase 7: animation tab in the browser editor |
| [`2ca420b`](https://github.com/thr32423/mb3d-rs/commit/2ca420ba7905cb809a37b2b640aea5dff64c4e23) | Phase 7: mb3d batch - batch rendering of parameter files |
| [`8ebaa7c`](https://github.com/thr32423/mb3d-rs/commit/8ebaa7c6327aaf9a77a38e47d056b31c1dbe39cb) | Phase 7: mb3d animate - render animation frames |
| [`92356e5`](https://github.com/thr32423/mb3d-rs/commit/92356e554c2974d3ab3b38d58e9d7c1ae652216a) | Phase 7: animation files (.m3a read/write, .m3k text format) |
| [`52ee74d`](https://github.com/thr32423/mb3d-rs/commit/52ee74d9a9ba9f46814a01c5a98d7f702bfd856b) | Phase 7: interpolate the light values of animation frames |
| [`b45e1d7`](https://github.com/thr32423/mb3d-rs/commit/b45e1d7886b336e80e44009c69d0e84fbeb1e6ff) | Phase 7: keyframe interpolation (Interpolate2frames, Interpolate3framesBezier) |
| [`e0cddb3`](https://github.com/thr32423/mb3d-rs/commit/e0cddb38ff23d19e6602f0390e57ec3d75be0d47) | Phase 1-6 baseline (imported from mb3d-rs-phase6.tar.gz) |
