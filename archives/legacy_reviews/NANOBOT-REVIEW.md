# NanoBot Framework Review

> **Repository:** github.com/HKUDS/nanobot
> **Language:** Python 3.11+
> **Position:** Ultra-lightweight OpenClaw alternative from HKU research lab

---

## What Makes It Special

1. **Virtual tool-call pattern** — Structured LLM decisions via forced tool calls instead of free-text parsing. Used for heartbeat (skip/run) and memory consolidation.
2. **Post-run evaluation gate** — LLM evaluates if background task results warrant notification. Prevents noise.
3. **Token-based memory consolidation** — Estimates prompt tokens, consolidates at user-turn boundaries. Not message-count based.
4. **Legal tool-call boundary detection** — `_find_legal_start()` prevents orphan tool results in history windows.
5. **10+ built-in channels** — Telegram, Discord, WhatsApp, Feishu, DingTalk, Slack, Email, QQ, Matrix, Mochat, WeCom
6. **20+ LLM providers** — Auto-detection via keyword/prefix matching. Provider addition = 2 lines.
7. **Progressive skills loading** — Summary in prompt, full content loaded on-demand via `read_file`
8. **Error poisoning prevention** — Error responses NOT persisted to session history
9. **SSRF protection** — Private IP blocking, DNS resolution validation
10. **Skills requirements checking** — `requires.bins`, `requires.env` validation before availability

---

## Architecture

- Python asyncio-based
- Message bus: `asyncio.Queue` decouples channels from agent
- Pydantic v2 config with env var support
- ~6,000 lines of core agent code
- LiteLLM as routing layer for most providers

---

## Key Innovations For Savant

### A. Virtual Tool-Call Pattern (MOST IMPORTANT)
Instead of parsing free-text responses for decisions, force a structured tool call:
```python
# Heartbeat decides via tool call, not text parsing
heartbeat_tool = {
    "function": {
        "name": "heartbeat",
        "parameters": {"action": {"type": "string", "enum": ["skip", "run"]}}
    }
}
# Force tool_choice
response = await provider.chat(messages, tools=[heartbeat_tool], tool_choice={"type": "function", ...})
```

This is more reliable than HEARTBEAT_OK tokens or regex parsing. Savant's heartbeat should use this.

### B. Post-Run Evaluation Gate
After background tasks complete, a separate LLM call decides whether to notify:
```python
evaluate_notification(should_notify: bool, reason: str)
```
Prevents notification fatigue. Falls back to "notify" on failure.

### C. Token-Based Context Management
Replace message-count limits with token estimation. Consolidation boundary at user-turn boundaries preserves valid tool-call chains.

### D. Error Poisoning Prevention
Error responses are NOT persisted to session history. Prevents permanent 400 loops from bad tool calls.

### E. Legal Tool-Call Boundaries
`_find_legal_start()` ensures history window starts at a valid point — no orphan tool results.

---

## What Savant Does Better

| Area | Savant |
|------|--------|
| **Performance** | Rust vs Python (GIL-limited) |
| **Memory** | LSM + vector vs MEMORY.md + HISTORY.md files |
| **Security** | WASM + enclaves vs SSRF protection only |
| **Swarm** | Orchestration vs single agent + subagents |
| **Dashboard** | Web UI vs no UI |
| **Speculative execution** | Not present in NanoBot |
| **Cognitive layer** | Not present in NanoBot |
| **Canvas system** | Not present in NanoBot |
| **Desktop app** | Tauri vs CLI only |

---

## What We're Missing From NanoBot

1. Virtual tool-call pattern for structured decisions (CRITICAL)
2. Post-run evaluation gate for notifications
3. Token-based memory consolidation
4. Legal tool-call boundary detection
5. Error poisoning prevention (don't persist errors)
6. 10+ channels (Feishu, DingTalk, QQ, WeCom, Matrix)
7. 20+ provider auto-detection
8. Progressive skills loading
9. Skills requirements checking
10. SSRF protection

---

## Key Takeaway

NanoBot's research patterns are the most intellectually rigorous. The virtual tool-call pattern should replace Savant's heartbeat text parsing. The token-based consolidation and legal boundary detection are essential for production stability.
