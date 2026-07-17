// Core types matching Rust models
export interface Agent {
  id: string;
  name: string;
  description: string | null;
  system_prompt: string | null;
  model: string | null;
  temperature: number | null;
  max_tokens: number | null;
  permissions: PermissionPolicy;
  created_at: string;
  updated_at: string;
}

export interface AgentInput {
  name: string;
  description: string | null;
  system_prompt: string | null;
  model: string | null;
  temperature: number | null;
  max_tokens: number | null;
  permissions: PermissionPolicy | null;
}

export interface PermissionPolicy {
  allowed_hosts: string[];
  allowed_networks: string[];
  allow_file_access: boolean;
  allow_loopback: boolean;
  max_nodes: number;
  max_loops: number;
  max_request_size: number;
  max_response_size: number;
  max_execution_seconds: number;
}

export interface Workflow {
  id: string;
  agent_id: string;
  name: string;
  description: string | null;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  created_at: string;
  updated_at: string;
}

export interface WorkflowNode {
  id: string;
  workflow_id: string;
  node_type: NodeType;
  name: string;
  config: unknown;
  position_x: number;
  position_y: number;
}

export interface WorkflowEdge {
  id: string;
  workflow_id: string;
  source_node_id: string;
  target_node_id: string;
  source_handle: string | null;
  target_handle: string | null;
  condition_expression: string | null;
}

export interface WorkflowInput {
  agent_id: string;
  name: string;
  description: string | null;
  nodes: WorkflowNodeInput[];
  edges: WorkflowEdgeInput[];
}

export interface WorkflowNodeInput {
  node_type: NodeType;
  name: string;
  config: unknown;
  position_x: number;
  position_y: number;
}

export interface WorkflowEdgeInput {
  source_node_id: string;
  target_node_id: string;
  source_handle: string | null;
  target_handle: string | null;
  condition_expression: string | null;
}

export type NodeType =
  | "start" | "input" | "model" | "http_request"
  | "condition" | "template" | "end";

export type RunStatus =
  | "pending" | "running" | "paused" | "failed"
  | "succeeded" | "cancelled" | "timed_out";

export type NodeStatus =
  | "pending" | "running" | "paused" | "waiting_approval"
  | "failed" | "succeeded" | "cancelled";

export interface WorkflowRun {
  id: string;
  workflow_id: string;
  status: RunStatus;
  started_at: string | null;
  completed_at: string | null;
  error: string | null;
  trigger_type: string;
  created_at: string;
}

export interface NodeRun {
  id: string;
  workflow_run_id: string;
  node_id: string;
  node_type: NodeType;
  status: NodeStatus;
  input: unknown | null;
  output: unknown | null;
  error: string | null;
  started_at: string | null;
  completed_at: string | null;
  attempts: number;
  max_attempts: number;
  created_at: string;
}

export interface ProviderInfo {
  id: string;
  name: string;
  provider_type: string;
  base_url: string;
  has_api_key: boolean;
}

export interface SecretInfo {
  key: string;
  exists: boolean;
}

export interface AuditLog {
  id: string;
  event_type: string;
  entity_type: string;
  entity_id: string | null;
  user: string | null;
  details: unknown | null;
  created_at: string;
}

export interface UrlValidationResult {
  valid: boolean;
  message: string;
}

export interface ApiError {
  message: string;
  code?: string;
}