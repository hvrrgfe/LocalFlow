import 'dart:convert';
import 'package:sqflite/sqflite.dart';
import 'package:path/path.dart' as p;
import 'package:uuid/uuid.dart';
import '../models/mod.dart';

/// Local SQLite database service.
/// Mirrors the Rust SQLite schema for local-first operation.
/// In production, this will be replaced by Rust Core FFI calls.
class DatabaseService {
  static Database? _db;
  static const _uuid = Uuid();

  static Future<Database> get database async {
    if (_db != null) return _db!;
    _db = await _initDatabase();
    return _db!;
  }

  static Future<Database> _initDatabase() async {
    final dbPath = await getDatabasesPath();
    final path = p.join(dbPath, 'localflow.db');

    return openDatabase(
      path,
      version: 1,
      onCreate: _createTables,
      onUpgrade: _onUpgrade,
    );
  }

  static Future<void> _createTables(Database db, int version) async {
    await db.execute('''
      CREATE TABLE IF NOT EXISTS agents (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        system_prompt TEXT,
        model TEXT,
        temperature REAL,
        max_tokens INTEGER,
        permissions TEXT NOT NULL DEFAULT '{}',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      )
    ''');

    await db.execute('''
      CREATE TABLE IF NOT EXISTS workflows (
        id TEXT PRIMARY KEY,
        agent_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
      )
    ''');

    await db.execute('''
      CREATE TABLE IF NOT EXISTS workflow_nodes (
        id TEXT PRIMARY KEY,
        workflow_id TEXT NOT NULL,
        node_type TEXT NOT NULL,
        name TEXT NOT NULL,
        config TEXT NOT NULL DEFAULT '{}',
        position_x REAL NOT NULL DEFAULT 0,
        position_y REAL NOT NULL DEFAULT 0
      )
    ''');

    await db.execute('''
      CREATE TABLE IF NOT EXISTS workflow_edges (
        id TEXT PRIMARY KEY,
        workflow_id TEXT NOT NULL,
        source_node_id TEXT NOT NULL,
        target_node_id TEXT NOT NULL,
        source_handle TEXT,
        target_handle TEXT,
        condition_expression TEXT
      )
    ''');

    await db.execute('''
      CREATE TABLE IF NOT EXISTS workflow_runs (
        id TEXT PRIMARY KEY,
        workflow_id TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        started_at TEXT,
        completed_at TEXT,
        error TEXT,
        trigger_type TEXT NOT NULL DEFAULT 'manual',
        created_at TEXT NOT NULL
      )
    ''');

    await db.execute('''
      CREATE TABLE IF NOT EXISTS node_runs (
        id TEXT PRIMARY KEY,
        workflow_run_id TEXT NOT NULL,
        node_id TEXT NOT NULL,
        node_type TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'pending',
        input TEXT,
        output TEXT,
        error TEXT,
        started_at TEXT,
        completed_at TEXT,
        attempts INTEGER NOT NULL DEFAULT 0,
        max_attempts INTEGER NOT NULL DEFAULT 3,
        created_at TEXT NOT NULL
      )
    ''');

    await db.execute('''
      CREATE TABLE IF NOT EXISTS audit_logs (
        id TEXT PRIMARY KEY,
        event_type TEXT NOT NULL,
        entity_type TEXT NOT NULL,
        entity_id TEXT,
        user TEXT,
        details TEXT,
        created_at TEXT NOT NULL
      )
    ''');
  }

  static Future<void> _onUpgrade(Database db, int oldVersion, int newVersion) async {
    // Future migrations
  }

  // ========== Agents ==========

  static Future<List<Agent>> listAgents() async {
    final db = await database;
    final rows = await db.query('agents', orderBy: 'updated_at DESC');
    return rows.map((r) => _rowToAgent(r)).toList();
  }

  static Future<Agent?> getAgent(String id) async {
    final db = await database;
    final rows = await db.query('agents', where: 'id = ?', whereArgs: [id]);
    if (rows.isEmpty) return null;
    return _rowToAgent(rows.first);
  }

  static Future<Agent> createAgent(AgentInput input) async {
    final db = await database;
    final id = _uuid.v4();
    final now = DateTime.now().toUtc().toIso8601String();
    final permissions = (input.permissions ?? const PermissionPolicy()).toJson();

    await db.insert('agents', {
      'id': id,
      'name': input.name,
      'description': input.description,
      'system_prompt': input.systemPrompt,
      'model': input.model,
      'temperature': input.temperature,
      'max_tokens': input.maxTokens,
      'permissions': jsonEncode(permissions),
      'created_at': now,
      'updated_at': now,
    });

    return (await getAgent(id))!;
  }

  static Future<Agent> updateAgent(String id, AgentInput input) async {
    final db = await database;
    final now = DateTime.now().toUtc().toIso8601String();
    final permissions = (input.permissions ?? const PermissionPolicy()).toJson();

    await db.update(
      'agents',
      {
        'name': input.name,
        'description': input.description,
        'system_prompt': input.systemPrompt,
        'model': input.model,
        'temperature': input.temperature,
        'max_tokens': input.maxTokens,
        'permissions': jsonEncode(permissions),
        'updated_at': now,
      },
      where: 'id = ?',
      whereArgs: [id],
    );

    return (await getAgent(id))!;
  }

  static Future<void> deleteAgent(String id) async {
    final db = await database;
    await db.delete('agents', where: 'id = ?', whereArgs: [id]);
    // Cascading deletes handled by app logic
    await db.delete('workflows', where: 'agent_id = ?', whereArgs: [id]);
  }

  static Agent _rowToAgent(Map<String, dynamic> row) {
    return Agent(
      id: row['id'] as String,
      name: row['name'] as String,
      description: row['description'] as String?,
      systemPrompt: row['system_prompt'] as String?,
      model: row['model'] as String?,
      temperature: (row['temperature'] as num?)?.toDouble(),
      maxTokens: row['max_tokens'] as int?,
      permissions: PermissionPolicy.fromJson(
        jsonDecode(row['permissions'] as String? ?? '{}'),
      ),
      createdAt: DateTime.parse(row['created_at'] as String),
      updatedAt: DateTime.parse(row['updated_at'] as String),
    );
  }

  // ========== Workflows ==========

  static Future<List<Workflow>> listWorkflows({String? agentId}) async {
    final db = await database;
    final where = agentId != null ? 'agent_id = ?' : null;
    final whereArgs = agentId != null ? [agentId] : null;
    final rows = await db.query('workflows',
        where: where, whereArgs: whereArgs, orderBy: 'updated_at DESC');
    final results = <Workflow>[];
    for (final row in rows) {
      results.add(await _rowToWorkflow(row));
    }
    return results;
  }

  static Future<Workflow?> getWorkflow(String id) async {
    final db = await database;
    final rows = await db.query('workflows', where: 'id = ?', whereArgs: [id]);
    if (rows.isEmpty) return null;
    return _rowToWorkflow(rows.first);
  }

  static Future<Workflow> createWorkflow(WorkflowInput input) async {
    final db = await database;
    final wfId = _uuid.v4();
    final now = DateTime.now().toUtc().toIso8601String();

    await db.insert('workflows', {
      'id': wfId,
      'agent_id': input.agentId,
      'name': input.name,
      'description': input.description,
      'created_at': now,
      'updated_at': now,
    });

    for (final nodeInput in input.nodes) {
      final nodeId = _uuid.v4();
      await db.insert('workflow_nodes', {
        'id': nodeId,
        'workflow_id': wfId,
        'node_type': nodeInput.nodeType.toJson(),
        'name': nodeInput.name,
        'config': jsonEncode(nodeInput.config),
        'position_x': nodeInput.positionX,
        'position_y': nodeInput.positionY,
      });
    }

    for (final edgeInput in input.edges) {
      await db.insert('workflow_edges', {
        'id': _uuid.v4(),
        'workflow_id': wfId,
        'source_node_id': edgeInput.sourceNodeId,
        'target_node_id': edgeInput.targetNodeId,
        'source_handle': edgeInput.sourceHandle,
        'target_handle': edgeInput.targetHandle,
        'condition_expression': edgeInput.conditionExpression,
      });
    }

    return (await getWorkflow(wfId))!;
  }

  static Future<Workflow> updateWorkflow(String id, WorkflowInput input) async {
    final db = await database;
    final now = DateTime.now().toUtc().toIso8601String();

    await db.update(
      'workflows',
      {'name': input.name, 'description': input.description, 'updated_at': now},
      where: 'id = ?',
      whereArgs: [id],
    );

    // Replace nodes
    await db.delete('workflow_nodes', where: 'workflow_id = ?', whereArgs: [id]);
    await db.delete('workflow_edges', where: 'workflow_id = ?', whereArgs: [id]);

    for (final nodeInput in input.nodes) {
      await db.insert('workflow_nodes', {
        'id': _uuid.v4(),
        'workflow_id': id,
        'node_type': nodeInput.nodeType.toJson(),
        'name': nodeInput.name,
        'config': jsonEncode(nodeInput.config),
        'position_x': nodeInput.positionX,
        'position_y': nodeInput.positionY,
      });
    }

    for (final edgeInput in input.edges) {
      await db.insert('workflow_edges', {
        'id': _uuid.v4(),
        'workflow_id': id,
        'source_node_id': edgeInput.sourceNodeId,
        'target_node_id': edgeInput.targetNodeId,
        'source_handle': edgeInput.sourceHandle,
        'target_handle': edgeInput.targetHandle,
        'condition_expression': edgeInput.conditionExpression,
      });
    }

    return (await getWorkflow(id))!;
  }

  static Future<void> deleteWorkflow(String id) async {
    final db = await database;
    await db.delete('workflow_nodes', where: 'workflow_id = ?', whereArgs: [id]);
    await db.delete('workflow_edges', where: 'workflow_id = ?', whereArgs: [id]);
    await db.delete('workflow_runs', where: 'workflow_id = ?', whereArgs: [id]);
    await db.delete('workflows', where: 'id = ?', whereArgs: [id]);
  }

  static Future<Workflow> _rowToWorkflow(Map<String, dynamic> row) async {
    final db = await database;
    final nodes = await db.query('workflow_nodes',
        where: 'workflow_id = ?', whereArgs: [row['id']]);
    final edges = await db.query('workflow_edges',
        where: 'workflow_id = ?', whereArgs: [row['id']]);

    return Workflow(
      id: row['id'] as String,
      agentId: row['agent_id'] as String,
      name: row['name'] as String,
      description: row['description'] as String?,
      nodes: nodes.map((n) => WorkflowNode(
        id: n['id'] as String,
        workflowId: n['workflow_id'] as String,
        nodeType: NodeType.fromJson(n['node_type'] as String),
        name: n['name'] as String,
        config: jsonDecode(n['config'] as String? ?? '{}'),
        positionX: (n['position_x'] as num?)?.toDouble() ?? 0,
        positionY: (n['position_y'] as num?)?.toDouble() ?? 0,
      )).toList(),
      edges: edges.map((e) => WorkflowEdge(
        id: e['id'] as String,
        workflowId: e['workflow_id'] as String,
        sourceNodeId: e['source_node_id'] as String,
        targetNodeId: e['target_node_id'] as String,
        sourceHandle: e['source_handle'] as String?,
        targetHandle: e['target_handle'] as String?,
        conditionExpression: e['condition_expression'] as String?,
      )).toList(),
      createdAt: DateTime.parse(row['created_at'] as String),
      updatedAt: DateTime.parse(row['updated_at'] as String),
    );
  }

  // ========== Runs ==========

  static Future<List<WorkflowRun>> listRuns(String workflowId) async {
    final db = await database;
    final rows = await db.query('workflow_runs',
        where: 'workflow_id = ?',
        whereArgs: [workflowId],
        orderBy: 'created_at DESC');
    return rows.map((r) => _rowToRun(r)).toList();
  }

  static Future<WorkflowRun> createRun(String workflowId) async {
    final db = await database;
    final id = _uuid.v4();
    final now = DateTime.now().toUtc().toIso8601String();

    await db.insert('workflow_runs', {
      'id': id,
      'workflow_id': workflowId,
      'status': 'running',
      'started_at': now,
      'trigger_type': 'manual',
      'created_at': now,
    });

    return (await getRun(id))!;
  }

  static Future<WorkflowRun> getRun(String id) async {
    final db = await database;
    final rows = await db.query('workflow_runs', where: 'id = ?', whereArgs: [id]);
    return _rowToRun(rows.first);
  }

  static Future<void> updateRunStatus(
      String id, RunStatus status, {String? error}) async {
    final db = await database;
    final now = DateTime.now().toUtc().toIso8601String();
    final updates = <String, dynamic>{
      'status': status.toJson(),
      if (error != null) 'error': error,
    };
    if (status.isTerminal) {
      updates['completed_at'] = now;
    }
    await db.update('workflow_runs', updates,
        where: 'id = ?', whereArgs: [id]);
  }

  static WorkflowRun _rowToRun(Map<String, dynamic> row) {
    return WorkflowRun(
      id: row['id'] as String,
      workflowId: row['workflow_id'] as String,
      status: RunStatus.fromJson(row['status'] as String),
      startedAt: row['started_at'] != null
          ? DateTime.parse(row['started_at'] as String)
          : null,
      completedAt: row['completed_at'] != null
          ? DateTime.parse(row['completed_at'] as String)
          : null,
      error: row['error'] as String?,
      triggerType: row['trigger_type'] as String? ?? 'manual',
      createdAt: DateTime.parse(row['created_at'] as String),
    );
  }

  // ========== Node Runs ==========

  static Future<List<NodeRun>> getNodeRuns(String runId) async {
    final db = await database;
    final rows = await db.query('node_runs',
        where: 'workflow_run_id = ?', whereArgs: [runId]);
    return rows.map((r) => NodeRun(
      id: r['id'] as String,
      workflowRunId: r['workflow_run_id'] as String,
      nodeId: r['node_id'] as String,
      nodeType: r['node_type'] as String,
      status: NodeStatus.fromJson(r['status'] as String),
      input: r['input'] != null
          ? jsonDecode(r['input'] as String)
          : null,
      output: r['output'] != null
          ? jsonDecode(r['output'] as String)
          : null,
      error: r['error'] as String?,
      startedAt: r['started_at'] != null
          ? DateTime.parse(r['started_at'] as String)
          : null,
      completedAt: r['completed_at'] != null
          ? DateTime.parse(r['completed_at'] as String)
          : null,
      attempts: r['attempts'] ?? 0,
      maxAttempts: r['max_attempts'] ?? 3,
      createdAt: DateTime.parse(r['created_at'] as String),
    )).toList();
  }

  // ========== Audit ==========

  static Future<List<AuditLog>> getAuditLogs({int limit = 50}) async {
    final db = await database;
    final rows = await db.query('audit_logs',
        orderBy: 'created_at DESC', limit: limit);
    return rows.map((r) => AuditLog(
      id: r['id'] as String,
      eventType: r['event_type'] as String,
      entityType: r['entity_type'] as String,
      entityId: r['entity_id'] as String?,
      user: r['user'] as String?,
      details: r['details'] != null
          ? jsonDecode(r['details'] as String)
          : null,
      createdAt: DateTime.parse(r['created_at'] as String),
    )).toList();
  }

  static Future<void> writeAuditLog({
    required String eventType,
    required String entityType,
    String? entityId,
    String? user,
    Map<String, dynamic>? details,
  }) async {
    final db = await database;
    await db.insert('audit_logs', {
      'id': _uuid.v4(),
      'event_type': eventType,
      'entity_type': entityType,
      'entity_id': entityId,
      'user': user,
      'details': details != null ? jsonEncode(details) : null,
      'created_at': DateTime.now().toUtc().toIso8601String(),
    });
  }
}