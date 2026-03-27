use crate::models::SshProfile;
use adw::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;

pub struct AddConnectionDialog {
    pub window: adw::Window,
    pub title_widget: adw::WindowTitle,
    pub name_entry: adw::EntryRow,
    pub host_entry: adw::EntryRow,
    pub port_entry: gtk::SpinButton,
    pub user_entry: adw::EntryRow,
    pub pass_entry: adw::PasswordEntryRow,
    pub identity_entry: gtk::Entry,
    pub identity_button: gtk::Button,
    pub save_button: gtk::Button,
}

impl AddConnectionDialog {
    pub fn new<W: IsA<gtk::Window>>(parent: &W) -> Self {
        let window = adw::Window::builder()
            .title("New Connection")
            .transient_for(parent)
            .modal(true)
            .default_width(500)
            .default_height(580)
            .build();

        let toolbar_view = adw::ToolbarView::new();

        let title_widget = adw::WindowTitle::new("New Connection", "");
        let header = adw::HeaderBar::builder()
            .title_widget(&title_widget)
            .build();

        let cancel_button = gtk::Button::builder().label("Cancel").build();

        let save_button = gtk::Button::builder()
            .label("Save")
            .css_classes(["suggested-action"])
            .build();

        header.pack_start(&cancel_button);
        header.pack_end(&save_button);
        toolbar_view.add_top_bar(&header);

        let window_cancel = window.clone();
        cancel_button.connect_clicked(move |_| {
            window_cancel.close();
        });

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(500)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(16)
            .margin_end(16)
            .build();

        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .build();

        let server_group = adw::PreferencesGroup::builder()
            .title("Server")
            .description("Connection details for the remote host.")
            .build();

        
        let server_icon = gtk::Image::from_icon_name("network-server-symbolic");
        server_icon.add_css_class("dim-label");

        let name_entry = adw::EntryRow::builder().title("Connection Name").build();
        name_entry.add_prefix(&server_icon);
        server_group.add(&name_entry);

        let host_icon = gtk::Image::from_icon_name("network-workgroup-symbolic");
        host_icon.add_css_class("dim-label");

        let host_entry = adw::EntryRow::builder()
            .title("Hostname / IP Address")
            .build();
        host_entry.add_prefix(&host_icon);
        server_group.add(&host_entry);

        
        let port_icon = gtk::Image::from_icon_name("preferences-system-network-symbolic");
        port_icon.add_css_class("dim-label");

        let port_adj = gtk::Adjustment::new(22.0, 1.0, 65535.0, 1.0, 10.0, 0.0);
        let port_entry = gtk::SpinButton::builder()
            .adjustment(&port_adj)
            .valign(gtk::Align::Center)
            .numeric(true)
            .width_chars(6)
            .build();

        let port_row = adw::ActionRow::builder()
            .title("Port")
            .subtitle("Default: 22")
            .build();
        port_row.add_prefix(&port_icon);
        port_row.add_suffix(&port_entry);
        server_group.add(&port_row);

        main_box.append(&server_group);

        let auth_group = adw::PreferencesGroup::builder()
            .title("Authentication")
            .description("Credentials used to log in to the server.")
            .build();

        let user_icon = gtk::Image::from_icon_name("avatar-default-symbolic");
        user_icon.add_css_class("dim-label");

        let user_entry = adw::EntryRow::builder().title("Username").build();
        user_entry.add_prefix(&user_icon);
        auth_group.add(&user_entry);

        let pass_icon = gtk::Image::from_icon_name("dialog-password-symbolic");
        pass_icon.add_css_class("dim-label");

        let pass_entry = adw::PasswordEntryRow::builder().title("Password").build();
        pass_entry.add_prefix(&pass_icon);
        auth_group.add(&pass_entry);

        
        let key_icon = gtk::Image::from_icon_name("channel-secure-symbolic");
        key_icon.add_css_class("dim-label");

        let identity_entry = gtk::Entry::builder()
            .placeholder_text("Optional \u{2014} select SSH key file")
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();
        identity_entry.add_css_class("flat");

        let identity_button = gtk::Button::builder()
            .icon_name("document-open-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Browse for SSH key")
            .css_classes(["flat"])
            .build();

        let identity_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .valign(gtk::Align::Center)
            .build();
        identity_box.append(&identity_entry);
        identity_box.append(&identity_button);

        let identity_row = adw::ActionRow::builder()
            .title("Identity File")
            .subtitle("SSH private key for key-based auth")
            .build();
        identity_row.add_prefix(&key_icon);
        identity_row.add_suffix(&identity_box);
        auth_group.add(&identity_row);

        main_box.append(&auth_group);

        let hint_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_start(4)
            .build();
        let hint_icon = gtk::Image::from_icon_name("dialog-information-symbolic");
        hint_icon.add_css_class("dim-label");
        let hint_label = gtk::Label::builder()
            .label("Passwords are stored securely in your system keyring.")
            .css_classes(["dim-label", "caption"])
            .wrap(true)
            .xalign(0.0)
            .build();
        hint_box.append(&hint_icon);
        hint_box.append(&hint_label);
        main_box.append(&hint_box);

        clamp.set_child(Some(&main_box));
        scroll.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&scroll));
        window.set_content(Some(&toolbar_view));

        Self {
            window,
            title_widget,
            name_entry,
            host_entry,
            port_entry,
            user_entry,
            pass_entry,
            identity_entry,
            identity_button,
            save_button,
        }
    }

    pub fn prepopulate(&self, conn: &SshProfile, password: Option<&str>) {
        self.window.set_title(Some("Edit Connection"));
        self.title_widget.set_title("Edit Connection");
        self.save_button.set_label("Update");

        self.name_entry.set_text(&conn.name);
        self.host_entry.set_text(&conn.host);
        self.port_entry.set_value(conn.port as f64);
        self.user_entry.set_text(&conn.username);
        if let Some(p) = password {
            self.pass_entry.set_text(p);
        }
        if let Some(id_file) = &conn.identity_file {
            self.identity_entry.set_text(&id_file.to_string_lossy());
        }
    }
}
