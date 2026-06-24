package db

import (
	"database/sql"
	"fmt"

	_ "github.com/lib/pq"
)

// Database wraps PostgreSQL connection
type Database struct {
	db *sql.DB
}

// NewDatabase creates a new database connection
func NewDatabase(dsn string) (*Database, error) {
	db, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	// Test connection
	if err := db.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	// Run migrations
	if err := runMigrations(db); err != nil {
		return nil, fmt.Errorf("failed to run migrations: %w", err)
	}

	return &Database{db: db}, nil
}

// Close closes the database connection
func (d *Database) Close() error {
	return d.db.Close()
}

// DB returns the underlying sql.DB
func (d *Database) DB() *sql.DB {
	return d.db
}

// runMigrations runs database migrations
func runMigrations(db *sql.DB) error {
	migrations := []string{
		// Users table
		`CREATE TABLE IF NOT EXISTS users (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			email VARCHAR(255) UNIQUE NOT NULL,
			password_hash VARCHAR(255) NOT NULL,
			name VARCHAR(255) NOT NULL,
			role VARCHAR(50) NOT NULL DEFAULT 'member',
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
			updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Organizations table
		`CREATE TABLE IF NOT EXISTS organizations (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			name VARCHAR(255) NOT NULL,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
			updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Organization members table
		`CREATE TABLE IF NOT EXISTS organization_members (
			organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
			user_id UUID REFERENCES users(id) ON DELETE CASCADE,
			role VARCHAR(50) NOT NULL DEFAULT 'member',
			joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
			PRIMARY KEY (organization_id, user_id)
		)`,

		// Meetings table (synced from desktop)
		`CREATE TABLE IF NOT EXISTS meetings (
			id UUID PRIMARY KEY,
			organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
			created_by UUID REFERENCES users(id) ON DELETE SET NULL,
			title TEXT,
			meeting_type VARCHAR(100),
			location TEXT,
			language VARCHAR(10),
			topic TEXT,
			started_at TIMESTAMP WITH TIME ZONE,
			ended_at TIMESTAMP WITH TIME ZONE,
			duration_seconds INTEGER,
			audio_file_path TEXT,
			audio_format VARCHAR(50),
			audio_size_bytes BIGINT,
			model_used VARCHAR(100),
			stt_provider VARCHAR(100),
			llm_provider VARCHAR(100),
			status VARCHAR(50) NOT NULL DEFAULT 'pending',
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
			updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Transcript segments table
		`CREATE TABLE IF NOT EXISTS transcript_segments (
			id BIGSERIAL PRIMARY KEY,
			meeting_id UUID REFERENCES meetings(id) ON DELETE CASCADE,
			speaker_id VARCHAR(100),
			start_ms BIGINT NOT NULL,
			end_ms BIGINT NOT NULL,
			text TEXT NOT NULL,
			confidence REAL,
			is_final BOOLEAN DEFAULT TRUE,
			version INTEGER DEFAULT 1,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Summaries table
		`CREATE TABLE IF NOT EXISTS summaries (
			id BIGSERIAL PRIMARY KEY,
			meeting_id UUID REFERENCES meetings(id) ON DELETE CASCADE,
			content TEXT NOT NULL,
			generation_mode VARCHAR(50) NOT NULL,
			model_used VARCHAR(100),
			prompt_template VARCHAR(255),
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Action items table
		`CREATE TABLE IF NOT EXISTS action_items (
			id BIGSERIAL PRIMARY KEY,
			meeting_id UUID REFERENCES meetings(id) ON DELETE CASCADE,
			description TEXT NOT NULL,
			assignee VARCHAR(255),
			due_date TIMESTAMP WITH TIME ZONE,
			status VARCHAR(50) DEFAULT 'pending',
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Decisions table
		`CREATE TABLE IF NOT EXISTS decisions (
			id BIGSERIAL PRIMARY KEY,
			meeting_id UUID REFERENCES meetings(id) ON DELETE CASCADE,
			description TEXT NOT NULL,
			rationale TEXT,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Sync queue table
		`CREATE TABLE IF NOT EXISTS sync_queue (
			id BIGSERIAL PRIMARY KEY,
			meeting_id UUID REFERENCES meetings(id) ON DELETE CASCADE,
			action VARCHAR(50) NOT NULL,
			payload JSONB NOT NULL,
			status VARCHAR(50) DEFAULT 'pending',
			retry_count INTEGER DEFAULT 0,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
			processed_at TIMESTAMP WITH TIME ZONE
		)`,

		// Audit log table
		`CREATE TABLE IF NOT EXISTS audit_log (
			id BIGSERIAL PRIMARY KEY,
			user_id UUID REFERENCES users(id) ON DELETE SET NULL,
			action VARCHAR(100) NOT NULL,
			resource_type VARCHAR(100),
			resource_id UUID,
			details JSONB,
			ip_address VARCHAR(45),
			user_agent TEXT,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
		)`,

		// Create indexes
		`CREATE INDEX IF NOT EXISTS idx_meetings_organization ON meetings(organization_id)`,
		`CREATE INDEX IF NOT EXISTS idx_meetings_created_by ON meetings(created_by)`,
		`CREATE INDEX IF NOT EXISTS idx_meetings_status ON meetings(status)`,
		`CREATE INDEX IF NOT EXISTS idx_transcript_segments_meeting ON transcript_segments(meeting_id)`,
		`CREATE INDEX IF NOT EXISTS idx_summaries_meeting ON summaries(meeting_id)`,
		`CREATE INDEX IF NOT EXISTS idx_action_items_meeting ON action_items(meeting_id)`,
		`CREATE INDEX IF NOT EXISTS idx_decisions_meeting ON decisions(meeting_id)`,
		`CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(status)`,
		`CREATE INDEX IF NOT EXISTS idx_audit_log_user ON audit_log(user_id)`,
		`CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON audit_log(created_at)`,
	}

	for _, migration := range migrations {
		if _, err := db.Exec(migration); err != nil {
			return fmt.Errorf("failed to execute migration: %w", err)
		}
	}

	return nil
}
