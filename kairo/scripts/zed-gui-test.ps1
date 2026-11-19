# Launch the REAL zed.exe built from this branch on a REAL legacy project,
# drive it with real keystrokes, and capture the real screen.
$ErrorActionPreference = 'Continue'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$ZED = 'E:\github\zed\target\debug\zed.exe'
$ROOT = 'D:\GPU-Tools\Temp\opencode\zed-gui-test'
$USERDATA = Join-Path $ROOT 'userdata'
$PROJECT = Join-Path $ROOT 'legacy-sample'
$SHOTS = 'E:\github\zed\kairo\ui-prototype\screenshots'

Remove-Item $ROOT -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $USERDATA | Out-Null
New-Item -ItemType Directory -Force $SHOTS | Out-Null
Copy-Item 'G:\spaces\kairo-ide\legacy-sample' $PROJECT -Recurse -Force
Remove-Item (Join-Path $PROJECT '.kairo') -Recurse -Force -ErrorAction SilentlyContinue

# Isolated Zed settings pointing at the real toolchain on this machine.
$settings = @{
    kairo = @{
        agent_path = 'G:\spaces\kairo-ide\runtime-agent\kairo-runtime.exe'
        bundled_dir = 'G:\spaces\kairo-ide\bundled'
        compiler_java_home = 'D:\kairo\tmp\qa-20260909\java\zulu6.22.0.3-jdk6.0.119-win_x64'
        tomcat_java_home = 'D:\kairo\tmp\qa-20260909\java\zulu6.22.0.3-jdk6.0.119-win_x64'
        source_level = '1.6'
        target_level = '1.6'
        encoding = 'gbk'
        auto_start_agent = $true
    }
} | ConvertTo-Json -Depth 8
$configDir = Join-Path $USERDATA 'config'
New-Item -ItemType Directory -Force $configDir | Out-Null
Set-Content (Join-Path $configDir 'settings.json') $settings -Encoding UTF8

function Capture([string]$name) {
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $path = Join-Path $SHOTS $name
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    Write-Output "screenshot=$path"
}

function Keys([string]$keys) {
    [System.Windows.Forms.SendKeys]::SendWait($keys)
    Start-Sleep -Seconds 2
}

Write-Output "launching REAL zed.exe: $ZED"
$proc = Start-Process -FilePath $ZED -ArgumentList @('--user-data-dir', $USERDATA, $PROJECT) -PassThru
Write-Output "zed pid=$($proc.Id)"
Start-Sleep -Seconds 25
Capture 'zed-real-01-startup.png'

# Toggle the Kairo panel with the real keybinding.
Keys '^%{K}'
Capture 'zed-real-02-panel.png'

# Open command palette and search for Kairo commands.
Keys '{F1}'
Keys 'kairo'
Capture 'zed-real-03-palette.png'
Keys '{ESC}'

# Trigger Scan Project via command palette (bootstrap).
Keys '{F1}'
Keys 'kairo::Scan Project'
Keys '{ENTER}'
Start-Sleep -Seconds 12
Capture 'zed-real-04-scan.png'

Write-Output "zed running=$(-not $proc.HasExited)"
Write-Output 'NOTE: keep the process for manual inspection; stop with Stop-Process when done.'
