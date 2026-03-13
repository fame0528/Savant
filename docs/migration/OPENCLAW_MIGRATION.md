# OpenClaw to Savant Migration Guide

Welcome to Savant v1.5.0. This guide outlines the steps to migrate your legacy 103-agent swarm from OpenClaw/ZeroClaw to the Rust-native Savant framework.

## 1. Direct Framework Comparison

| Feature | OpenClaw (Legacy) | Savant (v1.5.0) |
| :--- | :--- | :--- |
| **Foundation** | Python / JS | **Rust Native** |
| **Security** | Ad-hoc / Key-shuffling | **CCT (Crypto-Cap Tokens)** |
| **IPC** | JSON over HTTP/Files | **Zero-Copy Memory (O(1))** |
| **Logic** | Scripted ReAct | **Speculative ReAct (ECHO)** |
| **Scaling** | Soft-cap at 12 agents | **Stable 500+ agent swarm** |
| **Memory** | Unstructured JSON | **Verified Hybrid LSM (VHSS)** |

## 2. Migration Steps

### Step 1: Identity Conversion
Savant's `IDENTITY.md` and `SOUL.md` files follow a stricter format for AAA quality.
- Legacy `persona.txt` → Rename to `SOUL.md`
- Ensure `SOUL.md` is between 150-200 lines (AAA requirement).
- Use the adapter in `savant_core::migration` for automated parsing.

### Step 2: Config Mapping
Your `agent.json` must be mapped to the new `savant.json` structure.

**Legacy Shape:**
```json
{
  "id": "agent-001",
  "provider": "openrouter",
  "skills": ["web_search"]
}
```

**Savant Shape:**
```json
{
  "agent_id": "agent-001",
  "model_provider": "OpenRouter",
  "allowed_skills": ["web_search"],
  "heartbeat_interval": 600
}
```

### Step 3: Injection Points
If you have legacy Python tools, wrap them using the `LegacyNative` adapter in Savant's `AgentLoop`. This allows them to run within the new IPC substrate while maintaining backward compatibility.

## 3. Automated Bridge
Use the following command to auto-convert a legacy workspace:
```bash
savant --migrate --from ./legacy_project --to ./savant_workspace
```

## 4. Support
Refer to [collective_intelligence.md](../collective_intelligence.md) for details on how your agents will participate in the new consensus model.
