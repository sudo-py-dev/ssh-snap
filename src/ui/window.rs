use gtk4 as gtk;
use libadwaita as adw;
use vte4 as vte;

use gtk::prelude::*;
use adw::prelude::*;
use vte::prelude::*;
use crate::models::SshProfile;

pub struct Window {
    pub window: adw::ApplicationWindow,
    pub list_box: gtk::ListBox,
    pub split_view: adw::OverlaySplitView,
    pub sidebar_stack: gtk::Stack,
    pub terminal_stack: adw::ViewStack,
    pub status_page: adw::StatusPage,
    pub connecting_page: adw::StatusPage,
    pub dashboard_view: gtk::Box,
    pub dashboard_flow: gtk::FlowBox,
    pub dashboard_stack: gtk::Stack,
    pub main_stack: gtk::Stack,
    pub add_button: gtk::Button,
    pub search_button: gtk::ToggleButton,
    pub layout_toggle: gtk::ToggleButton,
    pub _menu_button: gtk::MenuButton,
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(1100)
            .default_height(750)
            .title("SshSnap")
            .build();

        let header_bar = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("SshSnap", ""))
            .build();
        
        let add_button = gtk::Button::builder().icon_name("list-add-symbolic").tooltip_text("Add New Connection").build();
        let search_button = gtk::ToggleButton::builder().icon_name("system-search-symbolic").tooltip_text("Search Connections").build();
        let layout_toggle = gtk::ToggleButton::builder().icon_name("view-grid-symbolic").tooltip_text("Toggle Layout").build();

        header_bar.pack_start(&add_button);
        header_bar.pack_end(&search_button);
        header_bar.pack_end(&layout_toggle);

        let menu_model = gio::Menu::new();
        menu_model.append(Some("Preferences"), Some("app.preferences"));
        menu_model.append(Some("Help"), Some("app.help"));
        menu_model.append(Some("About SshSnap"), Some("app.about"));
        let menu_button = gtk::MenuButton::builder().icon_name("open-menu-symbolic").menu_model(&menu_model).build();
        header_bar.pack_end(&menu_button);
        
        let split_view = adw::OverlaySplitView::builder().sidebar_width_fraction(0.28).min_sidebar_width(220.0).build();
        let list_box = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::Single).css_classes(["navigation-sidebar", "sidebar-list"]).build();
        
        let sidebar_stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
        let empty_sidebar = gtk::Box::builder().orientation(gtk::Orientation::Vertical).valign(gtk::Align::Center).margin_start(12).margin_end(12).spacing(8).build();
        empty_sidebar.append(&gtk::Image::builder().icon_name("list-add-symbolic").pixel_size(32).css_classes(["dim-label"]).build());
        empty_sidebar.append(&gtk::Label::builder().label("No Connections").css_classes(["heading"]).build());
        empty_sidebar.append(&gtk::Label::builder().label("add connection by press the pluse").wrap(true).justify(gtk::Justification::Center).css_classes(["dim-label"]).build());

        sidebar_stack.add_named(&gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).child(&list_box).build(), Some("list"));
        sidebar_stack.add_named(&empty_sidebar, Some("empty"));

        split_view.set_sidebar(Some(&sidebar_stack));

        let terminal_stack = adw::ViewStack::builder().hexpand(true).vexpand(true).build();
        let status_page = adw::StatusPage::builder()
            .title("SshSnap Connection Manager")
            .icon_name("network-workgroup-symbolic")
            .build();
        let connecting_page = adw::StatusPage::builder().title("Connecting...").build();
        connecting_page.set_child(Some(&gtk::Spinner::builder().spinning(true).width_request(48).height_request(48).halign(gtk::Align::Center).build()));

        terminal_stack.add_titled(&status_page, Some("status"), "Status");
        terminal_stack.add_titled(&connecting_page, Some("connecting"), "Connecting");
        split_view.set_content(Some(&terminal_stack));

        let dashboard_flow = gtk::FlowBox::builder().valign(gtk::Align::Start).max_children_per_line(4).min_children_per_line(1).selection_mode(gtk::SelectionMode::None).column_spacing(12).row_spacing(12).build();
        let dashboard_stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).vexpand(true).build();
        let dashboard_view = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_top(24).margin_bottom(24).margin_start(24).margin_end(24).spacing(12).build();
        
        let empty_dashboard = adw::StatusPage::builder()
            .title("No Connections Yet")
            .description("Press the '+' button above to add your first SSH server.")
            .icon_name("network-workgroup-symbolic")
            .build();
        
        dashboard_stack.add_named(&gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).child(&dashboard_flow).build(), Some("flow"));
        dashboard_stack.add_named(&empty_dashboard, Some("empty"));

        dashboard_view.append(&gtk::Label::builder().label("Server Dashboard").css_classes(["title-1"]).halign(gtk::Align::Start).build());
        dashboard_view.append(&dashboard_stack);

        let main_stack = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
        main_stack.add_named(&split_view, Some("sidebar"));
        main_stack.add_named(&dashboard_view, Some("cards"));

        let toolbar_view = adw::ToolbarView::builder().content(&main_stack).build();
        toolbar_view.add_top_bar(&header_bar);
        window.set_content(Some(&toolbar_view));
        
        Self { window, list_box, split_view, sidebar_stack, terminal_stack, status_page, connecting_page, dashboard_view, dashboard_flow, dashboard_stack, main_stack, add_button, search_button, layout_toggle, _menu_button: menu_button }
    }

    fn create_action_buttons(&self, is_active: bool) -> (gtk::Button, gtk::Button, gtk::Button) {
        let (icon, color) = if is_active { ("media-playback-stop-symbolic", "destructive-action") } else { ("media-playback-start-symbolic", "success") };
        (
            gtk::Button::builder().icon_name(icon).valign(gtk::Align::Center).css_classes([color]).build(),
            gtk::Button::builder().icon_name("document-edit-symbolic").valign(gtk::Align::Center).css_classes(["flat"]).build(),
            gtk::Button::builder().icon_name("user-trash-symbolic").valign(gtk::Align::Center).css_classes(["flat"]).build()
        )
    }

    pub fn add_connection_row(&self, conn: &SshProfile, is_active: bool) -> (gtk::Button, gtk::Button, gtk::Button) {
        let (c, e, d) = self.create_action_buttons(is_active);
        let row = adw::ActionRow::builder().title(&conn.name).subtitle(&format!("{}@{}:{}", conn.username, conn.host, conn.port)).selectable(true).build();
        row.add_suffix(&c); row.add_suffix(&e); row.add_suffix(&d);
        self.list_box.append(&row);
        (c, e, d)
    }

    pub fn add_dashboard_card(&self, conn: &SshProfile, is_active: bool) -> (gtk::Button, gtk::Button, gtk::Button) {
        let (c, e, d) = self.create_action_buttons(is_active);
        let card_box = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).margin_top(12).margin_bottom(12).margin_start(12).margin_end(12).build();
        let icon = gtk::Image::from_icon_name(if is_active { "network-offline-symbolic" } else { "network-server-symbolic" });
        icon.set_pixel_size(48);
        card_box.append(&icon);
        card_box.append(&gtk::Label::builder().label(&conn.name).css_classes(["heading"]).halign(gtk::Align::Center).build());
        
        let footer = gtk::Box::builder().orientation(gtk::Orientation::Horizontal).spacing(12).halign(gtk::Align::Center).build();
        footer.append(&c); footer.append(&e); footer.append(&d);
        card_box.append(&footer);

        self.dashboard_flow.append(&gtk::Frame::builder().child(&card_box).css_classes(["card"]).build());
        (c, e, d)
    }

    pub fn clear_connections(&self) {
        while let Some(child) = self.list_box.first_child() { self.list_box.remove(&child); }
    }

    pub fn toggle_sidebar(&self) { self.split_view.set_collapsed(!self.split_view.is_collapsed()); }

    pub fn show_terminal(&self, conn_id: &str, terminal: &vte::Terminal) {
        if terminal.parent().is_none() { self.terminal_stack.add_titled(terminal, Some(conn_id), conn_id); }
        self.terminal_stack.set_visible_child(terminal);
    }

    pub fn show_status_page(&self) { self.terminal_stack.set_visible_child_name("status"); }

    pub fn show_connecting(&self, conn_name: &str) {
        self.connecting_page.set_title(&format!("Connecting to {}...", conn_name));
        self.terminal_stack.set_visible_child_name("connecting");
    }
}

pub fn get_foreground_for_background(bg_color: Option<&str>) -> String {
    let bg_rgba = bg_color.and_then(|h| gtk::gdk::RGBA::parse(h).ok());
    if let Some(bg) = bg_rgba {
        let luminance = 0.2126 * bg.red() + 0.7152 * bg.green() + 0.0722 * bg.blue();
        if luminance > 0.4 { "#1e1e1e".into() } else { "#f0f0f0".into() }
    } else {
        "#d0d0d0".into()
    }
}

pub fn create_terminal(bg_color: Option<String>, fg_color: Option<String>) -> vte::Terminal {
    let terminal = vte::Terminal::new();
    let bg_rgba = bg_color.as_deref().and_then(|h| gtk::gdk::RGBA::parse(h).ok());
    
    if let Some(ref bg) = bg_rgba { terminal.set_color_background(bg); }
    
    let effective_fg_hex = fg_color.unwrap_or_else(|| get_foreground_for_background(bg_color.as_deref()));
    let effective_fg = gtk::gdk::RGBA::parse(&effective_fg_hex).unwrap_or_else(|_| {
        gtk::gdk::RGBA::parse("#ffffff").unwrap_or_else(|_| gtk::gdk::RGBA::RED)
    });

    terminal.set_color_foreground(&effective_fg);
    if let Ok(highlight) = gtk::gdk::RGBA::parse("#3584e4") {
        terminal.set_color_highlight(Some(&highlight));
    }
    terminal.set_color_cursor(Some(&effective_fg));
    terminal.set_cursor_blink_mode(vte::CursorBlinkMode::On);
    terminal.set_scrollback_lines(10000);
    terminal.set_hexpand(true);
    terminal.set_vexpand(true);
    terminal.set_font(Some(&pango::FontDescription::from_string("Monospace 11")));
    terminal
}

