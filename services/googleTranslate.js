import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Soup from 'gi://Soup?version=3.0';

import {SecretStore} from './secretStore.js';

const DEFAULT_ENDPOINT = 'https://translation.googleapis.com/language/translate/v2';
const REQUEST_TIMEOUT_SECONDS = 30;

export const ErrorCode = Object.freeze({
    MISSING_CREDENTIAL: 'missing-credential',
    NETWORK: 'network',
    UNAUTHORIZED: 'unauthorized',
    QUOTA: 'quota',
    SERVER: 'server',
    MALFORMED: 'malformed',
    CANCELLED: 'cancelled',
});

export class TranslateError extends Error {
    constructor(code, message) {
        super(message);
        this.name = 'TranslateError';
        this.code = code;
    }
}

function callAsync(asyncFn, finishFn, ...args) {
    return new Promise((resolve, reject) => {
        args.push((_sourceObject, result) => {
            try {
                resolve(finishFn(result));
            } catch (error) {
                reject(error);
            }
        });
        asyncFn(...args);
    });
}

function sendAndRead(session, message, cancellable) {
    return callAsync(
        session.send_and_read_async.bind(session),
        session.send_and_read_finish.bind(session),
        message,
        GLib.PRIORITY_DEFAULT,
        cancellable,
    );
}

function normalizedError(error, message) {
    if (error && error.code === Gio.IOErrorEnum.CANCELLED)
        return new TranslateError(ErrorCode.CANCELLED, 'Translation request cancelled.');
    return new TranslateError(ErrorCode.NETWORK, `Translation request failed: ${message ?? error}`);
}

function translateServiceError(status, payload) {
    const message = payload?.error?.message ?? payload?.error?.status ?? `HTTP ${status}`;
    let code = ErrorCode.SERVER;
    if (/api key/i.test(message)) code = ErrorCode.UNAUTHORIZED;
    else if (status === 401 || status === 403) code = ErrorCode.UNAUTHORIZED;
    else if (status === 429) code = ErrorCode.QUOTA;
    else if (status === 400) code = ErrorCode.MALFORMED;
    return new TranslateError(code, message);
}

function throwIfCancelled(cancellable) {
    if (cancellable?.is_cancelled?.())
        throw new TranslateError(ErrorCode.CANCELLED, 'Translation request cancelled.');
}

export async function translate({text, source, target, cancellable}) {
    if (
        typeof text !== 'string' ||
        text.trim().length === 0 ||
        typeof target !== 'string' ||
        target.trim().length === 0
    )
        throw new TranslateError(
            ErrorCode.MALFORMED,
            'Both text and target language are required.',
        );

    throwIfCancelled(cancellable);
    if (source === target) return {text, detectedSourceLanguage: source};

    const apiKey = await SecretStore.getApiKey();
    if (!apiKey)
        throw new TranslateError(
            ErrorCode.MISSING_CREDENTIAL,
            'No Google Cloud API key configured.',
        );

    throwIfCancelled(cancellable);
    const requestBody = {q: text, target, format: 'text'};
    if (source && source !== 'auto') requestBody.source = source;

    const message = Soup.Message.new('POST', DEFAULT_ENDPOINT);
    message.request_headers.replace('X-Goog-Api-Key', apiKey);
    message.request_headers.replace('Content-Type', 'application/json');
    message.set_request_body_from_bytes(
        'application/json',
        new GLib.Bytes(new TextEncoder().encode(JSON.stringify(requestBody))),
    );

    const session = new Soup.Session({timeout: REQUEST_TIMEOUT_SECONDS});
    let bytes;
    try {
        throwIfCancelled(cancellable);
        bytes = await sendAndRead(session, message, cancellable ?? null);
    } catch (error) {
        throw normalizedError(error);
    }

    if (message.status_code !== 200) {
        const payload = _parseJson(bytes);
        throw payload
            ? translateServiceError(message.status_code, payload)
            : new TranslateError(ErrorCode.SERVER, `Google returned HTTP ${message.status_code}.`);
    }

    const response = _parseJson(bytes);
    const translations = response?.data?.translations;
    if (!Array.isArray(translations) || translations.length === 0)
        throw new TranslateError(
            ErrorCode.MALFORMED,
            'Google response did not contain a translation.',
        );

    const entry = translations[0];
    return {
        text: entry.translatedText,
        detectedSourceLanguage: entry.detectedSourceLanguage,
    };
}

function _parseJson(bytes) {
    try {
        return JSON.parse(new TextDecoder().decode(bytes.toArray()));
    } catch (_error) {
        return null;
    }
}
