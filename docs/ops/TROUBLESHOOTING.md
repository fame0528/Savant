# Troubleshooting Guide

> **Last Updated:** 2026-05-25 (v0.3.4)

---

## Common Issues

### FjallError: Locked

**Symptom:** `FjallError: Locked` error during startup
**Cause:** Two Fjall database instances trying to open the same path
**Fix:** Ensure `db_path` and `memory_db_path` are different directories in `config/savant.toml`:
```toml
[system]
db_path = "./data/savant"         # Substrate storage
memory_db_path = "./data/memory"  # Agent memory (MUST be different)
```

### Gateway Won't Start — Port in Use

**Symptom:** `cargo run` fails with port already in use
**Cause:** Previous instance still running or port 3000 occupied
**Fix:**
```bash
# Find and kill process on port 3000
netstat -ano | findstr :3000       # Windows
lsof -i :3000                      # Linux/macOS
```

### 401 Unauthorized on API Requests

**Symptom:** All REST API requests return 401 Unauthorized
**Cause:** `dashboard_api_key` is set but request doesn't include the key
**Fix:**

- Include `Authorization: Bearer <key>` or `X-API-Key: <key>` header
- Or set `dashboard_api_key = ""` in `savant.toml` for development mode (no auth)
- Restart gateway after changing `dashboard_api_key`

### 403 Forbidden on Config Changes

**Symptom:** ConfigSet returns "Field is immutable at runtime"
**Cause:** Attempting to change a security-critical field via API/WebSocket
**Fix:** Edit `config/savant.toml` directly and restart. Immutable fields:

- `server.dashboard_api_key`
- `server.host`
- `server.port`
- `server.signing_key`
- `security.enable_blocklist_sync`

### Dashboard WebSocket Connection Failed

**Symptom:** Dashboard shows "Disconnected"
**Cause:** Gateway not running or wrong WebSocket URL
**Fix:**

- Verify gateway is running: `curl http://localhost:3000/live` should return "OK"
- Check WebSocket URL is `ws://127.0.0.1:3000/ws` (default in dashboard)
- Set `NEXT_PUBLIC_WS_URL` env var if using custom host/port

### Agent Discovery Failed

**Symptom:** No agents discovered during startup
**Cause:** Workspaces directory doesn't exist or is empty
**Fix:** Create workspace directories:
```bash
mkdir -p workspaces/agents
mkdir -p workspaces/substrate
```

### Ollama Not Starting

**Symptom:** `Ollama auto-start failed` error
**Cause:** Ollama not installed or not in PATH
**Fix:**

- Install Ollama: <https://ollama.com/download>
- Verify: `ollama --version`
- The system falls back gracefully with a clear error message
- Set `SAVANT_DISABLE_EMBEDDINGS=1` to skip embedding features entirely

### Ollama Model Not Found

**Symptom:** `Model not found` error for embedding model
**Cause:** Embedding model not pulled
**Fix:**
```bash
ollama pull gemma4:e4b
```
The system auto-pulls missing models on startup if Ollama is running.

### Resource Governor Throttling Agents

**Symptom:** Agents deferred with `[governor] Deferring '...' — HIGH pressure`
**Cause:** System under CPU or memory pressure
**Fix:**

- Check system resources: `htop` or Task Manager
- Increase thresholds in `[resource_governor]`:
  ```toml
  memory_high_pct = 90.0
  cpu_high_pct = 90.0
  max_agents_high = 8
  ```
- Or disable governor: `[resource_governor].enabled = false`

### Consciousness Budget Exhausted

**Symptom:** `[consciousness] Budget exhausted — skipping thought`
**Cause:** Token budget exceeded for current hour/day
**Fix:**

- Increase budget in code (future: configurable in savant.toml)
- Wait for hourly reset
- Check quiet hours: 10PM–7AM UTC blocks consciousness operations

### Circuit Breaker Open

**Symptom:** `Circuit breaker is OPEN — provider temporarily unavailable`
**Cause:** Provider failed 5+ consecutive times
**Fix:**

- Wait 60 seconds for automatic recovery (HalfOpen state)
- Check provider status: is the API key valid? Is the service up?
- Check logs for the underlying error

### LEARNINGS.md Rotation

**Symptom:** LEARNINGS.md entries disappearing
**Cause:** File exceeded 100KB and was rotated to `LEARNINGS-ARCHIVE-{timestamp}.md`
**Fix:** This is expected behavior. Check `LEARNINGS-ARCHIVE-*.md` files for old entries. No data is lost.

### Filtered Learning Entries

**Symptom:** Expected learning entries not appearing in LEARNINGS.md
**Cause:** Entry was filtered by quality gate
**Fix:** Check `FILTERED.jsonl` for rejected entries with reasons:

- `not_grounded` — entry lacks environmental grounding
- `duplicate` — content-hash dedup detected duplicate
- Entry exceeded 2000 char limit

### Docker Sandbox Not Working

**Symptom:** `Docker connection failed` error
**Cause:** Docker Desktop not running or not installed
**Fix:**

- Start Docker Desktop
- Verify: `docker ps` should work without errors
- Check: `docker --version` should show version 20+

### Skill Installation Failed

**Symptom:** `ClawHub install failed` error
**Cause:** Network issue or invalid skill name
**Fix:**

- Check internet connection
- Verify skill exists: `curl https://clawhub.com/api/skills/<name>`
- Check `skills/` directory permissions

### API Key Not Found

**Symptom:** `No OpenRouter API key found` warning
**Cause:** Missing or invalid `OR_MASTER_KEY` in `.env`
**Fix:**

- Set `OR_MASTER_KEY=sk-or-v1-...` in `.env`
- Or set `SAVANT_DEV_MODE=1` for development mode
- Restart the gateway after changing `.env`

### Slow Performance

**Symptom:** Gateway responses take >1 second
**Possible causes:**

- Model is too large for available RAM
- Network latency to AI provider
- Too many concurrent agents (governor throttling)
- Debug build instead of release

**Fix:**

- Check `cargo run --release` (debug builds are slow)
- Monitor with `RUST_LOG=info` to see timing information
- Reduce `max_tokens` in `config/savant.toml`
- Check resource governor pressure: look for `[governor]` log lines
- Use cost-aware routing: simple tasks use cheap models automatically

### Glass House Sync Not Working

**Symptom:** Obsidian vault not updating
**Cause:** `vault_path` not set or directory doesn't exist
**Fix:**

- Set `[obsidian].vault_path` in `savant.toml`
- Ensure directory exists and is writable
- Check logs for `[obsidian]` messages
- Sync runs every 5 minutes (configurable via `sync_interval_secs`)

### Config Not Reloading

**Symptom:** Changes to `savant.toml` not taking effect
**Cause:** File watcher not detecting changes (rare)
**Fix:**

- Restart the gateway
- Check logs for `config: Loading from` messages
- Ensure you're editing the correct `savant.toml` (check path in logs)

---

## Getting Help

- Documentation: `docs/` directory
- Architecture: `docs/architecture/README.md`
- Config reference: `docs/config/CONFIG.md`
- API reference: `docs/api/README.md`
- Evolution guide: `docs/evolution/evolution-system.md`
- Development process: `dev/DEVELOPMENT-WORKFLOW.md`
- Contributing: `CONTRIBUTING.md`
