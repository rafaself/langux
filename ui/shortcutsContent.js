import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

import {createPreferencesDialog} from './dialogContent.js';
import {createPreferenceButton} from './prefsWidgets.js';

const MODIFIER_LABELS = {
    Alt: 'Alt',
    Control: 'Ctrl',
    Ctrl: 'Ctrl',
    Meta: 'Meta',
    Primary: 'Ctrl',
    Shift: 'Shift',
    Super: 'Super',
};

function formatShortcut(shortcut) {
    const modifiers = [];
    const key = shortcut.replace(/<([^>]+)>/g, (_match, modifier) => {
        modifiers.push(MODIFIER_LABELS[modifier] ?? modifier);
        return '';
    });

    if (!key) return modifiers.join('+');
    if (key.length === 1) return [...modifiers, key.toUpperCase()].join('+');

    const keyLabel = key.replace(/[_-]/g, ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
    return [...modifiers, keyLabel].join('+');
}

function configuredOpenShortcut(settings) {
    const shortcuts = settings.get_strv('open-shortcut').map(formatShortcut).filter(Boolean);
    return shortcuts.length > 0 ? shortcuts.join(' or ') : 'Unassigned';
}

function buildShortcutRows(settings) {
    const shortcuts = [
        ['Open or toggle Langux', configuredOpenShortcut(settings)],
        ['Translate text', 'Enter or Ctrl+Enter'],
        ['Insert a new line', 'Shift+Enter'],
        ['Close the popup', 'Escape or Close'],
        ['Choose source/target language', 'Language buttons'],
        ['Swap languages', 'Swap button'],
        ['Copy translated text', 'Copy button'],
        ['Open Preferences', 'Settings button'],
        ['Translate while typing', 'After 1 second idle'],
    ];
    const group = new Adw.PreferencesGroup({title: 'Shortcuts and actions'});

    for (const [action, command] of shortcuts) {
        const row = new Adw.ActionRow({title: action});
        row.add_suffix(
            new Gtk.Label({
                label: command,
                css_classes: ['dim-label'],
                selectable: false,
                valign: Gtk.Align.CENTER,
            }),
        );
        group.add(row);
    }

    return group;
}

function showShortcutsDialog(settings, window) {
    const dialog = createPreferencesDialog({
        title: 'Langux shortcuts',
        description: 'Keyboard shortcuts and available translator actions.',
        groups: [buildShortcutRows(settings)],
    });
    dialog.present(window);
}

export function buildShortcutsGroup({settings, window, group: parentGroup = null}) {
    const group = parentGroup ?? new Adw.PreferencesGroup({title: 'Shortcuts'});
    const row = new Adw.ActionRow({
        title: 'Shortcuts',
        subtitle: 'Keyboard shortcuts and translator actions',
    });
    const showButton = createPreferenceButton('Show');
    row.add_suffix(showButton);
    row.activatable_widget = showButton;
    group.add(row);

    showButton.connect('clicked', () => showShortcutsDialog(settings, window));
    return group;
}
