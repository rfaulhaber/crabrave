# Testing Guide

This document describes the testing strategy for crabrave and how to run different types of tests.

## Test Overview

Crabrave has three types of tests:

1. **Unit Tests** (47 tests) - Test individual components and builders
2. **Mock Server Tests** (11 tests) - Test full request/response cycles with wiremock
3. **Integration Tests** (16 tests) - Test against the real Tumblr API (optional, ignored by default)
4. **Doc Tests** (45 tests) - Ensure documentation examples compile

**Total: 119 tests**

## Running Tests

### Run All Tests (Except Integration)

This runs unit tests, mock server tests, and doc tests:

```bash
cargo test
```

Expected output:
- ✅ 47 unit tests passed
- ✅ 11 mock server tests passed
- ✅ 45 doc tests passed
- ⏭️ 16 integration tests ignored

### Run Only Mock Server Tests

These tests use wiremock to simulate the Tumblr API:

```bash
cargo test --test mock_server
```

Mock server tests verify:
- Full HTTP request/response cycles
- JSON serialization/deserialization
- Error handling (404, 429, 5xx)
- Rate limit detection
- All major API endpoints

### Run Integration Tests

Integration tests make real API calls to Tumblr. They require credentials and are ignored by default.

**Authentication Priority:**
1. Environment variables (TUMBLR_CONSUMER_KEY, TUMBLR_CONSUMER_SECRET, TUMBLR_ACCESS_TOKEN)
2. Token file at ~/.tumblr_tokens.json (created by oauth-helper)
3. Fail with helpful error message

#### Method 1: Using Environment Variables (Recommended for CI/CD)

The simplest way to run integration tests in CI/CD environments:

```bash
# Set required environment variables
export TUMBLR_CONSUMER_KEY="your_consumer_key"
export TUMBLR_CONSUMER_SECRET="your_consumer_secret"
export TUMBLR_ACCESS_TOKEN="your_access_token"

# Optional: For automatic token refresh
export TUMBLR_REFRESH_TOKEN="your_refresh_token"

# Run integration tests
cargo test --test integration -- --ignored
```

Environment variables take priority over the token file, making it easy to use different credentials for testing.

#### Method 2: Using OAuth2 Helper (Recommended for Local Development)

The easiest way to set up integration tests is to use the OAuth2 helper tool:

```bash
# Step 1: Run the interactive OAuth2 helper
cargo run -p oauth-helper

# Follow the prompts to:
# 1. Enter your consumer key/secret (from https://www.tumblr.com/oauth/apps)
# 2. Authorize the app in your browser
# 3. Paste the authorization code
# 4. Tokens are automatically saved to ~/.tumblr_tokens.json

# Step 2: Run integration tests (will use saved tokens automatically)
cargo test --test integration -- --ignored
```

**What the helper does:**
- Guides you through the complete OAuth2 flow
- Opens your browser for authorization
- Exchanges the authorization code for tokens
- Saves tokens (including refresh token) to `~/.tumblr_tokens.json`
- Integration tests automatically load and refresh tokens as needed

**No manual token management required!** The integration tests will:
1. Check environment variables first (if set)
2. Load tokens from `~/.tumblr_tokens.json` (if file exists)
3. Use the refresh token to get a fresh access token automatically
4. Work indefinitely without re-authorization (as long as the refresh token is valid)

#### Method 3: Manual OAuth2 Flow (Advanced)

If you can't use the helper tool, you can manually obtain tokens:

**Step 1:** Get consumer credentials
```bash
# These are the only env vars needed if not using the helper
export TUMBLR_CONSUMER_KEY="your_consumer_key"
export TUMBLR_CONSUMER_SECRET="your_consumer_secret"
```

**Step 2:** Run the OAuth2 helper (even if you have credentials)
```bash
cargo run -p oauth-helper
# This will guide you through authorization and save tokens
```

**Note:** You cannot run integration tests without going through the OAuth2 flow. Access tokens must be obtained via the helper tool - they cannot be provided manually via environment variables.

**Getting Credentials:**
1. Register an app at https://www.tumblr.com/oauth/apps
2. Get your consumer key and secret
3. Run `cargo run -p oauth-helper` (recommended)
   - OR manually complete OAuth flow to get access token

**OAuth2 Flow Tests:**

For testing the OAuth2 flow specifically (optional):

```bash
# Optional: For testing code exchange (requires a fresh authorization code)
export TUMBLR_OAUTH2_AUTH_CODE="your_authorization_code"

# Optional: Custom redirect URI (defaults to http://localhost:8080/callback)
export TUMBLR_OAUTH2_REDIRECT_URI="http://localhost:8080/callback"
```

**Note:** Authorization codes are single-use and expire quickly.

#### 2. Run Integration Tests

```bash
# Run all integration tests
cargo test --test integration -- --ignored

# Run a specific integration test
cargo test --test integration integration_blog_info -- --ignored
```

#### 3. What Gets Tested

Integration tests verify:
- ✅ Blog info retrieval
- ✅ Blog avatars
- ✅ Blog posts with filters
- ✅ User info
- ✅ User dashboard
- ✅ Tagged post search
- ✅ Fetching specific posts
- ✅ Rate limit handling
- ✅ Error handling (404s)
- ✅ User likes
- ✅ User following list
- ✅ OAuth2 authorization URL generation
- ✅ OAuth2 callback parameter parsing
- ✅ OAuth2 code exchange (optional, requires TUMBLR_OAUTH2_AUTH_CODE)
- ✅ OAuth2 token refresh (optional, requires TUMBLR_OAUTH2_REFRESH_TOKEN)
- ✅ OAuth2 full flow client creation

**Note:** Integration tests only read data, they don't create, modify, or delete anything on your Tumblr account.

### Run Tests with Output

To see `println!` output from tests:

```bash
cargo test -- --nocapture
```

## Test Categories

### Unit Tests (`lib.rs`, `handlers/*.rs`, `npf.rs`)

Test individual components:
- Builder construction
- Type conversions
- Helper methods
- NPF content block creation

**Location:** Inline in source files with `#[cfg(test)]`

### Mock Server Tests (`tests/mock_server.rs`)

Test API client behavior without real API:
- HTTP request construction
- Response parsing
- Error handling
- Rate limiting
- All major endpoints

**Technology:** wiremock

### Integration Tests (`tests/integration.rs`)

Test against real Tumblr API:
- End-to-end functionality
- Real API compatibility
- Actual network behavior

**Requirements:** Valid Tumblr API credentials

## Continuous Integration

For CI pipelines, run:

```bash
# Run all tests except integration
cargo test

# Also run clippy
cargo clippy -- -D warnings
```

Integration tests should be run separately with credentials stored as secrets.

## Test Coverage

| Module | Unit Tests | Mock Tests | Integration Tests |
|--------|------------|------------|-------------------|
| Blogs API | ✅ | ✅ | ✅ |
| Users API | ✅ | ✅ | ✅ |
| Posts API | ✅ | ✅ | ✅ |
| Tagged API | ✅ | ✅ | ✅ |
| Communities API | ✅ | ✅ | Partial |
| NPF | ✅ | ✅ | - |
| Error Handling | ✅ | ✅ | ✅ |
| Rate Limiting | - | ✅ | ✅ |

## Writing New Tests

### Adding a Mock Server Test

```rust
#[tokio::test]
async fn test_my_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/my/endpoint"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "meta": {"status": 200, "msg": "OK"},
            "response": { /* your response */ }
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let result = client.my_method().await;

    assert!(result.is_ok());
}
```

### Adding an Integration Test

```rust
#[tokio::test]
#[ignore]  // Important!
async fn integration_my_test() {
    let client = test_client().expect("Failed to create client");

    let result = client.my_method().await;

    match result {
        Ok(data) => {
            // Verify result
            assert!(!data.is_empty());
        }
        Err(e) => panic!("Test failed: {}", e),
    }
}
```

## Troubleshooting

### Mock Server Tests Failing

- Check that response JSON matches Tumblr's API format
- Verify the endpoint path and query parameters
- Ensure proper `meta` and `response` envelope structure

### Integration Tests Failing

- Verify environment variables are set correctly
- Check your OAuth credentials are valid
- Ensure your test blog exists
- Some endpoints require specific permissions

### Rate Limiting

If integration tests hit rate limits:
- Wait before retrying (check `retry-after` header)
- Reduce the number of tests run in quick succession
- Use mock server tests for rapid development

## Performance

- **Unit tests:** ~0.00s (instant)
- **Mock server tests:** ~0.07s (fast)
- **Doc tests:** ~0.00s (instant)
- **Integration tests:** Variable (depends on network and API response time)

Mock server tests provide fast feedback while maintaining high confidence in the HTTP client behavior.
