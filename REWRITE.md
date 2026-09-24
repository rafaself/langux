# Langux 1.0 rewrite

The final GNOME Shell extension release is
[v0.1.1](https://github.com/rafaself/langux/releases/tag/v0.1.1), preserved at
the `v0.1.1` Git tag. A GitHub tag ruleset blocks updates and deletion of this
tag. The earlier `v0.1.0` release remains available as well.

Work under [epic #9](https://github.com/rafaself/langux/issues/9), beginning
with issue #11, targets a single standalone Rust/GTK application. Treat the
extension only as a behavioral and product reference. Do not port its GJS
modules, preserve GNOME Shell internals, add compatibility layers for them, or
maintain a parallel extension implementation. The new application is the only
architectural source of truth for Langux 1.0.
