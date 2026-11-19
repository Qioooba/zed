# Real Zed GUI click regression with forced foreground focus.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public class WinFocus {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h,int n);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  public static void ForceFocus(IntPtr h) {
    uint targetPid; uint targetThread = GetWindowThreadProcessId(h, out targetPid);
    uint current = GetCurrentThreadId();
    AttachThreadInput(current, targetThread, true);
    ShowWindow(h, 9);
    SetForegroundWindow(h);
    AttachThreadInput(current, targetThread, false);
  }
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,IntPtr e);
  public static void Click(int x,int y){ SetCursorPos(x,y); mouse_event(0x0002,0,0,0,IntPtr.Zero); mouse_event(0x0004,0,0,0,IntPtr.Zero); }
}
'@

$ZED = 'E:\github\zed\target\debug\zed.exe'
$USERDATA = 'D:\GPU-Tools\Temp\opencode\zed-gui-test\userdata'
$PROJECT = 'D:\GPU-Tools\Temp\opencode\zed-gui-test\legacy-sample'
$SHOTS = 'E:\github\zed\kairo\ui-prototype\screenshots'

# Minimize every other visible window.
Get-Process | Where-Object { $_.MainWindowHandle -ne 0 } | ForEach-Object {
  [WinFocus]::ShowWindow($_.MainWindowHandle, 6) | Out-Null
}

$proc = Start-Process -FilePath $ZED -ArgumentList @('--user-data-dir', $USERDATA, $PROJECT) -PassThru
Start-Sleep -Seconds 25
$hwnd = $proc.MainWindowHandle
[WinFocus]::ForceFocus($hwnd)
Start-Sleep -Seconds 2

function Shot([string]$name) {
  $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
  $bmp.Save((Join-Path $SHOTS $name), [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $bmp.Dispose()
  Write-Output "shot=$name"
}

Shot 'zed-click-01-start.png'

# Click into editor area, then open palette with real keys.
[WinFocus]::Click(600, 300)
Start-Sleep -Milliseconds 400
[System.Windows.Forms.SendKeys]::SendWait('{F1}')
Start-Sleep -Seconds 2
[System.Windows.Forms.SendKeys]::SendWait('kairo')
Start-Sleep -Seconds 2
Shot 'zed-click-02-palette.png'
[System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
Start-Sleep -Seconds 3
Shot 'zed-click-03-after-command.png'

Write-Output "zed pid=$($proc.Id) running=$(-not $proc.HasExited)"
