import { spawn } from 'node:child_process';
import http from 'node:http';

const exePath = 'C:\\Users\\singh\\AppData\\Local\\TZ AI DEBLOATER TOOL\\tz-ai-debloater-tool.exe';
const proc = spawn(exePath, [], {
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222' },
  stdio: 'ignore'
});

setTimeout(async () => {
  const data = await new Promise((res, rej) => {
    http.get('http://localhost:9222/json', (r) => {
      let b = '';
      r.on('data', c => b += c);
      r.on('end', () => res(b));
    }).on('error', rej);
  });
  const page = JSON.parse(data).find(t => t.type === 'page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  ws.onopen = () => {
    ws.send(JSON.stringify({
      id: 1,
      method: 'Runtime.evaluate',
      params: {
        expression: 'JSON.stringify(Object.getOwnPropertyDescriptor(window.__TAURI_INTERNALS__, "invoke"))',
        returnByValue: true
      }
    }));
  };
  ws.onmessage = (ev) => {
    console.log('invoke descriptor:', JSON.parse(ev.data).result?.result?.value);
    ws.close();
    proc.kill();
  };
}, 2500);

