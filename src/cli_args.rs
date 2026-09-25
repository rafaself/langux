use std::ffi::OsStr;

pub const USAGE: &str = "Usage: langux [--toggle] [--help]\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invocation {
    Resident,
    Toggle,
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidArgument(String);

impl std::fmt::Display for InvalidArgument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "unexpected argument {:?}", self.0)
    }
}

pub fn parse<'a>(
    arguments: impl IntoIterator<Item = &'a OsStr>,
) -> Result<Invocation, InvalidArgument> {
    let mut toggle = false;
    let mut help = false;
    let mut options_ended = false;

    for argument in arguments {
        if !options_ended && argument == OsStr::new("--") {
            options_ended = true;
            continue;
        }

        if !options_ended && argument == OsStr::new("--toggle") {
            toggle = true;
        } else if !options_ended
            && (argument == OsStr::new("--help") || argument == OsStr::new("-h"))
        {
            help = true;
        } else {
            return Err(InvalidArgument(argument.to_string_lossy().into_owned()));
        }
    }

    Ok(if help {
        Invocation::Help
    } else if toggle {
        Invocation::Toggle
    } else {
        Invocation::Resident
    })
}

#[cfg(test)]
mod tests {
    use super::{InvalidArgument, Invocation, parse};
    use std::ffi::OsStr;

    fn parse_args(args: &[&str]) -> Result<Invocation, InvalidArgument> {
        parse(args.iter().map(OsStr::new))
    }

    #[test]
    fn no_arguments_requests_resident_start() {
        assert_eq!(parse_args(&[]), Ok(Invocation::Resident));
    }

    #[test]
    fn toggle_option_requests_toggle_activation() {
        assert_eq!(parse_args(&["--toggle"]), Ok(Invocation::Toggle));
    }

    #[test]
    fn help_option_is_supported() {
        assert_eq!(parse_args(&["--help"]), Ok(Invocation::Help));
        assert_eq!(parse_args(&["-h"]), Ok(Invocation::Help));
        assert_eq!(parse_args(&["--toggle", "--help"]), Ok(Invocation::Help));
    }

    #[test]
    fn unknown_options_and_positional_arguments_are_rejected() {
        assert_eq!(
            parse_args(&["--other"]),
            Err(InvalidArgument("--other".to_owned()))
        );
        assert_eq!(
            parse_args(&["input.txt"]),
            Err(InvalidArgument("input.txt".to_owned()))
        );
    }

    #[test]
    fn arguments_after_option_terminator_are_rejected() {
        assert_eq!(parse_args(&["--"]), Ok(Invocation::Resident));
        assert_eq!(
            parse_args(&["--", "input.txt"]),
            Err(InvalidArgument("input.txt".to_owned()))
        );
    }
}
