use gtk::Application;
use gtk::prelude::*;

mod credential_preferences;
mod input_key_behavior;
mod language_controls;
mod language_selection;
mod preferences;
mod secret_translation_provider;
mod settings;
mod translation_flow;
mod translation_presentation;
mod translation_view;
mod translator_window;

const APPLICATION_ID: &str = "io.github.rafaself.Langux";

fn main() {
    let app = Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    app.connect_activate(translator_window::build);
    app.run();
}
