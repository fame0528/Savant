import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { useAppStore } from "@/lib/store";

export function TerminalTabs() {
  const { ptySessions, activePtyId, setActivePtyId, addPtySession, appendPtyOutput } = useAppStore();
  const terminalRefs = useRef<Record<string, Terminal>>({});
  const containerRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const fitAddons = useRef<Record<string, FitAddon>>({});

  const createTerminal = useCallback(async () => {
    const id = crypto.randomUUID();
    addPtySession(id);

    try {
      await invoke("pty_create", { shell: null, cwd: null });
    } catch (e) {
      console.error("Failed to create PTY:", e);
    }

    setActivePtyId(id);
  }, [addPtySession, setActivePtyId]);

  // Mount terminal when activePtyId changes
  useEffect(() => {
    if (!activePtyId) return;
    const container = containerRefs.current[activePtyId];
    if (!container || terminalRefs.current[activePtyId]) return;

    const term = new Terminal({
      theme: {
        background: "#000e1a",
        foreground: "#1aff00",
        cursor: "#0088ff",
        selectionBackground: "rgba(238, 255, 0, 0.15)",
        black: "#000e1a",
        red: "#ff0000",
        green: "#1aff00",
        yellow: "#eeff00",
        blue: "#0088ff",
        magenta: "#ff00e6",
        cyan: "#0088ff",
        white: "#1aff00",
      },
      fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
      fontSize: 13,
      cursorBlink: true,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    term.open(container);
    fitAddon.fit();

    terminalRefs.current[activePtyId] = term;
    fitAddons.current[activePtyId] = fitAddon;

    // Handle input
    term.onData((data) => {
      invoke("pty_write", { id: activePtyId, data }).catch(console.error);
    });

    // Handle resize
    term.onResize(({ cols, rows }) => {
      invoke("pty_resize", { id: activePtyId, cols, rows }).catch(console.error);
    });

    // Listen for PTY output
    const unlisten = listen<{ id: string; data: string }>(`pty:${activePtyId}`, (event) => {
      if (event.payload.id === activePtyId) {
        term.write(event.payload.data);
        appendPtyOutput(activePtyId, event.payload.data);
      }
    });

    // Fit on resize
    const handleResize = () => {
      fitAddon.fit();
    };
    window.addEventListener("resize", handleResize);

    return () => {
      unlisten.then((f) => f());
      window.removeEventListener("resize", handleResize);
      term.dispose();
      delete terminalRefs.current[activePtyId];
      delete fitAddons.current[activePtyId];
    };
  }, [activePtyId, appendPtyOutput]);

  const sessionIds = Object.keys(ptySessions);

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      {/* Tab bar */}
      <div className="flex items-center gap-1 px-2 py-1 bg-void border-b border-primary/10 shrink-0">
        {sessionIds.map((id) => (
          <button
            key={id}
            onClick={() => setActivePtyId(id)}
            className={`px-3 py-1 text-xs rounded transition-colors ${
              id === activePtyId
                ? "bg-primary/15 text-primary"
                : "text-success/50 hover:text-success/80 hover:bg-primary/5"
            }`}
          >
            Terminal {id.slice(0, 4)}
          </button>
        ))}
        <button
          onClick={createTerminal}
          className="px-2 py-1 text-xs text-primary/40 hover:text-primary hover:bg-primary/10 rounded transition-colors"
        >
          +
        </button>
      </div>

      {/* Terminal containers */}
      <div className="flex-1 overflow-hidden relative">
        {sessionIds.length === 0 && (
          <div className="flex items-center justify-center h-full text-primary/30">
            <button
              onClick={createTerminal}
              className="px-4 py-2 border border-primary/20 rounded hover:bg-primary/10 hover:text-primary transition-colors"
            >
              + New Terminal
            </button>
          </div>
        )}
        {sessionIds.map((id) => (
          <div
            key={id}
            ref={(el) => { containerRefs.current[id] = el; }}
            className={`absolute inset-0 ${id === activePtyId ? "block" : "hidden"}`}
          />
        ))}
      </div>
    </div>
  );
}
