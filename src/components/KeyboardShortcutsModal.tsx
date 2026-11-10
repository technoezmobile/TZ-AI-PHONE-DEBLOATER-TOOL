import React from 'react';
import { X, Command, Keyboard } from 'lucide-react';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

export const KeyboardShortcutsModal: React.FC<Props> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  const shortcuts = [
    { key: 'Ctrl + K', desc: 'Open Command Palette' },
    { key: 'Ctrl + 1', desc: 'Navigate to Connected Devices' },
    { key: 'Ctrl + 2', desc: 'Navigate to Package Explorer' },
    { key: 'Ctrl + 3', desc: 'Navigate to AI Assistant' },
    { key: 'Ctrl + 4', desc: 'Navigate to Plan Review' },
    { key: 'Ctrl + 5', desc: 'Navigate to History & Recovery' },
    { key: 'Ctrl + 6', desc: 'Navigate to Settings' },
    { key: 'Ctrl + 7', desc: 'Navigate to About & Notices' },
    { key: 'Ctrl + R / F5', desc: 'Refresh device inventory' },
    { key: 'Ctrl + / or ?', desc: 'Open this Keyboard Shortcuts guide' },
    { key: 'Escape', desc: 'Close open dialogs, drawers, or command palette' },
    { key: 'Enter', desc: 'Send AI prompt (in composer) / Confirm action' },
    { key: 'Shift + Enter', desc: 'New line in AI prompt composer' },
  ];

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-box"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 520, width: '92%' }}
      >
        <div className="modal-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Keyboard size={16} style={{ color: 'var(--accent-teal)' }} />
            <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)', margin: 0 }}>
              Keyboard Shortcuts Guide
            </h3>
          </div>
          <button
            onClick={onClose}
            style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
          >
            <X size={15} />
          </button>
        </div>

        <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 6, maxHeight: 380, overflowY: 'auto' }}>
          <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 8 }}>
            Use these system shortcuts to quickly navigate and operate the console without touching the mouse:
          </p>

          <table className="data-table" style={{ fontSize: 11.5 }}>
            <thead>
              <tr>
                <th style={{ width: 140 }}>Shortcut</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {shortcuts.map((s) => (
                <tr key={s.key}>
                  <td>
                    <kbd
                      style={{
                        background: 'var(--surface-raised)',
                        border: '1px solid var(--border-hairline)',
                        borderRadius: 'var(--radius-xs)',
                        padding: '2px 6px',
                        fontFamily: 'var(--font-mono)',
                        fontSize: 11,
                        color: 'var(--text-primary)',
                      }}
                    >
                      {s.key}
                    </kbd>
                  </td>
                  <td style={{ color: 'var(--text-secondary)' }}>{s.desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="modal-footer" style={{ justifyContent: 'flex-end' }}>
          <button className="btn btn-secondary btn-sm" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

