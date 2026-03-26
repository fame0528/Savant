"use client";

import { useState, useEffect } from "react";
import styles from "./SplashScreen.module.css";

interface SplashScreenProps {
  onComplete: () => void;
}

interface StatusMessage {
  text: string;
  timestamp: number;
}

export default function SplashScreen({ onComplete }: SplashScreenProps) {
  const [status, setStatus] = useState<string>("Initializing...");
  const [history, setHistory] = useState<StatusMessage[]>([]);
  const [fadeOut, setFadeOut] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    // Auto-dismiss after 15s if no ignition event received
    const timeout = setTimeout(() => {
      setStatus("Gateway not responding — entering dashboard...");
      setTimeout(() => {
        setFadeOut(true);
        setTimeout(onComplete, 600);
      }, 1000);
    }, 15000);

    const setupListener = async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<string>("system-log-event", (event) => {
          const data = event.payload;
          if (!data) return;

          setStatus(data);
          setHistory((prev) => [
            ...prev.slice(-10),
            { text: data, timestamp: Date.now() },
          ]);

          if (
            data.includes("Swarm Ignition Sequence Complete") ||
            data.includes("Swarm is already active")
          ) {
            setTimeout(() => {
              setFadeOut(true);
              setTimeout(onComplete, 600);
            }, 800);
          }
        });
      } catch {
        // Not in Tauri context — use WebSocket fallback
        const handleWS = (event: MessageEvent) => {
          const data = typeof event.data === "string" ? event.data : "";
          if (!data) return;
          setStatus(data);
          setHistory((prev) => [
            ...prev.slice(-10),
            { text: data, timestamp: Date.now() },
          ]);
          if (
            data.includes("Swarm Ignition Sequence Complete") ||
            data.includes("Swarm is already active")
          ) {
            setTimeout(() => {
              setFadeOut(true);
              setTimeout(onComplete, 600);
            }, 800);
          }
        };
        window.addEventListener("message", handleWS);
        unlisten = () => window.removeEventListener("message", handleWS);
      }
    };

    setupListener();

    return () => {
      clearTimeout(timeout);
      if (unlisten) unlisten();
    };
  }, [onComplete]);

  return (
    <div className={`${styles.splash} ${fadeOut ? styles.fadeOut : ""}`}>
      <div className={styles.content}>
        <img
          src="/img/savant.png"
          alt="Savant"
          className={styles.logo}
          width={120}
          height={120}
        />
        <h1 className={styles.title}>SAVANT</h1>
        <p className={styles.subtitle}>One Mind. A Thousand Faces.</p>

        <div className={styles.spinner} />

        <div className={styles.status}>
          <span className={styles.statusDot} />
          {status}
        </div>

        {history.length > 1 && (
          <div className={styles.history}>
            {history.slice(-5).map((msg, i) => (
              <div key={i} className={styles.historyItem}>
                {msg.text}
              </div>
            ))}
          </div>
        )}

        <button onClick={onComplete} className={styles.skipBtn}>
          Skip to Dashboard
        </button>
      </div>
    </div>
  );
}
