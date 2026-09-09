param([ValidateSet('create', 'remove')][string]$Mode)
$ErrorActionPreference = 'Stop'
if ($env:CI -ne 'true') { throw 'Run only in an isolated CI account with synthetic application data.' }
$key = 'HKCU:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$applicationName = 'kotoba-desktop.exe'
if ($Mode -eq 'create') {
    $existing = Get-ItemProperty -Path $key -Name $applicationName -ErrorAction SilentlyContinue
    if ($null -ne $existing) { throw 'Refusing to overwrite an existing application policy.' }
    if (!(Test-Path $key)) { New-Item -Path $key -Force | Out-Null }
    New-ItemProperty -Path $key -Name $applicationName -PropertyType String -Value '--remote-debugging-port=9222 --remote-debugging-address=127.0.0.1' | Out-Null
    Write-Output 'Created an application-specific loopback debugging policy for the isolated CI run.'
} else {
    Remove-ItemProperty -Path $key -Name $applicationName
    Write-Output 'Removed the CI application debugging policy.'
}
