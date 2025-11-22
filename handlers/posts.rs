//! Post creation, editing, and deletion API endpoints

use crate::{BlogIdentifier, Crabrave, CrabResult};
use serde::{Deserialize, Serialize};

/// API for post-related operations
///
/// Provides access to creating, editing, fetching, and deleting posts.
///
/// # Example
///
/// ```no_run
/// use crabrave::Crabrave;
///
/// # async fn example() -> Result<(), crabrave::CrabError> {
/// let crab = Crabrave::builder()
///     .consumer_key("key")
///     .consumer_secret("secret")
///     .access_token("token")
///     .build()?;
///
/// // Get a specific post
/// let post = crab.posts().get("my-blog", "123456").await?;
///
/// // Create a text post
/// let new_post = crab.posts()
///     .create_text("my-blog")
///     .title("Hello World")
///     .body("This is my first post!")
///     .tags(vec!["rust", "programming"])
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Posts {
    client: Crabrave,
}

impl Posts {
    /// Creates a new Posts API
    pub(crate) fn new(client: Crabrave) -> Self {
        Self { client }
    }

    /// Gets a specific post by ID
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
    /// * `id` - Post ID
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let post = crab.posts().get("staff", "123456789").await?;
    /// println!("Post: {}", post.post.id);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The blog or post doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn get(
        &self,
        blog: impl Into<BlogIdentifier>,
        id: impl Into<String>,
    ) -> CrabResult<PostResponse> {
        let blog = blog.into();
        let id = id.into();
        let path = format!("blog/{}/posts/{}", blog.as_str(), id);
        self.client.get(&path).await
    }

    /// Deletes a post
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
    /// * `id` - Post ID
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
    /// crab.posts().delete("my-blog", "123456789").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The blog or post doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn delete(
        &self,
        blog: impl Into<BlogIdentifier>,
        id: impl Into<String>,
    ) -> CrabResult<DeleteResponse> {
        let blog = blog.into();
        let id = id.into();
        let path = format!("blog/{}/post/delete?id={}", blog.as_str(), id);
        self.client.post(&path, &serde_json::json!({})).await
    }

    /// Creates a new text post
    ///
    /// Returns a builder for configuring the text post.
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
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
    /// let post = crab.posts()
    ///     .create_text("my-blog")
    ///     .title("My Title")
    ///     .body("Post content here")
    ///     .tags(vec!["rust", "code"])
    ///     .state("published")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_text(&self, blog: impl Into<BlogIdentifier>) -> CreateTextPostBuilder {
        CreateTextPostBuilder::new(self.client.clone(), blog.into())
    }

    /// Creates a new quote post
    ///
    /// Returns a builder for configuring the quote post.
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
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
    /// let post = crab.posts()
    ///     .create_quote("my-blog")
    ///     .quote("To be or not to be")
    ///     .source("Shakespeare")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_quote(&self, blog: impl Into<BlogIdentifier>) -> CreateQuotePostBuilder {
        CreateQuotePostBuilder::new(self.client.clone(), blog.into())
    }

    /// Creates a new link post
    ///
    /// Returns a builder for configuring the link post.
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
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
    /// let post = crab.posts()
    ///     .create_link("my-blog")
    ///     .url("https://example.com")
    ///     .title("Check this out")
    ///     .description("An interesting link")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_link(&self, blog: impl Into<BlogIdentifier>) -> CreateLinkPostBuilder {
        CreateLinkPostBuilder::new(self.client.clone(), blog.into())
    }

    /// Creates a new photo post
    ///
    /// Returns a builder for configuring the photo post.
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
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
    /// let post = crab.posts()
    ///     .create_photo("my-blog")
    ///     .source("https://example.com/image.jpg")
    ///     .caption("A beautiful photo")
    ///     .tags(vec!["photo", "art"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_photo(&self, blog: impl Into<BlogIdentifier>) -> CreatePhotoPostBuilder {
        CreatePhotoPostBuilder::new(self.client.clone(), blog.into())
    }

    /// Creates a new NPF (Neue Post Format) post
    ///
    /// NPF is Tumblr's modern content block system that allows rich, mixed-media posts.
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
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
    /// let post = crab.posts()
    ///     .create_npf("my-blog")
    ///     .content(vec![
    ///         ContentBlock::heading("My Post", 1),
    ///         ContentBlock::text("This is the body of my post."),
    ///         ContentBlock::image("https://example.com/image.jpg"),
    ///     ])
    ///     .tags(vec!["npf", "modern"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_npf(&self, blog: impl Into<BlogIdentifier>) -> CreateNpfPostBuilder {
        CreateNpfPostBuilder::new(self.client.clone(), blog.into())
    }

    /// Edits an existing post
    ///
    /// Returns a builder for configuring the post edits.
    ///
    /// # Arguments
    ///
    /// * `blog` - Blog identifier (name, hostname, or UUID)
    /// * `id` - Post ID to edit
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
    /// let edited = crab.posts()
    ///     .edit("my-blog", "123456")
    ///     .title("Updated Title")
    ///     .body("Updated content")
    ///     .tags(vec!["updated", "edited"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn edit(
        &self,
        blog: impl Into<BlogIdentifier>,
        id: impl Into<String>,
    ) -> EditPostBuilder {
        EditPostBuilder::new(self.client.clone(), blog.into(), id.into())
    }

    /// Reblogs a post
    ///
    /// # Arguments
    ///
    /// * `blog` - Your blog identifier to reblog to
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
    /// crab.posts()
    ///     .reblog("my-blog", "123456", "reblogkey")
    ///     .comment("Great post!")
    ///     .tags(vec!["reblog", "interesting"])
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn reblog(
        &self,
        blog: impl Into<BlogIdentifier>,
        id: impl Into<String>,
        reblog_key: impl Into<String>,
    ) -> ReblogBuilder {
        ReblogBuilder::new(
            self.client.clone(),
            blog.into(),
            id.into(),
            reblog_key.into(),
        )
    }
}

/// Response from getting a single post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    /// The requested post
    pub post: crate::handlers::blog::Post,
}

/// Response from deleting a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    /// Post ID that was deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Builder for creating a text post
///
/// Text posts are the simplest post type, containing a title and body.
pub struct CreateTextPostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    title: Option<String>,
    body: Option<String>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
}

impl CreateTextPostBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier) -> Self {
        Self {
            client,
            blog,
            title: None,
            body: None,
            tags: Vec::new(),
            state: None,
            slug: None,
            date: None,
        }
    }

    /// Sets the title of the post
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the body content of the post (HTML allowed)
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the tags for the post
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the post state
    ///
    /// # Arguments
    ///
    /// * `state` - One of: "published", "draft", "queue", "private"
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets a custom URL slug for the post
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Sets a custom publish date (GMT timestamp)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Sends the request to create the post
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - Required fields are missing
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "type": "text",
        });

        if let Some(title) = self.title {
            body["title"] = serde_json::json!(title);
        }
        if let Some(body_text) = self.body {
            body["body"] = serde_json::json!(body_text);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }
        if let Some(slug) = self.slug {
            body["slug"] = serde_json::json!(slug);
        }
        if let Some(date) = self.date {
            body["date"] = serde_json::json!(date);
        }

        let path = format!("blog/{}/post", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

/// Response from creating a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostResponse {
    /// ID of the created post
    pub id: String,
}

/// Builder for creating a quote post
///
/// Quote posts display a quoted text with an optional source attribution.
pub struct CreateQuotePostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    quote: Option<String>,
    source: Option<String>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
}

impl CreateQuotePostBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier) -> Self {
        Self {
            client,
            blog,
            quote: None,
            source: None,
            tags: Vec::new(),
            state: None,
            slug: None,
            date: None,
        }
    }

    /// Sets the quoted text
    pub fn quote(mut self, quote: impl Into<String>) -> Self {
        self.quote = Some(quote.into());
        self
    }

    /// Sets the source of the quote (HTML allowed)
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets the tags for the post
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the post state ("published", "draft", "queue", "private")
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets a custom URL slug for the post
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Sets a custom publish date (GMT timestamp)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Sends the request to create the post
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "type": "quote",
        });

        if let Some(quote) = self.quote {
            body["quote"] = serde_json::json!(quote);
        }
        if let Some(source) = self.source {
            body["source"] = serde_json::json!(source);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }
        if let Some(slug) = self.slug {
            body["slug"] = serde_json::json!(slug);
        }
        if let Some(date) = self.date {
            body["date"] = serde_json::json!(date);
        }

        let path = format!("blog/{}/post", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

/// Builder for creating a link post
///
/// Link posts share a URL with optional title and description.
pub struct CreateLinkPostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    thumbnail: Option<String>,
    excerpt: Option<String>,
    author: Option<String>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
}

impl CreateLinkPostBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier) -> Self {
        Self {
            client,
            blog,
            url: None,
            title: None,
            description: None,
            thumbnail: None,
            excerpt: None,
            author: None,
            tags: Vec::new(),
            state: None,
            slug: None,
            date: None,
        }
    }

    /// Sets the URL to link to (required)
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the title of the link
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the description of the link (HTML allowed)
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets a thumbnail URL for the link
    pub fn thumbnail(mut self, thumbnail: impl Into<String>) -> Self {
        self.thumbnail = Some(thumbnail.into());
        self
    }

    /// Sets an excerpt from the linked page
    pub fn excerpt(mut self, excerpt: impl Into<String>) -> Self {
        self.excerpt = Some(excerpt.into());
        self
    }

    /// Sets the author of the linked content
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Sets the tags for the post
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the post state ("published", "draft", "queue", "private")
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets a custom URL slug for the post
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Sets a custom publish date (GMT timestamp)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Sends the request to create the post
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "type": "link",
        });

        if let Some(url) = self.url {
            body["url"] = serde_json::json!(url);
        }
        if let Some(title) = self.title {
            body["title"] = serde_json::json!(title);
        }
        if let Some(description) = self.description {
            body["description"] = serde_json::json!(description);
        }
        if let Some(thumbnail) = self.thumbnail {
            body["thumbnail"] = serde_json::json!(thumbnail);
        }
        if let Some(excerpt) = self.excerpt {
            body["excerpt"] = serde_json::json!(excerpt);
        }
        if let Some(author) = self.author {
            body["author"] = serde_json::json!(author);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }
        if let Some(slug) = self.slug {
            body["slug"] = serde_json::json!(slug);
        }
        if let Some(date) = self.date {
            body["date"] = serde_json::json!(date);
        }

        let path = format!("blog/{}/post", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

/// Builder for creating a photo post
///
/// Photo posts can include images from URLs or uploaded files.
pub struct CreatePhotoPostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    source: Option<String>,
    link: Option<String>,
    caption: Option<String>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
}

impl CreatePhotoPostBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier) -> Self {
        Self {
            client,
            blog,
            source: None,
            link: None,
            caption: None,
            tags: Vec::new(),
            state: None,
            slug: None,
            date: None,
        }
    }

    /// Sets the photo source URL
    ///
    /// This can be an external URL to an image that Tumblr will fetch.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Sets a clickthrough link for the photo
    pub fn link(mut self, link: impl Into<String>) -> Self {
        self.link = Some(link.into());
        self
    }

    /// Sets the caption for the photo (HTML allowed)
    pub fn caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }

    /// Sets the tags for the post
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the post state ("published", "draft", "queue", "private")
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets a custom URL slug for the post
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Sets a custom publish date (GMT timestamp)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Sends the request to create the post
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - Required fields are missing
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "type": "photo",
        });

        if let Some(source) = self.source {
            body["source"] = serde_json::json!(source);
        }
        if let Some(link) = self.link {
            body["link"] = serde_json::json!(link);
        }
        if let Some(caption) = self.caption {
            body["caption"] = serde_json::json!(caption);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }
        if let Some(slug) = self.slug {
            body["slug"] = serde_json::json!(slug);
        }
        if let Some(date) = self.date {
            body["date"] = serde_json::json!(date);
        }

        let path = format!("blog/{}/post", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

/// Builder for creating an NPF (Neue Post Format) post
///
/// NPF allows creating rich, structured posts with content blocks.
pub struct CreateNpfPostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    content: Vec<crate::npf::ContentBlock>,
    layout: Option<Vec<crate::npf::LayoutBlock>>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
}

impl CreateNpfPostBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier) -> Self {
        Self {
            client,
            blog,
            content: Vec::new(),
            layout: None,
            tags: Vec::new(),
            state: None,
            slug: None,
            date: None,
        }
    }

    /// Sets the content blocks for the post
    pub fn content(mut self, content: Vec<crate::npf::ContentBlock>) -> Self {
        self.content = content;
        self
    }

    /// Adds a single content block to the post
    pub fn add_block(mut self, block: crate::npf::ContentBlock) -> Self {
        self.content.push(block);
        self
    }

    /// Sets the layout for the content blocks
    pub fn layout(mut self, layout: Vec<crate::npf::LayoutBlock>) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Sets the tags for the post
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the post state ("published", "draft", "queue", "private")
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets a custom URL slug for the post
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Sets a custom publish date (GMT timestamp)
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    /// Sends the request to create the NPF post
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - Content blocks are empty
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "content": self.content,
        });

        if let Some(layout) = self.layout {
            body["layout"] = serde_json::json!(layout);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }
        if let Some(slug) = self.slug {
            body["slug"] = serde_json::json!(slug);
        }
        if let Some(date) = self.date {
            body["date"] = serde_json::json!(date);
        }

        let path = format!("blog/{}/posts", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

/// Builder for editing an existing post
///
/// This builder allows you to modify any field of an existing post.
pub struct EditPostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    id: String,
    title: Option<String>,
    body: Option<String>,
    tags: Option<Vec<String>>,
    state: Option<String>,
    slug: Option<String>,
}

impl EditPostBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier, id: String) -> Self {
        Self {
            client,
            blog,
            id,
            title: None,
            body: None,
            tags: None,
            state: None,
            slug: None,
        }
    }

    /// Sets the title of the post
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the body content of the post (HTML allowed)
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the tags for the post
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = Some(tags.into_iter().map(|t| t.into()).collect());
        self
    }

    /// Sets the post state ("published", "draft", "queue", "private")
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sets a custom URL slug for the post
    pub fn slug(mut self, slug: impl Into<String>) -> Self {
        self.slug = Some(slug.into());
        self
    }

    /// Sends the request to edit the post
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The post doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "id": self.id,
        });

        if let Some(title) = self.title {
            body["title"] = serde_json::json!(title);
        }
        if let Some(body_text) = self.body {
            body["body"] = serde_json::json!(body_text);
        }
        if let Some(tags) = self.tags {
            body["tags"] = serde_json::json!(tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }
        if let Some(slug) = self.slug {
            body["slug"] = serde_json::json!(slug);
        }

        let path = format!("blog/{}/post/edit", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

/// Builder for reblogging a post
///
/// Allows adding a comment and tags when reblogging.
pub struct ReblogBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    id: String,
    reblog_key: String,
    comment: Option<String>,
    tags: Vec<String>,
    state: Option<String>,
}

impl ReblogBuilder {
    fn new(client: Crabrave, blog: BlogIdentifier, id: String, reblog_key: String) -> Self {
        Self {
            client,
            blog,
            id,
            reblog_key,
            comment: None,
            tags: Vec::new(),
            state: None,
        }
    }

    /// Adds a comment to the reblog
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Sets the tags for the reblog
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Sets the post state ("published", "draft", "queue", "private")
    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Sends the request to reblog the post
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The original post doesn't exist
    /// - Invalid reblog key
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<CreatePostResponse> {
        let mut body = serde_json::json!({
            "id": self.id,
            "reblog_key": self.reblog_key,
        });

        if let Some(comment) = self.comment {
            body["comment"] = serde_json::json!(comment);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }

        let path = format!("blog/{}/post/reblog", self.blog.as_str());
        self.client.post(&path, &body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_text_post_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = CreateTextPostBuilder::new(client, blog)
            .title("Test Post")
            .body("This is a test")
            .tags(vec!["test", "example"])
            .state("draft")
            .slug("test-post");

        assert_eq!(builder.title, Some("Test Post".to_string()));
        assert_eq!(builder.body, Some("This is a test".to_string()));
        assert_eq!(builder.tags, vec!["test", "example"]);
        assert_eq!(builder.state, Some("draft".to_string()));
        assert_eq!(builder.slug, Some("test-post".to_string()));
    }

    #[test]
    fn test_create_text_post_builder_defaults() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = CreateTextPostBuilder::new(client, blog);

        assert!(builder.title.is_none());
        assert!(builder.body.is_none());
        assert!(builder.tags.is_empty());
        assert!(builder.state.is_none());
    }

    #[test]
    fn test_create_quote_post_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = CreateQuotePostBuilder::new(client, blog)
            .quote("To be or not to be")
            .source("Shakespeare")
            .tags(vec!["quotes", "literature"])
            .state("published");

        assert_eq!(builder.quote, Some("To be or not to be".to_string()));
        assert_eq!(builder.source, Some("Shakespeare".to_string()));
        assert_eq!(builder.tags, vec!["quotes", "literature"]);
        assert_eq!(builder.state, Some("published".to_string()));
    }

    #[test]
    fn test_create_link_post_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = CreateLinkPostBuilder::new(client, blog)
            .url("https://example.com")
            .title("Example Site")
            .description("A great example")
            .tags(vec!["links", "resources"]);

        assert_eq!(builder.url, Some("https://example.com".to_string()));
        assert_eq!(builder.title, Some("Example Site".to_string()));
        assert_eq!(builder.description, Some("A great example".to_string()));
        assert_eq!(builder.tags, vec!["links", "resources"]);
    }

    #[test]
    fn test_create_photo_post_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = CreatePhotoPostBuilder::new(client, blog)
            .source("https://example.com/photo.jpg")
            .caption("Beautiful sunset")
            .link("https://example.com")
            .tags(vec!["photo", "sunset"]);

        assert_eq!(builder.source, Some("https://example.com/photo.jpg".to_string()));
        assert_eq!(builder.caption, Some("Beautiful sunset".to_string()));
        assert_eq!(builder.link, Some("https://example.com".to_string()));
        assert_eq!(builder.tags, vec!["photo", "sunset"]);
    }

    #[test]
    fn test_edit_post_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = EditPostBuilder::new(client, blog, "123456".to_string())
            .title("Updated Title")
            .body("Updated body")
            .tags(vec!["updated", "edited"])
            .state("published");

        assert_eq!(builder.id, "123456");
        assert_eq!(builder.title, Some("Updated Title".to_string()));
        assert_eq!(builder.body, Some("Updated body".to_string()));
        assert_eq!(builder.tags, Some(vec!["updated".to_string(), "edited".to_string()]));
        assert_eq!(builder.state, Some("published".to_string()));
    }

    #[test]
    fn test_reblog_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = ReblogBuilder::new(client, blog, "123456".to_string(), "reblogkey".to_string())
            .comment("Great post!")
            .tags(vec!["reblog", "interesting"])
            .state("published");

        assert_eq!(builder.id, "123456");
        assert_eq!(builder.reblog_key, "reblogkey");
        assert_eq!(builder.comment, Some("Great post!".to_string()));
        assert_eq!(builder.tags, vec!["reblog", "interesting"]);
        assert_eq!(builder.state, Some("published".to_string()));
    }

    #[test]
    fn test_create_npf_post_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder = CreateNpfPostBuilder::new(client, blog)
            .add_block(crate::npf::ContentBlock::text("Hello"))
            .add_block(crate::npf::ContentBlock::image("https://example.com/img.jpg"))
            .tags(vec!["npf", "modern"]);

        assert_eq!(builder.content.len(), 2);
        assert_eq!(builder.tags, vec!["npf", "modern"]);
    }
}
