// ignore_for_file: use_null_aware_elements

import 'dart:convert';

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:http/http.dart' as http;

const localCommercialApiBaseUrl = 'http://127.0.0.1:2624/v1/';
const legacyOnlineCommercialApiBaseUrl = 'https://api.tex2doc.cn/v1/';

String get defaultCommercialApiBaseUrl {
  if (kIsWeb && (Uri.base.scheme == 'http' || Uri.base.scheme == 'https')) {
    return Uri.base.resolve('/v1/').toString();
  }
  return localCommercialApiBaseUrl;
}

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

  Future<AdminProfile> adminMe(String accessToken) async {
    final response = await _http.get(
      _adminUri('me'),
      headers: _headers(accessToken: accessToken),
    );
    return AdminProfile.fromJson(_decode(response) as Map<String, dynamic>);
  }

  Future<AdminDashboardSummary> adminDashboard(String accessToken) async {
    final response = await _http.get(
      _adminUri('dashboard'),
      headers: _headers(accessToken: accessToken),
    );
    return AdminDashboardSummary.fromJson(
      _decode(response) as Map<String, dynamic>,
    );
  }

  Future<Map<String, dynamic>> downloads() async {
    return _decode(await _http.get(baseUri.resolve('downloads')))
        as Map<String, dynamic>;
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

  Future<Map<String, dynamic>> createWaitlistLead({
    required String email,
    String? displayName,
    String? company,
    String? scenario,
    String? source,
  }) async {
    return _postJson('waitlist', {
      'email': email,
      if (displayName != null && displayName.trim().isNotEmpty)
        'display_name': displayName.trim(),
      if (company != null && company.trim().isNotEmpty)
        'company': company.trim(),
      if (scenario != null && scenario.trim().isNotEmpty)
        'scenario': scenario.trim(),
      if (source != null && source.trim().isNotEmpty) 'source': source.trim(),
    });
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

  Future<List<RedeemCodeBatch>> redeemCodeBatches({
    required String adminToken,
  }) async {
    final response = await _http.get(
      _adminUri('redeem-code-batches'),
      headers: _headers(accessToken: adminToken),
    );
    final value = _decode(response) as List<dynamic>;
    return value
        .map((item) => RedeemCodeBatch.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<RedeemCodeBatch> redeemCodeBatchDetail({
    required String adminToken,
    required String batchId,
  }) async {
    final response = await _http.get(
      _adminUri('redeem-code-batches/$batchId'),
      headers: _headers(accessToken: adminToken),
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

  /// Admin: paginated redeem-codes list with optional filters.
  Future<AdminRedeemCodeListResult> adminListRedeemCodes({
    required String adminToken,
    String? stockStatus,
    String? batchId,
    String? packageId,
    String? search,
    int page = 1,
    int pageSize = 50,
  }) async {
    final q = <String, String>{
      if (stockStatus != null) 'stock_status': stockStatus,
      if (batchId != null) 'batch_id': batchId,
      if (packageId != null) 'package_id': packageId,
      if (search != null && search.isNotEmpty) 'search': search,
      'page': page.toString(),
      'page_size': pageSize.toString(),
    };
    final uri = baseUri.replace(path: '/admin/v1/redeem-codes', queryParameters: q);
    final response = await _http.get(
      uri,
      headers: _headers(accessToken: adminToken),
    );
    return AdminRedeemCodeListResult.fromJson(_decode(response) as Map<String, dynamic>);
  }

  /// Admin: bulk mark codes as "stocked" (上货).
  Future<int> adminBulkStockRedeemCodes({
    required String adminToken,
    required List<String> codeIds,
  }) async {
    final response = await _http.post(
      _adminUri('redeem-codes'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({'code_ids': codeIds}),
    );
    final data = _decode(response) as Map<String, dynamic>;
    return (data['affected'] as num?)?.toInt() ?? 0;
  }

  /// Admin: restock (reset) codes to "new" from plaintext codes in a text file.
  Future<int> adminRestockRedeemCodes({
    required String adminToken,
    required String codes,
  }) async {
    final response = await _http.post(
      _adminUri('redeem-codes/restock'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({'codes': codes}),
    );
    final data = _decode(response) as Map<String, dynamic>;
    return (data['affected'] as num?)?.toInt() ?? 0;
  }

  /// Admin: export redeem-codes list as Excel with current filter.
  Future<List<int>> adminExportRedeemCodesExcel({
    required String adminToken,
    String? stockStatus,
    String? batchId,
    String? packageId,
    String? search,
  }) async {
    final q = <String, String>{
      if (stockStatus != null) 'stock_status': stockStatus,
      if (batchId != null) 'batch_id': batchId,
      if (packageId != null) 'package_id': packageId,
      if (search != null && search.isNotEmpty) 'search': search,
    };
    final uri = baseUri.replace(path: '/admin/v1/redeem-codes/export.xlsx', queryParameters: q.isEmpty ? null : q);
    final response = await _http.get(
      uri,
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

  Future<List<FeedbackThread>> adminFeedbackThreads(String adminToken) async {
    final response = await _http.get(
      _adminUri('feedback/threads'),
      headers: _headers(accessToken: adminToken),
    );
    final value = _decode(response) as List<dynamic>;
    return value
        .map((item) => FeedbackThread.fromJson(item as Map<String, dynamic>))
        .toList(growable: false);
  }

  Future<FeedbackThread> adminUpdateFeedbackThread({
    required String adminToken,
    required String threadId,
    String? status,
    String? priority,
  }) async {
    final body = <String, dynamic>{
      if (status != null) 'status': status,
      if (priority != null) 'priority': priority,
    };
    final response = await _http.patch(
      _adminUri('feedback/threads/$threadId'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode(body),
    );
    return FeedbackThread.fromJson(_decode(response) as Map<String, dynamic>);
  }

  Future<FeedbackMessage> adminReplyFeedbackThread({
    required String adminToken,
    required String threadId,
    required String content,
    bool isInternal = false,
  }) async {
    final response = await _http.post(
      _adminUri('feedback/threads/$threadId/messages'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({'content': content, 'is_internal': isInternal}),
    );
    return FeedbackMessage.fromJson(_decode(response) as Map<String, dynamic>);
  }

  Future<List<Map<String, dynamic>>> adminUsers(String adminToken) async {
    return _decodeMapList(
      await _http.get(
        _adminUri('users'),
        headers: _headers(accessToken: adminToken),
      ),
    );
  }

  Future<List<Map<String, dynamic>>> adminUsageLedger(String adminToken) async {
    return _decodeMapList(
      await _http.get(
        _adminUri('usage-ledger'),
        headers: _headers(accessToken: adminToken),
      ),
    );
  }

  Future<List<Map<String, dynamic>>> adminManualOrders(
    String adminToken,
  ) async {
    return _decodeMapList(
      await _http.get(
        _adminUri('manual-orders'),
        headers: _headers(accessToken: adminToken),
      ),
    );
  }

  Future<Map<String, dynamic>> adminCreateManualOrder({
    required String adminToken,
    required String userId,
    required String rechargeType,
    required String packageId,
    required int quantity,
    required int amountCents,
    String currency = 'CNY',
    String? note,
  }) async {
    final response = await _http.post(
      _adminUri('manual-orders'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({
        'user_id': userId,
        'recharge_type': rechargeType,
        'package_id': packageId,
        'quantity': quantity,
        'amount_cents': amountCents,
        'currency': currency,
        if (note != null && note.trim().isNotEmpty) 'note': note.trim(),
      }),
    );
    return _decode(response) as Map<String, dynamic>;
  }

  Future<List<Map<String, dynamic>>> adminWaitlist(String adminToken) async {
    return _decodeMapList(
      await _http.get(
        _adminUri('waitlist'),
        headers: _headers(accessToken: adminToken),
      ),
    );
  }

  Future<List<Map<String, dynamic>>> adminReleases(String adminToken) async {
    return _decodeMapList(
      await _http.get(
        _adminUri('releases'),
        headers: _headers(accessToken: adminToken),
      ),
    );
  }

  Future<Map<String, dynamic>> adminPublishRelease({
    required String adminToken,
    required String channel,
    required String platform,
    required String version,
    required String downloadUrl,
    required String sha256,
    String arch = 'x64',
    String? minAppVersion,
    String? signature,
    String? signatureAlgorithm,
    int? fileSizeBytes,
    String? releaseTitle,
    bool isPrerelease = false,
    Map<String, dynamic>? strategy,
  }) async {
    final response = await _http.post(
      _adminUri('releases'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({
        'channel': channel,
        'platform': platform,
        'arch': arch,
        'version': version,
        'download_url': downloadUrl,
        'sha256': sha256,
        if (minAppVersion != null && minAppVersion.trim().isNotEmpty)
          'min_app_version': minAppVersion.trim(),
        if (signature != null && signature.trim().isNotEmpty)
          'signature': signature.trim(),
        if (signatureAlgorithm != null && signatureAlgorithm.trim().isNotEmpty)
          'signature_algorithm': signatureAlgorithm.trim(),
        if (fileSizeBytes != null) 'file_size_bytes': fileSizeBytes,
        if (releaseTitle != null && releaseTitle.trim().isNotEmpty)
          'release_title': releaseTitle.trim(),
        'is_prerelease': isPrerelease,
        if (strategy != null) 'strategy': strategy,
      }),
    );
    return _decode(response) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> adminRollbackRelease({
    required String adminToken,
    required String releaseId,
    String? reason,
  }) async {
    final response = await _http.post(
      _adminUri('releases/$releaseId/rollback'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({
        if (reason != null && reason.trim().isNotEmpty) 'reason': reason.trim(),
      }),
    );
    return _decode(response) as Map<String, dynamic>;
  }

  Future<List<Map<String, dynamic>>> adminReleaseAudit(
    String adminToken,
  ) async {
    return _decodeMapList(
      await _http.get(
        _adminUri('release-audit'),
        headers: _headers(accessToken: adminToken),
      ),
    );
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
      await _postJson(
        'feedback/threads/$threadId/messages',
        body,
        accessToken: accessToken,
      ),
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

  List<Map<String, dynamic>> _decodeMapList(http.Response response) {
    final value = _decode(response) as List<dynamic>;
    return value
        .map((item) => Map<String, dynamic>.from(item as Map))
        .toList(growable: false);
  }

  static Uri _normalizeBaseUrl(String value) {
    final valueTrimmed = value.trim();
    final trimmed =
        valueTrimmed.isEmpty || valueTrimmed == legacyOnlineCommercialApiBaseUrl
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
  final String role;

  UserProfile({
    required this.id,
    required this.email,
    required this.displayName,
    required this.planId,
    required this.role,
  });

  bool get isAdminRole =>
      role == 'admin' || role == 'operator' || role == 'support';

  factory UserProfile.fromJson(Map<String, dynamic> json) {
    return UserProfile(
      id: json['id'] as String,
      email: json['email'] as String,
      displayName: json['display_name'] as String?,
      planId: json['plan_id'] as String,
      role: json['role'] as String? ?? 'user',
    );
  }
}

class AdminProfile {
  final UserProfile user;
  final List<String> permissions;

  AdminProfile({required this.user, required this.permissions});

  factory AdminProfile.fromJson(Map<String, dynamic> json) {
    return AdminProfile(
      user: UserProfile.fromJson(json['user'] as Map<String, dynamic>),
      permissions: (json['permissions'] as List<dynamic>? ?? const [])
          .map((item) => item.toString())
          .toList(growable: false),
    );
  }
}

class AdminDashboardSummary {
  final int billingPlans;
  final int redeemBatches;
  final int feedbackThreads;
  final int openFeedback;
  final List<String> releaseChannels;
  final List<String> modules;
  final String generatedAt;

  AdminDashboardSummary({
    required this.billingPlans,
    required this.redeemBatches,
    required this.feedbackThreads,
    required this.openFeedback,
    required this.releaseChannels,
    required this.modules,
    required this.generatedAt,
  });

  factory AdminDashboardSummary.fromJson(Map<String, dynamic> json) {
    final counts = json['counts'] as Map<String, dynamic>? ?? const {};
    return AdminDashboardSummary(
      billingPlans: (counts['billing_plans'] as num?)?.toInt() ?? 0,
      redeemBatches: (counts['redeem_batches'] as num?)?.toInt() ?? 0,
      feedbackThreads: (counts['feedback_threads'] as num?)?.toInt() ?? 0,
      openFeedback: (counts['open_feedback'] as num?)?.toInt() ?? 0,
      releaseChannels: (json['release_channels'] as List<dynamic>? ?? const [])
          .map((item) => item.toString())
          .toList(growable: false),
      modules: (json['modules'] as List<dynamic>? ?? const [])
          .map((item) => item.toString())
          .toList(growable: false),
      generatedAt: json['generated_at']?.toString() ?? '',
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
      provider: json['provider'] as String? ?? 'manual-order',
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
  final String stockStatus;
  final String? stockedBy;
  final String? stockedAt;
  final String? redeemedRechargeId;
  final String? redeemedAt;
  final String? restockedBy;
  final String? restockedAt;
  final String createdAt;

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
    required this.stockStatus,
    this.stockedBy,
    this.stockedAt,
    this.redeemedRechargeId,
    this.redeemedAt,
    this.restockedBy,
    this.restockedAt,
    required this.createdAt,
  });

  factory RedeemCodeRecord.fromJson(Map<String, dynamic> json) {
    return RedeemCodeRecord(
      redeemId: json['redeem_id'] as String? ?? (json['code_id'] as String? ?? ''),
      batchId: json['batch_id'] as String,
      batchNo: json['batch_no'] as String,
      codePreview: json['code_preview'] as String,
      packageId: json['package_id'] as String,
      packageName: json['package_name'] as String,
      rechargeType: json['recharge_type'] as String,
      quantity: json['quantity'] as int,
      status: json['status'] as String,
      stockStatus: json['stock_status'] as String? ?? 'new',
      stockedBy: json['stocked_by'] as String?,
      stockedAt: json['stocked_at'] as String?,
      redeemedRechargeId: json['redeemed_recharge_id'] as String?,
      redeemedAt: json['redeemed_at'] as String?,
      restockedBy: json['restocked_by'] as String?,
      restockedAt: json['restocked_at'] as String?,
      createdAt: json['created_at'] as String? ?? DateTime.now().toUtc().toIso8601String(),
    );
  }

  String get label => '$codePreview: $packageName, $stockStatus';
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
  final String provider;
  final String phase;
  final String status;
  final String? message;
  final String? planId;
  final String? userId;
  final String? returnUrl;
  final String? successUrl;
  final String? cancelUrl;

  BillingSession({
    required this.url,
    required this.expiresAt,
    required this.provider,
    required this.phase,
    required this.status,
    this.message,
    this.planId,
    this.userId,
    this.returnUrl,
    this.successUrl,
    this.cancelUrl,
  });

  factory BillingSession.fromJson(Map<String, dynamic> json) {
    return BillingSession(
      url: json['url'] as String? ?? '',
      expiresAt: json['expires_at'] as String? ?? '',
      provider: json['provider'] as String? ?? 'manual-order',
      phase: json['phase'] as String? ?? 'phase_a',
      status: json['status'] as String? ?? 'pending_manual',
      message: json['message'] as String?,
      planId: json['plan_id'] as String?,
      userId: json['user_id'] as String?,
      returnUrl: json['return_url'] as String?,
      successUrl: json['success_url'] as String?,
      cancelUrl: json['cancel_url'] as String?,
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
    final storage = json['storage_info'] ?? json['storage'];
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
      storageInfo: storage is Map<String, dynamic>
          ? ConversionStorageInfo.fromJson(storage)
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
      updatedAt:
          (json['updated_at'] as String?) ?? json['created_at'] as String,
    );
  }

  String get statusLabel {
    switch (status) {
      case 'open':
        return 'Open';
      case 'in_progress':
        return 'In Progress';
      case 'resolved':
        return 'Resolved';
      case 'closed':
        return 'Closed';
      default:
        return status;
    }
  }

  String get typeLabel => feedbackType == 'issue' ? 'Issue' : 'Requirement';

  String get priorityLabel {
    switch (priority) {
      case 'low':
        return 'Low';
      case 'high':
        return 'High';
      case 'urgent':
        return 'Urgent';
      case 'normal':
        return 'Normal';
      default:
        return priority;
    }
  }
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
      case 'user':
        return 'You';
      case 'admin':
        return 'Support';
      case 'system':
        return 'System';
      default:
        return senderType;
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

  factory FileMeta.fromKey(String? key, int? bytes) {
    return FileMeta(key: key ?? '', bytes: bytes);
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

  ConversionStorageInfo({
    this.path,
    this.sourceZip,
    this.resultDocx,
    this.conversionLog,
  });

  factory ConversionStorageInfo.fromJson(Map<String, dynamic> json) {
    final sourceZipKey = json['source_zip_key'] as String?;
    final resultDocxKey = json['result_docx_key'] as String?;
    final conversionLogKey =
        (json['conversion_log_key'] ?? json['result_log_key']) as String?;
    return ConversionStorageInfo(
      path: json['path'] as String?,
      sourceZip: (json['source_zip'] as Map<String, dynamic>?) != null
          ? FileMeta.fromJson(json['source_zip'] as Map<String, dynamic>)
          : sourceZipKey != null
          ? FileMeta.fromKey(sourceZipKey, (json['zip_bytes'] as num?)?.toInt())
          : null,
      resultDocx: (json['result_docx'] as Map<String, dynamic>?) != null
          ? FileMeta.fromJson(json['result_docx'] as Map<String, dynamic>)
          : resultDocxKey != null
          ? FileMeta.fromKey(
              resultDocxKey,
              (json['docx_bytes'] as num?)?.toInt(),
            )
          : null,
      conversionLog: (json['conversion_log'] as Map<String, dynamic>?) != null
          ? FileMeta.fromJson(json['conversion_log'] as Map<String, dynamic>)
          : conversionLogKey != null
          ? FileMeta.fromKey(
              conversionLogKey,
              (json['log_bytes'] as num?)?.toInt(),
            )
          : null,
    );
  }

  bool get hasZip => sourceZip != null;
  bool get hasDocx => resultDocx != null;
  bool get hasLog => conversionLog != null;
  bool get hasAny => hasZip || hasDocx || hasLog;
}

/// Paginated result from the admin redeem-codes list endpoint.
class AdminRedeemCodeListResult {
  final List<RedeemCodeRecord> records;
  final int total;
  final int page;
  final int pageSize;

  AdminRedeemCodeListResult({
    required this.records,
    required this.total,
    required this.page,
    required this.pageSize,
  });

  factory AdminRedeemCodeListResult.fromJson(Map<String, dynamic> json) {
    return AdminRedeemCodeListResult(
      records: ((json['records'] as List<dynamic>?) ?? const [])
          .map((item) => RedeemCodeRecord.fromJson(item as Map<String, dynamic>))
          .toList(growable: false),
      total: (json['total'] as num?)?.toInt() ?? 0,
      page: (json['page'] as num?)?.toInt() ?? 1,
      pageSize: (json['page_size'] as num?)?.toInt() ?? 50,
    );
  }

  int get totalPages => (total / pageSize).ceil();
  bool get hasNextPage => page < totalPages;
  bool get hasPrevPage => page > 1;
}

// ─────────────────────────────────────────────────────────────────────────────
// Automation R&D API Client
// ─────────────────────────────────────────────────────────────────────────────

/// Automation request summary for dashboard overview.
class AutomationSummary {
  final int pendingApproval;
  final int waitingDev;
  final int inDevelopment;
  final int localFailed;
  final int ciFailed;
  final int deployed;
  final int total;

  AutomationSummary({
    required this.pendingApproval,
    required this.waitingDev,
    required this.inDevelopment,
    required this.localFailed,
    required this.ciFailed,
    required this.deployed,
    required this.total,
  });

  factory AutomationSummary.fromJson(Map<String, dynamic> json) {
    return AutomationSummary(
      pendingApproval: (json['pending_approval'] as num?)?.toInt() ?? 0,
      waitingDev: (json['waiting_dev'] as num?)?.toInt() ?? 0,
      inDevelopment: (json['in_development'] as num?)?.toInt() ?? 0,
      localFailed: (json['local_failed'] as num?)?.toInt() ?? 0,
      ciFailed: (json['ci_failed'] as num?)?.toInt() ?? 0,
      deployed: (json['deployed'] as num?)?.toInt() ?? 0,
      total: (json['total'] as num?)?.toInt() ?? 0,
    );
  }
}

/// Automation request record.
class AutomationRequest {
  final String id;
  final String shortId;
  final String sourceType;
  final String sourceId;
  final String? feedbackThreadId;
  final String title;
  final String requestType;
  final String status;
  final String priority;
  final String riskLevel;
  final String? aiSummary;
  final String? claimedBy;
  final String? branchName;
  final String? prUrl;
  final String? ciRunUrl;
  final String? deployedVersion;
  final String createdAt;
  final String updatedAt;

  AutomationRequest({
    required this.id,
    required this.shortId,
    required this.sourceType,
    required this.sourceId,
    this.feedbackThreadId,
    required this.title,
    required this.requestType,
    required this.status,
    required this.priority,
    required this.riskLevel,
    this.aiSummary,
    this.claimedBy,
    this.branchName,
    this.prUrl,
    this.ciRunUrl,
    this.deployedVersion,
    required this.createdAt,
    required this.updatedAt,
  });

  factory AutomationRequest.fromJson(Map<String, dynamic> json) {
    return AutomationRequest(
      id: json['id'] as String? ?? '',
      shortId: json['short_id'] as String? ?? '',
      sourceType: json['source_type'] as String? ?? '',
      sourceId: json['source_id'] as String? ?? '',
      feedbackThreadId: json['feedback_thread_id'] as String?,
      title: json['title'] as String? ?? '',
      requestType: json['request_type'] as String? ?? 'unknown',
      status: json['status'] as String? ?? 'submitted',
      priority: json['priority'] as String? ?? 'normal',
      riskLevel: json['risk_level'] as String? ?? 'unknown',
      aiSummary: json['ai_summary'] as String?,
      claimedBy: json['claimed_by'] as String?,
      branchName: json['branch_name'] as String?,
      prUrl: json['pr_url'] as String?,
      ciRunUrl: json['ci_run_url'] as String?,
      deployedVersion: json['deployed_version'] as String?,
      createdAt: json['created_at'] as String? ?? '',
      updatedAt: json['updated_at'] as String? ?? '',
    );
  }

  String get statusLabel => _statusLabels[status] ?? status;
  String get riskLabel => _riskLabels[riskLevel] ?? riskLevel;
  String get typeLabel => _typeLabels[requestType] ?? requestType;
  String get sourceLabel => _sourceLabels[sourceType] ?? sourceType;

  static const _statusLabels = {
    'submitted': 'Submitted',
    'triaged': 'Triaged',
    'needs_approval': 'Needs Approval',
    'queued_for_dev': 'Queued',
    'claimed': 'Claimed',
    'coding': 'Coding',
    'local_validating': 'Validating',
    'local_failed': 'Local Failed',
    'pr_open': 'PR Open',
    'ci_running': 'CI Running',
    'ci_failed': 'CI Failed',
    'ready_for_merge': 'Ready',
    'production_deployed': 'Deployed',
    'notified': 'Notified',
    'needs_human': 'Needs Human',
    'blocked': 'Blocked',
    'closed': 'Closed',
    'rejected': 'Rejected',
  };

  static const _riskLabels = {
    'low': 'Low',
    'medium': 'Medium',
    'high': 'High',
    'critical': 'Critical',
    'unknown': 'Unknown',
  };

  static const _typeLabels = {
    'bug': 'Bug',
    'requirement': 'Requirement',
    'docs': 'Docs',
    'test': 'Test',
    'ops': 'Ops',
    'unknown': 'Unknown',
  };

  static const _sourceLabels = {
    'feedback': 'Feedback',
    'github_issue': 'GitHub Issue',
    'admin_manual': 'Manual',
    'ci_failure': 'CI Failure',
  };
}

/// Automation event for timeline.
class AutomationEvent {
  final String id;
  final String requestId;
  final String eventType;
  final String actorType;
  final String? actorId;
  final String? actorName;
  final String? fromStatus;
  final String? toStatus;
  final String message;
  final Map<String, dynamic> payload;
  final String createdAt;

  AutomationEvent({
    required this.id,
    required this.requestId,
    required this.eventType,
    required this.actorType,
    this.actorId,
    this.actorName,
    this.fromStatus,
    this.toStatus,
    required this.message,
    required this.payload,
    required this.createdAt,
  });

  factory AutomationEvent.fromJson(Map<String, dynamic> json) {
    return AutomationEvent(
      id: json['id'] as String? ?? '',
      requestId: json['request_id'] as String? ?? '',
      eventType: json['event_type'] as String? ?? '',
      actorType: json['actor_type'] as String? ?? '',
      actorId: json['actor_id'] as String?,
      actorName: json['actor_name'] as String?,
      fromStatus: json['from_status'] as String?,
      toStatus: json['to_status'] as String?,
      message: json['message'] as String? ?? '',
      payload: (json['payload'] as Map<String, dynamic>?) ?? {},
      createdAt: json['created_at'] as String? ?? '',
    );
  }
}

/// Automation agent record.
class AutomationAgent {
  final String id;
  final String hostname;
  final String agentVersion;
  final String status;
  final String? currentRequestId;
  final Map<String, dynamic> capabilities;
  final int totalTasksCompleted;
  final int totalTasksFailed;
  final String lastHeartbeatAt;
  final String registeredAt;

  AutomationAgent({
    required this.id,
    required this.hostname,
    required this.agentVersion,
    required this.status,
    this.currentRequestId,
    required this.capabilities,
    required this.totalTasksCompleted,
    required this.totalTasksFailed,
    required this.lastHeartbeatAt,
    required this.registeredAt,
  });

  factory AutomationAgent.fromJson(Map<String, dynamic> json) {
    return AutomationAgent(
      id: json['id'] as String? ?? '',
      hostname: json['hostname'] as String? ?? '',
      agentVersion: json['agent_version'] as String? ?? '',
      status: json['status'] as String? ?? 'offline',
      currentRequestId: json['current_request_id'] as String?,
      capabilities: (json['capabilities'] as Map<String, dynamic>?) ?? {},
      totalTasksCompleted: (json['total_tasks_completed'] as num?)?.toInt() ?? 0,
      totalTasksFailed: (json['total_tasks_failed'] as num?)?.toInt() ?? 0,
      lastHeartbeatAt: json['last_heartbeat_at'] as String? ?? '',
      registeredAt: json['registered_at'] as String? ?? '',
    );
  }

  double get successRate {
    final total = totalTasksCompleted + totalTasksFailed;
    if (total == 0) return 0;
    return totalTasksCompleted / total * 100;
  }
}

extension AutomationApiClientExt on CommercialApiClient {
  Future<AutomationSummary> adminAutomationSummary(String adminToken) async {
    final response = await _http.get(
      _adminUri('automation/summary'),
      headers: _headers(accessToken: adminToken),
    );
    return AutomationSummary.fromJson(_decodeMap(response));
  }

  Future<List<AutomationRequest>> adminAutomationRequests(
    String adminToken, {
    String? status,
    String? riskLevel,
    String? sourceType,
    String? search,
    int? limit,
    int? offset,
  }) async {
    final queryParams = <String, String>{};
    if (status != null) queryParams['status'] = status;
    if (riskLevel != null) queryParams['risk_level'] = riskLevel;
    if (sourceType != null) queryParams['source_type'] = sourceType;
    if (search != null) queryParams['search'] = search;
    if (limit != null) queryParams['limit'] = limit.toString();
    if (offset != null) queryParams['offset'] = offset.toString();

    final uri = _adminUri('automation/requests').replace(
      queryParameters: queryParams.isEmpty ? null : queryParams,
    );

    final response = await _http.get(uri, headers: _headers(accessToken: adminToken));
    final list = _decodeList(response);
    return list.map((json) => AutomationRequest.fromJson(json)).toList();
  }

  Future<AutomationRequest> adminAutomationRequest(String adminToken, String id) async {
    final response = await _http.get(
      _adminUri('automation/requests/$id'),
      headers: _headers(accessToken: adminToken),
    );
    return AutomationRequest.fromJson(_decodeMap(response));
  }

  Future<List<AutomationEvent>> adminAutomationEvents(String adminToken, String requestId) async {
    final response = await _http.get(
      _adminUri('automation/requests/$requestId/events'),
      headers: _headers(accessToken: adminToken),
    );
    final list = _decodeList(response);
    return list.map((json) => AutomationEvent.fromJson(json)).toList();
  }

  Future<AutomationRequest> adminAutomationApprove(String adminToken, String requestId) async {
    final response = await _http.post(
      _adminUri('automation/requests/$requestId/approve'),
      headers: _headers(accessToken: adminToken),
    );
    return AutomationRequest.fromJson(_decodeMap(response));
  }

  Future<AutomationRequest> adminAutomationReject(
    String adminToken,
    String requestId,
    String reason,
  ) async {
    final response = await _http.post(
      _adminUri('automation/requests/$requestId/reject'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({'reason': reason}),
    );
    return AutomationRequest.fromJson(_decodeMap(response));
  }

  Future<AutomationRequest> adminAutomationRetry(String adminToken, String requestId) async {
    final response = await _http.post(
      _adminUri('automation/requests/$requestId/retry'),
      headers: _headers(accessToken: adminToken),
    );
    return AutomationRequest.fromJson(_decodeMap(response));
  }

  Future<AutomationRequest> adminAutomationEscalate(
    String adminToken,
    String requestId,
    String assignee,
  ) async {
    final response = await _http.post(
      _adminUri('automation/requests/$requestId/escalate'),
      headers: _headers(accessToken: adminToken),
      body: jsonEncode({'assignee': assignee}),
    );
    return AutomationRequest.fromJson(_decodeMap(response));
  }

  Future<List<AutomationAgent>> adminAutomationAgents(String adminToken) async {
    final response = await _http.get(
      _adminUri('automation/agents'),
      headers: _headers(accessToken: adminToken),
    );
    final list = _decodeList(response);
    return list.map((json) => AutomationAgent.fromJson(json)).toList();
  }

  Future<AutomationAgent> adminAutomationPauseAgent(String adminToken, String agentId) async {
    final response = await _http.post(
      _adminUri('automation/agents/$agentId/pause'),
      headers: _headers(accessToken: adminToken),
    );
    return AutomationAgent.fromJson(_decodeMap(response));
  }

  Future<AutomationAgent> adminAutomationResumeAgent(String adminToken, String agentId) async {
    final response = await _http.post(
      _adminUri('automation/agents/$agentId/resume'),
      headers: _headers(accessToken: adminToken),
    );
    return AutomationAgent.fromJson(_decodeMap(response));
  }
}
