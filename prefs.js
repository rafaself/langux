import Adw from 'gi://Adw';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

import {buildApiKeyGroup, buildTranslationGroup} from './ui/prefsContent.js';

export default class LanguxPreferences extends ExtensionPreferences {
    async fillPreferencesWindow(window) {
        const settings = this.getSettings();

        const page = new Adw.PreferencesPage();
        page.add(buildTranslationGroup(settings));
        page.add(buildApiKeyGroup());
        window.add(page);

        window.set_title('Langux Settings');
    }
}