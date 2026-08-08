#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UUID="org.gnome.shell.extensions.langux"

command -v gnome-extensions >/dev/null 2>&1 || {
    echo "error: gnome-extensions is not available (GNOME Shell is not installed?)" >&2
    exit 1
}

STAGING="$(mktemp -d)"
trap 'rm -rf "$STAGING"' EXIT

mkdir -p "$STAGING"/ui "$STAGING"/services "$STAGING"/schemas
cp extension.js prefs.js metadata.json stylesheet.css stylesheet-dark.css "$STAGING"/
cp ui/*.js "$STAGING"/ui/
cp services/*.js "$STAGING"/services/
cp schemas/*.xml "$STAGING"/schemas/
glib-compile-schemas "$STAGING"/schemas

mkdir -p "$ROOT"/dist
# --extra-source includes nested runtime directories, which `pack` skips by
# default (ui/ and services/ hold all Langux UI and service modules).
gnome-extensions pack \
    --extra-source="$STAGING/ui" \
    --extra-source="$STAGING/services" \
    --out-dir "$ROOT"/dist \
    --force \
    "$STAGING"
mv "$ROOT"/dist/"$UUID".shell-extension.zip "$ROOT"/dist/langux.zip
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$ROOT"/dist/langux.zip > "$ROOT"/dist/langux.zip.sha256
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$ROOT"/dist/langux.zip > "$ROOT"/dist/langux.zip.sha256
else
    echo "warning: no checksum tool found; skipping dist/langux.zip.sha256" >&2
fi

echo "Packaged dist/langux.zip"
