//! Media source types for uploading images, videos, and other media to Tumblr
//!
//! This module provides types for specifying media to upload when creating or editing posts.

use camino::{Utf8Path, Utf8PathBuf};

/// A source of media data to upload to Tumblr
///
/// MediaSource can be created from file paths or byte data, with automatic
/// MIME type detection from file extensions.
///
/// # Examples
///
/// ```no_run
/// # use crabrave::media::MediaSource;
/// // From file path (auto-detects filename and MIME type)
/// let source = MediaSource::from_path("/path/to/image.jpg");
///
/// // From bytes with explicit filename
/// let bytes = vec![0u8; 1024];
/// let source = MediaSource::from_bytes("photo.jpg", bytes);
///
/// // Override MIME type
/// let source = MediaSource::from_bytes("image.data", vec![0u8; 1024])
///     .with_mime_type("image/jpeg");
/// ```
#[derive(Debug, Clone)]
pub struct MediaSource {
    filename: String,
    mime_type: Option<String>,
    data: MediaData,
}

/// Internal representation of media data
#[derive(Debug, Clone)]
enum MediaData {
    /// Media loaded from bytes in memory
    Bytes(Vec<u8>),
    /// Media to be loaded from a file path
    Path(Utf8PathBuf),
}

impl MediaSource {
    /// Creates a MediaSource from a file path
    ///
    /// Automatically extracts the filename and detects MIME type from the file extension.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the media file
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::media::MediaSource;
    /// let source = MediaSource::from_path("/path/to/video.mp4");
    /// ```
    pub fn from_path(path: impl Into<Utf8PathBuf>) -> Self {
        let path = path.into();
        let filename = path
            .file_name()
            .unwrap_or("file")
            .to_string();
        let mime_type = detect_mime_type_from_filename(&filename);

        Self {
            filename,
            mime_type,
            data: MediaData::Path(path),
        }
    }

    /// Creates a MediaSource from bytes
    ///
    /// Automatically detects MIME type from the filename extension.
    ///
    /// # Arguments
    ///
    /// * `filename` - Name for the file (used for MIME type detection)
    /// * `data` - Binary data to upload
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::media::MediaSource;
    /// let bytes = std::fs::read("/path/to/image.jpg").unwrap();
    /// let source = MediaSource::from_bytes("image.jpg", bytes);
    /// ```
    pub fn from_bytes(filename: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let filename = filename.into();
        let mime_type = detect_mime_type_from_filename(&filename);

        Self {
            filename,
            mime_type,
            data: MediaData::Bytes(data.into()),
        }
    }

    /// Overrides the MIME type
    ///
    /// By default, MIME type is auto-detected from the filename extension.
    /// Use this method to specify a different MIME type.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crabrave::media::MediaSource;
    /// let source = MediaSource::from_bytes("data.bin", vec![0u8; 100])
    ///     .with_mime_type("image/png");
    /// ```
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Gets the filename
    pub(crate) fn filename(&self) -> &str {
        &self.filename
    }

    /// Gets the MIME type, if set
    pub(crate) fn mime_type(&self) -> Option<&str> {
        self.mime_type.as_deref()
    }

    /// Reads the media data into bytes
    ///
    /// For path-based sources, this reads the file from disk.
    /// For byte-based sources, this returns a clone of the bytes.
    pub(crate) fn read_bytes(&self) -> std::io::Result<Vec<u8>> {
        match &self.data {
            MediaData::Bytes(bytes) => Ok(bytes.clone()),
            MediaData::Path(path) => fs_err::read(path),
        }
    }
}

/// Detects MIME type from a filename extension
///
/// Returns `None` if the extension is not recognized.
fn detect_mime_type_from_filename(filename: &str) -> Option<String> {
    let extension = Utf8Path::new(filename)
        .extension()
        .map(|e| e.to_lowercase())?;

    let mime_type = match extension.as_str() {
        // Images
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",

        // Videos
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mkv" => "video/x-matroska",
        "m4v" => "video/x-m4v",

        // Audio
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "wma" => "audio/x-ms-wma",

        _ => return None,
    };

    Some(mime_type.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let data = vec![1, 2, 3, 4];
        let source = MediaSource::from_bytes("image.jpg", data.clone());

        assert_eq!(source.filename(), "image.jpg");
        assert_eq!(source.mime_type(), Some("image/jpeg"));
        assert_eq!(source.read_bytes().unwrap(), data);
    }

    #[test]
    fn test_with_mime_type() {
        let source = MediaSource::from_bytes("file.bin", vec![1, 2, 3])
            .with_mime_type("application/octet-stream");

        assert_eq!(source.mime_type(), Some("application/octet-stream"));
    }

    #[test]
    fn test_detect_mime_type_images() {
        assert_eq!(
            detect_mime_type_from_filename("photo.jpg"),
            Some("image/jpeg".to_string())
        );
        assert_eq!(
            detect_mime_type_from_filename("image.PNG"),
            Some("image/png".to_string())
        );
        assert_eq!(
            detect_mime_type_from_filename("animation.gif"),
            Some("image/gif".to_string())
        );
    }

    #[test]
    fn test_detect_mime_type_videos() {
        assert_eq!(
            detect_mime_type_from_filename("video.mp4"),
            Some("video/mp4".to_string())
        );
        assert_eq!(
            detect_mime_type_from_filename("clip.WEBM"),
            Some("video/webm".to_string())
        );
    }

    #[test]
    fn test_detect_mime_type_audio() {
        assert_eq!(
            detect_mime_type_from_filename("song.mp3"),
            Some("audio/mpeg".to_string())
        );
        assert_eq!(
            detect_mime_type_from_filename("track.flac"),
            Some("audio/flac".to_string())
        );
    }

    #[test]
    fn test_detect_mime_type_unknown() {
        assert_eq!(detect_mime_type_from_filename("file.xyz"), None);
        assert_eq!(detect_mime_type_from_filename("noextension"), None);
    }
}
