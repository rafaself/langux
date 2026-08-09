import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';

function configureAboutDialog(widget) {
    if (!widget)
        return;

    if (widget instanceof Gtk.Label)
        widget.selectable = false;

    if (widget instanceof Adw.ActionRow && widget.visible && widget.title === '_Website')
        widget.title = 'GitHub';

    for (let child = widget.get_first_child(); child; child = child.get_next_sibling())
        configureAboutDialog(child);
}

export function buildAboutGroup({metadata, window}) {
    const version = metadata['version-name'];
    const repositoryUrl = metadata.url;

    const group = new Adw.PreferencesGroup({title: 'About'});
    const row = new Adw.ActionRow({
        title: `About ${metadata.name}`,
        subtitle: `Version ${version}`,
    });
    const aboutButton = new Gtk.Button({
        label: 'About',
        css_classes: ['flat'],
        valign: Gtk.Align.CENTER,
    });
    row.add_suffix(aboutButton);
    row.activatable_widget = aboutButton;
    group.add(row);

    let aboutDialog = null;
    aboutButton.connect('clicked', () => {
        if (!aboutDialog) {
            aboutDialog = new Adw.AboutDialog({
                application_name: metadata.name,
                version,
                developer_name: 'Langux contributors',
                website: repositoryUrl,
                license_type: Gtk.License.GPL_3_0,
            });
        }

        // Adw.AboutDialog generates the website and legal rows internally.
        // Keep the repository action explicit and all generated text
        // non-selectable, including the Legal page content.
        configureAboutDialog(aboutDialog.get_child());
        aboutDialog.present(window);
    });

    return group;
}
