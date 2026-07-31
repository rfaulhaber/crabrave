//! Tests for the generic `_links` pagination cursor model wired onto list responses.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crabrave::Crabrave;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create a test client pointed at a mock server
async fn test_client(mock_server: &MockServer) -> Crabrave {
    Crabrave::builder()
        .consumer_key("test_key")
        .consumer_secret("test_secret")
        .access_token("test_token")
        .base_url(mock_server.uri())
        .build()
        .expect("Failed to build test client")
}

#[tokio::test]
async fn test_blog_posts_links_next() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_posts_npf.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs("crabrave-demo")
        .posts()
        .send()
        .await
        .expect("posts request failed");

    let links = result.links.expect("expected _links on response");
    let next = links.next.expect("expected _links.next");
    assert!(!next.href.as_deref().expect("expected href").is_empty());
    assert_eq!(next.method.as_deref(), Some("GET"));

    let pairs = next.query_pairs();
    assert!(
        pairs
            .iter()
            .any(|(k, v)| k == "page_number" && !v.is_empty()),
        "expected page_number in query_pairs, got: {:?}",
        pairs
    );
    assert!(pairs.iter().any(|(k, v)| k == "npf" && v == "true"));
    assert!(
        pairs
            .iter()
            .any(|(k, v)| k == "tumblelog" && v == "crabrave-demo")
    );
}

#[tokio::test]
async fn test_user_following_links_next() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_following.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/following"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .following()
        .send()
        .await
        .expect("following request failed");

    // Fixture's _links.next.href is "/v2/user/following?offset=21".
    let links = result.links.expect("expected _links on response");
    let next = links.next.expect("expected _links.next");
    assert_eq!(next.href.as_deref(), Some("/v2/user/following?offset=21"));
    assert_eq!(
        next.query_pairs(),
        vec![("offset".to_string(), "21".to_string())]
    );
}

#[tokio::test]
async fn test_user_likes_links_next() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_likes.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/likes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .likes()
        .send()
        .await
        .expect("likes request failed");

    // Fixture's _links.next.href is "/v2/user/likes?before=1785435832".
    let links = result.links.expect("expected _links on response");
    let next = links.next.expect("expected _links.next");
    assert_eq!(
        next.href.as_deref(),
        Some("/v2/user/likes?before=1785435832")
    );
    assert_eq!(
        next.query_pairs(),
        vec![("before".to_string(), "1785435832".to_string())]
    );
}

#[tokio::test]
async fn test_missing_links_key_deserializes_to_none() {
    let mock_server = MockServer::start().await;
    let body = serde_json::json!({
        "meta": {"status": 200, "msg": "OK"},
        "response": {
            "blogs": [],
            "total_blogs": 0
        }
    });

    Mock::given(method("GET"))
        .and(path("/user/following"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .following()
        .send()
        .await
        .expect("following request failed");

    assert!(result.links.is_none());
}
