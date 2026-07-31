//! Mock server tests for the full `/user/limits` response shape
//!
//! Tumblr's docs show this endpoint's key set as explicitly non-exhaustive
//! (the example response ends in "..."), so this test locks in the full set
//! of nine limit categories observed in a live capture.

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
async fn test_user_limits_full_response() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_limits.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/limits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().limits().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let limits = result.unwrap().user;

    assert!(limits.blogs.is_some());
    assert!(limits.posts.is_some());
    assert!(limits.community_posts.is_some());
    assert!(limits.photos.is_some());
    assert!(limits.videos.is_some());
    assert!(limits.video_seconds.is_some());
    assert!(limits.follows.is_some());
    assert!(limits.likes.is_some());
    assert!(limits.reply_likes.is_some());

    let follows = limits.follows.unwrap();
    assert_eq!(follows.limit, 200);
    assert_eq!(follows.remaining, 198);

    let community_posts = limits.community_posts.unwrap();
    assert_eq!(community_posts.limit, 500);
    assert_eq!(community_posts.remaining, 500);
}
