$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "1.31.8"
$tag = "release-$version"
$target = Join-Path $content "nginx-$version"
$archive = Join-Path $content "nginx-$version.tar.gz"
$defaultEntry = "src\core\nginx.c"
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
$upstream = "https://github.com/nginx/nginx/archive/refs/tags/$tag.tar.gz"

. (Join-Path $PSScriptRoot "source-pack-common.ps1")

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "Nginx $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null

Write-Host "Downloading Nginx $version source..."
Invoke-PackDownload -Urls @("https://gh-proxy.com/$upstream", $upstream) -Archive $archive

$top = Get-ArchiveTop -Tar $tar -Archive $archive
Remove-Tree -Path $target
New-Item -ItemType Directory -Path $target -Force | Out-Null

Write-Host "Extracting Nginx source..."
& $tar -xf $archive -C $target --strip-components=1 "$top/src" "$top/LICENSE" "$top/README.md"
if ($LASTEXITCODE -ne 0) {
    throw "Nginx source extraction failed."
}
Remove-Item -LiteralPath $archive -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "nginx-$version",
  "name": "Nginx",
  "implementation": "Nginx",
  "version": "$version",
  "language": "C",
  "source_language": "C",
  "license": "BSD-2-Clause",
  "default_entry": "src/core/nginx.c"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("Nginx {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
