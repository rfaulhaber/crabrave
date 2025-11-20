# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`crabrave` is a Rust HTTP client library for the Tumblr API. The project implements OAuth2 authentication and provides a client interface for interacting with Tumblr's API endpoints.

**Design Philosophy:** This client should be modeled after [Octocrab](https://github.com/XAMPPRocky/octocrab) - designed to be very ergonomic to use within Rust. The API should feel natural and idiomatic, with a focus on developer experience similar to how Octocrab provides an elegant interface for GitHub's API.

## Development Environment

This project uses Nix flakes for reproducible development environments. To enter the development shell:

```bash
nix develop
```

The development shell includes:
- Rust stable toolchain
- clippy (linter)
- rust-analyzer (LSP)
- cargo-nextest (test runner)
- claude-code

## Build Commands

**Build the library:**
```bash
cargo build
```

**Build with release optimizations:**
```bash
cargo build --release
```

**Run linter:**
```bash
cargo clippy
```

**Format code:**
```bash
nix fmt
```

## Testing

**Run all tests:**
```bash
cargo test
```

**Run tests with nextest (faster, parallel):**
```bash
cargo nextest run
```

**Run a specific test:**
```bash
cargo test test_name
```

## Architecture

The project uses a modular structure with the main library in `lib.rs` and specialized modules:

### Core Components

**`Crabrave` Client** (`lib.rs`)
- Main entry point with builder pattern initialization
- Supports OAuth1, OAuth2, and API-key-only authentication
- Runtime-agnostic async implementation
- Automatic User-Agent header configuration
- Built-in rate limit detection

**Error Handling** (`error.rs`)
- `CrabError` enum with comprehensive error types
- Rate limit errors include retry-after information
- Proper error chains with thiserror

**Response Parsing** (`response.rs`)
- `ApiResponse<T>` envelope parser
- Smart status checking before deserialization
- Handles Tumblr's `{meta, response}` structure

**Models** (`models.rs`)
- `Blog`, `User`, `Page<T>` - Core data structures
- `BlogIdentifier` - Enum supporting name/hostname/UUID
- All models use serde for JSON serialization

### API Modules (handlers/)

**Naming Convention:** Uses simple plural names (not "Handler") for cleaner API

**Blogs API** (`handlers/blog.rs`)
```rust
crab.blogs("staff").info().await?
crab.blogs("staff").posts().limit(20).send().await?
crab.blogs("staff").avatar(Some(128)).await?
```

**Users API** (`handlers/user.rs`)
```rust
crab.users().info().await?
crab.users().dashboard().limit(20).send().await?
crab.users().likes().before(timestamp).send().await?
crab.users().follow("blog").await?
```

### Builder Pattern

All complex queries use type-safe builders:
- `PostsBuilder` - Blog posts with filters
- `DashboardBuilder` - User dashboard with options
- `LikesBuilder` - Liked posts with pagination

### API Endpoints

- Base URL: `https://api.tumblr.com/v2`
- OAuth2 endpoints: `/oauth2/authorize` and `/oauth2/token`
- All requests include required User-Agent header
- Rate limits: 300/min per IP, 1000/hr per key

### Dependencies

- `reqwest`: HTTP client (json, multipart, rustls-tls)
- `serde`/`serde_json`: Serialization
- `thiserror`: Custom error types
- `url`: URL parsing

## Implementation Plan

**See `IMPLEMENTATION_PLAN.md` for the complete implementation roadmap**, including:
- Detailed architecture design
- API module organization (Blogs, Users, Posts, Tagged, Communities)
- Type system and models
- Phase-by-phase task list with completion status
- Technical decisions and rationale

**Current Status:** Phase 4 (API Modules) - ~40% complete
- ✅ Blogs API complete
- ✅ Users API complete
- 🚧 Posts, Tagged, Communities APIs remaining

The implementation plan provides comprehensive context for continuing development of this library.

## Important Notes

- Uses Rust edition 2024
- The flake.nix has a placeholder `projectName = "CHANGEME"` that should be updated to "crabrave"
- TLS uses `rustls-tls` rather than native-tls for better portability
- User-Agent header is **required** by Tumblr API (apps may be suspended without it)
