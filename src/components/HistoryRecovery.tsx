import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  History,
  RotateCcw,
  Camera,
  Trash2,
  Download,
  Upload,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Info,
  X,
  FileText
} from 'lucide-react';
import {
  ActionPlan,
  DeviceInfo,
  DeviceSnapshot,
  JournalEntry,
  AndroidUser,
  RecoveryProposal,
  OperationOutcome,
  DebloatProfile
} from '../types';

interface Props {
  device: DeviceInfo | null;
  user: AndroidUser | null;
  onRefresh: () => void;
  onOpenPlan: (plan: ActionPlan) => void;
}

export const HistoryRecovery: React.FC<Props> = ({ device, user, onRefresh, onOpenPlan }) => {
  const [journal, setJournal] = useState<JournalEntry[]>([]);
  const [snapshots, setSnapshots] = useState<DeviceSnapshot[]>([]);
  const [activeTab, setActiveTab] = useState<'journal' | 'snapshots' | 'profiles'>('journal');
  const [isLoading, setIsLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Rollback Proposal Modal State
  const [activeProposal, setActiveProposal] = useState<{ entry: JournalEntry; proposal: RecoveryProposal } | null>(null);
  const [isExecutingRecovery, setIsExecutingRecovery] = useState(false);

  // Snapshot Creation Modal State
  const [showSnapshotModal, setShowSnapshotModal] = useState(false);
  const [snapshotNote, setSnapshotNote] = useState('');
  const [isCreatingSnapshot, setIsCreatingSnapshot] = useState(false);

  // Snapshot Inspection Modal State
  const [inspectingSnapshot, setInspectingSnapshot] = useState<DeviceSnapshot | null>(null);

  // Profile Export / Import State
  const [exportedProfileJson, setExportedProfileJson] = useState<string | null>(null);
  const [importProfileText, setImportProfileText] = useState('');

  // Clear Journal Confirmation
  const [showClearConfirm, setShowClearConfirm] = useState(false);

  const loadHistory = async () => {
    setIsLoading(true);
    setErrorMessage(null);
    try {
      const jEntries: JournalEntry[] = await invoke('get_journal_history');
      setJournal(Array.isArray(jEntries) ? jEntries : []);
      const snaps: DeviceSnapshot[] = await invoke('list_snapshots');
      setSnapshots(Array.isArray(snaps) ? snaps : []);
    } catch (e: any) {
      setErrorMessage(`Failed to load history: ${e?.message || e}`);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadHistory();
  }, []);

  const handleClearHistory = async () => {
    setShowClearConfirm(false);
    try {
      await invoke('clear_journal_history');
      setSuccessMessage('Operation journal cleared successfully');
      loadHistory();
    } catch (e: any) {
      setErrorMessage(`Failed to clear journal: ${e?.message || e}`);
    }
  };

  const handleCreateSnapshot = async () => {
    if (!device || !user) {
      setErrorMessage('Please connect and select an active Android device and user profile');
      return;
    }
    if (!snapshotNote.trim()) {
      setErrorMessage('Please enter a descriptive label for the snapshot');
      return;
    }

    setIsCreatingSnapshot(true);
    setErrorMessage(null);
    try {
      await invoke('create_snapshot', {
        deviceSerial: device.serial,
        userId: user.id,
        note: snapshotNote.trim(),
      });
      setShowSnapshotModal(false);
      setSnapshotNote('');
      setSuccessMessage('Snapshot captured and saved successfully');
      loadHistory();
    } catch (e: any) {
      setErrorMessage(`Snapshot capture failed: ${e?.message || e}`);
    } finally {
      setIsCreatingSnapshot(false);
    }
  };

  // Step 1 of Rollback: Propose Recovery via Backend Policy
  const handleInitiateRollback = async (entry: JournalEntry) => {
    if (!device || !user) {
      setErrorMessage('Please connect and select an active Android device first');
      return;
    }

    setErrorMessage(null);
    try {
      const proposal: RecoveryProposal = await invoke('propose_recovery_action', {
        entryId: entry.entry_id,
        currentSerial: device.serial,
        currentUserId: user.id,
      });
      setActiveProposal({ entry, proposal });
    } catch (e: any) {
      setErrorMessage(`Recovery proposal failed: ${e?.message || e}`);
    }
  };

  // Step 2 of Rollback: Execute Backend Recovery
  const handleConfirmExecuteRecovery = async () => {
    if (!activeProposal || !device || !user) return;

    setIsExecutingRecovery(true);
    setErrorMessage(null);
    try {
      const outcomes: OperationOutcome[] = await invoke('execute_recovery', {
        entryId: activeProposal.entry.entry_id,
        currentSerial: device.serial,
        currentUserId: user.id,
      });

      const outcome = outcomes[outcomes.length - 1];
      if (outcome && outcomes.every((o) => o.success && o.verified)) {
        setSuccessMessage(`Recovery executed successfully on ${outcome.package_id}`);
        setActiveProposal(null);
        loadHistory();
        onRefresh();
      } else {
        setErrorMessage(`Recovery failed: ${outcome?.message || 'No operations completed'}`);
      }
    } catch (e: any) {
      setErrorMessage(`Execution error: ${e?.message || e}`);
    } finally {
      setIsExecutingRecovery(false);
    }
  };

  // Profile Export
  const handleExportProfile = async () => {
    if (!device || !user) {
      setErrorMessage('Select a device and user profile to export');
      return;
    }
    setErrorMessage(null);
    try {
      const profile: DebloatProfile = await invoke('export_profile', {
        deviceSerial: device.serial,
        userId: user.id,
      });
      setExportedProfileJson(JSON.stringify(profile, null, 2));
      setSuccessMessage(`Profile exported: ${profile.actions.length} package state rules recorded`);
    } catch (e: any) {
      setErrorMessage(`Export failed: ${e?.message || e}`);
    }
  };

  // Profile Import
  const handleImportProfile = async () => {
    if (!device || !user) {
      setErrorMessage('Select a device and profile before reviewing imported actions');
      return;
    }
    if (!importProfileText.trim()) {
      setErrorMessage('Please paste valid profile JSON to import');
      return;
    }
    setErrorMessage(null);
    try {
      const profile: DebloatProfile = await invoke('import_profile', {
        profileJson: importProfileText.trim(),
      });
      onOpenPlan({
        id: crypto.randomUUID(),
        revision: 1,
        target_device_serial: device.serial,
        target_user_id: user.id,
        explanation: `Imported profile: ${profile.profile_name}. Every selected rule is checked against this device before execution.`,
        preserved_functionality: [],
        created_at: Math.floor(Date.now() / 1000),
        approved_by_user: false,
        is_valid: false,
        invalidation_reason: null,
        actions: profile.actions.map((a) => ({
          package_id: a.package_id,
          package_name: null,
          proposed_action: a.desired_action,
          reason: 'Imported package state rule',
          evidence: 'User-supplied profile; requires local validation',
          expected_side_effects: null,
          uncertainty: 'High',
          removal_category: 'Unlisted',
          is_protected: false,
          protection_warning: null,
          selected: true,
          is_executable: true,
        })),
      });
      setImportProfileText('');
    } catch (e: any) {
      setErrorMessage(`Import error: ${e?.message || e}`);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <h1 style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>
            Operation Journal & Device Recovery
          </h1>
          <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 2 }}>
            Audit trail of executed mutations, pre-change state records, and rollback operations
          </p>
        </div>
        <div style={{ display: 'flex', gap: 8 }}>
          <button
            className="btn btn-secondary btn-sm"
            onClick={() => {
              setSnapshotNote(`Manual checkpoint ${new Date().toLocaleTimeString()}`);
              setShowSnapshotModal(true);
            }}
            disabled={!device || !user}
          >
            <Camera size={13} />
            Capture snapshot
          </button>
          <button className="btn btn-secondary btn-sm" onClick={loadHistory} disabled={isLoading}>
            <RotateCcw size={13} className={isLoading ? 'spin' : ''} />
            Refresh
          </button>
        </div>
      </div>

      {/* Messages */}
      {errorMessage && (
        <div className="error-banner">
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <AlertTriangle size={15} style={{ flexShrink: 0 }} />
              <span>{errorMessage}</span>
            </div>
            <button
              onClick={() => setErrorMessage(null)}
              style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer' }}
            >
              <X size={14} />
            </button>
          </div>
        </div>
      )}

      {successMessage && (
        <div className="badge badge-teal" style={{ width: '100%', padding: '10px 14px', fontSize: 12, justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <CheckCircle2 size={15} style={{ flexShrink: 0 }} />
            <span>{successMessage}</span>
          </div>
          <button
            onClick={() => setSuccessMessage(null)}
            style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer' }}
          >
            <X size={14} />
          </button>
        </div>
      )}

      {/* Non-Root Rollback Disclaimers */}
      <div className="card" style={{ padding: '12px 16px', fontSize: 12, lineHeight: 1.5 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--text-primary)', fontWeight: 600, marginBottom: 4 }}>
          <Info size={14} style={{ color: 'var(--accent-teal)' }} />
          State Recording and Rollback Boundaries
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 3, color: 'var(--text-secondary)' }}>
          <div>
            • <strong>State records:</strong> Snapshots document package status (enabled, disabled, uninstalled) per profile. They do not store application user data.
          </div>
          <div>
            • <strong>Non-root restoration:</strong> Restoring re-enables or reinstalls the system APK preserved in the read-only /system partition. App data purged during uninstall cannot be recovered.
          </div>
          <div>
            • <strong>Target protection:</strong> Recovery operations enforce target device serial and user profile identity to prevent accidental cross-device changes.
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div style={{ display: 'flex', gap: 8, borderBottom: '1px solid var(--border-hairline)' }}>
        <button
          onClick={() => setActiveTab('journal')}
          style={{
            background: 'none',
            border: 'none',
            borderBottom: activeTab === 'journal' ? '2px solid var(--accent-teal)' : '2px solid transparent',
            color: activeTab === 'journal' ? 'var(--text-primary)' : 'var(--text-muted)',
            padding: '8px 12px',
            fontSize: 12,
            fontWeight: 500,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            transition: 'color 120ms var(--ease-out)',
          }}
        >
          <History size={14} />
          Operation audit trail ({journal.length})
        </button>
        <button
          onClick={() => setActiveTab('snapshots')}
          style={{
            background: 'none',
            border: 'none',
            borderBottom: activeTab === 'snapshots' ? '2px solid var(--accent-teal)' : '2px solid transparent',
            color: activeTab === 'snapshots' ? 'var(--text-primary)' : 'var(--text-muted)',
            padding: '8px 12px',
            fontSize: 12,
            fontWeight: 500,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            transition: 'color 120ms var(--ease-out)',
          }}
        >
          <Camera size={14} />
          Saved snapshots ({snapshots.length})
        </button>
        <button
          onClick={() => setActiveTab('profiles')}
          style={{
            background: 'none',
            border: 'none',
            borderBottom: activeTab === 'profiles' ? '2px solid var(--accent-teal)' : '2px solid transparent',
            color: activeTab === 'profiles' ? 'var(--text-primary)' : 'var(--text-muted)',
            padding: '8px 12px',
            fontSize: 12,
            fontWeight: 500,
            cursor: 'pointer',
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            transition: 'color 120ms var(--ease-out)',
          }}
        >
          <FileText size={14} />
          Profiles & Presets
        </button>

        {activeTab === 'journal' && journal.length > 0 && (
          <button
            onClick={() => setShowClearConfirm(true)}
            className="btn btn-secondary btn-sm"
            style={{ marginLeft: 'auto', marginBottom: 4 }}
          >
            <Trash2 size={12} />
            Clear journal
          </button>
        )}
      </div>

      {/* Tab 1: Journal */}
      {activeTab === 'journal' && (
        <div className="data-table-container">
          <div style={{ overflowY: 'auto' }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th>Timestamp</th>
                  <th>Target device</th>
                  <th>Package identifier</th>
                  <th>Action</th>
                  <th>Outcome</th>
                  <th>Message</th>
                  <th style={{ textAlign: 'right' }}>Rollback</th>
                </tr>
              </thead>
              <tbody>
                {isLoading ? (
                  <tr>
                    <td colSpan={7} style={{ textAlign: 'center', padding: 24, color: 'var(--text-muted)' }}>
                      Loading audit trail...
                    </td>
                  </tr>
                ) : journal.length === 0 ? (
                  <tr>
                    <td colSpan={7} style={{ textAlign: 'center', padding: 24, color: 'var(--text-muted)' }}>
                      No recorded operations in local audit trail.
                    </td>
                  </tr>
                ) : (
                  journal.map((item) => {
                    const matchesDevice = device?.serial === item.device_serial;
                    const matchesUser = user?.id === item.user_id;

                    return (
                      <tr key={item.entry_id}>
                        <td style={{ color: 'var(--text-muted)', fontSize: 11, whiteSpace: 'nowrap', fontVariantNumeric: 'tabular-nums' }}>
                          {new Date(item.timestamp * 1000).toLocaleString()}
                        </td>
                        <td style={{ fontSize: 11.5 }}>
                          <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>{item.device_serial}</span>
                          <span style={{ color: 'var(--text-muted)', marginLeft: 4 }}>(User {item.user_id})</span>
                        </td>
                        <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: 'var(--accent-teal)' }}>
                          {item.package_id}
                        </td>
                        <td>
                          <span
                            className={`badge ${
                              item.action === 'Disable'
                                ? 'badge-amber'
                                : item.action === 'UninstallUser'
                                ? 'badge-rose'
                                : 'badge-teal'
                            }`}
                          >
                            {item.action}
                          </span>
                        </td>
                        <td>
                          {item.success ? (
                            <span className="badge badge-teal">
                              <CheckCircle2 size={10} /> Succeeded
                            </span>
                          ) : (
                            <span className="badge badge-rose">
                              <XCircle size={10} /> Failed
                            </span>
                          )}
                        </td>
                        <td
                          style={{
                            fontSize: 11.5,
                            color: 'var(--text-secondary)',
                            maxWidth: 240,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {item.message}
                        </td>
                        <td style={{ textAlign: 'right' }}>
                          <button
                            className="btn btn-secondary btn-sm"
                            onClick={() => handleInitiateRollback(item)}
                            disabled={!matchesDevice || !matchesUser}
                            title={
                              !matchesDevice || !matchesUser
                                ? 'Device or profile mismatch: cannot execute cross-device recovery'
                                : 'Propose inverse recovery action'
                            }
                          >
                            <RotateCcw size={11} />
                            Rollback
                          </button>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Tab 2: Snapshots */}
      {activeTab === 'snapshots' && (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: 12 }}>
          {snapshots.length === 0 ? (
            <div
              className="card"
              style={{
                gridColumn: '1 / -1',
                padding: 32,
                textAlign: 'center',
                color: 'var(--text-muted)',
                fontSize: 12.5,
              }}
            >
              No state snapshots saved yet. Snapshots are automatically recorded before applying AI plans, or can be captured manually.
            </div>
          ) : (
            snapshots.map((snap) => (
              <div key={snap.snapshot_id} className="card" style={{ padding: 14 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 6 }}>
                  <div>
                    <div style={{ fontWeight: 600, color: 'var(--text-primary)', fontSize: 13 }}>
                      {snap.note || 'State Snapshot'}
                    </div>
                    <div style={{ fontSize: 11, color: 'var(--text-muted)', marginTop: 2, fontFamily: 'var(--font-mono)' }}>
                      {snap.device_serial} · User {snap.user_id}
                    </div>
                  </div>
                  <span className="badge badge-zinc" style={{ fontVariantNumeric: 'tabular-nums' }}>
                    {snap.package_states.length} packages
                  </span>
                </div>

                <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 12, fontVariantNumeric: 'tabular-nums' }}>
                  Captured: {new Date(snap.timestamp * 1000).toLocaleString()}
                </div>

                <div style={{ display: 'flex', gap: 8 }}>
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={() => setInspectingSnapshot(snap)}
                  >
                    View package states
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {/* Tab 3: Profiles & Presets */}
      {activeTab === 'profiles' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          <div className="card" style={{ padding: 16 }}>
            <div style={{ fontSize: 13.5, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>
              Export Connected Device Profile
            </div>
            <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 12 }}>
              Export current package configuration for {device ? `${device.brand} ${device.model}` : 'device'} into a shareable debloat profile
            </p>
            <button className="btn btn-secondary btn-sm" onClick={handleExportProfile} disabled={!device || !user}>
              <Download size={13} />
              Export current state profile
            </button>

            {exportedProfileJson && (
              <div style={{ marginTop: 12 }}>
                <textarea
                  className="input-field"
                  style={{ width: '100%', height: 140, fontFamily: 'var(--font-mono)', fontSize: 11 }}
                  readOnly
                  value={exportedProfileJson}
                />
              </div>
            )}
          </div>

          <div className="card" style={{ padding: 16 }}>
            <div style={{ fontSize: 13.5, fontWeight: 600, color: 'var(--text-primary)', marginBottom: 4 }}>
              Import Profile Preset
            </div>
            <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 12 }}>
              Paste a validated debloat profile JSON document to load predefined package state rules
            </p>
            <textarea
              className="input-field"
              placeholder="Paste profile JSON here..."
              style={{ width: '100%', height: 100, fontFamily: 'var(--font-mono)', fontSize: 11, marginBottom: 8 }}
              value={importProfileText}
              onChange={(e) => setImportProfileText(e.target.value)}
            />
            <button className="btn btn-secondary btn-sm" onClick={handleImportProfile} disabled={!importProfileText.trim()}>
              <Upload size={13} />
              Import profile preset
            </button>
          </div>
        </div>
      )}

      {/* Modal: Rollback Proposal */}
      {activeProposal && (
        <div className="modal-overlay" onClick={() => !isExecutingRecovery && setActiveProposal(null)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 520, width: '100%' }}>
            <div className="modal-header">
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
                <RotateCcw size={16} style={{ color: 'var(--accent-teal)' }} />
                Confirm Inverse Recovery Action
              </h3>
              <button
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                onClick={() => !isExecutingRecovery && setActiveProposal(null)}
              >
                <X size={15} />
              </button>
            </div>
            <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                A recovery action will be executed on the active device:
              </div>

              <div
                className="card"
                style={{
                  background: 'var(--surface-deep)',
                  padding: 12,
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 6,
                  fontSize: 12,
                }}
              >
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Package:</span>{' '}
                  <code style={{ color: 'var(--accent-teal)' }}>{activeProposal.entry.package_id}</code>
                </div>
                <div>
                  <span style={{ color: 'var(--text-muted)' }}>Original mutation:</span>{' '}
                  <span className="badge badge-zinc">{activeProposal.entry.action}</span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ color: 'var(--text-muted)' }}>Recovery actions:</span>
                  <div style={{ display: 'flex', gap: 4 }}>
                    {activeProposal.proposal.steps.map((act, i) => (
                      <span key={i} className="badge badge-teal">
                        {act}
                      </span>
                    ))}
                  </div>
                </div>
                <div style={{ fontSize: 11, color: 'var(--text-muted)', marginTop: 4 }}>
                  Target: {activeProposal.proposal.device_serial} · User {activeProposal.proposal.user_id}
                </div>
              </div>

              <div style={{ fontSize: 11.5, color: 'var(--text-muted)', lineHeight: 1.4 }}>
                Restoring re-enables or reinstalls the system package. Any application user data purged by previous operations cannot be retrieved.
              </div>
            </div>
            <div className="modal-footer" style={{ justifyContent: 'flex-end', gap: 8 }}>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setActiveProposal(null)}
                disabled={isExecutingRecovery}
              >
                Cancel
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleConfirmExecuteRecovery}
                disabled={isExecutingRecovery}
              >
                {isExecutingRecovery ? 'Executing...' : 'Execute recovery'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Create Snapshot */}
      {showSnapshotModal && (
        <div className="modal-overlay" onClick={() => !isCreatingSnapshot && setShowSnapshotModal(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 440, width: '100%' }}>
            <div className="modal-header">
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
                <Camera size={16} style={{ color: 'var(--accent-teal)' }} />
                Capture State Snapshot
              </h3>
              <button
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                onClick={() => !isCreatingSnapshot && setShowSnapshotModal(false)}
              >
                <X size={15} />
              </button>
            </div>
            <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                Snapshot label / description:
              </div>
              <input
                type="text"
                className="input-field"
                placeholder="e.g. Pre-optimization checkpoint"
                value={snapshotNote}
                onChange={(e) => setSnapshotNote(e.target.value)}
                autoFocus
              />
              <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>
                Records enabled/disabled states for all installed packages on {device?.serial} (User {user?.id}).
              </div>
            </div>
            <div className="modal-footer" style={{ justifyContent: 'flex-end', gap: 8 }}>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setShowSnapshotModal(false)}
                disabled={isCreatingSnapshot}
              >
                Cancel
              </button>
              <button
                className="btn btn-primary btn-sm"
                onClick={handleCreateSnapshot}
                disabled={isCreatingSnapshot || !snapshotNote.trim()}
              >
                {isCreatingSnapshot ? 'Capturing...' : 'Save snapshot'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Inspect Snapshot Package States */}
      {inspectingSnapshot && (
        <div className="modal-overlay" onClick={() => setInspectingSnapshot(null)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 640, width: '100%', maxHeight: '80vh' }}>
            <div className="modal-header">
              <div>
                <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>
                  {inspectingSnapshot.note || 'Snapshot Details'}
                </h3>
                <div style={{ fontSize: 11, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
                  {inspectingSnapshot.device_serial} · User {inspectingSnapshot.user_id} · {inspectingSnapshot.package_states.length} packages
                </div>
              </div>
              <button
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                onClick={() => setInspectingSnapshot(null)}
              >
                <X size={15} />
              </button>
            </div>
            <div className="modal-body">
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Package identifier</th>
                    <th>State</th>
                    <th>Installed</th>
                  </tr>
                </thead>
                <tbody>
                  {inspectingSnapshot.package_states.map((pkg) => (
                    <tr key={pkg.package_id}>
                      <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: 'var(--text-primary)' }}>
                        {pkg.package_id}
                      </td>
                      <td>
                        <span className={`badge ${pkg.state === 'Enabled' ? 'badge-teal' : pkg.state === 'Disabled' ? 'badge-amber' : 'badge-rose'}`}>
                          {pkg.state}
                        </span>
                      </td>
                      <td style={{ fontSize: 11, color: pkg.state !== 'Uninstalled' ? 'var(--accent-teal)' : 'var(--text-muted)' }}>
                        {pkg.state !== 'Uninstalled' ? 'Yes' : 'No'}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="modal-footer" style={{ justifyContent: 'flex-end' }}>
              <button className="btn btn-secondary btn-sm" onClick={() => setInspectingSnapshot(null)}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modal: Clear Journal Confirmation */}
      {showClearConfirm && (
        <div className="modal-overlay" onClick={() => setShowClearConfirm(false)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 400, width: '100%' }}>
            <div className="modal-header">
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>
                Clear Operation Journal?
              </h3>
            </div>
            <div className="modal-body" style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
              This will erase all recorded operation history from the local database. Saved snapshots will remain intact. This action cannot be undone.
            </div>
            <div className="modal-footer" style={{ justifyContent: 'flex-end', gap: 8 }}>
              <button className="btn btn-secondary btn-sm" onClick={() => setShowClearConfirm(false)}>
                Cancel
              </button>
              <button className="btn btn-danger btn-sm" onClick={handleClearHistory}>
                Clear journal
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
