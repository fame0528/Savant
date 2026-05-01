"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { useDashboard } from "@/context/DashboardContext";
import styles from "./health.module.css";

interface AgentHealth {
  id: string;
  name: string;
  status: string;
  memory: string;
  lastActive: string;
}

export default function HealthPage() {
  const ctx = useDashboard();
  const [agents, setAgents] = useState<AgentHealth[]>([]);
  const [gatewayLatency, setGatewayLatency] = useState<number | null>(null);

  // Use shared WebSocket connection status from DashboardContext
  const connected = ctx.connectionStatus === 'NOMINAL';

  // Fetch agent health via HTTP API as fallback/primary
  useEffect(() => {
    const fetchHealth = async () => {
      try {
        const start = Date.now();
        const resp = await fetch(`http://${typeof window !== 'undefined' ? window.location.hostname : '127.0.0.1'}:${process.env.NEXT_PUBLIC_GATEWAY_PORT || 8080}/api/agents`);
        setGatewayLatency(Date.now() - start);
        if (resp.ok) {
          const data = await resp.json();
          const agentList = Array.isArray(data) ? data : (data.agents || []);
          setAgents(agentList.map((a: Record<string, any>) => ({
            id: a.id || a.agent_id || "",
            name: a.name || a.agent_name || "Unknown",
            status: a.status || "Unknown",
            memory: a.memory || "—",
            lastActive: a.lastActive || a.last_active || "—",
          })));
        }
      } catch {
        // Gateway may not have /api/agents endpoint — agents will show as empty
      }
    };

    fetchHealth();
    const interval = setInterval(fetchHealth, 5000);
    return () => clearInterval(interval);
  }, []);

  // Also listen for agent.discovered events from shared WebSocket
  useEffect(() => {
    if (ctx.agents.length > 0) {
      setAgents(ctx.agents.map((a: any) => ({
        id: a.id || "",
        name: a.name || "Unknown",
        status: "Active",
        memory: "—",
        lastActive: "—",
      })));
    }
  }, [ctx.agents]);

  const metrics = [
    { label: "Gateway WebSocket", value: connected ? "Connected" : "Disconnected", status: connected ? "ok" as const : "error" as const },
    { label: "Gateway Latency", value: gatewayLatency !== null ? `${gatewayLatency}ms` : "—", status: gatewayLatency !== null && gatewayLatency < 100 ? "ok" as const : gatewayLatency !== null ? "warn" as const : "error" as const },
    { label: "Gateway HTTP", value: `Port ${process.env.NEXT_PUBLIC_GATEWAY_PORT || 8080}`, status: "ok" as const },
    { label: "Dashboard", value: "Running", status: "ok" as const, detail: "Next.js 16" },
    { label: "Protocol", value: "WebSocket", status: "ok" as const, detail: "Savant ControlFrame" },
    { label: "Key Management", value: "OpenRouter", status: "ok" as const, detail: "Free models only" },
  ];

  return (
    <div className={styles.container}>
      <div className={styles.header}>
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
