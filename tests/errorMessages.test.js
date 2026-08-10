import test from 'node:test';
import assert from 'node:assert/strict';

import {friendlyMessage, needsSettingsAction} from '../ui/errorMessages.js';

test('maps every translator error code to the spec-friendly message', () => {
    assert.equal(friendlyMessage('missing-credential'), "Google API key isn't configured.");
    assert.equal(friendlyMessage('unauthorized'), 'Google rejected the API key.');
    assert.equal(friendlyMessage('network'), 'Unable to reach Google Translation.');
    assert.equal(friendlyMessage('quota'), 'Google Translation quota exceeded.');
    assert.equal(friendlyMessage('server'), 'Google Translation is temporarily unavailable.');
    assert.equal(friendlyMessage('malformed'), 'Translation failed.');
});

test('known codes never leak raw messages or payloads', () => {
    for (const code of [
        'missing-credential',
        'unauthorized',
        'network',
        'quota',
        'server',
        'malformed',
    ])
        assert.doesNotMatch(friendlyMessage(code), /(error|message|null|undefined|status)/i);
});

test('unknown or cancelled codes fall back to the generic message', () => {
    assert.equal(friendlyMessage('cancelled'), 'Translation failed.');
    assert.equal(friendlyMessage('what-even'), 'Translation failed.');
    assert.equal(friendlyMessage(undefined), 'Translation failed.');
    assert.equal(friendlyMessage(null, 'custom fallback'), 'custom fallback');
});

test('missing credentials and rejected keys offer the settings path', () => {
    for (const code of ['missing-credential', 'unauthorized'])
        assert.equal(needsSettingsAction(code), true);
    for (const code of ['network', 'quota', 'server', 'malformed', 'cancelled'])
        assert.equal(needsSettingsAction(code), false);
});
