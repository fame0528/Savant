"use client";

import { useState, useEffect } from "react";
import { isTauri } from "@/lib/tauri";
import { logger } from "@/lib/logger";

const getGatewayUrl = () => {
  if (isTauri()) return process.env.NEXT_PUBLIC_GATEWAY_URL || "http://localhost:8080";
  if (typeof window !== "undefined") {
    const envUrl = process.env.NEXT_PUBLIC_GATEWAY_URL;
    if (envUrl) return envUrl;
    const host = window.location.hostname || "127.0.0.1";
    const port = process.env.NEXT_PUBLIC_GATEWAY_PORT || "8080";
    return `http://${host}:${port}`;
  }
  return "http://localhost:8080";
};

interface McpServer {
  name: string;
  url: string;
  has_auth: boolean;
}

interface ServersResponse {
  servers: McpServer[];
  count: number;
}

export default function McpPage() {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionStatus, setActionStatus] = useState<string | null>(null);

  // Add server form
  const [addName, setAddName] = useState("");
  const [addUrl, setAddUrl] = useState("");
  const [addToken, setAddToken] = useState("");

  // Install from Smithery form
  const [smitheryName, setSmitheryName] = useState("");

  useEffect(() => {
    loadServers();
  }, []);

  const loadServers = async () => {
    setLoading(true);
    setError(null);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/mcp/servers`);
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const data: ServersResponse = await resp.json();
      setServers(data.servers);
    } catch (e: any) {
      setError(`Failed to load MCP servers: ${e.message}`);
    } finally {
      setLoading(false);
    }
  };

  const handleAddServer = async () => {
    if (!addName || !addUrl) {
      setError("Name and URL are required");
      return;
    }
    setActionStatus("Adding server...");
    setError(null);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/mcp/servers/add`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name: addName,
          url: addUrl,
          auth_token: addToken || undefined,
        }),
      });
      if (!resp.ok) {
        const data = await resp.json();
        throw new Error(data.error || `HTTP ${resp.status}`);
      }
      setAddName("");
      setAddUrl("");
      setAddToken("");
      setActionStatus("Server added!");
      setTimeout(() => setActionStatus(null), 2000);
      await loadServers();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setActionStatus(null);
    }
  };

  const handleInstallSmithery = async () => {
    if (!smitheryName) {
      setError("Server name is required");
      return;
    }
    setActionStatus(`Installing ${smitheryName}...`);
    setError(null);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/mcp/servers/install`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ server_name: smitheryName }),
      });
      if (!resp.ok) {
        const data = await resp.json();
        throw new Error(data.error || `HTTP ${resp.status}`);
      }
      setSmitheryName("");
      setActionStatus("Installed!");
      setTimeout(() => setActionStatus(null), 2000);
      await loadServers();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setActionStatus(null);
    }
  };

  const handleRemove = async (name: string) => {
    setActionStatus(`Removing ${name}...`);
    setError(null);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/mcp/servers/remove`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      });
      if (!resp.ok) {
        const data = await resp.json();
        throw new Error(data.error || `HTTP ${resp.status}`);
      }
      setActionStatus("Removed!");
      setTimeout(() => setActionStatus(null), 2000);
      await loadServers();
    } catch (e: any) {
      setError(e.message);
    }
  };

  const handleUninstall = async (serverName: string) => {
    setActionStatus(`Uninstalling ${serverName}...`);
    setError(null);
    try {
      const gatewayUrl = getGatewayUrl();
      const resp = await fetch(`${gatewayUrl}/api/mcp/servers/uninstall`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ server_name: serverName }),
      });
      if (!resp.ok) {
        const data = await resp.json();
        throw new Error(data.error || `HTTP ${resp.status}`);
      }
      setActionStatus("Uninstalled!");
      setTimeout(() => setActionStatus(null), 2000);
      await loadServers();
    } catch (e: any) {
      setError(e.message);
    }
  };

  return (
    <div style={{ padding: "2rem", maxWidth: "800px", margin: "0 auto" }}>
      <h1 style={{ fontSize: "1.5rem", fontWeight: "bold", marginBottom: "1.5rem" }}>
        MCP Server Management
      </h1>

      {error && (
        <div style={{
          padding: "0.75rem",
          marginBottom: "1rem",
          backgroundColor: "#4a1c1c",
          borderRadius: "6px",
          color: "#f87171",
          fontSize: "0.875rem",
        }}>
          {error}
        </div>
      )}

      {actionStatus && (
        <div style={{
          padding: "0.75rem",
          marginBottom: "1rem",
          backgroundColor: "#1c3a4a",
          borderRadius: "6px",
          color: "#60a5fa",
          fontSize: "0.875rem",
        }}>
          {actionStatus}
        </div>
      )}

      {/* Install from Smithery */}
      <div style={{
        padding: "1.25rem",
        marginBottom: "1.5rem",
        backgroundColor: "#1a1a2e",
        borderRadius: "8px",
        border: "1px solid #2a2a4e",
      }}>
        <h2 style={{ fontSize: "1rem", fontWeight: "600", marginBottom: "0.75rem" }}>
          Install from Smithery Marketplace
        </h2>
        <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
          <input
            type="text"
            value={smitheryName}
            onChange={(e) => setSmitheryName(e.target.value)}
            placeholder="@anthropic/mcp-server-filesystem"
            style={{
              flex: 1,
              padding: "0.5rem 0.75rem",
              backgroundColor: "#0d0d1a",
              border: "1px solid #3a3a5e",
              borderRadius: "4px",
              color: "#e0e0e0",
              fontSize: "0.875rem",
            }}
          />
          <button
            onClick={handleInstallSmithery}
            disabled={!smitheryName}
            style={{
              padding: "0.5rem 1rem",
              backgroundColor: smitheryName ? "#3b82f6" : "#374151",
              color: "white",
              border: "none",
              borderRadius: "4px",
              cursor: smitheryName ? "pointer" : "not-allowed",
              fontSize: "0.875rem",
              fontWeight: "500",
            }}
          >
            Install
          </button>
        </div>
      </div>

      {/* Add Custom Server */}
      <div style={{
        padding: "1.25rem",
        marginBottom: "1.5rem",
        backgroundColor: "#1a1a2e",
        borderRadius: "8px",
        border: "1px solid #2a2a4e",
      }}>
        <h2 style={{ fontSize: "1rem", fontWeight: "600", marginBottom: "0.75rem" }}>
          Add Custom MCP Server
        </h2>
        <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
          <div style={{ display: "flex", gap: "0.75rem" }}>
            <input
              type="text"
              value={addName}
              onChange={(e) => setAddName(e.target.value)}
              placeholder="Server name (e.g., filesystem)"
              style={{
                flex: 1,
                padding: "0.5rem 0.75rem",
                backgroundColor: "#0d0d1a",
                border: "1px solid #3a3a5e",
                borderRadius: "4px",
                color: "#e0e0e0",
                fontSize: "0.875rem",
              }}
            />
            <input
              type="text"
              value={addUrl}
              onChange={(e) => setAddUrl(e.target.value)}
              placeholder="ws://localhost:3001/mcp"
              style={{
                flex: 1,
                padding: "0.5rem 0.75rem",
                backgroundColor: "#0d0d1a",
                border: "1px solid #3a3a5e",
                borderRadius: "4px",
                color: "#e0e0e0",
                fontSize: "0.875rem",
              }}
            />
          </div>
          <div style={{ display: "flex", gap: "0.75rem", alignItems: "center" }}>
            <input
              type="password"
              value={addToken}
              onChange={(e) => setAddToken(e.target.value)}
              placeholder="Auth token (optional)"
              style={{
                flex: 1,
                padding: "0.5rem 0.75rem",
                backgroundColor: "#0d0d1a",
                border: "1px solid #3a3a5e",
                borderRadius: "4px",
                color: "#e0e0e0",
                fontSize: "0.875rem",
              }}
            />
            <button
              onClick={handleAddServer}
              disabled={!addName || !addUrl}
              style={{
                padding: "0.5rem 1rem",
                backgroundColor: addName && addUrl ? "#10b981" : "#374151",
                color: "white",
                border: "none",
                borderRadius: "4px",
                cursor: addName && addUrl ? "pointer" : "not-allowed",
                fontSize: "0.875rem",
                fontWeight: "500",
              }}
            >
              Add Server
            </button>
          </div>
        </div>
      </div>

      {/* Server List */}
      <div style={{
        padding: "1.25rem",
        backgroundColor: "#1a1a2e",
        borderRadius: "8px",
        border: "1px solid #2a2a4e",
      }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.75rem" }}>
          <h2 style={{ fontSize: "1rem", fontWeight: "600" }}>
            Configured Servers ({servers.length})
          </h2>
          <button
            onClick={loadServers}
            style={{
              padding: "0.25rem 0.75rem",
              backgroundColor: "#374151",
              color: "#9ca3af",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
              fontSize: "0.75rem",
            }}
          >
            Refresh
          </button>
        </div>

        {loading ? (
          <p style={{ color: "#6b7280", fontSize: "0.875rem" }}>Loading...</p>
        ) : servers.length === 0 ? (
          <p style={{ color: "#6b7280", fontSize: "0.875rem" }}>
            No MCP servers configured. Add one above or install from Smithery.
          </p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {servers.map((server) => (
              <div
                key={server.name}
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  padding: "0.75rem",
                  backgroundColor: "#0d0d1a",
                  borderRadius: "6px",
                  border: "1px solid #2a2a4e",
                }}
              >
                <div>
                  <div style={{ fontWeight: "500", fontSize: "0.875rem", color: "#e0e0e0" }}>
                    {server.name}
                  </div>
                  <div style={{ fontSize: "0.75rem", color: "#6b7280", marginTop: "0.25rem" }}>
                    {server.url}
                    {server.has_auth && (
                      <span style={{ marginLeft: "0.5rem", color: "#f59e0b" }}>
                        auth
                      </span>
                    )}
                  </div>
                </div>
                <div style={{ display: "flex", gap: "0.5rem" }}>
                  <button
                    onClick={() => handleRemove(server.name)}
                    style={{
                      padding: "0.25rem 0.5rem",
                      backgroundColor: "#7c2d12",
                      color: "#fca5a5",
                      border: "none",
                      borderRadius: "4px",
                      cursor: "pointer",
                      fontSize: "0.75rem",
                    }}
                  >
                    Remove
                  </button>
                  <button
                    onClick={() => handleUninstall(server.name)}
                    style={{
                      padding: "0.25rem 0.5rem",
                      backgroundColor: "#7f1d1d",
                      color: "#fca5a5",
                      border: "none",
                      borderRadius: "4px",
                      cursor: "pointer",
                      fontSize: "0.75rem",
                    }}
                  >
                    Uninstall
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <p style={{ marginTop: "1rem", color: "#6b7280", fontSize: "0.75rem" }}>
        MCP servers are configured in savant.toml under [mcp]. Changes take effect on next agent restart.
      </p>
    </div>
  );
}
