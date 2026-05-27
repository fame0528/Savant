import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import CodeMirror from "@uiw/react-codemirror";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { html } from "@codemirror/lang-html";
import { css } from "@codemirror/lang-css";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { nord } from "@uiw/codemirror-theme-nord";
import { tokyoNight } from "@uiw/codemirror-theme-tokyo-night";
import { githubDark } from "@uiw/codemirror-theme-github";
import { useAppStore } from "@/lib/store";

interface EditorTab {
  id: string;
  path: string;
  name: string;
  content: string;
  language: string;
  modified: boolean;
}

const LANGUAGE_MAP: Record<string, string> = {
  ".ts": "typescript",
  ".tsx": "typescript",
  ".js": "javascript",
  ".jsx": "javascript",
  ".py": "python",
  ".rs": "rust",
  ".html": "html",
  ".htm": "html",
  ".css": "css",
  ".scss": "css",
  ".json": "json",
  ".md": "markdown",
  ".toml": "toml",
  ".yaml": "yaml",
  ".yml": "yaml",
  ".sh": "shell",
  ".bash": "shell",
};

function getLanguage(ext: string): string {
  return LANGUAGE_MAP[ext.toLowerCase()] || "text";
}

function getLanguageExtension(lang: string) {
  switch (lang) {
    case "typescript":
    case "javascript":
      return javascript({ jsx: true, typescript: lang === "typescript" });
    case "python":
      return python();
    case "rust":
      return rust();
    case "html":
      return html();
    case "css":
      return css();
    case "json":
      return json();
    case "markdown":
      return markdown();
    default:
      return [];
  }
}

function getTheme(theme: string) {
  switch (theme) {
    case "nord":
      return nord;
    case "tokyo-night":
      return tokyoNight;
    case "github-dark":
      return githubDark;
    default:
      return tokyoNight;
  }
}

export function EditorPane() {
  const { theme } = useAppStore();
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const saveTimeouts = useRef<Record<string, ReturnType<typeof setTimeout>>>({});

  const activeTab = tabs.find((t) => t.id === activeTabId);

  const openFile = useCallback(async (path: string) => {
    // Check if already open
    const existing = tabs.find((t) => t.path === path);
    if (existing) {
      setActiveTabId(existing.id);
      return;
    }

    setLoading(true);
    try {
      const content = await invoke<string>("fs_read_file", { path });
      const name = path.split(/[/\\]/).pop() || "untitled";
      const ext = name.includes(".") ? "." + name.split(".").pop() : "";
      const language = getLanguage(ext);

      const newTab: EditorTab = {
        id: crypto.randomUUID(),
        path,
        name,
        content,
        language,
        modified: false,
      };

      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(newTab.id);
    } catch (e) {
      console.error("Failed to open file:", e);
    } finally {
      setLoading(false);
    }
  }, [tabs]);

  const handleChange = useCallback((value: string) => {
    if (!activeTabId) return;

    setTabs((prev) =>
      prev.map((t) =>
        t.id === activeTabId ? { ...t, content: value, modified: true } : t
      )
    );

    // Debounce save
    if (saveTimeouts.current[activeTabId]) {
      clearTimeout(saveTimeouts.current[activeTabId]);
    }
    saveTimeouts.current[activeTabId] = setTimeout(async () => {
      const tab = tabs.find((t) => t.id === activeTabId);
      if (tab) {
        try {
          await invoke("fs_write_file", { path: tab.path, content: value });
          setTabs((prev) =>
            prev.map((t) =>
              t.id === activeTabId ? { ...t, modified: false } : t
            )
          );
        } catch (e) {
          console.error("Failed to save:", e);
        }
      }
    }, 1000);
  }, [activeTabId, tabs]);

  const closeTab = useCallback((id: string) => {
    setTabs((prev) => {
      const newTabs = prev.filter((t) => t.id !== id);
      if (activeTabId === id) {
        setActiveTabId(newTabs.length > 0 ? newTabs[newTabs.length - 1].id : null);
      }
      return newTabs;
    });
    if (saveTimeouts.current[id]) {
      clearTimeout(saveTimeouts.current[id]);
      delete saveTimeouts.current[id];
    }
  }, [activeTabId]);

  // Expose openFile globally for file explorer double-click
  useEffect(() => {
    (window as unknown as Record<string, unknown>).__editorOpenFile = openFile;
    return () => {
      delete (window as unknown as Record<string, unknown>).__editorOpenFile;
    };
  }, [openFile]);

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      {/* Tab bar */}
      <div className="flex items-center gap-0 bg-void border-b border-primary/10 shrink-0 overflow-x-auto">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            onClick={() => setActiveTabId(tab.id)}
            className={`flex items-center gap-1 px-3 py-1.5 text-xs cursor-pointer border-r border-primary/5 shrink-0 ${
              tab.id === activeTabId
                ? "bg-void text-primary border-b-2 border-b-primary"
                : "text-success/50 hover:text-success/80 hover:bg-primary/5"
            }`}
          >
            <span className="truncate max-w-[120px]">
              {tab.modified ? "● " : ""}{tab.name}
            </span>
            <button
              onClick={(e) => { e.stopPropagation(); closeTab(tab.id); }}
              className="ml-1 text-primary/30 hover:text-primary text-[10px] leading-none"
            >
              ×
            </button>
          </div>
        ))}
        {tabs.length === 0 && !loading && (
          <div className="px-3 py-1.5 text-xs text-primary/20">
            No file open — double-click a file in the explorer
          </div>
        )}
        {loading && (
          <div className="px-3 py-1.5 text-xs text-primary/30">Loading...</div>
        )}
      </div>

      {/* Editor */}
      <div className="flex-1 overflow-hidden">
        {activeTab ? (
          <CodeMirror
            value={activeTab.content}
            onChange={handleChange}
            theme={getTheme(theme)}
            extensions={[getLanguageExtension(activeTab.language)]}
            height="100%"
            style={{ height: "100%", overflow: "auto" }}
            basicSetup={{
              lineNumbers: true,
              highlightActiveLineGutter: true,
              highlightActiveLine: true,
              foldGutter: true,
              autocompletion: true,
              bracketMatching: true,
              closeBrackets: true,
              indentOnInput: true,
            }}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-primary/20 text-sm">
            <div className="text-center">
              <p className="text-2xl mb-2">⬛</p>
              <p>Open a file to start editing</p>
              <p className="text-xs mt-1 text-primary/10">
                Double-click in explorer or use Ctrl+O
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
