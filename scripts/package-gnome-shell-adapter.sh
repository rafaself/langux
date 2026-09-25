#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UUID="langux-shell@rafaself.github.io"
OUTPUT="$ROOT/dist/langux-gnome-shell-adapter.zip"
STAGING="$(mktemp -d)"
trap 'rm -r -- "$STAGING"' EXIT

mkdir -p "$STAGING/icons" "$ROOT/dist"
cp "$ROOT/gnome-shell-adapter/extension.js" \
    "$ROOT/gnome-shell-adapter/metadata.json" \
    "$ROOT/gnome-shell-adapter/stylesheet.css" \
    "$STAGING/"
cp "$ROOT/gnome-shell-adapter/icons/langux.svg" "$STAGING/icons/"

if command -v gnome-extensions >/dev/null 2>&1; then
    gnome-extensions pack \
        --extra-source="$STAGING/icons" \
        --out-dir "$ROOT/dist" \
        --force \
        "$STAGING"
    mv "$ROOT/dist/$UUID.shell-extension.zip" "$OUTPUT"
else
    command -v zip >/dev/null 2>&1 || {
        echo "error: neither gnome-extensions nor zip is available" >&2
        exit 1
    }
    (cd "$STAGING" && zip -qr "$OUTPUT" .)
fi

if command -v sha256sum >/dev/null 2>&1; then
    (cd "$ROOT/dist" && sha256sum "$(basename "$OUTPUT")" > "$(basename "$OUTPUT").sha256")
elif command -v shasum >/dev/null 2>&1; then
    (cd "$ROOT/dist" && shasum -a 256 "$(basename "$OUTPUT")" > "$(basename "$OUTPUT").sha256")
else
    echo "warning: no checksum tool found; skipping adapter checksum" >&2
fi

echo "Packaged $OUTPUT"
