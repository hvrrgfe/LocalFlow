<div align="center">

# 🤖 LocalFlow

**Build AI agents and workflows — 100% local, fully private.**

[![Rust](https://img.shields.io/badge/Rust-2024-dea584?logo=rust)](https://www.rust-lang.org)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-ffc131?logo=tauri)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-18-61dafb?logo=react)](https://react.dev)
[![Flutter](https://img.shields.io/badge/Flutter-3-02569b?logo=flutter)](https://flutter.dev)
[![SQLite](https://img.shields.io/badge/SQLite-3-003b57?logo=sqlite)](https://www.sqlite.org)
[![License MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/hvrrgfe/LocalFlow)](https://github.com/hvrrgfe/LocalFlow/releases)

**中文 · [English](#english) · [文档](#文档--documentation)**

</div>

---

## What is LocalFlow?

LocalFlow is a **local-first** AI Agent and workflow builder. No cloud accounts, no data leakage — everything stays on your machine.

Think of it as a self-hosted alternative to Coze, Dify, or LangFlow, but with a security-first design: API keys live in your system keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service, Android Keystore), logs are automatically redacted, and all dangerous features are off by default.

### Quick demo

```python
# Create an Agent in LocalFlow:
# - Give it a system prompt
# - Point it to an OpenAI-compatible API
# - Build a DAG workflow with drag & drop
# - Run it — all locally, all private
```

---

## ✨ Features

| | Feature | Detail |
|---|---|---|
| 🔒 | **Local-First** | Everything stored on your machine. No uploads. Ever. |
| 🔑 | **Keychain-Backed Secrets** | API keys stored in OS keychain, never in configs, logs, or exports |
| 🛡️ | **SSRF Protection** | Blocks localhost, private IPs, cloud metadata by default |
| 📝 | **Audit Logging** | Every sensitive action logged; secrets auto-redacted |
| 🔄 | **DAG Workflow Engine** | 7 node types, cycle detection, cancel/retry/resume |
| 🎯 | **Zero-Trust API** | External responses can't modify prompts, permissions, or tools |
| 📦 | **Cross-Platform** | Windows (Tauri + React) + Android (Flutter), shared Rust core |

---

## 📦 Download

| Platform | Link |
|----------|------|
| 🪟 Windows EXE | [Download v0.1.0](https://github.com/hvrrgfe/LocalFlow/releases/tag/v0.1.0) |
| 📱 Android APK | Coming soon |
| 🐧 macOS / Linux | Planned |

> Or build from source — see [Build](#-build-from-source) below.

---

## 🚀 Quick Start (Windows)

### Prerequisites

- [Rust](https://rustup.rs) 1.78+ (MSVC toolchain)
- [Node.js](https://nodejs.org) 18+
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (pre-installed on Windows 10+)

### Run from source

```powershell
# 1. Build frontend
cd apps\desktop
npm install
npm run build

# 2. Build and run Tauri app
cd ..\..

# Set proper linker environment
$env:LIB = "path\to\your\libs"
cargo run -p localflow-desktop
```

### Dev mode (hot-reload frontend)

```powershell
# Terminal 1: Vite dev server
cd apps\desktop
npm run dev

# Terminal 2: Tauri window
cargo run -p localflow-desktop
```

Open `http://localhost:1420` to preview the frontend (Rust calls won't work without the backend).

---

## 🧱 Architecture

```
┌─────────────────────────────────────────────────────┐
│                 User Interface Layer                 │
├────────────────────┬────────────────────────────────┤
│  🪟 Windows        │  📱 Android                    │
│  (Tauri 2 + React) │  (Flutter + Provider)          │
├────────────────────┴────────────────────────────────┤
│                  Tauri Commands / FFI                │
├─────────────────────────────────────────────────────┤
│                  Rust Core Library                   │
├───────────┬────────┬────────┬────────┬──────────────┤
│  Storage  │ Work-  │ Model  │ Secret │  Security    │
│  (SQLite) │ flow   │ Provid-│ Vault  │  (SSRF,     │
│           │ Engine │ ers    │        │   redact)    │
├───────────┴────────┴────────┴────────┴──────────────┤
│                    SQLite / OS Keychain              │
└─────────────────────────────────────────────────────┘
```

### Crate map

| Crate | What it does |
|-------|-------------|
| `crates/core` | Domain models, errors, state machine |
| `crates/storage` | SQLite via rusqlite, migrations |
| `crates/secret-vault` | Keychain abstraction (InMemory, OS Keychain) |
| `crates/security` | URL validation, path sanitization, log redaction |
| `crates/audit` | Audit event logging |
| `crates/workflow-engine` | DAG executor with retry/resume |
| `crates/model-providers` | OpenAI-compatible + custom HTTP clients |
| `crates/api-tools` | OpenAPI 3 JSON/YAML parser |

---

## 🔐 Security Model

LocalFlow is built **Secure by Default**. Here's what that means:

### API Keys

```
❌ Never in:   config files, logs, error messages, exports, memory dumps
✅ Only in:    OS keychain (Windows Credential Manager / macOS Keychain /
               Linux Secret Service / Android Keystore)
```

Access via secret references: `secret://provider/deepseek`

### Network Restrictions

Default blocklist:
- `localhost`, `127.0.0.1`, `0.0.0.0`
- Private subnets (`10.x`, `172.16-31.x`, `192.168.x`)
- Cloud metadata endpoints (`169.254.169.254`)
- `file://` protocol

Override via `allowed_hosts` allowlist and explicit permission flags.

### Workflow Safety

- External API responses = **untrusted data**
- External content **cannot** modify: system prompts, permissions, tool allowlists
- No Shell/Python/JS execution nodes (planned for future with explicit opt-in)
- Single node failure never crashes the process
- All async tasks support: cancel, timeout (configurable), limited retry (default 3), failure recovery

### Node State Machine

```
PENDING → RUNNING → SUCCEEDED
                   → FAILED → (retry → RUNNING) or terminal
                   → CANCELLED
                   → PAUSED / WAITING_APPROVAL
```

Default timeouts: HTTP **10s** · Model **120s** · Workflow **10min**

---

## 🧪 Testing

```bash
# Run the full suite (136+ tests)
cargo test --workspace

# Lint and format
cargo fmt --all
cargo clippy --all-targets
```

Test coverage includes: API timeout, 500 errors, malformed JSON, network disconnect, DB restart, process kill, workflow cancel, duplicate runs, malicious paths, SSRF addresses, oversized input, secret leakage.

---

## 🗺️ Roadmap

- [x] **MVP** — Agent CRUD, OpenAI API, DAG workflow, audit logging
- [ ] **Phase 2** — OpenAPI import, Template node, Condition node
- [ ] **Phase 3** — Android app (Flutter), plugin system (sandboxed)
- [ ] **Phase 4** — Local LLM support (llama.cpp, ONNX), offline mode
- [ ] **Future** — Knowledge base (RAG), collaborative workflows, P2P sync

---

## 📖 Documentation & Documentation

| Resource | Link |
|----------|------|
| Desktop Build Guide | [apps/desktop/BUILD.md](apps/desktop/BUILD.md) |
| Android Build Guide | [apps/mobile/BUILD_APK.md](apps/mobile/BUILD_APK.md) |
| GitHub Setup | [scripts/setup-github.ps1](scripts/setup-github.ps1) |
| Security Policy | [SECURITY.md](SECURITY.md) |

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md) code of conduct.

---

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

---

<div align="center">

**LocalFlow** · Your data, your agents, your workflows. 100% local. 🔒

</div>