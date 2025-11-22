//! User-related API endpoints

use crate::{BlogIdentifier, Crabrave, CrabResult, User};
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
        LikesBuilder::new(self.client.clone())
    }

    /// Gets the blogs the user is following
    ///
    /// # Arguments
    ///
    /// * `limit` - Optional limit on number of results (default 20, max 20)
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
    /// let following = crab.users().following(Some(20), None).await?;
    /// println!("Following {} blogs", following.total_blogs);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn following(
        &self,
        limit: Option<u32>,
        offset: Option<u64>,
    ) -> CrabResult<FollowingResponse> {
        let mut path = "user/following".to_string();
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
}

/// Response from the user info endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// User information
    pub user: User,
}

/// Builder for querying the user's dashboard
pub struct DashboardBuilder {
    client: Crabrave,
    limit: Option<u32>,
    offset: Option<u64>,
    post_type: Option<String>,
    since_id: Option<String>,
    reblog_info: Option<bool>,
    notes_info: Option<bool>,
}

impl DashboardBuilder {
    fn new(client: Crabrave) -> Self {
        Self {
            client,
            limit: None,
            offset: None,
            post_type: None,
            since_id: None,
            reblog_info: None,
            notes_info: None,
        }
    }

    /// Sets the number of posts to return (max 20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the post offset for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Filters posts by type (text, photo, quote, link, chat, audio, video)
    pub fn post_type(mut self, post_type: impl Into<String>) -> Self {
        self.post_type = Some(post_type.into());
        self
    }

    /// Returns posts after this ID
    pub fn since_id(mut self, id: impl Into<String>) -> Self {
        self.since_id = Some(id.into());
        self
    }

    /// Include reblog information in responses
    pub fn reblog_info(mut self, include: bool) -> Self {
        self.reblog_info = Some(include);
        self
    }

    /// Include notes information in responses
    pub fn notes_info(mut self, include: bool) -> Self {
        self.notes_info = Some(include);
        self
    }

    /// Sends the request and returns the dashboard posts
    pub async fn send(self) -> CrabResult<DashboardResponse> {
        let mut path = "user/dashboard".to_string();
        let mut params = Vec::new();

        if let Some(limit) = self.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = self.offset {
            params.push(format!("offset={}", offset));
        }
        if let Some(post_type) = &self.post_type {
            params.push(format!("type={}", post_type));
        }
        if let Some(since_id) = &self.since_id {
            params.push(format!("since_id={}", since_id));
        }
        if let Some(reblog_info) = self.reblog_info {
            params.push(format!("reblog_info={}", reblog_info));
        }
        if let Some(notes_info) = self.notes_info {
            params.push(format!("notes_info={}", notes_info));
        }

        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        self.client.get(&path).await
    }
}

/// Response from the dashboard endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    /// List of posts from the user's dashboard
    pub posts: Vec<crate::handlers::blog::Post>,
}

/// Builder for querying the user's liked posts
pub struct LikesBuilder {
    client: Crabrave,
    limit: Option<u32>,
    offset: Option<u64>,
    before: Option<i64>,
    after: Option<i64>,
}

impl LikesBuilder {
    fn new(client: Crabrave) -> Self {
        Self {
            client,
            limit: None,
            offset: None,
            before: None,
            after: None,
        }
    }

    /// Sets the number of posts to return (max 20, default 20)
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the post offset for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Returns posts liked before this timestamp (Unix time)
    pub fn before(mut self, timestamp: i64) -> Self {
        self.before = Some(timestamp);
        self
    }

    /// Returns posts liked after this timestamp (Unix time)
    pub fn after(mut self, timestamp: i64) -> Self {
        self.after = Some(timestamp);
        self
    }

    /// Sends the request and returns the liked posts
    pub async fn send(self) -> CrabResult<LikesResponse> {
        let mut path = "user/likes".to_string();
        let mut params = Vec::new();

        if let Some(limit) = self.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(offset) = self.offset {
            params.push(format!("offset={}", offset));
        }
        if let Some(before) = self.before {
            params.push(format!("before={}", before));
        }
        if let Some(after) = self.after {
            params.push(format!("after={}", after));
        }

        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }

        self.client.get(&path).await
    }
}

/// Response from the likes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikesResponse {
    /// List of liked posts
    pub liked_posts: Vec<crate::handlers::blog::Post>,
    /// Total number of liked posts
    #[serde(default)]
    pub liked_count: u64,
}

/// Response from the following endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowingResponse {
    /// Total number of blogs being followed
    pub total_blogs: u64,
    /// List of blogs being followed
    pub blogs: Vec<crate::Blog>,
}

/// Request body for follow/unfollow operations
#[derive(Debug, Serialize)]
struct FollowRequest {
    url: String,
}

/// Response from follow/unfollow operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowResponse {
    /// Information about the followed/unfollowed blog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<crate::Blog>,
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

        assert_eq!(builder.limit, Some(10));
        assert_eq!(builder.offset, Some(20));
        assert_eq!(builder.post_type, Some("photo".to_string()));
        assert_eq!(builder.reblog_info, Some(true));
        assert_eq!(builder.notes_info, Some(false));
    }

    #[test]
    fn test_likes_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = LikesBuilder::new(client)
            .limit(15)
            .offset(30)
            .before(1234567890)
            .after(1234567800);

        assert_eq!(builder.limit, Some(15));
        assert_eq!(builder.offset, Some(30));
        assert_eq!(builder.before, Some(1234567890));
        assert_eq!(builder.after, Some(1234567800));
    }
}
