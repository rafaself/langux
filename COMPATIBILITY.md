# Desktop compatibility validation

This record documents the available validation for issue [#29](https://github.com/rafaself/langux/issues/29).

The results below describe the pre-tray behavior at the recorded revision.
They do not validate the resident, hidden startup and action dispatch added in
issue [#34](https://github.com/rafaself/langux/issues/34).

## Artifact and host

- Validation date: 2026-09-24.
- Source revision: `9ded6c00880124b679dc6bf04180e9e5be3c1959`.
- Artifact: a clean build of `flatpak/io.github.rafaself.Langux.yml`, installed and launched with the Flatpak Builder application.
- Installed Flatpak app commit: `29a1f8a04ab6a0f3aa3373874414ee7aa2becfcc3b34153f812826a5a65a0c5f`.
- Runtime: `org.gnome.Platform/x86_64/51`.
- Tested host: Fedora Linux 43, GNOME Shell 49.10, Wayland.
- The manifest has no desktop-specific build flags. Its sandbox permissions are network, IPC, Wayland with fallback X11, and the Secret Service bus name; it sets `GSK_RENDERER=cairo` and grants no host filesystem or device access.

The Flatpak artifact was tested only on the Fedora/GNOME host. It was not transferred to or run on Omarchy/Hyprland.

## Fedora and GNOME results

| Workflow | Result | Evidence and limits |
| --- | --- | --- |
| Clean Flatpak build and install | Pass | Built from the source revision above with `--force-clean`; Flatpak app commit recorded above. |
| App activation | Partial | A cold `flatpak run io.github.rafaself.Langux --toggle` launched the app. Two further `--toggle` calls routed to the running instance, which remained a single Flatpak app process. Window visibility and input focus were not inspected. |
| Settings persistence | Pass | Changed `target-language` to `fr` through the app's packaged GSettings schema, read it from a fresh Flatpak process, then restored the previous `en` value. |
| Secret Service availability | Partial | The sandbox could introspect `org.freedesktop.secrets`. Credential save/retrieve/remove was not exercised. No Langux Google API key was available in the session. |
| Global Shortcuts portal availability | Partial | The desktop exposed `org.freedesktop.portal.GlobalShortcuts`. Creating/approving a binding and pressing it were not tested. |
| Shutdown | Partial | `flatpak kill` stopped the app process. This was a forced stop, not a window-close test; it emitted a GDK frame-timing warning during termination. Graceful close behavior remains unverified. |
| Translation | Not tested | No API key was available, and no request was sent to Google. Network permission is present in the artifact, but live translation was not verified. |
| Clipboard, focus, and keyboard workflow | Not tested | Native window controls were unavailable in this execution, so copy/paste, focus, and keyboard interactions could not be exercised. |

The available CUA runtime exposed browser controls only; native window inventory and interaction methods were unavailable. This prevented direct inspection of the GTK window and its controls.

## Omarchy and Hyprland

Validation on Omarchy + Hyprland + Wayland is **deferred at the user's direction**. This host has no Hyprland session or Omarchy installation. The deferred run is not a compatibility pass, and the same-artifact acceptance criterion has therefore not been verified across both official environments.

## Automated checks

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --locked`, run in the GNOME 51 SDK: passed. Results were 18 app tests, 45 `langux-core` unit tests, 5 `langux-core` secret-store tests, 4 `langux-core` translation-provider tests, and 4 `langux-secret-service` tests.
- The Flatpak release build compiled successfully in the GNOME 51 SDK.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: no result. The installed SDK's `cargo-clippy` launcher could not select a default Rust toolchain. Clippy findings are therefore unverified.

GTK interaction, live Google translation, and Omarchy/Hyprland validation remain unverified; this record does not claim they passed.

## Tray-first Flatpak packaging (#37)

This validation covers the SNI and filtered session-bus policy added in
`0df8c3419385bbf7eb67662efcd502f3286f5f84`. It does not replace the earlier
translator-window checks above.

### Artifact and host

- Validation date: 2026-09-25.
- Artifact: `dist/langux-1.0.0-x86_64.flatpak`.
- Flatpak app commit: `cd92b139c4e39d6e83d6a6d90fa1a9581180f253b42f1505074cb6b3e7788625` (`stable`, user installation).
- Artifact SHA-256: `54c5ef975c4423e8e8e8e298e03b51a949d35a99f7cac69386db7e28b20ec104`.
- Runtime: `org.gnome.Platform/x86_64/51`.
- Tested host: Fedora Linux 43 Workstation, GNOME Shell 49.10, Wayland, Flatpak 1.16.6.

### Results

| Workflow | Result | Evidence and limits |
| --- | --- | --- |
| Rust validation | Pass | `scripts/check-rust.sh` passed in the GNOME 51 SDK using Rust 1.85.1: formatting, Clippy, all 83 workspace tests, and release build. |
| Bundle build and install | Pass | `scripts/build-flatpak.sh` completed and verified the SHA-256 checksum; the bundle installed as the `stable` user ref. |
| Session-bus policy | Pass | The installed policy grants `talk` only to `org.freedesktop.secrets` and `org.kde.StatusNotifierWatcher`. The SNI item name is in Langux's default app-owned namespace. No full session-bus socket or external own-name wildcard is granted. |
| Normal launch and SNI registration | Pass | A normal `flatpak run` stayed resident; the live GNOME watcher listed `io.github.rafaself.Langux.StatusNotifierItem_2_1` and reported a host registered. The window was not visually inspected. |
| Tray Quit and cleanup | Pass | Invoking dbusmenu item ID 2 (`Quit`) exited the app; `flatpak ps` showed no Langux process and the watcher removed the item. |
| Missing tray host | Partial | The app and docs report that it stays hidden and can be opened explicitly with `--toggle`; unit coverage verifies normal startup requests no window action and SNI shutdown/watcher loss. A packaged launch in a hostless graphical session was not verified. |
| Start on login | Not supported | Langux has no autostart setting or entry and does not request the XDG Background portal. |

The packaged workflow was exercised only on this Fedora/GNOME host. Hyprland
validation remains deferred at the user's direction.

### GNOME tray host setup (#38)

GNOME Shell needs a registered StatusNotifier host to display Langux's SNI tray
icon. Install and enable the official [AppIndicator and KStatusNotifierItem
Support extension](https://extensions.gnome.org/extension/615/appindicator-support/)
with a release marked active for the installed GNOME Shell version. The
following session-bus query reports whether a host is registered:

```sh
gdbus call --session --dest org.kde.StatusNotifierWatcher \
  --object-path /StatusNotifierWatcher \
  --method org.freedesktop.DBus.Properties.Get \
  org.kde.StatusNotifierWatcher IsStatusNotifierHostRegistered
```

A result containing `<true>` indicates a registered host. Without a watcher or
registered host, Langux logs a warning and keeps the translator hidden; run
`langux --toggle` (or `flatpak run io.github.rafaself.Langux --toggle`) to open
it. This is setup guidance, not an end-to-end compatibility claim for a
specific GNOME Shell or extension release. The partial issue #39 run below
records one tested host and workflow.

## Tray-first Fedora GNOME follow-up (#39)

This is a partial run of the tray-first acceptance workflow. It was stopped
when the active GNOME session locked; the remaining desktop checks require an
unlocked graphical session.

### Artifact and host

- Validation date: 2026-09-25.
- Source checkout: `232e3f8650def9a8a711b6917c1f239a2c0b9297` (`develop`, #38).
- Artifact: the existing `dist/langux-1.0.0-x86_64.flatpak` from #37; it was
  not rebuilt during this run.
- Artifact SHA-256: `54c5ef975c4423e8e8e8e298e03b51a949d35a99f7cac69386db7e28b20ec104`.
- Installed Flatpak app commit: `cd92b139c4e39d6e83d6a6d90fa1a9581180f253b42f1505074cb6b3e7788625`
  (`stable`, user installation).
- Runtime: `org.gnome.Platform/x86_64/51`.
- Tested host: Fedora Linux 43 Workstation, GNOME Shell 49.10, Wayland,
  Flatpak 1.16.6.
- Tray host: `gnome-shell-extension-appindicator` RPM `61-1.fc43`; the
  AppIndicator/KStatusNotifierItem extension was active at the start.

### Results

| Workflow | Result | Evidence and limits |
| --- | --- | --- |
| Normal launch and hidden window | Partial | `flatpak run --branch=stable io.github.rafaself.Langux` initially left one stable app instance running with no translator frame in the AT-SPI tree. The SNI watcher initially reported a registered host and listed Langux's item. About a minute later, the app log warned that no watcher was running. The cause and resident behavior after host loss were not established. |
| SNI activation and Show/Hide | Pass | Calling the item's SNI `Activate` method made the Langux frame visible in AT-SPI. Invoking dbusmenu item ID 1 (`Show / Hide Translator`) hid it; a subsequent `Activate` showed it again. |
| Tray item visibility | Partial | The watcher listed the Langux item, and the GNOME Shell accessibility tree exposed a `Langux` button with `visible=true` and `showing=false`. GNOME Shell denied both window introspection and screenshot requests, so the icon was not visually confirmed. |
| Close-to-hide | Not tested | The session locked before the window-close action could be checked. |
| Menu Quit and cleanup | Not tested | By the time the Quit event was attempted, the app's item bus name no longer had an owner, so the action could not be invoked. |
| One-instance behavior | Partial | A second normal `flatpak run` returned 0. An immediate `flatpak ps` snapshot showed two app instances; a later snapshot showed only the original stable instance. No sustained duplicate was confirmed, but the transient second instance was not explained. |
| Translation and copy | Not tested | No translation request was made and the API key was not inspected. Clipboard behavior remains unverified. |
| Default global shortcut | Not tested at runtime | This run did not capture portal calls during startup or inspect registered shortcut bindings. |

During the run, GNOME's ScreenSaver portal reported the session locked and the
AppIndicator extension became inactive along with other GNOME extensions. The
session was not unlocked or restarted. As a result, the full Fedora/GNOME
acceptance workflow remains incomplete and issue #39 stays open. Hyprland
validation remains deferred at the user's direction.

## GNOME Shell popup adapter (#41)

The implementation adds a new Shell-side presentation adapter under
`gnome-shell-adapter/`. It declares GNOME Shell 49, launches the hidden Rust
application through a desktop action, and uses the app's D-Bus namespace for
language/input state and translation actions. The Rust app authorizes bridge
calls only from the current `org.gnome.Shell` D-Bus owner. No additional
Flatpak bus permission or production Rust dependency is part of the adapter.

This implementation record does not claim a Fedora/GNOME graphical acceptance
pass. Issue #39 remains open for the follow-up host validation, and issue #40
remains deferred. Hyprland/Waybar support and validation are not claimed.

### Fedora GNOME popup validation attempt after #41

This resumed run built and installed the current Flatpak and GNOME Shell popup
adapter on the Fedora host. It stopped before popup interaction because the
running GNOME Shell session did not recognize the newly installed adapter.
The session was not restarted or logged out.

#### Artifact and host

- Validation date: 2026-09-25.
- Source checkout: `883d147` (`develop`, #41).
- Flatpak bundle: `dist/langux-1.0.0-x86_64.flatpak`, built from this checkout;
  SHA-256 `cd4efd47848d73510f990a9c4b87ceb1c8388ed0d9d8c37c1a6913d058ea49db`.
- Installed user Flatpak commit:
  `b464ead59999eb7880577a540b4c6503a7a2a92565cb2e5cc8db19ceaf69cb5e`
  (`stable`, x86_64); runtime `org.gnome.Platform/x86_64/51`.
- Tested host: Fedora Linux 43 Workstation, GNOME Shell 49.10, Wayland,
  Flatpak 1.16.6.
- SNI host: AppIndicator and KStatusNotifierItem Support, RPM
  `gnome-shell-extension-appindicator-61-1.fc43`; extension state was active,
  and `StatusNotifierWatcher.IsStatusNotifierHostRegistered` returned true.
- The session was unlocked at preflight (`org.gnome.ScreenSaver.GetActive`
  returned false).
- Adapter bundle SHA-256:
  `cec60368ef6d334c946e0bbb63631c51059750f7a2f7293b9059720030f71c3b`.

#### Build and setup results

| Step | Result | Evidence and limits |
| --- | --- | --- |
| Rust checks | Pass | `RUSTUP_TOOLCHAIN=1.85.1 scripts/check-rust.sh`: formatting, Clippy, 88 workspace tests, and release build passed. |
| Flatpak build and installation | Pass | `dbus-run-session -- scripts/build-flatpak.sh` completed and verified the bundle checksum; the user `stable` Flatpak was updated to the commit above. |
| Adapter packaging and file installation | Pass | `scripts/install-gnome-shell-adapter.sh` packaged and copied the adapter into `~/.local/share/gnome-shell/extensions/langux-shell@rafaself.github.io/`; its files and metadata were present. |
| Avoiding duplicate Langux entries | Partial | Disabled the historical `langux@rafaself.github.io` extension as required by the installer; its files remain installed. The AppIndicator host stayed enabled. |
| Adapter recognition and activation | Blocked | `gnome-extensions info langux-shell@rafaself.github.io` reported that the extension does not exist. `gnome-extensions enable langux-shell@rafaself.github.io` returned exit status 2 with the same message. The installer notes that GNOME Shell may require a logout/login to load a newly installed extension; no session restart or logout was attempted. |

#### Tray workflow results

| Workflow | Result | Evidence and limits |
| --- | --- | --- |
| Normal resident launch, one tray icon, hidden popup | Not tested | The app was not launched after the adapter activation attempt failed. |
| Reference-matched popup placement and visual appearance | Blocked | The adapter was not recognized by the live Shell session, so it could not be opened. No screenshot was captured in this run. |
| Popup dismiss/close, resident process, and Quit cleanup | Not tested | These checks require the popup adapter to load and were not attempted. |
| Repeated launch and one-instance behavior | Not tested | The app was not launched. |
| Translation and clipboard copy | Not tested | No key was inspected, added, or exposed; the UI was unavailable. |
| Default global shortcut | Not verified at runtime | No app or adapter startup occurred to observe portal activity or bindings. |

Issue #39 remains open because the live GNOME Shell session did not recognize
the new adapter, leaving the tray popup and process lifecycle unverified. The
same Flatpak revision and adapter can be checked again after a normal GNOME
session reload. Translation and copy also remain unverified. Hyprland/Waybar
validation remains deferred under #40.
