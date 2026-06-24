import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';

import 'workspace_app.dart';

void main() {
  runApp(DocEngineApp(isWeb: kIsWeb, mode: DocEngineAppMode.user));
}
