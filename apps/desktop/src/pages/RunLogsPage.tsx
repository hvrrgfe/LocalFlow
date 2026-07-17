import { useState, useEffect } from "react";
import { useParams } from "react-router-dom";
import { useRuns, useWorkflows } from "../hooks/useApi";
import * as api from "../lib/api";
import type { NodeRun } from "../types";

export default function RunLogsPage() {
  const { workflowId } = useParams<{ workflowId: string }>();
  const { data: allWorkflows } = useWorkflows(undefined);

  const [selectedWf, setSelectedWf] = useState<string>(workflowId ?? "");
  const { data: runs, loading, refresh } = useRuns(selectedWf || undefined);
  const [nodeRuns, setNodeRuns] = useState<Record<string, NodeRun[]>>({});
  const [expandedRun, setExpandedRun] = useState<string | null>(null);

  useEffect(() => {
    if (workflowId) setSelectedWf(workflowId);
  }, [workflowId]);

  const toggleExpand = async (runId: string) => {
    if (expandedRun === runId) {
      setExpandedRun(null);
      return;
    }
    setExpandedRun(runId);
    if (!nodeRuns[runId]) {
      try {
        const nodes = await api.getNodeRuns(runId);
        setNodeRuns((prev) => ({ ...prev, [runId]: nodes }));
      } catch (e) {
        // ignore
      }
    }
  };

  const handleCancel = async (runId: string) => {
    try {
      await api.cancelRun(runId);
      refresh();
    } catch (e) {
      alert("取消失败: " + String(e));
    }
  };

  const handleRetry = async (runId: string) => {
    if (!selectedWf) return;
    try {
      await api.retryRun(selectedWf, runId);
      refresh();
    } catch (e) {
      alert("重试失败: " + String(e));
    }
  };

  const statusClass = (status: string) => {
    switch (status) {
      case "succeeded": return "status-ok";
      case "failed": case "timed_out": case "cancelled": return "status-err";
      case "running": return "status-running";
      default: return "status-pending";
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>运行日志</h1>
        <div className="page-actions">
          <button className="btn btn-secondary" onClick={refresh}>刷新</button>
        </div>
      </div>

      <div className="form-row">
        <label>选择工作流</label>
        <select value={selectedWf} onChange={(e) => setSelectedWf(e.target.value)}>
          <option value="">-- 全部 --</option>
          {allWorkflows?.map((wf) => (
            <option key={wf.id} value={wf.id}>
              {wf.name} ({wf.agent_id.slice(0, 8)})
            </option>
          ))}
        </select>
      </div>

      {loading ? (
        <div className="page-loading">加载中...</div>
      ) : runs && runs.length > 0 ? (
        <div className="run-list">
          {runs.map((run) => (
            <div key={run.id} className="run-card" onClick={() => toggleExpand(run.id)}>
              <div className="run-card-header">
                <span className={"run-status-dot " + statusClass(run.status)} />
                <span className="run-status-label">{run.status}</span>
                <span className="run-time">
                  {run.started_at ? new Date(run.started_at).toLocaleString() : "未开始"}
                </span>
                {run.completed_at && (
                  <span className="run-duration">
                    ({Math.round(
                      (new Date(run.completed_at).getTime() - new Date(run.started_at!).getTime()) / 1000
                    )}s)
                  </span>
                )}
              </div>
              <div className="run-card-actions">
                {run.status === "running" && (
                  <button className="btn btn-sm btn-danger" onClick={(e) => { e.stopPropagation(); handleCancel(run.id); }}>
                    取消
                  </button>
                )}
                {(run.status === "failed" || run.status === "cancelled" || run.status === "timed_out") && (
                  <button className="btn btn-sm" onClick={(e) => { e.stopPropagation(); handleRetry(run.id); }}>
                    重试
                  </button>
                )}
              </div>
              {run.error && <div className="run-error-detail">{run.error}</div>}

              {expandedRun === run.id && nodeRuns[run.id] && (
                <div className="node-runs">
                  <h4>节点运行详情</h4>
                  <table className="table">
                    <thead>
                      <tr>
                        <th>节点</th>
                        <th>类型</th>
                        <th>状态</th>
                        <th>尝试次数</th>
                        <th>错误</th>
                      </tr>
                    </thead>
                    <tbody>
                      {nodeRuns[run.id].map((nr) => (
                        <tr key={nr.id}>
                          <td>{nr.node_id.slice(0, 8)}</td>
                          <td>{nr.node_type}</td>
                          <td className={statusClass(nr.status)}>{nr.status}</td>
                          <td>{nr.attempts}/{nr.max_attempts}</td>
                          <td className="error-text">{nr.error || ""}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          ))}
        </div>
      ) : (
        <div className="empty-state">
          <p>没有运行记录。</p>
        </div>
      )}
    </div>
  );
}