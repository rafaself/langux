use std::collections::{HashMap, VecDeque};

use crate::{LanguageCode, SourceLanguage, TranslationRequest, TranslationResult};

/// A bounded, session-only least-recently-used translation cache.
///
/// Entries are keyed by the source language, target language, and exact input
/// text. The cache is entirely in memory and is discarded with its owner.
#[derive(Debug)]
pub struct TranslationCache {
    capacity: usize,
    entries: HashMap<TranslationCacheKey, TranslationResult>,
    recency: VecDeque<TranslationCacheKey>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct TranslationCacheKey {
    source_language: SourceLanguage,
    target_language: LanguageCode,
    exact_text: String,
}

impl TranslationCacheKey {
    fn from_request(request: &TranslationRequest) -> Self {
        Self {
            source_language: request.source_language.clone(),
            target_language: request.target_language.clone(),
            exact_text: request.text.clone(),
        }
    }
}

impl TranslationCache {
    /// Creates an empty cache with the requested maximum number of entries.
    /// A capacity of zero disables storage and lookups.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            recency: VecDeque::new(),
        }
    }

    /// Returns the maximum number of cached translations.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of currently cached translations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether the cache contains no translations.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns a clone of a cached translation and marks it most recently used.
    pub fn get(&mut self, request: &TranslationRequest) -> Option<TranslationResult> {
        if self.capacity == 0 {
            return None;
        }

        let key = TranslationCacheKey::from_request(request);
        let result = self.entries.get(&key)?.clone();
        self.mark_recently_used(key);
        Some(result)
    }

    /// Stores a successful translation and evicts least-recently-used entries
    /// until the configured capacity is satisfied.
    pub fn insert(&mut self, request: &TranslationRequest, result: TranslationResult) {
        if self.capacity == 0 {
            return;
        }

        let key = TranslationCacheKey::from_request(request);
        self.entries.insert(key.clone(), result);
        self.mark_recently_used(key);
        self.evict_over_capacity();
    }

    /// Changes the maximum number of entries, immediately evicting old ones
    /// when the cache shrinks. Zero clears and disables the cache.
    pub fn resize(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.evict_over_capacity();
    }

    /// Removes all cached translations.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.recency.clear();
    }

    fn mark_recently_used(&mut self, key: TranslationCacheKey) {
        self.recency.retain(|entry| entry != &key);
        self.recency.push_back(key);
    }

    fn evict_over_capacity(&mut self) {
        while self.entries.len() > self.capacity {
            let Some(least_recently_used) = self.recency.pop_front() else {
                break;
            };
            self.entries.remove(&least_recently_used);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{LanguageCode, SourceLanguage, TranslationRequest, TranslationResult};

    use super::TranslationCache;

    fn code(value: &str) -> LanguageCode {
        LanguageCode::new(value).expect("valid language code")
    }

    fn request(source: &str, target: &str, text: &str) -> TranslationRequest {
        TranslationRequest::new(
            text,
            if source == "auto" {
                SourceLanguage::AutoDetect
            } else {
                SourceLanguage::Specific(code(source))
            },
            code(target),
        )
    }

    fn result(text: &str) -> TranslationResult {
        TranslationResult::new(text, None)
    }

    #[test]
    fn cache_key_includes_source_target_and_exact_input() {
        let mut cache = TranslationCache::new(4);
        let original = request("en", "pt", "hello");
        cache.insert(&original, result("olá"));
        cache.insert(&request("pt", "pt", "hello"), result("hello"));
        cache.insert(&request("en", "en", "hello"), result("hello"));
        cache.insert(&request("en", "pt", " hello"), result(" olá"));

        assert_eq!(cache.get(&original), Some(result("olá")));
        assert_eq!(
            cache.get(&request("pt", "pt", "hello")),
            Some(result("hello"))
        );
        assert_eq!(
            cache.get(&request("en", "en", "hello")),
            Some(result("hello"))
        );
        assert_eq!(
            cache.get(&request("en", "pt", " hello")),
            Some(result(" olá"))
        );
        assert_eq!(cache.get(&request("en", "pt", "hello ")), None);
    }

    #[test]
    fn cache_reads_refresh_recency_and_evict_the_least_recent_entry() {
        let mut cache = TranslationCache::new(2);
        let first = request("auto", "en", "first");
        let second = request("auto", "en", "second");
        let third = request("auto", "en", "third");
        cache.insert(&first, result("1"));
        cache.insert(&second, result("2"));

        assert_eq!(cache.get(&first), Some(result("1")));
        cache.insert(&third, result("3"));

        assert_eq!(cache.get(&first), Some(result("1")));
        assert_eq!(cache.get(&second), None);
        assert_eq!(cache.get(&third), Some(result("3")));
    }

    #[test]
    fn resize_evicts_old_entries_and_zero_disables_storage() {
        let mut cache = TranslationCache::new(3);
        let one = request("auto", "en", "one");
        let two = request("auto", "en", "two");
        let three = request("auto", "en", "three");
        cache.insert(&one, result("1"));
        cache.insert(&two, result("2"));
        cache.insert(&three, result("3"));

        cache.resize(1);
        assert_eq!(cache.capacity(), 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&three), Some(result("3")));
        assert_eq!(cache.get(&one), None);

        cache.resize(0);
        assert!(cache.is_empty());
        cache.insert(&request("auto", "en", "four"), result("4"));
        assert_eq!(cache.get(&request("auto", "en", "four")), None);
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = TranslationCache::new(2);
        let first = request("auto", "en", "hello");
        cache.insert(&first, result("translated"));

        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.get(&first), None);
    }
}
