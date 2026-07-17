import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/mod.dart';
import '../providers/mod.dart';
import '../widgets/mod.dart';

class WorkflowEditorPage extends StatefulWidget {
  final String agentId;

  const WorkflowEditorPage({super.key, required this.agentId});

  @override
  State<WorkflowEditorPage> createState() => _WorkflowEditorPageState();
}

class _WorkflowEditorPageState extends State<WorkflowEditorPage> {
  final _nodeTypes = ['start', 'input', 'model', 'http_request', 'condition', 'template', 'end'];
  final Map<String, String> _nodeLabels = {
    'start': '开始', 'input': '输入', 'model': '模型',
    'http_request': 'HTTP', 'condition': '条件', 'template': '模板', 'end': '结束',
  };

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<WorkflowProvider>().loadWorkflows(agentId: widget.agentId);
    });
  }

  Future<void> _createWorkflow() async {
    final provider = context.read<WorkflowProvider>();
    await provider.createWorkflow(WorkflowInput(
      agentId: widget.agentId,
      name: '默认工作流',
    ));
  }

  Future<void> _addNode(Workflow wf, String nodeType) async {
    final provider = context.read<WorkflowProvider>();
    final nodes = List<WorkflowNodeInput>.from(
      wf.nodes.map((n) => WorkflowNodeInput(
        nodeType: n.nodeType,
        name: n.name,
        config: n.config,
        positionX: n.positionX,
        positionY: n.positionY,
      )),
    );
    nodes.add(WorkflowNodeInput(
      nodeType: NodeType.fromJson(nodeType),
      name: '${nodeType}_${nodes.length + 1}',
      positionX: 100 + (nodes.length % 4) * 200,
      positionY: (nodes.length ~/ 4) * 150,
    ));
    await provider.updateWorkflow(wf.id, WorkflowInput(
      agentId: wf.agentId,
      name: wf.name,
      description: wf.description,
      nodes: nodes,
      edges: wf.edges.map((e) => WorkflowEdgeInput(
        sourceNodeId: e.sourceNodeId,
        targetNodeId: e.targetNodeId,
      )).toList(),
    ));
  }

  Future<void> _removeNode(Workflow wf, String nodeId) async {
    final provider = context.read<WorkflowProvider>();
    await provider.updateWorkflow(wf.id, WorkflowInput(
      agentId: wf.agentId,
      name: wf.name,
      nodes: wf.nodes
          .where((n) => n.id != nodeId)
          .map((n) => WorkflowNodeInput(
            nodeType: n.nodeType, name: n.name,
            config: n.config, positionX: n.positionX, positionY: n.positionY,
          )).toList(),
      edges: wf.edges
          .where((e) => e.sourceNodeId != nodeId && e.targetNodeId != nodeId)
          .map((e) => WorkflowEdgeInput(
            sourceNodeId: e.sourceNodeId, targetNodeId: e.targetNodeId,
          )).toList(),
    ));
  }

  Future<void> _startRun(String workflowId) async {
    await context.read<WorkflowProvider>().startRun(workflowId);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('工作流编辑器')),
      body: Consumer<WorkflowProvider>(
        builder: (context, provider, _) {
          if (provider.loading) return const LoadingIndicator();

          if (provider.workflows.isEmpty) {
            return EmptyState(
              icon: Icons.account_tree_outlined,
              title: '还没有工作流',
              actionLabel: '创建默认工作流',
              onAction: _createWorkflow,
            );
          }

          final wf = provider.selectedWorkflow ?? provider.workflows.first;
          if (wf == null) return const LoadingIndicator();

          return Column(
            children: [
              ErrorBanner(message: provider.error, onDismiss: provider.clearError),
              // Run status
              if (provider.runs.isNotEmpty)
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(8),
                  color: Colors.blue[50],
                  child: Row(
                    children: [
                      RunStatusBadge(status: provider.runs.first.status),
                      const SizedBox(width: 8),
                      if (provider.runs.first.error != null)
                        Expanded(child: Text(provider.runs.first.error!, style: const TextStyle(fontSize: 12, color: Colors.red))),
                    ],
                  ),
                ),
              // Toolbar
              Container(
                padding: const EdgeInsets.all(8),
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      const Text('添加: ', style: TextStyle(fontSize: 13)),
                      ..._nodeTypes.map((nt) => Padding(
                        padding: const EdgeInsets.only(right: 4),
                        child: ActionChip(
                          label: Text('+${_nodeLabels[nt] ?? nt}', style: const TextStyle(fontSize: 11)),
                          onPressed: () => _addNode(wf, nt),
                          visualDensity: VisualDensity.compact,
                        ),
                      )),
                      const Spacer(),
                      IconButton(
                        icon: const Icon(Icons.play_arrow, color: Colors.green),
                        onPressed: () => _startRun(wf.id),
                        tooltip: '运行工作流',
                      ),
                    ],
                  ),
                ),
              ),
              // Node list
              Expanded(
                child: wf.nodes.isEmpty
                    ? const EmptyState(icon: Icons.add_box_outlined, title: '工作流为空，点击上方按钮添加节点')
                    : RefreshIndicator(
                        onRefresh: () => provider.loadWorkflows(agentId: widget.agentId),
                        child: ListView.builder(
                          padding: const EdgeInsets.all(12),
                          itemCount: wf.nodes.length,
                          itemBuilder: (context, index) {
                            final node = wf.nodes[index];
                            return Card(
                              child: ListTile(
                                leading: Container(
                                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                                  decoration: BoxDecoration(
                                    color: _nodeColor(node.nodeType).withValues(alpha: 0.15),
                                    borderRadius: BorderRadius.circular(6),
                                  ),
                                  child: Text(node.nodeType.toJson(),
                                      style: TextStyle(fontSize: 10, fontWeight: FontWeight.bold, color: _nodeColor(node.nodeType))),
                                ),
                                title: Text(node.name, style: const TextStyle(fontSize: 14)),
                                trailing: IconButton(
                                  icon: const Icon(Icons.close, size: 18),
                                  onPressed: () => _removeNode(wf, node.id),
                                ),
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

  Color _nodeColor(NodeType type) {
    switch (type) {
      case NodeType.start: return Colors.green;
      case NodeType.input: return Colors.blue;
      case NodeType.model: return Colors.purple;
      case NodeType.httpRequest: return Colors.orange;
      case NodeType.condition: return Colors.red;
      case NodeType.template: return Colors.teal;
      case NodeType.end: return Colors.grey;
    }
  }
}