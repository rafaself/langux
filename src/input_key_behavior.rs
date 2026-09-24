#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputKey {
    Escape,
    Enter,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum KeyAction {
    Close,
    Translate,
    PassThrough,
}

pub(crate) fn action(
    manual_mode: bool,
    key: InputKey,
    control: bool,
    shift: bool,
    other_modifier: bool,
) -> KeyAction {
    match key {
        InputKey::Escape if !control && !shift && !other_modifier => KeyAction::Close,
        InputKey::Enter if shift || other_modifier => KeyAction::PassThrough,
        InputKey::Enter if manual_mode || control => KeyAction::Translate,
        InputKey::Escape | InputKey::Enter | InputKey::Other => KeyAction::PassThrough,
    }
}

#[cfg(test)]
mod tests {
    use super::{InputKey, KeyAction, action};

    #[test]
    fn manual_enter_translates_but_shift_enter_is_left_for_text_editing() {
        assert_eq!(
            action(true, InputKey::Enter, false, false, false),
            KeyAction::Translate
        );
        assert_eq!(
            action(true, InputKey::Enter, true, false, false),
            KeyAction::Translate
        );
        assert_eq!(
            action(true, InputKey::Enter, false, true, false),
            KeyAction::PassThrough
        );
        assert_eq!(
            action(true, InputKey::Enter, true, true, false),
            KeyAction::PassThrough
        );
    }

    #[test]
    fn live_enter_edits_text_while_control_enter_translates_immediately() {
        assert_eq!(
            action(false, InputKey::Enter, false, false, false),
            KeyAction::PassThrough
        );
        assert_eq!(
            action(false, InputKey::Enter, true, false, false),
            KeyAction::Translate
        );
        assert_eq!(
            action(false, InputKey::Enter, false, true, false),
            KeyAction::PassThrough
        );
    }

    #[test]
    fn escape_closes_without_consuming_modified_escape_or_other_keys() {
        assert_eq!(
            action(false, InputKey::Escape, false, false, false),
            KeyAction::Close
        );
        assert_eq!(
            action(false, InputKey::Escape, true, false, false),
            KeyAction::PassThrough
        );
        assert_eq!(
            action(false, InputKey::Other, false, false, false),
            KeyAction::PassThrough
        );
    }
}
