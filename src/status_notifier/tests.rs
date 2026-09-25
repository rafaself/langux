use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::gio;
use gtk::glib;
use gtk::glib::variant::ToVariant;

use crate::application_actions::Action;

use super::{
    ITEM_INTERFACE, ITEM_PATH, MENU_PATH, StatusNotifierService, WATCHER_INTERFACE, WATCHER_NAME,
    WATCHER_PATH, item_service_name_for_pid,
};

const WATCHER_XML: &str = r#"
<node>
  <interface name="org.kde.StatusNotifierWatcher">
    <method name="RegisterStatusNotifierItem"><arg name="service" type="s" direction="in"/></method>
    <property name="IsStatusNotifierHostRegistered" type="b" access="read"/>
    <signal name="StatusNotifierHostRegistered"/>
    <signal name="StatusNotifierHostUnregistered"/>
  </interface>
</node>
"#;

#[test]
fn status_notifier_registers_dispatches_menu_and_tears_down() {
    let bus = gio::TestDBus::new(gio::TestDBusFlags::NONE);
    bus.up();
    let address = bus.bus_address().expect("test bus address").to_string();
    let watcher_connection = connect_to_test_bus(&address);
    let item_connection = connect_to_test_bus(&address);
    let client_connection = connect_to_test_bus(&address);

    let item_name = item_service_name_for_pid(424242);
    let registered_items = Rc::new(RefCell::new(Vec::<String>::new()));
    let host_available = Rc::new(Cell::new(false));
    let watched_item_name = item_name.clone();
    let _item_name_watch = watcher_connection.subscribe_to_signal(
        Some("org.freedesktop.DBus"),
        Some("org.freedesktop.DBus"),
        Some("NameOwnerChanged"),
        Some("/org/freedesktop/DBus"),
        Some(item_name.as_str()),
        gio::DBusSignalFlags::NONE,
        {
            let registered_items = registered_items.clone();
            move |signal| {
                let name = signal.parameters.child_get::<String>(0);
                let old_owner = signal.parameters.child_get::<String>(1);
                let new_owner = signal.parameters.child_get::<String>(2);
                if name == watched_item_name && !old_owner.is_empty() && new_owner.is_empty() {
                    registered_items
                        .borrow_mut()
                        .retain(|registered| registered != &name);
                }
            }
        },
    );
    let watcher_interface = gio::DBusNodeInfo::for_xml(WATCHER_XML)
        .expect("valid mock watcher XML")
        .lookup_interface(WATCHER_INTERFACE)
        .expect("mock watcher interface");
    let watcher_registration = watcher_connection
        .register_object(WATCHER_PATH, &watcher_interface)
        .method_call({
            let registered_items = registered_items.clone();
            move |_connection, _sender, _path, _interface, method, parameters, invocation| {
                if method == "RegisterStatusNotifierItem" {
                    registered_items
                        .borrow_mut()
                        .push(parameters.child_get::<String>(0));
                    invocation.return_value(None);
                } else {
                    invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.UnknownMethod",
                        "Unknown mock watcher method",
                    );
                }
            }
        })
        .property({
            let host_available = host_available.clone();
            move |_connection, _sender, _path, _interface, property| {
                assert_eq!(property, "IsStatusNotifierHostRegistered");
                host_available.get().to_variant()
            }
        })
        .build()
        .expect("export mock watcher");

    let watcher_owner = gio::bus_own_name_on_connection(
        &watcher_connection,
        WATCHER_NAME,
        gio::BusNameOwnerFlags::NONE,
        |_, _| {},
        |_, _| panic!("mock watcher bus name was lost unexpectedly"),
    );
    spin_until(|| name_has_owner(&client_connection, WATCHER_NAME));

    let actions = Rc::new(RefCell::new(Vec::<Action>::new()));
    let dispatcher = Rc::new({
        let actions = actions.clone();
        move |action| actions.borrow_mut().push(action)
    });
    let service =
        StatusNotifierService::start(item_connection.clone(), item_name.clone(), dispatcher)
            .expect("start SNI service");

    spin_until(|| registered_items.borrow().len() == 1 && service.registered_with_watcher.get());
    assert_eq!(registered_items.borrow().as_slice(), &[item_name.clone()]);
    assert!(service.watcher_available.get());
    assert!(service.registered_with_watcher.get());

    let item_id = get_property(
        &client_connection,
        &item_name,
        ITEM_PATH,
        ITEM_INTERFACE,
        "Id",
    );
    assert_eq!(item_id, "io.github.rafaself.Langux");
    assert_eq!(
        get_property(
            &client_connection,
            &item_name,
            ITEM_PATH,
            ITEM_INTERFACE,
            "Title",
        ),
        "Langux"
    );
    let icon_name = get_property(
        &client_connection,
        &item_name,
        ITEM_PATH,
        ITEM_INTERFACE,
        "IconName",
    );
    assert_eq!(icon_name, "io.github.rafaself.Langux");
    let tooltip = call(
        &client_connection,
        Some(item_name.as_str()),
        ITEM_PATH,
        "org.freedesktop.DBus.Properties",
        "Get",
        Some(&(ITEM_INTERFACE, "ToolTip").to_variant()),
        Some(glib::VariantTy::new("(v)").expect("tooltip property type")),
    );
    let tooltip = tooltip.child_value(0).get::<glib::Variant>().unwrap();
    assert_eq!(tooltip.type_().as_str(), "(sa(iiay)ss)");
    assert_eq!(tooltip.child_get::<String>(2), "Langux");
    assert_eq!(tooltip.child_get::<String>(3), "Quick translator");

    call(
        &client_connection,
        Some(item_name.as_str()),
        ITEM_PATH,
        ITEM_INTERFACE,
        "Activate",
        Some(&(0_i32, 0_i32).to_variant()),
        None,
    );
    assert_eq!(actions.borrow().as_slice(), &[Action::Toggle]);

    let layout = call(
        &client_connection,
        Some(item_name.as_str()),
        MENU_PATH,
        "com.canonical.dbusmenu",
        "GetLayout",
        Some(&(0_i32, -1_i32, Vec::<String>::new()).to_variant()),
        Some(glib::VariantTy::new("(u(ia{sv}av))").expect("dbusmenu layout reply type")),
    );
    let root = layout.child_value(1);
    assert_eq!(root.type_().as_str(), "(ia{sv}av)");
    let children = root.child_value(2).get::<Vec<glib::Variant>>().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(
        layout_property(&client_connection, &item_name, 1, "label"),
        "Show / Hide Translator"
    );
    assert_eq!(
        layout_property(&client_connection, &item_name, 2, "label"),
        "Quit"
    );

    call(
        &client_connection,
        Some(item_name.as_str()),
        MENU_PATH,
        "com.canonical.dbusmenu",
        "Event",
        Some(&(1_i32, "clicked", 0_u32.to_variant(), 0_u32).to_variant()),
        None,
    );
    call(
        &client_connection,
        Some(item_name.as_str()),
        MENU_PATH,
        "com.canonical.dbusmenu",
        "Event",
        Some(&(2_i32, "clicked", 0_u32.to_variant(), 0_u32).to_variant()),
        None,
    );
    assert_eq!(
        actions.borrow().as_slice(),
        &[Action::Toggle, Action::Toggle, Action::Quit]
    );

    host_available.set(true);
    watcher_connection
        .emit_signal(
            None,
            WATCHER_PATH,
            WATCHER_INTERFACE,
            "StatusNotifierHostRegistered",
            None,
        )
        .expect("emit host registered");
    spin_until(|| service.host_available.get() == Some(true));

    host_available.set(false);
    watcher_connection
        .emit_signal(
            None,
            WATCHER_PATH,
            WATCHER_INTERFACE,
            "StatusNotifierHostUnregistered",
            None,
        )
        .expect("emit host unregistered");
    spin_until(|| service.host_available.get() == Some(false));

    gio::bus_unown_name(watcher_owner);
    spin_until(|| !service.watcher_available.get());
    assert!(!service.registered_with_watcher.get());

    let watcher_owner = gio::bus_own_name_on_connection(
        &watcher_connection,
        WATCHER_NAME,
        gio::BusNameOwnerFlags::NONE,
        |_, _| {},
        |_, _| panic!("mock watcher bus name was lost unexpectedly"),
    );
    spin_until(|| {
        service.watcher_available.get()
            && service.registered_with_watcher.get()
            && registered_items.borrow().len() == 2
    });

    drop(service);
    spin_until(|| !name_has_owner(&client_connection, item_name.as_str()));
    assert!(!name_has_owner(&client_connection, item_name.as_str()));
    spin_until(|| registered_items.borrow().is_empty());

    gio::bus_unown_name(watcher_owner);
    let _ = watcher_connection.unregister_object(watcher_registration);
    let _ = watcher_connection.close_sync(None::<&gio::Cancellable>);
    let _ = item_connection.close_sync(None::<&gio::Cancellable>);
    let _ = client_connection.close_sync(None::<&gio::Cancellable>);
    gio::TestDBus::unset();
    bus.down();
}

#[test]
fn item_bus_name_uses_the_flatpak_default_owned_namespace() {
    assert_eq!(
        item_service_name_for_pid(424242),
        "io.github.rafaself.Langux.StatusNotifierItem_424242_1"
    );
}

fn connect_to_test_bus(address: &str) -> gio::DBusConnection {
    gio::DBusConnection::for_address_sync(
        address,
        gio::DBusConnectionFlags::AUTHENTICATION_CLIENT
            .union(gio::DBusConnectionFlags::MESSAGE_BUS_CONNECTION),
        None,
        None::<&gio::Cancellable>,
    )
    .expect("connect to isolated test D-Bus")
}

fn call(
    connection: &gio::DBusConnection,
    destination: Option<&str>,
    path: &str,
    interface: &str,
    method: &str,
    parameters: Option<&glib::Variant>,
    reply_type: Option<&glib::VariantTy>,
) -> glib::Variant {
    let (sender, receiver) = std::sync::mpsc::channel();
    connection.call(
        destination,
        path,
        interface,
        method,
        parameters,
        reply_type,
        gio::DBusCallFlags::NONE,
        3000,
        None::<&gio::Cancellable>,
        move |result| {
            let _ = sender.send(result);
        },
    );
    let context = glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match receiver.try_recv() {
            Ok(result) => return result.expect("D-Bus call succeeds"),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                panic!("D-Bus call callback disconnected")
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for D-Bus reply"
        );
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

fn get_property(
    connection: &gio::DBusConnection,
    destination: &str,
    path: &str,
    interface: &str,
    property: &str,
) -> String {
    let reply = call(
        connection,
        Some(destination),
        path,
        "org.freedesktop.DBus.Properties",
        "Get",
        Some(&(interface, property).to_variant()),
        Some(glib::VariantTy::new("(v)").expect("property reply type")),
    );
    reply
        .child_value(0)
        .get::<glib::Variant>()
        .unwrap()
        .get::<String>()
        .unwrap()
}

fn layout_property(
    connection: &gio::DBusConnection,
    destination: &str,
    id: i32,
    property: &str,
) -> String {
    let reply = call(
        connection,
        Some(destination),
        MENU_PATH,
        "com.canonical.dbusmenu",
        "GetProperty",
        Some(&(id, property).to_variant()),
        Some(glib::VariantTy::new("(v)").expect("menu property reply type")),
    );
    reply
        .child_value(0)
        .get::<glib::Variant>()
        .unwrap()
        .get::<String>()
        .unwrap()
}

fn name_has_owner(connection: &gio::DBusConnection, name: &str) -> bool {
    connection
        .call_sync(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "NameHasOwner",
            Some(&(name,).to_variant()),
            Some(glib::VariantTy::new("(b)").expect("NameHasOwner reply type")),
            gio::DBusCallFlags::NONE,
            1000,
            None::<&gio::Cancellable>,
        )
        .expect("ask test bus about a name")
        .child_get::<bool>(0)
}

fn spin_until(mut condition: impl FnMut() -> bool) {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for D-Bus fixture"
        );
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
