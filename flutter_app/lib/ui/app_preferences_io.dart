import 'dart:convert';
import 'dart:io';

Future<String?> readPreference(String key) async {
  final values = await _readAll();
  return values[key];
}

Future<void> writePreference(String key, String value) async {
  final values = await _readAll();
  values[key] = value;
  final file = await _preferenceFile();
  await file.parent.create(recursive: true);
  await file.writeAsString(jsonEncode(values));
}

Future<Map<String, String>> _readAll() async {
  final file = await _preferenceFile();
  if (!await file.exists()) return <String, String>{};
  try {
    final decoded =
        jsonDecode(await file.readAsString()) as Map<String, dynamic>;
    return decoded.map((key, value) => MapEntry(key, value.toString()));
  } on Object {
    return <String, String>{};
  }
}

Future<File> _preferenceFile() async {
  final env = Platform.environment;
  final base = env['APPDATA'] ?? env['HOME'] ?? env['USERPROFILE'] ?? '.';
  return File('$base/.tex2doc/flutter-ui-preferences.json');
}
