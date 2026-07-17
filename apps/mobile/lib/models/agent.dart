class PermissionPolicy {
  final List<String> allowedHosts;
  final List<String> allowedNetworks;
  final bool allowFileAccess;
  final bool allowLoopback;
  final int maxNodes;
  final int maxLoops;
  final int maxRequestSize;
  final int maxResponseSize;
  final int maxExecutionSeconds;

  const PermissionPolicy({
    this.allowedHosts = const [],
    this.allowedNetworks = const [],
    this.allowFileAccess = false,
    this.allowLoopback = false,
    this.maxNodes = 50,
    this.maxLoops = 10,
    this.maxRequestSize = 10485760,
    this.maxResponseSize = 10485760,
    this.maxExecutionSeconds = 600,
  });

  factory PermissionPolicy.fromJson(Map<String, dynamic> json) {
    return PermissionPolicy(
      allowedHosts: List<String>.from(json['allowed_hosts'] ?? []),
      allowedNetworks: List<String>.from(json['allowed_networks'] ?? []),
      allowFileAccess: json['allow_file_access'] ?? false,
      allowLoopback: json['allow_loopback'] ?? false,
      maxNodes: json['max_nodes'] ?? 50,
      maxLoops: json['max_loops'] ?? 10,
      maxRequestSize: json['max_request_size'] ?? 10485760,
      maxResponseSize: json['max_response_size'] ?? 10485760,
      maxExecutionSeconds: json['max_execution_seconds'] ?? 600,
    );
  }

  Map<String, dynamic> toJson() => {
    'allowed_hosts': allowedHosts,
    'allowed_networks': allowedNetworks,
    'allow_file_access': allowFileAccess,
    'allow_loopback': allowLoopback,
    'max_nodes': maxNodes,
    'max_loops': maxLoops,
    'max_request_size': maxRequestSize,
    'max_response_size': maxResponseSize,
    'max_execution_seconds': maxExecutionSeconds,
  };
}

class Agent {
  final String id;
  final String name;
  final String? description;
  final String? systemPrompt;
  final String? model;
  final double? temperature;
  final int? maxTokens;
  final PermissionPolicy permissions;
  final DateTime createdAt;
  final DateTime updatedAt;

  const Agent({
    required this.id,
    required this.name,
    this.description,
    this.systemPrompt,
    this.model,
    this.temperature,
    this.maxTokens,
    required this.permissions,
    required this.createdAt,
    required this.updatedAt,
  });

  factory Agent.fromJson(Map<String, dynamic> json) {
    return Agent(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String?,
      systemPrompt: json['system_prompt'] as String?,
      model: json['model'] as String?,
      temperature: (json['temperature'] as num?)?.toDouble(),
      maxTokens: json['max_tokens'] as int?,
      permissions: PermissionPolicy.fromJson(json['permissions'] ?? {}),
      createdAt: DateTime.parse(json['created_at'] as String),
      updatedAt: DateTime.parse(json['updated_at'] as String),
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'description': description,
    'system_prompt': systemPrompt,
    'model': model,
    'temperature': temperature,
    'max_tokens': maxTokens,
    'permissions': permissions.toJson(),
    'created_at': createdAt.toIso8601String(),
    'updated_at': updatedAt.toIso8601String(),
  };
}

class AgentInput {
  final String name;
  final String? description;
  final String? systemPrompt;
  final String? model;
  final double? temperature;
  final int? maxTokens;
  final PermissionPolicy? permissions;

  const AgentInput({
    required this.name,
    this.description,
    this.systemPrompt,
    this.model,
    this.temperature,
    this.maxTokens,
    this.permissions,
  });

  Map<String, dynamic> toJson() => {
    'name': name,
    'description': description,
    'system_prompt': systemPrompt,
    'model': model,
    'temperature': temperature,
    'max_tokens': maxTokens,
    'permissions': permissions?.toJson(),
  };
}