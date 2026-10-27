$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "15.1.0"
$branch = "releases/gcc-15"
$target = Join-Path $content "gcc-$version"
$archive = Join-Path $content "gcc-$version.zip"
$defaultEntry = "gcc/tree.c"

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "GCC $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null
Write-Host "Downloading GCC $version source..."
curl.exe -L --retry 3 --retry-delay 2 --max-time 3600 --silent --show-error `
    -o $archive "https://codeload.github.com/gcc-mirror/gcc/zip/refs/heads/$branch"
if ($LASTEXITCODE -ne 0) {
    throw "GCC source download failed."
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$prefix = "gcc-releases-gcc-15/"
$include = @("gcc/", "libgcc/", "libstdc++-v3/", "include/", "libcpp/")
$looseFiles = @("COPYING", "COPYING3", "README")

if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
}

Write-Host "Extracting GCC compiler and runtime sources..."
$zip = [System.IO.Compression.ZipFile]::OpenRead($archive)
try {
    $count = 0
    foreach ($entry in $zip.Entries) {
        if (-not $entry.FullName.StartsWith($prefix)) { continue }
        $relative = $entry.FullName.Substring($prefix.Length)
        if ([string]::IsNullOrEmpty($relative) -or $relative.EndsWith('/')) { continue }

        $wanted = $false
        foreach ($directory in $include) {
            if ($relative.StartsWith($directory)) {
                $wanted = $true
                break
            }
        }
        if (-not $wanted -and ($looseFiles -contains $relative)) {
            $wanted = $true
        }
        if (-not $wanted) { continue }

        $destination = Join-Path $target ($relative -replace '/', '\')
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
  "id": "gcc-$version",
  "name": "GCC",
  "implementation": "GCC",
  "version": "$version",
  "language": "C / C++",
  "source_language": "C",
  "license": "GPL-3.0-or-later",
  "default_entry": "$defaultEntry"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("GCC {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
