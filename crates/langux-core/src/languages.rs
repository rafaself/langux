use crate::{LanguageCode, SourceLanguage};

/// A language in Langux's initial, provider-independent supported-language catalog.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Language {
    code: &'static str,
    name: &'static str,
}

impl Language {
    /// Returns the BCP 47-style language code.
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// Returns the language's English display name.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

const SUPPORTED_LANGUAGES: [Language; 9] = [
    Language {
        code: "en",
        name: "English",
    },
    Language {
        code: "pt",
        name: "Portuguese",
    },
    Language {
        code: "es",
        name: "Spanish",
    },
    Language {
        code: "ja",
        name: "Japanese",
    },
    Language {
        code: "fr",
        name: "French",
    },
    Language {
        code: "de",
        name: "German",
    },
    Language {
        code: "it",
        name: "Italian",
    },
    Language {
        code: "ko",
        name: "Korean",
    },
    Language {
        code: "zh",
        name: "Chinese",
    },
];

/// Returns the supported languages in their stable display order.
pub fn supported_languages() -> &'static [Language] {
    &SUPPORTED_LANGUAGES
}

/// Finds a supported language by its exact provider code.
pub fn find_supported_language(code: &str) -> Option<&'static Language> {
    SUPPORTED_LANGUAGES
        .iter()
        .find(|language| language.code == code)
}

/// Creates a supported language pair that is safe to persist as a translation
/// default. Auto-detection is accepted only for the source; explicit equal
/// source and target languages are rejected.
pub fn preferred_language_pair(source_code: &str, target_code: &str) -> Option<LanguagePair> {
    let source_language = if source_code == "auto" {
        SourceLanguage::AutoDetect
    } else {
        SourceLanguage::Specific(LanguageCode::new(source_code).ok()?)
    };
    let target_language = LanguageCode::new(target_code).ok()?;
    let pair = LanguagePair::new(source_language, target_language).ok()?;

    if matches!(pair.source_language(), SourceLanguage::Specific(source) if source == pair.target_language())
    {
        return None;
    }

    Some(pair)
}

/// A validated source and target language selection.
///
/// The target is a `LanguageCode`, and construction checks that it belongs to
/// the supported catalog. Auto-detection is represented only by
/// `SourceLanguage`, so it cannot be selected as a target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguagePair {
    source_language: SourceLanguage,
    target_language: LanguageCode,
}

impl LanguagePair {
    /// Creates a language pair after checking explicit languages against the
    /// supported catalog.
    pub fn new(
        source_language: SourceLanguage,
        target_language: LanguageCode,
    ) -> Result<Self, LanguagePairError> {
        if matches!(&source_language, SourceLanguage::Specific(code) if find_supported_language(code.as_str()).is_none())
        {
            return Err(LanguagePairError::UnsupportedSourceLanguage);
        }

        if find_supported_language(target_language.as_str()).is_none() {
            return Err(LanguagePairError::UnsupportedTargetLanguage);
        }

        Ok(Self {
            source_language,
            target_language,
        })
    }

    /// Returns the selected source language.
    pub fn source_language(&self) -> &SourceLanguage {
        &self.source_language
    }

    /// Returns the selected target language.
    pub fn target_language(&self) -> &LanguageCode {
        &self.target_language
    }

    /// Returns whether the current pair can be swapped.
    pub fn can_swap(&self) -> bool {
        matches!(&self.source_language, SourceLanguage::Specific(_))
    }

    /// Returns a pair with source and target exchanged.
    ///
    /// Auto-detected source selections cannot be swapped because there is no
    /// explicit source language to use as the new target.
    pub fn swap(&self) -> Result<Self, LanguageSwapError> {
        let SourceLanguage::Specific(source_language) = &self.source_language else {
            return Err(LanguageSwapError::AutoDetectedSource);
        };

        Ok(Self {
            source_language: SourceLanguage::Specific(self.target_language.clone()),
            target_language: source_language.clone(),
        })
    }
}

/// The reason a source and target selection could not be constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguagePairError {
    /// The explicit source code is not in the supported catalog.
    UnsupportedSourceLanguage,
    /// The target code is not in the supported catalog.
    UnsupportedTargetLanguage,
}

/// The reason a language pair could not be swapped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LanguageSwapError {
    /// An auto-detected source does not identify a language to use as target.
    AutoDetectedSource,
}

#[cfg(test)]
mod tests {
    use crate::{LanguageCode, SourceLanguage};

    use super::{
        LanguagePair, LanguagePairError, LanguageSwapError, find_supported_language,
        preferred_language_pair, supported_languages,
    };

    fn code(value: &str) -> LanguageCode {
        LanguageCode::new(value).expect("valid language code")
    }

    #[test]
    fn catalog_contains_the_initial_supported_languages() {
        let catalog: Vec<_> = supported_languages()
            .iter()
            .map(|language| (language.code(), language.name()))
            .collect();

        for expected in [
            ("en", "English"),
            ("pt", "Portuguese"),
            ("es", "Spanish"),
            ("ja", "Japanese"),
            ("fr", "French"),
            ("de", "German"),
            ("it", "Italian"),
            ("ko", "Korean"),
            ("zh", "Chinese"),
        ] {
            assert!(catalog.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn catalog_lookup_requires_an_exact_supported_code() {
        assert_eq!(
            find_supported_language("pt").map(|language| language.name()),
            Some("Portuguese")
        );
        assert_eq!(find_supported_language("pt-BR"), None);
        assert_eq!(find_supported_language("auto"), None);
        assert_eq!(find_supported_language("unknown"), None);
    }

    #[test]
    fn preferred_pair_rejects_unsupported_or_identical_languages() {
        assert!(preferred_language_pair("auto", "en").is_some());
        assert!(preferred_language_pair("pt", "en").is_some());
        assert!(preferred_language_pair("unsupported", "en").is_none());
        assert!(preferred_language_pair("auto", "auto").is_none());
        assert!(preferred_language_pair("pt", "pt").is_none());
    }

    #[test]
    fn pair_accepts_auto_detect_only_as_source() {
        let pair = LanguagePair::new(SourceLanguage::AutoDetect, code("en"))
            .expect("auto-detect source with explicit target is valid");

        assert_eq!(pair.source_language(), &SourceLanguage::AutoDetect);
        assert_eq!(pair.target_language().as_str(), "en");
        assert!(!pair.can_swap());
        assert_eq!(pair.swap(), Err(LanguageSwapError::AutoDetectedSource));
    }

    #[test]
    fn pair_rejects_unsupported_explicit_languages() {
        assert_eq!(
            LanguagePair::new(SourceLanguage::Specific(code("nl")), code("en")),
            Err(LanguagePairError::UnsupportedSourceLanguage)
        );
        assert_eq!(
            LanguagePair::new(SourceLanguage::Specific(code("pt")), code("auto")),
            Err(LanguagePairError::UnsupportedTargetLanguage)
        );
    }

    #[test]
    fn swap_exchanges_two_explicit_languages() {
        let pair = LanguagePair::new(SourceLanguage::Specific(code("pt")), code("en"))
            .expect("supported languages");

        assert!(pair.can_swap());
        let swapped = pair.swap().expect("explicit languages can be swapped");
        assert_eq!(
            swapped.source_language(),
            &SourceLanguage::Specific(code("en"))
        );
        assert_eq!(swapped.target_language().as_str(), "pt");
        assert_eq!(
            pair.source_language(),
            &SourceLanguage::Specific(code("pt"))
        );
        assert_eq!(pair.target_language().as_str(), "en");
    }
}
