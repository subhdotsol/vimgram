mod app;
mod telegram;
mod ui;

use std::io;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use grammers_client::Update;
use ratatui::{backend::CrosstermBackend, Terminal};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use crossterm::event::EventStream;

use app::{App, FindResult};
use telegram::auth::{authenticate, prompt_for_credentials};
use telegram::client::{TelegramClient, delete_session};
use telegram::accounts::AccountRegistry;
use ui::draw::draw;
use ui::input::handle_key;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Load .env file if present
    dotenvy::dotenv().ok();

    // Load account registry
    let mut account_registry = AccountRegistry::load();

    // Get API credentials (priority: Env, then Config File, then Prompt)
    let (api_id, api_hash) = match (
        std::env::var("TELEGRAM_API_ID"),
        std::env::var("TELEGRAM_API_HASH"),
    ) {
        (Ok(id), Ok(hash)) => (id.parse::<i32>().expect("Invalid API_ID"), hash),
        _ => {
            use telegram::client::Credentials;
            if let Some(creds) = Credentials::load() {
                (creds.api_id, creds.api_hash)
            } else {
                println!("╔═══════════════════════════════════╗");
                println!("║         ViMGRAM v0.2.0            ║");
                println!("╚═══════════════════════════════════╝");
                let (id, hash) = prompt_for_credentials();
                
                // Save for next time
                let creds = Credentials { api_id: id, api_hash: hash.clone() };
                if let Err(e) = creds.save() {
                    eprintln!("Warning: Failed to save credentials: {}", e);
                }
                (id, hash)
            }
        }
    };

    // Connect with account from registry, or use legacy connect
    println!("🔌 Connecting to Telegram...");
    let tg = if account_registry.has_accounts() {
        let active_id = account_registry.active.clone();
        TelegramClient::connect_with_account(api_id, &api_hash, &active_id).await?
    } else {
        TelegramClient::connect(api_id, &api_hash).await?
    };

    if !tg.is_authorized().await? {
        authenticate(&tg.client).await?;
        tg.save_session()?;
        
        // Update the current account's info in registry
        let me = tg.client.get_me().await?;
        let phone = me.phone().unwrap_or("Unknown").to_string();
        let name = me.first_name().to_string();
        
        if account_registry.has_accounts() {
            // Update existing account's info
            if let Some(account) = account_registry.accounts.iter_mut().find(|a| a.id == account_registry.active) {
                account.phone = phone;
                account.name = name;
            }
        } else {
            // First account - add it
            let account_id = account_registry.add_account(phone, name);
            account_registry.set_active(&account_id);
        }
        let _ = account_registry.save();
    }

    let me = tg.client.get_me().await?;
    println!("✅ Logged in as @{}", me.username().unwrap_or("unknown"));
    println!("🚀 Starting Vimgram...");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    app.loading_status = Some("Loading chats...".to_string());
    
    // Set account info in app state
    let account_info: Vec<(String, String)> = account_registry.accounts
        .iter()
        .map(|a| (a.id.clone(), format!("{} ({})", a.name, a.phone)))
        .collect();
    app.set_account_info(account_registry.active.clone(), account_info);

    // Add welcome chat (the keybindings box is rendered by draw_welcome_box in draw.rs)
    app.add_chat(1, "Welcome".to_string());

    // Load dialogs (just chat names, no messages for faster loading)
    // Limit to 100 chats to prevent overload
    // Also cache the grammers Chat objects for O(1) lookup later
    let mut chat_cache: HashMap<i64, grammers_client::types::Chat> = HashMap::new();
    let mut dialogs = tg.client.iter_dialogs();
    let mut count = 0;
    const MAX_CHATS: usize = 100;
    while let Some(dialog) = dialogs.next().await? {
        if count >= MAX_CHATS {
            break;
        }
        let chat = dialog.chat();
        chat_cache.insert(chat.id(), chat.clone());
        app.add_chat(chat.id(), chat.name().to_string());
        count += 1;
    }
    // Wrap in Arc<RwLock> for sharing with async tasks (allows mutable updates for new users)
    let chat_cache = Arc::new(RwLock::new(chat_cache));

    app.loading_status = None;
    // Let lazy loading handle message fetching for the first chat too
    app.needs_message_load = true;

    // Create a channel for updates
    let (tx, mut rx) = mpsc::unbounded_channel();
    let client_clone = tg.client.clone();

    // Spawn update listener task
    tokio::spawn(async move {
        loop {
            match client_clone.next_update().await {
                Ok(Some(update)) => {
                    if tx.send(update).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    // Wait a bit before retrying on error
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Create a channel for loaded messages (chat_id, messages)
    type LoadedMessages = (i64, Vec<(String, String, bool)>);
    let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<LoadedMessages>();

    // Create a channel for find user results
    type FindUserResult = (String, Result<(i64, String, grammers_client::types::Chat), String>);
    let (find_tx, mut find_rx) = mpsc::unbounded_channel::<FindUserResult>();

    // Main loop
    let mut reader = EventStream::new();

    loop {
        // Draw UI
        terminal.draw(|f| draw(f, &app))?;

        // Handle reloading status from previous loop
        if app.reload_requested {
            app.reload_requested = false;
            // ... (reload logic is handled below in the select loop now via manual calls if needed, 
            // but actually we should keep the reload logic inline or just trigger message fetch)
             if let Some(chat_id) = app.current_chat_id() {
                // Find the chat and fetch messages
                let mut dialogs = tg.client.iter_dialogs();
                while let Some(dialog) = dialogs.next().await? {
                    if dialog.chat().id() == chat_id {
                        // Clear existing messages for this chat
                        app.messages.remove(&chat_id);

                        // Fetch last 50 messages
                        let mut messages_iter = tg.client.iter_messages(dialog.chat());
                        let mut fetched = 0;
                        while let Some(msg) = messages_iter.next().await? {
                            if fetched >= 50 {
                                break;
                            }
                            let sender = if msg.outgoing() {
                                "You".to_string()
                            } else {
                                msg.sender()
                                    .map(|s| {
                                        let name = s.name().to_string();
                                        if name.is_empty() {
                                            dialog.chat().name().to_string()
                                        } else {
                                            name
                                        }
                                    })
                                    .unwrap_or_else(|| dialog.chat().name().to_string())
                            };
                            app.add_message(
                                chat_id,
                                sender,
                                msg.text().to_string(),
                                msg.outgoing(),
                            );
                            fetched += 1;
                        }

                        // Reverse messages to show oldest first
                        if let Some(msgs) = app.messages.get_mut(&chat_id) {
                            msgs.reverse();
                        }
                        break;
                    }
                }
            }
        }

        // Lazy-load messages for currently selected chat in background (non-blocking)
        if app.needs_message_load {
            app.needs_message_load = false;
            if let Some(chat_id) = app.current_chat_id() {
                // Only load if we don't have messages for this chat yet
                if !app.messages.contains_key(&chat_id) && chat_id != 1 {
                    // Check if we're already loading this chat
                    if app.pending_load != Some(chat_id) {
                        app.loading_status = Some("Loading...".to_string());
                        app.pending_load = Some(chat_id);
                        
                        // Spawn background loader using cached chat (O(1) lookup!)
                        let client = tg.client.clone();
                        let loader_tx = msg_tx.clone();
                        let cache = chat_cache.clone();
                        tokio::spawn(async move {
                            // Use cached chat directly - no dialog iteration!
                            let cache_read = cache.read().await;
                            if let Some(cached_chat) = cache_read.get(&chat_id) {
                                let chat_name = cached_chat.name().to_string();
                                let cached_chat = cached_chat.clone();
                                drop(cache_read); // Release lock before async iteration
                                let mut messages_iter = client.iter_messages(&cached_chat);
                                let mut loaded_msgs: Vec<(String, String, bool)> = Vec::new();
                                let mut fetched = 0;
                                while let Ok(Some(msg)) = messages_iter.next().await {
                                    if fetched >= 50 {
                                        break;
                                    }
                                    let sender = if msg.outgoing() {
                                        "You".to_string()
                                    } else {
                                        msg.sender()
                                            .map(|s| {
                                                let name = s.name().to_string();
                                                if name.trim().is_empty() {
                                                    if chat_name.trim().is_empty() { String::new() } else { chat_name.clone() }
                                                } else {
                                                    name
                                                }
                                            })
                                            .unwrap_or_else(|| {
                                                if chat_name.trim().is_empty() { String::new() } else { chat_name.clone() }
                                            })
                                    };
                                    loaded_msgs.push((sender, msg.text().to_string(), msg.outgoing()));
                                    fetched += 1;
                                }
                                // Reverse to oldest-first and send via channel
                                loaded_msgs.reverse();
                                let _ = loader_tx.send((chat_id, loaded_msgs));
                            }
                        });
                    }
                } else {
                    // Already have messages, just clear loading status
                    app.loading_status = None;
                    app.pending_load = None;
                }
            }
        }

        // Handle find user request
        if let Some(username) = app.find_requested.take() {
            let client = tg.client.clone();
            let find_tx_clone = find_tx.clone();
            let username_clone = username.clone();
            tokio::spawn(async move {
                match client.resolve_username(&username_clone).await {
                    Ok(Some(chat)) => {
                        let id = chat.id();
                        let name = chat.name().to_string();
                        let _ = find_tx_clone.send((username_clone, Ok((id, name, chat))));
                    }
                    Ok(None) => {
                        let _ = find_tx_clone.send((username_clone.clone(), Err(format!("User @{} not found", username_clone))));
                    }
                    Err(e) => {
                        let _ = find_tx_clone.send((username_clone, Err(format!("Error: {}", e))));
                    }
                }
            });
        }

        tokio::select! {
            // Handle Keyboard Input
            maybe_event = reader.next().fuse() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                         if let Some(message_to_send) = handle_key(&mut app, key) {
                            // Send message to current chat using cached chat (O(1) lookup!)
                            if let Some(chat_id) = app.current_chat_id() {
                                let cache_read = chat_cache.read().await;
                                if let Some(cached_chat) = cache_read.get(&chat_id) {
                                    let cached_chat = cached_chat.clone();
                                    drop(cache_read); // Release lock before async operation
                                    tg.client
                                        .send_message(&cached_chat, message_to_send.clone())
                                        .await?;
                                    app.add_message(
                                        chat_id,
                                        "You".to_string(),
                                        message_to_send,
                                        true,
                                    );
                                }
                            }
                        }
                        if app.should_quit || app.disconnect_requested || app.add_account_requested || app.switch_account_requested.is_some() {
                            break;
                        }
                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    _ => {}
                }
            }

            // Handle Telegram Updates
            Some(update) = rx.recv() => {
               if let Update::NewMessage(msg) = update {
                    if !msg.outgoing() {
                        let chat = msg.chat();
                        // Get sender name - fallback to chat name for private chats
                        let mut sender_name = msg.sender()
                            .map(|s| {
                                let name = s.name().to_string();
                                if name.trim().is_empty() { 
                                    let cname = chat.name().to_string();
                                    if cname.trim().is_empty() { String::new() } else { cname }
                                } else { name }
                            })
                            .unwrap_or_else(|| {
                                let cname = chat.name().to_string();
                                if cname.trim().is_empty() { String::new() } else { cname }
                            });

                        // If sender is still Unknown, try to refresh via dialogs
                        if sender_name == "Unknown" || sender_name.trim().is_empty() {
                            // If it's a DM (positive ID), the chat name IS the sender name.
                            // Trust the chat name over "Unknown"
                            let mut resolved_name = chat.name().to_string();
                            
                            // If even the chat name from the update is "Unknown", check our local cache
                            if (resolved_name == "Unknown" || resolved_name.trim().is_empty()) && chat.id() > 0 {
                                if let Some(existing_chat) = app.chats.iter().find(|c| c.id == chat.id()) {
                                    resolved_name = existing_chat.name.clone();
                                }
                            }

                            if chat.id() > 0 && !resolved_name.trim().is_empty() && resolved_name != "Unknown" {
                                sender_name = resolved_name;
                            } else {
                                // Fetch the latest dialog (which should be this new message)
                                // This also naturally updates the cache
                                let mut dialogs = tg.client.iter_dialogs();
                                if let Ok(Some(dialog)) = dialogs.next().await {
                                    if dialog.chat().id() == chat.id() {
                                        let name = dialog.chat().name().to_string();
                                        if !name.trim().is_empty() {
                                            sender_name = name;
                                        }
                                    }
                                }
                            }
                        }

                        app.add_chat(chat.id(), chat.name().to_string());
                        app.add_message(chat.id(), sender_name, msg.text().to_string(), false);
                    }
                }
            }

            // Handle loaded messages from background task
            Some((chat_id, messages)) = msg_rx.recv() => {
                // Only apply if this is still the chat we're waiting for (debounce)
                if app.pending_load == Some(chat_id) {
                    for (sender, text, outgoing) in messages {
                        app.add_message(chat_id, sender, text, outgoing);
                    }
                    app.loading_status = None;
                    app.pending_load = None;
                }
                // If user navigated away, just ignore the loaded messages
            }

            // Handle find user results
            Some((username, result)) = find_rx.recv() => {
                match result {
                    Ok((id, name, chat)) => {
                        // Add the user to the chat list and cache
                        app.add_chat(id, name.clone());
                        chat_cache.write().await.insert(id, chat);
                        app.set_find_result(FindResult::Found { id, name });
                    }
                    Err(msg) => {
                        if msg.contains("not found") {
                            app.set_find_result(FindResult::NotFound(username));
                        } else {
                            app.set_find_result(FindResult::Error(msg));
                        }
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle disconnect request
    if app.disconnect_requested {
        match delete_session() {
            Ok(true) => println!("🔌 Session deleted. Run vimgram again to log in with a new account."),
            Ok(false) => println!("⚠️ No session file found."),
            Err(e) => println!("❌ Failed to delete session: {}", e),
        }
    } else if let Some(account_id) = app.switch_account_requested {
        // Switch to the selected account and auto-restart
        account_registry.set_active(&account_id);
        let _ = account_registry.save();
        println!("🔄 Switching to account: {}...", account_id);
        
        // Auto-restart by exec'ing ourselves
        let exe = std::env::current_exe().expect("Failed to get current executable");
        let args: Vec<String> = std::env::args().collect();
        
        // Use exec to replace current process (Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let mut cmd = std::process::Command::new(&exe);
            cmd.args(&args[1..]);
            let err = cmd.exec();
            eprintln!("Failed to restart: {}", err);
        }
        
        // On non-Unix, just tell user to restart
        #[cfg(not(unix))]
        {
            println!("   Run vimgram again to load the account.");
        }
    } else if app.add_account_requested {
        // Create a new account entry and set it as active (session doesn't exist yet)
        let new_id = format!("account_{}", account_registry.accounts.len() + 1);
        account_registry.accounts.push(telegram::accounts::Account {
            id: new_id.clone(),
            phone: "New".to_string(),
            name: "New Account".to_string(),
        });
        account_registry.set_active(&new_id);
        let _ = account_registry.save();
        
        // Auto-restart for new account authentication
        println!("➕ Adding new account...");
        let exe = std::env::current_exe().expect("Failed to get current executable");
        let args: Vec<String> = std::env::args().collect();
        
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let mut cmd = std::process::Command::new(&exe);
            cmd.args(&args[1..]);
            let err = cmd.exec();
            eprintln!("Failed to restart: {}", err);
        }
        
        #[cfg(not(unix))]
        {
            println!("   Run vimgram again to authenticate the new account.");
        }
    } else {
        println!("👋 Goodbye!");
    }
    Ok(())
}
