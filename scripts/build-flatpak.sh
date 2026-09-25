#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.rafaself.Langux"
FLATHUB_REPO="https://dl.flathub.org/repo/flathub.flatpakrepo"
MANIFEST="$ROOT/flatpak/$APP_ID.yml"

for command_name in flatpak sha256sum; do
    command -v "$command_name" >/dev/null 2>&1 || {
        echo "error: required command not found: $command_name" >&2
        exit 1
    }
done

if command -v flatpak-builder >/dev/null 2>&1; then
    builder=(flatpak-builder)
elif flatpak run --user --branch=stable org.flatpak.Builder --version >/dev/null 2>&1; then
    builder=(flatpak run --user --branch=stable org.flatpak.Builder)
else
    echo "error: install flatpak-builder or org.flatpak.Builder from Flathub" >&2
    exit 1
fi

version="$(awk '
    /^\[package\]$/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && $1 == "version" && $2 == "=" {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' "$ROOT/Cargo.toml")"
if [[ -z "$version" ]]; then
    echo "error: could not read the Langux package version from Cargo.toml" >&2
    exit 1
fi

if [[ -n "${LANGUX_RELEASE_TAG:-}" && "$LANGUX_RELEASE_TAG" != "v$version" ]]; then
    echo "error: release tag '$LANGUX_RELEASE_TAG' must match Cargo.toml version 'v$version'" >&2
    exit 1
fi

arch="$(flatpak --default-arch)"
artifact_name="langux-$version-$arch.flatpak"
artifact="$ROOT/dist/$artifact_name"
checksum="$artifact.sha256"
mkdir -p "$ROOT/dist"
rm -f -- "$artifact" "$checksum"

cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/langux-flatpak-release"
mkdir -p "$cache_root"
build_root="$(mktemp -d "$cache_root/build.XXXXXXXX")"
trap 'rm -rf -- "$build_root"' EXIT
source_date_epoch="$(git -C "$ROOT" log -1 --format=%ct HEAD)"
source_timestamp="$(date --utc --date="@$source_date_epoch" +%Y-%m-%dT%H:%M:%SZ)"

"${builder[@]}" \
    --user \
    --force-clean \
    --disable-cache \
    --disable-rofiles-fuse \
    --disable-updates \
    --override-source-date-epoch="$source_date_epoch" \
    --install-deps-from=flathub \
    --default-branch=stable \
    --state-dir="$cache_root/state" \
    "$build_root/build" \
    "$MANIFEST"

flatpak build-export \
    --timestamp="$source_timestamp" \
    --arch="$arch" \
    "$build_root/repo" \
    "$build_root/build" \
    stable

flatpak build-bundle \
    "$build_root/repo" \
    "$artifact" \
    "$APP_ID" \
    stable \
    --runtime-repo="$FLATHUB_REPO" \
    --arch="$arch"

(cd "$ROOT/dist" && sha256sum -- "$artifact_name" > "${artifact_name}.sha256")
(cd "$ROOT/dist" && sha256sum --check "${artifact_name}.sha256")

echo "Built $artifact and $checksum"
