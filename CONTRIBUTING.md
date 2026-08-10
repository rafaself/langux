# Contributing

Thanks for contributing to Langux. Keep it simple and keep it in MVP scope.

## MVP-scope rule

Langux is a focused v0.1: open → translate → copy. Features outside that flow
(persistent history, favorites, multiple providers, backends, accounts, telemetry)
stay out of the MVP. Live translation is bounded to the user-controlled debounce
workflow and is not persistent. See the epic (issue #1) and the issue checklist
before starting work. If in doubt, ask in the issue first.

## Workflow

```text
fork/clone
  ↓
create a branch
  ↓
install/test locally
  ↓
open a pull request
```

1. Fork the repository and clone your fork.
2. Create a branch: `git checkout -b feat/my-change`.
3. Install the pinned developer tools: `npm ci`.
4. Make changes following `AGENTS.md` (modules stay small and separated; no
   Node/npm runtime dependencies; no Shell/GTK cross-imports in pure modules).
5. Test locally:

   ```sh
   npm run check
   scripts/dev-install.sh      # per-user install from your checkout
   ```

   For a full headless verification (enable/disable/re-enable without a display),
   see the headless instructions in `AGENTS.md`.
6. Open a pull request against `main`. Reference the issue you are solving
   (e.g. `Closes #3`).

## Reading shell logs

```bash
journalctl -f -o cat /usr/bin/gnome-shell          # live shell log
journalctl -f | grep -iE "langux|error|critical"  # Langux lines plus errors
```

Remember: Langux deliberately never logs the API key or translation text.

## License

Contributions are licensed under GPL-3.0-or-later (see `LICENSE`).
