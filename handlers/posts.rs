//! Post creation, editing, and deletion API endpoints
//!
//! This module provides builders and types for working with Tumblr posts using
//! the Neue Post Format (NPF). Post operations are accessed through the Blogs handler:
//!
//! - `crab.blogs("blog").post("id").get()` - Fetch a post
//! - `crab.blogs("blog").post("id").delete()` - Delete a post
//! - `crab.blogs("blog").post("id").edit()` - Edit a post
//! - `crab.blogs("blog").create_post()` - Create a new post
//! - `crab.blogs("blog").reblog(id, key)` - Reblog a post
//!
//! # Example
//!
//! ```no_run
//! # use crabrave::Crabrave;
//! # use crabrave::npf::ContentBlock;
//! # use crabrave::media::MediaSource;
//! # async fn example() -> Result<(), crabrave::CrabError> {
//! # let crab = Crabrave::builder()
//! #     .consumer_key("key")
//! #     .consumer_secret("secret")
//! #     .access_token("token")
//! #     .build()?;
//! // Create a post
//! let post = crab.blogs("my-blog")
//!     .create_post()
//!     .add_block(ContentBlock::text("Hello!"))
//!     .send()
//!     .await?;
//!
//! // Fetch a post
//! let post = crab.blogs("my-blog").post("123456").get().await?;
//!
//! // Delete a post
//! crab.blogs("my-blog").post("123456").delete().await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    BlogIdentifier, CrabResult, Crabrave,
    handlers::blog::NpfPost,
    media::MediaSource,
    npf::{ContentBlock, LayoutBlock, MediaObject},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Response from getting a single post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    /// The requested post
    pub post: NpfPost,
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

/// Response from muting a post's notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteResponse {
    /// Whether the post is now muted
    #[serde(default)]
    pub muted: bool,
    /// Unix timestamp when the mute expires (0 means forever muted)
    #[serde(default)]
    pub mute_end_timestamp: i64,
}

/// Builder for creating an NPF (Neue Post Format) post
///
/// NPF allows creating rich, structured posts with content blocks.
///
/// # Media Uploads
///
/// To upload images or videos, use the `media_source()` method to associate
/// media files with identifiers referenced in your content blocks.
///
/// # Example
///
/// ```no_run
/// # use crabrave::Crabrave;
/// # use crabrave::npf::{ContentBlock, MediaObject};
/// # use crabrave::media::MediaSource;
/// # async fn example() -> Result<(), crabrave::CrabError> {
/// # let crab = Crabrave::builder()
/// #     .consumer_key("key")
/// #     .consumer_secret("secret")
/// #     .access_token("token")
/// #     .build()?;
/// // Create a post with an uploaded image
/// let post = crab.posts()
///     .create("my-blog")
///     .add_image(MediaSource::from_path("/path/to/image.jpg"))
///     .tags(vec!["photography"])
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct CreatePostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    content: Vec<ContentBlock>,
    layout: Option<Vec<LayoutBlock>>,
    tags: Vec<String>,
    state: Option<String>,
    slug: Option<String>,
    date: Option<String>,
    media_sources: HashMap<String, MediaSource>,
    next_media_id: usize,
}

impl CreatePostBuilder {
    pub(crate) fn new(client: Crabrave, blog: BlogIdentifier) -> Self {
        Self {
            client,
            blog,
            content: Vec::new(),
            layout: None,
            tags: Vec::new(),
            state: None,
            slug: None,
            date: None,
            media_sources: HashMap::new(),
            next_media_id: 0,
        }
    }

    /// Generates a unique media identifier
    fn generate_media_id(&mut self) -> String {
        let id = format!("media_{}", self.next_media_id);
        self.next_media_id += 1;
        id
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

    /// Adds media to upload with an explicit identifier
    ///
    /// The identifier must match the one used in your content blocks' MediaObject.
    /// For most cases, consider using `add_image()` or `add_video()` which
    /// auto-generate identifiers.
    ///
    /// # Arguments
    ///
    /// * `identifier` - Unique identifier to link media to content blocks
    /// * `source` - Media source (file path or bytes)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::npf::{ContentBlock, MediaObject};
    /// # use crabrave::media::MediaSource;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// // Manually control identifiers
    /// let post = crab.posts()
    ///     .create("my-blog")
    ///     .content(vec![
    ///         ContentBlock::Image {
    ///             media: vec![MediaObject {
    ///                 url: String::new(),
    ///                 media_type: Some("image/jpeg".to_string()),
    ///                 identifier: Some("my_image".to_string()),
    ///                 width: None,
    ///                 height: None,
    ///                 original_dimensions_missing: None,
    ///                 cropped: None,
    ///                 has_original_dimensions: None,
    ///             }],
    ///             alt_text: Some("My photo".to_string()),
    ///             caption: None,
    ///             attribution: None,
    ///         }
    ///     ])
    ///     .media_source("my_image", MediaSource::from_path("/path/to/image.jpg"))
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn media_source(mut self, identifier: impl Into<String>, source: MediaSource) -> Self {
        self.media_sources.insert(identifier.into(), source);
        self
    }

    /// Adds an image to the post with auto-generated identifier
    ///
    /// This is a convenience method that automatically creates an image content
    /// block and associates it with the media source.
    ///
    /// # Arguments
    ///
    /// * `source` - Media source for the image
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::media::MediaSource;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// // Simple image upload
    /// let post = crab.posts()
    ///     .create("my-blog")
    ///     .add_image(MediaSource::from_path("/path/to/photo.jpg"))
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_image(mut self, source: MediaSource) -> Self {
        let identifier = self.generate_media_id();
        let mime_type = source.mime_type().map(|s| s.to_string());

        self.content.push(ContentBlock::Image {
            media: vec![MediaObject {
                url: String::new(),
                media_type: mime_type,
                identifier: Some(identifier.clone()),
                width: None,
                height: None,
                original_dimensions_missing: None,
                cropped: None,
                has_original_dimensions: None,
            }],
            alt_text: None,
            caption: None,
            attribution: None,
        });

        self.media_sources.insert(identifier, source);
        self
    }

    /// Adds a video to the post with auto-generated identifier
    ///
    /// This is a convenience method that automatically creates a video content
    /// block and associates it with the media source.
    ///
    /// # Arguments
    ///
    /// * `source` - Media source for the video
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::media::MediaSource;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// // Simple video upload
    /// let post = crab.posts()
    ///     .create("my-blog")
    ///     .add_video(MediaSource::from_path("/path/to/video.mp4"))
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_video(mut self, source: MediaSource) -> Self {
        let identifier = self.generate_media_id();
        let mime_type = source.mime_type().map(|s| s.to_string());

        self.content.push(ContentBlock::Video {
            media: Some(vec![MediaObject {
                url: String::new(),
                media_type: mime_type,
                identifier: Some(identifier.clone()),
                width: None,
                height: None,
                original_dimensions_missing: None,
                cropped: None,
                has_original_dimensions: None,
            }]),
            url: None,
            provider: Some("tumblr".to_string()),
            embed_html: None,
            embed_iframe: None,
            embed_url: None,
            poster: None,
            attribution: None,
            can_autoplay_on_cellular: None,
            duration: None,
            metadata: None,
        });

        self.media_sources.insert(identifier, source);
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

        // Use multipart if there are media sources, otherwise use regular JSON POST
        if self.media_sources.is_empty() {
            self.client.post(&path, &body).await
        } else {
            self.client
                .post_multipart(&path, &body, self.media_sources)
                .await
        }
    }
}

/// Builder for editing an existing post using NPF format
///
/// This builder allows you to modify any field of an existing post using
/// Tumblr's Neue Post Format (NPF) with content blocks.
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
/// let edited = crab.posts()
///     .edit("my-blog", "123456")
///     .content(vec![
///         ContentBlock::heading("Updated Title", 1),
///         ContentBlock::text("Updated content with NPF!"),
///     ])
///     .tags(vec!["updated", "npf"])
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct EditPostBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    id: String,
    content: Option<Vec<ContentBlock>>,
    layout: Option<Vec<LayoutBlock>>,
    tags: Option<Vec<String>>,
    state: Option<String>,
    slug: Option<String>,
    media_sources: HashMap<String, MediaSource>,
    next_media_id: usize,
}

impl EditPostBuilder {
    pub(crate) fn new(client: Crabrave, blog: BlogIdentifier, id: String) -> Self {
        Self {
            client,
            blog,
            id,
            content: None,
            layout: None,
            tags: None,
            state: None,
            slug: None,
            media_sources: HashMap::new(),
            next_media_id: 0,
        }
    }

    /// Generates a unique media identifier
    fn generate_media_id(&mut self) -> String {
        let id = format!("media_{}", self.next_media_id);
        self.next_media_id += 1;
        id
    }

    /// Sets the content blocks for the post (replaces existing content)
    pub fn content(mut self, content: Vec<ContentBlock>) -> Self {
        self.content = Some(content);
        self
    }

    /// Adds a single content block to the post
    pub fn add_block(mut self, block: ContentBlock) -> Self {
        self.content.get_or_insert_with(Vec::new).push(block);
        self
    }

    /// Sets the layout for the content blocks
    pub fn layout(mut self, layout: Vec<LayoutBlock>) -> Self {
        self.layout = Some(layout);
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

    /// Adds media to upload with an explicit identifier
    ///
    /// The identifier must match the one used in your content blocks' MediaObject.
    /// For most cases, consider using `add_image()` or `add_video()` which
    /// auto-generate identifiers.
    pub fn media_source(mut self, identifier: impl Into<String>, source: MediaSource) -> Self {
        self.media_sources.insert(identifier.into(), source);
        self
    }

    /// Adds an image to the post with auto-generated identifier
    ///
    /// This is a convenience method that automatically creates an image content
    /// block and associates it with the media source.
    pub fn add_image(mut self, source: MediaSource) -> Self {
        let identifier = self.generate_media_id();
        let mime_type = source.mime_type().map(|s| s.to_string());

        let image_block = ContentBlock::Image {
            media: vec![MediaObject {
                url: String::new(),
                media_type: mime_type,
                identifier: Some(identifier.clone()),
                width: None,
                height: None,
                original_dimensions_missing: None,
                cropped: None,
                has_original_dimensions: None,
            }],
            alt_text: None,
            caption: None,
            attribution: None,
        };

        self.content.get_or_insert_with(Vec::new).push(image_block);
        self.media_sources.insert(identifier, source);
        self
    }

    /// Adds a video to the post with auto-generated identifier
    ///
    /// This is a convenience method that automatically creates a video content
    /// block and associates it with the media source.
    pub fn add_video(mut self, source: MediaSource) -> Self {
        let identifier = self.generate_media_id();
        let mime_type = source.mime_type().map(|s| s.to_string());

        let video_block = ContentBlock::Video {
            media: Some(vec![MediaObject {
                url: String::new(),
                media_type: mime_type,
                identifier: Some(identifier.clone()),
                width: None,
                height: None,
                original_dimensions_missing: None,
                cropped: None,
                has_original_dimensions: None,
            }]),
            url: None,
            provider: Some("tumblr".to_string()),
            embed_html: None,
            embed_iframe: None,
            embed_url: None,
            poster: None,
            attribution: None,
            can_autoplay_on_cellular: None,
            duration: None,
            metadata: None,
        };

        self.content.get_or_insert_with(Vec::new).push(video_block);
        self.media_sources.insert(identifier, source);
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
    pub async fn send(self) -> CrabResult<EditPostResponse> {
        let mut body = serde_json::json!({});

        if let Some(content) = self.content {
            body["content"] = serde_json::json!(content);
        }
        if let Some(layout) = self.layout {
            body["layout"] = serde_json::json!(layout);
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

        let path = format!("blog/{}/posts/{}", self.blog.as_str(), self.id);

        // Use multipart if there are media sources, otherwise use regular JSON PUT
        if self.media_sources.is_empty() {
            self.client.put(&path, &body).await
        } else {
            self.client
                .put_multipart(&path, &body, self.media_sources)
                .await
        }
    }
}

/// Response from editing a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditPostResponse {
    /// ID of the edited post
    #[serde(default)]
    pub id: String,
}

/// Builder for reblogging a post
///
/// Allows adding content and tags when reblogging. Supports both simple
/// text comments and full NPF content blocks.
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
/// // Simple text comment
/// crab.posts()
///     .reblog("my-blog", "123456", "reblogkey")
///     .comment("Great post!")
///     .tags(vec!["reblog"])
///     .send()
///     .await?;
///
/// // NPF content blocks for richer comments
/// crab.posts()
///     .reblog("my-blog", "123456", "reblogkey")
///     .content(vec![
///         ContentBlock::heading("My thoughts", 1),
///         ContentBlock::text("This is a really interesting post!"),
///     ])
///     .send()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct ReblogBuilder {
    client: Crabrave,
    blog: BlogIdentifier,
    id: String,
    reblog_key: String,
    content: Option<Vec<ContentBlock>>,
    layout: Option<Vec<LayoutBlock>>,
    comment: Option<String>,
    tags: Vec<String>,
    state: Option<String>,
}

impl ReblogBuilder {
    pub(crate) fn new(
        client: Crabrave,
        blog: BlogIdentifier,
        id: String,
        reblog_key: String,
    ) -> Self {
        Self {
            client,
            blog,
            id,
            reblog_key,
            content: None,
            layout: None,
            comment: None,
            tags: Vec::new(),
            state: None,
        }
    }

    /// Sets the content blocks for the reblog comment (NPF format)
    ///
    /// Use this for rich content. For simple text, use `comment()` instead.
    pub fn content(mut self, content: Vec<ContentBlock>) -> Self {
        self.content = Some(content);
        self
    }

    /// Adds a single content block to the reblog
    pub fn add_block(mut self, block: ContentBlock) -> Self {
        self.content.get_or_insert_with(Vec::new).push(block);
        self
    }

    /// Sets the layout for the content blocks
    pub fn layout(mut self, layout: Vec<LayoutBlock>) -> Self {
        self.layout = Some(layout);
        self
    }

    /// Adds a simple text comment to the reblog
    ///
    /// For richer content, use `content()` with NPF blocks instead.
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
            "parent_tumblelog_uuid": self.id,
            "reblog_key": self.reblog_key,
        });

        // NPF content takes precedence over simple comment
        if let Some(content) = self.content {
            body["content"] = serde_json::json!(content);
        } else if let Some(comment) = self.comment {
            body["comment"] = serde_json::json!(comment);
        }

        if let Some(layout) = self.layout {
            body["layout"] = serde_json::json!(layout);
        }
        if !self.tags.is_empty() {
            body["tags"] = serde_json::json!(self.tags.join(","));
        }
        if let Some(state) = self.state {
            body["state"] = serde_json::json!(state);
        }

        let path = format!("blog/{}/posts", self.blog.as_str());
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
            .add_block(crate::npf::ContentBlock::text("Updated content"))
            .tags(vec!["updated", "edited"])
            .state("published");

        assert_eq!(builder.id, "123456");
        assert!(builder.content.is_some());
        assert_eq!(builder.content.as_ref().unwrap().len(), 1);
        assert_eq!(
            builder.tags,
            Some(vec!["updated".to_string(), "edited".to_string()])
        );
        assert_eq!(builder.state, Some("published".to_string()));
    }

    #[test]
    fn test_reblog_builder_with_comment() {
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
        assert!(builder.content.is_none());
        assert_eq!(builder.tags, vec!["reblog", "interesting"]);
        assert_eq!(builder.state, Some("published".to_string()));
    }

    #[test]
    fn test_reblog_builder_with_npf_content() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");
        let builder =
            ReblogBuilder::new(client, blog, "123456".to_string(), "reblogkey".to_string())
                .add_block(crate::npf::ContentBlock::text("My thoughts on this"))
                .tags(vec!["reblog"]);

        assert_eq!(builder.id, "123456");
        assert!(builder.content.is_some());
        assert_eq!(builder.content.as_ref().unwrap().len(), 1);
        assert!(builder.comment.is_none());
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

    #[test]
    fn test_create_post_with_image_upload() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");

        let media_source = MediaSource::from_bytes("image.jpg", vec![1, 2, 3, 4]);
        let builder = CreatePostBuilder::new(client, blog)
            .add_block(ContentBlock::text("Check out this image!"))
            .add_image(media_source)
            .tags(vec!["photo"]);

        assert_eq!(builder.content.len(), 2);
        assert_eq!(builder.media_sources.len(), 1);
        assert!(builder.media_sources.contains_key("media_0"));
    }

    #[test]
    fn test_create_post_with_multiple_media() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");

        let img1 = MediaSource::from_bytes("image1.jpg", vec![1, 2, 3]);
        let img2 = MediaSource::from_bytes("image2.png", vec![4, 5, 6]);

        let builder = CreatePostBuilder::new(client, blog)
            .add_image(img1)
            .add_image(img2);

        assert_eq!(builder.content.len(), 2);
        assert_eq!(builder.media_sources.len(), 2);
        assert!(builder.media_sources.contains_key("media_0"));
        assert!(builder.media_sources.contains_key("media_1"));
    }

    #[test]
    fn test_create_post_with_explicit_media_source() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");

        let media_source = MediaSource::from_bytes("custom.jpg", vec![1, 2, 3]);
        let builder = CreatePostBuilder::new(client, blog)
            .content(vec![ContentBlock::Image {
                media: vec![MediaObject {
                    url: String::new(),
                    media_type: Some("image/jpeg".to_string()),
                    identifier: Some("custom_id".to_string()),
                    width: None,
                    height: None,
                    original_dimensions_missing: None,
                    cropped: None,
                    has_original_dimensions: None,
                }],
                alt_text: Some("Custom image".to_string()),
                caption: None,
                attribution: None,
            }])
            .media_source("custom_id", media_source);

        assert_eq!(builder.content.len(), 1);
        assert_eq!(builder.media_sources.len(), 1);
        assert!(builder.media_sources.contains_key("custom_id"));
    }

    #[test]
    fn test_edit_post_with_image() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let blog = BlogIdentifier::from("my-blog");

        let media_source = MediaSource::from_bytes("updated.jpg", vec![1, 2, 3]);
        let builder =
            EditPostBuilder::new(client, blog, "123456".to_string()).add_image(media_source);

        assert!(builder.content.is_some());
        assert_eq!(builder.content.as_ref().unwrap().len(), 1);
        assert_eq!(builder.media_sources.len(), 1);
    }

    #[test]
    fn test_media_source_mime_type_detection() {
        let jpg = MediaSource::from_bytes("test.jpg", vec![1, 2, 3]);
        assert_eq!(jpg.mime_type(), Some("image/jpeg"));

        let png = MediaSource::from_bytes("test.png", vec![1, 2, 3]);
        assert_eq!(png.mime_type(), Some("image/png"));

        let mp4 = MediaSource::from_bytes("video.mp4", vec![1, 2, 3]);
        assert_eq!(mp4.mime_type(), Some("video/mp4"));

        let unknown = MediaSource::from_bytes("file.xyz", vec![1, 2, 3]);
        assert_eq!(unknown.mime_type(), None);
    }

    #[test]
    fn test_media_source_custom_mime_type() {
        let source =
            MediaSource::from_bytes("data.bin", vec![1, 2, 3]).with_mime_type("application/custom");
        assert_eq!(source.mime_type(), Some("application/custom"));
    }
}
