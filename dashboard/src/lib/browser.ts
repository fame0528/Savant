const isTauri = (): boolean => {
  return typeof window !== 'undefined' && !!((window as any).__TAURI_INTERNALS__ || (window as any).__TAURI__);
};

export interface BrowserTabInfo {
  id: string;
  url: string;
  title: string;
  loading: boolean;
  agent_name: string | null;
}

export async function showBrowser(): Promise<string> {
  if (!isTauri()) return "Not running under Tauri";
  return await (window as any).__TAURI_INTERNALS__.invoke("show_browser");
}

export async function hideBrowser(): Promise<string> {
  if (!isTauri()) return "Not running under Tauri";
  return await (window as any).__TAURI_INTERNALS__.invoke("hide_browser");
}

export async function browserGetTabs(): Promise<BrowserTabInfo[]> {
  if (!isTauri()) return [];
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_get_tabs");
}

export async function browserNavigate(url: string): Promise<string> {
  if (!isTauri()) return "Not running under Tauri";
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_navigate", { url });
}

export async function browserGoBack(): Promise<string> {
  if (!isTauri()) return "Not running under Tauri";
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_go_back");
}

export async function browserGoForward(): Promise<string> {
  if (!isTauri()) return "Not running under Tauri";
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_go_forward");
}

export async function browserReload(): Promise<string> {
  if (!isTauri()) return "Not running under Tauri";
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_reload");
}
