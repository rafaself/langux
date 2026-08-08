# AGENTS.md

## Repository purpose

Langux is a local-first quick translator for GNOME Shell: press a shortcut, type or paste
text, translate with Google Cloud Translation (Basic v2), copy the result. No backend, no
accounts, no telemetry, no history. See the epic in issue #1 and implementation issues #2-#8.

## Repository layout

- `extension.js` — main entry point for the GNOME Shell process.
- `metadata.json` — extension metadata (UUID `org.gnome.shell.extensions.langux`).
- `stylesheet.css`, `stylesheet-dark.css` — Shell UI styling (light and dark theme applied by shell; the dark variant is loaded when the theme name contains "dark").
- `schemas/org.gnome.shell.extensions.langux.gschema.xml` — GSettings schema.
- `ui/languages.js` — pure language list/helpers (no Shell imports; unit-testable).
- `ui/translatorPopup.js` — the translator popup (St/Clutter) and its keyboard-first UX.
- `tests/` — unit tests for pure modules, run with the Node built-in test runner.
- Planned (later issues): `prefs.js` (GTK4/libadwaita), service layer (Soup 3).

Mandatory reading before changing code: `metadata.json`, `extension.js`, and the issue being
implemented. Keep files small; add a new module instead of growing existing ones.

## Working rules

- Use modern GJS ES modules only: `gi://` for libraries, `resource:///org/gnome/shell/…` for
  shell modules (`extension.js`, `panelMenu.js`, `main.js`, …). No imports.js legacy style.
- Compatibility target is GNOME Shell 49; declare in `metadata.json` only versions actually
  tested on.
- Do not import GTK4/libadwaita into the Shell process; they belong to prefs only.
- Do not add Node/npm runtime dependencies; plain JavaScript only.
- Never store API keys or secrets in GSettings. Secrets belong in libsecret/GNOME Keyring
  (issue #4).
- Never translate automatically on keystrokes; translation starts only on explicit user
  action (e.g. `Ctrl+Enter`).
- `enable()` must create and `disable()` must destroy every actor, signal, keybinding, and
  network request created while enabled. Nothing alive after `disable()`.
- Keep scope to the issued checklist; anything not needed for open → translate → copy stays
  out of v0.1.
- Use TDD whenever convenient: write a failing test first, then the minimal code to make it
  pass, then refactor. Skip it only when the change is trivially verifiable through the
  headless lifecycle check or when testing would outweigh the code being tested.
- Tests must reflect the project's domain and expected behavior (translation workflow,
  lifecycle safety, correct state transitions), not just keep the suite green; never change
  or add tests merely to satisfy the runner.

## Useful commands

```sh
# Compile the GSettings schema (required after editing gschema.xml)
glib-compile-schemas schemas/

# Inspect schema keys/defaults from the repo copy
gsettings --schemadir schemas/ list-keys org.gnome.shell.extensions.langux
gsettings --schemadir schemas/ get org.gnome.shell.extensions.langux source-language

# JavaScript syntax check (gjs --check was removed in gjs 1.85+)
node --input-type=module --check < extension.js

# Node's built-in test runner for the pure ui/ modules (no npm dependencies)
node --test

# Test the extension in an installed copy
UUID=org.gnome.shell.extensions.langux
rm -rf ~/.local/share/gnome-shell/extensions/$UUID
mkdir -p ~/.local/share/gnome-shell/extensions/$UUID/ui
cp -r metadata.json extension.js stylesheet.css stylesheet-dark.css \
  ~/.local/share/gnome-shell/extensions/$UUID/
cp ui/languages.js ui/translatorPopup.js ~/.local/share/gnome-shell/extensions/$UUID/ui/
cp -r schemas ~/.local/share/gnome-shell/extensions/$UUID/ && \
  rm ~/.local/share/gnome-shell/extensions/$UUID/schemas/gschemas.compiled
glib-compile-schemas ~/.local/share/gnome-shell/extensions/$UUID/schemas/
```

## Verification

Run the schema compile + defaults checks, the syntax check, and `node --test` after every
change, then exercise the full lifecycle. The live session ignores changes until the next
shell start, so run a headless shell on an isolated bus:

```bash
dbus-run-session -- bash -c '
  export XDG_RUNTIME_DIR=$(mktemp -d)
  export GSETTINGS_BACKEND=memory   # isolate dconf; never writes the real session
  gnome-shell --headless --virtual-monitor 1280x800 > /tmp/langux-shell.log 2>&1 &
  until busctl --user list 2>/dev/null | grep -q org.gnome.Shell; do sleep 0.5; done
  gnome-extensions enable org.gnome.shell.extensions.langux
  gnome-extensions info org.gnome.shell.extensions.langux   # must show State: ACTIVE
  gnome-extensions disable org.gnome.shell.extensions.langux # must show State: INACTIVE
  gnome-extensions enable org.gnome.shell.extensions.langux  # must stay ACTIVE (no duplicates)
  grep -i "langux" /tmp/langux-shell.log   # no errors
'
```

Note: in a freshly started headless shell the very first `enable` may report `State:
INITIALIZED` and only settle to `ACTIVE` on a disable/enable cycle; stock extensions behave
the same in the harness, so judge by the log (no CRITICAL/error lines) and the re-enable
state, not by the first `info` call.

Definition of done for an issue: checklist checked against a real headless shell run,
`extension.js` loads/enables/disables without errors, no duplicate actors or signals on
re-enable, GSettings defaults verified, schema compiles. Commit messages follow the pattern
`type(scope): message`, e.g. `feat(indicator): add Langux panel button`; mention the issue
number when applicable (e.g. `(Closes #3)`). Do not open PRs unless asked; push and close
the issue at the end.