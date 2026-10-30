$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "19.3.1"
$tag = "v$version"
$target = Join-Path $content "react-$version"
$archive = Join-Path $content "react-$version.tar.gz"
$defaultEntry = "packages\react-reconciler\src\ReactFiberWorkLoop.js"
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
$upstream = "https://github.com/facebook/react/archive/refs/tags/$tag.tar.gz"
. (Join-Path $PSScriptRoot "source-pack-common.ps1")
$packages = @(
    "react",
    "react-dom",
    "react-dom-bindings",
    "react-reconciler",
    "scheduler",
    "shared",
    "react-is",
    "react-server"
)

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "React $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null

Write-Host "Downloading React $version source..."
Invoke-PackDownload -Urls @("https://gh-proxy.com/$upstream", $upstream) -Archive $archive

$top = Get-ArchiveTop -Tar $tar -Archive $archive
$members = @($packages | ForEach-Object { "$top/packages/$_/src" })
$members += "$top/LICENSE"
$members += "$top/README.md"

Remove-Tree -Path $target
New-Item -ItemType Directory -Path $target -Force | Out-Null

Write-Host "Extracting React package sources..."
& $tar -xf $archive -C $target --strip-components=1 @members
if ($LASTEXITCODE -ne 0) {
    throw "React source extraction failed."
}
Remove-Item -LiteralPath $archive -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "react-$version",
  "name": "React",
  "implementation": "React",
  "version": "$version",
  "language": "JavaScript",
  "source_language": "JavaScript",
  "license": "MIT",
  "default_entry": "packages/react-reconciler/src/ReactFiberWorkLoop.js"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("React {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
