#!/bin/bash
# usage: bench/perf.sh BIN LABEL  -> startup time + idle CPU + idle wakeups (headless weston+Xwayland)
BIN=$1; L=$2; LOG=/tmp/rail-perf-$L.log; : > $LOG
export XDG_RUNTIME_DIR=/run/user/$(id -u); unset WAYLAND_DISPLAY DISPLAY
weston --backend=headless --xwayland --socket=wl-perf-$L --width=1280 --height=800 >/tmp/weston-perf-$L.log 2>&1 &
WPID=$!; sleep 4
XD=$(grep -o 'display :[0-9]*' /tmp/weston-perf-$L.log | head -1 | sed 's/display //')
export DISPLAY=$XD GDK_BACKEND=x11 RAIL_SOFTWARE_RENDER=1 RAIL_DATA_DIR=/tmp/rail-perf-data-$L
rm -rf $RAIL_DATA_DIR; mkdir -p $RAIL_DATA_DIR
T0=$(date +%s.%N)
$BIN >/tmp/rail-perf-run-$L.log 2>&1 &
RPID=$!
for i in $(seq 1 200); do
  if xdotool search --onlyvisible --name . >/dev/null 2>&1; then break; fi; sleep 0.05
done
T1=$(date +%s.%N)
echo "time_to_first_window_s=$(awk -v a=$T1 -v b=$T0 "BEGIN{printf \"%.2f\", a-b}")" >> $LOG
sleep 6   # let start page + web process settle
tree(){ ps -eo pid,ppid | awk -v r=$RPID 'BEGIN{p[r]=1} {pp[$1]=$2} END{for(k in pp){x=k;while(x>1){if(x==r){print k;break};x=pp[x]}}}'; }
cpu(){ for p in $RPID $(tree); do awk '{print $14+$15}' /proc/$p/stat 2>/dev/null; done | awk '{s+=$1} END{print s+0}'; }
ctx(){ for p in $RPID $(tree); do for t in /proc/$p/task/*; do awk '/voluntary_ctxt_switches/{s+=$2}END{print s+0}' $t/status 2>/dev/null; done; done | awk '{s+=$1} END{print s+0}'; }
C0=$(cpu); X0=$(ctx); sleep 10; C1=$(cpu); X1=$(ctx)
HZ=$(getconf CLK_TCK)
echo "idle_cpu_pct_10s=$(awk -v a=$C1 -v b=$C0 -v h=$HZ "BEGIN{printf \"%.2f\", (a-b)/h/10*100}") ctxswitches_per_s=$(awk -v a=$X1 -v b=$X0 "BEGIN{printf \"%d\", (a-b)/10}")" >> $LOG
echo "procs_in_tree=$(tree | wc -l) rail_rss_MB=$(ps -o rss= -p $RPID | awk '{printf "%.0f",$1/1024}') tree_rss_MB=$(for p in $RPID $(tree); do ps -o rss= -p $p; done | awk '{s+=$1} END{printf "%.0f",s/1024}')" >> $LOG
echo "threads_rail=$(ls /proc/$RPID/task | wc -l)" >> $LOG
kill $RPID 2>/dev/null; sleep 1; kill $WPID 2>/dev/null; echo FINISHED >> $LOG
