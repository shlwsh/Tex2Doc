// flutter_app widget test（桌面 / Web 共享 UI 的最小 smoke test）
//
// 验证：应用启动后渲染 "Doc-engine" 标题与状态卡 / 转换卡。

import 'package:flutter_test/flutter_test.dart';

import 'package:doc_engine/product/product_home_app.dart';
import 'package:doc_engine/workspace_app.dart';

void main() {
  testWidgets('App boots and shows title', (WidgetTester tester) async {
    await tester.pumpWidget(const DocEngineApp(isWeb: false));
    await tester.pump();
    expect(find.text('Tex2Doc'), findsWidgets);
  });

  testWidgets('Product home exposes user and admin entries', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(const ProductHomeApp());
    await tester.pump();
    expect(find.text('Tex2Doc'), findsWidgets);
    expect(find.text('用户登录'), findsOneWidget);
    expect(find.text('管理端'), findsWidgets);
  });

  testWidgets('Admin app boots as an independent entry', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      const DocEngineApp(isWeb: true, mode: DocEngineAppMode.admin),
    );
    await tester.pump();
    expect(find.text('Tex2Doc'), findsWidgets);
  });
}
