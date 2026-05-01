"use client";

import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import styles from "../app/page.module.css";

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

export default function FormattedContent({ content, msgId, thoughts }: { content: string, msgId?: string, thoughts?: string }) {
  const cleaned = cleanMessage(content);
  if (!cleaned && !thoughts) return null;
  return (
    <div className={styles.messageBody}>
      {thoughts && thoughts.trim() && <CollapsibleThoughts thoughts={thoughts} />}
      {cleaned && (
        <ReactMarkdown remarkPlugins={[remarkGfm]} components={{
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
                  <div className={styles.codeCopyButton} onClick={() => {
                    navigator.clipboard.writeText(codeString);
                  }}>
                    COPY
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
        }}>
          {cleaned}
        </ReactMarkdown>
      )}
    </div>
  );
}
