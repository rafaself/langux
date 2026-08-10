#!/usr/bin/env bash
set -uo pipefail

# Headless GNOME-runtime API-surface probe for the APIs Langux actually uses.
# Catches the class of bugs that pure unit tests (node --test) cannot: wrong
# method spellings on introspected C libraries (Gtk.Box.add vs Gtk.Box.append
# in GTK4) and resource:/// import paths that don't resolve at runtime.
#
# Result lines:  RHECK: <check> OK|SKIP|FAIL
#   SKIP      - a prerequisite typelib/gresource isn't installed here (fine).
#   FAIL      - the check ran against the real runtime and came out wrong.
# Exit code is nonzero if any FAIL is emitted.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

FAILURES=0

if ! command -v gjs >/dev/null 2>&1; then
    echo "RHECK: runtime probe SKIP (gjs not installed)"
    echo "runtime check result: 0 failure(s)"
    exit 0
fi

typelib_exists() { # $1 = name like "Gtk-4.0"
    local name="$1" dir
    for dir in \
        /usr/lib/girepository-1.0 \
        /usr/lib64/girepository-1.0 \
        /usr/lib/*/girepository-1.0 \
        /usr/local/lib/girepository-1.0; do
        [ -f "$dir/$name.typelib" ] && return 0
    done
    return 1
}

emitok() { echo "RHECK: $1 OK"; }
emit_skip() { echo "RHECK: $1 SKIP"; }

probe_js() { # $1 = label, $2 = gjs file
    local label="$1" file="$2" out rc
    out="$(timeout 30 gjs -m "$file" 2>&1)"
    rc=$?
    if grep -q 'RHECK: FAIL' <<<"$out"; then
        FAILURES=$((FAILURES + 1))
        echo "$out"
        return 1
    fi
    if [ "$rc" -ne 0 ]; then
        FAILURES=$((FAILURES + 1))
        echo "RHECK: $label FAIL (gjs exited $rc)"
        echo "$out" | tail -4 | sed 's/^/    /'
        return 1
    fi
    if grep -q 'RHECK:' <<<"$out"; then
        echo "$out"
    else
        echo "RHECK: $label OK"
    fi
}

# --- GTK4 / libadwaita (used by prefsContent.js) -------------------------------
if typelib_exists "Gtk-4.0" && typelib_exists "Adw-1"; then
    GTK_CHECK="$(mktemp --suffix=.mjs)"
    cat > "$GTK_CHECK" <<'EOF'
import Gtk from 'gi://Gtk?version=4.0';
import Adw from 'gi://Adw?version=1';

const check = (name, cond) => console.log(`RHECK: ${name} ${cond ? 'OK' : 'FAIL'}`);
check('Gtk.Box.append exists', Object.getOwnPropertyNames(Gtk.Box.prototype).includes('append'));
check('Gtk.Box has no add()', !Object.getOwnPropertyNames(Gtk.Box.prototype).includes('add'));
check('Adw.PreferencesGroup.add exists', Object.getOwnPropertyNames(Adw.PreferencesGroup.prototype).includes('add'));
check('Adw.PreferencesPage.add exists', Object.getOwnPropertyNames(Adw.PreferencesPage.prototype).includes('add'));
check('Adw.PreferencesWindow.add exists', Object.getOwnPropertyNames(Adw.PreferencesWindow.prototype).includes('add'));
check('Adw.PreferencesWindow.set_focus callable', typeof Adw.PreferencesWindow.prototype.set_focus === 'function');
check('Adw.ComboRow constructible', typeof Adw.ComboRow === 'function');
check('Adw.SwitchRow.active exists', Object.getOwnPropertyNames(Adw.SwitchRow.prototype).includes('active'));
check('Adw.SpinRow.value exists', Object.getOwnPropertyNames(Adw.SpinRow.prototype).includes('value'));
check('Adw.SpinRow.adjustment exists', Object.getOwnPropertyNames(Adw.SpinRow.prototype).includes('adjustment'));
check('Adw.MessageDialog.add_response exists', Object.getOwnPropertyNames(Adw.MessageDialog.prototype).includes('add_response'));
check('Adw.MessageDialog.set_extra_child callable', typeof Adw.MessageDialog.prototype.set_extra_child === 'function');
check('Adw.MessageDialog.body_use_markup exists', Object.getOwnPropertyNames(Adw.MessageDialog.prototype).includes('body_use_markup'));
check('Adw.MessageDialog.present callable', typeof Adw.MessageDialog.prototype.present === 'function');
check('Adw.MessageDialog.set_transient_for callable', typeof Adw.MessageDialog.prototype.set_transient_for === 'function');
check('Adw.Dialog constructible', typeof Adw.Dialog === 'function');
check('Adw.Dialog.present callable', typeof Adw.Dialog.prototype.present === 'function');
check('Adw.Dialog.close callable', typeof Adw.Dialog.prototype.close === 'function');
check('Adw.Dialog.set_child callable', typeof Adw.Dialog.prototype.set_child === 'function');
check('Adw.ToolbarView constructible', typeof Adw.ToolbarView === 'function');
check('Adw.ToolbarView.add_top_bar callable', typeof Adw.ToolbarView.prototype.add_top_bar === 'function');
check('Adw.ToolbarView.set_content callable', typeof Adw.ToolbarView.prototype.set_content === 'function');
check('Adw.HeaderBar constructible', typeof Adw.HeaderBar === 'function');
check('Adw.PreferencesDialog constructible', typeof Adw.PreferencesDialog === 'function');
check('Adw.PreferencesDialog.add exists', Object.getOwnPropertyNames(Adw.PreferencesDialog.prototype).includes('add'));
check('Adw.PreferencesPage.description exists', Object.getOwnPropertyNames(Adw.PreferencesPage.prototype).includes('description'));
check('Adw.ActionRow.activatable_widget exists', Object.getOwnPropertyNames(Adw.ActionRow.prototype).includes('activatable_widget'));
check('Adw.AboutDialog constructible', typeof Adw.AboutDialog === 'function');
check('Adw.AboutDialog.present callable', typeof Adw.AboutDialog.prototype.present === 'function');
check('Adw.AboutDialog.application_name exists', Object.getOwnPropertyNames(Adw.AboutDialog.prototype).includes('application_name'));
check('Adw.AboutDialog.license_type exists', Object.getOwnPropertyNames(Adw.AboutDialog.prototype).includes('license_type'));
check('Adw.AboutDialog.website exists', Object.getOwnPropertyNames(Adw.AboutDialog.prototype).includes('website'));
check('Adw.AboutDialog.issue_url exists', Object.getOwnPropertyNames(Adw.AboutDialog.prototype).includes('issue_url'));
check('Gtk.Label.selectable exists', Object.getOwnPropertyNames(Gtk.Label.prototype).includes('selectable'));
check('Gtk.Widget.width_request exists', Object.getOwnPropertyNames(Gtk.Widget.prototype).includes('width_request'));
EOF
    probe_js "GTK4/libadwaita surface" "$GTK_CHECK"
    rm -f "$GTK_CHECK"
else
    echo "RHECK: GTK4/libadwaita surface SKIP (Gtk-4.0/Adw-1 typelibs absent)"
fi

# --- libsecret / libsoup -------------------------------------------------------
if typelib_exists "Secret-1" && typelib_exists "Soup-3.0"; then
    SVC_CHECK="$(mktemp --suffix=.mjs)"
    cat > "$SVC_CHECK" <<'EOF'
import Gio from 'gi://Gio';
import Secret from 'gi://Secret';
import Soup from 'gi://Soup?version=3.0';

const check = (name, cond) => console.log(`RHECK: ${name} ${cond ? 'OK' : 'FAIL'}`);
check('libsecret password_lookup callable', typeof Secret.password_lookup === 'function');
check('libsoup3 Session constructible', typeof Soup.Session === 'function');
check('libsoup3 Message.new callable', typeof Soup.Message.new === 'function');
check('libsoup3 send_and_read_async callable', typeof Soup.Session.prototype.send_and_read_async === 'function');
check('Gio.AppInfo.launch_default_for_uri callable', typeof Gio.AppInfo.launch_default_for_uri === 'function');
EOF
    probe_js "service libs" "$SVC_CHECK"
    rm -f "$SVC_CHECK"
else
    echo "RHECK: service libs SKIP (Secret-1/Soup-3.0 typelibs absent)"
fi

# --- update service module ----------------------------------------------------
if typelib_exists "Soup-3.0"; then
    UPDATE_CHECK="$(mktemp --tmpdir="$ROOT/services" .runtime-update-XXXXXX.mjs)"
    cat > "$UPDATE_CHECK" <<'EOF'
import {checkForUpdates, UpdateChecker} from './updateChecker.js';
import {UPDATE_API_URL, UPDATE_PAGE_URL} from '../ui/updateInfo.js';
import {buildAboutGroup} from '../ui/aboutContent.js';
import {createPreferencesDialog} from '../ui/dialogContent.js';
import {createPreferenceButton} from '../ui/prefsWidgets.js';
import {buildShortcutsGroup} from '../ui/shortcutsContent.js';
import {buildUpdatesGroup} from '../ui/updatesContent.js';

const check = (name, cond) => console.log(`RHECK: ${name} ${cond ? 'OK' : 'FAIL'}`);
check('updateChecker module loads', typeof checkForUpdates === 'function');
check('updateChecker exposes cancellation', typeof UpdateChecker === 'function');
check('about Preferences module loads', typeof buildAboutGroup === 'function');
check('preferences dialog helper loads', typeof createPreferencesDialog === 'function');
check('preference button helper loads', typeof createPreferenceButton === 'function');
check('shortcuts Preferences module loads', typeof buildShortcutsGroup === 'function');
check('updates Preferences module loads', typeof buildUpdatesGroup === 'function');
check('update API is fixed HTTPS GitHub URL',
    UPDATE_API_URL === 'https://api.github.com/repos/rafaself/langux/releases/latest');
check('update page is fixed HTTPS GitHub URL',
    UPDATE_PAGE_URL === 'https://github.com/rafaself/langux/releases/latest');
EOF
    probe_js "update service" "$UPDATE_CHECK"
    rm -f "$UPDATE_CHECK"
else
    echo "RHECK: update service SKIP (Soup-3.0 typelib absent)"
fi

# --- prefs.js resource path resolves at runtime -------------------------------
PREFS_CHECK="$(mktemp --suffix=.mjs)"
cat > "$PREFS_CHECK" <<'EOF'
import Gio from 'gi://Gio';
import system from 'system';

const candidates = [
    '/usr/share/gnome-shell/org.gnome.Shell.Extensions.src.gresource',
    '/usr/share/gnome-shell/org.gnome.Shell.Extensions.gresource',
];
const path = '/org/gnome/Shell/Extensions/js/extensions/prefs.js';

for (const file of candidates) {
    let resource;
    try {
        resource = Gio.Resource.load(file);
    } catch (err) {
        continue;
    }
    try {
        if (resource.lookup_data(path, Gio.ResourceLookupFlags.NONE))
            console.log('RHECK: prefs.js resource path OK');
        else
            console.log('RHECK: prefs.js resource path FAIL');
        system.exit(0);
    } catch (err) {
        console.log('RHECK: prefs.js resource path FAIL');
        system.exit(1);
    }
}
console.log('RHECK: prefs.js resource path SKIP (no gnome-shell gresource found)');
EOF
probe_js "prefs.js resource path" "$PREFS_CHECK"
rm -f "$PREFS_CHECK"

echo "runtime check result: $FAILURES failure(s)"
[ "$FAILURES" -eq 0 ]
