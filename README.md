# Flowfull Rust Starter

Production-ready Rust backend starter for Flowless applications. It is modeled after the Go starter, but built with idiomatic Rust, Axum, Tokio, SQLx, local Bridge validation, typed configuration, health checks, protected routes, optional authentication, admin guards, local TTL caching, optional Redis, and tests.

This starter is intentionally not a product app. The task routes are mock CRUD examples so teams can replace them quickly with their own domain logic.

## Stack

- Axum for HTTP routing and middleware
- Tokio for async runtime
- SQLx for database pools and health checks
- Reqwest for local Flowless Bridge validation
- Moka local TTL cache with optional Redis backing
- PASETO v4.public trust tokens for secure starter workflows
- Tower HTTP CORS and tracing layers

## Flowfull Crate Dependency

This template uses the published Flowfull Rust client crate:

```toml
flowfull = "0.1.0"
```

It does not use a local path dependency. Bridge validation remains implemented locally in `src/auth/bridge_validator.rs` so the starter owns its middleware behavior and can stay compatible with Flowless Bridge deployments even when users do not need direct client helpers.

## Quick Start

```powershell
Copy-Item .env.example .env
cargo run
```

Default server:

```text
http://localhost:3001
```

Useful endpoints:

- `GET /`
- `GET /health`
- `GET /health/all`
- `GET /api/public`
- `GET /api/protected` with `X-Session-Id`
- `GET /api/optional`
- `GET /api/profile` with `X-Session-Id`
- `GET /api/admin/dashboard` with an admin session
- `/api/tasks` mock protected CRUD routes

## Required Production Configuration

Set these before production deploys:

```env
ENVIRONMENT=production
DATABASE_URL=postgres://user:password@host:5432/database
FLOWLESS_API_URL=https://your-flowless-api.example.com
BRIDGE_VALIDATION_SECRET=replace-with-a-long-random-secret
TRUST_TOKEN_PRIVATE_KEY=replace-with-generated-private-key
DEV_CORS_RELAXED=false
```

Generate a trust token keypair:

```powershell
cargo run --example generate_trust_token_key
```

## Bridge Validation

The local validator sends:

- `POST {FLOWLESS_API_URL}{BRIDGE_VALIDATION_ENDPOINT}`
- `X-Bridge-Secret: {BRIDGE_VALIDATION_SECRET}`
- JSON body with `session_id`, optional `ip`, `user_agent`, `device_id`, and optional `bridge_secret`

Default endpoint:

```env
BRIDGE_VALIDATION_ENDPOINT=/auth/bridge/validate
```

Older Flowless deployments can use:

```env
BRIDGE_VALIDATION_ENDPOINT=/api/bridge/validate
```

Compatibility body secret:

```env
BRIDGE_SECRET_IN_BODY=true
```

## Authentication Behavior

Session extraction order:

1. Header from `SESSION_HEADER_NAME`, default `X-Session-Id`
2. Cookie from `SESSION_COOKIE_NAME`, default `session_id`
3. Query `session_id` when `SESSION_ALLOW_QUERY=true`

Middleware:

- `require_auth`: returns `401` when missing or invalid
- `optional_auth`: continues anonymous when absent or invalid
- `require_admin`: returns `403` for non-admin user types

Validated `SessionData` is stored in Axum request extensions.

## Verification

```powershell
cargo fmt --all -- --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --all-features --no-deps
```

## Project Layout

```text
src/
  auth/      Bridge validation, validation modes, middleware
  cache/     Local TTL cache plus optional Redis
  config/    Typed env configuration and validation
  db/        SQLx pool and health checks
  routes/    Root, health, public/protected/admin/tasks examples
  tokens/    PASETO v4.public key generation and verification
docs/        Architecture and operational notes
scripts/     Helper scripts
tests/       Integration and unit-style starter tests
to-do/       Implementation plan and follow-up checklist
```
