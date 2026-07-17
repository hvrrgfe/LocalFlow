enum NodeType {
  start,
  input,
  model,
  httpRequest,
  condition,
  template,
  end;

  String toJson() => name;
  static NodeType fromJson(String json) {
    return NodeType.values.firstWhere(
      (e) => e.name == json,
      orElse: () => NodeType.start,
    );
  }
}

class WorkflowNode {
  final String id;
  final String workflowId;
  final NodeType nodeType;
  final String name;
  final Map<String, dynamic> config;
  final double positionX;
  final double positionY;

  const WorkflowNode({
    required this.id,
    required this.workflowId,
    required this.nodeType,
    required this.name,
    this.config = const {},
    this.positionX = 0,
    this.positionY = 0,
  });

  factory WorkflowNode.fromJson(Map<String, dynamic> json) {
    return WorkflowNode(
      id: json['id'] as String,
      workflowId: json['workflow_id'] as String,
      nodeType: NodeType.fromJson(json['node_type'] as String),
      name: json['name'] as String,
      config: Map<String, dynamic>.from(json['config'] ?? {}),
      positionX: (json['position_x'] as num?)?.toDouble() ?? 0,
      positionY: (json['position_y'] as num?)?.toDouble() ?? 0,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'workflow_id': workflowId,
    'node_type': nodeType.toJson(),
    'name': name,
    'config': config,
    'position_x': positionX,
    'position_y': positionY,
  };
}

class WorkflowEdge {
  final String id;
  final String workflowId;
  final String sourceNodeId;
  final String targetNodeId;
  final String? sourceHandle;
  final String? targetHandle;
  final String? conditionExpression;

  const WorkflowEdge({
    required this.id,
    required this.workflowId,
    required this.sourceNodeId,
    required this.targetNodeId,
    this.sourceHandle,
    this.targetHandle,
    this.conditionExpression,
  });

  factory WorkflowEdge.fromJson(Map<String, dynamic> json) {
    return WorkflowEdge(
      id: json['id'] as String,
      workflowId: json['workflow_id'] as String,
      sourceNodeId: json['source_node_id'] as String,
      targetNodeId: json['target_node_id'] as String,
      sourceHandle: json['source_handle'] as String?,
      targetHandle: json['target_handle'] as String?,
      conditionExpression: json['condition_expression'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'workflow_id': workflowId,
    'source_node_id': sourceNodeId,
    'target_node_id': targetNodeId,
    'source_handle': sourceHandle,
    'target_handle': targetHandle,
    'condition_expression': conditionExpression,
  };
}

class Workflow {
  final String id;
  final String agentId;
  final String name;
  final String? description;
  final List<WorkflowNode> nodes;
  final List<WorkflowEdge> edges;
  final DateTime createdAt;
  final DateTime updatedAt;

  const Workflow({
    required this.id,
    required this.agentId,
    required this.name,
    this.description,
    this.nodes = const [],
    this.edges = const [],
    required this.createdAt,
    required this.updatedAt,
  });

  factory Workflow.fromJson(Map<String, dynamic> json) {
    return Workflow(
      id: json['id'] as String,
      agentId: json['agent_id'] as String,
      name: json['name'] as String,
      description: json['description'] as String?,
      nodes: (json['nodes'] as List?)
          ?.map((n) => WorkflowNode.fromJson(n))
          .toList() ?? [],
      edges: (json['edges'] as List?)
          ?.map((e) => WorkflowEdge.fromJson(e))
          .toList() ?? [],
      createdAt: DateTime.parse(json['created_at'] as String),
      updatedAt: DateTime.parse(json['updated_at'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'agent_id': agentId,
    'name': name,
    'description': description,
    'nodes': nodes.map((n) => n.toJson()).toList(),
    'edges': edges.map((e) => e.toJson()).toList(),
    'created_at': createdAt.toIso8601String(),
    'updated_at': updatedAt.toIso8601String(),
  };
}

class WorkflowInput {
  final String agentId;
  final String name;
  final String? description;
  final List<WorkflowNodeInput> nodes;
  final List<WorkflowEdgeInput> edges;

  const WorkflowInput({
    required this.agentId,
    required this.name,
    this.description,
    this.nodes = const [],
    this.edges = const [],
  });

  Map<String, dynamic> toJson() => {
    'agent_id': agentId,
    'name': name,
    'description': description,
    'nodes': nodes.map((n) => n.toJson()).toList(),
    'edges': edges.map((e) => e.toJson()).toList(),
  };
}

class WorkflowNodeInput {
  final NodeType nodeType;
  final String name;
  final Map<String, dynamic> config;
  final double positionX;
  final double positionY;

  const WorkflowNodeInput({
    required this.nodeType,
    required this.name,
    this.config = const {},
    this.positionX = 0,
    this.positionY = 0,
  });

  Map<String, dynamic> toJson() => {
    'node_type': nodeType.toJson(),
    'name': name,
    'config': config,
    'position_x': positionX,
    'position_y': positionY,
  };
}

class WorkflowEdgeInput {
  final String sourceNodeId;
  final String targetNodeId;
  final String? sourceHandle;
  final String? targetHandle;
  final String? conditionExpression;

  const WorkflowEdgeInput({
    required this.sourceNodeId,
    required this.targetNodeId,
    this.sourceHandle,
    this.targetHandle,
    this.conditionExpression,
  });

  Map<String, dynamic> toJson() => {
    'source_node_id': sourceNodeId,
    'target_node_id': targetNodeId,
    'source_handle': sourceHandle,
    'target_handle': targetHandle,
    'condition_expression': conditionExpression,
  };
}