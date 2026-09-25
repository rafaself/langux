# Langux 1.0 rewrite policy

The last GNOME Shell extension release is
[v0.1.1](https://github.com/rafaself/langux/releases/tag/v0.1.1), preserved at
the `v0.1.1` Git tag. The earlier `v0.1.0` release remains available as well.
Both releases are historical and are not the standalone application.

The active Langux application is the greenfield Rust and GTK 4 rewrite tracked
by [epic #9](https://github.com/rafaself/langux/issues/9). The root Cargo
workspace, `src/`, and `crates/` are its architecture and source of truth. The
GJS extension source is retained for release history and behavioral reference;
do not port its modules or maintain it as a parallel product.

The official desktop targets are Fedora Workstation with GNOME and Omarchy
with Hyprland, both on Wayland. One application and Flatpak are intended to
serve both environments. Current validation and deferred checks are recorded
in [`COMPATIBILITY.md`](COMPATIBILITY.md); target status must not be presented
as a completed compatibility test.
