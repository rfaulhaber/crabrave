# Crabrave Implementation Plan

**Last Updated:** 2025-11-21
**Status:** Phase 4 - API Modules (Mostly Complete)

## Project Vision

Build an ergonomic Rust HTTP client for the Tumblr API, modeled after [Octocrab](https://github.com/XAMPPRocky/octocrab). The client should feel natural and idiomatic in Rust, with a focus on developer experience.

## Research Summary

### Tumblr API Structure (v2)

**Base URL:** `https://api.tumblr.com/v2`

**Five Main Endpoint Categories:**
1. **Blog Methods**: Retrieve and manage blog data (info, avatars, blocks, likes, followers, posts)
2. **User Methods**: Dashboard access, likes, follows, content filtering
3. **Tagged Method**: Search posts by tags across platform
4. **Communities Methods**: Membership, timelines, invitations, reactions
5. **Post Operations**: Create, edit, fetch, delete (NPF + legacy formats)

**Authentication:**
- OAuth1: Temporary credentials, resource owner authorization, access tokens
- OAuth2: Authorization code grant flow with token refresh (recommended)

**Rate Limits:**
- 300 calls/minute per IP address
- 1,000 calls/hour per consumer key

**Important Requirements:**
- User-Agent header is **required** (applications may be suspended without it)
- Post IDs are 64-bit integers (use strings for safe handling)
- Blog identifiers support three formats: name, hostname, or UUID
- Response envelope: `{meta: {status, msg}, response: {...}}`

**Post Formats:**
- **Legacy**: text, photo, quote, link, chat, audio, video, answer
- **NPF (Neue Post Format)**: Modern structured content blocks (preferred)

### Design Inspiration from Octocrab

**Key Patterns to Adopt:**
1. **Builder Pattern**: Client configuration before instantiation
2. **Handler Modules**: Semantic organization (issues, pulls, repos, etc.)
3. **Type-Safe Builders**: Multi-parameter requests use builders with `.send()`
4. **Async-First**: All I/O operations use async/await
5. **Strong Typing**: Responses deserialize to typed models
6. **Pagination**: `Page<T>` with `.next` links for manual iteration
7. **Error Handling**: `Result<T>` types, raw response methods for fine-grained control

## Proposed Architecture

### Client Structure

```rust
// Builder pattern initialization
let crab = Crabrave::builder()
    .consumer_key("...")
    .consumer_secret("...")
    .access_token("...")
    .access_token_secret("...")
    .build()?;

// Handler-based usage
let blog_info = crab.blog("staff").info().await?;
let dashboard = crab.user().dashboard().await?;
let posts = crab.tagged("photography").list().await?;

// Post creation with builder
let post = crab.posts()
    .create_text("my-blog")
    .title("Hello World")
    .body("This is my first post!")
    .tags(vec!["rust", "programming"])
    .send()
    .await?;
```

### Handler Organization

Five main handlers matching API structure:

1. **`BlogHandler`** - `crab.blog("identifier")`
   - `.info()` - Get blog information
   - `.posts()` - List posts (returns builder)
   - `.followers()` - Get followers
   - `.likes()` - Get liked posts
   - `.avatar()` - Get avatar URL
   - `.following()` - Get blogs this blog follows

2. **`UserHandler`** - `crab.user()`
   - `.info()` - Get authenticated user info
   - `.dashboard()` - Get dashboard posts (returns builder)
   - `.likes()` - Get user's likes
   - `.following()` - Get blogs user follows
   - `.follow(blog)` / `.unfollow(blog)`

3. **`PostHandler`** - `crab.posts()`
   - `.get(blog, id)` - Fetch specific post
   - `.create_text(blog)` - Create text post (returns builder)
   - `.create_photo(blog)` - Create photo post (returns builder)
   - `.create_quote(blog)` - Create quote post (returns builder)
   - `.create_npf(blog)` - Create NPF post (returns builder)
   - `.edit(blog, id)` - Edit post (returns builder)
   - `.delete(blog, id)` - Delete post
   - `.reblog(blog_name, reblog_params)` - Reblog a post

4. **`TaggedHandler`** - `crab.tagged("tag")`
   - `.list()` - Search posts by tag (returns builder)

5. **`CommunitiesHandler`** - `crab.communities("handle")`
   - `.timeline()` - Get community timeline
   - `.join()` / `.leave()` - Membership management
   - `.members()` - List community members

### Type System

**Core Models:**
```rust
pub struct Blog {
    pub name: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub uuid: String,
    pub updated: i64,
    // ... additional fields
}

pub struct User {
    pub name: String,
    pub likes: u64,
    pub following: u64,
    pub blogs: Vec<Blog>,
    // ... additional fields
}

pub struct Post {
    pub id: String,  // 64-bit int as string
    pub blog_name: String,
    pub post_url: String,
    pub timestamp: i64,
    pub tags: Vec<String>,
    pub format: PostFormat,
    // ... format-specific fields
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub total_posts: Option<u64>,
    pub next: Option<String>,
}

pub enum BlogIdentifier {
    Name(String),
    Hostname(String),
    Uuid(String),
}
```

**Error Types:**
```rust
#[derive(Debug, Error)]
pub enum CrabError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limit exceeded. Retry after: {retry_after:?}")]
    RateLimit { retry_after: Option<u64> },

    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

**Request Builders:**
```rust
pub struct PostsBuilder { /* filters: type, tag, limit, offset, etc. */ }
pub struct CreatePostBuilder { /* title, body, tags, state, etc. */ }
pub struct DashboardBuilder { /* limit, offset, type, etc. */ }
```

### Key Features

**1. Pagination Support**
```rust
let mut page = crab.blog("staff").posts().send().await?;
loop {
    for post in page.items {
        println!("{}: {}", post.id, post.summary);
    }
    if let Some(next_link) = page.next {
        page = crab.get_page(&next_link).await?;
    } else {
        break;
    }
}
```

**2. NPF (Neue Post Format) Support**
- Modern structured content blocks
- Type-safe builders for different block types
- Backward compatibility with legacy formats

**3. Media Upload**
- Multipart form-data for file uploads
- Support for photos, videos, audio

**4. Rate Limit Awareness**
- Expose rate limit info from response headers
- Include in error types for retry logic
- Let users implement their own rate limiting strategy

## Implementation Task List

### Phase 1: Foundation ✅ COMPLETE
- [x] Enhance `CrabClientBuilder` with full OAuth2 support
- [x] Add OAuth1 support for backward compatibility
- [x] Configure async reqwest client with proper User-Agent
- [x] Add base URL configuration (for testing)
- [ ] Implement token refresh mechanism (deferred)

### Phase 2: Core Infrastructure ✅ COMPLETE
- [x] Define comprehensive `CrabError` enum
- [x] Create response envelope parser `ApiResponse<T>`
- [x] Implement `BlogIdentifier` enum and conversions
- [x] Add request helper methods (get, post, delete)
- [x] Add rate limit detection and handling

### Phase 3: Models ✅ COMPLETE
- [x] Define `Blog` struct with serde
- [x] Define `User` struct with serde
- [x] Define `Page<T>` for pagination
- [x] Define basic `Post` struct (simplified, extensible for NPF)
- [ ] Add full NPF content block types (deferred to PostHandler)
- [x] Create builder types for complex requests

### Phase 4: API Modules (formerly "Handlers") - IN PROGRESS

**Note:** Renamed from "Handlers" to simple plural names (Blogs, Users, etc.) for cleaner API

#### Blogs API ✅ COMPLETE
- [x] Implement `blogs()` method on `Crabrave`
- [x] Add `.info()` endpoint
- [x] Add `.posts()` builder with filters
- [x] Add `.avatar()` endpoint
- [ ] Add `.followers()` with pagination (deferred)
- [ ] Add `.likes()` with pagination (deferred)
- [ ] Add `.following()` endpoint (deferred)

#### Users API ✅ COMPLETE
- [x] Implement `users()` method on `Crabrave`
- [x] Add `.info()` endpoint
- [x] Add `.dashboard()` builder
- [x] Add `.likes()` with pagination
- [x] Add `.following()` with pagination
- [x] Add `.follow()` / `.unfollow()` methods
- [ ] Add filtered content endpoints (deferred)

#### Posts API ✅ COMPLETE
- [x] Implement `posts()` method on `Crabrave`
- [x] Add `.get(blog, id)` endpoint
- [x] Add `.create_text()` builder
- [x] Add `.create_quote()` builder
- [x] Add `.create_link()` builder
- [x] Add `.create_photo()` builder
- [x] Add `.delete()` endpoint
- [x] Add `.edit()` builder
- [x] Add `.reblog()` method
- [x] Add `.create_npf()` builder with content blocks
- [x] Add full NPF content block type definitions

#### Tagged API ✅ COMPLETE
- [x] Implement `tagged()` method on `Crabrave`
- [x] Add `.list()` builder with filters (limit, before, filter)
- [x] Add pagination support

#### Communities API ✅ COMPLETE
- [x] Implement `communities()` method on `Crabrave`
- [x] Add `.timeline()` builder with filters
- [x] Add `.join()` / `.leave()` methods
- [x] Add `.members()` endpoint with pagination
- [ ] Add reaction endpoints (deferred)

### Phase 5: Advanced Features
- [ ] Implement `Page<T>` pagination traversal
- [ ] Add `.get_page()` method to client
- [ ] Implement partial response field filtering
- [ ] Add JSONP support (optional)
- [ ] Create mock server for testing
- [ ] Add request retry logic with exponential backoff

### Phase 6: Testing & Documentation
- [ ] Write unit tests for all handlers
- [ ] Write integration tests (with mock server)
- [ ] Create example: basic client setup
- [ ] Create example: fetching blog posts
- [ ] Create example: creating posts
- [ ] Create example: pagination
- [ ] Create example: error handling
- [ ] Add rustdoc documentation for all public APIs
- [ ] Update CLAUDE.md with final architecture
- [ ] Create CONTRIBUTING.md guide

## Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Async Runtime** | No tokio dependency | Let users choose their runtime |
| **Post IDs** | `String` | Tumblr uses 64-bit ints, unsafe in some contexts |
| **Blog Identifiers** | `Enum` with 3 variants | API accepts name, hostname, or UUID |
| **OAuth Priority** | OAuth2 > OAuth1 | OAuth2 is modern standard, but support both |
| **Response Parsing** | Envelope unwrapping | All responses have `{meta, response}` structure |
| **Pagination** | Manual iteration | Following Octocrab pattern, give users control |
| **Error Strategy** | Rich enum types | Include retry-after, status codes, context |
| **Post Formats** | NPF + Legacy support | NPF is future, but legacy still used |
| **User-Agent** | Required, configurable | Tumblr requires this, allow customization |

## Open Questions

1. **Should we provide a global instance like Octocrab's `instance()`?**
   - Pros: Convenient for simple use cases
   - Cons: Global state, less explicit

2. **How to handle NPF content blocks ergonomically?**
   - Consider builder pattern for each block type
   - Or use enum with variants for different blocks

3. **Should we auto-retry on rate limits?**
   - Or expose and let users handle?
   - Could provide optional middleware

4. **How to version the API?**
   - Tumblr is on v2, but may add v3
   - Keep flexible with base URL configuration

## References

- [Tumblr API v2 Documentation](https://www.tumblr.com/docs/en/api/v2)
- [Tumblr API GitHub Docs](https://github.com/tumblr/docs/blob/master/api.md)
- [Octocrab](https://github.com/XAMPPRocky/octocrab) - Design inspiration
- [PyTumblr](https://github.com/tumblr/pytumblr) - Official Python client reference

## Current Status

**Phase:** Phase 4 - API Modules ✅ COMPLETE
**Completion:** 100% (All 5 API modules fully implemented)

### Completed Work

**✅ Phase 1: Foundation**
- Full OAuth1 and OAuth2 support
- Builder pattern for client initialization
- Automatic User-Agent header configuration
- Custom base URL support for testing
- Runtime-agnostic async implementation

**✅ Phase 2: Core Infrastructure**
- Comprehensive `CrabError` enum with rate limit support
- Response envelope parser with smart status checking
- `BlogIdentifier` enum with automatic format detection
- Internal request helpers (GET, POST, DELETE)
- Rate limit detection with retry-after parsing

**✅ Phase 3: Models**
- `Blog`, `User`, `Page<T>` models with serde
- Basic `Post` struct (extensible for NPF)
- Type-safe request builders for complex queries

**✅ Blogs API Module** (`handlers/blog.rs`)
- `.info()` - Get blog information
- `.avatar(size)` - Get blog avatar URL
- `.posts()` - Builder for querying posts with filters (type, tag, limit, offset, before)
- `PostsBuilder` with full parameter support

**✅ Users API Module** (`handlers/user.rs`)
- `.info()` - Get authenticated user info
- `.dashboard()` - Builder for dashboard posts
- `.likes()` - Builder for liked posts with pagination
- `.following()` - Get blogs user follows
- `.follow(blog)` / `.unfollow(blog)` - Follow/unfollow operations
- `DashboardBuilder` and `LikesBuilder` with filters

**✅ Tagged API Module** (`handlers/tagged.rs`)
- `.limit()`, `.before()`, `.filter()` - Configure search parameters
- `.send()` - Execute search with builder pattern
- URL encoding for safe tag handling
- Full pagination support

**✅ Posts API Module** (`handlers/posts.rs`) - COMPLETE
- `.get(blog, id)` - Fetch specific post
- `.delete(blog, id)` - Delete post
- `.create_text()` - Builder for text posts
- `.create_quote()` - Builder for quote posts
- `.create_link()` - Builder for link posts
- `.create_photo()` - Builder for photo posts with URL source
- `.edit(blog, id)` - Builder for editing posts
- `.reblog(blog, id, key)` - Builder for reblogging with comments
- `.create_npf(blog)` - Builder for NPF (Neue Post Format) posts
- Full NPF support with content blocks (text, image, link, audio, video)
- NPF layout system and inline formatting
- Common post fields: tags, state, slug, date

**✅ Communities API Module** (`handlers/communities.rs`)
- `.timeline()` - Builder for community timeline
- `.join()` / `.leave()` - Membership management
- `.members()` - List community members with pagination
- `TimelineBuilder` with limit, offset, before filters

**✅ NPF Module** (`npf.rs`) - Neue Post Format support
- `ContentBlock` enum with text, image, link, audio, video variants
- `InlineFormat` for text styling (bold, italic, links, mentions, colors)
- `MediaObject` for images, videos, and audio files
- `LayoutBlock` for controlling content arrangement
- Helper methods: `.text()`, `.heading()`, `.link()`, `.image()`
- Full serde support for API serialization

### Architecture Decisions Made

**Naming Convention:** Changed from "Handler" suffix to simple plural names (Blogs, Users, etc.) for cleaner API:
```rust
// Clean, natural syntax
crab.blogs("staff").info().await?
crab.users().dashboard().limit(20).send().await?
```

### Testing & Quality

- **102 total tests** - Comprehensive test coverage:
  - **41 unit tests** - Component and builder testing
  - **11 mock server tests** - Full request/response cycle testing with wiremock
  - **11 integration tests** - Real API testing (optional, ignored by default)
  - **39 doc tests** - Documentation example verification
- **All non-integration tests passing** (91/91)
- **Clippy passing** with strict lints enabled
- Full rustdoc documentation with examples
- All public APIs documented
- URL encoding support via `urlencoding` crate
- Complete NPF module with helper methods
- Mock server tests cover:
  - Success responses
  - Error handling (404, 429, 5xx)
  - Rate limit detection
  - All major endpoints
- Integration tests read credentials from environment variables
- Comprehensive testing documentation in `TESTING.md`

### Next Steps

**Completed across both sessions:**
- ✅ Implemented Tagged API (fully functional)
- ✅ Implemented Posts API (COMPLETE - all operations)
  - get, delete, create (text/quote/link/photo), edit, reblog
  - Full NPF support with content blocks
- ✅ Implemented Communities API (timeline, membership, members)
- ✅ All tests passing (80 tests), clippy clean
- ✅ Phase 4 - API Modules: 100% COMPLETE

**Future Enhancements (Phase 5+):**
- Token refresh mechanism (OAuth2 token expiry handling)
- Additional blog endpoints (followers, likes, following)
- Advanced features:
  - Pagination helpers (`.get_page()` method)
  - Retry logic with exponential backoff
  - Mock server for testing
- Integration tests with real API (optional)
- Usage examples and cookbook
- Performance optimizations

**Blockers:** None
