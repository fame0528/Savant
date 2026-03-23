"use client";

import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

export default function ChangelogPage() {
  const [content, setContent] = useState<string>("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Changelog content is embedded since we can't read files at runtime in static HTML
    setContent(getChangelogContent());
    setLoading(false);
  }, []);

  if (loading) {
    return (
      <div style={{ padding: "2rem", color: "#888" }}>Loading changelog...</div>
    );
  }

  return (
    <div style={{ padding: "2rem", maxWidth: "900px", margin: "0 auto" }}>
      <h1
        style={{
          fontSize: "1.5rem",
          fontWeight: "bold",
          marginBottom: "1.5rem",
          color: "var(--accent)",
        }}
      >
        Changelog
      </h1>
      <div
        style={{
          background: "var(--glass-bg)",
          padding: "1.5rem",
          borderRadius: "8px",
          border: "1px solid var(--glass-border)",
          fontSize: "0.85rem",
          lineHeight: "1.7",
        }}
      >
        <ReactMarkdown remarkPlugins={[remarkGfm]}>{content}</ReactMarkdown>
      </div>
    </div>
  );
}

function getChangelogContent(): string {
  return `## v1.6.0 (2026-03-22)

### New Features
- **Tauri 2.x Upgrade** — Migrated from Tauri 1.7 to 2.x with modern API
- **Auto-Updater** — Automatic update checks on startup via GitHub Releases
- **Splash Screen** — Loading screen with status messages during initialization
- **Version Display** — Version number shown in sidebar
- **Changelog Page** — In-app release history
- **25 Channels** — Slack, Email, Signal, IRC, Matrix, Feishu, DingTalk, WeCom, LINE, Google Chat, Teams, Mattermost, Webhook, WhatsApp Business, Bluesky, Reddit, Nostr, Twitch, Notion, Voice, X + existing

### Agent Framework
- **Session/Thread/Turn Model** — Persistent session state in CortexaDB
- **Provider Chain** — Error classification, cooldown, circuit breaker, response cache
- **Context Compaction** — 3-strategy compaction prevents context overflow
- **Approval Gating** — Destructive tools require human consent
- **Tool Coercion** — Automatic argument correction against JSON Schema
- **Self-Repair** — Tool health tracking, stuck detection, recovery
- **Hook System** — 6 lifecycle events, 3 execution strategies
- **Mount Security** — 16 blocked patterns for Docker containers

### Infrastructure
- **MCP Integration** — Tool discovery at startup, schema passthrough
- **Smithery CLI** — Install MCP servers from marketplace via dashboard
- **OMEGA-VIII Audit** — 111/111 CRITICAL violations fixed, zero unwrap in production

---

## v1.5.0 (2026-03-21)

### New Features
- **Dashboard Overhaul** — 24 issues audited and fixed
- **Real-Time WebSocket** — Live streaming with reconnection
- **Cognitive Insights** — Collapsible thoughts sidebar
- **Virtual Scrolling** — Message windowing for performance
- **Dark/Light Mode** — Full theme support

### Infrastructure
- **Tool System Revamp** — 9 phases including schemas, coercion, tags
- **Workspace Scaffolding** — Auto-creates skills directory on boot
- **Secure File Ops** — Path traversal prevention, workspace boundaries
`;
}
