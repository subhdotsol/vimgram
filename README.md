# ⚡️ Vimgram

**A blazing fast, Vim-native Telegram client for your terminal.**

Vimgram fills the gap between heavy GUI clients and limited CLI tools. It brings the full power of Telegram DMs, groups, and channels into your terminal with a focus on **speed** and **keyboard-driven efficiency**.

---

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

## 🛠 Installation & Setup

### Prerequisites
- **Rust** (latest stable)
- A Telegram **API ID** and **API Hash** (get them from [my.telegram.org](https://my.telegram.org))

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

## 🎮 Keybindings

Vimgram is modal, just like Vim.

### **NORMAL Mode** (Default)
| Key | Action |
|:---:|---|
| `j` / `k` | Scroll history **down** (newer) / **up** (older) |
| `h` / `l` | Switch focus between **Friends List** and **Chat** |
| `i` | Enter **INSERT** mode (start typing) |
| `q` | Quit Vimgram |

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
