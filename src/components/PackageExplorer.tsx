import React, { useState, useMemo, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Search,
  CheckSquare,
  Square,
  RotateCcw,
  Trash2,
  PowerOff,
  Power,
  X,
  ChevronRight,
  Lock,
  ShieldCheck,
  AlertTriangle,
  CheckCircle2,
  Info,
  Filter
} from 'lucide-react';
import {
  ActionType,
  DeviceInfo,
  DevicePackage,
  AndroidUser,
  OperationOutcome,
  AndroidPackageType,
  BatchPreflightResult
} from '../types';
import { useDebounce } from '../hooks/useDebounce';

export type ExplorerView = 'candidates' | 'all' | 'system' | 'removed';

interface Props {
  device: DeviceInfo | null;
  user: AndroidUser | null;
  packages: DevicePackage[];
  isLoading: boolean;
  onRefresh: () => void;
}

const ROM_UTILITIES = new Set([
  'com.miui.compass',
  'com.miui.notes',
  'com.miui.calculator',
  'com.miui.weather2',
  'com.miui.screenrecorder',
  'com.miui.soundrecorder',
  'com.xiaomi.scanner',
  'com.miui.fm',
  'com.sec.android.app.popupcalculator',
  'com.samsung.android.app.notes',
  'com.google.android.calculator',
  'com.google.android.deskclock'
]);

export const PackageExplorer: React.FC<Props> = ({
  device,
  user,
  packages,
  isLoading,
  onRefresh,
}) => {
  // Views: Default is 'candidates' (Debloat candidates)
  const [activeView, setActiveView] = useState<ExplorerView>('candidates');
  const [searchInput, setSearchInput] = useState('');
  const debouncedSearch = useDebounce(searchInput, 150);
  const [stateFilter, setStateFilter] = useState<'All' | 'Enabled' | 'Disabled' | 'Uninstalled'>('All');
  const [categoryFilter, setCategoryFilter] = useState<string>('All');
  const [originFilter, setOriginFilter] = useState<string>('All');
  const [selectedPackages, setSelectedPackages] = useState<Set<string>>(new Set());
  const [activePackageDetail, setActivePackageDetail] = useState<DevicePackage | null>(null);

  const hasActiveFilters = useMemo(() => {
    const isCustomView = activeView !== 'candidates';
    const hasSearch = searchInput.trim().length > 0;
    const hasState = stateFilter !== 'All';
    const hasCategory = categoryFilter !== 'All';
    const hasOrigin = originFilter !== 'All';
    return isCustomView || hasSearch || hasState || hasCategory || hasOrigin;
  }, [activeView, searchInput, stateFilter, categoryFilter, originFilter]);

  const resetAllFilters = () => {
    setActiveView('candidates');
    setSearchInput('');
    setStateFilter('All');
    setCategoryFilter('All');
    setOriginFilter('All');
  };

  // Notifications
  const [feedbackMessage, setFeedbackMessage] = useState<{ text: string; type: 'success' | 'error' } | null>(null);

  // Batch Action & Preflight Revalidation Modal
  const [pendingBatchAction, setPendingBatchAction] = useState<ActionType | null>(null);
  const [preflightResult, setPreflightResult] = useState<BatchPreflightResult | null>(null);
  const [isPreflightValidating, setIsPreflightValidating] = useState(false);
  const [preflightError, setPreflightError] = useState<string | null>(null);
  const batchCancelRef = useRef(false);
  const [isExecutingBatch, setIsExecutingBatch] = useState(false);
  const [batchProgress, setBatchProgress] = useState<{ current: number; total: number } | null>(null);
  const [batchResults, setBatchResults] = useState<{ succeeded: number; failed: number } | null>(null);

  useEffect(() => {
    setSelectedPackages(new Set());
    setActivePackageDetail(null);
    setPendingBatchAction(null);
    setPreflightResult(null);
    batchCancelRef.current = true;
  }, [device?.serial, user?.id]);

  // Compute view package counts
  const viewCounts = useMemo(() => {
    let candidates = 0;
    let system = 0;
    let removed = 0;

    for (const pkg of packages) {
      if (pkg.state === 'Uninstalled') {
        removed++;
      } else if (pkg.removal_eligibility === 'Eligible') {
        candidates++;
      }

      const isSys =
        pkg.package_type === 'System' ||
        pkg.package_type === 'UpdatedSystem' ||
        (pkg.list_category || '').toLowerCase() === 'oem' ||
        pkg.package_id.startsWith('com.mediatek.') ||
        pkg.package_id.startsWith('vendor.mediatek.') ||
        pkg.package_id.startsWith('com.miui.') ||
        pkg.package_id.startsWith('com.xiaomi.') ||
        pkg.package_id.startsWith('com.sec.') ||
        pkg.package_id.startsWith('com.samsung.') ||
        pkg.package_id.startsWith('com.qualcomm.');
      if (isSys) {
        system++;
      }
    }

    return {
      candidates,
      all: packages.length,
      system,
      removed,
    };
  }, [packages]);

  // Filter packages based on active view and search/dropdown filters
  const filteredPackages = useMemo(() => {
    return packages.filter((pkg) => {
      // 1. View filter
      if (activeView === 'candidates') {
        if (pkg.state === 'Uninstalled' || pkg.removal_eligibility !== 'Eligible') {
          return false;
        }
      } else if (activeView === 'removed') {
        if (pkg.state !== 'Uninstalled') {
          return false;
        }
      } else if (activeView === 'system') {
        const isSys =
          pkg.package_type === 'System' ||
          pkg.package_type === 'UpdatedSystem' ||
          (pkg.list_category || '').toLowerCase() === 'oem' ||
          pkg.package_id.startsWith('com.mediatek.') ||
          pkg.package_id.startsWith('vendor.mediatek.') ||
          pkg.package_id.startsWith('com.miui.') ||
          pkg.package_id.startsWith('com.xiaomi.') ||
          pkg.package_id.startsWith('com.sec.') ||
          pkg.package_id.startsWith('com.samsung.') ||
          pkg.package_id.startsWith('com.qualcomm.');
        if (!isSys) return false;
      }

      // 2. Search query (debounced)
      if (debouncedSearch.trim()) {
        const q = debouncedSearch.toLowerCase();
        const matchesId = pkg.package_id.toLowerCase().includes(q);
        const matchesDesc = (pkg.description || '').toLowerCase().includes(q);
        if (!matchesId && !matchesDesc) return false;
      }

      // 3. State filter (when not in 'removed' view)
      if (activeView !== 'removed' && stateFilter !== 'All' && pkg.state !== stateFilter) {
        return false;
      }

      // 4. Secondary Category filter
      if (categoryFilter !== 'All') {
        if (categoryFilter === 'System' && pkg.package_type === 'User') return false;
        if (categoryFilter === 'User' && pkg.package_type !== 'User') return false;
        if (
          ['Recommended', 'Advanced', 'Expert', 'Unsafe', 'Unlisted'].includes(categoryFilter) &&
          pkg.removal_recommendation.toLowerCase() !== categoryFilter.toLowerCase()
        ) {
          return false;
        }
      }

      // 5. Secondary Origin filter
      if (originFilter !== 'All' && (pkg.list_category || '').toLowerCase() !== originFilter.toLowerCase()) {
        return false;
      }

      return true;
    });
  }, [packages, activeView, debouncedSearch, stateFilter, categoryFilter, originFilter]);

  // Hidden selected packages calculation
  const visibleSelectedCount = useMemo(() => {
    let count = 0;
    for (const pkg of filteredPackages) {
      if (selectedPackages.has(pkg.package_id)) {
        count++;
      }
    }
    return count;
  }, [filteredPackages, selectedPackages]);

  const hiddenSelectedCount = selectedPackages.size - visibleSelectedCount;

  // Selection handlers
  const toggleSelect = (pkgId: string) => {
    setSelectedPackages((prev) => {
      const next = new Set(prev);
      if (next.has(pkgId)) {
        next.delete(pkgId);
      } else {
        next.add(pkgId);
      }
      return next;
    });
  };

  const selectAllVisible = () => {
    const next = new Set(selectedPackages);
    for (const pkg of filteredPackages) {
      if (!pkg.is_protected) {
        next.add(pkg.package_id);
      }
    }
    setSelectedPackages(next);
  };

  const clearSelection = () => {
    setSelectedPackages(new Set());
  };

  // Single Action Execution
  const handleSingleAction = async (packageId: string, action: ActionType) => {
    if (!device || !user) return;
    setFeedbackMessage(null);

    try {
      const outcome: OperationOutcome = await invoke('execute_package_action', {
        deviceSerial: device.serial,
        userId: user.id,
        packageId,
        action,
      });

      if (outcome.success && outcome.verified) {
        setFeedbackMessage({ text: `${action} executed on ${packageId}`, type: 'success' });
        onRefresh();
        if (activePackageDetail && activePackageDetail.package_id === packageId) {
          setActivePackageDetail((prev) =>
            prev
              ? {
                  ...prev,
                  state:
                    action === 'Disable'
                      ? 'Disabled'
                      : action === 'UninstallUser'
                      ? 'Uninstalled'
                      : 'Enabled',
                }
              : null
          );
        }
      } else {
        setFeedbackMessage({ text: `Failed on ${packageId}: ${outcome.message}`, type: 'error' });
      }
    } catch (e: any) {
      setFeedbackMessage({ text: `Error: ${e?.message || e}`, type: 'error' });
    }
  };

  // Preflight Batch Revalidation
  const initiateBatch = async (action: ActionType) => {
    if (!device || !user || selectedPackages.size === 0) return;
    setPendingBatchAction(action);
    setPreflightResult(null);
    setPreflightError(null);
    setBatchResults(null);
    setIsPreflightValidating(true);

    try {
      const res = await invoke<BatchPreflightResult>('validate_batch_selection', {
        deviceSerial: device.serial,
        userId: user.id,
        action,
        packageIds: Array.from(selectedPackages),
      });
      setPreflightResult(res);
    } catch (err: any) {
      setPreflightError(err?.message || String(err));
    } finally {
      setIsPreflightValidating(false);
    }
  };

  // Batch Action Execution
  const executeBatch = async () => {
    if (!device || !user || !pendingBatchAction || !preflightResult) return;
    const eligibleItems = preflightResult.items.filter((item) => item.eligible);
    if (eligibleItems.length === 0) return;

    batchCancelRef.current = false;
    setIsExecutingBatch(true);
    const count = eligibleItems.length;
    setBatchProgress({ current: 0, total: count });
    setBatchResults(null);

    let succeeded = 0;
    let failed = 0;
    let index = 0;

    for (const item of eligibleItems) {
      if (batchCancelRef.current) break;
      setBatchProgress({ current: index + 1, total: count });
      try {
        const outcome: OperationOutcome = await invoke('execute_package_action', {
          deviceSerial: device.serial,
          userId: user.id,
          packageId: item.package_id,
          action: pendingBatchAction,
        });
        if (outcome.success && outcome.verified) {
          succeeded++;
        } else {
          failed++;
        }
      } catch (e) {
        failed++;
      }
      index++;
    }

    setIsExecutingBatch(false);
    setBatchProgress(null);
    setBatchResults({ succeeded, failed });
    setSelectedPackages(new Set());
    onRefresh();
  };

  const getPackageTypeBadge = (type: AndroidPackageType) => {
    switch (type) {
      case 'System':
        return <span className="badge badge-zinc">System</span>;
      case 'UpdatedSystem':
        return <span className="badge badge-zinc" style={{ color: 'var(--accent-teal)', borderColor: 'var(--border-hairline)' }}>Updated System</span>;
      case 'User':
        return <span className="badge badge-zinc">User</span>;
      default:
        return <span className="badge badge-zinc">{type}</span>;
    }
  };

  const getRemovalBadge = (removal: string) => {
    switch (removal.toLowerCase()) {
      case 'recommended':
        return <span className="badge badge-teal">Recommended</span>;
      case 'advanced':
        return <span className="badge badge-zinc">Advanced</span>;
      case 'expert':
        return <span className="badge badge-amber">Expert</span>;
      case 'unsafe':
        return <span className="badge badge-rose">Unsafe</span>;
      default:
        return <span className="badge badge-zinc">Unlisted</span>;
    }
  };

  const getEligibilityBadge = (pkg: DevicePackage) => {
    if (ROM_UTILITIES.has(pkg.package_id)) {
      return <span className="badge badge-amber">Built-in utility</span>;
    }
    switch (pkg.removal_eligibility) {
      case 'Eligible':
        return <span className="badge badge-teal">Eligible</span>;
      case 'ReviewOnly':
        return <span className="badge badge-amber">Review-Only</span>;
      case 'Protected':
        return <span className="badge badge-rose">Protected</span>;
      case 'AlreadyRemoved':
        return <span className="badge badge-zinc">Removed</span>;
      default:
        return <span className="badge badge-zinc">Review</span>;
    }
  };

  const getStateBadge = (state: string) => {
    switch (state) {
      case 'Enabled':
        return <span className="badge badge-teal">Enabled</span>;
      case 'Disabled':
        return <span className="badge badge-amber">Disabled</span>;
      case 'Uninstalled':
        return <span className="badge badge-rose">Uninstalled</span>;
      default:
        return <span className="badge badge-zinc">{state}</span>;
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14, height: '100%' }}>
      {/* Header bar */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 10 }}>
        <div>
          <h1 style={{ fontSize: 16, fontWeight: 600, color: 'var(--text-primary)' }}>
            Package Inventory & Management
          </h1>
          <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 2 }}>
            Inspect, filter, disable, uninstall, or restore packages on{' '}
            <strong style={{ color: 'var(--text-primary)' }}>{device ? `${device.brand} ${device.model}` : 'device'}</strong>{' '}
            ({user ? user.display_label : 'User 0'})
          </p>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontSize: 12, color: 'var(--text-muted)', fontVariantNumeric: 'tabular-nums' }}>
            {filteredPackages.length} of {packages.length} packages shown
          </span>
        </div>
      </div>

      {/* Explicit Primary Views / Tabs */}
      <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
        <button
          className={`btn btn-sm ${activeView === 'candidates' ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setActiveView('candidates')}
          style={{ fontSize: 12 }}
        >
          <ShieldCheck size={13} />
          Debloat candidates ({viewCounts.candidates})
        </button>
        <button
          className={`btn btn-sm ${activeView === 'all' ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setActiveView('all')}
          style={{ fontSize: 12 }}
        >
          All packages ({viewCounts.all})
        </button>
        <button
          className={`btn btn-sm ${activeView === 'system' ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setActiveView('system')}
          style={{ fontSize: 12 }}
        >
          System & vendor ({viewCounts.system})
        </button>
        <button
          className={`btn btn-sm ${activeView === 'removed' ? 'btn-primary' : 'btn-secondary'}`}
          onClick={() => setActiveView('removed')}
          style={{ fontSize: 12 }}
        >
          <RotateCcw size={13} />
          Removed for this user ({viewCounts.removed})
        </button>
      </div>

      {/* Notifications */}
      {feedbackMessage && (
        <div
          className={`badge ${feedbackMessage.type === 'success' ? 'badge-teal' : 'badge-rose'}`}
          style={{ width: '100%', padding: '10px 14px', fontSize: 12, justifyContent: 'space-between' }}
        >
          <span>{feedbackMessage.text}</span>
          <button
            onClick={() => setFeedbackMessage(null)}
            style={{ background: 'none', border: 'none', color: 'inherit', cursor: 'pointer' }}
          >
            <X size={14} />
          </button>
        </div>
      )}

      {/* Controls Bar: Search & Sub-filters */}
      <div
        className="card"
        style={{
          padding: '10px 14px',
          display: 'flex',
          flexWrap: 'wrap',
          alignItems: 'center',
          gap: 10,
        }}
      >
        {/* Search */}
        <div style={{ position: 'relative', flex: 1, minWidth: 220 }}>
          <Search size={14} style={{ position: 'absolute', left: 10, top: 9, color: 'var(--text-muted)' }} />
          <input
            type="text"
            className="input-field"
            style={{ width: '100%', paddingLeft: 30, paddingRight: searchInput ? 28 : 10 }}
            placeholder="Search package ID, label, or description"
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
          />
          {searchInput && (
            <button
              onClick={() => setSearchInput('')}
              style={{
                position: 'absolute',
                right: 8,
                top: 8,
                background: 'none',
                border: 'none',
                color: 'var(--text-muted)',
                cursor: 'pointer',
                padding: 2,
                display: 'flex',
                alignItems: 'center',
              }}
              title="Clear search"
            >
              <X size={12} />
            </button>
          )}
        </div>

        {/* State Filter Buttons (only relevant if not in Removed tab) */}
        {activeView !== 'removed' && (
          <div style={{ display: 'flex', background: 'var(--surface-deep)', padding: 2, borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
            {(['All', 'Enabled', 'Disabled'] as const).map((s) => (
              <button
                key={s}
                onClick={() => setStateFilter(s)}
                style={{
                  background: stateFilter === s ? 'var(--surface-raised)' : 'transparent',
                  color: stateFilter === s ? 'var(--text-primary)' : 'var(--text-muted)',
                  border: 'none',
                  padding: '4px 10px',
                  borderRadius: 'var(--radius-xs)',
                  fontSize: 11.5,
                  fontWeight: 500,
                  cursor: 'pointer',
                  transition: 'all 120ms var(--ease-out)',
                }}
              >
                {s}
              </button>
            ))}
          </div>
        )}

        {/* Catalog Rating Filter */}
        <select
          className="select"
          style={{ width: 145, padding: '4px 8px', fontSize: 12 }}
          value={categoryFilter}
          onChange={(e) => setCategoryFilter(e.target.value)}
        >
          <option value="All">All ratings</option>
          <option value="Recommended">Recommended</option>
          <option value="Advanced">Advanced</option>
          <option value="Expert">Expert</option>
          <option value="Unsafe">Unsafe</option>
          <option value="Unlisted">Unlisted</option>
          <option value="System">System apps</option>
          <option value="User">User apps</option>
        </select>

        {/* Origin Dropdown */}
        <select
          className="select"
          style={{ width: 120, padding: '4px 8px', fontSize: 12 }}
          value={originFilter}
          onChange={(e) => setOriginFilter(e.target.value)}
        >
          <option value="All">All origins</option>
          <option value="aosp">AOSP</option>
          <option value="google">Google</option>
          <option value="oem">OEM</option>
          <option value="carrier">Carrier</option>
          <option value="misc">Miscellaneous</option>
        </select>
      </div>

      {/* Active Filters Bar (Advanced Filtering - UX Pattern 12) */}
      {hasActiveFilters && (
        <div className="filter-chip-bar">
          <span style={{ color: 'var(--text-muted)', fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}>
            <Filter size={11} />
            Active filters:
          </span>

          {activeView !== 'candidates' && (
            <span className="filter-chip">
              View: {activeView === 'all' ? 'All packages' : activeView === 'system' ? 'System & vendor' : 'Removed'}
              <button onClick={() => setActiveView('candidates')} title="Reset to debloat candidates">
                <X size={10} />
              </button>
            </span>
          )}

          {debouncedSearch.trim() && (
            <span className="filter-chip">
              Query: &quot;{debouncedSearch}&quot;
              <button onClick={() => setSearchInput('')} title="Clear query">
                <X size={10} />
              </button>
            </span>
          )}

          {activeView !== 'removed' && stateFilter !== 'All' && (
            <span className="filter-chip">
              State: {stateFilter}
              <button onClick={() => setStateFilter('All')} title="Clear state filter">
                <X size={10} />
              </button>
            </span>
          )}

          {categoryFilter !== 'All' && (
            <span className="filter-chip">
              Rating: {categoryFilter}
              <button onClick={() => setCategoryFilter('All')} title="Clear rating filter">
                <X size={10} />
              </button>
            </span>
          )}

          {originFilter !== 'All' && (
            <span className="filter-chip">
              Origin: {originFilter}
              <button onClick={() => setOriginFilter('All')} title="Clear origin filter">
                <X size={10} />
              </button>
            </span>
          )}

          <button
            onClick={resetAllFilters}
            style={{
              marginLeft: 'auto',
              background: 'none',
              border: 'none',
              color: 'var(--accent-teal)',
              fontSize: 11,
              cursor: 'pointer',
              textDecoration: 'underline',
              padding: '2px 4px',
            }}
          >
            Reset all filters
          </button>
        </div>
      )}

      {/* Batch Action Toolbar */}
      {selectedPackages.size > 0 && (
        <div
          className="card"
          style={{
            padding: '8px 14px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            flexWrap: 'wrap',
            gap: 10,
            background: 'var(--surface-deep)',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12 }}>
            <span style={{ color: 'var(--accent-teal)', fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>
              {selectedPackages.size} package{selectedPackages.size !== 1 ? 's' : ''} selected
              {hiddenSelectedCount > 0 && (
                <span style={{ marginLeft: 6, color: 'var(--status-caution)', fontWeight: 400 }}>
                  ({hiddenSelectedCount} hidden by current view/filter)
                </span>
              )}
            </span>
            <button
              onClick={clearSelection}
              style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', fontSize: 11, textDecoration: 'underline' }}
            >
              Deselect all
            </button>
          </div>

          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => initiateBatch('Disable')}
            >
              <PowerOff size={12} />
              Batch disable
            </button>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => initiateBatch('UninstallUser')}
              style={{ color: 'var(--status-negative)' }}
            >
              <Trash2 size={12} />
              Batch uninstall (user)
            </button>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => initiateBatch('Enable')}
            >
              <Power size={12} />
              Batch enable
            </button>
            <button
              className="btn btn-secondary btn-sm"
              onClick={() => initiateBatch('Restore')}
            >
              <RotateCcw size={12} />
              Batch restore
            </button>
          </div>
        </div>
      )}

      {/* Main View: Table + Detail Inspector Drawer */}
      <div style={{ display: 'flex', gap: 14, flex: 1, minHeight: 0 }}>
        {/* Table Container */}
        <div className="data-table-container">
          <div style={{ overflowY: 'auto', flex: 1 }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th style={{ width: 32, textAlign: 'center' }}>
                    <button
                      onClick={selectedPackages.size === filteredPackages.length && filteredPackages.length > 0 ? clearSelection : selectAllVisible}
                      style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer', padding: 0 }}
                      title="Select/Deselect visible"
                    >
                      {selectedPackages.size > 0 ? <CheckSquare size={13} style={{ color: 'var(--accent-teal)' }} /> : <Square size={13} />}
                    </button>
                  </th>
                  <th>Package identifier</th>
                  <th>Type</th>
                  <th>Catalog Rating</th>
                  <th>Removal Eligibility</th>
                  <th>State</th>
                  <th>Description</th>
                  <th style={{ width: 32 }}></th>
                </tr>
              </thead>
              <tbody>
                {isLoading ? (
                  Array.from({ length: 8 }).map((_, i) => (
                    <tr key={`skeleton-${i}`}>
                      <td style={{ textAlign: 'center' }}>
                        <div className="skeleton-box" style={{ width: 13, height: 13, margin: '0 auto' }} />
                      </td>
                      <td>
                        <div className="skeleton-box" style={{ width: `${130 + (i % 4) * 35}px`, height: 12, marginBottom: 4 }} />
                        <div className="skeleton-box" style={{ width: 65, height: 9 }} />
                      </td>
                      <td><div className="skeleton-box" style={{ width: 48, height: 18 }} /></td>
                      <td><div className="skeleton-box" style={{ width: 78, height: 18 }} /></td>
                      <td><div className="skeleton-box" style={{ width: 62, height: 18 }} /></td>
                      <td><div className="skeleton-box" style={{ width: 52, height: 18 }} /></td>
                      <td><div className="skeleton-box" style={{ width: `${160 + (i % 3) * 40}px`, height: 11 }} /></td>
                      <td></td>
                    </tr>
                  ))
                ) : filteredPackages.length === 0 ? (
                  <tr>
                    <td colSpan={8} style={{ textAlign: 'center', padding: 32, color: 'var(--text-muted)' }}>
                      {activeView === 'candidates'
                        ? 'No debloat candidates found matching criteria. The ROM inventory is clean or packages are protected.'
                        : 'No packages match the current filters.'}
                    </td>
                  </tr>
                ) : (
                  filteredPackages.map((pkg) => {
                    const isSelected = selectedPackages.has(pkg.package_id);
                    const isInspecting = activePackageDetail?.package_id === pkg.package_id;

                    return (
                      <tr
                        key={pkg.package_id}
                        onClick={() => setActivePackageDetail(pkg)}
                        className={`${isSelected ? 'table-row-selected' : ''} ${isInspecting ? 'table-row-inspecting' : ''}`}
                        style={{ cursor: 'pointer' }}
                      >
                        <td style={{ textAlign: 'center' }} onClick={(e) => e.stopPropagation()}>
                          <input
                            type="checkbox"
                            checked={isSelected}
                            disabled={pkg.is_protected}
                            onChange={() => toggleSelect(pkg.package_id)}
                            style={{ cursor: pkg.is_protected ? 'not-allowed' : 'pointer' }}
                          />
                        </td>
                        <td>
                          <div className="package-id-cell">
                            <span>{pkg.package_id}</span>
                            {pkg.is_protected && (
                              <span title={pkg.protection_reason || 'Protected by policy'}>
                                <Lock size={11} style={{ color: 'var(--status-negative)' }} />
                              </span>
                            )}
                          </div>
                          {pkg.list_category && (
                            <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>Origin: {pkg.list_category}</div>
                          )}
                        </td>
                        <td>{getPackageTypeBadge(pkg.package_type)}</td>
                        <td>{getRemovalBadge(pkg.removal_recommendation)}</td>
                        <td>{getEligibilityBadge(pkg)}</td>
                        <td>{getStateBadge(pkg.state)}</td>
                        <td
                          style={{
                            fontSize: 11.5,
                            color: 'var(--text-secondary)',
                            maxWidth: 260,
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                          }}
                        >
                          {pkg.description || 'No community description'}
                        </td>
                        <td style={{ textAlign: 'right' }}>
                          <ChevronRight size={13} style={{ color: isInspecting ? 'var(--accent-teal)' : 'var(--text-muted)' }} />
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>

        {/* Side Inspector Drawer */}
        {activePackageDetail && (
          <div
            className="card package-detail-drawer"
            style={{
              width: 330,
              padding: 16,
              display: 'flex',
              flexDirection: 'column',
              gap: 12,
              overflowY: 'auto',
              flexShrink: 0,
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
              <div>
                <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>
                  Package Inspector
                </div>
                <div style={{ fontWeight: 600, fontSize: 12.5, color: 'var(--text-primary)', wordBreak: 'break-all', fontFamily: 'var(--font-mono)', marginTop: 2 }}>
                  {activePackageDetail.package_id}
                </div>
                {activePackageDetail.list_category && (
                  <div style={{ fontSize: 10.5, color: 'var(--text-muted)', marginTop: 2 }}>
                    Origin: {activePackageDetail.list_category}
                  </div>
                )}
              </div>
              <button
                onClick={() => setActivePackageDetail(null)}
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
              >
                <X size={14} />
              </button>
            </div>

            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
              {getStateBadge(activePackageDetail.state)}
              {getPackageTypeBadge(activePackageDetail.package_type)}
              {getRemovalBadge(activePackageDetail.removal_recommendation)}
              {getEligibilityBadge(activePackageDetail)}
            </div>

            {/* Description & Details */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8, fontSize: 12 }}>
              <div>
                <span style={{ color: 'var(--text-muted)', display: 'block', fontSize: 11 }}>Description</span>
                <span style={{ color: 'var(--text-secondary)', lineHeight: 1.4 }}>
                  {activePackageDetail.description || 'No community description recorded for this package.'}
                </span>
              </div>

              {activePackageDetail.protection_reason && (
                <div>
                  <span style={{ color: 'var(--status-negative)', display: 'block', fontSize: 11 }}>Protection Policy</span>
                  <span style={{ color: 'var(--status-negative)' }}>{activePackageDetail.protection_reason}</span>
                </div>
              )}

              {activePackageDetail.dependencies.length > 0 && (
                <div>
                  <span style={{ color: 'var(--text-muted)', display: 'block', fontSize: 11 }}>Dependencies</span>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginTop: 2 }}>
                    {activePackageDetail.dependencies.map((dep) => (
                      <span key={dep} className="badge badge-zinc" style={{ fontSize: 10 }}>
                        {dep}
                      </span>
                    ))}
                  </div>
                </div>
              )}

              {activePackageDetail.needed_by.length > 0 && (
                <div>
                  <span style={{ color: 'var(--text-muted)', display: 'block', fontSize: 11 }}>Needed by</span>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginTop: 2 }}>
                    {activePackageDetail.needed_by.map((dep) => (
                      <span key={dep} className="badge badge-zinc" style={{ fontSize: 10 }}>
                        {dep}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Direct Mutations */}
            <div style={{ marginTop: 'auto', borderTop: '1px solid var(--border-hairline)', paddingTop: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
              <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 2 }}>
                Direct Mutations
              </div>
              {activePackageDetail.state === 'Enabled' && (
                <>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ justifyContent: 'center' }}
                    onClick={() => handleSingleAction(activePackageDetail.package_id, 'Disable')}
                    disabled={activePackageDetail.is_protected}
                  >
                    <PowerOff size={12} />
                    Disable package
                  </button>
                  <button
                    className="btn btn-secondary btn-sm"
                    style={{ justifyContent: 'center', color: 'var(--status-negative)' }}
                    onClick={() => handleSingleAction(activePackageDetail.package_id, 'UninstallUser')}
                    disabled={activePackageDetail.is_protected}
                  >
                    <Trash2 size={12} />
                    Uninstall for user
                  </button>
                </>
              )}

              {activePackageDetail.state === 'Disabled' && (
                <button
                  className="btn btn-secondary btn-sm"
                  style={{ justifyContent: 'center' }}
                  onClick={() => handleSingleAction(activePackageDetail.package_id, 'Enable')}
                >
                  <Power size={12} />
                  Enable package
                </button>
              )}

              {activePackageDetail.state === 'Uninstalled' && (
                <button
                  className="btn btn-primary btn-sm"
                  style={{ justifyContent: 'center' }}
                  onClick={() => handleSingleAction(activePackageDetail.package_id, 'Restore')}
                >
                  <RotateCcw size={12} />
                  Restore package
                </button>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Modal: Preflight Batch Revalidation & Confirmation */}
      {pendingBatchAction && (
        <div className="modal-overlay" onClick={() => !isExecutingBatch && setPendingBatchAction(null)}>
          <div className="modal-box" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 650, width: '95%' }}>
            <div className="modal-header">
              <h3 style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)', display: 'flex', alignItems: 'center', gap: 8 }}>
                Preflight Revalidation: Batch {pendingBatchAction}
              </h3>
              <button
                style={{ background: 'none', border: 'none', color: 'var(--text-muted)', cursor: 'pointer' }}
                onClick={() => !isExecutingBatch && setPendingBatchAction(null)}
              >
                <X size={15} />
              </button>
            </div>

            <div className="modal-body" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div style={{ fontSize: 12, color: 'var(--text-secondary)' }}>
                Target: <strong style={{ color: 'var(--text-primary)' }}>{device?.brand} {device?.model} ({device?.serial})</strong> ·{' '}
                Profile: <strong style={{ color: 'var(--text-primary)' }}>User {user?.id ?? 0} ({user?.display_label || 'Owner'})</strong>
              </div>

              {isPreflightValidating && (
                <div style={{ padding: '24px', textAlign: 'center', color: 'var(--text-muted)', fontSize: 12 }}>
                  Revalidating selection against fresh device state and protection policy...
                </div>
              )}

              {preflightError && (
                <div className="badge badge-rose" style={{ padding: '10px 14px', fontSize: 12, width: '100%' }}>
                  <AlertTriangle size={14} style={{ marginRight: 6 }} />
                  {preflightError}
                </div>
              )}

              {preflightResult && (
                <>
                  {/* Summary badges */}
                  <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                    <span className="badge badge-zinc">
                      Requested: <strong style={{ marginLeft: 4 }}>{preflightResult.total_requested}</strong>
                    </span>
                    <span className="badge badge-teal">
                      Eligible to execute: <strong style={{ marginLeft: 4 }}>{preflightResult.eligible_count}</strong>
                    </span>
                    {preflightResult.excluded_count > 0 && (
                      <span className="badge badge-rose">
                        Excluded by policy: <strong style={{ marginLeft: 4 }}>{preflightResult.excluded_count}</strong>
                      </span>
                    )}
                  </div>

                  {/* Preflight Revalidation Table */}
                  <div style={{ maxHeight: 240, overflowY: 'auto', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)' }}>
                    <table className="data-table" style={{ fontSize: 11 }}>
                      <thead>
                        <tr>
                          <th>Package ID</th>
                          <th>Type</th>
                          <th>Current State</th>
                          <th>Action</th>
                          <th>Status</th>
                        </tr>
                      </thead>
                      <tbody>
                        {preflightResult.items.map((item) => (
                          <tr key={item.package_id} style={{ background: item.eligible ? undefined : 'var(--surface-deep)' }}>
                            <td style={{ fontFamily: 'var(--font-mono)' }}>{item.package_id}</td>
                            <td>{item.package_type}</td>
                            <td>{item.current_state}</td>
                            <td style={{ fontWeight: 600 }}>{item.action}</td>
                            <td>
                              {item.eligible ? (
                                <span className="badge badge-teal" style={{ fontSize: 10 }}>
                                  <CheckCircle2 size={10} style={{ marginRight: 3 }} /> Eligible
                                </span>
                              ) : (
                                <span className="badge badge-rose" style={{ fontSize: 10 }} title={item.exclusion_reason || 'Excluded'}>
                                  <AlertTriangle size={10} style={{ marginRight: 3 }} />
                                  {item.exclusion_reason || 'Excluded'}
                                </span>
                              )}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </>
              )}

              {isExecutingBatch && batchProgress && (
                <div style={{ marginTop: 8 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
                    <span>Executing batch operations...</span>
                    <span>{batchProgress.current} / {batchProgress.total}</span>
                  </div>
                  <div style={{ height: 4, background: 'var(--surface-raised)', borderRadius: 'var(--radius-xs)', overflow: 'hidden' }}>
                    <div
                      style={{
                        height: '100%',
                        width: `${Math.round((batchProgress.current / batchProgress.total) * 100)}%`,
                        background: 'var(--accent-teal)',
                        transition: 'width 0.15s var(--ease-out)',
                      }}
                    />
                  </div>
                </div>
              )}

              {batchResults && (
                <div style={{ fontSize: 12, color: 'var(--accent-teal)', marginTop: 4 }}>
                  Batch finished: {batchResults.succeeded} succeeded, {batchResults.failed} failed
                </div>
              )}
            </div>

            <div className="modal-footer" style={{ justifyContent: 'flex-end', gap: 8 }}>
              {isExecutingBatch && (
                <button
                  className="btn btn-danger btn-sm"
                  onClick={() => { batchCancelRef.current = true; }}
                >
                  Stop after current
                </button>
              )}
              <button
                className="btn btn-secondary btn-sm"
                onClick={() => setPendingBatchAction(null)}
                disabled={isExecutingBatch}
              >
                {batchResults ? 'Close' : 'Cancel'}
              </button>
              {!batchResults && preflightResult && preflightResult.eligible_count > 0 && (
                <button
                  className="btn btn-primary btn-sm"
                  onClick={executeBatch}
                  disabled={isExecutingBatch}
                >
                  {isExecutingBatch ? 'Executing...' : `Confirm & Execute (${preflightResult.eligible_count} verified)`}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
