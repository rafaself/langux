#!/usr/bin/env bash
set -euo pipefail

UUID="langux@rafaself.github.io"
REPO="${LANGUX_REPO:-rafaself/Langux}"
TAG="${LANGUX_TAG:-latest}"
ARCHIVE_NAME="langux.zip"
SHA_NAME="langux.zip.sha256"

# Overrides for testing (e.g. file:// URLs with a local mock release).
ZIP_URL="${LANGUX_ZIP_URL:-}"
SHA_URL="${LANGUX_SHA_URL:-}"

command -v curl >/dev/null 2>&1 || { echo "error: curl is required" >&2; exit 1; }
command -v gnome-extensions >/dev/null 2>&1 || {
    echo "error: gnome-extensions is not available (GNOME Shell is not installed?)" >&2
    exit 1
}
if command -v sha256sum >/dev/null 2>&1; then
    SHA_TOOL="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
    SHA_TOOL="shasum -a 256"
else
    echo "error: no SHA-256 tool found (need sha256sum or shasum)" >&2
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

if [ -z "$ZIP_URL" ]; then
    echo "Resolving $ARCHIVE_NAME from GitHub release '$TAG' of $REPO..."
    BASE_URL="https://api.github.com/repos/$REPO/releases"
    RELEASE_JSON="$(curl -fsSL "${BASE_URL}/$([ "$TAG" = latest ] && echo latest || echo "tags/$TAG")" || true)"
    [ -n "$RELEASE_JSON" ] || { echo "error: could not resolve release '$TAG' of $REPO" >&2; exit 1; }
    ZIP_URL="$(printf '%s' "$RELEASE_JSON" | sed -n 's/.*"browser_download_url": *"\([^"]*'"$ARCHIVE_NAME"'\)".*/\1/p' | head -n 1)"
    SHA_URL="$(printf '%s' "$RELEASE_JSON" | sed -n 's/.*"browser_download_url": *"\([^"]*'"$SHA_NAME"'\)".*/\1/p' | head -n 1)"
    [ -n "$ZIP_URL" ] || { echo "error: release '$TAG' has no $ARCHIVE_NAME asset" >&2; exit 1; }
fi
[ -n "$SHA_URL" ] || { echo "error: no checksum file URL available" >&2; exit 1; }

echo "Downloading $ZIP_URL"
curl -fsSL "$ZIP_URL" -o "$WORK/$ARCHIVE_NAME"
echo "Downloading $SHA_URL"
curl -fsSL "$SHA_URL" -o "$WORK/$SHA_NAME"

echo "Verifying SHA-256..."
(cd "$WORK" && $SHA_TOOL -c "$SHA_NAME")

echo "Installing for the current user..."
gnome-extensions install --force "$WORK/$ARCHIVE_NAME"

echo
echo "Langux installed and its checksum verified ($ARCHIVE_NAME)."
echo
echo "Next steps:"
echo "  gnome-extensions enable $UUID"
echo "  Restart the session (log out and back in) or, on X11, press Alt+F2 and type 'r'."