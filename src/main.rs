use gtk::Application;
use gtk::prelude::*;

mod translator_window;

const APPLICATION_ID: &str = "io.github.rafaself.Langux";

fn main() {
    let app = Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    app.connect_activate(translator_window::build);
    app.run();
}
