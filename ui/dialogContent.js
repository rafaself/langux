import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

export function buildDialogContent({body, child = null}) {
    const content = new Gtk.Box({
        orientation: Gtk.Orientation.VERTICAL,
        spacing: 18,
        margin_top: 24,
        margin_bottom: 24,
        margin_start: 24,
        margin_end: 24,
    });
    content.append(new Gtk.Label({
        label: body,
        wrap: true,
        xalign: 0,
        hexpand: true,
        selectable: false,
    }));
    if (child)
        content.append(child);
    return content;
}

export function createHeaderDialog({title, content, contentWidth = 640}) {
    const dialog = new Adw.Dialog({
        title,
        content_width: contentWidth,
    });
    const toolbarView = new Adw.ToolbarView();
    toolbarView.add_top_bar(new Adw.HeaderBar());
    toolbarView.set_content(content);
    dialog.set_child(toolbarView);
    return dialog;
}
