import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAgents } from "../hooks/useApi";
import * as api from "../lib/api";

export default function AgentListPage() {
  const { data: agents, loading, error, refresh } = useAgents();
  const navigate = useNavigate();
  const [importing, setImporting] = useState(false);

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`确认删除 Agent「${name}」？此操作不可撤销。`)) return;
    try {
      await api.deleteAgent(id);
      refresh();
    } catch (e) {
      alert("删除失败: " + String(e));
    }
  };

  const handleExport = async (id: string) => {
    try {
      const json = await api.exportAgent(id);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `agent-${id}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert("导出失败: " + String(e));
    }
  };

  const handleImport = async () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      setImporting(true);
      try {
        const text = await file.text();
        JSON.parse(text);
        await api.importAgent(text);
        refresh();
      } catch (e) {
        alert("导入失败: " + String(e));
      } finally {
        setImporting(false);
      }
    };
    input.click();
  };

  if (loading) return <div className="page-loading">加载中...</div>;
  if (error) return <div className="page-error">加载失败: {error}</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h1>Agent 管理</h1>
        <div className="page-actions">
          <button className="btn btn-secondary" onClick={handleImport} disabled={importing}>
            {importing ? "导入中..." : "导入"}
          </button>
          <button className="btn btn-primary" onClick={() => navigate("/agents/new")}>
            创建 Agent
          </button>
        </div>
      </div>

      {agents && agents.length === 0 ? (
        <div className="empty-state">
          <p>还没有创建任何 Agent。</p>
          <button className="btn btn-primary" onClick={() => navigate("/agents/new")}>
            创建第一个 Agent
          </button>
        </div>
      ) : (
        <div className="card-grid">
          {agents?.map((agent) => (
            <div key={agent.id} className="card">
              <div className="card-body">
                <h3>{agent.name}</h3>
                <p className="card-desc">{agent.description || "无描述"}</p>
                <div className="card-meta">
                  <span>模型: {agent.model || "未设置"}</span>
                </div>
              </div>
              <div className="card-actions">
                <button className="btn btn-sm" onClick={() => navigate(`/agents/${agent.id}`)}>
                  编辑
                </button>
                <button className="btn btn-sm" onClick={() => navigate(`/agents/${agent.id}/workflow`)}>
                  工作流
                </button>
                <button className="btn btn-sm" onClick={() => handleExport(agent.id)}>
                  导出
                </button>
                <button className="btn btn-sm btn-danger" onClick={() => handleDelete(agent.id, agent.name)}>
                  删除
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}