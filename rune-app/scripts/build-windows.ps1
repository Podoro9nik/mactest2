param(
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [switch]$SkipSmoke,
    [string]$NsisPath
)
$ErrorActionPreference = 'Stop'
$appRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$repoRoot = Split-Path -Parent $appRoot
$toolchain = '1.99.1'
$releaseDir = Join-Path $appRoot 'target/release'
$distDir = Join-Path $appRoot 'dist'
$exePath = Join-Path $releaseDir 'chiikawizator-rune.exe'

if (-not $SkipBuild) {
    if (-not $env:CHIIKAWA_EMBEDDED_MCP_KEY) {
        $envPath = Join-Path $repoRoot '.env'
        if (Test-Path -LiteralPath $envPath) {
            foreach ($line in [IO.File]::ReadLines($envPath)) {
                if ($line -match '^\s*(?:CHIIKAWA_EMBEDDED_MCP_KEY|ELYUM_MCP_KEY)\s*=\s*(.+?)\s*$') {
                    $env:CHIIKAWA_EMBEDDED_MCP_KEY = $Matches[1].Trim().Trim('"').Trim("'")
                    break
                }
            }
        }
    }
    if (-not $env:CHIIKAWA_EMBEDDED_MCP_KEY) {
        throw 'Set CHIIKAWA_EMBEDDED_MCP_KEY before creating the personal installer.'
    }
    if (-not (Test-Path -LiteralPath (Join-Path $appRoot 'vendor/rune/crates/rune/Cargo.toml'))) {
        throw 'Rune checkout is missing at rune-app/vendor/rune. See the build documentation.'
    }
    # A fixed directory keeps packaging independent of the caller''s Cargo environment.
    $env:CARGO_TARGET_DIR = Join-Path $appRoot 'target'
    if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '2' }
    Push-Location $appRoot
    try {
        if (-not $SkipTests) {
            & cargo "+$toolchain" test --release --locked -- --test-threads=1
            if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed.' }
        }
        & cargo "+$toolchain" build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }
    } finally { Pop-Location }
}
if (-not (Test-Path -LiteralPath $exePath)) { throw "Executable is missing: $exePath" }
if (-not $SkipSmoke) {
    & node (Join-Path $PSScriptRoot 'verify-package.mjs') $exePath (Join-Path $appRoot 'verification/windows')
    if ($LASTEXITCODE -ne 0) { throw 'Packaged application verification failed.' }
}

if (-not $NsisPath) {
    $nsisCommand = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($nsisCommand) { $NsisPath = $nsisCommand.Source }
}
if (-not $NsisPath) {
    $nsisCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} 'NSIS/makensis.exe'),
        (Join-Path $env:LOCALAPPDATA 'tauri/nsis-3.11/makensis.exe'),
        (Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe')
    )
    $NsisPath = $nsisCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $NsisPath) { throw 'NSIS 3 is required to package the installer. Pass -NsisPath or add makensis.exe to PATH.' }
[IO.Directory]::CreateDirectory($distDir) | Out-Null
$installerPath = Join-Path $distDir 'Chiikawizator_4.1.0_windows-x64-setup.exe'
& $NsisPath '/V2' "/DAPP_ROOT=$appRoot" "/DEXE_PATH=$exePath" "/DOUTPUT_PATH=$installerPath" (Join-Path $appRoot 'installer/windows.nsi')
if ($LASTEXITCODE -ne 0) { throw 'NSIS packaging failed.' }
$hash = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText("$installerPath.sha256", "$hash  $([IO.Path]::GetFileName($installerPath))`n")
Write-Host "Installer ready: $installerPath"
