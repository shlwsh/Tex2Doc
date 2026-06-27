// Quick session model - stores the state of an activated quick assistant session.

import '../commercial_api.dart';

class QuickSession {
  final String apiBaseUrl;
  final String accessToken;
  final UserProfile profile;
  final String redeemCode;
  final UsageSummary usage;

  const QuickSession({
    required this.apiBaseUrl,
    required this.accessToken,
    required this.profile,
    required this.redeemCode,
    required this.usage,
  });

  QuickSession copyWith({UsageSummary? usage}) => QuickSession(
    apiBaseUrl: apiBaseUrl,
    accessToken: accessToken,
    profile: profile,
    redeemCode: redeemCode,
    usage: usage ?? this.usage,
  );
}
