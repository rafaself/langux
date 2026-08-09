import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

import {clearCacheOverSessionBus} from '../services/cacheControl.js';
import {SecretStore} from '../services/secretStore.js';
import {AUTO_LANGUAGE, LANGUAGES, languageLabel} from './languages.js';

const SOURCE_CODES = [AUTO_LANGUAGE, ...LANGUAGES.map(l => l.code)];
const TARGET_CODES = LANGUAGES.map(l => l.code);

export function buildTranslationGroup(settings) {
    const group = new Adw.PreferencesGroup({title: 'Translation'});

    const sourceRow = _buildLanguageRow(
        'Default source',
        SOURCE_CODES,
        settings.get_string('source-language'));
    sourceRow.connect('notify::selected', () => {
        settings.set_string('source-language', SOURCE_CODES[sourceRow.selected]);
    });

    const targetRow = _buildLanguageRow(
        'Default target',
        TARGET_CODES,
        settings.get_string('target-language'));
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

    const cacheAdjustment = new Gtk.Adjustment({
        lower: 0,
        upper: 1000,
        step_increment: 1,
        page_increment: 10,
        value: settings.get_int('translation-cache-size'),
    });
    const cacheRow = new Adw.SpinRow({
        title: 'Translation cache size',
        subtitle: 'Maximum successful translations kept when caching is enabled; zero disables it',
        adjustment: cacheAdjustment,
    });
    cacheRow.connect('notify::value', () => {
        settings.set_int('translation-cache-size', Math.round(cacheRow.value));
    });
    group.add(cacheRow);

    const clearRow = new Adw.ActionRow({
        title: 'Clear translation cache',
        subtitle: 'Remove successful translations from the running Shell session',
    });
    const clearButton = new Gtk.Button({
        label: 'Clear',
        valign: Gtk.Align.CENTER,
    });
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

    const configureButton = new Gtk.Button({label: 'Configure', css_classes: ['flat']});
    const replaceButton = new Gtk.Button({label: 'Replace', css_classes: ['flat']});
    const removeButton = new Gtk.Button({label: 'Remove', css_classes: ['flat']});

    const buttons = new Gtk.Box({orientation: Gtk.Orientation.HORIZONTAL, spacing: 6});
    buttons.append(configureButton);
    buttons.append(replaceButton);
    buttons.append(removeButton);
    row.add_suffix(buttons);

    function refresh() {
        SecretStore.hasApiKey()
            .then(has => {
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
        if (dialogParent())
            dialog.set_transient_for(dialogParent());
        dialog.set_extra_child(entry);
        dialog.add_response('cancel', 'Cancel');
        dialog.add_response('save', 'Save');
        dialog.set_response_enabled('save', false);
        entry.connect('notify::text', () => {
            dialog.set_response_enabled('save', entry.get_text().trim().length > 0);
        });
        dialog.connect('response', (dialog_, response) => {
            if (response !== 'save')
                return;

            const key = entry.get_text().trim();
            if (!key)
                return;

            SecretStore.saveApiKey(key)
                .then(refresh)
                .catch(notifyError);
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
        if (dialogParent())
            dialog.set_transient_for(dialogParent());
        dialog.add_response('cancel', 'Cancel');
        dialog.add_response('remove', 'Remove');
        dialog.set_response_appearance('remove', Adw.ResponseAppearance.DESTRUCTIVE);
        dialog.connect('response', (dialog_, response) => {
            if (response !== 'remove')
                return;
            SecretStore.deleteApiKey()
                .then(refresh)
                .catch(notifyError);
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
        if (dialogParent())
            dialog.set_transient_for(dialogParent());
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
    for (const code of codes)
        model.append(languageLabel(code));
    row.model = model;

    const index = codes.indexOf(selectedCode);
    row.selected = index >= 0 ? index : 0;
    return row;
}
