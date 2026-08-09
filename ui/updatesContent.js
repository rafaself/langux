import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {UpdateChecker, UpdateErrorCode} from '../services/updateChecker.js';
import {UPDATE_PAGE_URL} from './updateInfo.js';

export function buildUpdatesGroup({currentVersion, window, group: parentGroup = null}) {
    const group = parentGroup ?? new Adw.PreferencesGroup({title: 'Updates'});
    const row = new Adw.ActionRow({
        title: 'Check for updates',
        subtitle: `Installed version: ${currentVersion}`,
    });
    const checkButton = new Gtk.Button({
        label: 'Check',
        css_classes: ['flat'],
        valign: Gtk.Align.CENTER,
    });
    row.add_suffix(checkButton);
    group.add(row);

    const checker = new UpdateChecker();
    let disposed = false;
    let checking = false;

    window.connect('close-request', () => {
        disposed = true;
        checker.cancel();
        return false;
    });

    function presentDialog(dialog) {
        if (disposed)
            return;
        dialog.set_transient_for(window);
        dialog.present();
    }

    function showUpToDateDialog(info) {
        const dialog = new Adw.MessageDialog({
            heading: 'Langux is up to date',
            body: `Current version: ${info.currentVersion}\nLatest version: ${info.latestVersion}`,
            body_use_markup: false,
            default_response: 'close',
            close_response: 'close',
        });
        dialog.add_response('close', 'Close');
        presentDialog(dialog);
    }

    function showUpdateAvailableDialog(info) {
        const dialog = new Adw.MessageDialog({
            heading: 'Update available',
            body: `A newer stable release is available.\n\nCurrent version: ${info.currentVersion}\nLatest version: ${info.latestVersion}\nRelease: ${info.releaseTitle}`,
            body_use_markup: false,
            default_response: 'update',
            close_response: 'close',
        });
        dialog.add_response('close', 'Close');
        dialog.add_response('update', 'Open update page');
        dialog.set_response_appearance('update', Adw.ResponseAppearance.SUGGESTED);
        dialog.connect('response', (dialog_, response) => {
            if (response !== 'update')
                return;
            try {
                const launched = Gio.AppInfo.launch_default_for_uri(UPDATE_PAGE_URL, null);
                if (launched === false)
                    console.error('Could not open the Langux update page.');
            } catch (error) {
                console.error('Could not open the Langux update page.');
            }
        });
        presentDialog(dialog);
    }

    function showFailureDialog() {
        const dialog = new Adw.MessageDialog({
            heading: 'Could not check for updates',
            body: 'Unable to check for updates. Please try again later.',
            body_use_markup: false,
            default_response: 'close',
            close_response: 'close',
        });
        dialog.add_response('close', 'Close');
        presentDialog(dialog);
    }

    async function check() {
        if (disposed || checking)
            return;

        checking = true;
        checkButton.sensitive = false;
        try {
            const info = await checker.check(currentVersion);
            if (info.updateAvailable)
                showUpdateAvailableDialog(info);
            else
                showUpToDateDialog(info);
        } catch (error) {
            if (!disposed && error?.code !== UpdateErrorCode.CANCELLED)
                showFailureDialog();
        } finally {
            checking = false;
            if (!disposed)
                checkButton.sensitive = true;
        }
    }

    checkButton.connect('clicked', check);
    return group;
}
