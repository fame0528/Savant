"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import Link from "next/link";
import styles from "./marketplace.module.css";

interface Skill {
  name: string;
  description: string;
  source: string;
  installed: boolean;
}

export default function MarketplacePage() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("Connecting...");
  const [installing, setInstalling] = useState<string | null>(null);
  const socketRef = useRef<WebSocket | null>(null);

  const getWsUrl = () => {
    if (typeof window !== "undefined") {
      const envUrl = process.env.NEXT_PUBLIC_WS_URL;
      if (envUrl) return envUrl;
      const host = window.location.hostname || "127.0.0.1";
      const port = process.env.NEXT_PUBLIC_GATEWAY_PORT || "8080";
      return `ws://${host}:${port}/ws`;
    }
    return "";
  };

  const wsUrl = getWsUrl();

  const sendControlFrame = useCallback(
    (type: string, data: Record<string, unknown>) => {
      if (socketRef.current?.readyState === WebSocket.OPEN) {
        socketRef.current.send(
          JSON.stringify({
            session_id: "dashboard-session",
            payload: { type, ...data },
          })
        );
      }
    },
    []
  );

  useEffect(() => {
    if (!wsUrl) return;

    const ws = new WebSocket(wsUrl);
    ws.onopen = () => {
      setStatus("Connected. Loading skills...");
      sendControlFrame("SkillsList", {});
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const eventType = data.event || data.type;

        if (eventType === "SKILLS_LIST_RESULT") {
          setSkills(data.data?.skills || []);
          setStatus("");
        } else if (eventType === "SKILL_INSTALL_RESULT") {
          setInstalling(null);
          sendControlFrame("SkillsList", {});
        }
      } catch {
        // Ignore
      }
    };

    ws.onclose = () => {
      setStatus("Disconnected. Reconnecting...");
      setTimeout(() => {
        socketRef.current = new WebSocket(wsUrl);
      }, 3000);
    };

    socketRef.current = ws;
    return () => ws.close();
  }, [wsUrl, sendControlFrame]);

  const installSkill = (slug: string) => {
    setInstalling(slug);
    sendControlFrame("SkillInstall", { source: slug });
  };

  const filtered = skills.filter(
    (s) =>
      s.name.toLowerCase().includes(search.toLowerCase()) ||
      s.description.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Link href="/" className={styles.backLink}>
          ← Back to Dashboard
        </Link>
        <h1 className={styles.title}>Skill Marketplace</h1>
      </div>

      <input
        className={styles.search}
        placeholder="Search skills..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />

      {status && <div className={styles.status}>{status}</div>}

      <div className={styles.grid}>
        {filtered.map((skill) => (
          <div key={skill.name} className={styles.card}>
            <div className={styles.cardHeader}>
              <h3 className={styles.cardName}>{skill.name}</h3>
              {skill.installed && (
                <span className={styles.installed}>INSTALLED</span>
              )}
            </div>
            <p className={styles.cardDesc}>{skill.description}</p>
            <div className={styles.cardFooter}>
              <span className={styles.cardSource}>{skill.source}</span>
              {!skill.installed && (
                <button
                  className={styles.installBtn}
                  onClick={() => installSkill(skill.name)}
                  disabled={installing === skill.name}
                >
                  {installing === skill.name ? "INSTALLING..." : "INSTALL"}
                </button>
              )}
            </div>
          </div>
        ))}

        {filtered.length === 0 && !status && (
          <div className={styles.empty}>
            No skills found. Skills can be installed from ClawHub or placed
            directly in the <code>skills/</code> directory.
          </div>
        )}
      </div>
    </div>
  );
}
