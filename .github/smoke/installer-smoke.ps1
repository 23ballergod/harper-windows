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
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }

    // Pixels within `tolerance` of a color inside a box of a 32-bit BGRA screenshot:
    // { count, x and y of the first one (top to bottom), sum of x, sum of y }.
    public static long[] Scan(byte[] d, int stride, int w, int h, int x0, int y0, int x1, int y1, int r, int g, int b, int tolerance) {
        long count = 0, fx = -1, fy = -1, sx = 0, sy = 0;
        for (int y = Math.Max(0, y0); y < Math.Min(h, y1); y++) {
            for (int x = Math.Max(0, x0); x < Math.Min(w, x1); x++) {
                int i = y * stride + x * 4;
                if (Math.Abs(d[i + 2] - r) <= tolerance && Math.Abs(d[i + 1] - g) <= tolerance && Math.Abs(d[i] - b) <= tolerance) {
                    if (count == 0) { fx = x; fy = y; }
                    count++; sx += x; sy += y;
                }
            }
        }
        return new long[] { count, fx, fy, sx, sy };
    }

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

# --- Type mistakes into Notepad and Chrome ---------------------------------------------------
# Each window is photographed with the app running and again after it is stopped. Underlines are
# the colored pixels that differ between the two, so text, ClearType edges and the apps' own
# decorations cancel out.
$sentence = 'This is an test. Their going to the store. She go to school every day.'
$shell = New-Object -ComObject WScript.Shell
$tests = New-Object System.Collections.Generic.List[object]

function ForegroundRect {
    $r = New-Object Win+RECT
    [void][Win]::GetWindowRect([Win]::GetForegroundWindow(), [ref]$r)
    $r
}

function TypeInto([string] $label, [string] $title, [string] $shot, [bool] $required) {
    Start-Sleep 1
    [void]$shell.AppActivate($title)
    Start-Sleep 1
    $shell.SendKeys($sentence)
    Start-Sleep 15
    $rect = ForegroundRect
    $dark = Shot $shot
    if ($dark -gt 0.5 -and $darkBefore -lt 0.5) { $problems.Add("the screen was black while typing in $label") }
    if ($app.HasExited) { $problems.Add("the app quit while typing in $label") }
    $tests.Add([pscustomobject]@{ Label = $label; Title = $title; Shot = $shot; Rect = $rect; Required = $required })
}

function UnderlinePixels([string] $with, [string] $without, $rect) {
    $a = [System.Drawing.Bitmap]::FromFile((Join-Path $Out "$with.png"))
    $b = [System.Drawing.Bitmap]::FromFile((Join-Path $Out "$without.png"))
    $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $count = 0
    for ($y = [Math]::Max($rect.T, $vs.Top); $y -lt [Math]::Min($rect.B, $vs.Bottom); $y++) {
        for ($x = [Math]::Max($rect.L, $vs.Left); $x -lt [Math]::Min($rect.R, $vs.Right); $x++) {
            $p = $a.GetPixel($x - $vs.Left, $y - $vs.Top)
            $q = $b.GetPixel($x - $vs.Left, $y - $vs.Top)
            $max = [Math]::Max($p.R, [Math]::Max($p.G, $p.B))
            $min = [Math]::Min($p.R, [Math]::Min($p.G, $p.B))
            $diff = [Math]::Abs($p.R - $q.R) + [Math]::Abs($p.G - $q.G) + [Math]::Abs($p.B - $q.B)
            if ($max - $min -gt 80 -and $diff -gt 120) { $count++ }
        }
    }
    $a.Dispose(); $b.Dispose()
    $count
}

# The screen as raw 32-bit BGRA pixels, for [Win]::Scan.
function ScreenPixels {
    $vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
    $bmp = New-Object System.Drawing.Bitmap $vs.Width, $vs.Height, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($vs.Left, $vs.Top, 0, 0, $bmp.Size)
    $g.Dispose()
    $data = $bmp.LockBits((New-Object System.Drawing.Rectangle 0, 0, $bmp.Width, $bmp.Height), 'ReadOnly', 'Format32bppArgb')
    $bytes = New-Object byte[] ($data.Stride * $bmp.Height)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
    $stride = $data.Stride
    $bmp.UnlockBits($data)
    $bmp.Dispose()
    [pscustomobject]@{ Bytes = $bytes; Stride = $stride; Width = $vs.Width; Height = $vs.Height; Left = $vs.Left; Top = $vs.Top }
}

# Pixels of one color inside a box given in screen coordinates.
function CountColor($shot, [int] $l, [int] $t, [int] $r, [int] $b, $color, [int] $tolerance = 10) {
    [Win]::Scan($shot.Bytes, $shot.Stride, $shot.Width, $shot.Height, $l - $shot.Left, $t - $shot.Top, $r - $shot.Left, $b - $shot.Top, $color[0], $color[1], $color[2], $tolerance)
}

function ForegroundTitle {
    $title = New-Object System.Text.StringBuilder 256
    [void][Win]::GetWindowText([Win]::GetForegroundWindow(), $title, 256)
    $title.ToString()
}

# Underline colors, as in lint_kind_color.rs. Underlines are drawn fully opaque in these colors, and
# the suggestion card's main button is filled with the same color.
$lintColors = @(
    @(0x22, 0x8B, 0x22), @(0x8B, 0x45, 0x13), @(0x54, 0x0D, 0x6E), @(0xFF, 0x8C, 0x00), @(0x0E, 0xAD, 0x69),
    @(0x7D, 0x3C, 0x98), @(0x9B, 0x59, 0xB6), @(0xC7, 0x15, 0x85), @(0x3B, 0xCE, 0xAC), @(0x00, 0x8B, 0x8B),
    @(0xD4, 0x85, 0x0F), @(0x2E, 0x8B, 0x57), @(0x46, 0x82, 0xB4), @(0xC0, 0x61, 0xCB), @(0x00, 0xA6, 0x7C),
    @(0xEE, 0x42, 0x66), @(0xFF, 0xD2, 0x3F), @(0xFF, 0x6B, 0x35), @(0x1E, 0x90, 0xFF), @(0x4D, 0x4D, 0xFF)
)

# Rests the mouse on the first underline in a window the way a user would, checks that a solid
# suggestion card opens without taking focus from the app, clicks the card's main suggestion and
# checks that the text changed.
function SuggestionCardTest([string] $label, [string] $title) {
    [void]$shell.AppActivate($title)
    Start-Sleep 2
    $win = ForegroundRect
    $shot = ScreenPixels
    $underline = $null
    foreach ($color in $lintColors) {
        $s = CountColor $shot $win.L $win.T $win.R $win.B $color
        if ($s[0] -ge 6 -and (-not $underline -or $s[2] + $shot.Top -lt $underline.Y)) {
            $underline = [pscustomobject]@{ Color = $color; X = [int]$s[1] + $shot.Left; Y = [int]$s[2] + $shot.Top; Pixels = $s[0] }
        }
    }
    if (-not $underline) {
        Log "- No underline found to hover over"
        $problems.Add("no underline in $label to open a suggestion card from")
        return
    }
    $hex = ($underline.Color | ForEach-Object { '{0:X2}' -f $_ }) -join ''
    Log "- First underline: #$hex at $($underline.X),$($underline.Y)"

    # Card area: below the underline, as wide as the card (popup_rect_for_lint in render_state.rs).
    $cardL = $underline.X - 40; $cardR = $underline.X + 480; $cardT = $underline.Y + 4; $cardB = $underline.Y + 240
    $before = (CountColor $shot $cardL $cardT $cardR $cardB $underline.Color)[0]

    [void][Win]::SetCursorPos($underline.X + 4, $underline.Y - 6)
    Start-Sleep 2
    [void](Shot '6-chrome-card')
    $shot = ScreenPixels
    $button = CountColor $shot $cardL $cardT $cardR $cardB $underline.Color
    $added = $button[0] - $before
    Log "- After resting the mouse on it for 2 s: $added solid #$hex pixels below it (the card's main button is about 3000)"
    $focus = ForegroundTitle
    Log "- Window in front: '$focus'"
    if ($added -lt 800) {
        $problems.Add("resting the mouse on an underline in $label didn't open a solid, readable suggestion card")
        [void][Win]::SetCursorPos(0, 0)
        return
    }

    $cx = [int]($button[3] / $button[0]) + $shot.Left
    $cy = [int]($button[4] / $button[0]) + $shot.Top
    [void][Win]::SetCursorPos($cx, $cy)
    Start-Sleep -Milliseconds 300
    [Win]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [Win]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep 2
    $focus = ForegroundTitle
    Log "- Clicked the main suggestion at $cx,$cy; window in front: '$focus'"
    if ($focus -notlike "*$title*") { $problems.Add("clicking the suggestion card took focus away from $label (front window: '$focus')") }
    [void][Win]::SetCursorPos(0, 0)

    # Copy the text box's contents; the clipboard can be slow or busy on the runner, so try a few times.
    $after = $null
    for ($attempt = 1; $attempt -le 4 -and -not $after; $attempt++) {
        Set-Clipboard -Value ' ' -ErrorAction SilentlyContinue
        [void]$shell.AppActivate($title)
        Start-Sleep 1
        $shell.SendKeys('^a')
        Start-Sleep -Milliseconds 300
        $shell.SendKeys('^c')
        Start-Sleep 1
        $after = (Get-Clipboard -Raw -ErrorAction SilentlyContinue)
        if ($after) { $after = $after.Trim() }
        $shell.SendKeys('{END}')
    }
    Log "- Text after the click: '$after'"
    if (-not $after -or $after.Trim() -eq $sentence) { $problems.Add("clicking the main suggestion didn't change the text in $label") }
}

Log ""
Log "## Typing into Notepad"
$np = Start-Process notepad -PassThru
Start-Sleep 4
TypeInto 'Notepad' 'Notepad' '3-notepad-typed' $true
ShowWindows (Pids $dir) 'with Notepad focused'

Log ""
Log "## Typing into Chrome"
$chrome = @("$env:ProgramFiles\Google\Chrome\Application\chrome.exe", "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if ($chrome) {
    $html = "<title>Shah test</title><textarea autofocus style='font:22px sans-serif;width:90%;height:240px'></textarea>"
    $url = 'data:text/html,' + [uri]::EscapeDataString($html)
    $chromeProfile = Join-Path $env:TEMP 'shah-chrome-profile'
    Start-Process $chrome -ArgumentList @("--user-data-dir=$chromeProfile", '--no-first-run', '--no-default-browser-check', '--new-window', '--window-size=900,600', '--window-position=40,40', $url) | Out-Null
    Start-Sleep 10
    TypeInto 'Chrome' 'Shah test' '5-chrome-typed' $true
    Log ""
    Log "## Suggestion card in Chrome"
    SuggestionCardTest 'Chrome' 'Shah test'

    # Clicking the page outside the text box must clear its underlines straight away.
    Log ""
    Log "## Clicking out of the Chrome text box"
    [void]$shell.AppActivate('Shah test')
    Start-Sleep 1
    $win = ForegroundRect
    [void][Win]::SetCursorPos($win.L + 400, $win.B - 60)
    [Win]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [Win]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    [void][Win]::SetCursorPos(0, 0)
    Start-Sleep 3
    $shot = ScreenPixels
    $left = 0
    foreach ($color in $lintColors) { $left += (CountColor $shot $win.L $win.T $win.R $win.B $color)[0] }
    Log "- Underline-colored pixels 3 s after clicking the page: $left"
    if ($left -gt 30) { $problems.Add("underlines stayed after clicking out of the Chrome text box ($left pixels)") }

    # A page with mistakes but no text box in focus, like the Claude app's sidebar and buttons, must
    # not be checked: only text the user is typing gets underlines.
    Log ""
    Log "## Web page text outside a text box"
    $page = "<title>Shah page</title><body style='font:22px sans-serif'><p>This is an test. Their going to the store.</p><button>Their going to the store</button><p>She go to school every day.</p></body>"
    Start-Process $chrome -ArgumentList @("--user-data-dir=$chromeProfile", '--new-window', '--window-size=900,600', '--window-position=60,60', ('data:text/html,' + [uri]::EscapeDataString($page))) | Out-Null
    Start-Sleep 6
    [void]$shell.AppActivate('Shah page')
    Start-Sleep 10
    $win = ForegroundRect
    $shot = ScreenPixels
    [void](Shot '7-chrome-page')
    $marked = 0
    foreach ($color in $lintColors) { $marked += (CountColor $shot $win.L $win.T $win.R $win.B $color)[0] }
    Log "- Underline-colored pixels on the page: $marked (front window: '$(ForegroundTitle)')"
    if ($marked -gt 30) { $problems.Add("text on a web page outside any text box got underlined ($marked pixels)") }
    $overlays = [Win]::Describe((Pids $dir)) | Where-Object { $_ -match 'topmost=True' }
    Log "- Overlay windows showing with nothing to check: $(@($overlays).Count)"
    if (@($overlays).Count -gt 0) { $problems.Add("the overlay stayed on screen with nothing to check (it should hide, e.g. over games)") }
} else {
    Log "- Chrome is not installed on this machine"
}

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

Log ""
Log "## Underlines"
foreach ($t in $tests) {
    [void]$shell.AppActivate($t.Title)
    Start-Sleep 2
    [void](Shot "$($t.Shot)-without-app")
    $pixels = UnderlinePixels $t.Shot "$($t.Shot)-without-app" $t.Rect
    Log "- $($t.Label): $pixels colored pixels drawn by the app over the window"
    if ($pixels -lt 30) {
        if ($t.Required) { $problems.Add("no underlines appeared under the mistakes typed into $($t.Label)") }
    }
}
Get-Process -Name notepad, chrome -ErrorAction SilentlyContinue | Stop-Process -Force

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
