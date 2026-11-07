# Samples the mean RGB of a fixed region inside the editor/container in each
# dock screenshot (PNG coordinates; editor sits at PNG (9,69) from window origin
# (452,161) in dock-vinyl-win11.log). Visible editor pixels are Vinyl's
# background (38,45,54); a hidden tab shows the GPUI background (20,24,29).
# Overhang/clip is NOT sampled: the slot fills the window edge, so the
# editor region beyond the container lies outside the window (frame pixels).
param([string]$Dir = $PSScriptRoot)
Add-Type -AssemblyName System.Drawing

function Mean-Box([System.Drawing.Bitmap]$bmp, [int]$x0, [int]$y0, [int]$x1, [int]$y1) {
    $r = 0; $g = 0; $b = 0; $n = 0
    for ($x = $x0; $x -le $x1; $x++) {
        for ($y = $y0; $y -le $y1; $y++) {
            $c = $bmp.GetPixel($x, $y)
            $r += $c.R; $g += $c.G; $b += $c.B; $n++
        }
    }
    return ('{0},{1},{2}' -f [math]::Round($r / $n), [math]::Round($g / $n), [math]::Round($b / $n))
}

foreach ($name in @('dock-attached', 'dock-tab-hidden', 'dock-tab-shown', 'dock-resize-grow', 'dock-resize-shrink')) {
    $path = Join-Path $Dir "$name.png"
    if (-not (Test-Path $path)) { Write-Output "$name missing"; continue }
    $bmp = [System.Drawing.Bitmap]::FromFile($path)
    try {
        $inside = Mean-Box $bmp 20 100 60 140
        Write-Output ("{0} size={1}x{2} editor_region_rgb={3}" -f $name, $bmp.Width, $bmp.Height, $inside)
    } finally {
        $bmp.Dispose()
    }
}
