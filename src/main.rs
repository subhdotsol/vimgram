mod app;
mod telegram;
mod ui;

use std::io;
use std::time::Duration;

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

use app::App;
use telegram::auth::{authenticate, prompt_for_credentials};
use telegram::client::TelegramClient;
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
                println!("║         ViMGRAM v0.1.1            ║");
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

    // Connect and authenticate
    println!("🔌 Connecting to Telegram...");
    let tg = TelegramClient::connect(api_id, &api_hash).await?;

    if !tg.is_authorized().await? {
        authenticate(&tg.client).await?;
        tg.save_session()?;
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

    // Add welcome chat
    app.add_chat(1, "Welcome".to_string());
    app.add_message(
        1,
        "Bifrost".to_string(),
        "Welcome to Bifrost! Use hjkl to navigate, i to type, Enter to send.".to_string(),
        false,
    );

    // Load dialogs (just chat names, no messages for faster loading)
    // Limit to 100 chats to prevent overload
    let mut dialogs = tg.client.iter_dialogs();
    let mut count = 0;
    const MAX_CHATS: usize = 100;
    while let Some(dialog) = dialogs.next().await? {
        if count >= MAX_CHATS {
            break;
        }
        let chat = dialog.chat();
        app.add_chat(chat.id(), chat.name().to_string());
        count += 1;
    }

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

        // Lazy-load messages for currently selected chat if needed
        if app.needs_message_load {
            app.needs_message_load = false;
            if let Some(chat_id) = app.current_chat_id() {
                // Only load if we don't have messages for this chat yet
                if !app.messages.contains_key(&chat_id) && chat_id != 1 {
                    app.loading_status = Some("Loading messages...".to_string());
                    terminal.draw(|f| draw(f, &app))?; // Show loading status

                    // Find the dialog for this chat
                    let mut dialogs = tg.client.iter_dialogs();
                    while let Some(dialog) = dialogs.next().await? {
                        if dialog.chat().id() == chat_id {
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
                                            if name.trim().is_empty() {
                                                let cname = dialog.chat().name().to_string();
                                                if cname.trim().is_empty() { String::new() } else { cname }
                                            } else {
                                                name
                                            }
                                        })
                                        .unwrap_or_else(|| {
                                            let cname = dialog.chat().name().to_string();
                                            if cname.trim().is_empty() { String::new() } else { cname }
                                        })
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
                    app.loading_status = None;
                }
            }
        }

        tokio::select! {
            // Handle Keyboard Input
            maybe_event = reader.next().fuse() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                         if let Some(message_to_send) = handle_key(&mut app, key) {
                            // Send message to current chat
                            if let Some(chat_id) = app.current_chat_id() {
                                if let Some(chat) = app.chats.iter().find(|c| c.id == chat_id) {
                                    // Find the actual chat to send to
                                    let mut dialogs = tg.client.iter_dialogs();
                                    while let Some(dialog) = dialogs.next().await? {
                                        if dialog.chat().id() == chat_id {
                                            tg.client
                                                .send_message(dialog.chat(), message_to_send.clone())
                                                .await?;
                                            app.add_message(
                                                chat_id,
                                                "You".to_string(),
                                                message_to_send,
                                                true,
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if app.should_quit {
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
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("👋 Goodbye!");
    Ok(())
}
