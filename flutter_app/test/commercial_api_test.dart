import 'dart:convert';

import 'package:doc_engine/commercial_api.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test('register, usage, and plans map commercial API responses', () async {
    final client = CommercialApiClient(
      'http://localhost:8080/v1/',
      httpClient: MockClient((request) async {
        if (request.url.path == '/v1/auth/register') {
          expect(request.method, 'POST');
          return _json({
            'access_token': 'demo-access',
            'refresh_token': 'demo-refresh',
            'user': {
              'id': 'user_demo',
              'email': 'demo@example.com',
              'display_name': 'Demo',
              'plan_id': 'preview',
            },
          });
        }
        if (request.url.path == '/v1/usage') {
          expect(request.headers['authorization'], 'Bearer demo-access');
          return _json({
            'plan_id': 'preview',
            'cloud_conversions_used': 3,
            'cloud_conversions_limit': 100,
            'storage_bytes_used': 0,
            'storage_bytes_limit': 1073741824,
          });
        }
        if (request.url.path == '/v1/plans') {
          return _json([
            {
              'id': 'pro',
              'name': 'Pro',
              'price_cents': 2900,
              'currency': 'USD',
              'monthly_conversions': 1000,
            },
          ]);
        }
        return http.Response('not found', 404);
      }),
    );

    final auth = await client.register(
      email: 'demo@example.com',
      password: 'secret',
      displayName: 'Demo',
    );
    expect(auth.accessToken, 'demo-access');
    expect(auth.user.planId, 'preview');

    final usage = await client.usage(auth.accessToken);
    expect(usage.cloudConversionsRemaining, 97);

    final plans = await client.plans();
    expect(plans.single.label, 'pro: USD 29.00/mo, 1000 conversions');
  });
}

http.Response _json(Object body) {
  return http.Response(
    jsonEncode(body),
    200,
    headers: {'content-type': 'application/json'},
  );
}
