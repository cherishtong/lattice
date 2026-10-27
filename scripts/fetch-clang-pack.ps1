$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "20.1.0"
$target = Join-Path $content "clang-$version"
$archive = Join-Path $content "clang-$version.zip"
$defaultEntry = "clang/lib/Sema/Sema.cpp"

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "Clang $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null
Write-Host "Downloading LLVM/Clang $version source..."
curl.exe -L --retry 3 --retry-delay 2 --max-time 3600 --silent --show-error `
    -o $archive "https://codeload.github.com/llvm/llvm-project/zip/refs/tags/llvmorg-$version"
if ($LASTEXITCODE -ne 0) {
    throw "LLVM/Clang source download failed."
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$prefix = "llvm-project-llvmorg-$version/"
$include = @("clang/", "llvm/include/", "llvm/lib/Support/")
$looseFiles = @("LICENSE.TXT", "README.md")

if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
}

Write-Host "Extracting Clang frontend and LLVM support sources..."
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
  "id": "clang-$version",
  "name": "Clang",
  "implementation": "Clang",
  "version": "$version",
  "language": "C++",
  "source_language": "C++",
  "license": "Apache-2.0 WITH LLVM-exception",
  "default_entry": "$defaultEntry"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("Clang {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
