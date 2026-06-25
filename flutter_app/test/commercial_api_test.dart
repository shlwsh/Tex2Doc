import 'dart:convert';

import 'package:doc_engine/commercial_api.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test('commercial API default base URL uses local doc-server port', () {
    expect(
      CommercialApiClient('').baseUri.toString(),
      'http://127.0.0.1:2624/v1/',
    );
    expect(
      CommercialApiClient('http://127.0.0.1:2624/v1/').baseUri.toString(),
      'http://127.0.0.1:2624/v1/',
    );
  });

  test('register, usage, and plans map commercial API responses', () async {
    final client = CommercialApiClient(
      'http://localhost:2624/v1/',
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
      'http://localhost:2624/v1/',
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

  test('admin redeem code batch endpoints generate and export codes', () async {
    final client = CommercialApiClient(
      'http://localhost:2624/v1/',
      httpClient: MockClient((request) async {
        expect(request.headers['authorization'], 'Bearer demo-admin');
        if (request.url.path == '/admin/v1/redeem-code-batches' &&
            request.method == 'POST') {
          expect(request.method, 'POST');
          final body = jsonDecode(request.body) as Map<String, dynamic>;
          expect(body['package_id'], 'count_10');
          expect(body['quantity'], 2);
          return _json({
            'batch_id': 'redeem_batch_1',
            'batch_no': 'RC0001',
            'package_id': 'count_10',
            'package_name': '10 次转换包',
            'recharge_type': 'count',
            'quantity': 10,
            'generated_count': 2,
            'exported_count': 0,
            'status': 'active',
            'channel': 'web',
            'note': 'demo',
            'expires_at': null,
            'created_at': '2026-06-24T12:00:00Z',
            'codes': ['T2D-DEMO-0001', 'T2D-DEMO-0002'],
          });
        }
        if (request.url.path == '/admin/v1/redeem-code-batches' &&
            request.method == 'GET') {
          return _json([
            {
              'batch_id': 'redeem_batch_1',
              'batch_no': 'RC0001',
              'package_id': 'count_10',
              'package_name': '10 次转换包',
              'recharge_type': 'count',
              'quantity': 10,
              'generated_count': 2,
              'exported_count': 0,
              'status': 'active',
              'channel': 'web',
              'note': 'demo',
              'expires_at': null,
              'created_at': '2026-06-24T12:00:00Z',
              'codes': const [],
            },
          ]);
        }
        if (request.url.path ==
            '/admin/v1/redeem-code-batches/redeem_batch_1') {
          return _json({
            'batch_id': 'redeem_batch_1',
            'batch_no': 'RC0001',
            'package_id': 'count_10',
            'package_name': '10 次转换包',
            'recharge_type': 'count',
            'quantity': 10,
            'generated_count': 2,
            'exported_count': 0,
            'status': 'active',
            'channel': 'web',
            'note': 'demo',
            'expires_at': null,
            'created_at': '2026-06-24T12:00:00Z',
            'codes': ['T2D-DEMO-0001', 'T2D-DEMO-0002'],
          });
        }
        if (request.url.path ==
            '/admin/v1/redeem-code-batches/redeem_batch_1/export.xlsx') {
          return http.Response.bytes([0x50, 0x4b, 0x03, 0x04], 200);
        }
        return http.Response('not found', 404);
      }),
    );

    final batch = await client.createRedeemCodeBatch(
      adminToken: 'demo-admin',
      packageId: 'count_10',
      quantity: 2,
      channel: 'web',
      note: 'demo',
    );
    expect(batch.batchNo, 'RC0001');
    expect(batch.codes, hasLength(2));

    final batches = await client.redeemCodeBatches(adminToken: 'demo-admin');
    expect(batches.single.batchId, 'redeem_batch_1');
    expect(batches.single.codes, isEmpty);

    final detail = await client.redeemCodeBatchDetail(
      adminToken: 'demo-admin',
      batchId: batch.batchId,
    );
    expect(detail.codes, hasLength(2));

    final bytes = await client.exportRedeemCodeBatch(
      adminToken: 'demo-admin',
      batchId: batch.batchId,
    );
    expect(bytes, [0x50, 0x4b, 0x03, 0x04]);
  });
}

http.Response _json(Object body) {
  return http.Response(
    jsonEncode(body),
    200,
    headers: {'content-type': 'application/json'},
  );
}
