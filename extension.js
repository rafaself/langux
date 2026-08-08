import Gio from 'gi://Gio';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

const INDICATOR_ICON = 'accessories-dictionary-symbolic';

export default class LanguxExtension extends Extension {
    enable() {
        this._settings = this.getSettings();

        this._indicator = new PanelMenu.Button(0.0, 'Langux', true);
        this._indicator.add_child(new St.Icon({
            icon_name: INDICATOR_ICON,
            style_class: 'system-status-icon',
        }));
        Main.panel.addToStatusArea('langux', this._indicator, 1, 'right');
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
        this._settings = null;
    }
}
