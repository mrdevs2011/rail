#!/bin/bash
# usage: bench/css-cost.sh BIN MODE(with|without|nofile) RUN
BIN=$1; MODE=$2; R=$3; L=$MODE-$R; OUT=/tmp/css-bench.txt
export XDG_RUNTIME_DIR=/run/user/$(id -u)
BEFORE=$(ls $XDG_RUNTIME_DIR | grep -E '^wayland-[0-9]+$' | sort)
(cd "$(dirname "$(readlink -f "$0")")" && exec python3 -m http.server 18768 --bind 127.0.0.1 >/tmp/css-srv-$L.log 2>&1) &
SPID=$!
env -u DISPLAY -u WAYLAND_DISPLAY WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman labwc >/tmp/labwc-css-$L.log 2>&1 &
LPID=$!; sleep 4
SOCK=$(comm -13 <(echo "$BEFORE") <(ls $XDG_RUNTIME_DIR | grep -E '^wayland-[0-9]+$' | sort) | head -1)
export WAYLAND_DISPLAY=$SOCK GDK_BACKEND=wayland RAIL_SOFTWARE_RENDER=1 RAIL_DATA_DIR=/tmp/rail-css-data-$L
rm -rf $RAIL_DATA_DIR; mkdir -p $RAIL_DATA_DIR/.railassets/customize
[ "$MODE" = "without" ] && : > $RAIL_DATA_DIR/.railassets/customize/html.css
$BIN "http://127.0.0.1:18768/css-cost.html" >/tmp/rail-css-run-$L.log 2>&1 &
RPID=$!; sleep 8; $BIN "http://127.0.0.1:18768/css-cost.html" >/dev/null 2>&1; for i in $(seq 1 45); do grep -q "med=" /tmp/css-srv-$L.log 2>/dev/null && break; sleep 1; done; sleep 1
echo "$L $(grep -o 'med=[^ ]*' /tmp/css-srv-$L.log | head -1 | sed 's/HTTP.*//') css_bytes=$(stat -c %s $RAIL_DATA_DIR/.railassets/customize/html.css 2>/dev/null)" >> $OUT
kill $RPID 2>/dev/null; sleep 1; kill $LPID $SPID 2>/dev/null
