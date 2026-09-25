# Langux

<p align="left">
  <img src="data/icon-readme.svg" width="64" height="64" alt="Langux icon">
</p>

Langux is a keyboard-first, local-first translator for GNOME Shell: open a popup,
type or paste text, translate it with Google Cloud Translation Basic v2, and copy
the result. Translation requests go directly from your machine to Google; Langux
has no backend, account system, telemetry, or persistent translation history.

<img width="493" height="333" alt="image" src="https://github.com/user-attachments/assets/167fb53b-2986-47e8-8806-8ae155a7246c" />

## Features

- Open or toggle the translator with a configurable shortcut (`Super+T` by default).
- Translate while typing after one second of inactivity, or use explicit Enter mode.
- Use `Shift+Enter` for a new line; `Enter` and `Ctrl+Enter` translate in manual mode.
- Detect the source language automatically, choose source and target languages, and swap them.
- Copy translated text only through an explicit **Copy** action.
- Optionally reuse successful translations with a bounded in-memory cache, disabled by default.
- Store the Google API key in GNOME Keyring through libsecret, never in GSettings.
- Check manually for stable releases from the Preferences window; updates are never automatic.

## Compatibility and requirements

- GNOME Shell 49 on GNU/Linux, under X11 or Wayland.
- GJS with modern ES modules, plus the GNOME 49 system libraries `libsoup3` and `libsecret`.
- `gnome-extensions`, `curl`, and a SHA-256 tool (`sha256sum` or `shasum`) for release installation.
- A Google Cloud project with billing enabled and the Cloud Translation API enabled.

Other GNOME Shell versions may work, but only the version listed in
[`metadata.json`](metadata.json) is tested and supported by this project.

## Install a released version

Version tags matching the Rust package version (for example, `v0.1.2` for
`version = "0.1.2"` in `Cargo.toml`) run the release checks and publish an
x86_64 Flatpak bundle plus a SHA-256 checksum. The bundle is built from the
tagged source with the locked Rust dependencies.

Download both files from the GitHub Release, configure Flathub, and verify the
bundle before installing it. Replace the example filename with the version you
downloaded:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
sha256sum --check langux-0.1.2-x86_64.flatpak.sha256
flatpak install --user --bundle ./langux-0.1.2-x86_64.flatpak
flatpak run io.github.rafaself.Langux
```

The Flatpak bundle contains the application. Flatpak obtains the GNOME runtime
from Flathub when it is not already installed.

## Install from a checkout

Use this path to test local changes:

```sh
git clone https://github.com/rafaself/langux.git
cd langux
npm ci
npm run check
scripts/dev-install.sh
gnome-extensions enable langux@rafaself.github.io
```

`scripts/dev-install.sh` packages the checked-out extension and installs it for the
current user. The development dependencies are not included in the extension
archive.

## Build and run the standalone Flatpak

Add Flathub and install Flatpak Builder, the GNOME 51 runtime and SDK, and the
matching Rust toolchain extension:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder//stable org.gnome.Platform//51 org.gnome.Sdk//51 org.freedesktop.Sdk.Extension.rust-stable//26.08
```

From the repository root, build and install the current checkout, then launch or
toggle the application:

```sh
flatpak run --user --branch=stable org.flatpak.Builder \
  --user --install --force-clean --install-deps-from=flathub \
  --state-dir="$HOME/.cache/langux-flatpak-builder/state" \
  --repo="$HOME/.cache/langux-flatpak-builder/repo" \
  "$HOME/.cache/langux-flatpak-builder/build" "$PWD/flatpak/io.github.rafaself.Langux.yml"
flatpak run io.github.rafaself.Langux --toggle
```

The manifest uses the GNOME 51 runtime and SDK. Its sandbox permissions are
limited to network access for translation, Wayland with fallback X11 and IPC for
the GTK window, the Cairo renderer so no GPU device access is needed, and the
Secret Service D-Bus name for API-key storage. Clipboard and XDG portal access use
Flatpak's portal proxy; no host filesystem or device access is requested.

The Flatpak build is offline and pinned to `Cargo.lock`. If that lockfile changes,
regenerate `flatpak/cargo-sources.json` with the [Flatpak Cargo source generator](https://github.com/flatpak/flatpak-builder-tools/tree/41c20aa10819cdb2a4f3ca171758a96d1955c018/cargo).

To create a standalone bundle and checksum locally, install either the
`flatpak-builder` command or the Flathub `org.flatpak.Builder` app, then run:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
scripts/build-flatpak.sh
```

The build script uses the current Git commit time for `SOURCE_DATE_EPOCH`, the
locked Cargo dependency sources, and the manifest's GNOME 51 runtime. Flatpak's
bundle format also records its generation time, so separate bundle files can
have different checksums even when they contain the same exported application
commit. Release automation repeats the quality checks and packaging for the
version tag before publishing the bundle and its checksum. The checksum verifies
the downloaded artifact against the published file.

## Configure Google Cloud Translation

1. Create or select a Google Cloud project and enable billing.
2. Enable the **Cloud Translation API**.
3. Create an API key and restrict it to the Cloud Translation API. Add appropriate
   application restrictions and project quotas or budget alerts where possible.
4. Open Langux Preferences, select **Google Cloud → Configure**, and paste the key.
   Use **Replace** to change it or **Remove** to delete it.

The key is stored in GNOME Keyring and is never written to Langux settings, files,
logs, URLs, or the repository. Langux sends it to Google only in the
`X-Goog-Api-Key` HTTPS request header.

## Preferences and behavior

The defaults are source language `auto`, target language `en`, live translation
enabled, and translation caching disabled.

- Live mode sends non-blank text after it has remained unchanged for one second.
- Manual mode sends text only after `Enter` or `Ctrl+Enter`; `Shift+Enter` inserts a newline.
- The cache is session-only and can hold 0–1000 successful translations. Setting it to
  zero disables it; disabling or clearing the cache removes existing entries.
- The cache is cleared when the extension is disabled and is never written to disk.

## Global shortcuts

Langux requests a `Super+T` shortcut through the XDG GlobalShortcuts portal when
it starts. Portal shortcuts are tied to the running Langux process; closing the
last window exits the app and releases the portal binding. To activate Langux
after its window has been closed, configure your desktop or window manager to
run `langux --toggle` for the shortcut you want. This command launches Langux
when it is not running and toggles the window when it is already running.

- On GNOME, add `langux --toggle` as a custom keyboard shortcut in Keyboard
  Settings.
- On Hyprland, add a binding such as `bind = SUPER, T, exec, langux --toggle` to
  your Hyprland configuration.

## Privacy and data flow

- Translation text is sent directly from the local machine to Google Cloud Translation
  over HTTPS when live or manual translation is triggered.
- Input, output, API keys, and update responses are not written to disk or logs.
- Translated text is written to the system clipboard only when **Copy** is explicitly used.
- The optional cache stays in memory for the current Shell session and is disabled by default.
- Manual update checks contact only the fixed GitHub Releases API and request release
  metadata. Langux does not download, install, or reload updates by itself.

See [`SECURITY.md`](SECURITY.md) for the threat model and vulnerability reporting
instructions.

## Uninstall

```sh
gnome-extensions uninstall langux@rafaself.github.io
```

Removing the extension does not remove the API key from GNOME Keyring. Delete it
from Langux Preferences before uninstalling, or remove it later with `seahorse`.

## Development and checks

Install the pinned development tools with `npm ci`, then run the aggregate check:

```sh
npm run check             # syntax, tests, Biome, schema, and GNOME runtime probe
npm run format:check      # check formatting without changing files
npm run format            # intentionally format the scoped JavaScript files
scripts/package.sh        # build dist/langux.zip and its checksum
```

The pure-module tests can also be run independently:

```sh
npm test
```

For live Shell logs:

```sh
journalctl -f -o cat /usr/bin/gnome-shell
journalctl -f | grep -iE "langux|error|critical"
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the contribution workflow.

## License

Langux is licensed under [GPL-3.0-or-later](LICENSE).
