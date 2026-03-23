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
    // Listen for status updates from the backend
    const handleLog = (event: MessageEvent) => {
      try {
        const data = typeof event.data === "string" ? event.data : "";
        if (!data) return;

        // Update current status
        setStatus(data);

        // Add to history
        setHistory((prev) => [
          ...prev.slice(-10), // Keep last 10
          { text: data, timestamp: Date.now() },
        ]);

        // Check for completion
        if (
          data.includes("Swarm Ignition Sequence Complete") ||
          data.includes("Swarm is already active")
        ) {
          // Dismiss splash after short delay
          setTimeout(() => {
            setFadeOut(true);
            setTimeout(onComplete, 600);
          }, 800);
        }
      } catch {
        // Ignore parse errors
      }
    };

    // Listen for system log events via WebSocket or event source
    if (typeof window !== "undefined") {
      window.addEventListener("message", handleLog);

      // Also listen for Tauri events if available
      const checkTauri = async () => {
        try {
          const { listen } = await import("@tauri-apps/api/event");
          await listen("system-log-event", (event: { payload: unknown }) => {
            handleLog({ data: String(event.payload) } as MessageEvent);
          });
        } catch {
          // Not in Tauri context — use WebSocket
        }
      };
      checkTauri();
    }

    return () => {
      if (typeof window !== "undefined") {
        window.removeEventListener("message", handleLog);
      }
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
      </div>
    </div>
  );
}
