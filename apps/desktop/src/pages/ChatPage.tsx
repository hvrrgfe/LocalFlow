import { useState, useRef, useEffect } from "react";
import { useAgents } from "../hooks/useApi";
import * as api from "../lib/api";

interface Message {
  role: "user" | "assistant";
  content: string;
}

export default function ChatPage() {
  const { data: agents, loading } = useAgents();
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [lastUsage, setLastUsage] = useState<string>("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSend = async () => {
    const msg = input.trim();
    if (!msg || !selectedAgentId) return;

    setInput("");
    setMessages((prev) => [...prev, { role: "user", content: msg }]);
    setSending(true);

    try {
      const res = await api.chatWithAgent(selectedAgentId, msg);
      setMessages((prev) => [...prev, { role: "assistant", content: res.content }]);
      if (res.usage) {
        setLastUsage(`Tokens: ${res.usage.prompt_tokens} 输入 / ${res.usage.completion_tokens} 输出`);
      }
    } catch (e: any) {
      setMessages((prev) => [...prev, { role: "assistant", content: "错误: " + String(e) }]);
    } finally {
      setSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  if (loading) return <div className="page-loading">加载中...</div>;

  const selectedAgent = agents?.find((a) => a.id === selectedAgentId);

  return (
    <div className="page" style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 48px)", maxWidth: "none", padding: 0 }}>
      {/* Top bar */}
      <div style={{ padding: "12px 24px", borderBottom: "1px solid var(--border)", display: "flex", alignItems: "center", gap: 12, background: "#fff" }}>
        <h1 style={{ fontSize: "1.1rem", margin: 0 }}>对话</h1>
        <select
          value={selectedAgentId}
          onChange={(e) => { setSelectedAgentId(e.target.value); setMessages([]); setLastUsage(""); }}
          style={{ minWidth: 200, padding: "6px 10px", borderRadius: 6, border: "1px solid var(--border)" }}
        >
          <option value="">-- 选择 Agent --</option>
          {agents?.map((a) => (
            <option key={a.id} value={a.id}>{a.name}</option>
          ))}
        </select>
        {selectedAgent && (
          <span style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>
            模型: {selectedAgent.model || "未设置"}
          </span>
        )}
      </div>

      {/* Chat messages */}
      <div style={{ flex: 1, overflowY: "auto", padding: "16px 24px" }}>
        {!selectedAgentId ? (
          <div className="empty-state" style={{ marginTop: 60 }}>
            <p>请先从上方下拉列表选择一个 Agent</p>
            <p className="text-muted">配置好 API Key 后即可开始对话</p>
          </div>
        ) : messages.length === 0 ? (
          <div className="empty-state" style={{ marginTop: 60 }}>
            <p>开始与 {selectedAgent?.name} 对话</p>
            <p className="text-muted">
              {selectedAgent?.system_prompt ? `系统提示词: ${selectedAgent.system_prompt.slice(0, 100)}...` : "无系统提示词"}
            </p>
          </div>
        ) : (
          <div style={{ maxWidth: 800, margin: "0 auto" }}>
            {messages.map((m, i) => (
              <div key={i} style={{
                display: "flex",
                marginBottom: 16,
                justifyContent: m.role === "user" ? "flex-end" : "flex-start",
              }}>
                <div style={{
                  maxWidth: "70%",
                  padding: "10px 14px",
                  borderRadius: 12,
                  background: m.role === "user" ? "var(--primary)" : "#f0f0f0",
                  color: m.role === "user" ? "#fff" : "var(--text)",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                  lineHeight: 1.5,
                }}>
                  {m.content}
                </div>
              </div>
            ))}
            {sending && (
              <div style={{ display: "flex", marginBottom: 16 }}>
                <div style={{
                  padding: "10px 14px", borderRadius: 12,
                  background: "#f0f0f0", color: "var(--text-muted)",
                }}>
                  正在思考...
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </div>
        )}
      </div>

      {/* Usage info */}
      {lastUsage && (
        <div style={{ padding: "4px 24px", fontSize: "0.75rem", color: "var(--text-muted)", textAlign: "right", borderTop: "1px solid var(--border)" }}>
          {lastUsage}
        </div>
      )}

      {/* Input area */}
      <div style={{ padding: "12px 24px 16px", borderTop: "1px solid var(--border)", background: "#fff" }}>
        <div style={{ maxWidth: 800, margin: "0 auto", display: "flex", gap: 8 }}>
          <textarea
            style={{
              flex: 1, padding: "10px 14px", borderRadius: 8,
              border: "1px solid var(--border)", resize: "none",
              fontSize: "0.9rem", lineHeight: 1.5, minHeight: 44, maxHeight: 120,
            }}
            rows={1}
            placeholder={selectedAgentId ? "输入消息，Enter 发送，Shift+Enter 换行" : "请先选择一个 Agent"}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={!selectedAgentId || sending}
          />
          <button
            className="btn btn-primary"
            onClick={handleSend}
            disabled={!selectedAgentId || !input.trim() || sending}
            style={{ alignSelf: "flex-end", padding: "10px 20px" }}
          >
            发送
          </button>
        </div>
      </div>
    </div>
  );
}
