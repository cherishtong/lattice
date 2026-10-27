$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "1.99.1"
$target = Join-Path $content "rust-$version"
$archive = Join-Path $content "rust-$version.zip"
$defaultEntry = "library\core\src\lib.rs"

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "Rust $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null
Write-Host "Downloading Rust $version source..."
curl.exe -L --retry 3 --retry-delay 2 --max-time 3600 --silent --show-error `
    -o $archive "https://codeload.github.com/rust-lang/rust/zip/refs/tags/$version"
if ($LASTEXITCODE -ne 0) {
    throw "Rust source download failed."
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$prefix = "rust-$version/"
$include = @("compiler/", "library/", "src/", "LICENSES/")
$looseFiles = @("README.md", "RELEASES.md", "COPYRIGHT", "LICENSE-APACHE", "LICENSE-MIT")

if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
}

Write-Host "Extracting compiler, library and tooling source..."
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
  "id": "rust-$version",
  "name": "Rust",
  "implementation": "Rust",
  "version": "$version",
  "language": "Rust",
  "source_language": "Rust",
  "license": "MIT OR Apache-2.0",
  "default_entry": "library/core/src/lib.rs"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("Rust {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
