# Langux

Langux is a local-first quick translator for GNOME Shell. Press a shortcut, type or
paste text, translate it with Google Cloud Translation (Basic v2), copy the result.
No backend, no accounts, no telemetry, no history.

## Supported / tested versions

- GNOME Shell 49
- GJS with ES modules (no legacy `imports` style)
- libsoup3 and libsecret (GNOME 49 system libraries)

Other GNOME releases may work but are not tested; `metadata.json` lists only the
version validated on.

## Requirements

- GNOME Shell 49 on a GNU/Linux session (X11 or Wayland)
- `gnome-extensions`, `curl`, and a `sha256sum`/`shasum` tool for the installer
- A Google Cloud Translation **Basic v2** API key (see configuration below)

## Install from a GitHub Release

```sh
curl -fsSL https://raw.githubusercontent.com/rafaself/langux/main/scripts/install.sh | bash
```

The installer downloads the latest `langux.zip` and its SHA-256 checksum, verifies
the checksum **before** installing, installs for the current user only (no `sudo`),
cleans up temporary files, and prints the next steps.

Then enable the extension and restart the session:

```sh
gnome-extensions enable langux@rafaself.github.io
```

Restart the session (log out and back in) or, on X11, press `Alt+F2` and type `r`.

## Local development installation

```bash
git clone https://github.com/rafaself/langux.git
cd langux
scripts/dev-install.sh
gnome-extensions enable langux@rafaself.github.io
```

`dev-install.sh` runs `scripts/package.sh`, installs the archive with
`gnome-extensions install --force` (current user only), and prints enable/restart
hints. It fails clearly if `gnome-extensions` is unavailable.

## Configure a Google Cloud API key

Google Cloud Translation Basic v2 requires a paid Google Cloud project (with a
billable billing account), an enabled Translation API, and an API key:

1. Create a Google Cloud project and enable billing.
2. Enable the **Cloud Translation API** for the project.
3. Generate an **API key** (Credentials → Create credentials → API key).
4. Open Langux settings (`gnome-extensions prefs langux@rafaself.github.io`
   or the gear button in the Langux popup).
5. Under *Google Cloud*, click **Configure** and paste the key (click **Replace** to
   update later, **Remove** to delete it).

Security recommendations:

- Restrict the key to the **Cloud Translation API** only (API restrictions).
- Optionally restrict by IP/domain and set project-level quotas, budgets, and billing
  alerts to cap costs.
- The key is stored in **GNOME Keyring** (libsecret); it is never shown again after
  configuring and is never stored in Langux's own settings.

## Privacy

Langux has no backend and does not collect telemetry or translation history. When
the user explicitly translates text, it is sent directly from the local machine to
Google Cloud Translation over HTTPS. Langux does not persist translation input or
output. Translated text is only written to the system clipboard when the user
explicitly clicks Copy. Nothing else is stored locally beyond the API key (GNOME
Keyring); source and translated text are never saved, logged, or stored.

## Uninstall

```bash
gnome-extensions uninstall langux@rafaself.github.io
```

(Optionally disable first.) The API key stays in GNOME Keyring; remove it from the
Langux prefs window (or `seahorse`) if you want it gone.

## Development / logging

```bash
# watch Shell logs in a live session
journalctl -f -o cat /usr/bin/gnome-shell
# or with a grep for Langux plus errors
journalctl -f | grep -iE "langux|gnome-shell.*(error|critical)"

# unit tests (pure modules, no npm dependencies)
node --test
```

Logging is minimal by design: translation input/output and the API key are never
logged.

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).