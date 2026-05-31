"use client";

import { useRef, useEffect, useState, ReactNode, useCallback, Component } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useDashboard } from "@/context/DashboardContext";
import styles from "../app/page.module.css";
import SplashScreen from "@/components/SplashScreen";
import { getAppVersion, getHttpUrl } from "@/lib/tauri";
import FormattedContent from "@/components/FormattedContent";

// AuthImage — fetches images with auth headers, displays as blob URL.
// Avoids 401 spam from <img src> which can't send custom headers.
function AuthImage({ src, alt, onError, style }: { src: string; alt: string; onError?: (e: React.SyntheticEvent<HTMLImageElement>) => void; style?: React.CSSProperties }) {
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [directFallback, setDirectFallback] = useState(false);
  const ctx = useDashboard();
  useEffect(() => {
    let revoked = false;
    ctx.fetchAuthImage(src).then(url => {
      if (!revoked && url) {
        setBlobUrl(url);
      } else if (!revoked) {
        setDirectFallback(true);
      }
    });
    return () => { revoked = true; };
  }, [src, ctx.fetchAuthImage]);
  if (blobUrl) return <img src={blobUrl} alt={alt} onError={onError} style={style} />;
  if (directFallback) return <img src={src} alt={alt} onError={onError} style={style} />;
  return null;
}
import SetupWizard from "@/components/SetupWizard";

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



// Page header configuration
const PAGE_HEADERS: Record<string, { sub: string; title: string }> = {
  '/tune': { sub: 'AI Configuration', title: 'FINE-TUNING' },
  '/changelog': { sub: 'System Updates', title: 'CHANGELOG' },
  '/settings': { sub: 'System Configuration', title: 'SETTINGS' },
  '/marketplace': { sub: 'Agent Marketplace', title: 'MARKETPLACE' },
  '/mcp': { sub: 'Model Context Protocol', title: 'MCP SERVERS' },
  '/health': { sub: 'System Diagnostics', title: 'SYSTEM HEALTH' },
  '/faq': { sub: 'Frequently Asked Questions', title: 'FAQ' },
  '/browser': { sub: 'Web Navigation', title: 'BROWSER' },
};

export default function DashboardShell({ children }: { children: ReactNode }) {
  const ctx = useDashboard();
  const {
    activeAgent,
    isManifestMode,
    agents,
    connectionStatus,
    isSessionReady,
    showSplash,
    setShowSplash,
    showSetupWizard,
    setShowSetupWizard,
    setIsManifestMode,
    isCollapsed,
    setIsCollapsed,
    isRightCollapsed,
    setIsRightCollapsed,
    showDebug,
    setShowDebug,
    debugExpanded,
    setDebugExpanded,
    debugPaused,
    setDebugPaused,
    debugLogs,
    cognitiveInsights,
    collapsedInsights,
    toggleInsightCollapse,
    scrollInsightsToTop,
    insightScrollRef,
    handleCopy,
    setCopiedId,
    copiedId,
  } = ctx;

  const scrollRef = useRef<HTMLDivElement>(null);
  const debugScrollRef = useRef<HTMLDivElement>(null);
  const [inputValue, setInputValue] = useState("");
  const pathname = usePathname();
  const showChatInput = pathname === '/' && !isManifestMode;

  // System is ready only when WebSocket connected, session assigned, and app mounted
  const isSystemReady = connectionStatus === 'NOMINAL' && isSessionReady && ctx.isMounted;

  // Gate: show loading screen after splash, before system is ready
  // Auto-dismiss after 15s or allow manual skip
  const [forceSkip, setForceSkip] = useState(false);
  const showSystemLoading = !showSplash && !isSystemReady && !forceSkip;

  // Dynamic version from Tauri (reads tauri.conf.json at runtime — no env var needed)
  const [appVersion, setAppVersion] = useState<string>("0.0.0");
  useEffect(() => {
    getAppVersion().then(setAppVersion).catch(() => {});
  }, []);

  useEffect(() => {
    if (!showSystemLoading) return;
    const timer = setTimeout(() => setForceSkip(true), 15000);
    return () => clearTimeout(timer);
  }, [showSystemLoading]);

  const handleIgnite = useCallback(() => {
    if (!inputValue.trim()) return;
    ctx.sendChatMessage(
      'user',
      inputValue.trim(),
      ctx.activeAgent,
      !ctx.activeAgent
    );
    setInputValue("");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inputValue]);

  // Auto-scroll main content
  useEffect(() => {
    if (scrollRef.current) {
      requestAnimationFrame(() => {
        if (scrollRef.current) {
          scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
      });
    }
  }, [ctx.laneMessages, ctx.activeAgent, ctx.streamingContent]);

  // Auto-scroll insights on update
  useEffect(() => {
    if (insightScrollRef.current) {
      insightScrollRef.current.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }, [cognitiveInsights]);

  // Auto-dismiss splash, then show setup wizard if first run
  useEffect(() => {
    if (!ctx.isMounted) return;
    const splashTimer = setTimeout(() => {
      setShowSplash(false);
      // Show setup wizard after splash dismisses
      if (!localStorage.getItem("savant.setup.complete")) {
        setShowSetupWizard(true);
      }
    }, 5000);
    return () => clearTimeout(splashTimer);
  }, [ctx.isMounted, setShowSplash, setShowSetupWizard]);

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

  // Determine header content based on current page
  const pageHeader = PAGE_HEADERS[pathname];
  const headerSubtitle = pageHeader
    ? pageHeader.sub
    : isManifestMode
      ? 'Soul Manifestation Engine'
      : activeAgent
        ? 'Agent Lane'
        : 'Swarm Broadcast';
  const headerTitle = pageHeader
    ? pageHeader.title
    : isManifestMode
      ? (ctx.manifestName || 'SOUL MANIFESTATION')
      : (activeAgent ? ctx.getAgentMeta(activeAgent, 'assistant').name : 'SAVANT');

  return (
    <>
      {showSplash && <SplashScreen onComplete={() => setShowSplash(false)} />}
      
      {/* System Loading Gate — shown after splash, before backend is ready */}
      {showSystemLoading && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: 9998,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: '#050505', flexDirection: 'column', gap: '16px'
        }}>
          <img src="/img/logo.png" alt="Savant" style={{ width: '80px', height: '80px', objectFit: 'contain', opacity: 0.8 }} />
          <div style={{ color: 'var(--accent)', fontSize: '14px', fontWeight: 900, letterSpacing: '3px' }}>SAVANT</div>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginTop: '16px' }}>
            <div style={{
              width: '16px', height: '16px', border: '2px solid rgba(0,213,255,0.3)',
              borderTop: '2px solid var(--accent)', borderRadius: '50%',
              animation: 'spin 1s linear infinite'
            }} />
            <span style={{ fontSize: '12px', color: 'rgba(255,255,255,0.5)', letterSpacing: '1px' }}>
              {ctx.ignitionError ? `Startup error: ${ctx.ignitionError}` :
               connectionStatus !== 'NOMINAL' ? 'Connecting to gateway...' :
               !isSessionReady ? 'Establishing session...' :
               'Initializing...'}
            </span>
          </div>
          <button
            onClick={() => setForceSkip(true)}
            style={{
              marginTop: '24px', padding: '8px 20px', background: 'transparent',
              color: '#555', border: '1px solid #333', borderRadius: '6px',
              cursor: 'pointer', fontSize: '11px', letterSpacing: '1px'
            }}
          >
            Skip to Dashboard
          </button>
          <style>{`@keyframes spin { to { transform: rotate(360deg); } }`}</style>
        </div>
      )}
      
      <DashboardErrorBoundary>
      <div className={styles.container} style={{
        '--sidebar-width': isCollapsed ? '80px' : '280px',
        '--right-sidebar-width': isRightCollapsed ? '60px' : 'min(750px, 45vw)'
      } as React.CSSProperties}>

        {/* ===== LEFT SIDEBAR (Grid child 1) ===== */}
        <aside className={`${styles.sidebar} ${isCollapsed ? styles.sidebarCollapsed : ''}`}>
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '8px', marginBottom: '1rem', width: '100%', marginTop: '0' }}>
            <div style={{ width: isCollapsed ? '40px' : '70px', height: isCollapsed ? '40px' : '70px', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--glass-bg)', borderRadius: '14px', transition: 'all 0.3s' }}>
              <img src="/img/logo.png" alt="Savant Logo" style={{ maxHeight: '80%', maxWidth: '80%', objectFit: 'contain' }} onError={(e) => { e.currentTarget.style.display = 'none'; }} />
            </div>
            {!isCollapsed && <h2 className="neon-text" style={{ fontSize: '1.2rem', margin: '4px 0 0 0', textAlign: 'center', letterSpacing: '4px', color: 'var(--accent)' }}>SAVANT</h2>}
            {!isCollapsed && <span style={{ fontSize: '9px', color: '#666', letterSpacing: '1px', fontFamily: 'monospace' }}>{`v${appVersion}`}</span>}
          </div>

          <div style={{ flex: 1, overflowY: 'auto', width: '100%', paddingRight: isCollapsed ? '0' : '4px' }}>
            <div className={styles.agentTabs}>
              {!isCollapsed && <div style={{ fontSize: '11px', fontWeight: 800, letterSpacing: '2px', color: 'var(--accent)', opacity: 0.5, padding: '8px 16px 4px', textTransform: 'uppercase' }}>System</div>}
              
              <div className={`${styles.agentTab} ${activeAgent === null && !isManifestMode ? styles.agentTabActive : ''}`}
                onClick={() => ctx.handleLaneSwitch(null)}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); ctx.handleLaneSwitch(null); }}}
                role="button"
                tabIndex={0}
                aria-label="Swarm Broadcast"
                title="Swarm Broadcast"
                style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🌌</span>
                {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Swarm Broadcast</span>}
              </div>

              <div className={`${styles.agentTab} ${isManifestMode ? styles.agentTabActive : ''}`}
                onClick={() => ctx.handleLaneSwitch(null, true)}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); ctx.handleLaneSwitch(null, true); }}}
                role="button"
                tabIndex={0}
                aria-label="Manifest New Soul"
                title="Manifest New Soul"
                style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px', border: ctx.manifestDraft ? '1px solid var(--accent)' : '1px solid transparent', position: 'relative' }}>
                <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>✨</span>
                {!isCollapsed && (
                  <div style={{ display: 'flex', flexDirection: 'column' }}>
                    <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px', color: 'var(--accent)' }}>Manifest Soul</span>
                    {ctx.manifestDraft && <span style={{ fontSize: '8px', opacity: 0.6, letterSpacing: '1px' }}>DRAFT_IN_PROGRESS</span>}
                  </div>
                )}
              </div>

              <Link href="/evolution" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="Personality Evolution"
                  title="Personality Evolution"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🧬</span>
                  {!isCollapsed && (
                    <div style={{ display: 'flex', flexDirection: 'column' }}>
                      <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px', color: 'var(--accent)' }}>Evolution</span>
                      {ctx.proposedMutations?.length > 0 && <span style={{ fontSize: '8px', opacity: 0.6, letterSpacing: '1px' }}>{ctx.proposedMutations.length} PENDING</span>}
                    </div>
                  )}
                </div>
              </Link>

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

              <Link href="/settings" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="Settings"
                  title="Settings"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>⚙️</span>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Settings</span>}
                </div>
              </Link>

              <Link href="/marketplace" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="Marketplace"
                  title="Marketplace"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🏪</span>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Marketplace</span>}
                </div>
              </Link>

              <Link href="/mcp" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="MCP"
                  title="MCP"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🔌</span>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>MCP</span>}
                </div>
              </Link>

              <Link href="/health" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="Health"
                  title="Health"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>💓</span>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Health</span>}
                </div>
              </Link>

              <Link href="/faq" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="FAQ"
                  title="FAQ"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>❓</span>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>FAQ</span>}
                </div>
              </Link>

              <Link href="/browser" style={{ textDecoration: 'none', color: 'inherit' }}>
                <div className={styles.agentTab}
                  role="button"
                  tabIndex={0}
                  aria-label="Browser"
                  title="Browser"
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <span style={{ fontSize: isCollapsed ? '20px' : '16px' }}>🌐</span>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '1px' }}>Browser</span>}
                </div>
              </Link>

              {ctx.agents.length > 0 && !isCollapsed && <div style={{ fontSize: '11px', fontWeight: 800, letterSpacing: '2px', color: 'var(--accent)', opacity: 0.5, padding: '16px 16px 4px', textTransform: 'uppercase' }}>Agents</div>}
              {ctx.agents.length > 0 && isCollapsed && <div style={{ borderTop: '1px solid var(--border)', margin: '8px 0' }} />}

              {Array.isArray(ctx.agents) && ctx.agents.map((agent) => {
                const agentMeta = ctx.getAgentMeta(agent.id, 'assistant');
                return (
                <div key={agent.id} className={`${styles.agentTab} ${activeAgent === agent.id ? styles.agentTabActive : ''}`}
                  onClick={() => ctx.handleLaneSwitch(agent.id)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); ctx.handleLaneSwitch(agent.id); }}}
                  role="button"
                  tabIndex={0}
                  aria-label={`Agent: ${agentMeta.name}`}
                  title={agentMeta.name}
                  style={{ display: 'flex', flexDirection: isCollapsed ? 'column' : 'row', alignItems: 'center', gap: '12px', padding: isCollapsed ? '12px 0' : '10px 16px' }}>
                  <div className={styles.agentAvatar} style={{
                    width: isCollapsed ? '32px' : '24px', height: isCollapsed ? '32px' : '24px',
                    border: activeAgent === agent.id ? '2px solid var(--accent)' : 'none', flexShrink: 0,
                    fontSize: '10px', fontWeight: 900, color: 'var(--accent)'
                  }}>
                    <span style={{ position: 'absolute' }}>{agentMeta.name.charAt(0)}</span>
                    <AuthImage src={agentMeta.image || `${getHttpUrl()}/api/agents/${agent.id}/image`} alt={agentMeta.name} onError={(e) => { e.currentTarget.style.display = 'none'; }} />
                  </div>
                  {!isCollapsed && <span style={{ fontSize: '13px', fontWeight: 600, letterSpacing: '0.5px' }}>{agentMeta.name}</span>}
                </div>
                );
              })}
            </div>
          </div>

          {!isCollapsed && (
            <div style={{ marginTop: 'auto', paddingTop: '1rem', borderTop: '1px solid var(--border)' }}>
              <div className={styles.statLabel}>Swarm Capacity</div>
              <div className={styles.statValue}>{ctx.agents.length} / <span style={{ fontSize: '1.4rem' }}>∞</span></div>
            </div>
          )}
        </aside>

        {/* ===== MAIN COLUMN (Grid child 2) ===== */}
        <main className={styles.main}>
          <header className={styles.header}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
              <div>
                <div style={{ opacity: 0.5, fontSize: '10px', textTransform: 'uppercase', letterSpacing: '2px' }}>
                  {headerSubtitle}
                </div>
                <div style={{ color: 'var(--accent)', fontWeight: 800, fontSize: '18px' }}>
                  {headerTitle}
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
            {children}
          </div>

          {/* CHAT INPUT — inside <main>, below scrollable content. Always visible when showChatInput, disabled when not ready. */}
          {showChatInput && (
            <div className={styles.inputArea} style={{ opacity: isSystemReady ? 1 : 0.5, pointerEvents: isSystemReady ? 'auto' : 'none' }}>
              <div style={{ flex: 1, display: 'flex', gap: '12px', alignItems: 'center', background: 'var(--glass-bg)', borderRadius: '14px', padding: '8px 16px', border: '1px solid var(--border)', minHeight: '48px' }}>
                <input 
                  type="text" 
                  placeholder={isSystemReady ? (activeAgent ? `Message ${ctx.getAgentMeta(activeAgent, 'assistant').name}...` : "Broadcast directive to active swarm...") : "Waiting for swarm..."}
                  className={styles.chatInput} 
                  value={inputValue} 
                  onChange={(e) => setInputValue(e.target.value)} 
                  onKeyDown={(e) => e.key === 'Enter' && handleIgnite()} 
                  style={{ border: 'none', background: 'transparent' }}
                  disabled={!isSystemReady}
                />
                <button className={styles.sendButton} onClick={handleIgnite} onMouseDown={(e) => e.currentTarget.style.transform = 'scale(0.95)'} onMouseUp={(e) => e.currentTarget.style.transform = 'scale(1)'} disabled={!isSystemReady}>Generate</button>
              </div>
            </div>
          )}
        </main>

        {/* ===== RIGHT SIDEBAR (Grid child 3) — always present, content conditional ===== */}
        <aside className={`${styles.metrics} ${isRightCollapsed ? styles.metricsCollapsed : ''}`}>
          <div className={styles.collapseToggle} onClick={() => setIsRightCollapsed(!isRightCollapsed)} title={isRightCollapsed ? "Expand" : "Collapse"} style={{ top: '16px', left: isRightCollapsed ? '50%' : '16px', transform: isRightCollapsed ? 'translateX(-50%)' : 'none', right: 'auto', opacity: isRightCollapsed ? 1 : 0.5 }}>
            {isRightCollapsed ? '⇇' : '⇉'}
          </div>

          {!isRightCollapsed && activeAgent && (
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
                      const meta = ctx.getAgentMeta(insight.agent_id, 'assistant');
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
                                  <AuthImage src={meta.image} alt={meta.name} style={{ width: '20px', height: '20px', borderRadius: '50%', objectFit: 'cover', border: '1px solid var(--accent)', boxShadow: '0 0 5px var(--accent)' }} />
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
                              <FormattedContent content={insight.content} msgId={iid} />
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
        </aside>

      </div>
      </DashboardErrorBoundary>

      {/* DEBUG CONSOLE OVERLAY */}
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
              <button onClick={() => handleCopy(debugLogs.map(l => `[${l.timestamp.substr(11, 12)}] ${l.message}`).join("\n"), "debug")} style={{ background: copiedId === 'debug' ? '#00ff00' : 'var(--accent)', color: '#000', border: 'none', padding: '4px 12px', cursor: 'pointer', fontWeight: 'bold', transition: 'all 0.2s' }}>{copiedId === 'debug' ? '✓ COPIED' : 'COPY'}</button>
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
              const timeStr = formatEst(log.timestamp);
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
      {/* Setup Wizard — shown on first run after splash */}
      {showSetupWizard && (
        <SetupWizard onComplete={() => setShowSetupWizard(false)} />
      )}
    </>
  );
}
