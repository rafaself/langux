import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {UpdateChecker, UpdateErrorCode} from '../services/updateChecker.js';
import {createPreferencesDialog} from './dialogContent.js';
import {createPreferenceButton} from './prefsWidgets.js';
import {UPDATE_PAGE_URL} from './updateInfo.js';

function buildActionGroup(button) {
    const group = new Adw.PreferencesGroup();
    const row = new Adw.ActionRow({
        title: 'Open update page',
        subtitle: 'View the stable release on GitHub',
    });
    row.add_suffix(button);
    row.activatable_widget = button;
    group.add(row);
    return group;
}

export function buildUpdatesGroup({currentVersion, window, group: parentGroup = null}) {
    const group = parentGroup ?? new Adw.PreferencesGroup({title: 'Updates'});
    const row = new Adw.ActionRow({
        title: 'Check for updates',
        subtitle: `Installed version: ${currentVersion}`,
    });
    const checkButton = createPreferenceButton('Check');
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
        dialog.present(window);
    }

    function showUpToDateDialog(info) {
        const dialog = createPreferencesDialog({
            title: 'Langux is up to date',
            description: `Current version: ${info.currentVersion}\nLatest version: ${info.latestVersion}`,
        });
        presentDialog(dialog);
    }

    function showUpdateAvailableDialog(info) {
        const openButton = new Gtk.Button({
            label: 'Open update page',
            css_classes: ['suggested-action'],
            valign: Gtk.Align.CENTER,
        });
        const dialog = createPreferencesDialog({
            title: 'Update available',
            description: `A newer stable release is available.\n\nCurrent version: ${info.currentVersion}\nLatest version: ${info.latestVersion}\nRelease: ${info.releaseTitle}`,
            groups: [buildActionGroup(openButton)],
        });
        openButton.connect('clicked', () => {
            try {
                const launched = Gio.AppInfo.launch_default_for_uri(UPDATE_PAGE_URL, null);
                if (launched === false)
                    console.error('Could not open the Langux update page.');
            } catch (error) {
                console.error('Could not open the Langux update page.');
            }
            dialog.close();
        });
        presentDialog(dialog);
    }

    function showFailureDialog() {
        const dialog = createPreferencesDialog({
            title: 'Could not check for updates',
            description: 'Unable to check for updates. Please try again later.',
        });
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
