<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D per Linux, Windows e macOS">

# mb3d-rs — Mandelbulb3D per Linux, Windows e macOS
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · **Italiano** · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*Tradotto dal README inglese (versione 0.2.84). Dove i due differiscono, ha ragione il [README inglese](../../README.md).*

**Download: [l'ultima versione](https://github.com/thr32423/mb3d-rs/releases/latest)** · [Download](#download) · [Novità](../UPDATES.md)

mb3d-rs è [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D), il
programma di frattali 3D, riscritto in modo che funzioni sui computer di oggi:
su **Linux, Windows e macOS**, e con l'immagine calcolata sulla
**scheda grafica**, dove MB3D poteva usare solo il processore. Apre i file di
parametri di MB3D (`.m3p`, `.m3i`, parametri in formato testo dai forum), usa
le formule e le mappe di MB3D e produce le stesse immagini.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="La finestra principale"></a>

È composto da due programmi:

* **Mandelbulb3D** — il programma desktop, con le finestre di MB3D: la
  finestra principale, Formule, Illuminazione, Postproduzione, Navigatore,
  Creazione animazioni, Elaborazione batch, Render grandi, Monte Carlo,
  Esportazione voxel, Bulb Tracer 2, MutaGen e le altre. Se conosci MB3D,
  conosci questo programma; i tutorial di MB3D valgono anche qui.

  <details><summary>Per saperne di più</summary>

  Le finestre sono quelle di MB3D — costruite dalle definizioni originali
  delle finestre di MB3D, quindi ogni pulsante, cursore e suggerimento si
  trova dove si trova in MB3D:
  
  * **Finestra principale:** aprire e salvare i parametri (`.m3p`, `.m3i`,
    testo, gli appunti), salvare le immagini (PNG, JPEG, BMP, Z-buffer a
    16 bit), le modalità del mouse, i pulsanti di rotazione, la dimensione
    dell'immagine, i preset di qualità, l'annullamento, *Calcola 3D* con
    l'immagine che cresce e le statistiche, e tutte le schede (Calcolo,
    Interno, Info, Taglio, Julia, Camera, Colorazione, Stereo).
  * **Formule:** i sei slot delle formule, gli elenchi e le opzioni delle
    formule di MB3D, i tipi di ibrido, i preferiti e l'editor di formule JIT.
  * **Illuminazione:** le luci 1–6, i preset, la tavolozza, le schede
    Oggetto, Ambiente, Nebbia d. e Imm. sfondo; le modifiche ridisegnano
    subito l'immagine, senza un nuovo calcolo.
  * **Postproduzione:** normali sullo Z-buffer, ombre nette e ambientali,
    riflessi e trasparenza, profondità di campo.
  * **Navigatore:** cammina e guardati intorno con tastiera e mouse, poi
    *Vista al princ.*
  * **Creazione animazioni, Elaborazione batch, Render grandi, Monte Carlo,
    Esportazione voxel, Bulb Tracer 2** (mesh) con l'anteprima della mesh, il
    **Generatore di HeightMap, MutaGen, ZBuf16Bit, Sequenze mappe, Cartelle
    ini** — tutto come in MB3D.
  * **Sciame** (nuovo, nella scheda Strumenti): i tuoi computer renderizzano
    insieme, vedi [Rendering su più computer](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — un renderer a riga di comando per script e lavori batch:
  immagini, animazioni, elenchi batch, render Monte Carlo, esportazioni di
  voxel e mesh, tutto senza finestra.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## È sicuro da eseguire? Controllalo tu stesso

Non devi crederci sulla parola. [scanrepo.dev](https://www.scanrepo.dev)
è un servizio indipendente che esamina un repository GitHub alla ricerca dei
segni di software dannoso: codice che avvia altri programmi, si collega a
internet, nasconde ciò che fa o aggira i controlli di sicurezza del
linguaggio di programmazione. Chiunque può eseguirlo su questo repository in
qualsiasi momento:

**[▶ Analizza mb3d-rs su scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(clicca *Rescan* per un risultato aggiornato del codice attuale)

La versione 0.2.66 è stata valutata **sicura: 0 su 100, nessun rilievo**
(in precedenza: [il risultato della 0.2.44](../img/scan_0.2.44.png)). Una
versione precedente era stata segnalata, e abbiamo modificato il codice di
conseguenza ([issue #4](https://github.com/thr32423/mb3d-rs/issues/4));
ora è scritto per restare così. I programmi non contengono codice che si
collega a internet; l'unico altro programma che chiedono al sistema di aprire
è il tuo file manager, quando clicchi un pulsante per aprire una cartella.

Un'analisi cerca segnali d'allarme noti; non è una garanzia. Questi punti ti
permettono di verificare ulteriormente da solo:

* **Tutto il codice sorgente è qui.** I download vengono compilati da esso
  sui server di compilazione di GitHub, seguendo i passaggi che puoi leggere
  nel [workflow di rilascio](.github/workflows/release.yml), non sul
  computer personale di qualcuno.
* **Ogni versione elenca i checksum** (`SHA256SUMS`), così puoi confermare
  che il file scaricato è esattamente quello pubblicato; l'installazione in
  una riga qui sotto lo fa per te. Le versioni dalla 0.2.45 in poi
  includono anche le attestazioni di compilazione di GitHub: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` mostra da quale commit è stato compilato un
  download.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Novità rispetto a Mandelbulb3D

* **La scheda grafica calcola l'immagine** — l'immagine stessa, le ombre,
  l'occlusione ambientale, i riflessi e i render Monte Carlo. Su un computer
  tipico questo è parecchie volte più veloce del solo processore; vedi
  [Velocità](#speed) più sotto.
* **Modalità Auto**: la scheda grafica e il processore lavorano insieme su
  ogni immagine, e nessuno aspetta mai — se la scheda non è pronta, il
  processore parte comunque e la scheda si aggiunge.
* **Un LED nella barra del titolo** mostra dove è stata calcolata l'ultima
  immagine.
* **Dimensione dell'interfaccia** dal 75 al 200 % per gli schermi ad alta
  risoluzione.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="Temi visivi al 150 %"></a>

  <details><summary>Per saperne di più</summary>

  **Prefer. ▸ Temi ▸ Dimensione interfaccia** ingrandisce o rimpicciolisce
  tutte le finestre e i loro testi (dal 75 al 200 %), per gli schermi ad alta
  risoluzione; la disposizione resta esattamente come in MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Temi visivi al 100 %](../img/themes_100.png) | ![Temi visivi al 150 %](../img/themes_150.png) |

  </details>
* **Otto aspetti** per le finestre, tutti file di tema che puoi copiare e
  modificare: due riproducono gli aspetti di MB3D, sei sono nuovi.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Il tema Midnight"></a>

  <details><summary>Per saperne di più</summary>

  **Prefer. ▸ Temi** offre otto aspetti, ciascuno un file nella cartella
  `Themes`: *Glossy* (quello scuro predefinito) e *Windows* riproducono i due
  aspetti di MB3D, gli altri sei sono nuovi. Clicca un'immagine per vederla a
  grandezza piena.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: il predefinito di MB3D | **Windows**: il "Themes off" di MB3D |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: grigio neutro più chiaro, accenti blu acciaio | **Daylight**: grigio chiaro caldo, accenti verde acqua |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: blu notte profondo, accenti blu | **Ember**: antracite caldo, accenti arancioni |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="Alto contrasto"></a> |
  | **Forest**: grigio verde scuro, accenti verdi | **High contrast**: nero, testo bianco, evidenziazioni gialle |
  
  Un tema è un piccolo file di testo nella cartella `Themes` accanto al
  programma; copiane uno, cambiane i colori, e comparirà nell'elenco al
  prossimo avvio.

  </details>
* **Un crash lascia una nota su ogni sistema**: un panic del programma (o
  l'errore con cui termina) viene aggiunto a `mb3d_crash.txt`, accanto al
  programma o nella sua cartella dati (prima solo su Windows). Invia quel
  file insieme a una segnalazione di bug.
* **Tutte le schede grafiche del computer calcolano**, senza nulla da
  impostare: ogni scheda trovata prende fasce di righe dell'immagine dalla
  stessa riserva della prima scheda e del processore, così una scheda più
  veloce ne prende semplicemente di più; le ombre, l'occlusione, i raggi
  Monte Carlo e i riflessi si dividono allo stesso modo, e un'animazione
  renderizza un fotogramma per scheda alla volta. Una scheda lenta (una
  grafica integrata accanto a una dedicata) fa quello che può e non blocca
  mai un'immagine: finite le righe libere, le schede inattive e il processore
  si prendono le righe che non ha ancora consegnato. Una scheda che fallisce
  viene esclusa, le altre continuano. `mb3d --list-cards` mostra cosa è
  stato trovato. (Provato con una scheda aperta due volte e con un driver
  Vulkan software come seconda scheda; non ancora su un computer con due
  schede vere.)
* **Un'immagine non terminata è un file `.m3i`.** Ciò che un render fermato,
  andato in crash o terminato forzatamente ha calcolato viene conservato
  nella cache (la finestra Render non terminati, `mb3d resume`); per
  un'immagine quel file è ora in ogni momento un file immagine `.m3i`,
  `image.m3i` nella cartella del render, con le righe non ancora calcolate
  come sfondo. Copialo dove vuoi quando vuoi, oppure premi *Save m3i* nella
  finestra Render non terminati (`mb3d resume --m3i KEY file.m3i` dalla riga
  di comando; entrambi scrivono come `.m3i` anche il `data.bin` di una
  versione precedente), e aprilo qui o in Mandelbulb3D, senza conversioni.
  (I render Monte Carlo tengono i propri dati; il `.m3c` di un passaggio si
  scrive con `--m3c`.)
* **Scorciatoie per le finestre**: una combinazione di tasti porta in primo
  piano, da qualsiasi finestra, la finestra principale, il Navigatore,
  l'Animazione, Formule, Illuminazione, Postproduzione o la finestra Monte
  Carlo (da Ctrl+1 a Ctrl+7), e Ctrl+W chiude la finestra in cui ti trovi
  (su macOS Cmd). Imposta le tue in *Prefer. > Scorciatoie*: clicca un campo
  e premi i tasti; Backspace toglie una scorciatoia.
* **L'interfaccia nella tua lingua**: ogni testo delle finestre viene da un
  file di lingua; ne sono incluse dodici, e il programma scrive per te il
  file di una nuova lingua da tradurre.

  <details><summary>Per saperne di più</summary>

  Il badge della lingua all'estremità destra della barra del titolo della
  finestra principale mostra la lingua dell'interfaccia (`EN`); un clic apre
  la finestra della lingua. Una lingua è un file di testo nella cartella
  `Languages` accanto al programma. Incluse: English, Deutsch, Français,
  Italiano, Nederlands, Polski, Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文 e
  繁體中文 (ognuna con la sua bandiera; quelle dell'Asia orientale portano un
  sottoinsieme di Noto Sans CJK per i loro caratteri, l'hindi porta Noto Sans
  Devanagari, le cui lettere legate e i segni vocalici sono composti con il
  motore di shaping del testo rustybuzz).
  Le traduzioni sono state fatte a macchina e controllate nella struttura,
  non da madrelingua: le correzioni sono benvenute, ognuna è una riga del
  file. Per tradurre: scrivi il nome della tua lingua e premi *Crea file di
  lingua*; il programma scrive `Languages/<name>.lng` con ogni testo in
  inglese, uno per riga (`Button2.Caption = Calculate
  3D`). Traduci i testi a destra del `=` con un editor di testo e premi
  *Ricarica*: le finestre si aggiornano subito, senza riavvio. Una riga
  omessa mantiene il testo inglese. *Salva + esci* mantiene la lingua per il
  prossimo avvio.

  </details>
* **Linux, Windows e macOS** (Intel e Apple Silicon).
* **Un programma a riga di comando** per tutto ciò che non ha bisogno di una
  finestra.
* **Immagini molto grandi in un unico pezzo**: 15000 x 20000 pixel e oltre
  senza esaurire la memoria, e la profondità di campo usa tutti i core del
  processore (11 volte più veloce su 24 thread rispetto a prima della 0.2.45,
  con la stessa immagine).
* **Immagini a 16 bit:** file PNG con 65.536 livelli per colore invece di
  256, per modificarle senza bande.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="Immagini a 16 bit"></a>

  <details><summary>Per saperne di più</summary>

  MB3D salva immagini a 8 bit: 256 livelli per colore. È più che sufficiente
  per guardarle, ma quando un'immagine viene modificata in seguito —
  luminosità, contrasto, curve — le aree uniformi come lo sfondo o la nebbia
  si spezzano in bande visibili. **Salva img ▸ PNG** offre un secondo tipo di
  file, **PNG colore 16 bit**: i colori così come il programma li ha
  calcolati, prima di arrotondarli a 256 livelli, con tutte le ombre, i
  riflessi e la profondità di campo. Il file appare uguale, e resta uniforme
  quando viene modificato.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="Un gradiente di sfondo dopo un forte aumento del contrasto: a gradini a 8 bit, uniforme a 16 bit"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="La finestra di salvataggio con 'PNG colore 16 bit'"></a>

  </details>
* **Un filtro antirumore** per Monte Carlo e tutte le altre immagini, il
  denoising DCT noto da GIMP, e per i render Monte Carlo anche uno neurale,
  Intel Open Image Denoise.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="Filtro antirumore"></a>

  <details><summary>Per saperne di più</summary>

  Un **filtro antirumore** toglie la grana dalle immagini: il **denoising
  DCT** di *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT* di GIMP,
  il filtro di GIMP che si è adattato meglio a queste immagini (vedi i
  riconoscimenti più sotto).
  
  * **Finestra Monte Carlo ▸ -rumore** (un pulsante che MB3D teneva
    nascosto): filtra l'immagine mostrata, e quella salvata, con
    un'intensità che segue i raggi per pixel. Il rendering vero e proprio
    continua senza filtro, e il suo aggiornamento successivo la mostra di
    nuovo non filtrata.
  * **Postproduzione ▸ Filtro rumore**: *Filtra ora* filtra l'immagine della
    finestra principale; *Filtra automaticamente le nuove immagini* filtra
    ogni nuova immagine. *Intensità* è l'intensità del rumore da togliere
    (4 è leggera; valori più alti tolgono più grana e più dettaglio fine).
  
  Le immagini Monte Carlo sono quelle con più grana. Rispetto a immagini
  renderizzate con 512 raggi per pixel, le immagini filtrate sono risultate
  più vicine di quelle non filtrate in tutti i 13 test (quattro scene, da 4 a
  32 raggi): fino al 23 % a 4 raggi, dal 5 al 10 % a 8 raggi, dall'1 al 3 %
  a 32 raggi, dove resta poca grana.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="Un'immagine Monte Carlo con 4 raggi per pixel, la stessa con il filtro antirumore, e con 512 raggi"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="Postproduzione: la sezione Filtro rumore"></a>

  Il **filtro antirumore neurale** è
  [Intel Open Image Denoise](https://www.openimagedenoise.org), il filtro di
  Cycles di Blender e di molti altri path tracer, addestrato su immagini
  renderizzate. Filtra le medie dei raggi del rendering (prima di
  esposizione e saturazione), così l'immagine mantiene la sua luminosità.
  La libreria di Intel è inclusa nel programma (la cartella `oidn`; nel .deb
  e nel .rpm, `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` può indicarne un'altra.

  * **Finestra Monte Carlo ▸ -rumore IA**: filtra l'immagine mostrata, e
    quella salvata, come fa *-rumore*; il rendering continua senza filtro.
  * **Riga di comando**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * Impiega da 0,2 a 0,6 secondi per un'immagine 640 x 480 sul processore.

  La differenza dall'immagine a 512 raggi (media su tutti i pixel, più basso
  è meglio):

  | Scena | 4 raggi | filtro DCT | filtro neurale | 8 raggi | filtro DCT | filtro neurale |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  Il filtro neurale è il più vicino in tre delle quattro scene; sulla grana
  fine dell'insieme di Julia il filtro DCT è un po' più vicino. Si nota di
  più su vetro e specchi, dove pochi raggi lasciano più grana:

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="Un'immagine Monte Carlo con 4 raggi per pixel, con il filtro DCT, con il filtro neurale, e con 512 raggi"></a>

  </details>
* **Sfondi trasparenti** per immagini e animazioni.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="Sfondo trasparente"></a>

  <details><summary>Per saperne di più</summary>

  Le immagini e i fotogrammi delle animazioni possono essere salvati con uno
  **sfondo trasparente**, per mettere il frattale su un'altra immagine o in
  un editor video: **Salva img ▸ PNG ▸ PNG sfondo trasparente**, e in
  Creazione animazioni **Sfondo trasparente** (fotogrammi PNG). Con
  l'anti-aliasing (una scala di visualizzazione di 1:2 o più, oppure `--aa`)
  i bordi sono morbidi, e non portano alcun alone del vecchio colore di
  sfondo. Le immagini di profondità per il compositing c'erano già (*Salva
  anche lo Z-buffer*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="Un'immagine con il suo sfondo, e con uno sfondo trasparente mostrato sopra una scacchiera"></a>

  </details>
* **Video delle animazioni:** un file AVI direttamente dal programma, oppure
  il comando ffmpeg per un MP4 leggero.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="Video di un'animazione"></a>

  <details><summary>Per saperne di più</summary>

  In Creazione animazioni, **Crea anche un video (AVI)** unisce i fotogrammi
  in un video quando vengono renderizzati, ai **Fotogrammi al secondo** che
  imposti: `<project name>.avi` nella cartella di uscita, accanto ai
  fotogrammi, che vengono conservati. Si riproduce in VLC e mpv e si apre
  negli editor video (Kdenlive, Shotcut, DaVinci Resolve). Ogni fotogramma è
  memorizzato come immagine intera, quindi i file sono grandi: adatti per il
  montaggio e l'archiviazione, meno per la condivisione.
  
  Per un video leggero da condividere, **Comando FFmpeg** mette il comando
  per un MP4 (o, con sfondo trasparente, un WebM) negli appunti e nei
  messaggi. Installa [ffmpeg](https://ffmpeg.org), incolla il comando in un
  terminale quando i fotogrammi sono pronti, e crea il video; questo
  programma di per sé non avvia altri programmi.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="Creazione animazioni con Crea anche un video (AVI), Fotogrammi al secondo e Comando FFmpeg"></a>

  </details>
* **Un'anteprima dei file di parametri** nella finestra di apertura.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="Anteprima all'apertura dei parametri"></a>

  <details><summary>Per saperne di più</summary>

  **Apri ▸ Apri m3p** e **Apri m3i** mostrano una piccola immagine del file
  di parametri cliccato, con la sua dimensione e le sue formule, prima che
  venga aperto; i tasti freccia scorrono l'elenco con l'anteprima.
  L'immagine viene calcolata in sottofondo sul processore, quindi la
  finestra resta reattiva, e l'anteprima di ogni file viene conservata
  finché il programma è in esecuzione.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="La finestra di apertura con l'anteprima di un file di parametri"></a>

  </details>
* **Navigatore: valori esatti e Ripristina vista.** Clicca un valore e
  scrivilo; torna alla vista da cui è partito il Navigatore.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="Navigatore: valori esatti e Ripristina vista"></a>

  <details><summary>Per saperne di più</summary>

  * **Clicca un valore** nei pannelli del Navigatore — valori Julia, valori
    delle formule, rotazione 4D, i valori Misc — e scrivi il numero esatto,
    come in MB3D (angoli in gradi). Il cursore poi prosegue da quel valore.
  * **Clicca lo zoom** e scrivilo, quando i passi di zoom sono troppo
    grossolani (un desiderio dall'elenco delle segnalazioni di MB3D); uno
    zoom scritto resta (*Zoom e passi fissi*).
  * **Ripristina vista** torna alla posizione, direzione e zoom che il
    Navigatore ha preso dalla finestra principale; i valori regolati
    restano.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="Il Navigatore con i valori delle formule e la casella di inserimento di un valore cliccato"></a>

  </details>
* **Una nuova formula, SphereTree:** un albero di sfere che si toccano e si
  ramificano sempre più piccole, di Tom Lowe.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="Una nuova formula: SphereTree"></a>

  <details><summary>Per saperne di più</summary>

  **SphereTree** è una formula che Mandelbulb3D non ha: un albero di sfere
  che si toccano e si ramificano in sfere sempre più piccole, come schiuma o
  corallo. Tom Lowe, l'inventore della Mandelbox, l'ha pubblicata nel 2020
  (vedi i riconoscimenti più sotto). Si trova nell'elenco delle formule
  insieme alle formule dIFS e ha quattro impostazioni:
  
  * **PackRatio** (da 0 a 1.22): la dimensione delle sfere; a 1 si toccano
    appena, sopra 1 quelle esterne si staccano.
  * **Levels**: quanto è profondo l'albero (6 basta per la maggior parte
    delle immagini).
  * **Rotated**: 1 ruota ogni livello di un dodicesimo di giro, e l'albero
    diventa una palla rotonda di grappoli di sfere.
  * **Colour scale**: ogni ramo dell'albero ha il suo colore. Scegli *2ª
    scelta* (l'orbit trap) nelle impostazioni del colore dell'Illuminazione
    per vederli; questa impostazione li distribuisce sul gradiente di colore.
  
  Funziona da sola e dopo le trasformazioni dIFS (la terza immagine la
  ripete con *tilingbrickIFS*), e viene calcolata sulla scheda grafica come
  sul processore. Imposta le iterazioni a 1: la formula calcola l'intero
  albero in una volta. I parametri con SphereTree si aprono solo in questo
  programma, non in Mandelbulb3D.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="Una palla SphereTree (Rotated) nel renderer Monte Carlo, al crepuscolo"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="Un primo piano della schiuma di sfere di SphereTree"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="SphereTree ripetuta con tilingbrickIFS: uno sciame di alberi di sfere"></a> |
  |:---:|:---:|

  </details>
* **Una nuova formula, DomeTree:** una cupola da cui spunta un anello di
  cupole più piccole, ognuna con il suo anello, di Tom Lowe.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="Una nuova formula: DomeTree"></a>

  <details><summary>Per saperne di più</summary>

  **DomeTree** è il secondo albero di Tom Lowe dopo SphereTree (vedi i
  riconoscimenti): una sfera su un cui lato cresce un anello di cupole, ogni
  cupola con il suo anello più piccolo, livello dopo livello, fatto di
  inversioni circolari. Si trova nell'elenco delle formule insieme alle
  formule dIFS e ha cinque impostazioni:

  * **Levels**: quanto è profondo l'albero (16 basta e avanza).
  * **Children**: le cupole in ogni anello, da 4 a 10.
  * **Bend angle** (da 0.3 a 1.3 funziona meglio): quanto sporgono le
    cupole; valori bassi danno dischi piatti, valori alti cupole rotonde.
    Oltre circa 1.3 la superficie diventa granulosa.
  * **Colour scale**: ogni cupola ha il suo colore. Scegli *2ª scelta*
    (l'orbit trap) nelle impostazioni del colore dell'Illuminazione e un
    ampio intervallo di colori (per esempio da 0 a 100) per vederli.
  * **Start curved**: 1 (il predefinito) parte dalla sfera, 0 da un
    pavimento piatto su cui cresce il primo anello.

  Usa un *Moltiplicatore passo raggio* piccolo (0.2; 0.05 con 9 o 10
  figli): la stima della distanza della forma è approssimativa, come nel
  file Fragmentarium di TGlad stesso. Come SphereTree funziona sulla scheda
  grafica e sul processore, richiede solo 1 iterazione, e i parametri che la
  usano si aprono solo in questo programma.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="DomeTree con 8 figli nel renderer Monte Carlo, colorata per cupola"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 5 figli, bend angle 1.2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 6 figli, bend angle 0.5: cupole piatte"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 10 figli: merletto tra le cupole e una grande sfera"></a> |
  |:---:|:---:|:---:|

  </details>
* **Una nuova formula, UlamSpiral:** la spirale dei numeri primi di Ulam in
  3D, come città, skyline, sfere o blocco.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="Una nuova formula: UlamSpiral"></a>

  <details><summary>Per saperne di più</summary>

  Nel 1963 il matematico Stanisław Ulam, durante una riunione, assisteva a
  "una relazione lunga e molto noiosa" e scarabocchiava su un foglio a
  quadretti. La sua prima idea era di impostare problemi di scacchi; invece
  numerò i quadretti 1, 2, 3, … a spirale, con l'1 al centro, e cerchiò i
  numeri primi. I primi si allinearono su linee diagonali, come se seguissero
  uno schema, e lo fanno ancora quando un computer ne disegna milioni (Ulam
  ne provò 100.000 sul MANIAC II a Los Alamos; Martin Gardner mise
  l'immagine sulla copertina di Scientific American nel 1964). Nessuno l'ha
  spiegato del tutto fino a oggi.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="La spirale di Ulam: i numeri primi tra i numeri da 1 a 78.961 su una spirale quadrata, con l'inizio della spirale disegnato nell'angolo"></a>
  
  **UlamSpiral** costruisce quella spirale in 3D, una forma per ogni numero,
  in quattro varianti (*Mode*):
  
  * **0, torri dei primi**: una lastra per ogni numero e una torre su ogni
    primo. Le linee diagonali dei primi diventano file di torri.
  * **1, skyline dei divisori**: una colonna per ogni numero, tanto più alta
    quanti più divisori ha; i primi sono le strade basse tra le colonne.
  * **2, sfere dei divisori**: una sfera per ogni numero, tanto più grande
    quanti più divisori ha (i primi sono quelle piccole).
  * **3, un blocco di primi**: i numeri avvolti su gusci cubici invece che su
    un quadrato, un cubo su ogni primo.
  
  *Rings* imposta la dimensione della spirale (60 anelli sono i numeri fino a
  14.641), *Height* l'altezza delle torri o colonne, *Gap* lo spazio tra di
  esse. Con *2ª scelta* (l'orbit trap) nelle impostazioni del colore, il
  colore segue il numero di divisori, così i primi hanno un colore tutto
  loro. Come SphereTree funziona sulla scheda grafica e sul processore,
  funziona dopo le trasformazioni dIFS, richiede solo 1 iterazione, e i
  parametri che la usano si aprono solo in questo programma.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, torri dei primi: le file diagonali di numeri primi come una città, nel renderer Monte Carlo con nebbia, ombre nette e profondità di campo"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, skyline dei divisori"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, sfere dei divisori"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, un blocco di primi su gusci cubici"></a> |
  |:---:|:---:|:---:|

  </details>
* **Un render fermato riprende da dove era.** Ciò che un render ha calcolato
  viene conservato quando lo fermi, e anche quando il programma viene chiuso
  o va in crash, o il computer si spegne (viene salvato su disco ogni 5
  minuti): Calcola 3D, Monte Carlo (anche a metà di un passaggio), il
  fotogramma di un'animazione o l'immagine di un batch in corso,
  un'esportazione voxel, una mesh di Bulb Tracer, i riquadri di Render
  grande, e un render dello sciame (Ferma sul computer che ha creato il
  lavoro conserva ogni parte finita, e ogni computer conserva la parte a cui
  stava lavorando). Calcola 3D e Monte Carlo con gli stessi parametri
  chiedono se continuare, gli altri continuano da soli; il pulsante
  **Riprendi** (scheda Apri) elenca i render non terminati, per continuarli o
  eliminarli. Dalla riga di comando lo stesso comando continua da solo
  (anche un'immagine in `--tiles`, `montecarlo`, `animate`, `batch`, `voxel`
  e `mesh`); `--fresh` ricomincia da capo, `mb3d
  resume` li elenca. Un render ripreso è identico a uno calcolato tutto in
  una volta.
* **Rendering condiviso tra i tuoi computer**, uno "sciame": i computer che
  vedono una stessa cartella condivisa renderizzano insieme un'immagine, un
  Render grande, un'immagine Monte Carlo o un'animazione. Vedi
  [Rendering su più computer](#rendering-on-several-computers-the-swarm).
* **Una nuova formula, PseudoKleinian:** l'"architettura kleiniana" di sale,
  archi e sfere come un'unica formula con la sua distanza esatta.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="Una nuova formula: PseudoKleinian"></a>

  <details><summary>Per saperne di più</summary>

  Le sale, gli archi, i pilastri e le sfere annidate noti come "architettura
  kleiniana" sono tra le immagini di frattali 3D più amate. In MB3D si
  possono ottenere solo come ibrido di più formule con impostazioni
  accuratamente abbinate, la ricetta trovata da Theli-at nel 2011; knighty
  ha poi scritto la stessa cosa come un'unica formula con la sua stima della
  distanza, ed è **PseudoKleinian**. Ogni livello ripiega il punto in una
  scatola, lo inverte in una sfera e aggiunge uno spostamento, e la distanza
  viene misurata esattamente fino a un piano di pavimento dopo le pieghe,
  così le pareti risultano pulite e l'immagine si renderizza velocemente. Le
  impostazioni sono quelle di knighty: *Levels* (8), *Size* (la sfera di
  inversione), *CSize X/Y/Z* (la scatola; i suoi 0.924, 0.908, 0.924 danno
  le sale classiche, e piccole modifiche danno nuovi edifici), *Julia X/Y/Z*
  (uno spostamento per livello), *Shape* (0 il piano del pavimento, 1 un
  piano-cilindro di spessore *Thickness* passante per *Offset*, 2 una
  scatola arrotondata), *DE offset* (svuota la superficie) e *Colour scale*
  (con *2ª scelta*, l'orbit trap: il raggio d'orbita più piccolo, le
  classiche bande kleiniane).
  
  La struttura riempie tutto lo spazio, quindi la guardi dall'interno: parti
  con il centro della vista vicino all'origine e la camera una o due unità
  indietro, e muoviti tra le sale con il Navigatore. Nel renderer Monte
  Carlo un interno non riceve quasi luce dal cielo, quindi metti una luce
  posizionale nella sala (la prima immagine) oppure apri la struttura con un
  piano di taglio e guarda dentro dall'alto (la seconda). Funziona sulla
  scheda grafica e sul processore, richiede 1 iterazione, funziona dopo le
  trasformazioni dIFS e nelle combinazioni DE come le altre formule dIFS
  (anche se dentro una struttura che riempie lo spazio cambiano poco), e i
  parametri che la usano si aprono solo in questo programma.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="PseudoKleinian dal punto di vista di knighty: una sala di archi e sfere annidate illuminata da una lampada all'interno, nel renderer Monte Carlo con nebbia, ombre nette e profondità di campo"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="PseudoKleinian aperta con un piano di taglio e vista dall'alto: il reticolo di sfere e le celle tra di esse"></a>

  </details>
* **Coppie stereo in un solo passaggio**: entrambe le immagini per gli occhi
  renderizzate e mostrate affiancate, per la visione parallela o a occhi
  incrociati. Contributo di Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Una coppia stereo di Hydra di Dainbramage, affiancata per la visione parallela"></a>

  <details><summary>Per saperne di più</summary>

  **Stereo ▸ Rendi coppia stereo** renderizza l'immagine dell'occhio sinistro
  e poi quella del destro con le impostazioni stereo della scheda e le
  mostra affiancate nella finestra principale. Sono disposte per la visione
  parallela (immagine sinistra a sinistra); **Anteprima a occhi incrociati**
  le scambia per la visione a occhi incrociati. È un'anteprima:
  illuminazione e colori
  non si possono cambiare dopo senza renderizzare di nuovo, e non funziona
  con i riquadri. I pulsanti per le immagini dei singoli occhi ci sono
  ancora.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Una coppia stereo di Hydra di Dainbramage, affiancata per la visione parallela"></a>

  </details>
* **Tempo rimanente per l'intera immagine**: la barra sotto l'immagine si
  riempie una sola volta lungo tutto il calcolo, e accanto c'è il tempo
  rimanente fino a quando l'immagine è finita, ombre, occlusione e riflessi
  compresi.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="La barra di avanzamento e il tempo rimanente totale"></a>

  <details><summary>Per saperne di più</summary>

  La barra di MB3D ricomincia a ogni fase. Qui mostra l'intero calcolo: il
  rendering principale, poi le ombre nette e l'occlusione ambientale, i
  riflessi e la profondità di campo. La durata delle fasi successive al
  rendering principale viene stimata da ciò che la scena richiede e da come è
  andato il rendering principale; la stima è stata tarata su circa 1400
  render di 366 scene. Un'immagine calcolata di nuovo si aspetta i tempi
  dell'ultima volta.
  
  La prima immagine con nuove formule sulla scheda grafica attende la
  compilazione dei suoi shader: la riga allora dice *compilazione degli
  shader* con il tempo rimanente, che tiene conto del ritmo del processore e,
  quando gli shader saranno pronti in tempo, dell'ingresso della scheda.
  Dettagli in
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="La finestra principale durante le ombre nette e l'occlusione ambientale, con il tempo rimanente totale"></a>

  </details>
* **History** dietro l'icona dell'orologio nella barra del titolo: ogni
  immagine che hai calcolato con la sua miniatura, ogni file che hai aperto
  o salvato, come elenco, come righe o come riquadri di qualsiasi
  dimensione, raggruppati per giorno o in un'unica sequenza; puoi cercarvi,
  ordinarla, mostrare solo le immagini che differiscono e caricare qualsiasi
  voce con un doppio clic. Una cronologia di centinaia di migliaia di voci si
  apre all'istante.
* **L'illuminazione resta interattiva** dopo ombre nette e riflessi: sposta
  una luce o cambiane il colore e l'immagine cambia subito, come in
  Mandelbulb3D.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="La finestra principale e la finestra Illuminazione subito dopo una modifica dell'illuminazione"></a>

  <details><summary>Per saperne di più</summary>

  Renderizza in modo semplice, poi prova posizioni e colori delle luci: ogni
  modifica nella finestra Illuminazione ridisegna subito con le ombre così
  come sono state calcolate, come in Mandelbulb3D. Cambiare il colore o
  l'intensità di una luce non ricalcola mai le ombre. Le ombre di una luce
  che hai spostato, i riflessi e la profondità di campo tornano con i
  pulsanti della finestra Postproduzione, come in Mandelbulb3D.
  
  Un calcolo che hai fermato si può ancora continuare anche dopo aver
  cambiato l'illuminazione. Un modo di lavorarci: [Flusso di lavoro
  dell'illuminazione](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-it)
  nella wiki. Dettagli in
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="La finestra principale e la finestra Illuminazione subito dopo una modifica dell'illuminazione: l'immagine già più luminosa"></a>

  </details>

<a id="download"></a>

## Download

Programmi pronti all'uso per l'ultima versione, senza installazione:

| Sistema | Download |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (si installa come gli altri programmi) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (si installa come gli altri programmi) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple silicon (M1 e successivi) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Estrai l'archivio e avvia **Mandelbulb3D** (`Mandelbulb3D.exe` su Windows)
nella cartella estratta. La cartella è organizzata come un'installazione di
MB3D, con le formule e le mappe di MB3D e 80 file di parametri di esempio.

* Se Windows mostra "Windows ha protetto il PC", clicca *Ulteriori
  informazioni* ▸ *Esegui comunque*.
* macOS blocca i programmi scaricati che non sono firmati: esegui una volta
  `xattr -dr com.apple.quarantine .` nella cartella estratta.

**Alternativa per Linux e macOS:** invece di scaricare a mano, questa riga
scarica l'ultima versione in `~/mb3d-rs` (eseguila di nuovo più tardi per
aggiornare; lo script [download.sh](../../download.sh) si trova anche nella
cartella del programma):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Controlla il download prima di estrarlo, mette **Mandelbulb3D** nel menu
delle applicazioni (Linux) e crea i comandi `Mandelbulb3D` e `mb3d` che puoi
digitare in qualsiasi cartella. `sh ~/mb3d-rs/download.sh
--uninstall` toglie di nuovo la voce di menu e i comandi; i tuoi file
restano.

I programmi per Linux funzionano sulle distribuzioni dal 2018 circa in poi
(Ubuntu 18.04, Debian 10, Rocky/RHEL 8 e successive).

**Installato come gli altri programmi:** su Debian, Ubuntu e Mint, apri il
file `.deb` qui sopra (oppure `sudo apt install ./mb3d-rs-linux-x64.deb`);
su Fedora, RHEL e Rocky il file `.rpm` (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, su openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D si trova poi nel menu delle applicazioni e `mb3d` funziona in
qualsiasi cartella; le tue impostazioni e la History sono conservate in
`~/.config/mb3d-rs` e `~/.local/share/mb3d-rs`.

[Primi passi](../../GETTING_STARTED.md) accompagna nei primi passi; le
versioni precedenti e le note di rilascio sono nella pagina delle
[versioni](https://github.com/thr32423/mb3d-rs/releases).

<a id="guides-and-help"></a>

## Guide e aiuto

Le guide passo passo sono nella [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-it):

* [Installazione](https://github.com/thr32423/mb3d-rs/wiki/Installing-it): Windows, macOS e Linux, aggiornamento, dove si trovano i tuoi file
* [Flusso di lavoro dell'illuminazione](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-it): renderizza in modo semplice, poi prova luci, ombre e riflessi
* [Configurare lo sciame](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-it): rendering su più computer tramite una cartella condivisa
* [Risoluzione dei problemi e FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-it): quando qualcosa non funziona come previsto

Ogni versione con le sue modifiche: [l'elenco completo](../UPDATES.md).

<a id="rendering-on-several-computers-the-swarm"></a>

## Rendering su più computer (lo sciame)

I computer che vedono una stessa cartella condivisa renderizzano insieme:
un'immagine a riquadri, un Render grande, un'immagine Monte Carlo,
un'animazione o un batch. Ogni computer prende le parti che nessun altro ha
preso, così uno veloce ne prende di più, e il computer che ha avviato il
render mette insieme i risultati. La condivisione la fa la cartella: il
programma non apre alcuna connessione di rete e non parla con nessuno,
quindi è sicuro quanto la tua condivisione di file.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D con la finestra Sciame, aperta con il pulsante Sciame nella scheda Strumenti, mentre due computer renderizzano un'immagine a riquadri: la cartella condivisa, Condividi questo computer, i computer e cosa sta facendo ciascuno, il lavoro, e l'anteprima con i riquadri renderizzati, un riquadro in corso e quelli mancanti"></a>

<a id="setting-it-up"></a>

### Configurazione

Passo per passo, con cosa controllare: [Configurare lo sciame](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-it) nella wiki.

1. **Una cartella in cui ogni computer può scrivere:** una condivisione su un
   NAS, una condivisione Windows, una condivisione Samba o NFS. Su Linux,
   montala come cartella, per esempio
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   una condivisione aperta solo nel file manager (`smb://...`) non è una
   cartella che il programma possa usare.
2. **La stessa versione di mb3d-rs su ogni computer.** Un computer con
   un'altra versione compare nell'elenco ma non partecipa.
3. **In Mandelbulb3D:** il pulsante **Sciame** nella scheda Strumenti.
   Scegli la cartella condivisa (la stessa condivisione su ogni computer; il
   suo percorso può essere diverso), dai al computer un nome per l'elenco e
   attiva **Condividi questo computer**.

Tutto qui. La riga di stato dice cosa succede, per esempio *Condiviso con
omarchy: i render sono suddivisi tra i 2 computer*. Da solo nello sciame,
tutto viene renderizzato su questo computer come al solito.

<a id="what-is-shared"></a>

### Cosa viene condiviso

Finché *Condividi questo computer* è attivo, il computer renderizza le parti
che gli altri computer mettono nella cartella, e i suoi render vanno allo
sciame ogni volta che c'è un altro computer:

* **Calcola 3D** (e la finestra Batch) — l'immagine in N x M riquadri
  (impostati nella finestra Sciame), ricomposta qui con tutto ciò che dà il
  calcolo normale: modifiche dell'illuminazione, postproduzione, salvataggio
  come m3i (e la sua memoria, vedi sotto).
* **Render grande** — i riquadri vengono salvati uno per uno man mano che
  arrivano; l'intera immagine non viene mai tenuta in memoria, quindi questa
  è la strada per immagini di 100000 x 100000 pixel e oltre.
* **Monte Carlo** — come su un solo computer: ogni computer aggiunge
  passaggi al proprio blocco finché non premi Ferma, l'immagine si pulisce
  con i raggi di tutti, e Ferma fa la media dei blocchi. Nella modalità batch
  della finestra, i blocchi insieme si fermano al numero di raggi del batch.
* **Creazione animazioni** — i fotogrammi, uno per uno.

La finestra elenca i computer con i loro core, la scheda grafica e ciò che
sta facendo ciascuno, i lavori con il loro avanzamento, e un'anteprima del
lavoro più recente: i riquadri renderizzati, quelli in corso con una barra
di avanzamento, e quelli ancora mancanti.

<a id="on-the-command-line"></a>

### Dalla riga di comando

Su ogni computer che deve aiutare, avvia un worker sulla cartella:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Poi, su qualsiasi computer, renderizza con `--swarm` e la stessa cartella:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` mostra chi è nello sciame, cosa sta facendo ogni
computer e a che punto sono i lavori. Un'immagine Monte Carlo dalla riga di
comando viene divisa in blocchi con i propri raggi casuali: 4 blocchi da 64
raggi per pixel danno i 256 richiesti con `--rays 256`.

<a id="good-to-know"></a>

### Buono a sapersi

* **Se gli altri computer restano inattivi** e il loro elenco dei lavori è
  vuoto mentre questo computer renderizza: il computer che serve la cartella
  con Samba e la usa direttamente fa sì che gli altri ne tengano un elenco
  vecchio. Disattiva quella cache in `/etc/samba/smb.conf`: `smb3 directory leases = no`
  in `[global]`, e `oplocks = no` e `level2 oplocks = no` nella sezione della
  condivisione, poi `sudo systemctl restart smb` e rimonta la condivisione
  sugli altri computer.
* **Memoria:** un'immagine occupa circa 20 byte per pixel in questo
  programma (il suo buffer d'immagine), altri 10 nella finestra principale,
  e poi la maggiore di due fasi: il calcolo principale con ombre e
  occlusione (16 per pixel e 1,2 GB per la scheda), oppure la pittura (3 per
  pixel, altri 20 con la profondità di campo, e una fascia dei riflessi:
  3,5 GB al massimo, meno quando c'è poca memoria libera, dato che la fascia
  è dimensionata a metà di quella libera quando il passaggio inizia).
  Un'immagine 12000 x 6000: 6 GB. Quando un'immagine richiede più della
  memoria libera del computer (chiesta a Linux, Windows e macOS), Calcola 3D
  dice quanta ne serve e quanta è libera, e chiede prima di partire: il
  calcolo finirebbe nello swap e il programma potrebbe smettere di
  rispondere per minuti interi. Render grande, da solo o con lo sciame, è la
  via d'uscita: i suoi riquadri vengono salvati uno per uno, l'intera
  immagine non viene mai tenuta in memoria. Calcola 3D con lo sciame tiene
  l'immagine qui come fa il calcolo normale (senza la fase del calcolo
  principale, che i computer dei riquadri eseguono con la memoria di ciascun
  riquadro); un computer dello sciame lascia agli altri un riquadro troppo
  grande per la sua memoria.
* **Sicurezza:** un computer renderizza solo ciò che un lavoro chiede e
  scrive solo nella cartella del lavoro; il codice delle formule di un lavoro
  gira nell'emulatore di formule come ogni formula.
* **Velocità:** i riquadri viaggiano come buffer d'immagine completi, qualche
  centinaio di megabyte per un'immagine molto grande, quindi una
  condivisione veloce ripaga. Funziona anche un servizio di
  sincronizzazione che copia con un ritardo, ma più lentamente.
* **Computer che se ne vanno:** la parte che un computer stava renderizzando
  viene presa da un altro quando non se ne ha notizia da mezzo minuto. Un
  lavoro il cui proprietario è sparito da mezz'ora viene rimosso da
  chiunque lo veda. Un lavoro che fallisce su un computer viene annotato
  nella finestra e lasciato agli altri.
* **Fermarsi:** disattivare *Condividi questo computer* lascia prima finire
  la parte in corso; chiudere il programma la abbandona, e un altro computer
  la prende.
* **Fermare il lavoro** (Ferma sul computer che l'ha creato, oppure
  chiudendolo, oppure se va in crash): ogni parte già finita viene
  conservata su quel computer, e ogni computer conserva la parte che stava
  renderizzando. Avviando di nuovo lo stesso render si crea un lavoro con le
  sole parti mancanti, e un computer che riceve di nuovo una parte che aveva
  iniziato la continua. Lo stesso funziona dalla riga di comando: esegui di
  nuovo lo stesso comando. Un render Monte Carlo senza limite (quello della
  finestra) mette insieme i suoi blocchi quando viene fermato, come prima.

<a id="how-the-rendering-works"></a>

## Come funziona il rendering

Un'immagine frattale viene calcolata pixel per pixel: per ogni pixel si
segue un raggio nella scena finché non colpisce la superficie del frattale,
e la formula viene valutata decine di volte a ogni passo. MB3D lo fa sul
processore, usando tutti i suoi core. mb3d-rs può fare lo stesso, e può
anche affidare il lavoro alla **scheda grafica**, che ha migliaia di piccoli
processori ed è fatta proprio per questo genere di lavoro.

L'impostazione **"Calcola su"** (scheda Interno della finestra principale)
sceglie:

| | |
|---|---|
| **Auto** (predefinito) | La scheda e il processore si dividono l'immagine: la scheda prende le righe dall'alto, i thread del processore dal basso, finché non si incontrano. Le ombre e l'occlusione ambientale si dividono allo stesso modo, oppure le fa la sola scheda quando è chiaramente la più veloce. |
| **CPU** | Solo il processore, come in MB3D. |
| **Scheda grafica** | Solo la scheda (aspetta che il suo programma sia pronto). |

La prima volta che si usa una combinazione di formule, il driver grafico
deve preparare un programma per la scheda — lo *shader* — e questo richiede
da 10 a 15 secondi sul computer misurato più sotto. In modalità Auto il
processore parte semplicemente senza di esso e la scheda si aggiunge quando
è pronta, quindi un'immagine piccola non aspetta mai. Lo shader preparato
viene conservato per la volta successiva (dal driver grafico, su Linux nella
cartella `ShaderCache` accanto al programma), così l'immagine successiva con
le stesse formule parte subito sulla scheda.

Il **LED** a destra del titolo della finestra indica dove è stata calcolata
l'ultima immagine: verde sulla scheda grafica, arancione in parte, rosso sul
processore. Passandoci sopra con il puntatore si vedono i dettagli.

[Come funziona il rendering](../RENDERING.md) racconta tutta la storia con
immagini: cosa fa Auto la prima volta, perché la scheda non è sempre la
scelta più veloce, e cosa significano i colori del LED.

Non tutto può andare sulla scheda: alcune formule non possono essere
tradotte per essa, e le viste con zoom molto profondi vengono calcolate in
doppia precisione (cosa che solo alcune schede sanno fare). Il programma
decide tutto questo da solo; lo vedi solo nel LED.

<a id="speed"></a>

## Velocità

Tutte le misure su un solo computer: un AMD Ryzen 9 3900X (12 core, 24
thread) con una NVIDIA GeForce RTX 2060, su Linux. L'originale
**Mandelbulb3D 1.99sr37** è stato eseguito sulla stessa macchina con Wine
(il suo programma Windows ci gira senza modifiche, a piena velocità, usando
tutti i core), e cronometrato dal clic su *Calcola 3D* all'immagine finita.
Ogni file di parametri è stato renderizzato alla stessa dimensione
d'immagine da tutti e tre: MB3D, mb3d-rs solo sul processore (**CPU**) e
mb3d-rs in modalità **Auto** (scheda grafica e processore insieme). I tempi
Auto sono quelli del secondo render di un file, con lo shader della scheda
già preparato; il primo render di una nuova combinazione di formule aggiunge
i 10–15 s di preparazione dello shader su questa scheda.

<a id="mb3ds-80-example-parameter-files"></a>

### Gli 80 file di parametri di esempio di MB3D

I file di esempio forniti con MB3D (e presenti nella cartella `M3Parameter`
di questo programma), ciascuno alla propria dimensione d'immagine.

MB3D non è riuscito a calcolare uno dei file (*Test8*: si ferma con un errore "Division by zero"), quindi i confronti contano 79 file.

* **mb3d-rs sul solo processore è più veloce di MB3D** in 71 dei 79 file, con una mediana di **1,7 volte**; tutti i file insieme hanno richiesto a MB3D 18 min 21 s e a mb3d-rs sul processore 17 min 10 s.
* **La modalità Auto** è più veloce di MB3D in 77 file su 79 (mediana 2,1 volte); tutti i file insieme 8 min 17 s. In 30 dei file il processore ha finito da solo, soprattutto perché gli bastano pochi secondi, prima che la scheda abbia il suo turno — lì Auto è semplicemente il processore più un attimo di overhead. La scheda si è aggiunta in 49 file, i più lenti, dove ha reso l'immagine in mediana 1,6 volte più veloce del processore (fino a 13 volte: *Insect Menger*, 4,7 s invece di 63,0 s).
* In 4 file Auto è stato sensibilmente più lento del solo processore (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): lì la scheda è più lenta dei 24 thread del processore per quella scena.
* "Righe sulla scheda" è la quota dell'immagine elaborata dalla scheda grafica in modalità Auto (– quando il processore ha finito da solo).

<details>
<summary>Tutti gli 80 file (clicca per aprire)</summary>

| File di parametri | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | righe sulla scheda | Auto vs MB3D |
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
| Test8 | errore in MB3D | 0.1 s | 0.3 s |  | – |
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

Le immagini sono le stesse: mb3d-rs sul processore produce l'immagine di
MB3D a meno degli arrotondamenti, e le immagini Auto differiscono da quelle
del processore in media di una frazione di punto percentuale della gamma
dei colori (il dettaglio fine può scintillare in pixel diversi; le [note
tecniche](../TECHNICAL.md) riportano le misure).

<a id="the-command-line"></a>

## La riga di comando

`mb3d` renderizza senza finestra. Apri un terminale nella cartella del
programma (su Windows un prompt dei comandi nella cartella estratta, e
digita `mb3d` invece di `./mb3d`). Dopo l'installazione in una riga su
Linux o macOS, `mb3d` funziona in qualsiasi cartella, `man mb3d` mostra il
suo manuale, e il tasto Tab completa le sue opzioni in bash, zsh e fish:

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

Tutto il resto ha un sotto-comando con il proprio `--help`:

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

Le formule e le mappe vengono cercate accanto al programma; quelle di
un'altra installazione di MB3D si possono indicare con `--formulas` e
`--maps`. Le animazioni sono quelle di MB3D: keyframe con l'interpolazione
di MB3D, progetti `.m3a` aperti e salvati, fotogrammi scritti come in MB3D
in modo che un'animazione interrotta continui e più computer possano
condividerne una.

<a id="good-to-know-1"></a>

## Buono a sapersi

* **Stesse immagini, non identiche al bit.** MB3D calcolava nella precisione
  a 80 bit della vecchia aritmetica x87; questo programma usa quella a 64 bit
  di oggi. Le immagini corrispondono in ogni struttura, ma singoli pixel
  possono differire nell'ultima cifra del loro colore. I render Monte Carlo e
  MutaGen usano i propri numeri casuali, quindi i loro risultati non sono
  quelli di MB3D.
* **Le immagini della scheda grafica** differiscono leggermente da quelle
  del processore (precisione singola); nei dettagli molto fini lo
  scintillio può cadere su altri pixel. Gli zoom profondi passano da soli
  alla doppia precisione.
* **La velocità sul processore** è vicina a quella di MB3D: il codice
  macchina delle formule dai file `.m3f` di MB3D è stato tradotto in modo che
  giri su qualsiasi processore (anche Apple Silicon), a un piccolo costo; il
  ray marching in sé è più veloce. Vedi le tabelle sopra. Su Linux e Windows
  le formule dell'ibrido di una scena vengono anche compilate insieme in
  un'unica funzione quando un'immagine parte (in pochi millisecondi, con
  Cranelift), così si passano i valori nei registri del processore. Insieme
  a un ciclo più veloce sulle formule (su ogni sistema), i render su CPU degli
  80 file di esempio richiedono il 20 % di tempo in meno rispetto alla
  v0.2.74 (fino al 60 % in alcuni), con immagini identiche al bit. Una scena
  mantiene l'ibrido compilato solo se risulta più veloce alla misura;
  `MB3D_NO_JIT=1` lo disattiva. Su macOS è attivo anch'esso, ma è ancora in
  fase di test: `./jit_check.sh` nella cartella estratta lo verifica
  (confronta le formule compilate con quelle tradotte e renderizza i file di
  esempio con e senza; le immagini devono essere le stesse).
* **Le immagini grandi** si possono renderizzare in un unico pezzo
  (un'immagine 15000 x 20000 richiede circa 15 GB di memoria; con la
  profondità di campo ci vogliono ore) oppure a riquadri, anche su più
  computer (`--tiles`, e la finestra Render grandi), il che richiede meno
  memoria. Calcola 3D non avvia un'immagine che non entra nella memoria
  libera (su Linux, dove la memoria libera si può conoscere) e lo dice, e un
  computer dello sciame lascia agli altri un riquadro troppo grande per la
  sua memoria.
* **I render non terminati** sono conservati nella cartella della cache
  (`~/.cache/mb3d-rs/resume`, su Windows `%LOCALAPPDATA%\mb3d-rs\resume`,
  oppure accanto al programma): 18 byte per pixel per un'immagine
  (un'immagine 15360 x 15360 occupa 4,2 GB), 64 per un render Monte Carlo.
  Ciò che è stato calcolato viene scritto subito e salvato su disco ogni 5
  minuti. Un render fermato entro i suoi primi 10 secondi non conserva
  nulla; i render non terminati più vecchi di 30 giorni vengono eliminati, e
  i più vecchi quando tutti insieme occupano più di 50 GB. Un render
  continua solo con gli stessi parametri e la stessa versione del programma
  (e la stessa scelta di scheda o processore; l'esposizione e la saturazione
  di un render Monte Carlo possono cambiare). Un render Monte Carlo resta non
  terminato finché non raggiunge i suoi raggi (un batch, `--rays`,
  `--passes`) o non viene eliminato nell'elenco Riprendi.
* **Formule, mappe, temi e lingue più recenti o tuoi** vanno in
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` e `.../Languages`
  (con qualsiasi tipo di installazione; su Windows `%APPDATA%\mb3d-rs\...`).
  Vengono usati in aggiunta a quelli forniti con il programma, e sostituiscono
  un file fornito con lo stesso nome, così una formula aggiornata dai forum
  ha effetto senza toccare la cartella del programma.

<a id="updates"></a>

## Aggiornamenti

Ogni versione con le sue modifiche è elencata in [UPDATES.md](../UPDATES.md) (in inglese).

<a id="for-developers"></a>

## Per gli sviluppatori

Le [note tecniche](../TECHNICAL.md) descrivono come viene usata la scheda
grafica, cosa è stato portato dai sorgenti Delphi di MB3D e come, i tre modi
in cui viene eseguito il codice macchina delle formule di MB3D, il formato
dei file di scena e le sue chiavi, i controlli rispetto a MB3D, e la
compilazione dai sorgenti:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## Licenza

MB3D è distribuito con licenza LGPL 2.1. Questo port è un'opera derivata e
usa la stessa licenza.

<a id="credits"></a>

## Riconoscimenti

Le persone e i progetti su cui si basa mb3d-rs sono citati nel [README inglese](../../README.md#credits).
