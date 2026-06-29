import 'package:flutter/material.dart';

import '../../../commercial_api.dart';
import '../../../ui/app_components.dart';
import '../../../ui/app_i18n.dart';
import '../../../ui/app_tokens.dart';
import 'automation_request_list.dart';
import 'automation_agent_list.dart';
import 'automation_summary_cards.dart';

class AdminAutomationPanel extends StatefulWidget {
  final String apiBaseUrl;
  final String accessToken;

  const AdminAutomationPanel({
    super.key,
    required this.apiBaseUrl,
    required this.accessToken,
  });

  @override
  State<AdminAutomationPanel> createState() => _AdminAutomationPanelState();
}

class _AdminAutomationPanelState extends State<AdminAutomationPanel>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  late final CommercialApiClient _api;

  AutomationSummary? _summary;
  List<AutomationRequest> _requests = [];
  List<AutomationAgent> _agents = [];
  bool _loading = true;
  String? _error;
  bool _autoRefresh = false;

  // Filters
  String _statusFilter = 'all';
  String _riskFilter = 'all';
  String _sourceFilter = 'all';
  String _searchQuery = '';

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _api = CommercialApiClient(widget.apiBaseUrl);
    _loadData();
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  Future<void> _loadData() async {
    if (!mounted) return;
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final results = await Future.wait([
        _api.adminAutomationSummary(widget.accessToken),
        _api.adminAutomationRequests(
          widget.accessToken,
          status: _statusFilter == 'all' ? null : _statusFilter,
          riskLevel: _riskFilter == 'all' ? null : _riskFilter,
          sourceType: _sourceFilter == 'all' ? null : _sourceFilter,
          search: _searchQuery.isEmpty ? null : _searchQuery,
        ),
        _api.adminAutomationAgents(widget.accessToken),
      ]);

      if (!mounted) return;
      setState(() {
        _summary = results[0] as AutomationSummary;
        _requests = results[1] as List<AutomationRequest>;
        _agents = results[2] as List<AutomationAgent>;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = e.toString();
      });
    }
  }

  Future<void> _refresh() async {
    await _loadData();
  }

  void _onFilterChanged() {
    _loadData();
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppStrings.of(context);
    final theme = Theme.of(context);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Header
        Row(
          children: [
            Expanded(
              child: AppSectionHeader(
                title: strings.t('nav.automation'),
                description:
                    'AI-powered automated development workflow management',
              ),
            ),
            const SizedBox(width: AppSpacing.md),
            IconButton(
              icon: Icon(_autoRefresh ? Icons.sync : Icons.sync_outlined),
              tooltip: 'Auto-refresh',
              onPressed: () {
                setState(() {
                  _autoRefresh = !_autoRefresh;
                });
              },
            ),
            IconButton(
              icon: const Icon(Icons.refresh),
              tooltip: strings.t('common.refresh'),
              onPressed: _loading ? null : _refresh,
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.md),

        // Summary cards
        if (_summary != null) ...[
          AutomationSummaryCards(summary: _summary!),
          const SizedBox(height: AppSpacing.lg),
        ],

        // Error state
        if (_error != null) ...[
          ErrorState(message: _error!),
          const SizedBox(height: AppSpacing.lg),
        ],

        // Tab bar
        TabBar(
          controller: _tabController,
          tabs: [
            Tab(
              icon: const Icon(Icons.list_alt),
              text: strings.t('automation.requests'),
            ),
            Tab(
              icon: const Icon(Icons.terminal),
              text: strings.t('automation.agents'),
            ),
            Tab(
              icon: const Icon(Icons.history),
              text: strings.t('automation.history'),
            ),
          ],
          labelColor: theme.colorScheme.primary,
          unselectedLabelColor: theme.colorScheme.onSurfaceVariant,
        ),
        const SizedBox(height: AppSpacing.md),

        // Tab content
        Expanded(
          child: _loading
              ? const LoadingState(label: 'Loading...')
              : TabBarView(
                  controller: _tabController,
                  children: [
                    // Requests tab
                    AutomationRequestList(
                      requests: _requests,
                      accessToken: widget.accessToken,
                      api: _api,
                      statusFilter: _statusFilter,
                      riskFilter: _riskFilter,
                      sourceFilter: _sourceFilter,
                      searchQuery: _searchQuery,
                      onStatusFilterChanged: (v) {
                        setState(() => _statusFilter = v);
                        _onFilterChanged();
                      },
                      onRiskFilterChanged: (v) {
                        setState(() => _riskFilter = v);
                        _onFilterChanged();
                      },
                      onSourceFilterChanged: (v) {
                        setState(() => _sourceFilter = v);
                        _onFilterChanged();
                      },
                      onSearchChanged: (v) {
                        setState(() => _searchQuery = v);
                        _onFilterChanged();
                      },
                      onActionCompleted: _refresh,
                    ),
                    // Agents tab
                    AutomationAgentList(
                      agents: _agents,
                      api: _api,
                      accessToken: widget.accessToken,
                      onActionCompleted: _refresh,
                    ),
                    // History tab
                    _buildHistoryTab(),
                  ],
                ),
        ),
      ],
    );
  }

  Widget _buildHistoryTab() {
    return Center(child: EmptyState(label: 'No data'));
  }
}
