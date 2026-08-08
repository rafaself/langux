import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

import {TranslatorPopup} from './ui/translatorPopup.js';

const INDICATOR_ICON = 'accessories-dictionary-symbolic';
const SHORTCUT_BINDING = 'open-shortcut';
const MOCK_TRANSLATE_DELAY_MS = 500;

export default class LanguxExtension extends Extension {
    enable() {
        this._settings = this.getSettings();

        this._indicator = new PanelMenu.Button(0.0, 'Langux', true);
        this._indicator.add_child(new St.Icon({
            icon_name: INDICATOR_ICON,
            style_class: 'system-status-icon',
        }));

        this._popup = new TranslatorPopup(this._indicator, this._settings);
        this._popup.onTranslate = (text) => this._mockTranslate(text);

        this._indicator.setMenu(this._popup);
        Main.panel.addToStatusArea('langux', this._indicator, 1, 'right');

        Main.wm.addKeybinding(
            SHORTCUT_BINDING,
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW | Shell.ActionMode.POPUP,
            () => this._popup.toggle());
    }

    disable() {
        if (this._translateTimeoutId) {
            GLib.source_remove(this._translateTimeoutId);
            this._translateTimeoutId = null;
        }

        Main.wm.removeKeybinding(SHORTCUT_BINDING);

        this._indicator?.destroy();
        this._indicator = null;
        this._popup = null;
        this._settings = null;
    }

    _mockTranslate(text) {
        this._popup.setLoading();

        this._translateTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, MOCK_TRANSLATE_DELAY_MS, () => {
            this._translateTimeoutId = null;
            if (this._popup)
                this._popup.setResult(`(mock) ${text}`);
            return GLib.SOURCE_REMOVE;
        });
    }
}