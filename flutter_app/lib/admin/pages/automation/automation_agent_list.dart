import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_tokens.dart';

class AutomationAgentList extends StatelessWidget {
  final List<AutomationAgent> agents;
  final CommercialApiClient api;
  final String accessToken;
  final VoidCallback onActionCompleted;

  const AutomationAgentList({
    super.key,
    required this.agents,
    required this.api,
    required this.accessToken,
    required this.onActionCompleted,
  });

  @override
  Widget build(BuildContext context) {
    if (agents.isEmpty) {
      return const EmptyState(
        label: 'No agents',
      );
    }

    return ListView.separated(
      padding: const EdgeInsets.all(AppSpacing.md),
      itemCount: agents.length,
      separatorBuilder: (_, __) => const SizedBox(height: AppSpacing.md),
      itemBuilder: (context, index) {
        final agent = agents[index];
        return _AgentCard(
          agent: agent,
          api: api,
          accessToken: accessToken,
          onActionCompleted: onActionCompleted,
        );
      },
    );
  }
}

class _AgentCard extends StatefulWidget {
  final AutomationAgent agent;
  final CommercialApiClient api;
  final String accessToken;
  final VoidCallback onActionCompleted;

  const _AgentCard({
    required this.agent,
    required this.api,
    required this.accessToken,
    required this.onActionCompleted,
  });

  @override
  State<_AgentCard> createState() => _AgentCardState();
}

class _AgentCardState extends State<_AgentCard> {
  bool _actionInProgress = false;

  Color get _statusColor => switch (widget.agent.status) {
    'online' => Colors.green,
    'busy' => Colors.blue,
    'paused' => Colors.orange,
    _ => Colors.grey,
  };

  IconData get _statusIcon => switch (widget.agent.status) {
    'online' => Icons.check_circle,
    'busy' => Icons.work,
    'paused' => Icons.pause_circle,
    _ => Icons.offline_bolt,
  };

  Future<void> _pause() async {
    setState(() => _actionInProgress = true);
    try {
      await widget.api.adminAutomationPauseAgent(widget.accessToken, widget.agent.id);
      widget.onActionCompleted();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e.toString()), backgroundColor: Colors.red),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _actionInProgress = false);
      }
    }
  }

  Future<void> _resume() async {
    setState(() => _actionInProgress = true);
    try {
      await widget.api.adminAutomationResumeAgent(widget.accessToken, widget.agent.id);
      widget.onActionCompleted();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(e.toString()), backgroundColor: Colors.red),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _actionInProgress = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final agent = widget.agent;

    return AppCard(
      child: Padding(
        padding: const EdgeInsets.all(AppSpacing.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                // Status indicator
                Icon(_statusIcon, color: _statusColor, size: 20),
                const SizedBox(width: AppSpacing.sm),

                // Agent ID
                Text(
                  agent.id,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontFamily: 'monospace',
                  ),
                ),
                const Spacer(),

                // Actions
                if (agent.status == 'online' || agent.status == 'busy')
                  OutlinedButton.icon(
                    onPressed: _actionInProgress ? null : _pause,
                    icon: _actionInProgress
                        ? const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.pause, size: 16),
                    label: const Text('Pause'),
                  )
                else if (agent.status == 'paused')
                  FilledButton.icon(
                    onPressed: _actionInProgress ? null : _resume,
                    icon: _actionInProgress
                        ? const SizedBox(
                            width: 14,
                            height: 14,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              color: Colors.white,
                            ),
                          )
                        : const Icon(Icons.play_arrow, size: 16),
                    label: const Text('Resume'),
                  ),
              ],
            ),
            const SizedBox(height: AppSpacing.md),

            // Info grid
            Wrap(
              spacing: AppSpacing.lg,
              runSpacing: AppSpacing.sm,
              children: [
                _InfoItem(
                  icon: Icons.computer,
                  label: 'Hostname',
                  value: agent.hostname,
                ),
                _InfoItem(
                  icon: Icons.tag,
                  label: 'Version',
                  value: agent.agentVersion,
                ),
                _InfoItem(
                  icon: Icons.access_time,
                  label: 'Last Heartbeat',
                  value: _formatTime(agent.lastHeartbeatAt),
                ),
                _InfoItem(
                  icon: Icons.check_circle_outline,
                  label: 'Completed',
                  value: '${agent.totalTasksCompleted}',
                ),
                _InfoItem(
                  icon: Icons.error_outline,
                  label: 'Failed',
                  value: '${agent.totalTasksFailed}',
                ),
                _InfoItem(
                  icon: Icons.trending_up,
                  label: 'Success Rate',
                  value: '${agent.successRate.toStringAsFixed(1)}%',
                ),
              ],
            ),

            // Current task
            if (agent.currentRequestId != null) ...[
              const SizedBox(height: AppSpacing.md),
              Container(
                padding: const EdgeInsets.all(AppSpacing.sm),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primaryContainer.withValues(alpha: 0.3),
                  borderRadius: BorderRadius.circular(AppRadius.sm),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.work,
                      size: 16,
                      color: theme.colorScheme.primary,
                    ),
                    const SizedBox(width: AppSpacing.xs),
                    Text(
                      'Current task: ${agent.currentRequestId}',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.primary,
                      ),
                    ),
                  ],
                ),
              ),
            ],

            // Capabilities
            if (agent.capabilities.isNotEmpty) ...[
              const SizedBox(height: AppSpacing.md),
              Wrap(
                spacing: AppSpacing.xs,
                runSpacing: AppSpacing.xs,
                children: agent.capabilities.entries.map((e) {
                  return Chip(
                    label: Text(
                      e.key,
                      style: theme.textTheme.labelSmall,
                    ),
                    backgroundColor: theme.colorScheme.surfaceContainerHighest,
                    padding: EdgeInsets.zero,
                    materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    visualDensity: VisualDensity.compact,
                  );
                }).toList(),
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _formatTime(String isoTime) {
    try {
      final dt = DateTime.parse(isoTime);
      final now = DateTime.now();
      final diff = now.difference(dt);

      if (diff.inMinutes < 1) return 'just now';
      if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
      if (diff.inHours < 24) return '${diff.inHours}h ago';
      if (diff.inDays < 7) return '${diff.inDays}d ago';

      return '${dt.month}/${dt.day} ${dt.hour}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return isoTime;
    }
  }
}

class _InfoItem extends StatelessWidget {
  final IconData icon;
  final String label;
  final String value;

  const _InfoItem({
    required this.icon,
    required this.label,
    required this.value,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 14, color: theme.colorScheme.onSurfaceVariant),
        const SizedBox(width: 4),
        Text(
          '$label: ',
          style: theme.textTheme.labelSmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        Text(
          value,
          style: theme.textTheme.labelSmall?.copyWith(
            fontWeight: FontWeight.w500,
          ),
        ),
      ],
    );
  }
}
