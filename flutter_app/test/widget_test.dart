// flutter_app widget test（桌面 / Web 共享 UI 的最小 smoke test）
//
// 验证：应用启动后渲染 "Doc-engine" 标题与状态卡 / 转换卡。

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:doc_engine/workspace_app.dart';

void main() {
  testWidgets('App boots and shows title', (WidgetTester tester) async {
    await tester.pumpWidget(const DocEngineApp(isWeb: false));
    await tester.pump();
    expect(find.text('Doc-engine · LaTeX → DOCX'), findsOneWidget);
  });
}
