use ashpd::desktop::global_shortcuts::{Activated, GlobalShortcuts, NewShortcut};
use ashpd::zbus::zvariant::OwnedValue;
use futures_util::StreamExt;
use gtk::{Application, glib};
use std::{cell::Cell, collections::HashMap};

use crate::translator_window;

const TOGGLE_SHORTCUT_ID: &str = "io.github.rafaself.Langux.toggle";
const TOGGLE_SHORTCUT_DESCRIPTION: &str = "Show or hide Langux";
// XDG shortcut syntax uses XKB modifier names; LOGO is the Super key.
const PREFERRED_TRIGGER: &str = "LOGO+t";

thread_local! {
    static REGISTRATION_STARTED: Cell<bool> = const { Cell::new(false) };
}

/// Register the desktop-independent global activation shortcut.
///
/// Portal support and permission are optional. A failure leaves the normal
/// application and its `--toggle` activation command available.
pub fn register_toggle(app: &Application) {
    if REGISTRATION_STARTED.with(|started| started.replace(true)) {
        return;
    }

    let app = app.clone();
    glib::MainContext::default().spawn_local(async move {
        if let Err(error) = register_with_portal(&app).await {
            eprintln!(
                "Langux: XDG GlobalShortcuts is unavailable ({error}); use `langux --toggle` or configure a manual desktop shortcut."
            );
        }
    });
}

async fn register_with_portal(app: &Application) -> Result<(), String> {
    let portal = GlobalShortcuts::new()
        .await
        .map_err(|error| error.to_string())?;
    let session = portal
        .create_session()
        .await
        .map_err(|error| error.to_string())?;

    // Subscribe before binding so an activation cannot arrive between the
    // portal's successful response and signal subscription.
    let mut activated = portal
        .receive_activated()
        .await
        .map_err(|error| error.to_string())?;
    let request = portal
        .bind_shortcuts(
            &session,
            &[
                NewShortcut::new(TOGGLE_SHORTCUT_ID, TOGGLE_SHORTCUT_DESCRIPTION)
                    .preferred_trigger(PREFERRED_TRIGGER),
            ],
            None,
        )
        .await
        .map_err(|error| error.to_string())?;
    let binding = request.response().map_err(|error| error.to_string())?;

    if !binding
        .shortcuts()
        .iter()
        .any(|shortcut| shortcut.id() == TOGGLE_SHORTCUT_ID)
    {
        return Err("the portal did not bind the Langux toggle action".to_owned());
    }

    while let Some(event) = activated.next().await {
        if is_toggle_action(event.shortcut_id()) {
            translator_window::toggle_with_startup_id(app, activation_token(&event));
        }
    }

    Err("the portal activation signal stream ended".to_owned())
}

fn is_toggle_action(shortcut_id: &str) -> bool {
    shortcut_id == TOGGLE_SHORTCUT_ID
}

fn activation_token(event: &Activated) -> Option<&str> {
    activation_token_from_options(event.options())
}

fn activation_token_from_options(options: &HashMap<String, OwnedValue>) -> Option<&str> {
    options
        .get("activation_token")
        .and_then(|value| <&str>::try_from(value).ok())
        .filter(|token| !token.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        PREFERRED_TRIGGER, TOGGLE_SHORTCUT_ID, activation_token_from_options, is_toggle_action,
    };
    use ashpd::zbus::zvariant::{OwnedValue, Str};
    use std::collections::HashMap;

    #[test]
    fn identifies_only_the_langux_toggle_action() {
        assert!(is_toggle_action(TOGGLE_SHORTCUT_ID));
        assert!(!is_toggle_action("toggle"));
        assert!(!is_toggle_action("io.github.rafaself.Langux.show"));
    }

    #[test]
    fn preferred_shortcut_uses_the_xdg_logo_modifier() {
        assert_eq!(PREFERRED_TRIGGER, "LOGO+t");
    }

    #[test]
    fn extracts_the_portal_activation_token_when_present() {
        let mut options = HashMap::new();
        options.insert(
            "activation_token".to_owned(),
            OwnedValue::from(Str::from("wayland-token")),
        );

        assert_eq!(
            activation_token_from_options(&options),
            Some("wayland-token")
        );
    }

    #[test]
    fn ignores_missing_or_malformed_activation_tokens() {
        let mut options = HashMap::new();
        assert_eq!(activation_token_from_options(&options), None);

        options.insert("activation_token".to_owned(), OwnedValue::from(true));
        assert_eq!(activation_token_from_options(&options), None);

        options.insert(
            "activation_token".to_owned(),
            OwnedValue::from(Str::from("")),
        );
        assert_eq!(activation_token_from_options(&options), None);
    }
}
