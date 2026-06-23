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

  test('redeem code endpoints map commercial API responses', () async {
    final client = CommercialApiClient(
      'http://localhost:8080/v1/',
      httpClient: MockClient((request) async {
        expect(request.headers['authorization'], 'Bearer demo-access');
        if (request.url.path == '/v1/redeem-codes/options') {
          return _json({
            'enabled': true,
            'provider': 'redeem-code',
            'code_format_hint': 'T2D-XXXX-XXXX-XXXX-XX',
            'support_text': 'Enter code',
            'packages': [
              {
                'id': 'count_10',
                'name': '10 次转换包',
                'recharge_type': 'count',
                'quantity': 10,
              },
            ],
          });
        }
        if (request.url.path == '/v1/redeem-codes/redeem') {
          expect(request.method, 'POST');
          final body = jsonDecode(request.body) as Map<String, dynamic>;
          expect(body['code'], 'T2D-DEMO-CODE');
          return _json({
            'redeem_id': 'redeem_1',
            'recharge_id': 'recharge_1',
            'package_id': 'count_10',
            'package_name': '10 次转换包',
            'recharge_type': 'count',
            'quantity': 10,
            'count_balance': 12,
            'date_valid_until': null,
            'redeemed_at': '2026-06-23T15:01:00Z',
          });
        }
        if (request.url.path == '/v1/redeem-codes/records') {
          return _json([
            {
              'redeem_id': 'redeem_1',
              'batch_id': 'batch_1',
              'batch_no': 'RC001',
              'code_preview': 'T2DDEMO****CODE',
              'package_id': 'count_10',
              'package_name': '10 次转换包',
              'recharge_type': 'count',
              'quantity': 10,
              'status': 'redeemed',
              'redeemed_recharge_id': 'recharge_1',
              'redeemed_at': '2026-06-23T15:01:00Z',
            },
          ]);
        }
        return http.Response('not found', 404);
      }),
    );

    final options = await client.redeemCodeOptions('demo-access');
    expect(options.enabled, isTrue);
    expect(options.packages.single.id, 'count_10');

    final result = await client.redeemCode(
      accessToken: 'demo-access',
      code: 'T2D-DEMO-CODE',
    );
    expect(result.quantity, 10);
    expect(result.countBalance, 12);

    final records = await client.redeemCodeRecords('demo-access');
    expect(records.single.codePreview, 'T2DDEMO****CODE');
  });
}

http.Response _json(Object body) {
  return http.Response(
    jsonEncode(body),
    200,
    headers: {'content-type': 'application/json'},
  );
}
