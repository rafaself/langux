#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UUID="langux-shell@rafaself.github.io"

command -v gnome-extensions >/dev/null 2>&1 || {
    echo "error: gnome-extensions is not available (GNOME Shell is not installed?)" >&2
    exit 1
}

if gnome-extensions list --enabled | grep -Fxq "langux@rafaself.github.io"; then
    echo "error: the historical Langux extension is enabled and would create a duplicate panel entry" >&2
    echo "Disable it first with: gnome-extensions disable langux@rafaself.github.io" >&2
    exit 1
fi

"$ROOT/scripts/package-gnome-shell-adapter.sh"
gnome-extensions install --force "$ROOT/dist/langux-gnome-shell-adapter.zip"

echo
echo "Langux's GNOME Shell popup adapter is installed for the current user."
echo "Enable it with: gnome-extensions enable $UUID"
echo "On Wayland, log out and back in if Shell does not load the new extension."
