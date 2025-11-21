//! Response handling and parsing for the Tumblr API

use crate::{CrabError, CrabResult};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Standard Tumblr API response envelope
///
/// All Tumblr API responses follow this structure with metadata
/// in the `meta` field and the actual response data in the `response` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response metadata including status code and message
    pub meta: Meta,
    /// The actual response data
    pub response: T,
}

/// Response metadata returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    /// HTTP status code
    pub status: u16,
    /// Status message
    pub msg: String,
}

impl<T> ApiResponse<T> {
    /// Unwraps the response data, consuming the envelope
    pub fn into_response(self) -> T {
        self.response
    }

    /// Gets a reference to the response data
    pub fn response_ref(&self) -> &T {
        &self.response
    }

    /// Checks if the response was successful
    pub fn is_success(&self) -> bool {
        is_success(self.meta.status)
    }
}

/// Parses a Tumblr API response from JSON
///
/// This function handles the standard response envelope and extracts
/// the inner response data of type `T`.
#[allow(dead_code)] // Used in tests
pub fn parse_response<T: DeserializeOwned>(json: &str) -> CrabResult<T> {
    // First, parse just the meta to check status
    let value: serde_json::Value = serde_json::from_str(json)?;

    if let Some(meta) = value.get("meta")
        && let Some(status) = meta.get("status").and_then(|s| s.as_u64())
    {
        let status = status as u16;
        if !is_success(status) {
            let message = meta
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Err(CrabError::Api { status, message });
        }
    }

    // Status is OK, now deserialize the full response
    let envelope: ApiResponse<T> = serde_json::from_str(json)?;
    Ok(envelope.into_response())
}

/// Parses a Tumblr API response from bytes
pub fn parse_response_bytes<T: DeserializeOwned>(bytes: &[u8]) -> CrabResult<T> {
    // First, parse just the meta to check status
    let value: serde_json::Value = serde_json::from_slice(bytes)?;

    if let Some(meta) = value.get("meta")
        && let Some(status) = meta.get("status").and_then(|s| s.as_u64())
    {
        let status = status as u16;
        if !is_success(status) {
            let message = meta
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Err(CrabError::Api { status, message });
        }
    }

    // Status is OK, now deserialize the full response
    let envelope: ApiResponse<T> = serde_json::from_slice(bytes)?;
    Ok(envelope.into_response())
}

fn is_success(code: u16) -> bool {
    (200..300).contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_parse_success_response() {
        let json = json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "name": "test",
                "value": 42
            }
        });

        let result: CrabResult<TestData> = parse_response(&json.to_string());
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_parse_error_response() {
        let json = json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": {}
        });

        let result: CrabResult<TestData> = parse_response(&json.to_string());
        assert!(result.is_err());

        match result {
            Err(CrabError::Api { status, message }) => {
                assert_eq!(status, 404);
                assert_eq!(message, "Not Found");
            }
            _ => panic!("Expected Api error"),
        }
    }

    #[test]
    fn test_api_response_is_success() {
        let response = ApiResponse {
            meta: Meta {
                status: 200,
                msg: "OK".to_string(),
            },
            response: TestData {
                name: "test".to_string(),
                value: 42,
            },
        };

        assert!(response.is_success());
    }

    #[test]
    fn test_api_response_is_not_success() {
        let response: ApiResponse<()> = ApiResponse {
            meta: Meta {
                status: 500,
                msg: "Internal Server Error".to_string(),
            },
            response: (),
        };

        assert!(!response.is_success());
    }
}
