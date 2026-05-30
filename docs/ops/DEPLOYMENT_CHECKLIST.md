# Savant Production Deployment Checklist

> **Last Updated:** 2026-05-25 (v0.3.4)

---

## Prerequisites

- [ ] **Rust 1.75+** installed
- [ ] **Node.js 18+** installed (for dashboard)
- [ ] **Docker** installed and running (for sandbox execution)
- [ ] **AI Provider API Key** (OpenRouter, OpenAI, Anthropic, etc.) or Ollama for local models

---

## Environment Setup

- [ ] Copy `.env.example` to `.env` and configure:
  - `OR_MASTER_KEY` — OpenRouter API key (or use `SAVANT_DEV_MODE=1` for development)
- [ ] Review `config/savant.toml` settings:
  - `[ai].provider` — your AI provider
  - `[ai].model` — model name
  - `[server].port` — gateway port (default: 3000)
  - `[server].host` — listen address (default: `127.0.0.1`)
  - `[server].dashboard_api_key` — API key for REST/WS auth (empty = no auth in dev)
- [ ] Verify database directories will be created:
  - `./data/savant/` — sovereign substrate storage
  - `./data/memory/` — agent memory engine (MUST be separate path)

---

## Security Configuration

- [ ] Generate master key pair: `savant_cli --keygen`
- [ ] Set `SAVANT_MASTER_SECRET_KEY` and `SAVANT_MASTER_PUBLIC_KEY` in `.env`
- [ ] Ensure `.env` is in `.gitignore`
- [ ] Verify `config/savant.toml` has no secrets (only settings)
- [ ] **Production:** Set `dashboard_api_key` to a strong random value
- [ ] **Production:** Set `server.host` to `127.0.0.1` (not `0.0.0.0`)
- [ ] Verify immutable config fields are set correctly (require restart to change):
  - `server.dashboard_api_key`
  - `server.host`
  - `server.port`
  - `server.signing_key`
  - `security.enable_blocklist_sync`

---

## Resource Governor (v0.3.4)

- [ ] Review `[resource_governor]` thresholds for your hardware:
  - 8GB RAM: lower `memory_high_pct` to 70.0, `max_agents_high` to 2
  - 16GB RAM: defaults are fine
  - 32GB+ RAM: increase `max_agents_low` to 32
- [ ] Verify governor is enabled: `[resource_governor].enabled = true`

---

## Nix Sandbox (Linux/macOS only)

- Nix requires Unix-like environment (Linux or macOS)
- On Windows: returns `SavantError::Unsupported` with clear message
- For Nix on Windows: run Savant inside WSL2
- Ensure `nix` CLI is in PATH
- Enable flakes: `nix.settings.experimental-features = "flakes"`

---

## Docker Sandbox

- Docker Desktop (Windows/macOS) or Docker Engine (Linux)
- Network access for pulling container images
- Sufficient disk space for container images
- Default sandbox: `alpine:latest`

---

## Ollama (Local Models)

- [ ] Install Ollama: <https://ollama.com/download>
- [ ] Pull embedding model: `ollama pull gemma4:e4b`
- [ ] Verify Ollama is running: `curl http://localhost:11434/api/tags`
- [ ] The system auto-starts Ollama if not running and auto-pulls missing models

---

## Glass House (Obsidian Sync)

- [ ] Set `[obsidian].vault_path` to your Obsidian vault directory
- [ ] Verify vault path exists and is writable
- [ ] Configure `[obsidian].max_files` (default: 15,000)
- [ ] Configure `[obsidian].cold_storage_days` (default: 90)

---

## Launch

```bash
# Smart launcher (Windows)
start.bat

# Manual launch
cargo run --release --bin savant_cli    # Gateway + Swarm
cd dashboard && npm run dev             # Dashboard (separate terminal)
```

---

## Health Check

```bash
curl http://localhost:3000/live    # Should return "OK"
curl http://localhost:3000/ready   # Should return "OK"
curl http://localhost:3000/health  # Should return system status JSON
```

---

## API Authentication Test

```bash
# If dashboard_api_key is set:
curl -H "Authorization: Bearer YOUR_KEY" http://localhost:3000/api/agents
curl -H "X-API-Key: YOUR_KEY" http://localhost:3000/api/settings

# Should return 401 without key:
curl http://localhost:3000/api/agents  # → 401 Unauthorized
```

---

## Verification

- [ ] Dashboard loads at <http://localhost:3000>
- [ ] WebSocket connects successfully
- [ ] Agent discovery shows workspace agents
- [ ] Config auto-reload works (edit savant.toml, verify log message)
- [ ] Threat intel sync runs (MalwareBazaar + URLhaus)
- [ ] Skill installation works
- [ ] Resource governor shows pressure in logs: `[governor] Pressure: LOW`
- [ ] Consciousness daemon running: `[consciousness] State: IDLE`
- [ ] Memory search works: `curl -H "X-API-Key: KEY" "http://localhost:3000/api/memory/search?q=test"`
