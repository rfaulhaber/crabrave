# Live-capture fixtures

Every file here is a real Tumblr API v2 response, captured 2026-07-31
against the `crabrave-demo` test blog (plus account-level endpoints of
its owning account). They exist so the mock tests validate the client
against the API's actual wire shapes rather than against fixtures
written to match the code — see TODO.org Phase 4.

Post-capture processing, in order:

1. **Trimming** — long arrays (`posts`, `notes`, `liked_posts`, the
   `tagged` response) were sliced to the first few elements. Slicing
   only; no element was edited.
2. **Sanitization** — third-party blog names were mapped consistently
   to `example-blog-NNN` (the same pseudonym everywhere a name appears,
   including inside URLs and legacy HTML bodies), blog UUIDs to
   `t:FixtureUuidNNN…`, media/avatar URL hashes to runs of `a`s, and
   the owning account's blogs to `demo-primary`/`demo-sideblog`/
   `demo-bot` with generic titles and descriptions. Values only.
3. **Shape verification** — after each pass, the recursive structure
   (key paths, JSON types, array lengths) was checked to be identical
   to the raw capture.

Notable files:

- `blog_following_403.json`, `blog_likes_403.json` — real error
  envelopes: following/likes of a *secondary* blog are 403 for its own
  owner (likes additionally require the share-likes setting).
- `blog_followers.json`, `blog_submissions.json`, `blog_pages.json` —
  real empty-state responses; the demo blog has no followers,
  submissions, or pages, and these states can't be seeded via the API.
- `blog_notes*.json` — one heavily-noted reblog fetched in all five
  modes; `blog_notes_page2.json` was fetched by replaying the
  `_links.next` cursor (note `before_timestamp` is a quoted string).
- `post_create_*.json` / `post_delete.json` — creation returns `id` as
  a JSON string; deletion returns `id` as a JSON number plus
  `id_string`.
- `blog_drafts.json` / `blog_queue.json` — captured after seeding one
  draft and one queued post through the API (deleted afterwards).
- `avatar_redirect.json` — not a response envelope: records the 302 +
  `Location` behavior of the avatar endpoint.
- `user_dashboard_npf.json`, `user_likes.json`, `user_following.json`,
  `tagged.json` — account-level/global endpoints, so their elements are
  third-party posts (pseudonymized as above).

Re-capturing: `tumblr.nu` (see the adjacent project) with the account's
OAuth token; the capture/sanitize scripts live in the session scratchpad
and are cheap to recreate from this description.
