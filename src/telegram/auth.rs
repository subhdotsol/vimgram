use grammers_client::Client;
use std::io::{self, BufRead, Write};

pub async fn authenticate(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("📱 Telegram Authentication");
    println!("──────────────────────────");

    // Get phone number
    print!("Enter phone number (with country code, e.g. +91...): ");
    io::stdout().flush()?;
    let phone = io::stdin().lock().lines().next().unwrap()?;

    // Request login code
    let token = client.request_login_code(&phone).await?;

    // Get OTP
    print!("Enter the OTP sent to your Telegram: ");
    io::stdout().flush()?;
    let code = io::stdin().lock().lines().next().unwrap()?;

    // Sign in
    match client.sign_in(&token, &code).await {
        Ok(_user) => {
            println!("✅ Logged in successfully!");
        }
        Err(grammers_client::SignInError::PasswordRequired(password_token)) => {
            // 2FA is enabled
            print!("Enter your 2FA password: ");
            io::stdout().flush()?;
            let password = io::stdin().lock().lines().next().unwrap()?;
            client.check_password(password_token, password).await?;
            println!("✅ Logged in with 2FA!");
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

pub fn prompt_for_credentials() -> (i32, String) {
    println!("🔑 Telegram API Credentials");
    println!("Get these from https://my.telegram.org");
    println!("──────────────────────────────────────");

    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let api_id: i32 = loop {
        print!("API_ID: ");
        stdout.flush().unwrap();
        let mut input = String::new();
        stdin.read_line(&mut input).expect("Failed to read line");
        match input.trim().parse() {
            Ok(num) => break num,
            Err(_) => println!("❌ Invalid API_ID. Please enter a valid number."),
        }
    };

    let api_hash = loop {
        print!("API_HASH: ");
        stdout.flush().unwrap();
        let mut input = String::new();
        stdin.read_line(&mut input).expect("Failed to read line");
        let hash = input.trim().to_string();
        if !hash.is_empty() {
            break hash;
        }
        println!("❌ API_HASH cannot be empty.");
    };

    (api_id, api_hash)
}
