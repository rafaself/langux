import Adw from 'gi://Adw';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

import {
    buildApiKeyGroup,
    buildTranslationGroup,
} from './ui/prefsContent.js';
import {buildAboutGroup} from './ui/aboutContent.js';
import {buildUpdatesGroup} from './ui/updatesContent.js';

export default class LanguxPreferences extends ExtensionPreferences {
    async fillPreferencesWindow(window) {
        const settings = this.getSettings();

        const page = new Adw.PreferencesPage();
        page.add(buildTranslationGroup(settings));
        page.add(buildApiKeyGroup());
        page.add(buildUpdatesGroup({
            currentVersion: this.metadata['version-name'],
            window,
        }));
        page.add(buildAboutGroup({metadata: this.metadata, window}));
        window.add(page);

        window.set_title('Langux Settings');
    }
}
