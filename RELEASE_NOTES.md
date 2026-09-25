# Historical release notes: Langux v0.1.1 GNOME Shell extension

This release freezes the last GNOME Shell extension implementation before the
greenfield Langux 1.0 rewrite. Its complete source is preserved in Git at the
[`v0.1.1` tag](https://github.com/rafaself/langux/tree/v0.1.1); the earlier
[`v0.1.0` release](https://github.com/rafaself/langux/releases/tag/v0.1.0)
remains available. These notes apply only to the old GNOME Shell extension and
do not describe how to install or develop the standalone Rust application.

The rewrite policy is documented in [`REWRITE.md`](REWRITE.md) and follows
[epic #9](https://github.com/rafaself/langux/issues/9). Future work under that
epic targets the standalone application and does not carry forward GJS or GNOME
Shell implementation details.

## Install the archived extension

```sh
curl -fsSL https://raw.githubusercontent.com/rafaself/langux/v0.1.1/scripts/install.sh | bash
```

Enable and restart your session:

```sh
gnome-extensions enable langux@rafaself.github.io
```

Then configure your Google Cloud API key in the Langux settings window.

## Known limitations

- No clipboard-triggered translation
- No translation history
- No persistent translation cache
- No multiple translation providers
