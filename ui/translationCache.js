const DEFAULT_CAPACITY = 200;

function validateCapacity(capacity) {
    if (!Number.isInteger(capacity) || capacity < 0)
        throw new RangeError('Translation cache capacity must be a non-negative integer.');
    return capacity;
}

/**
 * Build a collision-free key for the three provider dimensions.
 *
 * The text is deliberately not trimmed or normalized. Whitespace can be
 * meaningful to a provider and two otherwise identical translations must not
 * share a cache entry when their raw input differs.
 */
export function translationCacheKey(source, target, rawText) {
    return JSON.stringify([source, target, rawText]);
}

/**
 * Small insertion-ordered LRU cache for successful translations.
 *
 * Map keeps the least-recently-used item at its front. A read deletes and
 * re-inserts the item so that it becomes most recently used.
 */
export class TranslationCache {
    constructor(capacity = DEFAULT_CAPACITY) {
        this._capacity = validateCapacity(capacity);
        this._entries = new Map();
    }

    get capacity() {
        return this._capacity;
    }

    set capacity(capacity) {
        this.resize(capacity);
    }

    get size() {
        return this._entries.size;
    }

    get(source, target, rawText) {
        if (this._capacity === 0)
            return undefined;

        const key = translationCacheKey(source, target, rawText);
        if (!this._entries.has(key))
            return undefined;

        const value = this._entries.get(key);
        this._entries.delete(key);
        this._entries.set(key, value);
        return value;
    }

    has(source, target, rawText) {
        return this._capacity > 0 && this._entries.has(
            translationCacheKey(source, target, rawText));
    }

    set(source, target, rawText, result) {
        if (this._capacity === 0)
            return;

        const key = translationCacheKey(source, target, rawText);
        this._entries.delete(key);
        this._entries.set(key, result);
        this._evict();
    }

    resize(capacity) {
        this._capacity = validateCapacity(capacity);
        this._evict();
    }

    setCapacity(capacity) {
        this.resize(capacity);
    }

    clear() {
        this._entries.clear();
    }

    _evict() {
        while (this._entries.size > this._capacity)
            this._entries.delete(this._entries.keys().next().value);
    }
}

export const DEFAULT_TRANSLATION_CACHE_SIZE = DEFAULT_CAPACITY;
