import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';

import 'user/user_app.dart';

void main() {
  runApp(UserApp(isWeb: kIsWeb));
}
