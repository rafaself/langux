use std::collections::HashMap;
use std::rc::Rc;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;

use crate::application_actions::Action;

use super::MENU_PATH;

const MENU_INTERFACE: &str = "com.canonical.dbusmenu";
const MENU_XML: &str = r#"
<node>
  <interface name="com.canonical.dbusmenu">
    <method name="GetLayout">
      <arg name="parentId" type="i" direction="in"/>
      <arg name="recursionDepth" type="i" direction="in"/>
      <arg name="propertyNames" type="as" direction="in"/>
      <arg name="revision" type="u" direction="out"/>
      <arg name="layout" type="(ia{sv}av)" direction="out"/>
    </method>
    <method name="GetGroupProperties">
      <arg name="ids" type="ai" direction="in"/>
      <arg name="propertyNames" type="as" direction="in"/>
      <arg name="properties" type="a(ia{sv})" direction="out"/>
    </method>
    <method name="GetProperty">
      <arg name="id" type="i" direction="in"/>
      <arg name="name" type="s" direction="in"/>
      <arg name="value" type="v" direction="out"/>
    </method>
    <method name="Event">
      <arg name="id" type="i" direction="in"/>
      <arg name="eventId" type="s" direction="in"/>
      <arg name="data" type="v" direction="in"/>
      <arg name="timestamp" type="u" direction="in"/>
    </method>
    <method name="EventGroup">
      <arg name="events" type="a(isvu)" direction="in"/>
      <arg name="errors" type="a(is)" direction="out"/>
    </method>
    <method name="AboutToShow">
      <arg name="id" type="i" direction="in"/>
      <arg name="needUpdate" type="b" direction="out"/>
    </method>
    <method name="AboutToShowGroup">
      <arg name="ids" type="ai" direction="in"/>
      <arg name="updates" type="ai" direction="out"/>
      <arg name="errors" type="ai" direction="out"/>
    </method>
    <property name="Version" type="u" access="read"/>
    <property name="TextDirection" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="IconThemePath" type="as" access="read"/>
  </interface>
</node>
"#;

const REVISION: u32 = 1;
const ROOT_ID: i32 = 0;
const TOGGLE_ID: i32 = 1;
const QUIT_ID: i32 = 2;

pub(super) fn register(
    connection: &gio::DBusConnection,
    action: Rc<dyn Fn(Action)>,
) -> Result<gio::RegistrationId, glib::Error> {
    let interface = gio::DBusNodeInfo::for_xml(MENU_XML)?
        .lookup_interface(MENU_INTERFACE)
        .expect("the menu XML defines its declared interface");

    connection
        .register_object(MENU_PATH, &interface)
        .method_call(
            move |_connection, _sender, _path, _interface, method, parameters, invocation| {
                match method {
                    "GetLayout" => {
                        let parent_id = parameters.child_get::<i32>(0);
                        let depth = parameters.child_get::<i32>(1);
                        let requested = parameters.child_get::<Vec<String>>(2);
                        let layout = layout(parent_id, depth, &requested);
                        invocation.return_value(Some(&glib::Variant::tuple_from_iter([
                            REVISION.to_variant(),
                            layout,
                        ])));
                    }
                    "GetGroupProperties" => {
                        let ids = parameters.child_get::<Vec<i32>>(0);
                        let requested = parameters.child_get::<Vec<String>>(1);
                        let properties = ids
                            .into_iter()
                            .filter_map(|id| {
                                item_properties(id, &requested).map(|props| (id, props))
                            })
                            .collect::<Vec<_>>();
                        invocation.return_value(Some(&properties.to_variant().tuple()));
                    }
                    "GetProperty" => {
                        let id = parameters.child_get::<i32>(0);
                        let name = parameters.child_get::<String>(1);
                        match item_property(id, &name) {
                            Some(value) => {
                                invocation.return_value(Some(&glib::Variant::tuple_from_iter([
                                    value.to_variant(),
                                ])))
                            }
                            None => invocation.return_dbus_error(
                                "com.canonical.dbusmenu.Error.UnknownProperty",
                                "The requested menu property does not exist",
                            ),
                        }
                    }
                    "Event" => {
                        let id = parameters.child_get::<i32>(0);
                        let event = parameters.child_get::<String>(1);
                        dispatch_event(id, &event, &action);
                        invocation.return_value(None);
                    }
                    "EventGroup" => {
                        let events =
                            parameters.child_get::<Vec<(i32, String, glib::Variant, u32)>>(0);
                        for (id, event, _data, _timestamp) in events {
                            dispatch_event(id, &event, &action);
                        }
                        let errors: Vec<(i32, String)> = Vec::new();
                        invocation.return_value(Some(&errors.to_variant().tuple()));
                    }
                    "AboutToShow" => {
                        invocation.return_value(Some(&(false,).to_variant()));
                    }
                    "AboutToShowGroup" => {
                        let updates: Vec<i32> = Vec::new();
                        let errors: Vec<i32> = Vec::new();
                        invocation.return_value(Some(&glib::Variant::tuple_from_iter([
                            updates.to_variant(),
                            errors.to_variant(),
                        ])));
                    }
                    _ => invocation.return_dbus_error(
                        "org.freedesktop.DBus.Error.UnknownMethod",
                        "Unknown dbusmenu method",
                    ),
                }
            },
        )
        .property(
            |_connection, _sender, _path, _interface, property| match property {
                "Version" => 3_u32.to_variant(),
                "TextDirection" => "ltr".to_variant(),
                "Status" => "normal".to_variant(),
                "IconThemePath" => Vec::<String>::new().to_variant(),
                _ => unreachable!("only declared dbusmenu properties are requested"),
            },
        )
        .build()
}

fn layout(parent_id: i32, recursion_depth: i32, requested: &[String]) -> glib::Variant {
    let (properties, children) = if parent_id == ROOT_ID && recursion_depth != 0 {
        let children = vec![
            item_layout(TOGGLE_ID, recursion_depth - 1, requested),
            item_layout(QUIT_ID, recursion_depth - 1, requested),
        ];
        (HashMap::new(), children)
    } else if let Some(properties) = item_properties(parent_id, requested) {
        (properties, Vec::new())
    } else {
        (HashMap::new(), Vec::new())
    };

    (parent_id, properties, children).to_variant()
}

fn item_layout(id: i32, recursion_depth: i32, requested: &[String]) -> glib::Variant {
    layout(id, recursion_depth, requested)
}

fn item_properties(id: i32, requested: &[String]) -> Option<HashMap<String, glib::Variant>> {
    let label = match id {
        TOGGLE_ID => "Show / Hide Translator",
        QUIT_ID => "Quit",
        _ => return None,
    };

    let mut properties = HashMap::new();
    if requested.is_empty() || requested.iter().any(|property| property == "label") {
        properties.insert("label".to_owned(), label.to_variant());
    }
    if requested.is_empty() || requested.iter().any(|property| property == "enabled") {
        properties.insert("enabled".to_owned(), true.to_variant());
    }
    if requested.is_empty() || requested.iter().any(|property| property == "visible") {
        properties.insert("visible".to_owned(), true.to_variant());
    }
    Some(properties)
}

fn item_property(id: i32, name: &str) -> Option<glib::Variant> {
    match (id, name) {
        (TOGGLE_ID, "label") => Some("Show / Hide Translator".to_variant()),
        (QUIT_ID, "label") => Some("Quit".to_variant()),
        (TOGGLE_ID | QUIT_ID, "enabled" | "visible") => Some(true.to_variant()),
        _ => None,
    }
}

fn dispatch_event(id: i32, event: &str, action: &Rc<dyn Fn(Action)>) {
    if event != "clicked" {
        return;
    }
    match id {
        TOGGLE_ID => action(Action::Toggle),
        QUIT_ID => action(Action::Quit),
        _ => {}
    }
}

trait VariantTupleExt {
    fn tuple(self) -> glib::Variant;
}

impl VariantTupleExt for glib::Variant {
    fn tuple(self) -> glib::Variant {
        glib::Variant::tuple_from_iter([self])
    }
}
