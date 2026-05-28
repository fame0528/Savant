import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * 🛰️ Savant Tauri Bridge
 * Unifies communication between the Next.js frontend and the Rust substrate.
 */
/**
 * Get the app version from Tauri (reads tauri.conf.json at runtime).
 * Falls back to /api/status endpoint in non-Tauri mode, then hardcoded fallback.
 */
export const getAppVersion = async (): Promise<string> => {
  if (isTauri()) {
    try {
      const { getVersion } = await import("@tauri-apps/api/app");
      return await getVersion();
    } catch {
      return "0.0.0";
    }
  }
  // Non-Tauri: try the gateway status endpoint
  try {
    const resp = await fetch("/api/status");
    if (resp.ok) {
      const data = await resp.json();
      return data.version || "0.0.0";
    }
  } catch {}
  return "0.0.0";
};

/**
 * Get the dashboard API key and gateway port from Tauri's ignite_swarm response.
 * In non-Tauri mode, returns defaults (empty key, port 8080).
 */
export const getDashboardConfig = async (): Promise<{ apiKey: string; port: number }> => {
  if (isTauri()) {
    try {
      const result = await invoke<{ dashboard_api_key: string; gateway_port: number }>("ignite_swarm");
      const apiKey = (result as any).dashboard_api_key || "";
      const port = (result as any).gateway_port || 8080;
      return { apiKey, port };
    } catch (e) {
      console.error("[tauri] getDashboardConfig invoke failed:", e);
      return { apiKey: "", port: 8080 };
    }
  }
  return { apiKey: "", port: 8080 };
};

/** @deprecated Use getDashboardConfig() instead */
export const getDashboardApiKey = async (): Promise<string> => {
  const config = await getDashboardConfig();
  return config.apiKey;
};

export const isTauri = (): boolean => {
  if (typeof window === "undefined") return false;
  return !!(window as any).__TAURI_INTERNALS__ || !!(window as any).__TAURI__;
};

export const igniteSwarm = async (): Promise<string> => {
  if (isTauri()) {
    const result = await invoke<{ status: string; dashboard_api_key: string }>("ignite_swarm");
    return result.status;
  }
  return "Not running under Tauri";
};

export const setupLogListener = async (onLog: (msg: string) => void) => {
  if (isTauri()) {
    return await listen<string>("log-event", (event: any) => {
      onLog(event.payload);
    });
  }
};

export const getStatus = async (): Promise<any> => {
  if (isTauri()) {
    return await invoke("get_swarm_status");
  }
  return { status: "EXTERNAL" };
};


// ─── Authenticated Fetch ──────────────────────────────────────────────
// Module-level API key cache. Set by DashboardContext on init.
let _dashboardApiKey = "";

/** Set the dashboard API key for authenticated fetches. Called by DashboardContext. */
export const setDashboardApiKey = (key: string) => { _dashboardApiKey = key; };

/** Get the current dashboard API key. */
export const getDashboardApiKeySync = () => _dashboardApiKey;

/**
 * Authenticated fetch wrapper. Automatically includes Authorization header
 * for all /api/* endpoints (except public ones like /api/setup/ and /api/config/).
 * Falls back to unauthenticated fetch for non-API URLs.
 */
export const authFetch = async (url: string, options: RequestInit = {}): Promise<Response> => {
  const headers: Record<string, string> = { ...(options.headers as Record<string, string> || {}) };
  const apiKey = _dashboardApiKey || (typeof process !== 'undefined' ? process.env.NEXT_PUBLIC_DASHBOARD_API_KEY || '' : '');
  if (apiKey && !headers['Authorization'] && !headers['x-api-key']) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }
  return fetch(url, { ...options, headers });
};
