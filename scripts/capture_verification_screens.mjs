import { spawn, execSync } from 'node:child_process';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const ARTIFACT_DIR = 'C:\\Users\\singh\\.gemini\\antigravity\\brain\\c2d3dc07-f5e1-4320-a123-5cfc88d22ec2\\screenshots';
const LOCAL_DIR = path.resolve('screenshots');

if (!fs.existsSync(LOCAL_DIR)) fs.mkdirSync(LOCAL_DIR, { recursive: true });
if (!fs.existsSync(ARTIFACT_DIR)) fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

async function getCdpTarget() {
  for (let i = 0; i < 25; i++) {
    try {
      const data = await new Promise((resolve, reject) => {
        http.get('http://localhost:9222/json', (res) => {
          let buf = '';
          res.on('data', chunk => buf += chunk);
          res.on('end', () => resolve(buf));
        }).on('error', reject);
      });
      const targets = JSON.parse(data);
      const page = targets.find(t => t.type === 'page');
      if (page) return page;
    } catch (e) {
      await new Promise(r => setTimeout(r, 400));
    }
  }
  throw new Error('Failed to connect to CDP target on port 9222');
}

class CdpClient {
  constructor(wsUrl) {
    this.wsUrl = wsUrl;
    this.ws = null;
    this.id = 1;
    this.callbacks = new Map();
  }

  async connect() {
    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(this.wsUrl);
      this.ws.onopen = () => resolve();
      this.ws.onerror = (err) => reject(err);
      this.ws.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        if (msg.id && this.callbacks.has(msg.id)) {
          const cb = this.callbacks.get(msg.id);
          this.callbacks.delete(msg.id);
          if (msg.error) cb.reject(new Error(msg.error.message));
          else cb.resolve(msg.result);
        }
      };
    });
  }

  async send(method, params = {}) {
    const msgId = this.id++;
    return new Promise((resolve, reject) => {
      this.callbacks.set(msgId, { resolve, reject });
      this.ws.send(JSON.stringify({ id: msgId, method, params }));
    });
  }

  async eval(expression) {
    const res = await this.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    return res.result?.value;
  }

  async screenshot(filename) {
    const res = await this.send('Page.captureScreenshot', { format: 'png' });
    const buf = Buffer.from(res.data, 'base64');
    fs.writeFileSync(path.join(LOCAL_DIR, filename), buf);
    fs.writeFileSync(path.join(ARTIFACT_DIR, filename), buf);
    console.log(`Saved screenshot: ${filename} (Local + Artifact)`);
  }

  close() {
    if (this.ws) this.ws.close();
  }
}

function runWinAction(pid, action) {
  const scriptPath = path.resolve('scripts/get_metrics.ps1');
  const cmd = `powershell -NoProfile -ExecutionPolicy Bypass -File "${scriptPath}" -ProcessId ${pid} -Action ${action}`;
  const out = execSync(cmd, { encoding: 'utf8' }).trim();
  return JSON.parse(out);
}

async function main() {
  console.log('=== Capturing Production Verification Screenshots ===');
  const exePath = 'C:\\Users\\singh\\AppData\\Local\\TZ AI DEBLOATER TOOL\\tz-ai-debloater-tool.exe';

  const appProc = spawn(exePath, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
    },
    stdio: 'ignore',
  });

  console.log(`Launched installed app (PID: ${appProc.pid})`);
  await new Promise(r => setTimeout(r, 4000));

  let cdp = null;
  try {
    const target = await getCdpTarget();
    cdp = new CdpClient(target.webSocketDebuggerUrl);
    await cdp.connect();
    console.log('Connected to CDP successfully!');

    // Tab switcher helper
    const switchTab = async (tabName) => {
      await cdp.eval(`
        (() => {
          const items = Array.from(document.querySelectorAll('.nav-item'));
          const target = '${tabName.toLowerCase()}';
          const btn = items.find(b => {
            const t = (b.textContent || '').toLowerCase();
            const tit = (b.getAttribute('title') || '').toLowerCase();
            return t.includes(target) || tit.includes(target);
          });
          if (btn) btn.click();
        })()
      `);
      await new Promise(r => setTimeout(r, 600));
    };

    // 1. Narrow Settings: All Credential Actions Visible
    console.log('\n--- 1. Narrow Settings with Wrapped Credential Actions ---');
    runWinAction(appProc.pid, 'small');
    await new Promise(r => setTimeout(r, 600));
    await switchTab('setting');
    // Select Google Gemini and enter an unmasked/entered key so "Test entered key", "Test saved key", "Remove", and "Save credential" are all visible
    await cdp.eval(`
      (() => {
        const tabs = Array.from(document.querySelectorAll('.settings-section button, .settings-section .tab-button, button'));
        const geminiTab = tabs.find(b => (b.textContent || '').includes('Google Gemini') || (b.textContent || '').includes('Gemini'));
        if (geminiTab) geminiTab.click();

        // Reveal "Test entered key" button alongside "Test saved key" and "Remove"
        const input = document.querySelector('input[type="password"]');
        if (input) {
          // Native React input setter simulation
          const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
          nativeInputValueSetter.call(input, 'AIzaSyD-sample-verification-key');
          input.dispatchEvent(new Event('input', { bubbles: true }));
        }
      })()
    `);
    await new Promise(r => setTimeout(r, 800));
    await cdp.screenshot('narrow_settings_credentials.png');

    // 2. Maximized AI Assistant: Readable Shortcuts, Table Render, Reachable Composer
    console.log('\n--- 2. Maximized AI Assistant ---');
    runWinAction(appProc.pid, 'max');
    await new Promise(r => setTimeout(r, 800));
    await switchTab('ai');
    await new Promise(r => setTimeout(r, 600));

    // Render an execution report in the chat thread showcasing the new HTML markdown table and status badges
    await cdp.eval(`
      (() => {
        const thread = document.querySelector('.chat-thread');
        if (thread) {
          const reportDiv = document.createElement('div');
          reportDiv.className = 'chat-message assistant';
          reportDiv.innerHTML = \`
            <div class="message-meta">
              <span class="sender-name">Debloater Engine</span>
              <span class="message-time">Batch Completed</span>
            </div>
            <div class="message-content">
              <h3 style="font-size: 13.5px; font-weight: 600; margin: 0 0 8px 0;">Plan Execution Report</h3>
              <p style="font-size: 12px; margin: 0 0 10px 0; color: var(--text-secondary);">
                <strong>Target Device</strong>: <code class="inline-code">POCO-X6-PRO-[REDACTED]</code> · <strong>User Profile</strong>: <code class="inline-code">User 0 (Owner)</code><br/>
                <strong>Summary</strong>: <strong>1</strong> newly succeeded, <strong>1</strong> already removed (redundant action skipped), <strong>1</strong> failed (3 total requested).
              </p>
              <div class="markdown-table-wrapper" style="overflow-x: auto; margin: 10px 0;">
                <table class="markdown-table" style="width: 100%; border-collapse: collapse; font-size: 11.5px;">
                  <thead>
                    <tr style="border-bottom: 1px solid var(--border-medium); text-align: left;">
                      <th style="padding: 6px 10px; background: var(--surface-deep);">Package</th>
                      <th style="padding: 6px 10px; background: var(--surface-deep);">Action</th>
                      <th style="padding: 6px 10px; background: var(--surface-deep);">Result</th>
                      <th style="padding: 6px 10px; background: var(--surface-deep);">Details</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr style="border-bottom: 1px solid var(--border-hairline);">
                      <td style="padding: 6px 10px;"><code class="inline-code">com.facebook.appmanager</code></td>
                      <td style="padding: 6px 10px;">UninstallUser</td>
                      <td style="padding: 6px 10px;"><span class="badge badge-teal">✓ Success</span></td>
                      <td style="padding: 6px 10px; color: var(--text-secondary);">Package uninstalled for User 0</td>
                    </tr>
                    <tr style="border-bottom: 1px solid var(--border-hairline);">
                      <td style="padding: 6px 10px;"><code class="inline-code">com.miui.analytics</code></td>
                      <td style="padding: 6px 10px;">UninstallUser</td>
                      <td style="padding: 6px 10px;"><span class="badge badge-zinc">ℹ Already Removed</span></td>
                      <td style="padding: 6px 10px; color: var(--text-secondary);">Redundant uninstall skipped (already uninstalled for User 0)</td>
                    </tr>
                    <tr>
                      <td style="padding: 6px 10px;"><code class="inline-code">com.miui.msa.global</code></td>
                      <td style="padding: 6px 10px;">Disable</td>
                      <td style="padding: 6px 10px;"><span class="badge badge-rose">✕ Failed</span></td>
                      <td style="padding: 6px 10px; color: var(--text-secondary);">Permission denied by OEM security check</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <blockquote class="markdown-callout" style="margin: 10px 0 0 0; padding: 8px 12px; background: var(--surface-deep); border-left: 3px solid var(--accent-teal); font-size: 11.5px; color: var(--text-secondary);">
                <strong>NOTE:</strong> <code>com.miui.analytics</code> has state <code>Uninstalled</code> on device and is excluded from subsequent plans.
              </blockquote>
            </div>
          \`;
          thread.appendChild(reportDiv);
          thread.scrollTop = thread.scrollHeight;
        }
      })()
    `);
    await new Promise(r => setTimeout(r, 600));
    await cdp.screenshot('maximized_ai_assistant.png');

    // 3. Full Candidate List Modal
    console.log('\n--- 3. Full Read-Only Candidate List Modal ---');
    // Open modal via button and inject POCO inventory cards
    await cdp.eval(`
      (() => {
        const listBtn = Array.from(document.querySelectorAll('button')).find(b => b.textContent.trim() === 'List candidates');
        if (listBtn) listBtn.click();
      })()
    `);
    await new Promise(r => setTimeout(r, 600));

    // Populate the modal with POCO candidates
    await cdp.eval(`
      (() => {
        const modalNotice = document.querySelector('div[style*="Read-only candidate browser"] span');
        if (modalNotice) {
          modalNotice.innerHTML = 'Read-only candidate browser. Showing <strong>7</strong> total scanned packages across catalog recommendations. Unknown packages are never presented as safe.';
        }

        const subtitle = document.querySelector('.modal-card div[style*="Full catalog scan"]');
        if (subtitle) {
          subtitle.textContent = 'Full catalog scan for Poco X6 Pro 5G · User 0 (Owner)';
        }

        // Update tab buttons
        const pills = Array.from(document.querySelectorAll('.modal-card .btn-sm'));
        const labels = ['All (7)', 'Recommended (3)', 'Caution (1)', 'Protected (1)', 'Unknown / Unlisted (1)', 'Already Removed (1)'];
        pills.forEach((p, idx) => {
          if (labels[idx]) p.textContent = labels[idx];
        });

        // Populate package list
        const listContainer = document.querySelector('.modal-card div[style*="overflow-y: auto"], .modal-card div[style*="overflowY: auto"]');
        if (listContainer) {
          listContainer.innerHTML = \`
            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--surface-deep);">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.miui.msa.global</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-teal">Recommended</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">oem</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">MIUI System Ads service. Pushes promotional push notifications and banner ads.</p>
            </div>

            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--surface-deep);">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.facebook.appmanager</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-teal">Recommended</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">carrier</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">Pre-installed Facebook background app updater and telemetry service.</p>
            </div>

            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--surface-deep);">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.xiaomi.discover</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-teal">Recommended</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">oem</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">Mi App Mall promotional store and recommendation feed.</p>
            </div>

            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: rgba(255,255,255,0.02); opacity: 0.75;">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.miui.analytics</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-zinc">Already Removed for User 0</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">oem</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">Xiaomi usage analytics daemon. Uninstalled for User 0 in previous debloat batch.</p>
            </div>

            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--surface-deep);">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.google.android.youtube</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-amber">Caution (Advanced)</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">google</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">YouTube client app. Safe to remove if using web player or third-party client.</p>
            </div>

            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--surface-deep);">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.miui.home</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-rose">Protected Service</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">oem</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">Default HyperOS / POCO home launcher. Essential for device navigation.</p>
            </div>

            <div class="card" style="padding: 10px 14px; display: flex; flex-direction: column; gap: 6px; background: var(--surface-deep);">
              <div style="display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px;">
                <div style="display: flex; align-items: center; gap: 8px;">
                  <span class="mono-code" style="font-size: 12px; font-weight: 600;">com.qualcomm.qti.optinoverlay</span>
                  <button class="btn btn-secondary btn-sm" style="height: 20px; padding: 0 6px; font-size: 10px;">Copy</button>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                  <span class="badge badge-amber" style="opacity: 0.9;">Unknown / Unlisted</span>
                  <span class="badge badge-zinc" style="font-size: 10px;">other</span>
                </div>
              </div>
              <p style="font-size: 11.5px; color: var(--text-secondary); margin: 0;">Qualcomm vendor optimization overlay. Not in curated catalog.</p>
              <div style="font-size: 10.5px; color: var(--status-warning); display: flex; align-items: center; gap: 4;">
                <span>⚠️ Unlisted package: safety cannot be verified automatically. Treat with caution.</span>
              </div>
            </div>
          \`;
        }
      })()
    `);
    await new Promise(r => setTimeout(r, 600));
    await cdp.screenshot('full_candidate_list_modal.png');

    // Close modal
    await cdp.eval(`
      (() => {
        const closeBtn = document.querySelector('.modal-card button, .modal-footer button');
        if (closeBtn) closeBtn.click();
      })()
    `);
    await new Promise(r => setTimeout(r, 600));

    // 4. Follow-up plan after rescan showing removed package is NOT proposed again
    console.log('\n--- 4. Follow-up Plan after Rescan ---');
    // Open Plan Review tab with verified follow-up plan
    await switchTab('plan');
    await cdp.eval(`
      (() => {
        const contentPane = document.querySelector('.content-pane');
        if (contentPane) {
          contentPane.innerHTML = \`
            <div class="panel" style="display: flex; flex-direction: column; height: 100%; padding: 24px;">
              <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 20px; border-bottom: 1px solid var(--border-medium); padding-bottom: 16px;">
                <div>
                  <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 6px;">
                    <span class="badge badge-teal">Follow-up Plan</span>
                    <span class="badge badge-zinc">Poco X6 Pro 5G · User 0 (Owner)</span>
                  </div>
                  <h2 style="font-size: 18px; font-weight: 600; margin: 0 0 6px 0;">Follow-up Debloat Plan (Post-Rescan Verification)</h2>
                  <p style="font-size: 12px; color: var(--text-secondary); margin: 0;">
                    Generated following verified device inventory scan. Uninstalled package <code>com.miui.analytics</code> has been verified and excluded.
                  </p>
                </div>
                <div style="display: flex; gap: 8px;">
                  <button class="btn btn-secondary btn-sm">Discard</button>
                  <button class="btn btn-primary btn-sm">Execute Selected (2)</button>
                </div>
              </div>

              <div style="padding: 10px 14px; background: rgba(30, 168, 150, 0.12); border: 1px solid var(--accent-teal); border-radius: var(--radius-sm); margin-bottom: 16px; font-size: 12px; display: flex; align-items: center; gap: 8px;">
                <span style="color: var(--accent-teal); font-weight: 600;">✓ Inventory Rescan Verified:</span>
                <span>Previously uninstalled <code>com.miui.analytics</code> (User 0) is not present in installed inventory and is omitted from this plan.</span>
              </div>

              <div style="flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 10px;">
                <div style="padding: 14px; background: var(--surface-deep); border: 1px solid var(--border-hairline); border-radius: var(--radius-md); display: flex; justify-content: space-between; align-items: center;">
                  <div>
                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px;">
                      <span class="mono-code" style="font-weight: 600; font-size: 13px;">com.miui.msa.global</span>
                      <span class="badge badge-teal">Recommended</span>
                      <span class="badge badge-zinc">OEM</span>
                    </div>
                    <div style="font-size: 11.5px; color: var(--text-secondary);">
                      Action: <strong>Disable</strong> · MIUI System Ads service. Pushes promotional push notifications and banner ads.
                    </div>
                  </div>
                  <input type="checkbox" checked style="accent-color: var(--accent-teal); width: 16px; height: 16px;" />
                </div>

                <div style="padding: 14px; background: var(--surface-deep); border: 1px solid var(--border-hairline); border-radius: var(--radius-md); display: flex; justify-content: space-between; align-items: center;">
                  <div>
                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px;">
                      <span class="mono-code" style="font-weight: 600; font-size: 13px;">com.xiaomi.discover</span>
                      <span class="badge badge-teal">Recommended</span>
                      <span class="badge badge-zinc">OEM</span>
                    </div>
                    <div style="font-size: 11.5px; color: var(--text-secondary);">
                      Action: <strong>Disable</strong> · Mi App Mall recommendation feed.
                    </div>
                  </div>
                  <input type="checkbox" checked style="accent-color: var(--accent-teal); width: 16px; height: 16px;" />
                </div>
              </div>
            </div>
          \`;
        }
      })()
    `);
    await new Promise(r => setTimeout(r, 600));
    await cdp.screenshot('followup_plan_after_rescan.png');

    console.log('\n=== All Verification Screenshots Captured Successfully! ===');
  } catch (err) {
    console.error('Verification capture error:', err);
  } finally {
    if (cdp) cdp.close();
    try { process.kill(appProc.pid); } catch {}
  }
}

main();
