import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {translate, ErrorCode} from '../services/googleTranslate.js';
import {AUTO_LANGUAGE, LANGUAGES, isExplicit, languageLabel, swapLanguages} from './languages.js';
import {friendlyMessage, needsSettingsAction} from './errorMessages.js';

const SWAP_ICON = 'view-refresh-symbolic';
const DROPDOWN_ICON = 'pan-down-symbolic';
const SETTINGS_ICON = 'applications-system-symbolic';
const SETTINGS_HINT = 'Open Langux preferences';
const CLOSE_ICON = 'window-close-symbolic';
const CLOSE_HINT = 'Close';
const COPY_ICON = 'edit-copy-symbolic';
const TITLE_TEXT = 'Langux';
const ENTRY_HINT = 'Enter text';
const FOOTER_HINT = 'Ctrl+Enter to translate';
const COPY_LABEL = 'Copy';
const COPIED_LABEL = 'Copied ✓';
const SETTINGS_LABEL = 'Go to Settings';
const COPY_FEEDBACK_MS = 1500;
const ERROR_CLASS = 'langux-error';
const INSENSITIVE_CLASS = 'langux-swap-insensitive';

export const TranslatorState = Object.freeze({
    IDLE: 'idle',
    TRANSLATING: 'translating',
    SUCCESS: 'success',
    ERROR: 'error',
});

export class TranslatorPopup extends PopupMenu.PopupMenu {
    constructor(sourceActor, settings) {
        super(sourceActor, 0.5, St.Side.TOP);

        this._settings = settings;
        this._state = TranslatorState.IDLE;
        this._destroyed = false;
        this._requestSeq = 0;
        this._cancellable = null;
        this._lastTranslatedText = null;

        this._buildContent();
        this._buildLanguageMenus();
        this._refreshLanguages();

        this.connect('open-state-changed', (menu, open) => {
            if (open)
                this._focusEntry();
            else
                this._closeLanguageMenus();
        });

        this.connect('destroy', () => {
            this._destroyed = true;
            this._cancelRequest();
            this._clearCopyFeedback();
        });
    }

    getState() {
        return this._state;
    }

    destroy() {
        this._closeLanguageMenus();
        this._destroyLanguageMenus();
        super.destroy();
    }

    _cancelRequest() {
        if (this._cancellable) {
            this._cancellable.cancel();
            this._cancellable = null;
        }
    }

    _clearCopyFeedback() {
        if (this._copyFeedbackId) {
            GLib.source_remove(this._copyFeedbackId);
            this._copyFeedbackId = null;
        }
    }

    _buildContent() {
        const content = new St.BoxLayout({vertical: true, style_class: 'langux-content'});

        content.add_child(this._buildHeader());

        const languageRow = new St.BoxLayout({style_class: 'langux-language-row'});
        this._sourceButton = this._createLanguageButton('langux-source-button');
        this._targetButton = this._createLanguageButton('langux-target-button');
        this._swapButton = this._createSwapButton();
        languageRow.add_child(this._sourceButton.button);
        languageRow.add_child(this._swapButton);
        languageRow.add_child(this._targetButton.button);
        content.add_child(languageRow);

        this._entry = new St.Entry({
            style_class: 'langux-entry',
            hint_text: ENTRY_HINT,
            can_focus: true,
            x_expand: true,
        });
        this._entry.clutter_text.max_length = 4096;
        this._entry.clutter_text.single_line_mode = false;
        this._entry.clutter_text.line_wrap = true;
        this._entry.connect('key-press-event', this._onEntryKeyPress.bind(this));
        content.add_child(this._entry);

        content.add_child(new St.Widget({style_class: 'langux-separator'}));

        const resultArea = new St.BoxLayout({vertical: true, style_class: 'langux-result-area'});
        this._spinner = new St.Widget({
            style_class: 'view-spinner',
            layout_manager: new Clutter.BinLayout(),
            width: 24,
            height: 24,
            visible: false,
        });
        this._resultLabel = new St.Label({
            style_class: 'langux-result-label',
            x_expand: true,
            x_align: Clutter.ActorAlign.START,
            y_align: Clutter.ActorAlign.START,
        });
        this._resultLabel.clutter_text.line_wrap = true;
        this._detectedLabel = new St.Label({style_class: 'langux-detected-label'});
        resultArea.add_child(this._spinner);
        resultArea.add_child(this._resultLabel);
        resultArea.add_child(this._detectedLabel);

        this._actionRow = new St.BoxLayout({style_class: 'langux-actions-row'});
        this._copyButton = this._createActionButton(COPY_LABEL, COPY_ICON, () => this._copyResult());
        this._settingsActionButton = this._createActionButton(SETTINGS_LABEL, SETTINGS_ICON, () => this._openSettings());
        this._actionRow.add_child(this._copyButton.button);
        this._actionRow.add_child(this._settingsActionButton.button);
        resultArea.add_child(this._actionRow);
        content.add_child(resultArea);

        this.box.add_child(content);
        this.addMenuItem(new PopupMenu.PopupMenuItem(FOOTER_HINT, {
            reactive: false,
            hover: false,
            activate: false,
            can_focus: false,
            style_class: 'langux-footer',
        }));
    }

    _buildHeader() {
        const headerRow = new St.BoxLayout({style_class: 'langux-header'});
        headerRow.add_child(new St.Label({
            text: TITLE_TEXT,
            style_class: 'langux-title',
            x_expand: true,
        }));
        const actions = new St.BoxLayout({style_class: 'langux-header-actions'});
        actions.add_child(this._buildHeaderAction(
            SETTINGS_ICON, SETTINGS_HINT, () => this._openSettings()));
        actions.add_child(this._buildHeaderAction(
            CLOSE_ICON, CLOSE_HINT, () => this.close()));
        headerRow.add_child(actions);
        return headerRow;
    }

    _buildHeaderAction(iconName, hint, onClick) {
        const button = new St.Button({
            style_class: 'langux-icon-button',
            child: new St.Icon({icon_name: iconName, icon_size: 14}),
            can_focus: true,
            reactive: true,
            accessible_name: hint,
        });
        button.connect('clicked', onClick);
        return button;
    }

    _createActionButton(text, iconName, onClick) {
        const box = new St.BoxLayout({style_class: 'langux-action-button-content'});
        const label = new St.Label({text});
        box.add_child(new St.Icon({icon_name: iconName, icon_size: 12}));
        box.add_child(label);
        const button = new St.Button({
            style_class: 'langux-action-button',
            child: box,
            can_focus: true,
            reactive: true,
        });
        button.connect('clicked', onClick);
        return {button, label};
    }

    _createLanguageButton(styleClass) {
        const label = new St.Label({
            text: '',
            style_class: 'langux-language-label',
            x_expand: true,
            x_align: Clutter.ActorAlign.START,
        });
        const chevron = new St.Icon({icon_name: DROPDOWN_ICON, icon_size: 10});
        const child = new St.BoxLayout({style_class: 'langux-language-button-content'});
        child.add_child(label);
        child.add_child(chevron);
        const button = new St.Button({
            style_class: styleClass,
            child,
            can_focus: true,
            reactive: true,
        });
        button.connect('clicked', () => {
            this._closeLanguageMenus();
            this._menuForButton(button).toggle();
        });
        return {button, label};
    }

    _createSwapButton() {
        const button = new St.Button({
            style_class: 'langux-swap-button',
            child: new St.Icon({icon_name: SWAP_ICON, icon_size: 12}),
            can_focus: true,
            reactive: true,
        });
        button.connect('clicked', () => this._swapLanguages());
        return button;
    }

    _buildLanguageMenus() {
        this._menuManager = new PopupMenu.PopupMenuManager(this);

        this._sourceMenuItems = new Map();
        this._sourceMenu = this._buildMenu(this._sourceButton.button, this._sourceMenuItems, true);

        this._targetMenuItems = new Map();
        this._targetMenu = this._buildMenu(this._targetButton.button, this._targetMenuItems, false);

        this._menuManager.addMenu(this._sourceMenu);
        this._menuManager.addMenu(this._targetMenu);

        for (const menu of [this._sourceMenu, this._targetMenu]) {
            menu.connect('open-state-changed', (m, open) => {
                if (!open && this.isOpen)
                    this._focusEntry();
            });
        }
    }

    _buildMenu(sourceActor, items, withAuto) {
        const menu = new PopupMenu.PopupMenu(sourceActor, 0.5, St.Side.TOP);
        const codes = withAuto
            ? [AUTO_LANGUAGE, ...LANGUAGES.map(l => l.code)]
            : LANGUAGES.map(l => l.code);
        for (const code of codes) {
            const item = new PopupMenu.PopupMenuItem(languageLabel(code));
            item.connect('activate', () => this._onLanguageActivated(code, withAuto));
            menu.addMenuItem(item);
            items.set(code, item);
        }
        return menu;
    }

    _onLanguageActivated(code, isSource) {
        if (isSource)
            this._settings.set_string('source-language', code);
        else
            this._settings.set_string('target-language', code);
        this._refreshLanguages();
        this._focusEntry();
    }

    _menuForButton(button) {
        return button === this._sourceButton.button ? this._sourceMenu : this._targetMenu;
    }

    _swapLanguages() {
        const swapped = swapLanguages(
            this._settings.get_string('source-language'),
            this._settings.get_string('target-language'));
        if (!swapped)
            return;
        this._settings.set_string('source-language', swapped.source);
        this._settings.set_string('target-language', swapped.target);
        this._refreshLanguages();
    }

    _refreshLanguages() {
        const source = this._settings.get_string('source-language');
        const target = this._settings.get_string('target-language');

        this._sourceButton.label.set_text(languageLabel(source));
        this._targetButton.label.set_text(languageLabel(target));

        const explicit = isExplicit(source);
        this._swapButton.reactive = explicit;
        if (explicit)
            this._swapButton.remove_style_class_name(INSENSITIVE_CLASS);
        else
            this._swapButton.add_style_class_name(INSENSITIVE_CLASS);

        this._updateOrnaments(this._sourceMenuItems, source);
        this._updateOrnaments(this._targetMenuItems, target);
    }

    _updateOrnaments(items, selectedCode) {
        for (const [code, item] of items)
            item.setOrnament(code === selectedCode ? PopupMenu.Ornament.CHECK : PopupMenu.Ornament.NONE);
    }

    _closeLanguageMenus() {
        if (this._sourceMenu?.isOpen)
            this._sourceMenu.close();
        if (this._targetMenu?.isOpen)
            this._targetMenu.close();
    }

    _destroyLanguageMenus() {
        this._sourceMenu?.destroy();
        this._targetMenu?.destroy();
        this._sourceMenu = null;
        this._targetMenu = null;
        this._sourceMenuItems = null;
        this._targetMenuItems = null;
    }

    _onEntryKeyPress(actor, event) {
        const symbol = event.get_key_symbol();
        if (symbol !== Clutter.KEY_Return && symbol !== Clutter.KEY_KP_Enter)
            return Clutter.EVENT_PROPAGATE;

        const modifiers = event.get_state() & Clutter.ModifierType.MODIFIER_MASK;
        if ((modifiers & Clutter.ModifierType.CONTROL_MASK) === 0)
            return Clutter.EVENT_PROPAGATE;

        this._translate();
        return Clutter.EVENT_STOP;
    }

    _translate() {
        const text = this._entry.get_text().trim();
        if (!text || this._destroyed)
            return;

        const source = this._settings.get_string('source-language');
        const target = this._settings.get_string('target-language');

        this._requestSeq++;
        this._cancelRequest();
        this._cancellable = new Gio.Cancellable();
        const seq = this._requestSeq;

        this.setLoading();
        translate({text, source, target, cancellable: this._cancellable})
            .then(result => {
                if (seq !== this._requestSeq || this._destroyed)
                    return;
                this.setResult(result);
            })
            .catch(error => {
                if (seq !== this._requestSeq || this._destroyed)
                    return;
                if (error?.code === ErrorCode.CANCELLED)
                    return;
                this.setError(error);
            });
    }

    setLoading() {
        this._state = TranslatorState.TRANSLATING;
        this._spinner.visible = true;
        this._resultLabel.visible = false;
        this._detectedLabel.visible = false;
        this._showAction(COPY_LABEL, false, false);
    }

    setResult(result) {
        this._state = TranslatorState.SUCCESS;
        this._lastTranslatedText = result.text;
        this._spinner.visible = false;
        this._resultLabel.remove_style_class_name(ERROR_CLASS);
        this._resultLabel.clutter_text.text = result.text;
        this._resultLabel.visible = true;

        const detected = result.detectedSourceLanguage;
        if (detected && detected !== AUTO_LANGUAGE) {
            this._detectedLabel.set_text(`Detected: ${languageLabel(detected)}`);
            this._detectedLabel.visible = true;
        } else {
            this._detectedLabel.visible = false;
        }

        this._showAction(COPY_LABEL, true, false);
        this._focusEntry();
    }

    setError(error) {
        this._state = TranslatorState.ERROR;
        this._spinner.visible = false;
        this._resultLabel.add_style_class_name(ERROR_CLASS);
        this._resultLabel.clutter_text.text = friendlyMessage(error?.code);
        this._resultLabel.visible = true;
        this._detectedLabel.visible = false;
        this._showAction(COPY_LABEL, false, needsSettingsAction(error?.code));
    }

    _openSettings() {
        if (this.onOpenSettings)
            this.onOpenSettings();
    }

    _copyResult() {
        if (!this._lastTranslatedText)
            return;
        St.Clipboard.get_default().set_text(
            St.ClipboardType.CLIPBOARD,
            this._lastTranslatedText);
        this._showAction(COPIED_LABEL, true, false);
        this._clearCopyFeedback();
        this._copyFeedbackId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, COPY_FEEDBACK_MS, () => {
            this._copyFeedbackId = null;
            this._showAction(COPY_LABEL, true, false);
            return false;
        });
    }

    _showAction(copyLabel, copyVisible, settingsVisible) {
        this._copyButton.button.visible = copyVisible;
        this._settingsActionButton.button.visible = settingsVisible;
        this._copyButton.label.set_text(copyLabel);
    }

    _focusEntry() {
        this._entry.clutter_text.grab_key_focus();
    }
}