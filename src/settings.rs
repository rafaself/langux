use gtk::gio;
use gtk::gio::prelude::*;
use langux_core::{LanguagePair, SourceLanguage, TranslationMode};

use langux_core::preferred_language_pair;

pub const SCHEMA_ID: &str = "io.github.rafaself.langux";
pub const SOURCE_LANGUAGE_KEY: &str = "source-language";
pub const TARGET_LANGUAGE_KEY: &str = "target-language";
pub const LIVE_TRANSLATION_KEY: &str = "translate-while-typing";
pub const CACHE_ENABLED_KEY: &str = "translation-cache-enabled";
pub const CACHE_CAPACITY_KEY: &str = "translation-cache-size";

const DEFAULT_SOURCE_LANGUAGE: &str = "auto";
const DEFAULT_TARGET_LANGUAGE: &str = "en";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreferenceWriteError {
    InvalidLanguagePair,
    SettingsUnavailable,
}

pub fn open() -> gio::Settings {
    let settings = gio::Settings::new(SCHEMA_ID);
    if read_language_pair(&settings).is_none() {
        write_language_pair(
            &settings,
            &preferred_language_pair(DEFAULT_SOURCE_LANGUAGE, DEFAULT_TARGET_LANGUAGE)
                .expect("default language pair must remain supported"),
        );
    }
    settings
}

pub fn language_pair(settings: &gio::Settings) -> LanguagePair {
    if let Some(pair) = read_language_pair(settings) {
        pair
    } else {
        let pair = preferred_language_pair(DEFAULT_SOURCE_LANGUAGE, DEFAULT_TARGET_LANGUAGE)
            .expect("default language pair must remain supported");
        write_language_pair(settings, &pair);
        pair
    }
}

pub fn save_language_pair(
    settings: &gio::Settings,
    pair: &LanguagePair,
) -> Result<(), PreferenceWriteError> {
    let source = match pair.source_language() {
        SourceLanguage::AutoDetect => DEFAULT_SOURCE_LANGUAGE,
        SourceLanguage::Specific(code) => code.as_str(),
    };
    if preferred_language_pair(source, pair.target_language().as_str()).is_none() {
        return Err(PreferenceWriteError::InvalidLanguagePair);
    }
    if read_language_pair(settings).as_ref() == Some(pair) {
        return Ok(());
    }
    if write_language_pair(settings, pair) {
        Ok(())
    } else {
        Err(PreferenceWriteError::SettingsUnavailable)
    }
}

pub fn translation_mode(settings: &gio::Settings) -> TranslationMode {
    if settings.boolean(LIVE_TRANSLATION_KEY) {
        TranslationMode::Live
    } else {
        TranslationMode::Manual
    }
}

pub fn set_translation_mode(settings: &gio::Settings, mode: TranslationMode) {
    let _ = settings.set_boolean(LIVE_TRANSLATION_KEY, mode == TranslationMode::Live);
}

pub fn cache_capacity(settings: &gio::Settings) -> usize {
    if !settings.boolean(CACHE_ENABLED_KEY) {
        return 0;
    }

    settings.int(CACHE_CAPACITY_KEY).clamp(1, 1000) as usize
}

pub fn cache_enabled(settings: &gio::Settings) -> bool {
    settings.boolean(CACHE_ENABLED_KEY)
}

pub fn raw_cache_capacity(settings: &gio::Settings) -> i32 {
    settings.int(CACHE_CAPACITY_KEY).clamp(1, 1000)
}

fn read_language_pair(settings: &gio::Settings) -> Option<LanguagePair> {
    let source = settings.string(SOURCE_LANGUAGE_KEY);
    let target = settings.string(TARGET_LANGUAGE_KEY);
    preferred_language_pair(source.as_str(), target.as_str())
}

fn write_language_pair(settings: &gio::Settings, pair: &LanguagePair) -> bool {
    let source = match pair.source_language() {
        SourceLanguage::AutoDetect => DEFAULT_SOURCE_LANGUAGE,
        SourceLanguage::Specific(code) => code.as_str(),
    };
    settings.delay();
    let source_result = settings.set_string(SOURCE_LANGUAGE_KEY, source);
    let target_result = settings.set_string(TARGET_LANGUAGE_KEY, pair.target_language().as_str());
    if source_result.is_ok() && target_result.is_ok() {
        settings.apply();
        true
    } else {
        settings.revert();
        false
    }
}
