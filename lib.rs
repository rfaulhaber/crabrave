//! # Crabrave
//!
//! An ergonomic Rust HTTP client for the Tumblr API.
//!
//! Inspired by [Octocrab](https://github.com/XAMPPRocky/octocrab), Crabrave provides
//! a type-safe, async interface for interacting with Tumblr's REST API.
//!
//! ## Quick Start
//!
//! ### Using Existing Credentials
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
//!
//! ### OAuth2 Flow (Getting Tokens)
//!
//! ```no_run
//! use crabrave::oauth::OAuth2Config;
//! use crabrave::Crabrave;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Create OAuth2 config
//! let config = OAuth2Config::new(
//!     "your_consumer_key",
//!     "your_consumer_secret",
//!     "http://localhost:8080/callback"
//! );
//!
//! // 2. Generate authorization URL
//! let (auth_url, csrf_token) = config.authorize_url();
//! println!("Visit: {}", auth_url);
//! // Direct user to auth_url, they'll be redirected back with a code
//!
//! // 3. Exchange authorization code for token
//! let code = "code_from_callback";
//! let token = config.exchange_code(code).await?;
//!
//! // 4. Create client with the token
//! let crab = Crabrave::builder()
//!     .consumer_key("your_consumer_key")
//!     .consumer_secret("your_consumer_secret")
//!     .access_token(&token.access_token)
//!     .build()?;
//!
//! // 5. Use the client
//! let blog_info = crab.blogs("staff").info().await?;
//! # Ok(())
//! # }
//! ```

mod error;
pub mod handlers;
pub mod media;
pub mod models;
pub mod npf;
pub mod oauth;
mod response;

pub use error::{CrabError, CrabResult};
pub use handlers::{Blogs, Communities, Tagged, Users};
pub use models::{Blog, BlogIdentifier, Page, User};
pub use response::{ApiResponse, EmptyResponse, Meta};

use base64::Engine;
use hmac::{Hmac, Mac};
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Deserializer};
use sha1::Sha1;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::handlers::blog::{AvatarResponse, AvatarResponseUrl};

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
#[derive(Clone)]
enum Auth {
    /// OAuth2 authentication with access token
    OAuth2 {
        #[allow(dead_code)] // Stored for potential token refresh, but not used in requests
        consumer_key: String,
        #[allow(dead_code)] // Stored for potential token refresh, but not used in requests
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

    /// Creates an API accessor for searching posts by tag
    ///
    /// This provides access to public posts across the platform that have been
    /// tagged with a specific tag.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to search for
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// // Search for posts tagged with "photography"
    /// let posts = crab.tagged("photography").limit(20).send().await?;
    ///
    /// for post in posts.posts {
    ///     println!("Post from {}: {}", post.blog_name, post.id);
    /// }
    ///
    /// // Search with timestamp filter
    /// let older_posts = crab.tagged("art").before(1234567890).send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tagged(&self, tag: impl Into<String>) -> handlers::Tagged {
        handlers::Tagged::new(self.clone(), tag.into())
    }

    /// Creates an API accessor for community operations
    ///
    /// This provides access to community timelines, membership management,
    /// and member lists.
    ///
    /// # Arguments
    ///
    /// * `handle` - Community handle/identifier
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
    /// // Get community timeline
    /// let timeline = crab.communities("rust-community")
    ///     .timeline()
    ///     .limit(20)
    ///     .send()
    ///     .await?;
    ///
    /// // Join a community
    /// crab.communities("rust-community").join().await?;
    ///
    /// // Get community members
    /// let members = crab.communities("rust-community")
    ///     .members(Some(20), None)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn communities(&self, handle: impl Into<String>) -> Communities {
        Communities::new(self.clone(), handle.into())
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

    /// Generates an OAuth1 signature for a request
    fn generate_oauth1_signature(
        &self,
        method: &str,
        url: &str,
        consumer_key: &str,
        consumer_secret: &str,
        access_token: &str,
        access_token_secret: &str,
    ) -> String {
        // Generate timestamp and nonce
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        // Generate cryptographically random nonce (32 alphanumeric characters)
        let nonce: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        // Collect OAuth parameters (using String keys and values to avoid lifetime issues)
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("oauth_consumer_key".to_string(), consumer_key.to_string());
        params.insert("oauth_nonce".to_string(), nonce.clone());
        params.insert(
            "oauth_signature_method".to_string(),
            "HMAC-SHA1".to_string(),
        );
        params.insert("oauth_timestamp".to_string(), timestamp.clone());
        params.insert("oauth_token".to_string(), access_token.to_string());
        params.insert("oauth_version".to_string(), "1.0".to_string());

        // Parse URL to extract query parameters
        if let Ok(parsed_url) = url::Url::parse(url) {
            for (key, value) in parsed_url.query_pairs() {
                params.insert(key.to_string(), value.to_string());
            }
        }

        // Build parameter string
        let param_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Build signature base string
        let base_url = url.split('?').next().unwrap_or(url);
        let signature_base = format!(
            "{}&{}&{}",
            urlencoding::encode(method),
            urlencoding::encode(base_url),
            urlencoding::encode(&param_string)
        );

        // Build signing key
        let signing_key = format!(
            "{}&{}",
            urlencoding::encode(consumer_secret),
            urlencoding::encode(access_token_secret)
        );

        // Generate HMAC-SHA1 signature
        type HmacSha1 = Hmac<Sha1>;
        let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes())
            .unwrap_or_else(|_| panic!("HMAC can take key of any size"));
        mac.update(signature_base.as_bytes());
        let result = mac.finalize();
        let signature = base64::engine::general_purpose::STANDARD.encode(result.into_bytes());

        // Build Authorization header
        format!(
            r#"OAuth oauth_consumer_key="{}", oauth_nonce="{}", oauth_signature="{}", oauth_signature_method="HMAC-SHA1", oauth_timestamp="{}", oauth_token="{}", oauth_version="1.0""#,
            urlencoding::encode(consumer_key),
            urlencoding::encode(&nonce),
            urlencoding::encode(&signature),
            timestamp,
            urlencoding::encode(access_token)
        )
    }

    /// Applies authentication to a request builder based on the auth type
    fn apply_auth(
        &self,
        mut request: reqwest::RequestBuilder,
        method: &str,
        url: &str,
    ) -> reqwest::RequestBuilder {
        match self.auth.as_ref() {
            Auth::OAuth2 { access_token, .. } => {
                // Add Bearer token to Authorization header
                request = request.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", access_token),
                );
            }
            Auth::OAuth1 {
                consumer_key,
                consumer_secret,
                access_token,
                access_token_secret,
            } => {
                // Generate OAuth1 signature and add Authorization header
                let auth_header = self.generate_oauth1_signature(
                    method,
                    url,
                    consumer_key,
                    consumer_secret,
                    access_token,
                    access_token_secret,
                );
                request = request.header(reqwest::header::AUTHORIZATION, auth_header);
            }
            Auth::ApiKey { consumer_key } => {
                // Add API key as query parameter
                request = request.query(&[("api_key", consumer_key)]);
            }
        }
        request
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
        let request = self.client.get(&url);
        let request = self.apply_auth(request, "GET", &url);
        let response = request.send().await?;

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

    /// Makes a GET request with query parameters to the API
    ///
    /// This is an internal helper method used by handlers.
    /// The query parameter is serialized using serde, allowing for type-safe
    /// query parameters with automatic URL encoding.
    #[allow(dead_code)]
    pub(crate) async fn get_with_query<T, Q>(&self, path: &str, query: &Q) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        Q: serde::Serialize,
    {
        // Serialize query params to build the full URL for OAuth1 signature
        let query_string = serde_urlencoded::to_string(query).map_err(|e| {
            CrabError::InvalidResponse(format!("Failed to serialize query params: {}", e))
        })?;

        let base_url = self.url(path);
        let full_url = if query_string.is_empty() {
            base_url.clone()
        } else {
            format!("{}?{}", base_url, query_string)
        };

        let request = self.client.get(&base_url).query(query);
        let request = self.apply_auth(request, "GET", &full_url);
        let response = request.send().await?;

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

    pub(crate) async fn delete_with_query<T, Q>(&self, path: &str, query: &Q) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        Q: serde::Serialize,
    {
        // Serialize query params to build the full URL for OAuth1 signature
        let query_string = serde_urlencoded::to_string(query).map_err(|e| {
            CrabError::InvalidResponse(format!("Failed to serialize query params: {}", e))
        })?;

        let base_url = self.url(path);
        let full_url = if query_string.is_empty() {
            base_url.clone()
        } else {
            format!("{}?{}", base_url, query_string)
        };

        let request = self.client.delete(&base_url).query(query);
        let request = self.apply_auth(request, "DELETE", &full_url);
        let response = request.send().await?;

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

    /// A special variant of the generic GET but for handling the /blog/avatar endpoint specifically.
    /// The endpoint will return binary data if the request sent to it is not OAuth1, so we have to handle the response as a special case.
    pub(crate) async fn get_avatar(&self, path: &str) -> CrabResult<AvatarResponse> {
        let url = self.url(path);
        let request = self.client.get(&url);
        let request = self.apply_auth(request, "GET", &url);
        let response = request.send().await?;

        // Check for rate limiting
        if response.status().as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());

            return Err(CrabError::RateLimit { retry_after });
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = response.bytes().await?;

        // tumblr always returns PNGs
        if &content_type == "image/png" {
            Ok(AvatarResponse::ImageData(bytes.to_vec()))
        } else {
            let response: AvatarResponseUrl = response::parse_response_bytes(&bytes)?;
            Ok(AvatarResponse::ImageUrl {
                avatar_url: response.avatar_url,
            })
        }
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
        let request = self.client.post(&url).json(body);
        let request = self.apply_auth(request, "POST", &url);
        let response = request.send().await?;

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

    /// Makes a PUT request to the API
    ///
    /// This is an internal helper method used by handlers.
    #[allow(dead_code)]
    pub(crate) async fn put<T, B>(&self, path: &str, body: &B) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.url(path);
        let request = self.client.put(&url).json(body);
        let request = self.apply_auth(request, "PUT", &url);
        let response = request.send().await?;

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

    /// Makes a POST request with multipart/form-data to the API
    ///
    /// This is used for uploading media files along with post content.
    /// The request includes a "json" field containing the serialized body,
    /// plus additional fields for each media file keyed by their identifier.
    ///
    /// # Arguments
    ///
    /// * `path` - API endpoint path
    /// * `body` - JSON body to serialize
    /// * `media_sources` - Map of identifiers to media sources
    pub(crate) async fn post_multipart<T, B>(
        &self,
        path: &str,
        body: &B,
        media_sources: std::collections::HashMap<String, media::MediaSource>,
    ) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.url(path);

        // Build multipart form
        let mut form = reqwest::multipart::Form::new();

        // Add JSON part
        let json_str = serde_json::to_string(body).map_err(|e| {
            CrabError::InvalidResponse(format!("Failed to serialize request body: {}", e))
        })?;
        form = form.text("json", json_str);

        // Add media parts
        for (identifier, source) in media_sources {
            let bytes = source.read_bytes().map_err(|e| {
                CrabError::InvalidResponse(format!("Failed to read media source: {}", e))
            })?;

            let mut part = reqwest::multipart::Part::bytes(bytes).file_name(source.filename().to_string());

            if let Some(mime_type) = source.mime_type() {
                part = part.mime_str(mime_type).map_err(|e| {
                    CrabError::InvalidResponse(format!("Invalid MIME type '{}': {}", mime_type, e))
                })?;
            }

            form = form.part(identifier, part);
        }

        let request = self.client.post(&url).multipart(form);
        let request = self.apply_auth(request, "POST", &url);
        let response = request.send().await?;

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

    /// Makes a PUT request with multipart/form-data to the API
    ///
    /// Similar to post_multipart but uses PUT method for editing existing posts.
    pub(crate) async fn put_multipart<T, B>(
        &self,
        path: &str,
        body: &B,
        media_sources: std::collections::HashMap<String, media::MediaSource>,
    ) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = self.url(path);

        // Build multipart form
        let mut form = reqwest::multipart::Form::new();

        // Add JSON part
        let json_str = serde_json::to_string(body).map_err(|e| {
            CrabError::InvalidResponse(format!("Failed to serialize request body: {}", e))
        })?;
        form = form.text("json", json_str);

        // Add media parts
        for (identifier, source) in media_sources {
            let bytes = source.read_bytes().map_err(|e| {
                CrabError::InvalidResponse(format!("Failed to read media source: {}", e))
            })?;

            let mut part = reqwest::multipart::Part::bytes(bytes).file_name(source.filename().to_string());

            if let Some(mime_type) = source.mime_type() {
                part = part.mime_str(mime_type).map_err(|e| {
                    CrabError::InvalidResponse(format!("Invalid MIME type '{}': {}", mime_type, e))
                })?;
            }

            form = form.part(identifier, part);
        }

        let request = self.client.put(&url).multipart(form);
        let request = self.apply_auth(request, "PUT", &url);
        let response = request.send().await?;

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
        let request = self.client.delete(&url);
        let request = self.apply_auth(request, "DELETE", &url);
        let response = request.send().await?;

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

pub(crate) fn empty_object_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum EmptyOrValue<T> {
        Value(T),
        Empty {},
    }

    match EmptyOrValue::deserialize(deserializer)? {
        EmptyOrValue::Value(v) => Ok(Some(v)),
        EmptyOrValue::Empty {} => Ok(None),
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
