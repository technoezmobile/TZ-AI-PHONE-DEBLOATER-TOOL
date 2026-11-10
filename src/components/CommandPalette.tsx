import React, { useState, useEffect, useRef, useMemo } from 'react';
import {
  Search,
  Smartphone,
  Layers,
  Bot,
  History,
  Settings,
  Info,
  RefreshCw,
  Package,
  Keyboard,
  ShieldCheck,
  ArrowRight,
  X
} from 'lucide-react';
import { DevicePackage } from '../types';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onNavigate: (tab: 'devices' | 'packages' | 'ai' | 'plan' | 'history' | 'settings' | 'about', searchParam?: string) => void;
  onRescan: () => void;
  onOpenCandidateModal: () => void;
  onOpenShortcutsModal: () => void;
  packages: DevicePackage[];
}

interface CommandItem {
  id: string;
  category: 'Navigation' | 'Action' | 'Package Search' | 'Help';
  title: string;
  subtitle?: string;
  shortcut?: string;
  icon: React.ReactNode;
  run: () => void;
}

export const CommandPalette: React.FC<Props> = ({
  isOpen,
  onClose,
  onNavigate,
  onRescan,
  onOpenCandidateModal,
  onOpenShortcutsModal,
  packages,
}) => {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  const navigationCommands: CommandItem[] = [
    {
      id: 'nav-devices',
      category: 'Navigation',
      title: 'Connected Devices',
      subtitle: 'Inspect USB & Wireless ADB targets and user profiles',
      shortcut: 'Ctrl+1',
      icon: <Smartphone size={14} />,
      run: () => onNavigate('devices'),
    },
    {
      id: 'nav-packages',
      category: 'Navigation',
      title: 'Package Explorer',
      subtitle: 'Browse installed packages, debloat candidates, and vendor apps',
      shortcut: 'Ctrl+2',
      icon: <Layers size={14} />,
      run: () => onNavigate('packages'),
    },
    {
      id: 'nav-ai',
      category: 'Navigation',
      title: 'AI Assistant',
      subtitle: 'Interactive debloating planning and natural language queries',
      shortcut: 'Ctrl+3',
      icon: <Bot size={14} />,
      run: () => onNavigate('ai'),
    },
    {
      id: 'nav-plan',
      category: 'Navigation',
      title: 'Plan Review',
      subtitle: 'Review proposed actions, side effects, and verify safety',
      shortcut: 'Ctrl+4',
      icon: <ShieldCheck size={14} />,
      run: () => onNavigate('plan'),
    },
    {
      id: 'nav-history',
      category: 'Navigation',
      title: 'History & Recovery',
      subtitle: 'Audit log of executed operations and single-click restores',
      shortcut: 'Ctrl+5',
      icon: <History size={14} />,
      run: () => onNavigate('history'),
    },
    {
      id: 'nav-settings',
      category: 'Navigation',
      title: 'Settings & Credentials',
      subtitle: 'Configure AI connections, models, and ADB options',
      shortcut: 'Ctrl+6',
      icon: <Settings size={14} />,
      run: () => onNavigate('settings'),
    },
    {
      id: 'nav-about',
      category: 'Navigation',
      title: 'About & Notices',
      subtitle: 'TZ Labs information, dataset statistics, and licenses',
      shortcut: 'Ctrl+7',
      icon: <Info size={14} />,
      run: () => onNavigate('about'),
    },
  ];

  const actionCommands: CommandItem[] = [
    {
      id: 'action-rescan',
      category: 'Action',
      title: 'Rescan Device Inventory',
      subtitle: 'Query target device via ADB for fresh package states',
      shortcut: 'Ctrl+R',
      icon: <RefreshCw size={14} />,
      run: onRescan,
    },
    {
      id: 'action-candidates',
      category: 'Action',
      title: 'Open Candidate Inventory Modal',
      subtitle: 'Read-only browser across all catalog recommendation tiers',
      icon: <Layers size={14} />,
      run: onOpenCandidateModal,
    },
    {
      id: 'action-shortcuts',
      category: 'Help',
      title: 'Keyboard Shortcuts Reference',
      subtitle: 'View full cheat sheet of navigation and editor hotkeys',
      shortcut: 'Ctrl+/',
      icon: <Keyboard size={14} />,
      run: onOpenShortcutsModal,
    },
  ];

  // Matching packages based on search query
  const packageMatches: CommandItem[] = useMemo(() => {
    if (!query.trim() || query.length < 2) return [];
    const q = query.toLowerCase();
    return packages
      .filter((p) => p.package_id.toLowerCase().includes(q) || (p.description || '').toLowerCase().includes(q))
      .slice(0, 8)
      .map((p) => ({
        id: `pkg-${p.package_id}`,
        category: 'Package Search' as const,
        title: p.package_id,
        subtitle: `${p.state} · ${p.package_type} · ${p.removal_recommendation}`,
        icon: <Package size={14} style={{ color: 'var(--accent-teal)' }} />,
        run: () => onNavigate('packages', p.package_id),
      }));
  }, [query, packages, onNavigate]);

  const filteredCommands = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      return [...navigationCommands, ...actionCommands];
    }
    const navAndAction = [...navigationCommands, ...actionCommands].filter(
      (c) => c.title.toLowerCase().includes(q) || (c.subtitle && c.subtitle.toLowerCase().includes(q))
    );
    return [...navAndAction, ...packageMatches];
  }, [query, packageMatches]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev + 1) % Math.max(filteredCommands.length, 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((prev) => (prev - 1 + filteredCommands.length) % Math.max(filteredCommands.length, 1));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const item = filteredCommands[selectedIndex];
      if (item) {
        item.run();
        onClose();
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose} style={{ alignItems: 'flex-start', paddingTop: '10vh' }}>
      <div
        className="modal-box"
        onClick={(e) => e.stopPropagation()}
        style={{
          maxWidth: 580,
          width: '94%',
          background: 'var(--surface-raised)',
          border: '1px solid var(--border-medium)',
          boxShadow: '0 16px 40px rgba(0, 0, 0, 0.45)',
          padding: 0,
          overflow: 'hidden',
        }}
      >
        {/* Search Input Bar */}
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            padding: '12px 16px',
            borderBottom: '1px solid var(--border-hairline)',
            background: 'var(--surface-deep)',
          }}
        >
          <Search size={16} style={{ color: 'var(--text-muted)', flexShrink: 0 }} />
          <input
            ref={inputRef}
            type="text"
            className="input-field"
            style={{
              flex: 1,
              background: 'transparent',
              border: 'none',
              padding: 0,
              fontSize: 13,
              color: 'var(--text-primary)',
              outline: 'none',
              boxShadow: 'none',
            }}
            placeholder="Type a command, navigate, or search package ID... (Esc to close)"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            onKeyDown={handleKeyDown}
          />
          <kbd
            style={{
              fontSize: 10,
              padding: '2px 5px',
              background: 'var(--surface-raised)',
              border: '1px solid var(--border-hairline)',
              borderRadius: 'var(--radius-xs)',
              color: 'var(--text-muted)',
            }}
          >
            Esc
          </kbd>
        </div>

        {/* Command List Results */}
        <div style={{ maxHeight: 360, overflowY: 'auto', padding: '6px' }}>
          {filteredCommands.length === 0 ? (
            <div style={{ padding: '24px 16px', textAlign: 'center', color: 'var(--text-muted)', fontSize: 12 }}>
              No commands or packages match "{query}".
            </div>
          ) : (
            filteredCommands.map((item, idx) => {
              const isSelected = idx === selectedIndex;
              return (
                <div
                  key={item.id}
                  onClick={() => {
                    item.run();
                    onClose();
                  }}
                  onMouseEnter={() => setSelectedIndex(idx)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    padding: '8px 12px',
                    borderRadius: 'var(--radius-sm)',
                    background: isSelected ? 'var(--surface-deep)' : 'transparent',
                    cursor: 'pointer',
                    transition: 'background 100ms var(--ease-out)',
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10, minWidth: 0 }}>
                    <div
                      style={{
                        color: isSelected ? 'var(--accent-teal)' : 'var(--text-muted)',
                        display: 'flex',
                        alignItems: 'center',
                      }}
                    >
                      {item.icon}
                    </div>
                    <div style={{ minWidth: 0 }}>
                      <div
                        style={{
                          fontSize: 12.5,
                          fontWeight: isSelected ? 600 : 500,
                          color: isSelected ? 'var(--text-primary)' : 'var(--text-secondary)',
                          fontFamily: item.category === 'Package Search' ? 'var(--font-mono)' : 'inherit',
                          whiteSpace: 'nowrap',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                        }}
                      >
                        {item.title}
                      </div>
                      {item.subtitle && (
                        <div
                          style={{
                            fontSize: 11,
                            color: 'var(--text-muted)',
                            whiteSpace: 'nowrap',
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                          }}
                        >
                          {item.subtitle}
                        </div>
                      )}
                    </div>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
                    {item.shortcut && (
                      <kbd
                        style={{
                          fontSize: 10,
                          padding: '2px 5px',
                          background: 'var(--surface-raised)',
                          border: '1px solid var(--border-hairline)',
                          borderRadius: 'var(--radius-xs)',
                          color: 'var(--text-muted)',
                          fontFamily: 'var(--font-mono)',
                        }}
                      >
                        {item.shortcut}
                      </kbd>
                    )}
                    {isSelected && <ArrowRight size={13} style={{ color: 'var(--accent-teal)' }} />}
                  </div>
                </div>
              );
            })
          )}
        </div>

        {/* Footer info bar */}
        <div
          style={{
            padding: '6px 14px',
            borderTop: '1px solid var(--border-hairline)',
            background: 'var(--surface-deep)',
            fontSize: 11,
            color: 'var(--text-muted)',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
          }}
        >
          <span>Navigate with ↑ ↓ · Select with Enter</span>
          <span>TZ Command Palette</span>
        </div>
      </div>
    </div>
  );
};

