# myapp

A simple REST API built with [Axum](https://github.com/tokio-rs/axum) and Tokio.

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

| Variable       | Default                        | Description              |
|----------------|--------------------------------|--------------------------|
| `HOST`         | `0.0.0.0`                      | Bind address             |
| `PORT`         | `8080`                         | Listen port              |
| `DATABASE_URL` | `postgres://localhost/myapp`   | PostgreSQL connection    |
| `JWT_SECRET`   | *(required)*                   | Token signing secret     |
