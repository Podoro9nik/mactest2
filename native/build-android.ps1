param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $env:CHIIKAWA_EMBEDDED_MCP_KEY) {
    $keyLine = Get-Content -LiteralPath (Join-Path $repoRoot '.env') | Where-Object { $_ -match '^ELYUM_MCP_KEY=' } | Select-Object -First 1
    if ($keyLine) { $env:CHIIKAWA_EMBEDDED_MCP_KEY = $keyLine.Substring(14).Trim().Trim('"').Trim("'") }
}
if (-not $env:CHIIKAWA_EMBEDDED_MCP_KEY) { throw 'Set CHIIKAWA_EMBEDDED_MCP_KEY first.' }
$signingFile = Join-Path $repoRoot 'build/android-signing.secret.json'
if (Test-Path -LiteralPath $signingFile) {
    $signing = Get-Content -LiteralPath $signingFile -Raw | ConvertFrom-Json
    $env:CHIIKAWA_ANDROID_KEYSTORE = $signing.path
    $env:CHIIKAWA_ANDROID_KEY_ALIAS = $signing.alias
    $env:CHIIKAWA_ANDROID_STORE_PASSWORD = $signing.password
    $env:CHIIKAWA_ANDROID_KEY_PASSWORD = $signing.password
}
if (-not $env:CHIIKAWA_ANDROID_KEYSTORE) { throw 'Configure the private Android signing keystore environment variables.' }
if (-not $env:ANDROID_HOME) { $env:ANDROID_HOME = Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
Push-Location (Join-Path $PSScriptRoot 'android')
try {
    if (-not $SkipTests) {
        & .\gradlew.bat testDebugUnitTest --console=plain
        if ($LASTEXITCODE -ne 0) { throw 'Android tests failed' }
    }
    & .\gradlew.bat assembleRelease --console=plain
    if ($LASTEXITCODE -ne 0) { throw 'Android build failed' }
} finally { Pop-Location }
