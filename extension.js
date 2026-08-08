import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

import {TranslatorPopup} from './ui/translatorPopup.js';

const SHORTCUT_BINDING = 'open-shortcut';

export default class LanguxExtension extends Extension {
    enable() {
        this._settings = this.getSettings();

        this._indicator = new PanelMenu.Button(0.0, 'Langux', true);
        this._indicator.add_child(new St.Icon({
            gicon: Gio.FileIcon.new(
                Gio.File.new_for_path(`${this.path}/data/icon.svg`)),
            style_class: 'system-status-icon',
        }));

        this._popup = new TranslatorPopup(this._indicator, this._settings);
        this._popup.onOpenSettings = () => this.openPreferences();

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
        Main.wm.removeKeybinding(SHORTCUT_BINDING);

        this._indicator?.destroy();
        this._indicator = null;
        this._popup = null;
        this._settings = null;
    }
}