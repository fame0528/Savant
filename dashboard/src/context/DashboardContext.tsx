"use client";

import React, { createContext, useContext, useState, useEffect, useCallback, useRef, ReactNode } from "react";
import { useRouter } from "next/navigation";
import { isTauri, igniteSwarm } from "@/lib/tauri";
import { logger } from "@/lib/logger";
import dayjs from "dayjs";
import localizedFormat from "dayjs/plugin/localizedFormat";
dayjs.extend(localizedFormat);

// ─── Types ─────────────────────────────────────────────────────────────

interface Message {
  role: string;
  content: string;
  agent?: string;
  timestamp: string;
  thoughts?: string;
}

interface Agent {
  id: string;
  name: string;
  status: string;
  role: string;
  image?: string;
}

interface Insight {
  agent_id: string;
  content: string;
  category: string;
  timestamp: string;
}

export interface DashboardState {
  // UI state
  activeAgent: string | null;
  isManifestMode: boolean;
  manifestPrompt: string;
  manifestDraft: string;
  manifestMetrics: Record<string, number>;
  manifestName: string;
  isGenerating: boolean;
  isCollapsed: boolean;
  isRightCollapsed: boolean;
  showDebug: boolean;
  debugExpanded: boolean;
  debugPaused: boolean;
  copiedId: string | null;
  showSplash: boolean;
  
  // Data state
  agents: Agent[];
  laneMessages: Record<string, Message[]>;
  cognitiveInsights: Insight[];
  debugLogs: {timestamp: string, message: string}[];
  streamingContent: Map<string, string>;
  streamingThoughts: Map<string, string>;
  
  // Connection state
  connectionStatus: 'NOMINAL' | 'OFFLINE';
  isReady: boolean;
  isMounted: boolean;
  isSessionReady: boolean;
  typingAgents: Set<string>;
  syncedLanes: Set<string>;
  collapsedInsights: Set<string>;
  
  // UI refs
  insightScrollRef: React.RefObject<HTMLDivElement | null>;
  
  // Actions
  setActiveAgent: (id: string | null) => void;
  setManifestPrompt: (s: string) => void;
  setManifestDraft: (s: string) => void;
  setManifestName: (s: string) => void;
  setManifestMetrics: (m: Record<string, number>) => void;
  setIsGenerating: (b: boolean) => void;
  setIsCollapsed: (b: boolean) => void;
  setIsRightCollapsed: (b: boolean) => void;
  setShowDebug: (b: boolean) => void;
  setDebugExpanded: (b: boolean) => void;
  setDebugPaused: (b: boolean) => void;
   setCopiedId: (id: string | null) => void;
   setShowSplash: (b: boolean) => void;
   setIsManifestMode: (b: boolean) => void;
  setAgents: (agents: Agent[]) => void;
  setLaneMessages: (fn: (prev: Record<string, Message[]>) => Record<string, Message[]> ) => void;
  setCognitiveInsights: (insights: Insight[]) => void;
  setDebugLogs: (logs: {timestamp: string, message: string}[]) => void;
  setStreamingContent: (map: Map<string, string>) => void;
  setStreamingThoughts: (map: Map<string, string>) => void;
  setConnectionStatus: (s: 'NOMINAL' | 'OFFLINE') => void;
  setIsReady: (b: boolean) => void;
  setIsMounted: (b: boolean) => void;
  setTypingAgents: (set: Set<string>) => void;
  setSyncedLanes: (set: Set<string>) => void;
  setCollapsedInsights: (set: Set<string>) => void;
  
  // Helpers
   handleLaneSwitch: (laneId: string | null, isManifest?: boolean) => void;
   getAgentMeta: (agentId: string | undefined, role: string) => { name: string; image?: string | null; isUser?: boolean; isAgent?: boolean; isSystem?: boolean; isUnknown?: boolean };
  handleCopy: (text: string, id: string) => void;
  toggleInsightCollapse: (id: string) => void;
  scrollInsightsToTop: () => void;
  formatEst: (utcTimestamp: string) => string;
  sendControlFrame: (type: string, data: Record<string, unknown>) => void;
  sendChatMessage: (role: string, content: string, recipient: string | null, broadcast?: boolean) => void;
  requestLaneHistory: (laneId: string, limit?: number) => void;
  handleManifestSubmit: () => void;
  handleManifestCommit: () => void;
}

const DashboardContext = createContext<DashboardState | null>(null);

export function useDashboard() {
  const ctx = useContext(DashboardContext);
  if (!ctx) throw new Error("useDashboard must be used within DashboardProvider");
  return ctx;
}

// ─── Helper Functions ────────────────────────────────────────────────────

const getGatewayHost = () => {
  if (typeof window === "undefined") return "127.0.0.1";
  const host = window.location.hostname;
  if (!host || host === "localhost" || host.includes("tauri")) return "127.0.0.1";
  return host || "127.0.0.1";
};

const getGatewayPort = () => {
  if (typeof window !== "undefined") {
    const envPort = process.env.NEXT_PUBLIC_GATEWAY_PORT;
    if (envPort) return parseInt(envPort, 10);
  }
  return 8080;
};

const getWsUrl = () => `ws://${getGatewayHost()}:${getGatewayPort()}/ws`;
const getHttpUrl = () => `http://${getGatewayHost()}:${getGatewayPort()}`;

const WS_RECONNECT_MAX_DELAY = 30000;
const WS_RECONNECT_BASE_DELAY = 1000;

const cleanMessage = (content: string) => {
  if (!content) return "";
  let cleaned = content;
  cleaned = cleaned.replace(/(\[?\s*OPENROUTER PROCESSING\s*\]?\s*)+/gi, '');
  cleaned = cleaned.replace(/<environment_details>[\s\S]*?<\/environment_details>/gi, '');
  cleaned = cleaned.replace(/<function=[^>]*>[\s\S]*?<\/function>/gi, '');
  cleaned = cleaned.replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, '');
  cleaned = cleaned.replace(/<use_mcp_tool[\s\S]*?<\/use_mcp_tool>/gi, '');
  cleaned = cleaned.replace(/<read_file[\s\S]*?<\/read_file>/gi, '');
  cleaned = cleaned.replace(/<write_to_file[\s\S]*?<\/write_to_file>/gi, '');
  cleaned = cleaned.replace(/<execute_command[\s\S]*?<\/execute_command>/gi, '');
  cleaned = cleaned.replace(/<think>[\s\S]*?<\/think>/gi, '');
  cleaned = cleaned.replace(/<thinking>[\s\S]*?<\/thinking>/gi, '');
  cleaned = cleaned.replace(/<thought>[\s\S]*?<\/thought>/gi, '');
  cleaned = cleaned.replace(/<reasoning>[\s\S]*?<\/reasoning>/gi, '');
  if (cleaned.includes('"choices"') || cleaned.includes('"delta"')) {
    try {
      const match = cleaned.match(/"content"\s*:\s*"((?:[^"\\]|\\.)*)"/);
      if (match && match[1]) {
        cleaned = match[1].replace(/\\n/g, '\n').replace(/\\"/g, '"');
      }
    } catch (e) { }
  }
  return cleaned
    .replace(/\\n/g, '\n')
    .replace(/^[:\s\n]+/, '')
    .trim();
};

const CollapsibleThoughts = React.memo(({ thoughts }: { thoughts: string }) => {
  const [collapsed, setCollapsed] = React.useState(true);
  if (!thoughts.trim()) return null;
  return (
    <div style={{
      marginBottom: '12px', borderRadius: '8px', border: '1px solid rgba(255, 255, 0, 0.15)',
      background: 'rgba(255, 255, 0, 0.03)', overflow: 'hidden'
    }}>
      <div onClick={() => setCollapsed(!collapsed)} style={{
        padding: '8px 12px', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '8px',
        fontSize: '10px', fontWeight: 900, color: 'rgba(255, 255, 0, 0.7)', letterSpacing: '1px'
      }}>
        <span style={{ transform: collapsed ? 'rotate(0deg)' : 'rotate(90deg)', transition: 'transform 0.2s' }}>▶</span>
        THINKING
      </div>
      {!collapsed && (
        <div style={{ padding: '0 12px 12px', fontSize: '13px', lineHeight: '1.5', color: 'rgba(255, 255, 0, 0.6)', fontStyle: 'italic', whiteSpace: 'pre-wrap' }}>
          {thoughts}
        </div>
      )}
    </div>
  );
});
(CollapsibleThoughts as any).displayName = 'CollapsibleThoughts';

// ─── Provider Component ─────────────────────────────────────────────────

export function DashboardProvider({ children }: { children: ReactNode }) {
  const router = useRouter();
  // UI state
  const [activeAgent, setActiveAgent] = useState<string | null>(null);
  const [isManifestMode, setIsManifestMode] = useState(false);
  const [manifestPrompt, setManifestPrompt] = useState("");
  const [manifestDraft, setManifestDraft] = useState("");
  const [manifestMetrics, setManifestMetrics] = useState<Record<string, number>>({
    depth: 0, integrity: 0, fidelity: 0, ethics: 0, mission: 0,
    autonomy: 0, complexity: 0, resonance: 0, nuance: 0, stability: 0
  });
  const [manifestName, setManifestName] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isRightCollapsed, setIsRightCollapsed] = useState(false);
  const [showDebug, setShowDebug] = useState(false);
  const [debugExpanded, setDebugExpanded] = useState(false);
  const [debugPaused, setDebugPaused] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [showSplash, setShowSplash] = useState(true);
  
  // Data state
  const [agents, setAgents] = useState<Agent[]>([]);
  const [laneMessages, setLaneMessages] = useState<Record<string, Message[]>>({ global: [] });
  const [cognitiveInsights, setCognitiveInsights] = useState<Insight[]>([]);
  const [debugLogs, setDebugLogs] = useState<{timestamp: string, message: string}[]>([]);
  const [streamingContent, setStreamingContent] = useState<Map<string, string>>(new Map());
  const [streamingThoughts, setStreamingThoughts] = useState<Map<string, string>>(new Map());
  const streamingThoughtsRef = useRef<Map<string, string>>(new Map());
  const [collapsedInsights, setCollapsedInsights] = useState<Set<string>>(new Set());
  const insightScrollRef = useRef<HTMLDivElement>(null);
  
  // Connection state
  const [connectionStatus, setConnectionStatus] = useState<'NOMINAL' | 'OFFLINE'>('OFFLINE');
  const [isReady, setIsReady] = useState(false);
  const [isMounted, setIsMounted] = useState(false);
  const [isSessionReady, setIsSessionReady] = useState(false);
  const [typingAgents, setTypingAgents] = useState<Set<string>>(new Set());
  const [syncedLanes, setSyncedLanes] = useState<Set<string>>(new Set());
  
  const socketRef = useRef<WebSocket | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const isConnectingRef = useRef(false);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);

  // Keep ref in sync with state
  const setStreamingThoughtsSynced = useCallback((updater: (prev: Map<string, string>) => Map<string, string>) => {
    setStreamingThoughts(prev => {
      const next = updater(prev);
      streamingThoughtsRef.current = next;
      return next;
    });
  }, []);

  const getAgentMeta = useCallback((agentId: string | undefined, role: string) => {
    if (role === 'user') return { name: 'YOU', image: null, isUser: true };
    const id = agentId?.toLowerCase() || 'unknown';
    if (id === 'savant' || id === 'sav' || id === 'system') {
      return { name: 'SAVANT', image: `${getHttpUrl()}/api/agents/savant/image`, isSystem: true };
    }
    const agent = agents.find(a => a.id.toLowerCase() === id || a.name.toLowerCase() === id);
    if (agent) {
      const imageUrl = agent.image || `${getHttpUrl()}/api/agents/${agent.id}/image`;
      return { name: agent.name.toUpperCase(), image: imageUrl, isAgent: true, isSystem: false };
    }
    return { name: id.toUpperCase(), image: `${getHttpUrl()}/api/agents/${id}/image`, isUnknown: true };
  }, [agents]);

  const handleCopy = useCallback(async (text: string, id: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedId(id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch (err) {
      logger.error('Clipboard', 'Failed to copy text', err);
    }
  }, []);

  const toggleInsightCollapse = useCallback((id: string) => {
    setCollapsedInsights(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const scrollInsightsToTop = useCallback(() => {
    if (insightScrollRef.current) {
      insightScrollRef.current.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }, []);

  const formatEst = (utcTimestamp: string) => {
    try {
      const date = new Date(utcTimestamp);
      if (isNaN(date.getTime())) return "--:--";
      return new Intl.DateTimeFormat('en-US', {
        timeZone: 'America/New_York',
        hour: 'numeric',
        minute: 'numeric',
        hour12: true
      }).format(date);
    } catch (e) {
      return "--:--";
    }
  };

  // Removed renderFormattedContent; use FormattedContent component instead.

  const handleLaneSwitch = useCallback((laneId: string | null, isManifest: boolean = false) => {
    const normalizedId = laneId ? laneId.toLowerCase().trim() : null;
    setActiveAgent(normalizedId);
    setIsManifestMode(isManifest);
    setTypingAgents(new Set());
    setStreamingContent(new Map());
    if (normalizedId) {
      localStorage.setItem('activeAgent', normalizedId);
    } else if (!isManifest) {
      localStorage.removeItem('activeAgent');
    }
    // Navigate to chat page so sidebar clicks always bring user back to chat
    router.push('/');
    if (socketRef.current?.readyState === WebSocket.OPEN && !isManifest && sessionIdRef.current) {
      const laneKey = normalizedId || "global";
      if (!syncedLanes.has(laneKey)) {
        socketRef.current.send(JSON.stringify({
          session_id: sessionIdRef.current,
          payload: { type: "HistoryRequest", data: { lane_id: laneKey, limit: 100 } }
        }));
        setSyncedLanes(prev => new Set(prev).add(laneKey));
      }
    }
  }, [syncedLanes, router]);

  const processEvent = (type: string, evData: any) => {
    if (type === "session.assigned") {
      sessionIdRef.current = evData.session_id;
      setIsSessionReady(true);
      logger.info("[Dashboard] Session assigned:", evData.session_id);
    } else if (type === "agents.discovered") {
      const rawAgents = evData.agents || [];
      const uniqueAgents = Array.from(new Map(rawAgents.map((a: any, i: number) => {
        const id = a.id || `agent-${i}`;
        return [id, { ...a, id }];
      })).values());
      setAgents(uniqueAgents as Agent[]);
    } else if (type === "history") {
      const { lane_id, history } = evData;
      if (Array.isArray(history)) {
        setLaneMessages(prev => ({
          ...prev,
          [lane_id]: history
            .filter((m: any) => {
              const content = ((m.content as string) || '').toUpperCase();
              const isTel = m.is_telemetry === true || m.is_telemetry === 1;
              return !(isTel || content.includes('HEARTBEAT_OK') || content.includes('[PROACTIVE HEARTBEAT]'));
            })
            .map((m: any) => ({
              role: (m.role as string) || 'assistant',
              content: (m.content as string) || '',
              agent: (m.sender as string) || (m.agent_id as string) || 'unknown'
            }))
        }));
      }
    } else if (type === "chat.message") {
      const msg = evData;
      const content = msg.content || '';
      if (!content.trim()) return; // Skip empty messages (e.g., from failed streaming)
      const agentId = (msg.agent_id || msg.sender || 'unknown').toLowerCase().trim();
      setTypingAgents(prev => { const next = new Set(prev); next.delete(agentId); return next; });
      setStreamingContent(prev => { const next = new Map(prev); next.delete(agentId); return next; });
      const agentThoughts = streamingThoughtsRef.current.get(agentId);
      setStreamingThoughtsSynced(prev => { const next = new Map(prev); next.delete(agentId); return next; });
      if (msg.is_telemetry) {
        setCognitiveInsights(prev => [{
          agent_id: msg.agent_id || msg.sender || 'system',
          content: content,
          category: 'insight',
          timestamp: new Date().toISOString()
        }, ...prev].slice(0, 100));
        return;
      }
      const isTechnical = content.toUpperCase().includes('HEARTBEAT_OK') || content.toUpperCase().includes('[PROACTIVE HEARTBEAT]');
      if (isTechnical) {
        setCognitiveInsights(prev => [{
          agent_id: msg.agent_id || msg.sender || 'system',
          content: content,
          category: 'insight',
          timestamp: new Date().toISOString()
        }, ...prev].slice(0, 100));
        return;
      }
      const isSystemAgent = (agentId === 'savant' || agentId === 'sav' || agentId === 'system');
      const isBroadcast = (msg.broadcast === true || msg.recipient === 'swarm' || !msg.recipient) && !isSystemAgent;
      const primaryLane = isBroadcast ? "global" : (msg.recipient || agentId);
      const targetLanes = [primaryLane];
      if (msg.role === 'assistant' && agentId && agentId !== 'unknown' && !targetLanes.includes(agentId)) {
        targetLanes.push(agentId);
      }
      setLaneMessages((prev) => {
        const next = { ...prev };
        targetLanes.forEach(lane => {
          const laneId = lane.toLowerCase();
          next[laneId] = [...(next[laneId] || []), { 
            role: msg.role || 'assistant', 
            content: content, 
            agent: agentId, 
            timestamp: new Date().toISOString(), 
            thoughts: agentThoughts 
          }];
        });
        return next;
      });
    } else if (type === "chat.chunk") {
      const chunk = evData;
      const agentId = (chunk.agent_id || '').toLowerCase().trim();
      if (chunk.is_telemetry) {
        const thoughtContent = chunk.reasoning || chunk.content || '';
        if (thoughtContent.trim()) {
          setStreamingThoughtsSynced(prev => {
            const next = new Map(prev);
            const existing = next.get(agentId) || '';
            next.set(agentId, existing + thoughtContent);
            return next;
          });
        }
        return;
      }
      setTypingAgents(prev => { const next = new Set(prev); next.add(agentId); return next; });
      setStreamingContent(prev => { const next = new Map(prev); next.set(agentId, (next.get(agentId) || '') + (chunk.content || "")); return next; });
    } else if (type === "heartbeat") { /* ignore */ }
    else if (type === "agents.discovered") {
      const agentsList = evData.agents || [];
      if (Array.isArray(agentsList) && agentsList.length > 0) {
        setAgents(agentsList);
        logger.info('Agents', `Discovered ${agentsList.length} agents`);
        if (!activeAgent && agentsList[0]?.id) {
          setActiveAgent(agentsList[0].id);
        }
      }
    } else if (type === "swarm_insight_history") {
      const { history } = evData;
      if (Array.isArray(history)) {
        setCognitiveInsights(history.map((h: any) => ({ 
          agent_id: h.agent_id, 
          content: h.content, 
          category: h.category, 
          timestamp: h.timestamp 
        })));
      }
    } else if (type === "debug.log") {
      const logMsg = typeof evData === 'string' ? evData : (evData.message || JSON.stringify(evData));
      setDebugLogs(prev => [{ timestamp: new Date().toISOString(), message: logMsg }, ...prev]);
    } else if (type === "learning.insight") {
      const insight = evData;
      setCognitiveInsights(prev => [{
        agent_id: insight.agent_id || 'system',
        content: insight.content || '',
        category: insight.category || 'insight',
        timestamp: insight.timestamp || new Date().toISOString()
      }, ...prev].slice(0, 100));
    } else if (type === "update_success") {
      setIsManifestMode(false);
      setManifestDraft("");
    } else if (type === "system.config.updated") {
      const { section, notes } = evData;
      setCognitiveInsights(prev => [{
        agent_id: 'guardian',
        content: `SWARM_CONFIG_UPDATED: [${section}] ${notes?.join(' | ') || ''}`,
        category: 'insight',
        timestamp: new Date().toISOString()
      }, ...prev].slice(0, 100));
    } else if (type === "system.config.reset") {
      const { section } = evData;
      setCognitiveInsights(prev => [{
        agent_id: 'guardian',
        content: `SWARM_CONFIG_RESTORED: [${section}] System defaults applied across all manifestations.`,
        category: 'insight',
        timestamp: new Date().toISOString()
      }, ...prev].slice(0, 100));
    }
  };

  const connectWebSocket = useCallback(() => {
    if (isConnectingRef.current || socketRef.current) return;
    isConnectingRef.current = true;

    const socket = new WebSocket(getWsUrl());
    socketRef.current = socket;

    socket.onopen = () => {
      reconnectAttemptsRef.current = 0;
      isConnectingRef.current = false;
      
      // Send authentication frame first — session handshake begins
      const apiKey = process.env.NEXT_PUBLIC_DASHBOARD_API_KEY;
      if (!apiKey) {
        console.error('[SAVANT] NEXT_PUBLIC_DASHBOARD_API_KEY is not set. Dashboard auth will fail.');
      }
      socket.send(JSON.stringify({
        session_id: "",
        payload: `DASHBOARD_API_KEY:${apiKey || ''}`,
        signature: null
      }));

      // NOTE: InitialSync and SwarmInsightHistoryRequest are deferred
      // until the server sends session.assigned with the correct session ID.
      // Sending them before session establishment would cause silent rejection.
      setConnectionStatus('NOMINAL');
      setIsReady(true);
      setIsMounted(true);

      setTimeout(async () => {
        try {
          const resp = await fetch(`${getHttpUrl()}/api/agents`);
          if (resp.ok) {
            const data = await resp.json();
            if (data.agents && data.agents.length > 0) {
              logger.info('Agents', `HTTP fallback: found ${data.agents.length} agents`);
              setAgents(data.agents);
            }
          }
        } catch (e) {
          logger.warn('Agents', 'HTTP agent list fetch failed', e);
        }
      }, 3000);

      const saved = localStorage.getItem('activeAgent');
      if (saved) {
        const norm = saved.toLowerCase().trim();
        setTimeout(() => { handleLaneSwitch(norm); }, 100);
      }
    };

    socket.onclose = () => {
      setConnectionStatus('OFFLINE');
      setIsReady(false);
      socketRef.current = null;
      isConnectingRef.current = false;

      const attempt = reconnectAttemptsRef.current;
      const delay = Math.min(WS_RECONNECT_BASE_DELAY * Math.pow(2, attempt), WS_RECONNECT_MAX_DELAY);
      reconnectAttemptsRef.current = attempt + 1;
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = setTimeout(() => connectWebSocket(), delay);
    };

    socket.onerror = () => {
      setConnectionStatus('OFFLINE');
    };

    socket.onmessage = (event) => {
      const data = event.data;
      if (!data.startsWith("EVENT:")) return;
      try {
        const prefix = data.substring(0, data.indexOf(':'));
        const rawJson = data.substring(data.indexOf(':') + 1);
        const payload = JSON.parse(rawJson);
        if (prefix === "EVENT") {
          const eventType = payload.event_type;
          const evData = typeof payload.payload === 'string' ? JSON.parse(payload.payload) : payload.payload;

          // Session handshake: capture assigned session ID and trigger sync sequence
          if (eventType === "session.assigned") {
            sessionIdRef.current = evData.session_id;
            setIsSessionReady(true);
            logger.info("[Dashboard] Session established:", evData.session_id);

            // Now that session is established, send handshake messages
            if (socket.readyState === WebSocket.OPEN) {
              socket.send(JSON.stringify({
                session_id: evData.session_id,
                payload: { type: "InitialSync" }
              }));
              socket.send(JSON.stringify({
                session_id: evData.session_id,
                payload: { type: "SwarmInsightHistoryRequest", data: { limit: 50 } }
              }));
            }
          }

          processEvent(eventType, evData);
        }
      } catch (e) {
        logger.error('WebSocket', "Failed to parse message", e, data);
      }
    };
  }, [handleLaneSwitch, setAgents]);

  // WebSocket connection — run once only
  const initDoneRef = useRef(false);
  useEffect(() => {
    if (initDoneRef.current) return;
    initDoneRef.current = true;

    if (!isTauri()) {
      connectWebSocket();
      return () => {
        if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
        if (socketRef.current) socketRef.current.close();
      };
    }

    logger.info('Tauri', 'Running in Tauri mode');

    const initTauri = async () => {
      try {
        const result = await igniteSwarm();
        logger.info('Ignition', 'Swarm Ignition:', result);
        setConnectionStatus('NOMINAL');
        setIsMounted(true);
        setTimeout(() => { connectWebSocket(); }, 2000);
      } catch (e) {
        logger.error('Ignition', 'Ignition Failure:', e);
        setConnectionStatus('OFFLINE');
        setIsMounted(true);
      }
    };
    initTauri();

    let unlisten: (() => void) | null = null;
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen('system-log-event', (event: any) => {
        const logMsg = event.payload as string;
        logger.debug('System', logMsg);
        setDebugLogs(prev => [{ timestamp: new Date().toISOString(), message: logMsg }, ...prev]);
      }).then(u => unlisten = u).catch((e) => logger.error('Tauri', 'Failed to listen to system-log-event', e));
    }).catch((e) => logger.error('Tauri', 'Failed to import @tauri-apps/api/event', e));

    return () => {
      if (unlisten) unlisten();
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      if (socketRef.current) socketRef.current.close();
    };
  }, []);

  // Auto-dismiss splash — always dismiss after timeout regardless of connection state
  useEffect(() => {
    const splashTimer = setTimeout(() => setShowSplash(false), 8000);
    const mountedTimer = isMounted ? setTimeout(() => setShowSplash(false), 2000) : null;
    return () => {
      clearTimeout(splashTimer);
      if (mountedTimer) clearTimeout(mountedTimer);
    };
  }, [isMounted]);

  // Manifest metrics calculation
  useEffect(() => {
    if (!manifestDraft) return;
    const lineCount = manifestDraft.split('\n').length;
    const wordCount = manifestDraft.split(/\s+/).filter(Boolean).length;
    const sectionCount = (manifestDraft.match(/^##\s/gm) || []).length;
    const hasIdentity = /identity|archetype|designation/i.test(manifestDraft);
    const hasEthics = /ethical|guardrail|constraint|boundary/i.test(manifestDraft);
    const hasMission = /mission|directive|objective|goal/i.test(manifestDraft);
    const hasTCF = /technical|creative|fractal/i.test(manifestDraft);
    const density = lineCount > 0 ? wordCount / lineCount : 0;

    setManifestMetrics({
      depth: Math.min(100, (lineCount / 450) * 100),
      integrity: Math.min(100, (density / 25) * 100),
      fidelity: hasIdentity ? 90 : 30,
      ethics: hasEthics ? 95 : 20,
      mission: hasMission ? 85 : 25,
      autonomy: sectionCount >= 12 ? 90 : Math.min(80, sectionCount * 7),
      complexity: Math.min(100, (wordCount / 800) * 100),
      resonance: (sectionCount / 18) * 100,
      nuance: hasTCF ? 95 : 15,
      stability: lineCount > 300 ? 90 : 60
    });
  }, [manifestDraft]);

  const sendControlFrame = useCallback((type: string, data: Record<string, unknown> = {}) => {
    if (socketRef.current?.readyState === WebSocket.OPEN && sessionIdRef.current) {
      socketRef.current.send(JSON.stringify({
        session_id: sessionIdRef.current,
        payload: { type, data },
      }));
    } else if (!sessionIdRef.current) {
      logger.warn("Dashboard", "Cannot send control frame: session not established");
    }
  }, []);

  const sendChatMessage = useCallback((role: string, content: string, recipient: string | null, broadcast: boolean = false) => {
    if (!sessionIdRef.current) {
      logger.error("Dashboard", "Cannot send message: session not established. Waiting for session.assigned event.");
      return;
    }
    if (socketRef.current?.readyState === WebSocket.OPEN) {
      const payload = { role: role === 'user' ? 'user' : 'assistant', content, recipient, broadcast };
      console.log(`[Dashboard] Sending chat message (session: ${sessionIdRef.current}, recipient: ${recipient || 'global/broadcast'}, content: "${content.substring(0, 50)}...")`);

      // Add user message to local lane messages immediately
      const laneKey = recipient ? recipient.toLowerCase() : "global";
      setLaneMessages(prev => ({
        ...prev,
        [laneKey]: [...(prev[laneKey] || []), {
          role,
          content,
          agent: 'user',
          timestamp: new Date().toISOString(),
        }]
      }));

      socketRef.current.send(JSON.stringify({
        session_id: sessionIdRef.current,
        payload
      }));
    } else {
      logger.error("Dashboard", `Cannot send message: WebSocket state is ${socketRef.current?.readyState} (expected ${WebSocket.OPEN})`);
      return;
    }
  }, []);

  const requestLaneHistory = useCallback((laneId: string, limit: number = 100) => {
    sendControlFrame("HistoryRequest", { lane_id: laneId, limit });
  }, [sendControlFrame]);

  const handleManifestSubmit = useCallback(() => {
    if (!manifestPrompt.trim()) return;
    sendControlFrame("SoulManifest", { prompt: manifestPrompt.trim(), name: manifestName.trim() || undefined });
    setIsGenerating(true);
  }, [manifestPrompt, manifestName, sendControlFrame, setIsGenerating]);

  const handleManifestCommit = useCallback(() => {
    if (!manifestDraft.trim()) return;
    const nameMatch = manifestDraft.match(/^#\s+(.*)/m) || manifestDraft.match(/\*\*Name\*\*:\s*(.*)/);
    const agentName = nameMatch ? nameMatch[1].trim().replace(/[^a-zA-Z0-9-]/g, '-').toLowerCase() : "new-agent";
    sendControlFrame("SoulUpdate", { agent_id: agentName, content: manifestDraft });
  }, [manifestDraft, sendControlFrame]);

  const value: DashboardState = {
    activeAgent, setActiveAgent,
    isManifestMode, setIsManifestMode,
    manifestPrompt, setManifestPrompt,
    manifestDraft, setManifestDraft,
    manifestMetrics, setManifestMetrics,
    manifestName, setManifestName,
    isGenerating, setIsGenerating,
    isCollapsed, setIsCollapsed,
    isRightCollapsed, setIsRightCollapsed,
    showDebug, setShowDebug,
    debugExpanded, setDebugExpanded,
    debugPaused, setDebugPaused,
    copiedId, setCopiedId,
    showSplash, setShowSplash,
    agents, setAgents,
    laneMessages, setLaneMessages,
    cognitiveInsights, setCognitiveInsights,
    debugLogs, setDebugLogs,
    streamingContent, setStreamingContent,
    streamingThoughts, setStreamingThoughts,
    connectionStatus, setConnectionStatus,
    isReady, setIsReady,
    isMounted, setIsMounted,
    isSessionReady,
    typingAgents, setTypingAgents,
    syncedLanes, setSyncedLanes,
    collapsedInsights, setCollapsedInsights,
    insightScrollRef,
    handleLaneSwitch,
    getAgentMeta,
    handleCopy,
    toggleInsightCollapse,
    scrollInsightsToTop,
    formatEst,
    sendControlFrame,
    sendChatMessage,
    requestLaneHistory,
    handleManifestSubmit,
    handleManifestCommit,
  };

  return (
    <DashboardContext.Provider value={value}>
      {children}
    </DashboardContext.Provider>
  );
}
