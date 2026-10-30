param(
    [string[]] $Pack = @()
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$output = Join-Path $root "source-packs"
$tar = Join-Path $env:SystemRoot "System32\tar.exe"

if (-not (Test-Path -LiteralPath $content)) {
    throw "Source pack directory not found: $content"
}
New-Item -ItemType Directory -Path $output -Force | Out-Null

if ($Pack.Count -eq 0) {
    $Pack = Get-ChildItem -LiteralPath $content -Directory |
        Where-Object {
            -not $_.Name.StartsWith(".") -and
            $_.Name -ne "models" -and
            (Test-Path -LiteralPath (Join-Path $_.FullName ".lattice\pack.json"))
        } |
        ForEach-Object { $_.Name }
}

$total = 0
foreach ($name in $Pack) {
    $source = Join-Path $content $name
    $manifest = Join-Path $source ".lattice\pack.json"
    if (-not (Test-Path -LiteralPath $manifest)) {
        throw "Source pack manifest not found: $manifest"
    }

    $archive = Join-Path $output "$name.zip"
    Remove-Item -LiteralPath $archive -Force -ErrorAction SilentlyContinue
    Write-Host "Packing $name..."
    & $tar -a -c -f $archive `
        --exclude "*/.lattice/documents/*" `
        --exclude "*/.lattice/symbol-index-*.postcard" `
        --exclude "*/.lattice/src-broken/*" `
        -C $content $name
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to pack $name."
    }

    $file = Get-Item -LiteralPath $archive
    $total += $file.Length
    if ($file.Length -ge 100MB) {
        throw "Archive exceeds GitHub's 100 MB file limit: $archive"
    }
    Write-Host ("  {0}: {1:N1} MiB" -f $file.Name, ($file.Length / 1MB))
}

Write-Host ("Created {0} archives, {1:N1} MiB total in {2}" -f $Pack.Count, ($total / 1MB), $output)
Get-ChildItem -LiteralPath $output -Filter *.zip |
    ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "{0}  {1}" -f $hash, $_.Name
    }
