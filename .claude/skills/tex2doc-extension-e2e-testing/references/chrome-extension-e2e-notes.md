# Chrome Extension E2E Notes

## Known Failure Modes

- `localStorage is not defined` in `background.js`: MV3 service workers do not have `window.localStorage`. Use `browser.storage.local` / `chrome.storage.local` from service-worker code.
- `Could not establish connection. Receiving end does not exist.` when testing: calling `chrome.runtime.sendMessage` from inside the service worker does not reliably dispatch back to its own `onMessage`. Open an extension page (`chrome-extension://<id>/popup.html`) and send messages from there.
- `No refresh token available` before login: this should be a signed-out state, not a fatal background error. Test pre-login `REFRESH_SESSION` explicitly.
- `signal is aborted without reason` during register/login can be a frontend timeout symptom. Confirm direct API behavior; if direct `/v1/auth/register` returns `database error: pool timed out while waiting for an open connection`, fix PostgreSQL/doc-server state first.
- `/api/v1/health` returning `ok` does not prove the database pool is healthy. Always probe auth/register or `psql` when database symptoms appear.
- Building `.output/chrome-mv3` with a temporary `VITE_API_BASE_URL` can leave test ports in generated files. Prefer seeding extension settings at runtime; if a temporary build is necessary, rebuild back to the intended default afterward.
- New accounts may receive signup bonus balance. Mainflow tests should assert balance deltas (`+10` after redeem, `-1` after conversion) instead of hardcoding an initial `0`.
- Desktop/Slint cloud preflight must treat either plan quota or `count_balance` as usable entitlement. A user can have `cloud_conversions_limit=0` and positive `count_balance` from signup or redeem codes.

## Reliable Playwright Pattern

1. Build extension output after source changes:

```powershell
npm run build:chrome --prefix apps\browser-extension
```

2. Launch a persistent Chromium context with:

```js
await chromium.launchPersistentContext(userDataDir, {
  channel: 'chromium',
  headless: true,
  args: [
    `--disable-extensions-except=${extPath}`,
    `--load-extension=${extPath}`,
    '--no-sandbox',
  ],
});
```

3. Open `https://example.com` first to wake extension machinery, then get the service worker:

```js
const sw = context.serviceWorkers()[0] ?? await context.waitForEvent('serviceworker');
const extensionId = new URL(sw.url()).host;
```

4. Seed settings from the service worker:

```js
await sw.evaluate(async (base) => {
  const settings = {
    api_base_url: base,
    default_profile: 'standard',
    default_quality: 'balanced',
    default_mode: 'cloud',
    wasm_file_size_limit: 10 * 1024 * 1024,
    language: 'en',
    theme: 'system',
    polling_interval: 500,
  };
  await chrome.storage.sync.set({ tex2doc_settings: settings });
  await chrome.storage.local.set({ tex2doc_settings: settings });
  await chrome.storage.local.remove('tex2doc_session');
}, apiBaseUrl);
```

5. Open `popup.html` and send runtime messages from that page:

```js
const page = await context.newPage();
await page.goto(`chrome-extension://${extensionId}/popup.html`);
const reply = await page.evaluate((msg) => chrome.runtime.sendMessage(msg), {
  type: 'REGISTER',
  email,
  password,
  displayName: 'Chrome Extension E2E',
});
```

## Expected Assertions

- Registration succeeds and returned email matches the random test user.
- Post-registration refresh succeeds.
- Logout then login succeeds.
- Signup balance is read from the service response; tests should rely on deltas rather than assuming a fixed initial value.
- Local conversion returns DOCX bytes larger than 1000 and starting with `PK`.
- Cloud conversion reaches `completed`, has `cloudJobId`, downloadable DOCX starts with `PK\x03\x04`, and balance decreases.
- No service-worker `console.error`, page error, or web error is emitted.

## Mainflow Script Notes

- `scripts/test_mainflow.ps1` uses a temporary PostgreSQL database and drops it unless `-KeepDatabase` is set.
- Use `-SkipSlint` for quicker browser-extension regression checks.
- Use `-SkipChromeExtension` when diagnosing server or Slint paths only.
- Use `-SkipExtensionBuild` only when `.output/chrome-mv3` already includes current source changes.
- If the script fails before cleanup, check for a leftover `doc-server` process on the reported port and a temp database named `tex2doc_mainflow_*`.
