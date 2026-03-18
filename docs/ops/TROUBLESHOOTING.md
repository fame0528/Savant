# Troubleshooting Guide

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

### Gateway Won't Start

**Symptom:** `cargo run` fails with port already in use  
**Cause:** Previous instance still running or port 3000 occupied  
**Fix:**
```bash
# Find and kill process on port 3000
netstat -ano | findstr :3000       # Windows
lsof -i :3000                      # Linux/macOS
```

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
- Too many concurrent agents

**Fix:**
- Check `cargo run --release` (debug builds are slow)
- Monitor with `RUST_LOG=info` to see timing information
- Reduce `max_tokens` in `config/savant.toml`

## Getting Help

- Documentation: `docs/` directory
- Development process: `dev/development-process.md`
- Contributing: `CONTRIBUTING.md`
