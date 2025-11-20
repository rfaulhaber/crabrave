//! # Crabrave
//!
//! An ergonomic Rust HTTP client for the Tumblr API.
//!
//! Inspired by [Octocrab](https://github.com/XAMPPRocky/octocrab), Crabrave provides
//! a type-safe, async interface for interacting with Tumblr's REST API.
//!
//! ## Example
//!
//! ```
//! use crabrave::Crabrave;
//!
//! // Build a client with OAuth2 credentials
//! let crab = Crabrave::builder()
//!     .consumer_key("your_consumer_key")
//!     .consumer_secret("your_consumer_secret")
//!     .access_token("your_access_token")
//!     .build()?;
//!
//! // Use the client (requires async runtime)
//! // let blog_info = crab.blogs("staff").info().await?;
//! # Ok::<(), crabrave::CrabError>(())
//! ```

mod error;
pub mod handlers;
pub mod models;
mod response;

pub use error::{CrabError, CrabResult};
pub use handlers::{Blogs, Users};
pub use models::{Blog, BlogIdentifier, Page, User};
pub use response::{ApiResponse, Meta};

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::sync::Arc;

/// Base URL for the Tumblr API v2
pub const BASE_API_URL: &str = "https://api.tumblr.com/v2";

/// OAuth2 authorization endpoint
pub const OAUTH_AUTHORIZE_URL: &str = "https://www.tumblr.com/oauth2/authorize";

/// OAuth2 token endpoint
pub const OAUTH_TOKEN_URL: &str = "https://api.tumblr.com/v2/oauth2/token";

/// Default User-Agent header value
const DEFAULT_USER_AGENT: &str = concat!(
    "crabrave/",
    env!("CARGO_PKG_VERSION"),
    " (Rust HTTP Client for Tumblr)"
);

/// Authentication credentials for the Tumblr API
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields will be used when implementing API requests
enum Auth {
    /// OAuth2 authentication with access token
    OAuth2 {
        consumer_key: String,
        consumer_secret: String,
        access_token: String,
    },
    /// OAuth1 authentication (legacy)
    OAuth1 {
        consumer_key: String,
        consumer_secret: String,
        access_token: String,
        access_token_secret: String,
    },
    /// API key only (read-only access)
    ApiKey { consumer_key: String },
}

/// The main client for interacting with the Tumblr API
#[derive(Clone)]
pub struct Crabrave {
    client: reqwest::Client,
    base_url: String,
    #[allow(dead_code)] // Will be used when implementing API requests
    auth: Arc<Auth>,
}

impl Crabrave {
    /// Creates a new builder for constructing a `Crabrave` client
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crabrave::Crabrave;
    ///
    /// let crab = Crabrave::builder()
    ///     .consumer_key("key")
    ///     .consumer_secret("secret")
    ///     .access_token("token")
    ///     .build()?;
    /// # Ok::<(), crabrave::CrabError>(())
    /// ```
    pub fn builder() -> CrabraveBuilder {
        CrabraveBuilder::new()
    }

    /// Gets the base URL for API requests
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Gets a reference to the underlying reqwest client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Creates an API accessor for blog-related operations
    ///
    /// # Arguments
    ///
    /// * `identifier` - Blog identifier (name, hostname, or UUID)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// // Using blog name
    /// let info = crab.blogs("staff").info().await?;
    ///
    /// // Using hostname
    /// let info = crab.blogs("staff.tumblr.com").info().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn blogs(&self, identifier: impl Into<BlogIdentifier>) -> Blogs {
        Blogs::new(self.clone(), identifier.into())
    }

    /// Creates an API accessor for user-related operations
    ///
    /// This provides access to the authenticated user's dashboard,
    /// likes, following list, and other user-specific operations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// // Get user info
    /// let info = crab.users().info().await?;
    ///
    /// // Get dashboard posts
    /// let posts = crab.users().dashboard().limit(20).send().await?;
    ///
    /// // Follow a blog
    /// crab.users().follow("staff").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn users(&self) -> Users {
        Users::new(self.clone())
    }

    /// Constructs a full URL for an API endpoint
    ///
    /// # Arguments
    ///
    /// * `path` - The API endpoint path (e.g., "/blog/staff/info")
    pub(crate) fn url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, path)
    }

    /// Makes a GET request to the API
    ///
    /// This is an internal helper method used by handlers.
    #[allow(dead_code)]
    pub(crate) async fn get<T>(&self, path: &str) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = self.url(path);
        let response = self.client.get(&url).send().await?;

        // Check for rate limiting
        if response.status().as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());

            return Err(CrabError::RateLimit { retry_after });
        }

        let bytes = response.bytes().await?;
        response::parse_response_bytes(&bytes)
    }

    /// Makes a POST request to the API
    ///
    /// This is an internal helper method used by handlers.
    #[allow(dead_code)]
    pub(crate) async fn post<T, B>(&self, path: &str, body: &B) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.url(path);
        let response = self.client.post(&url).json(body).send().await?;

        // Check for rate limiting
        if response.status().as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());

            return Err(CrabError::RateLimit { retry_after });
        }

        let bytes = response.bytes().await?;
        response::parse_response_bytes(&bytes)
    }

    /// Makes a DELETE request to the API
    ///
    /// This is an internal helper method used by handlers.
    #[allow(dead_code)]
    pub(crate) async fn delete<T>(&self, path: &str) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = self.url(path);
        let response = self.client.delete(&url).send().await?;

        // Check for rate limiting
        if response.status().as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());

            return Err(CrabError::RateLimit { retry_after });
        }

        let bytes = response.bytes().await?;
        response::parse_response_bytes(&bytes)
    }
}

/// Builder for constructing a `Crabrave` client
///
/// This builder allows you to configure authentication credentials and client settings
/// before building the final client instance.
pub struct CrabraveBuilder {
    consumer_key: Option<String>,
    consumer_secret: Option<String>,
    access_token: Option<String>,
    access_token_secret: Option<String>,
    user_agent: Option<String>,
    base_url: Option<String>,
}

impl CrabraveBuilder {
    /// Creates a new builder with default settings
    fn new() -> Self {
        Self {
            consumer_key: None,
            consumer_secret: None,
            access_token: None,
            access_token_secret: None,
            user_agent: None,
            base_url: None,
        }
    }

    /// Sets the OAuth consumer key (also known as API key or client ID)
    ///
    /// This is required for all authentication methods.
    pub fn consumer_key(mut self, key: impl Into<String>) -> Self {
        self.consumer_key = Some(key.into());
        self
    }

    /// Sets the OAuth consumer secret (also known as API secret or client secret)
    ///
    /// This is required for OAuth authentication.
    pub fn consumer_secret(mut self, secret: impl Into<String>) -> Self {
        self.consumer_secret = Some(secret.into());
        self
    }

    /// Sets the OAuth access token
    ///
    /// This is the token obtained after completing the OAuth flow.
    pub fn access_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }

    /// Sets the OAuth1 access token secret
    ///
    /// This is only required for OAuth1 authentication (legacy).
    /// If provided, OAuth1 will be used instead of OAuth2.
    pub fn access_token_secret(mut self, secret: impl Into<String>) -> Self {
        self.access_token_secret = Some(secret.into());
        self
    }

    /// Sets a custom User-Agent header
    ///
    /// If not set, a default User-Agent will be used.
    /// Note: Tumblr requires a User-Agent header and may suspend applications without one.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Sets a custom base URL for API requests
    ///
    /// This is primarily useful for testing. The default is the official Tumblr API URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Builds the `Crabrave` client
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required credentials are missing
    /// - The HTTP client cannot be constructed
    pub fn build(self) -> CrabResult<Crabrave> {
        // Validate required credentials
        let consumer_key = self.consumer_key.ok_or(CrabError::MissingConsumerKey)?;

        // Determine authentication method based on provided credentials
        let auth = if let Some(access_token_secret) = self.access_token_secret {
            // OAuth1 authentication
            let consumer_secret = self
                .consumer_secret
                .ok_or(CrabError::MissingConsumerSecret)?;
            let access_token = self.access_token.ok_or(CrabError::MissingAccessToken)?;

            Auth::OAuth1 {
                consumer_key,
                consumer_secret,
                access_token,
                access_token_secret,
            }
        } else if let Some(access_token) = self.access_token {
            // OAuth2 authentication
            let consumer_secret = self
                .consumer_secret
                .ok_or(CrabError::MissingConsumerSecret)?;

            Auth::OAuth2 {
                consumer_key,
                consumer_secret,
                access_token,
            }
        } else {
            // API key only (read-only)
            Auth::ApiKey { consumer_key }
        };

        // Build headers
        let mut headers = HeaderMap::new();
        let user_agent = self
            .user_agent
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent).map_err(|_| CrabError::InvalidUserAgent)?,
        );

        // Build reqwest client
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(CrabError::HttpClient)?;

        let base_url = self.base_url.unwrap_or_else(|| BASE_API_URL.to_string());

        Ok(Crabrave {
            client,
            base_url,
            auth: Arc::new(auth),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_oauth2() {
        let result = Crabrave::builder()
            .consumer_key("test_key")
            .consumer_secret("test_secret")
            .access_token("test_token")
            .build();

        assert!(result.is_ok());
        let crab = result.unwrap();
        assert_eq!(crab.base_url(), BASE_API_URL);
    }

    #[test]
    fn test_builder_oauth1() {
        let result = Crabrave::builder()
            .consumer_key("test_key")
            .consumer_secret("test_secret")
            .access_token("test_token")
            .access_token_secret("test_token_secret")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_api_key_only() {
        let result = Crabrave::builder().consumer_key("test_key").build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_missing_consumer_key() {
        let result = Crabrave::builder()
            .consumer_secret("test_secret")
            .access_token("test_token")
            .build();

        assert!(matches!(result, Err(CrabError::MissingConsumerKey)));
    }

    #[test]
    fn test_builder_custom_base_url() {
        let custom_url = "https://test.example.com/api";
        let crab = Crabrave::builder()
            .consumer_key("test_key")
            .base_url(custom_url)
            .build()
            .unwrap();

        assert_eq!(crab.base_url(), custom_url);
    }

    #[test]
    fn test_builder_custom_user_agent() {
        let result = Crabrave::builder()
            .consumer_key("test_key")
            .user_agent("CustomAgent/1.0")
            .build();

        assert!(result.is_ok());
    }
}
