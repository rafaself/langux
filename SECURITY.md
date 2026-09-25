# Security Policy

Langux is a local-first desktop translator. The active application is a
standalone Rust and GTK 4 app distributed primarily as a Flatpak. It has no
Langux backend, account system, telemetry, analytics, or persistent translation
history.

## Data and credentials

- The Google Cloud Translation API key is stored through the user's Linux
  Secret Service. Langux does not store it in GSettings or a plaintext app
  file, and the Settings interface never displays a saved key.
- During translation, Langux retrieves the key into memory and sends it to
  `translation.googleapis.com` over HTTPS in the `X-Goog-Api-Key` request
  header. The client requires HTTPS and does not forward the key to redirects.
- Translation text is sent directly to Google Cloud Translation when the user
  triggers a request. Live translation is enabled by default and can be
  disabled in Settings. Google's service and data policies apply to submitted
  text.
- Source text and results are held in memory while in use. The optional
  successful-translation cache is bounded, in-memory, and disabled by default.
  Langux does not write translation text or results to disk or logs.
- Source and target languages, translation mode, and cache preferences are
  stored locally with GSettings. There is no persistent translation history.
- Results reach the system clipboard only when the user activates **Copy**.
- The standalone app has no automatic update installer. Flatpak installation
  and updates are managed by the user or desktop software tools.

The Flatpak requests network access, Wayland with fallback X11, IPC, and access
to the desktop Secret Service D-Bus name. It does not request unrestricted host
filesystem or device access. GTK/GDK clipboard access uses the granted Wayland
or X11 display connection; XDG Desktop Portal calls use a separate portal API.
See [`flatpak/io.github.rafaself.Langux.yml`](flatpak/io.github.rafaself.Langux.yml)
for the current sandbox permissions.

## Protecting an API key

The API key can authorize Google Cloud usage and may incur charges. Restrict it
to the Cloud Translation API, configure project quotas and budget alerts, and
use the Secret Service implementation provided by your desktop. If Secret
Service is missing or locked, Langux reports that secure storage is
unavailable; it has no plaintext-file fallback.

Do not commit API keys, include them in issue reports, or paste them into
public chats. Langux maintainers will never need the key to reproduce a
translation bug.

## Reporting a vulnerability

Report security issues privately through
[GitHub Security Advisories](https://github.com/rafaself/langux/security/advisories).
Include the affected commit or release, a minimal reproduction, and the impact
you observed. Do not include API keys, translation text, or other private data.

If private reporting is unavailable, open a regular issue titled
`[SECURITY] ...` with minimal reproduction details and no secrets.
