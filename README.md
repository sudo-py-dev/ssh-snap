<p align="center">
  <img src="assets/app_icon.png" width="128" height="128" alt="ssh-snap Logo">
</p>

# 🚀 ssh-snap

**A modern, clean SSH manager built for the GNOME desktop.**  
Built with 🦀 **Rust**, **GTK4**, and **Libadwaita**, `ssh-snap` is fast, safe, and easy to use.

---

## ✨ Key Features

### 🎨 Native Feel
*   **Modern Design**: Looks and feels like a native part of GNOME.
*   **Simple Layouts**: Easily switch between a **Dashboard** 📊 and a **Sidebar** 🗂️.
*   **Dark Mode**: Automatically matches your system colors.

### 🔐 Safe and Sound
*   **Strong Encryption**: Your data is securely locked with **AES-256-GCM** 🛡️.
*   **Trusted Security**: Uses **Argon2id** to make sure your keys are extra safe.
*   **Keychain Ready**: Saves your passwords safely in your system's built-in key storage 🔑.
*   **Identity Check**: Works with standard Linux tools to verify who you are.

### ⚡ Built-in Terminal
*   **Fast Connection**: A smooth, snappy terminal right inside the app ⌨️.
*   **Stay Organized**: Keep all your server sessions open and easy to find in one place.

---

## 📦 Installation

### 📥 Debian / Ubuntu (Recommended)
You can now install the pre-compiled `.deb` package directly:

```bash
sudo dpkg -i target/debian/ssh-snap_1.0.0-1_amd64.deb
sudo apt-get install -f  # Install missing dependencies
```

### 🛠️ Building from Source
Ensure you have the required development headers: `libgtk-4-dev`, `libadwaita-1-dev`, `libvte-2.91-gtk4-dev`, `libpam0g-dev`.

```bash
# Clone and Build
git clone https://github.com/sudo-py-dev/ssh-snap.git
cd ssh-snap
cargo build --release

# Run
./target/release/ssh-snap
```

---

## 🚀 Quick Start
1.  **Add a Server**: Click the **+** (Plus) button in the header bar.
2.  **Authenticate**: Enter your credentials. If "Secure Store" is enabled, your data is AES-encrypted at rest.
3.  **Connect**: Double-click any profile to launch an integrated SSH session immediately.

---

## 📂 Project Architecture
*   `src/core/`: Security engine (Encryption, PAM, Keyring).
*   `src/ui/`: GTK4/Adwaita components and window management.
*   `src/models/`: Robust data structures and persistence logic.

---

## 📜 License
Licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

---
<p align="center">
  Made with ❤️ for the Linux Community.
</p>
