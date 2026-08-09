import {TranslationCache, DEFAULT_TRANSLATION_CACHE_SIZE} from './translationCache.js';

export const TRANSLATION_DEBOUNCE_MS = 1000;

const noop = () => {};

function defaultSchedule(callback, delayMs) {
    if (typeof globalThis.setTimeout !== 'function')
        throw new Error('A translation timer scheduler is required in this runtime.');
    return globalThis.setTimeout(callback, delayMs);
}

function defaultCancelSchedule(sourceId) {
    if (typeof globalThis.clearTimeout === 'function')
        globalThis.clearTimeout(sourceId);
}

function sameKey(left, right) {
    return left?.source === right?.source &&
        left?.target === right?.target &&
        left?.text === right?.text;
}

function isBlank(text) {
    return text.trim().length === 0;
}

/**
 * Coordinates input changes and cancellable translation requests without
 * depending on Shell, GTK, or a particular timer implementation.
 *
 * The UI supplies the provider function and lifecycle callbacks. Keeping the
 * scheduling and request-generation rules here makes them testable without a
 * running GNOME Shell.
 */
export class TranslationController {
    constructor({
        translate,
        cache = new TranslationCache(DEFAULT_TRANSLATION_CACHE_SIZE),
        source = 'auto',
        target = 'en',
        translateWhileTyping = true,
        debounceMs = TRANSLATION_DEBOUNCE_MS,
        schedule = defaultSchedule,
        cancelSchedule = defaultCancelSchedule,
        createCancellable = () => ({cancel: noop}),
        onLoading = noop,
        onResult = noop,
        onError = noop,
        onClear = noop,
    } = {}) {
        if (typeof translate !== 'function')
            throw new TypeError('A translation function is required.');
        if (!Number.isFinite(debounceMs) || debounceMs < 0)
            throw new RangeError('Debounce delay must be a non-negative number.');
        if (typeof schedule !== 'function' || typeof cancelSchedule !== 'function')
            throw new TypeError('A timer scheduler and cancellation function are required.');

        this._translate = translate;
        this._cache = cache;
        this._source = source;
        this._target = target;
        this._text = '';
        this._translateWhileTyping = Boolean(translateWhileTyping);
        this._debounceMs = debounceMs;
        this._schedule = schedule;
        this._cancelSchedule = cancelSchedule;
        this._createCancellable = createCancellable;
        this._onLoading = onLoading;
        this._onResult = onResult;
        this._onError = onError;
        this._onClear = onClear;

        this._destroyed = false;
        this._generation = 0;
        this._cacheGeneration = 0;
        this._pendingTimer = null;
        this._activeRequest = null;
        this._lastResultKey = null;
    }

    get cache() {
        return this._cache;
    }

    get text() {
        return this._text;
    }

    get source() {
        return this._source;
    }

    get target() {
        return this._target;
    }

    get translateWhileTyping() {
        return this._translateWhileTyping;
    }

    get hasPendingTranslation() {
        return this._pendingTimer !== null;
    }

    get hasActiveRequest() {
        return this._activeRequest !== null;
    }

    setText(rawText) {
        if (this._destroyed)
            return false;

        const nextText = rawText ?? '';
        if (nextText === this._text)
            return false;

        this._text = nextText;
        this._lastResultKey = null;
        this._invalidateWork();
        this._onClear();

        if (this._translateWhileTyping && !isBlank(nextText))
            this._scheduleCurrentText();
        return true;
    }

    setContext(source, target) {
        if (this._destroyed)
            return false;
        if (source === this._source && target === this._target)
            return false;

        this._source = source;
        this._target = target;
        this._lastResultKey = null;
        this._invalidateWork();
        this._onClear();
        return true;
    }

    setTranslateWhileTyping(enabled) {
        if (this._destroyed)
            return false;

        const nextValue = Boolean(enabled);
        if (nextValue === this._translateWhileTyping)
            return false;
        this._translateWhileTyping = nextValue;

        if (!nextValue) {
            // An in-flight request is still valid for its input/context. Only
            // the not-yet-started debounce is invalidated by this setting.
            if (this._pendingTimer !== null) {
                this._generation++;
                this._cancelPendingTimer();
            }
            return true;
        }

        if (!isBlank(this._text) && !this._activeRequest &&
            !sameKey(this._lastResultKey, this._currentKey())) {
            this._scheduleCurrentText();
        }
        return true;
    }

    setCacheSize(size) {
        if (this._destroyed)
            return;
        this._cache.resize(size);
    }

    /**
     * Clear cached translations without changing what the popup displays.
     * A request already in flight may still update the UI, but its result is
     * not inserted after this clear action.
     */
    clearCache() {
        if (this._destroyed)
            return;
        this._cache.clear();
        this._cacheGeneration++;
    }

    translateNow() {
        if (this._destroyed)
            return false;
        if (isBlank(this._text)) {
            this._lastResultKey = null;
            this._invalidateWork();
            this._onClear();
            return false;
        }

        const currentKey = this._currentKey();
        if (this._activeRequest && sameKey(this._activeRequest.key, currentKey))
            return false;

        this._cancelPendingTimer();
        return this._requestCurrentText();
    }

    destroy() {
        if (this._destroyed)
            return;
        this._destroyed = true;
        this._generation++;
        this._cancelPendingTimer();
        this._cancelActiveRequest();
        this._cache.clear();
        this._lastResultKey = null;
    }

    _currentKey() {
        return {source: this._source, target: this._target, text: this._text};
    }

    _invalidateWork() {
        this._generation++;
        this._cancelPendingTimer();
        this._cancelActiveRequest();
    }

    _scheduleCurrentText() {
        if (this._destroyed || this._pendingTimer !== null || isBlank(this._text))
            return;

        const generation = this._generation;
        const key = this._currentKey();
        this._pendingTimer = this._schedule(() => {
            this._pendingTimer = null;
            if (this._destroyed || generation !== this._generation ||
                !sameKey(key, this._currentKey())) {
                return;
            }
            this._requestCurrentText();
        }, this._debounceMs);
    }

    _cancelPendingTimer() {
        if (this._pendingTimer === null)
            return;
        this._cancelSchedule(this._pendingTimer);
        this._pendingTimer = null;
    }

    _cancelActiveRequest() {
        if (!this._activeRequest)
            return;
        const request = this._activeRequest;
        this._activeRequest = null;
        request.cancellable?.cancel?.();
    }

    _requestCurrentText() {
        if (this._destroyed || isBlank(this._text))
            return false;

        const key = this._currentKey();
        if (this._activeRequest && sameKey(this._activeRequest.key, key))
            return false;

        this._generation++;
        this._cancelActiveRequest();

        const cached = this._cache.get(key.source, key.target, key.text);
        if (cached !== undefined) {
            this._lastResultKey = key;
            this._onResult(cached);
            return true;
        }

        const request = {
            key,
            generation: this._generation,
            cacheGeneration: this._cacheGeneration,
            cancellable: this._createCancellable(),
        };
        this._activeRequest = request;
        this._onLoading();

        Promise.resolve()
            .then(() => this._translate({
                text: key.text,
                source: key.source,
                target: key.target,
                cancellable: request.cancellable,
            }))
            .then(result => this._completeRequest(request, result))
            .catch(error => this._failRequest(request, error));
        return true;
    }

    _isCurrentRequest(request) {
        return !this._destroyed && this._activeRequest === request &&
            request.generation === this._generation &&
            sameKey(request.key, this._currentKey());
    }

    _completeRequest(request, result) {
        if (!this._isCurrentRequest(request))
            return;

        this._activeRequest = null;
        if (request.cacheGeneration === this._cacheGeneration)
            this._cache.set(request.key.source, request.key.target, request.key.text, result);
        this._lastResultKey = request.key;
        this._onResult(result);
    }

    _failRequest(request, error) {
        if (!this._isCurrentRequest(request))
            return;

        this._activeRequest = null;
        if (error?.code === 'cancelled')
            return;
        this._onError(error);
    }
}

export function isBlankTranslationText(text) {
    return isBlank(text);
}
