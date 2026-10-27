$ErrorActionPreference = "Stop"

. "$PSScriptRoot\prepare-native-build.ps1"

$root = $script:RepoRoot
$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Path $dist -Force | Out-Null

function Build-AppPackage {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name,
        [Parameter(Mandatory = $true)]
        [string[]] $CargoArguments
    )

    Write-Host "`nBuilding $Name..."
    & cargo @CargoArguments
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed for $Name (exit $LASTEXITCODE)."
    }

    $exe = Join-Path $dist "$Name.exe"
    $zip = Join-Path $dist "$Name.zip"
    Copy-Item (Join-Path $root "target\release\lattice.exe") $exe -Force
    if (Test-Path -LiteralPath $zip) {
        Remove-Item -LiteralPath $zip -Force
    }
    Compress-Archive -Path $exe -DestinationPath $zip -CompressionLevel Optimal

    $exeItem = Get-Item -LiteralPath $exe
    $zipItem = Get-Item -LiteralPath $zip
    [pscustomobject]@{
        Package = $Name
        ExeBytes = $exeItem.Length
        ExeMiB = [math]::Round($exeItem.Length / 1MB, 2)
        ZipBytes = $zipItem.Length
        ZipMiB = [math]::Round($zipItem.Length / 1MB, 2)
    }
}

$webview = Build-AppPackage -Name "lattice-webview" -CargoArguments @(
    "build", "--release", "-p", "lattice"
)

$native = Build-AppPackage -Name "lattice-native" -CargoArguments @(
    "build", "--release", "-p", "lattice", "--no-default-features", "--features", "native"
)

$rows = @($webview, $native)
$rows | Format-Table Package, ExeBytes, ExeMiB, ZipBytes, ZipMiB -AutoSize

$report = [ordered]@{
    generated_at = (Get-Date).ToString("o")
    target = "x86_64-pc-windows-msvc"
    profile = "release"
    compression = "ZIP / Deflate"
    packages = $rows
}
$report | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $dist "sizes.json") -Encoding utf8

Write-Host "`nApplication bundles only. Source Packs and the optional WebView2 Runtime are excluded."
Write-Host "Offline WebView2 deployments must budget the Evergreen Standalone Installer separately."
