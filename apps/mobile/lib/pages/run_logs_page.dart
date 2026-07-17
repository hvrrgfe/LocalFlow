import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/mod.dart';
import '../providers/mod.dart';
import '../widgets/mod.dart';

class RunLogsPage extends StatefulWidget {
  final String? workflowId;
  const RunLogsPage({super.key, this.workflowId});

  @override
  State<RunLogsPage> createState() => _RunLogsPageState();
}

class _RunLogsPageState extends State<RunLogsPage> {
  String? _selectedWfId;
  WorkflowProvider get _provider => context.read<WorkflowProvider>();

  @override
  void initState() {
    super.initState();
    _selectedWfId = widget.workflowId;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _provider.loadWorkflows();
      if (_selectedWfId != null) {
        _provider.loadRuns(_selectedWfId!);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('运行日志'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () {
              if (_selectedWfId != null) _provider.loadRuns(_selectedWfId!);
            },
          ),
        ],
      ),
      body: Consumer<WorkflowProvider>(
        builder: (context, provider, _) {
          return Column(
            children: [
              ErrorBanner(message: provider.error, onDismiss: provider.clearError),
              // Workflow selector
              if (provider.workflows.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.all(8),
                  child: DropdownButtonFormField<String>(
                    value: _selectedWfId,
                    decoration: const InputDecoration(
                      labelText: '选择工作流',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    items: [
                      const DropdownMenuItem(value: null, child: Text('全部工作流')),
                      ...provider.workflows.map((wf) => DropdownMenuItem(
                        value: wf.id,
                        child: Text('${wf.name} (${wf.agentId.substring(0, 8)})', style: const TextStyle(fontSize: 13)),
                      )),
                    ],
                    onChanged: (v) {
                      setState(() => _selectedWfId = v);
                      if (v != null) provider.loadRuns(v);
                    },
                  ),
                ),
              // Run list
              Expanded(
                child: provider.loading
                    ? const LoadingIndicator()
                    : provider.runs.isEmpty
                        ? const EmptyState(icon: Icons.history, title: '没有运行记录')
                        : RefreshIndicator(
                            onRefresh: () => _selectedWfId != null
                                ? provider.loadRuns(_selectedWfId!)
                                : Future<void>.value(),
                            child: ListView.builder(
                              padding: const EdgeInsets.all(12),
                              itemCount: provider.runs.length,
                              itemBuilder: (context, index) {
                                final run = provider.runs[index];
                                return Card(
                                  margin: const EdgeInsets.only(bottom: 8),
                                  child: ExpansionTile(
                                    leading: RunStatusBadge(status: run.status),
                                    title: Text(
                                      '运行 ${run.id.substring(0, 8)}',
                                      style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
                                    ),
                                    subtitle: Text(
                                      _formatTime(run.startedAt),
                                      style: const TextStyle(fontSize: 12),
                                    ),
                                    trailing: Row(
                                      mainAxisSize: MainAxisSize.min,
                                      children: [
                                        if (run.status.canRetry)
                                          IconButton(
                                            icon: const Icon(Icons.replay, size: 18),
                                            onPressed: () {
                                              if (_selectedWfId != null) {
                                                provider.startRun(_selectedWfId!);
                                              }
                                            },
                                          ),
                                        Icon(
                                          run.status == RunStatus.succeeded
                                              ? Icons.check_circle : Icons.info_outline,
                                          size: 18, color: Colors.grey,
                                        ),
                                      ],
                                    ),
                                    children: [
                                      if (run.error != null)
                                        Padding(
                                          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
                                          child: Text('错误: ${run.error}',
                                              style: const TextStyle(color: Colors.red, fontSize: 12)),
                                        ),
                                      if (_selectedWfId != null)
                                        Padding(
                                          padding: const EdgeInsets.all(8),
                                          child: FutureBuilder<List<NodeRun>>(
                                            future: DatabaseService.getNodeRuns(run.id),
                                            builder: (ctx, snapshot) {
                                              if (!snapshot.hasData || snapshot.data!.isEmpty) {
                                                return const Padding(
                                                  padding: EdgeInsets.all(8),
                                                  child: Text('暂无节点详情', style: TextStyle(fontSize: 12, color: Colors.grey)),
                                                );
                                              }
                                              return Column(
                                                children: snapshot.data!.map((nr) => ListTile(
                                                  dense: true,
                                                  leading: NodeStatusBadge(status: nr.status),
                                                  title: Text('${nr.nodeType}(${nr.nodeId.substring(0, 6)})',
                                                      style: const TextStyle(fontSize: 12)),
                                                  subtitle: nr.error != null
                                                      ? Text(nr.error!, style: const TextStyle(fontSize: 11, color: Colors.red))
                                                      : null,
                                                  trailing: Text('${nr.attempts}/${nr.maxAttempts}',
                                                      style: const TextStyle(fontSize: 11)),
                                                )).toList(),
                                              );
                                            },
                                          ),
                                        ),
                                    ],
                                  ),
                                );
                              },
                            ),
                          ),
              ),
            ],
          );
        },
      ),
    );
  }

  String _formatTime(DateTime? dt) {
    if (dt == null) return '未开始';
    return '${dt.year}-${_pad(dt.month)}-${_pad(dt.day)} ${_pad(dt.hour)}:${_pad(dt.minute)}';
  }

  String _pad(int n) => n.toString().padLeft(2, '0');
}