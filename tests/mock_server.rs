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
