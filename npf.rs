//! Neue Post Format (NPF) types
//!
//! NPF is Tumblr's modern content representation system that uses structured content blocks
//! instead of legacy post types. It provides a flexible way to create rich, mixed-media posts.
//!
//! See [this](https://www.tumblr.com/docs/npf) for Tumblr's NPF specification.

use serde::{Deserialize, Serialize};

/// A content block in the NPF format
///
/// Content blocks are the building pieces of NPF posts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    /// Text content block
    Text {
        /// The text content
        text: String,
        /// Optional subtype (heading1, heading2, indented, quote, ordered-list-item, unordered-list-item, chat, quirky)
        #[serde(skip_serializing_if = "Option::is_none")]
        subtype: Option<String>,
        /// Optional inline formatting
        #[serde(skip_serializing_if = "Option::is_none")]
        formatting: Option<Vec<InlineFormat>>,
    },
    /// Image content block
    Image {
        /// Media objects for this image
        media: Vec<MediaObject>,
        /// Optional alt text
        #[serde(skip_serializing_if = "Option::is_none")]
        alt_text: Option<String>,
        /// Optional caption
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        /// Optional attribution for the image source
        #[serde(skip_serializing_if = "Option::is_none")]
        attribution: Option<Attribution>,
    },
    /// Link content block
    Link {
        /// The URL to link to
        url: String,
        /// Optional display text
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Optional description
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Optional poster/thumbnail images
        #[serde(skip_serializing_if = "Option::is_none")]
        poster: Option<Vec<MediaObject>>,
    },
    /// Audio content block
    Audio {
        /// Media objects for this audio (used for Tumblr-hosted audio)
        #[serde(skip_serializing_if = "Option::is_none")]
        media: Option<Vec<MediaObject>>,
        /// External audio URL (used for external providers like Spotify, Soundcloud)
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// Audio provider (tumblr, spotify, soundcloud, bandcamp, etc.)
        #[serde(skip_serializing_if = "Option::is_none")]
        provider: Option<String>,
        /// Optional artist name
        #[serde(skip_serializing_if = "Option::is_none")]
        artist: Option<String>,
        /// Optional album name
        #[serde(skip_serializing_if = "Option::is_none")]
        album: Option<String>,
        /// Optional track title
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Optional embed HTML for external providers
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_html: Option<String>,
        /// Optional embed URL for external providers
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_url: Option<String>,
        /// Optional poster/thumbnail images
        #[serde(skip_serializing_if = "Option::is_none")]
        poster: Option<Vec<MediaObject>>,
        /// Optional attribution for the audio source
        #[serde(skip_serializing_if = "Option::is_none")]
        attribution: Option<Attribution>,
        /// Optional metadata from the provider
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<AudioMetadata>,
    },
    /// Video content block
    Video {
        /// Media objects for this video (used for Tumblr-hosted video)
        #[serde(skip_serializing_if = "Option::is_none")]
        media: Option<Vec<MediaObject>>,
        /// External video URL (used for external providers like YouTube, Vimeo)
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// Video provider (tumblr, youtube, vimeo, etc.)
        #[serde(skip_serializing_if = "Option::is_none")]
        provider: Option<String>,
        /// Optional embed HTML for external providers
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_html: Option<String>,
        /// Optional embed iframe for external providers
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_iframe: Option<EmbedIframe>,
        /// Optional embed URL for external providers
        #[serde(skip_serializing_if = "Option::is_none")]
        embed_url: Option<String>,
        /// Optional poster/thumbnail images
        #[serde(skip_serializing_if = "Option::is_none")]
        poster: Option<Vec<MediaObject>>,
        /// Optional attribution for the video source
        #[serde(skip_serializing_if = "Option::is_none")]
        attribution: Option<Attribution>,
        /// Whether the video can autoplay on cellular connections
        #[serde(skip_serializing_if = "Option::is_none")]
        can_autoplay_on_cellular: Option<bool>,
        /// Video duration in seconds
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<f64>,
        /// Optional metadata from the provider
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<VideoMetadata>,
    },
    /// Paywall content block for premium content
    Paywall {
        /// Paywall subtype
        #[serde(skip_serializing_if = "Option::is_none")]
        subtype: Option<PaywallSubtype>,
        /// URL for the paywall CTA
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        /// Text for the paywall CTA
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        /// Whether the paywall is for supporters only
        #[serde(skip_serializing_if = "Option::is_none")]
        is_visible: Option<bool>,
    },
}

/// Inline formatting for text blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineFormat {
    /// Start position in the text (inclusive)
    pub start: usize,
    /// End position in the text (exclusive)
    pub end: usize,
    /// Format type (bold, italic, strikethrough, small, link, mention, color)
    #[serde(rename = "type")]
    pub format_type: String,
    /// Optional URL (for link format type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional blog reference (for mention format type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<BlogReference>,
    /// Optional hex color (for color format type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
}

/// Reference to a blog (for mentions)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogReference {
    /// Blog UUID
    pub uuid: String,
    /// Blog name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Blog URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Media object for images, videos, and audio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaObject {
    /// Media URL (empty string when uploading new media)
    #[serde(default)]
    pub url: String,
    /// Media type (image/jpg, video/mp4, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    /// Identifier for linking to multipart upload data
    ///
    /// When uploading media, this identifier is used as the field name in the
    /// multipart/form-data request. Leave empty when referencing existing media by URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// Width in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Height in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Whether the original dimensions are missing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_dimensions_missing: Option<bool>,
    /// Whether this is a cropped version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cropped: Option<bool>,
    /// Whether the media has original dimensions available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_original_dimensions: Option<bool>,
}

/// Attribution for content sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Attribution {
    /// Attribution to another post
    Post {
        /// URL of the source post
        url: String,
        /// Post data
        #[serde(skip_serializing_if = "Option::is_none")]
        post: Option<AttributionPost>,
        /// Blog data
        #[serde(skip_serializing_if = "Option::is_none")]
        blog: Option<AttributionBlog>,
    },
    /// Attribution to a link/website
    Link {
        /// URL of the source
        url: String,
    },
    /// Attribution to a blog
    Blog {
        /// Blog data
        #[serde(skip_serializing_if = "Option::is_none")]
        blog: Option<AttributionBlog>,
    },
    /// Attribution to an app
    App {
        /// URL of the app
        url: String,
        /// App name
        #[serde(skip_serializing_if = "Option::is_none")]
        app_name: Option<String>,
        /// App display text
        #[serde(skip_serializing_if = "Option::is_none")]
        display_text: Option<String>,
        /// App logo
        #[serde(skip_serializing_if = "Option::is_none")]
        logo: Option<MediaObject>,
    },
}

/// Post data for attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionPost {
    /// Post ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Blog data for attribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionBlog {
    /// Blog UUID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Blog name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Blog URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Audio metadata from external providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetadata {
    /// Track ID from the provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Video metadata from external providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    /// Video ID from the provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// Embed iframe configuration for video embeds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedIframe {
    /// URL for the iframe src
    pub url: String,
    /// Width of the iframe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Height of the iframe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// Paywall subtype for premium content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaywallSubtype {
    /// Call-to-action paywall
    Cta,
    /// Divider paywall (content below is hidden)
    Divider,
    /// Disabled paywall
    Disabled,
}

/// Layout information for NPF posts
///
/// Layouts control how content blocks are arranged visually.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LayoutBlock {
    /// Rows layout - arrange blocks in rows
    Rows {
        /// Display configuration
        display: Vec<DisplayBlock>,
        /// Index after which to truncate (for "read more" functionality)
        #[serde(skip_serializing_if = "Option::is_none")]
        truncate_after: Option<usize>,
    },
    /// Ask layout - for ask/answer posts
    Ask {
        /// Content block indices that form the ask
        blocks: Vec<usize>,
        /// Attribution for who sent the ask
        #[serde(skip_serializing_if = "Option::is_none")]
        attribution: Option<AskAttribution>,
    },
}

/// Display block within a layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayBlock {
    /// Indices of the content blocks to display in this row
    pub blocks: Vec<usize>,
    /// Display mode for the blocks in this row
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<DisplayMode>,
}

/// Display mode for blocks in a row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMode {
    /// Mode type (weighted, carousel)
    #[serde(rename = "type")]
    pub mode_type: String,
}

/// Attribution for ask layouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskAttribution {
    /// Type of attribution (blog or anonymous)
    #[serde(rename = "type")]
    pub attribution_type: String,
    /// URL of the asking blog (for blog attribution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Blog information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blog: Option<AttributionBlog>,
}

impl ContentBlock {
    /// Creates a simple text content block
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text {
            text: text.into(),
            subtype: None,
            formatting: None,
        }
    }

    /// Creates a heading text block
    pub fn heading(text: impl Into<String>, level: u8) -> Self {
        let subtype = match level {
            1 => "heading1",
            2 => "heading2",
            _ => "heading1",
        };
        ContentBlock::Text {
            text: text.into(),
            subtype: Some(subtype.to_string()),
            formatting: None,
        }
    }

    /// Creates a link content block
    pub fn link(url: impl Into<String>) -> Self {
        ContentBlock::Link {
            url: url.into(),
            title: None,
            description: None,
            poster: None,
        }
    }

    /// Creates an image content block from a URL
    pub fn image(url: impl Into<String>) -> Self {
        ContentBlock::Image {
            media: vec![MediaObject {
                url: url.into(),
                media_type: None,
                identifier: None,
                width: None,
                height: None,
                original_dimensions_missing: None,
                cropped: None,
                has_original_dimensions: None,
            }],
            alt_text: None,
            caption: None,
            attribution: None,
        }
    }

    /// Creates an audio content block from a URL (for external providers)
    pub fn audio(url: impl Into<String>) -> Self {
        ContentBlock::Audio {
            media: None,
            url: Some(url.into()),
            provider: None,
            artist: None,
            album: None,
            title: None,
            embed_html: None,
            embed_url: None,
            poster: None,
            attribution: None,
            metadata: None,
        }
    }

    /// Creates a video content block from a URL (for external providers)
    pub fn video(url: impl Into<String>) -> Self {
        ContentBlock::Video {
            media: None,
            url: Some(url.into()),
            provider: None,
            embed_html: None,
            embed_iframe: None,
            embed_url: None,
            poster: None,
            attribution: None,
            can_autoplay_on_cellular: None,
            duration: None,
            metadata: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_block_creation() {
        let block = ContentBlock::text("Hello, world!");
        match block {
            ContentBlock::Text { text, .. } => {
                assert_eq!(text, "Hello, world!");
            }
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_heading_block_creation() {
        let block = ContentBlock::heading("Title", 1);
        match block {
            ContentBlock::Text { text, subtype, .. } => {
                assert_eq!(text, "Title");
                assert_eq!(subtype, Some("heading1".to_string()));
            }
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_link_block_creation() {
        let block = ContentBlock::link("https://example.com");
        match block {
            ContentBlock::Link { url, .. } => {
                assert_eq!(url, "https://example.com");
            }
            _ => panic!("Expected link block"),
        }
    }

    #[test]
    fn test_image_block_creation() {
        let block = ContentBlock::image("https://example.com/image.jpg");
        match block {
            ContentBlock::Image { media, .. } => {
                assert_eq!(media.len(), 1);
                assert_eq!(media[0].url, "https://example.com/image.jpg");
            }
            _ => panic!("Expected image block"),
        }
    }

    #[test]
    fn test_audio_block_creation() {
        let block = ContentBlock::audio("https://spotify.com/track/123");
        match block {
            ContentBlock::Audio { url, media, .. } => {
                assert_eq!(url, Some("https://spotify.com/track/123".to_string()));
                assert!(media.is_none());
            }
            _ => panic!("Expected audio block"),
        }
    }

    #[test]
    fn test_video_block_creation() {
        let block = ContentBlock::video("https://youtube.com/watch?v=abc");
        match block {
            ContentBlock::Video { url, media, .. } => {
                assert_eq!(url, Some("https://youtube.com/watch?v=abc".to_string()));
                assert!(media.is_none());
            }
            _ => panic!("Expected video block"),
        }
    }

    #[test]
    fn test_attribution_serialization() {
        let attr = Attribution::Link {
            url: "https://source.com".to_string(),
        };
        let json = serde_json::to_string(&attr).unwrap();
        assert!(json.contains("\"type\":\"link\""));
        assert!(json.contains("\"url\":\"https://source.com\""));
    }

    #[test]
    fn test_paywall_subtype() {
        let subtype = PaywallSubtype::Cta;
        let json = serde_json::to_string(&subtype).unwrap();
        assert_eq!(json, "\"cta\"");
    }

    #[test]
    fn test_display_mode() {
        let mode = DisplayMode {
            mode_type: "weighted".to_string(),
        };
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("\"type\":\"weighted\""));
    }
}
