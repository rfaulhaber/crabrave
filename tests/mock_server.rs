//! Mock server tests using wiremock
//!
//! These tests use a mock HTTP server to test the full request/response cycle
//! without requiring actual Tumblr API credentials.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::redundant_static_lifetimes
)]
// The community info fixture is a large, deeply-nested `json!` invocation
// that mirrors a real API response; the default limit isn't enough for it.
#![recursion_limit = "256"]

use crabrave::handlers::blog::AvatarResponse;
use crabrave::{CrabError, Crabrave};
use wiremock::matchers::{body_json, method, path, query_param};
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

#[tokio::test]
async fn test_blog_info_success() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_info.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").info().await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.blog.name, "crabrave-demo");
    assert_eq!(info.blog.title, "crabrave-demo");
    assert_eq!(info.blog.posts, 22);
}

#[tokio::test]
async fn test_blog_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/nonexistent/info"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("nonexistent").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::Api { status, .. } => assert_eq!(status, 404),
        _ => panic!("Expected API error"),
    }
}

#[tokio::test]
async fn test_rate_limit_error_per_hour() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-perhour-remaining", "0")
                .insert_header("x-ratelimit-perhour-reset", "60")
                .insert_header("x-ratelimit-perday-remaining", "500")
                .insert_header("x-ratelimit-perday-reset", "3600")
                .set_body_json(serde_json::json!({
                    "meta": {
                        "status": 429,
                        "msg": "Rate Limit Exceeded"
                    },
                    "response": []
                })),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::RateLimit { retry_after } => {
            assert_eq!(retry_after, Some(60));
        }
        _ => panic!("Expected rate limit error"),
    }
}

#[tokio::test]
async fn test_rate_limit_error_per_day() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-perhour-remaining", "100")
                .insert_header("x-ratelimit-perhour-reset", "60")
                .insert_header("x-ratelimit-perday-remaining", "0")
                .insert_header("x-ratelimit-perday-reset", "3600")
                .set_body_json(serde_json::json!({
                    "meta": {
                        "status": 429,
                        "msg": "Rate Limit Exceeded"
                    },
                    "response": []
                })),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::RateLimit { retry_after } => {
            assert_eq!(retry_after, Some(3600));
        }
        _ => panic!("Expected rate limit error"),
    }
}

#[tokio::test]
async fn test_rate_limit_both_exhausted_prefers_daily() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-perhour-remaining", "0")
                .insert_header("x-ratelimit-perhour-reset", "60")
                .insert_header("x-ratelimit-perday-remaining", "0")
                .insert_header("x-ratelimit-perday-reset", "7200")
                .set_body_json(serde_json::json!({
                    "meta": {
                        "status": 429,
                        "msg": "Rate Limit Exceeded"
                    },
                    "response": []
                })),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::RateLimit { retry_after } => {
            // When both limits are exhausted, the daily reset is used because
            // it's the longer wait and the hourly reset alone wouldn't help.
            assert_eq!(retry_after, Some(7200));
        }
        _ => panic!("Expected rate limit error"),
    }
}

#[tokio::test]
async fn test_rate_limit_no_headers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "meta": {
                "status": 429,
                "msg": "Rate Limit Exceeded"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::RateLimit { retry_after } => {
            // Without Tumblr rate limit headers, retry_after is unknown
            assert_eq!(retry_after, None);
        }
        _ => panic!("Expected rate limit error"),
    }
}

#[tokio::test]
async fn test_rate_limit_nonzero_remaining() {
    let mock_server = MockServer::start().await;

    // 429 but remaining counts are both nonzero (e.g., proxy-level throttle)
    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("x-ratelimit-perhour-remaining", "50")
                .insert_header("x-ratelimit-perday-remaining", "200")
                .set_body_json(serde_json::json!({
                    "meta": {
                        "status": 429,
                        "msg": "Rate Limit Exceeded"
                    },
                    "response": []
                })),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::RateLimit { retry_after } => {
            // Neither limit is at 0, so we can't determine a meaningful retry time
            assert_eq!(retry_after, None);
        }
        _ => panic!("Expected rate limit error"),
    }
}

#[tokio::test]
async fn test_blog_posts_with_filters() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/posts"))
        .and(query_param("limit", "5"))
        .and(query_param("type", "photo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id_string": "123456",
                        "blog_name": "staff",
                        "post_url": "https://staff.tumblr.com/post/123456",
                        "type": "photo",
                        "timestamp": 1234567890,
                        "tags": ["announcement"],
                        "note_count": 100
                    }
                ],
                "total_posts": 1000
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs("staff")
        .posts()
        .limit(5)
        .post_type("photo")
        .send()
        .await;

    assert!(result.is_ok());
    let posts = result.unwrap();
    assert_eq!(posts.posts.len(), 1);
    assert_eq!(posts.posts[0].post_type, "photo");
    assert_eq!(posts.total_posts, 1000);
}

#[tokio::test]
async fn test_blog_queue() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_queue.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts/queue"))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").queue().limit(5).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let queue = result.unwrap();
    assert_eq!(queue.posts.len(), 1);
    assert_eq!(queue.posts[0].id, "823667986154094592");
    assert_eq!(queue.posts[0].state.as_deref(), Some("queued"));
}

#[tokio::test]
async fn test_blog_queue_with_offset() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/queue", TEST_BLOG_NAME)))
        .and(query_param("limit", "10"))
        .and(query_param("offset", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 123460,
                        "id_string": "123460",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/123460", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567895,
                        "tags": [],
                        "note_count": 0
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .queue()
        .limit(10)
        .offset(5)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let queue = result.unwrap();
    assert_eq!(queue.posts.len(), 1);
    assert_eq!(queue.posts[0].id, "123460");
}

#[tokio::test]
async fn test_queue_reorder() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/queue/reorder",
            TEST_BLOG_NAME
        )))
        .and(body_json(serde_json::json!({
            "post_id": "123456",
            "insert_after": "0"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "success": true
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .reorder_queue("123456", "0")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_queue_reorder_after_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/queue/reorder",
            TEST_BLOG_NAME
        )))
        .and(body_json(serde_json::json!({
            "post_id": "123456",
            "insert_after": "789012"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "success": true
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .reorder_queue("123456", "789012")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_queue_shuffle() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/queue/shuffle",
            TEST_BLOG_NAME
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "success": true
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).shuffle_queue().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.success, Some(true));
}

#[tokio::test]
async fn test_blog_drafts() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_drafts.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts/draft"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").drafts().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let drafts = result.unwrap();
    assert_eq!(drafts.posts.len(), 1);
    assert_eq!(drafts.posts[0].id, "823667985992564736");
    assert_eq!(drafts.posts[0].state.as_deref(), Some("draft"));
}

#[tokio::test]
async fn test_blog_drafts_with_before_id() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/draft", TEST_BLOG_NAME)))
        .and(query_param("before_id", "333333"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 111111,
                        "id_string": "111111",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/111111", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567890,
                        "state": "draft",
                        "tags": [],
                        "note_count": 0
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .drafts()
        .before_id("333333")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let drafts = result.unwrap();
    assert_eq!(drafts.posts.len(), 1);
    assert_eq!(drafts.posts[0].id, "111111");
}

#[tokio::test]
async fn test_blog_drafts_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/draft", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).drafts().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let drafts = result.unwrap();
    assert!(drafts.posts.is_empty());
}

// =============================================================================
// Submission endpoint tests
// =============================================================================

#[tokio::test]
async fn test_blog_submissions() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/submission", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 444444,
                        "id_string": "444444",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/444444", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567890,
                        "state": "submission",
                        "tags": ["submitted"],
                        "note_count": 0,
                        "post_author": "friendly-submitter",
                        "is_submission": true
                    },
                    {
                        "id": 555555,
                        "id_string": "555555",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/555555", TEST_BLOG_NAME),
                        "type": "photo",
                        "timestamp": 1234567891,
                        "state": "submission",
                        "tags": [],
                        "note_count": 0,
                        "post_author": "photo-lover",
                        "is_submission": true
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).submissions().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert_eq!(submissions.posts.len(), 2);
    assert_eq!(submissions.posts[0].id, "444444");
    assert_eq!(
        submissions.posts[0].post_author,
        Some("friendly-submitter".to_string())
    );
    assert_eq!(submissions.posts[0].is_submission, Some(true));
    assert_eq!(submissions.posts[0].state, Some("submission".to_string()));
    assert_eq!(submissions.posts[1].id, "555555");
    assert_eq!(
        submissions.posts[1].post_author,
        Some("photo-lover".to_string())
    );
}

#[tokio::test]
async fn test_blog_submissions_with_offset() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/submission", TEST_BLOG_NAME)))
        .and(query_param("offset", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 666666,
                        "id_string": "666666",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/666666", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567892,
                        "state": "submission",
                        "tags": [],
                        "note_count": 0,
                        "post_author": "offset-submitter",
                        "is_submission": true
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .submissions()
        .offset(10)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert_eq!(submissions.posts.len(), 1);
    assert_eq!(submissions.posts[0].id, "666666");
}

#[tokio::test]
async fn test_blog_submissions_with_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/submission", TEST_BLOG_NAME)))
        .and(query_param("filter", "text"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 777777,
                        "id_string": "777777",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/777777", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567893,
                        "state": "submission",
                        "tags": [],
                        "note_count": 0,
                        "post_author": "text-only",
                        "is_submission": true
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .submissions()
        .filter("text")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert_eq!(submissions.posts.len(), 1);
    assert_eq!(submissions.posts[0].id, "777777");
}

#[tokio::test]
async fn test_blog_submissions_empty() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_submissions.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts/submission"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").submissions().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert!(submissions.posts.is_empty());
}

#[tokio::test]
async fn test_blog_submissions_anonymous() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/submission", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 888888,
                        "id_string": "888888",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/888888", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567894,
                        "state": "submission",
                        "tags": ["anonymous"],
                        "note_count": 0,
                        "is_submission": true,
                        "anonymous_name": "Anonymous Fan",
                        "anonymous_email": "anon@example.com"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).submissions().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert_eq!(submissions.posts.len(), 1);
    assert_eq!(submissions.posts[0].id, "888888");
    assert_eq!(
        submissions.posts[0].anonymous_name,
        Some("Anonymous Fan".to_string())
    );
    assert_eq!(
        submissions.posts[0].anonymous_email,
        Some("anon@example.com".to_string())
    );
    assert!(submissions.posts[0].post_author.is_none());
}

// =============================================================================
// Notifications endpoint tests
// =============================================================================

#[tokio::test]
async fn test_blog_notifications() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notifications.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notifications"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").notifications().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    // The only notification type present in this capture is milestone_post.
    assert_eq!(notifs.notifications.len(), 2);
    assert!(
        notifs
            .notifications
            .iter()
            .all(|n| n.notification_type == "milestone_post")
    );
    assert_eq!(
        notifs.notifications[0].target_tumblelog_name,
        Some("crabrave-demo".to_string())
    );

    // Check pagination links
    assert!(notifs.links.is_some());
    let links = notifs.links.unwrap();
    assert!(links.next.is_some());
    assert!(links.next.unwrap().href.contains("before="));
}

#[tokio::test]
async fn test_blog_notifications_with_before() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notifications", TEST_BLOG_NAME)))
        .and(query_param("before", "1234567800"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notifications": [
                    {
                        "id": "notif-older-001",
                        "type": "like",
                        "timestamp": 1234567750,
                        "unread": false,
                        "target_post_id": "111111",
                        "from_tumblelog_name": "older-liker"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .before(1234567800)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 1);
    assert_eq!(notifs.notifications[0].id, "notif-older-001");
}

#[tokio::test]
async fn test_blog_notifications_with_types_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notifications", TEST_BLOG_NAME)))
        .and(query_param("types[0]", "like"))
        .and(query_param("types[1]", "follow"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notifications": [
                    {
                        "id": "notif-like-001",
                        "type": "like",
                        "timestamp": 1234567890,
                        "unread": true,
                        "target_post_id": "123456",
                        "from_tumblelog_name": "liker"
                    },
                    {
                        "id": "notif-follow-001",
                        "type": "follow",
                        "timestamp": 1234567880,
                        "unread": true,
                        "from_tumblelog_name": "follower"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NotificationType;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .types(vec![NotificationType::Like, NotificationType::Follow])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 2);
    assert!(
        notifs
            .notifications
            .iter()
            .all(|n| n.notification_type == "like" || n.notification_type == "follow")
    );
}

#[tokio::test]
async fn test_blog_notifications_with_rollups_disabled() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notifications", TEST_BLOG_NAME)))
        .and(query_param("rollups", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notifications": [
                    {
                        "id": "notif-individual-001",
                        "type": "like",
                        "timestamp": 1234567890,
                        "unread": true,
                        "target_post_id": "123456",
                        "from_tumblelog_name": "liker1"
                    },
                    {
                        "id": "notif-individual-002",
                        "type": "like",
                        "timestamp": 1234567889,
                        "unread": true,
                        "target_post_id": "123456",
                        "from_tumblelog_name": "liker2"
                    },
                    {
                        "id": "notif-individual-003",
                        "type": "like",
                        "timestamp": 1234567888,
                        "unread": true,
                        "target_post_id": "123456",
                        "from_tumblelog_name": "liker3"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .rollups(false)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 3);
    // All individual like notifications should be present
    assert!(
        notifs
            .notifications
            .iter()
            .all(|n| n.notification_type == "like")
    );
}

#[tokio::test]
async fn test_blog_notifications_with_omit_post_ids() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notifications", TEST_BLOG_NAME)))
        .and(query_param("omit_post_ids[0]", "111111"))
        .and(query_param("omit_post_ids[1]", "222222"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notifications": [
                    {
                        "id": "notif-other-post",
                        "type": "like",
                        "timestamp": 1234567890,
                        "unread": true,
                        "target_post_id": "333333",
                        "from_tumblelog_name": "liker"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .omit_post_ids(vec!["111111", "222222"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 1);
    assert_eq!(
        notifs.notifications[0].target_post_id,
        Some("333333".to_string())
    );
}

#[tokio::test]
async fn test_blog_notifications_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notifications", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notifications": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).notifications().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert!(notifs.notifications.is_empty());
    assert!(notifs.links.is_none());
}

#[tokio::test]
async fn test_blog_notifications_ask_type() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notifications", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notifications": [
                    {
                        "id": "notif-ask-001",
                        "type": "ask",
                        "timestamp": 1234567890,
                        "unread": true,
                        "from_tumblelog_name": "curious-anon",
                        "target_tumblelog_name": TEST_BLOG_NAME,
                        "summary": "What's your favorite color?"
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).notifications().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 1);
    assert_eq!(notifs.notifications[0].notification_type, "ask");
    assert_eq!(
        notifs.notifications[0].summary,
        Some("What's your favorite color?".to_string())
    );
    assert_eq!(
        notifs.notifications[0].target_tumblelog_name,
        Some(TEST_BLOG_NAME.to_string())
    );
}

// =============================================================================
// Notes endpoint tests
// =============================================================================

#[tokio::test]
async fn test_blog_notes() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notes.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notes"))
        .and(query_param("id", "819597601864335360"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs("crabrave-demo")
        .notes("819597601864335360")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 8);
    assert_eq!(notes.total_notes, 10885);
    assert_eq!(notes.notes[0].note_type, "reblog");
    assert_eq!(notes.notes[0].blog_name, "example-blog-002");
    assert_eq!(
        notes.notes[0].reblog_parent_blog_name,
        Some("example-blog-014".to_string())
    );
    assert_eq!(notes.notes[2].note_type, "like");
    assert!(!notes.notes[2].followed);

    let links = notes.links.expect("expected pagination links");
    let next = links.next.expect("expected a next link");
    assert_eq!(next.query_params.before_timestamp, "1785084215");
}

#[tokio::test]
async fn test_blog_notes_likes_mode() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notes_likes.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notes"))
        .and(query_param("id", "819597601864335360"))
        .and(query_param("mode", "likes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs("crabrave-demo")
        .notes("819597601864335360")
        .mode(NoteMode::Likes)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 8);
    assert_eq!(notes.total_notes, 6597);
    assert!(notes.notes.iter().all(|n| n.note_type == "like"));
}

#[tokio::test]
async fn test_blog_notes_conversation_mode() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notes_conversation.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notes"))
        .and(query_param("id", "819597601864335360"))
        .and(query_param("mode", "conversation"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let notes = client
        .blogs("crabrave-demo")
        .notes("819597601864335360")
        .mode(NoteMode::Conversation)
        .send()
        .await
        .expect("Failed to get conversation-mode notes");

    assert_eq!(notes.total_notes, 10885);
    assert_eq!(notes.total_likes, Some(6597));
    assert_eq!(notes.total_reblogs, Some(4229));
    // Non-commentary interactions are collapsed into a single rollup group
    // that carries its own aggregates and a sample of the collapsed notes.
    assert_eq!(notes.rollup_notes.len(), 1);
    let group = &notes.rollup_notes[0];
    assert_eq!(group.total_notes, 10885);
    assert_eq!(group.total_likes, Some(6597));
    assert_eq!(group.total_reblogs, Some(4229));
    assert_eq!(group.notes.len(), 50);
    assert_eq!(group.notes[0].note_type, "reblog");
    assert_eq!(group.notes[0].blog_name, "example-blog-002");
}

#[tokio::test]
async fn test_blog_notes_reblogs_with_tags_mode() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notes_reblogs_with_tags.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notes"))
        .and(query_param("id", "819597601864335360"))
        .and(query_param("mode", "reblogs_with_tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs("crabrave-demo")
        .notes("819597601864335360")
        .mode(NoteMode::ReblogsWithTags)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 8);
    assert!(notes.notes.iter().all(|n| n.note_type == "reblog"));
    assert_eq!(
        notes.notes[0].reblog_parent_blog_name,
        Some("example-blog-014".to_string())
    );
    // None of these reblogs added tags, so the `tags` array is present but empty.
    assert!(notes.notes[0].tags.is_empty());
    // This mode's response carries only `notes` and `_links`; total_notes is
    // absent and defaults to 0.
    assert_eq!(notes.total_notes, 0);
}

#[tokio::test]
async fn test_blog_notes_with_before_timestamp() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .and(query_param("before_timestamp", "1234567800"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [
                    {
                        "type": "like",
                        "timestamp": 1234567750,
                        "blog_name": "older_liker",
                        "followed": false
                    }
                ],
                "total_notes": 150
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .before_timestamp(1234567800)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 1);
    assert_eq!(notes.notes[0].blog_name, "older_liker");
    assert!(notes.notes[0].timestamp < 1234567800);
}

#[tokio::test]
async fn test_blog_notes_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [],
                "total_notes": 0
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).notes("123456789").send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert!(notes.notes.is_empty());
    assert_eq!(notes.total_notes, 0);
}

#[tokio::test]
async fn test_blog_notes_rollup_mode() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notes_rollup.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notes"))
        .and(query_param("id", "819597601864335360"))
        .and(query_param("mode", "rollup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs("crabrave-demo")
        .notes("819597601864335360")
        .mode(NoteMode::Rollup)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 8);
    assert_eq!(notes.total_notes, 10885);
    // Rollup mode returns only likes and reblogs
    assert!(
        notes
            .notes
            .iter()
            .all(|n| n.note_type == "like" || n.note_type == "reblog")
    );
}

#[tokio::test]
async fn test_blog_notes_page2() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_notes_page2.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/notes"))
        .and(query_param("id", "819597601864335360"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs("crabrave-demo")
        .notes("819597601864335360")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 8);

    // The live API quotes before_timestamp inside _links.next.query_params;
    // this is the shape that used to fail to parse.
    let links = notes.links.expect("expected pagination links");
    let next = links.next.expect("expected a next link");
    assert_eq!(next.query_params.before_timestamp, "1784951304");
}

#[tokio::test]
async fn test_user_info() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_info.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().info().await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.user.name, "demo-primary");
    assert_eq!(info.user.likes, 111534);
    assert_eq!(info.user.blogs.len(), 4);
}

#[tokio::test]
async fn test_tagged_posts() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/tagged.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    // The tagged endpoint returns posts directly as the response array,
    // which this fixture reflects (no `response.posts` wrapper).
    Mock::given(method("GET"))
        .and(path("/tagged"))
        .and(query_param("tag", "photography"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.tagged("photography").limit(10).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let tagged = result.unwrap();
    assert_eq!(tagged.posts.len(), 4);
    assert_eq!(tagged.posts[0].id, "823667709349888002");
    assert!(
        tagged
            .posts
            .iter()
            .all(|p| p.tags.contains(&"photography".to_string()))
    );
}

#[tokio::test]
async fn test_delete_post() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/post_delete.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("POST"))
        .and(path("/blog/crabrave-demo/post/delete"))
        .and(body_json(serde_json::json!({ "id": "823667986154094592" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs("crabrave-demo")
        .post("823667986154094592")
        .delete()
        .await;

    assert!(result.is_ok());
    // The live API returns `id` as a JSON number plus `id_string`; both must
    // deserialize to the same value.
    let response = result.unwrap();
    assert_eq!(response.id, Some("823667986154094592".to_string()));
    assert_eq!(response.id_string, Some("823667986154094592".to_string()));
}

#[tokio::test]
async fn test_communities_timeline() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/timeline"))
        .and(query_param("npf", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "timeline": {
                    "elements": [
                        {
                            "id_string": "111222",
                            "blog_name": "rustdev",
                            "post_url": "https://rustdev.tumblr.com/post/111222",
                            "type": "blocks",
                            "timestamp": 1234567890,
                            "reblog_key": "reblogkey1",
                            "is_blocks_post_format": true,
                            "tags": ["rust"],
                            "note_count": 25,
                            "content": [{"type": "text", "text": "hello"}],
                            "layout": [],
                            "trail": [],
                            "followed": false,
                            "can_react": true,
                            "reactions": [
                                {
                                    "emoji": {
                                        "id": "735345404083896320",
                                        "grapheme": "👍",
                                        "base_slug": "thumbs_up"
                                    },
                                    "count": 61,
                                    "reacted": false
                                }
                            ]
                        }
                    ],
                    "_links": {
                        "next": {
                            "href": "/v2/communities/rust-community/timeline?cursor=CURSOR_OPAQUE_1",
                            "method": "GET",
                            "query_params": {
                                "community": "rust-community",
                                "rawurldecode": "1",
                                "cursor": "CURSOR_OPAQUE_1"
                            }
                        }
                    }
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.communities("rust-community").timeline().send().await;

    assert!(result.is_ok());
    let timeline = result.unwrap();
    assert_eq!(timeline.posts.len(), 1);
    assert_eq!(
        timeline.links.unwrap().next.unwrap().query_params.cursor,
        Some("CURSOR_OPAQUE_1".to_string())
    );
}

#[tokio::test]
async fn test_communities_timeline_type_polls() {
    use crabrave::handlers::communities::TimelineType;

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/timeline"))
        .and(query_param("type", "polls"))
        .and(query_param("npf", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "timeline": {
                    "elements": []
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .timeline()
        .timeline_type(TimelineType::Polls)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert!(result.unwrap().posts.is_empty());
}

#[tokio::test]
async fn test_communities_timeline_cursor() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/timeline"))
        .and(query_param("cursor", "CURSOR_OPAQUE_1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "timeline": {
                    "elements": []
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .timeline()
        .cursor("CURSOR_OPAQUE_1")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_npf_post_creation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/blog/myblog/posts"))
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
        .blogs("myblog")
        .create_post()
        .add_block(crabrave::npf::ContentBlock::heading("My Title", 1))
        .add_block(crabrave::npf::ContentBlock::text("Body text"))
        .tags(vec!["npf"])
        .send()
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.id, "555444333");
}

#[tokio::test]
async fn test_npf_post_creation_with_image() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "image-post-123"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .content(vec![
            crabrave::npf::ContentBlock::text("Check out this photo!"),
            crabrave::npf::ContentBlock::image("https://example.com/photo.jpg"),
        ])
        .tags(vec!["photo", "photography"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.id, "image-post-123");
}

#[tokio::test]
async fn test_npf_post_creation_as_draft() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/post_create_draft.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .add_block(crabrave::npf::ContentBlock::text("Work in progress..."))
        .state("draft")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.id, "823667985992564736");
}

#[tokio::test]
async fn test_npf_post_creation_queued() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/post_create_queue.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .content(vec![
            crabrave::npf::ContentBlock::heading("Scheduled Post", 1),
            crabrave::npf::ContentBlock::text("This will be posted later"),
        ])
        .state("queue")
        .tags(vec!["scheduled"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.id, "823667986154094592");
}

#[tokio::test]
async fn test_npf_post_creation_with_slug() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "slug-post-111"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .add_block(crabrave::npf::ContentBlock::heading("Custom URL Post", 1))
        .add_block(crabrave::npf::ContentBlock::text(
            "This post has a custom URL slug",
        ))
        .slug("custom-url-slug")
        .tags(vec!["custom-slug"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.id, "slug-post-111");
}

#[tokio::test]
async fn test_npf_post_creation_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .add_block(crabrave::npf::ContentBlock::text("Test"))
        .send()
        .await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_npf_post_creation_with_link_block() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "link-post-222"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .create_post()
        .content(vec![
            crabrave::npf::ContentBlock::text("Check out this awesome link:"),
            crabrave::npf::ContentBlock::link("https://www.rust-lang.org"),
        ])
        .tags(vec!["rust", "programming", "links"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.id, "link-post-222");
}

#[tokio::test]
async fn test_network_error() {
    // Create client with invalid URL to trigger network error
    let client = Crabrave::builder()
        .consumer_key("test")
        .consumer_secret("test")
        .access_token("test")
        .base_url("http://invalid-domain-that-does-not-exist-12345.com")
        .build()
        .expect("Failed to build client");

    let result = client.blogs("staff").info().await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CrabError::Http(_) => {} // Expected
        e => panic!("Expected HTTP error, got: {:?}", e),
    }
}

#[tokio::test]
async fn test_avatar_endpoint() {
    let avatar = include_bytes!("./fixtures/demo_avatar.png");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/demo/avatar/64"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(avatar)
                .insert_header("Content-Type", "image/png"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs("demo")
        .avatar(Some(64))
        .await
        .expect("Callout failed");

    assert!(matches!(result, AvatarResponse::ImageData(_)));
}

#[tokio::test]
async fn test_get_blocks() {
    let fixture = include_str!("fixtures/live/blog_blocks.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .blocks()
        .send()
        .await
        .expect("Callout failed");

    assert_eq!(result.blocked_tumblelogs.len(), 9);
}

#[tokio::test]
async fn test_block_blog() {
    let mock_server = MockServer::start().await;

    let blog_to_block = "johnny-depp-is-loved";

    Mock::given(method("POST"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .and(body_json(
            serde_json::json!({ "blocked_tumblelog": blog_to_block }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "already_blocked": false
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .block_blog(blog_to_block)
        .await
        .expect("callout failed");

    assert!(!result.already_blocked);
}

#[tokio::test]
async fn test_block_with_post_id() {
    let mock_server = MockServer::start().await;

    let post_to_block = "12345";

    Mock::given(method("POST"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .and(body_json(serde_json::json!({ "post_id": post_to_block })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "already_blocked": false
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .block_with_post_id(post_to_block.to_string())
        .await
        .expect("callout failed");

    assert!(!result.already_blocked);
}

#[tokio::test]
async fn test_bulk_block() {
    let mock_server = MockServer::start().await;

    let blogs_to_block = "foo,bar,baz";

    Mock::given(method("POST"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks/bulk")))
        .and(body_json(
            serde_json::json!({ "blocked_tumblelogs": blogs_to_block, "force": true }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    client
        .blogs(TEST_BLOG_NAME)
        .bulk_block(vec!["foo", "bar", "baz"], true)
        .await
        .expect("callout failed");
}

#[tokio::test]
async fn test_unblock() {
    let mock_server = MockServer::start().await;

    let blog_to_unblock = "foo";
    Mock::given(method("DELETE"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .and(query_param("blocked_tumblelog", blog_to_unblock))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    client
        .blogs(TEST_BLOG_NAME)
        .unblock(blog_to_unblock)
        .await
        .expect("callout failed");
}

#[tokio::test]
async fn test_unblock_all_anonymous() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .and(query_param("anonymous_only", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    client
        .blogs(TEST_BLOG_NAME)
        .unblock_all_anonymous()
        .await
        .expect("callout failed");
}

#[tokio::test]
async fn test_blog_likes() {
    let mock_server = MockServer::start().await;

    // Blog likes and user likes share a response shape, so the live capture
    // for the latter (there's no separate live blog-likes capture) backs both.
    let fixture = include_str!("fixtures/live/user_likes.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/likes")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .likes()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.liked_count, 111534);
    assert_eq!(result.liked_posts.len(), 3);
    assert_eq!(result.liked_posts[0].id, "823667562844930048");
}

#[tokio::test]
async fn test_blog_following() {
    let mock_server = MockServer::start().await;

    // Blog-following of a secondary blog is 403 for its own owner (see
    // test_blog_following_secondary_forbidden), so this happy path could not
    // be captured live; it reuses the user-following capture on the
    // assumption that the two endpoints share a response shape per Tumblr's
    // docs.
    let fixture = include_str!("fixtures/live/user_following.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/following")))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .following()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.total_blogs, 1241);
    assert_eq!(result.blogs.len(), 5);
}

#[tokio::test]
async fn test_user_following() {
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
        .expect("Callout to get blog likes failed");

    assert_eq!(result.total_blogs, 1241);
    assert_eq!(result.blogs.len(), 5);
    assert_eq!(result.blogs[0].name, "example-blog-068");
}

#[tokio::test]
async fn test_blog_followers() {
    let mock_server = MockServer::start().await;

    // The live capture for this endpoint is a real empty state (the demo blog
    // has no followers; see test_blog_followers_empty), so the populated case
    // here is a hand-crafted, unverified body.
    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/followers")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "total_users": 2450,
                "users": [
                    {
                        "name": "a-follower",
                        "url": "https://a-follower.tumblr.com/",
                        "following": true,
                        "updated": 1234567890
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .followers()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.total_users, 2450);
    assert_eq!(result.users.len(), 1);
}

#[tokio::test]
async fn test_blog_followers_empty() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_followers.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/followers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs("crabrave-demo")
        .followers()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.total_users, 0);
    assert!(result.users.is_empty());
}

#[tokio::test]
async fn test_blog_following_secondary_forbidden() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_following_403.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    // Fetching who a *secondary* blog follows is 403 for that blog's own owner.
    Mock::given(method("GET"))
        .and(path("/blog/demo-sideblog/following"))
        .respond_with(ResponseTemplate::new(403).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("demo-sideblog").following().send().await;

    match result {
        Err(CrabError::Api { status, .. }) => assert_eq!(status, 403),
        other => panic!("Expected Api error, got: {:?}", other.map(|_| ())),
    }
}

#[tokio::test]
async fn test_blog_likes_secondary_forbidden() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_likes_403.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    // Likes of a secondary blog are also 403 without the share-likes setting;
    // the envelope's `response` is a bare string here rather than an object.
    Mock::given(method("GET"))
        .and(path("/blog/demo-sideblog/likes"))
        .respond_with(ResponseTemplate::new(403).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("demo-sideblog").likes().send().await;

    match result {
        Err(CrabError::Api { status, .. }) => assert_eq!(status, 403),
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_followed_by() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/followed_by")))
        .and(query_param("query", "foobar"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "meta": {
            "status": 200,
            "msg": "OK"
        },
            "response": {
                "followed_by": true
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .followed_by("foobar")
        .await
        .expect("Callout to get blog likes failed");

    assert!(result);
}

// =============================================================================
// Posts endpoint tests
// =============================================================================

#[tokio::test]
async fn test_get_post() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/single_post.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts/819597601864335360"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs("crabrave-demo")
        .post("819597601864335360")
        .get()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let post_response = result.unwrap();
    assert_eq!(post_response.id, "819597601864335360");
    assert_eq!(post_response.blog_name, "crabrave-demo");
    assert_eq!(post_response.post_type, "blocks");
    assert_eq!(post_response.reblog_key, "4UatWJQP");
    assert_eq!(post_response.content.len(), 1);
}

#[tokio::test]
async fn test_edit_post_npf() {
    let mock_server = MockServer::start().await;

    // NPF edit uses PUT to /blog/{blog}/posts/{id} with content blocks
    Mock::given(method("PUT"))
        .and(path(format!("/blog/{}/posts/123456", TEST_BLOG_NAME)))
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
        .content(vec![
            crabrave::npf::ContentBlock::heading("Updated Title", 1),
            crabrave::npf::ContentBlock::text("Updated body content"),
        ])
        .tags(vec!["updated", "edited"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let edit_response = result.unwrap();
    assert_eq!(edit_response.id, "123456");
}

#[tokio::test]
async fn test_edit_post_npf_with_state() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(format!("/blog/{}/posts/789012", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "id": "789012"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("789012")
        .edit()
        .add_block(crabrave::npf::ContentBlock::text("Moving to draft"))
        .state("draft")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let edit_response = result.unwrap();
    assert_eq!(edit_response.id, "789012");
}

#[tokio::test]
async fn test_edit_post_npf_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(format!("/blog/{}/posts/nonexistent", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("nonexistent")
        .edit()
        .content(vec![crabrave::npf::ContentBlock::text("Test")])
        .send()
        .await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_reblog_post_with_comment() {
    let mock_server = MockServer::start().await;

    // NPF reblog uses POST to /blog/{blog}/posts with parent_tumblelog_uuid.
    // The exact body match proves a plain `.comment()` is sent as an NPF text
    // block, not a legacy top-level `comment` key.
    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .and(body_json(serde_json::json!({
            "parent_tumblelog_uuid": "parent-blog-uuid",
            "parent_post_id": "789012",
            "reblog_key": "abc123reblogkey",
            "content": [
                {"type": "text", "text": "Great post!"}
            ],
            "tags": "reblog,interesting"
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "999888"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .reblog("parent-blog-uuid", "789012", "abc123reblogkey")
        .comment("Great post!")
        .tags(vec!["reblog", "interesting"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let reblog_response = result.unwrap();
    assert_eq!(reblog_response.id, "999888");
}

#[tokio::test]
async fn test_reblog_post_with_npf_content() {
    let mock_server = MockServer::start().await;

    // NPF reblog with rich content blocks instead of simple comment
    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
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
        .reblog("parent-blog-uuid", "original-post-id", "reblogkey123")
        .content(vec![
            crabrave::npf::ContentBlock::heading("My thoughts", 1),
            crabrave::npf::ContentBlock::text("This is a really interesting post!"),
            crabrave::npf::ContentBlock::text("I have more to say about it."),
        ])
        .tags(vec!["thoughts", "reblog"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let reblog_response = result.unwrap();
    assert_eq!(reblog_response.id, "111222333");
}

#[tokio::test]
async fn test_reblog_post_simple() {
    let mock_server = MockServer::start().await;

    // Simple reblog without comment or content
    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "444555666"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .reblog("parent-blog-uuid", "source-post-id", "key123")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let reblog_response = result.unwrap();
    assert_eq!(reblog_response.id, "444555666");
}

#[tokio::test]
async fn test_reblog_post_to_draft() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "draft-reblog-123"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .reblog("parent-blog-uuid", "post-to-reblog", "reblogkey")
        .comment("Saving this for later")
        .state("draft")
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let reblog_response = result.unwrap();
    assert_eq!(reblog_response.id, "draft-reblog-123");
}

#[tokio::test]
async fn test_reblog_post_invalid_key() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "meta": {
                "status": 400,
                "msg": "Bad Request"
            },
            "response": {
                "errors": ["Invalid reblog key"]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .reblog("parent-blog-uuid", "post-id", "invalid-key")
        .send()
        .await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api { status, .. }) => {
            assert_eq!(status, 400);
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// Community endpoint tests
// =============================================================================

#[tokio::test]
async fn test_community_info() {
    use crabrave::handlers::communities::MemberRole;

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/test-community"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "name": "test-community",
                "uuid": "t:AbCdEfGh12345",
                "title": "Test Community",
                "description": "A community for testing",
                "about": "About the test community",
                "guidelines_summary": "",
                "guidelines": ["Be kind"],
                "language": "en_US",
                "location": "",
                "visibility": "public",
                "member_count": 1119,
                "members_online_count": 311,
                "mention_tag": "@@test-community",
                "is_member": true,
                "can_view": true,
                "can_post": true,
                "can_edit": false,
                "can_interact": true,
                "can_invite": false,
                "can_view_comments": true,
                "can_view_all_members": true,
                "post_count": 276,
                "unread_post_count": 0,
                "categories": [],
                "tags": ["testing", "rust"],
                "blog_ids": [],
                "boards": [],
                "created_ts": 1734388393,
                "avatar_image": [
                    {
                        "width": 512,
                        "height": 512,
                        "url": "https://64.media.tumblr.com/test/s512x512u_c1/avatar.jpg",
                        "accessories": []
                    },
                    {
                        "width": 128,
                        "height": 128,
                        "url": "https://64.media.tumblr.com/test/s128x128u_c1/avatar.jpg",
                        "accessories": []
                    }
                ],
                "population_cap": 10000,
                "join_type": "request",
                "has_content_label": false,
                "content_label_categories": [],
                "content_label_last_reporter": null,
                "role": "member",
                "blog": { "name": "my-blog" },
                "is_muted": false,
                "header_image": [
                    {
                        "media_key": "abc123:def456-4d",
                        "type": "image/jpeg",
                        "width": 1284,
                        "height": 527,
                        "url": "https://64.media.tumblr.com/test/s2048x3072/header.jpg",
                        "has_original_dimensions": true
                    }
                ],
                "related_communities": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.communities("test-community").info().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let community = result.unwrap();
    assert_eq!(community.name, "test-community");
    assert_eq!(community.uuid, "t:AbCdEfGh12345");
    assert_eq!(community.avatar_image.len(), 2);
    assert_eq!(community.header_image[0].width, Some(1284));
    assert_eq!(community.role, Some(MemberRole::Member));
    assert_eq!(community.is_muted, Some(false));
    assert_eq!(community.blog.unwrap().name, "my-blog");
    assert_eq!(community.join_type.as_deref(), Some("request"));
    assert_eq!(community.member_count, Some(1119));
}

#[tokio::test]
async fn test_community_info_for_edit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/test-community"))
        .and(query_param("context", "edit"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "name": "test-community",
                "uuid": "t:AbCdEfGh12345",
                "title": "Test Community",
                "handle_can_change": false,
                "handle_next_change_ts": 1234567890,
                "settings": {
                    "invite_links_enabled": true,
                    "join_type": "invite"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.communities("test-community").info_for_edit().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let community = result.unwrap();
    assert_eq!(community.settings.unwrap().invite_links_enabled, Some(true));
    assert_eq!(community.handle_can_change, Some(false));
}

#[tokio::test]
async fn test_community_edit() {
    use crabrave::handlers::communities::CommunityPrivacy;

    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/test-community"))
        .and(body_json(serde_json::json!({
            "title": "New Title",
            "privacy": 1
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "name": "test-community",
                "uuid": "t:AbCdEfGh12345",
                "title": "New Title",
                "handle_can_change": false,
                "handle_next_change_ts": 1234567890,
                "settings": {
                    "invite_links_enabled": true,
                    "join_type": "invite"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("test-community")
        .edit()
        .title("New Title")
        .privacy(CommunityPrivacy::Private)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().title, Some("New Title".to_string()));
}

#[tokio::test]
async fn test_community_join() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.communities("rust-community").join().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_join_with_invite_hash() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/members"))
        .and(body_json(serde_json::json!({
            "invite_hash": "abcd123"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .join_with_invite_hash("abcd123")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_leave() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/members/my-blog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.communities("rust-community").leave("my-blog").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_members() {
    use crabrave::handlers::communities::MemberFilter;

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/members"))
        .and(query_param("page", "2"))
        .and(query_param("filter", "admin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "count": 1119,
                "members": [
                    {
                        "name": "admin-blog",
                        "title": "Admin Blog",
                        "url": "https://admin-blog.tumblr.com/",
                        "uuid": "t:MemberUuid1",
                        "updated": 1740000000,
                        "description": "",
                        "avatar": [
                            {
                                "width": 64,
                                "height": 64,
                                "url": "https://64.media.tumblr.com/avatar/test64.jpg",
                                "accessories": []
                            }
                        ],
                        "can_show_badges": true,
                        "tumblrmart_accessories": {},
                        "followed": true,
                        "is_following_you": false,
                        "is_mutual": false,
                        "is_online": true,
                        "is_admin": true,
                        "is_moderator": false,
                        "joined_ts": 1734388393
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .members()
        .page(2)
        .filter(MemberFilter::Admin)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.total_members, 1119);
    assert!(response.members[0].is_admin);
    assert!(response.members[0].blog.followed);
    assert_eq!(response.members[0].joined_ts, Some(1734388393));
}

#[tokio::test]
async fn test_community_invite() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/invitations"))
        .and(body_json(serde_json::json!({
            "recipient_blog": "friend-blog"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .invite("friend-blog")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_invite_with_message() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/invitations"))
        .and(body_json(serde_json::json!({
            "recipient_blog": "friend-blog",
            "message": "come join us"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .invite_with_message("friend-blog", "come join us")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_add_reaction_slug() {
    use crabrave::handlers::communities::Reaction;

    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(
            "/communities/rust-community/posts/123456789/reactions",
        ))
        .and(body_json(serde_json::json!({
            "slug": ":heart:"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .add_reaction("123456789", Reaction::slug(":heart:"))
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_add_reaction_grapheme() {
    use crabrave::handlers::communities::Reaction;

    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(
            "/communities/rust-community/posts/123456789/reactions",
        ))
        .and(body_json(serde_json::json!({
            "grapheme": "👍"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .add_reaction("123456789", Reaction::grapheme("👍"))
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_add_reaction_celebration_banner() {
    use crabrave::handlers::communities::Reaction;

    let mock_server = MockServer::start().await;

    // On a reaction's first use, the API responds with a timeline object
    // containing a celebration banner instead of an empty body.
    Mock::given(method("PUT"))
        .and(path(
            "/communities/rust-community/posts/123456789/reactions",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "timeline": {
                    "elements": []
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .add_reaction("123456789", Reaction::slug(":heart:"))
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_remove_reaction() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/communities/rust-community/posts/123456789/reactions/735345404083896320",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .remove_reaction("123456789", "735345404083896320")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_remove_member_spam() {
    use crabrave::handlers::communities::RemovalReason;

    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/members/bad-blog"))
        .and(body_json(serde_json::json!({
            "reason": "spam"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .remove_member("bad-blog", RemovalReason::Spam)
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_remove_member_other_with_note() {
    use crabrave::handlers::communities::RemovalReason;

    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/members/bad-blog"))
        .and(body_json(serde_json::json!({
            "reason": "other",
            "note": "repeated harassment"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .remove_member(
            "bad-blog",
            RemovalReason::Other {
                note: "repeated harassment".into(),
            },
        )
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_ban_member() {
    use crabrave::handlers::communities::RemovalReason;

    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/members/bad-blog"))
        .and(body_json(serde_json::json!({
            "reason": "spam",
            "ban_user": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .ban_member("bad-blog", RemovalReason::Spam)
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_change_role() {
    use crabrave::handlers::communities::MemberRole;

    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/members/trusted-blog"))
        .and(body_json(serde_json::json!({
            "role": "moderator"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .change_role("trusted-blog", MemberRole::Moderator)
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

// =============================================================================
// User endpoint tests
// =============================================================================

#[tokio::test]
async fn test_user_dashboard() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_dashboard_npf.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/dashboard"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().dashboard().limit(10).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.posts.len(), 3);
    assert_eq!(response.posts[0].blog_name, "example-blog-015");
    assert_eq!(response.posts[1].blog_name, "example-blog-074");
}

#[tokio::test]
async fn test_user_likes() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_likes.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/likes"))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().likes().limit(5).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.liked_count, 111534);
    assert_eq!(response.liked_posts.len(), 3);
    assert_eq!(response.liked_posts[0].id, "823667562844930048");
}

#[tokio::test]
async fn test_user_follow() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/follow"))
        .and(body_json(serde_json::json!({
            "url": "staff"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "blog": {
                    "name": "staff",
                    "title": "Staff",
                    "description": "Tumblr Staff Blog",
                    "url": "https://staff.tumblr.com/",
                    "uuid": "t:staff123",
                    "updated": 1234567890,
                    "posts": 5000,
                    "is_nsfw": false,
                    "is_adult": false
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().follow("staff").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.blog.is_some());
    assert_eq!(response.blog.unwrap().name, "staff");
}

#[tokio::test]
async fn test_user_unfollow() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/unfollow"))
        .and(body_json(serde_json::json!({
            "url": "someuser"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "blog": {
                    "name": "someuser",
                    "title": "Some User",
                    "description": "Just a user",
                    "url": "https://someuser.tumblr.com/",
                    "uuid": "t:user456",
                    "updated": 1234567890,
                    "posts": 100,
                    "is_nsfw": false,
                    "is_adult": false
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().unfollow("someuser").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.blog.is_some());
    assert_eq!(response.blog.unwrap().name, "someuser");
}

// =============================================================================
// Edge case tests
// =============================================================================

#[tokio::test]
async fn test_empty_posts_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [],
                "total_posts": 0
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).posts().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.posts.is_empty());
    assert_eq!(response.total_posts, 0);
}

#[tokio::test]
async fn test_empty_queue_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/queue", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).queue().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.posts.is_empty());
}

#[tokio::test]
async fn test_empty_dashboard_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/dashboard"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().dashboard().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.posts.is_empty());
}

#[tokio::test]
async fn test_unauthorized_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/info"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().info().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_forbidden_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/queue", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "meta": {
                "status": 403,
                "msg": "Forbidden"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).queue().send().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 403);
            assert_eq!(message, "Forbidden");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_post_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/999999", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).post("999999").get().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// Pages endpoint tests
// =============================================================================

#[tokio::test]
async fn test_blog_pages() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "pages": [
                    {
                        "title": "About Me",
                        "body": "<h1>About</h1><p>Welcome to my blog!</p>",
                        "url": format!("https://{}.tumblr.com/about", TEST_BLOG_NAME),
                        "updated": 1234567890
                    },
                    {
                        "title": "Contact",
                        "body": "<h1>Contact</h1><p>Email me at test@example.com</p>",
                        "url": format!("https://{}.tumblr.com/contact", TEST_BLOG_NAME),
                        "updated": 1234567891
                    },
                    {
                        "title": "FAQ",
                        "body": "<h1>FAQ</h1><p>Frequently asked questions</p>",
                        "url": format!("https://{}.tumblr.com/faq", TEST_BLOG_NAME),
                        "updated": 1234567892
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).pages().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let pages = result.unwrap();
    assert_eq!(pages.pages.len(), 3);
    assert_eq!(pages.pages[0].title, "About Me");
    assert_eq!(
        pages.pages[0].url,
        format!("https://{}.tumblr.com/about", TEST_BLOG_NAME)
    );
    assert_eq!(pages.pages[0].updated, 1234567890);
    assert!(pages.pages[0].body.contains("Welcome to my blog!"));
    assert_eq!(pages.pages[1].title, "Contact");
    assert_eq!(pages.pages[2].title, "FAQ");
}

#[tokio::test]
async fn test_blog_pages_with_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages", TEST_BLOG_NAME)))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "pages": [
                    {
                        "title": "About Me",
                        "body": "<h1>About</h1><p>Welcome!</p>",
                        "url": format!("https://{}.tumblr.com/about", TEST_BLOG_NAME),
                        "updated": 1234567890
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).pages().limit(5).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let pages = result.unwrap();
    assert_eq!(pages.pages.len(), 1);
    assert_eq!(pages.pages[0].title, "About Me");
}

#[tokio::test]
async fn test_blog_pages_with_offset() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages", TEST_BLOG_NAME)))
        .and(query_param("offset", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "pages": [
                    {
                        "title": "Third Page",
                        "body": "<p>This is the third page</p>",
                        "url": format!("https://{}.tumblr.com/third", TEST_BLOG_NAME),
                        "updated": 1234567892
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).pages().offset(2).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let pages = result.unwrap();
    assert_eq!(pages.pages.len(), 1);
    assert_eq!(pages.pages[0].title, "Third Page");
}

#[tokio::test]
async fn test_blog_pages_with_limit_and_offset() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages", TEST_BLOG_NAME)))
        .and(query_param("limit", "10"))
        .and(query_param("offset", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "pages": [
                    {
                        "title": "Sixth Page",
                        "body": "<p>Page content</p>",
                        "url": format!("https://{}.tumblr.com/sixth", TEST_BLOG_NAME),
                        "updated": 1234567896
                    },
                    {
                        "title": "Seventh Page",
                        "body": "<p>More content</p>",
                        "url": format!("https://{}.tumblr.com/seventh", TEST_BLOG_NAME),
                        "updated": 1234567897
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .pages()
        .limit(10)
        .offset(5)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let pages = result.unwrap();
    assert_eq!(pages.pages.len(), 2);
    assert_eq!(pages.pages[0].title, "Sixth Page");
    assert_eq!(pages.pages[1].title, "Seventh Page");
}

#[tokio::test]
async fn test_blog_pages_empty() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_pages.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/pages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").pages().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let pages = result.unwrap();
    assert!(pages.pages.is_empty());
}

#[tokio::test]
async fn test_blog_pages_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).pages().send().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_blog_pages_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/nonexistent-blog/pages"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("nonexistent-blog").pages().send().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// Single page endpoint tests
// =============================================================================

#[tokio::test]
async fn test_blog_page_by_name() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages/about", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "page": {
                    "title": "About Me",
                    "body": "<h1>About</h1><p>Welcome to my blog! This is all about me.</p>",
                    "url": format!("https://{}.tumblr.com/about", TEST_BLOG_NAME),
                    "updated": 1234567890
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).page("about").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.page.title, "About Me");
    assert_eq!(
        response.page.url,
        format!("https://{}.tumblr.com/about", TEST_BLOG_NAME)
    );
    assert_eq!(response.page.updated, 1234567890);
    assert!(response.page.body.contains("Welcome to my blog!"));
}

#[tokio::test]
async fn test_blog_page_contact() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages/contact", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "page": {
                    "title": "Contact",
                    "body": "<h1>Contact</h1><p>Email: test@example.com</p>",
                    "url": format!("https://{}.tumblr.com/contact", TEST_BLOG_NAME),
                    "updated": 1234567891
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).page("contact").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.page.title, "Contact");
    assert!(response.page.body.contains("test@example.com"));
}

#[tokio::test]
async fn test_blog_page_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages/nonexistent", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).page("nonexistent").await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_blog_page_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages/private", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).page("private").await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// Post mute endpoint tests
// =============================================================================

#[tokio::test]
async fn test_post_mute() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/123456789/mute",
            TEST_BLOG_NAME
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "muted": true,
                "mute_end_timestamp": 0
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).post("123456789").mute().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.muted);
    assert_eq!(response.mute_end_timestamp, 0);
}

#[tokio::test]
async fn test_post_mute_with_expiration() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/987654321/mute",
            TEST_BLOG_NAME
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "muted": true,
                "mute_end_timestamp": 1735689600
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).post("987654321").mute().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert!(response.muted);
    assert_eq!(response.mute_end_timestamp, 1735689600);
}

#[tokio::test]
async fn test_post_mute_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/nonexistent/mute",
            TEST_BLOG_NAME
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .post("nonexistent")
        .mute()
        .await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_post_mute_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/123456789/mute",
            TEST_BLOG_NAME
        )))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).post("123456789").mute().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_post_mute_forbidden() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/blog/{}/posts/123456789/mute",
            TEST_BLOG_NAME
        )))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "meta": {
                "status": 403,
                "msg": "Forbidden"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).post("123456789").mute().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 403);
            assert_eq!(message, "Forbidden");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// User limits endpoint tests
// =============================================================================

#[tokio::test]
async fn test_user_limits() {
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
    let limits = result.unwrap();

    // Check posts limit
    let posts = limits.user.posts.expect("posts limit should be present");
    assert_eq!(posts.limit, 500);
    assert_eq!(posts.remaining, 484);
    assert_eq!(posts.reset_at, 1785556800);

    // Check photos limit
    let photos = limits.user.photos.expect("photos limit should be present");
    assert_eq!(photos.limit, 500);
    assert_eq!(photos.remaining, 497);

    // Check videos limit
    let videos = limits.user.videos.expect("videos limit should be present");
    assert_eq!(videos.limit, 40);
    assert_eq!(videos.remaining, 40);

    // Check video_seconds limit
    let video_seconds = limits
        .user
        .video_seconds
        .expect("video_seconds limit should be present");
    assert_eq!(video_seconds.limit, 7200);
    assert_eq!(video_seconds.remaining, 7200);

    // Check follows limit
    let follows = limits
        .user
        .follows
        .expect("follows limit should be present");
    assert_eq!(follows.limit, 200);
    assert_eq!(follows.remaining, 198);
}

#[tokio::test]
async fn test_user_limits_partial() {
    let mock_server = MockServer::start().await;

    // Test that partial limit data is handled correctly
    Mock::given(method("GET"))
        .and(path("/user/limits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "user": {
                    "posts": {
                        "description": "posts",
                        "limit": 250,
                        "remaining": 100,
                        "reset_at": 1735776000
                    }
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().limits().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let limits = result.unwrap();

    // Only posts should be present
    assert!(limits.user.posts.is_some());
    assert!(limits.user.photos.is_none());
    assert!(limits.user.videos.is_none());
    assert!(limits.user.video_seconds.is_none());
    assert!(limits.user.follows.is_none());
}

#[tokio::test]
async fn test_user_limits_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/limits"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().limits().await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// User like/unlike endpoint tests
// =============================================================================

#[tokio::test]
async fn test_user_like() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/like"))
        .and(body_json(serde_json::json!({
            "id": 123456789,
            "reblog_key": "aB1cD2eF3"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().like(123456789, "aB1cD2eF3").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_like_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/like"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().like(999999999, "invalid_key").await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_user_unlike() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/unlike"))
        .and(body_json(serde_json::json!({
            "id": 123456789,
            "reblog_key": "aB1cD2eF3"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().unlike(123456789, "aB1cD2eF3").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_unlike_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/unlike"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().unlike(999999999, "invalid_key").await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 404);
            assert_eq!(message, "Not Found");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_user_like_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/like"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "meta": {
                "status": 401,
                "msg": "Unauthorized"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().like(123456789, "aB1cD2eF3").await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 401);
            assert_eq!(message, "Unauthorized");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// User filtered tags endpoint tests
// =============================================================================

#[tokio::test]
async fn test_user_filtered_tags_get() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_filtered_tags.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/filtered_tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().filtered_tags().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.filtered_tags.len(), 1);
    assert!(
        response
            .filtered_tags
            .contains(&"crabrave-test-filter".to_string())
    );
}

#[tokio::test]
async fn test_user_filtered_tags_get_with_pagination() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/filtered_tags"))
        .and(query_param("limit", "10"))
        .and(query_param("offset", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "filtered_tags": ["tag6", "tag7"]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .filtered_tags()
        .limit(10)
        .offset(5)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.filtered_tags.len(), 2);
}

#[tokio::test]
async fn test_user_filtered_tags_add() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/filtered_tags"))
        .and(body_json(serde_json::json!({
            "filtered_tags": ["spoilers", "nsfw"]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .add_filtered_tags(vec!["spoilers", "nsfw"])
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_filtered_tags_remove() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/user/filtered_tags/spoilers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().remove_filtered_tag("spoilers").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_filtered_tags_remove_with_special_chars() {
    let mock_server = MockServer::start().await;

    // Tag with special characters should be URL-encoded
    Mock::given(method("DELETE"))
        .and(path("/user/filtered_tags/tag%20with%20spaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().remove_filtered_tag("tag with spaces").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_filtered_tags_add_limit_exceeded() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/filtered_tags"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "meta": {
                "status": 403,
                "msg": "Forbidden"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().add_filtered_tags(vec!["new-tag"]).await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 403);
            assert_eq!(message, "Forbidden");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

// =============================================================================
// User filtered content endpoint tests
// =============================================================================

#[tokio::test]
async fn test_user_filtered_content_get() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/user_filtered_content.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/user/filtered_content"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().filtered_content().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.filtered_content.len(), 1);
    assert!(
        response
            .filtered_content
            .contains(&"crabrave test filtered phrase".to_string())
    );
}

#[tokio::test]
async fn test_user_filtered_content_get_with_pagination() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/filtered_content"))
        .and(query_param("limit", "15"))
        .and(query_param("offset", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "filtered_content": ["content11", "content12"]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .filtered_content()
        .limit(15)
        .offset(10)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.filtered_content.len(), 2);
}

#[tokio::test]
async fn test_user_filtered_content_add() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/filtered_content"))
        .and(body_json(serde_json::json!({
            "filtered_content": ["spam", "unwanted content"]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .add_filtered_content(vec!["spam", "unwanted content"])
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_filtered_content_remove() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/user/filtered_content"))
        .and(query_param("filtered_content", "spam"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().remove_filtered_content("spam").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_user_filtered_content_add_limit_exceeded() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/filtered_content"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "meta": {
                "status": 403,
                "msg": "Forbidden"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .users()
        .add_filtered_content(vec!["new content"])
        .await;

    assert!(result.is_err());
    match result {
        Err(CrabError::Api {
            status, message, ..
        }) => {
            assert_eq!(status, 403);
            assert_eq!(message, "Forbidden");
        }
        Err(e) => panic!("Expected ApiError, got: {:?}", e),
        Ok(_) => panic!("Expected error, got success"),
    }
}

#[tokio::test]
async fn test_npf_posts() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_posts_npf.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    // NPF is now always used by default
    let result = client.blogs("crabrave-demo").posts().send().await;

    assert!(result.is_ok(), "failed with: {:?}", result);
    let posts = result.unwrap();
    assert_eq!(posts.total_posts, 22);
    assert_eq!(posts.posts.len(), 8);
    assert!(posts.posts.iter().all(|p| p.post_type == "blocks"));
    assert_eq!(posts.posts.get(2).unwrap().trail.len(), 1);
}

#[tokio::test]
async fn test_blog_posts_legacy() {
    let mock_server = MockServer::start().await;
    let fixture = include_str!("fixtures/live/blog_posts_legacy.json");
    let body: serde_json::Value = serde_json::from_str(fixture).expect("valid fixture JSON");

    // Same 8 posts as blog_posts_npf.json, captured without `?npf=true`:
    // content arrives as legacy HTML (in `body`, not NPF `content`), so
    // Post.content deserializes to empty via deserialize_content_blocks.
    Mock::given(method("GET"))
        .and(path("/blog/crabrave-demo/posts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("crabrave-demo").posts().send().await;

    assert!(result.is_ok(), "failed with: {:?}", result);
    let posts = result.unwrap();
    assert_eq!(posts.total_posts, 22);
    assert_eq!(posts.posts.len(), 8);
    assert_eq!(posts.posts[0].id, "819598039806263296");
    assert_eq!(posts.posts[0].post_type, "text");
    assert!(posts.posts[0].content.is_empty());
}

#[tokio::test]
async fn test_posts_with_poll_content_blocks() {
    let mock_server = MockServer::start().await;
    let mock_response = include_str!("./fixtures/content_or_string_enum.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client.blogs(TEST_BLOG_NAME).posts().send().await;

    assert!(result.is_ok(), "failed with: {:?}", result);
    let posts = result.unwrap();
    assert_eq!(posts.posts.len(), 20);

    // Every post in this fixture has a poll content block
    let first_post = &posts.posts[0];
    let poll_block = first_post
        .content
        .iter()
        .find(|b| matches!(b, crabrave::npf::ContentBlock::Poll { .. }));
    assert!(poll_block.is_some(), "expected a poll block in content");

    match poll_block.unwrap() {
        crabrave::npf::ContentBlock::Poll {
            question, answers, ..
        } => {
            assert_eq!(question, "Where do you usually exercise?");
            assert_eq!(answers.len(), 12);
        }
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn invalid_sequence_bug() {
    let mock_server = MockServer::start().await;
    let mock_response = include_str!("./fixtures/invalid_type_map_bug.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/12345")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    // NPF is now always used by default
    let result = client.blogs(TEST_BLOG_NAME).post("12345").get().await;

    assert!(result.is_ok(), "failed with: {:?}", result);

    let post = result.unwrap();

    assert_eq!(post.content.len(), 4);
}

#[tokio::test]
async fn missing_type_field_bug() {
    let mock_server = MockServer::start().await;
    let mock_response = include_str!("./fixtures/missing_type_field.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/12345")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    // NPF is now always used by default
    let result = client.blogs(TEST_BLOG_NAME).post("12345").get().await;

    assert!(result.is_ok(), "failed with: {:?}", result);

    let post = result.unwrap();

    assert_eq!(post.content.len(), 0);
    assert!(!post.trail.is_empty());
}

#[tokio::test]
async fn invalid_id_bug() {
    let mock_server = MockServer::start().await;
    let mock_response = include_str!("./fixtures/invalid_id.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/12345")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    // NPF is now always used by default
    let result = client.blogs(TEST_BLOG_NAME).post("12345").get().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn audio_post_bug() {
    let mock_server = MockServer::start().await;
    let mock_response = include_str!("./fixtures/audio_error.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts/12345")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client.blogs(TEST_BLOG_NAME).post("12345").get().await;

    assert!(result.is_ok(), "failed with {:?}", result);
}

// =============================================================================
// Hardening tests: non-envelope errors, error detail, avatar formats,
// invitations, legacy content, and lenient empty-shape parsing
// =============================================================================

#[tokio::test]
async fn test_gateway_error_non_envelope_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/info")))
        .respond_with(
            ResponseTemplate::new(502)
                .set_body_string("<html>Bad Gateway</html>")
                .insert_header("content-type", "text/html"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).info().await;

    match result {
        Err(CrabError::Api { status, .. }) => assert_eq!(status, 502),
        other => panic!("Expected Api error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_envelope_error_detail() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/info")))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "meta": {
                "status": 400,
                "msg": "Bad Request"
            },
            "response": {},
            "errors": [
                {
                    "title": "Bad Request",
                    "code": 8001,
                    "detail": "NPF JSON parameter is invalid",
                    "subcode": "8001.1"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).info().await;

    match result {
        Err(CrabError::Api { status, errors, .. }) => {
            assert_eq!(status, 400);
            assert_eq!(errors.len(), 1);
            assert_eq!(
                errors[0].detail.as_deref(),
                Some("NPF JSON parameter is invalid")
            );
        }
        other => panic!("Expected Api error, got: {:?}", other),
    }

    // Re-issue to check the error's Display separately, since `result` was consumed above.
    let err = client
        .blogs(TEST_BLOG_NAME)
        .info()
        .await
        .expect_err("expected error");
    let message = err.to_string();
    assert!(
        message.contains("NPF JSON parameter is invalid"),
        "message: {message}"
    );
    assert!(message.contains("8001.1"), "message: {message}");
}

#[tokio::test]
async fn test_avatar_endpoint_jpeg() {
    let avatar = include_bytes!("./fixtures/demo_avatar.png");
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/demo/avatar/64"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(avatar)
                .insert_header("Content-Type", "image/jpeg"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs("demo")
        .avatar(Some(64))
        .await
        .expect("Callout failed");

    assert!(matches!(result, AvatarResponse::ImageData(_)));
}

#[tokio::test]
async fn test_community_cancel_invitation() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/invitations/friend-blog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .cancel_invitation("friend-blog")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_cancel_invitation_do_not_invite() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/invitations/friend-blog"))
        .and(query_param("do_not_invite", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .cancel_invitation_do_not_invite("friend-blog")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_community_regenerate_invite_url() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/communities/rust-community/invite_hash"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "invite_link": "https://www.tumblr.com/join/AbCd123"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .regenerate_invite_url()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(
        result.unwrap().invite_link,
        "https://www.tumblr.com/join/AbCd123"
    );
}

#[tokio::test]
async fn test_community_invitation_status_pending() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/invitations/friend-blog"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "sender": { "name": "admin-blog" }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .invitation_status("friend-blog")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let status = result.unwrap().expect("expected a pending invitation");
    assert_eq!(status.sender.name, "admin-blog");
}

#[tokio::test]
async fn test_community_invitation_status_none() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/invitations/friend-blog"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "meta": {
                "status": 404,
                "msg": "Not Found"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .invitation_status("friend-blog")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_community_invitations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/invitations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "invitations": [
                    {
                        "sender": { "name": "admin-blog" },
                        "recipient": { "name": "friend-blog" },
                        "sent_on": 1234567890
                    }
                ],
                "_links": {
                    "next": {
                        "href": "/v2/communities/rust-community/invitations?before=1234567890",
                        "method": "GET",
                        "query_params": {
                            "before": "1234567890"
                        }
                    }
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .invitations()
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    let invitations = response.invitations;
    assert_eq!(invitations.len(), 1);
    assert_eq!(invitations[0].sender.name, "admin-blog");
    assert_eq!(invitations[0].recipient.name, "friend-blog");
    assert_eq!(invitations[0].sent_on, 1234567890);
    assert_eq!(
        response.links.unwrap().next.unwrap().query_params.before,
        Some("1234567890".to_string())
    );
}

#[tokio::test]
async fn test_community_invitations_before() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/invitations"))
        .and(query_param("before", "1234567890"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "invitations": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .invitations()
        .before(1234567890)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert!(result.unwrap().invitations.is_empty());
}

// The moderation endpoints appear nowhere in Tumblr's published API docs, so
// this fixture shape is unverified against the live API.
#[tokio::test]
async fn test_community_moderated_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/moderation/posts/123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "post": {
                    "id_string": "123456",
                    "blog_name": "member-blog",
                    "post_url": "https://member-blog.tumblr.com/post/123456",
                    "type": "blocks",
                    "timestamp": 1234567890,
                    "reblog_key": "rk",
                    "content": [],
                    "layout": [],
                    "trail": []
                },
                "moderation": {
                    "moderated_at": 1234567999,
                    "reason": "spam"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .moderated_post("123456")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    assert_eq!(result.unwrap().post.id, "123456");
}

// The moderation endpoints appear nowhere in Tumblr's published API docs, so
// this fixture shape is unverified against the live API.
#[tokio::test]
async fn test_community_moderate_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/communities/rust-community/moderation/posts/123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .moderate_post("123456")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

// The moderation endpoints appear nowhere in Tumblr's published API docs, so
// this fixture shape is unverified against the live API.
#[tokio::test]
async fn test_community_restore_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/moderation/posts/123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": []
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .restore_post("123456")
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
}

#[tokio::test]
async fn test_post_with_legacy_html_content() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/posts")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id_string": "123456",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/123456", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567890,
                        "tags": [],
                        "note_count": 0,
                        "content": "<p>legacy html</p>"
                    }
                ],
                "total_posts": 1
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).posts().send().await;

    assert!(result.is_ok(), "failed with: {:?}", result);
    let posts = result.unwrap();
    assert_eq!(posts.posts.len(), 1);
    assert!(posts.posts[0].content.is_empty());
}

#[tokio::test]
async fn test_blocks_entry_missing_tumblrmart_accessories() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "blocked_tumblelogs": [
                    {
                        "name": "some-blog",
                        "title": "Some Blog",
                        "description": "",
                        "url": "https://some-blog.tumblr.com/",
                        "uuid": "t:abc123",
                        "updated": 0,
                        "blocked_timestamp": 0,
                        "can_show_badges": true
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).blocks().send().await;

    assert!(result.is_ok(), "failed with: {:?}", result);
    let blocks = result.unwrap();
    assert_eq!(blocks.blocked_tumblelogs.len(), 1);
    assert!(
        blocks.blocked_tumblelogs[0]
            .tumblrmart_accessories
            .is_none()
    );
}

#[tokio::test]
async fn test_empty_response_accepts_object_shape() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/user/filtered_tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {}
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().add_filtered_tags(vec!["spoilers"]).await;

    assert!(result.is_ok(), "failed with: {:?}", result);
}
