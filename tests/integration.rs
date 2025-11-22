//! Integration tests against the real Tumblr API
//!
//! These tests are ignored by default and only run when explicitly requested.
//!
//! # Running Integration Tests
//!
//! ## Method 1: Using Environment Variables (Recommended for CI/CD)
//!
//! ```bash
//! # Set environment variables
//! export TUMBLR_CONSUMER_KEY="your_consumer_key"
//! export TUMBLR_CONSUMER_SECRET="your_consumer_secret"
//! export TUMBLR_ACCESS_TOKEN="your_access_token"
//! # Optional for OAuth2 token refresh:
//! export TUMBLR_REFRESH_TOKEN="your_refresh_token"
//!
//! # Run integration tests
//! cargo test --test integration -- --ignored
//! ```
//!
//! ## Method 2: Using the OAuth2 Helper (Recommended for Local Development)
//!
//! ```bash
//! # Run the helper to obtain and save OAuth2 tokens
//! cargo run -p oauth-helper
//!
//! # Run integration tests (will use saved tokens automatically)
//! cargo test --test integration -- --ignored
//! ```
//!
//! # Authentication Priority
//!
//! 1. Environment variables (TUMBLR_CONSUMER_KEY, TUMBLR_CONSUMER_SECRET, TUMBLR_ACCESS_TOKEN)
//! 2. Tokens from ~/.tumblr_tokens.json (created by oauth-helper binary)
//! 3. If neither available, fail with helpful instructions
//!
//! # OAuth2 Flow Tests
//!
//! Additional environment variables for OAuth2 flow testing:
//!
//! ```bash
//! # Optional: For testing code exchange (requires a fresh authorization code)
//! export TUMBLR_OAUTH2_AUTH_CODE="your_authorization_code"
//!
//! # Optional: For testing token refresh (requires a valid refresh token)
//! export TUMBLR_OAUTH2_REFRESH_TOKEN="your_refresh_token"
//!
//! # Required for OAuth2 flow: redirect URI registered with your app
//! export TUMBLR_OAUTH2_REDIRECT_URI="http://localhost:8080/callback"
//! ```

use crabrave::{oauth::{OAuth2Config, parse_callback}, Crabrave, CrabError};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct TokenStorage {
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    refresh_token: Option<String>,
    redirect_uri: String,
}

/// Helper to get required environment variable
fn get_env(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("Missing environment variable: {}", key))
}

/// Helper to get optional environment variable
fn get_env_optional(key: &str) -> Option<String> {
    env::var(key).ok()
}

/// Get path to token storage file
fn get_token_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(".tumblr_tokens.json");
    path
}

/// Load tokens from file if available
fn load_tokens_from_file() -> Option<TokenStorage> {
    let path = get_token_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    } else {
        None
    }
}

/// Creates a test client with proper OAuth2 authentication
///
/// Priority order:
/// 1. Environment variables (TUMBLR_CONSUMER_KEY, TUMBLR_CONSUMER_SECRET, TUMBLR_ACCESS_TOKEN)
/// 2. Tokens from ~/.tumblr_tokens.json file (created by oauth-helper binary)
/// 3. If neither available, fail with instructions
async fn test_client() -> Result<Crabrave, String> {
    // Priority 1: Try environment variables first
    if let (Ok(consumer_key), Ok(consumer_secret), Ok(access_token)) = (
        get_env("TUMBLR_CONSUMER_KEY"),
        get_env("TUMBLR_CONSUMER_SECRET"),
        get_env("TUMBLR_ACCESS_TOKEN"),
    ) {
        println!("🔑 Using credentials from environment variables");

        // Check if we have a refresh token to get a fresh access token
        if let Some(refresh_token) = get_env_optional("TUMBLR_REFRESH_TOKEN") {
            println!("🔄 Refresh token found, attempting to refresh access token...");
            let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
                .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

            let config = OAuth2Config::new(&consumer_key, &consumer_secret, &redirect_uri);

            match config.refresh_access_token(&refresh_token).await {
                Ok(new_token) => {
                    println!("✅ Got fresh access token from refresh token");
                    return Crabrave::builder()
                        .consumer_key(&consumer_key)
                        .consumer_secret(&consumer_secret)
                        .access_token(&new_token.access_token)
                        .build()
                        .map_err(|e| format!("Failed to build client: {}", e));
                }
                Err(e) => {
                    println!("⚠️  Token refresh failed: {}, using provided access token", e);
                }
            }
        }

        return Crabrave::builder()
            .consumer_key(&consumer_key)
            .consumer_secret(&consumer_secret)
            .access_token(&access_token)
            .build()
            .map_err(|e| format!("Failed to build client: {}", e));
    }

    // Priority 2: Try loading from token file
    if let Some(storage) = load_tokens_from_file() {
        println!("📁 Using tokens from {}", get_token_path().display());

        // If we have a refresh token, use it to get a fresh access token
        if let Some(ref refresh_token) = storage.refresh_token {
            println!("🔄 Refreshing access token...");
            let config = OAuth2Config::new(
                &storage.consumer_key,
                &storage.consumer_secret,
                &storage.redirect_uri,
            );

            match config.refresh_access_token(refresh_token).await {
                Ok(new_token) => {
                    println!("✅ Got fresh access token");
                    return Crabrave::builder()
                        .consumer_key(&storage.consumer_key)
                        .consumer_secret(&storage.consumer_secret)
                        .access_token(&new_token.access_token)
                        .build()
                        .map_err(|e| format!("Failed to build client: {}", e));
                }
                Err(e) => {
                    println!("⚠️  Token refresh failed: {}, using stored access token", e);
                }
            }
        }

        // Fall back to stored access token
        return Crabrave::builder()
            .consumer_key(&storage.consumer_key)
            .consumer_secret(&storage.consumer_secret)
            .access_token(&storage.access_token)
            .build()
            .map_err(|e| format!("Failed to build client: {}", e));
    }

    // Priority 3: No credentials found - provide helpful error message
    Err(format!(
        "\n\
        ❌ No OAuth2 credentials found!\n\
        \n\
        Integration tests require OAuth2 authentication.\n\
        \n\
        🚀 Option 1: Use Environment Variables (recommended for CI/CD)\n\
        \n\
           export TUMBLR_CONSUMER_KEY=\"your_consumer_key\"\n\
           export TUMBLR_CONSUMER_SECRET=\"your_consumer_secret\"\n\
           export TUMBLR_ACCESS_TOKEN=\"your_access_token\"\n\
           # Optional: For automatic token refresh\n\
           export TUMBLR_REFRESH_TOKEN=\"your_refresh_token\"\n\
        \n\
        🚀 Option 2: Use OAuth2 Helper (recommended for local development)\n\
        \n\
           1. Run the OAuth2 helper:\n\
              \x1b[1mcargo run -p oauth-helper\x1b[0m\n\
        \n\
           2. Enter your consumer key and secret\n\
              (Get these from: https://www.tumblr.com/oauth/apps)\n\
        \n\
           3. Authorize the app in your browser\n\
        \n\
           4. Tokens will be saved to: {}\n\
        \n\
           5. Run tests again:\n\
              \x1b[1mcargo test --test integration -- --ignored\x1b[0m\n\
        \n\
        ℹ️  The helper saves tokens (including refresh token) so you won't\n\
        need to re-authorize. Tests will automatically refresh expired tokens.\n\
        \n\
        📖 For more details, see TESTING.md\n\
        ",
        get_token_path().display()
    ))
}

/// Gets consumer credentials for OAuth2 flow tests
///
/// Priority order:
/// 1. Environment variables (TUMBLR_CONSUMER_KEY, TUMBLR_CONSUMER_SECRET)
/// 2. Token file (~/.tumblr_tokens.json)
fn get_consumer_credentials() -> Result<(String, String), String> {
    // Priority 1: Try environment variables first
    if let (Ok(consumer_key), Ok(consumer_secret)) = (
        get_env("TUMBLR_CONSUMER_KEY"),
        get_env("TUMBLR_CONSUMER_SECRET"),
    ) {
        return Ok((consumer_key, consumer_secret));
    }

    // Priority 2: Try loading from token file
    if let Some(storage) = load_tokens_from_file() {
        return Ok((storage.consumer_key, storage.consumer_secret));
    }

    // No credentials found
    Err(format!(
        "\n❌ No consumer credentials found!\n\
        \n\
        To run OAuth2 flow tests, you need consumer credentials.\n\
        \n\
        Option 1: Set environment variables\n\
           export TUMBLR_CONSUMER_KEY=\"your_key\"\n\
           export TUMBLR_CONSUMER_SECRET=\"your_secret\"\n\
        \n\
        Option 2: Run the OAuth2 helper\n\
           cargo run -p oauth-helper\n"
    ))
}

/// Gets the test blog name from environment
fn test_blog() -> Result<String, String> {
    get_env("TUMBLR_TEST_BLOG")
}

#[tokio::test]
#[ignore]
async fn integration_blog_info() {
    let client = test_client().await.expect("Failed to create client");

    // Test with Tumblr's official staff blog
    let result = client.blogs("staff").info().await;

    match result {
        Ok(info) => {
            println!("Blog name: {}", info.blog.name);
            println!("Blog title: {}", info.blog.title);
            println!("Total posts: {}", info.blog.posts);
            assert_eq!(info.blog.name, "staff");
        }
        Err(e) => panic!("Failed to get blog info: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_blog_avatar() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.blogs("staff").avatar(Some(64)).await;

    match result {
        Ok(avatar) => {
            println!("Avatar URL: {}", avatar.avatar_url);
            assert!(!avatar.avatar_url.is_empty());
            assert!(avatar.avatar_url.starts_with("http"));
        }
        Err(e) => panic!("Failed to get avatar: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_blog_posts() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.blogs("staff").posts().limit(5).send().await;

    match result {
        Ok(posts) => {
            println!("Retrieved {} posts", posts.posts.len());
            assert!(posts.posts.len() <= 5);
            for post in &posts.posts {
                println!("  Post {}: {}", post.id, post.post_type);
            }
        }
        Err(e) => panic!("Failed to get posts: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_user_info() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.users().info().await;

    match result {
        Ok(info) => {
            println!("Username: {}", info.user.name);
            println!("Following: {}", info.user.following);
            println!("Likes: {}", info.user.likes);
            println!("Blogs: {}", info.user.blogs.len());
            assert!(!info.user.name.is_empty());
        }
        Err(e) => panic!("Failed to get user info: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_user_dashboard() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.users().dashboard().limit(5).send().await;

    match result {
        Ok(dashboard) => {
            println!("Dashboard posts: {}", dashboard.posts.len());
            assert!(dashboard.posts.len() <= 5);
            for post in &dashboard.posts {
                println!("  Post from {}: {}", post.blog_name, post.post_type);
            }
        }
        Err(e) => panic!("Failed to get dashboard: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_tagged_posts() {
    let client = test_client().await.expect("Failed to create client");

    // Search for a popular tag
    let result = client.tagged("photography").limit(5).send().await;

    match result {
        Ok(tagged) => {
            println!("Tagged posts: {}", tagged.posts.len());
            assert!(tagged.posts.len() <= 5);
            for post in &tagged.posts {
                println!("  Post {}: {} tags", post.id, post.tags.len());
                assert!(post.tags.iter().any(|t| t.to_lowercase().contains("photo")));
            }
        }
        Err(e) => panic!("Failed to get tagged posts: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_get_specific_post() {
    let client = test_client().await.expect("Failed to create client");
    let blog = test_blog().expect("TUMBLR_TEST_BLOG not set");

    // First get a post ID from the blog
    let posts_result = client.blogs(blog.as_str()).posts().limit(1).send().await;

    match posts_result {
        Ok(posts) if !posts.posts.is_empty() => {
            let post_id = &posts.posts[0].id;
            println!("Testing with post ID: {}", post_id);

            // Now fetch that specific post
            let result = client.posts().get(blog.clone(), post_id.clone()).await;

            match result {
                Ok(post) => {
                    println!("Retrieved post: {}", post.post.id);
                    assert_eq!(&post.post.id, post_id);
                }
                Err(e) => panic!("Failed to get specific post: {}", e),
            }
        }
        Ok(_) => {
            println!("No posts found on blog, skipping test");
        }
        Err(e) => panic!("Failed to get posts for test: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_rate_limit_handling() {
    let client = test_client().await.expect("Failed to create client");

    // Make multiple rapid requests to potentially trigger rate limiting
    // This test demonstrates rate limit error handling
    for i in 0..5 {
        let result = client.blogs("staff").info().await;

        match result {
            Ok(_) => println!("Request {} succeeded", i + 1),
            Err(CrabError::RateLimit { retry_after }) => {
                println!("Hit rate limit! Retry after: {:?} seconds", retry_after);
                return; // Test passes - we handled the rate limit correctly
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    println!("Did not hit rate limit (normal behavior for low request volume)");
}

#[tokio::test]
#[ignore]
async fn integration_nonexistent_blog() {
    let client = test_client().await.expect("Failed to create client");

    let result = client
        .blogs("this-blog-definitely-does-not-exist-12345")
        .info()
        .await;

    match result {
        Err(CrabError::Api { status, message }) => {
            println!("Got expected error - Status: {}, Message: {}", status, message);
            assert_eq!(status, 404);
        }
        Ok(_) => panic!("Expected 404 error for nonexistent blog"),
        Err(e) => panic!("Got unexpected error type: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_user_likes() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.users().likes().limit(5).send().await;

    match result {
        Ok(likes) => {
            println!("Liked posts: {}", likes.liked_posts.len());
            println!("Total liked: {}", likes.liked_count);
            assert!(likes.liked_posts.len() <= 5);
        }
        Err(e) => panic!("Failed to get likes: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_user_following() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.users().following(Some(5), None).await;

    match result {
        Ok(following) => {
            println!("Following {} blogs", following.total_blogs);
            println!("Retrieved {} blogs", following.blogs.len());
            assert!(following.blogs.len() <= 5);
        }
        Err(e) => panic!("Failed to get following: {}", e),
    }
}

// ============================================================================
// OAuth2 Flow Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn integration_oauth2_authorize_url() {
    let (consumer_key, consumer_secret) = get_consumer_credentials()
        .expect("Consumer credentials required");
    let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
        .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

    let config = OAuth2Config::new(consumer_key, consumer_secret, redirect_uri.clone());
    let (auth_url, csrf_token) = config.authorize_url();

    println!("Authorization URL: {}", auth_url);
    println!("CSRF Token: {}", csrf_token.secret());

    // Verify the URL contains required parameters
    assert!(auth_url.contains("https://www.tumblr.com/oauth2/authorize"));
    assert!(auth_url.contains("client_id="));
    assert!(auth_url.contains(&format!("redirect_uri={}", urlencoding::encode(&redirect_uri))));
    assert!(auth_url.contains("state="));
    assert!(!csrf_token.secret().is_empty());

    println!("\n✅ Authorization URL generated successfully!");
    println!("Visit this URL to authorize your application:");
    println!("{}", auth_url);
}

#[tokio::test]
#[ignore]
async fn integration_oauth2_parse_callback() {
    // Test parsing a mock callback URL
    let callback_url = "http://localhost:8080/callback?code=test_code_123&state=test_state_456";
    let params = parse_callback(callback_url).expect("Failed to parse callback");

    println!("Parsed callback parameters:");
    for (key, value) in &params {
        println!("  {}: {}", key, value);
    }

    assert_eq!(params.get("code"), Some(&"test_code_123".to_string()));
    assert_eq!(params.get("state"), Some(&"test_state_456".to_string()));

    println!("\n✅ Callback parsing works correctly!");
}

#[tokio::test]
#[ignore]
async fn integration_oauth2_exchange_code() {
    let (consumer_key, consumer_secret) = get_consumer_credentials()
        .expect("Consumer credentials required");
    let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
        .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

    // This test requires a fresh authorization code
    let auth_code = match get_env("TUMBLR_OAUTH2_AUTH_CODE") {
        Ok(code) => code,
        Err(_) => {
            println!("⏭️  Skipping code exchange test - TUMBLR_OAUTH2_AUTH_CODE not set");
            println!("To test this, set TUMBLR_OAUTH2_AUTH_CODE with a fresh authorization code");
            println!("(Authorization codes are single-use and expire quickly)");
            return;
        }
    };

    let config = OAuth2Config::new(consumer_key, consumer_secret, redirect_uri);

    println!("Exchanging authorization code for access token...");
    let result = config.exchange_code(&auth_code).await;

    match result {
        Ok(token) => {
            println!("\n✅ Successfully exchanged code for token!");
            println!("Access token received: {}...", &token.access_token[..20.min(token.access_token.len())]);
            println!("Has refresh token: {}", token.can_refresh());
            println!("Has expiration: {}", token.has_expiration());

            if let Some(expires_in) = token.expires_in {
                println!("Expires in: {} seconds", expires_in);
            }

            assert!(!token.access_token.is_empty());
        }
        Err(e) => {
            // Authorization codes are single-use, so this might fail if already used
            println!("⚠️  Code exchange failed: {}", e);
            println!("Note: Authorization codes are single-use and expire quickly");
            println!("You may need to generate a fresh code for this test");
        }
    }
}

#[tokio::test]
#[ignore]
async fn integration_oauth2_refresh_token() {
    let (consumer_key, consumer_secret) = get_consumer_credentials()
        .expect("Consumer credentials required");
    let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
        .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

    // This test requires a valid refresh token
    let refresh_token = match get_env("TUMBLR_OAUTH2_REFRESH_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            println!("⏭️  Skipping token refresh test - TUMBLR_OAUTH2_REFRESH_TOKEN not set");
            println!("To test this, set TUMBLR_OAUTH2_REFRESH_TOKEN with a valid refresh token");
            println!("(Refresh tokens are obtained from the initial OAuth2 flow)");
            return;
        }
    };

    let config = OAuth2Config::new(consumer_key, consumer_secret, redirect_uri);

    println!("Refreshing access token...");
    let result = config.refresh_access_token(&refresh_token).await;

    match result {
        Ok(token) => {
            println!("\n✅ Successfully refreshed access token!");
            println!("New access token received: {}...", &token.access_token[..20.min(token.access_token.len())]);
            println!("Has new refresh token: {}", token.can_refresh());
            println!("Has expiration: {}", token.has_expiration());

            if let Some(expires_in) = token.expires_in {
                println!("Expires in: {} seconds", expires_in);
            }

            assert!(!token.access_token.is_empty());
        }
        Err(e) => panic!("Failed to refresh token: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn integration_oauth2_full_flow_client() {
    // This test demonstrates creating a client with an OAuth2 token
    let (consumer_key, consumer_secret) = get_consumer_credentials()
        .expect("Consumer credentials required");
    let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
        .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

    // Generate authorization URL
    let config = OAuth2Config::new(&consumer_key, &consumer_secret, &redirect_uri);
    let (auth_url, csrf_token) = config.authorize_url();

    println!("Step 1: Authorization URL generated");
    println!("  URL: {}...", &auth_url[..60]);
    println!("  CSRF: {}", csrf_token.secret());

    // If we have tokens from the helper, test creating a client with them
    if load_tokens_from_file().is_some() {
        println!("\nStep 2: Creating authenticated client from saved tokens");

        match test_client().await {
            Ok(client) => {
                // Test that the client works by fetching user info
                let result = client.users().info().await;

                match result {
                    Ok(info) => {
                        println!("\n✅ OAuth2 client working correctly!");
                        println!("Authenticated as: {}", info.user.name);
                    }
                    Err(e) => panic!("Failed to use OAuth2 client: {}", e),
                }
            }
            Err(e) => panic!("Failed to create client: {}", e),
        }
    } else {
        println!("\n⏭️  Skipping client creation - no tokens saved");
        println!("To test the full flow:");
        println!("  1. Run: cargo run --bin oauth");
        println!("  2. Run this test again");
    }
}
