use langux_core::{TranslationError, TranslationResult, TranslationState};

pub struct TranslationPresentation<'a> {
    pub status: &'static str,
    pub result: Option<&'a TranslationResult>,
    pub is_translating: bool,
    pub is_error: bool,
}

pub fn present(state: &TranslationState) -> TranslationPresentation<'_> {
    match state {
        TranslationState::Idle => TranslationPresentation {
            status: "",
            result: None,
            is_translating: false,
            is_error: false,
        },
        TranslationState::Translating => TranslationPresentation {
            status: "Translating…",
            result: None,
            is_translating: true,
            is_error: false,
        },
        TranslationState::Cancelled => TranslationPresentation {
            status: "Translation stopped.",
            result: None,
            is_translating: false,
            is_error: false,
        },
        TranslationState::Success(result) => TranslationPresentation {
            status: "Translation complete.",
            result: Some(result),
            is_translating: false,
            is_error: false,
        },
        TranslationState::Error(error) => TranslationPresentation {
            status: error_message(error),
            result: None,
            is_translating: false,
            is_error: true,
        },
    }
}

fn error_message(error: &TranslationError) -> &'static str {
    match error {
        TranslationError::MissingCredential => "Add a translation API key in Settings.",
        TranslationError::NetworkFailure => "Check your internet connection and try again.",
        TranslationError::UnauthorizedCredential => {
            "The API key was rejected. Check it in Settings."
        }
        TranslationError::QuotaOrRateLimit => {
            "The translation quota has been reached. Try again later."
        }
        TranslationError::ProviderFailure => {
            "The translation service is unavailable. Try again later."
        }
        TranslationError::MalformedResponse => {
            "The translation service returned an unreadable result. Try again later."
        }
        TranslationError::Cancelled => "Translation stopped.",
        _ => "Translation failed. Try again later.",
    }
}

#[cfg(test)]
mod tests {
    use langux_core::{LanguageCode, TranslationResult};

    use super::present;
    use langux_core::{TranslationError, TranslationState};

    #[test]
    fn presentation_covers_idle_translating_and_success_states() {
        let idle = present(&TranslationState::Idle);
        assert_eq!(idle.status, "");
        assert!(!idle.is_translating);
        assert!(idle.result.is_none());

        let translating = present(&TranslationState::Translating);
        assert_eq!(translating.status, "Translating…");
        assert!(translating.is_translating);

        let result = TranslationResult::new(
            "hello",
            Some(LanguageCode::new("pt").expect("valid detected language")),
        );
        let success_state = TranslationState::Success(result.clone());
        let success = present(&success_state);
        assert_eq!(success.status, "Translation complete.");
        assert_eq!(success.result, Some(&result));
        assert!(!success.is_error);
    }

    #[test]
    fn failures_show_normalized_actionable_text_without_provider_details() {
        let error = present(&TranslationState::Error(TranslationError::NetworkFailure));

        assert_eq!(
            error.status,
            "Check your internet connection and try again."
        );
        assert!(error.is_error);
        assert!(error.result.is_none());
        assert!(!error.status.contains("Google"));
    }
}
