-- =============================================================================
-- 002_redeem_codes_stock_status.sql
-- -----------------------------------------------------------------------------
-- Adds independent three-state lifecycle tracking to `redeem_codes`:
--   - `stock_status`         : new | stocked | redeemed | restocked
--   - `stocked_at`           : timestamp the code was marked "stocked" (上货)
--   - `redeemed_at` (kept)   : timestamp the code was redeemed (使用)
--   - `restocked_at`         : timestamp the code was reset back to "new"
--
-- Existing `status` column is preserved so legacy reads still work, and a
-- trigger keeps `status` in sync as a derived value of the new columns.
-- =============================================================================

ALTER TABLE redeem_codes
    ADD COLUMN IF NOT EXISTS stock_status TEXT NOT NULL DEFAULT 'new'
        CHECK (stock_status IN ('new', 'stocked', 'redeemed', 'restocked')),
    ADD COLUMN IF NOT EXISTS stocked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS restocked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS stocked_by UUID REFERENCES app_users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS restocked_by UUID REFERENCES app_users(id) ON DELETE SET NULL;

-- Backfill: legacy 'unused' / 'redeemed' rows map cleanly into stock_status.
UPDATE redeem_codes
   SET stock_status = CASE
       WHEN status = 'redeemed' THEN 'redeemed'
       WHEN status IN ('voided', 'expired') THEN 'new'
       ELSE 'new'
   END
 WHERE stock_status = 'new'
   AND (stocked_at IS NOT NULL OR redeemed_at IS NOT NULL OR status <> 'unused');

CREATE INDEX IF NOT EXISTS idx_redeem_codes_stock_status
    ON redeem_codes(stock_status, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_redeem_codes_stocked_at
    ON redeem_codes(stocked_at DESC)
    WHERE stocked_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_redeem_codes_restocked_at
    ON redeem_codes(restocked_at DESC)
    WHERE restocked_at IS NOT NULL;

-- Expand redeem_code_events enum to cover new lifecycle transitions.
ALTER TABLE redeem_code_events
    DROP CONSTRAINT IF EXISTS redeem_code_events_event_type_check;

ALTER TABLE redeem_code_events
    ADD CONSTRAINT redeem_code_events_event_type_check
        CHECK (event_type IN (
            'generated', 'exported', 'redeem_success', 'redeem_failed',
            'voided', 'expired', 'stocked', 'restocked'
        ));