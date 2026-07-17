use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;
use uuid::Uuid;

use localflow_core::models::{NodeRun, NodeStatus, RunStatus, Workflow, WorkflowNode, WorkflowRun};
use localflow_core::state_machine::{validate_node_transition, validate_run_transition};
use localflow_core::types::NodeType;
use localflow_secret_vault::SecretVault;
use localflow_storage::RunRepository;

use crate::executor::create_executor;
use crate::types::*;

/// DAG workflow runner that orchestrates node execution.
pub struct DagRunner {
    storage: Arc<RunRepository>,
    vault: Arc<dyn SecretVault>,
    max_nodes: usize,
    max_runtime_secs: u64,
    max_retry: i32,
    cancel_rx: Option<watch::Receiver<bool>>,
}

impl DagRunner {
    pub fn new(storage: Arc<RunRepository>, vault: Arc<dyn SecretVault>) -> Self {
        Self {
            storage,
            vault,
            max_nodes: DEFAULT_MAX_NODES,
            max_runtime_secs: DEFAULT_WORKFLOW_TIMEOUT_SECS,
            max_retry: DEFAULT_MAX_RETRY,
            cancel_rx: None,
        }
    }
    pub fn with_max_nodes(mut self, max: usize) -> Self {
        self.max_nodes = max;
        self
    }
    pub fn with_max_runtime(mut self, secs: u64) -> Self {
        self.max_runtime_secs = secs;
        self
    }
    pub fn with_max_retry(mut self, retry: i32) -> Self {
        self.max_retry = retry;
        self
    }
    pub fn with_cancel(mut self, rx: watch::Receiver<bool>) -> Self {
        self.cancel_rx = Some(rx);
        self
    }
}
impl DagRunner {
    pub async fn run_workflow_from_start(
        &self,
        workflow: Workflow,
        trigger_type: &str,
    ) -> ExecutionResult<WorkflowRun> {
        self.validate_workflow(&workflow)?;
        let mut run = self
            .storage
            .create_run(workflow.id, trigger_type)
            .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))?;
        self.transition_run(&run, RunStatus::Running, None).await?;
        run.status = RunStatus::Running;
        match self.execute_dag(&workflow, &mut run).await {
            Ok(()) => {
                self.transition_run(&run, RunStatus::Succeeded, None)
                    .await?;
            }
            Err(e) => {
                let err_msg = e.to_string();
                let status = match &e {
                    WorkflowExecutionError::Cancelled => RunStatus::Cancelled,
                    WorkflowExecutionError::Timeout(_) => RunStatus::TimedOut,
                    _ => RunStatus::Failed,
                };
                self.transition_run(&run, status, Some(err_msg)).await?;
            }
        }
        self.storage
            .get_run(run.id)
            .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))
    }

    pub async fn resume_workflow(
        &self,
        workflow: Workflow,
        existing_run: &WorkflowRun,
    ) -> ExecutionResult<WorkflowRun> {
        if !existing_run.can_retry() {
            return Err(WorkflowExecutionError::Validation(format!(
                "Run {} is not retryable (status={:?})",
                existing_run.id, existing_run.status
            )));
        }
        self.validate_workflow(&workflow)?;
        let all_node_runs = self
            .storage
            .list_node_runs(existing_run.id)
            .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))?;
        let mut node_run_map: HashMap<Uuid, NodeRun> = HashMap::new();
        for nr in &all_node_runs {
            node_run_map.insert(nr.node_id, nr.clone());
        }
        let mut run = existing_run.clone();
        self.transition_run(&run, RunStatus::Running, None).await?;
        run.status = RunStatus::Running;
        let mut upstream_outputs = HashMap::new();
        for nr in &all_node_runs {
            if nr.status == NodeStatus::Succeeded
                && let Some(output) = &nr.output
            {
                upstream_outputs.insert(nr.node_id, output.clone());
            }
        }
        match self
            .execute_dag_resume(&workflow, &mut run, &node_run_map, &mut upstream_outputs)
            .await
        {
            Ok(()) => {
                self.transition_run(&run, RunStatus::Succeeded, None)
                    .await?;
            }
            Err(e) => {
                let err_msg = e.to_string();
                let status = match &e {
                    WorkflowExecutionError::Cancelled => RunStatus::Cancelled,
                    WorkflowExecutionError::Timeout(_) => RunStatus::TimedOut,
                    _ => RunStatus::Failed,
                };
                self.transition_run(&run, status, Some(err_msg)).await?;
            }
        }
        self.storage
            .get_run(run.id)
            .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))
    }
}
impl DagRunner {
    async fn execute_dag(&self, workflow: &Workflow, run: &mut WorkflowRun) -> ExecutionResult<()> {
        let sorted = self.topological_sort(workflow)?;
        let mut upstream_outputs: HashMap<Uuid, serde_json::Value> = HashMap::new();
        let mut variables: HashMap<String, serde_json::Value> = HashMap::new();
        let start_time = Instant::now();

        for node_id in &sorted {
            self.check_cancelled().await?;
            self.check_timeout(start_time).await?;

            let node = workflow
                .nodes
                .iter()
                .find(|n| n.id == *node_id)
                .ok_or_else(|| {
                    WorkflowExecutionError::Internal(format!("Node {node_id} not found"))
                })?;

            let node_upstream = self.collect_upstream_outputs(node, workflow, &upstream_outputs);
            let max_attempts = if node.node_type == NodeType::HttpRequest {
                1
            } else {
                self.max_retry
            };

            let mut node_run = self
                .storage
                .create_node_run(run.id, node.id, node.node_type, max_attempts)
                .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))?;

            self.transition_node(&node_run, NodeStatus::Running, None, None, None)
                .await?;
            node_run.status = NodeStatus::Running;

            let result = self
                .execute_with_retry(
                    node,
                    &node_upstream,
                    &variables,
                    &mut node_run,
                    start_time,
                    workflow,
                    run,
                )
                .await;

            match result {
                Ok(output) => {
                    if output.untrusted {
                        tracing::warn!(node = %node.name, "Output from untrusted source");
                    }
                    upstream_outputs.insert(node.id, output.data.clone());
                    if let Some(branch) = &output.branch {
                        variables.insert(
                            format!("branch_{}", node.name),
                            serde_json::Value::String(branch.clone()),
                        );
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    self.transition_node(
                        &node_run,
                        NodeStatus::Failed,
                        None,
                        None,
                        Some(err_msg.clone()),
                    )
                    .await?;
                    return Err(match e {
                        WorkflowExecutionError::Cancelled => e,
                        WorkflowExecutionError::Timeout(_) => e,
                        _ => WorkflowExecutionError::NodeExecution {
                            node_name: node.name.clone(),
                            node_type: node.node_type,
                            error: err_msg,
                        },
                    });
                }
            }
        }
        Ok(())
    }
}
impl DagRunner {
    async fn execute_dag_resume(
        &self,
        workflow: &Workflow,
        run: &mut WorkflowRun,
        existing_node_runs: &HashMap<Uuid, NodeRun>,
        upstream_outputs: &mut HashMap<Uuid, serde_json::Value>,
    ) -> ExecutionResult<()> {
        let sorted = self.topological_sort(workflow)?;
        let mut variables: HashMap<String, serde_json::Value> = HashMap::new();
        let start_time = Instant::now();

        for node_id in &sorted {
            self.check_cancelled().await?;
            self.check_timeout(start_time).await?;

            let node = workflow
                .nodes
                .iter()
                .find(|n| n.id == *node_id)
                .ok_or_else(|| {
                    WorkflowExecutionError::Internal(format!("Node {node_id} not found"))
                })?;

            if let Some(nr) = existing_node_runs.get(node_id)
                && nr.status == NodeStatus::Succeeded
            {
                if let Some(output) = &nr.output {
                    upstream_outputs
                        .entry(node.id)
                        .or_insert_with(|| output.clone());
                }
                continue;
            }

            let node_upstream = self.collect_upstream_outputs(node, workflow, upstream_outputs);
            let mut node_run =
                existing_node_runs
                    .get(node_id)
                    .cloned()
                    .unwrap_or_else(|| NodeRun {
                        id: Uuid::new_v4(),
                        workflow_run_id: run.id,
                        node_id: node.id,
                        node_type: node.node_type,
                        status: NodeStatus::Pending,
                        input: None,
                        output: None,
                        error: None,
                        started_at: None,
                        completed_at: None,
                        attempts: 0,
                        max_attempts: self.max_retry,
                        created_at: chrono::Utc::now(),
                    });

            self.transition_node(&node_run, NodeStatus::Running, None, None, None)
                .await?;
            node_run.status = NodeStatus::Running;

            let result = self
                .execute_with_retry(
                    node,
                    &node_upstream,
                    &variables,
                    &mut node_run,
                    start_time,
                    workflow,
                    run,
                )
                .await;

            match result {
                Ok(output) => {
                    upstream_outputs.insert(node.id, output.data.clone());
                    if let Some(branch) = &output.branch {
                        variables.insert(
                            format!("branch_{}", node.name),
                            serde_json::Value::String(branch.clone()),
                        );
                    }
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    self.transition_node(
                        &node_run,
                        NodeStatus::Failed,
                        None,
                        None,
                        Some(err_msg.clone()),
                    )
                    .await?;
                    return Err(match e {
                        WorkflowExecutionError::Cancelled => e,
                        WorkflowExecutionError::Timeout(_) => e,
                        _ => WorkflowExecutionError::NodeExecution {
                            node_name: node.name.clone(),
                            node_type: node.node_type,
                            error: err_msg,
                        },
                    });
                }
            }
        }
        Ok(())
    }
}
impl DagRunner {
    #[allow(clippy::too_many_arguments)]
    async fn execute_with_retry(
        &self,
        node: &WorkflowNode,
        upstream_outputs: &HashMap<Uuid, serde_json::Value>,
        variables: &HashMap<String, serde_json::Value>,
        node_run: &mut NodeRun,
        start_time: Instant,
        workflow: &Workflow,
        workflow_run: &WorkflowRun,
    ) -> ExecutionResult<NodeOutput> {
        let mut last_error = None;
        let executor = create_executor(node.node_type, Some(self.vault.clone()));

        for attempt in 0..node_run.max_attempts {
            self.check_cancelled().await?;
            self.check_timeout(start_time).await?;
            if attempt > 0 {
                self.storage
                    .increment_attempt(node_run.id)
                    .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))?;
                node_run.attempts += 1;
                tracing::info!(node = %node.name, attempt, max = node_run.max_attempts, "Retrying node");
            }
            let ctx = ExecutionContext {
                workflow: workflow.clone(),
                workflow_run: workflow_run.clone(),
                node_runs: HashMap::new(),
                node_config: node.config.clone(),
                upstream_outputs: upstream_outputs.clone(),
                variables: variables.clone(),
                current_node: node.clone(),
            };
            match executor.execute(&ctx).await {
                Ok(output) => {
                    self.transition_node(
                        node_run,
                        NodeStatus::Succeeded,
                        None,
                        Some(output.data.clone()),
                        None,
                    )
                    .await?;
                    return Ok(output);
                }
                Err(e) => {
                    if !e.is_retryable() || attempt + 1 >= node_run.max_attempts {
                        return Err(e);
                    }
                    last_error = Some(e);
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            WorkflowExecutionError::Internal("All retry attempts exhausted".into())
        }))
    }

    fn collect_upstream_outputs(
        &self,
        node: &WorkflowNode,
        workflow: &Workflow,
        upstream_outputs: &HashMap<Uuid, serde_json::Value>,
    ) -> HashMap<Uuid, serde_json::Value> {
        let mut result = HashMap::new();
        for edge in &workflow.edges {
            if edge.target_node_id == node.id
                && let Some(output) = upstream_outputs.get(&edge.source_node_id)
            {
                result.insert(edge.source_node_id, output.clone());
            }
        }
        result
    }

    fn validate_workflow(&self, workflow: &Workflow) -> ExecutionResult<()> {
        if workflow.nodes.is_empty() {
            return Err(WorkflowExecutionError::Validation(
                "Workflow has no nodes".into(),
            ));
        }
        if workflow.nodes.len() > self.max_nodes {
            return Err(WorkflowExecutionError::Validation(format!(
                "Workflow has {} nodes, exceeds max of {}",
                workflow.nodes.len(),
                self.max_nodes
            )));
        }
        if workflow.start_node().is_none() {
            return Err(WorkflowExecutionError::DagValidation(
                "Workflow must have a Start node".into(),
            ));
        }
        if workflow.end_nodes().is_empty() {
            return Err(WorkflowExecutionError::DagValidation(
                "Workflow must have an End node".into(),
            ));
        }
        if !workflow.is_valid_dag() {
            return Err(WorkflowExecutionError::DagValidation(
                "Workflow contains cycles".into(),
            ));
        }
        Ok(())
    }

    fn topological_sort(&self, workflow: &Workflow) -> ExecutionResult<Vec<Uuid>> {
        let mut in_degree: HashMap<Uuid, usize> = HashMap::new();
        let mut adjacency: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for node in &workflow.nodes {
            in_degree.entry(node.id).or_insert(0);
            adjacency.entry(node.id).or_default();
        }
        for edge in &workflow.edges {
            adjacency
                .entry(edge.source_node_id)
                .or_default()
                .push(edge.target_node_id);
            *in_degree.entry(edge.target_node_id).or_insert(0) += 1;
        }
        let mut queue: Vec<Uuid> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut sorted = Vec::with_capacity(workflow.nodes.len());
        while let Some(n) = queue.pop() {
            sorted.push(n);
            if let Some(neighbors) = adjacency.get(&n) {
                for &next in neighbors {
                    if let Some(deg) = in_degree.get_mut(&next) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(next);
                        }
                    }
                }
            }
        }
        if sorted.len() != workflow.nodes.len() {
            return Err(WorkflowExecutionError::DagValidation(
                "Cycle detected".into(),
            ));
        }
        Ok(sorted)
    }
}
impl DagRunner {
    async fn transition_run(
        &self,
        run: &WorkflowRun,
        new_status: RunStatus,
        error: Option<String>,
    ) -> ExecutionResult<()> {
        validate_run_transition(run.status, new_status)
            .map_err(|e| WorkflowExecutionError::Validation(e.to_string()))?;
        self.storage
            .update_run_status(run.id, new_status, error)
            .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn transition_node(
        &self,
        node_run: &NodeRun,
        new_status: NodeStatus,
        input: Option<serde_json::Value>,
        output: Option<serde_json::Value>,
        error: Option<String>,
    ) -> ExecutionResult<()> {
        validate_node_transition(node_run.status, new_status)
            .map_err(|e| WorkflowExecutionError::Validation(e.to_string()))?;
        self.storage
            .update_node_run(node_run.id, new_status, input, output, error)
            .map_err(|e| WorkflowExecutionError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn check_cancelled(&self) -> ExecutionResult<()> {
        if let Some(ref rx) = self.cancel_rx
            && *rx.borrow()
        {
            return Err(WorkflowExecutionError::Cancelled);
        }
        Ok(())
    }

    async fn check_timeout(&self, start: Instant) -> ExecutionResult<()> {
        if start.elapsed().as_secs() > self.max_runtime_secs {
            return Err(WorkflowExecutionError::Timeout(self.max_runtime_secs));
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn sanitize_untrusted_output(output: &serde_json::Value) -> serde_json::Value {
    output.clone()
}
#[cfg(test)]
mod tests {
    use super::*;
    use localflow_core::models::{Workflow, WorkflowEdge, WorkflowNode};
    use localflow_core::types::NodeType;
    use localflow_secret_vault::InMemoryVault;
    use localflow_storage::RunRepository;
    use localflow_storage::init_test_db;

    fn make_workflow(nodes: Vec<(NodeType, &str)>, edges: Vec<(usize, usize)>) -> Workflow {
        let wf_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let workflow_nodes: Vec<WorkflowNode> = nodes
            .into_iter()
            .enumerate()
            .map(|(i, (nt, name))| WorkflowNode {
                id: Uuid::from_u128(i as u128 + 1),
                workflow_id: wf_id,
                node_type: nt,
                name: name.to_string(),
                config: serde_json::Value::Null,
                position_x: (i as f64) * 100.0,
                position_y: 0.0,
            })
            .collect();

        let workflow_edges: Vec<WorkflowEdge> = edges
            .into_iter()
            .map(|(src, tgt)| WorkflowEdge {
                id: Uuid::new_v4(),
                workflow_id: wf_id,
                source_node_id: Uuid::from_u128(src as u128 + 1),
                target_node_id: Uuid::from_u128(tgt as u128 + 1),
                source_handle: None,
                target_handle: None,
                condition_expression: None,
            })
            .collect();

        Workflow {
            id: wf_id,
            agent_id: Uuid::new_v4(),
            name: "Test Workflow".into(),
            description: None,
            nodes: workflow_nodes,
            edges: workflow_edges,
            created_at: now,
            updated_at: now,
        }
    }

    fn setup_runner() -> (DagRunner, Arc<RunRepository>) {
        let conn = init_test_db().unwrap();
        let conn_arc = Arc::new(std::sync::Mutex::new(conn));
        let storage = Arc::new(RunRepository::new(conn_arc));
        let vault = Arc::new(InMemoryVault::new());
        let runner = DagRunner::new(storage.clone(), vault);
        (runner, storage)
    }
    #[test]
    fn test_validate_empty_workflow() {
        let (runner, _) = setup_runner();
        let wf = make_workflow(vec![], vec![]);
        assert!(runner.validate_workflow(&wf).is_err());
    }

    #[test]
    fn test_validate_no_start_node() {
        let (runner, _) = setup_runner();
        let wf = make_workflow(vec![(NodeType::End, "End")], vec![]);
        assert!(runner.validate_workflow(&wf).is_err());
    }

    #[test]
    fn test_validate_valid_dag() {
        let (runner, _) = setup_runner();
        let wf = make_workflow(
            vec![
                (NodeType::Start, "S"),
                (NodeType::Input, "I"),
                (NodeType::End, "E"),
            ],
            vec![(0, 1), (1, 2)],
        );
        assert!(runner.validate_workflow(&wf).is_ok());
    }

    #[test]
    fn test_detect_cycle() {
        let wf = make_workflow(
            vec![
                (NodeType::Start, "S"),
                (NodeType::Input, "A"),
                (NodeType::End, "E"),
            ],
            vec![(0, 1), (1, 2), (2, 1)],
        );
        assert!(!wf.is_valid_dag());
    }

    #[test]
    fn test_topological_sort() {
        let (runner, _) = setup_runner();
        let wf = make_workflow(
            vec![
                (NodeType::Start, "S"),
                (NodeType::Input, "I"),
                (NodeType::End, "E"),
            ],
            vec![(0, 1), (1, 2)],
        );
        let sorted = runner.topological_sort(&wf).unwrap();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0], Uuid::from_u128(1));
        assert_eq!(sorted[2], Uuid::from_u128(3));
    }

    #[test]
    fn test_validate_exceeds_max_nodes() {
        let (runner, _) = setup_runner();
        let runner = runner.with_max_nodes(1);
        let wf = make_workflow(
            vec![(NodeType::Start, "S"), (NodeType::End, "E")],
            vec![(0, 1)],
        );
        assert!(runner.validate_workflow(&wf).is_err());
    }

    #[test]
    fn test_retryable_errors() {
        assert!(WorkflowExecutionError::Timeout(30).is_retryable());
        assert!(WorkflowExecutionError::Internal("oops".into()).is_retryable());
        assert!(!WorkflowExecutionError::Validation("bad".into()).is_retryable());
        assert!(!WorkflowExecutionError::Cancelled.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = WorkflowExecutionError::NodeExecution {
            node_name: "Model".into(),
            node_type: NodeType::Model,
            error: "API timeout".into(),
        };
        let s = err.to_string();
        assert!(s.contains("Model"));
        assert!(s.contains("API timeout"));
    }
}
