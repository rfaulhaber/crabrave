//! Mock server tests using wiremock
//!
//! These tests use a mock HTTP server to test the full request/response cycle
//! without requiring actual Tumblr API credentials.

use crabrave::handlers::blog::AvatarResponse;
use crabrave::{CrabError, Crabrave};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_BLOG_NAME: &'static str = "crabrave";

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

    // Mock response matching Tumblr API format
    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
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
                    "uuid": "t:123456",
                    "updated": 1234567890,
                    "posts": 1000,
                    "is_nsfw": false,
                    "is_adult": false
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs("staff").info().await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.blog.name, "staff");
    assert_eq!(info.blog.title, "Staff");
    assert_eq!(info.blog.posts, 1000);
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
        .respond_with(
            ResponseTemplate::new(429).set_body_json(serde_json::json!({
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

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/queue", TEST_BLOG_NAME)))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id": 123456,
                        "id_string": "123456",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/123456", TEST_BLOG_NAME),
                        "type": "text",
                        "timestamp": 1234567890,
                        "scheduled_publish_time": 1234567900,
                        "tags": ["queued"],
                        "note_count": 0
                    },
                    {
                        "id": 123457,
                        "id_string": "123457",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/123457", TEST_BLOG_NAME),
                        "type": "photo",
                        "timestamp": 1234567891,
                        "scheduled_publish_time": 1234568000,
                        "tags": ["photo", "queued"],
                        "note_count": 0
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).queue().limit(5).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let queue = result.unwrap();
    assert_eq!(queue.posts.len(), 2);
    assert_eq!(queue.posts[0].id, "123456");
    assert_eq!(queue.posts[1].id, "123457");
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

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/draft", TEST_BLOG_NAME)))
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
                        "tags": ["draft", "wip"],
                        "note_count": 0
                    },
                    {
                        "id": 222222,
                        "id_string": "222222",
                        "blog_name": TEST_BLOG_NAME,
                        "post_url": format!("https://{}.tumblr.com/post/222222", TEST_BLOG_NAME),
                        "type": "photo",
                        "timestamp": 1234567891,
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
    let result = client.blogs(TEST_BLOG_NAME).drafts().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let drafts = result.unwrap();
    assert_eq!(drafts.posts.len(), 2);
    assert_eq!(drafts.posts[0].id, "111111");
    assert_eq!(drafts.posts[1].id, "222222");
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

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/submission", TEST_BLOG_NAME)))
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
    let result = client.blogs(TEST_BLOG_NAME).submissions().send().await;

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
                        "id": "notif-001",
                        "type": "like",
                        "timestamp": 1234567890,
                        "unread": true,
                        "target_post_id": "123456",
                        "from_tumblelog_name": "friendly-blog"
                    },
                    {
                        "id": "notif-002",
                        "type": "reblog_naked",
                        "timestamp": 1234567880,
                        "unread": false,
                        "target_post_id": "123456",
                        "from_tumblelog_name": "reblogger-blog"
                    },
                    {
                        "id": "notif-003",
                        "type": "follow",
                        "timestamp": 1234567870,
                        "unread": true,
                        "from_tumblelog_name": "new-follower"
                    }
                ],
                "_links": {
                    "next": {
                        "href": "/blog/crabrave/notifications?before=1234567870"
                    }
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).notifications().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 3);
    assert_eq!(notifs.notifications[0].id, "notif-001");
    assert_eq!(notifs.notifications[0].notification_type, "like");
    assert!(notifs.notifications[0].unread);
    assert_eq!(
        notifs.notifications[0].from_tumblelog_name,
        Some("friendly-blog".to_string())
    );
    assert_eq!(notifs.notifications[1].notification_type, "reblog_naked");
    assert_eq!(notifs.notifications[2].notification_type, "follow");

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
        .and(query_param("types", "like,follow"))
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
        .and(query_param("omit_post_ids", "111111,222222"))
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

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [
                    {
                        "type": "reblog",
                        "timestamp": 1234567890,
                        "blog_name": "reblogger",
                        "blog_uuid": "t:abc123",
                        "blog_url": "https://reblogger.tumblr.com",
                        "followed": false,
                        "avatar_shape": "square",
                        "post_id": "987654321",
                        "reblog_parent_blog_name": TEST_BLOG_NAME
                    },
                    {
                        "type": "like",
                        "timestamp": 1234567880,
                        "blog_name": "liker",
                        "blog_uuid": "t:def456",
                        "blog_url": "https://liker.tumblr.com",
                        "followed": true,
                        "avatar_shape": "circle"
                    },
                    {
                        "type": "reply",
                        "timestamp": 1234567870,
                        "blog_name": "replier",
                        "blog_uuid": "t:ghi789",
                        "reply_text": "Great post!",
                        "followed": false
                    }
                ],
                "total_notes": 150,
                "_links": {
                    "next": {
                        "query_params": {
                            "id": "123456789",
                            "mode": "all",
                            "before_timestamp": 1234567870
                        }
                    }
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).notes("123456789").send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 3);
    assert_eq!(notes.total_notes, 150);
    assert_eq!(notes.notes[0].note_type, "reblog");
    assert_eq!(notes.notes[0].blog_name, "reblogger");
    assert_eq!(notes.notes[1].note_type, "like");
    assert!(notes.notes[1].followed);
    assert_eq!(notes.notes[2].note_type, "reply");
    assert_eq!(notes.notes[2].reply_text, Some("Great post!".to_string()));

    // Check pagination links
    assert!(notes.links.is_some());
    let links = notes.links.unwrap();
    assert!(links.next.is_some());
    assert_eq!(
        links.next.unwrap().query_params.before_timestamp,
        1234567870
    );
}

#[tokio::test]
async fn test_blog_notes_likes_mode() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .and(query_param("mode", "likes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [
                    {
                        "type": "like",
                        "timestamp": 1234567890,
                        "blog_name": "liker1",
                        "blog_uuid": "t:aaa111",
                        "followed": false
                    },
                    {
                        "type": "like",
                        "timestamp": 1234567880,
                        "blog_name": "liker2",
                        "blog_uuid": "t:bbb222",
                        "followed": true
                    }
                ],
                "total_notes": 50
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .mode(NoteMode::Likes)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 2);
    assert!(notes.notes.iter().all(|n| n.note_type == "like"));
}

#[tokio::test]
async fn test_blog_notes_conversation_mode() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .and(query_param("mode", "conversation"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [
                    {
                        "type": "reply",
                        "timestamp": 1234567890000_i64,
                        "blog_name": "replier1",
                        "reply_text": "I agree!",
                        "followed": false
                    },
                    {
                        "type": "reblog",
                        "timestamp": 1234567880000_i64,
                        "blog_name": "reblogger",
                        "added_text": "Adding my thoughts...",
                        "followed": true
                    }
                ],
                "rollup_notes": [
                    {
                        "type": "like",
                        "timestamp": 1234567870,
                        "blog_name": "liker",
                        "followed": false
                    }
                ],
                "total_notes": 100,
                "total_likes": 75,
                "total_reblogs": 25
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .mode(NoteMode::Conversation)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 2);
    assert_eq!(notes.rollup_notes.len(), 1);
    assert_eq!(notes.total_likes, Some(75));
    assert_eq!(notes.total_reblogs, Some(25));
    assert_eq!(notes.notes[0].reply_text, Some("I agree!".to_string()));
    assert_eq!(
        notes.notes[1].added_text,
        Some("Adding my thoughts...".to_string())
    );
}

#[tokio::test]
async fn test_blog_notes_reblogs_with_tags_mode() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .and(query_param("mode", "reblogs_with_tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [
                    {
                        "type": "reblog",
                        "timestamp": 1234567890,
                        "blog_name": "tagger1",
                        "blog_uuid": "t:tag111",
                        "followed": false,
                        "tags": ["cool", "nice", "reblogged"]
                    },
                    {
                        "type": "reblog",
                        "timestamp": 1234567880,
                        "blog_name": "tagger2",
                        "blog_uuid": "t:tag222",
                        "followed": true,
                        "tags": ["art", "favorite"]
                    }
                ],
                "total_notes": 30
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .mode(NoteMode::ReblogsWithTags)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 2);
    assert!(notes.notes.iter().all(|n| n.note_type == "reblog"));
    assert_eq!(notes.notes[0].tags, vec!["cool", "nice", "reblogged"]);
    assert_eq!(notes.notes[1].tags, vec!["art", "favorite"]);
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

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/notes", TEST_BLOG_NAME)))
        .and(query_param("id", "123456789"))
        .and(query_param("mode", "rollup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "notes": [
                    {
                        "type": "like",
                        "timestamp": 1234567890,
                        "blog_name": "liker",
                        "followed": false
                    },
                    {
                        "type": "reblog",
                        "timestamp": 1234567880,
                        "blog_name": "reblogger",
                        "followed": true
                    }
                ],
                "total_notes": 80
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    use crabrave::handlers::blog::NoteMode;
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .mode(NoteMode::Rollup)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notes = result.unwrap();
    assert_eq!(notes.notes.len(), 2);
    // Rollup mode returns only likes and reblogs
    assert!(
        notes
            .notes
            .iter()
            .all(|n| n.note_type == "like" || n.note_type == "reblog")
    );
}

#[tokio::test]
async fn test_user_info() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "user": {
                    "name": "testuser",
                    "likes": 42,
                    "following": 100,
                    "blogs": []
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().info().await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.user.name, "testuser");
    assert_eq!(info.user.likes, 42);
}

#[tokio::test]
async fn test_tagged_posts() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/tagged"))
        .and(query_param("tag", "rust"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": [
                    {
                        "id": 789012,
                        "id_string": "789012",
                        "blog_name": "rustblog",
                        "post_url": "https://rustblog.tumblr.com/post/789012",
                        "type": "text",
                        "timestamp": 1234567890,
                        "tags": ["rust", "programming"],
                        "note_count": 50
                    }
                ]
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.tagged("rust").limit(10).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let tagged = result.unwrap();
    assert_eq!(tagged.posts.len(), 1);
    assert!(tagged.posts[0].tags.contains(&"rust".to_string()));
}

#[tokio::test]
async fn test_delete_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/blog/myblog/post/delete"))
        .and(query_param("id", "123456"))
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
    let result = client.blogs("myblog").post("123456").delete().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_communities_timeline() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/timeline"))
        .and(query_param("limit", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "posts": [
                    {
                        "id_string": "111222",
                        "blog_name": "rustdev",
                        "post_url": "https://rustdev.tumblr.com/post/111222",
                        "type": "text",
                        "timestamp": 1234567890,
                        "tags": ["rust"],
                        "note_count": 25
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .communities("rust-community")
        .timeline()
        .limit(20)
        .send()
        .await;

    assert!(result.is_ok());
    let timeline = result.unwrap();
    assert_eq!(timeline.posts.len(), 1);
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

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "draft-post-456"
            }
        })))
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
    assert_eq!(response.id, "draft-post-456");
}

#[tokio::test]
async fn test_npf_post_creation_queued() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "meta": {
                "status": 201,
                "msg": "Created"
            },
            "response": {
                "id": "queued-post-789"
            }
        })))
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
    assert_eq!(response.id, "queued-post-789");
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
        Err(CrabError::Api { status, message }) => {
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
    let blocks_response = include_str!("./fixtures/get_blocks.json");

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/blocks")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(blocks_response)
                .insert_header("content-type", "application/json"),
        )
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

    assert_eq!(result.already_blocked, false);
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

    assert_eq!(result.already_blocked, false);
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

    let mock_response = include_str!("./fixtures/blog_likes.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/likes")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .likes()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.liked_count, 106883);
}

#[tokio::test]
async fn test_blog_following() {
    let mock_server = MockServer::start().await;

    let mock_response = include_str!("./fixtures/get_blog_following.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/following")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .blogs(TEST_BLOG_NAME)
        .following()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.total_blogs, 1190);
}

#[tokio::test]
async fn test_user_following() {
    let mock_server = MockServer::start().await;

    let mock_response = include_str!("./fixtures/get_user_following.json");

    Mock::given(method("GET"))
        .and(path(format!("/user/following")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;

    let result = client
        .users()
        .following()
        .send()
        .await
        .expect("Callout to get blog likes failed");

    assert_eq!(result.total_blogs, 1190);
}

#[tokio::test]
async fn test_blog_followers() {
    let mock_server = MockServer::start().await;

    let mock_response = include_str!("./fixtures/blog_followers.json");

    Mock::given(method("GET"))
        .and(path(format!("/blog/{TEST_BLOG_NAME}/followers")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(mock_response)
                .insert_header("content-type", "application/json"),
        )
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

    assert_eq!(result, true);
}

// =============================================================================
// Posts endpoint tests
// =============================================================================

#[tokio::test]
async fn test_get_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/posts/123456", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "object_type": "post",
                "type": "blocks",
                "id": 123456,
                "id_string": "123456",
                "tumblelog_uuid": "t:abc123",
                "blog_name": TEST_BLOG_NAME,
                "post_url": format!("https://{}.tumblr.com/post/123456", TEST_BLOG_NAME),
                "timestamp": 1234567890,
                "tags": ["test", "example"],
                "reblog_key": "abc123xyz",
                "interactability_reblog": "everyone",
                "content": [
                    {
                        "type": "text",
                        "text": "This is the post body"
                    }
                ],
                "layout": [],
                "trail": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).post("123456").get().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let post_response = result.unwrap();
    assert_eq!(post_response.id, "123456");
    assert_eq!(post_response.blog_name, TEST_BLOG_NAME);
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
        Err(CrabError::Api { status, message }) => {
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

    // NPF reblog uses POST to /blog/{blog}/posts with parent_tumblelog_uuid
    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/posts", TEST_BLOG_NAME)))
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
async fn test_community_join() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/communities/rust-community/members"))
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
    let result = client.communities("rust-community").join().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.success, Some(true));
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
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/communities/rust-community/members"))
        .and(query_param("limit", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "total_members": 150,
                "members": [
                    {
                        "blog": {
                            "name": "rustdev",
                            "title": "Rust Developer",
                            "description": "Learning Rust",
                            "url": "https://rustdev.tumblr.com/",
                            "uuid": "t:abc123",
                            "updated": 1234567890,
                            "posts": 50,
                            "is_nsfw": false,
                            "is_adult": false
                        },
                        "role": "member"
                    },
                    {
                        "blog": {
                            "name": "crabfan",
                            "title": "Crab Fan",
                            "description": "I love crabs",
                            "url": "https://crabfan.tumblr.com/",
                            "uuid": "t:def456",
                            "updated": 1234567891,
                            "posts": 25,
                            "is_nsfw": false,
                            "is_adult": false
                        },
                        "role": "moderator"
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
        .limit(10)
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.total_members, 150);
    assert_eq!(response.members.len(), 2);
    assert_eq!(response.members[0].blog.name, "rustdev");
}

// =============================================================================
// User endpoint tests
// =============================================================================

#[tokio::test]
async fn test_user_dashboard() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/dashboard"))
        .and(query_param("limit", "10"))
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
                        "blog_name": "friend1",
                        "post_url": "https://friend1.tumblr.com/post/111111",
                        "type": "text",
                        "timestamp": 1234567890,
                        "tags": [],
                        "note_count": 10
                    },
                    {
                        "id": 222222,
                        "id_string": "222222",
                        "blog_name": "friend2",
                        "post_url": "https://friend2.tumblr.com/post/222222",
                        "type": "photo",
                        "timestamp": 1234567891,
                        "tags": ["photo"],
                        "note_count": 50
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().dashboard().limit(10).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.posts.len(), 2);
    assert_eq!(response.posts[0].blog_name, "friend1");
    assert_eq!(response.posts[1].blog_name, "friend2");
}

#[tokio::test]
async fn test_user_likes() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/user/likes"))
        .and(query_param("limit", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "liked_posts": [
                    {
                        "id": 333333,
                        "id_string": "333333",
                        "blog_name": "coolblog",
                        "post_url": "https://coolblog.tumblr.com/post/333333",
                        "type": "text",
                        "timestamp": 1234567890,
                        "tags": ["liked"],
                        "note_count": 100
                    }
                ],
                "liked_count": 500
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().likes().limit(5).send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.liked_count, 500);
    assert_eq!(response.liked_posts.len(), 1);
    assert_eq!(response.liked_posts[0].id, "333333");
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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

    Mock::given(method("GET"))
        .and(path(format!("/blog/{}/pages", TEST_BLOG_NAME)))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "pages": []
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.blogs(TEST_BLOG_NAME).pages().send().await;

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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
                        "remaining": 245,
                        "reset_at": 1735776000
                    },
                    "photos": {
                        "description": "photo uploads",
                        "limit": 250,
                        "remaining": 250,
                        "reset_at": 1735776000
                    },
                    "videos": {
                        "description": "video uploads",
                        "limit": 20,
                        "remaining": 18,
                        "reset_at": 1735776000
                    },
                    "video_seconds": {
                        "description": "video upload seconds",
                        "limit": 3600,
                        "remaining": 3540,
                        "reset_at": 1735776000
                    },
                    "follows": {
                        "description": "follows",
                        "limit": 200,
                        "remaining": 195,
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

    // Check posts limit
    let posts = limits.user.posts.expect("posts limit should be present");
    assert_eq!(posts.description, "posts");
    assert_eq!(posts.limit, 250);
    assert_eq!(posts.remaining, 245);
    assert_eq!(posts.reset_at, 1735776000);

    // Check photos limit
    let photos = limits.user.photos.expect("photos limit should be present");
    assert_eq!(photos.description, "photo uploads");
    assert_eq!(photos.limit, 250);
    assert_eq!(photos.remaining, 250);

    // Check videos limit
    let videos = limits.user.videos.expect("videos limit should be present");
    assert_eq!(videos.description, "video uploads");
    assert_eq!(videos.limit, 20);
    assert_eq!(videos.remaining, 18);

    // Check video_seconds limit
    let video_seconds = limits
        .user
        .video_seconds
        .expect("video_seconds limit should be present");
    assert_eq!(video_seconds.description, "video upload seconds");
    assert_eq!(video_seconds.limit, 3600);
    assert_eq!(video_seconds.remaining, 3540);

    // Check follows limit
    let follows = limits
        .user
        .follows
        .expect("follows limit should be present");
    assert_eq!(follows.description, "follows");
    assert_eq!(follows.limit, 200);
    assert_eq!(follows.remaining, 195);
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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
        Err(CrabError::Api { status, message }) => {
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

    Mock::given(method("GET"))
        .and(path("/user/filtered_tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "filtered_tags": ["spoilers", "nsfw", "politics"]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().filtered_tags().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.filtered_tags.len(), 3);
    assert!(response.filtered_tags.contains(&"spoilers".to_string()));
    assert!(response.filtered_tags.contains(&"nsfw".to_string()));
    assert!(response.filtered_tags.contains(&"politics".to_string()));
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
        Err(CrabError::Api { status, message }) => {
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

    Mock::given(method("GET"))
        .and(path("/user/filtered_content"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "filtered_content": ["spam", "annoying phrase", "blocked user"]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.users().filtered_content().send().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.filtered_content.len(), 3);
    assert!(response.filtered_content.contains(&"spam".to_string()));
    assert!(
        response
            .filtered_content
            .contains(&"annoying phrase".to_string())
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
        Err(CrabError::Api { status, message }) => {
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
    let mock_response = include_str!("./fixtures/posts_npf.json");

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

    // NPF is now always used by default
    let result = client.blogs(TEST_BLOG_NAME).posts().send().await;

    assert!(result.is_ok(), "failed with: {:?}", result);
    let posts = result.unwrap();
    assert!(!posts.posts.is_empty());
    assert_eq!(posts.posts.get(2).unwrap().trail.len(), 1);
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
    assert!(post.trail.len() > 0);
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
