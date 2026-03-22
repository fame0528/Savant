# Sub-Plan: Settings, FAQ & Vision Integration

**Parent:** SESSION-2026-03-20.md Phase 5
**Status:** IN PROGRESS
**Started:** 2026-03-20T17:33:42-04:00

---

## Build Order

Since all 3 are interconnected (vision settings live on settings page), build them together:

### Step 1: Vision Service (Backend)
- [ ] Create `ollama_vision.rs` in `crates/core/src/utils/`
- [ ] Create `VisionProvider` trait in `crates/core/src/traits.rs`
- [ ] Register module in `utils/mod.rs`

### Step 2: Settings API (Backend)
- [ ] Create `/api/settings` GET handler in gateway
- [ ] Create `/api/settings` POST handler in gateway
- [ ] Read/write savant.toml + agent.json

### Step 3: Settings Page (Frontend)
- [ ] Create `dashboard/src/app/settings/page.tsx`
- [ ] Model config (chat, embedding, vision)
- [ ] Ollama config (URL, status)
- [ ] System config (ports, heartbeat)
- [ ] Save functionality

### Step 4: FAQ Page (Frontend)
- [ ] Create `dashboard/src/app/faq/page.tsx`
- [ ] Getting started guide
- [ ] Ollama setup guide
- [ ] OpenRouter setup guide
- [ ] Troubleshooting

### Step 5: Wire Everything
- [ ] Navigation links in sidebar
- [ ] Test settings save/load
- [ ] Test vision model description

---

## Design Decisions

- Settings page reads current config on load
- Save writes to files directly (no restart needed for most settings)
- Vision model is optional - falls back gracefully if Ollama unavailable
- FAQ is static content (no API needed)
- Settings accessible from sidebar navigation

---

*Updated as work progresses.*
