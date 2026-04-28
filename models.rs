//! Data models for the Tumblr API

use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifier for a Tumblr blog
///
/// Tumblr blogs can be identified by name, hostname, or UUID.
/// All three formats are interchangeable in the API.
///
/// # Examples
///
/// ```
/// use crabrave::models::BlogIdentifier;
///
/// let by_name = BlogIdentifier::from("staff");
/// let by_hostname = BlogIdentifier::Hostname("staff.tumblr.com".to_string());
/// let by_uuid = BlogIdentifier::Uuid("t:123456789".to_string());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlogIdentifier {
    /// Blog name (e.g., "staff")
    Name(String),
    /// Blog hostname (e.g., "staff.tumblr.com")
    Hostname(String),
    /// Blog UUID (e.g., "t:123456789")
    Uuid(String),
}

impl BlogIdentifier {
    /// Returns the identifier as a string suitable for use in API requests
    pub fn as_str(&self) -> &str {
        match self {
            BlogIdentifier::Name(s) | BlogIdentifier::Hostname(s) | BlogIdentifier::Uuid(s) => s,
        }
    }
}

impl fmt::Display for BlogIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<String> for BlogIdentifier {
    fn from(s: String) -> Self {
        if s.starts_with("t:") {
            BlogIdentifier::Uuid(s)
        } else if s.contains('.') {
            BlogIdentifier::Hostname(s)
        } else {
            BlogIdentifier::Name(s)
        }
    }
}

impl From<&str> for BlogIdentifier {
    fn from(s: &str) -> Self {
        BlogIdentifier::from(s.to_string())
    }
}

/// Information about a Tumblr blog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blog {
    /// Blog name (short name used in URLs)
    pub name: String,
    /// Blog title
    pub title: String,
    /// Blog description
    #[serde(default)]
    pub description: String,
    /// Blog URL
    pub url: String,
    /// Blog UUID
    pub uuid: String,
    /// Last update timestamp (Unix time)
    pub updated: i64,
    /// Total number of posts (from `posts` field)
    #[serde(default)]
    pub posts: u64,
    /// Total number of posts (from `total_posts` field, sometimes present)
    #[serde(default)]
    pub total_posts: u64,
    /// Whether the blog is NSFW
    #[serde(default)]
    pub is_nsfw: bool,
    /// Whether the blog can be followed
    #[serde(default)]
    pub can_be_followed: bool,
    /// Whether the user is following this blog
    #[serde(default)]
    pub followed: bool,

    // === Ask settings ===
    /// Whether the blog accepts asks
    #[serde(default)]
    pub ask: bool,
    /// Whether the blog accepts anonymous asks
    #[serde(default)]
    pub ask_anon: bool,
    /// Custom title for the ask page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_page_title: Option<String>,
    /// Whether asks allow media attachments
    #[serde(default)]
    pub asks_allow_media: bool,

    // === Interaction settings ===
    /// Whether the blog allows chat
    #[serde(default)]
    pub can_chat: bool,
    /// Whether the user can send fan mail to this blog
    #[serde(default)]
    pub can_send_fan_mail: bool,
    /// Whether the blog can be subscribed to
    #[serde(default)]
    pub can_subscribe: bool,
    /// Whether the user is subscribed to this blog
    #[serde(default)]
    pub subscribed: bool,
    /// Whether the blog shares its likes publicly
    #[serde(default)]
    pub share_likes: bool,
    /// Whether the blog shares who it follows publicly
    #[serde(default)]
    pub share_following: bool,

    // === Avatar and theme ===
    /// Blog avatar in various sizes
    #[serde(default)]
    pub avatar: Vec<AvatarImage>,
    /// Blog theme configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Theme ID
    #[serde(default)]
    pub theme_id: u64,

    // === Premium features ===
    /// Tumblrmart accessories (badges, etc.)
    #[serde(default, deserialize_with = "crate::empty_object_as_none")]
    pub tumblrmart_accessories: Option<TumblrmartAccessories>,
    /// Whether the blog can show badges
    #[serde(default)]
    pub can_show_badges: bool,

    // === Other flags ===
    /// Whether the blog is blocked from primary
    #[serde(default)]
    pub is_blocked_from_primary: bool,
}

/// Avatar image at a specific size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarImage {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// URL to the avatar image
    pub url: String,
    /// Avatar accessories (hats, frames, etc.)
    #[serde(default)]
    pub accessories: Vec<serde_json::Value>,
}

/// Blog theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Avatar shape ("circle" or "square")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_shape: Option<String>,
    /// Background color (hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    /// Body font name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_font: Option<String>,
    /// Header bounds for cropping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_bounds: Option<serde_json::Value>,
    /// Header image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image: Option<String>,
    /// Focused header image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image_focused: Option<String>,
    /// Header image poster URL (for video headers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image_poster: Option<String>,
    /// Scaled header image URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image_scaled: Option<String>,
    /// Whether the header stretches to fill width
    #[serde(default)]
    pub header_stretch: bool,
    /// Link color (hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_color: Option<String>,
    /// Whether to show the avatar
    #[serde(default)]
    pub show_avatar: bool,
    /// Whether to show the description
    #[serde(default)]
    pub show_description: bool,
    /// Whether to show the header image
    #[serde(default)]
    pub show_header_image: bool,
    /// Whether to show the title
    #[serde(default)]
    pub show_title: bool,
    /// Title color (hex)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_color: Option<String>,
    /// Title font name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_font: Option<String>,
    /// Title font weight ("regular", "bold")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_font_weight: Option<String>,
    /// Full header width in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_full_width: Option<u32>,
    /// Full header height in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_full_height: Option<u32>,
    /// Focus width for header cropping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_focus_width: Option<u32>,
    /// Focus height for header cropping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_focus_height: Option<u32>,
}

/// Information about a Tumblr user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Username
    pub name: String,
    /// Number of posts the user has liked
    #[serde(default)]
    pub likes: u64,
    /// Number of blogs the user is following
    #[serde(default)]
    pub following: u64,
    /// Blogs owned by this user
    #[serde(default)]
    pub blogs: Vec<Blog>,
}

/// Pagination wrapper for API responses
///
/// Many Tumblr API endpoints return paginated results. This struct
/// contains the items for the current page and optional links for navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    /// Items in the current page
    #[serde(flatten)]
    pub items: Vec<T>,
    /// Total number of items (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Link to the next page (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl<T> Page<T> {
    /// Creates a new page with the given items
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            total: None,
            next: None,
        }
    }

    /// Creates a new page with items and total count
    pub fn with_total(items: Vec<T>, total: u64) -> Self {
        Self {
            items,
            total: Some(total),
            next: None,
        }
    }

    /// Checks if there are more pages available
    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }

    /// Returns the number of items in the current page
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Checks if the page is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total: None,
            next: None,
        }
    }
}

/// Tumblrmart accessories (badges, checkmarks, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TumblrmartAccessories {
    /// List of badges the blog has
    #[serde(default)]
    pub badges: Vec<Badge>,
    /// Number of blue checkmarks purchased
    #[serde(default)]
    pub blue_checkmark_count: u8,
}

/// A Tumblrmart badge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    /// Badge product group identifier (e.g., "blue-checkmark")
    pub product_group: String,
    /// Badge image URLs at various sizes
    #[serde(default)]
    pub urls: Vec<String>,
    /// URL to purchase more of this badge
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_url: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_blog_identifier_from_name() {
        let id = BlogIdentifier::from("staff");
        assert_eq!(id, BlogIdentifier::Name("staff".to_string()));
        assert_eq!(id.as_str(), "staff");
    }

    #[test]
    fn test_blog_identifier_from_hostname() {
        let id = BlogIdentifier::from("staff.tumblr.com");
        assert_eq!(id, BlogIdentifier::Hostname("staff.tumblr.com".to_string()));
        assert_eq!(id.as_str(), "staff.tumblr.com");
    }

    #[test]
    fn test_blog_identifier_from_uuid() {
        let id = BlogIdentifier::from("t:123456789");
        assert_eq!(id, BlogIdentifier::Uuid("t:123456789".to_string()));
        assert_eq!(id.as_str(), "t:123456789");
    }

    #[test]
    fn test_blog_identifier_display() {
        let id = BlogIdentifier::Name("staff".to_string());
        assert_eq!(format!("{}", id), "staff");
    }

    #[test]
    fn test_page_creation() {
        let page = Page::new(vec![1, 2, 3]);
        assert_eq!(page.len(), 3);
        assert!(!page.has_next());
    }

    #[test]
    fn test_page_with_total() {
        let page = Page::with_total(vec![1, 2, 3], 100);
        assert_eq!(page.total, Some(100));
    }

    #[test]
    fn test_page_has_next() {
        let mut page = Page::new(vec![1, 2, 3]);
        assert!(!page.has_next());

        page.next = Some("next_url".to_string());
        assert!(page.has_next());
    }

    #[test]
    fn test_page_is_empty() {
        let empty_page: Page<i32> = Page::default();
        assert!(empty_page.is_empty());

        let page = Page::new(vec![1]);
        assert!(!page.is_empty());
    }
}
