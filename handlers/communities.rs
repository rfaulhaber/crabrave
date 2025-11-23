//! Communities API endpoints

use crate::{Crabrave, CrabResult};
use serde::{Deserialize, Serialize};

/// API for community-related operations
///
/// Provides access to community timelines, membership management, and member lists.
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
/// // Get community timeline
/// let timeline = crab.communities("rust-community")
///     .timeline()
///     .limit(20)
///     .send()
///     .await?;
///
/// // Join a community
/// crab.communities("rust-community").join().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Communities {
    client: Crabrave,
    handle: String,
}

impl Communities {
    /// Creates a new Communities API for the specified community
    pub(crate) fn new(client: Crabrave, handle: String) -> Self {
        Self { client, handle }
    }

    /// Gets the timeline for a community
    ///
    /// Returns a builder for configuring the timeline request.
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
    /// let timeline = crab.communities("rust-community")
    ///     .timeline()
    ///     .limit(20)
    ///     .send()
    ///     .await?;
    ///
    /// for post in timeline.posts {
    ///     println!("Post: {} from {}", post.id, post.blog_name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn timeline(&self) -> TimelineBuilder {
        TimelineBuilder::new(self.client.clone(), self.handle.clone())
    }

    /// Joins the community
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
    /// crab.communities("rust-community").join().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The community doesn't exist
    /// - User is already a member
    /// - Network request fails
    /// - API returns an error
    pub async fn join(&self) -> CrabResult<MembershipResponse> {
        let path = format!("community/{}/join", self.handle);
        self.client.post(&path, &serde_json::json!({})).await
    }

    /// Leaves the community
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
    /// crab.communities("rust-community").leave().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The community doesn't exist
    /// - User is not a member
    /// - Network request fails
    /// - API returns an error
    pub async fn leave(&self) -> CrabResult<MembershipResponse> {
        let path = format!("community/{}/leave", self.handle);
        self.client.post(&path, &serde_json::json!({})).await
    }

    /// Gets the list of community members
    ///
    /// # Arguments
    ///
    /// * `limit` - Optional limit on number of results (default 20)
    /// * `offset` - Optional offset for pagination
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
    /// let members = crab.communities("rust-community")
    ///     .members(Some(20), None)
    ///     .await?;
    /// println!("Members: {}", members.total_members);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn members(
        &self,
        limit: Option<u32>,
        offset: Option<u64>,
    ) -> CrabResult<MembersResponse> {
        let mut path = format!("community/{}/members", self.handle);
        let mut params = Vec::new();

        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = offset {
            params.push(format!("offset={}", offset));
        }

        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        self.client.get(&path).await
    }
}

/// Query parameters for community timeline
#[derive(Debug, Clone, Serialize, Default)]
struct TimelineQuery {
    /// Maximum number of posts to return (API max: 20, default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Post offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,

    /// Return posts before this timestamp (Unix time)
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<i64>,
}

/// Builder for querying a community timeline
pub struct TimelineBuilder {
    client: Crabrave,
    handle: String,
    query: TimelineQuery,
}

impl TimelineBuilder {
    fn new(client: Crabrave, handle: String) -> Self {
        Self {
            client,
            handle,
            query: TimelineQuery::default(),
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

    /// Returns posts before this timestamp (Unix time)
    pub fn before(mut self, timestamp: i64) -> Self {
        self.query.before = Some(timestamp);
        self
    }

    /// Sends the request and returns the timeline posts
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The community doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<TimelineResponse> {
        let path = format!("community/{}/timeline", self.handle);
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Response from the community timeline endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineResponse {
    /// List of posts from the community timeline
    pub posts: Vec<crate::handlers::blog::Post>,
}

/// Response from community membership operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipResponse {
    /// Success status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

/// Response from the members endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembersResponse {
    /// Total number of members in the community
    pub total_members: u64,
    /// List of member blogs
    pub members: Vec<crate::Blog>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = TimelineBuilder::new(client, "rust-community".to_string())
            .limit(10)
            .offset(20)
            .before(1234567890);

        assert_eq!(builder.handle, "rust-community");
        assert_eq!(builder.query.limit, Some(10));
        assert_eq!(builder.query.offset, Some(20));
        assert_eq!(builder.query.before, Some(1234567890));
    }

    #[test]
    fn test_timeline_builder_defaults() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = TimelineBuilder::new(client, "rust".to_string());

        assert_eq!(builder.handle, "rust");
        assert!(builder.query.limit.is_none());
        assert!(builder.query.offset.is_none());
        assert!(builder.query.before.is_none());
    }

    #[test]
    fn test_communities_struct() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let communities = Communities::new(client, "rust-community".to_string());

        assert_eq!(communities.handle, "rust-community");
    }
}
