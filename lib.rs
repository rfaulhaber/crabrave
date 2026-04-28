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
//! use crabrave::oauth::{OAuth2Config, OAuthScope};
//! use crabrave::Crabrave;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Create OAuth2 config
//! let config = OAuth2Config::new(
//!     "your_consumer_key",
//!     "your_consumer_secret",
//!     "http://localhost:8080/callback",
//!     vec![OAuthScope::Basic],
//!     
//! )?;
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
#[cfg(feature = "cookies")]
pub use reqwest::cookie::Jar as CookieJar;

use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Deserializer};
use std::collections::HashSet;
use std::sync::Arc;

use crate::handlers::blog::{AvatarResponse, AvatarResponseUrl};
use crate::oauth::OAuthScope;

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
struct Auth {
    #[allow(dead_code)] // Stored for potential token refresh, but not used in requests
    consumer_key: String,
    #[allow(dead_code)] // Stored for potential token refresh, but not used in requests
    consumer_secret: String,
    access_token: String,
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
    ///     .members()
    ///     .limit(20)
    ///     .send()
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

    /// Checks if the response indicates rate limiting and returns an error if so.
    fn check_rate_limit(response: &reqwest::Response) -> CrabResult<()> {
        fn get_header_number_value(header_name: &str, headers: &HeaderMap) -> Option<u64> {
            headers
                .get(header_name)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
        }

        if response.status().as_u16() == 429 {
            let headers = response.headers();

            let per_hour_remaining =
                get_header_number_value("x-ratelimit-perhour-remaining", headers);

            let per_day_remaining =
                get_header_number_value("x-ratelimit-perday-remaining", headers);

            // the API should return a 0 value for one of these headers if we hit the limit
            let retry_after = match (per_hour_remaining, per_day_remaining) {
                (_, Some(0)) => get_header_number_value("x-ratelimit-perday-reset", headers),
                (Some(0), _) => get_header_number_value("x-ratelimit-perhour-reset", headers),
                _ => None,
            };

            return Err(CrabError::RateLimit { retry_after });
        }
        Ok(())
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
        let request = self.client.get(&url).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );
        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

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
        let base_url = self.url(path);

        let request = self.client.get(&base_url).query(query).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );

        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

        let bytes = response.bytes().await?;

        response::parse_response_bytes(&bytes)
    }

    pub(crate) async fn delete_with_query<T, Q>(&self, path: &str, query: &Q) -> CrabResult<T>
    where
        T: serde::de::DeserializeOwned,
        Q: serde::Serialize,
    {
        let base_url = self.url(path);

        let request = self.client.delete(&base_url).query(query).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );

        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

        let bytes = response.bytes().await?;

        response::parse_response_bytes(&bytes)
    }

    /// A special variant of the generic GET but for handling the /blog/avatar endpoint specifically
    pub(crate) async fn get_avatar(&self, path: &str) -> CrabResult<AvatarResponse> {
        let url = self.url(path);
        let request = self.client.get(&url).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );
        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

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
        let request = self.client.post(&url).json(body).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );
        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

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
        let request = self.client.put(&url).json(body).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );
        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

        let bytes = response.bytes().await?;
        response::parse_response_bytes(&bytes)
    }

    /// Makes a multipart/form-data request to the API using the given HTTP method.
    ///
    /// This is used for uploading media files along with post content.
    /// The request includes a "json" field containing the serialized body,
    /// plus additional fields for each media file keyed by their identifier.
    async fn send_multipart<T, B>(
        &self,
        method: reqwest::Method,
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

            let mut part =
                reqwest::multipart::Part::bytes(bytes).file_name(source.filename().to_string());

            if let Some(mime_type) = source.mime_type() {
                part = part.mime_str(mime_type).map_err(|e| {
                    CrabError::InvalidResponse(format!("Invalid MIME type '{}': {}", mime_type, e))
                })?;
            }

            form = form.part(identifier, part);
        }

        let request = self
            .client
            .request(method.clone(), &url)
            .multipart(form)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.auth.access_token),
            );
        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

        let bytes = response.bytes().await?;
        response::parse_response_bytes(&bytes)
    }

    /// Makes a POST request with multipart/form-data to the API
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
        self.send_multipart(reqwest::Method::POST, path, body, media_sources)
            .await
    }

    /// Makes a PUT request with multipart/form-data to the API
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
        self.send_multipart(reqwest::Method::PUT, path, body, media_sources)
            .await
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
        let request = self.client.delete(&url).header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", self.auth.access_token),
        );
        let response = request.send().await?;

        Self::check_rate_limit(&response)?;

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
    user_agent: Option<String>,
    base_url: Option<String>,
    scopes: HashSet<OAuthScope>,
    #[cfg(feature = "cookies")]
    cookie_jar: Option<Arc<reqwest::cookie::Jar>>,
}

impl CrabraveBuilder {
    /// Creates a new builder with default settings
    fn new() -> Self {
        let mut scopes = HashSet::new();
        scopes.insert(OAuthScope::Basic);

        Self {
            consumer_key: None,
            consumer_secret: None,
            access_token: None,
            user_agent: None,
            base_url: None,
            scopes,
            #[cfg(feature = "cookies")]
            cookie_jar: None,
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

    /// Adds a new OAuth scope to this client.
    /// Note that by default, using the builder, the "basic" scope is added by default.
    pub fn add_scope(mut self, scope: OAuthScope) -> Self {
        self.scopes.insert(scope);
        self
    }

    /// Adds new OAuth scopes to this client.
    /// Note that by default, using the builder, the "basic" scope is added by default.
    pub fn add_scopes<S: IntoIterator<Item = OAuthScope>>(mut self, scope: S) -> Self {
        for scope in scope.into_iter() {
            self.scopes.insert(scope);
        }

        self
    }

    /// Sets a pre-built cookie jar on the client.
    ///
    /// This allows you to provide browser cookies that will be sent with every request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crabrave::{Crabrave, CookieJar};
    /// use std::sync::Arc;
    ///
    /// let jar = CookieJar::default();
    /// jar.add_cookie_str(
    ///     "session_id=abc123",
    ///     &"https://www.tumblr.com".parse().unwrap(),
    /// );
    ///
    /// let crab = Crabrave::builder()
    ///     .consumer_key("key")
    ///     .consumer_secret("secret")
    ///     .access_token("token")
    ///     .cookie_jar(Arc::new(jar))
    ///     .build()?;
    /// # Ok::<(), crabrave::CrabError>(())
    /// ```
    #[cfg(feature = "cookies")]
    pub fn cookie_jar(mut self, jar: Arc<reqwest::cookie::Jar>) -> Self {
        self.cookie_jar = Some(jar);
        self
    }

    /// Adds a cookie string to the client's cookie jar.
    ///
    /// If no cookie jar has been set, one will be created automatically.
    /// The cookie will be associated with the given URL for domain matching.
    ///
    /// # Arguments
    ///
    /// * `cookie` - A cookie string in the format `"name=value"` (additional attributes like
    ///   `Path`, `Domain`, etc. may also be included)
    /// * `url` - The URL to associate the cookie with (used for domain matching)
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
    ///     .add_cookie("session_id=abc123", &"https://www.tumblr.com".parse().unwrap())
    ///     .add_cookie("pfp=xyz789", &"https://www.tumblr.com".parse().unwrap())
    ///     .build()?;
    /// # Ok::<(), crabrave::CrabError>(())
    /// ```
    #[cfg(feature = "cookies")]
    pub fn add_cookie(mut self, cookie: &str, url: &url::Url) -> Self {
        let jar = self
            .cookie_jar
            .get_or_insert_with(|| Arc::new(reqwest::cookie::Jar::default()));
        jar.add_cookie_str(cookie, url);
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
        let consumer_secret = self
            .consumer_secret
            .ok_or(CrabError::MissingConsumerSecret)?;
        let access_token = self.access_token.ok_or(CrabError::MissingAccessToken)?;

        let auth = Auth {
            consumer_key,
            consumer_secret,
            access_token,
        };

        let mut headers = HeaderMap::new();
        let user_agent = self
            .user_agent
            .unwrap_or_else(|| DEFAULT_USER_AGENT.to_string());
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&user_agent).map_err(|_| CrabError::InvalidUserAgent)?,
        );

        #[allow(unused_mut)]
        let mut client_builder = reqwest::Client::builder().default_headers(headers);

        #[cfg(feature = "cookies")]
        if let Some(jar) = self.cookie_jar {
            client_builder = client_builder.cookie_provider(jar);
        }

        let client = client_builder.build().map_err(CrabError::HttpClient)?;

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

/// Custom deserializer for content blocks that handles both NPF arrays and legacy HTML strings.
/// Legacy posts have `content` as an HTML string, while NPF posts have it as an array of ContentBlock.
/// This deserializer returns the array for NPF posts and an empty Vec for legacy posts.
///
/// Uses explicit Value-based dispatch instead of an untagged enum so that
/// deserialization errors include the block index and type for easier debugging.
pub(crate) fn deserialize_content_blocks<'de, D>(
    deserializer: D,
) -> Result<Vec<npf::ContentBlock>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::String(_) => Ok(Vec::new()), // Legacy HTML content
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let block_type = v
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_owned();
                serde_json::from_value(v).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "content block index {i} (type: {block_type}): {e}"
                    ))
                })
            })
            .collect(),
        other => Err(serde::de::Error::custom(format!(
            "expected array or string for content, got {}",
            kind_of(&other),
        ))),
    }
}

/// Returns a human-readable name for a JSON value kind.
pub(crate) fn kind_of(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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
    fn test_builder_missing_consumer_key() {
        let result = Crabrave::builder()
            .consumer_secret("test_secret")
            .access_token("test_token")
            .build();

        assert!(matches!(result, Err(CrabError::MissingConsumerKey)));
    }

    #[test]
    fn test_builder_custom_user_agent() {
        let result = Crabrave::builder()
            .consumer_key("test_key")
            .consumer_secret("test_secret")
            .access_token("test_access_token")
            .user_agent("CustomAgent/1.0")
            .build();

        assert!(result.is_ok());
    }
}
