import React from 'react';
import { Info, AlertTriangle, CheckCircle2 } from 'lucide-react';

interface Props {
  text: string;
}

export const MessageText: React.FC<Props> = ({ text }) => {
  if (!text) return null;

  // Split into blocks: code blocks, tables, callouts, paragraphs
  const renderInline = (content: string) => {
    const parts = content.split(/(\*\*[^*]+\*\*|`[^`]+`)/g);
    return parts.map((part, i) => {
      if (part.startsWith('**') && part.endsWith('**')) {
        return <strong key={i}>{part.slice(2, -2)}</strong>;
      }
      if (part.startsWith('`') && part.endsWith('`')) {
        return <code key={i} className="inline-code">{part.slice(1, -1)}</code>;
      }
      return <React.Fragment key={i}>{part}</React.Fragment>;
    });
  };

  const renderCellContent = (cell: string) => {
    const trimmed = cell.trim();
    if (trimmed.includes('✓ Success')) {
      return <span className="badge badge-teal" style={{ fontSize: 10.5, padding: '2px 6px' }}>✓ Success</span>;
    }
    if (trimmed.includes('ℹ Already Removed') || trimmed.toLowerCase().includes('already in requested state') || trimmed.toLowerCase().includes('already uninstalled') || trimmed.toLowerCase().includes('already disabled')) {
      return <span className="badge badge-zinc" style={{ fontSize: 10.5, padding: '2px 6px' }}>ℹ Already Removed</span>;
    }
    if (trimmed.includes('✕ Failed') || trimmed.includes('DELETE_FAILED')) {
      return <span className="badge badge-rose" style={{ fontSize: 10.5, padding: '2px 6px' }}>✕ Failed</span>;
    }
    return renderInline(trimmed);
  };

  // Parse lines into logical blocks
  const lines = text.split('\n');
  const blocks: React.ReactNode[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // 1. Fenced Code Block
    if (line.startsWith('```')) {
      const codeLines = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // skip closing ```
      blocks.push(
        <details key={`code-${i}`} style={{ margin: '8px 0' }}>
          <summary style={{ cursor: 'pointer', fontSize: 11, color: 'var(--text-muted)' }}>
            Structured response
          </summary>
          <pre className="code-block" style={{ marginTop: 4, padding: 8, background: 'var(--surface-deep)', borderRadius: 4, fontSize: 11, overflowX: 'auto' }}>
            {codeLines.join('\n')}
          </pre>
        </details>
      );
      continue;
    }

    // 2. Table Block
    if (line.trim().startsWith('|') && line.trim().endsWith('|')) {
      const tableLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith('|') && lines[i].trim().endsWith('|')) {
        tableLines.push(lines[i].trim());
        i++;
      }

      if (tableLines.length >= 2) {
        // Line 0: Header, Line 1: Separator, Line 2+: Rows
        const headerCells = tableLines[0]
          .slice(1, -1)
          .split('|')
          .map((c) => c.trim());
        const dataLines = tableLines.slice(2);

        blocks.push(
          <div key={`table-${i}`} className="chat-table-wrapper" style={{ margin: '10px 0', overflowX: 'auto' }}>
            <table className="chat-table" style={{ width: '100%', borderCollapse: 'collapse', fontSize: 11.5 }}>
              <thead>
                <tr style={{ borderBottom: '1px solid var(--border-hairline)', background: 'var(--surface-deep)' }}>
                  {headerCells.map((h, hIdx) => (
                    <th key={hIdx} style={{ padding: '6px 10px', textAlign: 'left', fontWeight: 600, color: 'var(--text-secondary)' }}>
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {dataLines.map((rowLine, rIdx) => {
                  const cells = rowLine
                    .slice(1, -1)
                    .split('|')
                    .map((c) => c.trim());
                  return (
                    <tr key={rIdx} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                      {cells.map((cell, cIdx) => (
                        <td key={cIdx} style={{ padding: '6px 10px', verticalAlign: 'top', wordBreak: 'break-word', overflowWrap: 'anywhere' }}>
                          {renderCellContent(cell)}
                        </td>
                      ))}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        );
        continue;
      }
    }

    // 3. GitHub style Callout / Alert (> [!NOTE] or > ...)
    if (line.trim().startsWith('>')) {
      const calloutLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith('>')) {
        calloutLines.push(lines[i].replace(/^>\s?/, ''));
        i++;
      }
      const rawText = calloutLines.join('\n');
      const isNote = rawText.includes('[!NOTE]');
      const cleanContent = rawText.replace(/\[!(NOTE|WARNING|IMPORTANT)\]\s?/, '');

      blocks.push(
        <div
          key={`callout-${i}`}
          className="callout-card"
          style={{
            margin: '8px 0',
            padding: '8px 12px',
            background: 'var(--surface-deep)',
            borderLeft: `3px solid ${isNote ? 'var(--accent-teal)' : 'var(--status-warning)'}`,
            borderRadius: '0 4px 4px 0',
            fontSize: 11.5,
            color: 'var(--text-secondary)',
            display: 'flex',
            alignItems: 'flex-start',
            gap: 8,
          }}
        >
          {isNote ? <Info size={14} style={{ color: 'var(--accent-teal)', marginTop: 2, flexShrink: 0 }} /> : <AlertTriangle size={14} style={{ color: 'var(--status-warning)', marginTop: 2, flexShrink: 0 }} />}
          <div>{renderInline(cleanContent)}</div>
        </div>
      );
      continue;
    }

    // 4. Heading (### Heading)
    if (line.startsWith('### ')) {
      blocks.push(
        <h3 key={`h3-${i}`} style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)', margin: '10px 0 4px' }}>
          {renderInline(line.slice(4))}
        </h3>
      );
      i++;
      continue;
    }

    // 5. Standard paragraph / text line
    if (line.trim()) {
      blocks.push(
        <div key={`p-${i}`} style={{ margin: '3px 0', whiteSpace: 'pre-wrap', wordBreak: 'break-word', overflowWrap: 'anywhere' }}>
          {renderInline(line)}
        </div>
      );
    } else {
      // Empty line / spacer
      blocks.push(<div key={`space-${i}`} style={{ height: 4 }} />);
    }

    i++;
  }

  return <div className="message-text">{blocks}</div>;
};
