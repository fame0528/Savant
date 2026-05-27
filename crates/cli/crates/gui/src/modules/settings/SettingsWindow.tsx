import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "@/lib/store";

interface SettingsState {
  // AI
  provider: string;
  model: string;
  apiKey: string;
  temperature: number;
  maxTokens: number;
  // Server
  gatewayUrl: string;
  // Evolution
  evolutionEnabled: boolean;
  mutationRate: number;
  // Obsidian
  vaultPath: string;
  // Theme
  theme: string;
}

const DEFAULT_SETTINGS: SettingsState = {
  provider: "openai",
  model: "gpt-4",
  apiKey: "",
  temperature: 0.7,
  maxTokens: 4096,
  gatewayUrl: "ws://localhost:3000/ws",
  evolutionEnabled: true,
  mutationRate: 0.1,
  vaultPath: "",
  theme: "void",
};

const PROVIDERS = [
  { id: "openai", label: "OpenAI" },
  { id: "anthropic", label: "Anthropic" },
  { id: "google", label: "Google" },
  { id: "groq", label: "Groq" },
  { id: "xai", label: "xAI" },
  { id: "ollama", label: "Ollama (Local)" },
  { id: "lmstudio", label: "LM Studio" },
];

export function SettingsWindow({ onClose }: { onClose: () => void }) {
  const { theme, setTheme } = useAppStore();
  const [settings, setSettings] = useState<SettingsState>({ ...DEFAULT_SETTINGS, theme });
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [activeSection, setActiveSection] = useState("ai");

  useEffect(() => {
    // Load existing API keys
    const loadKeys = async () => {
      for (const provider of PROVIDERS) {
        try {
          const key = await invoke<string | null>("get_api_key", { provider: provider.id });
          if (key) {
            setSettings((prev) => ({ ...prev, apiKey: key }));
          }
        } catch {
          // Ignore
        }
      }
    };
    loadKeys();
  }, []);

  const handleSave = async () => {
    setSaving(true);
    try {
      if (settings.apiKey) {
        await invoke("set_api_key", { provider: settings.provider, key: settings.apiKey });
      }
      await invoke("gateway_connect", { url: settings.gatewayUrl });
      setTheme(settings.theme as typeof theme);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save settings:", e);
    } finally {
      setSaving(false);
    }
  };

  const updateSetting = <K extends keyof SettingsState>(key: K, value: SettingsState[K]) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
  };

  const sections = [
    { id: "ai", label: "AI Models" },
    { id: "server", label: "Server" },
    { id: "evolution", label: "Evolution" },
    { id: "obsidian", label: "Obsidian" },
    { id: "theme", label: "Theme" },
    { id: "shortcuts", label: "Shortcuts" },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-void border border-primary/20 rounded-lg w-[700px] h-[500px] flex flex-col shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-primary/15">
          <h2 className="text-primary font-bold text-sm">Settings</h2>
          <button
            onClick={onClose}
            className="text-primary/40 hover:text-primary transition-colors text-lg leading-none"
          >
            ×
          </button>
        </div>

        <div className="flex flex-1 overflow-hidden">
          {/* Sidebar */}
          <div className="w-40 border-r border-primary/10 py-2 shrink-0">
            {sections.map((section) => (
              <button
                key={section.id}
                onClick={() => setActiveSection(section.id)}
                className={`w-full text-left px-3 py-2 text-xs transition-colors ${
                  activeSection === section.id
                    ? "bg-primary/10 text-primary border-r-2 border-r-primary"
                    : "text-success/50 hover:text-success/80 hover:bg-primary/5"
                }`}
              >
                {section.label}
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="flex-1 overflow-y-auto p-4">
            {activeSection === "ai" && (
              <div className="space-y-4">
                <h3 className="text-primary text-xs font-bold uppercase">AI Provider</h3>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">Provider</label>
                  <select
                    value={settings.provider}
                    onChange={(e) => updateSetting("provider", e.target.value)}
                    className="w-full bg-void/50 border border-primary/15 rounded px-3 py-1.5 text-xs text-success"
                  >
                    {PROVIDERS.map((p) => (
                      <option key={p.id} value={p.id}>{p.label}</option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">Model</label>
                  <input
                    type="text"
                    value={settings.model}
                    onChange={(e) => updateSetting("model", e.target.value)}
                    className="w-full bg-void/50 border border-primary/15 rounded px-3 py-1.5 text-xs text-success"
                    placeholder="gpt-4, claude-3-opus, etc."
                  />
                </div>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">API Key</label>
                  <input
                    type="password"
                    value={settings.apiKey}
                    onChange={(e) => updateSetting("apiKey", e.target.value)}
                    className="w-full bg-void/50 border border-primary/15 rounded px-3 py-1.5 text-xs text-success"
                    placeholder="sk-..."
                  />
                </div>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">
                    Temperature: {settings.temperature}
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="2"
                    step="0.1"
                    value={settings.temperature}
                    onChange={(e) => updateSetting("temperature", parseFloat(e.target.value))}
                    className="w-full"
                  />
                </div>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">Max Tokens</label>
                  <input
                    type="number"
                    value={settings.maxTokens}
                    onChange={(e) => updateSetting("maxTokens", parseInt(e.target.value))}
                    className="w-full bg-void/50 border border-primary/15 rounded px-3 py-1.5 text-xs text-success"
                  />
                </div>
              </div>
            )}

            {activeSection === "server" && (
              <div className="space-y-4">
                <h3 className="text-primary text-xs font-bold uppercase">Gateway Server</h3>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">Gateway URL</label>
                  <input
                    type="text"
                    value={settings.gatewayUrl}
                    onChange={(e) => updateSetting("gatewayUrl", e.target.value)}
                    className="w-full bg-void/50 border border-primary/15 rounded px-3 py-1.5 text-xs text-success"
                    placeholder="ws://localhost:3000/ws"
                  />
                </div>
              </div>
            )}

            {activeSection === "evolution" && (
              <div className="space-y-4">
                <h3 className="text-primary text-xs font-bold uppercase">Evolution</h3>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={settings.evolutionEnabled}
                    onChange={(e) => updateSetting("evolutionEnabled", e.target.checked)}
                    className="accent-primary"
                  />
                  <label className="text-xs text-success/70">Enable evolution mutations</label>
                </div>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">
                    Mutation Rate: {settings.mutationRate}
                  </label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={settings.mutationRate}
                    onChange={(e) => updateSetting("mutationRate", parseFloat(e.target.value))}
                    className="w-full"
                  />
                </div>
              </div>
            )}

            {activeSection === "obsidian" && (
              <div className="space-y-4">
                <h3 className="text-primary text-xs font-bold uppercase">Obsidian Vault</h3>
                <div>
                  <label className="text-xs text-primary/50 block mb-1">Vault Path</label>
                  <input
                    type="text"
                    value={settings.vaultPath}
                    onChange={(e) => updateSetting("vaultPath", e.target.value)}
                    className="w-full bg-void/50 border border-primary/15 rounded px-3 py-1.5 text-xs text-success"
                    placeholder="~/obsidian-vault"
                  />
                </div>
              </div>
            )}

            {activeSection === "theme" && (
              <div className="space-y-4">
                <h3 className="text-primary text-xs font-bold uppercase">Theme</h3>
                {["void", "tokyo-night", "nord", "github-dark"].map((t) => (
                  <button
                    key={t}
                    onClick={() => { updateSetting("theme", t); setTheme(t as typeof theme); }}
                    className={`w-full text-left px-3 py-2 rounded text-xs transition-colors ${
                      settings.theme === t
                        ? "bg-primary/15 text-primary border border-primary/30"
                        : "text-success/50 hover:text-success/80 hover:bg-primary/5 border border-transparent"
                    }`}
                  >
                    {t}
                  </button>
                ))}
              </div>
            )}

            {activeSection === "shortcuts" && (
              <div className="space-y-4">
                <h3 className="text-primary text-xs font-bold uppercase">Keyboard Shortcuts</h3>
                <div className="space-y-2 text-xs">
                  {[
                    ["Ctrl+T", "New terminal tab"],
                    ["Ctrl+W", "Close tab"],
                    ["Ctrl+P", "Command palette"],
                    ["Ctrl+B", "Toggle sidebar"],
                    ["Ctrl+`", "Toggle terminal"],
                    ["Ctrl+Shift+P", "Settings"],
                    ["Ctrl+S", "Save file"],
                    ["Ctrl+Shift+F", "Search files"],
                  ].map(([key, desc]) => (
                    <div key={key} className="flex items-center gap-3">
                      <kbd className="px-2 py-0.5 bg-primary/10 text-primary rounded text-[10px] min-w-[80px] text-center">
                        {key}
                      </kbd>
                      <span className="text-success/50">{desc}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-primary/15">
          <span className="text-xs text-primary/20">
            {saved && <span className="text-success">✓ Saved</span>}
          </span>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="px-3 py-1 text-xs text-success/50 hover:text-success border border-primary/10 rounded hover:bg-primary/5 transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleSave}
              disabled={saving}
              className="px-3 py-1 text-xs bg-primary/15 text-primary rounded hover:bg-primary/25 disabled:opacity-30 transition-colors"
            >
              {saving ? "Saving..." : "Save"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
