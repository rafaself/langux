# Security Policy

Langux is a local-first GNOME Shell extension. Its threat surface is intentionally
small: it stores one credential (a Google Cloud API key) and makes HTTPS requests to
Google Cloud Translation when the user explicitly asks for a translation.

## Secret handling

- The Google Cloud API key is stored in **GNOME Keyring** via libsecret. It is never
  stored in GSettings, in extension files, or in any other plaintext location.
- Keys must never be committed to the repository, embedded in code, or logged.
- The key is sent to Google Cloud only, over HTTPS, in the `X-Goog-Api-Key` request
  header — never in URLs or query strings.
- Langux never logs or persists source or translated text, and keeps no translation
  history.
- Langux has no backend and collects no telemetry. Translation requests go directly
  from your machine to `translation.googleapis.com`.

## Reporting a vulnerability

Please report security issues privately rather than in a public issue:

1. Go to https://github.com/rafaself/Langux/security/advisories and create a
   private security advisory describing the issue.
2. Include the affected version, a minimal reproduction, and the impact you observed.
3. If private reporting is unavailable on the repository, open a regular issue with
   `[SECURITY]` in the title and minimal reproduction details (never paste API keys
   or translation content).

Maintainers aim to acknowledge reports within 7 days and to ship a fix and/or
guidance as soon as a fix is verified.

## Trust model

- Code: the repository, including the installer scripts, is fully auditable and
  installed per-user; nothing is fetched or executed at runtime beyond the packaged
  extension files.
- Installer: `scripts/install.sh` verifies the SHA-256 of the downloaded archive
  against a checksum published on the same GitHub Release before installing.
- Network: HTTPS only, against Google Cloud Translation, only on explicit user
  action.