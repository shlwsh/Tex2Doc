import 'dart:convert';

import 'package:http/http.dart' as http;

class CommercialApiException implements Exception {
  final int statusCode;
  final String message;

  CommercialApiException(this.statusCode, this.message);

  @override
  String toString() => 'HTTP $statusCode: $message';
}

class CommercialApiClient {
  final Uri baseUri;
  final http.Client _http;

  CommercialApiClient(String baseUrl, {http.Client? httpClient})
    : baseUri = _normalizeBaseUrl(baseUrl),
      _http = httpClient ?? http.Client();

  Future<AuthResponse> register({
    required String email,
    required String password,
    String? displayName,
  }) async {
    return AuthResponse.fromJson(
      await _postJson('auth/register', {
        'email': email,
        'password': password,
        'display_name': displayName,
      }),
    );
  }

  Future<AuthResponse> login({
    required String email,
    required String password,
  }) async {
    return AuthResponse.fromJson(
      await _postJson('auth/login', {'email': email, 'password': password}),
    );
  }

  Future<UsageSummary> usage(String accessToken) async {
    return UsageSummary.fromJson(
      await _getJson('usage', accessToken: accessToken),
    );
  }

  Future<List<PlanSummary>> plans() async {
    final value = await _getJson('plans');
    return (value as List<dynamic>)
        .map((item) => PlanSummary.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<dynamic> _getJson(String path, {String? accessToken}) async {
    final response = await _http.get(
      baseUri.resolve(path),
      headers: _headers(accessToken: accessToken),
    );
    return _decode(response);
  }

  Future<Map<String, dynamic>> _postJson(
    String path,
    Map<String, dynamic> body, {
    String? accessToken,
  }) async {
    final response = await _http.post(
      baseUri.resolve(path),
      headers: _headers(accessToken: accessToken),
      body: jsonEncode(body),
    );
    final value = _decode(response);
    return value as Map<String, dynamic>;
  }

  Map<String, String> _headers({String? accessToken}) {
    return {
      'content-type': 'application/json',
      if (accessToken != null && accessToken.isNotEmpty)
        'authorization': 'Bearer $accessToken',
    };
  }

  dynamic _decode(http.Response response) {
    final text = utf8.decode(response.bodyBytes);
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CommercialApiException(response.statusCode, text);
    }
    if (text.isEmpty) {
      return <String, dynamic>{};
    }
    return jsonDecode(text);
  }

  static Uri _normalizeBaseUrl(String value) {
    final trimmed = value.trim().isEmpty
        ? 'http://127.0.0.1:8080/v1/'
        : value.trim();
    final withSlash = trimmed.endsWith('/') ? trimmed : '$trimmed/';
    return Uri.parse(withSlash);
  }
}

class AuthResponse {
  final String accessToken;
  final String refreshToken;
  final UserProfile user;

  AuthResponse({
    required this.accessToken,
    required this.refreshToken,
    required this.user,
  });

  factory AuthResponse.fromJson(Map<String, dynamic> json) {
    return AuthResponse(
      accessToken: json['access_token'] as String,
      refreshToken: json['refresh_token'] as String,
      user: UserProfile.fromJson(json['user'] as Map<String, dynamic>),
    );
  }
}

class UserProfile {
  final String id;
  final String email;
  final String? displayName;
  final String planId;

  UserProfile({
    required this.id,
    required this.email,
    required this.displayName,
    required this.planId,
  });

  factory UserProfile.fromJson(Map<String, dynamic> json) {
    return UserProfile(
      id: json['id'] as String,
      email: json['email'] as String,
      displayName: json['display_name'] as String?,
      planId: json['plan_id'] as String,
    );
  }
}

class UsageSummary {
  final String planId;
  final int cloudConversionsUsed;
  final int cloudConversionsLimit;
  final int storageBytesUsed;
  final int storageBytesLimit;

  UsageSummary({
    required this.planId,
    required this.cloudConversionsUsed,
    required this.cloudConversionsLimit,
    required this.storageBytesUsed,
    required this.storageBytesLimit,
  });

  int get cloudConversionsRemaining =>
      cloudConversionsLimit - cloudConversionsUsed;

  factory UsageSummary.fromJson(Map<String, dynamic> json) {
    return UsageSummary(
      planId: json['plan_id'] as String,
      cloudConversionsUsed: json['cloud_conversions_used'] as int,
      cloudConversionsLimit: json['cloud_conversions_limit'] as int,
      storageBytesUsed: json['storage_bytes_used'] as int,
      storageBytesLimit: json['storage_bytes_limit'] as int,
    );
  }
}

class PlanSummary {
  final String id;
  final String name;
  final int priceCents;
  final String currency;
  final int monthlyConversions;

  PlanSummary({
    required this.id,
    required this.name,
    required this.priceCents,
    required this.currency,
    required this.monthlyConversions,
  });

  factory PlanSummary.fromJson(Map<String, dynamic> json) {
    return PlanSummary(
      id: json['id'] as String,
      name: json['name'] as String,
      priceCents: json['price_cents'] as int,
      currency: json['currency'] as String,
      monthlyConversions: json['monthly_conversions'] as int,
    );
  }

  String get label =>
      '$id: $currency ${(priceCents / 100).toStringAsFixed(2)}/mo, $monthlyConversions conversions';
}
