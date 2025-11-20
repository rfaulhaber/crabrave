# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`crabrave` is a Rust HTTP client library for the Tumblr API. The project implements both OAuth1 and OAuth2 authentication and provides a comprehensive client interface for interacting with Tumblr's API endpoints.

**Design Philosophy:** This client is modeled after [Octocrab](https://github.com/XAMPPRocky/octocrab) - designed to be very ergonomic to use within Rust. The API should feel natural and idiomatic, with a focus on developer experience similar to how Octocrab provides an elegant interface for GitHub's API.

**Project Structure:** This is a single Rust library crate with:
- Main library code at the repository root
- Handler modules organized in `handlers/` directory
- Integration tests in `tests/` directory
- No runtime dependencies like tokio (runtime-agnostic async)

## Development Environment

This project uses Nix flakes for reproducible development environments. To enter the development shell:

```bash
nix develop
```

The development shell includes:
- Rust stable toolchain (edition 2024)
- clippy (linter)
- rust-analyzer (LSP)
- cargo-nextest (test runner)
- claude-code

## Build Commands

**Build the library:**
```bash
cargo build
```

**Build with release optimizations:**
```bash
cargo build --release
```

**Run linter:**
```bash
cargo clippy
```

**Format code:**
```bash
nix fmt
```

## Testing

**Run all tests:**
```bash
cargo test
```

**Run tests with nextest (faster, parallel):**
```bash
cargo nextest run
```

**Run a specific test:**
```bash
cargo test test_name
```

## Architecture

The project uses a modular structure with the main library in `lib.rs` and specialized modules:

### Core Components

**`Crabrave` Client** (`lib.rs`)
- Main entry point with builder pattern initialization
- Supports OAuth1, OAuth2, and API-key-only authentication
- Runtime-agnostic async implementation (no tokio dependency in library)
- Automatic User-Agent header configuration
- Built-in rate limit detection (429 status handling)
- OAuth1 signature generation with HMAC-SHA1

**Error Handling** (`error.rs`)
- `CrabError` enum with comprehensive error types
- Rate limit errors include retry-after information
- Proper error chains with thiserror
- Specialized errors for authentication and API failures

**Response Parsing** (`response.rs`)
- `ApiResponse<T>` envelope parser
- Smart status checking before deserialization
- Handles Tumblr's `{meta, response}` structure
- `EmptyResponse` for endpoints that don't return data

**Models** (`models.rs`)
- `Blog`, `User` - Core data structures
- `BlogIdentifier` - Enum supporting name/hostname/UUID
- `Page<T>` - Pagination wrapper with total count and next page links
- `TumblrmartAccessories`, `Badge` - Premium features
- All models use serde for JSON serialization

**OAuth Module** (`oauth.rs`)
- `OAuth2Config` for managing OAuth2 authentication flow
- Authorization URL generation with CSRF token
- Token exchange functionality
- Uses oauth2 crate for standard OAuth2 flows

**NPF Module** (`npf.rs`)
- Neue Post Format (NPF) types for modern Tumblr posts
- `ContentBlock` enum: Text, Image, Link, Audio, Video
- `InlineFormat` for rich text formatting
- `MediaObject` for media handling
- `LayoutBlock` for content arrangement
- Helper constructors for common block types

### API Modules (handlers/)

**Naming Convention:** Uses simple plural names (not "Handler") for cleaner API

**Blogs API** (`handlers/blog.rs`)
```rust
// Blog information
crab.blogs("staff").info().await?

// Blog posts with builder
crab.blogs("staff").posts().limit(20).send().await?

// Avatar (returns URL or binary data depending on auth)
crab.blogs("staff").avatar(Some(128)).await?

// Followers and following
crab.blogs("staff").followers(Some(20), None).await?
crab.blogs("staff").following().limit(10).send().await?

// Likes
crab.blogs("staff").likes().limit(20).send().await?

// Blocks
crab.blogs("staff").blocks().limit(20).send().await?
```

**Users API** (`handlers/user.rs`)
```rust
// User information
crab.users().info().await?

// Dashboard posts
crab.users().dashboard().limit(20).send().await?

// User likes (via internal likes handler)
crab.users().likes().before(timestamp).send().await?

// Following operations (via internal following handler)
crab.users().following().limit(20).send().await?
crab.users().follow("blog").await?
crab.users().unfollow("blog").await?

// List joined communities
crab.users().joined_communities().await?

// Like/unlike posts
crab.users().like(post_id, reblog_key).await?
crab.users().unlike(post_id, reblog_key).await?

// Filtered tags management
crab.users().filtered_tags().limit(20).send().await?
crab.users().add_filtered_tags(vec!["spoilers", "nsfw"]).await?
crab.users().remove_filtered_tag("spoilers").await?

// Filtered content management
crab.users().filtered_content().limit(20).send().await?
crab.users().add_filtered_content(vec!["trigger warning"]).await?
crab.users().remove_filtered_content("trigger warning").await?
```

**Posts API** (`handlers/posts.rs`)
```rust
// Get a specific post
crab.blogs("my-blog").post("123456").get().await?

// Create posts using NPF
crab.blogs("my-blog")
    .create_post()
    .add_block(ContentBlock::text("Hello World!"))
    .tags(vec!["rust", "programming"])
    .send()
    .await?

// Create post with media uploads
crab.blogs("my-blog")
    .create_post()
    .add_image(MediaSource::from_path("/path/to/image.jpg"))
    .add_video(MediaSource::from_path("/path/to/video.mp4"))
    .add_audio(MediaSource::from_path("/path/to/audio.mp3"))
    .send()
    .await?

// Edit a post
crab.blogs("my-blog")
    .post("123456")
    .edit()
    .add_block(ContentBlock::text("Updated content"))
    .send()
    .await?

// Delete a post
crab.blogs("my-blog").post("123456").delete().await?

// Reblog a post
crab.blogs("my-blog")
    .reblog("source-blog", "123456", "reblog_key")
    .comment("Great post!")
    .send()
    .await?
```

**Tagged API** (`handlers/tagged.rs`)
```rust
// Search posts by tag
crab.tagged("photography").limit(20).send().await?
crab.tagged("art").before(1234567890).send().await?
crab.tagged("rust").filter("text").send().await?
```

**Communities API** (`handlers/communities.rs`)
```rust
// Community info
crab.communities("rust-community").info().await?

// Community timeline
crab.communities("rust-community")
    .timeline()
    .limit(20)
    .send()
    .await?

// Join/leave communities
crab.communities("rust-community").join().await?
crab.communities("rust-community").leave("my-blog").await?

// Get members with pagination
crab.communities("rust-community")
    .members()
    .limit(20)
    .send()
    .await?

// Member management (moderator/admin)
crab.communities("rust-community").remove_member("bad-blog").await?
crab.communities("rust-community").change_role("trusted-blog", MemberRole::Moderator).await?

// Mute/unmute notifications
crab.communities("rust-community").mute().await?
crab.communities("rust-community").unmute().await?

// Invitation management
crab.communities("rust-community").invitations().await?
crab.communities("rust-community").invite("friend-blog").await?
crab.communities("rust-community").cancel_invitation("friend-blog").await?
crab.communities("rust-community").invitation_status("friend-blog").await?
crab.communities("rust-community").regenerate_invite_url().await?

// Reactions on posts
crab.communities("rust-community").add_reaction("123456", "heart").await?
crab.communities("rust-community").remove_reaction("123456", "heart").await?

// Moderation (moderator/admin)
crab.communities("rust-community").moderated_post("123456").await?
crab.communities("rust-community").moderate_post("123456").await?
crab.communities("rust-community").restore_post("123456").await?
```

**Internal Handlers:**
- `following.rs` - Following/followers operations used by both Blogs and Users APIs
- `likes.rs` - Likes operations used by both Blogs and Users APIs

**Media Module** (`media.rs`)
```rust
// Create media sources for uploads
let image = MediaSource::from_path("/path/to/image.jpg");
let video = MediaSource::from_bytes("video.mp4", video_bytes);
let audio = MediaSource::from_path("/path/to/song.mp3")
    .with_mime_type("audio/mpeg");
```

### Builder Pattern

All complex queries use type-safe builders:
- `PostsBuilder` - Blog posts with filters (type, tag, limit, offset, etc.)
- `DashboardBuilder` - User dashboard with options
- `LikesBuilder` - Liked posts with pagination (limit, before, after)
- `FollowingBuilder` - Following list with pagination
- `TaggedBuilder` - Tagged posts search
- `TimelineBuilder` - Community timeline
- `MembersBuilder` - Community members with pagination
- `BlocksBuilder` - Blocked blogs
- `CreatePostBuilder` - NPF post creation with media uploads
- `EditPostBuilder` - NPF post editing with media uploads
- `ReblogBuilder` - Reblog with comments/content

### API Endpoints

- Base URL: `https://api.tumblr.com/v2`
- OAuth2 authorize: `https://www.tumblr.com/oauth2/authorize`
- OAuth2 token: `https://api.tumblr.com/v2/oauth2/token`
- All requests include required User-Agent header
- Rate limits: 300/min per IP, 1000/hr per key
- 429 status code triggers `CrabError::RateLimit` with retry-after

### Dependencies

**Core:**
- `reqwest`: HTTP client with features: json, multipart, rustls-tls, charset, system-proxy
- `serde`/`serde_json`: JSON serialization
- `serde_urlencoded`: Query parameter encoding
- `thiserror`: Custom error types
- `url`: URL parsing
- `oauth2`: OAuth2 flow implementation

**Cryptography (for OAuth1):**
- `base64`: Base64 encoding for signatures
- `hmac`: HMAC implementation
- `sha1`: SHA-1 hashing for OAuth1 signatures
- `urlencoding`: URL encoding for OAuth1 parameters

**Other:**
- `anyhow`: Error handling utilities

**Dev Dependencies:**
- `wiremock`: HTTP mocking for tests
- `tokio`: Async runtime for tests (not used in library)
- `dirs`: Directory utilities for tests

## Implementation Status

**Current Status:** All major API modules are implemented and functional

✅ **Complete:**
- Core client with OAuth1/OAuth2/API-key authentication
- Error handling and response parsing
- Models and type system
- NPF (Neue Post Format) support
- Media uploads (images, videos, audio) via multipart/form-data
- Blogs API (info, posts, avatar, followers, following, likes, blocks, drafts, queue, submissions, notifications, notes, pages)
- Users API (info, dashboard, likes, following, like/unlike, filtered tags, filtered content, joined communities)
- Posts API (get, create, edit, delete, reblog, mute) with NPF and media upload support
- Tagged API (search by tag)
- Communities API (info, timeline, join/leave, members, member management, mute/unmute, invitations, reactions, moderation)

🚧 **Potential Future Work:**
- Additional NPF content block types
- Pagination helper utilities (cursor-based iteration)
- Rate limit retry logic (automatic backoff)
- Token refresh for OAuth2

## Important Notes

- Uses Rust edition 2024
- TLS uses `rustls-tls` rather than native-tls for better portability
- User-Agent header is **required** by Tumblr API (apps may be suspended without it)
- The library has no runtime dependencies - users bring their own async runtime (tokio, async-std, etc.)
- Strict clippy configuration enforces no unwrap/expect/panic/todo/print in production code
- OAuth1 generates HMAC-SHA1 signatures for legacy app support
- The `/blog/avatar` endpoint returns binary data for non-OAuth1 requests, JSON for OAuth1
- Tagged endpoint returns posts directly, not wrapped in `response.posts` like other endpoints
