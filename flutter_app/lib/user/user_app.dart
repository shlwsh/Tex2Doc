import 'package:flutter/material.dart';

import '../shared/workspace_app.dart';

class UserApp extends StatelessWidget {
  final bool isWeb;

  const UserApp({super.key, required this.isWeb});

  @override
  Widget build(BuildContext context) {
    return DocEngineApp(isWeb: isWeb, mode: DocEngineAppMode.user);
  }
}
