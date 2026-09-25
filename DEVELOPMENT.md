# Development

The standalone Langux application uses Rust and GTK4. On Fedora, install the
native GTK4 development files and a Rust toolchain with Cargo, rustfmt, and
Clippy:

```sh
sudo dnf install gtk4-devel dbus-devel pkgconf-pkg-config
rustup component add rustfmt clippy
```

Run these commands from the repository root:

| Task | Command |
| --- | --- |
| Build | `cargo build --workspace` |
| Run the GTK application | `GSETTINGS_SCHEMA_DIR="$PWD/schemas" cargo run` |
| Format | `cargo fmt --all` |
| Check formatting | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace` |

Running the application requires an active graphical session. The Rust
workspace is independent of the frozen GNOME Shell extension and does not
require GJS, GNOME Shell, St, or Clutter.

Before the first local run, compile the app's settings schema with
`glib-compile-schemas schemas/`. Installed packages provide the compiled
schema through the normal GSettings schema path. Langux stores translation
defaults and cache preferences in GSettings; the API key remains in Linux
Secret Service.

## Application activation

The app uses its stable GApplication ID to route launches in one desktop
session to a single running instance. Launching `langux` shows the translator
and focuses its input field. `langux --toggle` hides a visible window, or shows
and focuses it when hidden; when no instance is running, it starts and shows
one. `langux --help` prints the supported options. Unsupported options and
positional arguments fail with a non-zero status.

## Global activation

At startup, Langux asks the XDG Desktop Portal GlobalShortcuts interface to bind
the **Show or hide Langux** action, preferring `Super+T`. The desktop may ask you
to approve or choose the key. If the portal is missing, unavailable, or the
binding is declined, the app remains usable and `langux --toggle` continues to
work.

For GNOME, add a custom shortcut in **Settings → Keyboard → View and Customize
Shortcuts → Custom Shortcuts** and use `langux --toggle` as its command.

For Hyprland, add a binding to an included Hyprland config:

```ini
bind = SUPER, T, exec, langux --toggle
```

Omarchy's current Lua configuration can use `~/.config/hypr/bindings.lua`:

```lua
o.bind("SUPER + T", "Toggle Langux", "langux --toggle")
```
