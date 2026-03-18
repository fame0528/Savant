# Savant Production Deployment Checklist

## Prerequisites

- [ ] **Rust 1.75+** installed
- [ ] **Node.js 18+** installed (for dashboard)
- [ ] **Docker** installed and running (for sandbox execution)
- [ ] **AI Provider API Key** (OpenRouter, OpenAI, Anthropic, etc.)

## Environment Setup

- [ ] Copy `.env.example` to `.env` and configure:
  - `OR_MASTER_KEY` — OpenRouter API key
  - `SAVANT_DEV_MODE=1` — for development (auto-generates keys)
- [ ] Review `config/savant.toml` settings:
  - `ai.provider` — your AI provider (openrouter, openai, anthropic, etc.)
  - `ai.model` — model name (e.g., `openrouter/healer-alpha`)
  - `server.port` — gateway port (default: 3000)
- [ ] Verify database directories will be created:
  - `./data/savant/` — sovereign substrate storage
  - `./data/memory/` — agent memory engine (MUST be separate path)

## Security Configuration

- [ ] Generate master key pair: `savant_cli --keygen`
- [ ] Set `SAVANT_MASTER_SECRET_KEY` and `SAVANT_MASTER_PUBLIC_KEY` in `.env`
- [ ] Ensure `.env` is in `.gitignore`
- [ ] Verify `config/savant.toml` has no secrets (only settings)

## Nix Sandbox (Linux/macOS only)

- Nix requires Unix-like environment (Linux or macOS)
- On Windows: returns `SavantError::Unsupported` with clear message
- For Nix on Windows: run Savant inside WSL2
- Ensure `nix` CLI is in PATH
- Enable flakes: `nix.settings.experimental-features = "flakes"`

## Docker Sandbox

- Docker Desktop (Windows/macOS) or Docker Engine (Linux)
- Network access for pulling container images
- Sufficient disk space for container images
- Default sandbox: `alpine:latest`

## Launch

```bash
# Smart launcher (Windows)
start.bat

# Manual launch
cargo run --release --bin savant_cli    # Gateway + Swarm
cd dashboard && npm run dev             # Dashboard (separate terminal)
```

## Health Check

```bash
curl http://localhost:3000/live    # Should return "OK"
curl http://localhost:3000/ready   # Should return "OK"
```

## Verification

- [ ] Dashboard loads at http://localhost:3000
- [ ] WebSocket connects successfully
- [ ] Agent discovery shows workspace agents
- [ ] Config auto-reload works (edit savant.toml, verify log message)
- [ ] Threat intel sync runs (MalwareBazaar + URLhaus)
- [ ] Skill installation works
