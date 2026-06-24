# Tex2Doc Flutter User

This release unit maps to the user mode in the current `flutter_app` project.

Current source path:

- `flutter_app/lib/main_user.dart`
- `flutter_app/lib/main.dart`
- `flutter_app/lib/user/user_app.dart`
- `flutter_app/lib/shared/workspace_app.dart`
- `flutter_app/lib/product/product_home_app.dart`

Build target:

```text
flutter build web --target lib/main_user.dart
```

Deployment target:

- `apps/rust-service/static/home`
- `apps/rust-service/static/user`

The user app must only call `/v1/*` user APIs. Admin pages and `/admin/v1/*` clients belong to the Flutter admin release unit.
