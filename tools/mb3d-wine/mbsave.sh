#!/bin/bash
# renders scratchpad/da/m3p/s$1.m3p in MB3D and saves its .m3i as da/mb_s$1.m3i
S=${MB3D_WINE_WORK:-$PWD}
D=$(cd "$(dirname "$0")" && pwd)
i=$1
$D/mbrun.sh $i
. $S/da/win_s$i
timeout 20 python3 $D/xw.py click $((WX+236)) $((WY+43)); sleep 1.5
timeout 20 python3 $D/xw.py click $((WX+194)) $((WY+75)); sleep 3
X=$((963+375)); Y=$((568+445))
timeout 20 python3 $D/xw.py click $X $Y; sleep 0.3
timeout 20 python3 $D/xw.py click $X $Y; sleep 0.1
timeout 20 python3 $D/xw.py click $X $Y
sleep 0.5
rm -f $S/da/mb_s$i.m3i
timeout 60 python3 $D/xw.py type "Z:${S//\//\\}\\da\\mb_s$i.m3i" enter
for k in $(seq 1 20); do [ -s $S/da/mb_s$i.m3i ] && break; sleep 1; done
ls -la $S/da/mb_s$i.m3i
for p in $(pgrep -f 'Mandelbulb3D.exe'); do [ "$(cat /proc/$p/comm 2>/dev/null)" != bash ] && kill $p; done
sleep 2
