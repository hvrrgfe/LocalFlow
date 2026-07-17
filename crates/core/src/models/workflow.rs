use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::types::NodeType;

/// A directed acyclic graph (DAG) workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single node in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub node_type: NodeType,
    pub name: String,
    /// Node-specific configuration as JSON.
    pub config: Value,
    pub position_x: f64,
    pub position_y: f64,
}

/// A directed edge connecting two nodes in a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    /// Optional condition expression for conditional branching.
    pub condition_expression: Option<String>,
}

/// Status of a workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Paused,
    Failed,
    Succeeded,
    Cancelled,
    TimedOut,
}

/// Status of a single node execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Paused,
    WaitingApproval,
    Failed,
    Succeeded,
    Cancelled,
}

/// A single execution run of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub status: RunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub trigger_type: String,
    pub created_at: DateTime<Utc>,
}

/// Execution record of a single node within a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRun {
    pub id: Uuid,
    pub workflow_run_id: Uuid,
    pub node_id: Uuid,
    pub node_type: NodeType,
    pub status: NodeStatus,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub max_attempts: i32,
    pub created_at: DateTime<Utc>,
}

impl Workflow {
    /// Check if the workflow forms a valid DAG (no cycles).
    /// Uses Kahn's algorithm for topological sort.
    pub fn is_valid_dag(&self) -> bool {
        let mut in_degree: std::collections::HashMap<Uuid, usize> =
            self.nodes.iter().map(|n| (n.id, 0)).collect();

        let mut adjacency: std::collections::HashMap<Uuid, Vec<Uuid>> =
            self.nodes.iter().map(|n| (n.id, Vec::new())).collect();

        for edge in &self.edges {
            if let Some(neighbors) = adjacency.get_mut(&edge.source_node_id) {
                neighbors.push(edge.target_node_id);
            }
            if let Some(degree) = in_degree.get_mut(&edge.target_node_id) {
                *degree += 1;
            }
        }

        let mut queue: Vec<Uuid> = in_degree
            .iter()
            .filter(|&(_, deg)| *deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut visited = 0;
        while let Some(node) = queue.pop() {
            visited += 1;
            if let Some(neighbors) = adjacency.get(&node) {
                for &next in neighbors {
                    if let Some(degree) = in_degree.get_mut(&next) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(next);
                        }
                    }
                }
            }
        }

        visited == self.nodes.len()
    }

    /// Get the start node of the workflow.
    pub fn start_node(&self) -> Option<&WorkflowNode> {
        self.nodes.iter().find(|n| n.node_type == NodeType::Start)
    }

    /// Get the end nodes of the workflow.
    pub fn end_nodes(&self) -> Vec<&WorkflowNode> {
        self.nodes
            .iter()
            .filter(|n| n.node_type == NodeType::End)
            .collect()
    }

    /// Get downstream nodes of a given node.
    pub fn downstream_nodes(&self, node_id: Uuid) -> Vec<Uuid> {
        self.edges
            .iter()
            .filter(|e| e.source_node_id == node_id)
            .map(|e| e.target_node_id)
            .collect()
    }
}

impl WorkflowRun {
    /// Check if the run is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled | RunStatus::TimedOut
        )
    }

    /// Check if the run can be retried.
    pub fn can_retry(&self) -> bool {
        matches!(self.status, RunStatus::Failed | RunStatus::TimedOut)
    }
}

impl NodeRun {
    /// Check if this node run is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            NodeStatus::Succeeded | NodeStatus::Failed | NodeStatus::Cancelled
        )
    }
}
