# Installs, launches and uninstalls the Windows installer the way a user would, and records what
# happens: file properties, what gets installed, which windows appear (with screenshots), whether
# the app keeps running, and what is left behind after uninstalling.
#
# Usage: installer-smoke.ps1 -Installer <setup.exe> -Out <results folder>

param(
    [Parameter(Mandatory)] [string] $Installer,
    [Parameter(Mandatory)] [string] $Out
)

$ErrorActionPreference = 'Continue'
$problems = New-Object System.Collections.Generic.List[string]
New-Item -ItemType Directory -Force $Out | Out-Null
$report = Join-Path $Out 'report.md'
Set-Content -Path $report -Value ''

function Log([string] $s) {
    Add-Content -Path $report -Value $s
    Write-Host $s
}

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public static class Win {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll", EntryPoint = "GetWindowLongPtrW")] public static extern IntPtr GetWindowLongPtr(IntPtr h, int i);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }

    // Visible top-level windows owned by any of the given processes.
    public static List<string> Describe(HashSet<uint> pids) {
        var list = new List<string>();
        EnumWindows((h, l) => {
            uint pid;
            GetWindowThreadProcessId(h, out pid);
            if (!pids.Contains(pid) || !IsWindowVisible(h)) return true;
            var title = new StringBuilder(256); GetWindowText(h, title, 256);
            var cls = new StringBuilder(256); GetClassName(h, cls, 256);
            RECT r; GetWindowRect(h, out r);
            long ex = GetWindowLongPtr(h, -20).ToInt64();
            bool layered = (ex & 0x80000) != 0, clickThrough = (ex & 0x20) != 0, topmost = (ex & 0x8) != 0;
            list.Add(String.Format(
                "pid {0}: '{1}' ({2}) at {3},{4} size {5}x{6}; topmost={7} layered={8} click-through={9} exstyle=0x{10:X}",
                pid, title, cls, r.L, r.T, r.R - r.L, r.B - r.T, topmost, layered, clickThrough, ex));
            return true;
        }, IntPtr.Zero);
        return list;
    }
}
"@
[Win]::SetProcessDPIAware() | Out-Null

function Shot([string] $name) {
    $bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
    } catch {
        Log "- Screenshot ``$name`` failed: $_"
        $g.Dispose(); $bmp.Dispose(); return -1
    }
    $g.Dispose()
    $bmp.Save((Join-Path $Out "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $n = 0; $dark = 0; $sum = 0.0
    for ($y = 0; $y -lt $bmp.Height; $y += 8) {
        for ($x = 0; $x -lt $bmp.Width; $x += 8) {
            $p = $bmp.GetPixel($x, $y)
            $l = 0.299 * $p.R + 0.587 * $p.G + 0.114 * $p.B
            $sum += $l; $n++
            if ($l -lt 10) { $dark++ }
        }
    }
    $bmp.Dispose()
    Log ("- Screenshot ``{0}.png``: {1}x{2}, average brightness {3:N0}/255, near-black {4:P0} of the screen" -f $name, $bounds.Width, $bounds.Height, ($sum / $n), ($dark / $n))
    $dark / $n
}

function Pids([string] $dir) {
    $set = New-Object 'System.Collections.Generic.HashSet[uint32]'
    Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($dir, 'OrdinalIgnoreCase') } |
        ForEach-Object { [void]$set.Add([uint32]$_.ProcessId) }
    , $set
}

function ShowWindows([System.Collections.Generic.HashSet[uint32]] $pids, [string] $label) {
    $wins = [Win]::Describe($pids)
    Log "- Visible windows ($label): $($wins.Count)"
    foreach ($w in $wins) { Log "  - $w" }
}

function FolderSize([string] $path) {
    if (-not (Test-Path $path)) { return 'absent' }
    $bytes = (Get-ChildItem $path -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
    '{0:N1} MB' -f ($bytes / 1MB)
}

function UninstallEntries {
    Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall', 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue |
        Get-ItemProperty | Where-Object { $_.DisplayName -match 'Harper|Shah' }
}

function Shortcuts {
    $places = @("$env:APPDATA\Microsoft\Windows\Start Menu\Programs", "$env:PUBLIC\Desktop", [Environment]::GetFolderPath('Desktop'))
    Get-ChildItem $places -Recurse -Filter *.lnk -ErrorAction SilentlyContinue | Where-Object { $_.Name -match 'Harper|Shah' }
}

function DataFolders([string] $id) {
    foreach ($p in @("$env:APPDATA\harper-windows", "$env:LOCALAPPDATA\harper-windows", "$env:APPDATA\$id", "$env:LOCALAPPDATA\$id")) {
        Log "  - ``$p``: $(FolderSize $p)"
    }
}

# --- The installer file ---------------------------------------------------------------------
$os = Get-CimInstance Win32_OperatingSystem
Log "# Installer smoke test"
Log ""
Log "Machine: $($os.Caption) $($os.Version); GPU: $((Get-CimInstance Win32_VideoController | ForEach-Object Name) -join ', ')"
Log ""
Log "## Installer file"
$inst = Get-Item $Installer
$vi = $inst.VersionInfo
Log "- File: ``$($inst.Name)``, $('{0:N1}' -f ($inst.Length / 1MB)) MB"
Log "- Properties: product '$($vi.ProductName)', description '$($vi.FileDescription)', company '$($vi.CompanyName)', copyright '$($vi.LegalCopyright)', version '$($vi.ProductVersion)'"
Log "- Code signature: $((Get-AuthenticodeSignature $inst.FullName).Status)"
$darkBefore = Shot '0-desktop-before'

# --- Installer window (what a user sees first) ------------------------------------------------
Log ""
Log "## Installer window"
$p = Start-Process $inst.FullName -PassThru
Start-Sleep 8
$set = New-Object 'System.Collections.Generic.HashSet[uint32]'; [void]$set.Add([uint32]$p.Id)
ShowWindows $set 'installer'
[void](Shot '1-installer-window')
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Start-Sleep 2

# --- Silent install ---------------------------------------------------------------------------
Log ""
Log "## Install"
$sw = [Diagnostics.Stopwatch]::StartNew()
$p = Start-Process $inst.FullName -ArgumentList '/S' -PassThru -Wait
Log "- Silent install exit code $($p.ExitCode) after $('{0:N0}' -f $sw.Elapsed.TotalSeconds) s"
$entry = UninstallEntries | Select-Object -First 1
if (-not $entry) {
    Log "- FAIL: no entry in Installed apps after installing"
    exit 1
}
$uninstaller = ($entry.UninstallString -replace '"', '').Trim()
$dir = Split-Path $uninstaller
Log "- Installed apps entry: '$($entry.DisplayName)' version $($entry.DisplayVersion), publisher '$($entry.Publisher)', size $('{0:N0}' -f ($entry.EstimatedSize / 1024)) MB"
Log "- Install folder: ``$dir``"
Get-ChildItem $dir -Recurse -File | ForEach-Object { Log ("  - {0} ({1:N1} MB)" -f $_.FullName.Substring($dir.Length + 1), ($_.Length / 1MB)) }
foreach ($s in Shortcuts) { Log "- Shortcut: ``$($s.FullName)``" }
$exe = Get-ChildItem $dir -Filter *.exe | Where-Object { $_.Name -notmatch '^uninstall' } | Select-Object -First 1
$avi = $exe.VersionInfo
Log "- App file properties: product '$($avi.ProductName)', description '$($avi.FileDescription)', company '$($avi.CompanyName)', copyright '$($avi.LegalCopyright)', version '$($avi.ProductVersion)'"

# --- Launch -----------------------------------------------------------------------------------
Log ""
Log "## Launch"
$env:RUST_LOG = 'info'
$env:RUST_BACKTRACE = '1'
$app = Start-Process $exe.FullName -PassThru -RedirectStandardError (Join-Path $Out 'app-stderr.txt') -RedirectStandardOutput (Join-Path $Out 'app-stdout.txt')
Start-Sleep 30
Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($dir, 'OrdinalIgnoreCase') } |
    ForEach-Object { Log "- Running: pid $($_.ProcessId), $($_.CommandLine), $('{0:N0}' -f ($_.WorkingSetSize / 1MB)) MB RAM" }
Log "- Main app still running after 30 s: $(-not $app.HasExited)$(if ($app.HasExited) { " (exit code $($app.ExitCode))" })"
ShowWindows (Pids $dir) 'after launch'
$darkAfter = Shot '2-after-launch'
if ($darkAfter -gt 0.5 -and $darkBefore -lt 0.5) { $problems.Add("the screen went black after launching the app ($('{0:P0}' -f $darkAfter) near-black)") }
if ($app.HasExited) { $problems.Add("the app quit within 30 seconds of launching (exit code $($app.ExitCode))") }

# --- Type a mistake into Notepad --------------------------------------------------------------
Log ""
Log "## Typing into Notepad"
$np = Start-Process notepad -PassThru
Start-Sleep 4
$shell = New-Object -ComObject WScript.Shell
if (-not $shell.AppActivate($np.Id)) { [void]$shell.AppActivate('Notepad') }
Start-Sleep 1
$shell.SendKeys('This is an test. Their going to the store. She go to school every day.')
Start-Sleep 15
Log "- Main app still running: $(-not $app.HasExited)"
ShowWindows (Pids $dir) 'with Notepad focused'
$fg = [Win]::GetForegroundWindow()
$npRect = New-Object Win+RECT
[void][Win]::GetWindowRect($fg, [ref]$npRect)
$darkTyped = Shot '3-notepad-typed'
if ($darkTyped -gt 0.5 -and $darkBefore -lt 0.5) { $problems.Add("the screen was black while typing in Notepad") }
if ($app.HasExited) { $problems.Add("the app quit while typing in Notepad") }

# Underlines are horizontal runs of colored pixels under Notepad's black-on-white text. ClearType
# also tints the edges of letters, but only a pixel or two wide, so count runs of 8 or more.
# Skip the title bar and menu (top 50 px) and the scroll bar.
$img = [System.Drawing.Bitmap]::FromFile((Join-Path $Out '3-notepad-typed.png'))
$vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
$runs = 0
for ($y = $npRect.T + 50; $y -lt [Math]::Min($npRect.T + 160, $npRect.B); $y++) {
    $run = 0
    for ($x = $npRect.L + 5; $x -lt $npRect.R - 30; $x++) {
        $px = $img.GetPixel($x - $vs.Left, $y - $vs.Top)
        $max = [Math]::Max($px.R, [Math]::Max($px.G, $px.B))
        $min = [Math]::Min($px.R, [Math]::Min($px.G, $px.B))
        if ($max - $min -gt 80) { $run++ } else { if ($run -ge 8) { $runs++ }; $run = 0 }
    }
    if ($run -ge 8) { $runs++ }
}
$img.Dispose()
Log "- Foreground window at $($npRect.L),$($npRect.T)-$($npRect.R),$($npRect.B); underline-like colored runs in its text area: $runs"
if ($runs -lt 2) { $problems.Add("no underlines appeared under the mistakes typed into Notepad") }
Get-Process -Name notepad -ErrorAction SilentlyContinue | Stop-Process -Force

# --- Leftover state before uninstalling --------------------------------------------------------
Log ""
Log "## State while installed"
$run = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
$runNames = $run.PSObject.Properties.Name | Where-Object { $_ -match 'Harper|Shah' }
Log "- Starts with Windows (Run key): $(if ($runNames) { $runNames -join ', ' } else { 'no' })"
$id = 'com.shahrewriter.app'
Log "- Data folders:"
DataFolders $id

(Pids $dir) | ForEach-Object { Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }
Start-Sleep 3

$logs = "$env:LOCALAPPDATA\$id\logs"
if (Test-Path $logs) {
    Copy-Item $logs (Join-Path $Out 'logs') -Recurse -Force
    foreach ($f in Get-ChildItem $logs -Filter *.log) {
        Log ""
        Log "### $($f.Name) (last 40 lines)"
        Log '```'
        Get-Content $f.FullName -Tail 40 | ForEach-Object { Log $_ }
        Log '```'
    }
} else {
    Log "- No log folder at ``$logs``"
}

# Stand-in for a downloaded AI model, to see whether uninstalling removes it.
$models = "$env:LOCALAPPDATA\$id\models"
New-Item -ItemType Directory -Force $models | Out-Null
fsutil file createnew (Join-Path $models 'stand-in-model.gguf') 104857600 | Out-Null

# --- Uninstaller window -----------------------------------------------------------------------
Log ""
Log "## Uninstall"
Start-Process $uninstaller | Out-Null
Start-Sleep 8
$set = New-Object 'System.Collections.Generic.HashSet[uint32]'
Get-Process | Where-Object { $_.Name -like 'Un_*' -or $_.Name -like 'Au_*' -or $_.Path -eq $uninstaller } | ForEach-Object { [void]$set.Add([uint32]$_.Id) }
ShowWindows $set 'uninstaller'
[void](Shot '4-uninstaller-window')
$set | ForEach-Object { Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue }
Start-Sleep 2

# --- Silent uninstall -------------------------------------------------------------------------
$p = Start-Process $uninstaller -ArgumentList '/S' -PassThru -Wait
for ($i = 0; $i -lt 90 -and (UninstallEntries); $i++) { Start-Sleep 1 }
Start-Sleep 3
Log "- Silent uninstall exit code $($p.ExitCode)"
Log "- Installed apps entry removed: $(-not (UninstallEntries))"
if (UninstallEntries) { $problems.Add("uninstalling left the Installed apps entry behind") }
if (Test-Path $dir) { $problems.Add("uninstalling left the install folder behind") }
Log "- Install folder: $(if (Test-Path $dir) { "still there: $((Get-ChildItem $dir -Recurse -Force | ForEach-Object Name) -join ', ')" } else { 'removed' })"
$left = Shortcuts
Log "- Shortcuts left: $(if ($left) { ($left | ForEach-Object FullName) -join ', ' } else { 'none' })"
$run = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
$runNames = $run.PSObject.Properties.Name | Where-Object { $_ -match 'Harper|Shah' }
Log "- Run key left: $(if ($runNames) { $runNames -join ', ' } else { 'none' })"
Log "- Data folders left (the stand-in model is 100 MB):"
DataFolders $id

Log ""
Log "## Verdict"
if ($problems.Count -eq 0) {
    Log "PASS: installs, launches without blacking out the screen, keeps running, and uninstalls."
} else {
    foreach ($p in $problems) { Log "- FAIL: $p" }
}
exit [int]($problems.Count -gt 0)
