-- Feedback Module + Enhanced Session Storage
-- Migration: 003_feedback_and_session_storage.sql
-- Target: docdb PostgreSQL

BEGIN;

-- ─────────────────────────────────────────────────────────────
-- 1. Enhance conversion_jobs with session file storage info
-- ─────────────────────────────────────────────────────────────
ALTER TABLE conversion_jobs
    ADD COLUMN IF NOT EXISTS source_zip_key TEXT,
    ADD COLUMN IF NOT EXISTS result_docx_key TEXT,
    ADD COLUMN IF NOT EXISTS result_report_key TEXT,
    ADD COLUMN IF NOT EXISTS report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS result_log_key TEXT,
    ADD COLUMN IF NOT EXISTS storage_path TEXT,
    ADD COLUMN IF NOT EXISTS zip_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS docx_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS log_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS worker_id TEXT,
    ADD COLUMN IF NOT EXISTS locked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS attempts INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS next_run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS started_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS failed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_queue
    ON conversion_jobs(status, next_run_at, created_at)
    WHERE status IN ('queued', 'normalizing', 'detecting', 'analyzing', 'compiling', 'rendering', 'verifying');

CREATE TABLE IF NOT EXISTS app_access_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

ALTER TABLE app_access_tokens
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS last_used_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS commercial_entitlements (
    user_id UUID PRIMARY KEY REFERENCES app_users(id) ON DELETE CASCADE,
    count_balance BIGINT NOT NULL DEFAULT 0 CHECK (count_balance >= 0),
    valid_until TIMESTAMPTZ,
    source_order_id TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS usage_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    conversion_job_id UUID REFERENCES conversion_jobs(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('reserve', 'commit', 'refund', 'grant', 'adjust')),
    quantity BIGINT NOT NULL,
    balance_after BIGINT,
    source TEXT NOT NULL DEFAULT 'system',
    reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ─────────────────────────────────────────────────────────────
-- 1b. Enhance uploads with result storage keys
-- ─────────────────────────────────────────────────────────────
ALTER TABLE uploads
    ADD COLUMN IF NOT EXISTS result_docx_key TEXT,
    ADD COLUMN IF NOT EXISTS result_log_key TEXT;

-- ─────────────────────────────────────────────────────────────
-- 2. Feedback threads
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS feedback_threads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    conversion_job_id UUID REFERENCES conversion_jobs(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    feedback_type TEXT NOT NULL CHECK (feedback_type IN ('issue', 'requirement')),
    status TEXT NOT NULL DEFAULT 'open'
        CHECK (status IN ('open', 'in_progress', 'resolved', 'closed')),
    priority TEXT NOT NULL DEFAULT 'normal'
        CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    admin_assignee UUID REFERENCES app_users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_feedback_threads_user
    ON feedback_threads(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_feedback_threads_job
    ON feedback_threads(conversion_job_id) WHERE conversion_job_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_feedback_threads_status
    ON feedback_threads(status, created_at DESC);

-- ─────────────────────────────────────────────────────────────
-- 3. Feedback messages (chat thread)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS feedback_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id UUID NOT NULL REFERENCES feedback_threads(id) ON DELETE CASCADE,
    parent_message_id UUID REFERENCES feedback_messages(id) ON DELETE SET NULL,
    sender_user_id UUID REFERENCES app_users(id) ON DELETE SET NULL,
    sender_type TEXT NOT NULL CHECK (sender_type IN ('user', 'admin', 'system')),
    content TEXT NOT NULL,
    attachments JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_internal BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_feedback_messages_thread
    ON feedback_messages(thread_id, created_at ASC);

COMMIT;
