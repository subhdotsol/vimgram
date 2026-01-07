use grammers_client::{Client, Update};

/// Listen for incoming Telegram updates and print messages to console
pub async fn listen_for_updates(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Listening for messages... (Press Ctrl+C to quit)");
    println!("────────────────────────────────────────────────────");
    println!();

    loop {
        let update = client.next_update().await?;
        
        match update {
            Some(Update::NewMessage(message)) if !message.outgoing() => {
                // Get sender name
                let sender = message.sender();
                let sender_name = match &sender {
                    Some(chat) => chat.name().to_string(),
                    None => String::new(),
                };

                // Get chat name (for groups/channels)
                let chat = message.chat();
                let chat_name = chat.name();

                // Get message text
                let text = message.text();

                // Format output based on whether it's a group or DM
                if chat_name != sender_name {
                    // Group message
                    println!("[{}] {}: {}", chat_name, sender_name, text);
                } else {
                    // Direct message
                    println!("{}: {}", sender_name, text);
                }
            }
            _ => {
                // Ignore other updates (read receipts, typing indicators, etc.)
            }
        }
    }
}
