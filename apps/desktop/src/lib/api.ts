import { invoke } from "@tauri-apps/api/core";
import type {
  Agent, AgentInput, Workflow, WorkflowInput, WorkflowRun, NodeRun,
  ProviderInfo, SecretInfo, AuditLog, UrlValidationResult,
} from "../types";

// ── Agent API ────────────────────────────────────────────────────

export async function listAgents(): Promise<Agent[]> {
  return invoke("list_agents");
}

export async function getAgent(id: string): Promise<Agent> {
  return invoke("get_agent", { id });
}

export async function createAgent(input: AgentInput): Promise<Agent> {
  return invoke("create_agent", { input });
}

export async function updateAgent(id: string, input: AgentInput): Promise<Agent> {
  return invoke("update_agent", { id, input });
}

export async function deleteAgent(id: string): Promise<void> {
  return invoke("delete_agent", { id });
}

export async function exportAgent(id: string): Promise<string> {
  return invoke("export_agent", { id });
}

export async function importAgent(jsonData: string): Promise<Agent> {
  return invoke("import_agent", { jsonData });
}

// ── Workflow API ─────────────────────────────────────────────────

export async function listWorkflows(agentId?: string): Promise<Workflow[]> {
  return invoke("list_workflows", { agentId: agentId ?? null });
}

export async function getWorkflow(id: string): Promise<Workflow> {
  return invoke("get_workflow", { id });
}

export async function createWorkflow(input: WorkflowInput): Promise<Workflow> {
  return invoke("create_workflow", { input });
}

export async function updateWorkflow(id: string, input: WorkflowInput): Promise<Workflow> {
  return invoke("update_workflow", { id, input });
}

export async function deleteWorkflow(id: string): Promise<void> {
  return invoke("delete_workflow", { id });
}

// ── Run API ──────────────────────────────────────────────────────

export async function listRuns(workflowId: string): Promise<WorkflowRun[]> {
  return invoke("list_runs", { workflowId });
}

export async function getRun(id: string): Promise<WorkflowRun> {
  return invoke("get_run", { id });
}

export async function startRun(workflowId: string): Promise<WorkflowRun> {
  return invoke("start_run", { workflowId });
}

export async function cancelRun(runId: string): Promise<void> {
  return invoke("cancel_run", { runId });
}

export async function retryRun(workflowId: string, runId: string): Promise<WorkflowRun> {
  return invoke("retry_run", { workflowId, runId });
}

export async function getNodeRuns(runId: string): Promise<NodeRun[]> {
  return invoke("get_node_runs", { runId });
}

// ── Provider API ─────────────────────────────────────────────────

export async function listProviders(): Promise<ProviderInfo[]> {
  return invoke("list_providers");
}

export async function saveProvider(id: string, name: string, baseUrl: string): Promise<void> {
  return invoke("save_provider", { id, name, baseUrl });
}

export async function deleteProvider(id: string): Promise<void> {
  return invoke("delete_provider", { id });
}

// ── Secret API (never returns values, only existence) ────────────

export async function storeSecret(key: string, value: string): Promise<void> {
  return invoke("store_secret", { key, value });
}

export async function deleteSecret(key: string): Promise<void> {
  return invoke("delete_secret", { key });
}

export async function listSecrets(): Promise<SecretInfo[]> {
  return invoke("list_secrets");
}

export async function checkSecretExists(key: string): Promise<boolean> {
  return invoke("check_secret_exists", { key });
}

// ── Security API ─────────────────────────────────────────────────

export async function getAuditLogs(limit?: number): Promise<AuditLog[]> {
  return invoke("get_audit_logs", { limit: limit ?? null });
}

export interface OpenApiImportResult {
  valid: boolean;
  message: string;
  endpoints: { name: string; description: string; method: string; path: string }[];
}

export async function importOpenApi(rawDocument: string): Promise<OpenApiImportResult> {
  return invoke("import_openapi", { rawDocument });
}

export async function validateUrl(url: string): Promise<UrlValidationResult> {
  return invoke("validate_url", { url });
}