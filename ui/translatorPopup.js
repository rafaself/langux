import Clutter from 'gi://Clutter';
import St from 'gi://St';

import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {AUTO_LANGUAGE, LANGUAGES, isExplicit, languageLabel, swapLanguages} from './languages.js';

const SWAP_ICON = 'view-refresh-symbolic';
const DROPDOWN_ICON = 'pan-down-symbolic';
const TITLE_TEXT = 'Langux';
const ENTRY_HINT = 'Type or paste text';
const FOOTER_HINT = 'Ctrl+Enter to translate';
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
        this._onTranslate = null;

        this._buildContent();
        this._buildLanguageMenus();
        this._refreshLanguages();

        this.connect('open-state-changed', (menu, open) => {
            if (open)
                this._focusEntry();
            else
                this._closeLanguageMenus();
        });
    }

    getState() {
        return this._state;
    }

    setLoading() {
        this._state = TranslatorState.TRANSLATING;
        this._spinner.visible = true;
        this._resultLabel.visible = false;
    }

    setResult(text, detectedLanguage = null) {
        if (detectedLanguage !== null)
            global.log(`[langux] detected language: ${detectedLanguage}`);
        this._state = TranslatorState.SUCCESS;
        this._spinner.visible = false;
        this._resultLabel.remove_style_class_name(ERROR_CLASS);
        this._resultLabel.clutter_text.text = text;
        this._resultLabel.visible = true;
    }

    setError(message) {
        this._state = TranslatorState.ERROR;
        this._spinner.visible = false;
        this._resultLabel.add_style_class_name(ERROR_CLASS);
        this._resultLabel.clutter_text.text = message;
        this._resultLabel.visible = true;
    }

    destroy() {
        this._closeLanguageMenus();
        this._destroyLanguageMenus();
        this._onTranslate = null;
        super.destroy();
    }

    _buildContent() {
        const content = new St.BoxLayout({vertical: true, style_class: 'langux-content'});

        content.add_child(new St.Label({text: TITLE_TEXT, style_class: 'langux-title'}));

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
            track_hover: true,
            x_expand: true,
        });
        this._entry.clutter_text.max_length = 4096;
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
        resultArea.add_child(this._spinner);
        resultArea.add_child(this._resultLabel);
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
        if (!text || this._state === TranslatorState.TRANSLATING || !this._onTranslate)
            return;
        this._onTranslate(text);
    }

    _focusEntry() {
        this._entry.clutter_text.grab_key_focus();
    }
}