import Adw from 'gi://Adw';

export function createPreferencesDialog({title, description, groups = [], contentWidth = 640}) {
    const dialog = new Adw.PreferencesDialog({
        title,
        content_width: contentWidth,
    });
    const page = new Adw.PreferencesPage({description});
    for (const group of groups) page.add(group);
    dialog.add(page);
    return dialog;
}
