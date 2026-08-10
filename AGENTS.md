# AGENTS.md

## Project

Langux is a local-first GNOME Shell 49 quick translator. The v0.1 workflow is
open → type or paste → translate with Google Cloud Translation Basic v2 → copy.
Live translation is enabled by default; explicit mode translates on Enter or
Ctrl+Enter, while Shift+Enter inserts a newline. The optional cache is disabled
by default. There is no backend, account, telemetry, or persistent history.

## Before changing code

- Read `metadata.json`, `extension.js`, and the relevant issue first.
- Keep changes within the issue and keep modules small; add a module instead of
  making an existing one unnecessarily large.
- Preserve the current architecture: `extension.js` is the Shell entry point,
  `prefs.js` is the GTK4/libadwaita entry point, `schemas/` contains GSettings,
  `tests/` contains pure-module tests, and `scripts/` contains developer checks.

## Rules

- Use modern GJS ES modules (`gi://` and `resource:///org/gnome/shell/...`);
  do not use legacy `imports.js`.
- Keep GTK4/libadwaita in preferences code only. Keep pure `ui/` helpers and
  service modules free of Shell/GTK imports when possible.
- Target GNOME Shell 49 and list only tested versions in `metadata.json`.
- Do not add Node/npm runtime dependencies. Ask for confirmation before adding
  a production dependency.
- Store API keys only in libsecret/GNOME Keyring, never in GSettings. Avoid
  security-sensitive changes; if one is necessary, explain the risk and get
  confirmation before proceeding.
- Preserve the fixed 1000 ms live-translation debounce and the manual-mode
  keyboard semantics.
- Every actor, signal, keybinding, and network request created by `enable()`
  must be cleaned up by `disable()`.
- Add or update domain-behavior tests when changing pure modules; do not change
  tests merely to make the runner pass.

## Validation

After installing the pinned developer tools with `npm ci`, run the checks
relevant to the files changed:

```sh
npm run check                             # syntax, tests, Biome, schema, runtime probe
npm run format:check                      # formatting-only check
gsettings --schemadir schemas/ list-keys org.gnome.shell.extensions.langux
gsettings --schemadir schemas/ get org.gnome.shell.extensions.langux source-language
```

For Shell changes, install the extension and run an isolated headless lifecycle
check: enable → disable → enable. Verify that the log has no Langux errors and
that re-enabling does not duplicate actors or signals. Keep GSettings isolated
with `GSETTINGS_BACKEND=memory`.

Interactive GTK preferences are verified manually in a real session; automated
coverage is limited to imports, static checks, and pure-module tests.

## Delivery

Review the diff for scope, lifecycle, secret-handling, and regression risks.
Do not commit, push, open a PR, or close an issue unless the task explicitly
requests it.
