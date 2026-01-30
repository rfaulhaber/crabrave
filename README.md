# crabrave

An ergonomic Rust client for the Tumblr API, inspired by [Octocrab](https://github.com/XAMPPRocky/octocrab).

## Features

- OAuth1 and OAuth2 authentication
- Type-safe builder patterns for all API operations
- NPF (Neue Post Format) support for modern posts
    - Note that this project supports NPF _only_. Legacy posts will be returned in the trail content of `Post` structs
- Media uploads 
- Runtime-agnostic async
- Comprehensive error handling with rate limit detection

## Installation

```toml
[dependencies]
crabrave = "0.4"
```

## Quick Start

```rust
use crabrave::Crabrave;

#[tokio::main]
async fn main() -> Result<(), crabrave::CrabError> {
    let crab = Crabrave::builder()
        .consumer_key("your_consumer_key")
        .consumer_secret("your_consumer_secret")
        .access_token("your_access_token")
        .build()?;

    // Get blog info
    let info = crab.blogs("staff").info().await?;
    println!("Blog: {}", info.blog.title);

    // Get dashboard posts
    let dashboard = crab.users().dashboard().limit(20).send().await?;
    for post in dashboard.posts {
        println!("{}: {}", post.blog_name, post.id);
    }

    Ok(())
}
```

## OAuth2 Flow

```rust
use crabrave::oauth::OAuth2Config;

// 1. Generate authorization URL
let config = OAuth2Config::new(
    "consumer_key",
    "consumer_secret",
    "http://localhost:8080/callback"
);
let (auth_url, csrf_token) = config.authorize_url();

// 2. Direct user to auth_url, receive code in callback

// 3. Exchange code for token
let token = config.exchange_code("authorization_code").await?;

// 4. Use token with client
let crab = Crabrave::builder()
    .consumer_key("consumer_key")
    .consumer_secret("consumer_secret")
    .access_token(&token.access_token)
    .build()?;
```

## Creating Posts

```rust
use crabrave::npf::ContentBlock;
use crabrave::media::MediaSource;

// Text post
crab.blogs("my-blog")
    .create_post()
    .add_block(ContentBlock::text("Hello from Rust!"))
    .tags(vec!["rust", "crabrave"])
    .send()
    .await?;

// Post with image upload
crab.blogs("my-blog")
    .create_post()
    .add_image(MediaSource::from_path("/path/to/image.jpg"))
    .send()
    .await?;
```

## Error Handling

```rust
use crabrave::CrabError;

match crab.blogs("nonexistent").info().await {
    Ok(info) => println!("{}", info.blog.name),
    Err(CrabError::Api { status, message }) => {
        println!("API error {}: {}", status, message);
    }
    Err(CrabError::RateLimit { retry_after }) => {
        println!("Rate limited, retry after {:?}s", retry_after);
    }
    Err(e) => println!("Error: {}", e),
}
```

## API Coverage

| Module | Endpoints |
|--------|-----------|
| Blogs | info, posts, avatar, followers, following, likes, blocks, drafts, queue, submissions, notifications, notes, pages |
| Users | info, dashboard, likes, following, follow/unfollow, like/unlike, filtered tags, filtered content |
| Posts | get, create, edit, delete, reblog, mute |
| Tagged | search posts by tag |
| Communities | info, timeline, join/leave, members, invitations, moderation, reactions |
