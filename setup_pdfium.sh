#!/usr/bin/env bash
# Downloads the native pdfium library (bblanchon/pdfium-binaries) into ./pdfium/
set -euo pipefail

DEST="$(dirname "$0")/pdfium"
mkdir -p "$DEST"

# On Windows this runs under Git Bash / MSYS, where uname reports MINGW64_NT-*
# or MSYS_NT-*. That build puts pdfium.dll in bin/, not lib/.
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)      ASSET="pdfium-linux-x64.tgz";  LIBDIR="lib" ;;
  Linux-aarch64)     ASSET="pdfium-linux-arm64.tgz"; LIBDIR="lib" ;;
  Darwin-x86_64)     ASSET="pdfium-mac-x64.tgz";    LIBDIR="lib" ;;
  Darwin-arm64)      ASSET="pdfium-mac-arm64.tgz";  LIBDIR="lib" ;;
  MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) ASSET="pdfium-win-x64.tgz";   LIBDIR="bin" ;;
  MINGW*-aarch64|MSYS*-aarch64)              ASSET="pdfium-win-arm64.tgz"; LIBDIR="bin" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)"; exit 1 ;;
esac

# Cross-building needs the library for the target, not for this machine:
#   PDFIUM_ASSET=pdfium-mac-x64.tgz ./setup_pdfium.sh
# Windows assets keep the library in bin/, everything else in lib/.
if [ -n "${PDFIUM_ASSET:-}" ]; then
  ASSET="$PDFIUM_ASSET"
  case "$ASSET" in
    *-win-*) LIBDIR="bin" ;;
    *)       LIBDIR="lib" ;;
  esac
fi

URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$ASSET"
echo "Downloading $URL ..."
curl -L --fail "$URL" | tar -xz -C "$DEST"
echo "OK: library in $DEST/$LIBDIR/"
