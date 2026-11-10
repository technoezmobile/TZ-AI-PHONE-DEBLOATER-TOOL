import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

const LOCAL_DIR = path.resolve('screenshots');
const ARTIFACT_DIR = 'C:\\Users\\singh\\.gemini\\antigravity\\brain\\c2d3dc07-f5e1-4320-a123-5cfc88d22ec2\\screenshots';

if (!fs.existsSync(LOCAL_DIR)) fs.mkdirSync(LOCAL_DIR, { recursive: true });
if (!fs.existsSync(ARTIFACT_DIR)) fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

const distDir = path.resolve('dist');
if (!fs.existsSync(distDir)) {
  console.error('dist directory does not exist! Run npm run build first.');
  process.exit(1);
}

const mockJournal = [
  {
    entry_id: 'entry-poco-01',
    timestamp: 1731170000000,
    device_serial: 'XGGE79BEEU6HEAIV',
    user_id: 0,
    package_id: 'com.miui.analytics',
    action: 'UninstallUser',
    previous_state: 'Enabled',
    resulting_state: 'Uninstalled',
    success: true,
    verified: true,
    message: 'Package uninstalled for User 0.',
    recovery_steps: ['Restore'],
  },
];

const pocoPackages = [
  // 1. Debloat Candidates (Eligible, Recommended)
  {
    package_id: 'com.miui.msa.global',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'Eligible',
    description: 'MIUI System Ads service. Pushes promotional push notifications and banner ads.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.facebook.appmanager',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'carrier',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'Eligible',
    description: 'Pre-installed Facebook background app updater and telemetry service.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.facebook.services',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'carrier',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'Eligible',
    description: 'Facebook sync and analytics daemon.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.xiaomi.discover',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'Eligible',
    description: 'Mi App Mall promotional store and recommendation feed.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.xiaomi.glgm',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'Eligible',
    description: 'Xiaomi Game Center promo push service and game recommendation overlay.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.mipay.wallet.in',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'Eligible',
    description: 'Mi Pay regional wallet push notification and banner service.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },

  // 2. MediaTek Vendor Services (Confirmed Critical = Protected; Uncertain = Review-Only)
  {
    package_id: 'com.mediatek.telephony',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Unsafe',
    removal_eligibility: 'Protected',
    description: 'MediaTek Telephony & Radio Interface Layer (RIL) service. Required for cellular calls and SMS.',
    dependencies: [],
    needed_by: [],
    is_protected: true,
    protection_reason: 'Confirmed critical telephony/modem/radio vendor service',
  },
  {
    package_id: 'com.mediatek.ims',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Unsafe',
    removal_eligibility: 'Protected',
    description: 'MediaTek IP Multimedia Subsystem (IMS) for VoLTE and VoWiFi calling.',
    dependencies: [],
    needed_by: [],
    is_protected: true,
    protection_reason: 'Confirmed critical telephony/modem/radio vendor service',
  },
  {
    package_id: 'com.mediatek.miravision',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Advanced',
    removal_eligibility: 'ReviewOnly',
    description: 'MediaTek MiraVision picture quality enhancement service. Kept for explicit review.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.mediatek.duraspeed',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Advanced',
    removal_eligibility: 'ReviewOnly',
    description: 'MediaTek DuraSpeed background process management service.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },

  // 3. ROM Built-in Utilities (Review-Only, kept OUT of default candidates)
  {
    package_id: 'com.miui.compass',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Advanced',
    removal_eligibility: 'ReviewOnly',
    description: 'MIUI Compass sensor utility app. Kept for explicit user review.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.miui.notes',
    state: 'Enabled',
    is_system: true,
    package_type: 'UpdatedSystem',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Advanced',
    removal_eligibility: 'ReviewOnly',
    description: 'MIUI Notes built-in utility app. Updated system application.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.miui.calculator',
    state: 'Enabled',
    is_system: true,
    package_type: 'UpdatedSystem',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Advanced',
    removal_eligibility: 'ReviewOnly',
    description: 'MIUI Calculator utility. Updated system application.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },

  // 4. Promotional Third-Party Apps (User apps, Unlisted)
  {
    package_id: 'com.block.juggle',
    state: 'Enabled',
    is_system: false,
    package_type: 'User',
    user_id: 0,
    list_category: 'Unlisted',
    removal_recommendation: 'Unlisted',
    removal_eligibility: 'ReviewOnly',
    description: 'Block Puzzle Juggle pre-installed promotional game. Third-party unlisted application.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.crazy.drop',
    state: 'Enabled',
    is_system: false,
    package_type: 'User',
    user_id: 0,
    list_category: 'Unlisted',
    removal_recommendation: 'Unlisted',
    removal_eligibility: 'ReviewOnly',
    description: 'Crazy Drop pre-installed promotional game. Third-party unlisted application.',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },

  // 5. Essential System / Active Launcher
  {
    package_id: 'com.miui.home',
    state: 'Enabled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Unsafe',
    removal_eligibility: 'Protected',
    description: 'Default POCO / HyperOS launcher. Critical for home screen interaction.',
    dependencies: [],
    needed_by: [],
    is_protected: true,
    protection_reason: 'Active Default Launcher for User 0',
  },

  // 6. Already Removed for this user (Uninstalled state)
  {
    package_id: 'com.miui.analytics',
    state: 'Uninstalled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'oem',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'AlreadyRemoved',
    description: 'Xiaomi usage analytics and diagnostic telemetry daemon (previously uninstalled).',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
  {
    package_id: 'com.google.android.apps.tachyon',
    state: 'Uninstalled',
    is_system: true,
    package_type: 'System',
    user_id: 0,
    list_category: 'google',
    removal_recommendation: 'Recommended',
    removal_eligibility: 'AlreadyRemoved',
    description: 'Google Meet / Duo pre-installed communications client (previously uninstalled).',
    dependencies: [],
    needed_by: [],
    is_protected: false,
    protection_reason: null,
  },
];

const mockData = {
  get_app_config: {
    custom_adb_path: null,
    active_connection_id: 'preset-tz-gateway',
    connections: [
      {
        id: 'preset-tz-gateway',
        name: 'TZ AI Gateway (Integrated Router)',
        mode: 'Gateway',
        preset: 'FreeLlmApi',
        base_url: 'http://localhost:3001/v1',
        protocol: 'OpenAiChatCompletions',
        credential_id: 'preset-tz-gateway',
        selected_model: 'gateway:auto',
        manual_model_id: null,
        last_test_status: 'Ready',
        last_verified_at: '2026-11-09T11:00:00Z',
        supported_capabilities: ['chat', 'streaming', 'model_listing', 'routing'],
        tested_for_generation: true,
        has_credential: true,
      },
    ],
    ai_consent_given: true,
    approved_destinations: ['http://localhost:3001'],
    auto_refresh_on_connect: true,
    confirm_each_action: true,
  },
  get_adb_status: {
    found: true,
    path: 'C:\\platform-tools\\adb.exe',
    version: 'Android Debug Bridge version 1.0.41',
    source: 'PlatformTools',
  },
  list_devices: [
    {
      serial: 'XGGE79BEEU6HEAIV',
      state: 'Ready',
      connection_type: 'USB',
      model: 'Poco X6 Pro 5G',
      brand: 'Xiaomi',
      manufacturer: 'Xiaomi',
      android_version: '15',
      android_sdk: 35,
      build_id: 'OS3.0.302.0.WNLINXM',
      security_patch: '2026-10-01',
    },
  ],
  list_users: [
    {
      id: 0,
      name: 'Owner',
      flags: 19,
      is_running: true,
      is_primary: true,
      profile_type: 'System',
      display_label: 'User 0 (Owner)',
    },
  ],
  scan_packages: pocoPackages,
  get_journal_history: mockJournal,
  get_audit_journal: mockJournal,
  list_snapshots: [],
  get_dataset_stats: {
    total_packages: 5399,
    recommended_count: 3120,
    advanced_count: 1450,
    expert_count: 620,
    unsafe_count: 209,
  },
};

const mockBridgeScript = `
<script>
window.__TAURI_INTERNALS__ = {
  invoke: async function(cmd, args) {
    const mock = ${JSON.stringify(mockData)};
    if (cmd === 'get_app_config') return mock.get_app_config;
    if (cmd === 'get_adb_status') return mock.get_adb_status;
    if (cmd === 'list_devices') return mock.list_devices;
    if (cmd === 'list_users') return mock.list_users;
    if (cmd === 'scan_packages') return mock.scan_packages;
    if (cmd === 'get_journal_history') return mock.get_journal_history;
    if (cmd === 'get_audit_journal') return mock.get_audit_journal;
    if (cmd === 'list_snapshots') return mock.list_snapshots;
    if (cmd === 'get_dataset_stats') return mock.get_dataset_stats;
    if (cmd === 'set_active_target') return null;
    if (cmd === 'save_app_config') return null;

    if (cmd === 'validate_batch_selection') {
      const ids = args.packageIds || [];
      const items = ids.map(id => {
        const pkg = mock.scan_packages.find(p => p.package_id === id);
        if (!pkg) {
          return {
            package_id: id,
            package_type: 'User',
            current_state: 'Uninstalled',
            action: args.action,
            target_user: args.userId || 0,
            eligible: false,
            exclusion_reason: 'Package not found on target device for this user profile'
          };
        }
        let eligible = true;
        let reason = null;
        if (pkg.state === 'Uninstalled' && (args.action === 'UninstallUser' || args.action === 'Disable')) {
          eligible = false;
          reason = 'Package is already uninstalled for this user';
        } else if (pkg.is_protected) {
          eligible = false;
          reason = 'Protected component: ' + (pkg.protection_reason || 'Critical system package');
        } else if (id.startsWith('com.mediatek.telephony') || id.startsWith('com.mediatek.ims')) {
          eligible = false;
          reason = 'Confirmed critical telephony/modem/radio vendor service';
        }
        return {
          package_id: pkg.package_id,
          package_type: pkg.package_type,
          current_state: pkg.state,
          action: args.action,
          target_user: args.userId || 0,
          eligible,
          exclusion_reason: reason
        };
      });
      return {
        items,
        total_requested: ids.length,
        eligible_count: items.filter(i => i.eligible).length,
        excluded_count: items.filter(i => !i.eligible).length
      };
    }

    return {};
  }
};
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener: () => {}
};
</script>
`;

const mimeTypes = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
};

const server = http.createServer((req, res) => {
  const urlPath = req.url.split('?')[0];
  let filePath = (urlPath === '/' || urlPath === '') ? path.join(distDir, 'index.html') : path.join(distDir, urlPath);
  if (!fs.existsSync(filePath)) {
    filePath = path.join(distDir, 'index.html');
  }

  const ext = path.extname(filePath);
  const contentType = mimeTypes[ext] || 'application/octet-stream';

  if (ext === '.html') {
    let html = fs.readFileSync(filePath, 'utf8');
    html = html.replace('<head>', '<head>' + mockBridgeScript);
    res.writeHead(200, { 'Content-Type': 'text/html' });
    res.end(html);
  } else {
    fs.readFile(filePath, (err, data) => {
      if (err) {
        res.writeHead(404);
        res.end();
      } else {
        res.writeHead(200, { 'Content-Type': contentType });
        res.end(data);
      }
    });
  }
});

const PORT = 4175;
server.listen(PORT, async () => {
  console.log(`POCO Verification server listening at http://localhost:${PORT}`);

  const chromePath = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';

  // Helper to capture a screenshot via CDP
  async function captureWithBrowser(url, outFile, actionsFn, windowSize = '1280,840') {
    const proc = spawn(chromePath, [
      '--remote-debugging-port=9333',
      '--headless',
      '--disable-gpu',
      '--hide-scrollbars',
      `--window-size=${windowSize}`,
      url,
    ]);

    try {
      // Connect to CDP
      let pageTarget = null;
      for (let i = 0; i < 30; i++) {
        try {
          const targets = await new Promise((resolve, reject) => {
            http.get('http://localhost:9333/json', (res) => {
              let buf = '';
              res.on('data', chunk => buf += chunk);
              res.on('end', () => resolve(JSON.parse(buf)));
            }).on('error', reject);
          });
          pageTarget = targets.find(t => t.type === 'page');
          if (pageTarget) break;
        } catch {
          await new Promise(r => setTimeout(r, 200));
        }
      }

      if (!pageTarget) throw new Error('Could not find CDP page target');

      const ws = new WebSocket(pageTarget.webSocketDebuggerUrl);
      let msgId = 1;
      const send = (method, params = {}) => new Promise((resolve, reject) => {
        const id = msgId++;
        const handler = (evt) => {
          const data = JSON.parse(evt.data);
          if (data.id === id) {
            ws.removeEventListener('message', handler);
            if (data.error) reject(new Error(data.error.message));
            else resolve(data.result);
          }
        };
        ws.addEventListener('message', handler);
        ws.send(JSON.stringify({ id, method, params }));
      });

      await new Promise(r => ws.onopen = r);

      // Wait for page to load
      await new Promise(r => setTimeout(r, 1200));

      // Run any DOM action if needed
      if (actionsFn) {
        await send('Runtime.evaluate', {
          expression: `(${actionsFn.toString()})()`,
          awaitPromise: true,
        });
        await new Promise(r => setTimeout(r, 800));
      }

      // Take screenshot
      const shot = await send('Page.captureScreenshot', { format: 'png' });
      const buf = Buffer.from(shot.data, 'base64');
      fs.writeFileSync(outFile, buf);

      // Copy to artifact dir
      const baseName = path.basename(outFile);
      fs.writeFileSync(path.join(ARTIFACT_DIR, baseName), buf);
      console.log(`✓ Saved: ${baseName} (Local + Artifact)`);

      ws.close();
    } finally {
      proc.kill();
      await new Promise(r => setTimeout(r, 400));
    }
  }

  try {
    // 1. Debloat Candidates View (Default view)
    console.log('\n--- 1. Capturing poco_debloat_candidates.png ---');
    await captureWithBrowser(
      `http://localhost:${PORT}/?tab=packages`,
      path.join(LOCAL_DIR, 'poco_debloat_candidates.png'),
      () => {
        // Assert view is Debloat candidates (default)
      }
    );

    // 2. System & Vendor View
    console.log('\n--- 2. Capturing poco_system_vendor.png ---');
    await captureWithBrowser(
      `http://localhost:${PORT}/?tab=packages`,
      path.join(LOCAL_DIR, 'poco_system_vendor.png'),
      () => {
        // Click "System & vendor" tab button
        const btns = Array.from(document.querySelectorAll('button'));
        const sysBtn = btns.find(b => b.textContent && b.textContent.includes('System & vendor'));
        if (sysBtn) sysBtn.click();
      }
    );

    // 3. Removed for this user View
    console.log('\n--- 3. Capturing poco_removed_user.png ---');
    await captureWithBrowser(
      `http://localhost:${PORT}/?tab=packages`,
      path.join(LOCAL_DIR, 'poco_removed_user.png'),
      () => {
        // Click "Removed for this user" tab button
        const btns = Array.from(document.querySelectorAll('button'));
        const remBtn = btns.find(b => b.textContent && b.textContent.includes('Removed for this user'));
        if (remBtn) remBtn.click();
      }
    );

    // 4. Preflight Revalidation Batch Modal
    console.log('\n--- 4. Capturing poco_batch_preflight.png ---');
    await captureWithBrowser(
      `http://localhost:${PORT}/?tab=packages`,
      path.join(LOCAL_DIR, 'poco_batch_preflight.png'),
      async () => {
        // Select all visible packages in Debloat candidates
        const chks = document.querySelectorAll('tbody input[type="checkbox"]');
        chks.forEach(c => { if (!c.disabled && !c.checked) c.click(); });
        await new Promise(r => setTimeout(r, 400));

        // Click "Batch uninstall (user)" button
        const batchBtns = Array.from(document.querySelectorAll('button'));
        const uninstallBtn = batchBtns.find(b => b.textContent && b.textContent.includes('Batch uninstall'));
        if (uninstallBtn) uninstallBtn.click();
      }
    );

    console.log('\nAll POCO verification screenshots captured successfully!');
  } catch (e) {
    console.error('Capture error:', e);
  } finally {
    server.close();
    process.exit(0);
  }
});

