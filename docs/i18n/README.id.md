<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D untuk Linux, Windows, dan macOS">

# mb3d-rs — Mandelbulb3D untuk Linux, Windows, dan macOS
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · **Bahasa Indonesia** · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Português](README.pt.md) · [Kaszëbsczi](README.csb.md) · [العربية](README.ar.md) · [اردو](README.ur.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*Diterjemahkan dari README bahasa Inggris (versi 0.2.86). Jika keduanya berbeda, [README bahasa Inggris](../../README.md) yang benar.*

**Unduhan: [rilis terbaru](https://github.com/thr32423/mb3d-rs/releases/latest)** · [Unduh](#download) · [Apa yang baru](../UPDATES.md)

mb3d-rs adalah [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D),
program fraktal 3D, yang ditulis ulang agar berjalan di komputer masa kini:
di **Linux, Windows, dan macOS**, dengan gambar yang dihitung di
**kartu grafis**, sedangkan MB3D hanya bisa memakai prosesor. Program ini
membuka file parameter MB3D (`.m3p`, `.m3i`, parameter teks dari forum),
memakai formula dan map MB3D, dan menghasilkan gambar yang sama.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="Jendela utama"></a>

Isinya dua program:

* **Mandelbulb3D** — program desktop, dengan jendela-jendela MB3D sendiri:
  jendela utama, Formula, Pencahayaan, Pascaproses, Navigator, Animasi,
  Batch, Render besar, Monte Carlo, Ekspor voxel, Bulb Tracer 2, MutaGen,
  dan lainnya. Jika Anda mengenal MB3D, Anda mengenal program ini; tutorial
  MB3D tetap berlaku.

  <details><summary>Selengkapnya</summary>

  Jendela-jendelanya adalah milik MB3D sendiri — dibuat dari definisi
  jendela asli MB3D, sehingga setiap tombol, slider, dan petunjuk berada di
  tempat yang sama seperti di MB3D:
  
  * **Jendela utama:** membuka dan menyimpan parameter (`.m3p`, `.m3i`,
    teks, clipboard), menyimpan gambar (PNG, JPEG, BMP, Z-buffer 16 bit),
    mode mouse, tombol rotasi, ukuran gambar, preset kualitas, undo,
    *Hitung 3D* dengan gambar yang terus tumbuh dan statistiknya, serta
    semua halaman (Perhitungan, Internal, Info, Pemotongan, Julia, Kamera,
    Pewarnaan, Stereo).
  * **Formula:** enam slot formula, daftar dan opsi formula MB3D, tipe
    hybrid, favorit, dan editor formula JIT.
  * **Pencahayaan:** cahaya 1–6, preset, palet, halaman Objek, Ambien,
    k.Kabut, dan Gbr latar; perubahan langsung menggambar ulang gambar, tanpa
    perhitungan baru.
  * **Pascaproses:** normal pada z-buffer, bayangan keras dan bayangan
    ambien, refleksi dan transparansi, depth of field.
  * **Navigator:** berjalan dan melihat sekeliling dengan tombol dan mouse,
    lalu *Tampilan ke utama*.
  * **Pembuat animasi, Pemrosesan batch, Render besar, Monte Carlo, Ekspor
    voxel, Bulb Tracer 2** (mesh) dengan pratinjau mesh, **Generator
    HeightMap, MutaGen, ZBuf16Bit, Urutan Peta, Ini Dirs** — semuanya seperti
    di MB3D.
  * **Swarm** (baru, di tab Alat): komputer-komputer Anda merender bersama,
    lihat [Merender di beberapa komputer](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — renderer baris perintah untuk skrip dan pekerjaan batch: gambar,
  animasi, daftar batch, render Monte Carlo, ekspor voxel dan mesh,
  semuanya tanpa jendela.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## Apakah aman dijalankan? Periksa sendiri

Anda tidak perlu sekadar percaya pada kata-kata kami. [scanrepo.dev](https://www.scanrepo.dev)
adalah layanan independen yang memeriksa repositori GitHub untuk mencari
tanda-tanda perangkat lunak berbahaya: kode yang menjalankan program lain,
terhubung ke internet, menyembunyikan apa yang dilakukannya, atau melewati
pemeriksaan keamanan bahasa pemrogramannya. Siapa pun dapat menjalankannya
pada repositori ini kapan saja:

**[▶ Pindai mb3d-rs di scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(klik *Rescan* untuk hasil baru dari kode saat ini)

Versi 0.2.85 dinilai **aman: 5 dari 100** (dipindai 30 Oktober 2026,
aturan mesin pemindai versi 10). Satu-satunya peringatannya adalah baris
yang menjalankan formula yang dikompilasi program saat berjalan demi
kecepatan: menjalankan kode mesin yang baru dibuat tidak bisa ditulis dalam
bentuk Rust yang diperiksa, dan baris itu serta jembatan ke filter derau
Intel adalah satu-satunya tempat program mengizinkannya; sisanya adalah
catatan informasi. (Sebelumnya: 0.2.66 dinilai 0 dari 100 oleh mesin yang
lebih lama, lihat juga [hasil 0.2.44](../img/scan_0.2.44.png).) Versi
yang lebih awal pernah ditandai, dan kami mengubah kodenya sebagai
tanggapan ([issue #4](https://github.com/thr32423/mb3d-rs/issues/4));
sekarang kode ditulis agar tetap begini. Program-program ini tidak berisi
kode yang terhubung ke internet; satu-satunya program lain yang mereka minta
dibuka oleh sistem Anda adalah pengelola file Anda, saat Anda mengklik tombol
untuk membuka folder.

Pemindaian mencari tanda-tanda peringatan yang sudah dikenal; itu bukan
jaminan. Hal-hal berikut memungkinkan Anda memeriksa lebih jauh sendiri:

* **Seluruh kode sumber ada di sini.** Unduhan dibangun dari kode ini oleh
  server build GitHub sendiri, dengan langkah-langkah yang bisa Anda baca di
  [alur kerja rilis](../../.github/workflows/release.yml), bukan di komputer
  pribadi siapa pun.
* **Setiap rilis mencantumkan checksum** (`SHA256SUMS`), sehingga Anda dapat
  memastikan bahwa file yang Anda unduh persis sama dengan yang diterbitkan;
  instalasi satu baris di bawah melakukannya untuk Anda. Rilis mulai 0.2.45
  juga membawa atestasi build GitHub: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` menunjukkan dari commit mana sebuah unduhan
  dibangun.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Apa yang baru dibandingkan Mandelbulb3D

* **Kartu grafis menghitung gambar** — gambarnya sendiri, bayangan,
  ambient occlusion, refleksi, dan render Monte Carlo. Di komputer biasa ini
  beberapa kali lebih cepat daripada prosesor saja; lihat
  [Kecepatan](#speed) di bawah.
* **Mode Otomatis**: kartu grafis dan prosesor mengerjakan setiap gambar
  bersama-sama, dan tidak ada yang pernah menunggu — jika kartu belum siap,
  prosesor tetap mulai dan kartu ikut bergabung kemudian.
* **Sebuah LED di bilah judul** menunjukkan di mana gambar terakhir dihitung.
* **Ukuran antarmuka** dari 75 hingga 200 % untuk layar beresolusi tinggi.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="Tema visual pada 150 %"></a>

  <details><summary>Selengkapnya</summary>

  **Prefs ▸ Tema ▸ Ukuran antarmuka** membuat semua jendela dan teksnya
  lebih besar atau lebih kecil (75 hingga 200 %), untuk layar beresolusi
  tinggi; tata letaknya tetap persis seperti di MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Tema visual pada 100 %](../img/themes_100.png) | ![Tema visual pada 150 %](../img/themes_150.png) |

  </details>
* **Delapan tampilan** untuk jendela, semuanya file tema yang bisa Anda
  salin dan ubah: dua meniru tampilan MB3D, enam lainnya baru.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Tema Midnight"></a>

  <details><summary>Selengkapnya</summary>

  **Prefs ▸ Tema** menawarkan delapan tampilan, masing-masing satu file di
  folder `Themes`: *Glossy* (bawaan yang gelap) dan *Windows* meniru dua
  tampilan MB3D sendiri, enam lainnya baru. Klik gambar untuk ukuran penuhnya.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: bawaan MB3D | **Windows**: "Nonaktifkan Tema" di MB3D |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: abu-abu netral lebih terang, aksen biru baja | **Daylight**: abu-abu terang yang hangat, aksen hijau toska |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: biru tua pekat, aksen biru | **Ember**: arang yang hangat, aksen oranye |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="High contrast"></a> |
  | **Forest**: abu-abu kehijauan gelap, aksen hijau | **High contrast**: hitam, teks putih, sorotan kuning |
  
  Tema adalah file teks kecil di folder `Themes` di samping program; salin
  satu, ubah warnanya, dan tema itu muncul di daftar pada saat program
  dijalankan berikutnya.

  </details>
* **Crash meninggalkan catatan di setiap sistem**: panic program (atau
  error yang mengakhirinya) ditambahkan ke `mb3d_crash.txt`, di samping
  program atau di folder datanya (sebelumnya hanya di Windows). Kirim file
  itu bersama laporan bug.
* **Semua kartu grafis di komputer ikut menghitung**, tanpa perlu diatur:
  setiap kartu yang ditemukan mengambil pita baris gambar dari kumpulan yang
  sama dengan kartu pertama dan prosesor, sehingga kartu yang lebih cepat
  cukup mengambil lebih banyak; bayangan, occlusion, sinar Monte Carlo, dan
  refleksi dibagi dengan cara yang sama, dan animasi merender satu frame per
  kartu sekaligus. Kartu yang lambat (grafis terintegrasi di samping kartu
  khusus) mengerjakan semampunya dan tidak pernah menahan gambar: begitu
  baris yang bebas habis, kartu yang menganggur dan prosesor mengambil alih
  baris yang belum diserahkannya. Kartu yang gagal ditinggalkan, yang lain
  tetap jalan. `mb3d --list-cards` menunjukkan apa yang ditemukan. (Diuji
  dengan satu kartu yang dibuka dua kali dan dengan driver Vulkan perangkat
  lunak sebagai kartu kedua; belum di komputer dengan dua kartu sungguhan.)
* **Gambar yang belum selesai adalah file `.m3i`.** Apa yang telah dihitung
  oleh render yang dihentikan, crash, atau dimatikan disimpan di cache
  (jendela Render belum selesai, `mb3d resume`); untuk sebuah gambar, file
  itu kini setiap saat merupakan file gambar `.m3i`, `image.m3i` di folder
  render, dengan baris yang belum dihitung sebagai latar belakang. Salin
  kapan pun Anda mau, atau tekan *Simpan m3i* di jendela Render belum
  selesai (`mb3d resume --m3i KEY file.m3i` di baris perintah; keduanya juga
  menulis `data.bin` dari versi sebelumnya sebagai `.m3i`), lalu buka di
  sini atau di Mandelbulb3D, tanpa perlu konversi. (Render Monte Carlo
  menyimpan catatannya sendiri, `.m3c` dari sebuah pass ditulis dengan
  `--m3c`.)
* **Tombol pintas untuk jendela**: sebuah kombinasi tombol membawa jendela
  utama, Navigator, Animasi, Formula, Pencahayaan, Pascaproses, atau jendela
  Monte Carlo ke depan dari jendela mana pun (Ctrl+1 hingga Ctrl+7), dan
  Ctrl+W menutup jendela tempat Anda berada (di macOS Cmd). Atur sendiri di
  *Prefs > Tombol pintas*: klik sebuah kolom lalu tekan tombolnya, Backspace
  menghapus tombol pintas.
* **Antarmuka dalam bahasa Anda**: setiap teks jendela berasal dari file
  bahasa; dua belas bahasa disertakan, dan program menulis file untuk bahasa
  baru agar Anda terjemahkan.

  <details><summary>Selengkapnya</summary>

  Lencana bahasa di ujung kanan bilah judul jendela utama menunjukkan
  bahasa antarmuka (`EN`); klik untuk membuka jendela bahasa. Sebuah bahasa
  adalah file teks di folder `Languages` di samping program.
  Disertakan: English, Deutsch, Français, Italiano, Nederlands, Polski,
  Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文, dan 繁體中文 (masing-masing
  dengan benderanya; bahasa-bahasa Asia Timur membawa subset Noto Sans CJK
  untuk karakternya, Hindi membawa Noto Sans Devanagari, yang huruf
  gabungan dan tanda vokalnya disusun dengan pembentuk teks rustybuzz).
  Terjemahannya dibuat oleh mesin dan diperiksa strukturnya, bukan oleh
  penutur asli: koreksi sangat diterima, masing-masing hanya satu baris di
  file. Untuk menerjemahkan: ketik nama bahasa Anda lalu tekan *Buat file
  bahasa*; program menulis `Languages/<name>.lng` dengan setiap teks dalam
  bahasa Inggris, satu per baris (`Button2.Caption = Calculate
  3D`). Terjemahkan teks di sebelah kanan `=` dengan editor teks lalu tekan
  *Muat ulang*: jendela-jendela langsung mengikuti, tanpa restart. Baris
  yang dilewati tetap memakai teks bahasa Inggris. *Simpan + keluar*
  menyimpan bahasa untuk saat program dijalankan berikutnya.

  </details>
* **Linux, Windows, dan macOS** (Intel dan Apple Silicon).
* **Program baris perintah** untuk semua yang tidak memerlukan jendela.
* **Gambar sangat besar dalam satu bagian**: 15000 x 20000 piksel dan lebih
  tanpa kehabisan memori, dan depth of field memakai semua inti prosesor
  (11 kali lebih cepat pada 24 thread dibanding sebelum 0.2.45, dengan
  gambar yang sama).
* **Gambar 16 bit:** file PNG dengan 65.536, bukan 256, tingkat per warna,
  untuk penyuntingan tanpa banding.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="Gambar 16 bit"></a>

  <details><summary>Selengkapnya</summary>

  MB3D menyimpan gambar 8 bit: 256 tingkat per warna. Itu lebih dari cukup
  untuk dilihat, tetapi saat gambar disunting setelahnya — kecerahan,
  kontras, kurva — area halus seperti latar belakang atau kabut pecah
  menjadi pita-pita yang terlihat. **Simpan gbr ▸ PNG** menawarkan tipe file
  kedua, **PNG 16 bit warna**: warna-warna seperti yang dihitung program,
  sebelum dibulatkan ke 256 tingkat, dengan semua bayangan, refleksi, dan
  depth of field. File-nya terlihat sama, dan tetap halus saat disunting.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="Gradien latar belakang setelah kontras dinaikkan kuat-kuat: bertingkat pada 8 bit, halus pada 16 bit"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="Dialog simpan dengan 'PNG 16 bit warna'"></a>

  </details>
* **Filter derau** untuk Monte Carlo dan semua gambar lain, DCT denoising
  yang dikenal dari GIMP, dan untuk render Monte Carlo juga filter neural,
  Intel Open Image Denoise.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="Filter derau"></a>

  <details><summary>Selengkapnya</summary>

  **Filter derau** menghilangkan bintik (grain) dari gambar: **DCT denoising**
  dari *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT* di GIMP, yang di
  antara filter GIMP paling cocok untuk gambar-gambar ini (lihat kredit di
  bawah).
  
  * **Jendela Monte Carlo ▸ -derau** (tombol yang dulu disembunyikan MB3D):
    memfilter gambar yang ditampilkan, dan yang disimpan, dengan kekuatan yang
    mengikuti jumlah sinar per piksel. Render itu sendiri berlanjut tanpa
    filter, dan pembaruan berikutnya menampilkannya lagi tanpa filter.
  * **Pascaproses ▸ Filter Derau**: *Filter sekarang* memfilter gambar di
    jendela utama; *Filter gambar baru otomatis* memfilter setiap gambar baru.
    *Kekuatan* adalah kekuatan derau yang akan dihilangkan (4 itu ringan;
    nilai lebih tinggi menghilangkan lebih banyak bintik dan lebih banyak
    detail halus).
  
  Gambar Monte Carlo punya bintik paling banyak. Dibandingkan dengan gambar
  yang dirender dengan 512 sinar per piksel, gambar yang difilter lebih dekat
  daripada yang tidak difilter di semua 13 uji (empat adegan, 4 hingga 32
  sinar): hingga 23 % pada 4 sinar, 5 hingga 10 % pada 8 sinar, 1 hingga 3 %
  pada 32 sinar, saat bintik yang tersisa sudah sedikit.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="Gambar Monte Carlo dengan 4 sinar per piksel, gambar yang sama dengan filter derau, dan dengan 512 sinar"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="Pascaproses: bagian Filter Derau"></a>

  **Filter derau neural** adalah
  [Intel Open Image Denoise](https://www.openimagedenoise.org), filter
  dari Cycles di Blender dan banyak path tracer lain, yang dilatih dengan
  gambar hasil render. Filter ini memfilter rata-rata sinar hasil render
  (sebelum exposure dan saturasi), sehingga gambar tetap mempertahankan
  kecerahannya. Pustaka Intel disertakan bersama program (folder `oidn`; di
  .deb dan .rpm, `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` dapat menunjuk ke
  pustaka lain.

  * **Jendela Monte Carlo ▸ -derau AI**: memfilter gambar yang ditampilkan,
    dan yang disimpan, seperti *-derau*; render berlanjut tanpa filter.
  * **Baris perintah**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * Prosesnya memakan 0,2 hingga 0,6 detik untuk gambar 640 x 480 di prosesor.

  Selisih terhadap gambar 512 sinar (rata-rata atas semua piksel, makin
  rendah makin baik):

  | Adegan | 4 sinar | filter DCT | filter neural | 8 sinar | filter DCT | filter neural |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  Filter neural paling dekat di tiga dari empat adegan; pada bintik halus
  himpunan Julia, filter DCT sedikit lebih dekat. Perbedaannya paling
  terlihat pada kaca dan cermin, tempat sinar yang sedikit meninggalkan
  bintik paling banyak:

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="Gambar Monte Carlo dengan 4 sinar per piksel, dengan filter DCT, dengan filter neural, dan dengan 512 sinar"></a>

  </details>
* **Latar belakang transparan** untuk gambar dan animasi.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="Latar belakang transparan"></a>

  <details><summary>Selengkapnya</summary>

  Gambar dan frame animasi dapat disimpan dengan **latar belakang
  transparan**, untuk menempatkan fraktal di atas gambar lain atau ke dalam
  editor video: **Simpan gbr ▸ PNG ▸ PNG latar transparan**, dan di
  Pembuat animasi **Latar belakang transparan** (frame PNG). Dengan anti-aliasing
  (skala tampilan 1:2 atau lebih, atau `--aa`) tepinya lembut, dan tidak
  membawa pinggiran warna latar belakang lama. Gambar kedalaman untuk
  compositing sudah ada sebelumnya (*Simpan juga Z buffer*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="Sebuah gambar dengan latar belakangnya, dan dengan latar belakang transparan yang ditampilkan di atas papan catur"></a>

  </details>
* **Video dari animasi:** file AVI langsung dari program, atau perintah
  ffmpeg untuk MP4 yang kecil.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="Video dari animasi"></a>

  <details><summary>Selengkapnya</summary>

  Di Pembuat animasi, **Buat juga video (AVI)** menggabungkan frame menjadi
  video saat frame-frame itu dirender, dengan **Frame per detik** yang Anda
  atur: `<project name>.avi` di folder keluaran, di samping frame, yang tetap
  disimpan. Video ini dapat diputar di VLC dan mpv dan dibuka di editor video
  (Kdenlive, Shotcut, DaVinci Resolve). Setiap frame disimpan sebagai gambar
  utuh, jadi file-nya besar: bagus untuk penyuntingan dan arsip, kurang
  cocok untuk dibagikan.
  
  Untuk video kecil yang akan dibagikan, **Perintah FFmpeg** menaruh perintah
  untuk MP4 (atau, dengan latar belakang transparan, WebM) di clipboard dan
  di pesan. Pasang [ffmpeg](https://ffmpeg.org), tempel perintahnya ke
  terminal setelah frame selesai, dan perintah itu membuat videonya; program
  ini sendiri tidak menjalankan program lain.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="Pembuat animasi dengan Buat juga video (AVI), Frame per detik, dan Perintah FFmpeg"></a>

  </details>
* **Pratinjau file parameter** di dialog Buka.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="Pratinjau saat membuka parameter"></a>

  <details><summary>Selengkapnya</summary>

  **Buka ▸ Buka m3p** dan **Buka m3i** menampilkan gambar kecil dari file
  parameter yang diklik, dengan ukuran dan formulanya, sebelum file itu
  dibuka; tombol panah bergerak melalui daftar beserta pratinjaunya. Gambar
  dihitung di latar belakang pada prosesor, sehingga dialog tetap responsif,
  dan pratinjau setiap file disimpan selama program berjalan.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="Dialog Buka dengan pratinjau sebuah file parameter"></a>

  </details>
* **Navigator: nilai tepat dan Reset tampilan.** Klik sebuah nilai dan
  ketikkan; kembali ke tampilan awal Navigator.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="Navigator: nilai tepat dan Reset tampilan"></a>

  <details><summary>Selengkapnya</summary>

  * **Klik sebuah nilai** di panel Navigator — nilai Julia, nilai formula,
    rotasi 4D, nilai Misc — dan ketik angka tepatnya, seperti di MB3D (sudut
    dalam derajat). Slider kemudian melanjutkan dari nilai itu.
  * **Klik zoom** dan ketikkan, jika langkah zoom terlalu kasar (permintaan
    dari daftar issue MB3D); zoom yang diketik tetap berlaku (*zoom tetap*).
  * **Reset tampilan** kembali ke posisi, arah, dan zoom yang diambil
    Navigator dari jendela utama; nilai-nilai yang sudah disesuaikan tetap.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="Navigator dengan nilai-nilai formula dan kotak isian dari nilai yang diklik"></a>

  </details>
* **Formula baru, SphereTree:** pohon bola-bola yang saling bersentuhan dan
  bercabang makin lama makin kecil, karya Tom Lowe.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="Formula baru: SphereTree"></a>

  <details><summary>Selengkapnya</summary>

  **SphereTree** adalah formula yang tidak dimiliki Mandelbulb3D: pohon
  bola yang saling bersentuhan dan bercabang menjadi bola yang makin lama
  makin kecil, seperti busa atau karang. Tom Lowe, penemu Mandelbox,
  menerbitkannya pada tahun 2020 (lihat kredit di bawah). Formula ini ada di
  daftar formula bersama formula dIFS dan punya empat pengaturan:
  
  * **PackRatio** (0 hingga 1,22): ukuran bola; pada 1 bola-bola tepat
    bersentuhan, di atas 1 bola-bola terluar terlepas.
  * **Levels**: seberapa dalam pohonnya (6 sudah cukup untuk kebanyakan
    gambar).
  * **Rotated**: 1 memutar setiap tingkat sebesar seperdua belas putaran, dan
    pohon menjadi bola bulat dari gugusan-gugusan bola.
  * **Colour scale**: setiap cabang pohon punya warnanya sendiri. Pilih
    *Pilihan ke-2* (orbit trap) di pengaturan warna Pencahayaan untuk
    melihatnya; pengaturan ini menyebarkannya di sepanjang gradien warna.
  
  Formula ini bekerja sendiri dan setelah transformasi dIFS (gambar ketiga
  mengulanginya dengan *tilingbrickIFS*), dan dihitung di kartu grafis
  maupun di prosesor. Atur iterasi ke 1: formula menghitung seluruh pohon
  sekaligus. Parameter dengan SphereTree hanya terbuka di program ini, tidak
  di Mandelbulb3D.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="Bola SphereTree (Rotated) di renderer Monte Carlo, saat senja"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="Close-up busa bola dari SphereTree"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="SphereTree diulang dengan tilingbrickIFS: sekawanan pohon bola"></a> |
  |:---:|:---:|

  </details>
* **Formula baru, DomeTree:** sebuah kubah yang menumbuhkan cincin
  kubah-kubah yang lebih kecil, masing-masing dengan cincinnya sendiri,
  karya Tom Lowe.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="Formula baru: DomeTree"></a>

  <details><summary>Selengkapnya</summary>

  **DomeTree** adalah pohon kedua Tom Lowe setelah SphereTree (lihat
  kredit): sebuah bola yang salah satu sisinya menumbuhkan cincin kubah,
  setiap kubah punya cincin lebih kecilnya sendiri, tingkat demi tingkat,
  tersusun dari inversi lingkaran. Formula ini ada di daftar formula bersama
  formula dIFS dan punya lima pengaturan:

  * **Levels**: seberapa dalam pohonnya (16 sudah cukup).
  * **Children**: jumlah kubah di setiap cincin, 4 hingga 10.
  * **Bend angle** (0,3 hingga 1,3 paling baik): seberapa jauh kubah
    menggembung; nilai rendah menghasilkan cakram datar, nilai tinggi kubah
    bulat. Di atas sekitar 1,3 permukaannya menjadi berbintik.
  * **Colour scale**: setiap kubah punya warnanya sendiri. Pilih
    *Pilihan ke-2* (orbit trap) di pengaturan warna Pencahayaan dan rentang
    warna yang lebar (misalnya 0 hingga 100) untuk melihatnya.
  * **Start curved**: 1 (bawaan) dimulai dengan bola, 0 dengan lantai datar
    yang menumbuhkan cincin pertama.

  Gunakan *Raystep* yang kecil (*Pengali langkah sinar* 0,2; 0,05 dengan 9
  atau 10 children): estimasi jarak bentuk ini kasar, seperti di file
  Fragmentarium milik TGlad sendiri. Seperti SphereTree, formula ini berjalan
  di kartu grafis dan prosesor, hanya perlu 1 iterasi, dan parameter
  dengannya hanya terbuka di program ini.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="DomeTree dengan 8 children di renderer Monte Carlo, diwarnai per kubah"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 5 children, bend angle 1,2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 6 children, bend angle 0,5: kubah datar"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 10 children: renda di antara kubah dan sebuah bola besar"></a> |
  |:---:|:---:|:---:|

  </details>
* **Formula baru, UlamSpiral:** spiral bilangan prima Ulam dalam 3D, sebagai
  kota, cakrawala gedung, bola-bola, atau balok.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="Formula baru: UlamSpiral"></a>

  <details><summary>Selengkapnya</summary>

  Pada tahun 1963 matematikawan Stanisław Ulam duduk mendengarkan "makalah
  yang panjang dan sangat membosankan" di sebuah pertemuan dan
  mencorat-coret selembar kertas berpetak. Ide pertamanya adalah menyusun
  soal catur; alih-alih, ia menomori petak-petak itu 1, 2, 3, … dalam
  spiral, 1 di tengah, lalu melingkari bilangan prima. Bilangan prima
  berjajar pada garis-garis diagonal, seolah mengikuti suatu pola, dan
  tetap begitu ketika komputer menggambar jutaan di antaranya (Ulam mencoba
  100.000 di MANIAC II di Los Alamos; Martin Gardner menaruh gambarnya di
  sampul Scientific American pada tahun 1964). Hingga hari ini belum ada yang
  bisa menjelaskannya sepenuhnya.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="Spiral Ulam: bilangan prima di antara bilangan 1 hingga 78.961 pada spiral persegi, dengan awal spiral digambar di sudut"></a>
  
  **UlamSpiral** membangun spiral itu dalam 3D, satu bentuk per bilangan,
  dalam empat bentuk (*Mode*):
  
  * **0, menara prima**: sebuah pelat untuk setiap bilangan dan sebuah menara
    di setiap bilangan prima. Garis-garis diagonal bilangan prima menjadi
    deretan menara.
  * **1, cakrawala pembagi**: sebuah kolom untuk setiap bilangan, makin
    tinggi makin banyak pembaginya; bilangan prima adalah jalan-jalan rendah
    di antaranya.
  * **2, bola pembagi**: sebuah bola untuk setiap bilangan, makin besar makin
    banyak pembaginya (bilangan prima adalah bola-bola kecil).
  * **3, balok bilangan prima**: bilangan-bilangan dililitkan pada
    kulit-kulit kubus, bukan persegi, dengan sebuah kubus di setiap bilangan
    prima.
  
  *Rings* mengatur ukuran spiral (60 cincin berarti bilangan hingga
  14.641), *Height* tinggi menara atau kolom, *Gap* jarak di antaranya.
  Dengan *Pilihan ke-2* (orbit trap) di pengaturan warna, warna mengikuti
  jumlah pembagi, sehingga bilangan prima punya warnanya sendiri. Seperti
  SphereTree, formula ini berjalan di kartu grafis dan prosesor, bekerja
  setelah transformasi dIFS, hanya perlu 1 iterasi, dan parameter dengannya
  hanya terbuka di program ini.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, menara prima: deretan diagonal bilangan prima sebagai sebuah kota, di renderer Monte Carlo dengan kabut, bayangan keras, dan depth of field"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, cakrawala pembagi"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, bola pembagi"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, balok bilangan prima pada kulit kubus"></a> |
  |:---:|:---:|:---:|

  </details>
* **Render yang dihentikan berlanjut dari tempatnya berhenti.** Apa yang
  sudah dihitung sebuah render disimpan saat render dihentikan, dan juga
  saat program ditutup atau crash, atau komputer mati (disimpan ke disk
  setiap 5 menit): Hitung 3D, Monte Carlo (juga di tengah sebuah pass),
  frame animasi atau gambar batch yang sedang berjalan, ekspor voxel, mesh
  Bulb Tracer, tile Render besar, dan render swarm (Hentikan di komputer
  yang membuat tugasnya menyimpan setiap bagian yang selesai, dan setiap
  komputer menyimpan bagian yang sedang dikerjakannya). Hitung 3D dan Monte
  Carlo dengan parameter yang sama bertanya apakah akan melanjutkan, yang
  lain melanjutkan sendiri; tombol **Lanjutkan** (tab Buka) mencantumkan
  render yang belum selesai, untuk dilanjutkan atau dihapus. Di baris
  perintah, perintah yang sama melanjutkan sendiri (juga gambar dalam
  `--tiles`, `montecarlo`, `animate`, `batch`, `voxel`, dan `mesh`);
  `--fresh` memulai dari awal, `mb3d resume` mencantumkannya. Render yang
  dilanjutkan sama dengan render yang dihitung sekaligus.
* **Render dibagi di antara komputer-komputer Anda**, sebuah "swarm":
  komputer-komputer yang melihat satu folder bersama merender gambar, Render
  besar, gambar Monte Carlo, atau animasi bersama-sama. Lihat
  [Merender di beberapa komputer](#rendering-on-several-computers-the-swarm).
* **Formula baru, PseudoKleinian:** "arsitektur Kleinian" berupa aula,
  lengkungan, dan bola sebagai satu formula dengan jarak tepatnya sendiri.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="Formula baru: PseudoKleinian"></a>

  <details><summary>Selengkapnya</summary>

  Aula, lengkungan, pilar, dan bola bersarang yang dikenal sebagai
  "arsitektur Kleinian" termasuk gambar fraktal 3D yang paling disukai. Di
  MB3D gambar ini hanya bisa dibuat sebagai hybrid dari beberapa formula
  dengan pengaturan yang dicocokkan dengan cermat, resep yang ditemukan
  Theli-at pada tahun 2011; knighty kemudian menulis hal yang sama sebagai
  satu formula dengan estimasi jaraknya sendiri, dan itulah
  **PseudoKleinian**. Setiap tingkat melipat titik ke dalam sebuah kotak,
  menginversinya pada sebuah bola, dan menambahkan offset, lalu jaraknya
  diukur secara tepat ke sebuah bidang lantai setelah lipatan-lipatan itu,
  sehingga dindingnya tampak bersih dan gambar dirender dengan cepat.
  Pengaturannya milik knighty: *Levels* (8), *Size* (bola inversi), *CSize
  X/Y/Z* (kotaknya; nilai 0.924, 0.908, 0.924 miliknya menghasilkan aula
  klasik, dan perubahan kecil menghasilkan bangunan baru), *Julia X/Y/Z*
  (offset per tingkat), *Shape* (0 bidang lantai, 1 bidang-silinder dengan
  *Thickness* melalui *Offset*, 2 kotak membulat), *DE offset* (melubangi
  permukaan) dan *Colour scale* (dengan *Pilihan ke-2*, orbit trap: radius
  orbit terkecil, pita-pita Kleinian klasik).
  
  Strukturnya mengisi seluruh ruang, jadi Anda melihatnya dari dalam: mulai
  dengan pusat tampilan di dekat titik asal dan kamera satu atau dua satuan
  di belakang, lalu bergeraklah melalui aula dengan Navigator. Di renderer
  Monte Carlo, bagian dalam hampir tidak mendapat cahaya langit, jadi
  tempatkan cahaya posisional di dalam aula (gambar pertama) atau potong
  strukturnya dengan bidang potong dan lihat ke dalam dari atas (gambar
  kedua). Formula ini berjalan di kartu grafis dan prosesor, perlu 1
  iterasi, bekerja setelah transformasi dIFS dan dalam kombinasi DE seperti
  formula dIFS lainnya (meski di dalam struktur yang mengisi ruang, hal itu
  tidak banyak mengubah), dan parameter dengannya hanya terbuka di program
  ini.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="PseudoKleinian dari sudut pandang knighty: aula lengkungan dan bola bersarang yang diterangi lampu di dalamnya, di renderer Monte Carlo dengan kabut, bayangan keras, dan depth of field"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="PseudoKleinian yang dipotong dengan bidang potong dan dilihat dari atas: kisi bola dan sel-sel di antaranya"></a>

  </details>
* **Pasangan stereo dalam satu langkah**: kedua gambar mata dirender dan
  ditampilkan berdampingan, untuk melihat secara paralel atau bersilang
  (cross-eyed). Kontribusi dari Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Pasangan stereo Hydra karya Dainbramage, berdampingan untuk melihat secara paralel"></a>

  <details><summary>Selengkapnya</summary>

  **Stereo ▸ Render pasangan stereo** merender gambar mata kiri lalu kanan
  dengan pengaturan stereo di tab itu dan menampilkannya berdampingan di
  jendela utama. Keduanya ditata untuk melihat secara paralel (gambar kiri
  di kiri); **Pratinjau cross-eyed** menukarnya untuk melihat secara
  bersilang. Ini adalah pratinjau: pencahayaan dan warna
  tidak bisa diubah setelahnya tanpa merender ulang, dan fitur ini tidak
  bekerja dengan tile. Tombol untuk gambar satu mata tetap ada.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Pasangan stereo Hydra karya Dainbramage, berdampingan untuk melihat secara paralel"></a>

  </details>
* **Sisa waktu untuk seluruh gambar**: bilah di bawah gambar terisi satu
  kali sepanjang seluruh perhitungan, dan di sampingnya sisa waktu hingga
  gambar selesai, termasuk bayangan, occlusion, dan refleksi.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="Bilah kemajuan dan total sisa waktu"></a>

  <details><summary>Selengkapnya</summary>

  Bilah MB3D mulai dari awal lagi di setiap tahap. Di sini bilah itu
  menunjukkan seluruh perhitungan: render utama, lalu bayangan keras dan
  ambient occlusion, refleksi, dan depth of field. Lama tahap-tahap setelah
  render utama diperkirakan dari apa yang diminta adegan dan bagaimana
  render utama berjalan; perkiraan ini dicocokkan pada sekitar 1400 render
  dari 366 adegan. Gambar yang dihitung ulang mengharapkan waktu yang sama
  dengan yang diperlukannya terakhir kali.
  
  Gambar pertama dengan formula baru di kartu grafis menunggu shader-nya
  dikompilasi: baris itu lalu bertuliskan *shader dikompilasi* dengan sisa
  waktunya, yang memperhitungkan laju prosesor dan, jika shader akan siap
  tepat waktu, bergabungnya kartu. Detailnya di
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="Jendela utama selama bayangan keras dan ambient occlusion, dengan total sisa waktu"></a>

  </details>
* **Riwayat** di balik ikon jam pada bilah judul: setiap gambar yang Anda
  hitung beserta thumbnail-nya, setiap file yang Anda buka atau simpan,
  sebagai daftar, sebagai baris, atau sebagai tile dengan ukuran berapa pun,
  dikelompokkan per hari atau dalam satu urutan; cari, urutkan, tampilkan
  hanya gambar yang berbeda, dan muat entri mana pun dengan klik ganda.
  Riwayat dengan ratusan ribu entri terbuka seketika.
* **Asisten AI dapat memakai mb3d** (`mb3d mcp`, Model Context
  Protocol): minta aplikasi desktop Claude atau Claude Code dengan kata-kata
  biasa, atau dengan suara melalui asistennya, untuk mendeskripsikan,
  mengubah, dan merender file parameter Anda, membiakkan variasi, atau
  merender apa yang terakhir Anda kerjakan; asisten melihat gambarnya dan
  dapat menyesuaikannya. Tombol **AI** di tab Prefs menghubungkannya dengan
  satu klik (atau `mb3d mcp --setup`). mb3d tetap offline (aplikasi asisten
  yang menjalankannya di komputer Anda), hanya menulis ke folder `AI`, dan
  tidak pernah menghapus apa pun; gambar di atas 4K memerlukan persetujuan
  Anda.
* **Log dari segalanya** di balik ikon halaman pada bilah judul: setiap
  klik, pengaturan, dan nilai yang diketik, setiap perhitungan dengan tahap
  dan waktunya, file yang dibuka dan disimpan, jendela, pesan, peringatan,
  dan error, di jendela yang mengikutinya secara langsung dan di satu file
  per hari. Sesi yang macet atau dimatikan dilaporkan saat program
  dijalankan berikutnya, dan laporan crash memuat baris-baris terakhir log.
* **Pencahayaan tetap langsung** setelah bayangan keras dan refleksi: geser
  sebuah cahaya atau ubah warnanya dan gambar langsung berubah, seperti di
  Mandelbulb3D.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="Jendela utama dan jendela Pencahayaan tepat setelah perubahan pencahayaan"></a>

  <details><summary>Selengkapnya</summary>

  Render dulu secara polos, lalu coba posisi dan warna cahaya: setiap
  perubahan di jendela Pencahayaan langsung menggambar ulang dengan bayangan
  seperti yang sudah dihitung, seperti di Mandelbulb3D. Mengubah warna atau
  intensitas cahaya tidak pernah menghitung bayangan lagi. Bayangan dari
  cahaya yang Anda geser, refleksi, dan depth of field kembali dengan
  tombol-tombol jendela Pascaproses, seperti di Mandelbulb3D.
  
  Perhitungan yang Anda hentikan juga masih bisa dilanjutkan setelah Anda
  mengubah pencahayaan. Satu cara bekerja dengannya: [Alur kerja pencahayaan](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-id)
  di wiki. Detailnya di
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="Jendela utama dan jendela Pencahayaan tepat setelah perubahan pencahayaan: gambar sudah lebih terang"></a>

  </details>

<a id="download"></a>

## Unduh

Program siap pakai untuk rilis terbaru, tanpa perlu instalasi:

| Sistem | Unduhan |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (terpasang seperti program lain) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (terpasang seperti program lain) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple silicon (M1 dan yang lebih baru) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Ekstrak lalu jalankan **Mandelbulb3D** (`Mandelbulb3D.exe` di Windows) di
folder hasil ekstrak. Folder itu tersusun seperti instalasi MB3D, dengan
formula, map, dan 80 contoh file parameter dari MB3D.

* Jika Windows menampilkan "Windows protected your PC", klik *More info* ▸
  *Run anyway*.
* macOS memblokir program unduhan yang tidak ditandatangani: jalankan
  `xattr -dr com.apple.quarantine .` sekali di folder hasil ekstrak.

**Alternatif untuk Linux dan macOS:** alih-alih mengunduh secara manual,
satu baris ini mengambil rilis terbaru ke `~/mb3d-rs` (jalankan lagi nanti
untuk memperbarui; skrip [download.sh](../../download.sh) juga ada di folder
program):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Skrip ini memeriksa unduhan sebelum mengekstraknya, menaruh **Mandelbulb3D**
di menu aplikasi Anda (Linux), dan membuat perintah `Mandelbulb3D` dan `mb3d`
yang bisa Anda ketik di folder mana pun. `sh ~/mb3d-rs/download.sh
--uninstall` menghapus lagi entri menu dan perintah-perintah itu; file Anda
tetap ada.

Program Linux berjalan di distribusi kira-kira dari tahun 2018 ke atas
(Ubuntu 18.04, Debian 10, Rocky/RHEL 8, dan yang lebih baru).

**Dipasang seperti program lain:** di Debian, Ubuntu, dan Mint, buka file
`.deb` di atas (atau `sudo apt install ./mb3d-rs-linux-x64.deb`); di
Fedora, RHEL, dan Rocky file `.rpm` (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, di openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D kemudian ada di menu aplikasi dan `mb3d` berfungsi di folder
mana pun; pengaturan dan Riwayat Anda disimpan di `~/.config/mb3d-rs` dan
`~/.local/share/mb3d-rs`.

[Memulai](../../GETTING_STARTED.md) memandu langkah-langkah pertama; versi
lama dan catatan rilis ada di halaman
[releases](https://github.com/thr32423/mb3d-rs/releases).

<a id="guides-and-help"></a>

## Panduan dan bantuan

Panduan langkah demi langkah ada di [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-id):

* [Instalasi](https://github.com/thr32423/mb3d-rs/wiki/Installing-id): Windows, macOS, dan Linux, pembaruan, letak file Anda
* [Alur kerja pencahayaan](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-id): render secara polos, lalu coba cahaya, bayangan, dan refleksi
* [Menyiapkan swarm](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-id): merender di beberapa komputer melalui folder bersama
* [Pemecahan masalah dan FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-id): saat sesuatu tidak bekerja seperti yang diharapkan

Setiap rilis beserta perubahannya: [daftar lengkapnya](../UPDATES.md).

<a id="rendering-on-several-computers-the-swarm"></a>

## Merender di beberapa komputer (swarm)

Komputer-komputer yang melihat satu folder bersama merender bersama: gambar
dalam tile, Render besar, gambar Monte Carlo, animasi, atau batch. Setiap
komputer mengambil bagian yang belum diambil komputer lain, sehingga komputer
yang cepat mengambil lebih banyak, dan komputer yang memulai render
menyatukan hasilnya. Folder itulah yang melakukan pembagian: program tidak
membuka koneksi jaringan dan tidak berbicara dengan siapa pun, jadi
keamanannya sama dengan berbagi file Anda.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D dengan jendela Swarm, dibuka dengan tombol Swarm di tab Alat, saat dua komputer merender gambar dalam tile: folder bersama, Bagikan komputer ini, komputer-komputer dan apa yang dikerjakan masing-masing, tugasnya, dan pratinjau dengan tile yang sudah dirender, tile yang sedang dikerjakan, dan yang belum ada"></a>

<a id="setting-it-up"></a>

### Menyiapkannya

Langkah demi langkah beserta apa yang perlu diperiksa: [Menyiapkan swarm](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-id) di wiki.

1. **Folder yang bisa ditulisi setiap komputer:** share di NAS, share
   Windows, share Samba, atau NFS. Di Linux, mount sebagai folder, misalnya
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   share yang hanya dibuka di pengelola file (`smb://...`) bukanlah folder
   yang bisa dipakai program.
2. **Versi mb3d-rs yang sama di setiap komputer.** Komputer dengan versi
   lain tercantum tetapi tidak ikut serta.
3. **Di Mandelbulb3D:** tombol **Swarm** di tab Alat. Pilih folder bersama
   (share yang sama di setiap komputer; path-nya boleh berbeda), beri nama
   komputer untuk daftar, dan centang **Bagikan komputer ini**.

Hanya itu. Baris status memberi tahu apa yang terjadi, misalnya *Berbagi
dengan omarchy: render dibagi di antara 2 komputer*. Jika sendirian di
swarm, semuanya dirender di komputer ini seperti biasa.

<a id="what-is-shared"></a>

### Apa yang dibagikan

Selama *Bagikan komputer ini* dicentang, komputer merender bagian-bagian
yang ditaruh komputer lain di folder, dan render-nya sendiri masuk ke swarm
setiap kali ada komputer lain di dalamnya:

* **Hitung 3D** (dan jendela Batch) — gambar dalam N x M tile (diatur di
  jendela Swarm), disatukan di sini dengan semua yang diberikan perhitungan
  biasa: perubahan pencahayaan, pascaproses, penyimpanan sebagai m3i (dan
  kebutuhan memorinya, lihat di bawah).
* **Render besar** — tile disimpan satu per satu saat tiba; seluruh gambar
  tidak pernah ditahan di memori, jadi inilah caranya untuk gambar
  berukuran 100000 x 100000 piksel dan lebih.
* **Monte Carlo** — seperti di satu komputer: setiap komputer menambahkan
  pass ke bloknya sendiri sampai Anda menekan Hentikan, gambar menjadi
  bersih dengan sinar dari semua komputer, dan Hentikan merata-ratakan
  blok-blok itu. Dalam mode batch jendela itu, blok-blok bersama-sama
  berhenti pada jumlah sinar batch.
* **Pembuat animasi** — frame-frame, satu per satu.

Jendela itu mencantumkan komputer-komputer dengan inti prosesor, kartu
grafis, dan apa yang sedang dikerjakan masing-masing, tugas-tugas dengan
kemajuannya, dan pratinjau tugas terbaru: tile yang sudah dirender, yang
sedang dikerjakan dengan bilah kemajuan, dan yang masih belum ada.

<a id="on-the-command-line"></a>

### Di baris perintah

Di setiap komputer yang akan membantu, jalankan worker pada folder itu:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Lalu, di komputer mana pun, render dengan `--swarm` dan folder yang sama:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` menunjukkan siapa saja yang ada di swarm, apa yang
dikerjakan setiap komputer, dan sejauh mana tugas-tugasnya. Gambar Monte
Carlo di baris perintah dibagi menjadi blok-blok dengan sinar acaknya
sendiri: 4 blok dengan 64 sinar per piksel memberikan 256 yang diminta
dengan `--rays 256`.

<a id="good-to-know"></a>

### Perlu diketahui

* **Jika komputer lain tetap menganggur** dan daftar tugasnya kosong
  sementara komputer ini merender: komputer yang menyajikan folder dengan
  Samba dan memakainya secara langsung membuat komputer lain menyimpan
  daftar isi folder yang lama. Matikan caching itu di
  `/etc/samba/smb.conf`: `smb3 directory leases = no` di `[global]`, serta
  `oplocks = no` dan `level2 oplocks = no` di bagian share, lalu
  `sudo systemctl restart smb` dan mount ulang share di komputer-komputer
  lain.
* **Memori:** sebuah gambar memakan sekitar 20 byte per piksel di program
  ini (buffer gambarnya), 10 lagi di jendela utama, lalu yang lebih besar
  dari dua tahap: perhitungan utama dengan bayangan dan occlusion-nya (16 per
  piksel dan 1,2 GB untuk kartu), atau pewarnaan (3 per piksel, 20 lagi
  dengan depth of field, dan satu pita refleksi: paling banyak 3,5 GB,
  lebih sedikit jika memori bebas sedikit, karena ukuran pita disesuaikan
  dengan separuh memori bebas saat pass dimulai). Gambar 12000 x 6000:
  6 GB. Jika sebuah gambar memerlukan lebih dari memori bebas komputer
  (ditanyakan ke Linux, Windows, dan macOS), Hitung 3D memberi tahu berapa
  yang diperlukan dan berapa yang bebas, lalu bertanya sebelum mulai:
  perhitungan akan melakukan swap dan program bisa berhenti merespons
  selama beberapa menit sekaligus. Render besar, sendiri atau dengan swarm,
  adalah jalan keluarnya: tile-nya disimpan satu per satu, seluruh gambar
  tidak pernah ditahan. Hitung 3D dengan swarm menahan gambar di sini
  seperti perhitungan biasa (tanpa tahap perhitungan utama, yang dikerjakan
  komputer-komputer tile dengan memori masing-masing tile); komputer swarm
  menyerahkan tile yang terlalu besar untuk memorinya kepada komputer lain.
* **Keamanan:** sebuah komputer hanya merender apa yang diminta tugas dan
  hanya menulis ke folder tugas itu; kode formula dari sebuah tugas
  berjalan di emulator formula seperti setiap formula.
* **Kecepatan:** tile dikirim sebagai buffer gambar penuh, beberapa ratus
  megabyte untuk gambar yang sangat besar, jadi share yang cepat sangat
  berguna. Layanan sinkronisasi yang menyalin dengan jeda juga bisa, tetapi
  lebih lambat.
* **Komputer yang pergi:** bagian yang sedang dirender sebuah komputer
  diambil komputer lain begitu komputer itu tidak terdengar kabarnya selama
  setengah menit. Tugas yang pemiliknya sudah pergi selama setengah jam
  dihapus oleh siapa pun yang melihatnya. Tugas yang gagal di sebuah
  komputer dicatat di jendela dan diserahkan kepada komputer lain.
* **Berhenti:** menghapus centang *Bagikan komputer ini* membiarkan bagian
  yang sedang berjalan selesai dulu; menutup program melepaskannya, dan
  komputer lain mengambilnya.
* **Menghentikan tugas** (Hentikan di komputer yang membuatnya, atau
  menutupnya, atau komputer itu crash): setiap bagian yang sudah selesai
  disimpan di komputer itu, dan setiap komputer menyimpan bagian yang sedang
  direndernya. Memulai render yang sama lagi membuat tugas yang hanya
  berisi bagian yang belum ada, dan komputer yang mendapat kembali bagian
  yang sudah dimulainya akan melanjutkannya. Hal yang sama berlaku di baris
  perintah: jalankan lagi perintah yang sama. Render Monte Carlo tanpa batas
  akhir (dari jendela) menyatukan blok-bloknya saat dihentikan, seperti
  sebelumnya.

<a id="how-the-rendering-works"></a>

## Cara kerja render

Gambar fraktal dihitung piksel demi piksel: untuk setiap piksel sebuah
sinar diikuti ke dalam adegan sampai mengenai permukaan fraktal, dan
formula dievaluasi puluhan kali di setiap langkah. MB3D melakukannya di
prosesor, memakai semua intinya. mb3d-rs bisa melakukan hal yang sama, dan
juga bisa menyerahkan pekerjaannya ke **kartu grafis**, yang punya ribuan
prosesor kecil dan dibuat tepat untuk pekerjaan seperti ini.

Pengaturan **"Hitung di"** (tab Internal di jendela utama) yang memilih:

| | |
|---|---|
| **Otomatis** (bawaan) | Kartu dan prosesor berbagi gambar: kartu mengambil baris dari atas, thread prosesor dari bawah, sampai keduanya bertemu. Bayangan dan ambient occlusion dibagi dengan cara yang sama, atau dikerjakan kartu sendirian jika kartu jelas lebih cepat. |
| **CPU** | Hanya prosesor, seperti di MB3D. |
| **Kartu grafis** | Hanya kartu (menunggu sampai programnya siap). |

Saat sebuah kombinasi formula dipakai pertama kali, driver grafis harus
menyiapkan program untuk kartu — *shader* — yang memakan 10 hingga 15 detik
di komputer yang diukur di bawah. Dalam mode Otomatis, prosesor langsung
mulai tanpa shader itu dan kartu bergabung saat siap, sehingga gambar kecil
tidak pernah menunggu. Shader yang sudah disiapkan disimpan untuk lain kali
(oleh driver grafis, di Linux di folder `ShaderCache` di samping program),
sehingga gambar berikutnya dengan formula yang sama langsung mulai di kartu.

**LED** di sebelah kanan judul jendela memberi tahu di mana gambar terakhir
dihitung: hijau di kartu grafis, oranye sebagian, merah di prosesor.
Mengarahkan kursor ke LED menampilkan detailnya.

[Cara kerja render](../RENDERING.md) menceritakan semuanya dengan gambar:
apa yang dilakukan Otomatis pertama kali, mengapa kartu tidak selalu
menjadi pilihan yang lebih cepat, dan apa arti warna-warna LED.

Tidak semuanya bisa dikerjakan kartu: beberapa formula tidak bisa
diterjemahkan untuknya, dan tampilan yang di-zoom sangat dalam dihitung
dengan presisi ganda (yang hanya bisa dilakukan sebagian kartu). Program
memutuskan semua ini sendiri; Anda hanya melihatnya di LED.

<a id="speed"></a>

## Kecepatan

Semua pengukuran di satu komputer: AMD Ryzen 9 3900X (12 inti, 24 thread)
dengan NVIDIA GeForce RTX 2060, di Linux. **Mandelbulb3D 1.99sr37** asli
dijalankan di mesin yang sama di bawah Wine (program Windows-nya berjalan di
sana tanpa perubahan, dengan kecepatan penuh, memakai semua inti), dan
diukur waktunya dari klik pada *Hitung 3D* hingga gambar selesai. Setiap
file parameter dirender dengan ukuran gambar yang sama oleh ketiganya: MB3D,
mb3d-rs hanya di prosesor (**CPU**), dan mb3d-rs dalam mode **Otomatis**
(kartu grafis dan prosesor bersama). Waktu Otomatis adalah waktu render
kedua sebuah file, dengan shader kartu yang sudah disiapkan; render pertama
untuk kombinasi formula baru menambahkan 10–15 detik persiapan shader di
kartu ini.

<a id="mb3ds-80-example-parameter-files"></a>

### 80 contoh file parameter MB3D

Contoh file yang disertakan bersama MB3D (dan di folder `M3Parameter`
program ini), masing-masing dengan ukuran gambarnya sendiri.

MB3D tidak bisa menghitung salah satu file (*Test8*: berhenti dengan error "Division by zero"), jadi perbandingannya menghitung 79 file.

* **mb3d-rs di prosesor saja lebih cepat daripada MB3D** di 71 dari 79 file, dengan median **1,7 kali**; semua file bersama-sama memakan 18 mnt 21 dtk di MB3D dan 17 mnt 10 dtk di mb3d-rs pada prosesor.
* **Mode Otomatis** lebih cepat daripada MB3D di 77 dari 79 file (median 2,1 kali); semua file bersama-sama 8 mnt 17 dtk. Di 30 file prosesor selesai sendirian, kebanyakan karena hanya perlu beberapa detik, sebelum kartu mendapat giliran — di sana Otomatis hanyalah prosesor ditambah sedikit overhead. Kartu ikut bekerja di 49 file, yang lebih lambat, dan di sana kartu membuat gambar dengan median 1,6 kali lebih cepat daripada prosesor (hingga 13 kali: *Insect Menger*, 4,7 dtk alih-alih 63,0 dtk).
* Di 4 file Otomatis terasa lebih lambat daripada prosesor saja (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): di sana kartu lebih lambat daripada 24 thread prosesor untuk adegan itu.
* "Baris di kartu" adalah bagian gambar yang dikerjakan kartu grafis dalam mode Otomatis (– jika prosesor selesai sendirian).

<details>
<summary>Semua 80 file (klik untuk membuka)</summary>

| File parameter | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Otomatis | baris di kartu | Otomatis vs MB3D |
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
| Test8 | error di MB3D | 0.1 s | 0.3 s |  | – |
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

Gambarnya sama: mb3d-rs di prosesor menghasilkan gambar MB3D hingga
selisih pembulatan, dan gambar Otomatis berbeda dari gambar prosesor rata-rata
sepersekian persen dari rentang warna (detail halus bisa berkilau di piksel
yang berbeda; [catatan teknis](../TECHNICAL.md) memuat pengukurannya).

<a id="the-command-line"></a>

## Baris perintah

`mb3d` merender tanpa jendela. Buka terminal di folder program (di Windows
command prompt di folder hasil ekstrak, dan ketik `mb3d` alih-alih
`./mb3d`). Setelah instalasi satu baris di Linux atau macOS, `mb3d`
berfungsi di folder mana pun, `man mb3d` menampilkan manualnya, dan tombol
Tab melengkapi opsinya di bash, zsh, dan fish:

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

Semua hal lain punya sub-perintah dengan `--help`-nya sendiri:

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

Formula dan map dicari di samping program; formula dan map dari instalasi
MB3D lain dapat diberikan dengan `--formulas` dan `--maps`.
Animasinya adalah animasi MB3D: keyframe dengan interpolasi MB3D, proyek
`.m3a` dibuka dan disimpan, frame ditulis seperti MB3D sehingga animasi
yang terputus dapat dilanjutkan dan beberapa komputer dapat berbagi satu
animasi.

<a id="good-to-know-1"></a>

## Perlu diketahui

* **Gambar sama, tetapi tidak identik per bit.** MB3D menghitung dengan
  presisi 80 bit dari aritmetika x87 lama; program ini memakai 64 bit masa
  kini. Gambar-gambarnya cocok dalam setiap struktur, tetapi piksel tunggal
  bisa berbeda di digit terakhir warnanya. Render Monte Carlo dan MutaGen
  memakai bilangan acaknya sendiri, sehingga hasilnya tidak sama dengan
  MB3D.
* **Gambar dari kartu grafis** sedikit berbeda dari gambar prosesor
  (presisi tunggal); pada detail yang sangat halus, kilaunya bisa jatuh di
  piksel lain. Zoom yang dalam beralih ke presisi ganda dengan sendirinya.
* **Kecepatan di prosesor** mendekati MB3D: kode mesin formula dari file
  `.m3f` MB3D diterjemahkan agar berjalan di prosesor apa pun (juga Apple
  Silicon), dengan sedikit biaya; ray marching-nya sendiri lebih cepat.
  Lihat tabel di atas. Di Linux dan Windows, formula-formula dari hybrid
  sebuah adegan juga dikompilasi bersama menjadi satu fungsi saat gambar
  dimulai (dalam beberapa milidetik, dengan Cranelift), sehingga formula
  saling menyerahkan nilainya di register prosesor. Bersama dengan
  perulangan yang lebih cepat atas formula-formula (di setiap sistem),
  render CPU dari 80 contoh file memakan waktu 20 % lebih sedikit daripada
  di v0.2.74 (hingga 60 % pada beberapa file), dengan gambar yang sama
  hingga ke bit. Sebuah adegan hanya mempertahankan hybrid yang dikompilasi
  jika hasil pengukurannya lebih cepat; `MB3D_NO_JIT=1` mematikannya. Di
  macOS fitur ini juga aktif, tetapi masih diuji di sana: `./jit_check.sh`
  di folder hasil ekstrak memeriksanya (skrip ini membandingkan
  formula yang dikompilasi dengan yang diterjemahkan dan merender contoh
  file dengan dan tanpa fitur itu; gambarnya harus sama).
* **Gambar besar** dapat dirender dalam satu bagian (gambar 15000 x 20000
  memerlukan sekitar 15 GB memori; dengan depth of field memakan waktu
  berjam-jam) atau dalam tile, juga di beberapa komputer (`--tiles`, dan
  jendela Render besar), yang memerlukan lebih sedikit memori. Hitung 3D
  tidak memulai gambar yang tidak muat di memori bebas (di Linux, tempat
  memori bebas dapat diketahui) dan memberitahukannya, dan komputer swarm
  menyerahkan tile yang terlalu besar untuk memorinya kepada komputer lain.
* **Render yang belum selesai** disimpan di folder cache (`~/.cache/mb3d-rs/resume`,
  di Windows `%LOCALAPPDATA%\mb3d-rs\resume`, atau di samping program):
  18 byte per piksel untuk gambar (gambar 15360 x 15360 memakan 4,2 GB),
  64 untuk render Monte Carlo. Apa yang sudah dihitung langsung ditulis dan
  di-flush ke disk setiap 5 menit. Render yang dihentikan dalam 10 detik
  pertamanya tidak menyimpan apa pun; render belum selesai yang lebih tua
  dari 30 hari dihapus, begitu pula yang tertua jika semuanya memakan lebih
  dari 50 GB. Render hanya bisa dilanjutkan dengan parameter dan versi
  program yang sama (dan pilihan kartu atau prosesor yang sama; exposure dan
  saturasi render Monte Carlo boleh berubah). Render Monte Carlo tetap
  belum selesai sampai mencapai jumlah sinarnya (batch, `--rays`,
  `--passes`) atau dihapus di daftar Lanjutkan.
* **Formula, map, tema, dan bahasa yang lebih baru atau buatan Anda sendiri**
  masuk ke `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes`, dan `.../Languages`
  (apa pun cara instalasinya; di Windows `%APPDATA%\mb3d-rs\...`). Semuanya
  dipakai sebagai tambahan bagi yang
  disertakan bersama program, dan menggantikan file bawaan dengan nama yang
  sama, sehingga formula yang diperbarui dari forum langsung berlaku tanpa
  menyentuh folder program.

<a id="updates"></a>

## Pembaruan

Setiap rilis beserta perubahannya tercantum di [UPDATES.md](../UPDATES.md) (dalam bahasa Inggris).

<a id="for-developers"></a>

## Untuk pengembang

[Catatan teknis](../TECHNICAL.md) menjelaskan bagaimana kartu grafis
dipakai, apa yang di-port dari sumber Delphi MB3D dan bagaimana caranya,
tiga cara kode mesin formula MB3D dijalankan, format file adegan dan
kunci-kuncinya, pemeriksaan terhadap MB3D, serta cara membangun dari kode
sumber:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## Lisensi

MB3D dilisensikan di bawah LGPL 2.1. Port ini adalah karya turunan dan
memakai lisensi yang sama.

<a id="credits"></a>

## Kredit

Orang-orang dan proyek yang menjadi dasar mb3d-rs disebutkan di [README bahasa Inggris](../../README.md#credits).
