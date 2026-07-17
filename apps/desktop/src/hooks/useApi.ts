import { useState, useEffect, useCallback } from "react";
import type { WorkflowRun } from "../types";
import * as api from "../lib/api";

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

function useAsync<T>(fn: () => Promise<T>, deps: unknown[]): AsyncState<T> & { refresh: () => void } {
  const [state, setState] = useState<AsyncState<T>>({ data: null, loading: true, error: null });
  const [tick, setTick] = useState(0);

  const refresh = useCallback(() => setTick((t) => t + 1), []);

  useEffect(() => {
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, error: null }));
    fn()
      .then((data) => { if (!cancelled) setState({ data, loading: false, error: null }); })
      .catch((err) => { if (!cancelled) setState({ data: null, loading: false, error: String(err) }); });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tick, ...deps]);

  return { ...state, refresh };
}

export function useAgents() {
  return useAsync(() => api.listAgents(), []);
}

export function useAgent(id: string | undefined) {
  return useAsync(() => id ? api.getAgent(id) : Promise.reject("No ID"), [id]);
}

export function useWorkflows(agentId: string | undefined) {
  return useAsync(() => api.listWorkflows(agentId), [agentId]);
}

export function useWorkflow(id: string | undefined) {
  return useAsync(() => id ? api.getWorkflow(id) : Promise.reject("No ID"), [id]);
}

export function useRuns(workflowId: string | undefined): AsyncState<WorkflowRun[]> & { refresh: () => void } {
  return useAsync(() => workflowId ? api.listRuns(workflowId) : Promise.resolve([]), [workflowId]);
}

export function useRun(id: string | undefined) {
  return useAsync(() => id ? api.getRun(id) : Promise.reject("No ID"), [id]);
}

export function useProviders() {
  return useAsync(() => api.listProviders(), []);
}

export function useSecrets() {
  return useAsync(() => api.listSecrets(), []);
}

export function useAuditLogs(limit?: number) {
  return useAsync(() => api.getAuditLogs(limit), [limit]);
}
