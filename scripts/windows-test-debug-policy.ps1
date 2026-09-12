param([ValidateSet('create', 'remove')][string]$Mode)
$ErrorActionPreference = 'Stop'
if ($env:CI -ne 'true') { throw 'Run only in an isolated CI account with synthetic application data.' }
# Hosted Windows runners are elevated. WebView2 intentionally ignores environment
# flags and HKCU overrides in elevated processes, but honors HKLM policy.
# https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security#for-an-elevated-host-app-use-appropriate-override-flags
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
$elevated = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (!$elevated) { throw 'This isolated runner policy requires an elevated CI process.' }
$key = 'HKLM:\Software\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
$applicationName = 'kotoba-desktop.exe'
if ($Mode -eq 'create') {
    $existing = Get-ItemProperty -Path $key -Name $applicationName -ErrorAction SilentlyContinue
    if ($null -ne $existing) { throw 'Refusing to overwrite an existing application policy.' }
    if (!(Test-Path $key)) { New-Item -Path $key -Force | Out-Null }
    New-ItemProperty -Path $key -Name $applicationName -PropertyType String -Value '--remote-debugging-port=9222 --remote-debugging-address=127.0.0.1' | Out-Null
    $actual = Get-ItemPropertyValue -Path $key -Name $applicationName
    if ($actual -ne '--remote-debugging-port=9222 --remote-debugging-address=127.0.0.1') { throw 'CI debugging policy readback failed.' }
    Write-Output "Elevated CI process: $elevated. Created and verified the application-specific HKLM loopback debugging policy."
} else {
    Remove-ItemProperty -Path $key -Name $applicationName
    Write-Output 'Removed the CI application debugging policy.'
}
