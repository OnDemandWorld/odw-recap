package models

import (
	"time"

	"github.com/google/uuid"
)

// User represents a user in the system
type User struct {
	ID           uuid.UUID `json:"id"`
	Email        string    `json:"email"`
	PasswordHash string    `json:"-"`
	Name         string    `json:"name"`
	Role         string    `json:"role"`
	CreatedAt    time.Time `json:"created_at"`
	UpdatedAt    time.Time `json:"updated_at"`
}

// Organization represents an organization
type Organization struct {
	ID        uuid.UUID `json:"id"`
	Name      string    `json:"name"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

// OrganizationMember represents a user's membership in an organization
type OrganizationMember struct {
	OrganizationID uuid.UUID `json:"organization_id"`
	UserID         uuid.UUID `json:"user_id"`
	Role           string    `json:"role"`
	JoinedAt       time.Time `json:"joined_at"`
}

// Meeting represents a meeting
type Meeting struct {
	ID              uuid.UUID  `json:"id"`
	OrganizationID  uuid.UUID  `json:"organization_id"`
	CreatedBy       *uuid.UUID `json:"created_by"`
	Title           *string    `json:"title"`
	MeetingType     *string    `json:"meeting_type"`
	Location        *string    `json:"location"`
	Language        *string    `json:"language"`
	Topic           *string    `json:"topic"`
	StartedAt       *time.Time `json:"started_at"`
	EndedAt         *time.Time `json:"ended_at"`
	DurationSeconds *int       `json:"duration_seconds"`
	AudioFilePath   *string    `json:"audio_file_path"`
	AudioFormat     *string    `json:"audio_format"`
	AudioSizeBytes  *int64     `json:"audio_size_bytes"`
	ModelUsed       *string    `json:"model_used"`
	STTProvider     *string    `json:"stt_provider"`
	LLMProvider     *string    `json:"llm_provider"`
	Status          string     `json:"status"`
	VaultEntryID    *string    `json:"vault_entry_id"`
	LoopTaskID      *string    `json:"loop_task_id"`
	CreatedAt       time.Time  `json:"created_at"`
	UpdatedAt       time.Time  `json:"updated_at"`
}

// TranscriptSegment represents a segment of transcript
type TranscriptSegment struct {
	ID         int64      `json:"id"`
	MeetingID  uuid.UUID  `json:"meeting_id"`
	SpeakerID  *string    `json:"speaker_id"`
	StartMs    int64      `json:"start_ms"`
	EndMs      int64      `json:"end_ms"`
	Text       string     `json:"text"`
	Confidence *float32   `json:"confidence"`
	IsFinal    bool       `json:"is_final"`
	Version    int        `json:"version"`
	CreatedAt  time.Time  `json:"created_at"`
}

// Summary represents a meeting summary
type Summary struct {
	ID             int64     `json:"id"`
	MeetingID      uuid.UUID `json:"meeting_id"`
	Content        string    `json:"content"`
	GenerationMode string    `json:"generation_mode"`
	ModelUsed      *string   `json:"model_used"`
	PromptTemplate *string   `json:"prompt_template"`
	CreatedAt      time.Time `json:"created_at"`
}

// ActionItem represents an action item from a meeting
type ActionItem struct {
	ID          int64      `json:"id"`
	MeetingID   uuid.UUID  `json:"meeting_id"`
	Description string     `json:"description"`
	Assignee    *string    `json:"assignee"`
	DueDate     *time.Time `json:"due_date"`
	Status      string     `json:"status"`
	CreatedAt   time.Time  `json:"created_at"`
}

// Decision represents a decision made in a meeting
type Decision struct {
	ID          int64     `json:"id"`
	MeetingID   uuid.UUID `json:"meeting_id"`
	Description string    `json:"description"`
	Rationale   *string   `json:"rationale"`
	CreatedAt   time.Time `json:"created_at"`
}

// AuditLog represents an audit log entry
type AuditLog struct {
	ID           int64      `json:"id"`
	UserID       *uuid.UUID `json:"user_id"`
	Action       string     `json:"action"`
	ResourceType *string    `json:"resource_type"`
	ResourceID   *uuid.UUID `json:"resource_id"`
	Details      *string    `json:"details"`
	IPAddress    *string    `json:"ip_address"`
	UserAgent    *string    `json:"user_agent"`
	CreatedAt    time.Time  `json:"created_at"`
}
