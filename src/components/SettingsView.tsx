import React, { useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Save,
  Lock,
  Plus,
  Trash2,
  RefreshCw,
  CheckCircle2,
  AlertTriangle,
  Activity
} from 'lucide-react';
import {
  AppConfig,
  ConnectionProfile,
  ConnectionMode,
  ApiProtocol,
  GenerationTestResult,
  GatewayReadinessInfo,
  ModelDiscoveryResult,
  ClassifiedError
} from '../types';

interface Props {
  config: AppConfig;
  onSaveConfig: (updated: AppConfig) => Promise<void>;
  onRefreshDevices: () => void;
}

export const SettingsView: React.FC<Props> = ({ config, onSaveConfig, onRefreshDevices }) => {
  const [formData, setFormData] = useState<AppConfig>({ ...config });
  const [activeConnIndex, setActiveConnIndex] = useState<number>(() => {
    const idx = config.connections.findIndex((c) => c.id === config.active_connection_id);
    return idx >= 0 ? idx : 0;
  });

  // Credential input state (ephemeral, never stored in state permanently)
  const [enteredSecret, setEnteredSecret] = useState('');
  const [credOpStatus, setCredOpStatus] = useState<string | null>(null);

  const discoverySequence = useRef(0);
  const [modelSearch, setModelSearch] = useState('');
  // Testing & Discovery States
  const [isTestingCred, setIsTestingCred] = useState(false);
  const [testResult, setTestResult] = useState<GenerationTestResult | null>(null);
  const [testError, setTestError] = useState<ClassifiedError | null>(null);

  const [isDiscovering, setIsDiscovering] = useState(false);
  const [discoveryResult, setDiscoveryResult] = useState<ModelDiscoveryResult | null>(null);

  const [isCheckingReadiness, setIsCheckingReadiness] = useState(false);
  const [readinessInfo, setReadinessInfo] = useState<GatewayReadinessInfo | null>(null);

  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccessMsg, setSaveSuccessMsg] = useState<string | null>(null);

  const activeConn: ConnectionProfile | undefined = formData.connections[activeConnIndex];

  const updateActiveConn = (patch: Partial<ConnectionProfile>) => {
    setFormData((prev) => {
      const nextConnections = [...prev.connections];
      if (nextConnections[activeConnIndex]) {
        const invalidates = ['base_url', 'protocol', 'selected_model', 'manual_model_id'].some((k) => k in patch);
        nextConnections[activeConnIndex] = {
          ...nextConnections[activeConnIndex],
          ...patch,
          ...(invalidates ? { tested_for_generation: false, last_verified_at: null } : {}),
        };
      }
      return { ...prev, connections: nextConnections };
    });
  };

  // Add new Custom Gateway Profile
  const handleAddProfile = () => {
    const newId = `profile-${Date.now()}`;
    const newProfile: ConnectionProfile = {
      id: newId,
      name: 'Custom Gateway Profile',
      mode: 'Gateway',
      preset: 'CustomGateway',
      base_url: 'http://localhost:8000/v1',
      protocol: 'OpenAiChatCompletions',
      credential_id: newId,
      selected_model: '',
      manual_model_id: null,
      last_test_status: null,
      last_verified_at: null,
      supported_capabilities: ['chat', 'streaming', 'model_listing'],
      tested_for_generation: false,
      has_credential: false,
    };

    setFormData((prev) => ({
      ...prev,
      connections: [...prev.connections, newProfile],
    }));
    setActiveConnIndex(formData.connections.length);
    setEnteredSecret('');
    setDiscoveryResult(null);
  };

  // Delete profile
  const handleDeleteProfile = (index: number) => {
    if (formData.connections.length <= 1) {
      alert('At least one connection profile must be retained');
      return;
    }
    const target = formData.connections[index];
    if (confirm(`Remove connection profile "${target.name}"?`)) {
      setFormData((prev) => {
        const next = prev.connections.filter((_, i) => i !== index);
        return {
          ...prev,
          connections: next,
          active_connection_id: next[0].id,
        };
      });
      setActiveConnIndex(0);
    }
  };

  // OS Credential Actions
  const handleSaveCredential = async () => {
    if (!activeConn) return;
    if (!enteredSecret.trim()) {
      setCredOpStatus('Enter an API key or token first');
      return;
    }

    try {
      setCredOpStatus('Saving to Windows Credential Manager...');
      await invoke('save_credential', {
        connectionId: activeConn.credential_id,
        secret: enteredSecret.trim(),
      });
      setCredOpStatus('Saved securely in Windows Credential Manager');
      setEnteredSecret('');
      updateActiveConn({ has_credential: true });
    } catch (e: any) {
      setCredOpStatus(`Error saving credential: ${e}`);
    }
  };

  const handleRemoveCredential = async () => {
    if (!activeConn) return;
    if (confirm(`Remove saved credential for "${activeConn.name}" from Windows Credential Manager?`)) {
      try {
        await invoke('remove_credential', { connectionId: activeConn.credential_id });
        updateActiveConn({ has_credential: false });
        setCredOpStatus('Credential removed from Windows Credential Manager');
      } catch (e: any) {
        setCredOpStatus(`Error removing credential: ${e}`);
      }
    }
  };

  const handleTestSaved = async () => {
    if (!activeConn) return;
    setIsTestingCred(true);
    setTestResult(null);
    setTestError(null);

    try {
      await onSaveConfig(formData);
      const res: GenerationTestResult = await invoke('test_saved_credential', {
        connectionId: activeConn.id,
      });
      setTestResult(res);
      updateActiveConn({
        tested_for_generation: true,
        last_test_status: 'Verified',
        last_verified_at: new Date().toISOString(),
      });
    } catch (e: any) {
      setTestError(e);
    } finally {
      setIsTestingCred(false);
    }
  };

  const handleTestEntered = async () => {
    if (!activeConn) return;
    if (!enteredSecret.trim()) {
      alert('Enter a key to test');
      return;
    }

    setIsTestingCred(true);
    setTestResult(null);
    setTestError(null);

    try {
      await onSaveConfig(formData);
      const res: GenerationTestResult = await invoke('test_new_credential', {
        connectionId: activeConn.id,
        secret: enteredSecret.trim(),
      });
      setTestResult(res);
    } catch (e: any) {
      setTestError(e);
    } finally {
      setIsTestingCred(false);
    }
  };

  // Discover Models
  const handleDiscoverModels = async () => {
    if (!activeConn) return;
    const sequence = ++discoverySequence.current;
    setIsDiscovering(true);
    setDiscoveryResult(null);

    try {
      await onSaveConfig(formData);
      const all = [];
      let token: string | null = null;
      const seen = new Set<string>();
      do {
        const page: ModelDiscoveryResult = await invoke('discover_models', { connectionId: activeConn.id, pageToken: token });
        if (sequence !== discoverySequence.current) return;
        all.push(...page.models);
        token = page.next_page_token;
        if (token && seen.has(token)) throw new Error('Repeated model pagination token');
        if (token) seen.add(token);
      } while (token && seen.size < 100);
      setDiscoveryResult({ success: true, connection_id: activeConn.id, models: Array.from(new Map(all.map((m) => [m.id, m])).values()), next_page_token: null, error: null });
    } catch (e: any) {
      alert(`Model discovery failed: ${e.message || JSON.stringify(e)}`);
    } finally {
      setIsDiscovering(false);
    }
  };

  // Check Readiness
  const handleCheckReadiness = async () => {
    if (!activeConn) return;
    setIsCheckingReadiness(true);
    setReadinessInfo(null);

    try {
      await onSaveConfig(formData);
      const info: GatewayReadinessInfo = await invoke('check_gateway_readiness', {
        connectionId: activeConn.id,
      });
      setReadinessInfo(info);
    } catch (e: any) {
      alert(`Readiness check failed: ${e.message || JSON.stringify(e)}`);
    } finally {
      setIsCheckingReadiness(false);
    }
  };

  // Save Settings atomically
  const handleSaveAll = async () => {
    setIsSaving(true);
    setSaveSuccessMsg(null);
    try {
      await onSaveConfig(formData);
      setSaveSuccessMsg('Settings saved successfully');
      setTimeout(() => setSaveSuccessMsg(null), 4000);
    } catch (e: any) {
      alert(`Failed to save settings: ${e}`);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="settings-fluid-container">
      {/* View Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 12 }}>
        <div>
          <h1 style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>
            Settings & Connection Profiles
          </h1>
          <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 2 }}>
            Configure local platform tools, direct API endpoints, self-hosted gateways, and OS credentials
          </p>
        </div>
        <button className="btn btn-primary" onClick={handleSaveAll} disabled={isSaving}>
          <Save size={14} />
          {isSaving ? 'Saving...' : 'Save settings'}
        </button>
      </div>

      {saveSuccessMsg && (
        <div className="badge badge-teal" style={{ width: '100%', padding: '10px 14px', fontSize: 12 }}>
          <CheckCircle2 size={15} />
          <span>{saveSuccessMsg}</span>
        </div>
      )}

      {/* Responsive 2-Column Grid */}
      <div className="settings-grid">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          {/* Connection Profiles Panel */}
          <div className="card">
            <div className="card-header">
              <div>
                <div className="card-title">AI Connection Profiles</div>
                <div className="card-desc">Manage direct provider APIs and self-hosted local gateways</div>
              </div>
              <button className="btn btn-secondary btn-sm" onClick={handleAddProfile}>
                <Plus size={14} />
                New profile
              </button>
            </div>

        {/* Profile Tabs */}
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, paddingBottom: 8, borderBottom: '1px solid var(--border-hairline)', marginBottom: 16 }}>
          {formData.connections.map((conn, idx) => (
            <button
              key={conn.id}
              className={`btn ${idx === activeConnIndex ? 'btn-primary' : 'btn-secondary'} btn-sm`}
              style={{ whiteSpace: 'nowrap' }}
              onClick={() => {
                ++discoverySequence.current;
                setIsDiscovering(false);
                setActiveConnIndex(idx);
                setEnteredSecret('');
                setCredOpStatus(null);
                setDiscoveryResult(null);
                setTestResult(null);
                setTestError(null);
              }}
            >
              {conn.name}
            </button>
          ))}
        </div>

        {activeConn && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            {/* Name & Mode */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 12 }}>
              <div>
                <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                  Profile Name
                </label>
                <div style={{ display: 'flex', gap: 6 }}>
                  <input
                    type="text"
                    className="input"
                    style={{ flex: 1 }}
                    value={activeConn.name}
                    onChange={(e) => updateActiveConn({ name: e.target.value })}
                  />
                  {formData.connections.length > 1 && (
                    <button
                      className="btn btn-danger btn-sm"
                      onClick={() => handleDeleteProfile(activeConnIndex)}
                      title="Delete this connection profile"
                    >
                      <Trash2 size={13} />
                    </button>
                  )}
                </div>
              </div>

              <div>
                <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                  Connection Mode
                </label>
                <select
                  className="select"
                  style={{ width: '100%' }}
                  value={activeConn.mode}
                  onChange={(e) => updateActiveConn({ mode: e.target.value as ConnectionMode })}
                >
                  <option value="DirectApi">Direct provider API</option>
                  <option value="Gateway">Local / Remote gateway</option>
                </select>
              </div>
            </div>

            {/* Base URL & Protocol */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 12 }}>
              <div>
                <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                  Base URL (Normalized endpoint)
                </label>
                <input
                  type="text"
                  className="input"
                  style={{ width: '100%' }}
                  value={activeConn.base_url}
                  onChange={(e) => updateActiveConn({ base_url: e.target.value })}
                  placeholder="https://api.openai.com/v1 or http://localhost:3001/v1"
                />
                <span style={{ fontSize: 10.5, color: 'var(--text-muted)', marginTop: 2, display: 'block' }}>
                  Path is automatically normalized without duplicate /v1. Localhost permits HTTP; remote endpoints require HTTPS.
                </span>
              </div>

              <div>
                <label style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block', marginBottom: 4 }}>
                  API Protocol
                </label>
                <select
                  className="select"
                  style={{ width: '100%' }}
                  value={activeConn.protocol}
                  onChange={(e) => updateActiveConn({ protocol: e.target.value as ApiProtocol })}
                >
                  <option value="GeminiNative">Gemini Native</option>
                  <option value="OpenAiChatCompletions">OpenAI Chat Completions</option>
                  <option value="OpenAiResponses">OpenAI Responses</option>
                  <option value="AnthropicMessages">Anthropic Messages</option>
                </select>
              </div>
            </div>

            {/* TZ AI Gateway Status Check */}
            {activeConn.preset === 'FreeLlmApi' && (
              <div style={{ padding: 12, background: 'var(--surface-deep)', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)', display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 8 }}>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 600 }}>TZ AI Gateway Status & Readiness</div>
                  <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                    Multi-provider router failing over across configured provider credentials
                  </div>
                </div>
                <button className="btn btn-secondary btn-sm" onClick={handleCheckReadiness} disabled={isCheckingReadiness}>
                  <Activity size={14} />
                  {isCheckingReadiness ? 'Checking...' : 'Check gateway status'}
                </button>
              </div>
            )}

            {readinessInfo && (
              <div style={{ padding: '10px 14px', background: readinessInfo.is_ready ? 'var(--status-positive-bg)' : 'var(--status-warning-bg)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)', fontSize: 12 }}>
                <span style={{ fontWeight: 600 }}>Status:</span> {readinessInfo.message}
                {readinessInfo.served_model && (
                  <span style={{ marginLeft: 12, color: 'var(--accent-teal)' }}>
                    (Served model: {readinessInfo.served_model})
                  </span>
                )}
              </div>
            )}

            {/* OS-Backed Credential Management */}
            <div style={{ padding: 14, background: 'var(--surface-deep)', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-hairline)', display: 'flex', flexDirection: 'column', gap: 10 }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 6 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <Lock size={15} style={{ color: 'var(--accent-teal)' }} />
                  <span style={{ fontSize: 12.5, fontWeight: 600 }}>OS-Backed Credential</span>
                </div>
                {activeConn.has_credential ? (
                  <span className="badge badge-teal">Saved in Windows Credential Manager</span>
                ) : (
                  <span className="badge badge-rose">No credential saved</span>
                )}
              </div>

              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                  <input
                    type="password"
                    className="input"
                    style={{ flex: '1 1 200px', minWidth: 160 }}
                    placeholder={activeConn.has_credential ? 'Enter new API key to replace saved credential' : 'Enter API key or Bearer token'}
                    value={enteredSecret}
                    onChange={(e) => setEnteredSecret(e.target.value)}
                  />
                  <button className="btn btn-primary btn-sm" onClick={handleSaveCredential} style={{ whiteSpace: 'nowrap' }}>
                    Save credential
                  </button>
                </div>
                <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                  {enteredSecret.trim() && (
                    <button className="btn btn-secondary btn-sm" onClick={handleTestEntered} disabled={isTestingCred} style={{ whiteSpace: 'nowrap' }}>
                      Test entered key
                    </button>
                  )}
                  {activeConn.has_credential && (
                    <>
                      <button className="btn btn-secondary btn-sm" onClick={handleTestSaved} disabled={isTestingCred} style={{ whiteSpace: 'nowrap' }}>
                        {isTestingCred ? 'Testing...' : 'Test saved key'}
                      </button>
                      <button className="btn btn-danger btn-sm" onClick={handleRemoveCredential} style={{ whiteSpace: 'nowrap' }}>
                        <Trash2 size={13} />
                        Remove
                      </button>
                    </>
                  )}
                </div>
              </div>

              {credOpStatus && (
                <div style={{ fontSize: 11, color: 'var(--text-secondary)' }}>
                  {credOpStatus}
                </div>
              )}
            </div>

            {/* Test Results Display */}
            {testResult && (
              <div className="badge badge-teal" style={{ width: '100%', padding: '10px 14px', fontSize: 12 }}>
                <CheckCircle2 size={15} />
                <span>{testResult.message}</span>
              </div>
            )}

            {testError && (
              <div className="error-banner">
                <div className="error-banner-header">
                  <AlertTriangle size={15} />
                  <span>{testError.error_type}</span>
                </div>
                <div>{testError.message}</div>
                <div style={{ color: 'var(--text-secondary)', fontSize: 11 }}>
                  Next step: {testError.suggested_action}
                </div>
                {testError.diagnostic_details && (
                  <div className="error-details-expander">{testError.diagnostic_details}</div>
                )}
              </div>
            )}

            {/* Model Discovery & Selection */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <label style={{ fontSize: 11, color: 'var(--text-muted)' }}>
                  Selected Model for this Connection
                </label>
                <div style={{ display: 'flex', gap: 6 }}>
                  <button className="btn btn-secondary btn-sm" onClick={handleDiscoverModels} disabled={isDiscovering}>
                    <RefreshCw size={12} className={isDiscovering ? 'spin' : ''} />
                    {isDiscovering ? 'Discovering...' : 'Discover models'}
                  </button>
                </div>
              </div>

              <input
                className="input"
                placeholder="Search discovered models"
                aria-label="Search discovered models"
                value={modelSearch}
                onChange={(e) => setModelSearch(e.target.value)}
              />
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 12 }}>
                <div>
                  <select
                    className="select"
                    style={{ width: '100%' }}
                    value={activeConn.selected_model || (activeConn.protocol === 'GeminiNative' ? 'gemini-3.5-flash' : '')}
                    onChange={(e) => updateActiveConn({ selected_model: e.target.value, manual_model_id: null })}
                  >
                    <option value="">Choose discovered model</option>
                    {discoveryResult?.models
                      .filter((m) => `${m.display_name} ${m.id}`.toLowerCase().includes(modelSearch.toLowerCase()))
                      .map((m) => (
                        <option key={m.id} value={m.id}>
                          {m.display_name} ({m.id})
                        </option>
                      ))}
                    {!discoveryResult && activeConn.selected_model && (
                      <option value={activeConn.selected_model}>{activeConn.selected_model}</option>
                    )}
                    {!discoveryResult && !activeConn.selected_model && activeConn.protocol === 'GeminiNative' && (
                      <option value="gemini-3.5-flash">Gemini 3.5 Flash (gemini-3.5-flash)</option>
                    )}
                  </select>
                </div>

                <div>
                  <input
                    type="text"
                    className="input"
                    style={{ width: '100%' }}
                    placeholder="Or enter manual model identifier"
                    value={activeConn.manual_model_id || ''}
                    onChange={(e) => updateActiveConn({ manual_model_id: e.target.value || null, selected_model: e.target.value || activeConn.selected_model })}
                  />
                </div>
              </div>

              {activeConn.tested_for_generation && (
                <div style={{ fontSize: 11, color: 'var(--status-positive)', display: 'flex', alignItems: 'center', gap: 4 }}>
                  <CheckCircle2 size={13} /> Tested and verified for generation at {activeConn.last_verified_at?.slice(0, 19).replace('T', ' ')}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      {/* Platform Tools & ADB Configuration */}
      <div className="card">
        <div className="card-header">
          <div>
            <div className="card-title">Android Debug Bridge (ADB) Path</div>
            <div className="card-desc">Specify a custom path to adb.exe or leave empty for auto-discovery</div>
          </div>
        </div>

        <div style={{ display: 'flex', gap: 8 }}>
          <input
            type="text"
            className="input"
            style={{ flex: 1 }}
            placeholder="Auto-discovered (system PATH, Android SDK, or local app data)"
            value={formData.custom_adb_path || ''}
            onChange={(e) => setFormData({ ...formData, custom_adb_path: e.target.value || null })}
          />
          <button className="btn btn-secondary" onClick={onRefreshDevices}>
            <RefreshCw size={13} />
            Test ADB
          </button>
        </div>
      </div>

      {/* AI Data Destination */}
      {activeConn && (
        <div className="card">
          <div className="card-title" style={{ marginBottom: 4 }}>AI data destination</div>
          <p className="card-desc" style={{ marginBottom: 10 }}>
            Chat messages, device model, Android version, package identifiers and community descriptions go to this endpoint. No device serial, Android account or personal files are included.
          </p>
          <div style={{ marginBottom: 12 }}>
            <code style={{ fontSize: 11, color: 'var(--accent-teal)' }}>{activeConn.base_url}</code>
          </div>
          <button
            className="btn btn-secondary btn-sm"
            onClick={async () => {
              const destination = activeConn.base_url.replace(/\/$/, '');
              const next = { ...formData, approved_destinations: Array.from(new Set([...formData.approved_destinations, destination])) };
              try {
                await onSaveConfig(next);
                setFormData(next);
                setSaveSuccessMsg('AI destination approved');
              } catch (e) {
                setCredOpStatus(String(e));
              }
            }}
          >
            Approve this destination for AI requests
          </button>
        </div>
      )}

      {/* Privacy and Consent */}
      <div className="card">
        <div className="card-header">
          <div>
            <div className="card-title">Privacy & Cloud Transmission Consent</div>
            <div className="card-desc">Explicit consent controls for cloud debloating assistance</div>
          </div>
        </div>

        <label style={{ display: 'flex', alignItems: 'flex-start', gap: 10, cursor: 'pointer', fontSize: 12.5, lineHeight: 1.5 }}>
          <input
            type="checkbox"
            checked={formData.ai_consent_given}
            onChange={(e) => setFormData({ ...formData, ai_consent_given: e.target.checked })}
            style={{ marginTop: 2 }}
          />
          <div>
            <span>I authorize sending chat messages, device model, Android version, package identifiers, and community descriptions to the selected AI endpoint.</span>
            <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>
              Hardware serial numbers, IMEIs, user accounts, and personal files are redacted from all prompts.
            </div>
          </div>
        </label>
      </div>
      </div>
      </div>
    </div>
  );
};
