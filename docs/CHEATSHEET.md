# Savant Dev Cheatsheet

## Commands You Need

| What | Command |
|------|---------|
| **Start dev** | `npm run dev` |
| **Build for release** | `npm run build` |
| **Run tests** | `npm run test` |
| **Clean build artifacts** | `npm run clean` |

That's it. `npm run dev` is all you need day-to-day.

## What `npm run dev` Does

1. Starts the Next.js dashboard dev server (port 3000, hot reload)
2. Compiles the Rust backend (debug mode, cached)
3. Opens the Savant desktop window
4. Dashboard connects to gateway on port 8080

Edit a `.tsx` file -> instant reload in the window.
Edit a `.rs` file -> recompiles only what changed, then reloads.

## Debug Console

Click the **SWARM_NOMINAL** status button (top right) to open the debug console. Shows all Rust tracing logs with:
- Dayjs `llll` time formatting
- Color-coded log levels (ERROR=red, WARN=orange, INFO=green, DEBUG=blue)
- COPY button to export all logs
- EXPAND for full-window view
- Select text to pause auto-scroll

## Project Layout

```
Savant/
  crates/desktop/src-tauri/   <- Tauri app (Rust backend + window)
  dashboard/                  <- Next.js frontend (React/TS)
  config/savant.toml          <- All non-secret settings (ports, models, paths)
  .env                        <- Secrets (OR_MASTER_KEY)
  workspaces/workspace-savant/ <- Savant's workspace (SOUL.md, agent.json, LEARNINGS.md)
  dev/                        <- Development tracking (plans, changelog)
  docs/                       <- Documentation (architecture, research)
```

## Port Map

| Service | Port |
|---------|------|
| Gateway (Rust) | 8080 |
| Dashboard dev server | 3000 |

## Key Files

| File | Purpose |
|------|---------|
| `config/savant.toml` | All non-secret settings |
| `.env` | OR_MASTER_KEY (never committed) |
| `workspaces/workspace-savant/SOUL.md` | Savant's identity/personality |
| `workspaces/workspace-savant/agent.json` | Agent config (model, provider, etc) |
| `workspaces/workspace-savant/LEARNINGS.md` | Savant's diary (emergent reflections) |
| `dev/plans/` | Session plans |
| `dev/CHANGELOG-INTERNAL.md` | Detailed changelog |

## If Something Breaks

1. Kill any running processes
2. `npm run clean`
3. `npm run dev`
