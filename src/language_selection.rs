use langux_core::{LanguageCode, LanguagePair, SourceLanguage, supported_languages};

pub fn language_pair(source_index: u32, target_index: u32) -> Option<LanguagePair> {
    let target_language = supported_languages().get(target_index as usize)?;
    let target_code = LanguageCode::new(target_language.code()).ok()?;

    let source_language = if source_index == 0 {
        SourceLanguage::AutoDetect
    } else {
        let language = supported_languages().get(source_index as usize - 1)?;
        SourceLanguage::Specific(LanguageCode::new(language.code()).ok()?)
    };

    LanguagePair::new(source_language, target_code).ok()
}

pub fn language_indices(pair: &LanguagePair) -> Option<(u32, u32)> {
    let source_index = match pair.source_language() {
        SourceLanguage::AutoDetect => 0,
        SourceLanguage::Specific(code) => {
            supported_languages()
                .iter()
                .position(|language| language.code() == code.as_str())? as u32
                + 1
        }
    };
    let target_index = supported_languages()
        .iter()
        .position(|language| language.code() == pair.target_language().as_str())?
        as u32;

    Some((source_index, target_index))
}

#[cfg(test)]
mod tests {
    use langux_core::SourceLanguage;

    use super::{language_indices, language_pair};
    use langux_core::preferred_language_pair;

    #[test]
    fn first_source_option_maps_to_auto_detection() {
        let pair = language_pair(0, 0).expect("auto-detect to first target language");

        assert_eq!(pair.source_language(), &SourceLanguage::AutoDetect);
        assert_eq!(pair.target_language().as_str(), "en");
    }

    #[test]
    fn explicit_source_selection_accounts_for_the_auto_detect_option() {
        let pair = language_pair(2, 0).expect("second explicit source language");

        assert_eq!(
            pair.source_language(),
            &SourceLanguage::Specific(
                langux_core::LanguageCode::new("pt").expect("catalog language code")
            )
        );
        assert_eq!(pair.target_language().as_str(), "en");
    }

    #[test]
    fn out_of_range_selections_are_rejected() {
        assert!(language_pair(u32::MAX, 0).is_none());
        assert!(language_pair(0, u32::MAX).is_none());
    }

    #[test]
    fn language_indices_round_trip_supported_pairs() {
        let pair = preferred_language_pair("pt", "ja").expect("valid preferences");
        let indices = language_indices(&pair).expect("catalog indices");

        assert_eq!(language_pair(indices.0, indices.1), Some(pair));
    }
}
