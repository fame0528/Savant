# Contributing to Savant

Thank you for your interest in contributing to Savant! This guide will help you get started.

---

## Prerequisites

- **Rust** 1.75+ (stable) — Install via [rustup.rs](https://rustup.rs/)
- **Node.js** 18+ — Install via [nvm](https://github.com/nvm-sh/nvm) or [nodejs.org](https://nodejs.org/)
- **Docker** — For sandbox execution ([docker.com](https://docker.com/))
- **Git** — For version control

---

## Getting Started

```bash
# Clone the repository
git clone https://github.com/your-org/savant.git
cd savant

# Build the project
cargo build

# Run tests
cargo test --workspace --lib

# Start the gateway and dashboard
cargo run --release --bin savant_cli
cd dashboard && npm install && npm run dev
```

---

## Code Style

All code must follow the conventions documented in [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md). Key rules:

- **No `unwrap()`/`expect()` in production code** — Use `?` operator or explicit error handling
- **No `todo!()`/`unimplemented!()`** — Every code path must be complete
- **No stubs or pseudo-code** — Every line must be production-ready
- **Clippy clean** — Zero warnings with `cargo clippy --workspace --all-targets -- -D warnings`
- **All tests pass** — `cargo test --workspace --lib` must pass with 0 failures

---

## Running Checks

Before submitting a PR, run all checks:

```bash
# Rust checks
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib
cargo fmt --check

# Dashboard checks (if modifying frontend)
cd dashboard && npx tsc --noEmit && npm run build
```

---

## Pull Request Process

1. **Fork** the repository
2. **Create a branch** from `main`: `git checkout -b feature/your-feature`
3. **Write tests** for your changes
4. **Run all checks** (see above)
5. **Submit a PR** with a clear description of what changed and why
6. **Respond to review** feedback promptly

---

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add cost-aware model routing
fix: resolve circuit breaker race condition
docs: update memory architecture reference
test: add provider chain integration tests
refactor: remove RetryProvider dead code
```

---

## Issue Reporting

When reporting a bug, include:

- **Steps to reproduce**
- **Expected behavior**
- **Actual behavior**
- **Environment** (OS, Rust version, Node version)
- **Logs** (if applicable)

---

## Security

For security vulnerabilities, please see [`docs/security/SECURITY.md`](docs/security/SECURITY.md) for responsible disclosure policy. Do **not** open a public issue for security vulnerabilities.

---

## Architecture

- [`docs/architecture/README.md`](docs/architecture/README.md) — System design overview
- [`docs/memory.md`](docs/memory.md) — Memory architecture
- [`docs/swarm.md`](docs/swarm.md) — Hivemind architecture
- [`docs/collective_intelligence.md`](docs/collective_intelligence.md) — Multi-agent consensus
- [`docs/evolution/evolution-system.md`](docs/evolution/evolution-system.md) — Evolution system guide

---

## Questions?

Open a [GitHub Discussion](https://github.com/your-org/savant/discussions) for questions about the codebase or architecture.
