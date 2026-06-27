// Quick activation service for shadow account (redeem code as email/password).
// This service handles the activation flow: login -> register if needed -> redeem code -> fetch usage.

import '../commercial_api.dart';
import 'quick_session.dart';

class QuickActivationResult {
  final String apiBaseUrl;
  final String accessToken;
  final UserProfile profile;
  final UsageSummary usage;
  final bool wasRegistered;

  const QuickActivationResult({
    required this.apiBaseUrl,
    required this.accessToken,
    required this.profile,
    required this.usage,
    required this.wasRegistered,
  });

  QuickSession toSession(String redeemCode) => QuickSession(
    apiBaseUrl: apiBaseUrl,
    accessToken: accessToken,
    profile: profile,
    redeemCode: redeemCode,
    usage: usage,
  );
}

class QuickActivationService {
  final String apiBaseUrl;

  QuickActivationService({required this.apiBaseUrl});

  /// Activates quick mode by:
  /// 1. Try login with code as email/password
  /// 2. If login fails (401/404), register a new shadow account
  /// 3. Redeem the code to activate quota
  /// 4. Fetch usage summary
  Future<QuickActivationResult> activate(String code) async {
    final client = CommercialApiClient(apiBaseUrl);
    bool wasRegistered = false;

    // Step 1: Try login
    try {
      final loginResult = await client.login(email: code, password: code);

      // Login succeeded. Try to redeem the code (409 = already redeemed is ok).
      try {
        await client.redeemCode(accessToken: loginResult.accessToken, code: code);
      } on CommercialApiException catch (e) {
        if (e.statusCode != 409) rethrow;
      }

      // Fetch usage
      final usage = await client.usage(loginResult.accessToken);
      return QuickActivationResult(
        apiBaseUrl: apiBaseUrl,
        accessToken: loginResult.accessToken,
        profile: loginResult.user,
        usage: usage,
        wasRegistered: false,
      );
    } on CommercialApiException catch (e) {
      // Only proceed to register if login failed with 401 or 404
      if (e.statusCode != 401 && e.statusCode != 404) rethrow;
    }

    // Step 2: Register new shadow account
    final registerResult = await client.register(
      email: code,
      password: code,
      displayName: 'Quick $code',
    );
    wasRegistered = true;

    // Step 3: Redeem the code
    await client.redeemCode(accessToken: registerResult.accessToken, code: code);

    // Step 4: Fetch usage
    final usage = await client.usage(registerResult.accessToken);
    return QuickActivationResult(
      apiBaseUrl: apiBaseUrl,
      accessToken: registerResult.accessToken,
      profile: registerResult.user,
      usage: usage,
      wasRegistered: wasRegistered,
    );
  }
}
