<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Linux、Windows、macOS 向けの Mandelbulb3D">

# mb3d-rs — Linux、Windows、macOS 向けの Mandelbulb3D
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · **日本語** · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*英語版 README（バージョン 0.2.84）からの翻訳です。両者の内容が異なる場合は、[英語版 README](../../README.md) が正しい内容です。*

**ダウンロード：[最新リリース](https://github.com/thr32423/mb3d-rs/releases/latest)** · [ダウンロード](#download) · [新着情報](../UPDATES.md)

mb3d-rs は、3D フラクタルプログラム [Mandelbulb3D](https://github.com/thargor6/mb3d)（MB3D）を、今日のコンピュータで動くように書き直したものです。**Linux、Windows、macOS** で動作し、
MB3D ではプロセッサしか使えなかった画像の計算を **グラフィックカード** で行えます。
MB3D のパラメータファイル（`.m3p`、`.m3i`、フォーラムに貼られたテキストパラメータ）を開き、
MB3D の数式とマップを使い、同じ画像を作ります。

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="メインウィンドウ"></a>

2 つのプログラムで構成されています。

* **Mandelbulb3D** — デスクトッププログラムです。MB3D と同じウィンドウを備えています：メインウィンドウ、数式、ライティング、ポストプロセス、ナビゲーター、アニメーション、バッチ処理、大判レンダリング、モンテカルロ、ボクセル書き出し、Bulb Tracer 2、MutaGen
  など。MB3D を知っていれば、このプログラムも使えます。MB3D のチュートリアルもそのまま役立ちます。

  <details><summary>詳しく</summary>

  ウィンドウは MB3D 自身のものです。MB3D の元のウィンドウ定義から作られているので、ボタン、スライダー、ヒントはすべて MB3D と同じ場所にあります。
  
  * **メインウィンドウ：** パラメータを開く・保存する（`.m3p`、`.m3i`、テキスト、クリップボード）、画像を保存する（PNG、JPEG、BMP、16 ビット Z バッファ）、マウスモード、回転ボタン、画像サイズ、品質プリセット、元に戻す、少しずつ出来上がる画像と統計を表示する
    *3D計算*、そしてすべてのページ（計算、内部、情報、カット、Julia、カメラ、カラーリング、ステレオ）。
  * **数式：** 6 つの数式スロット、MB3D の数式リストとオプション、ハイブリッドの種類、お気に入り、JIT 数式エディター。
  * **ライティング：** ライト 1〜6、プリセット、パレット、オブジェクト、アンビエント、
    d.フォグ、背景画像の各ページ。変更すると再計算なしで画像がすぐに描き直されます。
  * **ポストプロセス：** Z-buffer 上の法線、ハードシャドウとアンビエントシャドウ、反射と透明、被写界深度。
  * **ナビゲーター：** キーとマウスで歩き回り、見回してから *視点をメインへ*。
  * **アニメーションメーカー、バッチ処理、大判レンダリング、モンテカルロ、ボクセル書き出し、Bulb Tracer 2**（メッシュ）とメッシュプレビュー、**ハイトマップ生成、MutaGen、ZBuf16Bit、マップシーケンス、Ini フォルダー** — すべて
    MB3D と同じです。
  * **Swarm**（新機能、ツール タブ）：複数のコンピュータで一緒にレンダリングします。
    [複数のコンピュータでのレンダリング](#rendering-on-several-computers-the-swarm)を参照してください。

  </details>
* **mb3d** — スクリプトやバッチ処理向けのコマンドラインレンダラーです。画像、アニメーション、バッチリスト、モンテカルロレンダリング、ボクセルやメッシュの書き出しを、すべてウィンドウなしで行います。

<a id="is-it-safe-to-run-check-it-yourself"></a>

## 実行しても安全？ ご自身で確かめてください

私たちの言葉をそのまま信じる必要はありません。[scanrepo.dev](https://www.scanrepo.dev)
は、GitHub のリポジトリに有害なソフトウェアの兆候がないかを調べる独立したサービスです。調べるのは、他のプログラムを起動するコード、インターネットに接続するコード、動作を隠すコード、プログラミング言語の安全チェックを回避するコードです。このリポジトリに対して、誰でもいつでも実行できます。

**[▶ scanrepo.dev で mb3d-rs をスキャン](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
（現在のコードの最新の結果を見るには *Rescan* をクリックします）

バージョン 0.2.85 の評価は **安全：100 点中 5 点** でした（2026 年 10 月 30 日スキャン、スキャナーのエンジン 10 のルール）。唯一の警告は、プログラムが高速化のために実行時にコンパイルする数式を起動する行に対するものです。新しく生成した機械語を実行する処理は Rust の検査付きの形では書けず、プログラムがそれを許しているのはこの行と Intel のノイズ除去フィルターへの橋渡し部分だけです。残りは情報としての注記です。（以前の結果：0.2.66 は旧エンジンで 100 点中 0 点と評価されました。参考：
[0.2.44 の結果](../img/scan_0.2.44.png)。）以前のバージョンが警告を受けたことがあり、それを受けてコードを変更しました（[issue #4](https://github.com/thr32423/mb3d-rs/issues/4)）。現在はこの状態が保たれるように書かれています。プログラムにはインターネットに接続するコードは含まれていません。プログラムがシステムに開くよう求める他のプログラムは、フォルダを開くボタンをクリックしたときのファイルマネージャーだけです。

スキャンは既知の警告サインを探すものであり、保証ではありません。次の方法で、ご自身でさらに確認できます。

* **ソースコードはすべてここにあります。** ダウンロードファイルは、
  [リリースワークフロー](.github/workflows/release.yml)で読める手順に従って、
  GitHub 自身のビルドサーバーでこのソースからビルドされます。誰かの個人のコンピュータでビルドされるのではありません。
* **どのリリースにもチェックサム（`SHA256SUMS`）が付いています。** ダウンロードしたファイルが公開されたものとまったく同じであることを確認できます。下の 1 行のインストールはこの確認を自動で行います。0.2.45 以降のリリースには GitHub のビルド証明（attestation）も付いています。`gh attestation verify
  FILE --repo thr32423/mb3d-rs` で、ダウンロードファイルがどのコミットからビルドされたかがわかります。

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Mandelbulb3D との違い（新機能）

* **グラフィックカードが画像を計算します** — 画像そのもの、影、アンビエントオクルージョン、反射、モンテカルロレンダリングを計算します。一般的なコンピュータでは、プロセッサだけの場合より数倍速くなります。下の[速度](#speed)を参照してください。
* **自動モード**：グラフィックカードとプロセッサがすべての画像を一緒に計算し、待ち時間はありません。カードの準備ができていなくてもプロセッサが先に始め、カードは後から加わります。
* **タイトルバーの LED** が、直前の画像がどこで計算されたかを示します。
* **インターフェースのサイズ** を 75〜200 % で変えられます（高解像度画面向け）。

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="150 % のビジュアルテーマ"></a>

  <details><summary>詳しく</summary>

  **環境設定 ▸ テーマ ▸ インターフェースのサイズ** で、すべてのウィンドウとその文字を大きくしたり小さくしたりできます（75〜200 %）。高解像度画面向けで、レイアウトは MB3D とまったく同じままです。
  
  | 100 % | 150 % |
  |---|---|
  | ![100 % のビジュアルテーマ](../img/themes_100.png) | ![150 % のビジュアルテーマ](../img/themes_150.png) |

  </details>
* ウィンドウの **8 種類の外観**。どれもコピーして変更できるテーマファイルです。
  2 つは MB3D の外観を再現したもの、6 つは新しいものです。

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Midnight テーマ"></a>

  <details><summary>詳しく</summary>

  **環境設定 ▸ テーマ** には 8 種類の外観があり、それぞれ `Themes` フォルダ内のファイルです。*Glossy*（暗い既定のテーマ）と *Windows* は MB3D 自身の 2 つの外観を再現したもの、残りの 6 つは新しいものです。画像をクリックすると原寸で表示されます。
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**：MB3D の既定 | **Windows**：MB3D の「テーマを無効化」 |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**：明るめのニュートラルグレー、スチールブルーのアクセント | **Daylight**：暖かみのある明るいグレー、ティールのアクセント |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**：深いネイビー、青のアクセント | **Ember**：暖かみのあるチャコール、オレンジのアクセント |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="ハイコントラスト"></a> |
  | **Forest**：暗い緑がかったグレー、緑のアクセント | **High contrast**：黒、白い文字、黄色の強調 |
  
  テーマはプログラムの隣の `Themes` フォルダにある小さなテキストファイルです。
  1 つをコピーして色を変えると、次回の起動時に一覧に表示されます。

  </details>
* **どのシステムでもクラッシュの記録が残ります**：プログラムのパニック（または終了時のエラー）が、プログラムの隣またはデータフォルダにある `mb3d_crash.txt` に追記されます（以前は Windows のみ）。不具合を報告するときはこのファイルを添えてください。
* **コンピュータのすべてのグラフィックカードが計算します**。設定は不要です。見つかったカードはすべて、最初のカードやプロセッサと同じプールから画像の行の帯を受け取るので、速いカードほど多くを受け持ちます。影、オクルージョン、モンテカルロのレイ、反射も同じように分担され、アニメーションはカードごとに 1 フレームずつ同時にレンダリングされます。遅いカード（専用カードと並ぶ内蔵グラフィックスなど）はできる分だけを受け持ち、画像を待たせることはありません。空いている行がなくなると、手の空いたカードとプロセッサが、そのカードがまだ仕上げていない行を引き継ぎます。失敗したカードは外され、他のカードが続けます。
  `mb3d --list-cards` で見つかったカードを確認できます。（1 枚のカードを 2 回開いた場合と、ソフトウェア Vulkan ドライバーを 2 枚目のカードとした場合でテスト済みです。実際に 2 枚のカードを積んだコンピュータではまだテストしていません。）
* **未完成の画像は `.m3i` ファイルです。** 停止した、クラッシュした、または強制終了されたレンダリングが計算した分はキャッシュに保存されます（未完了のレンダリング ウィンドウ、
  `mb3d resume`）。画像の場合、このファイルはどの時点でも `.m3i` 画像ファイル（レンダリングのフォルダ内の `image.m3i`）になっており、まだ計算されていない行は背景になっています。いつでも好きなときにコピーできます。また、未完了のレンダリングウィンドウで *Save m3i* を押すこともできます（コマンドラインでは `mb3d
  resume --m3i KEY file.m3i`。どちらも以前のバージョンの `data.bin` を `.m3i` として書き出します）。このプログラムでも Mandelbulb3D でも、変換なしで開けます。（モンテカルロレンダリングは独自の記録を持ち、パスの `.m3c` は `--m3c` で書き出されます。）
* **ウィンドウのホットキー**：キーの組み合わせで、メインウィンドウ、ナビゲーター、アニメーション、数式、ライティング、ポストプロセス、モンテカルロの各ウィンドウを、どのウィンドウからでも前面に出せます（Ctrl+1〜Ctrl+7）。また Ctrl+W で今いるウィンドウを閉じます（macOS では Cmd）。*環境設定 > ホットキー* で自分のホットキーを設定できます。欄をクリックしてキーを押してください。Backspace でホットキーを解除します。
* **インターフェースをあなたの言語で**：ウィンドウのすべての文字列は言語ファイルから読み込まれます。12 の言語が同梱されており、新しい言語のファイルはプログラムが書き出してくれるので、それを翻訳できます。

  <details><summary>詳しく</summary>

  メインウィンドウのタイトルバー右端にある言語バッジが、インターフェースの言語（`EN`）を示します。クリックすると言語ウィンドウが開きます。言語は、プログラムの隣の `Languages` フォルダにあるテキストファイルです。同梱：English、Deutsch、Français、Italiano、Nederlands、Polski、
  Kaszëbsczi、हिन्दी、日本語、한국어、简体中文、繁體中文（それぞれ国旗付き。東アジアの言語には文字用に Noto Sans CJK のサブセットが、ヒンディー語には Noto Sans Devanagari が付属し、その結合文字や母音記号は rustybuzz テキストシェーパーで組まれます）。翻訳は機械翻訳で、構造は確認していますが、ネイティブスピーカーによる確認はしていません。修正を歓迎します。修正はどれもファイルの 1 行です。翻訳するには：言語の名前を入力して *言語ファイルを作成* を押します。プログラムが
  `Languages/<name>.lng` を書き出します。すべての文字列が英語で 1 行に 1 つ（`Button2.Caption = Calculate
  3D`）入っています。テキストエディターで `=` の右側の文字列を翻訳し、
  *再読み込み* を押すと、再起動なしでウィンドウがすぐに切り替わります。省いた行は英語のままです。*保存して終了* で、次回の起動時もその言語が使われます。

  </details>
* **Linux、Windows、macOS**（Intel と Apple Silicon）。
* ウィンドウの要らない作業すべてに使える **コマンドラインプログラム**。
* **とても大きな画像を 1 枚のまま**：15000 x 20000 ピクセル以上でもメモリ不足にならず、被写界深度はプロセッサのすべてのコアを使います（24 スレッドで 0.2.45 より前の
  11 倍の速さ、画像は同じ）。
* **16 ビット画像：** 1 色あたり 256 段階ではなく 65,536 段階の PNG ファイル。編集してもバンディング（階調の段差）が出ません。

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="16 ビット画像"></a>

  <details><summary>詳しく</summary>

  MB3D は 8 ビットの画像、つまり 1 色あたり 256 段階で保存します。見るだけなら十分ですが、あとから明るさ、コントラスト、トーンカーブなどを編集すると、背景やフォグのようななめらかな部分が目に見える帯に分かれてしまいます。**画像保存 ▸ PNG** には 2 つ目のファイル形式 **PNG 16 ビットカラー** があります。プログラムが計算したとおりの色を、
  256 段階に丸める前の状態で、すべての影、反射、被写界深度を含めて保存します。見た目は同じで、編集してもなめらかなままです。
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="コントラストを強く上げた後の背景のグラデーション：8 ビットでは段差、16 ビットではなめらか"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="「PNG 16 ビットカラー」がある保存ダイアログ"></a>

  </details>
* モンテカルロとその他すべての画像に使える **ノイズフィルタ**。GIMP でおなじみの
  DCT ノイズ除去です。モンテカルロレンダリングには、ニューラルフィルタ
  Intel Open Image Denoise も使えます。

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="ノイズフィルタ"></a>

  <details><summary>詳しく</summary>

  **ノイズフィルタ** は画像からざらつきを取り除きます。GIMP の
  *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT* にある **DCT ノイズ除去** で、
  GIMP のフィルタの中でこれらの画像に最も適していたものです（下のクレジットを参照）。
  
  * **モンテカルロ ウィンドウ ▸ -ノイズ**（MB3D では隠されていたボタン）：表示中の画像と保存する画像に、ピクセルあたりのレイ数に応じた強さでフィルタをかけます。レンダリング自体はフィルタなしで続き、次の更新でまたフィルタなしの画像が表示されます。
  * **ポストプロセス ▸ ノイズフィルタ**：*今フィルタ* はメインウィンドウの画像にフィルタをかけます。*新しい画像を自動的にフィルタ* は新しい画像すべてにかけます。
    *強さ* は取り除くノイズの強さです（4 は弱め。値を上げるとざらつきも細部も多く取り除かれます）。
  
  ざらつきが最も多いのはモンテカルロの画像です。ピクセルあたり 512 レイでレンダリングした画像と比べると、13 回のテストすべて（4 つのシーン、4〜32 レイ）で、フィルタをかけた画像のほうがかけない画像より近くなりました。4 レイで最大 23 %、8 レイで 5〜10 %、ざらつきがほとんど残らない 32 レイで 1〜3 % です。
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="ピクセルあたり 4 レイのモンテカルロ画像、同じ画像にノイズフィルタをかけたもの、512 レイのもの"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="ポストプロセス：ノイズフィルタの欄"></a>

  **ニューラルノイズフィルタ** は
  [Intel Open Image Denoise](https://www.openimagedenoise.org) です。Blender の
  Cycles をはじめ多くのパストレーサーで使われているフィルタで、レンダリング画像で学習されています。レンダリングのレイの平均値（露出と彩度を適用する前）にフィルタをかけるので、画像の明るさは保たれます。
  Intel のライブラリはプログラムに同梱されています（`oidn` フォルダ。.deb と
  .rpm では `/usr/lib/mb3d-rs/oidn`）。`MB3D_OIDN` で別のライブラリを指定できます。

  * **モンテカルロ ウィンドウ ▸ -ノイズAI**：*-ノイズ* と同じく、表示中の画像と保存する画像にフィルタをかけます。レンダリングはフィルタなしで続きます。
  * **コマンドライン**：`mb3d montecarlo scene.m3p --rays 8 --denoise-ai`。
  * 640 x 480 の画像で、プロセッサで 0.2〜0.6 秒かかります。

  512 レイの画像との差（全ピクセルの平均、小さいほど良い）：

  | シーン | 4 レイ | DCT フィルタ | ニューラルフィルタ | 8 レイ | DCT フィルタ | ニューラルフィルタ |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  4 つのシーンのうち 3 つでニューラルフィルタが最も近く、Julia 集合の細かなざらつきでは DCT フィルタのほうがわずかに近くなりました。ニューラルフィルタの効果が最も目立つのは、少ないレイで最もざらつきが残るガラスや鏡です。

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="ピクセルあたり 4 レイのモンテカルロ画像、DCT フィルタをかけたもの、ニューラルフィルタをかけたもの、512 レイのもの"></a>

  </details>
* 画像とアニメーションの **透明な背景**。

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="透明な背景"></a>

  <details><summary>詳しく</summary>

  画像とアニメーションのフレームは **透明な背景** で保存できます。フラクタルを別の画像に重ねたり、動画編集ソフトに取り込んだりするためのものです：
  **画像保存 ▸ PNG ▸ PNG 透明背景**、アニメーションメーカーでは **透明な背景**
  （PNG フレーム）。アンチエイリアス（表示倍率 1:2 以上、または `--aa`）を使うと縁がなめらかになり、元の背景色のにじみも残りません。合成用の深度画像は以前からありました（*Zバッファも保存*、`--depth`）。
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="背景付きの画像と、透明な背景をチェッカーボードの上に表示した画像"></a>

  </details>
* **アニメーションの動画：** プログラムから直接 AVI ファイルを作るか、小さな MP4 を作る ffmpeg コマンドを使えます。

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="アニメーションの動画"></a>

  <details><summary>詳しく</summary>

  アニメーションメーカーで **動画 (AVI) も作成** をオンにすると、フレームがレンダリングされた時点で、設定した **フレーム/秒** で動画にまとめます：出力フォルダのフレームの隣に `<project name>.avi` ができ、フレームも残ります。
  VLC や mpv で再生でき、動画編集ソフト（Kdenlive、Shotcut、DaVinci Resolve）で開けます。各フレームを 1 枚の画像として丸ごと保存するのでファイルは大きくなります。編集や保存には向いていますが、共有にはあまり向きません。
  
  共有用の小さな動画には、**FFmpegコマンド** が MP4（透明な背景の場合は WebM）を作るコマンドをクリップボードとメッセージ欄に入れます。[ffmpeg](https://ffmpeg.org) をインストールし、フレームができたらそのコマンドを端末に貼り付けると動画ができます。このプログラム自身が他のプログラムを起動することはありません。
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="動画 (AVI) も作成、フレーム/秒、FFmpegコマンドのあるアニメーションメーカー"></a>

  </details>
* 「開く」ダイアログでの **パラメータファイルのプレビュー**。

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="パラメータを開くときのプレビュー"></a>

  <details><summary>詳しく</summary>

  **開く ▸ m3p を開く** と **m3i を開く** では、クリックしたパラメータファイルを開く前に、その小さな画像がサイズや数式とともに表示されます。矢印キーでプレビューを見ながら一覧を移動できます。画像はバックグラウンドでプロセッサが計算するのでダイアログの反応は遅くならず、各ファイルのプレビューはプログラムの実行中は保持されます。
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="パラメータファイルのプレビューが表示された「開く」ダイアログ"></a>

  </details>
* **ナビゲーター：正確な値と視点のリセット。** 値をクリックして入力できます。ナビゲーターを開いたときの視点に戻せます。

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="ナビゲーター：正確な値と視点のリセット"></a>

  <details><summary>詳しく</summary>

  * ナビゲーターのパネル（Julia 値、数式の値、4D 回転、その他の値）で **値をクリック** し、
    MB3D と同じように正確な数値を入力できます（角度は度単位）。スライダーはその値から続きます。
  * ズームの刻みが粗すぎるときは **ズームをクリック** して入力できます（MB3D の
    issue リストにあった要望です）。入力したズームは保たれます（*ズームと歩幅を固定*）。
  * **視点をリセット** で、ナビゲーターがメインウィンドウから受け取った位置、方向、ズームに戻ります。調整した値はそのままです。
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="数式の値と、クリックした値の入力欄が表示されたナビゲーター"></a>

  </details>
* **新しい数式 SphereTree：** 接し合う球がどんどん小さく枝分かれしていく木。
  Tom Lowe によるものです。

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="新しい数式：SphereTree"></a>

  <details><summary>詳しく</summary>

  **SphereTree** は Mandelbulb3D にはない数式です。互いに接する球が、泡やサンゴのようにより小さな球へと枝分かれしていく木です。Mandelbox の考案者である Tom Lowe が 2020 年に発表しました（下のクレジットを参照）。数式リストの dIFS 数式の中にあり、
  4 つの設定があります。
  
  * **PackRatio**（0〜1.22）：球の大きさ。1 でちょうど接し、1 を超えると外側の球が離れます。
  * **Levels**：木の深さ（ほとんどの画像では 6 で十分です）。
  * **Rotated**：1 にすると各レベルを 12 分の 1 回転させ、木は球の房でできた丸いボールになります。
  * **Colour scale**：木の枝ごとに色が付きます。ライティングの色の設定で
    *第2選択*（オービットトラップ）を選ぶと見えます。この設定は色をグラデーション全体に広げます。
  
  単独でも dIFS 変換の後でも使え（3 枚目の画像は *tilingbrickIFS* で繰り返したもの）、グラフィックカードでもプロセッサでも計算されます。反復回数は 1 にしてください。この数式は木全体を一度に計算します。SphereTree を使ったパラメータはこのプログラムでのみ開け、Mandelbulb3D では開けません。
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="モンテカルロレンダラーで夕暮れの光に照らされた SphereTree のボール（Rotated）"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="SphereTree の球の泡のクローズアップ"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="tilingbrickIFS で繰り返した SphereTree：球の木の群れ"></a> |
  |:---:|:---:|

  </details>
* **新しい数式 DomeTree：** ドームから小さなドームの輪が生え、そのそれぞれにまた輪が生えます。Tom Lowe によるものです。

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="新しい数式：DomeTree"></a>

  <details><summary>詳しく</summary>

  **DomeTree** は SphereTree に続く Tom Lowe の 2 つ目の木です（クレジットを参照）。ボールの片側からドームの輪が生え、各ドームにはさらに小さな輪が生え、それがレベルごとに続きます。円反転でできています。数式リストの dIFS 数式の中にあり、5 つの設定があります。

  * **Levels**：木の深さ（16 で十分です）。
  * **Children**：各輪のドームの数、4〜10。
  * **Bend angle**（0.3〜1.3 が最適）：ドームの膨らみ具合。小さい値では平らな円盤、大きい値では丸いドームになります。約 1.3 を超えると表面がざらつきます。
  * **Colour scale**：ドームごとに色が付きます。ライティングの色の設定で *第2選択*
    （オービットトラップ）を選び、色の範囲を広く（たとえば 0〜100）すると見えます。
  * **Start curved**：1（既定）はボールから、0 は最初の輪が生える平らな床から始めます。

  小さな *レイステップ倍率*（0.2。子が 9 や 10 なら 0.05）を使ってください。TGlad 自身の
  Fragmentarium ファイルと同様、この形の距離推定は大まかです。SphereTree と同じくグラフィックカードでもプロセッサでも動き、反復は 1 回で済み、これを使ったパラメータはこのプログラムでのみ開けます。

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="モンテカルロレンダラーで描いた子 8 個の DomeTree、ドームごとに色分け"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree、子 5 個、Bend angle 1.2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree、子 6 個、Bend angle 0.5：平らなドーム"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree、子 10 個：ドームの間のレース模様と大きなボール"></a> |
  |:---:|:---:|:---:|

  </details>
* **新しい数式 UlamSpiral：** 素数のウラムの螺旋を 3D にしたもの。街、スカイライン、球、ブロックの形で表します。

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="新しい数式：UlamSpiral"></a>

  <details><summary>詳しく</summary>

  1963 年、数学者スタニスワフ・ウラムはある会議で「長くてとても退屈な発表」を聞かされながら、方眼紙に落書きをしていました。最初はチェスの問題を作ろうと考えていましたが、代わりにマス目に 1、2、3、… と螺旋状に番号を振り（中央が 1）、素数に丸を付けました。すると素数は、まるで規則があるかのように斜めの線の上に並びました。コンピュータで何百万個も描いても同じように並びます（ウラムはロスアラモスの MANIAC II で
  100,000 個を試し、マーティン・ガードナーは 1964 年にその図を Scientific American の表紙に載せました）。今日に至るまで、誰も完全には説明できていません。
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="ウラムの螺旋：正方形の螺旋上の 1〜78,961 のうちの素数。隅に螺旋の始まりを描いたもの"></a>
  
  **UlamSpiral** はこの螺旋を 3D で作ります。1 つの数に 1 つの形を当て、4 つの形（*Mode*）があります。
  
  * **0、素数の塔**：すべての数に板を置き、素数の上に塔を立てます。素数の斜めの線が塔の列になります。
  * **1、約数のスカイライン**：すべての数に柱を立て、約数が多いほど高くなります。素数は柱の間の低い通りになります。
  * **2、約数の球**：すべての数に球を置き、約数が多いほど大きくなります（素数は小さな球です）。
  * **3、素数のブロック**：数を正方形ではなく立方体の殻に巻き付け、素数ごとに立方体を置きます。
  
  *Rings* は螺旋の大きさ（60 周で 14,641 までの数）、*Height* は塔や柱の高さ、
  *Gap* はその間隔です。色の設定で *第2選択*（オービットトラップ）を選ぶと、色が約数の数に従うので、素数には独自の色が付きます。SphereTree と同じくグラフィックカードでもプロセッサでも動き、dIFS 変換の後でも使え、反復は 1 回で済み、これを使ったパラメータはこのプログラムでのみ開けます。
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral の素数の塔：素数の斜めの列を街として、フォグ、ハードシャドウ、被写界深度付きのモンテカルロレンダラーで描いたもの"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral、約数のスカイライン"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral、約数の球"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral、立方体の殻の上の素数のブロック"></a> |
  |:---:|:---:|:---:|

  </details>
* **停止したレンダリングは中断したところから続きます。** レンダリングが計算した分は、停止したときだけでなく、プログラムを閉じたとき、クラッシュしたとき、コンピュータの電源が切れたときにも保持されます（5 分ごとにディスクに保存されます）。対象は、
  3D計算、モンテカルロ（パスの途中でも）、アニメーションのフレームやバッチ処理中の画像、ボクセル書き出し、Bulb Tracer のメッシュ、大判レンダリングのタイル、そしてスウォームのレンダリングです（ジョブを作ったコンピュータで停止すると完成した部分はすべて保持され、各コンピュータは作業中だった部分を保持します）。同じパラメータの
  3D計算とモンテカルロは続けるかどうかを尋ね、その他は自動的に続きます。**再開**
  ボタン（開く タブ）で未完了のレンダリングが一覧表示され、続けるか削除するかを選べます。コマンドラインでは同じコマンドで自動的に続きます（`--tiles` の画像、
  `montecarlo`、`animate`、`batch`、`voxel`、`mesh` も）。`--fresh` で最初からやり直し、
  `mb3d
  resume` で一覧を表示します。続きから再開したレンダリングは、一度に計算したものと同じ結果になります。
* **複数のコンピュータでレンダリングを分担**する「スウォーム」：1 つの共有フォルダが見えるコンピュータどうしで、画像、大判レンダリング、モンテカルロ画像、アニメーションを一緒にレンダリングします。
  [複数のコンピュータでのレンダリング](#rendering-on-several-computers-the-swarm)を参照してください。
* **新しい数式 PseudoKleinian：** ホール、アーチ、球でできた「クライン建築」を、独自の正確な距離を持つ 1 つの数式にしたもの。

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="新しい数式：PseudoKleinian"></a>

  <details><summary>詳しく</summary>

  「クライン建築」として知られるホール、アーチ、柱、入れ子の球は、3D フラクタル画像の中でも特に愛されているものです。MB3D では、2011 年に Theli-at が見つけた手順に従い、慎重に合わせた設定で複数の数式のハイブリッドとしてしか作れません。その後 knighty が同じものを独自の距離推定を持つ 1 つの数式として書きました。それが
  **PseudoKleinian** です。各レベルで点を箱に折りたたみ、球で反転し、オフセットを加えます。距離は折りたたみの後の床面まで正確に測られるので、壁がきれいに出て、画像も速くレンダリングされます。設定は knighty のものです：*Levels*（8）、*Size*（反転の球）、
  *CSize X/Y/Z*（箱。彼の 0.924、0.908、0.924 で古典的なホールになり、少し変えると新しい建物になります）、*Julia X/Y/Z*（レベルごとのオフセット）、*Shape*（0 は床面、
  1 は *Offset* を通る厚さ *Thickness* の円柱面、2 は角の丸い箱）、*DE offset*
  （表面をくり抜く）、*Colour scale*（*第2選択*、つまりオービットトラップと組み合わせると、軌道の最小半径による古典的なクライン風の縞模様）。
  
  この構造は空間全体を埋めるので、内側から眺めることになります。視点の中心を原点の近くに、カメラを 1〜2 単位後ろに置いて始め、ナビゲーターでホールの中を移動してください。モンテカルロレンダラーでは室内に空の光がほとんど届かないので、ホールの中に位置ライトを置くか（1 枚目の画像）、カット面で構造を切り開いて上から覗いてください（2 枚目）。グラフィックカードでもプロセッサでも動き、反復は 1 回で済み、dIFS 変換の後でも、他の dIFS 数式と同じように DE の組み合わせでも使えます（ただし空間を埋める構造の中ではあまり変わりません）。これを使ったパラメータはこのプログラムでのみ開けます。
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="knighty の視点から見た PseudoKleinian：中のランプに照らされたアーチと入れ子の球のホール。フォグ、ハードシャドウ、被写界深度付きのモンテカルロレンダラーで描いたもの"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="カット面で切り開いて上から見た PseudoKleinian：球の格子とその間のセル"></a>

  </details>
* **ステレオペアを 1 回で**：左右両方の目の画像をレンダリングし、平行法または交差法で見られるように並べて表示します。Rathinagiri S
  （[SRathinaGiri](https://github.com/SRathinaGiri)）による貢献です。

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="平行法で見るために並べた Dainbramage の Hydra のステレオペア"></a>

  <details><summary>詳しく</summary>

  **ステレオ ▸ ステレオペアを描画** は、タブのステレオ設定で左目、続いて右目の画像をレンダリングし、メインウィンドウに並べて表示します。平行法向けに配置されています（左の画像が左）。**交差法プレビュー** で左右を入れ替えて交差法で見られます。これはプレビューです：ライティングや色は、レンダリングし直さないと後から変えられず、タイルでは使えません。片目ずつの画像のボタンも残っています。
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="平行法で見るために並べた Dainbramage の Hydra のステレオペア"></a>

  </details>
* **画像全体の残り時間**：画像の下のバーは計算全体を通して 1 回だけ満ちていき、その横に、影、オクルージョン、反射を含めて画像が完成するまでの残り時間が表示されます。

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="進行状況バーと全体の残り時間"></a>

  <details><summary>詳しく</summary>

  MB3D のバーは段階ごとに最初からやり直します。ここでは計算全体を示します：メインのレンダリング、続いてハードシャドウとアンビエントオクルージョン、反射、被写界深度です。メインのレンダリングの後の段階にかかる時間は、シーンが必要とする処理とメインのレンダリングの進み具合から見積もられます。この見積もりは 366 シーンの約 1400 回のレンダリングで調整されました。もう一度計算する画像では、前回かかった時間が見込まれます。
  
  グラフィックカードで新しい数式を使う最初の画像は、シェーダーのコンパイルを待ちます。そのとき行には残り時間とともに *シェーダーをコンパイル中* と表示されます。残り時間はプロセッサの進み具合をもとにし、シェーダーが間に合う場合はカードが加わることも考慮します。詳しくは
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar) を参照してください。
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="ハードシャドウとアンビエントオクルージョンの計算中のメインウィンドウと全体の残り時間"></a>

  </details>
* タイトルバーの時計アイコンの奥にある **History**：計算したすべての画像とそのサムネイル、開いたり保存したりしたすべてのファイルを、リスト、行、または任意の大きさのタイルで、日ごとにまとめて、または通しで表示します。検索、並べ替え、異なる画像だけの表示ができ、どの項目もダブルクリックで読み込めます。何十万件もの項目がある履歴でもすぐに開きます。
* ハードシャドウと反射の後も **ライティングはその場で反映されます**：ライトを動かしたり色を変えたりすると、Mandelbulb3D と同じように画像がすぐに変わります。

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="ライティングを変更した直後のメインウィンドウとライティング ウィンドウ"></a>

  <details><summary>詳しく</summary>

  まずは普通にレンダリングし、それからライトの位置や色を試してください。ライティング ウィンドウでの変更はどれも、Mandelbulb3D と同じように、計算済みの影を使ってすぐに描き直されます。ライトの色や強度を変えても、影が再計算されることはありません。動かしたライトの影、反射、被写界深度は、Mandelbulb3D と同じようにポストプロセス ウィンドウのボタンで元に戻せます。
  
  停止した計算は、ライティングを変えた後でも続けられます。これを使った作業の進め方：
  wiki の [ライティングの作業手順](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-ja)。詳しくは
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field) を参照してください。
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="ライティングを変更した直後のメインウィンドウとライティング ウィンドウ：画像はもう明るくなっている"></a>

  </details>

<a id="download"></a>

## ダウンロード

最新リリースの、すぐに実行できるプログラムです。インストールは不要です。

| システム | ダウンロード |
|---|---|
| Windows（64 ビット） | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux（64 ビット） | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian、Ubuntu、Mint（他のプログラムと同じようにインストール） | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora、RHEL、Rocky、openSUSE（他のプログラムと同じようにインストール） | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS、Apple silicon（M1 以降） | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS、Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

展開して、展開したフォルダ内の **Mandelbulb3D**（Windows では `Mandelbulb3D.exe`）を起動します。フォルダは MB3D のインストールと同じ構成で、MB3D の数式、マップ、
80 個のサンプルパラメータファイルが入っています。

* Windows に「Windows によって PC が保護されました」と表示されたら、*詳細情報* ▸
  *実行* をクリックします。
* macOS は署名のないダウンロードしたプログラムをブロックします。展開したフォルダで
  `xattr -dr com.apple.quarantine .` を一度実行してください。

**Linux と macOS の別の方法：** 手でダウンロードする代わりに、次の 1 行で最新リリースを
`~/mb3d-rs` に取得できます（後でもう一度実行すると更新されます。スクリプト
[download.sh](../../download.sh) はプログラムのフォルダにも入っています）。

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

展開する前にダウンロードを検証し、**Mandelbulb3D** をアプリケーションメニューに追加し（Linux）、どのフォルダでも入力できる `Mandelbulb3D` と `mb3d` コマンドを作ります。`sh ~/mb3d-rs/download.sh
--uninstall` でメニュー項目とコマンドを再び取り除きます。あなたのファイルは残ります。

Linux 版のプログラムは、2018 年ごろ以降のディストリビューション（Ubuntu 18.04、
Debian 10、Rocky/RHEL 8 以降）で動作します。

**他のプログラムと同じようにインストールする場合：** Debian、Ubuntu、Mint では上の
`.deb` ファイルを開きます（または `sudo apt install ./mb3d-rs-linux-x64.deb`）。
Fedora、RHEL、Rocky では `.rpm` ファイルを使います（`sudo dnf install
./mb3d-rs-linux-x64.rpm`、openSUSE では `sudo zypper install
./mb3d-rs-linux-x64.rpm`）。これで Mandelbulb3D がアプリケーションメニューに入り、`mb3d` がどのフォルダでも使えます。設定と History は `~/.config/mb3d-rs` と
`~/.local/share/mb3d-rs` に保存されます。

[はじめに](../../GETTING_STARTED.md) で最初の手順を説明しています。以前のバージョンとリリースノートは
[リリース](https://github.com/thr32423/mb3d-rs/releases) のページにあります。

<a id="guides-and-help"></a>

## ガイドとヘルプ

手順ごとのガイドは [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-ja) にあります。

* [インストール](https://github.com/thr32423/mb3d-rs/wiki/Installing-ja)：Windows、macOS、Linux、更新、ファイルの保存場所
* [ライティングの作業手順](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-ja)：まず普通にレンダリングし、それからライト、影、反射を試す
* [スウォームの設定](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-ja)：共有フォルダを通して複数のコンピュータでレンダリングする
* [トラブルシューティングと FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-ja)：思ったとおりに動かないとき

すべてのリリースとその変更点：[一覧](../UPDATES.md)。

<a id="rendering-on-several-computers-the-swarm"></a>

## 複数のコンピュータでのレンダリング（スウォーム）

1 つの共有フォルダが見えるコンピュータどうしで、一緒にレンダリングします。対象は、タイルに分けた画像、大判レンダリング、モンテカルロ画像、アニメーション、バッチ処理です。各コンピュータは他の誰も受け持っていない部分を取るので、速いコンピュータほど多くを受け持ち、レンダリングを始めたコンピュータが結果をまとめます。共有はフォルダが担います。プログラムはネットワーク接続を開かず、誰とも通信しないので、安全性はあなたのファイル共有と同じです。

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="ツール タブの Swarm ボタンで開いた Swarm ウィンドウ。2 台のコンピュータで画像をタイルに分けてレンダリング中：共有フォルダ、このコンピュータを共有、コンピュータとそれぞれの作業内容、ジョブ、レンダリング済みのタイル・作業中のタイル・未着手のタイルを示すプレビュー"></a>

<a id="setting-it-up"></a>

### 設定

確認すべき点を含めた手順：wiki の [スウォームの設定](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-ja)。

1. **すべてのコンピュータが書き込めるフォルダ：** NAS の共有、Windows の共有、
   Samba の共有、または NFS。Linux ではフォルダとしてマウントしてください。たとえば
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`
   です。ファイルマネージャーで開いただけの共有（`smb://...`）は、プログラムが使えるフォルダではありません。
2. **すべてのコンピュータに同じバージョンの mb3d-rs。** 別のバージョンのコンピュータは一覧に表示されますが、参加しません。
3. **Mandelbulb3D で：** ツール タブの **Swarm** ボタン。共有フォルダを選び（どのコンピュータでも同じ共有。パスは違っていてもかまいません）、一覧に表示するコンピュータの名前を付け、**このコンピュータを共有** にチェックを入れます。

これだけです。ステータス行に状況が表示されます。たとえば *omarchy と共有中:
レンダリングは 2 台のコンピュータで分担されます*。スウォームに自分しかいない場合は、いつもどおりすべてこのコンピュータでレンダリングされます。

<a id="what-is-shared"></a>

### 分担されるもの

*このコンピュータを共有* にチェックが入っている間、このコンピュータは他のコンピュータがフォルダに置いた部分をレンダリングし、他のコンピュータがスウォームにいるときは、自分のレンダリングもスウォームに回します。

* **3D計算**（とバッチ処理 ウィンドウ）— 画像を N x M 個のタイルに分け（Swarm
  ウィンドウで設定）、ここでまとめます。通常の計算と同じく、ライティングの変更、ポストプロセス、m3i としての保存（とそのメモリ。下記参照）がすべて使えます。
* **大判レンダリング** — タイルは届いた順に 1 枚ずつ保存されます。画像全体がメモリに置かれることはないので、100000 x 100000 ピクセル以上の画像にはこの方法を使います。
* **モンテカルロ** — 1 台のコンピュータの場合と同じです。停止を押すまで各コンピュータが自分のブロックにパスを加え、画像は全員のレイできれいになっていき、停止でブロックが平均されます。ウィンドウのバッチモードでは、ブロックの合計がバッチのレイ数に達したところで止まります。
* **アニメーションメーカー** — フレームを 1 枚ずつ。

ウィンドウには、コンピュータとそのコア、グラフィックカード、それぞれの作業内容、ジョブとその進行状況、最新のジョブのプレビューが表示されます。プレビューには、レンダリング済みのタイル、進行状況バー付きの作業中のタイル、まだ残っているタイルが表示されます。

<a id="on-the-command-line"></a>

### コマンドラインで

手伝わせたいすべてのコンピュータで、フォルダを指定してワーカーを起動します。

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

その後、どのコンピュータでも、`--swarm` と同じフォルダを指定してレンダリングします。

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` で、スウォームに誰がいるか、各コンピュータが何をしているか、ジョブがどこまで進んだかがわかります。コマンドラインのモンテカルロ画像は、それぞれ独自の乱数のレイを持つブロックに分けられます。ピクセルあたり 64 レイのブロック 4 つで、
`--rays 256` で指定した 256 になります。

<a id="good-to-know"></a>

### 知っておくと便利なこと

* **他のコンピュータが何もしないまま** で、このコンピュータがレンダリングしている間もそのジョブ一覧が空の場合：Samba でフォルダを公開し、自分でも直接そのフォルダを使っているコンピュータが、他のコンピュータに古いフォルダ一覧を保持させています。
  `/etc/samba/smb.conf` でそのキャッシュを無効にしてください：`[global]` に
  `smb3 directory leases = no`、共有のセクションに `oplocks = no` と
  `level2 oplocks = no` を設定し、`sudo systemctl restart smb` を実行してから、他のコンピュータで共有をマウントし直します。
* **メモリ：** このプログラムでは、画像 1 枚にピクセルあたり約 20 バイト（画像バッファ）、メインウィンドウでさらに 10 バイト、それに加えて 2 つの段階のうち大きいほうが必要です。1 つは影とオクルージョンを含むメインの計算（ピクセルあたり 16 バイトと、カード用に 1.2 GB）、もう 1 つは描画（ピクセルあたり 3 バイト、被写界深度があればさらに 20 バイト、そして反射の帯 1 つ分：最大 3.5 GB。帯の大きさはパスの開始時の空きメモリの半分に合わせるので、空きメモリが少なければ小さくなります）です。
  12000 x 6000 の画像で 6 GB です。画像がコンピュータの空きメモリ（Linux、Windows、
  macOS に問い合わせます）より多くを必要とする場合、3D計算は必要な量と空きの量を示し、始める前に確認します：計算がスワップを起こし、プログラムが何分も応答しなくなることがあるためです。解決策は大判レンダリングです（単独でも、スウォームでも）。タイルは
  1 枚ずつ保存され、画像全体が保持されることはありません。スウォームでの 3D計算は、通常の計算と同じく画像をこのコンピュータで保持します（ただしメインの計算の段階は除きます。これはタイルを受け持つコンピュータが、タイルごとのメモリで行います）。スウォームのコンピュータは、自分のメモリに対して大きすぎるタイルを他に任せます。
* **安全性：** コンピュータはジョブが求めるものだけをレンダリングし、ジョブのフォルダにだけ書き込みます。ジョブに含まれる数式コードは、すべての数式と同じく数式エミュレーターの中で実行されます。
* **速度：** タイルは画像バッファのまま丸ごと受け渡され、とても大きな画像では数百メガバイトになるので、速い共有ほど効果があります。遅れてコピーする同期サービスでも動きますが、遅くなります。
* **いなくなったコンピュータ：** あるコンピュータがレンダリングしていた部分は、そのコンピュータから 30 秒間応答がないと他のコンピュータが引き継ぎます。持ち主が
  30 分間いなくなったジョブは、それを見つけたコンピュータが削除します。あるコンピュータで失敗したジョブはウィンドウに記録され、他のコンピュータに任されます。
* **停止：** *このコンピュータを共有* のチェックを外すと、作業中の部分を先に仕上げます。プログラムを閉じるとその部分を手放し、他のコンピュータが引き継ぎます。
* **ジョブの停止**（ジョブを作ったコンピュータで停止を押す、プログラムを閉じる、またはクラッシュする）：完成済みの部分はすべてそのコンピュータに保持され、各コンピュータはレンダリング中だった部分を保持します。同じレンダリングをもう一度始めると、足りない部分だけのジョブが作られ、途中まで進めていた部分を再び受け取ったコンピュータはそれを続けます。コマンドラインでも同じで、同じコマンドをもう一度実行します。終わりを決めないモンテカルロレンダリング（ウィンドウのもの）は、これまでどおり停止したときにブロックをまとめます。

<a id="how-the-rendering-works"></a>

## レンダリングの仕組み

フラクタル画像は 1 ピクセルずつ計算されます。ピクセルごとに 1 本のレイをシーンの中へたどり、フラクタルの表面に当たるまで進めます。その各ステップで数式が何十回も評価されます。MB3D はこれをプロセッサで、すべてのコアを使って行います。mb3d-rs も同じことができ、さらにこの作業を **グラフィックカード** に任せることもできます。グラフィックカードには何千もの小さなプロセッサがあり、まさにこの種の作業のために作られています。

**「計算デバイス」** の設定（メインウィンドウの 内部 タブ）で選びます。

| | |
|---|---|
| **自動**（既定） | カードとプロセッサが画像を分担します。カードは上から、プロセッサのスレッドは下から行を取り、出会うところまで進みます。影とアンビエントオクルージョンも同じように分担されるか、カードが明らかに速い場合はカードだけで行われます。 |
| **CPU** | MB3D と同じく、プロセッサだけ。 |
| **グラフィックカード** | カードだけ（カード用のプログラムの準備ができるまで待ちます）。 |

ある数式の組み合わせを初めて使うとき、グラフィックドライバーはカード用のプログラム
— *シェーダー* — を用意する必要があります。下で測定したコンピュータでは 10〜15 秒かかります。自動モードではプロセッサがそれを待たずに始め、準備ができたらカードが加わるので、小さな画像が待たされることはありません。用意されたシェーダーは次回のために保存されるので（グラフィックドライバーによって。Linux ではプログラムの隣の
`ShaderCache` フォルダ）、同じ数式を使う次の画像はすぐにカードで始まります。

ウィンドウタイトルの右にある **LED** が、直前の画像がどこで計算されたかを示します：緑はグラフィックカード、オレンジは一部がカード、赤はプロセッサです。マウスを合わせると詳細が表示されます。

[レンダリングの仕組み](../RENDERING.md) では、画像を使って詳しく説明しています：初回に自動モードが何をするか、カードが常に速いとは限らない理由、LED の色の意味。

すべてをカードに任せられるわけではありません。カード用に変換できない数式がいくつかあり、非常に深くズームした視点は倍精度で計算されます（倍精度を扱えるカードは一部だけです）。これらはすべてプログラムが自動的に判断します。目に見えるのは LED だけです。

<a id="speed"></a>

## 速度

すべての測定は 1 台のコンピュータで行いました：AMD Ryzen 9 3900X（12 コア、24
スレッド）と NVIDIA GeForce RTX 2060、Linux です。オリジナルの
**Mandelbulb3D 1.99sr37** は同じマシンの Wine 上で実行し（その Windows 版プログラムはそこで変更なしに、全速力で、すべてのコアを使って動きます）、
*3D計算* のクリックから画像の完成までを計測しました。各パラメータファイルは、3 つすべてで同じ画像サイズでレンダリングしました：
MB3D、プロセッサのみの mb3d-rs（**CPU**）、**自動** モードの mb3d-rs
（グラフィックカードとプロセッサの併用）です。自動の時間は、カードのシェーダーがすでに用意された状態での、各ファイルの 2 回目のレンダリングのものです。新しい数式の組み合わせの最初のレンダリングでは、このカードではシェーダーの準備に 10〜15 秒が加わります。

<a id="mb3ds-80-example-parameter-files"></a>

### MB3D の 80 個のサンプルパラメータファイル

MB3D に付属する（そしてこのプログラムの `M3Parameter` フォルダにも入っている）サンプルファイルを、それぞれ固有の画像サイズでレンダリングしました。

MB3D はファイルのうち 1 つ（*Test8*：「Division by zero」エラーで止まります）を計算できなかったので、比較は 79 ファイルで数えています。

* **プロセッサだけの mb3d-rs は MB3D より速く**、79 ファイル中 71 ファイルで中央値 **1.7 倍** でした。全ファイルの合計は、MB3D が 18 分 21 秒、プロセッサだけの mb3d-rs が 17 分 10 秒でした。
* **自動モード** は 79 ファイル中 77 ファイルで MB3D より速く（中央値 2.1 倍）、全ファイルの合計は 8 分 17 秒でした。30 ファイルでは、カードの出番が来る前にプロセッサだけで計算が終わりました。ほとんどは数秒しかかからないためです。その場合、自動はプロセッサにわずかなオーバーヘッドが加わっただけのものです。カードが加わったのは 49 ファイル、時間のかかるほうのファイルで、そこでは画像がプロセッサより中央値 1.6 倍速くなりました（最大 13 倍：*Insect Menger*、63.0 秒の代わりに 4.7 秒）。
* 4 ファイルでは、自動がプロセッサだけの場合より目に見えて遅くなりました（*Ellarien - Shrooms are coming*、*MengerTrees*、*toricaleggs*、*TreePlanet*）。これらのシーンでは、カードが 24 スレッドのプロセッサより遅いためです。
* 「カードの行」は、自動モードでグラフィックカードがレイマーチングした画像の割合です（プロセッサだけで終わった場合は –）。

<details>
<summary>全 80 ファイル（クリックで開く）</summary>

| パラメータファイル | MB3D (Wine) | mb3d-rs CPU | mb3d-rs 自動 | カードの行 | 自動 対 MB3D |
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
| Test8 | MB3D でエラー | 0.1 s | 0.3 s |  | – |
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

画像は同じです。プロセッサでの mb3d-rs は丸め誤差の範囲で MB3D と同じ画像を作り、自動の画像とプロセッサの画像の差は、平均で色の範囲の 1 % にも満たないものです（細かな部分のきらめきが別のピクセルに出ることがあります。測定結果は
[技術資料](../TECHNICAL.md) にあります）。

<a id="the-command-line"></a>

## コマンドライン

`mb3d` はウィンドウなしでレンダリングします。プログラムのフォルダで端末を開いてください（Windows では展開したフォルダでコマンドプロンプトを開き、`./mb3d` の代わりに `mb3d` と入力します）。Linux や macOS で 1 行のインストールを使った場合、`mb3d` はどのフォルダでも使え、`man mb3d` でマニュアルが表示され、bash、zsh、fish では Tab キーでオプションが補完されます。

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

その他の機能にはそれぞれサブコマンドがあり、各サブコマンドに `--help` があります。

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

数式とマップはプログラムの隣から読み込まれます。別の MB3D インストールの数式とマップは
`--formulas` と `--maps` で指定できます。アニメーションは MB3D のものです：MB3D の補間によるキーフレーム、`.m3a`
プロジェクトの読み込みと保存、そして MB3D と同じ方法でのフレームの書き出しにより、中断したアニメーションを続けたり、複数のコンピュータで 1 つのアニメーションを分担したりできます。

<a id="good-to-know-1"></a>

## 知っておくと便利なこと

* **同じ画像ですが、ビット単位で同一ではありません。** MB3D は古い x87 演算の 80 ビット精度で計算していました。このプログラムは今日の 64 ビットを使います。画像はあらゆる構造が一致しますが、個々のピクセルの色が最後の桁で異なることがあります。モンテカルロレンダリングと MutaGen は独自の乱数を使うので、結果は MB3D のものとは異なります。
* **グラフィックカードの画像** はプロセッサの画像とわずかに異なります（単精度）。非常に細かい部分では、きらめきが別のピクセルに出ることがあります。深いズームでは自動的に倍精度に切り替わります。
* **プロセッサでの速度** は MB3D に近いものです：MB3D の `.m3f` ファイルにある数式の機械語は、どのプロセッサでも（Apple Silicon でも）動くように、わずかなコストで変換されています。レイマーチング自体はより高速です。上の表を参照してください。
  Linux と Windows では、画像の開始時にシーンのハイブリッドの数式もまとめて 1 つの関数にコンパイルされ（Cranelift で数ミリ秒）、数式どうしがプロセッサのレジスタで値を受け渡せるようになります。数式のループの高速化（全システム共通）と合わせて、80 個のサンプルファイルの CPU
  レンダリングは v0.2.74 より 20 % 短い時間で終わり（一部では最大 60 %）、画像はビット単位で同じです。シーンがコンパイルしたハイブリッドを使い続けるのは、測定して速かった場合だけです。`MB3D_NO_JIT=1` で無効にできます。macOS でも有効ですが、まだテスト中です：展開したフォルダで `./jit_check.sh` を実行すると確認できます（コンパイルした数式と変換した数式を比較し、サンプルファイルをコンパイルあり・なしでレンダリングします。画像は同じでなければなりません）。
* **大きな画像** は 1 枚のままでも（15000 x 20000 の画像には約 15 GB のメモリが必要で、被写界深度ありでは何時間もかかります）、タイルに分けても（複数のコンピュータでも可。
  `--tiles` と 大判レンダリング ウィンドウ）レンダリングでき、タイルのほうが必要なメモリは少なくなります。3D計算は、空きメモリに収まらない画像を始めず（空きメモリがわかる Linux の場合）、その旨を表示します。スウォームのコンピュータは、自分のメモリに対して大きすぎるタイルを他に任せます。
* **未完了のレンダリング** はキャッシュフォルダ（`~/.cache/mb3d-rs/resume`、
  Windows では `%LOCALAPPDATA%\mb3d-rs\resume`、またはプログラムの隣）に保存されます：画像ではピクセルあたり 18 バイト（15360 x 15360 の画像で 4.2 GB）、モンテカルロレンダリングでは 64 バイトです。計算した分はすぐに書き込まれ、5 分ごとにディスクへ確実に保存されます。開始から 10 秒以内に停止したレンダリングは何も残しません。
  30 日より古い未完了のレンダリングは削除され、全体で 50 GB を超えると古いものから削除されます。レンダリングを続けられるのは、パラメータとプログラムのバージョンが同じ場合だけです（カードかプロセッサかの選択も同じである必要があります。モンテカルロレンダリングの露出と彩度は変えてもかまいません）。モンテカルロレンダリングは、レイ数（バッチ、`--rays`、`--passes`）に達するか、再開の一覧で削除されるまで未完了のままです。
* **新しい数式や自作の数式、マップ、テーマ、言語** は
  `~/.local/share/mb3d-rs/M3Formulas`、`.../M3Maps`、`.../Themes`、`.../Languages` に置きます（インストール方法は問いません。Windows では `%APPDATA%\mb3d-rs\...`）。これらはプログラムに付属するものに加えて使われ、同じ名前の付属ファイルを置き換えるので、フォーラムで更新された数式も、プログラムのフォルダに手を付けずに使えます。

<a id="updates"></a>

## 更新履歴

すべてのリリースとその変更点は [UPDATES.md](../UPDATES.md)（英語）にあります。

<a id="for-developers"></a>

## 開発者向け

[技術資料](../TECHNICAL.md) では、グラフィックカードの使い方、MB3D の Delphi
ソースから何をどのように移植したか、MB3D の数式の機械語を実行する 3 つの方法、シーンファイルの形式とそのキー、MB3D との照合、そしてソースからのビルド方法を説明しています。

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## ライセンス

MB3D は LGPL 2.1 でライセンスされています。この移植版はその派生物であり、同じライセンスを使用しています。

<a id="credits"></a>

## クレジット

mb3d-rs が基にしている人々とプロジェクトは、[英語版 README](../../README.md#credits) に記載されています。
