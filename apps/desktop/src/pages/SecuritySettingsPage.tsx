import { useState } from "react";
import { useAuditLogs } from "../hooks/useApi";
import * as api from "../lib/api";
import type { UrlValidationResult } from "../types";

export default function SecuritySettingsPage() {
  const { data: logs, loading, refresh } = useAuditLogs(100);
  const [urlToCheck, setUrlToCheck] = useState("");
  const [urlResult, setUrlResult] = useState<UrlValidationResult | null>(null);
  const [checking, setChecking] = useState(false);

  const handleCheckUrl = async () => {
    if (!urlToCheck.trim()) return;
    setChecking(true);
    setUrlResult(null);
    try {
      const result = await api.validateUrl(urlToCheck);
      setUrlResult(result);
    } catch (e) {
      setUrlResult({ valid: false, message: String(e) });
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>安全设置</h1>
        <div className="page-actions">
          <button className="btn btn-secondary" onClick={refresh}>刷新</button>
        </div>
      </div>

      <section className="section">
        <h2>URL 安全检查器</h2>
        <p className="text-muted">检查 URL 是否允许访问，防止 SSRF 攻击。</p>
        <div className="form-row">
          <input
            type="text"
            className="flex-1"
            placeholder="输入 URL 进行检查"
            value={urlToCheck}
            onChange={(e) => setUrlToCheck(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleCheckUrl()}
          />
          <button className="btn btn-primary" onClick={handleCheckUrl} disabled={checking}>
            {checking ? "检查中..." : "检查"}
          </button>
        </div>
        {urlResult && (
          <div className={"url-result " + (urlResult.valid ? "url-ok" : "url-blocked")}>
            {urlResult.valid ? "✓ 允许访问" : "✗ 已阻止"}
            <p className="text-muted">{urlResult.message}</p>
          </div>
        )}
      </section>

      <section className="section">
        <h2>安全审计日志</h2>
        <p className="text-muted">记录所有敏感操作，日志不会泄漏密钥。</p>

        {loading ? (
          <div className="page-loading">加载中...</div>
        ) : logs && logs.length > 0 ? (
          <table className="table">
            <thead>
              <tr>
                <th>时间</th>
                <th>事件类型</th>
                <th>实体</th>
                <th>详情</th>
              </tr>
            </thead>
            <tbody>
              {logs.map((log) => (
                <tr key={log.id}>
                  <td className="text-nowrap">{new Date(log.created_at).toLocaleString()}</td>
                  <td><code>{log.event_type}</code></td>
                  <td>{log.entity_type}{log.entity_id ? " / " + log.entity_id.slice(0, 8) : ""}</td>
                  <td className="text-muted">{log.details ? JSON.stringify(log.details).slice(0, 100) : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="empty-state">
            <p>尚无审计日志。</p>
          </div>
        )}
      </section>
    </div>
  );
}