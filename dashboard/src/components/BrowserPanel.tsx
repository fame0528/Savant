"use client";

import React, { useState, useEffect, useCallback } from "react";
import {
  showBrowser,
  hideBrowser,
  browserGetTabs,
  browserNavigate,
  browserGoBack,
  browserGoForward,
  browserReload,
  browserNewTab,
  browserCloseTab,
  browserSwitchTab,
  BrowserTabInfo,
} from "../lib/browser";
import { useDashboard } from "../context/DashboardContext";

export default function BrowserPanel() {
  const ctx = useDashboard();
  const [tabs, setTabs] = useState<BrowserTabInfo[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [urlInput, setUrlInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const [isWindowVisible, setIsWindowVisible] = useState(false);

  // Poll tab state every 2 seconds.
  const pollTabs = useCallback(async () => {
    try {
      const result = await browserGetTabs();
      setTabs(result);
      if (result.length > 0) {
        if (!activeTabId || !result.some((t) => t.id === activeTabId)) {
          setActiveTabId(result[0].id);
          setUrlInput(result[0].url);
        }
        const anyLoading = result.some((t) => t.loading);
        setIsLoading(anyLoading);
      }
    } catch {
      // Browser engine may not be initialized.
    }
  }, [activeTabId]);

  useEffect(() => {
    pollTabs();
    const interval = setInterval(pollTabs, 2000);
    return () => clearInterval(interval);
  }, [pollTabs]);

  const handleNavigate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!urlInput.trim()) return;
    try {
      const url = urlInput.trim();
      const result = await browserNavigate(url);
      setStatusMsg(result);
      setIsLoading(true);
      setTimeout(() => pollTabs(), 500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Navigation failed: ${msg}`);
    }
  };

  const handleGoBack = async () => {
    try {
      await browserGoBack();
      setStatusMsg("Going back");
      setTimeout(() => pollTabs(), 500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Back failed: ${msg}`);
    }
  };

  const handleGoForward = async () => {
    try {
      await browserGoForward();
      setStatusMsg("Going forward");
      setTimeout(() => pollTabs(), 500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Forward failed: ${msg}`);
    }
  };

  const handleReload = async () => {
    try {
      await browserReload();
      setIsLoading(true);
      setStatusMsg("Reloading");
      setTimeout(() => pollTabs(), 500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Reload failed: ${msg}`);
    }
  };

  const handleNewTab = async () => {
    try {
      const result = await browserNewTab("about:blank");
      setStatusMsg(result);
      setTimeout(() => pollTabs(), 500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`New tab failed: ${msg}`);
    }
  };

  const handleCloseTab = async (tabId: string) => {
    try {
      await browserCloseTab(tabId);
      if (activeTabId === tabId) {
        setActiveTabId(null);
        setUrlInput("");
      }
      setTimeout(() => pollTabs(), 500);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Close tab failed: ${msg}`);
    }
  };

  const handleSwitchTab = async (tabId: string) => {
    try {
      await browserSwitchTab(tabId);
      setActiveTabId(tabId);
      const tab = tabs.find((t) => t.id === tabId);
      if (tab) setUrlInput(tab.url);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Switch tab failed: ${msg}`);
    }
  };

  const handleShowBrowser = async () => {
    try {
      await showBrowser();
      setIsWindowVisible(true);
      setStatusMsg("Browser window shown");
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Show browser failed: ${msg}`);
    }
  };

  const handleHideBrowser = async () => {
    try {
      await hideBrowser();
      setIsWindowVisible(false);
      setStatusMsg("Browser window hidden");
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setStatusMsg(`Hide browser failed: ${msg}`);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "12px", width: "100%" }}>
      {/* Window controls */}
      <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
        <button
          onClick={handleShowBrowser}
          style={{
            background: isWindowVisible ? "var(--panel-bg)" : "var(--accent-dim)",
            color: "var(--accent)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            padding: "4px 12px",
            fontSize: "12px",
            cursor: "pointer",
            fontFamily: "inherit",
          }}
          title="Show browser window on monitor 3"
        >
          &#x1f5a5; Show Window
        </button>
        <button
          onClick={handleHideBrowser}
          style={{
            background: "var(--panel-bg)",
            color: "var(--text-muted)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            padding: "4px 12px",
            fontSize: "12px",
            cursor: "pointer",
            fontFamily: "inherit",
          }}
          title="Hide browser window"
        >
          &#x1f5d6; Hide Window
        </button>
        {tabs.length > 0 && (
          <span
            style={{
              marginLeft: "auto",
              fontSize: "11px",
              color: "var(--text-muted)",
            }}
          >
            {tabs.length} tab{tabs.length !== 1 ? "s" : ""} open
          </span>
        )}
      </div>

      {/* Tab bar */}
      {tabs.length > 0 && (
        <div
          style={{
            display: "flex",
            gap: "4px",
            overflowX: "auto",
            paddingBottom: "4px",
            borderBottom: "1px solid var(--border)",
          }}
        >
          {tabs.map((tab) => (
            <div
              key={tab.id}
              role="button"
              tabIndex={0}
              onClick={() => handleSwitchTab(tab.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleSwitchTab(tab.id);
                }
              }}
              style={{
                display: "flex",
                alignItems: "center",
                gap: "6px",
                padding: "6px 12px",
                background: tab.id === activeTabId ? "var(--accent-dim)" : "var(--panel-bg)",
                border: `1px solid ${tab.id === activeTabId ? "var(--accent)" : "var(--border)"}`,
                borderRadius: "4px 4px 0 0",
                fontSize: "11px",
                cursor: "pointer",
                maxWidth: "200px",
                minWidth: "80px",
              }}
              title={tab.url}
            >
              {tab.loading && (
                <span style={{ color: "var(--accent)", animation: "spin 1s linear infinite" }}>
                  &#x27f3;
                </span>
              )}
              <span
                style={{
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                  flex: 1,
                  color: tab.id === activeTabId ? "var(--accent)" : "var(--text-muted)",
                }}
              >
                {tab.title || new URL(tab.url).hostname || tab.url}
              </span>
              {tab.agent_name && (
                <span
                  style={{
                    fontSize: "9px",
                    background: "var(--agent-color)",
                    color: "#000",
                    padding: "1px 4px",
                    borderRadius: "3px",
                    fontWeight: 700,
                  }}
                  title={`Opened by agent: ${tab.agent_name}`}
                >
                  A
                </span>
              )}
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleCloseTab(tab.id);
                }}
                style={{
                  background: "transparent",
                  border: "none",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                  fontSize: "14px",
                  padding: "0 2px",
                  lineHeight: 1,
                }}
                title="Close tab"
              >
                &#x2715;
              </button>
            </div>
          ))}
          <button
            onClick={handleNewTab}
            style={{
              background: "var(--panel-bg)",
              border: "1px solid var(--border)",
              borderRadius: "4px 4px 0 0",
              color: "var(--accent)",
              fontSize: "16px",
              padding: "4px 10px",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
            title="New tab"
          >
            &#x2b;
          </button>
        </div>
      )}

      {/* Address bar + navigation buttons */}
      <form onSubmit={handleNavigate} style={{ display: "flex", gap: "6px", alignItems: "center" }}>
        <button
          type="button"
          onClick={handleGoBack}
          style={{
            background: "var(--panel-bg)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            color: "var(--text-muted)",
            padding: "6px 10px",
            fontSize: "13px",
            cursor: "pointer",
          }}
          title="Go back"
        >
          &#x2190;
        </button>
        <button
          type="button"
          onClick={handleGoForward}
          style={{
            background: "var(--panel-bg)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            color: "var(--text-muted)",
            padding: "6px 10px",
            fontSize: "13px",
            cursor: "pointer",
          }}
          title="Go forward"
        >
          &#x2192;
        </button>
        <button
          type="button"
          onClick={handleReload}
          style={{
            background: "var(--panel-bg)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            color: "var(--text-muted)",
            padding: "6px 10px",
            fontSize: "13px",
            cursor: "pointer",
          }}
          title="Reload"
        >
          &#x21bb;
        </button>
        <input
          type="text"
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          placeholder="Enter URL and press Enter..."
          style={{
            flex: 1,
            background: "var(--panel-bg)",
            border: "1px solid var(--border)",
            borderRadius: "4px",
            color: "var(--text)",
            padding: "6px 12px",
            fontSize: "12px",
            fontFamily: "'JetBrains Mono', monospace",
            outline: "none",
          }}
        />
        <button
          type="submit"
          style={{
            background: "var(--accent-dim)",
            border: "1px solid var(--accent)",
            borderRadius: "4px",
            color: "var(--accent)",
            padding: "6px 16px",
            fontSize: "12px",
            cursor: "pointer",
            fontWeight: 600,
          }}
        >
          Go
        </button>
      </form>

      {/* Loading indicator */}
      {isLoading && (
        <div
          style={{
            height: "3px",
            background: "var(--accent)",
            borderRadius: "2px",
            animation: "loading-pulse 1.5s ease-in-out infinite",
          }}
        />
      )}

      {/* Status message */}
      {statusMsg && (
        <div
          style={{
            fontSize: "11px",
            color: "var(--text-muted)",
            padding: "4px 8px",
            background: "var(--panel-bg)",
            borderRadius: "4px",
            border: "1px solid var(--border)",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {statusMsg}
        </div>
      )}

      {/* Tab details */}
      {activeTabId && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: "4px",
            padding: "8px",
            background: "var(--panel-bg)",
            borderRadius: "4px",
            border: "1px solid var(--border)",
          }}
        >
          <div style={{ fontSize: "11px", color: "var(--text-muted)" }}>
            <span style={{ color: "var(--accent)", fontWeight: 600 }}>Active Tab:</span>{" "}
            {tabs.find((t) => t.id === activeTabId)?.title || "(no title)"}
          </div>
          <div style={{ fontSize: "10px", color: "var(--text-muted)", wordBreak: "break-all" }}>
            {tabs.find((t) => t.id === activeTabId)?.url || ""}
          </div>
        </div>
      )}

      {/* Empty state */}
      {tabs.length === 0 && (
        <div
          style={{
            textAlign: "center",
            padding: "20px",
            color: "var(--text-muted)",
            fontSize: "12px",
            border: "1px dashed var(--border)",
            borderRadius: "4px",
          }}
        >
          No open tabs. Click &#x2b; to open a new tab, or ask an agent to browse for you.
        </div>
      )}

      <style>{`
        @keyframes loading-pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.4; }
        }
      `}</style>
    </div>
  );
}
