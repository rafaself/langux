import test from 'node:test';
import assert from 'node:assert/strict';

import {TranslationCache, translationCacheKey} from '../ui/translationCache.js';

const result = (text) => ({text});

test('cache keys include source, target, and exact raw text', () => {
    const cache = new TranslationCache(4);
    cache.set('en', 'pt', 'hello', result('olá'));
    cache.set('pt', 'pt', 'hello', result('hello'));
    cache.set('en', 'en', 'hello', result('hello'));
    cache.set('en', 'pt', ' hello', result(' olá'));

    assert.equal(cache.get('en', 'pt', 'hello').text, 'olá');
    assert.equal(cache.get('pt', 'pt', 'hello').text, 'hello');
    assert.equal(cache.get('en', 'en', 'hello').text, 'hello');
    assert.equal(cache.get('en', 'pt', ' hello').text, ' olá');
    assert.notEqual(
        translationCacheKey('en', 'pt', 'hello'),
        translationCacheKey('en', 'pt', ' hello'),
    );
});

test('cache refreshes recency on reads and evicts the least recent entry', () => {
    const cache = new TranslationCache(2);
    cache.set('auto', 'en', 'first', result('1'));
    cache.set('auto', 'en', 'second', result('2'));
    assert.equal(cache.get('auto', 'en', 'first').text, '1');
    cache.set('auto', 'en', 'third', result('3'));

    assert.equal(cache.get('auto', 'en', 'first').text, '1');
    assert.equal(cache.get('auto', 'en', 'second'), undefined);
    assert.equal(cache.get('auto', 'en', 'third').text, '3');
});

test('resizing evicts immediately and zero capacity disables caching', () => {
    const cache = new TranslationCache(3);
    cache.set('auto', 'en', 'one', result('1'));
    cache.set('auto', 'en', 'two', result('2'));
    cache.set('auto', 'en', 'three', result('3'));

    cache.resize(1);
    assert.equal(cache.size, 1);
    assert.equal(cache.get('auto', 'en', 'three').text, '3');
    assert.equal(cache.get('auto', 'en', 'one'), undefined);

    cache.resize(0);
    assert.equal(cache.size, 0);
    cache.set('auto', 'en', 'four', result('4'));
    assert.equal(cache.get('auto', 'en', 'four'), undefined);
});

test('invalid capacities are rejected', () => {
    assert.throws(() => new TranslationCache(-1), RangeError);
    assert.throws(() => new TranslationCache(1.5), RangeError);
});

test('clear removes all entries', () => {
    const cache = new TranslationCache(2);
    cache.set('auto', 'en', 'text', result('result'));
    cache.clear();
    assert.equal(cache.size, 0);
    assert.equal(cache.has('auto', 'en', 'text'), false);
});
