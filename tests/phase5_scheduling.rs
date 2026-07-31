//! Mock server tests for post scheduling (`publish_on`) and backdating (`date`)
//!
//! Covers `CreatePostBuilder` and `EditPostBuilder`. Tumblr's docs specify both
//! parameters as ISO 8601 strings, with `publish_on` only taking effect when
//! `state` is `"queue"`; editing a scheduled post must resend `publish_on` or
//! the schedule is dropped.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crabrave::Crabrave;
use crabrave::npf::ContentBlock;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

const TEST_BLOG_NAME: &str = "crabrave";

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

/// Matches requests whose JSON body does not contain the given top-level key
///
/// `body_partial_json` only asserts a key's presence and value, so absence
/// needs its own matcher.
struct BodyLacksKey(&'static str);

impl Match for BodyLacksKey {
    fn matches(&self, request: &Request) -> bool {
        match request.body_json::<serde_json::Value>() {
            Ok(serde_json::Value::Object(map)) => !map.contains_key(self.0),
            _ => false,
        }
    }
}

#[tokio::test]
async fn test_create_post_sends_publish_on_when_queued() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .and(body_partial_json(serde_json::json!({
            "state": "queue",
            "publish_on": "2026-08-15T10:00:00Z"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "555444333"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .add_block(ContentBlock::text("Scheduled for later"))
        .state("queue")
        .publish_on("2026-08-15T10:00:00Z")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().id, "555444333");
}

#[tokio::test]
async fn test_create_post_sends_date_for_backdating() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .and(body_partial_json(serde_json::json!({
            "date": "2020-01-01T00:00:00Z"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "111222333"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .add_block(ContentBlock::text("Backdated post"))
        .date("2020-01-01T00:00:00Z")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().id, "111222333");
}

#[tokio::test]
async fn test_create_post_omits_publish_on_and_date_when_unset() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .and(BodyLacksKey("publish_on"))
        .and(BodyLacksKey("date"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "777888999"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .add_block(ContentBlock::text("Right now, please"))
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().id, "777888999");
}

#[tokio::test]
async fn test_edit_post_resends_publish_on_to_keep_schedule() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(format!("/blog/{}/posts/123456", TEST_BLOG_NAME)))
        .and(body_partial_json(serde_json::json!({
            "publish_on": "2026-09-01T09:30:00Z"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "id": "123456"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("123456")
        .edit()
        .content(vec![ContentBlock::text("Still queued, just edited")])
        .publish_on("2026-09-01T09:30:00Z")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().id, "123456");
}

#[tokio::test]
async fn test_edit_post_sends_date_for_backdating() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(format!("/blog/{}/posts/654321", TEST_BLOG_NAME)))
        .and(body_partial_json(serde_json::json!({
            "date": "2019-05-05T00:00:00Z"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "id": "654321"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("654321")
        .edit()
        .content(vec![ContentBlock::text("Re-backdated")])
        .date("2019-05-05T00:00:00Z")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().id, "654321");
}

#[tokio::test]
async fn test_edit_post_omits_publish_on_and_date_when_unset() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(format!("/blog/{}/posts/999000", TEST_BLOG_NAME)))
        .and(BodyLacksKey("publish_on"))
        .and(BodyLacksKey("date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "id": "999000"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("999000")
        .edit()
        .content(vec![ContentBlock::text("No schedule here")])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().id, "999000");
}
