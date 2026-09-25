# AGENTS.md

## Working agreements

- Unless a dependency is highly consolidated, safe, and trustable, ask before
  adding it to production code.
- Avoid changes that could introduce security vulnerabilities. If a security-
  sensitive change is necessary, explain the risk and get confirmation before
  proceeding.

## Project

Langux is a local-first Linux desktop translator. The active application is a
standalone Rust and GTK 4 app, distributed primarily as a Flatpak. Its v1.0
workflow is open or toggle → type or paste → translate with Google Cloud
Translation Basic v2 → copy. Live translation is enabled by default with a
fixed one-second debounce. Manual mode translates on Enter or Ctrl+Enter;
Shift+Enter inserts a newline. The optional cache is disabled by default and
stays in memory. There is no Langux backend, account system, telemetry,
analytics, or persistent translation history.

The JavaScript/GJS GNOME Shell extension is frozen historical code. Its last
release remains available at the `v0.1.1` tag. It is a behavioral reference,
not the active architecture or an implementation dependency.

## Before changing code

- Read the relevant issue and the surrounding code before editing. For
  architecture or packaging changes, read the root `Cargo.toml`, relevant
  crate manifest, and Flatpak manifest first.
- Keep changes within the issue and modules small. Put platform-independent
  translation behavior in `crates/langux-core`; keep Linux Secret Service
  integration in `crates/langux-secret-service`; keep GTK and desktop wiring
  in `src/`.
- Keep provider behavior independent of GTK presentation. Prefer Freedesktop
  and XDG interfaces for desktop integration; do not add GNOME Shell or
  Hyprland-specific runtime dependencies to the app core.
- Do not add Node/npm runtime dependencies or revive the GJS extension as the
  active implementation.

## Rules

- Target Linux desktop environments through Rust 2024 and GTK 4. The workspace
  declares Rust 1.85 as its minimum supported version; keep CI and docs aligned
  with that MSRV.
- Store API keys only through Linux Secret Service, never in GSettings,
  plaintext files, URLs, or logs. Do not expose or redisplay a saved key.
- Preserve the fixed 1000 ms live-translation debounce and manual-mode
  keyboard behavior unless an issue explicitly changes them.
- Tie GTK signals, timers, translation tasks, and portal sessions to their
  owning window or application lifecycle. Cancel or release them when their
  owner ends.
- Add or update domain-behavior tests when changing pure Rust modules. Do not
  change tests merely to make a runner pass.
- Keep compatibility statements tied to evidence in `COMPATIBILITY.md`;
  distinguish target environments from environments already validated.

## Validation

Install the native libraries listed in `DEVELOPMENT.md`, then run the Rust
checks relevant to changed files. For the full workspace check:

```sh
scripts/check-rust.sh
```

It runs formatting, Clippy, tests, and a release build. To launch a native
development build, compile the GSettings schema and set its directory:

```sh
glib-compile-schemas schemas/
GSETTINGS_SCHEMA_DIR="$PWD/schemas" cargo run --locked
```

GTK interaction requires a graphical session. Credential tests or live
translation may require an available Secret Service; never place a real API
key in test data. For Flatpak and desktop compatibility checks, follow
`DEVELOPMENT.md` and update `COMPATIBILITY.md` with the environment, revision,
results, and limits. Do not claim unperformed Hyprland validation.

## Delivery

Review the diff for issue scope, secret handling, lifecycle, and regression
risks. Use lowercase Conventional Commit subjects in the form
`type(scope): imperative message`. Do not commit, push, open a PR, or close an
issue unless the task explicitly requests it.
