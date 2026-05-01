import { invoke } from "@tauri-apps/api/core";

export interface BrowserTabInfo {
  id: string;
  url: string;
  title: string;
  loading: boolean;
  agent_name: string | null;
}

export async function showBrowser(): Promise<string> {
  return invoke<string>("show_browser");
}

export async function hideBrowser(): Promise<string> {
  return invoke<string>("hide_browser");
}

export async function browserGetTabs(): Promise<BrowserTabInfo[]> {
  try {
    const result = await invoke<BrowserTabInfo[]>("browser_get_tabs");
    return Array.isArray(result) ? result : [];
  } catch {
    return [];
  }
}

export async function browserNavigate(url: string): Promise<string> {
  return invoke<string>("browser_navigate", { url });
}

export async function browserGoBack(): Promise<string> {
  return invoke<string>("browser_go_back");
}

export async function browserGoForward(): Promise<string> {
  return invoke<string>("browser_go_forward");
}

export async function browserReload(): Promise<string> {
  return invoke<string>("browser_reload");
}

export async function browserNewTab(url: string): Promise<string> {
  return invoke<string>("browser_new_tab", { url });
}

export async function browserCloseTab(tabId: string): Promise<string> {
  return invoke<string>("browser_close_tab", { tab_id: tabId });
}

export async function browserSwitchTab(tabId: string): Promise<string> {
  return invoke<string>("browser_switch_tab", { tab_id: tabId });
}

export async function browserGetContent(): Promise<string> {
  return invoke<string>("browser_get_content");
}
