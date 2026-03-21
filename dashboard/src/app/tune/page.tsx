"use client";

import { useState, useEffect, useCallback } from "react";
import { isTauri } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import { useRouter } from "next/navigation";

const getGatewayUrl = () => {
  if (isTauri()) return process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8080";
  if (typeof window !== "undefined") {
    const envUrl = process.env.NEXT_PUBLIC_GATEWAY_URL;
    if (envUrl) return envUrl;
    const host = window.location.hostname || "127.0.0.1";
    const port = process.env.NEXT_PUBLIC_GATEWAY_PORT || "8080";
    return `http://${host}:${port}`;
  }
  return "http://localhost:8080";
};

interface TuneSettings {
  temperature: number;
  top_p: number;
  frequency_penalty: number;
  presence_penalty: number;
}

const PRESETS: Record<string, TuneSettings> = {
  "deep_observer": {
    temperature: 0.45,
    top_p: 0.85,
    frequency_penalty: 0.35,
    presence_penalty: 0.25,
  },
  "creative_spark": {
    temperature: 0.95,
    top_p: 0.95,
    frequency_penalty: 0.1,
    presence_penalty: 0.1,
  },
  "rapid_solver": {
    temperature: 0.2,
    top_p: 0.5,
    frequency_penalty: 0.0,
    presence_penalty: 0.0,
  }
};

export default function TunePage() {
  const [settings, setSettings] = useState<TuneSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const router = useRouter();

  const [descriptors, setDescriptors] = useState<Record<string, any>>({});
  const [validationNotes, setValidationNotes] = useState<string[]>([]);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const fetchSettings = useCallback(() => {
    setIsLoading(true);
    setLoadError(null);
    const gatewayUrl = getGatewayUrl();
    
    // Fetch settings and descriptors in parallel
    Promise.all([
      fetch(`${gatewayUrl}/api/settings`).then(r => r.json()),
      fetch(`${gatewayUrl}/api/models`).then(r => r.json())
    ])
    .then(([data, modelData]) => {
      if (data.status === "error") throw new Error(data.message);
      
      setSettings({
        temperature: data.temperature ?? 0.7,
        top_p: data.top_p ?? 0.9,
        frequency_penalty: data.frequency_penalty ?? 0.0,
        presence_penalty: data.presence_penalty ?? 0.0,
      });

      if (modelData.parameter_descriptors) {
        setDescriptors(modelData.parameter_descriptors);
      }
      
      setIsLoading(false);
    })
    .catch((e) => {
      console.error(e);
      setLoadError(`Gateway sync failed: ${e.message}`);
      setIsLoading(false);
    });
  }, []);

  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  const handleSaveBatch = async (newSettings: TuneSettings) => {
    setSettings(newSettings);
    setSaving(true);
    setValidationNotes([]);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/settings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(newSettings),
      });
      
      const data = await resp.json();
      if (!resp.ok) throw new Error(data.message || `HTTP ${resp.status}`);
      
      if (data.notes && data.notes.length > 0) {
        setValidationNotes(data.notes);
      }
      
      setSaved(true);
      setTimeout(() => {
        setSaved(false);
        setValidationNotes([]);
      }, 5000);
    } catch (e: any) {
      logger.error('Tune', `Failed to save batch:`, e);
      setLoadError(`Save failed: ${e.message}`);
    }
    setSaving(false);
  };

  const handleSaveSingle = async (key: string, value: number) => {
    if (!settings) return;
    const nextSettings = { ...settings, [key]: value };
    setSettings(nextSettings);
    
    setSaving(true);
    setValidationNotes([]);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/settings`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ [key]: value }),
      });
      
      const data = await resp.json();
      if (!resp.ok) throw new Error(data.message || `HTTP ${resp.status}`);
      
      if (data.notes && data.notes.length > 0) {
        setValidationNotes(data.notes);
      }

      setSaved(true);
      setTimeout(() => {
        setSaved(false);
        setValidationNotes([]);
      }, 5000);
    } catch (e: any) {
      logger.error('Tune', `Failed to save ${key}:`, e);
      setLoadError(`Save failed: ${e.message}`);
    }
    setSaving(false);
  };

  const handleReset = async () => {
    if (!confirm("Are you sure you want to restore all AI settings to system defaults?")) return;
    setSaving(true);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/settings/reset`, { method: "POST" });
      const data = await resp.json();
      if (!resp.ok) throw new Error(data.message || `HTTP ${resp.status}`);
      
      setSuccessMessage("RESTORATION_COMPLETE: System defaults applied.");
      setTimeout(() => setSuccessMessage(null), 4000);
      fetchSettings();
    } catch (e: any) {
      logger.error('Tune', `Failed to reset:`, e);
      setLoadError(`Reset failed: ${e.message}`);
    }
    setSaving(false);
  };

  const applyPreset = (id: string) => {
    if (PRESETS[id]) {
      handleSaveBatch(PRESETS[id]);
    }
  };

  const renderSlider = (label: string, key: keyof TuneSettings, fallbackMin: number, fallbackMax: number, fallbackStep: number, fallbackDesc: string) => {
    if (!settings) return null;
    const value = settings[key];
    const desc = descriptors[key];
    
    const min = desc?.min ?? fallbackMin;
    const max = desc?.max ?? fallbackMax;
    const step = desc?.step ?? fallbackStep;
    const description = desc?.description ?? fallbackDesc;
    
    return (
      <div style={{ marginBottom: "24px", background: "rgba(255,255,255,0.02)", padding: "16px", borderRadius: "12px", border: "1px solid var(--border)" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
          <span style={{ fontSize: "11px", color: "var(--accent)", fontWeight: 900, letterSpacing: "1px" }}>{(desc?.display_name || label).toUpperCase()}</span>
          <span style={{ fontSize: "12px", fontWeight: 900, color: "#fff", fontFamily: "monospace", background: "rgba(0,213,255,0.1)", padding: "2px 8px", borderRadius: "4px" }}>
            {value.toFixed(2)}
          </span>
        </div>
        <p style={{ fontSize: "10px", color: "#666", marginBottom: "12px", lineHeight: "1.4" }}>{description}</p>
        <input 
          type="range" 
          min={min} 
          max={max} 
          step={step} 
          value={value} 
          onChange={(e) => setSettings({ ...settings, [key]: parseFloat(e.target.value) })}
          onMouseUp={(e) => handleSaveSingle(key, parseFloat((e.target as HTMLInputElement).value))}
          style={{ 
            width: "100%", 
            accentColor: "var(--accent)",
            cursor: "pointer",
            height: "4px",
            background: "rgba(255,255,255,0.1)",
            borderRadius: "2px"
          }} 
        />
      </div>
    );
  };

  if (isLoading) return <div style={{ padding: "40px", color: "var(--accent)", fontFamily: "monospace" }}>INITIALIZING_TUNE_CORE...</div>;

  return (
    <div style={{ padding: "40px", maxWidth: "1000px", margin: "0 auto", display: "grid", gridTemplateColumns: "1fr 300px", gap: "40px" }}>
      <div>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "32px" }}>
          <div>
            <h1 style={{ color: "var(--accent)", fontSize: "24px", marginBottom: "4px", letterSpacing: "2px", fontWeight: 900 }}>FINE-TUNER</h1>
            <p style={{ color: "#555", fontSize: "11px", letterSpacing: "0.5px" }}>Sovereign weight configuration for the manifestation engine.</p>
          </div>
          <button 
            onClick={() => router.push('/')}
            style={{ background: "rgba(255,255,255,0.03)", border: "1px solid var(--border)", color: "var(--accent)", padding: "8px 20px", borderRadius: "8px", fontSize: "10px", fontWeight: 900, cursor: "pointer", letterSpacing: "1px" }}
          >
            BACK TO SWARM
          </button>
        </div>

        {loadError && (
          <div style={{ background: "rgba(255, 68, 68, 0.05)", border: "1px solid rgba(255, 68, 68, 0.2)", padding: "20px", borderRadius: "12px", color: "#ff4444", marginBottom: "32px", fontSize: "12px", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span>{loadError}</span>
            <button onClick={fetchSettings} style={{ background: "rgba(255,68,68,0.2)", border: "1px solid #ff4444", color: "#ff4444", padding: "4px 12px", borderRadius: "4px", fontSize: "10px", fontWeight: 900, cursor: "pointer" }}>RETRY</button>
          </div>
        )}

        {successMessage && (
          <div style={{ background: "rgba(0, 213, 255, 0.05)", border: "1px solid var(--accent)", padding: "16px", borderRadius: "12px", color: "var(--accent)", marginBottom: "32px", fontSize: "11px", fontWeight: 800 }}>
            {successMessage}
          </div>
        )}

        {validationNotes.length > 0 && (
          <div style={{ background: "rgba(255, 170, 0, 0.05)", border: "1px solid #ffaa00", padding: "16px", borderRadius: "12px", color: "#ffaa00", marginBottom: "32px", fontSize: "10px" }}>
            <div style={{ fontWeight: 900, marginBottom: "4px", letterSpacing: "1px" }}>GUARDIAN_INTERVENTION:</div>
            {validationNotes.map((note, i) => <div key={i}>• {note}</div>)}
          </div>
        )}

        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "20px" }}>
          {renderSlider("Temperature", "temperature", 0, 2, 0.05, "Entropy control. Higher = Creative expansion.")}
          {renderSlider("Top P", "top_p", 0, 1, 0.01, "Nucleus threshold. Lower = Deterministic focus.")}
          {renderSlider("Frequency Penalty", "frequency_penalty", -2, 2, 0.1, "Repetition suppression. Higher = Structural variety.")}
          {renderSlider("Presence Penalty", "presence_penalty", -2, 2, 0.1, "Topic innovation. Higher = Infinite observation.")}
        </div>

        <div style={{ marginTop: "32px", padding: "20px", borderRadius: "12px", background: "rgba(0,213,255,0.03)", border: "1px solid rgba(0,213,255,0.1)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: "12px", fontWeight: 900, color: "var(--accent)", marginBottom: "4px" }}>STILLNESS & PRESENCE MODE</div>
            <div style={{ fontSize: "10px", color: "#666" }}>Auto-optimizes penalties for deep relational observation.</div>
          </div>
          <button 
            onClick={() => handleSaveBatch({ ...settings!, frequency_penalty: 0.4, presence_penalty: 0.3 })}
            style={{ 
              background: settings?.presence_penalty === 0.3 ? "var(--accent)" : "transparent",
              color: settings?.presence_penalty === 0.3 ? "#000" : "var(--accent)",
              border: "1px solid var(--accent)",
              padding: "6px 16px", borderRadius: "20px", fontSize: "10px", fontWeight: 900, cursor: "pointer"
            }}
          >
            {settings?.presence_penalty === 0.3 ? "ACTIVE" : "ACTIVATE"}
          </button>
        </div>

        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "40px", paddingTop: "20px", borderTop: "1px solid var(--border)" }}>
          <div style={{ fontSize: "10px", color: "#444", letterSpacing: "1px", fontFamily: "monospace" }}>
            {saving ? "SYNCING_SAVANT_TOML..." : saved ? "✓ CONFIG_PERSISTED" : "AUTO_SAVE_ENABLED"}
          </div>
          <div style={{ display: "flex", gap: "12px" }}>
            <button 
              onClick={handleReset}
              style={{ background: "rgba(255,68,68,0.05)", border: "1px solid rgba(255,68,68,0.2)", color: "#ff4444", padding: "6px 12px", borderRadius: "6px", fontSize: "10px", fontWeight: 900, cursor: "pointer" }}
            >
              RESET TO DEFAULTS
            </button>
            <button 
              onClick={() => window.location.reload()}
              style={{ background: "transparent", border: "1px solid var(--border)", color: "#555", padding: "6px 12px", borderRadius: "6px", fontSize: "10px", fontWeight: 900, cursor: "pointer" }}
            >
              RELOAD 
            </button>
          </div>
        </div>
      </div>

      <aside style={{ borderLeft: "1px solid var(--border)", paddingLeft: "40px" }}>
        <h2 style={{ fontSize: "12px", fontWeight: 900, letterSpacing: "2px", marginBottom: "24px", color: "#fff", opacity: 0.6 }}>DEMEANOR_PRESETS</h2>
        
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          <button onClick={() => applyPreset("deep_observer")} style={{ textAlign: "left", background: "rgba(255,255,255,0.02)", border: "1px solid var(--border)", padding: "16px", borderRadius: "12px", cursor: "pointer", transition: "all 0.2s" }}>
            <div style={{ fontSize: "11px", color: "var(--accent)", fontWeight: 900, marginBottom: "4px" }}>DEEP OBSERVER</div>
            <div style={{ fontSize: "9px", color: "#666", lineHeight: "1.4" }}>Focused, still, and observant. Minimal repetition.</div>
          </button>

          <button onClick={() => applyPreset("creative_spark")} style={{ textAlign: "left", background: "rgba(255,255,255,0.02)", border: "1px solid var(--border)", padding: "16px", borderRadius: "12px", cursor: "pointer", transition: "all 0.2s" }}>
            <div style={{ fontSize: "11px", color: "var(--accent)", fontWeight: 900, marginBottom: "4px" }}>CREATIVE SPARK</div>
            <div style={{ fontSize: "9px", color: "#666", lineHeight: "1.4" }}>Expansive thoughts, diverse vocabulary, and rapid ideation.</div>
          </button>

          <button onClick={() => applyPreset("rapid_solver")} style={{ textAlign: "left", background: "rgba(255,255,255,0.02)", border: "1px solid var(--border)", padding: "16px", borderRadius: "12px", cursor: "pointer", transition: "all 0.2s" }}>
            <div style={{ fontSize: "11px", color: "var(--accent)", fontWeight: 900, marginBottom: "4px" }}>RAPID SOLVER</div>
            <div style={{ fontSize: "9px", color: "#666", lineHeight: "1.4" }}>Direct, deterministic, and efficient responses.</div>
          </button>
        </div>

        <div style={{ marginTop: "40px", padding: "20px", borderRadius: "12px", background: "#111", border: "1px solid #222" }}>
          <div style={{ fontSize: "10px", fontWeight: 900, color: "#444", marginBottom: "8px", letterSpacing: "1px" }}>TUNE_STATISTICS</div>
          <div style={{ fontSize: "9px", color: "#555", display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
            <span>DISK_IO:</span> <span>SUCCESS</span>
          </div>
          <div style={{ fontSize: "9px", color: "#555", display: "flex", justifyContent: "space-between" }}>
            <span>SYNC_LATENCY:</span> <span>&lt;2ms</span>
          </div>
        </div>
      </aside>
    </div>
  );
}
