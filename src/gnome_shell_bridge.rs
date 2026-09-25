//! Narrow D-Bus bridge between the GNOME Shell presentation adapter and the
//! resident Rust application. Translation and credentials remain in Rust.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;
use gtk::glib::variant::ToVariant;

use crate::application_actions;
use crate::translation_flow::TranslationSnapshot;
use crate::{status_notifier, translator_window};

const INTERFACE_NAME: &str = "io.github.rafaself.Langux.GnomeShellAdapter";
const OBJECT_PATH: &str = "/io/github/rafaself/Langux/GnomeShellAdapter";
const SHELL_NAME: &str = "org.gnome.Shell";
const INTERFACE_XML: &str = r#"
<node>
  <interface name="io.github.rafaself.Langux.GnomeShellAdapter">
    <method name="GetState">
      <arg name="state" type="(ssssssbss)" direction="out"/>
    </method>
    <method name="GetLanguages">
      <arg name="languages" type="a(ss)" direction="out"/>
    </method>
    <method name="SetInput">
      <arg name="text" type="s" direction="in"/>
    </method>
    <method name="SetLanguagePair">
      <arg name="source" type="s" direction="in"/>
      <arg name="target" type="s" direction="in"/>
    </method>
    <method name="Translate"/>
    <method name="Clear"/>
    <method name="Copy">
      <arg name="copied" type="b" direction="out"/>
    </method>
    <method name="OpenSettings"/>
    <method name="SetAdapterEnabled">
      <arg name="enabled" type="b" direction="in"/>
    </method>
    <method name="Quit"/>
  </interface>
</node>
"#;

thread_local! {
    static SERVICE: RefCell<Option<GnomeShellBridge>> = const { RefCell::new(None) };
    static SHELL_ADAPTER_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

struct GnomeShellBridge {
    connection: gio::DBusConnection,
    object_registration: Option<gio::RegistrationId>,
    shell_owner_changed: Option<gio::SignalSubscription>,
}

impl GnomeShellBridge {
    fn start(
        connection: gio::DBusConnection,
        app: &gtk::Application,
        adapter_enabled: bool,
    ) -> Result<Self, glib::Error> {
        let shell_owner = Rc::new(RefCell::new(None));
        let interface = gio::DBusNodeInfo::for_xml(INTERFACE_XML)?
            .lookup_interface(INTERFACE_NAME)
            .expect("the GNOME bridge XML declares its D-Bus interface");
        let object_registration =
            connection
                .register_object(OBJECT_PATH, &interface)
                .method_call({
                    let app_weak = app.downgrade();
                    let shell_owner = Rc::clone(&shell_owner);
                    move |_connection, sender, _path, _interface, method, parameters, invocation| {
                        if !authorized_shell_caller(shell_owner.borrow().as_deref(), sender) {
                            invocation.return_dbus_error(
                            "org.freedesktop.DBus.Error.AccessDenied",
                            "Only the active GNOME Shell process may use the Langux popup bridge",
                        );
                            return;
                        }

                        let Some(app) = app_weak.upgrade() else {
                            invocation.return_dbus_error(
                                "org.freedesktop.DBus.Error.Failed",
                                "Langux is shutting down",
                            );
                            return;
                        };

                        match method {
                            "GetState" => match translator_window::snapshot() {
                                Some(state) => invocation.return_value(Some(
                                    &glib::Variant::tuple_from_iter([state_variant(&state)]),
                                )),
                                None => invocation.return_dbus_error(
                                    "org.freedesktop.DBus.Error.Failed",
                                    "The translator is not ready",
                                ),
                            },
                            "GetLanguages" => {
                                invocation.return_value(Some(&glib::Variant::tuple_from_iter([
                                    languages_variant(),
                                ])))
                            }
                            "SetInput" => {
                                let text = parameters.child_get::<String>(0);
                                if text.chars().count() > 4096 {
                                    invocation.return_dbus_error(
                                        "org.freedesktop.DBus.Error.InvalidArgs",
                                        "Input text exceeds the 4096 character limit",
                                    );
                                } else {
                                    translator_window::set_input_text(&text);
                                    invocation.return_value(None);
                                }
                            }
                            "SetLanguagePair" => {
                                let source = parameters.child_get::<String>(0);
                                let target = parameters.child_get::<String>(1);
                                if translator_window::set_language_pair(&source, &target) {
                                    invocation.return_value(None);
                                } else {
                                    invocation.return_dbus_error(
                                        "org.freedesktop.DBus.Error.InvalidArgs",
                                        "The requested language pair is unsupported",
                                    );
                                }
                            }
                            "Translate" => {
                                translator_window::translate_now();
                                invocation.return_value(None);
                            }
                            "Clear" => {
                                translator_window::clear();
                                invocation.return_value(None);
                            }
                            "Copy" => {
                                invocation.return_value(Some(
                                    &(translator_window::copy_result(),).to_variant(),
                                ));
                            }
                            "OpenSettings" => {
                                translator_window::open_settings(&app);
                                invocation.return_value(None);
                            }
                            "SetAdapterEnabled" => {
                                let enabled = parameters.child_get::<bool>(0);
                                set_adapter_enabled(&app, enabled);
                                invocation.return_value(None);
                            }
                            "Quit" => {
                                application_actions::quit(&app);
                                invocation.return_value(None);
                            }
                            _ => invocation.return_dbus_error(
                                "org.freedesktop.DBus.Error.UnknownMethod",
                                "Unknown Langux GNOME Shell bridge method",
                            ),
                        }
                    }
                })
                .build()?;

        let shell_owner_changed = connection.subscribe_to_signal(
            Some("org.freedesktop.DBus"),
            Some("org.freedesktop.DBus"),
            Some("NameOwnerChanged"),
            Some("/org/freedesktop/DBus"),
            Some(SHELL_NAME),
            gio::DBusSignalFlags::NONE,
            {
                let shell_owner = Rc::clone(&shell_owner);
                move |signal| {
                    let new_owner = signal.parameters.child_get::<String>(2);
                    *shell_owner.borrow_mut() = (!new_owner.is_empty()).then_some(new_owner);
                }
            },
        );

        // Subscribe before querying so a Shell restart cannot happen between
        // the initial owner check and installation of the change handler.
        *shell_owner.borrow_mut() = current_name_owner(&connection, SHELL_NAME);

        SHELL_ADAPTER_ACTIVE.with(|active| active.set(adapter_enabled));
        if adapter_enabled {
            status_notifier::shutdown();
        } else {
            status_notifier::install(app);
        }
        Ok(Self {
            connection,
            object_registration: Some(object_registration),
            shell_owner_changed: Some(shell_owner_changed),
        })
    }
}

impl Drop for GnomeShellBridge {
    fn drop(&mut self) {
        self.shell_owner_changed.take();
        if let Some(registration) = self.object_registration.take() {
            let _ = self.connection.unregister_object(registration);
        }
    }
}

pub fn install(app: &gtk::Application, adapter_enabled: bool) {
    let Some(connection) = app.dbus_connection() else {
        SHELL_ADAPTER_ACTIVE.with(|active| active.set(false));
        if !adapter_enabled {
            status_notifier::install(app);
        }
        return;
    };

    match GnomeShellBridge::start(connection, app, adapter_enabled) {
        Ok(service) => SERVICE.with(|slot| *slot.borrow_mut() = Some(service)),
        Err(error) => {
            SHELL_ADAPTER_ACTIVE.with(|active| active.set(false));
            if !adapter_enabled {
                status_notifier::install(app);
            }
            glib::g_warning!("langux", "could not export the GNOME Shell bridge: {error}");
        }
    }
}

pub fn set_adapter_enabled(app: &gtk::Application, enabled: bool) {
    let changed = SHELL_ADAPTER_ACTIVE.with(|active| {
        let previous = active.replace(enabled);
        previous != enabled
    });
    if !changed {
        return;
    }
    if enabled {
        status_notifier::shutdown();
    } else {
        status_notifier::install(app);
    }
}

pub fn shutdown() {
    SERVICE.with(|slot| drop(slot.borrow_mut().take()));
    SHELL_ADAPTER_ACTIVE.with(|active| active.set(false));
    status_notifier::shutdown();
}

fn current_name_owner(connection: &gio::DBusConnection, name: &str) -> Option<String> {
    connection
        .call_sync(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "GetNameOwner",
            Some(&(name,).to_variant()),
            Some(glib::VariantTy::new("(s)").expect("string reply signature")),
            gio::DBusCallFlags::NONE,
            1000,
            None::<&gio::Cancellable>,
        )
        .ok()
        .map(|reply| reply.child_get::<String>(0))
}

fn authorized_shell_caller(owner: Option<&str>, sender: Option<&str>) -> bool {
    matches!((owner, sender), (Some(owner), Some(sender)) if owner == sender)
}

fn state_variant(state: &TranslationSnapshot) -> glib::Variant {
    glib::Variant::tuple_from_iter([
        state.source_language.to_variant(),
        state.target_language.to_variant(),
        state.input_text.to_variant(),
        state.translated_text.to_variant(),
        state.status.to_variant(),
        state.phase.to_variant(),
        state.is_error.to_variant(),
        state.mode.to_variant(),
        state.detected_source_language.to_variant(),
    ])
}

fn languages_variant() -> glib::Variant {
    let mut languages = vec![(String::from("auto"), String::from("Auto detect"))];
    languages.extend(
        langux_core::supported_languages()
            .iter()
            .map(|language| (language.code().to_owned(), language.name().to_owned())),
    );
    languages.to_variant()
}

#[cfg(test)]
mod tests {
    use super::{authorized_shell_caller, languages_variant, state_variant};
    use crate::translation_flow::TranslationSnapshot;

    #[test]
    fn popup_state_has_a_stable_private_snapshot_shape() {
        let snapshot = TranslationSnapshot {
            source_language: String::from("pt"),
            target_language: String::from("en"),
            input_text: String::from("olá"),
            translated_text: String::from("hello"),
            status: String::from("Translation complete."),
            phase: "success",
            is_error: false,
            mode: "live",
            detected_source_language: String::from("pt"),
        };
        let variant = state_variant(&snapshot);

        assert_eq!(variant.type_().as_str(), "(ssssssbss)");
        assert_eq!(
            variant.get::<(
                String,
                String,
                String,
                String,
                String,
                String,
                bool,
                String,
                String
            )>(),
            Some((
                String::from("pt"),
                String::from("en"),
                String::from("olá"),
                String::from("hello"),
                String::from("Translation complete."),
                String::from("success"),
                false,
                String::from("live"),
                String::from("pt"),
            ))
        );
    }

    #[test]
    fn popup_language_catalog_includes_auto_source_and_supported_targets() {
        let languages = languages_variant().get::<Vec<(String, String)>>().unwrap();

        assert_eq!(
            languages.len(),
            langux_core::supported_languages().len() + 1
        );
        assert_eq!(
            languages[0],
            (String::from("auto"), String::from("Auto detect"))
        );
        assert!(languages.iter().any(|(code, _)| code == "pt"));
    }

    #[test]
    fn popup_bridge_accepts_only_the_current_shell_unique_name() {
        assert!(authorized_shell_caller(Some(":1.42"), Some(":1.42")));
        assert!(!authorized_shell_caller(Some(":1.42"), Some(":1.43")));
        assert!(!authorized_shell_caller(Some(":1.42"), None));
        assert!(!authorized_shell_caller(None, Some(":1.42")));
        assert!(!authorized_shell_caller(
            Some(":1.42"),
            Some("org.gnome.Shell")
        ));
    }
}
