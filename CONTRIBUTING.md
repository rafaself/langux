# Contributing

Thanks for contributing to Langux. The active product is the standalone Rust
and GTK 4 application described in [epic #9](https://github.com/rafaself/langux/issues/9).
The frozen GNOME Shell extension is preserved as historical source and a
behavioral reference; it is not the architecture or release path for the app.

## Before you start

- Check the related issue and keep the change within its scope. The rewrite
  issues are sequenced so each change remains independently reviewable.
- Follow the architecture and security guidance in [`AGENTS.md`](AGENTS.md)
  and [`REWRITE.md`](REWRITE.md).
- Ask before adding a production dependency unless it is highly consolidated,
  safe, and trustable.
- Never place API keys in GSettings, files, code, tests, or logs. Use the
  Secret Service interface for credential work.
- Keep translation text and API keys out of diagnostics. Do not add a Langux
  backend, accounts, telemetry, persistent translation history, or automatic
  update installation.

## Workflow

1. Fork or clone the repository and create a focused branch.
2. Install the prerequisites and follow the build steps in
   [`DEVELOPMENT.md`](DEVELOPMENT.md).
3. Make the smallest change that satisfies the issue and add or update tests
   for changed domain behavior.
4. Run the relevant Rust checks:

   ```sh
   scripts/check-rust.sh
   ```

   For a full local validation, this checks formatting, Clippy, workspace
   tests, and a release build. The script requires GTK 4 and the native
   development libraries listed in [`DEVELOPMENT.md`](DEVELOPMENT.md).
5. Review the final diff for scope, secret handling, and lifecycle risks.
6. Push your branch and open a pull request against `main`, describing the
   change and linking the issue.

## Architecture

The Rust workspace is the only active application implementation. GTK code
belongs in `src/`; reusable translation domain behavior belongs in
`crates/langux-core`; Linux Secret Service integration belongs in
`crates/langux-secret-service`. Keep provider logic separate from presentation
and keep desktop-specific integration behind narrow platform boundaries.

The JavaScript, GJS, GNOME Shell metadata, and extension packaging files remain
for the frozen v0.1.x release history. Do not extend that implementation as a
way to implement Langux 1.0 issues.

## License

Contributions are licensed under GPL-3.0-or-later (see [`LICENSE`](LICENSE)).
