#!/usr/bin/env bash
set -euo pipefail

# Build a native Rust/GPUI Windows NSIS installer and updater metadata.
# The installer is per-user, uses the flavor-stable uninstall GUID, and keeps
# updater configuration beside the executables where the Rust shell resolves it.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${SYNARA_RUST_DESKTOP_OUT:-$ROOT/dist/rust-desktop-windows}"
VERSION="${SYNARA_DESKTOP_VERSION:-$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1] + "/apps/desktop/package.json"))["version"])' "$ROOT")}"
FLAVOR="${SYNARA_DESKTOP_FLAVOR:-production}"
CHANNEL="${SYNARA_UPDATE_CHANNEL:-synara}"
OWNER="${SYNARA_UPDATE_OWNER:-cmdr-chara}"
REPO="${SYNARA_UPDATE_REPO:-synara}"
HOST="${SYNARA_UPDATE_HOST:-github.com}"
PUBLISHER="${SYNARA_WINDOWS_PUBLISHER:-Synara}"
SKIP_BUILD=0
if [[ "${1:-}" == "--no-build" ]]; then
  SKIP_BUILD=1
fi

case "$FLAVOR" in
  production)
    APP_NAME="Synara"
    PRODUCT_STEM="Synara"
    EXECUTABLE="Synara.exe"
    INSTALL_GUID="368107a8-afe6-5db5-ab3b-d4f331684868"
    ICON_SOURCE="$ROOT/apps/desktop/resources/app-icon-windows.ico"
    ;;
  beta)
    APP_NAME="Synara Beta"
    PRODUCT_STEM="Synara-Beta"
    EXECUTABLE="Synara Beta.exe"
    CHANNEL="beta"
    INSTALL_GUID="a8e63b48-d4f3-4db5-9e12-368107afe65d"
    ICON_SOURCE="$ROOT/assets/beta/beta-windows.ico"
    ;;
  remote)
    APP_NAME="Synara Remote"
    PRODUCT_STEM="Synara-Remote"
    EXECUTABLE="Synara Remote.exe"
    CHANNEL="remote"
    INSTALL_GUID="7639a24e-c753-4f4c-b4e3-37b259bdc793"
    ICON_SOURCE="$ROOT/apps/desktop/resources/app-icon-windows.ico"
    ;;
  *)
    echo "unsupported flavor for the Rust Windows artifact: $FLAVOR" >&2
    exit 2
    ;;
esac

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*|Windows_NT) ;;
  *) echo "the Rust Windows packager must run in a Windows build environment" >&2; exit 2 ;;
esac
case "$(uname -m)" in
  x86_64|amd64) ;;
  *) echo "the initial Rust Windows packager supports x64 only: $(uname -m)" >&2; exit 2 ;;
esac

MAKENSIS="${MAKENSIS:-$(command -v makensis || true)}"
[[ -x "$MAKENSIS" ]] || { echo "makensis is required; install NSIS or set MAKENSIS to its executable" >&2; exit 2; }
[[ -f "$ICON_SOURCE" ]] || { echo "missing icon source: $ICON_SOURCE" >&2; exit 1; }

if [[ "$SKIP_BUILD" != 1 ]]; then
  cargo build --release --manifest-path "$ROOT/rust/shell/Cargo.toml"
  cargo build --release --manifest-path "$ROOT/rust/app/Cargo.toml"
fi

SHELL_BIN="$ROOT/rust/shell/target/release/synara-shell.exe"
BACKEND_BIN="$ROOT/rust/app/target/release/index.exe"
[[ -x "$SHELL_BIN" ]] || { echo "missing shell release binary: $SHELL_BIN" >&2; exit 1; }
[[ -x "$BACKEND_BIN" ]] || { echo "missing backend release binary: $BACKEND_BIN" >&2; exit 1; }

mkdir -p "$OUT_DIR"
UPDATE_CONFIG="$OUT_DIR/app-update.yml"
cat > "$UPDATE_CONFIG" <<YML
provider: github
owner: $OWNER
repo: $REPO
host: $HOST
YML
if [[ -n "${SYNARA_UPDATE_PUBLISHER_NAME:-}" ]]; then
  printf 'publisherName: %s\n' "$SYNARA_UPDATE_PUBLISHER_NAME" >> "$UPDATE_CONFIG"
fi

winpath() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
  else
    printf '%s' "$1"
  fi
}

INSTALLER_NAME="$PRODUCT_STEM-$VERSION-x64.exe"
INSTALLER_PATH="$OUT_DIR/$INSTALLER_NAME"
rm -f "$INSTALLER_PATH"

"$MAKENSIS" \
  "-DAPP_NAME=$APP_NAME" \
  "-DVERSION=$VERSION" \
  "-DEXECUTABLE_NAME=$EXECUTABLE" \
  "-DINSTALL_GUID=$INSTALL_GUID" \
  "-DSHELL_BIN=$(winpath "$SHELL_BIN")" \
  "-DBACKEND_BIN=$(winpath "$BACKEND_BIN")" \
  "-DUPDATE_CONFIG=$(winpath "$UPDATE_CONFIG")" \
  "-DICON_FILE=$(winpath "$ICON_SOURCE")" \
  "-DOUT_FILE=$(winpath "$INSTALLER_PATH")" \
  "-DPUBLISHER=$PUBLISHER" \
  "$(winpath "$ROOT/scripts/rust-windows-installer.nsi")"

SHA512="$(python3 - "$INSTALLER_PATH" <<'PY'
import base64,hashlib,sys
print(base64.b64encode(hashlib.sha512(open(sys.argv[1],'rb').read()).digest()).decode())
PY
)"
SIZE="$(python3 - "$INSTALLER_PATH" <<'PY'
import os,sys
print(os.path.getsize(sys.argv[1]))
PY
)"
MANIFEST="$OUT_DIR/$CHANNEL.yml"
cat > "$MANIFEST" <<YML
version: $VERSION
files:
  - url: $INSTALLER_NAME
    sha512: $SHA512
    size: $SIZE
path: $INSTALLER_NAME
sha512: $SHA512
YML

echo "installer: $INSTALLER_PATH"
echo "manifest: $MANIFEST"
