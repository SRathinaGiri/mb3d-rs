<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs：适用于 Linux、Windows 和 macOS 的 Mandelbulb3D">

# mb3d-rs — 适用于 Linux、Windows 和 macOS 的 Mandelbulb3D
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · **简体中文** · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*译自英文 README（版本 0.2.84）。两者如有出入，以[英文 README](../../README.md) 为准。*

**下载：[最新版本](https://github.com/thr32423/mb3d-rs/releases/latest)** · [下载](#download) · [更新内容](../UPDATES.md)

mb3d-rs 就是 [Mandelbulb3D](https://github.com/thargor6/mb3d)（MB3D）这款
3D 分形软件，经过重写后可以在如今的计算机上运行：支持
**Linux、Windows 和 macOS**，并且图像在**显卡**上计算，而 MB3D
只能使用处理器。它能打开 MB3D 的参数文件（`.m3p`、`.m3i`、论坛上的文本参数），使用 MB3D 的公式和贴图，生成相同的图像。

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="主窗口"></a>

它包含两个程序：

* **Mandelbulb3D** — 桌面程序，带有 MB3D 自己的各个窗口：主窗口、公式、光照、后期处理、导航器、动画、批处理、大图渲染、蒙特卡洛、体素导出、Bulb Tracer 2、MutaGen
  等等。熟悉 MB3D 的人就会用这个程序；MB3D 的教程同样适用。

  <details><summary>了解更多</summary>

  这些窗口就是 MB3D 自己的窗口——根据 MB3D 原始的窗口定义生成，因此每个按钮、滑块和提示都位于与 MB3D 中相同的位置：
  
  * **主窗口：** 打开和保存参数（`.m3p`、`.m3i`、文本、剪贴板），保存图片（PNG、JPEG、BMP、16 位 Z 缓冲），鼠标模式、旋转按钮、图像尺寸、质量预设、撤销，带有逐步生成的图像和统计信息的*计算 3D*，以及所有页面（计算、内部、信息、切割、Julia、相机、着色、立体）。
  * **公式：** 六个公式槽、MB3D 的公式列表和选项、混合类型、收藏，以及 JIT 公式编辑器。
  * **光照：** 光源 1–6、预设、调色板，物体、环境、动态雾和背景图页面；修改会立即重绘图像，无需重新计算。
  * **后期处理：** 基于 Z 缓冲的法线、硬阴影和环境光阴影，反射和透明，景深。
  * **导航器：** 用键盘和鼠标四处行走、环顾，然后点击*视图到主窗口*。
  * **动画制作器、批处理、大图渲染、蒙特卡洛、体素导出、Bulb Tracer 2**（网格）及网格预览，**高度图生成器、MutaGen、ZBuf16Bit、贴图序列、Ini 目录**——全都与
    MB3D 相同。
  * **集群**（新增，位于工具选项卡）：让您的多台计算机一起渲染，参见[在多台计算机上渲染](#rendering-on-several-computers-the-swarm)。

  </details>
* **mb3d** — 用于脚本和批处理任务的命令行渲染器：图片、动画、批处理列表、蒙特卡洛渲染、体素和网格导出，全都无需窗口。

<a id="is-it-safe-to-run-check-it-yourself"></a>

## 运行它安全吗？自己检查一下

您不必只听我们的一面之词。[scanrepo.dev](https://www.scanrepo.dev)
是一项独立服务，它会检查 GitHub 仓库中是否有恶意软件的迹象：启动其他程序、连接互联网、隐藏自身行为，或绕过编程语言安全检查的代码。任何人都可以随时对本仓库运行它：

**[▶ 在 scanrepo.dev 上扫描 mb3d-rs](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
（点击 *Rescan* 可获得当前代码的最新结果）

版本 0.2.85 的评级为**安全：100 分中 5 分**（扫描于 2026 年 10 月 30 日，扫描器引擎第 10 版的规则）。它唯一的警告针对的是启动程序在运行时为提速而编译的公式的那一行：运行新生成的机器码无法用 Rust 的受检形式编写，而程序只在这一行和通往 Intel 降噪滤镜的桥接代码中允许这样做；其余是一条提示性说明。（更早的：0.2.66 在旧版引擎下的评级为 100 分中 0 分，另请参阅：
[0.2.44 的结果](../img/scan_0.2.44.png)。）曾有一个较早的版本被标记，我们随即修改了代码（[issue #4](https://github.com/thr32423/mb3d-rs/issues/4)）；现在的代码就是按保持这一结果的要求编写的。程序中没有任何连接互联网的代码；它们唯一请求系统打开的其他程序是您的文件管理器，即在您点击按钮打开某个文件夹时。

扫描只查找已知的警示迹象，并不是保证。下面这些可以让您自己进一步核实：

* **全部源代码都在这里。** 下载文件由 GitHub 自己的构建服务器按照您可以在[发布工作流](.github/workflows/release.yml)中读到的步骤从源代码构建，而不是在任何人的个人计算机上构建。
* **每个版本都列出了校验和**（`SHA256SUMS`），因此您可以确认下载的文件与发布的文件完全一致；下文的一行安装命令会替您完成这一步。从 0.2.45 起的版本还附带 GitHub 的构建证明：`gh attestation verify
  FILE --repo thr32423/mb3d-rs` 会显示某个下载文件是从哪个提交
  构建的。

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## 与 Mandelbulb3D 相比有哪些新内容

* **由显卡计算图像**——图像本身、阴影、环境光遮蔽、反射以及蒙特卡洛渲染。在一台普通计算机上，这比单用处理器快好几倍；参见下文的[速度](#speed)。
* **自动模式**：显卡和处理器共同计算每一张图像，而且从不等待——如果显卡还没准备好，处理器照常开始，显卡随后加入。
* **标题栏中的指示灯**显示上一张图像是在哪里计算的。
* **界面大小**可在 75% 到 200% 之间调整，适合高分辨率屏幕。

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="150% 下的视觉主题"></a>

  <details><summary>了解更多</summary>

  **首选项 ▸ 主题 ▸ 界面大小**可以让所有窗口及其文字变大或变小（75% 到 200%），适合高分辨率屏幕；布局与 MB3D 完全一致。
  
  | 100 % | 150 % |
  |---|---|
  | ![100% 下的视觉主题](../img/themes_100.png) | ![150% 下的视觉主题](../img/themes_150.png) |

  </details>
* 窗口有**八种外观**，每一种都是可以复制和修改的主题文件：两种重现 MB3D 的外观，六种是新的。

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Midnight 主题"></a>

  <details><summary>了解更多</summary>

  **首选项 ▸ 主题**提供八种外观，每种都是 `Themes`
  文件夹中的一个文件：*Glossy*（默认的深色外观）和 *Windows* 重现了 MB3D 自己的两种外观，其余六种是新的。点击图片可查看原尺寸。
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**：MB3D 的默认外观 | **Windows**：MB3D 的"Themes off" |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**：较浅的中性灰，钢蓝色点缀 | **Daylight**：暖浅灰，青绿色点缀 |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**：深海军蓝，蓝色点缀 | **Ember**：暖炭灰，橙色点缀 |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="High contrast"></a> |
  | **Forest**：深绿灰，绿色点缀 | **High contrast**：黑底、白字、黄色高亮 |
  
  主题就是程序旁边 `Themes` 文件夹中的一个小文本文件；复制一个，修改其中的颜色，下次启动时它就会出现在列表中。

  </details>
* **在所有系统上，崩溃都会留下记录**：程序的 panic（或导致其退出的错误）会追加写入 `mb3d_crash.txt`，位于程序旁边或其数据文件夹中（以前只在 Windows 上有）。提交错误报告时请附上这个文件。
* **计算机上的所有显卡都参与计算**，无需任何设置：找到的每块显卡都从与第一块显卡和处理器相同的池中领取图像的行带，因此更快的显卡自然领取更多；阴影、遮蔽、蒙特卡洛光线和反射也以同样方式分担，动画则每块显卡同时渲染一帧。较慢的显卡（与独立显卡并存的集成显卡）尽力而为，绝不会拖住一张图像：一旦空闲的行分完，空闲的显卡和处理器就会接手它尚未交付的行。出错的显卡会被排除，其余的继续工作。
  `mb3d --list-cards` 显示找到了哪些显卡。（已用同一块显卡打开两次、以及用软件 Vulkan 驱动充当第二块显卡进行过测试；尚未在装有两块真实显卡的计算机上测试。）
* **未完成的图像是一个 `.m3i` 文件。** 被停止、崩溃或被强制结束的渲染已经计算出的内容会保存在缓存中（未完成的渲染窗口，`mb3d resume`）；对于图片，这个文件现在随时都是一个 `.m3i` 图像文件，即渲染文件夹中的 `image.m3i`，尚未计算的行显示为背景。您可以随时把它复制出来，或在未完成的渲染窗口中按 *Save m3i*（命令行上为 `mb3d
  resume --m3i KEY file.m3i`；两者也都能把旧版本的
  `data.bin` 写成 `.m3i`），然后在本程序或
  Mandelbulb3D 中打开它，无需转换。（蒙特卡洛渲染保留自己的记录，某一遍的 `.m3c` 用 `--m3c` 写出。）
* **窗口快捷键**：在任何窗口中，一个组合键就能把主窗口、导航器、动画、公式、光照、后期处理或蒙特卡洛窗口调到最前面（Ctrl+1 到 Ctrl+7），
  Ctrl+W 关闭当前所在的窗口（在 macOS 上为 Cmd）。可在*首选项 > 快捷键*中自行设置：点击一个输入框并按下按键，Backspace 删除快捷键。
* **使用您的语言的界面**：窗口中的每一段文字都来自语言文件；随附十二种语言，程序还能为新语言写出文件供您翻译。

  <details><summary>了解更多</summary>

  主窗口标题栏右端的语言标记显示界面语言（`EN`）；点击它会打开语言窗口。一种语言就是程序旁边 `Languages` 文件夹中的一个文本文件。随附：English、Deutsch、Français、Italiano、Nederlands、Polski、
  Kaszëbsczi、हिन्दी、日本語、한국어、简体中文和繁體中文（每种都带有国旗；东亚语言附带 Noto Sans CJK 的子集以显示其文字，印地语附带 Noto Sans Devanagari，其连写字母和元音符号由 rustybuzz 文字排版引擎排版）。这些翻译由机器完成，只检查了结构，未经母语者校对：欢迎指正，每处都只是文件中的一行。如何翻译：输入您的语言名称并按*创建语言文件*；程序会写出 `Languages/<name>.lng`，其中每段文字都是英文，每行一段（`Button2.Caption = Calculate
  3D`）。用文本编辑器翻译 `=` 右侧的文字，然后按
  *重新加载*：窗口会立即更新，无需重启。省略的行保持英文。*保存并退出*会在下次启动时继续使用该语言。

  </details>
* **Linux、Windows 和 macOS**（Intel 和 Apple Silicon）。
* **一个命令行程序**，用于所有不需要窗口的工作。
* **一次性渲染超大图片**：15000 x 20000 像素乃至更大，不会耗尽内存；景深使用全部处理器核心（在 24 线程上比 0.2.45 之前快 11 倍，图像相同）。
* **16 位图片：** 每种颜色有 65,536 级而不是 256 级的 PNG 文件，后期编辑时不会出现色带。

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="16 位图片"></a>

  <details><summary>了解更多</summary>

  MB3D 保存的是 8 位图片：每种颜色 256 级。用来观看绰绰有余，但图片在之后被编辑时——亮度、对比度、曲线——背景或雾气这类平滑区域就会裂成明显的色带。**保存图片 ▸ PNG** 提供了第二种文件类型 **PNG 16 位彩色**：即程序计算出的颜色，在被舍入为 256 级之前的值，包含所有阴影、反射和景深。文件看起来一样，编辑时依然平滑。
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="大幅提高对比度后的背景渐变：8 位有阶梯，16 位平滑"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="带有“PNG 16 位彩色”的保存对话框"></a>

  </details>
* **降噪滤镜**，适用于蒙特卡洛和所有其他图片，采用 GIMP 中熟知的 DCT 降噪；对于蒙特卡洛渲染还有一个神经网络滤镜，即 Intel Open Image Denoise。

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="降噪滤镜"></a>

  <details><summary>了解更多</summary>

  **降噪滤镜**去除图片中的颗粒：即 GIMP 中
  *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT* 的 **DCT 降噪**，它是
  GIMP 的各种滤镜中最适合这类图片的（参见下文的致谢）。
  
  * **蒙特卡洛窗口 ▸ -噪点**（MB3D 隐藏了的一个按钮）：对显示的图片和保存的图片进行过滤，强度随每像素光线数而定。渲染本身继续不经过滤地进行，下一次更新又会显示未过滤的图像。
  * **后期处理 ▸ 降噪滤镜**：*立即过滤*过滤主窗口中的图片；*自动过滤新图像*过滤每一张新图片。*强度*是要去除的噪点强度（4 为轻度；数值越高，去除的颗粒越多，细节也损失越多）。
  
  蒙特卡洛图片的颗粒最多。与每像素 512 条光线渲染的图片相比，在全部 13 项测试中（四个场景，4 到 32 条光线），过滤后的图片都比未过滤的更接近：4 条光线时最多接近 23%，
  8 条光线时 5% 到 10%，32 条光线时 1% 到 3%，此时已几乎没有颗粒。
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="每像素 4 条光线的蒙特卡洛图片、加了降噪滤镜的同一图片，以及 512 条光线的图片"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="后期处理：降噪滤镜部分"></a>

  **神经网络降噪滤镜**是
  [Intel Open Image Denoise](https://www.openimagedenoise.org)，即
  Blender 的 Cycles 和许多其他路径追踪器所用的滤镜，经过渲染图片的训练。它过滤的是渲染中光线的平均值（在曝光和饱和度之前），因此图片的亮度保持不变。
  Intel 的库随程序附带（`oidn` 文件夹；在 .deb
  和 .rpm 中为 `/usr/lib/mb3d-rs/oidn`）；可以用 `MB3D_OIDN` 指定另一个库。

  * **蒙特卡洛窗口 ▸ -噪点AI**：与 *-噪点*一样，过滤显示的图片和保存的图片；渲染继续不经过滤地进行。
  * **命令行**：`mb3d montecarlo scene.m3p --rays 8 --denoise-ai`。
  * 在处理器上处理一张 640 x 480 的图片需要 0.2 到 0.6 秒。

  与 512 条光线的图片之间的差异（所有像素的平均值，越低越好）：

  | 场景 | 4 条光线 | DCT 滤镜 | 神经网络滤镜 | 8 条光线 | DCT 滤镜 | 神经网络滤镜 |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  在四个场景中的三个里，神经网络滤镜最接近；在 Julia
  集的细密颗粒上，DCT 滤镜稍微接近一些。它在玻璃和镜面上效果最明显，因为那里光线少、颗粒最多：

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="每像素 4 条光线的蒙特卡洛图片，分别为加 DCT 滤镜、加神经网络滤镜，以及 512 条光线的图片"></a>

  </details>
* 图片和动画可使用**透明背景**。

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="透明背景"></a>

  <details><summary>了解更多</summary>

  图片和动画帧可以保存为**透明背景**，以便把分形叠加到另一张图片上或放进视频编辑软件：**保存图片 ▸ PNG ▸ PNG 透明背景**，在动画制作器中则是**透明背景**（PNG 帧）。开启抗锯齿时（观看比例 1:2 或更高，或 `--aa`），边缘是柔和的，并且不带旧背景颜色的毛边。用于合成的深度图早已具备（*同时保存 Z-buffer*，`--depth`）。
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="一张带背景的图片，以及显示在棋盘格上的透明背景图片"></a>

  </details>
* **动画视频：** 直接由程序生成 AVI 文件，或者生成用于制作小体积 MP4 的 ffmpeg 命令。

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="动画视频"></a>

  <details><summary>了解更多</summary>

  在动画制作器中，**同时生成视频 (AVI)** 会在帧渲染完成时按您设置的**每秒帧数**把它们合成为视频：输出文件夹中的 `<project name>.avi`，与帧放在一起，帧会保留。它可以在 VLC 和 mpv 中播放，也能在视频编辑软件（Kdenlive、
  Shotcut、DaVinci Resolve）中打开。每一帧都存为完整的图片，因此文件很大：适合编辑和保存，不太适合分享。
  
  若要一个便于分享的小视频，**FFmpeg 命令**会把生成 MP4
  （若带透明背景则为 WebM）的命令放到剪贴板和消息中。安装 [ffmpeg](https://ffmpeg.org)，在帧完成后把命令粘贴到终端中，它就会生成视频；本程序自身不会启动任何其他程序。
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="带有“同时生成视频 (AVI)”、“每秒帧数”和“FFmpeg 命令”的动画制作器"></a>

  </details>
* 打开对话框中可**预览参数文件**。

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="打开参数时的预览"></a>

  <details><summary>了解更多</summary>

  **打开 ▸ 打开 m3p** 和 **打开 m3i** 会在打开之前显示所点击的参数文件的小图，以及它的尺寸和公式；方向键可在列表中移动并随之预览。图片在后台由处理器计算，因此对话框保持响应，每个文件的预览在程序运行期间都会保留。
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="带有参数文件预览的打开对话框"></a>

  </details>
* **导航器：精确数值和重置视图。** 点击一个数值并输入；回到导航器开始时的视图。

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="导航器：精确数值和重置视图"></a>

  <details><summary>了解更多</summary>

  * 在导航器的面板中**点击一个数值**——Julia 值、公式值、4D 旋转、其他数值——然后输入精确的数字，与
    MB3D 中一样（角度以度为单位）。之后滑块会从该值继续。
  * 当缩放步长太粗时，**点击缩放值**并输入（这是
    MB3D 问题列表中的一个愿望）；输入的缩放值会保持不变（*固定缩放和步长*）。
  * **重置视图**回到导航器从主窗口取得的位置、方向和缩放；调整过的数值保持不变。
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="显示公式值以及所点击数值的输入框的导航器"></a>

  </details>
* **新公式 SphereTree：** 由相互接触的球体组成的树，分支越来越小，作者 Tom Lowe。

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="新公式：SphereTree"></a>

  <details><summary>了解更多</summary>

  **SphereTree** 是 Mandelbulb3D 没有的公式：一棵由彼此接触的球体组成的树，不断分出越来越小的球体，像泡沫或珊瑚。发明了 Mandelbox 的 Tom Lowe 于 2020 年发表了它（参见下文的致谢）。它在公式列表中与 dIFS 公式放在一起，有四项设置：
  
  * **PackRatio**（0 到 1.22）：球体的大小；为 1 时它们刚好接触，大于 1 时外层的球体会断开。
  * **Levels**：树的深度（大多数图片 6 就足够了）。
  * **Rotated**：为 1 时每一层转动十二分之一圈，整棵树变成一个由球体簇组成的圆球。
  * **Colour scale**：树的每个分支都有自己的颜色。在光照的颜色设置中选择*第二选择*（轨道陷阱）即可看到它们；此设置把它们分布到颜色渐变上。
  
  它可以单独使用，也可以放在 dIFS 变换之后（第三张图片用
  *tilingbrickIFS* 将其重复），并且既可以在显卡上计算，也可以在处理器上计算。把迭代次数设为 1：该公式一次就计算出整棵树。含有 SphereTree 的参数只能在本程序中打开，不能在 Mandelbulb3D 中打开。
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="蒙特卡洛渲染器中黄昏时分的 SphereTree 球（Rotated）"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="SphereTree 球体泡沫的特写"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="用 tilingbrickIFS 重复的 SphereTree：一群球体树"></a> |
  |:---:|:---:|

  </details>
* **新公式 DomeTree：** 一个穹顶长出一圈较小的穹顶，每个穹顶又有自己的一圈，作者 Tom Lowe。

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="新公式：DomeTree"></a>

  <details><summary>了解更多</summary>

  **DomeTree** 是 Tom Lowe 继 SphereTree 之后的第二棵树（参见致谢）：一个球的一侧长出一圈穹顶，每个穹顶又长出自己更小的一圈，一层接一层，由圆反演构成。它在公式列表中与 dIFS 公式放在一起，有五项设置：

  * **Levels**：树的深度（16 就足够了）。
  * **Children**：每一圈中的穹顶数，4 到 10。
  * **Bend angle**（0.3 到 1.3 效果最好）：穹顶隆起的程度；数值低时是扁平的圆盘，高时是圆穹顶。超过约 1.3 时表面会变得粗糙有颗粒。
  * **Colour scale**：每个穹顶都有自己的颜色。在光照的颜色设置中选择*第二选择*（轨道陷阱），并使用较宽的颜色范围（例如 0 到 100）即可看到它们。
  * **Start curved**：1（默认）从球开始，0 从一个长出第一圈的平坦地面开始。

  使用较小的*光线步长乘数*（0.2；9 或 10 个子穹顶时用 0.05）：该形状的距离估计比较粗略，与 TGlad 自己的 Fragmentarium 文件中一样。与
  SphereTree 一样，它在显卡和处理器上都能运行，只需要
  1 次迭代，含有它的参数只能在本程序中打开。

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="蒙特卡洛渲染器中有 8 个子穹顶、按穹顶着色的 DomeTree"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree，5 个子穹顶，弯曲角 1.2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree，6 个子穹顶，弯曲角 0.5：扁平的穹顶"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree，10 个子穹顶：穹顶之间的花边和一个大球"></a> |
  |:---:|:---:|:---:|

  </details>
* **新公式 UlamSpiral：** 三维的乌拉姆质数螺旋，呈现为城市、天际线、球体或方块。

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="新公式：UlamSpiral"></a>

  <details><summary>了解更多</summary>

  1963 年，数学家斯塔尼斯瓦夫·乌拉姆（Stanisław Ulam）在一次会议上听一篇"又长又非常无聊的报告"，在一张方格纸上随手涂画。他最初想摆几道国际象棋题；结果却把方格按螺旋编号为
  1、2、3……，1 在中间，再把质数圈出来。质数排列在对角线上，好像遵循某种规律，而当计算机画出数以百万计的质数时，它们依然如此（乌拉姆在洛斯阿拉莫斯的
  MANIAC II 上试了 100,000 个；马丁·加德纳 1964 年把这张图放上了《科学美国人》的封面）。直到今天也没有人完全解释清楚。
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="乌拉姆螺旋：方形螺旋上数字 1 到 78,961 中的质数，角落里画出了螺旋的起始部分"></a>
  
  **UlamSpiral** 在三维中构建这个螺旋，每个数字一个形状，有四种形式（*Mode*）：
  
  * **0，质数塔**：每个数字一块板，每个质数上一座塔。质数的对角线变成一排排塔。
  * **1，因数天际线**：每个数字一根柱子，因数越多越高；质数是柱子之间低矮的街道。
  * **2，因数球体**：每个数字一个球，因数越多越大（质数是小的那些）。
  * **3，质数方块**：数字绕立方体外壳而不是正方形排列，每个质数上一个立方体。
  
  *Rings* 设置螺旋的大小（60 圈包含不超过
  14,641 的数字），*Height* 设置塔或柱的高度，*Gap* 设置它们之间的间距。在颜色设置中选择*第二选择*（轨道陷阱）时，颜色随因数个数变化，因此质数有自己独特的颜色。与 SphereTree 一样，它在显卡和处理器上都能运行，可放在 dIFS 变换之后，只需要 1 次迭代，含有它的参数只能在本程序中打开。
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral 质数塔：质数的对角线排列成一座城市，在蒙特卡洛渲染器中带有雾、硬阴影和景深"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral，因数天际线"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral，因数球体"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral，立方体外壳上的质数方块"></a> |
  |:---:|:---:|:---:|

  </details>
* **停止的渲染可从原处继续。** 渲染停止时，已计算的内容会被保留；程序关闭或崩溃、计算机断电时也一样（每 5 分钟保存到磁盘一次）：计算 3D、蒙特卡洛（包括一遍进行到一半时）、正在进行的动画中的一帧或批处理中的一张图片、体素导出、Bulb Tracer 网格、大图渲染的分块，以及集群渲染（在创建任务的计算机上停止会保留所有已完成的部分，而每台计算机保留它正在处理的部分）。相同参数的计算 3D 和蒙特卡洛会询问是否继续，其他的则自动继续；**恢复**按钮（打开选项卡）列出未完成的渲染，可以继续或删除它们。在命令行上，同一条命令会自动继续（包括 `--tiles`、`montecarlo`、
  `animate`、`batch`、`voxel` 和 `mesh` 中的图片）；`--fresh` 重新开始，`mb3d
  resume` 列出它们。继续完成的渲染与一次性
  计算出的完全相同。
* **在您的多台计算机之间共享渲染**，即"集群"：能访问同一共享文件夹的计算机一起渲染一张图片、一次大图渲染、一张蒙特卡洛图片或一段动画。参见
  [在多台计算机上渲染](#rendering-on-several-computers-the-swarm)。
* **新公式 PseudoKleinian：** 由大厅、拱门和球体构成的"克莱因建筑"，作为一个带有精确距离的独立公式。

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="新公式：PseudoKleinian"></a>

  <details><summary>了解更多</summary>

  被称为"克莱因建筑"的大厅、拱门、柱子和嵌套球体是最受喜爱的 3D 分形图片之一。在 MB3D 中，只能用几个公式的混合并仔细匹配设置才能做出来，这是 Theli-at 在 2011 年找到的方法；随后 knighty 把同样的东西写成了一个带有自己距离估计的独立公式，这就是
  **PseudoKleinian**。每一层把点折叠进一个盒子，在球中反演，再加上一个偏移；距离在折叠之后精确地测量到一个地平面，因此墙面干净利落，图片渲染得很快。设置沿用 knighty 的：*Levels*（8）、*Size*（反演球）、*CSize X/Y/Z*（盒子；他的 0.924、0.908、0.924 给出经典的大厅，微小的改动会产生新的建筑）、*Julia X/Y/Z*
  （每层的偏移）、*Shape*（0 为地平面，1 为穿过 *Offset* 的厚度为 *Thickness* 的柱面-平面，2 为圆角盒）、*DE offset*（把表面挖空）和 *Colour scale*（配合*第二选择*，即轨道陷阱：最小轨道半径，经典的克莱因条纹）。
  
  这个结构充满整个空间，因此您是从内部观看它：开始时把视图中心放在原点附近，相机向后退一两个单位，然后用导航器在大厅中穿行。在蒙特卡洛渲染器中，室内几乎得不到天空光，因此要在大厅里放一盏定位光源（第一张图片），或者用切割平面把结构剖开，从上方往里看（第二张）。它在显卡和处理器上都能运行，需要 1 次迭代，可放在 dIFS 变换之后，也能像其他 dIFS 公式一样用于 DE 组合（不过在充满空间的结构内部，这些变化不大），含有它的参数只能在本程序中打开。
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="从 knighty 的视点看 PseudoKleinian：由内部一盏灯照亮的拱门和嵌套球体大厅，在蒙特卡洛渲染器中带有雾、硬阴影和景深"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="用切割平面剖开并从上方观看的 PseudoKleinian：球体的晶格及其间的空腔"></a>

  </details>
* **一步生成立体图像对**：两只眼睛的图像都会渲染出来并并排显示，用于平行视或交叉视观看。由 Rathinagiri S
  （[SRathinaGiri](https://github.com/SRathinaGiri)）贡献。

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Dainbramage 的 Hydra 的立体图像对，并排放置用于平行视观看"></a>

  <details><summary>了解更多</summary>

  **立体 ▸ 渲染立体图像对**先后渲染左眼和右眼图像，使用该选项卡的立体设置，并在主窗口中并排显示。它们按平行视排列（左图在左边）；**交叉眼预览**把它们对调，用于交叉视观看。这只是预览：之后若不重新渲染就无法修改光照和颜色，而且它不能与分块一起使用。单眼图像的按钮仍然保留。
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Dainbramage 的 Hydra 的立体图像对，并排放置用于平行视观看"></a>

  </details>
* **整张图片的剩余时间**：图片下方的进度条在整个计算过程中只填满一次，旁边显示到图片完成为止的剩余时间，包括阴影、遮蔽和反射。

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="进度条和总剩余时间"></a>

  <details><summary>了解更多</summary>

  MB3D 的进度条在每个阶段都会重新开始。这里它显示的是整个计算：主渲染，然后是硬阴影和环境光遮蔽、反射以及景深。主渲染之后各阶段需要多长时间，是根据场景的要求以及主渲染的进展估算的；这个估算是在 366 个场景的约 1400 次渲染上拟合出来的。再次计算的图片会以上一次所用的时间为预期。
  
  使用新公式在显卡上计算的第一张图片要等待着色器编译：这时状态行会显示 *正在编译着色器* 和剩余时间，该时间按处理器的速度计算，并且在着色器能及时就绪时，把显卡的加入也考虑在内。详见
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar)。
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="计算硬阴影和环境光遮蔽期间的主窗口，显示总剩余时间"></a>

  </details>
* 标题栏时钟图标后面的 **History**：您计算过的每一张图片及其缩略图，您打开或保存过的每一个文件，可以显示为列表、行或任意大小的图块，按天分组或连成一串；可以搜索、排序、只显示有差异的图片，双击即可载入任意条目。包含数十万条目的历史记录也能瞬间打开。
* 在硬阴影和反射之后，**光照依然实时生效**：移动光源或修改其颜色，图片会立即改变，与 Mandelbulb3D 中一样。

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="修改光照后的主窗口和光照窗口"></a>

  <details><summary>了解更多</summary>

  先做普通渲染，然后尝试不同的光源位置和颜色：在光照窗口中的每次修改都会立即重绘，阴影保持计算时的样子，与 Mandelbulb3D 中一样。修改光源的颜色或强度从不会重新计算阴影。移动过的光源的阴影、反射和景深可以用后期处理窗口中的按钮重新加上，与 Mandelbulb3D 中一样。
  
  修改光照之后，已停止的计算仍然可以继续。一种工作方法：wiki 中的[光照工作流程](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-zh-CN)。详见
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field)。
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="修改光照后的主窗口和光照窗口：图片已经变亮"></a>

  </details>

<a id="download"></a>

## 下载

最新版本的程序，下载即可运行，无需安装：

| 系统 | 下载 |
|---|---|
| Windows（64 位） | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux（64 位） | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian、Ubuntu、Mint（像其他软件一样安装） | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora、RHEL、Rocky、openSUSE（像其他软件一样安装） | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS，Apple 芯片（M1 及更新） | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS，Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

解压后，在解压出的文件夹中启动 **Mandelbulb3D**（Windows 上为 `Mandelbulb3D.exe`）。文件夹的结构与 MB3D 的安装目录相同，带有 MB3D 的公式、贴图和 80 个示例参数文件。

* 如果 Windows 显示"Windows 已保护你的电脑"（"Windows protected your PC"），请点击*更多信息*（*More info*）▸
  *仍要运行*（*Run anyway*）。
* macOS 会阻止未签名的下载程序：在解压出的文件夹中运行一次
  `xattr -dr com.apple.quarantine .`。

**Linux 和 macOS 的另一种方式：** 不必手动下载，这一行命令会把最新版本下载到 `~/mb3d-rs`（以后再次运行即可更新；脚本 [download.sh](../../download.sh) 也在程序文件夹中）：

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

它会在解压前校验下载的文件，把 **Mandelbulb3D** 放进应用程序菜单（Linux），并创建可在任何文件夹中输入的 `Mandelbulb3D` 和 `mb3d`
命令。`sh ~/mb3d-rs/download.sh
--uninstall` 会再次移除菜单项和命令；您的文件
保持不变。

Linux 版程序可在约 2018 年以后的发行版上运行（Ubuntu 18.04、
Debian 10、Rocky/RHEL 8 及更新版本）。

**像其他软件一样安装：** 在 Debian、Ubuntu 和 Mint 上，打开上面的
`.deb` 文件（或 `sudo apt install ./mb3d-rs-linux-x64.deb`）；在
Fedora、RHEL 和 Rocky 上打开 `.rpm` 文件（`sudo dnf install
./mb3d-rs-linux-x64.rpm`，在 openSUSE 上为 `sudo zypper install
./mb3d-rs-linux-x64.rpm`）。
之后 Mandelbulb3D 会出现在应用程序菜单中，`mb3d` 可在任何文件夹中使用；您的设置和 History 保存在 `~/.config/mb3d-rs` 和
`~/.local/share/mb3d-rs` 中。

[入门指南](../../GETTING_STARTED.md)带您完成最初的几步；旧版本和发布说明在
[releases](https://github.com/thr32423/mb3d-rs/releases) 页面上。

<a id="guides-and-help"></a>

## 指南与帮助

分步指南在 [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-zh-CN) 中：

* [安装](https://github.com/thr32423/mb3d-rs/wiki/Installing-zh-CN)：Windows、macOS 和 Linux，更新，您的文件在哪里
* [光照工作流程](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-zh-CN)：先做普通渲染，再尝试光源、阴影和反射
* [集群设置](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-zh-CN)：通过共享文件夹在多台计算机上渲染
* [故障排除与常见问题](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-zh-CN)：当某些功能未按预期工作时

每个版本及其改动：[完整列表](../UPDATES.md)。

<a id="rendering-on-several-computers-the-swarm"></a>

## 在多台计算机上渲染（集群）

能访问同一共享文件夹的计算机可以一起渲染：分块的图片、大图渲染、蒙特卡洛图片、动画或批处理。每台计算机领取别人还没领取的部分，因此快的计算机领取得更多，而启动渲染的计算机负责把结果拼合起来。共享由文件夹完成：程序不打开任何网络连接，也不与任何人通信，因此它与您的文件共享一样安全。

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D 和集群窗口（用工具选项卡上的集群按钮打开），两台计算机正在分块渲染一张图片：共享文件夹、共享本机、各台计算机及其正在做的事、任务，以及显示已渲染分块、进行中分块和缺失分块的预览"></a>

<a id="setting-it-up"></a>

### 设置

分步说明及检查要点：wiki 中的[集群设置](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-zh-CN)。

1. **一个每台计算机都能写入的文件夹：** NAS 上的共享、Windows
   共享、Samba 共享或 NFS。在 Linux 上把它挂载为文件夹，例如
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`；只在文件管理器中打开的共享（`smb://...`）不是程序能使用的文件夹。
2. **每台计算机上都是同一版本的 mb3d-rs。** 版本不同的计算机会被列出，但不参与工作。
3. **在 Mandelbulb3D 中：** 工具选项卡上的**集群**按钮。选择共享文件夹（每台计算机上是同一个共享；路径可以不同），给这台计算机起一个在列表中显示的名称，并勾选**共享本机**。

就这些。状态行会说明正在发生什么，例如*正在与 omarchy 共享：渲染在 2 台计算机之间分配*。如果集群中只有这一台计算机，所有内容照常在本机渲染。

<a id="what-is-shared"></a>

### 共享哪些内容

勾选*共享本机*期间，这台计算机会渲染其他计算机放进文件夹的部分，而只要集群中有另一台计算机，它自己的渲染就会交给集群：

* **计算 3D**（以及批处理窗口）——图片分成 N x M 块（在集群窗口中设置），在本机拼合，并具备常规计算的所有功能：修改光照、后期处理、保存为 m3i（以及它的内存，见下文）。
* **大图渲染**——分块到达时逐个保存；整张图片从不放在内存中，因此这是渲染
  100000 x 100000 像素及更大图片的方法。
* **蒙特卡洛**——与在一台计算机上一样：每台计算机不断向自己的块中添加渲染遍数，直到您按下停止；图片随所有人的光线逐渐变干净，停止时对各块取平均。在窗口的批处理模式中，各块合计达到批处理的光线数时停止。
* **动画制作器**——逐帧进行。

窗口中列出各台计算机及其核心数、显卡和各自正在做的事，各项任务及其进度，以及最新任务的预览：已渲染的分块、正在进行的分块（带进度条），以及仍然缺失的分块。

<a id="on-the-command-line"></a>

### 在命令行上

在每台要帮忙的计算机上，针对该文件夹启动一个 worker：

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

然后在任意一台计算机上，用 `--swarm` 和同一个文件夹进行渲染：

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` 显示集群中有哪些计算机、每台计算机在做什么以及任务进行到哪里。命令行上的蒙特卡洛图片会被分成若干块，每块有自己的随机光线：4 块、每像素 64 条光线，就得到 `--rays 256` 所要求的 256 条。

<a id="good-to-know"></a>

### 须知

* **如果其他计算机一直空闲**，在本机渲染时它们的任务列表是空的：用 Samba 提供该文件夹、同时自己也直接使用它的那台计算机会让其他计算机保留文件夹的旧列表。在
  `/etc/samba/smb.conf` 中关闭这种缓存：在 `[global]` 中设置 `smb3 directory leases = no`，在该共享的配置段中设置 `oplocks = no` 和 `level2 oplocks = no`，然后运行 `sudo systemctl restart smb`，并在其他计算机上重新挂载该共享。
* **内存：** 在本程序中，一张图片每像素大约占用 20 字节（其图像缓冲区），在主窗口中再加 10 字节，然后再加上两个阶段中较大的那个：带阴影和遮蔽的主计算（每像素 16 字节，显卡另需 1.2 GB），或者绘制（每像素 3 字节，有景深时再加 20 字节，外加一条反射的行带：最多 3.5 GB，空闲内存少时会更少，因为行带的大小是该遍开始时空闲内存的一半）。一张 12000 x 6000 的图片：6 GB。当一张图片需要的内存超过计算机的空闲内存时（向 Linux、
  Windows 和 macOS 查询），计算 3D 会说明它需要多少、还有多少空闲，并在开始前询问：否则计算会使用交换空间，程序可能每次都会长达数分钟没有响应。大图渲染（单独使用或配合集群）就是解决办法：它的分块逐个保存，整张图片从不放在内存中。配合集群使用的计算 3D 会像常规计算一样把图片保存在本机（但不包括主计算阶段，那由负责分块的计算机用各自分块的内存完成）；集群中的计算机会把超出自己内存的分块留给其他计算机。
* **安全：** 计算机只渲染任务所要求的内容，并且只写入任务的文件夹；来自任务的公式代码与所有公式一样，在公式模拟器中运行。
* **速度：** 分块以完整的图像缓冲区形式传递，对于超大图片是几百兆字节，因此快速的共享物有所值。延迟复制的同步服务也可以用，只是更慢。
* **离开的计算机：** 一台计算机如果半分钟没有消息，它正在渲染的部分就会被另一台接手。所有者离开半小时的任务，会被看到它的任何计算机删除。在某台计算机上失败的任务会在窗口中注明，并留给其他计算机。
* **停止：** 取消勾选*共享本机*会先让正在进行的部分完成；关闭程序则会放弃它，由另一台计算机接手。
* **停止任务**（在创建它的计算机上按停止、关闭该计算机上的程序，或程序崩溃）：所有已完成的部分都保留在那台计算机上，每台计算机也保留它正在渲染的部分。再次启动同一渲染时，只会为缺失的部分创建任务，而一台重新拿回自己已开始部分的计算机会继续完成它。在命令行上也一样：再次运行同一条命令即可。不限遍数的蒙特卡洛渲染（窗口中的那种）在停止时会把各块拼合起来，与以前一样。

<a id="how-the-rendering-works"></a>

## 渲染的原理

分形图片是逐像素计算的：对每个像素，沿一条光线深入场景，直到碰到分形的表面，每走一步都要把公式计算几十次。MB3D 在处理器上完成这些工作，使用它的所有核心。mb3d-rs 也能这样做，还能把工作交给**显卡**，显卡拥有成千上万个小处理器，正是为这类工作而设计的。

**"计算设备"**设置（主窗口的内部选项卡）用于选择：

| | |
|---|---|
| **自动**（默认） | 显卡和处理器分担图片：显卡从顶部领取行，处理器的线程从底部领取，直到二者相遇。阴影和环境光遮蔽也以同样方式分担；如果显卡明显更快，则由显卡单独完成。 |
| **CPU** | 只用处理器，与 MB3D 中一样。 |
| **显卡** | 只用显卡（它会等到自己的程序就绪）。 |

第一次使用某种公式组合时，显卡驱动必须为显卡准备一个程序——即*着色器*——在下文测量所用的计算机上需要 10 到 15 秒。在自动模式下，处理器直接开始而不等它，显卡就绪后再加入，因此小图片从不需要等待。准备好的着色器会保留到下次使用（由显卡驱动保存，在 Linux 上位于程序旁边的 `ShaderCache` 文件夹中），所以下一张使用相同公式的图片会立即在显卡上开始。

窗口标题右侧的**指示灯**表示上一张图片是在哪里计算的：绿色表示在显卡上，橙色表示部分在显卡上，红色表示在处理器上。把鼠标指向它可以看到详细信息。

[渲染的原理](../RENDERING.md)配合图片讲述了完整的过程：自动模式第一次做了什么，为什么显卡并不总是更快的选择，以及指示灯的各种颜色代表什么。

并非所有内容都能交给显卡：少数公式无法为显卡转换，而放大得非常深的视图要用双精度计算（只有部分显卡支持）。这一切都由程序自行决定；您只会在指示灯上看到。

<a id="speed"></a>

## 速度

所有测量都在同一台计算机上进行：AMD Ryzen 9 3900X（12 核，24
线程），配 NVIDIA GeForce RTX 2060，运行 Linux。原版
**Mandelbulb3D 1.99sr37** 在同一台机器上通过 Wine 运行（其
Windows 程序在那里无需修改即可全速运行，使用所有核心），计时从点击*计算 3D* 开始，到图片完成为止。每个参数文件都由三者以相同的图像尺寸渲染：
MB3D、只用处理器的 mb3d-rs（**CPU**）以及**自动**模式下的 mb3d-rs（显卡和处理器协同）。自动模式的时间是同一文件第二次渲染的时间，此时显卡的着色器已经准备好；新公式组合的第一次渲染在这块显卡上还要加上 10–15 秒的着色器准备时间。

<a id="mb3ds-80-example-parameter-files"></a>

### MB3D 的 80 个示例参数文件

MB3D 自带的示例文件（也在本程序的
`M3Parameter` 文件夹中），每个都使用自己的图像尺寸。

MB3D 无法计算其中一个文件（*Test8*：它会因"Division by zero"错误而停止），因此比较只统计 79 个文件。

* **只用处理器的 mb3d-rs 比 MB3D 更快**，79 个文件中有 71 个如此，中位数为 **1.7 倍**；全部文件合计，MB3D 用时 18 分 21 秒，处理器上的 mb3d-rs 用时 17 分 10 秒。
* **自动模式**在 79 个文件中有 77 个比 MB3D 快（中位数 2.1 倍）；全部文件合计 8 分 17 秒。其中 30 个文件由处理器单独完成，多半是因为它只需几秒钟，还没轮到显卡就结束了——在这些文件上，自动模式就是处理器加上片刻的额外开销。显卡参与了 49 个文件，即较慢的那些，在这些文件上它使图片比处理器快了中位数 1.6 倍（最高 13 倍：*Insect Menger*，4.7 秒而不是 63.0 秒）。
* 有 4 个文件的自动模式明显慢于单用处理器（*Ellarien - Shrooms are coming*、*MengerTrees*、*toricaleggs*、*TreePlanet*）：对这些场景而言，显卡比 24 个处理器线程更慢。
* "显卡上的行"是自动模式下由显卡进行光线步进的图片比例（– 表示由处理器单独完成）。

<details>
<summary>全部 80 个文件（点击展开）</summary>

| 参数文件 | MB3D (Wine) | mb3d-rs CPU | mb3d-rs 自动 | 显卡上的行 | 自动 vs MB3D |
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
| Test8 | MB3D 中出错 | 0.1 s | 0.3 s |  | – |
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

图片是一样的：处理器上的 mb3d-rs 生成的图片与 MB3D 的图片仅有舍入误差，而自动模式的图片与处理器的图片平均只相差颜色范围的百分之零点几（细微的细节可能会在不同的像素上闪烁；[技术说明](../TECHNICAL.md)
中有测量数据）。

<a id="the-command-line"></a>

## 命令行

`mb3d` 无需窗口即可渲染。在程序文件夹中打开一个终端（在 Windows 上，在解压出的文件夹中打开命令提示符，并输入 `mb3d`
而不是 `./mb3d`）。在 Linux 或 macOS 上用一行命令安装之后，`mb3d`
可在任何文件夹中使用，`man mb3d` 显示它的手册，在 bash、zsh 和 fish 中按 Tab 键可以补全它的选项：

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

其他所有功能都有子命令，各自带有 `--help`：

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
./mb3d mutagen scene.m3p -o muta                         # 15 variations
```

公式和贴图在程序旁边查找；另一个
MB3D 安装中的公式和贴图可以用 `--formulas` 和 `--maps` 指定。动画采用 MB3D 的格式：关键帧使用 MB3D 的插值方式，可以打开和保存 `.m3a`
项目，帧的写出方式与 MB3D 相同，因此中断的动画可以继续，多台计算机也可以共同渲染一段动画。

<a id="good-to-know-1"></a>

## 须知

* **图片相同，但并非逐位一致。** MB3D 使用旧式 x87 运算的 80 位精度计算；本程序使用如今的 64 位。图片在每个结构上都一致，但个别像素的颜色可能在最后一位上有差别。蒙特卡洛渲染和 MutaGen 使用自己的随机数，因此它们的结果与 MB3D 不同。
* **显卡计算的图片**与处理器的略有不同（单精度）；在非常细微的细节中，闪烁点可能落在其他像素上。深度放大时会自动切换到双精度。
* **处理器上的速度**与 MB3D 接近：MB3D `.m3f` 文件中公式的机器码经过转换，可以在任何处理器上运行（包括 Apple Silicon），代价很小；光线步进本身更快。参见上面的表格。在 Linux 和 Windows 上，图片开始计算时，场景混合公式中的各个公式还会被一起编译成一个函数（只需几毫秒，使用 Cranelift），使它们能通过处理器的寄存器传递数值。再加上更快的公式循环（在所有系统上），80 个示例文件的 CPU
  渲染比 v0.2.74 少用 20% 的时间（有些多达 60%），图片逐位相同。只有当编译后的混合公式测得更快时，场景才会保留它；`MB3D_NO_JIT=1` 可将其关闭。在 macOS 上它也默认开启，但仍在测试中：在解压出的文件夹中运行 `./jit_check.sh`
  即可检查（它会比较编译后的公式与转换后的公式，并在开启和关闭的情况下渲染示例文件；图片必须相同）。
* **大图片**可以一次性渲染（一张 15000 x 20000 的图片需要约 15 GB 内存；有景深时需要数小时），也可以分块渲染，还可以在多台计算机上渲染（`--tiles` 和大图渲染窗口），这样需要的内存更少。计算 3D 不会开始计算放不进空闲内存的图片（在 Linux 上，因为那里能得知空闲内存），并会给出提示；集群中的计算机会把超出自己内存的分块留给其他计算机。
* **未完成的渲染**保存在缓存文件夹中（`~/.cache/mb3d-rs/resume`，在 Windows 上为 `%LOCALAPPDATA%\mb3d-rs\resume`，或在程序旁边）：图片每像素 18 字节（一张 15360 x 15360 的图片占用 4.2 GB），蒙特卡洛渲染每像素 64 字节。已计算的内容会立即写入，并每 5 分钟刷新到磁盘一次。在最初
  10 秒内停止的渲染不保留任何内容；超过 30 天的未完成渲染会被删除，当它们总共占用超过 50 GB 时，最旧的也会被删除。渲染只有在参数和程序版本相同时才能继续（并且显卡或处理器的选择也要相同；蒙特卡洛渲染的曝光和饱和度可以改变）。蒙特卡洛渲染会一直保持未完成状态，直到达到它的光线数（批处理、`--rays`、`--passes`）或在恢复列表中被删除。
* **较新的或您自己的公式、贴图、主题和语言**放在
  `~/.local/share/mb3d-rs/M3Formulas`、`.../M3Maps`、`.../Themes` 和 `.../Languages` 中（无论用哪种方式安装；在 Windows 上为 `%APPDATA%\mb3d-rs\...`）。它们会与程序自带的那些一起使用，并替换同名的自带文件，因此论坛上更新的公式无需改动程序文件夹即可生效。

<a id="updates"></a>

## 更新

每个版本及其改动都列在 [UPDATES.md](../UPDATES.md) 中（英文）。

<a id="for-developers"></a>

## 面向开发者

[技术说明](../TECHNICAL.md)介绍了如何使用显卡，从 MB3D 的 Delphi 源代码移植了哪些内容以及如何移植，运行 MB3D 公式机器码的三种方式，场景文件格式及其键，与 MB3D 的对照检查，以及如何从源代码构建：

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## 许可证

MB3D 采用 LGPL 2.1 许可证。本移植版是衍生作品，使用相同的许可证。

<a id="credits"></a>

## 致谢

mb3d-rs 所依托的人员和项目列在[英文 README](../../README.md#credits) 中。
