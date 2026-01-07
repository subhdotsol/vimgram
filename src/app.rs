use std::collections::HashMap;

/// Application mode (Vim-style)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Search,
    AccountPicker,
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
    // Search mode state
    pub search_input: String,
    pub filtered_chat_indices: Vec<usize>,
    pub search_selected: usize,
    // Disconnect request
    pub disconnect_requested: bool,
    // Multi-account state
    pub current_account_id: String,
    pub account_names: Vec<(String, String)>,  // (id, display_name)
    pub account_picker_selected: usize,
    pub switch_account_requested: Option<String>,
    pub add_account_requested: bool,
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
            // Search mode state
            search_input: String::new(),
            filtered_chat_indices: Vec::new(),
            search_selected: 0,
            // Disconnect
            disconnect_requested: false,
            // Multi-account
            current_account_id: String::new(),
            account_names: Vec::new(),
            account_picker_selected: 0,
            switch_account_requested: None,
            add_account_requested: false,
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

    /// Enter search mode
    pub fn enter_search(&mut self) {
        self.mode = Mode::Search;
        self.search_input.clear();
        self.search_selected = 0;
        self.update_search_filter();
    }

    /// Exit search mode without jumping
    pub fn exit_search(&mut self) {
        self.mode = Mode::Normal;
        self.search_input.clear();
        self.filtered_chat_indices.clear();
    }

    /// Update filtered chat indices based on search input
    pub fn update_search_filter(&mut self) {
        let query = self.search_input.to_lowercase();
        self.filtered_chat_indices = self.chats
            .iter()
            .enumerate()
            .filter(|(_, chat)| {
                if query.is_empty() {
                    true // Show all when empty
                } else {
                    chat.name.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();
        
        // Reset selection if it's out of bounds
        if self.search_selected >= self.filtered_chat_indices.len() {
            self.search_selected = 0;
        }
    }

    /// Jump to the selected search result
    pub fn jump_to_selected_search_result(&mut self) {
        if let Some(&chat_index) = self.filtered_chat_indices.get(self.search_selected) {
            self.selected_chat = chat_index;
            self.scroll_offset = 0;
            self.needs_message_load = true;
            self.clear_current_unread();
        }
        self.exit_search();
    }

    /// Move selection up in search results
    pub fn search_move_up(&mut self) {
        if self.search_selected > 0 {
            self.search_selected -= 1;
        }
    }

    /// Move selection down in search results
    pub fn search_move_down(&mut self) {
        if self.search_selected < self.filtered_chat_indices.len().saturating_sub(1) {
            self.search_selected += 1;
        }
    }

    // ==================== Account Picker Methods ====================

    /// Enter account picker mode
    pub fn enter_account_picker(&mut self) {
        self.mode = Mode::AccountPicker;
        self.account_picker_selected = 0;
        // Find current account index
        for (i, (id, _)) in self.account_names.iter().enumerate() {
            if *id == self.current_account_id {
                self.account_picker_selected = i;
                break;
            }
        }
    }

    /// Exit account picker mode
    pub fn exit_account_picker(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Move up in account picker
    pub fn account_picker_move_up(&mut self) {
        if self.account_picker_selected > 0 {
            self.account_picker_selected -= 1;
        }
    }

    /// Move down in account picker (includes "+ Add Account" option)
    pub fn account_picker_move_down(&mut self) {
        // +1 for the "Add Account" option
        let max_index = self.account_names.len();
        if self.account_picker_selected < max_index {
            self.account_picker_selected += 1;
        }
    }

    /// Select the current account in picker
    pub fn select_account(&mut self) {
        if self.account_picker_selected < self.account_names.len() {
            // Switch to selected account
            let (account_id, _) = &self.account_names[self.account_picker_selected];
            if *account_id != self.current_account_id {
                self.switch_account_requested = Some(account_id.clone());
            }
            self.exit_account_picker();
        } else {
            // "Add Account" selected
            self.add_account_requested = true;
            self.exit_account_picker();
        }
    }

    /// Set the current account info
    pub fn set_account_info(&mut self, account_id: String, accounts: Vec<(String, String)>) {
        self.current_account_id = account_id;
        self.account_names = accounts;
    }
}
