# Recap Team Server

Go-based backend for Recap by ODW.ai team features.

## Features

- **Authentication**: short-lived JWT access tokens (15 min) + rotating refresh tokens (30 days, revocable)
- **Authorization**: RBAC with admin/member roles; user/role re-checked against the database on every request
- **Hardening**: fail-closed JWT secret, per-IP rate limiting on auth endpoints, request body limits, audit logging for auth events
- **Meetings API**: CRUD operations for meetings, scoped to their creator
- **Sync API**: Queue meeting data from desktop app for synchronization
- **Admin Dashboard**: User management, audit logs, stats
- **Database**: PostgreSQL with automatic migrations
- **Cache/Sessions**: Redis support

## Architecture

```
team-server/
├── cmd/server/         # Application entry point
├── internal/
│   ├── api/            # HTTP handlers and routes
│   ├── auth/           # JWT and middleware
│   ├── db/             # PostgreSQL and Redis clients
│   └── models/         # Data models
├── go.mod
└── README.md
```

## Environment Variables

```bash
PORT=8080
DATABASE_URL=postgres://user:password@localhost:5432/recap?sslmode=disable
REDIS_URL=localhost:6379
JWT_SECRET=<required: long random string, e.g. `openssl rand -hex 32`>
# Development only: boot without JWT_SECRET (insecure secret is used)
RECAP_INSECURE_DEV=1
# Optional: comma-separated CORS origins (enables credentialed CORS)
CORS_ALLOWED_ORIGINS=https://app.example.com
```

The server refuses to start without `JWT_SECRET` unless `RECAP_INSECURE_DEV=1`.

## Running Locally

```bash
cd team-server
go run ./cmd/server
```

## Building

```bash
cd team-server
go build -o recap-team-server ./cmd/server
```

## Testing

```bash
go test ./...
```

## Status

Core API, auth (with refresh rotation, rate limiting, and audit logging), and
database migrations are implemented. Remaining work includes organization
management and a worker that processes the sync queue — see
`IMPROVEMENT_PLAN.md` in the repository root.

## API Endpoints

### Public
- `GET /health` - Health check
- `POST /auth/register` - Register new user (rate limited)
- `POST /auth/login` - Login (rate limited)
- `POST /auth/refresh` - Rotate a refresh token, get a new access token (rate limited)
- `POST /auth/logout` - Revoke a refresh token

### Authenticated
- `GET /me` - Current user
- `GET /meetings` - List meetings
- `POST /meetings` - Create meeting
- `GET /meetings/{id}` - Get meeting
- `PUT /meetings/{id}` - Update meeting
- `DELETE /meetings/{id}` - Delete meeting
- `POST /sync` - Sync meeting data

### Admin
- `GET /admin/users` - List users
- `GET /admin/audit-log` - View audit log
- `GET /admin/stats` - Server stats
