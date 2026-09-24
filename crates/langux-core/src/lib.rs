//! Platform-independent types shared by Langux translation behavior.

mod languages;
mod translation_controller;
mod translation_debounce;

pub use languages::{
    Language, LanguagePair, LanguagePairError, LanguageSwapError, find_supported_language,
    supported_languages,
};
pub use translation_controller::{TranslationController, TranslationMode, TranslationState};
pub use translation_debounce::{
    DebounceTicket, DebounceUpdate, LIVE_TRANSLATION_DEBOUNCE, LiveTranslationDebouncer,
};

/// A structurally valid language identifier.
///
/// This validates the shape of a BCP 47-style language tag, not whether a
/// particular translation provider supports it. Provider support belongs in
/// the language catalog or provider implementation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LanguageCode(String);

impl LanguageCode {
    /// Creates a language code from a primary language subtag and optional
    /// hyphen-separated subtags.
    ///
    /// The primary subtag must contain 2–8 ASCII letters. Each following
    /// subtag must contain 1–8 ASCII letters or digits.
    pub fn new(value: impl Into<String>) -> Result<Self, LanguageCodeError> {
        let value = value.into();
        let mut subtags = value.split('-');
        let primary = subtags.next().ok_or(LanguageCodeError)?;

        if !(2..=8).contains(&primary.len())
            || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
            || subtags.any(|subtag| {
                !(1..=8).contains(&subtag.len())
                    || !subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
        {
            return Err(LanguageCodeError);
        }

        Ok(Self(value))
    }

    /// Returns the original language identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The input to a translation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationRequest {
    pub text: String,
    pub source_language: SourceLanguage,
    pub target_language: LanguageCode,
}

impl TranslationRequest {
    /// Creates a request with its source and target languages already validated.
    pub fn new(
        text: impl Into<String>,
        source_language: SourceLanguage,
        target_language: LanguageCode,
    ) -> Self {
        Self {
            text: text.into(),
            source_language,
            target_language,
        }
    }
}

/// The source language for a request, including explicit auto-detection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLanguage {
    AutoDetect,
    Specific(LanguageCode),
}

/// The normalized output of a translation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationResult {
    pub translated_text: String,
    pub detected_source_language: Option<LanguageCode>,
}

impl TranslationResult {
    /// Creates a normalized result without retaining provider response data.
    pub fn new(
        translated_text: impl Into<String>,
        detected_source_language: Option<LanguageCode>,
    ) -> Self {
        Self {
            translated_text: translated_text.into(),
            detected_source_language,
        }
    }
}

/// A provider-independent translation failure category.
///
/// Provider payloads and credentials are deliberately not retained here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TranslationError {
    /// No credential was available for the configured provider.
    MissingCredential,
    /// The request could not complete because of a network failure.
    NetworkFailure,
    /// The provider rejected the credential as invalid or unauthorized.
    UnauthorizedCredential,
    /// The provider reported exhausted quota or rate limiting.
    QuotaOrRateLimit,
    /// The provider or its server reported a general failure.
    ProviderFailure,
    /// The provider response could not be interpreted as a translation result.
    MalformedResponse,
    /// The request was cancelled before completion.
    Cancelled,
}

/// An invalid language identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LanguageCodeError;

#[cfg(test)]
mod tests {
    use super::{
        LanguageCode, LanguageCodeError, SourceLanguage, TranslationRequest, TranslationResult,
    };

    #[test]
    fn language_code_accepts_language_and_region_subtags() {
        let code = LanguageCode::new("pt-BR").expect("valid language code");

        assert_eq!(code.as_str(), "pt-BR");
    }

    #[test]
    fn language_code_rejects_empty_or_malformed_subtags() {
        for value in ["", "en-", "-US", "e", "en_US", "en-verylongtag"] {
            assert_eq!(
                LanguageCode::new(value),
                Err(LanguageCodeError),
                "{value:?}"
            );
        }
    }

    #[test]
    fn request_represents_auto_detection_explicitly() {
        let target = LanguageCode::new("en").expect("valid target language");
        let request = TranslationRequest::new("olá", SourceLanguage::AutoDetect, target.clone());

        assert_eq!(request.text, "olá");
        assert_eq!(request.source_language, SourceLanguage::AutoDetect);
        assert_eq!(request.target_language, target);
    }

    #[test]
    fn result_can_include_a_detected_source_language() {
        let detected = LanguageCode::new("pt").expect("valid detected language");
        let result = TranslationResult::new("hello", Some(detected.clone()));

        assert_eq!(result.translated_text, "hello");
        assert_eq!(result.detected_source_language, Some(detected));
    }
}
