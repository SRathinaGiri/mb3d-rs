<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Mandelbulb3D para Linux, Windows e macOS">

# mb3d-rs — Mandelbulb3D para Linux, Windows e macOS
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Français](README.fr.md) · [Bahasa Indonesia](README.id.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · **Português** · [Kaszëbsczi](README.csb.md) · [العربية](README.ar.md) · [اردو](README.ur.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · [한국어](README.ko.md) · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*Traduzido do README em inglês (versão 0.2.86). Onde os dois divergirem, vale o [README em inglês](../../README.md).*

**Downloads: [a versão mais recente](https://github.com/thr32423/mb3d-rs/releases/latest)** · [Download](#download) · [Novidades](../UPDATES.md)

O mb3d-rs é o [Mandelbulb3D](https://github.com/thargor6/mb3d) (MB3D), o
programa de fractais 3D, reescrito para rodar nos computadores de hoje: no
**Linux, Windows e macOS**, e com a imagem calculada na **placa de vídeo**,
onde o MB3D só podia usar o processador. Ele abre os arquivos de parâmetros
do MB3D (`.m3p`, `.m3i`, parâmetros em texto de fóruns), usa as fórmulas e
os mapas do MB3D e produz as mesmas imagens.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="A janela principal"></a>

Ele vem em dois programas:

* **Mandelbulb3D** — o programa de desktop, com as próprias janelas do MB3D:
  a janela principal, Fórmulas, Iluminação, Pós-processamento, Navegador,
  Animação, Lote, Renders grandes, Monte Carlo, Exportar voxels, Bulb
  Tracer 2, MutaGen e as demais. Quem conhece o MB3D conhece este programa;
  os tutoriais do MB3D continuam valendo.

  <details><summary>Saiba mais</summary>

  As janelas são as do próprio MB3D — feitas a partir das definições de
  janela originais do MB3D, então cada botão, controle deslizante e dica
  está onde está no MB3D:
  
  * **Janela principal:** abrir e salvar parâmetros (`.m3p`, `.m3i`, texto,
    a área de transferência), salvar imagens (PNG, JPEG, BMP, Z-buffer de
    16 bits), os modos do mouse, os botões de rotação, o tamanho da imagem,
    as predefinições de qualidade, desfazer, *Calcular 3D* com a imagem
    crescendo e as estatísticas, e todas as páginas (Cálculo, Interno,
    Infos, Corte, Julia, Câmera, Coloração, Estéreo).
  * **Fórmulas:** os seis espaços de fórmula, as listas e opções de fórmulas
    do MB3D, os tipos de híbrido, os favoritos e o editor de fórmulas JIT.
  * **Iluminação:** as luzes 1–6, as predefinições, a paleta, as páginas
    Objeto, Ambiente, n.din. e Imagem fundo; as mudanças repintam a imagem
    na hora, sem um novo cálculo.
  * **Pós-processamento:** normais no z-buffer, sombras duras e de ambiente,
    reflexos e transparência, profundidade de campo.
  * **Navegador:** ande e olhe em volta com o teclado e o mouse, depois
    *Vista ao principal*.
  * **Criador de animações, Processamento em lote, Renders grandes, Monte
    Carlo, Exportar voxels, Bulb Tracer 2** (malhas) com a prévia da malha,
    o **Gerador de mapa de altura, MutaGen, ZBuf16Bit, Sequências de mapas,
    Pastas Ini** — tudo como no MB3D.
  * **Enxame** (novo, na aba Ferramentas): seus computadores renderizam
    juntos, veja [Renderizar em vários computadores](#rendering-on-several-computers-the-swarm).

  </details>
* **mb3d** — um renderizador de linha de comando para scripts e trabalhos
  em lote: imagens, animações, listas de lote, renders Monte Carlo,
  exportações de voxels e de malhas, tudo sem janela.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## É seguro rodar? Confira você mesmo

Você não precisa acreditar na nossa palavra. O [scanrepo.dev](https://www.scanrepo.dev)
é um serviço independente que examina um repositório do GitHub em busca de
sinais de software nocivo: código que inicia outros programas, se conecta à
internet, esconde o que faz ou contorna as verificações de segurança da
linguagem de programação. Qualquer pessoa pode rodá-lo neste repositório a
qualquer momento:

**[▶ Analisar o mb3d-rs no scanrepo.dev](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(clique em *Rescan* para um resultado novo do código atual)

A versão 0.2.85 foi classificada como **segura: 5 de 100** (analisada em 30
de outubro de 2026, regras do mecanismo 10 do analisador). Seu único aviso é
a linha que inicia as fórmulas que o programa compila em tempo de execução
para ganhar velocidade: executar código de máquina recém-gerado não pode ser
escrito na forma verificada do Rust, e essa linha e a ponte para o filtro de
ruído da Intel são os únicos lugares onde o programa permite isso; o resto é
uma nota informativa. (Antes: a 0.2.66 foi classificada com 0 de 100 por um
mecanismo mais antigo, veja também [o resultado da 0.2.44](../img/scan_0.2.44.png).)
Uma versão anterior foi sinalizada, e mudamos o código em resposta
([issue #4](https://github.com/thr32423/mb3d-rs/issues/4)); agora ele é
escrito para continuar assim. Os programas não contêm nenhum código que se
conecte à internet; o único outro programa que eles pedem ao sistema para
abrir é o seu gerenciador de arquivos, quando você clica num botão para
abrir uma pasta.

Uma análise procura sinais de alerta conhecidos; não é uma garantia. Estes
pontos permitem que você confira mais por conta própria:

* **Todo o código-fonte está aqui.** Os downloads são compilados a partir
  dele pelos próprios servidores de compilação do GitHub, pelos passos que
  você pode ler [no workflow de release](../../.github/workflows/release.yml),
  e não no computador pessoal de ninguém.
* **Cada versão lista somas de verificação** (`SHA256SUMS`), para você
  confirmar que o arquivo baixado é exatamente o que foi publicado; a
  instalação em uma linha mais abaixo faz isso por você. As versões a partir
  da 0.2.45 também trazem os atestados de compilação do GitHub: `gh attestation verify
  FILE --repo thr32423/mb3d-rs` mostra a partir de qual commit um download
  foi compilado.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## O que há de novo em relação ao Mandelbulb3D

* **A placa de vídeo calcula a imagem** — a imagem em si, as sombras, a
  oclusão de ambiente, os reflexos e os renders Monte Carlo. Num computador
  típico isso é várias vezes mais rápido do que só o processador; veja
  [Velocidade](#speed) abaixo.
* **Modo Auto**: a placa de vídeo e o processador trabalham juntos em cada
  imagem, e nada fica esperando — se a placa não estiver pronta, o
  processador começa assim mesmo e a placa entra depois.
* **Um LED na barra de título** mostra onde a última imagem foi calculada.
* **Tamanho da interface** de 75 a 200 % para telas de alta resolução.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="Temas visuais a 150 %"></a>

  <details><summary>Saiba mais</summary>

  **Prefs ▸ Temas ▸ Tamanho da interface** deixa todas as janelas e seus
  textos maiores ou menores (75 a 200 %), para telas de alta resolução; os
  layouts continuam exatamente como no MB3D.
  
  | 100 % | 150 % |
  |---|---|
  | ![Temas visuais a 100 %](../img/themes_100.png) | ![Temas visuais a 150 %](../img/themes_150.png) |

  </details>
* **Oito aparências** para as janelas, todas arquivos de tema que você pode
  copiar e modificar: duas reproduzem as aparências do MB3D, seis são novas.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="O tema Midnight"></a>

  <details><summary>Saiba mais</summary>

  **Prefs ▸ Temas** oferece oito aparências, cada uma um arquivo na pasta
  `Themes`: *Glossy* (o padrão escuro) e *Windows* reproduzem as duas
  aparências do próprio MB3D, as outras seis são novas. Clique numa imagem
  para vê-la em tamanho real.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: o padrão do MB3D | **Windows**: o "Desativar temas" do MB3D |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: cinza neutro mais claro, detalhes azul-aço | **Daylight**: cinza claro quente, detalhes verde-azulados |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: azul-marinho profundo, detalhes azuis | **Ember**: carvão quente, detalhes laranja |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="High contrast"></a> |
  | **Forest**: cinza-esverdeado escuro, detalhes verdes | **High contrast**: preto, texto branco, destaques amarelos |
  
  Um tema é um pequeno arquivo de texto na pasta `Themes` ao lado do
  programa; copie um, mude as cores, e ele aparece na lista na próxima
  inicialização.

  </details>
* **Uma falha deixa um registro em todos os sistemas**: um pânico do
  programa (ou o erro com que ele termina) é acrescentado a `mb3d_crash.txt`,
  ao lado do programa ou na sua pasta de dados (antes, só no Windows).
  Envie esse arquivo junto com um relato de bug.
* **Todas as placas de vídeo do computador calculam**, sem nada a
  configurar: cada placa encontrada pega faixas de linhas da imagem do mesmo
  conjunto que a primeira placa e o processador, então uma placa mais rápida
  simplesmente pega mais; as sombras, a oclusão, os raios Monte Carlo e os
  reflexos são divididos da mesma forma, e uma animação renderiza um quadro
  por placa ao mesmo tempo. Uma placa lenta (gráficos integrados ao lado de
  uma placa dedicada) faz o que pode e nunca atrasa uma imagem: quando
  acabam as linhas livres, as placas ociosas e o processador assumem as
  linhas que ela ainda não entregou. Uma placa que falha é deixada de fora,
  as outras continuam. `mb3d --list-cards` mostra o que foi encontrado.
  (Testado com uma placa aberta duas vezes e com um driver Vulkan em
  software como segunda placa; ainda não num computador com duas placas
  reais.)
* **Uma imagem inacabada é um arquivo `.m3i`.** O que um render parado,
  que travou ou foi encerrado à força calculou fica guardado no cache (a
  janela Renders inacabados, `mb3d resume`); para uma imagem, esse arquivo
  agora é a todo momento um arquivo de imagem `.m3i`, `image.m3i` na pasta
  do render, com as linhas ainda não calculadas como fundo. Copie-o quando
  quiser, ou clique em *Salvar m3i* na janela Renders inacabados (`mb3d
  resume --m3i KEY file.m3i` na linha de comando; ambos também gravam o
  `data.bin` de uma versão anterior como `.m3i`), e abra-o aqui ou no
  Mandelbulb3D, sem conversão. (Os renders Monte Carlo guardam seus
  próprios registros; o `.m3c` de uma passada é gravado com `--m3c`.)
* **Atalhos para as janelas**: uma combinação de teclas traz para a frente,
  a partir de qualquer janela, a janela principal, o Navegador, a janela
  Animação, Fórmulas, Iluminação, Pós-processamento ou Monte Carlo (Ctrl+1 a
  Ctrl+7), e Ctrl+W fecha a janela em que você está (no macOS, Cmd). Defina
  os seus em *Prefs > Atalhos*: clique num campo e
  pressione as teclas; Backspace remove um atalho.
* **A interface no seu idioma**: todo texto das janelas vem de um arquivo
  de idioma; doze idiomas acompanham o programa, e ele grava o arquivo de um
  novo idioma para você traduzir.

  <details><summary>Saiba mais</summary>

  O selo de idioma na ponta direita da barra de título da janela principal
  mostra o idioma da interface (`EN`); um clique abre a janela de idioma.
  Um idioma é um arquivo de texto na pasta `Languages` ao lado do programa.
  Incluídos: English, Deutsch, Français, Italiano, Nederlands, Polski,
  Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文 e 繁體中文 (cada um com sua bandeira;
  os do leste asiático trazem um subconjunto da Noto Sans CJK para os seus
  caracteres, o hindi traz a Noto Sans Devanagari, cujas letras ligadas e
  sinais de vogal são compostos com o modelador de texto rustybuzz).
  As traduções foram feitas por máquina e verificadas quanto à estrutura,
  não por falantes nativos: correções são bem-vindas, cada uma é uma linha
  do arquivo. Para traduzir: digite o nome do seu idioma e
  clique em *Criar arquivo de idioma*; o programa grava `Languages/<name>.lng`
  com todos os textos em inglês, um por linha (`Button2.Caption = Calculate
  3D`). Traduza os textos à direita do `=` num editor de texto e clique em
  *Recarregar*: as janelas acompanham na hora, sem reiniciar. Uma linha
  omitida mantém o texto em inglês. *Salvar + sair* mantém o idioma para a
  próxima inicialização.

  </details>
* **Linux, Windows e macOS** (Intel e Apple Silicon).
* **Um programa de linha de comando** para tudo o que não precisa de janela.
* **Imagens muito grandes de uma vez só**: 15000 x 20000 pixels e mais sem
  esgotar a memória, e a profundidade de campo usa todos os núcleos do
  processador (11 vezes mais rápido em 24 threads do que antes da 0.2.45,
  com a mesma imagem).
* **Imagens de 16 bits:** arquivos PNG com 65.536 em vez de 256 níveis por
  cor, para editar sem faixas.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="Imagens de 16 bits"></a>

  <details><summary>Saiba mais</summary>

  O MB3D salva imagens de 8 bits: 256 níveis por cor. Isso basta para
  olhar, mas quando uma imagem é editada depois — brilho, contraste,
  curvas — áreas suaves como o fundo ou a névoa se quebram em faixas
  visíveis. **Salvar img. ▸ PNG** oferece um segundo tipo de arquivo, **PNG
  16 bits em cores**: as cores como o programa as calculou, antes de serem
  arredondadas para 256 níveis, com todas as sombras, reflexos e
  profundidade de campo. O arquivo tem a mesma aparência e continua suave
  quando é editado.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="Um gradiente de fundo após um forte aumento de contraste: degraus em 8 bits, suave em 16 bits"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="A caixa de diálogo de salvar com 'PNG 16 bits em cores'"></a>

  </details>
* **Um filtro de ruído** para Monte Carlo e todas as outras imagens, o
  DCT denoising conhecido do GIMP, e para renders Monte Carlo também um
  filtro neural, o Intel Open Image Denoise.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="Filtro de ruído"></a>

  <details><summary>Saiba mais</summary>

  Um **filtro de ruído** remove o granulado das imagens: o **DCT denoising**
  de *Filters ▸ Enhance ▸ Noise Reduction ▸ Denoise DCT* do GIMP, o filtro
  do GIMP que melhor se adequou a estas imagens (veja os créditos abaixo).
  
  * **Janela Monte Carlo ▸ -ruído** (um botão que o MB3D mantinha oculto):
    filtra a imagem mostrada, e também a imagem salva, com uma intensidade que acompanha
    os raios por pixel. A renderização em si continua sem filtro, e a
    próxima atualização a mostra de novo sem filtro.
  * **Pós-processamento ▸ Filtro de ruído**: *Filtrar agora* filtra a imagem
    da janela principal; *Filtrar novas imagens automaticamente* filtra cada
    imagem nova. *Intensidade* é a intensidade do ruído a remover (4 é
    leve; valores maiores removem mais granulado e mais detalhes finos).
  
  As imagens Monte Carlo são as que têm mais granulado. Comparadas com
  imagens renderizadas com 512 raios por pixel, as imagens filtradas ficaram
  mais próximas do que as não filtradas em todos os 13 testes (quatro
  cenas, 4 a 32 raios): até 23 % com 4 raios, 5 a 10 % com 8 raios, 1 a 3 %
  com 32 raios, quando resta pouco granulado.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="Uma imagem Monte Carlo com 4 raios por pixel, a mesma com o filtro de ruído, e com 512 raios"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="Pós-processamento: a seção Filtro de ruído"></a>

  O **filtro de ruído neural** é o
  [Intel Open Image Denoise](https://www.openimagedenoise.org), o filtro do
  Cycles do Blender e de muitos outros path tracers, treinado com imagens
  renderizadas. Ele filtra as médias dos raios da renderização (antes da
  exposição e da saturação), então a imagem mantém o seu brilho. A
  biblioteca da Intel vem com o programa (a pasta `oidn`; no .deb e no
  .rpm, `/usr/lib/mb3d-rs/oidn`); `MB3D_OIDN` pode indicar outra.

  * **Janela Monte Carlo ▸ -ruído IA**: filtra a imagem mostrada, e
    também a imagem salva, como faz o *-ruído*; a renderização continua sem filtro.
  * **Linha de comando**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * Leva de 0,2 a 0,6 segundo para uma imagem de 640 x 480 no processador.

  A diferença em relação à imagem de 512 raios (média sobre todos os
  pixels, menor é melhor):

  | Cena | 4 raios | filtro DCT | filtro neural | 8 raios | filtro DCT | filtro neural |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  O filtro neural é o mais próximo em três das quatro cenas; no granulado
  fino do conjunto de Julia, o filtro DCT fica um pouco mais próximo. Ele
  se destaca mais em vidro e espelhos, onde poucos raios deixam mais
  granulado:

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="Uma imagem Monte Carlo com 4 raios por pixel, com o filtro DCT, com o filtro neural, e com 512 raios"></a>

  </details>
* **Fundos transparentes** para imagens e animações.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="Fundo transparente"></a>

  <details><summary>Saiba mais</summary>

  Imagens e quadros de animação podem ser salvos com **fundo
  transparente**, para colocar o fractal sobre outra imagem ou num editor
  de vídeo: **Salvar img. ▸ PNG ▸ PNG com fundo transparente**, e no
  Criador de animações **Fundo transparente** (quadros PNG). Com
  anti-aliasing (uma escala de visualização de 1:2 ou mais, ou `--aa`) as
  bordas ficam suaves e não carregam nenhuma franja da antiga cor de fundo.
  Imagens de profundidade para composição já existiam (*Salvar também o Z
  buffer*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="Uma imagem com o seu fundo, e com fundo transparente mostrada sobre um tabuleiro xadrez"></a>

  </details>
* **Vídeos de animações:** um arquivo AVI direto do programa, ou o comando
  do ffmpeg para um MP4 pequeno.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="Vídeo de uma animação"></a>

  <details><summary>Saiba mais</summary>

  No Criador de animações, **Fazer também um vídeo (AVI)** junta os quadros
  num vídeo quando eles são renderizados, nos **Quadros por segundo** que
  você definir: `<project name>.avi` na pasta de saída, ao lado dos
  quadros, que são mantidos. Ele roda no VLC e no mpv e abre em editores de
  vídeo (Kdenlive, Shotcut, DaVinci Resolve). Cada quadro é guardado como
  uma imagem inteira, então os arquivos são grandes: bons para editar e
  guardar, menos para compartilhar.
  
  Para um vídeo pequeno para compartilhar, **Comando FFmpeg** coloca o
  comando para um MP4 (ou, com fundo transparente, um WebM) na área de
  transferência e nas mensagens. Instale o [ffmpeg](https://ffmpeg.org),
  cole o comando num terminal quando os quadros estiverem prontos, e ele
  faz o vídeo; este programa não inicia nenhum outro programa por conta
  própria.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="O Criador de animações com Fazer também um vídeo (AVI), Quadros por segundo e Comando FFmpeg"></a>

  </details>
* **Uma prévia dos arquivos de parâmetros** na caixa de diálogo Abrir.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="Prévia ao abrir parâmetros"></a>

  <details><summary>Saiba mais</summary>

  **Abrir ▸ Abrir m3p** e **Abrir m3i** mostram uma imagem pequena do
  arquivo de parâmetros clicado, com o seu tamanho e as suas fórmulas, antes
  de ele ser aberto; as setas do teclado percorrem a lista com a prévia. A
  imagem é calculada em segundo plano no processador, então a caixa de
  diálogo continua respondendo, e a prévia de cada arquivo é guardada
  enquanto o programa estiver aberto.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="A caixa de diálogo Abrir com a prévia de um arquivo de parâmetros"></a>

  </details>
* **Navegador: valores exatos e Redefinir vista.** Clique num valor e
  digite-o; volte à vista com que o Navegador começou.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="Navegador: valores exatos e Redefinir vista"></a>

  <details><summary>Saiba mais</summary>

  * **Clique num valor** nos painéis do Navegador — valores de Julia,
    valores das fórmulas, rotação 4D, os valores diversos — e digite o
    número exato, como no MB3D (ângulos em graus). O controle deslizante
    então continua a partir desse valor.
  * **Clique no zoom** e digite-o, quando os passos de zoom forem grossos
    demais (um pedido da lista de issues do MB3D); um zoom digitado
    permanece (*zoom fixo*).
  * **Redefinir vista** volta à posição, à direção e ao zoom que o
    Navegador recebeu da janela principal; os valores ajustados permanecem.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="O Navegador com os valores das fórmulas e a caixa de entrada de um valor clicado"></a>

  </details>
* **Uma nova fórmula, SphereTree:** uma árvore de esferas que se tocam e se
  ramificam em esferas cada vez menores, de Tom Lowe.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="Uma nova fórmula: SphereTree"></a>

  <details><summary>Saiba mais</summary>

  **SphereTree** é uma fórmula que o Mandelbulb3D não tem: uma árvore de
  esferas que se tocam e se ramificam em esferas cada vez menores, como
  espuma ou coral. Tom Lowe, que inventou o Mandelbox, a publicou em 2020
  (veja os créditos abaixo). Ela está na lista de fórmulas junto com as
  fórmulas dIFS e tem quatro parâmetros:
  
  * **PackRatio** (0 a 1,22): o tamanho das esferas; em 1 elas apenas se
    tocam, acima de 1 as externas se soltam.
  * **Levels**: a profundidade da árvore (6 basta para a maioria das
    imagens).
  * **Rotated**: 1 gira cada nível em um doze avos de volta, e a árvore vira
    uma bola redonda de aglomerados de esferas.
  * **Colour scale**: cada ramo da árvore tem a sua própria cor. Escolha
    *2ª escolha* (o orbit trap) nas configurações de cor da Iluminação para
    vê-las; este parâmetro as distribui pelo gradiente de cores.
  
  Ela funciona sozinha e depois de transformações dIFS (a terceira imagem a
  repete com *tilingbrickIFS*), e é calculada tanto na placa de vídeo quanto
  no processador. Defina as iterações como 1: a fórmula calcula a árvore
  inteira de uma vez. Parâmetros com SphereTree só abrem neste programa,
  não no Mandelbulb3D.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="Uma bola SphereTree (Rotated) no renderizador Monte Carlo, ao entardecer"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="Um close da espuma de esferas da SphereTree"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="SphereTree repetida com tilingbrickIFS: um enxame de árvores de esferas"></a> |
  |:---:|:---:|

  </details>
* **Uma nova fórmula, DomeTree:** uma cúpula da qual brota um anel de
  cúpulas menores, cada uma com o seu próprio anel, de Tom Lowe.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="Uma nova fórmula: DomeTree"></a>

  <details><summary>Saiba mais</summary>

  **DomeTree** é a segunda árvore de Tom Lowe depois da SphereTree (veja os
  créditos): uma bola de cujo lado cresce um anel de cúpulas, cada cúpula
  com o seu próprio anel menor, nível após nível, feita de inversões em
  círculos. Ela está na lista de fórmulas junto com as fórmulas dIFS e tem
  cinco parâmetros:

  * **Levels**: a profundidade da árvore (16 basta).
  * **Children**: as cúpulas de cada anel, de 4 a 10.
  * **Bend angle** (0,3 a 1,3 funcionam melhor): o quanto as cúpulas se
    abaulam; valores baixos dão discos planos, valores altos cúpulas
    redondas. Acima de cerca de 1,3 a superfície fica granulada.
  * **Colour scale**: cada cúpula tem a sua própria cor. Escolha *2ª
    escolha* (o orbit trap) nas configurações de cor da Iluminação e uma
    faixa de cores larga (por exemplo de 0 a 100) para vê-las.
  * **Start curved**: 1 (o padrão) começa com a bola, 0 com um piso plano
    do qual cresce o primeiro anel.

  Use um *Raystep* pequeno (0,2; 0,05 com 9 ou 10 filhos): a estimativa de
  distância da forma é grosseira, como no próprio arquivo Fragmentarium do
  TGlad. Como a SphereTree, ela roda na placa de vídeo e no processador,
  precisa de apenas 1 iteração, e parâmetros com ela só abrem neste
  programa.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="DomeTree com 8 filhos no renderizador Monte Carlo, colorida por cúpula"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 5 filhos, ângulo de curvatura 1,2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 6 filhos, ângulo de curvatura 0,5: cúpulas planas"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 10 filhos: renda entre as cúpulas e uma bola grande"></a> |
  |:---:|:---:|:---:|

  </details>
* **Uma nova fórmula, UlamSpiral:** a espiral de números primos de Ulam em
  3D, como cidade, silhueta de arranha-céus, esferas ou bloco.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="Uma nova fórmula: UlamSpiral"></a>

  <details><summary>Saiba mais</summary>

  Em 1963, o matemático Stanisław Ulam assistia a "uma palestra longa e
  muito chata" numa reunião e rabiscava numa folha de papel quadriculado.
  Sua primeira ideia foi montar problemas de xadrez; em vez disso, ele
  numerou os quadrados 1, 2, 3, … em espiral, com o 1 no meio, e circulou
  os números primos. Os primos se alinharam em linhas diagonais, como se
  seguissem um padrão, e continuam assim quando um computador desenha
  milhões deles (Ulam testou 100.000 no MANIAC II em Los Alamos; Martin
  Gardner pôs a imagem na capa da Scientific American em 1964). Até hoje
  ninguém conseguiu explicar isso por completo.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="A espiral de Ulam: os primos entre os números de 1 a 78.961 numa espiral quadrada, com o início da espiral desenhado no canto"></a>
  
  **UlamSpiral** constrói essa espiral em 3D, uma forma por número, em
  quatro formas (*Mode*):
  
  * **0, torres de primos**: uma placa para cada número e uma torre em cada
    primo. As linhas diagonais de primos viram fileiras de torres.
  * **1, silhueta de divisores**: uma coluna para cada número, tanto mais
    alta quanto mais divisores ele tem; os primos são as ruas baixas entre
    elas.
  * **2, esferas de divisores**: uma esfera para cada número, tanto maior
    quanto mais divisores ele tem (os primos são as pequenas).
  * **3, um bloco de primos**: os números enrolados em cascas de cubo em vez
    de um quadrado, um cubo em cada primo.
  
  *Rings* define o tamanho da espiral (60 anéis são os números até 14.641),
  *Height* a altura das torres ou colunas, *Gap* o espaço entre elas. Com
  *2ª escolha* (o orbit trap) nas configurações de cor, a cor acompanha o
  número de divisores, então os primos têm uma cor própria. Como a
  SphereTree, ela roda na placa de vídeo e no processador, funciona depois
  de transformações dIFS, precisa de apenas 1 iteração, e parâmetros com
  ela só abrem neste programa.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, torres de primos: as fileiras diagonais de números primos como uma cidade, no renderizador Monte Carlo com névoa, sombras duras e profundidade de campo"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, silhueta de divisores"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, esferas de divisores"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, um bloco de primos em cascas de cubo"></a> |
  |:---:|:---:|:---:|

  </details>
* **Um render parado continua de onde estava.** O que um render calculou
  é guardado quando ele é parado, e também quando o programa é fechado ou
  trava, ou quando o computador desliga (é salvo no disco a cada 5
  minutos): Calcular 3D, Monte Carlo (também no meio de uma passada), o
  quadro de uma animação ou a imagem de um lote em andamento, uma
  exportação de voxels, uma malha do Bulb Tracer, blocos de Render grande e
  um render em enxame (Parar no computador que criou a tarefa mantém todas
  as partes prontas, e cada computador mantém a parte em que estava).
  Calcular 3D e Monte Carlo com os mesmos parâmetros perguntam se devem
  continuar, os outros continuam sozinhos; o botão **Retomar** (aba Abrir)
  lista os renders inacabados, para continuá-los ou excluí-los. Na linha de
  comando o mesmo comando continua sozinho (também uma imagem em `--tiles`,
  `montecarlo`, `animate`, `batch`, `voxel` e `mesh`); `--fresh` começa de
  novo, `mb3d resume` os lista. Um render continuado é igual a um calculado
  de uma só vez.
* **Renderização dividida entre os seus computadores**, um "enxame":
  computadores que enxergam uma mesma pasta compartilhada renderizam juntos
  uma imagem, um Render grande, uma imagem Monte Carlo ou uma animação.
  Veja [Renderizar em vários computadores](#rendering-on-several-computers-the-swarm).
* **Uma nova fórmula, PseudoKleinian:** a "arquitetura kleiniana" de
  salões, arcos e esferas como uma só fórmula, com a sua própria distância
  exata.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="Uma nova fórmula: PseudoKleinian"></a>

  <details><summary>Saiba mais</summary>

  Os salões, arcos, pilares e esferas aninhadas conhecidos como
  "arquitetura kleiniana" estão entre as imagens de fractais 3D mais
  queridas. No MB3D elas só podem ser feitas como um híbrido de várias
  fórmulas com parâmetros cuidadosamente ajustados, a receita que Theli-at
  encontrou em 2011; depois, knighty escreveu a mesma coisa como uma única
  fórmula com a sua própria estimativa de distância, e essa é a
  **PseudoKleinian**. Cada nível dobra o ponto para dentro de uma caixa, o
  inverte numa esfera e soma um deslocamento, e a distância é medida
  exatamente até um plano de piso depois das dobras, então as paredes saem
  limpas e a imagem renderiza rápido. Os parâmetros são os de knighty:
  *Levels* (8), *Size* (a esfera de inversão), *CSize X/Y/Z* (a caixa; os
  seus 0,924, 0,908, 0,924 dão os salões clássicos, e pequenas mudanças
  dão novos edifícios), *Julia X/Y/Z* (um deslocamento por nível), *Shape*
  (0 o plano de piso, 1 um plano-cilindro de *Thickness* passando por
  *Offset*, 2 uma caixa arredondada), *DE offset* (escava a superfície) e
  *Colour scale* (com *2ª escolha*, o orbit trap: o menor raio de órbita,
  as clássicas faixas kleinianas).
  
  A estrutura preenche todo o espaço, então você a olha por dentro: comece
  com o centro da vista perto da origem e a câmera uma ou duas unidades
  atrás, e percorra os salões com o Navegador. No renderizador Monte Carlo,
  um interior quase não recebe luz do céu, então coloque uma luz posicional
  dentro do salão (a primeira imagem) ou abra a estrutura com um plano de
  corte e olhe de cima (a segunda). Ela roda na placa de vídeo e no
  processador, precisa de 1 iteração, funciona depois de transformações
  dIFS e em combinações DE como as outras fórmulas dIFS (embora dentro de
  uma estrutura que preenche o espaço elas mudem pouco), e parâmetros com
  ela só abrem neste programa.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="PseudoKleinian do ponto de vista de knighty: um salão de arcos e esferas aninhadas iluminado por uma lâmpada lá dentro, no renderizador Monte Carlo com névoa, sombras duras e profundidade de campo"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="PseudoKleinian aberta com um plano de corte e vista de cima: a treliça de esferas e as células entre elas"></a>

  </details>
* **Pares estéreo num só passo**: as imagens dos dois olhos renderizadas e
  mostradas lado a lado, para visão paralela ou cruzada. Contribuição de
  Rathinagiri S ([SRathinaGiri](https://github.com/SRathinaGiri)).

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="Um par estéreo de Hydra, de Dainbramage, lado a lado para visão paralela"></a>

  <details><summary>Saiba mais</summary>

  **Estéreo ▸ Renderizar par estéreo** renderiza a imagem do olho esquerdo
  e depois a do direito com as configurações estéreo da aba e as mostra
  lado a lado na janela principal. Elas são posicionadas para visão
  paralela (imagem esquerda à esquerda); **Prévia de olhos cruzados** as
  troca para a visão cruzada. É uma prévia: iluminação e cores
  não podem ser alteradas depois sem renderizar de novo, e não funciona com
  blocos. Os botões para as imagens de cada olho continuam lá.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="Um par estéreo de Hydra, de Dainbramage, lado a lado para visão paralela"></a>

  </details>
* **Tempo restante para a imagem inteira**: a barra sob a imagem se enche
  uma única vez ao longo de todo o cálculo, e ao lado dela aparece o tempo
  que falta até a imagem ficar pronta, incluindo sombras, oclusão e
  reflexos.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="A barra de progresso e o tempo restante total"></a>

  <details><summary>Saiba mais</summary>

  A barra do MB3D recomeça a cada etapa. Aqui ela mostra o cálculo
  inteiro: a renderização principal, depois as sombras duras e a oclusão de
  ambiente, os reflexos e a profundidade de campo. Quanto tempo as etapas
  depois da renderização principal levam é estimado a partir do que a cena
  pede e de como foi a renderização principal; a estimativa foi ajustada com
  cerca de 1400 renders de 366 cenas. Uma imagem calculada de novo espera
  os tempos que levou da última vez.
  
  A primeira imagem com novas fórmulas na placa de vídeo espera a
  compilação dos seus shaders: a linha então diz *compilando shaders* com o
  tempo restante, que considera o ritmo do processador e, quando os shaders
  ficarem prontos a tempo, a entrada da placa. Detalhes em
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar).
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="A janela principal durante as sombras duras e a oclusão de ambiente, com o tempo restante total"></a>

  </details>
* **Histórico** atrás do ícone de relógio da barra de título: cada imagem
  que você calculou com a sua miniatura, cada arquivo que você abriu ou
  salvou, como lista, como linhas ou como miniaturas de qualquer tamanho,
  agrupado por dia ou numa sequência só; pesquise, ordene, mostre só as
  imagens que diferem e carregue qualquer item com um clique duplo. Um
  histórico de centenas de milhares de itens abre na hora.
* **Assistentes de IA podem usar o mb3d** (`mb3d mcp`, o Model Context
  Protocol): peça ao app de desktop do Claude ou ao Claude Code, em
  palavras simples ou por voz através do assistente, para descrever,
  alterar e renderizar os seus arquivos de parâmetros, criar variações ou
  renderizar aquilo em que você trabalhou por último; o assistente vê as
  imagens e pode ajustá-las. O botão **IA** na aba Prefs o conecta com um
  clique (ou `mb3d mcp --setup`). O mb3d continua offline (o app do
  assistente o inicia no seu computador), grava apenas na pasta `AI` e
  nunca exclui nada; imagens acima de 4K precisam da sua autorização.
* **Um log de tudo** atrás do ícone de página da barra de título: cada
  clique, configuração e valor digitado, cada cálculo com as suas etapas e
  tempos, arquivos abertos e salvos, janelas, mensagens, avisos e erros,
  numa janela que os acompanha ao vivo e num arquivo por dia. Uma sessão
  que congelou ou foi encerrada à força é relatada na próxima
  inicialização, e um relatório de falha traz as últimas linhas do log.
* **A iluminação continua ao vivo** depois das sombras duras e dos
  reflexos: mova uma luz ou mude a sua cor e a imagem muda na hora, como
  no Mandelbulb3D.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="A janela principal e a janela Iluminação logo depois de uma mudança na iluminação"></a>

  <details><summary>Saiba mais</summary>

  Renderize de forma simples e depois experimente posições e cores de luz:
  cada mudança na janela Iluminação repinta na hora, com as sombras como
  foram calculadas, como no Mandelbulb3D. Mudar a cor ou a intensidade de
  uma luz nunca calcula as sombras de novo. As sombras de uma luz que você
  moveu, os reflexos e a profundidade de campo voltam com os botões da
  janela Pós-processamento, como no Mandelbulb3D.
  
  Um cálculo que você parou também pode continuar depois de você mudar a
  iluminação. Um jeito de trabalhar com isso: [Fluxo de trabalho de iluminação](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-pt)
  na wiki. Detalhes em
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field).
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="A janela principal e a janela Iluminação logo depois de uma mudança na iluminação: a imagem já mais clara"></a>

  </details>

<a id="download"></a>

## Download

Programas prontos para rodar da versão mais recente, sem instalação:

| Sistema | Download |
|---|---|
| Windows (64 bits) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux (64 bits) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint (instala como os outros programas) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE (instala como os outros programas) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple Silicon (M1 e posteriores) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

Descompacte e inicie o **Mandelbulb3D** (`Mandelbulb3D.exe` no Windows) na
pasta descompactada. A pasta é organizada como uma instalação do MB3D, com
as fórmulas, os mapas e 80 arquivos de parâmetros de exemplo do MB3D.

* Se o Windows mostrar "O Windows protegeu o computador", clique em *Mais
  informações* ▸ *Executar assim mesmo*.
* O macOS bloqueia programas baixados que não são assinados: rode
  `xattr -dr com.apple.quarantine .` uma vez na pasta descompactada.

**Alternativa para Linux e macOS:** em vez de baixar à mão, esta única
linha busca a versão mais recente em `~/mb3d-rs` (rode-a de novo mais tarde
para atualizar; o script [download.sh](../../download.sh) também está na
pasta do programa):

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

Ela verifica o download antes de descompactá-lo, coloca o **Mandelbulb3D**
no menu de aplicativos (Linux) e cria os comandos `Mandelbulb3D` e `mb3d`,
que você pode digitar em qualquer pasta. `sh ~/mb3d-rs/download.sh
--uninstall` remove de novo o item do menu e os comandos; os seus arquivos
ficam.

Os programas para Linux rodam em distribuições de cerca de 2018 em diante
(Ubuntu 18.04, Debian 10, Rocky/RHEL 8 e mais novas).

**Instalado como os outros programas:** no Debian, Ubuntu e Mint, abra o
arquivo `.deb` acima (ou `sudo apt install ./mb3d-rs-linux-x64.deb`); no
Fedora, RHEL e Rocky, o arquivo `.rpm` (`sudo dnf install
./mb3d-rs-linux-x64.rpm`, no openSUSE `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
O Mandelbulb3D fica então no menu de aplicativos e o `mb3d` funciona em
qualquer pasta; as suas configurações e o Histórico ficam guardados em
`~/.config/mb3d-rs` e `~/.local/share/mb3d-rs`.

[Primeiros passos](../../GETTING_STARTED.md) (em inglês) guia pelos primeiros
passos; versões mais antigas e as notas de versão estão na página de
[releases](https://github.com/thr32423/mb3d-rs/releases).

<a id="guides-and-help"></a>

## Guias e ajuda

Guias passo a passo estão na [wiki](https://github.com/thr32423/mb3d-rs/wiki/Home-pt):

* [Instalação](https://github.com/thr32423/mb3d-rs/wiki/Installing-pt): Windows, macOS e Linux, atualização, onde ficam os seus arquivos
* [Fluxo de trabalho de iluminação](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-pt): renderize de forma simples, depois experimente luzes, sombras e reflexos
* [Configuração do enxame](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-pt): renderizar em vários computadores através de uma pasta compartilhada
* [Solução de problemas e FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-pt): quando algo não funciona como esperado

Todas as versões com as suas mudanças: [a lista completa](../UPDATES.md) (em inglês).

<a id="rendering-on-several-computers-the-swarm"></a>

## Renderizar em vários computadores (o enxame)

Computadores que enxergam uma mesma pasta compartilhada renderizam juntos:
uma imagem em blocos, um Render grande, uma imagem Monte Carlo, uma
animação ou um lote. Cada computador pega as partes que ninguém mais pegou,
então um computador rápido pega mais delas, e o computador que iniciou o
render junta os resultados. Quem faz o compartilhamento é a pasta: o
programa não abre nenhuma conexão de rede e não fala com ninguém, então é
tão seguro quanto o seu compartilhamento de arquivos.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="O Mandelbulb3D com a janela Enxame, aberta com o botão Enxame da aba Ferramentas, enquanto dois computadores renderizam uma imagem em blocos: a pasta compartilhada, Compartilhar este computador, os computadores e o que cada um está fazendo, a tarefa, e a prévia com os blocos renderizados, um bloco em andamento e os que faltam"></a>

<a id="setting-it-up"></a>

### Configuração

Passo a passo, com o que verificar: [Configuração do enxame](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-pt) na wiki.

1. **Uma pasta na qual todos os computadores possam gravar:** um
   compartilhamento num NAS, um compartilhamento do Windows, um
   compartilhamento Samba ou NFS. No Linux, monte-o como pasta, por
   exemplo
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`;
   um compartilhamento apenas aberto no gerenciador de arquivos
   (`smb://...`) não é uma pasta que o programa possa usar.
2. **A mesma versão do mb3d-rs em todos os computadores.** Um computador
   com outra versão aparece na lista, mas não participa.
3. **No Mandelbulb3D:** o botão **Enxame** na aba Ferramentas. Escolha a
   pasta compartilhada (o mesmo compartilhamento em todos os computadores;
   o caminho pode ser diferente), dê ao computador um nome para a lista e
   marque **Compartilhar este computador**.

É só isso. A linha de status diz o que acontece, por exemplo *Compartilhando
com omarchy: os renders são divididos entre os 2 computadores*. Sozinho no
enxame, tudo é renderizado neste computador como de costume.

<a id="what-is-shared"></a>

### O que é compartilhado

Enquanto *Compartilhar este computador* estiver marcado, o computador
renderiza as partes que outros computadores colocam na pasta, e os seus
próprios renders vão para o enxame sempre que houver outro computador nele:

* **Calcular 3D** (e a janela de lote) — a imagem em N x M blocos
  (definido na janela Enxame), montada aqui com tudo o que o cálculo
  normal oferece: mudanças de iluminação, pós-processamento, salvar como
  m3i (e a sua memória, veja abaixo).
* **Render grande** — os blocos são salvos um a um conforme chegam; a
  imagem inteira nunca é mantida na memória, então este é o caminho para
  imagens de 100000 x 100000 pixels e mais.
* **Monte Carlo** — como num só computador: cada computador acrescenta
  passadas ao seu próprio bloco até você clicar em Parar, a imagem vai
  ficando limpa com os raios de todos, e Parar faz a média dos blocos. No
  modo de lote da janela, os blocos juntos param na contagem de raios do
  lote.
* **O Criador de animações** — os quadros, um a um.

A janela lista os computadores com os seus núcleos, a placa de vídeo e o
que cada um está fazendo, as tarefas com o seu progresso, e uma prévia da
tarefa mais recente: os blocos renderizados, os que estão em andamento com
uma barra de progresso, e os que ainda faltam.

<a id="on-the-command-line"></a>

### Na linha de comando

Em cada computador que deve ajudar, inicie um worker na pasta:

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

Depois, em qualquer computador, renderize com `--swarm` e a mesma pasta:

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER` mostra quem está no enxame, o que cada computador
está fazendo e em que ponto estão as tarefas. Uma imagem Monte Carlo na
linha de comando é dividida em blocos com os seus próprios raios
aleatórios: 4 blocos de 64 raios por pixel dão os 256 pedidos com
`--rays 256`.

<a id="good-to-know"></a>

### Bom saber

* **Se os outros computadores ficarem ociosos** e a lista de tarefas deles
  estiver vazia enquanto este computador renderiza: o computador que serve
  a pasta com Samba e a usa diretamente deixa os outros manterem uma
  listagem antiga dela. Desative esse cache em `/etc/samba/smb.conf`:
  `smb3 directory leases = no` em `[global]`, e `oplocks = no` e
  `level2 oplocks = no` na seção do compartilhamento, depois
  `sudo systemctl restart smb` e monte o compartilhamento de novo nos
  outros computadores.
* **Memória:** uma imagem ocupa cerca de 20 bytes por pixel neste programa
  (o seu buffer de imagem), mais 10 na janela principal, e depois a maior
  de duas etapas: o cálculo principal com as suas sombras e oclusão (16 por
  pixel e 1,2 GB para a placa), ou a pintura (3 por pixel, mais 20 com
  profundidade de campo, e uma faixa dos reflexos: no máximo 3,5 GB, menos
  quando há pouca memória livre, já que a faixa é dimensionada para a
  metade do que está livre quando a passada começa). Uma imagem de 12000 x
  6000: 6 GB. Quando uma imagem precisa de mais do que a memória livre do
  computador (consultada no Linux, Windows e macOS), Calcular 3D diz quanto
  ela precisa e quanto está livre, e pergunta antes de começar: o cálculo
  usaria a memória de troca e o programa poderia deixar de responder por
  minutos seguidos. O Render grande, sozinho ou com o enxame, é a saída: os
  seus blocos são salvos um a um, a imagem inteira nunca é mantida.
  Calcular 3D com o enxame mantém a imagem aqui como o cálculo normal faz
  (sem a etapa do cálculo principal, que os computadores dos blocos fazem
  com a memória de cada bloco); um computador do enxame deixa para os
  outros um bloco grande demais para a sua memória.
* **Segurança:** um computador renderiza apenas o que uma tarefa pede e
  grava apenas na pasta da tarefa; o código de fórmula de uma tarefa roda
  no emulador de fórmulas como qualquer fórmula.
* **Velocidade:** os blocos viajam como buffers de imagem completos,
  algumas centenas de megabytes para uma imagem muito grande, então um
  compartilhamento rápido compensa. Um serviço de sincronização que copia
  com atraso também funciona, mas mais devagar.
* **Computadores que saem:** a parte que um computador estava renderizando
  é assumida por outro depois de meio minuto sem notícias dele. Uma tarefa
  cujo dono sumiu há meia hora é removida por quem a encontrar. Uma tarefa
  que falha num computador é anotada na janela e deixada para os outros.
* **Parar:** desmarcar *Compartilhar este computador* deixa a parte em
  andamento terminar primeiro; fechar o programa a abandona, e outro
  computador a assume.
* **Parar a tarefa** (Parar no computador que a criou, ou fechá-lo, ou ele
  travar): cada parte já pronta é mantida nesse computador, e cada
  computador mantém a parte que estava renderizando. Iniciar o mesmo render
  de novo cria uma tarefa só com as partes que faltam, e um computador que
  recebe de volta uma parte que tinha começado a continua. O mesmo vale na
  linha de comando: rode o mesmo comando de novo. Um render Monte Carlo
  sem fim definido (o da janela) junta os seus blocos quando é parado,
  como antes.

<a id="how-the-rendering-works"></a>

## Como funciona a renderização

Uma imagem de fractal é calculada pixel por pixel: para cada pixel, um raio
é seguido cena adentro até atingir a superfície do fractal, e a fórmula é
avaliada dezenas de vezes a cada passo. O MB3D faz isso no processador,
usando todos os seus núcleos. O mb3d-rs pode fazer o mesmo, e também pode
passar o trabalho para a **placa de vídeo**, que tem milhares de pequenos
processadores e foi feita exatamente para esse tipo de tarefa.

A configuração **"Calcular em"** (aba Interno da janela principal) escolhe:

| | |
|---|---|
| **Auto** (padrão) | A placa e o processador dividem a imagem: a placa pega linhas de cima, as threads do processador de baixo, até se encontrarem. As sombras e a oclusão de ambiente são divididas da mesma forma, ou feitas só pela placa quando ela é claramente a mais rápida. |
| **CPU** | Só o processador, como no MB3D. |
| **Placa de vídeo** | Só a placa (ela espera até o seu programa ficar pronto). |

Na primeira vez em que uma combinação de fórmulas é usada, o driver de
vídeo precisa preparar um programa para a placa — o *shader* —, o que leva
de 10 a 15 segundos no computador medido abaixo. No modo Auto o processador
simplesmente começa sem ele e a placa entra quando estiver pronta, então
uma imagem pequena nunca espera. O shader preparado é guardado para a
próxima vez (pelo driver de vídeo; no Linux, na pasta `ShaderCache` ao lado
do programa), então a próxima imagem com as mesmas fórmulas já começa na
placa.

O **LED** à direita do título da janela diz onde a última imagem foi
calculada: verde na placa de vídeo, laranja em parte, vermelho no
processador. Apontar para ele mostra os detalhes.

[Como funciona a renderização](../RENDERING.md) (em inglês) conta a história
toda com imagens: o que o Auto faz na primeira vez, por que a placa nem
sempre é a escolha mais rápida e o que significam as cores do LED.

Nem tudo pode ir para a placa: algumas fórmulas não podem ser traduzidas
para ela, e vistas com zoom muito profundo são calculadas em precisão dupla
(o que só algumas placas conseguem). O programa decide tudo isso sozinho;
você só vê isso no LED.

<a id="speed"></a>

## Velocidade

Todas as medições num só computador: um AMD Ryzen 9 3900X (12 núcleos, 24
threads) com uma NVIDIA GeForce RTX 2060, no Linux. O **Mandelbulb3D
1.99sr37** original foi rodado na mesma máquina sob o Wine (o seu programa
para Windows roda ali sem alterações, a toda velocidade, usando todos os
núcleos), e cronometrado do clique em *Calcular 3D* até a imagem pronta.
Cada arquivo de parâmetros foi renderizado no mesmo tamanho de imagem pelos
três: o MB3D, o mb3d-rs só no processador (**CPU**) e o mb3d-rs no modo
**Auto** (placa de vídeo e processador juntos). Os tempos do Auto são os do
segundo render de um arquivo, com o shader da placa já preparado; o
primeiro render de uma nova combinação de fórmulas acrescenta os 10–15 s de
preparação do shader nesta placa.

<a id="mb3ds-80-example-parameter-files"></a>

### Os 80 arquivos de parâmetros de exemplo do MB3D

Os arquivos de exemplo que vêm com o MB3D (e na pasta `M3Parameter` deste
programa), cada um no seu próprio tamanho de imagem.

O MB3D não conseguiu calcular um dos arquivos (*Test8*: ele para com um erro de "Division by zero"), então as comparações contam 79 arquivos.

* **O mb3d-rs só no processador é mais rápido que o MB3D** em 71 dos 79 arquivos, em mediana **1,7 vez**; todos os arquivos juntos levaram 18 min 21 s no MB3D e 17 min 10 s no mb3d-rs no processador.
* **O modo Auto** é mais rápido que o MB3D em 77 de 79 arquivos (mediana de 2,1 vezes); todos os arquivos juntos, 8 min 17 s. Em 30 dos arquivos o processador terminou sozinho, na maioria porque precisa de só alguns segundos, antes de a placa ter a sua vez — ali o Auto é simplesmente o processador mais um instante de sobrecarga. A placa entrou em 49 arquivos, os mais lentos, onde deixou a imagem em mediana 1,6 vez mais rápida que o processador (até 13 vezes: *Insect Menger*, 4,7 s em vez de 63,0 s).
* Em 4 arquivos o Auto foi visivelmente mais lento que só o processador (*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*): ali a placa é mais lenta que as 24 threads do processador para a cena.
* "Linhas na placa" é a parte da imagem que a placa de vídeo percorreu no modo Auto (– quando o processador terminou sozinho).

<details>
<summary>Todos os 80 arquivos (clique para abrir)</summary>

| Arquivo de parâmetros | MB3D (Wine) | mb3d-rs CPU | mb3d-rs Auto | linhas na placa | Auto vs MB3D |
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
| Test8 | erro no MB3D | 0.1 s | 0.3 s |  | – |
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

As imagens são as mesmas: o mb3d-rs no processador produz a imagem do MB3D
até o arredondamento, e as imagens do Auto diferem das do processador em
média por uma fração de um por cento da faixa de cores (detalhes finos
podem cintilar em pixels diferentes; as [notas técnicas](../TECHNICAL.md)
(em inglês) trazem as medições).

<a id="the-command-line"></a>

## A linha de comando

O `mb3d` renderiza sem janela. Abra um terminal na pasta do programa (no
Windows, um prompt de comando na pasta descompactada, e digite `mb3d` em
vez de `./mb3d`). Depois da instalação em uma linha no Linux ou no macOS, o
`mb3d` funciona em qualquer pasta, `man mb3d` mostra o seu manual, e a
tecla Tab completa as suas opções no bash, zsh e fish:

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

Todo o resto tem um subcomando com o seu próprio `--help`:

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

As fórmulas e os mapas são encontrados ao lado do programa; os de outra
instalação do MB3D podem ser indicados com `--formulas` e `--maps`. As
animações são as do MB3D: quadros-chave com a interpolação do MB3D,
projetos `.m3a` abertos e salvos, quadros gravados como no MB3D, de modo
que uma animação interrompida continua e vários computadores podem
dividir uma.

<a id="good-to-know-1"></a>

## Bom saber

* **Mesmas imagens, não idênticas bit a bit.** O MB3D calculava na precisão
  de 80 bits da antiga aritmética x87; este programa usa a de 64 bits de
  hoje. As imagens coincidem em todas as estruturas, mas pixels isolados
  podem diferir no último dígito da sua cor. Os renders Monte Carlo e o
  MutaGen usam os seus próprios números aleatórios, então os seus
  resultados não são os do MB3D.
* **As imagens da placa de vídeo** diferem um pouco das do processador
  (precisão simples); em detalhes muito finos, a cintilação pode cair em
  outros pixels. Zooms profundos passam sozinhos para precisão dupla.
* **A velocidade no processador** é próxima da do MB3D: o código de máquina
  das fórmulas dos arquivos `.m3f` do MB3D foi traduzido para rodar em
  qualquer processador (também no Apple Silicon), com um pequeno custo; o
  ray marching em si é mais rápido. Veja as tabelas acima. No Linux e no
  Windows, as fórmulas do híbrido de uma cena também são compiladas juntas
  numa só função quando uma imagem começa (em poucos milissegundos, com o
  Cranelift), para que passem os seus valores adiante nos registradores do
  processador. Junto com um laço mais rápido sobre as fórmulas (em todos os
  sistemas), os renders na CPU dos 80 arquivos de exemplo levam 20 % menos
  tempo do que na v0.2.74 (até 60 % em alguns), com imagens idênticas até
  o bit. Uma cena só mantém o híbrido compilado se ele medir mais rápido;
  `MB3D_NO_JIT=1` o desliga. No macOS ele também está ativado, mas ainda
  em teste ali: `./jit_check.sh` na pasta descompactada o verifica (ele
  compara as fórmulas compiladas com as traduzidas e renderiza os arquivos
  de exemplo com e sem ele; as imagens precisam ser as mesmas).
* **Imagens grandes** podem ser renderizadas de uma vez só (uma imagem de
  15000 x 20000 precisa de cerca de 15 GB de memória; com profundidade de
  campo leva horas) ou em blocos, também em vários computadores
  (`--tiles`, e a janela Renders grandes), o que precisa de menos memória.
  Calcular 3D não começa uma imagem que não cabe na memória livre (no
  Linux, onde a memória livre pode ser consultada) e avisa, e um computador
  do enxame deixa para os outros um bloco grande demais para a sua
  memória.
* **Renders inacabados** ficam guardados na pasta de cache
  (`~/.cache/mb3d-rs/resume`, no Windows `%LOCALAPPDATA%\mb3d-rs\resume`,
  ou ao lado do programa): 18 bytes por pixel para uma imagem (uma imagem
  de 15360 x 15360 ocupa 4,2 GB), 64 para um render Monte Carlo. O que foi
  calculado é gravado na hora e descarregado no disco a cada 5 minutos. Um
  render parado nos seus primeiros 10 segundos não guarda nada; renders
  inacabados com mais de 30 dias são apagados, e também os mais antigos
  quando todos juntos ocupam mais de 50 GB. Um render só continua com os
  mesmos parâmetros e a mesma versão do programa (e a mesma escolha de
  placa ou processador; a exposição e a saturação de um render Monte Carlo
  podem mudar). Um render Monte Carlo continua inacabado até atingir os
  seus raios (um lote, `--rays`, `--passes`) ou ser excluído na lista
  Retomar.
* **Fórmulas, mapas, temas e idiomas mais novos ou seus** vão em
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes` e
  `.../Languages` (qualquer que seja a forma de instalação; no Windows
  `%APPDATA%\mb3d-rs\...`). Eles são usados além dos que vêm com o programa
  e substituem um arquivo incluído de mesmo nome, então uma fórmula
  atualizada dos fóruns passa a valer sem mexer na pasta do programa.

<a id="updates"></a>

## Atualizações

Todas as versões com as suas mudanças estão listadas em [UPDATES.md](../UPDATES.md) (em inglês).

<a id="for-developers"></a>

## Para desenvolvedores

As [notas técnicas](../TECHNICAL.md) (em inglês) descrevem como a placa de
vídeo é usada, o que foi portado das fontes em Delphi do MB3D e como, as
três formas de executar o código de máquina das fórmulas do MB3D, o formato
do arquivo de cena e as suas chaves, as verificações contra o MB3D e a
compilação a partir do código-fonte:

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## Licença

O MB3D é licenciado sob a LGPL 2.1. Este port é uma obra derivada e usa a
mesma licença.

<a id="credits"></a>

## Créditos

As pessoas e os projetos em que o mb3d-rs se baseia estão citados no [README em inglês](../../README.md#credits).
