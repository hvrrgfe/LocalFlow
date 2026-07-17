import 'package:flutter/foundation.dart';
import '../models/mod.dart';
import '../services/mod.dart';

class AgentProvider extends ChangeNotifier {
  List<Agent> _agents = [];
  Agent? _selectedAgent;
  bool _loading = false;
  String? _error;

  List<Agent> get agents => _agents;
  Agent? get selectedAgent => _selectedAgent;
  bool get loading => _loading;
  String? get error => _error;

  Future<void> loadAgents() async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _agents = await DatabaseService.listAgents();
    } catch (e) {
      _error = '加载 Agent 失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<void> loadAgent(String id) async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _selectedAgent = await DatabaseService.getAgent(id);
    } catch (e) {
      _error = '加载 Agent 失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<Agent?> createAgent(AgentInput input) async {
    try {
      final agent = await DatabaseService.createAgent(input);
      await DatabaseService.writeAuditLog(
        eventType: 'agent_created',
        entityType: 'agent',
        entityId: agent.id,
      );
      await loadAgents();
      return agent;
    } catch (e) {
      _error = '创建 Agent 失败: $e';
      notifyListeners();
      return null;
    }
  }

  Future<bool> updateAgent(String id, AgentInput input) async {
    try {
      await DatabaseService.updateAgent(id, input);
      await DatabaseService.writeAuditLog(
        eventType: 'agent_updated',
        entityType: 'agent',
        entityId: id,
      );
      await loadAgent(id);
      await loadAgents();
      return true;
    } catch (e) {
      _error = '更新 Agent 失败: $e';
      notifyListeners();
      return false;
    }
  }

  Future<bool> deleteAgent(String id) async {
    try {
      await DatabaseService.deleteAgent(id);
      await DatabaseService.writeAuditLog(
        eventType: 'agent_deleted',
        entityType: 'agent',
        entityId: id,
      );
      _selectedAgent = null;
      await loadAgents();
      return true;
    } catch (e) {
      _error = '删除 Agent 失败: $e';
      notifyListeners();
      return false;
    }
  }

  Future<String?> exportAgent(String id) async {
    try {
      final agent = await DatabaseService.getAgent(id);
      if (agent == null) return null;
      // Export without API keys, secrets, or internal IDs
      final export = {
        'name': agent.name,
        'description': agent.description,
        'system_prompt': agent.systemPrompt,
        'model': agent.model,
        'temperature': agent.temperature,
        'max_tokens': agent.maxTokens,
      };
      return _encodeJson(export);
    } catch (e) {
      _error = '导出失败: $e';
      notifyListeners();
      return null;
    }
  }

  Future<Agent?> importAgent(String jsonData) async {
    try {
      final data = _decodeJson(jsonData);
      final input = AgentInput(
        name: data['name'] ?? 'Imported Agent',
        description: data['description'],
        systemPrompt: data['system_prompt'],
        model: data['model'],
        temperature: (data['temperature'] as num?)?.toDouble(),
        maxTokens: data['max_tokens'] as int?,
      );
      return createAgent(input);
    } catch (e) {
      _error = '导入失败: $e';
      notifyListeners();
      return null;
    }
  }

  void clearError() {
    _error = null;
    notifyListeners();
  }

  String _encodeJson(Map<String, dynamic> data) {
    // Manual JSON encoding to avoid json_serializable dependency in prod
    final parts = <String>[];
    data.forEach((key, value) {
      if (value != null) {
        final encoded = value is String
            ? '"${value.replaceAll('"', '\\"').replaceAll('\n', '\\n')}"'
            : '$value';
        parts.add('"$key": $encoded');
      }
    });
    return '{${parts.join(", ")}}';
  }

  Map<String, dynamic> _decodeJson(String json) {
    // Simple JSON parser
    final result = <String, dynamic>{};
    final regex = RegExp(r'"([^"]+)":\s*("([^"]*)"|([^,}\s]+))');
    for (final match in regex.allMatches(json)) {
      final key = match.group(1)!;
      final value = match.group(3) ?? match.group(4) ?? '';
      result[key] = value;
    }
    return result;
  }
}