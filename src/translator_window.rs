use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, DropDown, Label, Orientation,
    Overlay, ScrolledWindow, StringList, TextView, WrapMode,
};
use langux_core::{LanguageCode, LanguagePair, SourceLanguage, supported_languages};

pub fn build(app: &Application) {
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
    settings_button.set_tooltip_text(Some("Settings are not available yet."));
    title_row.append(&title);
    title_row.append(&settings_button);
    content.append(&title_row);

    let source_names = std::iter::once("Detect language".to_owned())
        .chain(
            supported_languages()
                .iter()
                .map(|language| language.name().to_owned()),
        )
        .collect::<Vec<_>>();
    let target_names = supported_languages()
        .iter()
        .map(|language| language.name().to_owned())
        .collect::<Vec<_>>();
    let source_name_refs = source_names.iter().map(String::as_str).collect::<Vec<_>>();
    let target_name_refs = target_names.iter().map(String::as_str).collect::<Vec<_>>();
    let source_model = StringList::new(&source_name_refs);
    let target_model = StringList::new(&target_name_refs);

    let source_dropdown = DropDown::new(Some(source_model.clone()), None::<gtk::Expression>);
    source_dropdown.set_hexpand(true);
    source_dropdown.set_selected(0);
    let (source_selector, source_label) =
        selector("_Source", &source_dropdown, "Choose the source language.");

    let target_dropdown = DropDown::new(Some(target_model.clone()), None::<gtk::Expression>);
    target_dropdown.set_hexpand(true);
    if let Some(english_index) = supported_languages()
        .iter()
        .position(|language| language.code() == "en")
    {
        target_dropdown.set_selected(english_index as u32);
    }
    let (target_selector, target_label) =
        selector("_Target", &target_dropdown, "Choose the target language.");

    let swap_button = Button::with_label("Swap");
    swap_button.set_tooltip_text(Some(
        "Swap the source and target languages. Choose an explicit source language first.",
    ));
    swap_button.set_sensitive(false);

    let swap_button_for_selection = swap_button.clone();
    source_dropdown.connect_selected_notify(move |dropdown| {
        swap_button_for_selection.set_sensitive(dropdown.selected() != 0);
    });

    let source_dropdown_for_action = source_dropdown.clone();
    let target_dropdown_for_action = target_dropdown.clone();
    swap_button.connect_clicked(move |_| {
        let source_index = source_dropdown_for_action.selected();
        let target_index = target_dropdown_for_action.selected();
        let Some(pair) = language_pair(source_index, target_index) else {
            return;
        };
        let Ok(swapped_pair) = pair.swap() else {
            return;
        };

        let SourceLanguage::Specific(new_source) = swapped_pair.source_language() else {
            return;
        };
        let Some(new_source_index) = supported_languages()
            .iter()
            .position(|language| language.code() == new_source.as_str())
        else {
            return;
        };
        let Some(new_target_index) = supported_languages()
            .iter()
            .position(|language| language.code() == swapped_pair.target_language().as_str())
        else {
            return;
        };

        source_dropdown_for_action.set_selected(new_source_index as u32 + 1);
        target_dropdown_for_action.set_selected(new_target_index as u32);
    });

    let language_row = GtkBox::new(Orientation::Horizontal, 10);
    language_row.append(&source_selector);
    language_row.append(&swap_button);
    language_row.append(&target_selector);
    content.append(&language_row);

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

    let copy_button = Button::with_label("Copy");
    copy_button.set_tooltip_text(Some("Copy the translated text to the clipboard."));
    let result_buffer_for_copy = result_view.buffer();
    copy_button.connect_clicked(move |button| {
        let (start, end) = result_buffer_for_copy.bounds();
        let text = result_buffer_for_copy.text(&start, &end, true);
        if !text.is_empty() {
            button.display().clipboard().set_text(text.as_str());
        }
    });

    let result_heading = GtkBox::new(Orientation::Horizontal, 8);
    result_heading.append(&result_label);
    result_heading.append(&copy_button);

    let result_section = GtkBox::new(Orientation::Vertical, 8);
    result_section.set_vexpand(true);
    result_section.append(&result_heading);
    result_section.append(&result_overlay);
    content.append(&result_section);

    source_label.set_mnemonic_widget(Some(&source_dropdown));
    target_label.set_mnemonic_widget(Some(&target_dropdown));

    window.set_child(Some(&content));
    window.present();
}

fn selector(label_text: &str, dropdown: &DropDown, tooltip: &str) -> (GtkBox, Label) {
    let label = Label::new(Some(label_text));
    label.set_use_underline(true);
    label.set_xalign(0.0);

    let selector = GtkBox::new(Orientation::Vertical, 6);
    selector.set_hexpand(true);
    selector.append(&label);
    selector.append(dropdown);
    dropdown.set_tooltip_text(Some(tooltip));

    (selector, label)
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

fn language_pair(source_index: u32, target_index: u32) -> Option<LanguagePair> {
    if source_index == 0 {
        return None;
    }

    let source_language = supported_languages().get(source_index as usize - 1)?;
    let target_language = supported_languages().get(target_index as usize)?;
    let source_code = LanguageCode::new(source_language.code()).ok()?;
    let target_code = LanguageCode::new(target_language.code()).ok()?;

    LanguagePair::new(SourceLanguage::Specific(source_code), target_code).ok()
}
