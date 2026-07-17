use localflow_core::error::CoreError;
use localflow_core::models::{
    AgentInput, NodeRun, NodeStatus, PermissionPolicy, RunStatus, WorkflowNode, WorkflowRun,
};
use localflow_core::state_machine::{
    validate_node_transition, validate_run_transition,
};
use localflow_core::types::{AuditEventType, NodeType, SecretType};
use localflow_storage::{
    AgentRepository, AuditRepository, RunRepository, StorageEngine, WorkflowInput,
    WorkflowNodeInput, WorkflowRepository,
};
use serde_json::Value;
use uuid::Uuid;

fn setup_engine() -> StorageEngine {
    StorageEngine::new_in_memory().unwrap()
}

#[test]
fn test_full_agent_lifecycle() {
    let engine = setup_engine();

    // Create
    let input = AgentInput {
        name: "Integration Agent".into(),
        description: Some("Test agent".into()),
        system_prompt: Some("You are helpful".into()),
        model: Some("gpt-4".into()),
        temperature: Some(0.7),
        max_tokens: Some(4096),
        permissions: None,
    };
    let agent = engine.agents.create(input).unwrap();
    assert_eq!(agent.name, "Integration Agent");

    // Read
    let fetched = engine.agents.get(agent.id).unwrap();
    assert_eq!(fetched.system_prompt.as_deref(), Some("You are helpful"));

    // Update
    let update = AgentInput {
        name: "Updated Agent".into(),
        description: None,
        system_prompt: None,
        model: Some("gpt-4o".into()),
        temperature: None,
        max_tokens: None,
        permissions: None,
    };
    let updated = engine.agents.update(agent.id, update).unwrap();
    assert_eq!(updated.name, "Updated Agent");
    assert_eq!(updated.model.as_deref(), Some("gpt-4o"));

    // List
    let agents = engine.agents.list().unwrap();
    assert_eq!(agents.len(), 1);

    // Delete
    engine.agents.delete(agent.id).unwrap();
    let agents = engine.agents.list().unwrap();
    assert!(agents.is_empty());
}

#[test]
fn test_full_workflow_lifecycle() {
    let engine = setup_engine();
    let agent_id = Uuid::new_v4();

    // Create workflow
    let input = WorkflowInput {
        agent_id,
        name: "Integration Workflow".into(),
        description: Some("Test workflow".into()),
        nodes: vec![
            WorkflowNodeInput {
                node_type: NodeType::Start,
                name: "Start".into(),
                config: Value::Null,
                position_x: 0.0,
                position_y: 0.0,
            },
            WorkflowNodeInput {
                node_type: NodeType::Input,
                name: "User Input".into(),
                config: serde_json::json!({"prompt": "default"}),
                position_x: 100.0,
                position_y: 0.0,
            },
            WorkflowNodeInput {
                node_type: NodeType::Model,
                name: "GPT Call".into(),
                config: serde_json::json!({"model": "gpt-4", "temperature": 0.7}),
                position_x: 200.0,
                position_y: 0.0,
            },
            WorkflowNodeInput {
                node_type: NodeType::End,
                name: "End".into(),
                config: Value::Null,
                position_x: 300.0,
                position_y: 0.0,
            },
        ],
        edges: vec![],
    };

    let workflow = engine.workflows.create(input).unwrap();
    assert_eq!(workflow.name, "Integration Workflow");
    assert_eq!(workflow.nodes.len(), 4);

    // Read
    let fetched = engine.workflows.get(workflow.id).unwrap();
    assert_eq!(fetched.nodes.len(), 4);

    // List
    let workflows = engine.workflows.list(None).unwrap();
    assert_eq!(workflows.len(), 1);

    // Update
    let update_input = WorkflowInput {
        agent_id,
        name: "Updated Workflow".into(),
        description: Some("Updated".into()),
        nodes: vec![
            WorkflowNodeInput {
                node_type: NodeType::Start,
                name: "Start".into(),
                config: Value::Null,
                position_x: 0.0,
                position_y: 0.0,
            },
            WorkflowNodeInput {
                node_type: NodeType::End,
                name: "End".into(),
                config: Value::Null,
                position_x: 200.0,
                position_y: 0.0,
            },
        ],
        edges: vec![],
    };
    let updated = engine.workflows.update(workflow.id, update_input).unwrap();
    assert_eq!(updated.name, "Updated Workflow");
    assert_eq!(updated.nodes.len(), 2);

    // Delete
    engine.workflows.delete(workflow.id).unwrap();
    assert!(engine.workflows.get(workflow.id).is_err());
}

#[test]
fn test_workflow_run_lifecycle() {
    let engine = setup_engine();
    let wf_id = Uuid::new_v4();

    // Create run
    let run = engine.runs.create_run(wf_id, "manual").unwrap();
    assert_eq!(run.status, RunStatus::Pending);

    // Mark as running
    engine.runs.update_run_status(run.id, RunStatus::Running, None).unwrap();
    let fetched = engine.runs.get_run(run.id).unwrap();
    assert_eq!(fetched.status, RunStatus::Running);
    assert!(fetched.started_at.is_some());

    // Create node runs
    let node_run = engine.runs.create_node_run(run.id, Uuid::new_v4(), NodeType::Model, 3).unwrap();
    assert_eq!(node_run.status, NodeStatus::Pending);

    // Execute node
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Running,
        Some(serde_json::json!({"input": "test"})),
        None,
        None,
    ).unwrap();
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Succeeded,
        None,
        Some(serde_json::json!({"output": "result"})),
        None,
    ).unwrap();

    let completed_node = engine.runs.get_node_run(node_run.id).unwrap();
    assert_eq!(completed_node.status, NodeStatus::Succeeded);
    assert!(completed_node.completed_at.is_some());

    // Complete run
    engine.runs.update_run_status(run.id, RunStatus::Succeeded, None).unwrap();
    let completed_run = engine.runs.get_run(run.id).unwrap();
    assert_eq!(completed_run.status, RunStatus::Succeeded);
    assert!(completed_run.completed_at.is_some());

    // List runs
    let runs = engine.runs.list_runs(wf_id).unwrap();
    assert_eq!(runs.len(), 1);
}

#[test]
fn test_run_retry_after_failure() {
    let engine = setup_engine();
    let wf_id = Uuid::new_v4();
    let node_id = Uuid::new_v4();

    let run = engine.runs.create_run(wf_id, "manual").unwrap();
    let node_run = engine.runs.create_node_run(run.id, node_id, NodeType::HttpRequest, 3).unwrap();

    // Fail
    engine.runs.update_run_status(run.id, RunStatus::Running, None).unwrap();
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Running,
        None,
        None,
        None,
    ).unwrap();
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Failed,
        None,
        None,
        Some("Connection timeout".into()),
    ).unwrap();

    engine.runs.update_run_status(run.id, RunStatus::Failed, Some("Node failed".into())).unwrap();

    let failed_run = engine.runs.get_run(run.id).unwrap();
    assert_eq!(failed_run.status, RunStatus::Failed);

    // Retry - reset to running
    engine.runs.update_run_status(run.id, RunStatus::Running, None).unwrap();
    engine.runs.increment_attempt(node_run.id).unwrap();
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Running,
        None,
        None,
        None,
    ).unwrap();

    let retried_run = engine.runs.get_run(run.id).unwrap();
    assert_eq!(retried_run.status, RunStatus::Running);

    // Succeed on retry
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Succeeded,
        None,
        Some(serde_json::json!({"status": 200})),
        None,
    ).unwrap();
    engine.runs.update_run_status(run.id, RunStatus::Succeeded, None).unwrap();

    let final_run = engine.runs.get_run(run.id).unwrap();
    assert_eq!(final_run.status, RunStatus::Succeeded);

    let final_node = engine.runs.get_node_run(node_run.id).unwrap();
    assert_eq!(final_node.status, NodeStatus::Succeeded);
    assert_eq!(final_node.output, Some(serde_json::json!({"status": 200})));
}

#[test]
fn test_cancel_workflow_run() {
    let engine = setup_engine();
    let wf_id = Uuid::new_v4();

    let run = engine.runs.create_run(wf_id, "manual").unwrap();
    engine.runs.update_run_status(run.id, RunStatus::Running, None).unwrap();

    // Cancel the run
    engine.runs.update_run_status(run.id, RunStatus::Cancelled, Some("User cancelled".into())).unwrap();

    let cancelled = engine.runs.get_run(run.id).unwrap();
    assert_eq!(cancelled.status, RunStatus::Cancelled);
    assert_eq!(cancelled.error.as_deref(), Some("User cancelled"));
}

#[test]
fn test_unfinished_run_recovery() {
    let engine = setup_engine();
    let wf_id = Uuid::new_v4();

    // Create a running run (simulates crash while running)
    let run1 = engine.runs.create_run(wf_id, "manual").unwrap();
    engine.runs.update_run_status(run1.id, RunStatus::Running, None).unwrap();

    // Create a completed run
    let run2 = engine.runs.create_run(wf_id, "manual").unwrap();
    engine.runs.update_run_status(run2.id, RunStatus::Succeeded, None).unwrap();

    // Create a pending run
    let run3 = engine.runs.create_run(wf_id, "manual").unwrap();

    // Find unfinished runs
    let unfinished = engine.runs.find_unfinished_runs().unwrap();
    assert_eq!(unfinished.len(), 2); // run1 (running) and run3 (pending)

    let ids: Vec<Uuid> = unfinished.iter().map(|r| r.id).collect();
    assert!(ids.contains(&run1.id));
    assert!(ids.contains(&run3.id));
    assert!(!ids.contains(&run2.id));
}

#[test]
fn test_audit_log_during_agent_workflow() {
    let engine = setup_engine();
    let agent_id = Uuid::new_v4();

    // Create agent audit log
    engine.audit.create(
        AuditEventType::AgentCreated,
        "agent",
        Some(agent_id),
        Some("admin"),
        Some(serde_json::json!({"name": "Test Agent"})),
    ).unwrap();

    // Create workflow audit log
    let wf_id = Uuid::new_v4();
    engine.audit.create(
        AuditEventType::WorkflowCreated,
        "workflow",
        Some(wf_id),
        Some("admin"),
        Some(serde_json::json!({"agent_id": agent_id.to_string()})),
    ).unwrap();

    // Query agent logs
    let agent_logs = engine.audit.list(Some("agent"), Some(agent_id), 10).unwrap();
    assert_eq!(agent_logs.len(), 1);
    assert_eq!(agent_logs[0].event_type, AuditEventType::AgentCreated);

    // Query all logs
    let all_logs = engine.audit.list(None, None, 10).unwrap();
    assert_eq!(all_logs.len(), 2);
}

#[test]
fn test_state_machine_integration() {
    // Test that the state machine works correctly with storage

    // Valid transitions
    assert!(validate_run_transition(RunStatus::Pending, RunStatus::Running).is_ok());
    assert!(validate_run_transition(RunStatus::Running, RunStatus::Succeeded).is_ok());
    assert!(validate_run_transition(RunStatus::Failed, RunStatus::Running).is_ok());
    assert!(validate_run_transition(RunStatus::TimedOut, RunStatus::Running).is_ok());

    // Invalid transitions
    assert!(validate_run_transition(RunStatus::Pending, RunStatus::Succeeded).is_err());
    assert!(validate_run_transition(RunStatus::Succeeded, RunStatus::Running).is_err());
    assert!(validate_run_transition(RunStatus::Cancelled, RunStatus::Running).is_err());

    // Node transitions
    assert!(validate_node_transition(NodeStatus::Pending, NodeStatus::Running).is_ok());
    assert!(validate_node_transition(NodeStatus::Failed, NodeStatus::Running).is_ok());
    assert!(validate_node_transition(NodeStatus::Running, NodeStatus::WaitingApproval).is_ok());
}

#[test]
fn test_duplicate_run_detection() {
    let engine = setup_engine();
    let wf_id = Uuid::new_v4();

    // Creating multiple runs for the same workflow is allowed
    let run1 = engine.runs.create_run(wf_id, "manual").unwrap();
    let run2 = engine.runs.create_run(wf_id, "manual").unwrap();

    assert_ne!(run1.id, run2.id);

    let runs = engine.runs.list_runs(wf_id).unwrap();
    assert_eq!(runs.len(), 2);
}

#[test]
fn test_permission_policy_storage() {
    let engine = setup_engine();

    let permissions = PermissionPolicy {
        allowed_hosts: vec!["api.openai.com".into(), "*.example.com".into()],
        allow_file_access: false,
        allow_loopback: false,
        max_nodes: 25,
        max_loops: 5,
        max_request_size: 1024 * 1024,
        max_response_size: 1024 * 1024,
        max_execution_seconds: 300,
        ..Default::default()
    };

    let input = AgentInput {
        name: "Restricted Agent".into(),
        description: None,
        system_prompt: None,
        model: None,
        temperature: None,
        max_tokens: None,
        permissions: Some(permissions.clone()),
    };

    let agent = engine.agents.create(input).unwrap();
    assert_eq!(agent.permissions.allowed_hosts.len(), 2);
    assert_eq!(agent.permissions.max_nodes, 25);
    assert_eq!(agent.permissions.max_execution_seconds, 300);

    // Verify persistence
    let fetched = engine.agents.get(agent.id).unwrap();
    assert_eq!(fetched.permissions.allowed_hosts[0], "api.openai.com");
    assert_eq!(fetched.permissions.max_nodes, 25);
}

#[test]
fn test_node_run_multiple_attempts() {
    let engine = setup_engine();
    let run = engine.runs.create_run(Uuid::new_v4(), "manual").unwrap();

    let node_run = engine.runs.create_node_run(run.id, Uuid::new_v4(), NodeType::Model, 5).unwrap();
    assert_eq!(node_run.max_attempts, 5);

    // Simulate 3 failed attempts
    for i in 1..=3 {
        engine.runs.increment_attempt(node_run.id).unwrap();
        let fetched = engine.runs.get_node_run(node_run.id).unwrap();
        assert_eq!(fetched.attempts, i);
    }

    // Succeed on 4th attempt
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Running,
        None,
        None,
        None,
    ).unwrap();
    engine.runs.update_node_run(
        node_run.id,
        NodeStatus::Succeeded,
        None,
        Some(serde_json::json!({"result": "success"})),
        None,
    ).unwrap();

    let final_node = engine.runs.get_node_run(node_run.id).unwrap();
    assert_eq!(final_node.status, NodeStatus::Succeeded);
    assert_eq!(final_node.attempts, 3); // incremented 3 times, succeeded on 4th
}

#[test]
fn test_multiple_node_runs_in_workflow() {
    let engine = setup_engine();
    let wf_id = Uuid::new_v4();
    let run = engine.runs.create_run(wf_id, "manual").unwrap();

    let node_ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

    for &node_id in &node_ids {
        engine.runs.create_node_run(run.id, node_id, NodeType::Model, 3).unwrap();
    }

    let node_runs = engine.runs.list_node_runs(run.id).unwrap();
    assert_eq!(node_runs.len(), 5);
}
