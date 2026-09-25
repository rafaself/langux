import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Pango from 'gi://Pango';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

const APPLICATION_ID = 'io.github.rafaself.Langux';
const APPLICATION_DESKTOP_ID = `${APPLICATION_ID}.desktop`;
const APPLICATION_BUS_NAME = APPLICATION_ID;
const APPLICATION_OBJECT_PATH = '/io/github/rafaself/Langux/GnomeShellAdapter';
const APPLICATION_INTERFACE = 'io.github.rafaself.Langux.GnomeShellAdapter';
const REFRESH_INTERVAL_MS = 250;
const MAX_INPUT_LENGTH = 4096;

const STATE_REPLY_TYPE = new GLib.VariantType('((ssssssbss))');
const LANGUAGES_REPLY_TYPE = new GLib.VariantType('(a(ss))');
const BOOL_REPLY_TYPE = new GLib.VariantType('(b)');

export default class LanguxExtension extends Extension {
    enable() {
        this.loadStylesheet();
        this._languages = [['auto', 'Auto detect']];
        this._languagesLoaded = false;
        this._languagesRequestPending = false;
        this._snapshot = null;
        this._lastLaunchTime = 0;
        this._refreshSource = 0;
        this._languagesRetryAfter = 0;
        this._languageMenus = [];
        this._popupManager = null;

        this._indicator = new PanelMenu.Button(0.5, 'Langux', false);
        this._indicator.add_style_class_name('langux-panel-indicator');
        this._popupManager = new PopupMenu.PopupMenuManager(this._indicator);

        const iconFile = this.dir.get_child('icons').get_child('langux.svg');
        this._indicator.add_child(new St.Icon({
            gicon: Gio.FileIcon.new(iconFile),
            style_class: 'system-status-icon',
        }));

        this._buildPopup();
        this._buildContextMenu();
        Main.panel.addToStatusArea(this.uuid, this._indicator);

        this._indicator.menu.connect('open-state-changed', (_menu, isOpen) => {
            if (isOpen) {
                this._entry.grab_key_focus();
                this._launchApp();
                this._refresh();
                this._refreshSource = GLib.timeout_add(
                    GLib.PRIORITY_DEFAULT,
                    REFRESH_INTERVAL_MS,
                    () => {
                        if (!this._indicator?.menu.isOpen) {
                            this._refreshSource = 0;
                            return GLib.SOURCE_REMOVE;
                        }
                        this._refresh();
                        return GLib.SOURCE_CONTINUE;
                    });
            } else if (this._refreshSource !== 0) {
                GLib.source_remove(this._refreshSource);
                this._refreshSource = 0;
                for (const menu of this._languageMenus) {
                    menu.close();
                }
                this._contextMenu?.close();
            }
        });

        this._indicator.connect('button-press-event', (_actor, event) => {
            if (event.get_button() !== Clutter.BUTTON_SECONDARY)
                return Clutter.EVENT_PROPAGATE;

            this._indicator.menu.close();
            this._contextMenu.open();
            return Clutter.EVENT_STOP;
        });

        this._appOwnerSubscription = Gio.DBus.session.signal_subscribe(
            'org.freedesktop.DBus',
            'org.freedesktop.DBus',
            'NameOwnerChanged',
            '/org/freedesktop/DBus',
            APPLICATION_BUS_NAME,
            Gio.DBusSignalFlags.NONE,
            (_connection, _sender, _path, _interface, _signal, parameters) => {
                const [_name, _oldOwner, newOwner] = parameters.deepUnpack();
                if (newOwner)
                    this._setAdapterEnabled(true);
            });

        this._setAdapterEnabled(true);
        this._launchApp();
    }

    disable() {
        this._setAdapterEnabled(false);
        if (this._appOwnerSubscription !== undefined) {
            Gio.DBus.session.signal_unsubscribe(this._appOwnerSubscription);
            this._appOwnerSubscription = undefined;
        }
        if (this._refreshSource !== 0) {
            GLib.source_remove(this._refreshSource);
            this._refreshSource = 0;
        }

        for (const menu of this._languageMenus ?? []) {
            this._popupManager?.removeMenu(menu);
            menu.destroy();
        }
        this._languageMenus = [];
        if (this._contextMenu) {
            this._popupManager?.removeMenu(this._contextMenu);
            this._contextMenu.destroy();
        }
        this._contextMenu = null;
        this._popupManager = null;
        this._indicator?.destroy();
        this._indicator = null;
        this._entry = null;
        this._result = null;
        this._status = null;
        this._snapshot = null;
        this.unloadStylesheet();
    }

    _buildPopup() {
        const content = new PopupMenu.PopupBaseMenuItem({
            reactive: false,
            can_focus: false,
            style_class: 'langux-content-item',
        });
        const root = new St.BoxLayout({
            vertical: true,
            x_expand: true,
            style_class: 'langux-popup',
        });
        content.add_child(root);
        this._indicator.menu.addMenuItem(content);

        const header = new St.BoxLayout({
            x_expand: true,
            style_class: 'langux-header',
        });
        header.add_child(new St.Label({
            text: 'Langux',
            x_expand: true,
            y_align: Clutter.ActorAlign.CENTER,
            style_class: 'langux-title',
        }));
        header.add_child(this._iconButton('preferences-system-symbolic', 'Settings', () => {
            this._call('OpenSettings');
        }));
        header.add_child(this._iconButton('window-close-symbolic', 'Close popup', () => {
            this._indicator.menu.close();
        }));
        root.add_child(header);

        const languageRow = new St.BoxLayout({
            x_expand: true,
            style_class: 'langux-language-row',
        });
        this._sourceButton = this._textButton('Auto detect', 'langux-language-button');
        this._swapButton = this._textButton('‹ ›', 'langux-swap-button');
        this._targetButton = this._textButton('English', 'langux-language-button');
        languageRow.add_child(this._sourceButton);
        languageRow.add_child(this._swapButton);
        languageRow.add_child(this._targetButton);
        root.add_child(languageRow);

        this._createLanguageMenu(this._sourceButton, true);
        this._createLanguageMenu(this._targetButton, false);
        this._swapButton.connect('clicked', () => this._swapLanguages());

        this._entry = new St.Entry({
            can_focus: true,
            x_expand: true,
            style_class: 'langux-input',
            hint_text: 'Enter text',
        });
        const textActor = this._entry.get_clutter_text();
        textActor.set_single_line_mode(false);
        textActor.set_line_wrap(true);
        textActor.set_line_wrap_mode(Pango.WrapMode.WORD_CHAR);
        textActor.set_max_length(MAX_INPUT_LENGTH);
        textActor.connect('text-changed', () => {
            this._call('SetInput', new GLib.Variant('(s)', [this._entry.get_text()]));
        });
        textActor.connect('key-press-event', (_actor, event) => this._onInputKeyPress(event));
        root.add_child(this._entry);

        const clearRow = new St.BoxLayout({
            x_expand: true,
            x_align: Clutter.ActorAlign.END,
            style_class: 'langux-action-row',
        });
        clearRow.add_child(this._textButton('Clear', 'langux-action-button', () => {
            this._call('Clear');
            this._entry.grab_key_focus();
        }));
        root.add_child(clearRow);

        root.add_child(new St.Widget({style_class: 'langux-separator'}));
        root.add_child(new St.Label({
            text: 'Translation',
            x_align: Clutter.ActorAlign.START,
            style_class: 'langux-section-title',
        }));
        this._result = new St.Label({
            x_expand: true,
            y_expand: true,
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
            style_class: 'langux-result',
        });
        this._result.clutter_text.set_line_wrap(true);
        this._result.clutter_text.set_line_wrap_mode(Pango.WrapMode.WORD_CHAR);
        root.add_child(this._result);
        this._status = new St.Label({
            x_expand: true,
            style_class: 'langux-status',
        });
        root.add_child(this._status);
        this._setStatus('Starting Langux…', false);

        const copyRow = new St.BoxLayout({
            x_expand: true,
            x_align: Clutter.ActorAlign.END,
            style_class: 'langux-action-row',
        });
        copyRow.add_child(this._textButton('Copy', 'langux-action-button', () => {
            this._call('Copy', null, BOOL_REPLY_TYPE, ([copied]) => {
                this._setStatus(copied ? 'Copied.' : 'Nothing to copy.', false);
            });
        }));
        root.add_child(copyRow);
    }

    _buildContextMenu() {
        this._contextMenu = new PopupMenu.PopupMenu(this._indicator, 0.5, St.Side.TOP);
        this._contextMenu.addMenuItem(new PopupMenu.PopupMenuItem('Quit Langux'))
            .connect('activate', () => this._call('Quit'));
        this._contextMenu.actor.hide();
        Main.uiGroup.add_child(this._contextMenu.actor);
        this._popupManager.addMenu(this._contextMenu);
    }

    _createLanguageMenu(button, source) {
        const menu = new PopupMenu.PopupMenu(button, 0.5, St.Side.TOP);
        menu.actor.hide();
        Main.uiGroup.add_child(menu.actor);
        this._popupManager.addMenu(menu);
        this._languageMenus.push(menu);
        button.connect('clicked', () => {
            this._populateLanguageMenu(menu, source);
            menu.open();
        });
    }

    _populateLanguageMenu(menu, source) {
        menu.removeAll();
        const languages = source ? this._languages : this._languages.filter(([code]) => code !== 'auto');
        const current = source ? this._snapshot?.source_language : this._snapshot?.target_language;
        for (const [code, name] of languages) {
            const item = new PopupMenu.PopupMenuItem(name);
            item.setOrnament(code === current ? PopupMenu.Ornament.DOT : PopupMenu.Ornament.NONE);
            item.connect('activate', () => {
                const sourceCode = source ? code : this._snapshot?.source_language ?? 'auto';
                const targetCode = source ? this._snapshot?.target_language ?? 'en' : code;
                this._call('SetLanguagePair', new GLib.Variant('(ss)', [sourceCode, targetCode]));
                menu.close();
            });
            menu.addMenuItem(item);
        }
    }

    _iconButton(iconName, accessibleName, onClicked) {
        const button = new St.Button({
            can_focus: true,
            style_class: 'langux-icon-button',
            accessible_name: accessibleName,
            child: new St.Icon({icon_name: iconName, style_class: 'langux-header-icon'}),
        });
        button.connect('clicked', onClicked);
        return button;
    }

    _textButton(label, styleClass, onClicked = null) {
        const button = new St.Button({
            can_focus: true,
            x_expand: styleClass === 'langux-language-button',
            style_class: styleClass,
            child: new St.Label({
                text: label,
                x_expand: true,
                x_align: Clutter.ActorAlign.CENTER,
                y_align: Clutter.ActorAlign.CENTER,
            }),
        });
        if (onClicked)
            button.connect('clicked', onClicked);
        return button;
    }

    _swapLanguages() {
        const source = this._snapshot?.source_language;
        const target = this._snapshot?.target_language;
        if (!source || source === 'auto' || !target)
            return;

        this._call('SetLanguagePair', new GLib.Variant('(ss)', [target, source]));
    }

    _onInputKeyPress(event) {
        const key = event.get_key_symbol();
        const modifiers = event.get_state();
        const hasModifier = mask => (modifiers & mask) !== 0;

        if (key === Clutter.KEY_Escape) {
            this._indicator.menu.close();
            return Clutter.EVENT_STOP;
        }
        if (key === Clutter.KEY_Return || key === Clutter.KEY_KP_Enter) {
            const shift = hasModifier(Clutter.ModifierType.SHIFT_MASK);
            const control = hasModifier(Clutter.ModifierType.CONTROL_MASK);
            if (!shift && (control || this._snapshot?.mode === 'manual')) {
                this._call('Translate');
                return Clutter.EVENT_STOP;
            }
        }
        if ((key === Clutter.KEY_c || key === Clutter.KEY_C) &&
            hasModifier(Clutter.ModifierType.MOD1_MASK)) {
            this._call('Copy');
            return Clutter.EVENT_STOP;
        }
        return Clutter.EVENT_PROPAGATE;
    }

    _refresh() {
        if (!this._languagesLoaded && !this._languagesRequestPending &&
            GLib.get_monotonic_time() >= this._languagesRetryAfter) {
            this._languagesRequestPending = true;
            this._call('GetLanguages', null, LANGUAGES_REPLY_TYPE, ([languages]) => {
                this._languages = languages;
                this._languagesLoaded = true;
                this._languagesRequestPending = false;
            }, () => {
                this._languagesRequestPending = false;
                this._languagesRetryAfter = GLib.get_monotonic_time() + 2 * GLib.TIME_SPAN_SECOND;
            });
        }
        this._call('GetState', null, STATE_REPLY_TYPE, state => {
            const [source, target, input, result, status, phase, isError, mode,
                detectedSource] = state;
            this._snapshot = {
                source_language: source,
                target_language: target,
                input_text: input,
                translated_text: result,
                status,
                phase,
                is_error: isError,
                mode,
                detected_source_language: detectedSource,
            };
            if (!this._entry.has_key_focus() && this._entry.get_text() !== input)
                this._entry.set_text(input);
            this._result.set_text(result);
            this._setButtonLabel(this._sourceButton, this._languageName(source));
            this._setButtonLabel(this._targetButton, this._languageName(target));
            this._swapButton.set_sensitive(source !== 'auto');
            this._setStatus(status, isError || phase === 'error');
        }, () => {
            this._setStatus('Langux is starting or unavailable.', true);
        });
    }

    _languageName(code) {
        return this._languages.find(([languageCode]) => languageCode === code)?.[1] ?? code;
    }

    _setButtonLabel(button, text) {
        button.child.set_text(text);
    }

    _setStatus(text, isError) {
        if (!this._status)
            return;
        this._status.set_text(text);
        this._status.visible = text.length > 0;
        this._status.remove_style_class_name('langux-status-error');
        if (isError)
            this._status.add_style_class_name('langux-status-error');
    }

    _launchApp() {
        const now = GLib.get_monotonic_time();
        if (now - this._lastLaunchTime < 2 * GLib.TIME_SPAN_SECOND)
            return;

        const appInfo = Gio.DesktopAppInfo.new(APPLICATION_DESKTOP_ID);
        if (!appInfo) {
            this._setStatus('Install the Langux desktop application to translate.', true);
            return;
        }
        this._lastLaunchTime = now;
        try {
            appInfo.launch_action('GnomeShellPopup', global.create_app_launch_context(0, -1));
        } catch (_error) {
            this._setStatus('Could not start Langux.', true);
        }
    }

    _setAdapterEnabled(enabled) {
        this._call('SetAdapterEnabled', new GLib.Variant('(b)', [enabled]));
    }

    _call(method, parameters = null, replyType = null, onSuccess = null, onError = null) {
        Gio.DBus.session.call(
            APPLICATION_BUS_NAME,
            APPLICATION_OBJECT_PATH,
            APPLICATION_INTERFACE,
            method,
            parameters,
            replyType,
            Gio.DBusCallFlags.NONE,
            5000,
            null,
            (connection, result) => {
                try {
                    const reply = connection.call_finish(result);
                    onSuccess?.(reply.deepUnpack()[0]);
                } catch (error) {
                    // The app may still be starting or may have been quit. The
                    // panel icon remains available and retries on the next open.
                    onError?.(error);
                }
            });
    }
}
