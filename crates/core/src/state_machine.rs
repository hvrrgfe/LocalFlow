use crate::error::{CoreError, CoreResult};
use crate::models::{NodeStatus, RunStatus};

/// Validates a state transition for a workflow run.
pub fn validate_run_transition(from: RunStatus, to: RunStatus) -> CoreResult<()> {
    let valid = match (from, to) {
        // Normal flow
        (RunStatus::Pending, RunStatus::Running) => true,
        (RunStatus::Running, RunStatus::Paused) => true,
        (RunStatus::Running, RunStatus::Succeeded) => true,
        (RunStatus::Running, RunStatus::Failed) => true,
        (RunStatus::Running, RunStatus::Cancelled) => true,
        (RunStatus::Running, RunStatus::TimedOut) => true,
        // Resume from pause
        (RunStatus::Paused, RunStatus::Running) => true,
        (RunStatus::Paused, RunStatus::Cancelled) => true,
        // Retry from failure
        (RunStatus::Failed, RunStatus::Running) => true,
        (RunStatus::TimedOut, RunStatus::Running) => true,
        // Cancel from pending
        (RunStatus::Pending, RunStatus::Cancelled) => true,
        _ => false,
    };

    if valid {
        Ok(())
    } else {
        Err(CoreError::state_machine(format!(
            "Invalid run state transition: {from:?} -> {to:?}"
        )))
    }
}

/// Validates a state transition for a node execution.
pub fn validate_node_transition(from: NodeStatus, to: NodeStatus) -> CoreResult<()> {
    let valid = match (from, to) {
        // Normal flow
        (NodeStatus::Pending, NodeStatus::Running) => true,
        (NodeStatus::Running, NodeStatus::Paused) => true,
        (NodeStatus::Running, NodeStatus::WaitingApproval) => true,
        (NodeStatus::Running, NodeStatus::Succeeded) => true,
        (NodeStatus::Running, NodeStatus::Failed) => true,
        (NodeStatus::Running, NodeStatus::Cancelled) => true,
        // Resume from pause
        (NodeStatus::Paused, NodeStatus::Running) => true,
        (NodeStatus::Paused, NodeStatus::Cancelled) => true,
        // Resume from waiting
        (NodeStatus::WaitingApproval, NodeStatus::Running) => true,
        (NodeStatus::WaitingApproval, NodeStatus::Cancelled) => true,
        // Retry from failure
        (NodeStatus::Failed, NodeStatus::Running) => true,
        // Cancel from pending
        (NodeStatus::Pending, NodeStatus::Cancelled) => true,
        _ => false,
    };

    if valid {
        Ok(())
    } else {
        Err(CoreError::state_machine(format!(
            "Invalid node state transition: {from:?} -> {to:?}"
        )))
    }
}

/// Returns the set of terminal states for a run.
pub fn terminal_run_states() -> &'static [RunStatus] {
    &[
        RunStatus::Succeeded,
        RunStatus::Failed,
        RunStatus::Cancelled,
        RunStatus::TimedOut,
    ]
}

/// Returns the set of terminal states for a node.
pub fn terminal_node_states() -> &'static [NodeStatus] {
    &[
        NodeStatus::Succeeded,
        NodeStatus::Failed,
        NodeStatus::Cancelled,
    ]
}

/// Returns states from which retry is allowed.
pub fn retryable_run_states() -> &'static [RunStatus] {
    &[RunStatus::Failed, RunStatus::TimedOut]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_run_transitions() {
        // Pending -> Running
        assert!(validate_run_transition(RunStatus::Pending, RunStatus::Running).is_ok());
        // Running -> Succeeded
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Succeeded).is_ok());
        // Running -> Failed
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Failed).is_ok());
        // Running -> Cancelled
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Cancelled).is_ok());
        // Running -> Paused
        assert!(validate_run_transition(RunStatus::Running, RunStatus::Paused).is_ok());
        // Paused -> Running
        assert!(validate_run_transition(RunStatus::Paused, RunStatus::Running).is_ok());
        // Failed -> Running (retry)
        assert!(validate_run_transition(RunStatus::Failed, RunStatus::Running).is_ok());
        // TimedOut -> Running (retry)
        assert!(validate_run_transition(RunStatus::TimedOut, RunStatus::Running).is_ok());
    }

    #[test]
    fn test_invalid_run_transitions() {
        // Pending -> Succeeded (skip running)
        assert!(validate_run_transition(RunStatus::Pending, RunStatus::Succeeded).is_err());
        // Succeeded -> Running (already done)
        assert!(validate_run_transition(RunStatus::Succeeded, RunStatus::Running).is_err());
        // Cancelled -> Running
        assert!(validate_run_transition(RunStatus::Cancelled, RunStatus::Running).is_err());
        // TimedOut -> Succeeded
        assert!(validate_run_transition(RunStatus::TimedOut, RunStatus::Succeeded).is_err());
    }

    #[test]
    fn test_valid_node_transitions() {
        // Pending -> Running
        assert!(validate_node_transition(NodeStatus::Pending, NodeStatus::Running).is_ok());
        // Running -> Succeeded
        assert!(validate_node_transition(NodeStatus::Running, NodeStatus::Succeeded).is_ok());
        // Running -> Failed
        assert!(validate_node_transition(NodeStatus::Running, NodeStatus::Failed).is_ok());
        // Running -> WaitingApproval
        assert!(validate_node_transition(NodeStatus::Running, NodeStatus::WaitingApproval).is_ok());
        // Failed -> Running (retry)
        assert!(validate_node_transition(NodeStatus::Failed, NodeStatus::Running).is_ok());
    }

    #[test]
    fn test_invalid_node_transitions() {
        // Pending -> Succeeded
        assert!(validate_node_transition(NodeStatus::Pending, NodeStatus::Succeeded).is_err());
        // Succeeded -> Running
        assert!(validate_node_transition(NodeStatus::Succeeded, NodeStatus::Running).is_err());
        // Cancelled -> Running
        assert!(validate_node_transition(NodeStatus::Cancelled, NodeStatus::Running).is_err());
    }

    #[test]
    fn test_terminal_states() {
        let terms = terminal_run_states();
        assert!(terms.contains(&RunStatus::Succeeded));
        assert!(terms.contains(&RunStatus::Failed));
        assert!(terms.contains(&RunStatus::Cancelled));
        assert!(terms.contains(&RunStatus::TimedOut));
        assert!(!terms.contains(&RunStatus::Running));
    }

    #[test]
    fn test_retryable_states() {
        let retryable = retryable_run_states();
        assert!(retryable.contains(&RunStatus::Failed));
        assert!(retryable.contains(&RunStatus::TimedOut));
        assert!(!retryable.contains(&RunStatus::Cancelled));
    }
}
