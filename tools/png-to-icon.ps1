<#
.SYNOPSIS
    Converts a small monochrome PNG into the Flipper Zero .icon format.

.DESCRIPTION
    ufbt did this automatically for the C build. The Rust build has no fbt, so
    the .icon file is committed alongside the source PNG and regenerated with
    this script whenever the PNG changes.

    Output format: a leading 0x00 byte meaning "uncompressed", followed by XBM
    bitmap data - rows padded to whole bytes, least significant bit leftmost,
    a set bit meaning ink.

.PARAMETER Invert
    Flips ink and background. Use it if the icon shows up as a negative of the
    PNG on the device.

.EXAMPLE
    ./tools/png-to-icon.ps1 -Source ktool.png -Destination src/ktool.icon
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $Source,
    [Parameter(Mandatory)] [string] $Destination,
    [switch] $Invert
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$sourcePath = (Resolve-Path -LiteralPath $Source).Path
$destinationPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Destination)

$bitmap = [System.Drawing.Bitmap]::FromFile($sourcePath)
try {
    $bytesPerRow = [int][math]::Ceiling($bitmap.Width / 8)
    $icon = [byte[]]::new(1 + $bytesPerRow * $bitmap.Height)
    $icon[0] = 0x00

    for ($y = 0; $y -lt $bitmap.Height; $y++) {
        for ($x = 0; $x -lt $bitmap.Width; $x++) {
            $pixel = $bitmap.GetPixel($x, $y)
            $luminance = 0.299 * $pixel.R + 0.587 * $pixel.G + 0.114 * $pixel.B
            $isInk = ($pixel.A -ge 128) -and ($luminance -lt 128)
            if ($Invert) { $isInk = -not $isInk }

            if ($isInk) {
                $index = 1 + $y * $bytesPerRow + [int][math]::Floor($x / 8)
                $icon[$index] = $icon[$index] -bor (1 -shl ($x % 8))
            }
        }
    }

    [System.IO.File]::WriteAllBytes($destinationPath, $icon)
    "wrote $destinationPath - $($bitmap.Width)x$($bitmap.Height), $($icon.Length) bytes"
}
finally {
    $bitmap.Dispose()
}
