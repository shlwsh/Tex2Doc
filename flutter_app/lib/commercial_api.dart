import 'dart:convert';

import 'package:http/http.dart' as http;

const defaultCommercialApiBaseUrl = 'http://127.0.0.1:2624/v1/';
const legacyLocalCommercialApiBaseUrl = 'http://127.0.0.1:8080/v1/';
const legacyOnlineCommercialApiBaseUrl = 'https://api.tex2doc.cn/v1/';

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

  Future<RedeemCodeOptions> redeemCodeOptions(String accessToken) async {
    return RedeemCodeOptions.fromJson(
      await _getJson('redeem-codes/options', accessToken: accessToken)
          as Map<String, dynamic>,
    );
  }

  Future<RedeemCodeResult> redeemCode({
    required String accessToken,
    required String code,
  }) async {
    return RedeemCodeResult.fromJson(
      await _postJson('redeem-codes/redeem', {
        'code': code,
      }, accessToken: accessToken),
    );
  }

  Future<List<RedeemCodeRecord>> redeemCodeRecords(String accessToken) async {
    final value = await _getJson(
      'redeem-codes/records',
      accessToken: accessToken,
    );
    return (value as List<dynamic>)
        .map((item) => RedeemCodeRecord.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<RedeemCodeBatch> createRedeemCodeBatch({
    required String adminToken,
    required String packageId,
    required int quantity,
    String? channel,
    String? note,
    String? expiresAt,
  }) async {
    final body = <String, dynamic>{
      'package_id': packageId,
      'quantity': quantity,
      if (channel != null && channel.trim().isNotEmpty)
        'channel': channel.trim(),
      if (note != null && note.trim().isNotEmpty) 'note': note.trim(),
      if (expiresAt != null && expiresAt.trim().isNotEmpty)
        'expires_at': expiresAt.trim(),
    };
    final response = await _http.post(
      _adminUri('redeem-code-batches'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode(body),
    );
    return RedeemCodeBatch.fromJson(_decode(response) as Map<String, dynamic>);
  }

  Future<List<int>> exportRedeemCodeBatch({
    required String adminToken,
    required String batchId,
  }) async {
    final response = await _http.get(
      _adminUri('redeem-code-batches/$batchId/export.xlsx'),
      headers: _headers(accessToken: adminToken),
    );
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw CommercialApiException(
        response.statusCode,
        utf8.decode(response.bodyBytes),
      );
    }
    return response.bodyBytes;
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

  // ─── Feedback ────────────────────────────────────────────────────────────

  Future<List<FeedbackThread>> feedbackThreads(String accessToken) async {
    final value = await _getJson('feedback/threads', accessToken: accessToken);
    return (value as List<dynamic>)
        .map((item) => FeedbackThread.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<FeedbackThreadDetail> feedbackThread(
    String accessToken,
    String threadId,
  ) async {
    return FeedbackThreadDetail.fromJson(
      await _getJson('feedback/threads/$threadId', accessToken: accessToken)
          as Map<String, dynamic>,
    );
  }

  Future<CreateFeedbackResponse> createFeedbackThread({
    required String accessToken,
    required String title,
    required String feedbackType,
    required String content,
    String? conversionJobId,
    String? priority,
  }) async {
    final body = <String, dynamic>{
      'title': title,
      'feedback_type': feedbackType,
      'content': content,
      if (conversionJobId != null) 'conversion_job_id': conversionJobId,
      if (priority != null) 'priority': priority,
    };
    return CreateFeedbackResponse.fromJson(
      await _postJson('feedback/threads', body, accessToken: accessToken),
    );
  }

  Future<FeedbackMessage> addFeedbackMessage({
    required String accessToken,
    required String threadId,
    required String content,
    String? parentMessageId,
  }) async {
    final body = <String, dynamic>{
      'content': content,
      if (parentMessageId != null) 'parent_message_id': parentMessageId,
    };
    return FeedbackMessage.fromJson(
      await _postJson('feedback/threads/$threadId/messages', body, accessToken: accessToken),
    );
  }

  // ─── Session file downloads ──────────────────────────────────────────────

  Future<List<int>> downloadConversionZip({
    required String accessToken,
    required String jobId,
  }) async {
    final response = await _http.get(
      baseUri.resolve('conversions/$jobId/download/zip'),
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

  Future<List<int>> downloadConversionLog({
    required String accessToken,
    required String jobId,
  }) async {
    final response = await _http.get(
      baseUri.resolve('conversions/$jobId/download/log'),
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

  Uri _adminUri(String path) {
    final normalized = path.startsWith('/') ? path.substring(1) : path;
    return baseUri.replace(path: '/admin/v1/$normalized', query: null);
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
    final valueTrimmed = value.trim();
    final trimmed =
        valueTrimmed.isEmpty ||
            valueTrimmed == legacyLocalCommercialApiBaseUrl ||
            valueTrimmed == legacyOnlineCommercialApiBaseUrl
        ? defaultCommercialApiBaseUrl
        : valueTrimmed;
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

class RedeemCodeOptions {
  final bool enabled;
  final String provider;
  final String codeFormatHint;
  final String supportText;
  final List<RedeemPackageSummary> packages;

  RedeemCodeOptions({
    required this.enabled,
    required this.provider,
    required this.codeFormatHint,
    required this.supportText,
    required this.packages,
  });

  factory RedeemCodeOptions.fromJson(Map<String, dynamic> json) {
    return RedeemCodeOptions(
      enabled: json['enabled'] as bool? ?? false,
      provider: json['provider'] as String? ?? 'redeem-code',
      codeFormatHint:
          json['code_format_hint'] as String? ?? 'T2D-XXXX-XXXX-XXXX-XX',
      supportText: json['support_text'] as String? ?? '',
      packages: ((json['packages'] as List<dynamic>?) ?? const [])
          .map(
            (item) =>
                RedeemPackageSummary.fromJson(item as Map<String, dynamic>),
          )
          .toList(growable: false),
    );
  }
}

class RedeemPackageSummary {
  final String id;
  final String name;
  final String rechargeType;
  final int quantity;

  RedeemPackageSummary({
    required this.id,
    required this.name,
    required this.rechargeType,
    required this.quantity,
  });

  factory RedeemPackageSummary.fromJson(Map<String, dynamic> json) {
    return RedeemPackageSummary(
      id: json['id'] as String,
      name: json['name'] as String,
      rechargeType: json['recharge_type'] as String? ?? 'count',
      quantity: json['quantity'] as int,
    );
  }

  String get label => '$name ($quantity conversions)';
}

class RedeemCodeResult {
  final String redeemId;
  final String rechargeId;
  final String packageId;
  final String packageName;
  final String rechargeType;
  final int quantity;
  final int countBalance;
  final String? dateValidUntil;
  final String redeemedAt;

  RedeemCodeResult({
    required this.redeemId,
    required this.rechargeId,
    required this.packageId,
    required this.packageName,
    required this.rechargeType,
    required this.quantity,
    required this.countBalance,
    required this.dateValidUntil,
    required this.redeemedAt,
  });

  factory RedeemCodeResult.fromJson(Map<String, dynamic> json) {
    return RedeemCodeResult(
      redeemId: json['redeem_id'] as String,
      rechargeId: json['recharge_id'] as String,
      packageId: json['package_id'] as String,
      packageName: json['package_name'] as String,
      rechargeType: json['recharge_type'] as String,
      quantity: json['quantity'] as int,
      countBalance: json['count_balance'] as int,
      dateValidUntil: json['date_valid_until'] as String?,
      redeemedAt: json['redeemed_at'] as String,
    );
  }
}

class RedeemCodeRecord {
  final String redeemId;
  final String batchId;
  final String batchNo;
  final String codePreview;
  final String packageId;
  final String packageName;
  final String rechargeType;
  final int quantity;
  final String status;
  final String? redeemedRechargeId;
  final String? redeemedAt;

  RedeemCodeRecord({
    required this.redeemId,
    required this.batchId,
    required this.batchNo,
    required this.codePreview,
    required this.packageId,
    required this.packageName,
    required this.rechargeType,
    required this.quantity,
    required this.status,
    required this.redeemedRechargeId,
    required this.redeemedAt,
  });

  factory RedeemCodeRecord.fromJson(Map<String, dynamic> json) {
    return RedeemCodeRecord(
      redeemId: json['redeem_id'] as String,
      batchId: json['batch_id'] as String,
      batchNo: json['batch_no'] as String,
      codePreview: json['code_preview'] as String,
      packageId: json['package_id'] as String,
      packageName: json['package_name'] as String,
      rechargeType: json['recharge_type'] as String,
      quantity: json['quantity'] as int,
      status: json['status'] as String,
      redeemedRechargeId: json['redeemed_recharge_id'] as String?,
      redeemedAt: json['redeemed_at'] as String?,
    );
  }

  String get label => '$codePreview: $packageName, $status';
}

class RedeemCodeBatch {
  final String batchId;
  final String batchNo;
  final String packageId;
  final String packageName;
  final String rechargeType;
  final int quantity;
  final int generatedCount;
  final int exportedCount;
  final String status;
  final String? channel;
  final String? note;
  final String? expiresAt;
  final String createdAt;
  final List<String> codes;

  RedeemCodeBatch({
    required this.batchId,
    required this.batchNo,
    required this.packageId,
    required this.packageName,
    required this.rechargeType,
    required this.quantity,
    required this.generatedCount,
    required this.exportedCount,
    required this.status,
    required this.channel,
    required this.note,
    required this.expiresAt,
    required this.createdAt,
    required this.codes,
  });

  factory RedeemCodeBatch.fromJson(Map<String, dynamic> json) {
    return RedeemCodeBatch(
      batchId: json['batch_id'] as String,
      batchNo: json['batch_no'] as String,
      packageId: json['package_id'] as String,
      packageName: json['package_name'] as String,
      rechargeType: json['recharge_type'] as String,
      quantity: json['quantity'] as int,
      generatedCount: json['generated_count'] as int,
      exportedCount: json['exported_count'] as int,
      status: json['status'] as String,
      channel: json['channel'] as String?,
      note: json['note'] as String?,
      expiresAt: json['expires_at'] as String?,
      createdAt: json['created_at'] as String,
      codes: ((json['codes'] as List<dynamic>?) ?? const <dynamic>[])
          .map((item) => item.toString())
          .toList(growable: false),
    );
  }

  String get label => '$batchNo: $packageName x $generatedCount';
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
  final ConversionStorageInfo? storageInfo;

  ConversionJob({
    required this.jobId,
    this.uploadId,
    this.mainTex,
    this.profile,
    this.quality,
    this.engine,
    required this.status,
    required this.createdAt,
    required this.updatedAt,
    required this.docxReady,
    required this.reportReady,
    required this.errorCode,
    required this.error,
    this.storageInfo,
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
      storageInfo: (json['storage_info'] as Map<String, dynamic>?) != null
          ? ConversionStorageInfo.fromJson(json['storage_info'] as Map<String, dynamic>)
          : null,
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

// ─── Feedback models ────────────────────────────────────────────────────────────

class FeedbackThread {
  final String threadId;
  final String? conversionJobId;
  final String title;
  final String feedbackType;
  final String status;
  final String priority;
  final int messageCount;
  final String? latestMessageAt;
  final String createdAt;
  final String updatedAt;

  FeedbackThread({
    required this.threadId,
    this.conversionJobId,
    required this.title,
    required this.feedbackType,
    required this.status,
    required this.priority,
    required this.messageCount,
    this.latestMessageAt,
    required this.createdAt,
    required this.updatedAt,
  });

  factory FeedbackThread.fromJson(Map<String, dynamic> json) {
    return FeedbackThread(
      threadId: json['thread_id'] as String,
      conversionJobId: json['conversion_job_id'] as String?,
      title: json['title'] as String,
      feedbackType: json['feedback_type'] as String,
      status: json['status'] as String,
      priority: json['priority'] as String,
      messageCount: (json['message_count'] as num?)?.toInt() ?? 0,
      latestMessageAt: json['latest_message_at'] as String?,
      createdAt: json['created_at'] as String,
      updatedAt: (json['updated_at'] as String?) ?? json['created_at'] as String,
    );
  }

  String get statusLabel {
    switch (status) {
      case 'open': return 'Open';
      case 'in_progress': return 'In Progress';
      case 'resolved': return 'Resolved';
      case 'closed': return 'Closed';
      default: return status;
    }
  }

  String get typeLabel => feedbackType == 'issue' ? 'Issue' : 'Requirement';
}

class FeedbackMessage {
  final String messageId;
  final String threadId;
  final String? parentMessageId;
  final String? senderUserId;
  final String senderType;
  final String content;
  final bool isInternal;
  final String createdAt;

  FeedbackMessage({
    required this.messageId,
    required this.threadId,
    this.parentMessageId,
    this.senderUserId,
    required this.senderType,
    required this.content,
    required this.isInternal,
    required this.createdAt,
  });

  factory FeedbackMessage.fromJson(Map<String, dynamic> json) {
    return FeedbackMessage(
      messageId: json['message_id'] as String,
      threadId: json['thread_id'] as String,
      parentMessageId: json['parent_message_id'] as String?,
      senderUserId: json['sender_user_id'] as String?,
      senderType: json['sender_type'] as String,
      content: json['content'] as String,
      isInternal: json['is_internal'] as bool? ?? false,
      createdAt: json['created_at'] as String,
    );
  }

  String get senderLabel {
    switch (senderType) {
      case 'user': return 'You';
      case 'admin': return 'Support';
      case 'system': return 'System';
      default: return senderType;
    }
  }

  bool get isFromUser => senderType == 'user';
  bool get isFromAdmin => senderType == 'admin';
}

class FeedbackThreadDetail {
  final FeedbackThread thread;
  final List<FeedbackMessage> messages;

  FeedbackThreadDetail({required this.thread, required this.messages});

  factory FeedbackThreadDetail.fromJson(Map<String, dynamic> json) {
    return FeedbackThreadDetail(
      thread: FeedbackThread.fromJson(json['thread'] as Map<String, dynamic>),
      messages: ((json['messages'] as List<dynamic>?) ?? const <dynamic>[])
          .map((item) => FeedbackMessage.fromJson(item as Map<String, dynamic>))
          .toList(growable: false),
    );
  }
}

class CreateFeedbackResponse {
  final String threadId;
  final String status;
  final String createdAt;
  final String messageId;

  CreateFeedbackResponse({
    required this.threadId,
    required this.status,
    required this.createdAt,
    required this.messageId,
  });

  factory CreateFeedbackResponse.fromJson(Map<String, dynamic> json) {
    return CreateFeedbackResponse(
      threadId: json['thread_id'] as String,
      status: json['status'] as String,
      createdAt: json['created_at'] as String,
      messageId: json['message_id'] as String,
    );
  }
}

class FileMeta {
  final String key;
  final int? bytes;

  FileMeta({required this.key, this.bytes});

  factory FileMeta.fromJson(Map<String, dynamic> json) {
    return FileMeta(
      key: json['key'] as String,
      bytes: (json['bytes'] as num?)?.toInt(),
    );
  }

  String get sizeLabel {
    if (bytes == null) return '';
    if (bytes! < 1024) return '$bytes B';
    if (bytes! < 1024 * 1024) return '${(bytes! / 1024).toStringAsFixed(1)} KB';
    return '${(bytes! / 1024 / 1024).toStringAsFixed(1)} MB';
  }
}

class ConversionStorageInfo {
  final String? path;
  final FileMeta? sourceZip;
  final FileMeta? resultDocx;
  final FileMeta? conversionLog;

  ConversionStorageInfo({this.path, this.sourceZip, this.resultDocx, this.conversionLog});

  factory ConversionStorageInfo.fromJson(Map<String, dynamic> json) {
    return ConversionStorageInfo(
      path: json['path'] as String?,
      sourceZip: (json['source_zip'] as Map<String, dynamic>?) != null
          ? FileMeta.fromJson(json['source_zip'] as Map<String, dynamic>) : null,
      resultDocx: (json['result_docx'] as Map<String, dynamic>?) != null
          ? FileMeta.fromJson(json['result_docx'] as Map<String, dynamic>) : null,
      conversionLog: (json['conversion_log'] as Map<String, dynamic>?) != null
          ? FileMeta.fromJson(json['conversion_log'] as Map<String, dynamic>) : null,
    );
  }

  bool get hasZip => sourceZip != null;
  bool get hasDocx => resultDocx != null;
  bool get hasLog => conversionLog != null;
  bool get hasAny => hasZip || hasDocx || hasLog;
}
