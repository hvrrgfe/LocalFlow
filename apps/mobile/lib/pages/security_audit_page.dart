import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import '../providers/mod.dart';
import '../widgets/mod.dart';
import '../services/mod.dart';

class SecurityAuditPage extends StatefulWidget {
  const SecurityAuditPage({super.key});

  @override
  State<SecurityAuditPage> createState() => _SecurityAuditPageState();
}

class _SecurityAuditPageState extends State<SecurityAuditPage> {
  final _urlController = TextEditingController();
  String? _urlResult;
  bool? _urlValid;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<AuditProvider>().loadLogs();
    });
  }

  void _checkUrl() {
    final url = _urlController.text.trim();
    if (url.isEmpty) return;
    final valid = ProviderManager.isValidUrl(url);
    setState(() {
      _urlValid = valid;
      _urlResult = valid ? 'URL 安全，允许访问' : 'URL 已阻止（本地/内网/元数据地址）';
    });
  }

  @override
  void dispose() {
    _urlController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('安全审计'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () => context.read<AuditProvider>().loadLogs(),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(12),
        children: [
          // URL checker
          Card(
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('URL 安全检查器', style: TextStyle(fontWeight: FontWeight.w600)),
                  const SizedBox(height: 4),
                  const Text('检查 URL 是否允许访问，防止 SSRF 攻击',
                      style: TextStyle(fontSize: 12, color: Colors.grey)),
                  const SizedBox(height: 8),
                  Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _urlController,
                          decoration: const InputDecoration(
                            hintText: '输入 URL 进行检查',
                            border: OutlineInputBorder(),
                            isDense: true,
                          ),
                          onSubmitted: (_) => _checkUrl(),
                        ),
                      ),
                      const SizedBox(width: 8),
                      ElevatedButton(onPressed: _checkUrl, child: const Text('检查')),
                    ],
                  ),
                  if (_urlResult != null)
                    Container(
                      margin: const EdgeInsets.only(top: 8),
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: _urlValid! ? Colors.green[50] : Colors.red[50],
                        borderRadius: BorderRadius.circular(6),
                      ),
                      child: Row(
                        children: [
                          Icon(
                            _urlValid! ? Icons.check_circle : Icons.cancel,
                            size: 18,
                            color: _urlValid! ? Colors.green : Colors.red,
                          ),
                          const SizedBox(width: 8),
                          Text(_urlResult!,
                              style: TextStyle(
                                color: _urlValid! ? Colors.green[800] : Colors.red[800],
                                fontSize: 13,
                              )),
                        ],
                      ),
                    ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
          // Audit logs
          const Text('安全审计日志', style: TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
          const SizedBox(height: 4),
          const Text('记录所有敏感操作，日志不会泄露密钥',
              style: TextStyle(fontSize: 12, color: Colors.grey)),
          const SizedBox(height: 8),
          Consumer<AuditProvider>(
            builder: (context, provider, _) {
              if (provider.loading) return const LoadingIndicator();
              if (provider.logs.isEmpty) {
                return const EmptyState(icon: Icons.security, title: '尚无审计日志');
              }
              return Column(
                children: provider.logs.map((log) => Card(
                  margin: const EdgeInsets.only(bottom: 6),
                  child: ListTile(
                    dense: true,
                    leading: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                      decoration: BoxDecoration(
                        color: Colors.indigo[50],
                        borderRadius: BorderRadius.circular(4),
                      ),
                      child: Text(log.eventType, style: const TextStyle(fontSize: 10, fontFamily: 'monospace')),
                    ),
                    title: Text(
                      '${log.entityType}${log.entityId != null ? " / ${log.entityId!.substring(0, 8)}" : ""}',
                      style: const TextStyle(fontSize: 13),
                    ),
                    subtitle: Text(
                      '${log.createdAt.year}-${_pad(log.createdAt.month)}-${_pad(log.createdAt.day)} '
                      '${_pad(log.createdAt.hour)}:${_pad(log.createdAt.minute)}',
                      style: const TextStyle(fontSize: 11),
                    ),
                    trailing: log.details != null
                        ? Icon(Icons.info_outline, size: 16, color: Colors.grey[400])
                        : null,
                    onTap: log.details != null
                        ? () => showDialog(
                            context: context,
                            builder: (ctx) => AlertDialog(
                              title: const Text('日志详情'),
                              content: Text(log.details.toString()),
                              actions: [
                                TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('关闭')),
                              ],
                            ),
                          )
                        : null,
                  ),
                )).toList(),
              );
            },
          ),
        ],
      ),
    );
  }

  String _pad(int n) => n.toString().padLeft(2, '0');
}