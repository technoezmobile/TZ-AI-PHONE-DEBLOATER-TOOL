import { spawn } from 'node:child_process';
import http from 'node:http';

async function getCdpTarget() {
  for (let i = 0; i < 20; i++) {
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
      await new Promise(r => setTimeout(r, 500));
    }
  }
  throw new Error('Failed to find CDP page target on port 9222');
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
  close() {
    if (this.ws) this.ws.close();
  }
}

async function main() {
  const exePath = 'C:\\Users\\singh\\AppData\\Local\\TZ AI DEBLOATER TOOL\\tz-ai-debloater-tool.exe';
  const proc = spawn(exePath, [], {
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222' },
    stdio: 'ignore'
  });

  try {
    await new Promise(r => setTimeout(r, 3000));
    const target = await getCdpTarget();
    const cdp = new CdpClient(target.webSocketDebuggerUrl);
    await cdp.connect();

    // Check Tauri internals and properties
    const res = await cdp.eval(`
      (() => {
        return {
          hasInternals: !!window.__TAURI_INTERNALS__,
          desc: Object.getOwnPropertyDescriptor(window, '__TAURI_INTERNALS__'),
          keys: window.__TAURI_INTERNALS__ ? Object.keys(window.__TAURI_INTERNALS__) : []
        };
      })()
    `);
    console.log('Tauri Internals introspection:', JSON.stringify(res, null, 2));

    cdp.close();
  } catch (e) {
    console.error('Debug error:', e);
  } finally {
    try { proc.kill(); } catch {}
  }
}

main();
