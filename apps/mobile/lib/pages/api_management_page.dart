import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/mod.dart';
import '../widgets/mod.dart';
import '../services/mod.dart';

class ApiManagementPage extends StatefulWidget {
  const ApiManagementPage({super.key});

  @override
  State<ApiManagementPage> createState() => _ApiManagementPageState();
}

class _ApiManagementPageState extends State<ApiManagementPage> {
  final _keyController = TextEditingController();
  final _valueController = TextEditingController();
  bool _savingKey = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<ProviderManager>().loadProviders();
    });
  }

  Future<void> _saveApiKey(String providerId) async {
    final apiKey = _valueController.text.trim();
    if (apiKey.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请输入 API Key'));
      );
      return;
    }
    setState(() => _savingKey = true);
    await context.read<ProviderManager>().saveApiKey(providerId, apiKey);
    setState(() {
      _savingKey = false;
      _valueController.clear();
    });
  }

  @override
  void dispose() {
    _keyController.dispose();
    _valueController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('API 管理')),
      body: Consumer<ProviderManager>(
        builder: (context, provider, _) {
          if (provider.loading) return const LoadingIndicator();
          return Column(
            children: [
              ErrorBanner(message: provider.error, onDismiss: provider.clearError),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.all(12),
                  children: [
                    // Add API Key section
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            const Text('添加 API Key', style: TextStyle(fontWeight: FontWeight.w600)),
                            const SizedBox(height: 8),
                            Row(
                              children: [
                                Expanded(
                                  child: TextField(
                                    controller: _keyController,
                                    decoration: const InputDecoration(
                                      labelText: 'Provider ID',
                                      hintText: 'openai, deepseek',
                                      border: OutlineInputBorder(),
                                      isDense: true,
                                    ),
                                  ),
                                ),
                                const SizedBox(width: 8),
                                Expanded(
                                  child: TextField(
                                    controller: _valueController,
                                    obscureText: true,
                                    decoration: const InputDecoration(
                                      labelText: 'API Key',
                                      border: OutlineInputBorder(),
                                      isDense: true,
                                    ),
                                  ),
                                ),
                                const SizedBox(width: 8),
                                ElevatedButton(
                                  onPressed: _savingKey ? null : () => _saveApiKey(_keyController.text.trim()),
                                  child: _savingKey
                                      ? const SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))
                                      : const Text('保存'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),
                    // Provider list
                    if (provider.providers.isEmpty)
                      const EmptyState(icon: Icons.key, title: '尚未配置 API Provider')
                    else
                      ...provider.providers.map((p) => Card(
                        margin: const EdgeInsets.only(bottom: 8),
                        child: ListTile(
                          title: Text(p.name, style: const TextStyle(fontWeight: FontWeight.w500)),
                          subtitle: Text(p.baseUrl, style: const TextStyle(fontSize: 12, fontFamily: 'monospace')),
                          trailing: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                p.hasApiKey ? Icons.check_circle : Icons.cancel,
                                size: 18,
                                color: p.hasApiKey ? Colors.green : Colors.grey,
                              ),
                              const SizedBox(width: 4),
                              Text(p.hasApiKey ? '已配置' : '未配置', style: const TextStyle(fontSize: 12)),
                            ],
                          ),
                          onTap: () => _showProviderDialog(p.id, p.name, p.hasApiKey),
                        ),
                      )),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  void _showProviderDialog(String id, String name, bool hasKey) {
    showModalBottomSheet(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(title: Text(name), subtitle: Text('ID: $id')),
            ListTile(
              leading: const Icon(Icons.vpn_key_outlined),
              title: const Text(hasKey ? '更换 API Key' : '添加 API Key'),
              onTap: () {
                Navigator.pop(ctx);
                _keyController.text = id;
              },
            ),
            if (hasKey)
              ListTile(
                leading: const Icon(Icons.delete_outline, color: Colors.red),
                title: const Text('删除 API Key', style: TextStyle(color: Colors.red)),
                onTap: () {
                  Navigator.pop(ctx);
                  context.read<ProviderManager>().deleteApiKey(id);
                },
              ),
          ],
        ),
      ),
    );
  }
}