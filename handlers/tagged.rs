//! Tagged posts API endpoints

use crate::{Crabrave, CrabResult};
use serde::{Deserialize, Serialize};

/// API for searching posts by tag across the platform
///
/// Provides access to public posts that have been tagged with specific tags.
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
/// // Search for posts tagged with "photography"
/// let posts = crab.tagged("photography").limit(20).send().await?;
/// for post in posts.posts {
///     println!("Post: {} from {}", post.id, post.blog_name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Tagged {
    client: Crabrave,
    tag: String,
}

impl Tagged {
    /// Creates a new Tagged API for the specified tag
    pub(crate) fn new(client: Crabrave, tag: String) -> Self {
        Self { client, tag }
    }

    /// Returns a builder for querying tagged posts
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let posts = crab.tagged("photography")
    ///     .limit(20)
    ///     .before(1234567890)
    ///     .send()
    ///     .await?;
    ///
    /// for post in posts.posts {
    ///     println!("Post: {}", post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit(self, limit: u32) -> TaggedBuilder {
        TaggedBuilder::new(self.client, self.tag).limit(limit)
    }

    /// Returns posts before this timestamp
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let posts = crab.tagged("art")
    ///     .before(1234567890)
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn before(self, timestamp: i64) -> TaggedBuilder {
        TaggedBuilder::new(self.client, self.tag).before(timestamp)
    }

    /// Sets the post format filter
    ///
    /// # Arguments
    ///
    /// * `filter` - Post format ("text", "raw")
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let posts = crab.tagged("rust")
    ///     .filter("text")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter(self, filter: impl Into<String>) -> TaggedBuilder {
        TaggedBuilder::new(self.client, self.tag).filter(filter)
    }

    /// Sends the request with default parameters
    ///
    /// This is a convenience method that creates a builder and immediately sends it.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder().consumer_key("key").build()?;
    /// let posts = crab.tagged("photography").send().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(self) -> CrabResult<TaggedResponse> {
        TaggedBuilder::new(self.client, self.tag).send().await
    }
}

/// Builder for querying tagged posts
///
/// This builder allows you to configure various parameters for searching posts
/// by tag before sending the request.
pub struct TaggedBuilder {
    client: Crabrave,
    tag: String,
    limit: Option<u32>,
    before: Option<i64>,
    filter: Option<String>,
}

impl TaggedBuilder {
    fn new(client: Crabrave, tag: String) -> Self {
        Self {
            client,
            tag,
            limit: None,
            before: None,
            filter: None,
        }
    }

    /// Sets the number of posts to return (max 20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Returns posts before this timestamp (Unix time)
    pub fn before(mut self, timestamp: i64) -> Self {
        self.before = Some(timestamp);
        self
    }

    /// Sets the post format filter ("text", "raw")
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    /// Sends the request and returns the tagged posts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network request fails
    /// - API returns an error
    /// - Response cannot be parsed
    pub async fn send(self) -> CrabResult<TaggedResponse> {
        let mut params = vec![format!("tag={}", urlencoding::encode(&self.tag))];

        if let Some(limit) = self.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(before) = self.before {
            params.push(format!("before={}", before));
        }
        if let Some(filter) = &self.filter {
            params.push(format!("filter={}", filter));
        }

        let path = format!("tagged?{}", params.join("&"));
        self.client.get(&path).await
    }
}

/// Response from the tagged endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedResponse {
    /// List of posts matching the tag
    pub posts: Vec<crate::handlers::blog::Post>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tagged_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = TaggedBuilder::new(client, "photography".to_string())
            .limit(10)
            .before(1234567890)
            .filter("text");

        assert_eq!(builder.tag, "photography");
        assert_eq!(builder.limit, Some(10));
        assert_eq!(builder.before, Some(1234567890));
        assert_eq!(builder.filter, Some("text".to_string()));
    }

    #[test]
    fn test_tagged_builder_defaults() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = TaggedBuilder::new(client, "art".to_string());

        assert_eq!(builder.tag, "art");
        assert!(builder.limit.is_none());
        assert!(builder.before.is_none());
        assert!(builder.filter.is_none());
    }

    #[test]
    fn test_tagged_struct() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let tagged = Tagged::new(client, "rust".to_string());

        assert_eq!(tagged.tag, "rust");
    }
}
