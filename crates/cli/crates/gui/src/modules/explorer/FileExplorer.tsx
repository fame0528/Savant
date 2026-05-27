import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export function FileExplorer() {
  const [files, setFiles] = useState<FileNode[]>([]);
  const [currentPath, setCurrentPath] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDir = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<FileNode[]>("fs_list_dir", { path });
      setFiles(result);
      setCurrentPath(path);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // Load home directory on mount
    invoke<string>("get_home_dir").then((home) => {
      loadDir(home);
    }).catch(() => {
      loadDir(".");
    });
  }, [loadDir]);

  const handleClick = (node: FileNode) => {
    if (node.is_dir) {
      loadDir(node.path);
    }
  };

  const handleGoUp = () => {
    if (!currentPath) return;
    const parts = currentPath.split(/[/\\]/);
    parts.pop();
    const parent = parts.join("/") || "/";
    loadDir(parent);
  };

  return (
    <div className="w-72 bg-void border-r border-primary/15 flex flex-col shrink-0">
      {/* Header */}
      <div className="p-3 border-b border-primary/15 flex items-center justify-between">
        <span className="text-primary font-bold text-sm">Explorer</span>
        <button
          onClick={handleGoUp}
          className="text-xs text-primary/40 hover:text-primary transition-colors px-2 py-1 rounded hover:bg-primary/10"
        >
          ↑ Up
        </button>
      </div>

      {/* Current path */}
      <div className="px-3 py-1 border-b border-primary/5 text-[10px] text-primary/30 truncate">
        {currentPath || "Loading..."}
      </div>

      {/* File list */}
      <div className="flex-1 overflow-y-auto">
        {loading && (
          <div className="p-3 text-primary/30 text-xs">Loading...</div>
        )}
        {error && (
          <div className="p-3 text-alert/50 text-xs">{error}</div>
        )}
        {!loading && files.length === 0 && !error && (
          <div className="p-3 text-primary/30 text-xs">Empty directory</div>
        )}
        {files.map((node) => (
          <button
            key={node.path}
            onClick={() => handleClick(node)}
            className="w-full text-left px-3 py-1.5 text-xs text-success/70 hover:bg-primary/5 hover:text-success flex items-center gap-2 transition-colors"
          >
            <span className={node.is_dir ? "text-primary" : "text-success/50"}>
              {node.is_dir ? "📁" : "📄"}
            </span>
            <span className="truncate">{node.name}</span>
            {!node.is_dir && (
              <span className="ml-auto text-[10px] text-primary/20">
                {formatSize(node.size)}
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}
