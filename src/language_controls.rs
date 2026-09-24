use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, DropDown, Label, Orientation, StringList};
use langux_core::{LanguagePair, SourceLanguage, supported_languages};

use crate::language_selection::language_pair;

pub struct LanguageControls {
    pub row: GtkBox,
    pub source_dropdown: DropDown,
    pub target_dropdown: DropDown,
}

impl LanguageControls {
    pub fn build() -> Self {
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

        let source_dropdown = DropDown::new(Some(source_model), None::<gtk::Expression>);
        source_dropdown.set_hexpand(true);
        source_dropdown.set_selected(0);
        let (source_selector, source_label) =
            selector("_Source", &source_dropdown, "Choose the source language.");

        let target_dropdown = DropDown::new(Some(target_model), None::<gtk::Expression>);
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
            let Some(pair) = language_pair(
                source_dropdown_for_action.selected(),
                target_dropdown_for_action.selected(),
            ) else {
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

        let row = GtkBox::new(Orientation::Horizontal, 10);
        row.append(&source_selector);
        row.append(&swap_button);
        row.append(&target_selector);

        source_label.set_mnemonic_widget(Some(&source_dropdown));
        target_label.set_mnemonic_widget(Some(&target_dropdown));

        Self {
            row,
            source_dropdown,
            target_dropdown,
        }
    }

    pub fn selected_pair(&self) -> Option<LanguagePair> {
        language_pair(
            self.source_dropdown.selected(),
            self.target_dropdown.selected(),
        )
    }
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
