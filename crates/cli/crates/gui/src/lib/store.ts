import { create } from "zustand";

export type ActivePanel = "terminal" | "files" | "search" | "git" | "pet";
export type Theme = "tokyo-night" | "nord" | "github-dark" | "void";

export interface PetSummary {
  stage: string;
  stage_icon: string;
  mood: string;
  mood_icon: string;
  hunger: number;
  happiness: number;
  discipline: number;
  health: number;
  weight: number;
  cleanliness: number;
  poop_count: number;
  is_sick: boolean;
  is_sleeping: boolean;
  is_dead: boolean;
  age_hours: number;
  care_mistakes: number;
}

export interface Achievement {
  id: string;
  name: string;
  description: string;
  icon: string;
  unlocked_at: number | null;
}

export interface GamificationState {
  pet: PetSummary;
  xp: number;
  current_streak: number;
  longest_streak: number;
  checkin_count: number;
  total_sessions: number;
  total_turns: number;
  total_tool_calls: number;
  unlocked_achievements: number;
  total_achievements: number;
  achievements: Achievement[];
}

interface PtySession {
  id: string;
  output: string;
  cwd: string | null;
}

interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
}

interface AppState {
  // UI State
  activePanel: ActivePanel;
  theme: Theme;
  sidebarOpen: boolean;
  chatOpen: boolean;

  // System
  homeDir: string;
  platform: string;
  gatewayConnected: boolean;
  gatewayUrl: string | null;

  // PTY
  ptySessions: Record<string, PtySession>;
  activePtyId: string | null;

  // Chat
  chatMessages: ChatMessage[];
  chatInput: string;
  chatStreaming: boolean;

  // Gamification
  game: GamificationState | null;
  gameEvents: string[];
  gamePanelOpen: boolean;

  // Actions
  setActivePanel: (panel: ActivePanel) => void;
  setTheme: (theme: Theme) => void;
  setSidebarOpen: (open: boolean) => void;
  setChatOpen: (open: boolean) => void;
  setHomeDir: (dir: string) => void;
  setPlatform: (platform: string) => void;
  setGatewayStatus: (connected: boolean, url: string | null) => void;

  // PTY actions
  addPtySession: (id: string) => void;
  removePtySession: (id: string) => void;
  setActivePtyId: (id: string | null) => void;
  appendPtyOutput: (id: string, data: string) => void;
  clearPtyOutput: (id: string) => void;

  // Chat actions
  addChatMessage: (msg: Omit<ChatMessage, "id" | "timestamp">) => void;
  setChatInput: (input: string) => void;
  setChatStreaming: (streaming: boolean) => void;
  clearChat: () => void;

  // Gamification actions
  setGamificationState: (state: GamificationState) => void;
  addGameEvent: (event: string) => void;
  clearGameEvents: () => void;
  setGamePanelOpen: (open: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  activePanel: "terminal",
  theme: "void",
  sidebarOpen: true,
  chatOpen: true,
  homeDir: "",
  platform: "unknown",
  gatewayConnected: false,
  gatewayUrl: null,
  ptySessions: {},
  activePtyId: null,
  chatMessages: [],
  chatInput: "",
  chatStreaming: false,
  game: null,
  gameEvents: [],
  gamePanelOpen: false,

  setActivePanel: (panel) => set({ activePanel: panel }),
  setTheme: (theme) => set({ theme }),
  setSidebarOpen: (open) => set({ sidebarOpen: open }),
  setChatOpen: (open) => set({ chatOpen: open }),
  setHomeDir: (dir) => set({ homeDir: dir }),
  setPlatform: (platform) => set({ platform }),
  setGatewayStatus: (connected, url) => set({ gatewayConnected: connected, gatewayUrl: url }),

  addPtySession: (id) =>
    set((state) => ({
      ptySessions: {
        ...state.ptySessions,
        [id]: { id, output: "", cwd: null },
      },
      activePtyId: id,
    })),

  removePtySession: (id) =>
    set((state) => {
      const sessions = { ...state.ptySessions };
      delete sessions[id];
      return {
        ptySessions: sessions,
        activePtyId: state.activePtyId === id ? null : state.activePtyId,
      };
    }),

  setActivePtyId: (id) => set({ activePtyId: id }),

  appendPtyOutput: (id, data) =>
    set((state) => {
      const session = state.ptySessions[id];
      if (!session) return state;
      return {
        ptySessions: {
          ...state.ptySessions,
          [id]: { ...session, output: session.output + data },
        },
      };
    }),

  clearPtyOutput: (id) =>
    set((state) => {
      const session = state.ptySessions[id];
      if (!session) return state;
      return {
        ptySessions: {
          ...state.ptySessions,
          [id]: { ...session, output: "" },
        },
      };
    }),

  addChatMessage: (msg) =>
    set((state) => ({
      chatMessages: [
        ...state.chatMessages,
        { ...msg, id: crypto.randomUUID(), timestamp: Date.now() },
      ],
    })),

  setChatInput: (input) => set({ chatInput: input }),
  setChatStreaming: (streaming) => set({ chatStreaming: streaming }),
  clearChat: () => set({ chatMessages: [] }),

  setGamificationState: (state) => set({ game: state }),
  addGameEvent: (event) => set((s) => ({ gameEvents: [...s.gameEvents.slice(-49), event] })),
  clearGameEvents: () => set({ gameEvents: [] }),
  setGamePanelOpen: (open) => set({ gamePanelOpen: open }),
}));
