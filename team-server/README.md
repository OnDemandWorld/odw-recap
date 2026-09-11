# Recap Team Server

Go-based backend for Recap by ODW.ai team features.

## Features

- **Authentication**: JWT-based authentication with registration and login
- **Authorization**: RBAC with admin/member roles
- **Meetings API**: CRUD operations for meetings
- **Sync API**: Pushes meeting knowledge cross-product — uploads a Markdown
  summary to Vault (`POST {VAULT_API_URL}/files/upload`) and triggers a Loop
  workflow webhook for action items (`POST {LOOP_API_URL}/webhooks/{trigger_id}`,
  HMAC-SHA256 signed when a secret is configured). Also records the request in
  `sync_queue` for auditability.
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
REDIS_URL=localhost:6379            # bare host:port or redis://[:password@]host:port[/db]
JWT_SECRET=your-secret-key          # required in production; unset = random ephemeral secret
ALLOWED_ORIGINS=                    # optional, comma-separated CORS origins for browser clients

# Cross-product sync (Recap -> Vault / Loop)
VAULT_API_URL=http://localhost:8765
LOOP_API_URL=http://localhost:3000
LOOP_WEBHOOK_TRIGGER_ID=            # optional; when empty, Loop triggering is skipped
LOOP_WEBHOOK_SECRET=                # optional; HMAC-SHA256 shared secret for the Loop webhook
```

**JWT_SECRET policy:** when unset, the server starts with a random ephemeral
secret and all tokens become invalid on every restart. An explicitly
configured secret must be at least 32 characters or the server refuses to
start.

**Security notes:** JSON request bodies are capped at 4 MiB; login attempts
are rate-limited (10 per minute per client IP + account); CORS never allows
credentials.

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

Core API, auth, database migrations, tenant-scoped meetings (ownership
enforced on list/get/update/delete), best-effort audit logging, and the
cross-product sync forwarder are implemented. Remaining work includes
organization management and real-time sync processing.

See [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) for the Rust-side desktop-app
blockers (real audio capture, local whisper.cpp/llama.cpp inference, speaker
diarization, streaming transcription, pipeline-to-UI wiring, and code
signing/notarization/auto-update) that require a Rust toolchain to address.

## API Endpoints

### Public
- `GET /health` - Health check
- `POST /auth/register` - Register new user
- `POST /auth/login` - Login

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
