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
# WebKit is commonly nested below one or more bwrap processes; count the full
# descendant tree instead of only direct children of Rail.
web_pids(){
  ps -eo pid=,ppid=,rss=,args= | awk -v root="$RPID" '
    { pid[NR]=$1; parent[$1]=$2; command[$1]=$4 }
    END {
      for (i=1; i<=NR; i++) {
        x=pid[i]
        while (x>1 && (x in parent)) {
          if (parent[x]==root) {
            if (command[pid[i]] ~ /\/WebKitWebProcess$/) print pid[i]
            break
          }
          x=parent[x]
        }
      }
    }'
}
allcnt(){ web_pids | wc -l; }
echo "rail_alive=$(kill -0 $RPID 2>/dev/null && echo yes || echo no) base_web=$(allcnt)" >> $LOG
echo "windows: $(xdotool search --onlyvisible --name . 2>&1 | head -3 | tr '\n' ' ')" >> $LOG
xdotool mousemove 500 350 click 1; sleep 1
for i in $(seq 1 12); do xdotool key ctrl+t; sleep 1.5; done
echo "after_open_12 web=$(allcnt)" >> $LOG
sleep 45; echo "after_open_12_plus45s web=$(allcnt)" >> $LOG
# Walk back to the first (sleeping) tab and verify that it can wake again.
for i in $(seq 1 12); do xdotool key alt+h; sleep 0.2; done
sleep 6
echo "after_wake web=$(allcnt)" >> $LOG
ps -o rss= -p $RPID | awk '{printf "rail_rss_MB=%.0f\n",$1/1024}' >> $LOG
web_pids | xargs -r ps -o rss= -p | awk '{s+=$1} END {printf "web_tree_rss_MB=%.0f\n",s/1024}' >> $LOG
echo "rail_alive_end=$(kill -0 $RPID 2>/dev/null && echo yes || echo no)" >> $LOG
kill $RPID 2>/dev/null; sleep 1; kill $WPID 2>/dev/null
echo FINISHED >> $LOG
