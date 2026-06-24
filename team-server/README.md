# Recap Team Server

Go-based backend for Recap by ODW.ai team features.

## Features

- **Authentication**: JWT-based authentication with registration and login
- **Authorization**: RBAC with admin/member roles
- **Meetings API**: CRUD operations for meetings
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
JWT_SECRET=your-secret-key
```

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
