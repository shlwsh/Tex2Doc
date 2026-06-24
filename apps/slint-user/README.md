# Tex2Doc Slint User

This release unit contains the `doc-desktop-slint` package.

Responsibilities:

- Local desktop conversion workflow.
- Cloud conversion through user APIs.
- Account, billing, usage, feedback, history, and update checks.

Source path:

- `apps/slint-user`

Build target:

```text
cargo build -p doc-desktop-slint --release
```

The Slint user app must not call `/admin/v1/*` management APIs.
