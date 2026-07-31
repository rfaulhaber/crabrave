//! Opt-in smoke tests against the real Tumblr API.
//!
//! Hermetic by default: every test is a no-op unless `CRABRAVE_LIVE_TESTS=1`.
//! Credentials come from the environment (`TUMBLR_CONSUMER_KEY`,
//! `TUMBLR_CONSUMER_SECRET`, `TUMBLR_ACCESS_TOKEN`, optional
//! `TUMBLR_TEST_BLOG`) or, when a variable is absent, from the gitignored
//! `tokens.json` at the crate root, which uses the same key names. Access
//! tokens are short-lived — refresh first (e.g. `tumblr.nu oauth refresh`)
//! and sync `tokens.json` before running:
//!
//! ```sh
//! CRABRAVE_LIVE_TESTS=1 cargo nextest run --test live_smoke
//! ```
//!
//! Only read-only endpoints are exercised; nothing here mutates the account.

// print_stderr: skip notices are the whole point of an opt-in suite.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stderr
)]

use crabrave::Crabrave;
use crabrave::handlers::blog::AvatarResponse;

fn credential(key: &str) -> Option<String> {
    if let Ok(v) = std::env::var(key)
        && !v.is_empty()
    {
        return Some(v);
    }
    let tokens_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tokens.json");
    let raw = fs_err::read_to_string(tokens_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&raw).ok()?;
    Some(json.get(key)?.as_str()?.to_owned())
}

/// Returns `None` (test becomes a skip) unless live testing is opted into.
fn live_client() -> Option<(Crabrave, String)> {
    if std::env::var("CRABRAVE_LIVE_TESTS").map(|v| v == "1") != Ok(true) {
        eprintln!("live smoke test skipped: set CRABRAVE_LIVE_TESTS=1 to run");
        return None;
    }
    let crab = Crabrave::builder()
        .consumer_key(credential("TUMBLR_CONSUMER_KEY").expect("missing TUMBLR_CONSUMER_KEY"))
        .consumer_secret(
            credential("TUMBLR_CONSUMER_SECRET").expect("missing TUMBLR_CONSUMER_SECRET"),
        )
        .access_token(credential("TUMBLR_ACCESS_TOKEN").expect("missing TUMBLR_ACCESS_TOKEN"))
        .build()
        .expect("failed to build client");
    let blog = credential("TUMBLR_TEST_BLOG").unwrap_or_else(|| "crabrave-demo".to_owned());
    Some((crab, blog))
}

#[tokio::test]
async fn live_user_info() {
    let Some((crab, _)) = live_client() else {
        return;
    };
    let info = crab.users().info().await.expect("user info failed");
    assert!(!info.user.name.is_empty());
    assert!(!info.user.blogs.is_empty());
}

#[tokio::test]
async fn live_user_limits() {
    let Some((crab, _)) = live_client() else {
        return;
    };
    crab.users().limits().await.expect("user limits failed");
}

#[tokio::test]
async fn live_blog_info() {
    let Some((crab, blog)) = live_client() else {
        return;
    };
    let info = crab
        .blogs(blog.as_str())
        .info()
        .await
        .expect("blog info failed");
    assert_eq!(info.blog.name, blog);
}

#[tokio::test]
async fn live_blog_posts_and_notes() {
    let Some((crab, blog)) = live_client() else {
        return;
    };
    let posts = crab
        .blogs(blog.as_str())
        .posts()
        .limit(20)
        .send()
        .await
        .expect("posts failed");
    assert!(
        !posts.posts.is_empty(),
        "test blog unexpectedly has no posts"
    );

    // Notes need a post that actually has some; the test blog keeps a few
    // reblogs of well-noted posts around for exactly this.
    let noted = posts.posts.iter().find(|p| p.note_count > 0);
    match noted {
        Some(post) => {
            let notes = crab
                .blogs(blog.as_str())
                .notes(post.id.clone())
                .send()
                .await
                .expect("notes failed");
            assert!(!notes.notes.is_empty());
        }
        None => eprintln!("no post with notes on {blog}; notes leg skipped"),
    }
}

#[tokio::test]
async fn live_tagged() {
    let Some((crab, _)) = live_client() else {
        return;
    };
    let tagged = crab
        .tagged("photography")
        .limit(4)
        .send()
        .await
        .expect("tagged failed");
    assert!(!tagged.posts.is_empty());
}

#[tokio::test]
async fn live_avatar() {
    let Some((crab, blog)) = live_client() else {
        return;
    };
    match crab
        .blogs(blog.as_str())
        .avatar(Some(128))
        .await
        .expect("avatar failed")
    {
        AvatarResponse::ImageData(bytes) => assert!(!bytes.is_empty()),
        AvatarResponse::ImageUrl { avatar_url } => assert!(avatar_url.starts_with("http")),
    }
}

#[tokio::test]
async fn live_user_likes_and_following() {
    let Some((crab, _)) = live_client() else {
        return;
    };
    crab.users()
        .likes()
        .limit(3)
        .send()
        .await
        .expect("user likes failed");
    let following = crab
        .users()
        .following()
        .limit(3)
        .send()
        .await
        .expect("user following failed");
    assert!(following.total_blogs > 0);
}

#[tokio::test]
async fn live_drafts_queue_notifications() {
    let Some((crab, blog)) = live_client() else {
        return;
    };
    // Empty results are fine — the point is that real (possibly empty)
    // envelopes deserialize.
    crab.blogs(blog.as_str())
        .drafts()
        .send()
        .await
        .expect("drafts failed");
    crab.blogs(blog.as_str())
        .queue()
        .send()
        .await
        .expect("queue failed");
    crab.blogs(blog.as_str())
        .notifications()
        .send()
        .await
        .expect("notifications failed");
}
