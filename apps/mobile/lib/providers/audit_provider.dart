import 'package:flutter/foundation.dart';
import '../models/mod.dart';
import '../services/mod.dart';

class AuditProvider extends ChangeNotifier {
  List<AuditLog> _logs = [];
  bool _loading = false;
  String? _error;

  List<AuditLog> get logs => _logs;
  bool get loading => _loading;
  String? get error => _error;

  Future<void> loadLogs({int limit = 50}) async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _logs = await DatabaseService.getAuditLogs(limit: limit);
    } catch (e) {
      _error = '加载审计日志失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }
}