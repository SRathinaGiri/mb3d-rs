<img src="../../assets/social-preview.png" width="760" alt="mb3d-rs: Linux, Windows, macOS용 Mandelbulb3D">

# mb3d-rs — Linux, Windows, macOS용 Mandelbulb3D
<!-- languages:start -->
[English](../../README.md) · [Deutsch](README.de.md) · [Français](README.fr.md) · [Italiano](README.it.md) · [Nederlands](README.nl.md) · [Polski](README.pl.md) · [Kaszëbsczi](README.csb.md) · [हिन्दी](README.hi.md) · [日本語](README.ja.md) · **한국어** · [简体中文](README.zh-CN.md) · [繁體中文](README.zh-TW.md)
<!-- languages:end -->

*영어 README(버전 0.2.84)를 번역한 문서입니다. 두 문서의 내용이 다를 때는 [영어 README](../../README.md)가 맞습니다.*

**다운로드: [최신 릴리스](https://github.com/thr32423/mb3d-rs/releases/latest)** · [다운로드](#download) · [새 소식](../UPDATES.md)

mb3d-rs는 3D 프랙탈 프로그램인 [Mandelbulb3D](https://github.com/thargor6/mb3d)(MB3D)를
오늘날의 컴퓨터에서 실행되도록 다시 작성한 프로그램입니다. **Linux, Windows,
macOS**에서 실행되며, MB3D는 프로세서만 사용할 수 있었지만 이 프로그램은 그림을
**그래픽 카드**에서 계산합니다. MB3D의 파라미터 파일(`.m3p`, `.m3i`, 포럼에 올라온
텍스트 파라미터)을 열고, MB3D의 공식과 맵을 사용하며, 같은 그림을 만듭니다.

<a href="../img/main_window.jpg"><img src="../img/main_window.jpg" width="760" alt="메인 창"></a>

두 개의 프로그램으로 제공됩니다.

* **Mandelbulb3D** — 데스크톱 프로그램으로, MB3D의 창을 그대로 갖추고 있습니다.
  메인 창, 공식, 조명, 후처리, 내비게이터, 애니메이션, 일괄 처리, 큰 렌더링,
  몬테카를로, 복셀 내보내기, Bulb Tracer 2, MutaGen 등입니다. MB3D를 아신다면
  이 프로그램도 이미 아시는 것이며, MB3D의 튜토리얼도 그대로 적용됩니다.

  <details><summary>자세히 보기</summary>

  창은 MB3D의 것 그대로입니다. MB3D의 원래 창 정의로 만들었기 때문에 모든 버튼,
  슬라이더, 힌트가 MB3D와 같은 자리에 있습니다.
  
  * **메인 창:** 파라미터 열기와 저장(`.m3p`, `.m3i`, 텍스트, 클립보드), 그림
    저장(PNG, JPEG, BMP, 16비트 Z-buffer), 마우스 모드, 회전 버튼, 이미지 크기,
    품질 프리셋, 실행 취소, 그림이 점점 채워지고 통계가 표시되는 *3D 계산*, 그리고
    모든 페이지(계산, 내부, 정보, 절단, Julia, 카메라, 색상, 스테레오).
  * **공식:** 여섯 개의 공식 슬롯, MB3D의 공식 목록과 옵션, 하이브리드 유형,
    즐겨찾기, JIT 공식 편집기.
  * **조명:** 조명 1–6, 프리셋, 팔레트, 물체·앰비언트·동적 안개·배경 그림
    페이지. 변경하면 새로 계산하지 않고 그림을 바로 다시 그립니다.
  * **후처리:** Z-buffer 기반 법선, 하드 섀도와 앰비언트 섀도, 반사와 투명도,
    피사계 심도.
  * **내비게이터:** 키보드와 마우스로 걸어 다니며 둘러본 뒤 *시점을 메인으로*.
  * **애니메이션 메이커, 일괄 처리, 큰 렌더링, 몬테카를로, 복셀 내보내기,
    Bulb Tracer 2**(메시)와 메시 미리보기, **높이맵 생성기, MutaGen, ZBuf16Bit,
    맵 시퀀스, Ini 폴더** — 모두 MB3D와 같습니다.
  * **스웜**(새 기능, 도구 탭): 여러 대의 컴퓨터가 함께 렌더링합니다.
    [여러 컴퓨터에서 렌더링하기](#rendering-on-several-computers-the-swarm)를 참고하십시오.

  </details>
* **mb3d** — 스크립트와 일괄 작업을 위한 명령줄 렌더러입니다. 그림, 애니메이션,
  일괄 목록, 몬테카를로 렌더링, 복셀 및 메시 내보내기를 모두 창 없이 수행합니다.

<a id="is-it-safe-to-run-check-it-yourself"></a>

## 실행해도 안전한가요? 직접 확인해 보십시오

저희 말을 그대로 믿으실 필요는 없습니다. [scanrepo.dev](https://www.scanrepo.dev)는
GitHub 저장소에서 유해한 소프트웨어의 징후를 검사하는 독립 서비스입니다. 다른
프로그램을 실행하는 코드, 인터넷에 연결하는 코드, 하는 일을 숨기는 코드,
프로그래밍 언어의 안전 검사를 우회하는 코드를 찾습니다. 누구나 언제든지 이
저장소에 대해 검사를 실행할 수 있습니다.

**[▶ scanrepo.dev에서 mb3d-rs 검사하기](https://www.scanrepo.dev/scan/github/thr32423/mb3d-rs)**
(현재 코드에 대한 새 결과를 보려면 *Rescan*을 클릭하십시오)

버전 0.2.85는 **안전: 100점 중 5점** 판정을 받았습니다(2026년 10월 30일 검사, 검사기
엔진 10의 규칙). 유일한 경고는 프로그램이 속도를 위해 실행 중에 컴파일하는 수식을
시작하는 줄에 대한 것입니다. 새로 만든 기계어 코드를 실행하는 일은 Rust의 검사된
형태로는 작성할 수 없으며, 프로그램이 이를 허용하는 곳은 이 줄과 Intel 노이즈 제거
필터로 이어지는 연결 부분뿐입니다. 나머지는 참고용 안내입니다. (이전: 0.2.66은 예전
엔진에서 100점 중 0점을 받았습니다. [0.2.44 결과](../img/scan_0.2.44.png)도
참고하십시오.) 이전의 한 버전이 경고를 받았고, 이에 대응해
코드를 변경했습니다([issue #4](https://github.com/thr32423/mb3d-rs/issues/4)).
지금은 이 상태를 유지하도록 작성되어 있습니다. 프로그램에는 인터넷에 연결하는
코드가 없으며, 시스템에 열어 달라고 요청하는 다른 프로그램은 폴더를 여는 버튼을
클릭했을 때의 파일 관리자뿐입니다.

검사는 알려진 경고 징후를 찾는 것이지 보증은 아닙니다. 다음 방법으로 직접 더
확인할 수 있습니다.

* **모든 소스 코드가 여기에 있습니다.** 다운로드 파일은 누군가의 개인 컴퓨터가
  아니라 GitHub의 빌드 서버에서, [릴리스 워크플로](.github/workflows/release.yml)에서
  읽을 수 있는 단계에 따라 이 소스로부터 빌드됩니다.
* **모든 릴리스에는 체크섬**(`SHA256SUMS`)이 있어, 다운로드한 파일이 게시된 파일과
  정확히 같은지 확인할 수 있습니다. 아래의 한 줄 설치 명령은 이 확인을 대신
  해 줍니다. 0.2.45 이후의 릴리스에는 GitHub의 빌드 증명(attestation)도 있어,
  `gh attestation verify
  FILE --repo thr32423/mb3d-rs`로 다운로드 파일이 어느 커밋에서 빌드되었는지
  확인할 수 있습니다.

<a id="what-is-new-compared-with-mandelbulb3d"></a>

## Mandelbulb3D와 비교해 새로워진 점

* **그래픽 카드가 그림을 계산합니다** — 이미지 자체, 그림자, 앰비언트 오클루전,
  반사, 몬테카를로 렌더링까지. 일반적인 컴퓨터에서는 프로세서만 쓸 때보다 몇 배
  빠릅니다. 아래 [속도](#speed)를 참고하십시오.
* **자동 모드**: 그래픽 카드와 프로세서가 모든 그림을 함께 계산하며, 어느 쪽도
  기다리지 않습니다. 카드가 아직 준비되지 않았으면 프로세서가 먼저 시작하고, 카드는
  준비되는 대로 합류합니다.
* **제목 표시줄의 LED**가 마지막 그림이 어디에서 계산되었는지 보여 줍니다.
* 고해상도 화면을 위한 75–200 %의 **인터페이스 크기**.

  <a href="../img/themes_150.png"><img src="../img/themes_150.png" width="300" alt="150 %의 시각 테마"></a>

  <details><summary>자세히 보기</summary>

  **환경 설정 ▸ 테마 ▸ 인터페이스 크기**로 모든 창과 그 텍스트를 고해상도 화면에
  맞게 크게 또는 작게(75–200 %) 만들 수 있습니다. 레이아웃은 MB3D와 똑같이
  유지됩니다.
  
  | 100 % | 150 % |
  |---|---|
  | ![100 %의 시각 테마](../img/themes_100.png) | ![150 %의 시각 테마](../img/themes_150.png) |

  </details>
* 창의 **여덟 가지 모양**. 모두 복사해서 고칠 수 있는 테마 파일이며, 둘은 MB3D의
  모양을 재현하고 여섯은 새로 만든 것입니다.

  <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="300" alt="Midnight 테마"></a>

  <details><summary>자세히 보기</summary>

  **환경 설정 ▸ 테마**에서 여덟 가지 모양을 고를 수 있으며, 각각 `Themes` 폴더의
  파일 하나입니다. *Glossy*(어두운 기본 테마)와 *Windows*는 MB3D 고유의 두 모양을
  재현하고, 나머지 여섯은 새로 만든 것입니다. 그림을 클릭하면 원래 크기로 볼 수
  있습니다.
  
  | | |
  |---|---|
  | <a href="../img/themes/Glossy.jpg"><img src="../img/themes/Glossy.jpg" width="380" alt="Glossy"></a> | <a href="../img/themes/Windows.jpg"><img src="../img/themes/Windows.jpg" width="380" alt="Windows"></a> |
  | **Glossy**: MB3D의 기본 모양 | **Windows**: MB3D의 "Themes off" |
  | <a href="../img/themes/Graphite.jpg"><img src="../img/themes/Graphite.jpg" width="380" alt="Graphite"></a> | <a href="../img/themes/Daylight.jpg"><img src="../img/themes/Daylight.jpg" width="380" alt="Daylight"></a> |
  | **Graphite**: 더 밝은 중성 회색, 강철 파랑 강조색 | **Daylight**: 따뜻한 밝은 회색, 청록 강조색 |
  | <a href="../img/themes/Midnight.jpg"><img src="../img/themes/Midnight.jpg" width="380" alt="Midnight"></a> | <a href="../img/themes/Ember.jpg"><img src="../img/themes/Ember.jpg" width="380" alt="Ember"></a> |
  | **Midnight**: 짙은 남색, 파랑 강조색 | **Ember**: 따뜻한 숯색, 주황 강조색 |
  | <a href="../img/themes/Forest.jpg"><img src="../img/themes/Forest.jpg" width="380" alt="Forest"></a> | <a href="../img/themes/Highcontrast.jpg"><img src="../img/themes/Highcontrast.jpg" width="380" alt="High contrast"></a> |
  | **Forest**: 어두운 녹회색, 초록 강조색 | **High contrast**: 검정 바탕, 흰 글자, 노랑 강조 |
  
  테마는 프로그램 옆 `Themes` 폴더에 있는 작은 텍스트 파일입니다. 하나를 복사해
  색을 바꾸면 다음 실행 때 목록에 나타납니다.

  </details>
* **모든 시스템에서 충돌 시 기록을 남깁니다**: 프로그램의 패닉(또는 프로그램이
  종료될 때의 오류)은 프로그램 옆이나 데이터 폴더의 `mb3d_crash.txt`에 덧붙여
  기록됩니다(이전에는 Windows에서만). 버그를 보고할 때 이 파일을 함께 보내
  주십시오.
* **컴퓨터의 모든 그래픽 카드가 계산합니다.** 설정할 것은 없습니다. 발견된 모든
  카드가 첫 번째 카드 및 프로세서와 같은 풀에서 그림의 행 묶음을 가져가므로, 빠른
  카드가 자연히 더 많이 맡습니다. 그림자, 오클루전, 몬테카를로 광선, 반사도 같은
  방식으로 나누며, 애니메이션은 카드마다 한 프레임씩 동시에 렌더링합니다. 느린
  카드(전용 카드 옆의 내장 그래픽)는 할 수 있는 만큼만 하고 그림을 붙잡아 두지
  않습니다. 남은 행이 없어지면 쉬고 있는 카드와 프로세서가 그 카드가 아직 넘기지
  않은 행을 이어받습니다. 오류가 난 카드는 제외되고 나머지가 계속합니다.
  `mb3d --list-cards`로 발견된 카드를 확인할 수 있습니다. (카드 하나를 두 번 연
  경우와 소프트웨어 Vulkan 드라이버를 두 번째 카드로 쓴 경우로 시험했으며, 실제
  카드 두 장이 있는 컴퓨터에서는 아직 시험하지 않았습니다.)
* **완성되지 않은 그림은 `.m3i` 파일입니다.** 중지되었거나, 충돌했거나, 강제로
  종료된 렌더링이 계산한 내용은 캐시에 보관됩니다(미완성 렌더링 창,
  `mb3d resume`). 그림의 경우 이 파일은 이제 언제나 `.m3i` 이미지 파일이며, 렌더링
  폴더의 `image.m3i`이고 아직 계산되지 않은 행은 배경으로 채워져 있습니다. 언제든
  복사해 가거나, 미완성 렌더링 창에서 *Save m3i*를 누르면 됩니다(명령줄에서는 `mb3d
  resume --m3i KEY file.m3i`. 둘 다 이전 버전의 `data.bin`도 `.m3i`로 저장합니다).
  변환 없이 이 프로그램이나 Mandelbulb3D에서 열 수 있습니다. (몬테카를로 렌더링은
  자체 기록을 보관하며, 패스의 `.m3c`는 `--m3c`로 저장됩니다.)
* **창 단축키**: 키 조합 하나로 어느 창에서든 메인 창, 내비게이터, 애니메이션,
  공식, 조명, 후처리, 몬테카를로 창을 앞으로 가져옵니다(Ctrl+1 – Ctrl+7). Ctrl+W는
  현재 창을 닫습니다(macOS에서는 Cmd). *환경 설정 > 단축키*에서 직접 지정할 수
  있습니다. 칸을 클릭하고 키를 누르면 되며, Backspace는 단축키를 지웁니다.
* **여러분의 언어로 된 인터페이스**: 창의 모든 텍스트는 언어 파일에서 옵니다. 열두
  개 언어가 함께 제공되며, 새 언어의 파일은 프로그램이 만들어 주므로 번역만 하면
  됩니다.

  <details><summary>자세히 보기</summary>

  메인 창 제목 표시줄 오른쪽 끝의 언어 배지는 인터페이스 언어(`EN`)를 보여 주며,
  클릭하면 언어 창이 열립니다. 언어는 프로그램 옆 `Languages` 폴더에 있는 텍스트
  파일입니다. 함께 제공되는 언어: English, Deutsch, Français, Italiano, Nederlands,
  Polski, Kaszëbsczi, हिन्दी, 日本語, 한국어, 简体中文, 繁體中文(각각 국기 포함.
  동아시아 언어는 문자를 위해 Noto Sans CJK의 일부를 함께 제공하고, 힌디어는 Noto
  Sans Devanagari를 함께 제공하며 결합 문자와 모음 기호는 rustybuzz 텍스트 셰이퍼로
  조판합니다).
  번역은 기계로 만들었고 구조만 검사했을 뿐 원어민이 검토하지는 않았습니다. 수정은
  언제든 환영하며, 수정 하나는 파일의 한 줄입니다. 번역하려면 언어 이름을 입력하고
  *언어 파일 만들기*를 누르십시오. 프로그램이 모든 텍스트를 영어로 한 줄에 하나씩
  담은 `Languages/<name>.lng`를 만듭니다(`Button2.Caption = Calculate
  3D`). 텍스트 편집기로 `=` 오른쪽의 텍스트를 번역하고 *다시 불러오기*를 누르면
  재시작 없이 창에 바로 반영됩니다. 빠진 줄은 영어 텍스트로 남습니다. *저장 + 종료*를
  누르면 다음 실행 때도 그 언어를 사용합니다.

  </details>
* **Linux, Windows, macOS**(Intel 및 Apple Silicon).
* 창이 필요 없는 모든 작업을 위한 **명령줄 프로그램**.
* **아주 큰 그림을 한 장으로**: 15000 x 20000 픽셀 이상도 메모리 부족 없이
  렌더링하며, 피사계 심도는 모든 프로세서 코어를 사용합니다(24 스레드에서 0.2.45
  이전보다 11배 빠르고, 그림은 같습니다).
* **16비트 그림:** 색상당 256단계 대신 65,536단계를 담은 PNG 파일로, 편집해도
  띠(밴딩)가 생기지 않습니다.

  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="300" alt="16비트 그림"></a>

  <details><summary>자세히 보기</summary>

  MB3D는 8비트 그림, 즉 색상당 256단계로 저장합니다. 보기에는 충분하지만, 나중에
  밝기, 대비, 커브 등으로 그림을 편집하면 배경이나 안개처럼 매끄러운 부분이 눈에
  보이는 띠로 갈라집니다. **그림 저장 ▸ PNG**에는 두 번째 파일 형식인 **PNG 16비트
  색상**이 있습니다. 프로그램이 계산한 색상을 256단계로 반올림하기 전 그대로, 모든
  그림자, 반사, 피사계 심도와 함께 저장합니다. 파일은 똑같아 보이지만 편집해도
  매끄러움이 유지됩니다.
  
  <a href="../img/png16_compare.png"><img src="../img/png16_compare.png" width="760" alt="대비를 크게 높인 뒤의 배경 그라데이션: 8비트에서는 계단이 생기고 16비트에서는 매끄러움"></a>
  
  <a href="../img/save_png16.png"><img src="../img/save_png16.png" width="420" alt="'PNG 16비트 색상'이 있는 저장 대화 상자"></a>

  </details>
* 몬테카를로를 비롯한 모든 그림을 위한 **노이즈 필터**. GIMP에서 알려진 DCT
  노이즈 제거와, 몬테카를로 렌더링에는 신경망 필터인 Intel Open Image Denoise도
  있습니다.

  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="300" alt="노이즈 필터"></a>

  <details><summary>자세히 보기</summary>

  **노이즈 필터**는 그림의 입자감(그레인)을 없앱니다. GIMP의 *Filters ▸ Enhance ▸
  Noise Reduction ▸ Denoise DCT*에 있는 **DCT 노이즈 제거**로, GIMP의 필터 중 이
  그림들에 가장 잘 맞았습니다(아래 크레디트 참고).
  
  * **몬테카를로 창 ▸ -노이즈**(MB3D가 숨겨 두었던 버튼): 표시된 그림과 저장되는
    그림을 픽셀당 광선 수에 따른 강도로 필터링합니다. 렌더링 자체는 필터 없이
    계속되며, 다음 갱신 때 다시 필터 없는 그림이 표시됩니다.
  * **후처리 ▸ 노이즈 필터**: *지금 필터링*은 메인 창의 그림을 필터링하고, *새
    이미지를 자동으로 필터링*은 새 그림마다 필터링합니다. *강도*는 제거할 노이즈의
    세기입니다(4는 약한 편이며, 높일수록 그레인과 함께 미세한 디테일도 더 많이
    사라집니다).
  
  몬테카를로 그림에 그레인이 가장 많습니다. 픽셀당 512개 광선으로 렌더링한 그림과
  비교했을 때, 13번의 시험(장면 넷, 4–32 광선) 모두에서 필터링한 그림이 필터링하지
  않은 그림보다 더 가까웠습니다. 4 광선에서 최대 23 %, 8 광선에서 5–10 %, 그레인이
  거의 남지 않는 32 광선에서 1–3 %였습니다.
  
  <a href="../img/mc_noise.png"><img src="../img/mc_noise.png" width="760" alt="픽셀당 4 광선의 몬테카를로 그림, 같은 그림에 노이즈 필터를 적용한 것, 그리고 512 광선의 그림"></a>
  
  <a href="../img/postpro_noise.png"><img src="../img/postpro_noise.png" width="260" alt="후처리: 노이즈 필터 섹션"></a>

  **신경망 노이즈 필터**는
  [Intel Open Image Denoise](https://www.openimagedenoise.org)로, Blender의 Cycles를
  비롯한 여러 패스 트레이서에서 쓰이며 렌더링된 그림으로 학습된 필터입니다.
  렌더링의 광선 평균값을(노출과 채도 적용 전에) 필터링하므로 그림의 밝기가
  유지됩니다. Intel의 라이브러리는 프로그램과 함께 제공되며(`oidn` 폴더, .deb와
  .rpm에서는 `/usr/lib/mb3d-rs/oidn`), `MB3D_OIDN`으로 다른 라이브러리를 지정할 수
  있습니다.

  * **몬테카를로 창 ▸ -노이즈 AI**: *-노이즈*처럼 표시된 그림과 저장되는 그림을
    필터링하며, 렌더링은 필터 없이 계속됩니다.
  * **명령줄**: `mb3d montecarlo scene.m3p --rays 8 --denoise-ai`.
  * 640 x 480 그림에 프로세서로 0.2–0.6초가 걸립니다.

  512 광선 그림과의 차이(모든 픽셀의 평균, 낮을수록 좋음):

  | 장면 | 4 광선 | DCT 필터 | 신경망 필터 | 8 광선 | DCT 필터 | 신경망 필터 |
  |---|---|---|---|---|---|---|
  | TreePlanet | 1.89 % | 1.52 % | **1.23 %** | 0.88 % | 0.82 % | **0.79 %** |
  | Cutted sphere over carpet | 5.30 % | 3.87 % | **2.34 %** | 2.80 % | 2.37 % | **1.66 %** |
  | Julia on box | 1.61 % | **1.01 %** | 1.08 % | 0.98 % | **0.81 %** | 0.84 % |
  | MengerTrees | 1.95 % | 1.54 % | **1.26 %** | 1.15 % | 1.05 % | **0.94 %** |

  네 장면 중 세 장면에서 신경망 필터가 가장 가깝고, Julia 집합의 미세한 그레인에서는
  DCT 필터가 조금 더 가깝습니다. 효과는 광선이 적을 때 그레인이 가장 많이 남는
  유리와 거울에서 가장 잘 드러납니다.

  <a href="../img/mc_noise_ai.jpg"><img src="../img/mc_noise_ai.jpg" width="760" alt="픽셀당 4 광선의 몬테카를로 그림, DCT 필터 적용, 신경망 필터 적용, 그리고 512 광선의 그림"></a>

  </details>
* 그림과 애니메이션의 **투명 배경**.

  <a href="../img/transparent.png"><img src="../img/transparent.png" width="300" alt="투명 배경"></a>

  <details><summary>자세히 보기</summary>

  그림과 애니메이션 프레임을 **투명 배경**으로 저장해, 프랙탈을 다른 그림 위에
  얹거나 동영상 편집기에 넣을 수 있습니다. **그림 저장 ▸ PNG ▸ PNG 투명 배경**,
  그리고 애니메이션 메이커의 **투명 배경**(PNG 프레임)입니다. 안티에일리어싱(1:2
  이상의 보기 배율, 또는 `--aa`)을 쓰면 가장자리가 부드럽고, 이전 배경색의
  테두리가 남지 않습니다. 합성용 깊이 그림은 이전부터 있었습니다(*Z-buffer도
  저장*, `--depth`).
  
  <a href="../img/transparent.png"><img src="../img/transparent.png" width="760" alt="배경이 있는 그림과, 체커보드 위에 표시한 투명 배경 그림"></a>

  </details>
* **애니메이션 동영상:** 프로그램에서 바로 만드는 AVI 파일, 또는 작은 MP4를 위한
  ffmpeg 명령.

  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="300" alt="애니메이션 동영상"></a>

  <details><summary>자세히 보기</summary>

  애니메이션 메이커에서 **비디오(AVI)도 만들기**를 켜면 프레임이 렌더링될 때 설정한
  **초당 프레임 수**로 동영상을 만듭니다. 출력 폴더의 프레임 옆에
  `<project name>.avi`가 생기며, 프레임은 그대로 남습니다. VLC와 mpv에서 재생되고
  동영상 편집기(Kdenlive, Shotcut, DaVinci Resolve)에서 열립니다. 프레임마다 완전한
  그림으로 저장되므로 파일이 큽니다. 편집과 보관에는 좋지만 공유에는 덜
  적합합니다.
  
  공유용 작은 동영상이 필요하면 **FFmpeg 명령**이 MP4(투명 배경이면 WebM)를 만드는
  명령을 클립보드와 메시지에 넣어 줍니다. [ffmpeg](https://ffmpeg.org)를 설치하고,
  프레임이 다 만들어지면 터미널에 명령을 붙여 넣으면 동영상이 만들어집니다. 이
  프로그램은 스스로 다른 프로그램을 실행하지 않습니다.
  
  <a href="../img/anim_video.png"><img src="../img/anim_video.png" width="760" alt="비디오(AVI)도 만들기, 초당 프레임 수, FFmpeg 명령이 있는 애니메이션 메이커"></a>

  </details>
* 열기 대화 상자의 **파라미터 파일 미리보기**.

  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="300" alt="파라미터를 열 때의 미리보기"></a>

  <details><summary>자세히 보기</summary>

  **열기 ▸ m3p 열기**와 **m3i 열기**는 클릭한 파라미터 파일을 열기 전에 그 크기와
  공식과 함께 작은 그림으로 보여 줍니다. 화살표 키로 미리보기를 보며 목록을 이동할
  수 있습니다. 그림은 백그라운드에서 프로세서로 계산되므로 대화 상자가 멈추지
  않으며, 각 파일의 미리보기는 프로그램이 실행되는 동안 보관됩니다.
  
  <a href="../img/open_preview.png"><img src="../img/open_preview.png" width="760" alt="파라미터 파일 미리보기가 표시된 열기 대화 상자"></a>

  </details>
* **내비게이터: 정확한 값 입력과 시점 초기화.** 값을 클릭해 입력하고, 내비게이터를
  열었을 때의 시점으로 돌아갈 수 있습니다.

  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="300" alt="내비게이터: 정확한 값 입력과 시점 초기화"></a>

  <details><summary>자세히 보기</summary>

  * 내비게이터 패널의 **값을 클릭**하면(Julia 값, 공식 값, 4D 회전, 기타 값) MB3D처럼
    정확한 숫자를 입력할 수 있습니다(각도는 도 단위). 슬라이더는 그 값에서부터
    이어집니다.
  * 줌 단계가 너무 거칠 때는 **줌을 클릭**해 입력할 수 있습니다(MB3D 이슈 목록에
    있던 요청). 입력한 줌은 유지됩니다(*고정 줌 및 단계*).
  * **시점 초기화**는 내비게이터가 메인 창에서 가져온 위치, 방향, 줌으로
    되돌아갑니다. 조정한 값은 그대로 남습니다.
  
  <a href="../img/navigator_values.png"><img src="../img/navigator_values.png" width="640" alt="공식 값과, 클릭한 값의 입력 상자가 표시된 내비게이터"></a>

  </details>
* **새 공식 SphereTree:** 서로 맞닿은 구가 점점 작게 가지를 치는 나무로, Tom Lowe가
  만들었습니다.

  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="300" alt="새 공식: SphereTree"></a>

  <details><summary>자세히 보기</summary>

  **SphereTree**는 Mandelbulb3D에 없는 공식입니다. 서로 맞닿은 구들이 점점 더 작은
  구로 가지를 치는 나무로, 거품이나 산호처럼 보입니다. Mandelbox를 고안한 Tom
  Lowe가 2020년에 발표했습니다(아래 크레디트 참고). 공식 목록에서 dIFS 공식들과
  함께 있으며 설정은 네 가지입니다.
  
  * **PackRatio**(0–1.22): 구의 크기입니다. 1이면 구가 딱 맞닿고, 1보다 크면
    바깥쪽 구가 떨어져 나갑니다.
  * **Levels**: 나무의 깊이입니다(대부분의 그림에는 6이면 충분합니다).
  * **Rotated**: 1이면 단계마다 12분의 1 바퀴씩 돌려, 나무가 구 덩어리로 된 둥근
    공이 됩니다.
  * **Colour scale**: 나무의 가지마다 고유한 색이 있습니다. 조명의 색상 설정에서
    *2번째 선택*(오빗 트랩)을 고르면 보이며, 이 설정은 색을 색상 그라데이션에 펼쳐
    놓습니다.
  
  단독으로도, dIFS 변환 뒤에서도 동작하며(세 번째 그림은 *tilingbrickIFS*로
  반복한 것), 프로세서뿐 아니라 그래픽 카드에서도 계산됩니다. 반복 횟수는 1로
  설정하십시오. 이 공식은 나무 전체를 한 번에 계산합니다. SphereTree가 들어간
  파라미터는 Mandelbulb3D가 아닌 이 프로그램에서만 열립니다.
  
  <a href="../img/spheretree_ball.jpg"><img src="../img/spheretree_ball.jpg" width="760" alt="몬테카를로 렌더러로 해 질 녘에 렌더링한 SphereTree 공(Rotated)"></a>
  
  | <a href="../img/spheretree_close.jpg"><img src="../img/spheretree_close.jpg" width="380" alt="SphereTree의 구 거품을 가까이에서 본 모습"></a> | <a href="../img/spheretree_swarm.jpg"><img src="../img/spheretree_swarm.jpg" width="380" alt="tilingbrickIFS로 반복한 SphereTree: 구 나무의 무리"></a> |
  |:---:|:---:|

  </details>
* **새 공식 DomeTree:** 돔에서 더 작은 돔들이 고리 모양으로 돋아나고, 각 돔에 다시
  고리가 생기는 형태로, Tom Lowe가 만들었습니다.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="300" alt="새 공식: DomeTree"></a>

  <details><summary>자세히 보기</summary>

  **DomeTree**는 SphereTree에 이은 Tom Lowe의 두 번째 나무입니다(크레디트 참고).
  공의 한쪽 면에서 돔들이 고리 모양으로 자라나고, 각 돔에 다시 더 작은 고리가
  단계마다 생기며, 원 반전으로 만들어집니다. 공식 목록에서 dIFS 공식들과 함께
  있으며 설정은 다섯 가지입니다.

  * **Levels**: 나무의 깊이입니다(16이면 충분합니다).
  * **Children**: 각 고리의 돔 수로, 4–10입니다.
  * **Bend angle**(0.3–1.3이 가장 좋음): 돔이 부풀어 오르는 정도입니다. 낮은 값은
    납작한 원반, 높은 값은 둥근 돔이 됩니다. 약 1.3을 넘으면 표면이 거칠어집니다.
  * **Colour scale**: 돔마다 고유한 색이 있습니다. 조명의 색상 설정에서 *2번째
    선택*(오빗 트랩)과 넓은 색상 범위(예: 0–100)를 고르면 보입니다.
  * **Start curved**: 1(기본값)이면 공에서 시작하고, 0이면 첫 고리가 자라나는 평평한
    바닥에서 시작합니다.

  작은 *Raystep*(0.2, 자식이 9개나 10개면 0.05)을 사용하십시오. TGlad의
  Fragmentarium 파일과 마찬가지로 이 형태의 거리 추정은 대략적입니다. SphereTree처럼
  그래픽 카드와 프로세서에서 실행되고, 반복은 1회면 되며, 이 공식이 들어간
  파라미터는 이 프로그램에서만 열립니다.

  <a href="../img/dometree_mc.jpg"><img src="../img/dometree_mc.jpg" width="760" alt="몬테카를로 렌더러로 렌더링하고 돔마다 색을 입힌, 자식 8개의 DomeTree"></a>

  | <a href="../img/dometree_5.jpg"><img src="../img/dometree_5.jpg" width="250" alt="DomeTree, 자식 5개, bend angle 1.2"></a> | <a href="../img/dometree_flat.jpg"><img src="../img/dometree_flat.jpg" width="250" alt="DomeTree, 자식 6개, bend angle 0.5: 납작한 돔"></a> | <a href="../img/dometree_10.jpg"><img src="../img/dometree_10.jpg" width="250" alt="DomeTree, 자식 10개: 돔 사이의 레이스 무늬와 큰 공"></a> |
  |:---:|:---:|:---:|

  </details>
* **새 공식 UlamSpiral:** 소수로 이루어진 울람 나선을 3D로 만든 것으로, 도시,
  스카이라인, 구, 블록 형태가 있습니다.

  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="300" alt="새 공식: UlamSpiral"></a>

  <details><summary>자세히 보기</summary>

  1963년, 수학자 스타니스와프 울람(Stanisław Ulam)은 한 학회에서 "길고 몹시
  지루한 발표"를 듣다가 모눈종이에 낙서를 했습니다. 처음에는 체스 문제를 만들
  생각이었지만, 대신 칸에 1, 2, 3, …을 나선 모양으로(1을 가운데에) 번호를 매기고
  소수에 동그라미를 쳤습니다. 소수들은 마치 어떤 규칙을 따르는 것처럼 대각선
  위에 늘어섰고, 컴퓨터로 수백만 개를 그려도 여전히 그렇습니다(울람은 로스앨러모스의
  MANIAC II로 100,000개를 시험했고, 마틴 가드너가 1964년 Scientific American 표지에
  이 그림을 실었습니다). 오늘날까지 아무도 이를 완전히 설명하지 못했습니다.
  
  <a href="../img/ulam_2d.png"><img src="../img/ulam_2d.png" width="420" alt="울람 나선: 정사각형 나선 위의 1부터 78,961까지의 수 중 소수, 모서리에 나선의 시작 부분을 그린 그림"></a>
  
  **UlamSpiral**은 그 나선을 3D로, 수 하나마다 형태 하나씩, 네 가지 형식(*Mode*)으로
  만듭니다.
  
  * **0, 소수 탑**: 모든 수에 판이 하나씩 있고, 소수마다 탑이 섭니다. 소수의
    대각선이 탑의 줄이 됩니다.
  * **1, 약수 스카이라인**: 모든 수에 기둥이 하나씩 있고, 약수가 많을수록
    높습니다. 소수는 그 사이의 낮은 거리가 됩니다.
  * **2, 약수 구**: 모든 수에 구가 하나씩 있고, 약수가 많을수록 큽니다(소수는 작은
    구입니다).
  * **3, 소수 블록**: 수를 정사각형 대신 정육면체 껍질에 감고, 소수마다 정육면체를
    놓습니다.
  
  *Rings*는 나선의 크기(60 고리는 14,641까지의 수), *Height*는 탑이나 기둥의 높이,
  *Gap*은 그 사이의 간격입니다. 색상 설정에서 *2번째 선택*(오빗 트랩)을 고르면 색이
  약수의 개수를 따르므로 소수가 고유한 색을 갖습니다. SphereTree처럼 그래픽 카드와
  프로세서에서 실행되고, dIFS 변환 뒤에서도 동작하며, 반복은 1회면 되고, 이 공식이
  들어간 파라미터는 이 프로그램에서만 열립니다.
  
  <a href="../img/ulam_city.jpg"><img src="../img/ulam_city.jpg" width="760" alt="UlamSpiral, 소수 탑: 소수의 대각선 줄이 도시가 된 모습, 몬테카를로 렌더러에서 안개, 하드 섀도, 피사계 심도 적용"></a>
  
  | <a href="../img/ulam_skyline.jpg"><img src="../img/ulam_skyline.jpg" width="250" alt="UlamSpiral, 약수 스카이라인"></a> | <a href="../img/ulam_spheres.jpg"><img src="../img/ulam_spheres.jpg" width="250" alt="UlamSpiral, 약수 구"></a> | <a href="../img/ulam_block.jpg"><img src="../img/ulam_block.jpg" width="250" alt="UlamSpiral, 정육면체 껍질 위의 소수 블록"></a> |
  |:---:|:---:|:---:|

  </details>
* **중지한 렌더링은 멈춘 곳에서 이어집니다.** 렌더링이 계산한 내용은 중지했을 때뿐
  아니라 프로그램을 닫거나 프로그램이 충돌했을 때, 컴퓨터가 꺼졌을 때도
  보관됩니다(5분마다 디스크에 저장됩니다). 대상은 3D 계산, 몬테카를로(패스 도중도
  포함), 진행 중인 애니메이션의 프레임이나 일괄 처리의 그림, 복셀 내보내기, Bulb
  Tracer 메시, 큰 렌더링 타일, 스웜 렌더링입니다(작업을 만든 컴퓨터에서 중지하면
  완성된 부분이 모두 보관되고, 각 컴퓨터는 작업 중이던 부분을 보관합니다). 같은
  파라미터로 3D 계산과 몬테카를로를 실행하면 이어서 할지 묻고, 나머지는 저절로
  이어집니다. **이어서** 버튼(열기 탭)은 미완성 렌더링을 나열해 이어 하거나 삭제할
  수 있게 합니다. 명령줄에서는 같은 명령이 저절로 이어서 실행되며(`--tiles`의 그림,
  `montecarlo`, `animate`, `batch`, `voxel`, `mesh`도 포함), `--fresh`는 처음부터
  다시 시작하고 `mb3d
  resume`은 목록을 보여 줍니다. 이어서 완성한 렌더링은 한 번에 계산한 것과 같습니다.
* **여러 컴퓨터가 나눠 하는 렌더링**, 즉 "스웜": 공유 폴더 하나를 볼 수 있는
  컴퓨터들이 그림, 큰 렌더링, 몬테카를로 그림, 애니메이션을 함께 렌더링합니다.
  [여러 컴퓨터에서 렌더링하기](#rendering-on-several-computers-the-swarm)를
  참고하십시오.
* **새 공식 PseudoKleinian:** 홀, 아치, 구로 이루어진 "Kleinian architecture"를
  고유의 정확한 거리를 가진 하나의 공식으로 만든 것입니다.

  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="300" alt="새 공식: PseudoKleinian"></a>

  <details><summary>자세히 보기</summary>

  "Kleinian architecture"로 알려진 홀, 아치, 기둥, 겹겹이 들어찬 구는 가장 사랑받는
  3D 프랙탈 그림 중 하나입니다. MB3D에서는 설정을 세심하게 맞춘 여러 공식의
  하이브리드로만 만들 수 있었는데, 이것이 Theli-at이 2011년에 찾아낸 방법입니다.
  이후 knighty가 같은 것을 고유의 거리 추정을 가진 하나의 공식으로 작성했고, 그것이
  **PseudoKleinian**입니다. 각 단계에서 점을 상자 안으로 접고, 구에 대해 반전한 뒤
  오프셋을 더하며, 거리는 접기 후의 바닥 평면까지 정확하게 측정합니다. 그래서 벽이
  깔끔하게 나오고 그림이 빠르게 렌더링됩니다. 설정은 knighty의 것입니다. *Levels*(8),
  *Size*(반전 구), *CSize X/Y/Z*(상자. 그의 값 0.924, 0.908, 0.924가 고전적인 홀을
  만들고, 조금만 바꿔도 새로운 건물이 됩니다), *Julia X/Y/Z*(단계마다의 오프셋),
  *Shape*(0은 바닥 평면, 1은 *Offset*을 지나는 *Thickness* 두께의 원기둥-평면, 2는
  둥근 상자), *DE offset*(표면을 속이 비게 함), *Colour scale*(*2번째 선택*, 즉
  오빗 트랩과 함께 사용: 가장 작은 오빗 반지름, 고전적인 Kleinian 띠 무늬).
  
  이 구조는 공간 전체를 채우므로 안에서 바라보게 됩니다. 시점 중심을 원점 근처에,
  카메라를 한두 단위 뒤에 두고 시작해서 내비게이터로 홀 사이를 이동하십시오.
  몬테카를로 렌더러에서는 실내에 하늘빛이 거의 들지 않으므로, 홀 안에 위치 조명을
  두거나(첫 번째 그림) 절단 평면으로 구조를 잘라 위에서 들여다보십시오(두 번째
  그림). 그래픽 카드와 프로세서에서 실행되고, 반복은 1회면 되며, 다른 dIFS
  공식처럼 dIFS 변환 뒤와 DE 조합에서도 동작하고(다만 공간을 채우는 구조 안에서는
  변화가 크지 않습니다), 이 공식이 들어간 파라미터는 이 프로그램에서만 열립니다.
  
  <a href="../img/kleinian_hall.jpg"><img src="../img/kleinian_hall.jpg" width="760" alt="knighty의 시점에서 본 PseudoKleinian: 안쪽의 램프가 비추는 아치와 겹겹이 들어찬 구의 홀, 몬테카를로 렌더러에서 안개, 하드 섀도, 피사계 심도 적용"></a>
  
  <a href="../img/kleinian_top.jpg"><img src="../img/kleinian_top.jpg" width="760" alt="절단 평면으로 잘라 위에서 본 PseudoKleinian: 구의 격자와 그 사이의 칸"></a>

  </details>
* **한 번에 만드는 스테레오 쌍**: 양쪽 눈의 이미지를 렌더링해 나란히 보여 주며,
  평행법이나 교차법으로 볼 수 있습니다. Rathinagiri S
  ([SRathinaGiri](https://github.com/SRathinaGiri))가 기여했습니다.

  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="300" alt="평행법으로 보도록 나란히 놓은 Dainbramage의 Hydra 스테레오 쌍"></a>

  <details><summary>자세히 보기</summary>

  **스테레오 ▸ 스테레오 쌍 렌더링**은 탭의 스테레오 설정으로 왼쪽 눈, 이어서 오른쪽
  눈 이미지를 렌더링해 메인 창에 나란히 보여 줍니다. 평행법으로 보도록(왼쪽
  이미지가 왼쪽에) 배치되며, **교차시 미리보기**는 교차법으로 보도록 좌우를
  바꿉니다. 이것은 미리보기입니다. 조명과 색상은
  다시 렌더링하지 않고는 나중에 바꿀 수 없으며, 타일에서는 동작하지 않습니다. 한쪽
  눈 이미지용 버튼도 그대로 있습니다.
  
  <a href="../img/stereo_pair.jpg"><img src="../img/stereo_pair.jpg" width="760" alt="평행법으로 보도록 나란히 놓은 Dainbramage의 Hydra 스테레오 쌍"></a>

  </details>
* **그림 전체의 남은 시간**: 그림 아래의 막대가 전체 계산 동안 한 번만 차오르고,
  그 옆에 그림자, 오클루전, 반사를 포함해 그림이 완성될 때까지의 남은 시간이
  표시됩니다.

  <a href="../img/time_left.png"><img src="../img/time_left_bar.png" width="300" alt="진행 막대와 전체 남은 시간"></a>

  <details><summary>자세히 보기</summary>

  MB3D의 막대는 단계마다 처음부터 다시 시작합니다. 여기서는 전체 계산을
  보여 줍니다. 주 렌더링, 이어서 하드 섀도와 앰비언트 오클루전, 반사, 피사계
  심도입니다. 주 렌더링 이후 단계에 걸리는 시간은 장면이 요구하는 것과 주
  렌더링이 진행된 방식으로부터 추정하며, 이 추정은 366개 장면의 약 1400회
  렌더링으로 맞추었습니다. 다시 계산하는 그림은 지난번에 걸린 시간을 예상합니다.
  
  새 공식을 쓰는 첫 그림은 그래픽 카드에서 셰이더가 컴파일되기를 기다립니다. 이때
  줄에는 남은 시간과 함께 *셰이더 컴파일 중*이 표시되며, 남은 시간은 프로세서의
  속도와, 셰이더가 제때 준비될 경우 카드의 합류를 반영합니다. 자세한 내용은
  [TECHNICAL.md](../TECHNICAL.md#time-left-and-the-progress-bar)에 있습니다.
  
  <a href="../img/time_left.png"><img src="../img/time_left.png" width="640" alt="하드 섀도와 앰비언트 오클루전 계산 중의 메인 창과 전체 남은 시간"></a>

  </details>
* 제목 표시줄의 시계 아이콘 뒤에 있는 **History**: 계산한 모든 그림과 그 썸네일,
  열거나 저장한 모든 파일을 목록, 행, 원하는 크기의 타일로 보여 주며, 날짜별로
  묶거나 한 줄로 이어서 볼 수 있습니다. 검색하고, 정렬하고, 서로 다른 그림만
  표시하고, 더블클릭으로 어떤 항목이든 불러올 수 있습니다. 수십만 개 항목의
  기록도 바로 열립니다.
* 하드 섀도와 반사 후에도 **조명은 실시간으로 반영됩니다**. 조명을 옮기거나 색을
  바꾸면 Mandelbulb3D처럼 그림이 바로 바뀝니다.

  <a href="../img/lighting_live.png"><img src="../img/lighting_live_small.png" width="300" alt="조명을 바꾼 직후의 메인 창과 조명 창"></a>

  <details><summary>자세히 보기</summary>

  먼저 기본으로 렌더링한 다음 조명 위치와 색을 시험해 보십시오. 조명 창에서
  바꿀 때마다 Mandelbulb3D처럼 계산된 그림자를 그대로 둔 채 바로 다시 그립니다.
  조명의 색이나 세기를 바꿔도 그림자는 다시 계산하지 않습니다. 옮긴 조명의
  그림자, 반사, 피사계 심도는 Mandelbulb3D처럼 후처리 창의 버튼으로 되살립니다.
  
  중지한 계산은 조명을 바꾼 뒤에도 이어서 할 수 있습니다. 이를 활용한 작업 방법은
  위키의 [조명 작업 흐름](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-ko)을
  참고하십시오. 자세한 내용은
  [TECHNICAL.md](../TECHNICAL.md#lights-volumetric-light-cutting-planes-depth-of-field)에 있습니다.
  
  <a href="../img/lighting_live.png"><img src="../img/lighting_live.png" width="760" alt="조명을 바꾼 직후의 메인 창과 조명 창: 그림이 이미 더 밝아짐"></a>

  </details>

<a id="download"></a>

## 다운로드

최신 릴리스의 바로 실행할 수 있는 프로그램입니다. 설치할 필요가 없습니다.

| 시스템 | 다운로드 |
|---|---|
| Windows(64비트) | [mb3d-rs-windows-x64.zip](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-windows-x64.zip) |
| Linux(64비트) | [mb3d-rs-linux-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.tar.gz) |
| Debian, Ubuntu, Mint(다른 프로그램처럼 설치) | [mb3d-rs-linux-x64.deb](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.deb) |
| Fedora, RHEL, Rocky, openSUSE(다른 프로그램처럼 설치) | [mb3d-rs-linux-x64.rpm](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-linux-x64.rpm) |
| macOS, Apple silicon(M1 이상) | [mb3d-rs-macos-arm64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-arm64.tar.gz) |
| macOS, Intel | [mb3d-rs-macos-x64.tar.gz](https://github.com/thr32423/mb3d-rs/releases/latest/download/mb3d-rs-macos-x64.tar.gz) |

압축을 풀고, 풀린 폴더에서 **Mandelbulb3D**(Windows에서는 `Mandelbulb3D.exe`)를
실행하십시오. 폴더는 MB3D 설치 폴더처럼 구성되어 있으며, MB3D의 공식, 맵, 예제
파라미터 파일 80개가 들어 있습니다.

* Windows에 "Windows의 PC 보호"("Windows protected your PC")가 표시되면 *추가 정보*
  ▸ *실행*을 클릭하십시오.
* macOS는 서명되지 않은 다운로드 프로그램을 차단합니다. 풀린 폴더에서
  `xattr -dr com.apple.quarantine .`을 한 번 실행하십시오.

**Linux와 macOS의 다른 방법:** 직접 다운로드하는 대신, 다음 한 줄로 최신 릴리스를
`~/mb3d-rs`에 받을 수 있습니다(나중에 다시 실행하면 업데이트됩니다. 스크립트
[download.sh](../../download.sh)는 프로그램 폴더에도 있습니다).

```sh
curl -fsSL https://raw.githubusercontent.com/thr32423/mb3d-rs/master/download.sh | sh
```

이 스크립트는 압축을 풀기 전에 다운로드를 검사하고, **Mandelbulb3D**를 응용
프로그램 메뉴에 넣으며(Linux), 어느 폴더에서나 입력할 수 있는 `Mandelbulb3D`와
`mb3d` 명령을 만듭니다. `sh ~/mb3d-rs/download.sh
--uninstall`은 메뉴 항목과 명령을 다시 없애며, 여러분의 파일은 그대로 남습니다.

Linux 프로그램은 대략 2018년 이후의 배포판(Ubuntu 18.04, Debian 10, Rocky/RHEL 8
이상)에서 실행됩니다.

**다른 프로그램처럼 설치하기:** Debian, Ubuntu, Mint에서는 위의 `.deb` 파일을
여십시오(또는 `sudo apt install ./mb3d-rs-linux-x64.deb`). Fedora, RHEL, Rocky에서는
`.rpm` 파일입니다(`sudo dnf install
./mb3d-rs-linux-x64.rpm`, openSUSE에서는 `sudo zypper install
./mb3d-rs-linux-x64.rpm`).
그러면 Mandelbulb3D가 응용 프로그램 메뉴에 나타나고 `mb3d`가 어느 폴더에서나
동작합니다. 설정과 History는 `~/.config/mb3d-rs`와 `~/.local/share/mb3d-rs`에
보관됩니다.

[시작하기](../../GETTING_STARTED.md)에서 첫 단계를 차례로 안내합니다. 이전 버전과
릴리스 노트는 [릴리스](https://github.com/thr32423/mb3d-rs/releases) 페이지에
있습니다.

<a id="guides-and-help"></a>

## 안내서와 도움말

단계별 안내서는 [위키](https://github.com/thr32423/mb3d-rs/wiki/Home-ko)에 있습니다.

* [설치](https://github.com/thr32423/mb3d-rs/wiki/Installing-ko): Windows, macOS, Linux, 업데이트, 파일이 저장되는 위치
* [조명 작업 흐름](https://github.com/thr32423/mb3d-rs/wiki/Lighting-workflow-ko): 먼저 기본으로 렌더링한 뒤 조명, 그림자, 반사를 시험하기
* [스웜 설정](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-ko): 공유 폴더를 통해 여러 컴퓨터에서 렌더링하기
* [문제 해결 및 FAQ](https://github.com/thr32423/mb3d-rs/wiki/Troubleshooting-and-FAQ-ko): 무언가가 예상대로 동작하지 않을 때

모든 릴리스와 변경 사항: [전체 목록](../UPDATES.md).

<a id="rendering-on-several-computers-the-swarm"></a>

## 여러 컴퓨터에서 렌더링하기(스웜)

공유 폴더 하나를 볼 수 있는 컴퓨터들이 함께 렌더링합니다. 대상은 타일로 나눈
그림, 큰 렌더링, 몬테카를로 그림, 애니메이션, 일괄 처리입니다. 각 컴퓨터는 아무도
맡지 않은 부분을 가져가므로 빠른 컴퓨터가 더 많이 맡으며, 렌더링을 시작한
컴퓨터가 결과를 합칩니다. 공유는 폴더가 담당합니다. 프로그램은 네트워크 연결을
열지 않고 누구와도 통신하지 않으므로, 여러분의 파일 공유만큼 안전합니다.

<a href="../img/swarm_window.png"><img src="../img/swarm_window.png" width="760" alt="도구 탭의 스웜 버튼으로 연 스웜 창이 있는 Mandelbulb3D. 두 컴퓨터가 그림을 타일로 렌더링하는 중: 공유 폴더, 이 컴퓨터 공유, 컴퓨터 목록과 각 컴퓨터가 하는 일, 작업, 그리고 렌더링된 타일·진행 중인 타일·빠진 타일이 표시된 미리보기"></a>

<a id="setting-it-up"></a>

### 설정하기

확인할 점과 함께 단계별로: 위키의 [스웜 설정](https://github.com/thr32423/mb3d-rs/wiki/Swarm-setup-ko).

1. **모든 컴퓨터가 쓸 수 있는 폴더:** NAS의 공유, Windows 공유, Samba 공유 또는
   NFS입니다. Linux에서는 폴더로 마운트하십시오. 예:
   `sudo mount -t cifs //192.168.0.10/Public /mnt/share -o username=me,uid=$(id -u),gid=$(id -g)`.
   파일 관리자에서만 연 공유(`smb://...`)는 프로그램이 사용할 수 있는 폴더가
   아닙니다.
2. **모든 컴퓨터에 같은 버전의 mb3d-rs.** 다른 버전을 쓰는 컴퓨터는 목록에는
   나타나지만 참여하지 않습니다.
3. **Mandelbulb3D에서:** 도구 탭의 **스웜** 버튼을 누릅니다. 공유 폴더를 고르고(모든
   컴퓨터에서 같은 공유. 경로는 달라도 됩니다), 목록에 표시할 컴퓨터 이름을 정한
   뒤 **이 컴퓨터 공유**에 체크하십시오.

이것으로 끝입니다. 상태 줄에 무슨 일이 일어나는지 표시됩니다. 예: *omarchy와(과)
공유 중: 렌더링을 컴퓨터 2대가 나눠 맡습니다*. 스웜에 혼자 있으면 모든 렌더링이
평소처럼 이 컴퓨터에서 이루어집니다.

<a id="what-is-shared"></a>

### 공유되는 작업

*이 컴퓨터 공유*에 체크되어 있는 동안, 이 컴퓨터는 다른 컴퓨터가 폴더에 넣은
부분을 렌더링하며, 다른 컴퓨터가 스웜에 있을 때는 이 컴퓨터의 렌더링도 스웜으로
갑니다.

* **3D 계산**(및 일괄 처리 창) — 그림을 N x M 타일로(스웜 창에서 설정) 나누고, 이
  컴퓨터에서 평소 계산이 제공하는 모든 것과 함께 합칩니다. 조명 변경, 후처리, m3i로
  저장(과 그 메모리. 아래 참고)입니다.
* **큰 렌더링** — 타일이 도착하는 대로 하나씩 저장되며, 그림 전체를 메모리에
  올리지 않습니다. 따라서 100000 x 100000 픽셀 이상의 그림에는 이 방법을 쓰십시오.
* **몬테카를로** — 컴퓨터 한 대일 때와 같습니다. 중지를 누를 때까지 각 컴퓨터가
  자기 블록에 패스를 더하고, 모두의 광선으로 그림이 깨끗해지며, 중지하면 블록들의
  평균을 냅니다. 창의 일괄 모드에서는 블록을 합친 광선 수가 일괄 처리의 광선 수에
  도달하면 멈춥니다.
* **애니메이션 메이커** — 프레임을 하나씩.

창에는 컴퓨터 목록과 각 컴퓨터의 코어, 그래픽 카드, 하는 일, 작업 목록과 진행
상황, 그리고 최신 작업의 미리보기(렌더링된 타일, 진행 막대가 있는 진행 중인 타일,
아직 빠진 타일)가 표시됩니다.

<a id="on-the-command-line"></a>

### 명령줄에서

도움을 줄 모든 컴퓨터에서 폴더를 대상으로 워커를 시작하십시오.

```sh
mb3d swarm worker /mnt/share/mb3d-swarm --name kitchen-pc
```

그런 다음 아무 컴퓨터에서나 `--swarm`과 같은 폴더로 렌더링합니다.

```sh
mb3d big.m3p --tiles 4x3 --swarm /mnt/share/mb3d-swarm
mb3d animate flight.m3a --swarm /mnt/share/mb3d-swarm
mb3d batch *.m3p -o pictures --swarm /mnt/share/mb3d-swarm
mb3d montecarlo scene.m3p --rays 256 --swarm /mnt/share/mb3d-swarm
```

`mb3d swarm list FOLDER`는 스웜에 누가 있는지, 각 컴퓨터가 무엇을 하는지, 작업이
얼마나 진행되었는지 보여 줍니다. 명령줄의 몬테카를로 그림은 각자 고유한 무작위
광선을 가진 블록으로 나뉩니다. 픽셀당 64 광선의 블록 4개가 `--rays 256`으로 요청한
256을 채웁니다.

<a id="good-to-know"></a>

### 알아 두면 좋은 점

* **다른 컴퓨터들이 놀고 있다면**, 즉 이 컴퓨터가 렌더링하는 동안 다른 컴퓨터의
  작업 목록이 비어 있다면: Samba로 폴더를 제공하면서 그 폴더를 직접 사용하는
  컴퓨터는 다른 컴퓨터들이 폴더의 오래된 목록을 계속 갖고 있게 만듭니다.
  `/etc/samba/smb.conf`에서 이 캐싱을 끄십시오. `[global]`에 `smb3 directory leases = no`,
  공유 섹션에 `oplocks = no`와 `level2 oplocks = no`를 넣은 뒤
  `sudo systemctl restart smb`를 실행하고, 다른 컴퓨터에서 공유를 다시 마운트하십시오.
* **메모리:** 이 프로그램에서 그림 하나는 픽셀당 약 20바이트(이미지 버퍼), 메인
  창에서 10바이트를 더 차지하고, 여기에 두 단계 중 더 큰 쪽이 더해집니다. 그림자와
  오클루전을 포함한 주 계산(픽셀당 16바이트와 카드용 1.2 GB), 또는 그리기(픽셀당
  3바이트, 피사계 심도가 있으면 20바이트 더, 그리고 반사의 띠 하나: 최대 3.5 GB.
  띠의 크기는 패스가 시작될 때 남은 메모리의 절반으로 정해지므로 남은 메모리가
  적으면 더 작아집니다)입니다. 12000 x 6000 그림: 6 GB. 그림이 컴퓨터의 남은
  메모리(Linux, Windows, macOS에 문의)보다 많이 필요하면, 3D 계산은 필요한 양과
  남은 양을 알려 주고 시작하기 전에 묻습니다. 계산이 스왑을 일으켜 프로그램이 몇
  분씩 응답하지 않을 수 있기 때문입니다. 해결책은 혼자서든 스웜과 함께든 큰
  렌더링입니다. 타일이 하나씩 저장되고 그림 전체를 메모리에 올리지 않습니다.
  스웜을 쓰는 3D 계산은 평소 계산처럼 이 컴퓨터에 그림을 올려 둡니다(주 계산
  단계는 제외되며, 이는 타일을 맡은 컴퓨터들이 타일 하나만큼의 메모리로 합니다).
  스웜의 컴퓨터는 자기 메모리에 너무 큰 타일을 다른 컴퓨터에 넘깁니다.
* **안전:** 컴퓨터는 작업이 요청한 것만 렌더링하고 작업 폴더에만 씁니다. 작업에
  들어 있는 공식 코드는 모든 공식처럼 공식 에뮬레이터 안에서 실행됩니다.
* **속도:** 타일은 완전한 이미지 버퍼로 전달되며, 아주 큰 그림이면 수백 메가바이트에
  이르므로 빠른 공유가 유리합니다. 지연을 두고 복사하는 동기화 서비스도 동작하지만
  더 느립니다.
* **사라진 컴퓨터:** 어떤 컴퓨터가 렌더링하던 부분은 30초 동안 소식이 없으면 다른
  컴퓨터가 가져갑니다. 소유자가 30분 동안 사라진 작업은 그것을 본 컴퓨터가
  지웁니다. 어떤 컴퓨터에서 실패한 작업은 창에 표시되고 다른 컴퓨터에 맡겨집니다.
* **멈추기:** *이 컴퓨터 공유*의 체크를 풀면 진행 중인 부분을 먼저 마칩니다.
  프로그램을 닫으면 그 부분을 포기하고 다른 컴퓨터가 가져갑니다.
* **작업 중지**(작업을 만든 컴퓨터에서 중지하거나, 그 컴퓨터의 프로그램을 닫거나,
  충돌한 경우): 이미 완성된 모든 부분은 그 컴퓨터에 보관되고, 각 컴퓨터는
  렌더링하던 부분을 보관합니다. 같은 렌더링을 다시 시작하면 빠진 부분만으로 작업을
  만들며, 시작했던 부분을 다시 받은 컴퓨터는 그것을 이어서 합니다. 명령줄에서도
  같습니다. 같은 명령을 다시 실행하십시오. 끝이 정해지지 않은 몬테카를로
  렌더링(창에서 시작한 것)은 이전처럼 중지할 때 블록들을 합칩니다.

<a id="how-the-rendering-works"></a>

## 렌더링의 원리

프랙탈 그림은 픽셀 하나하나 계산됩니다. 픽셀마다 광선을 장면 속으로 따라가 프랙탈
표면에 닿을 때까지 진행하며, 단계마다 공식을 수십 번 계산합니다. MB3D는 이를
프로세서에서 모든 코어를 사용해 수행합니다. mb3d-rs도 똑같이 할 수 있으며, 수천
개의 작은 프로세서를 갖추고 바로 이런 작업을 위해 만들어진 **그래픽 카드**에 일을
넘길 수도 있습니다.

**"계산 장치"** 설정(메인 창의 내부 탭)에서 고릅니다.

| | |
|---|---|
| **자동**(기본값) | 카드와 프로세서가 그림을 나눠 계산합니다. 카드는 위에서부터, 프로세서의 스레드는 아래에서부터 행을 가져가 서로 만날 때까지 진행합니다. 그림자와 앰비언트 오클루전도 같은 방식으로 나누거나, 카드가 확실히 더 빠를 때는 카드 혼자 계산합니다. |
| **CPU** | MB3D처럼 프로세서만 사용합니다. |
| **그래픽 카드** | 카드만 사용합니다(카드용 프로그램이 준비될 때까지 기다립니다). |

공식 조합을 처음 사용할 때는 그래픽 드라이버가 카드용 프로그램, 즉 *셰이더*를
준비해야 하며, 아래에서 측정한 컴퓨터에서는 10–15초가 걸립니다. 자동 모드에서는
프로세서가 셰이더 없이 먼저 시작하고 카드는 준비되면 합류하므로, 작은 그림은 결코
기다리지 않습니다. 준비된 셰이더는 다음을 위해 보관되므로(그래픽 드라이버가 보관하며,
Linux에서는 프로그램 옆 `ShaderCache` 폴더), 같은 공식을 쓰는 다음 그림은 바로
카드에서 시작합니다.

창 제목 오른쪽의 **LED**는 마지막 그림이 어디에서 계산되었는지 알려 줍니다. 초록은
그래픽 카드, 주황은 일부, 빨강은 프로세서입니다. 마우스를 올리면 자세한 내용이
표시됩니다.

[렌더링의 원리](../RENDERING.md)에서 그림과 함께 전체 이야기를 다룹니다. 자동
모드가 처음에 무엇을 하는지, 왜 카드가 항상 더 빠른 선택은 아닌지, LED의 색이
무엇을 뜻하는지 설명합니다.

모든 것이 카드로 갈 수 있는 것은 아닙니다. 일부 공식은 카드용으로 변환할 수 없고,
아주 깊이 확대한 시점은 배정밀도로 계산합니다(일부 카드만 가능). 프로그램이 이
모든 것을 스스로 결정하며, 여러분은 LED로만 확인하게 됩니다.

<a id="speed"></a>

## 속도

모든 측정은 한 컴퓨터에서 했습니다. NVIDIA GeForce RTX 2060을 장착한 AMD Ryzen 9
3900X(12코어, 24스레드), Linux입니다. 원래의 **Mandelbulb3D 1.99sr37**은 같은
컴퓨터에서 Wine으로 실행했고(Windows 프로그램이 그대로, 모든 코어를 사용해 최고
속도로 실행됩니다), *3D 계산*을 클릭한 순간부터 그림이 완성될 때까지 시간을
쟀습니다. 각 파라미터 파일은 세 가지 모두에서 같은 이미지 크기로 렌더링했습니다.
MB3D, 프로세서만 쓰는 mb3d-rs(**CPU**), **자동** 모드(그래픽 카드와 프로세서를 함께
사용)의 mb3d-rs입니다. 자동 모드의 시간은 카드의 셰이더가 이미 준비된 상태에서
파일을 두 번째로 렌더링했을 때의 시간입니다. 새 공식 조합의 첫 렌더링에는 이
카드에서 셰이더 준비에 10–15초가 더 걸립니다.

<a id="mb3ds-80-example-parameter-files"></a>

### MB3D의 예제 파라미터 파일 80개

MB3D와 함께 제공되는(그리고 이 프로그램의 `M3Parameter` 폴더에 있는) 예제 파일로,
각각 자체 이미지 크기로 렌더링했습니다.

MB3D는 파일 하나(*Test8*: "Division by zero" 오류로 멈춤)를 계산하지 못했으므로, 비교는 79개 파일을 기준으로 합니다.

* **프로세서만 쓰는 mb3d-rs가 MB3D보다 빠른** 파일은 79개 중 71개이며, 중앙값은 **1.7배**입니다. 전체 파일을 합치면 MB3D는 18분 21초, 프로세서만 쓰는 mb3d-rs는 17분 10초가 걸렸습니다.
* **자동 모드**는 79개 중 77개 파일에서 MB3D보다 빠릅니다(중앙값 2.1배). 전체 파일 합계는 8분 17초입니다. 30개 파일에서는 카드가 차례를 받기 전에 프로세서가 혼자 끝냈는데, 대부분 몇 초밖에 걸리지 않기 때문입니다. 이 경우 자동 모드는 프로세서에 약간의 부가 시간이 더해진 것일 뿐입니다. 카드는 더 느린 49개 파일에서 합류했고, 그 파일들에서 프로세서보다 중앙값 1.6배 빠르게 그림을 만들었습니다(최대 13배: *Insect Menger*, 63.0초 대신 4.7초).
* 4개 파일에서는 자동 모드가 프로세서 혼자일 때보다 눈에 띄게 느렸습니다(*Ellarien - Shrooms are coming*, *MengerTrees*, *toricaleggs*, *TreePlanet*). 이 장면들에서는 카드가 24개 프로세서 스레드보다 느립니다.
* "카드가 맡은 행"은 자동 모드에서 그래픽 카드가 계산한 그림의 비율입니다(프로세서가 혼자 끝낸 경우 –).

<details>
<summary>파일 80개 전체(클릭하면 열림)</summary>

| 파라미터 파일 | MB3D(Wine) | mb3d-rs CPU | mb3d-rs 자동 | 카드가 맡은 행 | 자동 vs MB3D |
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
| Test8 | MB3D에서 오류 | 0.1 s | 0.3 s |  | – |
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

그림은 같습니다. 프로세서에서 실행한 mb3d-rs는 반올림 오차 범위 안에서 MB3D의
그림을 만들고, 자동 모드의 그림은 프로세서의 그림과 평균적으로 색 범위의 1 %에도
못 미치는 만큼 다릅니다(미세한 디테일의 반짝임이 다른 픽셀에 나타날 수 있습니다.
측정 결과는 [기술 노트](../TECHNICAL.md)에 있습니다).

<a id="the-command-line"></a>

## 명령줄

`mb3d`는 창 없이 렌더링합니다. 프로그램 폴더에서 터미널을 여십시오(Windows에서는
풀린 폴더에서 명령 프롬프트를 열고 `./mb3d` 대신 `mb3d`를 입력합니다). Linux나
macOS에서 한 줄 설치를 했다면 `mb3d`는 어느 폴더에서나 동작하고, `man mb3d`로
설명서를 볼 수 있으며, bash, zsh, fish에서 Tab 키로 옵션이 자동 완성됩니다.

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

그 밖의 모든 기능에는 각자 `--help`가 있는 하위 명령이 있습니다.

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

공식과 맵은 프로그램 옆에서 찾습니다. 다른 MB3D 설치의 공식과 맵은 `--formulas`와
`--maps`로 지정할 수 있습니다.
애니메이션은 MB3D의 방식 그대로입니다. MB3D의 보간을 쓰는 키프레임, `.m3a`
프로젝트 열기와 저장, MB3D처럼 기록되는 프레임 덕분에 중단된 애니메이션을 이어서
만들 수 있고 여러 컴퓨터가 하나를 나눠 렌더링할 수 있습니다.

<a id="good-to-know-1"></a>

## 알아 두면 좋은 점

* **같은 그림이지만 비트 단위로 같지는 않습니다.** MB3D는 옛 x87 연산의 80비트
  정밀도로 계산했고, 이 프로그램은 오늘날의 64비트를 사용합니다. 그림의 모든
  구조는 일치하지만, 개별 픽셀의 색이 마지막 자릿수에서 다를 수 있습니다.
  몬테카를로 렌더링과 MutaGen은 자체 난수를 사용하므로 결과가 MB3D와 같지
  않습니다.
* **그래픽 카드의 그림**은 프로세서의 그림과 조금 다릅니다(단정밀도). 아주 미세한
  디테일에서는 반짝임이 다른 픽셀에 나타날 수 있습니다. 깊은 확대에서는 저절로
  배정밀도로 전환됩니다.
* **프로세서에서의 속도**는 MB3D에 가깝습니다. MB3D의 `.m3f` 파일에 있는 공식의
  기계어 코드를 어떤 프로세서(Apple Silicon 포함)에서도 실행되도록 변환했기 때문에
  약간의 비용이 들지만, 광선 진행(레이 마칭) 자체는 더 빠릅니다. 위의 표를
  참고하십시오. Linux와 Windows에서는 장면의 하이브리드 공식들을 그림이 시작될 때
  하나의 함수로 함께 컴파일하므로(몇 밀리초 안에, Cranelift로), 공식들이 값을
  프로세서의 레지스터로 주고받습니다.
  공식을 도는 더 빠른 루프(모든 시스템)와 함께, 예제 파일 80개의 CPU 렌더링 시간이
  v0.2.74보다 20 % 줄었고(일부는 최대 60 %), 그림은 비트 단위까지 같습니다. 장면은
  컴파일된 하이브리드가 측정상 더 빠를 때만 그것을 사용하며, `MB3D_NO_JIT=1`로 끌 수
  있습니다. macOS에서도 켜져 있지만 아직 시험 중입니다. 풀린 폴더에서
  `./jit_check.sh`로 확인할 수 있습니다(컴파일된 공식을 변환된 공식과 비교하고, 예제
  파일을 켠 상태와 끈 상태로 렌더링합니다. 그림이 같아야 합니다).
* **큰 그림**은 한 장으로(15000 x 20000 그림에는 약 15 GB의 메모리가 필요하고,
  피사계 심도를 쓰면 몇 시간이 걸립니다) 또는 타일로, 여러 컴퓨터에서도
  렌더링할 수 있습니다(`--tiles`와 큰 렌더링 창). 타일 방식은 메모리가 덜
  필요합니다. 3D 계산은 남은 메모리에 들어가지 않는 그림을 시작하지 않고 그렇게
  알려 주며(남은 메모리를 알 수 있는 Linux에서), 스웜의 컴퓨터는 자기 메모리에 너무
  큰 타일을 다른 컴퓨터에 넘깁니다.
* **미완성 렌더링**은 캐시 폴더(`~/.cache/mb3d-rs/resume`,
  Windows에서는 `%LOCALAPPDATA%\mb3d-rs\resume`, 또는 프로그램 옆)에 보관됩니다.
  그림은 픽셀당 18바이트(15360 x 15360 그림은 4.2 GB), 몬테카를로 렌더링은 64바이트를
  차지합니다. 계산된 내용은 바로 기록되고 5분마다 디스크에 반영됩니다. 처음 10초
  안에 중지한 렌더링은 아무것도 보관하지 않습니다. 30일이 지난 미완성 렌더링은
  지워지고, 전체가 50 GB를 넘으면 가장 오래된 것부터 지워집니다. 렌더링은 같은
  파라미터와 같은 프로그램 버전으로만 이어집니다(카드나 프로세서 선택도 같아야
  하며, 몬테카를로 렌더링의 노출과 채도는 바꿔도 됩니다). 몬테카를로 렌더링은 목표
  광선 수(일괄 처리, `--rays`, `--passes`)에 도달하거나 이어서 목록에서 삭제될
  때까지 미완성 상태로 남습니다.
* **더 새로운 공식이나 직접 만든 공식, 맵, 테마, 언어**는
  `~/.local/share/mb3d-rs/M3Formulas`, `.../M3Maps`, `.../Themes`, `.../Languages`에
  넣으십시오(설치 방법과 관계없이. Windows에서는 `%APPDATA%\mb3d-rs\...`). 프로그램과
  함께 제공되는 것에 더해 사용되며, 같은 이름의 제공 파일을 대체합니다. 그래서
  포럼에서 받은 업데이트된 공식이 프로그램 폴더를 건드리지 않고도 적용됩니다.

<a id="updates"></a>

## 업데이트

모든 릴리스와 그 변경 사항은 [UPDATES.md](../UPDATES.md)(영어)에 있습니다.

<a id="for-developers"></a>

## 개발자를 위한 정보

[기술 노트](../TECHNICAL.md)에서는 그래픽 카드를 어떻게 사용하는지, MB3D의 Delphi
소스에서 무엇을 어떻게 이식했는지, MB3D 공식의 기계어 코드를 실행하는 세 가지
방법, 장면 파일 형식과 그 키, MB3D와의 비교 검사, 그리고 소스에서 빌드하는 방법을
설명합니다.

```sh
git clone https://github.com/thr32423/mb3d-rs
cd mb3d-rs
cargo build --release      # target/release/Mandelbulb3D and target/release/mb3d
```

<a id="license"></a>

## 라이선스

MB3D는 LGPL 2.1로 배포됩니다. 이 이식판은 파생 저작물이며 같은 라이선스를
사용합니다.

<a id="credits"></a>

## 크레디트

mb3d-rs의 바탕이 된 사람들과 프로젝트는 [영어 README](../../README.md#credits)에 나와 있습니다.
