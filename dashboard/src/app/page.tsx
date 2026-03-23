"use client";

import React, { useState, useEffect, useRef, useCallback, memo, Component, ReactNode } from "react";
import Link from "next/link";
import styles from "./page.module.css";
import ReactMarkdown from "react-markdown";
import { logger } from "@/lib/logger";
import SplashScreen from "@/components/SplashScreen";

// ─── Error Boundary ───────────────────────────────────────────────────
interface ErrorBoundaryProps { children: ReactNode }
interface ErrorBoundaryState { hasError: boolean; error: Error | null }

class DashboardErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("Dashboard error:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: '#050505', color: '#ff4444', fontFamily: 'monospace', flexDirection: 'column', gap: '16px' }}>
          <div style={{ fontSize: '14px', fontWeight: 900, letterSpacing: '2px' }}>SYSTEM ERROR</div>
          <div style={{ fontSize: '12px', opacity: 0.6, maxWidth: '600px', textAlign: 'center', whiteSpace: 'pre-wrap' }}>{this.state.error?.message}</div>
          <button onClick={() => window.location.reload()} style={{ background: 'var(--accent)', color: '#000', border: 'none', padding: '8px 24px', borderRadius: '8px', fontWeight: 900, cursor: 'pointer' }}>RELOAD</button>
        </div>
      );
    }
    return this.props.children;
  }
}
import remarkGfm from "remark-gfm";
import { isTauri, igniteSwarm } from "@/lib/tauri";
import dayjs from "dayjs";
import localizedFormat from "dayjs/plugin/localizedFormat";
dayjs.extend(localizedFormat);

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

const getGatewayHost = () => {
  if (typeof window === "undefined") return "127.0.0.1";
  return window.location.hostname || "127.0.0.1";
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
const GATEWAY_SESSION_ID = "dashboard-session";

const WS_RECONNECT_MAX_DELAY = 30000;
const WS_RECONNECT_BASE_DELAY = 1000;
const MAX_RENDERED_MESSAGES = 200;

export default function Dashboard() {
  const [activeAgent, setActiveAgent] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [isRightCollapsed, setIsRightCollapsed] = useState(false);
  const [inputValue, setInputValue] = useState("");
  const [laneMessages, setLaneMessages] = useState<Record<string, Message[]>>({ global: [] });
  const [agents, setAgents] = useState<Agent[]>([]);
  const [isReady, setIsReady] = useState(false);
  const [isMounted, setIsMounted] = useState(false);
  const [showSplash, setShowSplash] = useState(true);
  const [typingAgents, setTypingAgents] = useState<Set<string>>(new Set());
  const [streamingContent, setStreamingContent] = useState<Map<string, string>>(new Map());
  const [streamingThoughts, setStreamingThoughts] = useState<Map<string, string>>(new Map());
  const [connectionStatus, setConnectionStatus] = useState<'NOMINAL' | 'OFFLINE'>('OFFLINE');
  const [syncedLanes, setSyncedLanes] = useState<Set<string>>(new Set());
  const [useTauriEvents, setUseTauriEvents] = useState(false);
  const [debugLogs, setDebugLogs] = useState<{timestamp: string, message: string}[]>([]);
  const [showDebug, setShowDebug] = useState(false);
  const [copyFeedback, setCopyFeedback] = useState(false);
  const [debugExpanded, setDebugExpanded] = useState(false);
  const [debugPaused, setDebugPaused] = useState(false);
  const debugScrollRef = useRef<HTMLDivElement>(null);
  const [cognitiveInsights, setCognitiveInsights] = useState<Insight[]>([]);
  const [streamingInsights, setStreamingInsights] = useState<Map<string, string>>(new Map());
  const [collapsedInsights, setCollapsedInsights] = useState<Set<string>>(new Set());
  const [isManifestMode, setIsManifestMode] = useState(false);
  const [manifestPrompt, setManifestPrompt] = useState("");
  const [manifestDraft, setManifestDraft] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [manifestMetrics, setManifestMetrics] = useState<Record<string, number>>({
    depth: 0, integrity: 0, fidelity: 0, ethics: 0, mission: 0,
    autonomy: 0, complexity: 0, resonance: 0, nuance: 0, stability: 0
  });
  const [manifestName, setManifestName] = useState("");


  const socketRef = useRef<WebSocket | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const insightScrollRef = useRef<HTMLDivElement>(null);
  const isConnectingRef = useRef(false);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const streamingThoughtsRef = useRef<Map<string, string>>(new Map());

  // Sync streamingThoughts ref with state so handlers always read latest value
  const setStreamingThoughtsSynced = useCallback((updater: (prev: Map<string, string>) => Map<string, string>) => {
    setStreamingThoughts(prev => {
      const next = updater(prev);
      streamingThoughtsRef.current = next;
      return next;
    });
  }, []);

  // ─── cleanMessage (no component state needed, but kept for JSX access) ───

const cleanMessage = (content: string) => {
  if (!content) return "";
  let cleaned = content;
  cleaned = cleaned.replace(/(\[?\s*OPENROUTER PROCESSING\s*\]?\s*)+/gi, '');
  // Strip tool call tags (environment_details, function calls, etc.)
  cleaned = cleaned.replace(/<environment_details>[\s\S]*?<\/environment_details>/gi, '');
  cleaned = cleaned.replace(/<function=[^>]*>[\s\S]*?<\/function>/gi, '');
  cleaned = cleaned.replace(/<tool_call>[\s\S]*?<\/tool_call>/gi, '');
  cleaned = cleaned.replace(/<use_mcp_tool[\s\S]*?<\/use_mcp_tool>/gi, '');
  cleaned = cleaned.replace(/<read_file[\s\S]*?<\/read_file>/gi, '');
  cleaned = cleaned.replace(/<write_to_file[\s\S]*?<\/write_to_file>/gi, '');
  cleaned = cleaned.replace(/<execute_command[\s\S]*?<\/execute_command>/gi, '');
  // Strip thinking tags (backend may pass some through)
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

const CollapsibleThoughts = memo(({ thoughts }: { thoughts: string }) => {
  const [collapsed, setCollapsed] = useState(true);
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
CollapsibleThoughts.displayName = 'CollapsibleThoughts';

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

  const toggleInsightCollapse = (id: string) => {
    setCollapsedInsights(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

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

  const renderFormattedContent = (content: string, msgId: string = 'msg') => {
    const cleaned = cleanMessage(content);
    if (!cleaned) return null;
    return (
      <div className={styles.messageBody}>
        <ReactMarkdown 
          remarkPlugins={[remarkGfm]}
          components={{
            table: ({node, ...props}) => <table {...props} />,
            th: ({node, ...props}) => <th {...props} />,
            td: ({node, ...props}) => <td {...props} />,
            code: ({node, className, children, ...props}) => {
              const match = /language-(\w+)/.exec(className || '');
              const lang = match ? match[1] : '';
              const codeString = String(children).replace(/\n$/, '');
              const codeId = `code-${lang}-${codeString.slice(0, 50).replace(/\s/g, '')}`;
              if (!className || !className.includes('language-')) return <code style={{ background: 'rgba(0, 213, 255, 0.1)', padding: '2px 6px', borderRadius: '4px', fontSize: '13px', color: 'var(--accent)' }} {...props}>{children}</code>;
              return (
                <div className={styles.codeBlockContainer}>
                  <div className={styles.codeBlockHeader}>
                    <span>{lang || 'code'}</span>
                    <div className={styles.codeCopyButton} onClick={() => handleCopy(codeString, codeId)}>
                      {copiedId === codeId ? '✓ COPIED' : 'COPY'}
                    </div>
                  </div>
                  <pre style={{ margin: 0, padding: '16px', background: 'transparent', overflowX: 'auto' }}>
                    <code className={className} {...props}>{children}</code>
                  </pre>
                </div>
              );
            },
            p: ({node, ...props}) => <span {...props} style={{ display: 'block', marginBottom: '12px' }} />,
            h1: ({node, ...props}) => <h1 {...props} style={{ color: 'var(--accent)', borderBottom: '1px solid rgba(0, 213, 255, 0.2)', paddingBottom: '8px', marginTop: '24px', marginBottom: '16px', fontSize: '1.4rem' }} />,
            h2: ({node, ...props}) => <h2 {...props} style={{ color: 'var(--accent)', marginTop: '20px', marginBottom: '12px', fontSize: '1.2rem', borderLeft: '3px solid var(--accent)', paddingLeft: '12px' }} />,
            h3: ({node, ...props}) => <h3 {...props} style={{ opacity: 0.8, marginTop: '16px', marginBottom: '8px', fontSize: '1.1rem', textTransform: 'uppercase', letterSpacing: '1px' }} />,
            hr: ({node, ...props}) => <hr {...props} style={{ border: 'none', borderTop: '1px solid rgba(255, 255, 255, 0.1)', margin: '24px 0' }} />,
          }}
        >
          {cleaned}
        </ReactMarkdown>
      </div>
    );
  };

  const handleLaneSwitch = (laneId: string | null, isManifest: boolean = false) => {
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
    if (socketRef.current?.readyState === WebSocket.OPEN && !isManifest) {
      const laneKey = normalizedId || "global";
      if (!syncedLanes.has(laneKey)) {
        socketRef.current.send(JSON.stringify({
          session_id: GATEWAY_SESSION_ID,
          payload: { type: "HistoryRequest", data: { lane_id: laneKey, limit: 100 } }
        }));
        setSyncedLanes(prev => new Set(prev).add(laneKey));
      }
    }
  };

  useEffect(() => {
    if (scrollRef.current) {
      requestAnimationFrame(() => {
        if (scrollRef.current) {
          scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
      });
    }
  }, [laneMessages, activeAgent, streamingContent]);

  useEffect(() => {
    if (insightScrollRef.current) {
      insightScrollRef.current.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }, [cognitiveInsights]);

  const processEvent = (type: string, evData: any) => {
    if (type === "agents.discovered") {
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
      const agentId = (msg.agent_id || msg.sender || 'unknown').toLowerCase().trim();
      setTypingAgents(prev => { const next = new Set(prev); next.delete(agentId); return next; });
      setStreamingContent(prev => { const next = new Map(prev); next.delete(agentId); return next; });
      // Collect thoughts from ref (not state) to avoid stale closure
      const agentThoughts = streamingThoughtsRef.current.get(agentId);
      setStreamingThoughtsSynced(prev => { const next = new Map(prev); next.delete(agentId); return next; });
      if (msg.is_telemetry) {
        setStreamingInsights(prev => { const next = new Map(prev); next.delete(agentId); return next; });
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
          next[laneId] = [...(next[laneId] || []), { role: msg.role || 'assistant', content: content, agent: agentId, timestamp: new Date().toISOString(), thoughts: agentThoughts }];
        });
        return next;
      });
    } else if (type === "chat.chunk") {
      const chunk = evData;
      const agentId = (chunk.agent_id || '').toLowerCase().trim();
      if (chunk.is_telemetry) {
        // Thoughts — concatenate per agent (not array push) to avoid fragmenting
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
    else if (type === "swarm_insight_history") {
      const { history } = evData;
      if (Array.isArray(history)) {
        setCognitiveInsights(history.map((h: any) => ({ agent_id: h.agent_id, content: h.content, category: h.category, timestamp: h.timestamp })));
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

  const connectWebSocket = () => {
    if (isConnectingRef.current || socketRef.current) return;
    isConnectingRef.current = true;

    const socket = new WebSocket(getWsUrl());
    socketRef.current = socket;

    socket.onopen = () => {
      reconnectAttemptsRef.current = 0;
      isConnectingRef.current = false;
      
      // Send Auth frame first (required by Gateway)
      // Auth payload is a plain string, not an object
      socket.send(JSON.stringify({
        session_id: GATEWAY_SESSION_ID,
        payload: "DASHBOARD_API_KEY:savant-dev-key",
        signature: null
      }));
      
      // Then send InitialSync
      socket.send(JSON.stringify({ session_id: GATEWAY_SESSION_ID, payload: { type: "InitialSync" } }));
      setConnectionStatus('NOMINAL');
      socket.send(JSON.stringify({
        session_id: GATEWAY_SESSION_ID,
        payload: { type: "SwarmInsightHistoryRequest", data: { limit: 50 } }
      }));
      setIsReady(true);
      setIsMounted(true);
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

      // Schedule reconnect with exponential backoff
      const attempt = reconnectAttemptsRef.current;
      const delay = Math.min(WS_RECONNECT_BASE_DELAY * Math.pow(2, attempt), WS_RECONNECT_MAX_DELAY);
      reconnectAttemptsRef.current = attempt + 1;
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = setTimeout(() => connectWebSocket(), delay);
    };

    socket.onerror = () => {
      setConnectionStatus('OFFLINE');
    };

    // In Tauri mode, events come through Tauri event listener, not WebSocket
    if (!isTauri()) {
      socket.onmessage = (event) => {
        const data = event.data;
        if (!data.startsWith("EVENT:")) return;
        try {
          const prefix = data.substring(0, data.indexOf(':'));
          const rawJson = data.substring(data.indexOf(':') + 1);
          const payload = JSON.parse(rawJson);
          if (prefix === "EVENT") {
            processEvent(payload.event_type, typeof payload.payload === 'string' ? JSON.parse(payload.payload) : payload.payload);
          }
        } catch (e) {
          logger.error('WebSocket', "Failed to parse message", e, data);
        }
      };
    }
  };

  useEffect(() => {
    if (!isTauri()) {
      connectWebSocket();
      return;
    }

    setUseTauriEvents(true);
    logger.info('Tauri', 'Running in Tauri mode');

    // In Tauri mode, connect WebSocket for sending AND listen to Tauri events for receiving
    const initTauri = async () => {
      try {
        const result = await igniteSwarm();
        logger.info('Ignition', 'Swarm Ignition:', result);
        setConnectionStatus('NOMINAL');
        setIsMounted(true);
        // Connect WebSocket after swarm is ignited (Gateway is now running)
        setTimeout(() => { connectWebSocket(); }, 2000);
      } catch (e) {
        logger.error('Ignition', 'Ignition Failure:', e);
        setConnectionStatus('OFFLINE');
      }
    };
    initTauri();

    let unlisten: (() => void) | null = null;
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen('gateway-event', (event: any) => {
        try {
          const data = event.payload as string;
          if (data.startsWith("EVENT:")) {
            const ev = JSON.parse(data.substring(6));
            const evData = typeof ev.payload === 'string' ? JSON.parse(ev.payload) : ev.payload;
            processEvent(ev.event_type, evData);
          }
        } catch (e) {
          logger.error('Tauri', 'Failed to process event:', e);
        }
      }).then(u => unlisten = u).catch((e) => logger.error('Tauri', 'Failed to listen to gateway-event', e));
      listen('system-log-event', (event: any) => {
        const logMsg = event.payload as string;
        logger.debug('System', logMsg);
        setDebugLogs(prev => [{ timestamp: new Date().toISOString(), message: logMsg }, ...prev]);
      }).catch((e) => logger.error('Tauri', 'Failed to listen to system-log-event', e));
    }).catch((e) => logger.error('Tauri', 'Failed to import @tauri-apps/api/event', e));

    return () => {
      if (unlisten) unlisten();
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      if (socketRef.current) { socketRef.current.close(); socketRef.current = null; }
    };
  }, []);

  useEffect(() => {
    if (typingAgents.size === 0) return;
    const timer = setTimeout(() => { setTypingAgents(new Set()); }, 60000);
    return () => clearTimeout(timer);
  }, [typingAgents]);

  useEffect(() => {
    if (!showDebug || debugPaused || !debugScrollRef.current) return;
    debugScrollRef.current.scrollTop = 0;
  }, [debugLogs, showDebug]);

  const handleIgnite = () => {
    if (!inputValue.trim() || !socketRef.current) return;
    const normalizedRecipient = activeAgent ? activeAgent.toLowerCase().trim() : null;
    const request = { session_id: GATEWAY_SESSION_ID, payload: { role: "user", content: inputValue.trim(), recipient: normalizedRecipient } };
    socketRef.current.send(JSON.stringify(request));
    setTypingAgents(prev => { const next = new Set(prev); next.add(normalizedRecipient || "swarm"); return next; });
    setInputValue("");
  };

  const handleManifestSubmit = () => {
    if (!manifestPrompt.trim() || !socketRef.current) return;
    setIsGenerating(true);
    socketRef.current.send(JSON.stringify({
      session_id: GATEWAY_SESSION_ID,
      payload: { type: "SoulManifest", data: { prompt: manifestPrompt.trim(), name: manifestName.trim() || undefined } }
    }));
  };

  const handleManifestCommit = () => {
    if (!manifestDraft.trim() || !socketRef.current) return;
    const nameMatch = manifestDraft.match(/^#\s+(.*)/m) || manifestDraft.match(/\*\*Name\*\*:\s*(.*)/);
    const agentName = nameMatch ? nameMatch[1].trim().replace(/[^a-zA-Z0-9-]/g, '-').toLowerCase() : "new-agent";
    socketRef.current.send(JSON.stringify({
      session_id: GATEWAY_SESSION_ID,
      payload: { type: "SoulUpdate", data: { agent_id: agentName, content: manifestDraft } }
    }));
  };

  const handleBulkManifest = (agents: Record<string, unknown>[]) => {
    if (!agents || !agents.length || !socketRef.current) return;
    socketRef.current.send(JSON.stringify({
      session_id: GATEWAY_SESSION_ID,
      payload: { type: "BulkManifest", data: { agents } }
    }));
  };

  useEffect(() => {
    if (!manifestDraft) return;
    const lineCount = manifestDraft.split('\n').length;
    const wordCount = manifestDraft.split(/\s+/).length;
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

  useEffect(() => {
    if (!isMounted) return;
    const savedAgent = localStorage.getItem('activeAgent');
    if (savedAgent) setActiveAgent(savedAgent.toLowerCase().trim());

    // Auto-dismiss splash after 5 seconds if no status event
    const splashTimer = setTimeout(() => setShowSplash(false), 5000);
    return () => clearTimeout(splashTimer);
  }, [isMounted]);

  return (
    <>
      {showSplash && <SplashScreen onComplete={() => setShowSplash(false)} />}
      <DashboardErrorBoundary>
    <div className={styles.container} style={{
      '--sidebar-width': isCollapsed ? '80px' : '280px',
      '--right-sidebar-width': isRightCollapsed ? '60px' : 'min(750px, 45vw)'
    } as React.CSSProperties}>
      <aside className={`${styles.sidebar} ${isCollapsed ? styles.sidebarCollapsed : ''}`}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px', marginBottom: '1rem', width: '100%', marginTop: '0' }}>
          <div style={{ width: isCollapsed ? '40px' : '70px', height: isCollapsed ? '40px' : '70px', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--glass-bg)', borderRadius: '14px', transition: 'all 0.3s' }}>
            <img src="/img/logo.png" alt="Savant Logo" style={{ maxHeight: '80%', maxWidth: '80%', objectFit: 'contain' }} onError={(e) => { e.currentTarget.style.display = 'none'; }} />
          </div>
          {!isCollapsed && <h2 className="neon-text" style={{ fontSize: '1.2rem', margin: '4px 0 0 0', textAlign: 'center', letterSpacing: '4px', color: 'var(--accent)' }}>SAVANT</h2>}
          {!isCollapsed && <span style={{ fontSize: '9px', color: '#666', letterSpacing: '1px', fontFamily: 'monospace' }}>v1.6.0</span>}
        </div>

        <div style={{ flex: 1, overflowY: 'auto', width: '100%', paddingRight: isCollapsed ? '0' : '4px' }}>
          <div className={styles.agentTabs}>
            {!isCollapsed && <div style={{ fontSize: '11px', fontWeight: 800, letterSpacing: '2px', color: 'var(--accent)', opacity: 0.5, padding: '8px 16px 4px', textTransform: 'uppercase' }}>System</div>}
            
            <div className={`${styles.agentTab} ${activeAgent === null && !isManifestMode ? styles.agentTabActive : ''}`}
              onClick={() => handleLaneSwitch(null)}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleLaneSwitch(null); }}}
              role="button"
              tabIndex={0}
              aria-label="Swarm Broadcast"
              title="Swarm Broadcast"
              style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
              <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🌌</span>
              {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Swarm Broadcast</span>}
            </div>

            <div className={`${styles.agentTab} ${isManifestMode ? styles.agentTabActive : ''}`}
              onClick={() => handleLaneSwitch(null, true)}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleLaneSwitch(null, true); }}}
              role="button"
              tabIndex={0}
              aria-label="Manifest New Soul"
              title="Manifest New Soul"
              style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px', border: manifestDraft ? '1px solid var(--accent)' : '1px solid transparent', position: 'relative' }}>
              <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>✨</span>
              {!isCollapsed && (
                <div style={{ display: 'flex', flexDirection: 'column' }}>
                  <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px', color: 'var(--accent)' }}>Manifest Soul</span>
                  {manifestDraft && <span style={{ fontSize: '8px', opacity: 0.6, letterSpacing: '1px' }}>DRAFT_IN_PROGRESS</span>}
                </div>
              )}
            </div>

            <Link href="/tune" style={{ textDecoration: 'none', color: 'inherit' }}>
              <div className={styles.agentTab}
                role="button"
                tabIndex={0}
                aria-label="AI Fine-Tuning"
                title="AI Fine-Tuning"
                style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🎙️</span>
                {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Fine-Tuning</span>}
              </div>
            </Link>

            <Link href="/changelog" style={{ textDecoration: 'none', color: 'inherit' }}>
              <div className={styles.agentTab}
                role="button"
                tabIndex={0}
                aria-label="Changelog"
                title="Changelog"
                style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>📋</span>
                {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Changelog</span>}
              </div>
            </Link>

            {agents.length > 0 && !isCollapsed && <div style={{ fontSize: '11px', fontWeight: 800, letterSpacing: '2px', color: 'var(--accent)', opacity: 0.5, padding: '16px 16px 4px', textTransform: 'uppercase' }}>Agents</div>}
            {agents.length > 0 && isCollapsed && <div style={{ borderTop: '1px solid var(--border)', margin: '8px 0' }} />}

            {Array.isArray(agents) && agents.map((agent) => (
              <div key={agent.id} className={`${styles.agentTab} ${activeAgent === agent.id ? styles.agentTabActive : ''}`}
                onClick={() => handleLaneSwitch(agent.id)}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleLaneSwitch(agent.id); }}}
                role="button"
                tabIndex={0}
                aria-label={`Agent: ${agent.name}`}
                title={agent.name}
                style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                <div className={styles.agentAvatar} style={{
                  width: isCollapsed ? '32px' : '24px', height: isCollapsed ? '32px' : '24px',
                  border: activeAgent === agent.id ? '2px solid var(--accent)' : 'none', flexShrink: 0,
                  fontSize: '10px', fontWeight: 900, color: 'var(--accent)'
                }}>
                  <span style={{ position: 'absolute' }}>{agent.name.charAt(0).toUpperCase()}</span>
                  <img src={`${getHttpUrl()}/api/agents/${agent.id}/image?t=${Date.now()}`} alt={agent.name} onError={(e) => { e.currentTarget.style.display = 'none'; }} />
                </div>
                {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '0.5px' }}>{agent.name}</span>}
              </div>
            ))}
          </div>
        </div>

        {!isCollapsed && (
          <div style={{ marginTop: 'auto', paddingTop: '1rem', borderTop: '1px solid var(--border)' }}>
            <div className={styles.statLabel}>Swarm Capacity</div>
            <div className={styles.statValue}>{agents.length} / <span style={{ fontSize: '1.4rem' }}>∞</span></div>
          </div>
        )}
      </aside>

      <main className={styles.main}>
        <header className={styles.header}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
            <div>
              <div style={{ opacity: 0.5, fontSize: '10px', textTransform: 'uppercase', letterSpacing: '2px' }}>
                {isManifestMode ? 'Soul Manifestation Engine' : (activeAgent ? 'Agent Lane' : 'Swarm Broadcast')}
              </div>
              <div style={{ color: 'var(--accent)', fontWeight: 800, fontSize: '18px' }}>
                {isManifestMode ? (manifestName || 'SOUL MANIFESTATION') : (agents.find(a => a.id === activeAgent)?.name || (activeAgent === null ? 'SAVANT' : activeAgent?.toUpperCase() || 'SAVANT'))}
              </div>
            </div>
          </div>
          <div className={styles.glass} style={{
            padding: '6px 16px', fontSize: '11px', borderRadius: '20px',
            border: `1px solid ${connectionStatus === 'NOMINAL' ? 'var(--accent)' : '#ff4444'}`,
            color: connectionStatus === 'NOMINAL' ? 'var(--accent)' : '#ff4444',
            transition: 'all 0.3s ease', cursor: 'pointer'
          }} onClick={() => setShowDebug(!showDebug)}>
            ● {connectionStatus === 'NOMINAL' ? 'SWARM_NOMINAL' : 'SWARM_OFFLINE'}
          </div>
        </header>

        <div className={styles.content} ref={scrollRef}>
          {isManifestMode ? (
            <div className={styles.manifestDeck}>
              <div className={styles.manifestHeader}>
                <h2 style={{ color: 'var(--accent)', letterSpacing: '2px' }}>SOUL MANIFESTATION ENGINE</h2>
                <div style={{ opacity: 0.5, fontSize: '10px' }}>AAA QUALITY STANDARDS ACTIVE</div>
              </div>

              <div style={{ display: 'flex', gap: '8px', width: '100%' }}>
                <input className={styles.generatorPrompt} placeholder='Soul Name (e.g., "Prometheus")' value={manifestName} onChange={(e) => setManifestName(e.target.value)} style={{ width: '280px', flexShrink: 0 }} />
                <input className={styles.generatorPrompt} placeholder='e.g., "Manifest a business idea strategist who only operates with 0 cost..."' value={manifestPrompt} onChange={(e) => setManifestPrompt(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && handleManifestSubmit()} style={{ flex: 1 }} />
              </div>

              {isGenerating && (
                <div style={{ width: '100%', marginTop: '12px' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', fontWeight: 800, marginBottom: '4px', color: 'var(--accent)' }}>
                    <span>MANIFESTATION SEQUENCE IN PROGRESS...</span>
                    <span>{Math.round(manifestMetrics.depth)}%</span>
                  </div>
                  <div className={styles.metricBar} style={{ height: '4px', background: 'rgba(255,255,255,0.1)' }}>
                    <div className={styles.metricFill} style={{ width: `${manifestMetrics.depth}%`, height: '100%', transition: 'width 0.5s ease-out', boxShadow: '0 0 10px var(--accent)' }} />
                  </div>
                </div>
              )}

              <div className={styles.metricHud}>
                {Object.entries(manifestMetrics).map(([key, val]) => (
                  <div key={key} className={styles.metricCard}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '9px', fontWeight: 800, textTransform: 'uppercase', opacity: 0.7 }}>
                      <span>{key}</span>
                      <span>{Math.round(val)}%</span>
                    </div>
                    <div className={styles.metricBar}>
                      <div className={styles.metricFill} style={{ width: `${val}%` }} />
                    </div>
                  </div>
                ))}
              </div>

              <div className={styles.editorContainer}>
                <div className={styles.editorPane}>
                  <div className={styles.paneHeader}>
                    <span>DRAFT_BUFFER</span>
                    <span>{manifestDraft.split('\n').length} LINES</span>
                  </div>
                  <textarea className={styles.soulEditor} value={manifestDraft} onChange={(e) => setManifestDraft(e.target.value)} placeholder="The manifestation will appear here for refinement..." />
                </div>
                <div className={styles.editorPane}>
                  <div className={styles.paneHeader}>
                    <span>AAA_RATING</span>
                    <span>{manifestMetrics.depth >= 55 ? '✅ CERTIFIED' : '⚠️ DEPTH_WARNING'}</span>
                  </div>
                  <div className={styles.soulEditor} style={{ whiteSpace: 'pre-wrap', opacity: 0.8 }}>
                    {renderFormattedContent(manifestDraft || "_Awaiting manifestation generation sequence..._")}
                  </div>
                </div>
              </div>

              <div className={styles.actionRow}>
                <button className={styles.manifestButton} style={{ background: 'transparent', border: '1px solid var(--border)', color: '#fff' }} onClick={() => setIsManifestMode(false)}>ABORT</button>
                <button className={styles.manifestButton} onClick={handleManifestSubmit} disabled={isGenerating || !manifestPrompt}>{isGenerating ? "EXPLODING..." : "Generate"}</button>
                <button className={styles.manifestButton} onClick={handleManifestCommit} disabled={manifestMetrics.depth < 40} style={{ background: '#fff', color: '#000' }}>COMMIT TO REGISTRY</button>
              </div>
            </div>
          ) : (
            <>
          {(!laneMessages[activeAgent || "global"] || laneMessages[activeAgent || "global"].length === 0) && (
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <div style={{ textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', position: 'relative' }}>
                <img src="/img/logo.png" className={`${styles.logoWatermark} ${connectionStatus === 'NOMINAL' ? styles.ignitePulse : ''}`} style={{ marginBottom: '24px' }} />
                <div style={{ opacity: 0.4, fontSize: '14px', letterSpacing: '2px', color: 'var(--accent)' }}>
                  SAVANT
                </div>
                <div style={{ opacity: 0.2, fontSize: '11px', letterSpacing: '1px', marginTop: '8px' }}>
                  {connectionStatus === 'NOMINAL' ? 'SWARM ONLINE' : 'AWAITING CONNECTION'}
                </div>
              </div>
            </div>
          )}
          {(() => {
            const laneKey = activeAgent || "global";
            const allMessages = laneMessages[laneKey] || [];
            const hasMore = allMessages.length > MAX_RENDERED_MESSAGES;
            const visibleMessages = hasMore ? allMessages.slice(-MAX_RENDERED_MESSAGES) : allMessages;
            return (
              <>
                {hasMore && (
                  <div style={{ textAlign: 'center', padding: '12px' }}>
                    <button onClick={() => {/* Future: load from server */}} style={{
                      background: 'rgba(255,255,255,0.05)', border: '1px solid var(--border)', color: 'var(--accent)',
                      padding: '8px 16px', borderRadius: '8px', cursor: 'pointer', fontSize: '11px', fontWeight: 700
                    }}>
                      {allMessages.length - MAX_RENDERED_MESSAGES} earlier messages hidden
                    </button>
                  </div>
                )}
                {visibleMessages.map((msg, i) => {
            const meta = getAgentMeta(msg.agent, msg.role);
            return (
              <div key={i} className={styles.glass} style={{
                padding: '16px', width: '100%', display: 'flex', flexDirection: 'column',
                borderRadius: '12px', border: msg.role === 'assistant' ? '1px solid var(--accent)' : '1px solid var(--border)',
                background: msg.role === 'user' ? 'rgba(255, 255, 255, 0.05)' : 'rgba(0, 213, 255, 0.08)',
                boxShadow: msg.role === 'assistant' ? '0 4px 24px rgba(0, 0, 0, 0.4)' : 'none',
                marginBottom: '12px', transition: 'all 0.3s ease'
              }}>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className={styles.messageHeader}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <div className={styles.agentAvatar} style={{
                        width: '24px', height: '24px', background: msg.role === 'user' ? 'rgba(255,255,255,0.1)' : 'var(--glass-bg)',
                        display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0,
                        border: msg.role === 'assistant' ? '1px solid var(--accent)' : 'none'
                      }}>
                        {msg.role === 'user' ? (
                          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2.5"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
                        ) : (
                          meta.image ? <img src={`${meta.image}?t=cachebust`} alt={meta.name} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <span style={{ fontSize: '10px', fontWeight: 900 }}>{meta.name.charAt(0)}</span>
                        )}
                      </div>
                      <div style={{ fontSize: '10px', color: 'var(--accent)', fontWeight: 900, letterSpacing: '1.5px', opacity: 0.8 }}>{meta.name}</div>
                    </div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <div style={{ fontSize: '9px', color: 'rgba(255,255,255,0.3)', fontFamily: 'monospace' }}>{dayjs(msg.timestamp).format('hh:mm:ss A')}</div>
                      {msg.role === 'assistant' && (
                        <button onClick={() => handleCopy(msg.content, `msg-${i}`)} className={styles.copyButton}>
                          {copiedId === `msg-${i}` ? '✓ COPIED' : 'COPY'}
                        </button>
                      )}
                    </div>
                  </div>
                  {msg.thoughts && msg.thoughts.length > 0 && (
                    <CollapsibleThoughts thoughts={msg.thoughts} />
                  )}
                  <div style={{ fontSize: '15px', lineHeight: '1.6', wordBreak: 'break-word', color: '#eee', whiteSpace: 'pre-wrap', letterSpacing: '0.3px' }}>
                    {renderFormattedContent(msg.content, `msg-${i}`)}
                  </div>
                  {msg.role === 'assistant' && msg.content.includes("```json") && msg.content.includes("\"expansion_plan\"") && (
                    (() => {
                      try {
                        const jsonMatch = msg.content.match(/```json\s*([\s\S]*?)\s*```/);
                        if (jsonMatch) {
                          const plan = JSON.parse(jsonMatch[1]);
                          if (plan.expansion_plan) {
                            return (
                              <div className={styles.manifestDeck} style={{ marginTop: '16px', border: '1px solid var(--accent)', padding: '16px', borderRadius: '12px', background: 'rgba(0,0,0,0.3)' }}>
                                <div className={styles.manifestHeader} style={{ marginBottom: '12px' }}>
                                  <h3 style={{ color: 'var(--accent)', letterSpacing: '1px', fontSize: '14px', margin: 0 }}>PROPOSED SWARM EXPANSION</h3>
                                  <div style={{ fontSize: '9px', opacity: 0.5 }}>AEC_ENGINE_READY</div>
                                </div>
                                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginBottom: '16px' }}>
                                  {plan.expansion_plan.map((a: any, idx: number) => (
                                    <div key={idx} style={{ background: 'rgba(255,255,255,0.05)', padding: '8px 12px', borderRadius: '8px', border: '1px solid rgba(255,255,255,0.1)' }}>
                                      <div style={{ fontSize: '11px', fontWeight: 800, color: 'var(--accent)' }}>{a.name}</div>
                                      <div style={{ fontSize: '9px', opacity: 0.6 }}>{a.soul ? a.soul.split('\n')[0].substring(0, 30) : 'Standard Soul'}...</div>
                                    </div>
                                  ))}
                                </div>
                                <div className={styles.actionRow}>
                                  <button className={styles.manifestButton} style={{ padding: '8px 16px', fontSize: '11px' }} onClick={() => handleBulkManifest(plan.expansion_plan)}>
                                    DEPLOY {plan.expansion_plan.length} AGENTS TO SWARM
                                  </button>
                                </div>
                              </div>
                            );
                          }
                        }
                      } catch (e) { return null; }
                      return null;
                    })()
                  )}
                </div>
              </div>
            );
          })}
              </>
            );
          })()}
            {typingAgents.size > 0 && activeAgent && (
              <div className={styles.glass} style={{
                padding: '16px', width: '100%', display: 'flex', alignItems: 'center', gap: '12px',
                borderRadius: '12px', border: '1px solid var(--border)', marginBottom: '12px',
                background: 'rgba(0, 213, 255, 0.05)'
              }}>
                <div className={styles.agentAvatar} style={{ width: '24px', height: '24px', background: 'var(--glass-bg)', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
                  {(() => { const meta = getAgentMeta(activeAgent, 'assistant'); return meta.image ? <img src={`${meta.image}?t=cachebust`} alt={meta.name} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <span style={{ fontSize: '10px', fontWeight: 900 }}>{meta.name.charAt(0)}</span>; })()}
                </div>
                <div style={{ display: 'flex', gap: '4px', alignItems: 'center' }}>
                  <span className={styles.dot} />
                  <span className={styles.dot} style={{ animationDelay: '0.15s' }} />
                  <span className={styles.dot} style={{ animationDelay: '0.3s' }} />
                </div>
              </div>
            )}
            {streamingContent.size > 0 && Array.from(streamingContent.entries()).map(([agentId, content]) => {
              if (!content) return null;
              const meta = getAgentMeta(agentId, 'assistant');
              return (
                <div key={`streaming-${agentId}`} className={styles.glass} style={{
                  padding: '16px', width: '100%', display: 'flex', flexDirection: 'column',
                  borderRadius: '12px', border: '1px solid var(--accent)', marginBottom: '12px',
                  background: 'rgba(0, 213, 255, 0.08)', boxShadow: '0 4px 24px rgba(0, 0, 0, 0.4)'
                }}>
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div className={styles.messageHeader}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                        <div className={styles.agentAvatar} style={{ width: '24px', height: '24px', background: 'var(--glass-bg)', display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0, border: '1px solid var(--accent)' }}>
                          {meta.image ? <img src={`${meta.image}?t=cachebust`} alt={meta.name} style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <span style={{ fontSize: '10px', fontWeight: 900 }}>{meta.name.charAt(0)}</span>}
                        </div>
                        <div style={{ fontSize: '10px', color: 'var(--accent)', fontWeight: 900, letterSpacing: '1.5px', opacity: 0.8 }}>{meta.name}</div>
                      </div>
                    </div>
                    <div style={{ fontSize: '15px', lineHeight: '1.6', wordBreak: 'break-word', color: '#eee', whiteSpace: 'pre-wrap', letterSpacing: '0.3px' }}>
                      {renderFormattedContent(content, `streaming-${agentId}`)}
                      <span className={styles.streamingCursor} />
                    </div>
                  </div>
                </div>
              );
            })}
            <div />
          </>
        )}

        <div className={styles.scrollControls}>
          <div className={styles.scrollButton} title="Jump to Top" onClick={() => scrollRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}>▲</div>
          <div className={styles.scrollButton} title="Jump to Bottom" onClick={() => scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' })}>▼</div>
        </div>
      </div>

        <div className={styles.inputArea}>
          <div style={{ flex: 1, display: 'flex', gap: '8px', alignItems: 'center', background: 'var(--glass-bg)', borderRadius: '12px', padding: '4px 12px', border: '1px solid var(--border)' }}>
            <input type="text" placeholder={activeAgent ? `Message ${agents.find(a => a.id === activeAgent)?.name || 'Agent'}...` : "Broadcast directive to active swarm..."} className={styles.chatInput} value={inputValue} onChange={(e) => setInputValue(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && handleIgnite()} style={{ border: 'none', background: 'transparent' }} />
            <button className={styles.sendButton} onClick={handleIgnite} onMouseDown={(e) => e.currentTarget.style.transform = 'scale(0.95)'} onMouseUp={(e) => e.currentTarget.style.transform = 'scale(1)'}>Generate</button>
          </div>
        </div>
      </main>

      <aside className={`${styles.metrics} ${isRightCollapsed ? styles.metricsCollapsed : ''}`}>
        <div className={styles.collapseToggle} onClick={() => setIsRightCollapsed(!isRightCollapsed)} title={isRightCollapsed ? "Expand" : "Collapse"} style={{ top: '16px', left: isRightCollapsed ? '50%' : '16px', transform: isRightCollapsed ? 'translateX(-50%)' : 'none', right: 'auto', opacity: isRightCollapsed ? 1 : 0.5 }}>
          {isRightCollapsed ? '⇇' : '⇉'}
        </div>

        {!isRightCollapsed && (
          <>
            <div style={{ marginTop: '24px', borderTop: '1px solid var(--border)', paddingTop: '24px', flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
                <h3 className={styles.statLabel} style={{ margin: 0 }}>
                  REFLECTIONS
                </h3>
                <span style={{ fontSize: '10px', background: 'var(--accent)', color: '#000', padding: '2px 6px', borderRadius: '4px', fontWeight: 900 }}>LIVE</span>
              </div>
              
              <div className={styles.insightTimeline} ref={insightScrollRef}>
                <div className={styles.timelineGuide} />

                {cognitiveInsights.length === 0 && (
                  <div style={{ fontSize: '11px', opacity: 0.4, fontStyle: 'italic', textAlign: 'center', padding: '20px 0' }}>
                    No diary entries yet. Savant writes during heartbeat cycles.
                  </div>
                )}

                {cognitiveInsights.map((insight, idx) => {
                    const meta = getAgentMeta(insight.agent_id, 'assistant');
                    const iid = `insight-${idx}`;
                    const isCollapsed = collapsedInsights.has(iid);
                    return (
                      <div key={idx} className={styles.insightNode}>
                        <div className={styles.insightBullet} />
                        <div className={styles.insightCard}>
                          <div className={styles.insightHeader}
                            onClick={() => toggleInsightCollapse(iid)}
                            onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleInsightCollapse(iid); }}}
                            role="button"
                            tabIndex={0}
                            aria-expanded={!isCollapsed}
                            aria-label={`${meta.name} insight: ${insight.category}`}>
                            <div className={styles.insightIdentity}>
                              {meta.image ? (
                                <img src={`${meta.image}?t=cachebust`} alt={meta.name} className={styles.insightAvatar} />
                              ) : (
                                <div className={styles.insightAvatar} style={{ background: 'var(--accent)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '10px', color: '#000', fontWeight: 900 }}>
                                  {meta.name.substring(0, 1)}
                                </div>
                              )}
                              <div style={{ display: 'flex', flexDirection: 'column' }}>
                                <div style={{ fontSize: '10px', fontWeight: 900, color: '#fff', letterSpacing: '0.5px' }}>{meta.name}</div>
                                <div style={{ fontSize: '8px', opacity: 0.5 }}>{formatEst(insight.timestamp)}</div>
                              </div>
                            </div>
                            <div className={styles.insightActions}>
                              <button className={styles.insightCopy} onClick={(e) => { e.stopPropagation(); handleCopy(insight.content, iid); }}>
                                {copiedId === iid ? '✓' : 'COPY'}
                              </button>
                              <div style={{ textAlign: 'right' }}>
                                <div style={{ fontSize: '9px', fontWeight: 900, color: 'var(--accent)', opacity: 0.8 }}>{insight.category.toUpperCase()}</div>
                              </div>
                              <div style={{ marginLeft: '8px', transform: isCollapsed ? 'rotate(0deg)' : 'rotate(180deg)', transition: 'transform 0.3s' }}>▼</div>
                            </div>
                          </div>
                          <div className={`${styles.insightBody} ${isCollapsed ? styles.insightCollapsed : ''}`}>
                            {renderFormattedContent(insight.content, iid)}
                          </div>
                        </div>
                      </div>
                    );
                  })}
              </div>
              
              <div className={styles.scrollControls}>
                <div className={styles.scrollButton} onClick={() => insightScrollRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}>▲</div>
                <div className={styles.scrollButton} onClick={() => insightScrollRef.current?.scrollTo({ top: insightScrollRef.current?.scrollHeight || 0, behavior: 'smooth' })}>▼</div>
              </div>
            </div>
          </>
        )}

        {isRightCollapsed && (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '20px', marginTop: '40px' }}>
            <span title="Heartbeat">💓</span>
            <span title="Memory">💾</span>
            <span title="Context">🌐</span>
          </div>
        )}
      </aside>

      {showDebug && (
        <div style={{
          position: 'fixed', bottom: 0, left: 0, right: 0,
          height: debugExpanded ? '100vh' : '300px',
          background: 'rgba(0,0,0,0.95)', borderTop: debugExpanded ? 'none' : '2px solid var(--accent)',
          zIndex: 9999, display: 'flex', flexDirection: 'column',
          fontFamily: 'monospace', fontSize: '11px', transition: 'height 0.2s'
        }}>
          <div style={{
            padding: '8px 16px', background: '#111', borderBottom: '1px solid #333',
            display: 'flex', justifyContent: 'space-between', alignItems: 'center'
          }}>
            <span style={{ color: 'var(--accent)', fontWeight: 'bold' }}>
              🛠️ DEBUG CONSOLE ({debugLogs.length} entries) {debugPaused ? '⏸ PAUSED' : ''}
            </span>
            <div style={{ display: 'flex', gap: '8px' }}>
              <button onClick={() => {
                const text = debugLogs.map(l => `[${l.timestamp.substr(11, 12)}] ${l.message}`).join('\n');
                navigator.clipboard.writeText(text);
                setCopyFeedback(true);
                setTimeout(() => setCopyFeedback(false), 2000);
              }} style={{ background: copyFeedback ? '#00ff00' : 'var(--accent)', color: '#000', border: 'none', padding: '4px 12px', cursor: 'pointer', fontWeight: 'bold', transition: 'all 0.2s' }}>{copyFeedback ? '✓ COPIED' : 'COPY'}</button>
              <button onClick={() => setDebugExpanded(!debugExpanded)} style={{ background: '#333', color: '#fff', border: 'none', padding: '4px 12px', cursor: 'pointer' }}>{debugExpanded ? 'COLLAPSE' : 'EXPAND'}</button>
              <button onClick={() => { setShowDebug(false); setDebugExpanded(false); }} style={{ background: '#333', color: '#fff', border: 'none', padding: '4px 12px', cursor: 'pointer' }}>CLOSE</button>
            </div>
          </div>
          <div
            ref={debugScrollRef}
            onMouseUp={() => {
              const sel = window.getSelection();
              const paused = sel ? sel.toString().length > 0 : false;
              setDebugPaused(paused);
            }}
            style={{
              flex: 1, overflowY: 'auto', padding: '8px 16px',
              whiteSpace: 'pre-wrap', wordBreak: 'break-all'
            }}>
            {debugLogs.map((log, i) => {
              const levelMatch = log.message.match(/\[(ERROR|WARN|INFO|DEBUG)\]/);
              const level = levelMatch ? levelMatch[1] : 'INFO';
              const levelColor = level === 'ERROR' ? '#ff4444' : level === 'WARN' ? '#ffaa00' : level === 'DEBUG' ? '#8888ff' : '#00ff00';
              const timeStr = dayjs(log.timestamp).format('llll');
              return (
                <div key={i} style={{ lineHeight: '1.4', marginBottom: '2px', color: levelColor }}>
                  <span style={{ color: '#666' }}>[{timeStr}]</span>{' '}
                  <span style={{ color: levelColor, fontWeight: level === 'ERROR' ? 'bold' : 'normal' }}>[{level}]</span>{' '}
                  {log.message.replace(/\[(ERROR|WARN|INFO|DEBUG)\]\s*/, '')}
                </div>
              );
            })}
            {debugLogs.length === 0 && <div style={{ color: '#666' }}>No logs yet...</div>}
          </div>
        </div>
      )}
    </div>
    </DashboardErrorBoundary>
    </>
  );
}
