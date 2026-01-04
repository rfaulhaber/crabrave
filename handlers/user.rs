//! User-related API endpoints

use crate::{
    BlogIdentifier, CrabResult, Crabrave, EmptyResponse, User,
    handlers::{following::FollowingBuilder, likes::LikesBuilder},
};
use serde::{Deserialize, Serialize};

/// API for user-related endpoints
///
/// Provides access to the authenticated user's information, dashboard, likes,
/// and following/follower operations.
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
/// // Get user information
/// let user_info = crab.users().info().await?;
/// println!("User: {}", user_info.user.name);
///
/// // Get dashboard
/// let dashboard = crab.users().dashboard().limit(10).send().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Users {
    client: Crabrave,
}

impl Users {
    /// Creates a new Users API
    pub(crate) fn new(client: Crabrave) -> Self {
        Self { client }
    }

    /// Gets information about the authenticated user
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
    /// let info = crab.users().info().await?;
    /// println!("Username: {}", info.user.name);
    /// println!("Following: {}", info.user.following);
    /// println!("Likes: {}", info.user.likes);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - Network request fails
    /// - API returns an error
    pub async fn info(&self) -> CrabResult<UserInfo> {
        self.client.get("user/info").await
    }

    /// Gets the authenticated user's rate limits
    ///
    /// Returns information about daily limits for various actions like
    /// posting, uploading photos/videos, following blogs, and liking posts.
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
    /// let limits = crab.users().limits().await?;
    ///
    /// if let Some(posts) = &limits.user.posts {
    ///     println!("Posts remaining: {}/{}", posts.remaining, posts.limit);
    /// }
    /// if let Some(photos) = &limits.user.photos {
    ///     println!("Photos remaining: {}/{}", photos.remaining, photos.limit);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - Network request fails
    /// - API returns an error
    pub async fn limits(&self) -> CrabResult<UserLimitsResponse> {
        self.client.get("user/limits").await
    }

    /// Gets posts from the user's dashboard
    ///
    /// Returns a builder for configuring the dashboard request.
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
    /// let posts = crab.users()
    ///     .dashboard()
    ///     .limit(20)
    ///     .post_type("photo")
    ///     .send()
    ///     .await?;
    ///
    /// for post in posts.posts {
    ///     println!("Post from {}: {}", post.blog_name, post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn dashboard(&self) -> DashboardBuilder {
        DashboardBuilder::new(self.client.clone())
    }

    /// Gets the user's liked posts
    ///
    /// Returns a builder for configuring the likes request.
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
    /// let likes = crab.users()
    ///     .likes()
    ///     .limit(20)
    ///     .send()
    ///     .await?;
    ///
    /// for post in likes.liked_posts {
    ///     println!("Liked: {}", post.id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn likes(&self) -> LikesBuilder {
        LikesBuilder::user(self.client.clone())
    }

    pub fn following(&self) -> FollowingBuilder {
        FollowingBuilder::user(self.client.clone())
    }

    /// Follows a blog
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
    /// crab.users().follow("staff").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn follow(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<FollowResponse> {
        let blog = blog.into();
        let body = FollowRequest {
            url: blog.as_str().to_string(),
        };
        self.client.post("user/follow", &body).await
    }

    /// Unfollows a blog
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
    /// crab.users().unfollow("staff").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unfollow(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<FollowResponse> {
        let blog = blog.into();
        let body = FollowRequest {
            url: blog.as_str().to_string(),
        };
        self.client.post("user/unfollow", &body).await
    }

    /// Likes a post
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to like
    /// * `reblog_key` - The reblog key for the post (found in post data)
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
    /// // Like a post using its ID and reblog key
    /// crab.users().like(123456789, "aB1cD2eF3").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The post doesn't exist
    /// - The reblog key is invalid
    /// - Daily like limit has been reached (1000/day)
    pub async fn like(
        &self,
        post_id: u64,
        reblog_key: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        let body = LikeRequest {
            id: post_id,
            reblog_key: reblog_key.into(),
        };
        self.client.post("user/like", &body).await
    }

    /// Unlikes a post
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to unlike
    /// * `reblog_key` - The reblog key for the post (found in post data)
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
    /// // Unlike a previously liked post
    /// crab.users().unlike(123456789, "aB1cD2eF3").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The post doesn't exist
    /// - The reblog key is invalid
    pub async fn unlike(
        &self,
        post_id: u64,
        reblog_key: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        let body = LikeRequest {
            id: post_id,
            reblog_key: reblog_key.into(),
        };
        self.client.post("user/unlike", &body).await
    }

    /// Gets the user's filtered tags
    ///
    /// Returns a builder for configuring the filtered tags request with pagination.
    /// Filtered tags are excluded from the user's dashboard and search results.
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
    /// let filtered = crab.users().filtered_tags().limit(20).send().await?;
    /// for tag in filtered.filtered_tags {
    ///     println!("Filtered tag: {}", tag);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn filtered_tags(&self) -> FilteredTagsBuilder {
        FilteredTagsBuilder::new(self.client.clone())
    }

    /// Adds tags to the user's filtered tags list
    ///
    /// # Arguments
    ///
    /// * `tags` - One or more tags to filter
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
    /// // Add multiple tags to filter
    /// crab.users().add_filtered_tags(vec!["spoilers", "nsfw"]).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - An invalid/empty tag is provided (400)
    /// - Maximum filter limit reached (403) - limit is 1000 tags
    pub async fn add_filtered_tags(
        &self,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> CrabResult<EmptyResponse> {
        let body = FilteredTagsRequest {
            filtered_tags: tags.into_iter().map(Into::into).collect(),
        };
        self.client.post("user/filtered_tags", &body).await
    }

    /// Removes a tag from the user's filtered tags list
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to remove from the filter list
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
    /// // Remove a tag from the filter list
    /// crab.users().remove_filtered_tag("spoilers").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_filtered_tag(&self, tag: impl Into<String>) -> CrabResult<EmptyResponse> {
        let tag: String = tag.into();
        let encoded_tag = urlencoding::encode(&tag);
        let path = format!("user/filtered_tags/{}", encoded_tag);
        self.client.delete(&path).await
    }

    /// Gets the user's filtered content strings
    ///
    /// Returns a builder for configuring the filtered content request with pagination.
    /// Filtered content strings are hidden from the user's dashboard, including
    /// blog names in reblog trails.
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
    /// let filtered = crab.users().filtered_content().limit(20).send().await?;
    /// for content in filtered.filtered_content {
    ///     println!("Filtered content: {}", content);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn filtered_content(&self) -> FilteredContentBuilder {
        FilteredContentBuilder::new(self.client.clone())
    }

    /// Adds content strings to the user's filtered content list
    ///
    /// Content filtering is not case sensitive. Adding "horse" will also
    /// filter "HORSE", "Horse", and partial matches like "horses".
    ///
    /// # Arguments
    ///
    /// * `content` - One or more content strings to filter (max 250 chars each)
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
    /// // Add content strings to filter
    /// crab.users().add_filtered_content(vec!["spoiler", "trigger warning"]).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - An invalid/empty string is provided (400)
    /// - Maximum filter limit reached (403) - limit is 200 strings
    pub async fn add_filtered_content(
        &self,
        content: impl IntoIterator<Item = impl Into<String>>,
    ) -> CrabResult<EmptyResponse> {
        let body = FilteredContentRequest {
            filtered_content: content.into_iter().map(Into::into).collect(),
        };
        self.client.post("user/filtered_content", &body).await
    }

    /// Removes a content string from the user's filtered content list
    ///
    /// # Arguments
    ///
    /// * `content` - The content string to remove from the filter list
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
    /// // Remove a content string from the filter list
    /// crab.users().remove_filtered_content("spoiler").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_filtered_content(
        &self,
        content: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        let content: String = content.into();
        self.client
            .delete_with_query(
                "user/filtered_content",
                &serde_json::json!({ "filtered_content": content }),
            )
            .await
    }

    /// Gets the list of communities the authenticated user has joined
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
    /// let communities = crab.users().joined_communities().await?;
    /// for community in communities.communities {
    ///     println!("Community: {}", community.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - Network request fails
    /// - API returns an error
    pub async fn joined_communities(&self) -> CrabResult<JoinedCommunitiesResponse> {
        self.client.get("communities").await
    }
}

/// Response from the user info endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User information
    pub user: User,
}

/// Response from the user limits endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLimitsResponse {
    /// User limit information
    pub user: UserLimits,
}

/// Response from the joined communities endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinedCommunitiesResponse {
    /// List of communities the user has joined
    pub communities: Vec<crate::handlers::communities::Community>,
}

/// User rate limit information
///
/// Contains the current limits and remaining counts for various
/// daily actions. All fields are optional as the API may not
/// return all limit types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLimits {
    /// Daily post limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts: Option<Limit>,

    /// Daily photo upload limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<Limit>,

    /// Daily video upload limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub videos: Option<Limit>,

    /// Daily video duration limit (in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_seconds: Option<Limit>,

    /// Daily follow limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follows: Option<Limit>,
}

/// A single rate limit with description and remaining count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Limit {
    /// Human-readable description of this limit
    pub description: String,

    /// Maximum number of actions allowed in the time period
    pub limit: u64,

    /// Number of actions remaining until the limit resets
    pub remaining: u64,

    /// Unix timestamp when the limit will reset
    pub reset_at: u64,
}

/// Query parameters for the user dashboard
#[derive(Debug, Clone, Serialize, Default)]
struct DashboardQuery {
    /// Maximum number of posts to return (API max: 20, default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Post offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,

    /// Filter by post type (text, photo, quote, link, chat, audio, video)
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    post_type: Option<String>,

    /// Return posts after this ID
    #[serde(skip_serializing_if = "Option::is_none")]
    since_id: Option<String>,

    /// Include reblog information in responses
    #[serde(skip_serializing_if = "Option::is_none")]
    reblog_info: Option<bool>,

    /// Include notes information in responses
    #[serde(skip_serializing_if = "Option::is_none")]
    notes_info: Option<bool>,
}

/// Builder for querying the user's dashboard
pub struct DashboardBuilder {
    client: Crabrave,
    query: DashboardQuery,
}

impl DashboardBuilder {
    fn new(client: Crabrave) -> Self {
        Self {
            client,
            query: DashboardQuery::default(),
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

    /// Returns posts after this ID
    pub fn since_id(mut self, id: impl Into<String>) -> Self {
        self.query.since_id = Some(id.into());
        self
    }

    /// Include reblog information in responses
    pub fn reblog_info(mut self, include: bool) -> Self {
        self.query.reblog_info = Some(include);
        self
    }

    /// Include notes information in responses
    pub fn notes_info(mut self, include: bool) -> Self {
        self.query.notes_info = Some(include);
        self
    }

    /// Sends the request and returns the dashboard posts
    pub async fn send(self) -> CrabResult<DashboardResponse> {
        self.client
            .get_with_query("user/dashboard", &self.query)
            .await
    }
}

/// Response from the dashboard endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    /// List of posts from the user's dashboard
    pub posts: Vec<crate::handlers::blog::Post>,
}

/// Request body for follow/unfollow operations
#[derive(Debug, Serialize)]
struct FollowRequest {
    url: String,
}

/// Request body for like/unlike operations
#[derive(Debug, Serialize)]
struct LikeRequest {
    /// The post ID to like/unlike
    id: u64,
    /// The reblog key for the post
    reblog_key: String,
}

/// Response from follow/unfollow operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowResponse {
    /// Information about the followed/unfollowed blog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<crate::Blog>,
}

// =============================================================================
// Filtered Tags Types
// =============================================================================

/// Query parameters for filtered tags
#[derive(Debug, Clone, Serialize, Default)]
struct FilteredTagsQuery {
    /// Results per request, 1-20 inclusive (default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Starting position (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
}

/// Builder for querying filtered tags
pub struct FilteredTagsBuilder {
    client: Crabrave,
    query: FilteredTagsQuery,
}

impl FilteredTagsBuilder {
    fn new(client: Crabrave) -> Self {
        Self {
            client,
            query: FilteredTagsQuery::default(),
        }
    }

    /// Sets the number of results to return (max 20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Sets the starting position for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Sends the request and returns the filtered tags
    pub async fn send(self) -> CrabResult<FilteredTagsResponse> {
        self.client
            .get_with_query("user/filtered_tags", &self.query)
            .await
    }
}

/// Response from the filtered tags endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredTagsResponse {
    /// List of filtered tags
    pub filtered_tags: Vec<String>,
}

/// Request body for adding filtered tags
#[derive(Debug, Serialize)]
struct FilteredTagsRequest {
    filtered_tags: Vec<String>,
}

// =============================================================================
// Filtered Content Types
// =============================================================================

/// Query parameters for filtered content
#[derive(Debug, Clone, Serialize, Default)]
struct FilteredContentQuery {
    /// Results per request, 1-20 inclusive (default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Starting position (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
}

/// Builder for querying filtered content
pub struct FilteredContentBuilder {
    client: Crabrave,
    query: FilteredContentQuery,
}

impl FilteredContentBuilder {
    fn new(client: Crabrave) -> Self {
        Self {
            client,
            query: FilteredContentQuery::default(),
        }
    }

    /// Sets the number of results to return (max 20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Sets the starting position for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Sends the request and returns the filtered content strings
    pub async fn send(self) -> CrabResult<FilteredContentResponse> {
        self.client
            .get_with_query("user/filtered_content", &self.query)
            .await
    }
}

/// Response from the filtered content endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredContentResponse {
    /// List of filtered content strings
    pub filtered_content: Vec<String>,
}

/// Request body for adding filtered content
#[derive(Debug, Serialize)]
struct FilteredContentRequest {
    filtered_content: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = DashboardBuilder::new(client)
            .limit(10)
            .offset(20)
            .post_type("photo")
            .reblog_info(true)
            .notes_info(false);

        assert_eq!(builder.query.limit, Some(10));
        assert_eq!(builder.query.offset, Some(20));
        assert_eq!(builder.query.post_type, Some("photo".to_string()));
        assert_eq!(builder.query.reblog_info, Some(true));
        assert_eq!(builder.query.notes_info, Some(false));
    }

    #[test]
    fn test_likes_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = LikesBuilder::user(client)
            .limit(15)
            .offset(30)
            .before(1234567890)
            .after(1234567800);

        assert_eq!(builder.query().limit, Some(15));
        assert_eq!(builder.query().offset, Some(30));
        assert_eq!(builder.query().before, Some(1234567890));
        assert_eq!(builder.query().after, Some(1234567800));
    }
}
