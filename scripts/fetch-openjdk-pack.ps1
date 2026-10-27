$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "27"
$tag = "jdk-27-ga"
$target = Join-Path $content "openjdk-jdk-$version"
$defaultEntry = "src\java.base\share\classes\java\util\ArrayList.java"

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "OpenJDK $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null
$zipPath = Join-Path $content "openjdk-jdk-$version.zip"
Write-Host "Downloading OpenJDK $version source archive..."
curl.exe -L --retry 3 --retry-delay 2 --max-time 1800 --silent --show-error `
    -o $zipPath "https://codeload.github.com/openjdk/jdk/zip/refs/tags/$tag"
if ($LASTEXITCODE -ne 0) {
    throw "OpenJDK source download failed."
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$prefix = "$tag/src/java.base/share/classes/"
$destRoot = Join-Path $target "src\java.base\share\classes"
if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
}
New-Item -ItemType Directory -Path $destRoot -Force | Out-Null

Write-Host "Extracting java.base source..."
$zip = [System.IO.Compression.ZipFile]::OpenRead($zipPath)
try {
    $count = 0
    foreach ($entry in $zip.Entries) {
        if (-not $entry.FullName.StartsWith($prefix)) { continue }
        $relative = $entry.FullName.Substring($prefix.Length)
        if ([string]::IsNullOrEmpty($relative) -or $relative.EndsWith('/')) { continue }
        $destination = Join-Path $destRoot ($relative -replace '/', '\')
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
Remove-Item -LiteralPath $zipPath -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "openjdk-jdk-$version",
  "name": "OpenJDK",
  "implementation": "OpenJDK",
  "version": "$version",
  "language": "Java",
  "source_language": "Java",
  "license": "GPL-2.0-CE",
  "default_entry": "src/java.base/share/classes/java/util/ArrayList.java"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("OpenJDK {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
