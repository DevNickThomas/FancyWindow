# Renders resources\app.ico: the "Pane" mark (a frame holding one lit window) on a
# dark tile, as PNG entries at 16-256 px. Run after changing the design.
#   .\scripts\make-icon.ps1
Add-Type -AssemblyName System.Drawing

$IcoPath = Join-Path $PSScriptRoot '..\resources\app.ico'
$Sizes = 16, 32, 48, 64, 128, 256
$TileColor = [Drawing.Color]::FromArgb(0x16, 0x19, 0x1E)
$FrameColor = [Drawing.Color]::FromArgb(0xD5, 0xDA, 0xE1)
$LitColor = [Drawing.Color]::FromArgb(0x4D, 0xA3, 0xFF)

function RoundedRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object Drawing.Drawing2D.GraphicsPath
    $d = 2 * $r
    $p.AddArc($x, $y, $d, $d, 180, 90); $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90); $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure(); $p
}

function Render([int]$size) {
    $bmp = New-Object Drawing.Bitmap $size, $size, ([Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'
    $s = $size / 64.0
    $tile = RoundedRect 0 0 $size $size (14 * $s)
    $g.FillPath((New-Object Drawing.SolidBrush $TileColor), $tile)

    # Small sizes: a bigger mark and a thicker frame so it doesn't thin to nothing.
    $frac = if ($size -le 16) { 0.86 } elseif ($size -le 32) { 0.76 } else { 0.64 }
    $m = $size * $frac / 64.0
    $o = ($size - 64 * $m) / 2
    $stroke = [math]::Max(5 * $m, [math]::Min(2.0, $size / 10))
    $g.TranslateTransform($o, $o)

    $frame = RoundedRect (7 * $m) (7 * $m) (50 * $m) (50 * $m) (11 * $m)
    $g.DrawPath((New-Object Drawing.Pen $FrameColor, $stroke), $frame)

    # The lit window, with a stepped glow at sizes where a glow can be seen.
    $x = 31 * $m; $w = 18 * $m; $r = 4 * $m
    if ($size -ge 48) {
        foreach ($step in 4, 3, 2, 1) {
            $grow = $step * 1.4 * $m
            $glow = RoundedRect ($x - $grow) ($x - $grow) ($w + 2 * $grow) ($w + 2 * $grow) ($r + $grow)
            $g.FillPath((New-Object Drawing.SolidBrush ([Drawing.Color]::FromArgb(28, $LitColor))), $glow)
        }
    }
    $g.FillPath((New-Object Drawing.SolidBrush $LitColor), (RoundedRect $x $x $w $w ([math]::Max($r, 1))))
    $g.Dispose()
    $ms = New-Object IO.MemoryStream
    $bmp.Save($ms, [Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    , $ms.ToArray()
}

# ICO: 6-byte header, a 16-byte entry per image, then the PNG data.
$pngs = foreach ($size in $Sizes) { , (Render $size) }
$ico = New-Object IO.MemoryStream
$w = New-Object IO.BinaryWriter $ico
$w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$Sizes.Count)
$offset = 6 + 16 * $Sizes.Count
for ($i = 0; $i -lt $Sizes.Count; $i++) {
    $dim = if ($Sizes[$i] -ge 256) { 0 } else { $Sizes[$i] }
    $w.Write([byte]$dim); $w.Write([byte]$dim); $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([uint16]1); $w.Write([uint16]32)
    $w.Write([uint32]$pngs[$i].Length); $w.Write([uint32]$offset)
    $offset += $pngs[$i].Length
}
foreach ($png in $pngs) { $w.Write($png) }
$w.Flush()
[IO.File]::WriteAllBytes([IO.Path]::GetFullPath($IcoPath), $ico.ToArray())
Write-Host "wrote $IcoPath ($($ico.Length) bytes, $($Sizes -join '/') px)"
