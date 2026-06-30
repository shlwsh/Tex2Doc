-- Automation R&D Module
-- Migration: 004_automation_rnd.sql
-- Target: docdb PostgreSQL
-- Description: Tables for AI-powered automated development workflow

BEGIN;

-- ─────────────────────────────────────────────────────────────
-- 1. Automation requests (主申请表)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS automation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    short_id TEXT NOT NULL UNIQUE,
    source_type TEXT NOT NULL CHECK (source_type IN ('feedback', 'github_issue', 'admin_manual', 'ci_failure')),
    source_id TEXT NOT NULL,
    feedback_thread_id UUID REFERENCES feedback_threads(id) ON DELETE SET NULL,
    conversion_job_id UUID REFERENCES conversion_jobs(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    request_type TEXT NOT NULL DEFAULT 'unknown' CHECK (request_type IN ('bug', 'requirement', 'docs', 'test', 'ops', 'unknown')),
    status TEXT NOT NULL DEFAULT 'submitted'
        CHECK (status IN (
            'submitted',
            'triaged',
            'needs_approval',
            'queued_for_dev',
            'claimed',
            'coding',
            'local_validating',
            'local_failed',
            'pr_open',
            'ci_running',
            'ci_failed',
            'ready_for_merge',
            'production_deployed',
            'notified',
            'needs_human',
            'blocked',
            'closed',
            'rejected'
        )),
    priority TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'urgent')),
    risk_level TEXT NOT NULL DEFAULT 'unknown' CHECK (risk_level IN ('low', 'medium', 'high', 'critical', 'unknown')),
    ai_summary TEXT,
    impact_check_result JSONB NOT NULL DEFAULT '{}'::jsonb,
    acceptance_criteria JSONB NOT NULL DEFAULT '[]'::jsonb,
    claimed_by TEXT,
    claimed_at TIMESTAMPTZ,
    branch_name TEXT,
    worktree_path TEXT,
    pr_url TEXT,
    ci_run_url TEXT,
    local_validation_log TEXT,
    deployed_version TEXT,
    admin_approver_id UUID REFERENCES app_users(id) ON DELETE SET NULL,
    approved_at TIMESTAMPTZ,
    admin_rejector_id UUID REFERENCES app_users(id) ON DELETE SET NULL,
    rejected_at TIMESTAMPTZ,
    rejection_reason TEXT,
    escalated_to TEXT,
    escalated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (source_type, source_id)
);

CREATE INDEX IF NOT EXISTS idx_automation_requests_status ON automation_requests(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_automation_requests_risk ON automation_requests(risk_level, status);
CREATE INDEX IF NOT EXISTS idx_automation_requests_source ON automation_requests(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_automation_requests_feedback ON automation_requests(feedback_thread_id) WHERE feedback_thread_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_automation_requests_claimed ON automation_requests(claimed_by) WHERE claimed_by IS NOT NULL;

-- ─────────────────────────────────────────────────────────────
-- 2. Automation request events (事件时间线)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS automation_request_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES automation_requests(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'request_created',
        'triage_completed',
        'approval_requested',
        'approved',
        'rejected',
        'escalated',
        'agent_claimed',
        'agent_heartbeat',
        'branch_created',
        'impact_checked',
        'coding_started',
        'coding_completed',
        'local_validation_started',
        'local_validation_passed',
        'local_validation_failed',
        'pr_opened',
        'ci_started',
        'ci_passed',
        'ci_failed',
        'ready_for_merge',
        'merged',
        'production_deployed',
        'feedback_notified',
        'paused',
        'resumed',
        'retry_triggered',
        'closed'
    )),
    actor_type TEXT NOT NULL CHECK (actor_type IN ('user', 'admin', 'ai', 'agent', 'github', 'system')),
    actor_id TEXT,
    actor_name TEXT,
    from_status TEXT,
    to_status TEXT,
    message TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_automation_events_request ON automation_request_events(request_id, created_at ASC);

-- ─────────────────────────────────────────────────────────────
-- 3. Automation agents (开发机 Agent 注册)
-- ─────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS automation_agents (
    id TEXT PRIMARY KEY,
    hostname TEXT NOT NULL,
    agent_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'online' CHECK (status IN ('online', 'offline', 'busy', 'paused')),
    current_request_id UUID REFERENCES automation_requests(id) ON DELETE SET NULL,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    total_tasks_completed INTEGER NOT NULL DEFAULT 0,
    total_tasks_failed INTEGER NOT NULL DEFAULT 0,
    last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_task_at TIMESTAMPTZ,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_automation_agents_status ON automation_agents(status, last_heartbeat_at DESC);

-- ─────────────────────────────────────────────────────────────
-- 4. Helper function to generate short_id
-- ─────────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION generate_automation_short_id()
RETURNS TEXT AS $$
DECLARE
    chars TEXT := 'abcdefghijklmnopqrstuvwxyz0123456789';
    result TEXT := '';
    i INTEGER;
BEGIN
    FOR i IN 1..5 LOOP
        result := result || substr(chars, floor(random() * 36 + 1)::integer, 1);
    END LOOP;
    RETURN 'REQ-' || result;
END;
$$ LANGUAGE plpgsql;

-- ─────────────────────────────────────────────────────────────
-- 5. Trigger to update updated_at
-- ─────────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION update_automation_request_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION update_automation_agent_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_automation_requests_updated ON automation_requests;
CREATE TRIGGER trigger_automation_requests_updated
    BEFORE UPDATE ON automation_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_automation_request_timestamp();

DROP TRIGGER IF EXISTS trigger_automation_agents_updated ON automation_agents;
CREATE TRIGGER trigger_automation_agents_updated
    BEFORE UPDATE ON automation_agents
    FOR EACH ROW
    EXECUTE FUNCTION update_automation_agent_timestamp();

-- ─────────────────────────────────────────────────────────────
-- 6. Auto-create automation request from feedback (trigger)
-- ─────────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION create_automation_request_from_feedback()
RETURNS TRIGGER AS $$
DECLARE
    new_request_id UUID;
BEGIN
    -- Only trigger when priority is high or urgent
    IF NEW.priority IN ('high', 'urgent') THEN
        INSERT INTO automation_requests (
            source_type,
            source_id,
            feedback_thread_id,
            title,
            request_type,
            status,
            priority,
            risk_level,
            ai_summary,
            acceptance_criteria
        ) VALUES (
            'feedback',
            NEW.id::TEXT,
            NEW.id,
            NEW.title,
            CASE WHEN NEW.feedback_type = 'issue' THEN 'bug' ELSE 'requirement' END,
            'submitted',
            NEW.priority,
            CASE WHEN NEW.priority = 'urgent' THEN 'high' ELSE 'medium' END,
            'Auto-generated from feedback: ' || NEW.title,
            '[]'::jsonb
        )
        ON CONFLICT (source_type, source_id) DO NOTHING;

        -- Record event
        SELECT id INTO new_request_id FROM automation_requests
        WHERE source_type = 'feedback' AND source_id = NEW.id::TEXT;

        IF new_request_id IS NOT NULL THEN
            INSERT INTO automation_request_events (
                request_id, event_type, actor_type, actor_id, actor_name,
                message, payload
            ) VALUES (
                new_request_id, 'request_created', 'system', NULL, 'system',
                'Automation request auto-created from feedback thread', '{"feedback_priority": "' || NEW.priority || '"}'::jsonb
            );
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_feedback_to_automation ON feedback_threads;
CREATE TRIGGER trigger_feedback_to_automation
    AFTER INSERT ON feedback_threads
    FOR EACH ROW
    EXECUTE FUNCTION create_automation_request_from_feedback();

COMMIT;
