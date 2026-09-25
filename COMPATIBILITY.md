# Desktop compatibility validation

This record documents the available validation for issue [#29](https://github.com/rafaself/langux/issues/29).

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
