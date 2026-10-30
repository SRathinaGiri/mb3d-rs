<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D voor Linux, Windows en macOS">

# mb3d-rs — Mandelbulb3D voor Linux, Windows en macOS
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Italiano](README.it.md) · **Nederlands** · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*Vertaald uit de Engelse README (versie 0.2.84). Waar de twee verschillen, heeft de [Engelse README](../../README.md) gelijk.*

**Downloads: [de nieuwste release](https://github.com/thr32423/mb3d-rs/releases/latest)** · [Downloaden](#download) · [Wat is er nieuw](../UPDATES.md)

mb3d-rs is [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D), het
3D-fractalprogramma, herschreven zodat het op de computers van nu draait: op
**Linux, Windows en macOS**, en met de afbeelding berekend op de
**grafische kaart**, waar MB3D alleen de processor kon gebruiken. Het opent de
parameterbestanden van MB3D (`.m3p`, `.m3i`, tekstparameters van forums), gebruikt
de formules en maps van MB3D en maakt dezelfde afbeeldingen.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="Het hoofdvenster"></a>

Het bestaat uit twee programma's:

* **Mandelbulb3D** — het desktopprogramma, met de eigen vensters van MB3D: het
  hoofdvenster, Formules, Belichting, Nabewerking, Navigator, Animatiemaker,
  Batchverwerking, Grote renders, Monte Carlo, Voxelexport, Bulb Tracer 2, MutaGen
  en de rest. Wie MB3D kent, kent dit programma; de handleidingen van MB3D
  gelden ook hier.

  <details><summary>Meer lezen</summary>

  De vensters zijn die van MB3D zelf — gemaakt uit de oorspronkelijke
  vensterdefinities van MB3D, dus elke knop, schuifregelaar en tip staat waar hij in MB3D staat:
  
  * **Hoofdvenster:** parameters openen en opslaan (`.m3p`, `.m3i`, tekst, het
    klembord), afbeeldingen opslaan (PNG, JPEG, BMP, 16-bits Z-buffer), de muismodi,
    de rotatieknoppen, de afbeeldingsgrootte, de kwaliteitsvoorinstellingen, ongedaan maken,
    *3D berekenen* met de groeiende afbeelding en de statistieken, en alle
    tabbladen (Berekening, Intern, Info, Snijden, Julia, Camera,
    Kleuring, Stereo).
  * **Formules:** de zes formuleplaatsen, de formulelijsten en opties van MB3D,
    hybride typen, favorieten en de JIT-formule-editor.
  * **Belichting:** lichten 1–6, voorinstellingen, het palet, de tabbladen Object, Ambient,
    d.Mist en Achtergr.afb.; wijzigingen tekenen de afbeelding meteen opnieuw, zonder
    nieuwe berekening.
  * **Nabewerking:** normalen op de Z-buffer, harde schaduwen en ambient occlusion,
    reflecties en transparantie, scherptediepte.
  * **Navigator:** rondlopen en rondkijken met toetsen en muis, daarna *Beeld naar
    hoofd*.
  * **Animatiemaker, Batchverwerking, Grote renders, Monte Carlo, Voxelexport,
    Bulb Tracer 2** (meshes) met de meshvoorvertoning, de **HeightMap-generator,
    MutaGen, ZBuf16Bit, Map-reeksen, Ini-mappen** — alles zoals in
    MB3D.
  * **Zwerm** (nieuw, op het tabblad Extra): uw computers renderen samen,
    zie [Renderen op meerdere computers](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — een renderer voor de opdrachtregel, voor scripts en batchtaken: afbeeldingen,
  animaties, batchlijsten, Monte Carlo-renders, voxel- en mesh-exports,
  allemaal zonder venster.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## Is het veilig om uit te voeren? Controleer het zelf

U hoeft ons niet op ons woord te geloven. [scanrepo.dev](https://www.scanrepo.dev)
is een onafhankelijke dienst die een GitHub-repository onderzoekt op tekenen
van schadelijke software: code die andere programma's start, verbinding maakt met
internet, verbergt wat ze doet, of de veiligheidscontroles van de
programmeertaal omzeilt. Iedereen kan de scan op elk moment op deze repository uitvoeren:

**[▶ mb3d-rs scannen op scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(klik op *Rescan* voor een nieuw resultaat van de huidige code)

Versie 0.2.66 kreeg de beoordeling **veilig: 0 van 100, geen bevindingen** (eerder:
[het resultaat van 0.2.44](../img/scan_0.2.44.png)). Een eerdere versie werd gemarkeerd,
en daarop hebben we de code aangepast ([issue #4](https://github.com/thr32423/mb3d-rs/issues/4));
ze is nu zo geschreven dat dit zo blijft. De programma's bevatten geen code die
verbinding maakt met internet; het enige andere programma dat ze uw systeem laten
openen is uw bestandsbeheerder, wanneer u op een knop klikt om een map te openen.

Een scan zoekt naar bekende waarschuwingstekens; het is geen garantie. Hiermee
kunt u zelf verder controleren:

* **Alle broncode staat hier.** De downloads worden daaruit gebouwd door
  de eigen buildservers van GitHub, volgens de stappen die u kunt lezen in
  [de release-workflow](.github/workflows/release.yml), niet op iemands
  persoonlijke computer.
* **Elke release vermeldt controlesommen** (`SHA256SUMS`), zodat u kunt nagaan
  dat het bestand dat u downloadde precies het gepubliceerde is;
  de installatie in één regel hieronder doet dat voor u. Releases vanaf 0.2.45
  hebben ook de build-attestaties van GitHub: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` laat zien uit welke commit een download is
  gebouwd.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Wat is er nieuw ten opzichte van Mandelbulb3D

* **De grafische kaart berekent de afbeelding** — de afbeelding zelf, de
  schaduwen, de ambient occlusion, de reflecties en de Monte Carlo-renders.
  Op een gewone computer is dat meerdere keren sneller dan met
  alleen de processor; zie [Snelheid](#speed) hieronder.
* **Modus Auto**: de grafische kaart en de processor werken samen aan elke
  afbeelding, en niets wacht ooit — als de kaart nog niet klaar is, begint de
  processor alvast en schuift de kaart later aan.
* **Een LED in de titelbalk** laat zien waar de laatste afbeelding is berekend.
* **Interfacegrootte** van 75 tot 200 % voor schermen met hoge resolutie.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="Visuele thema's op 150 %"></a>

  <details><summary>Meer lezen</summary>

  **Voork. ▸ Thema's ▸ Interfacegrootte** maakt alle vensters en hun
  tekst groter of kleiner (75 tot 200 %), voor schermen met hoge resolutie; de
  indeling blijft precies zoals in MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Visuele thema's op 100 %](../img/themes_100.png) | ![Visuele thema's op 150 %](../img/themes_150.png) |

  </details>
* **Acht uiterlijken** voor de vensters, allemaal themabestanden die u kunt kopiëren en
  aanpassen: twee bootsen de uiterlijken van MB3D na, zes zijn nieuw.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Het thema Midnight"></a>

  <details><summary>Meer lezen</summary>

  **Voork. ▸ Thema's** biedt acht uiterlijken, elk een bestand in de map `Themes`:
  *Glossy* (het donkere standaardthema) en *Windows* bootsen de twee eigen
  uiterlijken van MB3D na, de andere zes zijn nieuw. Klik op een afbeelding voor de volledige grootte.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: standaard van MB3D | **Windows**: "Thema's uitschakelen" van MB3D |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: lichter neutraal grijs, staalblauwe accenten | **Daylight**: warm lichtgrijs, blauwgroene accenten |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: diep marineblauw, blauwe accenten | **Ember**: warm antraciet, oranje accenten |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="Hoog contrast"></a> |
  | **Forest**: donker groengrijs, groene accenten | **High contrast**: zwart, witte tekst, gele markeringen |
  
  Een thema is een klein tekstbestand in de map `Themes` naast het programma;
  kopieer er een, verander de kleuren, en het verschijnt bij de volgende start
  in de lijst.

  </details>
* **Een crash laat op elk systeem een notitie achter**: een panic van het programma (of
  de fout waarmee het eindigt) wordt toegevoegd aan `mb3d_crash.txt`, naast het
  programma of in de gegevensmap ervan (voorheen alleen op Windows). Stuur dat bestand
  mee met een foutmelding.
* **Alle grafische kaarten van de computer rekenen mee**, zonder dat u iets hoeft
  in te stellen: elke gevonden kaart neemt banden van rijen van de afbeelding uit dezelfde
  voorraad als de eerste kaart en de processor, zodat een snellere kaart gewoon
  meer neemt; de schaduwen, de occlusion, de Monte Carlo-stralen en de
  reflecties worden op dezelfde manier verdeeld, en een animatie rendert één frame
  per kaart tegelijk. Een trage kaart (geïntegreerde graphics naast een losse
  kaart) doet wat hij kan en houdt een afbeelding nooit op: zodra de vrije rijen
  op zijn, nemen de vrije kaarten en de processor de rijen over die hij nog
  niet heeft geleverd. Een kaart die faalt, wordt overgeslagen; de andere gaan door.
  `mb3d --list-cards` laat zien wat er is gevonden. (Getest met één kaart die twee keer
  werd geopend en met een software-Vulkan-driver als tweede kaart; nog niet op een
  computer met twee echte kaarten.)
* **Een onvoltooide afbeelding is een `.m3i`-bestand.** Wat een gestopte, gecrashte of
  afgebroken render heeft berekend, wordt in de cache bewaard (het venster Onvoltooide
  renders, `mb3d resume`); voor een afbeelding is dat bestand nu op elk moment een
  `.m3i`-afbeeldingsbestand, `image.m3i` in de map van de render,
  met de nog niet berekende rijen als achtergrond. Kopieer het eruit wanneer
  u wilt, of druk op *Save m3i* in het venster Onvoltooide renders (`mb3d
  resume --m3i KEY file.m3i` op de opdrachtregel; beide schrijven ook het
  `data.bin` van een eerdere versie als `.m3i`), en open het hier of in
  Mandelbulb3D, zonder conversie. (Monte Carlo-renders houden hun
  eigen gegevens bij; de `.m3c` van een pass wordt geschreven met `--m3c`.)
* **Sneltoetsen voor de vensters**: een toetscombinatie haalt het hoofdvenster,
  de Navigator, de Animatiemaker, Formules, Belichting, Nabewerking of het
  Monte Carlo-venster vanuit elk venster naar voren (Ctrl+1 tot Ctrl+7), en
  Ctrl+W sluit het venster waarin u zit (op macOS Cmd). Stel uw eigen sneltoetsen in onder *Voork. > Sneltoetsen*: klik op een veld en
  druk de toetsen in; Backspace haalt een sneltoets weg.
* **De interface in uw taal**: elke tekst van de vensters komt
  uit een taalbestand; er worden twaalf talen meegeleverd, en het programma schrijft het
  bestand van een nieuwe taal voor u om te vertalen.

  <details><summary>Meer lezen</summary>

  De taalbadge aan de rechterkant van de titelbalk van het hoofdvenster
  toont de interfacetaal (`EN`); een klik opent het taalvenster.
  Een taal is een tekstbestand in de map `Languages` naast het programma.
  Meegeleverd: English, Deutsch, Français, Italiano, Nederlands, Polski,
  Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文 en 繁體中文 (elk met een eigen vlag; de
  Oost-Aziatische talen brengen een deel van Noto Sans CJK mee voor hun tekens,
  Hindi brengt Noto Sans Devanagari mee, waarvan de verbonden letters en klinkertekens
  worden gezet met de tekstvormer rustybuzz).
  De vertalingen zijn machinaal gemaakt en op structuur gecontroleerd, niet door
  moedertaalsprekers: verbeteringen zijn welkom, elk is één regel van het bestand. Om te vertalen: typ de naam van uw taal en
  druk op *Taalbestand aanmaken*; het programma schrijft `Languages/<name>.lng`
  met elke tekst in het Engels, één per regel (`Button2.Caption = Calculate
  3D`). Vertaal de teksten rechts van de `=` met een teksteditor en druk op
  *Opnieuw laden*: de vensters volgen meteen, zonder herstart. Een weggelaten regel
  houdt de Engelse tekst. *Opslaan + afsluiten* bewaart de taal voor de volgende
  start.

  </details>
* **Linux, Windows en macOS** (Intel en Apple Silicon).
* **Een opdrachtregelprogramma** voor alles wat geen venster nodig heeft.
* **Zeer grote afbeeldingen in één stuk**: 15000 x 20000 pixels en meer
  zonder dat het geheugen opraakt, en de scherptediepte gebruikt alle
  processorkernen (11 keer sneller op 24 threads dan vóór 0.2.45, met
  dezelfde afbeelding).
* **16-bits afbeeldingen:** PNG-bestanden met 65.536 in plaats van 256 stappen per
  kleur, om te bewerken zonder banding.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="16-bits afbeeldingen"></a>

  <details><summary>Meer lezen</summary>

  MB3D slaat 8-bits afbeeldingen op: 256 stappen per kleur. Dat is ruim genoeg om naar
  te kijken, maar wanneer een afbeelding achteraf wordt bewerkt — helderheid, contrast,
  curven — vallen vlakke gebieden zoals de achtergrond of mist uiteen in zichtbare
  banden. **Afb. opslaan ▸ PNG** biedt een tweede bestandstype, **PNG 16 bit
  colour**: de kleuren zoals het programma ze berekende, voordat ze
  op 256 stappen worden afgerond, met alle schaduwen, reflecties en scherptediepte.
  Het bestand ziet er hetzelfde uit, en blijft vloeiend wanneer het wordt bewerkt.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="Een achtergrondverloop na een sterke contrastverhoging: trappen in 8 bit, vloeiend in 16 bit"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="Het opslagvenster met 'PNG 16 bit colour'"></a>

  </details>
* **Een ruisfilter** voor Monte Carlo en alle andere afbeeldingen, de DCT-ruisonderdrukking
  die bekend is uit GIMP, en voor Monte Carlo-renders ook een neuraal
  filter, Intel Open Image Denoise.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="Ruisfilter"></a>

  <details><summary>Meer lezen</summary>

  Een **ruisfilter** haalt korrel uit de afbeeldingen: de **DCT-ruisonderdrukking**
  van GIMP's *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT*, het filter van
  GIMP dat het best bij deze afbeeldingen paste (zie de vermeldingen hieronder).
  
  * **Monte Carlo-venster ▸ -ruis** (een knop die MB3D verborgen hield): filtert de
    getoonde afbeelding, en de opgeslagen afbeelding, met een sterkte die het aantal stralen
    per pixel volgt. Het renderen zelf gaat ongefilterd door, en de volgende update
    toont de afbeelding weer ongefilterd.
  * **Nabewerking ▸ Ruisfilter**: *Nu filteren* filtert de afbeelding van
    het hoofdvenster; *Nieuwe afbeeldingen automatisch filteren* filtert elke nieuwe
    afbeelding. *Sterkte* is de sterkte van de ruis die wordt verwijderd (4 is licht;
    hoger verwijdert meer korrel en meer fijn detail).
  
  Monte Carlo-afbeeldingen hebben de meeste korrel. Vergeleken met afbeeldingen gerenderd met
  512 stralen per pixel lagen de gefilterde afbeeldingen in alle 13 tests dichter bij dan de ongefilterde
  (vier scènes, 4 tot 32 stralen): tot 23 % bij 4 stralen,
  5 tot 10 % bij 8 stralen, 1 tot 3 % bij 32 stralen, waar weinig korrel over is.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="Een Monte Carlo-afbeelding met 4 stralen per pixel, dezelfde met het ruisfilter, en met 512 stralen"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="Nabewerking: het onderdeel Ruisfilter"></a>

  Het **neurale ruisfilter** is
  [Intel Open Image Denoise](https://www.openimagedenoise.org), het
  filter van Cycles in Blender en vele andere path tracers, getraind op
  gerenderde afbeeldingen. Het filtert de gemiddelden van de stralen van de render
  (vóór belichting en verzadiging), zodat de afbeelding haar helderheid houdt.
  De bibliotheek van Intel wordt met het programma meegeleverd (de map `oidn`; in de .deb
  en .rpm `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` kan een andere aanwijzen.

  * **Monte Carlo-venster ▸ -ruis AI**: filtert de getoonde afbeelding, en de
    opgeslagen afbeelding, zoals *-ruis* dat doet; het renderen gaat ongefilterd door.
  * **Opdrachtregel**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * Het kost 0,2 tot 0,6 seconde voor een afbeelding van 640 x 480 op de processor.

  Het verschil met de afbeelding met 512 stralen (gemiddelde over alle pixels, lager is
  beter):

  | Scène | 4 stralen | DCT-filter | neuraal filter | 8 stralen | DCT-filter | neuraal filter |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  Het neurale filter komt in drie van de vier scènes het dichtst bij; bij de fijne korrel
  van de Julia-verzameling komt het DCT-filter iets dichter bij. Het verschil is het grootst bij
  glas en spiegels, waar weinig stralen de meeste korrel achterlaten:

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="Een Monte Carlo-afbeelding met 4 stralen per pixel, met het DCT-filter, met het neurale filter, en met 512 stralen"></a>

  </details>
* **Transparante achtergronden** voor afbeeldingen en animaties.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="Transparante achtergrond"></a>

  <details><summary>Meer lezen</summary>

  Afbeeldingen en animatieframes kunnen worden opgeslagen met een **transparante
  achtergrond**, om de fractal op een andere afbeelding of in een videobewerker
  te zetten: **Afb. opslaan ▸ PNG ▸ PNG transparante achtergrond**, en in de
  Animatiemaker **Transparante achtergrond** (PNG-frames). Met anti-aliasing (een
  weergaveschaal van 1:2 of meer, of `--aa`) zijn de randen zacht, en ze
  hebben geen zoom van de oude achtergrondkleur. Diepteafbeeldingen voor
  compositing waren er al (*Ook Z-buffer opslaan*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="Een afbeelding met haar achtergrond, en met een transparante achtergrond getoond op een dambordpatroon"></a>

  </details>
* **Video's van animaties:** een AVI-bestand rechtstreeks uit het programma, of
  de ffmpeg-opdracht voor een kleine MP4.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="Video van een animatie"></a>

  <details><summary>Meer lezen</summary>

  In de Animatiemaker voegt **Ook een video maken (AVI)** de frames samen tot
  een video zodra ze gerenderd zijn, met de ingestelde **Frames per seconde**:
  `<project name>.avi` in de uitvoermap, naast de frames, die
  bewaard blijven. Hij speelt af in VLC en mpv en opent in videobewerkers (Kdenlive,
  Shotcut, DaVinci Resolve). Elk frame wordt als volledige afbeelding opgeslagen, dus de
  bestanden zijn groot: goed om te bewerken en te bewaren, minder om te delen.
  
  Voor een kleine video om te delen zet **FFmpeg-opdracht** de opdracht voor een MP4
  (of, met een transparante achtergrond, een WebM) op het klembord en in de
  berichten. Installeer [ffmpeg](https://ffmpeg.org), plak de opdracht in een
  terminal wanneer de frames klaar zijn, en de video wordt gemaakt; dit programma
  start zelf geen andere programma's.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="De Animatiemaker met Ook een video maken (AVI), Frames per seconde en FFmpeg-opdracht"></a>

  </details>
* **Een voorvertoning van parameterbestanden** in het venster Openen.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="Voorvertoning bij het openen van parameters"></a>

  <details><summary>Meer lezen</summary>

  **Openen ▸ m3p openen** en **m3i openen** tonen een kleine afbeelding van het
  aangeklikte parameterbestand, met de grootte en de formules, voordat het
  wordt geopend; met de pijltoetsen loopt u door de lijst met de voorvertoning. De
  afbeelding wordt op de achtergrond op de processor berekend, zodat het
  venster blijft reageren, en de voorvertoning van elk bestand blijft bewaard zolang het
  programma draait.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="Het venster Openen met de voorvertoning van een parameterbestand"></a>

  </details>
* **Navigator: exacte waarden en Beeld terugzetten.** Klik op een waarde en typ haar in;
  ga terug naar het beeld waarmee de Navigator begon.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="Navigator: exacte waarden en Beeld terugzetten"></a>

  <details><summary>Meer lezen</summary>

  * **Klik op een waarde** in de panelen van de Navigator — Julia-waarden, formulewaarden,
    4D-rotatie, de waarden onder Diversen — en typ het exacte getal, zoals in
    MB3D (hoeken in graden). De schuifregelaar gaat daarna verder vanaf die waarde.
  * **Klik op de zoom** en typ hem in, wanneer de zoomstappen te grof zijn (een
    wens uit de issuelijst van MB3D); een ingetypte zoom blijft staan (*Vaste zoom en stappen*).
  * **Beeld terugzetten** gaat terug naar de positie, richting en zoom die de
    Navigator van het hoofdvenster overnam; de aangepaste waarden blijven.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="De Navigator met de formulewaarden en het invoervak van een aangeklikte waarde"></a>

  </details>
* **Een nieuwe formule, SphereTree:** een boom van elkaar rakende bollen die zich steeds
  kleiner vertakken, van Tom Lowe.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="Een nieuwe formule: SphereTree"></a>

  <details><summary>Meer lezen</summary>

  **SphereTree** is een formule die Mandelbulb3D niet heeft: een boom van bollen
  die elkaar raken en zich vertakken in steeds kleinere bollen, zoals
  schuim of koraal. Tom Lowe, de bedenker van de Mandelbox, publiceerde haar in 2020
  (zie de vermeldingen hieronder). Ze staat in de formulelijst bij de dIFS-formules
  en heeft vier instellingen:
  
  * **PackRatio** (0 tot 1,22): de grootte van de bollen; bij 1 raken ze elkaar
    net, boven 1 breken de buitenste af.
  * **Levels**: hoe diep de boom gaat (6 is genoeg voor de meeste afbeeldingen).
  * **Rotated**: 1 draait elk niveau een twaalfde slag, en de boom
    wordt een ronde bal van bollenclusters.
  * **Colour scale**: elke tak van de boom heeft een eigen kleur. Kies
    *2e kleurkeuze* (de orbit trap) in de kleurinstellingen van de Belichting om
    ze te zien; deze instelling spreidt ze over het kleurverloop.
  
  Ze werkt los en na dIFS-transformaties (de derde afbeelding herhaalt haar
  met *tilingbrickIFS*), en ze wordt zowel op de grafische kaart als op de
  processor berekend. Zet de iteraties op 1: de formule berekent de
  hele boom in één keer. Parameters met SphereTree openen alleen in dit programma,
  niet in Mandelbulb3D.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="Een SphereTree-bal (Rotated) in de Monte Carlo-renderer, in de schemering"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="Een close-up van het bollenschuim van SphereTree"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="SphereTree herhaald met tilingbrickIFS: een zwerm bollenbomen"></a> |
  |:---:|:---:|

  </details>
* **Een nieuwe formule, DomeTree:** een koepel waaruit een ring van kleinere
  koepels groeit, elk met een eigen ring, van Tom Lowe.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="Een nieuwe formule: DomeTree"></a>

  <details><summary>Meer lezen</summary>

  **DomeTree** is de tweede boom van Tom Lowe na SphereTree (zie de
  vermeldingen): een bal waarvan één kant een ring van koepels laat groeien, elke koepel
  een eigen kleinere ring, niveau na niveau, gemaakt van cirkelinversies. Ze
  staat in de formulelijst bij de dIFS-formules en heeft vijf instellingen:

  * **Levels**: hoe diep de boom gaat (16 is genoeg).
  * **Children**: het aantal koepels in elke ring, 4 tot 10.
  * **Bend angle** (0,3 tot 1,3 werken het best): hoe ver de koepels uitbollen; lage
    waarden geven platte schijven, hoge waarden ronde koepels. Boven ongeveer 1,3 wordt het
    oppervlak korrelig.
  * **Colour scale**: elke koepel heeft een eigen kleur. Kies *2e kleurkeuze*
    (de orbit trap) in de kleurinstellingen van de Belichting en een breed
    kleurbereik (bijvoorbeeld 0 tot 100) om ze te zien.
  * **Start curved**: 1 (de standaard) begint met de bal, 0 met een vlakke
    vloer waaruit de eerste ring groeit.

  Gebruik een kleine *Raystep* (0,2; 0,05 met 9 of 10 children): de afstandsschatting
  van de vorm is grof, zoals in het eigen Fragmentarium-bestand van TGlad. Net als
  SphereTree draait ze op de grafische kaart en de processor, heeft ze maar
  1 iteratie nodig, en openen parameters met deze formule alleen in dit programma.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="DomeTree met 8 children in de Monte Carlo-renderer, gekleurd per koepel"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 5 children, bend angle 1,2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 6 children, bend angle 0,5: platte koepels"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 10 children: kant tussen de koepels en een grote bal"></a> |
  |:---:|:---:|:---:|

  </details>
* **Een nieuwe formule, UlamSpiral:** de spiraal van priemgetallen van Ulam in 3D, als
  een stad, een skyline, bollen of een blok.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="Een nieuwe formule: UlamSpiral"></a>

  <details><summary>Meer lezen</summary>

  In 1963 zat de wiskundige Stanisław Ulam op een bijeenkomst "een lange en erg
  saaie lezing" uit en zat hij te krabbelen op een vel ruitjespapier. Zijn
  eerste idee was schaakproblemen op te zetten; in plaats daarvan nummerde hij de vakjes
  1, 2, 3, … in een spiraal, 1 in het midden, en omcirkelde hij de priemgetallen.
  De priemgetallen kwamen op diagonale lijnen te liggen, alsof ze een patroon volgden, en
  dat doen ze nog steeds wanneer een computer er miljoenen tekent (Ulam probeerde er 100.000
  op de MANIAC II in Los Alamos; Martin Gardner zette de afbeelding in 1964 op de
  omslag van Scientific American). Niemand heeft het tot op
  vandaag volledig verklaard.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="De spiraal van Ulam: de priemgetallen onder de getallen 1 tot 78.961 op een vierkante spiraal, met het begin van de spiraal in de hoek getekend"></a>
  
  **UlamSpiral** bouwt die spiraal in 3D, één vorm per getal, in vier
  vormen (*Mode*):
  
  * **0, priemtorens**: een plaat voor elk getal en een toren op elk
    priemgetal. De diagonale lijnen van priemgetallen worden rijen torens.
  * **1, delersskyline**: een zuil voor elk getal, hoger naarmate het meer
    delers heeft; de priemgetallen zijn de lage straten ertussen.
  * **2, delersbollen**: een bol voor elk getal, groter naarmate het meer
    delers heeft (de priemgetallen zijn de kleine).
  * **3, een blok van priemgetallen**: de getallen gewikkeld om kubusschillen in plaats
    van een vierkant, een kubus op elk priemgetal.
  
  *Rings* bepaalt de grootte van de spiraal (60 ringen zijn de getallen tot
  14.641), *Height* de hoogte van de torens of zuilen, *Gap* de ruimte
  ertussen. Met *2e kleurkeuze* (de orbit trap) in de kleurinstellingen
  volgt de kleur het aantal delers, zodat de priemgetallen een eigen kleur
  krijgen. Net als SphereTree draait ze op de grafische kaart en de
  processor, werkt ze na dIFS-transformaties, heeft ze maar 1 iteratie nodig, en
  openen parameters met deze formule alleen in dit programma.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, priemtorens: de diagonale rijen priemgetallen als stad, in de Monte Carlo-renderer met mist, harde schaduwen en scherptediepte"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, delersskyline"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, delersbollen"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, een blok van priemgetallen op kubusschillen"></a> |
  |:---:|:---:|:---:|

  </details>
* **Een gestopte render gaat verder waar hij was.** Wat een render heeft berekend,
  blijft bewaard wanneer hij wordt gestopt, en ook wanneer het programma wordt gesloten of
  crasht, of de computer uitgaat (het wordt elke 5 minuten naar de schijf
  geschreven): 3D berekenen, Monte Carlo (ook midden in een pass), het
  frame van een animatie of de afbeelding van een lopende batch, een voxelexport,
  een Bulb Tracer-mesh, tegels van een Grote render, en een zwermrender (Stop
  op de computer die de taak maakte bewaart elk voltooid deel, en elke
  computer bewaart het deel waaraan hij bezig was). 3D berekenen en Monte Carlo met dezelfde
  parameters vragen of ze verder moeten gaan, de andere gaan uit
  zichzelf verder; de knop **Hervatten** (tabblad Openen) toont de onvoltooide
  renders, om ze voort te zetten of te verwijderen. Op de opdrachtregel gaat dezelfde
  opdracht uit zichzelf verder (ook een afbeelding in `--tiles`, `montecarlo`,
  `animate`, `batch`, `voxel` en `mesh`); `--fresh` begint opnieuw, `mb3d
  resume` toont ze. Een voortgezette render is gelijk aan een render die in
  één keer is berekend.
* **Renderen verdeeld over uw computers**, een "zwerm": computers die
  één gedeelde map zien, renderen samen een afbeelding, een Grote render, een Monte Carlo-afbeelding
  of een animatie. Zie
  [Renderen op meerdere computers](#rendering-on-several-computers-the-swarm).
* **Een nieuwe formule, PseudoKleinian:** de "Kleiniaanse architectuur" van
  zalen, bogen en bollen als één formule met een eigen exacte afstand.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="Een nieuwe formule: PseudoKleinian"></a>

  <details><summary>Meer lezen</summary>

  De zalen, bogen, pilaren en geneste bollen die bekendstaan als "Kleiniaanse
  architectuur" behoren tot de meest geliefde 3D-fractalafbeeldingen. In MB3D kunnen ze
  alleen worden gemaakt als hybride van meerdere formules met zorgvuldig op elkaar afgestemde
  instellingen, het recept dat Theli-at in 2011 vond; knighty schreef daarna hetzelfde
  als één formule met een eigen afstandsschatting, en dat is
  **PseudoKleinian**. Elk niveau vouwt het punt in een doos, inverteert het in
  een bol en telt er een verschuiving bij op, en de afstand wordt na de vouwen exact gemeten tot een
  vloervlak, zodat de wanden schoon uitkomen en de afbeelding
  snel rendert. De instellingen zijn die van knighty: *Levels* (8), *Size* (de
  inversiebol), *CSize X/Y/Z* (de doos; zijn 0,924, 0,908, 0,924 geven
  de klassieke zalen, en kleine wijzigingen geven nieuwe gebouwen), *Julia X/Y/Z*
  (een verschuiving per niveau), *Shape* (0 het vloervlak, 1 een cilindervlak met
  *Thickness* door *Offset*, 2 een afgeronde doos), *DE offset* (holt het
  oppervlak uit) en *Colour scale* (met *2e kleurkeuze*, de orbit trap: de
  kleinste baanstraal, de klassieke Kleiniaanse bandering).
  
  De structuur vult de hele ruimte, dus u kijkt ernaar van binnenuit: begin
  met het midden van het beeld bij de oorsprong en de camera een of twee eenheden terug,
  en beweeg met de Navigator door de zalen. In de Monte Carlo-renderer
  krijgt een interieur bijna geen hemellicht, dus zet een positioneel licht
  in de zaal (de eerste afbeelding) of snijd de structuur open met een
  snijvlak en kijk van boven naar binnen (de tweede). Ze draait op de grafische
  kaart en de processor, heeft 1 iteratie nodig, werkt na dIFS-transformaties
  en in DE-combinaties zoals de andere dIFS-formules (al veranderen die binnen een
  ruimtevullende structuur weinig), en parameters met deze formule
  openen alleen in dit programma.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="PseudoKleinian vanuit het gezichtspunt van knighty: een zaal van bogen en geneste bollen, verlicht door een lamp binnenin, in de Monte Carlo-renderer met mist, harde schaduwen en scherptediepte"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="PseudoKleinian opengesneden met een snijvlak en van boven gezien: het rooster van bollen en de cellen ertussen"></a>

  </details>
* **Stereoparen in één stap**: beide ooghelften gerenderd en naast elkaar
  getoond, om parallel of scheel te kijken. Bijgedragen door Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Een stereopaar van Hydra van Dainbramage, naast elkaar om parallel te kijken"></a>

  <details><summary>Meer lezen</summary>

  **Stereo ▸ Stereopaar renderen** rendert eerst de afbeelding voor het linkeroog en dan die voor het
  rechteroog met de stereo-instellingen van het tabblad, en toont ze naast elkaar in het
  hoofdvenster. Ze staan klaar om parallel te kijken (linkerafbeelding
  links); **Kruislings voorbeeld** verwisselt ze om scheel te kijken. Het is een
  voorvertoning: belichting en kleuren
  kunnen achteraf niet worden veranderd zonder opnieuw te renderen, en het werkt
  niet met tegels. De knoppen voor de afzonderlijke ooghelften zijn er nog steeds.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Een stereopaar van Hydra van Dainbramage, naast elkaar om parallel te kijken"></a>

  </details>
* **Resterende tijd voor de hele afbeelding**: de balk onder de afbeelding vult zich één keer
  over de hele berekening, en ernaast staat de resterende tijd tot de afbeelding
  klaar is, schaduwen, occlusion en reflecties inbegrepen.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="De voortgangsbalk en de totale resterende tijd"></a>

  <details><summary>Meer lezen</summary>

  De balk van MB3D begint bij elke fase opnieuw. Hier toont hij de hele
  berekening: het hoofdrenderen, dan de harde schaduwen en ambient
  occlusion, de reflecties en de scherptediepte. Hoe lang de fasen
  na het hoofdrenderen duren, wordt geschat uit wat de scène vraagt
  en hoe het hoofdrenderen verliep; de schatting is afgestemd op ongeveer 1400
  renders van 366 scènes. Een afbeelding die opnieuw wordt berekend, verwacht de tijden die ze
  de vorige keer nodig had.
  
  De eerste afbeelding met nieuwe formules op de grafische kaart wacht tot haar
  shaders zijn gecompileerd: de regel zegt dan *shaders compileren* met de resterende
  tijd, die uitgaat van het tempo van de processor en, wanneer de shaders op tijd
  klaar zullen zijn, van de kaart die aanschuift. Details in
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="Het hoofdvenster tijdens de harde schaduwen en ambient occlusion, met de totale resterende tijd"></a>

  </details>
* **History** achter het klokpictogram in de titelbalk: elke afbeelding die u
  berekende met haar miniatuur, elk bestand dat u opende of opsloeg, als lijst,
  als rijen of als tegels van elke grootte, gegroepeerd per dag of in één reeks; doorzoek
  haar, sorteer haar, toon alleen de afbeeldingen die verschillen, en laad elk item
  met een dubbelklik. Een geschiedenis van honderdduizenden items
  opent meteen.
* **De belichting blijft live** na harde schaduwen en reflecties: verplaats een licht
  of verander de kleur ervan en de afbeelding verandert meteen, zoals in Mandelbulb3D.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="Het hoofdvenster en het venster Belichting direct na een wijziging van de belichting"></a>

  <details><summary>Meer lezen</summary>

  Render eerst zonder extra's en probeer daarna lichtposities en kleuren uit: elke wijziging in
  het venster Belichting tekent meteen opnieuw met de schaduwen zoals ze
  berekend waren, zoals in Mandelbulb3D. Het veranderen van de kleur of intensiteit van een licht
  berekent de schaduwen nooit opnieuw. De schaduwen van een licht dat u verplaatste,
  de reflecties en de scherptediepte komen terug met de knoppen van
  het venster Nabewerking, zoals in Mandelbulb3D.
  
  Een berekening die u stopte, kan ook nog worden voortgezet nadat u de
  belichting hebt veranderd. Een manier van werken hiermee: [Werkwijze voor de belichting](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-nl)
  in de wiki. Details in
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="Het hoofdvenster en het venster Belichting direct na een wijziging van de belichting: de afbeelding is al lichter"></a>

  </details>

<a id="download"></a>

## Downloaden

Kant-en-klare programma's van de nieuwste release, geen installatie nodig:

| Systeem | Download |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (installeert zoals andere programma's) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (installeert zoals andere programma's) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple silicon (M1 en nieuwer) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Pak het uit en start **Mandelbulb3D** (`Mandelbulb3D.exe` op Windows) in
de uitgepakte map. De map is ingedeeld als een MB3D-installatie,
met de formules en maps van MB3D en 80 voorbeeldparameterbestanden.

* Als Windows "Windows heeft uw pc beschermd" toont, klik dan op *Meer info* ▸
  *Toch uitvoeren*.
* macOS blokkeert gedownloade programma's die niet zijn ondertekend: voer
  `xattr -dr com.apple.quarantine .` eenmaal uit in de uitgepakte map.

**Alternatief voor Linux en macOS:** in plaats van met de hand te downloaden haalt deze
ene regel de nieuwste release op naar `~/mb3d-rs` (voer hem later opnieuw uit
om bij te werken; het script [download.sh](../../download.sh) staat ook in de map
van het programma):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Het controleert de download voordat het die uitpakt, zet **Mandelbulb3D** in
uw programmamenu (Linux) en maakt de opdrachten `Mandelbulb3D` en `mb3d`
die u in elke map kunt typen. `sh ~/mb3d-rs/download.sh
--uninstall` haalt het menu-item en de opdrachten weer weg; uw bestanden
blijven staan.

De Linux-programma's draaien op distributies vanaf ongeveer 2018 (Ubuntu 18.04,
Debian 10, Rocky/RHEL 8 en nieuwer).

**Geïnstalleerd zoals andere programma's:** open op Debian, Ubuntu en Mint het
`.deb`-bestand hierboven (of `sudo apt install ./mb3d-rs-linux-x64.deb`); op
Fedora, RHEL en Rocky het `.rpm`-bestand (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, op openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D staat dan in het programmamenu en `mb3d` werkt in elke
map; uw instellingen en History worden bewaard in `~/.config/mb3d-rs` en
`~/.local/share/mb3d-rs`.

[Aan de slag](../../GETTING_STARTED.md) loopt de eerste stappen door; oudere
versies en de release-opmerkingen staan op de pagina
[releases](https://github.com/thr32423/mb3d-rs/releases).

<a id="guides-and-help"></a>

## Handleidingen en hulp

Stapsgewijze handleidingen staan in de [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-nl):

* [Installeren](https://github.com/thr32423/mb3d-rs/wiki/Installing-nl): Windows, macOS en Linux, bijwerken, waar uw bestanden staan
* [Werkwijze voor de belichting](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-nl): eerst zonder extra's renderen, daarna lichten, schaduwen en reflecties uitproberen
* [Zwerm instellen](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-nl): renderen op meerdere computers via een gedeelde map
* [Problemen oplossen en FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-nl): wanneer iets niet werkt zoals verwacht

Elke release met haar wijzigingen: [de volledige lijst](../UPDATES.md).

<a id="rendering-on-several-computers-the-swarm"></a>

## Renderen op meerdere computers (de zwerm)

Computers die één gedeelde map zien, renderen samen: een afbeelding in
tegels, een Grote render, een Monte Carlo-afbeelding, een animatie of een batch. Elke
computer neemt de delen die nog niemand heeft, zodat een snelle computer er meer
neemt, en de computer die de render startte, voegt de resultaten
samen. De map regelt het delen: het programma opent geen netwerkverbinding
en praat met niemand, dus het is zo veilig als uw bestandsdeling.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D met het venster Zwerm, geopend met de knop Zwerm op het tabblad Extra, terwijl twee computers een afbeelding in tegels renderen: de gedeelde map, Deze computer delen, de computers en wat elk doet, de taak, en de voorvertoning met gerenderde tegels, een tegel in bewerking en ontbrekende tegels"></a>

<a id="setting-it-up"></a>

### Instellen

Stap voor stap met wat u moet controleren: [Zwerm instellen](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-nl) in de wiki.

1. **Een map waarin elke computer kan schrijven:** een share op een NAS, een Windows-share,
   een Samba-share of NFS. Koppel hem op Linux aan als map, bijvoorbeeld
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   een share die alleen in de bestandsbeheerder is geopend (`smb://...`), is geen map
   die het programma kan gebruiken.
2. **Dezelfde versie van mb3d-rs op elke computer.** Een computer met
   een andere versie wordt vermeld maar doet niet mee.
3. **In Mandelbulb3D:** de knop **Zwerm** op het tabblad Extra. Kies de
   gedeelde map (dezelfde share op elke computer; het pad mag verschillen),
   geef de computer een naam voor de lijst, en vink **Deze computer
   delen** aan.

Dat is alles. De statusregel zegt wat er gebeurt, bijvoorbeeld *Gedeeld
met omarchy: renderingen worden over de 2 computers verdeeld*. Wie alleen in de
zwerm zit, rendert alles zoals gewoonlijk op deze computer.

<a id="what-is-shared"></a>

### Wat wordt gedeeld

Zolang *Deze computer delen* is aangevinkt, rendert de computer de delen die
andere computers in de map zetten, en gaan zijn eigen renders naar de zwerm
zodra er een andere computer in zit:

* **3D berekenen** (en het venster Batchverwerking) — de afbeelding in N x M tegels
  (in te stellen in het venster Zwerm), hier samengevoegd met alles wat de gewone
  berekening geeft: belichtingswijzigingen, nabewerking, opslaan als m3i (en
  het geheugengebruik ervan, zie hieronder).
* **Grote render** — de tegels worden een voor een opgeslagen zodra ze binnenkomen; de
  hele afbeelding wordt nooit in het geheugen gehouden, dus dit is de manier voor afbeeldingen
  van 100000 x 100000 pixels en meer.
* **Monte Carlo** — zoals op één computer: elke computer voegt passes toe aan zijn
  eigen blok tot u op Stop drukt, de afbeelding wordt schoner met de stralen van
  iedereen, en Stop middelt de blokken. In de batchmodus van het venster stoppen de
  blokken samen bij het aantal stralen van de batch.
* **De Animatiemaker** — de frames, een voor een.

Het venster toont de computers met hun kernen, grafische kaart en wat
elk aan het doen is, de taken met hun voortgang, en een voorvertoning van de nieuwste
taak: de gerenderde tegels, de tegels in bewerking met een voortgangsbalk, en de
tegels die nog ontbreken.

<a id="on-the-command-line"></a>

### Op de opdrachtregel

Start op elke computer die moet helpen een worker op de map:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Render daarna op een willekeurige computer met `--swarm` en dezelfde map:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` toont wie er in de zwerm zit, wat elke computer
doet en hoe ver de taken zijn. Een Monte Carlo-afbeelding op de opdrachtregel
wordt verdeeld in blokken met hun eigen willekeurige stralen: 4 blokken van 64 stralen per
pixel geven de 256 die met `--rays 256` zijn gevraagd.

<a id="good-to-know"></a>

### Goed om te weten

* **Als de andere computers niets doen** en hun takenlijst leeg is terwijl
  deze computer rendert: de computer die de map met Samba aanbiedt
  en hem zelf direct gebruikt, laat de andere een oude lijst ervan bewaren. Zet
  die caching uit in `/etc/samba/smb.conf`: `smb3 directory leases = no`
  in `[global]`, en `oplocks = no` en `level2 oplocks = no` in de sectie
  van de share, voer dan `sudo systemctl restart smb` uit en koppel de share
  opnieuw aan op de andere computers.
* **Geheugen:** een afbeelding neemt in dit programma ongeveer 20 bytes per pixel in
  (de afbeeldingsbuffer), 10 meer in het hoofdvenster, en dan de grootste van
  twee fasen: de hoofdberekening met haar schaduwen en occlusion (16 per
  pixel en 1,2 GB voor de kaart), of het tekenen (3 per pixel, 20 meer
  met scherptediepte, en één band van de reflecties: hooguit 3,5 GB,
  minder wanneer er weinig geheugen vrij is, omdat de band op de helft van het
  vrije geheugen bij de start van de pass wordt afgestemd). Een afbeelding van 12000 x 6000: 6 GB. Wanneer een
  afbeelding meer nodig heeft dan het vrije geheugen van de computer (opgevraagd bij Linux,
  Windows en macOS), zegt 3D berekenen hoeveel het nodig heeft en hoeveel er
  vrij is, en vraagt het voordat het begint: de berekening zou gaan swappen en het
  programma kan minutenlang niet reageren. Grote render, alleen
  of met de zwerm, is de uitweg: de tegels worden een voor een opgeslagen, de
  hele afbeelding wordt nooit vastgehouden. 3D berekenen met de zwerm houdt de
  afbeelding hier vast zoals de gewone berekening dat doet (zonder de fase van de
  hoofdberekening, die de computers van de tegels doen met het geheugen van elke
  tegel); een computer van de zwerm laat een tegel die te groot is voor zijn geheugen
  aan de andere over.
* **Veiligheid:** een computer rendert alleen wat een taak vraagt en schrijft alleen
  in de map van de taak; formulecode uit een taak draait in de formule-emulator,
  zoals elke formule.
* **Snelheid:** tegels reizen als volledige afbeeldingsbuffers, een paar honderd megabyte
  voor een zeer grote afbeelding, dus een snelle share loont. Een synchronisatiedienst die
  met vertraging kopieert, werkt ook, maar langzamer.
* **Computers die weggaan:** het deel waaraan een computer renderde, wordt
  door een andere overgenomen zodra er een halve minuut niets van hem is vernomen. Een taak waarvan
  de eigenaar al een half uur weg is, wordt verwijderd door wie hem ziet. Een taak
  die op een computer mislukt, wordt in het venster vermeld en aan de
  andere overgelaten.
* **Stoppen:** het uitvinken van *Deze computer delen* laat het lopende deel
  eerst afmaken; het programma sluiten geeft het op, en een andere computer
  neemt het over.
* **De taak stoppen** (Stop op de computer die hem maakte, die computer
  sluiten, of hij crasht): elk al voltooid deel blijft op die
  computer bewaard, en elke computer bewaart het deel dat hij aan het renderen was. Dezelfde
  render opnieuw starten maakt een taak van alleen de ontbrekende delen, en een
  computer die een deel terugkrijgt waaraan hij was begonnen, zet het voort. Hetzelfde
  werkt op de opdrachtregel: voer dezelfde opdracht opnieuw uit. Een open
  Monte Carlo-render (die van het venster) voegt zijn blokken samen wanneer hij wordt
  gestopt, zoals voorheen.

<a id="how-the-rendering-works"></a>

## Hoe het renderen werkt

Een fractalafbeelding wordt pixel voor pixel berekend: voor elke pixel wordt een straal
de scène in gevolgd tot hij het oppervlak van de fractal raakt, waarbij de formule
bij elke stap tientallen keren wordt geëvalueerd. MB3D doet dit op de
processor, met al zijn kernen. mb3d-rs kan hetzelfde, en het kan het werk ook
overdragen aan de **grafische kaart**, die duizenden kleine
processors heeft en precies voor dit soort werk is gebouwd.

De instelling **"Berekenen op"** (tabblad Intern van het hoofdvenster) kiest:

| | |
|---|---|
| **Auto** (standaard) | De kaart en de processor delen de afbeelding: de kaart neemt rijen van boven, de threads van de processor van onderen, tot ze elkaar ontmoeten. De schaduwen en de ambient occlusion worden op dezelfde manier gedeeld, of door de kaart alleen gedaan wanneer die duidelijk de snelste is. |
| **CPU** | Alleen de processor, zoals in MB3D. |
| **Grafische kaart** | Alleen de kaart (die wacht tot zijn programma klaar is). |

De eerste keer dat een combinatie van formules wordt gebruikt, moet de grafische driver
een programma voor de kaart voorbereiden — de *shader* — wat 10 tot
15 seconden duurt op de computer die hieronder is gemeten. In de modus Auto begint de processor
gewoon zonder en schuift de kaart aan zodra hij klaar is, dus een kleine
afbeelding wacht nooit. De voorbereide shader wordt bewaard voor de volgende keer (door
de grafische driver, op Linux in de map `ShaderCache` naast het
programma), zodat de volgende afbeelding met dezelfde formules meteen op de kaart
begint.

De **LED** rechts van de venstertitel vertelt waar de laatste afbeelding is
berekend: groen op de grafische kaart, oranje gedeeltelijk, rood op de
processor. Wie de muis erop houdt, ziet de details.

[Hoe het renderen werkt](../RENDERING.md) vertelt het hele verhaal met
afbeeldingen: wat Auto de eerste keer doet, waarom de kaart niet altijd de
snelste keuze is, en wat de kleuren van de LED betekenen.

Niet alles kan naar de kaart: een paar formules kunnen er niet voor
worden vertaald, en zeer diep ingezoomde beelden worden in dubbele precisie berekend
(wat maar sommige kaarten kunnen). Het programma beslist dit allemaal zelf; u ziet het alleen aan de
LED.

<a id="speed"></a>

## Snelheid

Alle metingen op één computer: een AMD Ryzen 9 3900X (12 kernen, 24
threads) met een NVIDIA GeForce RTX 2060, op Linux. De originele
**Mandelbulb3D 1.99sr37** draaide op dezelfde machine onder Wine (het
Windows-programma draait daar ongewijzigd, op volle snelheid, met alle kernen),
en werd getimed vanaf de klik op *3D berekenen* tot de voltooide afbeelding.
Elk parameterbestand werd door alle drie op dezelfde afbeeldingsgrootte gerenderd:
MB3D, mb3d-rs alleen op de processor (**CPU**) en mb3d-rs in de modus **Auto**
(grafische kaart en processor samen). De Auto-tijden zijn die van
de tweede render van een bestand, met de shader van de kaart al voorbereid; de
eerste render van een nieuwe formulecombinatie voegt de 10–15 s
shadervoorbereiding op deze kaart toe.

<a id="mb3ds-80-example-parameter-files"></a>

### De 80 voorbeeldparameterbestanden van MB3D

De voorbeeldbestanden die bij MB3D horen (en in de map
`M3Parameter` van dit programma), elk op zijn eigen afbeeldingsgrootte.

MB3D kon een van de bestanden niet berekenen (*Test8*: het stopt met de fout "Division by zero"), dus de vergelijkingen tellen 79 bestanden.

* **mb3d-rs alleen op de processor is sneller dan MB3D** in 71 van de 79 bestanden, met een mediaan van **1,7 keer**; alle bestanden samen kostten MB3D 18 min 21 s en mb3d-rs op de processor 17 min 10 s.
* **De modus Auto** is sneller dan MB3D in 77 van de 79 bestanden (mediaan 2,1 keer); alle bestanden samen 8 min 17 s. In 30 van de bestanden was de processor alleen klaar, meestal omdat hij maar een paar seconden nodig heeft, voordat de kaart aan de beurt komt — daar is Auto gewoon de processor plus een moment extra. De kaart deed mee in 49 bestanden, de tragere, waar hij de afbeelding met een mediaan van 1,6 keer sneller maakte dan de processor (tot 13 keer: *Insect Menger*, 4,7 s in plaats van 63,0 s).
* In 4 bestanden was Auto merkbaar trager dan de processor alleen (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): daar is de kaart voor de scène trager dan de 24 processorthreads.
* "Rijen op de kaart" is het deel van de afbeelding dat de grafische kaart in de modus Auto doorliep (– wanneer de processor alleen klaar was).

<details>
<summary>Alle 80 bestanden (klik om te openen)</summary>

| Parameterbestand | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | rijen op de kaart | Auto vs MB3D |
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
| Test8 | fout in MB3D | 0.1 s | 0.3 s |  | – |
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

De afbeeldingen zijn hetzelfde: mb3d-rs op de processor maakt de afbeelding van MB3D
op afronding na, en de Auto-afbeeldingen verschillen gemiddeld een
fractie van een procent van het kleurbereik van die van de processor (fijn detail kan
in andere pixels fonkelen; de [technische notities](../TECHNICAL.md)
bevatten de metingen).

<a id="the-command-line"></a>

## De opdrachtregel

`mb3d` rendert zonder venster. Open een terminal in de map van het programma
(op Windows een opdrachtprompt in de uitgepakte map, en typ `mb3d`
in plaats van `./mb3d`). Na de installatie in één regel op Linux of macOS werkt `mb3d`
in elke map, toont `man mb3d` de handleiding, en vult de Tab-toets
de opties aan in bash, zsh en fish:

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

Al het andere heeft een subopdracht met een eigen `--help`:

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

De formules en maps worden naast het programma gevonden; die van een andere
MB3D-installatie kunnen worden opgegeven met `--formulas` en `--maps`.
Animaties zijn die van MB3D: keyframes met de interpolatie van MB3D, `.m3a`-projecten
worden geopend en opgeslagen, en frames worden geschreven zoals MB3D dat doet, zodat een
onderbroken animatie verdergaat en meerdere computers er één kunnen delen.

<a id="good-to-know-1"></a>

## Goed om te weten

* **Dezelfde afbeeldingen, niet bit voor bit gelijk.** MB3D rekende in de 80-bits
  precisie van de oude x87-rekenkunde; dit programma gebruikt de 64 bit van nu. De
  afbeeldingen komen in elke structuur overeen, maar afzonderlijke pixels kunnen verschillen in het
  laatste cijfer van hun kleur. Monte Carlo-renders en MutaGen gebruiken hun
  eigen willekeurige getallen, dus hun resultaten zijn niet die van MB3D.
* **De afbeeldingen van de grafische kaart** verschillen iets van die van de processor
  (enkele precisie); in zeer fijn detail kan het fonkelen op andere
  pixels vallen. Diepe zooms schakelen zelf over op dubbele precisie.
* **De snelheid op de processor** ligt dicht bij die van MB3D: de machinecode van de formules
  uit de `.m3f`-bestanden van MB3D is vertaald zodat hij op elke
  processor draait (ook Apple Silicon), tegen een kleine prijs; het ray marching
  zelf is sneller. Zie de tabellen hierboven. Op Linux en Windows worden de
  formules van de hybride van een scène bovendien bij de start van een afbeelding samen gecompileerd tot één
  functie (in een paar milliseconden, met Cranelift),
  zodat ze hun waarden doorgeven in de registers van de processor.
  Samen met een snellere lus over de formules (op elk systeem) kosten CPU-renders
  van de 80 voorbeeldbestanden 20 % minder tijd dan in v0.2.74
  (bij sommige tot 60 %), met afbeeldingen die bit voor bit gelijk zijn. Een scène houdt de gecompileerde hybride alleen als die
  sneller blijkt; `MB3D_NO_JIT=1` schakelt het uit. Op macOS staat het
  ook aan, maar wordt het daar nog getest: `./jit_check.sh` in de uitgepakte
  map controleert het (het vergelijkt
  de gecompileerde formules met de vertaalde en rendert de voorbeeldbestanden
  met en zonder; de afbeeldingen moeten gelijk zijn).
* **Grote afbeeldingen** kunnen in één stuk worden gerenderd (een afbeelding van 15000 x 20000
  heeft ongeveer 15 GB geheugen nodig; met scherptediepte duurt het uren) of in
  tegels, ook op meerdere computers (`--tiles`, en het venster Grote renders),
  wat minder geheugen vraagt. 3D berekenen begint niet aan een
  afbeelding die niet in het vrije geheugen past (op Linux, waar het
  vrije geheugen kan worden opgevraagd) en zegt dat, en een computer van de zwerm
  laat een tegel die te groot is voor zijn geheugen aan de andere over.
* **Onvoltooide renders** worden bewaard in de cachemap (`~/.cache/mb3d-rs/resume`,
  op Windows `%LOCALAPPDATA%\mb3d-rs\resume`, of naast het programma):
  18 bytes per pixel voor een afbeelding (een afbeelding van 15360 x 15360 neemt 4,2 GB in),
  64 voor een Monte Carlo-render. Wat berekend is, wordt meteen geschreven en
  elke 5 minuten naar de schijf weggeschreven. Een render die binnen zijn eerste
  10 seconden wordt gestopt, bewaart niets; onvoltooide renders ouder dan 30 dagen verdwijnen, en de
  oudste ook wanneer ze samen meer dan 50 GB innemen. Een render gaat alleen verder
  met dezelfde parameters en programmaversie (en dezelfde keuze
  van kaart of processor; de belichting en verzadiging van een Monte Carlo-render
  mogen veranderen). Een Monte Carlo-render blijft onvoltooid tot hij zijn
  stralen bereikt (een batch, `--rays`, `--passes`) of in de lijst Hervatten wordt verwijderd.
* **Nieuwere of eigen formules, maps, thema's en talen** horen in
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` en `.../Languages`
  (bij elke manier van installeren; op Windows `%APPDATA%\mb3d-rs\...`). Ze
  worden gebruikt naast de bestanden die
  met het programma worden meegeleverd, en vervangen een meegeleverd bestand met dezelfde naam, zodat
  een bijgewerkte formule van de forums werkt zonder de map van het
  programma aan te raken.

<a id="updates"></a>

## Updates

Elke release met haar wijzigingen staat in [UPDATES.md](../UPDATES.md) (in het Engels).

<a id="for-developers"></a>

## Voor ontwikkelaars

De [technische notities](../TECHNICAL.md) beschrijven hoe de grafische kaart
wordt gebruikt, wat er uit de Delphi-bronnen van MB3D is overgezet en hoe, de drie
manieren waarop de machinecode van de formules van MB3D wordt uitgevoerd, het formaat van het scènebestand en de
sleutels ervan, de controles tegen MB3D, en het bouwen vanuit de broncode:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## Licentie

MB3D valt onder de LGPL 2.1. Deze port is een afgeleid werk en gebruikt
dezelfde licentie.

<a id="credits"></a>

## Vermeldingen

De mensen en projecten waarop mb3d-rs voortbouwt, worden genoemd in de [Engelse README](../../README.md#credits).
