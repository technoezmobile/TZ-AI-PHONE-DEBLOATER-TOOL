import React, { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Smartphone,
  Layers,
  Bot,
  History,
  Settings,
  Info,
  RefreshCw,
  ShieldCheck,
  User,
  AlertCircle,
  AlertTriangle,
  Menu,
  X,
  PanelLeftClose,
  PanelLeftOpen,
  Command,
  Keyboard
} from 'lucide-react';
import {
  ActionPlan,
  AdbStatus,
  AndroidUser,
  AppConfig,
  DeviceInfo,
  DevicePackage,
  OperationOutcome
} from './types';
import { DeviceDashboard } from './components/DeviceDashboard';
import { PackageExplorer } from './components/PackageExplorer';
import { AiAssistant } from './components/AiAssistant';
import { ActionPlanReview } from './components/ActionPlanReview';
import { HistoryRecovery } from './components/HistoryRecovery';
import { SettingsView } from './components/SettingsView';
import { AboutNotices } from './components/AboutNotices';
import { CommandPalette } from './components/CommandPalette';
import { KeyboardShortcutsModal } from './components/KeyboardShortcutsModal';
import { CandidateListModal } from './components/CandidateListModal';

const getInitialRouteState = (): {
  tab: 'devices' | 'packages' | 'ai' | 'plan' | 'history' | 'settings' | 'about';
  unknownRouteWarning: string | null;
} => {
  if (typeof window === 'undefined') return { tab: 'devices', unknownRouteWarning: null };
  const params = new URLSearchParams(window.location.search);
  const rawTab = params.get('tab');
  if (!rawTab) return { tab: 'devices', unknownRouteWarning: null };
  const tab = rawTab.toLowerCase();
  if (tab === 'packages' || tab === 'package') return { tab: 'packages', unknownRouteWarning: null };
  if (tab === 'ai') return { tab: 'ai', unknownRouteWarning: null };
  if (tab === 'plan' || tab === 'partial_failure') return { tab: 'plan', unknownRouteWarning: null };
  if (tab === 'history' || tab === 'journal') return { tab: 'history', unknownRouteWarning: null };
  if (tab === 'settings') return { tab: 'settings', unknownRouteWarning: null };
  if (tab === 'about') return { tab: 'about', unknownRouteWarning: null };
  if (tab === 'devices' || tab === 'device') return { tab: 'devices', unknownRouteWarning: null };
  return {
    tab: 'devices',
    unknownRouteWarning: `The requested view "${rawTab}" was not recognized. Restored Connected Devices console.`,
  };
};

const getInitialPlan = (): ActionPlan | null => {
  if (typeof window === 'undefined') return null;
  const params = new URLSearchParams(window.location.search);
  const tab = params.get('tab')?.toLowerCase();
  if (tab === 'plan' || tab === 'partial_failure') {
    return {
      id: 'plan-poco-debloat-2026',
      revision: 1,
      target_device_serial: 'MOCK-DEVICE-SERIAL-001',
      target_user_id: 0,
      explanation: 'Remove non-essential Facebook services and disable MIUI advertising services on User 0 (Owner).',
      preserved_functionality: ['Phone calls and SMS', 'Camera and Gallery', 'Default HyperOS navigation'],
      actions: [
        {
          package_id: 'com.facebook.appmanager',
          package_name: 'Facebook App Manager',
          proposed_action: 'UninstallUser',
          reason: 'Pre-installed background app updater and telemetry service.',
          evidence: 'Carrier bloatware running background analytics without user account',
          expected_side_effects: null,
          uncertainty: 'Low',
          removal_category: 'Recommended',
          is_protected: false,
          protection_warning: null,
          selected: true,
          is_executable: true,
        },
        {
          package_id: 'com.miui.msa.global',
          package_name: 'MIUI System Ads',
          proposed_action: 'Disable',
          reason: 'Pushes promotional push notifications and banner ads across system UI.',
          evidence: 'Confirmed advertising daemon on global MIUI / HyperOS builds',
          expected_side_effects: null,
          uncertainty: 'Low',
          removal_category: 'Recommended',
          is_protected: false,
          protection_warning: null,
          selected: true,
          is_executable: true,
        },
        {
          package_id: 'com.miui.analytics',
          package_name: 'MIUI Analytics',
          proposed_action: 'Disable',
          reason: 'Xiaomi telemetry and usage analytics service.',
          evidence: 'Non-critical background diagnostic daemon',
          expected_side_effects: null,
          uncertainty: 'Low',
          removal_category: 'Recommended',
          is_protected: false,
          protection_warning: null,
          selected: true,
          is_executable: true,
        },
        {
          package_id: 'com.google.android.youtube',
          package_name: 'YouTube',
          proposed_action: 'Disable',
          reason: 'Google YouTube pre-installed client.',
          evidence: 'Can be re-enabled or substituted with browser player',
          expected_side_effects: 'YouTube links will open in web browser',
          uncertainty: 'Medium',
          removal_category: 'Advanced',
          is_protected: false,
          protection_warning: null,
          selected: false,
          is_executable: true,
        },
      ],
      created_at: 1731170000000,
      approved_by_user: false,
      is_valid: true,
      invalidation_reason: null,
    };
  }
  return null;
};

export const App: React.FC = () => {
  const initialRoute = getInitialRouteState();
  const [activeTab, setActiveTab] = useState<'devices' | 'packages' | 'ai' | 'plan' | 'history' | 'settings' | 'about'>(initialRoute.tab);
  const [routeRecoveryNotice, setRouteRecoveryNotice] = useState<string | null>(initialRoute.unknownRouteWarning);
  const [isSidebarCompact, setIsSidebarCompact] = useState<boolean>(() => {
    if (typeof window === 'undefined') return false;
    return localStorage.getItem('tz_sidebar_compact') === 'true';
  });
  const [isMobileDrawerOpen, setIsMobileDrawerOpen] = useState<boolean>(false);

  // Command Palette & Global Modals
  const [isCommandPaletteOpen, setIsCommandPaletteOpen] = useState(false);
  const [isShortcutsModalOpen, setIsShortcutsModalOpen] = useState(false);
  const [isCandidateModalOpen, setIsCandidateModalOpen] = useState(false);

  const toggleSidebarCompact = () => {
    setIsSidebarCompact((prev) => {
      const next = !prev;
      localStorage.setItem('tz_sidebar_compact', String(next));
      return next;
    });
  };

  // Device & Connection State
  const [adbStatus, setAdbStatus] = useState<AdbStatus | null>(null);
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);
  const [users, setUsers] = useState<AndroidUser[]>([]);
  const [selectedUser, setSelectedUser] = useState<AndroidUser | null>(null);

  // Packages State
  const [packages, setPackages] = useState<DevicePackage[]>([]);
  const [isScanningPackages, setIsScanningPackages] = useState(false);
  const isScanningRef = useRef<boolean>(false);
  const scanSequenceRef = useRef<number>(0);

  // App Configuration
  const [config, setConfig] = useState<AppConfig>({
    custom_adb_path: null,
    active_connection_id: 'direct-gemini',
    connections: [],
    ai_consent_given: false,
    approved_destinations: [],
    auto_refresh_on_connect: true,
    confirm_each_action: true,
  });

  // Action Plan Review
  const [reviewingPlan, setReviewingPlan] = useState<ActionPlan | null>(getInitialPlan);
  const [lastExecutionReport, setLastExecutionReport] = useState<{
    plan: ActionPlan;
    outcomes: OperationOutcome[];
  } | null>(null);
  const [scanVerificationError, setScanVerificationError] = useState<string | null>(null);

  const handlePlanExecuted = async (outcomes?: OperationOutcome[], plan?: ActionPlan) => {
    // 1. Invalidate stale plan context
    setReviewingPlan(null);

    // 2. Attach execution report to conversation thread
    if (outcomes && plan) {
      setLastExecutionReport({ plan, outcomes });
    }

    // 3. Immediately refresh inventory for exact device and user profile
    const serial = plan?.target_device_serial || selectedDevice?.serial;
    const userId = plan?.target_user_id ?? selectedUser?.id;

    if (serial && userId !== undefined && userId !== null) {
      setIsScanningPackages(true);
      try {
        const pkgs: DevicePackage[] = await invoke('scan_packages', {
          deviceSerial: serial,
          userId,
        });
        setPackages(pkgs);
        setScanVerificationError(null);
      } catch (e: any) {
        console.error('Post-batch inventory refresh failed:', e);
        setScanVerificationError(
          `Device inventory refresh failed: ${e?.message || e}. A manual rescan is required before proposing another plan.`
        );
      } finally {
        setIsScanningPackages(false);
      }
    }

    refreshDevices();
  };

  // Load Initial Configuration
  const loadConfig = async () => {
    try {
      const cfg: AppConfig = await invoke('get_app_config');
      setConfig(cfg);
    } catch (e) {
      console.error('Failed to load app config:', e);
    }
  };

  // Check ADB Status
  const checkAdb = async () => {
    try {
      const status: AdbStatus = await invoke('get_adb_status');
      setAdbStatus(status);
      return status;
    } catch (e) {
      console.error('Failed to get ADB status:', e);
      return null;
    }
  };

  // Scan Devices with stable identity to prevent unnecessary re-renders
  const refreshDevices = useCallback(async () => {
    await checkAdb();
    try {
      const devs: DeviceInfo[] = await invoke('list_devices');
      setDevices(devs);

      // Maintain selection or select first ready device using stable identity
      setSelectedDevice((prev) => {
        if (!prev) {
          return devs.find((d) => d.state === 'Ready') || devs[0] || null;
        }
        const stillExists = devs.find((d) => d.serial === prev.serial);
        if (!stillExists) {
          return devs.find((d) => d.state === 'Ready') || devs[0] || null;
        }
        // Preserve existing object reference if identity & properties are identical
        if (
          stillExists.serial === prev.serial &&
          stillExists.state === prev.state &&
          stillExists.model === prev.model &&
          stillExists.brand === prev.brand &&
          stillExists.android_version === prev.android_version &&
          stillExists.connection_type === prev.connection_type
        ) {
          return prev;
        }
        return stillExists;
      });
    } catch (e) {
      console.error('Device listing failed:', e);
    }
  }, []);

  // Synchronize Active Target with Backend Session
  useEffect(() => {
    invoke('set_active_target', {
      serial: selectedDevice?.state === 'Ready' ? selectedDevice.serial : null,
      userId: selectedUser?.id ?? null,
    }).catch(console.error);
  }, [selectedDevice?.serial, selectedDevice?.state, selectedUser?.id]);

  // Load Users when device changes (Preserving explicit user profile selection)
  useEffect(() => {
    let cancelled = false;
    if (!selectedDevice || selectedDevice.state !== 'Ready') {
      setUsers([]);
      setSelectedUser(null);
      setPackages([]);
      return;
    }

    invoke<AndroidUser[]>('list_users', { deviceSerial: selectedDevice.serial })
      .then((uList) => {
        if (cancelled) return;
        setUsers(uList);
        if (uList.length > 0) {
          setSelectedUser((prev) => {
            if (prev) {
              const stillValid = uList.find((u) => u.id === prev.id);
              if (stillValid) {
                if (stillValid.name === prev.name && stillValid.is_running === prev.is_running) {
                  return prev; // Stable reference
                }
                return stillValid;
              }
            }
            return uList.find((u) => u.id === 0) || uList[0] || null;
          });
        } else {
          setSelectedUser(null);
        }
      })
      .catch((e) => {
        if (!cancelled) {
          console.error('Failed to list users:', e);
          setUsers([]);
          setSelectedUser(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [selectedDevice?.serial, selectedDevice?.state]);

  // Scan Packages for selected device & user with race-condition & overlap prevention
  const scanPackages = useCallback(async () => {
    const serial = selectedDevice?.serial;
    const userId = selectedUser?.id;
    if (!serial || selectedDevice?.state !== 'Ready' || userId === undefined || userId === null) {
      return;
    }

    if (isScanningRef.current) {
      return;
    }

    isScanningRef.current = true;
    const currentSeq = ++scanSequenceRef.current;
    setIsScanningPackages(true);

    try {
      const pkgs: DevicePackage[] = await invoke('scan_packages', {
        deviceSerial: serial,
        userId,
      });
      if (currentSeq === scanSequenceRef.current) {
        setPackages(pkgs);
      }
    } catch (e) {
      if (currentSeq === scanSequenceRef.current) {
        console.error('Package scan failed:', e);
        setPackages([]);
      }
    } finally {
      isScanningRef.current = false;
      if (currentSeq === scanSequenceRef.current) {
        setIsScanningPackages(false);
      }
    }
  }, [selectedDevice?.serial, selectedDevice?.state, selectedUser?.id]);

  // Trigger package scan on stable scalar dependencies
  const activeSerial = selectedDevice?.serial;
  const activeState = selectedDevice?.state;
  const activeUserId = selectedUser?.id;

  useEffect(() => {
    if (activeSerial && activeState === 'Ready' && activeUserId !== undefined && activeUserId !== null) {
      scanPackages();
    } else {
      setPackages([]);
    }
  }, [activeSerial, activeState, activeUserId, scanPackages]);

  useEffect(() => {
    if (!config.auto_refresh_on_connect) return;
    let busy = false;
    const timer = setInterval(async () => {
      if (busy) return;
      busy = true;
      try {
        await refreshDevices();
      } finally {
        busy = false;
      }
    }, 5000);
    return () => clearInterval(timer);
  }, [refreshDevices, config.auto_refresh_on_connect]);

  // Initial load
  useEffect(() => {
    loadConfig();
    refreshDevices();
  }, []);

  // Save Config handler
  const handleSaveConfig = async (updated: AppConfig) => {
    try {
      await invoke('save_app_config', { newConfig: updated });
      setConfig(updated);
      refreshDevices();
    } catch (e: any) {
      console.error(`Failed to save settings: ${e?.message || e}`);
      throw e;
    }
  };

  const handleUpdateConsent = (consent: boolean) => {
    const updated = { ...config, ai_consent_given: consent };
    handleSaveConfig(updated);
  };

  const handleTabChange = (tab: 'devices' | 'packages' | 'ai' | 'plan' | 'history' | 'settings' | 'about') => {
    setActiveTab(tab);
    setIsMobileDrawerOpen(false);
  };

  // Global Keyboard Shortcuts (UX Pattern 13)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isInput =
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.tagName === 'SELECT' ||
          target.isContentEditable);

      // Ctrl + K: Toggle Command Palette (UX Pattern 3)
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setIsCommandPaletteOpen((prev) => !prev);
        return;
      }

      // Ctrl + /: Toggle Keyboard Shortcuts Guide
      if ((e.ctrlKey || e.metaKey) && e.key === '/') {
        e.preventDefault();
        setIsShortcutsModalOpen((prev) => !prev);
        return;
      }

      // ?: Open Shortcuts Guide when not typing in form field
      if (!isInput && e.key === '?') {
        e.preventDefault();
        setIsShortcutsModalOpen((prev) => !prev);
        return;
      }

      // Escape: Dismiss active overlays/drawers
      if (e.key === 'Escape') {
        if (isCommandPaletteOpen) {
          setIsCommandPaletteOpen(false);
          return;
        }
        if (isShortcutsModalOpen) {
          setIsShortcutsModalOpen(false);
          return;
        }
        if (isCandidateModalOpen) {
          setIsCandidateModalOpen(false);
          return;
        }
        if (isMobileDrawerOpen) {
          setIsMobileDrawerOpen(false);
          return;
        }
      }

      // Ctrl + 1-7 Navigation Hotkeys
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey) {
        if (e.key === '1') {
          e.preventDefault();
          handleTabChange('devices');
        } else if (e.key === '2') {
          e.preventDefault();
          handleTabChange('packages');
        } else if (e.key === '3') {
          e.preventDefault();
          handleTabChange('ai');
        } else if (e.key === '4') {
          if (reviewingPlan) {
            e.preventDefault();
            handleTabChange('plan');
          }
        } else if (e.key === '5') {
          e.preventDefault();
          handleTabChange('history');
        } else if (e.key === '6') {
          e.preventDefault();
          handleTabChange('settings');
        } else if (e.key === '7') {
          e.preventDefault();
          handleTabChange('about');
        } else if (e.key.toLowerCase() === 'r') {
          // Prevent browser page reload; trigger package & device refresh
          e.preventDefault();
          if (selectedDevice?.state === 'Ready') {
            scanPackages();
          }
          refreshDevices();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [
    isCommandPaletteOpen,
    isShortcutsModalOpen,
    isCandidateModalOpen,
    isMobileDrawerOpen,
    reviewingPlan,
    selectedDevice?.state,
    scanPackages,
    refreshDevices,
  ]);

  return (
    <div className="app-container">
      {/* Mobile Drawer Backdrop */}
      {isMobileDrawerOpen && (
        <div className="sidebar-backdrop" onClick={() => setIsMobileDrawerOpen(false)} />
      )}

      {/* Sidebar Navigation (Responsive Rail + Full Mode + Drawer) */}
      <aside className={`sidebar ${isSidebarCompact ? 'sidebar-compact' : ''} ${isMobileDrawerOpen ? 'sidebar-drawer-open' : ''}`}>
        <div className="sidebar-header">
          <div className="sidebar-brand-group">
            <div className="logo-badge">TZ</div>
            <div className="logo-text-group">
              <div className="logo-title">TZ AI DEBLOATER</div>
              <div className="logo-sub">Desktop Tool</div>
            </div>
          </div>
          <button
            className="sidebar-toggle-btn"
            onClick={toggleSidebarCompact}
            title={isSidebarCompact ? 'Expand navigation' : 'Collapse navigation to rail'}
            aria-label={isSidebarCompact ? 'Expand navigation' : 'Collapse navigation to rail'}
          >
            {isSidebarCompact ? <PanelLeftOpen size={14} /> : <PanelLeftClose size={14} />}
          </button>
        </div>

        <ul className="sidebar-nav">
          <li>
            <button
              className={`nav-item ${activeTab === 'devices' ? 'active' : ''}`}
              onClick={() => handleTabChange('devices')}
              title="Devices"
            >
              <Smartphone className="nav-icon" size={16} />
              <span>Devices</span>
              {devices.filter((d) => d.state === 'Ready').length > 0 && (
                <span className="nav-badge">
                  {devices.filter((d) => d.state === 'Ready').length}
                </span>
              )}
            </button>
          </li>

          <li>
            <button
              className={`nav-item ${activeTab === 'packages' ? 'active' : ''}`}
              onClick={() => handleTabChange('packages')}
              title="Package Explorer"
            >
              <Layers className="nav-icon" size={16} />
              <span>Package Explorer</span>
              {packages.length > 0 && <span className="nav-badge">{packages.length}</span>}
            </button>
          </li>

          <li>
            <button
              className={`nav-item ${activeTab === 'ai' ? 'active' : ''}`}
              onClick={() => handleTabChange('ai')}
              title="AI Assistant"
            >
              <Bot className="nav-icon" size={16} />
              <span>AI Assistant</span>
            </button>
          </li>

          {reviewingPlan && (
            <li>
              <button
                className={`nav-item ${activeTab === 'plan' ? 'active' : ''}`}
                onClick={() => handleTabChange('plan')}
                title="Review Plan"
              >
                <ShieldCheck className="nav-icon" size={16} />
                <span>Review Plan</span>
                <span className="nav-badge tag-warning">Pending</span>
              </button>
            </li>
          )}

          <li>
            <button
              className={`nav-item ${activeTab === 'history' ? 'active' : ''}`}
              onClick={() => handleTabChange('history')}
              title="Journal & Recovery"
            >
              <History className="nav-icon" size={16} />
              <span>Journal & Recovery</span>
            </button>
          </li>

          <li>
            <button
              className={`nav-item ${activeTab === 'settings' ? 'active' : ''}`}
              onClick={() => handleTabChange('settings')}
              title="Settings"
            >
              <Settings className="nav-icon" size={16} />
              <span>Settings</span>
            </button>
          </li>

          <li>
            <button
              className={`nav-item ${activeTab === 'about' ? 'active' : ''}`}
              onClick={() => handleTabChange('about')}
              title="About & Notices"
            >
              <Info className="nav-icon" size={16} />
              <span>About & Notices</span>
            </button>
          </li>
        </ul>

        <div className="sidebar-footer">
          <div>
            ADB Bridge:{' '}
            <span className={adbStatus?.found ? 'badge badge-teal' : 'badge badge-rose'}>
              {adbStatus?.found ? 'Operational' : 'Missing'}
            </span>
          </div>
          <div style={{ marginTop: 2 }}>
            Target: <code>{selectedDevice ? selectedDevice.serial : 'None'}</code>
          </div>
          <div style={{ marginTop: 2, color: 'var(--text-muted)' }}>v1.1.0 · GPL-3.0</div>
        </div>
      </aside>

      {/* Main Content Area */}
      <div className="main-wrapper">
        {/* Top Persistent Target Strip */}
        <header className="top-bar">
          <div className="top-bar-left">
            <button
              className="drawer-hamburger-btn"
              onClick={() => setIsMobileDrawerOpen(!isMobileDrawerOpen)}
              title="Open navigation menu"
              aria-label="Open navigation menu"
            >
              <Menu size={15} />
            </button>

            {selectedDevice ? (
              <div className="target-strip">
                <span style={{ color: 'var(--text-muted)' }}>Target:</span>
                <span className="target-model-brand">
                  {selectedDevice.brand ? `${selectedDevice.brand} ${selectedDevice.model}` : selectedDevice.model}
                </span>
                <span className="target-serial-code">{selectedDevice.serial}</span>
                <span className={selectedDevice.state === 'Ready' ? 'badge badge-teal' : 'badge badge-amber'}>
                  {typeof selectedDevice.state === 'string' ? selectedDevice.state : 'Unknown'}
                </span>
                {selectedUser && (
                  <span className="badge badge-zinc">
                    <User size={11} /> {selectedUser.display_label}
                  </span>
                )}
              </div>
            ) : (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: 'var(--text-muted)', fontSize: 12 }}>
                <AlertCircle size={14} style={{ color: 'var(--status-warning)' }} />
                <span>No active target device. Connect phone via USB or Wireless ADB.</span>
              </div>
            )}
          </div>

          <div className="top-bar-right" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => setIsCommandPaletteOpen(true)}
              title="Command Palette (Ctrl+K)"
              style={{ display: 'inline-flex', alignItems: 'center', gap: 6, fontSize: 11.5 }}
            >
              <Command size={11} />
              <span>Commands</span>
              <kbd
                style={{
                  background: 'var(--surface-deep)',
                  border: '1px solid var(--border-hairline)',
                  borderRadius: 3,
                  padding: '1px 4px',
                  fontSize: 10,
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-muted)',
                }}
              >
                Ctrl+K
              </kbd>
            </button>

            <button
              className="btn btn-secondary btn-sm"
              onClick={() => setIsShortcutsModalOpen(true)}
              title="Keyboard Shortcuts Guide (Ctrl+/ or ?)"
              style={{ padding: '5px 8px' }}
            >
              <Keyboard size={12} />
            </button>

            {selectedDevice && selectedDevice.state === 'Ready' && (
              <button
                className="btn btn-secondary btn-sm"
                onClick={scanPackages}
                disabled={isScanningPackages}
              >
                <RefreshCw size={12} className={isScanningPackages ? 'spin' : ''} />
                {isScanningPackages ? 'Scanning packages...' : 'Rescan packages'}
              </button>
            )}
          </div>
        </header>

        {routeRecoveryNotice && (
          <div
            className="card"
            style={{
              margin: '10px 16px 0',
              padding: '8px 14px',
              borderRadius: 4,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              background: 'var(--status-caution-bg)',
              border: '1px solid var(--status-caution-border)',
              color: 'var(--text-primary)',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12 }}>
              <AlertCircle size={14} style={{ color: 'var(--status-caution)' }} />
              <span>{routeRecoveryNotice}</span>
            </div>
            <button
              onClick={() => setRouteRecoveryNotice(null)}
              style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', display: 'flex', alignItems: 'center' }}
              title="Dismiss notice"
            >
              <X size={13} />
            </button>
          </div>
        )}

        {scanVerificationError && (
          <div className="error-banner" style={{ margin: '10px 16px 0', borderRadius: 4 }}>
            <div className="error-banner-header">
              <AlertTriangle size={15} />
              <span>Inventory State Verification Block</span>
            </div>
            <div>{scanVerificationError}</div>
            <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
              <button className="btn btn-secondary btn-sm" onClick={scanPackages} disabled={isScanningPackages}>
                <RefreshCw size={11} className={isScanningPackages ? 'spin' : ''} />
                Rescan device packages
              </button>
            </div>
          </div>
        )}

        {/* Content Pane */}
        <main className="content-pane">
          {activeTab === 'devices' && (
            <DeviceDashboard
              adbStatus={adbStatus}
              devices={devices}
              selectedDevice={selectedDevice}
              users={users}
              selectedUser={selectedUser}
              onSelectDevice={(d) => setSelectedDevice(d)}
              onSelectUser={(u) => setSelectedUser(u)}
              onRefresh={refreshDevices}
            />
          )}

          {activeTab === 'packages' && (
            <PackageExplorer
              device={selectedDevice}
              user={selectedUser}
              packages={packages}
              isLoading={isScanningPackages}
              onRefresh={scanPackages}
            />
          )}

          {config.connections.length > 0 && (
            <div style={{ display: activeTab === 'ai' ? 'contents' : 'none' }}>
              <AiAssistant
                device={selectedDevice}
                user={selectedUser}
                packages={packages}
                config={config}
                lastExecutionReport={lastExecutionReport}
                onOpenPlanReview={(plan) => {
                  setReviewingPlan(plan);
                  setActiveTab('plan');
                }}
                onSelectPackage={() => {
                  setActiveTab('packages');
                }}
                onUpdateConsent={handleUpdateConsent}
                onUpdateConfig={handleSaveConfig}
              />
            </div>
          )}

          {activeTab === 'plan' && reviewingPlan && (
            <ActionPlanReview
              plan={reviewingPlan}
              device={selectedDevice}
              user={selectedUser}
              packages={packages}
              onClose={() => {
                setReviewingPlan(null);
                setActiveTab('ai');
              }}
              onExecutionCompleted={handlePlanExecuted}
            />
          )}

          {activeTab === 'history' && (
            <HistoryRecovery
              device={selectedDevice}
              user={selectedUser}
              onRefresh={scanPackages}
              onOpenPlan={(plan) => {
                setReviewingPlan(plan);
                setActiveTab('plan');
              }}
            />
          )}

          {activeTab === 'settings' && (
            <SettingsView
              config={config}
              onSaveConfig={handleSaveConfig}
              onRefreshDevices={refreshDevices}
            />
          )}

          {activeTab === 'about' && <AboutNotices />}
        </main>
      </div>

      {/* Floating Modal for Action Plan Review if reviewing plan outside plan tab */}
      {reviewingPlan && activeTab !== 'plan' && (
        <ActionPlanReview
          plan={reviewingPlan}
          device={selectedDevice}
          user={selectedUser}
          packages={packages}
          onClose={() => setReviewingPlan(null)}
          onExecutionCompleted={handlePlanExecuted}
        />
      )}

      {/* Global Command Palette (Ctrl+K - UX Pattern 3) */}
      <CommandPalette
        isOpen={isCommandPaletteOpen}
        onClose={() => setIsCommandPaletteOpen(false)}
        onNavigate={(tab) => {
          handleTabChange(tab);
          setIsCommandPaletteOpen(false);
        }}
        onRescan={() => {
          if (selectedDevice?.state === 'Ready') {
            scanPackages();
          }
          refreshDevices();
          setIsCommandPaletteOpen(false);
        }}
        onOpenCandidateModal={() => {
          setIsCommandPaletteOpen(false);
          setIsCandidateModalOpen(true);
        }}
        onOpenShortcutsModal={() => {
          setIsCommandPaletteOpen(false);
          setIsShortcutsModalOpen(true);
        }}
        packages={packages}
      />

      {/* Global Keyboard Shortcuts Guide Modal (Ctrl+/ - UX Pattern 13) */}
      <KeyboardShortcutsModal
        isOpen={isShortcutsModalOpen}
        onClose={() => setIsShortcutsModalOpen(false)}
      />

      {/* Candidates List Modal */}
      {isCandidateModalOpen && (
        <CandidateListModal
          isOpen={isCandidateModalOpen}
          onClose={() => setIsCandidateModalOpen(false)}
          device={selectedDevice}
          user={selectedUser}
          packages={packages}
        />
      )}
    </div>
  );
};

export default App;
