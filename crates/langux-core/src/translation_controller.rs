use crate::{LanguagePair, TranslationError, TranslationRequest, TranslationResult};

/// Determines when the UI should ask the controller to translate current text.
///
/// Live mode leaves scheduling to the caller so it can apply the debounce
/// policy. This controller deliberately does not own timers or UI concerns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationMode {
    /// Translate only when the user explicitly requests it.
    Manual,
    /// Translate after the caller's live-translation scheduling policy fires.
    Live,
}

/// The visible lifecycle state of the current translation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TranslationState {
    /// There is no current translation result or request.
    Idle,
    /// The executor is processing the current input.
    Translating,
    /// The current input was translated successfully.
    Success(TranslationResult),
    /// The current input could not be translated.
    Error(TranslationError),
}

/// Coordinates current text, language context, mode, and translation state.
///
/// The controller contains no provider or UI dependency. Callers may execute
/// returned requests asynchronously, or use [`Self::translate_with`] to
/// inject a synchronous executor.
pub struct TranslationController {
    input_text: String,
    language_pair: LanguagePair,
    mode: TranslationMode,
    state: TranslationState,
}

impl TranslationController {
    /// Creates an idle controller with the selected language pair and mode.
    pub fn new(language_pair: LanguagePair, mode: TranslationMode) -> Self {
        Self {
            input_text: String::new(),
            language_pair,
            mode,
            state: TranslationState::Idle,
        }
    }

    /// Returns the current input text.
    pub fn input_text(&self) -> &str {
        &self.input_text
    }

    /// Returns the current source and target language selection.
    pub fn language_pair(&self) -> &LanguagePair {
        &self.language_pair
    }

    /// Returns the current translation mode.
    pub fn mode(&self) -> TranslationMode {
        self.mode
    }

    /// Returns the current translation lifecycle state.
    pub fn state(&self) -> &TranslationState {
        &self.state
    }

    /// Replaces the input text and clears a stale result when it changes.
    pub fn set_input_text(&mut self, input_text: impl Into<String>) {
        let input_text = input_text.into();
        if self.input_text != input_text {
            self.input_text = input_text;
            self.state = TranslationState::Idle;
        }
    }

    /// Replaces the language context and clears a stale result when it changes.
    pub fn set_language_pair(&mut self, language_pair: LanguagePair) {
        if self.language_pair != language_pair {
            self.language_pair = language_pair;
            self.state = TranslationState::Idle;
        }
    }

    /// Changes whether callers should translate on explicit action or after
    /// their live-translation scheduling policy fires.
    pub fn set_mode(&mut self, mode: TranslationMode) {
        self.mode = mode;
    }

    /// Starts translation for the current input and returns its request.
    ///
    /// Whitespace-only input returns `None` and leaves the controller idle.
    /// The original input, including surrounding whitespace, is otherwise
    /// sent unchanged. The caller can execute the request and apply its result
    /// with [`Self::finish_translation`].
    pub fn begin_translation(&mut self) -> Option<TranslationRequest> {
        if self.input_text.trim().is_empty() {
            self.state = TranslationState::Idle;
            return None;
        }

        let request = TranslationRequest::new(
            self.input_text.clone(),
            self.language_pair.source_language().clone(),
            self.language_pair.target_language().clone(),
        );
        self.state = TranslationState::Translating;
        Some(request)
    }

    /// Applies a completed translation if the controller is still translating.
    ///
    /// Returns `false` when no translation is in progress, such as after the
    /// input or language context changed while a request was running.
    pub fn finish_translation(
        &mut self,
        outcome: Result<TranslationResult, TranslationError>,
    ) -> bool {
        if self.state != TranslationState::Translating {
            return false;
        }

        self.state = match outcome {
            Ok(result) => TranslationState::Success(result),
            Err(error) => TranslationState::Error(error),
        };
        true
    }

    /// Runs the current request through an injected synchronous executor.
    ///
    /// Blank input does not call the executor. Asynchronous callers can use
    /// [`Self::begin_translation`] and [`Self::finish_translation`] instead.
    pub fn translate_with<E>(&mut self, mut execute: E) -> &TranslationState
    where
        E: FnMut(TranslationRequest) -> Result<TranslationResult, TranslationError>,
    {
        if let Some(request) = self.begin_translation() {
            let outcome = execute(request);
            self.finish_translation(outcome);
        }
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::{LanguageCode, LanguagePair, SourceLanguage, TranslationError, TranslationResult};

    use super::{TranslationController, TranslationMode, TranslationState};

    fn code(value: &str) -> LanguageCode {
        LanguageCode::new(value).expect("valid language code")
    }

    fn pair(source: &str, target: &str) -> LanguagePair {
        LanguagePair::new(SourceLanguage::Specific(code(source)), code(target))
            .expect("supported language pair")
    }

    #[test]
    fn controller_starts_idle_with_explicit_configuration() {
        let controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);

        assert_eq!(controller.input_text(), "");
        assert_eq!(controller.language_pair(), &pair("pt", "en"));
        assert_eq!(controller.mode(), TranslationMode::Manual);
        assert_eq!(controller.state(), &TranslationState::Idle);
    }

    #[test]
    fn whitespace_only_input_never_starts_or_executes_translation() {
        let calls = Rc::new(Cell::new(0));
        let calls_from_executor = Rc::clone(&calls);
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Live);
        controller.set_input_text(" \n\t ");

        assert_eq!(controller.begin_translation(), None);
        assert_eq!(controller.state(), &TranslationState::Idle);
        assert_eq!(
            controller.translate_with(move |_| {
                calls_from_executor.set(calls_from_executor.get() + 1);
                Ok(TranslationResult::new("translated", None))
            }),
            &TranslationState::Idle
        );
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn begin_translation_exposes_request_and_translating_state() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("  olá  ");

        let request = controller.begin_translation().expect("nonblank input");

        assert_eq!(request.text, "  olá  ");
        assert_eq!(
            request.source_language,
            SourceLanguage::Specific(code("pt"))
        );
        assert_eq!(request.target_language, code("en"));
        assert_eq!(controller.state(), &TranslationState::Translating);
    }

    #[test]
    fn injected_executor_result_becomes_success_state() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");

        assert_eq!(
            controller.translate_with(|request| {
                assert_eq!(request.text, "olá");
                Ok(TranslationResult::new("hello", Some(code("pt"))))
            }),
            &TranslationState::Success(TranslationResult::new("hello", Some(code("pt"))))
        );
    }

    #[test]
    fn injected_executor_error_becomes_error_state() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");

        assert_eq!(
            controller.translate_with(|_| Err(TranslationError::NetworkFailure)),
            &TranslationState::Error(TranslationError::NetworkFailure)
        );
    }

    #[test]
    fn completion_is_ignored_after_input_or_context_change() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");
        assert!(controller.begin_translation().is_some());

        controller.set_input_text("bom dia");
        assert_eq!(controller.state(), &TranslationState::Idle);
        assert!(!controller.finish_translation(Ok(TranslationResult::new("hello", None))));

        assert!(controller.begin_translation().is_some());
        controller.set_language_pair(pair("pt", "es"));
        assert_eq!(controller.state(), &TranslationState::Idle);
        assert!(!controller.finish_translation(Err(TranslationError::NetworkFailure)));
    }

    #[test]
    fn unchanged_input_and_context_preserve_current_state() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");
        controller.translate_with(|_| Ok(TranslationResult::new("hello", None)));

        controller.set_input_text("olá");
        controller.set_language_pair(pair("pt", "en"));

        assert!(matches!(controller.state(), TranslationState::Success(_)));
    }

    #[test]
    fn mode_can_switch_without_changing_input_or_context() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");
        controller.set_mode(TranslationMode::Live);

        assert_eq!(controller.mode(), TranslationMode::Live);
        assert_eq!(controller.input_text(), "olá");
        assert_eq!(controller.language_pair(), &pair("pt", "en"));
        assert_eq!(controller.state(), &TranslationState::Idle);
    }
}
