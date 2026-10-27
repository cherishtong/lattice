$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "3.13.2"
$target = Join-Path $content "cpython-$version"
$archive = Join-Path $content "cpython-$version.zip"

if (Test-Path -LiteralPath (Join-Path $target "Objects\listobject.c")) {
    Write-Host "CPython $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null

# Official GitHub tag archive for the exact learning revision.
$url = "https://codeload.github.com/python/cpython/zip/refs/tags/v$version"
Write-Host "Downloading CPython $version source..."
Invoke-WebRequest -Uri $url -OutFile $archive

Write-Host "Extracting..."
$staging = Join-Path $content "extracting"
if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
Expand-Archive -Path $archive -DestinationPath $staging -Force

$extracted = Join-Path $staging "cpython-$version"
if (-not (Test-Path -LiteralPath (Join-Path $extracted "Objects\listobject.c"))) {
    throw "Unexpected CPython archive layout."
}

Move-Item -LiteralPath $extracted -Destination $target
Remove-Item -LiteralPath $staging -Recurse -Force
Remove-Item -LiteralPath $archive -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "cpython-$version",
  "name": "CPython",
  "implementation": "CPython",
  "version": "$version",
  "language": "Python",
  "source_language": "C",
  "license": "PSF-2.0",
  "default_entry": "Objects/listobject.c"
}
"@
Set-Content -LiteralPath (Join-Path $manifestDir "pack.json") -Value $manifest -Encoding utf8
# Windows PowerShell writes a UTF-8 BOM; use a BOM-free file for JSON readers.
$manifestPath = Join-Path $manifestDir "pack.json"
$manifestText = Get-Content -LiteralPath $manifestPath -Raw
[System.IO.File]::WriteAllText($manifestPath, $manifestText, (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object -Property Length -Sum).Sum
Write-Host ("CPython {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
