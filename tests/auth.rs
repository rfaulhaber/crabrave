//! Tests for api-key-only auth mode and empty-body 2xx response handling.
//!
//! These use a mock HTTP server (see tests/mock_server.rs for the pattern
//! this follows) rather than real Tumblr credentials.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crabrave::{CrabError, Crabrave};
use wiremock::matchers::{bearer_token, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Client authenticated with only a consumer key (no OAuth).
async fn api_key_only_client(mock_server: &MockServer) -> Crabrave {
    Crabrave::builder()
        .consumer_key("test_key")
        .base_url(mock_server.uri())
        .build()
        .expect("Failed to build test client")
}

/// Client authenticated with the full OAuth2 credential set.
async fn oauth_client(mock_server: &MockServer) -> Crabrave {
    Crabrave::builder()
        .consumer_key("test_key")
        .consumer_secret("test_secret")
        .access_token("test_token")
        .base_url(mock_server.uri())
        .build()
        .expect("Failed to build test client")
}

#[tokio::test]
async fn test_api_key_only_sends_api_key_param_and_no_authorization_header() {
    let mock_server = MockServer::start().await;

    let fixture = include_str!("fixtures/live/blog_info.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .and(query_param("api_key", "test_key"))
        .and(|req: &wiremock::Request| !req.headers.contains_key("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = api_key_only_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_access_token_sends_bearer_header_and_no_api_key_param() {
    let mock_server = MockServer::start().await;

    let fixture = include_str!("fixtures/live/blog_info.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .and(bearer_token("test_token"))
        .and(query_param_is_missing("api_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = oauth_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_empty_body_204_succeeds_for_empty_response_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/members/some-blog"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let client = oauth_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .leave("some-blog")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_empty_body_200_with_typed_response_yields_serialization_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = oauth_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    match result {
        Err(CrabError::Serialization(_)) => {}
        other => panic!("Expected Serialization error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_api_key_only_client_deserializes_real_blog_info_fixture() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_info.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/info"))
        .and(query_param("api_key", "test_key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = api_key_only_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").info().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let info = result.unwrap();
    assert_eq!(info.blog.name, "crabrave-demo");
    assert_eq!(info.blog.title, "crabrave-demo");
    assert_eq!(info.blog.posts, 22);
}
