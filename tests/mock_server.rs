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
        .get()
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
        .get()
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
        .get()
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
        .get()
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
        .get()
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
