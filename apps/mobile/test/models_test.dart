import 'package:flutter_test/flutter_test.dart';
import 'package:localflow/models/mod.dart';

void main() {
  group('Agent model', () {
    test('Agent.fromJson parses correctly', () {
      final json = {
        'id': '550e8400-e29b-41d4-a716-446655440000',
        'name': 'Test Agent',
        'description': 'A test agent',
        'system_prompt': 'You are helpful',
        'model': 'gpt-4o',
        'temperature': 0.7,
        'max_tokens': 4096,
        'permissions': {
          'allowed_hosts': [],
          'allowed_networks': [],
          'allow_file_access': false,
          'allow_loopback': false,
          'max_nodes': 50,
          'max_loops': 10,
          'max_request_size': 10485760,
          'max_response_size': 10485760,
          'max_execution_seconds': 600,
        },
        'created_at': '2025-01-01T00:00:00Z',
        'updated_at': '2025-01-01T00:00:00Z',
      };

      final agent = Agent.fromJson(json);
      expect(agent.name, 'Test Agent');
      expect(agent.model, 'gpt-4o');
      expect(agent.permissions.maxNodes, 50);
    });

    test('AgentInput toJson excludes nulls implicitly', () {
      final input = AgentInput(name: 'New Agent');
      final json = input.toJson();
      expect(json['name'], 'New Agent');
    });
  });

  group('Workflow model', () {
    test('Workflow.fromJson parses nodes and edges', () {
      final json = {
        'id': 'wf-1',
        'agent_id': 'agent-1',
        'name': 'Test WF',
        'nodes': [
          {
            'id': 'n1', 'workflow_id': 'wf-1',
            'node_type': 'start', 'name': 'Start',
            'config': {}, 'position_x': 0, 'position_y': 0,
          },
          {
            'id': 'n2', 'workflow_id': 'wf-1',
            'node_type': 'end', 'name': 'End',
            'config': {}, 'position_x': 200, 'position_y': 0,
          },
        ],
        'edges': [],
        'created_at': '2025-01-01T00:00:00Z',
        'updated_at': '2025-01-01T00:00:00Z',
      };

      final wf = Workflow.fromJson(json);
      expect(wf.nodes.length, 2);
      expect(wf.nodes[0].nodeType, NodeType.start);
    });
  });

  group('RunStatus', () {
    test('RunStatus.fromJson parses all variants', () {
      expect(RunStatus.fromJson('pending'), RunStatus.pending);
      expect(RunStatus.fromJson('running'), RunStatus.running);
      expect(RunStatus.fromJson('succeeded'), RunStatus.succeeded);
      expect(RunStatus.fromJson('failed'), RunStatus.failed);
      expect(RunStatus.fromJson('cancelled'), RunStatus.cancelled);
      expect(RunStatus.fromJson('timed_out'), RunStatus.timedOut);
    });

    test('RunStatus.isTerminal is correct', () {
      expect(RunStatus.succeeded.isTerminal, true);
      expect(RunStatus.failed.isTerminal, true);
      expect(RunStatus.cancelled.isTerminal, true);
      expect(RunStatus.timedOut.isTerminal, true);
      expect(RunStatus.running.isTerminal, false);
      expect(RunStatus.pending.isTerminal, false);
    });
  });
}