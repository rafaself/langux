use gtk::prelude::*;
use gtk::{
    Align, Box as GtkBox, Button, Label, Orientation, Overlay, ScrolledWindow, Spinner, TextBuffer,
    TextView, WrapMode,
};
use langux_core::{TranslationState, find_supported_language};

use crate::translation_presentation::present;

#[derive(Clone)]
pub struct TranslationView {
    pub section: GtkBox,
    result_buffer: TextBuffer,
    result_placeholder: Label,
    status_label: Label,
    detected_source_label: Label,
    spinner: Spinner,
    copy_button: Button,
}

impl TranslationView {
    pub fn build() -> Self {
        let (result_area, result_view) = text_area(false);
        let result_overlay = Overlay::new();
        result_overlay.set_child(Some(&result_area));

        let result_placeholder = Label::new(Some("Translation will appear here."));
        result_placeholder.set_halign(Align::Start);
        result_placeholder.set_valign(Align::Start);
        result_placeholder.set_margin_top(10);
        result_placeholder.set_margin_start(10);
        result_placeholder.set_can_target(false);
        result_overlay.add_overlay(&result_placeholder);

        let result_buffer = result_view.buffer();
        let result_placeholder_for_buffer = result_placeholder.clone();
        result_buffer.connect_changed(move |buffer| {
            result_placeholder_for_buffer.set_visible(buffer.char_count() == 0);
        });

        let result_label = Label::new(Some("Translation"));
        result_label.set_xalign(0.0);
        result_label.set_hexpand(true);
        result_label.set_mnemonic_widget(Some(&result_view));

        let copy_button = Button::with_mnemonic("_Copy");
        copy_button.set_tooltip_text(Some("Copy the translated text to the clipboard (Alt+C)."));
        let result_buffer_for_copy = result_buffer.clone();
        copy_button.connect_clicked(move |button| {
            let (start, end) = result_buffer_for_copy.bounds();
            let text = result_buffer_for_copy.text(&start, &end, true);
            if !text.is_empty() {
                button.display().clipboard().set_text(text.as_str());
            }
        });
        copy_button.set_sensitive(false);

        let result_heading = GtkBox::new(Orientation::Horizontal, 8);
        result_heading.append(&result_label);
        result_heading.append(&copy_button);

        let spinner = Spinner::new();
        spinner.set_visible(false);

        let status_label = Label::new(None);
        status_label.set_xalign(0.0);
        status_label.set_hexpand(true);
        status_label.set_wrap(true);

        let status_row = GtkBox::new(Orientation::Horizontal, 8);
        status_row.append(&spinner);
        status_row.append(&status_label);

        let detected_source_label = Label::new(None);
        detected_source_label.set_xalign(0.0);
        detected_source_label.set_visible(false);

        let result_section = GtkBox::new(Orientation::Vertical, 8);
        result_section.set_vexpand(true);
        result_section.append(&result_heading);
        result_section.append(&status_row);
        result_section.append(&result_overlay);
        result_section.append(&detected_source_label);

        Self {
            section: result_section,
            result_buffer,
            result_placeholder,
            status_label,
            detected_source_label,
            spinner,
            copy_button,
        }
    }

    pub fn render(&self, state: &TranslationState, invalid_language_pair: bool) {
        let presentation = present(state);
        let status = if invalid_language_pair {
            "Choose different source and target languages."
        } else {
            presentation.status
        };
        self.status_label.set_text(status);
        self.status_label.set_visible(!status.is_empty());
        if presentation.is_error {
            self.status_label.add_css_class("error");
        } else {
            self.status_label.remove_css_class("error");
        }

        if presentation.is_translating {
            self.spinner.set_visible(true);
            self.spinner.start();
        } else {
            self.spinner.stop();
            self.spinner.set_visible(false);
        }

        let translated_text = presentation
            .result
            .map(|result| result.translated_text.as_str())
            .unwrap_or_default();
        self.result_buffer.set_text(translated_text);
        self.result_placeholder
            .set_visible(translated_text.is_empty());
        self.copy_button.set_sensitive(!translated_text.is_empty());

        let detected_source = presentation
            .result
            .and_then(|result| result.detected_source_language.as_ref());
        if let Some(language_code) = detected_source {
            let language_name = find_supported_language(language_code.as_str())
                .map(|language| language.name())
                .unwrap_or(language_code.as_str());
            self.detected_source_label
                .set_text(&format!("Detected source: {language_name}"));
            self.detected_source_label.set_visible(true);
        } else {
            self.detected_source_label.set_text("");
            self.detected_source_label.set_visible(false);
        }
    }

    pub fn copy_result(&self) -> bool {
        let (start, end) = self.result_buffer.bounds();
        let text = self.result_buffer.text(&start, &end, true);
        if text.is_empty() {
            return false;
        }
        let display = self.section.display();
        display.clipboard().set_text(text.as_str());
        true
    }
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
