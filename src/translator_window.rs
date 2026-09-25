use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation, ScrolledWindow,
    TextView, WrapMode,
};
use std::sync::Arc;

use crate::language_controls::LanguageControls;
use crate::secret_translation_provider::SecretTranslationProvider;
use crate::settings;
use crate::translation_flow;
use crate::translation_view::TranslationView;
pub fn build(app: &Application) {
    let settings = settings::open();
    let initial_pair = settings::language_pair(&settings);
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Langux")
        .default_width(500)
        .default_height(520)
        .resizable(true)
        .build();

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

    let window_for_preferences = window.clone();
    let app_for_preferences = app.clone();
    let settings_for_preferences = settings.clone();
    settings_button.connect_clicked(move |_| {
        crate::preferences::present(
            &app_for_preferences,
            &window_for_preferences,
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
    translation_flow::connect(
        &window,
        &input_view,
        &language_controls.source_dropdown,
        &language_controls.target_dropdown,
        initial_pair,
        settings,
        translation_view,
        Arc::new(SecretTranslationProvider),
    );
    window.set_focus(Some(&input_view));
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
