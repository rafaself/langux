use std::cell::Cell;
use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Label, Orientation, PasswordEntry};
use langux_core::{SecretCredential, SecretStore, SecretStoreError};
use langux_secret_service::SecretServiceStore;

#[derive(Clone, Copy)]
enum CredentialAction {
    Save,
    Replace,
    Remove,
}

#[derive(Clone)]
struct CredentialControls {
    key_entry: PasswordEntry,
    status: Label,
    save_button: Button,
    replace_button: Button,
    remove_button: Button,
    available: Rc<Cell<bool>>,
    configured: Rc<Cell<bool>>,
}

impl CredentialControls {
    fn set_state(&self, available: bool, configured: bool, busy: bool) {
        self.available.set(available);
        self.configured.set(configured);
        self.save_button
            .set_sensitive(available && !configured && !busy);
        self.replace_button
            .set_sensitive(available && configured && !busy);
        self.remove_button
            .set_sensitive(available && configured && !busy);
        self.key_entry.set_sensitive(!busy);
    }
}

pub fn build() -> GtkBox {
    let section = GtkBox::new(Orientation::Vertical, 10);

    let explanation = Label::new(Some(
        "Configure a Google Cloud Translation API key. Langux stores it in your Linux Secret Service and never shows a saved key.",
    ));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    section.append(&explanation);

    let key_entry = PasswordEntry::new();
    key_entry.set_show_peek_icon(false);
    key_entry.set_placeholder_text(Some("Paste API key"));
    key_entry.set_activates_default(true);
    section.append(&key_entry);

    let save_button = Button::with_label("Save");
    let replace_button = Button::with_label("Replace");
    let remove_button = Button::with_label("Remove");
    let actions = GtkBox::new(Orientation::Horizontal, 8);
    actions.append(&save_button);
    actions.append(&replace_button);
    actions.append(&remove_button);
    section.append(&actions);

    let status = Label::new(Some("Checking secure storage…"));
    status.set_xalign(0.0);
    status.set_wrap(true);
    section.append(&status);

    let controls = CredentialControls {
        key_entry,
        status,
        save_button: save_button.clone(),
        replace_button: replace_button.clone(),
        remove_button: remove_button.clone(),
        available: Rc::new(Cell::new(false)),
        configured: Rc::new(Cell::new(false)),
    };

    let controls_for_save = controls.clone();
    save_button.connect_clicked(move |_| {
        run_action(&controls_for_save, CredentialAction::Save);
    });
    let controls_for_replace = controls.clone();
    replace_button.connect_clicked(move |_| {
        run_action(&controls_for_replace, CredentialAction::Replace);
    });
    let controls_for_remove = controls.clone();
    remove_button.connect_clicked(move |_| {
        run_action(&controls_for_remove, CredentialAction::Remove);
    });

    controls.set_state(false, false, true);
    let worker = gio::spawn_blocking(|| {
        let store = SecretServiceStore::new()?;
        store.exists()
    });
    glib::MainContext::default().spawn_local(async move {
        match worker.await.unwrap_or(Err(SecretStoreError::Failed)) {
            Ok(configured) => {
                controls.set_state(true, configured, false);
                controls.status.set_text(if configured {
                    "A Google API key is configured. The saved key is hidden."
                } else {
                    "No Google API key is configured."
                });
            }
            Err(error) => {
                controls.set_state(false, false, false);
                controls
                    .status
                    .set_text(&format!("Secure storage is unavailable: {error}"));
            }
        }
    });

    section
}

fn run_action(controls: &CredentialControls, action: CredentialAction) {
    let credential = match action {
        CredentialAction::Save | CredentialAction::Replace => {
            let value = controls.key_entry.text().to_string();
            controls.key_entry.set_text("");
            if value.trim().is_empty() {
                controls.status.set_text("Enter an API key first.");
                return;
            }
            Some(value)
        }
        CredentialAction::Remove => None,
    };

    controls.set_state(controls.available.get(), controls.configured.get(), true);
    controls.status.set_text(match action {
        CredentialAction::Save => "Saving API key to secure storage…",
        CredentialAction::Replace => "Replacing API key in secure storage…",
        CredentialAction::Remove => "Removing API key from secure storage…",
    });

    let controls = controls.clone();
    let worker = gio::spawn_blocking(move || {
        let store = SecretServiceStore::new()?;
        let credential = credential.map(SecretCredential::new);
        let result = match action {
            CredentialAction::Save => store.save(
                credential
                    .as_ref()
                    .expect("save action requires an entered credential"),
            ),
            CredentialAction::Replace => store.replace(
                credential
                    .as_ref()
                    .expect("replace action requires an entered credential"),
            ),
            CredentialAction::Remove => store.remove(),
        };
        let configured = store.exists().ok();
        Ok::<_, SecretStoreError>((result, configured))
    });

    glib::MainContext::default().spawn_local(async move {
        match worker.await.unwrap_or(Err(SecretStoreError::Failed)) {
            Ok((Ok(()), configured)) => {
                let configured = configured.unwrap_or(!matches!(action, CredentialAction::Remove));
                controls.set_state(true, configured, false);
                controls.status.set_text(match action {
                    CredentialAction::Save => "API key saved in secure storage.",
                    CredentialAction::Replace => "API key replaced in secure storage.",
                    CredentialAction::Remove => "API key removed from secure storage.",
                });
            }
            Ok((Err(error), Some(configured))) => {
                controls.set_state(true, configured, false);
                controls
                    .status
                    .set_text(&format!("Could not update the API key: {error}"));
            }
            Ok((Err(error), None)) | Err(error) => {
                controls.set_state(false, false, false);
                controls
                    .status
                    .set_text(&format!("Secure storage is unavailable: {error}"));
            }
        }
    });
}
