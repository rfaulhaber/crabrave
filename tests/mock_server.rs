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
async fn test_rate_limit_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/blog/staff/info"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "60")
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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .queue()
        .limit(5)
        .send()
        .await;

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
        .and(path(format!("/blog/{}/posts/queue/reorder", TEST_BLOG_NAME)))
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
        .and(path(format!("/blog/{}/posts/queue/reorder", TEST_BLOG_NAME)))
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
        .and(path(format!("/blog/{}/posts/queue/shuffle", TEST_BLOG_NAME)))
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
        .shuffle_queue()
        .await;

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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .drafts()
        .send()
        .await;

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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .drafts()
        .send()
        .await;

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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .submissions()
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert_eq!(submissions.posts.len(), 2);
    assert_eq!(submissions.posts[0].id, "444444");
    assert_eq!(submissions.posts[0].post_author, Some("friendly-submitter".to_string()));
    assert_eq!(submissions.posts[0].is_submission, Some(true));
    assert_eq!(submissions.posts[0].state, Some("submission".to_string()));
    assert_eq!(submissions.posts[1].id, "555555");
    assert_eq!(submissions.posts[1].post_author, Some("photo-lover".to_string()));
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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .submissions()
        .send()
        .await;

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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .submissions()
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let submissions = result.unwrap();
    assert_eq!(submissions.posts.len(), 1);
    assert_eq!(submissions.posts[0].id, "888888");
    assert_eq!(submissions.posts[0].anonymous_name, Some("Anonymous Fan".to_string()));
    assert_eq!(submissions.posts[0].anonymous_email, Some("anon@example.com".to_string()));
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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 3);
    assert_eq!(notifs.notifications[0].id, "notif-001");
    assert_eq!(notifs.notifications[0].notification_type, "like");
    assert!(notifs.notifications[0].unread);
    assert_eq!(notifs.notifications[0].from_tumblelog_name, Some("friendly-blog".to_string()));
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
    assert!(notifs.notifications.iter().all(|n| n.notification_type == "like" || n.notification_type == "follow"));
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
    assert!(notifs.notifications.iter().all(|n| n.notification_type == "like"));
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
    assert_eq!(notifs.notifications[0].target_post_id, Some("333333".to_string()));
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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .send()
        .await;

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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notifications()
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let notifs = result.unwrap();
    assert_eq!(notifs.notifications.len(), 1);
    assert_eq!(notifs.notifications[0].notification_type, "ask");
    assert_eq!(notifs.notifications[0].summary, Some("What's your favorite color?".to_string()));
    assert_eq!(notifs.notifications[0].target_tumblelog_name, Some(TEST_BLOG_NAME.to_string()));
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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .send()
        .await;

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
    assert_eq!(links.next.unwrap().query_params.before_timestamp, 1234567870);
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
    assert_eq!(notes.notes[1].added_text, Some("Adding my thoughts...".to_string()));
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
    let result = client
        .blogs(TEST_BLOG_NAME)
        .notes("123456789")
        .send()
        .await;

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
    assert!(notes.notes.iter().all(|n| n.note_type == "like" || n.note_type == "reblog"));
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
    let result = client.posts().delete("myblog", "123456").await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_communities_timeline() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/community/rust-community/timeline"))
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
        .posts()
        .create("myblog")
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
async fn test_network_error() {
    // Create client with invalid URL to trigger network error
    let client = Crabrave::builder()
        .consumer_key("test")
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

    let result = client
        .blogs(TEST_BLOG_NAME)
        .bulk_block(vec!["foo", "bar", "baz"], true)
        .await
        .expect("callout failed");

    assert_eq!(result, ());
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

    let result = client
        .blogs(TEST_BLOG_NAME)
        .unblock(blog_to_unblock)
        .await
        .expect("callout failed");

    assert_eq!(result, ());
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
                "post": {
                    "id": 123456,
                    "id_string": "123456",
                    "blog_name": TEST_BLOG_NAME,
                    "post_url": format!("https://{}.tumblr.com/post/123456", TEST_BLOG_NAME),
                    "type": "text",
                    "timestamp": 1234567890,
                    "tags": ["test", "example"],
                    "note_count": 42,
                    "title": "Test Post",
                    "body": "<p>This is the post body</p>"
                }
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.posts().get(TEST_BLOG_NAME, "123456").await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let post_response = result.unwrap();
    assert_eq!(post_response.post.id, "123456");
    assert_eq!(post_response.post.blog_name, TEST_BLOG_NAME);
}

#[tokio::test]
async fn test_edit_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/post/edit", TEST_BLOG_NAME)))
        .and(body_json(serde_json::json!({
            "id": "123456",
            "title": "Updated Title",
            "body": "Updated body content",
            "tags": "updated,edited"
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
        .posts()
        .edit(TEST_BLOG_NAME, "123456")
        .title("Updated Title")
        .body("Updated body content")
        .tags(vec!["updated", "edited"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let edit_response = result.unwrap();
    assert_eq!(edit_response.id, "123456");
}

#[tokio::test]
async fn test_reblog_post() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/blog/{}/post/reblog", TEST_BLOG_NAME)))
        .and(body_json(serde_json::json!({
            "id": "789012",
            "reblog_key": "abc123reblogkey",
            "comment": "Great post!",
            "tags": "reblog,interesting"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "meta": {
                "status": 200,
                "msg": "OK"
            },
            "response": {
                "id": "999888"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client
        .posts()
        .reblog(TEST_BLOG_NAME, "789012", "abc123reblogkey")
        .comment("Great post!")
        .tags(vec!["reblog", "interesting"])
        .send()
        .await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let reblog_response = result.unwrap();
    assert_eq!(reblog_response.id, "999888");
}

// =============================================================================
// Community endpoint tests
// =============================================================================

#[tokio::test]
async fn test_community_join() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/community/rust-community/join"))
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

    Mock::given(method("POST"))
        .and(path("/community/rust-community/leave"))
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
    let result = client.communities("rust-community").leave().await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.success, Some(true));
}

#[tokio::test]
async fn test_community_members() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/community/rust-community/members"))
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
                    {
                        "name": "crabfan",
                        "title": "Crab Fan",
                        "description": "I love crabs",
                        "url": "https://crabfan.tumblr.com/",
                        "uuid": "t:def456",
                        "updated": 1234567891,
                        "posts": 25,
                        "is_nsfw": false,
                        "is_adult": false
                    }
                ]
            }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.communities("rust-community").members(Some(10), None).await;

    assert!(result.is_ok(), "Failed with: {:?}", result);
    let response = result.unwrap();
    assert_eq!(response.total_members, 150);
    assert_eq!(response.members.len(), 2);
    assert_eq!(response.members[0].name, "rustdev");
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
    let result = client.posts().get(TEST_BLOG_NAME, "999999").await;

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
