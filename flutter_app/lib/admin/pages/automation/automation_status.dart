import 'package:flutter/material.dart';

class AutomationStatusInfo {
  final String label;
  final Color color;
  final IconData icon;

  const AutomationStatusInfo({
    required this.label,
    required this.color,
    required this.icon,
  });
}

AutomationStatusInfo getAutomationStatusInfo(String status) {
  return switch (status) {
    'submitted' => const AutomationStatusInfo(
      label: 'Submitted',
      color: Colors.grey,
      icon: Icons.inbox_outlined,
    ),
    'triaged' => const AutomationStatusInfo(
      label: 'Triaged',
      color: Colors.blue,
      icon: Icons.manage_search_outlined,
    ),
    'needs_approval' => const AutomationStatusInfo(
      label: 'Needs Approval',
      color: Colors.orange,
      icon: Icons.rule_folder_outlined,
    ),
    'queued_for_dev' => const AutomationStatusInfo(
      label: 'Queued',
      color: Colors.grey,
      icon: Icons.schedule,
    ),
    'claimed' => const AutomationStatusInfo(
      label: 'Claimed',
      color: Colors.blue,
      icon: Icons.account_tree_outlined,
    ),
    'coding' => const AutomationStatusInfo(
      label: 'Coding',
      color: Colors.blue,
      icon: Icons.code,
    ),
    'local_validating' => const AutomationStatusInfo(
      label: 'Validating',
      color: Colors.cyan,
      icon: Icons.science_outlined,
    ),
    'local_failed' => const AutomationStatusInfo(
      label: 'Local Failed',
      color: Colors.red,
      icon: Icons.error_outline,
    ),
    'pr_open' => const AutomationStatusInfo(
      label: 'PR Open',
      color: Colors.purple,
      icon: Icons.merge,
    ),
    'ci_running' => const AutomationStatusInfo(
      label: 'CI Running',
      color: Colors.cyan,
      icon: Icons.sync,
    ),
    'ci_failed' => const AutomationStatusInfo(
      label: 'CI Failed',
      color: Colors.deepOrange,
      icon: Icons.cancel,
    ),
    'ready_for_merge' => const AutomationStatusInfo(
      label: 'Ready',
      color: Colors.green,
      icon: Icons.check_circle_outline,
    ),
    'production_deployed' => const AutomationStatusInfo(
      label: 'Deployed',
      color: Colors.green,
      icon: Icons.verified_outlined,
    ),
    'notified' => const AutomationStatusInfo(
      label: 'Notified',
      color: Colors.green,
      icon: Icons.notifications_active,
    ),
    'needs_human' => const AutomationStatusInfo(
      label: 'Needs Human',
      color: Colors.orange,
      icon: Icons.support_agent,
    ),
    'blocked' => const AutomationStatusInfo(
      label: 'Blocked',
      color: Colors.deepOrange,
      icon: Icons.block,
    ),
    'closed' => const AutomationStatusInfo(
      label: 'Closed',
      color: Colors.grey,
      icon: Icons.archive,
    ),
    'rejected' => const AutomationStatusInfo(
      label: 'Rejected',
      color: Colors.red,
      icon: Icons.close,
    ),
    _ => AutomationStatusInfo(
      label: status,
      color: Colors.grey,
      icon: Icons.help_outline,
    ),
  };
}

bool canAutoApprove(String riskLevel) {
  return riskLevel != 'high' && riskLevel != 'critical';
}

bool canRetry(String status) {
  return status == 'local_failed' || status == 'ci_failed' || status == 'blocked';
}

bool canApprove(String status) {
  return status == 'triaged' || status == 'needs_approval';
}

bool canReject(String status) {
  return status != 'rejected' &&
      status != 'closed' &&
      status != 'production_deployed' &&
      status != 'notified';
}
