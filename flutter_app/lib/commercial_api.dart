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

  Future<RechargeOptions> rechargeOptions() async {
    return RechargeOptions.fromJson(
      await _getJson('recharge/options') as Map<String, dynamic>,
    );
  }

  Future<AuthResponse> refresh(String refreshToken) async {
    return AuthResponse.fromJson(
      await _postJson('auth/refresh', {'refresh_token': refreshToken}),
    );
  }

  Future<UserProfile> me(String accessToken) async {
    return UserProfile.fromJson(
      await _getJson('me', accessToken: accessToken) as Map<String, dynamic>,
    );
  }

  Future<BillingSession> checkout({
    required String accessToken,
    required String planId,
    required String successUrl,
    required String cancelUrl,
  }) async {
    return BillingSession.fromJson(
      await _postJson('billing/checkout', {
        'plan_id': planId,
        'success_url': successUrl,
        'cancel_url': cancelUrl,
      }, accessToken: accessToken),
    );
  }

  Future<BillingSession> portal({
    required String accessToken,
    required String returnUrl,
  }) async {
    return BillingSession.fromJson(
      await _postJson('billing/portal', {
        'return_url': returnUrl,
      }, accessToken: accessToken),
    );
  }

  Future<RechargeRecord> createRecharge({
    required String accessToken,
    required String rechargeType,
    required String packageId,
    int? quantity,
  }) async {
    final body = <String, dynamic>{
      'recharge_type': rechargeType,
      'package_id': packageId,
    };
    if (quantity != null) body['quantity'] = quantity;
    return RechargeRecord.fromJson(
      await _postJson('recharges', body, accessToken: accessToken),
    );
  }

  Future<List<RechargeRecord>> recharges(String accessToken) async {
    final value = await _getJson('recharges', accessToken: accessToken);
    return (value as List<dynamic>)
        .map((item) => RechargeRecord.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<UploadResponse> uploadProjectZip({
    required String accessToken,
    required List<int> bytes,
    String fileName = 'project.zip',
  }) async {
    final request = http.MultipartRequest('POST', baseUri.resolve('uploads'));
    request.headers.addAll(
      _headers(accessToken: accessToken)..remove('content-type'),
    );
    request.files.add(
      http.MultipartFile.fromBytes('file', bytes, filename: fileName),
    );
    final streamed = await _http.send(request);
    final response = await http.Response.fromStream(streamed);
    return UploadResponse.fromJson(_decode(response) as Map<String, dynamic>);
  }

  Future<ConversionJob> createConversion({
    required String accessToken,
    required String uploadId,
    required String mainTex,
    required String profile,
    required String quality,
  }) async {
    return ConversionJob.fromJson(
      await _postJson('conversions', {
        'upload_id': uploadId,
        'main_tex': mainTex,
        'profile': profile,
        'quality': quality,
      }, accessToken: accessToken),
    );
  }

  Future<ConversionJob> getConversion({
    required String accessToken,
    required String jobId,
  }) async {
    return ConversionJob.fromJson(
      await _getJson('conversions/$jobId', accessToken: accessToken)
          as Map<String, dynamic>,
    );
  }

  Future<List<ConversionJob>> conversions(String accessToken) async {
    final value = await _getJson('conversions', accessToken: accessToken);
    return (value as List<dynamic>)
        .map((item) => ConversionJob.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<List<int>> downloadConversionDocx({
    required String accessToken,
    required String jobId,
  }) async {
    final response = await _http.get(
      baseUri.resolve('conversions/$jobId/download/docx'),
      headers: _headers(accessToken: accessToken),
    );
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CommercialApiException(
        response.statusCode,
        utf8.decode(response.bodyBytes),
      );
    }
    return response.bodyBytes;
  }

  Future<ConversionReport> getConversionReport({
    required String accessToken,
    required String jobId,
  }) async {
    return ConversionReport.fromJson(
      await _getJson('conversions/$jobId/report', accessToken: accessToken)
          as Map<String, dynamic>,
    );
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
  final int countBalance;
  final String? dateValidUntil;
  final String? entitlementSourceOrderId;

  UsageSummary({
    required this.planId,
    required this.cloudConversionsUsed,
    required this.cloudConversionsLimit,
    required this.storageBytesUsed,
    required this.storageBytesLimit,
    required this.countBalance,
    required this.dateValidUntil,
    required this.entitlementSourceOrderId,
  });

  int get cloudConversionsRemaining =>
      cloudConversionsLimit - cloudConversionsUsed;

  String get entitlementLabel {
    final parts = <String>[];
    if (countBalance > 0) parts.add('$countBalance count credits');
    if (dateValidUntil != null && dateValidUntil!.isNotEmpty) {
      parts.add('valid until $dateValidUntil');
    }
    return parts.isEmpty ? 'preview quota' : parts.join(', ');
  }

  factory UsageSummary.fromJson(Map<String, dynamic> json) {
    return UsageSummary(
      planId: json['plan_id'] as String,
      cloudConversionsUsed: json['cloud_conversions_used'] as int,
      cloudConversionsLimit: json['cloud_conversions_limit'] as int,
      storageBytesUsed: json['storage_bytes_used'] as int,
      storageBytesLimit: json['storage_bytes_limit'] as int,
      countBalance: json['count_balance'] as int? ?? 0,
      dateValidUntil: json['date_valid_until'] as String?,
      entitlementSourceOrderId: json['entitlement_source_order_id'] as String?,
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

class RechargeOptions {
  final String currency;
  final String provider;
  final List<RechargePackage> countPackages;
  final List<RechargePackage> datePackages;

  RechargeOptions({
    required this.currency,
    required this.provider,
    required this.countPackages,
    required this.datePackages,
  });

  factory RechargeOptions.fromJson(Map<String, dynamic> json) {
    final count = json['count'] as Map<String, dynamic>? ?? const {};
    final date = json['date'] as Map<String, dynamic>? ?? const {};
    return RechargeOptions(
      currency: json['currency'] as String? ?? 'CNY',
      provider: json['provider'] as String? ?? 'mock-pay',
      countPackages: ((count['packages'] as List<dynamic>?) ?? const [])
          .map((item) => RechargePackage.fromJson(item as Map<String, dynamic>))
          .toList(growable: false),
      datePackages: ((date['packages'] as List<dynamic>?) ?? const [])
          .map((item) => RechargePackage.fromJson(item as Map<String, dynamic>))
          .toList(growable: false),
    );
  }
}

class RechargePackage {
  final String id;
  final String name;
  final int quantity;
  final int amountCents;

  RechargePackage({
    required this.id,
    required this.name,
    required this.quantity,
    required this.amountCents,
  });

  factory RechargePackage.fromJson(Map<String, dynamic> json) {
    return RechargePackage(
      id: json['id'] as String,
      name: json['name'] as String,
      quantity: (json['quantity'] ?? json['days'] ?? 0) as int,
      amountCents: json['amount_cents'] as int,
    );
  }

  String priceLabel(String currency) {
    final symbol = currency == 'CNY' ? '¥' : '$currency ';
    return '$name / $symbol${(amountCents / 100).toStringAsFixed(0)}';
  }
}

class RechargeRecord {
  final String rechargeId;
  final String rechargeType;
  final String packageId;
  final int quantity;
  final int amountCents;
  final String currency;
  final String status;
  final String provider;
  final String providerTradeId;
  final String createdAt;

  RechargeRecord({
    required this.rechargeId,
    required this.rechargeType,
    required this.packageId,
    required this.quantity,
    required this.amountCents,
    required this.currency,
    required this.status,
    required this.provider,
    required this.providerTradeId,
    required this.createdAt,
  });

  factory RechargeRecord.fromJson(Map<String, dynamic> json) {
    return RechargeRecord(
      rechargeId: json['recharge_id'] as String,
      rechargeType: json['recharge_type'] as String,
      packageId: json['package_id'] as String,
      quantity: json['quantity'] as int,
      amountCents: json['amount_cents'] as int,
      currency: json['currency'] as String,
      status: json['status'] as String,
      provider: json['provider'] as String,
      providerTradeId: json['provider_trade_id'] as String,
      createdAt: json['created_at'] as String,
    );
  }

  String get label =>
      '$packageId: $currency ${(amountCents / 100).toStringAsFixed(0)}, $status';
}

class BillingSession {
  final String url;
  final String expiresAt;

  BillingSession({required this.url, required this.expiresAt});

  factory BillingSession.fromJson(Map<String, dynamic> json) {
    return BillingSession(
      url: json['url'] as String,
      expiresAt: json['expires_at'] as String,
    );
  }
}

class UploadResponse {
  final String uploadId;
  final String status;
  final int bytes;
  final String? fileName;

  UploadResponse({
    required this.uploadId,
    required this.status,
    required this.bytes,
    this.fileName,
  });

  factory UploadResponse.fromJson(Map<String, dynamic> json) {
    return UploadResponse(
      uploadId: json['upload_id'] as String,
      status: json['status'] as String,
      bytes: json['bytes'] as int,
      fileName: json['file_name'] as String?,
    );
  }
}

enum ConversionStatus {
  queued,
  normalizing,
  detecting,
  analyzing,
  compiling,
  rendering,
  verifying,
  completed,
  failed,
  expired,
  pending,
  processing,
}

ConversionStatus _conversionStatusFromJson(String value) {
  return ConversionStatus.values.firstWhere(
    (status) => status.name == value,
    orElse: () => ConversionStatus.queued,
  );
}

class ConversionJob {
  final String jobId;
  final String? uploadId;
  final String? mainTex;
  final String? profile;
  final String? quality;
  final String? engine;
  final ConversionStatus status;
  final String createdAt;
  final String updatedAt;
  final bool docxReady;
  final bool reportReady;
  final String? errorCode;
  final String? error;

  ConversionJob({
    required this.jobId,
    required this.uploadId,
    required this.mainTex,
    required this.profile,
    required this.quality,
    required this.engine,
    required this.status,
    required this.createdAt,
    required this.updatedAt,
    required this.docxReady,
    required this.reportReady,
    required this.errorCode,
    required this.error,
  });

  bool get isTerminal =>
      status == ConversionStatus.completed ||
      status == ConversionStatus.failed ||
      status == ConversionStatus.expired;

  factory ConversionJob.fromJson(Map<String, dynamic> json) {
    return ConversionJob(
      jobId: json['job_id'] as String,
      uploadId: json['upload_id'] as String?,
      mainTex: json['main_tex'] as String?,
      profile: json['profile'] as String?,
      quality: json['quality'] as String?,
      engine: json['engine'] as String?,
      status: _conversionStatusFromJson(json['status'] as String),
      createdAt: json['created_at'] as String,
      updatedAt: json['updated_at'] as String,
      docxReady: json['docx_ready'] as bool,
      reportReady: json['report_ready'] as bool,
      errorCode: json['error_code'] as String?,
      error: json['error'] as String?,
    );
  }
}

class ConversionReport {
  final String jobId;
  final ConversionStatus status;
  final int qualityScore;
  final String profile;
  final String? mainTex;
  final String? executor;
  final String? backend;
  final String? qualityStatus;
  final int? compatibilityScore;
  final int? docxBytes;
  final List<String> warnings;
  final String? errorCode;
  final String message;

  ConversionReport({
    required this.jobId,
    required this.status,
    required this.qualityScore,
    required this.profile,
    required this.mainTex,
    required this.executor,
    required this.backend,
    required this.qualityStatus,
    required this.compatibilityScore,
    required this.docxBytes,
    required this.warnings,
    required this.errorCode,
    required this.message,
  });

  factory ConversionReport.fromJson(Map<String, dynamic> json) {
    return ConversionReport(
      jobId: json['job_id'] as String,
      status: _conversionStatusFromJson(json['status'] as String),
      qualityScore: json['quality_score'] as int,
      profile: json['profile'] as String,
      mainTex: json['main_tex'] as String?,
      executor: json['executor'] as String?,
      backend: json['backend'] as String?,
      qualityStatus: json['quality_status'] as String?,
      compatibilityScore: json['compatibility_score'] as int?,
      docxBytes: json['docx_bytes'] as int?,
      warnings: ((json['warnings'] as List<dynamic>?) ?? const <dynamic>[])
          .map((item) => item.toString())
          .toList(growable: false),
      errorCode: json['error_code'] as String?,
      message: json['message'] as String,
    );
  }
}
