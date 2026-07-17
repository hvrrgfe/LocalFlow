import { useState } from "react";
import { useParams } from "react-router-dom";
import { useWorkflows, useWorkflow, useRuns } from "../hooks/useApi";
import * as api from "../lib/api";
import type { NodeType, WorkflowNode, WorkflowEdge } from "../types";

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
  const [selectedWf, setSelectedWf] = useState<string | null>(null);

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

  return (
    <WorkflowCanvas
      workflowId={workflow.id}
      onSelectWorkflow={setSelectedWf}
      allWorkflows={workflows}
    />
  );
}

function WorkflowCanvas({ workflowId, onSelectWorkflow, allWorkflows }: {
  workflowId: string;
  onSelectWorkflow: (id: string | null) => void;
  allWorkflows: any[] | null;
}) {
  const { data: wf, loading, refresh } = useWorkflow(workflowId);
  const { data: runs, refresh: refreshRuns } = useRuns(workflowId);
  const [running, setRunning] = useState(false);
  const [showConfig, setShowConfig] = useState<string | null>(null);
  const [showEdgeModal, setShowEdgeModal] = useState(false);

  if (loading || !wf) return <div className="page-loading">加载中...</div>;

  const nodes = wf.nodes ?? [];
  const edges = wf.edges ?? [];

  const addNode = async (nodeType: NodeType) => {
    let defaultConfig: Record<string, any> = {};
    if (nodeType === "start") defaultConfig.variables = {};
    if (nodeType === "input") defaultConfig.prompt = "";
    if (nodeType === "model") defaultConfig = { provider: "", model_name: "", system_prompt: "", temperature: 0.7, max_tokens: 4096 };
    if (nodeType === "http_request") defaultConfig = { url: "", method: "GET", headers: {}, body: "" };
    if (nodeType === "condition") defaultConfig = { expression: "" };
    if (nodeType === "template") defaultConfig = { template: "", output_variable: "" };
    if (nodeType === "end") defaultConfig = { output_variable: "result" };

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
            config: defaultConfig,
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
      if (showConfig === nodeId) setShowConfig(null);
    } catch (e) {
      alert("删除节点失败: " + String(e));
    }
  };

  const updateNodeConfig = async (nodeId: string, config: any) => {
    try {
      await api.updateWorkflow(workflowId, {
        agent_id: wf.agent_id,
        name: wf.name,
        description: wf.description,
        nodes: wf.nodes.map((n) => ({
          node_type: n.node_type as NodeType,
          name: n.name,
          config: n.id === nodeId ? config : n.config,
          position_x: n.position_x,
          position_y: n.position_y,
        })),
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
      alert("保存配置失败: " + String(e));
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

  const selectedNode = showConfig ? wf.nodes.find((n) => n.id === showConfig) : null;

  return (
    <div className="page">
      <div className="page-header">
        <h1>工作流 {wf.name}</h1>
        <div className="page-actions">
          {allWorkflows && allWorkflows.length > 1 && (
            <select
              className="btn btn-secondary"
              value={wf.id}
              onChange={(e) => onSelectWorkflow(e.target.value)}
              style={{ minWidth: 120 }}
            >
              {allWorkflows.map((w) => (
                <option key={w.id} value={w.id}>{w.name}</option>
              ))}
            </select>
          )}
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
        <span style={{ marginLeft: 12 }}>|</span>
        <button className="btn btn-sm" onClick={() => setShowEdgeModal(true)} disabled={nodes.length < 2}>
          + 添加连线
        </button>
        <button className="btn btn-sm btn-danger" onClick={async () => {
          try {
            await api.updateWorkflow(workflowId, {
              agent_id: wf.agent_id, name: wf.name, description: wf.description,
              nodes: wf.nodes.map((n) => ({
                node_type: n.node_type as NodeType, name: n.name, config: n.config,
                position_x: n.position_x, position_y: n.position_y,
              })),
              edges: [],
            });
            refresh();
          } catch (e) { alert("清空连线失败: " + String(e)); }
        }} disabled={edges.length === 0}>
          清空连线
        </button>
      </div>

      <div style={{ display: "flex", gap: 16 }}>
        <div className="workflow-canvas" style={{ flex: showConfig ? 1 : 1 }}>
          {nodes.length === 0 ? (
            <div className="empty-state">
              <p>工作流为空。点击上方按钮添加节点。</p>
            </div>
          ) : (
            <div>
              <div className="node-grid">
                {nodes.map((node) => (
                  <div
                    key={node.id}
                    className={"workflow-node node-" + node.node_type}
                    onClick={() => setShowConfig(node.id === showConfig ? null : node.id)}
                    style={{ cursor: "pointer", border: node.id === showConfig ? "2px solid var(--primary)" : undefined }}
                  >
                    <div className="node-header">
                      <span className="node-type-badge">{node.node_type}</span>
                      <button className="btn-icon" onClick={(e) => { e.stopPropagation(); removeNode(node.id); }} title="删除节点">
                        {"\u2715"}
                      </button>
                    </div>
                    <div className="node-body">
                      <strong>{node.name}</strong>
                    </div>
                  </div>
                ))}
              </div>
              {edges.length > 0 && (
                <div style={{ marginTop: 16 }}>
                  <h4 style={{ fontSize: "0.85rem", marginBottom: 6 }}>连线 ({edges.length})</h4>
                  <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
                    {edges.map((edge) => {
                      const src = nodes.find((n) => n.id === edge.source_node_id);
                      const tgt = nodes.find((n) => n.id === edge.target_node_id);
                      return (
                        <div key={edge.id} style={{ fontSize: "0.8rem", background: "#f0f0f0", padding: "4px 8px", borderRadius: 4 }}>
                          {src?.name ?? "?"} → {tgt?.name ?? "?"}
                          {edge.condition_expression && <span style={{ color: "#666" }}> [{edge.condition_expression}]</span>}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {selectedNode && (
          <NodeConfigPanel
            node={selectedNode}
            onSave={updateNodeConfig}
            onClose={() => setShowConfig(null)}
          />
        )}
      </div>

      {showEdgeModal && (
        <EdgeModal
          nodes={nodes}
          existingEdges={edges}
          onSave={async (src, tgt, cond) => {
            try {
              await api.updateWorkflow(workflowId, {
                agent_id: wf.agent_id, name: wf.name, description: wf.description,
                nodes: wf.nodes.map((n) => ({
                  node_type: n.node_type as NodeType, name: n.name, config: n.config,
                  position_x: n.position_x, position_y: n.position_y,
                })),
                edges: [
                  ...wf.edges.map((e) => ({
                    source_node_id: e.source_node_id,
                    target_node_id: e.target_node_id,
                    source_handle: e.source_handle,
                    target_handle: e.target_handle,
                    condition_expression: e.condition_expression,
                  })),
                  { source_node_id: src, target_node_id: tgt, source_handle: null, target_handle: null, condition_expression: cond || null },
                ],
              });
              refresh();
              setShowEdgeModal(false);
            } catch (e) { alert("添加连线失败: " + String(e)); }
          }}
          onClose={() => setShowEdgeModal(false)}
        />
      )}
    </div>
  );
}

function NodeConfigPanel({ node, onSave, onClose }: {
  node: WorkflowNode;
  onSave: (id: string, config: any) => Promise<void>;
  onClose: () => void;
}) {
  const [config, setConfig] = useState<any>(node.config ?? {});
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    await onSave(node.id, config);
    setSaving(false);
  };

  const set = (key: string, value: any) => setConfig((prev: any) => ({ ...prev, [key]: value }));

  const renderFields = () => {
    switch (node.node_type) {
      case "start":
        return <p className="text-muted">Start 节点无需额外配置。作为工作流入口。</p>;

      case "input":
        return (
          <div className="form-group">
            <label>提示词模板</label>
            <textarea
              rows={6}
              className="font-mono"
              placeholder="输入提示词，使用 {{variable}} 引用上游输出"
              value={config.prompt ?? ""}
              onChange={(e) => set("prompt", e.target.value)}
            />
          </div>
        );

      case "model":
        return (
          <>
            <div className="form-group">
              <label>Provider ID</label>
              <input
                type="text"
                placeholder="如 provider/openai"
                value={config.provider ?? ""}
                onChange={(e) => set("provider", e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>模型名称</label>
              <input
                type="text"
                placeholder="如 gpt-4o, deepseek-chat"
                value={config.model_name ?? ""}
                onChange={(e) => set("model_name", e.target.value)}
              />
            </div>
            <div className="form-group">
              <label>系统提示词</label>
              <textarea
                rows={4}
                className="font-mono"
                value={config.system_prompt ?? ""}
                onChange={(e) => set("system_prompt", e.target.value)}
              />
            </div>
            <div className="form-row">
              <div className="form-group" style={{ flex: 1 }}>
                <label>Temperature</label>
                <input type="number" min={0} max={2} step={0.1}
                  value={config.temperature ?? 0.7}
                  onChange={(e) => set("temperature", parseFloat(e.target.value) || 0.7)} />
              </div>
              <div className="form-group" style={{ flex: 1 }}>
                <label>Max Tokens</label>
                <input type="number" min={1} max={128000}
                  value={config.max_tokens ?? 4096}
                  onChange={(e) => set("max_tokens", parseInt(e.target.value) || 4096)} />
              </div>
            </div>
          </>
        );

      case "http_request":
        return (
          <>
            <div className="form-group">
              <label>请求 URL</label>
              <input type="text" placeholder="https://api.example.com/data"
                value={config.url ?? ""}
                onChange={(e) => set("url", e.target.value)} />
            </div>
            <div className="form-group">
              <label>请求方法</label>
              <select value={config.method ?? "GET"}
                onChange={(e) => set("method", e.target.value)}>
                <option value="GET">GET</option>
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
                <option value="PATCH">PATCH</option>
                <option value="DELETE">DELETE</option>
              </select>
            </div>
            <div className="form-group">
              <label>Headers (JSON)</label>
              <textarea className="font-mono" rows={4}
                placeholder='{"Content-Type": "application/json"}'
                value={typeof config.headers === "object" ? JSON.stringify(config.headers, null, 2) : config.headers ?? ""}
                onChange={(e) => {
                  try { set("headers", JSON.parse(e.target.value)); }
                  catch { set("headers", e.target.value); }
                }} />
            </div>
            <div className="form-group">
              <label>请求体 (Body)</label>
              <textarea className="font-mono" rows={6}
                placeholder='{"key": "value"}'
                value={config.body ?? ""}
                onChange={(e) => set("body", e.target.value)} />
            </div>
          </>
        );

      case "condition":
        return (
          <div className="form-group">
            <label>条件表达式</label>
            <textarea className="font-mono" rows={4}
              placeholder={"示例:\n{{outputs.node_id.value}} > 100\n{{outputs.node_id.status}} == \"success\""}
              value={config.expression ?? ""}
              onChange={(e) => set("expression", e.target.value)} />
            <p className="text-muted" style={{ marginTop: 4 }}>
              支持 ==, !=, &gt;, &lt;, &gt;=, &lt;=, true, false
            </p>
          </div>
        );

      case "template":
        return (
          <>
            <div className="form-group">
              <label>模板内容</label>
              <textarea className="font-mono" rows={8}
                placeholder={"使用 {{variable}} 或 {{outputs.node_id.path}} 引用值"}
                value={config.template ?? ""}
                onChange={(e) => set("template", e.target.value)} />
            </div>
            <div className="form-group">
              <label>输出变量名</label>
              <input type="text" placeholder="result"
                value={config.output_variable ?? ""}
                onChange={(e) => set("output_variable", e.target.value)} />
            </div>
          </>
        );

      case "end":
        return (
          <div className="form-group">
            <label>输出变量名</label>
            <input type="text" placeholder="result"
              value={config.output_variable ?? "result"}
              onChange={(e) => set("output_variable", e.target.value)} />
            <p className="text-muted" style={{ marginTop: 4 }}>
              End 节点收集此变量的值作为工作流最终输出。
            </p>
          </div>
        );

      default:
        return <p className="text-muted">未知节点类型</p>;
    }
  };

  return (
    <div style={{
      width: 320, background: "#fff", border: "1px solid var(--border)",
      borderRadius: "var(--radius)", padding: 16,
      boxShadow: "0 2px 8px rgba(0,0,0,0.1)",
      alignSelf: "flex-start", position: "sticky", top: 16,
    }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <h3 style={{ fontSize: "1rem" }}>{node.name}</h3>
        <button className="btn-icon" onClick={onClose}>取消</button>
      </div>
      <p className="text-muted" style={{ marginBottom: 12, fontSize: "0.8rem" }}>
        类型: {node.node_type} | ID: {node.id.slice(0, 8)}
      </p>
      {renderFields()}
      <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
        <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
          {saving ? "保存中..." : "保存配置"}
        </button>
      </div>
    </div>
  );
}

function EdgeModal({ nodes, existingEdges, onSave, onClose }: {
  nodes: WorkflowNode[];
  existingEdges: WorkflowEdge[];
  onSave: (source: string, target: string, condition: string) => Promise<void>;
  onClose: () => void;
}) {
  const [src, setSrc] = useState("");
  const [tgt, setTgt] = useState("");
  const [cond, setCond] = useState("");
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    if (!src || !tgt) { alert("请选择源节点和目标节点"); return; }
    if (src === tgt) { alert("不能连接到自身"); return; }
    if (existingEdges.some((e) => e.source_node_id === src && e.target_node_id === tgt)) {
      alert("该连线已存在"); return;
    }
    setSaving(true);
    await onSave(src, tgt, cond);
    setSaving(false);
  };

  return (
    <div style={{
      position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)",
      display: "flex", alignItems: "center", justifyContent: "center", zIndex: 100,
    }} onClick={onClose}>
      <div style={{
        background: "#fff", borderRadius: "var(--radius)", padding: 24,
        width: 400, maxWidth: "90vw",
      }} onClick={(e) => e.stopPropagation()}>
        <h3 style={{ marginBottom: 16 }}>添加连线</h3>
        <div className="form-group">
          <label>源节点</label>
          <select value={src} onChange={(e) => setSrc(e.target.value)}>
            <option value="">-- 选择 --</option>
            {nodes.map((n) => (
              <option key={n.id} value={n.id}>{n.name} ({n.node_type})</option>
            ))}
          </select>
        </div>
        <div className="form-group">
          <label>目标节点</label>
          <select value={tgt} onChange={(e) => setTgt(e.target.value)}>
            <option value="">-- 选择 --</option>
            {nodes.map((n) => (
              <option key={n.id} value={n.id}>{n.name} ({n.node_type})</option>
            ))}
          </select>
        </div>
        <div className="form-group">
          <label>条件表达式（可选）</label>
          <input type="text" placeholder="仅 Condition 节点后使用"
            value={cond} onChange={(e) => setCond(e.target.value)} />
        </div>
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button className="btn" onClick={onClose}>取消</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? "添加中..." : "添加"}
          </button>
        </div>
      </div>
    </div>
  );
}