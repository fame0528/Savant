"use client";

import { useState, useCallback, useEffect } from "react";
import {
  showBrowser,
  hideBrowser,
  browserNavigate,
  browserGoBack,
  browserGoForward,
  browserReload,
  browserGetTabs,
} from "@/lib/browser";

export default function BrowserPanel() {
  const [url, setUrl] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState("");
  const [browserVisible, setBrowserVisible] = useState(false);

  const navigate = useCallback(async () => {
    if (!url.trim()) return;
    setLoading(true);
    setStatus("Navigating...");
    try {
      const result = await browserNavigate(url.trim());
      setCurrentUrl(url.trim());
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
    setLoading(false);
  }, [url]);

  const goBack = async () => {
    try {
      const result = await browserGoBack();
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  };

  const goForward = async () => {
    try {
      const result = await browserGoForward();
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  };

  const reload = async () => {
    setLoading(true);
    try {
      const result = await browserReload();
      setStatus(result);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
    setLoading(false);
  };

  const toggleVisibility = async () => {
    try {
      if (browserVisible) {
        await hideBrowser();
        setStatus("Browser hidden");
      } else {
        await showBrowser();
        setStatus("Browser visible");
      }
      setBrowserVisible(!browserVisible);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  };

  return (
    <div style={{
      padding: "24px",
      color: "#e0e0e0",
      fontFamily: "monospace",
      maxWidth: "900px",
    }}>
      <div style={{
        fontSize: "14px",
        fontWeight: 800,
        color: "var(--accent, #00d5ff)",
        marginBottom: "24px",
        letterSpacing: "2px",
      }}>
        AGENT BROWSER
      </div>

      <div style={{
        display: "flex",
        gap: "8px",
        marginBottom: "16px",
        alignItems: "center",
      }}>
        <button
          onClick={toggleVisibility}
          style={{
            padding: "8px 16px",
            background: browserVisible ? "#ff4444" : "var(--accent, #00d5ff)",
            color: "#000",
            border: "none",
            borderRadius: "8px",
            fontWeight: 700,
            cursor: "pointer",
            fontSize: "12px",
          }}
        >
          {browserVisible ? "HIDE" : "SHOW"} BROWSER
        </button>

        <button
          onClick={goBack}
          style={navButtonStyle}
          title="Go back"
        >
          ←
        </button>
        <button
          onClick={goForward}
          style={navButtonStyle}
          title="Go forward"
        >
          →
        </button>
        <button
          onClick={reload}
          style={navButtonStyle}
          title="Reload"
        >
          ↻
        </button>
      </div>

      <div style={{
        display: "flex",
        gap: "8px",
        marginBottom: "16px",
      }}>
        <input
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && navigate()}
          placeholder="Enter URL (e.g., https://google.com)"
          style={{
            flex: 1,
            padding: "10px 16px",
            background: "rgba(255,255,255,0.05)",
            border: "1px solid rgba(255,255,255,0.1)",
            borderRadius: "8px",
            color: "#e0e0e0",
            fontFamily: "monospace",
            fontSize: "13px",
            outline: "none",
          }}
        />
        <button
          onClick={navigate}
          disabled={loading}
          style={{
            padding: "10px 24px",
            background: loading ? "#444" : "var(--accent, #00d5ff)",
            color: "#000",
            border: "none",
            borderRadius: "8px",
            fontWeight: 700,
            cursor: loading ? "not-allowed" : "pointer",
            fontSize: "13px",
          }}
        >
          {loading ? "..." : "GO"}
        </button>
      </div>

      {status && (
        <div style={{
          fontSize: "11px",
          color: status.startsWith("Error") ? "#ff4444" : "rgba(255,255,255,0.5)",
          marginBottom: "16px",
          fontFamily: "monospace",
          wordBreak: "break-all",
        }}>
          {status}
        </div>
      )}

      <div style={{
        padding: "24px",
        background: "rgba(255,255,255,0.03)",
        borderRadius: "12px",
        border: "1px solid rgba(255,255,255,0.06)",
        fontSize: "12px",
        color: "rgba(255,255,255,0.4)",
        lineHeight: "1.6",
      }}>
        <div style={{ marginBottom: "12px", fontWeight: 700, color: "rgba(255,255,255,0.6)" }}>
          Browser Control Panel
        </div>
        <ul style={{ paddingLeft: "16px", margin: 0 }}>
          <li>Click <strong>SHOW BROWSER</strong> to open the browser window on monitor 3</li>
          <li>Enter a URL and press <strong>GO</strong> to navigate the browser</li>
          <li>Use ← → ↻ buttons for navigation controls</li>
          <li>The agent shares the same browser window and can control it via tool calls</li>
          <li>Browser appears at x=5120 (third monitor) by default</li>
        </ul>
      </div>
    </div>
  );
}

const navButtonStyle: React.CSSProperties = {
  padding: "8px 12px",
  background: "rgba(255,255,255,0.08)",
  color: "#e0e0e0",
  border: "1px solid rgba(255,255,255,0.1)",
  borderRadius: "8px",
  cursor: "pointer",
  fontSize: "16px",
};
