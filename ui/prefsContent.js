import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

import {clearCacheOverSessionBus} from '../services/cacheControl.js';
import {SecretStore} from '../services/secretStore.js';
import {AUTO_LANGUAGE, LANGUAGES, languageLabel} from './languages.js';
import {createPreferenceButton} from './prefsWidgets.js';

const SOURCE_CODES = [AUTO_LANGUAGE, ...LANGUAGES.map((l) => l.code)];
const TARGET_CODES = LANGUAGES.map((l) => l.code);
const CACHE_SIZE_OPTIONS = [
    {value: 0, label: '0'},
    {value: 50, label: '50'},
    {value: 100, label: '100'},
    {value: 200, label: '200'},
    {value: 500, label: '500'},
    {value: 1000, label: '1000'},
];

export function buildTranslationGroup(settings) {
    const group = new Adw.PreferencesGroup({title: 'Translation'});

    const sourceRow = _buildLanguageRow(
        'Default source',
        SOURCE_CODES,
        settings.get_string('source-language'),
    );
    sourceRow.connect('notify::selected', () => {
        settings.set_string('source-language', SOURCE_CODES[sourceRow.selected]);
    });

    const targetRow = _buildLanguageRow(
        'Default target',
        TARGET_CODES,
        settings.get_string('target-language'),
    );
    targetRow.connect('notify::selected', () => {
        settings.set_string('target-language', TARGET_CODES[targetRow.selected]);
    });

    group.add(sourceRow);
    group.add(targetRow);

    const liveRow = new Adw.SwitchRow({
        title: 'Translate while typing',
        subtitle: 'Translate one second after the text stops changing',
        active: settings.get_boolean('translate-while-typing'),
    });
    liveRow.connect('notify::active', () => {
        settings.set_boolean('translate-while-typing', liveRow.active);
    });
    group.add(liveRow);

    const cacheEnabledRow = new Adw.SwitchRow({
        title: 'Enable translation cache',
        subtitle: 'Reuse successful translations from memory during this Shell session',
        active: settings.get_boolean('translation-cache-enabled'),
    });
    cacheEnabledRow.connect('notify::active', () => {
        settings.set_boolean('translation-cache-enabled', cacheEnabledRow.active);
    });
    group.add(cacheEnabledRow);

    const cacheRow = _buildCacheSizeRow(settings);
    group.add(cacheRow);

    const clearRow = new Adw.ActionRow({
        title: 'Clear translation cache',
        subtitle: 'Remove successful translations from the running Shell session',
    });
    const clearButton = createPreferenceButton('Clear');
    clearButton.connect('clicked', () => {
        try {
            clearCacheOverSessionBus();
            clearRow.subtitle = 'Cache cleared';
        } catch (error) {
            console.error(`Failed to clear the translation cache: ${error?.message ?? error}`);
            clearRow.subtitle = 'No active cache to clear';
        }
    });
    clearRow.add_suffix(clearButton);
    group.add(clearRow);

    return group;
}

export function buildApiKeyGroup() {
    const group = new Adw.PreferencesGroup({title: 'Google Cloud'});
    const row = new Adw.ActionRow({title: 'API Key'});

    const configureButton = createPreferenceButton('Configure');
    const replaceButton = createPreferenceButton('Replace');
    const removeButton = createPreferenceButton('Remove');

    const buttons = new Gtk.Box({
        orientation: Gtk.Orientation.HORIZONTAL,
        spacing: 8,
        valign: Gtk.Align.CENTER,
    });
    buttons.append(configureButton);
    buttons.append(replaceButton);
    buttons.append(removeButton);
    row.add_suffix(buttons);

    function refresh() {
        SecretStore.hasApiKey()
            .then((has) => {
                row.subtitle = has ? 'Configured ✓' : 'Not configured';
                configureButton.visible = !has;
                replaceButton.visible = has;
                removeButton.visible = has;
            })
            .catch(() => {
                console.error('Failed to query the keyring; treating the key as unconfigured.');
                row.subtitle = 'Not configured';
                configureButton.visible = true;
                replaceButton.visible = false;
                removeButton.visible = false;
            });
    }

    function dialogParent() {
        const root = row.get_root();
        return root instanceof Gtk.Window ? root : null;
    }

    function showKeyDialog() {
        const entry = new Gtk.PasswordEntry({
            placeholder_text: 'Paste your Google Cloud API key',
            show_peek_icon: true,
        });

        const dialog = new Adw.MessageDialog({
            heading: 'Google Cloud API key',
            body: 'The key is stored in GNOME Keyring and is never displayed again.',
            default_response: 'save',
            close_response: 'cancel',
        });
        if (dialogParent()) dialog.set_transient_for(dialogParent());
        dialog.set_extra_child(entry);
        dialog.add_response('cancel', 'Cancel');
        dialog.add_response('save', 'Save');
        dialog.set_response_enabled('save', false);
        entry.connect('notify::text', () => {
            dialog.set_response_enabled('save', entry.get_text().trim().length > 0);
        });
        dialog.connect('response', (_dialog, response) => {
            if (response !== 'save') return;

            const key = entry.get_text().trim();
            if (!key) return;

            SecretStore.saveApiKey(key).then(refresh).catch(notifyError);
        });
        dialog.present();
    }

    function confirmRemoveDialog() {
        const dialog = new Adw.MessageDialog({
            heading: 'Remove API key?',
            body: 'The Google Cloud API key will be removed from GNOME Keyring.',
            default_response: 'cancel',
            close_response: 'cancel',
        });
        if (dialogParent()) dialog.set_transient_for(dialogParent());
        dialog.add_response('cancel', 'Cancel');
        dialog.add_response('remove', 'Remove');
        dialog.set_response_appearance('remove', Adw.ResponseAppearance.DESTRUCTIVE);
        dialog.connect('response', (_dialog, response) => {
            if (response !== 'remove') return;
            SecretStore.deleteApiKey().then(refresh).catch(notifyError);
        });
        dialog.present();
    }

    function notifyError(error) {
        console.error(`Failed to update the Google API key: ${error?.message ?? error}`);
        const dialog = new Adw.MessageDialog({
            heading: 'Could not update the API key',
            body: 'Accessing the keyring failed. See the journal for details.',
            close_response: 'close',
        });
        if (dialogParent()) dialog.set_transient_for(dialogParent());
        dialog.add_response('close', 'Close');
        dialog.present();
    }

    configureButton.connect('clicked', showKeyDialog);
    replaceButton.connect('clicked', showKeyDialog);
    removeButton.connect('clicked', confirmRemoveDialog);

    refresh();
    group.add(row);
    return group;
}

function _buildLanguageRow(title, codes, selectedCode) {
    const row = new Adw.ComboRow({title});

    const model = new Gtk.StringList();
    for (const code of codes) model.append(languageLabel(code));
    row.model = model;

    const index = codes.indexOf(selectedCode);
    row.selected = index >= 0 ? index : 0;
    return row;
}

function _buildCacheSizeRow(settings) {
    const currentSize = settings.get_int('translation-cache-size');
    const options = CACHE_SIZE_OPTIONS.some((option) => option.value === currentSize)
        ? CACHE_SIZE_OPTIONS
        : [
              ...CACHE_SIZE_OPTIONS.slice(
                  0,
                  CACHE_SIZE_OPTIONS.findIndex((option) => option.value > currentSize),
              ),
              {value: currentSize, label: `${currentSize} (Current)`},
              ...CACHE_SIZE_OPTIONS.slice(
                  CACHE_SIZE_OPTIONS.findIndex((option) => option.value > currentSize),
              ),
          ];
    const row = new Adw.ComboRow({
        title: 'Translation cache size',
        subtitle: 'Maximum successful translations kept when caching is enabled',
    });
    const model = new Gtk.StringList();
    for (const option of options) model.append(option.label);
    row.model = model;
    row.selected = options.findIndex((option) => option.value === currentSize);
    row.connect('notify::selected', () => {
        settings.set_int('translation-cache-size', options[row.selected].value);
    });
    return row;
}
