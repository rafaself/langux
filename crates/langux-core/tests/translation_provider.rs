use std::sync::Mutex;

use langux_core::{
    CancellationToken, LanguageCode, LanguagePair, SourceLanguage, TranslationController,
    TranslationError, TranslationMode, TranslationProvider, TranslationRequest, TranslationResult,
    TranslationState,
};

struct FakeProvider {
    requests: Mutex<Vec<TranslationRequest>>,
    outcome: Result<TranslationResult, TranslationError>,
}

impl FakeProvider {
    fn returning(outcome: Result<TranslationResult, TranslationError>) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            outcome,
        }
    }

    fn requests(&self) -> Vec<TranslationRequest> {
        self.requests
            .lock()
            .expect("fake provider request lock")
            .clone()
    }
}

impl TranslationProvider for FakeProvider {
    fn translate(
        &self,
        request: &TranslationRequest,
        cancellation: &CancellationToken,
    ) -> Result<TranslationResult, TranslationError> {
        if cancellation.is_cancelled() {
            return Err(TranslationError::Cancelled);
        }

        self.requests
            .lock()
            .expect("fake provider request lock")
            .push(request.clone());
        self.outcome.clone()
    }
}

fn code(value: &str) -> LanguageCode {
    LanguageCode::new(value).expect("valid language code")
}

fn pair(source: &str, target: &str) -> LanguagePair {
    LanguagePair::new(SourceLanguage::Specific(code(source)), code(target))
        .expect("supported language pair")
}

#[test]
fn controller_uses_the_explicitly_injected_provider() {
    let provider = FakeProvider::returning(Ok(TranslationResult::new("hello", Some(code("pt")))));
    let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Manual);
    controller.set_input_text("olá");

    assert_eq!(
        controller.translate_with_provider(&provider),
        &TranslationState::Success(TranslationResult::new("hello", Some(code("pt"))))
    );
    assert_eq!(
        provider.requests(),
        vec![TranslationRequest::new(
            "olá",
            SourceLanguage::Specific(code("pt")),
            code("en"),
        )]
    );
}

#[test]
fn injected_provider_drives_idle_translating_and_terminal_states() {
    let success_provider =
        FakeProvider::returning(Ok(TranslationResult::new("hello", Some(code("pt")))));
    let mut success_controller =
        TranslationController::new(pair("pt", "en"), TranslationMode::Live);
    assert_eq!(success_controller.state(), &TranslationState::Idle);
    success_controller.set_input_text("olá");

    let success_operation = success_controller
        .begin_translation()
        .expect("nonblank input starts a translation");
    assert_eq!(success_controller.state(), &TranslationState::Translating);
    let success_outcome = success_provider.translate(
        success_operation.request(),
        &success_operation.cancellation_token(),
    );
    assert!(success_controller.finish_translation(&success_operation, success_outcome));
    assert_eq!(
        success_controller.state(),
        &TranslationState::Success(TranslationResult::new("hello", Some(code("pt"))))
    );

    let error_provider = FakeProvider::returning(Err(TranslationError::NetworkFailure));
    let mut error_controller = TranslationController::new(pair("pt", "en"), TranslationMode::Live);
    error_controller.set_input_text("olá");

    let error_operation = error_controller
        .begin_translation()
        .expect("nonblank input starts a translation");
    assert_eq!(error_controller.state(), &TranslationState::Translating);
    let error_outcome = error_provider.translate(
        error_operation.request(),
        &error_operation.cancellation_token(),
    );
    assert!(error_controller.finish_translation(&error_operation, error_outcome));
    assert_eq!(
        error_controller.state(),
        &TranslationState::Error(TranslationError::NetworkFailure)
    );
}

#[test]
fn provider_and_controller_share_cancellation_and_ignore_late_completion() {
    let provider = FakeProvider::returning(Ok(TranslationResult::new("hello", None)));
    let mut controller = TranslationController::new(pair("pt", "en"), TranslationMode::Live);
    controller.set_input_text("olá");
    let operation = controller.begin_translation().expect("nonblank input");
    let cancellation = operation.cancellation_token();

    assert!(!cancellation.is_cancelled());
    assert!(controller.cancel_translation());
    assert!(cancellation.is_cancelled());

    let outcome = provider.translate(operation.request(), &cancellation);
    assert_eq!(outcome, Err(TranslationError::Cancelled));
    assert!(provider.requests().is_empty());
    assert!(!controller.finish_translation(&operation, outcome));
    assert_eq!(controller.state(), &TranslationState::Cancelled);
}

#[test]
fn blank_input_and_cache_hits_do_not_call_the_provider() {
    let provider = FakeProvider::returning(Ok(TranslationResult::new("hello", None)));
    let mut controller =
        TranslationController::with_cache_capacity(pair("pt", "en"), TranslationMode::Manual, 2);

    controller.set_input_text(" \n ");
    assert_eq!(
        controller.translate_with_provider(&provider),
        &TranslationState::Idle
    );
    assert!(provider.requests().is_empty());

    controller.set_input_text("olá");
    controller.translate_with_provider(&provider);
    controller.translate_with_provider(&provider);

    assert_eq!(provider.requests().len(), 1);
    assert_eq!(controller.cached_translation_count(), 1);
}
