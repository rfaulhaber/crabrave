//! Neue Post Format (NPF) types
//!
//! NPF is Tumblr's modern content representation system that uses structured content blocks
//! instead of legacy post types. It provides a flexible way to create rich, mixed-media posts.
//!
//! See [this](https://www.tumblr.com/docs/npf) for Tumblr's NPF specification.

use serde::{Deserialize, Deserializer, Serialize};

/// A content block in the NPF format
///
/// Content blocks are the building pieces of NPF posts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
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
        #[serde(
            skip_serializing_if = "Option::is_none",
            default,
            deserialize_with = "deserialize_media_single_or_vec"
        )]
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
    /// Poll content block
    Poll {
        /// Unique client-side identifier for this poll
        client_id: String,
        /// The poll question
        question: String,
        /// Available answers
        answers: Vec<PollAnswer>,
        /// Poll settings (voting rules, expiration, etc.)
        #[serde(skip_serializing_if = "Option::is_none")]
        settings: Option<PollSettings>,
        /// When the poll was created (human-readable)
        #[serde(skip_serializing_if = "Option::is_none")]
        created_at: Option<String>,
        /// When the poll was created (Unix timestamp)
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<i64>,
    },
    /// Catch-all for unrecognized content block types.
    /// Prevents deserialization failures when the API introduces new types.
    #[serde(other)]
    Unknown,
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
    /// Unique media key identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_key: Option<String>,
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
    /// Extracted colors from the image (c0, c1, c2, c3, c4 as hex strings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<MediaColors>,
    /// EXIF metadata from the original image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exif: Option<MediaExif>,
}

/// Extracted color palette from an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaColors {
    /// Primary color (hex without #)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c0: Option<String>,
    /// Secondary color (hex without #)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c1: Option<String>,
    /// Tertiary color (hex without #)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c2: Option<String>,
    /// Quaternary color (hex without #)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c3: Option<String>,
    /// Quinary color (hex without #)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c4: Option<String>,
}

/// EXIF metadata from an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaExif {
    /// Timestamp when the photo was taken (Unix timestamp as string)
    #[serde(rename = "Time", skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Camera make
    #[serde(rename = "Make", skip_serializing_if = "Option::is_none")]
    pub make: Option<String>,
    /// Camera model
    #[serde(rename = "Model", skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Focal length
    #[serde(rename = "FocalLength", skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<String>,
    /// Aperture value
    #[serde(rename = "Aperture", skip_serializing_if = "Option::is_none")]
    pub aperture: Option<String>,
    /// Exposure time
    #[serde(rename = "Exposure", skip_serializing_if = "Option::is_none")]
    pub exposure: Option<String>,
    /// ISO speed
    #[serde(rename = "ISO", skip_serializing_if = "Option::is_none")]
    pub iso: Option<String>,
}

/// Attribution for content sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
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
        /// App logo (boxed to reduce enum size)
        #[serde(skip_serializing_if = "Option::is_none")]
        logo: Option<Box<MediaObject>>,
    },
    /// Catch-all for unrecognized attribution types.
    /// Prevents deserialization failures when the API introduces new types.
    #[serde(other)]
    Unknown,
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

/// A single answer option in a poll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollAnswer {
    /// Unique client-side identifier for this answer
    pub client_id: String,
    /// Display text for this answer
    pub answer_text: String,
}

/// Settings controlling poll behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollSettings {
    /// Whether voters can select multiple answers
    #[serde(default)]
    pub multiple_choice: bool,
    /// Close status (e.g., "closed-after", "open")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_status: Option<String>,
    /// Seconds after creation when the poll expires
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_after: Option<u64>,
    /// Source platform (e.g., "tumblr")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Layout information for NPF posts
///
/// Layouts control how content blocks are arranged visually.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
#[non_exhaustive]
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
    /// Catch-all for unrecognized layout types.
    /// Prevents deserialization failures when the API introduces new types.
    #[serde(other)]
    Unknown,
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
                media_key: None,
                identifier: None,
                width: None,
                height: None,
                original_dimensions_missing: None,
                cropped: None,
                has_original_dimensions: None,
                colors: None,
                exif: None,
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

/// Deserializes a media field that can be either a single object or an array.
///
/// The Tumblr API sometimes returns `media` as a single `MediaObject` and sometimes
/// as an array of `MediaObject`s. This deserializer handles both cases.
fn deserialize_media_single_or_vec<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<MediaObject>>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MediaOneOrMany {
        One(Box<MediaObject>),
        Many(Vec<MediaObject>),
    }

    let value: Option<MediaOneOrMany> = Option::deserialize(deserializer)?;
    Ok(value.map(|v| match v {
        MediaOneOrMany::One(obj) => vec![*obj],
        MediaOneOrMany::Many(vec) => vec,
    }))
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

    #[test]
    fn test_poll_block_deserialization() {
        let json = serde_json::json!({
            "type": "poll",
            "client_id": "abc-123",
            "question": "Favorite color?",
            "answers": [
                { "client_id": "a1", "answer_text": "Red" },
                { "client_id": "a2", "answer_text": "Blue" }
            ],
            "settings": {
                "multiple_choice": true,
                "close_status": "open",
                "expire_after": 86400,
                "source": "tumblr"
            },
            "created_at": "2026-01-01 00:00:00 GMT",
            "timestamp": 1767225600
        });

        let block: ContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ContentBlock::Poll {
                client_id,
                question,
                answers,
                settings,
                created_at,
                timestamp,
            } => {
                assert_eq!(client_id, "abc-123");
                assert_eq!(question, "Favorite color?");
                assert_eq!(answers.len(), 2);
                assert_eq!(answers[0].answer_text, "Red");
                assert_eq!(answers[1].answer_text, "Blue");
                let settings = settings.unwrap();
                assert!(settings.multiple_choice);
                assert_eq!(settings.close_status.as_deref(), Some("open"));
                assert_eq!(settings.expire_after, Some(86400));
                assert_eq!(created_at.as_deref(), Some("2026-01-01 00:00:00 GMT"));
                assert_eq!(timestamp, Some(1767225600));
            }
            _ => panic!("Expected Poll block"),
        }
    }

    #[test]
    fn test_poll_block_minimal() {
        let json = serde_json::json!({
            "type": "poll",
            "client_id": "abc",
            "question": "Yes or no?",
            "answers": [
                { "client_id": "a1", "answer_text": "Yes" }
            ]
        });

        let block: ContentBlock = serde_json::from_value(json).unwrap();
        match block {
            ContentBlock::Poll {
                settings,
                created_at,
                timestamp,
                ..
            } => {
                assert!(settings.is_none());
                assert!(created_at.is_none());
                assert!(timestamp.is_none());
            }
            _ => panic!("Expected Poll block"),
        }
    }

    #[test]
    fn test_unknown_content_block_type() {
        let json = serde_json::json!({
            "type": "hologram",
            "data": "something new"
        });

        let block: ContentBlock = serde_json::from_value(json).unwrap();
        assert!(matches!(block, ContentBlock::Unknown));
    }

    #[test]
    fn test_unknown_attribution_type() {
        let json = serde_json::json!({
            "type": "ai_generated",
            "model": "test"
        });

        let attr: Attribution = serde_json::from_value(json).unwrap();
        assert!(matches!(attr, Attribution::Unknown));
    }

    #[test]
    fn test_unknown_layout_type() {
        let json = serde_json::json!({
            "type": "carousel",
            "speed": 5
        });

        let layout: LayoutBlock = serde_json::from_value(json).unwrap();
        assert!(matches!(layout, LayoutBlock::Unknown));
    }

    #[test]
    fn test_content_block_array_with_unknown_types() {
        let json = serde_json::json!([
            { "type": "text", "text": "hello" },
            { "type": "hologram", "data": "future" },
            { "type": "text", "text": "world" }
        ]);

        let blocks: Vec<ContentBlock> = serde_json::from_value(json).unwrap();
        assert_eq!(blocks.len(), 3);
        assert!(matches!(&blocks[0], ContentBlock::Text { text, .. } if text == "hello"));
        assert!(matches!(blocks[1], ContentBlock::Unknown));
        assert!(matches!(&blocks[2], ContentBlock::Text { text, .. } if text == "world"));
    }
}
