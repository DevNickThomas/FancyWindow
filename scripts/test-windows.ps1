# Opens throwaway windows to Alt+drag into Fancy Window, so manual testing
# never involves your real apps. Close them like any window.
#   .\scripts\test-windows.ps1 [-Count 2]
param([int]$Count = 2, [int]$Index = 0)

if ($Index -eq 0) {
    for ($i = 1; $i -le $Count; $i++) {
        Start-Process powershell -WindowStyle Hidden -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Index $i"
    }
    return
}

Add-Type -AssemblyName System.Windows.Forms
$colors = 'DarkOrange', 'SeaGreen', 'SteelBlue', 'IndianRed'
$f = New-Object System.Windows.Forms.Form
$f.Text = "FW Test $Index"; $f.StartPosition = 'Manual'
$f.Left = 60 + 80 * $Index; $f.Top = 60 + 60 * $Index; $f.Width = 420; $f.Height = 300
$f.BackColor = [System.Drawing.Color]::FromName($colors[($Index - 1) % $colors.Count])
$l = New-Object System.Windows.Forms.Label
$l.Text = $f.Text; $l.AutoSize = $true; $l.Left = 16; $l.Top = 16
$l.Font = New-Object System.Drawing.Font('Segoe UI', 18)
$f.Controls.Add($l)
# -WindowStyle Hidden applies to this process's first shown window, which is the form
# (the console belongs to conhost). Spend it on a throwaway show, then show for real.
$f.Show(); $f.Hide()
[System.Windows.Forms.Application]::Run($f)
