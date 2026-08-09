#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UUID="langux@rafaself.github.io"
cd "$ROOT"

STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

mkdir -p "$STAGING"/ui "$STAGING"/services "$STAGING"/schemas "$STAGING"/data
cp extension.js prefs.js metadata.json stylesheet.css stylesheet-dark.css "$STAGING"/
cp ui/*.js "$STAGING"/ui/
cp services/*.js "$STAGING"/services/
cp schemas/*.xml "$STAGING"/schemas/
cp data/icon.svg data/icon-light.svg "$STAGING"/data/
glib-compile-schemas "$STAGING"/schemas

mkdir -p "$ROOT"/dist

if command -v gnome-extensions >/dev/null 2>&1; then
    # Preferred path: GNOME's own packer. --extra-source includes nested runtime
    # directories (ui/ and services/), which `pack` skips by default.
    gnome-extensions pack \
        --extra-source="$STAGING/ui" \
        --extra-source="$STAGING/services" \
        --extra-source="$STAGING/data" \
        --out-dir "$ROOT"/dist \
        --force \
        "$STAGING"
    mv "$ROOT"/dist/"$UUID".shell-extension.zip "$ROOT"/dist/langux.zip
else
    # Fallback for environments without GNOME Shell (e.g. headless CI). Produces
    # the same whitelisted runtime content.
    command -v zip >/dev/null 2>&1 || {
        echo "error: neither gnome-extensions nor zip is available" >&2
        exit 1
    }
    (cd "$STAGING" && zip -qr "$ROOT"/dist/langux.zip .)
fi

if command -v sha256sum >/dev/null 2>&1; then
    (cd "$ROOT"/dist && sha256sum langux.zip > langux.zip.sha256)
elif command -v shasum >/dev/null 2>&1; then
    (cd "$ROOT"/dist && shasum -a 256 langux.zip > langux.zip.sha256)
else
    echo "warning: no checksum tool found; skipping dist/langux.zip.sha256" >&2
fi

echo "Packaged dist/langux.zip"
