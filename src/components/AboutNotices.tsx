import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Database,
  GitCommit,
  FileText,
  Palette,
  Server,
  Globe,
  ExternalLink,
  Shield,
  HelpCircle,
  Cpu,
  Info
} from 'lucide-react';
import { DatasetStats } from '../types';

export const AboutNotices: React.FC = () => {
  const [stats, setStats] = useState<DatasetStats | null>(null);

  useEffect(() => {
    invoke<DatasetStats>('get_dataset_stats')
      .then((s) => setStats(s))
      .catch((e) => console.error(e));
  }, []);

  return (
    <div className="about-fluid-container" style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      {/* 1. About the Tool */}
      <div className="card" style={{ padding: 18 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 12 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <div className="logo-badge" style={{ width: 46, height: 46, fontSize: 18 }}>
              TZ
            </div>
            <div>
              <h1 style={{ fontSize: 17, fontWeight: 600, color: 'var(--text-primary)', margin: 0 }}>
                TZ AI DEBLOATER TOOL
              </h1>
              <p style={{ fontSize: 12, color: 'var(--accent-teal)', fontWeight: 500, marginTop: 2, margin: 0 }}>
                Intelligent Android Maintenance & Package Management Console · Version 1.1.0
              </p>
            </div>
          </div>

          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <span className="badge badge-teal">Production Release</span>
            <span className="badge badge-zinc">Windows x64</span>
          </div>
        </div>

        <p style={{ fontSize: 12.5, color: 'var(--text-secondary)', marginTop: 14, lineHeight: 1.55 }}>
          <strong>TZ AI DEBLOATER TOOL</strong> is a desktop workstation engineered for Android power users, technicians,
          and developers. It establishes direct communication with Android devices via USB debugging or Android 11+
          wireless ADB. The tool enables thorough package inventory inspection, state management (disable, enable, per-user uninstall,
          system restore), and AI-assisted debloating governed by a strict, action-aware safety policy.
        </p>

        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 10, marginTop: 14 }}>
          <div style={{ background: 'var(--surface-deep)', padding: '10px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block' }}>Release Revision</span>
            <strong style={{ fontSize: 12.5, color: 'var(--text-primary)', fontFamily: 'var(--font-mono)' }}>v1.1.0 (Nov 10, 2026)</strong>
          </div>
          <div style={{ background: 'var(--surface-deep)', padding: '10px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block' }}>Target Architecture</span>
            <strong style={{ fontSize: 12.5, color: 'var(--text-primary)' }}>Windows 10/11 x64 (MSI / NSIS)</strong>
          </div>
          <div style={{ background: 'var(--surface-deep)', padding: '10px 12px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-hairline)' }}>
            <span style={{ fontSize: 11, color: 'var(--text-muted)', display: 'block' }}>Runtime Engine</span>
            <strong style={{ fontSize: 12.5, color: 'var(--text-primary)' }}>Tauri v2 + WebView2 + Rust</strong>
          </div>
        </div>
      </div>

      {/* 2. About Us / TZ Labs */}
      <div className="card" style={{ padding: 18 }}>
        <div className="card-header" style={{ marginBottom: 10 }}>
          <div className="card-title">
            <Globe size={16} style={{ color: 'var(--accent-teal)' }} />
            About TZ Labs
          </div>
          <a
            href="https://tzlabs.in"
            target="_blank"
            rel="noreferrer"
            className="btn btn-secondary btn-sm"
            style={{ fontSize: 11.5, gap: 6 }}
          >
            Visit tzlabs.in
            <ExternalLink size={12} />
          </a>
        </div>

        <p style={{ fontSize: 12.5, color: 'var(--text-secondary)', lineHeight: 1.55 }}>
          <strong>TZ Labs</strong> builds open-source developer utilities and system maintenance tools focused on performance,
          privacy, and operator safety. You can learn more about our projects, download the latest tooling, and review technical
          documentation at{' '}
          <a
            href="https://tzlabs.in"
            target="_blank"
            rel="noreferrer"
            style={{ color: 'var(--accent-teal)', textDecoration: 'underline' }}
          >
            https://tzlabs.in
          </a>.
        </p>

        <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginTop: 12 }}>
          <a
            href="https://tzlabs.in/debloater/docs"
            target="_blank"
            rel="noreferrer"
            className="badge badge-zinc"
            style={{ padding: '6px 12px', fontSize: 11.5, gap: 5, textDecoration: 'none', cursor: 'pointer' }}
          >
            <HelpCircle size={12} style={{ color: 'var(--accent-teal)' }} />
            Documentation & User Guide
          </a>
          <a
            href="https://tzlabs.in/debloater/privacy"
            target="_blank"
            rel="noreferrer"
            className="badge badge-zinc"
            style={{ padding: '6px 12px', fontSize: 11.5, gap: 5, textDecoration: 'none', cursor: 'pointer' }}
          >
            <Shield size={12} style={{ color: 'var(--accent-teal)' }} />
            Privacy & AI Data Handling
          </a>
          <a
            href="https://github.com/tzlabs/tz-ai-debloater/issues"
            target="_blank"
            rel="noreferrer"
            className="badge badge-zinc"
            style={{ padding: '6px 12px', fontSize: 11.5, gap: 5, textDecoration: 'none', cursor: 'pointer' }}
          >
            <ExternalLink size={12} style={{ color: 'var(--accent-teal)' }} />
            Issue Tracker & Technical Support
          </a>
        </div>
      </div>

      {/* 3. Dataset Statistics Card */}
      {stats && (
        <div className="card" style={{ padding: 18 }}>
          <div className="card-header" style={{ marginBottom: 10 }}>
            <div className="card-title">
              <Database size={15} style={{ color: 'var(--accent-teal)' }} />
              Embedded Community Package Catalog
            </div>
            <span className="badge badge-teal" style={{ fontVariantNumeric: 'tabular-nums' }}>
              {stats.total_packages} Cataloged Packages
            </span>
          </div>

          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(110px, 1fr))', gap: 8 }}>
            <div style={{ background: 'var(--surface-deep)', padding: '8px 10px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
              <span style={{ fontSize: 10.5, color: 'var(--text-muted)', display: 'block' }}>Recommended</span>
              <strong style={{ fontSize: 14, color: 'var(--status-positive)', fontVariantNumeric: 'tabular-nums' }}>{stats.recommended_count}</strong>
            </div>
            <div style={{ background: 'var(--surface-deep)', padding: '8px 10px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
              <span style={{ fontSize: 10.5, color: 'var(--text-muted)', display: 'block' }}>Advanced</span>
              <strong style={{ fontSize: 14, color: 'var(--text-primary)', fontVariantNumeric: 'tabular-nums' }}>{stats.advanced_count}</strong>
            </div>
            <div style={{ background: 'var(--surface-deep)', padding: '8px 10px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
              <span style={{ fontSize: 10.5, color: 'var(--text-muted)', display: 'block' }}>Expert</span>
              <strong style={{ fontSize: 14, color: 'var(--status-warning)', fontVariantNumeric: 'tabular-nums' }}>{stats.expert_count}</strong>
            </div>
            <div style={{ background: 'var(--surface-deep)', padding: '8px 10px', borderRadius: 'var(--radius-xs)', border: '1px solid var(--border-hairline)' }}>
              <span style={{ fontSize: 10.5, color: 'var(--text-muted)', display: 'block' }}>Unsafe</span>
              <strong style={{ fontSize: 14, color: 'var(--status-negative)', fontVariantNumeric: 'tabular-nums' }}>{stats.unsafe_count}</strong>
            </div>
          </div>
        </div>
      )}

      {/* 4. Upstream & Third-Party Notices Card */}
      <div className="card" style={{ padding: 18 }}>
        <div className="card-header" style={{ marginBottom: 10 }}>
          <div className="card-title">
            <FileText size={15} style={{ color: 'var(--accent-teal)' }} />
            Third-Party Software Notices & Attributions
          </div>
          <span className="badge badge-zinc">GPL-3.0 Project</span>
        </div>

        <p style={{ fontSize: 12, color: 'var(--text-secondary)', marginBottom: 12, lineHeight: 1.5 }}>
          In compliance with open-source licensing obligations, this application acknowledges and attributes the following upstream repositories, datasets, design systems, and frameworks:
        </p>

        {/* 1. Unslop Design System Credit */}
        <div style={{ background: 'var(--surface-deep)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)', padding: 12, marginBottom: 10 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <Palette size={13} style={{ color: 'var(--accent-teal)' }} />
              <strong style={{ color: 'var(--text-primary)', fontSize: 12.5 }}>
                Unslop Design System
              </strong>
            </div>
            <span className="badge badge-zinc">CC BY 4.0</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            Created by <strong>Damir Sokolovsky</strong> (<code style={{ color: 'var(--accent-teal)' }}>@heydamir</code> · <a href="https://heydamir.design/unslop" target="_blank" rel="noreferrer" style={{ color: 'var(--accent-teal)', textDecoration: 'underline' }}>heydamir.design/unslop</a>).
          </div>
          <p style={{ fontSize: 11.5, color: 'var(--text-secondary)', margin: 0, lineHeight: 1.4 }}>
            Provided the restrained, professional design guidelines, typography scale, graphite neutral ramp, hairline borders, and calm information hierarchy. Licensed under Creative Commons Attribution 4.0 International.
          </p>
        </div>

        {/* 2. TZ AI Gateway */}
        <div style={{ background: 'var(--surface-deep)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)', padding: 12, marginBottom: 10 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <Server size={13} style={{ color: 'var(--accent-teal)' }} />
              <strong style={{ color: 'var(--text-primary)', fontSize: 12.5 }}>
                TZ AI Gateway Integration
              </strong>
            </div>
            <span className="badge badge-zinc">MIT</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>
            Author: <strong>Tashfeen Ahmed</strong>
          </div>
          <p style={{ fontSize: 11.5, color: 'var(--text-secondary)', margin: 0, lineHeight: 1.4 }}>
            Documented self-hosted OpenAI-compatible AI gateway allowing local model routing, bearer authentication, and streaming completions.
          </p>
        </div>

        {/* 3. UAD-ng */}
        <div style={{ background: 'var(--surface-deep)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)', padding: 12, marginBottom: 10 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <strong style={{ color: 'var(--text-primary)', fontSize: 12.5 }}>
              Universal Android Debloater Next Generation (UAD-ng)
            </strong>
            <span className="badge badge-zinc">GPL-3.0</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
            <GitCommit size={12} />
            Commit inspected: <code style={{ color: 'var(--accent-teal)' }}>9fcad87bf78c52c4270504bc8880139d95a27a29</code>
          </div>
          <p style={{ fontSize: 11.5, color: 'var(--text-secondary)', margin: 0, lineHeight: 1.4 }}>
            Provided primary desktop ADB execution abstractions, user profile detection logic, and the comprehensive 5,399 package recommendation dataset (<code>uad_lists.json</code>).
          </p>
        </div>

        {/* 4. Canta */}
        <div style={{ background: 'var(--surface-deep)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)', padding: 12, marginBottom: 10 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <strong style={{ color: 'var(--text-primary)', fontSize: 12.5 }}>Canta (by samolego)</strong>
            <span className="badge badge-zinc">LGPL-3.0</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
            <GitCommit size={12} />
            Commit inspected: <code style={{ color: 'var(--accent-teal)' }}>e3ed0c9bf90ee6f8bfb7a92ba1e140327cda480f</code>
          </div>
          <p style={{ fontSize: 11.5, color: 'var(--text-secondary)', margin: 0, lineHeight: 1.4 }}>
            Inspected for on-device package classification concepts and debloating warnings. Shizuku Android-native code was omitted in favor of the desktop ADB architecture.
          </p>
        </div>

        {/* 5. Original UAD */}
        <div style={{ background: 'var(--surface-deep)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-sm)', padding: 12 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
            <strong style={{ color: 'var(--text-primary)', fontSize: 12.5 }}>Universal Android Debloater (Original by 0x192)</strong>
            <span className="badge badge-zinc">GPL-3.0</span>
          </div>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
            <GitCommit size={12} />
            Commit inspected: <code style={{ color: 'var(--accent-teal)' }}>11f27c671cba278d71296cdef4c5a5dba06add5e</code>
          </div>
          <p style={{ fontSize: 11.5, color: 'var(--text-secondary)', margin: 0, lineHeight: 1.4 }}>
            Historical lineage for debloat category mappings and non-root package command structures.
          </p>
        </div>
      </div>
    </div>
  );
};
