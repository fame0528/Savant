# Contributing to Savant

## Development Process

All development follows the process defined in `dev/development-process.md`:

1. **Audit** → Understand current state (`cargo check`, `cargo test`)
2. **Find Gaps** → Identify bugs, security issues, missing features
3. **Plan** → Add items to `dev/roadmap/roadmap-fix.md`
4. **Perfection** → Read all affected files 1-EOF before editing
5. **Implement** → Fix with production quality (no stubs, no TODOs)
6. **Document** → Update roadmap, changelog, and affected docs
7. **Push** → Commit and push to GitHub

## Quality Standards

- **Zero warnings** — `cargo check` must report zero warnings
- **Zero unwrap()** in non-test code — use proper error handling
- **No stubs** — every function must be fully implemented
- **No placeholders** — no `todo!()`, `unimplemented!()`, or `// TODO`
- **Tests pass** — all tests must pass before pushing
- **Production quality** — every line should be production-ready

## Running Tests

```bash
# All tests (skip slow memory tests)
cargo test --all -- --skip lsm_engine --skip vector_engine

# Specific crate
cargo test -p savant_core
cargo test -p savant_gateway

# With output
cargo test --all -- --nocapture
```

## Code Style

- Use `tracing` macros (`info!`, `warn!`, `error!`) for logging
- Use `?` operator for error propagation
- Use `#[instrument]` for async function tracing
- Write doc comments on all public functions
- Keep functions focused — one function, one purpose

## Project Structure

```
crates/         → Rust source code
dashboard/      → Next.js frontend
config/         → TOML configuration
dev/            → Development process & tracking
docs/           → User-facing documentation
skills/         → Installed skills
workspaces/     → Agent workspaces
```

## Submitting Changes

1. Create a branch from `main`
2. Follow the development process above
3. Push and create a Pull Request
4. CI will run automatically (`cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`)
