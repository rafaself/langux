export const UPDATE_API_URL = 'https://api.github.com/repos/rafaself/langux/releases/latest';
export const UPDATE_PAGE_URL = 'https://github.com/rafaself/langux/releases/latest';

// Descriptive aliases keep the source of the update metadata explicit at call
// sites while both URLs remain fixed constants.
export const GITHUB_API_URL = UPDATE_API_URL;
export const GITHUB_RELEASE_URL = UPDATE_PAGE_URL;

export const UPDATE_STATUS = Object.freeze({
    UP_TO_DATE: 'up-to-date',
    UPDATE_AVAILABLE: 'update-available',
});

const VERSION_PATTERN = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export class UpdateInfoError extends Error {
    constructor(message) {
        super(message);
        this.name = 'UpdateInfoError';
        this.code = 'malformed';
    }
}

/**
 * Normalize the stable version format used by the extension and GitHub tags.
 * Invalid values return null so callers can choose whether to reject or fall
 * back; release and comparison helpers reject them with UpdateInfoError.
 */
export function normalizeVersion(version) {
    if (typeof version !== 'string') return null;

    const match = VERSION_PATTERN.exec(version);
    if (!match) return null;

    return `${match[1]}.${match[2]}.${match[3]}`;
}

function requireVersion(version, label) {
    const normalized = normalizeVersion(version);
    if (!normalized)
        throw new UpdateInfoError(`${label} must be a stable major.minor.patch version.`);
    return normalized;
}

/**
 * Compare two stable major.minor.patch versions.
 * Returns -1 when left is older, 0 when equal, and 1 when newer.
 */
export function compareVersions(left, right) {
    const leftParts = requireVersion(left, 'Version').split('.');
    const rightParts = requireVersion(right, 'Version').split('.');

    for (let index = 0; index < leftParts.length; index++) {
        if (leftParts[index] === rightParts[index]) continue;
        return leftParts[index].length < rightParts[index].length ||
            (leftParts[index].length === rightParts[index].length &&
                leftParts[index] < rightParts[index])
            ? -1
            : 1;
    }
    return 0;
}

export function validateReleasePayload(payload) {
    if (!payload || typeof payload !== 'object' || Array.isArray(payload))
        throw new UpdateInfoError('The release response is not an object.');
    if (payload.draft !== false || payload.prerelease !== false)
        throw new UpdateInfoError('The release is not a stable published release.');

    const version = requireVersion(payload.tag_name, 'Release tag');
    if (typeof payload.name !== 'string' || payload.name.trim().length === 0)
        throw new UpdateInfoError('The release has no title.');

    return {
        version,
        title: payload.name.trim(),
    };
}

export const parseReleasePayload = validateReleasePayload;

export function isUpdateAvailable(currentVersion, latestVersion) {
    return compareVersions(currentVersion, latestVersion) < 0;
}

export const hasUpdate = isUpdateAvailable;

export function getUpdateInfo(currentVersion, releasePayload) {
    const current = requireVersion(currentVersion, 'Current version');
    const release = validateReleasePayload(releasePayload);
    const updateAvailable = isUpdateAvailable(current, release.version);

    return {
        currentVersion: current,
        latestVersion: release.version,
        releaseTitle: release.title,
        updateAvailable,
        status: updateAvailable ? UPDATE_STATUS.UPDATE_AVAILABLE : UPDATE_STATUS.UP_TO_DATE,
        releaseUrl: UPDATE_PAGE_URL,
    };
}
