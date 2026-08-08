#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UUID="langux@rafaself.github.io"

command -v gnome-extensions >/dev/null 2>&1 || {
    echo "error: gnome-extensions is not available (GNOME Shell is not installed?)" >&2
    exit 1
}

"$ROOT"/scripts/package.sh

gnome-extensions install --force "$ROOT"/dist/langux.zip

echo
echo "Langux installed for the current user."
echo
echo "Next steps:"
echo "  gnome-extensions enable $UUID"
echo "  Restart the session (log out and back in) or, on X11, press Alt+F2 and type 'r'."
