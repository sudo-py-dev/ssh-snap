# SshSnap

A modern, high-fidelity SSH connection manager designed exclusively for the Linux desktop (GNOME). Built with **Rust**, **GTK4**, and **Libadwaita**, it provides a seamless, secure, and professional experience for managing remote server access.

![SshSnap UI](/assets/image.png)

## Features

### 🖥️ Native GNOME Experience

- **Libadwaita Design**: Follows the GNOME HIG (Human Interface Guidelines) for a 100% native look and feel.
- **Adaptive UI**: Seamlessly transitions between a **Sidebar (List)** and **Dashboard (Grid)** layout.
- **Dark Mode Support**: Automatically respects your system-wide theme and color scheme.

### 🔒 Enterprise-Grade Security

- **At-Rest Encryption**: All connection profiles are encrypted with **AES-256-GCM**.
- **Argon2id KDF**: Secure password-to-key derivation ensures your data is cryptographically protected by your system password.
- **PAM Authentication**: Integrates with Linux's native **Pluggable Authentication Modules** for identity verification.
- **High-Fidelity Auth**: Real, official-style authentication prompts that mimic the GNOME Shell experience.

### ⌨️ Integrated Terminal

- **VTE4 Integration**: Fast, reliable, and native terminal emulation built directly into the app.
- **Persistent Sessions**: Your terminals stay active as long as the application is open.

## Installation

### Prerequisites

Ensure you have the following system dependencies installed:

- `gtk4`
- `libadwaita-1`
- `vte4`
- `libpam0g-dev`
- `libsecret-1-dev`

### Building from Source

```bash
# Clone the repository
git clone https://github.com/sudo-py-dev/ssh-snap.git

# Navigate to the project directory
cd ssh-snap

# Build the project
cargo build --release

# Run the application
./target/release/ssh-snap
```

## Usage

1. **Add Connection**: Press the **+** button in the header bar to create your first server profile.
2. **Switch Layouts**: Use the grid icon to toggle between a server dashboard and a quick-access sidebar.
3. **App Lock**: Enable high-security encryption in **Preferences > Security** to protect your profiles with your system password.

## Project Structure

- `src/main.rs`: Application entry point and UI controller.
- `src/core/storage.rs`: Secure storage engine (Argon2, AES, PAM).
- `src/models.rs`: Data models for profiles and settings.
- `src/ui/window.rs`: Main Adwaita window construction.
- `src/ui/dialogs/`: UI components for adding/editing connections.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
