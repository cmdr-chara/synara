#!/usr/bin/env bash
set -euo pipefail

# Build a native Rust/GPUI Linux AppDir/AppImage and updater metadata.
# linuxdeploy is required because the GPUI/WebKitGTK stack is dynamically
# linked; copying only the two executables would create a misleading artifact
# that depends on the build host's exact system libraries.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${SYNARA_RUST_DESKTOP_OUT:-$ROOT/dist/rust-desktop-linux}"
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
    PRODUCT_STEM="Synara"
    EXECUTABLE="synara"
    ICON_SOURCE="$ROOT/apps/desktop/resources/app-icon-linux.png"
    ;;
  beta)
    APP_NAME="Synara Beta"
    PRODUCT_STEM="Synara-Beta"
    EXECUTABLE="synara-beta"
    CHANNEL="beta"
    ICON_SOURCE="$ROOT/assets/beta/beta-universal-1024.png"
    ;;
  remote)
    APP_NAME="Synara Remote"
    PRODUCT_STEM="Synara-Remote"
    EXECUTABLE="synara-remote"
    CHANNEL="remote"
    ICON_SOURCE="$ROOT/apps/desktop/resources/app-icon-linux.png"
    ;;
  *)
    echo "unsupported flavor for the Rust Linux artifact: $FLAVOR" >&2
    exit 2
    ;;
esac

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "the Rust Linux packager must run on Linux" >&2
  exit 2
fi
case "$(uname -m)" in
  x86_64|amd64)
    ARCH="x64"
    PAYLOAD_ARCH="x86_64"
    APPIMAGE_ARCH="x86_64"
    MANIFEST_ARCH=""
    ;;
  arm64|aarch64)
    ARCH="arm64"
    PAYLOAD_ARCH="arm64"
    APPIMAGE_ARCH="aarch64"
    MANIFEST_ARCH="-arm64"
    ;;
  *)
    echo "unsupported Linux architecture: $(uname -m)" >&2
    exit 2
    ;;
esac

LINUXDEPLOY="${LINUXDEPLOY:-$(command -v linuxdeploy || true)}"
APPIMAGETOOL="${APPIMAGETOOL:-$(command -v appimagetool || true)}"
[[ -x "$LINUXDEPLOY" ]] || { echo "linuxdeploy is required; install it or set LINUXDEPLOY to its executable" >&2; exit 2; }
[[ -x "$APPIMAGETOOL" ]] || { echo "appimagetool is required; install it or set APPIMAGETOOL to its executable" >&2; exit 2; }
[[ -f "$ICON_SOURCE" ]] || { echo "missing icon source: $ICON_SOURCE" >&2; exit 1; }

if [[ "$SKIP_BUILD" != 1 ]]; then
  cargo build --release --manifest-path "$ROOT/rust/shell/Cargo.toml"
  cargo build --release --manifest-path "$ROOT/rust/app/Cargo.toml"
fi

SHELL_BIN="$ROOT/rust/shell/target/release/synara-shell"
BACKEND_BIN="$ROOT/rust/app/target/release/index"
[[ -x "$SHELL_BIN" ]] || { echo "missing shell release binary: $SHELL_BIN" >&2; exit 1; }
[[ -x "$BACKEND_BIN" ]] || { echo "missing backend release binary: $BACKEND_BIN" >&2; exit 1; }

APPDIR="$OUT_DIR/$PRODUCT_STEM.AppDir"
rm -rf "$APPDIR"
mkdir -p \
  "$APPDIR/usr/bin" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/1024x1024/apps" \
  "$APPDIR/usr/share/synara"

cp "$SHELL_BIN" "$APPDIR/usr/bin/$EXECUTABLE"
cp "$BACKEND_BIN" "$APPDIR/usr/bin/synara-backend"
cp "$BACKEND_BIN" "$APPDIR/usr/bin/synara-restore-migration-backup"
chmod 755 "$APPDIR/usr/bin/$EXECUTABLE" "$APPDIR/usr/bin/synara-backend" "$APPDIR/usr/bin/synara-restore-migration-backup"
cp "$ICON_SOURCE" "$APPDIR/usr/share/icons/hicolor/1024x1024/apps/$EXECUTABLE.png"

cat > "$APPDIR/usr/share/synara/app-update.yml" <<YML
provider: github
owner: $OWNER
repo: $REPO
host: $HOST
YML

cat > "$APPDIR/usr/share/applications/$EXECUTABLE.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=$APP_NAME
Comment=AI coding workspace
Exec=$EXECUTABLE
Icon=$EXECUTABLE
Categories=Development;IDE;
Terminal=false
StartupWMClass=$EXECUTABLE
X-AppImage-Version=$VERSION
DESKTOP

cat > "$APPDIR/AppRun" <<APPRUN
#!/bin/sh
HERE="\$(CDPATH= cd -- "\$(dirname -- "\$0")" && pwd)"
LD_LIBRARY_PATH="\$HERE/usr/lib\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
for libdir in "\$HERE"/usr/lib/*-linux-gnu; do
  [ -d "\$libdir" ] && LD_LIBRARY_PATH="\$libdir:\$LD_LIBRARY_PATH"
done
export LD_LIBRARY_PATH
PATH="\$HERE/usr/bin:\$PATH"
XDG_DATA_DIRS="\$HERE/usr/share\${XDG_DATA_DIRS:+:\$XDG_DATA_DIRS}"
export PATH XDG_DATA_DIRS
for webkit_exec in "\$HERE"/usr/libexec/webkit2gtk-* "\$HERE"/usr/lib/webkit2gtk-*; do
  [ -d "\$webkit_exec" ] && export WEBKIT_EXEC_PATH="\$webkit_exec"
done
[ -d "\$HERE/usr/lib/gio/modules" ] && export GIO_MODULE_DIR="\$HERE/usr/lib/gio/modules"
exec "\$HERE/usr/bin/$EXECUTABLE" "\$@"
APPRUN
chmod 755 "$APPDIR/AppRun"

# Resolve the dynamically linked runtime into the AppDir before appimagetool
# compresses it. The executables are passed explicitly so backend-only shared
# dependencies are included as well as the shell's WebKit/GTK chain.
"$LINUXDEPLOY" \
  --appdir "$APPDIR" \
  --executable "$APPDIR/usr/bin/$EXECUTABLE" \
  --executable "$APPDIR/usr/bin/synara-backend" \
  --desktop-file "$APPDIR/usr/share/applications/$EXECUTABLE.desktop" \
  --icon-file "$APPDIR/usr/share/icons/hicolor/1024x1024/apps/$EXECUTABLE.png"

APPIMAGE_NAME="$PRODUCT_STEM-$VERSION-$PAYLOAD_ARCH.AppImage"
rm -f "$OUT_DIR/$APPIMAGE_NAME"
mkdir -p "$OUT_DIR"
ARCH="$APPIMAGE_ARCH" "$APPIMAGETOOL" "$APPDIR" "$OUT_DIR/$APPIMAGE_NAME"
chmod 755 "$OUT_DIR/$APPIMAGE_NAME"

SHA512="$(python3 - "$OUT_DIR/$APPIMAGE_NAME" <<'PY'
import base64,hashlib,sys
print(base64.b64encode(hashlib.sha512(open(sys.argv[1],'rb').read()).digest()).decode())
PY
)"
SIZE="$(stat -c %s "$OUT_DIR/$APPIMAGE_NAME")"
MANIFEST="$OUT_DIR/$CHANNEL-linux$MANIFEST_ARCH.yml"
cat > "$MANIFEST" <<YML
version: $VERSION
files:
  - url: $APPIMAGE_NAME
    sha512: $SHA512
    size: $SIZE
path: $APPIMAGE_NAME
sha512: $SHA512
YML

echo "appdir: $APPDIR"
echo "appimage: $OUT_DIR/$APPIMAGE_NAME"
echo "manifest: $MANIFEST"
