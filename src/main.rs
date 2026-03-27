use gtk4 as gtk;
use libadwaita as adw;
use vte4 as vte;

use gtk::prelude::*;
use adw::prelude::*;
use vte::prelude::*;

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::rc::Rc;
use secrecy::SecretString;

use ssh_snap::app::AppState;
use ssh_snap::ui::window::{Window, create_terminal};
use ssh_snap::ui::dialogs::add_dialog::AddConnectionDialog;
use ssh_snap::models::SshProfile;
use ssh_snap::core::ssh::spawn_ssh_in_terminal;

#[tokio::main]
async fn main() -> glib::ExitCode {
    if let Err(e) = adw::init() {
        eprintln!("Failed to initialize libadwaita: {}", e);
        return glib::ExitCode::FAILURE;
    }
    
    let provider = gtk::CssProvider::new();
    provider.load_from_string("
        button.success { background-color: #26a269; color: white; }
        button.success:hover { background-color: #2ec27e; }
        row:selected button.success { outline: 1px solid rgba(255,255,255,0.5); background-color: #33d17a; }
    ");

    if let Some(display) = gtk::gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    } else {
        eprintln!("No display found during initialization");
    }
    
    let app_state = match AppState::new(None) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("Initialization failure: {}", e);
            return glib::ExitCode::FAILURE;
        }
    };
    
    let app = adw::Application::builder()
        .application_id("com.sudopydev.ssh-snap")
        .build();

    let active_terminals = Arc::new(RwLock::new(HashMap::new()));

    let state_activate = app_state.clone();
    let terms_activate = active_terminals.clone();
    app.connect_activate(move |a| {
        let settings = state_activate.data.read().map(|d| d.settings.clone()).unwrap_or_default();
        
        if settings.lock_enabled {
            let state_lock = state_activate.clone();
            let app_lock = a.clone();
            let terms_lock = terms_activate.clone();
            show_lock_screen(&a.clone(), state_activate.clone(), {
                let s = state_lock.clone();
                let t = terms_lock.clone();
                move || build_ui(&app_lock, s.clone(), t.clone())
            });
        } else {
            build_ui(a, state_activate.clone(), terms_activate.clone());
        }
    });
    
    setup_actions(&app, app_state.clone(), active_terminals.clone());
    app.run()
}

fn setup_actions(app: &adw::Application, _: Arc<AppState>, _: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>) {
    let help_action = gio::SimpleAction::new("help", None);
    help_action.connect_activate(move |_, _| {
        let help_window = adw::Window::builder()
            .default_width(500)
            .default_height(600)
            .title("SshSnap Help")
            .modal(true)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        let header_bar = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new("SshSnap Help", "User Guide"))
            .show_end_title_buttons(true)
            .build();
        
        toolbar_view.add_top_bar(&header_bar);

        let status_page = adw::StatusPage::builder()
            .title("Getting Started with SshSnap")
            .icon_name("help-faq-symbolic")
            .description(
                 "• <b>Add Servers</b>: Click the <b>+</b> button in the header bar to create a profile.\n\n\
                  • <b>Quick Connect</b>: Click the <b>Play/Connect</b> button next to any server to open its terminal.\n\n\
                  • <b>Manage Profiles</b>: Use the <b>Edit</b> (pencil) and <b>Delete</b> (trash) buttons next to each connection to modify your settings.\n\n\
                  • <b>Search &amp; Filter</b>: Click the search icon to instantly filter through your connection list.\n\n\
                  • <b>Layouts</b>: Toggle between the <b>Grid Dashboard</b> and <b>Sidebar List</b> to suit your workflow."
            )
            .build();

        toolbar_view.set_content(Some(&status_page));
        help_window.set_content(Some(&toolbar_view));
        help_window.present();
    });
    app.add_action(&help_action);

    let about_action = gio::SimpleAction::new("about", None);
    let app_weak = app.downgrade();
    about_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            let dialog = adw::AboutDialog::builder()
                .application_name("SshSnap")
                .version("1.0.0")
                .developer_name("sudo-py-dev")
                .comments("A modern SSH connection manager for Linux.")
                .build();
            dialog.add_link("GitHub Repository", "https://github.com/sudo-py-dev/ssh-snap");
            dialog.add_link("GitHub License", "https://github.com/sudo-py-dev/ssh-snap/blob/main/LICENSE");
            dialog.present(app.active_window().as_ref());
        }
    });
    app.add_action(&about_action);

    let preferences_action = gio::SimpleAction::new("preferences", None);
    app.add_action(&preferences_action);
}

fn build_ui(app: &adw::Application, state: Arc<AppState>, active_terminals: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>) {
    let window = Rc::new(Window::new(app));
    let settings = state.data.read().map(|d| d.settings.clone()).unwrap_or_default();
    let style_manager = adw::StyleManager::default();

    match settings.theme.as_str() {
        "light" => style_manager.set_color_scheme(adw::ColorScheme::ForceLight),
        "dark" => style_manager.set_color_scheme(adw::ColorScheme::ForceDark),
        _ => style_manager.set_color_scheme(adw::ColorScheme::Default),
    }

    if settings.layout_mode == "cards" {
        window.layout_toggle.set_active(true);
        window.main_stack.set_visible_child(&window.dashboard_view);
    }

    refresh_ui(window.clone(), state.clone(), active_terminals.clone());

    let s_select = state.clone();
    let w_select = window.clone();
    let t_select = active_terminals.clone();
    window.list_box.connect_row_activated(move |_, row| {
        if let Some(p) = s_select.get_profile_at(row.index() as usize) {
            initiate_ssh_snapion(w_select.clone(), &p, s_select.clone(), t_select.clone());
        }
    });

    let s_add = state.clone();
    let w_add = window.clone();
    let t_add = active_terminals.clone();
    window.add_button.connect_clicked(move |_| {
        let dialog = Rc::new(AddConnectionDialog::new(&w_add.window));
        setup_dialog_file_browse(&dialog);

        let s_save = s_add.clone();
        let w_refresh = w_add.clone();
        let t_refresh = t_add.clone();
        let d_save = dialog.clone();
        dialog.save_button.connect_clicked(move |_| {
            let name = d_save.name_entry.text().to_string();
            let host = d_save.host_entry.text().to_string();
            let port_val = d_save.port_entry.value();
            let port = if port_val >= 1.0 && port_val <= 65535.0 {
                port_val as u16
            } else {
                22 // Default fallback
            };
            let user = d_save.user_entry.text().to_string();
            let pass = SecretString::new(d_save.pass_entry.text().to_string());
            let identity_file = d_save.identity_entry.text().to_string();
            
            if name.is_empty() || host.is_empty() || user.is_empty() { return; }

            let profile = SshProfile {
                id: uuid::Uuid::new_v4(),
                name,
                host,
                port,
                username: user,
                identity_file: if identity_file.is_empty() { None } else { Some(std::path::PathBuf::from(identity_file)) },
            };

            if let Err(e) = s_save.add_profile(profile.clone()) {
                log::error!("Save failure: {}", e);
                return;
            }
            use secrecy::ExposeSecret;
            if !pass.expose_secret().is_empty() { 
                if let Err(e) = s_save.storage.save_password(&profile.id.to_string(), &pass) {
                    log::warn!("Failed to save password to keyring: {}", e);
                }
            }

            refresh_ui(w_refresh.clone(), s_save.clone(), t_refresh.clone());
            d_save.window.close();
        });
        dialog.window.present();
    });

    let w_search = window.clone();
    window.search_button.connect_toggled(move |_| w_search.toggle_sidebar());

    let w_layout = window.clone();
    let s_layout = state.clone();
    window.layout_toggle.connect_toggled(move |btn| {
        let new_mode = if btn.is_active() {
            w_layout.main_stack.set_visible_child(&w_layout.dashboard_view);
            btn.set_icon_name("view-list-bullet-symbolic");
            "cards"
        } else {
            w_layout.main_stack.set_visible_child(&w_layout.split_view);
            btn.set_icon_name("view-grid-symbolic");
            "sidebar"
        };
        
        if let Ok(mut data) = s_layout.data.write() {
            data.settings.layout_mode = new_mode.to_string();
            drop(data);
            let _ = s_layout.save_settings();
        }
    });

    if let Some(action) = app.lookup_action("preferences") {
        if let Some(sa) = action.downcast_ref::<gio::SimpleAction>() {
            let s_pref = state.clone();
            let t_pref = active_terminals.clone();
            let w_pref = window.clone();
            sa.connect_activate(move |_, _| show_preferences_dialog(s_pref.clone(), t_pref.clone(), w_pref.clone()));
        }
    }
    window.window.present();
}

fn setup_dialog_file_browse(dialog: &Rc<AddConnectionDialog>) {
    let identity_entry = dialog.identity_entry.clone();
    let dialog_window = dialog.window.clone();
    dialog.identity_button.connect_clicked(move |_| {
        let fd = gtk::FileDialog::builder().title("Select SSH Key").modal(true).build();
        let entry = identity_entry.clone();
        fd.open(Some(&dialog_window), None::<&gio::Cancellable>, move |res| {
            if let Ok(file) = res {
                if let Some(path) = file.path() { entry.set_text(&path.to_string_lossy()); }
            }
        });
    });
}

fn initiate_ssh_snapion(window: Rc<Window>, profile: &SshProfile, state: Arc<AppState>, terminals: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>) {
    if let Some(term) = terminals.read().ok().and_then(|t| t.get(&profile.id).cloned()) {
        window.show_terminal(&profile.id.to_string(), &term);
        term.grab_focus();
        window.main_stack.set_visible_child_name("sidebar");
        return;
    }

    let terminal = {
        let settings = state.data.read().map(|d| d.settings.clone()).unwrap_or_default();
        let term = create_terminal(settings.terminal_bg_color.clone(), settings.terminal_fg_color.clone());
        if let Ok(mut t) = terminals.write() { t.insert(profile.id, term.clone()); }
        term
    };

    window.main_stack.set_visible_child_name("sidebar");
    window.show_connecting(&profile.name);

    let window_handle = window.clone();
    let conn_id = profile.id;
    glib::timeout_add_local_once(std::time::Duration::from_millis(500), {
        let terminal = terminal.clone();
        move || {
            window_handle.show_terminal(&conn_id.to_string(), &terminal);
            terminal.grab_focus();
        }
    });

    let window_exit = window.clone();
    let state_exit = state.clone();
    let terminals_exit = terminals.clone();
    let terminal_v = terminal.clone();
    terminal.connect_child_exited(move |_, _| {
        if window_exit.layout_toggle.is_active() {
            window_exit.main_stack.set_visible_child(&window_exit.dashboard_view);
        } else {
            window_exit.show_status_page();
        }
        window_exit.terminal_stack.remove(&terminal_v);
        if let Ok(mut t) = terminals_exit.write() { t.remove(&conn_id); }
        refresh_ui(window_exit.clone(), state_exit.clone(), terminals_exit.clone());
    });

    spawn_ssh_in_terminal(&terminal, profile);
    terminal.grab_focus();
    refresh_ui(window, state, terminals);
}

fn handle_connection_action(profile: &SshProfile, window: Rc<Window>, state: Arc<AppState>, terminals: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>) {
    let term_to_remove = if let Ok(mut t) = terminals.write() { t.remove(&profile.id) } else { None };
    
    if let Some(term) = term_to_remove {
        window.terminal_stack.remove(&term);
        window.show_status_page();
        refresh_ui(window, state, terminals);
    } else {
        initiate_ssh_snapion(window, profile, state, terminals);
    }
}

fn refresh_ui(window: Rc<Window>, state: Arc<AppState>, terminals: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>) {
    window.clear_connections();
    while let Some(child) = window.dashboard_flow.first_child() { window.dashboard_flow.remove(&child); }

    let profiles = state.data.read().map(|d| d.profiles.clone()).unwrap_or_default();
    if profiles.is_empty() {
        window.dashboard_stack.set_visible_child_name("empty");
        window.sidebar_stack.set_visible_child_name("empty");
        window.terminal_stack.set_visible_child_name("status");
        window.status_page.set_description(None);
        return;
    }
    window.sidebar_stack.set_visible_child_name("list");
    window.status_page.set_description(Some("Select a connection from the sidebar to open a terminal."));
    window.dashboard_stack.set_visible_child_name("flow");

    for profile in profiles {
        let is_active = terminals.read().map(|t| t.contains_key(&profile.id)).unwrap_or(false);
        let (s_conn, s_edit, s_del) = window.add_connection_row(&profile, is_active);
        let (c_conn, c_edit, c_del) = window.add_dashboard_card(&profile, is_active);

        for (cb, eb, db) in [(s_conn, s_edit, s_del), (c_conn, c_edit, c_del)] {
            let p = profile.clone(); let w = window.clone(); let s = state.clone(); let t = terminals.clone();
            cb.connect_clicked({ let p = p.clone(); let w = w.clone(); let s = s.clone(); let t = t.clone(); move |_| handle_connection_action(&p, w.clone(), s.clone(), t.clone()) });
            eb.connect_clicked({ let p = p.clone(); let w = w.clone(); let s = s.clone(); let t = t.clone(); move |_| show_edit_dialog(w.clone(), s.clone(), p.clone(), t.clone()) });
                db.connect_clicked({ let p = p.clone(); let w = w.clone(); let s = s.clone(); let t = t.clone(); move |_| {
                    if let Err(e) = s.delete_profile(&p.id) { eprintln!("Delete error: {}", e); return; }
                    let term = if let Ok(mut t_mut) = t.write() { t_mut.remove(&p.id) } else { None };
                    if let Some(term) = term { w.terminal_stack.remove(&term); }
                    refresh_ui(w.clone(), s.clone(), t.clone());
                }});
        }
    }
}

fn show_edit_dialog(window: Rc<Window>, state: Arc<AppState>, profile: SshProfile, terminals: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>) {
    use secrecy::ExposeSecret;
    let password_opt = state.storage.get_password(&profile.id.to_string()).unwrap_or(None);
    let dialog = Rc::new(AddConnectionDialog::new(&window.window));
    dialog.prepopulate(&profile, password_opt.as_ref().map(|s| s.expose_secret().as_str()));
    setup_dialog_file_browse(&dialog);

    let s_save = state.clone(); let w_ref = window.clone(); let t_ref = terminals.clone(); let p_id = profile.id; let d_h = dialog.clone();
    dialog.save_button.connect_clicked(move |_| {
        let port_val = d_h.port_entry.value();
        let port = if port_val >= 1.0 && port_val <= 65535.0 {
            port_val as u16
        } else {
            22
        };

        let profile = SshProfile {
            id: p_id,
            name: d_h.name_entry.text().to_string(),
            host: d_h.host_entry.text().to_string(),
            port,
            username: d_h.user_entry.text().to_string(),
            identity_file: { let t = d_h.identity_entry.text().to_string(); if t.is_empty() { None } else { Some(std::path::PathBuf::from(t)) } },
        };
        if let Err(e) = s_save.update_profile(profile) { log::error!("Update failure: {}", e); return; }
        
        let pass_text = d_h.pass_entry.text().to_string();
        if !pass_text.is_empty() { 
            let secret_pass = SecretString::new(pass_text);
            if let Err(e) = s_save.storage.save_password(&p_id.to_string(), &secret_pass) {
                log::warn!("Failed to update password in keyring: {}", e);
            }
        }
        refresh_ui(w_ref.clone(), s_save.clone(), t_ref.clone());
        d_h.window.close();
    });
    dialog.window.present();
}

fn show_preferences_dialog(state: Arc<AppState>, terminals: Arc<RwLock<HashMap<uuid::Uuid, vte::Terminal>>>, _: Rc<Window>) {
    let pw = adw::PreferencesWindow::builder().title("Preferences").modal(true).default_width(500).build();
    let page = adw::PreferencesPage::builder().title("General").icon_name("settings-symbolic").build();

    let settings = state.data.read().map(|d| d.settings.clone()).unwrap_or_default();

        
    let appearance_group = adw::PreferencesGroup::builder().title("Appearance").build();
    let theme_row = adw::ComboRow::builder().title("Color Scheme").model(&gtk::StringList::new(&["System Default", "Light", "Dark"])).build();
    theme_row.set_selected(match settings.theme.as_str() { "light" => 1, "dark" => 2, _ => 0 });

    let s_theme = state.clone();
    theme_row.connect_selected_item_notify(move |c| {
        let sm = adw::StyleManager::default();
        if let Ok(mut data) = s_theme.data.write() {
            match c.selected() {
                1 => { sm.set_color_scheme(adw::ColorScheme::ForceLight); data.settings.theme = "light".to_string(); }
                2 => { sm.set_color_scheme(adw::ColorScheme::ForceDark); data.settings.theme = "dark".to_string(); }
                _ => { sm.set_color_scheme(adw::ColorScheme::Default); data.settings.theme = "default".to_string(); }
            }
            drop(data); let _ = s_theme.save_settings();
        }
    });
    appearance_group.add(&theme_row);

        
    let terminal_group = adw::PreferencesGroup::builder().title("Terminal").build();
    
    let color_dialog = gtk::ColorDialog::new();
    let bg_row = adw::ActionRow::builder().title("Background Color").build();
    let bg_btn = gtk::ColorDialogButton::builder().dialog(&color_dialog).valign(gtk::Align::Center).build();
    if let Some(rgba) = settings.terminal_bg_color.as_deref().and_then(|c| gtk::gdk::RGBA::parse(c).ok()) { bg_btn.set_rgba(&rgba); }
    
    let s_bg = state.clone(); let t_bg = terminals.clone();
    bg_btn.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        let hex = format!("#{:02X}{:02X}{:02X}", (rgba.red() * 255.0) as u8, (rgba.green() * 255.0) as u8, (rgba.blue() * 255.0) as u8);
        if let Ok(mut data) = s_bg.data.write() {
            data.settings.terminal_bg_color = Some(hex.clone());
            drop(data); let _ = s_bg.save_settings();
            
            if let Ok(terms) = t_bg.read() {
                for term in terms.values() { term.set_color_background(&rgba); }
            }
        }
    });
    bg_row.add_suffix(&bg_btn);

    let fg_row = adw::ActionRow::builder().title("Text Color").build();
    let fg_btn = gtk::ColorDialogButton::builder().dialog(&color_dialog).valign(gtk::Align::Center).build();
    if let Some(rgba) = settings.terminal_fg_color.as_deref().and_then(|c| gtk::gdk::RGBA::parse(c).ok()) { fg_btn.set_rgba(&rgba); }
    
    let s_fg = state.clone(); let t_fg = terminals.clone();
    fg_btn.connect_rgba_notify(move |btn| {
        let rgba = btn.rgba();
        let hex = format!("#{:02X}{:02X}{:02X}", (rgba.red() * 255.0) as u8, (rgba.green() * 255.0) as u8, (rgba.blue() * 255.0) as u8);
        if let Ok(mut data) = s_fg.data.write() {
            data.settings.terminal_fg_color = Some(hex.clone());
            drop(data); let _ = s_fg.save_settings();
            if let Ok(terms) = t_fg.read() {
                for term in terms.values() { 
                    term.set_color_foreground(&rgba);
                    term.set_color_cursor(Some(&rgba));
                }
            }
        }
    });
    fg_row.add_suffix(&fg_btn);

    terminal_group.add(&bg_row); terminal_group.add(&fg_row);

        
    let security_group = adw::PreferencesGroup::builder().title("Security").build();
    let lock_row = adw::SwitchRow::builder().title("App Lock").subtitle("Require system password on startup").active(settings.lock_enabled).build();
    

    let s_lock = state.clone();
    let pw_handle = pw.clone();
    lock_row.connect_active_notify(move |r| {
        let is_active = r.is_active();
        let currently_enabled = s_lock.data.read().map(|d| d.settings.lock_enabled).unwrap_or(false);
        
        if is_active && !currently_enabled {
            let r_v = r.clone(); let s_v = s_lock.clone();
            r_v.set_active(false);
            show_system_auth_confirmation(&pw_handle.clone().upcast::<gtk::Window>(), s_v.clone(), move || {
                r_v.set_active(true);
                if let Ok(mut data) = s_v.data.write() {
                    data.settings.lock_enabled = true;
                    drop(data); 
                    
                    let _ = s_v.save_profiles();
                    let _ = s_v.save_settings();
                }
            });
        } else if !is_active && currently_enabled {
            
            if let Ok(mut data) = s_lock.data.write() {
                data.settings.lock_enabled = false;
                drop(data); 
                
                let _ = s_lock.save_settings();
                if let Ok(mut k) = s_lock.storage.encryption_key.write() { *k = None; }
            }
        }
    });

    security_group.add(&lock_row); 

    page.add(&appearance_group); page.add(&terminal_group); page.add(&security_group);
    pw.add(&page); pw.present();
}

fn show_lock_screen(app: &adw::Application, state: Arc<AppState>, on_unlocked: impl Fn() + 'static) {
    let window = adw::Window::builder().application(app).title("Authentication Required").modal(true).default_width(400).resizable(false).build();
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_top(24).margin_bottom(24).margin_start(24).margin_end(24).spacing(18).build();
    let icon = gtk::Image::builder().icon_name("dialog-password-symbolic").pixel_size(64).css_classes(["dim-label"]).build();
    let title = gtk::Label::builder().label("Authentication Required").css_classes(["title-1"]).build();
    let subtitle = gtk::Label::builder().label("The application is locked and requires your system password to decrypt connection data.").wrap(true).justify(gtk::Justification::Center).build();
    let entry = gtk::PasswordEntry::builder().placeholder_text("System Password").activates_default(true).build();
    let unlock_btn = gtk::Button::builder().label("Unlock").css_classes(["pill", "suggested-action"]).build();

    content.append(&icon); content.append(&title); content.append(&subtitle); content.append(&entry); content.append(&unlock_btn);

    let s = state.clone(); let w = window.clone(); let on_unlock = Rc::new(on_unlocked);
    let entry_u = entry.clone();
    let unlock_fn = Rc::new(move || {
        let pass = SecretString::new(entry_u.text().to_string());
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        
        if s.storage.verify_system_password(&user, &pass) {
            if let Err(e) = s.storage.setup_encryption(&pass) {
                log::error!("Encryption setup failed: {}", e);
                entry_u.add_css_class("error");
                return;
            }
            if let Ok(profiles) = s.storage.load_profiles() {
                if let Ok(mut data) = s.data.write() { data.profiles = profiles; }
            }
            w.close();
            on_unlock();
        } else {
            entry_u.add_css_class("error");
            entry_u.set_text("");
        }
    });

    let u1 = unlock_fn.clone(); unlock_btn.connect_clicked(move |_| u1());
    let u2 = unlock_fn.clone(); entry.connect_activate(move |_| u2());
    window.set_content(Some(&content)); window.present();
}

fn show_system_auth_confirmation(parent: &gtk::Window, state: Arc<AppState>, on_success: impl Fn() + 'static) {
    let window = adw::Window::builder().transient_for(parent).modal(true).title("Security Setup").default_width(380).resizable(false).build();
    let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).margin_top(20).margin_bottom(20).margin_start(20).margin_end(20).spacing(16).build();
    let label = gtk::Label::builder().label("To enable App Lock, please verify your system login password. This password will be used to encrypt your connection profiles.").wrap(true).halign(gtk::Align::Start).build();
    let entry = gtk::PasswordEntry::builder().placeholder_text("Current System Password").activates_default(true).build();
    let btn_box = gtk::Box::builder().spacing(12).halign(gtk::Align::End).build();
    let cancel_btn = gtk::Button::builder().label("Cancel").build();
    let confirm_btn = gtk::Button::builder().label("Enable Lock").css_classes(["suggested-action"]).build();

    btn_box.append(&cancel_btn); btn_box.append(&confirm_btn);
    content.append(&label); content.append(&entry); content.append(&btn_box);
    window.set_content(Some(&content));

    let s = state.clone(); let w = window.clone();
    let on_s = Rc::new(on_success);
    let entry_c = entry.clone();
    
    let verify_fn = Rc::new(move || {
        let pass = SecretString::new(entry_c.text().to_string());
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        if s.storage.verify_system_password(&user, &pass) {
            if let Err(e) = s.storage.setup_encryption(&pass) {
                log::error!("Encryption setup failed: {}", e);
                entry_c.add_css_class("error");
                return;
            }
            w.close(); (*on_s)();
        } else {
            entry_c.add_css_class("error");
            entry_c.set_text("");
        }
    });

    let v1 = verify_fn.clone(); confirm_btn.connect_clicked(move |_| v1());
    let v2 = verify_fn.clone(); entry.connect_activate(move |_| v2());
    let w_close = window.clone(); cancel_btn.connect_clicked(move |_| w_close.close());
    window.present();
}

