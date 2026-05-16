# Bridge Validation

The starter implements Bridge validation locally with `reqwest` so the request middleware has a stable, explicit security path. The published `flowfull = "0.1.0"` crate is installed for client helpers and direct Flowfull usage where applications need it.

## Request

```http
POST /auth/bridge/validate
X-Bridge-Secret: <BRIDGE_VALIDATION_SECRET>
Content-Type: application/json
```

Body:

```json
{
  "session_id": "session_123",
  "ip": "127.0.0.1",
  "user_agent": "Mozilla/5.0",
  "device_id": "device_123"
}
```

When `BRIDGE_SECRET_IN_BODY=true`, the body also includes `bridge_secret` for older compatibility flows.

## Response Mapping

Flowless response user/session data maps into local `SessionData`:

- `user.id` -> `user_id`
- `user.email` -> `email`
- `user.name` -> `name`
- `user.user_type` -> `user_type`
- `session.expires_at` -> `expires_at`
- `validated_at` is set locally

## Validation Modes

- `DISABLED`: no IP, user-agent, or device fields.
- `STANDARD`: IP when enabled.
- `ADVANCED`: IP and user-agent when enabled.
- `STRICT`: IP, user-agent, and device ID when enabled.

These modes only shape the validation request. The final allow/reject decision belongs to Flowless.
