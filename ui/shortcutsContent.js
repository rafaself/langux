import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

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

    if (!key)
        return modifiers.join('+');
    if (key.length === 1)
        return [...modifiers, key.toUpperCase()].join('+');

    const keyLabel = key
        .replace(/[_-]/g, ' ')
        .replace(/\b\w/g, letter => letter.toUpperCase());
    return [...modifiers, keyLabel].join('+');
}

function configuredOpenShortcut(settings) {
    const shortcuts = settings.get_strv('open-shortcut')
        .map(formatShortcut)
        .filter(Boolean);
    return shortcuts.length > 0 ? shortcuts.join(' or ') : 'Unassigned';
}

function showShortcutsDialog(settings, window) {
    const dialog = new Adw.MessageDialog({
        heading: 'Langux shortcuts',
        body: [
            `Open or toggle Langux: ${configuredOpenShortcut(settings)}`,
            '',
            'Translator popup:',
            '• Enter or Ctrl+Enter — Translate',
            '• Shift+Enter — Insert a new line',
            '• Escape — Close the popup',
            '• Source and target language — Choose a language',
            '• Swap — Swap languages when the source is explicit',
            '• Copy — Copy the translated text',
            '• Settings — Open Preferences',
            '• Close — Close the popup',
            '',
            'When enabled, Translate while typing translates one second after typing stops.',
        ].join('\n'),
        body_use_markup: false,
        default_response: 'close',
        close_response: 'close',
    });
    dialog.add_response('close', 'Close');
    dialog.set_transient_for(window);
    dialog.present();
}

export function buildShortcutsGroup({settings, window, group: parentGroup = null}) {
    const group = parentGroup ?? new Adw.PreferencesGroup({title: 'Shortcuts'});
    const row = new Adw.ActionRow({
        title: 'Shortcuts',
        subtitle: 'Keyboard shortcuts and translator actions',
    });
    const showButton = new Gtk.Button({
        label: 'View',
        css_classes: ['flat'],
        valign: Gtk.Align.CENTER,
    });
    row.add_suffix(showButton);
    row.activatable_widget = showButton;
    group.add(row);

    showButton.connect('clicked', () => showShortcutsDialog(settings, window));
    return group;
}
