import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Soup from 'gi://Soup?version=3.0';

import {
    UPDATE_API_URL,
    getUpdateInfo,
    normalizeVersion,
} from '../ui/updateInfo.js';

export const UPDATE_REQUEST_TIMEOUT_SECONDS = 10;

export const UpdateErrorCode = Object.freeze({
    NETWORK: 'network',
    HTTP: 'http',
    MALFORMED: 'malformed',
    CANCELLED: 'cancelled',
});

export class UpdateCheckError extends Error {
    constructor(code, message) {
        super(message);
        this.name = 'UpdateCheckError';
        this.code = code;
    }
}

function callAsync(asyncFn, finishFn, ...args) {
    return new Promise((resolve, reject) => {
        args.push((sourceObject, result) => {
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
        cancellable);
}

function isCancelled(error, cancellable) {
    return error?.code === Gio.IOErrorEnum.CANCELLED ||
        (typeof cancellable?.is_cancelled === 'function' && cancellable.is_cancelled());
}

function normalizeRequestError(error, cancellable) {
    if (error instanceof UpdateCheckError)
        return error;
    if (isCancelled(error, cancellable))
        return new UpdateCheckError(UpdateErrorCode.CANCELLED, 'Update check cancelled.');
    return new UpdateCheckError(UpdateErrorCode.NETWORK, 'Unable to reach GitHub.');
}

function bytesToJson(bytes) {
    try {
        const data = bytes?.toArray?.();
        if (!data)
            throw new Error('Missing response body.');
        return JSON.parse(new TextDecoder().decode(data));
    } catch (error) {
        throw new UpdateCheckError(
            UpdateErrorCode.MALFORMED,
            'GitHub returned malformed release metadata.');
    }
}

function requestArguments(currentVersionOrOptions, maybeCancellable) {
    if (currentVersionOrOptions && typeof currentVersionOrOptions === 'object' &&
        Object.hasOwn(currentVersionOrOptions, 'currentVersion')) {
        return {
            currentVersion: currentVersionOrOptions.currentVersion,
            cancellable: currentVersionOrOptions.cancellable ?? null,
        };
    }
    return {
        currentVersion: currentVersionOrOptions,
        cancellable: maybeCancellable ?? null,
    };
}

/**
 * Fetch only the latest stable release metadata from GitHub.
 *
 * The optional positional form is retained for small callers:
 * checkForUpdates('0.1.0', cancellable). The object form is used by prefs.
 */
export async function checkForUpdates(currentVersionOrOptions, maybeCancellable) {
    const {currentVersion, cancellable} = requestArguments(
        currentVersionOrOptions,
        maybeCancellable);
    const activeCancellable = cancellable ?? new Gio.Cancellable();

    if (!normalizeVersion(currentVersion))
        throw new UpdateCheckError(
            UpdateErrorCode.MALFORMED,
            'The installed version is malformed.');

    const message = Soup.Message.new('GET', UPDATE_API_URL);
    message.request_headers.replace('Accept', 'application/vnd.github+json');
    message.request_headers.replace('User-Agent', 'Langux GNOME Shell extension');
    message.request_headers.replace('X-GitHub-Api-Version', '2022-11-28');

    const session = new Soup.Session({timeout: UPDATE_REQUEST_TIMEOUT_SECONDS});
    let bytes;
    try {
        bytes = await sendAndRead(session, message, activeCancellable);
    } catch (error) {
        throw normalizeRequestError(error, activeCancellable);
    }

    if (isCancelled(null, activeCancellable))
        throw new UpdateCheckError(UpdateErrorCode.CANCELLED, 'Update check cancelled.');

    if (message.status_code < 200 || message.status_code >= 300)
        throw new UpdateCheckError(
            UpdateErrorCode.HTTP,
            `GitHub returned HTTP ${message.status_code}.`);

    let payload;
    try {
        payload = bytesToJson(bytes);
    } catch (error) {
        throw normalizeRequestError(error, activeCancellable);
    }

    try {
        return getUpdateInfo(currentVersion, payload);
    } catch (error) {
        throw new UpdateCheckError(
            UpdateErrorCode.MALFORMED,
            'GitHub returned invalid release metadata.');
    }
}

/**
 * Own a request cancellable for a Preferences window or another short-lived
 * caller. No result is cached and a cancelled request never reaches the UI.
 */
export class UpdateChecker {
    constructor() {
        this._cancellable = null;
    }

    check(currentVersion) {
        this.cancel();
        const cancellable = new Gio.Cancellable();
        this._cancellable = cancellable;
        return checkForUpdates({currentVersion, cancellable}).finally(() => {
            if (this._cancellable === cancellable)
                this._cancellable = null;
        });
    }

    cancel() {
        this._cancellable?.cancel();
    }
}
