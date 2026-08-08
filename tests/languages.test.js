import {test} from 'node:test';
import assert from 'node:assert/strict';

import {
    AUTO_LANGUAGE,
    LANGUAGES,
    hasLanguage,
    isExplicit,
    languageLabel,
    swapLanguages,
} from '../ui/languages.js';

test('target languages match the issue list, excluding auto', () => {
    assert.deepEqual(LANGUAGES.map(l => l.code), [
        'en', 'pt', 'es', 'ja', 'fr', 'de', 'it', 'ko', 'zh',
    ]);
    assert.ok(LANGUAGES.every(l => l.code !== AUTO_LANGUAGE));
    assert.ok(LANGUAGES.every(l => l.label.length > 0));
});

test('auto is the only non-explicit source', () => {
    assert.equal(AUTO_LANGUAGE, 'auto');
    assert.equal(isExplicit(AUTO_LANGUAGE), false);
});

test('every target language is known and has a label', () => {
    for (const {code, label} of LANGUAGES) {
        assert.equal(hasLanguage(code), true);
        assert.equal(languageLabel(code), label);
    }
});

test('GSettings defaults are valid language values', () => {
    assert.equal(languageLabel('auto'), 'Auto detect');
    assert.equal(isExplicit('en'), true); // schema default target-language
});

test('swap is allowed only when the source is explicit', () => {
    assert.deepEqual(swapLanguages('en', 'pt'), {source: 'pt', target: 'en'});
    assert.deepEqual(swapLanguages('en', 'en'), {source: 'en', target: 'en'});
    assert.equal(swapLanguages(AUTO_LANGUAGE, 'en'), null);
});

test('unknown codes are not exposed and fall back to the code itself', () => {
    assert.equal(isExplicit('xx'), false);
    assert.equal(hasLanguage('xx'), false);
    assert.equal(languageLabel('xx'), 'xx');
});