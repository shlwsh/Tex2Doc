-- Tex2Doc 支付与收款数据库增量（PostgreSQL，业务库 docdb）。
-- 依赖：docs-zh/money/001_docdb_business_schema.sql（app_users / billing_plans / invoices / usage_periods 已存在）。
-- 设计目标：支持支付宝、微信收款；先聚合后直连；以「一次性购买 + 次数包/Credits」为核心。
-- 金额单位：统一使用「最小货币单位」整数（CNY 为「分」），列名沿用既有 *_cents 习惯，currency 区分币种。
--
-- 本地执行：
--   psql -U postgres -d docdb -f docs-zh/pay/002_docdb_payment_schema.sql

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ---------------------------------------------------------------------------
-- 0. 既有套餐补充人民币定价（不破坏既有 USD 行，按 *_cny 后缀新增 CNY 套餐）
--    Preview 免费保持不变；新增 CNY 计价的 Pro 套餐，供国内收款使用。
-- ---------------------------------------------------------------------------
INSERT INTO billing_plans (id, name, currency, price_cents, monthly_conversions, storage_bytes, features)
VALUES
    ('pro_cny', 'Pro（人民币）', 'CNY', 19900, 1000, 10737418240,
     '["priority-worker", "journal-profiles", "desktop-sync"]'::jsonb)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    currency = EXCLUDED.currency,
    price_cents = EXCLUDED.price_cents,
    monthly_conversions = EXCLUDED.monthly_conversions,
    storage_bytes = EXCLUDED.storage_bytes,
    features = EXCLUDED.features,
    active = true;

-- ---------------------------------------------------------------------------
-- 1. 可购买商品目录（一次性购买）。
--    kind = 'credit_pack'   → 购买后向 credit_wallets 发放 grant_conversions 次云转换额度。
--    kind = 'plan_period'   → 购买后授予 grant_plan_id 套餐 grant_period_days 天（写 subscriptions）。
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_products (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('credit_pack', 'plan_period')),
    currency TEXT NOT NULL DEFAULT 'CNY',
    price_cents INTEGER NOT NULL CHECK (price_cents >= 0),
    grant_conversions INTEGER NOT NULL DEFAULT 0 CHECK (grant_conversions >= 0),
    grant_plan_id TEXT REFERENCES billing_plans(id),
    grant_period_days INTEGER NOT NULL DEFAULT 0 CHECK (grant_period_days >= 0),
    active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO payment_products (id, name, kind, currency, price_cents, grant_conversions, grant_plan_id, grant_period_days)
VALUES
    ('credits_100',  '云转换 100 次包',  'credit_pack',  'CNY',  990,  100, NULL, 0),
    ('credits_500',  '云转换 500 次包',  'credit_pack',  'CNY', 3900,  500, NULL, 0),
    ('pro_month_cny', 'Pro 月卡',         'plan_period',  'CNY', 19900,   0, 'pro_cny', 30),
    ('pro_year_cny',  'Pro 年卡',         'plan_period',  'CNY', 199900,  0, 'pro_cny', 365)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    kind = EXCLUDED.kind,
    currency = EXCLUDED.currency,
    price_cents = EXCLUDED.price_cents,
    grant_conversions = EXCLUDED.grant_conversions,
    grant_plan_id = EXCLUDED.grant_plan_id,
    grant_period_days = EXCLUDED.grant_period_days,
    active = true;

-- ---------------------------------------------------------------------------
-- 2. 支付订单（业务侧一笔购买，对应商户订单号 out_trade_no）。
--    状态机：created → pending（已下单/出二维码）→ paid → (refunding → refunded)
--                    └→ closed（超时关单） └→ failed
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    out_trade_no TEXT NOT NULL UNIQUE,
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    product_id TEXT NOT NULL REFERENCES payment_products(id),
    channel TEXT NOT NULL CHECK (channel IN ('alipay', 'wechat')),
    provider TEXT NOT NULL,
    pay_scene TEXT NOT NULL CHECK (pay_scene IN ('native', 'web', 'wap', 'app', 'jsapi')),
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    currency TEXT NOT NULL DEFAULT 'CNY',
    subject TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'created'
        CHECK (status IN ('created', 'pending', 'paid', 'closed', 'refunding', 'refunded', 'failed')),
    provider_order_id TEXT,
    qr_code TEXT,
    pay_url TEXT,
    expires_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ,
    client_ip TEXT,
    idempotency_key TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_payment_orders_user_created
    ON payment_orders(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_payment_orders_status
    ON payment_orders(status) WHERE status IN ('created', 'pending');
CREATE UNIQUE INDEX IF NOT EXISTS uq_payment_orders_idem
    ON payment_orders(user_id, idempotency_key) WHERE idempotency_key IS NOT NULL;

-- ---------------------------------------------------------------------------
-- 3. 支付流水（渠道真实支付结果，来自异步通知或主动查询）。
--    (provider, provider_transaction_id) 唯一，天然幂等去重。
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES payment_orders(id) ON DELETE CASCADE,
    channel TEXT NOT NULL CHECK (channel IN ('alipay', 'wechat')),
    provider TEXT NOT NULL,
    provider_transaction_id TEXT NOT NULL,
    amount_cents INTEGER NOT NULL CHECK (amount_cents >= 0),
    currency TEXT NOT NULL DEFAULT 'CNY',
    status TEXT NOT NULL CHECK (status IN ('success', 'failed')),
    buyer_ref TEXT,
    raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_transaction_id)
);

CREATE INDEX IF NOT EXISTS idx_payment_transactions_order
    ON payment_transactions(order_id);

-- ---------------------------------------------------------------------------
-- 4. 退款单。
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_refunds (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    out_refund_no TEXT NOT NULL UNIQUE,
    order_id UUID NOT NULL REFERENCES payment_orders(id) ON DELETE CASCADE,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    reason TEXT,
    status TEXT NOT NULL DEFAULT 'created'
        CHECK (status IN ('created', 'processing', 'succeeded', 'failed')),
    provider_refund_id TEXT,
    operator TEXT,
    raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_payment_refunds_order
    ON payment_refunds(order_id);

-- ---------------------------------------------------------------------------
-- 5. 异步通知日志（验签前先落库，便于幂等、重放、排障与对账）。
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_notify_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel TEXT NOT NULL,
    provider TEXT NOT NULL,
    event_type TEXT,
    out_trade_no TEXT,
    provider_transaction_id TEXT,
    signature_valid BOOLEAN,
    processed BOOLEAN NOT NULL DEFAULT false,
    process_note TEXT,
    http_headers JSONB NOT NULL DEFAULT '{}'::jsonb,
    raw_body TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_payment_notify_out_trade
    ON payment_notify_logs(out_trade_no);
CREATE INDEX IF NOT EXISTS idx_payment_notify_txn
    ON payment_notify_logs(provider, provider_transaction_id);

-- ---------------------------------------------------------------------------
-- 6. Credit 钱包与流水（次数包额度的当前余额 + append-only 账本）。
--    云转换扣额优先走订阅 usage_periods，余额不足再扣 credit_wallets；具体策略见技术方案文档。
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS credit_wallets (
    user_id UUID PRIMARY KEY REFERENCES app_users(id) ON DELETE CASCADE,
    balance_conversions INTEGER NOT NULL DEFAULT 0 CHECK (balance_conversions >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS credit_ledger (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    delta INTEGER NOT NULL,
    balance_after INTEGER NOT NULL CHECK (balance_after >= 0),
    reason TEXT NOT NULL CHECK (reason IN ('purchase', 'consume', 'refund', 'admin_adjust', 'expire')),
    ref_type TEXT,
    ref_id TEXT,
    idempotency_key TEXT UNIQUE,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_credit_ledger_user_created
    ON credit_ledger(user_id, created_at DESC);

-- ---------------------------------------------------------------------------
-- 7. 对账批次（与渠道账单逐笔比对结果）。
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_reconciliation_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel TEXT NOT NULL,
    provider TEXT NOT NULL,
    bill_date DATE NOT NULL,
    total_orders INTEGER NOT NULL DEFAULT 0,
    matched INTEGER NOT NULL DEFAULT 0,
    mismatched INTEGER NOT NULL DEFAULT 0,
    missing_local INTEGER NOT NULL DEFAULT 0,
    missing_remote INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'completed', 'failed')),
    report_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, bill_date)
);
