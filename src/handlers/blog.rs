//! Blog-related API endpoints

use crate::{
    Blog, BlogIdentifier, CrabResult, Crabrave, EmptyResponse,
    handlers::{following::FollowingBuilder, likes::LikesBuilder},
    models::TumblrmartAccessories,
    npf::{ContentBlock, LayoutBlock},
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
    ) -> CrabResult<BlockBlogResponse> {
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
    ) -> CrabResult<BlockBlogResponse> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());
        let post_id = post_id.into();
        self.client
            .post(&path, &BlockBlogRequest::Post { post_id })
            .await
    }

    pub async fn bulk_block(
        &self,
        blogs: Vec<impl Into<String>>,
        force: bool,
    ) -> CrabResult<EmptyResponse> {
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
    }

    /// Unblocks a specific blog
    ///
    /// Removes a block on the specified blog, allowing them to interact with your blog again.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog to unblock, specified by any blog identifier
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
    /// // Unblock a specific blog
    /// crab.blogs("my-blog").unblock("annoying-blog").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unblock(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<EmptyResponse> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());
        let blog_id: BlogIdentifier = blog.into();

        self.client
            .delete_with_query(
                &path,
                &serde_json::json!({ "blocked_tumblelog": blog_id.as_str() }),
            )
            .await
    }

    /// Clears all anonymous IP blocks
    ///
    /// Removes all blocks on anonymous users (users who interacted without being logged in).
    /// This is a bulk operation that clears ALL anonymous blocks at once.
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
    /// // Clear all anonymous blocks
    /// crab.blogs("my-blog").unblock_all_anonymous().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unblock_all_anonymous(&self) -> CrabResult<EmptyResponse> {
        let path = format!("blog/{}/blocks", self.identifier.as_str());

        self.client
            .delete_with_query(&path, &serde_json::json!({ "anonymous_only": true }))
            .await
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

        let path = format!("blog/{}/followed_by", self.identifier.as_str());
        let resp: FollowedByResponse = self
            .client
            .get_with_query(&path, &[("query", blog_name.to_string())])
            .await?;
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

    /// Gets submission posts for the blog
    ///
    /// Returns a builder for configuring the submissions request.
    /// Requires OAuth authentication.
    ///
    /// Submissions are posts that have been submitted to your blog by other users
    /// for your consideration before publishing.
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
    /// let submissions = crab.blogs("my-blog")
    ///     .submissions()
    ///     .send()
    ///     .await?;
    ///
    /// for post in submissions.posts {
    ///     if let Some(author) = &post.post_author {
    ///         println!("Submission from: {}", author);
    ///     } else if let Some(anon_name) = &post.anonymous_name {
    ///         println!("Anonymous submission from: {}", anon_name);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn submissions(&self) -> SubmissionBuilder {
        SubmissionBuilder::new(self.client.clone(), self.identifier.clone())
    }

    /// Gets notifications for the blog
    ///
    /// Returns a builder for configuring the notifications request.
    /// Requires OAuth authentication.
    ///
    /// Notifications include likes, reblogs, follows, mentions, asks, and more.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::blog::NotificationType;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// // Get all notifications
    /// let all_notifs = crab.blogs("my-blog")
    ///     .notifications()
    ///     .send()
    ///     .await?;
    ///
    /// // Get only likes and reblogs
    /// let filtered = crab.blogs("my-blog")
    ///     .notifications()
    ///     .types(vec![NotificationType::Like, NotificationType::ReblogNaked])
    ///     .rollups(false)
    ///     .send()
    ///     .await?;
    ///
    /// for notif in filtered.notifications {
    ///     println!("{} notification from {:?}", notif.notification_type, notif.from_tumblelog_name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn notifications(&self) -> NotificationsBuilder {
        NotificationsBuilder::new(self.client.clone(), self.identifier.clone())
    }

    /// Gets notes (interactions) for a specific post on the blog
    ///
    /// Returns a builder for configuring the notes request.
    /// Requires API key authentication.
    ///
    /// Notes include likes, reblogs, and replies on a specific post.
    /// Different modes allow filtering the type of notes returned.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to fetch notes for
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::blog::NoteMode;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .build()?;
    /// // Get all notes on a post
    /// let notes = crab.blogs("my-blog")
    ///     .notes("123456789")
    ///     .send()
    ///     .await?;
    ///
    /// println!("Total notes: {}", notes.total_notes);
    ///
    /// // Get only likes
    /// let likes = crab.blogs("my-blog")
    ///     .notes("123456789")
    ///     .mode(NoteMode::Likes)
    ///     .send()
    ///     .await?;
    ///
    /// for note in likes.notes {
    ///     println!("{} liked this post", note.blog_name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn notes(&self, post_id: impl Into<String>) -> NotesBuilder {
        NotesBuilder::new(self.client.clone(), self.identifier.clone(), post_id)
    }

    /// Gets custom pages for the blog
    ///
    /// Returns a builder for configuring the pages request.
    /// Requires OAuth authentication.
    ///
    /// Pages are static content sections on a blog, distinct from regular posts.
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
    /// let pages = crab.blogs("my-blog")
    ///     .pages()
    ///     .limit(10)
    ///     .send()
    ///     .await?;
    ///
    /// for page in pages.pages {
    ///     println!("Page: {} - {}", page.title, page.url);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn pages(&self) -> PagesBuilder {
        PagesBuilder::new(self.client.clone(), self.identifier.clone())
    }

    /// Gets a specific custom page by name
    ///
    /// Requires OAuth authentication.
    ///
    /// # Arguments
    ///
    /// * `page_name` - The name/slug of the page to retrieve (e.g., "about", "contact")
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
    /// let page = crab.blogs("my-blog")
    ///     .page("about")
    ///     .await?;
    ///
    /// println!("Page: {} - {}", page.page.title, page.page.url);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn page(&self, page_name: impl AsRef<str>) -> CrabResult<SinglePageResponse> {
        let path = format!(
            "blog/{}/pages/{}",
            self.identifier.as_str(),
            page_name.as_ref()
        );
        self.client.get(&path).await
    }

    /// Accesses operations for a specific post
    ///
    /// Returns a handler for performing operations on an individual post,
    /// such as fetching, editing, deleting, or muting.
    ///
    /// # Arguments
    ///
    /// * `id` - Post ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// // Get a post
    /// let post = crab.blogs("staff").post("123456789").get().await?;
    ///
    /// // Delete a post
    /// crab.blogs("my-blog").post("123456").delete().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn post(&self, id: impl Into<String>) -> BlogPost {
        BlogPost::new(self.client.clone(), self.identifier.clone(), id.into())
    }

    /// Creates a new post on this blog
    ///
    /// Returns a builder for creating a post using NPF (Neue Post Format).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::npf::ContentBlock;
    /// # use crabrave::media::MediaSource;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// let post = crab.blogs("my-blog")
    ///     .create_post()
    ///     .add_block(ContentBlock::text("Hello World!"))
    ///     .add_image(MediaSource::from_path("/path/to/image.jpg"))
    ///     .tags(vec!["rust", "programming"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_post(&self) -> crate::handlers::posts::CreatePostBuilder {
        crate::handlers::posts::CreatePostBuilder::new(self.client.clone(), self.identifier.clone())
    }

    /// Reblogs a post to this blog
    ///
    /// Returns a builder for reblogging a post with optional comments and tags.
    ///
    /// # Arguments
    ///
    /// * `parent_tumblelog_uuid` - UUID of the blog that owns the post being reblogged
    /// * `id` - Post ID to reblog
    /// * `reblog_key` - Reblog key from the original post
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
    /// crab.blogs("my-blog")
    ///     .reblog("parent-blog-uuid", "123456", "reblogkey")
    ///     .comment("Great post!")
    ///     .tags(vec!["reblog"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn reblog(
        &self,
        parent_tumblelog_uuid: impl Into<String>,
        id: impl Into<String>,
        reblog_key: impl Into<String>,
    ) -> crate::handlers::posts::ReblogBuilder {
        crate::handlers::posts::ReblogBuilder::new(
            self.client.clone(),
            self.identifier.clone(),
            parent_tumblelog_uuid.into(),
            id.into(),
            reblog_key.into(),
        )
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

    /// Return posts after this timestamp (Unix time)
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<i64>,

    /// Return posts in NPF format.
    #[serde(skip_serializing_if = "Option::is_none")]
    npf: Option<bool>,
}

/// Builder for querying blog posts
///
/// This builder allows you to configure various parameters for fetching posts
/// from a blog before sending the request.
///
/// Posts are always returned in NPF (Neue Post Format) with structured content blocks.
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

    /// Returns posts before this timestamp (Unix time)
    pub fn after(mut self, timestamp: i64) -> Self {
        self.query.after = Some(timestamp);
        self
    }

    /// Sends the request and returns the posts in NPF format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(mut self) -> CrabResult<PostsResponse> {
        // Always request NPF format
        self.query.npf = Some(true);
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

    /// Return posts in NPF format
    #[serde(skip_serializing_if = "Option::is_none")]
    npf: Option<bool>,
}

/// Builder for querying a blog's post queue
///
/// This builder allows you to configure various parameters for fetching queued posts
/// from a blog before sending the request.
///
/// Posts are always returned in NPF (Neue Post Format) with structured content blocks.
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

    /// Sends the request and returns the queued posts in NPF format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(mut self) -> CrabResult<QueueResponse> {
        // Always request NPF format
        self.query.npf = Some(true);
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

    /// Return posts in NPF format
    #[serde(skip_serializing_if = "Option::is_none")]
    npf: Option<bool>,
}

/// Builder for querying a blog's draft posts
///
/// This builder allows you to configure various parameters for fetching draft posts
/// from a blog before sending the request.
///
/// Posts are always returned in NPF (Neue Post Format) with structured content blocks.
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

    /// Sends the request and returns the draft posts in NPF format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(mut self) -> CrabResult<DraftsResponse> {
        // Always request NPF format
        self.query.npf = Some(true);
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

/// Query parameters for fetching submission posts
#[derive(Debug, Clone, Serialize, Default)]
struct SubmissionQuery {
    /// Post number to start at (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,

    /// Response format filter: "text" for plain text, "raw" for user-entered format
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<String>,

    /// Return posts in NPF format
    #[serde(skip_serializing_if = "Option::is_none")]
    npf: Option<bool>,
}

/// Builder for querying a blog's submission posts
///
/// This builder allows you to configure various parameters for fetching submission posts
/// from a blog before sending the request.
///
/// Posts are always returned in NPF (Neue Post Format) with structured content blocks.
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
/// let submissions = crab.blogs("my-blog")
///     .submissions()
///     .offset(10)
///     .send()
///     .await?;
///
/// for post in submissions.posts {
///     println!("Submission from: {:?}", post.post_author);
/// }
/// # Ok(())
/// # }
/// ```
pub struct SubmissionBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: SubmissionQuery,
}

impl SubmissionBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self {
            client,
            identifier,
            query: SubmissionQuery::default(),
        }
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

    /// Sends the request and returns the submission posts in NPF format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing (OAuth required)
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(mut self) -> CrabResult<SubmissionResponse> {
        // Always request NPF format
        self.query.npf = Some(true);
        let path = format!("blog/{}/posts/submission", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Response from the submissions endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionResponse {
    /// List of submission posts
    pub posts: Vec<Post>,
}

/// Types of notifications that can be filtered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Like,
    Reply,
    Follow,
    MentionInReply,
    MentionInPost,
    ReblogNaked,
    ReblogWithContent,
    Ask,
    AnsweredAsk,
    NewGroupBlogMember,
    PostAttribution,
    PostFlagged,
    PostAppealAccepted,
    PostAppealRejected,
    WhatYouMissed,
    ConversationalNote,
}

impl NotificationType {
    /// Returns the string representation used in API requests
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Like => "like",
            NotificationType::Reply => "reply",
            NotificationType::Follow => "follow",
            NotificationType::MentionInReply => "mention_in_reply",
            NotificationType::MentionInPost => "mention_in_post",
            NotificationType::ReblogNaked => "reblog_naked",
            NotificationType::ReblogWithContent => "reblog_with_content",
            NotificationType::Ask => "ask",
            NotificationType::AnsweredAsk => "answered_ask",
            NotificationType::NewGroupBlogMember => "new_group_blog_member",
            NotificationType::PostAttribution => "post_attribution",
            NotificationType::PostFlagged => "post_flagged",
            NotificationType::PostAppealAccepted => "post_appeal_accepted",
            NotificationType::PostAppealRejected => "post_appeal_rejected",
            NotificationType::WhatYouMissed => "what_you_missed",
            NotificationType::ConversationalNote => "conversational_note",
        }
    }
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a Tumblr notification/activity item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Unique identifier for this notification
    pub id: String,
    /// Type of notification (like, reblog, follow, etc.)
    #[serde(rename = "type")]
    pub notification_type: String,
    /// Unix epoch timestamp when the notification occurred
    pub timestamp: i64,
    /// Whether this notification is unread
    #[serde(default)]
    pub unread: bool,
    /// The post ID this notification is about (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_post_id: Option<String>,
    /// The blog name that triggered this notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_tumblelog_name: Option<String>,
    /// Additional context about the notification (varies by type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_tumblelog_name: Option<String>,
    /// Media objects associated with the notification
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<serde_json::Value>,
    /// Summary text for the notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// Pagination links for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationLinks {
    /// Link to fetch the next page of notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<NotificationNextLink>,
}

/// Next page link structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationNextLink {
    /// The href for the next page (contains query parameters)
    pub href: String,
}

/// Response from the notifications endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsResponse {
    /// List of notifications
    pub notifications: Vec<Notification>,
    /// Pagination links
    #[serde(rename = "_links", default, skip_serializing_if = "Option::is_none")]
    pub links: Option<NotificationLinks>,
}

/// Query parameters for fetching notifications
#[derive(Debug, Clone, Serialize, Default)]
struct NotificationsQuery {
    /// Return notifications before this Unix epoch timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<i64>,

    /// Whether to consolidate similar activity items (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    rollups: Option<bool>,

    /// Filter by notification types (comma-separated in serialization)
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_types_array"
    )]
    types: Option<Vec<String>>,

    /// Post IDs to exclude from results (comma-separated in serialization)
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_string_array"
    )]
    omit_post_ids: Option<Vec<String>>,
}

fn serialize_types_array<S>(value: &Option<Vec<String>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(&v.join(",")),
        None => serializer.serialize_none(),
    }
}

fn serialize_string_array<S>(value: &Option<Vec<String>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(v) => serializer.serialize_str(&v.join(",")),
        None => serializer.serialize_none(),
    }
}

/// Builder for querying a blog's notifications
///
/// This builder allows you to configure various parameters for fetching notifications
/// from a blog before sending the request.
///
/// # Example
///
/// ```no_run
/// # use crabrave::Crabrave;
/// # use crabrave::handlers::blog::NotificationType;
/// # async fn example() -> Result<(), crabrave::CrabError> {
/// # let crab = Crabrave::builder()
/// #     .consumer_key("key")
/// #     .consumer_secret("secret")
/// #     .access_token("token")
/// #     .build()?;
/// let notifications = crab.blogs("my-blog")
///     .notifications()
///     .types(vec![NotificationType::Like, NotificationType::ReblogNaked])
///     .send()
///     .await?;
///
/// for notif in notifications.notifications {
///     println!("{}: {} from {:?}", notif.notification_type, notif.id, notif.from_tumblelog_name);
/// }
/// # Ok(())
/// # }
/// ```
pub struct NotificationsBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: NotificationsQuery,
}

impl NotificationsBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self {
            client,
            identifier,
            query: NotificationsQuery::default(),
        }
    }

    /// Return notifications before this Unix epoch timestamp
    ///
    /// Use this for pagination through notifications.
    pub fn before(mut self, timestamp: i64) -> Self {
        self.query.before = Some(timestamp);
        self
    }

    /// Whether to consolidate similar activity items
    ///
    /// When true (the default), similar notifications may be grouped together.
    /// Set to false to get individual notifications.
    pub fn rollups(mut self, rollups: bool) -> Self {
        self.query.rollups = Some(rollups);
        self
    }

    /// Filter notifications by type
    ///
    /// Only return notifications of the specified types.
    pub fn types(mut self, types: Vec<NotificationType>) -> Self {
        self.query.types = Some(types.iter().map(|t| t.to_string()).collect());
        self
    }

    /// Filter notifications by type using string values
    ///
    /// Only return notifications of the specified types.
    /// Use this if you need to specify types not covered by NotificationType enum.
    pub fn types_str(mut self, types: Vec<impl Into<String>>) -> Self {
        self.query.types = Some(types.into_iter().map(|t| t.into()).collect());
        self
    }

    /// Exclude notifications about specific posts
    ///
    /// Notifications about these post IDs will not be returned.
    pub fn omit_post_ids(mut self, post_ids: Vec<impl Into<String>>) -> Self {
        self.query.omit_post_ids = Some(post_ids.into_iter().map(|id| id.into()).collect());
        self
    }

    /// Sends the request and returns the notifications
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing (OAuth required)
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<NotificationsResponse> {
        let path = format!("blog/{}/notifications", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Response formatting modes for the notes endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoteMode {
    /// Returns all notes in reverse chronological order (default)
    #[default]
    All,
    /// Returns only like interactions
    Likes,
    /// Returns replies and reblogs with commentary; other notes in rollup_notes
    Conversation,
    /// Returns only likes and reblogs
    Rollup,
    /// Returns only reblog notes, each including a tags array
    ReblogsWithTags,
}

impl NoteMode {
    /// Returns the string representation used in API requests
    pub fn as_str(&self) -> &'static str {
        match self {
            NoteMode::All => "all",
            NoteMode::Likes => "likes",
            NoteMode::Conversation => "conversation",
            NoteMode::Rollup => "rollup",
            NoteMode::ReblogsWithTags => "reblogs_with_tags",
        }
    }
}

impl std::fmt::Display for NoteMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Represents a note (interaction) on a Tumblr post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    /// Type of interaction (reblog, like, reply, posted, etc.)
    #[serde(rename = "type")]
    pub note_type: String,
    /// Unix epoch timestamp of the interaction
    pub timestamp: i64,
    /// Name of the interacting blog
    pub blog_name: String,
    /// Unique identifier of the interacting blog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog_uuid: Option<String>,
    /// URL of the interacting blog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog_url: Option<String>,
    /// Whether the requesting user follows this blog
    #[serde(default)]
    pub followed: bool,
    /// Shape of the blog's avatar (square or circle)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_shape: Option<String>,
    /// Post ID for reblog notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_id: Option<String>,
    /// Reblog parent blog name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblog_parent_blog_name: Option<String>,
    /// Reply text for reply notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_text: Option<String>,
    /// Formatted reply text (HTML)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatting: Option<Vec<serde_json::Value>>,
    /// Whether this note is from the original poster
    #[serde(default)]
    pub can_block: bool,
    /// Tags added in reblog (for reblogs_with_tags mode)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Added text in reblog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_text: Option<String>,
}

/// Pagination links for notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLinks {
    /// Link to fetch the next page of notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<NoteNextLink>,
}

/// Next page link structure for notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteNextLink {
    /// Query parameters for the next page
    pub query_params: NoteNextQueryParams,
}

/// Query parameters embedded in next link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteNextQueryParams {
    /// Post ID
    pub id: String,
    /// Mode for the next request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Timestamp before which to fetch notes
    pub before_timestamp: i64,
}

/// Response from the notes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotesResponse {
    /// List of notes
    pub notes: Vec<Note>,
    /// Notes excluded from primary array (conversation mode only)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rollup_notes: Vec<Note>,
    /// Total note count
    #[serde(default)]
    pub total_notes: u64,
    /// Total likes count (conversation mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_likes: Option<u64>,
    /// Total reblogs count (conversation mode)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_reblogs: Option<u64>,
    /// Pagination links
    #[serde(rename = "_links", default, skip_serializing_if = "Option::is_none")]
    pub links: Option<NoteLinks>,
}

/// Query parameters for fetching notes
#[derive(Debug, Clone, Serialize, Default)]
struct NotesQuery {
    /// Post ID to fetch notes for (required)
    id: String,

    /// Response formatting mode
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,

    /// Fetch notes created before this timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    before_timestamp: Option<i64>,
}

/// Builder for querying notes on a blog post
///
/// This builder allows you to configure various parameters for fetching notes
/// (interactions like likes, reblogs, and replies) on a specific post.
///
/// # Example
///
/// ```no_run
/// # use crabrave::Crabrave;
/// # use crabrave::handlers::blog::NoteMode;
/// # async fn example() -> Result<(), crabrave::CrabError> {
/// # let crab = Crabrave::builder()
/// #     .consumer_key("key")
/// #     .build()?;
/// // Get all notes on a post
/// let notes = crab.blogs("my-blog")
///     .notes("123456789")
///     .send()
///     .await?;
///
/// println!("Total notes: {}", notes.total_notes);
///
/// // Get only likes
/// let likes = crab.blogs("my-blog")
///     .notes("123456789")
///     .mode(NoteMode::Likes)
///     .send()
///     .await?;
///
/// // Get reblogs with their tags
/// let reblogs = crab.blogs("my-blog")
///     .notes("123456789")
///     .mode(NoteMode::ReblogsWithTags)
///     .send()
///     .await?;
///
/// for note in reblogs.notes {
///     println!("{} reblogged with tags: {:?}", note.blog_name, note.tags);
/// }
/// # Ok(())
/// # }
/// ```
pub struct NotesBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: NotesQuery,
}

impl NotesBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier, post_id: impl Into<String>) -> Self {
        Self {
            client,
            identifier,
            query: NotesQuery {
                id: post_id.into(),
                mode: None,
                before_timestamp: None,
            },
        }
    }

    /// Sets the response formatting mode
    ///
    /// - `All` (default): Returns all notes in reverse chronological order
    /// - `Likes`: Returns only like interactions
    /// - `Conversation`: Returns replies and reblogs with commentary
    /// - `Rollup`: Returns only likes and reblogs
    /// - `ReblogsWithTags`: Returns only reblogs, each including tags
    pub fn mode(mut self, mode: NoteMode) -> Self {
        self.query.mode = Some(mode.to_string());
        self
    }

    /// Fetch notes created before this Unix timestamp
    ///
    /// Use this for pagination through notes. In "conversation" mode,
    /// this should be in microseconds; otherwise in seconds.
    pub fn before_timestamp(mut self, timestamp: i64) -> Self {
        self.query.before_timestamp = Some(timestamp);
        self
    }

    /// Sends the request and returns the notes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The post doesn't exist
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<NotesResponse> {
        let path = format!("blog/{}/notes", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Query parameters for fetching blog pages
#[derive(Debug, Clone, Serialize, Default)]
struct PagesQuery {
    /// Maximum number of pages to return (1-20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Page offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
}

/// Builder for querying a blog's custom pages
///
/// This builder allows you to configure various parameters for fetching pages
/// from a blog before sending the request.
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
/// let pages = crab.blogs("my-blog")
///     .pages()
///     .limit(10)
///     .offset(5)
///     .send()
///     .await?;
///
/// for page in pages.pages {
///     println!("Page: {} at {}", page.title, page.url);
/// }
/// # Ok(())
/// # }
/// ```
pub struct PagesBuilder {
    client: Crabrave,
    identifier: BlogIdentifier,
    query: PagesQuery,
}

impl PagesBuilder {
    fn new(client: Crabrave, identifier: BlogIdentifier) -> Self {
        Self {
            client,
            identifier,
            query: PagesQuery::default(),
        }
    }

    /// Sets the number of pages to return (1-20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Sets the page offset for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Sends the request and returns the pages
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid or missing (OAuth required)
    /// - The blog doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<PagesResponse> {
        let path = format!("blog/{}/pages", self.identifier.as_str());
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Response from the pages endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagesResponse {
    /// List of custom pages
    pub pages: Vec<BlogPage>,
}

/// Response from the single page endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinglePageResponse {
    /// The requested page
    pub page: BlogPage,
}

/// Represents a custom page on a Tumblr blog
///
/// Pages are static content sections distinct from regular posts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPage {
    /// Page title
    pub title: String,
    /// Page body content (HTML)
    pub body: String,
    /// Full URL to the page
    pub url: String,
    /// Unix timestamp when the page was last updated
    #[serde(default)]
    pub updated: i64,
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

/// Represents a Tumblr post in NPF (Neue Post Format)
///
/// This struct captures all fields from the Tumblr API response when using NPF mode.
/// NPF provides structured content blocks instead of legacy HTML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    // === Identification ===
    /// Post ID (uses id_string from API for precision with large IDs)
    #[serde(rename(deserialize = "id_string"))]
    pub id: String,
    /// Name of the blog that created this post
    pub blog_name: String,
    /// URL of the post
    pub post_url: String,
    /// Type of post (always "blocks" in NPF mode)
    #[serde(rename = "type")]
    pub post_type: String,
    /// Original post type before NPF conversion ("regular", "photo", etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_type: Option<String>,
    /// Blog UUID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tumblelog_uuid: Option<String>,
    /// Object type (usually "post")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,

    // === Timestamps ===
    /// Timestamp when the post was created (Unix time)
    pub timestamp: i64,
    /// Human-readable date string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Scheduled publish time (Unix timestamp, for queued posts)
    #[serde(default)]
    pub scheduled_publish_time: i64,
    /// Human-readable publish time (for queued posts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_on: Option<String>,

    // === NPF Content ===
    /// Content blocks (NPF format)
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    /// Layout blocks (NPF format)
    #[serde(default)]
    pub layout: Vec<LayoutBlock>,
    /// Reblog trail
    #[serde(default)]
    pub trail: Vec<TrailItem>,

    // === Metadata ===
    /// Post tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Short text summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Note count (reblogs + likes)
    #[serde(default)]
    pub note_count: u64,
    /// URL slug
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Short URL (tmblr.co)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_url: Option<String>,
    /// Reblog key (needed for reblogging this post)
    #[serde(default)]
    pub reblog_key: String,
    /// Current state (published, queued, draft, private, submission)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Queued state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queued_state: Option<String>,
    /// Whether this post uses NPF (blocks) format
    #[serde(default)]
    pub is_blocks_post_format: bool,
    /// Community Labels (mature-content advisory) applied to this post, if any.
    ///
    /// Typically `None`: Tumblr only returns this object to first-party clients,
    /// so third-party API credentials receive posts without it. See [`CommunityLabels`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_labels: Option<CommunityLabels>,

    // === Blog info ===
    /// Embedded blog information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<Blog>,

    // === Reblog chain ===
    /// ID of the parent post (direct reblog source)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_post_id: Option<String>,
    /// UUID of the parent blog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tumblelog_uuid: Option<String>,
    /// URL of the parent post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_post_url: Option<String>,
    /// ID of the post this is reblogged from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_from_id: Option<String>,
    /// URL of the post this is reblogged from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_from_url: Option<String>,
    /// Name of the blog this was reblogged from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_from_name: Option<String>,
    /// ID of the root post in the reblog chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_root_id: Option<String>,
    /// URL of the root post in the reblog chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_root_url: Option<String>,
    /// Name of the original poster
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reblogged_root_name: Option<String>,

    // === User relationship ===
    /// Whether the authenticated user follows this blog
    #[serde(default)]
    pub followed: bool,
    /// Whether the authenticated user has liked this post
    #[serde(default)]
    pub liked: bool,

    // === Interaction permissions ===
    /// Whether the user can like this post
    #[serde(default)]
    pub can_like: bool,
    /// Whether the user can reblog this post
    #[serde(default)]
    pub can_reblog: bool,
    /// Whether the user can reply to this post
    #[serde(default)]
    pub can_reply: bool,
    /// Whether the user can send this post in a message
    #[serde(default)]
    pub can_send_in_message: bool,
    /// Whether the user can mute notifications for this post
    #[serde(default)]
    pub can_mute: bool,
    /// Whether to display the blog's avatar
    #[serde(default)]
    pub display_avatar: bool,
    /// Reblog interactability setting
    #[serde(default)]
    pub interactability_reblog: String,

    // === Blaze/promotion ===
    /// Whether this post is currently blazed
    #[serde(default)]
    pub is_blazed: bool,
    /// Whether there's a pending blaze for this post
    #[serde(default)]
    pub is_blaze_pending: bool,
    /// Whether this post can be ignited (boosted)
    #[serde(default)]
    pub can_ignite: bool,
    /// Whether this post can be blazed
    #[serde(default)]
    pub can_blaze: bool,

    // === Mute state ===
    /// Whether notifications for this post are muted
    #[serde(default)]
    pub muted: bool,
    /// Timestamp when the mute ends (0 if not muted or muted indefinitely)
    #[serde(default)]
    pub mute_end_timestamp: i64,

    // === Submission info ===
    /// Author of the submission (only present for submission posts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_author: Option<String>,
    /// Whether this is a submission post
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_submission: Option<bool>,
    /// Name of anonymous submitter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous_name: Option<String>,
    /// Email of anonymous submitter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous_email: Option<String>,
}

/// Tumblr Community Labels applied to a post — the mature-content advisory system
/// introduced in December 2022 (`mature` umbrella plus `sexual_themes`, `violence`,
/// and `drug_use` subcategories).
///
/// Undocumented in the public API reference. The object was observed live only in
/// Tumblr's first-party web frontend, where it is camelCased; the snake_case keys used
/// here are inferred from Tumblr's otherwise-uniform API naming. Tumblr returns this
/// object only to first-party clients, so posts fetched with third-party API credentials
/// arrive without it and [`Post::community_labels`] deserializes to `None`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommunityLabels {
    /// Whether the post carries any community label. When `true` with an empty
    /// `categories`, the post is flagged under the generic `mature` label.
    #[serde(default)]
    pub has_community_label: bool,
    /// Advisory subcategories, e.g. `sexual_themes`, `violence`, `drug_use`. Kept as
    /// strings so categories Tumblr may add later cannot break deserialization.
    #[serde(default)]
    pub categories: Vec<String>,
    /// What applied the label: `author` (self-flagged) or `classifier` (automated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_reporter: Option<String>,
}

/// Item in a reblog trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailItem {
    /// Content blocks in this trail item (NPF format)
    /// Note: Legacy posts may have this as a string; we skip those
    #[serde(default, deserialize_with = "crate::deserialize_content_blocks")]
    pub content: Vec<ContentBlock>,
    /// Raw HTML content (legacy format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_raw: Option<String>,
    /// Layout blocks for this trail item
    #[serde(default)]
    pub layout: Vec<LayoutBlock>,
    /// Post information for this trail item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<TrailPost>,
    /// Blog information for this trail item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<TrailBlog>,
    /// Whether this trail item is the root post
    #[serde(default)]
    pub is_root_item: bool,
}

/// Post reference in a trail item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailPost {
    /// Post ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Blog reference in a trail item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailBlog {
    /// Blog name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Blog URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Blog UUID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
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
pub struct BlockBlogResponse {
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

/// Handler for operations on a specific blog post
///
/// Provides methods for fetching, editing, deleting, and muting an individual post.
/// Created by calling `crab.blogs("blog-name").post("post-id")`.
#[derive(Clone)]
pub struct BlogPost {
    client: Crabrave,
    blog: BlogIdentifier,
    id: String,
}

impl BlogPost {
    pub(crate) fn new(client: Crabrave, blog: BlogIdentifier, id: String) -> Self {
        Self { client, blog, id }
    }

    /// Fetches this post
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let post = crab.blogs("staff").post("123456789").get().await?;
    /// println!("Post ID: {}", post.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(self) -> CrabResult<Post> {
        let path = format!("blog/{}/posts/{}", self.blog.as_str(), self.id);
        self.client.get(&path).await
    }

    /// Deletes this post
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
    /// crab.blogs("my-blog").post("123456").delete().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(self) -> CrabResult<crate::handlers::posts::DeleteResponse> {
        let path = format!("blog/{}/post/delete", self.blog.as_str());
        self.client
            .post(&path, &serde_json::json!({ "id": self.id }))
            .await
    }

    /// Edits this post
    ///
    /// Returns a builder for editing the post using NPF (Neue Post Format).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::npf::ContentBlock;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// let edited = crab.blogs("my-blog")
    ///     .post("123456")
    ///     .edit()
    ///     .content(vec![
    ///         ContentBlock::heading("Updated Title", 1),
    ///         ContentBlock::text("Updated content!"),
    ///     ])
    ///     .tags(vec!["updated"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn edit(self) -> crate::handlers::posts::EditPostBuilder {
        crate::handlers::posts::EditPostBuilder::new(self.client, self.blog, self.id)
    }

    /// Mutes notifications for this post
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
    /// let response = crab.blogs("my-blog").post("123456").mute().await?;
    /// println!("Post muted: {}", response.muted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mute(self) -> CrabResult<crate::handlers::posts::MuteResponse> {
        let path = format!("blog/{}/posts/{}/mute", self.blog.as_str(), self.id);
        self.client.post(&path, &serde_json::json!({})).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_posts_builder_path_no_params() {
        let client = Crabrave::builder()
            .consumer_key("test")
            .consumer_secret("test")
            .access_token("test")
            .build()
            .unwrap();
        let identifier = BlogIdentifier::from("staff");
        let builder = PostsBuilder::new(client, identifier);

        // We can't easily test the async send(), but we can verify the builder constructs correctly
        assert!(builder.query.limit.is_none());
        assert!(builder.query.offset.is_none());
    }

    #[test]
    fn test_posts_builder_with_params() {
        let client = Crabrave::builder()
            .consumer_key("test")
            .consumer_secret("test")
            .access_token("test")
            .build()
            .unwrap();
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

    // The minimal set of non-defaulted `Post` fields; community labels are layered on top.
    const BASE_POST_FIELDS: &str = r#"
        "id_string": "820735101718626304",
        "blog_name": "rydengg",
        "post_url": "https://rydengg.tumblr.com/post/820735101718626304",
        "type": "blocks",
        "timestamp": 1782713987
    "#;

    #[test]
    fn test_post_deserializes_community_labels_with_categories() {
        let json = format!(
            r#"{{ {BASE_POST_FIELDS}, "community_labels": {{
                "has_community_label": true,
                "categories": ["sexual_themes", "drug_use"],
                "last_reporter": "author"
            }} }}"#
        );
        let post: Post = serde_json::from_str(&json).unwrap();
        let labels = post
            .community_labels
            .expect("labeled post should carry community_labels");
        assert!(labels.has_community_label);
        assert_eq!(labels.categories, ["sexual_themes", "drug_use"]);
        assert_eq!(labels.last_reporter.as_deref(), Some("author"));
    }

    #[test]
    fn test_post_deserializes_mature_umbrella_label() {
        // The shape observed on live posts: flagged mature with no explicit subcategory.
        let json = format!(
            r#"{{ {BASE_POST_FIELDS}, "community_labels": {{
                "has_community_label": true,
                "categories": [],
                "last_reporter": "classifier"
            }} }}"#
        );
        let post: Post = serde_json::from_str(&json).unwrap();
        let labels = post.community_labels.unwrap();
        assert!(labels.has_community_label);
        assert!(labels.categories.is_empty());
    }

    #[test]
    fn test_post_without_community_labels_is_none() {
        // Regression guard: the field is absent from third-party API responses, so
        // `#[serde(default)]` must yield `None` rather than fail to deserialize.
        let json = format!(r#"{{ {BASE_POST_FIELDS} }}"#);
        let post: Post = serde_json::from_str(&json).unwrap();
        assert!(post.community_labels.is_none());
    }

    #[test]
    fn test_post_without_labels_omits_field_when_serialized() {
        // `skip_serializing_if` keeps the envelope clean for the common unlabeled case.
        let json = format!(r#"{{ {BASE_POST_FIELDS} }}"#);
        let post: Post = serde_json::from_str(&json).unwrap();
        let out = serde_json::to_string(&post).unwrap();
        assert!(!out.contains("community_labels"));
    }
}
