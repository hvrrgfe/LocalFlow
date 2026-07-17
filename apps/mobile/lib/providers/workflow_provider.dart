import 'package:flutter/foundation.dart';
import '../models/mod.dart';
import '../services/mod.dart';

class WorkflowProvider extends ChangeNotifier {
  List<Workflow> _workflows = [];
  Workflow? _selectedWorkflow;
  List<WorkflowRun> _runs = [];
  List<NodeRun> _nodeRuns = [];
  bool _loading = false;
  String? _error;

  List<Workflow> get workflows => _workflows;
  Workflow? get selectedWorkflow => _selectedWorkflow;
  List<WorkflowRun> get runs => _runs;
  List<NodeRun> get nodeRuns => _nodeRuns;
  bool get loading => _loading;
  String? get error => _error;

  Future<void> loadWorkflows({String? agentId}) async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _workflows = await DatabaseService.listWorkflows(agentId: agentId);
    } catch (e) {
      _error = '加载工作流失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<void> loadWorkflow(String id) async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _selectedWorkflow = await DatabaseService.getWorkflow(id);
    } catch (e) {
      _error = '加载工作流失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<Workflow?> createWorkflow(WorkflowInput input) async {
    try {
      final wf = await DatabaseService.createWorkflow(input);
      await DatabaseService.writeAuditLog(
        eventType: 'workflow_created',
        entityType: 'workflow',
        entityId: wf.id,
      );
      await loadWorkflows(agentId: input.agentId);
      return wf;
    } catch (e) {
      _error = '创建工作流失败: $e';
      notifyListeners();
      return null;
    }
  }

  Future<bool> updateWorkflow(String id, WorkflowInput input) async {
    try {
      await DatabaseService.updateWorkflow(id, input);
      await DatabaseService.writeAuditLog(
        eventType: 'workflow_updated',
        entityType: 'workflow',
        entityId: id,
      );
      await loadWorkflow(id);
      return true;
    } catch (e) {
      _error = '更新工作流失败: $e';
      notifyListeners();
      return false;
    }
  }

  Future<bool> deleteWorkflow(String id) async {
    try {
      await DatabaseService.deleteWorkflow(id);
      await DatabaseService.writeAuditLog(
        eventType: 'workflow_deleted',
        entityType: 'workflow',
        entityId: id,
      );
      _selectedWorkflow = null;
      return true;
    } catch (e) {
      _error = '删除工作流失败: $e';
      notifyListeners();
      return false;
    }
  }

  // ========== Runs ==========

  Future<void> loadRuns(String workflowId) async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _runs = await DatabaseService.listRuns(workflowId);
    } catch (e) {
      _error = '加载运行记录失败: $e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<WorkflowRun?> startRun(String workflowId) async {
    try {
      final run = await DatabaseService.createRun(workflowId);
      await DatabaseService.writeAuditLog(
        eventType: 'run_started',
        entityType: 'workflow_run',
        entityId: run.id,
      );
      await loadRuns(workflowId);
      return run;
    } catch (e) {
      _error = '启动运行失败: $e';
      notifyListeners();
      return null;
    }
  }

  Future<void> cancelRun(String runId, String workflowId) async {
    try {
      await DatabaseService.updateRunStatus(runId, RunStatus.cancelled,
          error: '用户取消');
      await loadRuns(workflowId);
    } catch (e) {
      _error = '取消运行失败: $e';
      notifyListeners();
    }
  }

  Future<void> loadNodeRuns(String runId) async {
    try {
      _nodeRuns = await DatabaseService.getNodeRuns(runId);
      notifyListeners();
    } catch (e) {
      _error = '加载节点运行记录失败: $e';
      notifyListeners();
    }
  }

  void clearError() {
    _error = null;
    notifyListeners();
  }
}