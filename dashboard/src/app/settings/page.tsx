"use client";

import { useState, useEffect, useCallback } from "react";
import { useDashboard } from "@/context/DashboardContext";

interface Settings {
  chat_model: string;
  embedding_model: string;
  vision_model: string;
  ollama_url: string;
  gateway_port: number;
  agents_path: string;
  db_path: string;
}

const getGatewayUrl = () => {
  if (typeof window !== "undefined") {
    const host = window.location.hostname || "127.0.0.1";
    const port = process.env.NEXT_PUBLIC_GATEWAY_PORT || "8080";
    return `http://${host}:${port}`;
  }
  return "http://127.0.0.1:8080";
};

export default function SettingsPage() {
  const ctx = useDashboard();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [ollamaStatus, setOllamaStatus] = useState<"checking" | "connected" | "disconnected">("checking");
  const [loadError, setLoadError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const gatewayUrl = getGatewayUrl();
    fetch(`${gatewayUrl}/api/settings`)
      .then(r => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json();
      })
      .then(data => {
        setSettings(data);
        setIsLoading(false);
        if (data.ollama_url) checkOllama(data.ollama_url);
      })
      .catch((e) => {
        setLoadError(`Gateway unreachable: ${e.message}`);
        setIsLoading(false);
      });
  }, []);

  const checkOllama = useCallback(async (url: string) => {
    try {
      const resp = await fetch(`${url}/api/tags`);
      setOllamaStatus(resp.ok ? "connected" : "disconnected");
    } catch {
      setOllamaStatus("disconnected");
    }
  }, []);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/settings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          chat_model: settings.chat_model,
          vision_model: settings.vision_model,
          ollama_url: settings.ollama_url,
        }),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setLoadError(`Failed to save: ${e instanceof Error ? e.message : 'Unknown error'}`);
    }
    setSaving(false);
  };

  if (isLoading) return <div style={{ padding: "40px", color: "var(--accent)" }}>Connecting to gateway...</div>;

  return (
    <div style={{ padding: "40px", width: "100%", maxWidth: "1200px", margin: "0 auto" }}>
      <h1 style={{ color: "var(--accent)", fontSize: "24px", marginBottom: "32px", letterSpacing: "2px" }}>SETTINGS</h1>

      {loadError && (
        <div style={{ background: "rgba(255, 170, 0, 0.1)", border: "1px solid rgba(255, 170, 0, 0.3)", padding: "12px 16px", borderRadius: "8px", marginBottom: "24px", fontSize: "12px", color: "#ffaa00" }}>
          {loadError} — Showing cached defaults. Changes will be saved when gateway is available.
        </div>
      )}

      {/* Models Section */}
      <section style={{ marginBottom: "40px" }}>
        <h2 style={{ color: "#fff", fontSize: "14px", letterSpacing: "2px", marginBottom: "16px", opacity: 0.6 }}>MODELS</h2>
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ fontSize: "11px", color: "var(--accent)", letterSpacing: "1px" }}>CHAT MODEL</span>
            <input value={settings?.chat_model || ""} onChange={e => setSettings({ ...settings, chat_model: e.target.value } as Settings)}
              style={{ background: "rgba(255,255,255,0.05)", border: "1px solid var(--border)", padding: "10px 14px", color: "#fff", borderRadius: "8px", fontSize: "14px" }} />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ fontSize: "11px", color: "var(--accent)", letterSpacing: "1px" }}>EMBEDDING MODEL</span>
            <input value={settings?.embedding_model || ""} readOnly
              style={{ background: "rgba(255,255,255,0.02)", border: "1px solid var(--border)", padding: "10px 14px", color: "#888", borderRadius: "8px", fontSize: "14px" }} />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ fontSize: "11px", color: "var(--accent)", letterSpacing: "1px" }}>VISION MODEL</span>
            <input value={settings?.vision_model || ""} onChange={e => setSettings({ ...settings, vision_model: e.target.value } as Settings)}
              style={{ background: "rgba(255,255,255,0.05)", border: "1px solid var(--border)", padding: "10px 14px", color: "#fff", borderRadius: "8px", fontSize: "14px" }} />
          </label>
        </div>
      </section>

      {/* Ollama Section */}
      <section style={{ marginBottom: "40px" }}>
        <h2 style={{ color: "#fff", fontSize: "14px", letterSpacing: "2px", marginBottom: "16px", opacity: 0.6 }}>OLLAMA</h2>
        <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          <label style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
            <span style={{ fontSize: "11px", color: "var(--accent)", letterSpacing: "1px" }}>SERVER URL</span>
            <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
              <input value={settings?.ollama_url || ""} onChange={e => setSettings({ ...settings, ollama_url: e.target.value } as Settings)}
                style={{ flex: 1, background: "rgba(255,255,255,0.05)", border: "1px solid var(--border)", padding: "10px 14px", color: "#fff", borderRadius: "8px", fontSize: "14px" }} />
              <button onClick={() => settings?.ollama_url && checkOllama(settings.ollama_url)} style={{
                background: ollamaStatus === "connected" ? "rgba(0,255,0,0.1)" : "rgba(255,0,0,0.1)",
                border: `1px solid ${ollamaStatus === "connected" ? "rgba(0,255,0,0.3)" : "rgba(255,0,0,0.3)"}`,
                padding: "10px 16px", borderRadius: "8px", color: ollamaStatus === "connected" ? "#0f0" : "#f00",
                fontSize: "10px", fontWeight: 900, letterSpacing: "1px", cursor: "pointer"
              }}>
                {ollamaStatus === "checking" ? "..." : ollamaStatus === "connected" ? "CONNECTED" : "OFFLINE"}
              </button>
            </div>
          </label>
        </div>
      </section>

      {/* System Section */}
      <section style={{ marginBottom: "40px" }}>
        <h2 style={{ color: "#fff", fontSize: "14px", letterSpacing: "2px", marginBottom: "16px", opacity: 0.6 }}>SYSTEM</h2>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
          <div style={{ background: "rgba(255,255,255,0.03)", padding: "12px", borderRadius: "8px", border: "1px solid var(--border)" }}>
            <div style={{ fontSize: "10px", color: "var(--accent)", letterSpacing: "1px", marginBottom: "4px" }}>GATEWAY PORT</div>
            <div style={{ fontSize: "18px", fontWeight: 900 }}>{settings?.gateway_port || 8080}</div>
          </div>
          <div style={{ background: "rgba(255,255,255,0.03)", padding: "12px", borderRadius: "8px", border: "1px solid var(--border)" }}>
            <div style={{ fontSize: "10px", color: "var(--accent)", letterSpacing: "1px", marginBottom: "4px" }}>AGENTS PATH</div>
            <div style={{ fontSize: "12px", fontFamily: "monospace", opacity: 0.7 }}>{settings?.agents_path || "—"}</div>
          </div>
        </div>
      </section>

      {/* Save */}
      <button onClick={handleSave} disabled={saving} style={{
        background: saved ? "rgba(0,255,0,0.2)" : "var(--accent)",
        color: saved ? "#0f0" : "#000",
        border: "none", padding: "12px 32px", borderRadius: "8px",
        fontSize: "12px", fontWeight: 900, letterSpacing: "2px", cursor: "pointer",
        transition: "all 0.2s"
      }}>
        {saving ? "SAVING..." : saved ? "✓ SAVED" : "SAVE SETTINGS"}
      </button>
    </div>
  );
}
