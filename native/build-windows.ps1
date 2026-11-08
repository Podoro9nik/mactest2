param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
$nativeRoot = $PSScriptRoot
$repoRoot = Split-Path -Parent $nativeRoot
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '2' }
if (-not $env:CHIIKAWA_EMBEDDED_MCP_KEY) {
    $envFile = Join-Path $repoRoot '.env'
    if (Test-Path -LiteralPath $envFile) {
        $keyLine = Get-Content -LiteralPath $envFile | Where-Object { $_ -match '^ELYUM_MCP_KEY=' } | Select-Object -First 1
        if ($keyLine) { $env:CHIIKAWA_EMBEDDED_MCP_KEY = $keyLine.Substring(14).Trim().Trim('"').Trim("'") }
    }
}
if (-not $env:CHIIKAWA_EMBEDDED_MCP_KEY) { throw 'Set CHIIKAWA_EMBEDDED_MCP_KEY before making a personal distribution.' }
Push-Location $nativeRoot
try {
    & npm.cmd ci
    if ($LASTEXITCODE -ne 0) { throw 'npm ci failed' }
    & node scripts/prepare-codex.mjs
    if ($LASTEXITCODE -ne 0) { throw 'Codex preparation failed' }
    Push-Location desktop
    try {
        if (-not $SkipTests) {
            & cargo test --locked
            if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
        }
        & ..\node_modules\.bin\tauri.cmd build --bundles nsis
        if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed' }
    } finally { Pop-Location }
} finally { Pop-Location }
