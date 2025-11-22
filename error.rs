//! Error types for the Crabrave library

use thiserror::Error;

/// A specialized `Result` type for Crabrave operations
pub type CrabResult<T> = Result<T, CrabError>;

/// Errors that can occur when using the Crabrave client
#[derive(Debug, Error)]
pub enum CrabError {
    /// Missing consumer key (API key) in builder
    #[error("Missing consumer key (API key)")]
    MissingConsumerKey,

    /// Missing consumer secret in builder
    #[error("Missing consumer secret")]
    MissingConsumerSecret,

    /// Missing access token in builder
    #[error("Missing access token")]
    MissingAccessToken,

    /// Invalid User-Agent header value
    #[error("Invalid User-Agent header value")]
    InvalidUserAgent,

    /// HTTP client construction error
    #[error("Failed to construct HTTP client: {0}")]
    HttpClient(#[source] reqwest::Error),

    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error (status {status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from the API
        message: String,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded. Retry after: {retry_after:?} seconds")]
    RateLimit {
        /// Number of seconds to wait before retrying (if provided by API)
        retry_after: Option<u64>,
    },

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// OAuth error
    #[error("OAuth error: {0}")]
    OAuth(String),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// URL parsing error
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// Invalid response from API
    #[error("Invalid API response: {0}")]
    InvalidResponse(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CrabError::MissingConsumerKey;
        assert_eq!(err.to_string(), "Missing consumer key (API key)");

        let err = CrabError::RateLimit {
            retry_after: Some(60),
        };
        assert_eq!(
            err.to_string(),
            "Rate limit exceeded. Retry after: Some(60) seconds"
        );

        let err = CrabError::Api {
            status: 404,
            message: "Blog not found".to_string(),
        };
        assert_eq!(err.to_string(), "API error (status 404): Blog not found");
    }
}
