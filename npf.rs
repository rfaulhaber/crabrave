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
        /// Media objects for this audio
        media: Vec<MediaObject>,
        /// Optional artist name
        #[serde(skip_serializing_if = "Option::is_none")]
        artist: Option<String>,
        /// Optional album name
        #[serde(skip_serializing_if = "Option::is_none")]
        album: Option<String>,
        /// Optional track title
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Video content block
    Video {
        /// Media objects for this video
        media: Vec<MediaObject>,
        /// Optional poster/thumbnail images
        #[serde(skip_serializing_if = "Option::is_none")]
        poster: Option<Vec<MediaObject>>,
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
    /// Media URL
    pub url: String,
    /// Media type (image/jpg, video/mp4, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    /// Width in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Height in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
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
    },
}

/// Display block within a layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayBlock {
    /// Index of the content block to display
    pub blocks: Vec<usize>,
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
                width: None,
                height: None,
            }],
            alt_text: None,
            caption: None,
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
}
