# Langux v0.1.0 — first MVP release

Langux is a local-first quick translator for GNOME Shell: press the shortcut, type
or paste text, translate with Google Cloud Translation Basic v2, copy the result.

- Supported/tested GNOME Shell version: **49**.
- Bring your own key: this release requires a **Google Cloud Translation Basic v2
  API key** (paid Google Cloud project, API enabled, key restricted to the
  Translation API as recommended).
- Local-first privacy model: no backend, no accounts, no telemetry, and no persistent
  translation history. Live translation is enabled by default after a one-second
  debounce, with a manual Enter/Ctrl+Enter mode available. Successful results are
  kept only in a bounded in-memory cache; the API key lives in GNOME Keyring.
- The manifest file `dist/langux.zip.sha256` is published next to the archive so
  installs can verify the download before installing.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/rafaself/langux/main/scripts/install.sh | bash
```

Enable and restart your session:

```sh
gnome-extensions enable langux@rafaself.github.io
```

Then configure your Google Cloud API key in the Langux settings window.

## Known limitations (v0.1.0)

- No clipboard-triggered translation
- No translation history
- No persistent translation cache
- No multiple translation providers
