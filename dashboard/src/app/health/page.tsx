"use client";

import { useState, useEffect, useRef } from "react";
import Link from "next/link";
import styles from "./health.module.css";

interface HealthMetric {
  label: string;
  value: string;
  status: "ok" | "warn" | "error";
  detail?: string;
}

interface AgentHealth {
  id: string;
  name: string;
  status: string;
  memory: string;
  lastActive: string;
}

export default function HealthPage() {
  const [metrics, setMetrics] = useState<HealthMetric[]>([]);
  const [agents, setAgents] = useState<AgentHealth[]>([]);
  const [connected, setConnected] = useState(false);
  const socketRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

  const healthMetrics: HealthMetric[] = [
    { label: "WebSocket", value: connected ? "Connected" : "Disconnected", status: connected ? "ok" : "error" },
    { label: "Dashboard", value: "Running", status: "ok", detail: "Next.js 16" },
    { label: "Protocol", value: "WebSocket", status: "ok", detail: "Savant ControlFrame" },
    { label: "Auto Key Mgmt", value: "OpenRouter", status: "ok", detail: "Free models only" },
  ];

  useEffect(() => {
    if (!wsUrl) return;

    let shouldReconnect = true;

    const connect = () => {
      if (!shouldReconnect) return;
      const ws = new WebSocket(wsUrl);
      socketRef.current = ws;

      ws.onopen = () => {
        setConnected(true);
        ws.send(JSON.stringify({
          request_id: "health-init",
          session_id: "dashboard-session",
          payload: { type: "SwarmMonitor", data: {} },
        }));
      };

      ws.onmessage = (event) => {
        try {
          const raw = event.data;
          let data: any;
          if (typeof raw === 'string' && raw.startsWith("EVENT:")) {
            data = JSON.parse(raw.substring(6));
          } else {
            data = JSON.parse(raw);
          }
          const payload = typeof data.payload === 'string' ? JSON.parse(data.payload) : data.payload;
          const eventType = data.event_type || data.event || data.type || payload?.type;

          if (eventType === "SWARM_STATUS") {
            const swarmData = payload?.data || data.data || data;
            const agentList = (swarmData.agents || []).map((a: Record<string, string>) => ({
              id: a.id || a.agent_id || "",
              name: a.name || a.agent_name || "Unknown",
              status: a.status || "Unknown",
              memory: a.memory || "—",
              lastActive: a.lastActive || a.last_active || "—",
            }));
            setAgents(agentList);
          }
        } catch {
          // Ignore parse errors
        }
      };

      ws.onclose = () => {
        setConnected(false);
        socketRef.current = null;
        if (shouldReconnect) {
          reconnectTimerRef.current = setTimeout(connect, 3000);
        }
      };

      ws.onerror = () => {
        setConnected(false);
      };
    };

    connect();

    setMetrics(healthMetrics);

    return () => {
      shouldReconnect = false;
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      if (socketRef.current) { socketRef.current.close(); socketRef.current = null; }
    };
  }, [wsUrl]);

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Link href="/" className={styles.backLink}>
          ← Back to Dashboard
        </Link>
        <h1 className={styles.title}>System Health</h1>
        <span
          className={styles.status}
          style={{ color: connected ? "#00ff88" : "#ff4444" }}
        >
          {connected ? "● ONLINE" : "● OFFLINE"}
        </span>
      </div>

      {/* System Metrics */}
      <div className={styles.grid}>
        {metrics.map((m, i) => (
          <div key={i} className={styles.card}>
            <div className={styles.cardLabel}>{m.label}</div>
            <div
              className={styles.cardValue}
              style={{
                color:
                  m.status === "ok"
                    ? "#00ff88"
                    : m.status === "warn"
                    ? "#ffaa00"
                    : "#ff4444",
              }}
            >
              {m.value}
            </div>
            {m.detail && (
              <div className={styles.cardDetail}>{m.detail}</div>
            )}
          </div>
        ))}
      </div>

      {/* Agent Status */}
      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Agent Status</h2>
        {agents.length === 0 ? (
          <div className={styles.empty}>
            No agents detected. Agents appear when the swarm is running.
          </div>
        ) : (
          <table className={styles.table}>
            <thead>
              <tr>
                <th>Name</th>
                <th>ID</th>
                <th>Status</th>
                <th>Memory</th>
                <th>Last Active</th>
              </tr>
            </thead>
            <tbody>
              {agents.map((a) => (
                <tr key={a.id}>
                  <td className={styles.name}>{a.name}</td>
                  <td className={styles.id}>{a.id}</td>
                  <td>
                    <span
                      className={styles.badge}
                      style={{
                        color:
                          a.status === "Active"
                            ? "#00ff88"
                            : a.status === "Idle"
                            ? "#ffaa00"
                            : "#ff4444",
                      }}
                    >
                      {a.status}
                    </span>
                  </td>
                  <td>{a.memory}</td>
                  <td>{a.lastActive}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {/* Quick Stats */}
      <div className={styles.grid}>
        <div className={styles.card}>
          <div className={styles.cardLabel}>Total Agents</div>
          <div className={styles.cardValue}>{agents.length}</div>
        </div>
        <div className={styles.card}>
          <div className={styles.cardLabel}>Active</div>
          <div className={styles.cardValue} style={{ color: "#00ff88" }}>
            {agents.filter((a) => a.status === "Active").length}
          </div>
        </div>
        <div className={styles.card}>
          <div className={styles.cardLabel}>Idle</div>
          <div className={styles.cardValue} style={{ color: "#ffaa00" }}>
            {agents.filter((a) => a.status === "Idle").length}
          </div>
        </div>
        <div className={styles.card}>
          <div className={styles.cardLabel}>Capacity</div>
          <div className={styles.cardValue}>∞</div>
        </div>
      </div>
    </div>
  );
}
