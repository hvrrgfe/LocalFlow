# LocalFlow Desktop - Development Build Guide

## Prerequisites

- Rust 1.78+ (MSVC toolchain for Windows)
- Node.js 18+ and npm
- Tauri CLI: `npm install -g @tauri-apps/cli`

## Setup

```bash
cd apps/desktop
npm install
```

## Development

### Start Vite dev server

```bash
cd apps/desktop
npm run dev
```

### Start Tauri dev mode (Vite + Tauri window)

```bash
cd apps/desktop
npm run tauri dev
```

### Rust check (without UI)

```bash
cargo check -p localflow-desktop
```

## Build for Windows EXE

### 1. Set up build environment

The build requires MSVC tools and the Windows SDK. The project uses a custom linker (lld-link.exe).

```powershell
$env:Path = "C:\Users\User\.local\mingw64\mingw64\bin;$env:Path"
$env:LIB = "D:\Steam\LocalFlow\.cargo;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\um\x64;C:\Program Files (x86)\Windows Kits\10\Lib\10.0.26100.0\ucrt\x64"
$env:CARGO_HOME = "$env:USERPROFILE\.cargo"
```

### 2. Build the frontend

```bash
cd apps/desktop
npm run build
```

### 3. Build the Tauri app

```bash
cd apps/desktop
npm run tauri build
```

The built EXE will be at `apps/desktop/src-tauri/target/release/localflow-desktop.exe`.

### 4. Alternative: Build Rust binary separately

```powershell
cd D:\Steam\LocalFlow
cargo build --release -p localflow-desktop
```

The binary will be at `target/release/localflow-desktop.exe`.
Note: The standalone binary won't have the WebView2 frontend embedded.
Use `npm run tauri build` for a complete standalone installer.

## Project Structure

```
apps/desktop/
  src/                    # React + TypeScript frontend
    components/           # Shared UI components (Layout, ErrorBoundary)
    hooks/                # React hooks for Tauri invoke() calls
    pages/                # Page components
    lib/                  # Tauri API wrapper
    types.ts              # TypeScript type definitions
  src-tauri/              # Tauri Rust backend
    src/commands/         # Tauri command handlers
    src/lib.rs            # App state and command registration
    src/main.rs           # Entry point
    tauri.conf.json       # Tauri configuration
    capabilities/         # Tauri 2 capabilities/permissions
```

## Configuration Notes

- Dev server runs on port 1420
- HMR port: 1421 (when using `TAURI_DEV_HOST`)
- Frontend expects Tauri backend at `http://localhost:1420`
- The app uses `localflow-core`, `localflow-storage`, `localflow-secret-vault`,
  `localflow-security`, `localflow-audit`, `localflow-workflow-engine`, and
  `localflow-model-providers` workspace crates
- API keys are stored in-memory vault (not exposed to frontend)
- All sensitive operations are validated by Rust backend