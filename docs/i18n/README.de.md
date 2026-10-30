<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D für Linux, Windows und macOS">

# mb3d-rs — Mandelbulb3D für Linux, Windows und macOS
<!-- languages:start -->
[English](../../README.md) · **Deutsch** · [Français](README.fr.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*Übersetzt aus dem englischen README (Version 0.2.84). Wo die beiden voneinander abweichen, gilt das [englische README](../../README.md).*

**Downloads: [die neueste Version](https://github.com/thr32423/mb3d-rs/releases/latest)** · [Download](#download) · [Was ist neu](../UPDATES.md)

mb3d-rs ist [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D), das
3D-Fraktalprogramm, so neu geschrieben, dass es auf heutigen Computern läuft:
unter **Linux, Windows und macOS**, und mit dem Bild, das auf der
**Grafikkarte** berechnet wird, wo MB3D nur den Prozessor nutzen konnte. Es
öffnet die Parameterdateien von MB3D (`.m3p`, `.m3i`, Textparameter aus Foren),
verwendet die Formeln und Maps von MB3D und erzeugt dieselben Bilder.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="Das Hauptfenster"></a>

Es besteht aus zwei Programmen:

* **Mandelbulb3D** — das Desktop-Programm, mit den eigenen Fenstern von MB3D:
  dem Hauptfenster, Formeln, Beleuchtung, Nachbearbeitung, Navigator,
  Animationen, Stapelverarbeitung, Große Renderings, Monte Carlo, Voxel-Export,
  Bulb Tracer 2, MutaGen und den übrigen. Wer MB3D kennt, kennt dieses
  Programm; die Anleitungen für MB3D gelten auch hier.

  <details><summary>Mehr lesen</summary>

  Die Fenster sind die von MB3D — erzeugt aus den ursprünglichen
  Fensterdefinitionen von MB3D, sodass jede Schaltfläche, jeder Regler und
  jeder Hinweis dort ist, wo er in MB3D ist:
  
  * **Hauptfenster:** Parameter öffnen und speichern (`.m3p`, `.m3i`, Text,
    die Zwischenablage), Bilder speichern (PNG, JPEG, BMP, 16-Bit-Z-Buffer),
    die Mausmodi, die Drehknöpfe, Bildgröße, die Qualitätsvorgaben,
    Rückgängig, *3D berechnen* mit dem wachsenden Bild und der Statistik,
    und alle Seiten (Berechnung, Intern, Infos, Schnitt, Julia, Kamera,
    Färbung, Stereo).
  * **Formeln:** die sechs Formelplätze, die Formellisten und Optionen von
    MB3D, Hybridtypen, Favoriten und der JIT-Formeleditor.
  * **Beleuchtung:** Lichter 1–6, Vorgaben, die Palette, die Seiten Objekt,
    Umgebung, d.Nebel und Hintergrundbild; Änderungen zeichnen das Bild sofort
    neu, ohne neue Berechnung.
  * **Nachbearbeitung:** Normalen auf dem Z-Buffer, harte Schatten und
    Umgebungsschatten, Reflexionen und Transparenz, Tiefenschärfe.
  * **Navigator:** mit Tasten und Maus umhergehen und sich umsehen, dann
    *Ansicht zum Haupt.*
  * **Animationseditor, Stapelverarbeitung, Große Renderings, Monte Carlo,
    Voxel-Export, Bulb Tracer 2** (Meshes) mit der Mesh-Vorschau, der
    **HeightMap-Generator, MutaGen, ZBuf16Bit, Map-Sequenzen, Ini-Ordner** —
    alles wie in MB3D.
  * **Schwarm** (neu, auf der Registerkarte Tools): Ihre Computer rendern
    gemeinsam, siehe [Rendern auf mehreren Computern](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — ein Kommandozeilen-Renderer für Skripte und Stapelaufträge:
  Bilder, Animationen, Stapellisten, Monte-Carlo-Renderings, Voxel- und
  Mesh-Exporte, alles ohne Fenster.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## Ist es sicher? Prüfen Sie es selbst

Sie müssen sich nicht auf unser Wort verlassen. [scanrepo.dev](https://www.scanrepo.dev)
ist ein unabhängiger Dienst, der ein GitHub-Repository auf Anzeichen
schädlicher Software untersucht: Code, der andere Programme startet, sich mit
dem Internet verbindet, verbirgt, was er tut, oder die Sicherheitsprüfungen
der Programmiersprache umgeht. Jeder kann ihn jederzeit auf dieses Repository
anwenden:

**[▶ mb3d-rs auf scanrepo.dev prüfen](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(klicken Sie auf *Rescan*, um ein frisches Ergebnis für den aktuellen Code zu bekommen)

Version 0.2.85 wurde als **sicher: 5 von 100** bewertet (gescannt am 30. Oktober 2026,
Regeln der Engine 10 des Scanners). Ihre einzige Warnung betrifft die Zeile, die die
Formeln startet, die das Programm zur Laufzeit für mehr Geschwindigkeit kompiliert:
frisch erzeugten Maschinencode auszuführen lässt sich nicht in der geprüften Form von
Rust schreiben, und diese Zeile und die Brücke zu Intels Rauschfilter sind die
einzigen Stellen, an denen das Programm es erlaubt; der Rest ist ein informativer
Hinweis. (Früher: 0.2.66 wurde von einer älteren Engine mit 0 von 100 bewertet, siehe auch
[das Ergebnis von 0.2.44](../img/scan_0.2.44.png).) Eine frühere Version wurde
beanstandet, und wir haben den Code daraufhin geändert ([Issue #4](https://github.com/thr32423/mb3d-rs/issues/4));
er ist nun so geschrieben, dass es so bleibt. Die Programme enthalten keinen
Code, der sich mit dem Internet verbindet; das einzige andere Programm, das
sie Ihr System öffnen lassen, ist Ihr Dateimanager, wenn Sie auf eine
Schaltfläche klicken, um einen Ordner zu öffnen.

Ein Scan sucht nach bekannten Warnzeichen; eine Garantie ist er nicht. Hiermit
können Sie selbst weiter prüfen:

* **Der gesamte Quellcode ist hier.** Die Downloads werden daraus von GitHubs
  eigenen Build-Servern gebaut, nach den Schritten, die Sie im
  [Release-Workflow](.github/workflows/release.yml) nachlesen können, nicht
  auf dem persönlichen Computer von irgendjemandem.
* **Jede Version listet Prüfsummen** (`SHA256SUMS`), damit Sie bestätigen
  können, dass die heruntergeladene Datei genau die veröffentlichte ist;
  die Ein-Zeilen-Installation unten erledigt das für Sie. Versionen ab 0.2.45
  tragen außerdem GitHubs Build-Attestierungen: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` zeigt, aus welchem Commit ein Download
  gebaut wurde.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Was ist neu gegenüber Mandelbulb3D

* **Die Grafikkarte berechnet das Bild** — das Bild selbst, die Schatten,
  die Umgebungsverdeckung (Ambient Occlusion), die Reflexionen und die
  Monte-Carlo-Renderings. Auf einem typischen Computer ist das um ein
  Mehrfaches schneller als der Prozessor allein; siehe [Geschwindigkeit](#speed) unten.
* **Auto-Modus**: Grafikkarte und Prozessor arbeiten bei jedem Bild
  zusammen, und nie wartet etwas — ist die Karte noch nicht bereit, beginnt
  der Prozessor trotzdem, und die Karte steigt dazu ein.
* **Eine LED in der Titelleiste** zeigt, wo das letzte Bild berechnet wurde.
* **Oberflächengröße** von 75 bis 200 % für hochauflösende Bildschirme.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="Visuelle Designs bei 150 %"></a>

  <details><summary>Mehr lesen</summary>

  **Einstellungen ▸ Designs ▸ Oberflächengröße** macht alle Fenster und
  ihren Text größer oder kleiner (75 bis 200 %), für hochauflösende
  Bildschirme; die Anordnung bleibt genau wie in MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Visuelle Designs bei 100 %](../img/themes_100.png) | ![Visuelle Designs bei 150 %](../img/themes_150.png) |

  </details>
* **Acht Erscheinungsbilder** für die Fenster, alle als Design-Dateien, die
  Sie kopieren und ändern können: zwei geben das Aussehen von MB3D wieder,
  sechs sind neu.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Das Design Midnight"></a>

  <details><summary>Mehr lesen</summary>

  **Einstellungen ▸ Designs** bietet acht Erscheinungsbilder, jedes eine Datei
  im Ordner `Themes`: *Glossy* (der dunkle Standard) und *Windows* geben die
  beiden Erscheinungsbilder von MB3D wieder, die anderen sechs sind neu.
  Klicken Sie auf ein Bild, um es in voller Größe zu sehen.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: der Standard von MB3D | **Windows**: „Themes off“ von MB3D |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: helleres neutrales Grau, stahlblaue Akzente | **Daylight**: warmes helles Grau, petrolfarbene Akzente |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: tiefes Marineblau, blaue Akzente | **Ember**: warmes Anthrazit, orange Akzente |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="Hoher Kontrast"></a> |
  | **Forest**: dunkles Grüngrau, grüne Akzente | **High contrast**: Schwarz, weißer Text, gelbe Hervorhebungen |
  
  Ein Design ist eine kleine Textdatei im Ordner `Themes` neben dem Programm;
  kopieren Sie eine, ändern Sie ihre Farben, und sie erscheint beim nächsten
  Start in der Liste.

  </details>
* **Ein Absturz hinterlässt auf jedem System eine Notiz**: ein Panic des
  Programms (oder der Fehler, mit dem es endet) wird an `mb3d_crash.txt`
  angehängt, neben dem Programm oder in seinem Datenordner (früher nur unter
  Windows). Schicken Sie diese Datei mit einem Fehlerbericht mit.
* **Alle Grafikkarten des Computers rechnen**, ohne dass etwas einzustellen
  ist: jede gefundene Karte nimmt sich Bänder von Bildzeilen aus demselben
  Vorrat wie die erste Karte und der Prozessor, sodass eine schnellere Karte
  einfach mehr nimmt; Schatten, Verdeckung, Monte-Carlo-Strahlen und
  Reflexionen werden genauso geteilt, und eine Animation rendert gleichzeitig
  ein Bild pro Karte. Eine langsame Karte (integrierte Grafik neben einer
  eigenständigen) tut, was sie kann, und hält nie ein Bild auf: Sobald die
  freien Zeilen vergeben sind, übernehmen die untätigen Karten und der
  Prozessor die Zeilen, die sie noch nicht geliefert hat. Eine Karte, die
  versagt, wird ausgelassen, die anderen machen weiter.
  `mb3d --list-cards` zeigt, was gefunden wurde. (Getestet mit einer Karte,
  die zweimal geöffnet wurde, und mit einem Software-Vulkan-Treiber als
  zweiter Karte; noch nicht auf einem Computer mit zwei echten Karten.)
* **Ein unfertiges Bild ist eine `.m3i`-Datei.** Was ein angehaltenes,
  abgestürztes oder abgebrochenes Rendering berechnet hat, bleibt im Cache
  erhalten (das Fenster Unfertige Renderings, `mb3d resume`); bei einem Bild
  ist diese Datei nun zu jedem Zeitpunkt eine `.m3i`-Bilddatei, `image.m3i`
  im Ordner des Renderings, mit den noch nicht berechneten Zeilen als
  Hintergrund. Kopieren Sie sie heraus, wann immer Sie möchten, oder drücken
  Sie *Save m3i* im Fenster Unfertige Renderings (`mb3d
  resume --m3i KEY file.m3i` auf der Kommandozeile; beide schreiben auch die
  `data.bin` einer früheren Version als `.m3i`), und öffnen Sie sie hier oder
  in Mandelbulb3D, ohne Umwandlung. (Monte-Carlo-Renderings führen eigene
  Aufzeichnungen; die `.m3c` eines Durchgangs wird mit `--m3c` geschrieben.)
* **Hotkeys für die Fenster**: eine Tastenkombination holt aus jedem Fenster
  das Hauptfenster, den Navigator, die Animation, Formeln, Beleuchtung,
  Nachbearbeitung oder das Monte-Carlo-Fenster nach vorne (Strg+1 bis
  Strg+7), und Strg+W schließt das Fenster, in dem Sie sind (unter macOS
  Cmd). Eigene legen Sie unter *Einstellungen > Hotkeys* fest: in ein Feld
  klicken und die Tasten drücken, Rücktaste entfernt einen Hotkey.
* **Die Oberfläche in Ihrer Sprache**: jeder Text der Fenster stammt aus
  einer Sprachdatei; zwölf Sprachen werden mitgeliefert, und das Programm
  schreibt die Datei einer neuen Sprache, damit Sie sie übersetzen können.

  <details><summary>Mehr lesen</summary>

  Das Sprachsymbol am rechten Ende der Titelleiste des Hauptfensters zeigt
  die Sprache der Oberfläche (`EN`); ein Klick öffnet das Sprachfenster.
  Eine Sprache ist eine Textdatei im Ordner `Languages` neben dem Programm.
  Mitgeliefert: English, Deutsch, Français, Italiano, Nederlands, Polski,
  Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文 und 繁體中文 (jede mit ihrer Flagge;
  die ostasiatischen bringen eine Teilmenge von Noto Sans CJK für ihre Zeichen
  mit, Hindi bringt Noto Sans Devanagari mit, dessen verbundene Buchstaben und
  Vokalzeichen mit dem Textformer rustybuzz gesetzt werden).
  Die Übersetzungen wurden maschinell erstellt und auf ihre Struktur geprüft,
  nicht von Muttersprachlern: Korrekturen sind willkommen, jede ist eine Zeile
  der Datei. Zum Übersetzen: geben Sie den Namen Ihrer Sprache ein und drücken
  Sie *Sprachdatei erstellen*; das Programm schreibt `Languages/<name>.lng`
  mit jedem Text auf Englisch, einer pro Zeile (`Button2.Caption = Calculate
  3D`). Übersetzen Sie die Texte rechts vom `=` mit einem Texteditor und
  drücken Sie *Neu laden*: die Fenster folgen sofort, ohne Neustart. Eine
  ausgelassene Zeile behält den englischen Text. *Speichern + Beenden* behält
  die Sprache für den nächsten Start.

  </details>
* **Linux, Windows und macOS** (Intel und Apple Silicon).
* **Ein Kommandozeilenprogramm** für alles, was kein Fenster braucht.
* **Sehr große Bilder in einem Stück**: 15000 x 20000 Pixel und mehr, ohne
  dass der Speicher ausgeht, und die Tiefenschärfe nutzt alle
  Prozessorkerne (auf 24 Threads 11-mal schneller als vor 0.2.45, mit
  demselben Bild).
* **16-Bit-Bilder:** PNG-Dateien mit 65.536 statt 256 Stufen pro Farbe, für
  die Bearbeitung ohne Streifenbildung.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="16-Bit-Bilder"></a>

  <details><summary>Mehr lesen</summary>

  MB3D speichert 8-Bit-Bilder: 256 Stufen pro Farbe. Zum Anschauen ist das
  reichlich, aber wenn ein Bild danach bearbeitet wird — Helligkeit, Kontrast,
  Kurven —, zerfallen glatte Flächen wie der Hintergrund oder Nebel in
  sichtbare Streifen. **Bild speichern ▸ PNG** bietet einen zweiten Dateityp,
  **PNG 16 Bit Farbe**: die Farben so, wie das Programm sie berechnet hat,
  bevor sie auf 256 Stufen gerundet werden, mit allen Schatten, Reflexionen
  und der Tiefenschärfe. Die Datei sieht gleich aus und bleibt glatt, wenn sie
  bearbeitet wird.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="Ein Hintergrundverlauf nach einer starken Kontrasterhöhung: Stufen in 8 Bit, glatt in 16 Bit"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="Der Speichern-Dialog mit „PNG 16 Bit Farbe“"></a>

  </details>
* **Ein Rauschfilter** für Monte Carlo und alle anderen Bilder, die aus GIMP
  bekannte DCT-Entrauschung, und für Monte-Carlo-Renderings außerdem ein
  neuronaler, Intel Open Image Denoise.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="Rauschfilter"></a>

  <details><summary>Mehr lesen</summary>

  Ein **Rauschfilter** entfernt Körnung aus den Bildern: die **DCT-Entrauschung**
  aus GIMPs *Filter ▸ Verbessern ▸ Rauschminderung ▸ Denoise DCT*, die von
  GIMPs Filtern am besten zu diesen Bildern passte (siehe die Danksagungen
  unten).
  
  * **Monte-Carlo-Fenster ▸ -Rauschen** (eine Schaltfläche, die MB3D versteckt
    hatte): filtert das angezeigte und das gespeicherte Bild, mit einer Stärke,
    die sich nach den Strahlen pro Pixel richtet. Das Rendern selbst läuft
    ungefiltert weiter, und seine nächste Aktualisierung zeigt es wieder
    ungefiltert.
  * **Nachbearbeitung ▸ Rauschfilter**: *Jetzt filtern* filtert das Bild des
    Hauptfensters; *Neue Bilder automatisch filtern* filtert jedes neue.
    *Stärke* ist die Stärke des zu entfernenden Rauschens (4 ist leicht;
    höhere Werte entfernen mehr Körnung und mehr feine Details).
  
  Monte-Carlo-Bilder haben die meiste Körnung. Im Vergleich mit Bildern, die
  mit 512 Strahlen pro Pixel gerendert wurden, lagen die gefilterten Bilder in
  allen 13 Tests näher als die ungefilterten (vier Szenen, 4 bis 32 Strahlen):
  um bis zu 23 % bei 4 Strahlen, 5 bis 10 % bei 8 Strahlen, 1 bis 3 % bei
  32 Strahlen, wo nur noch wenig Körnung übrig ist.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="Ein Monte-Carlo-Bild mit 4 Strahlen pro Pixel, dasselbe mit dem Rauschfilter und mit 512 Strahlen"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="Nachbearbeitung: der Abschnitt Rauschfilter"></a>

  Der **neuronale Rauschfilter** ist
  [Intel Open Image Denoise](https://www.openimagedenoise.org), der Filter
  von Blenders Cycles und vielen anderen Pathtracern, trainiert an
  gerenderten Bildern. Er filtert die Mittelwerte der Strahlen des Renderings
  (vor Belichtung und Sättigung), sodass das Bild seine Helligkeit behält.
  Intels Bibliothek wird mit dem Programm geliefert (der Ordner `oidn`; im
  .deb und .rpm `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` kann eine andere
  angeben.

  * **Monte-Carlo-Fenster ▸ -Rauschen KI**: filtert das angezeigte und das
    gespeicherte Bild wie *-Rauschen*; das Rendern läuft ungefiltert weiter.
  * **Kommandozeile**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * Er braucht auf dem Prozessor 0,2 bis 0,6 Sekunden für ein Bild von
    640 x 480.

  Die Abweichung vom Bild mit 512 Strahlen (Mittel über alle Pixel, niedriger
  ist besser):

  | Szene | 4 Strahlen | DCT-Filter | neuronaler Filter | 8 Strahlen | DCT-Filter | neuronaler Filter |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  Der neuronale Filter liegt in drei der vier Szenen am nächsten; bei der
  feinen Körnung der Julia-Menge liegt der DCT-Filter etwas näher. Am
  deutlichsten zeigt er sich bei Glas und Spiegeln, wo wenige Strahlen die
  meiste Körnung hinterlassen:

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="Ein Monte-Carlo-Bild mit 4 Strahlen pro Pixel, mit dem DCT-Filter, mit dem neuronalen Filter und mit 512 Strahlen"></a>

  </details>
* **Transparente Hintergründe** für Bilder und Animationen.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="Transparenter Hintergrund"></a>

  <details><summary>Mehr lesen</summary>

  Bilder und Animationsbilder können mit **transparentem Hintergrund**
  gespeichert werden, um das Fraktal auf ein anderes Bild oder in ein
  Videoschnittprogramm zu bringen: **Bild speichern ▸ PNG ▸ PNG mit
  transparentem Hintergrund**, und im Animationseditor **Transparenter
  Hintergrund** (PNG-Bilder). Mit Kantenglättung (ein Anzeigemaßstab von 1:2
  oder mehr, oder `--aa`) sind die Kanten weich und tragen keinen Saum der
  alten Hintergrundfarbe. Tiefenbilder für das Compositing gab es schon
  (*Auch Z-Buffer speichern*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="Ein Bild mit seinem Hintergrund und mit transparentem Hintergrund über einem Schachbrettmuster"></a>

  </details>
* **Videos von Animationen:** eine AVI-Datei direkt aus dem Programm, oder
  der ffmpeg-Befehl für ein kleines MP4.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="Video einer Animation"></a>

  <details><summary>Mehr lesen</summary>

  Im Animationseditor fügt **Auch ein Video erstellen (AVI)** die Bilder beim
  Rendern zu einem Video zusammen, mit den eingestellten **Bilder pro
  Sekunde**: `<project name>.avi` im Ausgabeordner, neben den Einzelbildern,
  die erhalten bleiben. Es läuft in VLC und mpv und lässt sich in
  Videoschnittprogrammen öffnen (Kdenlive, Shotcut, DaVinci Resolve). Jedes
  Bild wird als ganzes Bild gespeichert, daher sind die Dateien groß: gut zum
  Schneiden und Aufbewahren, weniger zum Teilen.
  
  Für ein kleines Video zum Teilen legt **FFmpeg-Befehl** den Befehl für ein
  MP4 (oder, mit transparentem Hintergrund, ein WebM) in die Zwischenablage und
  in die Meldungen. Installieren Sie [ffmpeg](https://ffmpeg.org), fügen Sie
  den Befehl in ein Terminal ein, wenn die Bilder fertig sind, und es erzeugt
  das Video; dieses Programm startet selbst keine anderen Programme.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="Der Animationseditor mit Auch ein Video erstellen (AVI), Bilder pro Sekunde und FFmpeg-Befehl"></a>

  </details>
* **Eine Vorschau von Parameterdateien** im Öffnen-Dialog.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="Vorschau beim Öffnen von Parametern"></a>

  <details><summary>Mehr lesen</summary>

  **Öffnen ▸ m3p öffnen** und **m3i öffnen** zeigen ein kleines Bild der
  angeklickten Parameterdatei, mit ihrer Größe und ihren Formeln, bevor sie
  geöffnet wird; mit den Pfeiltasten bewegen Sie sich samt Vorschau durch die
  Liste. Das Bild wird im Hintergrund auf dem Prozessor berechnet, sodass der
  Dialog bedienbar bleibt, und die Vorschau jeder Datei bleibt erhalten,
  solange das Programm läuft.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="Der Öffnen-Dialog mit der Vorschau einer Parameterdatei"></a>

  </details>
* **Navigator: genaue Werte und Ansicht zurücksetzen.** Einen Wert anklicken
  und eintippen; zu der Ansicht zurückkehren, mit der der Navigator begann.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="Navigator: genaue Werte und Ansicht zurücksetzen"></a>

  <details><summary>Mehr lesen</summary>

  * **Klicken Sie auf einen Wert** in den Feldern des Navigators — Julia-Werte,
    Formelwerte, 4D-Drehung, die sonstigen Werte — und tippen Sie die genaue
    Zahl ein, wie in MB3D (Winkel in Grad). Der Regler macht dann ab diesem
    Wert weiter.
  * **Klicken Sie auf den Zoom** und tippen Sie ihn ein, wenn die Zoomstufen zu
    grob sind (ein Wunsch aus der Issue-Liste von MB3D); ein eingetippter Zoom
    bleibt bestehen (*Fester Zoom*).
  * **Ansicht zurücksetzen** kehrt zu Position, Richtung und Zoom zurück, die
    der Navigator aus dem Hauptfenster übernommen hat; die angepassten Werte
    bleiben.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="Der Navigator mit den Formelwerten und dem Eingabefeld eines angeklickten Wertes"></a>

  </details>
* **Eine neue Formel, SphereTree:** ein Baum aus sich berührenden Kugeln, die
  sich in immer kleinere verzweigen, von Tom Lowe.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="Eine neue Formel: SphereTree"></a>

  <details><summary>Mehr lesen</summary>

  **SphereTree** ist eine Formel, die Mandelbulb3D nicht hat: ein Baum aus
  Kugeln, die einander berühren und sich in immer kleinere Kugeln verzweigen,
  wie Schaum oder Koralle. Tom Lowe, der Erfinder der Mandelbox, hat sie 2020
  veröffentlicht (siehe die Danksagungen unten). Sie steht in der Formelliste
  bei den dIFS-Formeln und hat vier Einstellungen:
  
  * **PackRatio** (0 bis 1,22): die Größe der Kugeln; bei 1 berühren sie sich
    gerade, über 1 brechen die äußeren ab.
  * **Levels**: wie tief der Baum reicht (6 genügt für die meisten Bilder).
  * **Rotated**: 1 dreht jede Ebene um ein Zwölftel einer Umdrehung, und der
    Baum wird zu einer runden Kugel aus Kugelhaufen.
  * **Colour scale**: jeder Zweig des Baums hat seine eigene Farbe. Wählen Sie
    *2. Wahl* (die Orbit-Trap) in den Farbeinstellungen der Beleuchtung, um sie
    zu sehen; diese Einstellung verteilt sie über den Farbverlauf.
  
  Sie funktioniert allein und nach dIFS-Transformationen (das dritte Bild
  wiederholt sie mit *tilingbrickIFS*) und wird auf der Grafikkarte ebenso wie
  auf dem Prozessor berechnet. Stellen Sie die Iterationen auf 1: die Formel
  berechnet den ganzen Baum auf einmal. Parameter mit SphereTree lassen sich
  nur in diesem Programm öffnen, nicht in Mandelbulb3D.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="Eine SphereTree-Kugel (Rotated) im Monte-Carlo-Renderer, in der Dämmerung"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="Eine Nahaufnahme des Kugelschaums von SphereTree"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="SphereTree, wiederholt mit tilingbrickIFS: ein Schwarm von Kugelbäumen"></a> |
  |:---:|:---:|

  </details>
* **Eine neue Formel, DomeTree:** eine Kuppel, aus der ein Ring kleinerer
  Kuppeln wächst, jede mit ihrem eigenen Ring, von Tom Lowe.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="Eine neue Formel: DomeTree"></a>

  <details><summary>Mehr lesen</summary>

  **DomeTree** ist Tom Lowes zweiter Baum nach SphereTree (siehe die
  Danksagungen): eine Kugel, aus deren einer Seite ein Ring von Kuppeln wächst,
  jede Kuppel mit ihrem eigenen kleineren Ring, Ebene um Ebene, gebildet aus
  Kreisinversionen. Sie steht in der Formelliste bei den dIFS-Formeln und hat
  fünf Einstellungen:

  * **Levels**: wie tief der Baum reicht (16 genügt).
  * **Children**: die Kuppeln in jedem Ring, 4 bis 10.
  * **Bend angle** (0,3 bis 1,3 funktionieren am besten): wie stark sich die
    Kuppeln wölben; niedrige Werte ergeben flache Scheiben, hohe runde
    Kuppeln. Über etwa 1,3 wird die Oberfläche körnig.
  * **Colour scale**: jede Kuppel hat ihre eigene Farbe. Wählen Sie *2. Wahl*
    (die Orbit-Trap) in den Farbeinstellungen der Beleuchtung und einen
    breiten Farbbereich (zum Beispiel 0 bis 100), um sie zu sehen.
  * **Start curved**: 1 (der Standard) beginnt mit der Kugel, 0 mit einem
    flachen Boden, aus dem der erste Ring wächst.

  Verwenden Sie einen kleinen *Raystep* (0,2; 0,05 bei 9 oder 10 Kindern): die
  Abstandsschätzung der Form ist grob, wie in TGlads eigener
  Fragmentarium-Datei. Wie SphereTree läuft sie auf der Grafikkarte und dem
  Prozessor, braucht nur 1 Iteration, und Parameter damit lassen sich nur in
  diesem Programm öffnen.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="DomeTree mit 8 Kindern im Monte-Carlo-Renderer, pro Kuppel eingefärbt"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 5 Kinder, Biegewinkel 1,2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 6 Kinder, Biegewinkel 0,5: flache Kuppeln"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 10 Kinder: Spitzenmuster zwischen den Kuppeln und eine große Kugel"></a> |
  |:---:|:---:|:---:|

  </details>
* **Eine neue Formel, UlamSpiral:** Ulams Spirale der Primzahlen in 3D, als
  Stadt, Skyline, Kugeln oder Block.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="Eine neue Formel: UlamSpiral"></a>

  <details><summary>Mehr lesen</summary>

  1963 saß der Mathematiker Stanisław Ulam auf einer Tagung einen „langen und
  sehr langweiligen Vortrag“ ab und kritzelte auf ein Blatt Karopapier. Zuerst
  wollte er Schachaufgaben aufstellen; stattdessen nummerierte er die Kästchen
  1, 2, 3, … in einer Spirale, die 1 in der Mitte, und kreiste die Primzahlen
  ein. Die Primzahlen reihten sich auf diagonalen Linien auf, als folgten sie
  einem Muster, und sie tun es noch, wenn ein Computer Millionen davon zeichnet
  (Ulam probierte 100.000 auf dem MANIAC II in Los Alamos aus; Martin Gardner
  brachte das Bild 1964 auf die Titelseite des Scientific American). Bis heute
  hat es niemand vollständig erklärt.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="Die Ulam-Spirale: die Primzahlen unter den Zahlen 1 bis 78.961 auf einer quadratischen Spirale, mit dem Anfang der Spirale in der Ecke eingezeichnet"></a>
  
  **UlamSpiral** baut diese Spirale in 3D, eine Form pro Zahl, in vier
  Gestalten (*Mode*):
  
  * **0, Primzahltürme**: eine Platte für jede Zahl und ein Turm auf jeder
    Primzahl. Die diagonalen Linien der Primzahlen werden zu Reihen von Türmen.
  * **1, Teiler-Skyline**: eine Säule für jede Zahl, umso höher, je mehr Teiler
    sie hat; die Primzahlen sind die niedrigen Straßen dazwischen.
  * **2, Teilerkugeln**: eine Kugel für jede Zahl, umso größer, je mehr Teiler
    sie hat (die Primzahlen sind die kleinen).
  * **3, ein Block aus Primzahlen**: die Zahlen um Würfelschalen gewickelt statt
    um ein Quadrat, ein Würfel auf jeder Primzahl.
  
  *Rings* legt die Größe der Spirale fest (60 Ringe sind die Zahlen bis
  14.641), *Height* die Höhe der Türme oder Säulen, *Gap* den Abstand zwischen
  ihnen. Mit *2. Wahl* (der Orbit-Trap) in den Farbeinstellungen folgt die
  Farbe der Zahl der Teiler, sodass die Primzahlen eine eigene Farbe haben. Wie
  SphereTree läuft sie auf der Grafikkarte und dem Prozessor, funktioniert nach
  dIFS-Transformationen, braucht nur 1 Iteration, und Parameter damit lassen
  sich nur in diesem Programm öffnen.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, Primzahltürme: die diagonalen Reihen der Primzahlen als Stadt, im Monte-Carlo-Renderer mit Nebel, harten Schatten und Tiefenschärfe"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, Teiler-Skyline"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, Teilerkugeln"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, ein Block aus Primzahlen auf Würfelschalen"></a> |
  |:---:|:---:|:---:|

  </details>
* **Ein angehaltenes Rendering macht dort weiter, wo es war.** Was ein
  Rendering berechnet hat, bleibt erhalten, wenn es angehalten wird, und auch,
  wenn das Programm geschlossen wird oder abstürzt oder der Computer ausgeht
  (es wird alle 5 Minuten auf die Festplatte gesichert): 3D berechnen, Monte
  Carlo (auch mitten in einem Durchgang), das Bild einer Animation oder das
  Bild eines laufenden Stapels, ein Voxel-Export, ein Bulb-Tracer-Mesh, die
  Kacheln eines großen Renderings und ein Schwarm-Rendering (Stopp auf dem
  Computer, der den Auftrag erstellt hat, behält jeden fertigen Teil, und jeder
  Computer behält den Teil, an dem er war). 3D berechnen und Monte Carlo mit
  denselben Parametern fragen, ob sie fortsetzen sollen, die anderen setzen von
  selbst fort; die Schaltfläche **Fortsetzen** (Registerkarte Öffnen) listet
  die unfertigen Renderings auf, um sie fortzusetzen oder zu löschen. Auf der
  Kommandozeile setzt derselbe Befehl von selbst fort (auch ein Bild in
  `--tiles`, `montecarlo`, `animate`, `batch`, `voxel` und `mesh`); `--fresh`
  beginnt neu, `mb3d resume` listet sie auf. Ein fortgesetztes Rendering ist
  dasselbe wie eines, das in einem Zug berechnet wurde.
* **Rendern, aufgeteilt auf Ihre Computer**, ein „Schwarm“: Computer, die einen
  gemeinsamen Ordner sehen, rendern zusammen ein Bild, ein großes Rendering,
  ein Monte-Carlo-Bild oder eine Animation. Siehe
  [Rendern auf mehreren Computern](#rendering-on-several-computers-the-swarm).
* **Eine neue Formel, PseudoKleinian:** die „Kleinsche Architektur“ aus Hallen,
  Bögen und Kugeln als eine Formel mit eigenem exaktem Abstand.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="Eine neue Formel: PseudoKleinian"></a>

  <details><summary>Mehr lesen</summary>

  Die Hallen, Bögen, Säulen und verschachtelten Kugeln, bekannt als „Kleinsche
  Architektur“ (Kleinian architecture), gehören zu den beliebtesten
  3D-Fraktalbildern. In MB3D lassen sie sich nur als Hybrid mehrerer Formeln
  mit sorgfältig abgestimmten Einstellungen erzeugen, nach dem Rezept, das
  Theli-at 2011 fand; knighty schrieb dann dasselbe als eine Formel mit
  eigener Abstandsschätzung, und das ist **PseudoKleinian**. Jede Ebene faltet
  den Punkt in eine Box, invertiert ihn an einer Kugel und addiert einen
  Versatz, und der Abstand wird nach den Faltungen exakt zu einer Bodenebene
  gemessen, sodass die Wände sauber herauskommen und das Bild schnell rendert.
  Die Einstellungen sind die von knighty: *Levels* (8), *Size* (die
  Inversionskugel), *CSize X/Y/Z* (die Box; seine 0.924, 0.908, 0.924 ergeben
  die klassischen Hallen, und kleine Änderungen ergeben neue Gebäude),
  *Julia X/Y/Z* (ein Versatz pro Ebene), *Shape* (0 die Bodenebene, 1 eine
  Zylinder-Ebene der Dicke *Thickness* durch *Offset*, 2 eine abgerundete Box),
  *DE offset* (höhlt die Oberfläche aus) und *Colour scale* (mit *2. Wahl*, der
  Orbit-Trap: der kleinste Orbit-Radius, die klassische Kleinsche Bänderung).
  
  Die Struktur füllt den ganzen Raum, also betrachten Sie sie von innen:
  beginnen Sie mit dem Ansichtsmittelpunkt nahe dem Ursprung und der Kamera
  ein oder zwei Einheiten zurück, und bewegen Sie sich mit dem Navigator durch
  die Hallen. Im Monte-Carlo-Renderer bekommt ein Innenraum fast kein
  Himmelslicht, also setzen Sie ein Positionslicht in die Halle (das erste
  Bild) oder schneiden Sie die Struktur mit einer Schnittebene auf und schauen
  Sie von oben hinein (das zweite). Sie läuft auf der Grafikkarte und dem
  Prozessor, braucht 1 Iteration, funktioniert nach dIFS-Transformationen und
  in DE-Kombinationen wie die anderen dIFS-Formeln (auch wenn diese innerhalb
  einer raumfüllenden Struktur wenig ändern), und Parameter damit lassen sich
  nur in diesem Programm öffnen.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="PseudoKleinian aus knightys Blickwinkel: eine Halle aus Bögen und verschachtelten Kugeln, von einer Lampe im Inneren beleuchtet, im Monte-Carlo-Renderer mit Nebel, harten Schatten und Tiefenschärfe"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="PseudoKleinian, mit einer Schnittebene aufgeschnitten und von oben gesehen: das Gitter aus Kugeln und die Zellen dazwischen"></a>

  </details>
* **Stereopaare in einem Schritt**: beide Augenbilder gerendert und
  nebeneinander gezeigt, für den Parallel- oder Kreuzblick. Beigetragen von
  Rathinagiri S ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Ein Stereopaar von Dainbramages Hydra, nebeneinander für den Parallelblick"></a>

  <details><summary>Mehr lesen</summary>

  **Stereo ▸ Stereopaar rendern** rendert erst das Bild für das linke, dann
  das für das rechte Auge mit den Stereo-Einstellungen der Registerkarte und
  zeigt sie nebeneinander im Hauptfenster. Sie sind für den Parallelblick
  angeordnet (linkes Bild links); **Kreuzblick-Vorschau** vertauscht sie für
  den Kreuzblick. Es ist eine Vorschau: Beleuchtung und Farben
  lassen sich danach nicht ändern, ohne neu zu rendern, und es funktioniert
  nicht mit Kacheln. Die Schaltflächen für die einzelnen Augenbilder gibt es
  weiterhin.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Ein Stereopaar von Dainbramages Hydra, nebeneinander für den Parallelblick"></a>

  </details>
* **Restzeit für das ganze Bild**: der Balken unter dem Bild füllt sich
  einmal über die ganze Berechnung, und daneben steht die Zeit, bis das Bild
  fertig ist, einschließlich Schatten, Verdeckung und Reflexionen.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="Der Fortschrittsbalken und die gesamte Restzeit"></a>

  <details><summary>Mehr lesen</summary>

  Der Balken von MB3D beginnt bei jeder Stufe von vorn. Hier zeigt er die
  ganze Berechnung: das Hauptrendering, dann die harten Schatten und die
  Umgebungsverdeckung, die Reflexionen und die Tiefenschärfe. Wie lange die
  Stufen nach dem Hauptrendering dauern, wird daraus geschätzt, was die Szene
  verlangt und wie das Hauptrendering lief; die Schätzung wurde an etwa 1400
  Renderings von 366 Szenen angepasst. Ein erneut berechnetes Bild erwartet die
  Zeiten, die es beim letzten Mal gebraucht hat.
  
  Das erste Bild mit neuen Formeln auf der Grafikkarte wartet, bis seine
  Shader kompiliert sind: die Zeile sagt dann *Shader werden kompiliert* mit
  der Restzeit, die das Tempo des Prozessors zugrunde legt und, wenn die
  Shader rechtzeitig bereit sein werden, das Einsteigen der Karte. Details in
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="Das Hauptfenster während der harten Schatten und der Umgebungsverdeckung, mit der gesamten Restzeit"></a>

  </details>
* **History** hinter dem Uhrsymbol der Titelleiste: jedes berechnete Bild mit
  seinem Vorschaubild, jede geöffnete oder gespeicherte Datei, als Liste, als
  Zeilen oder als Kacheln beliebiger Größe, nach Tagen gruppiert oder am
  Stück; durchsuchen, sortieren, nur die Bilder zeigen, die sich
  unterscheiden, und jeden Eintrag mit einem Doppelklick laden. Eine History
  mit Hunderttausenden von Einträgen öffnet sich sofort.
* **Die Beleuchtung bleibt live** nach harten Schatten und Reflexionen:
  verschieben Sie ein Licht oder ändern Sie seine Farbe, und das Bild ändert
  sich sofort, wie in Mandelbulb3D.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="Das Hauptfenster und das Fenster Beleuchtung direkt nach einer Änderung der Beleuchtung"></a>

  <details><summary>Mehr lesen</summary>

  Rendern Sie schlicht und probieren Sie dann Lichtpositionen und Farben aus:
  jede Änderung im Fenster Beleuchtung zeichnet sofort neu, mit den Schatten,
  so wie sie berechnet wurden, wie in Mandelbulb3D. Eine Änderung der Farbe
  oder Intensität eines Lichts berechnet die Schatten nie neu. Die Schatten
  eines verschobenen Lichts, die Reflexionen und die Tiefenschärfe kommen mit
  den Schaltflächen des Fensters Nachbearbeitung zurück, wie in Mandelbulb3D.
  
  Eine angehaltene Berechnung lässt sich auch dann noch fortsetzen, wenn Sie
  die Beleuchtung geändert haben. Eine Arbeitsweise damit: [Arbeitsablauf Beleuchtung](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-de)
  im Wiki. Details in
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="Das Hauptfenster und das Fenster Beleuchtung direkt nach einer Änderung der Beleuchtung: das Bild schon heller"></a>

  </details>

<a id="download"></a>

## Download

Fertige Programme der neuesten Version, ohne Installation:

| System | Download |
|---|---|
| Windows (64 Bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 Bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (wird wie andere Programme installiert) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (wird wie andere Programme installiert) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple Silicon (M1 und neuer) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Entpacken Sie das Archiv und starten Sie **Mandelbulb3D** (`Mandelbulb3D.exe`
unter Windows) im entpackten Ordner. Der Ordner ist aufgebaut wie eine
MB3D-Installation, mit den Formeln und Maps von MB3D und 80
Beispiel-Parameterdateien.

* Wenn Windows „Der Computer wurde durch Windows geschützt“ anzeigt, klicken
  Sie auf *Weitere Informationen* ▸ *Trotzdem ausführen*.
* macOS blockiert heruntergeladene Programme, die nicht signiert sind: führen
  Sie einmal `xattr -dr com.apple.quarantine .` im entpackten Ordner aus.

**Alternative für Linux und macOS:** statt von Hand herunterzuladen, holt
diese eine Zeile die neueste Version nach `~/mb3d-rs` (führen Sie sie später
erneut aus, um zu aktualisieren; das Skript [download.sh](../../download.sh)
liegt auch im Ordner des Programms):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Es prüft den Download vor dem Entpacken, legt **Mandelbulb3D** in Ihr
Anwendungsmenü (Linux) und richtet die Befehle `Mandelbulb3D` und `mb3d` ein,
die Sie in jedem Ordner eingeben können. `sh ~/mb3d-rs/download.sh
--uninstall` entfernt den Menüeintrag und die Befehle wieder; Ihre Dateien
bleiben.

Die Linux-Programme laufen auf Distributionen ab etwa 2018 (Ubuntu 18.04,
Debian 10, Rocky/RHEL 8 und neuer).

**Wie andere Programme installiert:** unter Debian, Ubuntu und Mint öffnen
Sie die `.deb`-Datei oben (oder `sudo apt install ./mb3d-rs-linux-x64.deb`);
unter Fedora, RHEL und Rocky die `.rpm`-Datei (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, unter openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D steht dann im Anwendungsmenü, und `mb3d` funktioniert in jedem
Ordner; Ihre Einstellungen und die History werden in `~/.config/mb3d-rs` und
`~/.local/share/mb3d-rs` aufbewahrt.

[Erste Schritte](../../GETTING_STARTED.md) führt durch die ersten Schritte;
ältere Versionen und die Versionshinweise stehen auf der Seite
[Releases](https://github.com/thr32423/mb3d-rs/releases).

<a id="guides-and-help"></a>

## Anleitungen und Hilfe

Schritt-für-Schritt-Anleitungen stehen im [Wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-de):

* [Installation](https://github.com/thr32423/mb3d-rs/wiki/Installing-de): Windows, macOS und Linux, Aktualisieren, wo Ihre Dateien liegen
* [Arbeitsablauf Beleuchtung](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-de): schlicht rendern, dann Lichter, Schatten und Reflexionen ausprobieren
* [Schwarm einrichten](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-de): Rendern auf mehreren Computern über einen gemeinsamen Ordner
* [Fehlerbehebung und FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-de): wenn etwas nicht wie erwartet funktioniert

Jede Version mit ihren Änderungen: [die ganze Liste](../UPDATES.md).

<a id="rendering-on-several-computers-the-swarm"></a>

## Rendern auf mehreren Computern (der Schwarm)

Computer, die einen gemeinsamen Ordner sehen, rendern zusammen: ein Bild in
Kacheln, ein großes Rendering, ein Monte-Carlo-Bild, eine Animation oder einen
Stapel. Jeder Computer nimmt sich die Teile, die noch kein anderer hat, sodass
ein schneller mehr davon nimmt, und der Computer, der das Rendering gestartet
hat, setzt die Ergebnisse zusammen. Das Teilen erledigt der Ordner: das
Programm öffnet keine Netzwerkverbindung und spricht mit niemandem, es ist
also so sicher wie Ihre Dateifreigabe.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D mit dem Schwarm-Fenster, geöffnet mit der Schaltfläche Schwarm auf der Registerkarte Tools, während zwei Computer ein Bild in Kacheln rendern: der gemeinsame Ordner, Diesen Computer teilen, die Computer und was jeder tut, der Auftrag und die Vorschau mit gerenderten Kacheln, einer Kachel in Arbeit und fehlenden"></a>

<a id="setting-it-up"></a>

### Einrichten

Schritt für Schritt, mit dem, was zu prüfen ist: [Schwarm einrichten](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-de) im Wiki.

1. **Ein Ordner, in den jeder Computer schreiben kann:** eine Freigabe auf
   einem NAS, eine Windows-Freigabe, eine Samba-Freigabe oder NFS. Unter
   Linux binden Sie ihn als Ordner ein, zum Beispiel
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   eine Freigabe, die nur im Dateimanager geöffnet ist (`smb://...`), ist kein
   Ordner, den das Programm nutzen kann.
2. **Dieselbe Version von mb3d-rs auf jedem Computer.** Ein Computer mit einer
   anderen Version wird aufgelistet, macht aber nicht mit.
3. **In Mandelbulb3D:** die Schaltfläche **Schwarm** auf der Registerkarte
   Tools. Wählen Sie den gemeinsamen Ordner (auf jedem Computer dieselbe
   Freigabe; ihr Pfad darf sich unterscheiden), geben Sie dem Computer einen
   Namen für die Liste und setzen Sie das Häkchen bei **Diesen Computer
   teilen**.

Das ist alles. Die Statuszeile sagt, was geschieht, zum Beispiel *Geteilt mit
omarchy: Renderings werden auf die 2 Computer aufgeteilt*. Allein im Schwarm
wird alles wie gewohnt auf diesem Computer gerendert.

<a id="what-is-shared"></a>

### Was geteilt wird

Solange *Diesen Computer teilen* angehakt ist, rendert der Computer die Teile,
die andere Computer in den Ordner legen, und seine eigenen Renderings gehen an
den Schwarm, sobald ein anderer Computer darin ist:

* **3D berechnen** (und das Fenster Stapelverarbeitung) — das Bild in N x M
  Kacheln (im Schwarm-Fenster eingestellt), hier zusammengesetzt mit allem,
  was die gewöhnliche Berechnung liefert: Änderungen der Beleuchtung,
  Nachbearbeitung, Speichern als m3i (und sein Speicherbedarf, siehe unten).
* **Großes Rendering** — die Kacheln werden einzeln gespeichert, sobald sie
  ankommen; das ganze Bild wird nie im Speicher gehalten, daher ist das der
  Weg für Bilder von 100000 x 100000 Pixeln und mehr.
* **Monte Carlo** — wie auf einem Computer: jeder Computer fügt seinem eigenen
  Block Durchgänge hinzu, bis Sie Stopp drücken, das Bild wird mit den
  Strahlen aller sauberer, und Stopp mittelt die Blöcke. Im Stapelmodus des
  Fensters halten die Blöcke zusammen bei der Strahlenzahl des Stapels an.
* **Der Animationseditor** — die Einzelbilder, eines nach dem anderen.

Das Fenster listet die Computer mit ihren Kernen, ihrer Grafikkarte und dem,
was jeder gerade tut, die Aufträge mit ihrem Fortschritt und eine Vorschau des
neuesten Auftrags: die gerenderten Kacheln, die in Arbeit befindlichen mit
einem Fortschrittsbalken und die noch fehlenden.

<a id="on-the-command-line"></a>

### Auf der Kommandozeile

Starten Sie auf jedem Computer, der helfen soll, einen Worker auf dem Ordner:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Dann rendern Sie auf einem beliebigen Computer mit `--swarm` und demselben
Ordner:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` zeigt, wer im Schwarm ist, was jeder Computer gerade
tut und wie weit die Aufträge sind. Ein Monte-Carlo-Bild auf der
Kommandozeile wird in Blöcke mit eigenen Zufallsstrahlen aufgeteilt: 4 Blöcke
mit 64 Strahlen pro Pixel ergeben die 256, die mit `--rays 256` verlangt
wurden.

<a id="good-to-know"></a>

### Gut zu wissen

* **Wenn die anderen Computer untätig bleiben** und ihre Auftragsliste leer
  ist, während dieser Computer rendert: der Computer, der den Ordner per Samba
  bereitstellt und ihn selbst direkt nutzt, lässt die anderen eine alte
  Auflistung davon behalten. Schalten Sie dieses Caching in
  `/etc/samba/smb.conf` ab: `smb3 directory leases = no` in `[global]` sowie
  `oplocks = no` und `level2 oplocks = no` im Abschnitt der Freigabe, dann
  `sudo systemctl restart smb` und die Freigabe auf den anderen Computern neu
  einbinden.
* **Speicher:** ein Bild braucht in diesem Programm etwa 20 Byte pro Pixel
  (sein Bildpuffer), 10 weitere im Hauptfenster und dann die größere von zwei
  Stufen: die Hauptberechnung mit ihren Schatten und der Verdeckung (16 pro
  Pixel und 1,2 GB für die Karte) oder das Zeichnen (3 pro Pixel, 20 weitere
  mit Tiefenschärfe, und ein Band der Reflexionen: höchstens 3,5 GB, weniger,
  wenn wenig Speicher frei ist, da das Band auf die Hälfte dessen bemessen
  wird, was beim Start des Durchgangs frei ist). Ein Bild von 12000 x 6000:
  6 GB. Wenn ein Bild mehr als den freien Speicher des Computers braucht
  (abgefragt bei Linux, Windows und macOS), sagt 3D berechnen, wie viel es
  braucht und wie viel frei ist, und fragt vor dem Start nach: die Berechnung
  würde auslagern, und das Programm könnte minutenlang nicht reagieren. Ein
  großes Rendering, allein oder mit dem Schwarm, ist der Ausweg: seine Kacheln
  werden einzeln gespeichert, das ganze Bild wird nie gehalten. 3D berechnen
  mit dem Schwarm hält das Bild hier, wie es die gewöhnliche Berechnung tut
  (ohne die Stufe der Hauptberechnung, die die Computer der Kacheln mit dem
  Speicher der jeweiligen Kachel erledigen); ein Computer des Schwarms
  überlässt eine Kachel, die für seinen Speicher zu groß ist, den anderen.
* **Sicherheit:** ein Computer rendert nur, was ein Auftrag verlangt, und
  schreibt nur in den Ordner des Auftrags; Formelcode aus einem Auftrag läuft
  wie jede Formel im Formel-Emulator.
* **Geschwindigkeit:** Kacheln werden als vollständige Bildpuffer übertragen,
  bei einem sehr großen Bild einige hundert Megabyte, daher lohnt sich eine
  schnelle Freigabe. Ein Synchronisierungsdienst, der mit Verzögerung kopiert,
  funktioniert auch, aber langsamer.
* **Computer, die wegfallen:** den Teil, den ein Computer gerendert hat,
  übernimmt ein anderer, sobald eine halbe Minute nichts mehr von ihm zu hören
  war. Ein Auftrag, dessen Besitzer seit einer halben Stunde weg ist, wird
  von dem entfernt, der ihn sieht. Ein Auftrag, der auf einem Computer
  fehlschlägt, wird im Fenster vermerkt und den anderen überlassen.
* **Aufhören:** das Häkchen bei *Diesen Computer teilen* zu entfernen lässt
  den laufenden Teil erst fertig werden; das Programm zu schließen gibt ihn
  auf, und ein anderer Computer übernimmt ihn.
* **Den Auftrag anhalten** (Stopp auf dem Computer, der ihn erstellt hat, oder
  dieser wird geschlossen oder stürzt ab): jeder bereits fertige Teil bleibt
  auf diesem Computer erhalten, und jeder Computer behält den Teil, den er
  gerade gerendert hat. Wird dasselbe Rendering erneut gestartet, entsteht ein
  Auftrag nur aus den fehlenden Teilen, und ein Computer, der einen Teil
  zurückbekommt, den er begonnen hatte, setzt ihn fort. Auf der Kommandozeile
  funktioniert es genauso: denselben Befehl erneut ausführen. Ein offenes
  Monte-Carlo-Rendering (das des Fensters) setzt seine Blöcke beim Anhalten
  zusammen, wie bisher.

<a id="how-the-rendering-works"></a>

## Wie das Rendern funktioniert

Ein Fraktalbild wird Pixel für Pixel berechnet: für jedes Pixel wird ein
Strahl in die Szene verfolgt, bis er die Oberfläche des Fraktals trifft, wobei
die Formel bei jedem Schritt Dutzende Male ausgewertet wird. MB3D macht das
auf dem Prozessor und nutzt alle seine Kerne. mb3d-rs kann dasselbe, und es
kann die Arbeit auch an die **Grafikkarte** übergeben, die Tausende kleiner
Prozessoren hat und genau für diese Art Aufgabe gebaut ist.

Die Einstellung **„Berechnen auf“** (Registerkarte Intern des Hauptfensters)
wählt aus:

| | |
|---|---|
| **Auto** (Standard) | Karte und Prozessor teilen sich das Bild: die Karte nimmt Zeilen von oben, die Threads des Prozessors von unten, bis sie sich treffen. Die Schatten und die Umgebungsverdeckung werden genauso geteilt oder von der Karte allein erledigt, wenn sie deutlich die schnellere ist. |
| **CPU** | Nur der Prozessor, wie in MB3D. |
| **Grafikkarte** | Nur die Karte (sie wartet, bis ihr Programm bereit ist). |

Wenn eine Kombination von Formeln zum ersten Mal verwendet wird, muss der
Grafiktreiber ein Programm für die Karte vorbereiten — den *Shader* —, was auf
dem unten gemessenen Computer 10 bis 15 Sekunden dauert. Im Auto-Modus beginnt
der Prozessor einfach ohne ihn, und die Karte steigt ein, wenn sie bereit ist,
sodass ein kleines Bild nie wartet. Der vorbereitete Shader wird für das
nächste Mal aufbewahrt (vom Grafiktreiber, unter Linux im Ordner
`ShaderCache` neben dem Programm), sodass das nächste Bild mit denselben
Formeln sofort auf der Karte startet.

Die **LED** rechts vom Fenstertitel zeigt, wo das letzte Bild berechnet wurde:
grün auf der Grafikkarte, orange teilweise, rot auf dem Prozessor. Zeigen Sie
mit der Maus darauf, um die Details zu sehen.

[Wie das Rendern funktioniert](../RENDERING.md) erzählt die ganze Geschichte
mit Bildern: was Auto beim ersten Mal tut, warum die Karte nicht immer die
schnellere Wahl ist und was die Farben der LED bedeuten.

Nicht alles kann auf die Karte: einige wenige Formeln lassen sich nicht für
sie übersetzen, und sehr tief gezoomte Ansichten werden in doppelter
Genauigkeit berechnet (was nur manche Karten können). Das Programm entscheidet
all das selbst; Sie sehen es nur an der LED.

<a id="speed"></a>

## Geschwindigkeit

Alle Messungen auf einem Computer: ein AMD Ryzen 9 3900X (12 Kerne, 24
Threads) mit einer NVIDIA GeForce RTX 2060, unter Linux. Das ursprüngliche
**Mandelbulb3D 1.99sr37** lief auf derselben Maschine unter Wine (sein
Windows-Programm läuft dort unverändert, mit voller Geschwindigkeit und auf
allen Kernen) und wurde vom Klick auf *3D berechnen* bis zum fertigen Bild
gemessen. Jede Parameterdatei wurde von allen dreien in derselben Bildgröße
gerendert: MB3D, mb3d-rs nur auf dem Prozessor (**CPU**) und mb3d-rs im
**Auto**-Modus (Grafikkarte und Prozessor zusammen). Die Auto-Zeiten sind die
des zweiten Renderings einer Datei, mit dem bereits vorbereiteten Shader der
Karte; das erste Rendering einer neuen Formelkombination kostet zusätzlich die
10–15 s der Shader-Vorbereitung auf dieser Karte.

<a id="mb3ds-80-example-parameter-files"></a>

### Die 80 Beispiel-Parameterdateien von MB3D

Die Beispieldateien, die mit MB3D geliefert werden (und im Ordner
`M3Parameter` dieses Programms), jede in ihrer eigenen Bildgröße.

MB3D konnte eine der Dateien nicht berechnen (*Test8*: es bricht mit dem Fehler „Division by zero“ ab), daher zählen die Vergleiche 79 Dateien.

* **mb3d-rs allein auf dem Prozessor ist schneller als MB3D** bei 71 der 79 Dateien, im Median **1,7-mal**; alle Dateien zusammen brauchten mit MB3D 18 min 21 s und mit mb3d-rs auf dem Prozessor 17 min 10 s.
* **Der Auto-Modus** ist bei 77 von 79 Dateien schneller als MB3D (Median 2,1-mal); alle Dateien zusammen 8 min 17 s. Bei 30 der Dateien wurde der Prozessor allein fertig, meist weil er nur wenige Sekunden braucht, bevor die Karte an die Reihe kommt — dort ist Auto einfach der Prozessor plus einen Moment Zusatzaufwand. Die Karte stieg bei 49 Dateien ein, den langsameren, wo sie das Bild im Median 1,6-mal schneller machte als der Prozessor (bis zu 13-mal: *Insect Menger*, 4,7 s statt 63,0 s).
* Bei 4 Dateien war Auto merklich langsamer als der Prozessor allein (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): dort ist die Karte für die Szene langsamer als die 24 Threads des Prozessors.
* „Zeilen auf der Karte“ ist der Anteil des Bildes, den die Grafikkarte im Auto-Modus abgetastet hat (–, wenn der Prozessor allein fertig wurde).

<details>
<summary>Alle 80 Dateien (zum Öffnen klicken)</summary>

| Parameterdatei | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | Zeilen auf der Karte | Auto vs. MB3D |
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
| Test8 | Fehler in MB3D | 0.1 s | 0.3 s |  | – |
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

Die Bilder sind dieselben: mb3d-rs auf dem Prozessor erzeugt das Bild von
MB3D bis auf Rundungsunterschiede, und die Auto-Bilder weichen von denen des
Prozessors im Mittel um einen Bruchteil eines Prozents des Farbbereichs ab
(feine Details können in anderen Pixeln funkeln; die
[technischen Hinweise](../TECHNICAL.md) enthalten die Messungen).

<a id="the-command-line"></a>

## Die Kommandozeile

`mb3d` rendert ohne Fenster. Öffnen Sie ein Terminal im Ordner des Programms
(unter Windows eine Eingabeaufforderung im entpackten Ordner, und tippen Sie
`mb3d` statt `./mb3d`). Nach der Ein-Zeilen-Installation unter Linux oder
macOS funktioniert `mb3d` in jedem Ordner, `man mb3d` zeigt sein Handbuch, und
die Tab-Taste vervollständigt seine Optionen in bash, zsh und fish:

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

Alles andere hat einen Unterbefehl mit eigenem `--help`:

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

Die Formeln und Maps werden neben dem Programm gefunden; die einer anderen
MB3D-Installation lassen sich mit `--formulas` und `--maps` angeben.
Animationen sind die von MB3D: Schlüsselbilder mit der Interpolation von MB3D,
`.m3a`-Projekte werden geöffnet und gespeichert, Einzelbilder wie bei MB3D
geschrieben, sodass eine unterbrochene Animation fortgesetzt werden kann und
sich mehrere Computer eine teilen können.

<a id="good-to-know-1"></a>

## Gut zu wissen

* **Dieselben Bilder, nicht bitgenau.** MB3D rechnete mit der 80-Bit-Genauigkeit
  der alten x87-Arithmetik; dieses Programm verwendet die heutigen 64 Bit. Die
  Bilder stimmen in jeder Struktur überein, aber einzelne Pixel können in der
  letzten Stelle ihrer Farbe abweichen. Monte-Carlo-Renderings und MutaGen
  verwenden eigene Zufallszahlen, daher sind ihre Ergebnisse nicht die von
  MB3D.
* **Die Bilder der Grafikkarte** weichen leicht von denen des Prozessors ab
  (einfache Genauigkeit); in sehr feinen Details kann das Funkeln auf andere
  Pixel fallen. Tiefe Zooms wechseln von selbst zu doppelter Genauigkeit.
* **Die Geschwindigkeit auf dem Prozessor** liegt nahe an der von MB3D: der
  Maschinencode der Formeln aus den `.m3f`-Dateien von MB3D wurde so übersetzt,
  dass er auf jedem Prozessor läuft (auch Apple Silicon), mit geringen
  Einbußen; das Ray Marching selbst ist schneller. Siehe die Tabellen oben.
  Unter Linux und Windows werden die Formeln des Hybrids einer Szene beim Start
  eines Bildes außerdem zu einer einzigen Funktion zusammen kompiliert (in
  wenigen Millisekunden, mit Cranelift), sodass sie ihre Werte in den
  Registern des Prozessors weiterreichen. Zusammen mit einer schnelleren
  Schleife über die Formeln (auf jedem System) brauchen CPU-Renderings der 80
  Beispieldateien 20 % weniger Zeit als in v0.2.74 (bei einigen bis zu 60 %),
  mit bitgleichen Bildern. Eine Szene behält den kompilierten Hybrid nur, wenn
  er sich als schneller erweist; `MB3D_NO_JIT=1` schaltet ihn ab. Unter macOS
  ist er ebenfalls eingeschaltet, wird dort aber noch getestet:
  `./jit_check.sh` im entpackten Ordner prüft ihn (es vergleicht die
  kompilierten Formeln mit den übersetzten und rendert die Beispieldateien mit
  und ohne ihn; die Bilder müssen gleich sein).
* **Große Bilder** lassen sich in einem Stück rendern (ein Bild von 15000 x
  20000 braucht etwa 15 GB Speicher; mit Tiefenschärfe dauert es Stunden) oder
  in Kacheln, auch auf mehreren Computern (`--tiles` und das Fenster Große
  Renderings), was weniger Speicher braucht. 3D berechnen startet kein Bild,
  das nicht in den freien Speicher passt (unter Linux, wo sich der freie
  Speicher feststellen lässt), und sagt das, und ein Computer des Schwarms
  überlässt eine Kachel, die für seinen Speicher zu groß ist, den anderen.
* **Unfertige Renderings** werden im Cache-Ordner aufbewahrt
  (`~/.cache/mb3d-rs/resume`, unter Windows `%LOCALAPPDATA%\mb3d-rs\resume`,
  oder neben dem Programm): 18 Byte pro Pixel bei einem Bild (ein Bild von
  15360 x 15360 braucht 4,2 GB), 64 bei einem Monte-Carlo-Rendering. Was
  berechnet wurde, wird sofort geschrieben und alle 5 Minuten auf die
  Festplatte übertragen. Ein Rendering, das in seinen ersten 10 Sekunden
  angehalten wird, behält nichts; unfertige Renderings, die älter als 30 Tage
  sind, werden entfernt, und die ältesten, wenn alle zusammen mehr als 50 GB
  belegen. Ein Rendering lässt sich nur mit denselben Parametern und derselben
  Programmversion fortsetzen (und derselben Wahl von Karte oder Prozessor;
  Belichtung und Sättigung eines Monte-Carlo-Renderings dürfen sich ändern).
  Ein Monte-Carlo-Rendering bleibt unfertig, bis es seine Strahlen erreicht
  (ein Stapel, `--rays`, `--passes`) oder in der Fortsetzen-Liste gelöscht
  wird.
* **Neuere oder eigene Formeln, Maps, Designs und Sprachen** kommen nach
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` und
  `.../Languages` (bei jeder Installationsart; unter Windows
  `%APPDATA%\mb3d-rs\...`). Sie werden zusätzlich zu den mitgelieferten
  verwendet und ersetzen eine mitgelieferte Datei gleichen Namens, sodass eine
  aktualisierte Formel aus den Foren wirkt, ohne den Ordner des Programms
  anzurühren.

<a id="updates"></a>

## Aktualisierungen

Jede Version mit ihren Änderungen ist in [UPDATES.md](../UPDATES.md) aufgeführt (auf Englisch).

<a id="for-developers"></a>

## Für Entwickler

Die [technischen Hinweise](../TECHNICAL.md) beschreiben, wie die Grafikkarte
genutzt wird, was aus den Delphi-Quellen von MB3D portiert wurde und wie, die
drei Arten, wie der Formel-Maschinencode von MB3D ausgeführt wird, das
Format der Szenendateien und seine Schlüssel, die Vergleiche mit MB3D und das
Bauen aus dem Quellcode:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## Lizenz

MB3D steht unter der LGPL 2.1. Diese Portierung ist ein abgeleitetes Werk und
verwendet dieselbe Lizenz.

<a id="credits"></a>

## Danksagungen

Die Menschen und Projekte, auf denen mb3d-rs aufbaut, werden im [englischen README](../../README.md#credits) genannt.
