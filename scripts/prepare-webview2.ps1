param([switch]$WithDriver)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Set-StrictMode -Version Latest

$manifest = Get-Content (Join-Path $PSScriptRoot 'webview2-runtime.json') -Raw | ConvertFrom-Json
$root = Split-Path $PSScriptRoot -Parent
$runtimeRoot = Join-Path $root 'src-tauri/runtime'
$destination = Join-Path $runtimeRoot 'webview2'
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('kotoba-webview2-' + [guid]::NewGuid())

function Get-VerifiedArchive($url, $hash, $path) {
    Invoke-WebRequest -Uri $url -OutFile $path
    if ((Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) {
        throw 'Downloaded archive does not match the pinned SHA256.'
    }
}

function Assert-MicrosoftBinary($path) {
    $signature = Get-AuthenticodeSignature $path
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=Microsoft Corporation') {
        throw "Microsoft Authenticode signature verification failed: $path"
    }
    $version = (Get-Item $path).VersionInfo.ProductVersion
    if ($version -ne $manifest.version) {
        throw "Unexpected binary version: $version"
    }
}

New-Item -ItemType Directory -Path $temporary | Out-Null
try {
    if (Test-Path $destination) {
        throw 'Runtime directory already exists. Move it aside before preparing a verified runtime.'
    }
    $cab = Join-Path $temporary 'runtime.cab'
    Get-VerifiedArchive $manifest.url $manifest.sha256 $cab
    $expanded = New-Item -ItemType Directory -Path (Join-Path $temporary 'expanded')
    & expand.exe $cab '-F:*' $expanded.FullName | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'WebView2 CAB expansion failed.' }
    $executables = @(Get-ChildItem $expanded.FullName -Recurse -Filter msedgewebview2.exe)
    if ($executables.Count -ne 1) { throw 'Expected exactly one WebView2 executable.' }
    Assert-MicrosoftBinary $executables[0].FullName
    New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
    Copy-Item $executables[0].Directory.FullName $destination -Recurse
    Write-Output "Verified Fixed Version WebView2 $($manifest.version), SHA256 $($manifest.sha256)"

    if ($WithDriver) {
        $driverZip = Join-Path $temporary 'driver.zip'
        Get-VerifiedArchive $manifest.driver.url $manifest.driver.sha256 $driverZip
        $driverDestination = Join-Path $runtimeRoot 'driver'
        if (Test-Path $driverDestination) { throw 'Driver directory already exists.' }
        Expand-Archive $driverZip -DestinationPath $driverDestination
        Assert-MicrosoftBinary (Join-Path $driverDestination 'msedgedriver.exe')
    }
} finally {
    Remove-Item $temporary -Recurse -Force
}
