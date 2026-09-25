use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, CheckButton, DropDown, Label, Orientation,
    SpinButton, StringList,
};
use langux_core::TranslationMode;

use crate::credential_preferences;
use crate::language_controls::LanguageControls;
use crate::language_selection::language_indices;
use crate::settings;

pub fn present(app: &Application, parent: &ApplicationWindow, settings: gio::Settings) {
    let window = ApplicationWindow::builder()
        .application(app)
        .transient_for(parent)
        .modal(true)
        .title("Langux Settings")
        .default_width(520)
        .resizable(false)
        .build();

    let content = GtkBox::new(Orientation::Vertical, 14);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);

    let translation_heading = heading("Translation defaults");
    content.append(&translation_heading);

    let pair = settings::language_pair(&settings);
    let language_controls = LanguageControls::build(&pair);
    content.append(&language_controls.row);

    let language_status = Label::new(None);
    language_status.set_xalign(0.0);
    language_status.set_wrap(true);
    language_status.add_css_class("error");
    language_status.set_visible(false);
    content.append(&language_status);

    let mode_names = ["Live translation", "Manual translation"];
    let mode_refs = mode_names.iter().copied().collect::<Vec<_>>();
    let mode_model = StringList::new(&mode_refs);
    let mode_dropdown = DropDown::new(Some(mode_model), None::<gtk::Expression>);
    mode_dropdown.set_selected(u32::from(
        settings::translation_mode(&settings) == TranslationMode::Manual,
    ));
    let mode_label = Label::new(Some("_Translation mode"));
    mode_label.set_use_underline(true);
    mode_label.set_xalign(0.0);
    mode_label.set_mnemonic_widget(Some(&mode_dropdown));
    let mode_row = GtkBox::new(Orientation::Horizontal, 10);
    mode_row.append(&mode_label);
    mode_row.append(&mode_dropdown);
    content.append(&mode_row);

    let cache_enabled = CheckButton::with_label("Enable session translation cache");
    cache_enabled.set_active(settings::cache_enabled(&settings));
    content.append(&cache_enabled);

    let cache_capacity = SpinButton::with_range(1.0, 1000.0, 1.0);
    cache_capacity.set_numeric(true);
    cache_capacity.set_value(settings::raw_cache_capacity(&settings) as f64);
    cache_capacity.set_sensitive(cache_enabled.is_active());
    let cache_capacity_label = Label::new(Some("Maximum cached translations"));
    cache_capacity_label.set_xalign(0.0);
    cache_capacity_label.set_mnemonic_widget(Some(&cache_capacity));
    let cache_capacity_row = GtkBox::new(Orientation::Horizontal, 10);
    cache_capacity_row.append(&cache_capacity_label);
    cache_capacity_row.append(&cache_capacity);
    content.append(&cache_capacity_row);

    let credential_heading = heading("Google Cloud Translation");
    content.append(&credential_heading);
    content.append(&credential_preferences::build());

    window.set_child(Some(&content));
    connect_language_preferences(
        &settings,
        &language_controls.source_dropdown,
        &language_controls.target_dropdown,
        &language_status,
    );

    let settings_for_mode = settings.clone();
    mode_dropdown.connect_selected_notify(move |dropdown| {
        let mode = if dropdown.selected() == 1 {
            TranslationMode::Manual
        } else {
            TranslationMode::Live
        };
        settings::set_translation_mode(&settings_for_mode, mode);
    });

    let settings_for_cache = settings.clone();
    let capacity_for_cache = cache_capacity.clone();
    cache_enabled.connect_toggled(move |button| {
        capacity_for_cache.set_sensitive(button.is_active());
        let _ = settings_for_cache.set_boolean(settings::CACHE_ENABLED_KEY, button.is_active());
    });

    let settings_for_capacity = settings.clone();
    cache_capacity.connect_value_changed(move |spin_button| {
        let _ =
            settings_for_capacity.set_int(settings::CACHE_CAPACITY_KEY, spin_button.value_as_int());
    });

    let changed_handlers = Rc::new(RefCell::new(connect_settings_to_preferences(
        &settings,
        &language_controls.source_dropdown,
        &language_controls.target_dropdown,
        &language_status,
        &mode_dropdown,
        &cache_enabled,
        &cache_capacity,
    )));

    let settings_for_close = settings.clone();
    let changed_handlers_for_close = Rc::clone(&changed_handlers);
    window.connect_close_request(move |_| {
        for handler in changed_handlers_for_close.borrow_mut().drain(..) {
            settings_for_close.disconnect(handler);
        }
        glib::Propagation::Proceed
    });

    window.present();
}

fn heading(text: &str) -> Label {
    let label = Label::new(Some(text));
    label.set_xalign(0.0);
    label.add_css_class("heading");
    label
}

fn connect_language_preferences(
    settings: &gio::Settings,
    source: &DropDown,
    target: &DropDown,
    status: &Label,
) {
    let target_weak = target.downgrade();
    let source_settings = settings.clone();
    let status_weak = status.downgrade();
    let source_status_weak = status_weak.clone();
    source.connect_selected_notify(move |source| {
        let Some(target) = target_weak.upgrade() else {
            return;
        };
        persist_pair_from_controls(&source_settings, source, &target, &source_status_weak);
    });

    let source_weak = source.downgrade();
    let target_settings = settings.clone();
    let target_status_weak = status_weak;
    target.connect_selected_notify(move |target| {
        let Some(source) = source_weak.upgrade() else {
            return;
        };
        persist_pair_from_controls(&target_settings, &source, target, &target_status_weak);
    });
}

fn persist_pair_from_controls(
    settings: &gio::Settings,
    source: &DropDown,
    target: &DropDown,
    status: &glib::WeakRef<Label>,
) {
    let Some(pair) = crate::language_selection::language_pair(source.selected(), target.selected())
    else {
        show_language_error(status);
        return;
    };

    match settings::save_language_pair(settings, &pair) {
        Ok(()) => {
            if let Some(status) = status.upgrade() {
                status.set_text("");
                status.set_visible(false);
            }
        }
        Err(settings::PreferenceWriteError::InvalidLanguagePair) => {
            show_language_error(status);
        }
        Err(settings::PreferenceWriteError::SettingsUnavailable) => {
            if let Some(status) = status.upgrade() {
                status.set_text("Translation preferences could not be saved.");
                status.set_visible(true);
            }
        }
    }
}

fn show_language_error(status: &glib::WeakRef<Label>) {
    if let Some(status) = status.upgrade() {
        status.set_text("Choose different source and target languages.");
        status.set_visible(true);
    }
}

fn connect_settings_to_preferences(
    settings: &gio::Settings,
    source: &DropDown,
    target: &DropDown,
    language_status: &Label,
    mode: &DropDown,
    cache_enabled: &CheckButton,
    cache_capacity: &SpinButton,
) -> Vec<glib::SignalHandlerId> {
    let source_weak = source.downgrade();
    let target_weak = target.downgrade();
    let status_weak = language_status.downgrade();
    let update_languages = Rc::new(move |settings: &gio::Settings| {
        let pair = settings::language_pair(settings);
        let Some((source_index, target_index)) = language_indices(&pair) else {
            return;
        };
        if let Some(source) = source_weak.upgrade()
            && source.selected() != source_index
        {
            source.set_selected(source_index);
        }
        if let Some(target) = target_weak.upgrade()
            && target.selected() != target_index
        {
            target.set_selected(target_index);
        }
        if let Some(status) = status_weak.upgrade() {
            status.set_text("");
            status.set_visible(false);
        }
    });

    let source_handler = settings.connect_changed(Some(settings::SOURCE_LANGUAGE_KEY), {
        let update_languages = update_languages.clone();
        move |settings, _| update_languages(settings)
    });
    let target_handler = settings.connect_changed(Some(settings::TARGET_LANGUAGE_KEY), {
        let update_languages = update_languages.clone();
        move |settings, _| update_languages(settings)
    });

    let mode_weak = mode.downgrade();
    let mode_handler =
        settings.connect_changed(Some(settings::LIVE_TRANSLATION_KEY), move |settings, _| {
            if let Some(mode) = mode_weak.upgrade() {
                let selected =
                    u32::from(settings::translation_mode(settings) == TranslationMode::Manual);
                if mode.selected() != selected {
                    mode.set_selected(selected);
                }
            }
        });

    let enabled_weak = cache_enabled.downgrade();
    let cache_enabled_handler =
        settings.connect_changed(Some(settings::CACHE_ENABLED_KEY), move |settings, _| {
            if let Some(cache_enabled) = enabled_weak.upgrade() {
                let enabled = settings::cache_enabled(settings);
                if cache_enabled.is_active() != enabled {
                    cache_enabled.set_active(enabled);
                }
            }
        });

    let capacity_weak = cache_capacity.downgrade();
    let capacity_handler =
        settings.connect_changed(Some(settings::CACHE_CAPACITY_KEY), move |settings, _| {
            if let Some(cache_capacity) = capacity_weak.upgrade() {
                let capacity = settings::raw_cache_capacity(settings) as f64;
                if (cache_capacity.value() - capacity).abs() > f64::EPSILON {
                    cache_capacity.set_value(capacity);
                }
            }
        });

    vec![
        source_handler,
        target_handler,
        mode_handler,
        cache_enabled_handler,
        capacity_handler,
    ]
}
