# myapp

A REST API built with [Axum](https://github.com/tokio-rs/axum) and Tokio,
featuring versioned endpoints, structured logging, and Prometheus metrics.

## Requirements

- Rust 1.75 or later
- PostgreSQL 14+

## Getting started

```bash
# Copy and edit the example environment file
cp .env.example .env

# Apply database migrations
cargo run --bin migrate

# Start the server
cargo run
```

The server listens on `http://0.0.0.0:8080` by default.

## Environment variables

| Variable              | Default                        | Description                   |
|-----------------------|--------------------------------|-------------------------------|
| `HOST`                | `0.0.0.0`                      | Bind address                  |
| `PORT`                | `8080`                         | Listen port                   |
| `DATABASE_URL`        | `postgres://localhost/myapp`   | PostgreSQL connection          |
| `DATABASE_POOL_SIZE`  | `10`                           | Connection pool size           |
| `JWT_SECRET`          | *(required)*                   | Token signing secret           |
| `JWT_EXPIRY_SECS`     | `3600`                         | Token lifetime in seconds      |
| `METRICS_ENABLED`     | `true`                         | Enable Prometheus metrics      |

## API versions

- **v1** — stable, original endpoint set
- **v2** — enriched user profiles, stricter pagination limits

See `docs/api.md` for the full reference.
