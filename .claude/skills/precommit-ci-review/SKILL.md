---
name: precommit-ci-review
description: Use when Codex needs to internally review Tex2Doc code before committing, pushing, or opening/updating a PR, especially to catch GitHub Actions CI failures, GitHub review blockers, Rust/Flutter static-analysis issues, formatting drift, GitNexus impact risks, and accidental unrelated changes.
---

# Precommit CI Review

## Purpose

Run a local, CI-aligned review before a Tex2Doc commit or push. Treat this as an internal GitHub reviewer: find blockers first, verify with commands, keep unrelated user changes out of the commit, and produce a concise go/no-go result.

## Workflow

1. Inspect repository state:
   - Run `git status --short --branch`.
   - Run `git diff --stat` and `git diff --check`.
   - Identify unrelated or pre-existing changes; do not revert them.
2. Read CI entry points when the branch or workflow may have changed:
   - `.github/workflows/ci.yml`
   - `scripts/ci/preflight.mjs`
   - `package.json`
   - `Cargo.toml`
3. For symbol edits, follow project GitNexus rules:
   - Before editing a function, method, class, struct, or route handler, run `impact({target, direction: "upstream"})`.
   - Warn before editing when GitNexus reports HIGH or CRITICAL risk.
   - Before committing, run `detect_changes({scope: "all"})`; for regression review against default branch, use `detect_changes({scope: "compare", base_ref: "main"})`.
4. Run checks that mirror GitHub PR CI:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `flutter pub get` in `flutter_app`
   - `flutter analyze` in `flutter_app`
5. Run the stronger local gate when time allows or the user asks for thoroughness:
   - `npm run ci:preflight`
   - Note that API integration tests are skipped when `DATABASE_URL` is not set.
6. Review generated diffs:
   - Confirm behavior-changing edits are intentional.
   - Separate pure `cargo fmt` churn from logic changes.
   - Check that docs or metadata changes such as GitNexus symbol counts are intentional or clearly reported.
7. Report in Chinese by default for this project:
   - Blockers first.
   - Then validation commands and pass/fail status.
   - Then files changed and any residual risk.

## GitHub Review Failure Points To Catch

- Rust formatting drift: `cargo fmt --all -- --check` fails on reordered imports, wrapped chains, long headers, missing trailing newline, or rustfmt line wrapping.
- Clippy warnings as hard errors: CI sets `RUSTFLAGS=-D warnings` and runs `cargo clippy --workspace --all-targets -- -D warnings`.
- Common clippy blockers seen in this repo:
  - `needless_borrows_for_generic_args`, especially `.bind(&value)` when `.bind(value)` is enough.
  - `too_many_arguments`; prefer a small request/options struct when a public helper grows beyond the lint threshold.
  - `dead_code`; remove unused helpers or wire them into real behavior instead of silencing unless there is a clear compatibility reason.
- Flutter static analysis: `flutter analyze` must pass after `flutter pub get`.
- Dirty or mixed worktree: unrelated files can get included accidentally after automated formatting.
- GitNexus scope risk: formatting can mark many symbols as touched; distinguish this from behavior changes but still report HIGH/CRITICAL detect_changes results.
- CI/environment mismatch:
  - GitHub PR CI runs Ubuntu and Windows Rust checks.
  - Linux native dependencies are installed in workflow, but local Windows checks may not expose Linux-only build problems.
  - `DATABASE_URL` is optional in local preflight; absence means doc-server API integration tests are not run.
- Workflow drift: if `.github/workflows/*.yml` changes, re-read the workflow instead of relying on remembered commands.

## Recommended Fix Patterns

- For formatting-only failures, run `cargo fmt --all`, then re-run `cargo fmt --all -- --check`.
- For clippy `needless_borrows_for_generic_args`, apply the exact suggested borrow removal only when ownership/lifetime remains valid.
- For clippy `too_many_arguments`, introduce a request/options struct near the function, update all call sites, and keep field names explicit at the call site.
- For clippy `dead_code`, prefer deleting private unused helpers. If the item is part of an external contract, document why and use the narrowest allow/expect annotation.
- For Flutter analyze findings, fix source issues rather than editing generated files.
- For format churn, avoid hand-reformatting unrelated files; let formatter produce stable output and summarize it separately.

## Output Template

Use this shape for a precommit review result:

```markdown
**结论**
可以提交 / 暂不建议提交。

**阻断项**
- [P0/P1] 文件:行 - 问题和影响。

**已验证**
- `command` - 通过/失败/跳过原因。

**变更范围**
- 行为改动：
- 格式化/元数据改动：
- GitNexus detect_changes：风险等级、直接影响、受影响流程。

**剩余风险**
- 未运行的检查、环境差异、需要人工确认的点。
```

