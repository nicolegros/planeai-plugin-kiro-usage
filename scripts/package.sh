#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:-$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)}"
PACKAGE_NAME="planeai-plugin-kiro-usage"
DIST="$ROOT/dist"
STAGING="$DIST/$PACKAGE_NAME"
BINARY="$ROOT/target/release/planeai-plugin-kiro-usage"

if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  echo "This v1 packaging script must run on macOS arm64." >&2
  exit 1
fi

cargo build --manifest-path "$ROOT/Cargo.toml" --release
rm -rf "$STAGING"
mkdir -p "$STAGING/bin/macos-arm64" "$STAGING/ui"
cp "$ROOT/package/planeai-plugin.json" "$STAGING/planeai-plugin.json"
cp "$ROOT/package/ui/entry.js" "$STAGING/ui/entry.js"
cp "$BINARY" "$STAGING/bin/macos-arm64/planeai-plugin-kiro-usage"
chmod 755 "$STAGING/bin/macos-arm64/planeai-plugin-kiro-usage"
(
  cd "$DIST"
  tar -czf "$PACKAGE_NAME-$VERSION-macos-arm64.tar.gz" "$PACKAGE_NAME"
)
printf 'Created %s\n' "$DIST/$PACKAGE_NAME-$VERSION-macos-arm64.tar.gz"
