# Quickstart

## 1. Configure

```powershell
Copy-Item .env.example .env
```

For local development, the defaults are enough to start the server. Protected routes need a real Flowless Bridge endpoint or a mocked endpoint during tests.

## 2. Run

```powershell
cargo run
```

Open:

```text
http://localhost:3001/health
```

## 3. Test Auth Routes

Public:

```powershell
Invoke-RestMethod http://localhost:3001/api/public
```

Protected:

```powershell
Invoke-RestMethod http://localhost:3001/api/protected -Headers @{"X-Session-Id"="your-session-id"}
```

Optional:

```powershell
Invoke-RestMethod http://localhost:3001/api/optional
```

## 4. Validate

```powershell
cargo fmt --all -- --check
cargo check
cargo test
```

## 5. Flowfull Client

The starter already depends on the published Flowfull Rust client:

```toml
flowfull = "0.1.0"
```

The starter's local Bridge validator remains the default security path for request middleware.
