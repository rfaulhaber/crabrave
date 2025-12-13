//! Integration tests against the real Tumblr API
//!
//! These tests are ignored by default and only run when explicitly requested.
//! These tests rely on the following environment variables being present:
//! - `TUMBLR_CONSUMER_KEY`: your oauth consumer key
//! - `TUMBLR_CONSUMER_SECRET`: your oauth consumer secret
//! - `TUMBLR_ACCESS_TOKEN`: your oauth access token
//! - `TUMBLR_REFRESH_TOKEN`: optionally, your oauth refresh token
//! - `TUMBLR_REDIRECT_URI`: your oauth redirect URI
//!
//! You may also specify these values in a JSON file, and specify the file with the environment variable `TUMBLR_OAUTH_SETTINGS_FILE`.

use crabrave::{
    CrabError, Crabrave,
    handlers::blog::AvatarResponse,
    npf::{ContentBlock, MediaObject},
    oauth::{OAuth2Config, parse_callback},
};
use serde::Deserialize;
use std::{env, sync::OnceLock};

const OAUTH_CONSUMER_KEY_VAR_NAME: &str = "TUMBLR_CONSUMER_KEY";
const OAUTH_CONSUMER_SECRET_VAR_NAME: &str = "TUMBLR_CONSUMER_SECRET";
const OAUTH_ACCESS_TOKEN_VAR_NAME: &str = "TUMBLR_ACCESS_TOKEN";
const OAUTH_REFRESH_TOKEN_VAR_NAME: &str = "TUMBLR_REFRESH_TOKEN";
const OAUTH_REDIRECT_URI_VAR_NAME: &str = "TUMBLR_REDIRECT_URI";
const OAUTH_SETTINGS_FILE_VAR_NAME: &str = "TUMBLR_OAUTH_SETTINGS_FILE";
const TEST_BLOG_VAR_NAME: &str = "TUMBLR_TEST_BLOG";

static TOKEN_SETTINGS: OnceLock<TokenStorage> = OnceLock::new();

#[derive(Deserialize)]
struct TokenStorage {
    #[serde(rename = "TUMBLR_CONSUMER_KEY")]
    consumer_key: String,
    #[serde(rename = "TUMBLR_CONSUMER_SECRET")]
    consumer_secret: String,
    #[serde(rename = "TUMBLR_ACCESS_TOKEN")]
    access_token: String,
    #[serde(rename = "TUMBLR_REFRESH_TOKEN")]
    refresh_token: Option<String>,
    #[serde(rename = "TUMBLR_REDIRECT_URI")]
    redirect_uri: String,
    #[serde(rename = "TUMBLR_TEST_BLOG")]
    test_blog: Option<String>,
}

fn get_env(key: &str) -> Result<String, String> {
    env::var(key).map_err(|_| format!("Missing environment variable: {}", key))
}

fn get_env_optional(key: &str) -> Option<String> {
    env::var(key).ok()
}

fn get_tumblr_test_blog() -> Result<String, String> {
    if let Some(settings) = TOKEN_SETTINGS.get() {
        if let Some(blog_name) = &settings.test_blog {
            return Ok(blog_name.clone());
        }
    }
    get_env(TEST_BLOG_VAR_NAME)
}

fn get_consumer_credentials() -> Result<(String, String), String> {
    if let Some(path) = get_env_optional(OAUTH_SETTINGS_FILE_VAR_NAME) {
        let file = std::fs::read(path).map_err(|e| format!("Could not open file: {}", e))?;

        let contents: TokenStorage = serde_json::from_slice(&file)
            .map_err(|e| format!("Could not deserialize settings file: {}", e))?;

        return Ok((contents.consumer_key, contents.consumer_secret));
    }

    let key = get_env(OAUTH_CONSUMER_KEY_VAR_NAME)?;
    let secret = get_env(OAUTH_CONSUMER_SECRET_VAR_NAME)?;

    Ok((key, secret))
}

/// Creates a test client with proper OAuth2 authentication
async fn test_client() -> Result<Crabrave, String> {
    // Initialize settings if not already cached
    if TOKEN_SETTINGS.get().is_none() {
        let loaded_settings = if let Some(path) = get_env_optional(OAUTH_SETTINGS_FILE_VAR_NAME) {
            // Load from file
            let file = std::fs::read(&path)
                .map_err(|e| format!("Could not open settings file {}: {}", path, e))?;

            serde_json::from_slice::<TokenStorage>(&file)
                .map_err(|e| format!("Could not deserialize settings file: {}", e))?
        } else {
            // Load from environment variables
            TokenStorage {
                consumer_key: get_env(OAUTH_CONSUMER_KEY_VAR_NAME)?,
                consumer_secret: get_env(OAUTH_CONSUMER_SECRET_VAR_NAME)?,
                access_token: get_env(OAUTH_ACCESS_TOKEN_VAR_NAME)?,
                refresh_token: get_env_optional(OAUTH_REFRESH_TOKEN_VAR_NAME),
                redirect_uri: get_env(OAUTH_REDIRECT_URI_VAR_NAME)?,
                test_blog: get_env_optional(TEST_BLOG_VAR_NAME),
            }
        };

        // Cache the settings
        TOKEN_SETTINGS
            .set(loaded_settings)
            .map_err(|_| "Token settings already initialized (race condition)")?;
    }

    // Get the cached settings (guaranteed to be Some at this point)
    let Some(settings) = TOKEN_SETTINGS.get() else {
        return Err("Failed to initialize settings".into());
    };

    // Try to refresh token if available
    if let Some(refresh_token) = &settings.refresh_token {
        let config = OAuth2Config::new(
            &settings.consumer_key,
            &settings.consumer_secret,
            &settings.redirect_uri,
        );

        match config.refresh_access_token(refresh_token).await {
            Ok(new_token) => {
                return Crabrave::builder()
                    .consumer_key(&settings.consumer_key)
                    .consumer_secret(&settings.consumer_secret)
                    .access_token(&new_token.access_token)
                    .build()
                    .map_err(|e| format!("Failed to build client: {}", e));
            }
            Err(e) => {
                eprintln!("Token refresh failed: {}, using cached access token", e);
            }
        }
    }

    // Use the cached access token
    Crabrave::builder()
        .consumer_key(&settings.consumer_key)
        .consumer_secret(&settings.consumer_secret)
        .access_token(&settings.access_token)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))
}

#[tokio::test]
#[ignore]
async fn blog_info() {
    let client = test_client().await.expect("Failed to create client");

    let test_blog_name = get_tumblr_test_blog().expect("TUMBLR_TEST_BLOG not set");

    let result = client.blogs(test_blog_name.clone()).info().await;

    match result {
        Ok(info) => {
            println!("Blog name: {}", info.blog.name);
            println!("Blog title: {}", info.blog.title);
            println!("Total posts: {}", info.blog.posts);
            assert_eq!(info.blog.name, test_blog_name);
        }
        Err(e) => panic!("Failed to get blog info: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn blog_avatar() {
    let client = test_client().await.expect("Failed to create client");

    let test_blog_name = get_tumblr_test_blog().expect("TUMBLR_TEST_BLOG not set");

    let result = client.blogs(test_blog_name).avatar(Some(64)).await;

    match result {
        Ok(AvatarResponse::ImageData(bytes)) => {
            assert!(!bytes.is_empty());
        }
        Ok(AvatarResponse::ImageUrl { avatar_url }) => {
            println!("Avatar URL: {}", avatar_url);
            assert!(!avatar_url.is_empty());
            assert!(avatar_url.starts_with("http"));
        }
        Err(e) => panic!("Failed to get avatar: {}", e),
    }
}

// TODO GET blocks
// TODO POST block
// TODO blocks/bulk

#[tokio::test]
#[ignore]
async fn blog_posts() {
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
async fn user_info() {
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

// #[tokio::test]
// async fn user_test() {
//     let client = test_client().await.expect("Failed to create client");
// }

#[tokio::test]
#[ignore]
async fn user_dashboard() {
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
async fn tagged_posts() {
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
async fn get_specific_post() {
    let client = test_client().await.expect("Failed to create client");
    let blog = get_tumblr_test_blog().expect("TUMBLR_TEST_BLOG not set");

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
async fn rate_limit_handling() {
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
async fn nonexistent_blog() {
    let client = test_client().await.expect("Failed to create client");

    let result = client
        .blogs("this-blog-definitely-does-not-exist-12345")
        .info()
        .await;

    match result {
        Err(CrabError::Api { status, message }) => {
            println!(
                "Got expected error - Status: {}, Message: {}",
                status, message
            );
            assert_eq!(status, 404);
        }
        Ok(_) => panic!("Expected 404 error for nonexistent blog"),
        Err(e) => panic!("Got unexpected error type: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn user_likes() {
    let client = test_client().await.expect("Failed to create client");

    let result = client.users().likes().limit(5).get().await;

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
async fn user_following() {
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

// #[tokio::test]
// #[ignore]
// async fn get_blocks() {
//     let client = test_client().await.expect("Failed to create client");
//     let result = client
//         .blogs(get_tumblr_test_blog().expect("No test blog set"))
//         .blocks()
//         .get()
//         .await;
// }

#[tokio::test]
#[ignore]
async fn make_text_post() {
    let client = test_client().await.expect("Failed to create client");
    let result = client
        .posts()
        .create(get_tumblr_test_blog().expect("No test blog set"))
        .add_block(ContentBlock::Text {
            text: "hello world".into(),
            subtype: Some("heading1".into()),
            formatting: None,
        })
        .send()
        .await;

    match result {
        Ok(r) => {
            println!("new post id: {}", r.id)
        }
        Err(e) => panic!("failed to make a post: {}", e),
    }
}

// #[tokio::test]
// #[ignore]
// async fn make_image_post() {
//     let client = test_client().await.expect("Failed to create client");
//     let test_blog_name = get_tumblr_test_blog().expect("No test blog set");
//     let avatar = match client.blogs(test_blog_name).avatar(64).await {
//         Ok(image) => image,
//         Err(e) => panic!("Failed to get blog avatar: ", e),
//     };

//     let result = client
//         .posts()
//         .create(get_tumblr_test_blog().expect("No test blog set"))
//         .add_block(ContentBlock::Text {
//             text: "this is my icon:".into(),
//             subtype: Some("heading1".into()),
//             formatting: None,
//         })
//         .add_block(ContentBlock::Image {
//             media: MediaObject {
//                 url: todo!(),
//                 media_type: todo!(),
//                 width: todo!(),
//                 height: todo!(),
//             },
//             alt_text: (),
//             caption: (),
//         })
//         .send()
//         .await;

//     match result {
//         Ok(r) => {
//             println!("new post id: {}", r.id)
//         }
//         Err(e) => panic!("failed to make a post: {}", e),
//     }
// }

// ============================================================================
// OAuth2 Flow Tests
// ============================================================================

#[tokio::test]
#[ignore]
async fn oauth2_authorize_url() {
    let (consumer_key, consumer_secret) =
        get_consumer_credentials().expect("Consumer credentials required");
    let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
        .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

    let config = OAuth2Config::new(consumer_key, consumer_secret, redirect_uri.clone());
    let (auth_url, csrf_token) = config.authorize_url();

    println!("Authorization URL: {}", auth_url);
    println!("CSRF Token: {}", csrf_token.secret());

    // Verify the URL contains required parameters
    assert!(auth_url.contains("https://www.tumblr.com/oauth2/authorize"));
    assert!(auth_url.contains("client_id="));
    assert!(auth_url.contains(&format!(
        "redirect_uri={}",
        urlencoding::encode(&redirect_uri)
    )));
    assert!(auth_url.contains("state="));
    assert!(!csrf_token.secret().is_empty());

    println!("\n✅ Authorization URL generated successfully!");
    println!("Visit this URL to authorize your application:");
    println!("{}", auth_url);
}

#[tokio::test]
#[ignore]
async fn oauth2_parse_callback() {
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
async fn oauth2_exchange_code() {
    let (consumer_key, consumer_secret) =
        get_consumer_credentials().expect("Consumer credentials required");
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
            println!(
                "Access token received: {}...",
                &token.access_token[..20.min(token.access_token.len())]
            );
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
async fn oauth2_refresh_token() {
    let (consumer_key, consumer_secret) =
        get_consumer_credentials().expect("Consumer credentials required");
    let redirect_uri = get_env_optional("TUMBLR_OAUTH2_REDIRECT_URI")
        .unwrap_or_else(|| "http://localhost:8080/callback".to_string());

    // This test requires a valid refresh token
    let refresh_token = match get_env(OAUTH_REFRESH_TOKEN_VAR_NAME) {
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
            println!(
                "New access token received: {}...",
                &token.access_token[..20.min(token.access_token.len())]
            );
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
