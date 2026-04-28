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

/// A response type for endpoints that return an empty array
///
/// Some Tumblr API endpoints return an empty array `[]` in the response field
/// when there's no useful data to return (e.g., bulk operations, deletions).
/// This type allows those responses to be deserialized properly while
/// conveying that no useful data is returned.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmptyResponse(Vec<serde_json::Value>);

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

/// Parses a Tumblr API response from a JSON string
///
/// This function handles the standard response envelope and extracts
/// the inner response data of type `T`.
#[allow(dead_code)] // Used in tests
pub fn parse_response<T: DeserializeOwned>(json: &str) -> CrabResult<T> {
    parse_response_bytes(json.as_bytes())
}

/// Parses a Tumblr API response from bytes
///
/// Parses the JSON once into the envelope, checks the status code, then
/// converts the inner response value into the target type.
///
/// When deserialization fails on a response containing a `posts` array,
/// each post is tried individually to identify which one caused the error.
pub fn parse_response_bytes<T: DeserializeOwned>(bytes: &[u8]) -> CrabResult<T> {
    let envelope: ApiResponse<serde_json::Value> = serde_json::from_slice(bytes)?;

    if !is_success(envelope.meta.status) {
        return Err(CrabError::Api {
            status: envelope.meta.status,
            message: envelope.meta.msg,
        });
    }

    serde_json::from_value(envelope.response.clone()).map_err(|original_err| {
        // If the response has a "posts" array, try each post individually
        // to produce a more useful error pointing at the specific failing post.
        if let Some(posts) = envelope.response.get("posts").and_then(|p| p.as_array()) {
            for (i, post) in posts.iter().enumerate() {
                let post_id = post.get("id_string").and_then(|v| v.as_str());
                if let Err(e) = serde_json::from_value::<crate::handlers::blog::Post>(
                    post.clone(),
                ) {
                    let id_info = post_id.map_or(String::new(), |id| format!(" (id: {id})"));
                    return CrabError::InvalidResponse(format!(
                        "failed to parse post at index {i}{id_info}: {e}"
                    ));
                }
            }
        }
        CrabError::Serialization(original_err)
    })
}

fn is_success(code: u16) -> bool {
    (200..300).contains(&code)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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

    /// Helper: build a minimal valid post JSON object.
    fn minimal_post(id: &str) -> serde_json::Value {
        json!({
            "type": "blocks",
            "id_string": id,
            "blog_name": "testblog",
            "post_url": "https://testblog.tumblr.com/post/1",
            "timestamp": 1700000000,
            "reblog_key": "abc",
            "content": [],
            "layout": [],
            "trail": []
        })
    }

    #[test]
    fn test_parse_response_post_error_includes_index_and_id() {
        use crate::handlers::blog::PostsResponse;

        // Second post has an invalid content block that will fail parsing
        let mut bad_post = minimal_post("999");
        bad_post["content"] = json!([
            { "type": "text" }
        ]);

        let envelope = json!({
            "meta": { "status": 200, "msg": "OK" },
            "response": {
                "posts": [
                    minimal_post("100"),
                    bad_post,
                ],
                "total_posts": 2
            }
        });

        let result: CrabResult<PostsResponse> = parse_response(&envelope.to_string());
        assert!(result.is_err());

        let err = result.unwrap_err();
        let msg = err.to_string();
        // Should point at post index 1 with its id
        assert!(
            msg.contains("index 1") && msg.contains("999"),
            "expected error to contain post index and id, got: {msg}",
        );
    }

    #[test]
    fn test_parse_response_valid_posts_succeed() {
        use crate::handlers::blog::PostsResponse;

        let envelope = json!({
            "meta": { "status": 200, "msg": "OK" },
            "response": {
                "posts": [
                    minimal_post("100"),
                    minimal_post("200"),
                ],
                "total_posts": 2
            }
        });

        let result: CrabResult<PostsResponse> = parse_response(&envelope.to_string());
        assert!(result.is_ok(), "unexpected error: {:?}", result);
        assert_eq!(result.unwrap().posts.len(), 2);
    }
}
