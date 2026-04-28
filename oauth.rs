//! OAuth2 authentication helpers
//!
//! This module provides utilities to complete the OAuth2 authorization flow
//! with Tumblr's API.

use crate::{CrabError, CrabResult, OAUTH_AUTHORIZE_URL, OAUTH_TOKEN_URL};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use std::collections::HashMap;

/// An OAuth scope that this client will use.
/// See [Tumblr's documentation around OAuth authorization](https://www.tumblr.com/docs/en/api/v2#oauth2-authorization) for more information.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum OAuthScope {
    /// "basic" scope
    Basic,
    /// "write" scope
    Write,
    /// "offline_access" scope
    Offline,
}

/// OAuth2 configuration for Tumblr
pub struct OAuth2Config {
    client_id: String,
    client_secret: String,
    redirect_uri: oauth2::RedirectUrl,
    scopes: Vec<OAuthScope>,
}

impl OAuth2Config {
    /// Creates a new OAuth2 configuration
    ///
    /// # Arguments
    ///
    /// * `client_id` - Your application's consumer key
    /// * `client_secret` - Your application's consumer secret
    /// * `redirect_uri` - The redirect URI registered with your app
    /// * `scopes` - OAuth scopes to use. See `OAuthScope` for possible values
    ///
    /// # Errors
    ///
    /// Returns `CrabError::Auth` if the redirect URI is not a valid URL.
    ///
    /// # Example
    ///
    /// ```
    /// use crabrave::oauth::{OAuth2Config, OAuthScope};
    ///
    /// let config = OAuth2Config::new(
    ///     "your_consumer_key",
    ///     "your_consumer_secret",
    ///     "http://localhost:8080/callback",
    ///     vec![OAuthScope::Basic],
    /// )?;
    /// # Ok::<(), crabrave::CrabError>(())
    /// ```
    pub fn new<S: IntoIterator<Item = OAuthScope>>(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
        scopes: S,
    ) -> CrabResult<Self> {
        let redirect_uri = redirect_uri.into();
        // Validate redirect URI eagerly so callers get a clear error at construction time
        let redirect_uri = RedirectUrl::new(redirect_uri.clone())
            .map_err(|e| CrabError::Auth(format!("Invalid redirect URI: {e}")))?;
        Ok(Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri,
            scopes: scopes.into_iter().collect(),
        })
    }

    /// Returns this OAuth config's redirect URI.
    pub fn redirect_uri(&self) -> &str {
        self.redirect_uri.url().as_str()
    }

    /// Generates the authorization URL and CSRF token
    ///
    /// Direct the user to this URL to authorize your application.
    ///
    /// # Returns
    ///
    /// A tuple of (authorization_url, csrf_token). Store the CSRF token
    /// and verify it matches when the user is redirected back.
    ///
    /// # Example
    ///
    /// ```
    /// use crabrave::oauth::{OAuth2Config, OAuthScope};
    ///
    /// let config = OAuth2Config::new("key", "secret", "http://localhost/callback", vec![OAuthScope::Basic])?;
    /// let (auth_url, csrf_token) = config.authorize_url();
    ///
    /// println!("Visit this URL to authorize: {}", auth_url);
    /// println!("CSRF token (verify this later): {}", csrf_token.secret());
    /// # Ok::<(), crabrave::CrabError>(())
    /// ```
    pub fn authorize_url(&self) -> (String, CsrfToken) {
        fn map_scope(scope: &OAuthScope) -> Scope {
            match scope {
                OAuthScope::Basic => Scope::new("basic".to_string()),
                OAuthScope::Write => Scope::new("write".to_string()),
                OAuthScope::Offline => Scope::new("offline_access".to_string()),
            }
        }

        // unwrapping because we know the constant url values are valid
        #[allow(clippy::expect_used)]
        let auth_url = AuthUrl::new(OAUTH_AUTHORIZE_URL.to_string()).expect("Authorize URL is invalid. Please report this as a bug to codeberg.org/ryf/crabrave/issues along with the code that produced this issue.");
        #[allow(clippy::expect_used)]
        let token_url = TokenUrl::new(OAUTH_TOKEN_URL.to_string()).expect("Authorize URL is invalid. Please report this as a bug to codeberg.org/ryf/crabrave/issues along with the code that produced this issue.");

        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(self.redirect_uri.clone());

        let scopes: Vec<Scope> = self.scopes.iter().map(map_scope).collect();

        let (auth_url, csrf_token) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes)
            .url();

        (auth_url.to_string(), csrf_token)
    }

    /// Exchanges an authorization code for an access token
    ///
    /// After the user authorizes your app, Tumblr redirects to your redirect_uri
    /// with a `code` parameter. Use this method to exchange that code for tokens.
    ///
    /// # Arguments
    ///
    /// * `code` - The authorization code from the callback
    ///
    /// # Returns
    ///
    /// An `OAuth2Token` containing the access token and optional refresh token.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crabrave::oauth::{OAuth2Config, OAuthScope};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = OAuth2Config::new("key", "secret", "http://localhost/callback", vec![OAuthScope::Basic])?;
    ///
    /// // After user authorizes and you receive the code in the callback:
    /// let code = "authorization_code_from_callback";
    /// let token = config.exchange_code(code).await?;
    ///
    /// println!("Access token: {}", token.access_token);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exchange_code(&self, code: impl Into<String>) -> CrabResult<OAuth2Token> {
        let auth_url = AuthUrl::new(OAUTH_AUTHORIZE_URL.to_string())?;
        let token_url = TokenUrl::new(OAUTH_TOKEN_URL.to_string())?;

        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(self.redirect_uri.clone());

        let http_client = reqwest::Client::new();
        let token_result = client
            .exchange_code(AuthorizationCode::new(code.into()))
            .request_async(&http_client)
            .await
            .map_err(|e| CrabError::Auth(format!("Token exchange failed: {}", e)))?;

        Ok(OAuth2Token {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_result.expires_in().map(|d| d.as_secs()),
        })
    }

    /// Exchanges a refresh token for a new access token
    ///
    /// If your access token expires, use this method with your refresh token
    /// to get a new access token without requiring user interaction.
    ///
    /// # Arguments
    ///
    /// * `refresh_token` - The refresh token from a previous token exchange
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crabrave::oauth::{OAuth2Config, OAuthScope};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = OAuth2Config::new("key", "secret", "http://localhost/callback", vec![OAuthScope::Basic])?;
    ///
    /// // Use the refresh token from a previous exchange:
    /// let refresh_token = "your_refresh_token";
    /// let new_token = config.refresh_access_token(refresh_token).await?;
    ///
    /// println!("New access token: {}", new_token.access_token);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refresh_access_token(
        &self,
        refresh_token: impl Into<String>,
    ) -> CrabResult<OAuth2Token> {
        // we know our hard-coded URLs are valid
        #[allow(clippy::expect_used)]
        let auth_url = AuthUrl::new(OAUTH_AUTHORIZE_URL.to_string()).expect("Authorize URL is invalid. Please report this as a bug to codeberg.org/ryf/crabrave/issues along with the code that produced this issue.");
        #[allow(clippy::expect_used)]
        let token_url = TokenUrl::new(OAUTH_TOKEN_URL.to_string()).expect("Authorize URL is invalid. Please report this as a bug to codeberg.org/ryf/crabrave/issues along with the code that produced this issue.");

        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(self.redirect_uri.clone());

        let http_client = reqwest::Client::new();
        let token_result = client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.into()))
            .request_async(&http_client)
            .await
            .map_err(|e| CrabError::Auth(format!("Token refresh failed: {}", e)))?;

        Ok(OAuth2Token {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_in: token_result.expires_in().map(|d| d.as_secs()),
        })
    }
}

/// OAuth2 token information
///
/// Contains the access token and optional refresh token received from Tumblr.
#[derive(Debug, Clone)]
pub struct OAuth2Token {
    /// The access token used to authenticate API requests
    pub access_token: String,
    /// Optional refresh token to get new access tokens
    pub refresh_token: Option<String>,
    /// Optional token expiration time in seconds
    pub expires_in: Option<u64>,
}

impl OAuth2Token {
    /// Checks if the token has an expiration time
    pub fn has_expiration(&self) -> bool {
        self.expires_in.is_some()
    }

    /// Checks if a refresh token is available
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }
}

/// Helper function to parse OAuth2 callback parameters
///
/// Use this to extract the authorization code and state from the callback URL.
///
/// # Arguments
///
/// * `callback_url` - The full callback URL with query parameters
///
/// # Returns
///
/// A HashMap of query parameters (code, state, etc.)
///
/// # Example
///
/// ```
/// use crabrave::oauth::parse_callback;
///
/// let callback = "http://localhost/callback?code=abc123&state=xyz789";
/// let params = parse_callback(callback).unwrap();
///
/// assert_eq!(params.get("code"), Some(&"abc123".to_string()));
/// assert_eq!(params.get("state"), Some(&"xyz789".to_string()));
/// ```
pub fn parse_callback(callback_url: &str) -> CrabResult<HashMap<String, String>> {
    let url = url::Url::parse(callback_url)
        .map_err(|e| CrabError::Auth(format!("Invalid callback URL: {}", e)))?;

    let params: HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    if params.is_empty() {
        return Err(CrabError::Auth(
            "No parameters found in callback URL".to_string(),
        ));
    }

    Ok(params)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_config_creation() {
        let config = OAuth2Config::new(
            "key",
            "secret",
            "http://localhost/callback",
            vec![OAuthScope::Basic],
        )
        .unwrap();
        assert_eq!(config.client_id, "key");
        assert_eq!(config.client_secret, "secret");
        assert_eq!(config.redirect_uri(), "http://localhost/callback");
    }

    #[test]
    fn test_oauth_config_invalid_redirect_uri() {
        let result = OAuth2Config::new("key", "secret", "not a valid url", vec![OAuthScope::Basic]);
        assert!(result.is_err());
    }

    #[test]
    fn test_authorize_url_generation() {
        let config = OAuth2Config::new(
            "key",
            "secret",
            "http://localhost/callback",
            vec![OAuthScope::Basic],
        )
        .unwrap();
        let (auth_url, csrf_token) = config.authorize_url();

        assert!(auth_url.contains("https://www.tumblr.com/oauth2/authorize"));
        assert!(auth_url.contains("client_id=key"));
        assert!(auth_url.contains("redirect_uri="));
        assert!(!csrf_token.secret().is_empty());
    }

    #[test]
    fn test_parse_callback() {
        let callback = "http://localhost/callback?code=abc123&state=xyz789";
        let params = parse_callback(callback).unwrap();

        assert_eq!(params.get("code"), Some(&"abc123".to_string()));
        assert_eq!(params.get("state"), Some(&"xyz789".to_string()));
    }

    #[test]
    fn test_parse_callback_invalid() {
        let result = parse_callback("not a url");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_callback_no_params() {
        let result = parse_callback("http://localhost/callback");
        assert!(result.is_err());
    }

    #[test]
    fn test_oauth_token_helpers() {
        let token = OAuth2Token {
            access_token: "token".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_in: Some(3600),
        };

        assert!(token.has_expiration());
        assert!(token.can_refresh());

        let token_no_refresh = OAuth2Token {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_in: None,
        };

        assert!(!token_no_refresh.has_expiration());
        assert!(!token_no_refresh.can_refresh());
    }
}
