use std::collections::HashMap;

/// Application mode (Vim-style)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
}

/// Which panel is focused
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Friends,
    Chats,
}

/// A chat/contact in the friends list
#[derive(Debug, Clone)]
pub struct Chat {
    pub id: i64,
    pub name: String,
    pub last_message: Option<String>,
    pub unread: u32,
}

/// A message in a chat
#[derive(Debug, Clone)]
pub struct Message {
    pub sender: String,
    pub text: String,
    pub outgoing: bool,
}

/// Main application state
pub struct App {
    pub mode: Mode,
    pub panel: Panel,
    pub chats: Vec<Chat>,
    pub messages: HashMap<i64, Vec<Message>>,
    pub selected_chat: usize,
    pub selected_message: usize,
    pub scroll_offset: usize,
    pub input: String,
    pub should_quit: bool,
    pub reload_requested: bool,
    pub loading_status: Option<String>,
    pub needs_message_load: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            panel: Panel::Friends,
            chats: Vec::new(),
            messages: HashMap::new(),
            selected_chat: 0,
            selected_message: 0,
            scroll_offset: 0,
            input: String::new(),
            should_quit: false,
            reload_requested: false,
            loading_status: None,
            needs_message_load: true,
        }
    }

    /// Get currently selected chat ID
    pub fn current_chat_id(&self) -> Option<i64> {
        self.chats.get(self.selected_chat).map(|c| c.id)
    }

    /// Get messages for currently selected chat
    pub fn current_messages(&self) -> Vec<&Message> {
        if let Some(id) = self.current_chat_id() {
            self.messages.get(&id).map(|m| m.iter().collect()).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Clear unread count for currently selected chat
    pub fn clear_current_unread(&mut self) {
        if let Some(chat) = self.chats.get_mut(self.selected_chat) {
            chat.unread = 0;
        }
    }

    /// Move selection up in the current panel
    pub fn move_up(&mut self) {
        match self.panel {
            Panel::Friends => {
                if self.selected_chat > 0 {
                    self.selected_chat -= 1;
                    self.clear_current_unread();
                    self.scroll_offset = 0; // Reset scroll when switching chats
                    self.needs_message_load = true; // Trigger lazy loading
                }
            }
            Panel::Chats => {
                // Scroll up (back in history)
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
        }
    }

    /// Move selection down in the current panel
    pub fn move_down(&mut self) {
        match self.panel {
            Panel::Friends => {
                if self.selected_chat < self.chats.len().saturating_sub(1) {
                    self.selected_chat += 1;
                    self.clear_current_unread();
                    self.scroll_offset = 0; // Reset scroll when switching chats
                    self.needs_message_load = true; // Trigger lazy loading
                }
            }
            Panel::Chats => {
                // Scroll down (forward in history)
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
        }
    }

    /// Switch between panels
    pub fn switch_panel(&mut self) {
        self.panel = match self.panel {
            Panel::Friends => Panel::Chats,
            Panel::Chats => Panel::Friends,
        };
    }

    /// Enter insert mode
    pub fn enter_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    /// Exit insert mode
    pub fn exit_insert(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Add a chat to the list
    pub fn add_chat(&mut self, id: i64, name: String) {
        if !self.chats.iter().any(|c| c.id == id) {
            self.chats.push(Chat {
                id,
                name,
                last_message: None,
                unread: 0,
            });
        }
    }

    /// Add a message to a chat
    pub fn add_message(&mut self, chat_id: i64, sender: String, text: String, outgoing: bool) {
        let messages = self.messages.entry(chat_id).or_insert_with(Vec::new);
        messages.push(Message { sender, text: text.clone(), outgoing });
        
        // Update last message preview
        if let Some(chat) = self.chats.iter_mut().find(|c| c.id == chat_id) {
            chat.last_message = Some(text);
            if !outgoing {
                chat.unread += 1;
            }
        }
    }
}
