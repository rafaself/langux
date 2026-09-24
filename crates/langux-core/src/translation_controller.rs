use crate::{
    CancellationToken, LanguagePair, TranslationCache, TranslationError, TranslationProvider,
    TranslationRequest, TranslationResult,
};

/// The default number of successful translations retained in memory.
/// Zero keeps the optional cache disabled until a caller configures a capacity.
pub const DEFAULT_TRANSLATION_CACHE_CAPACITY: usize = 0;

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
    /// The current translation was cancelled.
    ///
    /// Cancellation is a lifecycle outcome, not a provider failure to show
    /// as an error to the user.
    Cancelled,
    /// The current input was translated successfully.
    Success(TranslationResult),
    /// The current input could not be translated.
    Error(TranslationError),
}

/// One translation request and its cancellation signal.
///
/// Clones share the same signal, so a worker can retain an operation while
/// the controller cancels it. Callers should check [`Self::is_cancelled`]
/// before applying any work and return its completion to the controller with
/// [`TranslationController::finish_translation`].
#[derive(Clone, Debug)]
pub struct TranslationOperation {
    request: TranslationRequest,
    cancellation: CancellationToken,
}

impl TranslationOperation {
    /// Returns the request associated with this operation.
    pub fn request(&self) -> &TranslationRequest {
        &self.request
    }

    /// Returns whether the controller has cancelled this operation.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    /// Returns a cloneable signal for passing to an injected provider.
    ///
    /// Clones share the controller-owned cancellation state, so a worker can
    /// observe cancellation after the operation has moved off-thread.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

/// Coordinates current text, language context, mode, and translation state.
///
/// The controller contains no provider or UI dependency. Callers may execute
/// returned operations asynchronously, or use [`Self::translate_with`] to
/// inject a synchronous executor.
pub struct TranslationController {
    input_text: String,
    language_pair: LanguagePair,
    mode: TranslationMode,
    state: TranslationState,
    active_operation: Option<CancellationToken>,
    translation_cache: TranslationCache,
}

impl TranslationController {
    /// Creates an idle controller with the selected language pair and mode.
    pub fn new(language_pair: LanguagePair, mode: TranslationMode) -> Self {
        Self::with_cache_capacity(language_pair, mode, DEFAULT_TRANSLATION_CACHE_CAPACITY)
    }

    /// Creates an idle controller with an explicit session-cache capacity.
    /// A capacity of zero disables caching.
    pub fn with_cache_capacity(
        language_pair: LanguagePair,
        mode: TranslationMode,
        cache_capacity: usize,
    ) -> Self {
        Self {
            input_text: String::new(),
            language_pair,
            mode,
            state: TranslationState::Idle,
            active_operation: None,
            translation_cache: TranslationCache::new(cache_capacity),
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

    /// Returns the configured maximum number of cached translations.
    pub fn translation_cache_capacity(&self) -> usize {
        self.translation_cache.capacity()
    }

    /// Returns the number of successful translations currently cached.
    pub fn cached_translation_count(&self) -> usize {
        self.translation_cache.len()
    }

    /// Changes the cache capacity, immediately evicting old entries if needed.
    /// A capacity of zero clears and disables caching.
    pub fn resize_translation_cache(&mut self, capacity: usize) {
        self.translation_cache.resize(capacity);
    }

    /// Clears the session-only translation cache.
    pub fn clear_translation_cache(&mut self) {
        self.translation_cache.clear();
    }

    /// Replaces the input text and clears a stale result when it changes.
    pub fn set_input_text(&mut self, input_text: impl Into<String>) {
        let input_text = input_text.into();
        if self.input_text != input_text {
            self.cancel_active_operation();
            self.input_text = input_text;
            self.state = TranslationState::Idle;
        }
    }

    /// Replaces the language context and clears a stale result when it changes.
    pub fn set_language_pair(&mut self, language_pair: LanguagePair) {
        if self.language_pair != language_pair {
            self.cancel_active_operation();
            self.language_pair = language_pair;
            self.state = TranslationState::Idle;
        }
    }

    /// Changes whether callers should translate on explicit action or after
    /// their live-translation scheduling policy fires.
    pub fn set_mode(&mut self, mode: TranslationMode) {
        self.mode = mode;
    }

    /// Starts translation for the current input and returns its operation.
    ///
    /// Whitespace-only input returns `None` and leaves the controller idle.
    /// The original input, including surrounding whitespace, is otherwise
    /// sent unchanged. The caller can execute the request and apply its result
    /// with [`Self::finish_translation`]. Starting a new operation cancels
    /// any operation that is still active. A cache hit changes the state to
    /// success and returns `None`, so no provider work is started.
    pub fn begin_translation(&mut self) -> Option<TranslationOperation> {
        self.cancel_active_operation();

        if self.input_text.trim().is_empty() {
            self.state = TranslationState::Idle;
            return None;
        }

        let request = TranslationRequest::new(
            self.input_text.clone(),
            self.language_pair.source_language().clone(),
            self.language_pair.target_language().clone(),
        );
        if let Some(result) = self.translation_cache.get(&request) {
            self.state = TranslationState::Success(result);
            return None;
        }

        let cancellation = CancellationToken::new();
        self.active_operation = Some(cancellation.clone());
        self.state = TranslationState::Translating;
        Some(TranslationOperation {
            request,
            cancellation,
        })
    }

    /// Cancels the active operation, if present.
    ///
    /// The operation's cancellation signal is set before the controller moves
    /// to [`TranslationState::Cancelled`]. Returns `false` when there is no
    /// active operation to cancel.
    pub fn cancel_translation(&mut self) -> bool {
        let Some(cancellation) = self.active_operation.take() else {
            return false;
        };

        cancellation.cancel();
        self.state = TranslationState::Cancelled;
        true
    }

    /// Applies a completed translation only if its operation is still active.
    ///
    /// Returns `false` when the operation was cancelled, superseded, or its
    /// input or language context changed while it was running.
    pub fn finish_translation(
        &mut self,
        operation: &TranslationOperation,
        outcome: Result<TranslationResult, TranslationError>,
    ) -> bool {
        let Some(active_cancellation) = self.active_operation.as_ref() else {
            return false;
        };
        if !active_cancellation.belongs_to_same_operation(&operation.cancellation)
            || operation.is_cancelled()
        {
            return false;
        }

        self.active_operation = None;

        self.state = match outcome {
            Ok(result) => {
                self.translation_cache
                    .insert(&operation.request, result.clone());
                TranslationState::Success(result)
            }
            Err(TranslationError::Cancelled) => {
                operation.cancellation.cancel();
                TranslationState::Cancelled
            }
            Err(error) => TranslationState::Error(error),
        };
        true
    }

    /// Runs the current request through an injected synchronous executor.
    ///
    /// Blank input and cache hits do not call the executor. Asynchronous
    /// callers can use [`Self::begin_translation`] and
    /// [`Self::finish_translation`] instead.
    pub fn translate_with<E>(&mut self, mut execute: E) -> &TranslationState
    where
        E: FnMut(TranslationRequest) -> Result<TranslationResult, TranslationError>,
    {
        if let Some(operation) = self.begin_translation() {
            let outcome = execute(operation.request.clone());
            self.finish_translation(&operation, outcome);
        }
        &self.state
    }

    /// Runs the current request through an explicitly injected provider.
    ///
    /// Blank input and cache hits do not call the provider. For asynchronous
    /// execution, use [`Self::begin_translation`], pass the operation's
    /// request and cancellation token to the provider, and apply its result
    /// with [`Self::finish_translation`].
    pub fn translate_with_provider<P>(&mut self, provider: &P) -> &TranslationState
    where
        P: TranslationProvider + ?Sized,
    {
        if let Some(operation) = self.begin_translation() {
            let outcome = provider.translate(&operation.request, &operation.cancellation);
            self.finish_translation(&operation, outcome);
        }
        &self.state
    }

    fn cancel_active_operation(&mut self) {
        if let Some(cancellation) = self.active_operation.take() {
            cancellation.cancel();
        }
    }
}

impl Drop for TranslationController {
    fn drop(&mut self) {
        self.cancel_active_operation();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use crate::{LanguageCode, LanguagePair, SourceLanguage, TranslationError, TranslationResult};

    use super::{
        DEFAULT_TRANSLATION_CACHE_CAPACITY, TranslationController, TranslationMode,
        TranslationState,
    };

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
        assert_eq!(
            controller.translation_cache_capacity(),
            DEFAULT_TRANSLATION_CACHE_CAPACITY
        );
    }

    #[test]
    fn cache_hit_skips_executor_and_restores_cached_success() {
        let calls = Rc::new(Cell::new(0));
        let mut controller =
            TranslationController::with_cache_capacity(pair("pt", "en"), TranslationMode::Live, 2);
        controller.set_input_text("olá");

        let calls_for_executor = Rc::clone(&calls);
        assert_eq!(
            controller.translate_with(move |_| {
                calls_for_executor.set(calls_for_executor.get() + 1);
                Ok(TranslationResult::new("hello", Some(code("pt"))))
            }),
            &TranslationState::Success(TranslationResult::new("hello", Some(code("pt"))))
        );
        assert_eq!(calls.get(), 1);

        let calls_for_executor = Rc::clone(&calls);
        assert_eq!(
            controller.translate_with(move |_| {
                calls_for_executor.set(calls_for_executor.get() + 1);
                Ok(TranslationResult::new("unexpected", None))
            }),
            &TranslationState::Success(TranslationResult::new("hello", Some(code("pt"))))
        );

        assert_eq!(calls.get(), 1);
        assert_eq!(controller.cached_translation_count(), 1);
    }

    #[test]
    fn zero_capacity_disables_controller_caching() {
        let calls = Rc::new(Cell::new(0));
        let mut controller = TranslationController::with_cache_capacity(
            pair("pt", "en"),
            TranslationMode::Manual,
            0,
        );
        controller.set_input_text("olá");

        for _ in 0..2 {
            let calls_for_executor = Rc::clone(&calls);
            controller.translate_with(move |_| {
                calls_for_executor.set(calls_for_executor.get() + 1);
                Ok(TranslationResult::new("hello", None))
            });
        }

        assert_eq!(calls.get(), 2);
        assert_eq!(controller.translation_cache_capacity(), 0);
        assert_eq!(controller.cached_translation_count(), 0);
    }

    #[test]
    fn controller_can_resize_and_clear_its_cache() {
        let calls = Rc::new(Cell::new(0));
        let mut controller = TranslationController::with_cache_capacity(
            pair("pt", "en"),
            TranslationMode::Manual,
            3,
        );

        for input in ["one", "two", "three"] {
            controller.set_input_text(input);
            let calls_for_executor = Rc::clone(&calls);
            controller.translate_with(move |_| {
                calls_for_executor.set(calls_for_executor.get() + 1);
                Ok(TranslationResult::new("translated", None))
            });
        }
        assert_eq!(controller.cached_translation_count(), 3);

        controller.resize_translation_cache(1);
        assert_eq!(controller.translation_cache_capacity(), 1);
        assert_eq!(controller.cached_translation_count(), 1);

        controller.set_input_text("one");
        let calls_for_executor = Rc::clone(&calls);
        controller.translate_with(move |_| {
            calls_for_executor.set(calls_for_executor.get() + 1);
            Ok(TranslationResult::new("translated again", None))
        });
        assert_eq!(calls.get(), 4);
        assert_eq!(controller.cached_translation_count(), 1);

        controller.clear_translation_cache();
        assert_eq!(controller.cached_translation_count(), 0);
    }

    #[test]
    fn asynchronous_begin_returns_no_operation_for_a_cache_hit() {
        let mut controller = TranslationController::with_cache_capacity(
            pair("pt", "en"),
            TranslationMode::Manual,
            2,
        );
        controller.set_input_text("olá");
        let operation = controller.begin_translation().expect("initial cache miss");
        assert!(
            controller.finish_translation(&operation, Ok(TranslationResult::new("hello", None)))
        );

        controller.set_input_text("bom dia");
        controller.set_input_text("olá");
        assert!(controller.begin_translation().is_none());
        assert_eq!(
            controller.state(),
            &TranslationState::Success(TranslationResult::new("hello", None))
        );
    }

    #[test]
    fn whitespace_only_input_never_starts_or_executes_translation() {
        let calls = Rc::new(Cell::new(0));
        let calls_from_executor = Rc::clone(&calls);
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Live);
        controller.set_input_text(" \n\t ");

        assert!(controller.begin_translation().is_none());
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

        let operation = controller.begin_translation().expect("nonblank input");

        assert_eq!(operation.request().text, "  olá  ");
        assert_eq!(
            operation.request().source_language,
            SourceLanguage::Specific(code("pt"))
        );
        assert_eq!(operation.request().target_language, code("en"));
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
        let input_operation = controller.begin_translation().expect("input request");

        controller.set_input_text("bom dia");
        assert!(input_operation.is_cancelled());
        assert_eq!(controller.state(), &TranslationState::Idle);
        assert!(
            !controller
                .finish_translation(&input_operation, Ok(TranslationResult::new("hello", None)))
        );

        let language_operation = controller.begin_translation().expect("language request");
        controller.set_language_pair(pair("pt", "es"));
        assert!(language_operation.is_cancelled());
        assert_eq!(controller.state(), &TranslationState::Idle);
        assert!(
            !controller
                .finish_translation(&language_operation, Err(TranslationError::NetworkFailure))
        );
    }

    #[test]
    fn starting_a_new_operation_cancels_the_old_and_rejects_its_late_result() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Live);
        controller.set_input_text("first");
        let first = controller.begin_translation().expect("first operation");

        controller.set_input_text("latest");
        let latest = controller.begin_translation().expect("latest operation");

        assert!(first.is_cancelled());
        assert!(!latest.is_cancelled());
        assert!(
            !controller
                .finish_translation(&first, Ok(TranslationResult::new("stale result", None)))
        );
        assert!(
            controller
                .finish_translation(&latest, Ok(TranslationResult::new("current result", None)))
        );
        assert_eq!(
            controller.state(),
            &TranslationState::Success(TranslationResult::new("current result", None))
        );
    }

    #[test]
    fn out_of_order_completion_cannot_replace_a_newer_result() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Live);
        controller.set_input_text("same input");
        let older = controller.begin_translation().expect("older operation");
        let newer = controller.begin_translation().expect("newer operation");

        assert!(
            controller.finish_translation(&newer, Ok(TranslationResult::new("newer result", None)))
        );
        assert!(
            !controller
                .finish_translation(&older, Ok(TranslationResult::new("older result", None)))
        );
        assert_eq!(
            controller.state(),
            &TranslationState::Success(TranslationResult::new("newer result", None))
        );
    }

    #[test]
    fn explicit_and_normalized_cancellation_do_not_become_provider_errors() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");
        let operation = controller.begin_translation().expect("operation");

        assert!(controller.cancel_translation());
        assert!(operation.is_cancelled());
        assert_eq!(controller.state(), &TranslationState::Cancelled);
        assert!(!controller.cancel_translation());
        assert!(!controller.finish_translation(&operation, Err(TranslationError::NetworkFailure)));

        let next = controller.begin_translation().expect("next operation");
        assert!(controller.finish_translation(&next, Err(TranslationError::Cancelled)));
        assert_eq!(controller.state(), &TranslationState::Cancelled);
    }

    #[test]
    fn dropping_controller_cancels_outstanding_operation() {
        let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
        controller.set_input_text("olá");
        let operation = controller.begin_translation().expect("operation");

        drop(controller);

        assert!(operation.is_cancelled());
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
