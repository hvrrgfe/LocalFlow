import { useState } from "react";
import { useParams } from "react-router-dom";
import { useWorkflows, useWorkflow, useRuns } from "../hooks/useApi";
import * as api from "../lib/api";
import type { NodeType } from "../types";

const NODE_TYPES: { value: NodeType; label: string }[] = [
  { value: "start", label: "Start" },
  { value: "input", label: "Input" },
  { value: "model", label: "Model" },
  { value: "http_request", label: "HTTP Request" },
  { value: "condition", label: "Condition" },
  { value: "template", label: "Template" },
  { value: "end", label: "End" },
];

export default function WorkflowEditorPage() {
  const { id: agentId } = useParams<{ id: string }>();
  const { data: workflows, loading, refresh } = useWorkflows(agentId);
  const [selectedWf, _setSelectedWf] = useState<string | null>(null);

  if (loading) return <div className="page-loading">加载中...</div>;

  const workflow = workflows?.find((w) => w.id === selectedWf) ?? workflows?.[0];

  if (!workflow) {
    return (
      <div className="page">
        <div className="page-header">
          <h1>工作流编辑器</h1>
        </div>
        <div className="empty-state">
          <p>该 Agent 还没有工作流。</p>
          <button
            className="btn btn-primary"
            onClick={async () => {
              try {
                await api.createWorkflow({
                  agent_id: agentId!,
                  name: "默认工作流",
                  description: null,
                  nodes: [],
                  edges: [],
                });
                refresh();
              } catch (e) {
                alert("创建失败: " + String(e));
              }
            }}
          >
            创建默认工作流
          </button>
        </div>
      </div>
    );
  }

  return <WorkflowCanvas workflowId={workflow.id} />;
}

function WorkflowCanvas({ workflowId }: { workflowId: string }) {
  const { data: wf, loading, refresh } = useWorkflow(workflowId);
  const { data: runs, refresh: refreshRuns } = useRuns(workflowId);
  const [running, setRunning] = useState(false);

  if (loading || !wf) return <div className="page-loading">加载中...</div>;

  const nodes = wf.nodes ?? [];

  const addNode = async (nodeType: NodeType) => {
    try {
      await api.updateWorkflow(workflowId, {
        agent_id: wf.agent_id,
        name: wf.name,
        description: wf.description,
        nodes: [
          ...wf.nodes.map((n) => ({
            node_type: n.node_type as NodeType,
            name: n.name,
            config: n.config,
            position_x: n.position_x,
            position_y: n.position_y,
          })),
          {
            node_type: nodeType,
            name: `${nodeType}_${nodes.length + 1}`,
            config: {},
            position_x: 100 + (nodes.length % 4) * 200,
            position_y: Math.floor(nodes.length / 4) * 150,
          },
        ],
        edges: wf.edges.map((e) => ({
          source_node_id: e.source_node_id,
          target_node_id: e.target_node_id,
          source_handle: e.source_handle,
          target_handle: e.target_handle,
          condition_expression: e.condition_expression,
        })),
      });
      refresh();
    } catch (e) {
      alert("添加节点失败: " + String(e));
    }
  };

  const removeNode = async (nodeId: string) => {
    try {
      await api.updateWorkflow(workflowId, {
        agent_id: wf.agent_id,
        name: wf.name,
        description: wf.description,
        nodes: wf.nodes
          .filter((n) => n.id !== nodeId)
          .map((n) => ({
            node_type: n.node_type as NodeType,
            name: n.name,
            config: n.config,
            position_x: n.position_x,
            position_y: n.position_y,
          })),
        edges: wf.edges
          .filter((e) => e.source_node_id !== nodeId && e.target_node_id !== nodeId)
          .map((e) => ({
            source_node_id: e.source_node_id,
            target_node_id: e.target_node_id,
            source_handle: e.source_handle,
            target_handle: e.target_handle,
            condition_expression: e.condition_expression,
          })),
      });
      refresh();
    } catch (e) {
      alert("删除节点失败: " + String(e));
    }
  };

  const handleRun = async () => {
    setRunning(true);
    try {
      await api.startRun(workflowId);
      refreshRuns();
    } catch (e) {
      alert("运行失败: " + String(e));
    } finally {
      setRunning(false);
    }
  };

  const handleCancel = async (runId: string) => {
    try {
      await api.cancelRun(runId);
      refreshRuns();
    } catch (e) {
      alert("取消失败: " + String(e));
    }
  };

  const latestRun = runs && runs.length > 0 ? runs[0] : null;

  return (
    <div className="page">
      <div className="page-header">
        <h1>工作流: {wf.name}</h1>
        <div className="page-actions">
          <button className="btn btn-primary" onClick={handleRun} disabled={running}>
            {running ? "运行中..." : "运行"}
          </button>
        </div>
      </div>

      {latestRun && (
        <div className={"run-status-banner run-" + latestRun.status}>
          状态: {latestRun.status}
          {latestRun.status === "running" && (
            <button className="btn btn-sm btn-danger" onClick={() => handleCancel(latestRun.id)}>
              取消
            </button>
          )}
          {latestRun.error && <span className="run-error">错误: {latestRun.error}</span>}
        </div>
      )}

      <div className="workflow-toolbar">
        <span>添加节点:</span>
        {NODE_TYPES.map((nt) => (
          <button key={nt.value} className="btn btn-sm" onClick={() => addNode(nt.value)}>
            +{nt.label}
          </button>
        ))}
      </div>

      <div className="workflow-canvas">
        {nodes.length === 0 ? (
          <div className="empty-state">
            <p>工作流为空。点击上方按钮添加节点。</p>
          </div>
        ) : (
          <div className="node-grid">
            {nodes.map((node) => (
              <div key={node.id} className={"workflow-node node-" + node.node_type}>
                <div className="node-header">
                  <span className="node-type-badge">{node.node_type}</span>
                  <button className="btn-icon" onClick={() => removeNode(node.id)} title="删除节点">
                    {"\u2715"}
                  </button>
                </div>
                <div className="node-body">
                  <strong>{node.name}</strong>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}