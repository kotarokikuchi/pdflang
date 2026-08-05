#!/usr/bin/env bash
# Keeps the version line in docs/*/README.md in step with Cargo.toml.
#
#   ./scripts/sync-version.sh           rewrites the docs
#   ./scripts/sync-version.sh --check   fails if they disagree (used by CI)
#
# Each translated README states the version once, in its own sentence, so the
# first semver in the file is the one to replace — no per-language pattern.
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
[ -n "$VERSION" ] || { echo "could not read the version from Cargo.toml" >&2; exit 1; }

check=0
[ "${1:-}" = "--check" ] && check=1

status=0
for f in docs/*/README.md; do
  current="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' "$f" | head -1 || true)"
  if [ -z "$current" ]; then
    echo "$f: no version found" >&2
    status=1
    continue
  fi
  [ "$current" = "$VERSION" ] && continue
  if [ "$check" -eq 1 ]; then
    echo "$f: says $current, Cargo.toml says $VERSION" >&2
    status=1
  else
    # -i.bak then remove: the bare -i differs between GNU and BSD sed.
    sed -i.bak "0,/[0-9]\{1,\}\.[0-9]\{1,\}\.[0-9]\{1,\}/s//$VERSION/" "$f"
    rm -f "$f.bak"
    echo "$f: $current -> $VERSION"
  fi
done

if [ "$check" -eq 1 ] && [ "$status" -ne 0 ]; then
  echo >&2
  echo "Run ./scripts/sync-version.sh and commit the result." >&2
fi
exit "$status"
