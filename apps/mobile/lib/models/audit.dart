class AuditLog {
  final String id;
  final String eventType;
  final String entityType;
  final String? entityId;
  final String? user;
  final Map<String, dynamic>? details;
  final DateTime createdAt;

  const AuditLog({
    required this.id,
    required this.eventType,
    required this.entityType,
    this.entityId,
    this.user,
    this.details,
    required this.createdAt,
  });

  factory AuditLog.fromJson(Map<String, dynamic> json) {
    return AuditLog(
      id: json['id'] as String,
      eventType: json['event_type'] as String,
      entityType: json['entity_type'] as String,
      entityId: json['entity_id'] as String?,
      user: json['user'] as String?,
      details: json['details'] != null
          ? Map<String, dynamic>.from(json['details'])
          : null,
      createdAt: DateTime.parse(json['created_at'] as String),
    );
  }
}