//! Response handling and parsing for the Tumblr API

use crate::{ApiErrorDetail, CrabError, CrabResult};
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
    /// Raw error objects from the envelope, present on error responses
    ///
    /// Kept as raw JSON so a malformed entry can never fail envelope parsing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<serde_json::Value>,
}

/// Response metadata returned by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    /// HTTP status code
    pub status: u16,
    /// Status message
    pub msg: String,
}

/// A response type for endpoints that return no useful data
///
/// Some Tumblr API endpoints return an empty array `[]` in the response field
/// when there's no useful data to return (e.g., bulk operations, deletions),
/// while others (per Tumblr's docs, e.g. filtered tags/content) return an
/// empty object `{}` or `null` instead. This type accepts any of the three
/// shapes and normalizes them to an (possibly empty) `Vec`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EmptyResponse(Vec<serde_json::Value>);

impl<'de> Deserialize<'de> for EmptyResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Null | serde_json::Value::Object(_) => Ok(EmptyResponse(Vec::new())),
            serde_json::Value::Array(arr) => Ok(EmptyResponse(arr)),
            other => Err(serde::de::Error::custom(format!(
                "expected null, object, or array for empty response, got {}",
                crate::kind_of(&other),
            ))),
        }
    }
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
/// Thin wrapper around [`parse_response_bytes_with_status`] for callers (and
/// existing tests) that don't have an HTTP status to hand; it assumes 200,
/// which preserves the original behavior of surfacing non-JSON bodies as
/// `Serialization` errors.
pub fn parse_response_bytes<T: DeserializeOwned>(bytes: &[u8]) -> CrabResult<T> {
    parse_response_bytes_with_status(bytes, 200)
}

/// Parses a Tumblr API response from bytes, given the HTTP status of the request.
///
/// Parses the JSON once into the envelope, checks the envelope status, then
/// converts the inner response value into the target type.
///
/// When deserialization fails on a response containing a `posts` array,
/// each post is tried individually to identify which one caused the error.
///
/// If the body isn't a JSON envelope at all (e.g. a CDN error page), the HTTP
/// status is used to decide how to report it: a non-2xx status yields an
/// `Api` error carrying the status and a snippet of the raw body, while a 2xx
/// status preserves the original `Serialization` error.
pub(crate) fn parse_response_bytes_with_status<T: DeserializeOwned>(
    bytes: &[u8],
    http_status: u16,
) -> CrabResult<T> {
    // Some endpoints (e.g. DELETE .../members/{blog}) return 204 with a
    // completely empty body rather than a JSON envelope. There's no `meta`
    // to check here, so success is inferred from the HTTP status alone;
    // `EmptyResponse` accepts `null`, while a typed response that actually
    // needs data still fails with the same `Serialization` error it would
    // have gotten from an empty envelope.
    if is_success(http_status) && bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return serde_json::from_value(serde_json::Value::Null).map_err(CrabError::Serialization);
    }

    let envelope: ApiResponse<serde_json::Value> = match serde_json::from_slice(bytes) {
        Ok(envelope) => envelope,
        Err(parse_err) => {
            return if is_success(http_status) {
                Err(CrabError::Serialization(parse_err))
            } else {
                let snippet: String = String::from_utf8_lossy(bytes).chars().take(200).collect();
                Err(CrabError::Api {
                    status: http_status,
                    message: format!("non-JSON response body: {snippet}"),
                    errors: Vec::new(),
                })
            };
        }
    };

    if !is_success(envelope.meta.status) {
        let errors = extract_error_details(&envelope.errors);
        let mut message = envelope.meta.msg.clone();
        let summaries: Vec<String> = errors
            .iter()
            .map(format_error_detail)
            .filter(|s| !s.is_empty())
            .collect();
        if !summaries.is_empty() {
            message.push_str("; ");
            message.push_str(&summaries.join("; "));
        }
        return Err(CrabError::Api {
            status: envelope.meta.status,
            message,
            errors,
        });
    }

    serde_json::from_value(envelope.response.clone()).map_err(|original_err| {
        // If the response has a "posts" array, try each post individually
        // to produce a more useful error pointing at the specific failing post.
        if let Some(posts) = envelope.response.get("posts").and_then(|p| p.as_array()) {
            for (i, post) in posts.iter().enumerate() {
                let post_id = post.get("id_string").and_then(|v| v.as_str());
                if let Err(e) = serde_json::from_value::<crate::handlers::blog::Post>(post.clone())
                {
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

/// Best-effort extraction of structured error details from an envelope's raw `errors` array.
///
/// Each entry is read leniently via `.get()`/`.as_str()`/`.as_i64()` rather than
/// deserialized strictly, so a malformed or unexpected entry degrades to a
/// mostly-empty `ApiErrorDetail` instead of failing the whole response.
fn extract_error_details(errors: &[serde_json::Value]) -> Vec<ApiErrorDetail> {
    errors
        .iter()
        .map(|e| ApiErrorDetail {
            title: e.get("title").and_then(|v| v.as_str()).map(str::to_owned),
            code: e.get("code").and_then(serde_json::Value::as_i64),
            detail: e.get("detail").and_then(|v| v.as_str()).map(str::to_owned),
            subcode: e.get("subcode").and_then(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            }),
        })
        .collect()
}

/// Renders one error detail as a compact summary, omitting parts that are absent.
fn format_error_detail(detail: &ApiErrorDetail) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(title) = &detail.title {
        parts.push(title.clone());
    }

    let mut code_part = String::new();
    if let Some(code) = detail.code {
        code_part.push_str(&format!("code {code}"));
    }
    if let Some(subcode) = &detail.subcode {
        if code_part.is_empty() {
            code_part.push_str(&format!("subcode {subcode}"));
        } else {
            code_part.push_str(&format!(", subcode {subcode}"));
        }
    }
    if !code_part.is_empty() {
        parts.push(format!("({code_part})"));
    }

    if let Some(text) = &detail.detail {
        parts.push(text.clone());
    }

    parts.join(": ")
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
            Err(CrabError::Api {
                status,
                message,
                errors,
            }) => {
                assert_eq!(status, 404);
                assert_eq!(message, "Not Found");
                assert!(errors.is_empty());
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
            errors: Vec::new(),
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
            errors: Vec::new(),
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

    #[test]
    fn test_parse_error_response_with_error_details() {
        let json = json!({
            "meta": { "status": 400, "msg": "Bad Request" },
            "response": {},
            "errors": [
                {
                    "title": "Bad Request",
                    "code": 8001,
                    "detail": "NPF is invalid",
                    "subcode": "8001.1"
                }
            ]
        });

        let result: CrabResult<TestData> = parse_response(&json.to_string());
        let err = result.unwrap_err();

        match &err {
            CrabError::Api { status, errors, .. } => {
                assert_eq!(*status, 400);
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0].title.as_deref(), Some("Bad Request"));
                assert_eq!(errors[0].code, Some(8001));
                assert_eq!(errors[0].detail.as_deref(), Some("NPF is invalid"));
                assert_eq!(errors[0].subcode.as_deref(), Some("8001.1"));
            }
            other => panic!("Expected Api error, got: {other:?}"),
        }

        let msg = err.to_string();
        assert!(msg.contains("NPF is invalid"), "message: {msg}");
        assert!(msg.contains("8001.1"), "message: {msg}");
    }

    #[test]
    fn test_parse_error_response_with_code_only_detail() {
        let json = json!({
            "meta": { "status": 400, "msg": "Bad Request" },
            "response": {},
            "errors": [
                { "code": 8001, "subcode": "8001.1" }
            ]
        });

        let result: CrabResult<TestData> = parse_response(&json.to_string());
        let err = result.unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("code 8001"), "message: {msg}");
        assert!(msg.contains("8001.1"), "message: {msg}");
    }

    #[test]
    fn test_parse_non_json_body_with_error_status_yields_api_error() {
        let result: CrabResult<TestData> =
            parse_response_bytes_with_status(b"<html>bad gateway</html>", 502);

        match result {
            Err(CrabError::Api {
                status, message, ..
            }) => {
                assert_eq!(status, 502);
                assert!(message.contains("bad gateway"), "message: {message}");
            }
            other => panic!("Expected Api error, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_non_json_body_with_success_status_yields_serialization_error() {
        let result: CrabResult<TestData> =
            parse_response_bytes_with_status(b"<html>bad gateway</html>", 200);

        match result {
            Err(CrabError::Serialization(_)) => {}
            other => panic!("Expected Serialization error, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_empty_body_with_success_status_yields_empty_response() {
        let result: CrabResult<EmptyResponse> = parse_response_bytes_with_status(b"", 204);
        assert_eq!(result.unwrap(), EmptyResponse(Vec::new()));
    }

    #[test]
    fn test_parse_empty_body_with_success_status_and_typed_response_yields_serialization_error() {
        let result: CrabResult<TestData> = parse_response_bytes_with_status(b"", 200);

        match result {
            Err(CrabError::Serialization(_)) => {}
            other => panic!("Expected Serialization error, got: {other:?}"),
        }
    }

    #[test]
    fn test_empty_response_accepts_array_object_and_null() {
        let from_array: EmptyResponse = serde_json::from_str("[]").unwrap();
        assert_eq!(from_array, EmptyResponse(Vec::new()));

        let from_object: EmptyResponse = serde_json::from_str("{}").unwrap();
        assert_eq!(from_object, EmptyResponse(Vec::new()));

        let from_null: EmptyResponse = serde_json::from_str("null").unwrap();
        assert_eq!(from_null, EmptyResponse(Vec::new()));
    }
}
