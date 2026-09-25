# Langux

<p align="left">
  <img src="data/icon-readme.svg" width="64" height="64" alt="Langux icon">
</p>

Langux is a quick translator for the Linux desktop. The active application is a
standalone Rust and GTK 4 app: keep it resident in the desktop session, open or
hide its window, type or paste text, translate with Google Cloud Translation
Basic v2, and copy the result. Translation requests go directly from your
computer to Google. Langux has no backend, accounts, telemetry, or persistent
translation history.

## Features

- Start Langux without opening its window; run `langux --toggle` to show or
  hide the translator.
- Focus the input when the window opens; use `Escape` to close it.
- Translate after one second without input, or select manual translation.
- Use `Shift+Enter` to insert a line. In manual mode, `Enter` or `Ctrl+Enter`
  translates; in live mode, `Ctrl+Enter` translates immediately.
- Detect the source language, select source and target languages, and swap them.
- Copy the result only through the **Copy** button (`Alt+C`).
- Optionally reuse successful translations with a bounded in-memory cache,
  disabled by default.
- Store the Google API key in the desktop's Linux Secret Service, not in
  application settings or a plaintext file.
- Distribute the same application as a Flatpak for the official GNOME and
  Hyprland targets.

## Supported environments and validation

Langux's official target environments are Fedora Workstation with GNOME on
Wayland and Omarchy with Hyprland on Wayland. These are targets for one app and
one Flatpak, not separate desktop-specific editions. Other Linux desktops may
work, but are outside the project's support guarantee.

The available desktop validation is recorded in
[`COMPATIBILITY.md`](COMPATIBILITY.md). The Flatpak was built and exercised on
Fedora Linux 43 with GNOME Shell 49.10. **The Omarchy/Hyprland run is deferred**
and has not passed. The compatibility record also lists unverified GTK
interactions and live translation. The project does not claim that the full
cross-desktop workflow has been validated.

## Install

See the [GitHub Releases](https://github.com/rafaself/langux/releases) page for
available downloads. The existing `v0.1.0` and `v0.1.1` releases are the
historical GNOME Shell extension and do not install the standalone app. No
standalone Flatpak release is published yet; until one is available, build the
app from this checkout using the [development guide](DEVELOPMENT.md).

Future standalone releases will include a Flatpak bundle and a SHA-256
checksum. After downloading both files into the same directory, verify and
install the bundle with:

```sh
sha256sum --check langux-VERSION-ARCH.flatpak.sha256
flatpak install --user --bundle ./langux-VERSION-ARCH.flatpak
flatpak run io.github.rafaself.Langux --toggle
```

Replace `VERSION` and `ARCH` with the names in the release assets. The Flatpak
runtime is downloaded from Flathub if it is not already installed.

## Build and run from a checkout

For a native development build, install GTK 4, D-Bus, and Rust development
tools, then follow [DEVELOPMENT.md](DEVELOPMENT.md). To build the same Flatpak
format used for release artifacts, install Flatpak and Flathub's Builder app,
then run from the repository root:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder
dbus-run-session -- scripts/build-flatpak.sh
```

Install and launch the locally built bundle:

```sh
flatpak install --user --bundle "dist/langux-1.0.0-$(flatpak --default-arch).flatpak"
flatpak run io.github.rafaself.Langux
```

The `1.0.0` in the bundle filename is the package version in this checkout;
use the version printed by the generated artifact if it has changed.

The manifest uses the GNOME 51 runtime and SDK. It grants network access for
translation, Wayland with fallback X11 and IPC for the GTK window, and access
to the desktop Secret Service for the API key. GTK/GDK clipboard access uses
the selected Wayland or X11 display connection. The app does not currently
call XDG Desktop Portals or request host filesystem or device access.

## Configure Google Cloud Translation

1. Create or select a Google Cloud project, enable billing, and enable the
   **Cloud Translation API**.
2. Create an API key restricted to the Cloud Translation API. Set appropriate
   application restrictions and project quotas or budget alerts where
   possible.
3. Open Langux **Settings**, paste the key, and choose **Save**. Use **Replace**
   to change it or **Remove** to delete it.

Langux stores the key through the desktop's Linux Secret Service. The saved key
is hidden in the UI and is never written to GSettings, application files, URLs,
or logs. For a translation request, Langux reads it into memory and sends it to
Google in the `X-Goog-Api-Key` HTTPS header. Translation text is also sent
directly to Google when a translation is triggered. Google Cloud billing and
data policies apply to those requests.

## Preferences and behavior

The defaults are source language `auto`, target language `en`, live translation
enabled, and translation caching disabled. Preferences store only source and
target languages, translation mode, and cache settings.

- Live mode translates non-blank input after it has remained unchanged for one
  second.
- Manual mode translates on `Enter` or `Ctrl+Enter`; `Shift+Enter` inserts a
  newline. In live mode, `Enter` inserts a newline and `Ctrl+Enter` translates
  immediately.
- The optional cache holds up to 1000 successful translations in memory. It is
  disabled by default and can be disabled by setting its capacity to zero.
- The cache is discarded when the application process exits. Translation
  history is never written to disk.

## Opening and hiding from a command line

The desktop launcher starts one resident Langux process with its window hidden.
To open or hide the translator from a terminal, pass `--toggle`:

For a Flatpak install, run:

```sh
flatpak run io.github.rafaself.Langux --toggle
```

For a native install, run `langux --toggle`. A custom desktop shortcut can run
the same command if needed.

- GNOME: add the command under **Settings → Keyboard → View and Customize
  Shortcuts → Custom Shortcuts**.
- Hyprland: add a binding such as
  `bind = SUPER, T, exec, flatpak run io.github.rafaself.Langux --toggle` to
  the Hyprland configuration.

## Privacy and data flow

- Translation text and the API key go directly from Langux to Google Cloud
  Translation over HTTPS when a translation is triggered. Langux operates no
  translation proxy or backend.
- The API key is stored through Linux Secret Service and is not stored in
  GSettings or a plaintext application file.
- Source text, results, and the optional cache stay in memory while in use;
  Langux does not maintain persistent history or write translation content to
  application settings.
- The cache is disabled by default. Copying a result writes it to the system
  clipboard only after an explicit **Copy** action.
- Langux has no user accounts, telemetry, analytics, or automatic update
  installer.

See [`SECURITY.md`](SECURITY.md) for the threat model and vulnerability
reporting instructions. See [`COMPATIBILITY.md`](COMPATIBILITY.md) for the
current platform validation and its limits.

## Legacy GNOME Shell extension

The GJS extension is frozen and is not part of the active Langux application.
Its last release, [v0.1.1](https://github.com/rafaself/langux/releases/tag/v0.1.1),
and its source tag remain available for users who need the old extension.
Historical extension setup notes are in
[`RELEASE_NOTES.md`](RELEASE_NOTES.md). The extension's GJS files and install
scripts in this repository are not used to build or run the Rust app.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the Rust contribution workflow and
[`DEVELOPMENT.md`](DEVELOPMENT.md) for local build and validation commands.

## License

Langux is licensed under [GPL-3.0-or-later](LICENSE).
