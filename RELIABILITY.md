# Rail Reliability & Fault-Tolerance Architecture

This document records Rail reliability mechanisms and the checks currently used to validate them. It is not a claim that regressions, data loss, or platform-specific failures are impossible.

---

## 1. Regression Prevention

Core workflows are protected by scoped changes, automated tests, and manual verification. A passing test suite reduces regression risk but does not prove that every site or desktop environment behaves identically:
- **No paid runtime dependency**: Core browsing does not require a paid service or proprietary telemetry.
- **Strict Profile Isolation**: Cookies, caches, IndexedDB, and local storage are partitioned by dedicated `WebKitNetworkSession` instances.
- **Keyboard-First Paradigm**: All user interactions have deterministically assigned actions and hotkeys.
- **Monochrome Interface**: Black-and-white visual language with restrained accent use.

---

## 2. Transactional Persistence & Self-Healing Engine

File persistence in Rail operates through a transactional storage engine (`src/storage.rs`) designed to withstand sudden OS crashes, power interruptions, and filesystem corruptions:

```mermaid
flowchart TD
    A[Data Change in UI/App] --> B[Serialize to Memory Buffer]
    B --> C[Write to Same-Directory .tmp File]
    C --> D[Physical Hardware Flush: fsync / sync_all]
    D --> E{Original File Exists & Non-Empty?}
    E -- Yes --> F[Update Redundant .bak Snapshot]
    E -- No --> G[Proceed]
    F --> H[Atomic Rename: .tmp to Target Path]
    G --> H
    H --> I[Atomic replacement complete; directory sync attempted]
```

### Self-Healing on Load
Recovery is best-effort and depends on the backup being present, readable, and valid. It cannot protect against disk failure, filesystem bugs, or simultaneous external modification.

When loading persisted states (`settings.json`, `session.txt`, `shortcuts-customize.json`, `profiles.json`, `history.txt`, `downloads.json`):
1. **Primary Read & Validate**: The primary file is inspected and validated (e.g. valid JSON parsing, non-empty session markers).
2. **Defensive Auto-Recovery**: If the primary file is missing, zero-length, or corrupted, the storage engine inspects the `.bak` replica.
3. **Automated Restoration**: If the `.bak` file satisfies the validation contract, it is atomically restored to the primary file path and loaded seamlessly into memory.
4. **Log Diagnostics**: Diagnostic notices are logged via standard `[rail]` error channels.

---

## 3. Concurrency & Event Loop Confinement

- **Main Thread Purity**: All GTK UI construction, widget manipulation, and WebKit callbacks execute strictly on the GTK main loop.
- **Non-Blocking Background I/O**:
  - History snapshots are sorted and persisted on a dedicated background worker thread (`rail-history`) through epoch-stamped message queues.
  - Cloud backup encryption (AES-256-GCM) and decompression execute in background worker threads.
- **Event-driven work**: Page and UI work is primarily event-driven. A lightweight main-loop watchdog samples responsiveness; it is not a guarantee of zero idle CPU usage.

---

## 4. Clipboard & File System Bridging

- **Web-to-Desktop Native File Drop/Paste**: Linux WebKitGTK natively ignores file lists placed on the clipboard by external file managers (Nautilus, Dolphin, Thunar). Rail intercepts `Ctrl+V` in the capture phase, inspects `text/uri-list` on the native clipboard, and replays synthetic file drop payloads directly into the focused page, resolving paste barriers on web applications.

---

## 5. Verification Matrix

The automated test suite currently covers:
- **Storage Resilience**: Deliberately injected corruption triggers successful self-healing from backup.
- **Format Integrity**: TSV and JSON parsers roundtrip properly without loss.
- **Keyboard Maps & Actions**: Every shortcut maps to a registered action ID with collision resolution.
- **Compiler & Linter Standards**: Release checks are expected to pass `cargo check --release` and `cargo clippy --release -- -D warnings`.
