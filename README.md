# Langux

<p align="left">
  <img src="data/icon-readme.svg" width="64" height="64" alt="Langux icon">
</p>

Langux is a quick translator for the Linux desktop. Its resident Rust
application handles preferences, credentials, providers, and Google Cloud
Translation Basic v2 requests. On GNOME Shell, a thin Shell adapter presents
the translator in a compact popup below its panel icon; other desktops can
open the GTK window through the StatusNotifier tray item or `--toggle`.
Translation requests go directly from your computer to Google. Langux has no
backend, accounts, telemetry, or persistent translation history.

## Features

- Keep Langux resident without opening a standalone translator window; on
  GNOME Shell, click the panel icon to open the anchored popup.
- Use the panel icon's right-click menu to quit; close or dismiss the popup to
  return to the tray-only state.
- On other desktops, run `langux --toggle` to show or hide the GTK window.
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

### GNOME Shell popup

The compact GNOME popup is provided by a separate Shell extension. If the
historical Langux extension is enabled, disable it first to avoid a duplicate
panel entry. Then build and install the new adapter after installing the
Langux desktop application:

```sh
scripts/install-gnome-shell-adapter.sh
gnome-extensions enable langux-shell@rafaself.github.io
```

Disable the historical extension with
`gnome-extensions disable langux@rafaself.github.io` when it is present.

The adapter starts Langux hidden when it is enabled in the session and displays
the translator directly below its panel icon. It owns only the GNOME
presentation; the Rust application still handles translation, settings, and
credentials. Disabling the adapter restores the StatusNotifier tray item. The
adapter currently declares GNOME Shell 49 compatibility; its end-to-end host
validation is tracked separately in [`COMPATIBILITY.md`](COMPATIBILITY.md).

If the adapter is disabled, GNOME Shell needs a StatusNotifier host to display
the fallback tray item. Install the official [AppIndicator and
KStatusNotifierItem Support
extension](https://extensions.gnome.org/extension/615/appindicator-support/)
with a release marked active for your GNOME Shell version, then enable it in
the Extensions app. To check whether a host is registered, run:

```sh
gdbus call --session --dest org.kde.StatusNotifierWatcher \
  --object-path /StatusNotifierWatcher \
  --method org.freedesktop.DBus.Properties.Get \
  org.kde.StatusNotifierWatcher IsStatusNotifierHostRegistered
```

A result containing `<true>` means a host is registered. If no watcher or host
is available, Langux logs a warning and keeps its window hidden. Open it with
`flatpak run io.github.rafaself.Langux --toggle`, or use `langux --toggle` for
a native installation.

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

The historical translation extension is frozen. Its last release,
[v0.1.1](https://github.com/rafaself/langux/releases/tag/v0.1.1), and source
tag remain available for users who need that old extension. Historical setup
notes are in [`RELEASE_NOTES.md`](RELEASE_NOTES.md). The isolated
`gnome-shell-adapter/` extension is new presentation code; it does not use the
historical extension's translation logic or files.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the Rust contribution workflow and
[`DEVELOPMENT.md`](DEVELOPMENT.md) for local build and validation commands.

## License

Langux is licensed under [GPL-3.0-or-later](LICENSE).
