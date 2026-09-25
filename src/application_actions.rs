use std::cell::RefCell;

use gtk::prelude::*;
use gtk::{Application, gio};

use crate::cli_args::Invocation;
use crate::translator_window;

thread_local! {
    static RESIDENT_HOLD: RefCell<Option<gio::ApplicationHoldGuard>> = const { RefCell::new(None) };
}

/// Actions available to desktop integrations such as the tray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Toggle,
    Quit,
}

impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::Quit => "quit",
        }
    }
}

/// Convert a parsed command-line invocation to the application action it
/// requests. A normal launch deliberately has no window action.
pub fn requested_action(invocation: Invocation, toggle_option: bool) -> Option<Action> {
    match invocation {
        Invocation::Toggle => Some(Action::Toggle),
        Invocation::Resident if toggle_option => Some(Action::Toggle),
        Invocation::Resident | Invocation::Help => None,
    }
}

/// Install the stable GAction interface used by command-line and tray entry
/// points. Integrations activate these actions without calling window code.
pub fn install(app: &Application) {
    let toggle = gio::SimpleAction::new(Action::Toggle.name(), None);
    let app_weak = app.downgrade();
    toggle.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            dispatch(&mut GtkActionHandler(&app), Action::Toggle);
        }
    });
    app.add_action(&toggle);

    let quit = gio::SimpleAction::new(Action::Quit.name(), None);
    let app_weak = app.downgrade();
    quit.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            dispatch(&mut GtkActionHandler(&app), Action::Quit);
        }
    });
    app.add_action(&quit);
}

/// Keep the application alive when it has no visible windows.
pub fn hold_resident(app: &Application) {
    RESIDENT_HOLD.with(|hold| {
        *hold.borrow_mut() = Some(app.hold());
    });
}

/// Release the resident hold and end the application event loop.
pub fn quit(app: &Application) {
    RESIDENT_HOLD.with(|hold| {
        drop(hold.borrow_mut().take());
    });
    app.quit();
}

/// Activate an application action by its public GAction name.
pub fn activate(app: &Application, action: Action) {
    app.activate_action(action.name(), None);
}

trait ActionHandler {
    fn toggle(&mut self);
    fn quit(&mut self);
}

fn dispatch(handler: &mut impl ActionHandler, action: Action) {
    match action {
        Action::Toggle => handler.toggle(),
        Action::Quit => handler.quit(),
    }
}

struct GtkActionHandler<'a>(&'a Application);

impl ActionHandler for GtkActionHandler<'_> {
    fn toggle(&mut self) {
        translator_window::toggle(self.0);
    }

    fn quit(&mut self) {
        quit(self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Action, ActionHandler, RESIDENT_HOLD, dispatch, hold_resident, install, quit,
        requested_action,
    };
    use crate::cli_args::Invocation;
    use gtk::gio::ApplicationFlags;
    use gtk::prelude::*;

    #[derive(Default)]
    struct FakeHandler {
        visible: bool,
        running: bool,
        toggles: usize,
    }

    impl ActionHandler for FakeHandler {
        fn toggle(&mut self) {
            self.visible = !self.visible;
            self.toggles += 1;
        }

        fn quit(&mut self) {
            self.running = false;
        }
    }

    #[test]
    fn cold_start_has_no_window_action() {
        assert_eq!(
            requested_action(Invocation::Resident, false),
            None,
            "a normal launch must stay resident without opening the window"
        );
    }

    #[test]
    fn repeated_toggle_requests_route_to_the_same_action() {
        let mut handler = FakeHandler {
            running: true,
            ..Default::default()
        };
        for invocation in [Invocation::Toggle, Invocation::Toggle] {
            let action = requested_action(invocation, false).expect("toggle action");
            dispatch(&mut handler, action);
        }

        assert_eq!(handler.toggles, 2);
        assert!(!handler.visible);
        assert!(handler.running);
    }

    #[test]
    fn command_line_toggle_uses_the_primary_action_route() {
        assert_eq!(
            requested_action(Invocation::Resident, true),
            Some(Action::Toggle)
        );
        assert_eq!(
            requested_action(Invocation::Toggle, false),
            Some(Action::Toggle)
        );
    }

    #[test]
    fn quit_dispatches_to_the_application_shutdown_action() {
        let mut handler = FakeHandler {
            running: true,
            ..Default::default()
        };
        dispatch(&mut handler, Action::Quit);
        assert!(!handler.running);
    }

    #[test]
    fn registered_actions_are_available_to_desktop_integrations() {
        let app = gtk::Application::builder()
            .application_id("io.github.rafaself.Langux.ActionTest")
            .flags(ApplicationFlags::NON_UNIQUE)
            .build();
        install(&app);

        assert!(app.lookup_action(Action::Toggle.name()).is_some());
        assert!(app.lookup_action(Action::Quit.name()).is_some());
    }

    #[test]
    fn resident_hold_is_released_by_quit() {
        let app = gtk::Application::builder()
            .application_id("io.github.rafaself.Langux.LifecycleTest")
            .flags(ApplicationFlags::NON_UNIQUE)
            .build();

        hold_resident(&app);
        assert!(RESIDENT_HOLD.with(|hold| hold.borrow().is_some()));
        quit(&app);
        assert!(RESIDENT_HOLD.with(|hold| hold.borrow().is_none()));
    }
}
