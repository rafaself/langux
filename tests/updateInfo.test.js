import test from 'node:test';
import assert from 'node:assert/strict';

import {
    UPDATE_API_URL,
    UPDATE_PAGE_URL,
    UPDATE_STATUS,
    compareVersions,
    getUpdateInfo,
    normalizeVersion,
    validateReleasePayload,
} from '../ui/updateInfo.js';

const release = (tagName = 'v0.2.0', overrides = {}) => ({
    tag_name: tagName,
    name: 'Langux 0.2.0',
    draft: false,
    prerelease: false,
    ...overrides,
});

test('normalizes stable versions with or without a leading v', () => {
    assert.equal(normalizeVersion('v0.1.0'), '0.1.0');
    assert.equal(normalizeVersion('0.1.0'), '0.1.0');
    assert.equal(normalizeVersion('v12.3.40'), '12.3.40');
});

test('compares equal, older, and newer stable versions', () => {
    assert.equal(compareVersions('0.1.0', 'v0.1.0'), 0);
    assert.equal(compareVersions('0.1.0', '0.2.0'), -1);
    assert.equal(compareVersions('v1.2.0', '1.1.9'), 1);
});

test('rejects malformed versions instead of guessing a release order', () => {
    for (const version of ['', 'v1.2', '1.2.3-beta', '1.2.03', '1.2.3.4', null])
        assert.equal(normalizeVersion(version), null);

    assert.throws(() => compareVersions('0.1', '0.2.0'), /version/i);
    assert.throws(() => getUpdateInfo('v0.1', release()), /version/i);
});

test('validates required published stable release fields', () => {
    assert.deepEqual(validateReleasePayload(release()), {
        version: '0.2.0',
        title: 'Langux 0.2.0',
    });

    for (const payload of [
        {},
        release('not-a-version'),
        release('v0.2.0', {name: ''}),
        release('v0.2.0', {draft: true}),
        release('v0.2.0', {prerelease: true}),
        release('v0.2.0', {draft: undefined}),
    ]) {
        assert.throws(() => validateReleasePayload(payload), /release|stable|title|version/i);
    }
});

test('reports update status for equal, older, and newer releases', () => {
    const equal = getUpdateInfo('0.2.0', release('v0.2.0'));
    assert.equal(equal.status, UPDATE_STATUS.UP_TO_DATE);
    assert.equal(equal.updateAvailable, false);

    const older = getUpdateInfo('0.3.0', release('v0.2.0'));
    assert.equal(older.status, UPDATE_STATUS.UP_TO_DATE);
    assert.equal(older.updateAvailable, false);

    const newer = getUpdateInfo('0.1.0', release('v0.2.0'));
    assert.equal(newer.status, UPDATE_STATUS.UPDATE_AVAILABLE);
    assert.equal(newer.updateAvailable, true);
    assert.equal(newer.currentVersion, '0.1.0');
    assert.equal(newer.latestVersion, '0.2.0');
    assert.equal(newer.releaseTitle, 'Langux 0.2.0');
    assert.equal(newer.releaseUrl, UPDATE_PAGE_URL);
});

test('uses the fixed GitHub metadata and release page endpoints', () => {
    assert.equal(UPDATE_API_URL, 'https://api.github.com/repos/rafaself/langux/releases/latest');
    assert.equal(UPDATE_PAGE_URL, 'https://github.com/rafaself/langux/releases/latest');
});
