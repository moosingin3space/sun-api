---
globs:
  - "**/*.rs"
---

Follow these Rust conventions:

- Do not use `unwrap()` or `expect()` in production code.
- Use `Result` and `Option` instead of panicking.
- Follow the Rust style guide. Enforce this by running `cargo clippy` and `cargo fmt`.
- Do not import dependencies without user consent.
- Do not assume `tokio` is available, as Cloudflare Workers do not provide it.

We use the following libraries:

- `serde`: for serialization and deserialization.
- `axum`: as an HTTP router.
