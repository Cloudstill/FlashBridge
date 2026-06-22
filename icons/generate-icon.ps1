# Generates icons/icon.ico — an app icon with a lightning bolt on a
# rounded indigo background, multi-resolution (16/32/48/256), PNG entries.
# Uses System.Drawing (GDI+) always present on Windows .NET.
[CmdletBinding()]
param(
    [string]$OutDir = (Split-Path -Parent $MyInvocation.MyCommand.Path)
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir | Out-Null }
$iconPath = Join-Path $OutDir 'icon.ico'

function New-IconBitmap {
    param([int]$size)
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    # Rounded background: indigo gradient.
    $rect = New-Object System.Drawing.Rectangle 0, 0, $size, $size
    $radius = [int]($size * 0.22)
    $bgPath = New-Object System.Drawing.Drawing2D.GraphicsPath
    $bgPath.AddArc($rect.X, $rect.Y, $radius, $radius, 180, 90)
    $bgPath.AddArc($rect.Right - $radius, $rect.Y, $radius, $radius, 270, 90)
    $bgPath.AddArc($rect.Right - $radius, $rect.Bottom - $radius, $radius, $radius, 0, 90)
    $bgPath.AddArc($rect.X, $rect.Bottom - $radius, $radius, $radius, 90, 90)
    $bgPath.CloseFigure()

    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush `
        $rect, ([System.Drawing.Color]::FromArgb(255, 58, 90, 200)), ([System.Drawing.Color]::FromArgb(255, 30, 42, 120)), 90
    $g.FillPath($brush, $bgPath)
    $brush.Dispose()

    # Lightning bolt (white). Points as flat arrays: x0,y0,x1,y1,...
    $cx = $size / 2.0
    $s = $size / 100.0
    # design coords (offset from cx, y) for a bolt.
    $coords = 8,12, -22,56, -6,56, -14,88, 22,42, 4,42, 16,12
    $count = $coords.Length / 2
    $pts = New-Object 'System.Drawing.PointF[]' $count
    for ($k = 0; $k -lt $count; $k++) {
        $dx = [double]$coords[$k * 2]
        $dy = [double]$coords[$k * 2 + 1]
        $pts[$k] = New-Object System.Drawing.PointF (($cx + $dx * $s), ($dy * $s))
    }
    $bolt = New-Object System.Drawing.Drawing2D.GraphicsPath
    $bolt.AddPolygon($pts)
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
    $g.FillPath($white, $bolt)
    $white.Dispose()

    $g.Dispose()
    return $bmp
}

$sizes = @(256, 48, 32, 16)
$pngs = New-Object 'System.Collections.Generic.List[byte[]]'
foreach ($s in $sizes) {
    $bmp = New-IconBitmap -size $s
    $pngStream = New-Object System.IO.MemoryStream
    $bmp.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngs.Add($pngStream.ToArray())
    $pngStream.Dispose()
    $bmp.Dispose()
}

# Assemble .ico: ICONDIR + ICONDIRENTRY[] + image data (PNG).
$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $ms

$bw.Write([uint16]0)                       # reserved
$bw.Write([uint16]1)                       # type = icon
$bw.Write([uint16]$sizes.Length)           # count

$dirLen = 6 + 16 * $sizes.Length
$offset = $dirLen
foreach ($s in $sizes) {
    $png = $pngs[[array]::IndexOf($sizes, $s)]
    $w = if ($s -ge 256) { [byte]0 } else { [byte]$s }
    $h = $w
    $bw.Write($w)            # width
    $bw.Write($h)            # height
    $bw.Write([byte]0)       # color count
    $bw.Write([byte]0)       # reserved
    $bw.Write([uint16]1)     # planes
    $bw.Write([uint16]32)    # bpp
    $bw.Write([uint32]$png.Length)  # size
    $bw.Write([uint32]$offset)      # offset
    $offset += $png.Length
}

foreach ($s in $sizes) {
    $png = $pngs[[array]::IndexOf($sizes, $s)]
    $bw.Write($png)
}

[System.IO.File]::WriteAllBytes($iconPath, $ms.ToArray())
$bw.Dispose(); $ms.Dispose()
Write-Output "Wrote $iconPath"
