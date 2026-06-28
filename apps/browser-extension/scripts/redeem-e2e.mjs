#!/usr/bin/env node
/**
 * redeem-e2e.mjs — Redeem-code 自动账户联调脚本
 *
 * 对应 P0-1：未登录用户 → 输入兑换码 → 自动获得账户 + 会话。
 *
 * 流程：
 *   1. 用已 bootstrap 的管理员登录拿 token
 *   2. 用管理员 token 创建一个 auto_provision=true 的批次，拿其中一个 code
 *   3. 匿名 POST /api/v1/redeem-codes/redeem  → 期望返回 access/refresh/user/is_new_account=true
 *   4. 用拿到的 access_token 调 /api/v1/usage  → 期望 200 + 余额 ≥ 10
 *   5. 再匿名调同一个 code  → 期望 409 code_already_redeemed
 *   6. 用第一次拿到的 token 调一个新 code  → 期望 200 + is_new_account=false
 *   7. 创建 auto_provision=false 批次 → 匿名兑换 → 期望 401 redeem_requires_login
 *
 * 环境变量：
 *   TEX2DOC_BASE_URL              (default: http://127.0.0.1:2624)
 *   TEX2DOC_ADMIN_EMAIL           (default: p01-admin@example.com)
 *   TEX2DOC_ADMIN_PASSWORD        (default: p01-admin-pass-123456)
 */

const BASE = process.env.TEX2DOC_BASE_URL ?? 'http://127.0.0.1:2624';
const ADMIN_EMAIL = process.env.TEX2DOC_ADMIN_EMAIL ?? 'p01-admin@example.com';
const ADMIN_PASSWORD = process.env.TEX2DOC_ADMIN_PASSWORD ?? 'p01-admin-pass-123456';

const log = (...args) => console.log('[redeem-e2e]', ...args);
const fail = (msg, detail) => {
  console.error('[redeem-e2e] FAIL:', msg);
  if (detail !== undefined) console.error(JSON.stringify(detail, null, 2));
  process.exitCode = 1;
};
const ok = (msg) => console.log('[redeem-e2e] PASS:', msg);
const assert = (cond, msg, detail) => {
  if (!cond) fail(msg, detail);
  else ok(msg);
};

async function http(method, path, { body, token } = {}) {
  const headers = { 'content-type': 'application/json' };
  if (token) headers.authorization = `Bearer ${token}`;
  const res = await fetch(`${BASE}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let json;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = text;
  }
  return { status: res.status, body: json };
}

async function waitForServer(maxAttempts = 30) {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const res = await http('GET', '/api/v1/health');
      if (res.status === 200) return;
    } catch {
      /* server not up yet */
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  fail(`server at ${BASE} did not become healthy after ${maxAttempts}s`);
  throw new Error('server-unreachable');
}

async function ensureAdmin() {
  const login = await http('POST', '/api/v1/auth/login', {
    body: { email: ADMIN_EMAIL, password: ADMIN_PASSWORD },
  });
  if (login.status === 200 && login.body?.access_token) {
    return { id: login.body.user.id, token: login.body.access_token };
  }
  fail('could not acquire admin session', { login });
  throw new Error('admin-unavailable');
}

async function createBatch(token, opts) {
  const res = await http('POST', '/admin/v1/redeem-code-batches', {
    token,
    body: {
      package_id: 'count_10',
      quantity: opts?.quantity ?? 2,
      channel: 'e2e',
      note: `redeem-e2e ${new Date().toISOString()}`,
      auto_provision: !!opts?.auto_provision,
    },
  });
  if (res.status !== 200) {
    fail(`admin create batch failed (status=${res.status})`, res.body);
    throw new Error('batch-create-failed');
  }
  const code = res.body.codes?.[0];
  if (!code) {
    fail('batch returned no codes', res.body);
    throw new Error('batch-empty');
  }
  return { batch: res.body, code };
}

async function main() {
  await waitForServer();
  log(`base url: ${BASE}`);

  const admin = await ensureAdmin();
  ok(`admin session acquired (${admin.id})`);

  const { code: anonCode } = await createBatch(admin.token, { quantity: 1, auto_provision: true });
  log(`created auto_provision batch with code prefix: ${anonCode.slice(0, 8)}…`);

  const redeem1 = await http('POST', '/api/v1/redeem-codes/redeem', {
    body: { code: anonCode },
  });
  assert(redeem1.status === 200, `anonymous redeem returns 200 (got ${redeem1.status})`, redeem1.body);
  assert(
    typeof redeem1.body?.access_token === 'string',
    'response includes access_token',
    redeem1.body,
  );
  assert(
    typeof redeem1.body?.refresh_token === 'string',
    'response includes refresh_token',
    redeem1.body,
  );
  assert(
    !!redeem1.body?.user?.id,
    'response includes user.id',
    redeem1.body,
  );
  assert(
    redeem1.body?.is_new_account === true,
    'is_new_account === true on first redeem',
    redeem1.body,
  );

  const usage = await http('GET', '/api/v1/usage', { token: redeem1.body.access_token });
  assert(usage.status === 200, 'authed /usage returns 200', usage.body);
  assert(
    Number(usage.body?.count_balance ?? 0) >= 10,
    'count_balance >= 10 after redeem of count_10',
    usage.body,
  );

  const replay = await http('POST', '/api/v1/redeem-codes/redeem', {
    body: { code: anonCode },
  });
  log(`second redeem of same code: status=${replay.status} body=${JSON.stringify(replay.body)}`);
  assert(
    replay.status === 409 && replay.body?.error === 'code_already_redeemed',
    'second redeem of same code is rejected as already_redeemed',
    replay.body,
  );

  const { code: anonCode2 } = await createBatch(admin.token, { quantity: 1, auto_provision: true });
  const replay2 = await http('POST', '/api/v1/redeem-codes/redeem', {
    token: redeem1.body.access_token,
    body: { code: anonCode2 },
  });
  assert(
    replay2.status === 200 && replay2.body?.is_new_account === false,
    'reusing existing token on a new code returns is_new_account=false',
    replay2.body,
  );

  const { code: legacyCode } = await createBatch(admin.token, {
    quantity: 1,
    auto_provision: false,
  });
  const legacyAnon = await http('POST', '/api/v1/redeem-codes/redeem', {
    body: { code: legacyCode },
  });
  log(`legacy anon redeem: status=${legacyAnon.status} body=${JSON.stringify(legacyAnon.body)}`);
  assert(
    legacyAnon.status === 401 && legacyAnon.body?.error === 'redeem_requires_login',
    'auto_provision=false batch returns 401 redeem_requires_login for anonymous',
    legacyAnon.body,
  );

  if (process.exitCode) {
    log('one or more checks failed');
  } else {
    log('all checks passed');
  }
}

main().catch((error) => {
  if (error?.message !== 'server-unreachable' && error?.message !== 'admin-unavailable') {
    console.error('[redeem-e2e] uncaught:', error);
  }
  process.exit(process.exitCode ?? 1);
});
