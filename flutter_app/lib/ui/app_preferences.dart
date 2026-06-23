import 'app_preferences_stub.dart'
    if (dart.library.io) 'app_preferences_io.dart'
    if (dart.library.js_interop) 'app_preferences_web.dart'
    as platform;

class AppPreferences {
  static Future<String?> read(String key) => platform.readPreference(key);

  static Future<void> write(String key, String value) =>
      platform.writePreference(key, value);
}
