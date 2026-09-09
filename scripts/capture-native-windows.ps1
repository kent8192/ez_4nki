param([int]$ApplicationProcessId, [string]$Destination)
$ErrorActionPreference = 'Stop'
if ($env:CI -ne 'true') { throw 'Run only in an isolated CI account with synthetic application data.' }

$processes = @(Get-CimInstance Win32_Process)
$ids = [System.Collections.Generic.HashSet[int]]::new()
$null = $ids.Add($ApplicationProcessId)
do {
    $added = $false
    foreach ($process in $processes) {
        if ($ids.Contains([int]$process.ParentProcessId) -and $ids.Add([int]$process.ProcessId)) { $added = $true }
    }
} while ($added)
$processes | Where-Object { $ids.Contains([int]$_.ProcessId) } |
    Select-Object ProcessId, ParentProcessId, SessionId, Name, ExecutablePath, CommandLine |
    ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Destination 'native-processes.json')

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$root = [System.Windows.Automation.AutomationElement]::RootElement
$windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
$elements = @()
foreach ($window in $windows) {
    if (!$ids.Contains($window.Current.ProcessId)) { continue }
    $children = $window.FindAll([System.Windows.Automation.TreeScope]::Subtree, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in ($children | Select-Object -First 200)) {
        $elements += [pscustomobject]@{ Name=$element.Current.Name; ClassName=$element.Current.ClassName; ControlType=$element.Current.ControlType.ProgrammaticName; ProcessId=$element.Current.ProcessId }
    }
}
ConvertTo-Json -InputObject $elements -Depth 4 | Set-Content (Join-Path $Destination 'native-windows.json')

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = [System.Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bounds.Size)
    $bitmap.Save((Join-Path $Destination 'native-desktop.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}

$runtime = Join-Path (Split-Path $env:KOTOBA_TEST_APPLICATION -Parent) 'runtime/webview2'
Get-ChildItem $runtime | Select-Object Name, Length, Mode |
    ConvertTo-Json -Depth 4 | Set-Content (Join-Path $Destination 'installed-runtime-files.json')
