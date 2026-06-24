# Tex2Doc Rust Service

This release unit maps to the current `crates/server` package.

Responsibilities:

- HTTP API for user and admin clients.
- PostgreSQL persistence and schema initialization.
- Conversion worker queue and file storage.
- Static hosting for the product home, Flutter user app, and Flutter admin app.
- Invocation of the shared Rust conversion engine crates.

Current source path:

- `crates/server`

Static deployment root:

- `apps/rust-service/static/home`
- `apps/rust-service/static/user`
- `apps/rust-service/static/admin`

Runtime configuration:

- `TEX2DOC_STATIC_DIR` can override the static root. The default is `apps/rust-service/static`.
