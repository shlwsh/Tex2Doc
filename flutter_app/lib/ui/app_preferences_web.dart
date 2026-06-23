import 'package:web/web.dart' as web;

Future<String?> readPreference(String key) async {
  return web.window.localStorage.getItem(key);
}

Future<void> writePreference(String key, String value) async {
  web.window.localStorage.setItem(key, value);
}
