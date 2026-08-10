import test from 'node:test';
import assert from 'node:assert/strict';

import {TranslationCache} from '../ui/translationCache.js';
import {TranslationController} from '../ui/translationController.js';

class FakeScheduler {
    constructor() {
        this.now = 0;
        this.nextId = 1;
        this.timers = new Map();
    }

    schedule(callback, delay) {
        const id = this.nextId++;
        this.timers.set(id, {callback, at: this.now + delay});
        return id;
    }

    cancel(id) {
        this.timers.delete(id);
    }

    advance(ms) {
        this.now += ms;
        let ran;
        do {
            ran = false;
            for (const [id, timer] of [...this.timers]) {
                if (timer.at > this.now) continue;
                this.timers.delete(id);
                timer.callback();
                ran = true;
            }
        } while (ran);
    }
}

function deferred() {
    let resolve;
    let reject;
    const promise = new Promise((res, rej) => {
        resolve = res;
        reject = rej;
    });
    return {promise, resolve, reject};
}

function setup({
    translateWhileTyping = true,
    cacheEnabled = false,
    cacheSize = 20,
    active = true,
    source = 'auto',
    target = 'en',
    translate,
} = {}) {
    const scheduler = new FakeScheduler();
    const events = [];
    const requests = [];
    const provider =
        translate ??
        ((args) => {
            requests.push(args);
            return Promise.resolve({text: `translated:${args.text}`});
        });
    const controller = new TranslationController({
        translate: provider,
        cache: new TranslationCache(cacheSize),
        cacheEnabled,
        active,
        source,
        target,
        translateWhileTyping,
        schedule: scheduler.schedule.bind(scheduler),
        cancelSchedule: scheduler.cancel.bind(scheduler),
        createCancellable: () => {
            const cancellable = {
                cancelled: false,
                cancel() {
                    this.cancelled = true;
                },
            };
            return cancellable;
        },
        onLoading: () => events.push(['loading']),
        onResult: (result) => events.push(['result', result.text]),
        onError: (error) => events.push(['error', error.code]),
        onClear: () => events.push(['clear']),
    });
    return {controller, scheduler, events, requests};
}

const settle = () => new Promise((resolve) => setImmediate(resolve));

test('typing debounces, resets the delay, and preserves exact request text', async () => {
    const {controller, scheduler, requests, events} = setup();

    controller.setText(' hello');
    scheduler.advance(900);
    assert.equal(requests.length, 0);
    controller.setText(' hello ');
    scheduler.advance(999);
    assert.equal(requests.length, 0);
    scheduler.advance(1);
    await settle();

    assert.equal(requests.length, 1);
    assert.equal(requests[0].text, ' hello ');
    assert.deepEqual(events.slice(0, 3), [['clear'], ['clear'], ['loading']]);
});

test('manual mode waits for Enter and immediate translation cancels debounce', async () => {
    const {controller, scheduler, requests} = setup({translateWhileTyping: false});

    controller.setText('manual');
    scheduler.advance(2000);
    assert.equal(requests.length, 0);
    assert.equal(controller.translateNow(), true);
    await settle();
    assert.equal(requests.length, 1);
    assert.equal(requests[0].text, 'manual');
});

test('destroy before the provider starts prevents any provider call', async () => {
    let calls = 0;
    const {controller, scheduler} = setup({
        translate: () => {
            calls++;
            return Promise.resolve({text: 'should not run'});
        },
    });

    controller.setText('destroy before start');
    scheduler.advance(1000);
    controller.destroy();
    await settle();
    await settle();

    assert.equal(calls, 0);
});

test('inactive controllers never schedule or start translations', async () => {
    const {controller, scheduler, requests} = setup({active: false});

    controller.setText('closed popup');
    scheduler.advance(2000);
    await settle();
    assert.equal(requests.length, 0);
    assert.equal(controller.translateNow(), false);

    controller.setActive(true);
    assert.equal(scheduler.timers.size, 0);
    controller.setText('open popup');
    assert.equal(scheduler.timers.size, 1);
});

test('equal source and target languages never start a translation', async () => {
    const {controller, scheduler, requests} = setup({source: 'en', target: 'en'});

    controller.setText('same language');
    assert.equal(scheduler.timers.size, 0);
    scheduler.advance(2000);
    await settle();
    assert.equal(requests.length, 0);
    assert.equal(controller.translateNow(), false);
});

test('blank input clears pending work and never reaches the provider', async () => {
    const {controller, scheduler, requests, events} = setup();

    controller.setText('text');
    controller.setText('   ');
    scheduler.advance(2000);
    await settle();

    assert.equal(requests.length, 0);
    assert.deepEqual(events, [['clear'], ['clear']]);
});

test('runtime mode changes cancel pending debounce and can enable it for current text', async () => {
    const {controller, scheduler, requests} = setup({translateWhileTyping: false});

    controller.setText('runtime');
    controller.setTranslateWhileTyping(true);
    scheduler.advance(999);
    assert.equal(requests.length, 0);
    scheduler.advance(1);
    await settle();
    assert.equal(requests.length, 1);

    controller.setTranslateWhileTyping(false);
    controller.setText('manual now');
    scheduler.advance(2000);
    assert.equal(requests.length, 1);
});

test('disabling live translation cancels an active request', async () => {
    const pending = deferred();
    const {controller, scheduler, requests} = setup({
        translate: (args) => {
            requests.push(args);
            return pending.promise;
        },
    });

    controller.setText('stop live request');
    scheduler.advance(1000);
    await settle();
    assert.equal(requests.length, 1);

    assert.equal(controller.setTranslateWhileTyping(false), true);
    assert.equal(controller.hasActiveRequest, false);
    assert.equal(requests[0].cancellable.cancelled, true);

    pending.resolve({text: 'stale'});
    await settle();
    await settle();
});

test('duplicate text changes do not add timers or duplicate requests', async () => {
    const {controller, scheduler, requests} = setup({cacheEnabled: true});

    controller.setText('same');
    controller.setText('same');
    assert.equal(scheduler.timers.size, 1);
    scheduler.advance(1000);
    await settle();
    await settle();
    assert.equal(requests.length, 1);

    controller.translateNow();
    await settle();
    assert.equal(requests.length, 1);
});

test('cache is disabled by default and successful results are not retained', async () => {
    const {controller, scheduler, requests} = setup();

    assert.equal(controller.cacheEnabled, false);
    controller.setText('private');
    scheduler.advance(1000);
    await settle();
    await settle();

    controller.setText('other');
    controller.setText('private');
    scheduler.advance(1000);
    await settle();
    await settle();

    assert.equal(requests.length, 2);
    assert.equal(controller.cache.get('auto', 'en', 'private'), undefined);
});

test('enabling the cache at runtime stores and reuses later results', async () => {
    const {controller, scheduler, requests} = setup();

    controller.setText('not cached');
    scheduler.advance(1000);
    await settle();
    await settle();

    controller.setCacheEnabled(true);
    assert.equal(controller.cacheEnabled, true);
    controller.setText('cached');
    scheduler.advance(1000);
    await settle();
    await settle();
    controller.setText('other');
    controller.setText('cached');
    scheduler.advance(1000);
    await settle();
    await settle();

    assert.equal(requests.length, 2);
    assert.equal(controller.cache.get('auto', 'en', 'cached').text, 'translated:cached');
});

test('disabling the cache clears entries and blocks an in-flight write', async () => {
    const pending = deferred();
    const {controller, scheduler} = setup({
        cacheEnabled: true,
        translate: () => pending.promise,
    });
    controller.cache.set('auto', 'en', 'existing', {text: 'old'});

    controller.setText('in flight');
    scheduler.advance(1000);
    await settle();
    controller.setCacheEnabled(false);
    assert.equal(controller.cacheEnabled, false);
    assert.equal(controller.cache.size, 0);

    pending.resolve({text: 'must not cache'});
    await settle();
    await settle();
    assert.equal(controller.cache.get('auto', 'en', 'in flight'), undefined);
});

test('cache hits avoid a second provider request', async () => {
    const {controller, scheduler, requests, events} = setup({cacheEnabled: true});

    controller.setText('cached');
    scheduler.advance(1000);
    await settle();
    await settle();
    controller.setText('different');
    controller.setText('cached');
    scheduler.advance(1000);
    await settle();
    await settle();

    assert.equal(requests.length, 1);
    assert.equal(events.filter((event) => event[0] === 'result').length, 2);
});

test('Enter-style immediate translation cancels an existing debounce', async () => {
    const {controller, scheduler, requests} = setup();

    controller.setText('enter');
    assert.equal(controller.translateNow(), true);
    scheduler.advance(1000);
    await settle();
    await settle();
    assert.equal(requests.length, 1);
});

test('stale async responses cannot update output or cache', async () => {
    const first = deferred();
    const second = deferred();
    const {controller, scheduler, events, requests} = setup({
        cacheEnabled: true,
        translate: (args) => {
            requests.push(args);
            return requests.length === 1 ? first.promise : second.promise;
        },
    });

    controller.setText('first');
    scheduler.advance(1000);
    await settle();
    controller.setText('second');
    controller.translateNow();
    await settle();

    first.resolve({text: 'old'});
    await settle();
    assert.equal(
        events.some((event) => event[1] === 'old'),
        false,
    );
    assert.equal(controller.cache.get('auto', 'en', 'first'), undefined);

    second.resolve({text: 'new'});
    await settle();
    assert.equal(
        events.some((event) => event[1] === 'new'),
        true,
    );
    assert.equal(controller.cache.get('auto', 'en', 'second').text, 'new');
});

test('failed requests are not cached and can be retried', async () => {
    let calls = 0;
    const {controller, scheduler, requests} = setup({
        cacheEnabled: true,
        translate: (args) => {
            requests.push(args);
            calls++;
            return calls === 1
                ? Promise.reject(Object.assign(new Error('offline'), {code: 'network'}))
                : Promise.resolve({text: 'retry'});
        },
    });

    controller.setText('retryable');
    scheduler.advance(1000);
    await settle();
    await settle();
    assert.equal(controller.cache.get('auto', 'en', 'retryable'), undefined);

    controller.translateNow();
    await settle();
    await settle();
    assert.equal(requests.length, 2);
});

test('clearing during a request prevents that response from repopulating cache', async () => {
    const pending = deferred();
    const {controller, scheduler, requests} = setup({
        cacheEnabled: true,
        translate: (args) => {
            requests.push(args);
            return pending.promise;
        },
    });

    controller.setText('in flight');
    scheduler.advance(1000);
    await settle();
    controller.clearCache();
    pending.resolve({text: 'not cached after clear'});
    await settle();
    await settle();

    assert.equal(controller.cache.get('auto', 'en', 'in flight'), undefined);
});

test('context changes cancel requests without automatically retrying', async () => {
    const pending = deferred();
    const {controller, scheduler, requests, events} = setup({
        translate: (args) => {
            requests.push(args);
            return pending.promise;
        },
    });

    controller.setText('context');
    scheduler.advance(1000);
    await settle();
    controller.setContext('pt', 'en');
    pending.resolve({text: 'stale'});
    await settle();
    assert.equal(requests.length, 1);
    assert.equal(
        events.some((event) => event[1] === 'stale'),
        false,
    );
    assert.equal(controller.hasPendingTranslation, false);
});

test('destroy cancels pending work and clears the cache', async () => {
    const {controller, scheduler, requests} = setup();
    controller.setText('destroy me');
    controller.destroy();
    scheduler.advance(2000);
    await settle();

    assert.equal(requests.length, 0);
    assert.equal(controller.cache.size, 0);
    assert.equal(controller.translateNow(), false);
});

test('cancelWork cancels active requests and ignores their results', async () => {
    const pending = deferred();
    const {controller, scheduler, requests, events} = setup({
        translate: (args) => {
            requests.push(args);
            return pending.promise;
        },
    });

    controller.setText('close popup');
    scheduler.advance(1000);
    await settle();
    assert.equal(requests.length, 1);

    assert.equal(controller.cancelWork(), true);
    assert.equal(controller.hasActiveRequest, false);
    assert.equal(requests[0].cancellable.cancelled, true);

    pending.resolve({text: 'must be ignored'});
    await settle();
    await settle();
    assert.equal(
        events.some((event) => event[1] === 'must be ignored'),
        false,
    );
});

test('deactivating cancels active requests and blocks later triggers', async () => {
    const pending = deferred();
    const {controller, scheduler, requests} = setup({
        translate: (args) => {
            requests.push(args);
            return pending.promise;
        },
    });

    controller.setText('toggle off');
    scheduler.advance(1000);
    await settle();
    assert.equal(requests.length, 1);

    assert.equal(controller.setActive(false), true);
    assert.equal(controller.active, false);
    assert.equal(controller.hasActiveRequest, false);
    assert.equal(requests[0].cancellable.cancelled, true);
    assert.equal(controller.translateNow(), false);

    pending.resolve({text: 'must be ignored'});
    await settle();
    await settle();
    assert.equal(requests.length, 1);
});
