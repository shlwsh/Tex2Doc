---
name: tex2doc-extension-e2e-testing
description: Use when validating or debugging Tex2Doc Chrome browser-extension end-to-end flows, especially registration, login/session refresh, local WASM conversion, cloud conversion, docx download, PostgreSQL/doc-server readiness, Playwright MV3 service-worker behavior, or scripts/test_mainflow.ps1 automation.
---

# Tex2Doc Extension E2E Testing

## Default Workflow

1. Prefer `scripts/test_mainflow.ps1` for full automation. It creates an isolated PostgreSQL database, starts an isolated `doc-server`, and validates Web/API, Slint, and Chrome extension flows.
2. For a faster Chrome-focused run, use:

```powershell
.\scripts\test_mainflow.ps1 -SkipSlint
```

3. For API/Slint-only diagnosis, skip Chrome:

```powershell
.\scripts\test_mainflow.ps1 -SkipChromeExtension
```

4. If the extension output is already freshly built and source did not change, add `-SkipExtensionBuild`.
5. Keep `-KeepDatabase` only when you need to inspect tables after a failure.

## Chrome Extension Coverage

The mainflow Chrome phase must cover:

- Open a real Chromium persistent context with `.output/chrome-mv3` loaded as an unpacked extension.
- Use an extension page such as `popup.html` to call `chrome.runtime.sendMessage`; do not send messages from the service worker to itself.
- Seed `tex2doc_settings` in both `chrome.storage.sync` and `chrome.storage.local` with the isolated API origin.
- Verify pre-login `REFRESH_SESSION` returns signed-out without service-worker console errors.
- Verify `REGISTER`, post-register `REFRESH_SESSION`, `LOGOUT`, `LOGIN`, and `FETCH_USAGE`.
- Run local conversion via `globalThis.__tex2docConvertZip` in the service worker and check DOCX `PK` magic bytes.
- Run cloud conversion via `CLOUD_CONVERT_AND_POLL`, poll `FETCH_JOBS` until completed, download DOCX through the API with the stored token, and verify balance decreases.
- Treat any service-worker `console.error`, page error, or web error as a failing signal.

## Environment Checks

Before chasing extension code, verify infrastructure:

```powershell
Invoke-RestMethod http://127.0.0.1:<port>/api/v1/health
psql.exe postgres://postgres:postgres@127.0.0.1:5432/postgres -Atc "select 1"
```

If health passes but auth/register hangs or returns pool timeout, PostgreSQL may be down while an old `doc-server` still answers `/health`. Restart PostgreSQL or use the isolated mainflow script.

## Debugging Reference

Read [references/chrome-extension-e2e-notes.md](references/chrome-extension-e2e-notes.md) when a run fails, when updating `scripts/test_mainflow.ps1`, or when diagnosing MV3 service-worker/session/storage issues.
