$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "7.0.2"
$tag = "v$version"
$target = Join-Path $content "typescript-$version"
$defaultEntry = "src\compiler\checker.ts"

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "TypeScript $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null
$staging = Join-Path $content "typescript-$version-clone"
if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}

Write-Host "Sparse cloning TypeScript $version compiler source..."
git clone --depth 1 --branch $tag --filter=blob:none --sparse `
    https://github.com/microsoft/TypeScript.git $staging
if ($LASTEXITCODE -ne 0) {
    throw "TypeScript clone failed."
}
git -C $staging sparse-checkout set src
if ($LASTEXITCODE -ne 0) {
    throw "TypeScript sparse checkout failed."
}
if (-not (Test-Path -LiteralPath (Join-Path $staging $defaultEntry))) {
    throw "Unexpected TypeScript source layout."
}

if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
}
New-Item -ItemType Directory -Path $target -Force | Out-Null
Move-Item -LiteralPath (Join-Path $staging "src") -Destination (Join-Path $target "src")
foreach ($file in @("package.json", "README.md", "LICENSE.txt", "ThirdPartyNoticeText.txt")) {
    $source = Join-Path $staging $file
    if (Test-Path -LiteralPath $source) {
        Copy-Item -LiteralPath $source -Destination (Join-Path $target $file)
    }
}
Remove-Item -LiteralPath $staging -Recurse -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "typescript-$version",
  "name": "TypeScript",
  "implementation": "TypeScript",
  "version": "$version",
  "language": "TypeScript",
  "source_language": "TypeScript",
  "license": "Apache-2.0",
  "default_entry": "src/compiler/checker.ts"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("TypeScript {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
