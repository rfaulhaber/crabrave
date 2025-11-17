use reqwest::Url;
use thiserror::Error;

const BASE_API_URL: &str = "https://api.tumblr.com/v2";
const OAUTH_AUTHORIZE_URL: &str = "www.tumblr.com/oauth2/authorize";
const OAUTH_TOKEN_URL: &str = "https://api.tumblr.com/v2/oauth2/token";

#[derive(Debug, Error)]
pub enum CrabClientBuilderError {
    #[error("Missing consumer key")]
    MissingConsumerKey,
    #[error("Missing consumer secret")]
    MissingConsumerSecret,
}

pub struct CrabClient {
    reqwest: reqwest::Client,
}

pub struct CrabClientBuilder {
    consumer_key: Option<String>,
    consumer_secret: Option<String>,
}

impl CrabClientBuilder {
    pub fn consumer_key(mut self, key: String) -> Self {
        self.consumer_key = Some(key);
        self
    }

    pub fn consumer_secret(mut self, secret: String) -> Self {
        self.consumer_secret = Some(secret);
        self
    }
}
