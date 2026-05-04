export interface BrowserTabInfo {
  id: string;
  url: string;
  title: string;
  loading: boolean;
  agent_name: string | null;
}

export async function showBrowser(): Promise<string> {
  return await (window as any).__TAURI_INTERNALS__.invoke("show_browser");
}

export async function hideBrowser(): Promise<string> {
  return await (window as any).__TAURI_INTERNALS__.invoke("hide_browser");
}

export async function browserGetTabs(): Promise<BrowserTabInfo[]> {
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_get_tabs");
}

export async function browserNavigate(url: string): Promise<string> {
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_navigate", { url });
}

export async function browserGoBack(): Promise<string> {
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_go_back");
}

export async function browserGoForward(): Promise<string> {
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_go_forward");
}

export async function browserReload(): Promise<string> {
  return await (window as any).__TAURI_INTERNALS__.invoke("browser_reload");
}
