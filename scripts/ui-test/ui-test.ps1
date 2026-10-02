# End-to-end user-story tests for Fancy Window. Drives the real desktop (mouse,
# keyboard, hotkeys) on the primary monitor, using its own Fancy Window instance
# (--profile uitest) and throwaway "FW Test N" forms.
#
# Safety: every click, drag and key press first checks that the window under the
# cursor (or in the foreground) belongs to the test instance or a test form, and
# refuses otherwise. Nothing is ever sent to the user's other windows.
#
#   .\ui-test.ps1 -Exe ...\fancy-window.exe -Out <dir> [-Only S01,S02] [-HotkeysFrom settings.json]
param(
    [Parameter(Mandatory)] [string]$Exe,
    [Parameter(Mandatory)] [string]$Out,
    [string[]]$Only = @(),
    # A settings.json whose "hotkeys" the test instance should use, e.g. your everyday one.
    [string]$HotkeysFrom = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -Path "$PSScriptRoot\UI.cs" -ReferencedAssemblies System.Drawing
New-Item -ItemType Directory -Force $Out | Out-Null

$Profile_ = 'uitest'
$Title = "Fancy Window [$Profile_]"
$TestForms = Join-Path $PSScriptRoot '..\test-windows.ps1'
$ExeDir = Split-Path $Exe
$SettingsFile = Join-Path $ExeDir "settings-$Profile_.json"
$CrashLog = Join-Path $ExeDir "crash-$Profile_.log"
$Tol = 3

$VK = @{ Ctrl = [uint16]0x11; Alt = [uint16]0x12; Shift = [uint16]0x10; Win = [uint16]0x5B; Esc = [uint16]0x1B; Enter = [uint16]0x0D; Down = [uint16]0x28 }

$script:Results = [System.Collections.Generic.List[object]]::new()
$script:Story = ''
$script:Pids = [System.Collections.Generic.HashSet[uint32]]::new()

function Log($text) { Write-Host $text; Add-Content -Path "$Out\log.txt" -Value $text }
function Check([string]$what, [bool]$ok, [string]$detail = '') {
    $script:Results.Add([pscustomobject]@{ Story = $script:Story; Check = $what; Ok = $ok; Detail = $detail })
    Log ("  [{0}] {1}{2}" -f ($(if ($ok) { 'PASS' } else { 'FAIL' })), $what, $(if ($detail) { "  ($detail)" } else { '' }))
}
function Near([int[]]$a, [double[]]$b) {
    for ($i = 0; $i -lt 4; $i++) { if ([math]::Abs($a[$i] - $b[$i]) -gt $Tol) { return $false } }
    return $true
}
function Fmt($r) { ($r | ForEach-Object { [math]::Round($_) }) -join ',' }
function Wait-Until([scriptblock]$cond, [int]$ms = 3000) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.ElapsedMilliseconds -lt $ms) { if (& $cond) { return $true }; Start-Sleep -Milliseconds 50 }
    return [bool](& $cond)
}
function Shot($name) {
    $c = [UI]::Rect($script:FW)
    [UI]::Shot($c[0] - 10, $c[1] - 10, $c[2] + 20, $c[3] + 20, "$Out\$name.png")
}

# ---- hotkeys: defaults, overridden by the user's saved chords -------------------

$Chords = @{ marginUp = 'Alt+Win+OemPlus'; marginDown = 'Alt+Win+OemMinus'; bringToFront = 'Alt+Win+PageUp'; sendToBack = 'Alt+Win+PageDown'
             resetLayout = 'Alt+Win+Home'; cycleNext = 'Alt+Win+OemCloseBrackets'; cyclePrevious = 'Alt+Win+OemOpenBrackets' }
$UserHotkeys = @{}
if ($HotkeysFrom -and (Test-Path $HotkeysFrom)) {
    $saved = (Get-Content -Raw $HotkeysFrom | ConvertFrom-Json).hotkeys
    if ($saved) { foreach ($p in $saved.PSObject.Properties) { $Chords[$p.Name] = $p.Value; $UserHotkeys[$p.Name] = $p.Value } }
}
$KeyCodes = @{ OemPlus = 0xBB; OemMinus = 0xBD; OemOpenBrackets = 0xDB; OemCloseBrackets = 0xDD; OemComma = 0xBC; OemPeriod = 0xBE; PageUp = 0x21; PageDown = 0x22; Home = 0x24 }
function Parse-Chord([string]$text) {
    $parts = $text -split '\+'
    $mods = foreach ($m in $parts[0..($parts.Count - 2)]) { switch ($m) { 'Ctrl' { $VK.Ctrl } 'Alt' { $VK.Alt } 'Shift' { $VK.Shift } 'Win' { $VK.Win } } }
    $k = $parts[-1]
    $code = if ($KeyCodes.ContainsKey($k)) { $KeyCodes[$k] } elseif ($k.Length -eq 1) { [int][char]$k.ToUpper() } else { throw "unknown key $k" }
    , @([uint16[]]$mods, [uint16]$code)
}

# ---- safety ---------------------------------------------------------------------

function OwnedPid([IntPtr]$h) { $h -ne [IntPtr]::Zero -and $script:Pids.Contains([UI]::Pid($h)) }
function OursAt([int]$x, [int]$y) { OwnedPid ([UI]::RootAt($x, $y)) }
function OursInFront { OwnedPid ([UI]::GetForegroundWindow()) }
function Guard([int]$x, [int]$y) {
    if (-not (OursAt $x $y)) { throw "refused: ($x,$y) is over '$([UI]::Title([UI]::RootAt($x, $y)))', not a test window" }
}
function SClick([int]$x, [int]$y) { Guard $x $y; [UI]::Click($x, $y) }
function SRightClick([int]$x, [int]$y) { Guard $x $y; [UI]::RightClick($x, $y) }
function SDrag([int]$x1, [int]$y1, [int]$x2, [int]$y2, [uint16[]]$hold = @(), [switch]$CheckEnd) {
    Guard $x1 $y1
    if ($CheckEnd) { Guard $x2 $y2 }
    [UI]::Drag($x1, $y1, $x2, $y2, $hold, 25, 12)
}
function SKey([uint16]$vk) {
    if (-not (OursInFront)) { throw "refused key: foreground is '$([UI]::Title([UI]::GetForegroundWindow()))'" }
    [UI]::KeyDown($vk); [UI]::KeyUp($vk); Start-Sleep -Milliseconds 60
}
function SModClick([uint16]$mod, [int]$x, [int]$y) {
    Guard $x $y
    [UI]::KeyDown($mod); Start-Sleep -Milliseconds 60
    try { [UI]::Click($x, $y) } finally { [UI]::KeyUp($mod) }
    Start-Sleep -Milliseconds 250
}

# Lifts the test instance (hosted windows come with it) and the parked forms above
# everything, then activates it by clicking the empty middle of its status bar.
function Front {
    if (-not ($script:FW -and [UI]::IsWindow($script:FW))) { return }
    foreach ($i in 1..4) { if ($T[$i] -and [UI]::IsWindow($T[$i]) -and -not (Hosted $T[$i])) { [UI]::Raise($T[$i]) } }
    [UI]::Raise($script:FW)
    Start-Sleep -Milliseconds 150
    $c = [UI]::Client($script:FW)
    SClick ($c[0] + [int]($c[2] / 2)) ($c[1] + $c[3] - 12)
    Start-Sleep -Milliseconds 150
}

# Hotkeys go to the Fancy Window that owns the foreground window, so only press one
# when that is the test instance (or a window it hosts).
function Hotkey([string]$command) {
    $fg = [UI]::GetForegroundWindow()
    if (-not ($fg -eq $script:FW -or [UI]::Owner($fg) -eq $script:FW)) { Front }
    $fg = [UI]::GetForegroundWindow()
    if (-not ($fg -eq $script:FW -or [UI]::Owner($fg) -eq $script:FW)) { throw "refused hotkey: foreground is '$([UI]::Title($fg))'" }
    $chord = Parse-Chord $Chords[$command]
    [UI]::Chord($chord[0], $chord[1]); Start-Sleep -Milliseconds 350
}

# Popup menus (#32768) open by Fancy Window.
function MenuOpen { @([UI]::OfProcess($script:FWProc.Id) | Where-Object { [UI]::ClassName($_) -eq '#32768' }).Count -gt 0 }

# ---- geometry ---------------------------------------------------------------

# Canvas in screen px: below the 26px menu bar, above the 24px status bar (100% scale).
function Canvas { $c = [UI]::Client($script:FW); @($c[0], ($c[1] + 26), $c[2], ($c[3] - 50)) }

# A zone given as fractions of the canvas, e.g. 0,0,0.5,0.5 for top-left of a 2x2.
function ZoneRect([double]$fx, [double]$fy, [double]$fw, [double]$fh) {
    $c = Canvas
    @(($c[0] + $c[2] * $fx), ($c[1] + $c[3] * $fy), ($c[2] * $fw), ($c[3] * $fh))
}
function Center($z) { @([int]($z[0] + $z[2] / 2), [int]($z[1] + $z[3] / 2)) }

# Where a hosted window should sit: 2 + margin from canvas edges, 4 + margin next to splitters.
# Zone headers are on by default (Layout > Hide zone headers): 28 DIP over each window.
$HeaderHeight = 28
function HostRect($z, [double]$margin = 0) {
    $c = Canvas
    $base = 2 + $margin
    $l = if ($z[0] -le $c[0] + 0.5) { $base } else { $base + 2 }
    $t = $HeaderHeight + $(if ($z[1] -le $c[1] + 0.5) { $base } else { $base + 2 })
    $r = if ($z[0] + $z[2] -ge $c[0] + $c[2] - 0.5) { $base } else { $base + 2 }
    $b = if ($z[1] + $z[3] -ge $c[1] + $c[3] - 0.5) { $base } else { $base + 2 }
    @(($z[0] + $l), ($z[1] + $t), ($z[2] - $l - $r), ($z[3] - $t - $b))
}

function Hosted([IntPtr]$h) { [UI]::Owner($h) -eq $script:FW -and -not [UI]::HasThickFrame($h) }

# Drags a test window by its title bar (hosted windows keep their caption) so the
# cursor lands on (x, y), holding Alt unless -NoAlt.
function AltDrop([IntPtr]$h, $at, [switch]$NoAlt) {
    if (-not (Hosted $h)) { [UI]::Raise($h); Start-Sleep -Milliseconds 150 }
    $r = [UI]::Rect($h)
    $hold = if ($NoAlt) { [uint16[]]@() } else { [uint16[]]@($VK.Alt) }
    SDrag ($r[0] + 120) ($r[1] + 14) $at[0] $at[1] $hold
    Start-Sleep -Milliseconds 500
}

# ---- setup ------------------------------------------------------------------

function Start-TestForm([int]$i) {
    Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$TestForms`" -Index $i"
}
function Get-TestForm([int]$i) {
    if (-not (Wait-Until { [UI]::Find("FW Test $i").Count -gt 0 } 10000)) { throw "FW Test $i never appeared" }
    $h = [UI]::Find("FW Test $i")[0]
    [void]$script:Pids.Add([UI]::Pid($h))
    $h
}
function Start-FW {
    $p = Start-Process $Exe -ArgumentList "--profile $Profile_" -PassThru
    if (-not (Wait-Until { [UI]::Find($Title).Count -gt 0 } 8000)) { throw 'test instance never appeared' }
    $script:FW = [UI]::Find($Title)[0]
    $script:FWProc = $p
    [void]$script:Pids.Add([uint32]$p.Id)
    [UI]::Place($script:FW, 100, 80, 1400, 900)
    Start-Sleep -Milliseconds 400
}
function Stop-FW {
    if ($script:FW -and [UI]::IsWindow($script:FW)) { [UI]::Close($script:FW); $null = Wait-Until { -not [UI]::IsWindow($script:FW) } 4000 }
}

# Spare spots for unhosted test windows, right of Fancy Window on the primary monitor.
function Park([IntPtr]$h, [int]$slot) { [UI]::Place($h, 1560, 80 + 300 * $slot, 420, 280); Start-Sleep -Milliseconds 150 }

function Run($id, $name, [scriptblock]$body) {
    if ($Only.Count -and $Only -notcontains $id) { return }
    $script:Story = "$id $name"
    Log "`n== $id  $name"
    try { Front; & $body } catch { Check 'ran without being refused or erroring' $false $_.Exception.Message }
    # Never leave a modifier held, whatever happened.
    foreach ($k in $VK.Ctrl, $VK.Alt, $VK.Shift, $VK.Win) { [UI]::KeyUp($k) }
}

# Every instance hears every Alt+drop and hit-tests it against its own canvas, so no other
# Fancy Window may overlap the primary monitor, where all test drags happen.
foreach ($h in [UI]::Find('Fancy Window')) {
    # Drops are hit-tested against the client area; a maximised frame overhangs its monitor by a few px.
    $r = [UI]::Client($h)
    if (-not [UI]::Minimized($h) -and $r[0] -lt 2560 -and $r[0] + $r[2] -gt 0 -and $r[1] -lt 1440 -and $r[1] + $r[3] -gt 0) {
        throw "Another Fancy Window overlaps the primary monitor at $($r -join ','); move it to another monitor first."
    }
}

# Fresh start: no stray windows from earlier runs, settings seeded with only the user's hotkeys.
foreach ($t in @($Title) + (1..5 | ForEach-Object { "FW Test $_" })) { foreach ($h in [UI]::Find($t)) { [UI]::Close($h) } }
Start-Sleep -Milliseconds 800
Remove-Item -ErrorAction SilentlyContinue $SettingsFile, "$SettingsFile.bak", $CrashLog, "$Out\log.txt"
# No BOM: Windows PowerShell's utf8 adds one, which the settings parser may reject.
[IO.File]::WriteAllText($SettingsFile, (@{ hotkeys = $UserHotkeys } | ConvertTo-Json))
Log ("hotkeys under test: " + (($Chords.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join ', '))

Start-FW
1..4 | ForEach-Object { Start-TestForm $_ }
$T = @{}; 1..4 | ForEach-Object { $T[$_] = Get-TestForm $_; Park $T[$_] ($_ - 1) }

# ---- stories ----------------------------------------------------------------

Run S01 'Launches with a 2x2 grid, profile title and status bar' {
    Check 'window title shows profile' ([UI]::Title($script:FW) -eq $Title)
    Shot 'S01-launch'
    $c = Canvas
    $zone = [UI]::Pixel(($c[0] + $c[2] / 4), ($c[1] + $c[3] / 4))
    Check 'zones are painted with the accent tint' ($zone -eq '#183041') $zone
    $cl = [UI]::Client($script:FW)
    $bar = [UI]::Pixel(($cl[0] + [int]($cl[2] * 0.6)), ($cl[1] + $cl[3] - 4))
    Check 'status bar is the accent colour' ($bar -eq '#007ACC') $bar
    [UI]::Shot($cl[0], $cl[1] + $cl[3] - 24, $cl[2], 24, "$Out\S01-status-bar.png")
}

Run S02 'Alt+drag a window into a zone hosts it' {
    $z = ZoneRect 0 0 0.5 0.5
    AltDrop $T[1] (Center $z)
    $r = [UI]::Rect($T[1]); $want = HostRect $z
    Check 'owned by Fancy Window, frame stripped' (Hosted $T[1])
    Check 'fills the top-left zone' (Near $r $want) "got $(Fmt $r) want $(Fmt $want)"
    Check 'hosted window is above Fancy Window' ([UI]::Above($T[1], $script:FW))
    Shot 'S02-hosted-one'
}

Run S03 'Host two more windows' {
    $zones = @{ 2 = (ZoneRect 0.5 0 0.5 0.5); 3 = (ZoneRect 0 0.5 0.5 0.5) }
    $script:T3Original = [UI]::Rect($T[3])
    foreach ($i in 2, 3) {
        AltDrop $T[$i] (Center $zones[$i])
        $r = [UI]::Rect($T[$i]); $want = HostRect $zones[$i]
        Check "FW Test $i hosted in its zone" ((Hosted $T[$i]) -and (Near $r $want)) "got $(Fmt $r) want $(Fmt $want)"
    }
    Shot 'S03-hosted-three'
}

Run S04 'Dragging a hosted window without Alt snaps it back' {
    $z = ZoneRect 0 0 0.5 0.5
    AltDrop $T[1] @(1700, 700) -NoAlt
    $r = [UI]::Rect($T[1])
    Check 'still hosted' (Hosted $T[1])
    Check 'snaps back to its zone' (Near $r (HostRect $z)) "got $(Fmt $r)"
}

Run S05 'Dropping onto an occupied zone: the old window goes back where it came from' {
    $z = ZoneRect 0 0.5 0.5 0.5
    AltDrop $T[4] (Center $z)
    Check 'FW Test 4 hosted in bottom-left' ((Hosted $T[4]) -and (Near ([UI]::Rect($T[4])) (HostRect $z)))
    $r3 = [UI]::Rect($T[3])
    Check 'FW Test 3 released (frame back, no owner)' ((-not (Hosted $T[3])) -and [UI]::HasThickFrame($T[3]) -and [UI]::Owner($T[3]) -eq [IntPtr]::Zero)
    Check 'FW Test 3 restored to its original rect' (Near $r3 $script:T3Original) "got $(Fmt $r3) want $(Fmt $script:T3Original)"
    Shot 'S05-swap'
}

Run S06 'Dragging the vertical splitter resizes hosted windows live' {
    $c = Canvas
    $x = [int]($c[0] + $c[2] / 2); $y = [int]($c[1] + $c[3] / 4)
    $w1 = ([UI]::Rect($T[1]))[2]; $w2 = ([UI]::Rect($T[2]))[2]
    # The splitter is only 4px; aim at its centre column.
    SDrag $x $y ($x + 150) $y -CheckEnd
    Start-Sleep -Milliseconds 400
    $d1 = ([UI]::Rect($T[1]))[2] - $w1; $d2 = ([UI]::Rect($T[2]))[2] - $w2
    Check 'left window grew by ~150' ([math]::Abs($d1 - 150) -le 6) "delta $d1"
    Check 'right window shrank by ~150' ([math]::Abs($d2 + 150) -le 6) "delta $d2"
    Shot 'S06-splitter'
    SDrag ($x + 150) $y $x $y -CheckEnd
    Start-Sleep -Milliseconds 400
    $back = ([UI]::Rect($T[1]))[2]
    Check 'dragging back restores the width' ([math]::Abs($back - $w1) -le 2) "width $back, was $w1"
    Shot 'S06-back'
}

Run S07 'Margin hotkeys widen and narrow the gap around hosted windows' {
    $z = ZoneRect 0 0 0.5 0.5
    Log "  T1 width before: $(([UI]::Rect($T[1]))[2])"
    Hotkey marginUp
    Log "  T1 width after one press: $(([UI]::Rect($T[1]))[2])"
    Hotkey marginUp
    $r = [UI]::Rect($T[1])
    Check 'margin 4 after two presses' (Near $r (HostRect $z 4)) "got $(Fmt $r) want $(Fmt (HostRect $z 4))"
    Shot 'S07-margin'
    Hotkey marginDown; Hotkey marginDown
    Check 'margin back to 0' (Near ([UI]::Rect($T[1])) (HostRect $z 0))
}

Run S08 'Clicking a hosted window shows which one is active' {
    $p = Center ([UI]::Rect($T[2])); SClick $p[0] $p[1]; Start-Sleep -Milliseconds 400
    Check 'FW Test 2 is foreground' ([UI]::GetForegroundWindow() -eq $T[2])
    Shot 'S08-active-T2'
    $p = Center ([UI]::Rect($T[1])); SClick $p[0] $p[1]; Start-Sleep -Milliseconds 400
    Check 'FW Test 1 is foreground' ([UI]::GetForegroundWindow() -eq $T[1])
    Shot 'S08-active-T1'
    # Ring pixels just outside each hosted window's left edge, mid-height (the zone header sits above).
    $r = [UI]::Rect($T[1]); $active = [UI]::Pixel(($r[0] - 1), ($r[1] + $r[3] / 2))
    $r2 = [UI]::Rect($T[2]); $inactive = [UI]::Pixel(($r2[0] - 1), ($r2[1] + $r2[3] / 2))
    Log "  ring colours: active=$active inactive=$inactive"
    Check 'active window ring stands out from inactive ones' ($active -ne $inactive) "active $active inactive $inactive"
    # The hosted window's drop shadow darkens the outermost bevel pixel a little.
    $blue = [Convert]::ToInt32($active.Substring(5, 2), 16)
    Check 'active ring is a bright accent bevel' ($blue -ge 0xB0) $active
}

Run S09 'Cycle hotkeys move focus through hosted windows in reading order' {
    # Reading order: T1 (TL), T2 (TR), T4 (BL). Another instance is running, so pressing
    # "next" on the last window would hand focus to it: never press past T4.
    $order = @($T[1], $T[2], $T[4])
    $p = Center ([UI]::Rect($T[1])); SClick $p[0] $p[1]; Start-Sleep -Milliseconds 300
    $seen = @()
    for ($i = 0; $i -lt 3 -and [UI]::GetForegroundWindow() -ne $T[4]; $i++) { Hotkey cycleNext; $seen += [UI]::GetForegroundWindow() }
    $names = ($seen | ForEach-Object { [UI]::Title($_) }) -join ' > '
    Log "  next ($($Chords.cycleNext)): $names"
    Check 'next reaches the last window in reading order' ($seen[-1] -eq $T[4]) $names
    Shot 'S09-cycle'
    Hotkey cyclePrevious
    $back = [UI]::GetForegroundWindow()
    Check "previous ($($Chords.cyclePrevious)) goes back to FW Test 2" ($back -eq $T[2]) ([UI]::Title($back))
}

Run S10 'Send behind shows a reminder; bring forward hides it' {
    $p = Center ([UI]::Rect($T[2])); SClick $p[0] $p[1]; Start-Sleep -Milliseconds 300
    Hotkey sendToBack
    Start-Sleep -Milliseconds 300
    $ind = [UI]::Find('Fancy Window - behind')
    Check 'reminder is showing' ($ind.Count -eq 1)
    if ($ind.Count) { $r = [UI]::Rect($ind[0]); [UI]::Shot($r[0] - 4, $r[1] - 4, $r[2] + 8, $r[3] + 8, "$Out\S10-reminder.png") }
    Check 'hosted windows still above Fancy Window' ([UI]::Above($T[1], $script:FW) -and [UI]::Above($T[2], $script:FW))
    Hotkey bringToFront
    Start-Sleep -Milliseconds 300
    Check 'reminder hidden' ([UI]::Find('Fancy Window - behind').Count -eq 0)
    Check 'Fancy Window is foreground' ([UI]::GetForegroundWindow() -eq $script:FW)
    Check 'hosted windows still above Fancy Window' ([UI]::Above($T[1], $script:FW))
}

Run S11 'Ctrl+click splits the empty zone into columns, Shift+click into rows' {
    $p = Center (ZoneRect 0.5 0.5 0.5 0.5)
    [UI]::MoveTo($p[0] + 3, $p[1]); Start-Sleep -Milliseconds 100
    Guard $p[0] $p[1]
    [UI]::KeyDown($VK.Ctrl); Start-Sleep -Milliseconds 200
    Shot 'S11-ctrl-preview'
    [UI]::KeyUp($VK.Ctrl)
    SModClick $VK.Ctrl $p[0] $p[1]
    $q = Center (ZoneRect 0.75 0.5 0.25 0.5)
    SModClick $VK.Shift $q[0] $q[1]
    Shot 'S11-splits'
    Check 'hosted windows untouched by splitting an empty zone' (Near ([UI]::Rect($T[2])) (HostRect (ZoneRect 0.5 0 0.5 0.5)))
}

Run S12 'Right-click a splitter merges the zones beside it' {
    $c = Canvas
    SRightClick ([int]($c[0] + $c[2] * 0.75)) ([int]($c[1] + $c[3] * 0.6)); Start-Sleep -Milliseconds 300
    Shot 'S12-merged'
}

Run S13 'Layout menu: Three columns keeps hosted windows in reading order' {
    $c = [UI]::Client($script:FW)
    SClick ($c[0] + 62) ($c[1] + 13); Start-Sleep -Milliseconds 400   # "Layout"
    Check 'Layout menu opened' (MenuOpen)
    Shot 'S13-layout-menu'
    if (MenuOpen) { 1..6 | ForEach-Object { SKey $VK.Down }; SKey $VK.Enter; Start-Sleep -Milliseconds 500 }
    $cols = @((ZoneRect 0 0 (1/3) 1), (ZoneRect (1/3) 0 (1/3) 1), (ZoneRect (2/3) 0 (1/3) 1))
    $ok = $true; $got = @()
    foreach ($pair in @(@($T[1], 0), @($T[2], 1), @($T[4], 2))) {
        $r = [UI]::Rect($pair[0]); $got += (Fmt $r)
        if (-not (Near $r (HostRect $cols[$pair[1]]))) { $ok = $false }
    }
    Check 'FW Test 1, 2, 4 fill the three columns in order' $ok ($got -join ' | ')
    Shot 'S13-three-columns'
}

Run S14 'Resizing Fancy Window reflows hosted windows' {
    [UI]::Place($script:FW, 100, 80, 1200, 800); Start-Sleep -Milliseconds 500
    $r = [UI]::Rect($T[1]); $want = HostRect (ZoneRect 0 0 (1/3) 1)
    Check 'first column window follows the new size' (Near $r $want) "got $(Fmt $r) want $(Fmt $want)"
    Shot 'S14-resized'
    [UI]::Place($script:FW, 100, 80, 1400, 900); Start-Sleep -Milliseconds 500
}

Run S15 'Alt+drag a hosted window out lets it go where it is dropped' {
    AltDrop $T[4] @(1750, 1000)
    Check 'FW Test 4 released (owner cleared, frame back)' ((-not (Hosted $T[4])) -and [UI]::HasThickFrame($T[4]))
    $r = [UI]::Rect($T[4])
    Check 'stays where it was dropped' ($r[0] -gt 1500) "at $(Fmt $r)"
    Shot 'S15-dragged-out'
}

Run S16 'Closing a hosted app frees its zone' {
    [UI]::Close($T[2]); Start-Sleep -Milliseconds 2600
    Check 'FW Test 2 gone, Fancy Window still running' ((-not [UI]::IsWindow($T[2])) -and [UI]::IsWindow($script:FW))
    Shot 'S16-closed-one'
    AltDrop $T[3] (Center (ZoneRect (1/3) 0 (1/3) 1))
    Check 'FW Test 3 hosted in the freed column' (Hosted $T[3])
}

Run S17 'Reset hotkey goes back to 2x2 and releases windows to where they came from' {
    # By design (tests/app_attach.rs reset_releases_all_windows) reset lets every window go.
    $p = Center ([UI]::Rect($T[1])); SClick $p[0] $p[1]; Start-Sleep -Milliseconds 200
    Hotkey resetLayout; Start-Sleep -Milliseconds 600
    Shot 'S17-reset'
    $r1 = [UI]::Rect($T[1]); $r3 = [UI]::Rect($T[3])
    Check 'FW Test 1 released' (-not (Hosted $T[1]) -and [UI]::HasThickFrame($T[1]))
    Check 'FW Test 1 back at its parking spot, not left over Fancy Window' (Near $r1 @(1560, 80, 420, 280)) "got $(Fmt $r1)"
    Check 'FW Test 3 back at its parking spot' (Near $r3 @(1560, 680, 420, 280)) "got $(Fmt $r3)"
}

Run S18 'Workspaces menu opens' {
    $c = [UI]::Client($script:FW)
    SClick ($c[0] + 140) ($c[1] + 13); Start-Sleep -Milliseconds 400   # "Workspaces"
    Check 'Workspaces menu opened' (MenuOpen)
    Shot 'S18-workspaces-menu'
    if (MenuOpen) { SKey $VK.Esc; Start-Sleep -Milliseconds 300 }
}

Run S19 'Exit lets hosted windows go in place and saves settings' {
    Stop-FW
    Start-Sleep -Milliseconds 400
    Check 'Fancy Window closed' (-not [UI]::IsWindow($script:FW))
    Check 'FW Test 1 survived, released in place' ([UI]::IsWindow($T[1]) -and [UI]::HasThickFrame($T[1]) -and [UI]::Owner($T[1]) -eq [IntPtr]::Zero)
    Check 'settings file written' (Test-Path $SettingsFile)
    $saved = Get-Content -Raw $SettingsFile | ConvertFrom-Json
    Check 'custom hotkeys kept' ($saved.hotkeys.cycleNext -eq $Chords.cycleNext) "$($saved.hotkeys.cycleNext)"
    Check 'no crash log' (-not (Test-Path $CrashLog))
}

Run S20 'Relaunch restores the window bounds' {
    Start-FW
    # Start-FW re-places it; check the saved bounds instead.
    $saved = Get-Content -Raw $SettingsFile | ConvertFrom-Json
    Check 'saved bounds match the last position' ($saved.windowLeft -eq 100 -and $saved.windowTop -eq 80 -and $saved.windowWidth -eq 1400) "$($saved.windowLeft),$($saved.windowTop) $($saved.windowWidth)x$($saved.windowHeight)"
    Shot 'S20-relaunched'
}

# ---- teardown ---------------------------------------------------------------

Stop-FW
foreach ($i in 1..4) { if ($T[$i] -and [UI]::IsWindow($T[$i])) { [UI]::Close($T[$i]) } }
$fail = @($script:Results | Where-Object { -not $_.Ok })
Log ("`n{0} checks, {1} failed" -f $script:Results.Count, $fail.Count)
$script:Results | ConvertTo-Json -Depth 3 | Set-Content -Encoding utf8 "$Out\results.json"
