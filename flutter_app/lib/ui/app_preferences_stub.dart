final Map<String, String> _memoryPreferences = <String, String>{};

Future<String?> readPreference(String key) async => _memoryPreferences[key];

Future<void> writePreference(String key, String value) async {
  _memoryPreferences[key] = value;
}
