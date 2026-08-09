import Adw from 'gi://Adw';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

import {
    buildApiKeyGroup,
    buildTranslationGroup,
} from './ui/prefsContent.js';
import {buildAboutGroup} from './ui/aboutContent.js';
import {buildShortcutsGroup} from './ui/shortcutsContent.js';
import {buildUpdatesGroup} from './ui/updatesContent.js';

function findInitialFocus(widget) {
    if (!widget)
        return null;

    if (widget instanceof Adw.ComboRow ||
        widget instanceof Adw.SwitchRow ||
        widget instanceof Adw.SpinRow)
        return widget;

    for (let child = widget.get_first_child(); child; child = child.get_next_sibling()) {
        const focusWidget = findInitialFocus(child);
        if (focusWidget)
            return focusWidget;
    }

    return null;
}

export default class LanguxPreferences extends ExtensionPreferences {
    async fillPreferencesWindow(window) {
        const settings = this.getSettings();

        const page = new Adw.PreferencesPage();
        page.add(buildTranslationGroup(settings));
        page.add(buildApiKeyGroup());
        const updatesAboutGroup = new Adw.PreferencesGroup({title: 'More'});
        buildUpdatesGroup({
            currentVersion: this.metadata['version-name'],
            window,
            group: updatesAboutGroup,
        });
        buildShortcutsGroup({settings, window, group: updatesAboutGroup});
        buildAboutGroup({metadata: this.metadata, window, group: updatesAboutGroup});
        page.add(updatesAboutGroup);
        window.add(page);

        window.set_title('Langux Settings');

        const initialFocus = findInitialFocus(page);
        if (initialFocus)
            window.set_focus(initialFocus);
    }
}
