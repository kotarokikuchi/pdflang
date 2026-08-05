#!/usr/bin/env bash
# Downloads the native pdfium library (bblanchon/pdfium-binaries) into ./pdfium/
set -euo pipefail

DEST="$(dirname "$0")/pdfium"
mkdir -p "$DEST"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)  ASSET="pdfium-linux-x64.tgz" ;;
  Linux-aarch64) ASSET="pdfium-linux-arm64.tgz" ;;
  Darwin-x86_64) ASSET="pdfium-mac-x64.tgz" ;;
  Darwin-arm64)  ASSET="pdfium-mac-arm64.tgz" ;;
  *) echo "Unsupported platform: $(uname -s)-$(uname -m)"; exit 1 ;;
esac

URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$ASSET"
echo "Downloading $URL ..."
curl -L --fail "$URL" | tar -xz -C "$DEST"
echo "OK: library in $DEST/lib/"
