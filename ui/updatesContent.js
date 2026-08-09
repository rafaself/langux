import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {UpdateChecker, UpdateErrorCode} from '../services/updateChecker.js';
import {buildDialogContent, createHeaderDialog} from './dialogContent.js';
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
        dialog.present(window);
    }

    function showUpToDateDialog(info) {
        const dialog = createHeaderDialog({
            title: 'Langux is up to date',
            content: buildDialogContent({
                body: `Current version: ${info.currentVersion}\nLatest version: ${info.latestVersion}`,
            }),
        });
        presentDialog(dialog);
    }

    function showUpdateAvailableDialog(info) {
        const openButton = new Gtk.Button({
            label: 'Open update page',
            css_classes: ['suggested-action'],
            halign: Gtk.Align.END,
        });
        const dialog = createHeaderDialog({
            title: 'Update available',
            content: buildDialogContent({
                body: `A newer stable release is available.\n\nCurrent version: ${info.currentVersion}\nLatest version: ${info.latestVersion}\nRelease: ${info.releaseTitle}`,
                child: openButton,
            }),
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
        const dialog = createHeaderDialog({
            title: 'Could not check for updates',
            content: buildDialogContent({
                body: 'Unable to check for updates. Please try again later.',
            }),
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
