import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * 🛰️ Savant Tauri Bridge
 * Unifies communication between the Next.js frontend and the Rust substrate.
 */
export const isTauri = (): boolean => {
  return typeof window !== "undefined" && !!(window as any).__TAURI__;
};

export const igniteSwarm = async (): Promise<string> => {
  if (isTauri()) {
    console.log("🚀 [Tauri] Triggering Swarm Ignition...");
    return await invoke<string>("ignite_swarm");
  }
  return "Not running under Tauri";
};

export const setupLogListener = async (onLog: (msg: string) => void) => {
  if (isTauri()) {
    console.log("🔭 [Tauri] Subscribing to System Logs...");
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
