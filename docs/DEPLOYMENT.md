# Deployment

## Docker

```powershell
docker compose up --build
```

## Production Checklist

- Set `ENVIRONMENT=production`.
- Use Postgres or another production database URL.
- Set a long random `BRIDGE_VALIDATION_SECRET`.
- Generate and set `TRUST_TOKEN_PRIVATE_KEY`.
- Set `DEV_CORS_RELAXED=false`.
- Set explicit `CORS_ORIGINS`.
- Configure `FLOWLESS_API_URL` to the deployed Flowless API.
- Decide whether Redis is needed for multi-instance session validation cache.

## Health Checks

- `/health`: process status.
- `/health/db`: database ping.
- `/health/cache`: cache status.
- `/health/all`: combined status.

Use `/health/all` for deployment readiness when the database must be reachable before traffic.
