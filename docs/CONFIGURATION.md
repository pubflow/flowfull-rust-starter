# Configuration

Configuration is loaded from defaults, optional `.env`, and environment variables.

## Required In Production

```env
ENVIRONMENT=production
DATABASE_URL=postgres://user:password@host:5432/database
FLOWLESS_API_URL=https://your-flowless-api.example.com
BRIDGE_VALIDATION_SECRET=replace-with-a-long-random-secret
TRUST_TOKEN_PRIVATE_KEY=replace-with-generated-private-key
DEV_CORS_RELAXED=false
```

## Bridge

- `FLOWLESS_API_URL`: Flowless API base URL.
- `BRIDGE_VALIDATION_ENDPOINT`: default `/auth/bridge/validate`.
- `BRIDGE_VALIDATION_SECRET`: sent as `X-Bridge-Secret`.
- `BRIDGE_SECRET_IN_BODY`: compatibility option for older servers.
- `BRIDGE_VALIDATION_TIMEOUT_MS`: request timeout.
- `BRIDGE_RETRY_ATTEMPTS`: retry count with exponential backoff.

## Session

- `SESSION_HEADER_NAME`: default `X-Session-Id`.
- `SESSION_COOKIE_NAME`: default `session_id`.
- `SESSION_ALLOW_QUERY`: enables `?session_id=...`.
- `SESSION_VALIDATION_CACHE_TTL`: local/Redis cache TTL in seconds.

## Cache

- `CACHE_ENABLED`: enables local cache.
- `CACHE_MAX_CAPACITY`: max local cache entries.
- `REDIS_URL`: optional Redis backing store.

## CORS

When `DEV_CORS_RELAXED=true` and credentials are enabled, the starter mirrors the request origin instead of using `*`, because browsers reject wildcard origins with credentialed requests.
