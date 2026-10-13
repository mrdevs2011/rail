# Rail

**Rail** — a niri-inspired keyboard-driven web browser built with GTK4 and WebKitGTK. It features an infinite horizontal strip of columns, vertical window stacks, and completely isolated multi-profiles.

![Rail Screenshot](https://raw.githubusercontent.com/mrdevs2011/rail/main/screenshot.png) <!-- if available or omit -->

## Features

- **Niri-Style 2D Tiling**:
  - Horizontal columns that pan smoothly across an infinite ribbon.
  - Vertical window stacks within columns (`Ctrl+V` / `Ctrl+Shift+T`).
  - Seamless 2D keyboard navigation (`Ctrl+H/J/K/L`) and window movement (`Ctrl+Shift+H/J/K/L`).
- **Completely Isolated Multi-Profiles**:
  - Independent cookies, localStorage, indexedDB, caches, and sessions per profile via isolated `WebKitNetworkSession` directories (`~/.local/share/rail/profiles/<id>`).
  - Profile manager modal (`Ctrl+P` or `Ctrl+Shift+P`).
  - Quick profile selector directly within the URL bar dropdown.
- **Responsive Layout**:
  - Adapts smoothly to half-screen window snapping and narrow viewports without collapsing layout elements.
- **Lightweight & Fast**:
  - Written in Rust using GTK4 and WebKitGTK 6.0.
  - Native binary is only ~500 KB.

## Quick Install (Linux)

To install Rail via a single command:

```bash
curl -fsSL https://raw.githubusercontent.com/mrdevs2011/rail/main/install.sh | bash
```

Run with:
```bash
rail
```
or launch it with a URL:
```bash
rail https://github.com
```

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + G` | Open URL address bar |
| `Ctrl + Enter` | Edit URL of currently focused window |
| `Ctrl + T` | New tab (create column to the right) |
| `Ctrl + V` / `Ctrl + Shift + T` | Split vertically (new window down in column) |
| `Ctrl + W` | Close focused window |
| `Ctrl + H` / `Ctrl + L` | Focus column Left / Right |
| `Ctrl + J` / `Ctrl + K` | Focus window Down / Up |
| `Ctrl + Shift + H` / `Ctrl + Shift + L` | Move column Left / Right |
| `Ctrl + Shift + J` / `Ctrl + Shift + K` | Move window Down / Up |
| `Ctrl + P` / `Ctrl + Shift + P` | Open Profiles Manager |
| `Ctrl + F` | Toggle Full / Default column width |
| `Ctrl + O` | Toggle Overview mode |
| `Ctrl + B` | Toggle Sidebar |
| `Alt + Left` / `Alt + Right` | Navigate Back / Forward |
| `Ctrl + R` | Reload page |
| `Ctrl + +` / `Ctrl + -` / `Ctrl + 0` | Zoom in / Zoom out / Reset zoom |

## Building from Source

Prerequisites:
- Rust (Cargo 1.85+)
- GTK4 development headers (`gtk4-devel` / `libgtk-4-dev`)
- WebKitGTK 6.0 development headers (`webkitgtk-6.0-devel` / `libwebkitgtk-6.0-dev`)

```bash
git clone https://github.com/mrdevs2011/rail.git
cd rail
cargo build --release
```

The compiled binary will be located at `target/release/rail`.
