#!/bin/bash
# usage: bench/blank-tabs.sh BIN LABEL   (weston headless + Xwayland + xdotool)
BIN=$1; LABEL=$2
export XDG_RUNTIME_DIR=/run/user/$(id -u)
SOCK=wl-railb-$LABEL
LOG=/tmp/rail-blank-$LABEL.log; : > $LOG
unset WAYLAND_DISPLAY DISPLAY
weston --backend=headless --xwayland --socket=$SOCK --width=1000 --height=700 >/tmp/weston2-$LABEL.log 2>&1 &
WPID=$!
sleep 4
XD=$(grep -o 'display :[0-9]*' /tmp/weston2-$LABEL.log | head -1 | sed 's/display //')
echo "xdisplay=$XD" >> $LOG
[ -z "$XD" ] && { echo "NO XWAYLAND" >> $LOG; kill $WPID; echo FINISHED >> $LOG; exit 1; }
export DISPLAY=$XD GDK_BACKEND=x11 RAIL_SOFTWARE_RENDER=1
export RAIL_DATA_DIR=/tmp/rail-blank-data-$LABEL
rm -rf $RAIL_DATA_DIR; mkdir -p $RAIL_DATA_DIR
$BIN > /tmp/rail-run2-$LABEL.log 2>&1 &
RPID=$!
sleep 8
allcnt(){ ps -eo pid,ppid,args | awk -v p=$RPID '$2==p && /WebKitWebProcess/' | wc -l; }
echo "rail_alive=$(kill -0 $RPID 2>/dev/null && echo yes || echo no) base_web=$(allcnt)" >> $LOG
echo "windows: $(xdotool search --onlyvisible --name . 2>&1 | head -3 | tr '\n' ' ')" >> $LOG
xdotool mousemove 500 350 click 1; sleep 1
for i in $(seq 1 12); do xdotool key ctrl+t; sleep 1.5; done
echo "after_open_12 web=$(allcnt)" >> $LOG
sleep 45; echo "after_open_12_plus45s web=$(allcnt)" >> $LOG
sleep 6
ps -o rss= -p $RPID | awk '{printf "rail_rss_MB=%.0f\n",$1/1024}' >> $LOG
echo "rail_alive_end=$(kill -0 $RPID 2>/dev/null && echo yes || echo no)" >> $LOG
kill $RPID 2>/dev/null; sleep 1; kill $WPID 2>/dev/null
echo FINISHED >> $LOG
