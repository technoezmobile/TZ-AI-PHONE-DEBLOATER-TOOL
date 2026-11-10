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
  async eval(expression) {
    const res = await new Promise((resolve, reject) => {
      const msgId = this.id++;
      this.callbacks.set(msgId, { resolve, reject });
      this.ws.send(JSON.stringify({
        id: msgId,
        method: 'Runtime.evaluate',
        params: { expression, returnByValue: true, awaitPromise: true }
      }));
    });
    return res.result?.value;
  }
  close() {
    if (this.ws) this.ws.close();
  }
}

async function main() {
  console.log('Testing live Gemini via installed app...');
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

    console.log('Connected to installed app via CDP. Invoking test_saved_credential("preset-gemini")...');
    const res = await cdp.eval(`
      (async () => {
        try {
          const result = await window.__TAURI_INTERNALS__.invoke('test_saved_credential', { connectionId: 'preset-gemini' });
          return { ok: true, result };
        } catch (err) {
          return { ok: false, error: err };
        }
      })()
    `);

    console.log('Test Saved Credential Result:', JSON.stringify(res, null, 2));

    cdp.close();
  } catch (e) {
    console.error('Error during live test:', e);
  } finally {
    try { proc.kill(); } catch {}
  }
}

main();

