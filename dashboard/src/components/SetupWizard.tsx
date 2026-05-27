"use client";

import { useState, useEffect, useRef, useCallback } from "react";
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

function isTauri(): boolean {
  if (typeof window === "undefined") return false;
  return !!(window as any).__TAURI_INTERNALS__ || !!(window as any).__TAURI__;
}

async function openUrl(url: string) {
  if (isTauri()) {
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
      return;
    } catch { /* fall through */ }
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

async function pickFolder(): Promise<string | null> {
  if (isTauri()) {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false, title: "Select Obsidian Vault Folder" });
      if (selected && typeof selected === "string") return selected;
      return null;
    } catch (e) {
      console.warn("[SetupWizard] Dialog plugin failed, trying invoke:", e);
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const selected = await invoke<string | null>("plugin:dialog|open", {
          options: { directory: true, multiple: false, title: "Select Obsidian Vault Folder" },
        });
        if (selected && typeof selected === "string") return selected;
      } catch (e2) {
        console.error("[SetupWizard] Dialog invoke failed:", e2);
      }
      return null;
    }
  }
  return null;
}

function detectHardware(): { totalRam: number; cpuCores: number; isLaptop: boolean } {
  const nav = navigator as unknown as Record<string, unknown>;
  const ram = (nav.deviceMemory as number) || 8;
  const cores = navigator.hardwareConcurrency || 4;
  const isLaptop = /mobile|tablet|ipad|iphone|android/i.test(navigator.userAgent) || cores <= 4;
  return { totalRam: ram, cpuCores: cores, isLaptop };
}

function recommendVariant(hw: { totalRam: number; isLaptop: boolean }): GemmaVariant {
  const score = hw.totalRam * 0.5 + (hw.isLaptop ? -2 : 0);
  if (score >= 20) return GEMMA_VARIANTS[3];
  if (score >= 12) return GEMMA_VARIANTS[2];
  if (score >= 6) return GEMMA_VARIANTS[1];
  return GEMMA_VARIANTS[0];
}

type Phase = "detect" | "configure" | "install" | "done";

export default function SetupWizard({ onComplete }: SetupWizardProps) {
  const [phase, setPhase] = useState<Phase>("detect");
  const [selectedVariant, setSelectedVariant] = useState<GemmaVariant | null>(null);
  const [vaultPath, setVaultPath] = useState("");
  const [installing, setInstalling] = useState(false);
  const [installStatus, setInstallStatus] = useState("");
  const [installProgress, setInstallProgress] = useState<{ completed?: number; total?: number } | null>(null);
  const [installElapsed, setInstallElapsed] = useState(0);
  const [ollamaRunning, setOllamaRunning] = useState(false);
  const [startingOllama, setStartingOllama] = useState(false);
  const [startOllamaMsg, setStartOllamaMsg] = useState("");
  const [retryCount, setRetryCount] = useState(0);
  const [errorMsg, setErrorMsg] = useState("");
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const elapsedRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const checkOllama = useCallback(async (): Promise<{ running: boolean; modelFound: boolean; modelName?: string }> => {
    try {
      const resp = await fetch(`${getGatewayUrl()}/api/setup/check`);
      const data = await resp.json();
      setOllamaRunning(data.ollama_running);
      return {
        running: data.ollama_running,
        modelFound: data.model_available,
        modelName: data.model_name,
      };
    } catch {
      setOllamaRunning(false);
      return { running: false, modelFound: false };
    }
  }, []);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  const startPolling = useCallback(() => {
    if (pollRef.current) return;
    pollRef.current = setInterval(async () => {
      setRetryCount((c) => c + 1);
      const result = await checkOllama();
      if (result.running && result.modelFound) {
        stopPolling();
        setPhase("done");
        localStorage.setItem("savant.setup.complete", "true");
        setTimeout(() => onComplete(), 1500);
      } else if (result.running) {
        stopPolling();
        setPhase("configure");
      }
    }, 3000);
  }, [checkOllama, stopPolling, onComplete]);

  // Initial auto-detection
  useEffect(() => {
    const hw = detectHardware();
    setSelectedVariant(recommendVariant(hw));

    (async () => {
      const result = await checkOllama();
      if (result.running && result.modelFound) {
        // Everything ready — skip wizard entirely
        setPhase("done");
        localStorage.setItem("savant.setup.complete", "true");
        setTimeout(() => onComplete(), 1500);
        return;
      }
      if (result.running) {
        setPhase("configure");
      } else {
        setPhase("configure");
        startPolling();
      }
    })();

    return () => stopPolling();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleStartOllama = async () => {
    setStartingOllama(true);
    setStartOllamaMsg("Starting Ollama...");
    setErrorMsg("");
    try {
      const resp = await fetch(`${getGatewayUrl()}/api/setup/start-ollama`, { method: "POST" });
      const data = await resp.json();
      if (data.status === "success") {
        setStartOllamaMsg(data.message);
        setOllamaRunning(true);
        const result = await checkOllama();
        if (result.modelFound) {
          setPhase("done");
          localStorage.setItem("savant.setup.complete", "true");
          setTimeout(() => onComplete(), 1500);
          return;
        }
        setPhase("configure");
      } else {
        setStartOllamaMsg(data.message || "Failed to start Ollama");
        setErrorMsg(data.message || "Failed to start Ollama");
      }
    } catch {
      setStartOllamaMsg("Could not reach gateway");
      setErrorMsg("Could not reach gateway — is Savant running?");
    } finally {
      setStartingOllama(false);
    }
  };

  const saveConfig = async (modelTag: string, vault: string) => {
    const updates = [
      { section: "browser", key: "vision_model", value: modelTag },
      { section: "browser", key: "embedding_model", value: "nomic-embed-text" },
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
      } catch { /* continue */ }
    }
  };

  const startInstall = async () => {
    if (!selectedVariant) return;
    setPhase("install");
    setInstalling(true);
    setInstallElapsed(0);
    setInstallProgress(null);
    setInstallStatus(`Downloading ${selectedVariant.label}...`);
    stopPolling();
    setErrorMsg("");

    elapsedRef.current = setInterval(() => setInstallElapsed((t) => t + 1), 1000);

    try {
      await saveConfig(selectedVariant.tag, vaultPath);

      const resp = await fetch(`${getGatewayUrl()}/api/setup/install-model-stream`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ model: selectedVariant.tag }),
      });

      if (!resp.ok || !resp.body) {
        throw new Error(`HTTP ${resp.status}`);
      }

      const reader = resp.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n");
        buffer = lines.pop() || "";

        for (const line of lines) {
          if (!line.startsWith("data:")) continue;
          const jsonStr = line.slice(5).trim();
          if (!jsonStr) continue;
          try {
            const event = JSON.parse(jsonStr);
            if (event.status === "progress") {
              setInstallStatus(event.message || "Pulling...");
              if (event.completed !== undefined && event.total !== undefined) {
                setInstallProgress({ completed: event.completed, total: event.total });
              }
            } else if (event.status === "success") {
              if (elapsedRef.current) clearInterval(elapsedRef.current);
              setInstallStatus(`${selectedVariant.label} installed successfully!`);
              setInstalling(false);
              localStorage.setItem("savant.setup.complete", "true");
              setTimeout(() => onComplete(), 2000);
              return;
            } else if (event.status === "error") {
              throw new Error(event.message);
            }
          } catch { /* skip malformed */ }
        }
      }

      if (elapsedRef.current) clearInterval(elapsedRef.current);
      setInstallStatus(`${selectedVariant.label} installed successfully!`);
      setInstalling(false);
      localStorage.setItem("savant.setup.complete", "true");
      setTimeout(() => onComplete(), 2000);
    } catch (e) {
      if (elapsedRef.current) clearInterval(elapsedRef.current);
      const msg = e instanceof Error ? e.message : "Connection error";
      setInstallStatus(`Error: ${msg}`);
      setErrorMsg(msg);
      setInstalling(false);
    }
  };

  const handleSkip = () => {
    localStorage.setItem("savant.setup.complete", "true");
    onComplete();
  };

  const handlePickFolder = async () => {
    const path = await pickFolder();
    if (path) setVaultPath(path);
  };

  const formatElapsed = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return m > 0 ? `${m}m ${sec}s` : `${sec}s`;
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
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
          <div className={styles.checkOk}>&#10003;</div>
          <div className={styles.subtitle}>Gemma is installed and ready to use.</div>
        </div>
      </div>
    );
  }

  if (phase === "install") {
    const pct = installProgress?.total
      ? Math.round(((installProgress.completed || 0) / installProgress.total) * 100)
      : null;

    return (
      <div className={styles.overlay}>
        <div className={styles.card}>
          <div className={styles.title}>Installing Gemma</div>
          {installing && <div className={styles.spinner} />}
          <div className={styles.subtitle}>{installStatus}</div>
          {pct !== null && installProgress?.total && (
            <div className={styles.progressBar}>
              <div className={styles.progressFill} style={{ width: `${pct}%` }} />
              <span className={styles.progressLabel}>
                {pct}% — {formatBytes(installProgress.completed || 0)} / {formatBytes(installProgress.total)}
              </span>
            </div>
          )}
          {installing && installElapsed > 0 && (
            <div className={styles.elapsed}>
              {formatElapsed(installElapsed)} elapsed
              {installElapsed > 120 && " — large model, this may take a few minutes"}
            </div>
          )}
          {!installing && errorMsg && (
            <button onClick={() => { setPhase("configure"); setInstallStatus(""); setErrorMsg(""); startPolling(); }} className={styles.retryBtn}>
              Go Back
            </button>
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
          {ollamaRunning
            ? "Select a model to install. You can change these anytime in Settings."
            : "Start Ollama to continue. You can change these anytime in Settings."}
        </div>

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
                {recommendVariant(detectHardware()).tag === v.tag && (
                  <span className={styles.recommendedBadge}>Recommended</span>
                )}
              </div>
              <div className={styles.variantDesc}>{v.desc}</div>
            </button>
          ))}
        </div>

        <div className={styles.sectionLabel}>Obsidian Vault Path (optional)</div>
        <div className={styles.inputRow}>
          <input
            type="text"
            className={styles.textInput}
            placeholder="Leave empty for default (workspace/memory-vault)"
            value={vaultPath}
            onChange={(e) => setVaultPath(e.target.value)}
          />
          <button className={styles.browseBtn} onClick={handlePickFolder} title="Browse for folder">&#128193;</button>
        </div>
        <div className={styles.inputHint}>Where your memory vault will be stored. You can change this later in Settings.</div>

        {!ollamaRunning && (
          <div className={styles.issue}>
            <div className={styles.issueRow}>
              <span className={styles.issueIcon}>&#9888;</span>
              <div style={{ flex: 1 }}>
                <strong>Ollama is not running.</strong>
                <div className={styles.issueSteps}>
                  <button className={styles.startOllamaBtn} onClick={handleStartOllama} disabled={startingOllama}>
                    {startingOllama ? "Starting Ollama..." : "Start Ollama"}
                  </button>
                  <span className={styles.orDivider}>or</span>
                  <button className={styles.linkBtn} onClick={() => openUrl("https://ollama.com/download")}>
                    Download &amp; install Ollama
                  </button>
                </div>
                {startOllamaMsg && <div className={styles.startOllamaMsg}>{startOllamaMsg}</div>}
                <div className={styles.pollStatus}>
                  {retryCount > 0 && (
                    <><span className={styles.pollDot} /> Auto-detecting Ollama... (checked {retryCount}x)</>
                  )}
                </div>
              </div>
            </div>
          </div>
        )}

        {ollamaRunning && !installing && (
          <div className={styles.success}><span>&#10003;</span> Ollama detected and running</div>
        )}

        {errorMsg && (
          <div className={styles.issue} style={{ marginTop: 0 }}>
            <span className={styles.issueIcon}>&#9888;</span> {errorMsg}
          </div>
        )}

        <button onClick={startInstall} disabled={!selectedVariant || !ollamaRunning} className={styles.installBtn}>
          {ollamaRunning ? `Install ${selectedVariant?.label || "Gemma"}` : "Waiting for Ollama..."}
        </button>

        <button onClick={handleSkip} className={styles.skipBtn}>Skip for Now</button>
      </div>
    </div>
  );
}
