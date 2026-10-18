#!/bin/bash
# usage: bench/theme-warnings.sh BIN LABEL
BIN=$1; L=$2
export XDG_RUNTIME_DIR=/run/user/$(id -u)
BEFORE=$(ls $XDG_RUNTIME_DIR | grep -E '^wayland-[0-9]+$' | sort)
env -u DISPLAY -u WAYLAND_DISPLAY WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman labwc >/tmp/labwc-theme-$L.log 2>&1 &
LPID=$!; sleep 4
SOCK=$(comm -13 <(echo "$BEFORE") <(ls $XDG_RUNTIME_DIR | grep -E '^wayland-[0-9]+$' | sort) | head -1)
export WAYLAND_DISPLAY=$SOCK GDK_BACKEND=wayland RAIL_SOFTWARE_RENDER=1 RAIL_DATA_DIR=/tmp/rail-theme-data-$L
rm -rf $RAIL_DATA_DIR; mkdir -p $RAIL_DATA_DIR
$BIN >/tmp/rail-theme-run-$L.log 2>&1 &
RPID=$!; sleep 12
grim /tmp/rail-theme-$L.png 2>/tmp/grim-$L.err
echo "sock=$SOCK alive=$(kill -0 $RPID 2>/dev/null && echo yes || echo no) theme_warnings=$(grep -c 'Theme parser' /tmp/rail-theme-run-$L.log)" > /tmp/rail-theme-$L.result
kill $RPID 2>/dev/null; sleep 1; kill $LPID 2>/dev/null
echo FINISHED >> /tmp/rail-theme-$L.result
