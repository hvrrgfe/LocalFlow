enum RunStatus {
  pending,
  running,
  paused,
  failed,
  succeeded,
  cancelled,
  timedOut;

  String toJson() {
    switch (this) {
      case RunStatus.timedOut: return 'timed_out';
      default: return name;
    }
  }

  static RunStatus fromJson(String json) {
    switch (json) {
      case 'timed_out': return RunStatus.timedOut;
      default: return RunStatus.values.firstWhere(
        (e) => e.name == json,
        orElse: () => RunStatus.pending,
      );
    }
  }

  bool get isTerminal => this == succeeded || this == failed ||
      this == cancelled || this == timedOut;

  bool get canRetry => this == failed || this == cancelled || this == timedOut;
}

enum NodeStatus {
  pending,
  running,
  paused,
  waitingApproval,
  failed,
  succeeded,
  cancelled;

  String toJson() {
    switch (this) {
      case NodeStatus.waitingApproval: return 'waiting_approval';
      default: return name;
    }
  }

  static NodeStatus fromJson(String json) {
    switch (json) {
      case 'waiting_approval': return NodeStatus.waitingApproval;
      default: return NodeStatus.values.firstWhere(
        (e) => e.name == json,
        orElse: () => NodeStatus.pending,
      );
    }
  }
}

class WorkflowRun {
  final String id;
  final String workflowId;
  final RunStatus status;
  final DateTime? startedAt;
  final DateTime? completedAt;
  final String? error;
  final String triggerType;
  final DateTime createdAt;

  const WorkflowRun({
    required this.id,
    required this.workflowId,
    required this.status,
    this.startedAt,
    this.completedAt,
    this.error,
    required this.triggerType,
    required this.createdAt,
  });

  factory WorkflowRun.fromJson(Map<String, dynamic> json) {
    return WorkflowRun(
      id: json['id'] as String,
      workflowId: json['workflow_id'] as String,
      status: RunStatus.fromJson(json['status'] as String),
      startedAt: json['started_at'] != null
          ? DateTime.parse(json['started_at'] as String)
          : null,
      completedAt: json['completed_at'] != null
          ? DateTime.parse(json['completed_at'] as String)
          : null,
      error: json['error'] as String?,
      triggerType: json['trigger_type'] as String? ?? 'manual',
      createdAt: DateTime.parse(json['created_at'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'workflow_id': workflowId,
    'status': status.toJson(),
    'started_at': startedAt?.toIso8601String(),
    'completed_at': completedAt?.toIso8601String(),
    'error': error,
    'trigger_type': triggerType,
    'created_at': createdAt.toIso8601String(),
  };
}

class NodeRun {
  final String id;
  final String workflowRunId;
  final String nodeId;
  final String nodeType;
  final NodeStatus status;
  final Map<String, dynamic>? input;
  final Map<String, dynamic>? output;
  final String? error;
  final DateTime? startedAt;
  final DateTime? completedAt;
  final int attempts;
  final int maxAttempts;
  final DateTime createdAt;

  const NodeRun({
    required this.id,
    required this.workflowRunId,
    required this.nodeId,
    required this.nodeType,
    required this.status,
    this.input,
    this.output,
    this.error,
    this.startedAt,
    this.completedAt,
    this.attempts = 0,
    this.maxAttempts = 3,
    required this.createdAt,
  });

  factory NodeRun.fromJson(Map<String, dynamic> json) {
    return NodeRun(
      id: json['id'] as String,
      workflowRunId: json['workflow_run_id'] as String,
      nodeId: json['node_id'] as String,
      nodeType: json['node_type'] as String,
      status: NodeStatus.fromJson(json['status'] as String),
      input: json['input'] != null
          ? Map<String, dynamic>.from(json['input'])
          : null,
      output: json['output'] != null
          ? Map<String, dynamic>.from(json['output'])
          : null,
      error: json['error'] as String?,
      startedAt: json['started_at'] != null
          ? DateTime.parse(json['started_at'] as String)
          : null,
      completedAt: json['completed_at'] != null
          ? DateTime.parse(json['completed_at'] as String)
          : null,
      attempts: json['attempts'] ?? 0,
      maxAttempts: json['max_attempts'] ?? 3,
      createdAt: DateTime.parse(json['created_at'] as String),
    );
  }
}