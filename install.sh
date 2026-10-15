#!/usr/bin/env bash
# Rail browser installer
set -euo pipefail

APP_NAME="Rail"
REPO="${RAIL_REPO:-mrdevs2011/rail}"
VERSION="${RAIL_VERSION:-latest}"
INSTALL_DIR="${RAIL_DIR:-$HOME/MR/Applictions}"
BIN_DIR="$HOME/.local/bin"
BIN_LINK="$BIN_DIR/rail"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_CACHE="$HOME/.cache/rail-icons"

echo "==> $APP_NAME o‘rnatilmoqda..."
mkdir -p "$BIN_DIR" "$DESKTOP_DIR"

download() {
  if command -v curl >/dev/null 2>&1; then
    curl -fL --progress-bar -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --show-progress -O "$2" "$1"
  else
    echo "Xato: curl yoki wget kerak!" >&2
    exit 1
  fi
}

check_webkit() {
  ldconfig -p 2>/dev/null | grep -q "libwebkitgtk-6.0.so" || \
  [ -f /usr/lib/libwebkitgtk-6.0.so.4 ] || \
  [ -f /usr/lib/x86_64-linux-gnu/libwebkitgtk-6.0.so.4 ] || \
  [ -f /lib/libwebkitgtk-6.0.so.4 ]
}

ensure_dependencies() {
  if check_webkit; then
    echo "✓ Tizimda WebKit6 topildi! Ortiqcha paketlar yuklanmaydi."
    return 0
  fi

  echo "==> WebKitGTK 6.0 tizimda topilmadi. O‘rnatishga harakat qilinmoqda..."
  if command -v pacman >/dev/null 2>&1; then
    sudo pacman -S --needed --noconfirm webkitgtk-6.0 gtk4 || true
  elif command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update && sudo apt-get install -y libwebkitgtk-6.0-4 libgtk-4-1 || true
  elif command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y webkitgtk6.0 gtk4 || true
  fi

  if check_webkit; then
    echo "✓ WebKit6 o‘rnatildi!"
    return 0
  else
    echo "Ogohlantirish: WebKit6 tizim paketlaridan o‘rnatilmadi. Standalone AppImage yuklab olinadi..."
    return 1
  fi
}

# 1. Tizimda WebKit6 borligini tekshiramiz
USE_STANDALONE=0
if ! ensure_dependencies; then
  USE_STANDALONE=1
fi

if [ "$USE_STANDALONE" -eq 0 ]; then
  # Faqat ~400 KB binar fayl yuklanadi
  TARGET="$BIN_LINK"
  echo "==> Rail binar fayli yuklab olinmoqda (atigi ~400 KB)..."
  if [ "$VERSION" = "latest" ]; then
    download "https://github.com/${REPO}/releases/latest/download/rail-linux-x86_64" "$TARGET.new"
  else
    download "https://github.com/${REPO}/releases/download/${VERSION}/rail-linux-x86_64" "$TARGET.new"
  fi
  chmod +x "$TARGET.new"
  mv -f "$TARGET.new" "$TARGET"
else
  # WebKit bo'lmasa, to'liq AppImage yuklanadi
  mkdir -p "$INSTALL_DIR"
  TARGET="$INSTALL_DIR/rail-x86_64.AppImage"
  echo "==> Rail Standalone AppImage yuklab olinmoqda..."
  download "https://github.com/${REPO}/releases/download/v0.1.0/rail-x86_64.AppImage" "$TARGET"
  chmod +x "$TARGET"
  ln -sfn "$TARGET" "$BIN_LINK"
fi

# Icon
ICON="$HOME/.local/share/icons/hicolor/scalable/apps/rail.svg"
mkdir -p "$(dirname "$ICON")"
cat > "$ICON" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
  <rect width="256" height="256" rx="40" fill="#000000"/>
  <g transform="translate(-6 -12)">
    <path d="M96 104V176M96 142C96 116 114 106 142 108" fill="none" stroke="#ffffff" stroke-width="20" stroke-linecap="square" stroke-linejoin="round"/>
    <circle cx="170" cy="176" r="11" fill="#38bdf8"/>
  </g>
</svg>
EOF

rm -f "$DESKTOP_DIR/dev.rail.browser.desktop"
cat > "$DESKTOP_DIR/rail.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Rail
GenericName=Web Browser
Comment=A browser that scrolls sideways
Exec="$BIN_LINK" %u
Icon=$ICON
Terminal=false
Categories=Network;WebBrowser;
MimeType=text/html;x-scheme-handler/http;x-scheme-handler/https;
StartupWMClass=dev.rail.browser
EOF

chmod +x "$DESKTOP_DIR/rail.desktop" 2>/dev/null || true
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo "==> Rail muvaffaqiyatli o‘rnatildi!"
echo "Joylashuv: $TARGET"
echo "Ishga tushirish uchun: rail"
