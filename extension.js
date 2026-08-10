import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';

import {createCacheControlObject, CACHE_BUS_NAME, CACHE_OBJECT_PATH} from './services/cacheControl.js';
import {TranslatorPopup} from './ui/translatorPopup.js';

const SHORTCUT_BINDING = 'open-shortcut';
const ICON_LIGHT_UI = 'data/icon.svg';
const ICON_DARK_UI = 'data/icon-light.svg';

export default class LanguxExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._colorSchemeChangedId = null;
        this._cacheBusOwnerId = null;
        this._cacheBusConnection = null;
        this._cacheControlObject = null;
        this._cacheControlExported = false;

        this._indicator = new PanelMenu.Button(0.0, 'Langux', true);
        this._icon = new St.Icon({
            gicon: this._iconForColorScheme(),
            style_class: 'system-status-icon langux-panel-icon',
        });
        this._indicator.add_child(this._icon);

        this._colorSchemeChangedId = St.Settings.get().connect(
            'notify::color-scheme',
            () => { this._icon.gicon = this._iconForColorScheme(); });

        this._popup = new TranslatorPopup(this._indicator, this._settings);
        this._popup.onOpenSettings = () => this.openPreferences();

        this._indicator.setMenu(this._popup);
        Main.panel.addToStatusArea('langux', this._indicator, 1, 'right');

        this._exportCacheControl();

        Main.wm.addKeybinding(
            SHORTCUT_BINDING,
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW | Shell.ActionMode.POPUP,
            () => this._popup.toggle());
    }

    disable() {
        Main.wm.removeKeybinding(SHORTCUT_BINDING);

        this._destroyCacheControl();

        if (this._colorSchemeChangedId !== null) {
            St.Settings.get().disconnect(this._colorSchemeChangedId);
            this._colorSchemeChangedId = null;
        }

        this._popup?.destroy();
        this._indicator?.destroy();
        this._icon = null;
        this._indicator = null;
        this._popup = null;
        this._settings = null;
    }

    _exportCacheControl() {
        this._cacheControlObject = createCacheControlObject(
            () => this._popup?.clearCache());
        this._cacheBusOwnerId = Gio.bus_own_name(
            Gio.BusType.SESSION,
            CACHE_BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            connection => {
                if (!this._cacheControlObject || !this._popup)
                    return;
                this._cacheBusConnection = connection;
                this._cacheControlObject.export(connection, CACHE_OBJECT_PATH);
                this._cacheControlExported = true;
            },
            null,
            () => this._unexportCacheControl());
    }

    _unexportCacheControl() {
        if (this._cacheControlExported && this._cacheControlObject) {
            this._cacheControlObject.unexport();
            this._cacheControlExported = false;
        }
        this._cacheBusConnection = null;
    }

    _destroyCacheControl() {
        if (this._cacheBusOwnerId !== null) {
            Gio.bus_unown_name(this._cacheBusOwnerId);
            this._cacheBusOwnerId = null;
        }
        this._unexportCacheControl();
        this._cacheControlObject = null;
    }

    _iconForColorScheme() {
        const name = Main.getStyleVariant() === 'dark' ? ICON_DARK_UI : ICON_LIGHT_UI;
        return Gio.FileIcon.new(
            Gio.File.new_for_path(`${this.path}/${name}`));
    }
}
