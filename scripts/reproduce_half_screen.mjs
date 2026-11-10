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

function callMetricsPs(pid, action = 'get') {
  const scriptPath = path.resolve('scripts/get_metrics.ps1');
  const cmd = `powershell -NoProfile -ExecutionPolicy Bypass -File "${scriptPath}" -ProcessId ${pid} -Action ${action}`;
  const out = execSync(cmd, { encoding: 'utf8' }).trim();
  return JSON.parse(out);
}

async function main() {
  console.log('=== Starting Repro Diagnostics for Installed Windows App ===');
  const exePath = 'C:\\Users\\singh\\AppData\\Local\\TZ AI DEBLOATER TOOL\\tz-ai-debloater-tool.exe';
  if (!fs.existsSync(exePath)) {
    console.error('Installed exe not found at:', exePath);
    process.exit(1);
  }

  const appProc = spawn(exePath, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
    },
    detached: false,
    stdio: 'ignore',
  });

  console.log(`App launched with PID: ${appProc.pid}`);
  await new Promise(r => setTimeout(r, 3500));

  let cdp = null;
  try {
    const target = await getCdpTarget();
    console.log(`Found CDP target: ${target.title} (${target.webSocketDebuggerUrl})`);
    cdp = new CdpClient(target.webSocketDebuggerUrl);
    await cdp.connect();
    console.log('Connected to WebView2 CDP!');

    const getDomMetrics = async () => {
      return await cdp.eval(`
        (() => {
          const root = document.getElementById('root');
          const appContainer = document.querySelector('.app-container');
          const sidebar = document.querySelector('.sidebar');
          const mainWrapper = document.querySelector('.main-wrapper');
          const contentPane = document.querySelector('.content-pane');
          const targetStrip = document.querySelector('.top-bar');

          const rect = el => el ? {
            w: Math.round(el.getBoundingClientRect().width),
            h: Math.round(el.getBoundingClientRect().height),
            l: Math.round(el.getBoundingClientRect().left),
            t: Math.round(el.getBoundingClientRect().top)
          } : null;

          return {
            dpr: window.devicePixelRatio,
            innerWidth: window.innerWidth,
            innerHeight: window.innerHeight,
            outerWidth: window.outerWidth,
            outerHeight: window.outerHeight,
            htmlClientWidth: document.documentElement.clientWidth,
            htmlClientHeight: document.documentElement.clientHeight,
            bodyClientWidth: document.body.clientWidth,
            bodyClientHeight: document.body.clientHeight,
            root: rect(root),
            appContainer: rect(appContainer),
            sidebar: rect(sidebar),
            mainWrapper: rect(mainWrapper),
            contentPane: rect(contentPane),
            targetStrip: rect(targetStrip),
            activeTab: document.querySelector('.nav-item.active')?.textContent.trim() || 'none',
          };
        })()
      `);
    };

    const testState = async (stateName, action) => {
      console.log(`\n========================================`);
      console.log(`Testing Window State: [${stateName}] (action: ${action})`);
      console.log(`========================================`);
      const win32 = callMetricsPs(appProc.pid, action);
      await new Promise(r => setTimeout(r, 600)); // allow WebView2 reflow
      const dom = await getDomMetrics();

      console.log(`Win32 Native Window: ${win32.WindowWidth}x${win32.WindowHeight} at (${win32.WindowLeft},${win32.WindowTop})`);
      console.log(`Win32 Client Area:   ${win32.ClientWidth}x${win32.ClientHeight}`);
      console.log(`Display DPI Scale:   ${win32.Dpi} DPI (${win32.ScalePercent}%)`);
      console.log(`WebView Viewport:    ${dom.innerWidth}x${dom.innerHeight} (dpr: ${dom.dpr})`);
      console.log(`Root Bounds:         ${dom.root ? `${dom.root.w}x${dom.root.h}` : 'null'}`);
      console.log(`App Container:       ${dom.appContainer ? `${dom.appContainer.w}x${dom.appContainer.h}` : 'null'}`);
      console.log(`Sidebar:             ${dom.sidebar ? `${dom.sidebar.w}x${dom.sidebar.h}` : 'null'}`);
      console.log(`Main Wrapper:        ${dom.mainWrapper ? `${dom.mainWrapper.w}x${dom.mainWrapper.h}` : 'null'}`);
      console.log(`Content Pane:        ${dom.contentPane ? `${dom.contentPane.w}x${dom.contentPane.h}` : 'null'}`);

      // Calculate difference between client area and DOM
      const expectedLogicalClientW = Math.round(win32.ClientWidth / dom.dpr);
      const expectedLogicalClientH = Math.round(win32.ClientHeight / dom.dpr);
      const unusedViewportW = dom.innerWidth - (dom.appContainer ? dom.appContainer.w : 0);
      const unusedViewportH = dom.innerHeight - (dom.appContainer ? dom.appContainer.h : 0);

      console.log(`Logical Client Area: ${expectedLogicalClientW}x${expectedLogicalClientH}`);
      console.log(`Unused Viewport:     W=${unusedViewportW}px, H=${unusedViewportH}px`);

      // Screenshot
      const screenshotPath = path.resolve(`screenshots/repro_${stateName.toLowerCase()}.png`);
      await cdp.screenshot(screenshotPath);
      console.log(`Saved screenshot: ${screenshotPath}`);

      return { stateName, win32, dom, unusedViewportW, unusedViewportH };
    };

    // 1. Restored default
    const r1 = await testState('Restored', 'get');

    // 2. Snapped half-screen (960x1040)
    const r2 = await testState('Snapped_HalfScreen', 'snap');

    // 3. Maximized
    const r3 = await testState('Maximized', 'max');

    // Restore back
    callMetricsPs(appProc.pid, 'restore');

    console.log('\n=== Repro Test Completed Successfully ===');
  } catch (err) {
    console.error('Repro script failed:', err);
  } finally {
    if (cdp) cdp.close();
    try {
      process.kill(appProc.pid);
    } catch {}
  }
}

main();
