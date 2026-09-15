param(
    [string]$Source = (Join-Path $PSScriptRoot '..\..\assets\logo.ico')
)

$ErrorActionPreference = 'Stop'
$packageRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\chromium'))
$outputRoot = [System.IO.Path]::GetFullPath((Join-Path $packageRoot 'icons'))
if (-not $outputRoot.StartsWith(
        $packageRoot + [System.IO.Path]::DirectorySeparatorChar,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
    throw 'Icon output escaped the Chromium package root.'
}

Add-Type -AssemblyName System.Drawing
$sourcePath = (Resolve-Path -LiteralPath $Source).Path
[System.IO.Directory]::CreateDirectory($outputRoot) | Out-Null
foreach ($size in 16, 32, 48, 128) {
    $icon = [System.Drawing.Icon]::new(
        $sourcePath,
        [System.Drawing.Size]::new($size, $size)
    )
    $color = $icon.ToBitmap()
    $color.Save(
        (Join-Path $outputRoot "app-enabled-$size.png"),
        [System.Drawing.Imaging.ImageFormat]::Png
    )

    if ($size -in 16, 32) {
        $gray = [System.Drawing.Bitmap]::new(
            $color.Width,
            $color.Height,
            [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
        )
        for ($y = 0; $y -lt $color.Height; $y++) {
            for ($x = 0; $x -lt $color.Width; $x++) {
                $pixel = $color.GetPixel($x, $y)
                $luma = [int][Math]::Round(
                    0.2126 * $pixel.R + 0.7152 * $pixel.G + 0.0722 * $pixel.B
                )
                $muted = [int][Math]::Round(0.72 * $luma + 0.28 * 128)
                $gray.SetPixel(
                    $x,
                    $y,
                    [System.Drawing.Color]::FromArgb($pixel.A, $muted, $muted, $muted)
                )
            }
        }
        $gray.Save(
            (Join-Path $outputRoot "app-disabled-$size.png"),
            [System.Drawing.Imaging.ImageFormat]::Png
        )
        $gray.Dispose()
    }

    $color.Dispose()
    $icon.Dispose()
}

Write-Output "Generated Chromium icons from $sourcePath"