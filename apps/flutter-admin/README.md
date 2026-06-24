# Tex2Doc Flutter Admin

This release unit maps to the admin mode in the current `flutter_app` project.

Current source path:

- `flutter_app/lib/main_admin.dart`
- `flutter_app/lib/admin/admin_app.dart`
- `flutter_app/lib/admin/pages/*`
- `flutter_app/lib/shared/workspace_app.dart`

Build target:

```text
flutter build web --target lib/main_admin.dart
```

Deployment target:

- `apps/rust-service/static/admin`

The admin app must call `/admin/v1/*` for management APIs and validate administrator identity through `/admin/v1/me`.
