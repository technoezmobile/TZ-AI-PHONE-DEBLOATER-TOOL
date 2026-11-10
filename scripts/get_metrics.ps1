param(
    [int]$ProcessId,
    [string]$Action = "get"
)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class WinApiHelper {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);

    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    public static IntPtr FindTauriHwnd(uint pid) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hWnd, lParam) => {
            uint p;
            GetWindowThreadProcessId(hWnd, out p);
            if (p == pid && IsWindowVisible(hWnd)) {
                var cls = new StringBuilder(256);
                GetClassName(hWnd, cls, 256);
                var title = new StringBuilder(256);
                GetWindowText(hWnd, title, 256);
                if (cls.ToString().Contains("Tauri") || title.ToString().Contains("DEBLOATER")) {
                    found = hWnd;
                    return false;
                }
            }
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@ -ErrorAction SilentlyContinue

$hwnd = [WinApiHelper]::FindTauriHwnd($ProcessId)
if ($hwnd -eq [IntPtr]::Zero) {
    Start-Sleep -Milliseconds 800
    $hwnd = [WinApiHelper]::FindTauriHwnd($ProcessId)
}

if ($Action -eq "snap") {
    [WinApiHelper]::ShowWindow($hwnd, 9) | Out-Null
    Start-Sleep -Milliseconds 200
    # Snap to left half of primary monitor (960 x 1040)
    [WinApiHelper]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, 960, 1040, 0x0040) | Out-Null
} elseif ($Action -eq "max") {
    [WinApiHelper]::ShowWindow($hwnd, 3) | Out-Null
} elseif ($Action -eq "restore") {
    [WinApiHelper]::ShowWindow($hwnd, 9) | Out-Null
} elseif ($Action -eq "small") {
    [WinApiHelper]::ShowWindow($hwnd, 9) | Out-Null
    Start-Sleep -Milliseconds 200
    [WinApiHelper]::SetWindowPos($hwnd, [IntPtr]::Zero, 100, 100, 980, 640, 0x0040) | Out-Null
}

Start-Sleep -Milliseconds 700

$wRect = New-Object WinApiHelper+RECT
$cRect = New-Object WinApiHelper+RECT
[WinApiHelper]::GetWindowRect($hwnd, [ref]$wRect) | Out-Null
[WinApiHelper]::GetClientRect($hwnd, [ref]$cRect) | Out-Null
$dpi = [WinApiHelper]::GetDpiForWindow($hwnd)

$result = [PSCustomObject]@{
    Hwnd = ("0x{0:X}" -f $hwnd.ToInt64())
    Dpi = $dpi
    ScalePercent = [math]::Round(($dpi / 96) * 100)
    WindowWidth = ($wRect.Right - $wRect.Left)
    WindowHeight = ($wRect.Bottom - $wRect.Top)
    ClientWidth = ($cRect.Right - $cRect.Left)
    ClientHeight = ($cRect.Bottom - $cRect.Top)
    WindowLeft = $wRect.Left
    WindowTop = $wRect.Top
}

$result | ConvertTo-Json -Compress
