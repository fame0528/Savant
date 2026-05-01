"use client";

import { useState, useEffect, useCallback } from "react";
import { useDashboard } from "@/context/DashboardContext";
import styles from "./marketplace.module.css";

interface Skill {
  name: string;
  description: string;
  source: string;
  installed: boolean;
}

const getGatewayUrl = () => {
  if (typeof window !== "undefined") {
    const host = window.location.hostname || "127.0.0.1";
    const port = process.env.NEXT_PUBLIC_GATEWAY_PORT || "8080";
    return `http://${host}:${port}`;
  }
  return "http://127.0.0.1:8080";
};

export default function MarketplacePage() {
  const ctx = useDashboard();
  const [skills, setSkills] = useState<Skill[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [unavailable, setUnavailable] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);

  const loadSkills = useCallback(async () => {
    try {
      setLoading(true);
      setUnavailable(false);
      const resp = await fetch(`${getGatewayUrl()}/api/skills`);
      if (resp.status === 404) {
        setUnavailable(true);
        setSkills([]);
        return;
      }
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data = await resp.json();
      const skillList = Array.isArray(data) ? data : (data.skills || []);
      setSkills(skillList.map((s: Record<string, unknown>) => ({
        name: (s.name as string) || (s.slug as string) || "Unknown",
        description: (s.description as string) || "",
        source: (s.source as string) || "local",
        installed: (s.installed as boolean) === true || (s.status as string) === "installed",
      })));
    } catch {
      setUnavailable(true);
      setSkills([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSkills();
  }, [loadSkills]);

  const installSkill = async (slug: string) => {
    setInstalling(slug);
    try {
      const resp = await fetch(`${getGatewayUrl()}/api/skills/install`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source: slug }),
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      await loadSkills();
    } catch {
      // Silently fail — marketplace is coming soon
    } finally {
      setInstalling(null);
    }
  };

  const filtered = skills.filter(
    (s) =>
      s.name.toLowerCase().includes(search.toLowerCase()) ||
      s.description.toLowerCase().includes(search.toLowerCase())
  );

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <h1 className={styles.title}>Skill Marketplace</h1>
        <span style={{ fontSize: "10px", color: ctx.connectionStatus === "NOMINAL" ? "#00ff88" : "#ff4444" }}>
          {ctx.connectionStatus === "NOMINAL" ? "● CONNECTED" : "● DISCONNECTED"}
        </span>
      </div>

      <input
        className={styles.search}
        placeholder="Search skills..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />

      {loading && <div className={styles.status}>Loading skills...</div>}

      {unavailable && (
        <div className={styles.empty}>
          <div style={{ fontSize: "24px", marginBottom: "12px" }}>🏪</div>
          <div style={{ fontSize: "14px", fontWeight: 900, letterSpacing: "1px", marginBottom: "8px" }}>MARKETPLACE COMING SOON</div>
          <div style={{ fontSize: "12px", opacity: 0.6, lineHeight: "1.6" }}>
            The skill marketplace is under development. Skills can be installed manually by placing them in the <code>skills/</code> directory.
          </div>
        </div>
      )}

      {!unavailable && !loading && (
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

          {filtered.length === 0 && (
            <div className={styles.empty}>
              No skills found. Skills can be installed from ClawHub or placed
              directly in the <code>skills/</code> directory.
            </div>
          )}
        </div>
      )}
    </div>
  );
}
