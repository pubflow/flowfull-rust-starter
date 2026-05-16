# Contributing

## Goals

Keep this starter small, secure, and easy to replace. It should provide backend structure and Flowless security patterns, not become a product-specific app.

## Development Checks

Run before opening a PR:

```powershell
cargo fmt --all -- --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --all-features --no-deps
```

## Guidelines

- Keep Bridge validation behavior compatible with Flowless.
- Keep task routes mock-oriented unless a separate example app is requested.
- Do not add local path dependencies to the Flowfull Rust client.
- Prefer typed configuration over ad hoc environment reads.
- Add tests for middleware, validation, cache, token, and route behavior when changing auth-sensitive code.
