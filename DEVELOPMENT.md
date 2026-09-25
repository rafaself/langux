# Development

The active Langux application is a Rust workspace with a GTK 4 interface. It
uses Linux Secret Service for the Google Cloud Translation API key. Normal
startup does not call XDG Desktop Portals. The frozen GJS extension is kept as
historical reference; it is not part of the Rust build.

## Prerequisites

The workspace declares Rust 1.85 as its minimum supported version. CI runs
Rust 1.85.1 with `rustfmt` and `clippy`. Use Rust 1.85 or newer.

On Fedora, install the native development files and D-Bus session tools:

```sh
sudo dnf install gtk4-devel dbus-devel dbus-daemon glib2-devel pkgconf-pkg-config
```

Install rustup if needed, then install Rust with the `rustfmt` and `clippy`
components. For example:

```sh
rustup toolchain install 1.85.1 --component rustfmt --component clippy
```

The Rust build also needs `glib-compile-schemas` to run the application from a
checkout. On Fedora it is provided by the GLib development packages.

## Build, run, and test

From the repository root, compile the settings schema and run the application:

```sh
glib-compile-schemas schemas/
RUSTUP_TOOLCHAIN=1.85.1 GSETTINGS_SCHEMA_DIR="$PWD/schemas" cargo run --locked
```

Running the app requires an active graphical session and an available GTK 4
display backend. Settings are stored with GSettings; the API key is stored
separately through Linux Secret Service.

Run the same quality checks used in CI:

```sh
RUSTUP_TOOLCHAIN=1.85.1 scripts/check-rust.sh
```

The script checks formatting, runs Clippy with warnings denied, runs all
workspace tests, and builds an optimized workspace binary. The tests do not
need a real Google API key; live translation does require one configured in
the desktop's Secret Service. The individual commands are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

To build and launch a debug app without installing it:

```sh
glib-compile-schemas schemas/
RUSTUP_TOOLCHAIN=1.85.1 GSETTINGS_SCHEMA_DIR="$PWD/schemas" cargo run --locked -- --toggle
```

The desktop launcher starts Langux resident with the translator window hidden.
`langux --toggle` starts the app if it is not running, or shows/hides the
window in the existing application instance. Normal startup does not request a
global shortcut. `langux --help` lists supported options. Unsupported options
and positional arguments return a non-zero status.

## Build a Flatpak bundle

The Flatpak manifest uses the GNOME 51 runtime and SDK and the Rust SDK
extension. On Fedora, install Flatpak and D-Bus session tools, configure
Flathub, and install Flatpak Builder:

```sh
sudo dnf install flatpak dbus-daemon
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.flatpak.Builder
```

Build a bundle from the current checkout, then install it:

```sh
dbus-run-session -- scripts/build-flatpak.sh
flatpak install --user --bundle "dist/langux-1.0.0-$(flatpak --default-arch).flatpak"
flatpak run io.github.rafaself.Langux
```

The generated bundle version comes from the root `Cargo.toml`; update the
filename above if that package version changes. The builder resolves the GNOME
runtime, SDK, and Rust extension from Flathub. Rust crate sources are vendored
in `flatpak/cargo-sources.json`, and the build runs offline against
`Cargo.lock`. If the lockfile changes, regenerate the vendored sources with the
[Flatpak Cargo source generator](https://github.com/flatpak/flatpak-builder-tools/tree/41c20aa10819cdb2a4f3ca171758a96d1955c018/cargo).

The sandbox grants only the `org.freedesktop.secrets` and
`org.kde.StatusNotifierWatcher` session-bus names. The SNI item uses a name
under `io.github.rafaself.Langux`, which Flatpak allows the app to own by
default. The manifest does not grant the full session bus or a wildcard
third-party bus namespace. Langux has no start-on-login option, does not add
an autostart entry, and does not request the XDG Background portal.

Tray display requires a desktop StatusNotifierWatcher and a registered tray
host. If either is unavailable, Langux stays hidden after normal startup and
logs a warning; `flatpak run io.github.rafaself.Langux --toggle` remains an
explicit way to show or hide the window.

The bundle build writes a SHA-256 checksum beside the artifact. For a
version-tagged release, the tag must match the package version in
`Cargo.toml`, for example `v1.0.0` for package version `1.0.0`. The CI and
release workflows run the Rust checks before packaging; release bundles are
published with a SHA-256 checksum that verifies the downloaded artifact's
bytes.

Rust crates are locked by `Cargo.lock` and their vendored source checksums.
The build normalizes its source date epoch and the timestamp on the exported
OSTree commit. This does not promise bit-for-bit identical bundles across clean
builds: the GNOME 51 runtime and SDK branches and the `rust-stable` SDK
extension are resolved from Flathub when building and can be updated without
a source change. The `flatpak build-export --timestamp` option controls the
OSTree commit timestamp embedded in the bundle; it does not pin those build
inputs. The checksum lets users verify the exact published artifact, not that
a rebuild will have identical bytes. Existing `v0.1.0` and `v0.1.1` tags
predate the rewrite and are the historical extension releases.

## Application activation and shortcuts

The app uses its stable GApplication ID and command-line handler to route
launches in one desktop session to the resident primary process. A normal
launch stays hidden. `--toggle` sends the shared Toggle action to show or hide
the translator window. The shared action interface also provides Quit for
desktop integrations. Normal startup does not request a global shortcut or
open the window.

For a Flatpak install, run `flatpak run io.github.rafaself.Langux --toggle`.
For a native install, use `langux --toggle`. A custom desktop shortcut can
invoke the same command.

- GNOME: add a custom shortcut in **Settings → Keyboard → View and Customize
  Shortcuts → Custom Shortcuts**.
- Hyprland: add a binding to an included Hyprland configuration:

  ```ini
  bind = SUPER, T, exec, flatpak run io.github.rafaself.Langux --toggle
  ```

  For an Omarchy Lua binding, use `~/.config/hypr/bindings.lua`:

  ```lua
  o.bind("SUPER + T", "Toggle Langux", "flatpak run io.github.rafaself.Langux --toggle")
  ```

## Validation limits

See [`COMPATIBILITY.md`](COMPATIBILITY.md) for the evidence collected for
issue #29. The same Flatpak was exercised on Fedora Workstation with GNOME;
the Omarchy/Hyprland run was deferred and remains unverified. That record also
lists the GTK interactions and other end-to-end checks that could not be
completed in the available environment.
