# Routes and Middleware

This starter is meant to make new API routes fast to add without weakening the Flowless security model. Most application work happens in two places:

- Add route handlers in `src/routes/api.rs` or a new file under `src/routes/`.
- Register those handlers in `src/lib.rs` with the right auth middleware.

## Current Routes

### Root and Health

- `GET /`: service metadata.
- `GET /health`: process status.
- `GET /health/db`: SQLx database ping.
- `GET /health/cache`: local/Redis cache status.
- `GET /health/all`: combined health report.

### API Examples

- `GET /api/public`: no auth.
- `GET /api/protected`: requires a valid Flowless session.
- `GET /api/optional`: works anonymous or authenticated.
- `GET /api/profile`: requires auth and reads the validated session.
- `GET /api/admin/dashboard`: requires auth, then `admin` role.
- `GET /api/test/config`: dev-friendly config summary.

### Mock Task Routes

- `GET /api/tasks`
- `POST /api/tasks`
- `GET /api/tasks/{id}`
- `PUT /api/tasks/{id}`
- `DELETE /api/tasks/{id}`

Task routes are intentionally mock-only. They demonstrate route shape, auth, JSON extraction, path extraction, and session access without locking applications into a schema.

## Auth Middleware

The starter exposes these auth helpers from `crate::auth`:

| Helper | Use when | Behavior |
| --- | --- | --- |
| `require_auth` | The route must have a valid Flowless session | Missing or invalid session returns `401` |
| `optional_auth` | The route can personalize when a session exists | Missing or invalid session continues anonymous |
| `require_roles(["role"])` | The route allows one or more custom user types | Missing session returns `401`; wrong role returns `403` |
| `require_roles_csv("role,other")` | Same as `require_roles`, but handy for comma-separated literals | Trims whitespace and ignores empty entries |
| `require_admin()` | Shortcut for `require_roles(["admin"])` | Missing session returns `401`; non-admin returns `403` |

`require_auth` and `optional_auth` need `AppState`, so register them with `middleware::from_fn_with_state(state.clone(), ...)`.

Role guards read the already validated `SessionData` from request extensions, so they must run after `require_auth` has inserted the session.

## Session Sources

The middleware extracts a session ID in this order:

1. Header from `SESSION_HEADER_NAME`, default `X-Session-Id`.
2. Cookie from `SESSION_COOKIE_NAME`, default `session_id`.
3. Query parameter `session_id` only when `SESSION_ALLOW_QUERY=true`.

Examples:

```http
GET /api/profile
X-Session-Id: session_123
```

```http
GET /api/profile
Cookie: session_id=session_123
```

```http
GET /api/profile?session_id=session_123
```

Query sessions are convenient for local testing but should normally stay disabled in production.

## Handler Patterns

### Public Handler

Use this for routes that should never require a session.

```rust
use axum::{Json, response::IntoResponse};
use serde_json::json;

pub async fn public_stats() -> impl IntoResponse {
    Json(json!({
        "message": "Anyone can call this",
        "authenticated": false
    }))
}
```

Register it in `src/lib.rs`:

```rust
.route("/stats", get(routes::api::public_stats))
```

### Required Auth Handler

Use `Extension<SessionData>` when the route is protected. `require_auth` validates the session through Flowless Bridge, caches the result, then inserts `SessionData` into request extensions.

```rust
use axum::{extract::Extension, Json, response::IntoResponse};
use serde_json::json;
use crate::auth::SessionData;

pub async fn my_account(
    Extension(session): Extension<SessionData>,
) -> impl IntoResponse {
    Json(json!({
        "user_id": session.user_id,
        "email": session.email,
        "user_type": session.user_type
    }))
}
```

Register it with `require_auth`:

```rust
.route(
    "/account",
    get(routes::api::my_account).layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_auth,
    )),
)
```

### Optional Auth Handler

Use `Option<Extension<SessionData>>` when the route should work both anonymous and authenticated. This is good for public pages, pricing, preview APIs, or content where logged-in users receive extra data.

```rust
use axum::{extract::Extension, Json, response::IntoResponse};
use serde_json::json;
use crate::auth::SessionData;

pub async fn feed(
    session: Option<Extension<SessionData>>,
) -> impl IntoResponse {
    if let Some(Extension(session)) = session {
        Json(json!({
            "mode": "personalized",
            "user_id": session.user_id
        }))
    } else {
        Json(json!({
            "mode": "public"
        }))
    }
}
```

Register it with `optional_auth`:

```rust
.route(
    "/feed",
    get(routes::api::feed).layer(middleware::from_fn_with_state(
        state.clone(),
        auth::optional_auth,
    )),
)
```

### Admin Handler

Admin routes should use both middleware layers:

1. `require_auth` validates the session and inserts `SessionData`.
2. `require_admin()` checks `user_type` is exactly `admin`.

The order in `src/lib.rs` should match the existing route:

```rust
.route(
    "/admin/reports",
    get(routes::api::admin_reports)
        .layer(auth::require_admin())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        )),
)
```

This reads a little inside-out because Axum/Tower layers wrap the service. In this route, `require_auth` runs first, inserts `SessionData`, and then `require_admin` can read it. Keep this exact order for admin routes.

The handler can then safely read the session:

```rust
use axum::{extract::Extension, Json, response::IntoResponse};
use serde_json::json;
use crate::auth::SessionData;

pub async fn admin_reports(
    Extension(session): Extension<SessionData>,
) -> impl IntoResponse {
    Json(json!({
        "message": "Admin report",
        "admin_user_id": session.user_id
    }))
}
```

### Custom Role Handler

Use `require_roles` when a route should allow user types other than only `admin`, or when a route intentionally allows multiple roles.

```rust
.route(
    "/support/inbox",
    get(routes::api::support_inbox)
        .layer(auth::require_roles(["admin", "support", "owner"]))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        )),
)
```

If you prefer a comma-separated literal, use `require_roles_csv`:

```rust
.route(
    "/reports",
    get(routes::api::reports)
        .layer(auth::require_roles_csv("admin, manager, analyst"))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        )),
)
```

This is configured by the application developer in route code, not by environment variables. That keeps access rules close to the route they protect.

The guard checks `SessionData.user_type`, which comes from the Flowless Bridge response field `user.user_type`.

### Nesting Protected Route Groups

When several routes share the same requirement, create a nested router and apply middleware once.

```rust
let billing_routes = Router::new()
    .route("/", get(routes::api::billing_overview))
    .route("/invoices", get(routes::api::billing_invoices))
    .route("/payment-method", post(routes::api::update_payment_method));

let api_routes = Router::new()
    .nest(
        "/billing",
        billing_routes.layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        )),
    );
```

This keeps `src/lib.rs` readable as the app grows.

Role guards can also be applied to a whole group:

```rust
let support_routes = Router::new()
    .route("/inbox", get(routes::api::support_inbox))
    .route("/tickets/{id}", get(routes::api::support_ticket))
    .layer(auth::require_roles(["admin", "support"]))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_auth,
    ));

let api_routes = Router::new()
    .nest("/support", support_routes);
```

### JSON Body and Path Params

Use Axum extractors normally. Protected routes can combine `Extension<SessionData>`, `Path`, `Query`, and `Json`.

```rust
use axum::{
    extract::{Extension, Path},
    Json,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use crate::auth::SessionData;

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub completed: Option<bool>,
}

pub async fn update_real_task(
    Extension(session): Extension<SessionData>,
    Path(task_id): Path<String>,
    Json(body): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    Json(json!({
        "task_id": task_id,
        "owner_user_id": session.user_id,
        "title": body.title,
        "completed": body.completed
    }))
}
```

Register it:

```rust
.route(
    "/tasks/{id}",
    put(routes::api::update_real_task).layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_auth,
    )),
)
```

### Adding a New Route File

For larger apps, split domains into separate route files.

1. Create `src/routes/billing.rs`.
2. Export it from `src/routes/mod.rs`.
3. Register the router in `src/lib.rs`.

Example `src/routes/mod.rs`:

```rust
pub mod api;
pub mod billing;
pub mod health;
```

Example `src/routes/billing.rs`:

```rust
use axum::{extract::Extension, Json, response::IntoResponse};
use serde_json::json;
use crate::auth::SessionData;

pub async fn overview(
    Extension(session): Extension<SessionData>,
) -> impl IntoResponse {
    Json(json!({
        "user_id": session.user_id,
        "plan": "example"
    }))
}
```

Example registration:

```rust
let billing_routes = Router::new()
    .route("/", get(routes::billing::overview))
    .layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_auth,
    ));

let api_routes = Router::new()
    .nest("/billing", billing_routes);
```

## Error Behavior

Auth middleware returns simple status codes by design:

- `401 Unauthorized`: missing session, invalid session, Bridge validation rejected, Bridge unreachable after retries.
- `403 Forbidden`: session exists but `require_admin`, `require_roles`, or `require_roles_csv` rejected the role.

Route handlers should return domain-specific errors for application logic. Keep authentication and authorization errors in middleware so every route behaves consistently.

## Recommended Route Style

- Keep public routes explicit and rare.
- Prefer group-level middleware for route families.
- Use `Extension<SessionData>` only on routes protected by `require_auth`.
- Use `Option<Extension<SessionData>>` only on routes protected by `optional_auth`.
- Keep admin routes layered with both `require_auth` and `require_admin()`.
- Use `require_roles([...])` or `require_roles_csv("...")` for product-specific roles such as `owner`, `manager`, `support`, or `analyst`.
- Keep mock examples small; move real business logic into services/modules as the app grows.

## Quick Checklist

When adding a route:

1. Decide whether it is public, optional auth, required auth, admin, or custom-role protected.
2. Add the handler in `src/routes/api.rs` or a domain route file.
3. If it needs user data, add `Extension<SessionData>` to the handler.
4. Register the route in `src/lib.rs`.
5. Apply the correct middleware layer.
6. Add a test when the route is auth-sensitive.
