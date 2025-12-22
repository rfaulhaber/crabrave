//! Blog-related API endpoints

use crate::{
    Blog, BlogIdentifier, CrabResult, Crabrave, EmptyResponse,
    handlers::{following::FollowingBuilder, likes::LikesBuilder},
    models::TumblrmartAccessories,
};
use serde::{Deserialize, Serialize};

/// API for blog-related endpoints
///
/// Provides access to blog information, posts, followers, and other blog-specific operations.
///
/// # Example
///
/// ```no_run
/// use crabrave::Crabrave;
///
/// # async fn example() -> Result<(), crabrave::CrabError> {
/// let crab = Crabrave::builder()
///     .consumer_key("key")
///     .build()?;
///
/// // Get blog information
/// let info = crab.blogs("staff").info().await?;
/// println!("Blog: {} - {}", info.blog.name, info.blog.title);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Blogs {
    client: Crabrave,
    identifier: BlogIdentifier,
}

impl Blogs {
    /// Creates a new Blogs API for the specified blog
    pub(crate) fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self { client, identifier }
    }

    /// Gets information about the blog
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let info = crab.blogs("staff").info().await?;
    /// println!("Blog title: {}", info.blog.title);
    /// println!("Total posts: {}", info.blog.posts);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn info(&self) -> CrabResult<BlogInfo> {
        let path = format!("blog/{}/info", self.identifier.as_str());
        self.client.get(&path).await
    }

    /// Gets the avatar URL for the blog
    ///
    /// # Arguments
    ///
    /// * `size` - Optional size for the avatar (16, 24, 30, 40, 48, 64, 96, 128, 512)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::blog::AvatarResponse;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let avatar = crab.blogs("staff").avatar(Some(128)).await?;
    /// match avatar {
    ///     AvatarResponse::ImageUrl { avatar_url } => println!("Avatar URL: {}", avatar_url),
    ///     AvatarResponse::ImageData(data) => println!("Got {} bytes of image data", data.len()),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn avatar(&self, size: Option<u16>) -> CrabResult<AvatarResponse> {
        let path = if let Some(size) = size {
            format!("blog/{}/avatar/{}", self.identifier.as_str(), size)
        } else {
            format!("blog/{}/avatar", self.identifier.as_str())
        };
        self.client.get_avatar(&path).await
    }

    pub fn blocks(&self) -> BlocksBuilder {
        BlocksBuilder::new(self.client.clone(), self.identifier.clone())
    }

    pub async fn block_blog(
        &self,
        blog: impl Into<BlogIdentifier>,
    ) -> CrabResult<BlockBlogRespsone> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());
        let blog_id: BlogIdentifier = blog.into();
        self.client
            .post(
                &path,
                &BlockBlogRequest::Blog {
                    blocked_tumblelog: blog_id.to_string(),
                },
            )
            .await
    }

    pub async fn block_with_post_id(
        &self,
        post_id: impl Into<String>,
    ) -> CrabResult<BlockBlogRespsone> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());
        let post_id = post_id.into();
        self.client
            .post(&path, &BlockBlogRequest::Post { post_id })
            .await
    }

    pub async fn bulk_block(&self, blogs: Vec<impl Into<String>>, force: bool) -> CrabResult<()> {
        let path = format!("blog/{}/blocks/bulk", self.identifier.as_str());
        let blogs_str = blogs
            .into_iter()
            .map(|b| b.into())
            .collect::<Vec<String>>()
            .join(",");

        self.client
            .post(
                &path,
                &BulkBlockRequest {
                    blocked_tumblelogs: blogs_str,
                    force,
                },
            )
            .await
            .map(|_resp: EmptyResponse| ())
    }

    // TODO unblock via anonymous ID
    pub async fn unblock(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<()> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());
        let blog_id: BlogIdentifier = blog.into();

        self.client
            .delete_with_query(
                &path,
                &serde_json::json!({ "blocked_tumblelog": blog_id.as_str() }),
            )
            .await
            .map(|_resp: EmptyResponse| ())
    }

    /// Get this blog's likes
    pub fn likes(&self) -> LikesBuilder {
        LikesBuilder::blog(self.client.clone(), self.identifier.clone())
    }

    /// Get the blogs following this blog
    pub fn following(&self) -> FollowingBuilder {
        FollowingBuilder::blog(self.client.clone(), self.identifier.clone())
    }

    pub fn followers(&self) -> FollowersBuilder {
        FollowersBuilder::new(self.client.clone(), self.identifier.clone())
    }

    pub async fn followed_by(&self, blog_name: impl std::fmt::Display) -> CrabResult<bool> {
        #[derive(Deserialize)]
        struct FollowedByResponse {
            followed_by: bool,
        }

        let path = format!("blog/{}/followed_by?query={}", self.identifier, blog_name);
        let resp: FollowedByResponse = self.client.get(&path).await?;
        Ok(resp.followed_by)
    }

    /// Gets posts from the blog
    ///
    /// Returns a builder for configuring the posts request.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let posts = crab.blogs("staff")
    ///     .posts()
    ///     .limit(20)
    ///     .offset(0)
    ///     .send()
    ///     .await?;
    ///
    /// for post in posts.posts {
    ///     println!("Post: {}", post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn posts(&self) -> PostsBuilder {
        PostsBuilder::new(self.client.clone(), self.identifier.clone())
    }

    /// Gets queued posts for the blog
    ///
    /// Returns a builder for configuring the queue request.
    /// Requires OAuth authentication.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// let queued = crab.blogs("my-blog")
    ///     .queue()
    ///     .limit(10)
    ///     .send()
    ///     .await?;
    ///
    /// for post in queued.posts {
    ///     println!("Queued post: {}", post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn queue(&self) -> QueueBuilder {
        QueueBuilder::new(self.client.clone(), self.identifier.clone())
    }

    /// Reorders a post within the queue
    ///
    /// Moves a post to a new position in the queue.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to move
    /// * `insert_after` - The ID of the post to insert after, or "0" to move to the first position
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// // Move post 123456 to the first position
    /// crab.blogs("my-blog").reorder_queue("123456", "0").await?;
    ///
    /// // Move post 123456 to after post 789012
    /// crab.blogs("my-blog").reorder_queue("123456", "789012").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reorder_queue(
        &self,
        post_id: impl Into<String>,
        insert_after: impl Into<String>,
    ) -> CrabResult<QueueActionResponse> {
        let path = format!("blog/{}/posts/queue/reorder", self.identifier.as_str());
        let body = ReorderQueueRequest {
            post_id: post_id.into(),
            insert_after: insert_after.into(),
        };
        self.client.post(&path, &body).await
    }

    /// Shuffles the queue randomly
    ///
    /// Randomly reorders all posts in the queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// crab.blogs("my-blog").shuffle_queue().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn shuffle_queue(&self) -> CrabResult<QueueActionResponse> {
        let path = format!("blog/{}/posts/queue/shuffle", self.identifier.as_str());
        self.client.post(&path, &serde_json::json!({})).await
    }

    /// Gets draft posts for the blog
    ///
    /// Returns a builder for configuring the drafts request.
    /// Requires OAuth authentication.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// let drafts = crab.blogs("my-blog")
    ///     .drafts()
    ///     .send()
    ///     .await?;
    ///
    /// for post in drafts.posts {
    ///     println!("Draft post: {}", post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn drafts(&self) -> DraftsBuilder {
        DraftsBuilder::new(self.client.clone(), self.identifier.clone())
    }
}

/// Response from the blog info endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogInfo {
    /// Blog information
    pub blog: Blog,
}

/// Response from the avatar endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AvatarResponse {
    ImageData(Vec<u8>),
    ImageUrl { avatar_url: String },
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AvatarResponseUrl {
    pub avatar_url: String,
}

/// Query parameters for fetching blog posts
#[derive(Debug, Clone, Serialize, Default)]
struct PostsQuery {
    /// Maximum number of posts to return (API max: 20, default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Post offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,

    /// Filter by post type (text, photo, quote, link, chat, audio, video)
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    post_type: Option<String>,

    /// Filter by tag
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,

    /// Return posts before this timestamp (Unix time)
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<i64>,
}

/// Builder for querying blog posts
///
/// This builder allows you to configure various parameters for fetching posts
/// from a blog before sending the request.
pub struct PostsBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: PostsQuery,
}

impl PostsBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self {
            client,
            identifier,
            query: PostsQuery::default(),
        }
    }

    /// Sets the number of posts to return (max 20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Sets the post offset for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Filters posts by type (text, photo, quote, link, chat, audio, video)
    pub fn post_type(mut self, post_type: impl Into<String>) -> Self {
        self.query.post_type = Some(post_type.into());
        self
    }

    /// Filters posts by tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.query.tag = Some(tag.into());
        self
    }

    /// Returns posts before this timestamp (Unix time)
    pub fn before(mut self, timestamp: i64) -> Self {
        self.query.before = Some(timestamp);
        self
    }

    /// Sends the request and returns the posts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<PostsResponse> {
        let path = format!("blog/{}/posts", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Query parameters for fetching queued posts
#[derive(Debug, Clone, Serialize, Default)]
struct QueueQuery {
    /// Maximum number of posts to return (1-20, default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Post number to start at (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,

    /// Response format filter: "text" for plain text, "raw" for user-entered format
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
}

/// Builder for querying a blog's post queue
///
/// This builder allows you to configure various parameters for fetching queued posts
/// from a blog before sending the request.
pub struct QueueBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: QueueQuery,
}

impl QueueBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self {
            client,
            identifier,
            query: QueueQuery::default(),
        }
    }

    /// Sets the number of posts to return (1-20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Sets the post offset for pagination (default 0)
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Sets the response format filter
    ///
    /// Options:
    /// - "text": Returns plain text only, strips HTML
    /// - "raw": Returns the content in the user-entered format
    /// - None (default): Returns HTML formatted content
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.query.filter = Some(filter.into());
        self
    }

    /// Sends the request and returns the queued posts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<QueueResponse> {
        let path = format!("blog/{}/posts/queue", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Response from the queue endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueResponse {
    /// List of queued posts
    pub posts: Vec<Post>,
}

/// Request body for reordering a post in the queue
#[derive(Debug, Clone, Serialize)]
struct ReorderQueueRequest {
    /// The ID of the post to move
    post_id: String,
    /// The ID of the post to insert after (use "0" for first position)
    insert_after: String,
}

/// Response from queue reorder/shuffle operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueActionResponse {
    /// Success status (may not always be present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

/// Query parameters for fetching draft posts
#[derive(Debug, Clone, Serialize, Default)]
struct DraftsQuery {
    /// Return posts that have appeared before this ID
    #[serde(skip_serializing_if = "Option::is_none")]
    before_id: Option<String>,

    /// Response format filter: "text" for plain text, "raw" for user-entered format
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,
}

/// Builder for querying a blog's draft posts
///
/// This builder allows you to configure various parameters for fetching draft posts
/// from a blog before sending the request.
pub struct DraftsBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: DraftsQuery,
}

impl DraftsBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self {
            client,
            identifier,
            query: DraftsQuery::default(),
        }
    }

    /// Return posts that have appeared before this ID
    ///
    /// Use this for pagination through draft posts.
    pub fn before_id(mut self, id: impl Into<String>) -> Self {
        self.query.before_id = Some(id.into());
        self
    }

    /// Sets the response format filter
    ///
    /// Options:
    /// - "text": Returns plain text only, strips HTML
    /// - "raw": Returns the content in the user-entered format
    /// - None (default): Returns HTML formatted content
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.query.filter = Some(filter.into());
        self
    }

    /// Sends the request and returns the draft posts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<DraftsResponse> {
        let path = format!("blog/{}/posts/draft", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Response from the drafts endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftsResponse {
    /// List of draft posts
    pub posts: Vec<Post>,
}

/// Response from the posts endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostsResponse {
    /// List of posts
    pub posts: Vec<Post>,
    /// Total number of posts in the blog
    #[serde(default)]
    pub total_posts: u64,
    /// Information about the blog (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<Blog>,
}

/// Represents a Tumblr post
///
/// Note: This is a simplified representation. The full post structure
/// varies significantly based on post type and format (legacy vs NPF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    /// Post ID
    // posts have an id and id_string field.
    // I'm opting to grab the ID string and treat that as the ID instead of grabbing the integer value.
    #[serde(rename(deserialize = "id_string"))]
    pub id: String,
    /// ID of the post this is reblogged from (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_from_id: Option<String>,
    /// URL of the post this is reblogged from (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_from_url: Option<String>,
    /// Name of the blog that created this post
    pub blog_name: String,
    /// URL of the post
    pub post_url: String,
    /// Type of post (text, photo, quote, link, chat, audio, video, answer)
    #[serde(rename = "type")]
    pub post_type: String,
    /// Timestamp when the post was created (Unix time)
    pub timestamp: i64,
    /// Post tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Short text summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Note count (reblogs + likes)
    #[serde(default)]
    pub note_count: u64,
    /// Current state (published, queued, draft, private)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Serialize, Default)]
struct BlocksQuery {
    limit: Option<u32>,
    offset: Option<u64>,
}

pub struct BlocksBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: BlocksQuery,
}

impl BlocksBuilder {
    pub fn new(client: Crabrave, identifier: impl Into<BlogIdentifier>) -> Self {
        Self {
            client,
            identifier: identifier.into(),
            query: BlocksQuery::default(),
        }
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    pub async fn send(self) -> CrabResult<BlocksResponse> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedBlog {
    pub name: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub uuid: String,
    pub updated: i64,
    pub blocked_timestamp: i64,
    #[serde(deserialize_with = "crate::empty_object_as_none")]
    pub tumblrmart_accessories: Option<TumblrmartAccessories>,
    pub can_show_badges: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocksResponse {
    pub blocked_tumblelogs: Vec<BlockedBlog>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum BlockBlogRequest {
    Blog { blocked_tumblelog: String },
    Post { post_id: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockBlogRespsone {
    pub already_blocked: bool,
}

#[derive(Debug, Clone, Serialize)]
struct BulkBlockRequest {
    blocked_tumblelogs: String,
    force: bool,
}

pub struct FollowersBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: FollowersQuery,
}

#[derive(Serialize, Default)]
struct FollowersQuery {
    limit: Option<u32>,
    offset: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FollowersResponse {
    pub total_users: u64,
    pub users: Vec<FollowerUser>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FollowerUser {
    pub name: String,
    pub url: String,
    pub following: bool,
    pub updated: i64,
}

impl FollowersBuilder {
    pub fn new(client: Crabrave, identifier: impl Into<BlogIdentifier>) -> Self {
        Self {
            client,
            identifier: identifier.into(),
            query: FollowersQuery::default(),
        }
    }

    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    pub async fn send(self) -> CrabResult<FollowersResponse> {
        let path = format!("blog/{}/followers", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posts_builder_path_no_params() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let identifier = BlogIdentifier::from("staff");
        let builder = PostsBuilder::new(client, identifier);

        // We can't easily test the async send(), but we can verify the builder constructs correctly
        assert!(builder.query.limit.is_none());
        assert!(builder.query.offset.is_none());
    }

    #[test]
    fn test_posts_builder_with_params() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let identifier = BlogIdentifier::from("staff");
        let builder = PostsBuilder::new(client, identifier)
            .limit(10)
            .offset(20)
            .post_type("photo")
            .tag("art");

        assert_eq!(builder.query.limit, Some(10));
        assert_eq!(builder.query.offset, Some(20));
        assert_eq!(builder.query.post_type, Some("photo".to_string()));
        assert_eq!(builder.query.tag, Some("art".to_string()));
    }
}
