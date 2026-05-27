import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GitFile {
  path: string;
  status: "modified" | "added" | "deleted" | "untracked" | "staged";
}

interface GitCommit {
  hash: string;
  message: string;
  author: string;
  date: string;
}

interface GitBranch {
  name: string;
  current: boolean;
  remote: boolean;
}

export function GitPanel() {
  const [files, setFiles] = useState<GitFile[]>([]);
  const [commits, setCommits] = useState<GitCommit[]>([]);
  const [branches, setBranches] = useState<GitBranch[]>([]);
  const [currentBranch, setCurrentBranch] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [commitMessage, setCommitMessage] = useState("");
  const [activeTab, setActiveTab] = useState<"changes" | "history" | "branches">("changes");
  const [sessionId, setSessionId] = useState<string | null>(null);

  const runGit = useCallback(async (command: string): Promise<string> => {
    if (!sessionId) return "";
    try {
      const result = await invoke<{ stdout: string; stderr: string; exit_code: number; timed_out: boolean }>(
        "shell_session_run",
        { sessionId, command, timeoutSecs: 15 }
      );
      return result.stdout;
    } catch {
      return "";
    }
  }, [sessionId]);

  const loadGitStatus = useCallback(async () => {
    setLoading(true);
    try {
      // Create a session for git commands
      const sid = await invoke<string>("shell_session_create", { cwd: null });
      setSessionId(sid);

      const statusOutput = await runGit("git status --porcelain");
      const gitFiles: GitFile[] = [];
      for (const line of statusOutput.split("\n")) {
        if (line.length < 3) continue;
        const statusCode = line.slice(0, 2).trim();
        const path = line.slice(3).trim();
        let status: GitFile["status"] = "modified";
        if (statusCode === "A" || statusCode === "??") status = "added";
        else if (statusCode === "D") status = "deleted";
        else if (statusCode === "??") status = "untracked";
        else if (statusCode.startsWith("M")) status = "modified";
        else if (statusCode.startsWith(" ")) status = "staged";
        gitFiles.push({ path, status });
      }
      setFiles(gitFiles);

      const branchOutput = await runGit("git branch --show-current");
      setCurrentBranch(branchOutput.trim());

      const logOutput = await runGit("git log --oneline -20 --format='%H|%s|%an|%ar'");
      const gitCommits: GitCommit[] = [];
      for (const line of logOutput.split("\n")) {
        if (!line.trim()) continue;
        const parts = line.split("|");
        if (parts.length >= 4) {
          gitCommits.push({
            hash: parts[0].slice(0, 7),
            message: parts[1],
            author: parts[2],
            date: parts[3],
          });
        }
      }
      setCommits(gitCommits);

      const allBranchesOutput = await runGit("git branch -a --format='%(refname:short)|%(HEAD)'");
      const gitBranches: GitBranch[] = [];
      for (const line of allBranchesOutput.split("\n")) {
        if (!line.trim()) continue;
        const parts = line.split("|");
        if (parts.length >= 2) {
          const name = parts[0].trim();
          const isCurrent = parts[1].trim() === "*";
          const isRemote = name.startsWith("origin/") || name.startsWith("remote/");
          gitBranches.push({ name, current: isCurrent, remote: isRemote });
        }
      }
      setBranches(gitBranches);
    } catch {
      // Git not available
    } finally {
      setLoading(false);
    }
  }, [runGit]);

  useEffect(() => {
    loadGitStatus();
  }, [loadGitStatus]);

  const stageAll = async () => {
    try {
      await runGit("git add -A");
      loadGitStatus();
    } catch (e) {
      console.error("Failed to stage:", e);
    }
  };

  const commit = async () => {
    if (!commitMessage.trim()) return;
    try {
      await runGit(`git commit -m "${commitMessage.replace(/"/g, '\\"')}"`);
      setCommitMessage("");
      loadGitStatus();
    } catch (e) {
      console.error("Failed to commit:", e);
    }
  };

  const getStatusColor = (status: GitFile["status"]) => {
    switch (status) {
      case "added": return "text-success";
      case "modified": return "text-warning";
      case "deleted": return "text-alert";
      case "untracked": return "text-primary/40";
      case "staged": return "text-success";
      default: return "text-success/50";
    }
  };

  const getStatusIcon = (status: GitFile["status"]) => {
    switch (status) {
      case "added": return "A";
      case "modified": return "M";
      case "deleted": return "D";
      case "untracked": return "?";
      case "staged": return "S";
      default: return " ";
    }
  };

  return (
    <div className="flex flex-col h-full w-80 border-r border-primary/15">
      <div className="flex items-center gap-2 px-3 py-2 border-b border-primary/10">
        <span className="text-primary font-bold text-xs">Git</span>
        {currentBranch && (
          <span className="text-xs text-success/50 bg-primary/10 px-2 py-0.5 rounded">
            {currentBranch}
          </span>
        )}
        <button
          onClick={loadGitStatus}
          className="ml-auto text-xs text-primary/40 hover:text-primary transition-colors"
        >
          ↻
        </button>
      </div>

      <div className="flex gap-0 border-b border-primary/10">
        {(["changes", "history", "branches"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-3 py-1.5 text-xs transition-colors ${
              activeTab === tab
                ? "text-primary border-b-2 border-b-primary"
                : "text-success/40 hover:text-success/70"
            }`}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">
        {loading && <div className="p-3 text-xs text-primary/30">Loading...</div>}

        {activeTab === "changes" && !loading && (
          <div className="p-2">
            {files.length === 0 && (
              <div className="text-xs text-primary/20 text-center py-4">No changes</div>
            )}
            <div className="flex gap-2 mb-2">
              <button
                onClick={stageAll}
                className="px-2 py-1 text-[10px] bg-success/10 text-success rounded hover:bg-success/20 transition-colors"
              >
                Stage All
              </button>
            </div>
            {files.map((file) => (
              <div
                key={file.path}
                className="flex items-center gap-2 px-2 py-1 text-xs hover:bg-primary/5 rounded cursor-pointer"
              >
                <span className={`w-3 text-center font-bold ${getStatusColor(file.status)}`}>
                  {getStatusIcon(file.status)}
                </span>
                <span className="text-success/70 truncate flex-1">{file.path}</span>
              </div>
            ))}
            <div className="mt-3 px-2">
              <input
                type="text"
                value={commitMessage}
                onChange={(e) => setCommitMessage(e.target.value)}
                placeholder="Commit message..."
                className="w-full bg-void/50 border border-primary/15 rounded px-2 py-1 text-xs text-success placeholder-primary/20 focus:outline-none focus:border-primary/30"
              />
              <button
                onClick={commit}
                disabled={!commitMessage.trim()}
                className="mt-1 w-full px-2 py-1 text-xs bg-primary/15 text-primary rounded hover:bg-primary/25 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              >
                Commit
              </button>
            </div>
          </div>
        )}

        {activeTab === "history" && !loading && (
          <div className="p-2">
            {commits.length === 0 && (
              <div className="text-xs text-primary/20 text-center py-4">No commits</div>
            )}
            {commits.map((commit) => (
              <div key={commit.hash} className="px-2 py-1.5 text-xs hover:bg-primary/5 rounded">
                <div className="flex items-center gap-2">
                  <span className="text-primary font-bold">{commit.hash}</span>
                  <span className="text-success/50">{commit.date}</span>
                </div>
                <div className="text-success/70 truncate">{commit.message}</div>
                <div className="text-primary/30 text-[10px]">{commit.author}</div>
              </div>
            ))}
          </div>
        )}

        {activeTab === "branches" && !loading && (
          <div className="p-2">
            {branches.length === 0 && (
              <div className="text-xs text-primary/20 text-center py-4">No branches</div>
            )}
            {branches.filter((b) => !b.remote).map((branch) => (
              <div
                key={branch.name}
                className={`flex items-center gap-2 px-2 py-1.5 text-xs rounded cursor-pointer ${
                  branch.current ? "bg-primary/10" : "hover:bg-primary/5"
                }`}
                onClick={async () => {
                  if (!branch.current) {
                    await runGit(`git checkout "${branch.name}"`);
                    loadGitStatus();
                  }
                }}
              >
                <span className={branch.current ? "text-success font-bold" : "text-primary/60"}>
                  {branch.current ? "* " : "  "}
                </span>
                <span className={branch.current ? "text-success" : "text-primary/70"}>{branch.name}</span>
              </div>
            ))}
            {branches.some((b) => b.remote) && (
              <div className="mt-2 pt-2 border-t border-primary/10">
                <div className="text-[10px] text-primary/30 px-2 mb-1">Remote</div>
                {branches.filter((b) => b.remote).map((branch) => (
                  <div
                    key={branch.name}
                    className="flex items-center gap-2 px-2 py-1 text-xs text-primary/40 hover:bg-primary/5 rounded cursor-pointer"
                    onClick={async () => {
                      const localName = branch.name.replace(/^origin\//, "").replace(/^remote\//, "");
                      await runGit(`git checkout -b "${localName}" "${branch.name}"`);
                      loadGitStatus();
                    }}
                  >
                    <span className="text-primary/20">  </span>
                    <span>{branch.name}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
