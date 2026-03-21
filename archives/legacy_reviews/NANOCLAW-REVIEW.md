# NanoClaw Framework Review

> **Repository:** github.com/qwibitai/nanoclaw
> **Language:** TypeScript (Node.js)
> **Position:** Ultra-minimal container-isolated agent

---

## What Makes It Special

1. **Credential proxy** — Real API keys NEVER enter containers. HTTP proxy injects auth headers transparently. This eliminates an entire class of security vulnerability.
2. **Container isolation per group** — Each chat group gets its own Docker container with explicit volume mounts. Groups cannot see each other.
3. **Filesystem IPC** — Host↔container communication via JSON files. Simple, debuggable, no networking.
4. **Skills over features** — Core stays minimal. New capabilities contributed as mergeable skill branches.
5. **Hierarchical CLAUDE.md memory** — Global/group/file levels. No vector DB needed.
6. **Conversation archive on compaction** — Pre-compact hook saves full transcript as Markdown.
7. **Self-registering channels** — Factory pattern with dynamic loading.
8. **Main channel privilege model** — Self-chat has elevated admin control.
9. **External tamper-proof security config** — Mount allowlist stored outside project root.
10. **Idle timeout with sentinel shutdown** — `_close` file signals container wind-down.

---

## Architecture

- Single Node.js process, no microservices
- Channels → SQLite → Polling loop → Docker Container (Claude Agent SDK) → Response
- ~35k tokens total codebase (extremely lean)
- File-based IPC between host and containers
- Claude Agent SDK for agent inference (not owned)

---

## Key Innovations For Savant

### A. Credential Proxy Pattern
The single most adoptable security pattern. Real secrets never enter the agent environment. An HTTP proxy on the host injects credentials transparently. Savant already has a master key system — this would complement it.

### B. External Security Config
Security rules stored at `~/.config/nanoclaw/` — outside project root, never mounted into containers. Agents cannot modify their own security rules.

### C. Per-Group Isolation
Each group gets isolated: memory, sessions, filesystem, IPC namespace, container. This is the container-native approach to multi-tenancy.

### D. Self-Registering Plugin Factory
```typescript
registerChannel(name, factory)
```
Plugins register at module load, return null if unconfigured. Barrel imports trigger registration.

### E. Read-Only Project Root
Main group's project root mounted read-only. Agent cannot modify host application code.

---

## What Savant Does Better

| Area | Savant |
|------|--------|
| **Performance** | Rust vs Node.js |
| **Memory** | LSM + vector vs CLAUDE.md files |
| **Dashboard** | Web UI vs no UI |
| **Multi-model** | 15 providers vs Anthropic-only |
| **Tool system** | Rust tools vs MCP-only |
| **Desktop app** | Tauri vs CLI only |
| **Swarm orchestration** | Built-in vs container teams |

---

## What We're Missing From NanoClaw

1. Credential proxy (CRITICAL security improvement)
2. External tamper-proof security config
3. Per-group container isolation
4. Read-only project root mounting
5. File-based IPC for container communication
6. Conversation archiving on compaction
7. Idle timeout with sentinel shutdown
8. Main channel privilege model
9. Skills-over-features contribution model
10. .env shadowing in containers

---

## Key Takeaway

NanoClaw's security model is its crown jewel. The credential proxy and external security config are directly adoptable. The container isolation per group is the gold standard for multi-tenancy. Savant should adopt the credential proxy immediately.
