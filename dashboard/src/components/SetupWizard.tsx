"use client";

import { useState, useEffect } from "react";
import styles from "./SetupWizard.module.css";

interface GemmaVariant {
  tag: string;
  label: string;
  vram: string;
  desc: string;
  minRam: number;
}

const GEMMA_VARIANTS: GemmaVariant[] = [
  { tag: "gemma4:e2b", label: "Gemma 4 E2B", vram: "3 GB", desc: "Minimal. Runs on any hardware, including CPU-only.", minRam: 4 },
  { tag: "gemma4:e4b", label: "Gemma 4 E4B", vram: "8 GB", desc: "Recommended. Best quality-to-size ratio.", minRam: 10 },
  { tag: "gemma4:26b", label: "Gemma 4 26B", vram: "18 GB", desc: "High performance. Needs a powerful GPU.", minRam: 24 },
  { tag: "gemma4:31b", label: "Gemma 4 31B", vram: "22 GB", desc: "Maximum quality. Workstation-grade hardware.", minRam: 32 },
];

interface HardwareInfo {
  totalRam: number;
  cpuCores: number;
  gpuName: string;
  isLaptop: boolean;
}

interface SetupWizardProps {
  onComplete: () => void;
}

const getGatewayUrl = () => {
  if (typeof window !== "undefined") {
    const host = window.location.hostname || "127.0.0.1";
    const port = process.env.NEXT_PUBLIC_GATEWAY_PORT || "8080";
    return `http://${host}:${port}`;
  }
  return "http://localhost:8080";
};

function detectHardware(): HardwareInfo {
  const nav = navigator as unknown as Record<string, unknown>;
  const ram = (nav.deviceMemory as number) || 8;
  const cores = navigator.hardwareConcurrency || 4;
  const isLaptop = /mobile|tablet|ipad|iphone|android/i.test(navigator.userAgent) || cores <= 4;

  let gpuName = "Unknown";
  try {
    const canvas = document.createElement("canvas");
    const gl = canvas.getContext("webgl") as WebGLRenderingContext | null;
    if (gl) {
      const debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
      if (debugInfo) {
        gpuName = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL) || "Unknown";
      }
    }
  } catch {
    // WebGL not available
  }

  return { totalRam: ram, cpuCores: cores, gpuName, isLaptop };
}

function recommendVariant(hw: HardwareInfo): GemmaVariant {
  const score = hw.totalRam * 0.5 + (hw.isLaptop ? -2 : 0);
  if (score >= 20) return GEMMA_VARIANTS[3];
  if (score >= 12) return GEMMA_VARIANTS[2];
  if (score >= 6) return GEMMA_VARIANTS[1];
  return GEMMA_VARIANTS[0];
}

export default function SetupWizard({ onComplete }: SetupWizardProps) {
  const [phase, setPhase] = useState<"detect" | "configure" | "install" | "done">("detect");
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [selectedVariant, setSelectedVariant] = useState<GemmaVariant | null>(null);
  const [vaultPath, setVaultPath] = useState("");
  const [installing, setInstalling] = useState(false);
  const [installStatus, setInstallStatus] = useState("");
  const [ollamaRunning, setOllamaRunning] = useState(false);

  useEffect(() => {
    const hw = detectHardware();
    setHardware(hw);
    setSelectedVariant(recommendVariant(hw));
    checkOllama();
  }, []);

  const checkOllama = async () => {
    try {
      const resp = await fetch(`${getGatewayUrl()}/api/setup/check`);
      const data = await resp.json();
      setOllamaRunning(data.ollama_running);
      if (data.ollama_running && data.model_available) {
        setPhase("done");
        localStorage.setItem("savant.setup.complete", "true");
        setTimeout(() => onComplete(), 1500);
      } else {
        setPhase("configure");
      }
    } catch {
      setOllamaRunning(false);
      setPhase("configure");
    }
  };

  const saveConfig = async (modelTag: string, vault: string) => {
    const updates = [
      { section: "browser", key: "vision_model", value: modelTag },
      { section: "browser", key: "embedding_model", value: modelTag },
    ];
    if (vault.trim()) {
      updates.push({ section: "obsidian", key: "vault_path", value: vault.trim() });
    }
    for (const u of updates) {
      try {
        await fetch(`${getGatewayUrl()}/api/config/set`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(u),
        });
      } catch {
        // Continue even if one fails
      }
    }
  };

  const startInstall = async () => {
    if (!selectedVariant) return;
    setPhase("install");
    setInstalling(true);
    setInstallStatus(`Downloading ${selectedVariant.label}...`);

    try {
      // Save config first
      await saveConfig(selectedVariant.tag, vaultPath);

      // Pull the model
      const resp = await fetch(`${getGatewayUrl()}/api/setup/install-model`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: selectedVariant.tag }),
      });
      const data = await resp.json();
      if (data.status === "success") {
        setInstallStatus(`${selectedVariant.label} installed successfully!`);
        setInstalling(false);
        localStorage.setItem("savant.setup.complete", "true");
        setTimeout(() => onComplete(), 2000);
      } else {
        setInstallStatus(`Error: ${data.message}`);
        setInstalling(false);
      }
    } catch {
      setInstallStatus("Failed to connect to gateway");
      setInstalling(false);
    }
  };

  if (phase === "detect") {
    return (
      <div className={styles.overlay}>
        <div className={styles.card}>
          <div className={styles.title}>Detecting Hardware</div>
          <div className={styles.spinner} />
          <div className={styles.subtitle}>Analyzing your system capabilities...</div>
        </div>
      </div>
    );
  }

  if (phase === "done") {
    return (
      <div className={styles.overlay}>
        <div className={styles.card}>
          <div className={styles.title}>Ready</div>
          <div className={styles.checkOk}>✓</div>
          <div className={styles.subtitle}>Gemma is installed and ready to use.</div>
        </div>
      </div>
    );
  }

  if (phase === "install") {
    return (
      <div className={styles.overlay}>
        <div className={styles.card}>
          <div className={styles.title}>Installing Gemma</div>
          {installing && <div className={styles.spinner} />}
          <div className={styles.subtitle}>
            {installStatus || `Downloading ${selectedVariant?.label}...`}
          </div>
          {installStatus && !installing && (
            <div className={styles.statusMsg}>{installStatus}</div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className={styles.overlay}>
      <div className={styles.card}>
        <div className={styles.title}>Setup Savant</div>
        <div className={styles.subtitle}>
          Configure your local AI model and vault. You can change these anytime in Settings.
        </div>

        {hardware && (
          <div className={styles.hwInfo}>
            <div className={styles.hwLabel}>Detected Hardware</div>
            <div className={styles.hwDetails}>
              <span>RAM: ~{hardware.totalRam} GB</span>
              <span>CPU: {hardware.cpuCores} cores</span>
              <span>GPU: {hardware.gpuName !== "Unknown" ? hardware.gpuName : "Unknown"}</span>
            </div>
          </div>
        )}

        <div className={styles.sectionLabel}>Gemma Model</div>
        <div className={styles.variantList}>
          {GEMMA_VARIANTS.map((v) => (
            <button
              key={v.tag}
              className={`${styles.variantCard} ${selectedVariant?.tag === v.tag ? styles.variantSelected : ""}`}
              onClick={() => setSelectedVariant(v)}
            >
              <div className={styles.variantHeader}>
                <span className={styles.variantLabel}>{v.label}</span>
                <span className={styles.variantVram}>{v.vram} VRAM</span>
                {hardware && recommendVariant(hardware).tag === v.tag && (
                  <span className={styles.recommendedBadge}>Recommended</span>
                )}
              </div>
              <div className={styles.variantDesc}>{v.desc}</div>
            </button>
          ))}
        </div>

        <div className={styles.sectionLabel}>Obsidian Vault Path</div>
        <div className={styles.inputRow}>
          <input
            type="text"
            className={styles.textInput}
            placeholder="Leave empty for default (workspace/memory-vault)"
            value={vaultPath}
            onChange={(e) => setVaultPath(e.target.value)}
          />
        </div>
        <div className={styles.inputHint}>
          Where your memory vault will be stored. You can change this later in Settings.
        </div>

        {!ollamaRunning && (
          <div className={styles.issue}>
            Ollama is not running.{" "}
            <a href="https://ollama.com/download" target="_blank" rel="noopener noreferrer">
              Install Ollama
            </a>{" "}
            first, then return here.
          </div>
        )}

        <button
          onClick={startInstall}
          disabled={!selectedVariant || !ollamaRunning}
          className={styles.installBtn}
        >
          {ollamaRunning
            ? `Install ${selectedVariant?.label || "Gemma"}`
            : "Ollama Required"}
        </button>

        <button onClick={() => onComplete()} className={styles.skipBtn}>
          Skip for Now
        </button>
      </div>
    </div>
  );
}
