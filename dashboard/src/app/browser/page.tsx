"use client";

import React from "react";
import BrowserPanel from "../../components/BrowserPanel";

export default function BrowserPage() {
  return (
    <div style={{ padding: "16px", maxWidth: "800px" }}>
      <h2
        style={{
          fontSize: "14px",
          fontWeight: 700,
          color: "var(--accent)",
          marginBottom: "16px",
          letterSpacing: "1px",
        }}
      >
        &#x1f310; BROWSER CONTROL
      </h2>
      <p
        style={{
          fontSize: "12px",
          color: "var(--text-muted)",
          marginBottom: "16px",
          lineHeight: 1.6,
        }}
      >
        Control the browser window on monitor 3. Navigate to URLs, manage tabs,
        and watch agents browse the web. Use the address bar to navigate, or let
        an agent control the browser autonomously.
      </p>
      <BrowserPanel />
    </div>
  );
}
