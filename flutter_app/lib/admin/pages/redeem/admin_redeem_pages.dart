import 'package:flutter/material.dart';

import '../../../shared/workspace_app.dart';

class AdminRedeemManagePage extends StatelessWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminRedeemManagePage({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  Widget build(BuildContext context) {
    return AdminRedeemManagePanel(
      apiBaseUrl: apiBaseUrl,
      adminToken: accessToken,
    );
  }
}

class AdminRedeemRecordsPage extends StatelessWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminRedeemRecordsPage({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  Widget build(BuildContext context) {
    return AdminRedeemRecordsPanel(
      apiBaseUrl: apiBaseUrl,
      adminToken: accessToken,
    );
  }
}
