import { useState } from "react";
import { useProviders, useSecrets } from "../hooks/useApi";
import * as api from "../lib/api";

export default function ApiManagementPage() {
  const { data: providers, loading: loadingProviders, refresh: refreshProviders } = useProviders();
  const { data: secrets, loading: loadingSecrets, refresh: refreshSecrets } = useSecrets();

  const [newProvider, setNewProvider] = useState({ id: "", name: "", baseUrl: "" });
  const [newSecret, setNewSecret] = useState({ key: "", value: "" });
  const [savingProvider, setSavingProvider] = useState(false);
  const [savingSecret, setSavingSecret] = useState(false);

  const handleSaveProvider = async () => {
    if (!newProvider.id.trim() || !newProvider.name.trim() || !newProvider.baseUrl.trim()) {
      alert("请填写所有字段");
      return;
    }
    setSavingProvider(true);
    try {
      await api.saveProvider(newProvider.id, newProvider.name, newProvider.baseUrl);
      setNewProvider({ id: "", name: "", baseUrl: "" });
      refreshProviders();
    } catch (e) {
      alert("保存失败: " + String(e));
    } finally {
      setSavingProvider(false);
    }
  };

  const handleDeleteProvider = async (id: string) => {
    if (!confirm("确认删除此 Provider？关联的 API Key 不会自动删除。")) return;
    try {
      await api.deleteProvider(id);
      refreshProviders();
    } catch (e) {
      alert("删除失败: " + String(e));
    }
  };

  const handleSaveSecret = async () => {
    if (!newSecret.key.trim() || !newSecret.value.trim()) {
      alert("请填写所有字段");
      return;
    }
    setSavingSecret(true);
    try {
      await api.storeSecret(newSecret.key, newSecret.value);
      setNewSecret({ key: "", value: "" });
      refreshSecrets();
    } catch (e) {
      alert("保存失败: " + String(e));
    } finally {
      setSavingSecret(false);
    }
  };

  const handleDeleteSecret = async (key: string) => {
    if (!confirm("确认删除此密钥？")) return;
    try {
      await api.deleteSecret(key);
      refreshSecrets();
    } catch (e) {
      alert("删除失败: " + String(e));
    }
  };

  if (loadingProviders && loadingSecrets) return <div className="page-loading">加载中...</div>;

  return (
    <div className="page">
      <div className="page-header">
        <h1>API 管理</h1>
      </div>

      <section className="section">
        <h2>API Provider</h2>
        <div className="manage-section">
          <div className="form-row">
            <input
              type="text"
              placeholder="Provider ID (如 deepseek)"
              value={newProvider.id}
              onChange={(e) => setNewProvider({ ...newProvider, id: e.target.value })}
            />
            <input
              type="text"
              placeholder="名称 (如 DeepSeek)"
              value={newProvider.name}
              onChange={(e) => setNewProvider({ ...newProvider, name: e.target.value })}
            />
            <input
              type="text"
              placeholder="Base URL (如 https://api.deepseek.com/v1)"
              value={newProvider.baseUrl}
              onChange={(e) => setNewProvider({ ...newProvider, baseUrl: e.target.value })}
            />
            <button className="btn btn-primary" onClick={handleSaveProvider} disabled={savingProvider}>
              {savingProvider ? "保存中..." : "添加"}
            </button>
          </div>

          {providers && providers.length > 0 ? (
            <table className="table">
              <thead>
                <tr>
                  <th>ID</th>
                  <th>名称</th>
                  <th>Base URL</th>
                  <th>API Key</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {providers.map((p) => (
                  <tr key={p.id}>
                    <td>{p.id}</td>
                    <td>{p.name}</td>
                    <td className="font-mono">{p.base_url}</td>
                    <td>{p.has_api_key ? "✓ 已配置" : "✗ 未配置"}</td>
                    <td>
                      <button className="btn btn-sm btn-danger" onClick={() => handleDeleteProvider(p.id)}>
                        删除
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="empty-state">
              <p>尚未配置任何 API Provider。</p>
            </div>
          )}
        </div>
      </section>

      <section className="section">
        <h2>API Key 管理</h2>
        <p className="text-muted">API Key 加密存储，前端无法读取已保存的值。</p>
        <div className="manage-section">
          <div className="form-row">
            <input
              type="text"
              placeholder="Key (如 provider/deepseek)"
              value={newSecret.key}
              onChange={(e) => setNewSecret({ ...newSecret, key: e.target.value })}
            />
            <input
              type="password"
              placeholder="API Key 值"
              value={newSecret.value}
              onChange={(e) => setNewSecret({ ...newSecret, value: e.target.value })}
            />
            <button className="btn btn-primary" onClick={handleSaveSecret} disabled={savingSecret}>
              {savingSecret ? "保存中..." : "保存"}
            </button>
          </div>

          {secrets && secrets.length > 0 ? (
            <table className="table">
              <thead>
                <tr>
                  <th>Key</th>
                  <th>状态</th>
                  <th>操作</th>
                </tr>
              </thead>
              <tbody>
                {secrets.map((s) => (
                  <tr key={s.key}>
                    <td className="font-mono">{s.key}</td>
                    <td>{s.exists ? "✓ 已配置" : "✗ 不存在"}</td>
                    <td>
                      <button className="btn btn-sm btn-danger" onClick={() => handleDeleteSecret(s.key)}>
                        删除
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="empty-state">
              <p>尚未配置任何 API Key。</p>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}
