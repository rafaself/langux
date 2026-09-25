//! Freedesktop StatusNotifierItem integration over the application's existing
//! GIO session-bus connection.

mod dbus_menu;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;

use crate::application_actions::{self, Action};

const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/Menu";
const APPLICATION_ID: &str = "io.github.rafaself.Langux";
const APP_NAME: &str = "Langux";
const APP_TOOLTIP: &str = "Quick translator";

const ITEM_XML: &str = r#"
<node>
  <interface name="org.kde.StatusNotifierItem">
    <property name="Category" type="s" access="read"/>
    <property name="Id" type="s" access="read"/>
    <property name="Title" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="WindowId" type="u" access="read"/>
    <property name="IconName" type="s" access="read"/>
    <property name="IconPixmap" type="a(iiay)" access="read"/>
    <property name="OverlayIconName" type="s" access="read"/>
    <property name="OverlayIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionIconName" type="s" access="read"/>
    <property name="AttentionIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionMovieName" type="s" access="read"/>
    <property name="ToolTip" type="(sa(iiay)ss)" access="read"/>
    <property name="ItemIsMenu" type="b" access="read"/>
    <property name="Menu" type="o" access="read"/>
    <method name="ContextMenu">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Activate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="SecondaryActivate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Scroll">
      <arg name="delta" type="i" direction="in"/>
      <arg name="orientation" type="s" direction="in"/>
    </method>
    <signal name="NewTitle"/>
    <signal name="NewIcon"/>
    <signal name="NewAttentionIcon"/>
    <signal name="NewOverlayIcon"/>
    <signal name="NewToolTip"/>
    <signal name="NewStatus"><arg name="status" type="s"/></signal>
  </interface>
</node>
"#;

thread_local! {
    static SERVICE: RefCell<Option<StatusNotifierService>> = const { RefCell::new(None) };
}

/// Register the tray item using the session-bus connection owned by GApplication.
pub fn install(app: &gtk::Application) {
    let Some(connection) = app.dbus_connection() else {
        warn_tray_unavailable("the application has no session-bus connection");
        return;
    };

    let app_weak = app.downgrade();
    let action: ActionDispatcher = Rc::new(move |action| {
        if let Some(app) = app_weak.upgrade() {
            application_actions::activate(&app, action);
        }
    });

    match StatusNotifierService::start(connection, item_service_name(), action) {
        Ok(service) => SERVICE.with(|slot| *slot.borrow_mut() = Some(service)),
        Err(error) => warn_tray_unavailable(&format!("could not export the tray item: {error}")),
    }
}

/// Release the exported objects and bus names before the application exits.
pub fn shutdown() {
    SERVICE.with(|slot| drop(slot.borrow_mut().take()));
}

type ActionDispatcher = Rc<dyn Fn(Action)>;

struct StatusNotifierService {
    connection: gio::DBusConnection,
    item_registration: Option<gio::RegistrationId>,
    menu_registration: Option<gio::RegistrationId>,
    item_name_owner: Option<gio::OwnerId>,
    watcher_watch: Option<Box<dyn FnOnce()>>,
    host_registered: Option<gio::SignalSubscription>,
    host_unregistered: Option<gio::SignalSubscription>,
    stopped: Rc<Cell<bool>>,
    item_name_owned: Rc<Cell<bool>>,
    watcher_available: Rc<Cell<bool>>,
    registered_with_watcher: Rc<Cell<bool>>,
    host_available: Rc<Cell<Option<bool>>>,
}

impl StatusNotifierService {
    fn start(
        connection: gio::DBusConnection,
        item_name: String,
        action: ActionDispatcher,
    ) -> Result<Self, glib::Error> {
        let item_interface = gio::DBusNodeInfo::for_xml(ITEM_XML)?
            .lookup_interface(ITEM_INTERFACE)
            .expect("the SNI XML defines its declared interface");
        let item_registration = connection
            .register_object(ITEM_PATH, &item_interface)
            .method_call({
                let action = action.clone();
                move |_connection, _sender, _path, _interface, method, _parameters, invocation| {
                    match method {
                        "Activate" | "SecondaryActivate" => action(Action::Toggle),
                        // The host reads the exported dbusmenu object when it
                        // handles this request; no replacement GTK menu is needed.
                        "ContextMenu" | "Scroll" => {}
                        _ => {
                            invocation.return_dbus_error(
                                "org.freedesktop.DBus.Error.UnknownMethod",
                                "Unknown StatusNotifierItem method",
                            );
                            return;
                        }
                    }
                    invocation.return_value(None);
                }
            })
            .property(|_connection, _sender, _path, _interface, property| item_property(property))
            .build()?;

        let menu_registration = match dbus_menu::register(&connection, action) {
            Ok(registration) => registration,
            Err(error) => {
                let _ = connection.unregister_object(item_registration);
                return Err(error);
            }
        };

        let stopped = Rc::new(Cell::new(false));
        let item_name_owned = Rc::new(Cell::new(false));
        let watcher_available = Rc::new(Cell::new(false));
        let registered_with_watcher = Rc::new(Cell::new(false));
        let host_available = Rc::new(Cell::new(None));

        let host_registered = connection.subscribe_to_signal(
            Some(WATCHER_NAME),
            Some(WATCHER_INTERFACE),
            Some("StatusNotifierHostRegistered"),
            Some(WATCHER_PATH),
            None,
            gio::DBusSignalFlags::NONE,
            {
                let connection = connection.clone();
                let stopped = stopped.clone();
                let registered_with_watcher = registered_with_watcher.clone();
                let host_available = host_available.clone();
                move |_| {
                    if !stopped.get() && registered_with_watcher.get() {
                        query_host_availability(
                            &connection,
                            stopped.clone(),
                            registered_with_watcher.clone(),
                            host_available.clone(),
                        );
                    }
                }
            },
        );
        let host_unregistered = connection.subscribe_to_signal(
            Some(WATCHER_NAME),
            Some(WATCHER_INTERFACE),
            Some("StatusNotifierHostUnregistered"),
            Some(WATCHER_PATH),
            None,
            gio::DBusSignalFlags::NONE,
            {
                let connection = connection.clone();
                let stopped = stopped.clone();
                let registered_with_watcher = registered_with_watcher.clone();
                let host_available = host_available.clone();
                move |_| {
                    if !stopped.get() && registered_with_watcher.get() {
                        query_host_availability(
                            &connection,
                            stopped.clone(),
                            registered_with_watcher.clone(),
                            host_available.clone(),
                        );
                    }
                }
            },
        );

        let item_owner = gio::bus_own_name_on_connection(
            &connection,
            &item_name,
            gio::BusNameOwnerFlags::NONE,
            {
                let connection = connection.clone();
                let item_name = item_name.clone();
                let stopped = stopped.clone();
                let item_name_owned = item_name_owned.clone();
                let watcher_available = watcher_available.clone();
                let registered_with_watcher = registered_with_watcher.clone();
                let host_available = host_available.clone();
                move |_, _| {
                    if stopped.get() {
                        return;
                    }
                    item_name_owned.set(true);
                    if watcher_available.get() {
                        register_with_watcher(
                            &connection,
                            &item_name,
                            stopped.clone(),
                            registered_with_watcher.clone(),
                            host_available.clone(),
                        );
                    }
                }
            },
            {
                let stopped = stopped.clone();
                let item_name_owned = item_name_owned.clone();
                move |_, name| {
                    item_name_owned.set(false);
                    if !stopped.get() {
                        warn_tray_unavailable(&format!(
                            "could not own the StatusNotifierItem bus name {name}"
                        ));
                    }
                }
            },
        );

        let watcher_watch_id = gio::bus_watch_name_on_connection(
            &connection,
            WATCHER_NAME,
            gio::BusNameWatcherFlags::NONE,
            {
                let connection = connection.clone();
                let item_name = item_name.clone();
                let stopped = stopped.clone();
                let item_name_owned = item_name_owned.clone();
                let watcher_available = watcher_available.clone();
                let registered_with_watcher = registered_with_watcher.clone();
                let host_available = host_available.clone();
                move |_, _, _| {
                    if stopped.get() {
                        return;
                    }
                    watcher_available.set(true);
                    if item_name_owned.get() {
                        register_with_watcher(
                            &connection,
                            &item_name,
                            stopped.clone(),
                            registered_with_watcher.clone(),
                            host_available.clone(),
                        );
                    }
                }
            },
            {
                let stopped = stopped.clone();
                let watcher_available = watcher_available.clone();
                let registered_with_watcher = registered_with_watcher.clone();
                let host_available = host_available.clone();
                move |_, _| {
                    watcher_available.set(false);
                    registered_with_watcher.set(false);
                    host_available.set(None);
                    if !stopped.get() {
                        warn_tray_unavailable(
                            "no StatusNotifierWatcher is running; the translator stays hidden. Enable a tray host or run `langux --toggle` to open it",
                        );
                    }
                }
            },
        );
        let watcher_watch = Box::new(move || gio::bus_unwatch_name(watcher_watch_id));

        Ok(Self {
            connection,
            item_registration: Some(item_registration),
            menu_registration: Some(menu_registration),
            item_name_owner: Some(item_owner),
            watcher_watch: Some(watcher_watch),
            host_registered: Some(host_registered),
            host_unregistered: Some(host_unregistered),
            stopped,
            item_name_owned,
            watcher_available,
            registered_with_watcher,
            host_available,
        })
    }

    fn stop(&mut self) {
        if self.stopped.replace(true) {
            return;
        }

        self.host_registered.take();
        self.host_unregistered.take();
        if let Some(unwatch) = self.watcher_watch.take() {
            unwatch();
        }
        if let Some(registration) = self.menu_registration.take() {
            let _ = self.connection.unregister_object(registration);
        }
        if let Some(registration) = self.item_registration.take() {
            let _ = self.connection.unregister_object(registration);
        }
        if let Some(owner) = self.item_name_owner.take() {
            gio::bus_unown_name(owner);
        }
        self.item_name_owned.set(false);
        self.watcher_available.set(false);
        self.registered_with_watcher.set(false);
        self.host_available.set(None);
    }
}

impl Drop for StatusNotifierService {
    fn drop(&mut self) {
        self.stop();
    }
}

fn item_service_name() -> String {
    format!("org.kde.StatusNotifierItem-{}-1", std::process::id())
}

fn item_property(name: &str) -> glib::Variant {
    match name {
        "Category" => "ApplicationStatus".to_variant(),
        "Id" => APPLICATION_ID.to_variant(),
        "Title" => APP_NAME.to_variant(),
        "Status" => "Active".to_variant(),
        "WindowId" => 0_u32.to_variant(),
        "IconName" => APPLICATION_ID.to_variant(),
        "IconPixmap" | "OverlayIconPixmap" | "AttentionIconPixmap" => {
            Vec::<(i32, i32, Vec<u8>)>::new().to_variant()
        }
        "OverlayIconName" | "AttentionIconName" | "AttentionMovieName" => "".to_variant(),
        "ToolTip" => (
            APPLICATION_ID,
            Vec::<(i32, i32, Vec<u8>)>::new(),
            APP_NAME,
            APP_TOOLTIP,
        )
            .to_variant(),
        "ItemIsMenu" => false.to_variant(),
        "Menu" => glib::variant::ObjectPath::try_from(MENU_PATH)
            .expect("the exported menu path is valid")
            .to_variant(),
        _ => unreachable!("only declared read-only SNI properties are requested"),
    }
}

fn register_with_watcher(
    connection: &gio::DBusConnection,
    item_name: &str,
    stopped: Rc<Cell<bool>>,
    registered_with_watcher: Rc<Cell<bool>>,
    host_available: Rc<Cell<Option<bool>>>,
) {
    if stopped.get() {
        return;
    }
    registered_with_watcher.set(false);
    host_available.set(None);
    connection.call(
        Some(WATCHER_NAME),
        WATCHER_PATH,
        WATCHER_INTERFACE,
        "RegisterStatusNotifierItem",
        Some(&(item_name,).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        3000,
        None::<&gio::Cancellable>,
        {
            let connection = connection.clone();
            move |result| match result {
                Ok(_) if !stopped.get() => {
                    registered_with_watcher.set(true);
                    query_host_availability(
                        &connection,
                        stopped,
                        registered_with_watcher,
                        host_available,
                    );
                }
                Ok(_) => {}
                Err(error) if !stopped.get() => warn_tray_unavailable(&format!(
                    "StatusNotifierWatcher registration failed: {error}"
                )),
                Err(_) => {}
            }
        },
    );
}

fn query_host_availability(
    connection: &gio::DBusConnection,
    stopped: Rc<Cell<bool>>,
    registered_with_watcher: Rc<Cell<bool>>,
    host_available: Rc<Cell<Option<bool>>>,
) {
    if stopped.get() || !registered_with_watcher.get() {
        return;
    }
    connection.call(
        Some(WATCHER_NAME),
        WATCHER_PATH,
        "org.freedesktop.DBus.Properties",
        "Get",
        Some(&(WATCHER_INTERFACE, "IsStatusNotifierHostRegistered").to_variant()),
        Some(glib::VariantTy::new("(v)").expect("valid D-Bus property reply type")),
        gio::DBusCallFlags::NONE,
        3000,
        None::<&gio::Cancellable>,
        move |result| {
            if stopped.get() || !registered_with_watcher.get() {
                return;
            }
            match result {
                Ok(reply) => {
                    let value = reply
                        .child_value(0)
                        .get::<glib::Variant>()
                        .and_then(|value| value.get::<bool>());
                    host_available.set(value);
                    if value == Some(false) {
                        warn_tray_unavailable(
                            "no StatusNotifierHost is registered; the translator stays hidden. Enable a tray host or run `langux --toggle` to open it",
                        );
                    }
                }
                Err(error) => {
                    host_available.set(None);
                    warn_tray_unavailable(&format!(
                        "could not verify that a tray host is registered: {error}"
                    ));
                }
            }
        },
    );
}

fn warn_tray_unavailable(message: &str) {
    glib::g_warning!("langux", "{message}");
}

#[cfg(test)]
mod tests;
