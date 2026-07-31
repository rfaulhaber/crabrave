//! Mock server tests for Phase 5 `blog.rs` additions: `sort`/`filter`/`reblog_info`/
//! `notes_info` on the posts endpoint, indexed multi-value query encoding for tags
//! and notification filters, timed post muting, `post_format`, and the new
//! `Notification`/`Post` fields.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crabrave::Crabrave;
use crabrave::handlers::blog::{NotificationType, Post, PostFilter, PostFormat, SortOrder};
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

fn empty_posts_response() -> serde_json::Value {
    serde_json::json!({
        "meta": { "status": 200, "msg": "OK" },
        "response": { "posts": [] }
    })
}

fn empty_notifications_response() -> serde_json::Value {
    serde_json::json!({
        "meta": { "status": 200, "msg": "OK" },
        "response": { "notifications": [] }
    })
}

fn empty_array_response() -> serde_json::Value {
    serde_json::json!({
        "meta": { "status": 200, "msg": "OK" },
        "response": []
    })
}

#[tokio::test]
async fn test_posts_with_multiple_tags_uses_indexed_keys() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts")))
        .and(query_param("tag[0]", "a"))
        .and(query_param("tag[1]", "b"))
        .and(query_param_is_missing("tag"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_posts_response()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .posts()
        .tags(["a", "b"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_posts_sort_filter_reblog_notes_info_on_wire() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts")))
        .and(query_param("sort", "asc"))
        .and(query_param("filter", "text"))
        .and(query_param("reblog_info", "true"))
        .and(query_param("notes_info", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_posts_response()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .posts()
        .sort(SortOrder::Asc)
        .filter(PostFilter::Text)
        .reblog_info(true)
        .notes_info(true)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_notifications_omit_post_ids_and_types_use_indexed_keys() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/notifications")))
        .and(query_param("omit_post_ids[0]", "111111"))
        .and(query_param("omit_post_ids[1]", "222222"))
        .and(query_param("types[0]", "like"))
        .and(query_param_is_missing("omit_post_ids"))
        .and(query_param_is_missing("types"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_notifications_response()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .omit_post_ids(vec!["111111", "222222"])
        .types(vec![NotificationType::Like])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_mute_for_sends_duration_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/123456789/mute")))
        .and(body_json(serde_json::json!({ "mute_length_seconds": 60 })))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_array_response()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("123456789")
        .mute_for(60)
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_unmute_sends_delete_to_mute_path() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/123456789/mute")))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_array_response()))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("123456789")
        .unmute()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_post_get_with_legacy_format_sends_query_param() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/123456789")))
        .and(query_param("post_format", "legacy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "status": 200, "msg": "OK" },
            "response": {
                "id_string": "123456789",
                "blog_name": TEST_BLOG_NAME,
                "post_url": "https://crabrave.tumblr.com/post/123456789",
                "type": "blocks",
                "timestamp": 1782713987
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("123456789")
        .post_format(PostFormat::Legacy)
        .get()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_notification_deserializes_reblog_and_reply_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/notifications")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": { "status": 200, "msg": "OK" },
            "response": {
                "notifications": [
                    {
                        "id": "notif-reblog-001",
                        "type": "reblog_with_content",
                        "timestamp": 1234567890,
                        "unread": true,
                        "target_post_id": "555555",
                        "from_tumblelog_name": "reblogger",
                        "post_id": "555555",
                        "post_tags": ["rust", "crabrave"],
                        "added_text": "this is great!"
                    },
                    {
                        "id": "notif-reply-001",
                        "type": "reply",
                        "timestamp": 1234567880,
                        "unread": true,
                        "target_post_id": "666666",
                        "from_tumblelog_name": "replier",
                        "reply_text": "totally agree"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let notifs = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .send()
        .await
        .expect("request should succeed");

    assert_eq!(notifs.notifications.len(), 2);

    let reblog = &notifs.notifications[0];
    assert_eq!(reblog.post_id.as_deref(), Some("555555"));
    assert_eq!(reblog.post_tags, vec!["rust", "crabrave"]);
    assert_eq!(reblog.added_text.as_deref(), Some("this is great!"));
    assert!(reblog.reply_text.is_none());

    let reply = &notifs.notifications[1];
    assert_eq!(reply.reply_text.as_deref(), Some("totally agree"));
    assert!(reply.post_id.is_none());
    assert!(reply.post_tags.is_empty());
}

#[test]
fn test_post_deserializes_scheduling_fields() {
    let json = r#"{
        "id_string": "820735101718626304",
        "blog_name": "crabrave",
        "post_url": "https://crabrave.tumblr.com/post/820735101718626304",
        "type": "blocks",
        "timestamp": 1782713987,
        "queued_state": "scheduled",
        "scheduled_publish_time": 1782800000,
        "publish_on": "2026-08-01T12:00:00Z"
    }"#;

    let post: Post = serde_json::from_str(json).expect("valid post JSON");
    assert_eq!(post.queued_state.as_deref(), Some("scheduled"));
    assert_eq!(post.scheduled_publish_time, 1782800000);
    assert_eq!(post.publish_on.as_deref(), Some("2026-08-01T12:00:00Z"));
}
