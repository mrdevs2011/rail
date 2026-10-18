# Rail benchmarks

Small shell harnesses used to put numbers on performance changes. Every script starts its own headless
compositor and a throw-away `RAIL_DATA_DIR` under `/tmp`, so it never touches your real profile or session.

| Script | Measures | Needs |
|---|---|---|
| `perf.sh BIN LABEL` | time to first window, idle CPU and context switches over 10 s, RSS, process count | weston, xdotool |
| `blank-tabs.sh BIN LABEL` | web-process count right after opening 12 blank tabs and again 45 s later (blank-tab suspend) | weston, xdotool |
| `css-cost.sh BIN with\|without RUN` | median time to rebuild a 1500-block DOM with and without the default `html.css` | labwc, python3 |
| `theme-warnings.sh BIN LABEL` | GTK "Theme parser" warning count + a screenshot (`/tmp/rail-theme-LABEL.png`) | labwc, grim |

Results land in `/tmp/` (each script prints where). Compare two binaries by running the same script with
two labels and diffing the output.

Caveat: everything runs with `RAIL_SOFTWARE_RENDER=1` (llvmpipe). CPU and RAM numbers are useful for
before/after comparisons on one machine; they say nothing reliable about GPU frame times. Screenshot
cadence under software rendering is too coarse to judge animation smoothness. For that, use
`jank.log` (written once a minute by a normal session) on real hardware.
