[CmdletBinding()]
param(
    [string]$KeyPath = (Join-Path $env:USERPROFILE ".tauri\relay-updater.key")
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $KeyPath -PathType Leaf)) {
    throw "Relay updater key not found at $KeyPath"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$tauriDir = Join-Path $repoRoot "src-tauri"
$tauriConfig = Get-Content -Raw -LiteralPath (Join-Path $tauriDir "tauri.conf.json") | ConvertFrom-Json
$version = [string]$tauriConfig.version
$binary = Join-Path $tauriDir "target\release\relay.exe"
$portable = Join-Path $tauriDir "target\release\Relay_${version}_x64-portable.exe"
$installer = Join-Path $tauriDir "target\release\bundle\nsis\Relay_${version}_x64-setup.exe"
$signature = "$installer.sig"
$resolvedKeyPath = (Resolve-Path -LiteralPath $KeyPath).Path
$securePassword = $null
$credential = $null
$locationPushed = $false

try {
    Push-Location $tauriDir
    $locationPushed = $true
    cargo tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "The signed Relay build failed with exit code $LASTEXITCODE"
    }
    if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
        throw "Relay installer not found at $installer"
    }
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "Relay release binary not found at $binary"
    }
    Copy-Item -LiteralPath $binary -Destination $portable -Force

    $securePassword = Read-Host "Relay updater key password" -AsSecureString
    $credential = [PSCredential]::new("relay-updater", $securePassword)
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $credential.GetNetworkCredential().Password
    cargo tauri signer sign --private-key-path $resolvedKeyPath $installer
    if ($LASTEXITCODE -ne 0) {
        throw "The Relay installer signing failed with exit code $LASTEXITCODE"
    }
    if (-not (Test-Path -LiteralPath $signature -PathType Leaf)) {
        throw "Updater signature not found at $signature"
    }

    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    $env:RELAY_SIGNED_INSTALLER = $installer
    cargo test updater::tests::signed_release_artifact_matches_pinned_key_and_rejects_tampering -- --ignored --exact
    if ($LASTEXITCODE -ne 0) {
        throw "The signed updater artifact did not pass verification"
    }

    Write-Host "Signed Relay $version release verified:" -ForegroundColor Green
    Write-Host $installer
    Write-Host $signature
    Write-Host $portable
}
finally {
    if ($locationPushed) {
        Pop-Location
    }
    Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    Remove-Item Env:\RELAY_SIGNED_INSTALLER -ErrorAction SilentlyContinue
    $credential = $null
    if ($null -ne $securePassword) {
        $securePassword.Dispose()
    }
}
