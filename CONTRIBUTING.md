# Contributing to LocalFlow

We love your input! We want to make contributing to LocalFlow as easy and transparent as possible.

## Getting Started

1. Fork the repository
2. Install prerequisites (Rust, Node.js, WebView2)
3. Run `cargo build --workspace` to build all crates
4. Run `cargo test --workspace` to run tests

## Development Workflow

### Rust Core

```powershell
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace --lib

# Run linting
cargo clippy --workspace
```

### Desktop App

```powershell
cd apps/desktop
npm install
npm run dev   # Development with hot reload
npx tauri build  # Production build
```

## Code Style

- Rust: Follow `rustfmt` conventions. Run `cargo fmt` before committing.
- TypeScript: Follow the existing patterns in the codebase.
- All public functions must have doc comments.
- No `unwrap()` or `expect()` on user input or external API responses.
- Add tests for all new functionality.

## Pull Request Process

1. Update the README with details of changes if needed
2. Update the CHANGELOG (if it exists)
3. Ensure all tests pass
4. Get review from at least one maintainer

## Code of Conduct

Please note that this project is released with a [Code of Conduct](CODE_OF_CONDUCT.md). By participating you agree to abide by its terms.
