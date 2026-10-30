$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "3.5.44"
$tag = "v$version"
$target = Join-Path $content "vue-$version"
$archive = Join-Path $content "vue-$version.tar.gz"
$defaultEntry = "packages\runtime-core\src\renderer.ts"
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
$upstream = "https://github.com/vuejs/core/archive/refs/tags/$tag.tar.gz"

. (Join-Path $PSScriptRoot "source-pack-common.ps1")

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "Vue $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null

Write-Host "Downloading Vue $version source..."
Invoke-PackDownload -Urls @("https://gh-proxy.com/$upstream", $upstream) -Archive $archive

$entries = & $tar -tf $archive
$top = Get-ArchiveTop -Tar $tar -Archive $archive
$members = @(
    $entries |
        ForEach-Object {
            if ($_ -match "^($([regex]::Escape($top))/packages/[^/]+/src)/") {
                $Matches[1]
            }
        } |
        Sort-Object -Unique
)
$members += "$top/LICENSE"
$members += "$top/README.md"
if ($members.Count -le 2) {
    throw "Unexpected Vue source layout."
}

Remove-Tree -Path $target
New-Item -ItemType Directory -Path $target -Force | Out-Null

Write-Host "Extracting Vue package sources..."
& $tar -xf $archive -C $target --strip-components=1 @members
if ($LASTEXITCODE -ne 0) {
    throw "Vue source extraction failed."
}
Remove-Item -LiteralPath $archive -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "vue-$version",
  "name": "Vue",
  "implementation": "Vue",
  "version": "$version",
  "language": "TypeScript",
  "source_language": "TypeScript",
  "license": "MIT",
  "default_entry": "packages/runtime-core/src/renderer.ts"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("Vue {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
