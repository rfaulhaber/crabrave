//! Post creation, editing, and deletion API endpoints

use crate::{
    BlogIdentifier, CrabResult, Crabrave,
    handlers::blog::Post,
    npf::{ContentBlock, LayoutBlock},
};
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
    pub fn create(&self, blog: impl Into<BlogIdentifier>) -> CreatePostBuilder {
        CreatePostBuilder::new(self.client.clone(), blog.into())
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
    pub fn edit(&self, blog: impl Into<BlogIdentifier>, id: impl Into<String>) -> EditPostBuilder {
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
    pub post: Post,
}

/// Response from deleting a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    /// Post ID that was deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Response from creating a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostResponse {
    /// ID of the created post
    pub id: String,
}

/// Builder for creating an NPF (Neue Post Format) post
///
/// NPF allows creating rich, structured posts with content blocks.
pub struct CreatePostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    content: Vec<ContentBlock>,
    layout: Option<Vec<LayoutBlock>>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
}

impl CreatePostBuilder {
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
    pub fn content(mut self, content: Vec<ContentBlock>) -> Self {
        self.content = content;
        self
    }

    /// Adds a single content block to the post
    pub fn add_block(mut self, block: ContentBlock) -> Self {
        self.content.push(block);
        self
    }

    /// Sets the layout for the content blocks
    pub fn layout(mut self, layout: Vec<LayoutBlock>) -> Self {
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
        assert_eq!(
            builder.tags,
            Some(vec!["updated".to_string(), "edited".to_string()])
        );
        assert_eq!(builder.state, Some("published".to_string()));
    }

    #[test]
    fn test_reblog_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder =
            ReblogBuilder::new(client, blog, "123456".to_string(), "reblogkey".to_string())
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
        let builder = CreatePostBuilder::new(client, blog)
            .add_block(crate::npf::ContentBlock::text("Hello"))
            .add_block(crate::npf::ContentBlock::image(
                "https://example.com/img.jpg",
            ))
            .tags(vec!["npf", "modern"]);

        assert_eq!(builder.content.len(), 2);
        assert_eq!(builder.tags, vec!["npf", "modern"]);
    }
}
