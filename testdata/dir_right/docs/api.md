# API Reference

Base URL: `http://localhost:8080`

---

## Authentication

All endpoints under `/api/v1/` and `/api/v2/` require a valid JWT in the
`Authorization` header:

```
Authorization: Bearer <token>
```

Obtain a token by posting credentials to `/auth/login`.

---

## Health

### `GET /health`

Returns the current service status and application version.

**Response 200**
```json
{ "status": "ok", "version": "0.2.0" }
```

### `GET /health/ready`

Returns `200` when the database is reachable, `503` otherwise.

### `GET /health/live`

Always returns `200` while the process is running.

---

## Users — v1

### `GET /api/v1/users`

List users with pagination.

**Query parameters**

| Name       | Type   | Default | Description              |
|------------|--------|---------|--------------------------|
| `page`     | int    | 1       | Page number (1-based)    |
| `per_page` | int    | 20      | Items per page (max 100) |
| `sort_by`  | string | —       | Field to sort by         |

### `POST /api/v1/users`

Create a new user account.

**Request body**
```json
{
  "email": "user@example.com",
  "username": "alice",
  "password": "SecurePass1"
}
```

### `GET /api/v1/users/:id`

Fetch a single user by UUID.

### `PUT /api/v1/users/:id`

Update a user's email or username.

### `DELETE /api/v1/users/:id`

Permanently delete a user. Returns `204 No Content`.

---

## Users — v2

The v2 endpoints return enriched user objects that include post count and
last-activity timestamp. Pagination is capped at 50 items per page.

### `GET /api/v2/users`

### `GET /api/v2/users/:id`

---

## Posts

### `GET /api/v1/posts`
### `POST /api/v1/posts`
### `GET /api/v1/posts/:id`
### `PUT /api/v1/posts/:id`
### `DELETE /api/v1/posts/:id`
### `GET /api/v1/posts/:id/comments`
### `POST /api/v1/posts/:id/comments`

---

## Admin

Requires `Admin` role.

### `GET /api/v1/admin/stats`

Returns user count, post count, cache size, and metrics snapshot.

### `POST /api/v1/admin/cache/clear`

Evicts all in-memory cache entries.

### `POST /api/v1/admin/users/:id/ban`

Suspends a user account.

### `POST /api/v1/admin/users/:id/unban`

Reinstates a previously banned user.
