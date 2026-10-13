#!/bin/bash
# runs MB3D on $MB3D_WINE_WORK/da/m3p/s$1.m3p, clicks Calculate 3D, waits until the image settles
S=${MB3D_WINE_WORK:-$PWD}
D=$(cd "$(dirname "$0")" && pwd)
i=$1
for p in $(pgrep -f 'Mandelbulb3D.exe'); do [ "$(cat /proc/$p/comm 2>/dev/null)" != bash ] && kill $p; done
sleep 3
cd "${MB3D_WINE_DIR:-$HOME/Mandelbulb3Dv199sr37}" && (wine Mandelbulb3D.exe "Z:${S//\//\\}\\da\\m3p\\s$i.m3p" > $S/wine.log 2>&1 &)
for k in $(seq 1 20); do sleep 3; timeout 20 python3 $D/xw.py list 2>/dev/null | grep -q "'s$i'" && break; done
sleep 5
# the main window's position (Wine places it differently per desktop)
read WX WY < <(timeout 20 python3 $D/xw.py list | sed -n "s/.*'s$i' (\([0-9-]*\), \([0-9-]*\),.*/\1 \2/p")
echo "WX=${WX:-48} WY=${WY:-169}" > $S/da/win_s$i
timeout 20 python3 $D/xw.py click $((${WX:-48}+1000)) $((${WY:-169}+117))
t0=$(date +%s); prev=
for k in $(seq 1 120); do
  sleep 10
  timeout 30 python3 $D/xw.py grab s$i $S/da/mb_s$i.png >/dev/null
  # done: the button says "Calculate 3D" again (not "Stop")
  e=$(magick $S/da/mb_s$i.png -crop 78x38+962+99 +repage $D/btn_ready.png -metric AE -compare -format '%[distortion]' info: 2>/dev/null)
  awk -v e="$e" 'BEGIN{exit !(e != "" && e < 0.003)}' && break
done
echo "s$i: $(( $(date +%s) - t0 )) s"
