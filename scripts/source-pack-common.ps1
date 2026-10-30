function Invoke-PackDownload {
    param(
        [string[]] $Urls,
        [string] $Archive
    )

    foreach ($url in $Urls) {
        & curl.exe -L --fail --retry 3 --retry-delay 2 --max-time 1800 --silent --show-error -o $Archive $url
        if ($LASTEXITCODE -eq 0) {
            return
        }
    }
    throw "Source archive download failed: $Archive"
}

function Get-ArchiveTop {
    param(
        [string] $Tar,
        [string] $Archive
    )

    $entry = & $Tar -tf $Archive | Select-Object -First 1
    if (-not $entry) {
        throw "Source archive is empty: $Archive"
    }
    return $entry.TrimEnd('/')
}

function Remove-Tree {
    param([string] $Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    $empty = Join-Path ([System.IO.Path]::GetTempPath()) ("lattice-empty-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $empty -Force | Out-Null
    try {
        robocopy $empty $Path /MIR /R:1 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
    } finally {
        Remove-Item -LiteralPath $empty -Recurse -Force -ErrorAction SilentlyContinue
    }
    Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
}
