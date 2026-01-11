# ⚡️ Vimgram

**A blazing fast, Vim-native Telegram client for your terminal.**

Vimgram fills the gap between heavy GUI clients and limited CLI tools. It brings the full power of Telegram DMs, groups, and channels into your terminal with a focus on **speed** and **keyboard-driven efficiency**.

---

# Demo



https://github.com/user-attachments/assets/2558c324-24fe-4c79-814c-6fbabb741f04



<img width="2560" height="1439" alt="Vimgram Screenshot" src="https://github.com/user-attachments/assets/7429db5e-c01a-47bf-ac97-e8f2fbdf6244" />

## ✨ Features

- **🚀 Instant Startup**: Lazy-loads chats for immediate access, handling hundreds of conversations without breaking a sweat.
- **⚡️ Real-time**: Messages stream in effectively instantly. No manual refreshing needed.
- **⌨️ Vim-Native**: Navigate entirely with `hjkl`. If you know Vim, you already know Vimgram.
- **📜 Smart Scrolling**:
  - Auto-scrolls to the newest message.
  - "Stick-to-bottom" behavior while reading live chats.
  - Infinite history scrolling (up/down).
- **🔒 Secure**: Full MTProto encryption using `grammers`. Supports 2FA (Password) login.
- **🎨 Beautiful TUI**: Clean, bottom-aligned chat view with color-coded senders and robust handling of emojis/formatting.

---

## 📋 Prerequisites

Before installing, you'll need a Telegram **API ID** and **API Hash**:
1. Log in to your Telegram account at [my.telegram.org/apps](https://my.telegram.org/apps).
2. Go to **API development tools**.
3. Create a new application (the details don't matter much).
4. Copy your **App api_id** and **App api_hash**.

---

## 🛠 Installation

### Option 1: Install via Cargo (Recommended)
If you have Rust installed, just run:
```bash
cargo install vimgram
```
Then run it:
```bash
vimgram
```

### Option 2: Build from Source
**Prerequisites**
- **Rust** (latest stable)
- Your **API ID** and **API Hash** (see above)

### 1. Clone & Config
```bash
git clone https://github.com/subhdotsol/vimgram.git
cd vimgram

# Create .env file
echo "TELEGRAM_API_ID=123456" >> .env
echo "TELEGRAM_API_HASH=your_api_hash" >> .env
```

### 2. Run
```bash
cargo run --release
```
*On first run, you will be prompted to enter your phone number and login code.*

---

## 🛡 Security & Privacy

We take your privacy seriously. Vimgram is designed with a "trust no one" architecture:

- **Open Source**: The entire codebase is open and transparent. You can inspect exactly how your data is handled.
- **Local Storage**: Your `session.dat` and API credentials are stored **locally** on your machine (in your OS-standard configuration directory). They are **never** sent to us or any third-party server.
- **Direct Connection**: Vimgram facilitates a direct connection between your machine and Telegram's official MTProto servers. There is no middleman backend.
- **Your Keys, Your Control**: We ask for your own API ID/Hash so that you are in full control of your session and not subject to shared rate limits.

---

## 🎮 Keybindings

Vimgram is modal, just like Vim.

### **NORMAL Mode** (Default)
| Key | Action |
|:---:|---|
| `j` / `k` | Scroll history **down** (newer) / **up** (older) |
| `h` / `l` | Switch focus between **Friends List** and **Chat** |
| `/` | Enter **SEARCH** mode (filter friends list) |
| `:` | Enter **COMMAND** mode |
| `i` | Enter **INSERT** mode (start typing) |
| `q` | Quit Vimgram |

### **COMMAND Mode**
| Key | Action |
|:---:|---|
| `:find @user` | Search for **any** Telegram user by username |
| `:q` | Quit Vimgram |
| `Esc` | Cancel, return to **NORMAL** |

### **SEARCH Mode**
| Key | Action |
|:---:|---|
| `Type` | Filter friends by name |
| `↑` / `↓` | Navigate filtered results |
| `Enter` | **Jump** to selected chat |
| `Esc` | Cancel search, return to **NORMAL** |

### **INSERT Mode**
| Key | Action |
|:---:|---|
| `Type` | Type your message |
| `Enter` | **Send** message |
| `Esc` | Return to **NORMAL** mode |

---

## 🏗 Architecture

Vimgram is built on a robust Rust stack:
- **[Grammers](https://github.com/Lonami/grammers)**: Pure Rust MTProto client implementation.
- **[Ratatui](https://github.com/ratatui-org/ratatui)**: Advanced terminal UI rendering.
- **[Tokio](https://tokio.rs)**: Async runtime for handling concurrent updates and input.

### Project Structure
- `src/main.rs`: Entry point & event loop.
- `src/app.rs`: State management (Redux-style).
- `src/ui/`: Drawing logic & layout.
- `src/telegram/`: Auth & networking layer.

---

<p align="center">
  <i>Built with ❤️ in Rust</i>
</p>
