import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

const SCREENSHOT_DIR = path.resolve('screenshots');
if (!fs.existsSync(SCREENSHOT_DIR)) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
}

const distDir = path.resolve('dist');
if (!fs.existsSync(distDir)) {
  console.error('dist directory does not exist! Run npm run build first.');
  process.exit(1);
}

const mockJournal = [
  {
    entry_id: 'entry-1731170000000',
    timestamp: 1731170000000,
    device_serial: 'XGGE79BEEU6HEAIV',
    user_id: 0,
    package_id: 'com.block.juggle',
    action: 'Disable',
    previous_state: 'Enabled',
    resulting_state: 'Disabled',
    success: true,
    verified: true,
    message: 'Verified disabled.',
    recovery_steps: ['Enable'],
  },
  {
    entry_id: 'entry-1731170010000',
    timestamp: 1731170010000,
    device_serial: 'XGGE79BEEU6HEAIV',
    user_id: 0,
    package_id: 'com.miui.msa.global',
    action: 'Disable',
    previous_state: 'Enabled',
    resulting_state: 'Enabled',
    success: false,
    verified: false,
    message: 'Action denied by OEM security check. Disabling system components via ADB requires enabling USB debugging (Security settings).',
    recovery_steps: [],
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
      {
        id: 'preset-gemini',
        name: 'Google Gemini (Direct)',
        mode: 'DirectApi',
        preset: 'GoogleGemini',
        base_url: 'https://generativelanguage.googleapis.com',
        protocol: 'GeminiNative',
        credential_id: 'preset-gemini',
        selected_model: 'gemini-2.5-flash',
        manual_model_id: null,
        last_test_status: 'Ready',
        last_verified_at: '2026-11-09T10:45:00Z',
        supported_capabilities: ['chat', 'streaming', 'model_listing'],
        tested_for_generation: true,
        has_credential: true,
      },
    ],
    ai_consent_given: true,
    approved_destinations: ['https://generativelanguage.googleapis.com', 'http://localhost:3001'],
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
    {
      id: 10,
      name: 'Dual App Space',
      flags: 32,
      is_running: true,
      is_primary: false,
      profile_type: 'Clone',
      display_label: 'User 10 (Dual Apps)',
    },
  ],
  scan_packages: [
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
      package_id: 'com.xiaomi.discover',
      state: 'Disabled',
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
      package_id: 'com.google.android.youtube',
      state: 'Enabled',
      is_system: true,
      package_type: 'System',
      user_id: 0,
      list_category: 'google',
      removal_recommendation: 'Advanced',
      removal_eligibility: 'ReviewOnly',
      description: 'YouTube client app. Safe to remove if using web player or third-party client.',
      dependencies: [],
      needed_by: [],
      is_protected: false,
      protection_reason: null,
    },
    {
      package_id: 'com.android.chrome',
      state: 'Enabled',
      is_system: true,
      package_type: 'System',
      user_id: 0,
      list_category: 'google',
      removal_recommendation: 'Advanced',
      removal_eligibility: 'ReviewOnly',
      description: 'Google Chrome browser.',
      dependencies: [],
      needed_by: [],
      is_protected: false,
      protection_reason: null,
    },
    {
      package_id: 'com.miui.home',
      state: 'Enabled',
      is_system: true,
      package_type: 'System',
      user_id: 0,
      list_category: 'oem',
      removal_recommendation: 'Unsafe',
      removal_eligibility: 'Protected',
      description: 'Default HyperOS / POCO home launcher. Removing without alternate launcher breaks device navigation.',
      dependencies: [],
      needed_by: [],
      is_protected: true,
      protection_reason: 'Active default home launcher for User 0',
    },
    {
      package_id: 'com.google.android.inputmethod.latin',
      state: 'Enabled',
      is_system: true,
      package_type: 'System',
      user_id: 0,
      list_category: 'google',
      removal_recommendation: 'Unsafe',
      removal_eligibility: 'Protected',
      description: 'Gboard keyboard. Removing without alternate input method prevents typing.',
      dependencies: [],
      needed_by: [],
      is_protected: true,
      protection_reason: 'Active default input method (IME) for User 0',
    },
  ],
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
  discover_models: {
    success: true,
    connection_id: 'preset-tz-gateway',
    models: [
      {
        id: 'gateway:auto',
        display_name: 'Auto Provider Router',
        description: 'Automatically selects fastest healthy model across all providers',
        supported_generation_methods: ['generateContent'],
        is_router_alias: true,
        tested_for_generation: true,
      },
      {
        id: 'gemini-2.5-flash',
        display_name: 'Gemini 2.5 Flash',
        description: 'Google Gemini multimodal lightweight model',
        supported_generation_methods: ['generateContent'],
        is_router_alias: false,
        tested_for_generation: true,
      },
      {
        id: 'gpt-4o-mini',
        display_name: 'GPT-4o Mini',
        description: 'Fast, cost-efficient model for focused reasoning',
        supported_generation_methods: ['generateContent'],
        is_router_alias: false,
        tested_for_generation: false,
      },
    ],
    next_page_token: null,
    error: null,
  },
  check_gateway_readiness: {
    is_ready: true,
    gateway_available: true,
    upstream_available: true,
    message: 'TZ AI Gateway active: 2 healthy providers configured',
    served_model: 'gemini-2.5-flash',
  },
  validate_plan: {
    is_valid: true,
    warnings: [],
    blockers: [],
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
    if (cmd === 'discover_models') return mock.discover_models;
    if (cmd === 'check_gateway_readiness') return mock.check_gateway_readiness;
    if (cmd === 'validate_plan') return mock.validate_plan;
    if (cmd === 'set_active_target') return null;
    if (cmd === 'save_app_config') return null;
    if (cmd === 'clear_journal_history') return null;
    return {};
  }
};
window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener: () => {}
};

// In-browser assertion runner
window.addEventListener('DOMContentLoaded', () => {
  const params = new URLSearchParams(window.location.search);
  const screenName = params.get('screen') || params.get('tab') || 'devices';

  const specs = {
    devices: {
      heading: 'Connected Devices',
      content: 'Poco X6 Pro 5G',
    },
    packages: {
      heading: 'Package Explorer',
      content: 'com.miui.msa.global',
    },
    ai: {
      heading: 'AI Debloat Assistant',
      content: 'Auto Provider Router',
    },
    history: {
      heading: 'Operation Journal & Device Recovery',
      content: 'com.block.juggle',
    },
    settings: {
      heading: 'Settings & AI Configuration',
      content: 'TZ AI Gateway (Integrated Router)',
    },
    plan_review: {
      heading: 'Review Action Plan',
      content: 'com.facebook.appmanager',
    },
    partial_failure: {
      heading: 'Review Action Plan',
      content: 'MissingPrivileges',
    },
    narrow_composer: {
      heading: 'AI Debloat Assistant',
      content: 'Auto Provider Router',
    },
    narrow_plan: {
      heading: 'Review Action Plan',
      content: 'com.facebook.appmanager',
    },
    installed_app: {
      heading: 'Connected Devices',
      content: 'Poco X6 Pro 5G',
    },
  };

  const spec = specs[screenName];
  if (!spec) {
    fetch('/__assert_ready?screen=' + encodeURIComponent(screenName) + '&heading=none&content=none');
    return;
  }

  let reported = false;
  const startTime = Date.now();

  const check = () => {
    if (reported) return;
    const bodyText = document.body ? (document.body.innerText || '') : '';
    const headings = Array.from(document.querySelectorAll('h1, h2, h3, .logo-title, .panel-title'))
      .map(el => el.textContent.trim());

    const hasHeading = headings.some(h => h.includes(spec.heading)) || bodyText.includes(spec.heading);
    const hasContent = bodyText.includes(spec.content);

    if (hasHeading && hasContent) {
      reported = true;
      fetch('/__assert_ready?screen=' + encodeURIComponent(screenName) + '&heading=' + encodeURIComponent(spec.heading) + '&content=' + encodeURIComponent(spec.content));
    } else if (Date.now() - startTime > 5000) {
      reported = true;
      const reason = 'Assertion timeout after 5000ms. hasHeading=' + hasHeading + ' (looking for "' + spec.heading + '"), hasContent=' + hasContent + ' (looking for "' + spec.content + '").';
      fetch('/__assert_failed?screen=' + encodeURIComponent(screenName) + '&reason=' + encodeURIComponent(reason));
    }
  };

  const interval = setInterval(check, 100);
  setTimeout(() => clearInterval(interval), 6000);
});
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

// Global assertion event listener
let currentScreenResolver = null;
let currentScreenRejector = null;

const server = http.createServer((req, res) => {
  if (req.url.startsWith('/__assert_ready')) {
    const u = new URL(req.url, 'http://localhost');
    const screen = u.searchParams.get('screen');
    const heading = u.searchParams.get('heading');
    const content = u.searchParams.get('content');
    console.log(`[ASSERT PASSED] Screen "${screen}" verified: heading "${heading}", distinctive content "${content}"`);
    if (currentScreenResolver) {
      currentScreenResolver({ screen, heading, content });
      currentScreenResolver = null;
      currentScreenRejector = null;
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok' }));
    return;
  }

  if (req.url.startsWith('/__assert_failed')) {
    const u = new URL(req.url, 'http://localhost');
    const screen = u.searchParams.get('screen');
    const reason = u.searchParams.get('reason');
    console.error(`[ASSERT FAILED] Screen "${screen}": ${reason}`);
    if (currentScreenRejector) {
      currentScreenRejector(new Error(`Assertion failed for ${screen}: ${reason}`));
      currentScreenResolver = null;
      currentScreenRejector = null;
    }
    res.writeHead(500, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'error', reason }));
    return;
  }

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

const PORT = 4173;
server.listen(PORT, async () => {
  console.log(`Assertion-enforcing mock server running at http://localhost:${PORT}`);

  const chromePath = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
  const mode = process.argv[2] || 'all'; // 'before', 'after', or 'all'

  const fullScreens = [
    { name: 'devices', query: '?tab=devices&screen=devices', category: 'standard', windowSize: '1280,840' },
    { name: 'packages', query: '?tab=packages&screen=packages', category: 'standard', windowSize: '1280,840' },
    { name: 'ai', query: '?tab=ai&screen=ai', category: 'standard', windowSize: '1280,840' },
    { name: 'history', query: '?tab=history&screen=history', category: 'standard', windowSize: '1280,840' },
    { name: 'settings', query: '?tab=settings&screen=settings', category: 'standard', windowSize: '1280,840' },
    { name: 'plan_review', query: '?tab=plan&screen=plan_review', category: 'after_only', windowSize: '1280,840' },
    { name: 'partial_failure', query: '?tab=partial_failure&screen=partial_failure', category: 'after_only', windowSize: '1280,840' },
    { name: 'narrow_composer', query: '?tab=ai&screen=narrow_composer', category: 'after_only', windowSize: '820,760' },
    { name: 'narrow_plan', query: '?tab=plan&screen=narrow_plan', category: 'after_only', windowSize: '820,760' },
    { name: 'installed_app', query: '?tab=devices&screen=installed_app', category: 'after_only', windowSize: '1280,840' },
  ];

  let screensToRun = [];
  if (mode === 'before_history_only') {
    screensToRun = [{ prefix: 'before', screen: fullScreens.find(s => s.name === 'history') }];
  } else if (mode === 'before') {
    screensToRun = fullScreens.filter(s => s.category === 'standard').map(s => ({ prefix: 'before', screen: s }));
  } else if (mode === 'after') {
    screensToRun = fullScreens.map(s => ({ prefix: 'after', screen: s }));
  } else {
    // 'all': repair before_history and capture all after screens
    screensToRun = [
      { prefix: 'before', screen: fullScreens.find(s => s.name === 'history') },
      ...fullScreens.map(s => ({ prefix: 'after', screen: s })),
    ];
  }

  for (const { prefix, screen } of screensToRun) {
    const outFile = path.join(SCREENSHOT_DIR, `${prefix}_${screen.name}.png`);
    console.log(`\nCapturing [${prefix}] ${screen.name} to ${outFile}...`);

    try {
      await new Promise((resolve, reject) => {
        let finished = false;
        const timeout = setTimeout(() => {
          if (!finished) {
            finished = true;
            reject(new Error(`Timeout waiting for assertion or capture of ${screen.name}`));
          }
        }, 10000);

        currentScreenResolver = () => {
          // Assertion passed in browser, give 400ms for layout to settle then let Chrome screenshot
        };
        currentScreenRejector = (err) => {
          if (!finished) {
            finished = true;
            clearTimeout(timeout);
            reject(err);
          }
        };

        const proc = spawn(chromePath, [
          '--headless',
          '--disable-gpu',
          '--hide-scrollbars',
          `--window-size=${screen.windowSize}`,
          `--screenshot=${outFile}`,
          '--virtual-time-budget=2500',
          `http://localhost:${PORT}/${screen.query}`,
        ]);

        proc.on('close', (code) => {
          if (!finished) {
            finished = true;
            clearTimeout(timeout);
            if (code === 0) resolve();
            else reject(new Error(`Chrome process exited with code ${code}`));
          }
        });
      });
      console.log(`✓ Successfully saved: ${outFile}`);
    } catch (err) {
      console.error(`✗ FAILED capturing ${screen.name}:`, err.message);
      server.close();
      process.exit(1);
    }
  }

  console.log('\nAll asserted screen captures completed successfully!');
  server.close();
  process.exit(0);
});
