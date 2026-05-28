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
 * Get the dashboard API key from Tauri's ignite_swarm response.
 * In non-Tauri mode, returns empty string (gateway accepts empty keys by default).
 */
export const getDashboardApiKey = async (): Promise<string> => {
  if (isTauri()) {
    try {
      // ignite_swarm now returns a JSON object with dashboard_api_key
      const result = await invoke<{ dashboard_api_key: string }>("ignite_swarm");
      return result.dashboard_api_key || "";
    } catch {
      return "";
    }
  }
  return "";
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
