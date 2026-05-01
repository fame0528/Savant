"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useDashboard } from "@/context/DashboardContext";
import styles from "./page.module.css";
import FormattedContent from "@/components/FormattedContent";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

const MAX_RENDERED_MESSAGES = 200;

export default function ChatPage() {
  const ctx = useDashboard();
  const {
    activeAgent,
    isManifestMode,
    laneMessages,
    streamingContent,
    streamingThoughts,
    typingAgents,
    agents,
    manifestPrompt,
    setManifestPrompt,
    manifestDraft,
    setManifestDraft,
    manifestName,
    setManifestName,
    isGenerating,
    setIsGenerating,
    manifestMetrics,
    setManifestMetrics,
    cognitiveInsights,
    handleLaneSwitch,
    handleCopy,
    getAgentMeta,
    sendControlFrame,
    requestLaneHistory,
    formatEst,
    showDebug,
    setShowDebug,
    copiedId,
    setCopiedId,
    connectionStatus,
  } = ctx;

  const scrollRef = useRef<HTMLDivElement>(null);
  const insightScrollRef = useRef<HTMLDivElement>(null);
  const [collapsedInsights, setCollapsedInsights] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (scrollRef.current) {
      requestAnimationFrame(() => {
        if (scrollRef.current) {
          scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
      });
    }
  }, [laneMessages, activeAgent, streamingContent]);

  const toggleInsightCollapse = useCallback((id: string) => {
    setCollapsedInsights(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const handleManifestSubmit = () => {
    if (!manifestPrompt.trim()) return;
    sendControlFrame("SoulManifest", { prompt: manifestPrompt.trim(), name: manifestName.trim() || undefined });
    setIsGenerating(true);
  };

  const handleManifestCommit = () => {
    if (!manifestDraft.trim()) return;
    const nameMatch = manifestDraft.match(/^#\s+(.*)/m) || manifestDraft.match(/\*\*Name\*\*:\s*(.*)/);
    const agentName = nameMatch ? nameMatch[1].trim().replace(/[^a-zA-Z0-9-]/g, '-').toLowerCase() : "new-agent";
    sendControlFrame("SoulUpdate", { agent_id: agentName, content: manifestDraft });
  };

  const handleBulkManifest = (agents: Record<string, unknown>[]) => {
    if (!agents || !agents.length) return;
    sendControlFrame("BulkManifest", { agents });
  };

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

  if (isManifestMode) {
    return (
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
              <FormattedContent content={manifestDraft || "_Awaiting manifestation generation sequence..._"} />
            </div>
          </div>
        </div>

        <div className={styles.actionRow}>
          <button className={styles.manifestButton} style={{ background: 'transparent', border: '1px solid var(--border)', color: '#fff' }} onClick={() => handleLaneSwitch(null, false)}>ABORT</button>
          <button className={styles.manifestButton} onClick={handleManifestSubmit} disabled={isGenerating || !manifestPrompt}>{isGenerating ? "EXPLODING..." : "Generate"}</button>
          <button className={styles.manifestButton} onClick={handleManifestCommit} disabled={manifestMetrics.depth < 40} style={{ background: '#fff', color: '#000' }}>COMMIT TO REGISTRY</button>
        </div>
      </div>
    );
  }

  // CHAT VIEW
  const laneKey = activeAgent || "global";
  const allMessages = laneMessages[laneKey] || [];
  const hasMore = allMessages.length > MAX_RENDERED_MESSAGES;
  const visibleMessages = hasMore ? allMessages.slice(-MAX_RENDERED_MESSAGES) : allMessages;

  return (
    <>
      {(!allMessages || allMessages.length === 0) && (
        <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <div style={{ textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', position: 'relative' }}>
            <img src="/img/logo.png" className={`${styles.logoWatermark} ${connectionStatus === 'NOMINAL' ? styles.ignitePulse : ''}`} style={{ marginBottom: '24px' }} />
            <div style={{ opacity: 0.4, fontSize: '14px', letterSpacing: '2px', color: 'var(--accent)' }}>
              {activeAgent ? (ctx.agents.find(a => a.id === activeAgent)?.name?.toUpperCase() || activeAgent?.toUpperCase()) : 'SAVANT'}
            </div>
            <div style={{ opacity: 0.2, fontSize: '11px', letterSpacing: '1px', marginTop: '8px' }}>
              {connectionStatus === 'NOMINAL' ? (
                <span style={{ display: 'flex', alignItems: 'center', gap: '8px', justifyContent: 'center' }}>
                  <span style={{
                    width: '10px', height: '10px', border: '1.5px solid rgba(0,213,255,0.3)',
                    borderTop: '1.5px solid var(--accent)', borderRadius: '50%',
                    animation: 'spin 1s linear infinite', display: 'inline-block'
                  }} />
                  Loading conversation...
                </span>
              ) : 'AWAITING CONNECTION'}
            </div>
          </div>
        </div>
      )}

      {(() => {
        return (
          <>
            {hasMore && (
              <div style={{ textAlign: 'center', padding: '12px' }}>
                <button onClick={() => requestLaneHistory(laneKey, 1000)} style={{
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
                        <div style={{ fontSize: '9px', color: 'rgba(255,255,255,0.3)', fontFamily: 'monospace' }}>{(typeof msg.timestamp === 'string' ? new Date(msg.timestamp) : new Date()).toLocaleTimeString()}</div>
                        {msg.role === 'assistant' && (
                          <button onClick={() => handleCopy(msg.content, `msg-${i}`)} className={styles.copyButton}>
                            {copiedId === `msg-${i}` ? '✓ COPIED' : 'COPY'}
                          </button>
                        )}
                      </div>
                    </div>
                    <div style={{ fontSize: '15px', lineHeight: '1.6', wordBreak: 'break-word', color: '#eee', whiteSpace: 'pre-wrap', letterSpacing: '0.3px' }}>
                      <FormattedContent content={msg.content} msgId={`msg-${i}`} thoughts={msg.thoughts} />
                    </div>
                  </div>
                </div>
              );
            })}

            {Array.from(streamingContent.entries()).map(([agentId, content]) => {
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
                      <FormattedContent content={content} msgId={`streaming-${agentId}`} />
                      <span className={styles.streamingCursor} />
                    </div>
                  </div>
                </div>
              );
            })}
          </>
        );
      })()}
    </>
  );
}
