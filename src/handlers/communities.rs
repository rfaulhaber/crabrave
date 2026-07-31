//! Communities API endpoints
//!
//! Provides access to Tumblr community features including membership management,
//! timelines, invitations, moderation, and reactions.
//!
//! Communities is a beta feature; Tumblr documents that these endpoints may
//! change in backwards-incompatible ways until the feature is fully released.

use crate::npf::MediaObject;
use crate::{Blog, BlogIdentifier, CrabError, CrabResult, Crabrave, EmptyResponse};
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
    /// The API returns the community object directly; see [`Community`] for
    /// which fields are visible to which roles.
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
    /// println!("Community: {}", info.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn info(&self) -> CrabResult<Community> {
        let path = format!("communities/{}", self.handle);
        self.client.get(&path).await
    }

    /// Gets information about the community in the `edit` context
    ///
    /// Sends `?context=edit`, which asks the API to include the additional
    /// editable parts of the community — [`Community::settings`],
    /// [`Community::handle_can_change`] and [`Community::handle_next_change_ts`].
    /// Admin-only: for non-admins the live API rejects the request with a 403
    /// (error code 23004) rather than ignoring the parameter.
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
    /// let info = crab.communities("rust-community").info_for_edit().await?;
    /// if let Some(settings) = info.settings {
    ///     println!("invite links enabled: {:?}", settings.invite_links_enabled);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn info_for_edit(&self) -> CrabResult<Community> {
        let path = format!("communities/{}", self.handle);
        self.client
            .get_with_query(&path, &ContextQuery { context: "edit" })
            .await
    }

    /// Edits the community's settings
    ///
    /// Requires admin privileges. Returns a builder for choosing which fields
    /// to change; on success the API responds with the updated community
    /// object (including the edit-context fields).
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
    /// let updated = crab.communities("rust-community")
    ///     .edit()
    ///     .title("Rustaceans")
    ///     .description("All things Rust")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn edit(&self) -> EditCommunityBuilder {
        EditCommunityBuilder::new(self.client.clone(), self.handle.clone())
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
    /// Joining without an invite hash succeeds when the community is
    /// free-to-join or the requesting user has a pending invitation. To join
    /// via an invite link, use
    /// [`join_with_invite_hash`](Self::join_with_invite_hash).
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
    /// - The community is at capacity (error code 23003)
    /// - The pending invitation no longer exists (error code 23015)
    /// - Network request fails
    /// - API returns an error
    pub async fn join(&self) -> CrabResult<EmptyResponse> {
        self.join_inner(None).await
    }

    /// Joins the community using an invite link's hash
    ///
    /// The hash is the trailing component of a community invite link
    /// (`https://www.tumblr.com/join/{invite_hash}`).
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
    ///     .join_with_invite_hash("abcd123")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the invite hash is invalid (400 with error code
    /// 23014), plus the same failure modes as [`join`](Self::join).
    pub async fn join_with_invite_hash(
        &self,
        invite_hash: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        self.join_inner(Some(invite_hash.into())).await
    }

    async fn join_inner(&self, invite_hash: Option<String>) -> CrabResult<EmptyResponse> {
        let path = format!("communities/{}/members", self.handle);
        self.client.put(&path, &JoinRequest { invite_hash }).await
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
    /// Returns a builder for configuring the members request. Results are
    /// paginated by page number (20 members per page); use
    /// [`MembersBuilder::page`] to fetch later pages and
    /// [`MembersBuilder::filter`] to restrict the result set.
    ///
    /// The full member list is only visible to members, invitees, and
    /// free-to-join public communities; admin and moderator lists are always
    /// visible for public communities.
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
    ///     .page(2)
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
    /// * `reason` - Why the member is being removed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::communities::RemovalReason;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// crab.communities("rust-community")
    ///     .remove_member("problematic-blog", RemovalReason::Spam)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remove_member(
        &self,
        blog: impl Into<BlogIdentifier>,
        reason: RemovalReason,
    ) -> CrabResult<EmptyResponse> {
        self.remove_member_with_ban(blog, reason, None).await
    }

    /// Removes a member from the community and bans them from rejoining
    ///
    /// Requires moderator or admin privileges.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier of the member to remove
    /// * `reason` - Why the member is being removed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::communities::RemovalReason;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// crab.communities("rust-community")
    ///     .ban_member("problematic-blog", RemovalReason::Spam)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ban_member(
        &self,
        blog: impl Into<BlogIdentifier>,
        reason: RemovalReason,
    ) -> CrabResult<EmptyResponse> {
        self.remove_member_with_ban(blog, reason, Some(true)).await
    }

    async fn remove_member_with_ban(
        &self,
        blog: impl Into<BlogIdentifier>,
        reason: RemovalReason,
        ban_user: Option<bool>,
    ) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!("communities/{}/members/{}", self.handle, blog_id.as_str());
        let (reason, note) = reason.into_wire();
        self.client
            .delete_with_body(
                &path,
                &RemoveMemberRequest {
                    reason,
                    note,
                    ban_user,
                },
            )
            .await
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
    /// Per Tumblr's docs, muting currently only hides the unread count.
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
    /// Requires admin privileges. Returns a builder; invitations are sorted
    /// in reverse chronological order, 20 per page, with a `_links` object
    /// providing the next page boundary when more remain.
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
    /// let invitations = crab.communities("rust-community")
    ///     .invitations()
    ///     .send()
    ///     .await?;
    /// for invite in invitations.invitations {
    ///     println!("Pending invite for: {}", invite.recipient.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn invitations(&self) -> InvitationsBuilder {
        InvitationsBuilder::new(self.client.clone(), self.handle.clone())
    }

    /// Invites a blog to join the community
    ///
    /// Requires admin privileges. Admins can send 250 invites per day at a
    /// maximum rate of 1 per second, across all of their communities.
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
        self.invite_with_optional_message(blog, None).await
    }

    /// Invites a blog to join the community with an accompanying message
    ///
    /// Requires admin privileges. The API caps the message at
    /// 100 characters; longer messages are rejected by the server.
    ///
    /// # Arguments
    ///
    /// * `blog` - The blog identifier to invite
    /// * `message` - A short message to include with the invitation (max 100 characters)
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
    ///     .invite_with_message("friend-blog", "Would love to have you here!")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invite_with_message(
        &self,
        blog: impl Into<BlogIdentifier>,
        message: impl Into<String>,
    ) -> CrabResult<EmptyResponse> {
        self.invite_with_optional_message(blog, Some(message.into()))
            .await
    }

    async fn invite_with_optional_message(
        &self,
        blog: impl Into<BlogIdentifier>,
        message: Option<String>,
    ) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!("communities/{}/invitations", self.handle);
        self.client
            .put(
                &path,
                &InviteRequest {
                    recipient_blog: blog_id.to_string(),
                    message,
                },
            )
            .await
    }

    /// Cancels an invitation to the community
    ///
    /// This serves both sides of an invitation: an admin cancelling a pending
    /// invite they sent, or a recipient declining an invite they received. To
    /// decline and also block future re-invitations, use
    /// [`cancel_invitation_do_not_invite`](Self::cancel_invitation_do_not_invite).
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

    /// Cancels an invitation and disallows any future re-invitations
    ///
    /// Sends the `do_not_invite` flag alongside the cancellation, which the
    /// API documents as the decline-and-block flow: the blog cannot be
    /// re-invited to this community afterwards.
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
    /// crab.communities("rust-community")
    ///     .cancel_invitation_do_not_invite("friend-blog")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel_invitation_do_not_invite(
        &self,
        blog: impl Into<BlogIdentifier>,
    ) -> CrabResult<EmptyResponse> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!(
            "communities/{}/invitations/{}",
            self.handle,
            blog_id.as_str()
        );
        self.client
            .delete_with_query(
                &path,
                &DoNotInviteQuery {
                    do_not_invite: true,
                },
            )
            .await
    }

    /// Checks the invitation status for a blog
    ///
    /// Returns `None` if there is no pending invitation for the blog (the API
    /// responds 404 in that case), or `Some` with the sender otherwise.
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
    /// if let Some(status) = status {
    ///     println!("invited by {}", status.sender.name);
    /// } else {
    ///     println!("no pending invitation");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invitation_status(
        &self,
        blog: impl Into<BlogIdentifier>,
    ) -> CrabResult<Option<InvitationStatus>> {
        let blog_id: BlogIdentifier = blog.into();
        let path = format!(
            "communities/{}/invitations/{}",
            self.handle,
            blog_id.as_str()
        );
        match self.client.get(&path).await {
            Ok(status) => Ok(Some(status)),
            Err(CrabError::Api { status: 404, .. }) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Regenerates the community invite URL
    ///
    /// Requires admin privileges. Note that this invalidates the previous
    /// invite link.
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
    /// println!("New invite link: {}", response.invite_link);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns 403 with error code 23017 if invite links are disabled for the
    /// community, or 23012 if the user is not allowed to regenerate the link.
    pub async fn regenerate_invite_url(&self) -> CrabResult<InviteLinkResponse> {
        let path = format!("communities/{}/invite_hash", self.handle);
        self.client.post(&path, &serde_json::json!({})).await
    }

    /// Adds a reaction to a post in the community
    ///
    /// On the first use of a reaction the API may respond with a timeline
    /// object containing a celebration banner instead of an empty body; both
    /// shapes are accepted and discarded.
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post to react to
    /// * `reaction` - The reaction to add
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::Crabrave;
    /// # use crabrave::handlers::communities::Reaction;
    /// # async fn example() -> Result<(), crabrave::CrabError> {
    /// # let crab = Crabrave::builder()
    /// #     .consumer_key("key")
    /// #     .consumer_secret("secret")
    /// #     .access_token("token")
    /// #     .build()?;
    /// crab.communities("rust-community")
    ///     .add_reaction("123456789", Reaction::slug(":heart:"))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add_reaction(
        &self,
        post_id: impl Into<String>,
        reaction: Reaction,
    ) -> CrabResult<EmptyResponse> {
        let path = format!(
            "communities/{}/posts/{}/reactions",
            self.handle,
            post_id.into()
        );
        self.client.put(&path, &reaction).await
    }

    /// Removes a reaction from a post in the community
    ///
    /// # Arguments
    ///
    /// * `post_id` - The ID of the post
    /// * `reaction_id` - The unique identifier of the reaction to remove, or
    ///   its emoji grapheme. Timeline posts expose reactions with an
    ///   `emoji.id` field usable here.
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
    ///     .remove_reaction("123456789", "👍")
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
    /// **Unverified endpoint:** the moderation endpoints appear nowhere in
    /// Tumblr's published API documentation, and their request/response
    /// shapes could not be confirmed against the live API.
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
    /// **Unverified endpoint:** the moderation endpoints appear nowhere in
    /// Tumblr's published API documentation, and their request/response
    /// shapes could not be confirmed against the live API.
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
    /// **Unverified endpoint:** the moderation endpoints appear nowhere in
    /// Tumblr's published API documentation, and their request/response
    /// shapes could not be confirmed against the live API.
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
struct ContextQuery {
    context: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct JoinRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    invite_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DoNotInviteQuery {
    do_not_invite: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ChangeRoleRequest {
    role: MemberRole,
}

#[derive(Debug, Clone, Serialize)]
struct InviteRequest {
    recipient_blog: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct RemoveMemberRequest {
    reason: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ban_user: Option<bool>,
}

// ============================================================================
// Response types
// ============================================================================

/// A Tumblr community
///
/// The info endpoint returns this object directly in the response envelope.
/// Only `name` and `uuid` are present in every context; Tumblr's "short"
/// community objects carry just `name`, `uuid`, `title`, `member_count`,
/// `mention_tag`, `can_view`, and `avatar_image`, and several fields are
/// role-gated (admin-only or edit-context-only), so everything else is
/// optional.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Community {
    /// Community name/handle, without the double-@ prefix
    pub name: String,
    /// Unique identifier of the community (usable as a community handle)
    pub uuid: String,
    /// Community display title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Community description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The About information of the community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Community guidelines, one string per guideline
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guidelines: Vec<String>,
    /// Short summary of the guidelines (observed on the live API; undocumented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidelines_summary: Option<String>,
    /// Community language code, e.g. `en_US` (observed on the live API; undocumented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Community visibility, `public` or `private`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    /// Number of members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u64>,
    /// How many members are "online" now
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members_online_count: Option<u64>,
    /// The community's handle with the double-@ prefix, e.g. `@@community-name`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_tag: Option<String>,
    /// Whether the current user is a member
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_member: Option<bool>,
    /// Whether the community can be viewed by the current user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_view: Option<bool>,
    /// Whether the current user can post to this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_post: Option<bool>,
    /// Whether the current user can edit this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_edit: Option<bool>,
    /// Whether the current user can interact with posts in this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_interact: Option<bool>,
    /// Whether the current user can invite others to this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_invite: Option<bool>,
    /// Whether the current user can view comments in this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_view_comments: Option<bool>,
    /// Whether the current user can view the full member list (observed on
    /// the live API; undocumented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_view_all_members: Option<bool>,
    /// The number of posts published in this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_count: Option<u64>,
    /// The number of "unread" posts for the current user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unread_post_count: Option<u64>,
    /// The tags used to describe this community
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Unix timestamp of when this community was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_ts: Option<i64>,
    /// The current user's invitation, present only if they have one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitation: Option<CommunityInvitation>,
    /// Whether the community handle can be changed (admins, edit context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle_can_change: Option<bool>,
    /// If the handle cannot be changed, the next time it can be (admins, edit context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle_next_change_ts: Option<i64>,
    /// The community's current settings values (admins, edit context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<CommunitySettings>,
    /// The community's avatar, in various sizes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub avatar_image: Vec<MediaObject>,
    /// The community's header image, in various sizes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header_image: Vec<MediaObject>,
    /// The population cap of the community, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub population_cap: Option<u64>,
    /// How the community can be joined
    ///
    /// Tumblr documents `invite` and `free`, but the live API also returns
    /// undocumented values (e.g. `request`), so this is left as a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_type: Option<String>,
    /// The invite link to join the community (admins only; `None` if disabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_link: Option<String>,
    /// How many pending invitations there are (admins only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending_invitations_count: Option<u64>,
    /// The current user's role in the community (observed on the live API;
    /// undocumented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MemberRole>,
    /// Whether the current user has muted the community (observed on the
    /// live API; undocumented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_muted: Option<bool>,
    /// The current user's blog within this community (observed on the live
    /// API; undocumented)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<CondensedBlog>,
}

/// The current user's pending invitation, embedded in a [`Community`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityInvitation {
    /// The kind of invitation, e.g. `direct`
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub invitation_type: Option<String>,
    /// Blog that sent the invitation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<CondensedBlog>,
}

/// Editable community settings, visible to admins in the edit context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunitySettings {
    /// Whether an invite link is available for this community
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invite_links_enabled: Option<bool>,
    /// What join options non-members have, `free` or `invite`
    ///
    /// Note that the top-level [`Community::join_type`] takes precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_type: Option<String>,
}

/// Member role in a community
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum MemberRole {
    /// Regular member
    Member,
    /// Community moderator
    Moderator,
    /// Community administrator
    Admin,
    /// Catch-all for unrecognized roles.
    /// Prevents deserialization failures when the API introduces new roles.
    #[serde(other)]
    Unknown,
}

/// A reaction to add to a community post
///
/// The API accepts either a named slug (e.g. `:thumbs_up:`) or a raw unicode
/// emoji grapheme (e.g. 👍); if both were somehow sent, slug takes precedence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reaction {
    /// A named reaction slug, e.g. `:thumbs_up:`
    Slug(String),
    /// A raw unicode emoji, e.g. 👍
    Grapheme(String),
}

impl Reaction {
    /// Creates a reaction from a named slug, e.g. `:thumbs_up:`
    pub fn slug(s: impl Into<String>) -> Self {
        Self::Slug(s.into())
    }

    /// Creates a reaction from a raw unicode emoji, e.g. 👍
    pub fn grapheme(g: impl Into<String>) -> Self {
        Self::Grapheme(g.into())
    }
}

/// Reason for removing a member from a community
///
/// Required by the moderation endpoint; the `Other` variant carries the
/// mandatory note the API requires when no other reason fits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemovalReason {
    /// Wire value: `community_guidelines`
    CommunityGuidelines,
    /// Wire value: `off_topic`
    OffTopic,
    /// Wire value: `spam`
    Spam,
    /// Wire value: `inappropriate_behavior`
    InappropriateBehavior,
    /// Wire value: `tumblr_guidelines`
    TumblrGuidelines,
    /// Wire value: `other`, with a required explanatory note
    Other {
        /// Explanation for the removal, required by the API when reason is `other`
        note: String,
    },
}

impl RemovalReason {
    fn into_wire(self) -> (&'static str, Option<String>) {
        match self {
            Self::CommunityGuidelines => ("community_guidelines", None),
            Self::OffTopic => ("off_topic", None),
            Self::Spam => ("spam", None),
            Self::InappropriateBehavior => ("inappropriate_behavior", None),
            Self::TumblrGuidelines => ("tumblr_guidelines", None),
            Self::Other { note } => ("other", Some(note)),
        }
    }
}

/// Envelope for the timeline endpoint: the timeline object arrives nested
/// under a `timeline` key inside the response envelope.
#[derive(Debug, Clone, Deserialize)]
struct TimelineEnvelope {
    timeline: TimelineResponse,
}

/// A page of a community timeline
///
/// The wire format nests posts under `timeline.elements`; this type flattens
/// that away and exposes them as `posts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineResponse {
    /// Posts on this page of the timeline
    #[serde(rename = "elements", default)]
    pub posts: Vec<crate::handlers::blog::Post>,
    /// Pagination links; `next` carries the cursor for the following page
    #[serde(rename = "_links", default, skip_serializing_if = "Option::is_none")]
    pub links: Option<TimelineLinks>,
}

/// Pagination links for a community timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLinks {
    /// Link to fetch the next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<TimelineNextLink>,
}

/// Next page link for a community timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineNextLink {
    /// Path (with query string) of the next page
    pub href: String,
    /// HTTP method for the next request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Query parameters for the next page
    #[serde(default)]
    pub query_params: TimelineNextQueryParams,
}

/// Query parameters embedded in a timeline next-page link
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineNextQueryParams {
    /// Opaque pagination cursor, passed to [`TimelineBuilder::cursor`]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Response from the members endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembersResponse {
    /// Total number of members in the community (the API returns this as `count`)
    #[serde(rename = "count")]
    pub total_members: u64,
    /// List of community members
    pub members: Vec<CommunityMember>,
}

/// A member of a community
///
/// The members endpoint returns each member as a blog object with the
/// community-specific fields inlined alongside it, so the blog fields are
/// flattened rather than nested under a `blog` key. Roles are expressed as the
/// `is_admin`/`is_moderator` flags rather than a single role field. Whether
/// the requesting user follows this member is available as `blog.followed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityMember {
    /// Member's blog information
    #[serde(flatten)]
    pub blog: Blog,
    /// Whether this member is a community administrator
    #[serde(default)]
    pub is_admin: bool,
    /// Whether this member is a community moderator
    #[serde(default)]
    pub is_moderator: bool,
    /// Whether this member follows the authenticated user
    #[serde(default)]
    pub is_following_you: bool,
    /// Whether this member is currently online
    #[serde(default)]
    pub is_online: bool,
    /// Whether the authenticated user and this member follow each other
    #[serde(default)]
    pub is_mutual: bool,
    /// Unix timestamp of when the member joined the community
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_ts: Option<i64>,
}

/// Response from the invitations endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationsResponse {
    /// List of pending invitations, in reverse chronological order
    pub invitations: Vec<Invitation>,
    /// Pagination links, present when more than one page of invitations exists
    #[serde(rename = "_links", default, skip_serializing_if = "Option::is_none")]
    pub links: Option<InvitationLinks>,
}

/// Pagination links for the invitations endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationLinks {
    /// Link to fetch the next page of invitations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<InvitationNextLink>,
}

/// Next page link for the invitations endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationNextLink {
    /// Path (with query string) of the next page
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    /// HTTP method for the next request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Query parameters for the next page
    #[serde(default)]
    pub query_params: InvitationNextQueryParams,
}

/// Query parameters embedded in an invitations next-page link
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InvitationNextQueryParams {
    /// Timestamp boundary for the next page, passed to
    /// [`InvitationsBuilder::before`]
    ///
    /// Kept as a string: sibling `_links` on other endpoints quote their
    /// timestamps, and this endpoint's shape is admin-gated so it could not
    /// be confirmed live. Numbers are coerced.
    #[serde(
        default,
        deserialize_with = "deserialize_opt_timestamp_as_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub before: Option<String>,
}

/// Deserializes an optional timestamp leniently, accepting either a quoted
/// string or a bare number and normalizing to a string.
fn deserialize_opt_timestamp_as_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        serde_json::Value::Null => Ok(None),
        other => Err(serde::de::Error::custom(format!(
            "expected string or number timestamp, got {}",
            crate::kind_of(&other),
        ))),
    }
}

/// A pending community invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invitation {
    /// Blog that sent the invitation
    pub sender: CondensedBlog,
    /// Blog that was invited
    pub recipient: CondensedBlog,
    /// Unix timestamp of when the invitation was sent
    pub sent_on: i64,
}

/// Condensed blog object embedded in community responses.
///
/// Only `name` is guaranteed; common blog fields are captured when present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CondensedBlog {
    /// Blog name
    pub name: String,
    /// Blog title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Blog URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Blog UUID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Blog description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Last update timestamp (Unix time)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<i64>,
    /// Blog avatar, in various sizes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub avatar: Vec<MediaObject>,
}

/// Former name of [`CondensedBlog`], kept as an alias for compatibility
pub type InvitationBlog = CondensedBlog;

/// A pending community invitation, as reported by the invitation-status endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationStatus {
    /// Blog that sent the invitation
    pub sender: CondensedBlog,
}

/// Response from regenerating the community invite URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteLinkResponse {
    /// The new invite link
    pub invite_link: String,
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

/// Which community timeline to fetch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimelineType {
    /// Recent posts (the default)
    Recent,
    /// Poll posts
    Polls,
    /// Discovery timeline
    Discover,
}

/// Query parameters for community timeline
#[derive(Debug, Clone, Serialize, Default)]
struct TimelineQuery {
    /// Which timeline to fetch (`recent`, `polls`, `discover`)
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    timeline_type: Option<TimelineType>,

    /// Opaque cursor from `_links.next` for fetching the following page
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,

    /// Return posts in NPF format
    #[serde(skip_serializing_if = "Option::is_none")]
    npf: Option<bool>,
}

/// Builder for querying a community timeline
///
/// Pagination is cursor-based: the response's `links.next.query_params.cursor`
/// value is fed back via [`cursor`](Self::cursor) to fetch the next page.
/// (The endpoint silently ignores `limit`/`offset`-style parameters.)
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

    /// Selects which timeline to fetch (defaults to recent)
    pub fn timeline_type(mut self, timeline_type: TimelineType) -> Self {
        self.query.timeline_type = Some(timeline_type);
        self
    }

    /// Sets the pagination cursor, taken from a previous response's
    /// `links.next.query_params.cursor`
    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.query.cursor = Some(cursor.into());
        self
    }

    /// Sends the request and returns the timeline posts in NPF format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The community doesn't exist or can't be viewed (403 with the info
    ///   endpoint's error codes)
    /// - Network request fails
    /// - API returns an error
    pub async fn send(mut self) -> CrabResult<TimelineResponse> {
        // npf=true is undocumented for this endpoint but honored by the live
        // API; without it, elements arrive in legacy (non-blocks) format.
        self.query.npf = Some(true);
        let path = format!("communities/{}/timeline", self.handle);
        let envelope: TimelineEnvelope = self.client.get_with_query(&path, &self.query).await?;
        Ok(envelope.timeline)
    }
}

/// Which subset of community members to list
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberFilter {
    /// All members (the default)
    All,
    /// Only admins
    Admin,
    /// Only moderators
    Moderator,
    /// Only blogs the requesting user is following
    Following,
    /// Only blogs with a mutual following relationship
    Mutual,
}

/// Query parameters for community members
#[derive(Debug, Clone, Serialize, Default)]
struct MembersQuery {
    /// The page of results to fetch (default 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,

    /// Filter to a specific set of members
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<MemberFilter>,
}

/// Builder for querying community members
///
/// Results are paginated by page number, 20 members per page.
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

    /// Sets the page of results to fetch (pages start at 1)
    pub fn page(mut self, page: u32) -> Self {
        self.query.page = Some(page);
        self
    }

    /// Restricts the results to a specific set of members
    pub fn filter(mut self, filter: MemberFilter) -> Self {
        self.query.filter = Some(filter);
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

/// Query parameters for pending invitations
#[derive(Debug, Clone, Serialize, Default)]
struct InvitationsQuery {
    /// The timestamp to use for the next page boundary
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<i64>,
}

/// Builder for listing a community's pending invitations
pub struct InvitationsBuilder {
    client: Crabrave,
    handle: String,
    query: InvitationsQuery,
}

impl InvitationsBuilder {
    fn new(client: Crabrave, handle: String) -> Self {
        Self {
            client,
            handle,
            query: InvitationsQuery::default(),
        }
    }

    /// Returns invitations sent before this timestamp, taken from a previous
    /// response's `links.next.query_params.before`
    pub fn before(mut self, timestamp: i64) -> Self {
        self.query.before = Some(timestamp);
        self
    }

    /// Sends the request and returns the pending invitations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication is invalid
    /// - The requesting user is not an admin of the community (403)
    /// - Network request fails
    /// - API returns an error
    pub async fn send(self) -> CrabResult<InvitationsResponse> {
        let path = format!("communities/{}/invitations", self.handle);
        self.client.get_with_query(&path, &self.query).await
    }
}

/// Privacy setting for a community
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunityPrivacy {
    /// Anyone can view the community
    Public,
    /// Only members can view the community
    Private,
}

impl Serialize for CommunityPrivacy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // The API expects a number: 0 for public, 1 for private.
        serializer.serialize_u8(match self {
            Self::Public => 0,
            Self::Private => 1,
        })
    }
}

/// Body for editing a community
#[derive(Debug, Clone, Serialize, Default)]
struct EditCommunityRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    privacy: Option<CommunityPrivacy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    about: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guidelines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invite_links_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    join_type: Option<String>,
}

/// Builder for editing a community's settings
///
/// Only the fields that are explicitly set are sent, so unset fields keep
/// their current values.
pub struct EditCommunityBuilder {
    client: Crabrave,
    handle: String,
    body: EditCommunityRequest,
}

impl EditCommunityBuilder {
    fn new(client: Crabrave, handle: String) -> Self {
        Self {
            client,
            handle,
            body: EditCommunityRequest::default(),
        }
    }

    /// Sets the title of the community
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.body.title = Some(title.into());
        self
    }

    /// Sets the description/tagline of the community
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.body.description = Some(description.into());
        self
    }

    /// Changes the handle/URL of the community
    ///
    /// The handle can only be edited once every 7 days; check
    /// [`Community::handle_can_change`] and [`Community::handle_next_change_ts`]
    /// first. Updating the handle invalidates the previous community URL and
    /// frees the old handle up to be taken by another community.
    pub fn handle(mut self, handle: impl Into<String>) -> Self {
        self.body.handle = Some(handle.into());
        self
    }

    /// Sets the community's privacy
    pub fn privacy(mut self, privacy: CommunityPrivacy) -> Self {
        self.body.privacy = Some(privacy);
        self
    }

    /// Sets the tags that describe the community
    pub fn tags(mut self, tags: Vec<impl Into<String>>) -> Self {
        self.body.tags = Some(tags.into_iter().map(|t| t.into()).collect());
        self
    }

    /// Sets the About information of the community
    pub fn about(mut self, about: impl Into<String>) -> Self {
        self.body.about = Some(about.into());
        self
    }

    /// Sets the community guidelines, one string per guideline
    pub fn guidelines(mut self, guidelines: Vec<impl Into<String>>) -> Self {
        self.body.guidelines = Some(guidelines.into_iter().map(|g| g.into()).collect());
        self
    }

    /// Enables or disables invite links for the community
    pub fn invite_links_enabled(mut self, enabled: bool) -> Self {
        self.body.invite_links_enabled = Some(enabled);
        self
    }

    /// Sets the join type
    ///
    /// Tumblr documents `invite` and `free`; the live API also uses
    /// undocumented values (e.g. `request`), so this accepts any string.
    pub fn join_type(mut self, join_type: impl Into<String>) -> Self {
        self.body.join_type = Some(join_type.into());
        self
    }

    /// Sends the edit and returns the updated community
    ///
    /// The response includes the edit-context fields, as if `context=edit`
    /// had been passed to the info endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A provided parameter is invalid (400)
    /// - The user doesn't have permission to edit the community (403)
    /// - The API failed to save the community (503)
    /// - Network request fails
    pub async fn send(self) -> CrabResult<Community> {
        let path = format!("communities/{}", self.handle);
        self.client.put(&path, &self.body).await
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn test_client() -> Crabrave {
        Crabrave::builder()
            .consumer_key("test")
            .consumer_secret("test")
            .access_token("test")
            .build()
            .unwrap()
    }

    #[test]
    fn test_timeline_builder() {
        let builder = TimelineBuilder::new(test_client(), "rust-community".to_string())
            .timeline_type(TimelineType::Polls)
            .cursor("abc123");

        assert_eq!(builder.handle, "rust-community");
        assert_eq!(builder.query.timeline_type, Some(TimelineType::Polls));
        assert_eq!(builder.query.cursor, Some("abc123".to_string()));
    }

    #[test]
    fn test_timeline_builder_defaults() {
        let builder = TimelineBuilder::new(test_client(), "rust".to_string());

        assert_eq!(builder.handle, "rust");
        assert!(builder.query.timeline_type.is_none());
        assert!(builder.query.cursor.is_none());
    }

    #[test]
    fn test_timeline_type_serialization() {
        assert_eq!(
            serde_urlencoded::to_string([("type", TimelineType::Recent)]).unwrap(),
            "type=recent"
        );
        assert_eq!(
            serde_urlencoded::to_string([("type", TimelineType::Polls)]).unwrap(),
            "type=polls"
        );
        assert_eq!(
            serde_urlencoded::to_string([("type", TimelineType::Discover)]).unwrap(),
            "type=discover"
        );
    }

    #[test]
    fn test_communities_struct() {
        let communities = Communities::new(test_client(), "rust-community".to_string());

        assert_eq!(communities.handle, "rust-community");
    }

    #[test]
    fn test_members_builder() {
        let builder = MembersBuilder::new(test_client(), "rust-community".to_string())
            .page(2)
            .filter(MemberFilter::Admin);

        assert_eq!(builder.handle, "rust-community");
        assert_eq!(builder.query.page, Some(2));
        assert_eq!(builder.query.filter, Some(MemberFilter::Admin));
    }

    #[test]
    fn test_member_filter_serialization() {
        for (filter, expected) in [
            (MemberFilter::All, "filter=all"),
            (MemberFilter::Admin, "filter=admin"),
            (MemberFilter::Moderator, "filter=moderator"),
            (MemberFilter::Following, "filter=following"),
            (MemberFilter::Mutual, "filter=mutual"),
        ] {
            assert_eq!(
                serde_urlencoded::to_string([("filter", filter)]).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn test_invitations_builder() {
        let builder =
            InvitationsBuilder::new(test_client(), "rust-community".to_string()).before(1234567890);

        assert_eq!(builder.handle, "rust-community");
        assert_eq!(builder.query.before, Some(1234567890));
    }

    #[test]
    fn test_community_privacy_serialization() {
        assert_eq!(
            serde_json::to_string(&CommunityPrivacy::Public).unwrap(),
            "0"
        );
        assert_eq!(
            serde_json::to_string(&CommunityPrivacy::Private).unwrap(),
            "1"
        );
    }

    #[test]
    fn test_edit_builder_only_sends_set_fields() {
        let builder = EditCommunityBuilder::new(test_client(), "rust-community".to_string())
            .title("New Title")
            .privacy(CommunityPrivacy::Private);

        let body = serde_json::to_value(&builder.body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"title": "New Title", "privacy": 1})
        );
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
        assert_eq!(
            serde_json::to_string(&MemberRole::Unknown).unwrap(),
            "\"unknown\""
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
        assert_eq!(
            serde_json::from_str::<MemberRole>("\"owner\"").unwrap(),
            MemberRole::Unknown
        );
    }

    #[test]
    fn test_timeline_response_flattens_elements() {
        let json = serde_json::json!({
            "timeline": {
                "elements": [],
                "_links": {
                    "next": {
                        "href": "/v2/communities/x/timeline?cursor=abc",
                        "method": "GET",
                        "query_params": { "cursor": "abc", "rawurldecode": "1" }
                    }
                }
            }
        });
        let envelope: TimelineEnvelope = serde_json::from_value(json).unwrap();
        let timeline = envelope.timeline;
        assert!(timeline.posts.is_empty());
        let next = timeline.links.unwrap().next.unwrap();
        assert_eq!(next.query_params.cursor.as_deref(), Some("abc"));
    }

    #[test]
    fn test_invitation_next_query_params_lenient_before() {
        let quoted: InvitationNextQueryParams =
            serde_json::from_str(r#"{"before": "1234"}"#).unwrap();
        assert_eq!(quoted.before.as_deref(), Some("1234"));

        let bare: InvitationNextQueryParams = serde_json::from_str(r#"{"before": 1234}"#).unwrap();
        assert_eq!(bare.before.as_deref(), Some("1234"));

        let absent: InvitationNextQueryParams = serde_json::from_str("{}").unwrap();
        assert!(absent.before.is_none());
    }
}
