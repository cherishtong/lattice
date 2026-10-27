$ErrorActionPreference = "Stop"

. "$PSScriptRoot\prepare-native-build.ps1"

Push-Location $script:RepoRoot
try {
    cargo run -p lattice --no-default-features --features native
} finally {
    Pop-Location
}
