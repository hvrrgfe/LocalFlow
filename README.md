<div align="center">

# LocalFlow 🤖

**本地优先的 AI Agent 和工作流编排工具**

Local-first AI Agent & Workflow orchestration tool

[![Rust](https://img.shields.io/badge/Rust-2024-dea584?logo=rust)](https://www.rust-lang.org)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-ffc131?logo=tauri)](https://v2.tauri.app)
[![React](https://img.shields.io/badge/React-18-61dafb?logo=react)](https://react.dev)
[![Flutter](https://img.shields.io/badge/Flutter-3-02569b?logo=flutter)](https://flutter.dev)
[![SQLite](https://img.shields.io/badge/SQLite-3-003b57?logo=sqlite)](https://www.sqlite.org)
[![License MIT](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/hvrrgfe/LocalFlow)](https://github.com/hvrrgfe/LocalFlow/releases)

[English](#english) · [中文](#中文)

---

</div>

## 中文

### 项目简介

LocalFlow 是一个**完全本地运行**的 AI Agent 和工作流编排工具。与 Coze、Dify 等云端平台不同，LocalFlow 的所有项目配置、工作流定义、知识库元数据、运行记录和密钥都保存在**用户本机**，不依赖任何云端账号。

用户可自行导入 OpenAI 兼容 API、自定义 HTTP API 和 OpenAPI 文档，在完全离线的环境中构建 AI 工作流。

### 核心特性

| 特性 | 说明 |
|------|------|
| 🔒 **本地优先** | 所有数据保存在用户本机，不上传任何源码、文件或日志 |
| 🔑 **密钥安全** | API Key 仅存储在系统密钥链（Windows Credential Manager / macOS Keychain / Linux Secret Service），代码/日志/导出文件不含密钥 |
| 🛡️ **SSRF 防护** | 默认禁止访问 localhost、内网地址、云元数据地址和 file:// 协议 |
| 📝 **审计日志** | 所有敏感操作写入审计表，日志自动脱敏 Authorization/Bearer/api_key |
| 🔄 **DAG 工作流** | 7 种内置节点类型，DAG 执行，拒绝循环，支持取消、超时、重试和失败恢复 |
| 🎯 **零信任架构** | 外部 API 返回内容视为不可信数据，不能修改系统提示词、权限和工具白名单 |
| 📦 **跨平台** | Windows 桌面（Tauri 2 + React）+ Android 移动端（Flutter）共用同一 Rust Core |

### 技术栈

```
apps/desktop/   → Tauri 2 + React + TypeScript + Vite (Windows)
apps/mobile/    → Flutter + Dart + Provider (Android)
crates/core/            → 领域模型、错误类型、状态机
crates/storage/         → SQLite 存储层（rusqlite + 迁移）
crates/secret-vault/    → 密钥存储抽象（InMemory / 系统密钥链）
crates/security/        → SSRF 检测、路径校验、日志脱敏
crates/audit/           → 审计日志服务
crates/workflow-engine/ → DAG 工作流执行器
crates/model-providers/ → OpenAI 兼容 / 自定义 HTTP 模型调用
crates/api-tools/       → OpenAPI 3 解析
```

### 快速开始

#### Windows 桌面端

```powershell
# 1. 构建前端
cd apps\desktop
npm install
npm run build

# 2. 构建后端 (需要 MSVC 工具链)
cd ..\..

# 确保正确的链接器路径
$env:LIB = "D:\Steam\LocalFlow\.cargo;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64"
cargo build --release -p localflow-desktop

# 3. 运行
cargo run -p localflow-desktop
```

> 可从 [Releases 页面](https://github.com/hvrrgfe/LocalFlow/releases) 直接下载已编译的 EXE。

#### 开发模式

```powershell
# 终端 1: Vite 前端开发服务器
cd apps\desktop
npm run dev

# 终端 2: Tauri 桌面窗口
cargo run -p localflow-desktop
```

浏览器打开 `http://localhost:1420` 可查看前端（无 Rust 后端时仅显示 UI 布局）。

### 安全模型

LocalFlow 遵循**默认安全**原则：

1. **API Key 永不外泄**
   - 只能通过 Secret Reference 使用 (`secret://provider/deepseek`)
   - 存储于系统密钥链，不出现在配置文件、日志、工作流文件、导出包或错误信息中
   - 日志自动脱敏 `Authorization`、`Bearer`、`api_key`、`token`、`password`

2. **网络请求严格限制**
   - 默认禁止：localhost、127.0.0.1、0.0.0.0、内网地址、云元数据、file://
   - 支持 `allowed_hosts` 白名单
   - 请求体/响应体大小默认限制 10MB

3. **工作流执行安全**
   - 外部 API 返回内容视为不可信数据
   - 外部内容不能修改系统提示词、权限和工具白名单
   - 默认不提供 Shell/Python/JavaScript 执行节点
   - 单节点失败不会导致主进程崩溃

4. **文件系统安全**
   - 所有文件路径经过规范化和目录穿越检查
   - 不允许读取用户 SSH、云服务凭证、浏览器 Cookie 和系统目录

### 节点状态机

每个工作流节点有明确的状态生命周期：

```
PENDING → RUNNING → SUCCEEDED
                   → FAILED → (重试) → RUNNING
                   → CANCELLED
                   → PAUSED
                   → WAITING_APPROVAL
```

默认超时：HTTP 10s / 模型调用 120s / 单个工作流 10min。默认最多重试 3 次。

### 编译与测试

```powershell
cargo test --workspace    # 运行所有测试 (136+ 个)
cargo fmt --all           # 代码格式化
cargo clippy --all-targets # Lint 检查
```

---

## English

### Overview

**LocalFlow** is a fully local-first AI Agent and workflow orchestration tool. Unlike cloud platforms such as Coze or Dify, LocalFlow stores all project configurations, workflow definitions, knowledge base metadata, run logs, and secrets **on the user's local machine** with zero cloud dependency.

Users can configure OpenAI-compatible APIs, custom HTTP APIs, and import OpenAPI documents to build AI workflows in a completely offline environment.

### Key Features

| Feature | Description |
|---------|-------------|
| 🔒 **Local-First** | All data stored locally, no upload of code, files, prompts, or logs |
| 🔑 **Secret Safety** | API keys stored in system keychain only (Windows Credential Manager / macOS Keychain / Linux Secret Service); never in code, configs, logs, or exports |
| 🛡️ **SSRF Protection** | Blocks localhost, private IPs, cloud metadata endpoints, and file:// by default |
| 📝 **Audit Logging** | All sensitive operations logged; automatic redaction of Authorization, Bearer, api_key, token, password |
| 🔄 **DAG Workflows** | 7 built-in node types, DAG execution with cycle detection; supports cancel, timeout, retry, and failure recovery |
| 🎯 **Zero-Trust** | External API responses treated as untrusted; cannot modify system prompts, permissions, or tool allowlists |
| 📦 **Cross-Platform** | Windows desktop (Tauri 2 + React) + Android (Flutter) sharing a unified Rust Core |

### Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Shell | Tauri 2 (Rust) |
| Desktop UI | React 18, TypeScript, Vite |
| Mobile UI | Flutter 3, Dart, Provider |
| Database | SQLite via rusqlite |
| Async Runtime | Tokio |
| Serialization | Serde JSON |
| Logging | tracing + structured logs |
| Crypto (fallback) | Argon2id + AES-256-GCM |
| System Keychain | Windows Credential Manager, macOS Keychain, Linux Secret Service, Android Keystore |

### Quick Start

#### Windows Desktop

```powershell
# 1. Build frontend
cd apps\desktop
npm install
npm run build

# 2. Build backend (requires MSVC toolchain)
cd ..
cargo build --release -p localflow-desktop

# 3. Run
cargo run -p localflow-desktop
```

> Pre-built EXEs are available on the [Releases page](https://github.com/hvrrgfe/LocalFlow/releases).

### Security Architecture

LocalFlow follows **Secure by Default** principles:

1. **API Key Protection**
   - Accessed exclusively via Secret References (`secret://provider/deepseek`)
   - Stored in system keychain — never written to configs, logs, workflow exports, or error messages
   - Automatic log redaction of `Authorization`, `Bearer`, `api_key`, `token`, `password`

2. **Network Request Hardening**
   - Blocked by default: localhost, 127.0.0.1, 0.0.0.0, private subnets, cloud metadata, file://
   - `allowed_hosts` allowlist support
   - Request/response body size limits (default 10MB)

3. **Workflow Execution Safety**
   - External API responses treated as untrusted data
   - External content cannot modify system prompts, permissions, or tool definitions
   - No built-in Shell, Python, or JavaScript execution nodes
   - Single node failure does not crash the main process

4. **File System Safety**
   - All file paths normalized and checked for directory traversal
   - Cannot access user SSH keys, cloud credentials, browser cookies, or system directories

### Node State Machine

Every workflow node follows a strict state lifecycle:

```
PENDING → RUNNING → SUCCEEDED
                   → FAILED → (retry) → RUNNING
                   → CANCELLED
                   → PAUSED
                   → WAITING_APPROVAL
```

Default timeouts: HTTP 10s / Model call 120s / Single workflow 10min. Max 3 retries for retryable errors only.

### Testing

```powershell
cargo test --workspace    # 136+ tests across all crates
cargo fmt --all           # Code formatting
cargo clippy --all-targets # Linting
```

### Project Structure

```
LocalFlow/
├── apps/
│   ├── desktop/          # Tauri 2 + React Windows app
│   │   ├── src/          # React TypeScript frontend
│   │   └── src-tauri/    # Tauri Rust backend
│   └── mobile/           # Flutter Android app
├── crates/
│   ├── core/             # Domain models, errors, state machine
│   ├── storage/          # SQLite storage layer
│   ├── secret-vault/     # Secret storage abstraction
│   ├── security/         # URL validation, redaction, sanitization
│   ├── audit/            # Audit logging
│   ├── workflow-engine/  # DAG workflow executor
│   ├── model-providers/  # OpenAI/custom HTTP providers
│   └── api-tools/        # OpenAPI 3 parsing
├── tests/                # Integration tests
├── docs/                 # Documentation
└── scripts/              # Utility scripts
```

### License

MIT License — see [LICENSE](LICENSE) for details.

---

<div align="center">

**LocalFlow** — Your data, your agents, your workflows. All local.

</div>