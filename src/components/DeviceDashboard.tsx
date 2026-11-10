import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Smartphone,
  Wifi,
  RefreshCw,
  AlertTriangle,
  Download,
  Info,
  User,
  X
} from 'lucide-react';
import { AdbStatus, AndroidUser, DeviceInfo } from '../types';

interface Props {
  adbStatus: AdbStatus | null;
  devices: DeviceInfo[];
  selectedDevice: DeviceInfo | null;
  users: AndroidUser[];
  selectedUser: AndroidUser | null;
  onSelectDevice: (device: DeviceInfo) => void;
  onSelectUser: (user: AndroidUser) => void;
  onRefresh: () => void;
}

export const DeviceDashboard: React.FC<Props> = ({
  adbStatus,
  devices,
  selectedDevice,
  users,
  selectedUser,
  onSelectDevice,
  onSelectUser,
  onRefresh,
}) => {
  const [showWirelessModal, setShowWirelessModal] = useState(false);
  const [wirelessHost, setWirelessHost] = useState('');
  const [pairingPort, setPairingPort] = useState('37000');
  const [pairingCode, setPairingCode] = useState('');
  const [connectPort, setConnectPort] = useState('5555');
  const [wirelessStatus, setWirelessStatus] = useState<string | null>(null);
  const [isPairing, setIsPairing] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const [isDownloadingTools, setIsDownloadingTools] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<{ progress: number; message: string } | null>(null);
  const [showDevGuide, setShowDevGuide] = useState(false);
  const [isOnboardingDismissed, setIsOnboardingDismissed] = useState<boolean>(() => {
    return localStorage.getItem('tz_onboarding_dismissed') === 'true';
  });

  const dismissOnboarding = () => {
    setIsOnboardingDismissed(true);
    localStorage.setItem('tz_onboarding_dismissed', 'true');
  };

  // Download platform-tools
  const handleDownloadPlatformTools = async () => {
    setIsDownloadingTools(true);
    setErrorMessage(null);
    setDownloadProgress({ progress: 0, message: 'Initiating official platform-tools download...' });

    const unlisten = await listen<{ progress: number; message: string }>(
      'platform-tools-progress',
      (event) => {
        setDownloadProgress(event.payload);
      }
    );

    try {
      await invoke('download_platform_tools');
      onRefresh();
    } catch (e: any) {
      setErrorMessage(`Platform-tools setup error: ${e?.message || e}`);
    } finally {
      unlisten();
      setIsDownloadingTools(false);
      setDownloadProgress(null);
    }
  };

  // Pair Wireless ADB
  const handlePairWireless = async () => {
    if (!wirelessHost.trim() || !pairingPort.trim() || !pairingCode.trim()) {
      setWirelessStatus('Enter Host IP, Pairing Port, and 6-digit Pairing Code');
      return;
    }
    setIsPairing(true);
    setWirelessStatus('Pairing with wireless target...');
    try {
      const msg: string = await invoke('pair_wireless_adb', {
        host: wirelessHost.trim(),
        port: parseInt(pairingPort.trim(), 10),
        code: pairingCode.trim(),
      });
      setWirelessStatus(`Success: ${msg}`);
      onRefresh();
    } catch (e: any) {
      setWirelessStatus(`Pairing failed: ${e?.message || e}`);
    } finally {
      setIsPairing(false);
    }
  };

  // Connect Wireless ADB
  const handleConnectWireless = async () => {
    if (!wirelessHost.trim() || !connectPort.trim()) {
      setWirelessStatus('Enter Host IP and Connection Port');
      return;
    }
    setIsConnecting(true);
    setWirelessStatus('Connecting to wireless device...');
    try {
      const msg: string = await invoke('connect_wireless_adb', {
        host: wirelessHost.trim(),
        port: parseInt(connectPort.trim(), 10),
      });
      setWirelessStatus(`Connected: ${msg}`);
      setShowWirelessModal(false);
      onRefresh();
    } catch (e: any) {
      setWirelessStatus(`Connection failed: ${e?.message || e}`);
    } finally {
      setIsConnecting(false);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {/* ADB Status Banner */}
      {adbStatus && !adbStatus.found && (
        <div className="card" style={{ borderLeft: '3px solid var(--status-negative)', background: 'var(--surface-card)' }}>
          <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 16 }}>
            <div style={{ display: 'flex', gap: 12 }}>
              <AlertTriangle style={{ color: 'var(--status-negative)', marginTop: 2, flexShrink: 0 }} size={18} />
              <div>
                <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>
                  Android Debug Bridge (ADB) not found
                </h3>
                <p style={{ fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.4, marginBottom: 8 }}>
                  TZ AI DEBLOATER TOOL requires official Android Platform Tools to communicate with target devices.
                </p>
                {downloadProgress && (
                  <div style={{ marginTop: 8, marginBottom: 8 }}>
                    <div style={{ height: 4, background: 'var(--surface-raised)', borderRadius: 'var(--radius-xs)', overflow: 'hidden' }}>
                      <div
                        style={{
                          height: '100%',
                          width: `${Math.round(downloadProgress.progress * 100)}%`,
                          background: 'var(--accent-teal)',
                          transition: 'width 0.2s var(--ease-out)',
                        }}
                      />
                    </div>
                    <span style={{ fontSize: 11, color: 'var(--text-muted)', marginTop: 4, display: 'inline-block' }}>
                      {downloadProgress.message}
                    </span>
                  </div>
                )}
              </div>
            </div>
            <button
              className="btn btn-primary btn-sm"
              onClick={handleDownloadPlatformTools}
              disabled={isDownloadingTools}
            >
              <Download size={13} />
              {isDownloadingTools ? 'Downloading...' : 'Install platform tools'}
            </button>
          </div>
        </div>
      )}

      {errorMessage && (
        <div className="error-banner">
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span>{errorMessage}</span>
            <button
              onClick={() => setErrorMessage(null)}
              style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
            >
              <X size={14} />
            </button>
          </div>
        </div>
      )}

      {/* Main Top Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 10 }}>
        <div>
          <h1 style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>
            Device Discovery & Targets
          </h1>
          <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 2 }}>
            Manage USB connected phones and Android 11+ wireless debugging targets
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button className="btn btn-secondary btn-sm" onClick={() => setShowWirelessModal(true)}>
            <Wifi size={13} />
            Wireless debugging
          </button>
          <button className="btn btn-secondary btn-sm" onClick={onRefresh}>
            <RefreshCw size={13} />
            Refresh devices
          </button>
        </div>
      </div>

      {/* Onboarding Checklist (Pattern 15: Skippable Local Checklist) */}
      {!isOnboardingDismissed && (
        <div className="card" style={{ padding: 14, background: 'var(--surface-deep)', border: '1px solid var(--border-hairline)' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Info size={14} style={{ color: 'var(--accent-teal)' }} />
              <strong style={{ fontSize: 12.5, color: 'var(--text-primary)' }}>Getting Started Checklist</strong>
            </div>
            <button
              onClick={dismissOnboarding}
              style={{ background: 'none', border: 'none', color: 'var(--text-muted)', fontSize: 11, cursor: 'pointer', textDecoration: 'underline' }}
            >
              Dismiss checklist
            </button>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: 8, fontSize: 11.5 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: adbStatus?.found ? 'var(--status-positive)' : 'var(--text-muted)' }}>
              <span>{adbStatus?.found ? '✓' : '○'}</span>
              <span>1. Platform Tools ADB Ready</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: selectedDevice?.state === 'Ready' ? 'var(--status-positive)' : 'var(--text-muted)' }}>
              <span>{selectedDevice?.state === 'Ready' ? '✓' : '○'}</span>
              <span>2. Phone Authorized (Ready)</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: selectedUser ? 'var(--status-positive)' : 'var(--text-muted)' }}>
              <span>{selectedUser ? '✓' : '○'}</span>
              <span>3. User Profile Selected</span>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: selectedDevice && selectedUser ? 'var(--accent-teal)' : 'var(--text-muted)' }}>
              <span>{selectedDevice && selectedUser ? '✓' : '○'}</span>
              <span>4. Ready to Debloat (Ctrl+2)</span>
            </div>
          </div>
        </div>
      )}

      {/* Devices List Grid */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))', gap: 12 }}>
        {devices.length === 0 ? (
          <div className="card" style={{ gridColumn: '1 / -1', padding: '36px 20px', textAlign: 'center' }}>
            <Smartphone size={32} style={{ color: 'var(--text-muted)', margin: '0 auto 10px auto' }} />
            <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>
              No Android devices detected
            </h3>
            <p style={{ fontSize: 12, color: 'var(--text-secondary)', maxWidth: 460, margin: '0 auto 16px auto', lineHeight: 1.5 }}>
              Connect your phone via USB cable and ensure <strong>USB Debugging</strong> is enabled in Developer Options.
            </p>
            <button className="btn btn-secondary btn-sm" onClick={() => setShowDevGuide(!showDevGuide)}>
              <Info size={13} />
              {showDevGuide ? 'Hide instructions' : 'View USB debugging instructions'}
            </button>
          </div>
        ) : (
          devices.map((dev) => {
            const isSelected = selectedDevice?.serial === dev.serial;
            const isReady = dev.state === 'Ready';

            return (
              <div
                key={dev.serial}
                className="card"
                style={{
                  borderColor: isSelected ? 'var(--accent-teal)' : 'var(--border-hairline)',
                  background: isSelected ? 'var(--surface-raised)' : 'var(--surface-card)',
                  cursor: 'pointer',
                  position: 'relative',
                  padding: 16,
                  transition: 'background-color 120ms var(--ease-out), border-color 120ms var(--ease-out)',
                }}
                onClick={() => onSelectDevice(dev)}
              >
                <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 12 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <div
                      style={{
                        width: 32,
                        height: 32,
                        borderRadius: 'var(--radius-sm)',
                        background: 'var(--surface-deep)',
                        border: '1px solid var(--border-hairline)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        color: isSelected ? 'var(--accent-teal)' : 'var(--text-muted)',
                      }}
                    >
                      <Smartphone size={16} />
                    </div>
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
                        {dev.brand ? `${dev.brand} ${dev.model}` : dev.model}
                      </div>
                      <div style={{ fontSize: 11, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                        {dev.serial}
                      </div>
                    </div>
                  </div>

                  <span className={`badge ${isReady ? 'badge-teal' : 'badge-amber'}`}>
                    {typeof dev.state === 'string' ? dev.state : JSON.stringify(dev.state)}
                  </span>
                </div>

                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6, fontSize: 11 }}>
                  <div style={{ background: 'var(--surface-deep)', padding: '6px 8px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
                    <span style={{ color: 'var(--text-muted)' }}>Android: </span>
                    <strong style={{ color: 'var(--text-primary)', fontVariantNumeric: 'tabular-nums' }}>{dev.android_version || 'N/A'}</strong>
                  </div>
                  <div style={{ background: 'var(--surface-deep)', padding: '6px 8px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
                    <span style={{ color: 'var(--text-muted)' }}>SDK: </span>
                    <strong style={{ color: 'var(--text-primary)', fontVariantNumeric: 'tabular-nums' }}>{dev.android_sdk || 'N/A'}</strong>
                  </div>
                  <div style={{ background: 'var(--surface-deep)', padding: '6px 8px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
                    <span style={{ color: 'var(--text-muted)' }}>Connection: </span>
                    <strong style={{ color: 'var(--text-primary)' }}>{dev.connection_type}</strong>
                  </div>
                  <div style={{ background: 'var(--surface-deep)', padding: '6px 8px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
                    <span style={{ color: 'var(--text-muted)' }}>Patch: </span>
                    <strong style={{ color: 'var(--text-primary)' }}>{dev.security_patch || 'N/A'}</strong>
                  </div>
                </div>

                {dev.state === 'Unauthorized' && (
                  <div
                    style={{
                      marginTop: 10,
                      padding: '8px 10px',
                      borderRadius: 'var(--radius-xs)',
                      background: 'var(--status-warning-bg)',
                      border: '1px solid var(--status-warning-border)',
                      fontSize: 11,
                      color: 'var(--status-warning)',
                      lineHeight: 1.4,
                    }}
                  >
                    <strong>Prompt on device:</strong> Unlock phone and tap <em>Allow USB Debugging</em>.
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Target Android User / Profile Selector */}
      {selectedDevice && selectedDevice.state === 'Ready' && (
        <div className="card" style={{ padding: 16 }}>
          <div className="card-header" style={{ marginBottom: 10 }}>
            <div className="card-title">
              <User size={15} style={{ color: 'var(--accent-teal)' }} />
              Target Android User Profile
            </div>
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>
              Deterministic profile target (never assumes User 0)
            </span>
          </div>

          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            {users.map((u) => {
              const isUserSelected = selectedUser?.id === u.id;
              return (
                <button
                  key={u.id}
                  onClick={() => onSelectUser(u)}
                  style={{
                    padding: '8px 12px',
                    borderRadius: 'var(--radius-sm)',
                    border: `1px solid ${isUserSelected ? 'var(--accent-teal)' : 'var(--border-hairline)'}`,
                    background: isUserSelected ? 'var(--surface-raised)' : 'var(--surface-deep)',
                    color: isUserSelected ? 'var(--accent-teal)' : 'var(--text-primary)',
                    cursor: 'pointer',
                    fontSize: 12,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    transition: 'all 120ms var(--ease-out)',
                    textAlign: 'left',
                  }}
                >
                  <span
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: '50%',
                      background: isUserSelected ? 'var(--accent-teal)' : 'var(--text-muted)',
                    }}
                  />
                  <div>
                    <div style={{ fontWeight: 500 }}>{u.display_label}</div>
                    <div style={{ fontSize: 10, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                      Profile ID: {u.id} {u.is_running ? '· Running' : ''}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}

      {/* Developer Options & USB Debugging Guide */}
      {(showDevGuide || devices.length === 0) && (
        <div className="card" style={{ padding: 16 }}>
          <div className="card-header" style={{ marginBottom: 10 }}>
            <div className="card-title">
              <Info size={15} style={{ color: 'var(--accent-teal)' }} />
              How to enable Developer Options & USB debugging
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(240px, 1fr))', gap: 10, fontSize: 12, color: 'var(--text-secondary)' }}>
            <div style={{ background: 'var(--surface-deep)', padding: '10px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
              <strong style={{ color: 'var(--text-primary)', display: 'block', marginBottom: 4 }}>1. Enable Developer Options</strong>
              Open <em>Settings → About Phone</em> and tap <strong>Build Number</strong> 7 times until you see "You are now a developer!".
            </div>
            <div style={{ background: 'var(--surface-deep)', padding: '10px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
              <strong style={{ color: 'var(--text-primary)', display: 'block', marginBottom: 4 }}>2. Enable USB Debugging</strong>
              Open <em>Settings → Developer Options</em> and toggle on <strong>USB Debugging</strong>.
            </div>
            <div style={{ background: 'var(--surface-deep)', padding: '10px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
              <strong style={{ color: 'var(--text-primary)', display: 'block', marginBottom: 4 }}>3. Authorize Computer</strong>
              Connect phone via USB. On the prompt <em>"Allow USB debugging?"</em>, check <strong>Always allow</strong> and tap <strong>Allow</strong>.
            </div>
          </div>
        </div>
      )}

      {/* Wireless ADB Modal */}
      {showWirelessModal && (
        <div className="modal-overlay" onClick={() => setShowWirelessModal(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 460, width: '100%' }}>
            <div className="modal-header">
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
                <Wifi size={16} style={{ color: 'var(--accent-teal)' }} />
                Wireless ADB Pairing & Connection
              </h3>
              <button
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                onClick={() => setShowWirelessModal(false)}
              >
                <X size={15} />
              </button>
            </div>

            <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <p style={{ fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.4, margin: 0 }}>
                Android 11+ supports wireless debugging with an official 6-digit pairing code.
                Go to <em>Developer Options → Wireless Debugging → Pair device with pairing code</em>.
              </p>

              <div>
                <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                  Device IP Address / Host
                </label>
                <input
                  type="text"
                  className="input-field"
                  style={{ width: '100%' }}
                  placeholder="e.g. 192.168.1.105"
                  value={wirelessHost}
                  onChange={(e) => setWirelessHost(e.target.value)}
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
                <div>
                  <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                    Pairing Port
                  </label>
                  <input
                    type="text"
                    className="input-field"
                    style={{ width: '100%' }}
                    placeholder="e.g. 37123"
                    value={pairingPort}
                    onChange={(e) => setPairingPort(e.target.value)}
                  />
                </div>
                <div>
                  <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                    6-digit Pairing Code
                  </label>
                  <input
                    type="text"
                    className="input-field"
                    style={{ width: '100%' }}
                    placeholder="e.g. 849201"
                    maxLength={6}
                    value={pairingCode}
                    onChange={(e) => setPairingCode(e.target.value)}
                  />
                </div>
              </div>

              <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                <button
                  className="btn btn-secondary btn-sm"
                  onClick={handlePairWireless}
                  disabled={isPairing}
                >
                  {isPairing ? 'Pairing...' : 'Step 1: Pair device'}
                </button>
              </div>

              <hr style={{ borderColor: 'var(--border-hairline)', margin: '4px 0' }} />

              <div>
                <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                  Connection Port (from main Wireless Debugging page)
                </label>
                <input
                  type="text"
                  className="input-field"
                  style={{ width: '100%' }}
                  placeholder="e.g. 5555 or 41235"
                  value={connectPort}
                  onChange={(e) => setConnectPort(e.target.value)}
                />
              </div>

              {wirelessStatus && (
                <div
                  className={`badge ${
                    wirelessStatus.startsWith('Error') || wirelessStatus.startsWith('Pairing failed') || wirelessStatus.startsWith('Connection failed')
                      ? 'badge-rose'
                      : 'badge-teal'
                  }`}
                  style={{ width: '100%', padding: '8px 10px', fontSize: 11, justifyContent: 'flex-start' }}
                >
                  {wirelessStatus}
                </div>
              )}
            </div>

            <div className="modal-footer" style={{ justifyContent: 'flex-end', gap: 8 }}>
              <button className="btn btn-secondary btn-sm" onClick={() => setShowWirelessModal(false)}>
                Cancel
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleConnectWireless}
                disabled={isConnecting}
              >
                {isConnecting ? 'Connecting...' : 'Step 2: Connect'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
