use crate::{Blog, BlogIdentifier, CrabResult, Crabrave};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct FollowingQuery {
    /// Maximum number of posts to return (API max: 20, default: 20)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    /// Post offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

pub struct FollowingBuilder {
    client: Crabrave,
    query: FollowingQuery,
    blog: Option<BlogIdentifier>,
}

impl FollowingBuilder {
    pub fn user(client: Crabrave) -> Self {
        Self {
            client,
            query: FollowingQuery::default(),
            blog: None,
        }
    }

    pub fn blog(client: Crabrave, name: impl Into<BlogIdentifier>) -> Self {
        Self {
            client,
            query: FollowingQuery::default(),
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

    pub async fn send(self) -> CrabResult<FollowingResponse> {
        match self.blog {
            Some(id) => {
                self.client
                    .get_with_query(format!("blog/{}/following", id).as_str(), &self.query)
                    .await
            }
            None => {
                self.client
                    .get_with_query("user/following", &self.query)
                    .await
            }
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct FollowingResponse {
    pub blogs: Vec<Blog>,
    pub total_blogs: u64,
}
