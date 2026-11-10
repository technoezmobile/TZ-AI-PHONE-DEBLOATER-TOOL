import React, { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import {
  Send,
  Square,
  RefreshCw,
  AlertTriangle,
  ArrowRight,
  Trash2,
  ShieldCheck,
  Package,
  Bot,
  ListFilter
} from 'lucide-react';
import { MessageText } from './MessageText';
import { CandidateListModal } from './CandidateListModal';
import {
  ActionPlan,
  AndroidUser,
  AppConfig,
  ChatMessage,
  ClassifiedError,
  DeviceInfo,
  DevicePackage,
  DiscoveredModel,
  ModelDiscoveryResult,
  OperationOutcome,
  ShortcutIntent,
  StreamChunkPayload,
  StreamDonePayload,
  StreamErrorPayload
} from '../types';

interface Props {
  device: DeviceInfo | null;
  user: AndroidUser | null;
  packages: DevicePackage[];
  config: AppConfig;
  lastExecutionReport?: { plan: ActionPlan; outcomes: OperationOutcome[] } | null;
  onOpenPlanReview: (plan: ActionPlan) => void;
  onSelectPackage?: (pkgId: string) => void;
  onUpdateConsent: (consent: boolean) => void;
  onUpdateConfig: (updated: AppConfig) => Promise<void>;
}

export const AiAssistant: React.FC<Props> = ({
  device,
  user,
  packages,
  config,
  lastExecutionReport,
  onOpenPlanReview,
  onSelectPackage,
  onUpdateConfig,
}) => {
  const [messages, setMessages] = useState<ChatMessage[]>([
    {
      id: 'welcome',
      role: 'assistant',
      content:
        'I am your debloating assistant inside **TZ AI DEBLOATER TOOL**.\n\nDescribe what you want removed or preserved on your phone (e.g. *"remove promotional apps, analytics bloatware, and Facebook tracking while keeping camera, banking, and system updates"*). I will inspect your device catalog and produce an explicit, reviewable action plan.',
      timestamp: Date.now(),
    },
  ]);

  const [input, setInput] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [currentRequestId, setCurrentRequestId] = useState<string | null>(null);
  const [showCandidateModal, setShowCandidateModal] = useState(false);

  // Active Connection & Discovered Models
  const activeConn = config.connections.find((c) => c.id === config.active_connection_id) || config.connections[0];
  const [discoveredModels, setDiscoveredModels] = useState<DiscoveredModel[]>([]);
  const [isDiscoveringModels, setIsDiscoveringModels] = useState(false);

  // Error & Diagnostic state
  const [activeError, setActiveError] = useState<ClassifiedError | null>(null);
  const [showErrorDiagnostics, setShowErrorDiagnostics] = useState(false);

  const activeRequestRef = useRef<string | null>(null);
  const listenersReady = useRef<Promise<unknown>>(Promise.resolve());
  const discoverySequence = useRef(0);
  const [modelSearch, setModelSearch] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const taRef = useRef<HTMLTextAreaElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages, isStreaming]);

  // Bounded auto-growing textarea (liquid-multimodal-input pattern)
  useEffect(() => {
    const ta = taRef.current;
    if (!ta) return;
    ta.style.height = 'auto';
    ta.style.height = `${Math.min(Math.max(ta.scrollHeight, 44), 140)}px`;
  }, [input]);

  // Post plan execution results into the conversation thread when completed
  useEffect(() => {
    if (!lastExecutionReport) return;
    const { plan, outcomes } = lastExecutionReport;

    const alreadyRemoved = outcomes.filter(
      (o) =>
        o.message?.toLowerCase().includes('already uninstalled') ||
        o.message?.toLowerCase().includes('already disabled') ||
        o.message?.toLowerCase().includes('already removed') ||
        o.message?.toLowerCase().includes('redundant') ||
        o.message?.toLowerCase().includes('already in requested state')
    );
    const newlyExecuted = outcomes.filter((o) => !alreadyRemoved.includes(o));
    const newlySucc = newlyExecuted.filter((o) => o.success).length;
    const newlyFail = newlyExecuted.filter((o) => !o.success).length;
    const selectedCount = plan.actions.filter((a) => a.selected && !a.is_protected && a.is_executable !== false).length;
    const skipped = selectedCount > outcomes.length ? selectedCount - outcomes.length : 0;

    let summaryText = `### Plan Execution Report\n\n` +
      `**Target Device**: \`${plan.target_device_serial}\` · **User Profile**: \`User ${plan.target_user_id}\`\n\n` +
      `**Summary**: **${newlySucc}** newly succeeded, **${alreadyRemoved.length}** already removed / in requested state, **${newlyFail}** failed, **${skipped}** skipped (${selectedCount} total requested).\n\n` +
      `| Package | Action | Result | Details |\n` +
      `| :--- | :--- | :--- | :--- |\n`;

    for (const o of outcomes) {
      const isAlready = alreadyRemoved.includes(o);
      const statusLabel = isAlready ? 'ℹ Already Removed' : o.success ? '✓ Success' : '✕ Failed';
      const cleanMsg = (o.message || '').replace(/\|/g, '\\|');
      summaryText += `| \`${o.package_id}\` | ${o.action} | ${statusLabel} | ${cleanMsg} |\n`;
    }

    if (newlyFail > 0 || skipped > 0) {
      summaryText += `\n> [!NOTE]\n> Failed or skipped actions can be retried using the **Create retry plan** button in the review modal or by asking below.`;
    }

    setMessages((prev) => [
      ...prev,
      {
        id: `exec-report-${Date.now()}`,
        role: 'assistant',
        content: summaryText,
        timestamp: Date.now(),
      },
    ]);
  }, [lastExecutionReport]);

  // Discover models for active connection
  const handleDiscoverModels = async () => {
    if (!activeConn) return;
    const sequence = ++discoverySequence.current;
    setIsDiscoveringModels(true);
    try {
      const all: DiscoveredModel[] = [];
      const seen = new Set<string>();
      let token: string | null = null;
      do {
        const res: ModelDiscoveryResult = await invoke('discover_models', {
          connectionId: activeConn.id,
          pageToken: token,
        });
        if (sequence !== discoverySequence.current) return;
        all.push(...res.models);
        token = res.next_page_token;
        if (token && seen.has(token)) throw new Error('Repeated model pagination token');
        if (token) seen.add(token);
      } while (token && seen.size < 100);

      // De-duplicate by id
      const unique = Array.from(new Map(all.map((m) => [m.id, m])).values());
      setDiscoveredModels(unique);
      setActiveError(null);
    } catch (e: any) {
      console.error('Model discovery error:', e);
      setActiveError({
        error_type: 'ModelUnavailable',
        message: e.message || 'Model discovery failed',
        suggested_action: 'Check connection credentials or enter manual model ID in Settings',
        diagnostic_details: JSON.stringify(e),
        http_status: null,
      });
    } finally {
      setIsDiscoveringModels(false);
    }
  };

  // Sync selected model across chat picker and settings
  const handleSelectModel = async (modelId: string) => {
    if (!activeConn) return;
    const updated = config.connections.map((c) =>
      c.id === activeConn.id ? { ...c, selected_model: modelId, manual_model_id: null } : c
    );
    await onUpdateConfig({ ...config, connections: updated });
  };

  // Change active connection profile
  const handleSelectConnection = async (connId: string) => {
    await handleStopGeneration();
    await onUpdateConfig({ ...config, active_connection_id: connId });
    setDiscoveredModels([]);
    setActiveError(null);
  };

  // Clear conversation history
  const handleClearChat = () => {
    if (isStreaming) return;
    setMessages([
      {
        id: 'welcome',
        role: 'assistant',
        content:
          'I am your debloating assistant inside **TZ AI DEBLOATER TOOL**.\n\nDescribe what you want removed or preserved on your phone (e.g. *"remove promotional apps, analytics bloatware, and Facebook tracking while keeping camera, banking, and system updates"*). I will inspect your device catalog and produce an explicit, reviewable action plan.',
        timestamp: Date.now(),
      },
    ]);
    setActiveError(null);
  };

  // Setup streaming event listeners
  useEffect(() => {
    const unlistenChunk = listen<StreamChunkPayload>('ai-stream-chunk', (event) => {
      if (event.payload.request_id === activeRequestRef.current && activeRequestRef.current !== null) {
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.role === 'assistant') {
            const updated = { ...last, content: last.content + event.payload.text };
            return [...prev.slice(0, -1), updated];
          } else {
            return [
              ...prev,
              {
                id: Date.now().toString(),
                role: 'assistant',
                content: event.payload.text,
                timestamp: Date.now(),
              },
            ];
          }
        });
      }
    });

    const unlistenPlan = listen<{ request_id: string; plan: ActionPlan; validation: any }>('ai-plan-generated', (event) => {
      if (event.payload.request_id !== activeRequestRef.current || !activeRequestRef.current) return;
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.role === 'assistant') {
          const updated = {
            ...last,
            plan_id: event.payload.plan.id,
            plan: event.payload.plan,
          };
          return [...prev.slice(0, -1), updated];
        }
        return prev;
      });
    });

    const unlistenDone = listen<StreamDonePayload>('ai-stream-done', (event) => {
      if (event.payload.request_id === activeRequestRef.current && activeRequestRef.current !== null) {
        setIsStreaming(false);
        activeRequestRef.current = null;
        setCurrentRequestId(null);
        if (event.payload.served_model) {
          setMessages((prev) => {
            const last = prev[prev.length - 1];
            if (last && last.role === 'assistant' && !last.served_model) {
              return [...prev.slice(0, -1), { ...last, served_model: event.payload.served_model }];
            }
            return prev;
          });
        }
      }
    });

    const unlistenError = listen<StreamErrorPayload>('ai-stream-error', (event) => {
      if (event.payload.request_id === activeRequestRef.current && activeRequestRef.current !== null) {
        setIsStreaming(false);
        activeRequestRef.current = null;
        setCurrentRequestId(null);
        setActiveError(event.payload.error);
        setMessages((prev) => {
          const last = prev[prev.length - 1];
          if (last && last.role === 'assistant' && !last.content.trim()) {
            return [
              ...prev.slice(0, -1),
              {
                ...last,
                content: `> [!WARNING]\n> Request failed: **${event.payload.error.error_type}**\n> ${event.payload.error.message}\n> Next step: ${event.payload.error.suggested_action}`,
              },
            ];
          }
          return prev;
        });
      }
    });

    listenersReady.current = Promise.all([unlistenChunk, unlistenPlan, unlistenDone, unlistenError]);
    return () => {
      const id = activeRequestRef.current;
      activeRequestRef.current = null;
      if (id) void invoke('cancel_ai_request', { requestId: id });
      unlistenChunk.then((f) => f());
      unlistenPlan.then((f) => f());
      unlistenDone.then((f) => f());
      unlistenError.then((f) => f());
    };
  }, []);

  if (!activeConn) return <div className="card">Loading AI connections...</div>;

  // Send Prompt with Streaming
  const handleSendPrompt = async (promptText?: string, intent?: ShortcutIntent | null) => {
    const textToSend = promptText || input;
    if (!textToSend.trim() || isStreaming) return;

    if (!device || device.state !== 'Ready') {
      setActiveError({
        error_type: 'Unknown',
        message: 'No ready Android device selected',
        suggested_action: 'Connect your device via USB or Wireless ADB in the Devices view',
        diagnostic_details: null,
        http_status: null,
      });
      return;
    }

    if (!user) {
      setActiveError({
        error_type: 'Unknown',
        message: 'No Android user profile selected',
        suggested_action: 'Select an active user profile in the top bar',
        diagnostic_details: null,
        http_status: null,
      });
      return;
    }

    if (!config.ai_consent_given) {
      setActiveError({
        error_type: 'PermissionAccess',
        message: 'AI cloud analysis consent has not been granted',
        suggested_action: 'Enable consent in Settings to allow sanitized package recommendations',
        diagnostic_details: null,
        http_status: null,
      });
      return;
    }

    const effectiveModel =
      activeConn.manual_model_id ||
      activeConn.selected_model ||
      (activeConn.protocol === 'GeminiNative' ? 'gemini-3.5-flash' : '');

    if (!effectiveModel) {
      setActiveError({
        error_type: 'ModelUnavailable',
        message: 'No model selected for the active connection',
        suggested_action: 'Click "Refresh models" in the toolbar or specify a model in Settings',
        diagnostic_details: null,
        http_status: null,
      });
      return;
    }

    const reqId = `req-${Date.now()}`;
    activeRequestRef.current = reqId;
    setCurrentRequestId(reqId);
    setActiveError(null);

    const userMsg: ChatMessage = {
      id: Date.now().toString(),
      role: 'user',
      content: textToSend.trim(),
      timestamp: Date.now(),
    };

    const nextMessages = [...messages, userMsg];
    setMessages([...nextMessages, { id: `assistant-${reqId}`, role: 'assistant', content: '', timestamp: Date.now() }]);
    setInput('');
    if (taRef.current) {
      taRef.current.style.height = '44px';
    }
    setIsStreaming(true);

    try {
      await listenersReady.current;
      await invoke('stream_ai_chat', {
        requestId: reqId,
        connectionId: activeConn.id,
        userPrompt: textToSend.trim(),
        deviceSerial: device.serial,
        userId: user.id,
        conversationHistory: intent
          ? []
          : messages
              .filter((m) => m.id !== 'welcome' && !m.plan && m.content.trim())
              .slice(-20),
        shortcutIntent: intent ?? null,
      });
    } catch (e: any) {
      if (activeRequestRef.current !== reqId) return;
      activeRequestRef.current = null;
      setIsStreaming(false);
      setCurrentRequestId(null);
      setActiveError(
        typeof e === 'object' && e.error_type
          ? e
          : {
              error_type: 'Unknown',
              message: String(e?.message || e),
              suggested_action: 'Check connection settings and endpoint credentials',
              diagnostic_details: null,
              http_status: null,
            }
      );
    }
  };

  // Stop Generation / Cancel backend request
  const handleStopGeneration = async () => {
    if (currentRequestId) {
      const stoppedId = currentRequestId;
      activeRequestRef.current = null;
      try {
        await invoke('cancel_ai_request', { requestId: stoppedId });
      } catch (e) {
        console.error('Cancel request error:', e);
      }
      setIsStreaming(false);
      setCurrentRequestId(null);
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.role === 'assistant') {
          const finalContent = last.content.trim()
            ? `${last.content}\n\n> [!NOTE]\n> Generation stopped by operator.`
            : `> [!NOTE]\n> Generation stopped by operator.`;
          return [...prev.slice(0, -1), { ...last, content: finalContent }];
        }
        return prev;
      });
    }
  };

  const shortcuts = [
    {
      id: 'bloatware',
      label: 'Review bloatware',
      intent: 'ReviewBloatware' as const,
      prompt:
        'Review bloatware: show recommended debloat candidates across OEM, carrier, and promotional packages with reasons and a structured plan for review.',
      tooltip: 'Inspect recommended debloat candidates across OEM and carrier packages',
    },
    {
      id: 'trackers',
      label: 'Review trackers',
      intent: 'ReviewTrackers' as const,
      prompt:
        'Review trackers: restrict candidates strictly to catalog-supported tracking, telemetry, and analytics packages. Leave unrelated promotional apps out.',
      tooltip: 'Restrict candidates strictly to catalog-supported tracking and analytics packages',
    },
    {
      id: 'promos',
      label: 'Uninstall promos',
      intent: 'UninstallPromos' as const,
      prompt:
        'Uninstall promos: propose Uninstall for this user only for eligible promotional apps installed for this profile. Keep explicit confirmation.',
      tooltip: 'Propose per-user uninstall for eligible promotional apps installed for this profile',
    },
    {
      id: 'candidates',
      label: 'List candidates',
      intent: null,
      prompt: '',
      tooltip: 'Open read-only candidate inventory for the active profile (no plan generated)',
    },
  ];

  const currentEffectiveModel =
    activeConn.manual_model_id ||
    activeConn.selected_model ||
    (activeConn.protocol === 'GeminiNative' ? 'gemini-3.5-flash' : '');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 10 }}>
      {/* Chat Header Status & Actions */}
      <div className="chat-toolbar">
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <Bot size={15} style={{ color: 'var(--accent-teal)' }} />
          <span style={{ fontWeight: 600, fontSize: 13, color: 'var(--text-primary)' }}>AI Debloat Assistant</span>
          <span className="badge badge-zinc">{activeConn.protocol}</span>
          {activeConn.tested_for_generation ? (
            <span className="badge badge-teal">Verified</span>
          ) : (
            <span className="badge badge-zinc">Untested</span>
          )}
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <button
            className="btn btn-secondary btn-sm"
            onClick={handleClearChat}
            disabled={isStreaming}
            title="Clear conversation thread"
            style={{ height: 26, padding: '0 8px' }}
          >
            <Trash2 size={11} />
            Clear chat
          </button>
        </div>
      </div>

      {/* Error Banner */}
      {activeError && (
        <div className="error-banner">
          <div className="error-banner-header">
            <AlertTriangle size={15} />
            <span>{activeError.error_type}</span>
          </div>
          <div>{activeError.message}</div>
          <div style={{ color: 'var(--text-secondary)', fontSize: 11 }}>
            Next step: {activeError.suggested_action}
          </div>
          {activeError.diagnostic_details && (
            <div>
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setShowErrorDiagnostics(!showErrorDiagnostics)}
                style={{ fontSize: 10, height: 22, padding: '0 6px' }}
              >
                {showErrorDiagnostics ? 'Hide diagnostics' : 'Show diagnostics'}
              </button>
              {showErrorDiagnostics && (
                <div className="error-details-expander" style={{ marginTop: 6 }}>
                  {activeError.diagnostic_details}
                </div>
              )}
            </div>
          )}
        </div>
      )}

      {/* Messages Thread */}
      <div className="chat-thread">
        {messages.map((msg) => (
          <div key={msg.id} className={`chat-message ${msg.role}`}>
            <div className="message-meta">
              <span>{msg.role === 'user' ? 'Operator' : 'Debloater Engine'}</span>
              <span>·</span>
              <span className="tabular-nums">
                {new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
              {msg.served_model && (
                <span className="badge badge-zinc" style={{ fontSize: 10, padding: '1px 6px' }}>
                  {msg.served_model}
                </span>
              )}
            </div>
            <div className="message-bubble" style={{ wordBreak: 'break-word', overflowWrap: 'anywhere' }}>
              {isStreaming && msg.role === 'assistant' && !msg.content ? (
                <div className="assistant-loading-indicator">
                  <span className="pulse-dot" />
                  <span>Analyzing device catalog and formulating recommendations...</span>
                </div>
              ) : (
                <MessageText text={msg.content} />
              )}

              {/* Plan Card if Structured Actions Proposed */}
              {msg.plan && (
                <div
                  className="card"
                  style={{
                    marginTop: 12,
                    padding: 14,
                    background: 'var(--surface-deep)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 10,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 6 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                      <ShieldCheck size={16} style={{ color: 'var(--accent-teal)' }} />
                      <span style={{ fontWeight: 600, fontSize: 13 }}>Structured Debloating Plan</span>
                    </div>
                    <span className="badge badge-zinc">{msg.plan.actions.length} proposed actions</span>
                  </div>

                  <p style={{ fontSize: 12, color: 'var(--text-secondary)', margin: 0 }}>
                    {msg.plan.explanation}
                  </p>

                  {/* Proposed Package Badges */}
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 4 }}>
                    {msg.plan.actions.map((act) => (
                      <button
                        key={act.package_id}
                        className={`badge ${
                          act.proposed_action === 'UninstallUser'
                            ? 'badge-rose'
                            : act.proposed_action === 'Disable'
                            ? 'badge-amber'
                            : 'badge-teal'
                        }`}
                        style={{
                          cursor: onSelectPackage ? 'pointer' : 'default',
                          display: 'flex',
                          alignItems: 'center',
                          gap: 4,
                          fontSize: 11,
                          padding: '2px 7px',
                          maxWidth: '100%',
                          wordBreak: 'break-all',
                        }}
                        title={`Action: ${act.proposed_action}${onSelectPackage ? ' — click to inspect package' : ''}`}
                        onClick={() => onSelectPackage?.(act.package_id)}
                      >
                        <Package size={10} style={{ flexShrink: 0 }} />
                        <span>{act.package_id}</span>
                        <span style={{ opacity: 0.8 }}>({act.proposed_action})</span>
                      </button>
                    ))}
                  </div>

                  <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 4 }}>
                    <button
                      className="btn btn-primary btn-sm"
                      onClick={() => onOpenPlanReview(msg.plan!)}
                      style={{ display: 'flex', alignItems: 'center', gap: 6 }}
                    >
                      Review & Execute Plan
                      <ArrowRight size={13} />
                    </button>
                  </div>
                </div>
              )}
            </div>
          </div>
        ))}
        <div ref={messagesEndRef} />
      </div>

      {/* Action Shortcuts */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, padding: '2px 0', flexShrink: 0 }}>
        {shortcuts.map((s) => (
          <button
            key={s.id}
            className="btn btn-secondary btn-sm"
            style={{ fontSize: 11, padding: '4px 10px', whiteSpace: 'nowrap' }}
            onClick={() => {
              if (s.id === 'candidates') {
                setShowCandidateModal(true);
              } else {
                handleSendPrompt(s.prompt, s.intent);
              }
            }}
            disabled={isStreaming}
            title={s.tooltip}
          >
            {s.id === 'candidates' && <ListFilter size={11} style={{ marginRight: 4 }} />}
            {s.label}
          </button>
        ))}
      </div>

      {/* Adapted Multimodal Composer (liquid-multimodal-input pattern) */}
      <div className="composer-card">
        <textarea
          ref={taRef}
          className="composer-textarea"
          placeholder="Describe packages to remove, preserve, or inspect..."
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              handleSendPrompt();
            }
          }}
          disabled={isStreaming}
          rows={1}
        />

        <div className="composer-toolbar">
          <div className="composer-controls-left">
            {/* Connection Selector */}
            <div className="chat-control-item">
              <span>Connection:</span>
              <select
                className="chat-control-select"
                value={activeConn.id}
                disabled={isStreaming}
                onChange={(e) => handleSelectConnection(e.target.value)}
              >
                {config.connections.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Model Selector with real model IDs */}
            <div className="chat-control-item">
              <span>Model:</span>
              <input
                aria-label="Search models"
                className="input-field"
                placeholder="Filter..."
                value={modelSearch}
                onChange={(e) => setModelSearch(e.target.value)}
                style={{ width: 85, height: 24, fontSize: 11 }}
              />
              <select
                className="chat-control-select"
                value={currentEffectiveModel}
                disabled={isStreaming}
                onChange={(e) => handleSelectModel(e.target.value)}
                style={{ maxWidth: 170 }}
              >
                <option value="">Choose a model</option>
                {discoveredModels
                  .filter((m) => `${m.display_name} ${m.id}`.toLowerCase().includes(modelSearch.toLowerCase()))
                  .map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.display_name} · {m.id}
                    </option>
                  ))}
                {!discoveredModels.length && currentEffectiveModel && (
                  <option value={currentEffectiveModel}>{currentEffectiveModel}</option>
                )}
              </select>

              <button
                className="btn btn-secondary btn-sm"
                onClick={handleDiscoverModels}
                disabled={isDiscoveringModels}
                title="Refresh models from endpoint"
                style={{ height: 24, padding: '0 6px' }}
              >
                <RefreshCw size={11} className={isDiscoveringModels ? 'spin' : ''} />
              </button>
            </div>
          </div>

          <div className="composer-controls-right">
            <span style={{ fontSize: 10.5, color: 'var(--text-muted)' }}>
              <span className="keycap">Enter</span> send · <span className="keycap">Shift</span>+<span className="keycap">Enter</span> newline
            </span>

            {isStreaming ? (
              <button
                className="btn btn-danger btn-sm"
                onClick={handleStopGeneration}
                style={{ height: 28, padding: '0 12px', display: 'flex', alignItems: 'center', gap: 5 }}
              >
                <Square size={12} />
                Stop
              </button>
            ) : (
              <button
                className="btn btn-primary btn-sm"
                onClick={() => handleSendPrompt()}
                disabled={!input.trim()}
                style={{ height: 28, padding: '0 12px', display: 'flex', alignItems: 'center', gap: 5 }}
              >
                <Send size={12} />
                Send
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Candidate List Modal */}
      <CandidateListModal
        isOpen={showCandidateModal}
        onClose={() => setShowCandidateModal(false)}
        device={device}
        user={user}
        packages={packages}
      />
    </div>
  );
};
