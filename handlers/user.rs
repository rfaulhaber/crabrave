//! User-related API endpoints

use crate::{BlogIdentifier, CrabResult, Crabrave, User, handlers::likes::LikesBuilder};
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
        LikesBuilder::user(self.client.clone())
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
