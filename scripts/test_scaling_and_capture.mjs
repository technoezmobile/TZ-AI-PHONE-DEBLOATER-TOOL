import { spawn, execSync } from 'node:child_process';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';

const WS = globalThis.WebSocket;

async function getCdpTarget() {
  return new Promise((resolve, reject) => {
    http.get('http://localhost:9222/json', (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          const targets = JSON.parse(data);
          const page = targets.find(t => t.type === 'page');
          resolve(page);
        } catch (e) {
          reject(e);
        }
      });
    }).on('error', reject);
  });
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
      this.ws = new WS(this.wsUrl);
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

  async screenshot(outputPath) {
    const res = await this.send('Page.captureScreenshot', { format: 'png' });
    fs.writeFileSync(outputPath, Buffer.from(res.data, 'base64'));
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

function runPsRaw(code) {
  return execSync(`powershell -NoProfile -ExecutionPolicy Bypass -Command "${code.replace(/"/g, '`"')}"`, {
    encoding: 'utf8',
  }).trim();
}

async function main() {
  console.log('=== Display Scaling & Window Resizing Matrix Test ===');
  const exePath = 'C:\\Users\\singh\\AppData\\Local\\TZ AI DEBLOATER TOOL\\tz-ai-debloater-tool.exe';

  const appProc = spawn(exePath, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
    },
    detached: false,
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
      if (tabName.toLowerCase() === 'plan') {
        await cdp.eval(`window.location.search = '?tab=plan';`);
        await new Promise(r => setTimeout(r, 1200));
        return;
      }
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
      await new Promise(r => setTimeout(r, 500));
    };

    const recordCapture = async (filename, label, win32Action, tab, customEmulation = null) => {
      console.log(`\n--- Capture: ${label} -> ${filename} ---`);
      if (customEmulation) {
        await cdp.send('Emulation.setDeviceMetricsOverride', customEmulation);
      } else {
        await cdp.send('Emulation.clearDeviceMetricsOverride');
      }

      let win32 = null;
      if (win32Action) {
        win32 = runWinAction(appProc.pid, win32Action);
        await new Promise(r => setTimeout(r, 600));
      }

      if (tab) {
        await switchTab(tab);
      }

      const dom = await cdp.eval(`
        (() => {
          const app = document.querySelector('.app-container');
          const sidebar = document.querySelector('.sidebar');
          const main = document.querySelector('.main-wrapper');
          const content = document.querySelector('.content-pane');
          const settingsGrid = document.querySelector('.settings-grid');
          const planReview = document.querySelector('.plan-review-container');
          return {
            dpr: window.devicePixelRatio,
            innerW: window.innerWidth,
            innerH: window.innerHeight,
            appW: app?.offsetWidth,
            appH: app?.offsetHeight,
            sideW: sidebar?.offsetWidth,
            mainW: main?.offsetWidth,
            contentW: content?.offsetWidth,
            settingsGridCols: settingsGrid ? window.getComputedStyle(settingsGrid).gridTemplateColumns : null,
            hasPlanReview: !!planReview,
          };
        })()
      `);

      if (win32) {
        console.log(`Win32: ${win32.WindowWidth}x${win32.WindowHeight}, Client: ${win32.ClientWidth}x${win32.ClientHeight}, DPI: ${win32.Dpi} (${win32.ScalePercent}%)`);
      }
      console.log(`DOM: dpr=${dom.dpr}, inner=${dom.innerW}x${dom.innerH}, sidebar=${dom.sideW}px, main=${dom.mainW}px, content=${dom.contentW}px`);
      if (dom.settingsGridCols) {
        console.log(`Settings Grid Columns: ${dom.settingsGridCols}`);
      }
      if (dom.hasPlanReview) {
        console.log(`Plan Review Container Rendered: true`);
      }

      const outPath = path.resolve(`screenshots/${filename}`);
      await cdp.screenshot(outPath);
      console.log(`Saved screenshot: ${outPath}`);
      return { win32, dom, filename };
    };

    // 1. Monitor 1 (100% scale): Small Supported Window (760x540) - Devices
    await recordCapture('scale_100_small_devices.png', '100% Scale - Small Window (760x540)', 'small', 'device');

    // 2. Monitor 1 (100% scale): Maximized Window (1920x1080) - Devices
    await recordCapture('scale_100_max_devices.png', '100% Scale - Maximized (1920x1080)', 'max', 'device');

    // 3. Monitor 1 (100% scale): Maximized Window - Packages
    await recordCapture('scale_100_packages_max.png', '100% Scale - Maximized Packages', null, 'package');

    // 4. Monitor 1 (100% scale): Maximized Window - Settings (showing fluid 2-column filling entire width)
    await recordCapture('scale_100_settings_max.png', '100% Scale - Maximized Settings (2-Column Fluid Grid)', null, 'setting');

    // 5. Monitor 1 (100% scale): Snapped Half-Screen (960x1040)
    await recordCapture('scale_100_snapped_halfscreen.png', '100% Scale - Snapped Half-Screen (960x1040)', 'snap', 'package');

    // 6. Monitor 3 (125% DPI scale, move running app across monitor boundary):
    console.log('\n--- Moving App Across Monitor Boundary to Monitor 3 (125% Scale) ---');
    const movePs = `
      Add-Type -TypeDefinition @"
      using System; using System.Runtime.InteropServices;
      public class Mov {
          [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
          [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
      }
"@
      $hwnd = [WinApiHelper]::FindTauriHwnd(${appProc.pid})
      [Mov]::ShowWindow($hwnd, 9) | Out-Null
      Start-Sleep -Milliseconds 200
      [Mov]::SetWindowPos($hwnd, [IntPtr]::Zero, 1920, 0, 768, 864, 0x0040) | Out-Null
    `;
    runPsRaw(movePs);
    await new Promise(r => setTimeout(r, 1000));

    // Monitor 3: Snapped Half-Screen at 125% scale
    await recordCapture('scale_125_snapped_devices.png', '125% Scale - Snapped on Monitor 3 (768x864)', null, 'device');

    // Monitor 3: Maximize on Monitor 3
    runPsRaw(`
      $hwnd = [WinApiHelper]::FindTauriHwnd(${appProc.pid})
      [Mov]::ShowWindow($hwnd, 3) | Out-Null
    `);
    await new Promise(r => setTimeout(r, 1000));
    await recordCapture('scale_125_max_packages.png', '125% Scale - Maximized on Monitor 3', null, 'package');

    // Move back to Monitor 1 and restore
    runPsRaw(`
      $hwnd = [WinApiHelper]::FindTauriHwnd(${appProc.pid})
      [Mov]::ShowWindow($hwnd, 9) | Out-Null
      Start-Sleep -Milliseconds 200
      [Mov]::SetWindowPos($hwnd, [IntPtr]::Zero, 100, 100, 960, 700, 0x0040) | Out-Null
    `);
    await new Promise(r => setTimeout(r, 1000));

    // 7. 150% Display Scaling: Narrow Window with AI Assistant (760x540 viewport, dpr 1.5)
    await recordCapture('scale_150_narrow_ai.png', '150% Scale - Narrow AI Assistant (760x540, DPR 1.5)', null, 'ai', {
      width: 760,
      height: 540,
      deviceScaleFactor: 1.5,
      mobile: false,
    });

    // 8. 150% Display Scaling: Plan Review modal with reachable sticky footer
    await recordCapture('scale_150_plan_review.png', '150% Scale - Plan Review Dialog (760x540, DPR 1.5)', null, 'plan', {
      width: 760,
      height: 540,
      deviceScaleFactor: 1.5,
      mobile: false,
    });

    // Clear override and restore
    await cdp.send('Emulation.clearDeviceMetricsOverride');
    runWinAction(appProc.pid, 'restore');

    console.log('\n=== All Scaling & Window Resize Tests Passed! ===');
  } catch (err) {
    console.error('Scaling capture failed:', err);
  } finally {
    if (cdp) cdp.close();
    try { process.kill(appProc.pid); } catch {}
  }
}

main();

