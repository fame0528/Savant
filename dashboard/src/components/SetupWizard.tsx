"use client";

import { useState, useEffect } from "react";
import styles from "./SetupWizard.module.css";

interface SetupCheck {
  ollama_running: boolean;
  ollama_installed: boolean;
  model_available: boolean;
  model_name: string;
  issues: string[];
  instructions: string[];
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

export default function SetupWizard({ onComplete }: SetupWizardProps) {
  const [check, setCheck] = useState<SetupCheck | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installStatus, setInstallStatus] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    runCheck();
  }, []);

  const runCheck = async () => {
    setLoading(true);
    try {
      const resp = await fetch(`${getGatewayUrl()}/api/setup/check`);
      const data: SetupCheck = await resp.json();
      setCheck(data);

      // If everything is good, dismiss
      if (data.issues.length === 0) {
        setTimeout(onComplete, 1500);
      }
    } catch {
      setCheck({
        ollama_running: false,
        ollama_installed: false,
        model_available: false,
        model_name: "qwen3-embedding:4b",
        issues: ["Cannot connect to gateway"],
        instructions: ["Make sure the gateway is running on port 8080"],
      });
    } finally {
      setLoading(false);
    }
  };

  const installModel = async () => {
    setInstalling(true);
    setInstallStatus("Downloading qwen3-embedding:4b...");
    try {
      const resp = await fetch(`${getGatewayUrl()}/api/setup/install-model`, {
        method: "POST",
      });
      const data = await resp.json();
      if (data.status === "success") {
        setInstallStatus("Model installed successfully!");
        await runCheck();
      } else {
        setInstallStatus(`Error: ${data.message}`);
      }
    } catch {
      setInstallStatus("Failed to connect to gateway");
    } finally {
      setInstalling(false);
    }
  };

  if (loading) {
    return (
      <div className={styles.overlay}>
        <div className={styles.card}>
          <div className={styles.title}>Checking Dependencies</div>
          <div className={styles.spinner} />
        </div>
      </div>
    );
  }

  if (!check || check.issues.length === 0) {
    return null; // Everything is good
  }

  return (
    <div className={styles.overlay}>
      <div className={styles.card}>
        <div className={styles.title}>Setup Required</div>
        <div className={styles.subtitle}>
          Savant needs a few things to run properly
        </div>

        <div className={styles.checklist}>
          <div className={styles.checkItem}>
            <span className={check.ollama_running ? styles.checkOk : styles.checkFail}>
              {check.ollama_running ? "✓" : "✗"}
            </span>
            <span>Ollama running</span>
          </div>
          <div className={styles.checkItem}>
            <span className={check.model_available ? styles.checkOk : styles.checkFail}>
              {check.model_available ? "✓" : "✗"}
            </span>
            <span>{check.model_name} available</span>
          </div>
        </div>

        {check.issues.map((issue, i) => (
          <div key={i} className={styles.issue}>{issue}</div>
        ))}

        {check.instructions.map((inst, i) => (
          <div key={i} className={styles.instruction}>{inst}</div>
        ))}

        {!check.model_available && check.ollama_running && (
          <button
            onClick={installModel}
            disabled={installing}
            className={styles.installBtn}
          >
            {installing ? "Installing..." : `Install ${check.model_name}`}
          </button>
        )}

        {installStatus && (
          <div className={styles.status}>{installStatus}</div>
        )}

        <button onClick={runCheck} className={styles.retryBtn}>
          Check Again
        </button>

        <button onClick={onComplete} className={styles.skipBtn}>
          Skip for Now
        </button>
      </div>
    </div>
  );
}
