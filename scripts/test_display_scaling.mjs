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

function runPs(script) {
  return execSync(`powershell -NoProfile -ExecutionPolicy Bypass -Command "${script.replace(/"/g, '`"')}"`, {
    encoding: 'utf8',
  }).trim();
}

async function main() {
  console.log('=== Testing Display Scaling (100% and 125% Monitor) ===');
  const exePath = 'C:\\Users\\singh\\AppData\\Local\\TZ AI DEBLOATER TOOL\\tz-ai-debloater-tool.exe';
  const appProc = spawn(exePath, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
    },
    detached: false,
    stdio: 'ignore',
  });

  await new Promise(r => setTimeout(r, 3500));

  let cdp = null;
  try {
    const target = await getCdpTarget();
    cdp = new CdpClient(target.webSocketDebuggerUrl);
    await cdp.connect();

    const psFindHwnd = `
      Add-Type -TypeDefinition @"
      using System; using System.Runtime.InteropServices; using System.Text;
      public class F {
          [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc f, IntPtr l);
          [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
          [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int m);
          [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
          public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
          public static IntPtr Find(uint pid) {
              IntPtr res = IntPtr.Zero;
              EnumWindows((h, l) => {
                  uint p; GetWindowThreadProcessId(h, out p);
                  if (p == pid && IsWindowVisible(h)) {
                      var t = new StringBuilder(256); GetWindowText(h, t, 256);
                      if (t.ToString().Contains("DEBLOATER")) { res = h; return false; }
                  }
                  return true;
              }, IntPtr.Zero);
              return res;
          }
      }
"@
      [F]::Find(${appProc.pid}).ToInt64()
    `;
    const hwnd = runPs(psFindHwnd);
    console.log(`Target window HWND: ${hwnd}`);

    const moveAndMeasure = async (label, x, y, w, h, state = 9) => {
      console.log(`\n--- ${label} ---`);
      runPs(`
        Add-Type -TypeDefinition @"
        using System; using System.Runtime.InteropServices;
        public class M {
            [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
            [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
            [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
            [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out R r);
            [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
            [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, R, B; }
        }
"@
        $h = [IntPtr]${hwnd}
        [M]::ShowWindow($h, ${state}) | Out-Null
        if (${state} -eq 9) {
            [M]::SetWindowPos($h, [IntPtr]::Zero, ${x}, ${y}, ${w}, ${h}, 0x0040) | Out-Null
        }
      `);

      await new Promise(r => setTimeout(r, 800));

      const metricsJson = runPs(`
        $h = [IntPtr]${hwnd}
        $wr = New-Object M+R; $cr = New-Object M+R
        [M]::GetWindowRect($h, [ref]$wr) | Out-Null
        [M]::GetClientRect($h, [ref]$cr) | Out-Null
        $dpi = [M]::GetDpiForWindow($h)
        [PSCustomObject]@{
            Dpi = $dpi
            Scale = [math]::Round(($dpi/96)*100)
            WinW = ($wr.R - $wr.L)
            WinH = ($wr.B - $wr.T)
            ClientW = ($cr.R - $cr.L)
            ClientH = ($cr.B - $cr.T)
            WinL = $wr.L
            WinT = $wr.T
        } | ConvertTo-Json -Compress
      `);

      const native = JSON.parse(metricsJson);
      const dom = await cdp.eval(`
        (() => {
          return {
            dpr: window.devicePixelRatio,
            innerW: window.innerWidth,
            innerH: window.innerHeight,
            screenW: window.screen.width,
            screenH: window.screen.height,
            rootW: document.getElementById('root')?.offsetWidth,
            rootH: document.getElementById('root')?.offsetHeight,
            appW: document.querySelector('.app-container')?.offsetWidth,
            appH: document.querySelector('.app-container')?.offsetHeight,
            sideW: document.querySelector('.sidebar')?.offsetWidth,
            mainW: document.querySelector('.main-wrapper')?.offsetWidth,
          };
        })()
      `);

      console.log(`Native Win: ${native.WinW}x${native.WinH} at (${native.WinL},${native.WinT}), Client: ${native.ClientW}x${native.ClientH}, Scale: ${native.Scale}%`);
      console.log(`DOM: dpr=${dom.dpr}, inner=${dom.innerW}x${dom.innerH}, root=${dom.rootW}x${dom.rootH}, app=${dom.appW}x${dom.appH}, sidebar=${dom.sideW}, main=${dom.mainW}`);

      await cdp.screenshot(`screenshots/scale_${label.toLowerCase().replace(/[^a-z0-9]/g, '_')}.png`);
    };

    // Test on Display 1 (100% scale)
    await moveAndMeasure('Monitor1_Restored_100pct', 100, 100, 1280, 840, 9);
    await moveAndMeasure('Monitor1_Snapped_100pct', 0, 0, 960, 1040, 9);
    await moveAndMeasure('Monitor1_Maximized_100pct', 0, 0, 0, 0, 3);

    // Test on Display 3 (125% scale, offset at X=1920)
    await moveAndMeasure('Monitor3_Restored_125pct', 2000, 100, 1280, 840, 9);
    await moveAndMeasure('Monitor3_Snapped_125pct', 1920, 0, 768, 864, 9);
    await moveAndMeasure('Monitor3_Maximized_125pct', 1920, 0, 0, 0, 3);

    // Restore to normal
    runPs(`[M]::ShowWindow([IntPtr]${hwnd}, 9)`);
  } catch (err) {
    console.error('Scale test failed:', err);
  } finally {
    if (cdp) cdp.close();
    try { process.kill(appProc.pid); } catch {}
  }
}

main();

