# Langux

<p align="left">
  <img src="data/icon-readme.svg" width="64" height="64" alt="Langux icon">
</p>

Langux is a keyboard-first, local-first translator for GNOME Shell: open a popup,
type or paste text, translate it with Google Cloud Translation Basic v2, and copy
the result. Translation requests go directly from your machine to Google; Langux
has no backend, account system, telemetry, or persistent translation history.

## Features

- Open or toggle the translator with a configurable shortcut (`Super+T` by default).
- Translate while typing after one second of inactivity, or use explicit Enter mode.
- Use `Shift+Enter` for a new line; `Enter` and `Ctrl+Enter` translate in manual mode.
- Detect the source language automatically, choose source and target languages, and swap them.
- Copy translated text only through an explicit **Copy** action.
- Optionally reuse successful translations with a bounded in-memory cache, disabled by default.
- Store the Google API key in GNOME Keyring through libsecret, never in GSettings.
- Check manually for stable releases from the Preferences window; updates are never automatic.

## Compatibility and requirements

- GNOME Shell 49 on GNU/Linux, under X11 or Wayland.
- GJS with modern ES modules, plus the GNOME 49 system libraries `libsoup3` and `libsecret`.
- `gnome-extensions`, `curl`, and a SHA-256 tool (`sha256sum` or `shasum`) for release installation.
- A Google Cloud project with billing enabled and the Cloud Translation API enabled.

Other GNOME Shell versions may work, but only the version listed in
[`metadata.json`](metadata.json) is tested and supported by this project.

## Install a released version

Download the installer, inspect it if desired, and run it with Bash:

```sh
curl -fsSL https://raw.githubusercontent.com/rafaself/langux/main/scripts/install.sh \
  -o /tmp/langux-install.sh
less /tmp/langux-install.sh
bash /tmp/langux-install.sh
```

The installer resolves the latest GitHub Release, downloads the extension and its
SHA-256 checksum, verifies the archive before installation, and installs only for
the current user. It does not use `sudo`.

Enable Langux after installation:

```sh
gnome-extensions enable langux@rafaself.github.io
```

Restart the session if necessary (log out and back in, or press `Alt+F2` and type
`r` on X11). Open Preferences with:

```sh
gnome-extensions prefs langux@rafaself.github.io
```

## Install from a checkout

Use this path to test local changes:

```sh
git clone https://github.com/rafaself/langux.git
cd langux
npm ci
npm run check
scripts/dev-install.sh
gnome-extensions enable langux@rafaself.github.io
```

`scripts/dev-install.sh` packages the checked-out extension and installs it for the
current user. The development dependencies are not included in the extension
archive.

## Configure Google Cloud Translation

1. Create or select a Google Cloud project and enable billing.
2. Enable the **Cloud Translation API**.
3. Create an API key and restrict it to the Cloud Translation API. Add appropriate
   application restrictions and project quotas or budget alerts where possible.
4. Open Langux Preferences, select **Google Cloud → Configure**, and paste the key.
   Use **Replace** to change it or **Remove** to delete it.

The key is stored in GNOME Keyring and is never written to Langux settings, files,
logs, URLs, or the repository. Langux sends it to Google only in the
`X-Goog-Api-Key` HTTPS request header.

## Preferences and behavior

The defaults are source language `auto`, target language `en`, live translation
enabled, and translation caching disabled.

- Live mode sends non-blank text after it has remained unchanged for one second.
- Manual mode sends text only after `Enter` or `Ctrl+Enter`; `Shift+Enter` inserts a newline.
- The cache is session-only and can hold 0–1000 successful translations. Setting it to
  zero disables it; disabling or clearing the cache removes existing entries.
- The cache is cleared when the extension is disabled and is never written to disk.

## Privacy and data flow

- Translation text is sent directly from the local machine to Google Cloud Translation
  over HTTPS when live or manual translation is triggered.
- Input, output, API keys, and update responses are not written to disk or logs.
- Translated text is written to the system clipboard only when **Copy** is explicitly used.
- The optional cache stays in memory for the current Shell session and is disabled by default.
- Manual update checks contact only the fixed GitHub Releases API and request release
  metadata. Langux does not download, install, or reload updates by itself.

See [`SECURITY.md`](SECURITY.md) for the threat model and vulnerability reporting
instructions.

## Uninstall

```sh
gnome-extensions uninstall langux@rafaself.github.io
```

Removing the extension does not remove the API key from GNOME Keyring. Delete it
from Langux Preferences before uninstalling, or remove it later with `seahorse`.

## Development and checks

Install the pinned development tools with `npm ci`, then run the aggregate check:

```sh
npm run check             # syntax, tests, Biome, schema, and GNOME runtime probe
npm run format:check      # check formatting without changing files
npm run format            # intentionally format the scoped JavaScript files
scripts/package.sh        # build dist/langux.zip and its checksum
```

The pure-module tests can also be run independently:

```sh
npm test
```

For live Shell logs:

```sh
journalctl -f -o cat /usr/bin/gnome-shell
journalctl -f | grep -iE "langux|error|critical"
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the contribution workflow.

## License

Langux is licensed under [GPL-3.0-or-later](LICENSE).
