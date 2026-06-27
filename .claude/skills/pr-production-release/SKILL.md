---
name: pr-production-release
description: Use when Codex needs to guide or implement Tex2Doc's path from PR readiness through GitHub CI, merge to main, production deployment, release package generation, post-deploy verification, rollback, and future automated release-flow improvements.
---

# PR Production Release

## Purpose

Use this skill to operate or automate the full Tex2Doc release path: prepare a PR, pass GitHub checks, merge safely, deploy production from `main`, verify the live service, and define rollback or follow-up actions.

## Release Path

1. **Pre-PR local gate**
   - Use `$precommit-ci-review` before committing or pushing.
   - Required local commands:
     - `cargo fmt --all -- --check`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `flutter pub get` from `flutter_app`
     - `flutter analyze` from `flutter_app`
   - Stronger local gate:
     - `npm run ci:preflight`
     - Record whether `DATABASE_URL` was absent and API integration tests were skipped.
   - Run GitNexus `detect_changes({scope: "all"})` before committing; compare against `main` for regression review when needed.
2. **PR creation and review**
   - Push a branch and open a PR into `main`.
   - Confirm GitHub Actions `CI` passes:
     - Rust matrix on `ubuntu-latest` and `windows-latest`.
     - Flutter web/client checks on `ubuntu-latest`.
   - Treat `RUSTFLAGS=-D warnings` as a hard gate.
   - Do not merge with unresolved HIGH/CRITICAL GitNexus risk unless the risk is explicitly accepted and documented.
3. **Merge to main**
   - Merge only after PR CI is green.
   - `push` to `main` triggers `.github/workflows/deploy-production.yml`.
   - Expect the production deployment workflow to build on `ubuntu-22.04` to match production glibc 2.35.
4. **Production build**
   - Build `doc-server` with `cargo build -p doc-server --release`.
   - Build Flutter web entries:
     - Home: `flutter build web --release --target lib/main.dart --base-href /`
     - User: `flutter build web --release --target lib/main_user.dart --base-href /user/`
     - Admin: `flutter build web --release --target lib/main_admin.dart --base-href /admin/`
   - Stage bundle shape:
     - `server/doc-server`
     - `static/home/`
     - `static/user/`
     - `static/admin/`
   - Upload artifact `tex2doc-production`.
5. **Production deployment**
   - GitHub environment: `production`.
   - Required secrets:
     - `PROD_SSH_HOST`
     - `PROD_SSH_USER`
     - `PROD_SSH_KEY`
     - Optional `PROD_SSH_PORT`, default `22`
     - Optional `PROD_DEPLOY_DIR`, default `/opt/tex2doc`
   - Deployment uploads `/tmp/tex2doc-production.tar.gz`, extracts to `$DEPLOY_DIR/releases/<timestamp>`, and updates `$DEPLOY_DIR/current`.
   - Deployment restarts `tex2doc-server`, validates nginx, reloads nginx, checks `http://127.0.0.1:2624/api/v1/health`, removes the temporary bundle, and keeps the latest five releases.
6. **Post-deploy verification**
   - From server:
     - `systemctl status tex2doc-server --no-pager`
     - `curl -fsS http://127.0.0.1:2624/api/v1/health`
     - `curl -I http://127.0.0.1/`
     - `curl -I http://127.0.0.1/user/`
     - `curl -I http://127.0.0.1/admin/`
   - From external network, replace host with the configured domain or production IP:
     - `curl -I http://<prod-host>/`
     - `curl -fsS http://<prod-host>/api/v1/health`
   - Verify browser routes:
     - `/`
     - `/user/`
     - `/admin/`
     - `/api/v1/health`
7. **Release packages**
   - `.github/workflows/release-packages.yml` runs on `workflow_dispatch` or tags matching `v*`.
   - Native packages:
     - Windows: `tex2doc-windows-x64.zip`
     - Linux: `tex2doc-linux-x64.tar.gz`
   - Web static bundle:
     - `tex2doc-web-static.tar.gz`
   - Use tags only after production or release-candidate verification is complete.

## Rollback

Use rollback when post-deploy health checks, browser checks, or business smoke tests fail.

1. SSH to production.
2. List releases:
   - `ls -1dt /opt/tex2doc/releases/*`
3. Point `current` to a known-good release:
   - `ln -sfn /opt/tex2doc/releases/<release-id> /opt/tex2doc/current`
4. Restart and reload:
   - `sudo systemctl restart tex2doc-server`
   - `sudo nginx -t`
   - `sudo systemctl reload nginx`
5. Re-run health and route checks.

## Automation Design Notes

- Keep PR CI fast: formatting, clippy, and Flutter analyze are the required GitHub checks.
- Keep full regression checks in `npm run ci:preflight` until CI time and external dependencies are stable.
- Preserve the `ubuntu-22.04` production build runner while the server runs Ubuntu 22.04.
- Keep macOS release jobs optional until runner queue delays no longer block delivery.
- Add GitHub `production` environment required reviewers for safer automatic deployment from `main`.
- Never commit server passwords, private keys, database passwords, or production `.env` files.
- Prefer a release manifest in future automation with:
  - commit SHA
  - branch/tag
  - build workflow run URL
  - artifact checksum
  - release directory
  - deployment timestamp
  - health-check result
  - rollback target
- Future deploy automation should expose a single go/no-go summary:
  - PR CI result
  - preflight result
  - deploy workflow result
  - production health result
  - browser route result
  - rollback readiness

## Failure Modes

- PR CI fails on formatting drift or clippy warnings promoted by `-D warnings`.
- Windows and Linux Rust checks can disagree; wait for both matrix jobs.
- Production binary can fail on glibc mismatch if built on a newer runner than production.
- Secrets missing or malformed prevent SSH setup.
- `sudo` password prompts in production break non-interactive deploys; server sudoers must allow the limited systemctl/nginx commands.
- Nginx route config can make `/user/` or `/admin/` fail while `/` still works.
- Health checks may pass locally on `127.0.0.1:2624` while external nginx routing is broken; verify both.
- Database migrations/schema initialization can pass build but fail runtime if production env vars or database permissions are wrong.

## Report Template

```markdown
**发布结论**
可以发布 / 暂停发布 / 已回滚。

**PR 与 CI**
- PR：
- CI：
- 本地 preflight：
- GitNexus 风险：

**部署**
- 触发方式：main push / workflow_dispatch / tag
- Workflow：
- Artifact：
- Release dir：

**生产验证**
- API health：
- `/`：
- `/user/`：
- `/admin/`：
- 外网访问：

**回滚准备**
- 当前版本：
- 上一个可回滚版本：
- 回滚命令是否可执行：

**后续自动化**
- 需要固化的检查：
- 需要补充的 secrets/environment：
- 需要新增的发布报告或通知：
```

