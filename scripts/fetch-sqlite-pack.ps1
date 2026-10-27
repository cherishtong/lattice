param(
    [string] $Version = "3.54.1",
    [string] $Encoded = "3540100",
    [string] $Year = "2026",
    [string] $ExpectedSha3 = "877c6f4996ce7adb368686bce21638de9e3d91534832ad3d9d119037d1bc22ad"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = $Version
$encoded = $Encoded
$target = Join-Path $content "sqlite-$version"
$archive = Join-Path $content "sqlite-src-$encoded.zip"
$defaultEntry = "src/main.c"
$expectedSha3 = $ExpectedSha3

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "SQLite $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null
if (-not (Test-Path -LiteralPath $archive)) {
    $url = $null
    foreach ($candidateYear in @($Year)) {
        $candidate = "https://www.sqlite.org/$candidateYear/sqlite-src-$encoded.zip"
        $head = curl.exe -I -L --max-time 25 --silent --show-error $candidate
        if ($head -match "200 OK") {
            $url = $candidate
            break
        }
    }
    if (-not $url) {
        throw "SQLite source download failed."
    }

    $head = curl.exe -I -L --max-time 25 --silent --show-error $url
    $length = 0
    foreach ($line in $head) {
        if ($line -match "^Content-length:\s*(\d+)") {
            $length = [int64]$Matches[1]
            break
        }
    }
    if ($length -le 0) {
        throw "Could not read the SQLite source archive size."
    }

    Write-Host ("Downloading SQLite {0} source from {1} ({2:N2} MiB) ..." -f $version, $url, ($length / 1MB))
    $partsDir = Join-Path $content "sqlite-parts"
    if (Test-Path -LiteralPath $partsDir) {
        Remove-Item -LiteralPath $partsDir -Recurse -Force
    }
    New-Item -ItemType Directory -Path $partsDir -Force | Out-Null
    $chunk = 1048576
    $count = [math]::Ceiling($length / $chunk)
    $processes = @()
    for ($i = 0; $i -lt $count; $i++) {
        $start = $i * $chunk
        $end = [math]::Min(($i + 1) * $chunk - 1, $length - 1)
        $part = Join-Path $partsDir ("part{0:D2}.bin" -f $i)
        $arguments = @(
            "-L", "--fail", "--max-time", "600", "--silent", "--show-error",
            "-r", "$start-$end", "-o", $part, $url
        )
        $processes += Start-Process -FilePath "curl.exe" -ArgumentList $arguments -PassThru -WindowStyle Hidden
    }
    $processes | Wait-Process
    $failed = $processes | Where-Object { $_.ExitCode -ne 0 }
    $parts = Get-ChildItem -LiteralPath $partsDir -Filter "part*.bin" | Sort-Object Name
    $received = ($parts | Measure-Object Length -Sum).Sum
    if ($failed -or $received -ne $length) {
        Remove-Item -LiteralPath $partsDir -Recurse -Force
        throw "SQLite parallel download failed: received $received / $length bytes."
    }

    $output = [System.IO.File]::Create($archive)
    try {
        foreach ($part in $parts) {
            $input = [System.IO.File]::OpenRead($part.FullName)
            try { $input.CopyTo($output) } finally { $input.Dispose() }
        }
    } finally {
        $output.Dispose()
    }
    Remove-Item -LiteralPath $partsDir -Recurse -Force
} else {
    Write-Host "Using existing archive $archive"
}

$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLower()
Write-Host "Archive SHA-256: $hash"
Write-Host "Official SHA3-256: $expectedSha3"

Add-Type -AssemblyName System.IO.Compression.FileSystem
$prefix = "sqlite-src-$encoded/"
if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
}

Write-Host "Extracting canonical SQLite source tree..."
$zip = [System.IO.Compression.ZipFile]::OpenRead($archive)
try {
    $count = 0
    foreach ($entry in $zip.Entries) {
        if (-not $entry.FullName.StartsWith($prefix)) { continue }
        $relative = $entry.FullName.Substring($prefix.Length)
        if ([string]::IsNullOrEmpty($relative)) { continue }
        $destination = Join-Path $target ($relative -replace '/', '\')
        if ($relative.EndsWith('/')) {
            New-Item -ItemType Directory -Path $destination -Force | Out-Null
            continue
        }
        $parent = Split-Path -Parent $destination
        if (-not (Test-Path -LiteralPath $parent)) {
            New-Item -ItemType Directory -Path $parent -Force | Out-Null
        }
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $destination, $true)
        $count++
    }
} finally {
    $zip.Dispose()
}
Remove-Item -LiteralPath $archive -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "sqlite-$version",
  "name": "SQLite",
  "implementation": "SQLite",
  "version": "$version",
  "language": "C",
  "source_language": "C",
  "license": "Public Domain",
  "default_entry": "$defaultEntry"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("SQLite {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
