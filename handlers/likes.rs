//! Likes are their own module as likes can be retrieved relative to a blog or a user. They return the same information.

use crate::{BlogIdentifier, CrabResult, Crabrave};
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
}

/// Builder for querying the user's liked posts
pub struct LikesBuilder {
    client: Crabrave,
    query: LikesQuery,
    blog: Option<BlogIdentifier>,
}

impl LikesBuilder {
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

    /// Sends the request and returns the liked posts
    pub async fn get(self) -> CrabResult<LikesResponse> {
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

/// Response from the likes endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LikesResponse {
    /// List of liked posts
    pub liked_posts: Vec<crate::handlers::blog::Post>,
    /// Total number of liked posts
    #[serde(default)]
    pub liked_count: u64,
}
