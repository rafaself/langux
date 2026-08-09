import Gtk from 'gi://Gtk';

export function createPreferenceButton(label) {
    return new Gtk.Button({
        label,
        css_classes: ['flat'],
        valign: Gtk.Align.CENTER,
    });
}
