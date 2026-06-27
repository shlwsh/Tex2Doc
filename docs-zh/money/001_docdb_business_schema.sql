-- Tex2Doc commercial database baseline for PostgreSQL.
-- Target database: docdb
-- Suggested local bootstrap:
--   createdb -U postgres docdb
--   psql -U postgres -d docdb -f docs-zh/money/001_docdb_business_schema.sql

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS app_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    role TEXT NOT NULL DEFAULT 'user'
        CHECK (role IN ('user', 'admin', 'operator', 'support')),
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'locked', 'disabled', 'pending', 'deleted')),
    default_plan_id TEXT NOT NULL DEFAULT 'preview',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE app_users
    ADD COLUMN IF NOT EXISTS role TEXT NOT NULL DEFAULT 'user'
    CHECK (role IN ('user', 'admin', 'operator', 'support'));

ALTER TABLE app_users
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active',
    ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;

ALTER TABLE app_users
    DROP CONSTRAINT IF EXISTS app_users_role_check,
    DROP CONSTRAINT IF EXISTS app_users_status_check;

ALTER TABLE app_users
    ADD CONSTRAINT app_users_role_check
    CHECK (role IN ('user', 'admin', 'operator', 'support')),
    ADD CONSTRAINT app_users_status_check
    CHECK (status IN ('active', 'locked', 'disabled', 'pending', 'deleted'));

CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    device_label TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS app_access_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_app_access_tokens_user
    ON app_access_tokens(user_id, created_at DESC);

ALTER TABLE app_access_tokens
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN IF NOT EXISTS last_used_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS billing_plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    price_cents INTEGER NOT NULL DEFAULT 0 CHECK (price_cents >= 0),
    monthly_conversions INTEGER NOT NULL CHECK (monthly_conversions >= 0),
    storage_bytes BIGINT NOT NULL CHECK (storage_bytes >= 0),
    features JSONB NOT NULL DEFAULT '[]'::jsonb,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES billing_plans(id),
    provider TEXT NOT NULL DEFAULT 'manual',
    provider_customer_id TEXT,
    provider_subscription_id TEXT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('trialing', 'active', 'past_due', 'canceled', 'expired')),
    current_period_start TIMESTAMPTZ NOT NULL,
    current_period_end TIMESTAMPTZ NOT NULL,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_user_status
    ON subscriptions(user_id, status);

CREATE TABLE IF NOT EXISTS invoices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    subscription_id UUID REFERENCES subscriptions(id) ON DELETE SET NULL,
    provider_invoice_id TEXT,
    amount_due_cents INTEGER NOT NULL CHECK (amount_due_cents >= 0),
    amount_paid_cents INTEGER NOT NULL DEFAULT 0 CHECK (amount_paid_cents >= 0),
    currency TEXT NOT NULL DEFAULT 'USD',
    status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'open', 'paid', 'void', 'uncollectible')),
    hosted_invoice_url TEXT,
    due_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS usage_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    plan_id TEXT NOT NULL REFERENCES billing_plans(id),
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    cloud_conversions_limit INTEGER NOT NULL,
    storage_bytes_limit BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, period_start, period_end)
);

CREATE TABLE IF NOT EXISTS usage_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    usage_period_id UUID NOT NULL REFERENCES usage_periods(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL CHECK (event_type IN ('cloud_conversion', 'storage_bytes')),
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    source_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_usage_events_user_period
    ON usage_events(user_id, usage_period_id, event_type);

CREATE TABLE IF NOT EXISTS commercial_entitlements (
    user_id UUID PRIMARY KEY REFERENCES app_users(id) ON DELETE CASCADE,
    count_balance BIGINT NOT NULL DEFAULT 0 CHECK (count_balance >= 0),
    valid_until TIMESTAMPTZ,
    source_order_id TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS redeem_packages (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    package_type TEXT NOT NULL CHECK (package_type IN ('count', 'date')),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    currency TEXT NOT NULL DEFAULT 'CNY',
    suggested_amount_cents INTEGER NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS recharges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    recharge_type TEXT NOT NULL CHECK (recharge_type IN ('count', 'date')),
    package_id TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    amount_cents INTEGER NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'CNY',
    status TEXT NOT NULL CHECK (status IN ('pending_manual', 'paid', 'refunded', 'voided')),
    provider TEXT NOT NULL,
    provider_trade_id TEXT NOT NULL UNIQUE,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE recharges
    DROP CONSTRAINT IF EXISTS recharges_status_check;

UPDATE recharges
SET status = 'paid'
WHERE status = 'paid_mock';

ALTER TABLE recharges
    ADD CONSTRAINT recharges_status_check
    CHECK (status IN ('pending_manual', 'paid', 'refunded', 'voided'));

CREATE INDEX IF NOT EXISTS idx_recharges_user_created
    ON recharges(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS manual_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    recharge_id UUID REFERENCES recharges(id) ON DELETE SET NULL,
    recharge_type TEXT NOT NULL CHECK (recharge_type IN ('count', 'date')),
    package_id TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    amount_cents INTEGER NOT NULL DEFAULT 0 CHECK (amount_cents >= 0),
    currency TEXT NOT NULL DEFAULT 'CNY',
    status TEXT NOT NULL DEFAULT 'paid'
        CHECK (status IN ('pending', 'paid', 'refunded', 'voided')),
    operator_id UUID REFERENCES app_users(id) ON DELETE SET NULL,
    payment_note TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_manual_orders_user_created
    ON manual_orders(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_manual_orders_status_created
    ON manual_orders(status, created_at DESC);

CREATE TABLE IF NOT EXISTS redeem_code_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_no TEXT NOT NULL UNIQUE,
    package_id TEXT NOT NULL REFERENCES redeem_packages(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    generated_count INTEGER NOT NULL DEFAULT 0,
    exported_count INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'paused', 'voided', 'exhausted')),
    channel TEXT,
    note TEXT,
    expires_at TIMESTAMPTZ,
    created_by UUID REFERENCES app_users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS redeem_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id UUID NOT NULL REFERENCES redeem_code_batches(id) ON DELETE CASCADE,
    package_id TEXT NOT NULL REFERENCES redeem_packages(id),
    code_hash TEXT NOT NULL UNIQUE,
    code_ciphertext BYTEA NOT NULL,
    code_nonce BYTEA NOT NULL,
    code_preview TEXT NOT NULL,
    key_version TEXT NOT NULL DEFAULT 'v1',
    status TEXT NOT NULL DEFAULT 'unused'
        CHECK (status IN ('unused', 'redeemed', 'voided', 'expired')),
    redeemed_by UUID REFERENCES app_users(id) ON DELETE SET NULL,
    redeemed_recharge_id UUID REFERENCES recharges(id) ON DELETE SET NULL,
    redeemed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_redeem_codes_batch_status
    ON redeem_codes(batch_id, status);

CREATE INDEX IF NOT EXISTS idx_redeem_codes_redeemed_by
    ON redeem_codes(redeemed_by, redeemed_at DESC);

CREATE TABLE IF NOT EXISTS redeem_code_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    redeem_code_id UUID REFERENCES redeem_codes(id) ON DELETE SET NULL,
    user_id UUID REFERENCES app_users(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'generated', 'exported', 'redeem_success', 'redeem_failed',
        'voided', 'expired'
    )),
    ip_hash TEXT,
    user_agent TEXT,
    reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS uploads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    object_key TEXT NOT NULL,
    bytes BIGINT NOT NULL CHECK (bytes >= 0),
    sha256 TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'stored'
        CHECK (status IN ('stored', 'expired', 'deleted')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS conversion_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    upload_id UUID REFERENCES uploads(id) ON DELETE SET NULL,
    main_tex TEXT NOT NULL,
    profile TEXT NOT NULL DEFAULT 'auto',
    quality TEXT NOT NULL DEFAULT 'standard',
    engine TEXT NOT NULL DEFAULT 'semantic-engine',
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN (
            'queued', 'normalizing', 'detecting', 'analyzing',
            'compiling', 'rendering', 'verifying',
            'completed', 'failed', 'expired'
        )),
    result_docx_key TEXT,
    result_report_key TEXT,
    report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    source_zip_key TEXT,
    result_log_key TEXT,
    storage_path TEXT,
    zip_bytes BIGINT,
    docx_bytes BIGINT,
    log_bytes BIGINT,
    worker_id TEXT,
    locked_at TIMESTAMPTZ,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    queued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    idempotency_key TEXT,
    engine_version TEXT DEFAULT '1.0.0',
    profile_version TEXT,
    last_error_code TEXT,
    error_code TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

ALTER TABLE conversion_jobs
    ADD COLUMN IF NOT EXISTS result_docx_key TEXT,
    ADD COLUMN IF NOT EXISTS result_report_key TEXT,
    ADD COLUMN IF NOT EXISTS report_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS source_zip_key TEXT,
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
    ADD COLUMN IF NOT EXISTS failed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS idempotency_key TEXT,
    ADD COLUMN IF NOT EXISTS engine_version TEXT DEFAULT '1.0.0',
    ADD COLUMN IF NOT EXISTS profile_version TEXT,
    ADD COLUMN IF NOT EXISTS last_error_code TEXT;

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_user_created
    ON conversion_jobs(user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_queue
    ON conversion_jobs(status, next_run_at, created_at)
    WHERE status IN ('queued', 'normalizing', 'detecting', 'analyzing', 'compiling', 'rendering', 'verifying');

CREATE TABLE IF NOT EXISTS usage_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    conversion_job_id UUID REFERENCES conversion_jobs(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'reserve', 'commit', 'refund', 'grant', 'adjust'
    )),
    quantity BIGINT NOT NULL,
    balance_after BIGINT,
    source TEXT NOT NULL DEFAULT 'system',
    reason TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_usage_ledger_job_event_unique
    ON usage_ledger(conversion_job_id, event_type)
    WHERE conversion_job_id IS NOT NULL
      AND event_type IN ('reserve', 'commit', 'refund');

CREATE INDEX IF NOT EXISTS idx_usage_ledger_user_created
    ON usage_ledger(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_usage_ledger_job
    ON usage_ledger(conversion_job_id);

CREATE TABLE IF NOT EXISTS release_manifests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel TEXT NOT NULL CHECK (channel IN ('stable', 'beta', 'dev')),
    platform TEXT NOT NULL CHECK (platform IN ('windows', 'macos', 'linux', 'web', 'android', 'ios')),
    arch TEXT NOT NULL DEFAULT 'x64',
    version TEXT NOT NULL,
    download_url TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    signature TEXT NOT NULL,
    signature_algorithm TEXT NOT NULL DEFAULT 'sha256',
    file_size_bytes BIGINT NOT NULL DEFAULT 0,
    release_title TEXT NOT NULL DEFAULT '',
    release_notes TEXT NOT NULL DEFAULT '',
    published_by TEXT NOT NULL DEFAULT 'system',
    is_prerelease BOOLEAN NOT NULL DEFAULT false,
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    active BOOLEAN NOT NULL DEFAULT true,
    deprecated_at TIMESTAMPTZ,
    deprecated_by TEXT,
    deprecation_reason TEXT,
    UNIQUE (channel, platform, arch, version)
);

ALTER TABLE release_manifests
    DROP CONSTRAINT IF EXISTS release_manifests_platform_check;

ALTER TABLE release_manifests
    ADD CONSTRAINT release_manifests_platform_check
    CHECK (platform IN ('windows', 'macos', 'linux', 'web', 'android', 'ios'));

ALTER TABLE release_manifests
    ADD COLUMN IF NOT EXISTS signature_algorithm TEXT NOT NULL DEFAULT 'sha256',
    ADD COLUMN IF NOT EXISTS file_size_bytes BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS release_title TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS published_by TEXT NOT NULL DEFAULT 'system',
    ADD COLUMN IF NOT EXISTS is_prerelease BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS deprecated_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deprecated_by TEXT,
    ADD COLUMN IF NOT EXISTS deprecation_reason TEXT;

CREATE TABLE IF NOT EXISTS release_strategies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    release_id UUID NOT NULL REFERENCES release_manifests(id) ON DELETE CASCADE,
    strategy_type TEXT NOT NULL DEFAULT 'recommended'
        CHECK (strategy_type IN ('optional', 'recommended', 'force', 'grayroll')),
    min_required_version TEXT,
    force_deadline_at TIMESTAMPTZ,
    block_if_outdated BOOLEAN NOT NULL DEFAULT false,
    rollout_percentage INTEGER NOT NULL DEFAULT 100 CHECK (rollout_percentage BETWEEN 0 AND 100),
    prompt_title TEXT NOT NULL DEFAULT '',
    prompt_message TEXT NOT NULL DEFAULT '',
    prompt_dismissable BOOLEAN NOT NULL DEFAULT true,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by TEXT NOT NULL DEFAULT 'system',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (release_id, strategy_type)
);

CREATE TABLE IF NOT EXISTS release_rollout_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    release_id UUID NOT NULL REFERENCES release_manifests(id) ON DELETE CASCADE,
    strategy_id UUID REFERENCES release_strategies(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    previous_percentage INTEGER,
    new_percentage INTEGER,
    event_reason TEXT,
    triggered_by TEXT NOT NULL DEFAULT 'system',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS release_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    target_release_id UUID REFERENCES release_manifests(id) ON DELETE SET NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_release_audit_created
    ON release_audit_log(created_at DESC);

CREATE TABLE IF NOT EXISTS app_update_preferences (
    user_id TEXT NOT NULL DEFAULT '',
    device_id TEXT NOT NULL,
    auto_update BOOLEAN NOT NULL DEFAULT true,
    update_channel TEXT NOT NULL DEFAULT 'stable'
        CHECK (update_channel IN ('stable', 'beta', 'dev')),
    platform TEXT NOT NULL,
    app_version TEXT NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, device_id)
);

CREATE TABLE IF NOT EXISTS waitlist_leads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL,
    identity TEXT,
    paper_type TEXT,
    current_tool TEXT,
    pain_point TEXT,
    paid_intent TEXT,
    status TEXT NOT NULL DEFAULT 'new'
        CHECK (status IN ('new', 'contacted', 'invited', 'converted', 'closed')),
    follow_up_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_waitlist_leads_status_created
    ON waitlist_leads(status, created_at DESC);

INSERT INTO billing_plans (id, name, currency, price_cents, monthly_conversions, storage_bytes, features)
VALUES
    ('preview', 'Preview', 'USD', 0, 100, 1073741824, '["local-convert", "cloud-preview", "quality-report"]'::jsonb),
    ('pro', 'Pro', 'USD', 2900, 1000, 10737418240, '["priority-worker", "journal-profiles", "desktop-sync"]'::jsonb)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    currency = EXCLUDED.currency,
    price_cents = EXCLUDED.price_cents,
    monthly_conversions = EXCLUDED.monthly_conversions,
    storage_bytes = EXCLUDED.storage_bytes,
    features = EXCLUDED.features,
    active = true;

INSERT INTO redeem_packages (id, name, package_type, quantity, currency, suggested_amount_cents)
VALUES
    ('count_3', '3 次转换包', 'count', 3, 'CNY', 300),
    ('count_10', '10 次转换包', 'count', 10, 'CNY', 1000),
    ('count_30', '30 次转换包', 'count', 30, 'CNY', 3000)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    package_type = EXCLUDED.package_type,
    quantity = EXCLUDED.quantity,
    currency = EXCLUDED.currency,
    suggested_amount_cents = EXCLUDED.suggested_amount_cents,
    active = true;
