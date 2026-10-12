#!/usr/bin/env bash
# Rail browser installer
set -euo pipefail

APP_NAME="Rail"
REPO="${RAIL_REPO:-mrdevs2011/rail}"
INSTALL_DIR="${RAIL_DIR:-$HOME/MR/Applictions}"
BIN_DIR="$HOME/.local/bin"
BIN_LINK="$BIN_DIR/rail"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_CACHE="$HOME/.cache/rail-icons"

echo "==> $APP_NAME o‘rnatilmoqda..."
mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$DESKTOP_DIR" "$ICON_CACHE"

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
  download "https://github.com/${REPO}/releases/download/v0.1.0/rail-x86_64" "$TARGET"
  chmod +x "$TARGET"
else
  # WebKit bo'lmasa, to'liq AppImage yuklanadi
  TARGET="$INSTALL_DIR/rail-x86_64.AppImage"
  echo "==> Rail Standalone AppImage yuklab olinmoqda..."
  download "https://github.com/${REPO}/releases/download/v0.1.0/rail-x86_64.AppImage" "$TARGET"
  chmod +x "$TARGET"
  ln -sfn "$TARGET" "$BIN_LINK"
fi

# Icon
ICON="$ICON_CACHE/rail.svg"
cat > "$ICON" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
  <rect width="256" height="256" rx="56" fill="#16161e"/>
  <rect x="34" y="70" width="86" height="116" rx="14" fill="#7aa2f7"/>
  <rect x="136" y="70" width="86" height="116" rx="14" fill="#2a2b3d"/>
  <circle cx="198" cy="206" r="9" fill="#7aa2f7"/>
</svg>
EOF

cat > "$DESKTOP_DIR/dev.rail.browser.desktop" <<EOF
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

chmod +x "$DESKTOP_DIR/dev.rail.browser.desktop" 2>/dev/null || true
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo "==> Rail muvaffaqiyatli o‘rnatildi!"
echo "Joylashuv: $TARGET"
echo "Ishga tushirish uchun: rail"
