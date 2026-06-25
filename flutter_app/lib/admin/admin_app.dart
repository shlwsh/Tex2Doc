import 'package:flutter/material.dart';

import '../shared/workspace_app.dart';

class AdminApp extends StatelessWidget {
  final bool isWeb;

  const AdminApp({super.key, required this.isWeb});

  @override
  Widget build(BuildContext context) {
    return DocEngineApp(isWeb: isWeb, mode: DocEngineAppMode.admin);
  }
}
