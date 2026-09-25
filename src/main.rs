use gtk::gio;
use gtk::gio::prelude::*;
use gtk::glib;
use gtk::{Application, gio::ApplicationCommandLine};

mod cli_args;
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

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id(APPLICATION_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.add_main_option(
        "toggle",
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Show or hide the Langux window",
        None,
    );

    app.connect_activate(translator_window::show);
    app.connect_command_line(handle_command_line);
    app.run()
}

fn handle_command_line(app: &Application, command_line: &ApplicationCommandLine) -> glib::ExitCode {
    let arguments = command_line.arguments();
    let user_arguments = arguments
        .iter()
        .skip(1)
        .map(|argument| argument.as_os_str());
    let toggle_option = command_line.options_dict().contains("toggle");

    match cli_args::parse(user_arguments) {
        Ok(cli_args::Invocation::Show) if toggle_option => translator_window::toggle(app),
        Ok(cli_args::Invocation::Show) => translator_window::show(app),
        Ok(cli_args::Invocation::Toggle) => translator_window::toggle(app),
        Ok(cli_args::Invocation::Help) => {
            print_command_line(command_line, cli_args::USAGE, false);
        }
        Err(error) => {
            print_command_line(
                command_line,
                &format!("langux: {error}\n{}", cli_args::USAGE),
                true,
            );
            return glib::ExitCode::new(2);
        }
    }

    glib::ExitCode::SUCCESS
}

fn print_command_line(command_line: &ApplicationCommandLine, message: &str, to_stderr: bool) {
    use gtk::glib::translate::ToGlibPtr;
    use std::ffi::CString;

    let message = CString::new(message).expect("CLI messages do not contain NUL bytes");
    let command_line = command_line.to_glib_none().0;

    // GIO exposes these methods as variadic C functions. Use a fixed "%s"
    // format so user-provided arguments are always printed as data.
    unsafe {
        if to_stderr {
            gio::ffi::g_application_command_line_printerr(
                command_line,
                c"%s".as_ptr(),
                message.as_ptr(),
            );
        } else {
            gio::ffi::g_application_command_line_print(
                command_line,
                c"%s".as_ptr(),
                message.as_ptr(),
            );
        }
    }
}
