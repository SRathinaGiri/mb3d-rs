<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs：適用於 Linux、Windows 與 macOS 的 Mandelbulb3D">

# mb3d-rs — 適用於 Linux、Windows 與 macOS 的 Mandelbulb3D
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · **繁體中文**
<!-- languages:end -->

*本文譯自英文版 README（版本 0.2.84）。兩者如有出入，以[英文版 README](../../README.md) 為準。*

**下載：[最新版本](https://github.com/thr32423/mb3d-rs/releases/latest)** · [下載](#download) · [更新內容](../UPDATES.md)

mb3d-rs 就是 [Mandelbulb3D](https://github.com/thargor6/mb3d)（MB3D）這套
3D 碎形軟體，經過重寫後可在現今的電腦上執行：支援
**Linux、Windows 與 macOS**，而且圖片可以交由**顯示卡**計算，
MB3D 當年只能使用處理器。它能開啟 MB3D 的參數檔（`.m3p`、`.m3i`、論壇上的文字參數），使用 MB3D 的公式與貼圖，算出相同的圖片。

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="主視窗"></a>

它包含兩個程式：

* **Mandelbulb3D** — 桌面程式，沿用 MB3D 原本的各個視窗：主視窗、公式、光照、後製處理、導航器、動畫製作器、批次處理、大型渲染、蒙地卡羅、體素匯出、Bulb Tracer 2、MutaGen
  等等。會用 MB3D 的人就會用這個程式；MB3D 的教學在這裡同樣適用。

  <details><summary>詳細說明</summary>

  這些視窗就是 MB3D 自己的視窗 — 依照 MB3D 原始的視窗定義製作，因此每個按鈕、滑桿和提示都在 MB3D 中原本的位置：
  
  * **主視窗：**開啟與儲存參數（`.m3p`、`.m3i`、文字、剪貼簿），儲存圖片（PNG、JPEG、BMP、16 位元 Z-buffer），各種滑鼠模式、旋轉按鈕、圖片尺寸、品質預設、復原，以及邊算邊顯示圖片與統計資料的*計算 3D*，還有所有分頁（計算、內部、資訊、切割、Julia、相機、著色、立體）。
  * **公式：**六個公式欄位、MB3D 的公式清單與選項、混合（hybrid）類型、我的最愛，以及 JIT 公式編輯器。
  * **光照：**燈光 1–6、預設組、色盤，以及物件、環境、動態霧與背景圖分頁；變更會立刻重繪圖片，不必重新計算。
  * **後製處理：**以 z-buffer 計算法線、硬陰影與環境陰影、反射與透明、景深。
  * **導航器：**用鍵盤和滑鼠四處走動、觀看，然後按*視角送至主程式*。
  * **動畫製作器、批次處理、大型渲染、蒙地卡羅、體素匯出、Bulb Tracer 2**（網格）與網格預覽、**高度圖產生器、MutaGen、ZBuf16Bit、貼圖序列、Ini 目錄** — 全都與
    MB3D 相同。
  * **群集**（新功能，位於工具分頁）：讓您的多台電腦一起渲染，請參閱[在多台電腦上渲染](#rendering-on-several-computers-the-swarm)。

  </details>
* **mb3d** — 供指令碼與批次工作使用的命令列渲染器：圖片、動畫、批次清單、蒙地卡羅渲染、體素與網格匯出，全都不需要視窗。

<a id="is-it-safe-to-run-check-it-yourself"></a>

## 執行它安全嗎？您可以自己檢查

您不必只聽我們的說法。[scanrepo.dev](https://www.scanrepo.dev)
是一個獨立服務，會檢查 GitHub 儲存庫是否有惡意軟體的跡象：啟動其他程式、連上網際網路、隱藏自身行為，或繞過程式語言安全檢查的程式碼。任何人隨時都可以對這個儲存庫執行掃描：

**[▶ 在 scanrepo.dev 上掃描 mb3d-rs](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
（按 *Rescan* 可取得目前程式碼的最新結果）

版本 0.2.66 的評等為**安全：100 分中 0 分，沒有任何發現**（先前：
[0.2.44 的結果](../img/scan_0.2.44.png)）。更早的某個版本曾被標記，我們因此修改了程式碼（[issue #4](https://github.com/thr32423/mb3d-rs/issues/4)）；現在的寫法就是要讓它保持這樣。程式中沒有任何連上網際網路的程式碼；它唯一會請系統開啟的其他程式，是您按下開啟資料夾的按鈕時所開啟的檔案管理員。

掃描只能找出已知的警訊，並不是保證。以下方式讓您可以自己進一步檢查：

* **所有原始碼都在這裡。**下載檔是由 GitHub 自己的建置伺服器依照您可以在[發行工作流程](.github/workflows/release.yml)中讀到的步驟，從原始碼建置而成，而不是在任何人的個人電腦上建置。
* **每個版本都列出校驗和**（`SHA256SUMS`），讓您確認下載的檔案與發布的檔案完全相同；下方的一行安裝指令會替您做這項檢查。0.2.45 起的版本也附有 GitHub 的建置證明：`gh attestation verify
  FILE --repo thr32423/mb3d-rs` 會顯示某個下載檔是從哪個 commit
  建置的。

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## 與 Mandelbulb3D 相比有哪些新功能

* **由顯示卡計算圖片** — 包括圖片本身、陰影、環境光遮蔽、反射以及蒙地卡羅渲染。在一般的電腦上，這比只用處理器快上好幾倍；請參閱下方的[速度](#speed)。
* **自動模式**：顯示卡與處理器一起計算每張圖片，而且絕不互相等待 — 如果顯示卡還沒準備好，處理器就先開始，顯示卡隨後加入。
* **標題列上的指示燈**會顯示上一張圖片是在哪裡計算的。
* **介面大小**可在 75 到 200 % 之間調整，適合高解析度螢幕。

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="視覺主題，150 %"></a>

  <details><summary>詳細說明</summary>

  **偏好設定 ▸ 主題 ▸ 介面大小**可以把所有視窗及其文字放大或縮小（75 到 200 %），適合高解析度螢幕；版面配置仍與 MB3D 完全相同。
  
  | 100 % | 150 % |
  |---|---|
  | ![視覺主題，100 %](../img/themes_100.png) | ![視覺主題，150 %](../img/themes_150.png) |

  </details>
* **八種視窗外觀**，全都是可以複製和修改的主題檔：其中兩種重現 MB3D 的外觀，六種是新的。

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Midnight 主題"></a>

  <details><summary>詳細說明</summary>

  **偏好設定 ▸ 主題**提供八種外觀，每種都是 `Themes`
  資料夾中的一個檔案：*Glossy*（深色的預設外觀）與 *Windows* 重現 MB3D 自己的兩種外觀，其他六種是新的。按一下圖片可看完整大小。
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**：MB3D 的預設外觀 | **Windows**：MB3D 的「Themes off」 |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**：較淺的中性灰，鋼藍色點綴 | **Daylight**：溫暖的淺灰，藍綠色點綴 |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**：深海軍藍，藍色點綴 | **Ember**：溫暖的炭灰，橘色點綴 |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="高對比"></a> |
  | **Forest**：深綠灰，綠色點綴 | **High contrast**：黑底、白字、黃色醒目標示 |
  
  主題是程式旁邊 `Themes` 資料夾中的一個小文字檔；複製一份、修改顏色，下次啟動時它就會出現在清單中。

  </details>
* **程式當掉時在每個系統上都會留下記錄**：程式的 panic（或結束時的錯誤）會附加到 `mb3d_crash.txt`，位於程式旁邊或其資料資料夾中（以前只有 Windows 會這樣）。回報錯誤時請附上這個檔案。
* **電腦上的所有顯示卡都會參與計算**，不必做任何設定：找到的每張顯示卡都從與第一張顯示卡及處理器相同的工作池中領取一段段的圖片列，所以比較快的顯示卡自然領得多；陰影、環境光遮蔽、蒙地卡羅光線與反射也以同樣方式分攤，動畫則每張顯示卡同時渲染一個影格。較慢的顯示卡（獨立顯示卡旁邊的內建顯示晶片）盡力而為，絕不會拖住整張圖片：沒有剩下可領的列之後，閒置的顯示卡和處理器會接手它還沒交回的列。失敗的顯示卡會被排除，其他的繼續運作。
  `mb3d --list-cards` 會顯示找到了哪些顯示卡。（已用同一張顯示卡開啟兩次、以及用軟體 Vulkan 驅動程式充當第二張顯示卡測試過；尚未在裝有兩張實體顯示卡的電腦上測試。）
* **未完成的圖片就是一個 `.m3i` 檔。**被停止、當掉或強制結束的渲染已計算的部分會保留在快取中（未完成的渲染視窗、`mb3d resume`）；對於圖片，這個檔案現在隨時都是一個 `.m3i` 圖片檔，也就是渲染資料夾中的 `image.m3i`，尚未計算的列以背景顯示。您可以隨時把它複製出來，或在未完成的渲染視窗中按 *Save m3i*（命令列上是 `mb3d
  resume --m3i KEY file.m3i`；兩者也都會把舊版本的
  `data.bin` 寫成 `.m3i`），然後在這裡或在
  Mandelbulb3D 中開啟，不需要轉換。（蒙地卡羅渲染有自己的記錄，每一輪的 `.m3c` 可用 `--m3c` 寫出。）
* **視窗快速鍵**：一組按鍵就能從任何視窗把主視窗、導航器、動畫、公式、光照、後製處理或蒙地卡羅視窗帶到最前面（Ctrl+1 到 Ctrl+7），而
  Ctrl+W 會關閉目前所在的視窗（macOS 上為 Cmd）。可在*偏好設定 > 快速鍵*中自訂：按一下欄位再按下按鍵，Backspace 可移除快速鍵。
* **以您的語言顯示介面**：視窗中的每段文字都來自語言檔；隨附十二種語言，而且程式會替新語言寫出檔案，讓您翻譯。

  <details><summary>詳細說明</summary>

  主視窗標題列右端的語言標記顯示介面語言（`EN`）；按一下即可開啟語言視窗。語言是程式旁邊 `Languages` 資料夾中的一個文字檔。隨附：English、Deutsch、Français、Italiano、Nederlands、Polski、
  Kaszëbsczi、हिन्दी、日本語、한국어、简体中文與繁體中文（各附國旗；東亞語言附帶 Noto Sans CJK 的子集以顯示其文字，印地語附帶 Noto Sans Devanagari，其連字與母音符號由 rustybuzz 文字排版引擎排版）。這些翻譯是機器翻譯並經過結構檢查，並非由母語人士校對：歡迎指正，每一處都只是檔案中的一行。翻譯方法：輸入您語言的名稱並按*建立語言檔案*；程式會寫出 `Languages/<name>.lng`，其中每段文字都是英文，一行一段（`Button2.Caption = Calculate
  3D`）。用文字編輯器翻譯 `=` 右邊的文字，然後按
  *重新載入*：視窗會立刻跟著更新，不必重新啟動。省略的行會保留英文文字。*儲存並離開*會保留這個語言供下次啟動使用。

  </details>
* **Linux、Windows 與 macOS**（Intel 與 Apple Silicon）。
* **命令列程式**，用於所有不需要視窗的工作。
* **超大圖片一次完成**：15000 x 20000 像素以上也不會記憶體不足，而且景深會使用所有處理器核心（在 24 執行緒上比 0.2.45 之前快 11 倍，圖片相同）。
* **16 位元圖片：**每個色彩有 65,536 階而非 256 階的 PNG 檔，後續編輯時不會出現色帶。

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="16 位元圖片"></a>

  <details><summary>詳細說明</summary>

  MB3D 儲存的是 8 位元圖片：每個色彩 256 階。拿來觀看綽綽有餘，但圖片事後再編輯時 — 亮度、對比、曲線 — 平滑的區域（例如背景或霧）就會裂成明顯的色帶。**儲存圖片 ▸ PNG** 提供第二種檔案類型 **PNG 16 位元色彩**：也就是程式計算出來、尚未捨入到 256 階的色彩，包含所有陰影、反射和景深。檔案看起來一樣，編輯時則保持平滑。
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="大幅提高對比後的背景漸層：8 位元出現階梯，16 位元保持平滑"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="選有「PNG 16 位元色彩」的儲存對話框"></a>

  </details>
* **雜訊濾波器**，適用於蒙地卡羅及所有其他圖片，採用 GIMP 中常見的
  DCT 降噪；蒙地卡羅渲染另有一個神經網路濾波器：Intel Open Image Denoise。

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="雜訊濾波器"></a>

  <details><summary>詳細說明</summary>

  **雜訊濾波器**會去除圖片中的顆粒：它是
  GIMP *濾鏡 ▸ 強化 ▸ 雜訊消除 ▸ Denoise DCT* 的 **DCT 降噪**，在
  GIMP 的各種濾鏡中最適合這類圖片（請參閱下方的致謝）。
  
  * **蒙地卡羅視窗 ▸ -雜訊**（MB3D 隱藏起來的按鈕）：對顯示中的圖片及儲存的圖片濾波，強度依每像素的光線數而定。渲染本身繼續以未濾波的方式進行，下一次更新又會顯示未濾波的畫面。
  * **後製處理 ▸ 雜訊濾波**：*立即濾波*會對主視窗的圖片濾波；*自動濾波新圖片*會對每張新圖片濾波。*強度*是要去除的雜訊強度（4 為輕度；數值越高，去除的顆粒越多，細節也損失越多）。
  
  蒙地卡羅圖片的顆粒最多。與每像素 512 條光線渲染的圖片相比，濾波後的圖片在全部 13 項測試中（四個場景、
  4 到 32 條光線）都比未濾波的更接近：4 條光線時最多接近 23 %，
  8 條光線時 5 到 10 %，32 條光線時 1 到 3 %，此時顆粒已所剩不多。

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="每像素 4 條光線的蒙地卡羅圖片、加上雜訊濾波器的同一張圖片，以及 512 條光線的圖片"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="後製處理：雜訊濾波區塊"></a>

  **神經網路雜訊濾波器**是
  [Intel Open Image Denoise](https://www.openimagedenoise.org)，也就是
  Blender Cycles 與許多其他路徑追蹤器所用的濾波器，以渲染圖片訓練而成。它對渲染中光線的平均值濾波（在曝光與飽和度之前），所以圖片能保持原本的亮度。
  Intel 的函式庫隨程式附上（`oidn` 資料夾；在 .deb
  與 .rpm 中為 `/usr/lib/mb3d-rs/oidn`）；可用 `MB3D_OIDN` 指定另一個。

  * **蒙地卡羅視窗 ▸ -雜訊AI**：與 *-雜訊*一樣，對顯示中的圖片及儲存的圖片濾波；渲染則繼續以未濾波的方式進行。
  * **命令列**：`mb3d montecarlo scene.m3p --rays 8 --denoise-ai`。
  * 在處理器上處理一張 640 x 480 的圖片需要 0.2 到 0.6 秒。

  與 512 條光線圖片的差異（所有像素的平均值，越低越好）：

  | 場景 | 4 條光線 | DCT 濾波器 | 神經網路濾波器 | 8 條光線 | DCT 濾波器 | 神經網路濾波器 |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  在四個場景中，有三個以神經網路濾波器最接近；在 Julia
  集合的細小顆粒上，DCT 濾波器稍微接近一些。差異在玻璃與鏡面上最明顯，因為光線少時那裡的顆粒最多：

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="每像素 4 條光線的蒙地卡羅圖片，分別加上 DCT 濾波器、神經網路濾波器，以及 512 條光線的圖片"></a>

  </details>
* **透明背景**，適用於圖片與動畫。

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="透明背景"></a>

  <details><summary>詳細說明</summary>

  圖片與動畫影格可以儲存成**透明背景**，以便把碎形放到另一張圖片上或放進影片編輯軟體：**儲存圖片 ▸ PNG ▸ PNG 透明背景**，在動畫製作器中則是**透明背景**（PNG 影格）。開啟反鋸齒時（檢視比例 1:2 以上，或 `--aa`），邊緣是柔和的，而且不會殘留舊背景色的邊框。供合成用的深度圖片原本就有（*同時儲存 Z-buffer*、`--depth`）。
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="一張有背景的圖片，以及在棋盤格上顯示的透明背景版本"></a>

  </details>
* **動畫影片：**由程式直接輸出 AVI 檔，或提供製作小型 MP4 的 ffmpeg 指令。

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="動畫影片"></a>

  <details><summary>詳細說明</summary>

  在動畫製作器中，**同時製作影片 (AVI)** 會在影格渲染完成時依您設定的**每秒影格數**把影格合成一段影片：輸出資料夾中的 `<project name>.avi`，與影格放在一起，影格也會保留。它可以在 VLC 和 mpv 中播放，也能在影片編輯軟體中開啟（Kdenlive、
  Shotcut、DaVinci Resolve）。每個影格都以完整圖片儲存，所以檔案很大：適合編輯和保存，比較不適合分享。
  
  若要製作方便分享的小型影片，**FFmpeg 指令**會把製作 MP4
  （或在透明背景時製作 WebM）的指令放到剪貼簿和訊息中。安裝 [ffmpeg](https://ffmpeg.org)，影格完成後把指令貼到終端機，就會產生影片；這個程式本身不會啟動其他程式。
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="動畫製作器中的同時製作影片 (AVI)、每秒影格數與 FFmpeg 指令"></a>

  </details>
* **開啟對話框中的參數檔預覽**。

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="開啟參數時的預覽"></a>

  <details><summary>詳細說明</summary>

  **開啟 ▸ 開啟 m3p** 與**開啟 m3i** 會在開啟之前顯示所點選參數檔的小圖，以及其尺寸與公式；方向鍵可以一邊預覽一邊在清單中移動。預覽圖在背景中由處理器計算，所以對話框仍能即時回應，而且程式執行期間每個檔案的預覽都會保留。
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="顯示參數檔預覽的開啟對話框"></a>

  </details>
* **導航器：精確數值與重設視角。**按一下數值即可輸入；也能回到導航器開始時的視角。

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="導航器：精確數值與重設視角"></a>

  <details><summary>詳細說明</summary>

  * 在導航器的面板中**按一下數值** — Julia 值、公式值、4D 旋轉、其他（Misc）數值 — 然後輸入精確的數字，與
    MB3D 相同（角度以度為單位）。之後滑桿會從該數值繼續調整。
  * 縮放步進太粗時，**按一下縮放值**並輸入（這是
    MB3D 問題清單中的一項願望）；輸入的縮放值會固定下來（*固定縮放*）。
  * **重設視角**會回到導航器從主視窗取得的位置、方向和縮放；調整過的數值則保留。
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="顯示公式數值以及點選數值後輸入框的導航器"></a>

  </details>
* **新公式 SphereTree：**由彼此相切的球體組成、越分越小的樹，作者是 Tom Lowe。

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="新公式：SphereTree"></a>

  <details><summary>詳細說明</summary>

  **SphereTree** 是 Mandelbulb3D 沒有的公式：一棵由彼此相切的球體構成、不斷分枝成越來越小球體的樹，像泡沫或珊瑚。發明 Mandelbox 的 Tom Lowe 於 2020 年發表了它（請參閱下方的致謝）。它與 dIFS 公式一起列在公式清單中，有四項設定：
  
  * **PackRatio**（0 到 1.22）：球體的大小；為 1 時剛好相切，大於 1 時外側的球會斷開。
  * **Levels**：樹的深度（大多數圖片用 6 就夠了）。
  * **Rotated**：設為 1 時每一層轉動十二分之一圈，整棵樹會變成由球體團簇構成的圓球。
  * **Colour scale**：樹的每個分枝都有自己的顏色。在光照的色彩設定中選擇 *2nd choice*（軌道陷阱）即可看到；這項設定會把它們分布到色彩漸層上。
  
  它可以單獨使用，也能接在 dIFS 變換之後（第三張圖片用
  *tilingbrickIFS* 重複它），而且在顯示卡和處理器上都能計算。請把迭代次數設為 1：這個公式會一次算出整棵樹。含有 SphereTree 的參數只能在這個程式中開啟，無法在 Mandelbulb3D 中開啟。
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="蒙地卡羅渲染器中黃昏時分的 SphereTree 球（Rotated）"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="SphereTree 球體泡沫的特寫"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="以 tilingbrickIFS 重複的 SphereTree：一群球體樹"></a> |
  |:---:|:---:|

  </details>
* **新公式 DomeTree：**一個長出一圈較小圓頂的圓頂，每個小圓頂又各有自己的一圈，作者是 Tom Lowe。

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="新公式：DomeTree"></a>

  <details><summary>詳細說明</summary>

  **DomeTree** 是 Tom Lowe 繼 SphereTree 之後的第二棵樹（請參閱致謝）：一顆球，一側長出一圈圓頂，每個圓頂又有自己較小的一圈，一層接一層，由圓反演構成。它與 dIFS 公式一起列在公式清單中，有五項設定：

  * **Levels**：樹的深度（16 就很夠了）。
  * **Children**：每一圈的圓頂數，4 到 10。
  * **Bend angle**（0.3 到 1.3 效果最好）：圓頂鼓起的程度；數值低會得到扁平的圓盤，數值高則是圓潤的圓頂。超過約 1.3 時表面會變得粗糙。
  * **Colour scale**：每個圓頂都有自己的顏色。在光照的色彩設定中選擇 *2nd choice*（軌道陷阱），並設定較寬的色彩範圍（例如 0 到 100）即可看到。
  * **Start curved**：1（預設）從球開始，0 則從一片會長出第一圈的平坦地面開始。

  請使用較小的*光線步進倍率*（0.2；9 或 10 個 children 時用 0.05）：這個形狀的距離估計很粗略，與 TGlad 自己的 Fragmentarium 檔案相同。與
  SphereTree 一樣，它可在顯示卡和處理器上執行，只需要
  1 次迭代，含有它的參數只能在這個程式中開啟。

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="蒙地卡羅渲染器中有 8 個 children、每個圓頂各自上色的 DomeTree"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree，5 個 children，bend angle 1.2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree，6 個 children，bend angle 0.5：扁平的圓頂"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree，10 個 children：圓頂之間的蕾絲與一顆大球"></a> |
  |:---:|:---:|:---:|

  </details>
* **新公式 UlamSpiral：**烏拉姆的質數螺旋化為 3D，可呈現為城市、天際線、球體或方塊。

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="新公式：UlamSpiral"></a>

  <details><summary>詳細說明</summary>

  1963 年，數學家 Stanisław Ulam 在一場會議中聽著「一篇冗長又非常無聊的論文」，在方格紙上隨手塗鴉。他起初想設計西洋棋題目；結果卻把方格依螺旋編號為
  1、2、3、…，1 在正中央，然後把質數圈起來。質數排列在斜線上，彷彿遵循某種規律，而且用電腦畫出數百萬個時依然如此（Ulam 在 Los Alamos 的 MANIAC II 上試了 100,000 個；Martin Gardner 在 1964 年把這張圖放上《Scientific American》的封面）。直到今天仍沒有人能完全解釋它。
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="烏拉姆螺旋：方形螺旋上 1 到 78,961 之間的質數，角落畫出螺旋的起點"></a>
  
  **UlamSpiral** 把這個螺旋建成 3D，每個數字一個形狀，共有四種形式（*Mode*）：
  
  * **0，質數塔**：每個數字一塊平板，每個質數上立一座塔。斜向排列的質數就成了一排排塔。
  * **1，因數天際線**：每個數字一根柱子，因數越多柱子越高；質數就是柱子之間低矮的街道。
  * **2，因數球體**：每個數字一顆球，因數越多球越大（質數是小球）。
  * **3，質數方塊**：數字繞著立方體外殼而非正方形排列，每個質數上一個立方體。
  
  *Rings* 設定螺旋的大小（60 圈是到
  14,641 為止的數字），*Height* 設定塔或柱子的高度，*Gap* 設定它們之間的間距。在色彩設定中選擇 *2nd choice*（軌道陷阱）時，顏色會依因數個數而變，所以質數會有自己的顏色。與 SphereTree 一樣，它可在顯示卡和處理器上執行，能接在 dIFS 變換之後，只需要 1 次迭代，含有它的參數只能在這個程式中開啟。
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral 質數塔：斜向排列的質數化為一座城市，於蒙地卡羅渲染器中加上霧、硬陰影與景深"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral，因數天際線"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral，因數球體"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral，立方體外殼上的質數方塊"></a> |
  |:---:|:---:|:---:|

  </details>
* **停止的渲染會從原處繼續。**渲染被停止時，已計算的部分會保留下來，程式關閉或當掉、或電腦關機時也一樣（每 5 分鐘存到磁碟一次）：包括計算 3D、蒙地卡羅（也包括一輪計算的中途）、動畫的某個影格或進行中批次的某張圖片、體素匯出、Bulb Tracer 網格、大型渲染的分塊，以及群集渲染（在建立工作的那台電腦上按停止會保留每個已完成的部分，而每台電腦會保留它正在算的部分）。相同參數的計算 3D 與蒙地卡羅會詢問是否繼續，其他的則會自動繼續；**繼續**按鈕（開啟分頁）會列出未完成的渲染，可以繼續或刪除。在命令列上，同一個指令會自動繼續（也適用於 `--tiles` 的圖片、`montecarlo`、
  `animate`、`batch`、`voxel` 與 `mesh`）；`--fresh` 會重新開始，`mb3d
  resume` 會列出它們。繼續完成的渲染與一次算完的結果相同。
* **在您的多台電腦之間分擔渲染**，也就是「群集」：能看到同一個共用資料夾的電腦一起渲染圖片、大型渲染、蒙地卡羅圖片或動畫。請參閱
  [在多台電腦上渲染](#rendering-on-several-computers-the-swarm)。
* **新公式 PseudoKleinian：**由大廳、拱門與球體構成的「Kleinian 建築」，以單一公式實現，並有自己的精確距離。

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="新公式：PseudoKleinian"></a>

  <details><summary>詳細說明</summary>

  被稱為「Kleinian 建築」的大廳、拱門、柱子與層層相套的球體，是最受喜愛的 3D 碎形圖片之一。在 MB3D 中，它們只能用多個公式的混合、搭配仔細調校的設定來製作，那是 Theli-at 在 2011 年找到的配方；之後 knighty 把同樣的東西寫成一個有自己距離估計的公式，這就是
  **PseudoKleinian**。每一層都把點摺進一個方盒，在球面上反演，再加上一個偏移量，而距離是在摺疊之後精確量到一個地平面，所以牆面乾淨俐落，圖片也渲染得很快。設定沿用 knighty 的：*Levels*（8）、*Size*（反演球）、*CSize X/Y/Z*（方盒；他的 0.924、0.908、0.924 會得到經典的大廳，稍作變化就有新的建築）、*Julia X/Y/Z*
  （每一層的偏移）、*Shape*（0 為地平面，1 為通過 *Offset*、厚度為
  *Thickness* 的圓柱平面，2 為圓角方盒）、*DE offset*（把表面挖空）以及 *Colour scale*（搭配 *2nd choice*，即軌道陷阱：最小的軌道半徑，也就是經典的 Kleinian 條紋）。
  
  這個結構填滿整個空間，所以您是從內部觀看它：一開始把視角中心放在原點附近，相機往後退一兩個單位，再用導航器在大廳中穿梭。在蒙地卡羅渲染器中，室內幾乎照不到天空光，所以要在大廳裡放一個定位光源（第一張圖片），或用切割平面把結構切開，從上方往內看（第二張）。它可在顯示卡和處理器上執行，需要 1 次迭代，能接在 dIFS 變換之後使用，也能像其他 dIFS 公式一樣用於 DE 組合（不過在填滿空間的結構中，這些組合的效果不大），含有它的參數只能在這個程式中開啟。
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="從 knighty 的視點看 PseudoKleinian：由內部燈光照亮、滿是拱門與層層相套球體的大廳，於蒙地卡羅渲染器中加上霧、硬陰影與景深"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="用切割平面切開並從上方觀看的 PseudoKleinian：球體構成的格架與其間的空格"></a>

  </details>
* **一步完成立體影像對**：兩眼的影像都會渲染並左右並排顯示，可用平行法或交叉法觀看。由 Rathinagiri S
  （[SRathinaGiri](https://github.com/SRathinaGiri)）貢獻。

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Dainbramage 的 Hydra 立體影像對，左右並排供平行法觀看"></a>

  <details><summary>詳細說明</summary>

  **立體 ▸ 渲染立體影像對**會依分頁中的立體設定，先渲染左眼、再渲染右眼影像，並在主視窗中左右並排顯示。它們是依平行法排列的（左圖在左邊）；**交叉眼預覽**會把它們對調，供交叉法觀看。這是預覽：之後若不重新渲染，就無法變更光照和色彩，而且不能搭配分塊使用。單眼影像的按鈕仍然保留。
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Dainbramage 的 Hydra 立體影像對，左右並排供平行法觀看"></a>

  </details>
* **整張圖片的剩餘時間**：圖片下方的進度列在整個計算過程中只會填滿一次，旁邊顯示到圖片完成為止的剩餘時間，包括陰影、環境光遮蔽與反射。

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="進度列與總剩餘時間"></a>

  <details><summary>詳細說明</summary>

  MB3D 的進度列在每個階段都會重新開始。這裡顯示的是整個計算：主要渲染，接著是硬陰影與環境光遮蔽、反射以及景深。主要渲染之後的各階段需要多久，是依場景的需求以及主要渲染的進行情況估算的；這個估算是以 366 個場景、約 1400 次渲染擬合而成。再次計算的圖片會以上一次花費的時間為準。
  
  在顯示卡上使用新公式的第一張圖片要等著色器編譯：這時那一行會顯示*正在編譯著色器*以及剩餘時間，估算時會採用處理器的速度，若著色器能及時準備好，也會把顯示卡加入計算。詳情請見
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar)。
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="計算硬陰影與環境光遮蔽期間的主視窗，顯示總剩餘時間"></a>

  </details>
* 標題列時鐘圖示後面的 **History**：您計算過的每張圖片及其縮圖、開啟或儲存過的每個檔案，可以用清單、列或任意大小的磚塊顯示，可依日期分組或連續列出；可以搜尋、排序、只顯示不同的圖片，按兩下就能載入任一項目。即使有數十萬筆項目的歷史記錄也能立刻開啟。
* 加上硬陰影與反射之後，**光照仍可即時調整**：移動燈光或變更其顏色，圖片會立刻跟著改變，與 Mandelbulb3D 相同。

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="變更光照後的主視窗與光照視窗"></a>

  <details><summary>詳細說明</summary>

  先單純地渲染，再試試燈光的位置和顏色：在光照視窗中的每項變更都會立刻以原先計算的陰影重繪，與 Mandelbulb3D 相同。變更燈光的顏色或強度絕不會重新計算陰影。被移動之燈光的陰影、反射和景深，可用後製處理視窗中的按鈕重新算出，與 Mandelbulb3D 相同。
  
  變更光照之後，被您停止的計算也仍然可以繼續。一種搭配的工作方式：wiki 中的[光照工作流程](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-zh-TW)。詳情請見
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field)。
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="變更光照後的主視窗與光照視窗：圖片已經變亮"></a>

  </details>

<a id="download"></a>

## 下載

最新版本的免安裝即用程式：

| 系統 | 下載 |
|---|---|
| Windows（64 位元） | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux（64 位元） | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian、Ubuntu、Mint（像其他程式一樣安裝） | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora、RHEL、Rocky、openSUSE（像其他程式一樣安裝） | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS，Apple silicon（M1 及更新機型） | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS，Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

解壓縮後，在解壓縮出來的資料夾中啟動 **Mandelbulb3D**（Windows 上為 `Mandelbulb3D.exe`）。資料夾的配置就像 MB3D 的安裝目錄，包含 MB3D 的公式、貼圖以及 80 個範例參數檔。

* 如果 Windows 顯示「Windows 已保護您的電腦」，請按*其他資訊* ▸
  *仍要執行*。
* macOS 會封鎖未簽署的下載程式：請在解壓縮出來的資料夾中執行一次
  `xattr -dr com.apple.quarantine .`。

**Linux 與 macOS 的另一種方式：**不必手動下載，這一行指令會把最新版本下載到 `~/mb3d-rs`（之後再執行一次即可更新；指令碼 [download.sh](../../download.sh) 也在程式資料夾中）：

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

它會在解壓縮前檢查下載的檔案，把 **Mandelbulb3D** 放進應用程式選單（Linux），並建立可在任何資料夾中輸入的 `Mandelbulb3D` 與 `mb3d`
指令。`sh ~/mb3d-rs/download.sh
--uninstall` 會再移除選單項目和指令；您的檔案會保留。

Linux 版程式可在約 2018 年以後的發行版上執行（Ubuntu 18.04、
Debian 10、Rocky/RHEL 8 及更新版本）。

**像其他程式一樣安裝：**在 Debian、Ubuntu 和 Mint 上，開啟上方的
`.deb` 檔（或 `sudo apt install ./mb3d-rs-linux-x64.deb`）；在
Fedora、RHEL 和 Rocky 上則用 `.rpm` 檔（`sudo dnf install
./mb3d-rs-linux-x64.rpm`，openSUSE 上為 `sudo zypper install
./mb3d-rs-linux-x64.rpm`）。之後 Mandelbulb3D 就會出現在應用程式選單中，`mb3d` 可在任何資料夾中使用；您的設定與 History 保存在 `~/.config/mb3d-rs` 和
`~/.local/share/mb3d-rs`。

[入門指南](../../GETTING_STARTED.md)會帶您走過最初幾個步驟；舊版本與發行說明請見
[releases](https://github.com/thr32423/mb3d-rs/releases) 頁面。

<a id="guides-and-help"></a>

## 指南與說明

逐步指南位於 [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-zh-TW)：

* [安裝](https://github.com/thr32423/mb3d-rs/wiki/Installing-zh-TW)：Windows、macOS 與 Linux、更新、您的檔案放在哪裡
* [光照工作流程](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-zh-TW)：先單純地渲染，再試試燈光、陰影與反射
* [群集設定](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-zh-TW)：透過共用資料夾在多台電腦上渲染
* [疑難排解與常見問題](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-zh-TW)：當某些功能不如預期時

每個版本及其變更：[完整清單](../UPDATES.md)。

<a id="rendering-on-several-computers-the-swarm"></a>

## 在多台電腦上渲染（群集）

能看到同一個共用資料夾的電腦可以一起渲染：分塊計算的圖片、大型渲染、蒙地卡羅圖片、動畫或批次。每台電腦都領取別人還沒拿的部分，所以快的電腦會領得比較多，而啟動渲染的那台電腦會把結果組合起來。分享是透過資料夾完成的：程式不會開啟任何網路連線，也不和任何對象通訊，所以安全程度與您的檔案共用相同。

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D 與群集視窗（用工具分頁上的群集按鈕開啟），兩台電腦正在分塊渲染一張圖片：共用資料夾、共享這台電腦、各台電腦及其目前工作、工作項目，以及顯示已完成分塊、進行中分塊與缺少分塊的預覽"></a>

<a id="setting-it-up"></a>

### 設定方式

逐步說明及檢查事項：wiki 中的[群集設定](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-zh-TW)。

1. **一個每台電腦都能寫入的資料夾：**NAS 上的共用、Windows
   共用、Samba 共用或 NFS。在 Linux 上請把它掛載成資料夾，例如
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`；只在檔案管理員中開啟的共用（`smb://...`）並不是程式能使用的資料夾。
2. **每台電腦上都是相同版本的 mb3d-rs。**版本不同的電腦會被列出，但不會參與。
3. **在 Mandelbulb3D 中：**工具分頁上的**群集**按鈕。選擇共用資料夾（每台電腦上都是同一個共用；路徑可以不同），為電腦取一個顯示在清單中的名稱，然後勾選**共享這台電腦**。

就這樣。狀態列會說明目前的情況，例如*與 omarchy 共享：渲染工作分攤給這 2 台電腦*。群集中只有一台電腦時，一切照常在這台電腦上渲染。

<a id="what-is-shared"></a>

### 分擔哪些工作

勾選*共享這台電腦*時，這台電腦會渲染其他電腦放進資料夾的部分，而只要群集中還有其他電腦，它自己的渲染也會交給群集：

* **計算 3D**（以及批次處理視窗）— 圖片分成 N x M 個分塊（在群集視窗中設定），在這裡組合起來，並具備一般計算的所有功能：光照變更、後製處理、存成 m3i（以及其記憶體用量，見下方）。
* **大型渲染** — 分塊一到就逐一儲存；整張圖片從不會保存在記憶體中，所以這是製作
  100000 x 100000 像素以上圖片的方法。
* **蒙地卡羅** — 與單台電腦時相同：每台電腦都在自己的區塊中持續累加計算輪次，直到您按下停止；圖片會隨著所有人的光線變得乾淨，按停止時會把各區塊平均。在視窗的批次模式中，各區塊合計達到批次的光線數時就會停止。
* **動畫製作器** — 影格一個接一個。

視窗會列出各台電腦的核心數、顯示卡以及各自正在做的事、各項工作及其進度，以及最新工作的預覽：已渲染的分塊、帶有進度列的進行中分塊，以及仍然缺少的分塊。

<a id="on-the-command-line"></a>

### 在命令列上

在每台要幫忙的電腦上，對該資料夾啟動一個 worker：

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

然後在任何一台電腦上，以 `--swarm` 和同一個資料夾渲染：

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` 會顯示群集中有誰、每台電腦正在做什麼，以及各項工作進行到哪裡。命令列上的蒙地卡羅圖片會分成各自使用隨機光線的區塊：4 個每像素 64 條光線的區塊就是 `--rays 256` 所要求的 256 條。

<a id="good-to-know"></a>

### 須知

* **如果其他電腦一直閒置**，而這台電腦在渲染時它們的工作清單是空的：以 Samba 提供該資料夾、自己也直接使用它的那台電腦，會讓其他電腦保留舊的檔案清單。請在 `/etc/samba/smb.conf` 中關閉這種快取：在 `[global]` 中設 `smb3 directory leases = no`，在該共用的區段中設 `oplocks = no` 與 `level2 oplocks = no`，然後執行 `sudo systemctl restart smb`，並在其他電腦上重新掛載該共用。
* **記憶體：**在這個程式中，一張圖片每像素約占 20 位元組（其影像緩衝區），在主視窗中再多 10 位元組，然後再加上兩個階段中較大者：含陰影與環境光遮蔽的主要計算（每像素 16 位元組，顯示卡另需 1.2 GB），或繪製階段（每像素 3 位元組，有景深時再加 20，再加上反射的一個條帶：最多 3.5 GB，可用記憶體少時會更小，因為條帶大小設為該階段開始時可用記憶體的一半）。一張 12000 x 6000 的圖片：6 GB。當圖片需要的記憶體超過電腦的可用記憶體（向 Linux、
  Windows 與 macOS 查詢）時，計算 3D 會說明需要多少、可用多少，並在開始前先詢問：計算將會使用置換空間，程式可能會每次停止回應好幾分鐘。大型渲染（單獨使用或搭配群集）就是解決辦法：它的分塊逐一儲存，從不保存整張圖片。搭配群集的計算 3D 會像一般計算一樣在這裡保存圖片（但不含主要計算階段，那部分由負責分塊的電腦以各分塊的記憶體完成）；群集中的電腦會把超出其記憶體的分塊留給其他電腦。
* **安全性：**電腦只會渲染工作所要求的內容，也只會寫入該工作的資料夾；工作中的公式程式碼與所有公式一樣在公式模擬器中執行。
* **速度：**分塊以完整的影像緩衝區傳遞，非常大的圖片可達數百 MB，所以快速的共用很值得。有延遲的同步服務也能用，只是比較慢。
* **離開的電腦：**某台電腦正在渲染的部分，在它半分鐘沒有回應後就會由另一台電腦接手。擁有者已離開半小時的工作，會被看到它的任何電腦移除。在某台電腦上失敗的工作會記在視窗中，並留給其他電腦。
* **停止：**取消勾選*共享這台電腦*會先讓進行中的部分完成；關閉程式則會放棄該部分，由另一台電腦接手。
* **停止工作**（在建立工作的電腦上按停止、關閉該電腦，或程式當掉）：每個已完成的部分都會保留在那台電腦上，而每台電腦會保留它正在渲染的部分。再次啟動同一個渲染時，只會為缺少的部分建立工作，而拿回自己先前已開始之部分的電腦會接著算下去。在命令列上也一樣：再執行一次同一個指令即可。沒有設定終點的蒙地卡羅渲染（視窗中的那種）在停止時會把各區塊組合起來，與以前相同。

<a id="how-the-rendering-works"></a>

## 渲染的原理

碎形圖片是逐一像素計算的：每個像素都有一條光線射入場景，一路追蹤直到碰到碎形的表面，每一步都要計算公式數十次。MB3D 在處理器上做這件事，使用所有核心。mb3d-rs 也能這樣做，還能把工作交給**顯示卡**；顯示卡有數千個小型處理器，正是為這類工作而設計的。

**「計算裝置」**設定（主視窗的內部分頁）用來選擇：

| | |
|---|---|
| **自動**（預設） | 顯示卡與處理器分擔圖片：顯示卡從上方開始領取列，處理器的執行緒從下方開始，直到兩者相遇。陰影與環境光遮蔽也以同樣方式分攤，或在顯示卡明顯較快時完全由顯示卡負責。 |
| **CPU** | 只用處理器，與 MB3D 相同。 |
| **顯示卡** | 只用顯示卡（會等到它的程式準備好）。 |

第一次使用某種公式組合時，顯示卡驅動程式必須為顯示卡準備一個程式 — 也就是*著色器* — 在下方量測用的電腦上需要 10 到 15 秒。在自動模式中，處理器會直接開始，不必等它，顯示卡準備好後再加入，所以小圖片永遠不必等待。準備好的著色器會保留到下次使用（由顯示卡驅動程式保存，在 Linux 上位於程式旁邊的 `ShaderCache` 資料夾），所以下一張使用相同公式的圖片會立刻在顯示卡上開始計算。

視窗標題右邊的**指示燈**會告訴您上一張圖片是在哪裡計算的：綠色是顯示卡，橘色是部分在顯示卡，紅色是處理器。把滑鼠指向它可以看到詳細資訊。

[渲染的原理](../RENDERING.md)以圖片說明完整的來龍去脈：自動模式第一次會做什麼、為什麼顯示卡不一定是較快的選擇，以及指示燈的顏色代表什麼。

並非所有工作都能交給顯示卡：有少數公式無法為顯示卡轉譯，而放大得非常深的視角要以雙精度計算（只有部分顯示卡能做到）。這一切都由程式自行決定；您只會在指示燈上看到結果。

<a id="speed"></a>

## 速度

所有量測都在同一台電腦上進行：AMD Ryzen 9 3900X（12 核心、24
執行緒）搭配 NVIDIA GeForce RTX 2060，作業系統為 Linux。原版的
**Mandelbulb3D 1.99sr37** 在同一台機器上透過 Wine 執行（它的
Windows 程式在那裡不需修改就能全速執行，並使用所有核心），計時從按下*計算 3D* 開始，到圖片完成為止。每個參數檔都由這三者以相同的圖片尺寸渲染：
MB3D、只用處理器的 mb3d-rs（**CPU**），以及**自動**
模式的 mb3d-rs（顯示卡與處理器一起）。自動模式的時間是同一個檔案第二次渲染的時間，此時顯示卡的著色器已準備好；新公式組合的第一次渲染，在這張顯示卡上還要多花 10–15 秒準備著色器。

<a id="mb3ds-80-example-parameter-files"></a>

### MB3D 的 80 個範例參數檔

MB3D 隨附的範例檔（也在這個程式的
`M3Parameter` 資料夾中），各自以自己的圖片尺寸渲染。

MB3D 無法計算其中一個檔案（*Test8*：它會因「Division by zero」錯誤而停止），所以比較時只計算 79 個檔案。

* **只用處理器的 mb3d-rs 比 MB3D 快**：79 個檔案中有 71 個，中位數為 **1.7 倍**；全部檔案合計，MB3D 花了 18 分 21 秒，mb3d-rs 只用處理器花了 17 分 10 秒。
* **自動模式**在 79 個檔案中有 77 個比 MB3D 快（中位數 2.1 倍）；全部檔案合計 8 分 17 秒。其中 30 個檔案在輪到顯示卡之前就由處理器獨自完成，大多是因為只需要幾秒鐘 — 這時自動模式就等於處理器再加上一點點額外負擔。顯示卡參與了 49 個檔案，也就是較慢的那些，在這些檔案中它讓圖片比只用處理器快了中位數 1.6 倍（最多 13 倍：*Insect Menger*，4.7 秒而非 63.0 秒）。
* 有 4 個檔案的自動模式明顯比只用處理器慢（*Ellarien - Shrooms are coming*、*MengerTrees*、*toricaleggs*、*TreePlanet*）：在這些場景中，顯示卡比 24 個處理器執行緒還慢。
* 「顯示卡計算的列」是自動模式中由顯示卡進行光線步進的圖片比例（– 表示由處理器獨自完成）。

<details>
<summary>全部 80 個檔案（按一下展開）</summary>

| 參數檔 | MB3D（Wine） | mb3d-rs CPU | mb3d-rs 自動 | 顯示卡計算的列 | 自動 vs MB3D |
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
| Test8 | MB3D 中出錯 | 0.1 s | 0.3 s |  | – |
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

圖片是相同的：mb3d-rs 在處理器上算出的圖片與 MB3D 的圖片只有捨入誤差的差別，而自動模式的圖片與處理器的圖片平均只差色彩範圍的百分之一以下（細微的細節可能會在不同的像素上閃爍；[技術說明](../TECHNICAL.md)
中有量測數據）。

<a id="the-command-line"></a>

## 命令列

`mb3d` 不需要視窗就能渲染。在程式的資料夾中開啟終端機（在 Windows 上，於解壓縮出來的資料夾中開啟命令提示字元，並輸入 `mb3d`
而不是 `./mb3d`）。在 Linux 或 macOS 上以一行指令安裝之後，`mb3d`
可在任何資料夾中使用，`man mb3d` 會顯示其說明手冊，而在 bash、zsh 和 fish 中按 Tab 鍵可以補全它的選項：

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

其他所有功能都有各自的子指令，也各有自己的 `--help`：

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

公式與貼圖會在程式旁邊尋找；另一個
MB3D 安裝目錄中的公式與貼圖可以用 `--formulas` 和 `--maps` 指定。動畫採用 MB3D 的格式：關鍵影格使用 MB3D 的插補方式，可開啟和儲存 `.m3a`
專案，影格的寫法與 MB3D 相同，因此中斷的動畫可以接著算下去，多台電腦也能分擔同一段動畫。

<a id="good-to-know-1"></a>

## 須知

* **圖片相同，但並非逐位元相同。**MB3D 以舊式 x87 運算的 80 位元精度計算；這個程式使用現今的 64 位元。圖片的每個結構都相符，但個別像素的色彩可能在最後一位數上有所不同。蒙地卡羅渲染與 MutaGen 使用自己的亂數，所以結果與 MB3D 不同。
* **顯示卡算出的圖片**與處理器的稍有不同（單精度）；在非常細微的細節中，閃爍點可能落在不同的像素上。深度縮放時會自動切換為雙精度。
* **處理器上的速度**與 MB3D 相近：MB3D `.m3f` 檔中公式的機器碼經過轉譯，可在任何處理器上執行（包括 Apple Silicon），代價很小；光線步進本身則更快。請參閱上方的表格。在 Linux 和 Windows 上，場景混合公式中的各個公式還會在圖片開始時一起編譯成一個函式（只需幾毫秒，使用 Cranelift），讓它們在處理器的暫存器中傳遞數值。再加上更快的公式迴圈（所有系統皆適用），80 個範例檔的 CPU
  渲染時間比 v0.2.74 少了 20 %
  （部分檔案最多少 60 %），圖片則逐位元相同。場景只有在編譯後的混合公式量測起來比較快時才會保留它；`MB3D_NO_JIT=1` 可關閉此功能。在 macOS 上它也預設開啟，但仍在測試中：在解壓縮出來的資料夾中執行 `./jit_check.sh`
  可以檢查（它會比較編譯後的公式與轉譯的公式，並在開啟與關閉此功能時渲染範例檔案；兩者的圖片必須相同）。
* **大型圖片**可以一次渲染完成（一張 15000 x 20000 的圖片約需 15 GB 記憶體；加上景深要花好幾個小時），也可以分塊渲染，甚至在多台電腦上渲染（`--tiles`，以及大型渲染視窗），這樣需要的記憶體較少。計算 3D 不會開始一張無法放進可用記憶體的圖片（在 Linux 上，因為可以得知可用記憶體），並會說明原因；群集中的電腦也會把超出其記憶體的分塊留給其他電腦。
* **未完成的渲染**保存在快取資料夾中（`~/.cache/mb3d-rs/resume`，在 Windows 上為 `%LOCALAPPDATA%\mb3d-rs\resume`，或程式旁邊）：圖片每像素 18 位元組（一張 15360 x 15360 的圖片占 4.2 GB），蒙地卡羅渲染每像素 64 位元組。已計算的部分會立刻寫入，並且每 5 分鐘寫回磁碟一次。在前 10 秒內就停止的渲染不會保留任何東西；超過 30 天的未完成渲染會被刪除，全部合計超過 50 GB 時則刪除最舊的。渲染只有在參數與程式版本相同時才能繼續（顯示卡或處理器的選擇也要相同；蒙地卡羅渲染的曝光與飽和度可以變更）。蒙地卡羅渲染會一直維持未完成狀態，直到達到其光線數（批次、`--rays`、`--passes`）或在繼續清單中被刪除。
* **較新的或您自己的公式、貼圖、主題與語言**請放到
  `~/.local/share/mb3d-rs/M3Formulas`、`.../M3Maps`、`.../Themes` 與 `.../Languages`
  （不論以何種方式安裝；在 Windows 上為 `%APPDATA%\mb3d-rs\...`）。它們會與程式隨附的檔案一併使用，並取代隨附的同名檔案，所以從論壇取得的更新版公式不必動到程式資料夾就能生效。

<a id="updates"></a>

## 更新

每個版本及其變更都列在 [UPDATES.md](../UPDATES.md) 中（英文）。

<a id="for-developers"></a>

## 給開發人員

[技術說明](../TECHNICAL.md)介紹了如何使用顯示卡、從 MB3D 的 Delphi 原始碼移植了哪些部分以及如何移植、執行
MB3D 公式機器碼的三種方式、場景檔格式及其鍵值、與 MB3D 的比對檢查，以及如何從原始碼建置：

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## 授權

MB3D 採用 LGPL 2.1 授權。這個移植版是衍生著作，採用相同的授權。

<a id="credits"></a>

## 致謝

mb3d-rs 所依據的人員與專案列在[英文版 README](../../README.md#credits) 中。
