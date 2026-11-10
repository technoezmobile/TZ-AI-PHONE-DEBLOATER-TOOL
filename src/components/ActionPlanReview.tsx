import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  ShieldCheck,
  ShieldAlert,
  AlertTriangle,
  Play,
  RotateCcw,
  CheckCircle2,
  X,
  Lock,
  ArrowRight,
  Ban,
  ChevronDown,
  ChevronRight,
  Terminal
} from 'lucide-react';
import {
  ActionPlan,
  DeviceInfo,
  AndroidUser,
  DevicePackage,
  OperationOutcome,
  PlanValidationResult,
  PlanApprovalSession,
  PlanProgressEvent,
  PlanExecutionCompletePayload
} from '../types';

interface Props {
  plan: ActionPlan;
  device: DeviceInfo | null;
  user: AndroidUser | null;
  packages: DevicePackage[];
  onClose: () => void;
  onExecutionCompleted: (outcomes?: OperationOutcome[], plan?: ActionPlan) => void;
}

export const ActionPlanReview: React.FC<Props> = ({
  plan,
  device,
  user,
  packages,
  onClose,
  onExecutionCompleted,
}) => {
  const [currentPlan, setCurrentPlan] = useState<ActionPlan>(plan);
  const [userAcknowledgedRisk, setUserAcknowledgedRisk] = useState(false);
  const [continueOnIndependentFailure, setContinueOnIndependentFailure] = useState(true);
  const [isExecuting, setIsExecuting] = useState(false);
  const [executionProgress, setExecutionProgress] = useState<PlanProgressEvent | null>(null);
  const [executionResults, setExecutionResults] = useState<OperationOutcome[] | null>(() => {
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      if (params.get('tab') === 'partial_failure' || params.get('partial_failure') === 'true') {
        const outcomes: OperationOutcome[] = [
          {
            package_id: 'com.facebook.appmanager',
            action: 'UninstallUser',
            success: true,
            exit_code: 0,
            previous_state: 'Enabled',
            resulting_state: 'Uninstalled',
            message: 'Successfully uninstalled for User 0 (Owner). Verification confirmed package not present in user profile.',
            verified: true,
            execution_backend: 'DirectAdbPm',
            command_args: ['pm', 'uninstall', '-k', '--user', '0', 'com.facebook.appmanager'],
            target_device: 'MOCK-DEVICE-SERIAL-001',
            target_user_id: 0,
            raw_stdout: 'Success\n',
            raw_stderr: '',
          },
          {
            package_id: 'com.miui.msa.global',
            action: 'Disable',
            success: false,
            exit_code: 255,
            previous_state: 'Enabled',
            resulting_state: 'Enabled',
            message: 'Action denied by OEM security check. Disabling system components on Xiaomi / HyperOS requires enabling USB debugging (Security settings).',
            verified: false,
            classified_cause: 'MissingPrivileges',
            execution_backend: 'DirectAdbPm',
            command_args: ['pm', 'disable-user', '--user', '0', 'com.miui.msa.global'],
            target_device: 'MOCK-DEVICE-SERIAL-001',
            target_user_id: 0,
            raw_stdout: '',
            raw_stderr: 'Error: java.lang.SecurityException: Neither user 2000 nor current process has android.permission.MANAGE_USERS.\n',
          },
          {
            package_id: 'com.miui.analytics',
            action: 'Disable',
            success: false,
            exit_code: null,
            previous_state: 'Enabled',
            resulting_state: 'Enabled',
            message: 'Skipped execution because preceding operation failed and safe abort was triggered.',
            verified: false,
            execution_backend: 'DirectAdbPm',
            command_args: [],
            target_device: 'MOCK-DEVICE-SERIAL-001',
            target_user_id: 0,
            raw_stdout: '',
            raw_stderr: '',
          },
        ];
        return outcomes;
      }
    }
    return null;
  });
  const [executionSummary, setExecutionSummary] = useState<PlanExecutionCompletePayload | null>(() => {
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      if (params.get('tab') === 'partial_failure' || params.get('partial_failure') === 'true') {
        const summary: PlanExecutionCompletePayload = {
          plan_id: plan.id,
          total: 3,
          successful: 1,
          failed: 1,
          attempted_count: 2,
          succeeded_count: 1,
          failed_count: 1,
          skipped_count: 1,
          unknown_count: 0,
        };
        return summary;
      }
    }
    return null;
  });
  const [expandedDiagnostics, setExpandedDiagnostics] = useState<Record<string, boolean>>(() => {
    const init: Record<string, boolean> = {};
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      if (params.get('tab') === 'partial_failure' || params.get('partial_failure') === 'true') {
        init['com.miui.msa.global'] = true;
      }
    }
    return init;
  });
  const [isCreatingRetry, setIsCreatingRetry] = useState(false);
  const [validationResult, setValidationResult] = useState<PlanValidationResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Check device targeting match
  const deviceMismatch = device ? plan.target_device_serial !== device.serial || device.state !== 'Ready' : true;
  const userMismatch = user ? plan.target_user_id !== user.id : true;

  const toggleDiagnostic = (pkgId: string) => {
    setExpandedDiagnostics((prev) => ({ ...prev, [pkgId]: !prev[pkgId] }));
  };

  // Toggle selection of individual proposed action
  const toggleActionSelection = (pkgId: string) => {
    if (isExecuting) return;
    setCurrentPlan((prev) => ({
      ...prev,
      revision: prev.revision + 1,
      actions: prev.actions.map((a) => {
        if (a.package_id === pkgId && !a.is_protected && a.is_executable !== false) {
          return { ...a, selected: !a.selected };
        }
        return a;
      }),
    }));
    setValidationResult(null);
  };

  // Pre-validate current action selection against device policy
  const handleValidatePlan = async () => {
    if (!device || user === null || user === undefined) {
      setErrorMessage('A ready device and user profile are required for pre-check');
      return;
    }
    try {
      const res: PlanValidationResult = await invoke('validate_plan', {
        plan: currentPlan,
        currentSerial: device.serial,
        currentUserId: user.id,
      });
      setValidationResult(res);
    } catch (e: any) {
      setErrorMessage(`Validation error: ${e?.message || e}`);
    }
  };

  // Create retry plan for remaining actions
  const handleCreateRetryPlan = async () => {
    try {
      setIsCreatingRetry(true);
      setErrorMessage(null);
      const succeededIds = new Set(
        (executionResults || []).filter((r) => r.success && r.verified).map((r) => r.package_id)
      );
      const remainingPackageIds = currentPlan.actions
        .filter((a) => !succeededIds.has(a.package_id))
        .map((a) => a.package_id);

      if (remainingPackageIds.length === 0) {
        setErrorMessage('No failed or skipped actions remaining to retry');
        return;
      }

      const retryPlan: ActionPlan = await invoke('create_retry_plan', {
        originalPlan: currentPlan,
        remainingPackageIds,
      });
      if (!retryPlan.actions || retryPlan.actions.length === 0) {
        setErrorMessage('No failed or skipped actions remaining to retry');
        return;
      }
      setCurrentPlan(retryPlan);
      setExecutionResults(null);
      setExecutionSummary(null);
      setUserAcknowledgedRisk(false);
      setValidationResult(null);
    } catch (e: any) {
      setErrorMessage(`Failed to create retry plan: ${e?.message || e}`);
    } finally {
      setIsCreatingRetry(false);
    }
  };

  // Cryptographic approval & plan execution
  const handleApproveAndExecute = async () => {
    if (deviceMismatch || userMismatch) {
      setErrorMessage('Device or profile target mismatch. Re-verify the target before execution.');
      return;
    }

    const selectedPkgIds = currentPlan.actions
      .filter((a) => a.selected && a.is_executable !== false && !a.is_protected)
      .map((a) => a.package_id);

    if (selectedPkgIds.length === 0) {
      setErrorMessage('No executable actions are selected for execution.');
      return;
    }

    setErrorMessage(null);
    setIsExecuting(true);
    setExecutionResults(null);
    setExecutionSummary(null);

    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;

    try {
      const session: PlanApprovalSession = await invoke('request_plan_approval', {
        plan: currentPlan,
      });

      unlistenProgress = await listen<PlanProgressEvent>('plan-execution-progress', (event) => {
        setExecutionProgress(event.payload);
      });

      unlistenComplete = await listen<PlanExecutionCompletePayload>(
        'plan-execution-complete',
        (event) => {
          setExecutionSummary(event.payload);
        }
      );

      const outcomes: OperationOutcome[] = await invoke('execute_action_plan', {
        planId: currentPlan.id,
        revision: currentPlan.revision,
        approvalToken: session.approval_token,
        continueOnIndependentFailure,
      });

      setExecutionResults(outcomes);
      if (outcomes.length < selectedPkgIds.length) {
        setErrorMessage(`Execution stopped after ${outcomes.length} of ${selectedPkgIds.length} actions. Review the results below.`);
      }
      onExecutionCompleted(outcomes, currentPlan);
    } catch (e: any) {
      setErrorMessage(`Execution error: ${e?.message || e}`);
    } finally {
      if (unlistenProgress) unlistenProgress();
      if (unlistenComplete) unlistenComplete();
      setIsExecuting(false);
      setExecutionProgress(null);
    }
  };

  const selectedCount = currentPlan.actions.filter(
    (a) => a.selected && a.is_executable !== false && !a.is_protected
  ).length;

  const getResultingState = (action: string) => {
    switch (action) {
      case 'Disable':
        return 'Disabled for user';
      case 'UninstallUser':
        return 'Uninstalled (kept in system)';
      case 'Enable':
        return 'Enabled';
      case 'Restore':
        return 'Restored';
      default:
        return 'Modified';
    }
  };

  const getOutcomeBadge = (outcome: OperationOutcome) => {
    if (outcome.message && outcome.message.includes('Already in requested state')) {
      return { label: 'Already in requested state', cls: 'badge-zinc' };
    }
    if (outcome.verified) {
      if (outcome.action === 'UninstallUser' && outcome.success) {
        return { label: `Verified uninstalled for user ${outcome.target_user_id ?? user?.id ?? 0}`, cls: 'badge-rose' };
      }
      if (outcome.action === 'Disable' && outcome.success) {
        return { label: 'Verified disabled', cls: 'badge-amber' };
      }
      if ((outcome.action === 'Enable' || outcome.action === 'Restore') && outcome.success) {
        return { label: 'Verified restored', cls: 'badge-teal' };
      }
    }
    if (outcome.success) {
      return { label: 'Success', cls: 'badge-teal' };
    }
    return { label: 'Failed', cls: 'badge-rose' };
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal-box"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 940, width: '95vw', maxHeight: '90vh' }}
      >
        {/* Modal Header */}
        <div className="modal-header">
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <ShieldCheck size={18} style={{ color: 'var(--accent-teal)' }} />
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>
                Review Proposed Action Plan
              </h3>
            </div>
            <div style={{ fontSize: 11, color: 'var(--text-muted)', marginTop: 2 }}>
              Deterministic safety policy gate and explicit single-use approval boundary
            </div>
          </div>
          <button
            style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: 4 }}
            onClick={onClose}
          >
            <X size={16} />
          </button>
        </div>

        {/* Modal Body */}
        <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
          {/* Error Banner */}
          {errorMessage && (
            <div className="error-banner">
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <AlertTriangle size={15} />
                <span>{errorMessage}</span>
              </div>
            </div>
          )}

          {/* Target Invalidation Warning if mismatch */}
          {(deviceMismatch || userMismatch) && (
            <div
              className="error-banner"
              style={{
                display: 'flex',
                flexDirection: 'row',
                alignItems: 'flex-start',
                gap: 10,
              }}
            >
              <AlertTriangle size={16} style={{ flexShrink: 0, marginTop: 2 }} />
              <div>
                <strong>Target mismatch:</strong> The connected device or active profile has changed since this plan was formulated.
                <div style={{ fontSize: 11, marginTop: 3, fontFamily: 'var(--font-mono)', color: 'var(--text-secondary)' }}>
                  Plan Target: {plan.target_device_serial} (User {plan.target_user_id}) · Active Target: {device?.serial || 'None'} (User {user?.id ?? 'None'})
                </div>
              </div>
            </div>
          )}

          {/* Plan Summary Card */}
          <div className="card" style={{ padding: 14 }}>
            <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
              Plan Summary & Rationale
            </div>
            <p style={{ fontSize: 12.5, color: 'var(--text-primary)', lineHeight: 1.5, margin: 0 }}>
              {currentPlan.explanation}
            </p>

            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, alignItems: 'center', marginTop: 10 }}>
              <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>Preserved systems:</span>
              {currentPlan.preserved_functionality.length > 0 ? (
                currentPlan.preserved_functionality.map((func) => (
                  <span key={func} className="badge badge-zinc">
                    {func}
                  </span>
                ))
              ) : (
                <span className="badge badge-zinc">Calls, SMS, Emergency, System UI</span>
              )}
            </div>
          </div>

          {/* Validation Status if triggered */}
          {validationResult && (
            <div
              className="card"
              style={{
                padding: '10px 14px',
                borderLeft: `3px solid ${validationResult.is_valid ? 'var(--accent-teal)' : 'var(--status-negative)'}`,
              }}
            >
              {validationResult.is_valid ? (
                <div style={{ color: 'var(--accent-teal)', display: 'flex', alignItems: 'center', gap: 8, fontSize: 12 }}>
                  <CheckCircle2 size={15} />
                  <span>
                    Pre-check passed: {validationResult.executable_count} executable actions match target device.
                  </span>
                </div>
              ) : (
                <div style={{ color: 'var(--status-negative)', fontSize: 12 }}>
                  <strong style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
                    <ShieldAlert size={14} /> Validation policy warnings:
                  </strong>
                  <ul style={{ margin: 0, paddingLeft: 18 }}>
                    {validationResult.errors.map((err: string, i: number) => (
                      <li key={i}>{err}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}

          {/* Actions State-Diff Table */}
          <div className="data-table-container">
            <div style={{ overflowY: 'auto', maxHeight: 320 }}>
              <table className="data-table">
                <thead>
                  <tr>
                    <th style={{ width: 32, textAlign: 'center' }}>✓</th>
                    <th>Package identifier</th>
                    <th>State transition</th>
                    <th>Reason & evidence</th>
                    <th>Category</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {currentPlan.actions.map((act) => {
                    const isMissing = act.is_executable === false;
                    const isLocked = act.is_protected;
                    const isUnsafeOrUnlisted =
                      act.removal_category === 'Unsafe' || act.removal_category === 'Unlisted';

                    return (
                      <tr
                        key={act.package_id}
                        style={{
                          opacity: act.selected ? 1 : 0.45,
                        }}
                      >
                        <td style={{ textAlign: 'center' }}>
                          <input
                            type="checkbox"
                            checked={act.selected && !isLocked && !isMissing}
                            disabled={isLocked || isMissing || isExecuting}
                            onChange={() => toggleActionSelection(act.package_id)}
                            style={{ cursor: isLocked || isMissing ? 'not-allowed' : 'pointer' }}
                          />
                        </td>
                        <td>
                          <div className="package-id-cell">
                            {act.package_id}
                          </div>
                          {act.package_name && (
                            <div style={{ fontSize: 10.5, color: 'var(--text-muted)' }}>{act.package_name}</div>
                          )}
                        </td>
                        <td>
                          <div className="state-transition">
                            <span style={{ color: 'var(--text-muted)' }}>{packages.find((p) => p.package_id === act.package_id)?.state || 'Unknown'}</span>
                            <ArrowRight size={10} className="transition-arrow" />
                            <span
                              className={`badge ${
                                act.proposed_action === 'Disable'
                                  ? 'badge-amber'
                                  : act.proposed_action === 'UninstallUser'
                                  ? 'badge-rose'
                                  : 'badge-teal'
                              }`}
                            >
                              {act.proposed_action}
                            </span>
                          </div>
                          <div style={{ fontSize: 10, color: 'var(--text-muted)', marginTop: 2 }}>
                            → {getResultingState(act.proposed_action)}
                          </div>
                        </td>
                        <td>
                          <div style={{ fontSize: 11.5, color: 'var(--text-secondary)' }}>{act.reason}</div>
                          {act.evidence && (
                            <div style={{ fontSize: 10, color: 'var(--text-muted)', marginTop: 2 }}>{act.evidence}</div>
                          )}
                        </td>
                        <td>
                          <span
                            className={`badge ${
                              act.removal_category === 'Recommended'
                                ? 'badge-teal'
                                : act.removal_category === 'Advanced'
                                ? 'badge-zinc'
                                : act.removal_category === 'Expert'
                                ? 'badge-amber'
                                : 'badge-rose'
                            }`}
                          >
                            {act.removal_category || 'Unlisted'}
                          </span>
                        </td>
                        <td>
                          {isLocked ? (
                            <span className="badge badge-rose" title="Protected by policy (active launcher or system IME)">
                              <Lock size={10} /> Protected
                            </span>
                          ) : isMissing ? (
                            <span className="badge badge-zinc" title="Package not installed on connected device">
                              <Ban size={10} /> Missing
                            </span>
                          ) : isUnsafeOrUnlisted ? (
                            <span className="badge badge-amber" title="Manual verification recommended">
                              Review
                            </span>
                          ) : (
                            <span className="badge badge-teal">Executable</span>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>

          {/* Execution Progress Bar */}
          {isExecuting && executionProgress && (
            <div className="card" style={{ padding: 12 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, marginBottom: 6 }}>
                <span style={{ color: 'var(--text-primary)' }}>
                  Executing <strong>{executionProgress.action}</strong> on <code>{executionProgress.package_id}</code>...
                </span>
                <span style={{ fontWeight: 600, color: 'var(--accent-teal)', fontVariantNumeric: 'tabular-nums' }}>
                  {executionProgress.current} / {executionProgress.total}
                </span>
              </div>
              <div style={{ height: 4, background: 'var(--surface-raised)', borderRadius: 'var(--radius-xs)', overflow: 'hidden' }}>
                <div
                  style={{
                    height: '100%',
                    width: `${Math.round((executionProgress.current / executionProgress.total) * 100)}%`,
                    background: 'var(--accent-teal)',
                    transition: 'width 0.2s var(--ease-out)',
                  }}
                />
              </div>
            </div>
          )}

          {/* Execution Logs / Outcomes with Technical Diagnostics */}
          {executionResults && (
            <div className="card" style={{ padding: 14, display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-primary)' }}>
                  Verified Execution Results
                </div>
                {executionSummary && (
                  <div style={{ fontSize: 11, color: 'var(--text-muted)', display: 'flex', gap: 12, fontVariantNumeric: 'tabular-nums' }}>
                    <span>Attempted: {executionSummary.attempted_count}</span>
                    <span style={{ color: 'var(--accent-teal)' }}>Succeeded: {executionSummary.succeeded_count}</span>
                    <span style={{ color: executionSummary.failed_count > 0 ? 'var(--status-negative)' : 'var(--text-muted)' }}>Failed: {executionSummary.failed_count}</span>
                    <span style={{ color: executionSummary.skipped_count > 0 ? 'var(--status-warning)' : 'var(--text-muted)' }}>Skipped: {executionSummary.skipped_count}</span>
                  </div>
                )}
              </div>

              {/* Outcomes List with Expandable Diagnostics */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {executionResults.map((r) => {
                  const badge = getOutcomeBadge(r);
                  const isExpanded = !!expandedDiagnostics[r.package_id];
                  return (
                    <div
                      key={r.package_id}
                      style={{
                        background: 'var(--surface-deep)',
                        border: `1px solid ${r.success ? 'var(--border-hairline)' : 'var(--status-negative-border)'}`,
                        borderRadius: 'var(--radius-sm)',
                        padding: '10px 12px',
                      }}
                    >
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                          <span style={{ color: r.success ? 'var(--accent-teal)' : 'var(--status-negative)', fontWeight: 700 }}>
                            {r.success ? '✓' : '✕'}
                          </span>
                          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--text-primary)' }}>
                            {r.package_id}
                          </span>
                          <span className={`badge ${badge.cls}`} style={{ fontSize: 10 }}>
                            {badge.label}
                          </span>
                          {r.execution_backend && (
                            <span className="badge badge-zinc" style={{ fontSize: 10 }}>
                              {r.execution_backend === 'TzHelper' ? 'TZ Helper (app_process)' : 'Direct ADB PM'}
                            </span>
                          )}
                        </div>

                        <button
                          className="btn btn-secondary btn-sm"
                          style={{ height: 22, fontSize: 10, padding: '0 8px', display: 'flex', alignItems: 'center', gap: 4 }}
                          onClick={() => toggleDiagnostic(r.package_id)}
                        >
                          <Terminal size={10} />
                          {isExpanded ? 'Hide diagnostics' : 'Diagnostics'}
                          {isExpanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
                        </button>
                      </div>

                      <div style={{ fontSize: 11.5, color: r.success ? 'var(--text-secondary)' : 'var(--status-negative)', marginTop: 4 }}>
                        {r.message}
                      </div>

                      {/* Expandable Diagnostic Panel */}
                      {isExpanded && (
                        <div
                          className="error-details-expander"
                          style={{
                            marginTop: 8,
                            padding: '10px 12px',
                            display: 'flex',
                            flexDirection: 'column',
                            gap: 6,
                          }}
                        >
                          {r.classified_cause && (
                            <div>
                              <span style={{ color: 'var(--text-muted)' }}>Classified Cause: </span>
                              <strong style={{ color: 'var(--status-warning)' }}>{r.classified_cause}</strong>
                            </div>
                          )}

                          {r.classified_cause === 'MissingPrivileges' && (
                            <div
                              style={{
                                padding: '6px 8px',
                                background: 'var(--status-warning-bg)',
                                border: '1px solid var(--status-warning-border)',
                                borderRadius: 'var(--radius-xs)',
                                color: 'var(--status-warning)',
                                fontSize: 11,
                              }}
                            >
                              💡 <strong>Actionable remedy:</strong> On Xiaomi / MIUI / HyperOS devices, component state modifications (such as <code>disable-user</code>) require enabling <em>"USB debugging (Security settings)"</em> in Developer Options with an active SIM card inserted. Alternatively, <code>pm uninstall -k --user &lt;id&gt;</code> does not require this toggle.
                            </div>
                          )}

                          {r.command_args && r.command_args.length > 0 && (
                            <div>
                              <span style={{ color: 'var(--text-muted)' }}>Command: </span>
                              <code style={{ color: 'var(--text-primary)' }}>
                                adb shell {r.command_args.join(' ')}
                              </code>
                            </div>
                          )}

                          {r.exit_code !== null && r.exit_code !== undefined && (
                            <div>
                              <span style={{ color: 'var(--text-muted)' }}>Exit code: </span>
                              <span style={{ color: r.exit_code === 0 ? 'var(--accent-teal)' : 'var(--status-negative)' }}>
                                {r.exit_code}
                              </span>
                            </div>
                          )}

                          {r.raw_stdout && r.raw_stdout.trim() && (
                            <div>
                              <div style={{ color: 'var(--text-muted)', marginBottom: 2 }}>Standard Output:</div>
                              <pre style={{ background: 'var(--surface-deep)', padding: '6px 8px', borderRadius: 3, margin: 0, overflowX: 'auto', color: 'var(--text-secondary)', fontSize: 10 }}>
                                {r.raw_stdout.trim()}
                              </pre>
                            </div>
                          )}

                          {r.raw_stderr && r.raw_stderr.trim() && (
                            <div>
                              <div style={{ color: 'var(--status-negative)', marginBottom: 2 }}>Standard Error:</div>
                              <pre style={{ background: 'var(--surface-deep)', padding: '6px 8px', borderRadius: 3, margin: 0, overflowX: 'auto', color: 'var(--status-negative)', fontSize: 10 }}>
                                {r.raw_stderr.trim()}
                              </pre>
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>

              {/* Retry Plan Generation if failures or skips occurred */}
              {executionResults.some((r) => !r.success) && (
                <div style={{ display: 'flex', justifyContent: 'flex-start', marginTop: 4 }}>
                  <button
                    className="btn btn-secondary btn-sm"
                    onClick={handleCreateRetryPlan}
                    disabled={isCreatingRetry}
                    style={{ display: 'flex', alignItems: 'center', gap: 6 }}
                  >
                    <RotateCcw size={12} className={isCreatingRetry ? 'spin' : ''} />
                    {isCreatingRetry ? 'Formulating retry plan...' : 'Create retry plan for failed & remaining actions'}
                  </button>
                </div>
              )}
            </div>
          )}

          {/* Execution Controls & Safety Confirmation */}
          {!executionResults && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {/* Batch execution continuation option */}
              <div
                className="card"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '10px 14px',
                }}
              >
                <input
                  type="checkbox"
                  id="continue-on-failure-checkbox"
                  checked={continueOnIndependentFailure}
                  onChange={(e) => setContinueOnIndependentFailure(e.target.checked)}
                  style={{ cursor: 'pointer' }}
                />
                <label
                  htmlFor="continue-on-failure-checkbox"
                  style={{ fontSize: 12, color: 'var(--text-primary)', cursor: 'pointer' }}
                >
                  Continue other supported packages if an independent package fails
                </label>
              </div>

              {/* Confirmation Checkbox */}
              <div
                className="card"
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: 10,
                  padding: '12px 14px',
                }}
              >
                <input
                  type="checkbox"
                  id="ack-risk-checkbox"
                  checked={userAcknowledgedRisk}
                  onChange={(e) => setUserAcknowledgedRisk(e.target.checked)}
                  style={{ marginTop: 2, cursor: 'pointer' }}
                />
                <label
                  htmlFor="ack-risk-checkbox"
                  style={{ fontSize: 12, color: 'var(--text-secondary)', cursor: 'pointer', lineHeight: 1.4 }}
                >
                  I have reviewed the state transitions above for{' '}
                  <strong style={{ color: 'var(--text-primary)' }}>{currentPlan.target_device_serial}</strong> (User{' '}
                  {currentPlan.target_user_id}). I understand that non-root debloating uninstalls or disables packages
                  for this user profile, and pre-change state snapshots are recorded in the local audit journal.
                </label>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer */}
        <div className="modal-footer" style={{ justifyContent: 'space-between' }}>
          <div>
            <button
              className="btn btn-secondary btn-sm"
              onClick={handleValidatePlan}
              disabled={isExecuting || deviceMismatch || userMismatch}
            >
              Pre-check policy
            </button>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            {isExecuting && (
              <button
                className="btn btn-danger btn-sm"
                onClick={() =>
                  invoke('cancel_device_operations', { deviceSerial: currentPlan.target_device_serial }).catch((e) =>
                    setErrorMessage(String(e))
                  )
                }
              >
                Stop after current action
              </button>
            )}
            <button className="btn btn-secondary btn-sm" onClick={onClose} disabled={isExecuting}>
              {executionResults ? 'Close' : 'Cancel'}
            </button>
            {!executionResults && (
              <button
                className="btn btn-primary btn-sm"
                onClick={handleApproveAndExecute}
                disabled={
                  isExecuting ||
                  !userAcknowledgedRisk ||
                  selectedCount === 0 ||
                  deviceMismatch ||
                  userMismatch
                }
              >
                <Play size={13} />
                Approve & execute {selectedCount} action{selectedCount !== 1 ? 's' : ''}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
