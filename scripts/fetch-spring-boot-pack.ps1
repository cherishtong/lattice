$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$content = Join-Path $root ".content"
$version = "4.1.2"
$tag = "v$version"
$target = Join-Path $content "spring-boot-$version"
$archive = Join-Path $content "spring-boot-$version.tar.gz"
$defaultEntry = "core\spring-boot\src\main\java\org\springframework\boot\SpringApplication.java"
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
$upstream = "https://github.com/spring-projects/spring-boot/archive/refs/tags/$tag.tar.gz"

. (Join-Path $PSScriptRoot "source-pack-common.ps1")

if (Test-Path -LiteralPath (Join-Path $target $defaultEntry)) {
    Write-Host "Spring Boot $version source already present at $target"
    exit 0
}

New-Item -ItemType Directory -Path $content -Force | Out-Null

Write-Host "Downloading Spring Boot $version source..."
Invoke-PackDownload -Urls @("https://gh-proxy.com/$upstream", $upstream) -Archive $archive

$entries = & $tar -tf $archive
$top = Get-ArchiveTop -Tar $tar -Archive $archive
$members = @(
    $entries |
        ForEach-Object {
            if ($_ -match "^($([regex]::Escape($top))/(?:core|module)/[^/]+/src/main/java)/") {
                $Matches[1]
            }
        } |
        Sort-Object -Unique
)
$members += "$top/LICENSE.txt"
$members += "$top/README.adoc"
if ($members.Count -le 2) {
    throw "Unexpected Spring Boot source layout."
}

Remove-Tree -Path $target
New-Item -ItemType Directory -Path $target -Force | Out-Null

Write-Host "Extracting Spring Boot framework modules..."
& $tar -xf $archive -C $target --strip-components=1 @members
if ($LASTEXITCODE -ne 0) {
    throw "Spring Boot source extraction failed."
}
Remove-Item -LiteralPath $archive -Force

$manifestDir = Join-Path $target ".lattice"
New-Item -ItemType Directory -Path $manifestDir -Force | Out-Null
$manifest = @"
{
  "id": "spring-boot-$version",
  "name": "Spring Boot",
  "implementation": "Spring Boot",
  "version": "$version",
  "language": "Java",
  "source_language": "Java",
  "license": "Apache-2.0",
  "default_entry": "core/spring-boot/src/main/java/org/springframework/boot/SpringApplication.java"
}
"@
[System.IO.File]::WriteAllText(
    (Join-Path $manifestDir "pack.json"),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

$files = Get-ChildItem -LiteralPath $target -Recurse -File
$bytes = ($files | Measure-Object Length -Sum).Sum
Write-Host ("Spring Boot {0} ready: {1:N0} files, {2:N1} MiB" -f $version, $files.Count, ($bytes / 1MB))
