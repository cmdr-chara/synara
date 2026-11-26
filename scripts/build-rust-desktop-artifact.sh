#!/usr/bin/env bash
set -euo pipefail

# Build a minimal native Rust/GPUI macOS app bundle and updater zip.
# This is intentionally separate from the Electron artifact pipeline while the
# Rust shell reaches parity; artifact identity follows packages/shared desktopIdentity.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${SYNARA_RUST_DESKTOP_OUT:-$ROOT/dist/rust-desktop}"
VERSION="${SYNARA_DESKTOP_VERSION:-$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1] + "/apps/desktop/package.json"))["version"])' "$ROOT")}"
FLAVOR="${SYNARA_DESKTOP_FLAVOR:-production}"
CHANNEL="${SYNARA_UPDATE_CHANNEL:-synara}"
OWNER="${SYNARA_UPDATE_OWNER:-cmdr-chara}"
REPO="${SYNARA_UPDATE_REPO:-synara}"
HOST="${SYNARA_UPDATE_HOST:-github.com}"
SKIP_BUILD=0
if [[ "${1:-}" == "--no-build" ]]; then
  SKIP_BUILD=1
fi

case "$FLAVOR" in
  production)
    APP_NAME="Synara"
    BUNDLE_ID="com.emanueledipietro.synara"
    ;;
  beta)
    APP_NAME="Synara Beta"
    BUNDLE_ID="com.emanueledipietro.synara.beta"
    CHANNEL="beta"
    ;;
  remote)
    APP_NAME="Synara Remote"
    BUNDLE_ID="com.emanueledipietro.synara.remote"
    CHANNEL="remote"
    ;;
  *)
    echo "unsupported flavor for the Rust desktop artifact: $FLAVOR" >&2
    exit 2
    ;;
esac

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "the initial Rust desktop packager currently supports macOS only" >&2
  exit 2
fi
case "$(uname -m)" in
  arm64|aarch64) ARCH="arm64" ;;
  x86_64|amd64) ARCH="x64" ;;
  *) ARCH="$(uname -m)" ;;
esac

if [[ "$SKIP_BUILD" != 1 ]]; then
  cargo build --release --manifest-path "$ROOT/rust/shell/Cargo.toml"
  cargo build --release --manifest-path "$ROOT/rust/app/Cargo.toml"
fi

SHELL_BIN="$ROOT/rust/shell/target/release/synara-shell"
BACKEND_BIN="$ROOT/rust/app/target/release/index"
[[ -x "$SHELL_BIN" ]] || { echo "missing shell release binary: $SHELL_BIN" >&2; exit 1; }
[[ -x "$BACKEND_BIN" ]] || { echo "missing backend release binary: $BACKEND_BIN" >&2; exit 1; }

APP_DIR="$OUT_DIR/$APP_NAME.app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
cp "$SHELL_BIN" "$APP_DIR/Contents/MacOS/synara-shell"
cp "$BACKEND_BIN" "$APP_DIR/Contents/MacOS/synara-backend"
chmod 755 "$APP_DIR/Contents/MacOS/synara-shell" "$APP_DIR/Contents/MacOS/synara-backend"
cp "$ROOT/apps/desktop/resources/icon.icns" "$APP_DIR/Contents/Resources/icon.icns"

cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$APP_NAME</string>
  <key>CFBundleDisplayName</key><string>$APP_NAME</string>
  <key>CFBundleExecutable</key><string>synara-shell</string>
  <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleIconFile</key><string>icon.icns</string>
  <key>SynaraDesktopFlavor</key><string>$FLAVOR</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSMicrophoneUsageDescription</key><string>Synara uses the microphone for voice input when you start dictation.</string>
  <key>NSCameraUsageDescription</key><string>Synara uses the camera only when you enable video input.</string>
</dict>
</plist>
PLIST

cat > "$APP_DIR/Contents/Resources/app-update.yml" <<YML
provider: github
owner: $OWNER
repo: $REPO
host: $HOST
YML

# Ad-hoc signing keeps the bundle launchable locally; release signing/notarization
# remains a separate release-pipeline concern.
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1 || true
fi

ZIP_NAME="${APP_NAME// /-}-$VERSION-$ARCH.zip"
ZIP_PATH="$OUT_DIR/$ZIP_NAME"
rm -f "$ZIP_PATH"
ditto -c -k --sequesterRsrc --keepParent "$APP_DIR" "$ZIP_PATH"

SHA512="$(python3 - "$ZIP_PATH" <<'PY'
import base64,hashlib,sys
print(base64.b64encode(hashlib.sha512(open(sys.argv[1],'rb').read()).digest()).decode())
PY
)"
SIZE="$(stat -f %z "$ZIP_PATH")"
MANIFEST="$OUT_DIR/$CHANNEL-mac.yml"
cat > "$MANIFEST" <<YML
version: $VERSION
files:
  - url: $ZIP_NAME
    sha512: $SHA512
    size: $SIZE
path: $ZIP_NAME
sha512: $SHA512
minimumSystemVersion: '13.0'
YML

echo "app: $APP_DIR"
echo "zip: $ZIP_PATH"
echo "manifest: $MANIFEST"
