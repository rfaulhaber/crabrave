//! Likes are their own module as likes can be retrieved relative to a blog or a user. They return the same information.

use crate::{BlogIdentifier, CrabError, CrabResult, Crabrave};
use serde::{Deserialize, Serialize};

/// Query parameters for user likes
#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct LikesQuery {
    /// Maximum number of posts to return (API max: 20, default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Post offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,

    /// Return posts liked before this timestamp (Unix time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<i64>,

    /// Return posts liked after this timestamp (Unix time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<i64>,

    /// Return posts in NPF format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npf: Option<bool>,
}

/// Builder for querying the user's liked posts
///
/// Posts are always returned in NPF (Neue Post Format) with structured content blocks.
pub struct LikesBuilder {
    client: Crabrave,
    query: LikesQuery,
    blog: Option<BlogIdentifier>,
}

impl LikesBuilder {
    #[cfg(test)]
    pub(crate) fn query(&self) -> &LikesQuery {
        &self.query
    }

    pub fn user(client: Crabrave) -> Self {
        Self {
            client,
            query: LikesQuery::default(),
            blog: None,
        }
    }

    pub fn blog(client: Crabrave, name: impl Into<BlogIdentifier>) -> Self {
        Self {
            client,
            query: LikesQuery::default(),
            blog: Some(name.into()),
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

    /// Returns posts liked before this timestamp (Unix time)
    pub fn before(mut self, timestamp: i64) -> Self {
        self.query.before = Some(timestamp);
        self
    }

    /// Returns posts liked after this timestamp (Unix time)
    pub fn after(mut self, timestamp: i64) -> Self {
        self.query.after = Some(timestamp);
        self
    }

    /// Sends the request and returns the liked posts in NPF format
    ///
    /// # Errors
    ///
    /// Returns [`CrabError::InvalidParameter`] if more than one of
    /// [`offset`](Self::offset), [`before`](Self::before), or
    /// [`after`](Self::after) is set — the API treats them as mutually
    /// exclusive pagination modes.
    pub async fn send(mut self) -> CrabResult<LikesResponse> {
        let pagination_modes = [
            self.query.offset.is_some(),
            self.query.before.is_some(),
            self.query.after.is_some(),
        ];
        if pagination_modes.iter().filter(|set| **set).count() > 1 {
            return Err(CrabError::InvalidParameter(
                "offset, before, and after are mutually exclusive".to_string(),
            ));
        }

        // Always request NPF format
        self.query.npf = Some(true);
        match self.blog {
            Some(id) => {
                self.client
                    .get_with_query(format!("blog/{}/likes", id).as_str(), &self.query)
                    .await
            }
            None => self.client.get_with_query("user/likes", &self.query).await,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_rejects_combined_pagination_params() {
        let crab = Crabrave::builder().consumer_key("key").build().unwrap();

        let result = LikesBuilder::user(crab).before(1).after(2).send().await;
        assert!(matches!(result, Err(CrabError::InvalidParameter(_))));
    }

    #[tokio::test]
    async fn test_send_rejects_offset_with_before() {
        let crab = Crabrave::builder().consumer_key("key").build().unwrap();

        let result = LikesBuilder::user(crab).offset(20).before(1).send().await;
        assert!(matches!(result, Err(CrabError::InvalidParameter(_))));
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
    /// Pagination cursors
    #[serde(rename = "_links", default, skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::models::Links>,
}
