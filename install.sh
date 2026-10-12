#!/usr/bin/env bash
# Rail browser installer (Standalone AppImage for Linux)
set -euo pipefail

APP_NAME="Rail"
REPO="${RAIL_REPO:-mrdevs2011/rail}"
INSTALL_DIR="${RAIL_DIR:-$HOME/MR/Applictions}"
BIN_LINK="$HOME/.local/bin/rail"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_CACHE="$HOME/.cache/rail-icons"

echo "==> $APP_NAME o‘rnatilmoqda..."
mkdir -p "$INSTALL_DIR" "$HOME/.local/bin" "$DESKTOP_DIR" "$ICON_CACHE"

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

TARGET="$INSTALL_DIR/rail-x86_64.AppImage"
API="https://api.github.com/repos/${REPO}/releases/latest"
TMP="$(mktemp)"

echo "==> Eng so‘nggi $APP_NAME AppImage yuklab olinmoqda..."
if curl -fsSL -o "$TMP" "$API" 2>/dev/null; then
  ASSET_URL="$(python3 -c "import json,sys; j=json.load(open(sys.argv[1])); print(next((a['browser_download_url'] for a in j.get('assets',[]) if 'rail' in a.get('name','').lower() and a.get('name','').endswith('.AppImage')), ''))" "$TMP" 2>/dev/null || true)"
  rm -f "$TMP"
  if [ -n "${ASSET_URL:-}" ]; then
    download "$ASSET_URL" "$TARGET"
  else
    download "https://github.com/${REPO}/releases/download/v0.1.0/rail-x86_64.AppImage" "$TARGET"
  fi
else
  rm -f "$TMP" 2>/dev/null || true
  download "https://github.com/${REPO}/releases/download/v0.1.0/rail-x86_64.AppImage" "$TARGET"
fi

chmod +x "$TARGET"
ln -sfn "$TARGET" "$BIN_LINK"

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
Exec="$TARGET" %u
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
