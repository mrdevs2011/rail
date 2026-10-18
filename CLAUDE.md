# Rail

Linux-only keyboard-driven spatial browser. Rust + GTK4 + WebKitGTK 6.
Niri-style infinite horizontal strip of columns, vertical stacks inside columns.
Version in Cargo.toml / package.json must stay in sync.

## Commands

```bash
cargo build --release          # produce target/release/rail
cargo run --release            # run from source
./rail-linux-x86_64            # run prebuilt binary in repo root
rail --update                  # self-update from GitHub Releases
```

Bench harnesses (headless / diagnostic):

```bash
bench/perf.sh
bench/blank-tabs.sh
bench/css-cost.sh
bench/theme-warnings.sh
```

CI: push to `main` → Linux build. Tag `v*` → release + SHA256SUMS.
Do not invent other package managers or platforms.

## Architecture (read before changing)

| Path | Role |
|------|------|
| `src/main.rs` | App shell, window, tabs, shortcuts, DevTools, omnibar, settings UI (~13k lines — search, do not rewrite wholesale) |
| `src/strip.rs` | Horizontal strip / column layout, off-screen unmap |
| `src/profile.rs` | Isolated profiles (separate WebKitNetworkSession) |
| `src/history.rs` | History + suggest (background write) |
| `src/sync.rs` | E2E encrypted cloud sync (AES-256, Supabase + Vercel) |
| `src/update.rs` | Auto-update from GitHub Releases |
| `src/jank.rs` | Main-loop jank aggregator → `jank.log` |
| `src/spring.rs` | Niri-style spring + flick for touchpad slide |
| `src/i18n*.rs` | uz (default) / en / ru strings |
| `docs/` | Multi-page site (architecture, shortcuts, profiles, …) + i18n |
| `api/sync.js` | Sync API helper |
| `bench/` | Perf / blank-tab / CSS-cost harnesses |

Invariants from ROADMAP (do not violate):

1. 100% free path — no paid services, no paid domains.
2. Black-and-white UI; only accent is a single blue dot.
3. Keyboard first — every action has a key; avoid site-conflicting shortcuts.
4. Profile = isolated world (cookies, storage, cache never mix).
5. Single Rust core; only window/webview layer may differ by OS (currently Linux only).
6. Smallest working step first, then polish.

## Workflow (mandatory)

1. **Investigate before edit.** Read the relevant module and recent `STATUS.md` / `git log`. For UI jank or freezes, read `jank.log` (written by `src/jank.rs`) and any `[rail]` stderr lines.
2. **Minimal diff.** Change only what the task requires. Do not refactor `main.rs` wholesale.
3. **Verify.** After code changes:
   - `cargo build --release` must succeed.
   - Prefer targeted checks (bench scripts or a short manual path) over "looks fine".
   - If the change touches timing/layout/scroll/slide, note what to watch in `jank.log`.
4. **STATUS.md.** After meaningful work, append a short dated block: what was done, blocker (or none), next step, relevant commit/hash if any. Do not rewrite old history.
5. **Version.** When releasing: bump `Cargo.toml` + `package.json` + site version together. Use existing release workflow; do not invent a new one.

## Code rules

- Prefer early returns and small helpers over deep nesting.
- No new dependencies without a clear need and user OK.
- No UA/fingerprint spoofing or false privacy claims (already removed once).
- WebKitGTK quirks (e.g. image paste bug 218519, DevTools attach) already have workarounds — extend them carefully; do not remove without a proven replacement.
- Logs: use existing `[rail]` / jank paths; do not spam stdout in hot paths.
- i18n: new user-visible strings go through `i18n` / `i18n_tables` for uz, en, ru.

## Forbidden

- `rm -rf`, force-push, editing generated release assets, or changing install paths without explicit request.
- Committing secrets, `.env`, or tokens.
- Claiming a task is done without a successful build (or an explicit reason it cannot be built in this environment).
- Expanding scope beyond the requested change.

## When stuck

- Search `src/` for the existing pattern (shortcuts, profile switch, update card, suspend policy).
- Check `STATUS.md` and recent commits for prior decisions.
- Prefer one focused fix + note in STATUS over a large speculative rewrite.
