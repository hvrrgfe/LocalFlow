import 'package:flutter/foundation.dart';
import '../models/mod.dart';
import '../services/mod.dart';

class ProviderManager extends ChangeNotifier {
  List<ProviderInfo> _providers = [];
  List<SecretInfo> _secrets = [];
  bool _loading = false;
  String? _error;

  List<ProviderInfo> get providers => _providers;
  List<SecretInfo> get secrets => _secrets;
  bool get loading => _loading;
  String? get error => _error;

  Future<void> loadProviders() async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      final agents = await DatabaseService.listAgents();
      final results = <ProviderInfo>[];
      final seen = <String>{};

      for (final agent in agents) {
        if (agent.model != null && seen.add(agent.id)) {
          final keyExists = await SecretService.secretExists('provider/${agent.id}');
          results.add(ProviderInfo(
            id: agent.id,
            name: '${agent.name} (${agent.model})',
            providerType: 'openai_compatible',
            baseUrl: 'https://api.openai.com/v1',
            hasApiKey: keyExists,
          ));
        }
      }

      // Default providers
      const defaults = ['openai', 'deepseek', 'custom'];
      for (final key in defaults) {
        if (seen.add(key)) {
          final keyExists = await SecretService.secretExists('provider/$key');
          results.add(ProviderInfo(
            id: key,
            name: key,
            providerType: 'openai_compatible',
            baseUrl: key == 'deepseek'
                ? 'https://api.deepseek.com/v1'
                : key == 'openai'
                    ? 'https://api.openai.com/v1'
                    : 'https://api.example.com/v1',
            hasApiKey: keyExists,
          ));
        }
      }

      _providers = results;
    } catch (e) {
      _error = '加载 Provider 失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<void> saveApiKey(String providerId, String apiKey) async {
    try {
      await SecretService.storeSecret('provider/$providerId', apiKey);
      await DatabaseService.writeAuditLog(
        eventType: 'api_key_stored',
        entityType: 'provider',
        entityId: providerId,
        details: {'provider_id': providerId},
      );
      await loadProviders();
    } catch (e) {
      _error = '保存 API Key 失败: $e';
      notifyListeners();
    }
  }

  Future<void> deleteApiKey(String providerId) async {
    try {
      await SecretService.deleteSecret('provider/$providerId');
      await loadProviders();
    } catch (e) {
      _error = '删除 API Key 失败: $e';
      notifyListeners();
    }
  }

  /// Validate a URL for SSRF safety
  static bool isValidUrl(String url) {
    try {
      final uri = Uri.parse(url);
      final host = uri.host.toLowerCase();

      // Block localhost and private IPs
      if (host == 'localhost' || host == '127.0.0.1' || host == '0.0.0.0') {
        return false;
      }
      if (host.startsWith('10.') || host.startsWith('172.16.') || host.startsWith('192.168.')) {
        return false;
      }
      // Block cloud metadata
      if (host.endsWith('169.254.169.254') || host == '169.254.169.254') {
        return false;
      }
      // Block file protocol
      if (uri.scheme == 'file') {
        return false;
      }
      return uri.scheme == 'https' || uri.scheme == 'http';
    } catch (_) {
      return false;
    }
  }
}