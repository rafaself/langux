use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Label};

const APPLICATION_ID: &str = "io.github.rafaself.Langux";

fn main() {
    let app = Application::builder()
        .application_id(APPLICATION_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let welcome = Label::new(Some("Welcome to Langux"));
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Langux")
        .default_width(480)
        .default_height(360)
        .child(&welcome)
        .build();

    window.present();
}
