$ErrorActionPreference = "Stop"

$script:RepoRoot = Split-Path -Parent $PSScriptRoot

if (-not $script:RepoRoot) {
    throw "Unable to locate the repository root."
}

$pythonHome = Join-Path $script:RepoRoot ".tools\python"
$python = Join-Path $pythonHome "python.exe"

# Blitz/Stylo generate CSS property code at build time and require Python.
# Keep the build tool isolated inside the repository; do not alter system Python.
if (-not (Test-Path -LiteralPath $python)) {
    $tools = Join-Path $script:RepoRoot ".tools"
    $archive = Join-Path $tools "python-embed.zip"
    New-Item -ItemType Directory -Path $tools -Force | Out-Null
    Write-Host "Downloading isolated Python 3.12.9 for the Blitz build..."
    Invoke-WebRequest `
        -Uri "https://www.python.org/ftp/python/3.12.9/python-3.12.9-embed-amd64.zip" `
        -OutFile $archive
    Expand-Archive -Path $archive -DestinationPath $pythonHome -Force
    Remove-Item -LiteralPath $archive -Force
}

$env:PYTHON3 = $python
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
