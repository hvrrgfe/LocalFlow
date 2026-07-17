import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../models/mod.dart';
import '../providers/mod.dart';

class AgentEditorPage extends StatefulWidget {
  final String? agentId;

  const AgentEditorPage({super.key, this.agentId});

  @override
  State<AgentEditorPage> createState() => _AgentEditorPageState();
}

class _AgentEditorPageState extends State<AgentEditorPage> {
  final _formKey = GlobalKey<FormState>();
  final _nameController = TextEditingController();
  final _descController = TextEditingController();
  final _promptController = TextEditingController();
  final _modelController = TextEditingController();

  double _temperature = 0.7;
  int _maxTokens = 4096;
  int _maxNodes = 50;
  int _maxExecutionSeconds = 600;
  bool _allowFileAccess = false;
  bool _allowLoopback = false;
  bool _saving = false;
  bool _isNew = true;

  @override
  void initState() {
    super.initState();
    _isNew = widget.agentId == null;
    if (!_isNew) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _loadAgent());
    }
  }

  Future<void> _loadAgent() async {
    final provider = context.read<AgentProvider>();
    await provider.loadAgent(widget.agentId!);
    if (!mounted) return;
    final agent = provider.selectedAgent;
    if (agent != null) {
      _nameController.text = agent.name;
      _descController.text = agent.description ?? '';
      _promptController.text = agent.systemPrompt ?? '';
      _modelController.text = agent.model ?? '';
      _temperature = agent.temperature ?? 0.7;
      _maxTokens = agent.maxTokens ?? 4096;
      _maxNodes = agent.permissions.maxNodes;
      _maxExecutionSeconds = agent.permissions.maxExecutionSeconds;
      _allowFileAccess = agent.permissions.allowFileAccess;
      _allowLoopback = agent.permissions.allowLoopback;
      setState(() {});
    }
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() => _saving = true);

    final input = AgentInput(
      name: _nameController.text.trim(),
      description: _descController.text.trim().isEmpty ? null : _descController.text.trim(),
      systemPrompt: _promptController.text.trim().isEmpty ? null : _promptController.text.trim(),
      model: _modelController.text.trim().isEmpty ? null : _modelController.text.trim(),
      temperature: _temperature,
      maxTokens: _maxTokens,
      permissions: PermissionPolicy(
        maxNodes: _maxNodes,
        maxExecutionSeconds: _maxExecutionSeconds,
        allowFileAccess: _allowFileAccess,
        allowLoopback: _allowLoopback,
      ),
    );

    final provider = context.read<AgentProvider>();
    bool success;
    if (_isNew) {
      final agent = await provider.createAgent(input);
      success = agent != null;
    } else {
      success = await provider.updateAgent(widget.agentId!, input);
    }

    setState(() => _saving = false);
    if (success && mounted) {
      Navigator.pop(context);
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _descController.dispose();
    _promptController.dispose();
    _modelController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_isNew ? '创建 Agent' : '编辑 Agent'),
        actions: [
          TextButton(
            onPressed: _saving ? null : _save,
            child: _saving
                ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                : const Text('保存'),
          ),
        ],
      ),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            TextFormField(
              controller: _nameController,
              decoration: const InputDecoration(labelText: '名称 *', border: OutlineInputBorder()),
              validator: (v) => (v == null || v.trim().isEmpty) ? '名称不能为空' : null,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _descController,
              decoration: const InputDecoration(labelText: '描述', border: OutlineInputBorder()),
              maxLines: 2,
            ),
            const SizedBox(height: 12),
            TextFormField(
              controller: _promptController,
              decoration: const InputDecoration(
                labelText: '系统提示词',
                border: OutlineInputBorder(),
              ),
              maxLines: 6,
              style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
            ),
            const SizedBox(height: 16),
            const Text('模型配置', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
            const SizedBox(height: 8),
            TextFormField(
              controller: _modelController,
              decoration: const InputDecoration(
                labelText: '模型 ID',
                hintText: 'gpt-4o, deepseek-chat',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: _buildSlider('Temperature', _temperature, 0, 2, 0.1, (v) => setState(() => _temperature = v)),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _buildNumberField('Max Tokens', _maxTokens, 1, 128000, (v) => setState(() => _maxTokens = v)),
                ),
              ],
            ),
            const SizedBox(height: 16),
            const Text('权限策略', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(child: _buildNumberField('最大节点数', _maxNodes, 1, 200, (v) => setState(() => _maxNodes = v))),
                const SizedBox(width: 12),
                Expanded(child: _buildNumberField('最大执行(秒)', _maxExecutionSeconds, 10, 3600, (v) => setState(() => _maxExecutionSeconds = v))),
              ],
            ),
            const SizedBox(height: 8),
            SwitchListTile(
              title: const Text('允许文件访问（危险）', style: TextStyle(fontSize: 14)),
              value: _allowFileAccess,
              onChanged: (v) => setState(() => _allowFileAccess = v),
            ),
            SwitchListTile(
              title: const Text('允许本地回环地址（危险）', style: TextStyle(fontSize: 14)),
              value: _allowLoopback,
              onChanged: (v) => setState(() => _allowLoopback = v),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildSlider(String label, double value, double min, double max, double step, ValueChanged<double> onChanged) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(label, style: const TextStyle(fontSize: 13)),
        Row(
          children: [
            Expanded(
              child: Slider(value: value, min: min, max: max, divisions: ((max - min) / step).round(), onChanged: onChanged),
            ),
            Text(value.toStringAsFixed(1), style: const TextStyle(fontSize: 13)),
          ],
        ),
      ],
    );
  }

  Widget _buildNumberField(String label, int value, int min, int max, ValueChanged<int> onChanged) {
    return TextFormField(
      initialValue: value.toString(),
      decoration: InputDecoration(labelText: label, border: const OutlineInputBorder(), isDense: true),
      keyboardType: TextInputType.number,
      onChanged: (v) {
        final n = int.tryParse(v);
        if (n != null && n >= min && n <= max) onChanged(n);
      },
    );
  }
}