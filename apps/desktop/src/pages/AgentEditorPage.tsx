import { useState, useEffect } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAgent } from "../hooks/useApi";
import * as api from "../lib/api";
import type { AgentInput } from "../types";

const emptyForm: AgentInput = {
  name: "",
  description: "",
  system_prompt: "",
  model: "",
  temperature: 0.7,
  max_tokens: 4096,
  permissions: {
    allowed_hosts: [],
    allowed_networks: [],
    allow_file_access: false,
    allow_loopback: false,
    max_nodes: 50,
    max_loops: 100,
    max_request_size: 10485760,
    max_response_size: 10485760,
    max_execution_seconds: 600,
  },
};

export default function AgentEditorPage() {
  const { id } = useParams<{ id: string }>();
  const isNew = id === "new" || !id;
  const { data: agent, loading: loadingAgent } = useAgent(isNew ? undefined : id);
  const navigate = useNavigate();

  const [form, setForm] = useState<AgentInput>(emptyForm);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!isNew && agent) {
      setForm({
        name: agent.name,
        description: agent.description,
        system_prompt: agent.system_prompt,
        model: agent.model,
        temperature: agent.temperature,
        max_tokens: agent.max_tokens,
        permissions: agent.permissions,
      });
    }
  }, [agent, isNew]);

  const handleSave = async () => {
    if (!form.name.trim()) {
      alert("名称不能为空");
      return;
    }
    setSaving(true);
    try {
      if (isNew) {
        await api.createAgent(form);
      } else {
        await api.updateAgent(id!, form);
      }
      navigate("/agents");
    } catch (e) {
      alert("保存失败: " + String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!isNew && loadingAgent) return <div className="page-loading">加载中...</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h1>{isNew ? "创建 Agent" : "编辑 Agent"}</h1>
        <div className="page-actions">
          <button className="btn btn-secondary" onClick={() => navigate("/agents")}>返回</button>
          <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
            {saving ? "保存中..." : "保存"}
          </button>
        </div>
      </div>

      <form className="form" onSubmit={(e) => { e.preventDefault(); handleSave(); }}>
        <div className="form-group">
          <label>名称 *</label>
          <input
            type="text"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            placeholder="Agent 名称"
            required
          />
        </div>
        <div className="form-group">
          <label>描述</label>
          <textarea
            value={form.description ?? ""}
            onChange={(e) => setForm({ ...form, description: e.target.value })}
            placeholder="Agent 描述（可选）"
            rows={2}
          />
        </div>
        <div className="form-group">
          <label>系统提示词</label>
          <textarea
            value={form.system_prompt ?? ""}
            onChange={(e) => setForm({ ...form, system_prompt: e.target.value })}
            placeholder="系统提示词（可选）"
            rows={6}
            className="font-mono"
          />
        </div>

        <fieldset className="form-fieldset">
          <legend>模型配置</legend>
          <div className="form-row">
            <div className="form-group flex-1">
              <label>模型 ID</label>
              <input
                type="text"
                value={form.model ?? ""}
                onChange={(e) => setForm({ ...form, model: e.target.value })}
                placeholder="gpt-4o, deepseek-chat 等"
              />
            </div>
            <div className="form-group">
              <label>Temperature</label>
              <input
                type="number"
                min={0}
                max={2}
                step={0.1}
                value={form.temperature ?? 0.7}
                onChange={(e) => setForm({ ...form, temperature: parseFloat(e.target.value) || 0.7 })}
              />
            </div>
            <div className="form-group">
              <label>Max Tokens</label>
              <input
                type="number"
                min={1}
                max={128000}
                value={form.max_tokens ?? 4096}
                onChange={(e) => setForm({ ...form, max_tokens: parseInt(e.target.value) || 4096 })}
              />
            </div>
          </div>
        </fieldset>

        <fieldset className="form-fieldset">
          <legend>权限策略</legend>
          <div className="form-row">
            <div className="form-group">
              <label>最大节点数</label>
              <input
                type="number"
                min={1}
                max={200}
                value={form.permissions?.max_nodes ?? 50}
                onChange={(e) => setForm({ ...form, permissions: { ...form.permissions!, max_nodes: parseInt(e.target.value) || 50 } })}
              />
            </div>
            <div className="form-group">
              <label>最大请求体 (bytes)</label>
              <input
                type="number"
                value={form.permissions?.max_request_size ?? 10485760}
                onChange={(e) => setForm({ ...form, permissions: { ...form.permissions!, max_request_size: parseInt(e.target.value) || 10485760 } })}
              />
            </div>
            <div className="form-group">
              <label>最大执行时间 (秒)</label>
              <input
                type="number"
                value={form.permissions?.max_execution_seconds ?? 600}
                onChange={(e) => setForm({ ...form, permissions: { ...form.permissions!, max_execution_seconds: parseInt(e.target.value) || 600 } })}
              />
            </div>
          </div>
          <div className="form-row">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={form.permissions?.allow_file_access ?? false}
                onChange={(e) => setForm({ ...form, permissions: { ...form.permissions!, allow_file_access: e.target.checked } })}
              />
              允许文件访问（危险）
            </label>
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={form.permissions?.allow_loopback ?? false}
                onChange={(e) => setForm({ ...form, permissions: { ...form.permissions!, allow_loopback: e.target.checked } })}
              />
              允许本地回环地址（危险）
            </label>
          </div>
        </fieldset>
      </form>
    </div>
  );
}
