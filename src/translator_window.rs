use std::cell::RefCell;

use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, DropDown, Label, Orientation,
    ScrolledWindow, TextView, WrapMode,
};
use langux_core::preferred_language_pair;
use std::sync::Arc;

use crate::language_controls::LanguageControls;
use crate::secret_translation_provider::SecretTranslationProvider;
use crate::settings;
use crate::translation_flow;
use crate::translation_view::TranslationView;

thread_local! {
    static WINDOW_STATE: RefCell<Option<WindowState>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct WindowState {
    window: glib::WeakRef<ApplicationWindow>,
    input_view: glib::WeakRef<TextView>,
    source_dropdown: glib::WeakRef<DropDown>,
    target_dropdown: glib::WeakRef<DropDown>,
    flow: translation_flow::TranslationFlowHandle,
    settings: gtk::gio::Settings,
}

pub fn initialize(app: &Application) {
    if window_state().is_none() {
        build(app);
    }
}

fn build(app: &Application) {
    let settings = settings::open();
    let initial_pair = settings::language_pair(&settings);
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Langux")
        .default_width(500)
        .default_height(520)
        .resizable(true)
        .build();
    // Connect before translation_flow's close handler so a user close hides
    // the window without cancelling its in-progress translation state.
    window.connect_close_request(|window| hide_on_close_request(|| window.set_visible(false)));

    let content = GtkBox::new(Orientation::Vertical, 16);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let title_row = GtkBox::new(Orientation::Horizontal, 12);
    let title = Label::new(Some("Langux"));
    title.add_css_class("title-1");
    title.set_xalign(0.0);
    title.set_hexpand(true);

    let settings_button = Button::with_label("Settings");
    settings_button.set_tooltip_text(Some("Change translation defaults and API key settings."));
    title_row.append(&title);
    title_row.append(&settings_button);
    content.append(&title_row);

    let window_for_preferences = window.downgrade();
    let app_for_preferences = app.clone();
    let settings_for_preferences = settings.clone();
    settings_button.connect_clicked(move |_| {
        let Some(window) = window_for_preferences.upgrade() else {
            return;
        };
        crate::preferences::present(
            &app_for_preferences,
            &window,
            settings_for_preferences.clone(),
        );
    });

    let language_controls = LanguageControls::build(&initial_pair);
    content.append(&language_controls.row);

    let (input_area, input_view) = text_area(true);
    let input_label = Label::new(Some("_Input text"));
    input_label.set_use_underline(true);
    input_label.set_xalign(0.0);
    input_label.set_mnemonic_widget(Some(&input_view));
    input_label.set_tooltip_text(Some("Enter the text to translate."));

    let input_section = GtkBox::new(Orientation::Vertical, 8);
    input_section.set_vexpand(true);
    input_section.append(&input_label);
    input_section.append(&input_area);
    content.append(&input_section);

    let keyboard_help = Label::new(Some(concat!(
        "Live mode translates after a pause; Ctrl+Enter translates immediately. ",
        "Manual mode uses Enter or Ctrl+Enter. Shift+Enter adds a line. ",
        "Escape closes Langux; Alt+C copies the result."
    )));
    keyboard_help.set_xalign(0.0);
    keyboard_help.set_wrap(true);
    keyboard_help.add_css_class("dim-label");
    content.append(&keyboard_help);

    let translation_view = TranslationView::build();
    content.append(&translation_view.section);

    window.set_child(Some(&content));
    let flow = translation_flow::connect(
        &window,
        &input_view,
        &language_controls,
        initial_pair,
        settings.clone(),
        translation_view,
        Arc::new(SecretTranslationProvider),
    );
    WINDOW_STATE.with(|state| {
        *state.borrow_mut() = Some(WindowState {
            window: window.downgrade(),
            input_view: input_view.downgrade(),
            source_dropdown: language_controls.source_dropdown.downgrade(),
            target_dropdown: language_controls.target_dropdown.downgrade(),
            flow,
            settings: settings.clone(),
        });
    });
}

pub fn toggle(app: &Application) {
    if let Some(state) = window_state() {
        let (Some(window), Some(input_view)) = (state.window.upgrade(), state.input_view.upgrade())
        else {
            return;
        };
        toggle_visibility(
            window.is_visible(),
            || present_input(&window, &input_view),
            || window.set_visible(false),
        );
    } else {
        build(app);
        if let Some(state) = window_state() {
            if let (Some(window), Some(input_view)) =
                (state.window.upgrade(), state.input_view.upgrade())
            {
                present_input(&window, &input_view);
            }
        }
    }
}

pub fn set_input_text(text: &str) {
    let Some(state) = window_state() else {
        return;
    };
    let Some(input_view) = state.input_view.upgrade() else {
        return;
    };
    let buffer = input_view.buffer();
    let (start, end) = buffer.bounds();
    if buffer.text(&start, &end, true).as_str() != text {
        buffer.set_text(text);
    }
}

pub fn set_language_pair(source: &str, target: &str) -> bool {
    let Some(pair) = preferred_language_pair(source, target) else {
        return false;
    };
    let Some((source_index, target_index)) = crate::language_selection::language_indices(&pair)
    else {
        return false;
    };
    let Some(state) = window_state() else {
        return false;
    };
    let (Some(source_dropdown), Some(target_dropdown)) = (
        state.source_dropdown.upgrade(),
        state.target_dropdown.upgrade(),
    ) else {
        return false;
    };
    source_dropdown.set_selected(source_index);
    target_dropdown.set_selected(target_index);
    true
}

pub fn translate_now() {
    if let Some(state) = window_state() {
        state.flow.translate_now();
    }
}

pub fn clear() {
    set_input_text("");
}

pub fn copy_result() -> bool {
    window_state().is_some_and(|state| state.flow.copy_result())
}

pub fn snapshot() -> Option<translation_flow::TranslationSnapshot> {
    window_state()?.flow.snapshot()
}

pub fn open_settings(app: &Application) {
    let Some(state) = window_state() else {
        return;
    };
    let Some(window) = state.window.upgrade() else {
        return;
    };
    crate::preferences::present(app, &window, state.settings);
}

fn toggle_visibility(is_visible: bool, show: impl FnOnce(), hide: impl FnOnce()) {
    if is_visible {
        hide();
    } else {
        show();
    }
}

fn hide_on_close_request(hide: impl FnOnce()) -> glib::Propagation {
    hide();
    glib::Propagation::Stop
}

fn window_state() -> Option<WindowState> {
    WINDOW_STATE.with(|state| {
        let state = state.borrow();
        state.as_ref().cloned()
    })
}

fn present_input(window: &ApplicationWindow, input_view: &TextView) {
    gtk::prelude::GtkWindowExt::set_focus(window, Some(input_view));
    window.present();
    input_view.grab_focus();
}

fn text_area(editable: bool) -> (ScrolledWindow, TextView) {
    let text_view = TextView::new();
    text_view.set_wrap_mode(WrapMode::WordChar);
    text_view.set_editable(editable);
    text_view.set_cursor_visible(editable);
    text_view.set_top_margin(8);
    text_view.set_bottom_margin(8);
    text_view.set_left_margin(8);
    text_view.set_right_margin(8);

    let scrolled = ScrolledWindow::new();
    scrolled.set_child(Some(&text_view));
    scrolled.set_min_content_height(112);
    scrolled.set_hexpand(true);
    scrolled.set_vexpand(true);

    (scrolled, text_view)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use gtk::glib;

    use super::{hide_on_close_request, toggle_visibility};

    struct FakeWindow {
        visible: Cell<bool>,
        presentations: Cell<usize>,
        hides: Cell<usize>,
        destroyed: Cell<bool>,
        input_text: RefCell<String>,
    }

    #[test]
    fn repeated_activation_shows_and_hides_the_same_window_state() {
        let window = FakeWindow {
            visible: Cell::new(false),
            presentations: Cell::new(0),
            hides: Cell::new(0),
            destroyed: Cell::new(false),
            input_text: RefCell::new(String::from("translation state survives hiding")),
        };

        for _ in 0..4 {
            toggle_visibility(
                window.visible.get(),
                || {
                    window.presentations.set(window.presentations.get() + 1);
                    window.visible.set(true);
                },
                || {
                    window.hides.set(window.hides.get() + 1);
                    window.visible.set(false);
                },
            );
        }

        assert_eq!(window.presentations.get(), 2);
        assert_eq!(window.hides.get(), 2);
        assert!(!window.visible.get());
        assert!(!window.destroyed.get());
        assert_eq!(
            window.input_text.borrow().as_str(),
            "translation state survives hiding"
        );
    }

    #[test]
    fn close_request_hides_and_stops_window_destruction() {
        let window = FakeWindow {
            visible: Cell::new(true),
            presentations: Cell::new(0),
            hides: Cell::new(0),
            destroyed: Cell::new(false),
            input_text: RefCell::new(String::from("translation state survives hiding")),
        };

        let propagation = hide_on_close_request(|| window.visible.set(false));
        if propagation == glib::Propagation::Proceed {
            window.destroyed.set(true);
        }

        assert_eq!(propagation, glib::Propagation::Stop);
        assert!(!window.visible.get());
        assert!(!window.destroyed.get());
        assert_eq!(
            window.input_text.borrow().as_str(),
            "translation state survives hiding"
        );
    }
}
