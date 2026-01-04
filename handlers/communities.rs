//! Communities API endpoints
//!
//! Provides access to Tumblr community features including membership management,
//! timelines, invitations, moderation, and reactions.

use crate::{Blog, BlogIdentifier, CrabResult, Crabrave, EmptyResponse};
use serde::{Deserialize, Serialize};

/// API for community-related operations
///
/// Provides access to community timelines, membership management, invitations,
/// moderation, and reactions.
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
/// // Get community info
/// let info = crab.communities("rust-community").info().await?;
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

    /// Gets information about the community
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
    /// let info = crab.communities("rust-community").info().await?;
    /// println!("Community: {}", info.community.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn info(&self) -> CrabResult<CommunityInfoResponse> {
        let path = format!("communities/{}", self.handle);
        self.client.get(&path).await
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
        let path = format!("communities/{}/members", self.handle);
        self.client.put(&path, &serde_json::json!({})).await
    }

    /// Leaves the community
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier to remove from the community (your blog)
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
    /// crab.communities("rust-community").leave("my-blog").await?;
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
    pub async fn leave(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!("communities/{}/members/{}", self.handle, blog_id.as_str());
        self.client.delete(&path).await
    }

    /// Gets the list of community members
    ///
    /// Returns a builder for configuring the members request.
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
    ///     .members()
    ///     .limit(20)
    ///     .send()
    ///     .await?;
    /// println!("Members: {}", members.total_members);
    /// # Ok(())
    /// # }
    /// ```
    pub fn members(&self) -> MembersBuilder {
        MembersBuilder::new(self.client.clone(), self.handle.clone())
    }

    /// Removes a member from the community
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier of the member to remove
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
    /// crab.communities("rust-community").remove_member("problematic-blog").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_member(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!("communities/{}/members/{}", self.handle, blog_id.as_str());
        self.client.delete(&path).await
    }

    /// Changes a member's role in the community
    ///
    /// Requires admin privileges.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier of the member
    /// * `role` - The new role to assign
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::communities::MemberRole;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// crab.communities("rust-community")
    ///     .change_role("trusted-blog", MemberRole::Moderator)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn change_role(
        &self,
        blog: impl Into<BlogIdentifier>,
        role: MemberRole,
    ) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!("communities/{}/members/{}", self.handle, blog_id.as_str());
        self.client.put(&path, &ChangeRoleRequest { role }).await
    }

    /// Mutes the community
    ///
    /// When muted, you will not receive notifications from the community.
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
    /// crab.communities("rust-community").mute().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mute(&self) -> CrabResult<EmptyResponse> {
        let path = format!("communities/{}/mute", self.handle);
        self.client.put(&path, &serde_json::json!({})).await
    }

    /// Unmutes the community
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
    /// crab.communities("rust-community").unmute().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unmute(&self) -> CrabResult<EmptyResponse> {
        let path = format!("communities/{}/mute", self.handle);
        self.client.delete(&path).await
    }

    /// Gets pending invitations for the community
    ///
    /// Requires moderator or admin privileges.
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
    /// let invitations = crab.communities("rust-community").invitations().await?;
    /// for invite in invitations.invitations {
    ///     println!("Pending invite for: {}", invite.blog.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invitations(&self) -> CrabResult<InvitationsResponse> {
        let path = format!("communities/{}/invitations", self.handle);
        self.client.get(&path).await
    }

    /// Invites a blog to join the community
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier to invite
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
    /// crab.communities("rust-community").invite("friend-blog").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invite(&self, blog: impl Into<BlogIdentifier>) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!("communities/{}/invitations", self.handle);
        self.client
            .put(&path, &InviteRequest { blog: blog_id.to_string() })
            .await
    }

    /// Cancels an invitation to the community
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier whose invitation to cancel
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
    /// crab.communities("rust-community").cancel_invitation("friend-blog").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_invitation(
        &self,
        blog: impl Into<BlogIdentifier>,
    ) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!(
            "communities/{}/invitations/{}",
            self.handle,
            blog_id.as_str()
        );
        self.client.delete(&path).await
    }

    /// Checks the invitation status for a blog
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier to check
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
    /// let status = crab.communities("rust-community")
    ///     .invitation_status("friend-blog")
    ///     .await?;
    /// println!("Invitation pending: {}", status.pending);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invitation_status(
        &self,
        blog: impl Into<BlogIdentifier>,
    ) -> CrabResult<InvitationStatusResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!(
            "communities/{}/invitations/{}",
            self.handle,
            blog_id.as_str()
        );
        self.client.get(&path).await
    }

    /// Regenerates the community invite URL
    ///
    /// Requires admin privileges.
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
    /// let response = crab.communities("rust-community")
    ///     .regenerate_invite_url()
    ///     .await?;
    /// println!("New invite hash: {}", response.invite_hash);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn regenerate_invite_url(&self) -> CrabResult<InviteHashResponse> {
        let path = format!("communities/{}/invite_hash", self.handle);
        self.client.post(&path, &serde_json::json!({})).await
    }

    /// Adds a reaction to a post in the community
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to react to
    /// * `reaction_id` - The reaction ID to add
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
    /// crab.communities("rust-community")
    ///     .add_reaction("123456789", "heart")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_reaction(
        &self,
        post_id: impl Into<String>,
        reaction_id: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        let path = format!(
            "communities/{}/posts/{}/reactions",
            self.handle,
            post_id.into()
        );
        self.client
            .put(&path, &ReactionRequest { reaction_id: reaction_id.into() })
            .await
    }

    /// Removes a reaction from a post in the community
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post
    /// * `reaction_id` - The reaction ID to remove
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
    /// crab.communities("rust-community")
    ///     .remove_reaction("123456789", "heart")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_reaction(
        &self,
        post_id: impl Into<String>,
        reaction_id: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        let path = format!(
            "communities/{}/posts/{}/reactions/{}",
            self.handle,
            post_id.into(),
            reaction_id.into()
        );
        self.client.delete(&path).await
    }

    /// Views a moderated post
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the moderated post
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
    /// let post = crab.communities("rust-community")
    ///     .moderated_post("123456789")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn moderated_post(
        &self,
        post_id: impl Into<String>,
    ) -> CrabResult<ModeratedPostResponse> {
        let path = format!(
            "communities/{}/moderation/posts/{}",
            self.handle,
            post_id.into()
        );
        self.client.get(&path).await
    }

    /// Moderates (removes) a post from the community
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to moderate
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
    /// crab.communities("rust-community")
    ///     .moderate_post("123456789")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn moderate_post(&self, post_id: impl Into<String>) -> CrabResult<EmptyResponse> {
        let path = format!(
            "communities/{}/moderation/posts/{}",
            self.handle,
            post_id.into()
        );
        self.client.delete(&path).await
    }

    /// Restores a moderated post to the community
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to restore
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
    /// crab.communities("rust-community")
    ///     .restore_post("123456789")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn restore_post(&self, post_id: impl Into<String>) -> CrabResult<EmptyResponse> {
        let path = format!(
            "communities/{}/moderation/posts/{}",
            self.handle,
            post_id.into()
        );
        self.client.put(&path, &serde_json::json!({})).await
    }
}

// ============================================================================
// Request types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
struct ChangeRoleRequest {
    role: MemberRole,
}

#[derive(Debug, Clone, Serialize)]
struct InviteRequest {
    blog: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReactionRequest {
    reaction_id: String,
}

// ============================================================================
// Response types
// ============================================================================

/// Response from the community info endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityInfoResponse {
    /// Community information
    pub community: Community,
}

/// A Tumblr community
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// Community name/handle
    pub name: String,
    /// Community display title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Community description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Number of members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u64>,
    /// Whether the current user is a member
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_member: Option<bool>,
    /// Current user's role in the community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberRole>,
    /// Whether the community is muted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_muted: Option<bool>,
    /// Community avatar URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Community header image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_url: Option<String>,
}

/// Member role in a community
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    /// Regular member
    Member,
    /// Community moderator
    Moderator,
    /// Community administrator
    Admin,
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
    /// List of community members
    pub members: Vec<CommunityMember>,
}

/// A member of a community
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMember {
    /// Member's blog information
    pub blog: Blog,
    /// Member's role in the community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberRole>,
    /// When the member joined
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<i64>,
}

/// Response from the invitations endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationsResponse {
    /// List of pending invitations
    pub invitations: Vec<Invitation>,
}

/// A pending community invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    /// Blog that was invited
    pub blog: Blog,
    /// When the invitation was sent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invited_at: Option<i64>,
    /// Who sent the invitation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invited_by: Option<Blog>,
}

/// Response from checking invitation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationStatusResponse {
    /// Whether there is a pending invitation
    #[serde(default)]
    pub pending: bool,
}

/// Response from regenerating invite URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteHashResponse {
    /// The new invite hash
    pub invite_hash: String,
}

/// Response from viewing a moderated post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeratedPostResponse {
    /// The moderated post
    pub post: crate::handlers::blog::Post,
    /// Moderation information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<ModerationInfo>,
}

/// Information about post moderation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationInfo {
    /// Who moderated the post
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderated_by: Option<Blog>,
    /// When the post was moderated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderated_at: Option<i64>,
    /// Reason for moderation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// ============================================================================
// Builders
// ============================================================================

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
        let path = format!("communities/{}/timeline", self.handle);
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Query parameters for community members
#[derive(Debug, Clone, Serialize, Default)]
struct MembersQuery {
    /// Maximum number of members to return
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,

    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
}

/// Builder for querying community members
pub struct MembersBuilder {
    client: Crabrave,
    handle: String,
    query: MembersQuery,
}

impl MembersBuilder {
    fn new(client: Crabrave, handle: String) -> Self {
        Self {
            client,
            handle,
            query: MembersQuery::default(),
        }
    }

    /// Sets the maximum number of members to return
    pub fn limit(mut self, limit: u32) -> Self {
        self.query.limit = Some(limit);
        self
    }

    /// Sets the offset for pagination
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset = Some(offset);
        self
    }

    /// Sends the request and returns the members list
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The community doesn't exist
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<MembersResponse> {
        let path = format!("communities/{}/members", self.handle);
        self.client.get_with_query(&path, &self.query).await
    }
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

    #[test]
    fn test_members_builder() {
        let client = Crabrave::builder().consumer_key("test").build().unwrap();
        let builder = MembersBuilder::new(client, "rust-community".to_string())
            .limit(50)
            .offset(100);

        assert_eq!(builder.handle, "rust-community");
        assert_eq!(builder.query.limit, Some(50));
        assert_eq!(builder.query.offset, Some(100));
    }

    #[test]
    fn test_member_role_serialization() {
        assert_eq!(
            serde_json::to_string(&MemberRole::Member).unwrap(),
            "\"member\""
        );
        assert_eq!(
            serde_json::to_string(&MemberRole::Moderator).unwrap(),
            "\"moderator\""
        );
        assert_eq!(
            serde_json::to_string(&MemberRole::Admin).unwrap(),
            "\"admin\""
        );
    }

    #[test]
    fn test_member_role_deserialization() {
        assert_eq!(
            serde_json::from_str::<MemberRole>("\"member\"").unwrap(),
            MemberRole::Member
        );
        assert_eq!(
            serde_json::from_str::<MemberRole>("\"moderator\"").unwrap(),
            MemberRole::Moderator
        );
        assert_eq!(
            serde_json::from_str::<MemberRole>("\"admin\"").unwrap(),
            MemberRole::Admin
        );
    }
}
