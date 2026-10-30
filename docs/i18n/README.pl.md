<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D dla Linuksa, Windows i macOS">

# mb3d-rs — Mandelbulb3D dla Linuksa, Windows i macOS
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · **Polski** · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->
*Przetłumaczone z angielskiego README (wersja 0.2.84). Tam, gdzie oba się różnią, rację ma [angielskie README](../../README.md).*

**Pobieranie: [najnowsze wydanie](https://github.com/thr32423/mb3d-rs/releases/latest)** · [Pobieranie](#download) · [Co nowego](../UPDATES.md)

mb3d-rs to [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D),
program do fraktali 3D, napisany od nowa tak, by działał na dzisiejszych
komputerach: na **Linuksie, Windows i macOS**, a obraz jest liczony na
**karcie graficznej**, podczas gdy MB3D potrafił używać tylko procesora.
Otwiera pliki parametrów MB3D (`.m3p`, `.m3i`, parametry tekstowe z forów),
używa formuł i map MB3D i tworzy te same obrazy.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="Okno główne"></a>

Składa się z dwóch programów:

* **Mandelbulb3D** — program okienkowy z oknami samego MB3D: okno
  główne, Formuły, Oświetlenie, Postprocessing, Nawigator, Animacje,
  Przetwarzanie wsadowe, Duże rendery, Monte Carlo, Eksport wokseli,
  Bulb Tracer 2, MutaGen i pozostałe. Kto zna MB3D, zna ten program;
  poradniki do MB3D nadal obowiązują.

  <details><summary>Więcej</summary>

  Okna są oknami MB3D — zbudowanymi z oryginalnych definicji okien MB3D,
  więc każdy przycisk, suwak i podpowiedź są tam, gdzie w MB3D:
  
  * **Okno główne:** otwieranie i zapisywanie parametrów (`.m3p`, `.m3i`,
    tekst, schowek), zapisywanie obrazów (PNG, JPEG, BMP, 16-bitowy
    Z-bufor), tryby myszy, przyciski obrotu, rozmiar obrazu, presety
    jakości, cofanie, *Oblicz 3D* z rosnącym obrazem i statystykami oraz
    wszystkie zakładki (Obliczenia, Wewnętrzne, Informacje, Cięcie, Julia,
    Kamera, Kolorowanie, Stereo).
  * **Formuły:** sześć gniazd formuł, listy i opcje formuł MB3D, typy
    hybryd, ulubione oraz edytor formuł JIT.
  * **Oświetlenie:** światła 1–6, presety, paleta, zakładki Obiekt,
    Otoczenie, d.Mgła i Obraz tła; zmiany od razu odświeżają obraz, bez
    nowego obliczania.
  * **Postprocessing:** normalne na Z-buforze, twarde cienie i cienie
    otoczenia, odbicia i przezroczystość, głębia ostrości.
  * **Nawigator:** chodzenie i rozglądanie się klawiszami i myszą, potem
    *Widok do głównego*.
  * **Kreator animacji, Przetwarzanie wsadowe, Duże rendery, Monte Carlo,
    Eksport wokseli, Bulb Tracer 2** (siatki) z podglądem siatki,
    **Generator map wysokości, MutaGen, ZBuf16Bit, Sekwencje map, Katalogi
    Ini** — wszystko jak w MB3D.
  * **Swarm** (nowość, na zakładce Narzędzia): Twoje komputery renderują
    razem, zob. [Renderowanie na kilku komputerach](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — renderer wiersza poleceń do skryptów i zadań wsadowych:
  obrazy, animacje, listy wsadowe, rendery Monte Carlo, eksport wokseli i
  siatek, wszystko bez okna.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## Czy uruchamianie jest bezpieczne? Sprawdź sam

Nie musisz wierzyć nam na słowo. [scanrepo.dev](https://www.scanrepo.dev)
to niezależna usługa, która bada repozytorium GitHub pod kątem oznak
szkodliwego oprogramowania: kodu, który uruchamia inne programy, łączy się
z internetem, ukrywa, co robi, albo omija zabezpieczenia języka
programowania. Każdy może ją w dowolnej chwili uruchomić na tym
repozytorium:

**[▶ Przeskanuj mb3d-rs na scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(kliknij *Rescan*, aby dostać świeży wynik dla bieżącego kodu)

Wersja 0.2.66 otrzymała ocenę **bezpieczna: 0 na 100, brak zastrzeżeń**
(wcześniej: [wynik dla 0.2.44](../img/scan_0.2.44.png)). Jedna z
wcześniejszych wersji została oznaczona i w odpowiedzi zmieniliśmy kod
([zgłoszenie #4](https://github.com/thr32423/mb3d-rs/issues/4)); teraz
jest pisany tak, by tak zostało. Programy nie zawierają kodu, który łączy
się z internetem; jedynym innym programem, o którego otwarcie proszą
system, jest Twój menedżer plików, gdy klikniesz przycisk otwierający
folder.

Skan szuka znanych sygnałów ostrzegawczych; nie jest gwarancją. Dalej
możesz sprawdzić sam:

* **Cały kod źródłowy jest tutaj.** Pliki do pobrania są z niego budowane
  przez serwery kompilacji samego GitHuba, według kroków, które można
  przeczytać w [procesie wydania](.github/workflows/release.yml), a nie na
  czyimś prywatnym komputerze.
* **Każde wydanie podaje sumy kontrolne** (`SHA256SUMS`), więc możesz
  potwierdzić, że pobrany plik jest dokładnie tym, który opublikowano;
  jednowierszowa instalacja poniżej robi to za Ciebie. Wydania od 0.2.45
  mają też poświadczenia kompilacji GitHuba: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` pokazuje, z którego commita zbudowano
  dany plik.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Co nowego w porównaniu z Mandelbulb3D

* **Obraz liczy karta graficzna** — sam obraz, cienie, okluzję otoczenia,
  odbicia i rendery Monte Carlo. Na typowym komputerze jest to kilka razy
  szybsze niż sam procesor; zob. [Szybkość](#speed) poniżej.
* **Tryb Auto**: karta graficzna i procesor pracują razem nad każdym
  obrazem i nic nigdy nie czeka — jeśli karta nie jest gotowa, procesor i
  tak zaczyna, a karta dołącza.
* **Dioda na pasku tytułu** pokazuje, gdzie policzono ostatni obraz.
* **Rozmiar interfejsu** od 75 do 200 % dla ekranów o wysokiej
  rozdzielczości.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="Motywy wizualne przy 150 %"></a>

  <details><summary>Więcej</summary>

  **Ustaw. ▸ Motywy ▸ Rozmiar interfejsu** powiększa lub pomniejsza
  wszystkie okna i ich teksty (od 75 do 200 %), dla ekranów o wysokiej
  rozdzielczości; układ okien zostaje dokładnie taki jak w MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Motywy wizualne przy 100 %](../img/themes_100.png) | ![Motywy wizualne przy 150 %](../img/themes_150.png) |

  </details>
* **Osiem wyglądów** okien, każdy w pliku motywu, który można skopiować i
  zmienić: dwa odtwarzają wygląd MB3D, sześć jest nowych.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Motyw Midnight"></a>

  <details><summary>Więcej</summary>

  **Ustaw. ▸ Motywy** oferuje osiem wyglądów, każdy jako plik w folderze
  `Themes`: *Glossy* (ciemny, domyślny) i *Windows* odtwarzają oba wyglądy
  MB3D, pozostałe sześć jest nowych. Kliknij obrazek, aby zobaczyć go w
  pełnym rozmiarze.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: domyślny wygląd MB3D | **Windows**: „Themes off” w MB3D |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: jaśniejszy neutralny szary, stalowoniebieskie akcenty | **Daylight**: ciepły jasnoszary, akcenty morskie |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: głęboki granat, niebieskie akcenty | **Ember**: ciepły grafit, pomarańczowe akcenty |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="Wysoki kontrast"></a> |
  | **Forest**: ciemny zielonoszary, zielone akcenty | **High contrast**: czerń, biały tekst, żółte wyróżnienia |
  
  Motyw to mały plik tekstowy w folderze `Themes` obok programu;
  skopiuj któryś, zmień kolory, a przy następnym uruchomieniu pojawi się
  na liście.

  </details>
* **Awaria zostawia notatkę w każdym systemie**: panika programu (lub
  błąd, na którym się kończy) jest dopisywana do `mb3d_crash.txt`, obok
  programu lub w jego folderze danych (wcześniej tylko w Windows). Dołącz
  ten plik do zgłoszenia błędu.
* **Liczą wszystkie karty graficzne komputera**, bez żadnych ustawień:
  każda znaleziona karta bierze pasy wierszy obrazu z tej samej puli co
  pierwsza karta i procesor, więc szybsza karta po prostu bierze więcej;
  cienie, okluzja, promienie Monte Carlo i odbicia są dzielone tak samo, a
  animacja renderuje naraz po jednej klatce na kartę. Wolna karta
  (zintegrowana grafika obok dedykowanej) robi, co może, i nigdy nie
  wstrzymuje obrazu: gdy wolne wiersze się skończą, bezczynne karty i
  procesor przejmują wiersze, których nie dostarczyła. Karta, która
  zawiedzie, zostaje pominięta, pozostałe pracują dalej.
  `mb3d --list-cards` pokazuje, co znaleziono. (Testowane z jedną kartą
  otwartą dwukrotnie i z programowym sterownikiem Vulkan jako drugą kartą;
  jeszcze nie na komputerze z dwiema prawdziwymi kartami.)
* **Niedokończony obraz to plik `.m3i`.** To, co policzył zatrzymany,
  zawieszony lub ubity render, jest przechowywane w pamięci podręcznej
  (okno Niedokończone rendery, `mb3d resume`); w przypadku obrazu ten plik
  jest teraz w każdej chwili plikiem obrazu `.m3i`, `image.m3i` w folderze
  renderu, z jeszcze niepoliczonymi wierszami jako tłem. Skopiuj go, kiedy
  chcesz, albo naciśnij *Save m3i* w oknie Niedokończone rendery (`mb3d
  resume --m3i KEY file.m3i` w wierszu poleceń; oba zapisują też `data.bin`
  wcześniejszej wersji jako `.m3i`) i otwórz go tutaj lub w Mandelbulb3D,
  bez żadnej konwersji. (Rendery Monte Carlo mają własne zapisy, `.m3c`
  przebiegu jest zapisywany z `--m3c`.)
* **Skróty klawiszowe do okien**: kombinacja klawiszy przywołuje na
  wierzch okno główne, Nawigator, okno Animacji, Formuł, Oświetlenia,
  Postprocessingu lub Monte Carlo z dowolnego okna (Ctrl+1 do Ctrl+7), a
  Ctrl+W zamyka okno, w którym jesteś (na macOS Cmd). Własne ustawisz w
  *Ustaw. > Skróty*: kliknij pole i naciśnij klawisze, Backspace usuwa
  skrót.
* **Interfejs w Twoim języku**: każdy tekst okien pochodzi z pliku
  językowego; dołączonych jest dwanaście języków, a program sam zapisze
  plik nowego języka do przetłumaczenia.

  <details><summary>Więcej</summary>

  Plakietka języka na prawym końcu paska tytułu okna głównego pokazuje
  język interfejsu (`EN`); kliknięcie otwiera okno języka. Język to plik
  tekstowy w folderze `Languages` obok programu. Dołączone: English,
  Deutsch, Français, Italiano, Nederlands, Polski, Kaszëbsczi, हिन्दी,
  日本語, 한국어, 简体中文 i 繁體中文 (każdy ze swoją flagą; języki
  wschodnioazjatyckie przynoszą podzbiór czcionki Noto Sans CJK dla swoich
  znaków, hindi przynosi Noto Sans Devanagari, której łączone litery i
  znaki samogłosek składa silnik kształtowania tekstu rustybuzz).
  Tłumaczenia zostały wykonane maszynowo i sprawdzone pod kątem struktury,
  a nie przez rodzimych użytkowników: poprawki są mile widziane, każda to
  jeden wiersz pliku. Aby przetłumaczyć: wpisz nazwę swojego języka i
  naciśnij *Utwórz plik języka*; program zapisze `Languages/<name>.lng` ze
  wszystkimi tekstami po angielsku, po jednym w wierszu (`Button2.Caption =
  Calculate 3D`). Przetłumacz teksty na prawo od `=` w edytorze tekstu i
  naciśnij *Wczytaj ponownie*: okna od razu się zmienią, bez ponownego
  uruchamiania. Pominięty wiersz zachowuje tekst angielski. *Zapisz +
  wyjdź* zachowuje język na następne uruchomienie.

  </details>
* **Linux, Windows i macOS** (Intel i Apple Silicon).
* **Program wiersza poleceń** do wszystkiego, co nie potrzebuje okna.
* **Bardzo duże obrazy w jednym kawałku**: 15000 x 20000 pikseli i więcej
  bez wyczerpania pamięci, a głębia ostrości używa wszystkich rdzeni
  procesora (11 razy szybciej na 24 wątkach niż przed 0.2.45, z tym samym
  obrazem).
* **Obrazy 16-bitowe:** pliki PNG z 65 536 zamiast 256 stopni na kolor, do
  obróbki bez pasów.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="Obrazy 16-bitowe"></a>

  <details><summary>Więcej</summary>

  MB3D zapisuje obrazy 8-bitowe: 256 stopni na kolor. Do oglądania to
  dużo, ale gdy obraz jest potem obrabiany — jasność, kontrast, krzywe —
  gładkie obszary, takie jak tło czy mgła, rozpadają się na widoczne pasy.
  **Zapisz obraz ▸ PNG** oferuje drugi typ pliku, **PNG 16-bitowy kolor**:
  kolory tak, jak program je policzył, zanim zostaną zaokrąglone do 256
  stopni, ze wszystkimi cieniami, odbiciami i głębią ostrości. Plik wygląda
  tak samo, a przy obróbce pozostaje gładki.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="Gradient tła po silnym podniesieniu kontrastu: schodki w 8 bitach, gładko w 16 bitach"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="Okno zapisu z „PNG 16-bitowy kolor”"></a>

  </details>
* **Filtr szumu** dla Monte Carlo i wszystkich innych obrazów,
  odszumianie DCT znane z GIMP-a, a dla renderów Monte Carlo także filtr
  neuronowy, Intel Open Image Denoise.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="Filtr szumu"></a>

  <details><summary>Więcej</summary>

  **Filtr szumu** usuwa ziarno z obrazów: to **odszumianie DCT** z GIMP-a
  (*Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT*), które spośród
  filtrów GIMP-a najlepiej pasowało do tych obrazów (zob. podziękowania
  poniżej).
  
  * **Okno Monte Carlo ▸ -szum** (przycisk, który MB3D miał ukryty):
    filtruje pokazany obraz, a także zapisywany, z siłą zależną od liczby
    promieni na piksel. Samo renderowanie trwa dalej bez filtra, a jego
    następna aktualizacja znów pokazuje obraz niefiltrowany.
  * **Postprocessing ▸ Filtr szumu**: *Filtruj teraz* filtruje obraz okna
    głównego; *Filtruj nowe obrazy automatycznie* filtruje każdy nowy.
    *Siła* to siła usuwanego szumu (4 to lekko; więcej usuwa więcej ziarna
    i więcej drobnych szczegółów).
  
  Obrazy Monte Carlo mają najwięcej ziarna. W porównaniu z obrazami
  renderowanymi z 512 promieniami na piksel obrazy filtrowane były bliższe
  niż niefiltrowane we wszystkich 13 testach (cztery sceny, od 4 do 32
  promieni): o do 23 % przy 4 promieniach, o 5 do 10 % przy 8 promieniach,
  o 1 do 3 % przy 32 promieniach, gdzie ziarna zostaje niewiele.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="Obraz Monte Carlo z 4 promieniami na piksel, ten sam z filtrem szumu i z 512 promieniami"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="Postprocessing: sekcja Filtr szumu"></a>

  **Neuronowy filtr szumu** to
  [Intel Open Image Denoise](https://www.openimagedenoise.org), filtr
  Cycles z Blendera i wielu innych path tracerów, wytrenowany na
  renderowanych obrazach. Filtruje średnie promieni renderu (przed
  ekspozycją i nasyceniem), więc obraz zachowuje jasność. Biblioteka Intela
  jest dołączona do programu (folder `oidn`; w pakietach .deb i .rpm
  `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` może wskazać inną.

  * **Okno Monte Carlo ▸ -szum AI**: filtruje pokazany obraz, a także
    zapisywany, tak jak *-szum*; renderowanie trwa dalej bez filtra.
  * **Wiersz poleceń**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * Zajmuje od 0,2 do 0,6 sekundy dla obrazu 640 x 480 na procesorze.

  Różnica względem obrazu z 512 promieniami (średnia po wszystkich
  pikselach, mniej znaczy lepiej):

  | Scena | 4 promienie | filtr DCT | filtr neuronowy | 8 promieni | filtr DCT | filtr neuronowy |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  Filtr neuronowy jest najbliższy w trzech z czterech scen; przy drobnym
  ziarnie zbioru Julii filtr DCT jest odrobinę bliższy. Najbardziej widać
  go na szkle i lustrach, gdzie przy niewielu promieniach zostaje
  najwięcej ziarna:

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="Obraz Monte Carlo z 4 promieniami na piksel, z filtrem DCT, z filtrem neuronowym i z 512 promieniami"></a>

  </details>
* **Przezroczyste tło** dla obrazów i animacji.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="Przezroczyste tło"></a>

  <details><summary>Więcej</summary>

  Obrazy i klatki animacji można zapisać z **przezroczystym tłem**, aby
  nałożyć fraktal na inny obraz albo wstawić go do edytora wideo:
  **Zapisz obraz ▸ PNG ▸ PNG z przezroczystym tłem**, a w Kreatorze animacji
  **Przezroczyste tło** (klatki PNG). Z antyaliasingiem (skala podglądu
  1:2 lub większa albo `--aa`) krawędzie są miękkie i nie mają obwódki w
  kolorze dawnego tła. Obrazy głębi do kompozycji były już wcześniej
  (*Zapisz też Z-bufor*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="Obraz z tłem i z przezroczystym tłem pokazanym na szachownicy"></a>

  </details>
* **Filmy z animacji:** plik AVI prosto z programu albo polecenie ffmpeg
  dla małego pliku MP4.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="Film z animacji"></a>

  <details><summary>Więcej</summary>

  W Kreatorze animacji opcja **Utwórz też wideo (AVI)** łączy wyrenderowane
  klatki w film z ustawioną liczbą **Klatek na sekundę**:
  `<project name>.avi` w folderze wyjściowym, obok klatek, które zostają.
  Odtwarza się w VLC i mpv i otwiera w edytorach wideo (Kdenlive, Shotcut,
  DaVinci Resolve). Każda klatka jest zapisana jako cały obraz, więc pliki
  są duże: dobre do montażu i archiwizacji, gorsze do udostępniania.
  
  Dla małego filmu do udostępnienia **Polecenie FFmpeg** umieszcza polecenie
  tworzące MP4 (albo, przy przezroczystym tle, WebM) w schowku i w
  komunikatach. Zainstaluj [ffmpeg](https://ffmpeg.org), po wyrenderowaniu
  klatek wklej polecenie do terminala, a ono utworzy film; ten program sam
  nie uruchamia żadnych innych programów.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="Kreator animacji z opcjami Utwórz też wideo (AVI), Klatek na sekundę i Polecenie FFmpeg"></a>

  </details>
* **Podgląd plików parametrów** w oknie otwierania.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="Podgląd przy otwieraniu parametrów"></a>

  <details><summary>Więcej</summary>

  **Otwórz ▸ Otwórz m3p** i **Otwórz m3i** pokazują mały obraz klikniętego
  pliku parametrów, z jego rozmiarem i formułami, zanim zostanie otwarty;
  klawisze strzałek przesuwają po liście razem z podglądem. Obraz jest
  liczony w tle na procesorze, więc okno dialogowe reaguje bez opóźnień, a
  podgląd każdego pliku jest przechowywany, dopóki program działa.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="Okno otwierania z podglądem pliku parametrów"></a>

  </details>
* **Nawigator: dokładne wartości i Przywróć widok.** Kliknij wartość i ją
  wpisz; wróć do widoku, od którego Nawigator zaczął.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="Nawigator: dokładne wartości i Przywróć widok"></a>

  <details><summary>Więcej</summary>

  * **Kliknij wartość** w panelach Nawigatora — wartości Julii, wartości
    formuły, obrót 4D, wartości Różne — i wpisz dokładną liczbę, jak w MB3D
    (kąty w stopniach). Suwak działa potem dalej od tej wartości.
  * **Kliknij zoom** i go wpisz, gdy kroki zoomu są zbyt grube (życzenie z
    listy zgłoszeń MB3D); wpisany zoom zostaje (*Stały zoom i kroki*).
  * **Przywróć widok** wraca do pozycji, kierunku i zoomu, które Nawigator
    pobrał z okna głównego; dostosowane wartości zostają.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="Nawigator z wartościami formuły i polem wpisywania klikniętej wartości"></a>

  </details>
* **Nowa formuła, SphereTree:** drzewo stykających się kul, które
  rozgałęziają się na coraz mniejsze, autorstwa Toma Lowe'a.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="Nowa formuła: SphereTree"></a>

  <details><summary>Więcej</summary>

  **SphereTree** to formuła, której Mandelbulb3D nie ma: drzewo kul, które
  stykają się ze sobą i rozgałęziają na coraz mniejsze kule, jak piana
  albo koral. Tom Lowe, wynalazca Mandelboxa, opublikował ją w 2020 roku
  (zob. podziękowania poniżej). Jest na liście formuł razem z formułami
  dIFS i ma cztery ustawienia:
  
  * **PackRatio** (od 0 do 1,22): wielkość kul; przy 1 akurat się stykają,
    powyżej 1 zewnętrzne się odrywają.
  * **Levels**: jak głęboko sięga drzewo (6 wystarcza dla większości
    obrazów).
  * **Rotated**: 1 obraca każdy poziom o jedną dwunastą obrotu, a drzewo
    staje się okrągłą kulą z gron kul.
  * **Colour scale**: każda gałąź drzewa ma własny kolor. Wybierz *2. wybór*
    (orbit trap) w ustawieniach koloru w Oświetleniu, aby je zobaczyć; to
    ustawienie rozkłada je na gradiencie kolorów.
  
  Działa samodzielnie i po transformacjach dIFS (trzeci obraz powtarza ją
  z *tilingbrickIFS*) i jest liczona zarówno na karcie graficznej, jak i na
  procesorze. Ustaw iteracje na 1: formuła liczy całe drzewo naraz.
  Parametry z SphereTree otwierają się tylko w tym programie, nie w
  Mandelbulb3D.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="Kula SphereTree (Rotated) w rendererze Monte Carlo, o zmierzchu"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="Zbliżenie piany z kul SphereTree"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="SphereTree powtórzona z tilingbrickIFS: rój drzew z kul"></a> |
  |:---:|:---:|

  </details>
* **Nowa formuła, DomeTree:** kopuła, z której wyrasta pierścień mniejszych
  kopuł, każda z własnym pierścieniem, autorstwa Toma Lowe'a.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="Nowa formuła: DomeTree"></a>

  <details><summary>Więcej</summary>

  **DomeTree** to drugie drzewo Toma Lowe'a po SphereTree (zob.
  podziękowania): kula, której jedna strona wypuszcza pierścień kopuł, a
  każda kopuła własny, mniejszy pierścień, poziom po poziomie, zbudowane z
  inwersji w okręgach. Jest na liście formuł razem z formułami dIFS i ma
  pięć ustawień:

  * **Levels**: jak głęboko sięga drzewo (16 w zupełności wystarcza).
  * **Children**: liczba kopuł w każdym pierścieniu, od 4 do 10.
  * **Bend angle** (najlepiej od 0,3 do 1,3): jak bardzo wybrzuszają się
    kopuły; niskie wartości dają płaskie dyski, wysokie okrągłe kopuły.
    Powyżej około 1,3 powierzchnia robi się ziarnista.
  * **Colour scale**: każda kopuła ma własny kolor. Wybierz *2. wybór*
    (orbit trap) w ustawieniach koloru w Oświetleniu i szeroki zakres
    kolorów (na przykład od 0 do 100), aby je zobaczyć.
  * **Start curved**: 1 (domyślnie) zaczyna od kuli, 0 od płaskiej
    podłogi, z której wyrasta pierwszy pierścień.

  Użyj małego *Raystep* (0,2; 0,05 przy 9 lub 10 kopułach potomnych):
  oszacowanie odległości tego kształtu jest zgrubne, jak w oryginalnym
  pliku Fragmentarium TGlada. Podobnie jak SphereTree działa na karcie
  graficznej i na procesorze, potrzebuje tylko 1 iteracji, a parametry z
  nią otwierają się tylko w tym programie.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="DomeTree z 8 kopułami potomnymi w rendererze Monte Carlo, każda kopuła w innym kolorze"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 5 kopuł potomnych, bend angle 1,2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 6 kopuł potomnych, bend angle 0,5: płaskie kopuły"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 10 kopuł potomnych: koronka między kopułami i duża kula"></a> |
  |:---:|:---:|:---:|

  </details>
* **Nowa formuła, UlamSpiral:** spirala liczb pierwszych Ulama w 3D, jako
  miasto, panorama, kule albo blok.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="Nowa formuła: UlamSpiral"></a>

  <details><summary>Więcej</summary>

  W 1963 roku matematyk Stanisław Ulam siedział na pewnym spotkaniu podczas
  „długiego i bardzo nudnego referatu” i bazgrał na kartce w kratkę. Jego
  pierwszym pomysłem było układanie zadań szachowych; zamiast tego
  ponumerował kratki 1, 2, 3, … wzdłuż spirali, z 1 pośrodku, i zakreślił
  liczby pierwsze. Liczby pierwsze ułożyły się na ukośnych liniach, jakby
  podlegały jakiemuś wzorowi, i nadal tak jest, gdy komputer narysuje ich
  miliony (Ulam spróbował 100 000 na MANIAC II w Los Alamos; Martin Gardner
  umieścił ten obraz na okładce Scientific American w 1964 roku). Do dziś
  nikt w pełni tego nie wyjaśnił.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="Spirala Ulama: liczby pierwsze wśród liczb od 1 do 78 961 na kwadratowej spirali, z początkiem spirali narysowanym w rogu"></a>
  
  **UlamSpiral** buduje tę spiralę w 3D, jeden kształt na liczbę, w czterech
  postaciach (*Mode*):
  
  * **0, wieże liczb pierwszych**: płytka dla każdej liczby i wieża na
    każdej liczbie pierwszej. Ukośne linie liczb pierwszych stają się
    rzędami wież.
  * **1, panorama dzielników**: kolumna dla każdej liczby, tym wyższa, im
    więcej ma dzielników; liczby pierwsze to niskie uliczki między nimi.
  * **2, kule dzielników**: kula dla każdej liczby, tym większa, im więcej
    ma dzielników (liczby pierwsze to te małe).
  * **3, blok liczb pierwszych**: liczby nawinięte na sześcienne powłoki
    zamiast na kwadrat, sześcian na każdej liczbie pierwszej.
  
  *Rings* ustala rozmiar spirali (60 pierścieni to liczby do 14 641),
  *Height* wysokość wież lub kolumn, *Gap* odstęp między nimi. Z *2.
  wybór* (orbit trap) w ustawieniach koloru kolor zależy od liczby
  dzielników, więc liczby pierwsze mają własny kolor. Podobnie jak
  SphereTree działa na karcie graficznej i na procesorze, po
  transformacjach dIFS, potrzebuje tylko 1 iteracji, a parametry z nią
  otwierają się tylko w tym programie.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, wieże liczb pierwszych: ukośne rzędy liczb pierwszych jako miasto, w rendererze Monte Carlo z mgłą, twardymi cieniami i głębią ostrości"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, panorama dzielników"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, kule dzielników"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, blok liczb pierwszych na sześciennych powłokach"></a> |
  |:---:|:---:|:---:|

  </details>
* **Zatrzymany render toczy się dalej od miejsca, w którym stanął.** To, co
  render policzył, zostaje zachowane, gdy go zatrzymasz, a także gdy
  program zostanie zamknięty lub ulegnie awarii albo komputer się wyłączy
  (co 5 minut jest zapisywane na dysk): Oblicz 3D, Monte Carlo (również w
  środku przebiegu), klatka animacji lub obraz trwającego przetwarzania
  wsadowego, eksport wokseli, siatka Bulb Tracera, kafelki Dużego renderu
  oraz render roju (Stop na komputerze, który utworzył zadanie, zachowuje
  każdą gotową część, a każdy komputer zachowuje część, nad którą
  pracował). Oblicz 3D i Monte Carlo z tymi samymi parametrami pytają, czy
  kontynuować, pozostałe kontynuują same; przycisk **Wznów** (zakładka
  Otwórz) wyświetla niedokończone rendery, by je kontynuować albo usunąć.
  W wierszu poleceń to samo polecenie kontynuuje samo (również obraz w
  `--tiles`, `montecarlo`, `animate`, `batch`, `voxel` i `mesh`); `--fresh`
  zaczyna od nowa, `mb3d resume` je wyświetla. Kontynuowany render jest
  taki sam jak policzony za jednym razem.
* **Renderowanie dzielone między Twoimi komputerami**, „rój” (swarm):
  komputery, które widzą jeden wspólny folder, renderują razem obraz, Duży
  render, obraz Monte Carlo lub animację. Zob.
  [Renderowanie na kilku komputerach](#rendering-on-several-computers-the-swarm).
* **Nowa formuła, PseudoKleinian:** „architektura kleinowska” z hal, łuków
  i kul jako jedna formuła z własną dokładną odległością.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="Nowa formuła: PseudoKleinian"></a>

  <details><summary>Więcej</summary>

  Hale, łuki, filary i zagnieżdżone kule znane jako „architektura
  kleinowska” należą do najbardziej lubianych obrazów fraktali 3D. W MB3D
  można je zrobić tylko jako hybrydę kilku formuł o starannie dobranych
  ustawieniach, według przepisu, który Theli-at znalazł w 2011 roku;
  knighty napisał potem to samo jako jedną formułę z własnym oszacowaniem
  odległości i to jest **PseudoKleinian**. Każdy poziom składa punkt do
  pudełka, odwraca go w kuli i dodaje przesunięcie, a odległość jest
  mierzona dokładnie do płaszczyzny podłogi po złożeniach, więc ściany
  wychodzą czyste, a obraz renderuje się szybko. Ustawienia są autorstwa
  knighty'ego: *Levels* (8), *Size* (kula inwersji), *CSize X/Y/Z* (pudełko;
  jego 0.924, 0.908, 0.924 dają klasyczne hale, a małe zmiany dają nowe
  budowle), *Julia X/Y/Z* (przesunięcie na poziom), *Shape* (0 płaszczyzna
  podłogi, 1 płaszczyzna cylindryczna o grubości *Thickness* przez
  *Offset*, 2 zaokrąglone pudełko), *DE offset* (wydrąża powierzchnię) i
  *Colour scale* (z *2. wybór*, orbit trap: najmniejszy promień orbity,
  klasyczne kleinowskie pasy).
  
  Struktura wypełnia całą przestrzeń, więc patrzy się na nią od środka:
  zacznij ze środkiem widoku blisko początku układu i kamerą jedną lub dwie
  jednostki wstecz, a po halach poruszaj się Nawigatorem. W rendererze
  Monte Carlo do wnętrza prawie nie dociera światło nieba, więc umieść w
  hali światło pozycyjne (pierwszy obraz) albo rozetnij strukturę
  płaszczyzną cięcia i zajrzyj z góry (drugi). Działa na karcie graficznej
  i na procesorze, potrzebuje 1 iteracji, działa po transformacjach dIFS i
  w kombinacjach DE jak inne formuły dIFS (choć wewnątrz struktury
  wypełniającej przestrzeń niewiele one zmieniają), a parametry z nią
  otwierają się tylko w tym programie.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="PseudoKleinian z punktu widzenia knighty'ego: hala z łukami i zagnieżdżonymi kulami oświetlona lampą w środku, w rendererze Monte Carlo z mgłą, twardymi cieniami i głębią ostrości"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="PseudoKleinian rozcięty płaszczyzną cięcia i widziany z góry: sieć kul i komórki między nimi"></a>

  </details>
* **Pary stereo w jednym kroku**: obrazy dla obu oczu renderowane i
  pokazywane obok siebie, do oglądania równoległego lub krzyżowego
  (zezem). Wkład Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Para stereo Hydry Dainbramage'a, obok siebie do oglądania równoległego"></a>

  <details><summary>Więcej</summary>

  **Stereo ▸ Renderuj parę stereo** renderuje obraz dla lewego, a potem
  prawego oka z ustawieniami stereo tej zakładki i pokazuje je obok siebie
  w oknie głównym. Są ułożone do oglądania równoległego (lewy obraz po
  lewej); **Podgląd krzyżowy** zamienia je miejscami do oglądania zezem.
  To podgląd: oświetlenia i kolorów nie można potem zmienić bez ponownego
  renderowania i nie działa on z kafelkami. Przyciski pojedynczych obrazów
  dla każdego oka nadal są dostępne.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Para stereo Hydry Dainbramage'a, obok siebie do oglądania równoległego"></a>

  </details>
* **Pozostały czas dla całego obrazu**: pasek pod obrazem wypełnia się
  jeden raz przez całe obliczenie, a obok niego widać czas pozostały do
  ukończenia obrazu, łącznie z cieniami, okluzją i odbiciami.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="Pasek postępu i łączny pozostały czas"></a>

  <details><summary>Więcej</summary>

  Pasek MB3D zaczyna od nowa na każdym etapie. Tutaj pokazuje całe
  obliczenie: główne renderowanie, potem twarde cienie i okluzję
  otoczenia, odbicia i głębię ostrości. Czas etapów po głównym renderowaniu
  jest szacowany na podstawie tego, czego wymaga scena i jak poszło główne
  renderowanie; oszacowanie dopasowano na około 1400 renderach 366 scen.
  Obraz liczony ponownie zakłada czasy z poprzedniego razu.
  
  Pierwszy obraz z nowymi formułami na karcie graficznej czeka na
  skompilowanie shaderów: wiersz pokazuje wtedy *kompilowanie shaderów* z
  pozostałym czasem, który uwzględnia tempo procesora oraz, jeśli shadery
  zdążą, dołączenie karty. Szczegóły w
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="Okno główne podczas liczenia twardych cieni i okluzji otoczenia, z łącznym pozostałym czasem"></a>

  </details>
* **History** (historia) pod ikoną zegara na pasku tytułu: każdy
  policzony obraz z miniaturą, każdy otwarty lub zapisany plik, jako
  lista, jako wiersze lub jako kafelki dowolnej wielkości, pogrupowane
  według dni albo w jednym ciągu; można ją przeszukiwać, sortować,
  pokazywać tylko różniące się obrazy i wczytać dowolny wpis podwójnym
  kliknięciem. Historia z setkami tysięcy wpisów otwiera się od razu.
* **Oświetlenie pozostaje na żywo** po twardych cieniach i odbiciach:
  przesuń światło lub zmień jego kolor, a obraz od razu się zmieni, jak w
  Mandelbulb3D.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="Okno główne i okno Oświetlenie tuż po zmianie oświetlenia"></a>

  <details><summary>Więcej</summary>

  Wyrenderuj obraz bez dodatków, a potem wypróbuj położenia i kolory
  świateł: każda zmiana w oknie Oświetlenie od razu odświeża obraz z
  cieniami takimi, jakie zostały policzone, jak w Mandelbulb3D. Zmiana
  koloru lub natężenia światła nigdy nie liczy cieni ponownie. Cienie
  przesuniętego światła, odbicia i głębia ostrości wracają po użyciu
  przycisków okna Postprocessing, jak w Mandelbulb3D.
  
  Zatrzymane obliczenie można też kontynuować po zmianie oświetlenia.
  Sposób pracy z tym: [Praca z oświetleniem](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-pl)
  w wiki. Szczegóły w
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="Okno główne i okno Oświetlenie tuż po zmianie oświetlenia: obraz już jaśniejszy"></a>

  </details>

<a id="download"></a>

## Pobieranie

Gotowe do uruchomienia programy z najnowszego wydania, bez instalacji:

| System | Pobieranie |
|---|---|
| Windows (64 bit) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bit) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (instaluje się jak inne programy) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (instaluje się jak inne programy) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple silicon (M1 i nowsze) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Rozpakuj archiwum i uruchom **Mandelbulb3D** (`Mandelbulb3D.exe` w Windows)
w rozpakowanym folderze. Folder jest ułożony jak instalacja MB3D, z
formułami i mapami MB3D oraz 80 przykładowymi plikami parametrów.

* Jeśli Windows pokaże „System Windows ochronił ten komputer”, kliknij
  *Więcej informacji* ▸ *Uruchom mimo to*.
* macOS blokuje pobrane programy, które nie są podpisane: uruchom raz
  `xattr -dr com.apple.quarantine .` w rozpakowanym folderze.

**Alternatywa dla Linuksa i macOS:** zamiast pobierać ręcznie, ten
jeden wiersz pobiera najnowsze wydanie do `~/mb3d-rs` (uruchom go
ponownie później, aby zaktualizować; skrypt [download.sh](../../download.sh)
jest też w folderze programu):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Sprawdza pobrany plik przed rozpakowaniem, dodaje **Mandelbulb3D** do menu
aplikacji (Linux) i tworzy polecenia `Mandelbulb3D` i `mb3d`, które można
wpisać w dowolnym folderze. `sh ~/mb3d-rs/download.sh --uninstall` usuwa
z powrotem pozycję menu i polecenia; Twoje pliki zostają.

Programy dla Linuksa działają na dystrybucjach mniej więcej od 2018 roku
(Ubuntu 18.04, Debian 10, Rocky/RHEL 8 i nowsze).

**Instalacja jak innych programów:** w Debianie, Ubuntu i Mincie otwórz
plik `.deb` powyżej (albo `sudo apt install ./mb3d-rs-linux-x64.deb`); w
Fedorze, RHEL i Rocky plik `.rpm` (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, w openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
Mandelbulb3D jest wtedy w menu aplikacji, a `mb3d` działa w dowolnym
folderze; Twoje ustawienia i historia są przechowywane w `~/.config/mb3d-rs`
i `~/.local/share/mb3d-rs`.

[Pierwsze kroki](../../GETTING_STARTED.md) prowadzą przez początek pracy
(po angielsku); starsze wersje i informacje o wydaniach są na stronie
[wydań](https://github.com/thr32423/mb3d-rs/releases).

<a id="guides-and-help"></a>

## Poradniki i pomoc

Poradniki krok po kroku są w [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-pl):

* [Instalacja](https://github.com/thr32423/mb3d-rs/wiki/Installing-pl): Windows, macOS i Linux, aktualizacja, gdzie są Twoje pliki
* [Praca z oświetleniem](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-pl): najpierw zwykły render, potem próby ze światłami, cieniami i odbiciami
* [Konfiguracja roju](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-pl): renderowanie na kilku komputerach przez wspólny folder
* [Rozwiązywanie problemów i FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-pl): gdy coś nie działa tak, jak powinno

Każde wydanie z jego zmianami: [pełna lista](../UPDATES.md).

<a id="rendering-on-several-computers-the-swarm"></a>

## Renderowanie na kilku komputerach (rój)

Komputery, które widzą jeden wspólny folder, renderują razem: obraz w
kafelkach, Duży render, obraz Monte Carlo, animację albo zadanie wsadowe.
Każdy komputer bierze części, których nikt inny nie ma, więc szybki
komputer bierze ich więcej, a komputer, który rozpoczął render, składa
wyniki. Dzieleniem zajmuje się folder: program nie otwiera żadnego
połączenia sieciowego i z nikim się nie komunikuje, więc jest tak
bezpieczny jak Twoje udostępnianie plików.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="Mandelbulb3D z oknem roju, otwartym przyciskiem Swarm na zakładce Narzędzia, gdy dwa komputery renderują obraz w kafelkach: wspólny folder, Udostępnij ten komputer, komputery i to, czym każdy się zajmuje, zadanie oraz podgląd z wyrenderowanymi kafelkami, kafelkiem w trakcie i brakującymi"></a>

<a id="setting-it-up"></a>

### Konfiguracja

Krok po kroku, z tym, co sprawdzić: [Konfiguracja roju](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-pl) w wiki.

1. **Folder, do którego każdy komputer może zapisywać:** udział na NAS-ie,
   udział Windows, udział Samba lub NFS. W Linuksie zamontuj go jako
   folder, na przykład
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   udział otwarty tylko w menedżerze plików (`smb://...`) nie jest
   folderem, którego program może użyć.
2. **Ta sama wersja mb3d-rs na każdym komputerze.** Komputer z inną wersją
   jest wyświetlany na liście, ale nie bierze udziału.
3. **W Mandelbulb3D:** przycisk **Swarm** na zakładce Narzędzia. Wybierz
   wspólny folder (ten sam udział na każdym komputerze; jego ścieżka może
   się różnić), nadaj komputerowi nazwę dla listy i zaznacz **Udostępnij
   ten komputer**.

To wszystko. Wiersz stanu mówi, co się dzieje, na przykład *Udostępnianie
z omarchy: rendery są dzielone między 2 komputery*. Gdy komputer
jest w roju sam, wszystko renderuje się na nim jak zwykle.

<a id="what-is-shared"></a>

### Co jest dzielone

Dopóki *Udostępnij ten komputer* jest zaznaczone, komputer renderuje
części, które inne komputery umieściły w folderze, a jego własne rendery
trafiają do roju, gdy tylko jest w nim inny komputer:

* **Oblicz 3D** (i okno Przetwarzanie wsadowe) — obraz w N x M kafelkach
  (ustawianych w oknie roju), składany tutaj ze wszystkim, co daje zwykłe
  obliczanie: zmiany oświetlenia, postprocessing, zapis jako m3i (i jego
  zapotrzebowanie na pamięć, zob. niżej).
* **Duży render** — kafelki są zapisywane jeden po drugim, w miarę jak
  docierają; cały obraz nigdy nie jest trzymany w pamięci, więc to jest
  sposób na obrazy 100000 x 100000 pikseli i większe.
* **Monte Carlo** — jak na jednym komputerze: każdy komputer dodaje
  przebiegi do własnego bloku, dopóki nie naciśniesz Stop, obraz oczyszcza
  się dzięki promieniom wszystkich, a Stop uśrednia bloki. W trybie
  wsadowym okna bloki razem zatrzymują się na liczbie promieni zadania
  wsadowego.
* **Kreator animacji** — klatki, jedna po drugiej.

Okno pokazuje listę komputerów z ich rdzeniami, kartą graficzną i tym,
czym każdy się zajmuje, zadania z postępem oraz podgląd najnowszego
zadania: wyrenderowane kafelki, te w trakcie z paskiem postępu i te,
których jeszcze brakuje.

<a id="on-the-command-line"></a>

### W wierszu poleceń

Na każdym komputerze, który ma pomagać, uruchom workera na folderze:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Następnie, na dowolnym komputerze, renderuj z `--swarm` i tym samym folderem:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` pokazuje, kto jest w roju, czym zajmuje się każdy
komputer i jak daleko są zadania. Obraz Monte Carlo w wierszu poleceń jest
dzielony na bloki z własnymi losowymi promieniami: 4 bloki po 64 promienie
na piksel dają 256 zamówionych przez `--rays 256`.

<a id="good-to-know"></a>

### Dobrze wiedzieć

* **Jeśli inne komputery pozostają bezczynne**, a ich lista zadań jest
  pusta, podczas gdy ten komputer renderuje: komputer, który udostępnia
  folder przez Sambę i sam z niego korzysta bezpośrednio, pozwala innym
  trzymać starą listę jego zawartości. Wyłącz to buforowanie w
  `/etc/samba/smb.conf`: `smb3 directory leases = no` w `[global]` oraz
  `oplocks = no` i `level2 oplocks = no` w sekcji udziału, potem `sudo
  systemctl restart smb` i ponownie zamontuj udział na pozostałych
  komputerach.
* **Pamięć:** obraz zajmuje w tym programie około 20 bajtów na piksel
  (jego bufor obrazu), 10 więcej w oknie głównym, a do tego większy z
  dwóch etapów: główne obliczenie z cieniami i okluzją (16 na piksel i
  1,2 GB dla karty) albo malowanie (3 na piksel, 20 więcej z głębią
  ostrości i jeden pas odbić: najwyżej 3,5 GB, mniej, gdy wolnej pamięci
  jest mało, bo pas ma połowę tego, co jest wolne na początku przebiegu).
  Obraz 12000 x 6000: 6 GB. Gdy obraz potrzebuje więcej niż wolna pamięć
  komputera (pobierana z Linuksa, Windows i macOS), Oblicz 3D podaje, ile
  potrzebuje i ile jest wolne, i pyta przed rozpoczęciem: obliczenie
  korzystałoby z pliku wymiany, a program mógłby przestać reagować na
  wiele minut. Wyjściem jest Duży render, sam lub z rojem: jego kafelki są
  zapisywane jeden po drugim, cały obraz nigdy nie jest trzymany. Oblicz 3D
  z rojem trzyma obraz tutaj tak jak zwykłe obliczanie (bez etapu głównego
  obliczenia, który komputery kafelków wykonują z pamięcią potrzebną na
  każdy kafelek); komputer roju zostawia innym kafelek zbyt duży dla jego
  pamięci.
* **Bezpieczeństwo:** komputer renderuje tylko to, o co prosi zadanie, i
  zapisuje tylko do folderu zadania; kod formuł z zadania działa w
  emulatorze formuł, jak każda formuła.
* **Szybkość:** kafelki są przesyłane jako pełne bufory obrazu, kilkaset
  megabajtów dla bardzo dużego obrazu, więc szybki udział się opłaca.
  Usługa synchronizacji kopiująca z opóźnieniem też działa, ale wolniej.
* **Komputery, które znikają:** część, którą renderował komputer, przejmuje
  inny, gdy przez pół minuty nie ma od niego wieści. Zadanie, którego
  właściciel zniknął na pół godziny, usuwa ten, kto je zauważy. Zadanie,
  które nie powiedzie się na jakimś komputerze, jest odnotowane w oknie i
  pozostawione innym.
* **Zatrzymywanie:** odznaczenie *Udostępnij ten komputer* pozwala
  najpierw dokończyć część w trakcie; zamknięcie programu ją porzuca, a
  przejmuje ją inny komputer.
* **Zatrzymanie zadania** (Stop na komputerze, który je utworzył, jego
  zamknięcie albo awaria): każda już gotowa część zostaje na tym
  komputerze, a każdy komputer zachowuje część, którą renderował. Ponowne
  uruchomienie tego samego renderu tworzy zadanie tylko z brakujących
  części, a komputer, który dostanie z powrotem rozpoczętą przez siebie
  część, kontynuuje ją. To samo działa w wierszu poleceń: uruchom ponownie
  to samo polecenie. Otwarty render Monte Carlo (z okna) składa swoje
  bloki po zatrzymaniu, jak dotąd.

<a id="how-the-rendering-works"></a>

## Jak działa renderowanie

Obraz fraktala jest liczony piksel po pikselu: dla każdego piksela promień
biegnie w głąb sceny, aż trafi w powierzchnię fraktala, a na każdym kroku
formuła jest obliczana dziesiątki razy. MB3D robi to na procesorze, na
wszystkich jego rdzeniach. mb3d-rs potrafi to samo, a może też oddać pracę
**karcie graficznej**, która ma tysiące małych procesorów i jest
zbudowana właśnie do takich zadań.

Ustawienie **„Obliczaj na”** (zakładka Wewnętrzne okna głównego) wybiera:

| | |
|---|---|
| **Auto** (domyślnie) | Karta i procesor dzielą się obrazem: karta bierze wiersze od góry, wątki procesora od dołu, aż się spotkają. Cienie i okluzja otoczenia są dzielone tak samo albo liczone przez samą kartę, gdy jest wyraźnie szybsza. |
| **CPU** | Tylko procesor, jak w MB3D. |
| **Karta graficzna** | Tylko karta (czeka, aż jej program będzie gotowy). |

Gdy jakaś kombinacja formuł jest używana po raz pierwszy, sterownik
graficzny musi przygotować program dla karty — *shader* — co na
komputerze z pomiarów poniżej trwa od 10 do 15 sekund. W trybie Auto
procesor po prostu zaczyna bez niego, a karta dołącza, gdy jest gotowa,
więc mały obraz nigdy nie czeka. Przygotowany shader jest zachowywany na
następny raz (przez sterownik graficzny, w Linuksie w folderze
`ShaderCache` obok programu), więc kolejny obraz z tymi samymi formułami
od razu startuje na karcie.

**Dioda** na prawo od tytułu okna mówi, gdzie policzono ostatni obraz:
zielona na karcie graficznej, pomarańczowa częściowo, czerwona na
procesorze. Wskazanie jej myszą pokazuje szczegóły.

[Jak działa renderowanie](../RENDERING.md) opowiada całą historię z
obrazkami (po angielsku): co robi Auto za pierwszym razem, dlaczego karta
nie zawsze jest szybszym wyborem i co oznaczają kolory diody.

Nie wszystko może trafić na kartę: kilku formuł nie da się dla niej
przetłumaczyć, a widoki bardzo głęboko przybliżone są liczone w podwójnej
precyzji (co potrafią tylko niektóre karty). Program decyduje o tym
wszystkim sam; widać to tylko po diodzie.

<a id="speed"></a>

## Szybkość

Wszystkie pomiary na jednym komputerze: AMD Ryzen 9 3900X (12 rdzeni, 24
wątki) z NVIDIA GeForce RTX 2060, w Linuksie. Oryginalny
**Mandelbulb3D 1.99sr37** był uruchamiany na tej samej maszynie pod Wine
(jego program dla Windows działa tam bez zmian, z pełną szybkością, na
wszystkich rdzeniach), a czas mierzono od kliknięcia *Oblicz 3D* do
gotowego obrazu. Każdy plik parametrów był renderowany w tym samym
rozmiarze obrazu przez wszystkie trzy: MB3D, mb3d-rs tylko na procesorze
(**CPU**) i mb3d-rs w trybie **Auto** (karta graficzna i procesor razem).
Czasy Auto pochodzą z drugiego renderu pliku, z już przygotowanym
shaderem karty; pierwszy render nowej kombinacji formuł dodaje 10–15 s
przygotowania shadera na tej karcie.

<a id="mb3ds-80-example-parameter-files"></a>

### 80 przykładowych plików parametrów MB3D

Pliki przykładowe dołączone do MB3D (i w folderze `M3Parameter` tego
programu), każdy w swoim własnym rozmiarze obrazu.

MB3D nie potrafił policzyć jednego z plików (*Test8*: zatrzymuje się z błędem „Division by zero”), więc porównania obejmują 79 plików.

* **mb3d-rs na samym procesorze jest szybszy od MB3D** w 71 z 79 plików, w medianie **1,7 raza**; wszystkie pliki razem zajęły MB3D 18 min 21 s, a mb3d-rs na procesorze 17 min 10 s.
* **Tryb Auto** jest szybszy od MB3D w 77 z 79 plików (mediana 2,1 raza); wszystkie pliki razem 8 min 17 s. W 30 plikach procesor skończył sam, głównie dlatego, że potrzebuje tylko kilku sekund, zanim karta dojdzie do głosu — tam Auto to po prostu procesor plus chwila narzutu. Karta dołączyła w 49 plikach, tych wolniejszych, gdzie przyspieszyła obraz w medianie 1,6 raza względem procesora (do 13 razy: *Insect Menger*, 4,7 s zamiast 63,0 s).
* W 4 plikach Auto był wyraźnie wolniejszy niż sam procesor (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): tam karta jest dla danej sceny wolniejsza niż 24 wątki procesora.
* „Wiersze na karcie” to część obrazu, którą karta graficzna policzyła w trybie Auto (– gdy procesor skończył sam).

<details>
<summary>Wszystkie 80 plików (kliknij, aby otworzyć)</summary>

| Plik parametrów | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | wiersze na karcie | Auto wobec MB3D |
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
| Test8 | błąd w MB3D | 0.1 s | 0.3 s |  | – |
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

Obrazy są takie same: mb3d-rs na procesorze tworzy obraz MB3D z
dokładnością do zaokrągleń, a obrazy Auto różnią się od obrazów z
procesora średnio o ułamek procenta zakresu kolorów (drobne szczegóły
mogą błyszczeć w innych pikselach; [notatki techniczne](../TECHNICAL.md)
zawierają pomiary).

<a id="the-command-line"></a>

## Wiersz poleceń

`mb3d` renderuje bez okna. Otwórz terminal w folderze programu (w Windows
wiersz polecenia w rozpakowanym folderze, i wpisuj `mb3d` zamiast
`./mb3d`). Po jednowierszowej instalacji w Linuksie lub macOS `mb3d`
działa w dowolnym folderze, `man mb3d` pokazuje jego podręcznik, a klawisz
Tab uzupełnia jego opcje w bash, zsh i fish:

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

Wszystko inne ma podpolecenie z własnym `--help`:

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

Formuły i mapy są wyszukiwane obok programu; te z innej instalacji MB3D
można podać przez `--formulas` i `--maps`. Animacje są animacjami MB3D:
klatki kluczowe z interpolacją MB3D, projekty `.m3a` otwierane i
zapisywane, klatki zapisywane jak w MB3D, tak że przerwana animacja
kontynuuje się, a kilka komputerów może dzielić jedną.

<a id="good-to-know-1"></a>

## Dobrze wiedzieć

* **Te same obrazy, nie identyczne co do bitu.** MB3D liczył w 80-bitowej
  precyzji dawnej arytmetyki x87; ten program używa dzisiejszej 64-bitowej.
  Obrazy zgadzają się w każdej strukturze, ale pojedyncze piksele mogą
  różnić się ostatnią cyfrą koloru. Rendery Monte Carlo i MutaGen używają
  własnych liczb losowych, więc ich wyniki nie są wynikami MB3D.
* **Obrazy z karty graficznej** nieco różnią się od obrazów z procesora
  (pojedyncza precyzja); w bardzo drobnych szczegółach błyski mogą
  wypaść w innych pikselach. Głębokie przybliżenia same przełączają się
  na podwójną precyzję.
* **Szybkość na procesorze** jest zbliżona do MB3D: kod maszynowy formuł
  z plików `.m3f` MB3D został przetłumaczony tak, by działał na każdym
  procesorze (również Apple Silicon), niewielkim kosztem; sam ray marching
  jest szybszy. Zob. tabele powyżej. W Linuksie i Windows formuły hybrydy
  sceny są też kompilowane razem w jedną funkcję przy starcie obrazu (w
  kilka milisekund, przez Cranelift), tak by przekazywały sobie wartości w
  rejestrach procesora. Razem z szybszą pętlą po formułach (w każdym
  systemie) rendery CPU 80 plików przykładowych zajmują o 20 % mniej czasu
  niż w v0.2.74 (w niektórych do 60 %), a obrazy są identyczne co do bitu.
  Scena zachowuje skompilowaną hybrydę tylko wtedy, gdy pomiar wykaże, że
  jest szybsza; `MB3D_NO_JIT=1` ją wyłącza. W macOS też jest włączona, ale
  wciąż tam testowana: `./jit_check.sh` w rozpakowanym folderze ją
  sprawdza (porównuje skompilowane formuły z przetłumaczonymi i renderuje
  pliki przykładowe z nią i bez niej; obrazy muszą być takie same).
* **Duże obrazy** można renderować w jednym kawałku (obraz 15000 x 20000
  potrzebuje około 15 GB pamięci; z głębią ostrości trwa to godziny) albo
  w kafelkach, także na kilku komputerach (`--tiles` i okno Duże rendery),
  co wymaga mniej pamięci. Oblicz 3D nie rozpoczyna obrazu, który nie
  mieści się w wolnej pamięci (w Linuksie, gdzie da się ustalić wolną
  pamięć), i to komunikuje, a komputer roju zostawia innym kafelek zbyt
  duży dla jego pamięci.
* **Niedokończone rendery** są przechowywane w folderze pamięci podręcznej
  (`~/.cache/mb3d-rs/resume`, w Windows `%LOCALAPPDATA%\mb3d-rs\resume`,
  albo obok programu): 18 bajtów na piksel dla obrazu (obraz 15360 x 15360
  zajmuje 4,2 GB), 64 dla renderu Monte Carlo. To, co policzono, jest
  zapisywane od razu i co 5 minut zrzucane na dysk. Render zatrzymany w
  ciągu pierwszych 10 sekund nic nie zachowuje; niedokończone rendery
  starsze niż 30 dni są usuwane, a najstarsze także wtedy, gdy wszystkie
  razem zajmują ponad 50 GB. Render kontynuuje się tylko z tymi samymi
  parametrami i tą samą wersją programu (i tym samym wyborem karty lub
  procesora; ekspozycja i nasycenie renderu Monte Carlo mogą się zmienić).
  Render Monte Carlo pozostaje niedokończony, dopóki nie osiągnie swojej
  liczby promieni (zadanie wsadowe, `--rays`, `--passes`) albo nie
  zostanie usunięty z listy Wznów.
* **Nowsze lub własne formuły, mapy, motywy i języki** trafiają do
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` i
  `.../Languages` (przy każdym sposobie instalacji; w Windows
  `%APPDATA%\mb3d-rs\...`). Są używane obok tych dołączonych do programu i
  zastępują dołączony plik o tej samej nazwie, więc zaktualizowana formuła
  z forum działa bez ruszania folderu programu.

<a id="updates"></a>

## Aktualizacje

Każde wydanie z jego zmianami jest opisane w [UPDATES.md](../UPDATES.md) (po angielsku).

<a id="for-developers"></a>

## Dla programistów

[Notatki techniczne](../TECHNICAL.md) (po angielsku) opisują, jak używana
jest karta graficzna, co i jak przeniesiono ze źródeł MB3D w Delphi, trzy
sposoby wykonywania kodu maszynowego formuł MB3D, format pliku sceny i
jego klucze, porównania z MB3D oraz budowanie ze źródeł:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## Licencja

MB3D jest objęty licencją LGPL 2.1. Ten port jest utworem zależnym i
używa tej samej licencji.

<a id="credits"></a>

## Podziękowania

Osoby i projekty, na których opiera się mb3d-rs, są wymienione w [angielskim README](../../README.md#credits).
