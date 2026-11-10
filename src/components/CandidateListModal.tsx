import React, { useState, useMemo } from 'react';
import { Search, X, ShieldAlert, ShieldCheck, AlertTriangle, Package, Check, Info } from 'lucide-react';
import { DeviceInfo, AndroidUser, DevicePackage, AndroidPackageType } from '../types';

interface Props {
  isOpen: boolean;
  onClose: () => void;
  device: DeviceInfo | null;
  user: AndroidUser | null;
  packages: DevicePackage[];
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

export const CandidateListModal: React.FC<Props> = ({
  isOpen,
  onClose,
  device,
  user,
  packages,
}) => {
  const [search, setSearch] = useState('');
  const [filterTab, setFilterTab] = useState<'all' | 'candidates' | 'system' | 'review' | 'protected' | 'removed'>('all');
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const stats = useMemo(() => {
    let candidates = 0;
    let system = 0;
    let reviewCount = 0;
    let protectedCount = 0;
    let removed = 0;

    for (const p of packages) {
      if (p.state === 'Uninstalled') {
        removed++;
        continue;
      }
      if (p.removal_eligibility === 'Protected') {
        protectedCount++;
      } else if (p.removal_eligibility === 'Eligible') {
        candidates++;
      } else {
        reviewCount++;
      }

      const isSys =
        p.package_type === 'System' ||
        p.package_type === 'UpdatedSystem' ||
        (p.list_category || '').toLowerCase() === 'oem' ||
        p.package_id.startsWith('com.mediatek.') ||
        p.package_id.startsWith('vendor.mediatek.') ||
        p.package_id.startsWith('com.miui.') ||
        p.package_id.startsWith('com.xiaomi.') ||
        p.package_id.startsWith('com.sec.') ||
        p.package_id.startsWith('com.samsung.');
      if (isSys) {
        system++;
      }
    }

    return {
      total: packages.length,
      candidates,
      system,
      review: reviewCount,
      protected: protectedCount,
      removed,
    };
  }, [packages]);

  const filteredCandidates = useMemo(() => {
    return packages.filter((p) => {
      // Search filter
      if (search.trim()) {
        const q = search.toLowerCase();
        const matchId = p.package_id.toLowerCase().includes(q);
        const matchDesc = (p.description || '').toLowerCase().includes(q);
        const matchCat = (p.list_category || '').toLowerCase().includes(q);
        if (!matchId && !matchDesc && !matchCat) return false;
      }

      // Tab filter
      if (filterTab === 'removed') {
        return p.state === 'Uninstalled';
      }
      if (p.state === 'Uninstalled') {
        return filterTab === 'all';
      }

      if (filterTab === 'candidates') {
        return p.removal_eligibility === 'Eligible';
      }
      if (filterTab === 'system') {
        return (
          p.package_type === 'System' ||
          p.package_type === 'UpdatedSystem' ||
          (p.list_category || '').toLowerCase() === 'oem' ||
          p.package_id.startsWith('com.mediatek.') ||
          p.package_id.startsWith('vendor.mediatek.') ||
          p.package_id.startsWith('com.miui.') ||
          p.package_id.startsWith('com.xiaomi.') ||
          p.package_id.startsWith('com.sec.') ||
          p.package_id.startsWith('com.samsung.')
        );
      }
      if (filterTab === 'review') {
        return p.removal_eligibility === 'ReviewOnly';
      }
      if (filterTab === 'protected') {
        return p.removal_eligibility === 'Protected';
      }

      return true;
    });
  }, [packages, search, filterTab]);

  const handleCopy = (pkgId: string) => {
    navigator.clipboard.writeText(pkgId);
    setCopiedId(pkgId);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const getPackageTypeBadge = (type: AndroidPackageType) => {
    switch (type) {
      case 'System':
        return <span className="badge badge-zinc">System</span>;
      case 'UpdatedSystem':
        return <span className="badge badge-zinc" style={{ color: 'var(--accent-teal)' }}>Updated System</span>;
      case 'User':
        return <span className="badge badge-zinc">User</span>;
      default:
        return <span className="badge badge-zinc">{type}</span>;
    }
  };

  const getEligibilityBadge = (pkg: DevicePackage) => {
    if (ROM_UTILITIES.has(pkg.package_id)) {
      return <span className="badge badge-amber">Built-in utility</span>;
    }
    switch (pkg.removal_eligibility) {
      case 'Eligible':
        return (
          <span className="badge badge-teal">
            <ShieldCheck size={11} /> Eligible
          </span>
        );
      case 'ReviewOnly':
        return (
          <span className="badge badge-amber">
            <AlertTriangle size={11} /> Review-Only
          </span>
        );
      case 'Protected':
        return (
          <span className="badge badge-rose">
            <ShieldAlert size={11} /> Protected
          </span>
        );
      case 'AlreadyRemoved':
        return <span className="badge badge-zinc">Removed</span>;
      default:
        return <span className="badge badge-zinc">Review</span>;
    }
  };

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop" style={{ zIndex: 1000 }} onClick={onClose}>
      <div
        className="modal-card"
        style={{ width: '92%', maxWidth: 840, maxHeight: '85vh', display: 'flex', flexDirection: 'column' }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="modal-header" style={{ padding: '14px 18px', borderBottom: '1px solid var(--border-hairline)' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <Package size={18} style={{ color: 'var(--accent-teal)' }} />
            <div>
              <h2 style={{ fontSize: 15, fontWeight: 600, color: 'var(--text-primary)', margin: 0 }}>
                Candidate Package Inventory
              </h2>
              <div style={{ fontSize: 11.5, color: 'var(--text-secondary)', marginTop: 2 }}>
                Full catalog scan for {device?.brand ? `${device.brand} ${device.model}` : 'Connected Device'} · User {user?.id ?? 0} ({user?.name || 'Owner'})
              </div>
            </div>
          </div>
          <button className="btn btn-secondary btn-sm" onClick={onClose} style={{ height: 26, width: 26, padding: 0 }}>
            <X size={14} />
          </button>
        </div>

        {/* Note */}
        <div style={{ padding: '10px 18px', background: 'var(--surface-deep)', borderBottom: '1px solid var(--border-hairline)', fontSize: 11.5, color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: 8 }}>
          <Info size={14} style={{ color: 'var(--accent-teal)', flexShrink: 0 }} />
          <span>
            Separating Android package type, community catalog rating, and device protection eligibility. Catalog recommendations never override device safety policies.
          </span>
        </div>

        {/* Toolbar & Filter Tabs */}
        <div style={{ padding: '12px 18px', display: 'flex', flexDirection: 'column', gap: 10, borderBottom: '1px solid var(--border-hairline)' }}>
          {/* Search */}
          <div style={{ position: 'relative' }}>
            <Search size={14} style={{ position: 'absolute', left: 10, top: '50%', transform: 'translateY(-50%)', color: 'var(--text-muted)' }} />
            <input
              type="text"
              className="input"
              style={{ width: '100%', paddingLeft: 30, height: 32, fontSize: 12 }}
              placeholder="Search candidate packages by identifier, description, or origin..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
            />
          </div>

          {/* Filter Pills */}
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
            <button
              className={`btn btn-sm ${filterTab === 'all' ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 11, padding: '3px 8px' }}
              onClick={() => setFilterTab('all')}
            >
              All ({stats.total})
            </button>
            <button
              className={`btn btn-sm ${filterTab === 'candidates' ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 11, padding: '3px 8px' }}
              onClick={() => setFilterTab('candidates')}
            >
              Debloat Candidates ({stats.candidates})
            </button>
            <button
              className={`btn btn-sm ${filterTab === 'system' ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 11, padding: '3px 8px' }}
              onClick={() => setFilterTab('system')}
            >
              System & Vendor ({stats.system})
            </button>
            <button
              className={`btn btn-sm ${filterTab === 'review' ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 11, padding: '3px 8px' }}
              onClick={() => setFilterTab('review')}
            >
              Review-Only ({stats.review})
            </button>
            <button
              className={`btn btn-sm ${filterTab === 'protected' ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 11, padding: '3px 8px' }}
              onClick={() => setFilterTab('protected')}
            >
              Protected ({stats.protected})
            </button>
            <button
              className={`btn btn-sm ${filterTab === 'removed' ? 'btn-primary' : 'btn-secondary'}`}
              style={{ fontSize: 11, padding: '3px 8px' }}
              onClick={() => setFilterTab('removed')}
            >
              Already Removed ({stats.removed})
            </button>
          </div>
        </div>

        {/* Scrollable Package List */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '12px 18px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {filteredCandidates.length === 0 ? (
            <div style={{ padding: 32, textAlign: 'center', color: 'var(--text-muted)', fontSize: 12 }}>
              No candidate packages match the current filter or search query.
            </div>
          ) : (
            filteredCandidates.map((pkg) => {
              const isRemoved = pkg.state === 'Uninstalled';

              return (
                <div
                  key={pkg.package_id}
                  className="card"
                  style={{
                    padding: '10px 14px',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 6,
                    background: isRemoved ? 'rgba(255,255,255,0.02)' : 'var(--surface-deep)',
                    opacity: isRemoved ? 0.75 : 1,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 8 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                      <span className="mono-code" style={{ fontSize: 12, fontWeight: 600, wordBreak: 'break-all' }}>
                        {pkg.package_id}
                      </span>
                      <button
                        className="btn btn-secondary btn-sm"
                        style={{ height: 20, padding: '0 6px', fontSize: 10 }}
                        onClick={() => handleCopy(pkg.package_id)}
                        title="Copy package identifier"
                      >
                        {copiedId === pkg.package_id ? <Check size={11} style={{ color: 'var(--accent-teal)' }} /> : 'Copy'}
                      </button>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
                      {getPackageTypeBadge(pkg.package_type)}
                      {getEligibilityBadge(pkg)}
                      <span className="badge badge-zinc" style={{ fontSize: 10 }}>
                        Catalog: {pkg.removal_recommendation}
                      </span>
                      {pkg.list_category && (
                        <span className="badge badge-zinc" style={{ fontSize: 10 }}>
                          {pkg.list_category}
                        </span>
                      )}
                    </div>
                  </div>

                  <p style={{ fontSize: 11.5, color: 'var(--text-secondary)', margin: 0, wordBreak: 'break-word', overflowWrap: 'anywhere' }}>
                    {pkg.description || 'No description available in community dataset.'}
                  </p>

                  {pkg.protection_reason && (
                    <div style={{ fontSize: 10.5, color: 'var(--status-negative)', display: 'flex', alignItems: 'center', gap: 4 }}>
                      <ShieldAlert size={11} />
                      <span>{pkg.protection_reason}</span>
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="modal-footer" style={{ padding: '10px 18px', borderTop: '1px solid var(--border-hairline)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>
            Showing {filteredCandidates.length} of {packages.length} packages
          </span>
          <button className="btn btn-secondary btn-sm" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
