import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/mod.dart';
import '../widgets/mod.dart';
import 'agent_editor_page.dart';
import 'workflow_editor_page.dart';

class AgentListPage extends StatefulWidget {
  const AgentListPage({super.key});

  @override
  State<AgentListPage> createState() => _AgentListPageState();
}

class _AgentListPageState extends State<AgentListPage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<AgentProvider>().loadAgents();
    });
  }

  Future<void> _importAgent() async {
    // File picker import would go here
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('导入功能需要 file_picker 插件')),
    );
  }

  Future<void> _deleteAgent(String id, String name) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除 Agent'),
        content: Text('确认删除「$name」？此操作不可撤销。'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('删除', style: TextStyle(color: Colors.red)),
          ),
        ],
      ),
    );
    if (confirmed == true && mounted) {
      context.read<AgentProvider>().deleteAgent(id);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Agent 管理'),
        actions: [
          IconButton(icon: const Icon(Icons.file_download_outlined), onPressed: _importAgent, tooltip: '导入'),
          IconButton(
            icon: const Icon(Icons.add),
            onPressed: () => Navigator.push(
              context, MaterialPageRoute(builder: (_) => const AgentEditorPage())),
            tooltip: '创建 Agent',
          ),
        ],
      ),
      body: Consumer<AgentProvider>(
        builder: (context, provider, _) {
          if (provider.loading) return const LoadingIndicator(message: '加载 Agent...');
          return Column(
            children: [
              ErrorBanner(message: provider.error, onDismiss: provider.clearError),
              Expanded(
                child: provider.agents.isEmpty
                    ? EmptyState(
                        icon: Icons.smart_toy_outlined,
                        title: '还没有任何 Agent',
                        subtitle: '创建第一个 Agent 开始使用 LocalFlow',
                        actionLabel: '创建 Agent',
                        onAction: () => Navigator.push(
                          context, MaterialPageRoute(builder: (_) => const AgentEditorPage())),
                      )
                    : RefreshIndicator(
                        onRefresh: provider.loadAgents,
                        child: ListView.builder(
                          padding: const EdgeInsets.all(12),
                          itemCount: provider.agents.length,
                          itemBuilder: (context, index) {
                            final agent = provider.agents[index];
                            return Card(
                              margin: const EdgeInsets.only(bottom: 8),
                              child: ListTile(
                                title: Text(agent.name, style: const TextStyle(fontWeight: FontWeight.w600)),
                                subtitle: Text(
                                  agent.description ?? '无描述',
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                  style: TextStyle(color: Colors.grey[600], fontSize: 13),
                                ),
                                trailing: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    IconButton(
                                      icon: const Icon(Icons.account_tree_outlined, size: 20),
                                      onPressed: () => Navigator.push(
                                        context, MaterialPageRoute(
                                          builder: (_) => WorkflowEditorPage(agentId: agent.id))),
                                      tooltip: '工作流',
                                    ),
                                    PopupMenuButton<String>(
                                      onSelected: (value) {
                                        switch (value) {
                                          case 'edit':
                                            Navigator.push(context, MaterialPageRoute(
                                              builder: (_) => AgentEditorPage(agentId: agent.id)));
                                          case 'delete':
                                            _deleteAgent(agent.id, agent.name);
                                        }
                                      },
                                      itemBuilder: (_) => [
                                        const PopupMenuItem(value: 'edit', child: Text('编辑')),
                                        const PopupMenuItem(
                                          value: 'delete',
                                          child: Text('删除', style: TextStyle(color: Colors.red)),
                                        ),
                                      ],
                                    ),
                                  ],
                                ),
                                onTap: () => Navigator.push(context, MaterialPageRoute(
                                  builder: (_) => AgentEditorPage(agentId: agent.id))),
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
}