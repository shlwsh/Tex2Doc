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
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'locked', 'deleted')),
    default_plan_id TEXT NOT NULL DEFAULT 'preview',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

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
    error_code TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_conversion_jobs_user_created
    ON conversion_jobs(user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS release_manifests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel TEXT NOT NULL CHECK (channel IN ('stable', 'beta', 'dev')),
    platform TEXT NOT NULL CHECK (platform IN ('windows', 'macos', 'linux')),
    arch TEXT NOT NULL DEFAULT 'x64',
    version TEXT NOT NULL,
    download_url TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    signature TEXT NOT NULL,
    release_notes TEXT NOT NULL DEFAULT '',
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE (channel, platform, arch, version)
);

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
