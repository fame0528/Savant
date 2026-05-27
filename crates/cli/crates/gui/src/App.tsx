import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Sidebar } from "./modules/sidebar/Sidebar";
import { TerminalTabs } from "./modules/terminal/TerminalTabs";
import { EditorPane } from "./modules/editor/EditorPane";
import { AiChat } from "./modules/ai/AiChat";
import { ToolApprovalOverlay } from "./modules/ai/ToolApproval";
import { FileExplorer } from "./modules/explorer/FileExplorer";
import { GitPanel } from "./modules/git/GitPanel";
import { StatusBar } from "./modules/statusbar/StatusBar";
import { Header } from "./modules/header/Header";
import { SettingsWindow } from "./modules/settings/SettingsWindow";
import { PetDisplay } from "./modules/pet/PetDisplay";
import { AchievementsPanel } from "./modules/pet/AchievementsPanel";
import { useAppStore } from "./lib/store";

interface Command {
  id: string;
  label: string;
  shortcut?: string;
  action: () => void;
}

function CommandPalette({ onClose }: { onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands: Command[] = [
    { id: "terminal", label: "Open Terminal", shortcut: "Ctrl+`", action: () => { useAppStore.getState().setActivePanel("terminal"); onClose(); } },
    { id: "files", label: "Open File Explorer", shortcut: "Ctrl+E", action: () => { useAppStore.getState().setActivePanel("files"); onClose(); } },
    { id: "search", label: "Search", shortcut: "Ctrl+Shift+F", action: () => { useAppStore.getState().setActivePanel("search"); onClose(); } },
    { id: "git", label: "Git Panel", shortcut: "Ctrl+G", action: () => { useAppStore.getState().setActivePanel("git"); onClose(); } },
    { id: "chat", label: "Toggle AI Chat", action: () => { useAppStore.getState().setChatOpen(!useAppStore.getState().chatOpen); onClose(); } },
    { id: "settings", label: "Open Settings", shortcut: "Ctrl+Shift+P", action: () => { onClose(); /* settings handled externally */ } },
    { id: "sidebar", label: "Toggle Sidebar", shortcut: "Ctrl+B", action: () => { useAppStore.getState().setSidebarOpen(!useAppStore.getState().sidebarOpen); onClose(); } },
  ];

  const filtered = commands.filter((c) =>
    c.label.toLowerCase().includes(query.toLowerCase())
  );

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && filtered.length > 0) {
      filtered[selectedIndex].action();
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh] bg-void/60 backdrop-blur-sm" onClick={onClose}>
      <div className="w-[420px] rounded-lg border border-primary/20 bg-void shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type a command..."
          className="w-full bg-transparent border-b border-primary/10 px-4 py-3 text-sm text-primary outline-none placeholder-primary/30"
        />
        <div className="max-h-60 overflow-y-auto">
          {filtered.length === 0 ? (
            <div className="px-4 py-3 text-xs text-primary/30">No commands found</div>
          ) : (
            filtered.map((cmd, i) => (
              <button
                key={cmd.id}
                onClick={cmd.action}
                className={`w-full flex items-center justify-between px-4 py-2 text-xs transition-colors ${
                  i === selectedIndex ? "bg-primary/15 text-primary" : "text-primary/60 hover:bg-primary/10"
                }`}
              >
                <span>{cmd.label}</span>
                {cmd.shortcut && <span className="text-primary/30 text-[10px]">{cmd.shortcut}</span>}
              </button>
            ))
          )}
        </div>
      </div>
    </div>
  );
}

function SearchPanel() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<{ file: string; line: number; text: string }[]>([]);
  const [searching, setSearching] = useState(false);

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    try {
      const res = await invoke<{ file: string; line: number; text: string }[]>("workspace_search", { query: query.trim() });
      setResults(res);
    } catch (e) {
      console.error("Search failed:", e);
      setResults([]);
    } finally {
      setSearching(false);
    }
  };

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      <div className="flex items-center gap-2 border-b border-primary/15 px-3 py-2">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          placeholder="Search workspace..."
          className="flex-1 bg-transparent text-xs text-primary outline-none placeholder-primary/30"
        />
        <button
          onClick={handleSearch}
          disabled={searching}
          className="px-3 py-1 text-xs bg-primary/10 text-primary rounded hover:bg-primary/20 disabled:opacity-50"
        >
          {searching ? "Searching..." : "Search"}
        </button>
      </div>
      <div className="flex-1 overflow-y-auto">
        {results.length === 0 && query && !searching ? (
          <div className="flex items-center justify-center h-full text-primary/20 text-xs">No results</div>
        ) : (
          results.map((r, i) => (
            <div
              key={i}
              className="flex items-baseline gap-2 px-3 py-1.5 text-xs hover:bg-primary/5 cursor-pointer"
              onClick={() => invoke("editor_open_file", { path: r.file, line: r.line }).catch(() => {})}
            >
              <span className="text-primary/30 font-mono shrink-0 w-16">{r.file.split(/[/\\]/).pop()}</span>
              <span className="text-primary/20 shrink-0 w-10">:{r.line}</span>
              <span className="text-primary/60 truncate">{r.text}</span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default function App() {
  const { activePanel } = useAppStore();
  const [initialized, setInitialized] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [petSubTab, setPetSubTab] = useState<"care" | "achievements">("care");
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);

  useEffect(() => {
    const init = async () => {
      try {
        const home = await invoke<string>("get_home_dir");
        useAppStore.getState().setHomeDir(home);
        const platform = await invoke<string>("get_platform");
        useAppStore.getState().setPlatform(platform);
        const status = await invoke<{ connected: boolean; url: string | null }>("gateway_status");
        useAppStore.getState().setGatewayStatus(status.connected, status.url);
        const gameState = await invoke<any>("gamification_get_state");
        useAppStore.getState().setGamificationState(gameState);
      } catch (e) {
        console.error("Init error:", e);
      }
      setInitialized(true);
    };
    init();

    const unlisten = listen("pty:output", (event) => {
      const payload = event.payload as { id: string; data: string };
      useAppStore.getState().appendPtyOutput(payload.id, payload.data);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  // Record panel usage for achievement tracking
  useEffect(() => {
    if (initialized && activePanel !== "pet") {
      invoke("gamification_record_panel", { panel: activePanel }).catch(() => {});
    }
  }, [activePanel, initialized]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === "p") {
        e.preventDefault();
        setCommandPaletteOpen(true);
      }
      if (e.ctrlKey && e.shiftKey && e.key === "P") {
        e.preventDefault();
        setSettingsOpen(true);
      }
      if (e.ctrlKey && e.key === "b") {
        e.preventDefault();
        useAppStore.getState().setSidebarOpen(!useAppStore.getState().sidebarOpen);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  if (!initialized) {
    return (
      <div className="flex h-screen items-center justify-center bg-void">
        <div className="text-primary font-mono text-lg animate-pulse">
          Initializing Savant CLI Companion...
        </div>
      </div>
    );
  }

  const renderMainContent = () => {
    switch (activePanel) {
      case "files":
        return (
          <div className="flex flex-1 overflow-hidden">
            <FileExplorer />
            <div className="flex-1 flex flex-col overflow-hidden">
              <EditorPane />
            </div>
          </div>
        );
      case "terminal":
        return <TerminalTabs />;
      case "search":
        return <SearchPanel />;
      case "git":
        return (
          <div className="flex flex-1 overflow-hidden">
            <GitPanel />
          </div>
        );
      case "pet":
        return (
          <div className="flex flex-1 overflow-hidden">
            <div className="flex flex-col w-72 border-r border-primary/15 overflow-hidden">
              <div className="flex border-b border-primary/15">
                <button
                  onClick={() => setPetSubTab("care")}
                  className={`flex-1 px-3 py-2 text-xs transition-colors ${
                    petSubTab === "care" ? "bg-primary/15 text-primary" : "text-primary/40 hover:text-primary"
                  }`}
                >{'\u{1f49a}'} Care</button>
                <button
                  onClick={() => setPetSubTab("achievements")}
                  className={`flex-1 px-3 py-2 text-xs transition-colors ${
                    petSubTab === "achievements" ? "bg-primary/15 text-primary" : "text-primary/40 hover:text-primary"
                  }`}
                >{'\u{1f3c6}'} Achievements</button>
              </div>
              <div className="flex-1 overflow-hidden">
                {petSubTab === "care" ? <PetDisplay /> : <AchievementsPanel />}
              </div>
            </div>
            <div className="flex-1 flex flex-col items-center justify-center text-primary/20 text-sm">
              Pet companion panel — your Tamagotchi-style development buddy
            </div>
          </div>
        );
      default:
        return <TerminalTabs />;
    }
  };

  return (
    <div className="flex h-screen flex-col bg-void text-success font-mono overflow-hidden">
      <Header onSettingsClick={() => setSettingsOpen(true)} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <div className="flex flex-1 overflow-hidden relative">
          <div className="flex flex-col flex-1 overflow-hidden">
            {renderMainContent()}
          </div>
          <AiChat />
        </div>
      </div>
      <StatusBar />
      <ToolApprovalOverlay />
      {commandPaletteOpen && <CommandPalette onClose={() => setCommandPaletteOpen(false)} />}
      {settingsOpen && <SettingsWindow onClose={() => setSettingsOpen(false)} />}
    </div>
  );
}
