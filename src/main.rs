mod app;
mod telegram;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use grammers_client::Update;
use ratatui::{backend::CrosstermBackend, Terminal};

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
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Load .env file if present
    dotenvy::dotenv().ok();

    // Get API credentials (from env or prompt)
    let (api_id, api_hash) = match (
        std::env::var("TELEGRAM_API_ID"),
        std::env::var("TELEGRAM_API_HASH"),
    ) {
        (Ok(id), Ok(hash)) => (id.parse::<i32>().expect("Invalid API_ID"), hash),
        _ => {
            println!("╔═══════════════════════════════════╗");
            println!("║         Bifrost v0.1.0            ║");
            println!("╚═══════════════════════════════════╝");
            prompt_for_credentials()
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
    println!("🚀 Starting TUI...");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();
    app.loading_status = Some("Loading chats...".to_string());

    // Add welcome chat
    app.add_chat(1, "Welcome".to_string());
    app.add_message(1, "Bifrost".to_string(), "Welcome to Bifrost! Use hjkl to navigate, i to type, Enter to send.".to_string(), false);

    // Load ALL dialogs (just chat names, no messages for faster loading)
    let mut dialogs = tg.client.iter_dialogs();
    let mut dialog_list: Vec<grammers_client::types::Dialog> = Vec::new();
    let mut count = 0;
    while let Some(dialog) = dialogs.next().await? {
        let chat = dialog.chat();
        app.add_chat(chat.id(), chat.name().to_string());
        dialog_list.push(dialog);
        count += 1;
        // Update status every 10 chats
        if count % 10 == 0 {
            app.loading_status = Some(format!("Loaded {} chats...", count));
        }
    }
    
    app.loading_status = Some(format!("Loaded {} chats!", count));
    
    // Only fetch messages for the first (selected) chat to start quickly
    if let Some(first_dialog) = dialog_list.first() {
        let chat = first_dialog.chat();
        let chat_id = chat.id();
        let mut messages_iter = tg.client.iter_messages(chat.clone());
        let mut fetched = 0;
        while let Some(msg) = messages_iter.next().await? {
            if fetched >= 50 { break; }
            let sender = if msg.outgoing() {
                "You".to_string()
            } else {
                msg.sender()
                    .map(|s| {
                        let name = s.name().to_string();
                        if name.is_empty() { chat.name().to_string() } else { name }
                    })
                    .unwrap_or_else(|| chat.name().to_string())
            };
            app.add_message(chat_id, sender, msg.text().to_string(), msg.outgoing());
            fetched += 1;
        }
        
        // Reverse messages to show oldest first
        if let Some(msgs) = app.messages.get_mut(&chat_id) {
            msgs.reverse();
        }
    }
    
    app.loading_status = None;
    app.needs_message_load = false;

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| draw(f, &app))?;

        // Check for quit
        if app.should_quit {
            break;
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
                                if fetched >= 50 { break; }
                                let sender = if msg.outgoing() {
                                    "You".to_string()
                                } else {
                                    msg.sender()
                                        .map(|s| {
                                            let name = s.name().to_string();
                                            if name.is_empty() { dialog.chat().name().to_string() } else { name }
                                        })
                                        .unwrap_or_else(|| dialog.chat().name().to_string())
                                };
                                app.add_message(chat_id, sender, msg.text().to_string(), msg.outgoing());
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

        // Handle reload request (r key)
        if app.reload_requested {
            app.reload_requested = false;
            if let Some(chat_id) = app.current_chat_id() {
                // Find the chat and fetch messages
                let mut dialogs = tg.client.iter_dialogs();
                while let Some(dialog) = dialogs.next().await? {
                    if dialog.chat().id() == chat_id {
                        // Clear existing messages for this chat
                        app.messages.remove(&chat_id);
                        
                        // Fetch last 20 messages
                        let mut messages_iter = tg.client.iter_messages(dialog.chat());
                        let mut fetched = 0;
                        while let Some(msg) = messages_iter.next().await? {
                            if fetched >= 50 { break; }
                            let sender = if msg.outgoing() {
                                "You".to_string()
                            } else {
                                msg.sender()
                                    .map(|s| {
                                        let name = s.name().to_string();
                                        if name.is_empty() { dialog.chat().name().to_string() } else { name }
                                    })
                                    .unwrap_or_else(|| dialog.chat().name().to_string())
                            };
                            app.add_message(chat_id, sender, msg.text().to_string(), msg.outgoing());
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

        // Poll for events (keyboard + telegram updates)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let Some(message_to_send) = handle_key(&mut app, key) {
                    // Send message to current chat
                    if let Some(chat_id) = app.current_chat_id() {
                        if let Some(chat) = app.chats.iter().find(|c| c.id == chat_id) {
                            // Find the actual chat to send to
                            let mut dialogs = tg.client.iter_dialogs();
                            while let Some(dialog) = dialogs.next().await? {
                                if dialog.chat().id() == chat_id {
                                    tg.client.send_message(dialog.chat(), message_to_send.clone()).await?;
                                    app.add_message(chat_id, "You".to_string(), message_to_send, true);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for Telegram updates (non-blocking)
        tokio::select! {
            update = tg.client.next_update() => {
                if let Ok(Some(Update::NewMessage(msg))) = update {
                    if !msg.outgoing() {
                        let chat = msg.chat();
                        // Get sender name - fallback to chat name for private chats
                        let sender = msg.sender()
                            .map(|s| {
                                let name = s.name().to_string();
                                if name.is_empty() { chat.name().to_string() } else { name }
                            })
                            .unwrap_or_else(|| chat.name().to_string());
                        app.add_chat(chat.id(), chat.name().to_string());
                        app.add_message(chat.id(), sender, msg.text().to_string(), false);
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    println!("👋 Goodbye!");
    Ok(())
}
