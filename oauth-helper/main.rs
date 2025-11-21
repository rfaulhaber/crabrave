//! OAuth2 Token Helper
//!
//! This interactive program helps you obtain OAuth2 tokens for use with crabrave.
//! It will guide you through the OAuth2 authorization flow and save your tokens
//! to a file that can be used by integration tests and your applications.
//!
//! # Usage
//!
//! ```bash
//! cargo run -p oauth-helper
//! ```
//!
//! # What this does
//!
//! 1. Prompts for your Tumblr app credentials (consumer key/secret)
//! 2. Generates an authorization URL
//! 3. Opens the URL in your browser (or displays it for you to visit)
//! 4. Waits for you to paste the authorization code
//! 5. Exchanges the code for access and refresh tokens
//! 6. Saves tokens to `.tumblr_tokens.json` in your home directory

use crabrave::oauth::OAuth2Config;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
struct TokenStorage {
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    redirect_uri: String,
}

fn get_token_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".tumblr_tokens.json");
    path
}

fn prompt(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        Crabrave OAuth2 Token Helper                           ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("This tool will help you obtain OAuth2 tokens for the Tumblr API.\n");

    // Check if tokens already exist
    let token_path = get_token_path();
    if token_path.exists() {
        println!("⚠️  Found existing tokens at: {}", token_path.display());
        let overwrite = prompt("Do you want to overwrite them? (y/N): ");
        if overwrite.to_lowercase() != "y" {
            println!("\nUsing existing tokens. To refresh, delete the file or answer 'y'.");
            return Ok(());
        }
    }

    println!("\n📋 Step 1: Get your Tumblr app credentials");
    println!("   Visit: https://www.tumblr.com/oauth/apps");
    println!("   Create an app if you haven't already.\n");

    let consumer_key = prompt("Enter your Consumer Key: ");
    let consumer_secret = prompt("Enter your Consumer Secret: ");

    println!("\n📋 Step 2: Set up redirect URI");
    println!("   This should match the redirect URI registered with your app.");
    let default_redirect = "http://localhost:8080/callback";
    println!("   Default: {}", default_redirect);

    let redirect_uri = prompt("Enter redirect URI (or press Enter for default): ");
    let redirect_uri = if redirect_uri.is_empty() {
        default_redirect.to_string()
    } else {
        redirect_uri
    };

    // Create OAuth2 config
    let config = OAuth2Config::new(&consumer_key, &consumer_secret, &redirect_uri);

    println!("\n🔐 Step 3: Authorize the application");
    let (auth_url, csrf_token) = config.authorize_url();

    println!("\n   Authorization URL:");
    println!("   {}\n", auth_url);
    println!("   CSRF Token: {}", csrf_token.secret());
    println!("\n   Opening browser... (if it doesn't open, copy the URL above)");

    // Try to open the browser
    if let Err(_) = open::that(&auth_url) {
        println!("   ⚠️  Could not open browser automatically.");
        println!("   Please visit the URL above manually.");
    }

    println!("\n📋 Step 4: Complete authorization");
    println!("   After authorizing, you'll be redirected to:");
    println!("   {}?code=XXXXX&state=YYYYY\n", redirect_uri);

    let code = prompt("Enter the 'code' from the redirect URL: ");

    println!("\n⏳ Exchanging authorization code for tokens...");

    match config.exchange_code(&code).await {
        Ok(token) => {
            println!("✅ Success! Obtained tokens:\n");
            println!("   Access Token: {}...", &token.access_token[..20.min(token.access_token.len())]);

            if let Some(ref refresh_token) = token.refresh_token {
                println!("   Refresh Token: {}...", &refresh_token[..20.min(refresh_token.len())]);
            }

            if let Some(expires_in) = token.expires_in {
                println!("   Expires in: {} seconds ({} hours)", expires_in, expires_in / 3600);
            }

            // Save tokens to file
            let storage = TokenStorage {
                consumer_key,
                consumer_secret,
                access_token: token.access_token,
                refresh_token: token.refresh_token,
                expires_in: token.expires_in,
                redirect_uri,
            };

            let json = serde_json::to_string_pretty(&storage)?;
            fs::write(&token_path, json)?;

            println!("\n💾 Tokens saved to: {}", token_path.display());
            println!("\n✨ All done! You can now use these tokens with crabrave.");
            println!("\nTo use in integration tests:");
            println!("  cargo test --test integration -- --ignored");
            println!("\nTo use in your code:");
            println!("  let tokens = std::fs::read_to_string(\"{}\").unwrap();", token_path.display());
            println!("  let storage: TokenStorage = serde_json::from_str(&tokens).unwrap();");
        }
        Err(e) => {
            eprintln!("\n❌ Failed to exchange code: {}", e);
            eprintln!("\nPossible issues:");
            eprintln!("  - The authorization code may have expired (they're single-use)");
            eprintln!("  - The redirect URI doesn't match what's registered");
            eprintln!("  - The consumer key/secret are incorrect");
            eprintln!("\nPlease try again with a fresh authorization code.");
            std::process::exit(1);
        }
    }

    Ok(())
}
