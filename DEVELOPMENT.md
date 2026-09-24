# Development

The standalone Langux application uses Rust and GTK4. On Fedora, install the
native GTK4 development files and a Rust toolchain with Cargo, rustfmt, and
Clippy:

```sh
sudo dnf install gtk4-devel pkgconf-pkg-config
rustup component add rustfmt clippy
```

Run these commands from the repository root:

| Task | Command |
| --- | --- |
| Build | `cargo build --workspace` |
| Run the GTK application | `cargo run` |
| Format | `cargo fmt --all` |
| Check formatting | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Test | `cargo test --workspace` |

Running the application requires an active graphical session. The Rust
workspace is independent of the frozen GNOME Shell extension and does not
require GJS, GNOME Shell, St, or Clutter.
