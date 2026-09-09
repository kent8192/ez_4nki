param(
    [Parameter(Mandatory)][ValidateSet('start', 'finish')][string]$Mode,
    [Parameter(Mandatory)][string]$StateDirectory,
    [Parameter(Mandatory)][string]$Destination
)
$ErrorActionPreference = 'Stop'
if ($env:CI -ne 'true') { throw 'Run only in a disposable CI VM with synthetic application data.' }
$backup = Join-Path $StateDirectory 'audit-policy.csv'
$started = Join-Path $StateDirectory 'network-start.txt'
# Audit_ObjectAccess_FirewallConnection, independent of the runner's locale.
# https://learn.microsoft.com/en-us/windows/win32/secauthz/auditing-constants
$subcategory = '{0CCE9226-69AE-11D9-BED3-505054503030}'
function Invoke-AuditPolicy([string[]]$Arguments) {
    & auditpol.exe @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Audit policy operation failed: $LASTEXITCODE" }
}
if ($Mode -eq 'start') {
    if ((Test-Path $backup) -or (Test-Path $started)) { throw 'Refusing to overwrite existing audit state.' }
    Invoke-AuditPolicy -Arguments @('/backup', "/file:$backup")
    try {
        Invoke-AuditPolicy -Arguments @('/set', "/subcategory:$subcategory", '/success:enable', '/failure:enable')
        [DateTime]::UtcNow.ToString('o') | Set-Content $started
    } catch {
        Invoke-AuditPolicy -Arguments @('/restore', "/file:$backup")
        throw
    }
    Write-Output 'Started WFP connection observation in the disposable CI VM.'
    exit
}

try {
    $start = [DateTime]::Parse((Get-Content $started -Raw), [Globalization.CultureInfo]::InvariantCulture, [Globalization.DateTimeStyles]::RoundtripKind)
    $installRoot = (Split-Path $env:KOTOBA_TEST_APPLICATION -Parent).ToLowerInvariant().Replace('/', '\')
    if ($installRoot -notmatch '^[a-z]:\\.+\\kotoba-installed$') { throw 'Unexpected CI installation directory.' }
    # WFP uses an NT volume prefix instead of a drive letter; match the complete
    # installation-directory suffix, including the trailing separator.
    $applicationPattern = '^(?:\\device\\harddiskvolume[0-9]+|[a-z]:)' + [regex]::Escape($installRoot.Substring(2) + '\')
    $records = @()
    $events = @(Get-WinEvent -FilterHashtable @{ LogName='Security'; Id=5154,5156,5157; StartTime=$start } -ErrorAction Stop)
    foreach ($event in $events) {
        $xml = [xml]$event.ToXml()
        $fields = @{}
        foreach ($field in $xml.Event.EventData.Data) { $fields[$field.Name] = [string]$field.'#text' }
        if ($fields.Application -notmatch $applicationPattern) { continue }
        $records += [pscustomobject]@{
            Time=$event.TimeCreated.ToUniversalTime().ToString('o'); EventId=$event.Id
            ProcessId=$fields.ProcessID; Application=$fields.Application; Direction=$fields.Direction
            SourceAddress=$fields.SourceAddress; SourcePort=$fields.SourcePort
            DestinationAddress=$fields.DestAddress; DestinationPort=$fields.DestPort; Protocol=$fields.Protocol
        }
    }
    # Only this application's and its bundled runtime's connection metadata is
    # retained. Do not export the host Security log, URLs, or packet payloads.
    ConvertTo-Json -InputObject $records -Depth 4 | Set-Content (Join-Path $Destination 'network-windows.json')
    if ($records.Count -eq 0) { throw 'No application WFP evidence; an empty capture is not a passing network check.' }
    $external = @($records | Where-Object {
        if ([string]::IsNullOrWhiteSpace($_.DestinationAddress)) { return $false }
        $address = [Net.IPAddress]::Parse($_.DestinationAddress)
        if ($address.IsIPv4MappedToIPv6) { $address = $address.MapToIPv4() }
        return ![Net.IPAddress]::IsLoopback($address)
    })
    if ($external.Count -gt 0) { throw "Observed $($external.Count) non-loopback connection events for the installed application/runtime." }
    Write-Output "No external IP destination in $($records.Count) WFP events for the installed application and bundled runtime."
} finally {
    Invoke-AuditPolicy -Arguments @('/restore', "/file:$backup")
    Write-Output 'Restored the disposable runner audit policy.'
}
