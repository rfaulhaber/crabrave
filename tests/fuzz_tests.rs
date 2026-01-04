//! Property-based fuzzing tests for crabrave
//!
//! These tests use proptest to generate random inputs and verify that
//! the library handles them gracefully without panicking.

use proptest::prelude::*;

// ============================================================================
// BlogIdentifier fuzzing
// ============================================================================

proptest! {
    /// Test that BlogIdentifier can handle any string input without panicking
    #[test]
    fn fuzz_blog_identifier_from_string(s in ".*") {
        use crabrave::BlogIdentifier;

        // Should never panic, just create a valid identifier
        let identifier: BlogIdentifier = s.as_str().into();
        let _ = identifier.as_str();
        let _ = identifier.to_string();
    }

    /// Test BlogIdentifier with various patterns that might be edge cases
    #[test]
    fn fuzz_blog_identifier_edge_cases(
        s in prop::string::string_regex(
            r"(t:[a-zA-Z0-9_-]+|[a-zA-Z0-9_-]+\.tumblr\.com|[a-zA-Z0-9_-]+)"
        ).unwrap()
    ) {
        use crabrave::BlogIdentifier;

        let identifier: BlogIdentifier = s.as_str().into();
        let str_repr = identifier.as_str();

        // The string representation should be non-empty
        prop_assert!(!str_repr.is_empty());
    }
}

// ============================================================================
// NPF ContentBlock fuzzing
// ============================================================================

proptest! {
    /// Test that text content blocks handle any string content
    #[test]
    fn fuzz_npf_text_block(text in ".*") {
        use crabrave::npf::ContentBlock;

        let block = ContentBlock::text(&text);

        // Serialize to JSON should work
        let json = serde_json::to_string(&block);
        prop_assert!(json.is_ok());

        // Deserialize back should work
        if let Ok(json_str) = json {
            let parsed: Result<ContentBlock, _> = serde_json::from_str(&json_str);
            prop_assert!(parsed.is_ok());
        }
    }

    /// Test heading blocks with various levels and content
    #[test]
    fn fuzz_npf_heading_block(text in ".*", level in 1u8..=2u8) {
        use crabrave::npf::ContentBlock;

        let block = ContentBlock::heading(&text, level);

        let json = serde_json::to_string(&block);
        prop_assert!(json.is_ok());
    }

    /// Test link blocks with various URLs
    #[test]
    fn fuzz_npf_link_block(
        url in r"https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/[a-zA-Z0-9._~:/?#\[\]@!$&'()*+,;=-]*)?"
    ) {
        use crabrave::npf::ContentBlock;

        let block = ContentBlock::link(&url);

        let json = serde_json::to_string(&block);
        prop_assert!(json.is_ok());
    }

    /// Test audio blocks with external URLs
    #[test]
    fn fuzz_npf_audio_block(
        url in r"https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/[a-zA-Z0-9._/-]*\.(mp3|ogg|wav)"
    ) {
        use crabrave::npf::ContentBlock;

        let block = ContentBlock::audio(&url);

        let json = serde_json::to_string(&block);
        prop_assert!(json.is_ok());
    }
}

// ============================================================================
// JSON Response parsing fuzzing
// ============================================================================

proptest! {
    /// Test that malformed JSON doesn't cause panics in response parsing
    #[test]
    fn fuzz_json_response_parsing(json in ".*") {
        use crabrave::Blog;

        // Attempt to parse as a Blog - should either succeed or return error, never panic
        let result: Result<Blog, _> = serde_json::from_str(&json);
        let _ = result; // We don't care about the result, just that it doesn't panic
    }

    /// Test parsing with JSON-like structures
    #[test]
    fn fuzz_json_like_structures(
        name in "[a-zA-Z0-9_-]{1,32}",
        title in ".*",
        posts in 0u64..1000000u64,
        updated in 1000000000i64..2000000000i64,
    ) {
        use crabrave::Blog;

        let json = serde_json::json!({
            "name": name,
            "title": title,
            "posts": posts,
            "url": format!("https://{}.tumblr.com/", name),
            "uuid": format!("t:{}", name),
            "updated": updated,
        });

        let result: Result<Blog, _> = serde_json::from_value(json);
        prop_assert!(result.is_ok());
    }

    /// Test parsing API response envelopes
    #[test]
    fn fuzz_api_response_envelope(
        status in 100u16..600u16,
        msg in "[A-Za-z ]{1,50}",
    ) {
        let json = serde_json::json!({
            "meta": {
                "status": status,
                "msg": msg
            },
            "response": {}
        });

        // Should parse without panicking
        let json_str = json.to_string();
        let _ = serde_json::from_str::<serde_json::Value>(&json_str);
    }
}

// ============================================================================
// Media source fuzzing
// ============================================================================

proptest! {
    /// Test MediaSource creation with various filenames doesn't panic
    #[test]
    fn fuzz_media_source_filename(
        filename in "[a-zA-Z0-9_.-]{1,100}\\.(jpg|png|gif|mp4|webm|mp3|ogg)"
    ) {
        use crabrave::media::MediaSource;

        let data = vec![0u8; 100];
        // Should not panic when creating with various filenames
        let _source = MediaSource::from_bytes(&filename, data);
    }

    /// Test MediaSource with custom MIME types doesn't panic
    #[test]
    fn fuzz_media_source_mime_override(
        filename in "[a-zA-Z0-9_-]{1,50}",
        mime_type in "(image|video|audio)/[a-z0-9+-]+"
    ) {
        use crabrave::media::MediaSource;

        let data = vec![0u8; 10];
        // Should not panic when setting custom MIME types
        let _source = MediaSource::from_bytes(&filename, data).with_mime_type(&mime_type);
    }
}

// ============================================================================
// Query parameter fuzzing
// ============================================================================

proptest! {
    /// Test that query parameters with special characters are handled
    #[test]
    fn fuzz_query_param_encoding(
        key in "[a-zA-Z_][a-zA-Z0-9_]{0,20}",
        value in ".*"
    ) {
        use std::collections::HashMap;

        let mut params: HashMap<String, String> = HashMap::new();
        params.insert(key.clone(), value.clone());

        // URL encoding should not panic
        let encoded = serde_urlencoded::to_string(&params);
        prop_assert!(encoded.is_ok());
    }

    /// Test tag encoding (tags can have various characters)
    #[test]
    fn fuzz_tag_values(tag in "[a-zA-Z0-9 _#@!&*()-]{1,100}") {
        // URL encoding of tags should work
        let encoded = urlencoding::encode(&tag);
        prop_assert!(!encoded.is_empty());

        // Should be decodable
        let decoded = urlencoding::decode(&encoded);
        prop_assert!(decoded.is_ok());
    }
}

// ============================================================================
// Post content fuzzing
// ============================================================================

proptest! {
    /// Test that post tags are handled correctly
    #[test]
    fn fuzz_post_tags(
        tags in prop::collection::vec("[a-zA-Z0-9 ]{1,50}", 0..20)
    ) {
        use crabrave::npf::ContentBlock;

        // Create a post body with these tags
        let content = vec![ContentBlock::text("Test post")];

        // Serialize to JSON
        let body = serde_json::json!({
            "content": content,
            "tags": tags
        });

        let json = serde_json::to_string(&body);
        prop_assert!(json.is_ok());
    }

    /// Test reblog keys (should be alphanumeric)
    #[test]
    fn fuzz_reblog_key(key in "[a-zA-Z0-9]{8,16}") {
        // Reblog keys should be valid strings that can be serialized
        let body = serde_json::json!({
            "reblog_key": key
        });

        let json = serde_json::to_string(&body);
        prop_assert!(json.is_ok());
    }
}

// ============================================================================
// OAuth parameter fuzzing
// ============================================================================

proptest! {
    /// Test OAuth nonce generation patterns
    #[test]
    fn fuzz_oauth_nonce_like(nonce in "[a-zA-Z0-9]{16,32}") {
        // Nonces should be URL-safe
        let encoded = urlencoding::encode(&nonce);
        prop_assert_eq!(encoded.as_ref(), nonce.as_str());
    }

    /// Test OAuth signature base string components
    #[test]
    fn fuzz_oauth_signature_components(
        _method in "(GET|POST|PUT|DELETE)",
        path in "/[a-zA-Z0-9/_-]{1,100}",
    ) {
        // Building OAuth signature base strings should not panic
        let base_url = format!("https://api.tumblr.com/v2{}", path);

        // URL parsing should work
        let parsed = url::Url::parse(&base_url);
        prop_assert!(parsed.is_ok());
    }
}

// ============================================================================
// Timestamp fuzzing
// ============================================================================

proptest! {
    /// Test that various timestamp values are handled
    #[test]
    fn fuzz_timestamps(timestamp in 0i64..i64::MAX) {
        let body = serde_json::json!({
            "timestamp": timestamp
        });

        let json = serde_json::to_string(&body);
        prop_assert!(json.is_ok());
    }

    /// Test post IDs (large integers)
    #[test]
    fn fuzz_post_ids(id in 0u64..u64::MAX) {
        let id_string = id.to_string();

        // Post IDs should be serializable as strings
        let body = serde_json::json!({
            "id_string": id_string
        });

        let json = serde_json::to_string(&body);
        prop_assert!(json.is_ok());
    }
}

// ============================================================================
// Complex structure fuzzing
// ============================================================================

proptest! {
    /// Test parsing posts with various field combinations
    #[test]
    fn fuzz_post_structure(
        id in 1u64..u64::MAX,
        blog_name in "[a-zA-Z0-9_-]{3,32}",
        post_type in "(text|photo|quote|link|chat|audio|video|answer)",
        timestamp in 1000000000i64..2000000000i64,
        note_count in 0u64..1000000u64,
        tags in prop::collection::vec("[a-zA-Z0-9]{1,20}", 0..10),
    ) {
        use crabrave::handlers::blog::Post;

        let json = serde_json::json!({
            "id_string": id.to_string(),
            "blog_name": blog_name,
            "post_url": format!("https://{}.tumblr.com/post/{}", blog_name, id),
            "type": post_type,
            "timestamp": timestamp,
            "note_count": note_count,
            "tags": tags,
        });

        let result: Result<Post, _> = serde_json::from_value(json);
        prop_assert!(result.is_ok());
    }
}

// ============================================================================
// Edge case tests (not randomized but important boundaries)
// ============================================================================

#[test]
fn test_empty_strings() {
    use crabrave::BlogIdentifier;

    // Empty string should still create a valid identifier
    let id: BlogIdentifier = "".into();
    assert_eq!(id.as_str(), "");
}

#[test]
fn test_very_long_strings() {
    use crabrave::npf::ContentBlock;

    // Very long text should be handled
    let long_text = "a".repeat(100_000);
    let block = ContentBlock::text(&long_text);

    let json = serde_json::to_string(&block);
    assert!(json.is_ok());
}

#[test]
fn test_unicode_content() {
    use crabrave::npf::ContentBlock;

    // Unicode content should work
    let unicode_text = "Hello 世界 🦀 مرحبا Привет";
    let block = ContentBlock::text(unicode_text);

    let json = serde_json::to_string(&block).unwrap();
    let parsed: ContentBlock = serde_json::from_str(&json).unwrap();

    if let ContentBlock::Text { text, .. } = parsed {
        assert_eq!(text, unicode_text);
    } else {
        panic!("Expected Text block");
    }
}

#[test]
fn test_special_characters_in_tags() {
    // Tags with special characters
    let special_tags = vec![
        "c++",
        "c#",
        "node.js",
        "1984",
        "year:2024",
        "it's working",
    ];

    for tag in special_tags {
        let encoded = urlencoding::encode(tag);
        let decoded = urlencoding::decode(&encoded).unwrap();
        assert_eq!(decoded, tag);
    }
}

#[test]
fn test_null_bytes_in_content() {
    use crabrave::npf::ContentBlock;

    // Content with null bytes
    let text_with_null = "Hello\x00World";
    let block = ContentBlock::text(text_with_null);

    // Should serialize (JSON will escape the null byte)
    let json = serde_json::to_string(&block);
    assert!(json.is_ok());
}
